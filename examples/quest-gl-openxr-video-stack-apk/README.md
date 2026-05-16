# Rusty XR Quest OpenXR GLES Video Stack APK

This opt-in public example is the OpenGL ES + OpenXR feasibility lane for the
multilayer video stack plan.

Current scope:

- request `XR_KHR_opengl_es_enable`;
- create an EGL/OpenGL ES context;
- create an OpenXR session with `XrGraphicsBindingOpenGLESAndroidKHR`;
- create one non-array color swapchain per eye;
- render distinct static left/right diagnostic grids through GL FBOs;
- create per-eye `GL_TEXTURE_EXTERNAL_OES` textures, `SurfaceTexture`
  instances, and Android output `Surface` objects;
- connect to broker-compatible RXYRVID1 H.264 streams on `127.0.0.1:8879`
  and `127.0.0.1:8880`, decode them with Android `MediaCodec` into the
  per-eye output surfaces, and call `updateTexImage()` only from the native GL
  render thread;
- use launch extras to choose broker synthetic or broker Camera2 H.264 input,
  source fps, ports, resolution, bitrate, capture duration, packet limit, and
  per-eye camera IDs;
- optionally open Camera2 directly inside the APK and route per-eye preview
  output to the same `SurfaceTexture`/OES render path;
- select `rustyxr.projectionBorderPolicy=solid-red` for an opaque red
  outside-projection hard mask or `passthrough-underlay` for the same region as
  transparent alpha with source-alpha blending;
- log `OpenXrGlesFeasibilityStatus` and `SurfaceTextureOesIngestStatus` JSON.

It does not include effect passes yet. Its physical camera paths are direct
Camera2 -> SurfaceTexture/OES and broker Camera2 -> H.264 ->
SurfaceTexture/OES. If no selected source can start, the app still renders the
static OpenXR/GLES grids and logs the source failure in the SurfaceTexture/OES
ingest diagnostics.

The manifest declares optional hand-tracking launch eligibility so a hands-only
headset setup can enter the app without a controller-required launch gate. This
example does not sample hand joints or controller actions.

## Build

