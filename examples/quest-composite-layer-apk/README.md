# Rusty XR Quest Composite Layer APK

This example is the first public Rusty XR immersive-Quest example. It is
separate from the minimal Android smoke test:

- the Android activity is a native Quest activity intended to enter OpenXR
  immersive mode
- the Android activity requests camera and headset-camera permissions, then
  starts either a Camera2 YUV diagnostic source or a Camera2 `PRIVATE`
  hardware-buffer probe source based on the selected tier
- the Rust native library owns the OpenXR/Vulkan frame loop and submits a
  custom diagnostic layer
- Tier 0 is a synthetic OpenXR/Vulkan smoke test with no camera
- Tier 1 performs a diagnostic flat camera copy: it converts the latest mono
  Camera2 YUV frame to RGBA on CPU, preserves the raw source aspect, stages a
  bounded preview, and copies the same image into both eye swapchain array
  layers
- `gpu-buffer-probe` requests Camera2 `PRIVATE` frames, imports
  `HardwareBuffer` / `AHardwareBuffer` images into Vulkan, samples them in the
  multiview shader, and logs fallback until pose-backed stereo metadata is
  available
- `gpu-projected` opens paired left/right Camera2 `PRIVATE` sources when the
  runtime exposes them, imports both hardware buffers, and uses the projection
  shader only when each eye has valid intrinsics and platform or public
  estimated-profile pose
- a synthetic fallback remains available for lifecycle smoke tests
- the layer uses the public `PlainStereoLayer`, mono source layout, visual
  border, and performance-hint contracts
- a Java foreground service can request Android MediaProjection consent in the
  headset and stream the final display-composite frames to Windows over the
  public Rusty XR media-pipeline packet protocol

The rendered path is intentionally simple. It proves the app-visible camera
route, layer submission, catalog, permissions, and Windows streaming workflow
without including downstream visual-effect behavior.

## Build

