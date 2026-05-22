# Rusty XR Makepad Q2Q Camera Shell

This is a standalone Makepad-first Quest lane. It exists so Rusty XR can compare
the current custom Android APK workflow with Makepad's generated Android/OpenXR
packaging without replacing either path too early.

The first pass was intentionally a synthetic OpenXR smoke app. It proved the
Makepad Quest APK path, emitted Rusty XR log markers, exercised Makepad's XR
root, and documented the integration gaps before adding camera transport,
stream, or broker behavior. The current source now keeps that synthetic stereo
comparison scene and adds a bounded Android NDK Camera2 metadata/acquisition
probe plus a delayed Makepad-owned paired hardware-buffer import probe so
renderer smoke runs can be lined up against the custom APK camera-stereo
baseline before performance parity is measured.

This example consumes the maintained Makepad fork branch as an app-shell
dependency only. Rusty XR core stays Makepad-independent; the relationship and
fork-patch policy are documented in
[../../docs/MAKEPAD_FORK_RELATIONSHIP.md](../../docs/MAKEPAD_FORK_RELATIONSHIP.md).

## Current Scope

- Uses `cargo-makepad android --variant=quest`.
- Uses the maintained Makepad fork branch
  `rusty-xr/android-libstd-packaging`. The exact Makepad revision for this
  example is pinned in `Cargo.lock`. Local evidence builds should run the
  wrapper with `-MakepadSourceRoot <makepad-fork-checkout>` so the packager
  itself comes from the maintained fork checkout. App dependency patching is a
  separate opt-in switch, `-PatchMakepadXrFromSource`, for the narrower case
  where an app build must consume uncommitted Makepad dependency changes.
- Uses `makepad-xr` with a minimal `XrRoot` plus a small synthetic stereo
  comparison scene. Earlier isolation passes tried a status panel, a simple
  cube marker, `XrPermissionsFlow`, and an empty root.
- Reads its startup marker values through `rusty-xr-runtime-config`, so this
  shell is already attached to a framework-neutral Rusty XR core crate.
- Emits `RUSTY_XR_MAKEPAD_Q2Q_STATUS` and
  `RUSTY_XR_MAKEPAD_STEREO_COMPARISON` on startup.
- On Android, emits those startup markers directly through logcat under a
  Rusty XR tag so device smoke tests have a reliable startup signal.
- On Android, starts one bounded NDK Camera2 diagnostic pass that emits
  `RUSTY_XR_MAKEPAD_CAMERA2_METADATA` after enumeration and
  `RUSTY_XR_MAKEPAD_CAMERA2_ACQUISITION` during setup, first-frame, and
  completion. The pass opens one selected `PRIVATE` `AImageReader` source and
  records the first hardware-buffer descriptor.
- On Android, after that bounded acquisition window, selects a left/right source
  pair, starts two Makepad-owned camera playbacks with distinct
  `VideoExternal` textures, and emits
  `RUSTY_XR_MAKEPAD_HARDWARE_BUFFER_IMPORT` plus
  `RUSTY_XR_MAKEPAD_STEREO_PROJECTION` as Makepad's Android/Vulkan video
  texture path enumerates, starts, prepares, and accepts both camera hardware
  buffers.
- Emits `RUSTY_XR_MAKEPAD_CADENCE` samples every five seconds, using Makepad
  `NextFrame` events for callback cadence and left/right
  `VideoTextureUpdated` events for Makepad camera texture progression. The
  marker also carries Makepad `XrUpdate` and draw-event rates so a low camera
  texture cadence can be separated from OpenXR loop cadence and app callback
  cadence.
- The maintained Makepad fork also emits public-safe Java activity and native
  bootstrap phase markers for Android activity creation, native handoff,
  EGL/GL setup, Vulkan readiness, and main-loop handoff.
- Keeps Android SDK, generated Java, generated manifest, native library, and APK
  output under ignored local build folders.
- The current source includes Makepad's `XrPermissionsFlow` so normal launcher
  startup can enter active XR presentation. Earlier isolation variants
  confirmed that the original GPU fault also appeared when the permission flow
  was removed.
- The current source imports a Makepad-owned paired camera source after XR
  startup and reports paired-buffer/projection readiness when both textures
  update. The current visible Makepad scene is an app-owned direct CPU-YUV
  camera panel. S59 proves live no-swap YUV color in that panel; S60 is the
  open step that must choose left versus right camera textures per eye before
  parity performance comparison resumes.
- The current source can also use broker-managed synthetic H.264 stereo streams
  instead of opening Camera2 directly. On the Quest Vulkan path, that route asks
  the maintained Makepad fork to decode the streams with Android MediaCodec and
  hand decoded YUV planes into the same per-eye panel. This is the deterministic
  input lane for comparing transport, projection-stage, and multilayer
  processing costs without relying on a physical camera scene. A zero-copy
  surface-texture route remains a separate performance target.

## Build

Install or update the Makepad cargo extension from a Makepad checkout:

```powershell
cargo install --force --path <makepad-checkout>\tools\cargo_makepad
```

Install the Android toolchain into a local cache that is not committed:

```powershell
cargo makepad android --abi=aarch64 --sdk-path <local-makepad-android-sdk> install-toolchain
```

Build the Quest APK from this example directory with a host-matched Android
SDK:

```powershell
cargo makepad android --abi=aarch64 --variant=quest --no-icon --sdk-path=<local-makepad-android-sdk> --package-name=<public-example-package> --app-label="Rusty XR Makepad Q2Q" build -p rusty-xr-makepad-q2q-camera-shell --release
```

For local evidence builds, prefer the wrapper because it preflights the selected
SDK before cargo runs and can run the maintained fork's `cargo-makepad` tool
without rewriting this example's app dependencies:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\makepad-q2q-camera-shell\tools\Build-MakepadStereoAlignmentApk.ps1 `
  -SdkPath <host-matched-sdk> `
  -MakepadSourceRoot <makepad-fork-checkout>
```

