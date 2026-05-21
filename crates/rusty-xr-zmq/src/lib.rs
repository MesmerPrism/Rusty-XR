//! Optional pure-Rust ZeroMQ adapter helpers for Rusty XR.
//!
//! This crate is the runtime counterpart to the ZeroMQ bridge manifests in
//! `rusty-xr-broker-model`. Default builds are model-only: no socket runtime is
//! linked unless the `runtime` feature is enabled. The runtime feature targets
//! the pure Rust `zeromq` crate and does not link to native `libzmq`.
//!
//! ```
//! use rusty_xr_broker_model::{
//!     BrokerPayloadKind, BrokerStreamDirection, BrokerTransportEndpoint,
//!     BrokerZeroMqBindMode, BrokerZeroMqBridgeManifest, BrokerZeroMqPattern,
//!     BROKER_LATENCY_SAMPLE_SCHEMA,
//! };
//! use rusty_xr_zmq::{ZmqOpenMode, ZmqPubSubReceiverConfig};
//!
//! let manifest = BrokerZeroMqBridgeManifest::new(
//!     "loopback-latency",
//!     BrokerTransportEndpoint::zeromq_tcp("127.0.0.1", 5557),
//!     BrokerZeroMqPattern::PubSub,
//!     BrokerStreamDirection::ProducerToConsumer,
//!     BrokerPayloadKind::Json,
//!     BROKER_LATENCY_SAMPLE_SCHEMA,
//! )
//! .with_bind_mode(BrokerZeroMqBindMode::Connect)
//! .with_topic_prefix("rustyxr.latency");
//!
//! let config = ZmqPubSubReceiverConfig::try_from_manifest(&manifest)
//!     .expect("manifest is a supported ZeroMQ receiver");
//! assert_eq!(config.open_mode, ZmqOpenMode::Connect);
//! assert_eq!(config.endpoint, "tcp://127.0.0.1:5557");
//! ```

use rusty_xr_broker_model::{
    BrokerPayloadKind, BrokerTransportKind, BrokerZeroMqBindMode, BrokerZeroMqBridgeManifest,
    BrokerZeroMqPattern, MAX_ZEROMQ_BRIDGE_MESSAGE_BYTES,
};
use std::{
    collections::VecDeque,
    error::Error,
    fmt,
    sync::{Arc, Mutex},
};

#[cfg(feature = "runtime")]
use std::{
    io,
    sync::mpsc::{self, Sender, TryRecvError},
    thread::{self, JoinHandle},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[cfg(feature = "runtime")]
use zeromq::{Socket, SocketRecv, SubSocket, ZmqMessage};

/// Crate version exposed for lightweight smoke checks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Versioned schema id for received ZeroMQ messages.
pub const ZMQ_RECEIVED_MESSAGE_SCHEMA: &str = "rusty.xr.zmq.received_message.v1";

/// Versioned schema id for ZeroMQ receiver snapshots.
pub const ZMQ_RECEIVER_SNAPSHOT_SCHEMA: &str = "rusty.xr.zmq.receiver_snapshot.v1";

const DEFAULT_QUEUE_CAPACITY: usize = 512;
const DEFAULT_RECEIVE_TIMEOUT_MS: u64 = 25;

/// Whether a runtime adapter binds or connects the configured endpoint.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZmqOpenMode {
    Bind,
    Connect,
}

/// Runtime state for a bounded ZeroMQ receiver.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZmqReceiverStatus {
    Starting,
    Connected,
    Fault,
    Stopped,
}

/// Errors returned by manifest conversion and runtime setup helpers.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ZmqAdapterError {
    InvalidManifest,
    UnsupportedPattern(BrokerZeroMqPattern),
    AmbiguousBindMode,
    MissingHost,
    MissingPort,
    MessageTooLarge { actual: usize, max: usize },
}

