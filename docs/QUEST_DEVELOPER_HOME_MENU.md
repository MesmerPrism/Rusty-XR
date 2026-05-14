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
helper-aware session state, and one observe-only recovery event. It does not
build an APK, launch a headset app, start ADB, or use a device.

## Schema Export

The hand-reviewed schema exporter includes:

- `home-panel-descriptor.schema.json`
- `home-session-state.schema.json`
- `home-launcher-entry.schema.json`
- `home-settings-shortcut.schema.json`
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

The home contracts give those docs a shared data shape without moving platform
behavior into the public core.
