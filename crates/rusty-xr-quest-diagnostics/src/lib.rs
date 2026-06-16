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

/// Schema identifier for the public OpenXR/OpenGL ES feasibility status.
pub const OPENXR_GLES_FEASIBILITY_SCHEMA: &str = "rusty.xr.quest.openxr_gles_feasibility.v1";

/// Schema identifier for public SurfaceTexture/OES ingest diagnostics.
pub const SURFACE_TEXTURE_OES_INGEST_SCHEMA: &str = "rusty.xr.quest.surface_texture_oes_ingest.v1";

/// Required OpenXR extension for Android OpenGL ES graphics binding.
pub const OPENXR_GLES_EXTENSION: &str = "XR_KHR_opengl_es_enable";

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

/// Capability descriptor for a Quest tooling provider such as ADB, Meta VR CLI
/// / hzdb compatibility tooling, or a companion-side wrapper.
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
                String::from("metavr"),
                String::from("mcp"),
                String::from("server"),
            ],
            transport: McpTransport::Stdio,
            provider: QuestToolProviderKind::HzdbMcp,
            project_local: true,
        }
    }

    pub fn hzdb_stdio_command(command: impl Into<String>, project_local: bool) -> Self {
        Self {
            server_name: String::from("meta-horizon-mcp"),
            command: command.into(),
            args: vec![String::from("mcp"), String::from("server")],
            transport: McpTransport::Stdio,
            provider: QuestToolProviderKind::HzdbMcp,
            project_local,
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

/// Coarse phase reached by an OpenXR/OpenGL ES feasibility run.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenXrGlesFeasibilityState {
    Unknown,
    NotStarted,
    ExtensionsEnumerated,
    EglContextReady,
    GraphicsRequirementsKnown,
    SessionReady,
    SwapchainsReady,
    Rendering,
    Failed,
}

impl OpenXrGlesFeasibilityState {
    pub const fn has_session(self) -> bool {
        matches!(
            self,
            Self::SessionReady | Self::SwapchainsReady | Self::Rendering
        )
    }

    pub const fn is_rendering(self) -> bool {
        matches!(self, Self::Rendering)
    }
}

/// Public, handle-free OpenXR extension availability row.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenXrGlesExtensionStatus {
    pub extension_name: String,
    pub required: bool,
    pub available: bool,
}

impl OpenXrGlesExtensionStatus {
    pub fn required(extension_name: impl Into<String>) -> Self {
        Self {
            extension_name: extension_name.into(),
            required: true,
            available: false,
        }
    }

    pub fn optional(extension_name: impl Into<String>) -> Self {
        Self {
            extension_name: extension_name.into(),
            required: false,
            available: false,
        }
    }

    pub const fn with_available(mut self, available: bool) -> Self {
        self.available = available;
        self
    }
}

/// Result of `xrGetOpenGLESGraphicsRequirementsKHR`, normalized for logs.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenXrGlesGraphicsRequirements {
    pub min_api_version: Option<String>,
    pub max_api_version: Option<String>,
}

impl OpenXrGlesGraphicsRequirements {
    pub fn new(min_api_version: impl Into<String>, max_api_version: impl Into<String>) -> Self {
        Self {
            min_api_version: Some(min_api_version.into()),
            max_api_version: Some(max_api_version.into()),
        }
    }
}

/// EGL/GLES context summary with no raw display, context, surface, or texture handles.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EglGlesContextStatus {
    pub egl_version: Option<String>,
    pub gles_version: Option<String>,
    pub glsl_version: Option<String>,
    pub vendor: Option<String>,
    pub renderer: Option<String>,
    pub config_red_bits: Option<u8>,
    pub config_green_bits: Option<u8>,
    pub config_blue_bits: Option<u8>,
    pub config_alpha_bits: Option<u8>,
    pub config_depth_bits: Option<u8>,
    pub config_stencil_bits: Option<u8>,
    pub config_samples: Option<u8>,
    pub egl_context_current: bool,
    pub external_oes_supported: bool,
}

impl EglGlesContextStatus {
    pub fn current_gles(gles_version: impl Into<String>) -> Self {
        Self {
            egl_version: None,
            gles_version: Some(gles_version.into()),
            glsl_version: None,
            vendor: None,
            renderer: None,
            config_red_bits: None,
            config_green_bits: None,
            config_blue_bits: None,
            config_alpha_bits: None,
            config_depth_bits: None,
            config_stencil_bits: None,
            config_samples: None,
            egl_context_current: true,
            external_oes_supported: false,
        }
    }

