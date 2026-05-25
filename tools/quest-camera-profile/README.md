# Quest Camera Profile Workflow Tools

These scripts provide a public, reusable workflow for comparing Quest camera
runtime profiles without committing screenshots, APKs, logcat dumps, or private
reference details.

The tools write into `artifacts/quest-camera-profile-runs/`, which is ignored
by the repository.

For the public raw-stack comparison across Vulkan/HWB, OpenGL/OES, and Makepad
CPU-YUV lanes, use
`tools\quest-camera-profile\Invoke-RawCameraStackAlignmentSuite.ps1` and the
lane definitions in
`docs\QUEST_RAW_CAMERA_STACK_ALIGNMENT_WORKFLOW.md`. The suite uses canonical
architecture names such as `vulkan-hwb-broker-h264-raw` while preserving the
older catalog runtime profile IDs as implementation aliases.

For the current screen-space plus blur ordering, start from
`docs\SCREEN_SPACE_AND_BLUR_ALIGNMENT_WORKFLOW.md`: broker-synthetic
`diagnostic-grid` and `motion-bar` first, raw projection area before blur, and
physical Brave stimulus only after the deterministic packets are coherent.

For suite runs with solid-red projection exterior fill, analyze the resulting
screenshot coordinate coverage with:

```powershell
python .\tools\quest-camera-profile\Analyze-RawStackScreenSpace.py `
  .\artifacts\raw-stack-suite\<session-id>
```

When a single suite runs multiple live broker-camera modes on the same H.264
ports, pass `-RestartBrokerBeforeBrokerModes` to
`Invoke-RawCameraStackAlignmentSuite.ps1`. It restarts the broker console before
each broker lane and writes `broker-restarts\` snapshots, which keeps stale
unbounded stream sockets from looking like black-frame receiver failures.
For controlled profile launches, keep `-ProjectionPropertyHygiene fail` on
`Invoke-QuestCameraProfileRun.ps1` unless the wrapper owns the full run state.
This fails before launch when persistent `debug.rustyxr.*` projection properties
would compete with launch extras. The raw-stack and canvas/custom parity suites
own the full state and use `clear`, writing `projection-property-hygiene.json`
under each case root before launching.
Projection-runtime readback validation compares those launch extras, or
Makepad `setprop`/`getprop` readback files, against the resolved
`RUSTY_XR_PROJECTION_RUNTIME_MANIFEST` fields in logcat. The profile runner
accepts `-ProjectionRuntimeReadback skip|warn|required`; the canvas/custom
suite upgrades the default `warn` mode to `required` when
resolved projection runtime consumption is enabled, which is the suite default.
The HWB/OES profile runner clears logcat before launch, starts a bounded
`adb logcat` process before `am start`, and stops it after screen capture. The
artifact keeps the historical `<runtime-profile>-logcat-tail.txt` filename for
tool compatibility, but it is a launch-to-capture window rather than a
post-run `logcat -d -t` tail.
The validator treats backend scope as part of the contract: if a log bundle
contains multiple backends for the same expected key, the caller must pass an
expected backend instead of letting one renderer's manifest satisfy another
renderer's launch/readback values.
For deterministic projection work, pass
`-BrokerH264SourceMode broker-synthetic -BrokerH264SyntheticPattern diagnostic-grid`.
Add `-BrokerH264SyntheticProjectionProfile camera-matched` when the synthetic
stream should use the same Camera2 projection geometry as the direct camera
path, or `-BrokerH264SyntheticProjectionProfile full-frame-diagnostic` when the
synthetic raster should act as projection-surface diagnostic content.
For source-agnostic transport checks, prefer
`-BrokerH264ProjectionGeometryProfile full-frame-diagnostic`; it applies the
same projection-geometry metadata style to broker-camera streams and keeps the
older synthetic profile switch as an alias only for synthetic source
generation. Use `-CameraProjectionGeometryProfile full-frame-diagnostic` for
direct Camera2 lanes; this is the active camera-as-content path.
The suite forwards the same broker source parameters into the Vulkan/HWB,
GL/OES, and Makepad broker lanes so the screen-space analyzer can report both
the hard solid-red footprint and the projection-stage rows found in each lane's
logcat. For camera-matched projection runs, the analyzer prefers
renderer-authored `leftExpectedSourceValidScreenUvRect` and
`rightExpectedSourceValidScreenUvRect` fields when present; its own
`screen_to_camera` footprint remains a model comparison, not the coordinate
source of truth. For `full-frame-diagnostic`, the analyzer uses the
renderer-authored full-frame intent to measure the visible stimulus envelope,
so backend color/decoder differences do not turn a disconnected top diagnostic
band into a false vertical-placement failure. If a full-frame lane has no
meaningful red exterior area because the renderer-authored source-valid
footprint fills the projection area, the lane is blocked for strict mask
segmentation instead of falling back to guide colors.

Projection-coordinate contracts include a `source_sampling` record, and the
analyzer also emits dedicated `source-sampling-contracts.jsonl` plus
`source-sampling-contract-summary.json`. Use those contracts to keep
architecture differences explicit: HWB reports hardware-buffer sampler
transform flags, GL/OES reports the Android `SurfaceTexture` transform or
identity decision, and Makepad reports the CPU-YUV shader `source_sample_uv`
convention. The analyzer also records dominant green horizontal feature rows
from screenshots as evidence. If broker-synthetic rows agree but live Camera2
rows diverge, assign the first owner to source sampling or texture/upload
metadata before touching projection-area offsets.
GL/OES also logs `sourceColorTransform` and `swapchainColorFormat`; keep those
as color/texture-upload evidence, separate from coordinate fields.

For environment-depth particle or mesh profiles, build the world-space contract
artifact from logcat markers with:

```powershell
python .\tools\quest-camera-profile\Build-DepthWorldSpaceContract.py `
  .\artifacts\quest-camera-profile-runs\<run>
```

