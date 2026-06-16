# Meta Quest Meta VR CLI / hzdb Provider Plan

This note folds the May 13, 2026 Meta Horizon OS developer-tooling update into
Rusty XR. It keeps the focus on development workflows, not distribution,
marketing, or store-growth guidance.

Sources:

- Meta developer blog:
  <https://developers.meta.com/horizon/blog/build-faster-and-earn-more-helping-VR-developers-succeed-on-meta-quest/>
- Meta AI tooling / MCP documentation:
  <https://developers.meta.com/horizon/essentials/ai-tooling-mcp/>
- Meta Quest agentic-tools and Meta VR CLI reference:
  <https://github.com/meta-quest/agentic-tools>
- Meta XR Unity MCP Extension:
  <https://developers.meta.com/horizon/documentation/unity/unity-mcp-extension/>
- Unity Project Setup Tool:
  <https://developers.meta.com/horizon/documentation/unity/unity-upst-overview/>

## Development Signal

Meta's current public development-tooling route is **Meta VR CLI** through the
`metavr` npm package. The CLI currently exposes Horizon Debug Bridge (`hzdb`)
commands and MQDH/editor bundles may still ship a `hzdb` executable, so Rusty
XR keeps `hzdb` as a compatibility/provider label while new manual setup
examples use `npx -y metavr`. The tooling overlaps with ADB-style device
operations, but it also adds Quest-specific documentation search, app and
device management, file transfer, screenshots, Perfetto capture/analysis, and
AI-agent access through MCP.

Rusty XR should treat `hzdb` as a vendor-specific provider, not as core
protocol. The public Rusty XR core owns data contracts, safety policy,
diagnostic manifests, and provider-neutral report shapes. Companion apps,
shell helpers, or local operator tools own actual `hzdb` invocation and fallback
to ADB when `hzdb` is missing or unsuitable.

For Rusty Kiosk, this provider loop is core workflow. The `hzdb` executable is
optional, but the habit is not: every Kiosk/headset transition should record
which provider command was used, what device signal it read, whether the target
state was Rusty Kiosk, a Rusty XR app, or an intentionally opened Meta panel,
and which fallback command should reproduce the observation.

On developer machines that use Meta Quest Developer Hub, MQDH may provide the
`hzdb` executable and the Codex MCP route. Treat that as operator/tooling state:
record the MCP server name, provider route, provider version, and tool-count
probe in the run evidence or local graph inventory. Do not encode a local MQDH
install path in public Rusty XR contracts.

Horizon OS 2.x changes the evidence context for Kiosk/headset runs. Record the
exact OS/PTC state, Navigator/Home state, restored or snapped panels, privacy
indicators, and intentional Meta system-panel transitions before interpreting a
screenshot, foreground-app result, or launch/return failure as Rusty XR
behavior.

## Already Present

Rusty XR already has partial `hzdb` overlap:

- Companion tooling cache documentation already lists `adb`, `hzdb`, and
  `scrcpy` as managed operator-side tools.
- Quest camera profile tooling can use `hzdb` for screenshot/proximity flows.
- Render-artifact diagnostics already prefer `hzdb perf` for Perfetto capture
  when companion tooling is available.
- Machine-wide tooling audit already identified selector-aware `hzdb`/ADB
  command contracts, screenshot/perf/wake/proximity fallbacks, and file
  transfer as reusable public candidates.
- `rusty-xr-quest-diagnostics` now models provider capabilities, device health,
  controller info, app/foreground state, log filters, screenshot captures, file
  operations, Perfetto sessions/metrics, docs/API/asset search results, MCP
  server config, and agent-skill metadata. The stable provider snapshot schema
  is exported as `quest-development-provider-snapshot.schema.json`.

That means the next useful work is parity and orchestration, not discovery.

## Provider Boundary

Core may model:

- provider capability manifests
- safe operation classes and gate requirements
- device health/readiness snapshots
- app lifecycle and foreground snapshots
- file-operation plans
- log filters
- screenshot capture manifests
- Perfetto trace sessions, extracted metrics, and comparison reports
- Quest documentation/API search results
- optional asset-search results for prototyping
- MCP server launch descriptors
- agent-skill metadata and local policy notes

