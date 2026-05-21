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
- `runtime`: adapter-crate runtime support when the crate itself is already an
  optional adapter rather than a core model crate.

If a feature would pull in a large SDK, renderer, app lifecycle, build system,
or platform service, prefer a separate adapter crate instead of a core feature.

## External Media And Control SDKs

External low-latency, codec, media, browser, or vendor-control SDKs should not
be pulled into core crates. Model their public-facing requirements as
framework-neutral contracts first, then use a separate adapter, tool, sidecar,
or downstream shell after dependency and license review.

Public core may contain:

- capability descriptors
- session offer/answer contracts
- timing and network-quality metrics
- security-policy descriptors
- adapter-neutral endpoint metadata
- provider-neutral Quest development tool descriptors such as `hzdb`/ADB/MCP
  capability manifests, operation safety classes, docs/API search result
  shapes, and trace/session report metadata

Public core must not contain:

- vendor SDK headers, source files, examples, or binaries
- copied proprietary packet layouts or wire formats
- native codec/media payloads
- release payloads that require separate commercial terms
- installed MCP server configs that mutate a user's editor or project without
  an explicit operator action

## Adapter Requirements

Before adding an adapter:

- Document which platform APIs it touches.
- Keep conversion boundaries thin and typed.
- Add synthetic tests that do not require hardware.
- Run the public boundary scanner.
- Keep app package names, signing, captured media, local paths, and release
  payloads out of the public repo.

For ZeroMQ specifically, keep native `libzmq` bindings out of public core.
Pure Rust runtime support belongs in the optional `rusty-xr-zmq` adapter crate
and must stay disabled by default.

## Examples Stay Deferred

Public examples should demonstrate stable APIs. Do not add examples that exist
mainly to stabilize temporary wiring. Add CI, serialization policy, schemas,
scanner/provenance, rustdoc, and adapter policy first.
