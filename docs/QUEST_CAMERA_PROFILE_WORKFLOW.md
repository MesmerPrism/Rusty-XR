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

The shader-side `external-cr-y-cb-bt601-narrow` mode is now a diagnostic, not a
release candidate, for this public Vulkan path. When used with the current
combined immutable sampler, it can produce a strongly green/discolored image,
which is consistent with applying a manual YCbCr decode to values that the
external sampler already presents like RGB. Keep this mode for devices or
adapter paths that truly expose channel-packed values at the shader boundary.

## Acquisition Difference To Test Next

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
do not make the same explicit AE target request. To isolate that difference,
test one public variable at a time:

1. `camera-stereo-gpu-composite-no-ae-target-065`: keep the combined
   `external-rgb` sampler and render scale `0.65`, but set
   `rustyxr.cameraTargetFps=0`, `rustyxr.cameraFpsMin=0`, and
   `rustyxr.cameraFpsMax=0`.
2. `camera-stereo-gpu-composite-reader-max-3-065`: keep the same public camera
   profile but set `rustyxr.cameraStereoImageReaderMaxImages=3`.
3. Only after those probes are valid, combine the settings with
   `-Override rustyxr.cameraTargetFps=0 -Override
   rustyxr.cameraStereoImageReaderMaxImages=3` and compare color, camera-pair
   cadence, stale-frame behavior, and OpenXR frame pacing.

Do not interpret a single green/discolored shader-decode run as evidence about
Camera2 acquisition. First return to `external-rgb`, then change acquisition
knobs.

## Workflow Gates

Use `tools/quest-camera-profile/Invoke-QuestCameraProfileRun.ps1` for
repeatable headset runs. It launches a catalog runtime profile, records the
exact launch extras, captures power/wake/VR-power snapshots, takes screenshots,
pulls logcat, and runs the validation helper.

A run should be rejected for color comparison when any of these are true:

- the screenshot is missing, empty, or camera-content ROIs are black-like
- logcat includes a sleep, screen-off, session-exit, or automation-disable
  transition during the capture window
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
- CPU upload count
- hardware-buffer import cache hits, misses, evictions, and size
- OpenXR display FPS, frame time, render scale, fixed-foveation state, and
  compositor tear/stale-frame signals

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
