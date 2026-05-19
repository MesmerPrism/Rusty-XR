use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    thread::{self, JoinHandle},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusty_xr_osc::{
    OscArgument, OscEndpointConfig, OscIoError, OscPacket, OscStreamRole, OscUdpSocket,
};

use crate::{log_error, log_info, RuntimeConfig};

static OSC_LISTENER: OnceLock<Mutex<Option<OscListenerHandle>>> = OnceLock::new();
static OSC_SNAPSHOT: OnceLock<Mutex<OscIngressSnapshot>> = OnceLock::new();

#[derive(Clone, Debug)]
pub(crate) struct OscIngressSnapshot {
    pub enabled: bool,
    pub listening: bool,
    pub bind_addr: String,
    pub local_addr: Option<String>,
    pub max_packet_bytes: usize,
    pub packet_count: u64,
    pub last_received_unix_ms: Option<u128>,
    pub last_peer: Option<String>,
    pub last_byte_len: usize,
    pub last_packet_summary: Option<String>,
    pub last_error: Option<String>,
    pub updated_unix_ms: u128,
}

impl Default for OscIngressSnapshot {
    fn default() -> Self {
        Self {
            enabled: false,
            listening: false,
            bind_addr: String::new(),
            local_addr: None,
            max_packet_bytes: 0,
            packet_count: 0,
            last_received_unix_ms: None,
            last_peer: None,
            last_byte_len: 0,
            last_packet_summary: None,
            last_error: None,
            updated_unix_ms: now_unix_ms(),
        }
    }
}

