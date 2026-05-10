# Quest Camera Profile Workflow

This note records the current public state of the Quest camera composite
example and the reusable workflow for continuing camera color, cadence, and
runtime-profile comparisons.

## Current State

The public Quest composite example now has the important structural pieces in
place: paired Camera2 `PRIVATE` camera acquisition, Android hardware-buffer
import, Vulkan sampling, metadata-backed stereo projection, explicit
source-eye mapping, texture-orientation controls, and the public feedback
border. Geometry and border checks should remain isolated from camera color and
performance probes unless a later change directly requires touching them.

The best current public baseline is the combined immutable-sampler
`external-rgb` path. On the tested Quest runtime it can hold the OpenXR display
loop at `72 Hz` at `rustyxr.xrRenderScale=0.65`, with CPU camera uploads
disabled and the hardware-buffer import cache stable after warm-up. That makes
it the profile to use for release-candidate projection and border checks.

The baseline still has open color and cadence gaps. It can look brighter,
lower-saturation, or cyan/washed relative to an optimized downstream raw-camera
renderer. Camera2 producer cadence is also separate from OpenXR submit cadence:
the renderer can submit at `72 Hz` while the paired Camera2 streams deliver
new buffers below display rate.

The April 30, 2026 acquisition probes narrowed the remaining blocker. A mono
Camera2 `PRIVATE` GPU-buffer probe at `1280x960` delivered ongoing frames while
the OpenXR loop held `72 Hz`, which means the public hardware-buffer import and
Vulkan sampling path are not globally broken. The concurrent separate stereo
Camera2 path can still stall with only a handful of new camera frames across
many OpenXR frames, even when power and VR-power snapshots report the headset
awake, mounted, and under a timed proximity hold. Those stale runs also showed
Camera HAL warnings about non-increasing stream timestamps.

The shader-side `external-cr-y-cb-bt601-narrow` mode is now a diagnostic, not a
release candidate, for this public Vulkan path. When used with the current
combined immutable sampler, it can produce a strongly green/discolored image,
which is consistent with applying a manual YCbCr decode to values that the
external sampler already presents like RGB. Keep this mode for devices or
adapter paths that truly expose channel-packed values at the shader boundary.

## Acquisition Findings And Next Axis

The next focused axis is camera acquisition, not projection or border
geometry. The current public Java Camera2 path uses:

- Java `ImageReader` objects for separate left/right `PRIVATE` streams
- stereo `ImageReader` max images defaulting to `8`
- an explicit `CONTROL_AE_TARGET_FPS_RANGE` request when
  `rustyxr.cameraTargetFps` or `rustyxr.cameraFpsMin` /
  `rustyxr.cameraFpsMax` is set
- public profiles that have commonly requested `72-72`, with the tested
  runtime applying `60-60`

Some lower-level downstream camera stacks use a smaller retained image pool and
do not make the same explicit AE target request. The public profile keeps those
differences as isolated catalog probes:

1. `camera-stereo-gpu-composite-no-ae-target-065`: keep the combined
   `external-rgb` sampler and render scale `0.65`, but set
   `rustyxr.cameraTargetFps=0`, `rustyxr.cameraFpsMin=0`, and
   `rustyxr.cameraFpsMax=0`.
2. `camera-stereo-gpu-composite-reader-max-3-065`: keep the same public camera
   profile but set `rustyxr.cameraStereoImageReaderMaxImages=3`.
3. The same settings can be combined with a single PowerShell override string,
   for example `-Override
   'rustyxr.cameraTargetFps=0,rustyxr.cameraStereoImageReaderMaxImages=3'`,
   after the individual probes pass the workflow gates.

On the tested runtime, rerunning those probes with the current validator did
not resolve the concurrent-stereo stale-frame condition. Lowering the requested
separate-eye size from `1280x1280` to `1280x960` also did not fix that
condition, while the mono GPU-buffer probe at the same height continued to
deliver live Camera2 frames. Treat the no-AE-target and max-images-3 profiles
as useful regression probes, but not as the current most likely parity fix.

The next serious public module split is acquisition: keep the Java Camera2
concurrent-separate Vulkan example as a documented path, and add or compare a
lower-level/native hardware-buffer reader path as a separate module/profile
before changing projection geometry, border behavior, or shader color math
again.

The public example now includes an opt-in native acquisition probe through
`rustyxr.cameraAcquisition=native-ndk`. That probe opens lower-level Android
NDK `ACamera*` sessions, creates `AImageReader` outputs with
`AIMAGE_FORMAT_PRIVATE` plus GPU-sampled hardware-buffer usage, calls
`AImageReader_acquireLatestImage`, takes an `AHardwareBuffer` reference,
deletes the `AImage` immediately, and publishes only the newest stereo pair
into the same Vulkan projection path. On the tested runtime, direct native
side-camera sessions matched those ownership details but still produced stale
camera progression in one side stream, so "native NDK" by itself is not yet a
validated fix. The next native tests should compare the effective camera source
and session shape, not keep retesting Java queue depth.

The native probe now logs every NDK camera ID that exposes lens-facing metadata,
including logical multi-camera capability, physical camera IDs, sensor sync
type, available `PRIVATE` output sizes, pose X, and pose reference. It also
logs whether the selected stereo source came from automatic synthetic dual-back
selection or explicit side IDs. Use `rustyxr.cameraStartDelayMs=<ms>` to test
acquisition lifecycle timing without changing projection/border/shader code,
and `rustyxr.nativeSourceMode=<label>` to tag native-source experiments in
logs.

