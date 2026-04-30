use crate::{Pose, Vec2};

/// Pixel dimensions for a frame, view, or payload.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImageSize {
    pub width: u32,
    pub height: u32,
}

impl ImageSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub const fn is_non_empty(self) -> bool {
        self.width > 0 && self.height > 0
    }
}

/// Stable public identifier for a logical camera source.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CameraSourceId {
    pub label: String,
    pub physical_id: Option<String>,
}

impl CameraSourceId {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            physical_id: None,
        }
    }

    pub fn with_physical_id(mut self, physical_id: impl Into<String>) -> Self {
        self.physical_id = Some(physical_id.into());
        self
    }
}

/// Named pixel domain for camera metadata.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraPixelDomainKind {
    #[default]
    DeliveredImage,
    ActiveArray,
    SensorPixelArray,
    Other,
}

/// Pixel domain for a camera frame, active array, or sensor array.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CameraPixelDomain {
    pub kind: CameraPixelDomainKind,
    pub size: ImageSize,
}

impl CameraPixelDomain {
    pub const fn new(kind: CameraPixelDomainKind, size: ImageSize) -> Self {
        Self { kind, size }
    }

    pub const fn delivered_image(size: ImageSize) -> Self {
        Self::new(CameraPixelDomainKind::DeliveredImage, size)
    }

    pub const fn active_array(size: ImageSize) -> Self {
        Self::new(CameraPixelDomainKind::ActiveArray, size)
    }

    pub const fn sensor_pixel_array(size: ImageSize) -> Self {
        Self::new(CameraPixelDomainKind::SensorPixelArray, size)
    }

    pub const fn is_valid(self) -> bool {
        self.size.is_non_empty()
    }
}

/// Pinhole camera intrinsics in the pixel domain described by the frame metadata.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraIntrinsics {
    pub focal_length_px: Vec2,
    pub principal_point_px: Vec2,
    pub skew_px: f32,
    pub image_size: ImageSize,
}

impl CameraIntrinsics {
    pub const fn new(
        focal_length_px: Vec2,
        principal_point_px: Vec2,
        image_size: ImageSize,
    ) -> Self {
        Self {
            focal_length_px,
            principal_point_px,
            skew_px: 0.0,
            image_size,
        }
    }

    pub const fn with_skew_px(mut self, skew_px: f32) -> Self {
        self.skew_px = skew_px;
        self
    }

    pub fn is_valid(self) -> bool {
        self.image_size.is_non_empty()
            && self.focal_length_px.is_finite()
            && self.principal_point_px.is_finite()
            && self.skew_px.is_finite()
            && self.focal_length_px.x > 0.0
            && self.focal_length_px.y > 0.0
    }
}

/// Camera pose relative to a named coordinate frame.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CameraExtrinsics {
    pub world_from_camera: Pose,
}

impl CameraExtrinsics {
    pub const fn new(world_from_camera: Pose) -> Self {
        Self { world_from_camera }
    }

    pub fn is_valid(self) -> bool {
        self.world_from_camera.is_finite()
    }
}

/// Explicit availability flags for metadata needed by camera projection.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CameraFrameMetadataFlags {
    pub missing_intrinsics: bool,
    pub missing_pose: bool,
}

impl CameraFrameMetadataFlags {
    pub const fn new(missing_intrinsics: bool, missing_pose: bool) -> Self {
        Self {
            missing_intrinsics,
            missing_pose,
        }
    }
}

/// Explicit camera-composite path tier for app shells and diagnostics.
///
/// This is a public routing label, not a platform implementation. The GPU
/// buffer probe is useful for validating Camera2/HardwareBuffer availability,
/// but it is not an aligned projection renderer.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraCompositeTier {
    /// Synthetic OpenXR/Vulkan smoke test with no camera source.
    Synthetic,
    /// Camera-source enumeration and capability diagnostics; not a visible
    /// camera-composite path.
    SourceDiagnostics,
    /// CPU YUV/RGBA bring-up path. It is diagnostic and not aligned.
    #[default]
    CpuDiagnosticFlatCopy,
    /// Camera2/HardwareBuffer import probe. It may sample GPU buffers but does
    /// not claim metadata-backed camera/view alignment.
    GpuBufferProbe,
    /// GPU-imported camera buffers with metadata-backed per-eye projection.
    GpuProjected,
}

