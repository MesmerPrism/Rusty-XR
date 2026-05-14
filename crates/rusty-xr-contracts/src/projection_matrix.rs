use crate::{Eye, ImageSize};

/// Versioned schema id for renderer-neutral projection/performance matrix packets.
pub const PROJECTION_PERFORMANCE_MATRIX_SCHEMA: &str = "rusty.xr.projection_performance_matrix.v1";

/// Coarse renderer or ingestion lane class used by comparison packets.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProjectionMatrixLaneKind {
    /// Renderer uses OpenXR presentation with Vulkan-visible hardware buffers.
    #[default]
    OpenXrVulkanHardwareBuffer,
    /// Renderer uses a framework path with CPU-visible YUV texture updates.
    FrameworkCpuYuv,
    /// Renderer uses OpenXR presentation with OpenGL ES SurfaceTexture/OES input.
    OpenXrOpenGlSurfaceTextureOes,
    /// Renderer is a private or external reference lane summarized only by data.
    Reference,
    /// Renderer is another public or downstream lane.
    Other,
}

/// Status for one ordered matrix gate.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MatrixStepStatus {
    /// The lane has not attempted this gate.
    #[default]
    NotRun,
    /// The lane produced comparable data and passed the gate.
    Passed,
    /// The lane produced comparable data and failed the gate.
    Failed,
    /// The lane cannot yet produce equivalent data.
    Blocked,
    /// The gate does not apply to this lane.
    NotApplicable,
    /// The lane produced partial data but the result is not decisive.
    Ambiguous,
}

impl MatrixStepStatus {
    pub const fn is_terminal_blocker(self) -> bool {
        matches!(self, Self::Failed | Self::Blocked)
    }
}

/// Projection matrix stage represented by a stage token or homography row.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProjectionStageKind {
    /// Display-eye screen UV to submitted surface/content UV.
    #[default]
    ScreenToSurface,
    /// Submitted surface/content UV to camera/source UV.
    SurfaceToCamera,
    /// Display-eye screen UV directly to camera/source UV.
    ScreenToCamera,
}

/// Guide or projection-footprint sampling domain.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProjectionGuideDomain {
    #[default]
    Unknown,
    DisplayScreen,
    SubmittedSurface,
    DirectSurfaceCamera,
    ScreenCamera,
    Other,
}

/// Fill policy used when projected camera UVs are invalid.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InvalidProjectionFillPolicy {
    #[default]
    Unknown,
    Black,
    Clamp,
    Repeat,
    OrientedSourceFallback,
    VisualContinuityFallback,
    Other,
}

/// Shared synthetic H.264 source configuration for deterministic comparisons.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixSyntheticVideoSource {
    pub pattern: String,
    pub size: ImageSize,
    pub left_port: u16,
    pub right_port: u16,
    pub bitrate_bps: u32,
    pub max_packets: u32,
    pub stream_header_projection_metadata: bool,
    pub live_unbounded: bool,
}

impl MatrixSyntheticVideoSource {
    pub fn broker_h264_diagnostic_grid_1280() -> Self {
        Self {
            pattern: String::from("diagnostic-grid"),
            size: ImageSize::new(1280, 1280),
            left_port: 8879,
            right_port: 8880,
            bitrate_bps: 6_000_000,
            max_packets: 0,
            stream_header_projection_metadata: true,
            live_unbounded: true,
        }
    }

    pub fn is_valid(&self) -> bool {
        non_empty(&self.pattern)
            && self.size.is_non_empty()
            && self.left_port != 0
            && self.right_port != 0
            && self.left_port != self.right_port
            && self.bitrate_bps > 0
            && self.live_unbounded == (self.max_packets == 0)
    }
}

/// Stage-token row for one eye and one projection transform.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectionStageTokenRow {
    pub lane_id: String,
    pub eye: Eye,
    pub stage: ProjectionStageKind,
    pub token: Option<String>,
    pub rows: Option<[[f32; 3]; 3]>,
    pub source: Option<String>,
}

