# Makepad Stereo Comparison Iteration

This document tracks the Makepad-first path toward a direct comparison with the
custom Rusty XR Quest APK `camera-stereo-gpu-composite` profile. It is
public-safe: generated APKs, device serials, raw logs, local SDK paths, and
private downstream behavior stay out of this note.

## Goal

Build a Makepad Quest APK that can be compared against the existing custom
Rusty XR in-app Camera2 stereo projection path.

The comparison must separate:

- Makepad Android packaging and activity ownership.
- Makepad Quest/Vulkan window-swapchain behavior.
- `makepad-xr` scene and OpenXR ownership.
- Camera2 acquisition.
- Hardware-buffer import.
- Stereo projection math and per-eye source selection.
- Border/effect/color shader policy.

## Baseline

The current custom APK baseline is the Rusty XR
`camera-stereo-gpu-composite` profile. That profile opens paired left/right
Camera2 `PRIVATE` sources, imports both hardware buffers, uses metadata-backed
per-eye projection, draws into the OpenXR projection layer, keeps CPU uploads
disabled, and reports paired GPU-buffer status in log markers.

The current Makepad branch baseline is the maintained Makepad fork branch with
the Android packaging fixes, Android Vulkan frame-fence wait, and public-safe
Android activity/bootstrap diagnostics applied. The branch has now passed both
the plain Quest/Vulkan counter smoke run and the Rusty XR synthetic stereo
launcher / generated-XR activity gate.

## Implementation Ladder

1. **Forked synthetic shell.** Build the existing Rusty XR Makepad shell against
   the maintained Makepad fork branch.
2. **Synthetic stereo projection marker.** Add public-safe profile keys and a
   visible synthetic stereo scene without Camera2 or broker transport.
3. **Projection contract bridge.** Mirror the custom APK projection profile
   vocabulary in the Makepad shell logs: source eye mapping, projection mode,
   scale, render scale, and accepted baseline.
4. **Camera metadata pass.** Add metadata/config logging before opening camera
   devices.
5. **Camera acquisition pass.** Add Camera2 permission/acquisition diagnostics,
   still without claiming projection parity.
6. **Hardware-buffer import pass.** Import camera buffers through Makepad's
   Android/Vulkan texture path and report paired buffer status.
7. **Stereo projection pass.** Implement metadata-backed per-eye projection and
   compare against the custom APK `camera-stereo-gpu-composite` and
   `quad-surface` profiles.
8. **Parity performance pass.** Capture comparable Makepad direct-XR and custom
   Rusty XR stereo projection diagnostics, while treating awake/proximity state
   and small hardware-buffer warnings as independent run-quality counters.

## Current Slice

The active slice is S91. S79 matched the public target's display-to-source
mapping, S80 showed that simply scaling fullscreen UVs before the camera
homography is the wrong parity move, S81/S82 kept live feed while narrowing the
gap to exact per-eye display state, and S83/S85 showed that shader-side
Makepad draw-pass projection or forced simplified `screen_to_camera` rows could
still render black despite live Camera2/YUV probes. S86 recovered real camera
detail through direct fullscreen YUV sampling, proving the current Makepad
fullscreen draw path and per-eye YUV texture sampling are healthy. A fresh
public fast `0.75` target recheck again reported `activeTier=gpu-projected`,
`alignedProjection=true`, paired buffers, `cpuUploadCount=0`, app-process
`Tear=0` / `Stale=0`, byte-distinct screenshot frames, and the target
`display-left-from-left-source` mapping. S87 therefore moves the remaining
projection state into a maintained Makepad fork API: the fork now exposes the
active per-eye OpenXR local-space pose and FOV to app Rust, and the example
computes the public target-style `surface_to_screen` -> `screen_to_camera`
homography chain from that runtime view state. The S87 device gate reached
active XR on the first launcher attempt, emitted `runtimeXrViewStateReady`,
kept stale S86/S85 paths absent, captured six byte-distinct screenshots, and
stayed app/global GPU-fault and fatal clean. Visual review showed a
target-like projected feed for the current headset pose, but also exposed a
Makepad-specific invalid-UV policy difference: S87 returned black outside valid
projected UVs, while the public fast shader falls back to an oriented
content-surface sample and dims it. S88 ports that invalid-UV fallback policy
and passed the guarded device gate: the fresh APK contained S88 strings and no
stale S87/S86/S85 path strings, reached active XR on the first launcher
attempt, emitted `runtimeXrViewStateReady`, `projectionHomographyReady`,
`s87RuntimeXrViewHomography=true`, and `s88TargetFastInvalidFallback=true`,
captured six byte-distinct screenshots, and stayed app/global GPU-fault plus
fatal clean while preserving the small hardware-buffer warning counter as a
separate class. Visual review kept the recovered projected camera feed and
made invalid/edge behavior closer to the target. The next split is S89: diff
the S88 runtime-view homography against the validated public fast `0.75` target
implementation and close the remaining close-range parallax/projection
geometry gap before any normalized performance comparison. S89 starts with one
Makepad-only rendering difference that the target does not have: the camera
panel shader still flattened `geom.CubeGeom`, so multiple cube faces could
overdraw the same fullscreen plane with geometry-provided UVs. S89 replaces
that with a single fullscreen quad and derives screen UV directly from the
quad position, preserving S88's runtime-view homography and invalid-UV fallback
policy. S89 was source/build/string-ready, but the Quest device gate was
blocked by missing ADB transport, so it produced no visual conclusion. S90
therefore closes a separate implementation delta found in the target diff:
the public target orders the stereo sources by Camera2 pose X and logs display
camera IDs, while the Makepad lane still selected by Camera2/Makepad list
indices. The maintained Makepad fork now exposes Android camera IDs in
`VideoInputDesc.name`, and the public example carries Camera2 IDs through the
projection plan, orders the selected Camera2 pair by physical pose X, and binds
Makepad video choices by camera ID before falling back to source index. The
next device gate should require `s90CameraIdSourceBinding=true` and
`sourceBindingMode=camera-id` before using the headset view to judge whether
the close-range parallax delta is gone or still needs projection math work. The
S90 source state is pushed on the public branch. A follow-up static diff against
the validated public fast `0.75` target confirmed that the current Makepad lane
now uses the same pose-X source ordering, runtime per-eye OpenXR view/FOV
inputs, shared `surface_to_screen` -> `screen_to_camera` homography helpers,
projection scale, preview FOV, raw overscan, and `left-right` source-eye mapping
as the target. S90 also adds host unit coverage for Makepad descriptor
`cameraId=` parsing and camera-ID-preferred pair binding so this correlation
step does not silently fall back to source index. Operator review then rejected
S90 as projection parity: the image still had the depth-dependent stereo
misalignment/parallax, an apparent left/right source-eye flip had returned, and
roll testing showed an orientation defect where a horizontal real-world surface
rotated toward vertical on screen. S91 therefore changes the active Makepad
math rather than acquisition: display-eye homography rows remain indexed by
display eye, source-eye texture selection is inverted, and the texture
orientation returns from the S86/S90 180-degree `flip-x-and-y` transform to the
earlier vertical-only orientation that preserved the stereo-baseline signal in
the public camera-model tests. The fresh S91 launcher gate reached active XR,
emitted the S91 source/display-row markers with stale S90 path counters at zero,
captured six byte-distinct screenshots, and stayed app/global GPU-fault and
fatal clean while the small hardware-buffer warning class remained visible. This
is a best-effort correction pending the next headset review; it is not yet a
visual acceptance claim, but it is suitable for objective transport/performance
diagnostics.

Historical context: S67 was a known-visible app-panel positive control for the
paired per-eye Makepad camera panel. S59 remains the color control: Android
YUV_420_888 plane order is sampled without swapping U/V, converted as limited
BT.601, and kept marked `visualReleaseAccepted=false` until final headset
inspection accepts the full projection. S60 added the Makepad `xr_view_id()`
shader signal and selected left/right YUV textures per eye. S61 fixed the app
logic so an early single-stream visual proof no longer terminates the paired
path. S62 then removed Makepad `XrEnv`'s unintended headset-camera competition
when `env_cube=false`; the device run reached paired left/right camera frames,
kept app-process GPU-fault/fatal/small-buffer counters at zero, retained
per-eye texture markers, and captured six byte-distinct screenshots.

The operator-visible S62 result was still not projection parity: the camera
content was live and coherent, but the panel remained a world-space surface, so
it stayed parked in the room when the headset moved. S63 therefore moved the
diagnostic panel vertex placement into active-eye view space using the
Makepad/OpenXR camera inverse while preserving the same paired YUV sampling
path. The first S63 device gate stayed clean and fresh, but operator inspection
reclassified the image: the apparent full-screen view was Meta passthrough, and
the app-owned camera panel was not visible. S64 tried a direct clip-space
head-locked control; its headset captures were byte-distinct and live, but the
app-owned panel/border was still absent while native passthrough remained
visible. S65 therefore changed two variables only for the visibility gate:
disable Makepad's native OpenXR passthrough composition layer in the maintained
fork and draw a thick black border around the app-owned panel. Its first device
run also exposed an unattended-launch gap: after a clean install, Scene Access
must be granted alongside Headset Camera before `XrPermissionsFlow` calls into
XR presenting. Once Scene Access was granted, S65 reached the generated XR
activity with nonzero XR update cadence and confirmed the native passthrough
layer was disabled, but the app-owned panel was still absent against the black
background. S66 therefore restored the last known visible world-space panel
pose, used a non-black app clear color, and bypassed camera sampling with a
solid neutral panel plus thick black border. Device validation reclassified
that gate again: after full Scene Access and Headset Camera grants, the
launcher path reached the generated XR activity with nonzero XR update cadence
and the native-passthrough-disabled marker, but the visible output still did
not show the non-black clear color, solid panel, or black border. A
loading/preflight screen remains a launch failure, not a passthrough-off
success, regardless of marker strings. This does not reopen the earlier panel
proof: S49-S59 already showed the Makepad app-owned camera panel visibly
rendering live camera content, with the known gap that it was world-space
anchored instead of mapped into the per-eye camera/head space. S67 therefore
restores that known-visible world-space panel as a positive control, using
native passthrough enabled, the no-swap limited-BT.601 live camera path, and the
black-border guide. The Makepad fork revision for this gate is `c3ea53e`, which
also adds end-frame diagnostics and an explicit native-passthrough toggle so
the passthrough-off branch can be isolated separately after the visible-panel
control is re-established. The gate now runs in three ordered steps: S67a first
verifies the passthrough-on known-visible panel target; S67b turns passthrough
off and requires a solid app-owned background plus panel; S68 starts from the
S67b passthrough-off visible-panel state and moves that same panel into
active-eye camera-inverse placement. S68 is a placement/liveness gate only: it
must keep the app-owned panel visible, fresh, and passthrough-off before it can
be inspected for non-world-space behavior, but it is not final camera
projection parity. The first S68 static gate passed that visibility side: both
launcher and direct generated-XR paths reached active XR, emitted the S68
active-eye placement markers with native passthrough disabled, captured six
byte-distinct screenshots each, and showed the app-owned live camera panel plus
pale border against the solid app-owned background. App/global GPU-fault and
fatal counters stayed at zero, while the known small hardware-buffer warning
class and environment-depth runtime logs remained visible. The only remaining
S68 classification is now complete after operator headset inspection: the
visible panels are cleanly projected per eye and no longer behave like the
earlier room/world-space surface. This makes S68 a camera/head-space panel pass,
not full stereo camera projection parity. The active gaps are source-eye
handedness and stereo alignment: the camera feeds are swapped left/right
between eyes, and the panels are not yet aligned well enough for a comfortable
stereo effect. S69 therefore fixes only the left/right source mapping before
S70 tackles projection alignment using the existing custom Rusty XR target
alignment notes.

Earlier context: the current Makepad path is the
scene-owned Makepad camera panel. S14 cleared the
active-presentation blocker for the Makepad launcher path, S16-S20 proved
scene-owned geometry, S26 restored the manual `VideoExternal` import route, S33
isolated the app-owned panel from native compositor passthrough ambiguity, S36
fixed the generated-XR activity handoff, and S40 proved that shader edits can
change the visible panel from stale proof colors to a neutral guide state.
S41-S45 then moved the blocker below Camera2 acquisition: CPU-side Y/U/V plane
content is present, active OpenXR cadence is present, the same result holds
with gain-boosted Y-plane sampling, synthetic R8 texture controls, all-slot
texture controls, and a marker-selected constant-gray bypass. S46 then proved
the visible panel does execute edited shader code when the fragment path returns
before any texture sampling. S47 then proved a generated R8 texture bound
through the same panel slot can be sampled visibly as a checker pattern. S48
disabled the generated replacement and sampled the real Makepad camera Y plane;
it passed as camera-plane proof, but the inherited `*8` luma gain saturated the
panel. S49 removed that gain and produced a clear monochrome camera image. The
operator visual review then reclassified the next blocker: the feed was visible
but upside down, and the diagnostic panel was still offset too low for direct
camera-front inspection. S50 kept the same early-return shader path, rotated the
direct camera sample 180 degrees, and moved the panel upward. Device validation
showed the S50b panel in a usable head-forward diagnostic position with an
upright monochrome camera image, while preserving active XR focus, passive
awake/proximity state, zero app-process GPU-fault/fatal counters, and the small
hardware-buffer warning counter. Operator review then confirmed the orientation
and placement but identified one remaining handedness issue: the panel image
was left/right mirrored relative to the camera feed. S51 kept the panel
position and replaced the 180-degree rotation with a vertical-only flip so the
image stays upright without the horizontal mirror. Device validation retained
active XR focus, emitted only the upright/unmirrored marker path, preserved
passive awake/proximity state, kept app-process GPU-fault/fatal counters at
zero, and kept the small hardware-buffer warning counter separate. Screenshot
inspection shows the diagnostic panel upright, centered, and no longer
horizontally mirrored; if live headset observation disagrees, treat that as the
next visual gate before color work. S52 then combined the real Y/U/V planes
with the same Makepad YUV-to-RGB math used by the stock video widget. Device
validation stayed active-XR and app-fault clean, and CPU-side texture probes
confirmed non-empty Y, U, and V plane content, but the headset view was
strongly green/cyan. Existing Rusty XR YUV/color notes classify that class of
visual as a sampler/decode-shape diagnostic rather than a reason to tune color
matrices blindly. S53 therefore visualizes the sampled Y, U, and V texture
slots as three grayscale bands in the same centered/upright early-return panel
before resuming channel-order, range, or matrix work. S53 passed: the headset
view showed three live grayscale bands for Y, U, and V, proving the GPU can see
all chroma texture slots and moving the blocker from texture binding to chroma
interpretation. S54 kept the same centered/upright panel and shader return
path, but swapped the U and V inputs into the Makepad YUV-to-RGB conversion.
That device gate stayed active-XR and app-fault clean, retained no stale S52/S53
markers, and produced a real color camera feed instead of the earlier
green/cyan decode failure. The remaining visible color gap is calibration:
the panel is still brighter/warmer than the validated Rusty XR hardware-buffer
sampler path. S55 compared swapped-U/V limited/full range and BT.601/BT.709
variants in one four-way grid and stayed active-XR, app-fault clean, and
visibly live, but its first implementation sampled different camera regions per
quadrant. That makes S55 a stability and marker pass, not a decisive color A/B.
S56 kept the same four formulas and remapped each quadrant to the same full
camera view so the screenshot could compare range/matrix choices directly. That
run stayed active-XR and app-fault clean and made the formula comparison fair.
The Rusty XR validated hardware-buffer logs report BT.601 narrow-range hints
for the same public camera format, but S58 proved those hints are not enough to
accept the Makepad CPU-YUV shader path blindly. S57 collapsed the diagnostic
grid into a swapped-U/V limited-BT.601 candidate and S58 removed the
center/split guide, leaving only a border overlay. The border cleanup worked,
but visual review showed a blue/cyan skin-tone regression in the camera panel.
That reopens color conversion and makes S59 a simpler no-swap limited-BT.601
control against Android YUV_420_888 plane-order semantics before paired/per-eye
projection parity:

- keep the maintained Makepad fork branch
  `rusty-xr/android-libstd-packaging` as the Android app-shell dependency
- preserve the completed Camera2 metadata, bounded acquisition, paired Makepad
  import, and metadata-backed projection-mapping markers
- validate active Makepad XR presentation through the launcher/normal activity
  path, because `XrPermissionsFlow` owns the switch into the generated XR
  activity
- treat direct generated-XR launches as bootstrap/control smokes until a run is
  explicitly designed around that switch behavior
