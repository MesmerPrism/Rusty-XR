//! Broker protocol and stream-manifest contracts for Rusty XR.
//!
//! This crate contains pure data models for broker control envelopes, stream
//! manifests, sample headers, timing stamps, drop counters, diagnostic binary
//! video headers, camera/source capabilities, H.264 stream invariants,
//! broker-described UI panels, stream registry snapshots, host manifests,
//! broker module manifests, lease-aware command authority contracts, and
//! negotiated transport lanes. It also models broker-owned clock snapshots,
//! stamps, health, and correlation reports. It does not open sockets, depend on
//! Android, or implement a Unity, Makepad, OpenXR, LSL, OSC, ZeroMQ, or video
//! backend.
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

pub mod control;
pub mod host;
pub mod module;
pub mod panel;
pub mod registry;

pub use control::*;
pub use host::*;
pub use module::*;
pub use panel::*;
pub use registry::*;

/// Crate version exposed for lightweight smoke checks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Versioned JSON schema id for broker client hello messages.
pub const BROKER_CLIENT_HELLO_SCHEMA: &str = "rusty.xr.broker.client_hello.v1";

/// Versioned JSON schema id for broker command messages.
pub const BROKER_COMMAND_SCHEMA: &str = "rusty.xr.broker.command.v1";

/// Versioned JSON schema id for broker command acknowledgement messages.
pub const BROKER_COMMAND_ACK_SCHEMA: &str = "rusty.xr.broker.command_ack.v1";

/// Versioned JSON schema id for broker command rejection details.
pub const BROKER_COMMAND_REJECTION_SCHEMA: &str = "rusty.xr.broker.command_rejection.v1";

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

/// Versioned JSON schema id for broker clock snapshots.
pub const BROKER_CLOCK_SNAPSHOT_SCHEMA: &str = "rusty.xr.clock.snapshot.v1";

/// Versioned JSON schema id for broker clock stamps on stored records.
pub const BROKER_CLOCK_STAMP_SCHEMA: &str = "rusty.xr.clock.stamp.v1";

/// Versioned JSON schema id for clock-domain correlation windows.
pub const BROKER_CLOCK_CORRELATION_SCHEMA: &str = "rusty.xr.clock.correlation.v1";

/// Versioned JSON schema id for broker clock health snapshots.
pub const BROKER_CLOCK_HEALTH_SCHEMA: &str = "rusty.xr.clock.health.v1";

/// Versioned JSON schema id for NTP-style clock sync probes.
pub const BROKER_CLOCK_SYNC_PROBE_SCHEMA: &str = "rusty.xr.clock.sync_probe.v1";

/// Versioned JSON schema id for broker transport session offers.
pub const BROKER_TRANSPORT_SESSION_OFFER_SCHEMA: &str =
    "rusty.xr.broker.transport_session_offer.v1";

/// Versioned JSON schema id for broker transport session answers.
pub const BROKER_TRANSPORT_SESSION_ANSWER_SCHEMA: &str =
    "rusty.xr.broker.transport_session_answer.v1";

/// Versioned JSON schema id for transport security policies.
pub const BROKER_TRANSPORT_SECURITY_POLICY_SCHEMA: &str =
    "rusty.xr.broker.transport_security_policy.v1";

/// Versioned JSON schema id for ZeroMQ bridge manifests.
pub const BROKER_ZEROMQ_BRIDGE_MANIFEST_SCHEMA: &str = "rusty.xr.broker.zeromq_bridge_manifest.v1";

/// Versioned JSON schema id for media sample timing reports.
pub const BROKER_MEDIA_SAMPLE_TIMING_SCHEMA: &str = "rusty.xr.broker.media_sample_timing.v1";

/// Versioned JSON schema id for network quality samples.
pub const BROKER_NETWORK_QUALITY_SAMPLE_SCHEMA: &str = "rusty.xr.broker.network_quality_sample.v1";

/// Versioned JSON schema id for Rusty XR diagnostic packet descriptors.
pub const BROKER_PACKET_DESCRIPTOR_SCHEMA: &str = "rusty.xr.broker.packet_descriptor.v1";

/// Versioned JSON schema id for camera/source capability manifests.
pub const BROKER_CAMERA_SOURCE_CAPABILITIES_SCHEMA: &str =
    "rusty.xr.broker.camera_source_capabilities.v1";

/// Versioned JSON schema id for H.264 stream invariant summaries.
pub const BROKER_H264_STREAM_INVARIANTS_SCHEMA: &str = "rusty.xr.broker.h264_stream_invariants.v1";

/// Public schema id for the Rusty XR-owned diagnostic binary video stream.
pub const BROKER_DIAGNOSTIC_VIDEO_STREAM_SCHEMA: &str = "rusty.xr.video_lab.binary_stream.v1";

/// Magic bytes for the Rusty XR-owned diagnostic video stream framing.
pub const BROKER_DIAGNOSTIC_VIDEO_MAGIC: &[u8; 8] = b"RXYRVID1";

/// Current Rusty XR-owned diagnostic binary video stream format version.
pub const BROKER_DIAGNOSTIC_VIDEO_BINARY_SCHEMA_VERSION: u32 = 3;

/// Fixed byte length of the diagnostic video stream header.
pub const BROKER_DIAGNOSTIC_VIDEO_HEADER_BYTES: usize = 32;

/// Fixed byte length of each v2/v3 diagnostic video packet header.
pub const BROKER_DIAGNOSTIC_VIDEO_PACKET_HEADER_BYTES: usize = 32;

/// Wire codec id for H.264 inside the diagnostic video stream framing.
pub const BROKER_DIAGNOSTIC_VIDEO_CODEC_H264: u32 = 1;

/// Maximum packet count accepted by bounded diagnostic video streams.
pub const BROKER_DIAGNOSTIC_VIDEO_MAX_PACKET_COUNT: u32 = 720;

/// Maximum payload size accepted for one diagnostic video packet.
pub const BROKER_DIAGNOSTIC_VIDEO_MAX_PACKET_BYTES: u32 = 1024 * 1024;

/// Maximum stream-header metadata payload for schema-3 diagnostic video streams.
pub const BROKER_DIAGNOSTIC_VIDEO_MAX_HEADER_METADATA_BYTES: u32 = 256 * 1024;

/// Diagnostic video packet flag for key frames.
pub const BROKER_DIAGNOSTIC_VIDEO_PACKET_FLAG_KEY_FRAME: u32 = 1;

/// Diagnostic video packet flag for codec configuration packets.
pub const BROKER_DIAGNOSTIC_VIDEO_PACKET_FLAG_CODEC_CONFIG: u32 = 2;

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

/// Maximum single message size advertised by a ZeroMQ bridge manifest.
pub const MAX_ZEROMQ_BRIDGE_MESSAGE_BYTES: u32 = 64 * 1024 * 1024;

/// Broker transport families that can carry a stream lane or control path.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerTransportKind {
    WebSocket,
    Tcp,
    ZeroMq,
    Udp,
    AdbForwardedTcp,
    Quic,
    WebTransport,
    WebRtcDiagnostic,
    ExternalSidecar,
    MetadataOnly,
}

/// ZeroMQ socket pattern expected by an external bridge.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerZeroMqPattern {
    Pair,
    PubSub,
    PushPull,
    RequestReply,
    DealerRouter,
}

/// Whether the bridge binds or connects the advertised ZeroMQ endpoint.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerZeroMqBindMode {
    Bind,
    Connect,
    Either,
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

/// Broad stream role for low-latency broker sessions.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerStreamKind {
    Media,
    Audio,
    Telemetry,
    Control,
    XrInput,
    Bio,
    Synthetic,
    Custom,
}

/// Codec or payload family negotiated for a transport stream.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerCodecId {
    H264,
    H265,
    Av1,
    RawLuma8,
    RawRgba8,
    Opus,
    PcmF32,
    Json,
    Custom,
}

/// Direction of samples on a negotiated stream.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerStreamDirection {
    ProducerToConsumer,
    ConsumerToProducer,
    Bidirectional,
    MetadataOnly,
}

/// Security gate required before a transport session can expose endpoints.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerSecurityMode {
    LoopbackOnly,
    PairingToken,
    PreSharedKey,
    ExternalSidecarOwned,
}

/// Lifecycle state for a negotiated transport session.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerTransportSessionState {
    Created,
    Offered,
    Accepted,
    Starting,
    Streaming,
    Draining,
    Closed,
    Failed,
}

/// Reason a packet or frame was dropped in a low-latency stream.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerPacketDropReason {
    LatePacket,
    DecodeTimeout,
    MissingKeyframe,
    SurfaceUnavailable,
    HardwareBufferImportFailed,
    ProjectionMetadataMissing,
    XrFrameBudgetExceeded,
    QueueOverflow,
    ClientShutdown,
    Unknown,
}

/// Timebase used by media timestamps in a camera or transport stream.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerTimestampDomain {
    ElapsedRealtime,
    CameraSensor,
    MediaPts,
    Unix,
    OpenXrPredictedDisplay,
    RelayReceive,
    Unknown,
}

/// Health state reported by a broker-owned clock service.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrokerClockHealthState {
    #[default]
    Healthy,
    Degraded,
    Unavailable,
}

/// Quality level for a clock-domain correlation estimate.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrokerClockCorrelationQuality {
    High,
    Medium,
    Low,
    #[default]
    Unavailable,
}

/// Reason the clock service marked a correlation window discontinuous.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrokerClockDiscontinuityReason {
    #[default]
    None,
    ServiceRestart,
    WallClockJump,
    SleepResume,
    RuntimeLoss,
    SampleGap,
    Unknown,
}

/// API path that produced a camera source capability report.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerCameraApiPath {
    AndroidCamera2,
    AndroidNdkCamera2,
    MetaPassthroughCameraApi,
    OpenXrPassthrough,
    Synthetic,
    Unknown,
}

/// Permission state observed for camera or headset-camera access.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerCameraPermissionState {
    Granted,
    Denied,
    Unavailable,
    NotRequired,
    Unknown,
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

    pub fn zeromq_tcp(host: impl Into<String>, port: u16) -> Self {
        Self {
            transport: BrokerTransportKind::ZeroMq,
            host: Some(host.into()),
            port: Some(port),
            path: None,
            channel_id: None,
            max_datagram_bytes: None,
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
            BrokerTransportKind::ZeroMq => {
                self.host
                    .as_deref()
                    .map(|host| !host.trim().is_empty())
                    .unwrap_or(false)
                    && self.port.map(|port| port > 0).unwrap_or(false)
                    && self.max_datagram_bytes.is_none()
            }
            BrokerTransportKind::Tcp
            | BrokerTransportKind::Udp
            | BrokerTransportKind::AdbForwardedTcp
            | BrokerTransportKind::Quic
            | BrokerTransportKind::WebTransport
            | BrokerTransportKind::WebRtcDiagnostic => {
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
            BrokerTransportKind::ExternalSidecar => self
                .channel_id
                .as_deref()
                .map(|channel_id| !channel_id.trim().is_empty())
                .unwrap_or(false),
            BrokerTransportKind::MetadataOnly => self
                .channel_id
                .as_deref()
                .map(|channel_id| !channel_id.trim().is_empty())
                .unwrap_or(false),
        }
    }

    pub fn is_loopback(&self) -> bool {
        matches!(
            self.transport,
            BrokerTransportKind::WebSocket
                | BrokerTransportKind::AdbForwardedTcp
                | BrokerTransportKind::MetadataOnly
        ) || self.host.as_deref().map(is_loopback_host).unwrap_or(false)
    }
}

/// Declarative manifest for a ZeroMQ bridge owned by an app or sidecar.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerZeroMqBridgeManifest {
    pub schema: String,
    pub bridge_id: String,
    pub endpoint: BrokerTransportEndpoint,
    pub pattern: BrokerZeroMqPattern,
    pub bind_mode: BrokerZeroMqBindMode,
    pub direction: BrokerStreamDirection,
    pub payload_kind: BrokerPayloadKind,
    pub payload_schema: String,
    pub stream_id: Option<String>,
    pub topic_prefix: Option<String>,
    pub max_message_bytes: Option<u32>,
    pub high_water_mark: Option<u32>,
    pub consent_data_categories: Vec<String>,
    pub notes: Vec<String>,
}