When comparing the depth path to live Camera2 or a passthrough-underlay witness,
pass the relevant `projection-coordinate-contracts.jsonl` files with
`--camera-contracts` and `--passthrough-contracts`. The depth artifact records
the intended chain
`depth UV -> depth view ray -> app reference-space point -> render-eye screen`,
while screenshot or passthrough evidence remains a witness rather than the
coordinate source of truth.

After projection and depth artifacts exist, build the joined comparison matrix
from the JSONL records:

```powershell
python .\tools\quest-camera-profile\Build-ProjectionDepthComparison.py `
  --camera-contracts .\artifacts\<live-direct>\projection-coordinate-contracts.jsonl `
  --camera-contracts .\artifacts\<live-broker>\projection-coordinate-contracts.jsonl `
  --passthrough-contracts .\artifacts\<passthrough>\projection-coordinate-contracts.jsonl `
  --depth-contracts .\artifacts\<depth>\depth-world-space-contracts.jsonl `
  --out-dir .\artifacts\<joined-comparison>
```

The joined artifact writes JSONL, JSON, and Markdown summaries by lane and eye.
It assigns every gap to the ledger owners: source metadata, texture/upload
convention, projection-area mapping, OpenXR reference-space geometry, backend
viewport convention, or analyzer evidence.

Projection rows are joinable with the depth/world-space lane only when they
carry the OpenXR contract fields emitted by the renderers:
`referenceSpace=app-reference-space`, `openxrReferenceSpace=<runtime-label>`,
`displayTimeSource=predicted-display-time`, `predictedDisplayTimeNs`,
`viewPoseFovSource`, and per-eye `left/rightRenderFovTangents`,
`left/rightRenderPosition`, and `left/rightRenderOrientation`. The depth
world-space contract records the matching `displayTimeNs` used for environment
depth acquire. Missing fields remain owned by OpenXR reference-space geometry,
not by the analyzer or blur pipeline.

For a physical monitor or printed alignment target, use the target witness
analyzer on an opacity-zero native-passthrough reference and an opaque custom
projection candidate:

```powershell
python .\tools\quest-camera-profile\Analyze-TargetAlignmentWitness.py `
  --reference .\artifacts\<native-passthrough>\screencap.png `
  --candidate .\artifacts\<custom-camera>\screencap.png `
  --out-dir .\artifacts\<target-witness>
```

The target analyzer detects high-saturation target features by eye, estimates
candidate-to-reference translation, and writes overlay PNGs plus JSON/Markdown
summaries. A zero-shift, high-correlation result is an alignment witness, while
a missing/black opacity-zero reference is owned by projection-area mapping
because the underlay witness did not expose the physical target. Large
single-marker deltas can remain analyzer evidence when the correlation result
is stable, especially when the physical screen shows duplicated target content.
For native passthrough comparisons, the green center cross is the primary
alignment signal. Full-frame border/correlation evidence is secondary because
the native compositor can warp the screen perimeter in ways that a raw custom
Camera2 projection is not expected to reproduce.
For large offset-response probes, pass `--skip-translation` to report direct
cross and feature coordinates without the expensive full-feature correlation
search; use that mode when the green-cross delta and logged projection fields
are the evidence being compared.

When a native-passthrough comparison needs a display-eye UV to mirror
screenshot mapping, capture the Vulkan/HWB composite `display-eye-uv-fiducial`
diagnostic and analyze it separately from camera content:

```powershell
python .\tools\quest-camera-profile\Analyze-DisplayEyeUvMapping.py `
  .\artifacts\<run>\display-eye-uv-fiducial-screenshot.png `
  --log .\artifacts\<run>\display-eye-uv-fiducial-logcat.txt `
  --out-dir .\artifacts\<run>\display-eye-uv-analysis `
  --label display-eye-uv-fiducial
```