The build script uses Android SDK, NDK, OpenJDK, `aapt2`, `d8`, `zipalign`,
`apksigner`, and the NDK `glslc` shader compiler. It also needs an Android
OpenXR loader library for the APK. Pass it explicitly with `-OpenXrLoaderPath`
or set `RUSTY_XR_OPENXR_LOADER`.

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-composite-layer-apk\tools\Build-QuestCompositeLayerApk.ps1 -OpenXrLoaderPath C:\path\to\libopenxr_loader.so
```

The output APK is:

```text
examples/quest-composite-layer-apk/build/outputs/rusty-xr-quest-composite-layer-debug.apk
```

Build outputs, loader binaries, and APK bytes are ignored and must not be
committed. The debug signing key is stored outside the repo under the current
user's LocalAppData so repeated local builds can reinstall over the previous
debug APK without signature churn.

## Quest OpenXR Activity Context

The native OpenXR setup intentionally initializes the Android loader and
creates the OpenXR instance with the current `AndroidApp` Activity pointer.
`android-activity` exposes an Application context through `ndk-context`; that
is useful for many Android calls, but it is not sufficient for Quest OpenXR
session readiness.

Use `AndroidApp::vm_as_ptr()` and `AndroidApp::activity_as_ptr()` for the
Android loader initialization and Android instance-create chain. If the wrong
context is passed, the app can create an instance but remain stuck at
`OpenXR state IDLE`. Quest runtime logs may also report a legacy or
non-context OpenXR client and `xrCreateSession: Activity is not yet in the
ready state`.

The native entrypoint also waits for foreground lifecycle readiness before
creating the OpenXR/Vulkan session. In practice, the app should be resumed,
focused, and backed by a native window before session setup begins.

## Camera Path Tiers

The visible custom layer is driven by Android Camera2 / headset-camera frames,
not by MediaProjection. The manifest declares `android.permission.CAMERA` and
`horizonos.permission.HEADSET_CAMERA`; the foreground Activity requests both
before starting `HeadsetCameraService`.

The example names the camera path explicitly:

- `synthetic-composite-layer`: Tier 0. It disables the camera and proves the
  OpenXR/Vulkan lifecycle.
- `camera-diagnostic-cpu-copy`: Tier 1. It converts `YUV_420_888` frames to
  RGBA on CPU and uploads the latest frame through a Vulkan transfer copy. This
  is a proof-of-route and diagnostic flat camera copy, not a metadata-backed
  camera/view alignment path. It throttles CPU camera delivery, preserves the
  raw source aspect, and stages a lower-resolution preview even when the
  selected source is larger.
- `camera-gpu-buffer-probe`: GPU-buffer probe. It asks Camera2 for `PRIVATE`
  frames, imports Android hardware buffers with Vulkan external-format /
  YCbCr-aware sampling, renders the sampled mono diagnostic surface, and logs
  `alignedProjection=false` until pose-backed stereo projection is available.
- `camera-source-diagnostics`: enumerates Camera2 IDs, physical IDs, logical
  multi-camera capability, concurrent-camera exposure, `PRIVATE` and YUV
  sizes, FPS ranges, calibration fields, pose fields, and stereo candidate
  reasons. The APK writes an app-private diagnostics JSON file, and Companion
  verification pulls it as `camera-source-diagnostics.json` when `--out` is
  used.
- `camera-stereo-gpu-composite`: Tier 2 stereo projection profile. It opens
  paired left/right Camera2 `PRIVATE` streams when available, imports both
  hardware buffers, scales per-eye intrinsics into the delivered image domain,
  resolves Camera2 sensor/reference pose through the current OpenXR head basis,
  builds a head-anchored preview-FOV content surface, projects that surface
  through each current OpenXR display-eye view/FOV, composes that with the
  selected source-camera projection into one screen-to-camera homography per
  display eye, applies independent left/right texture orientation, and samples
  the selected source-eye texture. Source-eye mapping selects the texture;
  homography selection follows the display eye.
  The verifier rejects it unless one final status line reports paired
  left/right GPU buffers, `stereoLayout=Separate`,
  `activeTier=gpu-projected`, `alignedProjection=true`, no CPU uploads,
  `poseSource=platform` or `poseSource=estimated-profile`, logged source-eye
  mapping, per-eye texture transforms, and explicit manual visual acceptance.

Tier 1 is not the preferred performance path; production adapters should use
GPU-sampled camera buffers, Android hardware-buffer import, and external-format
or YCbCr-aware Vulkan sampling.

The Java camera bridge sends public frame metadata to Rust:

- source label and Camera2 camera ID
- selected delivered image size and timestamp
- lens facing and selector score for diagnostics
- sensor orientation when Camera2 exposes it
- active-array and sensor-pixel domains when Camera2 exposes them
- focal length and principal point when Camera2 exposes intrinsic calibration
- explicit `missingIntrinsics` and `missingPose` flags
- requested and active camera tier labels
- transport label, such as CPU YUV/RGBA or Android hardware buffer
- public GPU-buffer descriptor fields when a `PRIVATE` frame exposes a hardware
  buffer
- `mono` stereo layout and a visible/logged mono-fallback reason
- paired left/right frame statistics when a logical physical or concurrent
  separate stereo provider is exposed and timestamp pairing succeeds
- numeric Camera2 intrinsic calibration, distortion, lens-pose translation,
  lens-pose rotation, lens-pose reference, selected stereo-pair score, and
  selected stereo-pair reason in `camera-source-diagnostics.json`

Useful launch extras:

- `rustyxr.camera`: `true` by default; set `false` for synthetic renderer
  smoke tests.
- `rustyxr.cameraTier`: `synthetic`, `cpu-diagnostic-flat-copy`,
  `camera-source-diagnostics`, `gpu-buffer-probe`, or `gpu-projected`. The
  GPU-buffer probe samples imported camera buffers without claiming stereo
  alignment. The `gpu-projected` tier is reserved for metadata-backed
  projection and must keep `alignedProjection=false` when pose or stereo
  metadata is missing.
- `rustyxr.cameraWidth` / `rustyxr.cameraHeight`: requested camera target
  dimensions. The default profile requests `1280x1280`.
- `rustyxr.cameraPreferredSquare`: preferred square Camera2 size. The default
  is `1280`.
- `rustyxr.cameraMaxDimension`: preferred maximum Camera2 dimension before
  larger formats are deprioritized. The default is `1920`.
- `rustyxr.cameraStereoLayout`: `mono`, `side-by-side`, `top-bottom`, or
  `separate`. The final stereo profile requests `separate`; it must not be
  treated as successful unless paired left/right GPU buffers are active.
- `rustyxr.cameraStereoPairMaxDeltaNs`: maximum left/right timestamp delta for
  pairing separate GPU camera frames. The public profile uses `5000000`.
- `rustyxr.cameraSourceEyeMapping`: `left-right` or `right-left`. It controls
  whether the display left eye samples the left/source-0 camera or the
  right/source-1 camera. This is a runtime visual diagnostic knob because
  Camera2 source IDs are device/runtime observations, not portable eye labels.
  The sample stereo profile uses `left-right` and relies on diagnostics plus
  manual inspection before acceptance.
- `rustyxr.leftCameraTextureRotation` / `rustyxr.rightCameraTextureRotation`:
  `rotate0`, `rotate90`, `rotate180`, or `rotate270`, with matching
  `FlipX`, `FlipY`, and `Mirror` extras for each eye. These transforms are
  applied after projection UV calculation and before sampling. The projected
  stereo profile starts at `rotate0` with no flip for each eye on the current
  Quest Camera2 hardware-buffer path. `FlipY` is kept as a diagnostic override
  for paths that expose a different texture origin; flat diagnostic surfaces
  may need a different transform because they do not use the same projection UV
  path.
- `rustyxr.cameraOrientationDiagnosticMode`: `off`,
  `cycle-source-eye-mapping`, `cycle-left-texture-transform`,
  `cycle-right-texture-transform`, or `cycle-all`. Cycling modes are for live
  diagnosis only and do not count as visual acceptance.
- `rustyxr.visualReleaseAccepted`: `false` by default. A release run may set it
  only together with `rustyxr.visualAcceptanceToken=manual-visual-accepted`
  after manual headset/cast inspection confirms upright imagery, correct
  source-eye mapping, stable projection under head motion, and a visible
  camera-driven soft border that blends back into the raw projected camera
  image rather than covering the interior with an opaque rim.
- `rustyxr.cameraEstimatedPose`: `false` by default. Set it only with an
  explicit calibration profile, together with `rustyxr.cameraEstimatedPoseX`,
  `Y`, `Z`, `Qx`, `Qy`, `Qz`, and `Qw`, to mark the pose source as
  `estimated-profile`. Estimated-profile pose is logged explicitly and is
  never treated as a platform-provided pose.
- `rustyxr.cameraEstimatedStereoPose`: `false` by default. Set it only when
  both `rustyxr.cameraEstimatedLeftPose*` and
  `rustyxr.cameraEstimatedRightPose*` launch extras define valid finite
  per-eye poses. Optional `rustyxr.cameraEstimatedPoseLabel`,
  `rustyxr.cameraEstimatedPoseVersion`, and
  `rustyxr.cameraPoseCoordinateConvention` describe the public calibration
  profile. A single shared estimated pose is not enough for stereo alignment.
- `rustyxr.cameraAllowCpuFallback`: set `false` when validating that a Tier 2
  request does not silently use the Tier 1 CPU path.
- `rustyxr.cameraCpuUploadHz`: diagnostic CPU conversion/upload cadence. The
  MVP CPU `YUV_420_888 -> RGBA -> Vulkan copy` path samples at roughly `4 Hz`
  at the ImageReader boundary so the OpenXR frame loop can continue submitting
  between camera uploads. Use `0` to disable CPU camera frame delivery while
  keeping the custom OpenXR layer running.
- `rustyxr.mediaProjection`: `false` by default; set `true` only when the
  final screen should be streamed back to Windows.

Treat selected Camera2 IDs as runtime diagnostics, not portable requirements.
The public selector preference is: stereo source when exposed by an adapter,
preferred/back-facing source, requested square size, formats at or below the
configured maximum dimension, square formats, pixel count, then frame rate.

If a platform provides camera intrinsics in an active-array sensor domain,
scale focal length, principal point, sensor resolution, and skew into the
delivered per-eye stream resolution before projecting. For side-by-side stereo
that per-eye resolution is half the delivered width; for separate-eye streams
it is the full delivered stream size for each eye. Normalize camera poses before
using them for eye selection or projection.

The alignment renderer logic is: build the head-anchored camera-content surface
once, project that surface through each current OpenXR display-eye view/FOV
into fullscreen display UV, compose that display-eye mapping with the selected
Camera2 source projection, and send one screen-to-camera homography per
display eye to the shader. The shader samples the mapped source camera texture
while keeping a separate content UV domain for the public soft border. When
intrinsics or pose metadata is missing, this example logs the fallback reason
and remains a GPU-buffer probe.
If a profile supplies an estimated calibration pose, diagnostics say
`poseSource=estimated-profile`; no default estimated pose is baked into the
public example. The Tier 1 profile visibly uses the diagnostic flat camera
copy; the GPU-buffer probe keeps that CPU copy disabled by default so it is not
mistaken for aligned projection.
Mono Camera2 fallback is labeled as mono fallback and should not be treated as
true stereo alignment.

The performant custom projection path does not do CPU YUV conversion,
per-frame CPU resampling, or full-eye staging uploads. It uses Camera2
`PRIVATE` hardware buffers imported as GPU-sampled textures, shader-space
projection/overscan/edge fade, a bounded external-buffer import cache, and no
environment-depth, environment-cube, physics, or MediaProjection work. It still
falls back to a flat diagnostic surface when pose or true stereo metadata is
missing.

## Run Through Companion

For a camera-only diagnostic renderer validation, install and launch from the
catalog with the `camera-diagnostic-cpu-copy` runtime profile:

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- catalog verify --path .\examples\quest-composite-layer-apk\catalog\rusty-xr-quest-composite-layer.catalog.json --app rusty-xr-quest-composite-layer --serial <serial> --stop-catalog-apps --install --launch --device-profile xr-composite-smoke-test --runtime-profile camera-diagnostic-cpu-copy --settle-ms 7000 --logcat-lines 1000 --out .\artifacts\verify
```

