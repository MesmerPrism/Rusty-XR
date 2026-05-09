//! Broker protocol and stream-manifest contracts for Rusty XR.
//!
//! This crate contains pure data models for broker control envelopes, stream
//! manifests, sample headers, timing stamps, drop counters, and negotiated
//! transport lanes. It does not open sockets, depend on Android, or implement a
//! Unity, Makepad, OpenXR, LSL, OSC, or video backend.
//!
//! Enable the `serde` feature when these public contracts need to cross
//! process boundaries.
//!
//! ```
//! use rusty_xr_broker_model::{
//!     BrokerPayloadKind, BrokerReliabilityClass, BrokerStreamManifest,
//! };
//!
//! let manifest = BrokerStreamManifest::new(
//!     "synthetic:wave",
//!     "broker",
//!     BrokerPayloadKind::Json,
//!     "rusty.xr.synthetic.wave.v1",
//! )
//! .with_reliability(BrokerReliabilityClass::LossTolerant)
//! .with_recommended_rate_hz(90.0);
//!
//! assert!(manifest.is_valid());
//! ```

/// Crate version exposed for lightweight smoke checks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Versioned JSON schema id for broker client hello messages.
pub const BROKER_CLIENT_HELLO_SCHEMA: &str = "rusty.xr.broker.client_hello.v1";

/// Versioned JSON schema id for broker command messages.
pub const BROKER_COMMAND_SCHEMA: &str = "rusty.xr.broker.command.v1";

/// Versioned JSON schema id for broker command acknowledgement messages.
pub const BROKER_COMMAND_ACK_SCHEMA: &str = "rusty.xr.broker.command_ack.v1";

/// Versioned JSON schema id for broker stream event messages.
pub const BROKER_STREAM_EVENT_SCHEMA: &str = "rusty.xr.broker.stream_event.v1";

/// Versioned JSON schema id for broker replay records.
pub const BROKER_REPLAY_RECORD_SCHEMA: &str = "rusty.xr.broker.replay_record.v1";

/// Versioned JSON schema id for broker stream manifests.
pub const BROKER_STREAM_MANIFEST_SCHEMA: &str = "rusty.xr.broker.stream_manifest.v1";

/// Versioned JSON schema id for broker session manifests.
pub const BROKER_SESSION_MANIFEST_SCHEMA: &str = "rusty.xr.broker.session_manifest.v1";

/// Versioned JSON schema id for broker stream sample headers.
pub const BROKER_STREAM_SAMPLE_HEADER_SCHEMA: &str = "rusty.xr.broker.stream_sample_header.v1";

/// Versioned JSON schema id for broker status snapshots.
pub const BROKER_STATUS_SCHEMA: &str = "rusty.xr.broker.status.v1";

/// Existing broker latency sample schema id used by public probes.
pub const BROKER_LATENCY_SAMPLE_SCHEMA: &str = "rusty.xr.broker.latency_sample.v1";

/// Versioned JSON schema id for deterministic synthetic wave samples.
pub const SYNTHETIC_WAVE_PAYLOAD_SCHEMA: &str = "rusty.xr.synthetic.wave.v1";

/// Public synthetic stream id used by broker smoke tests.
pub const STREAM_SYNTHETIC_WAVE: &str = "synthetic:wave";

/// Public OSC drive stream id used by broker comparison tests.
pub const STREAM_OSC_DRIVE_RADIUS: &str = "osc:/rusty-xr/drive/radius";

/// Public latency sample stream id.
pub const STREAM_LATENCY_SAMPLE: &str = "latency:sample";

/// Public diagnostic heart stream id.
pub const STREAM_BIO_HEART: &str = "bio:heart";

/// Public diagnostic breath stream id.
pub const STREAM_BIO_BREATH: &str = "bio:breath";

/// Maximum UDP payload accepted by a standard IPv4 datagram.
pub const MAX_UDP_DATAGRAM_BYTES: u32 = 65_507;

/// Broker transport families that can carry a stream lane or control path.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerTransportKind {
    WebSocket,
    Tcp,
    Udp,
    AdbForwardedTcp,
    MetadataOnly,
}

/// Delivery contract advertised by a broker stream.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerReliabilityClass {
    Reliable,
    LossTolerant,
    BestEffort,
    MetadataOnly,
}

/// Payload family for broker stream samples.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerPayloadKind {
    Json,
    Text,
    Binary,
    H264,
    H265,
    RawLuma8,
    Custom,
}

/// Endpoint details returned after a stream lane is negotiated.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerTransportEndpoint {
    pub transport: BrokerTransportKind,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub path: Option<String>,
    pub channel_id: Option<String>,
    pub max_datagram_bytes: Option<u32>,
    pub auth_required: bool,
}

impl BrokerTransportEndpoint {
    pub fn websocket(path: impl Into<String>) -> Self {
        Self {
            transport: BrokerTransportKind::WebSocket,
            host: None,
            port: None,
            path: Some(path.into()),
            channel_id: None,
            max_datagram_bytes: None,
            auth_required: false,
        }
    }