The renderer logs the fiducial contract in `display-eye-screen-uv` using
`projection_screen_uv_base`. The analyzer writes JSON, Markdown, and overlay
artifacts with marker coordinates, a global affine fit, a near-center
finite-difference mapping around the green center marker, and centerline
nonlinearity/asymmetry between the sampled 0.25/0.50/0.75 marker positions.
Use that local mapping as the bridge from center-cross screenshot-pixel deltas
to named projection-space adjustments; do not treat the screenshot eye halves
as a linear UV ruler.

`Analyze-TargetAlignmentWitness.py` can consume the mapping JSON:

```powershell
python .\tools\quest-camera-profile\Analyze-TargetAlignmentWitness.py `
  --reference .\artifacts\<run>\reference.png `
  --candidate .\artifacts\<run>\candidate.png `
  --display-eye-uv-mapping .\artifacts\<mapping-run>\display-eye-uv-mapping.json `
  --out-dir .\artifacts\<run>\target-analysis `
  --skip-translation
```

This adds local display-eye UV deltas and, when logs are provided, compares the
observed motion against the logged projection-area response model. If the local
mapped response still disagrees, keep the owner on projection-area content
mapping/projection geometry until a denser fiducial or response grid proves
otherwise.

Use `display-eye-uv-fiducial-unorm` to measure the submitted eye image basis
`projection_screen_uv_base`. Use `projection-content-uv-fiducial-unorm` to
measure the post-offset full-frame content basis `full_frame_content_uv`, which
is the named path used by frozen camera/source replay before source sampling.
Use `source-sampling-witness-unorm` when the content basis response is proven
but a physical feature in the frozen source image still disagrees: it renders
the actual sampled source image through the full-frame projection-area path and
overlays yellow/white `full_frame_content_uv` guides plus cyan/magenta final
source-sampler UV guides.
The same analyzer handles both; the log records the UV basis in
`displayEyeUvFiducialUvBasis`.

For the current camera-footprint milestone, run the canvas/collapsed pair
before using native passthrough as the target:

- `camera-stereo-gpu-composite-world-canvas-depth1-mediaprojection`
- `camera-stereo-gpu-composite-camera-footprint-canvas-equivalent-depth1`

The first profile draws the depth-1.0 head-anchored surface as real quad
geometry. The second profile should match it while using
`cameraProjectionMode=display-screen-homography` and
`cameraPipelinePreset=raw-projection-unorm` with
`projectionBorderPolicy=passthrough-underlay`: the shader maps display-eye
screen UV through `screen_to_surface`, then through `surface_to_camera`, and
alpha-disables pixels outside the valid camera footprint to reveal the
passthrough underlay. Treat `camera-stereo-gpu-composite-full-feed-control` as
a negative/lane-parity control only; it is not the custom passthrough footprint
because the raw Camera2 frame is not the full native passthrough FOV.

For strict canvas/collapsed equivalence, override both profiles with
`rustyxr.cameraRawOverlayOverscan=1.0`,
`rustyxr.projectionDepthMeters=1.0`, neutral `projectionArea*` values, and
`rustyxr.projectionBorderOpacity=0.0`. A passing comparison means the visible
canvas and collapsed custom projection share the same surface geometry. After
that, do native-passthrough alignment on the canvas profile first.

The native-passthrough depth sweep should capture two states for each depth:

- canvas-visible: `rustyxr.cameraProjectionMode=world-canvas`,
  `rustyxr.projectionLayerVisible=true`,
  `rustyxr.openxrPassthroughProbe=underlay`,
  `rustyxr.cameraRawOverlayOverscan=1.0`, and
  `rustyxr.mediaProjection=true`;
- passthrough-only reference: same launch context, but
  `rustyxr.projectionLayerVisible=false`,
  `rustyxr.openxrPassthroughProbe=underlay`, and
  `rustyxr.mediaProjection=true`.

`CompositeLayerActivity` hotloads runtime config from a new launch intent, so
an already-installed APK can be swept by relaunching with new extras rather
than rebuilding. Bracket `rustyxr.projectionDepthMeters` first, starting around
closer surfaces such as `0.5` meters when the canvas appears too far away. Once
depth is bracketed, adjust `rustyxr.cameraPreviewFovYDegrees` for vertical
height. Keep `cameraRawOverlayOverscan=1.0` until depth and height are close;
then use overscan only as a named coverage pad. Compare the green center cross
with `Analyze-TargetAlignmentWitness.py` using `--single-view` for
MediaProjection captures and the default per-eye split for HzDB screenshots.

The current native-passthrough aligned world-canvas reference is
`camera-stereo-gpu-composite-world-canvas-native-aligned-mediaprojection`.
It keeps the same direct stereo GPU Camera2 launch context as the depth-1
world-canvas diagnostic and changes only the solved visible-canvas geometry:
`rustyxr.projectionDepthMeters=1.434085`,
`rustyxr.cameraPreviewFovYDegrees=69.763084`,
`rustyxr.cameraPreviewOffsetYMeters=-0.168832`, and
`rustyxr.cameraRawOverlayOverscan=1.0`.

Launch this reference through the catalog runner or another launcher that
passes the complete runtime profile. Do not reproduce it with a minimal direct
`adb shell am start` that only sends the geometry keys: that omits the camera
profile context and can fall back to the slow diagnostic path. A bad launch is
visible in logs as `requestedTier=cpu-diagnostic-flat-copy`,
`stereoLayout=Mono`, `transport=cpu-yuv-rgba`, `uploadCadenceHz~4`,
`requestedAeFpsRange=device-controlled`, and `gpuImportSuccess=0`. A clean
reference launch keeps `cameraTier=gpu-projected`,
`cameraStereoLayout=separate`, `cameraSourceEyeMapping=left-right`,
`cameraTargetFps=72`, `cameraPipelinePreset=raw-projection-unorm`,
`projectionBorderPolicy=passthrough-underlay`, `cameraColorMode=external-rgb`,
`cameraAllowCpuFallback=false`, and
`cameraCpuUploadHz=0`; validation should then show stereo-left/right Camera2
streams, GPU import cache activity, camera cadence around the applied AE range,
and OpenXR cadence returning to display rate after warmup.

### Canvas/Custom Parity Suite

`Invoke-CanvasCustomProjectionParitySuite.ps1` captures the HWB, GLES/OES, and
Makepad canvas/custom matrix. It keeps the solved surface values from the
native-aligned canvas reference, but it must not force fullscreen projection
area values such as `projectionAreaScaleUv=1.0` with
`projectionAreaRadius*Uv=0.5`. Those are fullscreen diagnostics, not bounded
footprint proof values.

For GLES/OES, launch the canvas case with
`rustyxr.directCamera2OesProjectionGeometryProfile=full-frame-diagnostic` and
the custom case with
`rustyxr.directCamera2OesProjectionGeometryProfile=camera-projection`. The
full-frame canvas case maps through the solved screen-to-surface homography so
the camera frame lands on the bounded surface instead of filling the eye.

For HWB, keep the world-canvas reference on
`rustyxr.cameraProjectionGeometryProfile=full-frame-diagnostic`, and keep the
custom/collapsed profile explicit as
`rustyxr.cameraProjectionGeometryProfile=camera-projection` with the bounded
projection-area values `0.47 x 0.36`, corner `0.08`. Do not rely on the direct
Camera2 service default for the custom lane.

For Makepad, keep the evidence build on
`display-left-from-left-source` unless source-eye mapping is the test variable,
matching the HWB and GLES/OES left/right camera-feed convention. Canvas uses
`CameraProjectionMode=world-canvas` with
`full-frame-diagnostic` content mapped through the solved bounded surface;
custom uses `display-screen-homography` with `camera-projection`. Validation
logs should include panel target dimensions, left/right projection-area rects,
left/right expected source-valid rects, source-eye mapping, and
`s91DisplayIndexedHomographyRows=true`.

The suite pregrants normal runtime permissions and records the
`PROJECT_MEDIA` app-op readback before launching MediaProjection cases. It
does not tap Quest consent or selector surfaces. If the app-side receiver gets
no frame, approve MediaProjection manually in headset and rerun the failed
case. The receiver stays alive through the profile run and the suite converts
the latest recorded frame, not the first frame observed during startup. It
prunes older raw payload files while retaining the frame ledger.

The suite writes a labeled `canvas-custom-projection-parity-results.png`
contact sheet into the run root. Use `-EvidenceMode fast-visual` for the
low-latency headset screenshot path: it selects fast direct ADB headset
screenshots and does not run the analyzer unless `-RunAnalyzer` is passed. The
contact sheet falls back to raw headset screenshots when analysis is skipped.
Projection border policy and MediaProjection capture remain controlled by
`-ProjectionBorderPolicy` and `-SkipMediaProjection`; fast visual mode does not
hard-code those features. For the old screenshot-only solid diagnostic sweep,
pass `-EvidenceMode fast-visual` with `-ProjectionBorderPolicy solid-red` and
`-SkipMediaProjection`.
Use `-EvidenceMode full-evidence` for the slower diagnostic sweep: it enables
HzDB headset screenshots, MediaProjection receiver capture, analyzer overlays
and coordinate contracts, contact-sheet output, and timing records, while
leaving `-ProjectionBorderPolicy` independent. The
default `-EvidenceMode custom` preserves the individual `-SkipMediaProjection`,
`-HeadsetCaptureProvider`, `-RunAnalyzer`, `-SkipAnalyzer`, and
`-ProjectionBorderPolicy` switches for mixed investigations. The summary writes
`step-timings.jsonl` and `step-timing-summary.json`; HWB/OES profile runs also
write `profile-step-timings.jsonl`, `profile-step-timing-summary.json`, and a
per-run readiness summary. Makepad runs write `device-gate-timings.jsonl` and
`device-gate-timing-summary.json`.

By default the fast path uses `-CaptureReadinessMode contract`: the harness
starts a bounded streaming logcat window before launch, polls it for renderer
readiness markers such as source-sampling or projection-coordinate contracts,
waits only the configured `-ReadySettleMs`, then captures. The legacy fixed
warmup remains available with `-CaptureReadinessMode warmup`. Makepad follows
the same policy after its existing active-XR/frame/cadence readiness probe; a
fixed Makepad sample window is opt-in with `-UseFixedMakepadSampleWindow`.

The suite validates its JSON/JSONL artifact contract before returning. To check
a saved run manually:

```powershell
python .\tools\quest-camera-profile\Validate-CanvasCustomParityArtifacts.py `
  --suite-root .\artifacts\canvas-custom-projection-parity-suite\<run>
```