- keep the paired Makepad camera textures bound through the manual
  `VideoExternal` import route, retain the projection markers through cadence
  samples, and treat visual confirmation as a separate gate from marker success
- keep the S63 head-locked placement marker separate from true metadata-backed
  camera reprojection; it proves a HUD-style diagnostic surface follows the
  active eye, not yet that the custom projection math is final
- keep environment depth optional/non-fatal in the maintained Makepad fork so a
  depth-policy failure cannot block passthrough/projection startup
- keep the small `AHardwareBuffer` warning class visible as a separate counter
  from app-process GPU page-fault and fatal signatures
- use passive awake/proximity readback before and after samples; do not issue a
  new proximity-control command during comparison captures unless an operator
  explicitly asks for it
- capture a short multi-frame screenshot freshness sequence for visual gates
  and reject or annotate byte-identical sequences when live camera motion is
  expected
- after clean installs, grant both Scene Access and Headset Camera before
  classifying Makepad XR presentation. Without Scene Access, the launcher path
  can stay in the regular/preflight activity with `XrUpdate=0` while app-level
  camera and marker code still runs.

## S68 Run-Log Review Notes

A post-S68 review of the long device-run transcript found several items that
must stay explicit before S69/S70:

- S51 fixed horizontal image mirroring by changing the single-stream diagnostic
  sample to a vertical-only UV flip. The S68 defect is different: the left and
  right camera sources are swapped between eyes. S69 should swap the source-eye
  selection first, not reintroduce horizontal UV mirroring.
- S58 reclassified the earlier swapped-U/V color candidate as color-regressed.
  Keep S59's no-swap limited-BT.601 CPU-YUV shader path as the Makepad color
  control until a new device run proves a better candidate.
- S60/S61/S62 fixed per-eye texture selection, paired-stream waiting, and the
  Makepad environment-camera competition. Preserve those fixes while changing
  S69 source selection and S70 alignment.
- The visible S62 world-space target was recovered from ignored device
  artifacts rather than from a standalone committed source revision. Future
  visible gates should be committed and pushed before extended iteration, so
  they can be restored without artifact archaeology.
- A Makepad launch stuck in loading/preflight, or showing `XrUpdate=0`, is a
  failed launch even if app-level marker strings are present. Native compositor
  passthrough filling the display is not proof that the app-owned panel is
  visible.
- Use multi-frame screenshot freshness checks for every visual gate. A single
  screenshot is not enough to distinguish live camera content from stale or
  loading imagery.
- Keep CPU/GPU level, proximity/awake state, small hardware-buffer warnings,
  app-process GPU-fault counters, and fatal-signal counters as separate
  run-quality fields. Do not change proximity or performance levels during a
  comparison unless that is the variable under test.
- When more than one Android device is connected, all Quest commands must carry
  the selected Quest serial explicitly.
- The Makepad example is standalone rather than a root-workspace package. Use
  `cargo check --manifest-path examples/makepad-q2q-camera-shell/Cargo.toml`
  from the repository root, or `cargo check` from the example directory. Do not
  use root `cargo check -p` for this example.
- Launcher validation must record recovery class. If the first launcher start
  remains in loading/preflight, retry the launcher once and record that as
  `launcher-attempt-2`; use direct generated-XR launch only as an explicit
  fallback/control, not as a silent replacement for launcher success.

## Android Toolchain Path Policy

The custom Rusty XR APK scripts and the Makepad APK lane intentionally remain
separate build routes:

- Custom Rusty XR APK scripts use the shared
  `tools/android/Resolve-AndroidToolchain.ps1` resolver. It accepts a Unity
  `AndroidPlayerRoot` for compatibility, but can also use split SDK, NDK, and
  JDK roots through explicit parameters or environment variables. The resolver
  validates that the selected JDK tools exist and that `javac` can run, so a
  present but broken bundled OpenJDK is rejected before a long APK build.
- Makepad comparison builds use `cargo makepad android` with a Makepad Android
  SDK path. This is the preferred route for testing Makepad Android lifecycle,
  generated activities, and fork changes because it exercises the same
  packager Makepad downstream apps will use. Build this example from
  `examples/makepad-q2q-camera-shell` and pass Makepad Android options before
  the `build`/`run` subcommand with `--key=value` syntax.
- Keep both routes while they answer different questions. Collapse them only if
  the Makepad route can also build the custom Rusty XR profiles without losing
  the explicit OpenXR/Vulkan diagnostics, or if the custom scripts can exercise
  Makepad's generated Android surface without bypassing the behavior under
  test.

Makepad workflow guardrails from the S68 transcript review:

- A failed Makepad build can leave an older APK in place. Delete or timestamp
  the expected APK path before building, then record the fresh APK hash.
- Because APK compression can hide marker strings from plain text search,
  extract `lib/arm64-v8a/libmakepad.so` before checking diagnostic strings.
- Clean generated Makepad `target/` outputs before public pushes and public
  boundary scans. Generated APKs, wrapper output, and local compiler metadata
  are not source artifacts.
- Commit and push Makepad fork changes first, then update this Rusty XR
  example's lockfile/revision. Avoid relying on local-only fork edits during
  Quest device gates.
- Use the Makepad device-gate harness in
  `examples/makepad-q2q-camera-shell/tools/Invoke-MakepadQ2QDeviceGate.ps1`
  for headset runs. It installs with `--no-incremental` when available, grants
  ordinary camera/scene runtime permissions, probes the first launcher attempt,
  retries the launcher once on loading/preflight, records any direct-XR
  fallback separately, and captures multi-frame freshness evidence plus key
  fault/stale-marker counters.
- Clean `target/android/makepad-android-apk/` before Makepad APK rebuilds. That
  is the current generated APK output root for this example; cleaning the older
  `target/makepad-android-apk/` path is not enough to prove freshness.

The same counters remain valid for comparison runs: app-process GPU page-faults,
fatal signatures, small hardware-buffer warnings, runtime cadence,
camera/source progression, CPU upload count, projection-ready flags, and
visual-classification evidence. The current camera-streaming proof remains
open because the app-owned panel is visible and non-occluded, shader edits are
live, generated texture-slot sampling has a positive proof, the real camera
planes are visibly sampling after orientation and handedness correction, and
S59 has a live no-swap color control. Paired left/right source selection and
metadata-backed per-eye projection remain open. This slice does not add broker
streaming, private visual-effect policy, or downstream effect acceptance.

Device validation now uses two log windows:

- a short startup capture for Java activity, native bootstrap, and Rust app
  marker presence
- a longer liveness/fault capture for app-process GPU page-fault and fatal
  counters

The split matters because the Quest log buffer can lose startup-only marker
lines during noisy 90s captures even when the app started correctly.

## Hardware-Buffer Import Comparison

The known working custom path imports camera hardware buffers directly in the
app's Vulkan/OpenXR renderer: query `AHardwareBuffer` Vulkan properties, build
an external-memory image, include `VkExternalFormatANDROID` when the buffer
reports an external format, allocate/import Android hardware-buffer memory,
bind the image, create the YCbCr conversion / sampler / image view, cache the
import, and bind the resulting image into the projection pipeline.

The S6 Makepad diagnostic deliberately stops earlier. It lets Makepad own the
Android camera playback and texture update, then waits for
`VideoTextureUpdated`, which is only emitted after Makepad's Vulkan backend has
accepted the hardware buffer through its Android video texture import path. It
does not create Rusty XR projection descriptors, paired left/right imports, or
the custom app's import cache. That keeps the split focused on whether
Makepad's Android/Vulkan backend can import camera buffers on the same device
class where the custom Vulkan path is already known to work.

## Performance Comparison Gate

A final performance comparison against the custom stereo camera projection
baseline is almost open, but it must wait for visible camera pixels in the
Makepad headset view. S14 opened active-presentation and cadence comparison
again, S15 confirmed the custom path's visible stereo output, and S16 reports
camera-panel readiness from paired `VideoExternal` textures. S16 headset
inspection still showed the synthetic debug scene; S20 now shows a visible
scene-owned diagnostic panel but not camera texture sampling. Final parity
performance should use the first marker set that also visibly samples the
paired camera textures, plus a headset screenshot/operator visual check. S22
kept that gate closed: native `Video` widgets could be started inside an XR
view, but they did not produce a confirmed prepared/update sequence for visible
camera pixels.

Earlier S6 state proved that one Makepad-owned camera hardware buffer could
reach `VideoTextureUpdated` while still reporting
`pairedLeftRightGpuBuffers=false` and `alignedProjection=false`.

Before running parity performance diagnostics, the Makepad path must:

- own or bridge paired left/right camera buffer sources
- map the selected sources into per-eye projection descriptors
- report `pairedLeftRightGpuBuffers=true`
- report `alignedProjection=true` for the selected projection mode
- keep the current strict counters for app-process GPU page faults, fatal
  signals, and the separate small hardware-buffer warning class

The generated-XR S7 smoke has now flipped those markers true for the direct XR
activity path. Performance captures can start there, with visual inspection
still tracked separately through `visualInspection=required` and
`visualReleaseAccepted=false`. The normal launcher path remains a separate
activity/lifecycle regression after one S7 run hit a Horizon OS display-event
receiver failure before app startup markers.

The S8 comparison must avoid changing the headset keep-awake/proximity mode as
part of the run itself. Capture passive `vrpowermanager` and power snapshots
before and after each sample, then reject or annotate runs where the device
moves toward standby, an automation/proximity transition appears in logcat, or
a Horizon OS service restart occurs.

The first S8 batch is informative but not yet a fair normalized performance
comparison. A 75s direct generated-XR Makepad sample held the S7 markers,
reported paired/aligned projection readiness, kept CPU uploads at zero, and had
zero app-process GPU page-fault or fatal signatures. Runtime cadence rows
showed about 90/90Hz with app work around 3-5ms; the separate small
`AHardwareBuffer` warning class remained noisy and visible. The custom Rusty XR
0.65 profile remains the valid comparison baseline from this batch: it reported
paired/aligned projection, zero CPU uploads, zero GPU import failures, about
70.8/72Hz OpenXR cadence, and about 29Hz paired camera progression, with a
warning status due to sleep-timeout / compositor-delay signals. The attempted
custom 0.75 normalization run is invalid for comparison because it failed before
final projection markers with a display-event-receiver startup failure and also
reproduced a keep-awake/proximity state transition from the operator-maintained
hold toward standby; the keep-awake state was restored after the run.

The next fair comparison needs either a stable custom 0.75 rerun with the
awake/proximity and Horizon OS lifecycle behavior controlled, or a Makepad-side
0.65 / 72Hz profile knob. The shared scorecard also needs Makepad continuous
frame/camera cadence markers equivalent to the custom OpenXR frame and stereo
camera pair markers before it can compare camera progression directly.

S9 starts that marker gap. The Makepad example now emits
`RUSTY_XR_MAKEPAD_CADENCE` samples every five seconds. These samples use
Makepad `NextFrame` events for callback cadence and left/right
`VideoTextureUpdated` events for Makepad-owned camera texture progression. The
marker carries total/delta counts, interval rates, paired texture-update rate,
the current paired/projection readiness flags, and `cpuUploadCount=0`. This is
instrumentation only until a Quest run proves the marker appears in the direct
generated-XR path and the shared scorecard consumes it.

The first S9 direct generated-XR device validation passed. A 25s startup /
projection window retained startup, Camera2, hardware-buffer import, projection
complete, paired comparison, and cadence markers. A 60s liveness window retained
five cadence samples and kept app-process GPU page-fault and fatal counts at
zero. The retained cadence rows reported paired/projection readiness true,
`cpuUploadCount=0`, and about 12.5-12.7Hz for both Makepad `NextFrame` callback
and paired camera texture-update rates. Treat this as Makepad callback/camera
progression evidence, not display FPS; the scorecard continues to use `VrApi`
rows for runtime display cadence when they are present. The separate small
hardware-buffer warning class remained visible.

S10 starts the normalized comparison rerun. The Makepad side uses the S9
direct-XR scorecard input with paired/projection readiness, zero CPU uploads,
Makepad callback/camera texture cadence, `VrApi` timing, app fault counters, and
the visible small hardware-buffer warning class. The custom side reruns the
matching 0.75 camera projection profile with the external/device-side
proximity watchdogs already active and the comparison harness configured for
passive pre/post power-state readback instead of sending an additional timed
proximity hold. This split is intended to determine whether the earlier custom
0.75 failure was a stale lifecycle/proximity artifact and whether Makepad's
observed 12.5-12.7Hz camera texture cadence is a real parity bottleneck.

The first S10 same-session batch makes the comparison materially clearer. The
custom 0.75 profile no longer reproduces the previous display-event-receiver
startup failure when the watchdog state is preserved and the comparison harness
does not send an extra timed proximity hold. It reached focused GPU projection
at 0.75 render scale with paired/aligned buffers, `cpuUploadCount=0`, zero GPU
import failures, a steady OpenXR cadence near 72Hz, and about 49Hz paired camera
progression. It remains a warning baseline rather than a clean pass because
runtime sleep/wake and compositor warning counters were still present. A fresh
Makepad direct-XR sample in the same session stayed app-fault clean, retained
paired/projection readiness through cadence rows, reported `cpuUploadCount=0`,
and again reported only about 12.6Hz for Makepad `NextFrame` callback and
paired camera texture-update progression. Follow-up S11 launch-state inspection
showed that these Makepad rows should be treated as app-process/surface/camera
cadence evidence, not full immersive presentation evidence: the retained
Horizon OS launch logs show a volumetric-window launch path rather than a
confirmed loading-complete immersive handoff. The `VrApi` rows in these Makepad
artifacts therefore remain runtime context, not proof that the Makepad app view
was being presented in-headset. The small hardware-buffer warning class
remained visible on the Makepad side and still did not correlate with
app-process GPU page-fault or fatal signatures.

S10 therefore turns the next investigation from "can custom 0.75 launch
stably?" into "why is Makepad-owned camera texture progression much slower than
the custom Camera2 pair progression?" The next split should isolate whether that
12.6Hz signal is caused by Makepad `NextFrame` scheduling, Android video
playback cadence, source/FPS negotiation, or the diagnostic marker sampling
point.

S11 starts that isolation without changing the render path. The cadence marker
now counts Makepad `XrUpdate` and draw events alongside `NextFrame` callbacks
and `VideoTextureUpdated` camera texture updates. The selected Makepad camera
format markers also include the frame-rate metadata reported by Makepad's
`VideoFormat` abstraction. The next device run should show whether OpenXR
updates are running near runtime cadence while `NextFrame` and camera texture
updates are slower, or whether the Makepad XR loop itself is the cadence limit.

The first S11 device pass is partial and reclassifies the previous Makepad
performance gate. A clean install initially granted Android camera permission
but left the headset-camera runtime permission unset, so Camera2 enumeration
collapsed to one public source and the Makepad paired source selection reported
no stereo pair. After granting the requested runtime permissions and clearing a
stale permission-controller task, the app process reached a resumed/reported
drawn Android state and emitted S11 cadence rows with paired/projection flags
true and `cpuUploadCount=0`. The operator still saw the headset loading screen.
The retained Horizon OS logs show the generated activity entering a
volumetric-window launch state, not a confirmed loading-complete immersive
handoff. In that state, Makepad `NextFrame`, draw-event, and paired camera
texture-update rates clustered around 12.7-13.1Hz, while the app-level
`XrUpdate` counter stayed at zero. This is useful for locating the cadence
source inside Makepad's Android/event surface, but it is not valid
presentation-performance evidence.

S11 therefore adds a new gate before any Makepad-vs-custom performance
comparison: prove generated-activity presentation completion. The next split
should compare the Makepad generated manifest/activity launch path against the
known-good custom OpenXR path and against the earlier Makepad counter smoke,
checking for permission-dialog residue, Horizon loading-complete/immersive
handoff signals, and whether Makepad is submitting a true immersive OpenXR
client rather than only ticking a surface window with camera textures.

S12/S13 narrowed that loading-screen gate further. Adding Quest-style manifest
XR launch metadata and non-resizeable/focus-aware activity declarations was
accepted by the installed package, but did not by itself complete immersive
presentation. Adding Makepad's `XrPermissionsFlow` to the example changed the
valid smoke route: direct generated-XR activity launch becomes a
bootstrap/control split because the flow can switch back to the paired normal
activity, while the launcher/normal activity path is the route that should
switch into the generated XR activity for active presentation. That launcher
path reached Makepad OpenXR session creation and started passthrough, then
failed session setup when the runtime rejected environment-depth provider
creation. This is distinct from the earlier GPU page-fault depth split: the
current symptom is loading-screen/session completion, with zero app-process GPU
page-fault and fatal signatures in the retained app process.

