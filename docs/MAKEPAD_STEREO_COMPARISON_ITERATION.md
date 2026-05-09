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

## Current Slice

The current slice has completed step 6:

- keep the maintained Makepad fork branch
  `rusty-xr/android-libstd-packaging` as the Android app-shell dependency
- preserve the completed Camera2 metadata and bounded acquisition diagnostics
- after the bounded acquisition probe closes, started a Makepad-owned Android
  camera playback and import one camera hardware buffer through the
  Makepad/Android Vulkan texture path
- emitted an explicit `RUSTY_XR_MAKEPAD_HARDWARE_BUFFER_IMPORT` marker for the
  import attempt and result
- kept the small `AHardwareBuffer` warning class visible as a separate counter
  from app-process GPU page-fault and fatal signatures
- continue reporting `pairedLeftRightGpuBuffers=false` and
  `alignedProjection=false` until paired-buffer import and projection parity
  are implemented

This slice should prove only Makepad-path hardware-buffer import readiness. It
does not pass the exact one-frame `AImageReader` buffer from the Rusty XR-owned
metadata/acquisition probe into Makepad internals, claim stereo projection
parity, add broker streaming, or treat repeated small hardware-buffer warnings
as GPU page faults.

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

## Validation Rule

For future Quest validation, do not rely on one long logcat window for both
startup evidence and stability counters. Capture startup markers in a short
window, then run a separate longer liveness/fault window. Count small
hardware-buffer warnings separately from GPU page-fault and fatal signatures,
because the next Camera2 slices will intentionally introduce hardware-buffer
ownership.

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