For GPU-buffer validation, use the probe profile. In the current public example
success means GPU hardware buffers are imported and sampled by Vulkan, with
fallback clearly logged; it does not yet mean the headset view is a projected
stereo camera composite.

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- catalog verify --path .\examples\quest-composite-layer-apk\catalog\rusty-xr-quest-composite-layer.catalog.json --app rusty-xr-quest-composite-layer --serial <serial> --stop-catalog-apps --install --launch --device-profile xr-composite-smoke-test --runtime-profile camera-gpu-buffer-probe --settle-ms 7000 --logcat-lines 1000 --out .\artifacts\verify
```

For Windows screen-stream validation, start a Windows media receiver and
reverse the TCP port before launching with MediaProjection enabled:

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- media reverse --serial <serial> --device-port 8787 --host-port 8787

dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- media receive --port 8787 --out .\artifacts\media-stream --once
```

Then install and launch from the catalog with the `media-projection-stream`
runtime profile:

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- catalog verify --path .\examples\quest-composite-layer-apk\catalog\rusty-xr-quest-composite-layer.catalog.json --app rusty-xr-quest-composite-layer --serial <serial> --stop-catalog-apps --install --launch --device-profile xr-composite-smoke-test --runtime-profile media-projection-stream --media-receiver --settle-ms 7000 --logcat-lines 1000 --out .\artifacts\verify
```

If the app requests MediaProjection, accept the headset popup. Custom launchers
and desktop tools can assist with install, launch, ADB reverse, and diagnostics,
but they cannot bypass the headset consent prompt for normal user-facing
MediaProjection capture.

If the headset shows a system controller or consent dialog, complete that
prompt in the headset. ADB can install, launch, reverse ports, and capture
diagnostics, but it cannot provide physical controller tracking or grant
MediaProjection consent on behalf of the user.

On current Quest system UI, the MediaProjection flow can include a second
`Select view you want to share` panel. Select `Entire view` in the headset,
then press `Share`. Shell taps and UIAutomator dumps can see parts of this
panel but cannot reliably select the view or enable `Share`, so validation
harnesses should treat this as a required manual step.

## Expected Signals

- the resumed activity is `com.example.rustyxr.composite/.CompositeLayerActivity`
- logcat contains `Rusty XR initialized Android OpenXR loader with Activity context`
- logcat contains `Rusty XR composite layer contract`
- logcat contains `Rusty XR camera path config`
- logcat advances from `OpenXR state IDLE` to `READY`, `SYNCHRONIZED`,
  `VISIBLE`, and `FOCUSED`
- logcat reports swapchain creation and recurring `OpenXR frame` messages
- logcat contains `Headset camera capture session running`
- logcat contains `Rusty XR received headset camera frame`
- with `camera-diagnostic-cpu-copy`, logcat contains
  `Rusty XR uploaded diagnostic flat camera copy frame`
- with `camera-gpu-buffer-probe`, logcat contains
  `Rusty XR GPU-sampled diagnostic camera surface` or a clear GPU fallback
- with `camera-stereo-gpu-composite`, logcat must contain one
  `Rusty XR final projection status` line with `activeTier=gpu-projected`,
  `alignedProjection=true`, `stereoLayout=Separate`,
  `pairedLeftRightGpuBuffers=true`, `cpuUploadCount=0`, and
  `poseSource=platform` or `poseSource=estimated-profile`, plus
  `sourceEyeMapping`, `leftCameraTextureTransform`,
  `rightCameraTextureTransform`, `orientationCheck=true`,
  `orientationAccepted=true`, `visualInspection=accepted`, and
  `visualReleaseAccepted=true`
- logs are not sufficient for release: inspect the headset view, cast, or a
  screencap and fail the release gate if the feed is upside down, the border is
  absent, or the per-eye content is visibly swapped or divergent
- with `camera-diagnostic-cpu-copy`, headset view, cast, or screenshot shows
  the mono diagnostic camera surface copied into both eye layers with the
  public fallback/border region around it
- with `camera-gpu-buffer-probe`, CPU camera upload remains disabled unless
  explicitly requested and the visible camera surface is not claimed as aligned
  until imported-texture projection is active
- after consent, logcat contains `MediaProjection stream frame`
- the Windows receiver writes `display_composite_*.rgba` frames plus
  `frames.jsonl`

For an OpenXR-only camera smoke test, launch with MediaProjection disabled:

```powershell
adb shell am start -a android.intent.action.MAIN -c com.oculus.intent.category.VR -n com.example.rustyxr.composite/.CompositeLayerActivity --ez rustyxr.camera true --ez rustyxr.mediaProjection false --es rustyxr.cameraTier cpu-diagnostic-flat-copy --ei rustyxr.cameraWidth 1280 --ei rustyxr.cameraHeight 1280 --ei rustyxr.cameraPreferredSquare 1280 --ei rustyxr.cameraMaxDimension 1920 --ei rustyxr.cameraCpuUploadHz 4
```

Use this mode first when debugging loading or black-screen behavior. If OpenXR
reaches `FOCUSED`, camera frames arrive, and upload logs continue in this mode,
then the renderer and camera route are up. Any later blockage is likely in the
MediaProjection consent/selector flow or stream receiver path.

If a one-frame Windows receiver is used, a later app-side broken pipe can be
expected after the receiver exits. Treat the first received frame and matching
log line as the success signal for that short validation mode.

This example does not use native compositor passthrough, environment depth,
room mesh providers, private visual-effect layers, or downstream effect-stack
code.
