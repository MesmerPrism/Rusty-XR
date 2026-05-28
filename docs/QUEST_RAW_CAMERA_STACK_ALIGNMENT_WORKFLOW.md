# Quest Raw Camera Stack Alignment Workflow

This workflow keeps the public raw camera stacks comparable before downstream
apps add app-specific effects. It names each lane by the parts that matter for
alignment and performance: capture source, decode or texture handoff, render
API, and projection surface.

The goal is to align the public lanes to each other and to native passthrough
with repeatable diagnostics. Downstream apps can then consume the same lane
shape without moving app-specific visual behavior into this repository.

For the current ordered workflow that combines raw projection area alignment,
public diagnostic blur comparison, broker-synthetic stimuli, and the later
physical-screen Brave stimulus pass, see
[SCREEN_SPACE_AND_BLUR_ALIGNMENT_WORKFLOW.md](SCREEN_SPACE_AND_BLUR_ALIGNMENT_WORKFLOW.md).
For the renderer-parity work that adds canvas and app-side MediaProjection
support across HWB, GL/OES, and Makepad, see
[CANVAS_MEDIAPROJECTION_PARITY_IMPLEMENTATION_PLAN.md](CANVAS_MEDIAPROJECTION_PARITY_IMPLEMENTATION_PLAN.md).

## Passthrough Reference Frozen-Frame Replay

Use this protocol when solving custom composite projection against Meta native
passthrough without changing camera acquisition or renderer transport.

1. Display the visual-stimulus green center cross as the only intended
   alignment target. Record the stimulus page state, browser/display surface,
   host timestamp, and any operator caveat such as focus or partial occlusion.
2. Capture one Meta native-passthrough screenshot that shows the green cross.
   This screenshot is the reference target. Do not use it as camera input.
3. Capture one bounded Camera2 `YUV_420_888` frame per selected headset camera
   as close in time to the screenshot as the current tooling allows. Store the
   raw packed frame, a preview image, the camera id, image timestamp, source
   elapsed time, original plane strides, and capture command metadata.
4. Treat the stored camera frames as frozen source input. Replay them through
   the same source-agnostic broker/full-frame decode path used by synthetic and
   broker-camera validation, carrying explicit full-frame projection metadata.
5. Before translating screenshot-pixel deltas into display-eye UV projection
   changes, capture a display-eye UV fiducial screenshot through the same
   OpenXR submission path and analyze it with
   `Analyze-DisplayEyeUvMapping.py`. This records the mirror-capture mapping as
   evidence instead of assuming a linear eye-half-to-pixel scale.
6. Capture the custom composite replay screenshot and compare its green-cross
   centers to the native-passthrough screenshot in screenshot pixels and
   normalized eye space.

Recommended artifact layout:

```text
<suite-root>/
  reference/
    passthrough-screenshot.png
    passthrough-analysis/
    stimulus-state.json
  camera-frames/
    left/
      frame.nv21
      frame.jpg
      frame.json
    right/
      frame.nv21
      frame.jpg
      frame.json
  replay/
    left-source.h264
    right-source.h264
    projection-metadata-left.json
    projection-metadata-right.json
    custom-composite-screenshot.png
    replay-analysis/
  mirror-mapping/
    display-eye-uv-fiducial-screenshot.png
    display-eye-uv-mapping.json
    display-eye-uv-mapping-summary.md
    display-eye-uv-mapping-overlay.png
  alignment-report.json
  notes.md
```

Keep the responsibilities separate:

- Meta passthrough screenshot: reference-only screen-space target.
- Stored camera frames: frozen source input.
- Broker or replay transport: source-agnostic delivery lane.
- Custom composite projection geometry: the only tuning target.

If the replay cross does not align, name the first divergent layer before
changing code: capture timing, raw camera frame metadata, broker replay
metadata, source sampling, projection geometry, OpenXR reference-space/view
pose, compositor/screenshot coordinate convention, or analyzer evidence. Do not
hide residuals with blur, passthrough-opacity blending, renderer-local offsets,
or undocumented constants.

Before aligning either surface to native passthrough, prove the two public
custom-rendering forms are equivalent:

1. Capture `camera-stereo-gpu-composite-world-canvas-depth1-mediaprojection`.
   This draws the chosen depth-1.0 head-anchored content surface as actual quad
   geometry.
2. Capture
   `camera-stereo-gpu-composite-camera-footprint-canvas-equivalent-depth1`.
   This keeps the optimized `display-screen-homography` path and reconstructs
   that same content surface in the fullscreen shader.
3. Compare the MediaProjection and HzDB screenshots. They should agree on the
   valid camera footprint, green-cross position, and per-eye geometry up to the
   expected capture-method differences. If they do not, the first divergent
   layer is the canvas-to-collapsed mapping, not native passthrough alignment.

Both comparison profiles use `projectionDepthMeters=1.0`,
`cameraProjectionScale=1.0`, `projectionAreaScaleUv=1.0`,
`cameraPreviewFovYDegrees=60`, the delivered source-frame aspect, and a logged
`cameraRawOverlayOverscan` value. For the strict geometry handoff gate, override
`cameraRawOverlayOverscan=1.0` in both profiles so the visible canvas and the
collapsed shader use the same unpadded surface. The camera-footprint profile uses
`cameraPipelinePreset=raw-projection-unorm` with
`projectionBorderPolicy=passthrough-underlay`: it samples the full live Camera2
source but only emits color inside the reconstructed valid footprint; outside
that footprint it uses passthrough-underlay alpha rather than clamping or
stretching camera pixels over the full eye.

Once this pair agrees in MediaProjection and HzDB, use the `world-canvas` lane
as the native-passthrough alignment workbench. The canvas and collapsed custom
projection share the same surface geometry, and the collapsed path can be
translated from the solved canvas parameters later. This avoids retuning the
harder fullscreen shader while the visible surface depth/FOV relationship to
Meta passthrough is still unknown.

## Canvas To Native Passthrough Depth Sweep