The build script uses Android SDK, NDK, OpenJDK, `aapt2`, `d8`, `zipalign`,
`apksigner`, and a Quest-compatible OpenXR loader library. Pass the loader
explicitly with `-OpenXrLoaderPath` or set `RUSTY_XR_OPENXR_LOADER`.

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-gl-openxr-video-stack-apk\tools\Build-QuestGlOpenXrVideoStackApk.ps1 -OpenXrLoaderPath C:\path\to\libopenxr_loader.so
```

The output APK is:

```text
examples/quest-gl-openxr-video-stack-apk/build/outputs/rusty-xr-quest-gl-openxr-video-stack-debug.apk
```

Build outputs, loader binaries, and APK bytes are ignored and must not be
committed.

## Expected Signals

- the resumed activity is
  `com.example.rustyxr.opengles/.GlesOpenXrActivity`
- logcat contains `Rusty XR initialized Android OpenXR loader with Activity context`
- logcat contains `Rusty XR OpenXR GLES requirements`
- logcat contains `Rusty XR EGL/GLES context`
- logcat contains `Rusty XR OpenXR GLES swapchain`
- logcat contains recurring `Rusty XR OpenXR GLES frame` lines with
  `iteration2Ready=true`
- logcat contains `Rusty XR OpenXR GLES feasibility status` JSON with
  schema `rusty.xr.quest.openxr_gles_feasibility.v1`
- logcat contains `Rusty XR SurfaceTexture OES ingest status` JSON with
  schema `rusty.xr.quest.surface_texture_oes_ingest.v1` and state
  `DecoderStarted` when the decode probe starts, then `TextureUpdated` after a
  decoded frame is consumed by the GL render thread
- logcat contains `Rusty XR broker H.264 OES decode report` JSON with schema
  `rusty.xr.quest.broker_h264_oes_decode_probe.v1`
- the feasibility JSON is kept logcat-parseable by listing the selected and
  known renderable swapchain formats; the adjacent swapchain log line reports
  the raw runtime format count
- the headset view shows different left/right static grid patterns

Logs are not sufficient for visual acceptance. Inspect the headset view, cast,
or screenshot before treating the lane as visually proven.

## Runtime Profiles

The catalog exposes these GL/OES camera-path profiles:

- `gles-broker-synthetic-h264-oes-projection`: requests broker synthetic H.264
  with `diagnostic-grid`, 1280x1280, 6 Mbps, requested 50 fps, ports 8879/8880,
  and `max_packets=0`.
- `gles-broker-camera-h264-oes-projection`: requests broker Camera2 H.264 from
  camera IDs 50/51, 1280x1280, 6 Mbps, requested 50 fps, ports 8879/8880, and
  `max_packets=0`.
- `gles-direct-camera2-oes-projection`: opens Camera2 camera IDs 50/51 inside
  the APK, routes preview output directly to per-eye `SurfaceTexture`/OES
  surfaces, requests 1280x1280 and 50 fps, and renders the projected GL
  camera-space path.

The GL/OES APK reads the same public broker H.264 launch extras used by the
composite-layer example where they apply:

```text
rustyxr.brokerHost
rustyxr.brokerPort
rustyxr.brokerH264SourceMode
rustyxr.brokerH264SyntheticPattern
rustyxr.brokerH264StreamPort
rustyxr.brokerH264RightStreamPort
rustyxr.brokerH264LeftCameraId
rustyxr.brokerH264RightCameraId
rustyxr.brokerH264Width
rustyxr.brokerH264Height
rustyxr.brokerH264CaptureMs
rustyxr.brokerH264MaxPackets
rustyxr.brokerH264BitrateBps
rustyxr.brokerH264FrameRateHz
rustyxr.brokerH264LiveStream
rustyxr.brokerH264CommandTimeoutMs
rustyxr.brokerH264DecodeTimeoutMs
rustyxr.projectionBorderPolicy
rustyxr.projectionAreaOffsetYUv
```

Use `rustyxr.brokerH264SourceMode=broker-synthetic` for deterministic source
parity and `rustyxr.brokerH264SourceMode=broker-camera` for physical camera
checks.

Use `rustyxr.projectionBorderPolicy=solid-red` for image segmentation and
projection-area footprint checks. Use
`rustyxr.projectionBorderPolicy=passthrough-underlay` for operator alignment
against a native passthrough underlay. Both policies use the same hard public
projection-area mask over the full submitted eye surface: solid-red fills the
outside-projection region, while passthrough-underlay writes transparent alpha
there and requests OpenXR source-alpha blending. The visible transparent
background still depends on whether the runtime/app is submitting passthrough
behind the projection layer. A black outside region in a `solid-red` run is not
valid footprint evidence; rerun after checking the launch extras and shader
logs.
Use `rustyxr.projectionAreaOffsetYUv=<value>` for controlled vertical
projection-area centering sweeps after the hard-mask border is proven.
Use `rustyxr.projectionAreaOpacity=<0..1>` to fade valid projected OES camera
pixels and `rustyxr.projectionBorderOpacity=<0..1>` to fade the solid-red
outside-projection region independently. Opacity below `1.0` requests
source-alpha composition for the projection layer; whether transparent pixels
show native passthrough or a black compositor background depends on the active
runtime composition setup.
Set `rustyxr.processingLayer=blur` and `rustyxr.cameraBlurRadiusPx=<px>` to run
the same valid projected camera samples through the public 9-tap diagnostic blur
layer. Leave `rustyxr.processingLayer=raw` for projection-only checks.

The direct Camera2/OES path reads these launch extras:

```text
rustyxr.directCamera2OesCameraId
rustyxr.directCamera2OesLeftCameraId
rustyxr.directCamera2OesRightCameraId
rustyxr.directCamera2OesWidth
rustyxr.directCamera2OesHeight
rustyxr.directCamera2OesFrameRateHz
```

Direct Camera2/OES requires camera permissions to be granted before launch.
The profile does not change headset power, proximity, or stay-awake state.
