# Projection Coordinate Handoff - 2026-05-19

This is the public handoff for the screen-space, live Camera2, passthrough,
environment-depth, and later blur work. It is intentionally coordinate-first:
blur remains blocked until the raw projection contract is stable across the
active lanes.

## Status At Handoff

| Work item | Status | Handoff rule |
| --- | --- | --- |
| Freeze synthetic baseline | Done; keep as regression gate | Do not retune synthetic scale or flips unless a fresh regression contradicts the logged renderer contract. |
| Promote contract to live Camera2 | Implemented for direct and broker paths | Use explicit `physical-camera` metadata, source size, source UV, texture/upload transform, projection-area UV, screen UV, and renderer-authored expected footprint. |
| Passthrough underlay witness | Captured as a physical witness | Native passthrough is useful for visual alignment, but it is not the app-owned coordinate source of truth. |
| Depth / particle world-space lane | Contract artifact exists | Use `rusty.xr.depth_world_space_contract.v1` as the world-space baseline, then compare it with live Camera2 and passthrough evidence. |
| World-space quad vs direct per-eye shader | Reconciliation documented | Treat the direct shader as the world-space path collapsed only when the named stages match. |
| Blur | Still blocked | Blur consumes the accepted projection contract; it must not discover or hide coordinate errors. |

## Required Reads

Read these in order before changing geometry:

1. [Projection coordinate space ledger](PROJECTION_COORDINATE_SPACE_LEDGER.md)
2. [Synthetic projection coordinate alignment plan](SYNTHETIC_PROJECTION_COORDINATE_ALIGNMENT_PLAN.md)
3. [Quest raw camera stack alignment workflow](QUEST_RAW_CAMERA_STACK_ALIGNMENT_WORKFLOW.md)
4. [Environment depth particle anchoring](ENVIRONMENT_DEPTH_PARTICLE_ANCHORING.md)
5. [World-space quad and direct shader reconciliation](WORLD_SPACE_QUAD_DIRECT_SHADER_RECONCILIATION.md)
6. [Screen-space and blur alignment workflow](SCREEN_SPACE_AND_BLUR_ALIGNMENT_WORKFLOW.md)

For Makepad Android validation, use the public wrapper build path described in
[Android and Quest APK building](ANDROID_QUEST_APK_BUILDING.md). A plain Android
target `cargo check` is not the acceptance gate for the generated Makepad APK.

## What Changed In This Session

Synthetic baseline:

- `camera-matched` broker synthetic lanes now emit renderer-authored expected
  source-valid footprints. Analyzer-derived homography boxes are evidence and
  model checks only.
- `full-frame-diagnostic` uses the renderer-authored visible full-frame
  envelope for placement parity, so disconnected diagnostic bands do not create
  false center-Y failures.
- Makepad's recurring Y issue is resolved as two named conventions: source
  raster handling in the projection plan, and CPU-YUV sampler-origin conversion
  in the texture sampling layer. Do not relabel this as a generic manual flip.

Live Camera2 and passthrough:

- Direct and broker live Camera2 lanes use explicit `physical-camera` geometry
  instead of falling back to a synthetic head-anchored profile.
- Broker live Camera2 is the cleaner machine-checkable path because it carries
  the same stream-header metadata into each renderer.
- Opacity-zero, border-visible passthrough-underlay runs are physical witnesses
  only. They can expose real-world disagreement, but the owning coordinate
  source remains the run manifest, metadata, renderer logs, and transform rows.

Depth/world-space:

- The depth/particle baseline is now represented by
  `rusty.xr.depth_world_space_contract.v1`.
- The accepted chain is:

```text
environment-depth UV
-> depth view ray
-> metric depth-view point
-> app reference-space point
-> render-eye view
-> screen
```

- Infinite runtime far planes are represented explicitly as `far_z_infinite`;
  do not replace them with fake numeric far distances.

Makepad broker path:

- The broker physical/camera-matched Makepad path now consumes stream-header
  camera geometry instead of building a synthetic head-anchored preview plan.
  This fixed the smaller broker footprint relative to direct Camera2.
