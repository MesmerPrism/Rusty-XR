use serde_json::{json, Value};
use std::env;
use std::error::Error;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8765;
const EVENTS_PATH: &str = "/rustyxr/v1/events";
const COMMAND_SCHEMA: &str = "rusty.xr.broker.command.v1";
const LATENCY_SAMPLE_SCHEMA: &str = "rusty.xr.broker.latency_sample.v1";
const CLIENT_ID: &str = "rusty-xr-broker-client-probe";
const APP_LABEL: &str = "Rusty XR Broker Client Probe";

fn main() -> Result<(), Box<dyn Error>> {
    let options = ProbeOptions::parse(env::args().skip(1))?;
    match options.command.as_str() {
        "status" => {
            println!("{}", http_status(&options.host, options.port)?);
        }
        "capabilities" => {
            let response = send_command(&options.host, options.port, "list_capabilities", None)?;
            print_messages(&response);
        }
        "streams" => {
            let response = send_command(&options.host, options.port, "list_streams", None)?;
            print_messages(&response);
        }
        "subscribe" => {
            let stream = options
                .stream
                .as_deref()
                .ok_or("--stream <id> is required for subscribe")?;
            let response = send_command(&options.host, options.port, "subscribe", Some(stream))?;
            print_messages(&response);
        }
        "open-ui" => {
            let response = send_command(&options.host, options.port, "open_ui", None)?;
            print_messages(&response);
        }
        "close-ui" => {
            let response = send_command(&options.host, options.port, "close_ui", None)?;
            print_messages(&response);
        }
        "sample" => {
            let response = send_latency_sample(&options.host, options.port, options.subscribe)?;
            print_messages(&response);
        }
        _ => {
            return Err(format!(
                "unknown command '{}'; use status, capabilities, streams, subscribe, open-ui, close-ui, or sample",
                options.command
            )
            .into());
        }
    }

    Ok(())
}

fn http_status(host: &str, port: u16) -> io::Result<String> {
    let mut stream = TcpStream::connect((host, port))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    write!(
        stream,
        "GET /status HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n"
    )?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    let (_, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "HTTP response had no body"))?;
    Ok(body.to_string())
}

fn send_command(
    host: &str,
    port: u16,
    command: &str,
    stream_id: Option<&str>,
) -> io::Result<Vec<Value>> {
    let mut socket = connect_websocket(host, port)?;
    let mut messages = vec![read_text_frame_json(&mut socket)?];
    let command = build_command_json(command, stream_id);
    send_text_frame(&mut socket, command.to_string().as_bytes())?;
    messages.push(read_text_frame_json(&mut socket)?);
    Ok(messages)
}

fn send_latency_sample(host: &str, port: u16, subscribe: bool) -> io::Result<Vec<Value>> {
    let mut socket = connect_websocket(host, port)?;
    let mut messages = vec![read_text_frame_json(&mut socket)?];

    if subscribe {
        let subscribe_command = build_command_json("subscribe", Some("latency:sample"));
        send_text_frame(&mut socket, subscribe_command.to_string().as_bytes())?;
        messages.push(read_text_frame_json(&mut socket)?);
    }

    let sample = build_latency_sample_json(next_sequence_id());
    send_text_frame(&mut socket, sample.to_string().as_bytes())?;
    messages.push(read_text_frame_json(&mut socket)?);
    if subscribe {
        messages.push(read_text_frame_json(&mut socket)?);
    }

    Ok(messages)
}

fn connect_websocket(host: &str, port: u16) -> io::Result<TcpStream> {
    let mut stream = TcpStream::connect((host, port))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    write!(
        stream,
        "GET {EVENTS_PATH} HTTP/1.1\r\n\
         Host: {host}:{port}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
         Sec-WebSocket-Version: 13\r\n\
         \r\n"
    )?;

    let headers = read_http_headers(&mut stream)?;
    if !headers.starts_with("HTTP/1.1 101") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("broker did not accept WebSocket upgrade: {headers}"),
        ));
    }

    Ok(stream)
}

fn read_http_headers(stream: &mut TcpStream) -> io::Result<String> {
    let mut bytes = Vec::new();
    let mut one = [0u8; 1];
    while stream.read(&mut one)? == 1 {
        bytes.push(one[0]);
        if bytes.ends_with(b"\r\n\r\n") {
            break;
        }
        if bytes.len() > 16 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "HTTP header block was too large",
            ));
        }
    }

    String::from_utf8(bytes)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}

fn send_text_frame(stream: &mut TcpStream, payload: &[u8]) -> io::Result<()> {
    let frame = encode_client_text_frame(payload);
    stream.write_all(&frame)
}

fn encode_client_text_frame(payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(payload.len() + 14);
    frame.push(0x81);
    if payload.len() <= 125 {
        frame.push(0x80 | payload.len() as u8);
    } else if payload.len() <= u16::MAX as usize {
        frame.push(0x80 | 126);
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        frame.push(0x80 | 127);
        frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    }

    let mask = [0x12, 0x34, 0x56, 0x78];
    frame.extend_from_slice(&mask);
    for (index, byte) in payload.iter().enumerate() {
        frame.push(byte ^ mask[index % mask.len()]);
    }

    frame
}

fn read_text_frame_json(stream: &mut TcpStream) -> io::Result<Value> {
    let text = read_text_frame(stream)?;
    serde_json::from_str(&text)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}

