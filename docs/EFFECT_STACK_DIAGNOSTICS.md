# Effect Stack Diagnostics

Rusty XR can model a multi-pass visual pipeline without owning the renderer,
shader code, or product-specific visual behavior. The public contract surface
is:

- an ordered pass graph;
- logical intermediate buffers;
- named diagnostic layer taps;
- layer-level metric rows for offline or headset-captured comparisons.

This is useful for public examples, downstream app shells, and operator tools
that need to compare source, guide, edge, mask, displacement, and final
composition layers without copying private effect implementations into public
core.

## Public Boundary

Public Rusty XR owns data contracts and scorecard vocabulary:

- `EffectStackDescriptor`
- `EffectPassDescriptor`
- `EffectBufferDescriptor`
- `EffectDiagnosticLayer`
- `EffectLayerMetrics`
- `EffectLayerComparisonMetrics`
- `EffectStackComparisonReport`

Renderers and app shells own the implementation:

- shader code and render-pass construction;
- concrete color maps, blur kernels, thresholds, and displacement functions;
- native texture allocation, Vulkan/OpenXR/Android ownership, and frame loops;
- screenshots, logs, captures, generated APKs, and local validation artifacts.

Public docs and examples should use generic terms such as "color map",
"edge detector", "guide texture", "candidate", and "reference". Do not name a
downstream app, package id, private profile, private visual stack, or local
artifact path in committed public files.

## Pass Graph Shape

An `EffectStackDescriptor` is an ordered graph. Inputs may refer to:

- `source`;
- a declared buffer;
- an output buffer or pass id produced by an earlier pass.

The descriptor is intentionally not a full render graph. It does not describe
pipeline barriers, descriptor sets, image layouts, queue ownership, shader
entry points, or platform handles. Native adapters can translate the data into
their own backend-specific render graph.

Typical public-safe pass categories include:

- `Source`
- `LumaTransform`
- `Blur`
- `ColorMap`
- `EdgeDetection`
- `ScalarMap`
- `Displacement`
- `Composite`
- `DiagnosticTap`

## Diagnostic Layers

A diagnostic layer is a named tap that can be captured or summarized. Examples:

- raw source;
- luma guide;
- blurred guide;
- edge guide;
- scalar or mask guide;
- final composite.

The layer list gives comparison tools a stable order and label vocabulary even
when the underlying renderer uses different private pass placement.

## Metrics

The first public metric shape stays scalar and descriptive:

- active pixel fraction;
- luma mean and standard deviation;
- edge energy;
- high-frequency energy;
- luma RMSE and bias;
- edge ratio candidate-over-reference;
- high-frequency ratio candidate-over-reference.

The metrics are not a universal acceptance gate. They are a way to stop mixing
projection coverage, blur, guide generation, edge response, and final
composition into one visual judgment. A downstream project can set its own
tolerances while still exporting a public-safe report shape.

## Example

Run the contracts-only example:

```powershell
cargo run -p rusty-xr-contracts --example effect_stack_diagnostic_manifest --features serde
```

It emits a synthetic color/edge stack descriptor plus a comparison report. The
example does not render pixels, allocate textures, or implement an image
effect.

## Validation

For source changes in the public contracts:

```powershell
cargo fmt --all --check
cargo test -p rusty-xr-contracts --all-features
cargo run -p rusty-xr-contracts --example effect_stack_diagnostic_manifest --features serde
python tools/schema/export_schemas.py --check
python tools/boundary-scan/rusty_xr_boundary_scan.py --repo-root .
```