impl fmt::Display for ZmqAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidManifest => f.write_str("invalid ZeroMQ bridge manifest"),
            Self::UnsupportedPattern(pattern) => {
                write!(f, "unsupported ZeroMQ socket pattern: {pattern:?}")
            }
            Self::AmbiguousBindMode => {
                f.write_str("ZeroMQ runtime receiver requires an explicit bind or connect mode")
            }
            Self::MissingHost => f.write_str("ZeroMQ endpoint is missing a host"),
            Self::MissingPort => f.write_str("ZeroMQ endpoint is missing a port"),
            Self::MessageTooLarge { actual, max } => {
                write!(
                    f,
                    "ZeroMQ message is too large: {actual} bytes > {max} bytes"
                )
            }
        }
    }
}

impl Error for ZmqAdapterError {}

/// Receiver configuration derived from a public ZeroMQ bridge manifest.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZmqPubSubReceiverConfig {
    pub bridge_id: String,
    pub endpoint: String,
    pub topic_prefix: String,
    pub open_mode: ZmqOpenMode,
    pub queue_capacity: usize,
    pub receive_timeout_ms: u64,
    pub payload_schema: String,
    pub payload_kind: BrokerPayloadKind,
    pub max_message_bytes: usize,
}

impl ZmqPubSubReceiverConfig {
    /// Build a receiver config from a manifest with an explicit bind/connect mode.
    pub fn from_manifest_with_open_mode(
        manifest: &BrokerZeroMqBridgeManifest,
        open_mode: ZmqOpenMode,
    ) -> Result<Self, ZmqAdapterError> {
        validate_pub_sub_manifest(manifest)?;
        Ok(Self {
            bridge_id: manifest.bridge_id.clone(),
            endpoint: endpoint_url(manifest)?,
            topic_prefix: manifest.topic_prefix.clone().unwrap_or_default(),
            open_mode,
            queue_capacity: manifest
                .high_water_mark
                .map(|value| value as usize)
                .unwrap_or(DEFAULT_QUEUE_CAPACITY)
                .max(1),
            receive_timeout_ms: DEFAULT_RECEIVE_TIMEOUT_MS,
            payload_schema: manifest.payload_schema.clone(),
            payload_kind: manifest.payload_kind,
            max_message_bytes: manifest
                .max_message_bytes
                .unwrap_or(MAX_ZEROMQ_BRIDGE_MESSAGE_BYTES) as usize,
        })
    }

    /// Build a receiver config from a manifest that already specifies bind/connect mode.
    pub fn try_from_manifest(
        manifest: &BrokerZeroMqBridgeManifest,
    ) -> Result<Self, ZmqAdapterError> {
        let open_mode = match manifest.bind_mode {
            BrokerZeroMqBindMode::Bind => ZmqOpenMode::Bind,
            BrokerZeroMqBindMode::Connect => ZmqOpenMode::Connect,
            BrokerZeroMqBindMode::Either => return Err(ZmqAdapterError::AmbiguousBindMode),
        };
        Self::from_manifest_with_open_mode(manifest, open_mode)
    }

    pub const fn with_receive_timeout_ms(mut self, receive_timeout_ms: u64) -> Self {
        self.receive_timeout_ms = receive_timeout_ms;
        self
    }
}

impl Default for ZmqPubSubReceiverConfig {
    fn default() -> Self {
        Self {
            bridge_id: "rusty-xr-zmq-receiver".to_string(),
            endpoint: "tcp://127.0.0.1:5557".to_string(),
            topic_prefix: String::new(),
            open_mode: ZmqOpenMode::Connect,
            queue_capacity: DEFAULT_QUEUE_CAPACITY,
            receive_timeout_ms: DEFAULT_RECEIVE_TIMEOUT_MS,
            payload_schema: "unknown".to_string(),
            payload_kind: BrokerPayloadKind::Json,
            max_message_bytes: MAX_ZEROMQ_BRIDGE_MESSAGE_BYTES as usize,
        }
    }
}

/// A received ZeroMQ payload normalized for broker or app-frame ingestion.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZmqReceivedMessage {
    pub schema: String,
    pub bridge_id: String,
    pub endpoint: String,
    pub topic_prefix: String,
    pub sequence_number: u64,
    pub received_time_unix_ns: u128,
    pub payload_schema: String,
    pub payload_kind: BrokerPayloadKind,
    pub raw_bytes: Vec<u8>,
    pub utf8_text: Option<String>,
    pub decode_error: Option<String>,
}

