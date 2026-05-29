//! Interactive media route contracts.
//!
//! These data-only contracts describe low-latency media routes without making
//! broker core depend on a codec SDK, media runtime, renderer, or protocol
//! adapter. They keep control, high-rate payload transport, render/adoption,
//! and feedback state separate so brokers and UI clients can reason about a
//! route before any runtime backend is attached.

use crate::{
    BrokerCodecId, BrokerCommandAuthorityRequirement, BrokerControlScope, BrokerDataSensitivity,
    BrokerPayloadKind, BrokerTimestampDomain, BrokerTransportKind,
};

/// Versioned JSON schema id for interactive media route manifests.
pub const BROKER_INTERACTIVE_MEDIA_ROUTE_MANIFEST_SCHEMA: &str =
    "rusty.xr.broker.interactive_media_route_manifest.v1";

/// Versioned JSON schema id for interactive media route runtime-state snapshots.
pub const BROKER_INTERACTIVE_MEDIA_ROUTE_RUNTIME_STATE_SCHEMA: &str =
    "rusty.xr.broker.interactive_media_route_runtime_state.v1";

/// Versioned JSON schema id for interactive media feedback samples.
pub const BROKER_MEDIA_FEEDBACK_SAMPLE_SCHEMA: &str = "rusty.xr.broker.media_feedback_sample.v1";

/// Versioned JSON schema id for interactive media pipeline scorecards.
pub const BROKER_MEDIA_PIPELINE_SCORECARD_SCHEMA: &str =
    "rusty.xr.broker.media_pipeline_scorecard.v1";

/// One of the four planes in an interactive media route.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerInteractiveMediaPlane {
    Control,
    MediaData,
    RenderAdoption,
    Feedback,
}

impl BrokerInteractiveMediaPlane {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Control => "control",
            Self::MediaData => "media_data",
            Self::RenderAdoption => "render_adoption",
            Self::Feedback => "feedback",
        }
    }
}

/// Broad route direction relative to the active XR app.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerInteractiveMediaRouteDirection {
    IntoXrApp,
    OutOfXrApp,
    Bidirectional,
    Loopback,
    MetadataOnly,
}

/// Runtime owner for one route plane or resource handoff.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerMediaResourceOwner {
    Broker,
    ActiveXrApp,
    ExternalSidecar,
    Runtime,
    Operator,
    Unknown,
}

/// Backend class used for fallback and scorecard reporting.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerMediaBackendTier {
    Reference,
    Optimized,
    Hardware,
    External,
}

/// Consent or permission gate that must be satisfied before a route can run.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerMediaConsentRequirement {
    NotRequired,
    RuntimePermission,
    MediaProjectionConsent,
    OperatorApproval,
    ExternalPairing,
    Unknown,
}

/// Queue/adoption policy for decoded frames, textures, or submitted XR frames.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerMediaQueuePolicy {
    DropOldest,
    DropNewest,
    ReuseLatest,
    BlockProducer,
    BoundedBurst,
    Unknown,
}

/// Explicit prediction policy label for apparent-latency experiments.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerMediaPredictionPolicy {
    None,
    TimestampExtrapolation,
    ReprojectionHint,
    AppOwned,
    Unknown,
}

/// Lifecycle state for the route as a whole or for a single route plane.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerInteractiveMediaRouteLifecycleState {
    Planned,
    Negotiating,
    Starting,
    Streaming,
    Draining,
    Stopped,
    Degraded,
    Failed,
}

/// Per-frame lifecycle checkpoint used by feedback and diagnostics.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerMediaFrameLifecycleState {
    Unknown,
    Captured,
    EncodedPacketQueued,
    PacketSent,
    PacketReceived,
    DecoderInputQueued,
    DecodedFrameReady,
    TextureImported,
    XrSubmitted,
    PresentedEstimate,
    Dropped,
}

/// Scorecard verdict for one measured media-route window.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerMediaScoreVerdict {
    Pass,
    Warning,
    Fail,
    Unknown,
}

/// Plane descriptor for control, data, render/adoption, or feedback traffic.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerInteractiveMediaPlaneDescriptor {
    pub plane: BrokerInteractiveMediaPlane,
    pub owner: BrokerMediaResourceOwner,
    pub transport: BrokerTransportKind,
    pub stream_id: Option<String>,
    pub payload_kind: BrokerPayloadKind,
    pub payload_schema: String,
    pub high_rate: bool,
    pub json_control_path: bool,
    pub data_sensitivity: BrokerDataSensitivity,
    pub max_payload_bytes: Option<u32>,
    pub notes: Vec<String>,
}

