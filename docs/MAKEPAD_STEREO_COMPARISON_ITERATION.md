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

The current slice is step 8:

- keep the maintained Makepad fork branch
  `rusty-xr/android-libstd-packaging` as the Android app-shell dependency
- preserve the completed Camera2 metadata, bounded acquisition, paired Makepad
  import, and metadata-backed projection-mapping markers
- compare the direct generated-XR Makepad path with the custom Rusty XR stereo
  projection baseline using the same core counters where possible: app-process
  GPU page-faults, fatal signatures, small hardware-buffer warnings, `VrApi`
  cadence, camera/source progression, CPU upload count, and projection-ready
  flags
- kept the small `AHardwareBuffer` warning class visible as a separate counter
  from app-process GPU page-fault and fatal signatures
- use passive awake/proximity readback before and after samples; do not issue a
  new proximity-control command during comparison captures unless an operator
  explicitly asks for it

This slice should determine whether the Makepad direct generated-XR path is
cadence- and fault-comparable to the custom path after S7 proved paired import
and projection mapping. It does not add broker streaming, private visual-effect
policy, or visual release acceptance.

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

A proper performance comparison against the custom stereo camera projection
baseline is not open yet. The S6 Makepad APK proves that one Makepad-owned
camera hardware buffer can reach `VideoTextureUpdated`; it still reports
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
If the operator has already set a keep-awake hold, comparison harnesses should
prefer passive readback and skip their own timed proximity hold to avoid
competing state transitions.
The public camera-profile harness now writes `power-state-summary.json` after
each run, comparing the post-proximity-hold snapshot to the final capture. By
default this is a warning so existing smoke workflows keep producing artifacts;
unattended comparison jobs can opt into a hard stop with
`-FailOnPowerStateDrift`.

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
