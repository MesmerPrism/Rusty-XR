use rusty_xr_broker_model::{
    BrokerModuleHealthState, BrokerModuleLifecycleState, BrokerStreamRegistrySnapshot,
    BROKER_COMMAND_ACK_SCHEMA, BROKER_HOST_MANIFEST_COMMAND,
    BROKER_STREAM_REGISTRY_SNAPSHOT_COMMAND, BROKER_STREAM_REGISTRY_SNAPSHOT_HTTP_PATH,
};
use serde_json::{json, Value};
use std::env;
use std::error::Error;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8765;
const EVENTS_PATH: &str = "/rustyxr/v1/events";
const SIMULATOR_VERSION: &str = env!("CARGO_PKG_VERSION");
const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

fn main() -> Result<(), Box<dyn Error>> {
    let options = SimulatorOptions::parse(env::args().skip(1))?;
    if options.help {
        print_help();
        return Ok(());
    }

    let registry = load_registry_snapshot(&options)?;
    let listener = TcpListener::bind(format!("{}:{}", options.host, options.port))?;
    println!(
        "rusty-xr-broker-registry-simulator listening on {}:{} / profile {} / {}",
        options.host,
        options.port,
        options.profile.as_str(),
        registry.summary_line()
    );

    let mut handled = 0_usize;
    for stream in listener.incoming() {
        let stream = stream?;
        if let Err(error) = handle_connection(stream, &registry) {
            eprintln!("connection error: {error}");
        }
        handled = handled.saturating_add(1);
        if options
            .max_connections
            .map(|max_connections| handled >= max_connections)
            .unwrap_or(false)
        {
            break;
        }
    }

    Ok(())
}

fn handle_connection(
    mut stream: TcpStream,
    registry: &BrokerStreamRegistrySnapshot,
) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let request = read_http_request(&mut stream)?;
    let path = http_request_path(&request)?;

    if is_websocket_request(&request) {
        handle_websocket_connection(stream, &request, path, registry)
    } else {
        handle_http_request(stream, path, registry)
    }
}

fn handle_http_request(
    mut stream: TcpStream,
    path: &str,
    registry: &BrokerStreamRegistrySnapshot,
) -> io::Result<()> {
    match path {
        "/status" => write_http_json(&mut stream, 200, &build_status_value(registry)),
        BROKER_STREAM_REGISTRY_SNAPSHOT_HTTP_PATH => write_http_json(&mut stream, 200, registry),
        _ => write_http_json(
            &mut stream,
            404,
            &json!({
                "error": "not_found",
                "path": path,
                "supported": ["/status", BROKER_STREAM_REGISTRY_SNAPSHOT_HTTP_PATH]
            }),
        ),
    }
}

fn handle_websocket_connection(
    mut stream: TcpStream,
    request: &str,
    path: &str,
    registry: &BrokerStreamRegistrySnapshot,
) -> io::Result<()> {
    if path != EVENTS_PATH {
        write_http_json(
            &mut stream,
            404,
            &json!({
                "error": "not_found",
                "path": path,
                "supported": [EVENTS_PATH]
            }),
        )?;
        return Ok(());
    }

    write_websocket_handshake(&mut stream, request)?;
    send_text_frame(&mut stream, &build_status_value(registry).to_string())?;

    while let Some(text) = read_text_frame(&mut stream)? {
        let message = match serde_json::from_str::<Value>(&text) {
            Ok(value) => build_command_response(&value, registry),
            Err(error) => json!({
                "type": "error",
                "schema": BROKER_COMMAND_ACK_SCHEMA,
                "message": format!("invalid JSON command: {error}")
            }),
        };
        send_text_frame(&mut stream, &message.to_string())?;
    }

    Ok(())
}

fn build_command_response(command: &Value, registry: &BrokerStreamRegistrySnapshot) -> Value {
    let request_id = command
        .get("request_id")
        .and_then(Value::as_str)
        .unwrap_or("simulator-request");
    let command_name = command.get("command").and_then(Value::as_str).unwrap_or("");

    match command_name {
        BROKER_STREAM_REGISTRY_SNAPSHOT_COMMAND => command_ack(
            request_id,
            command_name,
            true,
            "registry_snapshot",
            Some(json!({ "registry": registry })),
        ),
        "status_request" => command_ack(
            request_id,
            command_name,
            true,
            "status",
            Some(build_status_value(registry)),
        ),
        "list_streams" => command_ack(
            request_id,
            command_name,
            true,
            "streams",
            Some(json!({ "streams": registry.streams })),
        ),
        "subscribe" => {
            let stream_id = command
                .get("params")
                .and_then(|params| params.get("stream"))
                .and_then(Value::as_str);
            let accepted = stream_id
                .map(|candidate| registry.stream(candidate).is_some())
                .unwrap_or(false);
            command_ack(
                request_id,
                command_name,
                accepted,
                if accepted {
                    "subscription accepted"
                } else {
                    "unknown stream"
                },
                stream_id.map(|stream_id| {
                    json!({
                        "subscription": {
                            "stream": stream_id,
                            "mode": "simulated_ack_only"
                        }
                    })
                }),
            )
        }
        BROKER_HOST_MANIFEST_COMMAND => command_ack(
            request_id,
            command_name,
            false,
            "host manifest not served by registry simulator",
            None,
        ),
        _ => command_ack(
            request_id,
            command_name,
            false,
            "unsupported simulator command",
            None,
        ),
    }
}