impl BrokerInteractiveMediaPlaneDescriptor {
    pub fn new(
        plane: BrokerInteractiveMediaPlane,
        owner: BrokerMediaResourceOwner,
        transport: BrokerTransportKind,
        payload_kind: BrokerPayloadKind,
        payload_schema: impl Into<String>,
        data_sensitivity: BrokerDataSensitivity,
    ) -> Self {
        Self {
            plane,
            owner,
            transport,
            stream_id: None,
            payload_kind,
            payload_schema: payload_schema.into(),
            high_rate: false,
            json_control_path: false,
            data_sensitivity,
            max_payload_bytes: None,
            notes: Vec::new(),
        }
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub const fn with_high_rate(mut self, high_rate: bool) -> Self {
        self.high_rate = high_rate;
        self
    }

    pub const fn with_json_control_path(mut self, json_control_path: bool) -> Self {
        self.json_control_path = json_control_path;
        self
    }

    pub const fn with_max_payload_bytes(mut self, max_payload_bytes: u32) -> Self {
        self.max_payload_bytes = Some(max_payload_bytes);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub const fn high_rate_uses_json_control_path(&self) -> bool {
        self.high_rate && self.json_control_path
    }

    pub fn is_valid(&self) -> bool {
        non_empty(&self.payload_schema)
            && self.stream_id.as_deref().map(non_empty).unwrap_or(true)
            && self
                .max_payload_bytes
                .map(|bytes| bytes > 0)
                .unwrap_or(true)
            && !self.high_rate_uses_json_control_path()
            && self.notes.iter().all(|note| non_empty(note))
    }
}

/// Latency budget for a media route and its component planes.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerMediaLatencyBudget {
    pub target_end_to_end_ms: f32,
    pub max_capture_to_encode_ms: Option<f32>,
    pub max_network_transit_ms: Option<f32>,
    pub max_receive_to_decode_ms: Option<f32>,
    pub max_decode_to_texture_ms: Option<f32>,
    pub max_texture_to_submit_ms: Option<f32>,
    pub max_jitter_buffer_ms: Option<f32>,
}

impl BrokerMediaLatencyBudget {
    pub const fn new(target_end_to_end_ms: f32) -> Self {
        Self {
            target_end_to_end_ms,
            max_capture_to_encode_ms: None,
            max_network_transit_ms: None,
            max_receive_to_decode_ms: None,
            max_decode_to_texture_ms: None,
            max_texture_to_submit_ms: None,
            max_jitter_buffer_ms: None,
        }
    }

    pub const fn with_capture_to_encode_ms(mut self, value: f32) -> Self {
        self.max_capture_to_encode_ms = Some(value);
        self
    }

    pub const fn with_network_transit_ms(mut self, value: f32) -> Self {
        self.max_network_transit_ms = Some(value);
        self
    }

    pub const fn with_receive_to_decode_ms(mut self, value: f32) -> Self {
        self.max_receive_to_decode_ms = Some(value);
        self
    }

    pub const fn with_decode_to_texture_ms(mut self, value: f32) -> Self {
        self.max_decode_to_texture_ms = Some(value);
        self
    }

    pub const fn with_texture_to_submit_ms(mut self, value: f32) -> Self {
        self.max_texture_to_submit_ms = Some(value);
        self
    }

    pub const fn with_jitter_buffer_ms(mut self, value: f32) -> Self {
        self.max_jitter_buffer_ms = Some(value);
        self
    }

    pub fn component_budget_ms(&self) -> f32 {
        [
            self.max_capture_to_encode_ms,
            self.max_network_transit_ms,
            self.max_receive_to_decode_ms,
            self.max_decode_to_texture_ms,
            self.max_texture_to_submit_ms,
            self.max_jitter_buffer_ms,
        ]
        .into_iter()
        .flatten()
        .sum()
    }

    pub fn is_valid(&self) -> bool {
        valid_non_negative_f32(self.target_end_to_end_ms)
            && [
                self.max_capture_to_encode_ms,
                self.max_network_transit_ms,
                self.max_receive_to_decode_ms,
                self.max_decode_to_texture_ms,
                self.max_texture_to_submit_ms,
                self.max_jitter_buffer_ms,
            ]
            .into_iter()
            .all(valid_optional_non_negative_f32)
    }
}

/// Policy for decoded-frame, texture, and XR-submit adoption.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerFrameAdoptionPolicy {
    pub resource_owner: BrokerMediaResourceOwner,
    pub queue_policy: BrokerMediaQueuePolicy,
    pub prediction_policy: BrokerMediaPredictionPolicy,
    pub max_in_flight_frames: u32,
    pub max_frame_age_ms: Option<f32>,
    pub allow_stale_frame_reuse: bool,
    pub release_after_submit: bool,
    pub keyframe_required_before_decode: bool,
    pub notes: Vec<String>,
}

impl BrokerFrameAdoptionPolicy {
    pub const fn new(
        resource_owner: BrokerMediaResourceOwner,
        queue_policy: BrokerMediaQueuePolicy,
        max_in_flight_frames: u32,
    ) -> Self {
        Self {
            resource_owner,
            queue_policy,
            prediction_policy: BrokerMediaPredictionPolicy::None,
            max_in_flight_frames,
            max_frame_age_ms: None,
            allow_stale_frame_reuse: false,
            release_after_submit: true,
            keyframe_required_before_decode: true,
            notes: Vec::new(),
        }
    }

    pub const fn with_prediction_policy(
        mut self,
        prediction_policy: BrokerMediaPredictionPolicy,
    ) -> Self {
        self.prediction_policy = prediction_policy;
        self
    }

    pub const fn with_max_frame_age_ms(mut self, value: f32) -> Self {
        self.max_frame_age_ms = Some(value);
        self
    }

    pub const fn with_allow_stale_frame_reuse(mut self, value: bool) -> Self {
        self.allow_stale_frame_reuse = value;
        self
    }

    pub const fn with_release_after_submit(mut self, value: bool) -> Self {
        self.release_after_submit = value;
        self
    }

    pub const fn with_keyframe_required_before_decode(mut self, value: bool) -> Self {
        self.keyframe_required_before_decode = value;
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        self.max_in_flight_frames > 0
            && self
                .max_frame_age_ms
                .map(valid_non_negative_f32)
                .unwrap_or(true)
            && self.notes.iter().all(|note| non_empty(note))
    }
}

/// Backend descriptor for route selection and fallback reporting.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerMediaBackendManifest {
    pub backend_id: String,
    pub label: String,
    pub tier: BrokerMediaBackendTier,
    pub transport: BrokerTransportKind,
    pub codec: Option<BrokerCodecId>,
    pub selected: bool,
    pub fallback_priority: u32,
    pub fallback_of: Option<String>,
    pub user_supplied_dependency: bool,
    pub dependency_label: Option<String>,
    pub rejection_reason: Option<String>,
    pub notes: Vec<String>,
}