impl BrokerZeroMqBridgeManifest {
    pub fn new(
        bridge_id: impl Into<String>,
        endpoint: BrokerTransportEndpoint,
        pattern: BrokerZeroMqPattern,
        direction: BrokerStreamDirection,
        payload_kind: BrokerPayloadKind,
        payload_schema: impl Into<String>,
    ) -> Self {
        Self {
            schema: BROKER_ZEROMQ_BRIDGE_MANIFEST_SCHEMA.to_string(),
            bridge_id: bridge_id.into(),
            endpoint,
            pattern,
            bind_mode: BrokerZeroMqBindMode::Either,
            direction,
            payload_kind,
            payload_schema: payload_schema.into(),
            stream_id: None,
            topic_prefix: None,
            max_message_bytes: None,
            high_water_mark: None,
            consent_data_categories: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub const fn with_bind_mode(mut self, bind_mode: BrokerZeroMqBindMode) -> Self {
        self.bind_mode = bind_mode;
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub fn with_topic_prefix(mut self, topic_prefix: impl Into<String>) -> Self {
        self.topic_prefix = Some(topic_prefix.into());
        self
    }

    pub fn with_max_message_bytes(mut self, max_message_bytes: u32) -> Self {
        self.max_message_bytes = Some(max_message_bytes.min(MAX_ZEROMQ_BRIDGE_MESSAGE_BYTES));
        self
    }

    pub const fn with_high_water_mark(mut self, high_water_mark: u32) -> Self {
        self.high_water_mark = Some(high_water_mark);
        self
    }

    pub fn with_consent_data_category(mut self, category: impl Into<String>) -> Self {
        self.consent_data_categories.push(category.into());
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub const fn is_pub_sub(&self) -> bool {
        matches!(self.pattern, BrokerZeroMqPattern::PubSub)
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_ZEROMQ_BRIDGE_MANIFEST_SCHEMA
            && non_empty_string(&self.bridge_id)
            && self.endpoint.transport == BrokerTransportKind::ZeroMq
            && self.endpoint.is_valid()
            && non_empty_string(&self.payload_schema)
            && self
                .stream_id
                .as_deref()
                .map(non_empty_string)
                .unwrap_or(true)
            && self
                .topic_prefix
                .as_deref()
                .map(non_empty_string)
                .unwrap_or(true)
            && self
                .max_message_bytes
                .map(valid_zeromq_message_size)
                .unwrap_or(true)
            && self.high_water_mark.map(|value| value > 0).unwrap_or(true)
            && self
                .consent_data_categories
                .iter()
                .all(|category| non_empty_string(category))
            && self.notes.iter().all(|note| non_empty_string(note))
    }
}

/// Security policy attached to a low-latency transport session.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerTransportSecurityPolicy {
    pub schema: String,
    pub mode: BrokerSecurityMode,
    pub non_loopback_allowed: bool,
    pub pairing_token_required: bool,
    pub expires_elapsed_ns: Option<u64>,
    pub capability_scope: Vec<String>,
}

impl BrokerTransportSecurityPolicy {
    pub fn loopback_only() -> Self {
        Self {
            schema: BROKER_TRANSPORT_SECURITY_POLICY_SCHEMA.to_string(),
            mode: BrokerSecurityMode::LoopbackOnly,
            non_loopback_allowed: false,
            pairing_token_required: false,
            expires_elapsed_ns: None,
            capability_scope: Vec::new(),
        }
    }

    pub fn pairing_token(expires_elapsed_ns: u64) -> Self {
        Self {
            schema: BROKER_TRANSPORT_SECURITY_POLICY_SCHEMA.to_string(),
            mode: BrokerSecurityMode::PairingToken,
            non_loopback_allowed: true,
            pairing_token_required: true,
            expires_elapsed_ns: Some(expires_elapsed_ns),
            capability_scope: Vec::new(),
        }
    }

    pub fn with_capability_scope(mut self, capability: impl Into<String>) -> Self {
        self.capability_scope.push(capability.into());
        self
    }

    pub fn allows_endpoint(&self, endpoint: &BrokerTransportEndpoint) -> bool {
        endpoint.is_loopback() || self.non_loopback_allowed
    }

    pub fn is_valid(&self) -> bool {
        let mode_shape_valid = match self.mode {
            BrokerSecurityMode::LoopbackOnly => {
                !self.non_loopback_allowed && !self.pairing_token_required
            }
            BrokerSecurityMode::PairingToken => {
                self.non_loopback_allowed
                    && self.pairing_token_required
                    && self.expires_elapsed_ns.is_some()
            }
            BrokerSecurityMode::PreSharedKey | BrokerSecurityMode::ExternalSidecarOwned => {
                self.non_loopback_allowed
            }
        };

        mode_shape_valid
            && self
                .capability_scope
                .iter()
                .all(|capability| !capability.trim().is_empty())
    }
}

/// Stream descriptor used by transport session offers and answers.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerTransportStreamDescriptor {
    pub stream_id: String,
    pub stream_kind: BrokerStreamKind,
    pub direction: BrokerStreamDirection,
    pub payload_kind: BrokerPayloadKind,
    pub payload_schema: String,
    pub codec: Option<BrokerCodecId>,
    pub reliability: BrokerReliabilityClass,
    pub ordered: bool,
    pub nominal_rate_hz: Option<f32>,
    pub target_latency_ms: Option<f32>,
    pub max_payload_bytes: Option<u32>,
}

impl BrokerTransportStreamDescriptor {
    pub fn new(
        stream_id: impl Into<String>,
        stream_kind: BrokerStreamKind,
        payload_kind: BrokerPayloadKind,
        payload_schema: impl Into<String>,
    ) -> Self {
        Self {
            stream_id: stream_id.into(),
            stream_kind,
            direction: BrokerStreamDirection::ProducerToConsumer,
            payload_kind,
            payload_schema: payload_schema.into(),
            codec: None,
            reliability: BrokerReliabilityClass::Reliable,
            ordered: true,
            nominal_rate_hz: None,
            target_latency_ms: None,
            max_payload_bytes: None,
        }
    }

    pub const fn with_direction(mut self, direction: BrokerStreamDirection) -> Self {
        self.direction = direction;
        self
    }

    pub const fn with_codec(mut self, codec: BrokerCodecId) -> Self {
        self.codec = Some(codec);
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

    pub fn with_nominal_rate_hz(mut self, nominal_rate_hz: f32) -> Self {
        self.nominal_rate_hz = Some(nominal_rate_hz);
        self
    }

    pub fn with_target_latency_ms(mut self, target_latency_ms: f32) -> Self {
        self.target_latency_ms = Some(target_latency_ms);
        self
    }

    pub fn with_max_payload_bytes(mut self, max_payload_bytes: u32) -> Self {
        self.max_payload_bytes = Some(max_payload_bytes);
        self
    }

    pub fn is_valid(&self) -> bool {
        !self.stream_id.trim().is_empty()
            && !self.payload_schema.trim().is_empty()
            && self
                .nominal_rate_hz
                .map(|rate| rate.is_finite() && rate > 0.0)
                .unwrap_or(true)
            && self
                .target_latency_ms
                .map(|latency| latency.is_finite() && latency >= 0.0)
                .unwrap_or(true)
            && self.max_payload_bytes.map(|size| size > 0).unwrap_or(true)
    }
}

/// Clean-room transport session offer from a client or operator.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerTransportSessionOffer {
    pub schema: String,
    pub session_id: String,
    pub client_id: String,
    pub requested_transports: Vec<BrokerTransportKind>,
    pub streams: Vec<BrokerTransportStreamDescriptor>,
    pub security: BrokerTransportSecurityPolicy,
    pub target_latency_ms: Option<f32>,
}

impl BrokerTransportSessionOffer {
    pub fn new(session_id: impl Into<String>, client_id: impl Into<String>) -> Self {
        Self {
            schema: BROKER_TRANSPORT_SESSION_OFFER_SCHEMA.to_string(),
            session_id: session_id.into(),
            client_id: client_id.into(),
            requested_transports: Vec::new(),
            streams: Vec::new(),
            security: BrokerTransportSecurityPolicy::loopback_only(),
            target_latency_ms: None,
        }
    }

    pub fn with_transport(mut self, transport: BrokerTransportKind) -> Self {
        self.requested_transports.push(transport);
        self
    }

    pub fn with_stream(mut self, stream: BrokerTransportStreamDescriptor) -> Self {
        self.streams.push(stream);
        self
    }

    pub fn with_security(mut self, security: BrokerTransportSecurityPolicy) -> Self {
        self.security = security;
        self
    }

    pub fn with_target_latency_ms(mut self, target_latency_ms: f32) -> Self {
        self.target_latency_ms = Some(target_latency_ms);
        self
    }

    pub fn is_valid(&self) -> bool {
        !self.session_id.trim().is_empty()
            && !self.client_id.trim().is_empty()
            && !self.requested_transports.is_empty()
            && !self.streams.is_empty()
            && self
                .streams
                .iter()
                .all(BrokerTransportStreamDescriptor::is_valid)
            && self.security.is_valid()
            && self
                .target_latency_ms
                .map(|latency| latency.is_finite() && latency >= 0.0)
                .unwrap_or(true)
    }
}

/// Broker answer to a clean-room transport session offer.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerTransportSessionAnswer {
    pub schema: String,
    pub session_id: String,
    pub accepted: bool,
    pub state: BrokerTransportSessionState,
    pub selected_transport: Option<BrokerTransportKind>,
    pub accepted_streams: Vec<BrokerTransportStreamDescriptor>,
    pub security: BrokerTransportSecurityPolicy,
    pub reason: Option<String>,
}

impl BrokerTransportSessionAnswer {
    pub fn accepted(
        session_id: impl Into<String>,
        selected_transport: BrokerTransportKind,
        security: BrokerTransportSecurityPolicy,
    ) -> Self {
        Self {
            schema: BROKER_TRANSPORT_SESSION_ANSWER_SCHEMA.to_string(),
            session_id: session_id.into(),
            accepted: true,
            state: BrokerTransportSessionState::Accepted,
            selected_transport: Some(selected_transport),
            accepted_streams: Vec::new(),
            security,
            reason: None,
        }
    }

    pub fn rejected(session_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            schema: BROKER_TRANSPORT_SESSION_ANSWER_SCHEMA.to_string(),
            session_id: session_id.into(),
            accepted: false,
            state: BrokerTransportSessionState::Failed,
            selected_transport: None,
            accepted_streams: Vec::new(),
            security: BrokerTransportSecurityPolicy::loopback_only(),
            reason: Some(reason.into()),
        }
    }

    pub fn with_stream(mut self, stream: BrokerTransportStreamDescriptor) -> Self {
        self.accepted_streams.push(stream);
        self
    }

    pub fn is_valid(&self) -> bool {
        !self.session_id.trim().is_empty()
            && self.security.is_valid()
            && self
                .accepted_streams
                .iter()
                .all(BrokerTransportStreamDescriptor::is_valid)
            && if self.accepted {
                self.selected_transport.is_some()
                    && !matches!(
                        self.state,
                        BrokerTransportSessionState::Failed | BrokerTransportSessionState::Closed
                    )
            } else {
                self.reason
                    .as_deref()
                    .map(|reason| !reason.trim().is_empty())
                    .unwrap_or(false)
            }
    }
}

/// Per-sample timing report for media and XR submission diagnostics.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerMediaSampleTiming {
    pub schema: String,
    pub session_id: String,
    pub stream_id: String,
    pub sequence_number: u64,
    pub source_capture_time_ns: Option<u64>,
    pub encode_start_time_ns: Option<u64>,
    pub encode_done_time_ns: Option<u64>,
    pub packet_send_time_ns: Option<u64>,
    pub packet_receive_time_ns: Option<u64>,
    pub decode_start_time_ns: Option<u64>,
    pub decode_done_time_ns: Option<u64>,
    pub texture_import_time_ns: Option<u64>,
    pub xr_submit_time_ns: Option<u64>,
    pub present_estimate_time_ns: Option<u64>,
}

impl BrokerMediaSampleTiming {
    pub fn new(
        session_id: impl Into<String>,
        stream_id: impl Into<String>,
        sequence_number: u64,
    ) -> Self {
        Self {
            schema: BROKER_MEDIA_SAMPLE_TIMING_SCHEMA.to_string(),
            session_id: session_id.into(),
            stream_id: stream_id.into(),
            sequence_number,
            source_capture_time_ns: None,
            encode_start_time_ns: None,
            encode_done_time_ns: None,
            packet_send_time_ns: None,
            packet_receive_time_ns: None,
            decode_start_time_ns: None,
            decode_done_time_ns: None,
            texture_import_time_ns: None,
            xr_submit_time_ns: None,
            present_estimate_time_ns: None,
        }
    }

