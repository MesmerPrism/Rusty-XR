# Makepad-Compatible Android Build Plan

This document explains how to move the public Rusty XR Quest examples from the
current custom APK build scripts toward a Makepad-compatible Android build
path. The goal is to make Quest-to-Quest camera streaming and related headset
diagnostics easier for Mac, Windows, and Linux contributors, especially
contributors who already build XR apps through Makepad.

## Why This Matters

The current Rusty XR Quest examples prove useful architecture:

- a Rust/OpenXR/Vulkan composite-layer APK
- a separate broker APK
- Camera2 `PRIVATE` hardware-buffer acquisition
- Android MediaCodec H.264 encode/decode probes
- metadata-backed stereo projection
- live broker stereo streaming profiles
- repeatable scorecard tooling for direct camera versus broker streaming

But the current APK packaging scripts are pragmatic Windows helpers. They are
not Makepad-native, and they are awkward for a Mac-based Makepad developer to
use directly.

Makepad already provides a fast mobile build route through `cargo-makepad`.
The public Makepad packaging guide documents Android setup with:

```bash
cargo install --force --git https://github.com/makepad/makepad.git --branch dev cargo-makepad
cargo makepad android install-toolchain
cargo makepad android build -p your_profile_name --release
```

Makepad also has a Live system for runtime UI/style iteration. The Makepad
architecture docs describe Live DSL updates as runtime-applied changes that do
not require recompiling Rust. Future Makepad work on streaming updated Rust
dynamic libraries into a running app would make the compatibility target even
more valuable: APK rebuilds would be needed only for platform, permission,
manifest, or ABI changes, while many Rust-side patches could use the hotpatch
path.

References:

- [Makepad Project Packaging Guide](https://makepad.rs/guide/appendix/packaging-guide)
- [Makepad Framework Architecture](https://book.makepad.rs/guide/start/makepad-framework-architecture)

## Current Rusty XR Build Shape

The relevant build scripts are:

- `examples/quest-composite-layer-apk/tools/Build-QuestCompositeLayerApk.ps1`
- `examples/quest-broker-apk/tools/Build-QuestBrokerApk.ps1`
- `examples/quest-minimal-apk/tools/Build-QuestMinimalApk.ps1`
- `examples/quest-broker-shell-helper/tools/Build-BrokerShellHelper.ps1`

These scripts do not depend on the Unity engine. They now prefer standalone
Android SDK/JDK/NDK roots for Android tooling:

- Android SDK
- Android NDK
- OpenJDK
- `aapt2`
- `d8`
- `zipalign`
- `apksigner`
- NDK clang / `glslc`

They currently assume Windows-oriented details:

- `RUSTY_XR_ANDROID_SDK_ROOT`, `RUSTY_XR_ANDROID_NDK_ROOT`, and
  `RUSTY_XR_ANDROID_JDK_ROOT`, or the standard `ANDROID_*` / `JAVA_HOME`
  variables
- explicit `UNITY_ANDROID_PLAYER_ROOT`, `ANDROID_PLAYER_ROOT`, or
  `-AndroidPlayerRoot` only for compatibility with a deliberately selected
  Unity AndroidPlayer install
- `windows-x86_64` NDK prebuilt toolchain paths
- `.exe`, `.bat`, and `.cmd` tool names
- `%LOCALAPPDATA%` debug keystore folders
- PowerShell as the primary orchestration shell

This is fine for local validation, but it is not the build path we should ask
Makepad users to adopt.

## Target Shape

The target is a Makepad-compatible public example where:

- Makepad owns the Android package build, install, and run loop.
- Rusty XR owns reusable contracts, camera metadata, stream framing,
  diagnostic scorecards, and renderer-agnostic policy.
- Android platform media pieces remain thin adapters: Camera2, MediaCodec,
  hardware-buffer handoff, permissions, and lifecycle.
- Quest-specific OpenXR/passthrough/camera capabilities remain explicit and
  profile-driven.
- Hotloadable runtime settings use stable public key names, so Makepad Live
  DSL, ADB intent extras, WebSocket control, or future hotpatch tools can all
  drive the same configuration surface.
- Generated APKs, logs, captures, stream payloads, keystores, and local SDK
  paths stay out of the repo.

The ideal contributor experience should eventually look like:

```bash
cargo makepad android install-toolchain
cargo makepad android build -p rusty-xr-q2q-receiver --release
cargo makepad android build -p rusty-xr-q2q-sender --release
```

or the equivalent `run` command when Makepad's profile supports install and
launch for a connected Quest.

## Current Host-Aware Packaging State

The maintained Rusty Quest Makepad fork has moved the packaging path toward
runtime SDK resolution. Packaging should inspect the selected `--sdk-path`
instead of treating installer URL constants as the build truth:

- resolve the installed Android platform from the SDK, or honor
  `ANDROID_PLATFORM`;
- resolve installed build-tools from the SDK, or honor
  `ANDROID_BUILD_TOOLS_VERSION`;
- choose the NDK host prebuilt for the build host, such as `windows-x86_64` on
  Windows and `linux-x86_64` on Linux/WSL;
- use host-correct tool names, including Windows suffixes only on Windows.

Installer defaults are a separate concern. If `install-toolchain` downloads a
Makepad-managed SDK, the URL set, directory names, Android platform,
build-tools version, and NDK version must be updated together. A half-upgraded
installer default can still leave packaging broken even when the selected SDK
itself is valid.

## Compatibility Principles

Keep the migration layered. Do not move all platform code into Rusty XR core.

1. **Contracts stay pure Rust.** Camera metadata, stereo layouts, runtime
   profile values, stream manifests, timing summaries, and scorecard parsing
   must remain usable without Android, OpenXR, Vulkan, or Makepad.
2. **Platform adapters stay thin.** Android Camera2, MediaCodec, OpenXR loader,
   Vulkan hardware-buffer import, and headset permissions belong in examples
   or optional adapters.
3. **Build system is a shell concern.** Makepad should be allowed to own APK
   packaging without changing Rusty XR's public contracts.
4. **Keep runtime keys stable.** Existing keys such as
   `rustyxr.xrRenderScale`, `rustyxr.cameraProjectionMode`,
   `rustyxr.brokerH264LiveDecode`, and `rustyxr.cameraPipelinePreset` should
   remain the shared control vocabulary.
5. **Do not hide profile changes.** Every hotloaded or hotpatched value should
   be reflected in logs and run manifests.
6. **Never require private tooling.** The public path may require Android SDK,
   NDK, JDK, Quest devices, and a user-supplied OpenXR loader, but not private
   repositories or local machine paths.

## Parallel Maintenance Recommendation

Maintain one coherent Rusty XR repository with one public core and two public
Quest shell lanes:

```text
Rusty XR core crates
  -> current custom Quest APK lane
  -> Makepad Quest APK lane
```

The goal is not to maintain two products. The goal is to maintain two thin
shells that consume the same public contracts:

- **Core crates** own runtime configuration, camera metadata, stream framing,
  projection/math helpers, diagnostics, and scorecard models. They stay pure
  Rust and framework-neutral.
- **The current custom APK lane** remains the measured diagnostic baseline for
  Camera2, MediaCodec, hardware-buffer handoff, projection cost, and scorecard
  interpretation.
- **The Makepad lane** is the ergonomic app-shell lane for Makepad Android
  packaging, `makepad-xr`, Live/studio iteration, permission flow, and future
  Makepad-native render or camera adapters.

This is feasible as long as the Makepad delta is managed as a shallow patch
queue instead of a long-lived forked framework. Acceptable Makepad-side patches
are accessibility and portability fixes, such as Windows path handling,
dependent shared-library packaging, documentation, or small bridge hooks needed
to expose public Rusty XR runtime profiles. Avoid invasive Makepad renderer,
camera, or lifecycle forks unless the patch is intended to be upstreamed.

Keep the parallel lane while these conditions hold:

- Rusty XR core stays Makepad-independent.
- Makepad-specific code stays in the Makepad example or optional adapter crates.
- The current custom APK lane remains the ground-truth diagnostic baseline.
- The Makepad patch queue stays small enough to rebase manually across upstream
  Makepad revisions.
- Every Makepad update runs the same smoke ladder and updates the comparison
  ledger when affordances, costs, or dependencies change.

Pause or drop the Makepad lane if it requires ongoing invasive renderer forks,
if the GPU fault cannot be isolated enough for useful measurement, or if Makepad
dependencies start shaping the public core APIs.

The repository-level ownership boundary for this branch/fork model is tracked
in [MAKEPAD_FORK_RELATIONSHIP.md](MAKEPAD_FORK_RELATIONSHIP.md).

## Iteration Plan

### Phase 0: Freeze The Known-Good Diagnostic Baseline

Keep the current public examples available while Makepad compatibility is being
added:

- `quest-composite-layer-apk`
- `quest-broker-apk`
- `quest-broker-shell-helper`
- `tools/quest-camera-profile`
- `tools/quest-streaming-diagnostics`
- `tools/video/serve_rxyrvid1_h264.py`

Before refactoring build or runtime ownership, preserve a scorecard for:

- synthetic compositor at `0.75` and `0.65`
- direct in-app Camera2 projected stereo at `0.75` and `0.65`
- broker existing-stream receive/decode at `0.75` and `0.65`
- broker live projected stereo at `0.75` and `0.65`

For any scorecard that will be used as comparison evidence, record the Quest
device performance props before launch and in the final artifact summary:
`debug.oculus.cpuLevel`, `debug.oculus.gpuLevel`,
`debug.oculus.foveation.dynamic`, and `debug.oculus.foveation.level`. Current
target evidence uses CPU/GPU level `4` / `4`; runs captured at other levels
remain useful diagnostics, but should not be presented as parity evidence
without repeating or normalizing the device profile.

This protects the current finding: the measured dominant cost is the shared
metadata-backed projected draw/render path, not broker receive/decode or
Java/native hardware-buffer handoff.

### Phase 1: Make The Existing Build Inputs Explicit

Document the exact artifacts the current scripts produce and consume:

- native Rust library output
- Java source list
- Android manifest
- packaged native libraries
- shader compiler inputs
- OpenXR loader input
- debug signing key
- catalog APK output path

Then make those inputs available as a small build manifest that a Makepad
profile or helper can consume. A useful first manifest can be source-only JSON
or TOML under the example folder, for example:

```text
examples/quest-composite-layer-apk/build-manifest.public.json
```

The manifest should describe files and capabilities, not local absolute paths.

Initial Phase 1 manifests now live beside the current public examples:

- `examples/quest-minimal-apk/build-manifest.public.json`
- `examples/quest-composite-layer-apk/build-manifest.public.json`
- `examples/quest-broker-apk/build-manifest.public.json`
- `examples/quest-broker-shell-helper/build-manifest.public.json`
- `examples/makepad-camera-shell/build-manifest.public.json`

They use schema version `rusty.xr.android-build-manifest.v1` and record source
inputs, generated build inputs, external tool/library inputs, generated
outputs, Android permissions/features, and scorecard-relevant capabilities. The
manifests are intentionally descriptive: they do not replace the current build
scripts yet, and they do not contain local SDK paths, generated APK bytes,
keystore paths, device serials, or run artifacts.

Validate them with:

```powershell
python tools\schema\check_android_build_manifest.py examples\quest-minimal-apk\build-manifest.public.json examples\quest-composite-layer-apk\build-manifest.public.json examples\quest-broker-apk\build-manifest.public.json examples\quest-broker-shell-helper\build-manifest.public.json examples\makepad-camera-shell\build-manifest.public.json
```

The validator checks the public manifest shape, verifies required source files
exist, allows explicit `scope: "workspace"` source inputs for shared core crates,
keeps artifact outputs under ignored `build/` folders, permits Makepad-generated
Android manifests under ignored `target/` folders, and rejects local absolute
path hints in portable fields.

### Phase 2: Create A Makepad Host Shell Skeleton

The first standalone Makepad shell now lives at:

```text
examples/makepad-camera-shell/
```

It intentionally does not attempt the full camera stack yet. The current
checked-in repro has been reduced to a minimal `XrRoot` while the Makepad XR GPU
fault is isolated. It currently proves:

- `cargo-makepad` can build and package the Android app
- the app installs and launches on Quest
- the Makepad Quest variant can generate the required OpenXR, passthrough, and
  headset camera manifest surface
- a `makepad-xr` `XrRoot` app can launch through the generated XR activity
- runtime profile values can be resolved through `rusty-quest-makepad-runtime-config` and
  logged at startup
- the app can emit the same public diagnostic marker style used by existing
  scorecard tools

Current device validation first exposed two blockers: the tested Makepad tool
needed Windows packaging fixes for generated wrapper paths and dependent Rust
shared libraries, and the Quest log reported GPU page faults during the Makepad
XR path. Earlier isolation variants showed that the fault does not require the
permission-flow widget, Makepad UI surfaces, a simple cube marker, the
environment cube, a persistent headset-camera grant, fixed foveation, or a
simple app-side queue-idle after submit. A control run of Makepad's upstream XR
example on the same headset reproduced the GPU page fault symptom. Follow-up
splits then ruled out depth provider/swapchain creation, passthrough creation,
composition-layer submission, OpenXR color swapchain creation, OpenXR
frame-loop work, OpenXR session creation, Makepad OpenXR instance creation, and
the generated-XR activity as required causes. The active bracket moved into
Makepad's Android Vulkan window-swapchain recreation after acquire/present
reports suboptimal. Suppressing that recreation stayed clean, a same-toolchain
Quest/Vulkan baseline still faulted, and waiting either device idle or the
current window-frame fence before recreation stayed clean.

The maintained Makepad fork now carries the targeted frame-fence wait as source
state, alongside the Windows Android packaging fixes needed for this build
lane. A no-diagnostic Quest/Vulkan counter repeat stayed clean, and the Rusty XR
Makepad shell now passes launcher plus generated-XR startup/liveness validation
against that fork state. The same shell also passes the Camera2
metadata/acquisition gate: both launch paths enumerate three `PRIVATE` sources,
select a back-facing 1280x1280 source with intrinsics and pose metadata, acquire
one hardware-buffer-backed frame, and complete the bounded probe with
`status=ok`. The direct generated-XR path now also emits paired Makepad
`VideoExternal` import/projection markers with `pairedLeftRightGpuBuffers=true`
and `alignedProjection=true`. The current validation method uses a short
startup/marker capture for Java activity, native bootstrap, Rust app, camera,
and import markers, plus a separate liveness/fault capture for app-process GPU
page-fault and fatal counters. Repeated small hardware-buffer warnings remain
tracked separately during paired import and performance comparison work.
S32 visual review reclassified the Makepad headset image as native compositor
passthrough plus a low app-owned panel rather than proof of custom projection
parity. The current S33 gate therefore treats marker success as necessary but
insufficient: the Makepad example must visibly show an app-owned, eye-aligned
camera texture panel before per-eye projection mapping or performance numbers
can be compared against the custom lane.
S33 then proved app-owned geometry and improved alignment but still rendered
solid split-proof colors instead of camera pixels. The later camera-pixel proof
path bound Makepad's Y/U/V camera plane textures directly. A subsequent
`VideoExternal` hardware-buffer import diagnostic proved the import path can be
entered, but it showed stale-frame and color issues, so the direct Camera2 lane
keeps CPU-YUV as the default while `cameraTexturePath` and `cpuUploadPath`
remain the source of truth before comparing performance.

Do not start with a full renderer port. Keep the launch and diagnostic log path
repeatable, keep hardware-buffer warnings separate from GPU-fault counters, and
use the direct generated-XR paired import/projection marker path before
interpreting projection or renderer measurements from this lane.

The Makepad-vs-current affordance and cost ledger is tracked in
[MAKEPAD_CAMERA_PARALLEL_APPROACH_COMPARISON.md](MAKEPAD_CAMERA_PARALLEL_APPROACH_COMPARISON.md).
The active Makepad XR GPU fault isolation log is tracked in
[MAKEPAD_XR_GPU_PAGE_FAULT_INVESTIGATION.md](MAKEPAD_XR_GPU_PAGE_FAULT_INVESTIGATION.md).

### Phase 3: Bridge Runtime Profiles To Makepad Live/Control

The current public examples use Android intent extras and catalog profiles.
Makepad integration should preserve those keys but allow multiple control
frontends:

- launch-time profile values
- Makepad Live DSL values where they naturally map to UI/style/projection
  tuning
- ADB or WebSocket control for automated test harnesses
- future Makepad Rust hotpatch state for code-defined behavior

Recommended adapter shape:

```text
RuntimeProfileSource
  -> Rusty XR key/value runtime config
  -> typed camera/render/stream settings
  -> Makepad Live fields or platform adapter settings
```

Keep a recurring log line that prints the resolved values. The scorecard tools
should be able to tell whether a requested value was actually applied.

### Phase 4: Move Stream Framing And Metrics First

Before moving Camera2 and rendering, move the lowest-risk shared pieces:

- `RXYRVID1` packet framing
- stream manifest fields
- per-eye packet counters
- pair/drop policy data structures
- timing metric names
- scorecard marker strings

These can become a small pure-Rust or mostly pure-Rust module that both the
existing examples and a Makepad shell can use. The Makepad app should be able
to emit scorecard-compatible logs before it renders camera content.

### Phase 5: Add Sender And Receiver Adapters

Add Makepad-compatible Android adapters in small slices:

1. **Receiver existing-stream mode.** Connect to a host-provided or broker
   proxy stream, decode H.264 with Android MediaCodec, and log decoded frames.
2. **Receiver hardware-buffer mode.** Decode into `ImageReader` `PRIVATE`
   buffers and pass handles into the renderer/import layer.
3. **Sender local Camera2 mode.** Capture left/right Camera2 `PRIVATE` buffers,
   encode with MediaCodec, and publish `RXYRVID1` streams.
4. **Sender metadata mode.** Attach camera intrinsics, pose metadata, source
   dimensions, timestamp basis, and source-eye labels.
5. **True Q2Q mode.** Bind sender streams to LAN-visible endpoints and let a
   second headset receive them.

Each slice should have a scorecard lane. Do not wait until the full Q2Q path
exists to measure.

### Phase 6: Port Or Replace The Projected Render Path

The current public example uses a custom OpenXR/Vulkan projection path. A
Makepad-compatible path should choose one of two designs:

1. **Makepad-native render integration.** Map Rusty XR camera metadata,
   homographies, stereo layout, and border/perimeter tuning into Makepad's draw
   and shader abstractions.
2. **Narrow native projection adapter.** Keep a small native OpenXR/Vulkan
   layer for headset camera projection while Makepad owns app packaging,
   runtime control, UI, and diagnostics.

The first design is cleaner if Makepad XR already owns the relevant OpenXR and
texture import hooks. The second design is safer if external Android hardware
buffer import and OpenXR projection-layer details are not yet exposed cleanly
through Makepad.

Whichever path is chosen, keep the scorecard stages stable:

- decoded image wait/acquire
- `HardwareBuffer` extraction
- native bridge
- GPU import hit/miss/failure
- projection shader/draw timing
- OpenXR frame time
- `VrApi` app and CPU+GPU time
- device performance profile and foveation props
- screenshot or headset-cast visual acceptance of the actual camera projection

### Phase 7: Align With Makepad Hotloading

The current Rusty XR examples have lightweight runtime hotload through a second
Android intent. That is useful for scalar settings, but it still keeps Rust code
inside the installed APK.

Makepad compatibility should prepare for two stronger iteration modes:

1. **Live DSL/runtime config hotload.** Use Makepad Live values for projection
   tuning, diagnostics UI, visual debug controls, stream endpoint labels, and
   other data-driven settings that do not require changing Rust code.
2. **Future Rust dynamic-library hotpatch.** When Makepad supports streaming a
   new compiled dynamic library into a running Android app, keep the hotpatch
   boundary explicit and ABI-safe.

For a future dynamic-library path, avoid passing raw Android, OpenXR, Vulkan,
or `AHardwareBuffer` ownership across the reloadable boundary. Prefer:

- stable C-ABI or versioned plain-data structs
- handles owned by the non-reloaded host
- frame-boundary swap points
- explicit `init`, `update`, `render_policy`, and `shutdown` calls
- no long-lived borrowed references across reloads
- logs that include host ABI version and hotpatch module version

Hotpatchable code should start with policy and math:

- stream-pair/drop policy
- camera projection math
- border/perimeter tuning
- diagnostic overlays
- scorecard metric aggregation

Do not start by hotpatching Android service lifecycle, decoder surfaces,
OpenXR session ownership, Vulkan device ownership, or hardware-buffer release
rules.

## Proposed Public Example Layout

A future Makepad-compatible example could use this shape:

```text
examples/makepad-camera-shell/
  Cargo.toml
  README.md
  src/
    app.rs
    runtime_profile.rs
    q2q_sender.rs
    q2q_receiver.rs
    diagnostics.rs
  android/
    AndroidManifest.xml
    permissions.md
  catalog/
    rusty-xr-makepad-camera.catalog.json
```

The catalog should keep the same conceptual profiles:

- synthetic Makepad/OpenXR smoke test
- receiver existing-stream decode
- receiver hardware-buffer projection
- sender local Camera2 H.264
- true Q2Q sender
- true Q2Q receiver
- render-scale `0.65` comparison profile

## Documentation Requirements

To make this accessible to other users, every Makepad-compatible example should
document:

- host OS setup for macOS first, then Windows/Linux notes
- Android SDK/NDK/JDK requirements
- `cargo-makepad` install and toolchain commands
- Quest Developer Mode and `adb devices` checks
- permissions requested by the APK
- how to build, install, and launch one headset
- how to select sender and receiver headsets
- how to run the Q2Q scorecard
- where artifacts are written
- how to reject invalid runs
- what is expected to work without private downstream code

Avoid "magic" local setup. If an OpenXR loader, Android SDK path, signing
identity, or headset IP is needed, make it an explicit user-supplied input.

## Success Criteria

The migration is successful when a new contributor on macOS can:

1. Clone Rusty XR and Makepad.
2. Install Makepad Android tooling through `cargo-makepad`.
3. Build a Rusty Quest Makepad-compatible Quest APK without Unity's AndroidPlayer
   folder.
4. Install it on at least one Quest.
5. Run a synthetic profile and get scorecard-compatible logs.
6. Run either receiver existing-stream mode or direct Camera2 mode.
7. Run a two-headset sender/receiver stream over Wi-Fi.
8. Produce a `scorecard.md` that compares transport/decode, handoff, projected
   render, OpenXR runtime cost, device performance levels, and visual
   acceptance state.
9. Iterate scalar visual/projection settings without rebuilding the APK.
10. Eventually hotpatch selected Rust policy/math modules without rebuilding
    the APK, once the upstream Makepad tool supports that flow.

## Non-Goals

This plan does not move private downstream behavior into Rusty XR. It also does
not require Makepad to adopt the current Rusty XR OpenXR/Vulkan example
internals wholesale. The intended outcome is a clean Makepad-compatible path
that reuses the public Rusty XR contracts, stream protocols, diagnostics, and
measured lessons while letting Makepad own the app-shell and build experience.