    pub fn udp(host: impl Into<String>, port: u16, max_datagram_bytes: u32) -> Self {
        Self {
            transport: BrokerTransportKind::Udp,
            host: Some(host.into()),
            port: Some(port),
            path: None,
            channel_id: None,
            max_datagram_bytes: Some(max_datagram_bytes.min(MAX_UDP_DATAGRAM_BYTES)),
            auth_required: false,
        }
    }

    pub fn metadata_only(channel_id: impl Into<String>) -> Self {
        Self {
            transport: BrokerTransportKind::MetadataOnly,
            host: None,
            port: None,
            path: None,
            channel_id: Some(channel_id.into()),
            max_datagram_bytes: None,
            auth_required: false,
        }
    }

    pub fn with_auth_required(mut self, auth_required: bool) -> Self {
        self.auth_required = auth_required;
        self
    }

    pub fn is_valid(&self) -> bool {
        match self.transport {
            BrokerTransportKind::WebSocket => self
                .path
                .as_deref()
                .map(|path| path.starts_with('/') && !path.trim().is_empty())
                .unwrap_or(false),
            BrokerTransportKind::Tcp
            | BrokerTransportKind::Udp
            | BrokerTransportKind::AdbForwardedTcp => {
                self.host
                    .as_deref()
                    .map(|host| !host.trim().is_empty())
                    .unwrap_or(false)
                    && self.port.map(|port| port > 0).unwrap_or(false)
                    && self
                        .max_datagram_bytes
                        .map(valid_datagram_size)
                        .unwrap_or(true)
            }
            BrokerTransportKind::MetadataOnly => self
                .channel_id
                .as_deref()
                .map(|channel_id| !channel_id.trim().is_empty())
                .unwrap_or(false),
        }
    }
}

/// Stream manifest returned by the broker before samples start flowing.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerStreamManifest {
    pub manifest_schema: String,
    pub stream_id: String,
    pub session_id: Option<String>,
    pub source_id: String,
    pub payload_kind: BrokerPayloadKind,
    pub payload_schema: String,
    pub sequence_start: u64,
    pub recommended_rate_hz: Option<f32>,
    pub max_datagram_bytes: Option<u32>,
    pub reliability: BrokerReliabilityClass,
    pub ordered: bool,
    pub endpoint: Option<BrokerTransportEndpoint>,
    pub heartbeat: Option<BrokerHeartbeatState>,
    pub drop_counters: BrokerDropCounters,
}

impl BrokerStreamManifest {
    pub fn new(
        stream_id: impl Into<String>,
        source_id: impl Into<String>,
        payload_kind: BrokerPayloadKind,
        payload_schema: impl Into<String>,
    ) -> Self {
        Self {
            manifest_schema: BROKER_STREAM_MANIFEST_SCHEMA.to_string(),
            stream_id: stream_id.into(),
            session_id: None,
            source_id: source_id.into(),
            payload_kind,
            payload_schema: payload_schema.into(),
            sequence_start: 0,
            recommended_rate_hz: None,
            max_datagram_bytes: None,
            reliability: BrokerReliabilityClass::Reliable,
            ordered: true,
            endpoint: None,
            heartbeat: None,
            drop_counters: BrokerDropCounters::default(),
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub const fn with_sequence_start(mut self, sequence_start: u64) -> Self {
        self.sequence_start = sequence_start;
        self
    }

    pub const fn with_reliability(mut self, reliability: BrokerReliabilityClass) -> Self {
        self.reliability = reliability;
        self
    }

    pub const fn with_ordered(mut self, ordered: bool) -> Self {
        self.ordered = ordered;
        self
    }

    pub fn with_recommended_rate_hz(mut self, recommended_rate_hz: f32) -> Self {
        self.recommended_rate_hz = Some(recommended_rate_hz);
        self
    }

    pub fn with_max_datagram_bytes(mut self, max_datagram_bytes: u32) -> Self {
        self.max_datagram_bytes = Some(max_datagram_bytes.min(MAX_UDP_DATAGRAM_BYTES));
        self
    }

    pub fn with_endpoint(mut self, endpoint: BrokerTransportEndpoint) -> Self {
        self.endpoint = Some(endpoint);
        self
    }

    pub const fn with_heartbeat(mut self, heartbeat: BrokerHeartbeatState) -> Self {
        self.heartbeat = Some(heartbeat);
        self
    }

    pub fn is_loss_tolerant(&self) -> bool {
        matches!(
            self.reliability,
            BrokerReliabilityClass::LossTolerant | BrokerReliabilityClass::BestEffort
        )
    }

    pub fn supports_udp_datagrams(&self) -> bool {
        self.endpoint
            .as_ref()
            .map(|endpoint| endpoint.transport == BrokerTransportKind::Udp)
            .unwrap_or(false)
            && self
                .max_datagram_bytes
                .map(valid_datagram_size)
                .unwrap_or(false)
    }

    pub fn is_valid(&self) -> bool {
        !self.stream_id.trim().is_empty()
            && !self.source_id.trim().is_empty()
            && !self.payload_schema.trim().is_empty()
            && self
                .recommended_rate_hz
                .map(|rate| rate.is_finite() && rate > 0.0)
                .unwrap_or(true)
            && self
                .max_datagram_bytes
                .map(valid_datagram_size)
                .unwrap_or(true)
            && self
                .endpoint
                .as_ref()
                .map(BrokerTransportEndpoint::is_valid)
                .unwrap_or(true)
    }
}

/// Per-sample metadata shared by JSON events and binary side-channel packets.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerStreamSampleHeader {
    pub schema: String,
    pub stream_id: String,
    pub session_id: Option<String>,
    pub source_id: String,
    pub payload_kind: BrokerPayloadKind,
    pub payload_schema: String,
    pub sequence_number: u64,
    pub broker_time_elapsed_ns: u64,
    pub broker_time_unix_ns: Option<u64>,
    pub source_time_ns: Option<u64>,
    pub source_time_unix_ns: Option<u64>,
    pub dropped_before_sample: u64,
    pub late_before_sample: u64,
}

impl BrokerStreamSampleHeader {
    pub fn new(
        manifest: &BrokerStreamManifest,
        sequence_number: u64,
        broker_time_elapsed_ns: u64,
    ) -> Self {
        Self {
            schema: BROKER_STREAM_SAMPLE_HEADER_SCHEMA.to_string(),
            stream_id: manifest.stream_id.clone(),
            session_id: manifest.session_id.clone(),
            source_id: manifest.source_id.clone(),
            payload_kind: manifest.payload_kind,
            payload_schema: manifest.payload_schema.clone(),
            sequence_number,
            broker_time_elapsed_ns,
            broker_time_unix_ns: None,
            source_time_ns: None,
            source_time_unix_ns: None,
            dropped_before_sample: manifest.drop_counters.dropped_samples,
            late_before_sample: manifest.drop_counters.late_samples,
        }
    }

