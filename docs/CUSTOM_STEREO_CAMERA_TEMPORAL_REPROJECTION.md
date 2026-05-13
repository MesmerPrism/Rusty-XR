# Custom Stereo Camera Temporal Reprojection

This document tracks the public Rusty XR plan for smoothing an app-owned
opaque stereo camera projection when camera cadence, camera latency, or the
projected draw path cannot keep up with head motion.

The goal is not to make camera frames newer. The goal is to bound how far the
visible projection changes between submitted XR frames, so fast motion produces
controlled visual lag instead of hard projection jumps.

## Current Baseline

Rusty XR already separates the relevant source classes:

- raw camera frames and metadata for the custom projection path
- runtime environment depth as an optional depth witness
- native passthrough as a compositor-owned layer
- MediaProjection as final-display inspection
- broker H.264 lanes as diagnostic transport/decode paths

The current public implementation has a `gpu-projected` Camera2 path with
paired GPU-imported buffers, per-eye Camera2 metadata, display-eye
screen-to-camera homographies, and render-scale comparison profiles. The
streaming matrix in
[QUEST_STREAMING_DIAGNOSTICS_WORKFLOW.md](QUEST_STREAMING_DIAGNOSTICS_WORKFLOW.md)
shows that the next performance and smoothness target is the shared
metadata-backed projected draw/render path, not Java image acquisition,
MediaCodec decode, or native hardware-buffer handoff.

## New Public Contract Slice

`rusty-xr-contracts` now carries data-only temporal projection contracts:

- `CameraFrameTiming`
- `StereoCameraFramePair`
- `CameraProjectionState`
- `ProjectionTargetState`
- `VisualProjectionState`
- `TemporalProjectionPolicy`
- `TemporalProjectionMode`
- `CameraFrameAdoptionMode`
- `TemporalProjectionEdgeMode`
- `TemporalProjectionMetrics`

These contracts deliberately do not implement Android Camera2, Vulkan imports,
OpenXR submission, depth acquire, shader sampling, or `XR_FB_space_warp`.
Adapters and examples can use them to report policy, state, and scorecard
metrics without changing rendering behavior.

## Target Architecture

```text
stereo camera pair / decoded H.264 pair
        |
camera frame timing and pair metadata
        |
target projection calculation
        |
temporal projection governor
        |
visual projection state
        |
opaque per-eye camera projection shader
        |
optional depth-aware reprojection
        |
optional space-warp motion-vector/depth submission
        |
OpenXR projection layer
        |
scorecard and headset-visible diagnostics
```

The governor separates:

- target projection: the physically current projection for the newest camera
  frame and predicted display pose
- visual projection: the projection actually submitted after smoothing,
  clamping, and frame-adoption policy

User-visible camera sampling should use the visual projection when temporal
smoothing is enabled.

## Cross-Stack Role

Temporal smoothing is shared projection behavior, not a transport feature. The
same policy and scorecard fields should apply to:

- the Rusty XR composite-layer Camera2 projection path
- the Rusty XR broker H.264 existing-stream projection path
- the standalone Makepad stereo projection comparison lane

Meta native passthrough remains a compositor-owned reference/witness stream.
It can be used for headset comparison, environment context, and alignment
checks, but it is not the source texture for the app-owned projection governor
and must not be treated as directly sampleable camera input.

The first useful comparison is:

```text
Rusty XR no-smoothing direct projection
Rusty XR no-smoothing broker/existing-stream projection
Rusty XR screen-motion clamp projection
Makepad no-smoothing projection with the same metadata contract
Makepad screen-motion clamp projection with the same metadata contract
native passthrough as a separate visual witness
```

Every run should record whether it is using raw camera frames, decoded H.264
hardware buffers, native compositor passthrough, or final-display capture. Do
not compare these as interchangeable camera sources.

## Immediate Runtime Plan

Use the following order before Q2Q online transport becomes the main moving
part:

1. Re-run direct and broker-live fast profiles with smoothing off and confirm
   `target_projection_motion_px_p95 == applied_projection_motion_px_p95`,
   residual is zero, held-frame count is zero, and ASW counters are zero.
2. Add `camera-stereo-temporal-pose-clamp-fast075` to prove left/right
   lockstep smoothing state and nonzero residual metrics.
3. Add `camera-stereo-temporal-screen-clamp-fast075` as the main comfort
   profile.
4. Add frame-adoption smoothing with bounded hold time.
5. Add shader-owned edge handling and invalid-UV metrics.
6. Add depth-aware and space-warp probes only after the planar governor is
   measurable.

