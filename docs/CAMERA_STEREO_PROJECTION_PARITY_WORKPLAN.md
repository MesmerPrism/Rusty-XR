# Camera Stereo Projection Parity Workplan

This note tracks the public Rusty XR path for bringing custom stereo Camera2
projection from "correct and visible" to "smooth with real GPU headroom." It is
intentionally public-safe: downstream app names, package IDs, raw local
artifacts, visual-effect stacks, generated APK identities, and private tuning
remain outside this repository.

## Target Map

Rusty XR's public baseline is the Quest composite-layer profile
`camera-stereo-gpu-composite-performance-065`.

The comparison target is a validated downstream custom stereo camera run that
uses the same broad class of app-owned stereo camera projection: headset camera
frames are acquired by the app shell, imported as GPU-readable textures, mapped
per eye, and submitted through an OpenXR presentation path. The downstream run
is not a source for public visual effects, exact tuning, package identity, or
private launch behavior. It is evidence that the public projection path should
be able to keep correct stereo while leaving more frame budget.

## Public Baseline

The accepted public `0.65` profile demonstrates:

- visible in-app stereo Camera2 projection
- selected concurrent left/right Camera2 sources with `1280x1280` square
  GPU-importable buffers
- `activeTier=gpu-projected`
- `alignedProjection=true`
- `stereoLayout=Separate`
- paired left/right GPU buffers
- `cpuUploadCount=0`
- stable Vulkan hardware-buffer imports with no continuing import failures or
  cache evictions after warm-up
- public camera projection metadata, per-eye homographies, and platform pose
  metadata flowing into the projection shader

The remaining issue is performance headroom. The representative accepted run
held roughly `70.7/72 Hz` near the end of the capture, but `VrApi` GPU
utilization was effectively saturated and the validation bundle still contained
sleep/tear warnings. That makes the run useful as a correctness baseline, not
as the final smoothness target.

## Current Fast Public Profiles

The current renderer-parity profiles are:

- `camera-stereo-gpu-composite-fast075`: direct in-app Camera2 stereo
  projection with the same Quest custom stereo geometry and the fast public
  raw-projection shader at render scale `0.75`.
- `camera-stereo-gpu-composite-fast065`: the same direct fast renderer path at
  render scale `0.65`.
- `broker-h264-stereo-live-openxr-projection-fast075-probe`: broker-owned
  Camera2 `50`/`51` capture, square `1280x1280` H.264 frames, hardware-buffer
  decode/import, frame-order live stereo pairing, and the fast public
  raw-projection shader at render scale `0.75`.
- `broker-h264-stereo-live-openxr-projection-fast065-probe`: the same broker
  fast path at render scale `0.65`.

Use the fast `0.75` profiles as the public renderer-parity lane. They hold
stereo geometry, GPU import, decode mode, and camera IDs constant while
removing the heavier soft-border shader work from the measurement. Minor
motion artifacts during head movement are tracked as stream/reprojection
compensation work, not as a stereo orientation failure.

