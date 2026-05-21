# Projection Coordinate Space Ledger

This ledger is the public source-of-truth scaffold for camera projection,
environment-depth particles, synthetic alignment, and later blur diagnostics.
Use it before changing renderer tuning, analyzer thresholds, or blur settings.

The immediate rule is coordinate-first: blur work resumes only after the raw
projection lanes agree on source geometry, coordinate ownership, and evidence
logging across Vulkan/HWB, GL/OES, and Makepad CPU-YUV.

## Primary Sources

These are the external source surfaces this ledger depends on. Keep local
renderer behavior anchored to these contracts, then document every app-level
assumption separately.

- Meta Passthrough Camera API overview:
  <https://developers.meta.com/horizon/documentation/unity/unity-pca-overview/>
- Android Camera2 `CameraCharacteristics`:
  <https://developer.android.com/reference/android/hardware/camera2/CameraCharacteristics>
- Android `SurfaceTexture.getTransformMatrix(...)`:
  <https://developer.android.com/reference/android/graphics/SurfaceTexture#getTransformMatrix(float[])>
- OpenXR `xrLocateViews`:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/xrLocateViews.html>
- OpenXR `XrView`:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/XrView.html>
- OpenXR `XrFovf`:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/XrFovf.html>
- OpenXR `XrReferenceSpaceCreateInfo`:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/XrReferenceSpaceCreateInfo.html>
- OpenXR `XR_META_environment_depth`:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/XR_META_environment_depth.html>
- OpenXR `XrEnvironmentDepthImageMETA`:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/XrEnvironmentDepthImageMETA.html>
- OpenXR `XrEnvironmentDepthImageViewMETA`:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/XrEnvironmentDepthImageViewMETA.html>

## Stop Lines

Do not use blur, color effects, or physical passthrough alignment as a
coordinate probe. They consume the accepted projection model; they do not
define it.

Before blur resumes, each raw lane must provide a projection-coordinate
contract for the current run:

- `vulkan-hwb-broker-h264-raw`
- `gles-oes-broker-h264-raw`
- `makepad-cpuyuv-broker-h264-raw`
- direct Camera2 variants when they are part of the run

The contract must prove both synthetic geometry profiles:

- `full-frame-diagnostic`: source raster is projection-surface diagnostic
  content and must render upright with full valid coverage.
- `camera-matched`: source raster is synthetic, but its metadata describes the
  selected camera-shaped projection footprint.

If `full-frame-diagnostic` passes and `camera-matched` fails, the broker,
decoder, and basic texture path are not the first suspects. Debug source to
projection mapping, projection-area normalization, source crop, and per-eye
homography stages first.

## Current Evidence Snapshot

Initial completed synthetic evidence on 2026-05-19 has this status:

- `camera-matched`: all three broker lanes now emit renderer-authored expected
  source-valid footprints with
  `expectedSourceValidFootprintSource=renderer-authored`. The analyzer projects
  those display-eye screen-UV rectangles into screenshot pixels and keeps its
  `screen_to_camera` derivation only as a model comparison.
- `full-frame-diagnostic`: all three broker lanes have `ready`
  projection-coordinate contracts, explicit source size, valid source UV rect,
  metadata readiness, all four homography stages, upright orientation, full
  valid coverage, and cross-lane footprint parity.
- Makepad's recurring Y issue is resolved as two named conventions, not as a
  hidden manual flip: broker top-left raster metadata is applied at the
  projection-plan/source-raster boundary, and Makepad CPU-YUV then converts the
  top-left raster into the backend sampler origin with
  `sourceSampleYFlip=1.0` and a sampler-origin reason.
- The previous full-frame center-Y failure was an analyzer measurement problem:
  the strict dense-component union under-measured HWB's darker top diagnostic
  band. Full-frame diagnostics now use the renderer-authored full-frame intent
  to measure the visible stimulus envelope rather than a single dense
  checkerboard component.

Later 2026-05-19 work promoted the same contract beyond synthetic evidence:

