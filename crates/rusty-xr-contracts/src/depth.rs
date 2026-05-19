use crate::{CameraExtrinsics, Eye, FieldOfView, ImageSize, Pose, Vec2, Vec3};

/// Versioned schema id for depth/world-space contract packets.
pub const DEPTH_WORLD_SPACE_CONTRACT_SCHEMA: &str = "rusty.xr.depth_world_space_contract.v1";

/// Public depth payload interpretation.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DepthFormat {
    Float32Meters,
    Uint16Millimeters,
    Uint16Raw,
}

/// Optional confidence payload interpretation.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfidenceFormat {
    None,
    Uint8,
    Float32,
}

/// Where a confidence signal for a depth frame came from.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DepthConfidenceSource {
    #[default]
    None,
    RuntimePayload,
    AppDerived,
    Unknown,
}

/// App-visible source behind a depth/world-space contract.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DepthWorldSpaceSourceKind {
    /// Runtime-generated environment depth, for example `XR_META_environment_depth`.
    #[default]
    RuntimeEnvironmentDepth,
    /// Synthetic depth fixture used for deterministic tests.
    Synthetic,
    /// Imported public fixture or offline capture summary.
    Imported,
    /// Another source summarized by a downstream adapter.
    Other,
}

/// Renderer path that consumes reconstructed depth/world-space points.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DepthWorldSpaceRenderPath {
    /// Current depth image shown as a per-eye texture diagnostic.
    #[default]
    FullscreenDepthVisualizer,
    /// Generated grid vertices are reconstructed from depth samples.
    GeneratedDepthMesh,
    /// Depth samples update retained reference-space billboard particles.
    RetainedMetricParticles,
    /// Depth samples update a scene-owned reference-space particle map.
    SceneParticleMap,
    /// Another renderer path summarized by a downstream adapter.
    Other,
}

/// Identity policy for samples that survive beyond one draw call.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DepthSampleIdentityPolicy {
    /// Samples are identified by the current depth raster or mesh vertex slot.
    #[default]
    DepthRasterSlot,
    /// Samples are retained in a ring buffer after reconstruction.
    RetainedReferencePoint,
    /// Samples are merged by quantized app-reference-space cells.
    ReferenceSpaceCell,
    /// The path does not retain individual world-space samples.
    NotRetained,
}

/// Named stage in the depth UV to render-eye chain.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DepthWorldSpaceStageKind {
    #[default]
    DepthUvToDepthViewRay,
    DepthViewRayToMetricPoint,
    DepthViewPointToReferenceSpace,
    ReferenceSpacePointToRenderEye,
    RenderEyePointToScreen,
}

/// Public-safe evidence row for one stage in a depth/world-space run.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DepthWorldSpaceStageEvidence {
    pub stage: DepthWorldSpaceStageKind,
    pub owner: String,
    pub evidence: String,
}

impl DepthWorldSpaceStageEvidence {
    pub fn new(
        stage: DepthWorldSpaceStageKind,
        owner: impl Into<String>,
        evidence: impl Into<String>,
    ) -> Self {
        Self {
            stage,
            owner: owner.into(),
            evidence: evidence.into(),
        }
    }

    pub fn is_valid(&self) -> bool {
        non_empty(&self.owner) && non_empty(&self.evidence)
    }
}

/// Runtime-supplied near/far range used to interpret normalized depth samples.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepthMetricRange {
    pub near_z_m: f32,
    pub far_z_m: f32,
}

impl DepthMetricRange {
    pub const fn new(near_z_m: f32, far_z_m: f32) -> Self {
        Self { near_z_m, far_z_m }
    }

    pub fn is_valid(self) -> bool {
        let finite_far = self.far_z_m.is_finite() && self.far_z_m > self.near_z_m;
        let infinite_far = self.far_z_m.is_infinite() && self.far_z_m.is_sign_positive();
        self.near_z_m.is_finite() && self.near_z_m > 0.0 && (finite_far || infinite_far)
    }