impl CameraCompositeTier {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Synthetic => "synthetic",
            Self::SourceDiagnostics => "camera-source-diagnostics",
            Self::CpuDiagnosticFlatCopy => "cpu-diagnostic-flat-copy",
            Self::GpuBufferProbe => "gpu-buffer-probe",
            Self::GpuProjected => "gpu-projected",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "synthetic" | "synthetic-openxr-vulkan" => Some(Self::Synthetic),
            "camera-source-diagnostics" | "source-diagnostics" => Some(Self::SourceDiagnostics),
            "cpu-diagnostic-flat-copy" | "diagnostic-flat-camera-copy" | "cpu-yuv" => {
                Some(Self::CpuDiagnosticFlatCopy)
            }
            "gpu-buffer-probe" | "camera-gpu-buffer-probe" | "gpu-projected-probe" => {
                Some(Self::GpuBufferProbe)
            }
            "gpu-projected" | "camera-stereo-gpu-composite" | "external-gpu" => {
                Some(Self::GpuProjected)
            }
            _ => None,
        }
    }
}

/// Generic description of a camera buffer that may be importable by a GPU API.
///
/// Platform adapters can fill this from Android `AHardwareBuffer`, EGL images,
/// DMA-BUF, or another shareable camera-buffer mechanism. The core contract
/// only records diagnostics and cache keys; it does not own native handles.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CameraGpuBufferDescriptor {
    pub source_label: String,
    pub size: ImageSize,
    pub format_label: String,
    pub native_format: Option<u64>,
    pub usage_flags: Option<u64>,
    pub layer_count: Option<u32>,
    pub stride_px: Option<u32>,
    pub buffer_id: Option<u64>,
}

impl CameraGpuBufferDescriptor {
    pub fn new(
        source_label: impl Into<String>,
        size: ImageSize,
        format_label: impl Into<String>,
    ) -> Self {
        Self {
            source_label: source_label.into(),
            size,
            format_label: format_label.into(),
            native_format: None,
            usage_flags: None,
            layer_count: None,
            stride_px: None,
            buffer_id: None,
        }
    }

    pub const fn with_native_format(mut self, native_format: u64) -> Self {
        self.native_format = Some(native_format);
        self
    }

    pub const fn with_usage_flags(mut self, usage_flags: u64) -> Self {
        self.usage_flags = Some(usage_flags);
        self
    }

    pub const fn with_layer_count(mut self, layer_count: u32) -> Self {
        self.layer_count = Some(layer_count);
        self
    }

    pub const fn with_stride_px(mut self, stride_px: u32) -> Self {
        self.stride_px = Some(stride_px);
        self
    }

    pub const fn with_buffer_id(mut self, buffer_id: u64) -> Self {
        self.buffer_id = Some(buffer_id);
        self
    }

    pub fn is_valid(&self) -> bool {
        self.size.is_non_empty()
            && !self.source_label.trim().is_empty()
            && !self.format_label.trim().is_empty()
            && self.layer_count.map(|value| value > 0).unwrap_or(true)
            && self.stride_px.map(|value| value > 0).unwrap_or(true)
    }
}

/// Quarter-turn texture rotation applied to sampled camera images.
///
/// This is separate from Camera2 `SENSOR_ORIENTATION`: opaque GPU camera
/// textures can arrive with an implementation-defined texture orientation even
/// when sensor orientation metadata is zero.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraImageRotation {
    #[default]
    Rotate0,
    Rotate90,
    Rotate180,
    Rotate270,
}

impl CameraImageRotation {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Rotate0 => "rotate0",
            Self::Rotate90 => "rotate90",
            Self::Rotate180 => "rotate180",
            Self::Rotate270 => "rotate270",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "0" | "rotate0" | "none" => Some(Self::Rotate0),
            "90" | "rotate90" => Some(Self::Rotate90),
            "180" | "rotate180" => Some(Self::Rotate180),
            "270" | "rotate270" => Some(Self::Rotate270),
            _ => None,
        }
    }

    pub const fn shader_bits(self) -> u32 {
        match self {
            Self::Rotate0 => 0,
            Self::Rotate90 => 1,
            Self::Rotate180 => 2,
            Self::Rotate270 => 3,
        }
    }
}

