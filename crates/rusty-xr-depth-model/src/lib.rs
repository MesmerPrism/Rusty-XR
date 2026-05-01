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
    ConfidenceFormat, DepthConfidenceSource, DepthFormat, DepthFrameDescriptor, DepthMetricRange,
    DepthPayloadDescriptor, DepthViewDescriptor, EnvironmentDepthState, ImageSize,
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
    pub runtime_capture_time_ns: Option<i64>,
    pub depth_range: Option<DepthMetricRange>,
    pub layer_index: Option<u32>,
    pub layer_count: u32,
    pub has_confidence: bool,
    pub confidence_source: DepthConfidenceSource,
    pub byte_len: usize,
}

impl DepthFrameSummary {
    pub fn from_descriptor(descriptor: DepthFrameDescriptor) -> Self {
        Self {
            frame_index: descriptor.frame_index,
            size: descriptor.depth_payload.size,
            format: descriptor.format,
            meter_scale: descriptor.meter_scale,
            runtime_capture_time_ns: descriptor.runtime_capture_time_ns,
            depth_range: descriptor.depth_range,
            layer_index: descriptor.layer_index,
            layer_count: descriptor.layer_count,
            has_confidence: descriptor.confidence_payload.is_some(),
            confidence_source: descriptor.confidence_source,
            byte_len: descriptor.depth_payload.byte_len,
        }
    }

    pub fn is_valid(self) -> bool {
        self.size.is_non_empty()
            && self.byte_len > 0
            && self.meter_scale.is_finite()
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
    }
}

/// Rolling environment-depth diagnostics for one headset run.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct EnvironmentDepthDiagnosticsSummary {
    pub xr_frame_count: u64,
    pub acquire_attempts: u64,
    pub acquired_frames: u64,
    pub unavailable_frames: u64,
    pub acquire_errors: u64,
    pub repeated_capture_time_count: u64,
    pub unique_capture_times: u64,
    pub observed_acquire_hz: f32,
    pub observed_depth_hz: f32,
    pub average_acquire_cpu_ms: f32,
    pub latest_frame: Option<DepthFrameSummary>,
    pub confidence_source: DepthConfidenceSource,
}

impl EnvironmentDepthDiagnosticsSummary {
    #[allow(clippy::too_many_arguments)]
    pub fn from_counts(
        xr_frame_count: u64,
        acquire_attempts: u64,
        acquired_frames: u64,
        unavailable_frames: u64,
        acquire_errors: u64,
        repeated_capture_time_count: u64,
        elapsed_secs: f32,
        total_acquire_cpu_ms: f32,
        latest_frame: Option<DepthFrameSummary>,
        confidence_source: DepthConfidenceSource,
    ) -> Self {
        let elapsed_secs = if elapsed_secs.is_finite() && elapsed_secs > 0.0 {
            elapsed_secs
        } else {
            0.0
        };
        let observed_acquire_hz = if elapsed_secs > 0.0 {
            acquire_attempts as f32 / elapsed_secs
        } else {
            0.0
        };
        let unique_capture_times =
            acquired_frames.saturating_sub(repeated_capture_time_count.min(acquired_frames));
        let observed_depth_hz = if elapsed_secs > 0.0 {
            unique_capture_times as f32 / elapsed_secs
        } else {
            0.0
        };
        let average_acquire_cpu_ms = if acquire_attempts > 0 && total_acquire_cpu_ms.is_finite() {
            total_acquire_cpu_ms / acquire_attempts as f32
        } else {
            0.0
        };

        Self {
            xr_frame_count,
            acquire_attempts,
            acquired_frames,
            unavailable_frames,
            acquire_errors,
            repeated_capture_time_count,
            unique_capture_times,
            observed_acquire_hz,
            observed_depth_hz,
            average_acquire_cpu_ms,
            latest_frame,
            confidence_source,
        }
    }

    pub fn is_valid(&self) -> bool {
        let classified_attempts = self
            .acquired_frames
            .saturating_add(self.unavailable_frames)
            .saturating_add(self.acquire_errors);

        self.xr_frame_count >= self.acquire_attempts
            && classified_attempts <= self.acquire_attempts
            && self.repeated_capture_time_count <= self.acquired_frames
            && self.unique_capture_times <= self.acquired_frames
            && self.acquire_attempts >= self.acquired_frames
            && self.acquire_attempts >= self.unavailable_frames
            && self.acquire_attempts >= self.acquire_errors
            && self.observed_acquire_hz.is_finite()
            && self.observed_depth_hz.is_finite()
            && self.average_acquire_cpu_ms.is_finite()
            && self
                .latest_frame
                .map(DepthFrameSummary::is_valid)
                .unwrap_or(true)
    }

    pub fn acquire_success_ratio(&self) -> Option<f32> {
        (self.acquire_attempts > 0)
            .then_some(self.acquired_frames as f32 / self.acquire_attempts as f32)
    }

    pub fn repeated_depth_frame_ratio(&self) -> Option<f32> {
        (self.acquired_frames > 0)
            .then_some(self.repeated_capture_time_count as f32 / self.acquired_frames as f32)
    }
}

/// CPU readback policy for optional depth processing paths.
///
/// This is adapter guidance, not a platform call. It lets a consumer keep
/// TSDF, mesh, or physics readback separate from lightweight GPU visualization.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DepthReadbackPolicy {
    pub enabled: bool,
    pub min_interval_ms: u32,
    pub require_new_capture_time: bool,
}

