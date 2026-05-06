# Dynamic Mesh Colliders

Rusty XR includes a framework-neutral helper for turning the current pose of a
dynamic triangle mesh into collider-ready geometry plus an optional diagnostic
visual shell.

The utility is data-only:

```text
native, engine, scan, or hand-mesh provider
  -> TriangleMeshSurface or HandMeshSnapshot
  -> DynamicMeshCollider
  -> collider surface + optional diagnostic shell
  -> adapter-owned physics engine and renderer
```

It does not cook native physics objects, create OpenXR handles, create renderer
buffers, or own collision callbacks. App shells and optional adapters remain
responsible for those integration details.

## Core API

`rusty-xr-particles` owns the public mesh-collider helper because it already
owns dynamic mesh surfaces, hand-mesh snapshot conversion, and particle/debug
render payloads:

- `DynamicMeshCollider`
- `DynamicMeshColliderConfig`
- `DynamicMeshColliderDiagnosticConfig`
- `DynamicMeshColliderUpdate`
- `DynamicMeshColliderUpdateStatus`
- `DynamicMeshColliderDiagnosticShell`
- `DynamicMeshColliderContact`
- `build_dynamic_mesh_collider_surface`

`DynamicMeshCollider::update_from_mesh` accepts any valid `TriangleMeshSurface`.
`DynamicMeshCollider::update_from_hand_mesh_snapshot` accepts public
`HandMeshSnapshot` frames, which lets a Meta/OpenXR hand-mesh adapter feed live
deformed hands into the same path.

The update result reports vertex count, triangle count, topology key, whether a
convex collider was requested, whether the current triangle count is eligible
for a convex adapter path, and how much diagnostic shell geometry was produced.

## Surface Inflation

`DynamicMeshColliderConfig::surface_inflation_meters` moves vertices outward
along vertex normals. If a `HandMeshSnapshot` contains runtime normals, those
normals are used. Otherwise the helper generates averaged mesh normals. If a
normal is degenerate, it falls back to the direction from mesh center to the
vertex, then to `Vec3::UP`.

This is useful when a downstream physics engine needs a slightly oversized
contact surface around a thin or fast-moving mesh. A public adapter can pass
the inflated surface to its own mesh-collider cooking step.

## Diagnostic Shell

The diagnostic shell is intentionally separate from the collider surface. It is
another `TriangleMeshSurface` with a `ColorRgba` tint and an additional shell
inflation amount. A renderer adapter can draw this shell as a translucent
wireframe or solid overlay without changing the actual collision surface.

This mirrors a useful engine-side workflow: keep the real collider geometry
owned by physics, but expose an oversized visual shell so hand alignment,
scale, and transform anchoring can be checked in headset.

## Queries

The helper includes small CPU queries for examples and diagnostics:

- `closest_point(point)` returns the nearest point, triangle normal, distance,
  and triangle index.
- `overlaps_sphere(center, radius)` uses closest-point distance plus
  `contact_padding_meters`.

These are not a replacement for a native physics engine. They are useful for
unit tests, simple probes, and keeping renderer/physics adapters honest before
native collider handles are introduced.

## Hand Mesh Flow

For live Meta/OpenXR hand mesh work, the runtime adapter owns extension
negotiation, hand tracker lifetime, per-frame deformation, reference-space
choice, and coordinate conversion. Rusty XR expects the adapter to emit
`HandMeshSnapshot` frames in the app's render convention:

```text
runtime hand mesh provider
  -> HandMeshSnapshot
  -> DynamicMeshCollider::update_from_hand_mesh_snapshot
  -> collider_surface
  -> adapter-owned MeshCollider, trigger, or proximity shape
```

The same live snapshots can also feed `LiveHandMeshParticleSampler` for stable
surface coordinates and neighborhoods, and `TriangleMeshSnapshot` conversion
for SDF fields.

## Example

The source-only example uses the portable hand mesh fixture, updates the
collider from an initial and deformed hand snapshot, generates a diagnostic
shell, and runs simple closest-point and sphere-overlap probes:

```powershell
cargo run -p rusty-xr-particles --example hand_mesh_dynamic_collider
```

A headset is not required for this example. A native hand-mesh adapter should
replace the fixture snapshot with live `HandMeshSnapshot` frames.

## Adapter Notes

When adapting this utility to an engine or native physics stack:

- keep transform scale handling explicit so baked vertices are not scaled
  twice;
- rebuild or update the native collider when `ChangedTopology` is returned;
- treat invalid surfaces as recoverable provider states;
- keep convex cooking behind triangle-count limits such as the engine's
  supported convex-mesh maximum;
- keep trigger flags, collision layers, materials, and callbacks adapter-owned;
- draw the diagnostic shell from `DynamicMeshColliderDiagnosticShell`, not from
  the physics object.

## Related Docs

- [Dynamic mesh coordinate sampling](DYNAMIC_MESH_COORDINATE_SAMPLING.md)
- [Dynamic mesh to SDF](DYNAMIC_MESH_TO_SDF.md)
- [Hand mesh particle runtime](HAND_MESH_PARTICLE_RUNTIME.md)
- [API surface review](API_SURFACE_REVIEW.md)