- live direct and broker Camera2 now use explicit source/content geometry
  instead of synthetic profile fallback;
- the broker live Camera2 path carries stream-header geometry through HWB,
  GL/OES, and Makepad, and is the cleanest machine-checkable live source path;
- opacity-zero, border-visible passthrough-underlay runs were captured as
  physical witnesses, not coordinate authorities;
- the depth/particle world-space path now emits
  `rusty.xr.depth_world_space_contract.v1` records;
- the world-space quad/direct shader equivalence conditions are documented in
  the reconciliation artifact;
- Makepad broker physical/camera-matched scale now comes from stream-header
  camera geometry, not a synthetic head-anchored preview plan;
- suite-level projection-area signs are stable: positive X moves right and
  positive Y moves down in display/screenshot coordinates.

The 2026-05-20 focused synthetic offset probes tightened that last rule. HWB,
GL/OES, and Makepad now log renderer-authored projection-area target rects and
centers in `display-eye-screen-uv`. Zero-offset, positive-Y, and small
positive-X broker-synthetic runs all report `ready` contracts with zero gaps
and passing cross-lane footprint parity. Makepad still normalizes a legacy
native horizontal property at the launcher boundary, but its vertical
projection-area offset is already the public positive-Y-down convention.

The 2026-05-21 native-passthrough replay pass adds an explicit HWB response
model to that contract. Vulkan/HWB final-status rows now distinguish the
projection-area target rect from the predicted offset response with
`projectionAreaOffsetResponseModel=screen_uv_delta_equals_offset_uv_div_projectionAreaScaleUv`
and per-eye `*ProjectionAreaOffsetResponseUv` fields. Treat target rect,
response model, and observed screenshot motion as separate evidence layers.
Those response fields are display-eye screen UV before runtime mirror capture;
ADB/Meta screenshot pixels can include compositor distortion and must remain a
measured witness until the run also logs an eye-UV-to-screenshot mapping.

The 2026-05-21 display-eye UV fiducial probe closes that evidence gap for the
current mirror-capture path. The Vulkan/HWB composite renderer can render a
`display-eye-uv-fiducial` diagnostic through the same OpenXR submission path.
It marks known `projection_screen_uv_base` positions in
`display-eye-screen-uv`, logs
`displayEyeUvFiducialSchema=rusty.xr.display_eye_uv_fiducial.v1`, and lets
`Analyze-DisplayEyeUvMapping.py` fit the observed mirror screenshot mapping.
The first measured run found all six fiducials in both eyes. A global affine
fit was useful as a witness but left residuals in the tens of pixels, and the
centerline segment slopes differed by roughly 5-8% across the sampled
0.25/0.50/0.75 marker positions. Center-cross alignment should therefore use
the logged near-center finite-difference mapping around the green center marker.
If a projection-area offset response still disagrees after that local mapping,
keep the next owner on projection-area content mapping or projection geometry
rather than treating mirror screenshot pixels as a global linear UV ruler.
The companion `projection-content-uv-fiducial` diagnostic renders the same
markers in `full_frame_content_uv`, after projection-area offset and area
mapping. Use it to prove whether a residual belongs to the post-offset content
path before tuning projection geometry again.

The 2026-05-20 physical-target matrix extends that contract to native
passthrough center-cross alignment. All six live Camera2 lanes classify as
`ready` when compared against an opacity-zero passthrough witness, using the
green center cross as the primary evidence target and keeping full-feature
correlation secondary because the native passthrough compositor can apply
peripheral warp that the custom projection is not expected to reproduce.

The resolved mismatch owners are:

- GL/OES opacity and brightness mismatch: texture/upload and color-transfer
  convention. Source alpha output must premultiply RGB before OpenXR
  source-alpha composition. External OES camera/video RGB is a separate color
  layer: the renderer logs and applies `sourceColorTransform=srgb-to-linear`
  before generic camera color controls, and prefers an sRGB OpenXR GLES
  swapchain when the runtime exposes one.