- Full-frame broker was already a separate projection-surface contract and was
  not the source of that Makepad scale problem.

Analyzer and suite semantics:

- The blue overlay means source-content envelope. It excludes diagnostic matte
  and is not the render surface, full eye frame, or largest connected component.
- Projection-area offset signs are now suite-level stable: positive X moves
  right and positive Y moves down in display/screenshot coordinates. HWB is the
  reference convention; GL/OES and Makepad normalize their backend-specific
  signs at their owning boundaries.
- The device workflow now distinguishes passive state watching from an active
  power/proximity watchdog. Long camera matrices should explicitly start and
  stop the active watchdog. If a local coordination board is used, the watchdog
  should be visible as non-exclusive keep-awake state, while actual installs,
  launches, screenshots, and ADB/log capture remain exclusive device work.

## Current Known Gaps

- A joined projection/depth comparator now exists at
  `tools/quest-camera-profile/Build-ProjectionDepthComparison.py`. A patched
  on-device comparison closed the OpenXR/reference-space logging gap for the
  live broker Camera2 lanes: projection rows now carry reference space, display
  time, and per-eye render pose/FOV, and the broker rows join as `ready` against
  a ready depth/world-space baseline. Direct Camera2 and opacity-zero
  passthrough witness rows still report `needs-evidence` from analyzer-only
  physical-target orientation ambiguity.
- Direct Camera2 physical-target runs can be visually useful while still
  reporting evidence ambiguity, because a real room target does not provide the
  same synthetic marker certainty as broker synthetic.
- Frame-level stereo sync diagnostics are still weak in the online relay path:
  byte counts and lane durations are available, but per-frame PTS, frame index,
  encoder input timestamp, relay receive timestamp, decoder output timestamp,
  and render-adoption timestamp are still needed.
- Any future architecture-specific manual adjustment must be named at its
  owning layer: source metadata, texture/upload convention, projection-area
  mapping, OpenXR reference-space geometry, backend viewport convention, or
  analyzer evidence.

## Next Agent Plan

1. Start with a clean read-only audit: check `git status`, read the required
   docs above, and inspect the latest local evidence index if one exists.
2. Keep the synthetic broker gates as regression tests. Run them only to catch
   regressions before changing live/depth behavior.
3. Keep the fresh physical/depth comparison matrix as the current baseline:
   broker Camera2 joins cleanly with depth/world-space, while direct Camera2
   and passthrough-underlay remain physical witnesses with analyzer-evidence
   ambiguity.
4. For every lane, preserve the same fields: source size, crop or valid rect,
   texture/upload transform, source UV, projection-area UV, screen UV,
   renderer-authored expected footprint, OpenXR view pose/FOV, and screenshot
   evidence.
5. When collecting the next physical pass, join projection-coordinate contracts
   with depth-world-space contracts into a single comparison artifact. The first
   divergent named stage owns the fix.
6. Only after live Camera2, passthrough witness, and depth/world-space evidence
   agree under the same contract should the blur workflow resume.

## Data Pipeline Improvements

Add these before the next long remote or physical-camera session:

- projection-coordinate contract rows must retain OpenXR app/reference space,
  display time, and per-eye render pose/FOV so live Camera2 can be compared to
  the depth/world-space baseline without assigning the gap to missing runtime
  geometry evidence;
- per-frame left/right frame id, source timestamp, encoder input timestamp,
  relay receive/send timestamp where relevant, decoder output timestamp, and
  render-adoption timestamp;
- explicit stereo pair id and tolerated left/right skew;
- source crop/valid rect and texture transform in every renderer log, even when
  the value is identity;
- a run manifest that records requested values, resolved values, defaults used,
  and the owner of each field;
- analyzer outputs that keep source content, render surface, expected
  source-valid footprint, intended projection mask, and invalid-source fill as
  separate boxes;
- power/proximity/watchdog state snapshots before every mode, plus a clear flag
  for passive watcher versus active enforcer.