fn command_ack(
    request_id: &str,
    command_name: &str,
    accepted: bool,
    message: &str,
    result: Option<Value>,
) -> Value {
    let mut value = json!({
        "type": "command_ack",
        "schema": BROKER_COMMAND_ACK_SCHEMA,
        "request_id": request_id,
        "command": command_name,
        "accepted": accepted,
        "message": message
    });
    if let Some(result) = result {
        value["result"] = result;
    }
    if !accepted {
        value["error"] = json!({
            "schema": "rusty.xr.broker.command_rejection.v1",
            "code": "simulator_rejected",
            "message": message
        });
    }
    value
}

fn build_status_value(registry: &BrokerStreamRegistrySnapshot) -> Value {
    json!({
        "type": "status",
        "brokerVersion": format!("registry-simulator-{SIMULATOR_VERSION}"),
        "activeWebSocketClients": 1,
        "commands": {
            "supported": [
                "status_request",
                "list_streams",
                "subscribe",
                BROKER_STREAM_REGISTRY_SNAPSHOT_COMMAND
            ]
        },
        "registry": {
            "schema": registry.schema,
            "broker_id": registry.broker_id,
            "revision": registry.revision,
            "modules": registry.modules.len(),
            "streams": registry.streams.len()
        },
        "clock": {
            "health": "simulated",
            "captured_unix_ns": unix_time_ns()
        }
    })
}

fn load_registry_snapshot(
    options: &SimulatorOptions,
) -> Result<BrokerStreamRegistrySnapshot, Box<dyn Error>> {
    let raw = match options.registry_path.as_ref() {
        Some(path) => fs::read_to_string(path)?,
        None => include_str!("../../../fixtures/broker-ui/synthetic-stream-registry-snapshot.json")
            .to_string(),
    };
    let mut registry: BrokerStreamRegistrySnapshot = serde_json::from_str(&raw)?;
    apply_profile(&mut registry, options.profile);
    if !registry.is_valid() {
        return Err("registry snapshot failed public model validation".into());
    }
    Ok(registry)
}

fn apply_profile(registry: &mut BrokerStreamRegistrySnapshot, profile: RegistryProfile) {
    match profile {
        RegistryProfile::Fixture => {}
        RegistryProfile::Degraded => {
            registry.revision = registry.revision.saturating_add(1);
            registry.captured_elapsed_ns = registry
                .captured_elapsed_ns
                .map(|elapsed_ns| elapsed_ns.saturating_add(1_000_000));
            let module_index = registry
                .modules
                .iter()
                .position(|module| module.module_id == "diagnostics.video_lab")
                .or_else(|| (!registry.modules.is_empty()).then_some(0));
            if let Some(module) = module_index.and_then(|index| registry.modules.get_mut(index)) {
                module.lifecycle_state = BrokerModuleLifecycleState::Degraded;
                module.revision = registry.revision;
                module.issue_codes = vec!["simulated_degraded_module".to_string()];
                for metric in &mut module.health_metrics {
                    metric.state = BrokerModuleHealthState::Warning;
                }
            }
        }
    }
}

fn read_http_request(stream: &mut TcpStream) -> io::Result<String> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 256];
    while bytes.len() < 16 * 1024 {
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..count]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8(bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
}

fn http_request_path(request: &str) -> io::Result<&str> {
    let request_line = request
        .lines()
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "empty HTTP request"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");
    if method != "GET" || !path.starts_with('/') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported HTTP request line: {request_line}"),
        ));
    }
    Ok(path)
}

fn is_websocket_request(request: &str) -> bool {
    request
        .lines()
        .any(|line| line.to_ascii_lowercase().starts_with("upgrade: websocket"))
}

fn header_value<'a>(request: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{}:", name.to_ascii_lowercase());
    request.lines().find_map(|line| {
        let lower = line.to_ascii_lowercase();
        if lower.starts_with(&prefix) {
            line.split_once(':').map(|(_, value)| value.trim())
        } else {
            None
        }
    })
}

fn write_http_json<T: serde::Serialize>(
    stream: &mut TcpStream,
    status: u16,
    value: &T,
) -> io::Result<()> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        _ => "OK",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
}

