use crate::Vec2;

/// Versioned schema id for home panel descriptors.
pub const HOME_PANEL_DESCRIPTOR_SCHEMA: &str = "rusty.xr.home.panel.v1";

/// Versioned schema id for home session state.
pub const HOME_SESSION_STATE_SCHEMA: &str = "rusty.xr.home.state.v1";

/// Versioned schema id for launcher entries.
pub const HOME_LAUNCHER_ENTRY_SCHEMA: &str = "rusty.xr.home.launcher_entry.v1";

/// Versioned schema id for settings shortcut descriptors.
pub const HOME_SETTINGS_SHORTCUT_SCHEMA: &str = "rusty.xr.home.settings_shortcut.v1";

/// Versioned schema id for focus recovery events.
pub const HOME_FOCUS_RECOVERY_EVENT_SCHEMA: &str = "rusty.xr.home.focus_recovery_event.v1";

/// High-level mode for a Rusty Kiosk, developer-home, or broker surface.
///
/// These are product and routing modes, not platform privileges. A normal app
/// can choose one of these modes for its own UI, but it does not become system
/// UI or an MDM/device-owner controller.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HomeMode {
    /// Normal 2D broker console / launcher surface.
    #[default]
    Normal2d,
    /// Own immersive app with runtime passthrough behind app-owned panels.
    ImmersivePassthrough,
    /// Own immersive app with a fully virtual background.
    ImmersiveVirtual,
    /// Explicit developer/lab mode where an external helper may supervise.
    DeveloperSupervisor,
    /// Real kiosk-style deployment through a managed-device route.
    ManagedKiosk,
}

/// Kind of panel that can appear in a 2D broker or Rusty Kiosk layout.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HomePanelKind {
    /// Broker-owned page such as launcher, diagnostics, streams, or settings.
    #[default]
    BrokerPage,
    /// App-owned native panel rendered by the home shell.
    LocalApplet,
    /// Bundled or local web applet, when an adapter provides a renderer.
    WebApplet,
    /// Cooperating app publishes status, commands, or a surface route.
    CooperatingApp,
    /// Decoded stream or remote surface rendered by the home shell.
    RemoteSurface,
    /// Documented system settings front door plus return-state tracking.
    SettingsShortcut,
    /// Diagnostic-only panel.
    Diagnostic,
}

/// Default placement policy for a home panel.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HomePanelPlacement {
    /// Standard 2D Android/Horizon panel.
    #[default]
    Flat2d,
    /// Head-locked XR panel.
    HeadLocked,
    /// World-locked XR panel.
    WorldLocked,
    /// Hand or wrist anchored quick panel.
    HandAnchored,
    /// Desk/table style world placement.
    Desk,
}

/// Public descriptor for a broker page or app-owned home panel.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct HomePanelDescriptor {
    pub schema: String,
    pub panel_id: String,
    pub title: String,
    pub kind: HomePanelKind,
    pub default_size_m: Vec2,
    pub min_size_m: Vec2,
    pub max_size_m: Vec2,
    pub placement: HomePanelPlacement,
    pub requires_helper: bool,
    pub commands: Vec<String>,
}

impl HomePanelDescriptor {
    pub fn new(panel_id: impl Into<String>, title: impl Into<String>, kind: HomePanelKind) -> Self {
        Self {
            schema: HOME_PANEL_DESCRIPTOR_SCHEMA.to_string(),
            panel_id: panel_id.into(),
            title: title.into(),
            kind,
            default_size_m: Vec2::new(0.72, 0.45),
            min_size_m: Vec2::new(0.35, 0.24),
            max_size_m: Vec2::new(1.20, 0.80),
            placement: HomePanelPlacement::Flat2d,
            requires_helper: false,
            commands: Vec::new(),
        }
    }

    pub const fn with_placement(mut self, placement: HomePanelPlacement) -> Self {
        self.placement = placement;
        self
    }

    pub const fn with_size_bounds(
        mut self,
        default_size_m: Vec2,
        min_size_m: Vec2,
        max_size_m: Vec2,
    ) -> Self {
        self.default_size_m = default_size_m;
        self.min_size_m = min_size_m;
        self.max_size_m = max_size_m;
        self
    }

