# World-Space Quad And Direct Shader Reconciliation

This document reconciles two camera projection paths that can look unrelated
in code:

- world-space quad rendering, where camera pixels are drawn onto a surface in
  an OpenXR reference space and then viewed by both eyes
- direct per-eye shader rendering, where each eye samples camera pixels from
  display-eye screen UV without materializing the surface

Treat them as the same geometry chain collapsed at different layers. The quad
path keeps the intermediate reference-space surface visible. The direct shader
path hides that surface inside `screen_to_camera` or equivalent stage rows.

The world-space baseline for this comparison is the environment-depth particle
contract:

```text
depth UV
-> depth view ray
-> metric depth-view point
-> app reference-space point
-> render-eye view
-> screen
```

That chain is recorded by `rusty.xr.depth_world_space_contract.v1`. It proves
the app can take runtime sensor data into reference-space geometry and render
it through the current per-eye OpenXR views. It does not by itself prove the
Camera2 source-camera model; it gives the reference-space and per-eye render
baseline that the camera quad/direct paths must agree with.

## Shared Geometry Chain

For a camera preview surface at a chosen pose and size, both paths should reduce
to this chain:

```text
display-eye screen UV
-> render-eye ray from current OpenXR view pose/FOV
-> app reference-space ray
-> intersection with chosen camera preview surface
-> preview surface UV
-> camera/source UV
-> texture/upload UV
-> sampled camera color or invalid-source fill
```

The world-space quad path evaluates this chain through actual reference-space
geometry. The direct per-eye shader path evaluates the same chain through
precomputed or shader-side transforms, usually:

```text
display-eye screen UV
-> projection-area mask
-> screen_to_surface
-> surface_to_camera
-> source valid rect / texture transform
-> sampled camera color or invalid-source fill
```

Equivalence requires the hidden direct-shader surface to be the same surface
the world-space path would have rendered.

## World-Space Quad Path

The quad path has these owned stages:

| Stage | Owner | Required evidence |
| --- | --- | --- |
| Source frame selection | Camera2, broker, or synthetic source | source size, format, timestamp, crop/valid rect, eye identity |
| Source-camera model | App projection profile plus Camera2 metadata when available | geometry profile, intrinsics/extrinsics state, orientation, valid source UV |
| Preview surface selection | Rusty XR projection model | reference-space pose, depth/distance, FOV-derived size, aspect, overscan |
| OpenXR view projection | Runtime `xrLocateViews` result | display time, reference space, per-eye pose and FOV |
| Backend projection | Vulkan/GL/Makepad renderer | viewport, projection Y convention, surface-to-screen mapping |

Camera metadata does not choose the quad size. Rusty XR chooses a preview
surface policy, for example:

```text
half_height = tan(preview_fov_y / 2) * depth_meters * overscan
half_width = half_height * source_aspect
```

`source_aspect` means the delivered camera/content aspect, not the OpenXR
display-eye FOV aspect. This keeps a square `1280x1280` camera frame on a
square physical content surface before depth/convergence tuning starts.

That policy can be useful because OpenXR handles the stereo view of the
reference-space surface once the surface is placed. It remains a projection
profile, not a runtime guarantee that the full real room lies on that plane.

The Vulkan composite example exposes this as the `world-canvas` diagnostic
projection mode. That mode rasterizes the selected preview surface as an actual
head-anchored quad and samples camera pixels with `surface_to_camera` rows. It
is meant to answer whether a mismatch belongs to surface depth/FOV/aspect or to
the later fullscreen collapse. It is not a license to add renderer-local
offsets: any accepted correction still belongs to the named preview surface
policy, source-camera model, or OpenXR view/reference-space stage.

## Direct Per-Eye Shader Path

The direct path has these owned stages:

| Stage | Owner | Required evidence |
| --- | --- | --- |
| Projection-area mask | Rusty XR projection-area contract | center, scale, radius, opacity, invalid-region policy |
| Screen to surface | App geometry collapse of the quad path | `screen_to_surface` or equivalent rows |
| Surface to camera | Source-camera projection model | `surface_to_camera`, `screen_to_camera`, valid rect |
| Texture/upload transform | Backend import/upload path | OES transform matrix, HWB import convention, CPU-YUV stride/upload rect, sampler origin |
| Final sample policy | Renderer shader | raw/blur/effect mode, invalid-source fill, border policy |

