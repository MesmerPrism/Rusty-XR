# Synthetic Projection Coordinate Alignment Plan

This is the active working plan for making the public broker-synthetic camera
projection lanes comparable before using live Camera2 frames, native
passthrough underlay, physical-screen stimuli, or downstream visual effects.

The immediate rule is synthetic first: if `diagnostic-grid` or `motion-bar`
cannot travel from broker raster to screenshot with the same orientation,
target footprint, and clipping semantics across Vulkan/HWB, GL/OES, and
Makepad CPU-YUV, then camera-feed and passthrough runs are not yet diagnostic.

## Current Finding

The current broker-synthetic camera-matched evidence is not aligned. The
diagnostic split colors show near-zero true source-invalid sampling, so the
main issue is not runaway invalid UV reads. The issue is that the renderers
choose different camera-source footprints inside their submitted per-eye
surfaces.

Observed valid-content coverage relative to the visible render surface:

| Lane | Width | Height | Area | Current interpretation |
| --- | ---: | ---: | ---: | --- |
| `gles-oes-broker-h264-raw` | ~0.825 | ~0.790 | ~0.652 | Current target: largest camera-matched footprint, upright markers |
| `vulkan-hwb-broker-h264-raw` | ~0.672 | ~0.634 | ~0.426 | Source-domain clipping: the lower checkerboard row and left/right edge markers are visibly lost, even though the run used full-feed launch overrides |
| `makepad-cpuyuv-broker-h264-raw` | ~0.680 | ~0.647 | ~0.440 | Projection-footprint scale issue: the whole stimulus is visible and upright, but it lands in a smaller/lower screen-space footprint than OES |

The full-frame diagnostic profile renders upright with full valid coverage in
all three lanes. That narrows the blocker to camera-matched source-to-projection
mapping and projection-area normalization, not the broker stimulus generator or
H.264 decoder globally.

The current visual diagnosis is lane-specific:

- HWB is a source-domain crop/clipping bug. The screenshot shows the bottom
  checkerboard row fully gone and the outer source markers at the left and right
  edges nearly gone. That is different from a simple downscale: the stimulus is
  not wholly visible inside a smaller rectangle.
- Makepad is not showing that crop class in the current broker-synthetic run.
  It shows the full diagnostic raster, including the top and bottom orientation
  bands, but the whole raster is mapped to a smaller screen-space footprint than
  OES. Treat it as a projection scale/window normalization issue.

Earlier analyzer output under-reported Makepad height because the diagnostic
stimulus is multi-band. The checkerboard and top color/header band are separate
dense components, and the old analyzer boxed only the largest connected
component. The analyzer now reports the dense valid-content union and draws the
largest component separately when it differs from the union. The real parity
failure remains: Makepad and HWB still cover less of the OES target footprint.

## Coordinate Contract

Each lane must report and preserve this chain:

```text
broker stimulus raster pixels
-> stream metadata and decoded texture UV
-> source/camera UV
-> projection-area UV
-> renderer-specific OpenXR per-eye surface
-> screenshot pixels
```

For `camera-matched`, the source raster may be synthetic, but its metadata must
still describe the selected camera-shaped projection. For
`full-frame-diagnostic`, the raster is diagnostic projection-surface content and
must not be interpreted as camera-shaped footage.

## Diagnostic Color Semantics

Do not overload red:

- actual camera/stimulus content: source raster or live camera sample;
- intended outside-projection mask: stable matte color;
- true invalid source UV or mapping failure: bright diagnostic red;
- guide/border lines: separate eye-specific guide colors.

A large intended-mask region is not automatically an error. It becomes an
alignment error when one renderer masks or clips substantially more of the same
camera-matched synthetic source than another renderer.

## Profile Naming Rule

Do not use `fast075` profiles for synthetic coordinate alignment. Those profile
names came from performance comparisons at `0.75` render/projection scale and
are ambiguous for the current goal.

For this workflow, profile and run names must say what is being tested:

