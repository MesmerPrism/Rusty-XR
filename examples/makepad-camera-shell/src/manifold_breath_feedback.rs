use serde_json::{json, Value as JsonValue};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    io::{Read, Write},
    net::TcpStream,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

pub(crate) const BREATH_FEEDBACK_STREAM_ID: &str = "stream.breath.feedback_state";
pub(crate) const BREATH_FEEDBACK_RECEIVER_ID: &str = "app.makepad_camera_shell.breath_feedback";
pub(crate) const BREATH_FEEDBACK_RECEIPT_COMMAND: &str = "breath_feedback.received";
pub(crate) const BREATH_FEEDBACK_RECEIPT_SCHEMA: &str = "rusty.manifold.breath.feedback_receipt.v1";

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ManifoldBreathFeedbackConfig {
    pub enabled: bool,
    pub broker_host: String,
    pub broker_port: u16,
    pub stream_id: String,
    pub receiver_id: String,
    pub connect_timeout_ms: u64,
}

impl Default for ManifoldBreathFeedbackConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            broker_host: "127.0.0.1".to_string(),
            broker_port: 8765,
            stream_id: BREATH_FEEDBACK_STREAM_ID.to_string(),
            receiver_id: BREATH_FEEDBACK_RECEIVER_ID.to_string(),
            connect_timeout_ms: 250,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BreathFeedbackSample {
    pub stream_id: String,
    pub sequence_id: u64,
    pub sample_time_unix_ns: i64,
    pub volume01: f64,
    pub phase: String,
    pub quality: String,
    pub payload_hash: String,
}

pub(crate) struct ManifoldBreathFeedbackSubscriber {
    config: ManifoldBreathFeedbackConfig,
    stop: Arc<AtomicBool>,
}

impl ManifoldBreathFeedbackSubscriber {
    pub(crate) fn new(config: ManifoldBreathFeedbackConfig) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let worker_config = config.clone();
        thread::spawn(move || run_breath_feedback_worker(worker_config, worker_stop));
        Self { config, stop }
    }

    pub(crate) fn config(&self) -> &ManifoldBreathFeedbackConfig {
        &self.config
    }
}

