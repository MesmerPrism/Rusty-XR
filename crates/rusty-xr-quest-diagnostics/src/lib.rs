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

/// Public tool provider family for Quest development operations.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestToolProviderKind {
    Adb,
    HzdbCli,
    HzdbMcp,
    RustyXrCompanion,
    BrokerShellHelper,
    Manual,
    Other,
}

/// Safety class for an external provider operation.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderOperationSafety {
    ReadOnly,
    BoundedCapture,
    FileRead,
    FileWrite,
    FileDelete,
    AppLifecycle,
    DeviceSetting,
    ShellCommand,
    NetworkForward,
    Root,
    Unknown,
}

impl ProviderOperationSafety {
    pub const fn requires_operator_gate(self) -> bool {
        !matches!(self, Self::ReadOnly | Self::BoundedCapture | Self::FileRead)
    }
}

/// Capability descriptor for a Quest tooling provider such as ADB, hzdb, or a
/// companion-side wrapper.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderCapability {
    pub provider: QuestToolProviderKind,
    pub capability_id: String,
    pub command_group: String,
    pub description: String,
    pub safety: ProviderOperationSafety,
    pub requires_device: bool,
    pub requires_network: bool,
}

impl ProviderCapability {
    pub fn hzdb(
        capability_id: impl Into<String>,
        command_group: impl Into<String>,
        safety: ProviderOperationSafety,
    ) -> Self {
        Self {
            provider: QuestToolProviderKind::HzdbCli,
            capability_id: capability_id.into(),
            command_group: command_group.into(),
            description: String::new(),
            safety,
            requires_device: true,
            requires_network: false,
        }
    }

    pub const fn requires_operator_gate(&self) -> bool {
        self.safety.requires_operator_gate()
    }
}

/// Pre-test device health summary for repeatable Quest validation runs.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceHealth {
    pub provider: QuestToolProviderKind,
    pub connected: bool,
    pub readiness: DeviceReadiness,
    pub battery_level_percent: Option<u8>,
    pub storage_available_bytes: Option<u64>,
    pub controller_count: u8,
    pub ui_ready: bool,
    pub issues: Vec<String>,
}

impl DeviceHealth {
    pub fn is_pretest_ready(&self, min_battery_percent: u8) -> bool {
        let battery_ok = self
            .battery_level_percent
            .map(|level| level >= min_battery_percent)
            .unwrap_or(false);

        self.connected
            && self.ui_ready
            && self.readiness.is_operator_ready()
            && battery_ok
            && self.issues.is_empty()
    }
}

/// Controller/controller-like device state visible to development tooling.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControllerInfo {
    pub role: String,
    pub connected: bool,
    pub battery_level_percent: Option<u8>,
    pub firmware_version: Option<String>,
    pub last_seen_elapsed_ns: Option<u64>,
}

/// Generic installed application metadata.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppInfo {
    pub package_name: String,
    pub label: Option<String>,
    pub version_name: Option<String>,
    pub version_code: Option<u64>,
    pub apk_path: Option<String>,
    pub debuggable: Option<bool>,
    pub enabled: Option<bool>,
}

/// Current foreground application snapshot.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForegroundApp {
    pub package_name: Option<String>,
    pub activity_name: Option<String>,
    pub process_id: Option<u32>,
    pub source: String,
}

impl ForegroundApp {
    pub fn is_package(&self, package_name: &str) -> bool {
        self.package_name.as_deref() == Some(package_name)
    }
}

/// Portable Android log severity selector.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Verbose,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

/// Portable Android log buffer selector.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogBuffer {
    Main,
    System,
    Crash,
    Radio,
    Events,
    All,
    Default,
}

/// Portable logcat output format selector.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogOutputFormat {
    Brief,
    Long,
    Process,
    Raw,
    Tag,
    Thread,
    ThreadTime,
    Time,
}

/// Structured log filter that can be mapped to ADB, hzdb, or companion tools.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogFilter {
    pub package_name: Option<String>,
    pub tag: Option<String>,
    pub min_level: Option<LogLevel>,
    pub pid: Option<u32>,
    pub regex: Option<String>,
    pub buffer: LogBuffer,
    pub output_format: LogOutputFormat,
    pub lines: Option<u32>,
    pub follow: bool,
    pub clear_before_read: bool,
}

impl LogFilter {
    pub fn recent_errors_for_package(package_name: impl Into<String>, lines: u32) -> Self {
        Self {
            package_name: Some(package_name.into()),
            tag: None,
            min_level: Some(LogLevel::Error),
            pid: None,
            regex: None,
            buffer: LogBuffer::Default,
            output_format: LogOutputFormat::ThreadTime,
            lines: Some(lines),
            follow: false,
            clear_before_read: false,
        }
    }
}