The native path also has a single-camera mirror isolation mode:
`rustyxr.nativeSourceMode=single-back-mirror`. It opens one native back-facing
camera source and publishes the same acquired hardware buffer to both display
eyes. This is not stereo-alignment evidence, but it is useful for separating
renderer/import behavior from concurrent side-camera delivery. On one tested
runtime, the single-camera mirror profile kept live camera progression when it
used one physical side-camera ID, while the other side-camera ID remained
sparse even when opened alone. That points the remaining stale-frame issue
toward effective camera source/provider policy rather than Vulkan import,
OpenXR rendering, or callback lifetime. Treat exact camera IDs as run-local
diagnostics, not portable requirements.

Callback lifetime remains an important rule even though it is not the current
root cause: acquire the `AHardwareBuffer` reference, release/delete the
`AImage` promptly, and keep renderer import/descriptor work outside the image
callback. The public native probe follows that order.

Identity public color controls are also a useful probe:
`rustyxr.cameraColorContrast=1.0`,
`rustyxr.cameraColorBrightness=0.0`, and
`rustyxr.cameraColorSaturation=1.0`. On the tested runtime that reduced one
variable but did not close the raw-color gap by itself. Combined with the green
YCbCr-diagnostic result, that keeps the next color axis focused on sampler /
descriptor shape, range/gamma assumptions, and post-sampler calibration rather
than stacking a second BT.601 decode into the combined-sampler default.

`rustyxr.openxrPassthroughProbe` is a separate OpenXR runtime-state diagnostic.
`client` creates an `XR_FB_passthrough` client/layer and leaves it running;
`warmup` creates and resumes the layer briefly, then pauses the passthrough
while keeping the app's camera-rendering path unchanged. The manifest declares
optional passthrough and scene permissions so runtimes that gate
`XR_FB_passthrough` exposure can advertise the extension. In headset tests,
this exposed the extension and changed runtime client state, but it did not
fix stale native camera progression; the always-on client mode also added
runtime camera-compute load. Treat it as a capability/session-state probe, not
as a color or performance candidate.

Do not interpret a single green/discolored shader-decode run as evidence about
Camera2 acquisition. First return to `external-rgb`, then change acquisition
knobs.

## Workflow Gates

Use `tools/quest-camera-profile/Invoke-QuestCameraProfileRun.ps1` for
repeatable headset runs. It launches a catalog runtime profile, records the
exact launch extras, captures power/wake/VR-power snapshots, takes screenshots,
pulls logcat, and runs the validation helper.

For visual camera/parity gates, add a short screenshot freshness sequence:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-fast075 `
  -CaptureHzdbScreencap `
  -FreshnessFrames 6 `
  -FreshnessIntervalMs 1000
```

The generated freshness summary records per-frame SHA-256 hashes and flags
duplicate hash groups. A byte-identical sequence is a run-quality warning:
inspect camera-frame counters and rerun before treating a screenshot as live
camera evidence.

A run should be rejected for color comparison when any of these are true:

- the screenshot is missing, empty, or camera-content ROIs are black-like
- a multi-frame screenshot freshness sequence is byte-identical when the run
  expects live camera content
- logcat includes a sleep, screen-off, session-exit, or automation-disable
  transition during the capture window
- `Rusty XR final projection status` shows that the OpenXR loop kept
  rendering while the camera frame counter barely advanced
- the app silently falls back to CPU camera uploads for a GPU profile
- hardware-buffer import cache misses or evictions continue after warm-up
- the profile changes projection, border, sampler, and acquisition at the same
  time

Track these signals together:

- active runtime profile and launch extras
- power/charging state, battery level, wakefulness, VR power manager state, and
  timed proximity hold result
- camera color mode, tone controls, Vulkan external format, suggested YCbCr
  model/range, and component mapping
- requested and applied Camera2 AE FPS ranges
- observed Camera2 stream and stereo-pair FPS
- native `ACamera` side-frame counts, camera IDs, timestamp deltas, and whether
  single-camera mirror mode was active
- CPU upload count
- hardware-buffer import cache hits, misses, evictions, and size
- OpenXR display FPS, frame time, render scale, fixed-foveation state, and
  compositor tear/stale-frame signals

For direct camera versus broker H.264 cost isolation, use the broader
[Quest Streaming Diagnostics Workflow](QUEST_STREAMING_DIAGNOSTICS_WORKFLOW.md).
It adds synthetic compositor, existing-stream receive/decode, broker live
projected stereo, render-scale, and stage-timing scorecards on top of the same
profile-run artifacts.

Treat Meta shell `Start sleep timeout`, `Sleep timeout exceeded`, and
`WaitForWake` lines as warning signals to inspect against the captured power
and VR-power snapshots. Reject the run when those warnings line up with actual
screen-off, power sleep, session exit, or automation-disable signals. If the
headset remains awake and mounted, stale camera-frame progression is still its
own invalid condition.

Weak charging should be recorded because it can make unattended runs unreliable
if the battery continues to drain while plugged in. It is a run-quality
condition, not by itself proof that the camera pipeline is wrong.

## Public Tool Boundary

The public workflow tools are intentionally generic. They may capture local
screenshots, logs, and comparison reports under ignored artifact folders, but
those files must not be staged. Public docs should describe runtime profiles,
Camera2/OpenXR/Vulkan behavior, validation gates, and reusable diagnostics
without naming downstream apps, private repositories, local machine paths,
package identities, headset serials, or private visual-effect stacks.