impl Drop for ManifoldBreathFeedbackSubscriber {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

pub(crate) fn build_breath_feedback_subscribe_command(
    config: &ManifoldBreathFeedbackConfig,
) -> JsonValue {
    json!({
        "type": "command",
        "schema": "rusty.xr.broker.command.v1",
        "request_id": "makepad-breath-feedback-subscribe",
        "command": "subscribe",
        "params": {
            "stream": config.stream_id,
            "receiver": config.receiver_id,
        }
    })
}

pub(crate) fn parse_breath_feedback_event(
    message: &JsonValue,
    expected_stream: &str,
) -> Option<BreathFeedbackSample> {
    if message.get("type").and_then(JsonValue::as_str) != Some("stream_event") {
        return None;
    }
    let payload = message.get("payload").filter(|value| value.is_object())?;
    let stream_id = first_string(
        &[
            message.get("stream"),
            message.get("stream_id"),
            payload.get("stream"),
            payload.get("stream_id"),
        ],
        expected_stream,
    );
    if stream_id != expected_stream {
        return None;
    }
    let volume01 = first_f64(&[
        payload.get("volume01"),
        payload.get("breath_volume01"),
        payload.get("volume_01"),
    ])?;
    let phase = first_string(
        &[payload.get("phase"), payload.get("breath_phase")],
        "unknown",
    );
    let quality = first_string(&[payload.get("quality")], "unknown");
    let sequence_id =
        first_u64(&[message.get("sequence_id"), payload.get("sequence_id")]).unwrap_or_default();
    let sample_time_unix_ns = first_i64(&[
        payload.get("sample_time_unix_ns"),
        payload.get("timestamp_unix_ns"),
        message.get("sample_time_unix_ns"),
    ])
    .unwrap_or_default();
    Some(BreathFeedbackSample {
        stream_id,
        sequence_id,
        sample_time_unix_ns,
        volume01: volume01.clamp(0.0, 1.0),
        phase,
        quality,
        payload_hash: stable_json_hash(payload),
    })
}

pub(crate) fn build_breath_feedback_receipt_command(
    config: &ManifoldBreathFeedbackConfig,
    sample: &BreathFeedbackSample,
    receipt_sequence_id: u64,
) -> JsonValue {
    json!({
        "type": "command",
        "schema": "rusty.xr.broker.command.v1",
        "request_id": format!("makepad-breath-feedback-receipt-{}", receipt_sequence_id),
        "command": BREATH_FEEDBACK_RECEIPT_COMMAND,
        "params": {
            "schema": BREATH_FEEDBACK_RECEIPT_SCHEMA,
            "received_stream": sample.stream_id.clone(),
            "received_sequence_id": sample.sequence_id,
            "received_sample_time_unix_ns": sample.sample_time_unix_ns,
            "receiver": config.receiver_id.clone(),
            "acknowledged": true,
            "volume01": sample.volume01,
            "phase": sample.phase.clone(),
            "quality": sample.quality.clone(),
            "payload_hash": sample.payload_hash.clone(),
        }
    })
}

fn run_breath_feedback_worker(config: ManifoldBreathFeedbackConfig, stop: Arc<AtomicBool>) {
    let mut client = BrokerWebSocketClient::new(config.clone());
    let mut receipt_sequence_id = 1_u64;
    while !stop.load(Ordering::Relaxed) {
        if client.ensure_subscribed().is_err() {
            client.reset();
            thread::sleep(Duration::from_millis(250));
            continue;
        }
        match client.recv_json() {
            Ok(Some(message)) => {
                if let Some(sample) = parse_breath_feedback_event(&message, &config.stream_id) {
                    let receipt = build_breath_feedback_receipt_command(
                        &config,
                        &sample,
                        receipt_sequence_id,
                    );
                    receipt_sequence_id = receipt_sequence_id.saturating_add(1);
                    if client.send_json(&receipt, receipt_sequence_id).is_err() {
                        client.reset();
                    }
                }
            }
            Ok(None) => {}
            Err(_) => {
                client.reset();
                thread::sleep(Duration::from_millis(250));
            }
        }
    }
}

struct BrokerWebSocketClient {
    config: ManifoldBreathFeedbackConfig,
    stream: Option<TcpStream>,
    subscribed: bool,
}

impl BrokerWebSocketClient {
    fn new(config: ManifoldBreathFeedbackConfig) -> Self {
        Self {
            config,
            stream: None,
            subscribed: false,
        }
    }

    fn reset(&mut self) {
        self.stream = None;
        self.subscribed = false;
    }

    fn ensure_subscribed(&mut self) -> Result<(), String> {
        if self.stream.is_none() {
            self.stream = Some(open_broker_websocket(&self.config)?);
            self.send_json(&build_hello_message(&self.config), 1)?;
        }
        if !self.subscribed {
            self.send_json(&build_breath_feedback_subscribe_command(&self.config), 2)?;
            self.subscribed = true;
        }
        Ok(())
    }

    fn send_json(&mut self, value: &JsonValue, sequence_id: u64) -> Result<(), String> {
        let text = value.to_string();
        let frame = websocket_client_text_frame(text.as_bytes(), sequence_id);
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| "missing_websocket_stream".to_string())?;
        stream
            .write_all(&frame)
            .map_err(|err| format!("websocket_write_failed:{err}"))?;
        stream
            .flush()
            .map_err(|err| format!("websocket_flush_failed:{err}"))?;
        Ok(())
    }