Use a fixed physical target with a clean green center cross. Keep headset pose,
target pose, brightness, and target content stable across the sweep. Use the
same already-installed composite-layer APK when possible; `CompositeLayerActivity`
hotloads runtime config from a new launch intent, so depth/FOV/visibility sweeps
do not require rebuilding the APK.

For each candidate depth:

1. Launch or hotload
   `camera-stereo-gpu-composite-world-canvas-depth1-mediaprojection` with
   overrides for the current depth and strict coverage:
   `rustyxr.cameraProjectionMode=world-canvas`,
   `rustyxr.projectionDepthMeters=<depth>`,
   `rustyxr.cameraRawOverlayOverscan=1.0`,
   `rustyxr.cameraProjectionScale=1.0`,
   `rustyxr.projectionAreaScaleUv=1.0`,
   `rustyxr.projectionAreaOffsetXUv=0.0`,
   `rustyxr.projectionAreaOffsetYUv=0.0`,
   `rustyxr.projectionAreaRadiusXUv=0.5`,
   `rustyxr.projectionAreaRadiusYUv=0.5`,
   `rustyxr.projectionAreaCornerRadiusUv=0.0`,
   `rustyxr.projectionBorderOpacity=0.0`,
   `rustyxr.projectionLayerVisible=true`,
   `rustyxr.openxrPassthroughProbe=underlay`, and
   `rustyxr.mediaProjection=true`.
2. Capture one MediaProjection frame and one HzDB per-eye screenshot with the
   canvas visible over native passthrough.
3. Hotload the same run with `rustyxr.projectionLayerVisible=false`,
   `rustyxr.openxrPassthroughProbe=underlay`, and
   `rustyxr.mediaProjection=true`.
4. Capture one MediaProjection frame and one HzDB per-eye screenshot of native
   passthrough only.
5. Run `tools/quest-camera-profile/Analyze-TargetAlignmentWitness.py` on the
   passthrough-only reference and canvas-visible candidate. Use
   `--single-view` for MediaProjection images and the default per-eye split for
   HzDB screenshots.
6. Record green-cross coordinates, per-eye deltas, `projectionDepthMeters`,
   `cameraPreviewFovYDegrees`, `cameraRawOverlayOverscan`, and the logged
   `surface_to_screen` / `screen_to_surface` matrices.

Start by bracketing depth. A practical first sweep is around the suspected
closer surface, for example `0.45`, `0.50`, `0.55`, `0.60`, `0.70`, and `1.00`
meters, then refine around the lowest center-cross residual. Only after depth
is bracketed should the sweep adjust `cameraPreviewFovYDegrees` as the vertical
height knob. Treat `cameraRawOverlayOverscan` as the last resort for coverage
padding, not as a primary alignment field.

The pass condition is not just a visually pleasant composite. The canvas-visible
green cross must land on the native-passthrough green cross in MediaProjection
and remain stereo-clean in HzDB. If no depth/height pair can satisfy that, stop
and report the first named divergent layer instead of hiding the mismatch with
blur, passthrough opacity, projection-area offsets, or undocumented constants.

## Current Canvas Alignment Reference

The current public reference for the visible world-canvas aligned to native
passthrough is
`camera-stereo-gpu-composite-world-canvas-native-aligned-mediaprojection`. It
uses the direct stereo GPU Camera2 world-canvas launch context and these solved
surface values:

```text
rustyxr.projectionDepthMeters=1.434085
rustyxr.cameraPreviewFovYDegrees=69.763084
rustyxr.cameraPreviewOffsetYMeters=-0.168832
rustyxr.cameraRawOverlayOverscan=1.0
rustyxr.projectionLayerVisible=true
```

Treat the launch context as part of the reference, not just the four geometry
numbers. The clean reference must be launched through the catalog profile, or
through an equivalent launcher that supplies the complete runtime profile:
`cameraTier=gpu-projected`, `cameraStereoLayout=separate`,
`cameraSourceEyeMapping=left-right`, `cameraTargetFps=72`,
`cameraPipelinePreset=raw-projection-unorm`,
`projectionBorderPolicy=passthrough-underlay`,
`cameraColorMode=external-rgb`, `cameraAllowCpuFallback=false`, and
`cameraCpuUploadHz=0`.

A geometry-only direct `adb shell am start` is not an equivalent reference
launch. If those camera/profile keys are omitted, the app can fall back to the
slow diagnostic lane. The bad-lane signature is:

```text
requestedTier=cpu-diagnostic-flat-copy
stereoLayout=Mono
transport=cpu-yuv-rgba
uploadCadenceHz~4
requestedAeFpsRange=device-controlled
gpuImportSuccess=0
```

The clean-lane signature for this reference is:

```text
requestedTier=gpu-projected
stereo-left/right Camera2 streams
Camera2 delivery cadence around the applied AE range
OpenXR cadence back at display rate after warmup
GPU import cache active
```

The final custom projection still needs an edge-coverage policy after the
canvas is aligned. Each raw eye camera can cover a little extra on its outer
edge. In per-eye projection, those edges can appear clean because native
passthrough remains visible underneath where the other eye has no camera
content. The eventual custom path must explicitly choose between a shared
clipped footprint, per-eye footprints with passthrough underlay outside each
footprint, or a deliberate fused/combined image mode.

## Canonical Lane Names

Use these names in run folders, summaries, and issue notes. Older runtime
profile IDs remain valid aliases for compatibility.

| Canonical lane | Camera source | Frame handoff | Render path | Main use |
| --- | --- | --- | --- | --- |
| `vulkan-hwb-direct-camera2-raw` | Camera2 on the headset | `ImageReader.PRIVATE` / `HardwareBuffer` / Vulkan import | Vulkan + OpenXR composition | Public raw direct-camera baseline |
| `vulkan-hwb-broker-h264-raw` | Broker Camera2 stream | H.264 -> MediaCodec -> `ImageReader.PRIVATE` / `HardwareBuffer` / Vulkan import | Vulkan + OpenXR composition | Public raw broker-camera baseline |
| `gles-oes-direct-camera2-raw` | Camera2 on the headset | `SurfaceTexture` / `GL_TEXTURE_EXTERNAL_OES` | OpenGL ES + OpenXR composition | Public direct OES baseline |
| `gles-oes-broker-h264-raw` | Broker Camera2 stream | H.264 -> MediaCodec -> `SurfaceTexture` / `GL_TEXTURE_EXTERNAL_OES` | OpenGL ES + OpenXR composition | Public broker OES baseline |
| `makepad-cpuyuv-direct-camera2-raw` | Camera2 on the headset | CPU YUV planes -> Makepad textures | Makepad/OpenXR shell | Public framework-cost reference |
| `makepad-cpuyuv-broker-h264-raw` | Broker Camera2 stream | H.264 -> MediaCodec CPU YUV planes -> Makepad textures | Makepad/OpenXR shell | Public broker framework-cost reference |