impl BrokerMediaBackendManifest {
    pub fn new(
        backend_id: impl Into<String>,
        label: impl Into<String>,
        tier: BrokerMediaBackendTier,
        transport: BrokerTransportKind,
    ) -> Self {
        Self {
            backend_id: backend_id.into(),
            label: label.into(),
            tier,
            transport,
            codec: None,
            selected: false,
            fallback_priority: 0,
            fallback_of: None,
            user_supplied_dependency: false,
            dependency_label: None,
            rejection_reason: None,
            notes: Vec::new(),
        }
    }

    pub const fn with_codec(mut self, codec: BrokerCodecId) -> Self {
        self.codec = Some(codec);
        self
    }

    pub const fn with_selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub const fn with_fallback_priority(mut self, fallback_priority: u32) -> Self {
        self.fallback_priority = fallback_priority;
        self
    }

    pub fn with_fallback_of(mut self, backend_id: impl Into<String>) -> Self {
        self.fallback_of = Some(backend_id.into());
        self
    }

    pub fn with_user_supplied_dependency(mut self, label: impl Into<String>) -> Self {
        self.user_supplied_dependency = true;
        self.dependency_label = Some(label.into());
        self
    }

    pub fn with_rejection_reason(mut self, reason: impl Into<String>) -> Self {
        self.rejection_reason = Some(reason.into());
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        valid_route_id(&self.backend_id)
            && non_empty(&self.label)
            && self
                .fallback_of
                .as_deref()
                .map(valid_route_id)
                .unwrap_or(true)
            && self
                .dependency_label
                .as_deref()
                .map(non_empty)
                .unwrap_or(!self.user_supplied_dependency)
            && self
                .rejection_reason
                .as_deref()
                .map(non_empty)
                .unwrap_or(true)
            && self.notes.iter().all(|note| non_empty(note))
    }
}

/// Schema-only manifest for one interactive media route.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerInteractiveMediaRouteManifest {
    pub schema: String,
    pub route_id: String,
    pub label: String,
    pub direction: BrokerInteractiveMediaRouteDirection,
    pub route_scope: BrokerControlScope,
    pub module_id: Option<String>,
    pub source_stream_ids: Vec<String>,
    pub output_stream_ids: Vec<String>,
    pub planes: Vec<BrokerInteractiveMediaPlaneDescriptor>,
    pub latency_budget: BrokerMediaLatencyBudget,
    pub frame_adoption: BrokerFrameAdoptionPolicy,
    pub backends: Vec<BrokerMediaBackendManifest>,
    pub feedback_stream_ids: Vec<String>,
    pub required_consents: Vec<BrokerMediaConsentRequirement>,
    pub data_sensitivity: BrokerDataSensitivity,
    pub command_authority: Vec<BrokerCommandAuthorityRequirement>,
    pub notes: Vec<String>,
}

impl BrokerInteractiveMediaRouteManifest {
    pub fn new(
        route_id: impl Into<String>,
        label: impl Into<String>,
        direction: BrokerInteractiveMediaRouteDirection,
        route_scope: BrokerControlScope,
        latency_budget: BrokerMediaLatencyBudget,
        frame_adoption: BrokerFrameAdoptionPolicy,
        data_sensitivity: BrokerDataSensitivity,
    ) -> Self {
        Self {
            schema: BROKER_INTERACTIVE_MEDIA_ROUTE_MANIFEST_SCHEMA.to_string(),
            route_id: route_id.into(),
            label: label.into(),
            direction,
            route_scope,
            module_id: None,
            source_stream_ids: Vec::new(),
            output_stream_ids: Vec::new(),
            planes: Vec::new(),
            latency_budget,
            frame_adoption,
            backends: Vec::new(),
            feedback_stream_ids: Vec::new(),
            required_consents: Vec::new(),
            data_sensitivity,
            command_authority: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn with_module_id(mut self, module_id: impl Into<String>) -> Self {
        self.module_id = Some(module_id.into());
        self
    }

    pub fn with_source_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.source_stream_ids.push(stream_id.into());
        self
    }

    pub fn with_output_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.output_stream_ids.push(stream_id.into());
        self
    }

    pub fn with_plane(mut self, plane: BrokerInteractiveMediaPlaneDescriptor) -> Self {
        self.planes.push(plane);
        self
    }

    pub fn with_backend(mut self, backend: BrokerMediaBackendManifest) -> Self {
        self.backends.push(backend);
        self
    }

    pub fn with_feedback_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.feedback_stream_ids.push(stream_id.into());
        self
    }

    pub fn with_required_consent(mut self, consent: BrokerMediaConsentRequirement) -> Self {
        self.required_consents.push(consent);
        self
    }

    pub fn with_command_authority(mut self, authority: BrokerCommandAuthorityRequirement) -> Self {
        self.command_authority.push(authority);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn selected_backend(&self) -> Option<&BrokerMediaBackendManifest> {
        self.backends.iter().find(|backend| backend.selected)
    }

    pub fn has_all_planes(&self) -> bool {
        [
            BrokerInteractiveMediaPlane::Control,
            BrokerInteractiveMediaPlane::MediaData,
            BrokerInteractiveMediaPlane::RenderAdoption,
            BrokerInteractiveMediaPlane::Feedback,
        ]
        .into_iter()
        .all(|plane| self.planes.iter().any(|candidate| candidate.plane == plane))
    }

    pub fn media_data_uses_json_control_path(&self) -> bool {
        self.planes.iter().any(|plane| {
            plane.plane == BrokerInteractiveMediaPlane::MediaData
                && plane.high_rate_uses_json_control_path()
        })
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_INTERACTIVE_MEDIA_ROUTE_MANIFEST_SCHEMA
            && valid_route_id(&self.route_id)
            && non_empty(&self.label)
            && self.route_scope.is_valid()
            && self
                .module_id
                .as_deref()
                .map(valid_route_id)
                .unwrap_or(true)
            && self.source_stream_ids.iter().all(|id| non_empty(id))
            && self.output_stream_ids.iter().all(|id| non_empty(id))
            && self.feedback_stream_ids.iter().all(|id| non_empty(id))
            && !self.source_stream_ids.is_empty()
            && !self.output_stream_ids.is_empty()
            && !self.feedback_stream_ids.is_empty()
            && !self.required_consents.is_empty()
            && self.has_all_planes()
            && unique_planes(&self.planes)
            && self
                .planes
                .iter()
                .all(BrokerInteractiveMediaPlaneDescriptor::is_valid)
            && !self.media_data_uses_json_control_path()
            && self.latency_budget.is_valid()
            && self.frame_adoption.is_valid()
            && !self.backends.is_empty()
            && self
                .backends
                .iter()
                .all(BrokerMediaBackendManifest::is_valid)
            && self
                .backends
                .iter()
                .filter(|backend| backend.selected)
                .count()
                <= 1
            && self
                .command_authority
                .iter()
                .all(BrokerCommandAuthorityRequirement::is_valid)
            && self.notes.iter().all(|note| non_empty(note))
    }
}