    fn recv_json(&mut self) -> Result<Option<JsonValue>, String> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| "missing_websocket_stream".to_string())?;
        let payload = read_websocket_frame(stream)?;
        if payload.is_empty() {
            return Ok(None);
        }
        let text =
            String::from_utf8(payload).map_err(|err| format!("websocket_text_invalid:{err}"))?;
        let value =
            serde_json::from_str(&text).map_err(|err| format!("websocket_json_invalid:{err}"))?;
        Ok(Some(value))
    }
}

fn build_hello_message(config: &ManifoldBreathFeedbackConfig) -> JsonValue {
    json!({
        "type": "hello",
        "client_id": config.receiver_id,
        "app_package": "rusty-xr-makepad-camera-shell",
        "role": "makepad_breath_feedback_subscriber",
    })
}

fn open_broker_websocket(config: &ManifoldBreathFeedbackConfig) -> Result<TcpStream, String> {
    let address = format!("{}:{}", config.broker_host, config.broker_port);
    let mut stream =
        TcpStream::connect(address).map_err(|err| format!("broker_connect_failed:{err}"))?;
    let timeout = Duration::from_millis(config.connect_timeout_ms.clamp(50, 5_000));
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
    let request = format!(
        "GET /rustyxr/v1/events HTTP/1.1\r\n\
         Host: {}:{}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: cnVzdHkteHItdWFrZXBhZC1icmVhdGg=\r\n\
         Sec-WebSocket-Version: 13\r\n\
         \r\n",
        config.broker_host, config.broker_port
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|err| format!("websocket_handshake_write_failed:{err}"))?;

    let mut response = Vec::with_capacity(512);
    let mut byte = [0_u8; 1];
    while response.len() < 4096 {
        stream
            .read_exact(&mut byte)
            .map_err(|err| format!("websocket_handshake_read_failed:{err}"))?;
        response.push(byte[0]);
        if response.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let response_text = String::from_utf8_lossy(&response);
    if !response_text.starts_with("HTTP/1.1 101") {
        return Err("websocket_handshake_rejected".to_string());
    }
    Ok(stream)
}

fn websocket_client_text_frame(payload: &[u8], sequence_id: u64) -> Vec<u8> {
    let mut frame = Vec::with_capacity(payload.len() + 16);
    frame.push(0x81);
    let mask = websocket_mask(sequence_id);
    if payload.len() <= 125 {
        frame.push(0x80 | payload.len() as u8);
    } else if payload.len() <= u16::MAX as usize {
        frame.push(0x80 | 126);
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        frame.push(0x80 | 127);
        frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    }
    frame.extend_from_slice(&mask);
    for (index, byte) in payload.iter().enumerate() {
        frame.push(*byte ^ mask[index % 4]);
    }
    frame
}

fn websocket_mask(sequence_id: u64) -> [u8; 4] {
    let bytes = sequence_id
        .wrapping_mul(0xD1B5_4A32_D192_ED03)
        .to_le_bytes();
    [bytes[0], bytes[3], bytes[5], bytes[7]]
}

fn read_websocket_frame(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    let mut header = [0_u8; 2];
    stream
        .read_exact(&mut header)
        .map_err(|err| format!("websocket_frame_header_read_failed:{err}"))?;
    let masked = (header[1] & 0x80) != 0;
    let mut len = (header[1] & 0x7f) as usize;
    if len == 126 {
        let mut extended = [0_u8; 2];
        stream
            .read_exact(&mut extended)
            .map_err(|err| format!("websocket_frame_len16_read_failed:{err}"))?;
        len = u16::from_be_bytes(extended) as usize;
    } else if len == 127 {
        let mut extended = [0_u8; 8];
        stream
            .read_exact(&mut extended)
            .map_err(|err| format!("websocket_frame_len64_read_failed:{err}"))?;
        len = u64::from_be_bytes(extended).min(1024 * 1024) as usize;
    }
    let mut mask = [0_u8; 4];
    if masked {
        stream
            .read_exact(&mut mask)
            .map_err(|err| format!("websocket_frame_mask_read_failed:{err}"))?;
    }
    let mut payload = vec![0_u8; len.min(1024 * 1024)];
    stream
        .read_exact(&mut payload)
        .map_err(|err| format!("websocket_frame_payload_read_failed:{err}"))?;
    if masked {
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[index % 4];
        }
    }
    Ok(payload)
}