    pub fn with_rgba_bits(mut self, red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        self.config_red_bits = Some(red);
        self.config_green_bits = Some(green);
        self.config_blue_bits = Some(blue);
        self.config_alpha_bits = Some(alpha);
        self
    }

    pub const fn has_current_gles_context(&self) -> bool {
        self.egl_context_current && self.gles_version.is_some()
    }
}

/// GL framebuffer completeness state for an acquired OpenXR swapchain image.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlFramebufferCompleteness {
    Unknown,
    Complete,
    IncompleteAttachment,
    IncompleteMissingAttachment,
    IncompleteDimensions,
    IncompleteUnsupported,
    IncompleteMultisample,
    IncompleteLayerTargets,
    OtherIncomplete,
}

impl GlFramebufferCompleteness {
    pub const fn is_complete(self) -> bool {
        matches!(self, Self::Complete)
    }
}

/// OpenXR swapchain format row reported by the GL feasibility example.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenXrGlesSwapchainFormat {
    pub format_id: i64,
    pub label: String,
    pub color_renderable: bool,
    pub depth_renderable: bool,
    pub selected: bool,
}

impl OpenXrGlesSwapchainFormat {
    pub fn color(format_id: i64, label: impl Into<String>) -> Self {
        Self {
            format_id,
            label: label.into(),
            color_renderable: true,
            depth_renderable: false,
            selected: false,
        }
    }

    pub const fn with_selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

/// Per-view OpenXR/GLES render status for static diagnostic output.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenXrGlesViewStatus {
    pub view_index: u32,
    pub recommended_width: u32,
    pub recommended_height: u32,
    pub swapchain_width: u32,
    pub swapchain_height: u32,
    pub acquired_image_index: Option<u32>,
    pub fbo_status: GlFramebufferCompleteness,
    pub viewport_x: i32,
    pub viewport_y: i32,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub diagnostic_pattern: String,
    pub last_rendered_frame_index: Option<u64>,
}

impl OpenXrGlesViewStatus {
    pub fn diagnostic_grid(
        view_index: u32,
        swapchain_width: u32,
        swapchain_height: u32,
        diagnostic_pattern: impl Into<String>,
    ) -> Self {
        Self {
            view_index,
            recommended_width: swapchain_width,
            recommended_height: swapchain_height,
            swapchain_width,
            swapchain_height,
            acquired_image_index: None,
            fbo_status: GlFramebufferCompleteness::Unknown,
            viewport_x: 0,
            viewport_y: 0,
            viewport_width: swapchain_width,
            viewport_height: swapchain_height,
            diagnostic_pattern: diagnostic_pattern.into(),
            last_rendered_frame_index: None,
        }
    }

    pub const fn viewport_matches_swapchain(&self) -> bool {
        self.viewport_x == 0
            && self.viewport_y == 0
            && self.viewport_width == self.swapchain_width
            && self.viewport_height == self.swapchain_height
    }
}

/// Public status payload for the OpenXR/OpenGL ES feasibility lane.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct OpenXrGlesFeasibilityStatus {
    pub schema: String,
    pub state: OpenXrGlesFeasibilityState,
    pub runtime_name: Option<String>,
    pub runtime_version: Option<String>,
    pub required_extensions: Vec<OpenXrGlesExtensionStatus>,
    pub graphics_requirements: Option<OpenXrGlesGraphicsRequirements>,
    pub context: Option<EglGlesContextStatus>,
    pub swapchain_formats: Vec<OpenXrGlesSwapchainFormat>,
    pub views: Vec<OpenXrGlesViewStatus>,
    pub frame_rate: Option<FrameRateSummary>,
    pub issue_codes: Vec<String>,
    pub notes: Vec<String>,
}