Use `-UseWindowsHost` when the selected SDK is a Windows-host SDK. Without that
switch, the wrapper expects a WSL/Linux-host SDK and a Linux NDK prebuilt. A
path being reachable from WSL is not enough; the SDK must contain tools for the
host that runs `cargo-makepad`.

For repeatable Rusty XR evidence builds, record which host lane built the APK.
If a clean WSL/Linux-host rerun still fails while Makepad removes a missing
bundled font asset, treat that as a Makepad packager-route failure rather than
ordinary stale staging. After cleaning
`target/android/makepad-android-apk/` once, switch to the Windows-host wrapper
lane with `-UseWindowsHost` or state that Linux-host packaging itself is being
tested.

If the `cargo_makepad` tool still tries to run hardcoded
`build-tools/33.0.1/aapt` when the wrapper selected a Windows SDK with a
different build-tools version, the wrong or stale Makepad packager is being
used. Do not create a fake SDK shadow as the primary fix; update or select the
Makepad fork whose packager resolves installed SDK tools and host executable
names from `--sdk-path`.

For projection-footprint alignment work, build a distinct alignment APK rather
than reusing the Q2Q package identity:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\makepad-q2q-camera-shell\tools\Build-MakepadStereoAlignmentApk.ps1 `
  -SdkPath <local-makepad-android-sdk>
```

Enable the synthetic footprint target before the alignment launch when the goal
is to compare the Makepad `screen_to_camera` footprint against the Rusty XR
diagnostic profile without live camera pixels:

```powershell
adb shell setprop debug.rustyxr.makepad.projection.area.diagnostic 1
```

Set the diagnostic to `2` for footprint-only comparison. This keeps the
camera-domain projection target and colored border but suppresses the
screen-to-surface white guide, which can move when projection-area offsets are
being tuned and is not part of stage-1 footprint alignment.

When the diagnostic footprint itself needs tuning, use the same hotload helper
with the projection-area offsets, scales, X keystone, and midpoint bow. These
controls adjust the screen-space UVs before the Makepad `screen_to_camera` and
`screen_to_surface` homographies are evaluated, so they affect the footprint
target rather than the camera-content window. Treat them as diagnostic probes:
the reset/default state keeps keystone and bow neutral because the Rusty XR
reference path does not apply an equivalent pre-homography screen-domain warp.

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\makepad-q2q-camera-shell\tools\Send-MakepadQ2QHorizontalOffset.ps1 `
  -ProjectionAreaDiagnostic 2 `
  -ProjectionAreaLeftUv <left-eye-uv-offset> `
  -ProjectionAreaRightUv <right-eye-uv-offset> `
  -ProjectionAreaVerticalUv <vertical-uv-offset> `
  -ProjectionAreaScaleX <horizontal-scale> `
  -ProjectionAreaScaleY <vertical-scale> `
  -ProjectionAreaKeystoneX <x-keystone> `
  -ProjectionAreaBowX <midpoint-x-bow>
```

For live-camera review, the app-owned red border is a projection-footprint
witness only when logs also identify `liveCameraWindowDomain=projected_camera_uv`
and `s118ProjectedFootprintLiveWindow=true`. Earlier red-border slices could
draw the marker around a centered in-surface camera window, which made the
marked images appear farther apart than the Rusty XR projected footprint even
when the homography matrices were nearly identical.

For final screenshot comparison, disable the red border after reset so visual
analyzers compare camera content instead of the diagnostic outline:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\makepad-q2q-camera-shell\tools\Send-MakepadQ2QHorizontalOffset.ps1 `
  -ProjectionBorderStrength 0
```

Run on a selected Quest device:

```powershell
cargo makepad android --devices=<quest-serial> --abi=aarch64 --variant=quest --no-icon --sdk-path=<local-makepad-android-sdk> --package-name=<public-example-package> --app-label="Rusty XR Makepad Q2Q" run -p rusty-xr-makepad-q2q-camera-shell --release
```

This example is not a root-workspace member. Use
`cargo check --manifest-path examples\makepad-q2q-camera-shell\Cargo.toml` from
the repository root, or plain `cargo check` from this directory; do not use
`cargo check -p rusty-xr-makepad-q2q-camera-shell` from the root workspace.
Treat that as the host-side Rust validation gate for parser, metadata, and
projection-math changes. Do not use a plain
`cargo check --target aarch64-linux-android` as the Android acceptance gate for
this Makepad lane; it can stop at Makepad's generated Android entrypoint model
and does not exercise the actual packager. Android acceptance is a successful
wrapper build through `Build-MakepadStereoAlignmentApk.ps1`, preferably with
`-MakepadSourceRoot <makepad-fork-checkout>` when Makepad-side packaging or
bridge code matters.

Keep the Makepad options before `build` or `run` and use `--key=value` for
paths and package/app values. Before treating an APK as fresh evidence, remove
or timestamp `target/android/makepad-android-apk/`, record the rebuilt APK
hash, and extract `lib/arm64-v8a/libmakepad.so` for diagnostic string checks.
Delete generated `target/` output before public pushes and public boundary
scans.

For unattended camera validation after install, pregrant the declared runtime
camera permissions before the measurement window:

```powershell
adb -s <quest-serial> shell pm grant <public-example-package> android.permission.CAMERA
adb -s <quest-serial> shell pm grant <public-example-package> horizonos.permission.HEADSET_CAMERA
```

MediaProjection is different: it still requires the headset consent flow for
each capture session. A launcher can prepare install, launch, and ordinary
runtime grants, but it should report a blocked MediaProjection consent state
instead of claiming it can bypass the headset prompt.

The generated APK lands under
`target/android/makepad-android-apk/rusty_xr_makepad_q2q_camera_shell/apk/`.

The Makepad runner starts the generated launcher activity. Use the launcher or
normal generated activity for active XR presentation; with `XrPermissionsFlow`
enabled, that path switches into the generated XR activity. Directly launching
the generated XR activity remains useful as a bootstrap/control smoke only,
because the permission flow can switch back to the paired normal activity:

```powershell
adb -s <quest-serial> shell am start -n <public-example-package>/<generated-xr-activity>
```

For end-user startup gates, prefer the guarded launcher harness over one-off
`monkey` or single `am start` commands:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\makepad-q2q-camera-shell\tools\Invoke-MakepadQ2QDeviceGate.ps1 `
  -Serial <quest-serial> `
  -Apk <fresh-makepad-apk> `
  -PackageName <public-example-package> `
  -LauncherActivity <generated-launcher-activity> `
  -XrActivity <generated-xr-activity> `
  -OutDir <ignored-artifact-dir>
```