- HWB and GL/OES per-eye center offsets: projection-area mapping. The common
  projection-area offset is only a fallback; HWB and GL/OES can accept per-eye
  left/right X and Y offsets.
- False GL/OES right-eye row selection: analyzer evidence. The green-cross
  detector chooses a strong axis near the green target median instead of the
  raw maximum row or column when other green target features are present.
- Full-frame GL/OES broker guide-only segmentation: analyzer evidence. When a
  renderer-authored full-frame source-valid footprint leaves no meaningful red
  invalid-fill region, the strict analyzer may use the cyan/yellow diagnostic
  guide signal plus the logged full-frame footprint. That is still diagnostic
  mask evidence, not a visible-content fallback.

The follow-up source-sampling audit keeps that owner model explicit.
Broker-synthetic runs place the dominant green feature at the same rows across
HWB, GL/OES, and Makepad. A later source-agnostic full-frame run extends that
result to actual broker Camera2 data: HWB and Makepad report dominant green
rows at left `657` / right `619`, GL/OES reports left `661` / right `625`,
and cross-lane valid-projection footprint parity passes with no contract gaps.
Direct Camera2 full-frame checks for HWB and GL/OES also pass with the same
rows. A post-rebuild focused matrix measured HWB at left `658` / right `619`
and GL/OES at left `661` / right `624`, with zero contract gaps and passing
cross-lane valid-projection footprint parity. That is evidence that direct and
broker camera lanes can share the same explicit full-frame rendering contract.
Makepad now has the same direct full-frame selector wired into the public
wrapper and renderer metadata path; an on-device Makepad direct rerun still
needs to confirm that selector against the already-passing broker-camera
full-frame lane.

The older physical-camera direct run remains useful as a negative control.
HWB landed the dominant green feature at left `828` / right `815` while GL/OES
landed at left `699` / right `651` and Makepad at left `697` / right `648`.
The named first divergent layer is the physical-camera source-sampling /
texture-upload contract: HWB sampled hardware-buffer input through
`cameraTextureTransformFlags`, while GL/OES used the Android
`SurfaceTexture` matrix and Makepad used its CPU-YUV
`source_sample_uv` convention. Do not repair that split with a lane-specific
projection-area Y offset.

Blur remains a downstream consumer of the stable projection contract. Do not
use blur, color effects, source crops, or renderer-local hidden offsets to
discover or hide coordinate errors.

## Coordinate Domains

Every renderer change should name the domain it reads, writes, or converts.
The same term must not mean different things in different lanes.