Downstream apps should use the same lane naming when they compare app-specific
effects, but public Rusty XR only owns the raw, reusable camera and projection
diagnostics.

## What Belongs Where

APK-owned behavior:

- package/activity identity and manifest permissions;
- renderer family: Vulkan/HWB, OpenGL/OES, or Makepad CPU-YUV;
- MediaCodec output target and texture ownership;
- raw projection shaders and projection-status logging;
- support for a full submitted XR surface with a hard camera-projection sub-area
  mask, toggled between solid diagnostic border and native-passthrough underlay;
- generic counters such as packet cadence, decoded-frame cadence, import churn,
  render cadence, frame freshness, and fatal/runtime markers.

Launch/profile behavior:

- direct camera versus broker camera source;
- camera IDs, resolution, requested camera/source fps, bitrate, stream ports,
  capture duration, and max packet count;
- device performance level, refresh rate, render scale, foveation, and warmup;
- projection border policy: `solid-red` for automated segmentation or
  `passthrough-underlay` for operator alignment against native passthrough;
- projection surface depth in meters. The suite-level
  `-ProjectionDepthMeters` default is forwarded as
  `rustyxr.projectionDepthMeters` for Vulkan/HWB and GL/OES, and
  `debug.rustyxr.projection.depth.meters` for Makepad. Lane-specific depth
  overrides must be logged rather than
  hidden in renderer constants;
- preview-surface shape values. The suite-level
  `-CameraPreviewFovYDegrees`, `-CameraPreviewOffsetYMeters`, and
  `-CameraRawOverlayOverscan` values are forwarded as the public
  `rustyxr.cameraPreview*` extras for Vulkan/HWB and GL/OES, and as
  `debug.rustyxr.camera.preview.*` / `debug.rustyxr.camera.raw.overlay.overscan`
  properties for Makepad. This keeps the native-aligned HWB canvas target
  replayable by OES and Makepad custom-projection lanes without hiding shape
  differences in renderer constants;
- projection-area offset sweep values such as
  `rustyxr.projectionAreaOffsetXUv`, `rustyxr.projectionAreaOffsetYUv`,
  per-eye variants such as `rustyxr.projectionAreaLeftOffsetXUv`,
  `rustyxr.projectionAreaLeftOffsetYUv`,
  `rustyxr.projectionAreaRightOffsetXUv`, and
  `rustyxr.projectionAreaRightOffsetYUv`. The shorter
  `rustyxr.projectionArea*` launch names and matching
  `debug.rustyxr.projection.area.*` Android properties are the cross-renderer
  contract. Makepad-specific projection-area aliases are stale hygiene keys,
  not accepted runtime inputs;
- independent projection-area and border opacity values such as
  `rustyxr.projectionAreaOpacity`, `rustyxr.projectionBorderOpacity`, and the
  matching `debug.rustyxr.projection.*.opacity` Android properties;
- color-derived projection alpha controls such as
  `rustyxr.projectionAlphaMode` and `debug.rustyxr.projection.alpha.mode`,
  with shared scale/bias controls;
- synthetic pattern selection when running broker-synthetic validation;
- screenshot, HzDB, logcat, freshness, visual-stimulus, and comparison capture
  options.

Keep these separate. A lane should be rerunnable with different source profiles
without rebuilding the APK unless renderer code, permissions, or diagnostics
changed.

## Build The Public APKs

Resolve Android and OpenXR tool paths with the local machine tooling, then build
only the APKs needed for the lanes under test.

Vulkan/HWB composite APK:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\examples\quest-composite-layer-apk\tools\Build-QuestCompositeLayerApk.ps1 `
  -OpenXrLoaderPath <openxr-loader-path>
```

OpenGL ES/OES APK:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\examples\quest-gl-openxr-video-stack-apk\tools\Build-QuestGlOpenXrVideoStackApk.ps1 `
  -OpenXrLoaderPath <openxr-loader-path>
```

Makepad CPU-YUV APK:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\examples\makepad-camera-shell\tools\Build-MakepadStereoAlignmentApk.ps1 `
  -UseWindowsHost `
  -SdkPath <makepad-android-sdk-path> `
  -MakepadSourceRoot <makepad-fork-checkout> `
  -DisplaySourceEyeMapping display-left-from-left-source