The harness installs with `--no-incremental` when available, grants ordinary
camera/scene runtime permissions, starts the normal Makepad launcher activity,
waits for the generated XR activity plus OpenXR/end-frame or visible-panel
markers, retries the launcher once if the first start remains in loading, and
then uses the direct generated-XR activity with `com.oculus.intent.category.VR`
as an explicit fallback.

For presentation controls where the normal launcher hop is not the test subject,
start the generated XR activity first:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\makepad-q2q-camera-shell\tools\Invoke-MakepadQ2QDeviceGate.ps1 `
  -Serial <quest-serial> `
  -Apk <fresh-makepad-apk> `
  -PackageName <public-example-package> `
  -LauncherActivity <generated-launcher-activity> `
  -XrActivity <generated-xr-activity> `
  -OutDir <ignored-artifact-dir> `
  -PreferDirectVrActivity
```

For broker H.264 parity, start the broker first, then let the guarded device
gate set the Makepad runtime properties before launch. Use broker-synthetic for
deterministic source/projection checks:

```powershell
adb -s <quest-serial> shell am start -n <broker-package>/<broker-activity>

powershell -ExecutionPolicy Bypass -File .\examples\makepad-q2q-camera-shell\tools\Invoke-MakepadQ2QDeviceGate.ps1 `
  -Serial <quest-serial> `
  -Apk <fresh-makepad-apk> `
  -PackageName <public-example-package> `
  -LauncherActivity <generated-launcher-activity> `
  -XrActivity <generated-xr-activity> `
  -OutDir <ignored-artifact-dir> `
  -PreferDirectVrActivity `
  -UseBrokerH264Synthetic
```

Use broker-camera for a physical Camera2 -> H.264 -> MediaCodec CPU-YUV run:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\makepad-q2q-camera-shell\tools\Invoke-MakepadQ2QDeviceGate.ps1 `
  -Serial <quest-serial> `
  -Apk <fresh-makepad-apk> `
  -PackageName <public-example-package> `
  -LauncherActivity <generated-launcher-activity> `
  -XrActivity <generated-xr-activity> `
  -OutDir <ignored-artifact-dir> `
  -PreferDirectVrActivity `
  -UseBrokerH264Camera `
  -BrokerH264LeftCameraId 50 `
  -BrokerH264RightCameraId 51 `
  -BrokerH264FrameRateHz 50
```

For raw projection-area alignment, the Makepad target uses the same full-layer
plus projected sub-area shape as the other public camera targets. The invalid
projection region can be launched as an opaque solid-red diagnostic border or
as a transparent border over native passthrough:

```powershell
# Direct Camera2, solid diagnostic border.
powershell -ExecutionPolicy Bypass -File .\examples\makepad-q2q-camera-shell\tools\Invoke-MakepadQ2QDeviceGate.ps1 `
  -Serial <quest-serial> `
  -Apk <fresh-makepad-apk> `
  -PackageName <public-example-package> `
  -LauncherActivity <generated-launcher-activity> `
  -XrActivity <generated-xr-activity> `
  -OutDir <ignored-artifact-dir> `
  -ProjectionBorderPolicy solid-red

# Direct Camera2, passthrough underlay border.
powershell -ExecutionPolicy Bypass -File .\examples\makepad-q2q-camera-shell\tools\Invoke-MakepadQ2QDeviceGate.ps1 `
  -Serial <quest-serial> `
  -Apk <fresh-makepad-apk> `
  -PackageName <public-example-package> `
  -LauncherActivity <generated-launcher-activity> `
  -XrActivity <generated-xr-activity> `
  -OutDir <ignored-artifact-dir> `
  -ProjectionBorderPolicy passthrough-underlay
```

The same `-ProjectionBorderPolicy` switch can be combined with
`-UseBrokerH264Camera` or `-UseBrokerH264Synthetic`. It sets
`debug.rustyxr.makepad.projection.border.policy` and pairs
`passthrough-underlay` with
`debug.rustyxr.makepad.native.passthrough.enabled=true`. The app writes alpha
zero outside the projected camera region for the underlay policy and keeps the
camera projection in the same full submitted render surface. Use the solid-red
policy when the actual projected camera footprint needs an unmistakable marker;
use passthrough-underlay for manual headset alignment against native
passthrough.
Add `-EnableNativePassthrough -ProjectionAreaOpacity <0..1>
-ProjectionBorderOpacity <0..1>` when you want a solid-red border with native
passthrough active behind the same full submitted surface. The area opacity
fades only valid projected camera pixels; the border opacity fades the
non-projection matte/border independently.
Add `-ProjectionDepthMeters <meters>` to set the head-anchored projection
surface depth explicitly. The guarded launcher writes
`debug.rustyxr.projection.depth.meters`, defaults to `1.0`, and logs the value
as `projectionDepthMeters` / `panelTargetDepthMeters` so Makepad depth remains
visible beside HWB and GL/OES. For canvas/custom parity runs, pass
`-CameraProjectionMode world-canvas -CameraProjectionGeometryProfile
full-frame-diagnostic` for the canvas-equivalent pass and
`-CameraProjectionMode display-screen-homography -CameraProjectionGeometryProfile
camera-projection` for the per-eye custom projection pass. The Makepad XR panel
is sized and placed from the same projection depth, preview FOV, vertical
offset, and raw overscan values before the camera shader runs. Keep the S91
build-time source mapping at `display-left-from-right-source` for current
projection evidence builds; override `-DisplaySourceEyeMapping` only when the
source-eye mapping itself is the variable under test.
Add `-ProjectionAlphaMode red|green|blue|luma` or an inverse variant when the
valid camera window should reveal native passthrough based on source color.
The Makepad path uses premultiplied RGB, so alpha-zero mask regions do not leak
camera color. `tools\Send-MakepadQ2QHorizontalOffset.ps1` accepts the same
alpha mode, scale, and bias properties for short headset A/B checks.
Add `-ProcessingLayer blur -BlurRadiusPx 2.0` to enable the public diagnostic
blur layer for valid camera samples while keeping the same projection border
policy. The gate writes `debug.rustyxr.makepad.processing.layer` and
`debug.rustyxr.makepad.blur.radius.px`, and the running app also accepts those
properties through `tools\Send-MakepadQ2QHorizontalOffset.ps1` for short
operator A/B checks.

