# Mesh Fixture Manifest

Rusty XR mesh fixtures are public, synthetic descriptions of mesh/topology test
data. They let a downstream dynamic-mesh consumer pressure-test topology,
sampling, SDF/depth, particle, collider, and render-payload utilities without
using recordings, calibration data, headset captures, private tuning constants,
or app-specific behavior.

The manifest contract lives in `rusty-xr-particles`:

- `MeshFixtureManifest`
- `MeshFixtureKind`
- `MeshFixtureNeighborTier`
- `MeshFixtureFrameRange`
- `MeshFixtureValidationExpectation`
- `MeshFixtureIntendedUse`
- `MeshFixtureProvenance`

The schema identifier is:

```text
rusty.xr.mesh_fixture_manifest.v1
```

The exported JSON Schema is:

```text
mesh-fixture-manifest.schema.json
```

## Public Fixtures

`rusty-xr-particles` exposes deterministic manifest builders for small
public-safe fixtures:

- `synthetic_grid_mesh_fixture_manifest`
- `synthetic_icosphere_mesh_fixture_manifest`
- `synthetic_deforming_grid_mesh_fixture_manifest`
- `synthetic_hand_mesh_fixture_manifest`

The matching mesh generators are:

- `build_fixture_grid_mesh`
- `build_fixture_icosphere_mesh`
- `build_fixture_deforming_grid_frames`
- existing `build_fixture_hand_mesh`

Committed JSON examples live in `fixtures/mesh/*.manifest.json`.

These fixtures are intentionally small. The manifests record fixture id, mesh
kind, topology key/hash, vertex count, index count, coordinate sample count,
coordinate space/convention/units, winding order, index format, expected
neighbor tiers, motion kind, allowed deformation frames, validation
expectations, intended uses, and provenance.

## Validation

Use the manifest helpers to verify internal consistency and generated-surface
counts:

```rust
let manifest = rusty_xr_particles::synthetic_deforming_grid_mesh_fixture_manifest();
manifest.validate()?;
manifest.validate_deformation_frame_count(4)?;
```

The source-only example can print the public manifests:

```powershell
cargo run -p rusty-xr-particles --example mesh_fixture_manifest --features serde
```

Schema generation is checked with:

```powershell
python tools\schema\export_schemas.py --check
```

## Boundary

Mesh fixtures should stay synthetic, generated, or public example data. Do not
put captured meshes, calibration constants, provider recordings, app-specific
effect behavior, headset screenshots, local paths, or private package identity
into fixture manifests.