/// Explicit post-projection camera texture transform.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraTextureTransform {
    pub rotation: CameraImageRotation,
    pub flip_x: bool,
    pub flip_y: bool,
    pub mirror: bool,
    pub source_label: String,
    pub reason: String,
}

impl CameraTextureTransform {
    pub fn new(source_label: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            rotation: CameraImageRotation::Rotate0,
            flip_x: false,
            flip_y: false,
            mirror: false,
            source_label: source_label.into(),
            reason: reason.into(),
        }
    }

    pub const fn with_rotation(mut self, rotation: CameraImageRotation) -> Self {
        self.rotation = rotation;
        self
    }

    pub const fn with_flip_x(mut self, flip_x: bool) -> Self {
        self.flip_x = flip_x;
        self
    }

    pub const fn with_flip_y(mut self, flip_y: bool) -> Self {
        self.flip_y = flip_y;
        self
    }

    pub const fn with_mirror(mut self, mirror: bool) -> Self {
        self.mirror = mirror;
        self
    }

    pub fn is_explicit_visual_check(&self) -> bool {
        !self.source_label.trim().is_empty()
            && !self.reason.trim().is_empty()
            && self.source_label != "default"
            && self.reason != "unspecified"
    }

    pub fn shader_flags(&self) -> u32 {
        self.rotation.shader_bits()
            | ((self.flip_x as u32) << 2)
            | ((self.flip_y as u32) << 3)
            | ((self.mirror as u32) << 4)
    }

    pub fn label(&self) -> String {
        let mut parts = vec![self.rotation.stable_id().to_string()];
        if self.flip_x {
            parts.push("flipX".to_string());
        }
        if self.flip_y {
            parts.push("flipY".to_string());
        }
        if self.mirror {
            parts.push("mirror".to_string());
        }
        parts.join("+")
    }

    pub fn apply_uv(&self, uv: Vec2) -> Vec2 {
        let mut result = match self.rotation {
            CameraImageRotation::Rotate0 => uv,
            CameraImageRotation::Rotate90 => Vec2::new(uv.y, 1.0 - uv.x),
            CameraImageRotation::Rotate180 => Vec2::new(1.0 - uv.x, 1.0 - uv.y),
            CameraImageRotation::Rotate270 => Vec2::new(1.0 - uv.y, uv.x),
        };
        if self.flip_x || self.mirror {
            result.x = 1.0 - result.x;
        }
        if self.flip_y {
            result.y = 1.0 - result.y;
        }
        result
    }
}

impl Default for CameraTextureTransform {
    fn default() -> Self {
        Self::new("default", "unspecified")
    }
}

/// Public diagnostics for deciding whether a camera frame can be projected.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraProjectionStatus {
    pub requested_tier: CameraCompositeTier,
    pub active_tier: CameraCompositeTier,
    pub gpu_import_available: bool,
    pub intrinsics_available: bool,
    pub pose_available: bool,
    pub fallback_reason: Option<String>,
}

impl CameraProjectionStatus {
    pub fn active(requested_tier: CameraCompositeTier) -> Self {
        Self {
            requested_tier,
            active_tier: requested_tier,
            gpu_import_available: matches!(
                requested_tier,
                CameraCompositeTier::GpuBufferProbe | CameraCompositeTier::GpuProjected
            ),
            intrinsics_available: true,
            pose_available: true,
            fallback_reason: None,
        }
    }

    pub fn fallback(
        requested_tier: CameraCompositeTier,
        active_tier: CameraCompositeTier,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            requested_tier,
            active_tier,
            gpu_import_available: false,
            intrinsics_available: false,
            pose_available: false,
            fallback_reason: Some(reason.into()),
        }
    }

    pub fn is_aligned_projection(&self) -> bool {
        self.requested_tier == CameraCompositeTier::GpuProjected
            && self.active_tier == CameraCompositeTier::GpuProjected
            && self.gpu_import_available
            && self.intrinsics_available
            && self.pose_available
            && self.fallback_reason.is_none()
    }
}