impl OpenXrGlesFeasibilityStatus {
    pub fn new() -> Self {
        Self {
            schema: String::from(OPENXR_GLES_FEASIBILITY_SCHEMA),
            state: OpenXrGlesFeasibilityState::NotStarted,
            runtime_name: None,
            runtime_version: None,
            required_extensions: vec![OpenXrGlesExtensionStatus::required(OPENXR_GLES_EXTENSION)],
            graphics_requirements: None,
            context: None,
            swapchain_formats: Vec::new(),
            views: Vec::new(),
            frame_rate: None,
            issue_codes: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn required_gles_extension_available(&self) -> bool {
        self.required_extensions.iter().any(|extension| {
            extension.extension_name == OPENXR_GLES_EXTENSION && extension.available
        })
    }

    pub fn selected_color_swapchain_format(&self) -> Option<&OpenXrGlesSwapchainFormat> {
        self.swapchain_formats
            .iter()
            .find(|format| format.selected && format.color_renderable)
    }

    pub fn is_iteration2_ready(&self) -> bool {
        self.state.is_rendering()
            && self.required_gles_extension_available()
            && self
                .context
                .as_ref()
                .map(EglGlesContextStatus::has_current_gles_context)
                .unwrap_or(false)
            && self.graphics_requirements.is_some()
            && self.selected_color_swapchain_format().is_some()
            && self.views.len() >= 2
            && self
                .views
                .iter()
                .all(|view| view.fbo_status.is_complete() && view.viewport_matches_swapchain())
            && self
                .frame_rate
                .map(|frame_rate| frame_rate.sample_count > 0)
                .unwrap_or(false)
            && self.issue_codes.is_empty()
    }
}

impl Default for OpenXrGlesFeasibilityStatus {
    fn default() -> Self {
        Self::new()
    }
}

/// Coarse phase reached by an Android SurfaceTexture/OES ingest path.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceTextureOesIngestState {
    Unknown,
    NotStarted,
    ExternalTextureCreated,
    SurfaceTextureCreated,
    OutputSurfaceReady,
    DecoderConfigured,
    DecoderStarted,
    FrameAvailable,
    TextureUpdated,
    Failed,
}

impl SurfaceTextureOesIngestState {
    pub const fn has_decoder(self) -> bool {
        matches!(
            self,
            Self::DecoderConfigured
                | Self::DecoderStarted
                | Self::FrameAvailable
                | Self::TextureUpdated
        )
    }

    pub const fn has_updated_texture(self) -> bool {
        matches!(self, Self::TextureUpdated)
    }
}

/// Per-eye, handle-free diagnostics for a SurfaceTexture backed by
/// `GL_TEXTURE_EXTERNAL_OES`.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceTextureOesEyeStatus {
    pub view_index: u32,
    pub stream_id: Option<String>,
    pub source_eye: Option<String>,
    pub external_texture_created: bool,
    pub surface_texture_created: bool,
    pub output_surface_created: bool,
    pub decoder_configured: bool,
    pub decoder_started: bool,
    pub source_width: Option<u32>,
    pub source_height: Option<u32>,
    pub frame_available_count: u64,
    pub update_tex_image_count: u64,
    pub skipped_update_count: u64,
    pub latest_stream_sequence: Option<u64>,
    pub latest_queued_pts_us: Option<i64>,
    pub latest_surface_texture_timestamp_ns: Option<i64>,
    pub latest_transform_matrix_hash: Option<String>,
    pub transform_matrix_sample_count: u64,
    pub decoder_error_count: u64,
    pub latest_decoder_error: Option<String>,
    pub last_update_frame_index: Option<u64>,
}

impl SurfaceTextureOesEyeStatus {
    pub fn new(view_index: u32) -> Self {
        Self {
            view_index,
            stream_id: None,
            source_eye: None,
            external_texture_created: false,
            surface_texture_created: false,
            output_surface_created: false,
            decoder_configured: false,
            decoder_started: false,
            source_width: None,
            source_height: None,
            frame_available_count: 0,
            update_tex_image_count: 0,
            skipped_update_count: 0,
            latest_stream_sequence: None,
            latest_queued_pts_us: None,
            latest_surface_texture_timestamp_ns: None,
            latest_transform_matrix_hash: None,
            transform_matrix_sample_count: 0,
            decoder_error_count: 0,
            latest_decoder_error: None,
            last_update_frame_index: None,
        }
    }

    pub fn for_stream(
        view_index: u32,
        stream_id: impl Into<String>,
        source_eye: impl Into<String>,
    ) -> Self {
        let mut status = Self::new(view_index);
        status.stream_id = Some(stream_id.into());
        status.source_eye = Some(source_eye.into());
        status
    }