/// Runtime state for one route plane.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerInteractiveMediaPlaneState {
    pub plane: BrokerInteractiveMediaPlane,
    pub lifecycle_state: BrokerInteractiveMediaRouteLifecycleState,
    pub latest_frame_state: BrokerMediaFrameLifecycleState,
    pub queue_depth: u32,
    pub dropped_count: u64,
    pub reused_frame_count: u64,
    pub latest_sequence_number: Option<u64>,
    pub issue_codes: Vec<String>,
}

impl BrokerInteractiveMediaPlaneState {
    pub const fn new(
        plane: BrokerInteractiveMediaPlane,
        lifecycle_state: BrokerInteractiveMediaRouteLifecycleState,
    ) -> Self {
        Self {
            plane,
            lifecycle_state,
            latest_frame_state: BrokerMediaFrameLifecycleState::Unknown,
            queue_depth: 0,
            dropped_count: 0,
            reused_frame_count: 0,
            latest_sequence_number: None,
            issue_codes: Vec::new(),
        }
    }

    pub const fn with_latest_frame_state(
        mut self,
        latest_frame_state: BrokerMediaFrameLifecycleState,
    ) -> Self {
        self.latest_frame_state = latest_frame_state;
        self
    }

    pub const fn with_queue_depth(mut self, queue_depth: u32) -> Self {
        self.queue_depth = queue_depth;
        self
    }

    pub const fn with_dropped_count(mut self, dropped_count: u64) -> Self {
        self.dropped_count = dropped_count;
        self
    }

    pub const fn with_reused_frame_count(mut self, reused_frame_count: u64) -> Self {
        self.reused_frame_count = reused_frame_count;
        self
    }

    pub const fn with_latest_sequence_number(mut self, sequence_number: u64) -> Self {
        self.latest_sequence_number = Some(sequence_number);
        self
    }

    pub fn with_issue_code(mut self, issue_code: impl Into<String>) -> Self {
        self.issue_codes.push(issue_code.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        self.issue_codes.iter().all(|issue| non_empty(issue))
    }
}

/// Low-rate feedback sample for one media route.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerMediaFeedbackSample {
    pub schema: String,
    pub route_id: String,
    pub session_id: String,
    pub measured_elapsed_ns: u64,
    pub stream_id: Option<String>,
    pub source_timestamp_domain: BrokerTimestampDomain,
    pub rtt_ms: Option<f32>,
    pub jitter_ms: Option<f32>,
    pub packet_loss01: Option<f32>,
    pub frame_age_ms: Option<f32>,
    pub queue_depth: u32,
    pub dropped_frames: u64,
    pub reused_frames: u64,
    pub delivered_frames: u64,
    pub issue_codes: Vec<String>,
}

