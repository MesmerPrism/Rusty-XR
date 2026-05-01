use crate::{CameraExtrinsics, Eye, FieldOfView, ImageSize, Pose};

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
}