    pub fn mark_surface_ready(mut self) -> Self {
        self.external_texture_created = true;
        self.surface_texture_created = true;
        self.output_surface_created = true;
        self
    }

    pub fn mark_decoder_started(mut self) -> Self {
        self.decoder_configured = true;
        self.decoder_started = true;
        self
    }

    pub fn record_update(
        &mut self,
        frame_index: u64,
        sequence: u64,
        queued_pts_us: i64,
        surface_timestamp_ns: i64,
        transform_hash: impl Into<String>,
    ) {
        self.frame_available_count = self.frame_available_count.saturating_add(1);
        self.update_tex_image_count = self.update_tex_image_count.saturating_add(1);
        self.latest_stream_sequence = Some(sequence);
        self.latest_queued_pts_us = Some(queued_pts_us);
        self.latest_surface_texture_timestamp_ns = Some(surface_timestamp_ns);
        self.latest_transform_matrix_hash = Some(transform_hash.into());
        self.transform_matrix_sample_count = self.transform_matrix_sample_count.saturating_add(1);
        self.last_update_frame_index = Some(frame_index);
    }

    pub const fn is_ready_without_errors(&self) -> bool {
        self.external_texture_created
            && self.surface_texture_created
            && self.output_surface_created
            && self.decoder_configured
            && self.decoder_started
            && self.update_tex_image_count > 0
            && self.decoder_error_count == 0
    }
}

/// Public status payload for broker or camera frames decoded into
/// SurfaceTexture/OES textures.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceTextureOesIngestStatus {
    pub schema: String,
    pub state: SurfaceTextureOesIngestState,
    pub session_id: Option<String>,
    pub codec_name: Option<String>,
    pub codec_mime: Option<String>,
    pub eyes: Vec<SurfaceTextureOesEyeStatus>,
    pub source_feed_rate: Option<FrameRateSummary>,
    pub texture_update_rate: Option<FrameRateSummary>,
    pub cpu_yuv_upload_count: u64,
    pub issue_codes: Vec<String>,
    pub notes: Vec<String>,
}

impl SurfaceTextureOesIngestStatus {
    pub fn new() -> Self {
        Self {
            schema: String::from(SURFACE_TEXTURE_OES_INGEST_SCHEMA),
            state: SurfaceTextureOesIngestState::NotStarted,
            session_id: None,
            codec_name: None,
            codec_mime: None,
            eyes: Vec::new(),
            source_feed_rate: None,
            texture_update_rate: None,
            cpu_yuv_upload_count: 0,
            issue_codes: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn with_eye(mut self, eye: SurfaceTextureOesEyeStatus) -> Self {
        self.eyes.push(eye);
        self
    }

    pub const fn uses_no_cpu_yuv_upload(&self) -> bool {
        self.cpu_yuv_upload_count == 0
    }

    pub fn is_iteration4_ready(&self) -> bool {
        self.state.has_updated_texture()
            && self.uses_no_cpu_yuv_upload()
            && self.eyes.len() >= 2
            && self
                .eyes
                .iter()
                .all(SurfaceTextureOesEyeStatus::is_ready_without_errors)
            && self
                .texture_update_rate
                .map(|frame_rate| frame_rate.sample_count > 0)
                .unwrap_or(false)
            && self.issue_codes.is_empty()
    }
}

impl Default for SurfaceTextureOesIngestStatus {
    fn default() -> Self {
        Self::new()
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
                String::from("metavr"),
                String::from("mcp"),
                String::from("server")
            ]
        );
    }