    pub fn has_infinite_far(self) -> bool {
        self.far_z_m.is_infinite() && self.far_z_m.is_sign_positive()
    }
}

/// JSON-safe near/far range for depth/world-space contract artifacts.
///
/// `DepthMetricRange` can represent an infinite far plane as `f32::INFINITY`,
/// which is useful in Rust but not valid JSON. This shape keeps that runtime
/// fact explicit for serialized contract artifacts.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepthWorldSpaceMetricRange {
    pub near_z_m: f32,
    pub far_z_m: Option<f32>,
    pub far_z_infinite: bool,
}

impl DepthWorldSpaceMetricRange {
    pub const fn new(near_z_m: f32, far_z_m: Option<f32>, far_z_infinite: bool) -> Self {
        Self {
            near_z_m,
            far_z_m,
            far_z_infinite,
        }
    }

    pub fn is_valid(self) -> bool {
        self.near_z_m.is_finite()
            && self.near_z_m > 0.0
            && if self.far_z_infinite {
                self.far_z_m.is_none()
            } else {
                self.far_z_m
                    .map(|far_z_m| far_z_m.is_finite() && far_z_m > self.near_z_m)
                    .unwrap_or(false)
            }
    }
}

impl From<DepthMetricRange> for DepthWorldSpaceMetricRange {
    fn from(value: DepthMetricRange) -> Self {
        if value.has_infinite_far() {
            Self::new(value.near_z_m, None, true)
        } else {
            Self::new(value.near_z_m, Some(value.far_z_m), false)
        }
    }
}

/// Describes a depth or confidence payload without owning its bytes.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepthPayloadDescriptor {
    pub size: ImageSize,
    pub byte_len: usize,
    pub row_stride_bytes: Option<usize>,
}

impl DepthPayloadDescriptor {
    pub const fn new(size: ImageSize, byte_len: usize) -> Self {
        Self {
            size,
            byte_len,
            row_stride_bytes: None,
        }
    }

    pub const fn with_row_stride_bytes(mut self, row_stride_bytes: usize) -> Self {
        self.row_stride_bytes = Some(row_stride_bytes);
        self
    }

    pub fn is_valid(self) -> bool {
        self.size.is_non_empty() && self.byte_len > 0
    }
}

/// Per-view metadata for a depth image layer.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepthViewDescriptor {
    pub eye: Eye,
    pub pose: Pose,
    pub fov: FieldOfView,
}

impl DepthViewDescriptor {
    pub const fn new(eye: Eye, pose: Pose, fov: FieldOfView) -> Self {
        Self { eye, pose, fov }
    }

    pub fn is_valid(self) -> bool {
        self.pose.is_finite() && self.fov.is_finite()
    }
}

/// Renderer-neutral contract for one environment-depth world-space run.
///
/// This is the depth counterpart to the camera projection-coordinate contract:
/// it records that app-visible depth samples become app reference-space
/// geometry before they are projected by the current render-eye view.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct DepthWorldSpaceContract {
    pub schema: String,
    pub contract_id: String,
    pub source_kind: DepthWorldSpaceSourceKind,
    pub render_path: DepthWorldSpaceRenderPath,
    pub depth_payload: DepthPayloadDescriptor,
    pub depth_format: DepthFormat,
    pub depth_range: DepthWorldSpaceMetricRange,
    pub runtime_capture_time_ns: Option<i64>,
    pub layer_count: u32,
    pub left_depth_view: DepthViewDescriptor,
    pub right_depth_view: DepthViewDescriptor,
    pub reference_space: String,
    pub reference_space_units: String,
    pub depth_uv_origin: String,
    pub depth_texture_transform: String,
    pub linearization: String,
    pub point_reconstruction: String,
    pub render_eye_view_source: String,
    pub projection_y_convention: String,
    pub render_target_size: Option<ImageSize>,
    pub sample_identity_policy: DepthSampleIdentityPolicy,
    pub passthrough_visible: bool,
    pub stages: Vec<DepthWorldSpaceStageEvidence>,
}