fn write_websocket_handshake(stream: &mut TcpStream, request: &str) -> io::Result<()> {
    let key = header_value(request, "sec-websocket-key").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "WebSocket request missing Sec-WebSocket-Key",
        )
    })?;
    let accept = websocket_accept_key(key);
    write!(
        stream,
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
    )
}

fn send_text_frame(stream: &mut TcpStream, text: &str) -> io::Result<()> {
    let payload = text.as_bytes();
    let mut frame = Vec::with_capacity(payload.len() + 10);
    frame.push(0x81);
    if payload.len() <= 125 {
        frame.push(payload.len() as u8);
    } else if payload.len() <= u16::MAX as usize {
        frame.push(126);
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    }
    frame.extend_from_slice(payload);
    stream.write_all(&frame)
}

fn read_text_frame(stream: &mut TcpStream) -> io::Result<Option<String>> {
    let mut header = [0_u8; 2];
    if let Err(error) = stream.read_exact(&mut header) {
        return match error.kind() {
            io::ErrorKind::UnexpectedEof | io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => {
                Ok(None)
            }
            _ => Err(error),
        };
    }

    let opcode = header[0] & 0x0f;
    if opcode == 8 {
        return Ok(None);
    }
    if opcode != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("expected text frame, got opcode {opcode}"),
        ));
    }

    let masked = (header[1] & 0x80) != 0;
    let mut length = u64::from(header[1] & 0x7f);
    if length == 126 {
        let mut bytes = [0_u8; 2];
        stream.read_exact(&mut bytes)?;
        length = u64::from(u16::from_be_bytes(bytes));
    } else if length == 127 {
        let mut bytes = [0_u8; 8];
        stream.read_exact(&mut bytes)?;
        length = u64::from_be_bytes(bytes);
    }
    if length > 1024 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "WebSocket frame was too large",
        ));
    }

    let mut mask = [0_u8; 4];
    if masked {
        stream.read_exact(&mut mask)?;
    }
    let mut payload = vec![0_u8; length as usize];
    stream.read_exact(&mut payload)?;
    if masked {
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[index % mask.len()];
        }
    }

    String::from_utf8(payload)
        .map(Some)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
}

fn websocket_accept_key(key: &str) -> String {
    let mut bytes = key.as_bytes().to_vec();
    bytes.extend_from_slice(WEBSOCKET_GUID.as_bytes());
    base64_encode(&sha1_digest(&bytes))
}