```

The Makepad build consumes a prepared Android SDK layout for Makepad. Pass
`-UseWindowsHost` when the selected SDK is a Windows-host SDK; omit it only for
an intentional WSL/Linux-host SDK and packager run. Pass `-MakepadSourceRoot`
for evidence runs that must use the maintained fork's Android packager and app
Makepad dependencies; the wrapper patches the app dependency to that checkout
by default. Use `-NoPatchMakepadXrFromSource` only for an intentional
upstream/pinned-dependency comparison. Current Makepad projection evidence uses
`display-left-from-left-source`, matching the HWB and GLES/OES camera-feed
convention. Captured build commands should pass it explicitly unless
source-eye mapping is the experiment. Host
`cargo check` and focused host tests are still useful for Makepad
parser/projection code, but plain
`cargo check --target aarch64-linux-android` is not the Makepad Android
acceptance gate because it does not exercise the generated activity/packager
path. For Android-only Rust edits, direct target `cargo check` and
`cargo test --target aarch64-linux-android --no-run` may be used as optional
target-compilation probes. If the no-run test probe compiles the edited Rust
modules and fails only at final test linking because no target linker is
configured, record it as partial Android-target Rust compilation evidence, not
as passed tests and not as an APK/package failure.
If a clean WSL/Linux-host Makepad rebuild repeats a missing bundled font
asset removal failure, treat it as a packager-route failure rather than hidden
staging state and switch to the Windows-host wrapper lane unless Linux-host
packaging is the variable being tested. If `cargo_makepad` then looks for a
hardcoded `build-tools/33.0.1/aapt` path while the wrapper selected a different
Windows build-tools version, the Makepad packager source/tool is stale for this
route; update or select the maintained fork/tool rather than creating SDK
shadow directories or executable aliases. The Vulkan/HWB and GL/OES APKs
consume the OpenXR loader directly.

For canvas/custom parity captures, do not force fullscreen projection-area
controls into the suite. The solved surface values are
`projectionDepthMeters=1.434085`,
`cameraPreviewFovYDegrees=69.763084`,
`cameraPreviewOffsetYMeters=-0.168832`, and
`cameraRawOverlayOverscan=1.0`; these are separate from the projection-area
mask/footprint knobs. The GLES/OES full-frame canvas case and Makepad
full-frame canvas case should map through their screen-to-surface homographies
so the effective source-valid footprint is bounded. Treat records whose
effective source-valid rect is fullscreen as renderer-geometry failures, even
when MediaProjection and HzDB images are nonblank.

For HWB, keep the canvas reference explicit as
`rustyxr.cameraProjectionGeometryProfile=full-frame-diagnostic`, but run the
custom/collapsed profile with
`rustyxr.cameraProjectionGeometryProfile=camera-projection` and the bounded
projection-area values (`projectionAreaRadiusXUv=0.47`,
`projectionAreaRadiusYUv=0.36`, `projectionAreaCornerRadiusUv=0.08`). This
prevents the direct Camera2 service default from silently turning the custom
path back into a fullscreen diagnostic.

Makepad app-side MediaProjection is not yet a geometry witness. Current
evidence shows it captures the Makepad Android/window surface rather than the
submitted OpenXR compositor layer. Use HzDB for Makepad geometry review and
keep the MediaProjection row labeled as a capture-route diagnostic until that
route is resolved.

## Single-Lane Launch Recipes

Use `tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1` for the
Vulkan/HWB and GL/OES APKs. Use
`examples\makepad-camera-shell\tools\Invoke-MakepadCameraDeviceGate.ps1` for
the Makepad APK.

For HWB/OES projection-runtime readback, the profile runner owns the logcat
window: it clears logcat, starts a bounded `adb logcat` process before launch,
and stops it after the screenshot/device captures. The artifact is still named
`<runtime-profile>-logcat-tail.txt` for compatibility, but new runs should be
interpreted as launch-to-capture windows, not post-run logcat tails.

Vulkan/HWB direct Camera2:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Catalog .\examples\quest-composite-layer-apk\catalog\rusty-xr-quest-composite-layer.catalog.json `
  -AppId rusty-xr-quest-composite-layer `
  -DeviceProfile xr-composite-comparison-level-5 `
  -RuntimeProfile camera-stereo-gpu-composite-full-feed-control `
  -Override rustyxr.cameraTargetFps=50,rustyxr.cameraPipelinePreset=raw-projection-unorm,rustyxr.cameraProjectionEffectMode=raw-projection,rustyxr.projectionBorderPolicy=solid-red,rustyxr.openxrPassthroughProbe=off,rustyxr.xrRenderScale=1,rustyxr.cameraProjectionScale=1,rustyxr.projectionDepthMeters=1,rustyxr.projectionAreaScaleUv=1,rustyxr.projectionAreaRadiusXUv=0.5,rustyxr.projectionAreaRadiusYUv=0.5,rustyxr.projectionAreaCornerRadiusUv=0 `
  -FreshnessFrames 6
```

Vulkan/HWB broker Camera2 -> H.264:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Catalog .\examples\quest-composite-layer-apk\catalog\rusty-xr-quest-composite-layer.catalog.json `
  -AppId rusty-xr-quest-composite-layer `
  -DeviceProfile xr-composite-comparison-level-5 `
  -RuntimeProfile broker-h264-stereo-live-openxr-projection-full-feed-control `
  -Override rustyxr.brokerH264CaptureMs=0,rustyxr.brokerH264MaxPackets=0,rustyxr.brokerH264FrameRateHz=50,rustyxr.cameraPipelinePreset=raw-projection-unorm,rustyxr.cameraProjectionEffectMode=raw-projection,rustyxr.projectionBorderPolicy=solid-red,rustyxr.openxrPassthroughProbe=off,rustyxr.xrRenderScale=1,rustyxr.cameraProjectionScale=1,rustyxr.projectionDepthMeters=1,rustyxr.projectionAreaScaleUv=1,rustyxr.projectionAreaRadiusXUv=0.5,rustyxr.projectionAreaRadiusYUv=0.5,rustyxr.projectionAreaCornerRadiusUv=0 `
  -FreshnessFrames 6
```

GL/OES direct Camera2:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Catalog .\examples\quest-gl-openxr-video-stack-apk\catalog\rusty-xr-quest-gl-openxr-video-stack.catalog.json `
  -AppId rusty-xr-quest-gl-openxr-video-stack `
  -DeviceProfile gles-openxr-comparison-level-5 `
  -RuntimeProfile gles-direct-camera2-oes-projection `
  -Override rustyxr.projectionBorderPolicy=solid-red `
  -FreshnessFrames 6
```

GL/OES broker Camera2 -> H.264:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Catalog .\examples\quest-gl-openxr-video-stack-apk\catalog\rusty-xr-quest-gl-openxr-video-stack.catalog.json `
  -AppId rusty-xr-quest-gl-openxr-video-stack `
  -DeviceProfile gles-openxr-comparison-level-5 `
  -RuntimeProfile gles-broker-camera-h264-oes-projection `
  -Override rustyxr.brokerH264CaptureMs=0,rustyxr.brokerH264MaxPackets=0,rustyxr.brokerH264FrameRateHz=50,rustyxr.projectionBorderPolicy=solid-red `
  -FreshnessFrames 6