Core must not:

- vendor Meta VR CLI / `hzdb`, ADB, Meta SDK binaries, Unity packages, or
  generated tool caches
- run device shell commands from core crates
- make `hzdb` required for Rusty XR consumers
- let an MCP server mutate a project or headset without explicit safety gates
- copy Meta agentic skill bodies into the repository

## Safety Classes

Use `rusty-xr-quest-diagnostics::ProviderOperationSafety` to plan operations.

| Safety class | Default handling |
| --- | --- |
| `ReadOnly` | Allowed for reports and planning. |
| `BoundedCapture` | Allowed when artifacts stay ignored and the headset/device resource is reserved by the operator workflow. |
| `FileRead` | Allowed with explicit source and destination paths. |
| `FileWrite`, `FileDelete` | Require an operator gate and dry-run support where practical. |
| `AppLifecycle` | Require operator gate when installing, clearing, stopping, or foregrounding apps. |
| `DeviceSetting` | Require operator gate, bounded duration, and restore notes. |
| `ShellCommand`, `NetworkForward`, `Root` | Require explicit operator gate and audit log. |

MCP should expose read-only status and planning tools first. Side-effecting MCP
calls should return a blocked plan unless the local operator shell has already
granted that operation family.

## Camera Readiness Signals

Camera acquisition runs need a stricter readiness gate than ordinary app
launches. Display-on, wakefulness, headset-mounted, ADB reachability, and a
foreground OpenXR app are necessary context, but they do not by themselves prove
that Camera2/PCA frames will flow. Recent Quest validation showed a state where
the screen and app launch path were usable while raw camera delivery did not
recover until the operator brought the headset fully back through the normal
system power-menu path.

Public diagnostics should therefore separate these signals:

- `display_ready`: screen/wakefulness/headset-mounted state.
- `app_launch_ready`: ADB or provider can foreground the target app.
- `broker_ready`: broker status and clock endpoints are reachable.
- `tracking_ready`: operator or provider evidence shows normal hand/controller
  tracking is active.
- `camera_ready`: Camera2/PCA frame counters advance and visible ROIs or an
  operator witness confirm live camera content.

Use explicit failure labels when these layers diverge. For example,
`display_awake_not_xr_ready` means the screen, ADB path, or app launch path may
work while tracking or camera services are still unavailable. A protected
system prompt or sensor-lock surface should be recorded as a system-readiness
blocker, not as target-app or camera-pipeline evidence. If the headset appears
awake but the tracking/camera indicator is off, hands/controllers do not track,
or provider evidence shows no camera clients, recover the XR runtime first and
rerun the camera stage.

Provider tools should preserve power, stay-awake, and proximity state by
default. `hzdb device proximity`, `configure-testing`, `adb shell svc power
stayon`, sensor-lock overrides, and automatic restore commands are
`DeviceSetting` operations: require explicit run intent and log the previous and
final states. A camera run that needs such a change should say so up front; a
run that only needs observation should use passive readback.

Host-launched shell-helper watchdogs are developer tools, not headset boot
services. They should be treated as lost after a headset reboot. Restart the
helper from an authorized host and re-check `display_ready`, `tracking_ready`,
and `camera_ready` before using post-reboot camera results.

Boot-autostart probes should be classified separately from XR readiness. A
normal debug helper can receive `BOOT_COMPLETED`, hold a bounded app wake lock,
launch a broker, and then launch an XR activity when app-visible broker status
is ready. That proves only `app_launch_ready`. It does not force Quest
proximity/mounted state, keep an off-face headset awake beyond platform policy,
or recover shell-helper authority after reboot. Provider readiness should keep
`sys.hmt.mounted`, power wakefulness, virtual proximity, tracking, and camera
frame counters as distinct evidence fields.