fn read_text_frame(stream: &mut TcpStream) -> io::Result<String> {
    let mut header = [0u8; 2];
    stream.read_exact(&mut header)?;
    let opcode = header[0] & 0x0f;
    if opcode == 8 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "broker closed WebSocket",
        ));
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
        let mut extended = [0u8; 2];
        stream.read_exact(&mut extended)?;
        length = u64::from(u16::from_be_bytes(extended));
    } else if length == 127 {
        let mut extended = [0u8; 8];
        stream.read_exact(&mut extended)?;
        length = u64::from_be_bytes(extended);
    }

    if length > 1024 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "WebSocket frame was too large",
        ));
    }

    let mut mask = [0u8; 4];
    if masked {
        stream.read_exact(&mut mask)?;
    }

    let mut payload = vec![0u8; length as usize];
    stream.read_exact(&mut payload)?;
    if masked {
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[index % mask.len()];
        }
    }

    String::from_utf8(payload)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}

fn build_command_json(command: &str, stream_id: Option<&str>) -> Value {
    let mut message = json!({
        "type": "command",
        "schema": COMMAND_SCHEMA,
        "request_id": format!("rust-{}", next_sequence_id()),
        "command": command,
        "client_id": CLIENT_ID,
        "app_label": APP_LABEL,
        "app_version": env!("CARGO_PKG_VERSION")
    });

    if let Some(stream_id) = stream_id {
        message["params"] = json!({
            "stream": stream_id
        });
    }

    message
}

fn build_latency_sample_json(sequence_id: u128) -> Value {
    json!({
        "type": "latency_sample",
        "schema": LATENCY_SAMPLE_SCHEMA,
        "sequence_id": sequence_id,
        "path": "rust_probe",
        "client_send_time_unix_ns": unix_time_ns(),
        "payload_size_bytes": 128,
        "client_id": CLIENT_ID,
        "app_label": APP_LABEL,
        "app_version": env!("CARGO_PKG_VERSION")
    })
}

fn print_messages(messages: &[Value]) {
    for message in messages {
        println!(
            "{}",
            serde_json::to_string_pretty(message).expect("json value should serialize")
        );
    }
}

fn next_sequence_id() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn unix_time_ns() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[derive(Debug)]
struct ProbeOptions {
    command: String,
    host: String,
    port: u16,
    stream: Option<String>,
    subscribe: bool,
}

impl ProbeOptions {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, Box<dyn Error>> {
        let mut command = None;
        let mut host = DEFAULT_HOST.to_string();
        let mut port = DEFAULT_PORT;
        let mut stream = None;
        let mut subscribe = false;
        let mut iterator = args.into_iter();

        while let Some(arg) = iterator.next() {
            match arg.as_str() {
                "--host" => {
                    host = iterator.next().ok_or("--host requires a value")?;
                }
                "--port" => {
                    let raw = iterator.next().ok_or("--port requires a value")?;
                    port = raw.parse()?;
                }
                "--stream" => {
                    stream = Some(iterator.next().ok_or("--stream requires a value")?);
                }
                "--subscribe" => {
                    subscribe = true;
                }
                _ if command.is_none() => {
                    command = Some(arg);
                }
                _ => {
                    return Err(format!("unrecognized argument '{arg}'").into());
                }
            }
        }

        Ok(Self {
            command: command.unwrap_or_else(|| "status".to_string()),
            host,
            port,
            stream,
            subscribe,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribe_command_uses_broker_envelope() {
        let command = build_command_json("subscribe", Some("latency:sample"));
        assert_eq!(command["type"], "command");
        assert_eq!(command["schema"], COMMAND_SCHEMA);
        assert_eq!(command["command"], "subscribe");
        assert_eq!(command["client_id"], CLIENT_ID);
        assert_eq!(command["params"]["stream"], "latency:sample");
    }

    #[test]
    fn open_ui_command_uses_broker_envelope() {
        let command = build_command_json("open_ui", None);
        assert_eq!(command["type"], "command");
        assert_eq!(command["schema"], COMMAND_SCHEMA);
        assert_eq!(command["command"], "open_ui");
        assert_eq!(command["client_id"], CLIENT_ID);
        assert!(command.get("params").is_none());
    }

    #[test]
    fn close_ui_command_uses_broker_envelope() {
        let command = build_command_json("close_ui", None);
        assert_eq!(command["type"], "command");
        assert_eq!(command["schema"], COMMAND_SCHEMA);
        assert_eq!(command["command"], "close_ui");
        assert_eq!(command["client_id"], CLIENT_ID);
        assert!(command.get("params").is_none());
    }

    #[test]
    fn latency_sample_uses_probe_metadata() {
        let sample = build_latency_sample_json(7);
        assert_eq!(sample["type"], "latency_sample");
        assert_eq!(sample["schema"], LATENCY_SAMPLE_SCHEMA);
        assert_eq!(sample["sequence_id"], 7);
        assert_eq!(sample["path"], "rust_probe");
        assert_eq!(sample["payload_size_bytes"], 128);
    }

    #[test]
    fn client_text_frame_is_masked() {
        let frame = encode_client_text_frame(b"hello");
        assert_eq!(frame[0], 0x81);
        assert_eq!(frame[1], 0x80 | 5);
        assert_eq!(&frame[2..6], &[0x12, 0x34, 0x56, 0x78]);
        let decoded: Vec<u8> = frame[6..]
            .iter()
            .enumerate()
            .map(|(index, byte)| byte ^ frame[2 + (index % 4)])
            .collect();
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn options_default_to_forwarded_broker() {
        let options = ProbeOptions::parse(Vec::<String>::new()).expect("options should parse");
        assert_eq!(options.command, "status");
        assert_eq!(options.host, DEFAULT_HOST);
        assert_eq!(options.port, DEFAULT_PORT);
    }
}
