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

Latest synthetic evidence includes the 2026-05-19 broker-synthetic runs plus
the 2026-05-20 focused projection-area offset probes. The analyzer now checks
the source-geometry fields and the renderer-authored projection-area target
fields in the same `display-eye-screen-uv` coordinate contract.

The three requested gates are now handled for the synthetic broker lanes:

- renderer-authored expected source-valid footprint fields are present and
  preferred over analyzer-derived boxes;
- renderer-authored projection-area target rects and centers are logged for
  HWB, GL/OES, and Makepad;
- Makepad full-frame Y orientation is upright after separating source-raster
  convention from Makepad CPU-YUV sampler-origin convention;
- full-frame vertical placement parity passes after the analyzer uses the
  full-frame visible stimulus envelope instead of a single dense component.
- shared positive X and positive Y projection-area offsets move the target
  right and down in the renderer-authored display-eye UV contract across the
  three lanes when launched through the suite-level controls. Native mirror or
  ADB screenshot pixels remain a separate evidence layer after runtime
  presentation.

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

The synthetic milestone is frozen as a regression gate. The same
source/texture/surface/screen contract has since been promoted into live
direct/broker Camera2, passthrough-underlay witness runs, and a
depth/world-space contract artifact while keeping blur disabled. The remaining
work is not another synthetic scale guess; it is to compare those live,
passthrough, and depth records under the same stage names.

The native-passthrough alignment pass adds one more measured layer before
tuning custom projection geometry: display-eye UV to mirror screenshot pixels.
The `display-eye-uv-fiducial` renderer diagnostic marks known
`display-eye-screen-uv` points and the analyzer records both a global affine
fit and a near-center finite-difference mapping around the green center marker.
Use that mapping to convert a native-reference versus frozen-replay
green-cross delta into named projection-space changes. Do not infer the mapping
from eye-half screenshot dimensions, because the mirror path can be non-linear.

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

Do not use `scale075` profiles for synthetic coordinate alignment. Those profile
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
| `scale075` profile names | Historical performance profile names look like geometry intent | Never use for coordinate gates |
| `rustyquest.cameraProjectionScale` | Changes HWB camera source surface before shader sampling | Log and tune separately from projection-area scale |
| `rustyquest.projectionAreaScaleUv` / `projection_area_scale_*` | Changes where the intended projection area lands on screen | Use for screen-footprint scale, not source crop |
| `contentUvScale=1.6000` | Makepad logs/hotloads it, but the active shader path does not use it as the projection footprint control | Do not tune until wired or renamed inactive |
| Android `debug.rustyquest.makepad.*` properties | Persist across launches and can contaminate direct/broker comparisons | Device gate must clear or explicitly set every relevant property |
| Analyzer dense component union | Measures visible content after color segmentation | Use as screenshot evidence, not as the transform source |
| Display-eye UV fiducial mapping | The mirror screenshot path can warp submitted display-eye UV non-linearly | Log it before converting screenshot deltas into projection-space tuning |

### Vulkan/HWB

Trace these fields in order:

1. catalog/runtime profile selected by the suite;
2. launch override for `rustyquest.cameraProjectionScale`;
3. launch override for `rustyquest.projectionAreaScaleUv`;
4. native config log fields `projectionScale`, `projectionAreaScaleUv`,
   `projectionAreaRadiusXUv`, and `projectionAreaRadiusYUv`;
5. generated `screen_to_camera` homography rows;
6. renderer-authored `projectionAreaOffsetResponseModel` and per-eye
   `*ProjectionAreaOffsetResponseUv` fields;
7. shader decision between intended mask and invalid-source fill.

The old `scale075` launch hypothesis and the earlier HWB source-crop diagnosis
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

The Makepad Y lessons now split into two different domains. Source-raster
orientation and CPU-YUV sampler-origin conversion remain separate and are
logged through `sourceSampleYFlip=1.0` with a sampler-origin reason. Projection
area placement, however, uses the same display/screenshot convention as HWB and
GL/OES: positive Y moves the projection area down. The Makepad launcher wrapper
therefore normalizes the legacy native horizontal offset sign only; it does not
invert the vertical projection-area offset. Do not re-label source sampling as
a projection-area flip, and do not tune `contentUvScale=1.6000` as active scale
evidence until it is either wired into the shader path or renamed
inactive/log-only.

## Regression And Promotion Order

1. Keep the projection-coordinate contract gate in place for every synthetic
   regression run.
2. Keep renderer-authored expected source-valid footprint fields mandatory for
   camera-matched mode; analyzer-derived boxes are evidence only.
3. Keep Makepad's raster convention, CPU-YUV sampler convention, and
   projection-area placement convention separately logged so Y issues cannot
   recur as hidden manual flips.
4. Use full-frame visible-envelope measurement for full-frame placement parity;
   dense-component boxes remain screenshot evidence, not geometry truth.
5. Use the frozen synthetic gates to catch regressions before live Camera2,
   passthrough, or depth/world-space changes.
6. For native-passthrough alignment, capture the display-eye UV fiducial and
   use its near-center mapping before applying a projection-space correction.
7. Compare live Camera2, passthrough-underlay witness, and depth/world-space
   contract records before resuming blur.

## Synthetic Regression Sweep

Run a synthetic sweep as regression evidence when a renderer, analyzer, broker,
or Makepad wrapper change could affect coordinate mapping:

- Renderer-authored expected footprint probe: each lane logs the expected
  camera-matched source-valid footprint in display-eye screen UV, with a source
  label and homography stage that generated it.
- Makepad convention probe: require both `sourceRasterOriginPolicy` and
  `sourceSampleYFlipReason` so source-raster correction and CPU-YUV sampler
  origin conversion remain different stages.
- Full-frame envelope probe: require full-frame visible stimulus envelopes to
  match across HWB, GL/OES, and Makepad before treating dense-component
  differences as geometry findings.

Do not start blur alignment if these probes regress against the same
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

- `camera-stereo-gpu-composite-scale075` remains a compatibility/performance
  profile.
- `broker-h264-stereo-live-openxr-projection-scale075-probe` remains a
  compatibility/performance profile.
- `camera-stereo-gpu-composite-full-feed-control` is the direct HWB
  raw-stack transport/parity control.
- `broker-h264-stereo-live-openxr-projection-full-feed-control` is the HWB
  broker raw-stack transport/parity control.

The raw-stack suite should launch the full-feed control profiles for HWB and
then pass explicit overrides for processing layer, border policy, projection
scale, projection-area scale, radius, corner radius, opacity, and offsets. If a
future screenshot name still contains `scale075`, the suite is launching the
wrong profile or the catalog/profile resolver returned stale data.

Public default projection scale is now full-feed (`1.0`). Any `0.75` projection
scale should come only from an explicitly named compatibility or performance
profile. This prevents missing launch overrides from silently reintroducing a
clipped camera-source footprint.

Do not use the full-feed controls as the custom passthrough replacement
footprint. The current custom-footprint gate is the depth-1.0 pair:
`camera-stereo-gpu-composite-world-canvas-depth1-mediaprojection` and
`camera-stereo-gpu-composite-camera-footprint-canvas-equivalent-depth1`. The
first renders the camera-content surface as a real world canvas; the second
should reproduce that result through the collapsed fullscreen shader by mapping
display-eye screen UV to the same surface before source sampling. The collapsed
profile uses `cameraPipelinePreset=raw-projection-unorm` with
`projectionBorderPolicy=passthrough-underlay` so the full Camera2 frame remains
the source input while only the valid reconstructed camera footprint contributes
color.

The depth-1.0 canvas/collapsed comparison is now the handoff point between
internal lane parity and native-passthrough alignment. With matching
`projectionDepthMeters`, `cameraPreviewFovYDegrees`, delivered source aspect,
and `cameraRawOverlayOverscan=1.0`, the real canvas and collapsed
camera-footprint shader should agree in both MediaProjection and HzDB
screenshots. After that equivalence is proven, tune native-passthrough
alignment on the `world-canvas` lane first and translate the resulting surface
parameters back into the collapsed custom profile.

For the passthrough solve, use native passthrough as the reference and sweep
only documented surface geometry at first:

- start with `projectionDepthMeters`, including closer values around `0.5`
  meters when the depth-1.0 surface appears too far away;
- then adjust `cameraPreviewFovYDegrees` as the vertical-height knob;
- leave `cameraRawOverlayOverscan` at `1.0` until depth and height are close,
  then use it only as a named coverage pad.

Do not use full-feed profiles, projection-area offsets, blur, passthrough
opacity, or renderer-local constants to make this comparison look good. If the
canvas cannot be aligned to the native-passthrough green cross using depth and
height, the result should be reported as a named divergence in the surface
model, source metadata, OpenXR view/reference-space state,
compositor/screenshot convention, or analyzer evidence.

The final custom-projection edge policy is still separate from this sweep.
Because the two raw eye cameras can cover slightly different outer-edge regions,
the per-eye custom path must later choose a documented behavior for those
regions: shared clipped footprint, per-eye footprint with passthrough underlay,
or an explicit fused/combined image mode.

The Java-side HWB `CompositeLayerActivity` defaults are also full-feed now:
`cameraProjectionScale=1.0`, `xrRenderScale=1.0`,
`projectionAreaRadiusXUv=0.5`, `projectionAreaRadiusYUv=0.5`, and
`projectionAreaCornerRadiusUv=0.0`. This matters because launch overrides
are not a substitute for sane defaults; a missing extra should fail into the
same coordinate intent, not into an old performance profile.

The native HWB `RuntimeConfig` defaults now match that full-feed intent:
`cameraProjectionScale=1.0`, `xrRenderScale=1.0`,
`projectionAreaRadiusXUv=0.5`, `projectionAreaRadiusYUv=0.5`, and
`projectionAreaCornerRadiusUv=0.0`.

The HWB clipping trace starts at `rustyquest.cameraProjectionScale` and
`rustyquest.projectionAreaScaleUv`. `cameraProjectionScale=0.75` changes the
camera-source footprint before the shader sees it; `projectionAreaScaleUv`
changes the screen/projection-area mapping. Both must be recorded separately.

The Makepad clipping trace starts at `debug.rustyquest.projection.scale` and
`debug.rustyquest.projection.area.*`, then continues through
`projection_area_screen_uv`, `screen_to_camera`, and `projection_area_mask`.
The standalone Makepad device gate keeps projection scale at `1.0` while using
`rustyquest.makepad.xrRenderScale=0.90` for the accepted target-local HWB performance
lane; full-resolution `1.0` render-scale runs are explicit stress runs.

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
   `scale075`, `TARGET_FULL_VIEW_CONTENT_UV_SCALE`, and `contentUvScale=1.6000`
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
