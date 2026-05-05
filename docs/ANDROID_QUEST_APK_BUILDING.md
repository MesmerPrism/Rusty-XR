# Android / Quest APK Building

Rusty XR is a public core workspace, not an APK shell. It is still important
that downstream Rust XR apps can build APKs cleanly, so this document records
the build responsibilities and the intended integration shape.

## Responsibility Split

Rusty XR public crates should own:

- framework-neutral contracts
- runtime configuration models
- camera, depth, SDF, particle, LSL, and diagnostic utility code
- optional thin adapters after the core contracts settle
- public examples that are authored specifically for this repository

The Android / Quest app shell should own:

- Android package identity and signing
- `AndroidManifest.xml` permissions and activity declarations
- OpenXR loader/runtime integration
- Android lifecycle and permission prompts
- Vulkan or renderer backend setup
- swapchains, frame loop, foveation, and platform timing
- headset install, launch, log capture, and visual validation
- app-specific assets, scenes, rendering policy, and release payloads

This split keeps the public core reusable and prevents app-specific package
names, tuning, release metadata, or generated artifacts from leaking into the
public repo.

## Recommended Shell Shape

Use one reusable Rust Android/OpenXR app shell per product family. The shell
depends on Rusty XR crates and converts platform-specific state into plain
public contracts:

```text
Android / OpenXR / renderer shell
  -> Rusty XR contracts and utility crates
  -> app or experiment logic
  -> render payloads, counters, commands
```

Experiment crates should not own APK packaging, OpenXR session setup, Android
permissions, or renderer lifecycle. They should receive snapshots such as poses,
eye views, camera metadata, hand meshes, SDF grids, runtime config, commands,
and frame timing. They should return render payloads, counters, and app-neutral
diagnostics.

## Build Routes

There are two practical public build routes for downstream apps.

### Existing Renderer Shell

Use a Rust renderer/application framework that already owns Android packaging,
activity lifecycle, native library loading, and renderer setup. In that model,
Rusty XR crates are ordinary Rust dependencies and the shell owns the APK.

The shell should expose a small adapter layer that converts framework types to
Rusty XR contracts. Keep that adapter feature-gated and separate from pure
contracts.

### Custom Android Shell

For a custom shell, keep these pieces local to the app repository:

- Android project or packaging tool configuration
- package name and signing config
- manifest permissions
- native entrypoint
- OpenXR loader setup
- Vulkan or other renderer initialization
- install / launch scripts

The shell can build Rust crates with Android targets such as:

```powershell
rustup target add aarch64-linux-android
```

The exact build command depends on the chosen shell/tooling. Common options are
framework-specific wrappers, `cargo-ndk`, or an Android Gradle project that
loads a Rust-produced native library. Do not put app-private package names,
keystore paths, headset serials, or release payload paths in Rusty XR.

## Minimum Quest Shell Checklist

A downstream Quest shell should document and test:

- target ABI: `arm64-v8a` / `aarch64-linux-android`
- Android SDK, NDK, build tools, and JDK versions
- OpenXR loader/runtime setup
- activity used for launch
- required permissions and runtime permission flow
- media-pipeline permission flow, if using camera, display capture, audio, or
  Windows streaming
- renderer backend and swapchain setup
- install command
- launch command
- log capture command
- expected success signals, such as process running, focused activity, frame
  loop active, nonzero draw calls, and headset-visible content

Rusty XR can provide public diagnostic models for those signals, but the actual
commands and app identifiers belong to the shell repo.

For media streaming and permission taxonomy, see
[MEDIA_PIPELINE_AND_PERMISSIONS.md](MEDIA_PIPELINE_AND_PERMISSIONS.md).

## Quest OpenXR Bring-Up Lessons

Quest OpenXR session readiness is sensitive to the Android context passed to
the OpenXR loader and instance creation path. If a Rust Android shell uses
`android-activity`, do not assume the value exposed through `ndk-context` is
the foreground `Activity`. It may be the application object, which is useful
for many Android calls but is not sufficient for Quest OpenXR session readiness.

Use the active `AndroidApp` pointers when initializing the Android OpenXR
loader and creating the instance:

- `AndroidApp::vm_as_ptr()` for the Java VM
- `AndroidApp::activity_as_ptr()` for the current Activity

The failure mode can be misleading. The app may create an OpenXR instance and
still remain visually stuck on the loading/black screen. Common log signals are
runtime warnings about a legacy or non-context OpenXR client,
`xrCreateSession: Activity is not yet in the ready state`, and a session that
never advances beyond `OpenXR state IDLE`.

