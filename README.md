# Rusty XR

Rusty XR is a public Rust workspace for reusable XR contracts, utility modules,
and workflow helpers used by Rust-native XR applications.

The project is intended to provide a clean shared foundation for app shells and
experiments that need common XR data models, runtime configuration helpers,
device diagnostics, passthrough-camera abstractions, depth and SDF contracts,
LSL utilities, and general particle or animation primitives.

Rusty XR is not a Quest application, not an APK distribution repo, and not a
replacement for an app-specific shell. Application repositories remain
responsible for package identity, platform integration, rendering policy,
assets, product behavior, signing, release payloads, and validation on target
hardware.

## Workspace

Initial crate layout:

- `rusty-xr-contracts`: shared XR data contracts.
- `rusty-xr-runtime-config`: runtime configuration and launch-property helpers.
- `rusty-xr-lsl`: Lab Streaming Layer models and utilities.
- `rusty-xr-quest-diagnostics`: reusable Quest diagnostic status models.
- `rusty-xr-camera-model`: camera metadata and projection helpers.
- `rusty-xr-depth-model`: depth-frame and environment-depth contracts.
- `rusty-xr-sdf`: signed-distance-field and mesh snapshot contracts.
- `rusty-xr-particles`: general particle and animation primitives.

The crates are intentionally small at the start. The first milestone is to
stabilize public contracts and tests before moving higher-level adapters into
the workspace.

## Makepad Acknowledgement

Rusty XR is built in acknowledgement of the Makepad project and ecosystem.
Makepad provides a major foundation for Rust-native UI, rendering, platform, and
application work, and it has made ambitious Rust XR experimentation practical.
This repository is designed to provide reusable contracts and utilities around
that style of Rust-native development while clearly crediting Makepad's role as
an enabling core.

See [ACKNOWLEDGEMENTS.md](ACKNOWLEDGEMENTS.md) for more detail.

## Plan

The current implementation plan lives in
[docs/IMPLEMENTATION_PLAN.md](docs/IMPLEMENTATION_PLAN.md).

## License

Rusty XR is licensed under the MIT License.

