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
- projection-area offset sweep values such as
  `rustyxr.cameraProjectionAreaOffsetXUv`,
  `rustyxr.cameraProjectionAreaOffsetYUv`, `rustyxr.projectionAreaOffsetXUv`,
  `rustyxr.projectionAreaOffsetYUv`,
  `debug.rustyxr.makepad.projection.area.offset.left.uv`,
  `debug.rustyxr.makepad.projection.area.offset.right.uv`, or
  `debug.rustyxr.makepad.projection.area.offset.vertical.uv`;
- independent projection-area and border opacity values such as
  `rustyxr.cameraProjectionAreaOpacity`,
  `rustyxr.cameraProjectionBorderOpacity`, `rustyxr.projectionAreaOpacity`,
  `rustyxr.projectionBorderOpacity`, and the matching Makepad
  `debug.rustyxr.makepad.projection.*.opacity` properties;
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
  -SdkPath <makepad-android-sdk-path> `
  -MakepadSourceRoot <makepad-fork-checkout>
```

The Makepad build consumes a prepared Android SDK layout for Makepad. Pass
`-MakepadSourceRoot` for evidence runs that must use the maintained fork's
Android packager; leave app dependency patching off unless an uncommitted
Makepad dependency change is explicitly under test. Host `cargo check` and
focused host tests are still useful for Makepad parser/projection code, but
plain `cargo check --target aarch64-linux-android` is not the Makepad Android
acceptance gate because it does not exercise the generated activity/packager
path. The Vulkan/HWB and GL/OES APKs consume the OpenXR loader directly.

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
  -RuntimeProfile camera-stereo-gpu-composite-full-feed-alignment `
  -Override rustyxr.cameraTargetFps=50,rustyxr.cameraPipelinePreset=raw-projection-solid-red-unorm,rustyxr.cameraProjectionEffectMode=raw-projection-solid-red,rustyxr.openxrPassthroughProbe=off,rustyxr.xrRenderScale=1,rustyxr.cameraProjectionScale=1,rustyxr.cameraProjectionAreaScaleUv=1,rustyxr.cameraProjectionAreaRadiusXUv=0.5,rustyxr.cameraProjectionAreaRadiusYUv=0.5,rustyxr.cameraProjectionAreaCornerRadiusUv=0 `
  -FreshnessFrames 6
```

Vulkan/HWB broker Camera2 -> H.264:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Catalog .\examples\quest-composite-layer-apk\catalog\rusty-xr-quest-composite-layer.catalog.json `
  -AppId rusty-xr-quest-composite-layer `
  -DeviceProfile xr-composite-comparison-level-5 `
  -RuntimeProfile broker-h264-stereo-live-openxr-projection-full-feed-alignment `
  -Override rustyxr.brokerH264CaptureMs=0,rustyxr.brokerH264MaxPackets=0,rustyxr.brokerH264FrameRateHz=50,rustyxr.cameraPipelinePreset=raw-projection-solid-red-unorm,rustyxr.cameraProjectionEffectMode=raw-projection-solid-red,rustyxr.openxrPassthroughProbe=off,rustyxr.xrRenderScale=1,rustyxr.cameraProjectionScale=1,rustyxr.cameraProjectionAreaScaleUv=1,rustyxr.cameraProjectionAreaRadiusXUv=0.5,rustyxr.cameraProjectionAreaRadiusYUv=0.5,rustyxr.cameraProjectionAreaCornerRadiusUv=0 `
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
Use `-ProcessingLayer blur -BlurRadiusPx 2.0` when comparing the same raw
projection area through the public diagnostic blur layer. The blur layer is a
small generic 9-tap sampler intended for processing-stack diagnostics; it is
not a downstream visual-effect preset or a performance-optimized separable blur.

The suite applies the same policy to every public lane:

| Renderer family | Border mapping | Blur mapping |
| --- | --- | --- |
| Vulkan/HWB | `raw-projection-solid-red-unorm` or `raw-projection-underlay-unorm` | `raw-projection-blur-solid-red-unorm` or `raw-projection-blur-underlay-unorm` plus `rustyxr.cameraBlurRadiusPx` |
| GL/OES | `rustyxr.projectionBorderPolicy=solid-red` or `passthrough-underlay` | `rustyxr.processingLayer=blur` plus `rustyxr.cameraBlurRadiusPx` |
| Makepad CPU-YUV | `debug.rustyxr.makepad.projection.border.policy=solid-red` or `passthrough-underlay` | `debug.rustyxr.makepad.processing.layer=blur` plus `debug.rustyxr.makepad.blur.radius.px` |

Use `-ProjectionAreaOffsetXUv <value>` and `-ProjectionAreaOffsetYUv <value>`
on the suite to run repeatable centering sweeps. The suite-level contract uses
screen/screenshot coordinates: positive X moves the projection area right and
positive Y moves it down. Renderer-specific sign or viewport conventions must
be normalized at the renderer/profile boundary before the app is launched.
Treat these values as projection-area placement controls; do not hide
source-crop, texture-origin, or analyzer problems behind them.
Use `-ProjectionAreaOpacity` for the projection-window fade and
`-ProjectionBorderOpacity` for the non-projection area/border fade. Opacity
changes must not move the camera projection area; rerun the solid-red
screen-space analyzer after each geometry change.

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
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- broker shell-helper start --serial <quest-serial> --rusty-xr-root . --proximity-watchdog --proximity-watchdog-until-stopped --proximity-watchdog-ensure-stay-awake --json
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
  -BrokerH264SyntheticProjectionProfile camera-matched `
  -ProjectionBorderPolicy passthrough-underlay `
  -ProcessingLayer blur `
  -BlurRadiusPx 2.0
```

The public Makepad example defaults to package
`com.example.rustyxr.makepad.alignment` and its generated launcher/XR
activities; pass the Makepad identity flags only for a differently packaged
APK.

Use `solid-red` for image-derived border checks and `passthrough-underlay` for
manual alignment with native passthrough. Leave `-ProcessingLayer raw` for
projection-only checks, and switch to `blur` only when comparing camera-sample
processing behavior across the lanes.
Use `-ProjectionAreaOffsetXUv <value>` and `-ProjectionAreaOffsetYUv <value>`
for controlled centering sweeps. Positive X and positive Y are defined in
display/screenshot coordinates: right and down. Use renderer-specific overrides
only when a lane has a documented OpenXR layer or viewport placement
convention that requires a different value; sign normalization belongs in the
renderer/profile boundary, not in the analyzer or source-content detection.

Use `-RestartBrokerBeforeBrokerModes` when multiple live broker-camera lanes
reuse the same H.264 ports in one suite. The switch restarts the broker console
before each broker lane and records `broker-restarts\` snapshots so stale
unbounded stream sockets do not masquerade as camera/projection failures.
Set `-BrokerH264SourceMode broker-synthetic` for the deterministic
broker-managed source lane. The suite forwards the same synthetic pattern,
source-geometry profile, stream ports, resolution, bitrate, requested FPS,
capture duration, and max packet settings into the Vulkan/HWB, GL/OES, and
Makepad broker modes, so their projection rows and screen-space masks can be
compared before physical camera variables are reintroduced. Use
`camera-matched` when synthetic pixels should follow the real Camera2
projection shape, and `full-frame-diagnostic` when the diagnostic raster should
exercise projection-surface coverage and orientation directly.

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

Reject mixed evidence: a `solid-red` run with feedback-colored, feathered, or
camera-sampled border pixels is a border-policy failure, not a valid projection
area measurement. Rerun with the corrected hard-mask profile before tuning
screen-space offsets.

If a lane is vertically or horizontally offset from native passthrough, record
the offset as a projection-space finding, not as an effect-stack finding. Do not
tune downstream effects until the raw lanes have been aligned or explicitly
bracketed.