```

Makepad direct Camera2:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\examples\makepad-camera-shell\tools\Invoke-MakepadCameraDeviceGate.ps1 `
  -Serial <quest-serial> `
  -Apk <makepad-apk> `
  -PackageName <makepad-package> `
  -LauncherActivity <launcher-activity> `
  -XrActivity <xr-activity> `
  -CameraProjectionGeometryProfile full-frame-diagnostic `
  -ProjectionBorderPolicy passthrough-underlay `
  -SampleSeconds 20
```

Makepad broker Camera2 -> H.264:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\examples\makepad-camera-shell\tools\Invoke-MakepadCameraDeviceGate.ps1 `
  -Serial <quest-serial> `
  -Apk <makepad-apk> `
  -PackageName <makepad-package> `
  -LauncherActivity <launcher-activity> `
  -XrActivity <xr-activity> `
  -ProjectionBorderPolicy passthrough-underlay `
  -UseBrokerH264Camera `
  -BrokerH264CaptureMs 0 `
  -BrokerH264MaxPackets 0 `
  -BrokerH264FrameRateHz 50 `
  -SampleSeconds 20
```

For automated segmentation, use `-ProjectionBorderPolicy solid-red`. This must
render the whole non-projection-area region as hard red; a screenshot with
feedback-color or camera samples in that region is not valid alignment evidence.
For operator alignment against native passthrough, use
`-ProjectionBorderPolicy passthrough-underlay`; the same non-projection-area
region must be transparent so the compositor passthrough underlay is visible.
For opacity sweeps, keep `-ProjectionBorderPolicy solid-red`, add
`-EnableNativePassthroughUnderlay`, and tune
`-ProjectionAreaOpacity <0..1>` separately from
`-ProjectionBorderOpacity <0..1>`. That keeps the full submitted XR surface and
the red border active while fading only the projected camera window against the
native passthrough background.
For Makepad source-alpha regressions, use the focused opacity ladder before
alignment evidence:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\tools\quest-camera-profile\Invoke-MakepadOpacityLadderGate.ps1 `
  -Serial <quest-serial> `
  -Apk <makepad-apk> `
  -Lane direct `
  -SkipInstall