impl BrokerMediaFeedbackSample {
    pub fn new(
        route_id: impl Into<String>,
        session_id: impl Into<String>,
        measured_elapsed_ns: u64,
    ) -> Self {
        Self {
            schema: BROKER_MEDIA_FEEDBACK_SAMPLE_SCHEMA.to_string(),
            route_id: route_id.into(),
            session_id: session_id.into(),
            measured_elapsed_ns,
            stream_id: None,
            source_timestamp_domain: BrokerTimestampDomain::Unknown,
            rtt_ms: None,
            jitter_ms: None,
            packet_loss01: None,
            frame_age_ms: None,
            queue_depth: 0,
            dropped_frames: 0,
            reused_frames: 0,
            delivered_frames: 0,
            issue_codes: Vec::new(),
        }
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub const fn with_source_timestamp_domain(mut self, domain: BrokerTimestampDomain) -> Self {
        self.source_timestamp_domain = domain;
        self
    }

    pub const fn with_rtt_ms(mut self, value: f32) -> Self {
        self.rtt_ms = Some(value);
        self
    }

    pub const fn with_jitter_ms(mut self, value: f32) -> Self {
        self.jitter_ms = Some(value);
        self
    }

    pub const fn with_packet_loss01(mut self, value: f32) -> Self {
        self.packet_loss01 = Some(value);
        self
    }

    pub const fn with_frame_age_ms(mut self, value: f32) -> Self {
        self.frame_age_ms = Some(value);
        self
    }

    pub const fn with_queue_depth(mut self, value: u32) -> Self {
        self.queue_depth = value;
        self
    }

    pub const fn with_counts(
        mut self,
        delivered_frames: u64,
        dropped_frames: u64,
        reused_frames: u64,
    ) -> Self {
        self.delivered_frames = delivered_frames;
        self.dropped_frames = dropped_frames;
        self.reused_frames = reused_frames;
        self
    }

    pub fn with_issue_code(mut self, issue_code: impl Into<String>) -> Self {
        self.issue_codes.push(issue_code.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_MEDIA_FEEDBACK_SAMPLE_SCHEMA
            && valid_route_id(&self.route_id)
            && non_empty(&self.session_id)
            && self.stream_id.as_deref().map(non_empty).unwrap_or(true)
            && self.rtt_ms.map(valid_non_negative_f32).unwrap_or(true)
            && self.jitter_ms.map(valid_non_negative_f32).unwrap_or(true)
            && self.packet_loss01.map(valid_unit_interval).unwrap_or(true)
            && self
                .frame_age_ms
                .map(valid_non_negative_f32)
                .unwrap_or(true)
            && self.issue_codes.iter().all(|issue| non_empty(issue))
    }
}

/// Summary scorecard for a measured media-route window.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerMediaPipelineScorecard {
    pub schema: String,
    pub route_id: String,
    pub session_id: String,
    pub window_start_elapsed_ns: u64,
    pub window_end_elapsed_ns: u64,
    pub target_latency_ms: Option<f32>,
    pub observed_p50_latency_ms: Option<f32>,
    pub observed_p95_latency_ms: Option<f32>,
    pub delivered_frame_count: u64,
    pub dropped_frame_count: u64,
    pub reused_frame_count: u64,
    pub keyframe_wait_count: u64,
    pub decoder_stall_count: u64,
    pub texture_import_failure_count: u64,
    pub xr_submit_miss_count: u64,
    pub score01: Option<f32>,
    pub verdict: BrokerMediaScoreVerdict,
    pub notes: Vec<String>,
}

impl BrokerMediaPipelineScorecard {
    pub fn new(
        route_id: impl Into<String>,
        session_id: impl Into<String>,
        window_start_elapsed_ns: u64,
        window_end_elapsed_ns: u64,
    ) -> Self {
        Self {
            schema: BROKER_MEDIA_PIPELINE_SCORECARD_SCHEMA.to_string(),
            route_id: route_id.into(),
            session_id: session_id.into(),
            window_start_elapsed_ns,
            window_end_elapsed_ns,
            target_latency_ms: None,
            observed_p50_latency_ms: None,
            observed_p95_latency_ms: None,
            delivered_frame_count: 0,
            dropped_frame_count: 0,
            reused_frame_count: 0,
            keyframe_wait_count: 0,
            decoder_stall_count: 0,
            texture_import_failure_count: 0,
            xr_submit_miss_count: 0,
            score01: None,
            verdict: BrokerMediaScoreVerdict::Unknown,
            notes: Vec::new(),
        }
    }

    pub const fn with_target_latency_ms(mut self, value: f32) -> Self {
        self.target_latency_ms = Some(value);
        self
    }

    pub const fn with_observed_latency_ms(mut self, p50: f32, p95: f32) -> Self {
        self.observed_p50_latency_ms = Some(p50);
        self.observed_p95_latency_ms = Some(p95);
        self
    }

    pub const fn with_frame_counts(mut self, delivered: u64, dropped: u64, reused: u64) -> Self {
        self.delivered_frame_count = delivered;
        self.dropped_frame_count = dropped;
        self.reused_frame_count = reused;
        self
    }

    pub const fn with_issue_counts(
        mut self,
        keyframe_wait: u64,
        decoder_stall: u64,
        texture_import_failure: u64,
        xr_submit_miss: u64,
    ) -> Self {
        self.keyframe_wait_count = keyframe_wait;
        self.decoder_stall_count = decoder_stall;
        self.texture_import_failure_count = texture_import_failure;
        self.xr_submit_miss_count = xr_submit_miss;
        self
    }

    pub const fn with_score(mut self, score01: f32, verdict: BrokerMediaScoreVerdict) -> Self {
        self.score01 = Some(score01);
        self.verdict = verdict;
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_MEDIA_PIPELINE_SCORECARD_SCHEMA
            && valid_route_id(&self.route_id)
            && non_empty(&self.session_id)
            && self.window_end_elapsed_ns >= self.window_start_elapsed_ns
            && self
                .target_latency_ms
                .map(valid_non_negative_f32)
                .unwrap_or(true)
            && self
                .observed_p50_latency_ms
                .map(valid_non_negative_f32)
                .unwrap_or(true)
            && self
                .observed_p95_latency_ms
                .map(valid_non_negative_f32)
                .unwrap_or(true)
            && self.score01.map(valid_unit_interval).unwrap_or(true)
            && self.notes.iter().all(|note| non_empty(note))
    }
}

/// Runtime state for one interactive media route.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerInteractiveMediaRouteRuntimeState {
    pub schema: String,
    pub route_id: String,
    pub session_id: String,
    pub lifecycle_state: BrokerInteractiveMediaRouteLifecycleState,
    pub revision: u64,
    pub selected_backend_id: Option<String>,
    pub started_elapsed_ns: Option<u64>,
    pub last_update_elapsed_ns: Option<u64>,
    pub plane_states: Vec<BrokerInteractiveMediaPlaneState>,
    pub feedback_sample: Option<BrokerMediaFeedbackSample>,
    pub scorecard: Option<BrokerMediaPipelineScorecard>,
    pub issue_codes: Vec<String>,
}

impl BrokerInteractiveMediaRouteRuntimeState {
    pub fn new(
        route_id: impl Into<String>,
        session_id: impl Into<String>,
        lifecycle_state: BrokerInteractiveMediaRouteLifecycleState,
        revision: u64,
    ) -> Self {
        Self {
            schema: BROKER_INTERACTIVE_MEDIA_ROUTE_RUNTIME_STATE_SCHEMA.to_string(),
            route_id: route_id.into(),
            session_id: session_id.into(),
            lifecycle_state,
            revision,
            selected_backend_id: None,
            started_elapsed_ns: None,
            last_update_elapsed_ns: None,
            plane_states: Vec::new(),
            feedback_sample: None,
            scorecard: None,
            issue_codes: Vec::new(),
        }
    }