When camera readiness is uncertain, start with a direct Camera2/HWB profile,
then move to broker-camera, SurfaceTexture/OES, CPU-YUV, or other codec lanes.
Do not use broker clock health, app foreground, screenshots, or passthrough
background imagery as substitutes for live camera-frame progression.

When a camera-ready run falls into standby, provider tooling should capture the
cause side as well as the failed camera retry. Preserve pre/mid/post
`dumpsys power`, VR power-manager dumps, foreground/focus, broker clock/status,
and recent logcat. A useful standby signature is VR power-manager events such
as `setActivityMonitorState: Idle`, `onDeviceIdle` with
`mountWakelock: false`, `releasePowerStateLock: MOUNTED`,
`setVirtualProxState(DISABLED)`, `Calling goToSleep()`, and `STANDBY`. A
subsequent ADB wake can restore display/app launch while leaving Camera2/PCA
frame production unavailable, so provider tools should not collapse
`display_ready` and `camera_ready`.

## Implementation Plan

### P0 - Capability Probe

Add a companion/provider probe that can discover `hzdb` without making it a
core dependency:

```powershell
npx -y metavr --version
```

If MQDH is installed, an operator tool may instead probe the MQDH-bundled
`hzdb` executable or the configured Codex MCP server. The normalized result
should say which route was used: `metavr-npx`, `mqdh-bundled`, `global-path`,
`companion-managed`, or `unknown`. Provider snapshots can model direct MQDH,
manual `metavr`, or managed executable routes with
`McpServerConfig::hzdb_stdio_command(...)` without storing local paths in
public examples.

Record the result as a `QuestDevelopmentProviderSnapshot` with capabilities
for `device`, `app`, `files`, `log`, `capture`, `perf`, `docs`, `asset`, and
`mcp` groups when present. Keep ADB as the fallback provider for basic device,
app, log, file, and screenshot operations.

### P1 - Diagnostics Parity

Compare existing Companion/ADB operations against `hzdb` groups:

- `device`: list/info/connect/wait/wake/battery/controllers/health-check/
  configure-testing/proximity
- `app`: install/uninstall/list/launch/stop/clear/info/path/foreground
- `files`: list/pull/push/remove/mkdir
- `log`: lines, tag, level, PID, regex, buffer, output format, clear, follow
- `capture`: screenshot by `metacam` or `screencap`

Close gaps by adding provider-neutral operation plans and normalized results
first, then mapping them in Companion or shell-helper code.

### P2 - Perfetto Pipeline

Treat `hzdb perf` as the preferred first-pass Perfetto path when available.
Perfetto is a deep-trace tier, not a routine camera-lane gate. Use it to
calibrate and explain the lighter Rusty XR diagnostics when stale counters,
camera texture lane summaries, or effect-layer A/B runs disagree. Because trace
capture and analysis add overhead, normal HWB/OES/Makepad validation should
continue to rely on focused log markers, freshness screenshots, Meta stale
counters, and lane summary JSON.

The provider should support:

- timed capture and background start/stop
- `standard`, `gpu`, `cpu`, `lightweight`, `full`, and `custom` presets
- app package targeting
- XR runtime metrics, GPU render-stage tracing, Vulkan layer tracing, GPU
  metrics, CPU scheduling, and extended scheduling flags
- trace loading, SQL query, GPU counter extraction, thread-state inspection,
  complete analysis, trace comparison, and UI open handoff

Rusty XR report attachments should store a `PerfTraceSession`, extracted
`PerfMetric` rows, the relevant app/package/foreground state, and the broker
clock or session stamp when available. Camera-profile wrappers can also emit a
`camera-perfetto-trace-plan.schema.json` artifact before any capture to record
the provider, preset, intended use, overhead policy, raw-trace policy, and
suggested `hzdb perf` commands. Raw `.pftrace` payloads stay in ignored artifact
folders.

### P3 - Docs-First Quest Verification

Add a docs/API provider path for Quest-specific assumptions before changing:

- Meta XR / OpenXR / Spatial SDK settings
- Android panel or 2D app behavior on Horizon OS
- passthrough, camera, environment depth, or hand/controller input behavior
- Unity project setup rules
- performance capture and trace-analysis procedures