    pub const fn requiring_helper(mut self) -> Self {
        self.requires_helper = true;
        self
    }

    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.commands.push(command.into());
        self
    }

    pub fn uses_helper_only_commands(&self) -> bool {
        self.commands
            .iter()
            .any(|command| helper_only_command(command))
    }

    pub fn is_valid(&self) -> bool {
        self.schema == HOME_PANEL_DESCRIPTOR_SCHEMA
            && stable_id(&self.panel_id)
            && non_empty(&self.title)
            && size_range_valid(self.default_size_m, self.min_size_m, self.max_size_m)
            && self.commands.iter().all(|command| stable_id(command))
            && (!self.uses_helper_only_commands() || self.requires_helper)
    }
}

/// Source that produced a launcher entry.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LauncherEntrySource {
    /// App-visible Android package manager query.
    #[default]
    PackageManager,
    /// Public or local catalog metadata.
    Catalog,
    /// User-entered package id or component.
    Manual,
    /// External helper observed or resolved the package.
    HelperObserved,
}

/// Public launcher row for a known target app.
///
/// This can describe a normal front-door launch, a catalog entry, or a helper
/// observed package. It does not imply install, force-stop, or shell identity.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LauncherEntry {
    pub schema: String,
    pub package_name: String,
    pub label: String,
    pub launch_component: Option<String>,
    pub source: LauncherEntrySource,
    pub requires_helper: bool,
    pub profile_id: Option<String>,
    pub warnings: Vec<String>,
}

impl LauncherEntry {
    pub fn new(package_name: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            schema: HOME_LAUNCHER_ENTRY_SCHEMA.to_string(),
            package_name: package_name.into(),
            label: label.into(),
            launch_component: None,
            source: LauncherEntrySource::PackageManager,
            requires_helper: false,
            profile_id: None,
            warnings: Vec::new(),
        }
    }

    pub fn with_launch_component(mut self, component: impl Into<String>) -> Self {
        self.launch_component = Some(component.into());
        self
    }

    pub const fn with_source(mut self, source: LauncherEntrySource) -> Self {
        self.source = source;
        self
    }

    pub const fn requiring_helper(mut self) -> Self {
        self.requires_helper = true;
        self
    }

    pub fn with_profile_id(mut self, profile_id: impl Into<String>) -> Self {
        self.profile_id = Some(profile_id.into());
        self
    }

    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        self.schema == HOME_LAUNCHER_ENTRY_SCHEMA
            && package_like(&self.package_name)
            && non_empty(&self.label)
            && self
                .launch_component
                .as_ref()
                .map(|component| non_empty(component))
                .unwrap_or(true)
            && self
                .profile_id
                .as_ref()
                .map(|id| stable_id(id))
                .unwrap_or(true)
            && self.warnings.iter().all(|warning| non_empty(warning))
    }
}

/// Broad settings category for a public shortcut descriptor.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsShortcutCategory {
    Network,
    Bluetooth,
    Display,
    Apps,
    Cast,
    Developer,
    Privacy,
    Boundary,
    #[default]
    Other,
}

/// Public descriptor for a user-visible settings front door.
///
/// Shortcuts open documented settings actions or app-owned panels. They should
/// be treated as UI navigation, not silent device-state changes.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsShortcutDescriptor {
    pub schema: String,
    pub shortcut_id: String,
    pub label: String,
    pub android_action: String,
    pub category: SettingsShortcutCategory,
    pub requires_confirmation: bool,
    pub requires_helper: bool,
    pub warning: Option<String>,
}

impl SettingsShortcutDescriptor {
    pub fn new(
        shortcut_id: impl Into<String>,
        label: impl Into<String>,
        android_action: impl Into<String>,
        category: SettingsShortcutCategory,
    ) -> Self {
        Self {
            schema: HOME_SETTINGS_SHORTCUT_SCHEMA.to_string(),
            shortcut_id: shortcut_id.into(),
            label: label.into(),
            android_action: android_action.into(),
            category,
            requires_confirmation: false,
            requires_helper: false,
            warning: None,
        }
    }

    pub const fn requiring_confirmation(mut self) -> Self {
        self.requires_confirmation = true;
        self
    }

