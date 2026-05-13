# Machine Repository Tooling Audit

This audit records additional reusable XR tools, contracts, diagnostics, and
workflow knowledge found across local repositories on this machine. It is
sanitized for the public Rusty XR repository.

This document is a backlog and extraction guide. It does not authorize copying
private code into Rusty XR. Prefer clean-room public contracts, schemas,
synthetic fixtures, and attribution-first documentation.

## Scope And Boundary

Public Rusty XR may receive:

- Framework-neutral data contracts and small deterministic helpers.
- Generic Quest, Android, Bluetooth, LSL, camera, depth, media, and build
  diagnostics.
- Public examples authored directly for this repository.
- Optional adapters after the public contracts are stable.

Keep out of Rusty XR:

- Private package IDs, signing data, headset serials, APK payloads, generated
  captures, local device logs, and study/session data.
- Private downstream visual-effect stacks, parity constants, and
  project-specific shader behavior.
- Private app-specific simulation logic and private biofeedback tuning.
- Meta, Unity, Android, Makepad, or third-party source code unless license and
  attribution are checked per file.
- Vendored external binaries such as `scrcpy`, `adb`, `hzdb`, `liblsl`, or
  Unity tools. Public code may check for them, download official releases where
  appropriate, or explain how to configure paths.

## Source Categories Checked

- Local Quest cast/orchestration tooling: Quest cast sessions, `scrcpy`
  process management, smoke tests, visual artifacts, and cast profile ideas.
- Local Android phone Quest companion tooling: USB-host ADB bootstrap, Wi-Fi ADB
  handoff, install/launch/file push, and diagnostics bundle analysis.
- Local Windows Quest operator tooling: `adb`/`hzdb` recovery, wake readiness,
  visual blocker states, monitoring, LSL state, and session kit docs.
- Local Unity room/depth tooling: Quest room mesh, semantic fallback geometry,
  live depth diagnostics, PCA/depth health, processed mesh caches, and scan
  fusion.
- Local Unity BLE/PCA tooling: BLE/Polar/LSL data handling, PCA/depth
  implementation pipeline, Android permission notes, CSV schemas, and
  validation tooling.
- Public/local PCA references: Camera2, MediaProjection, WebRTC,
  reconstruction, YUV conversion, and camera lifecycle examples.
- Local Rust particle/GPU experiments: low-dependency experiment boundaries,
  particle/SDF contracts, Android/OpenXR/Vulkan split, GPU buffer layouts, and
  compute pass roadmaps.
- Local runtime-profile and Quest app workflows: explicit adapter wiring, Unity
  build validation, hotload/runtime config profiles, device profiles, and
  session manifests.
- Local biofeedback and physiological-device workflows: Polar, AirRes-style
  Bluetooth, provider layering, controller label layouts, and debug HUD state
  models.
- Local Quest template repos: Meta-specific vs vendor-neutral template split and
  optional realtime media/control protocol examples.
- Bureau notes: `rust-quest-makepad-repo-family.md`,
  `source-repo-navigation.md`, `meta-passthrough-stack.md`,
  `unity-quest-repo-family.md`, and machine tooling notes.

## Highest-Priority Public Candidates

1. Quest USB/Wi-Fi ADB diagnostics schemas, failure classes, and an analyzer
   CLI with synthetic fixtures.
2. Quest tracking access boundary docs that keep foreground OpenXR pose
   sampling, Android sensors, and ADB/shell-helper diagnostics separate.
3. Quest wake, visual-readiness, foreground-activity, proximity, and blocker
   state models.
4. Selector-aware `hzdb`/ADB command contracts and documented fallback ladders
   for screenshots, perf traces, wake, proximity, and file transfer.
5. Room mesh, semantic fallback, processed scan, and environment-depth
   diagnostic contracts.
6. PCA/Camera2/MediaProjection capture lifecycle models, frame manifests, and
   permission request taxonomy.
7. Quest cast/session manifests and visual-test artifact schemas for public
   examples and regression evidence.
8. Runtime config, hotload profile, device profile, and session manifest
   schemas usable by Rust shells, Unity shells, and Windows operators.
