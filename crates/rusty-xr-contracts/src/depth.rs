use crate::{CameraExtrinsics, ImageSize};

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

/// Metadata for one depth frame.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepthFrameDescriptor {
    pub frame_index: u64,
    pub timestamp_ns: Option<u64>,
    pub format: DepthFormat,
    pub meter_scale: f32,
    pub depth_payload: DepthPayloadDescriptor,
    pub confidence_format: ConfidenceFormat,
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
            format,
            meter_scale,
            depth_payload,
            confidence_format: ConfidenceFormat::None,
            confidence_payload: None,
            extrinsics: None,
        }
    }

    pub fn with_timestamp_ns(mut self, timestamp_ns: u64) -> Self {
        self.timestamp_ns = Some(timestamp_ns);
        self
    }

    pub fn with_confidence_payload(
        mut self,
        confidence_format: ConfidenceFormat,
        confidence_payload: DepthPayloadDescriptor,
    ) -> Self {
        self.confidence_format = confidence_format;
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
            && self.depth_payload.is_valid()
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
}