/// Headset screenshot acquisition route.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenshotMethod {
    HzdbMetacam,
    HzdbScreencap,
    AdbScreencap,
    OvrMetrics,
    Unknown,
}

/// Screenshot capture request/manifest entry.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScreenshotCapture {
    pub method: ScreenshotMethod,
    pub output_path: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub artifact_label: String,
    pub includes_overlays: bool,
}

impl ScreenshotCapture {
    pub fn hzdb_screencap(output_path: impl Into<String>) -> Self {
        Self {
            method: ScreenshotMethod::HzdbScreencap,
            output_path: Some(output_path.into()),
            width: None,
            height: None,
            artifact_label: String::from("hzdb-screencap"),
            includes_overlays: true,
        }
    }
}

/// Device file operation shape for tooling wrappers.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceFileOperationKind {
    List,
    Pull,
    Push,
    Remove,
    MakeDir,
}

/// Structured file operation request/plan.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceFileOperation {
    pub kind: DeviceFileOperationKind,
    pub remote_path: String,
    pub local_path: Option<String>,
    pub recursive: bool,
    pub dry_run: bool,
}

impl DeviceFileOperation {
    pub const fn is_mutating(&self) -> bool {
        matches!(
            self.kind,
            DeviceFileOperationKind::Push
                | DeviceFileOperationKind::Remove
                | DeviceFileOperationKind::MakeDir
        )
    }
}

/// Perfetto capture preset.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PerfTraceMode {
    Standard,
    Gpu,
    Cpu,
    Lightweight,
    Full,
    Custom,
}

/// Perfetto trace lifecycle state.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PerfTraceState {
    Planned,
    Capturing,
    Captured,
    Loaded,
    Analyzed,
    Failed,
}

/// Perfetto trace session manifest.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PerfTraceSession {
    pub session_id: String,
    pub provider: QuestToolProviderKind,
    pub mode: PerfTraceMode,
    pub state: PerfTraceState,
    pub app_package: Option<String>,
    pub duration_ms: Option<u64>,
    pub output_path: Option<String>,
    pub xr_runtime: bool,
    pub gpu_render_stage: bool,
    pub gpu_metrics: bool,
    pub cpu_scheduling: bool,
    pub vulkan_layer: bool,
    pub extended_scheduling: bool,
}

impl PerfTraceSession {
    pub fn hzdb_custom(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            provider: QuestToolProviderKind::HzdbCli,
            mode: PerfTraceMode::Custom,
            state: PerfTraceState::Planned,
            app_package: None,
            duration_ms: None,
            output_path: None,
            xr_runtime: true,
            gpu_render_stage: true,
            gpu_metrics: true,
            cpu_scheduling: true,
            vulkan_layer: false,
            extended_scheduling: false,
        }
    }

    pub const fn is_ready_for_analysis(&self) -> bool {
        matches!(
            self.state,
            PerfTraceState::Captured | PerfTraceState::Loaded | PerfTraceState::Analyzed
        )
    }
}

/// Normalized performance metric extracted from a trace or log source.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PerfMetric {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub start_ts_ns: Option<u64>,
    pub end_ts_ns: Option<u64>,
    pub note: Option<String>,
}

/// Meta Quest documentation category used by docs-first workflows.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestDocumentationCategory {
    All,
    Unity,
    Unreal,
    SpatialSdk,
    Android,
    Native,
    Web,
    Resources,
    Design,
    Policy,
}

/// Documentation search result from an external provider.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocSearchResult {
    pub title: String,
    pub url: Option<String>,
    pub category: QuestDocumentationCategory,
    pub excerpt: String,
    pub source_id: Option<String>,
}

/// API reference family used by provider search results.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiReferencePlatform {
    Unity,
    Unreal,
    Android,
    Native,
    Unknown,
}

/// API reference search result from an external provider.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiReferenceResult {
    pub symbol: String,
    pub platform: ApiReferencePlatform,
    pub url: Option<String>,
    pub summary: String,
    pub package_or_namespace: Option<String>,
}

/// Optional 3D asset search result for prototyping workflows.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetSearchResult {
    pub asset_id: Option<String>,
    pub title: String,
    pub url: Option<String>,
    pub license_label: Option<String>,
    pub lod_count: Option<u8>,
    pub requires_auth: bool,
}

/// MCP transport shape for provider configuration.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpTransport {
    Stdio,
    Sse,
    StreamableHttp,
}