This keeps the visible renderer problem explicit. More network layers should
not be used to mask projection jumps that also appear in direct local camera
projection.

## Implementation Iterations

### Iteration 1: Contracts And Documentation

Status: implemented.

Scope:

- add the public temporal projection contracts listed above
- document source taxonomy boundaries
- document scorecard fields for target motion, applied motion, residual lag,
  held frames, invalid UVs, edge fill, and optional space-warp counters
- do not change Quest rendering behavior

Validation:

- `cargo test -p rusty-xr-contracts --all-features`
- docs link check
- public boundary scan before any push

### Iteration 2: Metrics-Only Instrumentation

Status: source-implemented for the public composite-layer example; headset
validation is pending.

Scope:

- log stereo pair delta, target projection motion, projection matrix changes,
  invalid UV percentage, and estimated per-frame pixel displacement
- add `TemporalProjectionMetrics` rows to the existing scorecard parser output
- keep visual output identical to the current baseline

Implementation notes:

- the projected Camera2 path now samples a fixed 5 by 5 screen-UV grid per eye
  across consecutive screen-to-camera homographies
- the reusable homography application and stereo projection-motion metric
  helper lives in `rusty-xr-camera-model`, so the core metric calculation is
  host-testable outside the Android renderer
- target and applied projection motion are reported as equal while smoothing is
  disabled
- residual, visual lag, held-frame, crossfade, edge-fill, and space-warp
  counters are reported as zero until those iterations exist
- camera frame age is logged only when the OpenXR predicted-display timestamp
  and camera midpoint timestamp appear to share a plausible clock domain;
  otherwise the field is marked `unavailable`

Acceptance:

- a no-smoothing run reports target and applied projection motion as equal
- scorecard JSON exposes `applied_projection_motion_px_p95`
- headset visual acceptance remains governed by the existing projection gate

### Iteration 3: Pose-Delta Clamp

Scope:

- add runtime profile `camera-stereo-temporal-pose-clamp-fast075`
- add `rustyxr.cameraTemporalProjectionEnabled=true`
- add `rustyxr.cameraTemporalMode=pose-delta-clamp`
- add `rustyxr.cameraTemporalStereoLockstep=true`
- clamp angular and linear changes in pose/view space
- keep left and right eyes lockstep-smoothed with one shared coefficient

Acceptance:

- stereo eyes never advance with different smoothing coefficients
- residual lag is visible in metrics
- `off` and baseline profiles remain available

### Iteration 4: Screen-Motion Clamp

Scope:

- add runtime profile `camera-stereo-temporal-screen-clamp-fast075`
- add `rustyxr.cameraTemporalMode=screen-motion-clamp`
- start with `rustyxr.cameraTemporalMaxPixelsPerFrame=18`
- start with `rustyxr.cameraTemporalCatchupHalfLifeMs=50`
- start with `rustyxr.cameraTemporalMaxVisualLagMs=120`
- sample a small grid per eye to estimate target-vs-visual screen motion
- cap applied motion in pixels per display frame

Acceptance:

- `applied_projection_motion_px_p95` stays under the configured cap except
  during explicit reset/discontinuity events
- `target_projection_motion_px_p95` may exceed the cap, proving the clamp is
  doing work
- fast yaw produces smooth catch-up rather than jumps

### Iteration 5: Frame Adoption Smoothing

Scope:

- add `hold-until-smooth`, `short-crossfade`, and `velocity-aware` adoption
  modes
- start with `rustyxr.cameraFrameAdoptionMode=hold-until-smooth`
- start with `rustyxr.cameraFrameAdoptionMaxJumpPx=24`
- start with `rustyxr.cameraFrameAdoptionMaxHoldMs=80`
- report held frame count, max hold duration, and crossfade count

Acceptance:

- high head-screen velocity prefers holding or capped adoption
- low head-screen velocity adopts new accepted pairs quickly
- max visual lag and max frame age policies bound lingering

### Iteration 6: Edge Handling And Debug Views

Scope:

- add temporal edge modes such as `clamp`, `clamp-soft`, and `fade-invalid`
- report invalid-UV and edge-fill percentages
- add residual and camera-age debug overlays in the example renderer

Acceptance:

- edge invalid regions are visible in diagnostics instead of hidden
- lingering near source edges stretches or fades smoothly rather than snapping

### Iteration 7: Depth-Aware Reprojection

Scope:

- use runtime environment depth when fresh and valid
- fall back to planar/screen clamp when depth is stale or unavailable
- reduce update confidence near depth discontinuities

Acceptance:

- depth-aware mode records depth age and fallback state
- stale or unavailable depth does not block the planar path
- depth work remains disabled in baseline projection profiles

### Iteration 8: Space-Warp Capability Probe

Scope:

- probe `XR_FB_space_warp`
- report recommended motion-vector dimensions and depth image support
- do not submit motion vectors yet

Acceptance:

- capability runs are scorecard-readable
- lack of support downgrades cleanly without changing camera projection

### Iteration 9: Planar Space-Warp Experiment

Scope:

- generate motion vectors from previous and current visual projection state
- default to capped visual motion rather than raw target motion
- skip submission on discontinuities such as pose reset, source-eye mapping
  change, texture transform change, or excessive frame age

Acceptance:

- generated/skipped/clamped frames are counted
- artifacts are evaluated against the no-space-warp temporal governor

### Iteration 10: Depth-Aware Capped Space-Warp

Scope:

- combine depth-aware reprojection with capped motion-vector generation
- keep the feature opt-in until headset evidence proves it helps

Acceptance:

- applied motion and stereo divergence improve or stay bounded
- halo, shimmer, invalid UVs, and edge-fill costs are visible in metrics

## Initial Runtime Knobs

Planned public runtime keys:

```text
rustyxr.cameraTemporalProjectionEnabled=true
rustyxr.cameraTemporalMode=screen-motion-clamp
rustyxr.cameraTemporalMaxPixelsPerFrame=18
rustyxr.cameraTemporalMaxAngularDegPerFrame=1.25
rustyxr.cameraTemporalMaxLinearMetersPerFrame=0.012
rustyxr.cameraTemporalCatchupHalfLifeMs=50
rustyxr.cameraTemporalMaxVisualLagMs=120
rustyxr.cameraTemporalMaxCameraFrameAgeMs=120
rustyxr.cameraFrameAdoptionMode=hold-until-smooth
rustyxr.cameraFrameAdoptionMaxJumpPx=24
rustyxr.cameraFrameAdoptionMaxHoldMs=80
rustyxr.cameraTemporalStereoLockstep=true
rustyxr.cameraTemporalEdgeMode=clamp-soft
```

These keys are not release defaults until the renderer and scorecard slices
exist. The public `TemporalProjectionPolicy::CONSERVATIVE_SCREEN_CLAMP`
constant records the intended first experiment without changing runtime
behavior.

## Scorecard Fields

The critical field is:

```text
applied_projection_motion_px_p95
```

Additional planned fields:

```text
camera_frame_age_ms_avg
camera_frame_age_ms_p95
depth_frame_age_ms_avg
stereo_pair_delta_ms_avg
target_projection_motion_px_avg
target_projection_motion_px_p95
applied_projection_motion_px_avg
applied_projection_motion_px_p95
projection_residual_px_avg
projection_residual_px_p95
visual_lag_ms_avg
visual_lag_ms_p95
held_frame_count
held_frame_duration_ms_max
frame_crossfade_count
invalid_uv_px_percent
edge_fill_px_percent
asw_enabled_frame_count
asw_skipped_frame_count
motion_vector_max_px
motion_vector_clamped_count
```

## Non-Goals

This plan does not try to:

- create newer camera frames
- clone native compositor passthrough
- treat MediaProjection as a camera source
- replace the stereo camera provider
- replace MediaCodec
- make fast-motion projection physically exact

The tradeoff is explicit: controlled lingering and measured residual lag are
preferred over visible projection jumps.

## References

- OpenXR `XrCompositionLayerSpaceWarpInfoFB`:
  <https://registry.khronos.org/OpenXR/specs/1.0/man/html/XrCompositionLayerSpaceWarpInfoFB.html>
- OpenXR `XrSystemSpaceWarpPropertiesFB`:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/XrSystemSpaceWarpPropertiesFB.html>
- OpenXR `XrEnvironmentDepthImageMETA`:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/XrEnvironmentDepthImageMETA.html>
- OpenXR `xrAcquireEnvironmentDepthImageMETA`:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/xrAcquireEnvironmentDepthImageMETA.html>
- Meta Passthrough Camera API overview:
  <https://developers.meta.com/horizon/documentation/unity/unity-pca-overview/>
- Unity URP Application Spacewarp:
  <https://docs.unity.cn/6000.2/Documentation/Manual/xr-graphics-spacewarp.html>