S14 moved that blocker into the maintained Makepad fork. The fork now treats
environment depth as optional across provider setup, swapchain/image
enumeration, provider start, per-frame acquire, and teardown, while
passthrough and projection remain strict. The launcher-path rerun then cleared
the loading-screen gate: the operator saw the synthetic stereo scene in headset
and the retained cadence rows showed Makepad app frames, `XrUpdate`, and draw
events at about 90Hz with paired camera texture progression around 50Hz. The
depth provider still failed at the runtime layer, but it was downgraded to a
visible optional-depth warning instead of aborting session creation. App-process
GPU page-fault and fatal counters stayed at zero, paired/projection markers
were true, `cpuUploadCount=0`, and the small hardware-buffer warning class
remained visible but much lower than earlier noisy loading-screen samples.

S15 ran the first post-S14 comparison batch and then split it into valid and
invalid evidence. The initial custom sample is discarded because Makepad was
still running and polluted the shared log window with Makepad cadence markers.
A clean custom 0.75 sample after force-stopping the Makepad package retained
proper visible custom stereo projection according to operator inspection, but
it was degraded: the live passive sample showed OpenXR around 52-53Hz against a
72Hz target, paired camera progression around 28Hz, zero GPU import failures,
zero CPU uploads, no app fatal or GPU-fault signatures, and a high compositor
slice-tear count. Power/proximity readback stayed mounted/awake. Therefore S15
is useful custom-baseline evidence and a Makepad active-presentation/import
comparison, but not a final parity-performance conclusion.

S16 adds a persistent Makepad XR camera panel that samples the left/right
`VideoExternal` textures with Makepad's XR view index. The first successful
launcher-path S16 validation retained `visibleCameraProjectionReady=true` in
startup and liveness windows, held app/`XrUpdate`/draw cadence near 90Hz, held
paired texture-update cadence near 50Hz, kept `cpuUploadCount=0`, and reported
zero app-process GPU page-fault and fatal signatures. The small hardware-buffer
warning class remained visible in the startup window. Follow-up headset
inspection reclassified S16 as marker-level only because the visible scene was
still the synthetic blue/red debug geometry. S17 now hides that fallback scene
after camera binding and adds a panel draw marker so screenshot inspection can
separate "panel not drawn" from "texture drawn but visually wrong."

S17-S20 continued that visual split. Removing the fallback scene produced a
black headset view with markers still true. A solid-color branch in the custom
shader also stayed black. Moving the panel under a scene-owned `XrNode` kept the
draw marker scene-owned but still stayed black. Replacing the custom shader
output with Makepad's existing `DrawCube` path from the same widget transform
produced a visible cyan diagnostic panel. That isolates the next blocker to the
custom shader / video-texture draw path, not the launcher path, XR scene
ownership, or paired texture readiness markers.

S21/S22 tested both directions from that control. Inheriting the custom shader
from `DrawCube`, first with the original deref field and then with Makepad's
`draw_super` convention, stayed app-fault clean but also exposed a shader
compile limitation around the attempted XR view-index helper, so it still
produced a black headset view. Replacing the custom shader with Makepad-native
`Video` widgets in an XR view started the native video-widget diagnostic path,
but no confirmed prepared camera playback or paired texture-update sequence
appeared and headset inspection remained dark. The native widget result points
at Makepad's default camera-video path rather than scene geometry: the Rusty XR
manual path uses the headset-camera playback permission route, while stock
`Video` camera playback currently routes through Makepad's ordinary camera
playback API. S23 added that permission option and proved the shader noise is
gone, but it also showed the selected `Video` widgets were already in a
`Playing` state by the time the diagnostic assigned the camera source. S24 added
an explicit cleanup/retry gate before assigning the headset-camera source, which
then exposed a Makepad Android cleanup completion gap: cleanup for a video id
without a retained platform resource left the widget in `CleaningUp`. S25 moved
that completion into the maintained Makepad fork and reran the visual gate. That
split is now rejected: the native `Video` widget surface started after one
reset, but no prepared/update sequence appeared and the old app-process GPU page
fault plus premature surface-free class returned. The cleanup-completion fork
patch has been reverted, and S26 disables the native widget diagnostic before
rerunning the launcher path as a recovery control.

S27-S29 returned to the manual `VideoExternal` route from that recovery state.
S27 disabled the solid diagnostic branch, S28 switched the shader to direct
`sample_video()` color, and S29 disabled the alignment-guide overlay. The
launcher path stayed app-fault clean with paired prepared/update markers and
`visibleCameraProjectionReady=true`. The direct generated-XR activity remains a
control path: one S29 direct launch briefly opened the generated activity, then
Horizon OS rebuilt the normal Makepad activity as a volumetric surface. The
launcher path remains the authoritative active-XR visual gate.

S30 corrected the custom Rusty XR comparison baseline after a degraded live
sample used the default `camera-stereo-gpu-composite` 0.75 render-scale profile.
That default profile is useful for visual geometry/color comparison, but recent
device runs show it can fall into stale-frame and slice-tear-heavy performance.
The documented performance comparison baseline is
`camera-stereo-gpu-composite-performance-065`: it keeps the same stereo
projection, border, source-eye mapping, and `external-rgb` sampler assumptions,
while lowering only the OpenXR render scale. A fresh harness run with
proximity-hold changes disabled and the previous VR foreground app stopped
focused the custom composite activity, showed visible in-headset stereo camera
projection, reached `observedOpenXrFps=70.7` against a 72Hz target with
`activeDisplayRefreshHz=72.0`, and retained paired/aligned GPU projection with
`cpuUploadCount=0`, `gpuImportFailure=0`, and no app-process fatal or GPU-fault
signatures. The last stereo-pair marker reported about 29Hz paired camera
progression with zero drops. The run remains a warning baseline, not a clean
release pass, because the log retained sleep/wake warning signals and a small
number of compositor slice-tear warnings.

## Attempt Ledger