The default broker-H.264 gate uses `127.0.0.1:8765`, left/right stream ports
`8879` / `8880`, `1280x1280`, 6 Mbps, and a live-bounded 45-second stream with
`max_packets=0`. Set `BrokerH264CaptureMs=0` when the gate needs to prove a
live/unbounded stream instead of a bounded capture window. The synthetic
profile also sets `diagnostic-grid`. The
broker-camera profile forwards requested left/right camera IDs and source FPS
to the public broker command. The Makepad path consumes broker stream-header
projection metadata, derives `surface_to_camera`, `screen_to_surface`, and
`screen_to_camera` rows from the current OpenXR view state, and reports decoded
CPU-YUV texture cadence. Treat this as deterministic source/projection-stage
or physical-camera transport parity for diagnostics; zero-copy texture
performance still needs its own run.

For this lane, `max_packets=0` is intentional: it requests the broker's
live/unbounded stream, and `BrokerH264CaptureMs=0` preserves that unbounded
duration through the generated Android broker player. Positive capture
durations remain useful when the test intentionally wants a bounded stream. If
a run reports one packet per eye, the decoder may not receive a complete
access-unit sequence and the source-parity gate is invalid. Do not replace
broker-synthetic H.264 parity with a locally generated texture unless the
question is explicitly renderer smoke rather than broker transport or
cross-stack input equivalence.

The summary records freshness hashes plus app/global GPU-fault, fatal, small
hardware-buffer, stale-marker, broker-H.264 decode, and texture-cadence
counters. Record whether the run was
`launcher-attempt-1`, `launcher-attempt-2`, `direct-vr-fallback`, or
`direct-vr-attempt-1`; do not silently merge those launch classes.

## Known Affordances And Gaps

- Makepad owns the generated Android manifest, Java activities, OpenXR loader
  packaging, debug signing, install, and launch flow.
- The Quest variant generates both a normal launcher activity and an XR activity.
- The Makepad runner starts the launcher activity. End-user startup validation
  should use the launcher path; direct VR-category XR launch is a presentation
  control path when comparing against direct Rusty XR launches.
- Rusty XR runtime profiles are not yet mapped from arbitrary Android intent
  extras into Makepad Rust. This smoke pass reads environment variables for
  desktop/tooling runs. The camera-alignment lane also has a narrow Android
  property hotload adapter for live headset tuning:
  `tools/Send-MakepadQ2QHorizontalOffset.ps1` writes `debug.rustyxr` properties
  for horizontal alignment strength, additive left/right/vertical UV offsets,
  projection-footprint offsets/scales/X-keystone/midpoint bow, the synthetic
  projection-area diagnostic toggle, camera-window content scale,
  projection-border strength, and projection-border policy; the running app
  polls those values. The
  projection-footprint keystone and bow controls are pre-homography diagnostics
  and reset to neutral.
- The first shared-core bridge is deliberately small: resolved profile values
  pass through `rusty-xr-runtime-config` before logging. Camera metadata,
  stream framing, and scorecard models should be added the same way, through
  core crates first and Makepad adapters second.
- The current lane uses synthetic status/stereo-comparison markers, a bounded
  Camera2 metadata/acquisition probe, and a paired Makepad hardware-buffer
  import/projection-mapping probe.
