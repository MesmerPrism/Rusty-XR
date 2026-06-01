# Quest Hand Mesh Capture System And Data Formats

Status: public-safe explanation for downstream Quest/OpenXR hand-mesh recorder
examples and Rusty Matter export tooling.

## System Overview

The hand-mesh capture system records hand movement from a Quest OpenXR app as a
Matter capture bundle, validates that bundle, and exports an animated binary
glTF file for standard 3D tools.

The capture bundle remains the authoritative source. The GLB is the portable
asset. Validation mesh frames bridge the two by preserving baked runtime mesh
deformation for inspection and parity checks.

## Component Boundaries

Rusty XR owns the headset runtime boundary:

- OpenXR extension negotiation;
- hand tracker lifetime;
- provider hand-mesh bind-data acquisition;
- per-frame hand polling;
- renderer/example integration;
- app-local capture writing.

Rusty Matter owns the geometry and animation model:

- rig schema;
- topology keys;
- bind mesh and skeleton data;
- runtime-to-bind joint mappings;
- joint clips;
- baked validation mesh frames;
- validation reports;
- GLB export semantics.

Host tooling owns installation and evidence movement while the route is still a
development workflow:

- install;
- launch;
- status polling;
- command-file adapter;
- artifact pull;
- local evidence collection.

Manifold should become command/session authority when the route is promoted:

- accepted and rejected commands;
- run ids;
- artifact cataloging;
- export request routing;
- audit records.

High-rate mesh or clip payloads should not be routed through low-rate JSON
command responses.

## Workflow

The expected workflow is:

1. Build and install a foreground Quest OpenXR recorder app.
2. Launch it idle.
3. Start a bounded capture through a host tool or future Manifold command.
4. Pull the app-owned capture bundle.
5. Validate the capture bundle through Matter.
6. Export animated GLB through Matter.
7. Parse the GLB back and inspect it visually.

The validation gate should prove:

- required hands are present;
- frame timestamps are monotonic;
- rig topology and array counts are consistent;
- runtime-to-bind mapping resolves;
- baked validation mesh frames match the rig topology;
- final status is complete;
- GLB export readiness is true.

## Data Formats

### Capture Manifest

`capture.manifest.json` is the bundle index. It records:

- session id;
- request id;
- runtime provider;
- reference space;
- coordinate convention;
- required OpenXR extensions;
- artifact filenames;
- frame and drop counts.

It is metadata, not high-rate animation data.

### Rig Files

`left.rig.json` and `right.rig.json` store the rest-state hand mesh and
skeleton:

- handedness;
- topology key;
- bind version;
- joint names, parents, radii, and bind poses;
- bind vertices and normals;
- triangle indices;
- UVs when available;
- vertex blend indices and weights;
- runtime joint set and mapping metadata.

The provider topology observed in one Quest hand-mesh run was 26 bind joints,
1360 vertices, 6942 indices, and 2314 triangles per hand. Treat those as
observed provider values, not constants. Captures should validate their own
topology keys and array counts.

### Joint Clips

`left.clip.jsonl` and `right.clip.jsonl` are newline-delimited animation clips.
Each line is one frame containing:

- frame index;
- timestamp;
- runtime joint poses;
- tracking flags;
- confidence;
- tip lengths;
- pinch strengths.

The clip records bone/joint movement. It does not duplicate every mesh vertex
on every frame.

### Validation Mesh Frames

`left.validation_mesh.jsonl` and `right.validation_mesh.jsonl` store baked mesh
frames. Each line contains already-deformed vertices and normals for a sampled
frame.

These files are useful for:

- direct visual inspection;
- wireframe or vertex debugging;
- handedness checks;
- coordinate-space checks;
- comparing exported skinning against runtime deformation.

They are heavier than skeletal clips and are best treated as evidence and
parity witnesses, not the main portable asset format.

### Status Stream

`status.jsonl` records the session lifecycle inside the pulled bundle:

- started;
- recording;
- close reason;
- complete or failed;
- frame counters;
- dropped-frame count;
- last error.

This makes a pulled bundle self-describing.

### Matter Validation Report

The Matter validation report checks the bundle and marks export readiness. The
important result is whether validation mesh playback and skinned GLB export are
ready.

### GLB

The GLB is the standard portable animated asset. It contains:

- mesh vertices;
- normals;
- triangle indices;
- skin weights;
- skeleton nodes;
- inverse bind matrices;
- translation and rotation keyframes.

The vertices are still present in the GLB. If a viewer does not show points or
wireframes, that is a viewer display choice. A viewer can render the same GLB
as shaded surfaces, wireframes, vertex points, bones, or combinations of these.

### Export Report

The GLB export report should record:

- source capture id;
- output asset path or catalog id;
- exported hand count;
- vertex, triangle, joint, frame, and animation-channel counts;
- mapping source;
- parse-back result;
- warnings and errors.

## Most Comprehensive Export

The most comprehensive export is a package, not only a GLB:

```text
capture.manifest.json
left.rig.json
right.rig.json
left.clip.jsonl
right.clip.jsonl
left.validation_mesh.jsonl
right.validation_mesh.jsonl
status.jsonl
matter-hand-capture-validation.json
animated-hand-capture.glb
matter-hand-animation-export-report.json
```

Use GLB for standard asset consumption. Keep the full Matter bundle when a
workflow needs re-export, validation, debugging, raw tracking metadata, or
future export targets.

## Next Hardening Steps

- Add GLB viewer toggles for wireframe, points, bones, and validation-frame
  comparison.
- Add damaged-input tests for missing or inconsistent rig, clip, and mapping
  data.
- Add optional frame decimation with explicit interpolation policy.
- Compare sampled GLB skinning against baked validation mesh frames.
- Promote the host wrapper into Manifold-commanded sessions and Host-managed
  artifact cataloging.