```

The ladder captures `0.0`, `0.5`, and `1.0` projection-area opacity with native
passthrough requested. The `0.0` row must show native passthrough only, `0.5`
must show an intentional native/custom blend, and `1.0` must show the custom
projection. Broker rows are not valid until their decoded texture counters are
nonzero. For broker rows, add `-Lane broker -RestartBrokerBeforeBrokerRows` so
stale live-stream sockets are cleared before each opacity row.

For manual blending against native passthrough after geometry is stable, use
`-ProjectionAlphaMode fixed|red|green|blue|luma|inverse-red|inverse-green|inverse-blue|inverse-luma`
with optional `-ProjectionAlphaScale` and `-ProjectionAlphaBias`. The effective
valid-camera alpha is `ProjectionAreaOpacity * clamp(mask * scale + bias)`.
For example, `-ProjectionAlphaMode green` keeps green stimulus regions opaque
while dark or non-green regions reveal the native passthrough underlay. Use this
only as a blend/mask witness; solid-red raw gates remain the coordinate
authority.

For GL/OES color matching, keep the first pass scalar and neutral-constrained.
The current OES path first names the external texture color-transfer layer:
`rustyxr.oesSourceColorTransfer=srgb-to-linear` converts
`GL_TEXTURE_EXTERNAL_OES` camera/video RGB before the generic camera color
matrix, offset, contrast, brightness, and saturation controls. Use native
passthrough at `-ProjectionAreaOpacity 0` as the reference, then capture the
default OES projection and one candidate with the same geometry and
`-ProjectionAreaOpacity 1`. Sample both color bars and neutral regions of the
target; do not fit a full matrix from saturated bars alone. If you need to
disable the OES transfer for an A/B check, set
`rustyxr.oesSourceColorTransfer=identity` and record that as the tested source
color convention. A scalar fallback should start from:

```powershell
-GlesCameraColorMatrix '1;0;0;0;1;0;0;0;1' `
-GlesCameraColorOffset '0;0;0' `
-GlesCameraColorContrast 1.0 `
-GlesCameraColorBrightness 0.0 `
-GlesCameraColorSaturation 1.0
```

Treat these values as stimulus and lighting dependent until a broader color
profile is captured. Peripheral native-passthrough curvature remains compositor
behavior; use the green center cross for geometry and the color bars plus
neutral target regions for color.
Use `-ProcessingLayer blur -BlurRadiusPx 2.0` when comparing the same raw
projection area through the public diagnostic blur layer. The blur layer is a
generic 25-tap sampler intended for processing-stack diagnostics; it is not a
downstream visual-effect preset or a performance-optimized separable blur.
Run `Build-SyntheticBlurComparison.py` on the raw and blur
`screen-space-report.json` files to verify that geometry stayed stable and that
the synthetic `diagnostic-grid` lost high-frequency edge energy under blur.

The suite applies the same policy to every public lane:

| Renderer family | Border mapping | Blur mapping |
| --- | --- | --- |
| Vulkan/HWB | `rustyxr.projectionBorderPolicy=solid-red` or `passthrough-underlay` with `rustyxr.cameraPipelinePreset=raw-projection-unorm` | `rustyxr.processingLayer=blur` plus `rustyxr.cameraBlurRadiusPx` |
| GL/OES | `rustyxr.projectionBorderPolicy=solid-red` or `passthrough-underlay` | `rustyxr.processingLayer=blur` plus `rustyxr.cameraBlurRadiusPx` |
| Makepad CPU-YUV | `debug.rustyxr.projection.border.policy=solid-red` or `passthrough-underlay` | `debug.rustyxr.processing.layer=blur` plus `debug.rustyxr.camera.blur.radius.px`; legacy `debug.rustyxr.makepad.*` aliases are compatibility-only |

Use `-ProjectionAreaOffsetXUv <value>` and `-ProjectionAreaOffsetYUv <value>`
on the suite to run repeatable centering sweeps. The suite-level contract uses
screen/screenshot coordinates: positive X moves the projection area right and
positive Y moves it down. Renderer-specific sign or viewport conventions must
be normalized at the renderer/profile boundary before the app is launched.
Prefer those suite-level controls for cross-lane work. The Makepad launcher
wrapper writes the same current `debug.rustyxr.projection.area.*` properties as
the other Android-property paths; stale Makepad-specific projection-area
aliases are hygiene-only cleanup keys.
Treat these values as projection-area placement controls; do not hide
source-crop, texture-origin, or analyzer problems behind them.
Use `-ProjectionAreaOpacity` for the projection-window fade and
`-ProjectionBorderOpacity` for the non-projection area/border fade. Opacity
changes must not move the camera projection area; rerun the solid-red
screen-space analyzer after each geometry change.
Use `-ProjectionAlphaMode` only after the hard-mask geometry contract is stable.
Color-derived alpha is a projection-layer blend mask, not a source metadata,
texture-origin, or OpenXR geometry correction.
GL/OES source-alpha composition expects premultiplied RGB. Opacity-zero
underlay witnesses are invalid if camera RGB remains visible, and that failure
belongs to the texture/upload convention before any projection-area tuning.

Transparent GL/OES pixels show compositor background unless a runtime
passthrough underlay is active for that app. Treat that as a composition
configuration difference, not a projection-area difference.
For Vulkan/HWB, `openxrPassthroughProbe=underlay` must be visible to the native
runtime before the OpenXR instance is created, because `XR_FB_passthrough` is an
instance extension. A valid HWB underlay witness logs an OpenXR passthrough
extension plan with `available=true`, `enabled=true`, and
`requestedAtStartup=true`, then logs `passthrough started mode=Underlay`. If the
custom projection renders but an opacity-zero native-underlay witness is
black/HUD-only, first check this startup-extension path before changing
projection geometry.
When comparing a physical target against native passthrough, use a clean
center-cross feature as the primary alignment target. The native passthrough
compositor can apply additional peripheral warp, so curved monitor borders or
edge residuals are expected witness semantics unless the center target is also
misaligned.

## Full Public Suite

`tools\quest-camera-profile\Invoke-RawCameraStackAlignmentSuite.ps1` runs the
six public raw lanes with canonical lane names and writes a single run summary
under `artifacts\raw-stack-suite\`. It does not reserve shared resources or
change headset power/proximity state; reserve resources in your local
coordination system before running it.
For broker-camera Makepad runs, it defaults to headset camera IDs `50` and `51`
and launches the generated XR activity directly because the normal launcher
activity is not the reliable XR presentation gate.
Use `-BrokerH264ProjectionGeometryProfile` for broker-camera or
broker-synthetic transport checks where the renderer should consume one
source-agnostic content geometry contract. Use
`-CameraProjectionGeometryProfile` for direct HWB, GL/OES, and Makepad Camera2
checks.
The active direct Camera2 diagnostic profiles are `full-frame-diagnostic`
(full delivered camera frame mapped onto the solved projection area) and
`camera-projection` (per-eye screen-to-camera homography through the solved
surface). Other direct Camera2 geometry-profile values are rejected or reported
as unsupported.
The
legacy
`-BrokerH264SyntheticProjectionProfile` parameter remains the synthetic-source
profile alias; it should not be the only geometry selector for actual camera
data fed through the broker.
The suite writes passive `state-snapshots\` before and after each mode. These
snapshots record ADB state, `dumpsys power`, `stay_on_while_plugged_in`, focus,
windows, VR power-manager state, and broker status/clock endpoints where
available. A mode preflight now requires ADB `device`, Android wakefulness
`Awake`, and a mounted VR power state before the renderer is launched; if that
readiness check fails, the mode is recorded as failed instead of collecting
camera evidence against a stale or unreachable headset. The suite summary
includes a state-transition audit when a mode changes wakefulness, VR power
state, virtual proximity state, or ADB state. Use that audit to distinguish
camera-readiness failures from normal timeout sleep, focus loss, broker state
changes, or a headset transition into screen-awake/camera-unready state. Do not
treat proximity settings alone as proof that the headset cannot enter a
camera-unready power state.
For long unattended verification, pass `-EnableStayAwakeGuard`. That explicitly
runs `svc power stayon true`, records the prior and resulting
`stay_on_while_plugged_in` values under `awake-guard\`, and leaves the guard in
place unless `-RestoreStayAwakeGuard` is also passed. A value such as
`mStayOn=false` or `stay_on_while_plugged_in=0` means the stay-awake guard is
off; it is not a keep-awake setting. This guard is separate from proximity
state and should not be described as a proximity override.
For autonomous camera sessions where off-face proximity, stay-awake, and wake
state all need active enforcement until an operator stops it, start the broker
shell-helper watchdog before the matrix:

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- broker shell-helper start --serial <quest-serial> --rusty-xr-root . --no-broker-report --skip-status --proximity-watchdog --proximity-watchdog-until-stopped --proximity-watchdog-ensure-stay-awake --json
```

That is an explicit active guard, separate from passive state snapshots. Stop
the helper before restoring normal proximity or treating a later run as
unmanaged headset evidence. When a shared local coordination board is in use,
record this guard as non-exclusive keep-awake/vitals state; it should not block
another operator from reserving the headset and ADB for an intentional install,
launch, screenshot, logcat, or validation action.

Example:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\tools\quest-camera-profile\Invoke-RawCameraStackAlignmentSuite.ps1 `
  -Serial <quest-serial> `
  -CompositeApk <composite-apk> `
  -GlesApk <gles-apk> `
  -MakepadApk <makepad-apk> `
  -Install `
  -EnableStayAwakeGuard `
  -RestartBrokerBeforeBrokerModes `
  -BrokerH264SourceMode broker-synthetic `
  -BrokerH264SyntheticPattern diagnostic-grid `
  -BrokerH264ProjectionGeometryProfile full-frame-diagnostic `
  -ProjectionBorderPolicy passthrough-underlay `
  -ProcessingLayer raw
