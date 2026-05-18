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

Latest synthetic evidence, captured on 2026-05-19, has this status:

- `full-frame-diagnostic`: all three broker lanes have `ready`
  projection-coordinate contracts with explicit source size, valid source UV
  rect, metadata readiness, and all four homography stages. The remaining
  failures are measured rendering differences: HWB is vertically low, Makepad
  is vertically high and inverted, and GL/OES is closest to the current center
  reference.
- `camera-matched`: all three broker lanes have complete source contracts and
  full visible projection coverage. Center parity is inside tolerance. The
  remaining gap is that expected camera-matched source footprints are
  analyzer-derived from `screen_to_camera` rows rather than emitted by each
  renderer as an authored expected box/mask.

Do not treat this as blur-ready. The next raw-coordinate work is to add
renderer-authored expected footprints for camera-matched runs and fix Makepad's
full-frame Y orientation before physical passthrough/depth witnesses are used.

## Coordinate Domains

Every renderer change should name the domain it reads, writes, or converts.
The same term must not mean different things in different lanes.

| Domain | Owner / source | Units and origin | Required next-step evidence |
| --- | --- | --- | --- |
| Camera2 active-array pixels | Android Camera2 metadata | Sensor pixel coordinates. Origin and crop are Camera2-defined. | Log active-array size, selected stream size, crop/valid rect when available. |
| Delivered image pixels | Camera2, MediaCodec, broker synthetic source, or CPU YUV frame | Raster pixel coordinates for the actual buffer delivered to the renderer. | Log width, height, format, row stride when relevant, timestamp domain, and source-eye identity. |
| Decoded texture UV | Hardware buffer, OES texture, or CPU-upload texture | Normalized texture coordinates after decoder/upload ownership. | Log texture size, valid UV rect, OES transform matrix, Y flip, and crop transform. |
| Camera/source UV | Camera projection model | Normalized camera image sample domain after source orientation and valid rect are applied. | Log `surface_to_camera`, `screen_to_camera`, source invalid-fill policy, and source rect clipping. |
| Content surface UV | Rusty XR projection model | Normalized coordinates on the intended camera/content surface. | Log content rect, content aspect, projection profile, and any overscan or scale. |
| Full submitted surface UV | Renderer/OpenXR swapchain image | Normalized coordinates over the full submitted eye surface or layer image. | Log full surface size, viewport, scissor, matte/border policy, and full-to-content mapping. |
| Projection-area UV | Rusty XR projection-area mask | Normalized intended visible camera area inside the submitted surface. | Log projection-area center, radius/scale, corner radius, opacity, and invalid-region policy. |
| Display-eye screen UV | Final per-eye submitted image before screenshot | Normalized screen-space domain per eye. | Log `surface_to_screen`, `screen_to_surface`, expected box, observed box, and per-eye tokens. |
| OpenXR view tangent space | OpenXR view pose/FOV | Eye-local rays derived from `XrView.pose` and `XrView.fov`. | Log display time, reference space, per-eye pose, and FOV angles. |
| OpenXR app reference space | App-chosen `LOCAL`, `STAGE`, or other reference space | Meters, runtime-defined origin for the chosen reference space. | Log reference-space type, pose composition, and any head-anchored surface pose. |
| Environment-depth UV | `XR_META_environment_depth` swapchain image | Normalized depth-image coordinates per depth view. | Log depth image index, near/far meters, depth view pose/FOV, timestamp when available, and hand-removal state if used. |
| Vulkan clip / NDC / viewport | Renderer backend | Clip/NDC after app projection, then viewport pixels. Vulkan viewport Y conventions can differ from shader math assumptions. | Log projection matrix convention, viewport dimensions, manual FOV projection, and any Y flip. |
| Screenshot pixels | ADB/HzDB/MediaProjection or analyzer input | Captured final-display pixels. | Use only as evidence. Record capture method, freshness, segmentation thresholds, boxes, and contact sheet. |

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

This can look good because OpenXR handles stereo display of the surface once it
is placed in reference space, and Camera2 intrinsics/extrinsics provide a
plausible source sample for that surface. It is still a plane approximation:
it is exact only for the chosen surface plane. Matching a whole room requires
depth-assisted geometry or an accepted approximation policy.

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
- source texture transform
- valid source UV rect
- projection-area mask

If the stages disagree, fix the first divergent stage. Do not compensate for a
bad source rect with a projection-area scale, or for a bad projection area with
a source crop.

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

## Three-Lane Rules

### Vulkan/HWB

Trace these before changing a visual parameter:

- source mode and geometry profile
- selected camera/source size and valid source UV rect
- `cameraProjectionScale`
- `cameraProjectionAreaScaleUv`
- projection-area radii and corner radius
- content/full-view mapping
- `screen_to_camera` homography
- shader branch between intended mask and invalid-source fill

Current diagnostic posture: if the outer source markers or bottom checkerboard
row disappear while `full-frame-diagnostic` is upright/full, treat HWB as a
source-domain crop or clipping probe before changing blur.

### GL/OES

Trace these before using OES as the reference:

- `SurfaceTexture` transform matrix from the latest image
- producer buffer size versus renderer texture size
- OES external texture UV after transform
- source valid UV rect and crop
- projection-area mask and matte/border policy
- per-eye `screen_to_camera` stage

OES can be the temporary camera-matched footprint target only while it remains
upright, unclipped, and has the largest plausible source footprint under the
same run manifest.

### Makepad CPU-YUV

Trace these before changing scale:

- stream metadata and resolved geometry profile
- `FrameOrientationDecision`
- source Y flip and stride handling
- CPU YUV plane size and upload rect
- runtime OpenXR view poses/FOV
- `projection_content_mapping_mode`
- `screen_to_camera`
- `projection_area_screen_uv`
- `projection_area_mask`

Current diagnostic posture: if the whole stimulus is visible but lands in a
smaller footprint than OES, treat Makepad as a projection scale/window
normalization probe. Do not tune inactive or compatibility-only log fields
until the active shader path consumes them.

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
  "projection": {
    "profile": "camera-matched",
    "preview_fov_y_degrees": 60.0,
    "depth_meters": 1.0,
    "overscan": 1.0,
    "aspect": 1.0,
    "projection_area_scale_uv": 1.0
  },
  "openxr": {
    "reference_space": "LOCAL-or-STAGE",
    "display_time_source": "predicted-display-time",
    "view_pose_fov_source": "xrLocateViews"
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
    "processing_layer": "raw",
    "blur_disabled_for_coordinate_gate": true
  },
  "analysis": {
    "screenshot_method": "adb-hzdb-or-mediaprojection",
    "freshness": "fresh-or-stale",
    "observed_box": "analyzer-output-reference",
    "expected_box": "manifest-or-transform-derived"
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
- `camera-matched` explains its source footprint in metadata, not analyzer
  inference.
- HWB, OES, and Makepad all log the same stage names even if their backend
  texture paths differ.
- Screenshot analysis records observed pixels but does not override the
  manifest or transform source of truth.
- Any physical-camera or passthrough-underlay run names the synthetic run it is
  meant to confirm.

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