impl ProjectionStageTokenRow {
    pub fn new(lane_id: impl Into<String>, eye: Eye, stage: ProjectionStageKind) -> Self {
        Self {
            lane_id: lane_id.into(),
            eye,
            stage,
            token: None,
            rows: None,
            source: None,
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub const fn with_rows(mut self, rows: [[f32; 3]; 3]) -> Self {
        self.rows = Some(rows);
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        stable_id(&self.lane_id)
            && self.eye != Eye::Mono
            && self
                .token
                .as_ref()
                .map(|token| non_empty(token))
                .unwrap_or(true)
            && self.rows.map(homography_rows_are_valid).unwrap_or(true)
            && (self.token.is_some() || self.rows.is_some())
    }
}

/// Observable span for a projection footprint at one normalized row.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ProjectionFootprintRowSpan {
    pub row_fraction: f32,
    pub x0_fraction: Option<f32>,
    pub x1_fraction: Option<f32>,
    pub width_fraction: f32,
    pub center_fraction: Option<f32>,
}

impl ProjectionFootprintRowSpan {
    pub const fn new(row_fraction: f32, width_fraction: f32) -> Self {
        Self {
            row_fraction,
            x0_fraction: None,
            x1_fraction: None,
            width_fraction,
            center_fraction: None,
        }
    }

    pub const fn with_span(mut self, x0_fraction: f32, x1_fraction: f32) -> Self {
        self.x0_fraction = Some(x0_fraction);
        self.x1_fraction = Some(x1_fraction);
        self.center_fraction = Some((x0_fraction + x1_fraction) * 0.5);
        self
    }

    pub fn is_valid(self) -> bool {
        unit(self.row_fraction)
            && unit(self.width_fraction)
            && self.x0_fraction.map(unit).unwrap_or(true)
            && self.x1_fraction.map(unit).unwrap_or(true)
            && self.center_fraction.map(unit).unwrap_or(true)
    }
}

/// Projection footprint summary for one lane/layer.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectionFootprintSummary {
    pub lane_id: String,
    pub layer_id: String,
    pub active_fraction: Option<f32>,
    pub bbox_fraction: Option<[f32; 4]>,
    pub row_spans: Vec<ProjectionFootprintRowSpan>,
    pub mask_iou_against_reference: Option<f32>,
    pub invalid_fill_policy: InvalidProjectionFillPolicy,
    pub guide_domain: ProjectionGuideDomain,
    pub explicit_valid_mask: bool,
    pub note: Option<String>,
}

impl ProjectionFootprintSummary {
    pub fn new(lane_id: impl Into<String>, layer_id: impl Into<String>) -> Self {
        Self {
            lane_id: lane_id.into(),
            layer_id: layer_id.into(),
            active_fraction: None,
            bbox_fraction: None,
            row_spans: Vec::new(),
            mask_iou_against_reference: None,
            invalid_fill_policy: InvalidProjectionFillPolicy::Unknown,
            guide_domain: ProjectionGuideDomain::Unknown,
            explicit_valid_mask: false,
            note: None,
        }
    }

    pub const fn with_active_fraction(mut self, active_fraction: f32) -> Self {
        self.active_fraction = Some(active_fraction);
        self
    }

    pub const fn with_bbox_fraction(mut self, bbox_fraction: [f32; 4]) -> Self {
        self.bbox_fraction = Some(bbox_fraction);
        self
    }

    pub fn with_row_span(mut self, row_span: ProjectionFootprintRowSpan) -> Self {
        self.row_spans.push(row_span);
        self
    }

    pub const fn with_mask_iou_against_reference(mut self, mask_iou: f32) -> Self {
        self.mask_iou_against_reference = Some(mask_iou);
        self
    }

    pub const fn with_invalid_fill_policy(mut self, policy: InvalidProjectionFillPolicy) -> Self {
        self.invalid_fill_policy = policy;
        self
    }

    pub const fn with_guide_domain(mut self, domain: ProjectionGuideDomain) -> Self {
        self.guide_domain = domain;
        self
    }

