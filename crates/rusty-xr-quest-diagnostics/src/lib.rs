//! Reusable Quest diagnostics models and helpers for Rusty XR.
//!
//! These are public status models, not device-control scripts. App-specific
//! package names, serials, launch activities, and release metadata belong in
//! downstream application repos.
//!
//! Enable the `serde` feature when diagnostic snapshots need to be exported to
//! JSON, logs, or operator manifests.

pub use rusty_xr_contracts::{CounterValue, RuntimeCounters};

/// Crate version exposed for lightweight smoke checks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Coarse headset/device power state.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevicePowerState {
    Unknown,
    Offline,
    Asleep,
    Awake,
}

/// Operator-visible readiness state.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceReadiness {
    Unknown,
    Disconnected,
    PowerOnly,
    SystemDialog,
    RuntimeReady,
    AppVisible,
}

impl DeviceReadiness {
    pub const fn is_operator_ready(self) -> bool {
        matches!(self, Self::RuntimeReady | Self::AppVisible)
    }
}

/// Generic package launch status.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageLaunchState {
    pub package_name: Option<String>,
    pub activity_name: Option<String>,
    pub process_running: bool,
    pub activity_focused: bool,
    pub permission_prompt_visible: bool,
}

impl PackageLaunchState {
    pub fn new(package_name: impl Into<String>) -> Self {
        Self {
            package_name: Some(package_name.into()),
            activity_name: None,
            process_running: false,
            activity_focused: false,
            permission_prompt_visible: false,
        }
    }

    pub fn is_launched(&self) -> bool {
        self.process_running && self.activity_focused && !self.permission_prompt_visible
    }
}

/// Generic frame timing summary.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameRateSummary {
    pub sample_count: u64,
    pub average_fps: f32,
    pub min_fps: f32,
    pub max_fps: f32,
}

impl FrameRateSummary {
    pub fn from_frame_deltas(deltas_seconds: &[f32]) -> Option<Self> {
        let mut summary = Self {
            sample_count: 0,
            average_fps: 0.0,
            min_fps: f32::INFINITY,
            max_fps: 0.0,
        };
        let mut fps_sum = 0.0;

        for delta in deltas_seconds.iter().copied() {
            if !delta.is_finite() || delta <= 0.0 {
                continue;
            }
            let fps = 1.0 / delta;
            summary.sample_count += 1;
            summary.min_fps = summary.min_fps.min(fps);
            summary.max_fps = summary.max_fps.max(fps);
            fps_sum += fps;
        }

        if summary.sample_count == 0 {
            None
        } else {
            summary.average_fps = fps_sum / summary.sample_count as f32;
            Some(summary)
        }
    }

    pub fn is_near_target_hz(self, target_hz: f32, tolerance_hz: f32) -> bool {
        target_hz.is_finite()
            && tolerance_hz.is_finite()
            && (self.average_fps - target_hz).abs() <= tolerance_hz.max(0.0)
    }
}

/// Public summary of the current Quest runtime state.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct QuestRuntimeStatus {
    pub power_state: DevicePowerState,
    pub readiness: DeviceReadiness,
    pub package: Option<PackageLaunchState>,
    pub frame_rate: Option<FrameRateSummary>,
    pub counters: RuntimeCounters,
}

impl QuestRuntimeStatus {
    pub fn new(power_state: DevicePowerState, readiness: DeviceReadiness) -> Self {
        Self {
            power_state,
            readiness,
            package: None,
            frame_rate: None,
            counters: RuntimeCounters::default(),
        }
    }

    pub fn is_app_visible(&self) -> bool {
        self.readiness == DeviceReadiness::AppVisible
            && self
                .package
                .as_ref()
                .map(PackageLaunchState::is_launched)
                .unwrap_or(false)
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
    fn package_launch_requires_focus_and_no_prompt() {
        let mut state = PackageLaunchState::new("com.example.xr");
        state.process_running = true;
        state.activity_focused = true;

        assert!(state.is_launched());

        state.permission_prompt_visible = true;
        assert!(!state.is_launched());
    }

    #[test]
    fn summarizes_frame_rate_from_deltas() {
        let summary = FrameRateSummary::from_frame_deltas(&[1.0 / 72.0, 1.0 / 72.0])
            .expect("summary should exist");

        assert_eq!(summary.sample_count, 2);
        assert!(summary.is_near_target_hz(72.0, 0.01));
    }

    #[test]
    fn app_visible_requires_runtime_and_launch_state() {
        let mut status =
            QuestRuntimeStatus::new(DevicePowerState::Awake, DeviceReadiness::AppVisible);
        let mut package = PackageLaunchState::new("com.example.xr");
        package.process_running = true;
        package.activity_focused = true;
        status.package = Some(package);

        assert!(status.is_app_visible());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn quest_status_round_trips_with_serde() {
        let mut status =
            QuestRuntimeStatus::new(DevicePowerState::Awake, DeviceReadiness::AppVisible);
        let mut package = PackageLaunchState::new("com.example.xr");
        package.process_running = true;
        package.activity_focused = true;
        status.package = Some(package);
        status.frame_rate = FrameRateSummary::from_frame_deltas(&[1.0 / 72.0]);

        let encoded = serde_json::to_string(&status).expect("status should serialize");
        let decoded: QuestRuntimeStatus =
            serde_json::from_str(&encoded).expect("status should deserialize");

        assert_eq!(decoded, status);
    }
}
