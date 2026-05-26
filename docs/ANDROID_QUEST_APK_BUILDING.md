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

For a Makepad-compatible Android build path that uses Makepad's own packager and
prepares for Makepad Live/hotpatch iteration, see
[MAKEPAD_ANDROID_BUILD_COMPATIBILITY_PLAN.md](MAKEPAD_ANDROID_BUILD_COMPATIBILITY_PLAN.md).
The current Makepad-first comparison lane is tracked in
[MAKEPAD_CAMERA_PARALLEL_APPROACH_COMPARISON.md](MAKEPAD_CAMERA_PARALLEL_APPROACH_COMPARISON.md)
and starts with
`examples/makepad-camera-shell/build-manifest.public.json`.
The current public examples also carry source-only Android build manifests:
`examples/quest-minimal-apk/build-manifest.public.json`,
`examples/quest-composite-layer-apk/build-manifest.public.json`,
`examples/quest-gl-openxr-video-stack-apk/build-manifest.public.json`,
`examples/quest-broker-apk/build-manifest.public.json`, and
`examples/quest-broker-shell-helper/build-manifest.public.json`. These
manifests describe inputs, generated outputs, external tool requirements,
permissions, and capabilities for future Makepad-compatible tooling without
changing the existing build scripts.

Validate the manifests with:

```powershell
python tools\schema\check_android_build_manifest.py examples\quest-minimal-apk\build-manifest.public.json examples\quest-composite-layer-apk\build-manifest.public.json examples\quest-gl-openxr-video-stack-apk\build-manifest.public.json examples\quest-broker-apk\build-manifest.public.json examples\quest-broker-shell-helper\build-manifest.public.json examples\makepad-camera-shell\build-manifest.public.json
```

## Android Toolchain Resolution

The custom Rusty XR APK PowerShell builders share
`tools/android/Resolve-AndroidToolchain.ps1`. Prefer split SDK, NDK, and JDK
roots. `-AndroidPlayerRoot` remains available only as an explicit compatibility
option for machines that deliberately use Unity's bundled Android tools:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-composite-layer-apk\tools\Build-QuestCompositeLayerApk.ps1 -AndroidSdkRoot <sdk-root> -AndroidNdkRoot <ndk-root> -JdkRoot <jdk-root> -OpenXrLoaderPath <path-to-libopenxr_loader.so>
```

Environment fallbacks are `RUSTY_XR_ANDROID_SDK_ROOT`,
`RUSTY_XR_ANDROID_NDK_ROOT`, `RUSTY_XR_ANDROID_JDK_ROOT`, `ANDROID_SDK_ROOT`,
`ANDROID_NDK_ROOT`, and `JAVA_HOME`. Explicit Unity compatibility fallbacks are
`UNITY_ANDROID_PLAYER_ROOT` and `ANDROID_PLAYER_ROOT`; the resolver does not
scan installed Unity editors automatically. The resolver validates that SDK
`build-tools` and `platforms` exist, that the NDK has an LLVM toolchain when a
native build needs it, and that the selected JDK contains the required tools and
can run `javac`. Use this custom-script route when testing the public custom
OpenXR/Vulkan examples and their explicit diagnostics.

The Makepad comparison example should continue using Makepad's Android packager
instead of the custom Rusty XR APK scripts. That route answers a different
question: whether the Makepad packager, generated Android activities,
lifecycle, and renderer surface match the custom Rusty XR path on Quest.

Prefer the example wrapper for local Makepad evidence builds:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\makepad-camera-shell\tools\Build-MakepadStereoAlignmentApk.ps1 -SdkPath <host-matched-sdk> -MakepadSourceRoot <makepad-fork-checkout>
```

The wrapper performs host/profile-aware preflight before invoking cargo. WSL or
Linux-host builds need an SDK with Linux NDK prebuilts and bare Linux tool
names. Windows-host builds need a Windows SDK with Windows NDK prebuilts and
`.exe` / `.bat` / `.cmd` tools; pass `-UseWindowsHost` for that lane. Do not
reuse a Windows SDK path from WSL just because the directory is reachable, and
do not reuse a Linux SDK path for a Windows host build.

When a prepared Windows-host SDK is the selected Rusty XR evidence lane, make
that explicit in the command:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\makepad-camera-shell\tools\Build-MakepadStereoAlignmentApk.ps1 `
  -UseWindowsHost `
  -SdkPath <windows-host-sdk> `
  -MakepadSourceRoot <makepad-fork-checkout> `
  -DisplaySourceEyeMapping display-left-from-left-source