    pub const fn with_explicit_valid_mask(mut self, explicit_valid_mask: bool) -> Self {
        self.explicit_valid_mask = explicit_valid_mask;
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        stable_id(&self.lane_id)
            && stable_id(&self.layer_id)
            && self.active_fraction.map(unit).unwrap_or(true)
            && self.bbox_fraction.map(four_unit_values).unwrap_or(true)
            && self.mask_iou_against_reference.map(unit).unwrap_or(true)
            && self.row_spans.iter().all(|row_span| row_span.is_valid())
            && self
                .note
                .as_ref()
                .map(|note| non_empty(note))
                .unwrap_or(true)
    }
}

/// Optional performance and pass-budget fields for one lane.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProjectionPerformanceScorecard {
    pub source_packet_fps: Option<f32>,
    pub decoder_input_access_unit_fps: Option<f32>,
    pub decoded_texture_update_fps: Option<f32>,
    pub surface_texture_update_count: Option<u64>,
    pub surface_texture_skipped_frame_count: Option<u64>,
    pub cpu_yuv_upload_update_fps: Option<f32>,
    pub hardware_buffer_import_count: Option<u64>,
    pub hardware_buffer_import_cache_miss_count: Option<u64>,
    pub hardware_buffer_import_cache_evict_count: Option<u64>,
    pub openxr_fps: Option<f32>,
    pub app_cpu_ms: Option<f32>,
    pub app_gpu_ms: Option<f32>,
    pub app_cpu_gpu_ms: Option<f32>,
    pub gpu_percent: Option<f32>,
    pub thermal_status: Option<i32>,
    pub performance_level_cpu: Option<u8>,
    pub performance_level_gpu: Option<u8>,
    pub pass_count: Option<u32>,
    pub fbo_switch_count: Option<u32>,
    pub render_target_switch_count: Option<u32>,
    pub intermediate_texture_bytes_per_frame: Option<u64>,
    pub frame_age_at_submit_ms: Option<f32>,
    pub repeated_render_frames_per_distinct_source_frame: Option<f32>,
    pub app_fatal_count: Option<u32>,
    pub gpu_fault_count: Option<u32>,
    pub android_runtime_crash_count: Option<u32>,
}

impl ProjectionPerformanceScorecard {
    pub fn is_valid(&self) -> bool {
        optional_non_negative(self.source_packet_fps)
            && optional_non_negative(self.decoder_input_access_unit_fps)
            && optional_non_negative(self.decoded_texture_update_fps)
            && optional_non_negative(self.cpu_yuv_upload_update_fps)
            && optional_non_negative(self.openxr_fps)
            && optional_non_negative(self.app_cpu_ms)
            && optional_non_negative(self.app_gpu_ms)
            && optional_non_negative(self.app_cpu_gpu_ms)
            && optional_non_negative(self.gpu_percent)
            && optional_non_negative(self.frame_age_at_submit_ms)
            && optional_non_negative(self.repeated_render_frames_per_distinct_source_frame)
    }
}

/// One lane row in a deterministic projection/performance matrix.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectionMatrixLaneReport {
    pub lane_id: String,
    pub label: String,
    pub kind: ProjectionMatrixLaneKind,
    pub source_feed: MatrixStepStatus,
    pub decoded_texture: MatrixStepStatus,
    pub projection_stage: MatrixStepStatus,
    pub projection_footprint: MatrixStepStatus,
    pub public_or_raw_layer: MatrixStepStatus,
    pub effect_or_guide_layer: MatrixStepStatus,
    pub performance_budget: MatrixStepStatus,
    pub stage_tokens: Vec<ProjectionStageTokenRow>,
    pub footprints: Vec<ProjectionFootprintSummary>,
    pub performance: Option<ProjectionPerformanceScorecard>,
    pub notes: Vec<String>,
    pub blockers: Vec<String>,
}

