# Rusty XR Implementation Plan

This document tracks the public implementation plan for Rusty XR.

Rusty XR is a reusable Rust workspace for XR contracts, utility modules, and
workflow helpers. It is designed to support app shells and experiments without
forcing those apps to share package identity, assets, rendering behavior, or
release workflows.

## Design Principles

- Keep shared contracts small, typed, and framework-neutral.
- Keep platform-specific integration behind optional adapters.
- Prefer plain Rust data at boundaries between app shells and experiments.
- Keep runtime configuration explicit and testable.
- Treat Quest/OpenXR diagnostics as reusable status models and helpers rather
  than app-specific scripts.
- Provide public examples that demonstrate the reusable APIs without requiring
  access to a downstream application.
- Preserve attribution for upstream foundations, especially Makepad.

## Target Workspace Areas

### XR Contracts

Shared pose, frame timing, eye-view, hand, render-payload, diagnostics, and
command contracts. The contract crate also owns public room mesh source state,
semantic room mesh snapshots, and capture lifecycle metadata because those
types are shared by depth, scan, diagnostics, examples, and operator tooling.

Initial requirement: no dependency on Android, OpenXR, Vulkan, Makepad, Unity,
Meta SDKs, or LSL.

### Runtime Configuration

Reusable parsing and validation for runtime settings, launch properties, and
configuration sources. Downstream applications can map their own names onto the
generic model.

### LSL Utilities

Reusable Lab Streaming Layer models and utility code for discovery, inlet and
outlet status, staleness handling, roundtrip checks, and telemetry payloads.

The native backend should stay optional so the pure models can compile and test
without a system LSL runtime.

### BLE And Polar H10 Utilities

Reusable BLE/GATT contracts, Android Bluetooth permission models, Polar H10
GATT identifiers, HR/RR decoding, ECG/ACC PMD frame decoding, PMD command
builders, and LSL schemas.

Android adapters, runtime permission prompts, native `liblsl` bindings, network
configuration, and hardware validation stay in app shells or optional adapter
crates.

### Quest Diagnostics

Reusable status models and helpers for interpreting common Quest development
signals such as package launch state, frame timing, runtime properties, and
device readiness.

### Camera Model

Reusable camera contracts and math helpers, including camera metadata,
intrinsics, extrinsics, stereo frame descriptors, and projection-related
utilities.

### Plain Stereo And Feedback Layers

Reusable descriptors for app-owned projected mono/stereo media layers,
side-by-side or separate-eye source layouts, aspect-fit content rectangles, and
simple visual feedback/content borders. Public tuning may include border-only
coverage/radius/edge/feedback scalar values and adapter performance hints for
custom stereo and optional screen-feedback layers.

These contracts do not implement the Passthrough Camera API, MediaProjection,
OpenXR composition submission, Vulkan texture import, or downstream
image-processing, geometric-effect, scene, or product tuning stacks.

### Native Platform Passthrough Descriptors

Reusable descriptors for compositor-owned Meta/OpenXR passthrough layers:
runtime reconstruction, projected mesh passthrough, placement as underlay or
overlay, opacity, edge rendering, mono color maps, brightness/contrast/
saturation, and 3D color-LUT bindings.

These contracts are data-only. They do not create OpenXR handles, submit
`XrCompositionLayerPassthroughFB`, upload triangle meshes, allocate native LUTs,
sample camera pixels, or ship downstream visual-effect behavior. See
[META_PASSTHROUGH_LAYER.md](META_PASSTHROUGH_LAYER.md).

### Depth Model

Reusable depth-frame and environment-depth contracts that can be consumed by app
shells, experiments, and analysis tools.

### SDF Model

Reusable signed-distance-field contracts, mesh snapshots, packed grids, sparse
views, and small reference utilities.

### Particle And Animation Primitives

Reusable particle state, simulation, interaction, animation, and render-payload
contracts that can run independently of a specific app shell.

### Optional Framework Adapters

Framework-specific adapters may be added after the public contracts settle.
Adapters should remain thin and optional.

Feature and adapter naming rules are documented in
[FEATURE_AND_ADAPTER_POLICY.md](FEATURE_AND_ADAPTER_POLICY.md). Core crates stay
framework-neutral; native platform integration should be either an explicit
small feature or a separate adapter crate.

### Serialization, Schemas, And Provenance

Optional serialization is available behind crate-level `serde` features. The
policy and schema-export workflow are documented in
[SERIALIZATION_AND_SCHEMA_POLICY.md](SERIALIZATION_AND_SCHEMA_POLICY.md).

Lightweight public provenance metadata lives in `provenance/` and is described
in [PROVENANCE.md](PROVENANCE.md). Exact private source paths and implementation
notes remain in the private planning workspace.

### AI Agent Skill

The public repo carries a raw AI-agent skill at
`skills/rusty-xr-builder/SKILL.md`. It should stay sanitized and portable so it
can be adapted into Codex-style or Claude-style skill systems without depending
on private downstream repos.

### GitHub Pages Documentation