| Domain | Owner / source | Units and origin | Required next-step evidence |
| --- | --- | --- | --- |
| Camera2 active-array pixels | Android Camera2 metadata | Sensor pixel coordinates. Origin and crop are Camera2-defined. | Log active-array size, selected stream size, crop/valid rect when available. |
| Delivered image pixels | Camera2, MediaCodec, broker synthetic source, or CPU YUV frame | Raster pixel coordinates for the actual buffer delivered to the renderer. | Log width, height, format, row stride when relevant, timestamp domain, and source-eye identity. |
| Decoded texture UV | Hardware buffer, OES texture, or CPU-upload texture | Normalized texture coordinates after decoder/upload ownership. | Log texture size, valid UV rect, OES transform matrix, source-sampling transform, Y flip when present, and crop transform. |
| Camera/source UV | Camera projection model | Normalized camera image sample domain after source orientation and valid rect are applied. | Log `surface_to_camera`, `screen_to_camera`, source invalid-fill policy, and source rect clipping. |
| Content surface UV | Rusty XR projection model | Normalized coordinates on the intended camera/content surface. | Log content rect, content aspect, projection profile, and any overscan or scale. |
| Full submitted surface UV | Renderer/OpenXR swapchain image | Normalized coordinates over the full submitted eye surface or layer image. | Log full surface size, viewport, scissor, matte/border policy, and full-to-content mapping. |
| Projection-area UV | Rusty XR projection-area mask | Normalized intended visible camera area inside the submitted surface. | Log projection-area center, radius/scale, corner radius, opacity, target screen-UV rect/center, and invalid-region policy. |
| Display-eye screen UV | Final per-eye submitted image before screenshot | Normalized screen-space domain per eye. | Log `surface_to_screen`, `screen_to_surface`, renderer-authored expected source-valid box, observed box, per-eye tokens, and display-eye UV fiducials when comparing against mirror screenshot pixels. |
| OpenXR view tangent space | OpenXR view pose/FOV | Eye-local rays derived from `XrView.pose` and `XrView.fov`. | Log display time, reference space, per-eye pose, and FOV angles. |
| OpenXR app reference space | App-chosen `LOCAL`, `STAGE`, or other reference space | Meters, runtime-defined origin for the chosen reference space. | Log reference-space type, pose composition, and any head-anchored surface pose. |
| Environment-depth UV | `XR_META_environment_depth` swapchain image | Normalized depth-image coordinates per depth view. | Log depth image index, near/far meters, depth view pose/FOV, timestamp when available, and hand-removal state if used. |
| Vulkan clip / NDC / viewport | Renderer backend | Clip/NDC after app projection, then viewport pixels. Vulkan viewport Y conventions can differ from shader math assumptions. | Log projection matrix convention, viewport dimensions, manual FOV projection, and any Y flip. |
| Screenshot pixels | ADB/HzDB/MediaProjection or analyzer input | Captured final-display pixels. | Use only as evidence. Record capture method, freshness, segmentation thresholds, boxes, contact sheet, and any measured display-eye-UV-to-screenshot mapping. |

## Source-Of-Truth Priority

For coordinate alignment, resolve disputes in this order:

1. Stream or stimulus metadata: raster size, aspect, orientation, source-eye
   mapping, valid source UV, and geometry profile.
2. Suite request or run manifest: requested projection area, border/matte
   policy, processing layer, opacity, scale, and radius.
3. Resolved app runtime logs: values applied after launch extras, catalog
   defaults, Android properties, and native defaults merge.
4. Shader or renderer transform logs: homography rows and stage names.
5. Screenshot analyzer measurements: observed final-display evidence only.

Analyzer output can falsify a run. It must not become the coordinate source of
truth unless the manifest, runtime logs, and transform logs already agree.

The detailed rule for comparing the reference-space quad path to the direct
per-eye shader path is tracked in
[WORLD_SPACE_QUAD_DIRECT_SHADER_RECONCILIATION.md](WORLD_SPACE_QUAD_DIRECT_SHADER_RECONCILIATION.md).
Use that document when deciding whether a mismatch is a real architecture
convention, a projection-plan error, or analyzer-only evidence.

## Core Transform Chains

### World-Space Plane Preview

The intuitive camera-feed-on-a-quad path is:

```text
Camera2 metadata + selected source frame
-> camera/source UV
-> head-anchored preview surface in app reference space
-> OpenXR per-eye view projection
-> submitted eye image
```

The camera metadata does not directly say how large to draw a world-space
quad. The app chooses a projection profile. The current plane model is:

```text
half_height = tan(preview_fov_y / 2) * depth_meters * overscan
half_width = half_height * aspect
```

For camera-content lanes, `aspect` is the delivered/source content aspect
(`contentWidth / contentHeight`). Do not substitute the OpenXR display-eye FOV
aspect here: that changes the physical canvas dimensions, which can create
stretch and stereo-convergence errors before any source-camera sampling math
runs. The Vulkan logs expose this as
`projectionSurfaceAspectContract=content_frame_aspect_not_display_eye_fov` plus
per-eye `ProjectionSurfaceAspect` fields.

This can look good because OpenXR handles stereo display of the surface once it
is placed in reference space, and Camera2 intrinsics/extrinsics provide a
plausible source sample for that surface. It is still a plane approximation:
it is exact only for the chosen surface plane. Matching a whole room requires
depth-assisted geometry or an accepted approximation policy.