    pub const fn with_source_capture_time_ns(mut self, value: u64) -> Self {
        self.source_capture_time_ns = Some(value);
        self
    }

    pub const fn with_packet_receive_time_ns(mut self, value: u64) -> Self {
        self.packet_receive_time_ns = Some(value);
        self
    }

    pub const fn with_decode_done_time_ns(mut self, value: u64) -> Self {
        self.decode_done_time_ns = Some(value);
        self
    }

    pub const fn with_texture_import_time_ns(mut self, value: u64) -> Self {
        self.texture_import_time_ns = Some(value);
        self
    }

    pub const fn with_xr_submit_time_ns(mut self, value: u64) -> Self {
        self.xr_submit_time_ns = Some(value);
        self
    }

    pub fn source_to_receive_latency_ns(&self) -> Option<u64> {
        self.packet_receive_time_ns?
            .checked_sub(self.source_capture_time_ns?)
    }

    pub fn receive_to_decode_latency_ns(&self) -> Option<u64> {
        self.decode_done_time_ns?
            .checked_sub(self.packet_receive_time_ns?)
    }

    pub fn decode_to_submit_latency_ns(&self) -> Option<u64> {
        self.xr_submit_time_ns?
            .checked_sub(self.decode_done_time_ns?)
    }

    pub fn is_valid(&self) -> bool {
        !self.session_id.trim().is_empty()
            && !self.stream_id.trim().is_empty()
            && ordered_optional_pair(self.encode_start_time_ns, self.encode_done_time_ns)
            && ordered_optional_pair(self.packet_send_time_ns, self.packet_receive_time_ns)
            && ordered_optional_pair(self.decode_start_time_ns, self.decode_done_time_ns)
            && ordered_optional_pair(self.decode_done_time_ns, self.texture_import_time_ns)
            && ordered_optional_pair(self.texture_import_time_ns, self.xr_submit_time_ns)
            && ordered_optional_pair(self.xr_submit_time_ns, self.present_estimate_time_ns)
    }
}

/// Network and jitter quality sample for a transport session or stream.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerNetworkQualitySample {
    pub schema: String,
    pub session_id: String,
    pub stream_id: Option<String>,
    pub measured_time_elapsed_ns: u64,
    pub packet_loss_estimate01: Option<f32>,
    pub late_packet_count: u64,
    pub decode_gap_count: u64,
    pub jitter_buffer_depth: u32,
    pub target_latency_ms: Option<f32>,
    pub actual_latency_ms: Option<f32>,
    pub clock_sync_quality01: Option<f32>,
}

impl BrokerNetworkQualitySample {
    pub fn new(session_id: impl Into<String>, measured_time_elapsed_ns: u64) -> Self {
        Self {
            schema: BROKER_NETWORK_QUALITY_SAMPLE_SCHEMA.to_string(),
            session_id: session_id.into(),
            stream_id: None,
            measured_time_elapsed_ns,
            packet_loss_estimate01: None,
            late_packet_count: 0,
            decode_gap_count: 0,
            jitter_buffer_depth: 0,
            target_latency_ms: None,
            actual_latency_ms: None,
            clock_sync_quality01: None,
        }
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub fn with_packet_loss_estimate01(mut self, packet_loss_estimate01: f32) -> Self {
        self.packet_loss_estimate01 = Some(packet_loss_estimate01);
        self
    }

    pub fn with_target_latency_ms(mut self, target_latency_ms: f32) -> Self {
        self.target_latency_ms = Some(target_latency_ms);
        self
    }

    pub fn with_actual_latency_ms(mut self, actual_latency_ms: f32) -> Self {
        self.actual_latency_ms = Some(actual_latency_ms);
        self
    }

    pub fn is_valid(&self) -> bool {
        !self.session_id.trim().is_empty()
            && self
                .stream_id
                .as_deref()
                .map(|stream_id| !stream_id.trim().is_empty())
                .unwrap_or(true)
            && self
                .packet_loss_estimate01
                .map(valid_unit_interval)
                .unwrap_or(true)
            && self
                .clock_sync_quality01
                .map(valid_unit_interval)
                .unwrap_or(true)
            && self
                .target_latency_ms
                .map(|latency| latency.is_finite() && latency >= 0.0)
                .unwrap_or(true)
            && self
                .actual_latency_ms
                .map(|latency| latency.is_finite() && latency >= 0.0)
                .unwrap_or(true)
    }
}

/// Rusty XR-owned packet descriptor for diagnostic binary payloads.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerPacketDescriptor {
    pub schema: String,
    pub session_id: String,
    pub stream_id: String,
    pub sequence_number: u64,
    pub payload_kind: BrokerPayloadKind,
    pub payload_byte_len: u32,
    pub key_frame: bool,
    pub drop_reason: Option<BrokerPacketDropReason>,
}

impl BrokerPacketDescriptor {
    pub fn new(
        session_id: impl Into<String>,
        stream_id: impl Into<String>,
        sequence_number: u64,
        payload_kind: BrokerPayloadKind,
        payload_byte_len: u32,
    ) -> Self {
        Self {
            schema: BROKER_PACKET_DESCRIPTOR_SCHEMA.to_string(),
            session_id: session_id.into(),
            stream_id: stream_id.into(),
            sequence_number,
            payload_kind,
            payload_byte_len,
            key_frame: false,
            drop_reason: None,
        }
    }

    pub const fn with_key_frame(mut self, key_frame: bool) -> Self {
        self.key_frame = key_frame;
        self
    }

    pub const fn with_drop_reason(mut self, drop_reason: BrokerPacketDropReason) -> Self {
        self.drop_reason = Some(drop_reason);
        self
    }

    pub fn is_valid(&self) -> bool {
        !self.session_id.trim().is_empty()
            && !self.stream_id.trim().is_empty()
            && self.payload_byte_len > 0
    }
}

/// Width/height tuple used by camera-source and H.264 stream contracts.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrokerVideoSize {
    pub width: u32,
    pub height: u32,
}

impl BrokerVideoSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub const fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }
}

/// Inclusive frame-rate range advertised or selected for a camera stream.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrokerFpsRange {
    pub min_hz: u32,
    pub max_hz: u32,
}

impl BrokerFpsRange {
    pub const fn new(min_hz: u32, max_hz: u32) -> Self {
        Self { min_hz, max_hz }
    }

    pub const fn is_valid(&self) -> bool {
        self.min_hz > 0 && self.max_hz >= self.min_hz
    }
}

/// Public camera/source capability manifest used before stream interpretation.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerCameraSourceCapabilities {
    pub schema: String,
    pub source_id: String,
    pub source_api_path: BrokerCameraApiPath,
    pub horizon_os_version_observed: Option<String>,
    pub camera_permission_state: BrokerCameraPermissionState,
    pub headset_camera_permission_state: BrokerCameraPermissionState,
    pub camera_id: Option<String>,
    pub physical_camera_ids: Vec<String>,
    pub meta_vendor_camera_source: Option<String>,
    pub meta_vendor_position: Option<String>,
    pub supported_private_sizes: Vec<BrokerVideoSize>,
    pub supported_yuv_sizes: Vec<BrokerVideoSize>,
    pub supported_fps_ranges: Vec<BrokerFpsRange>,
    pub selected_size: Option<BrokerVideoSize>,
    pub selected_fps_range: Option<BrokerFpsRange>,
    pub stream_min_frame_duration_ns: Option<u64>,
    pub timestamp_domain: BrokerTimestampDomain,
    pub selected_reason: Option<String>,
}

impl BrokerCameraSourceCapabilities {
    pub fn new(source_id: impl Into<String>, source_api_path: BrokerCameraApiPath) -> Self {
        Self {
            schema: BROKER_CAMERA_SOURCE_CAPABILITIES_SCHEMA.to_string(),
            source_id: source_id.into(),
            source_api_path,
            horizon_os_version_observed: None,
            camera_permission_state: BrokerCameraPermissionState::Unknown,
            headset_camera_permission_state: BrokerCameraPermissionState::Unknown,
            camera_id: None,
            physical_camera_ids: Vec::new(),
            meta_vendor_camera_source: None,
            meta_vendor_position: None,
            supported_private_sizes: Vec::new(),
            supported_yuv_sizes: Vec::new(),
            supported_fps_ranges: Vec::new(),
            selected_size: None,
            selected_fps_range: None,
            stream_min_frame_duration_ns: None,
            timestamp_domain: BrokerTimestampDomain::Unknown,
            selected_reason: None,
        }
    }

    pub const fn with_camera_permission_state(
        mut self,
        state: BrokerCameraPermissionState,
    ) -> Self {
        self.camera_permission_state = state;
        self
    }

    pub const fn with_headset_camera_permission_state(
        mut self,
        state: BrokerCameraPermissionState,
    ) -> Self {
        self.headset_camera_permission_state = state;
        self
    }

    pub fn with_camera_id(mut self, camera_id: impl Into<String>) -> Self {
        self.camera_id = Some(camera_id.into());
        self
    }

    pub const fn with_timestamp_domain(mut self, domain: BrokerTimestampDomain) -> Self {
        self.timestamp_domain = domain;
        self
    }

    pub fn with_selected_size(
        mut self,
        selected_size: BrokerVideoSize,
        selected_reason: impl Into<String>,
    ) -> Self {
        self.selected_size = Some(selected_size);
        self.selected_reason = Some(selected_reason.into());
        self
    }

    pub const fn with_selected_fps_range(mut self, selected_fps_range: BrokerFpsRange) -> Self {
        self.selected_fps_range = Some(selected_fps_range);
        self
    }

    pub const fn with_stream_min_frame_duration_ns(
        mut self,
        stream_min_frame_duration_ns: u64,
    ) -> Self {
        self.stream_min_frame_duration_ns = Some(stream_min_frame_duration_ns);
        self
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_CAMERA_SOURCE_CAPABILITIES_SCHEMA
            && !self.source_id.trim().is_empty()
            && self
                .horizon_os_version_observed
                .as_deref()
                .map(non_empty_string)
                .unwrap_or(true)
            && self
                .camera_id
                .as_deref()
                .map(non_empty_string)
                .unwrap_or(true)
            && self
                .physical_camera_ids
                .iter()
                .all(|id| non_empty_string(id))
            && self
                .meta_vendor_camera_source
                .as_deref()
                .map(non_empty_string)
                .unwrap_or(true)
            && self
                .meta_vendor_position
                .as_deref()
                .map(non_empty_string)
                .unwrap_or(true)
            && self
                .supported_private_sizes
                .iter()
                .all(BrokerVideoSize::is_valid)
            && self
                .supported_yuv_sizes
                .iter()
                .all(BrokerVideoSize::is_valid)
            && self
                .supported_fps_ranges
                .iter()
                .all(BrokerFpsRange::is_valid)
            && self
                .selected_size
                .as_ref()
                .map(BrokerVideoSize::is_valid)
                .unwrap_or(true)
            && self
                .selected_fps_range
                .as_ref()
                .map(BrokerFpsRange::is_valid)
                .unwrap_or(true)
            && self
                .stream_min_frame_duration_ns
                .map(|duration| duration > 0)
                .unwrap_or(true)
            && self
                .selected_reason
                .as_deref()
                .map(non_empty_string)
                .unwrap_or(self.selected_size.is_none())
    }
}

/// Public H.264 stream invariant summary used by stream-health gates.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerH264StreamInvariants {
    pub schema: String,
    pub session_id: String,
    pub stream_id: String,
    pub role: String,
    pub direction: BrokerStreamDirection,
    pub peer_id: Option<String>,
    pub track_id: Option<String>,
    pub eye: Option<String>,
    pub bitstream_format: String,
    pub encoder_name: Option<String>,
    pub decoder_name: Option<String>,
    pub width: u32,
    pub height: u32,
    pub bitrate_bps: Option<u32>,
    pub bitrate_mode_requested: Option<String>,
    pub bitrate_mode_applied: Option<String>,
    pub i_frame_interval_seconds: Option<u32>,
    pub encoder_latency_requested_frames: Option<u32>,
    pub encoder_latency_applied_frames: Option<u32>,
    pub decoder_low_latency_config_requested: Option<bool>,
    pub decoder_low_latency_parameter_succeeded: Option<bool>,
    pub codec_config_packet_count: u64,
    pub sps_present: bool,
    pub pps_present: bool,
    pub keyframe_count: u64,
    pub sync_frame_request_count: u64,
    pub sync_frame_request_on_start_succeeded: Option<bool>,
    pub decoder_output_mode: Option<String>,
    pub hardware_buffer_import_succeeded: Option<bool>,
    pub close_reason: Option<String>,
}