    pub fn with_broker_time_unix_ns(mut self, broker_time_unix_ns: u64) -> Self {
        self.broker_time_unix_ns = Some(broker_time_unix_ns);
        self
    }

    pub fn with_source_time_ns(mut self, source_time_ns: u64) -> Self {
        self.source_time_ns = Some(source_time_ns);
        self
    }

    pub fn with_source_time_unix_ns(mut self, source_time_unix_ns: u64) -> Self {
        self.source_time_unix_ns = Some(source_time_unix_ns);
        self
    }

    pub fn age_ns(&self, now_elapsed_ns: u64) -> Option<u64> {
        now_elapsed_ns.checked_sub(self.broker_time_elapsed_ns)
    }

    pub fn is_valid(&self) -> bool {
        !self.stream_id.trim().is_empty()
            && !self.source_id.trim().is_empty()
            && !self.payload_schema.trim().is_empty()
    }
}

/// Versioned stream event envelope carrying a typed payload.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerStreamEvent<TPayload = ()> {
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub message_type: String,
    pub schema: String,
    pub stream: String,
    pub subscription_id: Option<String>,
    pub header: BrokerStreamSampleHeader,
    pub payload: TPayload,
}

impl<TPayload> BrokerStreamEvent<TPayload> {
    pub fn new(header: BrokerStreamSampleHeader, payload: TPayload) -> Self {
        Self {
            message_type: "stream_event".to_string(),
            schema: BROKER_STREAM_EVENT_SCHEMA.to_string(),
            stream: header.stream_id.clone(),
            subscription_id: None,
            header,
            payload,
        }
    }

    pub fn with_subscription_id(mut self, subscription_id: impl Into<String>) -> Self {
        self.subscription_id = Some(subscription_id.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        !self.stream.trim().is_empty()
            && self.stream == self.header.stream_id
            && self.header.is_valid()
    }
}

/// Versioned JSONL-friendly record for broker session replay.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerReplayRecord<TPayload = ()> {
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub message_type: String,
    pub schema: String,
    pub session_id: String,
    pub stream: String,
    pub header: BrokerStreamSampleHeader,
    pub payload: TPayload,
}

impl<TPayload> BrokerReplayRecord<TPayload> {
    pub fn new(
        session_id: impl Into<String>,
        header: BrokerStreamSampleHeader,
        payload: TPayload,
    ) -> Self {
        Self {
            message_type: "replay_record".to_string(),
            schema: BROKER_REPLAY_RECORD_SCHEMA.to_string(),
            session_id: session_id.into(),
            stream: header.stream_id.clone(),
            header,
            payload,
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.session_id.trim().is_empty()
            && self.stream == self.header.stream_id
            && self
                .header
                .session_id
                .as_deref()
                .map(|header_session_id| header_session_id == self.session_id)
                .unwrap_or(true)
            && self.header.is_valid()
    }
}

/// Deterministic synthetic payload used to validate broker stream plumbing.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SyntheticWaveSample {
    pub sequence_number: u64,
    pub sample_time_elapsed_ns: u64,
    pub value01: f32,
    pub phase01: f32,
    pub valid: bool,
}

