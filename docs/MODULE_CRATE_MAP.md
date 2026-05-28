# Module And Crate Map

Rusty XR is a public foundation of small Rust crates and source examples. Core
crates should stay framework-neutral, dependency-light, and testable with
synthetic data.

## Dependency Direction

The default direction is:

1. Base data contracts.
2. Runtime configuration and schema/catalog contracts.
3. Deterministic math, parsing, validation, and utility helpers.
4. Diagnostic models and command evidence.
5. Source examples and optional adapters.
6. Downstream app shells.

Native Android, OpenXR, Vulkan, renderer, package identity, signing, release
payloads, and target-device validation belong in examples or downstream apps,
not in base crates.

## Base Contracts

| Crate or module | Role | Notes |
| --- | --- | --- |
| `rusty-xr-contracts` | Shared pose, timing, view, hand, room, render payload, plain layer, passthrough style, strobe, projection scorecard, effect-stack, and developer-home contracts. | This is the base crate for public data shapes. Keep it free of platform SDK calls and app policy. |
| `rusty-xr-contracts::effect_stack` | Data-only visual pass graphs, diagnostic taps, scalar reports, and layer comparison descriptors. | Public interface for visual-pipeline diagnostics; private visual recipes stay downstream. |
| `rusty-xr-contracts::home` | Developer-home panels, launcher entries, settings shortcuts, helper state, focus recovery, control-plane snapshots, and command evidence. | Models operator evidence without claiming to replace system Home or device-owner kiosk behavior. |
| `KioskCommandRunRecord` | Run-level envelope for command goal, provider evidence, fallback evidence, before/after control-plane state, surface intent, and outcome. | Use this when API, CLI, MCP, or manual routes need comparable kiosk evidence. |

## Runtime Config, Catalogs, And Profiles

| Crate or area | Role | Notes |
| --- | --- | --- |
| `rusty-xr-runtime-config` | Runtime keys, typed values, config maps, and Android property naming helpers. | Use for app-owned runtime profiles without hard-coding private launch identities. |
| `tools/schema` | Schema export and validation helpers. | Generated schemas are checked, not hand-maintained. |
| Example catalogs | Public Quest app catalog fixtures and build manifests. | Catalogs describe public examples only. Package identity remains example-owned. |

## Camera, Projection, Depth, SDF, Mesh, And Particles

| Crate or module | Role | Notes |
| --- | --- | --- |
| `rusty-xr-camera-model` | Camera metadata, projection helpers, timestamp matching, paired-camera diagnostics, and projection-space reports. | Renderer and provider integration remain adapter-owned. |
| `rusty-xr-depth-model` | Environment-depth contracts, range/confidence/cadence diagnostics, depth query summaries, and readback policy data. | Keep depth provider calls outside core. |
| `rusty-xr-sdf` | SDF primitives, packed grids, sparse TSDF snapshots, triangle mesh snapshots, and mesh-to-SDF reference conversion. | Accepts public mesh snapshots; does not own renderer or physics engines. |
| `rusty-xr-particles` | Deterministic particle storage, clocks, render payload helpers, mesh-surface sampling, hand-mesh fixture helpers, mesh fixture manifests, dynamic colliders, and SDF attraction. | Public topology hashes, manifest validation, and synthetic fixtures let downstream dynamic-mesh consumers test against public data; private behavior stays downstream. |

## Broker, Stream, Clock, Kiosk, And Quest Diagnostics

| Crate or module | Role | Notes |
| --- | --- | --- |
| `rusty-xr-broker-model` | Broker command envelopes, acknowledgements, stream manifests, sample headers, replay records, session manifests, transport offers/answers, ZeroMQ bridge manifests, packet descriptors, clock snapshots, stream health, broker panel descriptors, stream registry snapshots, and lease-aware command authority contracts. | Transport, socket, UI framework, device mutation, and codec implementation stay outside the model crate. |
| `rusty-xr-quest-diagnostics` | Quest readiness, provider status, tooling health, app/log/screenshot/file/perfetto/report models, and OpenXR/GLES feasibility statuses. | Device mutation must remain tool-gated by the caller. |
| `rusty-xr-contracts::home` | Kiosk/developer-home command evidence and focus-recovery event contracts. | Evidence records command goal, provider, fallback, foreground state, broker status, and operator intent. |
| `rusty-xr-contracts::home::KioskCommandRunRecord` | One public record shape for Rust API, broker API, CLI, MCP, and ADB fallback routes. | Exported as `home-kiosk-command-run-record.schema.json`. |

## Signal, Control, And Research Streams

| Crate | Role | Notes |
| --- | --- | --- |
| `rusty-xr-ble` | BLE permissions, adapter state, scan filters, GATT descriptors, and Android Bluetooth contracts. | Framework-neutral only. |
| `rusty-xr-lsl` | LSL stream descriptors, channel schemas, and sanitized stream roles. | Used by public examples and broker stream models. |
| `rusty-xr-osc` | OSC packets, type tags, bundles, and UDP helpers. | Runtime integration is example- or adapter-owned. |
| `rusty-xr-zmq` | Optional pure-Rust ZeroMQ adapter helpers, manifest-to-receiver config conversion, bounded receiver queues, and opt-in runtime receiver support. | Default builds remain socket-free; the runtime feature uses the pure Rust `zeromq` crate and does not link native `libzmq`. |
| `rusty-xr-polar` | Polar H10 HR/RR/ECG/ACC helper models and LSL descriptors. | Device connection policy stays downstream. |
| `rusty-xr-eye-model` | Screen-space gaze, XR gaze rays, AOI hits, derived processor events, and synthetic eye-data streams. | Provider adapters stay optional. |

## Debug And UI Data

| Crate | Role | Notes |
| --- | --- | --- |
| `rusty-xr-debug-canvas` | Diagnostic HUD documents, draw lists, tones, text runs, and input-neutral HUD commands. | Rendering and font atlas ownership stays in examples or downstream shells. |

## Public Example Packages

| Package or folder | Role |
| --- | --- |
| `examples/broker-client-probe` | Source-only Rust probe for broker status, commands, stream listing, console-open, and latency sampling. |
| `examples/quest-minimal-apk/native` | Minimal Rust-native Android smoke-test APK. |
| `examples/quest-composite-layer-apk/native` | Larger OpenXR/Vulkan diagnostic APK for public projection, depth, stream, HUD, and particle-map paths. |
| `examples/quest-gl-openxr-video-stack-apk/native` | OpenGL ES and OpenXR video-stack implementation lane. |
| `examples/quest-broker-apk` | Quest broker sidecar source tree and build helpers. |
| `examples/quest-broker-shell-helper` | Source-only Developer Mode shell-helper example. |
| `examples/makepad-camera-shell` | Excluded Makepad-first comparison lane. |

See [EXAMPLES_MATRIX.md](EXAMPLES_MATRIX.md) for validation commands and what
each example proves.

See [API_CLI_MCP_ENTRYPOINTS.md](API_CLI_MCP_ENTRYPOINTS.md) for the public
entrypoint map used by agents and operator tools.

## Current Boundary Read

The live workspace currently has coherent crate names. No crate split or rename
is justified until one of these conditions is true:

- A core crate starts owning native platform calls.
- A model crate accumulates unrelated behavior that cannot be described in one
  category above.
- A source example becomes the only place a reusable public data contract is
  tested.
- A downstream utility can pass the public extraction gate with synthetic tests
  and docs.

The current extraction pressure is best handled by documenting and extending
the existing public contracts before moving implementation code.