impl BrokerH264StreamInvariants {
    pub fn new(
        session_id: impl Into<String>,
        stream_id: impl Into<String>,
        role: impl Into<String>,
        direction: BrokerStreamDirection,
        size: BrokerVideoSize,
    ) -> Self {
        Self {
            schema: BROKER_H264_STREAM_INVARIANTS_SCHEMA.to_string(),
            session_id: session_id.into(),
            stream_id: stream_id.into(),
            role: role.into(),
            direction,
            peer_id: None,
            track_id: None,
            eye: None,
            bitstream_format: "AnnexB".to_string(),
            encoder_name: None,
            decoder_name: None,
            width: size.width,
            height: size.height,
            bitrate_bps: None,
            bitrate_mode_requested: None,
            bitrate_mode_applied: None,
            i_frame_interval_seconds: None,
            encoder_latency_requested_frames: None,
            encoder_latency_applied_frames: None,
            decoder_low_latency_config_requested: None,
            decoder_low_latency_parameter_succeeded: None,
            codec_config_packet_count: 0,
            sps_present: false,
            pps_present: false,
            keyframe_count: 0,
            sync_frame_request_count: 0,
            sync_frame_request_on_start_succeeded: None,
            decoder_output_mode: None,
            hardware_buffer_import_succeeded: None,
            close_reason: None,
        }
    }

    pub fn with_peer_id(mut self, peer_id: impl Into<String>) -> Self {
        self.peer_id = Some(peer_id.into());
        self
    }

    pub fn with_track_id(mut self, track_id: impl Into<String>) -> Self {
        self.track_id = Some(track_id.into());
        self
    }

    pub fn with_eye(mut self, eye: impl Into<String>) -> Self {
        self.eye = Some(eye.into());
        self
    }

    pub fn with_encoder_name(mut self, encoder_name: impl Into<String>) -> Self {
        self.encoder_name = Some(encoder_name.into());
        self
    }

    pub fn with_decoder_name(mut self, decoder_name: impl Into<String>) -> Self {
        self.decoder_name = Some(decoder_name.into());
        self
    }

    pub const fn with_bitrate_bps(mut self, bitrate_bps: u32) -> Self {
        self.bitrate_bps = Some(bitrate_bps);
        self
    }

    pub fn with_bitrate_modes(
        mut self,
        requested: impl Into<String>,
        applied: impl Into<String>,
    ) -> Self {
        self.bitrate_mode_requested = Some(requested.into());
        self.bitrate_mode_applied = Some(applied.into());
        self
    }

    pub const fn with_h264_start_config(
        mut self,
        codec_config_packet_count: u64,
        sps_present: bool,
        pps_present: bool,
        keyframe_count: u64,
    ) -> Self {
        self.codec_config_packet_count = codec_config_packet_count;
        self.sps_present = sps_present;
        self.pps_present = pps_present;
        self.keyframe_count = keyframe_count;
        self
    }

    pub const fn with_sync_frame_request_on_start_succeeded(mut self, succeeded: bool) -> Self {
        self.sync_frame_request_count = 1;
        self.sync_frame_request_on_start_succeeded = Some(succeeded);
        self
    }

    pub fn has_h264_start_config(&self) -> bool {
        self.codec_config_packet_count > 0
            && self.sps_present
            && self.pps_present
            && self.keyframe_count > 0
    }

    pub fn has_named_codec_components(&self) -> bool {
        self.encoder_name
            .as_deref()
            .map(non_empty_string)
            .unwrap_or(false)
            && self
                .decoder_name
                .as_deref()
                .map(non_empty_string)
                .unwrap_or(false)
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_H264_STREAM_INVARIANTS_SCHEMA
            && !self.session_id.trim().is_empty()
            && !self.stream_id.trim().is_empty()
            && !self.role.trim().is_empty()
            && !self.bitstream_format.trim().is_empty()
            && BrokerVideoSize::new(self.width, self.height).is_valid()
            && self.bitrate_bps.map(|bitrate| bitrate > 0).unwrap_or(true)
            && self
                .peer_id
                .as_deref()
                .map(non_empty_string)
                .unwrap_or(true)
            && self
                .track_id
                .as_deref()
                .map(non_empty_string)
                .unwrap_or(true)
            && self.eye.as_deref().map(non_empty_string).unwrap_or(true)
            && self
                .encoder_name
                .as_deref()
                .map(non_empty_string)
                .unwrap_or(true)
            && self
                .decoder_name
                .as_deref()
                .map(non_empty_string)
                .unwrap_or(true)
            && self
                .bitrate_mode_requested
                .as_deref()
                .map(non_empty_string)
                .unwrap_or(true)
            && self
                .bitrate_mode_applied
                .as_deref()
                .map(non_empty_string)
                .unwrap_or(true)
            && self
                .decoder_output_mode
                .as_deref()
                .map(non_empty_string)
                .unwrap_or(true)
            && self
                .close_reason
                .as_deref()
                .map(non_empty_string)
                .unwrap_or(true)
    }
}

/// Parse or write failure for Rusty XR diagnostic binary video framing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrokerDiagnosticVideoFormatError {
    ShortHeader { expected: usize, actual: usize },
    InvalidMagic { actual: [u8; 8] },
    UnsupportedSchemaVersion(i64),
    UnsupportedCodecId(i64),
    UnsupportedCodec(BrokerCodecId),
    InvalidDimensions { width: i64, height: i64 },
    InvalidPacketCount(i64),
    InvalidDeclaredPacketBytes(i64),
    InvalidHeaderMetadataBytes(i64),
    InvalidPayloadByteLen(i64),
    InvalidFlags(u64),
    InvalidTimestamp { field: &'static str, value: i128 },
    DeclaredPacketBytesMismatch { declared: u32, actual: u32 },
}

/// Fixed Rusty XR-owned diagnostic video stream header.
///
/// This is a clean-room bounded diagnostic format used by public examples and
/// tests. It is not a compatibility claim for any external low-latency SDK.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerDiagnosticVideoStreamHeader {
    pub schema: String,
    pub schema_version: u32,
    pub codec: BrokerCodecId,
    pub width: u32,
    pub height: u32,
    pub packet_count: u32,
    pub declared_packet_bytes: Option<u32>,
    pub header_metadata_bytes: u32,
}

impl BrokerDiagnosticVideoStreamHeader {
    pub fn h264(width: u32, height: u32, packet_count: u32) -> Self {
        Self {
            schema: BROKER_DIAGNOSTIC_VIDEO_STREAM_SCHEMA.to_string(),
            schema_version: BROKER_DIAGNOSTIC_VIDEO_BINARY_SCHEMA_VERSION,
            codec: BrokerCodecId::H264,
            width,
            height,
            packet_count,
            declared_packet_bytes: None,
            header_metadata_bytes: 0,
        }
    }

    pub const fn with_schema_version(mut self, schema_version: u32) -> Self {
        self.schema_version = schema_version;
        self
    }

    pub const fn with_declared_packet_bytes(mut self, declared_packet_bytes: u32) -> Self {
        self.declared_packet_bytes = Some(declared_packet_bytes);
        self
    }

    pub const fn with_header_metadata_bytes(mut self, header_metadata_bytes: u32) -> Self {
        self.header_metadata_bytes = header_metadata_bytes;
        self
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_DIAGNOSTIC_VIDEO_STREAM_SCHEMA && self.validate_for_encode().is_ok()
    }

    pub fn encode(
        &self,
    ) -> Result<[u8; BROKER_DIAGNOSTIC_VIDEO_HEADER_BYTES], BrokerDiagnosticVideoFormatError> {
        self.validate_for_encode()?;

        let mut bytes = [0u8; BROKER_DIAGNOSTIC_VIDEO_HEADER_BYTES];
        bytes[..BROKER_DIAGNOSTIC_VIDEO_MAGIC.len()].copy_from_slice(BROKER_DIAGNOSTIC_VIDEO_MAGIC);
        write_i32_be(&mut bytes, 8, self.schema_version as i32);
        write_i32_be(
            &mut bytes,
            12,
            diagnostic_video_codec_wire_id(self.codec).unwrap_or_default() as i32,
        );
        write_i32_be(&mut bytes, 16, self.width as i32);
        write_i32_be(&mut bytes, 20, self.height as i32);
        write_i32_be(&mut bytes, 24, self.packet_count as i32);
        write_i32_be(
            &mut bytes,
            28,
            if self.schema_version >= 3 {
                self.header_metadata_bytes as i32
            } else {
                self.declared_packet_bytes.unwrap_or_default() as i32
            },
        );
        Ok(bytes)
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, BrokerDiagnosticVideoFormatError> {
        if bytes.len() < BROKER_DIAGNOSTIC_VIDEO_HEADER_BYTES {
            return Err(BrokerDiagnosticVideoFormatError::ShortHeader {
                expected: BROKER_DIAGNOSTIC_VIDEO_HEADER_BYTES,
                actual: bytes.len(),
            });
        }

        let mut magic = [0u8; 8];
        magic.copy_from_slice(&bytes[..8]);
        if &magic != BROKER_DIAGNOSTIC_VIDEO_MAGIC {
            return Err(BrokerDiagnosticVideoFormatError::InvalidMagic { actual: magic });
        }

        let schema_version = read_i32_be(bytes, 8) as i64;
        if schema_version < 1
            || schema_version > BROKER_DIAGNOSTIC_VIDEO_BINARY_SCHEMA_VERSION as i64
        {
            return Err(BrokerDiagnosticVideoFormatError::UnsupportedSchemaVersion(
                schema_version,
            ));
        }

        let codec_id = read_i32_be(bytes, 12) as i64;
        let codec = diagnostic_video_codec_from_wire_id(codec_id)?;
        let width = read_i32_be(bytes, 16) as i64;
        let height = read_i32_be(bytes, 20) as i64;
        if width <= 0 || height <= 0 {
            return Err(BrokerDiagnosticVideoFormatError::InvalidDimensions { width, height });
        }

        let packet_count = read_i32_be(bytes, 24) as i64;
        if !(1..=BROKER_DIAGNOSTIC_VIDEO_MAX_PACKET_COUNT as i64).contains(&packet_count) {
            return Err(BrokerDiagnosticVideoFormatError::InvalidPacketCount(
                packet_count,
            ));
        }

        let tail_word = read_i32_be(bytes, 28) as i64;
        let (declared_packet_bytes, header_metadata_bytes) = if schema_version >= 3 {
            if tail_word < 0 || tail_word > BROKER_DIAGNOSTIC_VIDEO_MAX_HEADER_METADATA_BYTES as i64
            {
                return Err(
                    BrokerDiagnosticVideoFormatError::InvalidHeaderMetadataBytes(tail_word),
                );
            }
            (None, tail_word as u32)
        } else {
            if tail_word < 0 || tail_word > BROKER_DIAGNOSTIC_VIDEO_MAX_PACKET_BYTES as i64 {
                return Err(
                    BrokerDiagnosticVideoFormatError::InvalidDeclaredPacketBytes(tail_word),
                );
            }
            ((tail_word > 0).then_some(tail_word as u32), 0)
        };

        Ok(Self {
            schema: BROKER_DIAGNOSTIC_VIDEO_STREAM_SCHEMA.to_string(),
            schema_version: schema_version as u32,
            codec,
            width: width as u32,
            height: height as u32,
            packet_count: packet_count as u32,
            declared_packet_bytes,
            header_metadata_bytes,
        })
    }

    fn validate_for_encode(&self) -> Result<(), BrokerDiagnosticVideoFormatError> {
        if self.schema_version < 1
            || self.schema_version > BROKER_DIAGNOSTIC_VIDEO_BINARY_SCHEMA_VERSION
        {
            return Err(BrokerDiagnosticVideoFormatError::UnsupportedSchemaVersion(
                self.schema_version as i64,
            ));
        }
        if diagnostic_video_codec_wire_id(self.codec).is_none() {
            return Err(BrokerDiagnosticVideoFormatError::UnsupportedCodec(
                self.codec,
            ));
        }
        if self.width == 0
            || self.width > i32::MAX as u32
            || self.height == 0
            || self.height > i32::MAX as u32
        {
            return Err(BrokerDiagnosticVideoFormatError::InvalidDimensions {
                width: self.width as i64,
                height: self.height as i64,
            });
        }
        if !(1..=BROKER_DIAGNOSTIC_VIDEO_MAX_PACKET_COUNT).contains(&self.packet_count) {
            return Err(BrokerDiagnosticVideoFormatError::InvalidPacketCount(
                self.packet_count as i64,
            ));
        }
        if self.schema_version >= 3 {
            if self.declared_packet_bytes.is_some() {
                return Err(
                    BrokerDiagnosticVideoFormatError::InvalidDeclaredPacketBytes(
                        self.declared_packet_bytes.unwrap_or_default() as i64,
                    ),
                );
            }
            if self.header_metadata_bytes > BROKER_DIAGNOSTIC_VIDEO_MAX_HEADER_METADATA_BYTES {
                return Err(
                    BrokerDiagnosticVideoFormatError::InvalidHeaderMetadataBytes(
                        self.header_metadata_bytes as i64,
                    ),
                );
            }
        } else {
            if self.header_metadata_bytes != 0 {
                return Err(
                    BrokerDiagnosticVideoFormatError::InvalidHeaderMetadataBytes(
                        self.header_metadata_bytes as i64,
                    ),
                );
            }
        }
        if let Some(declared_packet_bytes) = self.declared_packet_bytes {
            if !valid_diagnostic_packet_bytes(declared_packet_bytes) {
                return Err(
                    BrokerDiagnosticVideoFormatError::InvalidDeclaredPacketBytes(
                        declared_packet_bytes as i64,
                    ),
                );
            }
        }
        Ok(())
    }
}

/// Fixed Rusty XR-owned diagnostic video packet header.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerDiagnosticVideoPacketHeader {
    pub pts_us: u64,
    pub flags: u32,
    pub payload_byte_len: u32,
    pub source_time_elapsed_ns: u64,
    pub source_time_unix_ns: u64,
}