Treat HWB and GLES/OES MediaProjection rows as app-frame evidence for the
rendered camera window. The Makepad MediaProjection row is currently a
capture-route diagnostic only: it captures the Makepad Android/window surface
instead of the submitted OpenXR compositor layer, so headset capture remains
the geometry witness for Makepad canvas/custom parity until that
MediaProjection route is understood. By default the analyzer and contact-sheet
builder report failures into the summary without aborting an otherwise clean
capture sweep; pass `-FailOnAnalyzerIssue` when those reporting checks should
gate the run.

## Camera Readiness Preflight

The runner preserves headset power, stay-awake, and proximity state by default.
If a run deliberately needs a timed `hzdb` proximity hold, pass
`-UseProximityHold` and record that choice in the run notes. The older
`-SkipProximityHold` flag remains accepted for scripts that already pass it,
but no proximity hold is taken unless `-UseProximityHold` is present.

Before camera-profile runs after sleep, standby, or sensor-lock transitions,
record passive state first:

- `adb get-state`
- power and VR power-manager dumps
- current foreground/focus
- broker `/status`, `/clock/now`, and `/clock/health` when available
- recent camera-service or app camera log markers

Display-on, wakefulness, and headset-mounted signals are useful context, but
they are not enough by themselves to prove Camera2/PCA readiness. If an
operator is wearing the headset, ask for a simple hand-tracking or system-menu
readiness confirmation before retrying camera acquisition. Treat an
operator-driven long power-button press to the power menu as a manual recovery
step, not as something this runner should automate.