The public documentation site lives in `docs/` and can be served by GitHub
Pages from the main branch. It should summarize crate roles, architecture,
boundaries, diagrams, and validation workflows without publishing private
planning notes or raw graph/audit artifacts.

CI checks Pages/local links, schema generation, and the public boundary scanner
before examples are added.

### Companion App Catalogs

Rusty XR Companion Apps is the public Windows operator repository for local APK
install/launch, device profiles, casting, diagnostics, and future public
example APK metadata. Rusty XR core owns the shared catalog schema shape:
`quest-app-catalog.schema.json`, currently versioned as
`rusty.xr.quest-app-catalog.v1`.

### General Tool Imports

Reusable Makepad/Rusty-XR-family tool candidates are tracked in
[GENERAL_TOOL_IMPORT_AUDIT.md](GENERAL_TOOL_IMPORT_AUDIT.md). Prefer extracting
plain contracts first: canvas/ray interaction, hand-menu anchors, menu command
models, room mesh snapshots, scan package manifests, sparse TSDF snapshots, and
geometry generators. Framework-native widgets, native OpenXR/Vulkan/Android
calls, and downstream app behavior stay in adapters or downstream app shells.

A broader sanitized machine-wide audit is tracked in
[MACHINE_REPO_TOOLING_AUDIT.md](MACHINE_REPO_TOOLING_AUDIT.md). It covers
additional Quest ADB/`hzdb`, Windows streaming, room mesh/depth, PCA/capture,
runtime profile, GPU layout, BLE/LSL/biofeedback, and clean-room import tooling
candidates from local sibling repositories.

### Android / Quest APK Shells

Rusty XR should support Rust-based Android and Quest app shells without
becoming an app-specific APK repository. The public build responsibility split
is documented in [ANDROID_QUEST_APK_BUILDING.md](ANDROID_QUEST_APK_BUILDING.md).
Concrete package names, signing, install scripts, release payloads, and headset
validation stay in downstream app repositories.

Media-pipeline streaming and permission handling are documented in
[MEDIA_PIPELINE_AND_PERMISSIONS.md](MEDIA_PIPELINE_AND_PERMISSIONS.md). Public
tools may include generic Windows receivers and protocol helpers, but app-side
capture and headset permission prompts remain shell responsibilities.

Current Quest media implementation status:

- Public Rusty XR includes camera/depth metadata, permission guidance, a generic
  Windows frame receiver, plain stereo layer descriptors, border tuning,
  composite-feedback tuning, performance hints, visual feedback border layout,
  and native platform passthrough style descriptors.
- Public Rusty XR now includes a clean Quest example APK that demonstrates a
  Camera2/headset-camera custom layer with OpenXR/Vulkan submission, Android
  hardware-buffer import, GPU sampling, paired left/right Camera2 streams,
  public Camera2 intrinsics/pose metadata, display-eye screen-to-camera
  homographies, a quad-surface comparison profile, and the public soft
  visual-feedback border. The fullscreen mapping and quad-surface mapping both
  exercise the core stereo camera stack, but the quad-surface profile remains
  an optimization/color-parity task rather than the final performance
  reference.
- MediaProjection remains a final-screen inspection stream only; it is not the
  raw camera source for the custom layer.
- Future native support should still stay in thin optional adapters or public
  examples rather than becoming private app-shell behavior inside core crates.

BLE, LSL, and Polar H10 data-pipeline integration is documented in
[BLE_LSL_POLAR_PIPELINE.md](BLE_LSL_POLAR_PIPELINE.md). Public crates may model
GATT paths, permission requirements, protocol payloads, and LSL stream schemas;
actual Bluetooth connections and native LSL transport code remain shell
responsibilities.

## Milestones

### Milestone 1: Public Skeleton

- [x] Create repository skeleton.
- [x] Add MIT license.
- [x] Add Makepad acknowledgement.
- [x] Add public implementation plan.
- [x] Add initial Rust workspace and crate placeholders.
- [x] Add CI for formatting, tests, clippy, rustdoc, docs links, schema export,
  and public boundary scanning.

### Milestone 2: Contracts

- [ ] Implement core pose, timing, camera, hand, render-payload, and diagnostics
  contracts.
- [ ] Add tests for contract behavior and serialization where applicable.
- [ ] Publish first tagged version once the contract crate is usable.

### Milestone 3: SDF And Particle Foundations

- [ ] Add general SDF and mesh snapshot contracts.
- [ ] Add particle and animation primitives.
- [ ] Add tests for deterministic simulation and payload generation.

### Milestone 4: Runtime Config And Diagnostics

- [ ] Add runtime configuration parsing and validation.
- [ ] Add reusable Quest diagnostic models.
- [ ] Add documentation for downstream app integration.

### Milestone 5: Camera And Depth Utilities

- [ ] Add camera model helpers.
- [ ] Add depth model helpers.
- [ ] Add public examples that demonstrate the reusable APIs.

### Milestone 6: Optional Adapters

- [ ] Add optional framework adapters only after the contracts are stable.
- [ ] Keep adapters small and feature-gated.
- [ ] Document which crates are framework-neutral and which require optional
  integrations.

