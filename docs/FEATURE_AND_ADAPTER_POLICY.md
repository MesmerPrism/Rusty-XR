# Feature And Adapter Policy

Rusty XR is a public utility core. Core crates stay framework-neutral; native
platform integration belongs behind explicit features or separate adapter
crates.

## Core Crates

Core crates may contain:

- Plain Rust data contracts.
- Deterministic validation, layout, parsing, and math helpers.
- Optional `serde` support behind a feature.
- Docs, rustdoc examples, schema exports, and synthetic tests.

Core crates must not contain:

- Android, OpenXR, Vulkan, Makepad, Unity, Meta SDK, WinRT, or native `liblsl`
  calls.
- Runtime package identity, launch aliases, signing data, release catalogs, or
  generated captures.
- Private visual stacks, private simulation behavior, private stream names, or
  project-specific study logic.

## Feature Names

Use these names when the dependency graph stays small and the adapter is clearly
optional:

- `serde`: opt-in serialization for public contracts.
- `makepad`: Makepad conversion helpers only.
- `openxr`: OpenXR conversion helpers only.
- `quest`: Quest-specific public status or configuration helpers only.
- `android`: Android platform conversion helpers only.
- `lsl-native`: native `liblsl` transport bindings.
- `ble-native`: native Bluetooth transport bindings.

If a feature would pull in a large SDK, renderer, app lifecycle, build system,
or platform service, prefer a separate adapter crate instead of a core feature.

## Adapter Requirements

Before adding an adapter:

- Document which platform APIs it touches.
- Keep conversion boundaries thin and typed.
- Add synthetic tests that do not require hardware.
- Run the public boundary scanner.
- Keep app package names, signing, captured media, local paths, and release
  payloads out of the public repo.

## Examples Stay Deferred

Public examples should demonstrate stable APIs. Do not add examples that exist
mainly to stabilize temporary wiring. Add CI, serialization policy, schemas,
scanner/provenance, rustdoc, and adapter policy first.