When a run falls into screen-awake but camera-unready state, preserve the
cause-side transition too. Compare pre/mid/post VR power-manager dumps and
recent logcat for `setActivityMonitorState: Idle`,
`releasePowerStateLock: MOUNTED`, `onDeviceIdle`,
`mountWakelock: false`, `setVirtualProxState(DISABLED)`,
`Calling goToSleep()`, and transition to `STANDBY`. A later ADB wake or app
launch can restore the screen while Camera2/PCA frame production is still
unavailable, so the next camera run must re-prove readiness with live frame
progression.

For longer raw-stack verification, `mStayOn=false` and
`stay_on_while_plugged_in=0` mean the Android stay-awake guard is off and normal
timeout sleep can still occur. Use the raw-stack suite's
`-EnableStayAwakeGuard` switch when the run should hold the headset awake; that
logs the prior value and runs `svc power stayon true`. This is a power-management
guard, not a proximity-sensor override.

For unattended camera sessions, use the explicit broker shell-helper watchdog
instead of relying on passive status refresh:

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- broker shell-helper start --serial <serial> --rusty-xr-root . --no-broker-report --skip-status --proximity-watchdog --proximity-watchdog-until-stopped --proximity-watchdog-ensure-stay-awake --json
```

That mode is an active operator-owned guard. It preserves `Virtual proximity
state: CLOSE`, re-applies `svc power stayon true`, wakes the display when power
readback drifts, and keeps reporting the repair counters through
`shellHelper.diagnostics.proximity_watchdog` until the shell helper is stopped.
When coordinating shared local resources, record that guard as non-exclusive
keep-awake/vitals state; reserve the headset and ADB only for active installs,
launches, screenshots, log capture, or validation actions.

The raw-stack suite also checks passive readiness before each mode. A mode is
recorded as failed before launch when ADB is not in `device` state, wakefulness
is unavailable or not `Awake`, or VR power state is unavailable, unmounted,
standby, or waiting for sleep. `-ContinueOnError` keeps the summary moving, but
it does not turn that preflight failure into valid camera evidence.

Use a direct Camera2/HWB profile first when proving camera readiness, then
advance to broker-camera or codec profiles. Accept a run only when camera-frame
progression, visible ROIs or operator witness, projection status, and import
failure counters agree.

## Run A Catalog Profile

From the Rusty XR repo root:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-performance-065 `
  -CaptureHzdbScreencap `
  -FreshnessFrames 6 `
  -FreshnessIntervalMs 1000
```

Useful acquisition probes:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-no-ae-target-065 `
  -CaptureHzdbScreencap

powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-reader-max-3-065 `
  -CaptureHzdbScreencap

powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-native-ndk-065 `
  -CaptureHzdbScreencap

powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-native-single-mirror-065 `
  -CaptureHzdbScreencap
```

