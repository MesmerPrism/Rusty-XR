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
- an optional broker H.264 consumer probe can request the public broker APK's
  app-camera H.264 side channel over device-local localhost, consume the
  `RXYRVID1` binary packets in the composite app process, and decode them with
  Android platform MediaCodec into either byte buffers or a Java-owned
  `SurfaceTexture` external texture, or into `ImageReader` `PRIVATE`
  hardware buffers that feed the existing Vulkan hardware-buffer import and
  OpenXR projection-layer draw path
- the layer uses the public `PlainStereoLayer`, mono source layout, visual
  border, and performance-hint contracts
- a Java foreground service can request Android MediaProjection consent in the
  headset and stream the final display-composite frames to Windows over the
  public Rusty XR media-pipeline packet protocol

The rendered path is intentionally simple. It proves the app-visible camera
route, layer submission, catalog, permissions, and Windows streaming workflow
without including downstream visual-effect behavior.

## Current Status And Known Gaps

The public stereo stack now has two working projected-camera modes:

- `display-screen-homography`: the fullscreen Vulkan multiview path. This is
  the current accepted public baseline for the paired Camera2 GPU-buffer
  projection and public soft feedback border.
- `quad-surface`: an A/B comparison path that reconstructs the content-surface
  coordinates a head-anchored quad would rasterize while still running through
  the public Vulkan fullscreen plumbing.

Both modes share the same paired Camera2 `PRIVATE` hardware-buffer import,
per-eye intrinsics/pose projection, explicit source-eye mapping, texture
orientation controls, and camera-driven border coordinate path. The
quad-surface profile is useful enough for collaboration and diagnosis, but it
is not yet the expected final performance or color reference. Optimized
downstream implementations can still be faster and can differ in final tone
mapping. Treat remaining quad-surface performance and color differences as an
open public example task, not as a blocker for the documented camera-stack
architecture.

## Build

