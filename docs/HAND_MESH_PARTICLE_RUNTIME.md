# Hand Mesh Particle Runtime

Rusty XR does not ship a Unity hand-mesh project or a Meta SDK adapter in core.
Engine projects and platform samples are reference material only. The public
implementation boundary is Rust-native and framework-neutral:

```text
native or engine adapter
  -> HandMeshSnapshot
  -> LiveHandMeshParticleSampler
  -> MeshSurfaceSampleSet
  -> RenderPayload
```

## Runtime Ownership

The adapter owns platform calls, feature negotiation, session timing, and
coordinate-space conversion. On Meta/OpenXR runtimes, the relevant public
extension is `XR_FB_hand_tracking_mesh`, whose API includes
`xrGetHandMeshFB`. The Rusty XR core crates do not create OpenXR handles or
link a platform SDK for this path.

The adapter should emit one `HandMeshSnapshot` per hand when live mesh data is
available. A snapshot contains the currently deformed vertices plus stable
triangle indices. If the runtime also provides normals, skinning indices, or
weights, those can travel in the same snapshot, but the particle sampler only
requires vertices and triangle indices.

## Particle Sampling

`LiveHandMeshParticleSampler` spreads a requested number of coordinates over
the first valid hand-mesh topology. Each coordinate stores:

- current position
- current normal
- source triangle index
- barycentric anchor inside that triangle

On later frames with the same `HandMeshTopologyKey`, the sampler does not
resample. It re-evaluates every barycentric anchor against the new deformed
vertex positions, so particle identity and neighbor lists remain stable while
the hand animates.

When the topology key changes, the sampler rebuilds the coordinate set and
nearest-neighbor tiers. This is the correct fallback if a runtime swaps mesh
resolution, hand side, index buffer, or provider mode.

## Neighborhoods

The sampler builds two same-surface nearest-neighbor tiers for local
interaction passes. It also exposes `MeshSurfaceCrossNeighborhood`, which can
link sampled coordinates from two hands. These are graph/topology utilities
only; oscillator, coupling, force, or app-specific behavior belongs in a
consumer crate or downstream shell.

## Availability

The real hand mesh is available only after the runtime session and hand-mesh
provider are ready. Startup code should handle `NoSnapshot` until the provider
returns a valid frame. Frame loops should treat invalid snapshots and invalid
surfaces as recoverable provider states, keep rendering the last valid payload
if desired, and resample when topology changes.

## Synthetic Example

The public example uses a procedural hand-like mesh because it is portable and
does not require headset hardware:

```powershell
cargo run -p rusty-xr-particles --example synthetic_hand_mesh_samples
```

That example exercises the same sampler, live deformed-mesh update path,
render-payload conversion, and cross-hand neighborhood construction that a
native adapter uses with runtime hand-mesh frames.

## References

- Khronos OpenXR `XR_FB_hand_tracking_mesh`:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/XR_FB_hand_tracking_mesh.html>