impl SyntheticWaveSample {
    pub fn new(sequence_number: u64, sample_time_elapsed_ns: u64, phase01: f32) -> Self {
        let phase01 = if phase01.is_finite() {
            phase01.rem_euclid(1.0)
        } else {
            0.0
        };
        let value01 = (0.5 + (phase01 * core::f32::consts::TAU).sin() * 0.5).clamp(0.0, 1.0);

        Self {
            sequence_number,
            sample_time_elapsed_ns,
            value01,
            phase01,
            valid: true,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.valid
            && self.value01.is_finite()
            && (0.0..=1.0).contains(&self.value01)
            && self.phase01.is_finite()
            && (0.0..=1.0).contains(&self.phase01)
    }
}

/// Validation error for deterministic synthetic broker stream generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerSyntheticError {
    InvalidRateHz,
}

/// Source-only synthetic stream generator for broker validation and replay tests.
#[derive(Clone, Debug, PartialEq)]
pub struct SyntheticWaveGenerator {
    pub manifest: BrokerStreamManifest,
    pub sample_period_ns: u64,
    pub next_sequence_number: u64,
    pub next_time_elapsed_ns: u64,
}

impl SyntheticWaveGenerator {
    pub fn new(source_id: impl Into<String>, rate_hz: f32) -> Result<Self, BrokerSyntheticError> {
        if !rate_hz.is_finite() || rate_hz <= 0.0 {
            return Err(BrokerSyntheticError::InvalidRateHz);
        }

        let sample_period_ns = (1_000_000_000.0 / rate_hz).round().max(1.0) as u64;
        let manifest = BrokerStreamManifest::new(
            STREAM_SYNTHETIC_WAVE,
            source_id,
            BrokerPayloadKind::Json,
            SYNTHETIC_WAVE_PAYLOAD_SCHEMA,
        )
        .with_reliability(BrokerReliabilityClass::LossTolerant)
        .with_ordered(false)
        .with_recommended_rate_hz(rate_hz);

        Ok(Self {
            manifest,
            sample_period_ns,
            next_sequence_number: 0,
            next_time_elapsed_ns: 0,
        })
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.manifest = self.manifest.with_session_id(session_id);
        self
    }

    pub fn next_sample(&mut self) -> BrokerStreamEvent<SyntheticWaveSample> {
        let sequence_number = self.next_sequence_number;
        let sample_time_elapsed_ns = self.next_time_elapsed_ns;
        let phase01 = (sequence_number % 60) as f32 / 60.0;
        let payload = SyntheticWaveSample::new(sequence_number, sample_time_elapsed_ns, phase01);
        let header =
            BrokerStreamSampleHeader::new(&self.manifest, sequence_number, sample_time_elapsed_ns);

        self.next_sequence_number = self.next_sequence_number.saturating_add(1);
        self.next_time_elapsed_ns = self
            .next_time_elapsed_ns
            .saturating_add(self.sample_period_ns);

        BrokerStreamEvent::new(header, payload)
    }
}

/// Public key/value metadata item for broker session manifests.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerSessionMetadata {
    pub key: String,
    pub value: String,
}

impl BrokerSessionMetadata {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.key.trim().is_empty()
    }
}

/// Manifest for one broker-owned recording or replay session.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerSessionManifest {
    pub schema: String,
    pub session_id: String,
    pub started_time_unix_ns: Option<u64>,
    pub ended_time_unix_ns: Option<u64>,
    pub streams: Vec<BrokerStreamManifest>,
    pub metadata: Vec<BrokerSessionMetadata>,
}

impl BrokerSessionManifest {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            schema: BROKER_SESSION_MANIFEST_SCHEMA.to_string(),
            session_id: session_id.into(),
            started_time_unix_ns: None,
            ended_time_unix_ns: None,
            streams: Vec::new(),
            metadata: Vec::new(),
        }
    }

    pub fn with_started_time_unix_ns(mut self, started_time_unix_ns: u64) -> Self {
        self.started_time_unix_ns = Some(started_time_unix_ns);
        self
    }

    pub fn with_stream(mut self, stream: BrokerStreamManifest) -> Self {
        self.streams.push(stream);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push(BrokerSessionMetadata::new(key, value));
        self
    }

    pub fn is_valid(&self) -> bool {
        !self.session_id.trim().is_empty()
            && self.streams.iter().all(BrokerStreamManifest::is_valid)
            && self.metadata.iter().all(BrokerSessionMetadata::is_valid)
            && self
                .ended_time_unix_ns
                .zip(self.started_time_unix_ns)
                .map(|(ended, started)| ended >= started)
                .unwrap_or(true)
    }
}