The build script uses Android SDK, NDK, OpenJDK, `aapt2`, `d8`, `zipalign`,
`apksigner`, and the NDK `glslc` shader compiler. It also needs an Android
OpenXR loader library for the APK. Pass it explicitly with `-OpenXrLoaderPath`
or set `RUSTY_XR_OPENXR_LOADER`.

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-composite-layer-apk\tools\Build-QuestCompositeLayerApk.ps1 -OpenXrLoaderPath C:\path\to\libopenxr_loader.so
```

The script defaults to target SDK `35`. Use `-TargetSdkVersion 33` only as a
local compatibility probe when comparing headset camera behavior across
runtime/permission policy, and record that value with the run artifacts.

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
- `environment-depth-diagnostics`: keeps app camera and MediaProjection paths
  off, starts the OpenXR environment-depth provider with native passthrough
  active, acquires at most once per XR frame, logs swapchain size, near/far
  range, runtime capture timestamps, acquire cost, observed acquire/depth
  cadence, hand-removal state, and confidence availability, and renders the
  current stereo depth texture in headset as a per-eye grayscale diagnostic,
  with the same rotate/flip UV transform semantics used by the camera
  projection shader.
- `environment-depth-mesh-overlay`: uses the same provider path but renders a
  transparent generated depth-grid surface over the submitted native
  passthrough underlay. The mesh path samples the stereo environment-depth
  texture in the vertex shader, reconstructs local-space points from the depth
  image pose, and projects those points through the current render-eye pose
  instead of rasterizing a fullscreen screen overlay. The wire pattern is
  computed from reconstructed depth positions, not from screen UVs. It uses a
  single dominant surface grid per fragment and colors mesh distance from the
  metric environment-depth sample with a ramp clamped at about 3 meters, while
  still marking local depth discontinuities so mesh stability can be inspected
  before CPU or TSDF chunk integration. The direct depth-swapchain visualizer
  intentionally draws only the current acquired image; persistent history needs
  owned depth copies or TSDF chunks rather than reused runtime swapchain images.
- `environment-depth-particle-overlay`: reconstructs accepted depth samples
  into the OpenXR local reference space, retains them in a GPU buffer, and
  renders them as metric billboards over the submitted passthrough underlay.
  The current implementation is a diagnostic bridge rather than a final scene
  map: the retained samples are still sourced from a regular view-sampled grid,
  so headset motion can make the visible sample lattice feel view-attached even
  though the samples are written in local scene coordinates. Use
  `environment-depth-scene-particle-map` for the first scene-owned variant with
  explicit confidence, merge/replace, cell-resolution, and retirement policies.
  See
  [environment depth particle anchoring](../../docs/ENVIRONMENT_DEPTH_PARTICLE_ANCHORING.md).
- `environment-depth-scene-particle-map`: keeps the same environment-depth
  provider and passthrough underlay active, but writes accepted samples into a
  persistent local-space particle map. Candidate depth points are quantized into
  metric local cells, mapped through a bounded spatial hash, confidence-blended
  with existing particles in the same cell, actively corrected from
  high-confidence visible free-space observations, and faded when stale.
  Invalid candidate samples preserve existing cells instead of clearing
  raster-derived slots, so cell lifetime is owned by the scene map. The draw
  path uses small alpha-clipped opaque default-disc particles for readable
  real-time scan feedback. This is a visual headset-validation map, not a CPU
  point-cloud, TSDF, or mesh export.
- `meta-hand-mesh-particles`: keeps camera and environment depth off, requests
  OpenXR hand tracking plus `XR_FB_hand_tracking_mesh`, retrieves the immutable
  hand bind mesh once per hand, skins it from per-frame
  `xrLocateHandJointsEXT` joint poses in a stage-anchored reference space when
  available, and feeds the resulting live mesh snapshots through the same
  sampler. The sampled coordinates keep their intra-hand neighbor tiers and
  establish a bounded cross-hand neighborhood for later interactions.
- `passthrough-only-layer-probe`: submits the native passthrough composition
  layer without the OpenXR projection layer. Use it to isolate compositor
  passthrough visibility from projection-layer alpha and overlay rendering.
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
- `rustyxr.cameraAcquisition`: `java-camera2` by default. Set
  `native-ndk` only for the experimental native acquisition probe, which uses
  Android NDK `ACamera*` plus `AImageReader` `PRIVATE` hardware buffers and
  feeds the same Vulkan projection path. Keep this as a separate acquisition
  axis from projection, border, and shader color checks.
- `rustyxr.cameraStartDelayMs`: optional delay before requesting/starting the
  headset camera. Use it to test acquisition lifecycle timing without changing
  projection, border, or shader code.
- `rustyxr.nativeSourceMode`: free-form label for native acquisition source
  experiments. It is logged with native camera selection so headset runs can
  distinguish automatic synthetic dual-back selection, explicit side IDs, and
  other source-shape probes.
- `rustyxr.cameraWidth` / `rustyxr.cameraHeight`: requested camera target
  dimensions. The default profile requests `1280x1280`. Explicit non-square
  requests, such as `1280x960`, are honored when the runtime exposes that size
  and the preferred-square override is disabled.
- `rustyxr.cameraPreferredSquare`: preferred square Camera2 size. The default
  is `1280`. Set it to `0` when testing a non-square acquisition size.
- `rustyxr.cameraMaxDimension`: preferred maximum Camera2 dimension before
  larger formats are deprioritized. The default is `1920`.
- `rustyxr.cameraStereoLayout`: `mono`, `side-by-side`, `top-bottom`, or
  `separate`. The final stereo profile requests `separate`; it must not be
  treated as successful unless paired left/right GPU buffers are active.
- `rustyxr.cameraStereoPairMaxDeltaNs`: soft left/right timestamp target for
  pairing separate GPU camera frames. The public profile uses `5000000`.
  Separate Camera2 streams can drift outside that target under concurrent
  delivery, so the service publishes the latest available left/right pair
  instead of starving the renderer, and logs `softPairOverMax` plus per-pair
  deltas for release validation. The Java pending-pair queue stays small, but
  the stereo `ImageReader` uses a deeper opaque-buffer pool so Vulkan-retained
  `AHardwareBuffer` imports do not starve Camera2.
- `rustyxr.cameraStereoImageReaderMaxImages`: max image count for each
  separate-eye `PRIVATE` `ImageReader`. The default public profile uses `8` so
  Java `Image` ownership, Camera2 producer buffers, and Vulkan-retained
  hardware-buffer imports have enough slack to avoid producer starvation.
  `3` is kept as an explicit acquisition diagnostic because some lower-level
  camera stacks retain fewer images; test it separately from AE target FPS
  changes.
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
- `rustyxr.cameraColorMode`: `external-rgb` by default. Use
  `external-cr-y-cb-bt601-narrow` when the external sampler exposes
  narrow-range BT.601-like channels in the observed `Cr/Y/Cb` order instead of
  already normalized RGB. The mode is applied before raw projection color,
  luma, and border feedback are resolved.
- `rustyxr.cameraColorContrast`, `rustyxr.cameraColorBrightness`, and
  `rustyxr.cameraColorSaturation`: public tone controls applied after any
  external-format normalization and before luma or border feedback is resolved.
  The final projected stereo profile and the quad-surface A/B profile use a
  small contrast/brightness lift so the camera-driven border and the projected
  raw feed stay in the same color domain.
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
- `rustyxr.cameraTargetFps`: optional Camera2 sensor-delivery request. When set
  without `rustyxr.cameraFpsMin` / `rustyxr.cameraFpsMax`, the service requests
  a fixed `CONTROL_AE_TARGET_FPS_RANGE`; the final projected stereo profiles
  request `72-72`, then log the selected supported range if the Camera2 HAL
  chooses a lower or wider range.
- `rustyxr.cameraFpsMin` / `rustyxr.cameraFpsMax`: optional Camera2 AE target
  FPS range request. This is an exposure/capture request to the Android camera
  stack, not a hard delivery guarantee. Horizon OS, the Camera2 HAL, stream
  size/format, exposure, lighting, concurrent-camera use, thermal state, and
  stream min-frame-duration limits can still produce a lower or different
  cadence.
- `rustyxr.mediaProjection`: `false` by default; set `true` only when the
  final screen should be streamed back to Windows.
- `rustyxr.brokerH264Consumer`: `false` by default. Set it to `true` for the
  composite app to run a bounded broker H.264 consumer probe. The probe sends
  `camera_provider.start_app_camera_h264_stream` to a broker already running
  on `127.0.0.1:8765`, connects to the broker's device-local H.264 binary
  stream, decodes the packets with Android platform MediaCodec, logs
  `rusty.xr.composite.broker_h264_consumer_probe.v1`, and stops. The default
  output mode is `surface-texture`, which renders decoder output to a
  Java-owned external OES texture through `SurfaceTexture`; `byte-buffer`
  remains available for regression comparisons. `hardware-buffer` decodes into
  an `ImageReader` `PRIVATE` surface, sends the acquired `HardwareBuffer`
  through the native GPU-frame bridge, and lets the existing Vulkan
  GPU-buffer-probe path draw it into the OpenXR projection layer. When the
  broker stream-start result includes selected Camera2 projection metadata, the
  decoded hardware-buffer frame forwards intrinsics, lens pose, pose source,
  pixel domains, and sensor orientation to the native metadata parser. Launch
  this with
  `rustyxr.camera=false` when isolating broker-owned capture from composite-app
  consumption. Optional extras:
  `rustyxr.brokerHost`, `rustyxr.brokerPort`,
  `rustyxr.brokerH264StreamPort`, `rustyxr.brokerH264CameraId`,
  `rustyxr.brokerH264LeftCameraId`, `rustyxr.brokerH264RightCameraId`,
  `rustyxr.brokerH264Width`, `rustyxr.brokerH264Height`,
  `rustyxr.brokerH264CaptureMs`, `rustyxr.brokerH264MaxPackets`,
  `rustyxr.brokerH264BitrateBps`,
  `rustyxr.brokerH264FrameRateHz`, and
  `rustyxr.brokerH264DecodeTimeoutMs`,
  `rustyxr.brokerH264DecodeOutputMode`, `rustyxr.brokerH264SourceMode`,
  `rustyxr.brokerH264SyntheticPattern`, `rustyxr.brokerH264LiveStream`, and
  `rustyxr.brokerH264LiveDecode`. The default source mode is
  `broker-camera`, which asks the running broker to start an app-context
  Camera2-to-H.264 stream. Set `rustyxr.brokerH264SourceMode=broker-synthetic`
  to request the broker's deterministic MediaCodec synthetic H.264 source
  instead; `rustyxr.brokerH264SyntheticPattern` can be `diagnostic-grid`,
  `checkerboard`, `luma-ramp`, or `motion-bar`. `diagnostic-grid` includes a
  checkerboard-anchored 1-pixel white line overlay for blur/projection
  diagnostics. `rustyxr.brokerH264FrameRateHz` requests the broker synthetic
  encoder cadence; the observed packet/decode cadence should be treated as the
  measured result because device encoder support may clamp or fall back. Set
  `rustyxr.brokerH264SourceMode=existing-stream` when a broker TCP proxy,
  laptop test source, or other tool has already exposed a `RXYRVID1` H.264
  stream on the configured port. Existing-stream mode skips the broker start
  command and only tests receive, decode, hardware-buffer handoff, and OpenXR
  draw. When
  `rustyxr.brokerH264LiveStream=true` and
  `rustyxr.brokerH264LiveDecode=true`, stereo hardware-buffer mode decodes
  packets as they arrive, pairs left/right decoded frames with a small queue,
  submits them immediately through the native stereo bridge, and closes the
  Java `HardwareBuffer` handles after native acquisition. Live decode also logs
  periodic `event=progress` `rusty.xr.composite.broker_h264_consumer_probe.v1`
  reports on the same `Rusty XR broker H.264 consumer probe:` marker so
  long-running validation can sample packet, decode, and native-pair cadence
  before the stream reaches its terminal report. For sustained broker-synthetic
  performance validation, use `rustyxr.brokerH264MaxPackets=0` and set
  `rustyxr.brokerH264CaptureMs` longer than the profile warmup plus sampling
  window so freshness checks run while decoded frames are still arriving. Set
  `rustyxr.brokerH264LiveDecode=false` to force the older retained-clip replay
  path for regression comparison.
- `rustyxr.openxrPassthroughProbe`: `off` by default. `client` creates an
  optional `XR_FB_passthrough` client/layer for runtime-state diagnostics;
  `warmup` creates and resumes the layer briefly, then pauses passthrough.
  This does not replace the custom camera composite and should be tested
  separately from acquisition and color changes.
- `rustyxr.depth`: `off` by default. `status` starts the environment-depth
  provider and logs acquisition diagnostics without changing the render clear
  color. `visualize` also maps acquisition state to the headset clear color:
  blue while waiting, green after a fresh acquired depth image, amber when the
  runtime reports no image for a frame, and red after an acquire error. This is
  a provider/cadence/status diagnostic, not a false-color depth-map renderer.
- `rustyxr.depthHandRemoval`: `false` by default. When supported by the
  runtime, this requests environment-depth hand removal before provider start
  and logs whether the setting was supported and applied.
- `rustyxr.handParticles`: `off` by default. `meta`, `openxr`, `hand-mesh`,
  or `on` enables the live OpenXR hand mesh path using `XR_EXT_hand_tracking`
  and `XR_FB_hand_tracking_mesh`; the app logs extension availability, hand
  mesh bind data, per-frame sampler status, and cross-hand neighbor link
  counts.

Camera delivery cadence and render cadence are separate. The GPU path can
request an AE target range for the Camera2 producer, while the OpenXR renderer
continues submitting at the headset display cadence and reuses the latest
available camera buffer between deliveries. When `XR_FB_display_refresh_rate`
is exposed, the example requests `72 Hz` and logs `activeDisplayRefreshHz` in
the recurring `OpenXR frame` line. Use logcat lines beginning with
`Camera2 AE FPS range` and `Camera2 delivery stats` to compare the requested
range, applied supported range, and observed image timestamp cadence.
Use the launch extra `rustyxr.xrDisplayRefreshHz=90.0` when a comparison must
match a 90 Hz app shell. Use `rustyxr.cameraTargetFps` only for Camera2
capture-rate experiments; it does not force display refresh. The final
projection status line also reports `cameraProjectionRenderFrameCount`,
`cameraDistinctFrameCount`, `cameraRepeatedRenderFrameCount`,
`cameraRendersPerCameraFrameAvg`, `cameraConsumedFrameHz`, and
`cameraProjectionRenderHz` so diagnostics can separate display-frame
submission from new camera-frame consumption.
Android documents
[`CONTROL_AE_TARGET_FPS_RANGE`](https://developer.android.com/reference/android/hardware/camera2/CaptureRequest#CONTROL_AE_TARGET_FPS_RANGE)
as an auto-exposure target range whose actual maximum can still be capped by
stream min-frame durations; Meta's
[Passthrough Camera API overview](https://developers.meta.com/horizon/documentation/unity/unity-pca-overview/#performance)
lists a `60Hz` data rate for the public PCA stack. In live validation on the
paired Quest 3S `50/51` Camera2 path,
`30-30` was honored at about `29.85 FPS`, while `60-60` was accepted by
Camera2 but delivered about `49.9 FPS` with the current concurrent
`1280x1280` stereo GPU-buffer configuration. If `72-72` is not supported or
does not deliver at display cadence on a device build, treat the
`Camera2 AE FPS range`, `Camera2 delivery stats`, and
`Stereo headset camera pair` lines as the concrete capture-side blocker while
the OpenXR renderer continues targeting `72 Hz`.

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
display eye to the shader. Because this public sample renders with a fullscreen
multiview pass rather than an actual projected quad mesh, it also sends
per-eye screen-to-content-surface and content-surface-to-screen homographies in
a uniform buffer. The shader samples the mapped source camera texture while
reconstructing the same content UV domain that a real head-anchored quad would
have produced. The border's guide, edge, and trail samples are routed through
the same final display-eye projection mapping before sampling camera content.
The brightness trigger for border bleed uses the current final projected camera
sample at that display pixel, with a small projected-neighborhood smooth, so
the soft bleed follows dark regions in the visible stereo composite instead of
raw left/right texture UVs, fullscreen screen UVs, or an inward guide sample.
`rustyxr.cameraProjectionMode` selects the display mapping used by the
projected profile. The default `display-screen-homography` mode renders a
fullscreen multiview pass and composes display-eye screen UVs back into the
head-anchored content surface before sampling the source camera. The
`quad-surface` comparison mode uses the same display-eye homography to recover
the content-surface UV that a real head-anchored quad would have rasterized,
then projects that surface point into the selected Camera2 source. This keeps
camera projection, per-eye source selection, and border sampling equivalent to
the accepted fullscreen path while leaving a stable launch switch for future
mesh-quad A/B work. Use the comparison mode for projection-geometry, sampler,
and color checks; do not treat it as a different camera source or downstream
effect stack. Current validation keeps this mode visually gated because its
performance and final color still need optimization against downstream
reference implementations.
When intrinsics or pose metadata is missing, this example logs the fallback
reason and remains a GPU-buffer probe.
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

The current Quest Camera2 hardware-buffer path is sampled through Vulkan
external-format import, and the public projected stereo profiles use
`rustyxr.cameraColorMode=external-rgb` with
`rustyxr.cameraColorContrast=1.1`, `rustyxr.cameraColorBrightness=0.04`, and
`rustyxr.cameraColorSaturation=1.0`. Use
`external-cr-y-cb-bt601-narrow` only as a diagnostic switch when a
device/runtime exposes YCbCr-like channels at the shader boundary; this switch
does not move the live path back to CPU-readable YUV frames.

The April 30, 2026 headset comparison pass confirmed this distinction on the
current public Vulkan path: the combined immutable-sampler `external-rgb` mode
is the usable baseline, while combined-sampler
`external-cr-y-cb-bt601-narrow` can produce a strongly green/discolored image.
Treat that result as a sampler/decode diagnostic, not as evidence that
projection, source-eye mapping, or the border regressed.

The catalog keeps camera path experiments as separate runtime profiles:

- `broker-h264-consumer-probe`: synthetic OpenXR layer plus broker H.264
  consumer fixture. It keeps the composite app's own camera path off, asks a
  running broker APK for a bounded app-camera H.264 stream over device-local
  localhost, and decodes the packets with Android MediaCodec into a
  `SurfaceTexture` external texture. This is a decode-consumption,
  decoder-surface, and external-texture update probe, not yet a Vulkan/OpenXR
  image handoff.
- `broker-h264-openxr-layer-probe`: OpenXR projection-layer fixture for broker
  H.264. It keeps the composite app's own Camera2 acquisition off, decodes the
  broker H.264 stream into `ImageReader` `PRIVATE` hardware buffers, passes
  those buffers through the same native `AHardwareBuffer` import bridge used by
  Camera2 GPU-buffer probes, and draws the decoded feed with the existing
  Vulkan GPU-buffer-probe path. The broker stream-start result now attaches the
  selected Camera2 source metadata when available, so this profile can verify
  `intrinsics=available`, `pose=available`, and `poseSource=platform` across
  broker encode, client decode, Vulkan import, and OpenXR draw. This is still a
  mono diagnostic feed path; aligned stereo projection requires paired source
  buffers and per-eye metadata.
- `broker-h264-stereo-openxr-projection-probe`: two-stream broker H.264
  fixture for finding the practical stereo limit. It starts independent
  left/right broker streams, decodes each into `ImageReader` `PRIVATE`
  hardware buffers, pairs decoded frames by index in the retained replay
  path, forwards the pair through the native stereo `AHardwareBuffer` bridge,
  and attempts the existing `gpu-projected` OpenXR path. Reports include
  per-eye resolution, packet counts, payload bitrate, decoded frame rate,
  native stereo pair acceptance, and left/right timestamp deltas. Supply
  device-specific left/right camera IDs as launch extras when the default
  source selection is not the intended stereo pair. The Quest custom stereo
  profiles pin Camera2 IDs `50` and `51`, the outside front camera pair used
  for current stereo diagnostics.
- `broker-h264-stereo-live-openxr-projection-probe`: the same broker-decoded
  stereo OpenXR path with the live-bounded H.264 provider enabled. The broker
  accepts the binary stream sockets before Camera2 capture starts, drains
  MediaCodec output directly to the stream, and writes schema-3 stream headers
  with session projection metadata plus packet source timestamps. The composite
  app receives, decodes, pairs, and submits left/right hardware-buffer frames
  as packets arrive instead of waiting for the whole declared packet count.
  Existing-stream receivers prefer stream-header projection metadata over
  launch extras when both are present. Logs include source packet cadence, wire
  receive cadence, decode cadence, per-eye resolution, live pair queue drops,
  stream-header metadata readiness, and native stereo-pair acceptance. This
  profile is a correctness milestone for Quest-to-Quest-style live streaming,
  not a performance target: current runs still need frame-cadence and
  render-time optimization before the path can be treated as production
  quality.
- `broker-h264-stereo-live-openxr-projection-scale065-probe`: the same
  live-bounded broker stereo path with `rustyxr.xrRenderScale=0.65`. Use it as
  the current performance comparison profile when the `0.75` visual-quality
  profile is render-cost limited.
- `broker-h264-stereo-live-openxr-projection-fast075-probe`: the same live
  broker stereo capture/decode/pair/import path at `rustyxr.xrRenderScale=0.75`
  with square `1280x1280` broker frames and frame-order live stereo pairing, but
  with the fast public raw-projection shader variant selected. Use it as the
  renderer-parity profile before reintroducing the accepted soft-border visual
  path. This profile is visually accepted for stereo orientation/alignment;
  motion-induced stream-latency artifacts remain a separate compensation task.
- `broker-h264-stereo-live-openxr-projection-fast065-probe`: the same fast
  raw-projection path at `rustyxr.xrRenderScale=0.65` for fragment-load headroom
  checks.
- `broker-h264` existing-stream mode: set
  `rustyxr.brokerH264SourceMode=existing-stream` when a broker TCP proxy,
  laptop test source, or other tool has already exposed a `RXYRVID1` H.264
  stream on the configured port. This skips the broker Camera2 start command
  and isolates incoming-stream receive, MediaCodec decode, hardware-buffer
  handoff, and OpenXR draw. For schema-3 streams, the receiver reads projection
  metadata from the stream header and uses it before launch extras or fallback
  profiles. Existing-stream mode is the preferred one-device simulation path
  for a remote sender because it exercises the receiver side without needing a
  second headset.
- Temporal projected profiles expose receiver-side smoothing and adoption
  controls through `rustyxr.cameraTemporalProjection*` and
  `rustyxr.cameraFrameAdoption*` launch values. The current frame-adoption mode
  is `hold-until-smooth`: when enabled, the projected path can keep the last
  accepted stereo frame/projection for a bounded hold window instead of
  adopting a candidate whose projected screen-space motion exceeds the
  configured jump budget. Final projection status rows report the adoption
  mode, whether the current frame was held, candidate motion p95, held-frame
  count/duration, crossfade count, invalid-UV percentage, and edge-fill
  percentage so automated scorecards can distinguish stable smoothing from a
  stale or black render path.
- `camera-stereo-gpu-composite`: aligned Vulkan hardware-buffer baseline. It
  keeps fixed foveation off and uses `external-rgb`, so it is the profile to
  use when validating projection, border behavior, CPU-upload avoidance, import
  cache reuse, and Camera2 delivery cadence.
- `camera-stereo-gpu-composite-performance-065`: same aligned Vulkan RGB path,
  but with `rustyxr.xrRenderScale=0.65`. Use this to separate shader/fragment
  headroom from camera delivery cadence without changing projection, border, or
  color assumptions.
- `camera-stereo-gpu-composite-fast075`: same direct in-app Camera2 stereo
  projection at `rustyxr.xrRenderScale=0.75`, but selects the fast public
  raw-projection shader path. Use it for direct renderer parity checks against
  the Q2Q fast profile.
- `camera-stereo-temporal-pose-clamp-fast075`: direct in-app Camera2 stereo
  projection with the fast public shader and `pose-delta-clamp` temporal mode.
  It uses one shared angular/linear pose-delta coefficient for both eyes and
  reports the resulting target/applied/residual screen-motion metrics, making
  it the deterministic lockstep proof before visually tuning the screen-motion
  clamp.
- `camera-stereo-gpu-composite-fast065`: same direct fast raw-projection path
  at `rustyxr.xrRenderScale=0.65` for fragment-load headroom checks.
- `camera-stereo-gpu-composite-ycbcr-diagnostic`: same projection and border,
  but with shader-side `Cr/Y/Cb` BT.601 narrow-range decode. Use this only
  when `Vulkan imported camera hardware buffer` diagnostics show the external
  sampler is not already converting to RGB.
- `camera-stereo-gpu-composite-foveation-experimental`: same sampler and
  projection settings, but enables the OpenXR fixed-foveation/fragment-density
  path. It is diagnostic until logs show `fixedFoveationEnabled=true`, no
  null fragment-density image handles, no framebuffer creation failure or
  driver crash, stable OpenXR cadence, and no stale camera frames.
- `camera-stereo-gpu-composite-no-ae-target-065`: same render scale,
  projection, border, and sampler as the `0.65` performance profile, but sends
  no explicit Camera2 AE FPS target. It isolates acquisition policy from color
  and projection.
- `camera-stereo-gpu-composite-reader-max-3-065`: same render scale,
  projection, border, and sampler as the `0.65` performance profile, but uses a
  smaller separate-eye `ImageReader` pool. It isolates Java Camera2 queue depth
  from color and projection.
- `camera-stereo-gpu-composite-native-ndk-065`: same render scale, projection,
  border, and sampler as the `0.65` performance profile, but swaps Java
  Camera2 acquisition for an experimental native `ACamera*` / `AImageReader`
  hardware-buffer path with no explicit AE target and reader max images `3`.
  It logs all NDK camera-source topology it can see and is an acquisition
  probe, not a release candidate until camera-frame progression remains live.
- `camera-stereo-gpu-composite-native-single-mirror-065`: same native
  hardware-buffer path, but opens one native back-facing camera source and
  mirrors the same acquired buffer into both display eyes. Use it to isolate
  renderer/import progression from concurrent stereo acquisition. It is not a
  stereo-alignment proof because both eyes receive the same camera buffer.
- `environment-depth-diagnostics`: starts the OpenXR environment-depth path
  with native passthrough active and validates provider support, swapchain
  creation, frame acquisition, runtime capture timestamp progression, observed
  depth cadence, average acquire CPU cost, depth range metadata, and
  confidence-source reporting. It renders the acquired `VK_FORMAT_D16_UNORM`
  depth swapchain as a stereo grayscale headset diagnostic, using layer `0` for
  the left eye and layer `1` for the right eye. The sampled depth UVs use a
  `rotate0+flipY` transform so the depth diagnostic is upright against the
  accepted camera projection surface. The `XR_META_environment_depth` API
  currently exposes no confidence texture or confidence flag, so this profile
  logs that explicitly.

Do not stack the shader-side YCbCr decode on top of an external sampler that is
already presenting RGB. The hardware-buffer import log reports
`suggestedYcbcrModel`, `suggestedYcbcrRange`, component mapping, external
format, Vulkan format, import cache size, and cache miss/eviction counters so a
test run can identify which path is active.

On the April 30, 2026 Quest headset validation pass, increasing the Vulkan
hardware-buffer import cache to match the stereo producer pool removed import
evictions after warm-up. The aligned projected path held `72 FPS` at
`rustyxr.xrRenderScale=0.65`, while the `0.75` baseline remained useful for
geometry/color comparison but did not hold display cadence on that run. The
same run showed Camera2 applying a `60-60` AE range for a `72-72` request and
delivering below display cadence, so camera delivery FPS must be evaluated
separately from OpenXR submit FPS.

The follow-up acquisition probes ruled out several simple Java Camera2 knobs
as standalone fixes. No explicit AE target, `ImageReader` max images `3`, a
wider stereo-pair window, and a lower `1280x960` separate-eye size did not stop
the concurrent-separate stereo path from stalling on the tested runtime. A mono
`PRIVATE` GPU-buffer probe at `1280x960` did keep receiving live frames, which
points the next public iteration toward separating acquisition modules:
document and preserve the Java Camera2 concurrent-stereo Vulkan path, then
compare it against a lower-level/native hardware-buffer reader path before
changing projection, border, or shader color math again. The fixed-foveation
diagnostic path is still not release-ready on runtimes that enumerate null
fragment-density image handles.

The native single-camera mirror probe sharpened that split. On one tested
runtime, one physical side-camera ID delivered live native frames in mirror
mode while the other side-camera ID remained sparse even when opened alone.
The exact IDs are runtime diagnostics, not portable requirements, but the
result is useful: a live mirror run shows that Vulkan hardware-buffer import,
the OpenXR render loop, and prompt `AImage` release are capable of steady frame
progression. If the full stereo native profile still stalls, focus next on
effective source/provider policy, side-camera timestamp behavior, or session
shape rather than projection, border coordinates, or renderer cache churn.

The native-reader follow-up keeps that module split explicit. It reproduces
the lower-level ownership shape of `ACamera*` sessions plus `AImageReader`
`PRIVATE` GPU-sampled buffers and logs native source enumeration, side-frame
counts, stereo-pair publication, and the active acquisition label. On the
tested runtime, direct native side-camera sessions still showed stale
progression in one side stream, so the next comparison is source/session shape
and timestamp behavior rather than another Java Camera2 queue-depth tweak.

The optional OpenXR passthrough probe is a separate runtime-state check. The
manifest includes optional passthrough and scene declarations so runtimes can
advertise `XR_FB_passthrough` when available. Creating a passthrough
client/layer confirmed the extension exposure path, but did not fix stale
native acquisition; the always-on client mode can add runtime camera-compute
load. Use `warmup` as the lighter probe when testing whether passthrough-client
state affects camera availability.

## Live Passthrough Hotload

The activity uses `singleTask` launch mode and accepts a second intent while it
is running. `onNewIntent` refreshes the native runtime config, so render/color
parameters and native passthrough style values can be changed without
force-stopping or reinstalling the APK.

Hotload profiles in the catalog:

- `passthrough-underlay-hotload-neutral`
- `passthrough-underlay-hotload-bcs`
- `passthrough-underlay-hotload-gradient`
- `passthrough-underlay-hotload-lut-opponent`
- `passthrough-underlay-hotload-lut-flicker-10hz`
- `passthrough-underlay-hotload-lut-flicker-40hz`
- `passthrough-underlay-hotload-lut-flicker-60hz`
- `full-field-red-black-flicker-10hz`
- `full-field-red-black-flicker-40hz`
- `full-field-red-black-flicker-60hz`

Strobe warning: the LUT flicker and full-field red/black flicker profiles are
intentional high-frequency visual stimuli. They can trigger seizures or other
adverse reactions in people with photosensitive epilepsy or other
light-sensitive conditions, and may also cause migraine, nausea, dizziness,
eyestrain, anxiety, or discomfort. Do not launch them around unconsenting
bystanders. Use only with explicit informed opt-in and stop immediately if
symptoms occur. See
`docs/VISUAL_STROBE_PROFILES.md` for the public safety and frequency notes.

Send a profile to the running headset app with:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-composite-layer-apk\tools\Send-QuestCompositeHotloadProfile.ps1 -Serial <serial> -RuntimeProfile passthrough-underlay-hotload-bcs
```

