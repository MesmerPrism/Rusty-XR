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
command contracts.

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

### Quest Diagnostics

Reusable status models and helpers for interpreting common Quest development
signals such as package launch state, frame timing, runtime properties, and
device readiness.

### Camera Model

Reusable camera contracts and math helpers, including camera metadata,
intrinsics, extrinsics, stereo frame descriptors, and projection-related
utilities.

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

## Milestones

### Milestone 1: Public Skeleton

- [x] Create repository skeleton.
- [x] Add MIT license.
- [x] Add Makepad acknowledgement.
- [x] Add public implementation plan.
- [x] Add initial Rust workspace and crate placeholders.
- [ ] Add CI for formatting, tests, and clippy.

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

## Tracking Table

| Area | Status | Notes |
| --- | --- | --- |
| Repository skeleton | `[x]` | Initial public workspace created. |
| License | `[x]` | MIT. |
| Makepad acknowledgement | `[x]` | Root README and acknowledgements file. |
| Core contracts | `[ ]` | Next implementation step. |
| Runtime config | `[ ]` | Planned crate placeholder exists. |
| LSL utilities | `[ ]` | Planned crate placeholder exists. |
| Quest diagnostics | `[ ]` | Planned crate placeholder exists. |
| Camera model | `[ ]` | Planned crate placeholder exists. |
| Depth model | `[ ]` | Planned crate placeholder exists. |
| SDF model | `[ ]` | Planned crate placeholder exists. |
| Particle and animation primitives | `[ ]` | Planned crate placeholder exists. |
| Public examples | `[ ]` | Planned after contracts are usable. |
| Optional adapters | `[ ]` | Deferred until contracts stabilize. |