- full-feed or full-frame projection coverage;
- camera-matched source footprint;
- direct camera or broker H.264 source;
- raw or blur processing;
- diagnostic split or passthrough-underlay border policy.

If a run uses `cameraProjectionScale=1.0` or `projectionAreaScale=1.0`, the
profile name must not imply `0.75`. If a performance run deliberately uses
`0.75`, it belongs in streaming/performance diagnostics, not in this alignment
gate.

## Pipeline Trace Targets

## Source-Of-Truth Audit

The current architecture is coherent in direction, but it is still too easy to
change the same concept through multiple surfaces without realizing it. For
coordinate-alignment work, treat the active source of truth in this order:

1. broker stream/stimulus metadata for raster orientation, raster size,
   content aspect, source UV rect, and mapping intent;
2. raw-stack suite parameters for the run's requested projection-area scale,
   radius, corner radius, opacity, border policy, and processing layer;
3. app-resolved runtime logs for the values actually applied after catalog,
   launch extras, Android properties, Java defaults, and native defaults merge;
4. shader mapping logs/homography rows for the active coordinate transform;
5. analyzer measurement records for screenshot-space observations only.

Do not use analyzer defaults, Android property remnants, or compatibility
runtime profile names as evidence that a renderer intentionally used a
coordinate value. They are evidence only after the resolved app log and run
manifest agree.

Known confusing settings:

| Setting/surface | Current risk | Alignment rule |
| --- | --- | --- |
| `fast075` profile names | Historical performance profile names look like geometry intent | Never use for coordinate gates |
| `rustyxr.cameraProjectionScale` | Changes HWB camera source surface before shader sampling | Log and tune separately from projection-area scale |
| `rustyxr.cameraProjectionAreaScaleUv` / `projection_area_scale_*` | Changes where the intended projection area lands on screen | Use for screen-footprint scale, not source crop |
| `contentUvScale=1.6000` | Makepad logs/hotloads it, but the active shader path does not use it as the projection footprint control | Do not tune until wired or renamed inactive |
| Android `debug.rustyxr.*` properties | Persist across launches and can contaminate direct/broker comparisons | Device gate must clear or explicitly set every relevant property |
| Analyzer dense component union | Measures visible content after color segmentation | Use as screenshot evidence, not as the transform source |

### Vulkan/HWB

Trace these fields in order:

1. catalog/runtime profile selected by the suite;
2. launch override for `rustyxr.cameraProjectionScale`;
3. launch override for `rustyxr.cameraProjectionAreaScaleUv`;
4. native config log fields `projectionScale`, `projectionAreaScaleUv`,
   `projectionAreaRadiusXUv`, and `projectionAreaRadiusYUv`;
5. generated `screen_to_camera` homography rows;
6. shader decision between intended mask and invalid-source fill.

The old `fast075` launch hypothesis is no longer sufficient for the current
evidence. The camera-matched run used `cameraProjectionScale=1`,
`cameraProjectionAreaScaleUv=1`, `cameraProjectionAreaRadiusXUv=0.5`,
`cameraProjectionAreaRadiusYUv=0.5`, and `cameraProjectionAreaCornerRadiusUv=0`,
yet the source raster still loses edge content. The active HWB bug is therefore
in the camera-matched source-domain mapping, most likely around
`projected_camera_matched_display_eye_homography`,
`screen_to_camera`, `camera_raw_overlay_overscan`,
`camera_preview_fov_y_degrees`, or `content_uv_scale`, rather than the
projection-area mask alone.

### GL/OES

OES is the current synthetic camera-matched coverage target. Keep it as the
reference only while it remains upright and close to the largest plausible
camera-source footprint.

Trace these fields:

1. broker metadata orientation and content geometry;
2. SurfaceTexture transform matrix application;
3. projection-area scale/radius/offset fields;
4. OES shader split between projection mask and invalid source UV.

### Makepad CPU-YUV

Trace these fields in order:

1. broker stream metadata parsed by Makepad;
2. `FrameOrientationDecision` and `source_sample_y_flip`;
3. `projection_content_mapping_mode`;
4. `screen_to_camera` homography rows from the broker camera-matched plan;
5. `projection_area_screen_uv`, `projection_area_mask`, and
   `projection_area_content_uv` in the Makepad shader;
6. final valid content bbox and orientation marker classification.

The first hypothesis is that Makepad applies a source Y transform that is
correct for one coordinate domain but wrong after camera-matched
`screen_to_camera` mapping. Full-frame being upright means this is not a global
decoder flip.

The current camera-matched broker evidence supersedes that first hypothesis for
the scale question: Makepad's stimulus is upright and complete, but its
projection footprint is smaller than OES. The next Makepad trace should compare
`screen_to_head_surface_uv`, `projection_depth_meters`,
`projection_preview_fov_y_degrees`, `projection_raw_overscan`, and the
`projection_area_screen_uv` scale against the OES footprint. Do not tune
`contentUvScale=1.6000` as if it were active scale evidence until it is either
wired into the shader path or renamed as inactive/log-only state.

## Iteration Order

1. Add analyzer language that treats current camera-matched evidence as
   geometry parity failure, not a pass.
2. Replace alignment-suite HWB runtime profiles with explicit full-feed
   alignment profiles and keep `fast075` only as a compatibility/performance
   alias.
3. Re-run full-frame synthetic across the three broker lanes and normalize
   render-surface placement first.
4. Re-run camera-matched synthetic across the three broker lanes and make valid
   content bbox width, height, area, center, and orientation agree within a
   small tolerance.
5. Only then switch from `raw` to `blur` while keeping geometry unchanged.
6. Only after raw and blur synthetic gates pass, use live camera frames and
   passthrough-underlay comparison.

## Next Headset Sweep

Run the next synthetic camera-matched sweep as two independent probes:

- HWB source-crop probe: keep projection-area radius/corner at full square and
  vary only HWB source-domain controls (`cameraProjectionScale`,
  `cameraPreviewFovYDegrees`, or `cameraRawOverlayOverscan`) until the outer
  source markers and bottom checkerboard row are visible. The acceptance signal
  is full source-raster visibility, not merely a larger content bbox.
- Makepad footprint-scale probe: keep source sampling and orientation fixed and
  vary only `makepad_projection_area_scale_x/y`. Because scale is applied before
  the area mask, values below `1.0` should enlarge the visible footprint. A first
  estimate from the current OES target is about `0.82` for both axes, but it must
  be verified by screenshot and pixel analysis.

Do not start blur alignment until both probes pass against the same
camera-matched synthetic stimulus.

## Pass Conditions

For a broker-synthetic camera-matched gate:

- no metadata defaults for orientation or content geometry;
- no inverted or ambiguous orientation markers;
- true invalid-source fill near zero;
- cross-lane valid-content bbox width and height agree within the configured
  analyzer tolerance;
- cross-lane center offsets agree within the configured analyzer tolerance;
- source-footprint IoU is reported for every eye where homography rows exist;
- any intended mask remains clearly distinguishable from true invalid-source
  fill.

Until those conditions hold, the current work item is coordinate mapping, not
blur tuning or physical-camera passthrough alignment.

## 2026-05-18 Implementation Notes

The first cleanup split compatibility/performance profiles from alignment
profiles:

- `camera-stereo-gpu-composite-fast075` remains a compatibility/performance
  profile.
- `broker-h264-stereo-live-openxr-projection-fast075-probe` remains a
  compatibility/performance profile.
- `camera-stereo-gpu-composite-full-feed-alignment` is the direct HWB
  alignment profile.
- `broker-h264-stereo-live-openxr-projection-full-feed-alignment` is the HWB
  broker alignment profile.