Durable success signals are:

- the app logs that the Android OpenXR loader was initialized with the Activity
  context
- the OpenXR session advances through `READY`, `SYNCHRONIZED`, `VISIBLE`, and
  `FOCUSED`
- swapchains are created at headset eye resolution
- frame logs continue after the first submitted frame
- for camera-driven examples, logcat reports camera frames being received and
  uploaded before the headset view, cast, or screenshot shows the submitted
  custom layer

Also wait for Android lifecycle foreground readiness before creating the
OpenXR/Vulkan session. A practical custom-shell gate is to wait until the app is
resumed, focused, and has a native window. When debugging renderer bring-up,
launch once with MediaProjection disabled so a capture consent overlay is not
mistaken for an OpenXR session failure. Validate camera-only rendering before
adding screen-streaming capture.

Keep the immersive VR Activity separate from the Android launcher entrypoint
when needed. A launcher alias or small launcher Activity can expose a normal
app icon, while the OpenXR Activity remains focused on VR lifecycle and runtime
requirements.

## Headset 2D Launchers

A normal Quest 2D Android app can act as a useful side-loaded app launcher
without any elevated shell helper. It can organize known package IDs, query
visible launcher activities, and launch packages that expose a front-door
Activity through Android `PackageManager` APIs. Package visibility still
matters for discovery on modern Android, and packages that do not expose a
public launch Activity may not be launchable from normal app mode.

An ADB-launched shell helper is a separate Developer Mode path. It can add
shell-backed package enumeration, explicit `am start`, force-stop,
foreground checks, and diagnostics, but only after an external authorized ADB
host starts the helper. A normal installed headset APK cannot promote itself
to Android `shell` or self-start that helper.

For the full public boundary, see
[QUEST_APP_LAUNCHING_AND_SHELL_HELPERS.md](QUEST_APP_LAUNCHING_AND_SHELL_HELPERS.md).

## Public Example Policy

Public examples that include Android or OpenXR code must be authored as clean
examples for Rusty XR. They should use synthetic data where possible, avoid
private package names and assets, and avoid copying private rendering behavior.

The first public APK example is `examples/quest-minimal-apk/`. It is a
Rust-native Android smoke test: a Java activity loads a Rust `cdylib`, displays
synthetic Rusty XR contract JSON, and emits basic frame-callback status. It is
not an OpenXR scene and does not touch passthrough camera, MediaProjection,
environment depth, Vulkan texture import, or native compositor layers.

Build it locally with:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-minimal-apk\tools\Build-QuestMinimalApk.ps1
```

The APK is written under `examples/quest-minimal-apk/build/`, which is ignored.
Use the catalog in `examples/quest-minimal-apk/catalog/` to install, launch,
and verify it through Rusty XR Companion Apps.

The first public immersive APK example is
`examples/quest-composite-layer-apk/`. It builds a Rust/OpenXR/Vulkan Quest APK
that requests headset-camera access and exposes explicit renderer tiers:
synthetic smoke test, CPU diagnostic flat camera copy, GPU-buffer probe, and
the paired-camera GPU projected headset-camera path. The accepted stereo path
uses GPU-imported Camera2 `PRIVATE` buffers plus metadata-backed shader
projection; probe and CPU tiers remain available for bring-up. Java bridges
public camera metadata to Rust,
including selected camera ID, delivered size, timestamp, optional sensor
orientation, optional pixel-domain and intrinsics data, requested/active tier
labels, transport labels, GPU hardware-buffer descriptors when available, and
explicit missing intrinsics / missing pose flags.

The Tier 2 profile requests Camera2 `PRIVATE` frames and probes their
`HardwareBuffer` / `AHardwareBuffer` descriptors without staging RGBA on CPU,
but it logs fallback while the projection shader/import renderer or pose-backed
stereo metadata is unavailable. The example does not claim true camera/view
alignment in that state. Full-rate custom projection should import GPU-sampled
camera hardware buffers instead of relying on the example's CPU YUV/RGBA copy
path. The diagnostic CPU path is throttled at the ImageReader boundary with
`rustyxr.cameraCpuUploadHz` so skipped frames do not still pay conversion cost.
MediaProjection remains optional and is used only to stream the final headset
screen to a Windows receiver for inspection.

Current live validation has moved the public example beyond the probe-only
state for the tested paired Camera2 provider: both the fullscreen
`display-screen-homography` profile and the `quad-surface` comparison profile
can render paired GPU-imported camera buffers with metadata-backed projection
and the public soft feedback border. The comparison profile remains
intentionally cautionary. It is useful for collaborators investigating
head-anchored quad geometry, sampler behavior, and tone controls, but it is not
yet the expected final performance or color reference.

Build it locally with:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-composite-layer-apk\tools\Build-QuestCompositeLayerApk.ps1 -OpenXrLoaderPath C:\path\to\libopenxr_loader.so
```