For the Vulkan composite example, `rustyxr.cameraProjectionMode=world-canvas`
is the diagnostic lane for this model. It draws the chosen head-anchored
surface as real OpenXR quad geometry and samples the source with
`surface_to_camera` rows. Use it before another fullscreen-shader tuning pass
when headset review says the source transport is coherent but the visible
projection lands at the wrong distance or has stretch/convergence errors.
MediaProjection can capture this rendered canvas as a final-display witness,
but it remains screenshot evidence; the named source of truth is still the
surface depth/FOV/overscan/aspect plus the logged OpenXR view state.

The current world-canvas comparison values have different authority levels:

- `cameraProjectionMode=world-canvas` is a diagnostic rendering choice.
- `projectionDepthMeters=0.75` is a named historical starting depth, not a
  measured passthrough or physical-screen distance. Depth-1.0 comparison
  profiles exist to test the default one-meter surface without changing any
  other geometry field.
- `cameraPreviewFovYDegrees=60` is the virtual surface angular height. It is
  not the Camera2 optical FOV and not the OpenXR display FOV.
- `cameraRawOverlayOverscan=1.06` is a small explicit pad around the
  camera-content surface. It must stay logged as a surface-coverage field, not
  as a hidden alignment offset.
- source frame width/height and per-eye OpenXR view pose/FOV are runtime facts.
  They are read from the delivered camera frame metadata and current `XrView`s,
  not copied from the catalog.

The custom camera-footprint path should be equivalent to the world canvas when
it uses the same depth/FOV/aspect/overscan values and the
`raw-projection-camera-footprint-underlay-unorm` preset to algebraically
collapse the surface into the fullscreen shader:

```text
display-eye screen UV
-> screen_to_surface
-> surface_to_camera
-> source texture sample
```

That equivalence is different from a full-feed/full-screen control. A
full-feed control is useful for source transport parity, but it is not the
custom passthrough replacement footprint because the raw Camera2 image does not
cover the same field of view as native passthrough. Outside the valid projected
camera footprint, use an explicit policy such as passthrough underlay,
transparent alpha, solid fill, matte, or documented border.

### Direct Per-Eye Shader Projection

The optimized per-eye shader path should be treated as the algebraic collapse
of the visible plane path:

```text
display-eye screen UV
-> preview/content surface UV
-> camera/source UV
-> texture sample or invalid-source fill
```

It is useful once correct, but it is harder to debug because the intermediate
world-space surface is hidden. Every lane must log the named stages:

- `surface_to_screen`
- `screen_to_surface`
- `surface_to_camera`
- `screen_to_camera`
- source sampling contract and texture transform
- valid source UV rect
- projection-area mask

The source sampling boundary is separate from projection-area mapping. Lanes
must report `sourceUvContract`, `sourceHomographyOutputUv`,
`sourceSampleInputUv`, `sourceSampleTransformStage`, `sourceSampleTransform`,
`sourceSampleTransformOwner`, `sourceSampleTransformApplied`,
`sourceSampleOutputUv`, `sourceSamplerUvOrigin`, `sourceSamplerYAxis`,
`sourceTextureTransformStage`, and `sourceTextureTransformOwner`. HWB owns this
through hardware-buffer sampler flags, GL/OES through Android
`SurfaceTexture` transform semantics, and Makepad through its CPU-YUV upload
and shader `source_sample_uv` convention. Those are architecture differences;
projection-area offsets are not a substitute for them.

OpenXR reference-space, predicted-display-time, per-eye render pose, and per-eye
FOV fields must be emitted in a short dedicated projection-contract marker.
Do not repeat those fields in already-long renderer status lines; Android
logcat can truncate the right-eye tail and turn valid renderer state into a
false analyzer gap.

If the stages disagree, fix the first divergent stage. Do not compensate for a
bad source rect with a projection-area scale, or for a bad projection area with
a source crop.