impl BrokerDiagnosticVideoPacketHeader {
    pub const fn new(pts_us: u64, payload_byte_len: u32) -> Self {
        Self {
            pts_us,
            flags: 0,
            payload_byte_len,
            source_time_elapsed_ns: 0,
            source_time_unix_ns: 0,
        }
    }

    pub const fn with_key_frame(mut self, key_frame: bool) -> Self {
        if key_frame {
            self.flags |= BROKER_DIAGNOSTIC_VIDEO_PACKET_FLAG_KEY_FRAME;
        } else {
            self.flags &= !BROKER_DIAGNOSTIC_VIDEO_PACKET_FLAG_KEY_FRAME;
        }
        self
    }

    pub const fn with_codec_config(mut self, codec_config: bool) -> Self {
        if codec_config {
            self.flags |= BROKER_DIAGNOSTIC_VIDEO_PACKET_FLAG_CODEC_CONFIG;
        } else {
            self.flags &= !BROKER_DIAGNOSTIC_VIDEO_PACKET_FLAG_CODEC_CONFIG;
        }
        self
    }

    pub const fn with_source_times(
        mut self,
        source_time_elapsed_ns: u64,
        source_time_unix_ns: u64,
    ) -> Self {
        self.source_time_elapsed_ns = source_time_elapsed_ns;
        self.source_time_unix_ns = source_time_unix_ns;
        self
    }

    pub const fn is_key_frame(&self) -> bool {
        self.flags & BROKER_DIAGNOSTIC_VIDEO_PACKET_FLAG_KEY_FRAME != 0
    }

    pub const fn is_codec_config(&self) -> bool {
        self.flags & BROKER_DIAGNOSTIC_VIDEO_PACKET_FLAG_CODEC_CONFIG != 0
    }

    pub fn is_valid(&self) -> bool {
        valid_diagnostic_packet_bytes(self.payload_byte_len)
            && self.pts_us <= i64::MAX as u64
            && self.flags <= i32::MAX as u32
            && self.source_time_elapsed_ns <= i64::MAX as u64
            && self.source_time_unix_ns <= i64::MAX as u64
    }

    pub fn encode(
        &self,
    ) -> Result<[u8; BROKER_DIAGNOSTIC_VIDEO_PACKET_HEADER_BYTES], BrokerDiagnosticVideoFormatError>
    {
        self.validate_for_encode()?;

        let mut bytes = [0u8; BROKER_DIAGNOSTIC_VIDEO_PACKET_HEADER_BYTES];
        write_i64_be(&mut bytes, 0, self.pts_us as i64);
        write_i32_be(&mut bytes, 8, self.flags as i32);
        write_i32_be(&mut bytes, 12, self.payload_byte_len as i32);
        write_i64_be(&mut bytes, 16, self.source_time_elapsed_ns as i64);
        write_i64_be(&mut bytes, 24, self.source_time_unix_ns as i64);
        Ok(bytes)
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, BrokerDiagnosticVideoFormatError> {
        if bytes.len() < BROKER_DIAGNOSTIC_VIDEO_PACKET_HEADER_BYTES {
            return Err(BrokerDiagnosticVideoFormatError::ShortHeader {
                expected: BROKER_DIAGNOSTIC_VIDEO_PACKET_HEADER_BYTES,
                actual: bytes.len(),
            });
        }

        let pts_us = read_non_negative_i64(bytes, 0, "pts_us")?;
        let flags = read_i32_be(bytes, 8) as i64;
        if flags < 0 {
            return Err(BrokerDiagnosticVideoFormatError::InvalidFlags(flags as u64));
        }

        let payload_byte_len = read_i32_be(bytes, 12) as i64;
        if !(1..=BROKER_DIAGNOSTIC_VIDEO_MAX_PACKET_BYTES as i64).contains(&payload_byte_len) {
            return Err(BrokerDiagnosticVideoFormatError::InvalidPayloadByteLen(
                payload_byte_len,
            ));
        }

        Ok(Self {
            pts_us,
            flags: flags as u32,
            payload_byte_len: payload_byte_len as u32,
            source_time_elapsed_ns: read_non_negative_i64(bytes, 16, "source_time_elapsed_ns")?,
            source_time_unix_ns: read_non_negative_i64(bytes, 24, "source_time_unix_ns")?,
        })
    }

    pub fn parse_for_stream(
        bytes: &[u8],
        stream: &BrokerDiagnosticVideoStreamHeader,
    ) -> Result<Self, BrokerDiagnosticVideoFormatError> {
        let packet = Self::parse(bytes)?;
        packet.validate_for_stream(stream)?;
        Ok(packet)
    }

    pub fn validate_for_stream(
        &self,
        stream: &BrokerDiagnosticVideoStreamHeader,
    ) -> Result<(), BrokerDiagnosticVideoFormatError> {
        if let Some(declared) = stream.declared_packet_bytes {
            if declared != self.payload_byte_len {
                return Err(
                    BrokerDiagnosticVideoFormatError::DeclaredPacketBytesMismatch {
                        declared,
                        actual: self.payload_byte_len,
                    },
                );
            }
        }
        Ok(())
    }

    pub fn to_h264_packet_descriptor(
        &self,
        session_id: impl Into<String>,
        stream_id: impl Into<String>,
        sequence_number: u64,
    ) -> BrokerPacketDescriptor {
        BrokerPacketDescriptor::new(
            session_id,
            stream_id,
            sequence_number,
            BrokerPayloadKind::H264,
            self.payload_byte_len,
        )
        .with_key_frame(self.is_key_frame())
    }

    fn validate_for_encode(&self) -> Result<(), BrokerDiagnosticVideoFormatError> {
        if !valid_diagnostic_packet_bytes(self.payload_byte_len) {
            return Err(BrokerDiagnosticVideoFormatError::InvalidPayloadByteLen(
                self.payload_byte_len as i64,
            ));
        }
        if self.flags > i32::MAX as u32 {
            return Err(BrokerDiagnosticVideoFormatError::InvalidFlags(
                self.flags as u64,
            ));
        }
        validate_i64_wire_timestamp("pts_us", self.pts_us)?;
        validate_i64_wire_timestamp("source_time_elapsed_ns", self.source_time_elapsed_ns)?;
        validate_i64_wire_timestamp("source_time_unix_ns", self.source_time_unix_ns)?;
        Ok(())
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

/// Current reading from the broker-owned clock service.
///
/// The canonical domain should be monotonic for storage and ordering. Unix time
/// is included for export labels and may move if wall-clock sync changes.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerClockSnapshot {
    pub schema: String,
    pub clock_id: String,
    pub clock_epoch_id: String,
    pub sequence_number: u64,
    pub canonical_domain: BrokerTimestampDomain,
    pub android_elapsed_realtime_ns: u64,
    pub android_realtime_unix_ns: Option<u64>,
    pub read_uncertainty_ns: u64,
    pub wall_clock_adjustment_counter: u64,
    pub health: BrokerClockHealthState,
}

impl BrokerClockSnapshot {
    pub fn new(
        clock_id: impl Into<String>,
        clock_epoch_id: impl Into<String>,
        sequence_number: u64,
        android_elapsed_realtime_ns: u64,
    ) -> Self {
        Self {
            schema: BROKER_CLOCK_SNAPSHOT_SCHEMA.to_string(),
            clock_id: clock_id.into(),
            clock_epoch_id: clock_epoch_id.into(),
            sequence_number,
            canonical_domain: BrokerTimestampDomain::ElapsedRealtime,
            android_elapsed_realtime_ns,
            android_realtime_unix_ns: None,
            read_uncertainty_ns: 0,
            wall_clock_adjustment_counter: 0,
            health: BrokerClockHealthState::Healthy,
        }
    }

    pub const fn with_unix_ns(mut self, unix_ns: u64) -> Self {
        self.android_realtime_unix_ns = Some(unix_ns);
        self
    }

    pub const fn with_read_uncertainty_ns(mut self, read_uncertainty_ns: u64) -> Self {
        self.read_uncertainty_ns = read_uncertainty_ns;
        self
    }

    pub const fn with_wall_clock_adjustment_counter(mut self, counter: u64) -> Self {
        self.wall_clock_adjustment_counter = counter;
        self
    }

    pub const fn with_health(mut self, health: BrokerClockHealthState) -> Self {
        self.health = health;
        self
    }

    pub fn as_stamp(&self) -> BrokerClockStamp {
        let mut stamp = BrokerClockStamp::new(
            self.clock_id.clone(),
            self.clock_epoch_id.clone(),
            self.android_elapsed_realtime_ns,
            self.sequence_number,
        )
        .with_uncertainty_ns(self.read_uncertainty_ns);
        if let Some(unix_ns) = self.android_realtime_unix_ns {
            stamp = stamp.with_event_unix_ns(unix_ns);
        }
        stamp
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_CLOCK_SNAPSHOT_SCHEMA
            && non_empty_string(&self.clock_id)
            && non_empty_string(&self.clock_epoch_id)
            && matches!(
                self.canonical_domain,
                BrokerTimestampDomain::ElapsedRealtime
            )
    }
}

/// Durable stamp attached to a broker record, stream sample, or marker.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerClockStamp {
    pub schema: String,
    pub clock_id: String,
    pub clock_epoch_id: String,
    pub canonical_domain: BrokerTimestampDomain,
    pub event_elapsed_realtime_ns: u64,
    pub event_unix_ns: Option<u64>,
    pub source_domain: Option<BrokerTimestampDomain>,
    pub source_time_ns: Option<u64>,
    pub correlation_id: Option<String>,
    pub uncertainty_ns: u64,
    pub sequence_number: u64,
}

impl BrokerClockStamp {
    pub fn new(
        clock_id: impl Into<String>,
        clock_epoch_id: impl Into<String>,
        event_elapsed_realtime_ns: u64,
        sequence_number: u64,
    ) -> Self {
        Self {
            schema: BROKER_CLOCK_STAMP_SCHEMA.to_string(),
            clock_id: clock_id.into(),
            clock_epoch_id: clock_epoch_id.into(),
            canonical_domain: BrokerTimestampDomain::ElapsedRealtime,
            event_elapsed_realtime_ns,
            event_unix_ns: None,
            source_domain: None,
            source_time_ns: None,
            correlation_id: None,
            uncertainty_ns: 0,
            sequence_number,
        }
    }

    pub const fn with_event_unix_ns(mut self, event_unix_ns: u64) -> Self {
        self.event_unix_ns = Some(event_unix_ns);
        self
    }

