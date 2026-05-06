# Dynamic Mesh To SDF

Rusty XR includes a framework-neutral CPU reference path for converting a
dynamic triangle mesh into a packed signed-distance field and using that field
to attract particles toward the mesh surface.

This utility is intentionally small and public:

```text
native, engine, scan, or fixture mesh provider
  -> TriangleMeshSnapshot
  -> build_sdf_from_mesh_bounds
  -> PackedSdfGrid
  -> step_particles_toward_sdf
  -> RenderPayload or adapter-owned simulation/renderer
```

It does not include OpenXR handle creation, renderer buffers, worker ownership,
GPU kernels, or app-specific simulation behavior.

## Core API

`rusty-xr-sdf` owns the mesh-to-SDF builder:

- `TriangleMeshSnapshot`
- `Bounds3`
- `MeshToSdfConfig`
- `MeshSdfSignMode`
- `build_sdf_from_mesh`
- `build_sdf_from_mesh_bounds`
- `PackedSdfGrid::sample`
- `PackedSdfGrid::sample_extrapolated`

`build_sdf_from_mesh` uses the mesh bounds plus config padding. Use
`build_sdf_from_mesh_bounds` when the SDF must cover nearby consumers, such as
a particle spawn volume around a hand.

`MeshSdfSignMode::ClosedMeshRaycast` expects a closed mesh and uses ray parity.
`MeshSdfSignMode::TriangleNormal` uses the closest triangle normal and is the
better first choice for open, thin, or runtime-deformed hand meshes.

## Particle Use Case

`rusty-xr-particles` provides a small CPU attraction helper:

- `SdfParticleAttractionConfig`
- `SdfParticleAttractionMode`
- `step_particles_toward_sdf`
- `CameraPose`
- `ParticleSphereSpawnConfig`
- `MeshSdfParticleAttractionScenarioConfig`
- `build_mesh_sdf_particle_attraction_scenario`

The stepper samples the SDF at each particle position, computes a surface
attraction acceleration from signed distance and normal, integrates velocity,
and updates the public `ParticleSet`. The resulting particles can be converted
to `RenderPayload` with the existing particle rendering helpers.

## Hand Mesh Runtime Flow

For Meta/OpenXR hand mesh work, the runtime adapter owns all platform calls and
coordinate conversion. Rusty XR core expects the adapter to emit deformed mesh
frames in app/world space:

```text
runtime hand mesh provider
  -> HandMeshSnapshot or TriangleMeshSnapshot
  -> optional LiveMeshSurfaceSampler for stable coordinates/neighbors
  -> build_sdf_from_mesh_bounds for a current field volume
  -> step_particles_toward_sdf or adapter-owned field consumer
```

The coordinate sampler and SDF builder can run side by side:

- use `LiveMeshSurfaceSampler` when stable mesh-attached coordinates and
  neighborhoods are needed;
- use `build_sdf_from_mesh_bounds` when particles, collision, or fields need a
  volumetric query around the current deformed mesh.

For live hands, rebuild the SDF on a worker thread or at a throttled cadence if
the voxel budget is high. The public builder is a deterministic CPU reference,
not the final high-throughput GPU path.

## Example

The portable example uses a hand mesh fixture and a particle sphere. It
rebuilds the SDF as the mesh moves and steps particles toward the surface:

```powershell
cargo run -p rusty-xr-particles --example hand_mesh_sdf_attraction
```

Expected output includes particle count, SDF sample count, affected particle
count, max speed, and SDF voxel count. A headset or native hand provider is not
required for this example.

## Boundary

Keep these in adapters or downstream apps:

- platform SDK calls and extension negotiation;
- OpenXR/Vulkan/Android renderer integration;
- worker scheduling and GPU SDF kernels;
- app-specific force fields, coupling, or visual behavior;
- captured meshes, logs, screenshots, package identity, and release payloads.

## Related Docs

- [Dynamic mesh coordinate sampling](DYNAMIC_MESH_COORDINATE_SAMPLING.md)
- [Dynamic mesh colliders](DYNAMIC_MESH_COLLIDERS.md)
- [Hand mesh particle runtime](HAND_MESH_PARTICLE_RUNTIME.md)
- [API surface review](API_SURFACE_REVIEW.md)