impl DepthWorldSpaceContract {
    pub fn environment_depth(
        contract_id: impl Into<String>,
        render_path: DepthWorldSpaceRenderPath,
        depth_payload: DepthPayloadDescriptor,
        depth_range: DepthMetricRange,
        left_depth_view: DepthViewDescriptor,
        right_depth_view: DepthViewDescriptor,
    ) -> Self {
        Self {
            schema: DEPTH_WORLD_SPACE_CONTRACT_SCHEMA.to_string(),
            contract_id: contract_id.into(),
            source_kind: DepthWorldSpaceSourceKind::RuntimeEnvironmentDepth,
            render_path,
            depth_payload,
            depth_format: DepthFormat::Uint16Raw,
            depth_range: depth_range.into(),
            runtime_capture_time_ns: None,
            layer_count: 2,
            left_depth_view,
            right_depth_view,
            reference_space: String::from("app-reference-space"),
            reference_space_units: String::from("meters"),
            depth_uv_origin: String::from("normalized-depth-image"),
            depth_texture_transform: String::from("identity"),
            linearization: String::from("near-far-depth-buffer-to-meters"),
            point_reconstruction: String::from("fov-tangent-ray-depth-view-to-reference-space"),
            render_eye_view_source: String::from("current-openxr-view-pose-fov"),
            projection_y_convention: String::from("renderer-defined"),
            render_target_size: None,
            sample_identity_policy: DepthSampleIdentityPolicy::DepthRasterSlot,
            passthrough_visible: false,
            stages: default_depth_world_space_stages(),
        }
    }

    pub fn with_runtime_capture_time_ns(mut self, runtime_capture_time_ns: i64) -> Self {
        self.runtime_capture_time_ns = Some(runtime_capture_time_ns);
        self
    }

    pub fn with_depth_texture_transform(mut self, transform: impl Into<String>) -> Self {
        self.depth_texture_transform = transform.into();
        self
    }

    pub fn with_reference_space(mut self, reference_space: impl Into<String>) -> Self {
        self.reference_space = reference_space.into();
        self
    }

    pub fn with_projection_y_convention(mut self, convention: impl Into<String>) -> Self {
        self.projection_y_convention = convention.into();
        self
    }

    pub fn with_render_target_size(mut self, size: ImageSize) -> Self {
        self.render_target_size = Some(size);
        self
    }

    pub const fn with_sample_identity_policy(mut self, policy: DepthSampleIdentityPolicy) -> Self {
        self.sample_identity_policy = policy;
        self
    }

    pub const fn with_passthrough_visible(mut self, passthrough_visible: bool) -> Self {
        self.passthrough_visible = passthrough_visible;
        self
    }

    pub fn with_stages(mut self, stages: Vec<DepthWorldSpaceStageEvidence>) -> Self {
        self.stages = stages;
        self
    }

    pub fn is_valid(&self) -> bool {
        self.schema == DEPTH_WORLD_SPACE_CONTRACT_SCHEMA
            && non_empty(&self.contract_id)
            && self.depth_payload.is_valid()
            && self.depth_range.is_valid()
            && self.layer_count >= 2
            && self.left_depth_view.eye == Eye::Left
            && self.right_depth_view.eye == Eye::Right
            && self.left_depth_view.is_valid()
            && self.right_depth_view.is_valid()
            && non_empty(&self.reference_space)
            && non_empty(&self.reference_space_units)
            && non_empty(&self.depth_uv_origin)
            && non_empty(&self.depth_texture_transform)
            && non_empty(&self.linearization)
            && non_empty(&self.point_reconstruction)
            && non_empty(&self.render_eye_view_source)
            && non_empty(&self.projection_y_convention)
            && self
                .render_target_size
                .map(ImageSize::is_non_empty)
                .unwrap_or(true)
            && !self.stages.is_empty()
            && self
                .stages
                .iter()
                .all(DepthWorldSpaceStageEvidence::is_valid)
    }
}