Ad-hoc values can be layered on top of the profile:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-composite-layer-apk\tools\Send-QuestCompositeHotloadProfile.ps1 -Serial <serial> -RuntimeProfile passthrough-underlay-hotload-gradient -Override "rustyxr.passthroughColorPhase=0.42","rustyxr.passthroughColorAmplitude=1.0"
```

Live native passthrough style extras currently include
`rustyxr.passthroughStyleMode` (`none`, `bcs`, `mono-to-rgba`, `color-lut`),
`rustyxr.passthroughOpacity`, `rustyxr.passthroughEdgeR/G/B/A`,
`rustyxr.passthroughBrightness`, `rustyxr.passthroughContrast`,
`rustyxr.passthroughSaturation`, `rustyxr.passthroughColorPhase`, and
`rustyxr.passthroughColorAmplitude`.

The `color-lut` mode uses `XR_META_passthrough_color_lut` when the runtime
exposes it. The example builds two RGB 3D LUTs: a smooth cyclic opponent-color
palette and its half-phase inverse. `rustyxr.passthroughLutFlickerHz` is the
full A/B cycle rate in hertz, so a 40 Hz flicker requires 80 state transitions
per second. The native loop logs measured `passthrough LUT flicker stats`
including observed frame rate, observed switch rate, and skipped half-cycles.
`rustyxr.xrDisplayRefreshHz` can request a runtime display-refresh target when
the device advertises `XR_FB_display_refresh_rate`.

The `full-field-red-black-flicker-*` profiles disable passthrough and flicker
the submitted projection-layer clear color between bright red and black. This
uses `rustyxr.fullFieldFlickerHz`, also interpreted as full red/black cycles per
second, and logs `full-field flicker stats` from the OpenXR frame loop. The
10 Hz profile has integer frame timing at 120 Hz, the 40 Hz profile is
frame-quantized at 120 Hz because each half-cycle averages 1.5 frames, and the
60 Hz profile requires a state change every displayed frame.

Camera acquisition, camera resolution, OpenXR swapchain format, and render
scale remain launch-time settings.

## Autonomous Camera Profile Runs

The public workflow helpers in `tools/quest-camera-profile` launch catalog
runtime profiles, capture power/wake/VR-power snapshots, record logcat and
screenshots, reject black-camera or standby-transition windows, and compare
local screenshots with per-ROI color metrics. See
`docs/QUEST_CAMERA_PROFILE_WORKFLOW.md` for the current test plan and
validation gates.

Example:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 -Serial <serial> -RuntimeProfile camera-stereo-gpu-composite-no-ae-target-065 -CaptureHzdbScreencap
```