/// Monotonic and optional wall-clock timestamp captured at a broker boundary.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrokerTimingStamp {
    pub elapsed_ns: u64,
    pub unix_ns: Option<u64>,
}

impl BrokerTimingStamp {
    pub const fn elapsed(elapsed_ns: u64) -> Self {
        Self {
            elapsed_ns,
            unix_ns: None,
        }
    }

    pub const fn with_unix_ns(mut self, unix_ns: u64) -> Self {
        self.unix_ns = Some(unix_ns);
        self
    }

    pub fn elapsed_since(self, earlier: Self) -> Option<u64> {
        self.elapsed_ns.checked_sub(earlier.elapsed_ns)
    }
}

/// Broker-visible packet and queue counters.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BrokerDropCounters {
    pub received_samples: u64,
    pub emitted_samples: u64,
    pub dropped_samples: u64,
    pub late_samples: u64,
    pub duplicate_samples: u64,
    pub out_of_order_samples: u64,
    pub queue_overflow_count: u64,
}

impl BrokerDropCounters {
    pub fn record_received(&mut self) {
        self.received_samples = self.received_samples.saturating_add(1);
    }

    pub fn record_emitted(&mut self) {
        self.emitted_samples = self.emitted_samples.saturating_add(1);
    }

    pub fn record_dropped(&mut self) {
        self.dropped_samples = self.dropped_samples.saturating_add(1);
    }

    pub fn record_late(&mut self) {
        self.late_samples = self.late_samples.saturating_add(1);
    }

    pub fn loss_count(self) -> u64 {
        self.dropped_samples
            .saturating_add(self.late_samples)
            .saturating_add(self.queue_overflow_count)
    }
}

/// Heartbeat state for stream timeout decisions.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrokerHeartbeatState {
    pub last_heartbeat_elapsed_ns: Option<u64>,
    pub timeout_after_ns: u64,
}

impl BrokerHeartbeatState {
    pub const fn new(timeout_after_ns: u64) -> Self {
        Self {
            last_heartbeat_elapsed_ns: None,
            timeout_after_ns,
        }
    }

    pub const fn with_last_heartbeat_elapsed_ns(mut self, last_heartbeat_elapsed_ns: u64) -> Self {
        self.last_heartbeat_elapsed_ns = Some(last_heartbeat_elapsed_ns);
        self
    }

    pub fn timed_out(self, now_elapsed_ns: u64) -> bool {
        self.last_heartbeat_elapsed_ns
            .map(|last| now_elapsed_ns.saturating_sub(last) > self.timeout_after_ns)
            .unwrap_or(true)
    }
}

/// First message a broker client can send after connection.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerClientHello {
    pub schema: String,
    pub client_id: String,
    pub app_label: Option<String>,
    pub app_version: Option<String>,
    pub protocol_min: u32,
    pub protocol_max: u32,
    pub session_token: Option<String>,
}

impl BrokerClientHello {
    pub fn new(client_id: impl Into<String>) -> Self {
        Self {
            schema: BROKER_CLIENT_HELLO_SCHEMA.to_string(),
            client_id: client_id.into(),
            app_label: None,
            app_version: None,
            protocol_min: 1,
            protocol_max: 1,
            session_token: None,
        }
    }

    pub fn with_app_label(mut self, app_label: impl Into<String>) -> Self {
        self.app_label = Some(app_label.into());
        self
    }

    pub fn with_app_version(mut self, app_version: impl Into<String>) -> Self {
        self.app_version = Some(app_version.into());
        self
    }

    pub const fn with_protocol_range(mut self, protocol_min: u32, protocol_max: u32) -> Self {
        self.protocol_min = protocol_min;
        self.protocol_max = protocol_max;
        self
    }

    pub fn is_valid(&self) -> bool {
        !self.client_id.trim().is_empty() && self.protocol_min <= self.protocol_max
    }
}

/// Versioned broker command envelope.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerCommand<TParams = ()> {
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub message_type: String,
    pub schema: String,
    pub request_id: String,
    pub client_id: String,
    pub command: String,
    pub params: Option<TParams>,
}

impl<TParams> BrokerCommand<TParams> {
    pub fn new(
        request_id: impl Into<String>,
        client_id: impl Into<String>,
        command: impl Into<String>,
        params: Option<TParams>,
    ) -> Self {
        Self {
            message_type: "command".to_string(),
            schema: BROKER_COMMAND_SCHEMA.to_string(),
            request_id: request_id.into(),
            client_id: client_id.into(),
            command: command.into(),
            params,
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.request_id.trim().is_empty()
            && !self.client_id.trim().is_empty()
            && !self.command.trim().is_empty()
    }
}

/// Versioned broker command acknowledgement envelope.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerCommandAck<TResult = ()> {
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub message_type: String,
    pub schema: String,
    pub request_id: String,
    pub accepted: bool,
    pub result: Option<TResult>,
    pub error: Option<String>,
}