/// Source of camera pose/extrinsics used by a projection path.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraPoseSource {
    /// Runtime/platform metadata supplied the pose.
    Platform,
    /// User supplied a public calibration profile.
    EstimatedProfile,
    /// Pose is unavailable; aligned projection must not be claimed.
    #[default]
    Missing,
}

impl CameraPoseSource {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Platform => "platform",
            Self::EstimatedProfile => "estimated-profile",
            Self::Missing => "missing",
        }
    }
}

/// Public user-supplied stereo calibration shape.
///
/// This records generic camera pose/intrinsics overrides and coordinate
/// convention. It intentionally carries no device-private defaults.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct StereoCameraCalibrationProfile {
    pub label: String,
    pub version: String,
    pub source_label: String,
    pub coordinate_convention: String,
    pub pose_source: CameraPoseSource,
    pub left_extrinsics: CameraExtrinsics,
    pub right_extrinsics: CameraExtrinsics,
    pub left_intrinsics: Option<CameraIntrinsics>,
    pub right_intrinsics: Option<CameraIntrinsics>,
    pub delivered_domain: CameraPixelDomain,
    pub sensor_orientation_degrees: Option<i32>,
}

impl StereoCameraCalibrationProfile {
    pub fn is_valid(&self) -> bool {
        let orientation_valid = self
            .sensor_orientation_degrees
            .map(|degrees| (0..360).contains(&degrees))
            .unwrap_or(true);
        !self.label.trim().is_empty()
            && !self.version.trim().is_empty()
            && !self.source_label.trim().is_empty()
            && !self.coordinate_convention.trim().is_empty()
            && self.pose_source != CameraPoseSource::Missing
            && self.left_extrinsics.is_valid()
            && self.right_extrinsics.is_valid()
            && self
                .left_intrinsics
                .map(CameraIntrinsics::is_valid)
                .unwrap_or(true)
            && self
                .right_intrinsics
                .map(CameraIntrinsics::is_valid)
                .unwrap_or(true)
            && self.delivered_domain.is_valid()
            && orientation_valid
    }
}

/// Public summary of one Camera2-like source discovered by a platform adapter.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CameraSourceDiagnostic {
    pub camera_id: String,
    pub physical_camera_ids: Vec<String>,
    pub logical_multi_camera: bool,
    pub concurrent_camera: bool,
    pub lens_facing: Option<String>,
    pub hardware_level: Option<String>,
    pub sensor_orientation_degrees: Option<i32>,
    pub active_array_size: Option<ImageSize>,
    pub sensor_pixel_array_size: Option<ImageSize>,
    pub private_output_sizes: Vec<ImageSize>,
    pub yuv_output_sizes: Vec<ImageSize>,
    pub fps_ranges: Vec<(i32, i32)>,
    pub intrinsics_available: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub intrinsic_calibration: Option<[f32; 5]>,
    pub distortion_available: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub distortion: Vec<f32>,
    pub lens_pose_available: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub lens_pose_translation: Option<[f32; 3]>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub lens_pose_rotation: Option<[f32; 4]>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub lens_pose_reference: Option<i32>,
}

/// Public diagnostic for accepting or rejecting a stereo camera candidate.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StereoCameraCandidateDiagnostic {
    pub provider_kind: String,
    pub left_camera_id: Option<String>,
    pub right_camera_id: Option<String>,
    pub accepted: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub score: Option<i64>,
    pub reason: String,
}

/// Full source-enumeration diagnostic payload emitted by the public Quest
/// example and saved by companion verification.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CameraSourceDiagnosticsReport {
    pub schema_version: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub requested_tier: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub selected_provider: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub fallback_reason: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub selected_stereo_pair_score: Option<i64>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub selected_stereo_pair_reason: Option<String>,
    pub sources: Vec<CameraSourceDiagnostic>,
    pub stereo_candidates: Vec<StereoCameraCandidateDiagnostic>,
}

/// Metadata for one camera frame without owning the image payload.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct CameraFrameMetadata {
    pub source: CameraSourceId,
    pub frame_index: u64,
    pub delivered_size: ImageSize,
    pub timestamp_ns: Option<u64>,
    pub sensor_orientation_degrees: Option<i32>,
    pub intrinsics: Option<CameraIntrinsics>,
    pub intrinsics_domain: Option<CameraPixelDomain>,
    pub sensor_pixel_domain: Option<CameraPixelDomain>,
    pub extrinsics: Option<CameraExtrinsics>,
    pub flags: CameraFrameMetadataFlags,
}