struct OscListenerHandle {
    bind_addr: String,
    max_packet_bytes: usize,
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl OscListenerHandle {
    fn stop(mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub(crate) fn ensure_listener(config: &RuntimeConfig) {
    let state = OSC_LISTENER.get_or_init(|| Mutex::new(None));
    let Ok(mut current) = state.lock() else {
        log_error("Rusty XR OSC listener state lock failed");
        update_snapshot(|snapshot| {
            snapshot.last_error = Some("listener state lock failed".to_string());
        });
        return;
    };

    if !config.osc_enabled {
        if let Some(listener) = current.take() {
            let bind_addr = listener.bind_addr.clone();
            listener.stop();
            log_info(format!("Rusty XR OSC listener stopped bind={bind_addr}"));
        }
        update_snapshot(|snapshot| {
            snapshot.enabled = false;
            snapshot.listening = false;
            snapshot.bind_addr = config.osc_listen_addr.clone();
            snapshot.max_packet_bytes = config.osc_max_packet_bytes;
        });
        return;
    }

    if current
        .as_ref()
        .map(|listener| {
            listener.bind_addr == config.osc_listen_addr
                && listener.max_packet_bytes == config.osc_max_packet_bytes
        })
        .unwrap_or(false)
    {
        update_snapshot(|snapshot| {
            snapshot.enabled = true;
            snapshot.bind_addr = config.osc_listen_addr.clone();
            snapshot.max_packet_bytes = config.osc_max_packet_bytes;
        });
        return;
    }

    if let Some(listener) = current.take() {
        let bind_addr = listener.bind_addr.clone();
        listener.stop();
        log_info(format!(
            "Rusty XR OSC listener restarted from bind={bind_addr}"
        ));
    }

    let bind_addr = config.osc_listen_addr.clone();
    let max_packet_bytes = config.osc_max_packet_bytes;
    update_snapshot(|snapshot| {
        snapshot.enabled = true;
        snapshot.listening = false;
        snapshot.bind_addr = bind_addr.clone();
        snapshot.local_addr = None;
        snapshot.max_packet_bytes = max_packet_bytes;
        snapshot.packet_count = 0;
        snapshot.last_received_unix_ms = None;
        snapshot.last_peer = None;
        snapshot.last_byte_len = 0;
        snapshot.last_packet_summary = None;
        snapshot.last_error = None;
    });
    let running = Arc::new(AtomicBool::new(true));
    let thread_running = Arc::clone(&running);
    let thread_bind_addr = bind_addr.clone();
    let thread =
        thread::spawn(move || listen_loop(thread_bind_addr, max_packet_bytes, thread_running));

    *current = Some(OscListenerHandle {
        bind_addr,
        max_packet_bytes,
        running,
        thread: Some(thread),
    });
}

pub(crate) fn ingress_snapshot() -> OscIngressSnapshot {
    OSC_SNAPSHOT
        .get_or_init(|| Mutex::new(OscIngressSnapshot::default()))
        .lock()
        .map(|snapshot| snapshot.clone())
        .unwrap_or_else(|_| OscIngressSnapshot {
            last_error: Some("snapshot lock failed".to_string()),
            ..OscIngressSnapshot::default()
        })
}

fn listen_loop(bind_addr: String, max_packet_bytes: usize, running: Arc<AtomicBool>) {
    let socket = match OscUdpSocket::bind(
        OscEndpointConfig::new(bind_addr.clone())
            .with_max_packet_bytes(max_packet_bytes)
            .with_role(OscStreamRole::SensorIngress),
    ) {
        Ok(socket) => socket,
        Err(error) => {
            log_error(format!(
                "Rusty XR OSC listener bind failed bind={bind_addr} error={error}"
            ));
            update_snapshot(|snapshot| {
                snapshot.enabled = true;
                snapshot.listening = false;
                snapshot.bind_addr = bind_addr.clone();
                snapshot.local_addr = None;
                snapshot.max_packet_bytes = max_packet_bytes;
                snapshot.last_error = Some(truncate_for_overlay(&error.to_string(), 96));
            });
            return;
        }
    };

    if let Err(error) = socket.set_read_timeout(Some(Duration::from_millis(250))) {
        log_error(format!(
            "Rusty XR OSC listener timeout setup failed bind={bind_addr} error={error}"
        ));
        update_snapshot(|snapshot| {
            snapshot.enabled = true;
            snapshot.listening = false;
            snapshot.bind_addr = bind_addr.clone();
            snapshot.max_packet_bytes = max_packet_bytes;
            snapshot.last_error = Some(truncate_for_overlay(&error.to_string(), 96));
        });
        return;
    }

    let local_addr = socket
        .local_addr()
        .map(|addr| addr.to_string())
        .unwrap_or_else(|_| bind_addr.clone());
    update_snapshot(|snapshot| {
        snapshot.enabled = true;
        snapshot.listening = true;
        snapshot.bind_addr = bind_addr.clone();
        snapshot.local_addr = Some(local_addr.clone());
        snapshot.max_packet_bytes = max_packet_bytes;
        snapshot.last_error = None;
    });
    log_info(format!(
        "Rusty XR OSC listener started bind={bind_addr} local={local_addr} maxPacketBytes={max_packet_bytes}"
    ));

    let mut packet_count = 0_u64;
    while running.load(Ordering::Relaxed) {
        match socket.recv_packet() {
            Ok(received) => {
                packet_count = packet_count.saturating_add(1);
                let summary = packet_summary(&received.packet);
                update_snapshot(|snapshot| {
                    snapshot.enabled = true;
                    snapshot.listening = true;
                    snapshot.bind_addr = bind_addr.clone();
                    snapshot.local_addr = Some(local_addr.clone());
                    snapshot.max_packet_bytes = max_packet_bytes;
                    snapshot.packet_count = packet_count;
                    snapshot.last_received_unix_ms = Some(now_unix_ms());
                    snapshot.last_peer = Some(received.peer_addr.to_string());
                    snapshot.last_byte_len = received.byte_len;
                    snapshot.last_packet_summary = Some(truncate_for_overlay(&summary, 160));
                    snapshot.last_error = None;
                });
                if packet_count <= 5 || packet_count.is_multiple_of(60) {
                    log_info(format!(
                        "Rusty XR OSC packet received count={} peer={} bytes={} {}",
                        packet_count, received.peer_addr, received.byte_len, summary
                    ));
                }
            }
            Err(OscIoError::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(error) => {
                log_error(format!("Rusty XR OSC packet receive failed error={error}"));
                update_snapshot(|snapshot| {
                    snapshot.enabled = true;
                    snapshot.listening = true;
                    snapshot.bind_addr = bind_addr.clone();
                    snapshot.local_addr = Some(local_addr.clone());
                    snapshot.max_packet_bytes = max_packet_bytes;
                    snapshot.last_error = Some(truncate_for_overlay(&error.to_string(), 96));
                });
            }
        }
    }

    update_snapshot(|snapshot| {
        snapshot.listening = false;
        snapshot.bind_addr = bind_addr.clone();
        snapshot.local_addr = Some(local_addr);
        snapshot.max_packet_bytes = max_packet_bytes;
    });
    log_info(format!("Rusty XR OSC listener exited bind={bind_addr}"));
}

fn packet_summary(packet: &OscPacket) -> String {
    match packet {
        OscPacket::Message(message) => format!(
            "message address={} args={} types={}",
            message.address,
            message.arguments.len(),
            message
                .arguments
                .iter()
                .map(argument_label)
                .collect::<Vec<_>>()
                .join(",")
        ),
        OscPacket::Bundle(bundle) => format!(
            "bundle timetag={} packets={}",
            bundle.timetag,
            bundle.packets.len()
        ),
    }
}

fn argument_label(argument: &OscArgument) -> &'static str {
    match argument {
        OscArgument::Int(_) => "int",
        OscArgument::Float(_) => "float",
        OscArgument::String(_) => "string",
        OscArgument::Blob(_) => "blob",
        OscArgument::Bool(true) => "true",
        OscArgument::Bool(false) => "false",
        OscArgument::Nil => "nil",
        OscArgument::Impulse => "impulse",
    }
}

fn update_snapshot(update: impl FnOnce(&mut OscIngressSnapshot)) {
    let state = OSC_SNAPSHOT.get_or_init(|| Mutex::new(OscIngressSnapshot::default()));
    if let Ok(mut snapshot) = state.lock() {
        update(&mut snapshot);
        snapshot.updated_unix_ms = now_unix_ms();
    }
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn truncate_for_overlay(value: &str, max_chars: usize) -> String {
    let mut output = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        output.push_str("...");
    }
    output
}