| Attempt | Slice | Validation | Result | Next |
| --- | --- | --- | --- | --- |
| S0 | Documentation and fork setup | Rusty XR public docs and Makepad fork pushed | Rusty XR branch and Makepad fork branch are clean and pushed; fork branch contains packaging, CSG metadata, frame-fence, and fork-maintenance guidance state | Build the Rusty XR Makepad shell against the fork |
| S1 | Synthetic stereo comparison marker and scene | `cargo fmt --manifest-path examples/makepad-q2q-camera-shell/Cargo.toml --check`; `cargo check --manifest-path examples/makepad-q2q-camera-shell/Cargo.toml`; docs link check; Makepad manifest check; `git diff --check`; `cargo-makepad android --variant=quest ... build --release` | Passed. The APK build bundled the local Rust shared dependency and produced a signed Quest APK under generated target output. The generated output was not committed. | Run Quest launcher and direct-XR smoke, then record whether `RUSTY_XR_MAKEPAD_STEREO_COMPARISON` appears and whether the fork-state frame-fence patch stays clean in the XR shell |
| S2 | Device liveness gate before marker-route fix | Clean install, runtime permission grants where allowed, 90s launcher capture, and 90s generated-XR activity capture | Partial. Both paths stayed alive/focused and showed no app-process GPU page-fault or fatal lines. Startup markers were absent from the retained 90s logs, and repeated small hardware-buffer warnings appeared. | Instrument marker routing and separate startup marker capture from long fault windows |
| S3 | Android marker route and Makepad bootstrap instrumentation | Rust app markers emitted through Android logcat; Makepad fork emitted public-safe Java activity and native bootstrap phase markers; clean APK string checks confirmed marker strings in Java and native outputs | Passed. The APK contained the Java activity, native bootstrap, and app marker strings. The first 90s samples still missed startup-only markers, which pointed to log retention/noise rather than missing code. | Use short startup captures for marker presence and keep 90s captures for liveness/fault counters |
| S4 | Resolved synthetic stereo device gate | Clean install; short startup capture plus 90s liveness/fault captures for both Makepad launcher and generated-XR activity paths | Passed for step 2. Both launch paths emitted Java activity markers, native bootstrap markers through Vulkan ready / before main loop, and the Rusty XR Q2Q plus stereo-comparison startup markers. Both launch paths stayed alive in 90s windows with no app-process GPU page-fault or fatal lines. Repeated small hardware-buffer warnings remain tracked separately. | Start the Camera2 metadata/acquisition pass, keeping hardware-buffer warnings separate from the GPU page-fault gate |
| S5 | Camera2 metadata/acquisition gate | Source validation, release APK build, clean install, runtime camera permission grants where allowed, short startup marker capture, and 90s liveness/fault capture with hardware-buffer warning counters kept separate | Passed for metadata/acquisition. Both Makepad launcher and generated-XR startup windows emitted Java/native/app markers, enumerated three Camera2 `PRIVATE` sources, selected a back-facing 1280x1280 source with intrinsics and pose metadata, received one hardware-buffer-backed frame, and completed with `status=ok`. The first-frame descriptor reported native format 35, usage 131840, one layer, stride 1280, and a present buffer id. Both 90s liveness windows stayed focused/alive with no app-process GPU page-fault or fatal lines. The small `AHardwareBuffer` 4x4 warning class remains visible: 528 / 929 lines in launcher startup/liveness and 612 / 1586 lines in generated-XR startup/liveness. | Start hardware-buffer import as a separate pass; keep the existing warning class and GPU-fault counters separate |
| S6 | Hardware-buffer import gate | Source validation, release APK build, clean install, short launcher and generated-XR marker captures, and 90s liveness/fault capture with small hardware-buffer warnings counted separately | Passed for single-buffer Makepad import readiness. Both short launch paths emitted Java/native/app markers, Camera2 metadata/acquisition markers, and the new hardware-buffer import marker sequence. Makepad enumerated three camera sources and 66 camera formats, selected a back-facing 1280x1280 YUV420 source, started the delayed `VideoExternal` import path, reported playback prepared at 1280x1280, and emitted `phase=texture-updated status=ok makepadVulkanImport=true`. Both short windows had zero app-process GPU page-fault and fatal lines. Small `AHardwareBuffer` 4x4 warning counts stayed visible: 428 in the launcher import window and 452 in the generated-XR import window. A generated-XR 90s liveness window had zero app-process GPU page-fault and fatal lines while retaining 1339 small warning lines; startup/import markers were expectedly evicted from that longer noisy log window. | Start the stereo projection pass by adding paired-buffer ownership and per-eye projection mapping, or first isolate the repeated 4x4 hardware-buffer warning class if it starts correlating with projection work |
| S7 | Paired import and projection-mapping gate | Source implementation, host validation, Quest APK build, direct generated-XR smoke, and marker-spam guard | Passed for the direct generated-XR path. The final short generated-XR smoke emitted startup, Camera2 metadata/acquisition, paired source enumeration, paired playback start, left/right prepared, left/right texture-updated, projection complete, and paired comparison markers. The selected pair was a back-facing 1280x1280 left/right pair with projection metadata ready; the completion marker reported `pairedLeftRightGpuBuffers=true`, `makepadVulkanImport=true`, `projectionMappingReady=true`, `alignedProjection=true`, `cpuUploadCount=0`, and `visualInspection=required`. App-process GPU page-fault and fatal counts were zero. The separate small `AHardwareBuffer` warning class remained visible. The normal launcher path remains separate after one S7 run hit a Horizon OS display-event-receiver startup failure before app markers. | Start parity performance diagnostics for the direct generated-XR path against the custom Rusty XR stereo camera projection baseline, while keeping launcher lifecycle, awake-state/proximity, and small hardware-buffer warning counters separate |
| S8 | Parity performance diagnostics | Passive pre/post awake-proximity readback, 75s Makepad direct generated-XR sample, custom Rusty XR 0.65 baseline sample, custom Rusty XR 0.75 normalization attempt, and shared scorecard/counter extraction | Partial. Makepad direct-XR held paired/aligned projection markers with zero CPU uploads and zero app-process GPU/fatal signatures while retaining the small `AHardwareBuffer` warning class. The custom 0.65 profile remains a valid warning baseline with paired/aligned projection, zero CPU uploads, zero GPU import failures, about 70.8/72Hz OpenXR cadence, and about 29Hz paired camera progression. The custom 0.75 normalization attempt is invalid: it failed before final projection markers with a display-event-receiver startup failure and reproduced a keep-awake/proximity transition toward standby despite the comparison run avoiding an explicit proximity hold. | Stabilize the custom 0.75 lifecycle/proximity state or add a Makepad 0.65 / 72Hz comparison profile; also add Makepad continuous frame/camera cadence markers before treating scorecard output as directly comparable |
| S9 | Makepad continuous cadence markers | Source implementation; host `cargo fmt`; host `cargo check`; docs/schema/link/boundary checks; APK build; direct generated-XR 25s startup/projection and 60s liveness windows; scorecard parser update | Passed for Makepad marker availability and scorecard consumption. The direct generated-XR run retained the full startup/projection marker chain in the short window and retained five cadence samples in the longer window. Cadence rows reported paired/projection readiness true, `cpuUploadCount=0`, and about 12.5-12.7Hz Makepad `NextFrame` callback / paired camera texture-update rates. App-process GPU page-fault and fatal counts stayed zero; the small hardware-buffer warning class remained visible. The public scorecard parser now extracts Makepad cadence/projection markers. This is still not a normalized parity conclusion because the custom comparison rerun has not been repeated under the new watchdog state. | Run a normalized comparison batch: Makepad direct-XR S9 scorecard plus stable custom-shell baseline with proximity/watchdog state controlled, then decide whether the Makepad 12.5Hz camera texture-update cadence is expected or a bottleneck |
| S10 | Normalized comparison rerun under watchdog control | Same-session Makepad direct-XR 0.75 sample plus custom Rusty XR 0.75 camera projection rerun with passive power-state readback and no extra harness proximity hold | Passed for custom 0.75 lifecycle mitigation, but later reclassified on the Makepad side. Custom 0.75 reached paired/aligned GPU projection with zero CPU uploads, zero GPU import failures, steady OpenXR near 72Hz, and about 49Hz paired camera progression, but retained runtime warning counters. Makepad direct-XR stayed app-fault clean, retained paired/projection readiness and zero CPU uploads, and repeated the about 12.6Hz Makepad `NextFrame` / camera texture-update cadence. S11 launch-state inspection showed the Makepad rows came from a volumetric-window/loading-screen state rather than confirmed immersive presentation, so S10 is app/surface/camera cadence evidence only for Makepad. Both samples preserved the watchdog-backed `CLOSE` / mounted state. | Fix/verify Makepad presentation completion before treating Makepad scorecards as performance-comparable to the custom path |
| S11 | Makepad cadence-source markers and launch-state correction | Source instrumentation, release APK build, clean install, runtime permission repair, generated-XR launch attempts, app marker capture, activity/window/logcat launch-state inspection | Partial. The S11 markers work, but the device run did not fully leave the loading screen. After headset-camera permission was repaired and a stale permission-controller task was cleared, the app process emitted cadence rows with paired/projection flags true and `cpuUploadCount=0`, but Horizon OS still showed a loading state and logged a volumetric-window launch path rather than a confirmed immersive handoff. In that partial state, `NextFrame`, draw-event, and paired texture-update rates clustered around 12.7-13.1Hz while app-level `XrUpdate` stayed zero; app-process GPU page-fault and fatal counts stayed zero; the small hardware-buffer warning class remained visible. S10 Makepad rows are therefore reclassified as app/surface/camera cadence evidence, not presentation-performance parity. | Fix/verify Makepad generated-activity presentation completion before any further performance comparison: pregrant runtime permissions, clear permission-controller residue, compare generated manifest/activity launch semantics with the known-good custom OpenXR path, and require loading-complete/immersive handoff evidence in the scorecard |
| S12 | Quest manifest/activity presentation split | Maintained Makepad fork manifest patch, generated manifest inspection, clean install, direct generated-XR launch-state smoke | Partial. The generated package carried required head-tracking, VR-only/focus-aware metadata, non-resizeable declarations, and native-library extraction. The installed package consumed the non-resizeable attributes, but the device still reported a volumetric-window/loading path and app-level `XrUpdate` stayed at zero. App-process GPU page-fault and fatal counters stayed zero. | Add the Makepad permission/presentation flow back to the Rusty XR example and validate through the launcher/normal activity path |
| S13 | Makepad presentation-flow route correction | Added `XrPermissionsFlow`; updated startup markers; release APK build; direct-XR control smoke; launcher/normal-activity smoke | Partial but decisive. Direct generated-XR launch is no longer the active-presentation route because the permission flow can switch back to the paired normal activity. The launcher path switched into the generated XR activity, reached Makepad OpenXR session creation, and started passthrough, then aborted session setup when environment-depth provider creation failed. App-process GPU page-fault and fatal counters stayed zero; the separate small hardware-buffer warning class stayed visible. | Make environment depth optional/non-fatal in the maintained Makepad fork, then rerun launcher-path validation |
| S14 | Optional environment-depth fork patch and launcher presentation pass | Makepad fork source patch; `rustfmt --check`; `cargo check -p makepad-platform`; `cargo check -p cargo-makepad`; `cargo test -p cargo-makepad`; Rusty XR lock update; host source validation; release APK build; clean install; launcher-path 90s device sample; operator visual confirmation | Passed for active Makepad presentation. The launcher path switched into active XR, showed the synthetic stereo scene in headset, retained 18 cadence markers, and reached about 90Hz Makepad app / `XrUpdate` / draw cadence with about 50Hz paired camera texture progression. Paired/projection markers were true with `cpuUploadCount=0`; app-process GPU page-fault and fatal counters were zero. The runtime still rejected environment-depth provider creation, but the new fork fallback logged it as optional and session creation continued. The invalid `cargo test -p cargo-makepad android::tests --lib` command still fails because `cargo-makepad` has no lib target; the package test target passes. | Start a fresh S15 parity performance comparison using the S14 Makepad launcher path and the known custom Rusty XR stereo camera projection baseline, while preserving passive awake/proximity readback and the separate small hardware-buffer warning counter |
| S15 | Active-presentation comparison and custom visual baseline correction | S14 Makepad scorecard input; custom 0.75 camera-projection rerun with Makepad force-stopped; passive live 30s custom sample; operator visual confirmation; power/proximity readback | Partial. The first custom sample was discarded because a still-running Makepad process polluted the shared log window. The clean custom 0.75 run and live passive sample confirmed proper visible custom stereo projection, `alignedProjection=true`, `pairedLeftRightGpuBuffers=true`, `cpuUploadCount=0`, and zero GPU import failures, but performance was degraded: OpenXR was about 52-53Hz against a 72Hz target, paired camera progression was about 28Hz, and compositor slice-tear counts were high. Power/proximity stayed mounted/awake. Makepad S14 remains active-presentation plus paired import/cadence evidence, but the visible Makepad scene is still synthetic rather than camera projection. | S16: draw the paired Makepad `VideoExternal` textures into visible XR geometry with per-eye source mapping, then rerun a clean Makepad-vs-custom performance comparison |
| S16 | Visible Makepad camera projection marker gate | Persistent XR camera panel implementation; host source validation; release APK build; clean install; launcher-path startup and liveness windows; passive power readback; headset screenshot/operator inspection | Marker-level pass, visual fail. The first placement attempt exposed a widget-tree lookup error, and the second exposed a Makepad script-scope resolution error. After making the panel persistent and referencing it through `mod.widgets.MakepadStereoCameraPanel`, the launcher path retained paired import/projection readiness, emitted `visibleCameraProjectionReady=true`, kept app/`XrUpdate`/draw cadence near 90Hz and paired texture-update cadence near 50Hz, and had zero app-process GPU page-fault and fatal signatures. Follow-up visual inspection still showed the synthetic blue/red debug scene, so `visualReleaseAccepted=false` remains correct. | S17: hide or remove the fallback synthetic scene after camera binding, require a panel draw marker, and verify the visible headset image before performance comparison |
| S17 | Remove fallback synthetic scene | Host validation, release APK build, clean install, launcher-path startup smoke, marker counts, and headset screenshot inspection | Partial. The synthetic blue/red debug scene was removed and the markers stayed true, but the headset view became black aside from runtime overlay. This proved S16's visible geometry was only the fallback scene. | Add a solid-color diagnostic branch to decide whether the camera panel geometry is visible before testing texture sampling |
| S18 | Solid custom-shader panel diagnostic | Host validation, release APK build, clean install, launcher-path startup smoke, marker counts, and headset screenshot inspection | Partial. The custom shader reported a drawn solid diagnostic panel, but headset inspection still showed a black scene. | Move the panel under a scene-owned `XrNode` and make the draw marker require an active XR scene state |
| S19 | Scene-owned panel routing | Host validation, release APK build, clean install, launcher-path startup smoke, marker counts, and headset screenshot inspection | Partial. The panel marker became scene-owned and remained fault-clean, but the custom shader still produced no visible pixels. | Replace the custom shader output with Makepad's known-good `DrawCube` primitive from the same widget transform |
| S20 | Makepad `DrawCube` panel positive control | Host validation, release APK build, clean install, launcher-path startup smoke, marker counts, and headset screenshot inspection | Passed for visible scene-owned panel geometry. A cyan diagnostic panel rendered visibly from the same camera-panel widget transform while paired/projection/cadence markers stayed true and app-process GPU page-fault/fatal counters stayed zero. | S21: fix or replace the custom video-texture shader path, using the visible `DrawCube` panel as the geometry/control baseline |
| S21 | Inherited custom video shader | Host validation, release APK build, clean install, launcher-path startup smoke, marker counts, and headset screenshot inspection | Partial. Both the inherited `DrawCube` shader variant and the `draw_super` convention variant stayed app-fault clean but still produced a black headset view; device logs also showed the attempted XR view-index shader helper was not available in that path. This keeps S20 as the positive scene/transform control and points away from activity/presentation as the cause. | Replace the custom shader route with a Makepad-native video-widget surface, or patch the general Makepad video draw path |
| S22 | Native Makepad `Video` widget surface | Host validation, release APK build, clean install, launcher-path startup smoke, marker counts, and headset screenshot inspection | Partial. The XR view started left/right Makepad `Video` widgets and stayed app-fault clean, but headset inspection remained dark and no confirmed prepared camera playback / paired texture-update sequence appeared. The diagnostic suggests the stock `Video` camera path is not using the headset-camera playback route needed for Quest raw camera sources. | S23: add a small Makepad-side headset-camera permission option for `Video` camera playback, then rerun the launcher visual gate |
| S23 | `Video` headset-camera permission option | Makepad fork patch and push; Rusty XR lock update; host validation; release APK build; clean install; launcher startup/liveness windows; screenshot inspection | Partial but useful. The Makepad `Video` widget now has a general headset-camera permission option and the Rusty XR example consumed the pushed fork revision. Device validation stayed app-fault clean, active XR cadence stayed near 90Hz, and the previous custom-shader compile noise was gone. However the native `Video` widgets were already `Playing` before the camera source was assigned, so `set_camera_permission` / `set_source_camera` were rejected and no prepared/update sequence appeared. Screenshot inspection remained dark. | S24: reset/cleanup the native `Video` widgets before assigning headset-camera sources, then rerun the visual gate |
| S24 | Native `Video` widget reset before source assignment | Source validation, release APK build, clean install, launcher startup/liveness windows, and screenshot inspection | Partial. The reset/retry gate fired and stayed app-fault clean, but the widgets remained in `CleaningUp` because the Android platform cleanup path did not emit `VideoPlaybackResourcesReleased` when no retained platform player or surface existed for that video id. No native video-widget surface start, prepared event, or texture update appeared; screenshot inspection remained dark. | S25: patch the maintained Makepad fork so Android video cleanup completes even without a retained platform resource, then rerun the native widget visual gate |
| S25 | Android video cleanup-completion experiment | Makepad fork patch and push; Rusty XR lock update; host validation; release APK build; clean install; launcher startup/liveness windows; log and screenshot inspection | Rejected. The native `Video` widget surface started after one reset, but no `VideoPlaybackPrepared` or `VideoTextureUpdated` sequence appeared. The old Quest app-process GPU page-fault class returned with premature surface-free lines tied to the app process. The headset view stayed dark. | Revert the cleanup-completion fork patch, disable the native `Video` widget diagnostic, and rerun the launcher path as an app-fault recovery control |
| S26 | Recovery to manual `VideoExternal` import path | Fork revert, Rusty XR lock update, diagnostic disable, host validation, APK rebuild, clean install, launcher smoke, scene-permission grant, liveness window, and screenshot inspection | Passed as a recovery control. With scene access granted, the launcher path focused the generated XR activity, the native `Video` widget diagnostic stayed disabled, paired import/projection markers completed, cadence returned to active-XR rates, app-process GPU page-fault and fatal counters stayed zero, and headset screenshot inspection showed the visible cyan diagnostic panel. | S27: disable the panel's solid diagnostic mode and sample the paired `VideoExternal` camera textures directly |
| S27 | Live `VideoExternal` texture sampling | Source toggle, host validation, APK rebuild, clean install, launcher smoke, liveness window, and screenshot inspection | Partial. The run stayed active-XR and app-fault clean with paired prepared/update markers retained, but headset inspection still showed the cyan/guide panel. Makepad's stock `Video` shader showed the likely cause: the panel double-converted `sample_video()` output as YUV even though the external-texture path already returns display color. | S28: use `sample_video()` color directly, then rerun the launcher visual gate |
| S28 | Direct `sample_video()` color path | Source patch, host validation, APK rebuild, launcher smoke, and screenshot inspection | Partial. The panel now samples `sample_video()` color directly instead of applying a YUV conversion to the returned sample, and the run stayed app-fault clean with paired prepared/update markers. Screenshot inspection still showed the cyan/guide pattern, so double-conversion was not the only visual blocker. | S29: disable the alignment guide overlay and mark the no-guide shader path explicitly |
| S29 | No-debug-overlay visual gate | Source patch, host validation, APK rebuild, in-place install, direct generated-XR control smoke, launcher-path smoke, marker/fault counters, and screenshot inspection | Partial. The forced alignment guide is now disabled by default and markers report `debugAlignmentGuide=false`. The direct generated-XR launch split again into a non-authoritative volumetric/normal-activity state, while the launcher path reached the generated XR activity, retained paired prepared/update and visible-panel markers, and had zero app-process GPU page-fault and fatal counters. Screenshot inspection no longer shows the white guide rectangles but still shows the cyan external-texture placeholder/capture result. | Isolate whether the cyan result is an adb capture limitation/protected camera-content artifact or a custom-shader texture sampling/binding issue; keep no-debug-overlay as the default visual gate |
| S30 | Custom baseline correction to 0.65 performance profile | Stopped foreground-conflicting VR apps, ran the Quest camera-profile harness with `camera-stereo-gpu-composite-performance-065`, `-SkipProximityHold`, a 35s warm-up, screenshot capture, logcat validation, and power-state summary | Passed as the current custom performance baseline, with warning status. The focused custom activity showed visible stereo camera projection, `activeTier=gpu-projected`, `alignedProjection=true`, `pairedLeftRightGpuBuffers=true`, `cpuUploadCount=0`, `gpuImportFailure=0`, no app-process fatal/GPU-fault signatures, `observedOpenXrFps=70.7` against 72Hz, and about 29Hz paired camera progression with zero drops. Warning status came from retained sleep/wake and small compositor slice-tear signals. | Use the 0.65 custom profile, not the default 0.75 profile, for the next fair Makepad-vs-custom performance comparison unless explicitly testing render-scale headroom |
| S31 | Downstream source-provenance target correction | Rebuilt a downstream Vulkan Camera2 target from a clean source-lineage worktree and validated it on-device after an operator camera-permission grant. The run used the generated XR activity route, render scale `0.75`, capture/export disabled, and explicit device performance props `debug.oculus.cpuLevel=4` / `debug.oculus.gpuLevel=4`. | Passed as a target-candidate correction, not as an unattended launcher pass. The headset visual inspection accepted the downstream target's stereo camera projection and performance. Parsed VrApi evidence for the accepted post-grant capture showed `72/72` to `73/72`, `Stale=0`, `SF=0.75`, and `GPU%` about `0.46..0.51` with average `0.4868`; app-process fatal and GPU-fault counts were zero. Small bounded `Tear` counters were present in that post-grant window and should stay visible in future comparisons. | Future Makepad-vs-custom performance comparisons must capture or normalize device CPU/GPU performance levels. The current leading performance hypothesis is GPU saturation: the accepted downstream target sits near 50% GPU usage, while stale/tear-heavy custom samples can approach full GPU utilization. |
| S32 | Controlled level-4 parity batch | Same-session custom Rusty XR profile sample and Makepad launcher-path sample with explicit device CPU/GPU level `4` / `4`, foveation props captured, foreground-conflicting VR apps stopped, startup/liveness logs, screenshots, and shared scorecard parsing. | Reclassified after operator visual review. The custom Rusty XR sample remains valid visible stereo camera projection and reached 72Hz with no app-process fatal/GPU-fault counters, but still sat near GPU saturation. The Makepad sample reached the generated XR activity, retained paired/projection markers, showed 90Hz `VrApi` with low GPU use, zero app-process fatal/GPU-fault counters, and the small hardware-buffer warning class remained bounded. However the visible room image was compositor passthrough, not proof of app-owned custom camera projection, and the app-owned blue/cyan panel was low in the view instead of aligned as the headset projection target. Therefore the Makepad half of S32 is app/fault/cadence evidence only, not projection parity. | S33: make the Makepad visual gate unambiguous by disabling or masking compositor passthrough, drawing an app-owned non-passthrough background, placing the camera panel in an eye-aligned position, and requiring the custom panel content itself to be visibly accepted before any performance comparison. |
| S33 | App-owned visual isolation gate | Source patch, host validation, release APK build, clean install, launcher-path 90s capture, screenshot inspection, and marker/fault counters. | Partial but useful. The corrected run installed cleanly, focused the generated XR activity through the launcher path, reached the split-proof marker path, stayed app-fault clean, and kept CPU/GPU device props at `4` / `4`. The screenshot reclassified the visual state again: the panel is now clearly app-owned and better aligned, but it still renders solid cyan/magenta proof halves rather than camera pixels. This proves the previous failure was not only passthrough ambiguity; the remaining visible-projection blocker is Makepad camera texture sampling/binding. | S34: bind and sample Makepad's Y/U/V camera plane textures directly for a visual proof path. Treat this as a camera-pixel proof, not final zero-copy performance parity, until the Vulkan YUV-plane import path is wired. |
| S34 | YUV-plane camera-pixel proof | Source patch, host validation, release APK build, clean install, launcher-path 90s capture, screenshot inspection, and marker/fault counters. | Partial and reclassified. The app stayed focused and app-fault clean, and right-side CPU YUV texture-update markers streamed repeatedly, but no left-side prepared/update marker completed, no YUV-ready marker reached the app-level handler, and the visible panel still showed cyan/magenta proof colors rather than camera pixels. The operator also identified an unwanted real-world occlusion/depth-clip class in one screenshot: room geometry could cover the app-owned panel. That occlusion is not desired for the camera-streaming proof and is now tracked separately from passthrough visibility and camera-pixel ownership. | S35: keep environment/depth clipping disabled for the proof panel, add raw YUV-ready/prepared/update id markers, and allow a single updating CPU YUV camera stream to bind into both panel halves as a camera-pixel proof before returning to paired stereo ownership. |
| S35 | Depth-clip-off single-stream YUV proof | Source patch, host validation, release APK build, clean install, launcher-path run, direct generated-XR control run, screenshot inspection, and marker/fault counters. | Partial and reclassified. The source-side depth-clip mitigation is still correct, and the unwanted real-world occlusion class was not present in the S35 screenshots. However neither device route produced a valid camera-pixel proof: the generated XR activity briefly appeared, then the app switched back to the normal activity surface. The headset view showed a volumetric environment with a black normal-activity panel, `XrUpdate` stayed at `0`, and no YUV-ready/raw-video/bind markers appeared. The cadence loop still ran and one camera stream updated around 50Hz, with zero app-process fatal/GPU-fault counters and CPU/GPU device levels recorded at `4` / `4`. | S36: fix the Makepad activity handoff so `xr_start_presenting()` cannot bounce from the generated XR activity back to the normal activity. Keep the depth-clip-off rule and rerun the same single-stream YUV proof gate only after active XR presentation is confirmed. |
| S36 | Explicit XR activity handoff | Makepad fork patch, Rusty XR lock update, generated Java/manifest inspection, release APK rebuild, clean install, launcher-path run, direct generated-XR control run, screenshot inspection, and marker/fault counters. | Passed for activity handoff, failed for camera-pixel visual proof. Both routes stayed in active XR presentation with `XrUpdate`/draw cadence near 90Hz, emitted YUV-ready/prepared/update plus single-stream-proof and visible-panel-bound markers, and kept app-process GPU-fault/fatal counters at zero. The unwanted real-world occlusion/depth-clip class remained off. Visual inspection still showed the blue/red app-owned proof panel rather than camera pixels, and cadence showed only the right camera stream updating at about 50Hz while the left stream stayed at zero. The small hardware-buffer warning class remained visible and bounded. | S37: patch the custom panel binding path to update/redraw draw vars in the same style as Makepad's `Video` widget, keep the activity handoff fork state, and rerun the launcher/direct visual proof gate before returning to paired buffer ownership. |
| S37 | Draw-vars camera texture bind proof | Source patch, host validation, release APK build, clean install, launcher-path run, direct generated-XR control run, screenshot inspection, and marker/fault counters. | Failed as a camera-pixel proof while keeping the system stable. The new `draw-vars-bound` marker appeared in both routes and app-process GPU-fault/fatal counters stayed zero, so the app is receiving the event and executing the post-bind path. Visual inspection still showed the blue/red app-owned proof panel, with no depth/environment occlusion. The logs also exposed a bookkeeping issue: both YUV texture handles can be ready while only one side has update events, causing the bind marker to under-report the single-stream fallback. | S38: bind the actually updating YUV stream into both panel halves for the visual proof and remove proof-color tint while camera-ready, so empty texture sampling shows as black and real camera sampling shows as camera pixels. |
| S38 | Updated-stream no-tint YUV proof | Source patch, host validation, release APK build, clean install, launcher-path run, screenshot inspection, and marker/fault counters. | Failed as a camera-pixel proof while staying app-fault clean. The bind path preferred the actually updating camera stream and reported `proofTintStrength=0.0`, but the visible panel still showed blue/red proof colors. That reclassified the blocker away from stream selection alone and toward shader-area state or waiting/default visual state. The unwanted real-world depth/environment occlusion class stayed off, and the small hardware-buffer warning class remained bounded. | S39: force the camera-ready/YUV/proof-tint values onto the active draw area as well as the Rust-side live fields, then rerun the visual gate. |
| S39 | Shader-area state proof | Source patch, host validation, release APK build, clean install, launcher-path run, screenshot inspection, and marker/fault counters. | Failed as a camera-pixel proof while further narrowing the fault. The `shaderAreaStateUpdate=true` marker appeared and counters stayed stable, but the visible panel still showed blue/red proof colors. This made the remaining ambiguity the panel's waiting/default color path versus actual texture sampling. | S40: make the waiting/default state neutral black, set the panel default to camera-ready, keep depth clipping off, and rerun both launcher and direct generated-XR routes. |
| S40 | Neutral-wait texture-content proof | Source patch, host validation, release APK build, clean install, launcher-path run, direct generated-XR control run, screenshot inspection, and marker/fault counters. | Decisive partial. Both routes stayed in active XR, emitted the same YUV-ready/prepared/update, draw-vars-bound, shader-area-state, single-stream-proof, and visible-panel markers, kept app-process GPU-fault/fatal counters at zero, and kept the small hardware-buffer warning class bounded. The app-owned panel changed from blue/red to neutral black with the expected guide border, proving the shader edits and active draw state are taking effect. The unwanted real-world depth/environment occlusion class remained off. Camera pixels still did not appear, so the remaining blocker is texture sampling/content in the Makepad YUV texture path, not passthrough ambiguity, depth occlusion, activity handoff, or stale proof-color state. | S41: inspect the Makepad `Video` YUV texture path against the custom panel sampler, then add a narrow texture-content proof that distinguishes empty/black texture content from wrong sampler/format binding before resuming paired left/right projection parity. |
| S41 | Y-plane texture-content proof | Source patch, host validation, release APK build, clean install, permission-state correction, launcher-path active-XR rerun, screenshot inspection, and marker/fault counters. | Partial. The first rerun was invalid for visual judgment because scene access was not restored and app-level `XrUpdate` cadence stayed at zero. The corrected launcher-path rerun restored scene/camera grants, reached active OpenXR presentation, emitted nonzero app/`XrUpdate`/draw cadence near runtime rate, and reported bounded texture-content probes with nonzero CPU-side Y/U/V plane content. App-process GPU-fault/fatal counters stayed zero and the small hardware-buffer warning class remained visible. The panel still rendered black inside the guide, so CPU-visible camera content is present but the custom panel sampler has not visually proven it. | S42: boost Y-plane luma in the shader so a dark-room camera frame cannot be mistaken for a zero GPU sample. |
| S42 | Gain-boosted Y-plane visual proof | Source patch, host validation, release APK build, clean install, active-XR launcher run, screenshot inspection, and marker/fault counters. | Failed as a camera-pixel proof while staying stable. The run retained active OpenXR cadence, texture-content probes, nonzero CPU-side plane content, and no app-process GPU-fault/fatal lines. The visible panel stayed black inside the guide even with gain-boosted luma sampling, making normal dark exposure an unlikely explanation. | S43: bind a generated non-camera R8 texture into the Y-plane slot as a positive sampler/slot control. |
| S43 | Synthetic R8 Y-slot control | Source patch, host validation, release APK build, clean install, active-XR launcher run, screenshot inspection, and marker/fault counters. | Failed as a sampler-slot proof while staying stable. The generated R8 control texture was created and bound into the camera-panel Y-plane path, markers stayed active, CPU-side camera content remained nonzero, and app-process GPU-fault/fatal counters stayed zero. The panel still rendered black inside the guide. | S44: bind the generated R8 control texture across all panel texture slots to distinguish a named-slot mismatch from a broader texture sampling/panel path issue. |
| S44 | Synthetic all-slot texture control | Source patch, host validation, release APK build, clean install, active-XR launcher run, screenshot inspection, and marker/fault counters. | Failed as an all-slot texture proof while staying stable. Binding the same generated R8 control across the panel texture slots did not change the visible black panel. Active OpenXR cadence, startup markers, visible-panel markers, and fault counters remained healthy, and the small hardware-buffer warning class stayed visible. | S45: bypass texture sampling entirely with a marker-selected constant shader color to decide whether the visible panel is executing that shader branch. |
| S45 | Constant shader bypass control | Source patch, host validation, release APK build, clean install, active-XR launcher run, screenshot inspection, and marker/fault counters. | Partial and decisive for the next split. The marker path reported the constant-bypass branch and the app remained active-XR, app-fault clean, and camera-texture-updated. Visual inspection still showed the same black panel inside the guide instead of the expected constant color. This moves the immediate blocker from camera content or texture slots to live shader-branch / visible-panel path verification. | S46: make the panel fragment output unconditional and visually obvious, then rerun. If that still stays black, trace the visible draw path instead of adding more texture diagnostics. |
| S46 | Unconditional shader-output control | Source patch, host validation, clean release APK rebuild with APK string check, clean install, active-XR launcher run, screenshot inspection, and marker/fault counters. | Passed as a shader-path control. The rebuilt APK contained the S46 marker strings and no S45 constant-bypass strings. The launcher path retained active OpenXR cadence, camera texture-update markers, zero app-process GPU-fault/fatal counters, and the separate small hardware-buffer warning class. The app-owned panel turned green with the expected guide overlay, proving the visible panel executes the edited shader body when the shader returns before texture sampling. S45 was therefore not a true texture-free bypass because texture sampling occurred before its branch. | S47: sample only a generated R8 texture in the shader, with no `sample_video()` or YUV pre-sampling, to isolate texture-slot binding from camera data and external-video sampling. |
| S47 | Direct generated-R8 texture sample | Source patch, host validation, clean release APK rebuild with APK string check, clean install, active-XR launcher run, screenshot inspection, and marker/fault counters. | Passed as a texture-slot control. The launcher path retained active OpenXR cadence, camera update markers, zero app-process GPU-fault/fatal counters, and the separate small hardware-buffer warning class. The panel visibly sampled the generated R8 texture as a checker pattern through `left_tex_y`, proving that the scene-owned shader can sample a normal `texture_2d(float)` slot when the shader returns before external-video and YUV pre-sampling. | S48: disable the generated replacement and sample only the real Makepad camera Y plane through the same direct shader path. |
| S48 | Direct camera Y-plane texture sample | Source patch, host validation, clean release APK rebuild with APK string check, clean install after ADB returned, active-XR launcher run, screenshot inspection, and marker/fault counters. | Passed as a real camera-plane proof. The launcher path retained active OpenXR cadence around runtime rate, direct camera-Y markers, CPU-visible Y/U/V content, zero app-process GPU-fault/fatal counters, and the separate small hardware-buffer warning class. The screenshot showed the app-owned panel sampling real camera content through `left_tex_y`; the panel was mostly saturated because the S42-era `*8` luma gain was still active. | S49: remove the diagnostic luma gain and rerun the same direct camera-Y sample before moving to YUV color conversion. |
| S49 | No-gain direct camera Y-plane texture sample | Source patch, host validation, clean release APK rebuild with APK string check, clean install, active-XR launcher run, screenshot inspection, and marker/fault counters. | Passed as a visible monochrome camera proof. The launcher path stayed active-XR and app-fault clean, retained camera update/cadence markers, and emitted the no-gain marker path only. The screenshot showed a real camera image in the app-owned panel without the S48 saturation. Operator visual review confirmed camera feed visibility and reclassified the immediate gap to upside-down orientation plus low/off-center panel placement. The visual is still monochrome and not yet paired/projection-parity aligned. | S50: rotate the direct camera-Y sample 180 degrees and move the app-owned diagnostic panel to the head-forward inspection position before color conversion. |
| S50 | Rotated and centered direct camera Y-plane diagnostic | Source patch, host validation, clean release APK rebuild with native-library string check, clean install, active-XR launcher run, screenshot inspection, passive power/proximity readback, and marker/fault counters. | Passed for upside-down correction and diagnostic placement, with a new handedness gap. The first retry moved the panel the wrong way and left it clipped at the bottom, so S50b kept the 180-degree sample rotation and moved the panel upward. The launcher path focused the generated XR activity, retained the S50 marker path with no stale no-gain markers, stayed awake/mounted with CPU/GPU props recorded, kept app-process GPU-fault/fatal counters at zero, and kept the small hardware-buffer warning class visible. Screenshot inspection and operator review showed a usable head-forward app-owned panel with upright monochrome camera content, but the image is still left/right mirrored relative to the camera feed. This is not final per-eye projection parity; it is a camera-pixel diagnostic position. | S51: keep the centered panel, replace the 180-degree rotation with a vertical-only flip, and rerun the visual gate before direct YUV color conversion. |
| S51 | Upright unmirrored direct camera Y-plane diagnostic | Source patch, host validation, clean release APK rebuild with native-library string check, clean install, active-XR launcher run, screenshot inspection, passive power/proximity readback, and marker/fault counters. | Passed as the mirror-correction candidate. The shader now samples direct camera-Y with a vertical-only flip, keeping the S50b centered panel placement while removing the horizontal mirror introduced by the 180-degree rotation. The launcher path focused the generated XR activity, emitted the upright/unmirrored marker path with no stale S50/S49 markers, stayed awake/mounted with CPU/GPU props recorded, kept app-process GPU-fault/fatal counters at zero, and kept the small hardware-buffer warning class visible. Screenshot inspection shows the panel upright, centered, and horizontally corrected; if live headset observation disagrees, rerun the handedness gate before color conversion. | S52: combine the real Y/U/V planes into direct color in the same early-return shader path, then resume paired left/right ownership and projection mapping. |
| S52 | Direct camera YUV-to-RGB color diagnostic | Source patch, host validation, clean release APK rebuild with native-library string check, clean install, active-XR launcher run, screenshot inspection, S52 log review, and Rusty XR YUV/color doc review. | Partial and diagnostic. The app stayed active-XR and app-fault clean, markers showed no stale monochrome path, and CPU texture probes reported non-empty Y/U/V planes with planar YUV metadata. The headset view was strongly green/cyan, matching the documented class where a green shader decode should be treated as sampler/decode-shape evidence, not final color calibration. | S53: render Y, U, and V texture slots directly as separate grayscale bands in the same centered/upright panel to prove whether GPU chroma-slot sampling matches the CPU plane probes. |
| S53 | Direct Y/U/V plane-slot visibility proof | Source patch, host validation, release APK rebuild with native-library string check, clean install, active-XR launcher run, screenshot inspection, passive power/proximity readback, and marker/fault counters. | Passed as a chroma-slot proof. The APK contained S53 markers and no stale S52 marker strings. The launcher path resumed the generated XR activity, emitted S53 marker rows, kept app-process GPU-fault/fatal counters at zero, kept the small hardware-buffer warning class bounded, and stayed awake/mounted. The screenshot showed the panel split into live Y, U, and V grayscale bands, proving that GPU texture slots for all three planes are visible. | S54: keep the same Y plane and panel transform, swap U/V in the YUV-to-RGB conversion, and rerun the visual gate before range/matrix tuning. |
| S54 | Direct camera YUV color with U/V swap | Source patch, host validation, clean release APK rebuild with native-library string check, clean install, active-XR launcher run, screenshot inspection, passive power/proximity readback, and marker/fault counters. | Passed as the active plane-order fix, with calibration still open. The APK contained S54 markers and no stale S52/S53 strings. The launcher path resumed the generated XR activity for 90 seconds, emitted S54 projection/color markers, kept app-process GPU-fault/fatal counters at zero, kept the small hardware-buffer warning class bounded, and stayed awake/mounted. The screenshot showed a real color camera feed in the app-owned panel instead of green/cyan. The feed remains visibly washed/warm relative to the Rusty XR hardware-buffer sampler baseline. | S55: keep the U/V swap and render a four-way limited/full range plus BT.601/BT.709 color grid in the same panel, then select the closest Makepad CPU-YUV formula before paired/per-eye projection parity. |
| S55 | Four-way YUV color grid | Source patch, host validation, release APK rebuild with native-library string check, clean install, active-XR launcher run, screenshot inspection, and marker/fault counters. | Stable but inconclusive as a calibration gate. The APK contained S55 markers and no stale S52/S53/S54 strings. The launcher path resumed the generated XR activity, emitted color-grid markers, kept app-process GPU-fault/fatal counters at zero, and kept the small hardware-buffer warning class bounded. The screenshot showed the grid and live camera feed, but each quadrant sampled a different source region, so visible differences were confounded by image content. | S56: keep the same four formulas but remap each quadrant to the same full camera view, then rerun the visual gate. |
| S56 | Four-way YUV color grid with quadrant-local UV remap | Source patch, host validation, release APK rebuild with native-library string check, clean install, active-XR launcher run, screenshot inspection, passive power/proximity readback, and marker/fault counters. | Passed as a fair calibration grid, with acceptance deferred to a single-formula run. The APK contained S56 remap markers and no stale S52-S55 strings. The launcher path resumed the generated XR activity for 90 seconds, emitted remap-grid markers, kept app-process GPU-fault/fatal counters at zero, kept the small hardware-buffer warning class bounded, and stayed awake with proximity negative/stay-on true. The screenshot showed each quadrant using the same full camera view. Together with the Rusty XR hardware-buffer logs reporting suggested `YCBCR_601` / narrow range, this selects swapped-U/V limited BT.601 as the next single-panel candidate. | S57: collapse the grid into the accepted swapped-U/V limited-BT.601 full-panel color path and rerun the visual gate before paired/per-eye projection parity. |
| S57 | Accepted swapped-U/V limited-BT.601 full-panel color | Source patch, host validation, release APK rebuild with native-library string check, clean install, active-XR launcher run, screenshot inspection, and marker/fault counters. | Passed as the single-panel color acceptance candidate. The APK contained S57 markers and no stale S52-S56 strings. The launcher path resumed the generated XR activity for 90 seconds, emitted accepted-color markers, kept app-process GPU-fault/fatal counters at zero, and kept the small hardware-buffer warning class bounded. The screenshot showed the real color camera feed across the full app-owned panel. The diagnostic cross is now the obvious remaining visual artifact. | S58: remove the center/split guide while keeping a border-only overlay, then rerun the visual gate before paired/per-eye projection mapping. |
| S58 | Border-only swapped-U/V color cleanup | Source patch, host validation, release APK rebuild with native-library string check, clean install, active-XR launcher run, screenshot inspection, passive power/proximity readback, and marker/fault counters. | Mixed result. The marker and screenshot evidence proved the shader edits were active and the visible guide was reduced to a border-only overlay. The app stayed active-XR, app-fault clean, and the small hardware-buffer warning class was zero in the sample. Visual review rejected the color claim: skin tones in the panel were visibly blue/cyan relative to the camera view. This reclassifies S57/S58 as useful color candidates, not accepted color parity. | S59: keep the same border-only placement and vertical flip, but use Android YUV_420_888 no-swap U/V sampling with limited BT.601 and mark color acceptance false until visual review passes. |
| S59 | No-swap limited-BT.601 border-only color control | Source patch, marker update, release APK build, native-library string check, clean install, permission repair, 90s launcher visual/counter gate, and six-frame screenshot freshness sequence. | Passed as the current Makepad CPU-YUV color control, not final parity. The APK contained the S59 no-swap marker path and no stale S58 swap/accepted-color strings. The first clean install stopped on the headset-camera permission flow, which confirmed the launcher needs the declared `horizonos.permission.HEADSET_CAMERA` runtime grant in addition to ordinary Android camera permission for unattended runs. After permission repair, the launcher path resumed active XR, emitted no-swap conversion markers, retained cadence and texture-content probes, kept app-process GPU-fault/fatal and small hardware-buffer counters at zero, and showed live camera feed with materially better skin-tone class than S58. Six screenshots captured at one-second intervals had six unique SHA-256 hashes, so the visual evidence was not a byte-identical freeze sequence. | S60: keep S59 color, select left/right YUV textures by active eye instead of sampling the left texture for both eyes, then rerun the same launcher/counter/freshness gate. |
| S60 | Per-eye Makepad YUV texture selection | Makepad fork shader-builtin patch, Rusty XR lock update, source validation, APK build, clean install, direct generated-XR smoke, marker/fault counters, and screenshot review. | Passed for shader-side eye selection, with app logic still incomplete. The fork exposed Makepad's XR multiview index through `xr_view_id()`, and the shader selected left/right YUV texture sets by active eye. The run stayed active-XR and app-fault clean, but the app still treated the first single stream as terminal, so paired-source completion was not yet reliable. | S61: keep the per-eye shader selector and make single-stream proof a waiting state until both left/right camera streams have updated. |
| S61 | Paired-stream completion after single-stream proof | Source patch, host validation, release APK build, clean install, direct generated-XR smoke, marker/fault counters, and screenshot review. | Partial. The app no longer terminates on the first single-stream visual proof and waits for paired left/right updates. The remaining blocker was Makepad `XrEnv` starting its own headset-camera path even with the environment cube visually disabled, which could compete with the app-owned camera acquisition path. | S62: gate the `XrEnv` headset-camera sync on `env_cube` so the diagnostic app owns the raw-camera streams during camera-panel validation. |
| S62 | Gate Makepad `XrEnv` camera when `env_cube=false` | Makepad fork patch and push, Rusty XR lock update, source validation, release APK build, clean install, direct generated-XR smoke, marker/fault counters, and six-frame screenshot freshness sequence. | Passed for paired source ownership and freshness. The run reached paired left/right camera frames, kept per-eye texture markers, kept app-process GPU-fault/fatal/small-buffer counters at zero, and captured six byte-distinct screenshots. Operator review showed live coherent camera content, but the panel remained a world-space surface that stayed parked in the room as the headset moved. | S63: preserve the paired YUV path and try an active-eye head-locked placement correction. |
| S63 | Active-eye view-space head-locked placement | Source patch, host validation, release APK build, clean install, direct generated-XR smoke, marker/fault counters, and six-frame screenshot freshness sequence. | Reclassified. The run stayed clean and the screenshot sequence was fresh, but operator inspection showed the app-owned panel was absent; the apparent full-view image was native passthrough, not a camera panel. This moves the blocker from camera content to panel geometry/layer isolation. | S64: run a more direct clip-space head-locked positive control to decide whether the attempted head-locked vertex path is being clipped or hidden. |
| S64 | Clip-space head-locked panel positive control | Source patch, host validation, release APK build, clean install, direct generated-XR smoke, partial artifact capture, and screenshot freshness check. | Reclassified as a visibility failure. The APK contained the S64 marker path, the generated XR activity launched, and six screenshots were byte-distinct, but the app-owned camera panel and border were still absent. Native passthrough and the system performance overlay remained visible, making screenshots ambiguous unless the app panel is isolated. | S65: disable Makepad's native OpenXR passthrough composition layer in the maintained fork, restore the known-visible world-space panel for the isolation gate, and draw a thick black border so app-panel presence is visually unambiguous. |
| S65 | Passthrough-off black-border panel visibility control | Makepad fork patch and push, Rusty XR lock update, source validation, release APK build, clean install, permission repair, direct generated-XR and launcher-path smokes, marker/fault counters, and screenshot freshness checks. | Partial and reclassified. The APK contained the S65 passthrough-off and black-border marker strings. Direct component starts and the first launcher path stayed in loading/preflight because Scene Access had not been granted after clean install, so `XrUpdate=0` and those screenshots are not app-panel evidence. After granting both Scene Access and Headset Camera, the launcher path switched into the generated XR activity, emitted the native-passthrough-disabled log, produced nonzero XR update cadence, and kept app-process GPU-fault/fatal counters at zero. However the headset view remained a black app background with no visible app-owned panel or border. | S66: restore the last known visible world-space panel pose, use a non-black app clear color, and render a solid neutral app-owned panel with a thick black border before returning to camera sampling. |
| S66 | Solid black-border panel visibility control | Source patch, host validation, release APK build, native-library string check, clean install, full permission grants, launcher-path XR smoke, marker/fault counters, and screenshot freshness checks. | Failed as a panel-visibility proof and reclassified as projection-layer output evidence. The APK contained the S66 solid-panel, black-border, and native-passthrough-off marker strings. With both Scene Access and Headset Camera granted, the launcher path reached the generated XR activity, emitted the native-passthrough-disabled log, produced nonzero XR update cadence, and kept app-process GPU-fault/fatal counters at zero while the small hardware-buffer warning class remained visible. However headset output still showed no non-black clear color, solid panel, or black border. Loading/preflight states from earlier attempts are explicitly not counted as passthrough-off success. | S67: isolate Makepad/OpenXR projection-layer output by comparing the same solid app panel with native passthrough on/off and inspecting `xrEndFrame`, environment blend mode, submitted layer state, and clear-color presentation before returning to camera sampling. |
| S67a | Passthrough-on known-visible panel target | Source patch, release APK build, clean install, launcher-path XR smoke, marker/fault counters, and screenshot review. | Failed visually after ADB transport recovery. The corrected S67a run reached the generated XR activity, emitted startup/projection/end-frame markers, reported `xrEndFrame` success with native passthrough enabled, kept app-process GPU-fault/fatal counters at zero, and kept the small hardware-buffer warning class visible. Operator review still showed no app-owned panel or border. S62 artifact archaeology then found the S67 restore was still not exact: the visible S62 build used the plain world-space vertex path, `camera_projection_scene` at `pos.y=0.92`, dark clear color `#x10171f`, direct per-eye YUV sampling, and a thin pale border. | S67a2: restore the S62-visible source traits exactly while keeping the maintained fork revision and passthrough-on positive-control scope. Do not proceed to passthrough-off or per-eye non-world-space placement until the S62-style panel is visible again. |
| S67a2 | S62 world-panel recovery positive control | Source patch, host validation, release APK build, native-library string check, clean install, permission grants, launcher-path and direct generated-XR visual/freshness samples, marker/fault counters, and screenshot review. | Passed as the restored S62-style visible-panel control. Both launcher and direct-XR runs reached active XR, emitted projection/comparison/end-frame markers, reported native passthrough enabled, retained the S62 recovery marker path, kept stale head-locked and previous-visible marker counts at zero, kept app-process and global GPU-fault/fatal/small-hardware-buffer counters at zero, and captured six byte-distinct screenshots per path. Visual review showed the app-owned live camera panel and thin pale border visible again, with the expected world-space anchoring. | S67b: change only the native passthrough/background isolation setting from this known-visible source state. Disable passthrough and pass only when the headset shows a solid app-owned background plus the visible app panel. |
| S67b | Passthrough-off solid background plus panel | Source patch, host validation, release APK build, native-library string check, clean install, permission grants, launcher-path and direct generated-XR visual/freshness samples, marker/fault counters, and screenshot review. | Qualified pass for the ordered passthrough-off milestone. Both launcher and direct-XR paths reached active XR, emitted S67b markers with `nativePassthrough=false`, reported OpenXR end-frame success with `environmentBlend=OPAQUE`, kept stale S67a2/S62 path markers at zero, and captured six byte-distinct screenshots per path. Visual review showed no Meta passthrough view: the app-owned live camera panel and pale border remained visible against a solid black background. The requested non-black clear color marker was present but did not appear visually, so clear-color fidelity remains open. The small 4x4 hardware-buffer warning class remained visible, while app/global GPU-fault and fatal counters stayed at zero. Environment-depth runtime logs were still present, but no depth-occlusion masking was visible in the screenshots. | S68: start from this passthrough-off visible-panel state and move the panel into per-eye non-world-space camera/head placement. Preserve screenshot freshness, explicit passthrough-off/end-frame evidence, the small hardware-buffer warning counter, and the depth/environment log watch. |
| S68 | Per-eye non-world-space panel placement | Source patch, host validation, release APK build, native-library string check, clean install, permission grants, launcher-path and direct generated-XR visual/freshness samples, marker/fault counters, screenshot review, passive motion-review capture, and operator headset-motion inspection. | Passed as a per-eye camera/head-space panel projection gate, not full stereo parity. The source started from the S67b passthrough-off visible-panel state, kept direct per-eye no-swap limited-BT.601 YUV sampling and the pale border, and replaced the S62 world-space vertex transform with active-eye `draw_pass.camera_inv` basis placement. Both launch paths reached active XR, emitted S68 placement markers with `nativePassthrough=false`, kept stale S67b/S62/head-locked marker counts at zero, captured six byte-distinct screenshots each, and visually showed the app-owned live camera panel plus pale border. App/global GPU-fault and fatal counters stayed at zero. The known small 4x4 hardware-buffer warning class and environment-depth runtime logs remained visible without visible depth occlusion. Operator inspection confirmed clean panels projected per eye. The remaining visual defects are that the camera feeds are swapped left/right between eyes and the panel alignment is not yet good enough for a proper stereo effect. | S69: fix only the left/right source-eye mapping while keeping S68 placement, passthrough-off state, color path, freshness checks, and fault counters stable. Then S70 should use the custom Rusty XR target alignment notes to tune stereo panel/projection alignment before any performance comparison. |
| S69 | Left/right source-eye mapping correction | Source patch, host validation, fresh release APK build, native-library string check, launcher-path sample, six-frame freshness sequence, marker/fault counters, screenshot review, and operator headset inspection. | Partial. The source kept the S68 per-eye `camera_inv` placement, S59 no-swap limited-BT.601 color path, and vertical-only UV orientation, then changed only the shader source-eye selector to `inverted_xr_view_id`. The APK hash was fresh and the extracted native library contained S69 markers with stale S68 path markers absent. The automated launcher sample eventually reached active XR with six byte-distinct screenshots, zero app/global GPU-fault and fatal counters, and paired cadence near display rate. Operator inspection added two important corrections: the first visible launch attempt can still remain stuck in loading and must be recorded/retried by the workflow, and the S69 source-eye swap made stereo eye alignment coherent but the image remains horizontally mirrored. | S69b: keep the S69 source-eye mapping and add a horizontal UV flip, then validate through the guarded launch harness before attempting projection alignment. |
| S69b | Horizontal mirror correction after source-eye swap | Source patch, standalone source validation, fresh release APK build from the documented Makepad lane, native-library string check, guarded device-gate launcher run, six-frame freshness sequence, marker/fault counters, and screenshot review. | Automated pass, operator acceptance pending. The fresh APK hash was recorded and `libmakepad.so` contained S69b marker strings while stale S69/S68 path labels were absent. The guarded harness reached active XR on `launcher-attempt-1` without needing launcher retry or direct-XR fallback, captured six byte-distinct screenshots, emitted 30 visible-panel markers by the final sample, and kept app/global GPU-fault and fatal counters at zero. The small hardware-buffer warning class remained visible. Screenshot review shows full per-eye live camera panels with pale borders and passthrough off. Operator follow-up clarified that the horizontal flip should be retained; the remaining defects are overlap alignment and a horizontal stretch where the panel is too wide. The run also exposed and fixed one workflow issue: the actual generated APK path is `target/android/makepad-android-apk/`, not the older `target/makepad-android-apk/` path. | S70 should retain the S69b flip and tune only the panel/projection geometry. |
| S70 | Head-centered square-aspect panel alignment | Source patch, standalone source validation, fresh release APK build, native-library string check, guarded device-gate launcher run, six-frame freshness sequence, marker/fault counters, and operator headset inspection. | Partial. The S69 source-eye mapping and S69b horizontal texture flip were retained. The panel geometry moved from the earlier wide per-eye panel to a shared head-centered plane at about `0.75m`, square `1280x1280` source aspect, `60deg` preview FOV, and `1.06` raw overscan, giving an approximate `0.92m x 0.92m` content surface. The automated gate reached active XR and produced fresh camera frames without app/global GPU-fault or fatal counters. Operator inspection accepted the aspect-ratio correction: real-world geometry is no longer horizontally stretched. The remaining blocker is depth-dependent stereo mismatch, strongest at close range and nearly imperceptible around `1m`, which suggests the shared Makepad quad is adding a physical convergence plane on top of the camera-pair disparity. | S71 should keep the S70 square aspect and S69b mirror/source-eye mapping, but return placement to active-eye camera-inverse space to remove shared-plane parallax before attempting full metadata-backed camera projection. |
| S71 | Active-eye square-aspect panel parallax isolation | Source patch, standalone source validation, fresh release APK build, native-library string check, guarded device-gate launcher run, six-frame freshness sequence, marker/fault counters, screenshot review, gate-script readiness fix, and operator headset inspection. | Failed visually, useful negative split. S71 kept the accepted S70 `0.92m x 0.92m` square aspect and the required S69b horizontal UV flip plus source-eye inversion. It changed only panel placement from shared head-centered geometry back to the active display-eye `camera_inv` basis, with markers `s71EyeCenteredPanel=true` and `s71SharedPlaneParallaxRemoved=true`. The fresh native library contained S71 markers, retained S70 square-aspect and S69b mirror markers, and had zero stale S70/S69/S68 path labels. The guarded launcher path reached active XR on `launcher-attempt-1`, captured six byte-distinct screenshots, emitted S71 markers, and kept app/global GPU-fault and fatal counters at zero while the small hardware-buffer warning class remained visible. Screenshot review showed live app-owned per-eye panels with the pale border and no native passthrough fill, but operator inspection reported the close-range stereo mismatch was slightly worse than S70. The harness summary now preserves the first ready launch attempt instead of letting the final sampling window override readiness when early OpenXR end-frame markers have scrolled out. | S72 should revert to the S70 head-centered square visual basis and diff directly against the Rusty XR custom projection path. The likely missing Makepad piece is not panel placement, but source-camera intrinsics/pose or a display-eye screen-to-camera homography applied to the sampled UVs. |
| S72 | Metadata-backed surface-to-camera UV projection | Source patch, standalone source validation, fresh release APK build, native-library string check, guarded launcher device gate, six-frame freshness sequence, marker/fault counters, screenshot review, and operator headset inspection. | Failed visually, useful negative split. S72 explicitly reversed the failed S71 panel-placement direction by returning to the S70 head-centered square visual basis, keeping the S69/S69b source-eye and texture orientation decisions, and moving the remaining correction into sampled UVs. The implementation depends on the public camera-model crate and computes per-source `surface_to_camera_uv_homography` rows from Camera2 intrinsics, lens pose, stereo reference center, and the same `60deg` / `0.75m` / `1.06` / square preview-surface constants used by the target path. The guarded launcher path reached active XR on `launcher-attempt-1`, emitted `projectionHomographyReady=true`, captured six byte-distinct screenshots, and kept app/global GPU-fault and fatal counters at zero while the small hardware-buffer warning class stayed visible. Visual result regressed to a mostly uniform camera-tinted panel with pale border even though CPU YUV probes still showed real nonzero camera content. | S73 should keep the same camera-model rows but move the Makepad shader handoff from row vector fields to scalar coefficients, because the existing successful YUV path already uses scalar draw vars. |
| S73 | Scalar-bound homography handoff | Source patch, standalone source validation, fresh release APK build, native-library string check, guarded launcher device gate, six-frame freshness sequence, marker/fault counters, screenshot review, and operator headset inspection. | Failed visually, narrowed the fault. S73 replaced the shader handoff for the homography from row-vector fields with scalar coefficients while leaving the S72 camera-model rows, source-eye mapping, texture flip, and head-centered square visual basis intact. The guarded launcher path again reached active XR on `launcher-attempt-1`, emitted scalar-binding and homography-ready markers, captured six byte-distinct screenshots, and preserved zero app/global GPU-fault and fatal counters with the small hardware-buffer warning class still tracked separately. Operator inspection reported a flashing monocolored surface whose color changed slightly with headset view. That means live camera content reaches the shader, but the effective projected UV field is collapsing or clamping to a tiny sample region instead of covering the full image. | S74 should hard-code the logged S72/S73 homography rows in the shader while retaining the same math and texture path. If S74 restores a spatial camera image, the issue is Makepad draw-var binding. If it stays monocolor, the issue is in the surface/screen projection UV chain or texture transform relative to the CPU-YUV sampling path. |
| S74 | Literal-row homography binding isolation | Source patch, standalone source validation, fresh release APK build, native-library string check, guarded launcher device gate, six-frame freshness sequence, marker/fault counters, screenshot review, and operator headset inspection. | Passed as a diagnostic split, not a final solution. S74 kept the S72/S73 camera-model projection intent and YUV sampling path but hard-coded the logged left/right homography rows inside the shader so dynamic Makepad draw-var binding was removed from the test. The APK contained S74 markers and no stale S73/S71/S70 path labels. The guarded launcher path reached active XR on `launcher-attempt-1`, emitted S74 and homography-ready markers, captured six byte-distinct screenshots, and kept app/global GPU-fault and fatal counters at zero while the small hardware-buffer warning class stayed visible. Operator inspection reported that S74 restored the camera feed with the known parallax issue. This proves the row values and homography math are usable, and the S72/S73 collapse came from the dynamic Makepad field handoff. A brief refreshed target run of the public direct in-app fast profile showed `activeTier=gpu-projected`, `alignedProjection=true`, `cpuUploadCount=0`, 72 FPS app-process VrApi lines, and no app GPU-fault/fatal signatures. | S75 should keep the S74 proven dynamic row values but deliver them through Makepad's pre-draw dynamic instance/uniform APIs rather than only patching the existing area. If S75 keeps the restored feed, continue into alignment/parallax parity against the public target. |
| S75 | Pre-draw dynamic homography binding | Source patch, standalone source validation, fresh release APK build, native-library string check, guarded launcher device gate, six-frame freshness sequence, marker/fault counters, screenshot review, and refreshed public target run. | Failed visually, useful negative split. S75 reverted from literal shader rows back to dynamic metadata rows and wrote every coefficient through `set_dyn_instance`, `set_uniform`, `set_instance_on_area`, and `set_uniform_on_area`. The guarded launcher path reached active XR on `launcher-attempt-1`, emitted S75 dynamic-binding and homography-ready markers, captured six byte-distinct screenshots, and kept app/global GPU-fault and fatal counters at zero while the small hardware-buffer warning class stayed visible. Screenshot review showed the pale-bordered app panel but no spatial camera image, while CPU YUV probes and metadata rows were still live. The refreshed public direct fast target again reported projected stereo, `cpuUploadCount=0`, and app-process VrApi 72/72 to 73/72 lines with `Tear=0` and `Stale=0`; global stale lines came from another runtime process and are not app-process target evidence. S75 therefore points below area/uniform patch timing: the panel still draws through nested `DrawCube` draw-vars, so its instance slice does not reliably include the wrapper's custom homography fields. | S76 should replace the nested `DrawCube` deref with a direct draw-vars panel shader so the live custom fields are contiguous with the draw-vars instance layout. If that restores the S74 camera feed dynamically, continue into the remaining parallax/alignment parity work against the public target. |
| S76 | Direct draw-vars homography panel | Source patch, standalone source validation, fresh release APK build, native-library string check, guarded launcher device gate, six-frame freshness sequence, marker/fault counters, screenshot review, and Rusty XR target diff. | Passed for the dynamic binding blocker. S76 moved the camera panel off the nested `DrawCube` instance root and gave it its own direct `DrawVars` plus cube geometry declarations. The fresh APK contained S76 marker strings and no stale S75 true/path markers. The guarded launcher path reached active XR on `launcher-attempt-1`, emitted S76 and homography-ready markers, captured six byte-distinct screenshots, and kept app/global GPU-fault and fatal counters at zero. Visual inspection showed live camera pixels through the dynamic metadata rows again, proving the S72/S73/S75 collapse was the nested draw-vars instance layout rather than Camera2 acquisition or projection math. App-process VrApi rows stayed at 90/90 to 91/90 with `Tear=0` and `Stale=0`; Makepad cadence rows reported about 90Hz app/XR/draw cadence and about 50Hz paired camera texture updates. The remaining visual gap is that the projected camera image does not yet cover the panel like the custom target; invalid projected UV regions appear as black/dark edge sampling. | S77 should port the public target shader's invalid-UV fallback behavior: when the projected camera UV is outside `[0,1]`, fall back to the oriented unprojected content UV instead of clamping the projected UV to a likely-black edge. This keeps S76's dynamic rows while matching the custom target's projected-content coverage behavior more closely. |
| S77 | Rusty XR invalid-UV fallback parity | Source patch, standalone source validation, fresh release APK build, native-library string check, guarded launcher device gate, six-frame freshness sequence, marker/fault counters, screenshot review, and target diff. | Partial visual pass. S77 preserved S76 direct draw-vars and homography binding, then changed only the shader's projected-UV sampling policy to match the public target fallback path: projected UVs outside `[0,1]` fall back to the oriented unprojected content UV instead of clamping to an edge sample. The fresh APK contained S77 transport/scene markers and no stale S76/S75 transport strings. The guarded launcher path reached active XR on `launcher-attempt-1`, emitted S77 plus homography-ready markers, captured six byte-distinct screenshots, and kept app/global GPU-fault plus fatal counters at zero while the known small hardware-buffer warning class stayed visible. App-process VrApi rows stayed at 90/90 to 91/90 with `Tear=0` and `Stale=0`. Screenshot review showed live camera coverage restored across the panel edge policy, but the result is still a diagnostic panel rather than the custom target's fullscreen projection. | S78 should remove the physical 3D-panel convergence from the test by rendering the same proven YUV and surface-to-camera homography path as a fullscreen clip-space per-eye panel. If that restores target-like parallax behavior, continue into fine alignment and performance comparison. |
| S78 | Clip-space surface-homography panel | Source patch, standalone source validation, fresh release APK build, native-library string check, guarded launcher device gate, six-frame freshness sequence, marker/fault counters, screenshot review, and target log diff. | Passed as a geometry split. S78 kept S76/S77 acquisition, YUV color, current source-eye mapping, direct draw-vars, and invalid-UV fallback, but changed the panel vertex path from a head-centered 3D plane to a fullscreen clip-space surface. The fresh APK contained S78 markers and no stale S77/S76 transport strings. The guarded launcher path reached active XR on `launcher-attempt-1`, emitted S78 plus homography-ready markers, captured six byte-distinct screenshots, and kept app/global GPU-fault plus fatal counters at zero. App-process VrApi rows stayed at 90/90 to 91/90 with `Tear=0` and `Stale=0`. Screenshot review showed a true clip-space camera surface with no native passthrough fill or physical 3D-panel world anchoring. A refreshed Rusty XR target log diff then exposed the next mismatch: the target reports `display-left-from-left-source`, while S78 still reports `display-left-from-right-source`. | S79 should keep S78 clip-space geometry but switch Makepad to the target's display-left-from-left source mapping and direct `xr_view_id` source selector, then compare headset alignment. |
| S79 | Target source-eye mapping parity | Source patch, standalone source validation, fresh release APK build, native-library string check, guarded launcher device gate, six-frame freshness sequence, marker/fault counters, screenshot review, and target shader diff. | Passed as a marker-parity/source-mapping split. S79 kept S78 clip-space geometry, S77 invalid-UV fallback, S76 direct draw-vars, and the same Camera2 homography rows, but changed the display-to-source mapping to match the refreshed Rusty XR target marker: `display-left-from-left-source` with `sourceEyeSelector=xr_view_id`. The fresh APK contained S79 markers, the target source-mapping string, and no stale S78/S77 transport strings. The guarded launcher path reached active XR, emitted S79 plus homography-ready markers, captured six byte-distinct screenshots, and kept app/global GPU-fault plus fatal counters at zero. App-process VrApi rows stayed at 90/90 to 91/90 with `Tear=0` and `Stale=0`. Screenshot review shows the same clip-space projection class with the target source-eye marker now aligned. | S80 should port the next target shader delta: map fullscreen panel UVs back toward the raw camera-content UV domain using the full-view/raw overscan scale before applying `surface_to_camera`, matching the public target's `full_view_content_uv_scale(2.10, 1.06)` contract. |
| S80 | Target full-view content scale | Source patch, standalone source validation, fresh release APK build, native-library string check, guarded launcher device gate, six-frame freshness sequence, marker/fault counters, screenshot review, and target shader diff. | Partial/negative visual split. S80 kept S79 target source-eye mapping and S78 clip-space geometry, then applied the public target's full-view/raw overscan scale directly to the fullscreen panel UV before the surface-to-camera homography. The app stayed active-XR, marker-visible, byte-distinct, and fault-clean, with the small hardware-buffer warning class still tracked separately. Visual review showed a shifted/zoomed camera surface and striped edge artifacts rather than better alignment. The follow-up target shader diff clarified why: the public projected path builds `screen_to_camera` from an actual OpenXR `surface_to_screen` transform and Camera2 `surface_to_camera` rows; it does not simply pre-scale the projected screen UV by the content scale. | S81 should undo the S80 pre-scale and instead reconstruct a display-screen ray to a head-anchored preview surface before applying the Camera2 homography. If the approximate constants are insufficient, the next implementation step is exposing exact OpenXR per-eye view/FOV state to the Makepad app path. |
| S81 | Dynamic screen-to-surface UV reconstruction | Source patch, standalone source validation, fresh release APK build, native-library string check, guarded launcher device gate, six-frame freshness sequence, marker/fault counters, screenshot review, and app-process cadence parse. | Partial. S81 kept the S78/S79 clip-space fullscreen panel and target source-eye mapping, disabled the stale S80 pre-scale marker, and reconstructed a per-eye display ray from `draw_pass.camera_inv`, a static display-FOV estimate, eye offset, preview depth, preview FOV, and raw overscan before applying the existing Camera2 homography rows. The fresh APK contained S81 markers and no stale S80 path. The launcher gate reached active XR on `launcher-attempt-1`, captured six byte-distinct frames, kept app/global GPU-fault and fatal counters at zero, and reported Makepad app/XR/draw cadence around 90Hz with paired texture updates around 50Hz. Visual review showed live camera content in a target-like fullscreen footprint, but the right eye still had a striped invalid/fallback region and the projection remained only approximate. | S82 should collapse the Rusty XR target-style chain on the CPU: compute display-screen-to-camera homography rows from Camera2 `surface_to_camera` and a display-eye `surface_to_screen` approximation, pass those direct rows to the shader, and use screen-to-surface rows only for invalid/fallback fill. If this is still off, expose exact OpenXR per-eye view/FOV state from the Makepad path. |
| S82 | Collapsed screen-to-camera homography | Source patch, standalone source validation, fresh release APK build, native-library string check, guarded launcher device gate, six-frame freshness sequence, marker/fault counters, screenshot review, and app-process cadence parse. | Negative visual split, useful for narrowing. S82 added CPU-collapsed screen-to-camera rows and separate screen-to-surface fallback rows to the Camera2 projection plan using the shared `rusty-xr-camera-model` helpers. The shader sampled projected camera UVs directly from display-screen UV and used screen-to-surface only for invalid/fallback seed fill. The fresh APK contained S82 markers, no stale S81/S80 path labels, and the target source-eye mapping. The launcher gate reached active XR on the first guarded attempt, emitted S82 and homography-ready markers, captured six byte-distinct frames, and kept app/global GPU-fault plus fatal counters at zero while the small hardware-buffer warning class remained visible. App-process VrApi stayed around display rate with `Tear=0` and `Stale=0`; Makepad app/XR/draw cadence remained around 90Hz with paired camera texture updates around 50Hz. Visual review still showed live camera content with invalid/fallback striping, so the remaining gap is not stale shader state or acquisition. | S83 should expose exact OpenXR per-eye view/FOV state from the Makepad path and use it to compute the same `surface_to_screen` -> `screen_to_camera` chain as the public target, rather than continuing to tune synthetic display-FOV and eye-offset constants. |
| S83 | Draw-pass projection inverse screen-to-surface | Source patch, standalone source validation, fresh release APK build, native-library string check, guarded launcher device gate, six-frame freshness sequence, marker/fault counters, screenshot review, and refreshed public target run. | Negative visual split, useful for narrowing. Inspection of the Makepad fork showed that `openxr_draw_pass_to_vulkan` already writes the active per-eye OpenXR projection and view matrices into `draw_pass.camera_projection` and `draw_pass.camera_inv` before rendering. S83 therefore avoided a fork API change and reconstructed the display ray in the shader from `inverse(draw_pass.camera_projection)` plus active-eye `camera_inv`, then applied the proven Camera2 `surface_to_camera` rows. The fresh APK contained S83 markers, stale S82 path labels were absent, the launcher gate reached active XR, emitted S83 markers, captured six byte-distinct frames, and kept app/global GPU-fault plus fatal counters at zero while the small hardware-buffer warning class stayed visible. CPU YUV probes still reported nonzero left/right camera content. Screenshot review rejected the visual result: the projected camera surface was black/blank instead of the S82/S74 live-feed class. A refreshed public fast `0.75` target run under the comparison device profile still reported `activeTier=gpu-projected`, `alignedProjection=true`, paired left/right GPU buffers, `cpuUploadCount=0`, target source-eye mapping, app-process `VrApi` at 72/72 to 73/72 with `Tear=0` and `Stale=0`, and six actually distinct frame hashes; its generic center-ROI validator was black-like because the current headset pose put most camera content low in the screenshot, not because app-frame progression froze. | S84 should preserve the recovered-feed lineage from S82/S76/S77, then either debug the Makepad shader projection-matrix convention with a near/far ray/fallback diagnostic or expose exact per-eye OpenXR view/FOV/head-center state from the maintained Makepad path so the public target's `surface_to_screen` -> `screen_to_camera` CPU chain can be reproduced directly. |
| S84 | Projection-inverse near/far with S82 fallback | Source patch, standalone source validation, fresh release APK build, native-library string check, guarded launcher device gate, six-frame freshness sequence, marker/fault counters, screenshot review, and refreshed public target evidence carried from S83. | Negative visual split, useful for narrowing. S84 kept the S83 exact-display experiment but reconstructed the display ray from near/far unprojected points instead of a single clip point, then bound S82's CPU-collapsed `screen_to_camera` rows as an explicit shader fallback. The fresh APK contained S84 marker strings, no stale S83 path labels, reached active XR on the guarded launcher path, emitted S84 and homography-ready markers, captured six byte-distinct screenshots, and kept app/global GPU-fault plus fatal counters at zero while the small hardware-buffer warning class remained visible. Camera2 acquisition and CPU YUV probes still showed live nonzero content, and native passthrough stayed disabled, but screenshot review showed only the app's black surface plus the runtime HUD. That means the exact branch can still select a valid-looking but visually black sample, and the fallback was not decisive enough to preserve the recovered-feed lineage. | S85 should force the S82 `screen_to_camera` fallback branch as the active sample path while keeping the S84 exact branch disabled and logged as stale/absent. If S85 restores live feed, the Makepad draw-pass projection inverse is the blocker. If S85 is still black, the new fallback-row binding path or branch wiring regressed and should be fixed before any further exact OpenXR view/FOV work. |
| S85 | Forced screen-to-camera fallback control | Source patch, standalone source validation, fresh release APK build, native-library string check, guarded launcher device gate, six-frame freshness sequence, marker/fault counters, screenshot review, and app log parse. | Negative visual split, useful for narrowing. S85 kept S84's bound row set but disabled the exact near/far projection-inverse branch by forcing its validity mask to zero. The fresh APK contained S85 path and marker strings, no stale S84 path/true markers, and reached active XR on the guarded launcher path. The run emitted S85 markers, counted `s82CollapsedScreenToCameraHomography=true`, captured six byte-distinct frames, and kept app/global GPU-fault plus fatal counters at zero. App logs reported live nonzero YUV plane content, bound left/right textures, `projectionHomographyReady=true`, and the expected screen-to-camera rows. Screenshot review still showed only the black app surface plus runtime HUD, so the forced `screen_to_camera` UV domain or branch wiring is not a visual recovery path. | S86 should remove homography sampling entirely and use direct fullscreen YUV UVs as a visual control. If direct UV restores camera feed, repair the screen-to-camera UV domain/row composition. If direct UV also stays black, trace the current fullscreen shader route against the earlier S49-S59 visible camera controls before changing projection math. |
| S86 | Direct fullscreen YUV sampler control | Source patch, standalone source validation, fresh release APK build, native-library string check, guarded launcher device gate, six-frame freshness sequence, marker/fault counters, screenshot review, and target shader diff. | Positive visual control, not projection parity. S86 removed projection rows from the active sample path and sampled `vec2(1 - full_view_uv.x, 1 - full_view_uv.y)` with the same per-eye YUV textures, border guide, active-XR path, and passthrough-off state. The fresh APK contained S86 strings and no stale S85/S84/S82-active strings. The guarded launcher path reached active XR, emitted S86 markers, captured six byte-distinct frames, and kept app/global GPU-fault plus fatal counters at zero while the small hardware-buffer warning class remained visible. Screenshot review showed real camera detail again, proving the current fullscreen draw/YUV sampling route is healthy. The public target shader diff confirms the projected fast path applies `screen_to_camera` to display-surface UV; S85 doing the same with simplified Makepad display-eye rows went visually dark. | Stop tuning constants in the app shader. Expose exact active per-eye OpenXR view/FOV state from the maintained Makepad path to app Rust, then recompute the public target's `surface_to_screen` -> `screen_to_camera` CPU homography chain with real runtime eye state instead of the current static display-FOV/eye-offset approximation. |
| S87 | Runtime OpenXR view/FOV homography | Makepad fork patch, fork validation, fork push, dependency lock update, app source patch, standalone source validation, fresh release APK build, native-library string check, guarded launcher device gate, six-frame freshness sequence, marker/fault counters, screenshot review, and refreshed public target comparison. | Passed as a structural parity gate, not final visual acceptance. The maintained Makepad fork exposes app-visible per-eye OpenXR local-space pose and FOV state through `XrState`, sourced from the same runtime view data that drives projection rendering. The Makepad example consumes that fork revision, stores the selected Camera2 stereo sources, recomputes the public target-style `surface_to_screen` -> `screen_to_camera` homography when runtime XR view state arrives, refreshes paired-import projection rows, and reports `s87RuntimeXrViewHomography=true` plus `runtimeXrViewStateReady`. The fresh APK contained S87 strings and no stale S86/S85 path strings. The launcher gate reached active XR on `launcher-attempt-1`, emitted runtime-view and homography-ready markers, captured six byte-distinct screenshots, and kept app/global GPU-fault plus fatal counters at zero while the small hardware-buffer warning class stayed visible. Screenshot review showed live projected camera content in the same low-in-frame headset-pose class as the refreshed public fast `0.75` target, not the previous black-surface class. The remaining visual delta is that S87 returns black when projected UVs are invalid, while the public fast shader falls back to an oriented content-surface sample and dims it. | S88 should port the target invalid-UV fallback policy while preserving S87 runtime-view rows, source-eye mapping, and fault/freshness gates. |
| S88 | Target fast invalid-UV fallback | Source patch, standalone source validation, fresh release APK build, native-library string check, guarded launcher device gate, six-frame freshness sequence, marker/fault counters, app-process cadence parse, and screenshot review. | Passed as a target-shader-policy gate, not final projection acceptance. S88 kept the S87 runtime OpenXR view/FOV homography rows and changed only the shader invalid-projection policy: invalid projected UVs now sample an oriented fallback seed derived from the runtime `screen_to_surface` homography and are dimmed, matching the public fast shader's behavior more closely than S87's black return. The fresh APK contained S88 strings and no stale S87/S86/S85 path strings. The guarded launcher path reached active XR on `launcher-attempt-1`, emitted `runtimeXrViewStateReady`, `projectionHomographyReady`, `s87RuntimeXrViewHomography=true`, and `s88TargetFastInvalidFallback=true`, captured six byte-distinct screenshots, and kept app/global GPU-fault plus fatal counters at zero while the small hardware-buffer warning class stayed visible. Visual review kept the recovered projected camera feed and target-like low-in-frame headset-pose class; edge/invalid regions now follow the target fallback policy more closely. The run is not a normalized performance comparison: this Makepad gate presented around 90Hz, while the refreshed public target sample presented around 72Hz. | S89 should diff the S88 runtime-view homography and shader sampling chain against the validated public fast `0.75` target implementation, then close the remaining close-range parallax/projection geometry gap before refresh-normalized performance testing. |
| S89 | Single-quad target screen UV | Source patch, standalone source validation, fresh release APK build, native-library string check, and attempted device-gate setup. | In progress, device gate blocked. S89 keeps S87/S88 runtime OpenXR view/FOV homography rows, target source-eye mapping, and invalid-UV fallback behavior, but replaces the Makepad-only flattened `CubeGeom` panel with one `QuadGeom` fullscreen surface. The vertex path now maps quad position directly to target-style screen UV and clip position, avoiding six-face overdraw and making the shader input domain match the public target fullscreen pass more closely. Markers report `s89SingleQuadTargetScreenUv=true` while retaining `s88TargetFastInvalidFallback=true`. Standalone `cargo fmt --check` and `cargo check` passed, with only the known Makepad duplicate-dependency warning. A fresh release APK built and the native library contained S89 marker/path strings with stale S88/S87/S86 path strings absent. The Quest gate did not run because no ADB transports were visible after checking multiple platform-tools paths and restarting the ADB server; this is a transport blocker, not a projection result. | Restore ADB transport, then run S89. Require active XR, runtime-view and homography-ready markers, stale S88/S87/S86 path counters at zero, byte-distinct screenshots, fault-clean logs, and headset/screenshot review focused on whether deterministic fullscreen UVs reduce or worsen the remaining close-range parallax. |
| S90 | Camera-ID source binding parity | Makepad fork patch, fork validation and push, lockfile update, source patch, standalone source validation, fresh release APK build, native-library string check, device-gate parser update, public-boundary scan, public push, static target diff, focused host unit tests, guarded launcher device gate, and operator headset review. | Objective runtime gate passed, but visual parity failed. The target diff showed a likely Makepad-only ambiguity: the public target selects the stereo pair by Camera2 pose X and reports display camera IDs, while Makepad Android descriptors previously named all back cameras identically and the example paired Makepad video choices by source index. The fork now appends a stable `cameraId=` token to Android video descriptors. The example now carries `leftCameraId` / `rightCameraId` through `StereoProjectionPlan`, orders the Camera2 stereo pair by pose X with the old index-scored selection as fallback, parses Makepad descriptor camera IDs, binds Makepad video streams by ID when possible, and reports `sourceBindingMode`. Standalone validation and focused host unit tests passed, a fresh release APK contained S90 and `cameraId=` strings with stale S89/S88/S87/S86 path strings absent, and the guarded launcher run reached active XR, emitted `s90CameraIdSourceBinding=true`, `sourceBindingMode=camera-id`, runtime-view, and homography-ready markers, captured six byte-distinct screenshots, and stayed fault-clean while preserving the small hardware-buffer warning counter separately. Operator review then reported persistent depth-dependent misalignment/parallax, an apparent left/right source-eye flip, and a roll/orientation defect where a horizontal real-world surface rotated toward vertical on screen. Source binding is therefore ruled out, but projection math is not. | S91 should separate display-eye homography row indexing from source-eye texture selection, restore the inverted source selector as a visual correction candidate, and remove the 180-degree texture UV rotation before any performance conclusions are drawn. |
| S91 | Display-row source-eye correction | Source patch, focused host tests, device-gate parser update, fresh release APK build, native-library string check, guarded launcher device gate, six-frame freshness sequence, marker/fault counters, and documentation update. | Objective gate passed, visual acceptance pending. S91 keeps S90's Camera2 camera-ID binding and runtime OpenXR view/FOV homography inputs, but changes the active shader path so `screen_to_camera` rows are selected by display eye while YUV texture selection is selected by the inverted source eye. The Camera2 plan now reports `display-left-from-right-source` and remaps display-left/display-right `surface_to_camera` rows before collapsing them into display-indexed `screen_to_camera` rows, matching the target renderer's separation between display-eye projection rows and source-eye texture selection. The texture orientation is also changed from the S86/S90 `flip-x-and-y` 180-degree transform to vertical-only UV correction, because earlier direct-Y evidence and camera-model tests showed that vertical-only preserves the stereo-baseline signal while 180-degree rotation can reverse it. Focused host validation passed, the fresh APK contained S91 path strings with stale S90 path strings absent, and the launcher gate reached active XR, emitted S91 display/source mapping markers, captured six byte-distinct screenshots, and stayed fault-clean while preserving the small hardware-buffer warning counter separately. | Use S91 for objective Makepad transport/performance diagnostics, but keep the projection-math result as unaccepted until operator headset review verifies source-eye mapping, close-range parallax, and roll/orientation behavior. |
| S92 | First normalized transport/performance comparison | Fresh public target APK rebuild, catalog-profile launch, guarded Makepad S91 launch, 90-second logcat windows, six-frame freshness sequences, app CPU/memory snapshots, `VrApi` parse, and screenshot inspection. | Diagnostic comparison completed, not a visual parity claim. Both runs used CPU/GPU level `4` / `4` and `VrApi` scale factor `0.75`, produced six byte-distinct screenshots, and stayed app/global GPU-fault plus fatal clean. The public target profile held about `72.9/72Hz` over the steady window with `Tear=0`, `Stale=0`, `App=1.73ms`, `CPU&GPU=1.48ms`, `GPU%=0.20`, `CPU%=0.21`, app-process `top` around `13.9%`, paired GPU buffers, and `cpuUploadCount=0`. Makepad S91 held about `90.5/90Hz` with `Tear=0`, `Stale=0`, `App=2.06ms`, `CPU&GPU=7.56ms`, `GPU%=0.28`, `CPU%=0.47`, app-process `top` around `93%`, app/XR/draw cadence around `90Hz`, and paired Makepad camera texture updates around `50Hz`; the small hardware-buffer warning class stayed visible and non-fatal. Screenshot review showed live camera content in both stacks, while Makepad projection math remains unaccepted until headset review. | Keep the public target as the projection-math authority. Use Makepad's current result as evidence that its launcher, XR loop, and camera texture cadence are healthy, but reduce its CPU-heavy camera path before treating it as the preferred effects or smoothing host. |