impl ZmqReceivedMessage {
    pub fn payload_text_without_topic(&self) -> Option<&str> {
        Some(strip_topic_prefix(
            self.utf8_text.as_deref()?,
            &self.topic_prefix,
        ))
    }

    pub const fn byte_len(&self) -> usize {
        self.raw_bytes.len()
    }
}

/// Snapshot of a bounded ZeroMQ receiver queue.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZmqReceiverSnapshot {
    pub schema: String,
    pub bridge_id: String,
    pub endpoint: String,
    pub topic_prefix: String,
    pub status: ZmqReceiverStatus,
    pub received_count: u64,
    pub drained_count: u64,
    pub dropped_count: u64,
    pub decode_error_count: u64,
    pub queue_len: usize,
    pub last_received_time_unix_ns: Option<u128>,
    pub fault: Option<String>,
}

/// Bounded queue shared by a runtime receiver and an app/render loop.
#[derive(Clone, Debug)]
pub struct ZmqMessageInbox {
    shared: Arc<Mutex<InboxState>>,
}

#[derive(Debug)]
struct InboxState {
    config: ZmqPubSubReceiverConfig,
    status: ZmqReceiverStatus,
    queue: VecDeque<ZmqReceivedMessage>,
    next_sequence_number: u64,
    received_count: u64,
    drained_count: u64,
    dropped_count: u64,
    decode_error_count: u64,
    last_received_time_unix_ns: Option<u128>,
    fault: Option<String>,
}

impl ZmqMessageInbox {
    pub fn new(config: ZmqPubSubReceiverConfig) -> Self {
        Self {
            shared: Arc::new(Mutex::new(InboxState {
                config,
                status: ZmqReceiverStatus::Starting,
                queue: VecDeque::new(),
                next_sequence_number: 0,
                received_count: 0,
                drained_count: 0,
                dropped_count: 0,
                decode_error_count: 0,
                last_received_time_unix_ns: None,
                fault: None,
            })),
        }
    }

    pub fn push_raw_message(
        &self,
        raw_bytes: Vec<u8>,
        received_time_unix_ns: u128,
    ) -> Result<(), ZmqAdapterError> {
        let mut shared = self
            .shared
            .lock()
            .expect("ZeroMQ inbox state lock should not be poisoned");
        let max_message_bytes = shared.config.max_message_bytes.max(1);
        if raw_bytes.len() > max_message_bytes {
            return Err(ZmqAdapterError::MessageTooLarge {
                actual: raw_bytes.len(),
                max: max_message_bytes,
            });
        }

        let (utf8_text, mut decode_error) = match String::from_utf8(raw_bytes.clone()) {
            Ok(text) => (Some(text), None),
            Err(err) => (
                Some(String::from_utf8_lossy(err.as_bytes()).to_string()),
                Some(format!("message is not valid UTF-8: {err}")),
            ),
        };
        if decode_error.is_none() {
            decode_error = utf8_text
                .as_deref()
                .and_then(|text| validate_topic_prefix(text, &shared.config.topic_prefix));
        }

        if decode_error.is_some() {
            shared.decode_error_count = shared.decode_error_count.saturating_add(1);
        }

        let message = ZmqReceivedMessage {
            schema: ZMQ_RECEIVED_MESSAGE_SCHEMA.to_string(),
            bridge_id: shared.config.bridge_id.clone(),
            endpoint: shared.config.endpoint.clone(),
            topic_prefix: shared.config.topic_prefix.clone(),
            sequence_number: shared.next_sequence_number,
            received_time_unix_ns,
            payload_schema: shared.config.payload_schema.clone(),
            payload_kind: shared.config.payload_kind,
            raw_bytes,
            utf8_text,
            decode_error,
        };
        shared.next_sequence_number = shared.next_sequence_number.saturating_add(1);
        push_message_locked(&mut shared, message);
        Ok(())
    }