```

The public Makepad example defaults to package
`io.github.mesmerprism.rustyxr.makepad.camera` and its generated launcher/XR
activities; pass the Makepad identity flags only for a differently packaged
APK.

Use `solid-red` for image-derived border checks and `passthrough-underlay` for
manual alignment with native passthrough. Leave `-ProcessingLayer raw` for
projection-only checks, and switch to `blur` only when comparing camera-sample
processing behavior across the lanes.
To verify the current HWB canvas-aligned target across non-HWB custom lanes,
pass the solved surface values explicitly:

```powershell
-ProjectionDepthMeters 1.434085 `
-CameraPreviewFovYDegrees 69.763084 `
-CameraPreviewOffsetYMeters -0.168832 `
-CameraRawOverlayOverscan 1.0
```

Reference validation on 2026-05-21 used freshly built GL/OES and Makepad APKs.
The Makepad APK was built through the Windows-host wrapper lane with
`-UseWindowsHost`, an explicit standalone Windows SDK path, and the maintained
Makepad fork passed through `-MakepadSourceRoot`; the build log identified a
source-built `cargo_makepad` from that fork, app Makepad dependencies patched
to the same checkout, and SDK build-tools `36.0.0`.
The validation suite then installed those APKs and ran only the non-HWB custom
projection modes:

```powershell
-Mode gles-oes-direct-camera2-raw,gles-oes-broker-h264-raw,makepad-cpuyuv-direct-camera2-raw,makepad-cpuyuv-broker-h264-raw `
-RestartBrokerBeforeBrokerModes `
-BrokerH264SourceMode broker-synthetic `
-BrokerH264SyntheticPattern diagnostic-grid `
-BrokerH264ProjectionGeometryProfile full-frame-diagnostic `
-ProjectionBorderPolicy solid-red `
-ProcessingLayer raw
```

The strict diagnostic-mask analyzer passed the two broker-synthetic custom
projection lanes with zero cross-lane footprint deltas between GL/OES and
Makepad. The direct live-Camera2 lanes completed and logged ready projection
coordinate contracts, but the strict analyzer correctly marked their screenshots
ambiguous because live camera pixels do not contain the diagnostic mask. Running
the analyzer with `--allow-visible-fallback` produced ready records for all four
modes, while the all-lane parity check remained evidence-only because it mixes
live-camera visible envelopes with broker-synthetic diagnostic footprints. Treat
broker-synthetic parity and direct-Camera2 visible-envelope evidence as separate
checks unless a future direct-camera diagnostic mask is added.

Use `-ProjectionAreaOffsetXUv <value>` and `-ProjectionAreaOffsetYUv <value>`
for controlled centering sweeps. Positive X and positive Y are defined in
display/screenshot coordinates: right and down. Use renderer-specific overrides
only when a lane has a documented OpenXR layer or viewport placement
convention that requires a different value; sign normalization belongs in the
renderer/profile boundary, not in the analyzer or source-content detection.
For eye-specific alignment, use renderer-specific left/right suite parameters
such as `-VulkanProjectionAreaLeftOffsetXUv`,
`-VulkanProjectionAreaRightOffsetXUv`,
`-VulkanProjectionAreaLeftOffsetYUv`,
`-VulkanProjectionAreaRightOffsetYUv`,
`-GlesProjectionAreaLeftOffsetXUv`,
`-GlesProjectionAreaRightOffsetXUv`,
`-GlesProjectionAreaLeftOffsetYUv`, and
`-GlesProjectionAreaRightOffsetYUv`. The common offset remains the fallback
for an eye whose per-eye value is not supplied.

Use `-RestartBrokerBeforeBrokerModes` when multiple live broker-camera lanes
reuse the same H.264 ports in one suite. The switch restarts the broker console
before each broker lane and records `broker-restarts\` snapshots so stale
unbounded stream sockets do not masquerade as camera/projection failures.
Set `-BrokerH264SourceMode broker-synthetic` for the deterministic
broker-managed source lane. The suite forwards the same synthetic pattern,
source-geometry profile, stream ports, resolution, bitrate, requested FPS,
capture duration, and max packet settings into the Vulkan/HWB, GL/OES, and
Makepad broker modes, so their projection rows and screen-space masks can be
compared before source-acquisition variables are reintroduced. Use
`-BrokerH264ProjectionGeometryProfile full-frame-diagnostic` when either
synthetic pixels or real camera pixels should exercise projection-surface
coverage and orientation directly through the same broker transport contract.
Use `camera-matched` only for synthetic negative controls that intentionally
replay the older physical Camera2 projection footprint.

After a solid-red suite run, measure each lane in the captured screenshot
coordinate system:

```powershell
python .\tools\quest-camera-profile\Analyze-RawStackScreenSpace.py `
  .\artifacts\raw-stack-suite\<session-id>
```

The report gives per-eye bounding boxes, center offsets, and row spans in
screen pixels. In `solid-red` runs, the analyzer requires the red
projection-area mask; if it is missing, the lane is marked ambiguous instead of
falling back to visible-content segmentation. Use the vertical offset values to
compare each lane against the eye-half center before changing projection knobs.
When lane logcat is available, the report also lists source/mode fields and the
projection-stage rows found in that lane. Use those rows as the input to
`tools\quest-stereo-alignment\Compare-HomographyStages.py` when the footprint
diff suggests a coordinate-chain mismatch.
Projection-coordinate contract rows should also carry the renderer-authored
projection-area target fields:
`projectionAreaTargetCoordinateSpace=display-eye-screen-uv`,
`projectionAreaOffsetConvention=positive-x-right-positive-y-down`,
`leftProjectionAreaScreenUvRect`, `rightProjectionAreaScreenUvRect`,
`leftProjectionAreaCenterUv`, and `rightProjectionAreaCenterUv`. Vulkan/HWB
rows also log the response model fields
`projectionAreaOffsetResponseModel`, `leftProjectionAreaOffsetResponseUv`, and
`rightProjectionAreaOffsetResponseUv`; use them to compare observed screenshot
motion against the named projection-area response before changing projection
geometry. If the display-eye UV response and screenshot-pixel motion disagree,
keep the disagreement assigned to the screenshot/compositor coordinate
convention until an eye-UV-to-screenshot mapping is logged. Missing target rect
or response fields make the row evidence-only for projection-area placement,
not a stable coordinate authority.