9. BLE/LSL/Polar/biofeedback stream contracts plus Android multicast and
   Bluetooth permission guidance.
10. GPU buffer-layout, pass-graph, ping-pong, and readback-budget guidance for
   particles, depth, and future Vulkan/Makepad adapters.
11. Clean-room import safety tooling: provenance notes, license checks, and
    public/private boundary scanners.

## Candidate Matrix

| Area | Source Category | Public Rusty XR Shape | Why Useful | Boundary / Defer |
| --- | --- | --- | --- | --- |
| Quest USB/Wi-Fi ADB diagnostics | Local Android phone Quest companion tooling | Diagnostic stage enum, failure enum, endpoint/subnet parser, analyzer CLI, synthetic bundle fixtures | Makes phone-side and Windows-side Quest debugging reproducible | No app package IDs, certs, APK payloads, or private traces |
| Quest tracking access boundary | Local Quest/OpenXR and shell-helper workflows | Public note for foreground OpenXR pose/velocity sampling, Android sensor limits, and ADB/shell-helper non-ownership of fused tracking | Prevents utilities from treating `adb shell` or a background service as a system-wide tracking source | Native OpenXR sampler remains app-owned; no private tracking internals |
| USB-host ADB bootstrap | Local Android phone Quest companion tooling | USB interface descriptor for ADB class/subclass/protocol `255/66/1`, permission-flow docs, TCP handoff model | Documents how a phone can bootstrap Quest Wi-Fi ADB without a PC | Android implementation stays adapter/example-only |
| Quest wake and visual readiness | Local Windows Quest operator tooling | `QuestWakeReadiness`, `QuestPowerStatus`, `ForegroundSnapshot`, visual blocker enum, proximity status | Captures the recurring issue that awake/mounted is not the same as visually ready | No project-specific recovery scripts or headset serials |
| `hzdb` and ADB wrapper contracts | Local Windows Quest operator tooling, bureau notes | Selector-aware command request/result schema, screenshot/perf/wake/proximity fallback guide | Standardizes official Meta tooling use without hiding failures | Do not vendor `hzdb`; paths remain user-configured |
| Screenshot and cast evidence | Local Quest cast/orchestration tooling, bureau notes | Cast session manifest, screenshot manifest, visual-test artifact folder schema | Provides regression artifacts for public examples and APK smoke tests | Use upstream `scrcpy` by reference; no bundled binaries |
| Scrcpy cast profiles | Local Quest cast/orchestration tooling | Device/cast profile descriptors, window layout bounds, capture target preference model | Reusable live visual source for Windows tooling | Attribute `scrcpy`; do not copy unlicensed quest-screen-caster code |
| Quest session kit manifests | Local operator, runtime-profile, and phone companion tooling | APK library manifest, hotload profile, device profile, selected target/session manifest schemas | Unifies install, launch, profile push, and monitoring workflows | Package identities and release payloads remain downstream-owned |
| Runtime config and hotload profiles | Local runtime-profile and Windows operator tooling | Generic key/value runtime profile, device profile, push/apply result model | Gives Rust, Unity, and Windows shells a shared public vocabulary | Private tuning constants and study keys stay out |
| Quest performance profile hints | Local runtime-profile, room/depth, and Rust particle tooling | Refresh/foveation/perf-level/overdraw/transparency/readback budget schemas | Makes performance levers explicit and testable | Use generic defaults, not private parity numbers |
| Room mesh provider contracts | Local Unity room/depth tooling | Room mesh source kind, state, anchor snapshot, semantic snapshot, processed snapshot stats | Generalizes room scans, MRUK, semantic fallback, and debug visualizers | Unity/MRUK/OVR adapters remain optional |
| Processed scan cache stats | Local Unity room/depth tooling | Geometry hash, cleanup stats, disconnected-island pruning stats, depth tolerance metadata | Helps compare raw scan meshes with cleaned/cacheable forms | No private scene geometry or captures |
| Environment-depth health | Local Unity room/depth and BLE/PCA tooling | Depth availability snapshot, accepted-frame counts, hardware support, degraded reason | Separates PCA, environment depth, occlusion, and native probe failures | Native `OVRPlugin`/OpenXR calls stay adapter-only |
| Live depth CPU frame descriptors | Local Unity room/depth tooling | Per-eye frame descriptor, meters units, width/height, timestamp, readback-pending state | Supports debug tools without locking into Unity or Meta SDKs | Raw GPU texture import and readback implementation deferred |
| PCA camera lifecycle | Public/local PCA references | Camera info, eye selection, intrinsics, YUV plane/stride, timestamp, session/dispose state machine | Needed before clean Android/Camera2 or OpenXR adapters | Check license per source; do not copy proprietary samples |
| MediaProjection permissions | Public/local MediaProjection references, media docs | Permission request taxonomy, foreground-service manifest guide, capture session state model | Clarifies headset popup behavior and Windows streaming routes | App must request consent; launchers cannot bypass it |
| WebRTC/PCA vision streams | Public/local PCA streaming references | Signaling config, frame stream descriptor, vision result packet, latency stats | Enables future external inference examples | Keep optional; no external service dependency in core |
| Capture dataset manifests | Public/local reconstruction references | Per-eye image/depth/pose frame index, YUV/RGB conversion notes, COLMAP/TSDF export metadata | Makes camera/depth captures reproducible and tool-agnostic | No generated datasets in repo |
| BLE/LSL/biofeedback streams | Local BLE/PCA, biofeedback, and runtime-profile tooling | Stream descriptor, channel schema, sample frame, reconnect status, multicast lock docs | Extends existing LSL/Polar work into generic physiological inputs | No private stream names, study logic, or app-specific simulation behavior |
| Polar data handling docs | Local BLE/PCA tooling and public Polar references | HR/RR/ECG/ACC CSV schemas, PMD command flow, LSL schema guide | Helps build clean APK and Windows examples later | Attribute Polar BLE SDK and MesmerPrism PolarH10 docs |
| AirRes-style actuator model | Local biofeedback tooling and public AirRes references | Bluetooth Classic RFCOMM/SPP stream and resistance command packet contracts | Provides a generic physiological output/actuator path | Cite AirRes Mask paper; no CAD/hardware repo import by default |
| Controller label and hand menu schemas | Local biofeedback and runtime-profile tooling | Controller label layout, terminal menu command schema, debug HUD state model | Useful for public XR debugging UI examples | No project assets or stylized visuals |
| Explicit adapter wiring policy | Local runtime-profile workflows | Consumer adapter guide: serialized/static references, warnings for missing refs, no hidden runtime auto-create | Keeps optional Unity/Makepad adapters predictable | This is documentation first |
| Build validation ladder | Bureau notes, Unity repos, Quest repos | Rust/Android/Unity validation docs: WSL fallback, Smart App Control note, 16 KB APK checks, log marker parsing | Reduces local build churn and explains APK constraints | No private build scripts or release payloads |
| GPU buffer layout guide | Local Rust particle/GPU experiments | Buffer schema docs, pass graph, ping-pong state update rule, readback cadence policy | Supports future Vulkan/Makepad compute adapters | Exclude app-specific simulation semantics |
| Hand mesh and SDF interaction contracts | Local Rust particle/GPU and Unity room/depth tooling | Deformed triangle mesh snapshot, packed SDF snapshot, GPU-resident descriptor, influence points | Connects hand tracking, particles, SDF, and debug physics | Native hand mesh extensions stay adapter-only |
| Public template matrix | Local Quest template repos | Platform-specific vs vendor-neutral example guidance, optional OSC/MIDI/NDI/control inputs | Helps decide which examples belong in Rusty XR | Do not import downstream effect behavior |
| Clean-room import scanner | Non-public planning checklists and repo audits | License/provenance report, boundary-name scanner, generated-artifact scanner | Protects the public/downstream boundary before examples/adapters | Scanner should flag, not auto-publish |