The query result shape should use `DocSearchResult` and `ApiReferenceResult`.
The workflow is "verify first, then edit"; it should not replace source-code
inspection or local tests.

### P4 - MCP Bridge

Support project-local MCP descriptors without making MCP the only path:

```json
{
  "servers": {
    "meta-horizon-mcp": {
      "command": "npx",
      "args": ["-y", "metavr", "mcp", "server"]
    }
  }
}
```

For OpenAI Codex installations configured through MQDH, the local TOML shape is
equivalent to:

```toml
[mcp_servers.meta-horizon-mcp]
command = "<mqdh-hzdb-executable>"
args = ["mcp", "server"]
```

Rusty XR should expose or consume this as an optional provider named
`meta.quest.hzdb` or `meta-horizon-mcp`. The safer architecture is:

```text
AI agent
  -> Rusty XR/Companion MCP safety layer
  -> provider operation catalog
  -> Meta VR CLI / hzdb CLI/MCP or ADB fallback
  -> Quest device
```

This keeps Rusty XR's safety policy and audit log in front of device mutation.

### P5 - Optional Unity, Android, And Asset Adapters

Keep these optional and downstream/tooling-owned:

- Meta XR Unity MCP Extension for Unity scene edits such as creating objects,
  transforms, grabbable interactions, UI interaction setup, and teleport
  hotspots. Current public release notes put Meta XR SDK 203.0 on Unity
  6000.0.66f2+ for multiple packages; downstream Unity projects should verify
  their exact package pins before treating a Quest provider issue as a Rusty XR
  contract issue.
- Unity Project Setup Tool report ingestion. Rusty XR can parse JSON setup
  reports later, but Unity Editor automation belongs in Unity project tooling.
- Android Studio plugin and Spatial Simulator workflows for Android panel app
  iteration. Physical-device validation is still required for multiple panels
  and immersive features.
- Meta Asset Library / `metavr asset search` or compatible `hzdb asset search`
  for placeholder assets. Keep it auth-aware and optional; do not vendor
  assets or generated models in core.

## Rusty Kiosk Tracking Setup

Rusty Kiosk should surface `hzdb`/ADB/provider status as operator evidence, not
hide it. The long-term workflow is to enter the Rusty XR custom menu by default
and enter Meta Home/Menu/settings only when the run explicitly requests a Meta
system surface. The provider layer is how agents distinguish those states
without guessing from one screenshot or one log line.

During headset runs:

- record provider kind and version
- record MCP server name, registration route, and tool-count probe when MCP is
  involved
- record device health before a run
- record foreground app before and after launch
- attach broker clock/status evidence when the broker is available, especially
  `/status`, `/kiosk/status`, `/clock/now`, `/clock/health`, and the
  `rustyKiosk.latest_command` fields
- capture screenshots and Perfetto traces through the provider manifest
- mark rejected runs when health, foreground app, UI readiness, or power state
  invalidates the sample

This lets Rusty Kiosk improve Quest signal mapping over time while keeping
Meta shell behavior, ADB behavior, and broker behavior distinguishable.

### Current Affordance Notes

Use these public-safe limits when interpreting provider evidence:

- A visible broker console is an app-owned operator surface. It can open a
  specific page through launch extras or broker commands, but it is still a
  normal app Activity.
- The broker System page may request documented Android settings intents such
  as general Settings, Wi-Fi, Bluetooth, or App Info. Classify the resulting
  Meta settings panel as `meta_panel_intentional` only when the run goal or
  operator action explicitly requested that transition.
- A normal app HOME-candidate or developer-home surface is not proof of
  physical Home/Menu routing. Treat Home/Menu transitions as platform-owned
  unless a separate role/default-owner validation proves otherwise for the
  target device policy.
- HOME role availability and request-intent creation are not enough to prove
  ownership. If a visible request-role flow returns canceled and the app still
  does not hold HOME, keep reboot/force-stop survival gates closed.