Use `-Override key=value` to test variables without adding a new catalog entry.
When invoking the script through `powershell -File`, pass multiple overrides as
a comma-separated list in one `-Override` argument:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-performance-065 `
  -Override 'rustyxr.cameraTargetFps=0,rustyxr.cameraStereoImageReaderMaxImages=3' `
  -CaptureHzdbScreencap
```

For color-pipeline A/B runs in one installed APK, prefer the named pipeline
preset shortcut over repeating the full set of feed, sampler, decode, tone, and
OpenXR color-format extras. Projection geometry is a separate axis, so use
`-CameraProjectionMode world-canvas`, `-CameraProjectionMode display-screen-homography`,
or `-CameraProjectionMode quad-surface` when a run needs to compare the explicit
world-canvas surface, fullscreen display homography, or quad-surface coordinate
reconstruction:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -RuntimeProfile camera-stereo-gpu-composite-native-single-mirror-065 `
  -CameraPipelinePreset raw-feed-unorm `
  -CameraProjectionMode display-screen-homography `
  -CaptureHzdbScreencap
```

Current public presets are `projected-srgb`, `raw-feed-unorm`,
`projected-unorm`, `raw-feed-srgb`, `shader-decode-unorm`,
`separate-decode-unorm`, and `raw-projection-unorm`. The
`raw-projection-unorm` preset keeps the raw feed and UNORM swapchain on the
direct raw-projection shader path; projection exterior fill is selected
separately with `rustyxr.projectionBorderPolicy`. The
`projectionBorderPolicy=solid-red` setting is the clean projection-area
alignment probe: valid projected camera pixels inside the hard public
projection-area mask stay raw, and every pixel outside that area is opaque red
for segmentation and operator checks. `projectionBorderPolicy=passthrough-underlay`
submits a public OpenXR passthrough underlay and makes that same
outside-projection area transparent, which is useful when comparing background
composition separately from raw camera sampling. The `rustyxr.processingLayer=blur`
setting keeps the selected projection-area policy but runs the valid camera
samples through a generic 25-tap diagnostic blur.
HWB, OES, and Makepad normalize the diagnostic blur texel step against the same
1280x1280 source domain used by the broker camera and synthetic diagnostic feed.
Use `rustyxr.cameraBlurRadiusPx` to adjust the visual sample radius for stack
comparison. The app-parsed runtime config log reports both the requested preset
and the resolved feed, sampler, decode, projection-effect, tone, blur radius,
and swapchain settings.
The `display-eye-uv-fiducial-unorm` preset renders a diagnostic marker pattern
at known display-eye UV positions through the same OpenXR submission path. It
is for mirror-capture mapping only; it ignores camera pixels and must not be
used as a camera-source alignment result.
`rustyxr.projectionAreaOpacity` fades valid projected camera pixels, while
`rustyxr.projectionBorderOpacity` fades the solid diagnostic border. For a
red-border passthrough alignment run, use a solid-red preset with
`rustyxr.openxrPassthroughProbe=underlay` and sweep only the opacity values; do
not switch geometry presets while measuring screen-space offsets.
`rustyxr.projectionAlphaMode` can derive valid-camera alpha from source
color after geometry is stable. Supported modes are `fixed`, `red`, `green`,
`blue`, `luma`, and the four inverse variants; the effective alpha is area
opacity multiplied by `clamp(mask * scale + bias)`.
Projection mode remains independent from those presets so border, sampler, and
color modules can be tested against both public geometry mappings in the same
APK.
Projection surface depth is also explicit: the alignment suite's
`-ProjectionDepthMeters` value is emitted by every lane and defaults to `1.0`
meter. Use lane-specific depth overrides only when testing a named architecture
difference; do not bury depth changes in renderer constants.

## Runtime Control Contract

The Quest example keeps A/B modules switchable through the same public runtime
keys whether they come from this PowerShell harness, another launcher, or a
desktop companion process that can send Android intent extras. Keep companion
integrations on these stable keys instead of duplicating shader-specific state:

| Key | Type | Purpose |
| --- | --- | --- |
| `rustyxr.cameraPipelinePreset` | string | Selects the feed/sampler/effect/color-format preset, for example `raw-projection-unorm`, `display-eye-uv-fiducial-unorm`, `projection-content-uv-fiducial-unorm`, or `source-sampling-witness-unorm`. |
| `rustyxr.projectionBorderPolicy` | string | Selects the projection exterior fill policy independently from the camera pipeline preset: `solid-red` or `passthrough-underlay`. |
| `rustyxr.processingLayer` | string | Selects the diagnostic content-processing layer. Use `raw` for unprocessed camera content or `blur` for the public diagnostic blur. |
| `rustyxr.cameraProjectionMode` | string | Selects projection geometry independently from the preset: `world-canvas`, `display-screen-homography`, or `quad-surface`. |
| `rustyxr.cameraProjectionGeometryProfile` | string | Selects direct Camera2 source/content geometry metadata. Active direct lanes accept `full-frame-diagnostic` for full-frame-to-projection-area checks and `camera-projection` for per-eye screen-to-camera homography checks; other values are rejected or reported as unsupported. |
| `rustyxr.directCamera2OesProjectionGeometryProfile` | string | GL/OES direct Camera2 override; falls back to `rustyxr.cameraProjectionGeometryProfile`. |
| `rustyxr.brokerH264ProjectionGeometryProfile` | string | Broker H.264 source/content geometry metadata for camera or synthetic streams; use this for source-agnostic transport checks. |
| `rustyxr.oesSourceColorTransfer` | string | GL/OES external texture color transfer before camera color controls. Default is `srgb-to-linear`; use `identity` only for an explicit OES source-convention A/B run. |
| `rustyxr.projectionDepthMeters` | float | Shared head-anchored projection surface depth in meters. |
| `rustyxr.cameraPreviewOffsetYMeters` | float | Vertical world-canvas surface offset in meters along tracking up; default is `0`. |
| `rustyxr.projectionAreaScaleUv` | float | Shared projection-area scale in display-eye screen UV. |
| `rustyxr.projectionAreaOffsetXUv` | float | Shared horizontal projection-area sweep knob for screen-space centering diagnostics. |
| `rustyxr.projectionAreaOffsetYUv` | float | Shared vertical projection-area sweep knob for screen-space centering diagnostics. |
| `rustyxr.projectionAreaLeftOffsetXUv` | float | Shared left-eye horizontal projection-area override; falls back to `rustyxr.projectionAreaOffsetXUv`. |
| `rustyxr.projectionAreaLeftOffsetYUv` | float | Shared left-eye vertical projection-area override; falls back to `rustyxr.projectionAreaOffsetYUv`. |
| `rustyxr.projectionAreaRightOffsetXUv` | float | Shared right-eye horizontal projection-area override; falls back to `rustyxr.projectionAreaOffsetXUv`. |
| `rustyxr.projectionAreaRightOffsetYUv` | float | Shared right-eye vertical projection-area override; falls back to `rustyxr.projectionAreaOffsetYUv`. |
| `rustyxr.projectionAreaOpacity` | float | Shared valid projection-window alpha, clamped to `0..1`. |
| `rustyxr.projectionBorderOpacity` | float | Shared solid border alpha, clamped to `0..1`. |
| `rustyxr.projectionAlphaMode` | string | Shared color-derived alpha mode: `fixed`, RGB, luma, or inverse variants. |
| `rustyxr.projectionAlphaScale` | float | Shared multiplier applied to the selected alpha mask, clamped to `0..4`. |
| `rustyxr.projectionAlphaBias` | float | Shared bias applied after alpha-mask scaling, clamped to `-1..1`. |
| `rustyxr.cameraBlurRadiusPx` | float | Sets the public diagnostic blur sample radius in 1280x1280 source pixels when `rustyxr.processingLayer=blur`. |
| `rustyxr.xrRenderScale` | float | Controls OpenXR swapchain scale for performance A/B runs. |
| `rustyxr.openxrPassthroughProbe` | string | Keeps native passthrough checks separate from camera projection: `off`, `warmup`, `client`, or `underlay`. |

Makepad uses the same public runtime-key contract through current Android
properties. Stale `debug.rustyxr.makepad.projection.*` projection aliases are
cleared by hygiene but are not accepted as projection-runtime inputs.
`debug.rustyxr.makepad.camera.projection.geometry.profile` selects direct
Camera2 `full-frame-diagnostic` geometry, while
`debug.rustyxr.projection.depth.meters` controls the head-anchored projection
surface depth and is logged back as `projectionDepthMeters`.
The Makepad raw-projection lane also accepts
`debug.rustyxr.camera.preview.fov.y.degrees`,
`debug.rustyxr.camera.preview.offset.y.meters`, and
`debug.rustyxr.camera.raw.overlay.overscan`; the raw-stack suite forwards these
from `-CameraPreviewFovYDegrees`, `-CameraPreviewOffsetYMeters`, and
`-CameraRawOverlayOverscan` so OES and Makepad can be checked against the same
canvas-solved surface shape as the Vulkan/HWB lane.

Projection-area offset keys share the suite-level display-eye screen-UV
contract: positive X moves the projection area right and positive Y moves it
down before final runtime mirror capture. Screenshot pixels are evidence after
runtime/compositor presentation, so large or peripheral screenshot deltas must
not be treated as a linear offset response unless that mapping is logged for
the run. Any renderer-specific sign convention should be normalized by the
renderer profile or launch wrapper before these keys reach the app-specific
backend.

The composite-layer example also exposes physical controller tuning for manual
camera/passthrough alignment: left stick Y adjusts `projectionDepthMeters`,
left stick X adjusts `cameraPreviewFovYDegrees`, right stick Y adjusts
`cameraPreviewOffsetYMeters`, right stick X adjusts `cameraRawOverlayOverscan`,
and the right primary button toggles `projectionLayerVisible`. The app writes
the current values to `files/controller-tuning-state.json`; this runner copies
that file into each run as `<label>-controller-tuning-state.json` when present.
When optional app-private diagnostics are absent, the runner records a
`<label>-*.missing.txt` sidecar instead of printing `run-as cat` errors or
treating the absence as a gate failure.

The app-parsed runtime config log is the authority for whether a switch was
actually applied. It reports the requested preset and the resolved feed,
projection-effect, sampler, import layout, color format, cycle rate, and render
scale. A companion process should record that parsed log line alongside any
operator note before treating a visual observation as valid.

ADB is still needed for install, cold launch, Android permission grants,
screencaps, and log capture. Lighter control channels can reuse the same key
names for operator-driven A/B switching, but they should not invent new setting
names unless the native runtime config parser is updated at the same time.

Native acquisition and OpenXR passthrough-client state are separate axes. To
test runtime passthrough exposure without adding a catalog profile, use
`-Override 'rustyxr.openxrPassthroughProbe=warmup'` or
`-Override 'rustyxr.openxrPassthroughProbe=client'`. Use
`projectionBorderPolicy=passthrough-underlay` when the passthrough layer should
be submitted as a visible underlay. Always compare those runs
against camera-frame progression; passthrough-client state is not a substitute
for live camera delivery.

To test acquisition lifecycle timing without changing projection or color, use
the native profile with a start delay and a log label:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-native-ndk-065 `
  -Override 'rustyxr.cameraStartDelayMs=2500,rustyxr.nativeSourceMode=delayed-synthetic-dual-back' `
  -CaptureHzdbScreencap
```