fn sha1_digest(input: &[u8]) -> [u8; 20] {
    let mut h0 = 0x67452301_u32;
    let mut h1 = 0xefcdab89_u32;
    let mut h2 = 0x98badcfe_u32;
    let mut h3 = 0x10325476_u32;
    let mut h4 = 0xc3d2e1f0_u32;

    let bit_len = (input.len() as u64) * 8;
    let mut data = input.to_vec();
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in data.chunks(64) {
        let mut words = [0_u32; 80];
        for (index, word) in words.iter_mut().take(16).enumerate() {
            let offset = index * 4;
            *word = u32::from_be_bytes([
                chunk[offset],
                chunk[offset + 1],
                chunk[offset + 2],
                chunk[offset + 3],
            ]);
        }
        for index in 16..80 {
            words[index] =
                (words[index - 3] ^ words[index - 8] ^ words[index - 14] ^ words[index - 16])
                    .rotate_left(1);
        }

        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;

        for (index, word) in words.iter().enumerate() {
            let (f, k) = match index {
                0..=19 => ((b & c) | ((!b) & d), 0x5a827999),
                20..=39 => (b ^ c ^ d, 0x6ed9eba1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1bbcdc),
                _ => (b ^ c ^ d, 0xca62c1d6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut output = [0_u8; 20];
    output[0..4].copy_from_slice(&h0.to_be_bytes());
    output[4..8].copy_from_slice(&h1.to_be_bytes());
    output[8..12].copy_from_slice(&h2.to_be_bytes());
    output[12..16].copy_from_slice(&h3.to_be_bytes());
    output[16..20].copy_from_slice(&h4.to_be_bytes());
    output
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        output.push(TABLE[(b0 >> 2) as usize] as char);
        output.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            output.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            output.push('=');
        }
    }
    output
}

fn unix_time_ns() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

fn print_help() {
    println!(
        "Usage: rusty-xr-broker-registry-simulator [--host 127.0.0.1] [--port 8765] [--profile fixture|degraded] [--registry path] [--max-connections count]"
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RegistryProfile {
    Fixture,
    Degraded,
}

impl RegistryProfile {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "fixture" => Ok(Self::Fixture),
            "degraded" => Ok(Self::Degraded),
            _ => Err(format!("unknown registry profile: {value}")),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Fixture => "fixture",
            Self::Degraded => "degraded",
        }
    }
}

#[derive(Debug)]
struct SimulatorOptions {
    host: String,
    port: u16,
    profile: RegistryProfile,
    registry_path: Option<PathBuf>,
    max_connections: Option<usize>,
    help: bool,
}

impl SimulatorOptions {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, Box<dyn Error>> {
        let mut options = Self {
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
            profile: RegistryProfile::Fixture,
            registry_path: None,
            max_connections: None,
            help: false,
        };
        let mut iterator = args.into_iter();
        while let Some(arg) = iterator.next() {
            match arg.as_str() {
                "--host" => options.host = iterator.next().ok_or("--host requires a value")?,
                "--port" => {
                    let value = iterator.next().ok_or("--port requires a value")?;
                    options.port = value.parse()?;
                    if options.port == 0 {
                        return Err("--port must be between 1 and 65535".into());
                    }
                }
                "--profile" => {
                    let value = iterator.next().ok_or("--profile requires a value")?;
                    options.profile = RegistryProfile::parse(&value)?;
                }
                "--registry" => {
                    options.registry_path = Some(PathBuf::from(
                        iterator.next().ok_or("--registry requires a path")?,
                    ));
                }
                "--max-connections" => {
                    let value = iterator
                        .next()
                        .ok_or("--max-connections requires a value")?;
                    let parsed = value.parse::<usize>()?;
                    if parsed == 0 {
                        return Err("--max-connections must be positive".into());
                    }
                    options.max_connections = Some(parsed);
                }
                "--help" | "-h" => options.help = true,
                _ => return Err(format!("unrecognized argument: {arg}").into()),
            }
        }
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_xr_broker_model::BROKER_COMMAND_SCHEMA;

    #[test]
    fn websocket_accept_matches_rfc_example() {
        assert_eq!(
            websocket_accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }

    #[test]
    fn fixture_registry_loads_and_validates() {
        let options = SimulatorOptions::parse(Vec::<String>::new()).expect("options should parse");
        let registry = load_registry_snapshot(&options).expect("registry should load");

        assert!(registry.is_valid());
        assert_eq!(registry.modules.len(), 5);
        assert!(registry.module("breath.synthetic").is_some());
    }

    #[test]
    fn degraded_profile_marks_module_health_without_breaking_links() {
        let options =
            SimulatorOptions::parse(vec!["--profile".to_string(), "degraded".to_string()])
                .expect("options should parse");
        let registry = load_registry_snapshot(&options).expect("registry should load");
        let module = registry
            .module("diagnostics.video_lab")
            .expect("video diagnostic module should exist");

        assert!(registry.is_valid());
        assert_eq!(registry.revision, 6);
        assert_eq!(module.lifecycle_state, BrokerModuleLifecycleState::Degraded);
        assert!(module
            .issue_codes
            .iter()
            .any(|issue| issue == "simulated_degraded_module"));
    }

    #[test]
    fn registry_command_ack_contains_nested_snapshot() {
        let options = SimulatorOptions::parse(Vec::<String>::new()).expect("options should parse");
        let registry = load_registry_snapshot(&options).expect("registry should load");
        let command = json!({
            "type": "command",
            "schema": BROKER_COMMAND_SCHEMA,
            "request_id": "registry-1",
            "command": BROKER_STREAM_REGISTRY_SNAPSHOT_COMMAND,
            "client_id": "test"
        });
        let ack = build_command_response(&command, &registry);

        assert_eq!(ack["type"], "command_ack");
        assert_eq!(ack["command"], BROKER_STREAM_REGISTRY_SNAPSHOT_COMMAND);
        assert_eq!(ack["accepted"], true);
        assert_eq!(
            ack["result"]["registry"]["schema"],
            "rusty.xr.broker.stream_registry_snapshot.v1"
        );
    }

    #[test]
    fn unknown_subscribe_is_rejected() {
        let options = SimulatorOptions::parse(Vec::<String>::new()).expect("options should parse");
        let registry = load_registry_snapshot(&options).expect("registry should load");
        let command = json!({
            "request_id": "sub-1",
            "command": "subscribe",
            "params": { "stream": "missing:stream" }
        });
        let ack = build_command_response(&command, &registry);

        assert_eq!(ack["accepted"], false);
        assert_eq!(ack["error"]["code"], "simulator_rejected");
    }

    #[test]
    fn status_mentions_registry_shape() {
        let options = SimulatorOptions::parse(Vec::<String>::new()).expect("options should parse");
        let registry = load_registry_snapshot(&options).expect("registry should load");
        let status = build_status_value(&registry);

        assert_eq!(status["type"], "status");
        assert_eq!(status["registry"]["modules"], 5);
        assert_eq!(
            status["commands"]["supported"][3],
            BROKER_STREAM_REGISTRY_SNAPSHOT_COMMAND
        );
    }
}