impl CameraFrameMetadata {
    pub fn new(source: CameraSourceId, frame_index: u64, intrinsics: CameraIntrinsics) -> Self {
        let delivered_size = intrinsics.image_size;
        Self {
            source,
            frame_index,
            delivered_size,
            timestamp_ns: None,
            sensor_orientation_degrees: None,
            intrinsics: Some(intrinsics),
            intrinsics_domain: Some(CameraPixelDomain::delivered_image(delivered_size)),
            sensor_pixel_domain: None,
            extrinsics: None,
            flags: CameraFrameMetadataFlags::new(false, true),
        }
    }

    pub fn without_intrinsics(
        source: CameraSourceId,
        frame_index: u64,
        delivered_size: ImageSize,
    ) -> Self {
        Self {
            source,
            frame_index,
            delivered_size,
            timestamp_ns: None,
            sensor_orientation_degrees: None,
            intrinsics: None,
            intrinsics_domain: None,
            sensor_pixel_domain: None,
            extrinsics: None,
            flags: CameraFrameMetadataFlags::new(true, true),
        }
    }

    pub fn with_timestamp_ns(mut self, timestamp_ns: u64) -> Self {
        self.timestamp_ns = Some(timestamp_ns);
        self
    }

    pub fn with_sensor_orientation_degrees(mut self, degrees: i32) -> Self {
        self.sensor_orientation_degrees = Some(degrees);
        self
    }

    pub fn with_intrinsics_domain(mut self, domain: CameraPixelDomain) -> Self {
        self.intrinsics_domain = Some(domain);
        self
    }

    pub fn with_sensor_pixel_domain(mut self, domain: CameraPixelDomain) -> Self {
        self.sensor_pixel_domain = Some(domain);
        self
    }

    pub fn with_extrinsics(mut self, extrinsics: CameraExtrinsics) -> Self {
        self.extrinsics = Some(extrinsics);
        self.flags.missing_pose = false;
        self
    }

    pub fn has_intrinsics(&self) -> bool {
        self.intrinsics.is_some() && !self.flags.missing_intrinsics
    }

    pub fn has_pose(&self) -> bool {
        self.extrinsics.is_some() && !self.flags.missing_pose
    }

    pub fn has_projection_metadata(&self) -> bool {
        self.has_intrinsics() && self.has_pose()
    }

    pub fn is_valid(&self) -> bool {
        let intrinsics_valid = match self.intrinsics {
            Some(intrinsics) => intrinsics.is_valid() && !self.flags.missing_intrinsics,
            None => self.flags.missing_intrinsics,
        };
        let pose_valid = match self.extrinsics {
            Some(extrinsics) => extrinsics.is_valid() && !self.flags.missing_pose,
            None => self.flags.missing_pose,
        };
        let orientation_valid = self
            .sensor_orientation_degrees
            .map(|degrees| (0..360).contains(&degrees))
            .unwrap_or(true);

        self.delivered_size.is_non_empty()
            && intrinsics_valid
            && pose_valid
            && orientation_valid
            && self
                .intrinsics_domain
                .map(CameraPixelDomain::is_valid)
                .unwrap_or(true)
            && self
                .sensor_pixel_domain
                .map(CameraPixelDomain::is_valid)
                .unwrap_or(true)
    }
}

/// Metadata for synchronized or near-synchronized stereo camera frames.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct StereoCameraFrameMetadata {
    pub frame_index: u64,
    pub left: CameraFrameMetadata,
    pub right: CameraFrameMetadata,
    pub midpoint_timestamp_ns: Option<u64>,
}

impl StereoCameraFrameMetadata {
    pub fn new(frame_index: u64, left: CameraFrameMetadata, right: CameraFrameMetadata) -> Self {
        Self {
            frame_index,
            left,
            right,
            midpoint_timestamp_ns: None,
        }
    }

    pub fn with_midpoint_timestamp_ns(mut self, midpoint_timestamp_ns: u64) -> Self {
        self.midpoint_timestamp_ns = Some(midpoint_timestamp_ns);
        self
    }