The Makepad comparison lane is not ready for performance comparison yet. It has
reached paired per-eye camera sampling with a no-swap limited-BT.601 CPU-YUV
control, but the current blocker is app-panel placement/isolation. Recent
head-locked placement attempts produced live passthrough screenshots without a
visible app-owned panel. The maintained Makepad fork can now disable native
OpenXR passthrough, but a clean install must grant both Scene Access and Headset
Camera before the Makepad permission flow enters XR presenting. With both
permissions granted, the passthrough-off gate reaches active XR and emits the
native-passthrough-disabled marker, but S66 still showed no non-black clear
color, solid app panel, or black border. The active Makepad blocker is now
projection-layer output isolation, not Camera2 acquisition or shader color
conversion. Loading/preflight states are failed launches and must not be
counted as passthrough-off success. This does not invalidate the earlier
Makepad panel proof: the panel and live camera feed already rendered visibly in
the headset, and the remaining parity issue was that the surface was anchored
in world space instead of camera/head space. The S67a passthrough-on rerun
confirmed active XR and clean end-frame submission but still did not show the
panel. S62 artifact review narrowed the recovery target to the exact visible
source traits: plain world-space vertex path, the higher S62 panel pose
(`pos.y=0.92`), dark clear color, direct per-eye YUV sampling, and the thin
pale border. S67a2 restored those traits on the maintained fork and passed both
launcher-path and direct generated-XR visual gates: the app-owned live camera
panel and thin pale border are visible again, both screenshot sequences were
byte-distinct, end-frame markers were present, stale head-locked markers were
absent, and GPU-fault/fatal/small-hardware-buffer counters stayed at zero. The
next Makepad step was S67b: change only the native passthrough/background
isolation setting and iterate until a solid app-owned background plus the same
visible panel is present. S67b passed that ordered milestone in both launcher
and direct generated-XR runs: native passthrough was disabled, OpenXR submitted
an opaque projection layer, the app-owned live camera panel and pale border were
visible against a solid black background, and both screenshot sequences were
byte-distinct. Two follow-up watches carry forward: the requested non-black
clear color did not appear visually despite marker presence, and the small 4x4
hardware-buffer warning class plus environment-depth runtime logs remained
visible without GPU faults, fatal signals, or visible depth occlusion. S68 is
now the active Makepad gate: it keeps the S67b passthrough-off live camera
sampling and border, but replaces the S62 world-space vertex transform with
active-eye `draw_pass.camera_inv` placement. Passing S68 requires the panel to
remain visible and fresh with native passthrough disabled; classifying it as
non-world-space still requires headset-motion inspection and is not yet final
metadata-backed projection parity. The first S68 static gate passed the
visibility/freshness side in both launcher and direct generated-XR paths: the
app-owned live camera panel and pale border remained visible with native
passthrough disabled, six-frame screenshot sequences were byte-distinct,
end-frame markers stayed successful, and app/global GPU-fault and fatal
counters stayed at zero. The small 4x4 hardware-buffer warning class and
environment-depth runtime logs remain tracked; no visible depth occlusion was
seen in the captured frames. A passive eight-frame motion-review capture found
the pale border at a stable screen-space bounding box, but frame deltas were
small, so that sequence is not conclusive unless the headset was deliberately
moved during capture. Follow-up operator headset inspection completed that
classification: the panels are now cleanly projected per eye and no longer
behave like the earlier world-space surface. The Makepad lane is still not at
stereo projection parity because the camera feeds are swapped left/right
between eyes and the panels need alignment tuning for a good stereo effect.
The next ordered gates are therefore source-eye mapping correction, horizontal
mirror correction, then stereo alignment using the existing custom Rusty XR
target alignment notes. S69 kept the S68 per-eye `camera_inv` placement, S59
no-swap limited-BT.601 color path, and vertical-only UV orientation, but
changed the Makepad display source-eye selector to `inverted_xr_view_id` /
`display-left-from-right-source`. Operator review reported coherent eye
alignment but remaining horizontal mirroring, so S69b keeps the source-eye
mapping and adds a horizontal UV flip. The Makepad lane should not advance to
performance comparison until source-eye mapping, mirror orientation, and
alignment have all passed.

S69b built a fresh Makepad APK from the documented standalone lane and validated
it with the guarded device-gate harness. The harness reached active XR on the
first launcher attempt, recorded six distinct screenshots, preserved zero
GPU-fault/fatal counters, and found no stale S69/S68 path labels in the
extracted native library. Screenshot review shows the horizontal mirror patch
active, but final visual acceptance still requires headset/operator
confirmation before moving to S70 stereo alignment.

## Sanitized Parity Target

The downstream target proves the desired behavior class:

- custom stereo Camera2 projection visible in headset
- no app fatal or GPU-fault signatures in the validated window
- no final stale/tear behavior under operator inspection
- materially lower GPU load than the public `0.65` run
- device performance state controlled during the run
- camera permission handled before the measurement window

Treat refresh rate, render scale, buffer scale, multisampling, foveation,
permission state, and power/device levels as normalization axes before
declaring any implementation delta. The public objective is not to clone the
downstream shell. The objective is to make Rusty XR's reusable projection
contracts, diagnostics, and public example path expose enough of the same
low-cost shape.

## Absorbable Public Work

Rusty XR can absorb these public-safe lessons:

- Keep the direct Camera2 performance profile separate from broker, depth,
  MediaProjection, and visual-effect profiles.
