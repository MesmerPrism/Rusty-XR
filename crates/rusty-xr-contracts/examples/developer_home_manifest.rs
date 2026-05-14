use rusty_xr_contracts::{
    ExternalLaunchState, FocusRecoveryAction, FocusRecoveryEvent, FocusRecoveryResult,
    HomeHelperState, HomeMode, HomePanelDescriptor, HomePanelKind, HomePanelPlacement,
    HomeSessionState, HomeSupervisorPolicy, HomeSupervisorState, LauncherEntry,
    SettingsShortcutCategory, SettingsShortcutDescriptor, Vec2,
};

#[derive(serde::Serialize)]
struct DeveloperHomeManifest {
    panels: Vec<HomePanelDescriptor>,
    launcher_entries: Vec<LauncherEntry>,
    settings_shortcuts: Vec<SettingsShortcutDescriptor>,
    session: HomeSessionState,
    recovery_events: Vec<FocusRecoveryEvent>,
}

fn main() {
    let panels = vec![
        HomePanelDescriptor::new("launcher", "Launcher", HomePanelKind::BrokerPage)
            .with_command("launcher.list")
            .with_command("launcher.start_front_door")
            .with_placement(HomePanelPlacement::HeadLocked),
        HomePanelDescriptor::new("system", "System", HomePanelKind::SettingsShortcut)
            .with_command("system.open_settings")
            .with_placement(HomePanelPlacement::WorldLocked),
        HomePanelDescriptor::new("clock", "Clock", HomePanelKind::LocalApplet)
            .with_command("clock.status")
            .with_command("clock.now")
            .with_command("clock.health")
            .with_command("clock.compare_openxr")
            .with_size_bounds(
                Vec2::new(0.54, 0.34),
                Vec2::new(0.32, 0.22),
                Vec2::new(0.90, 0.58),
            )
            .with_placement(HomePanelPlacement::HandAnchored),
        HomePanelDescriptor::new("diagnostics", "Diagnostics", HomePanelKind::Diagnostic)
            .with_command("system.get_foreground")
            .with_command("guardian.configure_mode")
            .with_size_bounds(
                Vec2::new(0.64, 0.40),
                Vec2::new(0.35, 0.24),
                Vec2::new(1.10, 0.75),
            )
            .requiring_helper(),
    ];

    let launcher_entries = vec![LauncherEntry::new("org.example.target", "Target App")
        .with_profile_id("demo.profile")
        .with_warning("Launching this app transfers focus away from the home shell.")];

    let settings_shortcuts = vec![
        SettingsShortcutDescriptor::new(
            "wifi",
            "Wi-Fi",
            "android.settings.WIFI_SETTINGS",
            SettingsShortcutCategory::Network,
        ),
        SettingsShortcutDescriptor::new(
            "apps",
            "App details",
            "android.settings.APPLICATION_DETAILS_SETTINGS",
            SettingsShortcutCategory::Apps,
        )
        .requiring_confirmation()
        .with_warning("This opens system UI; the home shell should observe and recover only after user action."),
    ];

    let session = HomeSessionState::new(HomeMode::DeveloperSupervisor)
        .with_active_panel("launcher")
        .with_active_panel("system")
        .with_active_panel("clock")
        .with_active_panel("diagnostics")
        .with_helper(HomeHelperState::connected(vec![
            "system.get_foreground".to_string(),
            "guardian.configure_mode".to_string(),
        ]))
        .with_supervisor(HomeSupervisorState::bounded(
            HomeSupervisorPolicy::ReturnToBrokerAfterLimbo,
            3,
            1_000,
        ))
        .with_last_external_launch(ExternalLaunchState::new(
            "org.example.target",
            "package_manager",
        ));

    let recovery_events = vec![FocusRecoveryEvent::new(
        "event-001",
        HomeSupervisorPolicy::ReturnToBrokerAfterLimbo,
        FocusRecoveryAction::Observe,
        FocusRecoveryResult::NotAttempted,
        "focus recovery is configured but this synthetic example does not launch apps",
    )];

    for panel in &panels {
        assert!(panel.is_valid(), "invalid panel: {}", panel.panel_id);
    }
    for entry in &launcher_entries {
        assert!(
            entry.is_valid(),
            "invalid launcher entry: {}",
            entry.package_name
        );
    }
    for shortcut in &settings_shortcuts {
        assert!(
            shortcut.is_valid(),
            "invalid settings shortcut: {}",
            shortcut.shortcut_id
        );
    }
    assert!(session.is_valid(), "invalid home session");
    for event in &recovery_events {
        assert!(
            event.is_valid(),
            "invalid recovery event: {}",
            event.event_id
        );
    }

    let manifest = DeveloperHomeManifest {
        panels,
        launcher_entries,
        settings_shortcuts,
        session,
        recovery_events,
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&manifest).expect("manifest should serialize")
    );
}