- The maintained Makepad fork now exposes an explicit native OpenXR passthrough
  toggle plus end-frame diagnostics. After a clean install, grant both Scene
  Access and Headset Camera before classifying XR presentation; otherwise the
  permission flow can remain on the regular/preflight activity while app markers
  still emit. Loading/preflight is a failed launch state, not passthrough-off
  visual success. Earlier gates already proved the app-owned panel can visibly
  render live camera content; the remaining parity gap was world-space
  anchoring rather than per-eye camera/head-space mapping. The current gate
  re-enables native passthrough and restores that known-visible world-space
  panel positive control. S67a stayed active-XR and fault-clean but did not
  visibly recover the panel; artifact review traced the visible S62 target to
  the plain world-space vertex path, the higher panel pose, dark clear color,
  direct per-eye YUV sampling, and the thin pale border. S67a2 restored that
  exact visible-panel control on the maintained fork: launcher and direct-XR
  samples both showed the live app-owned panel, six byte-distinct screenshots,
  clean end-frame markers, and no app-process GPU-fault/fatal/small-buffer
  counters. S67b then disabled native passthrough while preserving that panel:
  launcher and direct-XR samples both showed the live app-owned panel against a
  solid black app background, with opaque OpenXR end-frame submission and
  byte-distinct screenshots. The requested non-black clear color did not appear
  visually, and the small 4x4 hardware-buffer warning class remains a tracked
  counter. The active S68 gate keeps that passthrough-off visible-panel state,
  keeps direct per-eye no-swap limited-BT.601 YUV sampling, and moves the panel
  from the S62 world-space transform to active-eye `draw_pass.camera_inv`
  placement. S68 is a placement/liveness gate, not final projection parity:
  static screenshots can prove the panel is visible and fresh, while headset
  motion is needed to classify whether it follows camera/head space instead of
  staying fixed in world space. The first S68 static gate passed in both
  launcher and direct generated-XR paths: the app-owned live camera panel and
  pale border stayed visible with native passthrough disabled, six-frame
  screenshot sequences were byte-distinct, and app/global GPU-fault and fatal
  counters stayed at zero. The small hardware-buffer warning class and
  environment-depth runtime logs remain tracked separately. Operator headset
  inspection then accepted S68 as clean per-eye panel projection. The remaining
  gaps are narrower: the camera feeds are swapped left/right between eyes, and
  panel alignment is not yet good enough for the intended stereo effect. The
  next gates fix source-eye mapping first, then tune alignment against the
  custom Rusty XR target notes before performance comparison. S69 changed only
  the display source-eye selector to `inverted_xr_view_id` /
  `display-left-from-right-source`; acquisition-order source indices remained
  logged separately. Operator review reported that this made the stereo eye
  alignment coherent, but the image is still horizontally mirrored. S69b keeps
  the source-eye mapping and adds a horizontal UV flip. The first S69b guarded
  gate reached active XR on `launcher-attempt-1`, captured six byte-distinct
  screenshots, kept stale S69/S68 path labels absent from the extracted native
  library, and preserved zero GPU-fault/fatal counters. Screenshot review shows
  the horizontal mirror patch active. Operator follow-up clarified that the
  flip is still required; S70 keeps it and changes only geometry: the panel is
  head-centered between display eyes and narrowed to the Rusty XR square camera
  target surface, about `0.92m x 0.92m` at `0.75m` depth, to address overlap
  alignment and horizontal stretch. Operator review accepted the S70 aspect
  correction but found a depth-dependent stereo mismatch: close objects are
  still misaligned while objects near `1m` are nearly coherent. S71 keeps the
  square aspect and mirror/source-eye mapping, but returns placement to the
  active-eye camera-inverse basis so the visible Makepad panel does not add a
  shared physical convergence plane. The guarded S71 launcher gate reached
  active XR on the first attempt, captured six byte-distinct screenshots,
  retained S71 marker strings while stale S70/S69/S68 path labels were absent,
  and preserved zero app/global GPU-fault and fatal counters. Operator headset
  inspection decides whether the remaining close-range mismatch is panel
  convergence or metadata/intrinsics projection. Operator inspection reported
  S71 is slightly worse than S70 for close-range stereo alignment, so S72
  reverts to the S70 visual basis and ports the Rusty XR projection delta into
  UV sampling rather than continuing to move the Makepad panel. This is not a
  scalar shift approximation: the app computes per-source
  `surface_to_camera_uv_homography` rows from Camera2 intrinsics, lens pose,
  stereo reference center, and the head-anchored preview surface, then the
  shader applies the selected 3x3 mapping before the accepted texture flip.
  S72 and S73 did not pass visually: the guarded launcher path reached active
  XR and emitted homography markers, but the panel became a monocolored
  camera-reactive surface while CPU YUV probes still showed live camera
  content. S74 hard-coded the logged homography rows in the shader and
  restored the camera feed, with the known parallax issue still visible. That
  makes dynamic Makepad shader-field delivery the active blocker before
  alignment work can continue. S75 kept the metadata rows dynamic but still
  failed visually after writing through the pre-draw dynamic instance/uniform
  path as well as the existing area patching path. S76 then moved the panel off
  nested `DrawCube` ownership and onto direct draw-vars ownership; the guarded
  launcher gate restored live camera pixels through dynamic metadata rows, with
  six byte-distinct screenshots and zero app/global GPU-fault or fatal
  counters. The remaining parity gap is projected-UV coverage: the Makepad
  shader currently clamps invalid projected UVs, while the public Rusty XR
  target falls back to oriented unprojected content UVs.
- Run-log review after S68 found one important distinction for the next fix:
  S51 already solved horizontal image mirroring with a vertical-only UV flip.
  The S68 issue is source-eye mapping, so S69 should swap the selected
  left/right camera texture sets or active-eye selection without changing the
  accepted no-swap limited-BT.601 color path.
- On Windows, the tested Makepad revision required local `cargo-makepad`
  packaging fixes for generated wrapper paths and dependent Rust shared-library
  bundling.
- Earlier Quest validation reached the generated XR activity and emitted the
  marker, but the device log also reported GPU page faults. That fault class was
  later narrowed below this Rusty XR smoke panel.
- A control run of Makepad's upstream XR example on the same headset also
  reported GPU page faults, so the investigation moved into Makepad's
  Android/Vulkan path rather than this example's scene content.
- The depth-stack comparison showed that Makepad's eager environment-depth path
  differs from the non-Makepad Rusty XR composite stack, but follow-up splits
  still faulted without provider start, per-frame acquire/readback, or depth
  image view creation. Later splits also faulted without passthrough creation,
  without environment-depth provider/swapchain creation, and with zero
  composition layers submitted, without OpenXR color swapchain creation,
  without the OpenXR frame loop, without OpenXR session creation, and without
  Makepad OpenXR instance creation. A same-APK launch of the normal Makepad
  Android activity also reproduced the page-fault class. Later counter-app
  splits moved the active lead into Makepad's Android Vulkan window-swapchain
  recreation after acquire/present reports suboptimal: suppressing that
  recreation stayed clean, the same-toolchain baseline still faulted, and
  waiting the current window-frame fence before recreation stayed clean. That
  wait is now part of the maintained local Makepad fork state and a
  no-diagnostic counter repeat stayed clean. The current synthetic stereo APK
  built against that fork state now passes the launcher and generated-XR
  startup/liveness gate with no app-process GPU page-fault or fatal lines in
  the 90s windows. Repeated small hardware-buffer warnings remain tracked
  separately through Camera2 acquisition and Makepad hardware-buffer import.
  A later active-presentation gate reached Makepad OpenXR session creation and
  showed that passthrough could start while the runtime rejected environment
  depth provider setup; the maintained fork now treats environment depth as an
  optional feature so a depth-policy failure cannot block projection startup.
  See the repository-level GPU investigation note for the current attempt log.

## Validation Status

- `cargo check`: passes for this standalone package.
- Quest APK build: passes with the maintained Makepad fork branch, including
  dependent Rust shared-library bundling.