    pub fn drain_messages(&self) -> Vec<ZmqReceivedMessage> {
        let Ok(mut shared) = self.shared.lock() else {
            return Vec::new();
        };
        let drained: Vec<_> = shared.queue.drain(..).collect();
        shared.drained_count = shared.drained_count.saturating_add(drained.len() as u64);
        drained
    }

    pub fn snapshot(&self) -> ZmqReceiverSnapshot {
        let shared = self
            .shared
            .lock()
            .expect("ZeroMQ inbox state lock should not be poisoned");
        ZmqReceiverSnapshot {
            schema: ZMQ_RECEIVER_SNAPSHOT_SCHEMA.to_string(),
            bridge_id: shared.config.bridge_id.clone(),
            endpoint: shared.config.endpoint.clone(),
            topic_prefix: shared.config.topic_prefix.clone(),
            status: shared.status,
            received_count: shared.received_count,
            drained_count: shared.drained_count,
            dropped_count: shared.dropped_count,
            decode_error_count: shared.decode_error_count,
            queue_len: shared.queue.len(),
            last_received_time_unix_ns: shared.last_received_time_unix_ns,
            fault: shared.fault.clone(),
        }
    }

    pub fn set_status(&self, status: ZmqReceiverStatus) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.status = status;
        }
    }

    pub fn set_fault(&self, fault: impl Into<String>) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.status = ZmqReceiverStatus::Fault;
            shared.fault = Some(fault.into());
        }
    }
}

/// Runtime receiver handle available when the `runtime` feature is enabled.
#[cfg(feature = "runtime")]
pub struct ZmqReceiverHandle {
    inbox: ZmqMessageInbox,
    shutdown_tx: Sender<()>,
    join_handle: Option<JoinHandle<()>>,
}

#[cfg(feature = "runtime")]
impl ZmqReceiverHandle {
    pub fn inbox(&self) -> &ZmqMessageInbox {
        &self.inbox
    }

    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        let _ = self.shutdown_tx.send(());
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

#[cfg(feature = "runtime")]
impl Drop for ZmqReceiverHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Spawn a background PUB/SUB receiver using the pure Rust `zeromq` crate.
#[cfg(feature = "runtime")]
pub fn spawn_pub_sub_receiver(config: ZmqPubSubReceiverConfig) -> io::Result<ZmqReceiverHandle> {
    let inbox = ZmqMessageInbox::new(config.clone());
    let thread_inbox = inbox.clone();
    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let join_handle = thread::Builder::new()
        .name("rusty-xr-zmq-sub-receiver".to_string())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(err) => {
                    thread_inbox.set_fault(format!("failed to create ZeroMQ runtime: {err}"));
                    return;
                }
            };
            runtime.block_on(receiver_loop(config, thread_inbox, shutdown_rx));
        })?;

    Ok(ZmqReceiverHandle {
        inbox,
        shutdown_tx,
        join_handle: Some(join_handle),
    })
}

pub fn strip_topic_prefix<'a>(raw_text: &'a str, topic_prefix: &str) -> &'a str {
    if topic_prefix.is_empty() {
        return raw_text;
    }
    let Some(rest) = raw_text.strip_prefix(topic_prefix) else {
        return raw_text;
    };
    if rest.is_empty() {
        rest
    } else if rest.starts_with(char::is_whitespace) {
        rest.trim_start()
    } else {
        raw_text
    }
}

fn validate_pub_sub_manifest(manifest: &BrokerZeroMqBridgeManifest) -> Result<(), ZmqAdapterError> {
    if !manifest.is_valid() || manifest.endpoint.transport != BrokerTransportKind::ZeroMq {
        return Err(ZmqAdapterError::InvalidManifest);
    }
    if manifest.pattern != BrokerZeroMqPattern::PubSub {
        return Err(ZmqAdapterError::UnsupportedPattern(manifest.pattern));
    }
    Ok(())
}

fn endpoint_url(manifest: &BrokerZeroMqBridgeManifest) -> Result<String, ZmqAdapterError> {
    let host = manifest
        .endpoint
        .host
        .as_deref()
        .ok_or(ZmqAdapterError::MissingHost)?;
    let port = manifest.endpoint.port.ok_or(ZmqAdapterError::MissingPort)?;
    Ok(format!("tcp://{host}:{port}"))
}

