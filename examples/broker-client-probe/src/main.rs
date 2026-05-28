use rusty_xr_broker_model::{
    BROKER_COMMAND_SCHEMA, BROKER_LATENCY_SAMPLE_SCHEMA, BROKER_STREAM_REGISTRY_SNAPSHOT_COMMAND,
    BROKER_STREAM_REGISTRY_SNAPSHOT_HTTP_PATH, BROKER_TRANSPORT_SECURITY_POLICY_SCHEMA,
    BROKER_TRANSPORT_SESSION_OFFER_SCHEMA, STREAM_LATENCY_SAMPLE,
};
use serde_json::{json, Value};
use std::env;
use std::error::Error;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8765;
const EVENTS_PATH: &str = "/rustyxr/v1/events";
const CLIENT_ID: &str = "rusty-xr-broker-client-probe";
const APP_LABEL: &str = "Rusty XR Broker Client Probe";
const DEFAULT_TRANSPORT_SESSION_ID: &str = "probe-transport-session";
const DEFAULT_H264_DECODE_SESSION_ID: &str = "probe-h264-decode-session";
const DEFAULT_SYNTHETIC_H264_SESSION_ID: &str = "probe-synthetic-h264-session";

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
        "registry" => {
            let response = send_command(
                &options.host,
                options.port,
                BROKER_STREAM_REGISTRY_SNAPSHOT_COMMAND,
                None,
            )?;
            print_messages(&response);
        }
        "registry-http" => {
            println!("{}", http_registry_snapshot(&options.host, options.port)?);
        }
        "camera-provider" => {
            let response = send_command(
                &options.host,
                options.port,
                "camera_provider.get_status",
                None,
            )?;
            print_messages(&response);
        }
        "projection-profile" => {
            let response = send_command(
                &options.host,
                options.port,
                "camera_provider.get_projection_profile",
                None,
            )?;
            print_messages(&response);
        }
        "app-camera-probe" => {
            let response = send_command(
                &options.host,
                options.port,
                "camera_provider.run_app_camera_probe",
                Some(build_app_camera_probe_params_json(&options)),
            )?;
            print_messages(&response);
        }
        "synthetic-h264-stream" => {
            let response = send_command(
                &options.host,
                options.port,
                "media.start_synthetic_h264_stream",
                Some(build_synthetic_h264_stream_params_json(&options)),
            )?;
            print_messages(&response);
        }
        "app-camera-h264-decode-probe" => {
            let session_id = options
                .session
                .as_deref()
                .unwrap_or(DEFAULT_H264_DECODE_SESSION_ID);
            let response = send_command(
                &options.host,
                options.port,
                "camera_provider.run_app_camera_h264_decode_probe",
                Some(build_h264_decode_probe_params_json(session_id)),
            )?;
            print_messages(&response);
        }
        "shell-helper-status" => {
            let response =
                send_command(&options.host, options.port, "shell_helper.get_status", None)?;
            print_messages(&response);
        }
        "video-lab-status" => {
            let response = send_command(&options.host, options.port, "video_lab.get_status", None)?;
            print_messages(&response);
        }
        "video-lab-scorecard" => {
            let response =
                send_command(&options.host, options.port, "video_lab.get_scorecard", None)?;
            print_messages(&response);
        }
        "shell-helper-report-stub" => {
            let response = send_command(
                &options.host,
                options.port,
                "shell_helper.report_status",
                Some(json!({
                    "connected": true,
                    "helper_version": "stub",
                    "uid": "shell",
                    "capabilities": [
                        "shell.display.list",
                        "shell.camera.list"
                    ],
                    "active_streams": [],
                    "last_error": ""
                })),
            )?;
            print_messages(&response);
        }
        "video-manifest-stub" => {
            let response = send_command(
                &options.host,
                options.port,
                "video_lab.register_encoded_stream_manifest",
                Some(json!({
                    "schema": "rusty.xr.video_lab.encoded_stream_manifest.v1",
                    "stream_id": "synthetic_encoded_h264",
                    "session_id": "synthetic-h264-session",
                    "source": "synthetic",
                    "transport": "metadata_only",
                    "payload_transport": "pending_binary",
                    "mime_type": "video/avc",
                    "codec": "h264",
                    "decoder_target": "surface",
                    "width": 1280,
                    "height": 720,
                    "frame_rate_hz": 30,
                    "bitrate_bps": 4000000
                })),
            )?;
            print_messages(&response);
        }
        "video-sample-meta-stub" => {
            let response = send_command(
                &options.host,
                options.port,
                "video_lab.record_encoded_sample_metadata",
                Some(json!({
                    "schema": "rusty.xr.video_lab.encoded_sample_metadata.v1",
                    "stream_id": "synthetic_encoded_h264",
                    "session_id": "synthetic-h264-session",
                    "sequence_id": next_sequence_id(),
                    "source": "synthetic",
                    "transport": "metadata_only",
                    "payload_transport": "pending_binary",
                    "mime_type": "video/avc",
                    "codec": "h264",
                    "encoded_size_bytes": 0,
                    "key_frame": true,
                    "pts_us": 0,
                    "dts_us": 0,
                    "source_time_unix_ns": unix_time_ns(),
                    "source_time_elapsed_ns": 0
                })),
            )?;
            print_messages(&response);
        }
        "video-metric-stub" => {
            let response = send_command(
                &options.host,
                options.port,
                "video_lab.record_metric_sample",
                Some(json!({
                    "schema": "rusty.xr.video_lab.metric_sample.v1",
                    "stream_id": "synthetic_encoded_h264",
                    "source": "synthetic",
                    "transport": "metadata_only",
                    "codec": "none",
                    "sequence_id": next_sequence_id(),
                    "source_time_unix_ns": unix_time_ns(),
                    "client_receive_time_unix_ns": unix_time_ns(),
                    "decoder_output_time_unix_ns": 0,
                    "texture_available_time_unix_ns": 0,
                    "xr_submit_time_unix_ns": 0,
                    "dropped_frames": 0,
                    "stale_frames": 0,
                    "queue_depth": 0,
                    "width": 0,
                    "height": 0
                })),
            )?;
            print_messages(&response);
        }
        "h264-proxy-probe" => {
            let response = send_command(
                &options.host,
                options.port,
                "media.run_h264_tcp_proxy_probe",
                Some(build_h264_proxy_probe_params_json()),
            )?;
            print_messages(&response);
        }
        "transport-capabilities" => {
            let response = send_command(
                &options.host,
                options.port,
                "transport.describe_capabilities",
                None,
            )?;
            print_messages(&response);
        }
        "transport-create-session" => {
            let session_id = options
                .session
                .as_deref()
                .unwrap_or(DEFAULT_TRANSPORT_SESSION_ID);
            let response = send_command(
                &options.host,
                options.port,
                "transport.create_session",
                Some(build_transport_session_offer_json(session_id)),
            )?;
            print_messages(&response);
        }
        "transport-list-sessions" => {
            let response =
                send_command(&options.host, options.port, "transport.list_sessions", None)?;
            print_messages(&response);
        }
        "transport-get-session" => {
            let session_id = options
                .session
                .as_deref()
                .ok_or("--session <id> is required for transport-get-session")?;
            let response = send_command(
                &options.host,
                options.port,
                "transport.get_session",
                Some(json!({ "session_id": session_id })),
            )?;
            print_messages(&response);
        }
        "transport-close-session" => {
            let session_id = options
                .session
                .as_deref()
                .ok_or("--session <id> is required for transport-close-session")?;
            let response = send_command(
                &options.host,
                options.port,
                "transport.close_session",
                Some(json!({
                    "session_id": session_id,
                    "reason": "closed_by_probe"
                })),
            )?;
            print_messages(&response);
        }
        "subscribe" => {
            let stream = options
                .stream
                .as_deref()
                .ok_or("--stream <id> is required for subscribe")?;
            let response = send_command(
                &options.host,
                options.port,
                "subscribe",
                Some(json!({ "stream": stream })),
            )?;
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
                "unknown command '{}'; use status, capabilities, streams, registry, registry-http, camera-provider, projection-profile, app-camera-probe, synthetic-h264-stream, app-camera-h264-decode-probe, shell-helper-status, shell-helper-report-stub, video-lab-status, video-lab-scorecard, video-manifest-stub, video-sample-meta-stub, video-metric-stub, h264-proxy-probe, transport-capabilities, transport-create-session, transport-list-sessions, transport-get-session, transport-close-session, subscribe, open-ui, close-ui, or sample",
                options.command
            )
            .into());
        }
    }

    Ok(())
}