### Milestone 7: Utility Foundation Before Examples

- [x] Add CI baseline for fmt, tests, all-features tests, clippy, doctests,
  rustdoc, docs links, schemas, and public boundary scan.
- [x] Add opt-in `serde` features and representative round-trip tests.
- [x] Add custom public schema export script with generated output ignored.
- [x] Add public clean-room boundary scanner CLI and public-safe config.
- [x] Add public provenance metadata format and initial utility entries.
- [x] Add feature/adapter policy docs.
- [x] Add first synthetic public example for a contracts-only feedback layout.
- [x] Add room mesh/capture lifecycle contracts and companion catalog schema
  alignment.
- [x] Add synthetic composite feedback session example.
- [x] Add first minimal Rust-native Android APK smoke-test example.
- [x] Add native platform passthrough style descriptors and synthetic examples.
- [x] Add safety-gated visual strobe descriptors and synthetic frequency-plan
  examples.
- [ ] Re-audit utility surface before tagging.
- [ ] Add hardware, OpenXR, passthrough, media-capture, depth, and other
  native-adapter examples only after the utility surface review passes.

## Tracking Table

| Area | Status | Notes |
| --- | --- | --- |
| Repository skeleton | `[x]` | Initial public workspace created. |
| License | `[x]` | MIT. |
| Makepad acknowledgement | `[x]` | Root README and acknowledgements file. |
| CI baseline | `[x]` | GitHub Actions covers fmt, tests, all features, clippy, doctests, rustdoc, docs links, schema export, and public boundary scan. |
| Core contracts | In progress | Initial framework-neutral pose, timing, view, camera, depth, hand, render payload, layer, and counter contracts added. |
| Runtime config | In progress | Generic key/value parsing and Android property naming helpers added. |
| BLE utilities | In progress | Framework-neutral BLE UUID, GATT path, notification, operation, scan result, and Android permission models added. |
| LSL utilities | In progress | Pure stream descriptor, stream role, channel schema, discovery filter, endpoint status, staleness, roundtrip, biofeedback, and telemetry models added. |
| Polar H10 utilities | In progress | Public Polar GATT IDs, HR/RR decoder, uncompressed ECG/ACC PMD decoders, PMD command builders, and LSL schemas added. |
| Quest diagnostics | In progress | Generic readiness, package launch, and frame-rate status models added. |
| Camera model | In progress | Intrinsics scaling, projection, back-projection, and timestamp matching helpers added. |
| Plain stereo / feedback layers | In progress | Public mono/stereo media layer descriptors, source UV layout helpers, aspect-fit content rectangles, visual feedback border segments, border tuning, composite-feedback tuning, and performance hints added. |
| Native platform passthrough descriptors | In progress | Public Meta/OpenXR layer-purpose, placement, opacity, edge, color-map, BCS, and LUT descriptors added with contracts-only examples. |
| Visual strobe descriptors | In progress | Public full-field and passthrough-LUT strobe profile descriptors, display-frame frequency plans, 120 Hz constraints, and safety warnings added with a no-hardware example. |
| Depth model | In progress | Depth readiness, frame summary, per-view metadata, infinite-far range, cadence, and readback-policy helpers added. |
| SDF model | In progress | Packed SDF grid, sampling, bounds, triangle mesh snapshots, and data-only depth support/impact query contracts added. |
| Particle and animation primitives | In progress | Minimal particle state, fixed-step clock, and render payload generation added. |
| XR canvas and hand interaction | In progress | Public ray/canvas hit-test contracts, hand-menu anchors, activation modes, and hand influence points added. |
| Sparse scan / TSDF contracts | In progress | Public sparse TSDF samples, snapshots, scan surface samples, and scan-fusion stats added. |
| Room mesh and capture lifecycle | In progress | Public room mesh source state, semantic room mesh snapshots, and capture lifecycle/source metadata added. |
| Companion app catalog alignment | In progress | `quest-app-catalog` schema version aligned with Rusty XR Companion Apps catalog metadata. |
| General tool import audit | In progress | Makepad/Rust-XR-family candidates plus broader machine-wide Quest/tooling candidates documented with public/downstream boundaries. |
| GitHub Pages docs | In progress | Static public docs layer added under `docs/` with Mermaid diagrams and sanitized architecture guidance. |
| Serialization and schemas | In progress | Opt-in `serde` features, round-trip tests, and custom schema export script added. |
| Boundary scanner and provenance | In progress | Public scanner CLI/config and public utility provenance metadata added. |
| Feature and adapter policy | `[x]` | Adapter feature names, separate-crate rule, and pre-adapter boundary requirements documented. |
| Public examples | In progress | Synthetic layout, composite feedback, passthrough style catalog, audio-reactive passthrough style, and visual strobe profile examples added; minimal Rust-native Android APK smoke test added; first camera-driven Quest OpenXR/Vulkan custom-layer example added with optional MediaProjection screen streaming. |
| Optional adapters | `[ ]` | Deferred until contracts stabilize. |