/// Build the OpenXR-style depth-view position for a normalized depth UV.
///
/// The returned vector is in the depth view's local coordinate system, with
/// `-Z` forward, matching the Quest composite example shaders.
pub fn depth_view_position_from_uv(
    depth_view: DepthViewDescriptor,
    depth_uv: Vec2,
    depth_meters: f32,
) -> Option<Vec3> {
    if !depth_view.is_valid()
        || !unit(depth_uv.x)
        || !unit(depth_uv.y)
        || !depth_meters.is_finite()
        || depth_meters <= 0.0
    {
        return None;
    }

    let left = depth_view.fov.angle_left_radians.tan();
    let right = depth_view.fov.angle_right_radians.tan();
    let up = depth_view.fov.angle_up_radians.tan();
    let down = depth_view.fov.angle_down_radians.tan();
    let tangent_x = left + (right - left) * depth_uv.x;
    let tangent_y = up + (down - up) * depth_uv.y;
    Some(Vec3::new(
        tangent_x * depth_meters,
        tangent_y * depth_meters,
        -depth_meters,
    ))
}

/// Reconstruct one app-reference-space point from depth UV and metric depth.
pub fn reference_space_point_from_depth_uv(
    depth_view: DepthViewDescriptor,
    depth_uv: Vec2,
    depth_meters: f32,
) -> Option<Vec3> {
    depth_view_position_from_uv(depth_view, depth_uv, depth_meters)
        .map(|view_position| depth_view.pose.transform_point(view_position))
}

/// Project one app-reference-space point through a render-eye pose/FOV.
///
/// The returned UV is in top-left, y-down display-eye screen space.
pub fn render_eye_screen_uv_from_reference_point(
    render_view: DepthViewDescriptor,
    reference_point: Vec3,
) -> Option<Vec2> {
    if !render_view.is_valid() || !reference_point.is_finite() {
        return None;
    }

    let view_position = render_view.pose.inverse_transform_point(reference_point);
    let forward_z = -view_position.z;
    if !forward_z.is_finite() || forward_z <= 0.001 {
        return None;
    }

    let left = render_view.fov.angle_left_radians.tan();
    let right = render_view.fov.angle_right_radians.tan();
    let up = render_view.fov.angle_up_radians.tan();
    let down = render_view.fov.angle_down_radians.tan();
    let width = right - left;
    let height = up - down;
    if width.abs() <= f32::EPSILON || height.abs() <= f32::EPSILON {
        return None;
    }

    let tangent_x = view_position.x / forward_z;
    let tangent_y = view_position.y / forward_z;
    let u = (tangent_x - left) / width;
    let v_bottom_to_top = (tangent_y - down) / height;
    Some(Vec2::new(u, 1.0 - v_bottom_to_top))
}

/// Metadata for one depth frame.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepthFrameDescriptor {
    pub frame_index: u64,
    pub timestamp_ns: Option<u64>,
    pub runtime_capture_time_ns: Option<i64>,
    pub format: DepthFormat,
    pub meter_scale: f32,
    pub depth_range: Option<DepthMetricRange>,
    pub layer_index: Option<u32>,
    pub layer_count: u32,
    pub view: Option<DepthViewDescriptor>,
    pub depth_payload: DepthPayloadDescriptor,
    pub confidence_format: ConfidenceFormat,
    pub confidence_source: DepthConfidenceSource,
    pub confidence_payload: Option<DepthPayloadDescriptor>,
    pub extrinsics: Option<CameraExtrinsics>,
}