impl<TResult> BrokerCommandAck<TResult> {
    pub fn accepted(request_id: impl Into<String>, result: Option<TResult>) -> Self {
        Self {
            message_type: "command_ack".to_string(),
            schema: BROKER_COMMAND_ACK_SCHEMA.to_string(),
            request_id: request_id.into(),
            accepted: true,
            result,
            error: None,
        }
    }

    pub fn rejected(request_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            message_type: "command_ack".to_string(),
            schema: BROKER_COMMAND_ACK_SCHEMA.to_string(),
            request_id: request_id.into(),
            accepted: false,
            result: None,
            error: Some(error.into()),
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.request_id.trim().is_empty()
            && (self.accepted
                || self
                    .error
                    .as_deref()
                    .map(|error| !error.trim().is_empty())
                    .unwrap_or(false))
    }
}

/// Stream subscription negotiated by a client and broker.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerSubscription {
    pub subscription_id: String,
    pub client_id: String,
    pub stream_id: String,
    pub requested_reliability: Option<BrokerReliabilityClass>,
    pub accepted_endpoint: Option<BrokerTransportEndpoint>,
}

impl BrokerSubscription {
    pub fn new(
        subscription_id: impl Into<String>,
        client_id: impl Into<String>,
        stream_id: impl Into<String>,
    ) -> Self {
        Self {
            subscription_id: subscription_id.into(),
            client_id: client_id.into(),
            stream_id: stream_id.into(),
            requested_reliability: None,
            accepted_endpoint: None,
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.subscription_id.trim().is_empty()
            && !self.client_id.trim().is_empty()
            && !self.stream_id.trim().is_empty()
            && self
                .accepted_endpoint
                .as_ref()
                .map(BrokerTransportEndpoint::is_valid)
                .unwrap_or(true)
    }
}

/// Public capability advertised by the broker status endpoint.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerCapability {
    pub id: String,
    pub schema: Option<String>,
    pub state_changing: bool,
    pub transports: Vec<BrokerTransportKind>,
}

impl BrokerCapability {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            schema: None,
            state_changing: false,
            transports: Vec::new(),
        }
    }

    pub fn with_schema(mut self, schema: impl Into<String>) -> Self {
        self.schema = Some(schema.into());
        self
    }

    pub const fn with_state_changing(mut self, state_changing: bool) -> Self {
        self.state_changing = state_changing;
        self
    }

    pub fn with_transport(mut self, transport: BrokerTransportKind) -> Self {
        self.transports.push(transport);
        self
    }

    pub fn is_valid(&self) -> bool {
        !self.id.trim().is_empty()
    }
}

/// Minimal client summary for status snapshots.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerClientSummary {
    pub client_id: String,
    pub connection_id: Option<String>,
    pub active_subscription_count: u32,
    pub last_seen_elapsed_ns: Option<u64>,
}

impl BrokerClientSummary {
    pub fn new(client_id: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            connection_id: None,
            active_subscription_count: 0,
            last_seen_elapsed_ns: None,
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.client_id.trim().is_empty()
    }
}

/// Public broker status snapshot.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerStatus {
    pub schema: String,
    pub broker_id: String,
    pub uptime_elapsed_ns: u64,
    pub capabilities: Vec<BrokerCapability>,
    pub streams: Vec<BrokerStreamManifest>,
    pub clients: Vec<BrokerClientSummary>,
    pub drop_counters: BrokerDropCounters,
}