The generated screenshots, log bundles, manifests, validation JSON, and
comparison reports belong under ignored `artifacts/` folders and must not be
committed.

## Streaming Cost Matrix

Use the
[Quest Streaming Diagnostics Workflow](../../docs/QUEST_STREAMING_DIAGNOSTICS_WORKFLOW.md)
and [streaming diagnostics tools](../../tools/quest-streaming-diagnostics/README.md)
when comparing direct in-app Camera2 projection against broker H.264 streaming.
The current public workflow treats these as separate lanes: synthetic
compositor, direct projected Camera2, broker existing-stream receive/decode,
and broker live projected stereo, each at `rustyxr.xrRenderScale=0.75` and
`0.65` where applicable.

The important current lesson is that broker receive/decode and Java/native
hardware-buffer handoff can be isolated from the expensive path. In recent
public-example validation, both direct projected Camera2 and broker live
projected stereo were render-scale sensitive in the same way, while synthetic
compositor and broker receive/decode were not. That points the next
optimization pass at projected draw/render attribution rather than transport,
MediaCodec, `ImageReader`, or native bridge cost.

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

For environment-depth diagnostics, use the depth profile with logcat capture.
Companion validates that the OpenXR environment-depth provider started, a
swapchain was created, at least one image was acquired, runtime capture
timestamps progressed, depth range metadata was reported, and confidence state
was explicit.

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- catalog verify --path .\examples\quest-composite-layer-apk\catalog\rusty-xr-quest-composite-layer.catalog.json --app rusty-xr-quest-composite-layer --serial <serial> --stop-catalog-apps --install --launch --device-profile xr-composite-smoke-test --runtime-profile environment-depth-diagnostics --settle-ms 9000 --logcat-lines 1400 --out .\artifacts\verify
```

Use `environment-depth-mesh-overlay`, `environment-depth-particle-overlay`, or
`environment-depth-scene-particle-map` with the same command shape to render the
surface diagnostics over passthrough and check logcat for the corresponding
draw line. The scene particle map line is
`Rusty XR environment depth scene particle map draw`. These profiles submit
`openxrPassthroughProbe=underlay`; use `passthrough-only-layer-probe` to launch
the native passthrough layer with `rustyxr.projectionLayerVisible=false` when
isolating passthrough from projection-layer rendering. ADB screenshots may still
show protected compositor passthrough as black even when it is visible in the
headset.

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
- logcat reports swapchain creation and recurring `OpenXR frame` messages with
  `observedOpenXrFps`, `avgFrameMs`, `activeDisplayRefreshHz`,
  `fenceSync=slot-reuse`, import-cache counts, and descriptor-cache counts
- logcat contains `Headset camera capture session running`
- logcat contains `Rusty XR received headset camera frame`
- logcat contains `Camera2 delivery stats` and `Stereo headset camera pair`
  lines; the stereo pair line reports `softTargetNs`, `overSoftTarget`, and
  `softPairOverMax` so timestamp drift is visible without dropping the latest
  camera pair
- with `camera-diagnostic-cpu-copy`, logcat contains
  `Rusty XR uploaded diagnostic flat camera copy frame`
- with `camera-gpu-buffer-probe`, logcat contains
  `Rusty XR GPU-sampled diagnostic camera surface` or a clear GPU fallback
- with `environment-depth-diagnostics`, logcat contains
  `Rusty XR environment depth status` with `depthEnabled=true`,
  `providerRunning=true`, `swapchainCreated=true`, `acquiredFrames` greater
  than zero, nonzero `uniqueCaptureTimes`, `nearZ`, `farZ`,
  `avgAcquireCpuMs`, `observedDepthHz`, and explicit `confidenceSource` /
  `confidencePayload` fields. The visual profile also logs
  `Rusty XR environment depth visualizer draw` and displays the left/right
  runtime depth array layers as per-eye grayscale.
- with `environment-depth-mesh-overlay`, logcat contains
  `Rusty XR environment depth mesh overlay draw` with `cellMeters`,
  `discontinuityMeters`, `distanceColorMaxMeters=3`,
  `distanceColorSource=environment-depth-meters`,
  `projection=local-space-depth-surface`,
  `rasterization=world-space-generated-grid`,
  `generatedVertexCount`, `historyFramesDrawn=1`, `historyMaxAgeMs=0`,
  `dominantSurfaceGrid=true`, `screenUvGrid=false`, and
  `passthroughVisible=true`; the visualizer draw line also reports
  `depthMeshOverlay=true`, `depthMeshRasterization=world-space-generated-grid`,
  `depthMeshVertexCount`, and `depthMeshHistoryFramesDrawn`, and the OpenXR
  frame status reports the submitted passthrough underlay plus projection
  layer.
- with `environment-depth-particle-overlay`, logcat contains
  `Rusty XR environment depth particle overlay draw` with
  `projection=local-space-retained-particles`,
  `rasterization=metric-billboard-particles`, `particleCapacity`,
  `particleVertexCount`, `sampleStridePixels`, `distanceColorMaxMeters=3`,
  and `passthroughVisible=true`. The visual result is expected to show
  depth-colored particles over native passthrough, but headset-motion anchoring
  remains a manual design-validation item for the separate scene particle map.
- with `environment-depth-scene-particle-map`, logcat contains
  `Rusty XR environment depth scene particle map draw` with
  `projection=local-space-scene-particle-map`,
  `mapPolicy=spatial-hash-local-cells`, `cellMeters`, `hashProbeCount`,
  `staleFadeStartFrames`, `staleRetireFrames`,
  `invalidSamplePolicy=preserve-existing-cells`,
  `activeCorrectionPolicy=visible-free-space-ray-clear`,
  `occlusionPolicy=preserve-behind-current-depth`,
  `particleHalfSizeMeters=0.002..0.004`, `particleMask=default-disc`,
  `particleOpacity=alpha-clipped-opaque`,
  `depthPoseSource=view-space-composed`,
  `projectionYConvention=vulkan-positive-viewport-y-flipped-in-shader`,
  `particleCapacity`, `particleVertexCount`, `distanceColorMaxMeters=3`, and
  `passthroughVisible=true`. The visual result is expected to show
  depth-colored particles that remain attached to local space while new
  observations refresh or fill nearby cells and high-confidence current depth
  clears stale mapped cells from visible free space.
- with `camera-stereo-gpu-composite`, logcat must contain one
  `Rusty XR final projection status` line with `activeTier=gpu-projected`,
  `alignedProjection=true`, `stereoLayout=Separate`,
  `pairedLeftRightGpuBuffers=true`, `cpuUploadCount=0`, and
  `poseSource=platform` or `poseSource=estimated-profile`, plus
  `sourceEyeMapping`, `leftCameraTextureTransform`,
  `rightCameraTextureTransform`, `orientationCheck=true`,
  `orientationAccepted=true`, `visualInspection=accepted`, and
  `visualReleaseAccepted=true`
- with `broker-h264-stereo-openxr-projection-probe`, logcat should contain a
  `Rusty XR broker H.264 consumer probe` report with
  `stereo_pair_native_accepted_count` greater than zero and per-eye packet,
  resolution, payload, and decoded frame-rate fields. Projection success is
  only claimed when a `Rusty XR final projection status` line reports
  `activeTier=gpu-projected`, `alignedProjection=true`,
  `stereoLayout=Separate`, and `pairedLeftRightGpuBuffers=true`.
- with `broker-h264-stereo-live-openxr-projection-probe` or
  `broker-h264-stereo-live-openxr-projection-scale065-probe`, the same OpenXR
  projection checks apply, and the report should also show
  `live_stream_requested=true`, schema version `2`, non-zero per-eye source
  packet rates, non-zero wire packet rates, `live_decode_path=true`,
  `stereo_pairing_mode=live-decoded-frame-order`, and
  `stereo_live_pair_queue_drop_count`. This validates the provider drain path
  and concurrent stereo receive/decode/pair path separately from final visual
  release acceptance. Manual ADB launches must include explicit
  `cameraTextureTransformSource`, `leftCameraTextureTransformSource`, and
  `rightCameraTextureTransformSource` values matching the catalog profile;
  otherwise the renderer intentionally remains in `flat-probe` and will not
  prove the custom stereo projection geometry.
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
- with `rustyxr.brokerH264Consumer=true`, logcat contains
  `Rusty XR broker H.264 consumer probe` with `broker_command_accepted=true`,
  `stream_packet_count` greater than zero, `decode_succeeded=true`, and
  `decoded_frame_count` greater than zero. In the default `surface-texture`
  mode, it should also report `surface_target_created=true`,
  `external_texture_created=true`, `surface_release_count` greater than zero,
  and `surface_texture_update_count` greater than zero. This verifies
  cross-app broker H.264 consumption through a decoder Surface and Java
  external texture only; it does not yet make decoded frames available as a
  Vulkan/OpenXR texture.
- with `rustyxr.brokerH264DecodeOutputMode=hardware-buffer`, logcat should
  additionally contain `hardware_buffer_target_created=true`,
  `hardware_buffer_native_accepted_count` greater than zero,
  `Rusty XR received headset camera GPU buffer frame`, and
  `Rusty XR GPU-sampled diagnostic camera surface`. When broker Camera2
  metadata was available at stream start, the report should also contain
  `broker_projection_metadata_attached=true` and
  `broker_projection_metadata_ready=true`, while native logs should show
  `intrinsics=available`, `pose=available`, `poseSource=platform`, and
  `projection metadata is available`. That verifies the decoded broker H.264
  frames reached the Vulkan hardware-buffer import path, carried projection
  metadata into the native frame record, and were drawn into the app's
  submitted OpenXR projection layer.

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

Except for the explicit passthrough and environment-depth diagnostic profiles,
this example keeps native compositor passthrough, environment depth, room mesh
providers, private visual-effect layers, and downstream effect-stack code out of
ordinary camera profiles.