- Current projection parity slice: the example is on S91. S86 restored real
  camera detail through direct fullscreen YUV sampling, proving the current
  Makepad fullscreen draw/YUV texture path. The maintained Makepad fork now
  exposes runtime per-eye OpenXR pose/FOV state to app Rust, and S87 validated
  that state with a fresh fault-clean device gate and live projected camera
  screenshots. S88 keeps that runtime homography path and ports the public fast
  shader's invalid-UV fallback policy so edge/invalid regions use a dimmed
  oriented content-surface sample instead of an immediate black return. The
  S88 device gate passed for launcher active-XR, runtime-view markers,
  homography-ready markers, byte-distinct screenshots, and fault counters; the
  remaining parity blocker is the close-range projection/parallax geometry
  mismatch against the public fast target, not camera acquisition. S89 keeps
  the S88 projection math but replaces the flattened cube panel with a single
  fullscreen quad so the shader input UV domain matches the target fullscreen
  pass more directly. S90 then removes a source-correlation ambiguity: the
  fork exposes Android camera IDs in Makepad video descriptors, and this
  example binds Makepad video streams to the Camera2 projection plan by camera
  ID before falling back to source index. The S90 launcher gate now shows
  `s90CameraIdSourceBinding=true`, `sourceBindingMode=camera-id`,
  runtime-view/homography readiness, byte-distinct screenshots, and fault-clean
  app/global counters. Operator review rejected S90 as projection parity because
  the visual result still had depth-dependent misalignment/parallax, an
  apparent source-eye flip, and a roll/orientation defect. S91 keeps the S90
  acquisition and Camera2/Makepad camera-ID binding, but separates display-eye
  homography row selection from inverted source-eye texture selection and
  changes the active texture UV orientation from 180-degree `flip-x-and-y` to
  vertical-only. The fresh S91 gate reached active XR, emitted S91 markers,
  captured six byte-distinct screenshots, and stayed fault-clean; treat S91 as
  best-effort until the next headset review.
- Quest launcher run: installs, starts, emits Java activity, native bootstrap,
  `RUSTY_XR_MAKEPAD_Q2Q_STATUS`, and
  `RUSTY_XR_MAKEPAD_STEREO_COMPARISON` startup markers, switches into active
  XR presentation through `XrPermissionsFlow`, and shows the synthetic stereo
  scene in headset. The S14 launcher pass retained app/`XrUpdate`/draw cadence
  near 90Hz, paired camera texture progression near 50Hz, and no app-process
  GPU page-fault or fatal lines.
- Quest generated-XR activity launch: emits the same startup marker set in a
  short capture, reaches Vulkan ready / before main loop, and keeps the app
  process alive in the longer liveness window with no app-process GPU page-fault
  or fatal lines.
- Camera2 metadata/acquisition gate: both Makepad launcher and generated-XR
  startup windows enumerate three Camera2 `PRIVATE` sources, select a
  back-facing 1280x1280 source with intrinsics and pose metadata, receive one
  hardware-buffer-backed frame, and complete the bounded probe with
  `status=ok`. The first-frame descriptor reports native format 35, usage
  131840, one layer, stride 1280, and a present buffer id.
- Makepad hardware-buffer import gate: both Makepad launcher and generated-XR
  short windows enumerate three Makepad camera sources and 66 camera formats,
  select a back-facing 1280x1280 YUV420 source, start the delayed
  `VideoExternal` import path, prepare playback at 1280x1280, and emit
  `makepadVulkanImport=true` on `VideoTextureUpdated`.
- Performance comparison gate: active-presentation comparison reopened after
  S14, but final parity performance is still blocked on visible Makepad camera
  projection. S14 proved launcher-path XR presentation and paired import/cadence
  with the synthetic scene visible. S15 confirmed the custom Rusty XR baseline
  visibly renders proper stereo camera projection, but the current sample was
  performance-degraded. S16 proved marker-level Makepad camera-panel readiness,
  then visual inspection showed the headset still rendering the synthetic
  blue/red debug scene. S17-S20 removed that fallback scene, moved the panel
  under scene-owned `XrNode` routing, and proved a normal Makepad `DrawCube`
  diagnostic panel is visible from the same widget transform. S21 custom-shader
  variants still rendered black, and S22's native `Video` widget surface started
  but did not produce confirmed visible headset-camera playback. S23 added the
  Makepad `Video` headset-camera permission option and removed the unrelated
  custom-shader compile noise, but showed the video widgets must be reset before
  assigning headset-camera sources. S24 added that reset and exposed an Android
  video-cleanup completion gap. S25 tested a Makepad fork cleanup-completion
  patch, but that split was rejected and reverted after it reintroduced the
  Quest app-process GPU page-fault class when the native `Video` widget route
  proceeded. S26 restored the app-fault-clean manual `VideoExternal` route and
  showed the scene-owned cyan diagnostic panel in headset after scene access was
  granted. S27 disabled the solid diagnostic, S28 sampled `sample_video()`
  color directly, and S29 disabled the alignment-guide overlay by default. Use
  the launcher path for Makepad validation, not direct generated-XR launch, and
  verify the paired Makepad textures are the visible XR content before treating
  scorecards as final parity evidence.
- Current comparison target: the accepted source-provenance target run used
  visible stereo Camera2 projection at 72Hz, render scale `0.75`, no stale
  frames, and explicit Quest CPU/GPU level `4` / `4`. Future Makepad parity
  runs must capture the same device performance props, preserve the small
  hardware-buffer warning class separately from GPU-fault counters, and include
  screenshot or headset-cast visual review so a marker-only pass cannot be
  mistaken for projection parity.
- Current S92 performance comparison: the public fast target held about
  `72.9/72Hz` with zero numeric `Tear` / `Stale`, low app-process CPU, paired
  GPU buffers, and `cpuUploadCount=0`. Makepad S91 held about `90.5/90Hz` with
  zero numeric `Tear` / `Stale`, app/XR/draw cadence near `90Hz`, paired camera
  texture updates near `50Hz`, and substantially higher app-process CPU. Both
  runs used CPU/GPU level `4` / `4`, scale factor `0.75`, six byte-distinct
  screenshots, and fault-clean logs. Treat this as a transport/performance
  result only; Makepad projection math still requires headset visual acceptance.