| S93 | Refresh and camera-consumption comparability | Source instrumentation, scorecard parser update, and documentation prep only; no headset run while the device is unavailable. | Planned. S92 was useful but not fully refresh-normalized: the public fast target requested the Rusty XR default `72.0Hz` display refresh and observed about `72.9/72Hz`, while Makepad S91 ran at the runtime/device `90Hz` mode. Both paths showed camera delivery/update cadence near `50Hz`, so display cadence and camera delivery cadence are separate axes. The missing measurement is how often a submitted projection frame consumes a new camera pair versus reusing the previous camera pair with a fresh OpenXR head pose. | Next run should compare public fast `0.75` at explicit 72 Hz, public fast `0.75` at explicit 90 Hz via `rustyxr.xrDisplayRefreshHz=90.0`, Makepad S91 at current/default 90 Hz, and Makepad S91 at 72 Hz only if a safe Makepad refresh request exists. Require scorecard fields for requested/active display refresh, `VrApi` target, camera delivery/update Hz, camera consumed frame Hz, projection render Hz, renders per camera frame, app/XR/draw cadence, `Tear`, `Stale`, `Early`, CPU/GPU levels, render scale, CPU/GPU percentages, and byte-distinct screenshots. |

Operator visual note carried forward from S34/S35: the previous occlusion screenshot
showed real-world geometry masking the app-owned colored panel, while the
current S40 APK state does not. The current non-occluded state is preferred for the
camera-streaming proof because it avoids confusing runtime depth/environment
masking with app-owned camera pixels. If this occlusion class returns, treat it
as a separate regression in depth-clip/environment-depth state, not as evidence
for or against Camera2 acquisition.