- Expand the scorecard so parity comparisons always include source/build
  provenance, launch method, runtime settings, projection path, `VrApi` app and
  CPU+GPU time, GPU percentage, tear/stale counts, GPU import counters,
  camera-frame progression, app fatal signatures, and GPU fault signatures.
- Normalize device-level and permission setup in run manifests so one run is
  not compared against another with hidden runtime-state differences.
- Capture a short multi-frame screenshot freshness sequence for visual gates
  and record whether any frames are byte-identical before treating a screenshot
  as live camera evidence.
- Attribute projected draw cost into shader work, border/perimeter work,
  descriptor/import reuse, command recording, frame-buffer/render-pass churn,
  and submit behavior.
- Keep sampler and external-format handling explicit. Public logs should show
  whether camera imports use the expected external format path, sampler binding
  mode, import-cache size, descriptor-cache size, failures, misses, and
  evictions.
- Preserve public source taxonomy. Platform passthrough, raw Camera2, OpenXR
  environment depth, MediaProjection, and operator casting are separate witness
  streams, not interchangeable camera sources.
- Move only generic math, metadata, runtime-profile, scorecard, and optional
  adapter contracts into public crates. Keep downstream effect behavior and
  exact tuning downstream.

## Depth And Alignment Impact

Depth work changes how stereo alignment should be validated, but it should not
be folded into the main performance profile by default.

Use depth as an explicit witness or calibration profile:

- raw stereo camera frames remain the source for the custom camera projection
  path
- environment depth can provide a depth-prior proxy for reprojection checks
- MediaProjection can witness final display appearance, but it is not a raw
  camera source
- depth-assisted alignment should record camera timestamps, depth timestamps,
  predicted display time, projection profile, source-eye mapping, and capture
  timing in one scorecard bundle

The activation ladder for depth-informed alignment should stay diagnostic:

1. native runtime passthrough plus environment-depth mesh or particles
2. native runtime passthrough plus custom camera edge overlay
3. native runtime passthrough plus checker or wipe overlay
4. central custom projection window over native passthrough
5. mostly custom projection with a native passthrough border/reference region
6. full custom projection only after scorecard and operator inspection pass

The main parity profile remains depth-off until a depth-assisted correction has
been shown to reduce reprojection error without adding frame-budget risk.

## Implementation Order

1. Freeze the `camera-stereo-gpu-composite-performance-065` evidence as the
   public correctness baseline and keep future public reports tied to explicit
   run manifests.
2. Add scorecard fields for GPU percentage, tear/stale summaries, device-level
   state, camera permission state, import-cache behavior, descriptor-cache
   behavior, projection status, and fatal/GPU-fault signatures.
3. Split projected draw attribution into shader, border/perimeter, descriptor,
   import, command recording, render-pass/frame-buffer, and submit categories.
4. Add a depth-alignment witness profile that records depth/camera/display
   timing without changing the default direct Camera2 performance profile.
5. Promote only stable, framework-neutral findings into public contracts:
   projection metadata, timestamp pairing, scorecard schema, calibration
   descriptors, and optional adapter hooks.
6. Re-run the direct Camera2 matrix at `0.75` and `0.65` after each render-path
   slice, and keep the release gate on visible stereo, zero CPU uploads, no
   import churn, no app fatal/GPU-fault signatures, no final stale/tear, and
   clear GPU headroom.
7. Keep the fast raw-projection profiles in the catalog as the stable parity
   reference while future work reintroduces public border/feedback styling or
   adds stream-latency compensation.

## Public Acceptance Gate

Do not call the public profile parity-complete until a new Quest run shows:

- visible paired stereo camera projection in headset
- `activeTier=gpu-projected`
- `alignedProjection=true`
- `stereoLayout=Separate`
- paired left/right GPU buffers
- `cpuUploadCount=0`
- no sustained GPU import failures or cache evictions
- no app fatal or GPU-fault signatures
- no final-window stale/tear behavior
- no byte-identical multi-frame screenshot sequence when the profile is expected
  to show live camera content
- projected frame cadence at the target refresh rate
- materially lower GPU load than the current accepted `0.65` baseline
- a matching scorecard that records all normalization axes

Any report that uses downstream target evidence must remain sanitized before it
is copied into this public repository.
