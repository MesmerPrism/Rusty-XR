# Particle Billboard And Animation Performance

This note captures reusable particle-renderer lessons for Rusty XR consumers.
It is intentionally generic: downstream apps own their scenes, shaders, package
identity, runtime profiles, assets, and hardware validation artifacts.

## Core Rule

Keep particle centers in the intended OpenXR reference space, then choose a
billboard construction mode deliberately.

Two billboards can use the same stable scene-space particle centers while still
having different GPU cost and clipping behavior:

- Center-projected billboard: project the center once, expand the disc in
  clip/NDC space.
- World-expanded billboard vertices: expand every fan vertex in scene space,
  then project each vertex.

Do not use the billboard construction mode as a substitute for correct
OpenXR-space ownership. First fix reference-space anchoring, view-pose
composition, projection signs, and Vulkan viewport convention. Then compare
billboard construction.

## Public Rust Instance Prep

Rusty XR already provides backend-neutral particle records and instance packing
in `rusty-xr-particles`. Downstream renderers can use this to keep CPU-side
payload layout shared while still owning Vulkan, OpenXR, Metal, WebGPU, or
engine-specific draw code.

```rust
use rusty_xr_particles::{
    write_particle_billboard_instances, ColorRgba, ParticleBillboardBuildConfig,
    ParticleBillboardSortCamera, ParticleRender, ParticleSceneBasis, Vec3,
};

let particles = [
    ParticleRender::new(Vec3::new(0.0, 0.0, -1.0), 0.08, ColorRgba::WHITE),
    ParticleRender::new(Vec3::new(0.1, 0.0, -2.0), 0.08, ColorRgba::WHITE),
];

let basis = ParticleSceneBasis::default();
let sort_camera = ParticleBillboardSortCamera {
    position: Vec3::ZERO,
    forward: Vec3::FORWARD_NEG_Z,
};
let mut sort_indices = Vec::new();
let mut instances = Vec::new();

let stats = write_particle_billboard_instances(
    &particles,
    basis,
    ParticleBillboardBuildConfig {
        sort_back_to_front: true,
        ..ParticleBillboardBuildConfig::default()
    },
    Some(sort_camera),
    &mut sort_indices,
    &mut instances,
);

assert_eq!(stats.emitted_count, instances.len());
```

Use `ParticleSceneBasis` for app-owned scene placement. A downstream OpenXR
shell can capture that basis once at spawn or recenter time, then keep using it
until the user requests a recenter. Do not rebuild the basis from live headset
orientation every frame unless the content is intentionally head anchored.

## Billboard Construction Modes

### Center-Projected

The center-projected path is usually cheaper because it projects one point per
particle instance and expands the local fan in projected space.

```glsl
// Inputs:
// - world_center: scene/reference-space particle center
// - corner: triangle-fan corner in [-0.5, 0.5]
// - size_m: particle diameter in meters
// - eye pose/FOV for the current view

vec2 center_ndc;
float center_forward_m;
float center_depth;
if (!project_world_to_eye_clip(
        eye, world_center, center_ndc, center_forward_m, center_depth)) {
    emit_clipped_particle();
    return;
}

vec2 half_size_ndc = vec2(
    size_m / (center_forward_m * max(fov_right - fov_left, 0.0001)),
    size_m / (center_forward_m * max(fov_up - fov_down, 0.0001))
);

vec2 ndc = center_ndc + corner * half_size_ndc;
gl_Position = vec4(
    ndc * center_forward_m,
    center_depth * center_forward_m,
    center_forward_m
);
```

Useful properties:

- lower vertex math and projection cost
- simpler center-based near-plane rejection
- good default for dense transparent particle fields
- still scene-stable when the particle center and eye projection path are
  correct

Tradeoff:

- the quad/disc is an approximation around the projected center, not a literal
  scene-space mesh

### World-Expanded Vertices

The world-expanded path builds the billboard fan in scene/reference space, then
projects every expanded vertex.

```glsl
// Inputs:
// - world_center: scene/reference-space particle center
// - corner: triangle-fan corner in [-0.5, 0.5]
// - size_m: particle diameter in meters
// - eye_right/eye_up: billboard axes derived from the current eye pose

vec3 vertex_world =
    world_center +
    eye_right * corner.x * size_m +
    eye_up * corner.y * size_m;

vec2 ndc;
float vertex_forward_m;
float vertex_depth;
if (!project_world_to_eye_clip(
        eye, vertex_world, ndc, vertex_forward_m, vertex_depth)) {
    emit_clipped_particle();
    return;
}

gl_Position = vec4(
    ndc * vertex_forward_m,
    vertex_depth * vertex_forward_m,
    vertex_forward_m
);
```

Useful properties:

- geometrically explicit: each fan vertex has a scene-space position
- helpful as a diagnostic when diffing projection and clipping behavior
- can match renderers that already operate on scene-expanded quads

Tradeoffs:

- more vertex projection work
- more near-plane edge cases because individual fan vertices can clip while the
  center remains visible
- can be more expensive under dense transparent particle fields

## Animated Billboard Masks

For animated particle masks, prefer normal texture sampling paths over manual
storage-buffer atlas lookup unless a device profile proves the opposite.

Common options:

| option | shape | when useful |
|---|---|---|
| `sampler2DArray`, nearest layer | one texture sample from a pre-baked frame layer | fastest simple animated lookup |
| `sampler2DArray`, blended layers | two neighboring layer samples plus `mix` | smoother animation with predictable cost |
| procedural mask | no texture memory dependency | diagnostics, simple effects, or when the procedural shape is visually acceptable |
| manual storage-buffer atlas | integer index into a packed buffer | only after profiling; can be costly in fragment-heavy paths |

Example blended texture-array lookup:

```glsl
layout(set = 0, binding = 0) uniform sampler2DArray u_mask_frames;

float animated_mask(vec2 uv, float frame01, float frame_count) {
    float frame_pos = clamp(frame01, 0.0, 0.999) * (frame_count - 1.0);
    float frame0 = floor(frame_pos);
    float frame1 = min(frame0 + 1.0, frame_count - 1.0);
    float t = fract(frame_pos);
    float a0 = texture(u_mask_frames, vec3(uv, frame0)).r;
    float a1 = texture(u_mask_frames, vec3(uv, frame1)).r;
    return mix(a0, a1, t);
}
```

Rusty XR includes a small deterministic morphed-ring atlas builder so examples
and downstream apps can generate a mask fixture without committing binary
assets:

```rust
use rusty_xr_particles::{build_morphed_ring_atlas_rgba, MorphedRingAtlasConfig};

let atlas = build_morphed_ring_atlas_rgba(MorphedRingAtlasConfig {
    frame_resolution: 64,
    frame_count: 32,
    atlas_columns: 8,
    ..MorphedRingAtlasConfig::default()
});

assert_eq!(atlas.rgba.len(), atlas.width * atlas.height * 4);
```

Downstream renderers can upload those frames as a 2D array texture, a regular
atlas texture, or another renderer-native resource. The public crate only owns
the deterministic CPU fixture.

## Trails And Frozen Animation Frames

`ParticleTrailEmitter` copies each source `ParticleRender` into a trail slot at
spawn time, then fades and scales that frozen copy. The copied `frame01` does
not continue to advance unless the downstream app explicitly changes that
policy.

That makes trail fragments a useful isolation target:

- If trails are meant to be frozen snapshots, avoid expensive live animation
  lookups on trail fragments when a cheaper branch preserves the intended
  visual.
- Mark trail instances with an app-owned flag or auxiliary field if the shader
  needs to route them differently.
- Profile the replacement branch. A procedural branch is not automatically
  cheaper than texture sampling on every device.

```rust
use rusty_xr_particles::{ColorRgba, ParticleRender, ParticleTrailConfig, ParticleTrailEmitter, Vec3};

let mut source = ParticleRender::new(Vec3::ZERO, 0.08, ColorRgba::WHITE);
source.frame01 = 0.25;

let mut trails = ParticleTrailEmitter::new(ParticleTrailConfig {
    enabled: true,
    visuals_enabled: true,
    lifetime_seconds: 1.0,
    copies_per_second: 10.0,
    max_spawn_batches_per_frame: 1,
    copies_per_particle: 2,
    size_multiplier: 0.75,
});

trails.update(0.11, &[source]);

source.frame01 = 0.75;
trails.update(0.01, &[source]);

let frozen = trails
    .particles()
    .iter()
    .find(|particle| particle.color.a > 0.0)
    .expect("one trail copy is active");
assert_eq!(frozen.frame01, 0.25);
```

## Budget Before GPU Profiling

Transparent particle fields can look cheap in CPU logs while becoming
fragment-heavy in the headset. Track expected draw pressure even before a GPU
trace is available:

```rust
use rusty_xr_particles::{particle_billboard_render_budget, DEFAULT_PARTICLE_DISC_SEGMENTS};

let budget = particle_billboard_render_budget(
    2_562, // source particles
    17_934, // active trail particles
    DEFAULT_PARTICLE_DISC_SEGMENTS,
);

println!(
    "instances={} indices={}",
    budget.visible_instances,
    budget.total_indices
);
```

This is not a GPU timer. It is a cheap sanity check that makes overdraw and
triangle-fan pressure visible in logs and profile manifests.

## Quest Isolation Workflow

When the headset shows pixel pops, tearing, or corruption while FPS and coarse
CPU/GPU load look healthy, avoid jumping straight to broad reductions. Isolate
one stage at a time:

1. Record `VrApi` or runtime rows for `FPS`, `Tear`, `Stale`, app time,
   `CPU&GPU`, GPU load, render scale, foveation, and refresh rate.
2. Add app-side timing windows for simulation, trail update, merge, instance
   build, buffer upload, command recording, draw recording, and submit.
3. Hold visible instance count constant while switching only the fragment mask:
   storage-buffer atlas, texture array, procedural, solid diagnostic.
4. Hold fragment shader constant while switching only trail visibility or trail
   count.
5. Hold shader and count constant while switching only billboard construction:
   center-projected versus world-expanded vertices.
6. Keep every run in an ignored artifact folder with a manifest of the changed
   runtime values.

For the full Quest capture workflow, see
[Quest Render Artifact Diagnostics](QUEST_RENDER_ARTIFACT_DIAGNOSTICS.md).

## Downstream Boundary

Rusty XR should expose reusable contracts, CPU fixtures, render-budget helpers,
and public examples. Downstream apps should own:

- exact shader style and visual identity
- scene simulation and response curves
- package identity, runtime profile names, and launch commands
- hardware captures, screenshots, traces, and app-specific measured constants
