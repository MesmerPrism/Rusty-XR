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
- log `OpenXrGlesFeasibilityStatus` and `SurfaceTextureOesIngestStatus` JSON.

It intentionally does not include camera access or effect passes yet. If no
broker-compatible streams are listening on the default ports, the app still
renders the static OpenXR/GLES grids and logs the decode connection failure in
the SurfaceTexture/OES ingest diagnostics.

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