The accepted equivalence conditions are listed in
[WORLD_SPACE_QUAD_DIRECT_SHADER_RECONCILIATION.md](WORLD_SPACE_QUAD_DIRECT_SHADER_RECONCILIATION.md).
In short, the direct shader is the quad path only when it uses the same
reference-space surface, source-camera model, texture/upload convention,
projection-area mask, backend viewport convention, and render-eye pose/FOV.

### Environment-Depth Particles

The depth-particle path is the current world-space-first baseline:

```text
environment-depth UV + depth sample
-> depth view ray from XrEnvironmentDepthImageViewMETA.fov
-> metric point using near/far depth range
-> app reference-space point using the depth view pose
-> current render-eye view
-> clip/screen
```

This is not a pre-scanned room mesh. It is a live runtime depth texture path.
Scene-owned particle mapping then gives accepted samples stable identity in
local/reference-space cells instead of treating screen-raster slots as particle
identity.

The reusable lesson is that world data should become app reference-space
geometry before the final per-eye render whenever possible. Direct per-eye
camera sampling is a valid optimized path, but it must remain explainable as a
collapse of the same reference-space geometry.

Depth/world-space runs now have a matching contract artifact. The public schema
id is `rusty.xr.depth_world_space_contract.v1`, exposed by
`rusty-xr-contracts::DepthWorldSpaceContract`; the public no-hardware example
is:

```powershell
cargo run -p rusty-xr-contracts --example depth_world_space_contract --features serde
```

Quest log runs can be collapsed into
`depth-world-space-contracts.jsonl` plus
`depth-world-space-contract-summary.json` with:

```powershell
python .\tools\quest-camera-profile\Build-DepthWorldSpaceContract.py <run-root>
```

After live Camera2, passthrough-underlay, and depth runs have emitted JSONL
contracts, join them into a lane/eye comparison artifact with:

```powershell
python .\tools\quest-camera-profile\Build-ProjectionDepthComparison.py `
  --camera-contracts <live-direct-projection-coordinate-contracts.jsonl> `
  --camera-contracts <live-broker-projection-coordinate-contracts.jsonl> `
  --passthrough-contracts <passthrough-projection-coordinate-contracts.jsonl> `
  --depth-contracts <depth-world-space-contracts.jsonl> `
  --out-dir <joined-comparison-output>