    pub const fn requiring_helper(mut self) -> Self {
        self.requires_helper = true;
        self
    }

    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warning = Some(warning.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        self.schema == HOME_SETTINGS_SHORTCUT_SCHEMA
            && stable_id(&self.shortcut_id)
            && non_empty(&self.label)
            && android_action_like(&self.android_action)
            && self
                .warning
                .as_ref()
                .map(|warning| non_empty(warning))
                .unwrap_or(true)
    }
}

/// Optional helper status as reported to a broker or home shell.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HomeHelperState {
    pub connected: bool,
    pub uid_label: Option<String>,
    pub capabilities: Vec<String>,
    pub last_heartbeat_elapsed_ns: Option<u64>,
}

impl HomeHelperState {
    pub fn disconnected() -> Self {
        Self::default()
    }

    pub fn connected(capabilities: Vec<String>) -> Self {
        Self {
            connected: true,
            uid_label: None,
            capabilities,
            last_heartbeat_elapsed_ns: None,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.uid_label
            .as_ref()
            .map(|label| non_empty(label))
            .unwrap_or(true)
            && self
                .capabilities
                .iter()
                .all(|capability| stable_id(capability))
    }
}

/// Bounded developer supervisor policy.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HomeSupervisorPolicy {
    #[default]
    Disabled,
    ObserveOnly,
    ReturnToBrokerAfterLimbo,
    ReturnToTargetAfterHome,
    GuardedDemoSession,
    ManagedDevicePolicy,
}

/// Current developer supervisor state.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HomeSupervisorState {
    pub enabled: bool,
    pub policy: HomeSupervisorPolicy,
    pub max_attempts: u32,
    pub cooldown_ms: u32,
    pub attempt_count: u32,
    pub last_event_id: Option<String>,
}

impl HomeSupervisorState {
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            policy: HomeSupervisorPolicy::Disabled,
            max_attempts: 0,
            cooldown_ms: 0,
            attempt_count: 0,
            last_event_id: None,
        }
    }

    pub fn observe_only() -> Self {
        Self {
            enabled: true,
            policy: HomeSupervisorPolicy::ObserveOnly,
            max_attempts: 0,
            cooldown_ms: 0,
            attempt_count: 0,
            last_event_id: None,
        }
    }

    pub fn bounded(policy: HomeSupervisorPolicy, max_attempts: u32, cooldown_ms: u32) -> Self {
        Self {
            enabled: !matches!(policy, HomeSupervisorPolicy::Disabled),
            policy,
            max_attempts,
            cooldown_ms,
            attempt_count: 0,
            last_event_id: None,
        }
    }

    pub fn is_valid(&self) -> bool {
        if matches!(self.policy, HomeSupervisorPolicy::Disabled) {
            return !self.enabled && self.max_attempts == 0 && self.attempt_count == 0;
        }

        self.enabled
            && self.attempt_count <= self.max_attempts
            && self
                .last_event_id
                .as_ref()
                .map(|event_id| stable_id(event_id))
                .unwrap_or(true)
    }
}

impl Default for HomeSupervisorState {
    fn default() -> Self {
        Self::disabled()
    }
}

/// Last requested external app launch from a home surface.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalLaunchState {
    pub package_name: String,
    pub launch_mode: String,
    pub requested_at_unix_ms: Option<u64>,
    pub observed_foreground: Option<String>,
}

impl ExternalLaunchState {
    pub fn new(package_name: impl Into<String>, launch_mode: impl Into<String>) -> Self {
        Self {
            package_name: package_name.into(),
            launch_mode: launch_mode.into(),
            requested_at_unix_ms: None,
            observed_foreground: None,
        }
    }

    pub fn is_valid(&self) -> bool {
        package_like(&self.package_name)
            && stable_id(&self.launch_mode)
            && self
                .observed_foreground
                .as_ref()
                .map(|value| non_empty(value))
                .unwrap_or(true)
    }
}

/// Public state snapshot for a broker or immersive home session.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HomeSessionState {
    pub schema: String,
    pub mode: HomeMode,
    pub active_panels: Vec<String>,
    pub last_external_launch: Option<ExternalLaunchState>,
    pub helper: HomeHelperState,
    pub supervisor: HomeSupervisorState,
}

impl HomeSessionState {
    pub fn new(mode: HomeMode) -> Self {
        Self {
            schema: HOME_SESSION_STATE_SCHEMA.to_string(),
            mode,
            active_panels: Vec::new(),
            last_external_launch: None,
            helper: HomeHelperState::default(),
            supervisor: HomeSupervisorState::default(),
        }
    }

