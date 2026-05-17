use rusty_xr_contracts::{
    KioskCommandEvidence, KioskCommandOutcome, KioskCommandProvider, KioskCommandRunRecord,
    KioskControlPlaneStatus, KioskSurfaceIntent,
};

fn main() {
    let primary = KioskCommandEvidence::new("surface.current", KioskCommandProvider::HzdbMcp)
        .with_preferred_command("mcp:meta-horizon/app.foreground")
        .with_foreground_before("unknown")
        .with_foreground_after("org.example.rustyxr.broker/.MainActivity")
        .with_clock_epoch_id("clock.epoch.demo")
        .with_note("read_only_status_probe");

    let fallback = KioskCommandEvidence::new("surface.current", KioskCommandProvider::Broker)
        .with_preferred_command("GET /kiosk/status")
        .with_fallback_command("adb shell dumpsys window")
        .with_foreground_after("org.example.rustyxr.broker/.MainActivity")
        .with_clock_epoch_id("clock.epoch.demo");

    let before = KioskControlPlaneStatus::broker_panel_2d()
        .with_surface_intent(KioskSurfaceIntent::UnknownSurface);
    let after = KioskControlPlaneStatus::broker_panel_2d()
        .with_surface_intent(KioskSurfaceIntent::RustyKioskDefault)
        .with_clock_epoch_id("clock.epoch.demo")
        .with_latest_command(fallback.clone());

    let record = KioskCommandRunRecord::new("run-001", "surface.current", primary)
        .with_surface_intent(KioskSurfaceIntent::RustyKioskDefault)
        .with_fallback(fallback)
        .with_status_before(before)
        .with_status_after(after)
        .with_outcome(KioskCommandOutcome::Succeeded)
        .with_note("single public record for MCP, broker API, CLI, and ADB fallback evidence");

    assert!(record.is_valid(), "invalid kiosk command run record");

    println!(
        "{}",
        serde_json::to_string_pretty(&record).expect("record should serialize")
    );
}
