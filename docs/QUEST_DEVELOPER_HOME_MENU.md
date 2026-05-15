# Quest Developer Home Menu Contracts

Rusty XR now includes public, framework-neutral contracts for Rusty Kiosk, a
developer home or launcher surface. These contracts are deliberately data-only.
They can be used by a normal 2D broker console, an app-owned immersive home
shell, a companion app, or a downstream app that wants to publish panel
metadata.
The broker clock/timebase API can provide a Clock panel and shared timestamp
source for those surfaces; see [BROKER_CLOCK_AND_TIMEBASE.md](BROKER_CLOCK_AND_TIMEBASE.md).

They do not make a normal APK into system UI, device-owner policy, or an ADB
shell process.

In local development, Rusty Kiosk is the intended operator baseline: a stable
place to launch targets, inspect broker state, view the Clock panel, and record
unexpected focus or system-panel events. Public contracts still avoid claiming
system Home/Menu interception or managed-kiosk privileges.

Rusty Kiosk also owns the local tracking setup for Quest development signals.
That does not mean fused pose tracking. It means tracking which surface the
headset is actually showing, which command path changed it, whether a Meta
system menu was opened intentionally, and which provider evidence supports the
decision. The custom Rusty XR menu is the default operator surface; entering
Meta Home/Menu/settings should be an explicit, logged intent unless the test is
about raw Meta shell behavior.

## Layered Shape

Use four separate layers:

```text
2D broker console
  normal PackageManager launcher, settings front doors, diagnostics

own immersive home
  app-owned OpenXR scene, passthrough or virtual background, app-owned panels

developer supervisor
  optional external helper state and bounded focus-recovery event logging

managed kiosk
  separate MDM or managed-device route when policy-level lock-down is required
```

The contracts live in `rusty-xr-contracts::home` and are re-exported from the
crate root.

## Contract Groups

- `HomePanelDescriptor`: describes a broker page, local applet, cooperating app
  status panel, remote surface, settings shortcut, or diagnostics panel.
- `HomeSessionState`: describes current mode, active panel ids, helper state,
  supervisor state, and the last external launch request.
- `LauncherEntry`: describes a public-safe launch row for an installed or
  cataloged target app.
- `SettingsShortcutDescriptor`: describes a documented settings front door.
- `FocusRecoveryEvent`: records bounded recovery actions after focus loss is
  observed.

The helper boundary is explicit. A panel that declares helper-only commands such
as `launcher.force_stop`, `guardian.configure_mode`, or `system.get_foreground`
must set `requires_helper=true` to validate.

## Provider Evidence Loop

For each launch, recovery, settings shortcut, focus-recovery attempt, or
unexpected Meta shell transition, Rusty Kiosk run records should include:

- command goal and provider kind (`hzdb`, ADB, Companion, broker, helper, or
  manual)
- broker `/status`, `/clock/now`, `/clock/health`, and `clock_epoch_id` when
  available
- foreground app or panel before and after the operation
- whether the resulting surface is `rusty_kiosk_default`, `rusty_xr_target`,
  `meta_panel_intentional`, `meta_panel_unexpected`, or `unknown_surface`
- screenshot/capture method for visual witness when the state is ambiguous
- fallback command path if the preferred provider is unavailable

The command taxonomy, safety gates, and `hzdb`/ADB fallback table live in
[META_QUEST_HZDB_PROVIDER_PLAN.md](META_QUEST_HZDB_PROVIDER_PLAN.md). Rusty
Kiosk should use that provider plan as a core part of learning which Quest
signals are reliable enough for a custom Rusty XR development environment.

## Camera Readiness Gate

Rusty Kiosk can keep broker status, clock, and launch controls available while
the headset is still not ready to deliver raw camera frames. Treat broker health
and app foreground as separate from Camera2/PCA readiness.

Before camera validation after sleep, standby, or sensor-lock transitions,
collect passive evidence first: power state, foreground, broker status/clock,
tracking readiness when an operator is wearing the headset, and recent camera
log markers. Do not toggle stay-awake or proximity state as routine cleanup.
Only use proximity holds, sensor-lock overrides, or stay-awake commands when the
run explicitly requests that device-setting change.

The first camera proof should be a simple direct camera path with frame-counter
progression and visible camera content. Broker-camera, codec, or downstream
rendering lanes should be tested after that baseline is known good.

## Non-Goals

These contracts do not:

- intercept Home/Menu before the platform handles it
- suppress Guardian, permissions, account, Store, package installer, boundary,
  health, privacy, or safety UI
- install arbitrary APKs
- enable Developer Mode, ADB authorization, or Wi-Fi ADB
- host arbitrary non-cooperating Android apps as live controlled windows
- provide MDM/device-owner kiosk guarantees
- create OpenXR, Android, Vulkan, Makepad, Unity, or Meta SDK objects

## Example

Run the synthetic manifest example:

```powershell
cargo run -p rusty-xr-contracts --example developer_home_manifest --features serde
```

The example emits a manifest with launcher, system, clock, and diagnostics
panels, a normal target-app launcher entry, two settings shortcuts, a
helper-aware session state, a Rusty Kiosk control-plane snapshot, and one
observe-only recovery event. It does not build an APK, launch a headset app,
start ADB, or use a device.

## Broker Status Surface

The Quest broker APK now reports the current Rusty Kiosk control-plane phase in
its normal status payload:

- `GET /status` includes a `rustyKiosk` object.
- `GET /kiosk/status` returns the Rusty Kiosk object directly.
- WebSocket command `kiosk.get_status` returns the same object.
- stream `kiosk:control_plane` is broadcast when shell-helper or experiment
  control state changes.

The current implementation reports `BrokerPanel2d` or
`BrokerPanelWithShellHelper`. It intentionally reports
`immersive_home_visible=false` until an app-owned passthrough/full-XR home shell
is actually active.

## Schema Export

The hand-reviewed schema exporter includes:

- `home-panel-descriptor.schema.json`
- `home-session-state.schema.json`
- `home-launcher-entry.schema.json`
- `home-settings-shortcut.schema.json`
- `home-kiosk-control-plane-status.schema.json`
- `home-focus-recovery-event.schema.json`

Run:

```powershell
python tools/schema/export_schemas.py --check
```

## Relationship To Existing Quest Docs

Read these docs together:

- `QUEST_APP_LAUNCHING_AND_SHELL_HELPERS.md`: normal PackageManager launcher
  versus optional external helper.
- `QUEST_DISTRIBUTION_AND_ADB_BOUNDARY.md`: Store-style, SideQuest/GitHub,
  lab, Developer Mode, external ADB host, and shell-helper split.
- `RUSTY_XR_COMPANION_INTEGRATION.md`: companion-owned install, launch,
  catalog, helper lifecycle, and diagnostics routing.
- `MEDIA_PIPELINE_AND_PERMISSIONS.md`: permission, camera, MediaProjection, and
  media-source boundaries.
- `BROKER_CLOCK_AND_TIMEBASE.md`: broker-owned elapsed-realtime clock,
  cross-app clock queries, stream stamps, and OpenXR timeline comparison
  boundary.
- `META_QUEST_HZDB_PROVIDER_PLAN.md`: `hzdb`/ADB/MCP provider commands, safety
  classes, signal tracking labels, and Rusty Kiosk command-goal evidence.

The home contracts give those docs a shared data shape without moving platform
behavior into the public core.