To isolate renderer/import progression from concurrent stereo camera delivery,
use `camera-stereo-gpu-composite-native-single-mirror-065`. It opens one native
camera source and mirrors the same hardware buffer into both display eyes. You
can select a specific runtime camera ID for a local diagnostic run:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-native-single-mirror-065 `
  -Override 'rustyxr.nativeLeftCameraId=<camera-id>' `
  -CaptureHzdbScreencap
```

Each run captures battery, power, VR power manager, activity/window state,
logcat, a screen capture, optional `hzdb` screenshots, and a validation report.
When `-FreshnessFrames` is greater than one, the harness also captures a short
sequence of screenshots, writes per-frame SHA-256 hashes, flags duplicate hash
groups in `<runtime-profile>-freshness-summary.json`, and passes the sequence to
the validator. The validation report records how many captured frames had
visible non-black screen content and whether the sequence was byte-identical.
Use that freshness summary for camera/parity runs so a frozen or black
screenshot sequence is not mistaken for live camera feed. The harness records a
post-preflight power snapshot even when no proximity hold is requested; the
snapshot prefix is kept stable for existing parsers.

## Validate A Run

The run harness invokes validation automatically when the script is present.
You can also run it manually:

```powershell
python .\tools\quest-camera-profile\Validate-QuestCameraRun.py `
  --image .\artifacts\quest-camera-profile-runs\<run>\<label>-hzdb-screencap.png `
  --logcat .\artifacts\quest-camera-profile-runs\<run>\<label>-logcat-tail.txt `
  --label <label> `
  --sequence-dir .\artifacts\quest-camera-profile-runs\<run>\<label>-freshness-frames `
  --out .\artifacts\quest-camera-profile-runs\<run>\<label>-validation.json
```

