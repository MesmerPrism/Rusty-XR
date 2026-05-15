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
- log `OpenXrGlesFeasibilityStatus` and `SurfaceTextureOesIngestStatus` JSON.

It intentionally does not open Camera2 directly and it does not include effect
passes yet. Its physical camera path is broker Camera2 -> H.264 ->
SurfaceTexture/OES. If no broker-compatible streams are listening on the
selected ports, the app still renders the static OpenXR/GLES grids and logs the
decode connection failure in the SurfaceTexture/OES ingest diagnostics.

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
```

Use `rustyxr.brokerH264SourceMode=broker-synthetic` for deterministic source
parity and `rustyxr.brokerH264SourceMode=broker-camera` for physical camera
checks. Direct Camera2/OES is intentionally bracketed as a separate future
architecture rather than part of this example's current camera path.