The composite-layer build script defaults to target SDK `35`. It accepts
`-TargetSdkVersion 33` as an explicit local probe for headset camera and
permission-policy comparisons; do not treat that switch as a parity fix unless
the corresponding run artifacts show fresh camera progression and stable frame
cadence.

The APK is written under `examples/quest-composite-layer-apk/build/`, which is
ignored. Its catalog can be used with Rusty XR Companion Apps for install,
launch, runtime-profile extras, log capture, screenshot/cast inspection, and
media-receiver validation.

The first public broker APK proof-of-concept is
`examples/quest-broker-apk/`. It builds a separate Android APK/service for
localhost client messages, status/capability reporting, latency samples,
optional native LSL forwarding, optional OSC latency egress, and OSC ingress
values rebroadcast to localhost WebSocket clients. It now also exposes
camera-projection metadata, a bounded app-context Camera2 open/one-frame
capture probe, bounded app-context raw-luma and H.264 binary side-channel
probes, a broker-local Android MediaCodec H.264 decode-consumption probe, and
shell-helper status commands for future video-lab work, plus
`video_lab.register_encoded_stream_manifest`,
`video_lab.record_encoded_sample_metadata`, and `video_lab.record_metric_sample`
paths for encoded-stream contract and timing/drop/queue diagnostics. It
deliberately does not own streaming camera buffers, depth textures, OpenXR frame
timing, MediaProjection capture, production encoded-frame transport,
Vulkan/OpenXR texture import, or layer submission. The composite-layer example
has a separate broker H.264 consumer fixture that can render decoder output to
a Java-owned `SurfaceTexture` external texture for handoff telemetry, or decode
into `ImageReader` `PRIVATE` hardware buffers that the existing Vulkan
GPU-buffer-probe path imports and draws into the submitted OpenXR projection
layer. When the broker reports selected Camera2 projection metadata in the
stream-start result, the hardware-buffer consumer forwards intrinsics, pixel
domains, lens pose, and pose source into the native camera-frame metadata so
projection-readiness diagnostics can be checked before enabling a stereo
projected path.

Build it locally with:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-apk\tools\Build-QuestBrokerApk.ps1
```

To enable broker-to-laptop LSL forwarding, supply a license-compliant Android
`liblsl.so` explicitly:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-apk\tools\Build-QuestBrokerApk.ps1 -LslAndroidLibraryPath C:\path\to\liblsl.so
```

Rusty XR does not vendor `liblsl.so` in this example. Without it, the broker
still answers the localhost API, accepts samples, logs diagnostics, and can
test OSC ingress/egress. The APK is written under
`examples/quest-broker-apk/build/`, which is ignored. The catalog in
`examples/quest-broker-apk/catalog/` exposes runtime profiles for localhost
latency, OSC egress, and OSC drive ingress.

The broker proof has been validated with
[The Big Red Button Institute](https://github.com/MesmerPrism/the-big-red-button-institute),
the public Unity Quest example for comparing direct Unity OSC/BLE input against
broker-routed stream events on one visible scene target. When using ADB to exercise
Quest app commands, keep the limits in
[Quest ADB Input Workflow](QUEST_ADB_INPUT_WORKFLOW.md) in mind: synthetic
keyboard input can validate app command routes, but synthetic Android gamepad
keys are not controller parity for OVRInput/OpenXR bindings.

The source-only broker shell-helper example is
`examples/quest-broker-shell-helper/`. It builds a dex jar that can be pushed to
`/data/local/tmp` and launched with `adb shell app_process` after the user has
enabled and authorized ADB debugging. The helper reports UID, version, planned
capabilities, and optionally a bounded MediaCodec H.264/H.265/AV1 capability
probe plus metadata-only synthetic encoded-stream events to the broker. Treat it as
Developer Mode/operator tooling, not as an APK permission model:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-shell-helper\tools\Build-BrokerShellHelper.ps1
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-shell-helper\tools\Start-BrokerShellHelper.ps1 -Serial <serial> -ProbeCodecs -EmitSyntheticVideoMetadata
```