impl BrokerStatus {
    pub fn new(broker_id: impl Into<String>, uptime_elapsed_ns: u64) -> Self {
        Self {
            schema: BROKER_STATUS_SCHEMA.to_string(),
            broker_id: broker_id.into(),
            uptime_elapsed_ns,
            capabilities: Vec::new(),
            streams: Vec::new(),
            clients: Vec::new(),
            drop_counters: BrokerDropCounters::default(),
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.broker_id.trim().is_empty()
            && self.capabilities.iter().all(BrokerCapability::is_valid)
            && self.streams.iter().all(BrokerStreamManifest::is_valid)
            && self.clients.iter().all(BrokerClientSummary::is_valid)
    }
}

fn valid_datagram_size(value: u32) -> bool {
    value > 0 && value <= MAX_UDP_DATAGRAM_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;

    fn udp_manifest() -> BrokerStreamManifest {
        BrokerStreamManifest::new(
            STREAM_SYNTHETIC_WAVE,
            "synthetic-provider",
            BrokerPayloadKind::Json,
            SYNTHETIC_WAVE_PAYLOAD_SCHEMA,
        )
        .with_session_id("session-001")
        .with_reliability(BrokerReliabilityClass::LossTolerant)
        .with_ordered(false)
        .with_recommended_rate_hz(120.0)
        .with_max_datagram_bytes(1200)
        .with_endpoint(BrokerTransportEndpoint::udp("127.0.0.1", 47777, 1200))
    }

    #[test]
    fn exposes_workspace_version() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn validates_loss_tolerant_udp_manifest() {
        let manifest = udp_manifest();

        assert!(manifest.is_valid());
        assert!(manifest.is_loss_tolerant());
        assert!(manifest.supports_udp_datagrams());
        assert_eq!(manifest.manifest_schema, BROKER_STREAM_MANIFEST_SCHEMA);
    }

    #[test]
    fn rejects_invalid_stream_manifest_rate() {
        let manifest = BrokerStreamManifest::new(
            "",
            "source",
            BrokerPayloadKind::Json,
            SYNTHETIC_WAVE_PAYLOAD_SCHEMA,
        )
        .with_recommended_rate_hz(0.0);

        assert!(!manifest.is_valid());
    }

    #[test]
    fn sample_header_inherits_manifest_metadata() {
        let manifest = udp_manifest();
        let header = BrokerStreamSampleHeader::new(&manifest, 42, 1_000)
            .with_broker_time_unix_ns(2_000)
            .with_source_time_ns(900);

        assert!(header.is_valid());
        assert_eq!(header.stream_id, STREAM_SYNTHETIC_WAVE);
        assert_eq!(header.sequence_number, 42);
        assert_eq!(header.age_ns(1_250), Some(250));
        assert_eq!(header.source_time_ns, Some(900));
    }

    #[test]
    fn stream_event_requires_matching_header_stream() {
        let manifest = udp_manifest();
        let header = BrokerStreamSampleHeader::new(&manifest, 42, 1_000);
        let event = BrokerStreamEvent::new(header, "payload").with_subscription_id("sub-1");

        assert!(event.is_valid());
        assert_eq!(event.stream, STREAM_SYNTHETIC_WAVE);
        assert_eq!(event.subscription_id.as_deref(), Some("sub-1"));
    }

    #[test]
    fn synthetic_wave_generator_emits_valid_deterministic_events() {
        let mut generator = SyntheticWaveGenerator::new("synthetic-provider", 120.0)
            .expect("valid synthetic generator")
            .with_session_id("session-001");

        assert!(generator.manifest.is_valid());
        assert_eq!(generator.sample_period_ns, 8_333_334);

        let first = generator.next_sample();
        let second = generator.next_sample();

        assert!(first.is_valid());
        assert!(first.payload.is_valid());
        assert_eq!(first.header.sequence_number, 0);
        assert_eq!(first.payload.sample_time_elapsed_ns, 0);
        assert_eq!(second.header.sequence_number, 1);
        assert_eq!(second.payload.sample_time_elapsed_ns, 8_333_334);
        assert_eq!(second.header.payload_schema, SYNTHETIC_WAVE_PAYLOAD_SCHEMA);
    }

    #[test]
    fn replay_record_validates_session_and_header_stream() {
        let manifest = udp_manifest();
        let header = BrokerStreamSampleHeader::new(&manifest, 7, 500);
        let payload = SyntheticWaveSample::new(7, 500, 0.25);
        let record = BrokerReplayRecord::new("session-001", header, payload);

        assert!(record.is_valid());
        assert_eq!(record.schema, BROKER_REPLAY_RECORD_SCHEMA);
        assert_eq!(record.stream, STREAM_SYNTHETIC_WAVE);
    }

    #[test]
    fn session_manifest_validates_streams_and_metadata() {
        let session = BrokerSessionManifest::new("session-001")
            .with_started_time_unix_ns(2_000)
            .with_stream(udp_manifest())
            .with_metadata("purpose", "synthetic validation");

        assert!(session.is_valid());
        assert_eq!(session.schema, BROKER_SESSION_MANIFEST_SCHEMA);
    }

    #[test]
    fn heartbeat_reports_timeout() {
        let heartbeat = BrokerHeartbeatState::new(1_000).with_last_heartbeat_elapsed_ns(2_000);

        assert!(!heartbeat.timed_out(2_500));
        assert!(heartbeat.timed_out(3_001));
        assert!(BrokerHeartbeatState::new(1_000).timed_out(1));
    }

    #[test]
    fn drop_counters_saturate_and_count_losses() {
        let mut counters = BrokerDropCounters::default();
        counters.record_received();
        counters.record_emitted();
        counters.record_dropped();
        counters.record_late();

        assert_eq!(counters.received_samples, 1);
        assert_eq!(counters.emitted_samples, 1);
        assert_eq!(counters.loss_count(), 2);
    }

    #[test]
    fn command_and_ack_validate_required_ids() {
        let command = BrokerCommand::new("req-1", "client-1", "subscribe", Some("synthetic:wave"));
        let ack = BrokerCommandAck::accepted("req-1", Some("sub-1"));

        assert!(command.is_valid());
        assert!(ack.is_valid());
        assert!(BrokerCommandAck::<()>::rejected("req-2", "unknown stream").is_valid());
        assert!(!BrokerCommand::<()>::new("", "client-1", "subscribe", None).is_valid());
    }

    #[test]
    fn status_validates_nested_items() {
        let status = BrokerStatus {
            capabilities: vec![BrokerCapability::new("subscribe")
                .with_schema(BROKER_COMMAND_SCHEMA)
                .with_transport(BrokerTransportKind::WebSocket)],
            streams: vec![udp_manifest()],
            clients: vec![BrokerClientSummary::new("client-1")],
            ..BrokerStatus::new("broker", 100)
        };

        assert!(status.is_valid());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn stream_manifest_round_trips_with_serde() {
        let manifest = udp_manifest();

        let encoded = serde_json::to_string(&manifest).expect("manifest should serialize");
        let decoded: BrokerStreamManifest =
            serde_json::from_str(&encoded).expect("manifest should deserialize");

        assert_eq!(decoded, manifest);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn command_uses_public_type_field_with_serde() {
        let command = BrokerCommand::new("req-1", "client-1", "subscribe", Some("synthetic:wave"));

        let encoded = serde_json::to_value(&command).expect("command should serialize");

        assert_eq!(encoded["type"], "command");
        assert!(encoded.get("message_type").is_none());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn replay_record_uses_public_type_field_with_serde() {
        let manifest = udp_manifest();
        let header = BrokerStreamSampleHeader::new(&manifest, 1, 100);
        let record =
            BrokerReplayRecord::new("session-001", header, SyntheticWaveSample::new(1, 100, 0.1));

        let encoded = serde_json::to_value(&record).expect("replay record should serialize");

        assert_eq!(encoded["type"], "replay_record");
        assert!(encoded.get("message_type").is_none());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn synthetic_broker_replay_fixture_deserializes() {
        let session: BrokerSessionManifest = serde_json::from_str(include_str!(
            "../../../fixtures/replay/synthetic-broker-wave.session.json"
        ))
        .expect("fixture session should deserialize");
        let records = parse_replay_fixture_records(include_str!(
            "../../../fixtures/replay/synthetic-broker-wave.jsonl"
        ));

        assert!(session.is_valid());
        assert_eq!(session.session_id, "synthetic-broker-wave-session");
        assert_eq!(session.streams.len(), 1);
        assert_eq!(session.streams[0].stream_id, STREAM_SYNTHETIC_WAVE);
        assert_eq!(
            session.streams[0].payload_schema,
            SYNTHETIC_WAVE_PAYLOAD_SCHEMA
        );
        assert_eq!(records.len(), 4);

        for (index, record) in records.iter().enumerate() {
            assert!(record.is_valid());
            assert_eq!(record.session_id, session.session_id);
            assert_eq!(record.stream, STREAM_SYNTHETIC_WAVE);
            assert_eq!(record.header.sequence_number, index as u64);
            assert_eq!(
                record.header.source_time_ns,
                Some(record.header.broker_time_elapsed_ns)
            );
            assert_eq!(record.payload["sequence_number"], index as u64);
            assert_eq!(
                record.payload["sample_time_elapsed_ns"],
                record.header.broker_time_elapsed_ns
            );
            assert_eq!(record.payload["valid"], true);
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn synthetic_eye_replay_fixture_deserializes_through_broker_record_shape() {
        let session: BrokerSessionManifest = serde_json::from_str(include_str!(
            "../../../fixtures/replay/synthetic-eye-screen-gaze.session.json"
        ))
        .expect("fixture session should deserialize");
        let records = parse_replay_fixture_records(include_str!(
            "../../../fixtures/replay/synthetic-eye-screen-gaze.jsonl"
        ));

        assert!(session.is_valid());
        assert_eq!(session.session_id, "synthetic-eye-screen-gaze-session");
        assert_eq!(session.streams.len(), 1);
        assert_eq!(session.streams[0].stream_id, "eye.screen.gaze_point");
        assert_eq!(
            session.streams[0].payload_schema,
            "rusty.xr.eye.screen.gaze_point.v1"
        );
        assert_eq!(records.len(), 6);

        for (index, record) in records.iter().enumerate() {
            assert!(record.is_valid());
            assert_eq!(record.session_id, session.session_id);
            assert_eq!(record.stream, "eye.screen.gaze_point");
            assert_eq!(record.header.sequence_number, index as u64);
            assert_eq!(
                record.payload["schema"],
                "rusty.xr.eye.screen.gaze_point.v1"
            );
            assert_eq!(record.payload["base"]["sequence_number"], index as u64);
            assert_eq!(
                record.payload["base"]["sample_time_ns"],
                record
                    .header
                    .source_time_ns
                    .expect("fixture carries source time")
            );
        }

        let blink = &records[5].payload["base"]["validity"];
        assert_eq!(blink["sample_valid"], false);
        assert_eq!(blink["blink"], true);
        assert_eq!(blink["tracking_lost"], true);
    }

    #[cfg(feature = "serde")]
    fn parse_replay_fixture_records(jsonl: &str) -> Vec<BrokerReplayRecord<serde_json::Value>> {
        jsonl
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).expect("fixture record should deserialize"))
            .collect()
    }
}
