# General Tool Import Audit

This audit records reusable XR tools found in local Makepad/Rusty-XR-family
work and sibling Quest projects. The goal is to extract general contracts and
small testable utilities into Rusty XR without copying downstream app behavior,
framework-heavy widgets, package identity, generated artifacts, or proprietary
SDK code.

## Sources Checked

The audit used local/internal repositories and bureau routing notes as design
references. Public Rusty XR should not name or depend on those downstream
repositories. Use this document as a sanitized backlog for clean-room public
contracts, not as permission to copy private app code.

Source categories reviewed:

- Rust / Makepad / OpenXR Quest shells and earlier Rust XR baselines.
- Unity Quest room mesh, live depth, scan fusion, and TSDF sandboxes.
- Makepad UI, drawing, geometry, platform, and XR-adjacent utilities.
- Machine-level Quest, Android, camera, depth, streaming, and diagnostics notes.

Makepad code is MIT licensed in the inspected checkout. Keep Makepad
attribution when extracting concepts or code. Other local/internal repos should
be used as design references unless a clean public implementation is explicitly
approved.

The broader machine-wide follow-up audit is tracked separately in
[MACHINE_REPO_TOOLING_AUDIT.md](MACHINE_REPO_TOOLING_AUDIT.md).

## Already Started

### XR Canvas And Ray Interaction

Public extraction added:

- `InteractionRay`
- `XrCanvasSurface`
- `XrCanvasHit`

These live in `rusty-xr-contracts` and provide framework-neutral ray-to-canvas
hit testing for panels, hand menus, debug surfaces, and future Makepad adapters.

Next likely public step:

- Add optional scene-viewport ray helpers after the public math layer has a
  deliberate matrix type or an adapter converts framework matrices into plain
  Rusty XR data.

### Hand Menu And Hand Interaction Primitives

Public extraction added:

- `HandMenuAnchor`
- `HandMenuActivation`
- `HandInfluencePoint`

These are intentionally smaller than a full hand-menu widget. They describe
where a menu belongs, how it is activated, and how hand joints can influence
particles, surfaces, deformation, or UI proximity.

Next likely public step:

- Add generic hand-collider primitives (`capsule`, `sphere`, `box`) and a
  joint-to-influence-point helper that derives public influence points from
  `HandJointSnapshot`.

### Sparse Scan And TSDF Surface Contracts

Public extraction added:

- `VoxelCoord3`
- `SparseTsdfSample`
- `SparseTsdfSnapshot`
- `ScanSurfaceSample`
- `ScanFusionStats`

These live in `rusty-xr-sdf` as contracts only. They do not include native
environment-depth acquisition, framework depth calls, GPU fusion kernels, or
downstream scan behavior.

Next likely public step:

- Add a small sparse-TSDF query helper for near-surface candidates and support
  plane summaries before moving any heavier integration or meshing algorithm.

### Plain Stereo Layer And Visual Feedback Border

Public extraction added:

- `StereoMediaLayout`
- `StereoLayerContentMode`
- `PlainStereoLayer`
- `Rect2`
- `VisualFeedbackBorder`
- `VisualFeedbackBorderLayout`
- `FeedbackBorderTuning`
- `VisualFeedbackLayerTuning`
- `StereoLayerPerformanceHints`

These live in `rusty-xr-contracts` as plain data and layout helpers. They cover
source UV selection for mono, side-by-side, top/bottom, or separate-eye media,
aspect-preserving projected-surface layout, and simple rectangular border
segments around a fitted feedback/content surface. They also carry public
border-only tuning values and adapter performance hints for custom stereo and
visual feedback layers.

This extraction intentionally excludes downstream image-processing passes,
effect maps, geometric-effect implementations, scene behavior, and
project-specific shader code. It also excludes native Quest API calls,
MediaProjection, Camera2, OpenXR composition submission, Vulkan hardware-buffer
import, and Makepad widget code.

Next likely public step:

- Add a small public example that computes a 16:9 MediaProjection feedback inset
  and border layout, then emits the rectangles as JSON or render payload data.

## Strong Public Candidates

| Candidate | Source Category | Import Shape | Risk |
| --- | --- | --- | --- |
| Geometry generators | Makepad geometry utilities and local XR mesh helpers | Public `rusty-xr-geometry` helpers for quad, cube, ico-sphere, line/vector, and depth-mesh vertex layouts | Low if kept data-only and attributed |
| Popup/menu models | Makepad popup/window menu concepts | Menu item tree, command IDs, selection actions, keyboard/controller activation contracts | Low if widget rendering stays adapter-only |
| XR selection/router | Local XR scene selection models | Framework-neutral active-panel/activity state model for XR scenes | Low |
| Hand collider primitives | Local hand tracking and room/depth tooling | Capsule/sphere/box hand collider descriptors and snapshot validation | Low/medium; avoid physics-engine dependencies |
| Room mesh snapshots | Local room-mesh and semantic-scan tooling | Room mesh source kind, state, anchor snapshot, semantic snapshot, processed-mesh stats | Low if treated as contracts |
| Scene mesh semantic fallback | Local semantic mesh builders | Plane/box semantic mesh contracts and maybe tiny mesh generators | Low |
| Plain projected stereo media layer | Local custom stereo layer work | Optional adapter around public `PlainStereoLayer`; native renderer provides textures and draw calls | Low if shader/effect stack stays downstream |
| Border tuning and performance hints | Local custom stereo/feedback docs and code | Public scalar tuning structs and performance policies for adapter authors | Low if kept data-only |
| Visual feedback border example | Local composite-feedback and capture docs | Public border/inset layout example for MediaProjection feedback surfaces | Low if kept as geometry/layout only |
| Depth-readback novelty gates | Local TSDF/depth integration experiments | Generic novelty score and readback cadence policy | Medium; constants need public names and tests |
| Surface-net chunk extraction | Local depth debug mesh experiments | TSDF-to-chunked debug mesh extraction over public sparse snapshot | Medium; more algorithmic code and attribution needed |
| Depth support/impact planes | Local depth query experiments | Support-plane and impact-plane summaries for particles/physics | Medium; keep physics adapters optional |
| Scan package manifest | Local capture and scan workflows | JSON-friendly capture package descriptors, stream manifests, frame indexes | Low |

## Defer Or Keep Adapter-Only

- Full Makepad widget implementations: keep these in optional adapters because
  they depend on Makepad's widget tree, live design system, shader language, and
  draw/runtime internals.
- Native OpenXR, Vulkan, Android, Meta SDK, Unity, MRUK, OVR, or `liblsl`
  bindings: keep in downstream app shells or optional adapter crates.
- Physics worker code: useful as a proving path, but the public core should
  expose collision/query contracts first.
- Full depth integration and TSDF worker ownership: import only after the
  public sparse snapshot/query contracts settle and tests cover synthetic
  frames.
- Unclear-license reference code: use as behavioral reference only until
  license and provenance are verified.
- Private rendering stacks, app-specific simulation behavior, private stream
  names, signing, package identities, generated datasets, and project-specific
  shader code.

## Recommended Extraction Order

1. Finish contracts for XR canvas, hand menu, plain stereo/feedback layers,
   room mesh snapshots, and scan package manifests.
2. Add unit-tested geometry generators with Makepad attribution.
3. Add menu/selection contracts that can drive Makepad or another UI adapter.
4. Add sparse TSDF query helpers and support-plane summaries.
5. Add chunked debug-mesh extraction from public sparse TSDF snapshots.
6. Only then consider optional Makepad/OpenXR adapter crates.

This order keeps Rusty XR useful to app shells while preserving the current
public/downstream boundary.