impl DepthReadbackPolicy {
    pub const OFF: Self = Self {
        enabled: false,
        min_interval_ms: 0,
        require_new_capture_time: true,
    };

    pub const EVERY_FRAME: Self = Self {
        enabled: true,
        min_interval_ms: 0,
        require_new_capture_time: false,
    };

    pub const THROTTLED_MAPPING: Self = Self {
        enabled: true,
        min_interval_ms: 100,
        require_new_capture_time: true,
    };

    pub const fn new(enabled: bool, min_interval_ms: u32, require_new_capture_time: bool) -> Self {
        Self {
            enabled,
            min_interval_ms,
            require_new_capture_time,
        }
    }

    pub const fn is_enabled(self) -> bool {
        self.enabled
    }

    pub fn should_submit_readback(
        self,
        elapsed_since_last_ms: Option<u32>,
        capture_time_changed: bool,
    ) -> bool {
        if !self.enabled {
            return false;
        }
        if self.require_new_capture_time && !capture_time_changed {
            return false;
        }
        elapsed_since_last_ms
            .map(|elapsed_ms| elapsed_ms >= self.min_interval_ms)
            .unwrap_or(true)
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
        assert_eq!(summary.layer_count, 1);
        assert!(!summary.has_confidence);
        assert_eq!(summary.confidence_source, DepthConfidenceSource::None);
    }

    #[test]
    fn summarizes_timestamped_environment_depth_descriptor() {
        let descriptor = DepthFrameDescriptor::new(
            21,
            DepthFormat::Uint16Raw,
            1.0,
            DepthPayloadDescriptor::new(ImageSize::new(320, 320), 320 * 320 * 2),
        )
        .with_runtime_capture_time_ns(1_234)
        .with_depth_range(DepthMetricRange::new(0.2, 4.0))
        .with_layer(1, 2)
        .with_confidence_source(DepthConfidenceSource::AppDerived);

        let summary = DepthFrameSummary::from_descriptor(descriptor);

        assert!(summary.is_valid());
        assert_eq!(summary.runtime_capture_time_ns, Some(1_234));
        assert_eq!(summary.layer_index, Some(1));
        assert_eq!(summary.layer_count, 2);
        assert_eq!(summary.confidence_source, DepthConfidenceSource::AppDerived);
    }

    #[test]
    fn computes_environment_depth_cadence_summary() {
        let frame = DepthFrameSummary {
            frame_index: 8,
            size: ImageSize::new(320, 320),
            format: DepthFormat::Uint16Raw,
            meter_scale: 1.0,
            runtime_capture_time_ns: Some(99),
            depth_range: Some(DepthMetricRange::new(0.1, 8.0)),
            layer_index: Some(0),
            layer_count: 2,
            has_confidence: false,
            confidence_source: DepthConfidenceSource::None,
            byte_len: 320 * 320 * 2,
        };

        let summary = EnvironmentDepthDiagnosticsSummary::from_counts(
            120,
            90,
            45,
            40,
            5,
            3,
            3.0,
            9.0,
            Some(frame),
            DepthConfidenceSource::None,
        );

        assert!(summary.is_valid());
        assert_eq!(summary.observed_acquire_hz, 30.0);
        assert_eq!(summary.unique_capture_times, 42);
        assert_eq!(summary.observed_depth_hz, 14.0);
        assert_eq!(summary.average_acquire_cpu_ms, 0.1);
        assert_eq!(summary.acquire_success_ratio(), Some(0.5));
        assert_eq!(summary.repeated_depth_frame_ratio(), Some(3.0 / 45.0));

        let invalid_counts = EnvironmentDepthDiagnosticsSummary {
            acquire_attempts: 10,
            acquired_frames: 8,
            unavailable_frames: 8,
            ..summary.clone()
        };
        assert!(!invalid_counts.is_valid());
    }

    #[test]
    fn readback_policy_keeps_mapping_paths_throttled() {
        let policy = DepthReadbackPolicy::THROTTLED_MAPPING;

        assert!(!DepthReadbackPolicy::OFF.should_submit_readback(None, true));
        assert!(policy.should_submit_readback(None, true));
        assert!(!policy.should_submit_readback(Some(50), true));
        assert!(policy.should_submit_readback(Some(100), true));
        assert!(!policy.should_submit_readback(Some(120), false));
        assert!(DepthReadbackPolicy::EVERY_FRAME.should_submit_readback(Some(0), false));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn depth_summary_round_trips_with_serde() {
        let value = DepthFrameSummary {
            frame_index: 4,
            size: ImageSize::new(4, 2),
            format: DepthFormat::Uint16Millimeters,
            meter_scale: 0.001,
            runtime_capture_time_ns: Some(42),
            depth_range: Some(DepthMetricRange::new(0.1, 3.0)),
            layer_index: Some(0),
            layer_count: 2,
            has_confidence: false,
            confidence_source: DepthConfidenceSource::None,
            byte_len: 16,
        };

        let encoded = serde_json::to_string(&value).expect("summary should serialize");
        let decoded: DepthFrameSummary =
            serde_json::from_str(&encoded).expect("summary should deserialize");

        assert_eq!(decoded, value);
    }
}