- Current S93 refresh-normalized split: the public fast `0.75` target requested
  and activated `90.000Hz`, held `OpenXR` about `90Hz`, consumed camera pairs at
  `50.001Hz`, rendered projection frames at `90.007Hz`, and averaged `1.800`
  renders per distinct camera frame. Makepad S91 also held app/`XrUpdate`/draw
  cadence around `90Hz` with paired texture updates around `50Hz`. Operator
  headset review found that the Meta performance HUD itself was stereo-misaligned
  while Makepad was active, then stable/aligned after switching back to the
  public Rusty XR target under the same `90Hz` / level-4 device state. Treat the
  active Makepad blocker as XR presentation/view/layer state before further
  shader-only projection tuning.
- Current S95 direct-XR control: starting the generated XR activity directly
  with the Oculus VR category reached the expected runtime/projection markers
  and stayed fault-clean, but did not fix the headset-visible Meta performance
  HUD misalignment. Raw Horizon window metadata matched the public target's
  window class closely enough that the next split needed an upstream Makepad XR
  example as a third baseline before more camera-shader tuning.
- Current S96 upstream-baseline control: an upstream Makepad `dev` XR example
  build entered the generated XR activity, then toggled back to Makepad's
  normal Android activity and displayed a 2D screen. This is the same
  symmetric-activity-toggle failure isolated earlier in the Makepad recovery
  notes. The upstream scene-selection / hand-panel style UI is not a valid HUD
  baseline until the generated XR activity stays foreground through a minimal
  directional XR handoff guard.
- Current S97 guarded-upstream control: the same upstream example with only the
  directional XR handoff guard stayed in the generated XR activity, showed the
  old Makepad scene-picker style UI, and operator review did not see the Meta
  performance HUD stereo misalignment. Upstream GPU page-fault warning lines
  remain visible, so this is a HUD/presentation baseline rather than a
  GPU-fault-clean renderer baseline.
- Current S98 maintained-example control: this example now has a native
  passthrough-on HUD split that keeps the S91 camera/projection shader path but
  restores the two-layer OpenXR submission shape. The direct generated-XR gate
  reached active XR, emitted `s98NativePassthroughHudSplit=true`, submitted
  `nativePassthrough=true`, `projectionBlendSourceAlpha=true`, `layerCount=2`,
  captured six byte-distinct frames, and stayed GPU-fault/fatal clean while
  preserving the small hardware-buffer warning class. Operator headset review
  still saw Meta performance-HUD stereo misalignment, so this split did not
  fix the maintained camera path's HUD presentation defect.
- Current S99 maintained-fork scene-picker control: the original Makepad XR
  scene picker built from the same maintained fork stayed in the generated XR
  activity, submitted the same native-passthrough/two-layer OpenXR frame shape,
  and operator review reported that the Meta performance HUD was not
  stereo-misaligned. The remaining HUD defect is therefore specific to this
  camera example path. The smallest next split is render-scale: S99 used the
  fork's high default XR target size, while this camera example currently
  forces `0.75`.
- Current S100 render-scale control: raising this camera example to the
  scene-picker/default Makepad XR scale preserved HUD alignment only during
  launch and the green camera-arming placeholder. The HUD misalignment appeared
  when live camera content replaced the placeholder, and the high scale
  regressed stale frames, 90 FPS stability, and CPU load. Continue camera-path
  isolation at `0.75`.
- Current S101 camera-feed-suppressed control: camera acquisition/import stayed
  active at `0.75`, but the shader rendered a controlled diagnostic surface
  instead of sampling live YUV after arming. Operator review reported good HUD
  alignment. The remaining trigger is live camera projection/content, with a
  new coverage suspect: the diagnostic surface appeared to cover more area than
  the normal camera projection.
- Current S102/S103 coverage split: S102 kept live YUV sampling active but
  forced full-surface identity coverage, which kept the HUD aligned while
  making the camera feed intentionally full-screen. S103 keeps that full
  submitted surface active and moves coverage into the shader: camera pixels are
  drawn inside a content window with matte and border instead of resizing the
  OpenXR layer. A stable-link S103 rerun reached active XR, produced six
  byte-distinct freshness hashes, and passed operator headset review for HUD
  alignment plus the prior distance-dependent parallax defect. Keep S103 as the
  render-stack baseline; the remaining visual task is horizontal eye alignment
  inside the camera window.
- Current S104/S105 horizontal split: S104's `surface_to_camera` center-delta
  correction was objective-clean but still visibly offset. S105 follows the
  public target's projected shader path more closely by using the
  `screen_to_camera` center delta, then layers hotloadable manual UV offsets on
  top for headset-side fine tuning without rebuilds. The first screenshot
  derived tuning pass selected `Strength=0.425` as the default candidate
  because it best matched the public target's normalized left/right
  camera-content disparity while staying below the higher-strength range where
  edge striping became visible.
- Current S109 validation slice: camera-looking output is not sufficient visual
  proof when native passthrough may be active or the projection is full-screen.
  This slice disables native passthrough for the example and draws an
  unmistakable red border around the app-owned camera projection window. Treat
  screenshots without that marker as launch/presentation evidence only, not
  alignment evidence.
- Current S110 tuning slice: the camera-window sampler has a vertical UV
  hotload knob in the same `debug.rustyxr` property lane as the horizontal
  offsets and content scale. Keep per-device values in run notes or broker
  state; do not bake headset-specific alignment constants into reusable source
  without a separate public validation pass.
- Current cadence probe: rolling `RUSTY_XR_MAKEPAD_CADENCE` samples include
  Makepad `NextFrame`, draw-event, `XrUpdate`, and paired left/right camera
  texture-update counters. The S14 active launcher sample reported
  app/`XrUpdate`/draw cadence near 90Hz and paired texture-update cadence near
  50Hz.
- Current tracked warning: repeated small hardware-buffer lines appear in the
  device logs. They are not counted as GPU page faults, persisted through the
  successful paired import/projection marker gate, and should stay visible in
  the iteration ledger during performance comparison work.