fn validate_topic_prefix(raw_text: &str, topic_prefix: &str) -> Option<String> {
    if topic_prefix.is_empty() || raw_text.starts_with(topic_prefix) {
        None
    } else {
        Some(format!(
            "message does not start with topic prefix {topic_prefix:?}"
        ))
    }
}

fn push_message_locked(shared: &mut InboxState, message: ZmqReceivedMessage) {
    if shared.queue.len() >= shared.config.queue_capacity.max(1) {
        shared.queue.pop_front();
        shared.dropped_count = shared.dropped_count.saturating_add(1);
    }
    shared.received_count = shared.received_count.saturating_add(1);
    shared.last_received_time_unix_ns = Some(message.received_time_unix_ns);
    shared.queue.push_back(message);
}

#[cfg(feature = "runtime")]
fn unix_time_ns() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[cfg(feature = "runtime")]
async fn receiver_loop(
    config: ZmqPubSubReceiverConfig,
    inbox: ZmqMessageInbox,
    shutdown_rx: mpsc::Receiver<()>,
) {
    let mut socket = SubSocket::new();
    let setup_result = match config.open_mode {
        ZmqOpenMode::Bind => socket.bind(&config.endpoint).await.map(|_| ()),
        ZmqOpenMode::Connect => socket.connect(&config.endpoint).await,
    };
    if let Err(err) = setup_result {
        inbox.set_fault(format!("failed to open {}: {err}", config.endpoint));
        return;
    }
    if let Err(err) = socket.subscribe(&config.topic_prefix).await {
        inbox.set_fault(format!(
            "failed to subscribe to topic prefix {:?}: {err}",
            config.topic_prefix
        ));
        return;
    }

    inbox.set_status(ZmqReceiverStatus::Connected);
    let receive_timeout = Duration::from_millis(config.receive_timeout_ms.max(1));
    loop {
        match shutdown_rx.try_recv() {
            Ok(()) | Err(TryRecvError::Disconnected) => {
                inbox.set_status(ZmqReceiverStatus::Stopped);
                return;
            }
            Err(TryRecvError::Empty) => {}
        }

        match tokio::time::timeout(receive_timeout, socket.recv()).await {
            Ok(Ok(message)) => match message_to_bytes(message) {
                Ok(bytes) => {
                    if let Err(err) = inbox.push_raw_message(bytes, unix_time_ns()) {
                        inbox.set_fault(err.to_string());
                        return;
                    }
                }
                Err(err) => {
                    inbox.set_fault(err);
                    return;
                }
            },
            Ok(Err(err)) => {
                inbox.set_fault(format!("failed to receive ZeroMQ message: {err}"));
                return;
            }
            Err(_) => {}
        }
    }
}

