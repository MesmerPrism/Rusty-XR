//! Depth frame and environment-depth contracts for Rusty XR.
//!
//! This crate keeps environment-depth state and payload summaries generic. It
//! models runtime-generated depth textures, not raw low-level sensor feeds. It
//! does not own provider creation, OpenXR extension calls, or app policy.
//!
//! Public adapters should make depth encoding explicit. Raw samples are not
//! assumed to be metric depth unless the descriptor carries a documented
//! conversion, and confidence payloads are optional.
//!
//! Enable the `serde` feature when depth summaries need to be serialized for
//! diagnostics, manifests, or operator tooling.

pub use rusty_xr_contracts::{
    ConfidenceFormat, DepthFormat, DepthFrameDescriptor, DepthPayloadDescriptor,
    EnvironmentDepthState, ImageSize,
};

/// Crate version exposed for lightweight smoke checks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Coarse state classification for diagnostics and UI.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DepthReadiness {
    Unsupported,
    PermissionRequired,
    ProviderNotCreated,
    ProviderStopped,
    WaitingForFrame,
    Ready,
}

impl DepthReadiness {
    pub const fn from_state(state: EnvironmentDepthState) -> Self {
        if !state.supported {
            Self::Unsupported
        } else if !state.permission_granted {
            Self::PermissionRequired
        } else if !state.provider_created {
            Self::ProviderNotCreated
        } else if !state.provider_running {
            Self::ProviderStopped
        } else if !state.frame_available {
            Self::WaitingForFrame
        } else {
            Self::Ready
        }
    }

    pub const fn is_ready(self) -> bool {
        matches!(self, Self::Ready)
    }
}

/// Summary of a depth frame suitable for logs, tests, and telemetry.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DepthFrameSummary {
    pub frame_index: u64,
    pub size: ImageSize,
    pub format: DepthFormat,
    pub meter_scale: f32,
    pub has_confidence: bool,
    pub byte_len: usize,
}

impl DepthFrameSummary {
    pub fn from_descriptor(descriptor: DepthFrameDescriptor) -> Self {
        Self {
            frame_index: descriptor.frame_index,
            size: descriptor.depth_payload.size,
            format: descriptor.format,
            meter_scale: descriptor.meter_scale,
            has_confidence: descriptor.confidence_payload.is_some(),
            byte_len: descriptor.depth_payload.byte_len,
        }
    }

    pub fn is_valid(self) -> bool {
        self.size.is_non_empty()
            && self.byte_len > 0
            && self.meter_scale.is_finite()
            && self.meter_scale > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_workspace_version() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn classifies_environment_depth_state() {
        let state = EnvironmentDepthState {
            supported: true,
            permission_granted: true,
            provider_created: true,
            provider_running: true,
            frame_available: false,
        };

        assert_eq!(
            DepthReadiness::from_state(state),
            DepthReadiness::WaitingForFrame
        );
    }

    #[test]
    fn summarizes_depth_descriptor() {
        let descriptor = DepthFrameDescriptor::new(
            12,
            DepthFormat::Uint16Raw,
            0.001,
            DepthPayloadDescriptor::new(ImageSize::new(320, 240), 320 * 240 * 2),
        );
        let summary = DepthFrameSummary::from_descriptor(descriptor);

        assert!(summary.is_valid());
        assert_eq!(summary.frame_index, 12);
        assert!(!summary.has_confidence);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn depth_summary_round_trips_with_serde() {
        let value = DepthFrameSummary {
            frame_index: 4,
            size: ImageSize::new(4, 2),
            format: DepthFormat::Uint16Millimeters,
            meter_scale: 0.001,
            has_confidence: false,
            byte_len: 16,
        };

        let encoded = serde_json::to_string(&value).expect("summary should serialize");
        let decoded: DepthFrameSummary =
            serde_json::from_str(&encoded).expect("summary should deserialize");

        assert_eq!(decoded, value);
    }
}