## Android, Quest, And Windows Tooling Notes

- Rust builds on this Windows machine can be affected by Smart App Control
  blocking local Cargo helpers. Public docs should mention the WSL fallback
  pattern explicitly, using an explicit distro such as `wsl -d Ubuntu`.
- Official `liblsl` runtime use should remain a local configuration issue. This
  machine uses a configured `lsl.dll` path, but Rusty XR should not hard-code
  it.
- Android/Quest tooling should document configured `adb`, `hzdb`, Unity, and
  SDK paths as environment-driven checks, not repo-owned constants.
- BLE on Windows should document the pattern of requesting GATT service access
  before characteristic discovery, then retrying transient `AccessDenied` or
  `Unreachable` results.
- Quest screenshot guidance should prefer the right capture method for the
  target surface: `hzdb`/metacam and `/sdcard/Oculus/Screenshots` for shell or
  immersive evidence, with `adb screencap` treated as unreliable in some VR
  contexts.
- Quest tracking guidance should prefer a foreground OpenXR app for fused
  headset/controller pose and velocity. ADB, `dumpsys`, and shell helpers are
  diagnostics routes, not a supported public fused tracking stream. See
  [QUEST_TRACKING_ACCESS_BOUNDARY.md](QUEST_TRACKING_ACCESS_BOUNDARY.md).
