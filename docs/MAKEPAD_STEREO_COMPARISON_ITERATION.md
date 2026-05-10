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

The current slice is S48, a direct camera Y-plane sampler control for the
scene-owned Makepad camera panel. S14 cleared the active-presentation blocker
for the Makepad launcher path, S16-S20 proved scene-owned geometry, S26
restored the manual `VideoExternal` import route, S33 isolated the app-owned
panel from native compositor passthrough ambiguity, S36 fixed the generated-XR
activity handoff, and S40 proved that shader edits can change the visible
panel from stale proof colors to a neutral guide state. S41-S45 then moved the
blocker below Camera2 acquisition: CPU-side Y/U/V plane content is present,
active OpenXR cadence is present, the same result holds with gain-boosted
Y-plane sampling, synthetic R8 texture controls, all-slot texture controls,
and a marker-selected constant-gray bypass. S46 then proved the visible panel
does execute edited shader code when the fragment path returns before any
texture sampling. S47 then proved a generated R8 texture bound through the same
panel slot can be sampled visibly as a checker pattern. The next split disables
the generated replacement and samples only the real Makepad camera Y plane,
still before any `sample_video()` or YUV conversion work:

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
- keep environment depth optional/non-fatal in the maintained Makepad fork so a
  depth-policy failure cannot block passthrough/projection startup
- keep the small `AHardwareBuffer` warning class visible as a separate counter
  from app-process GPU page-fault and fatal signatures
- use passive awake/proximity readback before and after samples; do not issue a
  new proximity-control command during comparison captures unless an operator
  explicitly asks for it

The same counters remain valid for comparison runs: app-process GPU page-faults,
fatal signatures, small hardware-buffer warnings, runtime cadence,
camera/source progression, CPU upload count, projection-ready flags, and
visual-classification evidence. The current camera-streaming proof remains
open because the app-owned panel is visible and non-occluded, shader edits are
live, and generated texture-slot sampling now has a positive proof. Real camera
plane sampling is the next acceptance gate. This slice does not add broker
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
| S48 | Direct camera Y-plane texture sample | Source patch, host validation, clean release APK rebuild with APK string check. | Device gate pending. The source and APK are ready, and the APK contains the direct camera-Y marker path while the S47 generated-R8 marker path is absent. The planned device run could not start because no Quest ADB target was enumerated after the build; an ADB server restart and Wi-Fi ADB connect attempt did not restore a device. | When a Quest ADB target reappears, clean-install the S48 APK, pregrant the active-XR permission set, and run the launcher screenshot/counter gate. |

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