The raw-stack suite should launch the full-feed alignment profiles for HWB and
then pass explicit overrides for processing layer, border policy, projection
scale, projection-area scale, radius, corner radius, opacity, and offsets. If a
future screenshot name still contains `fast075`, the suite is launching the
wrong profile or the catalog/profile resolver returned stale data.

Public default projection scale is now full-feed (`1.0`). Any `0.75` projection
scale should come only from an explicitly named compatibility or performance
profile. This prevents missing launch overrides from silently reintroducing a
clipped camera-source footprint.

The Java-side HWB `CompositeLayerActivity` defaults are also full-feed now:
`cameraProjectionScale=1.0`, `xrRenderScale=1.0`,
`cameraProjectionAreaRadiusXUv=0.5`, `cameraProjectionAreaRadiusYUv=0.5`, and
`cameraProjectionAreaCornerRadiusUv=0.0`. This matters because launch overrides
are not a substitute for sane defaults; a missing extra should fail into the
same coordinate intent, not into an old performance profile.

The native HWB `RuntimeConfig` defaults now match that full-feed intent:
`cameraProjectionScale=1.0`, `xrRenderScale=1.0`,
`cameraProjectionAreaRadiusXUv=0.5`, `cameraProjectionAreaRadiusYUv=0.5`, and
`cameraProjectionAreaCornerRadiusUv=0.0`.

The HWB clipping trace starts at `rustyxr.cameraProjectionScale` and
`rustyxr.cameraProjectionAreaScaleUv`. `cameraProjectionScale=0.75` changes the
camera-source footprint before the shader sees it; `cameraProjectionAreaScaleUv`
changes the screen/projection-area mapping. Both must be recorded separately.

The Makepad clipping trace starts at `debug.rustyxr.projection.scale` and
`debug.rustyxr.makepad.projection.area.*`, then continues through
`projection_area_screen_uv`, `screen_to_camera`, and `projection_area_mask`.
The standalone Makepad device gate now defaults projection and XR render scale
to `1.0`; scaled runs must opt in.

The analyzer now emits a cross-lane parity check. A run can have usable
projection-mapping records and still fail `cross-lane-valid-projection-footprint`
when the lanes disagree on footprint dimensions, center, area, or orientation.

Reference-lane audit notes:

- System-design references point to a source-of-truth problem, not a shader-only
  problem: the same coordinate intent currently exists as catalog defaults,
  launch overrides, Android extras/properties, app defaults, shader uniforms,
  stream metadata, and analyzer assumptions.
- Zero-style command-contract guidance applies well here: each run should emit
  one machine-readable coordinate contract that records requested values,
  resolved values, defaults used, and the source that won each field.
- Waza-style health guidance applies to the instruction surface: prefer a short
  summary plus explicit risk codes over long historical logs when diagnosing
  whether a run is blocked by metadata, launch config, shader mapping, or
  analyzer segmentation.
- DroidDesk is mostly a reference for long-running Android session robustness,
  not projection math. The useful lesson here is to log OS/session lifecycle
  constraints explicitly instead of hiding them as environment assumptions.

Current architecture risks to fix:

1. Settings are still scattered across profile JSON, PowerShell launchers,
   Android extras, Android system properties, Rust defaults, Java defaults,
   Makepad live uniforms, and analyzer defaults.
2. Some names describe implementation history rather than current intent:
   `fast075`, `TARGET_FULL_VIEW_CONTENT_UV_SCALE`, and `contentUvScale=1.6000`
   are easy to misread during alignment work.
3. `contentUvScale` is currently logged/hotloaded in Makepad but is not used by
   the active shader path. Until it is either wired into the mapping or renamed
   as inactive, it should not be used as evidence for clipping.
4. Analyzer boxes must distinguish dense content union, largest component,
   render surface, expected source-domain footprint, intended mask, and true
   invalid-source fill.

Next structural target: add a compact `projection-coordinate-contract` record
per lane that joins launch request, resolved app config, stream metadata,
shader mapping fields, and analyzer measurements into one JSON object.