    pub const fn with_source_time(
        mut self,
        source_domain: BrokerTimestampDomain,
        source_time_ns: u64,
    ) -> Self {
        self.source_domain = Some(source_domain);
        self.source_time_ns = Some(source_time_ns);
        self
    }

    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    pub const fn with_uncertainty_ns(mut self, uncertainty_ns: u64) -> Self {
        self.uncertainty_ns = uncertainty_ns;
        self
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_CLOCK_STAMP_SCHEMA
            && non_empty_string(&self.clock_id)
            && non_empty_string(&self.clock_epoch_id)
            && matches!(
                self.canonical_domain,
                BrokerTimestampDomain::ElapsedRealtime
            )
            && self
                .source_domain
                .zip(self.source_time_ns)
                .map(|(domain, _)| !matches!(domain, BrokerTimestampDomain::Unknown))
                .unwrap_or(self.source_domain.is_none() && self.source_time_ns.is_none())
            && self
                .correlation_id
                .as_deref()
                .map(non_empty_string)
                .unwrap_or(true)
    }
}

/// Affine estimate from one timestamp domain into another.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerClockCorrelation {
    pub schema: String,
    pub correlation_id: String,
    pub source_domain: BrokerTimestampDomain,
    pub target_domain: BrokerTimestampDomain,
    pub sample_count: u32,
    pub window_start_elapsed_ns: u64,
    pub window_end_elapsed_ns: u64,
    pub offset_ns: i64,
    pub drift_ppm: f32,
    pub rms_error_ns: u64,
    pub max_error_ns: u64,
    pub p95_error_ns: u64,
    pub uncertainty_ns: u64,
    pub quality: BrokerClockCorrelationQuality,
    pub last_discontinuity_reason: BrokerClockDiscontinuityReason,
}

impl BrokerClockCorrelation {
    pub fn new(
        correlation_id: impl Into<String>,
        source_domain: BrokerTimestampDomain,
        target_domain: BrokerTimestampDomain,
        window_start_elapsed_ns: u64,
        window_end_elapsed_ns: u64,
    ) -> Self {
        Self {
            schema: BROKER_CLOCK_CORRELATION_SCHEMA.to_string(),
            correlation_id: correlation_id.into(),
            source_domain,
            target_domain,
            sample_count: 0,
            window_start_elapsed_ns,
            window_end_elapsed_ns,
            offset_ns: 0,
            drift_ppm: 0.0,
            rms_error_ns: 0,
            max_error_ns: 0,
            p95_error_ns: 0,
            uncertainty_ns: 0,
            quality: BrokerClockCorrelationQuality::Unavailable,
            last_discontinuity_reason: BrokerClockDiscontinuityReason::None,
        }
    }

    pub const fn with_sample_count(mut self, sample_count: u32) -> Self {
        self.sample_count = sample_count;
        self
    }

    pub const fn with_offset_ns(mut self, offset_ns: i64) -> Self {
        self.offset_ns = offset_ns;
        self
    }

    pub const fn with_drift_ppm(mut self, drift_ppm: f32) -> Self {
        self.drift_ppm = drift_ppm;
        self
    }

    pub const fn with_error_stats(
        mut self,
        rms_error_ns: u64,
        max_error_ns: u64,
        p95_error_ns: u64,
        uncertainty_ns: u64,
    ) -> Self {
        self.rms_error_ns = rms_error_ns;
        self.max_error_ns = max_error_ns;
        self.p95_error_ns = p95_error_ns;
        self.uncertainty_ns = uncertainty_ns;
        self
    }

    pub const fn with_quality(mut self, quality: BrokerClockCorrelationQuality) -> Self {
        self.quality = quality;
        self
    }

    pub const fn with_discontinuity(mut self, reason: BrokerClockDiscontinuityReason) -> Self {
        self.last_discontinuity_reason = reason;
        self
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_CLOCK_CORRELATION_SCHEMA
            && non_empty_string(&self.correlation_id)
            && !matches!(self.source_domain, BrokerTimestampDomain::Unknown)
            && !matches!(self.target_domain, BrokerTimestampDomain::Unknown)
            && self.source_domain != self.target_domain
            && self.window_end_elapsed_ns >= self.window_start_elapsed_ns
            && self.drift_ppm.is_finite()
            && self.max_error_ns >= self.rms_error_ns
            && self.max_error_ns >= self.p95_error_ns
            && (!matches!(self.quality, BrokerClockCorrelationQuality::Unavailable)
                || self.sample_count == 0)
    }
}

/// Health summary for the broker-owned clock service.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerClockHealth {
    pub schema: String,
    pub clock_id: String,
    pub clock_epoch_id: String,
    pub health: BrokerClockHealthState,
    pub wall_clock_adjustment_counter: u64,
    pub last_snapshot: BrokerClockSnapshot,
    pub active_correlations: Vec<BrokerClockCorrelation>,
}

impl BrokerClockHealth {
    pub fn new(last_snapshot: BrokerClockSnapshot) -> Self {
        Self {
            schema: BROKER_CLOCK_HEALTH_SCHEMA.to_string(),
            clock_id: last_snapshot.clock_id.clone(),
            clock_epoch_id: last_snapshot.clock_epoch_id.clone(),
            health: last_snapshot.health,
            wall_clock_adjustment_counter: last_snapshot.wall_clock_adjustment_counter,
            last_snapshot,
            active_correlations: Vec::new(),
        }
    }

    pub fn with_correlation(mut self, correlation: BrokerClockCorrelation) -> Self {
        self.active_correlations.push(correlation);
        self
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_CLOCK_HEALTH_SCHEMA
            && non_empty_string(&self.clock_id)
            && non_empty_string(&self.clock_epoch_id)
            && self.last_snapshot.is_valid()
            && self
                .active_correlations
                .iter()
                .all(BrokerClockCorrelation::is_valid)
    }
}

/// Four-timestamp probe for estimating host-target clock offset.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerClockSyncProbe {
    pub schema: String,
    pub probe_id: String,
    pub sequence_number: u64,
    pub host_send_unix_ns: u64,
    pub target_receive_elapsed_ns: u64,
    pub target_receive_unix_ns: u64,
    pub target_send_elapsed_ns: u64,
    pub target_send_unix_ns: u64,
    pub host_receive_unix_ns: Option<u64>,
}

impl BrokerClockSyncProbe {
    pub fn new(
        probe_id: impl Into<String>,
        sequence_number: u64,
        host_send_unix_ns: u64,
        target_receive: BrokerTimingStamp,
        target_send: BrokerTimingStamp,
    ) -> Option<Self> {
        Some(Self {
            schema: BROKER_CLOCK_SYNC_PROBE_SCHEMA.to_string(),
            probe_id: probe_id.into(),
            sequence_number,
            host_send_unix_ns,
            target_receive_elapsed_ns: target_receive.elapsed_ns,
            target_receive_unix_ns: target_receive.unix_ns?,
            target_send_elapsed_ns: target_send.elapsed_ns,
            target_send_unix_ns: target_send.unix_ns?,
            host_receive_unix_ns: None,
        })
    }

    pub const fn with_host_receive_unix_ns(mut self, host_receive_unix_ns: u64) -> Self {
        self.host_receive_unix_ns = Some(host_receive_unix_ns);
        self
    }

    pub fn round_trip_ns(&self) -> Option<u64> {
        self.host_receive_unix_ns?
            .checked_sub(self.host_send_unix_ns)?
            .checked_sub(
                self.target_send_unix_ns
                    .checked_sub(self.target_receive_unix_ns)?,
            )
    }

    pub fn target_minus_host_offset_ns(&self) -> Option<i128> {
        let host_receive = self.host_receive_unix_ns?;
        let outbound = self.target_receive_unix_ns as i128 - self.host_send_unix_ns as i128;
        let inbound = self.target_send_unix_ns as i128 - host_receive as i128;
        Some((outbound + inbound) / 2)
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_CLOCK_SYNC_PROBE_SCHEMA
            && non_empty_string(&self.probe_id)
            && self.target_send_elapsed_ns >= self.target_receive_elapsed_ns
            && self.target_send_unix_ns >= self.target_receive_unix_ns
            && self
                .host_receive_unix_ns
                .map(|receive| receive >= self.host_send_unix_ns)
                .unwrap_or(true)
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

/// Structured rejection details carried by a broker command acknowledgement.
///
/// Existing brokers may send only `code` and `message`; the optional fields
/// let newer brokers point clients at the missing lease, capability, role, or
/// registry revision without making command acknowledgements framework-aware.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerCommandRejection {
    pub schema: Option<String>,
    pub code: String,
    pub message: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub retryable: bool,
    pub required_capability: Option<String>,
    pub required_role: Option<String>,
    pub required_lease_scope: Option<BrokerControlScope>,
    pub current_revision: Option<u64>,
    pub lease_id: Option<String>,
}

impl BrokerCommandRejection {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            schema: Some(BROKER_COMMAND_REJECTION_SCHEMA.to_string()),
            code: code.into(),
            message: message.into(),
            retryable: false,
            required_capability: None,
            required_role: None,
            required_lease_scope: None,
            current_revision: None,
            lease_id: None,
        }
    }

    pub const fn with_retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }

    pub fn with_required_capability(mut self, capability: impl Into<String>) -> Self {
        self.required_capability = Some(capability.into());
        self
    }

    pub fn with_required_role(mut self, role: impl Into<String>) -> Self {
        self.required_role = Some(role.into());
        self
    }

    pub fn with_required_lease_scope(mut self, scope: BrokerControlScope) -> Self {
        self.required_lease_scope = Some(scope);
        self
    }

    pub const fn with_current_revision(mut self, current_revision: u64) -> Self {
        self.current_revision = Some(current_revision);
        self
    }

    pub fn with_lease_id(mut self, lease_id: impl Into<String>) -> Self {
        self.lease_id = Some(lease_id.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        self.schema
            .as_deref()
            .map(|schema| schema == BROKER_COMMAND_REJECTION_SCHEMA)
            .unwrap_or(true)
            && !self.code.trim().is_empty()
            && !self.message.trim().is_empty()
            && self
                .required_capability
                .as_deref()
                .map(|capability| !capability.trim().is_empty())
                .unwrap_or(true)
            && self
                .required_role
                .as_deref()
                .map(|role| !role.trim().is_empty())
                .unwrap_or(true)
            && self
                .required_lease_scope
                .as_ref()
                .map(BrokerControlScope::is_valid)
                .unwrap_or(true)
            && self
                .lease_id
                .as_deref()
                .map(|lease_id| !lease_id.trim().is_empty())
                .unwrap_or(true)
    }
}

impl From<&str> for BrokerCommandRejection {
    fn from(message: &str) -> Self {
        Self::new("rejected", message)
    }
}