## Validation Rule

For future Quest validation, do not rely on one long logcat window for both
startup evidence and stability counters. Capture startup markers in a short
window, then run a separate longer liveness/fault window. Count small
hardware-buffer warnings separately from GPU page-fault and fatal signatures,
because the next Camera2 slices will intentionally introduce hardware-buffer
ownership.

For autonomous device runs, also treat the headset awake/proximity control
state as a preflight condition. One S7 validation sequence coincided with the
device moving back toward standby after a Horizon OS display-event-receiver
failure and service restart. If this recurs, record the immediately preceding
adb action separately from app-level marker results.
For performance comparisons, also record `debug.oculus.cpuLevel` and
`debug.oculus.gpuLevel` before launch and in the final artifact summary. A
Makepad-vs-custom result is only comparable if both sides use the same declared
device performance levels, or if the levels are intentionally varied as a test
axis.
If the operator has already set a keep-awake hold, comparison harnesses should
prefer passive readback and skip their own timed proximity hold to avoid
competing state transitions.
The public camera-profile harness now writes `power-state-summary.json` after
each run, comparing the post-proximity-hold snapshot to the final capture. By
default this is a warning so existing smoke workflows keep producing artifacts;
unattended comparison jobs can opt into a hard stop with
`-FailOnPowerStateDrift`.