    pub fn with_active_panel(mut self, panel_id: impl Into<String>) -> Self {
        self.active_panels.push(panel_id.into());
        self
    }

    pub fn with_helper(mut self, helper: HomeHelperState) -> Self {
        self.helper = helper;
        self
    }

    pub fn with_supervisor(mut self, supervisor: HomeSupervisorState) -> Self {
        self.supervisor = supervisor;
        self
    }

    pub fn with_last_external_launch(mut self, launch: ExternalLaunchState) -> Self {
        self.last_external_launch = Some(launch);
        self
    }

    pub fn panel_is_active(&self, panel_id: &str) -> bool {
        self.active_panels.iter().any(|active| active == panel_id)
    }

    pub fn is_valid(&self) -> bool {
        self.schema == HOME_SESSION_STATE_SCHEMA
            && self
                .active_panels
                .iter()
                .all(|panel_id| stable_id(panel_id))
            && self
                .last_external_launch
                .as_ref()
                .map(ExternalLaunchState::is_valid)
                .unwrap_or(true)
            && self.helper.is_valid()
            && self.supervisor.is_valid()
    }
}

/// Focus-recovery action recorded by developer supervisor mode.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FocusRecoveryAction {
    #[default]
    Observe,
    ReturnToBroker,
    ReturnToTarget,
    OpenSystemPanel,
    StopSupervisor,
}

/// Focus-recovery result recorded by developer supervisor mode.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FocusRecoveryResult {
    #[default]
    NotAttempted,
    Started,
    Succeeded,
    Failed,
    SkippedProtectedPrompt,
    CooldownActive,
    MaxAttemptsReached,
}

/// Structured event for bounded focus recovery.
///
/// This records actions after focus transitions are observed. It does not
/// describe Home/Menu interception or protected system prompt dismissal.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusRecoveryEvent {
    pub schema: String,
    pub event_id: String,
    pub policy: HomeSupervisorPolicy,
    pub action: FocusRecoveryAction,
    pub result: FocusRecoveryResult,
    pub reason: String,
    pub previous_foreground: Option<String>,
    pub requested_target: Option<String>,
    pub attempt_count: u32,
    pub event_time_unix_ms: Option<u64>,
}

impl FocusRecoveryEvent {
    pub fn new(
        event_id: impl Into<String>,
        policy: HomeSupervisorPolicy,
        action: FocusRecoveryAction,
        result: FocusRecoveryResult,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            schema: HOME_FOCUS_RECOVERY_EVENT_SCHEMA.to_string(),
            event_id: event_id.into(),
            policy,
            action,
            result,
            reason: reason.into(),
            previous_foreground: None,
            requested_target: None,
            attempt_count: 0,
            event_time_unix_ms: None,
        }
    }

    pub fn with_previous_foreground(mut self, previous_foreground: impl Into<String>) -> Self {
        self.previous_foreground = Some(previous_foreground.into());
        self
    }

    pub fn with_requested_target(mut self, requested_target: impl Into<String>) -> Self {
        self.requested_target = Some(requested_target.into());
        self
    }

    pub const fn with_attempt_count(mut self, attempt_count: u32) -> Self {
        self.attempt_count = attempt_count;
        self
    }

    pub const fn with_event_time_unix_ms(mut self, event_time_unix_ms: u64) -> Self {
        self.event_time_unix_ms = Some(event_time_unix_ms);
        self
    }

    pub fn is_valid(&self) -> bool {
        self.schema == HOME_FOCUS_RECOVERY_EVENT_SCHEMA
            && stable_id(&self.event_id)
            && non_empty(&self.reason)
            && self
                .previous_foreground
                .as_ref()
                .map(|foreground| non_empty(foreground))
                .unwrap_or(true)
            && self
                .requested_target
                .as_ref()
                .map(|target| non_empty(target))
                .unwrap_or(true)
    }
}

fn size_range_valid(default_size_m: Vec2, min_size_m: Vec2, max_size_m: Vec2) -> bool {
    default_size_m.is_finite()
        && min_size_m.is_finite()
        && max_size_m.is_finite()
        && min_size_m.x > 0.0
        && min_size_m.y > 0.0
        && max_size_m.x >= min_size_m.x
        && max_size_m.y >= min_size_m.y
        && default_size_m.x >= min_size_m.x
        && default_size_m.y >= min_size_m.y
        && default_size_m.x <= max_size_m.x
        && default_size_m.y <= max_size_m.y
}