impl ProjectionMatrixLaneReport {
    pub fn new(
        lane_id: impl Into<String>,
        label: impl Into<String>,
        kind: ProjectionMatrixLaneKind,
    ) -> Self {
        Self {
            lane_id: lane_id.into(),
            label: label.into(),
            kind,
            source_feed: MatrixStepStatus::NotRun,
            decoded_texture: MatrixStepStatus::NotRun,
            projection_stage: MatrixStepStatus::NotRun,
            projection_footprint: MatrixStepStatus::NotRun,
            public_or_raw_layer: MatrixStepStatus::NotRun,
            effect_or_guide_layer: MatrixStepStatus::NotRun,
            performance_budget: MatrixStepStatus::NotRun,
            stage_tokens: Vec::new(),
            footprints: Vec::new(),
            performance: None,
            notes: Vec::new(),
            blockers: Vec::new(),
        }
    }

    pub const fn with_source_feed(mut self, status: MatrixStepStatus) -> Self {
        self.source_feed = status;
        self
    }

    pub const fn with_decoded_texture(mut self, status: MatrixStepStatus) -> Self {
        self.decoded_texture = status;
        self
    }

    pub const fn with_projection_stage(mut self, status: MatrixStepStatus) -> Self {
        self.projection_stage = status;
        self
    }

    pub const fn with_projection_footprint(mut self, status: MatrixStepStatus) -> Self {
        self.projection_footprint = status;
        self
    }

    pub const fn with_public_or_raw_layer(mut self, status: MatrixStepStatus) -> Self {
        self.public_or_raw_layer = status;
        self
    }

    pub const fn with_effect_or_guide_layer(mut self, status: MatrixStepStatus) -> Self {
        self.effect_or_guide_layer = status;
        self
    }

    pub const fn with_performance_budget(mut self, status: MatrixStepStatus) -> Self {
        self.performance_budget = status;
        self
    }

    pub fn with_stage_token(mut self, row: ProjectionStageTokenRow) -> Self {
        self.stage_tokens.push(row);
        self
    }

    pub fn with_footprint(mut self, footprint: ProjectionFootprintSummary) -> Self {
        self.footprints.push(footprint);
        self
    }

    pub fn with_performance(mut self, performance: ProjectionPerformanceScorecard) -> Self {
        self.performance = Some(performance);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_blocker(mut self, blocker: impl Into<String>) -> Self {
        self.blockers.push(blocker.into());
        self
    }

    pub fn first_blocking_step(&self) -> Option<&'static str> {
        [
            ("source_feed", self.source_feed),
            ("decoded_texture", self.decoded_texture),
            ("projection_stage", self.projection_stage),
            ("projection_footprint", self.projection_footprint),
            ("public_or_raw_layer", self.public_or_raw_layer),
            ("effect_or_guide_layer", self.effect_or_guide_layer),
            ("performance_budget", self.performance_budget),
        ]
        .into_iter()
        .find_map(|(step, status)| status.is_terminal_blocker().then_some(step))
    }

    pub fn is_valid(&self) -> bool {
        stable_id(&self.lane_id)
            && non_empty(&self.label)
            && self
                .stage_tokens
                .iter()
                .all(ProjectionStageTokenRow::is_valid)
            && self
                .footprints
                .iter()
                .all(ProjectionFootprintSummary::is_valid)
            && self
                .performance
                .as_ref()
                .map(ProjectionPerformanceScorecard::is_valid)
                .unwrap_or(true)
            && self.notes.iter().all(|note| non_empty(note))
            && self.blockers.iter().all(|blocker| non_empty(blocker))
    }
}

/// Single packet for a deterministic cross-lane projection and performance matrix.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectionPerformanceMatrixPacket {
    pub schema: String,
    pub packet_id: String,
    pub source: MatrixSyntheticVideoSource,
    pub lanes: Vec<ProjectionMatrixLaneReport>,
    pub notes: Vec<String>,
}