For visual gates, classify headset imagery into three separate buckets:
native compositor passthrough behind the projection layer, app-owned panel
content, and environment-depth/depth-clip occlusion of app geometry. The current
camera-streaming proof should keep the app panel's depth clipping disabled so
room geometry cannot cover the diagnostic panel and masquerade as camera
alignment behavior.
The occlusion bucket is intentionally independent of camera ownership: a
headset screenshot where room geometry covers the app-owned panel is evidence
that a runtime depth/depth-clip path is active, not evidence that the app is
sampling raw camera pixels. For the current camera-streaming proof, that
occlusion should remain off. The live comparison state after S35 is therefore
directionally better for camera-streaming isolation even though camera pixels
are not yet proven.

Follow-up inspection of the invalid S8 custom 0.75 run found no retained
`automation_disable`, `prox_close`, or `setVirtualProxState` command lines in
the logcat window. Instead, the app-side display-event-receiver failure was
followed by a fatal `android.ui` / `system_server` path, `VrPowerManagerService`
was recreated, and the recreated VR power-manager event log started from a new
initial state without the previous virtual `CLOSE` override. Treat this as a
service-restart loss of the virtual keep-awake override. The mitigation is to
make autonomous runners preserve or reapply the operator-requested keep-awake
hold after such a reset, rather than relying on one synthetic wake signal.

