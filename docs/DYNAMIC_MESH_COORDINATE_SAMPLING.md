# Dynamic Mesh Coordinate Sampling

Rusty XR includes a framework-neutral utility for placing stable coordinates on
a triangle mesh and updating those coordinates as the mesh deforms.

This is the general module behind the hand-mesh particle example. It is not a
particle simulation by itself, and it does not require OpenXR, Vulkan, Android,
Unity, Makepad, or a Meta SDK dependency.

```text
native, engine, scan, or fixture provider
  -> TriangleMeshSurface
  -> LiveMeshSurfaceSampler
  -> MeshSurfaceSampleSet
  -> optional particles, fields, collisions, gestures, or app interactions
```

## Core API

`TriangleMeshSurface` is a plain indexed triangle mesh:

- `vertices: Vec<Vec3>`
- `triangles: Vec<[usize; 3]>`

`LiveMeshSurfaceSampler` turns a valid mesh into a stable
`MeshSurfaceSampleSet`. Each sample stores:

- current position
- current normal
- source triangle index
- barycentric coordinates inside that triangle

The first valid mesh is sampled with `MeshSurfaceSampleConfig`. Later meshes
with the same `MeshSurfaceTopologyKey` update the sample positions and normals
by re-evaluating the stored triangle and barycentric coordinates against the new
vertices. The sample identities are preserved.

If vertex count, triangle count, or index topology changes, the sampler returns
`ResampledTopology` and builds a new coordinate set.

## Neighborhoods

`MeshSurfaceSampleSet` stores two same-surface nearest-neighbor tiers. These
are intended for local interaction passes, such as influence, collision,
gesture propagation, or later simulation behavior.

The neighbor tiers are preserved during normal deformed-vertex updates so
callers can keep stable interaction identities. A caller can rebuild them when
it wants nearest neighbors in the current deformed pose:

```rust
sampler.rebuild_neighbor_tiers();
```

Two coordinate sets can also be linked with
`MeshSurfaceCrossNeighborhood`. This is useful for left/right hands or any pair
of dynamic surfaces that should be aware of nearby coordinates on the other
surface.

## Hand Mesh Flow

Hand meshes use the same generic path with one extra public contract:
`HandMeshSnapshot`. A native adapter converts runtime hand-mesh data into a
snapshot, then the particle crate converts the snapshot into
`TriangleMeshSurface`.

```text
runtime hand mesh provider
  -> HandMeshSnapshot
  -> TriangleMeshSurface
  -> LiveMeshSurfaceSampler or LiveHandMeshParticleSampler
  -> MeshSurfaceSampleSet
```

`LiveHandMeshParticleSampler` is a convenience wrapper for hand snapshots. It
keeps the generic sampled coordinate set and can also emit public
`RenderPayload`/particle records for examples.

## Meta/OpenXR Adapter Boundary

On Meta/OpenXR runtimes, an adapter can use `XR_FB_hand_tracking_mesh` to read
the immutable hand bind mesh and `XR_EXT_hand_tracking` joint locations for
per-frame deformation. The OpenXR documentation describes
`XrHandTrackingMeshFB` as containing joint, vertex, and index arrays, and notes
that the fully populated mesh data is immutable during the corresponding
instance lifetime and intended to be combined with per-frame
`xrLocateHandJointsEXT` data.

Rusty XR core does not create OpenXR handles. The adapter is responsible for:

- extension negotiation
- hand tracker lifetime
- bind-mesh retrieval
- joint-location polling
- skinning or provider-side deformation
- reference-space choice
- coordinate-system conversion into the app's render convention
- deciding when a snapshot is usable

The public Quest example demonstrates this adapter-owned shape and then feeds
the resulting `HandMeshSnapshot` frames into the same sampler used by the
portable fixture example.

## Visualization

Particles are one consumer of the coordinate set. The same samples can also
drive:

- hand/surface influence fields
- collision or proximity queries
- gesture-local state
- mesh-attached annotations
- debug draw payloads
- future interaction or simulation modules

App-specific oscillator, coupling, force, or visual-effect behavior stays in
consumer crates or downstream app shells.

For volumetric field consumers, use the companion dynamic mesh-to-SDF utility:
`TriangleMeshSnapshot` can be converted into `PackedSdfGrid`, and particles can
be stepped toward the current SDF surface. See
[dynamic mesh to SDF](DYNAMIC_MESH_TO_SDF.md).

For physics or proximity consumers, use the dynamic mesh collider utility:
`TriangleMeshSurface` or `HandMeshSnapshot` can be converted into
collider-ready mesh data and an optional diagnostic shell. See
[dynamic mesh colliders](DYNAMIC_MESH_COLLIDERS.md).

The portable fixture example does not require headset hardware:

```powershell
cargo run -p rusty-xr-particles --example dynamic_mesh_coordinates
```

## References

- Khronos OpenXR `XR_FB_hand_tracking_mesh`:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/XR_FB_hand_tracking_mesh.html>
- Khronos OpenXR `XrHandTrackingMeshFB`:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/XrHandTrackingMeshFB.html>
- Khronos OpenXR `XrHandJointLocationsEXT`:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/XrHandJointLocationsEXT.html>