For that mapping, launch the Vulkan/HWB composite APK with the
`display-eye-uv-fiducial-unorm` pipeline preset and capture the mirror
screenshot plus logcat tail. Then run:

```powershell
python .\tools\quest-camera-profile\Analyze-DisplayEyeUvMapping.py `
  .\artifacts\<run>\display-eye-uv-fiducial-screenshot.png `
  --log .\artifacts\<run>\display-eye-uv-fiducial-logcat.txt `
  --out-dir .\artifacts\<run>\display-eye-uv-analysis
```

The analyzer writes the global affine fit, a near-center finite-difference
mapping around the green center fiducial, centerline nonlinearity/asymmetry
evidence, and an overlay. Use the near-center mapping for first-order
green-cross alignment, then pass the mapping JSON to
`Analyze-TargetAlignmentWitness.py --display-eye-uv-mapping <mapping.json>` so
the target analyzer reports local display-eye UV deltas alongside raw
screenshot-pixel deltas. Treat large affine residuals or centerline asymmetry as
mirror/compositor evidence, not permission to introduce renderer-local offsets.
If the projection-area offset response still disagrees after the local mapping,
the next divergent layer is projection-area content mapping or projection
geometry, not the screenshot coordinate convention by itself.

When that happens, rerun the same fiducial analyzer on
`projection-content-uv-fiducial-unorm`. That preset renders markers in
`full_frame_content_uv`, the post-offset content basis used by frozen-frame
replay before source sampling. Comparing the submitted-eye fiducial against this
post-offset content fiducial separates mirror/screenshot mapping from
projection-area content response.

## Diagnostic Loop

Run the comparison in this order:

1. Confirm camera readiness. Display-on and launchability are not enough; verify
   camera frames or camera-readiness markers before judging a lane.
2. Run broker-synthetic `diagnostic-grid` and `motion-bar` when isolating
   projection, color/luma, and temporal adoption without physical camera noise.
3. Run direct physical camera lanes.
4. Run broker physical camera lanes.
5. Compare solid-border captures for footprint and row spans.
6. Compare passthrough-underlay launches while wearing the headset for native
   passthrough alignment.
7. Only then add downstream app-specific visual effects.

If a headset sleeps during a run, stop the mode sequence and preserve the next
state snapshot. The important fields are wakefulness, last sleep reason, last
sleep/wake times, stay-on setting, foreground/focus, and whether broker clock
and stream status remained healthy. Also inspect VR power-manager events around
`setActivityMonitorState: Idle`, `onDeviceIdle`, `mountWakelock: false`,
`releasePowerStateLock: MOUNTED`, `setVirtualProxState(DISABLED)`, and
`Calling goToSleep()`. Resume camera validation only after the operator or
platform state has returned to camera-ready status and live camera-frame
progression has been re-proven.

Useful tools:

- `tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1` for app launch,
  profile overrides, HzDB/screenshot capture, freshness checks, and log bundles.
- `tools\quest-camera-profile\Analyze-DisplayEyeUvMapping.py` for
  display-eye-UV-to-mirror-screenshot mapping from the fiducial preset.
- `tools\quest-stereo-alignment\Analyze-StereoAlignment.py` for image-derived
  stereo checks.
- `tools\quest-stereo-alignment\Compare-HomographyStages.py` for stage-token
  and homography comparisons.
- `tools\quest-visual-stimulus\run-sync-stimulus.py` for browser-driven
  physical stimulus sessions. Treat its event log as correlation evidence, not
  proof of submitted per-eye render targets.
- `examples\makepad-camera-shell\tools\Invoke-MakepadCameraDeviceGate.ps1` for
  Makepad direct and broker camera gates.

Record these fields for each lane:

- foreground/focus before and after launch;
- source type, camera IDs, resolution, bitrate, requested fps, observed fps;
- packet, access-unit, decoded-frame, texture-update, and render cadence;
- repeated/stale-frame evidence;
- hardware-buffer import churn or SurfaceTexture skipped-frame counts where
  applicable;
- projection shader/path and border policy;
- fatal, GPU fault, and AndroidRuntime markers.

## Alignment Notes

The full render surface and the camera projection area are separate concepts.
For diagnostics, keep the app rendering to the full XR surface and control the
outside-projection region with the border policy:

- `solid-red`: best for automated footprint and row-span extraction;
- `passthrough-underlay`: best for manual alignment to native passthrough;
- transparent or underlay borders should not change the actual projection
  coordinates.

In this workflow, `projectionBorderPolicy` is the raw projection exterior fill
policy: it controls the `surface_minus_feed` region inside the submitted
projection surface when no effect layer consumes that exterior. That raw
exterior fill is part of the projection-surface witness, even when the current
fill is solid red. It is separate from diagnostic guide borders, fiducials,
source-validity overlays, source-sampling witnesses, and effect-run debug
regions. A `solid-red` raw parity run should have no cyan/yellow guide
overlays; if those guides are visible, treat the run as diagnostic-overlay
evidence rather than hard-mask footprint evidence.

For `processingLayer=peripheral-stretch`, do not use the solid-red exterior as
a raw footprint measurement. In target-local metadata runs, the coherent core
is the resolved target footprint minus any configured inner transition band.
The transition band is still inside the target footprint and belongs to the
processing layer, not to `projectionBorderPolicy`. The effect exterior is the
visible render surface outside that target footprint. Analyzer summaries should
classify those runs as `effect-run`, with `borderRegionSemantics` set to
`visible-render-surface-minus-target-footprint` and transition-region fields
read from renderer logs rather than inferred from screenshot color alone.

Reject mixed evidence: a `solid-red` run with feedback-colored, feathered, or
camera-sampled border pixels is a border-policy failure, not a valid projection
area measurement. Rerun with the corrected hard-mask profile before tuning
screen-space offsets.

If a lane is vertically or horizontally offset from native passthrough, record
the offset as a projection-space finding, not as an effect-stack finding. Do not
tune downstream effects until the raw lanes have been aligned or explicitly
bracketed.