Follow-up companion validation restarted the source companion app from a fresh
session, observed the existing keep-awake hold, then induced a controlled
device-side proximity reset. Passive readback showed the virtual state move to
`DISABLED` briefly, then return to `CLOSE` before standby markers appeared. The
companion wake helper now applies the same durable keep-awake hold so a wake
action does not shorten an operator-maintained autonomous-run hold. Raw device
logs remain private.

Additional autonomous-run hardening has been added through the optional broker
shell helper. Because the helper is launched by an authorized ADB host and runs
as Android `shell`, it can maintain the same virtual-close hold from the device
side while the external Companion watchdog remains active. The coordination rule
is intentionally idempotent: both watchdogs preserve `Virtual proximity state:
CLOSE`, neither sends normal-proximity restoration while preserving a hold, and
the shell helper only rebroadcasts `prox_close` after readback shows a non-close
virtual state.

The first coexistence gate passed with both watchdogs active. Baseline readback
was `CLOSE` / `HEADSET_MOUNTED`; a controlled virtual-proximity reset was
followed by a shell-helper `reapply_count=1`, later passive readbacks remained
`CLOSE`, and no standby progression appeared in the validation window. The stop
marker path also reported the shell helper disconnected cleanly, then the helper
was restarted with the normal long-duration hold for continued autonomous-run
protection. Raw device logs remain private.

## Open Questions

- Does the forked Makepad shell stay clean over longer Quest/Vulkan XR repeats
  and repeated launch/stop cycles, not only the current 90s synthetic stereo
  samples?
- Are the repeated small hardware-buffer warnings benign Makepad/Quest surface
  churn, or do they expose a future collision with Camera2 hardware-buffer
  import? S6 result: the warning class persisted during successful Makepad
  import and did not correlate with app-process GPU page-fault or fatal lines,
  so it remains visible but non-blocking for the next split.
- Should the first camera pass use Makepad's headset-camera surface, a Rusty
  XR-owned Camera2 adapter, or a thin bridge that reports both surfaces?
  Answer for S5/S6: use a Rusty XR-owned Android NDK Camera2 metadata and
  one-frame acquisition probe first, then a Makepad-owned Android camera
  playback for the first Makepad/Vulkan hardware-buffer import proof.
- Should the first projection pass reproduce `display-screen-homography`,
  `quad-surface`, or both?
- Which activity/bootstrap/app markers should become shared scorecard inputs
  before camera frames are introduced?
