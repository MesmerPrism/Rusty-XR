# Serialization And Schema Policy

Rusty XR keeps serialization opt-in. Pure Rust consumers should be able to use
the core crates without pulling in `serde`, JSON tooling, native SDKs, or
downstream app dependencies.

## Serde Feature

- Each public utility crate exposes a `serde` feature.
- The `serde` feature derives `Serialize` and `Deserialize` for stable public
  data contracts.
- Default features stay dependency-light.
- Crates that contain public contracts from another Rusty XR crate forward the
  dependency feature, for example `rusty-xr-particles/serde` enables
  `rusty-xr-contracts/serde`.
- Runtime adapters may serialize their own platform state, but private package
  identity, signing details, generated captures, and app-specific aliases must
  stay in downstream repos.

## Round-Trip Tests

Every crate with a `serde` feature should carry at least one representative
round-trip test. The CI baseline runs:

```powershell
cargo test --workspace --all-features
```

That verifies the optional serialization path without making serde mandatory
for normal builds.

## Schema Export

Rusty XR currently uses a small custom schema export script instead of
`schemars`. That is deliberate: the public contract shapes are still settling,
and hand-reviewed schemas are easier to keep stable while examples and adapters
are deferred.

Run:

```powershell
python tools/schema/export_schemas.py --out generated/schemas
```

The generated location is ignored by git. Commit schema artifacts only after the
contract has stabilized and the file has been reviewed as a public API.

CI validates that schema generation works with:

```powershell
python tools/schema/export_schemas.py --check
```

Quest app catalogs can also be checked directly:

```powershell
python tools/schema/check_quest_app_catalog.py tools/schema/fixtures/quest-app-catalog.example.json
```

## Initial Public Schemas

The initial export covers:

- Runtime configuration.
- Debug canvas documents and draw lists, once external renderer contracts
  stabilize.
- LSL telemetry and stream descriptors.
- OSC endpoint status and decoded packet records, once their downstream use
  stabilizes.
- Camera frame metadata.
- Depth frame summaries.
- Plain stereo layer descriptors.
- Quest session manifests.
- Quest app catalogs for Rusty XR Companion metadata.
- Capture source and room mesh source states.
- Semantic room mesh snapshots.
- Mesh fixture manifests.
- Polar accelerometer frames.
- Scan surface samples.
- Broker command envelopes, acknowledgements, stream manifests, stream sample
  headers, stream events, replay records, synthetic wave samples, session
  manifests, clock snapshots, clock stamps, clock correlations, clock health,
  and clock sync probes.
- Eye screen-gaze points, XR gaze rays, screen AOI hits, and derived processor
  events.
- Developer-home panel descriptors, home session state, launcher entries,
  settings shortcuts, and focus-recovery events.
- Effect-stack descriptors and comparison reports.
- Canvas/custom projection parity suite summaries, timing records, screen-space
  reports, projection mapping records, projection-coordinate contracts, and
  source-sampling contracts.
- Camera texture lane contracts and summaries for comparing direct HWB, direct
  OES, Makepad CPU-YUV, and Makepad HWB external resource architectures.
- Projection property hygiene summaries for launch wrappers that clear or gate
  persistent `debug.rustyxr.*` projection properties.
- Projection runtime readback reports that compare launch extras or Android
  property readbacks against the resolved manifest logged by the renderer.

Future schemas should be added only after the corresponding contract has tests
and a clear downstream use.