    #[test]
    fn hzdb_mcp_config_can_use_configured_hzdb_executable() {
        let config = McpServerConfig::hzdb_stdio_command("<mqdh-hzdb-executable>", false);

        assert_eq!(config.server_name, "meta-horizon-mcp");
        assert_eq!(config.command, "<mqdh-hzdb-executable>");
        assert_eq!(config.transport, McpTransport::Stdio);
        assert_eq!(config.provider, QuestToolProviderKind::HzdbMcp);
        assert!(!config.project_local);
        assert_eq!(
            config.args,
            vec![String::from("mcp"), String::from("server")]
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

    #[test]
    fn openxr_gles_status_defaults_to_not_ready() {
        let status = OpenXrGlesFeasibilityStatus::new();

        assert_eq!(status.schema, OPENXR_GLES_FEASIBILITY_SCHEMA);
        assert!(!status.required_gles_extension_available());
        assert!(!status.is_iteration2_ready());
    }

    #[test]
    fn openxr_gles_status_requires_rendering_fbos_and_cadence() {
        let mut status = OpenXrGlesFeasibilityStatus::new();
        status.state = OpenXrGlesFeasibilityState::Rendering;
        status.required_extensions[0].available = true;
        status.graphics_requirements = Some(OpenXrGlesGraphicsRequirements::new(
            "OpenGL ES 3.0",
            "OpenGL ES 3.2",
        ));
        status.context =
            Some(EglGlesContextStatus::current_gles("OpenGL ES 3.2").with_rgba_bits(8, 8, 8, 8));
        status
            .swapchain_formats
            .push(OpenXrGlesSwapchainFormat::color(0x8058, "GL_RGBA8").with_selected(true));
        status.views = vec![
            OpenXrGlesViewStatus {
                fbo_status: GlFramebufferCompleteness::Complete,
                acquired_image_index: Some(0),
                last_rendered_frame_index: Some(1),
                ..OpenXrGlesViewStatus::diagnostic_grid(0, 1440, 1584, "left-grid")
            },
            OpenXrGlesViewStatus {
                fbo_status: GlFramebufferCompleteness::Complete,
                acquired_image_index: Some(0),
                last_rendered_frame_index: Some(1),
                ..OpenXrGlesViewStatus::diagnostic_grid(1, 1440, 1584, "right-grid")
            },
        ];
        status.frame_rate = FrameRateSummary::from_frame_deltas(&[1.0 / 72.0]);

        assert!(status.is_iteration2_ready());

        status.views[1].viewport_width = 1024;
        assert!(!status.is_iteration2_ready());
    }

    #[test]
    fn surface_texture_oes_ingest_requires_two_updated_eyes_and_no_cpu_upload() {
        let mut left =
            SurfaceTextureOesEyeStatus::for_stream(0, "video:left", "left").mark_surface_ready();
        left = left.mark_decoder_started();
        left.record_update(10, 42, 1_000, 2_000, "m44:identity");

        let mut right =
            SurfaceTextureOesEyeStatus::for_stream(1, "video:right", "right").mark_surface_ready();
        right = right.mark_decoder_started();
        right.record_update(10, 43, 1_001, 2_001, "m44:identity");

        let mut status = SurfaceTextureOesIngestStatus::new()
            .with_eye(left)
            .with_eye(right);
        status.state = SurfaceTextureOesIngestState::TextureUpdated;
        status.texture_update_rate = FrameRateSummary::from_frame_deltas(&[1.0 / 72.0]);

        assert!(status.is_iteration4_ready());

        status.cpu_yuv_upload_count = 1;
        assert!(!status.is_iteration4_ready());
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

    #[cfg(feature = "serde")]
    #[test]
    fn openxr_gles_status_round_trips_with_serde() {
        let mut status = OpenXrGlesFeasibilityStatus::new();
        status.state = OpenXrGlesFeasibilityState::ExtensionsEnumerated;
        status.required_extensions[0].available = true;

        let encoded = serde_json::to_string(&status).expect("status should serialize");
        let decoded: OpenXrGlesFeasibilityStatus =
            serde_json::from_str(&encoded).expect("status should deserialize");

        assert_eq!(decoded, status);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn surface_texture_oes_ingest_status_round_trips_with_serde() {
        let mut eye = SurfaceTextureOesEyeStatus::for_stream(0, "video:left", "left")
            .mark_surface_ready()
            .mark_decoder_started();
        eye.record_update(7, 99, 10_000, 11_000, "m44:abcd");

        let mut status = SurfaceTextureOesIngestStatus::new().with_eye(eye);
        status.state = SurfaceTextureOesIngestState::TextureUpdated;
        status.codec_mime = Some(String::from("video/avc"));

        let encoded = serde_json::to_string(&status).expect("status should serialize");
        let decoded: SurfaceTextureOesIngestStatus =
            serde_json::from_str(&encoded).expect("status should deserialize");

        assert_eq!(decoded, status);
    }
}