fn http_status(host: &str, port: u16) -> io::Result<String> {
    http_get_body(host, port, "/status")
}

fn http_registry_snapshot(host: &str, port: u16) -> io::Result<String> {
    http_get_body(host, port, BROKER_STREAM_REGISTRY_SNAPSHOT_HTTP_PATH)
}

fn http_get_body(host: &str, port: u16, path: &str) -> io::Result<String> {
    let mut stream = TcpStream::connect((host, port))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.write_all(build_http_get_request(host, port, path)?.as_bytes())?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    let (_, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "HTTP response had no body"))?;
    Ok(body.to_string())
}

fn build_http_get_request(host: &str, port: u16, path: &str) -> io::Result<String> {
    if !path.starts_with('/') || path.contains('\r') || path.contains('\n') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid HTTP path: {path}"),
        ));
    }
    Ok(format!(
        "GET {path} HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n"
    ))
}

fn send_command(
    host: &str,
    port: u16,
    command: &str,
    params: Option<Value>,
) -> io::Result<Vec<Value>> {
    let mut socket = connect_websocket(host, port)?;
    let mut messages = vec![read_text_frame_json(&mut socket)?];
    let command = build_command_json(command, params);
    send_text_frame(&mut socket, command.to_string().as_bytes())?;
    messages.push(read_text_frame_json(&mut socket)?);
    Ok(messages)
}

