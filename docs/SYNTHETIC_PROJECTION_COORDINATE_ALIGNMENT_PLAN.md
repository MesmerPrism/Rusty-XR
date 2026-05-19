# Synthetic Projection Coordinate Alignment Plan

This is the active working plan for making the public broker-synthetic camera
projection lanes comparable before using live Camera2 frames, native
passthrough underlay, physical-screen stimuli, or downstream visual effects.

The immediate rule is synthetic first: if `diagnostic-grid` or `motion-bar`
cannot travel from broker raster to screenshot with the same orientation,
target footprint, and clipping semantics across Vulkan/HWB, GL/OES, and
Makepad CPU-YUV, then camera-feed and passthrough runs are not yet diagnostic.

The coordinate domain and source-of-truth rules live in
[PROJECTION_COORDINATE_SPACE_LEDGER.md](PROJECTION_COORDINATE_SPACE_LEDGER.md).
Treat that ledger as the contract scaffold for this plan. Blur, physical
passthrough, and downstream effects remain blocked until the ledger fields are
logged or intentionally marked unavailable for each active lane.

## Current Finding

Latest evidence is from the 2026-05-19 broker-synthetic runs after the
projection-coordinate contract scaffold was added to the analyzer and the
HWB/GL/Makepad lanes were made to log the same source-geometry fields.

The three requested gates are now handled for the synthetic broker lanes:

- renderer-authored expected source-valid footprint fields are present and
  preferred over analyzer-derived boxes;
- Makepad full-frame Y orientation is upright after separating source-raster
  convention from Makepad CPU-YUV sampler-origin convention;
- full-frame vertical placement parity passes after the analyzer uses the
  full-frame visible stimulus envelope instead of a single dense component.

The current full-frame result is:

| Lane | Avg center Y | Orientation | Current interpretation |
| --- | ---: | --- | --- |
| `vulkan-hwb-broker-h264-raw` | `+0.012` | Upright | Full visible envelope, renderer-authored footprint IoU `1.000`. |
| `gles-oes-broker-h264-raw` | `-0.006` | Upright | Full visible envelope, renderer-authored footprint IoU `1.000`. |
| `makepad-cpuyuv-broker-h264-raw` | `+0.013` | Upright | Full visible envelope, renderer-authored footprint IoU `1.000`; CPU-YUV sampler-origin conversion is explicit. |

`camera-matched` now has clean source-contract coverage across the three lanes.
The previous HWB crop/smaller-footprint diagnosis has been superseded by the
new run: all three lanes show full visible projection coverage and center parity
is inside tolerance. Its expected source-valid footprint is now renderer
authored, while the analyzer-derived `screen_to_camera` footprint remains an
evidence/model check only.

Camera-matched source-domain model evidence:

| Lane | Avg center Y | Source-valid IoU | Source-invalid fraction | Orientation |
| --- | ---: | ---: | ---: | --- |
| `vulkan-hwb-broker-h264-raw` | `-0.000` | `0.494` | `0.279` | Upright |
| `gles-oes-broker-h264-raw` | `-0.007` | `0.494` | `0.270` | Upright |
| `makepad-cpuyuv-broker-h264-raw` | `-0.004` | `0.269` | `0.442` | One eye ambiguous, one upright |

The next synthetic milestone is therefore not another scale guess. It is to
carry the same source/texture/surface/screen contract into live direct/broker
Camera2, passthrough-underlay witness runs, and depth/world-space reference
checks while keeping blur disabled.

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

Each analyzed run emits or preserves the projection-coordinate contract
described by the ledger in `projection-coordinate-contracts.jsonl`, with a
compact `projection-coordinate-contract-summary.json` beside the screen-space
summary. The contract joins source size, resolved geometry profile, metadata
state, valid source UV rect, projection profile, OpenXR view source,
homography-stage tokens, mask policy, screenshot evidence, and explicit gaps.
Analyzer boxes are evidence, not replacements for the run manifest and
transform logs.

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

The old `fast075` launch hypothesis and the earlier HWB source-crop diagnosis
are superseded by the 2026-05-19 camera-matched sweep. HWB now has full visible
coverage and center parity, so the next HWB work is not a projection-area scale
tune. It is to log a renderer-authored camera-matched expected source-valid
footprint and compare it with the analyzer's `screen_to_camera` model.

### GL/OES

OES is the current synthetic camera-matched orientation reference. Keep it as a
reference only while it remains upright and its renderer-authored expected
source footprint agrees with the analyzer model.

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

The full-frame sweep showed a Makepad-specific vertical inversion even though
the stream metadata was explicit and the contract was `ready`. The fix keeps
the two layer boundaries separate: broker top-left raster metadata is applied
in the projection plan, while Makepad CPU-YUV sampler-origin conversion is
logged as `sourceSampleYFlip=1.0` with a sampler-origin reason. The follow-up
full-frame run is upright; the camera-matched run no longer reports inverted
markers, though one eye can still be ambiguous because the camera-matched valid
footprint intentionally masks much of the marker area. Do not re-label this as
a manual flip, and do not tune `contentUvScale=1.6000` as active scale evidence
until it is either wired into the shader path or renamed inactive/log-only.

## Iteration Order

1. Keep the projection-coordinate contract gate in place for every synthetic
   run.
2. Keep renderer-authored expected source-valid footprint fields mandatory for
   camera-matched mode; analyzer-derived boxes are evidence only.
3. Keep Makepad's raster convention and CPU-YUV sampler convention separately
   logged so the Y issue cannot recur as a hidden manual flip.
4. Use full-frame visible-envelope measurement for full-frame placement parity;
   dense-component boxes remain screenshot evidence, not geometry truth.
5. Re-run camera-matched synthetic and require center, orientation, and
   renderer-authored expected footprint agreement.
6. Only after raw synthetic gates pass, use live camera frames,
   passthrough-underlay comparison, and then blur while keeping geometry
   unchanged.

## Next Headset Sweep

Run the next synthetic sweep as two independent probes:

- Renderer-authored expected footprint probe: each lane logs the expected
  camera-matched source-valid footprint in display-eye screen UV, with a source
  label and homography stage that generated it.
- Makepad convention probe: require both `sourceRasterOriginPolicy` and
  `sourceSampleYFlipReason` so source-raster correction and CPU-YUV sampler
  origin conversion remain different stages.
- Full-frame envelope probe: require full-frame visible stimulus envelopes to
  match across HWB, GL/OES, and Makepad before treating dense-component
  differences as geometry findings.

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

Current structural target: make fresh `full-frame-diagnostic` and
`camera-matched` runs pass the compact `projection-coordinate-contract` gate in
all three broker lanes before using direct Camera2 or passthrough-underlay runs
as physical-world witnesses.