    pub fn with_selected_backend_id(mut self, backend_id: impl Into<String>) -> Self {
        self.selected_backend_id = Some(backend_id.into());
        self
    }

    pub const fn with_started_elapsed_ns(mut self, value: u64) -> Self {
        self.started_elapsed_ns = Some(value);
        self
    }

    pub const fn with_last_update_elapsed_ns(mut self, value: u64) -> Self {
        self.last_update_elapsed_ns = Some(value);
        self
    }

    pub fn with_plane_state(mut self, plane_state: BrokerInteractiveMediaPlaneState) -> Self {
        self.plane_states.push(plane_state);
        self
    }

    pub fn with_feedback_sample(mut self, feedback_sample: BrokerMediaFeedbackSample) -> Self {
        self.feedback_sample = Some(feedback_sample);
        self
    }

    pub fn with_scorecard(mut self, scorecard: BrokerMediaPipelineScorecard) -> Self {
        self.scorecard = Some(scorecard);
        self
    }

    pub fn with_issue_code(mut self, issue_code: impl Into<String>) -> Self {
        self.issue_codes.push(issue_code.into());
        self
    }

    pub fn is_streaming(&self) -> bool {
        self.lifecycle_state == BrokerInteractiveMediaRouteLifecycleState::Streaming
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_INTERACTIVE_MEDIA_ROUTE_RUNTIME_STATE_SCHEMA
            && valid_route_id(&self.route_id)
            && non_empty(&self.session_id)
            && self
                .selected_backend_id
                .as_deref()
                .map(valid_route_id)
                .unwrap_or(!self.is_streaming())
            && ordered_optional_pair(self.started_elapsed_ns, self.last_update_elapsed_ns)
            && !self.plane_states.is_empty()
            && unique_plane_states(&self.plane_states)
            && self
                .plane_states
                .iter()
                .all(BrokerInteractiveMediaPlaneState::is_valid)
            && self
                .feedback_sample
                .as_ref()
                .map(BrokerMediaFeedbackSample::is_valid)
                .unwrap_or(true)
            && self
                .scorecard
                .as_ref()
                .map(BrokerMediaPipelineScorecard::is_valid)
                .unwrap_or(true)
            && self.issue_codes.iter().all(|issue| non_empty(issue))
    }
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_route_id(value: &str) -> bool {
    non_empty(value)
        && value.chars().all(|ch| {
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '_' | '-')
        })
        && value
            .chars()
            .next()
            .map(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
            .unwrap_or(false)
        && value
            .chars()
            .last()
            .map(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
            .unwrap_or(false)
}

fn valid_non_negative_f32(value: f32) -> bool {
    value.is_finite() && value >= 0.0
}

fn valid_optional_non_negative_f32(value: Option<f32>) -> bool {
    value.map(valid_non_negative_f32).unwrap_or(true)
}

fn valid_unit_interval(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn ordered_optional_pair(start: Option<u64>, end: Option<u64>) -> bool {
    match (start, end) {
        (Some(start), Some(end)) => end >= start,
        _ => true,
    }
}

fn unique_planes(planes: &[BrokerInteractiveMediaPlaneDescriptor]) -> bool {
    let mut seen = Vec::new();
    for plane in planes {
        if seen.contains(&plane.plane) {
            return false;
        }
        seen.push(plane.plane);
    }
    true
}

fn unique_plane_states(plane_states: &[BrokerInteractiveMediaPlaneState]) -> bool {
    let mut seen = Vec::new();
    for plane_state in plane_states {
        if seen.contains(&plane_state.plane) {
            return false;
        }
        seen.push(plane_state.plane);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::{
        BrokerFrameAdoptionPolicy, BrokerInteractiveMediaPlane,
        BrokerInteractiveMediaPlaneDescriptor, BrokerInteractiveMediaPlaneState,
        BrokerInteractiveMediaRouteDirection, BrokerInteractiveMediaRouteLifecycleState,
        BrokerInteractiveMediaRouteManifest, BrokerInteractiveMediaRouteRuntimeState,
        BrokerMediaBackendManifest, BrokerMediaBackendTier, BrokerMediaConsentRequirement,
        BrokerMediaFeedbackSample, BrokerMediaFrameLifecycleState, BrokerMediaLatencyBudget,
        BrokerMediaPipelineScorecard, BrokerMediaQueuePolicy, BrokerMediaResourceOwner,
        BrokerMediaScoreVerdict, BROKER_INTERACTIVE_MEDIA_ROUTE_MANIFEST_SCHEMA,
        BROKER_INTERACTIVE_MEDIA_ROUTE_RUNTIME_STATE_SCHEMA,
    };
    use crate::{
        BrokerCodecId, BrokerCommandAuthorityRequirement, BrokerControlScope,
        BrokerDataSensitivity, BrokerPayloadKind, BrokerTimestampDomain, BrokerTransportKind,
    };

    fn synthetic_route_manifest() -> BrokerInteractiveMediaRouteManifest {
        let route_scope = BrokerControlScope::new("media.synthetic", "media.synthetic")
            .with_resource_id("route:media.synthetic.h264");
        let latency_budget = BrokerMediaLatencyBudget::new(45.0)
            .with_capture_to_encode_ms(8.0)
            .with_network_transit_ms(12.0)
            .with_receive_to_decode_ms(8.0)
            .with_decode_to_texture_ms(6.0)
            .with_texture_to_submit_ms(5.0)
            .with_jitter_buffer_ms(6.0);
        let frame_adoption = BrokerFrameAdoptionPolicy::new(
            BrokerMediaResourceOwner::ActiveXrApp,
            BrokerMediaQueuePolicy::ReuseLatest,
            3,
        )
        .with_max_frame_age_ms(50.0)
        .with_allow_stale_frame_reuse(true)
        .with_note("Active XR app owns texture adoption and OpenXR submission.");

        BrokerInteractiveMediaRouteManifest::new(
            "media.synthetic.h264",
            "Synthetic H264 into XR app",
            BrokerInteractiveMediaRouteDirection::IntoXrApp,
            route_scope,
            latency_budget,
            frame_adoption,
            BrokerDataSensitivity::Diagnostic,
        )
        .with_module_id("video.lab")
        .with_source_stream_id("video_lab.encoded_stream_manifest")
        .with_output_stream_id("video_lab.encoded_sample_metadata")
        .with_plane(
            BrokerInteractiveMediaPlaneDescriptor::new(
                BrokerInteractiveMediaPlane::Control,
                BrokerMediaResourceOwner::Broker,
                BrokerTransportKind::WebSocket,
                BrokerPayloadKind::Json,
                "rusty.xr.broker.command.v1",
                BrokerDataSensitivity::Diagnostic,
            )
            .with_json_control_path(true)
            .with_note("Commands negotiate route setup and report low-rate state only."),
        )
        .with_plane(
            BrokerInteractiveMediaPlaneDescriptor::new(
                BrokerInteractiveMediaPlane::MediaData,
                BrokerMediaResourceOwner::ExternalSidecar,
                BrokerTransportKind::AdbForwardedTcp,
                BrokerPayloadKind::H264,
                "rusty.xr.video_lab.binary_stream.v1",
                BrokerDataSensitivity::Diagnostic,
            )
            .with_stream_id("video_lab.encoded_sample_metadata")
            .with_high_rate(true)
            .with_max_payload_bytes(1_048_576)
            .with_note("High-rate encoded bytes use a binary lane, not JSON commands."),
        )
        .with_plane(
            BrokerInteractiveMediaPlaneDescriptor::new(
                BrokerInteractiveMediaPlane::RenderAdoption,
                BrokerMediaResourceOwner::ActiveXrApp,
                BrokerTransportKind::MetadataOnly,
                BrokerPayloadKind::Json,
                "rusty.xr.broker.media_adoption_state.v1",
                BrokerDataSensitivity::Diagnostic,
            )
            .with_note("The active XR app owns decode, texture import, projection, and submit."),
        )
        .with_plane(
            BrokerInteractiveMediaPlaneDescriptor::new(
                BrokerInteractiveMediaPlane::Feedback,
                BrokerMediaResourceOwner::ActiveXrApp,
                BrokerTransportKind::WebSocket,
                BrokerPayloadKind::Json,
                "rusty.xr.broker.media_feedback_sample.v1",
                BrokerDataSensitivity::Diagnostic,
            )
            .with_stream_id("video_lab.metric_sample")
            .with_note("Feedback is low-rate route health and scorecard telemetry."),
        )
        .with_backend(
            BrokerMediaBackendManifest::new(
                "android.mediacodec.surface",
                "Android MediaCodec surface",
                BrokerMediaBackendTier::Hardware,
                BrokerTransportKind::AdbForwardedTcp,
            )
            .with_codec(BrokerCodecId::H264)
            .with_selected(true)
            .with_fallback_priority(0)
            .with_note("Uses platform codec APIs supplied by the app shell."),
        )
        .with_backend(
            BrokerMediaBackendManifest::new(
                "bytebuffer.reference",
                "Byte-buffer reference decoder",
                BrokerMediaBackendTier::Reference,
                BrokerTransportKind::MetadataOnly,
            )
            .with_codec(BrokerCodecId::H264)
            .with_fallback_priority(1)
            .with_fallback_of("android.mediacodec.surface")
            .with_note("Reference lane for diagnostics, not the production texture path."),
        )
        .with_feedback_stream_id("video_lab.metric_sample")
        .with_required_consent(BrokerMediaConsentRequirement::NotRequired)
        .with_command_authority(BrokerCommandAuthorityRequirement::read_only(
            "media.route.get_status",
            "media.synthetic",
        ))
        .with_note("Schema-only fixture; no runtime media backend is loaded by this manifest.")
    }

    #[test]
    fn media_route_manifest_separates_planes_and_backends() {
        let manifest = synthetic_route_manifest();

        assert!(manifest.is_valid());
        assert_eq!(
            manifest.schema,
            BROKER_INTERACTIVE_MEDIA_ROUTE_MANIFEST_SCHEMA
        );
        assert!(manifest.has_all_planes());
        assert!(!manifest.media_data_uses_json_control_path());
        assert_eq!(
            manifest
                .selected_backend()
                .map(|backend| backend.backend_id.as_str()),
            Some("android.mediacodec.surface")
        );
        assert!(manifest.latency_budget.component_budget_ms() <= 45.0);
    }

    #[test]
    fn media_route_manifest_rejects_high_rate_json_control_payloads() {
        let mut manifest = synthetic_route_manifest();
        let data_plane = manifest
            .planes
            .iter_mut()
            .find(|plane| plane.plane == BrokerInteractiveMediaPlane::MediaData)
            .expect("media data plane exists");
        data_plane.json_control_path = true;

        assert!(!manifest.is_valid());
        assert!(manifest.media_data_uses_json_control_path());
    }

    #[test]
    fn media_route_runtime_state_binds_feedback_and_scorecard() {
        let feedback =
            BrokerMediaFeedbackSample::new("media.synthetic.h264", "session-001", 1_000_000)
                .with_stream_id("video_lab.metric_sample")
                .with_source_timestamp_domain(BrokerTimestampDomain::ElapsedRealtime)
                .with_frame_age_ms(22.5)
                .with_jitter_ms(2.0)
                .with_packet_loss01(0.0)
                .with_queue_depth(1)
                .with_counts(90, 1, 2);
        let scorecard = BrokerMediaPipelineScorecard::new(
            "media.synthetic.h264",
            "session-001",
            1_000_000,
            2_000_000,
        )
        .with_target_latency_ms(45.0)
        .with_observed_latency_ms(31.0, 44.0)
        .with_frame_counts(90, 1, 2)
        .with_score(0.92, BrokerMediaScoreVerdict::Pass);
        let state = BrokerInteractiveMediaRouteRuntimeState::new(
            "media.synthetic.h264",
            "session-001",
            BrokerInteractiveMediaRouteLifecycleState::Streaming,
            8,
        )
        .with_selected_backend_id("android.mediacodec.surface")
        .with_started_elapsed_ns(900_000)
        .with_last_update_elapsed_ns(2_000_000)
        .with_plane_state(
            BrokerInteractiveMediaPlaneState::new(
                BrokerInteractiveMediaPlane::Control,
                BrokerInteractiveMediaRouteLifecycleState::Streaming,
            )
            .with_latest_frame_state(BrokerMediaFrameLifecycleState::PacketReceived),
        )
        .with_plane_state(
            BrokerInteractiveMediaPlaneState::new(
                BrokerInteractiveMediaPlane::MediaData,
                BrokerInteractiveMediaRouteLifecycleState::Streaming,
            )
            .with_latest_frame_state(BrokerMediaFrameLifecycleState::PacketReceived)
            .with_latest_sequence_number(90),
        )
        .with_plane_state(
            BrokerInteractiveMediaPlaneState::new(
                BrokerInteractiveMediaPlane::RenderAdoption,
                BrokerInteractiveMediaRouteLifecycleState::Streaming,
            )
            .with_latest_frame_state(BrokerMediaFrameLifecycleState::XrSubmitted)
            .with_reused_frame_count(2),
        )
        .with_plane_state(
            BrokerInteractiveMediaPlaneState::new(
                BrokerInteractiveMediaPlane::Feedback,
                BrokerInteractiveMediaRouteLifecycleState::Streaming,
            )
            .with_latest_frame_state(BrokerMediaFrameLifecycleState::PresentedEstimate),
        )
        .with_feedback_sample(feedback)
        .with_scorecard(scorecard);

        assert!(state.is_valid());
        assert_eq!(
            state.schema,
            BROKER_INTERACTIVE_MEDIA_ROUTE_RUNTIME_STATE_SCHEMA
        );
        assert!(state.is_streaming());
    }

    #[test]
    fn streaming_route_state_requires_selected_backend() {
        let state = BrokerInteractiveMediaRouteRuntimeState::new(
            "media.synthetic.h264",
            "session-001",
            BrokerInteractiveMediaRouteLifecycleState::Streaming,
            1,
        )
        .with_plane_state(BrokerInteractiveMediaPlaneState::new(
            BrokerInteractiveMediaPlane::Control,
            BrokerInteractiveMediaRouteLifecycleState::Streaming,
        ));

        assert!(!state.is_valid());
    }

    #[test]
    fn h264_feedback_sample_counts_are_low_rate_metadata() {
        let feedback =
            BrokerMediaFeedbackSample::new("media.synthetic.h264", "session-001", 1_000_000)
                .with_stream_id("video_lab.metric_sample")
                .with_rtt_ms(18.0)
                .with_jitter_ms(2.0)
                .with_packet_loss01(0.02)
                .with_counts(120, 2, 4);

        assert!(feedback.is_valid());
        assert_eq!(feedback.delivered_frames, 120);
    }

    #[test]
    fn scorecard_rejects_invalid_windows_and_scores() {
        let invalid_window =
            BrokerMediaPipelineScorecard::new("media.synthetic.h264", "session-001", 2_000, 1_000);
        let invalid_score =
            BrokerMediaPipelineScorecard::new("media.synthetic.h264", "session-001", 1_000, 2_000)
                .with_score(1.5, BrokerMediaScoreVerdict::Warning);

        assert!(!invalid_window.is_valid());
        assert!(!invalid_score.is_valid());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn public_interactive_media_route_manifest_fixture_deserializes() {
        let manifest: BrokerInteractiveMediaRouteManifest = serde_json::from_str(include_str!(
            "../../../fixtures/broker-ui/synthetic-interactive-media-route-manifest.json"
        ))
        .expect("public interactive media route manifest should deserialize");

        assert!(manifest.is_valid());
        assert_eq!(manifest.route_id, "media.synthetic.h264");
        assert!(manifest.has_all_planes());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn public_interactive_media_route_runtime_fixture_deserializes() {
        let state: BrokerInteractiveMediaRouteRuntimeState = serde_json::from_str(include_str!(
            "../../../fixtures/broker-ui/synthetic-interactive-media-route-runtime-state.json"
        ))
        .expect("public interactive media route runtime state should deserialize");

        assert!(state.is_valid());
        assert_eq!(state.route_id, "media.synthetic.h264");
        assert!(state.is_streaming());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn public_media_feedback_and_scorecard_fixtures_deserialize() {
        let feedback: BrokerMediaFeedbackSample = serde_json::from_str(include_str!(
            "../../../fixtures/broker-ui/synthetic-media-feedback-sample.json"
        ))
        .expect("public media feedback fixture should deserialize");
        let scorecard: BrokerMediaPipelineScorecard = serde_json::from_str(include_str!(
            "../../../fixtures/broker-ui/synthetic-media-pipeline-scorecard.json"
        ))
        .expect("public media scorecard fixture should deserialize");

        assert!(feedback.is_valid());
        assert!(scorecard.is_valid());
        assert_eq!(scorecard.verdict, BrokerMediaScoreVerdict::Pass);
    }
}