/// MCP server launch/config descriptor.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpServerConfig {
    pub server_name: String,
    pub command: String,
    pub args: Vec<String>,
    pub transport: McpTransport,
    pub provider: QuestToolProviderKind,
    pub project_local: bool,
}

impl McpServerConfig {
    pub fn hzdb_stdio_npx() -> Self {
        Self {
            server_name: String::from("meta-horizon-mcp"),
            command: String::from("npx"),
            args: vec![
                String::from("-y"),
                String::from("@meta-quest/hzdb"),
                String::from("mcp"),
                String::from("server"),
            ],
            transport: McpTransport::Stdio,
            provider: QuestToolProviderKind::HzdbMcp,
            project_local: true,
        }
    }
}

/// Agent skill metadata that a Rusty XR tool can reference without copying the
/// skill body into core.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentSkill {
    pub id: String,
    pub source: String,
    pub description: String,
    pub recommended: bool,
    pub local_policy_note: String,
}

/// Combined provider snapshot for reports and broker/companion status pages.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestDevelopmentProviderSnapshot {
    pub provider: QuestToolProviderKind,
    pub version: Option<String>,
    pub capabilities: Vec<ProviderCapability>,
    pub device_health: Option<DeviceHealth>,
    pub foreground_app: Option<ForegroundApp>,
    pub mcp: Option<McpServerConfig>,
    pub notes: Vec<String>,
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
    fn provider_safety_gates_mutating_operations() {
        let status = ProviderCapability::hzdb(
            "device.health-check",
            "device",
            ProviderOperationSafety::ReadOnly,
        );
        assert!(!status.requires_operator_gate());

        let shell =
            ProviderCapability::hzdb("shell", "shell", ProviderOperationSafety::ShellCommand);
        assert!(shell.requires_operator_gate());
    }

    #[test]
    fn device_health_requires_ready_ui_battery_and_no_issues() {
        let mut health = DeviceHealth {
            provider: QuestToolProviderKind::HzdbCli,
            connected: true,
            readiness: DeviceReadiness::AppVisible,
            battery_level_percent: Some(80),
            storage_available_bytes: Some(1024 * 1024 * 1024),
            controller_count: 2,
            ui_ready: true,
            issues: Vec::new(),
        };

        assert!(health.is_pretest_ready(30));

        health.issues.push(String::from("system dialog visible"));
        assert!(!health.is_pretest_ready(30));
    }

    #[test]
    fn log_filter_defaults_to_recent_error_tail() {
        let filter = LogFilter::recent_errors_for_package("com.example.xr", 250);

        assert_eq!(filter.package_name.as_deref(), Some("com.example.xr"));
        assert_eq!(filter.min_level, Some(LogLevel::Error));
        assert_eq!(filter.output_format, LogOutputFormat::ThreadTime);
        assert_eq!(filter.lines, Some(250));
        assert!(!filter.follow);
    }

    #[test]
    fn hzdb_mcp_config_uses_project_local_stdio_npx() {
        let config = McpServerConfig::hzdb_stdio_npx();

        assert_eq!(config.server_name, "meta-horizon-mcp");
        assert_eq!(config.command, "npx");
        assert_eq!(config.transport, McpTransport::Stdio);
        assert_eq!(config.provider, QuestToolProviderKind::HzdbMcp);
        assert!(config.project_local);
        assert_eq!(
            config.args,
            vec![
                String::from("-y"),
                String::from("@meta-quest/hzdb"),
                String::from("mcp"),
                String::from("server")
            ]
        );
    }

    #[test]
    fn perf_trace_session_marks_captured_traces_as_analyzable() {
        let mut session = PerfTraceSession::hzdb_custom("trace-001");

        assert!(!session.is_ready_for_analysis());

        session.state = PerfTraceState::Captured;
        assert!(session.is_ready_for_analysis());
        assert!(session.xr_runtime);
        assert!(session.gpu_render_stage);
        assert!(session.cpu_scheduling);
    }

    #[test]
    fn file_operations_flag_mutations() {
        let read = DeviceFileOperation {
            kind: DeviceFileOperationKind::Pull,
            remote_path: String::from("/sdcard/report.json"),
            local_path: Some(String::from("artifacts/report.json")),
            recursive: false,
            dry_run: false,
        };
        assert!(!read.is_mutating());

        let remove = DeviceFileOperation {
            kind: DeviceFileOperationKind::Remove,
            remote_path: String::from("/sdcard/tmp"),
            local_path: None,
            recursive: true,
            dry_run: true,
        };
        assert!(remove.is_mutating());
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