impl DepthFrameDescriptor {
    pub const fn new(
        frame_index: u64,
        format: DepthFormat,
        meter_scale: f32,
        depth_payload: DepthPayloadDescriptor,
    ) -> Self {
        Self {
            frame_index,
            timestamp_ns: None,
            runtime_capture_time_ns: None,
            format,
            meter_scale,
            depth_range: None,
            layer_index: None,
            layer_count: 1,
            view: None,
            depth_payload,
            confidence_format: ConfidenceFormat::None,
            confidence_source: DepthConfidenceSource::None,
            confidence_payload: None,
            extrinsics: None,
        }
    }

    pub fn with_timestamp_ns(mut self, timestamp_ns: u64) -> Self {
        self.timestamp_ns = Some(timestamp_ns);
        self
    }

    pub fn with_runtime_capture_time_ns(mut self, runtime_capture_time_ns: i64) -> Self {
        self.runtime_capture_time_ns = Some(runtime_capture_time_ns);
        self
    }

    pub fn with_depth_range(mut self, depth_range: DepthMetricRange) -> Self {
        self.depth_range = Some(depth_range);
        self
    }

    pub fn with_layer(mut self, layer_index: u32, layer_count: u32) -> Self {
        self.layer_index = Some(layer_index);
        self.layer_count = layer_count;
        self
    }

    pub fn with_view(mut self, view: DepthViewDescriptor) -> Self {
        self.view = Some(view);
        self
    }

    pub fn with_confidence_source(mut self, confidence_source: DepthConfidenceSource) -> Self {
        self.confidence_source = confidence_source;
        self
    }

    pub fn with_confidence_payload(
        mut self,
        confidence_format: ConfidenceFormat,
        confidence_payload: DepthPayloadDescriptor,
    ) -> Self {
        self.confidence_format = confidence_format;
        self.confidence_source = DepthConfidenceSource::RuntimePayload;
        self.confidence_payload = Some(confidence_payload);
        self
    }

    pub fn with_extrinsics(mut self, extrinsics: CameraExtrinsics) -> Self {
        self.extrinsics = Some(extrinsics);
        self
    }

    pub fn is_valid(self) -> bool {
        self.meter_scale.is_finite()
            && self.meter_scale > 0.0
            && self.layer_count > 0
            && self
                .layer_index
                .map(|layer_index| layer_index < self.layer_count)
                .unwrap_or(true)
            && self
                .depth_range
                .map(DepthMetricRange::is_valid)
                .unwrap_or(true)
            && self.depth_payload.is_valid()
            && self.view.map(DepthViewDescriptor::is_valid).unwrap_or(true)
            && self
                .confidence_payload
                .map(DepthPayloadDescriptor::is_valid)
                .unwrap_or(true)
            && self
                .extrinsics
                .map(CameraExtrinsics::is_valid)
                .unwrap_or(true)
    }
}

/// Generic environment-depth lifecycle state.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EnvironmentDepthState {
    pub supported: bool,
    pub permission_granted: bool,
    pub provider_created: bool,
    pub provider_running: bool,
    pub frame_available: bool,
}

impl EnvironmentDepthState {
    pub const fn inactive() -> Self {
        Self {
            supported: false,
            permission_granted: false,
            provider_created: false,
            provider_running: false,
            frame_available: false,
        }
    }

    pub const fn is_active(self) -> bool {
        self.supported && self.permission_granted && self.provider_created && self.provider_running
    }
}