fn send_latency_sample(host: &str, port: u16, subscribe: bool) -> io::Result<Vec<Value>> {
    let mut socket = connect_websocket(host, port)?;
    let mut messages = vec![read_text_frame_json(&mut socket)?];

    if subscribe {
        let subscribe_command = build_command_json(
            "subscribe",
            Some(json!({ "stream": STREAM_LATENCY_SAMPLE })),
        );
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

fn build_command_json(command: &str, params: Option<Value>) -> Value {
    let mut message = json!({
        "type": "command",
        "schema": BROKER_COMMAND_SCHEMA,
        "request_id": format!("rust-{}", next_sequence_id()),
        "command": command,
        "client_id": CLIENT_ID,
        "app_label": APP_LABEL,
        "app_version": env!("CARGO_PKG_VERSION")
    });

    if let Some(params) = params {
        message["params"] = params;
    }

    message
}

fn build_latency_sample_json(sequence_id: u128) -> Value {
    json!({
        "type": "latency_sample",
        "schema": BROKER_LATENCY_SAMPLE_SCHEMA,
        "sequence_id": sequence_id,
        "path": "rust_probe",
        "client_send_time_unix_ns": unix_time_ns(),
        "payload_size_bytes": 128,
        "client_id": CLIENT_ID,
        "app_label": APP_LABEL,
        "app_version": env!("CARGO_PKG_VERSION")
    })
}

fn build_transport_session_offer_json(session_id: &str) -> Value {
    json!({
        "schema": BROKER_TRANSPORT_SESSION_OFFER_SCHEMA,
        "session_id": session_id,
        "client_id": CLIENT_ID,
        "requested_transports": ["AdbForwardedTcp", "Tcp", "WebSocket"],
        "streams": [{
            "stream_id": "camera.left.h264",
            "stream_kind": "Media",
            "direction": "ProducerToConsumer",
            "payload_kind": "H264",
            "payload_schema": "video/h264",
            "codec": "H264",
            "reliability": "LossTolerant",
            "ordered": false,
            "nominal_rate_hz": 60.0,
            "target_latency_ms": 35.0,
            "max_payload_bytes": 65507
        }],
        "security": {
            "schema": BROKER_TRANSPORT_SECURITY_POLICY_SCHEMA,
            "mode": "LoopbackOnly",
            "non_loopback_allowed": false,
            "pairing_token_required": false,
            "expires_elapsed_ns": null,
            "capability_scope": ["camera_provider.start_app_camera_h264_stream"]
        },
        "target_latency_ms": 35.0
    })
}

fn build_h264_decode_probe_params_json(session_id: &str) -> Value {
    json!({
        "session_id": session_id,
        "preferred_width": 720,
        "preferred_height": 480,
        "capture_ms": 900,
        "max_packets": 12,
        "bitrate_bps": 1_000_000,
        "decode_timeout_ms": 5000
    })
}

fn build_h264_proxy_probe_params_json() -> Value {
    json!({
        "width": 64,
        "height": 64,
        "packet_count": 4,
        "packet_bytes": 96,
        "timeout_ms": 10000
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
    session: Option<String>,
    camera_id: Option<String>,
    frame_output_dir: Option<String>,
    persist_frame: bool,
    jpeg_quality: u8,
    device_port: Option<u16>,
    host_port: Option<u16>,
    width: Option<u32>,
    height: Option<u32>,
    capture_ms: Option<u32>,
    max_packets: Option<u32>,
    bitrate_bps: Option<u32>,
    frame_rate_hz: Option<u32>,
    accept_timeout_ms: Option<u32>,
    synthetic_pattern: Option<String>,
    synthetic_image_path: Option<String>,
    projection_profile: Option<String>,
    subscribe: bool,
}

impl ProbeOptions {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, Box<dyn Error>> {
        let mut command = None;
        let mut host = DEFAULT_HOST.to_string();
        let mut port = DEFAULT_PORT;
        let mut stream = None;
        let mut session = None;
        let mut camera_id = None;
        let mut frame_output_dir = None;
        let mut persist_frame = false;
        let mut jpeg_quality = 95;
        let mut device_port = None;
        let mut host_port = None;
        let mut width = None;
        let mut height = None;
        let mut capture_ms = None;
        let mut max_packets = None;
        let mut bitrate_bps = None;
        let mut frame_rate_hz = None;
        let mut accept_timeout_ms = None;
        let mut synthetic_pattern = None;
        let mut synthetic_image_path = None;
        let mut projection_profile = None;
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
                "--session" => {
                    session = Some(iterator.next().ok_or("--session requires a value")?);
                }
                "--camera-id" => {
                    camera_id = Some(iterator.next().ok_or("--camera-id requires a value")?);
                }
                "--persist-frame" => {
                    persist_frame = true;
                }
                "--frame-output-dir" => {
                    persist_frame = true;
                    frame_output_dir = Some(
                        iterator
                            .next()
                            .ok_or("--frame-output-dir requires a value")?,
                    );
                }
                "--jpeg-quality" => {
                    let raw = iterator.next().ok_or("--jpeg-quality requires a value")?;
                    jpeg_quality = raw.parse()?;
                    if !(1..=100).contains(&jpeg_quality) {
                        return Err("--jpeg-quality must be between 1 and 100".into());
                    }
                }
                "--device-port" => {
                    device_port = Some(parse_port_arg("--device-port", iterator.next())?);
                }
                "--host-port" => {
                    host_port = Some(parse_port_arg("--host-port", iterator.next())?);
                }
                "--width" => {
                    width = Some(parse_positive_u32_arg("--width", iterator.next())?);
                }
                "--height" => {
                    height = Some(parse_positive_u32_arg("--height", iterator.next())?);
                }
                "--capture-ms" => {
                    capture_ms = Some(parse_positive_u32_arg("--capture-ms", iterator.next())?);
                }
                "--max-packets" => {
                    max_packets = Some(parse_positive_u32_arg("--max-packets", iterator.next())?);
                }
                "--bitrate-bps" => {
                    bitrate_bps = Some(parse_positive_u32_arg("--bitrate-bps", iterator.next())?);
                }
                "--frame-rate-hz" => {
                    frame_rate_hz =
                        Some(parse_positive_u32_arg("--frame-rate-hz", iterator.next())?);
                }
                "--accept-timeout-ms" => {
                    accept_timeout_ms = Some(parse_positive_u32_arg(
                        "--accept-timeout-ms",
                        iterator.next(),
                    )?);
                }
                "--synthetic-pattern" => {
                    synthetic_pattern = Some(
                        iterator
                            .next()
                            .ok_or("--synthetic-pattern requires a value")?,
                    );
                }
                "--synthetic-image-path" => {
                    synthetic_image_path = Some(
                        iterator
                            .next()
                            .ok_or("--synthetic-image-path requires a value")?,
                    );
                }
                "--projection-profile" => {
                    projection_profile = Some(
                        iterator
                            .next()
                            .ok_or("--projection-profile requires a value")?,
                    );
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
            session,
            camera_id,
            frame_output_dir,
            persist_frame,
            jpeg_quality,
            device_port,
            host_port,
            width,
            height,
            capture_ms,
            max_packets,
            bitrate_bps,
            frame_rate_hz,
            accept_timeout_ms,
            synthetic_pattern,
            synthetic_image_path,
            projection_profile,
            subscribe,
        })
    }
}

fn parse_port_arg(name: &str, raw: Option<String>) -> Result<u16, Box<dyn Error>> {
    let value: u16 = raw
        .ok_or_else(|| format!("{name} requires a value"))?
        .parse()?;
    if value == 0 {
        return Err(format!("{name} must be between 1 and 65535").into());
    }
    Ok(value)
}

fn parse_positive_u32_arg(name: &str, raw: Option<String>) -> Result<u32, Box<dyn Error>> {
    let value: u32 = raw
        .ok_or_else(|| format!("{name} requires a value"))?
        .parse()?;
    if value == 0 {
        return Err(format!("{name} must be positive").into());
    }
    Ok(value)
}

fn build_app_camera_probe_params_json(options: &ProbeOptions) -> Value {
    let mut params = json!({
        "max_attempts": if options.camera_id.is_some() { 1 } else { 3 },
        "preferred_width": options.width.unwrap_or(640),
        "preferred_height": options.height.unwrap_or(480),
        "capture_timeout_ms": 2500
    });
    if let Some(camera_id) = options.camera_id.as_deref() {
        params["camera_id"] = json!(camera_id);
    }
    if options.persist_frame {
        params["persist_frame"] = json!(true);
        params["jpeg_quality"] = json!(options.jpeg_quality);
    }
    if let Some(frame_output_dir) = options.frame_output_dir.as_deref() {
        params["frame_output_dir"] = json!(frame_output_dir);
    }
    params
}

fn build_synthetic_h264_stream_params_json(options: &ProbeOptions) -> Value {
    let session_id = options
        .session
        .as_deref()
        .unwrap_or(DEFAULT_SYNTHETIC_H264_SESSION_ID);
    let width = options.width.unwrap_or(720);
    let height = options.height.unwrap_or(480);
    let synthetic_pattern =
        options
            .synthetic_pattern
            .as_deref()
            .unwrap_or(if options.synthetic_image_path.is_some() {
                "image-file"
            } else {
                "diagnostic-grid"
            });
    let projection_profile = options
        .projection_profile
        .as_deref()
        .unwrap_or("full-frame-diagnostic");

    let mut params = json!({
        "session_id": session_id,
        "device_port": options.device_port.unwrap_or(8879),
        "host_port": options.host_port.unwrap_or(18879),
        "preferred_width": width,
        "preferred_height": height,
        "content_width": width,
        "content_height": height,
        "capture_ms": options.capture_ms.unwrap_or(10_000),
        "max_packets": options.max_packets.unwrap_or(300),
        "accept_timeout_ms": options.accept_timeout_ms.unwrap_or(60_000),
        "bitrate_bps": options.bitrate_bps.unwrap_or(2_000_000),
        "frame_rate_hz": options.frame_rate_hz.unwrap_or(30),
        "live_stream": true,
        "synthetic_pattern": synthetic_pattern,
        "synthetic_projection_profile": projection_profile,
        "projection_geometry_profile": projection_profile
    });
    if let Some(image_path) = options.synthetic_image_path.as_deref() {
        params["synthetic_image_path"] = json!(image_path);
    }
    if let Some(camera_id) = options.camera_id.as_deref() {
        params["camera_id"] = json!(camera_id);
    }
    params
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribe_command_uses_broker_envelope() {
        let command = build_command_json("subscribe", Some(json!({ "stream": "latency:sample" })));
        assert_eq!(command["type"], "command");
        assert_eq!(command["schema"], BROKER_COMMAND_SCHEMA);
        assert_eq!(command["command"], "subscribe");
        assert_eq!(command["client_id"], CLIENT_ID);
        assert_eq!(command["params"]["stream"], STREAM_LATENCY_SAMPLE);
    }

    #[test]
    fn registry_http_request_uses_public_path_constant() {
        let request =
            build_http_get_request("127.0.0.1", 8765, BROKER_STREAM_REGISTRY_SNAPSHOT_HTTP_PATH)
                .expect("registry request should build");

        assert!(request.starts_with("GET /stream_registry/snapshot HTTP/1.1"));
        assert!(request.contains("Host: 127.0.0.1:8765"));
        assert!(build_http_get_request("127.0.0.1", 8765, "bad-path").is_err());
    }

    #[test]
    fn projection_profile_command_uses_broker_envelope() {
        let command = build_command_json("camera_provider.get_projection_profile", None);
        assert_eq!(command["type"], "command");
        assert_eq!(command["schema"], BROKER_COMMAND_SCHEMA);
        assert_eq!(command["command"], "camera_provider.get_projection_profile");
        assert_eq!(command["client_id"], CLIENT_ID);
        assert!(command.get("params").is_none());
    }

    #[test]
    fn shell_helper_report_can_send_stub_capabilities() {
        let command = build_command_json(
            "shell_helper.report_status",
            Some(json!({
                "connected": true,
                "helper_version": "stub",
                "uid": "shell",
                "capabilities": ["shell.display.list"]
            })),
        );
        assert_eq!(command["type"], "command");
        assert_eq!(command["command"], "shell_helper.report_status");
        assert_eq!(command["params"]["uid"], "shell");
        assert_eq!(command["params"]["capabilities"][0], "shell.display.list");
    }

    #[test]
    fn video_metric_stub_uses_metric_schema() {
        let command = build_command_json(
            "video_lab.record_metric_sample",
            Some(json!({
                "schema": "rusty.xr.video_lab.metric_sample.v1",
                "stream_id": "synthetic_encoded_h264",
                "source": "synthetic",
                "transport": "metadata_only",
                "codec": "none",
                "sequence_id": 1
            })),
        );
        assert_eq!(command["type"], "command");
        assert_eq!(command["command"], "video_lab.record_metric_sample");
        assert_eq!(
            command["params"]["schema"],
            "rusty.xr.video_lab.metric_sample.v1"
        );
        assert_eq!(command["params"]["stream_id"], "synthetic_encoded_h264");
    }

    #[test]
    fn video_scorecard_command_uses_broker_envelope() {
        let command = build_command_json("video_lab.get_scorecard", None);
        assert_eq!(command["type"], "command");
        assert_eq!(command["schema"], BROKER_COMMAND_SCHEMA);
        assert_eq!(command["command"], "video_lab.get_scorecard");
        assert!(command.get("params").is_none());
    }

    #[test]
    fn video_manifest_stub_uses_manifest_schema() {
        let command = build_command_json(
            "video_lab.register_encoded_stream_manifest",
            Some(json!({
                "schema": "rusty.xr.video_lab.encoded_stream_manifest.v1",
                "stream_id": "synthetic_encoded_h264",
                "session_id": "synthetic-h264-session",
                "mime_type": "video/avc",
                "width": 1280,
                "height": 720
            })),
        );
        assert_eq!(command["type"], "command");
        assert_eq!(
            command["command"],
            "video_lab.register_encoded_stream_manifest"
        );
        assert_eq!(
            command["params"]["schema"],
            "rusty.xr.video_lab.encoded_stream_manifest.v1"
        );
        assert_eq!(command["params"]["mime_type"], "video/avc");
    }

    #[test]
    fn video_sample_metadata_stub_uses_sample_schema() {
        let command = build_command_json(
            "video_lab.record_encoded_sample_metadata",
            Some(json!({
                "schema": "rusty.xr.video_lab.encoded_sample_metadata.v1",
                "stream_id": "synthetic_encoded_h264",
                "session_id": "synthetic-h264-session",
                "sequence_id": 1,
                "encoded_size_bytes": 0
            })),
        );
        assert_eq!(command["type"], "command");
        assert_eq!(
            command["command"],
            "video_lab.record_encoded_sample_metadata"
        );
        assert_eq!(
            command["params"]["schema"],
            "rusty.xr.video_lab.encoded_sample_metadata.v1"
        );
        assert_eq!(command["params"]["session_id"], "synthetic-h264-session");
    }

    #[test]
    fn transport_session_offer_uses_clean_room_schemas() {
        let offer = build_transport_session_offer_json("transport-test");
        assert_eq!(offer["schema"], BROKER_TRANSPORT_SESSION_OFFER_SCHEMA);
        assert_eq!(offer["session_id"], "transport-test");
        assert_eq!(
            offer["security"]["schema"],
            BROKER_TRANSPORT_SECURITY_POLICY_SCHEMA
        );
        assert_eq!(offer["security"]["mode"], "LoopbackOnly");
        assert_eq!(offer["streams"][0]["payload_kind"], "H264");
        assert_eq!(offer["streams"][0]["codec"], "H264");
        assert_eq!(offer["requested_transports"][0], "AdbForwardedTcp");
    }

    #[test]
    fn transport_get_session_uses_session_param() {
        let command = build_command_json(
            "transport.get_session",
            Some(json!({ "session_id": "transport-test" })),
        );
        assert_eq!(command["type"], "command");
        assert_eq!(command["command"], "transport.get_session");
        assert_eq!(command["params"]["session_id"], "transport-test");
    }

    #[test]
    fn options_accept_session_id() {
        let options = ProbeOptions::parse(vec![
            "transport-get-session".to_string(),
            "--session".to_string(),
            "transport-test".to_string(),
        ])
        .expect("options should parse");
        assert_eq!(options.command, "transport-get-session");
        assert_eq!(options.session.as_deref(), Some("transport-test"));
    }

    #[test]
    fn h264_decode_probe_uses_session_param() {
        let params = build_h264_decode_probe_params_json("transport-test");
        assert_eq!(params["session_id"], "transport-test");
        assert_eq!(params["preferred_width"], 720);
        assert_eq!(params["preferred_height"], 480);
        assert_eq!(params["max_packets"], 12);

        let command = build_command_json(
            "camera_provider.run_app_camera_h264_decode_probe",
            Some(params),
        );
        assert_eq!(
            command["command"],
            "camera_provider.run_app_camera_h264_decode_probe"
        );
        assert_eq!(command["params"]["session_id"], "transport-test");
    }

    #[test]
    fn h264_proxy_probe_uses_bounded_synthetic_source() {
        let command = build_command_json(
            "media.run_h264_tcp_proxy_probe",
            Some(build_h264_proxy_probe_params_json()),
        );
        assert_eq!(command["command"], "media.run_h264_tcp_proxy_probe");
        assert_eq!(command["params"]["packet_count"], 4);
        assert_eq!(command["params"]["packet_bytes"], 96);
    }

    #[test]
    fn open_ui_command_uses_broker_envelope() {
        let command = build_command_json("open_ui", None);
        assert_eq!(command["type"], "command");
        assert_eq!(command["schema"], BROKER_COMMAND_SCHEMA);
        assert_eq!(command["command"], "open_ui");
        assert_eq!(command["client_id"], CLIENT_ID);
        assert!(command.get("params").is_none());
    }

    #[test]
    fn close_ui_command_uses_broker_envelope() {
        let command = build_command_json("close_ui", None);
        assert_eq!(command["type"], "command");
        assert_eq!(command["schema"], BROKER_COMMAND_SCHEMA);
        assert_eq!(command["command"], "close_ui");
        assert_eq!(command["client_id"], CLIENT_ID);
        assert!(command.get("params").is_none());
    }

    #[test]
    fn latency_sample_uses_probe_metadata() {
        let sample = build_latency_sample_json(7);
        assert_eq!(sample["type"], "latency_sample");
        assert_eq!(sample["schema"], BROKER_LATENCY_SAMPLE_SCHEMA);
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