```

The joined artifact keeps passthrough as a physical witness and assigns every
gap to one owner layer: source metadata, texture/upload convention,
projection-area mapping, OpenXR reference-space geometry, backend viewport
convention, or analyzer evidence.

The accepted depth contract must name the owner of each stage:

- `DepthUvToDepthViewRay`: runtime depth-view FOV.
- `DepthViewRayToMetricPoint`: near/far depth conversion to meters.
- `DepthViewPointToReferenceSpace`: depth view pose composed into app
  reference space.
- `ReferenceSpacePointToRenderEye`: current render-eye pose.
- `RenderEyePointToScreen`: current render-eye FOV and backend projection
  convention.

Use that record as the world-space baseline when comparing live Camera2 and
passthrough-underlay evidence. If a direct per-eye shader diverges from the
world-space path, fix the first named stage that differs; do not add a
renderer-local offset without naming the stage that owns it.

## Three-Lane Rules

### Vulkan/HWB

Trace these before changing a visual parameter:

- source mode and geometry profile
- selected camera/source size and valid source UV rect
- `cameraProjectionScale`
- `projectionDepthMeters` (meters to the head-anchored projection surface;
  `cameraProjectionScale` is not a depth fallback)
- `projectionAreaScaleUv` and the other shared `projectionArea*` runtime keys
- projection-area radii and corner radius
- content/full-view mapping
- `screen_to_camera` homography
- shader branch between intended mask and invalid-source fill

Current diagnostic posture: if the outer source markers or bottom checkerboard
row disappear while `full-frame-diagnostic` is upright/full, first distinguish
an analyzer segmentation artifact from a source-domain crop/import issue.
Nonblack/visible envelope parity and renderer-authored expected footprints are
the tie-breakers; do not tune HWB projection-area offsets from a single dense
component bbox.

### GL/OES

Trace these before using OES as the reference:

- `SurfaceTexture` transform matrix from the latest image
- producer buffer size versus renderer texture size
- OES external texture UV after transform
- source valid UV rect and crop
- external-OES source color transfer and selected GLES swapchain color format
- `projectionDepthMeters` (meters to the head-anchored projection surface)
- projection-area mask and matte/border policy
- per-eye `screen_to_camera` stage
- source-alpha output convention

OES can be the temporary camera-matched footprint target only while it remains
upright, unclipped, and has the largest plausible source footprint under the
same run manifest.
For source-alpha OpenXR composition, GL/OES shader output must be
premultiplied: RGB is multiplied by the projection-area or border alpha before
it reaches the compositor. If an opacity-zero witness still shows custom camera
RGB, classify the first mismatch as texture/upload convention rather than
projection-area geometry.

### Makepad CPU-YUV

Trace these before changing scale:

- stream metadata and resolved geometry profile
- `FrameOrientationDecision`
- source Y flip and stride handling
- CPU YUV plane size and upload rect
- runtime OpenXR view poses/FOV
- `projection_depth_meters` / `projectionDepthMeters` (meters to the
  head-anchored projection surface)
- `projection_content_mapping_mode`
- `screen_to_camera`
- `projection_area_screen_uv`
- `projection_area_mask`

Current diagnostic posture: Makepad's active shader uses top-left/y-down
display-screen UV for `projection_area_screen_uv` and `screen_to_camera`.
Broker top-left raster handling belongs in the projection plan for homography
modes, while Makepad CPU-YUV sampler-origin conversion belongs in the texture
sampling decision and is logged as `sourceSampleYFlip=1.0` with a
sampler-origin reason. Do not collapse these into a generic "manual Y flip";
that label hides the layer boundary that previously let the bug recur.

## Projection Coordinate Contract

Each alignment run should emit or preserve a compact public-safe contract.
`tools/quest-camera-profile/Analyze-RawStackScreenSpace.py` writes this as
`projection-coordinate-contracts.jsonl` plus
`projection-coordinate-contract-summary.json` beside the screen-space summary.
The schema name is currently `rusty.xr.projection-coordinate-contract.v1`.

```json
{
  "lane": "gles-oes-broker-h264-raw",
  "source_mode": "broker-synthetic",
  "geometry_profile": "camera-matched",
  "source": {
    "requested_width": 1280,
    "requested_height": 1280,
    "resolved_width": 1280,
    "resolved_height": 1280,
    "format": "h264-or-camera-yuv",
    "timestamp_domain": "source-specific",
    "eye": "left-or-right"
  },
  "metadata": {
    "source": "stream-manifest-or-camera2",
    "intrinsics_state": "present-defaulted-or-missing",
    "extrinsics_state": "present-defaulted-or-missing",
    "orientation_state": "explicit-defaulted-or-missing",
    "valid_source_uv_rect": [0.0, 0.0, 1.0, 1.0]
  },
  "source_sampling": {
    "contract": "screen_to_camera_content_uv_to_renderer_sampler",
    "homography_output_uv": "content-normalized-top-left-y-down",
    "sample_input_uv": "screen-to-camera-homography-output",
    "sample_transform_stage": "post_homography_pre_texture_sample",
    "sample_transform": "renderer-specific-transform-or-identity",
    "sample_transform_owner": "hwb-oes-or-cpu-yuv-owner",
    "sample_transform_applied": true,
    "sample_output_uv": "renderer-sampler-uv",
    "sampler_uv_origin": "renderer-specific",
    "sampler_y_axis": "renderer-specific"
  },
  "projection": {
    "profile": "camera-matched",
    "preview_fov_y_degrees": 60.0,
    "depth_meters": 1.0,
    "overscan": 1.0,
    "aspect": 1.0,
    "projection_area_scale_uv": 1.0,
    "projection_area_left_offset_uv": [-0.02, 0.01],
    "projection_area_right_offset_uv": [0.02, 0.01]
  },
  "openxr": {
    "reference_space": "app-reference-space",
    "openxr_reference_space": "LOCAL-or-STAGE",
    "display_time_source": "predicted-display-time",
    "predicted_display_time_ns": 123456789,
    "view_pose_fov_source": "xrLocateViews",
    "render_views": {
      "left": {
        "fov_tangents": [-1.0, 1.0, 1.0, -1.0],
        "position": [-0.03, 1.5, 0.0, 1.0],
        "orientation": [0.0, 0.0, 0.0, 1.0]
      },
      "right": {
        "fov_tangents": [-1.0, 1.0, 1.0, -1.0],
        "position": [0.03, 1.5, 0.0, 1.0],
        "orientation": [0.0, 0.0, 0.0, 1.0]
      }
    }
  },
  "transforms": {
    "surface_to_screen": "logged-row-token",
    "screen_to_surface": "logged-row-token",
    "surface_to_camera": "logged-row-token",
    "screen_to_camera": "logged-row-token",
    "texture_transform": "logged-row-token-or-not-applicable"
  },
  "mask_and_processing": {
    "invalid_region_policy": "solid-diagnostic-or-transparent-underlay",
    "projection_alpha_mode": "fixed-red-green-blue-luma-or-inverse",
    "projection_alpha_scale": 1.0,
    "projection_alpha_bias": 0.0,
    "processing_layer": "raw",
    "blur_disabled_for_coordinate_gate": true
  },
  "analysis": {
    "screenshot_method": "adb-hzdb-or-mediaprojection",
    "freshness": "fresh-or-stale",
    "observed_box": "analyzer-output-reference",
    "expected_box": "renderer-authored-display-eye-screen-uv"
  }
}
```

## Evidence Checklist

Before accepting a coordinate run:

- The run name says `full-frame-diagnostic` or `camera-matched`; ambiguous
  performance names are rejected.
- Every relevant Android property, launch extra, and runtime default is either
  explicitly set or explicitly logged as defaulted.
- Intended outside-projection mask, true invalid-source fill, guide/border
  lines, and actual source content use distinguishable colors.
- `full-frame-diagnostic` is upright/full in all active lanes.
- `camera-matched` explains its source footprint in renderer-authored
  display-eye screen UV, with analyzer inference retained only as a model
  comparison.
- `physical-camera` is not an active Camera2 lane; it names older
  source-sampling evidence only.
- HWB, OES, and Makepad all log the same stage names even if their backend
  texture paths differ.
- Screenshot analysis records observed pixels but does not override the
  manifest or transform source of truth.
- Any passthrough-underlay witness run names the synthetic or full-frame camera
  run it is meant to confirm.
- Any environment-depth particle or mesh run emits a depth/world-space contract
  artifact, including depth texture size, near/far range, capture time,
  depth-view FOV/pose, current render-eye FOV/pose, sample identity policy, and
  passthrough visibility. Infinite runtime far planes must be represented with
  an explicit `far_z_infinite` flag, not a fake numeric far clip.

## How OpenXR Fits

OpenXR supplies the per-eye view pose/FOV for a display time, reference-space
pose composition, and final runtime composition of submitted layers. It does
not automatically map Android Camera2 pixels onto the world or into a display
eye. Rusty XR owns that mapping.

The productive division is:

- put real or reconstructed scene points into app reference space when the data
  represents world geometry;
- derive per-eye screen-space sampling only after the reference-space geometry
  and source-camera model are explicit;
- keep analyzer screenshots as final-display witnesses, not coordinate
  authorities.

Only after this division is stable should the blur workflow continue.

## Current Handoff

The 2026-05-19 continuation state is summarized in
[PROJECTION_COORDINATE_HANDOFF_2026_05_19.md](PROJECTION_COORDINATE_HANDOFF_2026_05_19.md).
Use that document as the next-agent entry point, then return here for the
coordinate vocabulary and source-of-truth rules.