```

If a WSL/Linux-host rebuild fails in Makepad's packager while removing a missing
bundled font asset even after the concrete
`examples/makepad-camera-shell/target/android/makepad-android-apk/` output
subtree has been cleaned once, do not keep cycling staging cleanup as the
primary fix. Treat that as a host-packager-route failure, switch to the
Windows-host wrapper lane, or document that Linux-host Makepad packaging is the
actual thing under test.

If the selected `cargo_makepad` tool ignores the wrapper's SDK preflight and
tries a hardcoded path such as `build-tools/33.0.1/aapt` against a Windows SDK
that actually contains a different `aapt.exe`, the Makepad packager source is
stale for this route. Do not repair that by fabricating a repo-local SDK shadow
with aliased executables. Update or select a Makepad fork/tool whose Android
packager resolves installed build-tools, platform, Java, NDK prebuilt, and host
executable names from the selected `--sdk-path`.

Use `-MakepadSourceRoot` or `RUSTY_XR_MAKEPAD_SOURCE_ROOT` for Makepad local
evidence builds. The wrapper requires that source root by default, selects the
fork checkout's `tools/cargo_makepad` tool, and patches the app's Makepad
dependency to the same checkout for that build. The committed lockfile remains
the default host Rust dependency pin. The current Makepad evidence default is
`display-left-from-left-source`; pass `-DisplaySourceEyeMapping` explicitly in
captured build commands so source-eye mapping cannot drift between runs. Use
`-NoPatchMakepadXrFromSource` only for an intentional upstream or
pinned-dependency comparison.

For the public Makepad comparison example, run the Makepad build from
`examples/makepad-camera-shell` and pass Android options before the
`build`/`run` subcommand. Direct `cargo makepad` remains useful for upstream or
portable Makepad workflows, but the caller is responsible for passing a
host-matched SDK path. The tested command shape is:

```powershell
cargo makepad android --abi=aarch64 --variant=quest --no-icon --sdk-path=<local-makepad-android-sdk> --package-name=<public-example-package> --app-label="Rusty XR Makepad Camera" build -p rusty-xr-makepad-camera-shell --release
```

For run/install:

```powershell
cargo makepad android --devices=<quest-serial> --abi=aarch64 --variant=quest --no-icon --sdk-path=<local-makepad-android-sdk> --package-name=<public-example-package> --app-label="Rusty XR Makepad Camera" run -p rusty-xr-makepad-camera-shell --release
```

Before using a Makepad APK as evidence, remove or timestamp the expected output
path, record the fresh APK hash, and extract `lib/arm64-v8a/libmakepad.so` for
diagnostic string checks. Direct text search against the APK can miss compressed
native-library strings. Remove generated Makepad `target/` output before public
pushes and public boundary scans.

The current Makepad Android packager writes the generated APK below
`examples/makepad-camera-shell/target/android/makepad-android-apk/`. Clean
that concrete output folder before a rebuild; cleaning an older
`target/makepad-android-apk/` path is not sufficient evidence that the next APK
is fresh.

The Makepad comparison example is standalone rather than a root-workspace
package. Validate source with:

```powershell
cargo check --manifest-path examples\makepad-camera-shell\Cargo.toml
cargo test --locked --manifest-path examples\makepad-camera-shell\Cargo.toml
```

or by running the equivalent commands from the example directory. Do not use
root `cargo check -p` for this example.

For Makepad Android, split validation into two gates:

1. Host Rust gate: `cargo check --manifest-path
   examples\makepad-camera-shell\Cargo.toml` plus `cargo test --locked
   --manifest-path examples\makepad-camera-shell\Cargo.toml` cover parser,
   metadata, projection-math changes, and committed lockfile resolution.
2. Android/package gate: `Build-MakepadStereoAlignmentApk.ps1` with
   `-MakepadSourceRoot <makepad-fork-checkout>` or
   `RUSTY_XR_MAKEPAD_SOURCE_ROOT` is the target acceptance gate because it
   exercises Makepad's generated Android activity model and packager while
   keeping the packager source and app Makepad dependencies aligned.

A direct `cargo check --target aarch64-linux-android` is not the authoritative
Makepad Android gate for this example. It compiles the Rust target, but it does
not exercise Makepad's generated Android activity model and packager.

Optional Android-target Rust probe: when a change touches Android-only Rust in
the Makepad example, `cargo check --manifest-path
examples\makepad-camera-shell\Cargo.toml --target aarch64-linux-android` can
compile Android-only Rust modules because the source has an Android-only binary
`main` shim while Makepad packaging still launches through the JNI entrypoint
emitted by `app_main!`. `cargo test --manifest-path
examples\makepad-camera-shell\Cargo.toml --target aarch64-linux-android
--no-run` can be used as extra evidence. Treat these as probes, not required
gates. If the no-run test probe compiles the edited Rust modules and then fails
only at final test linking because no target `cc`/NDK linker is configured,
report that as "Android-target Rust compilation reached the edited path; final
test-link failed due to missing target linker." Do not call that "tests
passed," do not fail the whole workflow solely for that known linker stop, and
do not add local linker setup just for the probe unless the project
intentionally promotes it to a required gate. If it is promoted later, document
a deliberate local NDK linker configuration such as
`CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER` as a separate toolchain
requirement.

For headset gates, prefer
`examples/makepad-camera-shell/tools/Invoke-MakepadCameraDeviceGate.ps1`.
It records whether the app reached XR on the first launcher attempt, a second
launcher retry, or direct generated-XR fallback, so first-launch loading
failures are not hidden by a later successful launch. The harness also records
freshness hashes and key fault counters alongside each launch attempt. Pass the
generated launcher and XR activity names explicitly so the public script does
not bake in downstream app identifiers.

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

The same rule applies to broker integration. A broker, 2D console, companion,
or ADB shell helper can receive, store, analyze, forward, and visualize pose
samples, but the OpenXR sampling itself belongs in the active XR app or in a
plugin/module loaded by that app. Do not design a background broker service as
the owner of headset/controller OpenXR tracking while another immersive app is
running. See [QUEST_TRACKING_ACCESS_BOUNDARY.md](QUEST_TRACKING_ACCESS_BOUNDARY.md).

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

## OpenXR / Vulkan Coordinate Space Checks

Treat world-space anchoring and clip-space rasterization as separate gates.
An app can store particles, meshes, or diagnostic surfaces in a stable
OpenXR reference space and still make them appear head-locked if the final
Vulkan projection path uses the wrong screen-space convention.

For headset-stable world content, validate the chain in this order:

- Create or select a stable scene reference space, such as `LOCAL_FLOOR`,
  `STAGE`, or `LOCAL`, and submit the projection layer in that same space.
- Use `VIEW` only to locate the current head and per-eye offsets, then compose
  each eye pose back into the stable scene space before projection.
- Keep renderer-owned particle or mesh positions in scene coordinates; do not
  rebuild their world basis from the live headset orientation every frame
  unless the content is intentionally head anchored.
- Build clip coordinates through a projection path whose Vulkan viewport
  convention matches the projection matrix or manual FOV math.

The last point is easy to miss. OpenXR FOV projection math is usually written
in eye tangent space where positive Y is up. Vulkan framebuffer coordinates
commonly use a top-left framebuffer origin. If a renderer relies on
OpenXR-style projection matrices or equivalent manual tangent/FOV math, use a
matching viewport convention, for example `y = framebuffer_height` and
`height = -framebuffer_height`, or explicitly fold the Y inversion into the
projection. Mixing an OpenXR-style projection with a positive-height Vulkan
viewport can make stable world-space content look like it is moving with
headset orientation, even when the logged scene anchor and OpenXR view poses
are correct.

This symptom is a render-space bug, not an anchoring bug. Before changing
scene placement code, log a fixed scene basis and compare direct stable-space
`locate_views` results against any `VIEW`-space composition path. If those
poses agree while the visual still appears head-relative, diff the final
viewport, projection matrix, clip-space sign conventions, and per-eye shader
selection against a known-good renderer.

For intermittent headset-visible pixel pops, screen tears, stale frames, or
other render artifacts after the coordinate-space path is correct, use the
direct-device workflow in
[QUEST_RENDER_ARTIFACT_DIAGNOSTICS.md](QUEST_RENDER_ARTIFACT_DIAGNOSTICS.md).
For live camera or broker H.264 streaming cost isolation, use
[QUEST_STREAMING_DIAGNOSTICS_WORKFLOW.md](QUEST_STREAMING_DIAGNOSTICS_WORKFLOW.md)
so synthetic compositor, direct projected Camera2, broker receive/decode, and
broker live projected paths are compared as separate lanes.

## Headset 2D Launchers

A normal Quest 2D Android app can act as a useful side-loaded app launcher
without any elevated shell helper. It can organize known package IDs, query
visible launcher activities, and launch packages that expose a front-door
Activity through Android `PackageManager` APIs. Package visibility still
matters for discovery on modern Android, and packages that do not expose a
public launch Activity may not be launchable from normal app mode.

The same normal-app boundary allows the broker console to control data sources
that the broker APK itself owns through standard Android permissions. For
example, a headset-launched broker console can request Bluetooth runtime
permission, start a broker-owned Polar PMD BLE source, keep publishing
localhost `bio:breath`, and then launch a target XR app through a normal
front-door Activity. That flow does not require the ADB shell helper because
the broker remains the foreground permission owner for its own BLE source.
For sideloaded/debug builds, expect the launcher entrypoint to appear in the
headset's sideloaded or Unknown Sources app view. A public example can provide
its own label and icon, but system quick-access pinning remains launcher-owned
UI state rather than something a normal app should force.
For 2D console apps, declare an explicit Android manifest `<layout>` size on
the launch activity so Horizon OS panel controls have predictable starting
dimensions for resize, reposition, and focused/theater presentation. The
focused panel mode is system UI behavior; app code should remain resilient to
pause/resume and visibility changes rather than assuming it can own that mode.

An ADB-launched shell helper is a separate Developer Mode path. It can add
shell-backed package enumeration, explicit `am start`, force-stop,
foreground checks, and diagnostics, but only after an external authorized ADB
host starts the helper. A normal installed headset APK cannot promote itself
to Android `shell` or self-start that helper.

Device harnesses that launch one public XR example should also clear sibling
public XR example packages before launch. A focused OpenXR app can coexist with
a paused or stopping package process from a previous lane, so standalone
profile runs and Makepad gates record a prelaunch sibling force-stop artifact
before starting the active package. Use the explicit skip switches only when
the test is intentionally measuring cross-app residency.

For the full public boundary, see
[QUEST_APP_LAUNCHING_AND_SHELL_HELPERS.md](QUEST_APP_LAUNCHING_AND_SHELL_HELPERS.md).
For the distribution boundary between Store-style 2D apps, SideQuest or GitHub
developer builds, external ADB hosts, Wi-Fi ADB, and shell helpers, see
[QUEST_DISTRIBUTION_AND_ADB_BOUNDARY.md](QUEST_DISTRIBUTION_AND_ADB_BOUNDARY.md).

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
and the public projection-border policy. The comparison profile remains
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

The first public OpenXR/OpenGL ES feasibility example is
`examples/quest-gl-openxr-video-stack-apk/`. It builds a Rust native Quest APK
that requests `XR_KHR_opengl_es_enable`, creates an EGL/OpenGL ES context,
creates an OpenXR session with `XrGraphicsBindingOpenGLESAndroidKHR`, renders
distinct static left/right diagnostic grids into per-eye GL swapchains, and
logs the public `OpenXrGlesFeasibilityStatus` payload.

Build it locally with:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-gl-openxr-video-stack-apk\tools\Build-QuestGlOpenXrVideoStackApk.ps1 -OpenXrLoaderPath C:\path\to\libopenxr_loader.so
```