impl ProjectionPerformanceMatrixPacket {
    pub fn new(packet_id: impl Into<String>, source: MatrixSyntheticVideoSource) -> Self {
        Self {
            schema: PROJECTION_PERFORMANCE_MATRIX_SCHEMA.to_string(),
            packet_id: packet_id.into(),
            source,
            lanes: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn with_lane(mut self, lane: ProjectionMatrixLaneReport) -> Self {
        self.lanes.push(lane);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn first_blocked_lane(&self) -> Option<&ProjectionMatrixLaneReport> {
        self.lanes
            .iter()
            .find(|lane| lane.first_blocking_step().is_some())
    }

    pub fn is_valid(&self) -> bool {
        self.schema == PROJECTION_PERFORMANCE_MATRIX_SCHEMA
            && stable_id(&self.packet_id)
            && self.source.is_valid()
            && !self.lanes.is_empty()
            && all_unique(self.lanes.iter().map(|lane| lane.lane_id.as_str()))
            && self.lanes.iter().all(ProjectionMatrixLaneReport::is_valid)
            && self.notes.iter().all(|note| non_empty(note))
    }
}

fn all_unique<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let values: Vec<&str> = values.collect();
    values
        .iter()
        .enumerate()
        .all(|(index, value)| !values.iter().skip(index + 1).any(|other| other == value))
}

fn stable_id(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':' | '/' | '+'))
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn finite(value: f32) -> bool {
    value.is_finite()
}

fn non_negative(value: f32) -> bool {
    value.is_finite() && value >= 0.0
}

fn unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn optional_non_negative(value: Option<f32>) -> bool {
    value.map(non_negative).unwrap_or(true)
}

fn four_unit_values(values: [f32; 4]) -> bool {
    values.into_iter().all(unit)
}

fn homography_rows_are_valid(rows: [[f32; 3]; 3]) -> bool {
    rows.into_iter().flatten().all(finite)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> [[f32; 3]; 3] {
        [[1.0, 0.0, 0.01], [0.0, 1.0, 0.02], [0.0, 0.0, 1.0]]
    }

    #[test]
    fn broker_grid_source_is_live_unbounded() {
        let source = MatrixSyntheticVideoSource::broker_h264_diagnostic_grid_1280();
        assert!(source.is_valid());
        assert_eq!(source.max_packets, 0);
        assert!(source.live_unbounded);
        assert!(source.stream_header_projection_metadata);
    }

    #[test]
    fn matrix_packet_tracks_blocking_lane() {
        let source = MatrixSyntheticVideoSource::broker_h264_diagnostic_grid_1280();
        let reference = ProjectionMatrixLaneReport::new(
            "reference",
            "reference renderer",
            ProjectionMatrixLaneKind::Reference,
        )
        .with_source_feed(MatrixStepStatus::Passed)
        .with_decoded_texture(MatrixStepStatus::Passed)
        .with_projection_stage(MatrixStepStatus::Passed)
        .with_stage_token(
            ProjectionStageTokenRow::new(
                "reference",
                Eye::Left,
                ProjectionStageKind::ScreenToCamera,
            )
            .with_rows(rows()),
        )
        .with_footprint(
            ProjectionFootprintSummary::new("reference", "raw")
                .with_active_fraction(0.82)
                .with_bbox_fraction([0.1, 0.1, 0.9, 0.9])
                .with_row_span(ProjectionFootprintRowSpan::new(0.5, 0.8).with_span(0.1, 0.9))
                .with_explicit_valid_mask(true),
        );
        let gl_lane = ProjectionMatrixLaneReport::new(
            "gl_oes",
            "OpenXR GLES OES lane",
            ProjectionMatrixLaneKind::OpenXrOpenGlSurfaceTextureOes,
        )
        .with_source_feed(MatrixStepStatus::Passed)
        .with_decoded_texture(MatrixStepStatus::Passed)
        .with_projection_stage(MatrixStepStatus::Blocked)
        .with_blocker("projection stage rows not emitted yet");
        let packet = ProjectionPerformanceMatrixPacket::new("packet-001", source)
            .with_lane(reference)
            .with_lane(gl_lane);

        assert!(packet.is_valid());
        assert_eq!(
            packet
                .first_blocked_lane()
                .and_then(ProjectionMatrixLaneReport::first_blocking_step),
            Some("projection_stage")
        );
    }
}