#[cfg(feature = "runtime")]
fn message_to_bytes(message: ZmqMessage) -> Result<Vec<u8>, String> {
    Vec::<u8>::try_from(message).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_xr_broker_model::{
        BrokerStreamDirection, BrokerTransportEndpoint, BROKER_LATENCY_SAMPLE_SCHEMA,
    };

    fn test_manifest() -> BrokerZeroMqBridgeManifest {
        BrokerZeroMqBridgeManifest::new(
            "loopback-latency",
            BrokerTransportEndpoint::zeromq_tcp("127.0.0.1", 5557),
            BrokerZeroMqPattern::PubSub,
            BrokerStreamDirection::ProducerToConsumer,
            BrokerPayloadKind::Json,
            BROKER_LATENCY_SAMPLE_SCHEMA,
        )
        .with_bind_mode(BrokerZeroMqBindMode::Connect)
        .with_topic_prefix("rustyxr.latency")
        .with_high_water_mark(2)
        .with_max_message_bytes(1024)
    }

    #[test]
    fn config_derives_from_explicit_pub_sub_manifest() {
        let config =
            ZmqPubSubReceiverConfig::try_from_manifest(&test_manifest()).expect("valid manifest");

        assert_eq!(config.bridge_id, "loopback-latency");
        assert_eq!(config.endpoint, "tcp://127.0.0.1:5557");
        assert_eq!(config.topic_prefix, "rustyxr.latency");
        assert_eq!(config.open_mode, ZmqOpenMode::Connect);
        assert_eq!(config.queue_capacity, 2);
        assert_eq!(config.max_message_bytes, 1024);
    }

    #[test]
    fn config_rejects_ambiguous_bind_mode_for_runtime_use() {
        let manifest = BrokerZeroMqBridgeManifest::new(
            "loopback-latency",
            BrokerTransportEndpoint::zeromq_tcp("127.0.0.1", 5557),
            BrokerZeroMqPattern::PubSub,
            BrokerStreamDirection::ProducerToConsumer,
            BrokerPayloadKind::Json,
            BROKER_LATENCY_SAMPLE_SCHEMA,
        );

        assert_eq!(
            ZmqPubSubReceiverConfig::try_from_manifest(&manifest),
            Err(ZmqAdapterError::AmbiguousBindMode)
        );
        assert!(ZmqPubSubReceiverConfig::from_manifest_with_open_mode(
            &manifest,
            ZmqOpenMode::Bind
        )
        .is_ok());
    }

    #[test]
    fn config_rejects_non_pub_sub_manifest() {
        let manifest = BrokerZeroMqBridgeManifest::new(
            "request-reply",
            BrokerTransportEndpoint::zeromq_tcp("127.0.0.1", 5557),
            BrokerZeroMqPattern::RequestReply,
            BrokerStreamDirection::Bidirectional,
            BrokerPayloadKind::Json,
            BROKER_LATENCY_SAMPLE_SCHEMA,
        )
        .with_bind_mode(BrokerZeroMqBindMode::Connect);

        assert_eq!(
            ZmqPubSubReceiverConfig::try_from_manifest(&manifest),
            Err(ZmqAdapterError::UnsupportedPattern(
                BrokerZeroMqPattern::RequestReply
            ))
        );
    }

    #[test]
    fn inbox_drops_oldest_when_bounded() {
        let inbox = ZmqMessageInbox::new(
            ZmqPubSubReceiverConfig::try_from_manifest(&test_manifest()).expect("valid manifest"),
        );

        inbox
            .push_raw_message(b"rustyxr.latency {\"sequence\":1}".to_vec(), 100)
            .expect("message fits");
        inbox
            .push_raw_message(b"rustyxr.latency {\"sequence\":2}".to_vec(), 101)
            .expect("message fits");
        inbox
            .push_raw_message(b"rustyxr.latency {\"sequence\":3}".to_vec(), 102)
            .expect("message fits");

        let snapshot = inbox.snapshot();
        assert_eq!(snapshot.received_count, 3);
        assert_eq!(snapshot.dropped_count, 1);
        assert_eq!(snapshot.queue_len, 2);

        let drained = inbox.drain_messages();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].sequence_number, 1);
        assert_eq!(drained[1].sequence_number, 2);
        assert_eq!(
            drained[0].payload_text_without_topic(),
            Some("{\"sequence\":2}")
        );
    }

    #[test]
    fn topic_prefix_validation_is_non_destructive() {
        let inbox = ZmqMessageInbox::new(
            ZmqPubSubReceiverConfig::try_from_manifest(&test_manifest()).expect("valid manifest"),
        );

        inbox
            .push_raw_message(b"other.topic {\"sequence\":1}".to_vec(), 100)
            .expect("message fits");

        let drained = inbox.drain_messages();
        assert!(drained[0].decode_error.is_some());
        assert_eq!(
            drained[0].payload_text_without_topic(),
            Some("other.topic {\"sequence\":1}")
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip_snapshot_when_enabled() {
        let inbox = ZmqMessageInbox::new(ZmqPubSubReceiverConfig::default());
        let snapshot = inbox.snapshot();

        let json = serde_json::to_string(&snapshot).expect("snapshot should serialize");
        let decoded: ZmqReceiverSnapshot =
            serde_json::from_str(&json).expect("snapshot should deserialize");
        assert_eq!(decoded, snapshot);
    }
}