The APK is written under `examples/quest-gl-openxr-video-stack-apk/build/`,
which is ignored. The current example also creates per-eye
`GL_TEXTURE_EXTERNAL_OES` / `SurfaceTexture` output surfaces, connects to
broker-compatible H.264 streams on local loopback, decodes them with Android
MediaCodec into those surfaces, and calls `updateTexImage()` from the native GL
render thread. It still does not include camera access or effect passes yet.

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
deliberately does not own production camera sessions, depth textures, OpenXR
frame timing, MediaProjection capture, unbounded encoded-frame transport, or
release layer submission. The composite-layer example has broker H.264 consumer
fixtures that can render decoder output to a Java-owned `SurfaceTexture`
external texture for handoff telemetry, or decode into `ImageReader` `PRIVATE`
hardware buffers that the existing Vulkan/OpenXR GPU-buffer path imports and
draws into the submitted projection layer. The stereo live-bounded fixture
accepts left/right stream sockets before capture, decodes arriving packets into
timestamp-nearest paired hardware buffers, forwards intrinsics, pixel domains,
lens pose, and pose source into native camera-frame metadata, and can validate
the `gpu-projected` OpenXR stereo path. It is still a diagnostic profile;
unbounded sessions, remote-device validation, and release performance remain
future work.

The broker H.264 encoder path records the selected platform encoder,
hardware/software status, size/rate and bitrate-mode support, requested/applied
CBR mode, encoder output-format changes, SPS/PPS CSD as base64 manifest fields,
sync-frame request status, codec-config packet counts, video packet counts, and
Camera2 capture-start timestamps/frame numbers. Live-bounded streams decouple
MediaCodec draining from TCP writes with a bounded writer queue, keeping
codec-config packets and newer keyframes ahead of droppable non-keyframes while
reporting queue depth, writer packet counts, and drop counters. Decoder probes
and composite H.264 consumers request Android decoder low-latency mode
separately from encoder latency hints. Codec-config packets are reported
separately from real video packets so short bounded probes do not hide frame
starvation behind SPS/PPS setup data.