This path is correct only when its `screen_to_camera` map is the collapsed form
of the same world-space plane and camera/source model. If the shader needs an
architecture-specific operation, it must be named at this layer. Makepad
CPU-YUV sampler-origin conversion is one such operation; it is not a projection
plan flip and should not be described as a generic manual Y flip.

## Equivalence Conditions

Accept the direct shader as equivalent to the quad path only when all of these
conditions hold:

- Same display time and OpenXR reference space.
- Same render-eye pose/FOV for the eye being rendered.
- Same preview surface pose, distance, size, aspect, and overscan policy.
- Same source-camera geometry profile and camera/source UV convention.
- Same valid source UV rect and crop policy.
- Same texture/upload transform and sampler-origin convention.
- Same projection-area mask and invalid-source fill policy.
- Same backend projection Y convention and viewport mapping.

If any item differs, the paths are not equivalent. The difference may be a
deliberate architecture rule, but it must be logged where it is owned.

## Divergence Ownership

Use this table before changing renderer code:

| Symptom | First owner to inspect | Do not hide it by |
| --- | --- | --- |
| Source appears upright in one lane and inverted in another | source raster origin, texture/upload transform, sampler origin | projection-area offset or analyzer orientation override |
| Quad path aligns but direct shader is shifted | screen-to-surface collapse, projection-area mask, render-eye FOV/viewport | source crop or texture Y flip |
| Direct shader and broker synthetic agree, live Camera2 differs | live Camera2 geometry profile, crop, orientation, intrinsics/extrinsics state | synthetic fallback profile |
| Depth particles anchor in the room but camera quad drifts | camera preview surface pose/profile, camera extrinsics approximation | depth projection changes |
| Passthrough witness and app border disagree | app-owned border/projection metadata first, then compositor/witness interpretation | treating passthrough pixels as app source UV |
| Analyzer bbox disagrees with renderer-authored footprint | analyzer segmentation semantics and selected screenshot attempt | changing renderer placement from analyzer-only evidence |

Manual adjustment is allowed only after this table names the owner. The
adjustment then becomes part of that owner's contract and must be logged as
runtime evidence.

## Using The Depth Baseline

The depth contract gives a clean world-space witness because it starts from
runtime depth view metadata rather than Camera2 preview pixels:

```text
XR_META_environment_depth image/view
-> depth view FOV ray
-> metric point
-> app reference-space point
-> current render-eye pose/FOV
-> screen
```

Compare live Camera2 and passthrough witness runs against it in this order:

1. Confirm depth particles report `ready` contracts with depth texture size,
   near/far range, depth view FOV/pose, render-eye FOV/pose, sample identity
   policy, and passthrough visibility.
2. Confirm live Camera2 runs report `physical-camera`, source size, source
   valid rect, orientation, texture/upload transform, projection-area UV, and
   renderer-authored expected footprint.
3. Use opacity-zero passthrough-underlay runs as physical witnesses only. The
   Meta compositor view helps judge real-world alignment, but it is not the
   app's source-image coordinate system.
4. When a mismatch appears, classify it as source metadata, texture/upload
   convention, projection/surface mapping, OpenXR/reference-space geometry, or
   analyzer evidence before editing code.

## Acceptance Gate Before Blur

Blur can consume the geometry only after this reconciliation is true for the
active lanes:

- Synthetic broker full-frame and camera-matched runs remain green.
- Live direct and broker Camera2 lanes use explicit `physical-camera`
  contracts, not synthetic fallback profiles.
- The depth world-space contract is ready and provides the reference-space
  baseline.
- Passthrough-underlay runs are treated as physical witnesses with app-owned
  border/projection metadata.
- Any architecture-specific convention is named at its owning layer and logged.

After that, the blur shader is allowed to depend on the accepted
`screen_to_camera` or equivalent mapping. It must not become the place where
projection geometry is corrected.