fn default_depth_world_space_stages() -> Vec<DepthWorldSpaceStageEvidence> {
    vec![
        DepthWorldSpaceStageEvidence::new(
            DepthWorldSpaceStageKind::DepthUvToDepthViewRay,
            "runtime-depth-view-fov",
            "depth_uv plus XrEnvironmentDepthImageViewMETA.fov",
        ),
        DepthWorldSpaceStageEvidence::new(
            DepthWorldSpaceStageKind::DepthViewRayToMetricPoint,
            "depth-linearization",
            "near/far depth range converts raw depth to meters",
        ),
        DepthWorldSpaceStageEvidence::new(
            DepthWorldSpaceStageKind::DepthViewPointToReferenceSpace,
            "depth-view-pose",
            "depth view pose composed into app reference space",
        ),
        DepthWorldSpaceStageEvidence::new(
            DepthWorldSpaceStageKind::ReferenceSpacePointToRenderEye,
            "current-render-eye-view",
            "reference point transformed by current render-eye pose",
        ),
        DepthWorldSpaceStageEvidence::new(
            DepthWorldSpaceStageKind::RenderEyePointToScreen,
            "current-render-eye-fov",
            "render-eye FOV projects metric point to submitted eye image",
        ),
    ]
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Quat, Vec3};

    #[test]
    fn environment_depth_active_requires_running_provider() {
        let state = EnvironmentDepthState {
            supported: true,
            permission_granted: true,
            provider_created: true,
            provider_running: false,
            frame_available: true,
        };

        assert!(!state.is_active());
    }

    #[test]
    fn depth_metric_range_allows_infinite_far_plane() {
        let range = DepthMetricRange::new(0.1, f32::INFINITY);

        assert!(range.is_valid());
        assert!(range.has_infinite_far());
        assert!(!DepthMetricRange::new(0.0, f32::INFINITY).is_valid());
        assert!(!DepthMetricRange::new(0.1, f32::NEG_INFINITY).is_valid());
    }

    #[test]
    fn depth_frame_descriptor_accepts_per_eye_view_metadata() {
        let fov = FieldOfView::new(-0.7, 0.7, 0.7, -0.7);
        let view = DepthViewDescriptor::new(Eye::Left, Pose::new(Vec3::ZERO, Quat::IDENTITY), fov);
        let descriptor = DepthFrameDescriptor::new(
            7,
            DepthFormat::Uint16Raw,
            1.0,
            DepthPayloadDescriptor::new(ImageSize::new(320, 320), 320 * 320 * 2),
        )
        .with_layer(0, 2)
        .with_view(view)
        .with_depth_range(DepthMetricRange::new(0.1, f32::INFINITY));

        assert!(descriptor.is_valid());
    }

    #[test]
    fn depth_world_space_contract_records_reference_space_chain() {
        let fov = FieldOfView::new(-0.7, 0.7, 0.7, -0.7);
        let left = DepthViewDescriptor::new(Eye::Left, Pose::IDENTITY, fov);
        let right = DepthViewDescriptor::new(Eye::Right, Pose::IDENTITY, fov);
        let contract = DepthWorldSpaceContract::environment_depth(
            "synthetic-depth-contract",
            DepthWorldSpaceRenderPath::SceneParticleMap,
            DepthPayloadDescriptor::new(ImageSize::new(320, 320), 320 * 320 * 2),
            DepthMetricRange::new(0.1, f32::INFINITY),
            left,
            right,
        )
        .with_sample_identity_policy(DepthSampleIdentityPolicy::ReferenceSpaceCell)
        .with_passthrough_visible(true);

        assert!(contract.is_valid());
    }

    #[test]
    fn depth_uv_reconstructs_and_projects_through_render_eye() {
        let fov = FieldOfView::new(-0.7, 0.7, 0.7, -0.7);
        let view = DepthViewDescriptor::new(Eye::Left, Pose::IDENTITY, fov);
        let reference_point =
            reference_space_point_from_depth_uv(view, Vec2::new(0.5, 0.5), 1.0).unwrap();
        let screen_uv = render_eye_screen_uv_from_reference_point(view, reference_point).unwrap();

        assert!(reference_point.x.abs() < 1.0e-6);
        assert!(reference_point.y.abs() < 1.0e-6);
        assert!((reference_point.z + 1.0).abs() < 1.0e-6);
        assert!((screen_uv.x - 0.5).abs() < 1.0e-6);
        assert!((screen_uv.y - 0.5).abs() < 1.0e-6);
    }
}