- When a panel launch is ambiguous, prefer broker `/status`, `/kiosk/status`,
  `/clock/health`, foreground state, and capture-method-labeled screenshots
  over a single UI-tree dump. Horizon shell placeholder or system-panel windows
  can coexist with a healthy broker service.

### Command Goals

Prefer `hzdb` where available because it names Quest-specific intent. Keep ADB
fallbacks documented so the workflow still works on machines without `hzdb`.
Use placeholders for app ids and paths in public docs.

| Goal | Preferred command shape | Fallback / companion shape | Safety |
| --- | --- | --- | --- |
| Confirm provider availability | `npx -y metavr --version` | Companion provider probe or configured binary path check | Read-only |
| Preflight headset state | `hzdb device health-check --json` | `adb devices`; `adb shell dumpsys power`; broker `/status` | Read-only |
| Check battery/controllers | `hzdb device battery --json`; `hzdb device controllers --json` | `adb shell dumpsys battery`; controller-specific app/runtime status when available | Read-only |
| Confirm current surface | `hzdb app foreground --json` | broker `GET /kiosk/status`; `adb shell dumpsys window` focused-window filter; broker `shellHelper` status | Read-only |
| Return to Rusty Kiosk | broker command/API or `hzdb app launch <broker-package>` | Companion catalog launch or `adb shell am start ...`; verify with broker `rustyKiosk.phase` | App lifecycle gate |
| Intentionally open Meta settings/menu | Rusty Kiosk settings shortcut with logged intent | documented Android settings intent or operator manual action | App lifecycle gate |
| Capture visual witness | `hzdb capture screenshot --method metacam ...` or `--method screencap` | `adb shell screencap` / Companion screenshot fallback | Bounded capture |
| Capture focused logs | `hzdb log --lines <n> --level <level> --regex <pattern>` | `adb logcat -d -v threadtime` plus local filter | Read-only |
| Pull a run artifact | `hzdb files pull <remote> <local>` | `adb pull <remote> <local>` | File read |
| Run Perfetto analysis | `hzdb perf capture ...`; `hzdb perf analyze-trace ...` | device-side `perfetto`, MQDH, Android Studio, or preserved `.pftrace` | Bounded capture |
| Change proximity/testing state | `hzdb device proximity --disable --duration-ms <ms>` or `configure-testing setup` | shell-helper watchdog or ADB setting command | Device-setting gate |
| Mutate app data or files | `hzdb app clear`; `hzdb files rm` | ADB `pm clear` / `rm` | Explicit destructive gate |

### Intent Labels

Every Rusty Kiosk run should classify UI transitions with one of these labels:

- `rusty_kiosk_default`: starting from or returning to the broker/custom menu.
- `rusty_xr_target`: foreground is an intended Rusty XR app or downstream test
  app.
- `meta_panel_intentional`: a Meta shell/settings/menu panel was opened by a
  logged shortcut or operator request.
- `meta_panel_unexpected`: Meta shell/menu took focus without a logged intent.
- `unknown_surface`: foreground/window evidence and visual evidence disagree or
  are incomplete.

Unexpected Meta panel/menu transitions should update the private signal map
before changing automation. The goal is to learn which command and status
signals are reliable enough for the custom environment path.

## Immediate Next Steps

1. Wire the new `rusty-xr-quest-diagnostics` provider models into JSON report
   output in Companion.
2. Add a Companion `hzdb probe` or provider-status command that emits
   `QuestDevelopmentProviderSnapshot`, including MQDH/Codex MCP route when
   detected.
3. Make Rusty Kiosk run manifests include provider version, command goal,
   intent label, foreground before/after, broker clock/status, and fallback
   command.
4. Thread broker `rustyKiosk` control-plane status into Companion run reports
   once Companion owns the launcher/report writer path.
5. Add provider-neutral app foreground/info/path and device health commands.
6. Attach `PerfTraceSession` and `PerfMetric` rows to Quest render and
   streaming diagnostics reports.
7. Add project-local MCP config generation or inventory as a dry-run-first
   operation.
8. Gate MCP and CLI side effects using `ProviderOperationSafety`.
