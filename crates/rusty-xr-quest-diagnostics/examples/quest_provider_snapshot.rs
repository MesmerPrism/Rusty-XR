use rusty_xr_quest_diagnostics::{
    DeviceHealth, DeviceReadiness, ForegroundApp, McpServerConfig, ProviderCapability,
    ProviderOperationSafety, QuestDevelopmentProviderSnapshot, QuestToolProviderKind,
};

fn main() {
    let snapshot = QuestDevelopmentProviderSnapshot {
        provider: QuestToolProviderKind::HzdbCli,
        version: Some(String::from("1.x")),
        capabilities: vec![
            ProviderCapability::hzdb(
                "device.health-check",
                "device",
                ProviderOperationSafety::ReadOnly,
            ),
            ProviderCapability::hzdb(
                "perf.capture",
                "perf",
                ProviderOperationSafety::BoundedCapture,
            ),
            ProviderCapability::hzdb("shell", "shell", ProviderOperationSafety::ShellCommand),
        ],
        device_health: Some(DeviceHealth {
            provider: QuestToolProviderKind::HzdbCli,
            connected: true,
            readiness: DeviceReadiness::AppVisible,
            battery_level_percent: Some(80),
            storage_available_bytes: Some(8 * 1024 * 1024 * 1024),
            controller_count: 2,
            ui_ready: true,
            issues: Vec::new(),
        }),
        foreground_app: Some(ForegroundApp {
            package_name: Some(String::from("com.example.public")),
            activity_name: Some(String::from(".MainActivity")),
            process_id: Some(1234),
            source: String::from("provider.foreground"),
        }),
        mcp: Some(McpServerConfig::hzdb_stdio_command(
            "<mqdh-hzdb-executable>",
            false,
        )),
        notes: vec![String::from(
            "Side-effecting provider operations should be operator-gated.",
        )],
    };

    println!("{snapshot:#?}");
}