impl From<String> for BrokerCommandRejection {
    fn from(message: String) -> Self {
        Self::new("rejected", message)
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
    pub error: Option<BrokerCommandRejection>,
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

    pub fn rejected(
        request_id: impl Into<String>,
        error: impl Into<BrokerCommandRejection>,
    ) -> Self {
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
            && ((self.accepted && self.error.is_none())
                || (!self.accepted
                    && self.result.is_none()
                    && self
                        .error
                        .as_ref()
                        .map(BrokerCommandRejection::is_valid)
                        .unwrap_or(false)))
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

fn diagnostic_video_codec_wire_id(codec: BrokerCodecId) -> Option<u32> {
    match codec {
        BrokerCodecId::H264 => Some(BROKER_DIAGNOSTIC_VIDEO_CODEC_H264),
        _ => None,
    }
}

fn diagnostic_video_codec_from_wire_id(
    codec_id: i64,
) -> Result<BrokerCodecId, BrokerDiagnosticVideoFormatError> {
    match codec_id {
        value if value == BROKER_DIAGNOSTIC_VIDEO_CODEC_H264 as i64 => Ok(BrokerCodecId::H264),
        _ => Err(BrokerDiagnosticVideoFormatError::UnsupportedCodecId(
            codec_id,
        )),
    }
}

fn valid_diagnostic_packet_bytes(value: u32) -> bool {
    (1..=BROKER_DIAGNOSTIC_VIDEO_MAX_PACKET_BYTES).contains(&value)
}

fn read_i32_be(bytes: &[u8], offset: usize) -> i32 {
    i32::from_be_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn read_i64_be(bytes: &[u8], offset: usize) -> i64 {
    i64::from_be_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ])
}

fn read_non_negative_i64(
    bytes: &[u8],
    offset: usize,
    field: &'static str,
) -> Result<u64, BrokerDiagnosticVideoFormatError> {
    let value = read_i64_be(bytes, offset);
    if value < 0 {
        return Err(BrokerDiagnosticVideoFormatError::InvalidTimestamp {
            field,
            value: value as i128,
        });
    }
    Ok(value as u64)
}

fn write_i32_be(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn write_i64_be(bytes: &mut [u8], offset: usize, value: i64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
}

fn validate_i64_wire_timestamp(
    field: &'static str,
    value: u64,
) -> Result<(), BrokerDiagnosticVideoFormatError> {
    if value > i64::MAX as u64 {
        return Err(BrokerDiagnosticVideoFormatError::InvalidTimestamp {
            field,
            value: value as i128,
        });
    }
    Ok(())
}

fn valid_datagram_size(value: u32) -> bool {
    value > 0 && value <= MAX_UDP_DATAGRAM_BYTES
}

fn valid_zeromq_message_size(value: u32) -> bool {
    value > 0 && value <= MAX_ZEROMQ_BRIDGE_MESSAGE_BYTES
}

fn valid_unit_interval(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn non_empty_string(value: &str) -> bool {
    !value.trim().is_empty()
}

fn ordered_optional_pair(earlier: Option<u64>, later: Option<u64>) -> bool {
    later
        .zip(earlier)
        .map(|(later, earlier)| later >= earlier)
        .unwrap_or(true)
}

fn is_loopback_host(host: &str) -> bool {
    let host = host.trim().to_ascii_lowercase();
    host == "localhost" || host == "::1" || host == "[::1]" || host.starts_with("127.")
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
    fn transport_session_offer_validates_streams_and_security() {
        let stream = BrokerTransportStreamDescriptor::new(
            "camera.left.h264",
            BrokerStreamKind::Media,
            BrokerPayloadKind::H264,
            "video/h264",
        )
        .with_codec(BrokerCodecId::H264)
        .with_reliability(BrokerReliabilityClass::LossTolerant)
        .with_ordered(false)
        .with_nominal_rate_hz(60.0)
        .with_target_latency_ms(25.0)
        .with_max_payload_bytes(64_000);
        let offer = BrokerTransportSessionOffer::new("session-001", "client-1")
            .with_transport(BrokerTransportKind::AdbForwardedTcp)
            .with_stream(stream)
            .with_security(
                BrokerTransportSecurityPolicy::loopback_only()
                    .with_capability_scope("camera_provider.start_app_camera_h264_stream"),
            )
            .with_target_latency_ms(35.0);

        assert!(offer.is_valid());
        assert_eq!(offer.schema, BROKER_TRANSPORT_SESSION_OFFER_SCHEMA);
    }

    #[test]
    fn zeromq_endpoint_validates_loopback_tcp() {
        let endpoint = BrokerTransportEndpoint::zeromq_tcp("127.0.0.1", 5555);
        let invalid = BrokerTransportEndpoint::zeromq_tcp("127.0.0.1", 0);
        let lan = BrokerTransportEndpoint::zeromq_tcp("192.168.0.20", 5555);
        let datagram_sized = BrokerTransportEndpoint {
            max_datagram_bytes: Some(1200),
            ..endpoint.clone()
        };

        assert!(endpoint.is_valid());
        assert!(endpoint.is_loopback());
        assert!(!invalid.is_valid());
        assert!(lan.is_valid());
        assert!(!lan.is_loopback());
        assert!(!datagram_sized.is_valid());
    }

    #[test]
    fn zeromq_bridge_manifest_validates_pattern_and_payload() {
        let manifest = BrokerZeroMqBridgeManifest::new(
            "lab-zero-mq-json",
            BrokerTransportEndpoint::zeromq_tcp("127.0.0.1", 5555),
            BrokerZeroMqPattern::PubSub,
            BrokerStreamDirection::ProducerToConsumer,
            BrokerPayloadKind::Json,
            BROKER_LATENCY_SAMPLE_SCHEMA,
        )
        .with_bind_mode(BrokerZeroMqBindMode::Bind)
        .with_stream_id(STREAM_LATENCY_SAMPLE)
        .with_topic_prefix("rustyxr.latency")
        .with_max_message_bytes(4096)
        .with_high_water_mark(1000)
        .with_consent_data_category("clock")
        .with_note("loopback validation");

        assert!(manifest.is_valid());
        assert!(manifest.is_pub_sub());
        assert_eq!(manifest.schema, BROKER_ZEROMQ_BRIDGE_MANIFEST_SCHEMA);
        assert_eq!(manifest.endpoint.transport, BrokerTransportKind::ZeroMq);
    }

    #[test]
    fn zeromq_bridge_manifest_rejects_invalid_descriptors() {
        let valid_endpoint = BrokerTransportEndpoint::zeromq_tcp("127.0.0.1", 5555);
        let wrong_transport = BrokerZeroMqBridgeManifest::new(
            "lab-zero-mq-json",
            BrokerTransportEndpoint::udp("127.0.0.1", 5555, 1200),
            BrokerZeroMqPattern::PubSub,
            BrokerStreamDirection::ProducerToConsumer,
            BrokerPayloadKind::Json,
            BROKER_LATENCY_SAMPLE_SCHEMA,
        );
        let empty_schema = BrokerZeroMqBridgeManifest::new(
            "lab-zero-mq-json",
            valid_endpoint.clone(),
            BrokerZeroMqPattern::PubSub,
            BrokerStreamDirection::ProducerToConsumer,
            BrokerPayloadKind::Json,
            "",
        );
        let empty_topic = BrokerZeroMqBridgeManifest::new(
            "lab-zero-mq-json",
            valid_endpoint.clone(),
            BrokerZeroMqPattern::PubSub,
            BrokerStreamDirection::ProducerToConsumer,
            BrokerPayloadKind::Json,
            BROKER_LATENCY_SAMPLE_SCHEMA,
        )
        .with_topic_prefix("");
        let empty_category = BrokerZeroMqBridgeManifest::new(
            "lab-zero-mq-json",
            valid_endpoint.clone(),
            BrokerZeroMqPattern::PubSub,
            BrokerStreamDirection::ProducerToConsumer,
            BrokerPayloadKind::Json,
            BROKER_LATENCY_SAMPLE_SCHEMA,
        )
        .with_consent_data_category("");
        let empty_note = BrokerZeroMqBridgeManifest::new(
            "lab-zero-mq-json",
            valid_endpoint,
            BrokerZeroMqPattern::PubSub,
            BrokerStreamDirection::ProducerToConsumer,
            BrokerPayloadKind::Json,
            BROKER_LATENCY_SAMPLE_SCHEMA,
        )
        .with_note("");

        assert!(!wrong_transport.is_valid());
        assert!(!empty_schema.is_valid());
        assert!(!empty_topic.is_valid());
        assert!(!empty_category.is_valid());
        assert!(!empty_note.is_valid());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn zeromq_bridge_manifest_serializes_with_serde() {
        let manifest = BrokerZeroMqBridgeManifest::new(
            "lab-zero-mq-json",
            BrokerTransportEndpoint::zeromq_tcp("127.0.0.1", 5555),
            BrokerZeroMqPattern::RequestReply,
            BrokerStreamDirection::Bidirectional,
            BrokerPayloadKind::Json,
            BROKER_COMMAND_SCHEMA,
        )
        .with_bind_mode(BrokerZeroMqBindMode::Connect)
        .with_stream_id("control:zeromq");

        let json = serde_json::to_string(&manifest).expect("manifest should serialize");
        let decoded: BrokerZeroMqBridgeManifest =
            serde_json::from_str(&json).expect("manifest should deserialize");

        assert_eq!(decoded, manifest);
        assert!(decoded.is_valid());
    }

    #[test]
    fn security_policy_gates_non_loopback_endpoints() {
        let loopback = BrokerTransportEndpoint {
            transport: BrokerTransportKind::Tcp,
            host: Some("127.0.0.1".to_string()),
            port: Some(8791),
            path: None,
            channel_id: None,
            max_datagram_bytes: None,
            auth_required: false,
        };
        let lan = BrokerTransportEndpoint {
            host: Some("192.168.0.20".to_string()),
            ..loopback.clone()
        };
        let loopback_only = BrokerTransportSecurityPolicy::loopback_only();
        let paired = BrokerTransportSecurityPolicy::pairing_token(10_000);

        assert!(loopback.is_loopback());
        assert!(!lan.is_loopback());
        assert!(loopback_only.allows_endpoint(&loopback));
        assert!(!loopback_only.allows_endpoint(&lan));
        assert!(paired.allows_endpoint(&lan));
        assert!(paired.is_valid());
    }

    #[test]
    fn transport_answer_requires_reason_when_rejected() {
        let stream = BrokerTransportStreamDescriptor::new(
            STREAM_SYNTHETIC_WAVE,
            BrokerStreamKind::Synthetic,
            BrokerPayloadKind::Json,
            SYNTHETIC_WAVE_PAYLOAD_SCHEMA,
        );
        let accepted = BrokerTransportSessionAnswer::accepted(
            "session-001",
            BrokerTransportKind::AdbForwardedTcp,
            BrokerTransportSecurityPolicy::loopback_only(),
        )
        .with_stream(stream);
        let rejected = BrokerTransportSessionAnswer::rejected("session-002", "unsupported codec");

        assert!(accepted.is_valid());
        assert!(rejected.is_valid());
        assert!(!BrokerTransportSessionAnswer::rejected("session-003", "").is_valid());
    }

    #[test]
    fn media_sample_timing_reports_stage_latencies() {
        let timing = BrokerMediaSampleTiming::new("session-001", "camera.left.h264", 7)
            .with_source_capture_time_ns(1_000)
            .with_packet_receive_time_ns(1_700)
            .with_decode_done_time_ns(2_200)
            .with_texture_import_time_ns(2_400)
            .with_xr_submit_time_ns(2_900);

        assert!(timing.is_valid());
        assert_eq!(timing.source_to_receive_latency_ns(), Some(700));
        assert_eq!(timing.receive_to_decode_latency_ns(), Some(500));
        assert_eq!(timing.decode_to_submit_latency_ns(), Some(700));
    }

    #[test]
    fn media_sample_timing_rejects_reversed_stage_order() {
        let mut timing = BrokerMediaSampleTiming::new("session-001", "camera.left.h264", 7)
            .with_packet_receive_time_ns(2_000);
        timing.packet_send_time_ns = Some(3_000);

        assert!(!timing.is_valid());
    }

    #[test]
    fn network_quality_sample_validates_unit_ranges() {
        let valid = BrokerNetworkQualitySample::new("session-001", 10_000)
            .with_stream_id("camera.left.h264")
            .with_packet_loss_estimate01(0.05)
            .with_target_latency_ms(35.0)
            .with_actual_latency_ms(42.5);
        let invalid_loss =
            BrokerNetworkQualitySample::new("session-001", 10_000).with_packet_loss_estimate01(1.5);

        assert!(valid.is_valid());
        assert!(!invalid_loss.is_valid());
    }

    #[test]
    fn packet_descriptor_requires_payload_bytes() {
        let packet = BrokerPacketDescriptor::new(
            "session-001",
            "camera.left.h264",
            42,
            BrokerPayloadKind::H264,
            1200,
        )
        .with_key_frame(true);
        let dropped = BrokerPacketDescriptor::new(
            "session-001",
            "camera.left.h264",
            43,
            BrokerPayloadKind::H264,
            1,
        )
        .with_drop_reason(BrokerPacketDropReason::LatePacket);
        let empty = BrokerPacketDescriptor::new(
            "session-001",
            "camera.left.h264",
            44,
            BrokerPayloadKind::H264,
            0,
        );

        assert!(packet.is_valid());
        assert!(dropped.is_valid());
        assert!(!empty.is_valid());
    }

    #[test]
    fn camera_source_capabilities_report_selected_source_and_timing_domain() {
        let mut capabilities =
            BrokerCameraSourceCapabilities::new("camera2:0", BrokerCameraApiPath::AndroidCamera2)
                .with_camera_permission_state(BrokerCameraPermissionState::Granted)
                .with_headset_camera_permission_state(BrokerCameraPermissionState::Granted)
                .with_camera_id("0")
                .with_timestamp_domain(BrokerTimestampDomain::ElapsedRealtime)
                .with_selected_size(
                    BrokerVideoSize::new(720, 480),
                    "closest_preferred_private_size",
                )
                .with_selected_fps_range(BrokerFpsRange::new(30, 30))
                .with_stream_min_frame_duration_ns(33_333_333);
        capabilities
            .supported_private_sizes
            .push(BrokerVideoSize::new(720, 480));
        capabilities
            .supported_yuv_sizes
            .push(BrokerVideoSize::new(640, 480));
        capabilities
            .supported_fps_ranges
            .push(BrokerFpsRange::new(30, 30));

        assert!(capabilities.is_valid());
        assert_eq!(
            capabilities.schema,
            BROKER_CAMERA_SOURCE_CAPABILITIES_SCHEMA
        );
        assert_eq!(
            capabilities.timestamp_domain,
            BrokerTimestampDomain::ElapsedRealtime
        );
    }

    #[test]
    fn camera_source_capabilities_reject_invalid_selected_values() {
        let invalid_size =
            BrokerCameraSourceCapabilities::new("camera2:0", BrokerCameraApiPath::AndroidCamera2)
                .with_selected_size(BrokerVideoSize::new(0, 480), "closest");
        let invalid_fps =
            BrokerCameraSourceCapabilities::new("camera2:0", BrokerCameraApiPath::AndroidCamera2)
                .with_selected_fps_range(BrokerFpsRange::new(60, 30));
        let missing_reason = BrokerCameraSourceCapabilities {
            selected_size: Some(BrokerVideoSize::new(720, 480)),
            selected_reason: Some("".to_string()),
            ..BrokerCameraSourceCapabilities::new("camera2:0", BrokerCameraApiPath::AndroidCamera2)
        };

        assert!(!invalid_size.is_valid());
        assert!(!invalid_fps.is_valid());
        assert!(!missing_reason.is_valid());
        assert!(!BrokerCameraSourceCapabilities::new("", BrokerCameraApiPath::Unknown).is_valid());
    }

    #[test]
    fn h264_stream_invariants_gate_codec_config_and_keyframes() {
        let invariants = BrokerH264StreamInvariants::new(
            "session-001",
            "camera.left.h264",
            "receiver",
            BrokerStreamDirection::ProducerToConsumer,
            BrokerVideoSize::new(720, 480),
        )
        .with_peer_id("quest-a")
        .with_track_id("left")
        .with_eye("left")
        .with_encoder_name("c2.qti.avc.encoder")
        .with_decoder_name("c2.qti.avc.decoder")
        .with_bitrate_bps(1_000_000)
        .with_bitrate_modes("CBR", "CBR")
        .with_h264_start_config(2, true, true, 1)
        .with_sync_frame_request_on_start_succeeded(true);

        assert!(invariants.is_valid());
        assert!(invariants.has_h264_start_config());
        assert!(invariants.has_named_codec_components());
        assert_eq!(invariants.schema, BROKER_H264_STREAM_INVARIANTS_SCHEMA);
    }

    #[test]
    fn h264_stream_invariants_reject_structural_gaps() {
        let invalid = BrokerH264StreamInvariants::new(
            "",
            "camera.left.h264",
            "receiver",
            BrokerStreamDirection::ProducerToConsumer,
            BrokerVideoSize::new(720, 480),
        );
        let missing_config = BrokerH264StreamInvariants::new(
            "session-001",
            "camera.left.h264",
            "receiver",
            BrokerStreamDirection::ProducerToConsumer,
            BrokerVideoSize::new(720, 480),
        )
        .with_h264_start_config(0, false, true, 0);

        assert!(!invalid.is_valid());
        assert!(missing_config.is_valid());
        assert!(!missing_config.has_h264_start_config());
    }

    #[test]
    fn diagnostic_video_v3_headers_round_trip() {
        let stream =
            BrokerDiagnosticVideoStreamHeader::h264(64, 64, 4).with_header_metadata_bytes(1024);
        let stream_bytes = stream.encode().expect("stream header should encode");

        assert_eq!(&stream_bytes[..8], BROKER_DIAGNOSTIC_VIDEO_MAGIC);
        assert_eq!(stream_bytes.len(), BROKER_DIAGNOSTIC_VIDEO_HEADER_BYTES);
        assert_eq!(read_i32_be(&stream_bytes, 8), 3);
        assert_eq!(read_i32_be(&stream_bytes, 28), 1024);

        let parsed_stream = BrokerDiagnosticVideoStreamHeader::parse(&stream_bytes)
            .expect("stream header should parse");
        assert_eq!(parsed_stream, stream);
        assert!(parsed_stream.is_valid());

        let packet = BrokerDiagnosticVideoPacketHeader::new(33_333, 96)
            .with_key_frame(true)
            .with_codec_config(true)
            .with_source_times(1_000_000, 2_000_000);
        let packet_bytes = packet.encode().expect("packet header should encode");
        let parsed_packet =
            BrokerDiagnosticVideoPacketHeader::parse_for_stream(&packet_bytes, &parsed_stream)
                .expect("packet header should parse");

        assert_eq!(parsed_packet, packet);
        assert!(parsed_packet.is_key_frame());
        assert!(parsed_packet.is_codec_config());
        assert_eq!(
            packet_bytes.len(),
            BROKER_DIAGNOSTIC_VIDEO_PACKET_HEADER_BYTES
        );

        let descriptor =
            parsed_packet.to_h264_packet_descriptor("session-001", "camera.left.h264", 7);
        assert!(descriptor.is_valid());
        assert_eq!(descriptor.payload_byte_len, 96);
        assert!(descriptor.key_frame);
    }

    #[test]
    fn diagnostic_video_headers_reject_malformed_inputs() {
        let mut stream_bytes = BrokerDiagnosticVideoStreamHeader::h264(64, 64, 4)
            .encode()
            .expect("stream header should encode");
        stream_bytes[0] = b'X';
        assert!(matches!(
            BrokerDiagnosticVideoStreamHeader::parse(&stream_bytes),
            Err(BrokerDiagnosticVideoFormatError::InvalidMagic { .. })
        ));

        let mut unsupported_version = BrokerDiagnosticVideoStreamHeader::h264(64, 64, 4)
            .encode()
            .expect("stream header should encode");
        write_i32_be(&mut unsupported_version, 8, 4);
        assert_eq!(
            BrokerDiagnosticVideoStreamHeader::parse(&unsupported_version),
            Err(BrokerDiagnosticVideoFormatError::UnsupportedSchemaVersion(
                4
            ))
        );

        let fixed_size_stream = BrokerDiagnosticVideoStreamHeader::h264(64, 64, 4)
            .with_schema_version(2)
            .with_declared_packet_bytes(128);
        let mismatch = BrokerDiagnosticVideoPacketHeader::new(0, 96)
            .encode()
            .expect("packet header should encode");
        assert_eq!(
            BrokerDiagnosticVideoPacketHeader::parse_for_stream(&mismatch, &fixed_size_stream),
            Err(
                BrokerDiagnosticVideoFormatError::DeclaredPacketBytesMismatch {
                    declared: 128,
                    actual: 96,
                },
            )
        );

        assert_eq!(
            BrokerDiagnosticVideoPacketHeader::new(0, 0).encode(),
            Err(BrokerDiagnosticVideoFormatError::InvalidPayloadByteLen(0))
        );
    }

    #[test]
    fn command_and_ack_validate_required_ids() {
        let command = BrokerCommand::new("req-1", "client-1", "subscribe", Some("synthetic:wave"));
        let ack = BrokerCommandAck::accepted("req-1", Some("sub-1"));
        let rejection = BrokerCommandRejection::new("unknown_stream", "unknown stream")
            .with_retryable(false)
            .with_current_revision(8);

        assert!(command.is_valid());
        assert!(ack.is_valid());
        assert!(BrokerCommandAck::<()>::rejected("req-2", rejection).is_valid());
        assert!(BrokerCommandAck::<()>::rejected("req-3", "unknown stream").is_valid());
        assert!(!BrokerCommandAck::<()>::rejected("req-4", "").is_valid());
        assert!(!BrokerCommand::<()>::new("", "client-1", "subscribe", None).is_valid());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn command_ack_deserializes_java_style_rejection_object() {
        let ack: BrokerCommandAck<serde_json::Value> = serde_json::from_str(
            r#"{
                "type": "command_ack",
                "schema": "rusty.xr.broker.command_ack.v1",
                "request_id": "req-1",
                "accepted": false,
                "result": null,
                "error": {
                    "code": "missing_lease",
                    "message": "Command requires an active lease."
                }
            }"#,
        )
        .expect("java-style command rejection should deserialize");

        assert!(ack.is_valid());
        let error = ack.error.expect("rejected ack should include error");
        assert_eq!(error.schema, None);
        assert_eq!(error.code, "missing_lease");
        assert_eq!(error.message, "Command requires an active lease.");
        assert!(!error.retryable);
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

    #[test]
    fn clock_snapshot_converts_to_storage_stamp() {
        let snapshot = BrokerClockSnapshot::new("broker-clock", "epoch-001", 7, 1_000)
            .with_unix_ns(2_000)
            .with_read_uncertainty_ns(50)
            .with_wall_clock_adjustment_counter(1);
        let stamp = snapshot
            .as_stamp()
            .with_source_time(BrokerTimestampDomain::OpenXrPredictedDisplay, 900);

        assert!(snapshot.is_valid());
        assert!(stamp.is_valid());
        assert_eq!(stamp.schema, BROKER_CLOCK_STAMP_SCHEMA);
        assert_eq!(stamp.event_elapsed_realtime_ns, 1_000);
        assert_eq!(stamp.event_unix_ns, Some(2_000));
        assert_eq!(stamp.uncertainty_ns, 50);
        assert_eq!(
            stamp.source_domain,
            Some(BrokerTimestampDomain::OpenXrPredictedDisplay)
        );
    }

    #[test]
    fn clock_correlation_requires_distinct_known_domains() {
        let correlation = BrokerClockCorrelation::new(
            "openxr-window-001",
            BrokerTimestampDomain::OpenXrPredictedDisplay,
            BrokerTimestampDomain::ElapsedRealtime,
            1_000,
            2_000,
        )
        .with_sample_count(120)
        .with_offset_ns(-200)
        .with_drift_ppm(0.25)
        .with_error_stats(50, 200, 150, 250)
        .with_quality(BrokerClockCorrelationQuality::High);
        let invalid = BrokerClockCorrelation::new(
            "bad",
            BrokerTimestampDomain::ElapsedRealtime,
            BrokerTimestampDomain::ElapsedRealtime,
            2_000,
            1_000,
        );

        assert!(correlation.is_valid());
        assert!(!invalid.is_valid());
    }

    #[test]
    fn clock_health_wraps_snapshot_and_correlations() {
        let snapshot = BrokerClockSnapshot::new("broker-clock", "epoch-001", 1, 1_000);
        let correlation = BrokerClockCorrelation::new(
            "host-window-001",
            BrokerTimestampDomain::Unix,
            BrokerTimestampDomain::ElapsedRealtime,
            1_000,
            2_000,
        )
        .with_sample_count(16)
        .with_quality(BrokerClockCorrelationQuality::Medium);
        let health = BrokerClockHealth::new(snapshot).with_correlation(correlation);

        assert!(health.is_valid());
        assert_eq!(health.schema, BROKER_CLOCK_HEALTH_SCHEMA);
        assert_eq!(health.active_correlations.len(), 1);
    }

    #[test]
    fn clock_sync_probe_reports_ntp_style_offset() {
        let target_receive = BrokerTimingStamp::elapsed(1_000).with_unix_ns(10_050);
        let target_send = BrokerTimingStamp::elapsed(1_100).with_unix_ns(10_150);
        let probe = BrokerClockSyncProbe::new("probe-001", 1, 10_000, target_receive, target_send)
            .expect("target stamps include unix time")
            .with_host_receive_unix_ns(10_300);

        assert!(probe.is_valid());
        assert_eq!(probe.round_trip_ns(), Some(200));
        assert_eq!(probe.target_minus_host_offset_ns(), Some(-50));
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
    fn transport_offer_round_trips_with_serde() {
        let offer = BrokerTransportSessionOffer::new("session-001", "client-1")
            .with_transport(BrokerTransportKind::AdbForwardedTcp)
            .with_stream(BrokerTransportStreamDescriptor::new(
                STREAM_SYNTHETIC_WAVE,
                BrokerStreamKind::Synthetic,
                BrokerPayloadKind::Json,
                SYNTHETIC_WAVE_PAYLOAD_SCHEMA,
            ));

        let encoded = serde_json::to_string(&offer).expect("offer should serialize");
        let decoded: BrokerTransportSessionOffer =
            serde_json::from_str(&encoded).expect("offer should deserialize");

        assert_eq!(decoded, offer);
        assert!(decoded.is_valid());
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