- Current source/build slice: paired Makepad hardware-buffer import and
  projection-mapping markers are validated against the maintained fork branch,
  launcher-path active presentation is validated, the fallback synthetic scene
  has been removed from the visual gate, and the rejected native `Video` widget
  diagnostic is disabled. S32 operator review reclassified the Makepad visual
  evidence as native compositor passthrough plus a low app-owned rectangle, not
  custom projection parity. S33 proved app-owned geometry and improved
  alignment, but the panel still rendered solid split-proof colors rather than
  camera pixels. S34 switched the custom panel to Makepad Y/U/V camera plane
  textures, but device validation still showed proof colors: only one CPU YUV
  stream updated, no paired visual bind completed, and one screenshot exposed an
  unwanted depth-clip/occlusion class where room geometry could cover the app
  panel. S35 keeps panel depth clipping disabled and treats a single updating
  CPU YUV stream as a camera-pixel proof only, not final zero-copy or stereo
  projection parity. The S35 device run did not yet validate that proof because
  the generated XR activity bounced back to the normal activity surface; the
  next slice is an explicit Makepad Android XR activity handoff fix before
  rerunning the YUV proof. The desired visual state for this proof is an
  app-owned panel without runtime depth/environment occlusion; passthrough
  behind the app layer is expected, but real-world geometry covering the panel
  is tracked as a separate depth-clip regression. S36 fixed the activity
  handoff: launcher and direct generated-XR routes now stay in active XR,
  emit YUV-ready/prepared/update and single-stream-proof markers, and remain
  app-fault clean. The visible panel still shows the blue/red proof state
  instead of camera pixels, so S37 focuses on the custom panel's late draw-var
  texture binding/redraw path before paired left/right ownership resumes.
  S37 added an explicit draw-vars-bound marker and kept the same stable/fault
  profile, but the visual result remained blue/red. S38 preferred the actually
  updating YUV stream and removed proof tint, and S39 forced the live
  camera-ready/YUV state onto the active draw area, but both still rendered the
  blue/red proof colors. S40 then made the waiting/default path neutral black
  and set the panel default to camera-ready. The S40 device gate is decisive:
  launcher and direct generated-XR routes stay in active XR, remain
  app-fault clean, keep depth/environment occlusion off, and now render a
  neutral black app-owned panel instead of blue/red. That proves the shader
  edits are active and moves the remaining blocker to Makepad YUV texture
  sampling/content rather than passthrough ambiguity, depth occlusion, activity
  handoff, or stale proof-color state. S41-S45 then proved the camera update
  event carries nonzero CPU-side Y/U/V plane content and kept active OpenXR
  presentation stable through gain-boosted luma, generated R8 Y-slot, generated
  all-slot, and constant shader-bypass controls. S46 proved the visible panel
  executes the edited shader body when the fragment path returns before any
  texture sampling: the panel turned green with the same guide overlay. The
  S47 then visibly sampled a generated R8 texture through the panel's
  `left_tex_y` slot, proving ordinary 2D texture-slot sampling works. The
  S48 disabled that generated replacement and visibly sampled the real Makepad
  camera Y plane through the same direct shader path. S49 removed the earlier
  gain-boost diagnostic and showed a no-gain monochrome camera image in the
  panel. Operator visual review confirmed camera feed visibility and moved the
  next gate to S50: rotate the direct camera-Y sample and center the app-owned
  diagnostic panel for head-forward inspection. S50b passed orientation and
  placement, but operator review identified a remaining left/right mirror. S51
  keeps the centered panel and uses a vertical-only camera-Y flip; the device
  gate retained clean counters and screenshot inspection shows the direct
  camera-Y panel upright, centered, and horizontally corrected. S52 then tried
  direct YUV-to-RGB color conversion in the same early-return shader path. The
  run stayed active-XR and app-fault clean, and CPU-side probes confirmed
  non-empty Y/U/V planes, but the headset view was strongly green/cyan. Per the
  Rusty XR color notes, that result is now treated as sampler/decode-shape
  evidence rather than final color calibration. The S53 gate visualizes the
  sampled Y, U, and V texture slots as separate grayscale bands before channel
  order, range, or matrix tuning resumes. S53 passed that slot-visibility gate:
  the screenshot shows live Y/U/V grayscale bands and clean counters, so the
  remaining color blocker is chroma interpretation. S54 swaps U and V in the
  YUV-to-RGB conversion while keeping the same centered/upright panel; that
  device gate shows real color camera feed instead of the earlier green/cyan
  failure, with the app still active-XR and app-fault clean. S55 keeps that
  plane-order fix and compares swapped-U/V limited/full range plus BT.601/BT.709
  variants in one grid, but its first device screenshot sampled different source
  regions per quadrant. S56 keeps the same formulas and remaps each quadrant to
  the same full camera view; that device gate stayed clean and made the
  comparison fair. The Rusty XR hardware-buffer baseline reports BT.601 narrow
  range for the same camera format, so S57 collapsed the grid into a full-panel
  swapped-U/V limited-BT.601 candidate. S58 removed the center/split diagnostic
  guide and proved the shader edits were active with a border-only overlay, but
  visual review rejected the color claim because skin tones in the panel shifted
  blue/cyan. S59 therefore keeps the border-only panel and vertical flip, samples
  Android YUV_420_888 planes without swapping U/V, and keeps color acceptance
  false until per-eye projection catches up. The S59 run also adds a
  multi-frame screenshot freshness check: future visual gates should record
  whether expected live-camera screenshots are byte-identical before accepting a
  still image as live feed evidence. The first S59 clean install showed why
  launchers should pregrant `horizonos.permission.HEADSET_CAMERA` as well as
  `android.permission.CAMERA`; after that grant, S59 emitted the no-swap marker
  path, stayed app-fault clean, and produced a non-byte-identical screenshot
  sequence.

The current step-by-step implementation ledger is tracked in
[../../docs/MAKEPAD_STEREO_COMPARISON_ITERATION.md](../../docs/MAKEPAD_STEREO_COMPARISON_ITERATION.md).