fn helper_only_command(command: &str) -> bool {
    command == "launcher.force_stop"
        || command == "launcher.start_component"
        || command == "system.get_foreground"
        || command == "system.get_panel_state"
        || command.starts_with("guardian.")
        || command.starts_with("logs.")
        || command.starts_with("system.capture_")
}

fn stable_id(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':' | '/' | '+'))
}

fn package_like(value: &str) -> bool {
    let value = value.trim();
    value.contains('.') && stable_id(value)
}

fn android_action_like(value: &str) -> bool {
    let value = value.trim();
    value.starts_with("android.settings.") && stable_id(value)
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_descriptor_validates_size_and_helper_boundary() {
        let launcher = HomePanelDescriptor::new("launcher", "Launcher", HomePanelKind::BrokerPage)
            .with_command("launcher.list")
            .with_command("launcher.start_front_door");

        assert!(launcher.is_valid());
        assert!(!launcher.uses_helper_only_commands());

        let unsafe_panel = HomePanelDescriptor::new("launch", "Launch", HomePanelKind::BrokerPage)
            .with_command("launcher.force_stop");
        assert!(!unsafe_panel.is_valid());

        let helper_panel = unsafe_panel.requiring_helper();
        assert!(helper_panel.is_valid());
        assert!(helper_panel.uses_helper_only_commands());
    }

    #[test]
    fn launcher_entry_does_not_require_helper_for_front_door_launch() {
        let entry = LauncherEntry::new("org.example.target", "Target App")
            .with_profile_id("demo.profile")
            .with_warning("Launch may transfer focus away from the home shell.");

        assert!(entry.is_valid());
        assert!(!entry.requires_helper);
    }

    #[test]
    fn settings_shortcut_requires_documented_settings_action_shape() {
        let shortcut = SettingsShortcutDescriptor::new(
            "wifi",
            "Wi-Fi",
            "android.settings.WIFI_SETTINGS",
            SettingsShortcutCategory::Network,
        );
        let invalid = SettingsShortcutDescriptor::new(
            "wifi",
            "Wi-Fi",
            "com.example.PRIVATE_SETTINGS",
            SettingsShortcutCategory::Network,
        );

        assert!(shortcut.is_valid());
        assert!(!invalid.is_valid());
    }

    #[test]
    fn session_state_tracks_panels_helper_and_supervisor() {
        let helper = HomeHelperState::connected(vec![
            "launcher.start_component".to_string(),
            "guardian.configure_mode".to_string(),
        ]);
        let supervisor =
            HomeSupervisorState::bounded(HomeSupervisorPolicy::ReturnToBrokerAfterLimbo, 3, 1_000);
        let state = HomeSessionState::new(HomeMode::DeveloperSupervisor)
            .with_active_panel("launcher")
            .with_active_panel("diagnostics")
            .with_helper(helper)
            .with_supervisor(supervisor)
            .with_last_external_launch(ExternalLaunchState::new(
                "org.example.target",
                "package_manager",
            ));

        assert!(state.is_valid());
        assert!(state.panel_is_active("diagnostics"));
        assert!(!state.panel_is_active("streams"));
    }

    #[test]
    fn focus_recovery_event_records_reactive_action() {
        let event = FocusRecoveryEvent::new(
            "event-001",
            HomeSupervisorPolicy::ReturnToBrokerAfterLimbo,
            FocusRecoveryAction::ReturnToBroker,
            FocusRecoveryResult::Succeeded,
            "observed focus loss",
        )
        .with_previous_foreground("system.home")
        .with_requested_target("broker")
        .with_attempt_count(1);

        assert!(event.is_valid());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn home_session_round_trips_with_serde() {
        let state = HomeSessionState::new(HomeMode::ImmersivePassthrough)
            .with_active_panel("launcher")
            .with_active_panel("system");

        let encoded = serde_json::to_string(&state).expect("state should serialize");
        let decoded: HomeSessionState =
            serde_json::from_str(&encoded).expect("state should deserialize");

        assert_eq!(decoded, state);
    }
}