Build it locally with:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-apk\tools\Build-QuestBrokerApk.ps1
```

To enable broker-to-laptop LSL forwarding, supply a license-compliant Android
`liblsl.so` explicitly:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-apk\tools\Build-QuestBrokerApk.ps1 -LslAndroidLibraryPath C:\path\to\liblsl.so
```

For local machines that should always build the broker with native LSL enabled,
set `RUSTY_XR_ANDROID_LIBLSL` to that same `liblsl.so` path. Use
`-RequireNativeLsl` in validation or release-prep scripts so the build fails
instead of silently producing an LSL-fallback APK:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-apk\tools\Build-QuestBrokerApk.ps1 -RequireNativeLsl
```

The build script checks `RUSTY_XR_ANDROID_LIBLSL` in the process, user, and
machine environment scopes, in that order.

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

The source-first Makepad comparison example is
`examples/makepad-camera-shell/`. It is a standalone package that uses
`cargo-makepad android --variant=quest` to generate the Android manifest, Java
activities, OpenXR loader packaging, signing, APK output, install, and run
surface. The lane started as a synthetic smoke app, then added Camera2
metadata/acquisition, paired texture import, and a broker-managed synthetic
H.264 input path. Broker-synthetic validation is source parity only when it
requests the broker's left/right H.264 streams through the public command and
`RXYRVID1` framing, consumes stream-header projection metadata, and reports
decoded texture cadence. A local generated texture is only a renderer smoke
witness.

On Quest Vulkan/XR builds, the verified broker path decodes with Android
MediaCodec and uploads decoded YUV planes into Makepad textures. The older GL
external-texture handoff depends on a texture-handle event that this path does
not produce, so zero-copy surface-texture transport must be measured as a
separate follow-up. Current broker-synthetic gates record unbounded
`max_packets=0` stream headers, per-eye stream metadata, prepared decode state,
CPU-YUV texture readiness, texture-update cadence, decode errors, and derived
`surface_to_camera`, `screen_to_surface`, and `screen_to_camera` rows. Repeated
small hardware-buffer warnings remain tracked separately during import and
performance comparison work. Current isolation moved the earlier GPU page-fault
class below `makepad-xr` and into Makepad's Android Vulkan window-swapchain
recreation after acquire/present reports suboptimal; the maintained fork
carries the current frame-fence wait candidate.

For video-only receiver experiments, OpenXR and Vulkan should not be conflated.
Retail Quest apps still need an XR presentation path for coherent left/right
eye timing, pose prediction, lens correction, and compositor submission; a
plain Android GL window is a compositor-managed panel rather than a public API
for targeting one eye display. However, an app can be shaped as an OpenXR app
with an OpenGL ES renderer: OpenXR owns the session and stereo swapchains,
while GL owns `SurfaceTexture` / `GL_TEXTURE_EXTERNAL_OES` video ingestion,
FBO-based processing passes, and final eye rendering. Treat this as a separate
architecture candidate from the current Vulkan `AHardwareBuffer` path. Its
main expected benefit is simpler Android video ingestion; its main expected
cost is validating and maintaining a second renderer/presentation lane.