fn first_string(values: &[Option<&JsonValue>], default: &str) -> String {
    values
        .iter()
        .filter_map(|value| value.and_then(JsonValue::as_str))
        .find(|value| !value.trim().is_empty())
        .unwrap_or(default)
        .trim()
        .to_string()
}

fn first_f64(values: &[Option<&JsonValue>]) -> Option<f64> {
    values
        .iter()
        .filter_map(|value| value.and_then(JsonValue::as_f64))
        .find(|value| value.is_finite())
}

fn first_u64(values: &[Option<&JsonValue>]) -> Option<u64> {
    values
        .iter()
        .filter_map(|value| {
            value.and_then(JsonValue::as_u64).or_else(|| {
                value
                    .and_then(JsonValue::as_i64)
                    .and_then(|item| u64::try_from(item).ok())
            })
        })
        .next()
}

fn first_i64(values: &[Option<&JsonValue>]) -> Option<i64> {
    values
        .iter()
        .filter_map(|value| {
            value.and_then(JsonValue::as_i64).or_else(|| {
                value
                    .and_then(JsonValue::as_u64)
                    .and_then(|item| i64::try_from(item).ok())
            })
        })
        .next()
}

fn stable_json_hash(value: &JsonValue) -> String {
    let mut hasher = DefaultHasher::new();
    value.to_string().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_breath_feedback_subscription_command() {
        let config = ManifoldBreathFeedbackConfig {
            enabled: true,
            ..ManifoldBreathFeedbackConfig::default()
        };
        let command = build_breath_feedback_subscribe_command(&config);
        assert_eq!(command["command"], "subscribe");
        assert_eq!(command["params"]["stream"], BREATH_FEEDBACK_STREAM_ID);
        assert_eq!(command["params"]["receiver"], BREATH_FEEDBACK_RECEIVER_ID);
    }

    #[test]
    fn parses_feedback_stream_event_and_builds_receipt() {
        let config = ManifoldBreathFeedbackConfig::default();
        let event = json!({
            "type": "stream_event",
            "stream": "stream.breath.feedback_state",
            "sequence_id": 7,
            "payload": {
                "stream_id": "stream.breath.feedback_state",
                "sample_time_unix_ns": 1777900000000000000_i64,
                "volume01": 0.62,
                "phase": "inhale",
                "quality": "stable"
            }
        });
        let sample =
            parse_breath_feedback_event(&event, BREATH_FEEDBACK_STREAM_ID).expect("sample parses");
        assert_eq!(sample.sequence_id, 7);
        assert_eq!(sample.phase, "inhale");
        assert!((sample.volume01 - 0.62).abs() < 0.0001);

        let receipt = build_breath_feedback_receipt_command(&config, &sample, 11);
        assert_eq!(receipt["command"], BREATH_FEEDBACK_RECEIPT_COMMAND);
        assert_eq!(receipt["params"]["schema"], BREATH_FEEDBACK_RECEIPT_SCHEMA);
        assert_eq!(
            receipt["params"]["received_stream"],
            BREATH_FEEDBACK_STREAM_ID
        );
        assert_eq!(receipt["params"]["received_sequence_id"], 7);
        assert_eq!(receipt["params"]["acknowledged"], true);
    }

    #[test]
    fn ignores_unmatched_feedback_stream() {
        let event = json!({
            "type": "stream_event",
            "stream": "stream.breath.volume",
            "payload": {"volume01": 0.5}
        });
        assert!(parse_breath_feedback_event(&event, BREATH_FEEDBACK_STREAM_ID).is_none());
    }
}
