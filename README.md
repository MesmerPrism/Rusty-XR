# Rusty XR

Rusty XR is a public Rust workspace for reusable XR contracts, utility modules,
and workflow helpers used by Rust-native XR applications.

The project is intended to provide a clean shared foundation for app shells and
experiments that need common XR data models, runtime configuration helpers,
device diagnostics, passthrough-camera abstractions, depth and SDF contracts,
plain stereo/feedback layer descriptors and tuning hints, LSL utilities, and
general particle or animation primitives.

Rusty XR is not a Quest application, not an APK distribution repo, and not a
replacement for an app-specific shell. Application repositories remain
responsible for package identity, platform integration, rendering policy,
assets, product behavior, signing, release payloads, and validation on target
hardware.

## Workspace

Initial crate layout:

- `rusty-xr-contracts`: shared XR data contracts.
- `rusty-xr-runtime-config`: runtime configuration and launch-property helpers.
- `rusty-xr-ble`: framework-neutral BLE and Android Bluetooth contracts.
- `rusty-xr-lsl`: Lab Streaming Layer models and utilities.
- `rusty-xr-polar`: Polar H10 data contracts and protocol helpers.
- `rusty-xr-quest-diagnostics`: reusable Quest diagnostic status models.
- `rusty-xr-camera-model`: camera metadata and projection helpers.
- `rusty-xr-depth-model`: depth-frame and environment-depth contracts.
- `rusty-xr-sdf`: signed-distance-field and mesh snapshot contracts.
- `rusty-xr-particles`: general particle and animation primitives.

The crates are intentionally small at the start. The first milestone is to
stabilize public contracts and tests before moving higher-level adapters into
the workspace.

## Examples

The public examples are synthetic contracts-only demos:
[crates/rusty-xr-contracts/examples/README.md](crates/rusty-xr-contracts/examples/README.md).
The first APK-producing example is a minimal Rust-native Android smoke test:
[examples/quest-minimal-apk/README.md](examples/quest-minimal-apk/README.md).
The first immersive Quest example is a Rust/OpenXR/Vulkan APK with explicit
synthetic, CPU diagnostic, GPU-buffer probe, and paired-camera GPU projection
tiers. MediaProjection is optional and is used only to stream the final headset
screen back to Windows for inspection:
[examples/quest-composite-layer-apk/README.md](examples/quest-composite-layer-apk/README.md).
The current raw-camera example has two public projected stereo modes:
`display-screen-homography` and `quad-surface`. Both use the paired Camera2
GPU-buffer path and the public soft feedback border, but `quad-surface` is
still an A/B comparison profile rather than the final performance or color
reference.

The contracts examples and minimal APK can be run without headset hardware or
downstream app code:

```powershell
cargo run -p rusty-xr-contracts --example plain_stereo_feedback_layout --features serde
cargo run -p rusty-xr-contracts --example composite_feedback_session --features serde
powershell -ExecutionPolicy Bypass -File .\examples\quest-minimal-apk\tools\Build-QuestMinimalApk.ps1
```

The immersive Quest example requires a Quest-compatible OpenXR loader and
hardware validation:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-composite-layer-apk\tools\Build-QuestCompositeLayerApk.ps1 -OpenXrLoaderPath C:\path\to\libopenxr_loader.so
```

Quest raw camera, platform passthrough, environment depth, MediaProjection, and
operator casting sources are intentionally distinct. See
[docs/QUEST_VISUAL_SOURCE_TAXONOMY.md](docs/QUEST_VISUAL_SOURCE_TAXONOMY.md)
before interpreting camera-composite diagnostics.

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

The GitHub Pages documentation layer lives in [docs/index.html](docs/index.html)
and is designed to publish directly from the repository `docs/` folder.

The Android / Quest APK responsibility split is documented in
[docs/ANDROID_QUEST_APK_BUILDING.md](docs/ANDROID_QUEST_APK_BUILDING.md).
The source workspace and catalog verification flow with Rusty XR Companion
Apps is documented in
[docs/RUSTY_XR_COMPANION_INTEGRATION.md](docs/RUSTY_XR_COMPANION_INTEGRATION.md).

Media-pipeline streaming and APK permission guidance is documented in
[docs/MEDIA_PIPELINE_AND_PERMISSIONS.md](docs/MEDIA_PIPELINE_AND_PERMISSIONS.md).

BLE, LSL, and Polar H10 data-pipeline guidance is documented in
[docs/BLE_LSL_POLAR_PIPELINE.md](docs/BLE_LSL_POLAR_PIPELINE.md).

General-purpose XR tool import candidates are tracked in
[docs/GENERAL_TOOL_IMPORT_AUDIT.md](docs/GENERAL_TOOL_IMPORT_AUDIT.md).
The broader sanitized machine-wide repository/tooling audit is tracked in
[docs/MACHINE_REPO_TOOLING_AUDIT.md](docs/MACHINE_REPO_TOOLING_AUDIT.md).
Serialization/schema policy, adapter policy, and provenance rules are tracked in
[docs/SERIALIZATION_AND_SCHEMA_POLICY.md](docs/SERIALIZATION_AND_SCHEMA_POLICY.md),
[docs/FEATURE_AND_ADAPTER_POLICY.md](docs/FEATURE_AND_ADAPTER_POLICY.md), and
[docs/PROVENANCE.md](docs/PROVENANCE.md).

Current public API review notes are tracked in
[docs/API_SURFACE_REVIEW.md](docs/API_SURFACE_REVIEW.md). The shared catalog
boundary with Rusty XR Companion Apps is tracked in
[docs/RUSTY_XR_COMPANION_INTEGRATION.md](docs/RUSTY_XR_COMPANION_INTEGRATION.md).

## AI Agent Skill

A raw, portable AI-agent skill for working on this repository lives at
[skills/rusty-xr-builder/SKILL.md](skills/rusty-xr-builder/SKILL.md). It is
written so it can be copied into Codex-style or Claude-style skill workflows.

For local Quest verification with Rusty XR Companion Apps, keep both public
repos as siblings under one workspace folder and run the companion CLI
workspace guide from the companion repo:

```powershell
dotnet run --project .\src\RustyXr.Companion.Cli -- workspace guide --root <workspace>
```

## License

Rusty XR is licensed under the MIT License.