- Depth diagnostics should keep PCA, environment depth, direct occlusion
  readback, and native OVR/OpenXR probes separate. Native probe failure can be
  informational while environment depth is still working.
- Only one active environment-depth consumer should be used while validating
  depth availability because platform limits can reject parallel consumers.

## Permissions To Keep Documented

- BLE on Android 12 and later: `BLUETOOTH_SCAN` and `BLUETOOTH_CONNECT` are
  runtime Nearby Devices permissions.
- BLE scanning on Android 11 and older generally requires
  `ACCESS_FINE_LOCATION`, and the location toggle can still affect scan
  results.
- LSL discovery over Wi-Fi can require `ACCESS_NETWORK_STATE`,
  `ACCESS_WIFI_STATE`, `CHANGE_WIFI_MULTICAST_STATE`, and a
  `WifiManager.MulticastLock` in the app shell.
- PCA/depth apps need the relevant headset-camera, scene, and passthrough
  permissions declared and requested by the app when the target platform
  requires a runtime prompt.
- MediaProjection requires a foreground service declaration and an explicit
  user consent flow for each capture session. A custom launcher can guide the
  user or start the target activity, but it cannot silently grant
  MediaProjection capture.
- Development launchers can often grant declared runtime permissions with ADB
  for test devices. Public examples should still implement the foreground
  in-headset request path for permissions that require a user popup.

## Already Covered By Current Rusty XR Work

- BLE, LSL, and Polar H10 public models and data-pipeline docs.
- Media pipeline, Windows frame receiver, and APK permission guidance.
- Android/Quest APK shell responsibility split.
- Quest tracking access boundary for OpenXR, Android sensors, and ADB/shell
  diagnostics.
- Camera/depth/SDF contracts.
- Plain stereo layer, visual feedback border, border tuning, and approved
  performance hints.
- XR canvas/ray contracts, hand menu anchors, hand influence points, sparse TSDF
  snapshots, and scan-fusion stats.

## Recommended Extraction Order

1. Add docs and schemas first, with synthetic examples and no downstream repo
   dependency.
2. Add analyzer CLIs for diagnostics bundles, visual-test manifests, runtime
   profiles, and capture manifests.
3. Promote stable schemas into Rust crates after tests prove the shape.
4. Add thin optional adapters only after at least one clean public example uses
   the contracts without private repos.
5. Keep downstream integration for later, after private consumers can be routed
   through the public crates without changing public boundaries.

## Do Not Import Yet

- Native Quest passthrough composition, PCA/Camera2 acquisition, MediaProjection
  capture, OpenXR environment-depth provider start/acquire, or Vulkan external
  texture import code.
- Full Makepad widgets, Unity/MRUK/OVR adapters, Android foreground services,
  or Windows WPF operator shells.
- Private downstream effect-stack layers beyond the approved public
  border/feedback contracts.
- Private app-specific simulation topology, study logic, or
  generated physiological/session datasets.
- Unlicensed or unclear-license reference code. Treat it as behavior to study,
  not source to copy.