    pub fn is_valid(&self) -> bool {
        self.left.is_valid() && self.right.is_valid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_intrinsics_require_positive_focal_length() {
        let valid = CameraIntrinsics::new(
            Vec2::new(500.0, 510.0),
            Vec2::new(320.0, 240.0),
            ImageSize::new(640, 480),
        );
        let invalid = CameraIntrinsics::new(
            Vec2::new(0.0, 510.0),
            Vec2::new(320.0, 240.0),
            ImageSize::new(640, 480),
        );

        assert!(valid.is_valid());
        assert!(!invalid.is_valid());
    }

    #[test]
    fn camera_metadata_can_explicitly_report_missing_projection_inputs() {
        let metadata = CameraFrameMetadata::without_intrinsics(
            CameraSourceId::new("camera2-mono"),
            7,
            ImageSize::new(1280, 1280),
        )
        .with_timestamp_ns(123)
        .with_sensor_orientation_degrees(90);

        assert!(metadata.is_valid());
        assert!(metadata.flags.missing_intrinsics);
        assert!(metadata.flags.missing_pose);
        assert!(!metadata.has_projection_metadata());
    }

    #[test]
    fn camera_composite_tier_parses_public_profile_names() {
        assert_eq!(
            CameraCompositeTier::parse("synthetic"),
            Some(CameraCompositeTier::Synthetic)
        );
        assert_eq!(
            CameraCompositeTier::parse("camera-source-diagnostics"),
            Some(CameraCompositeTier::SourceDiagnostics)
        );
        assert_eq!(
            CameraCompositeTier::parse("diagnostic-flat-camera-copy"),
            Some(CameraCompositeTier::CpuDiagnosticFlatCopy)
        );
        assert_eq!(
            CameraCompositeTier::parse("camera-stereo-gpu-composite"),
            Some(CameraCompositeTier::GpuProjected)
        );
        assert_eq!(
            CameraCompositeTier::parse("camera-gpu-buffer-probe"),
            Some(CameraCompositeTier::GpuBufferProbe)
        );
        assert_eq!(CameraCompositeTier::parse("private-profile"), None);
    }

    #[test]
    fn gpu_buffer_descriptor_requires_public_shape() {
        let descriptor = CameraGpuBufferDescriptor::new(
            "Camera2 PRIVATE",
            ImageSize::new(1280, 1280),
            "AHardwareBuffer",
        )
        .with_native_format(35)
        .with_usage_flags(0x100)
        .with_layer_count(1)
        .with_stride_px(1280)
        .with_buffer_id(7);

        assert!(descriptor.is_valid());
        assert!(!CameraGpuBufferDescriptor::default().is_valid());
    }

    #[test]
    fn camera_texture_transform_rotates_and_flips_uv() {
        let transform = CameraTextureTransform::new("public-live-check", "upright camera texture")
            .with_rotation(CameraImageRotation::Rotate180)
            .with_flip_x(true);

        assert_eq!(
            CameraImageRotation::parse("rotate180"),
            Some(CameraImageRotation::Rotate180)
        );
        assert_eq!(transform.shader_flags(), 0b0110);
        assert_eq!(transform.label(), "rotate180+flipX");
        let uv = transform.apply_uv(Vec2::new(0.2, 0.3));
        assert!((uv.x - 0.2).abs() < 1.0e-6);
        assert!((uv.y - 0.7).abs() < 1.0e-6);
        assert!(transform.is_explicit_visual_check());
        assert!(!CameraTextureTransform::default().is_explicit_visual_check());
    }

    #[test]
    fn projection_status_does_not_claim_alignment_when_fallback_is_active() {
        let active = CameraProjectionStatus::active(CameraCompositeTier::GpuProjected);
        let fallback = CameraProjectionStatus::fallback(
            CameraCompositeTier::GpuProjected,
            CameraCompositeTier::CpuDiagnosticFlatCopy,
            "missing camera pose",
        );

        assert!(active.is_aligned_projection());
        assert!(!fallback.is_aligned_projection());
    }

    #[test]
    fn gpu_buffer_probe_never_claims_aligned_projection() {
        let status = CameraProjectionStatus::active(CameraCompositeTier::GpuBufferProbe);

        assert!(status.gpu_import_available);
        assert!(!status.is_aligned_projection());
    }

    #[test]
    fn calibration_profile_requires_non_missing_pose_source() {
        let profile = StereoCameraCalibrationProfile {
            label: "user supplied".to_string(),
            version: "v1".to_string(),
            source_label: "test-profile".to_string(),
            coordinate_convention: "right-handed head space".to_string(),
            pose_source: CameraPoseSource::EstimatedProfile,
            left_extrinsics: CameraExtrinsics::new(Pose::IDENTITY),
            right_extrinsics: CameraExtrinsics::new(Pose::IDENTITY),
            left_intrinsics: None,
            right_intrinsics: None,
            delivered_domain: CameraPixelDomain::delivered_image(ImageSize::new(1280, 1280)),
            sensor_orientation_degrees: Some(0),
        };
        let mut missing_pose = profile.clone();
        missing_pose.pose_source = CameraPoseSource::Missing;

        assert!(profile.is_valid());
        assert!(!missing_pose.is_valid());
        assert_eq!(
            CameraPoseSource::EstimatedProfile.stable_id(),
            "estimated-profile"
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn camera_source_diagnostics_round_trip_with_serde() {
        let report = CameraSourceDiagnosticsReport {
            schema_version: "rusty.xr.camera-source-diagnostics.v1".to_string(),
            requested_tier: Some("camera-source-diagnostics".to_string()),
            selected_provider: Some("logical-physical".to_string()),
            fallback_reason: Some("selected logical-physical 0a/0b".to_string()),
            selected_stereo_pair_score: Some(42),
            selected_stereo_pair_reason: Some("selected logical-physical 0a/0b".to_string()),
            sources: vec![CameraSourceDiagnostic {
                camera_id: "0".to_string(),
                physical_camera_ids: vec!["0a".to_string(), "0b".to_string()],
                logical_multi_camera: true,
                concurrent_camera: false,
                lens_facing: Some("back".to_string()),
                hardware_level: Some("limited".to_string()),
                sensor_orientation_degrees: Some(0),
                active_array_size: Some(ImageSize::new(1280, 1280)),
                sensor_pixel_array_size: Some(ImageSize::new(1280, 1280)),
                private_output_sizes: vec![ImageSize::new(1280, 1280)],
                yuv_output_sizes: vec![ImageSize::new(640, 480)],
                fps_ranges: vec![(30, 60)],
                intrinsics_available: true,
                intrinsic_calibration: Some([500.0, 510.0, 320.0, 240.0, 0.0]),
                distortion_available: true,
                distortion: vec![0.01, -0.02, 0.0, 0.0, 0.0],
                lens_pose_available: false,
                lens_pose_translation: Some([0.03, 0.0, 0.0]),
                lens_pose_rotation: Some([0.0, 0.0, 0.0, 1.0]),
                lens_pose_reference: Some(1),
            }],
            stereo_candidates: vec![StereoCameraCandidateDiagnostic {
                provider_kind: "logical-physical".to_string(),
                left_camera_id: Some("0a".to_string()),
                right_camera_id: Some("0b".to_string()),
                accepted: true,
                score: Some(42),
                reason: "two physical cameras expose PRIVATE output through a logical camera"
                    .to_string(),
            }],
        };

        let encoded = serde_json::to_string(&report).expect("diagnostics should serialize");
        let decoded: CameraSourceDiagnosticsReport =
            serde_json::from_str(&encoded).expect("diagnostics should deserialize");

        assert_eq!(decoded, report);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn camera_metadata_round_trips_with_serde() {
        let intrinsics = CameraIntrinsics::new(
            Vec2::new(500.0, 510.0),
            Vec2::new(320.0, 240.0),
            ImageSize::new(640, 480),
        );
        let metadata = CameraFrameMetadata::new(CameraSourceId::new("left"), 42, intrinsics)
            .with_timestamp_ns(123);

        let encoded = serde_json::to_string(&metadata).expect("metadata should serialize");
        let decoded: CameraFrameMetadata =
            serde_json::from_str(&encoded).expect("metadata should deserialize");

        assert_eq!(decoded, metadata);
    }
}
