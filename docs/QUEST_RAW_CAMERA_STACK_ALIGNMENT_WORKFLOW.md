# Quest Raw Camera Stack Alignment Workflow

This workflow keeps the public raw camera stacks comparable before downstream
apps add app-specific effects. It names each lane by the parts that matter for
alignment and performance: capture source, decode or texture handoff, render
API, and projection surface.

The goal is to align the public lanes to each other and to native passthrough
with repeatable diagnostics. Downstream apps can then consume the same lane
shape without moving app-specific visual behavior into this repository.

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
- support for solid diagnostic border versus native-passthrough underlay;
- generic counters such as packet cadence, decoded-frame cadence, import churn,
  render cadence, frame freshness, and fatal/runtime markers.

Launch/profile behavior:

- direct camera versus broker camera source;
- camera IDs, resolution, requested camera/source fps, bitrate, stream ports,
  capture duration, and max packet count;
- device performance level, refresh rate, render scale, foveation, and warmup;
- projection border policy: `solid-red` for automated segmentation or
  `passthrough-underlay` for operator alignment against native passthrough;
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
  -File .\examples\makepad-q2q-camera-shell\tools\Build-MakepadStereoAlignmentApk.ps1 `
  -SdkPath <makepad-android-sdk-path>
```

The Makepad build consumes a prepared Android SDK layout for Makepad. The
Vulkan/HWB and GL/OES APKs consume the OpenXR loader directly.

## Single-Lane Launch Recipes

Use `tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1` for the
Vulkan/HWB and GL/OES APKs. Use
`examples\makepad-q2q-camera-shell\tools\Invoke-MakepadQ2QDeviceGate.ps1` for
the Makepad APK.

Vulkan/HWB direct Camera2:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Catalog .\examples\quest-composite-layer-apk\catalog\rusty-xr-quest-composite-layer.catalog.json `
  -AppId rusty-xr-quest-composite-layer `
  -DeviceProfile xr-composite-comparison-level-5 `
  -RuntimeProfile camera-stereo-gpu-composite-fast075 `
  -Override rustyxr.cameraTargetFps=50,rustyxr.cameraPipelinePreset=raw-projection-solid-red-unorm,rustyxr.cameraProjectionEffectMode=raw-projection-solid-red,rustyxr.openxrPassthroughProbe=off `
  -FreshnessFrames 6
```

Vulkan/HWB broker Camera2 -> H.264:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Catalog .\examples\quest-composite-layer-apk\catalog\rusty-xr-quest-composite-layer.catalog.json `
  -AppId rusty-xr-quest-composite-layer `
  -DeviceProfile xr-composite-comparison-level-5 `
  -RuntimeProfile broker-h264-stereo-live-openxr-projection-fast075-probe `
  -Override rustyxr.brokerH264CaptureMs=0,rustyxr.brokerH264MaxPackets=0,rustyxr.brokerH264FrameRateHz=50,rustyxr.cameraPipelinePreset=raw-projection-solid-red-unorm,rustyxr.cameraProjectionEffectMode=raw-projection-solid-red,rustyxr.openxrPassthroughProbe=off `
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
  -File .\examples\makepad-q2q-camera-shell\tools\Invoke-MakepadQ2QDeviceGate.ps1 `
  -Serial <quest-serial> `
  -Apk <makepad-apk> `
  -PackageName <makepad-package> `
  -LauncherActivity <launcher-activity> `
  -XrActivity <xr-activity> `
  -ProjectionBorderPolicy passthrough-underlay `
  -SampleSeconds 20
```

Makepad broker Camera2 -> H.264:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\examples\makepad-q2q-camera-shell\tools\Invoke-MakepadQ2QDeviceGate.ps1 `
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

For automated segmentation, use `-ProjectionBorderPolicy solid-red`. For
operator alignment against native passthrough, use
`-ProjectionBorderPolicy passthrough-underlay`.

The suite applies the same policy to every public lane:

| Renderer family | `solid-red` mapping | `passthrough-underlay` mapping |
| --- | --- | --- |
| Vulkan/HWB | `raw-projection-solid-red-unorm` plus `raw-projection-solid-red` | `raw-projection-underlay-unorm` plus the public OpenXR passthrough underlay |
| GL/OES | `rustyxr.projectionBorderPolicy=solid-red` writes opaque red outside valid projected camera UVs | `rustyxr.projectionBorderPolicy=passthrough-underlay` writes transparent alpha outside valid projected camera UVs and requests alpha source blending |
| Makepad CPU-YUV | `debug.rustyxr.makepad.projection.border.policy=solid-red` | `debug.rustyxr.makepad.projection.border.policy=passthrough-underlay` plus native passthrough request |

Transparent GL/OES pixels show compositor background unless a runtime
passthrough underlay is active for that app. Treat that as a composition
configuration difference, not a projection-area difference.

## Full Public Suite

`tools\quest-camera-profile\Invoke-RawCameraStackAlignmentSuite.ps1` runs the
six public raw lanes with canonical lane names and writes a single run summary
under `artifacts\raw-stack-suite\`. It does not reserve shared resources or
change headset power/proximity state; reserve resources in your local
coordination system before running it.
For broker-camera Makepad runs, it defaults to headset camera IDs `50` and `51`
and launches the generated XR activity directly because the normal launcher
activity is not the reliable XR presentation gate.
The suite writes passive `state-snapshots\` before and after each mode. These
snapshots record ADB state, `dumpsys power`, `stay_on_while_plugged_in`, focus,
windows, and broker status/clock endpoints where available. Use them to
distinguish camera-readiness failures from normal timeout sleep, focus loss, or
broker state changes. Do not treat proximity settings alone as proof that the
headset cannot enter a camera-unready power state.
For long unattended verification, pass `-EnableStayAwakeGuard`. That explicitly
runs `svc power stayon true`, records the prior and resulting
`stay_on_while_plugged_in` values under `awake-guard\`, and leaves the guard in
place unless `-RestoreStayAwakeGuard` is also passed. A value such as
`mStayOn=false` or `stay_on_while_plugged_in=0` means the stay-awake guard is
off; it is not a keep-awake setting. This guard is separate from proximity
state and should not be described as a proximity override.

Example:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\tools\quest-camera-profile\Invoke-RawCameraStackAlignmentSuite.ps1 `
  -Serial <quest-serial> `
  -CompositeApk <composite-apk> `
  -GlesApk <gles-apk> `
  -MakepadApk <makepad-apk> `
  -MakepadPackageName <makepad-package> `
  -MakepadLauncherActivity <launcher-activity> `
  -MakepadXrActivity <xr-activity> `
  -Install `
  -EnableStayAwakeGuard `
  -ProjectionBorderPolicy passthrough-underlay
```

Use `solid-red` for image-derived border checks and `passthrough-underlay` for
manual alignment with native passthrough.

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
and stream status remained healthy. Resume camera validation only after the
operator or platform state has returned to camera-ready status.

Useful tools:

- `tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1` for app launch,
  profile overrides, HzDB/screenshot capture, freshness checks, and log bundles.
- `tools\quest-stereo-alignment\Analyze-StereoAlignment.py` for image-derived
  stereo checks.
- `tools\quest-stereo-alignment\Compare-HomographyStages.py` for stage-token
  and homography comparisons.
- `tools\quest-visual-stimulus\run-sync-stimulus.py` for browser-driven
  physical stimulus sessions. Treat its event log as correlation evidence, not
  proof of submitted per-eye render targets.
- `examples\makepad-q2q-camera-shell\tools\Invoke-MakepadQ2QDeviceGate.ps1` for
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

If a lane is vertically or horizontally offset from native passthrough, record
the offset as a projection-space finding, not as an effect-stack finding. Do not
tune downstream effects until the raw lanes have been aligned or explicitly
bracketed.