The `*-logcat-tail.txt` name is retained for existing tools. New profile runs
write it from a bounded launch-to-capture `adb logcat` process so early runtime
manifest markers are part of the validation window.

The validator rejects obvious black-camera screenshots, log windows with
screen-off, power-sleep, session-exit, or automation-disable signals, and runs
where `Rusty XR final projection status` shows stale camera frames while
OpenXR keeps rendering. Meta shell sleep-timeout lines are warnings to compare
against the captured power and VR-power snapshots; by themselves they are not
treated as proof that the headset display entered standby. A run can hold
OpenXR display cadence and still be invalid for color comparison if the camera
ROIs are black or the camera frame counter only advances a few frames across
hundreds of OpenXR frames. For native acquisition runs, the validation report
also includes native side-frame counts, camera IDs, timestamp deltas, and the
single-camera mirror flag when those log lines are present.

## Compare Screenshots

Use the image comparison helper for public A/B runs or local downstream
reference comparisons. Keep the images and reports under ignored artifact
folders.

```powershell
python .\tools\quest-camera-profile\Compare-QuestCameraImages.py `
  --reference .\artifacts\quest-camera-profile-runs\<run-a>\<image-a>.png `
  --candidate .\artifacts\quest-camera-profile-runs\<run-b>\<image-b>.png `
  --out-dir .\artifacts\quest-camera-profile-runs\<comparison>
```

The report includes per-ROI mean RGB, luma, saturation, RMSE, and a simple
candidate-to-reference channel fit. The contact sheet marks the sampled ROIs so
the metrics can be checked visually before drawing conclusions.
