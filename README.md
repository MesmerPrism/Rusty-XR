# Rusty XR

Rusty XR is a public Rust workspace for reusable XR contracts, utility modules,
and workflow helpers used by Rust-native XR applications.

The project is intended to provide a clean shared foundation for app shells and
experiments that need common XR data models, runtime configuration helpers,
device diagnostics, passthrough-camera abstractions, depth and SDF contracts,
plain stereo/feedback layer descriptors and tuning hints, native
passthrough-layer style descriptors, safety-gated visual strobe descriptors,
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
- `rusty-xr-ble`: framework-neutral BLE and Android Bluetooth contracts.
- `rusty-xr-debug-canvas`: normalized debug/test canvas and diagnostic HUD
  state primitives.
- `rusty-xr-lsl`: Lab Streaming Layer models and utilities.
- `rusty-xr-osc`: Open Sound Control packet and UDP helpers.
- `rusty-xr-polar`: Polar H10 data contracts and protocol helpers.
- `rusty-xr-quest-diagnostics`: reusable Quest diagnostic status models.
- `rusty-xr-camera-model`: camera metadata and projection helpers.
- `rusty-xr-depth-model`: depth-frame and environment-depth contracts.
- `rusty-xr-sdf`: signed-distance-field and mesh snapshot contracts.
- `rusty-xr-particles`: general particle, animation, mesh-surface sampling,
  live hand-mesh particle anchors, and neighborhood primitives.

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
tiers. It also includes an explicit environment-depth diagnostics profile that
starts the OpenXR environment-depth provider, logs depth resolution, near/far
range, runtime capture timestamps, acquire cost, observed depth cadence, and
confidence availability, and renders a stereo grayscale depth texture diagnostic
in headset. It also includes generated depth-mesh and retained local-space
particle overlays for validating live environment-depth surface reconstruction
over native passthrough:
[examples/quest-composite-layer-apk/README.md](examples/quest-composite-layer-apk/README.md).
The current particle anchoring limitation and scene-owned follow-up plan are
documented in
[docs/ENVIRONMENT_DEPTH_PARTICLE_ANCHORING.md](docs/ENVIRONMENT_DEPTH_PARTICLE_ANCHORING.md).
The same APK has an optional generic diagnostic HUD path that can be driven
from runtime configuration, ADB hotload intents, or future controller, LSL,
OSC, and app-specific input adapters. MediaProjection is optional and is used
only to stream the final headset screen back to Windows for inspection.
Diagnostic HUD stereo rendering options are documented in
[docs/DIAGNOSTIC_HUD_STEREO_RENDERING.md](docs/DIAGNOSTIC_HUD_STEREO_RENDERING.md).
The current raw-camera example has two public projected stereo modes:
`display-screen-homography` and `quad-surface`. Both use the paired Camera2
GPU-buffer path and the public soft feedback border, but `quad-surface` is
still an A/B comparison profile rather than the final performance or color
reference.
Current camera color and cadence work is tracked as explicit runtime profiles
and reusable headset-run tools:
[docs/QUEST_CAMERA_PROFILE_WORKFLOW.md](docs/QUEST_CAMERA_PROFILE_WORKFLOW.md).
The first broker APK proof-of-concept is a separate Quest sidecar service with
localhost status/WebSocket endpoints, optional LSL forwarding, OSC latency
egress, runtime-configurable OSC-to-WebSocket drive events, generic published
stream events, Polar-compatible broker bio streams, and a 2D broker console that
XR clients can open through the broker command API. The console also includes a
normal-mode Launcher page for named app lists, visible launchable-app search,
and PackageManager-based app launching from the headset. It now also exposes a
camera-projection metadata provider, a bounded app-context Camera2 capture
probe, bounded app-context raw-luma and H.264 Camera2 side-channel probes,
an Android platform MediaCodec decode-consumption probe, shell-helper status
surface, and video-lab streams for stream manifests, sample metadata, and
timing metrics while keeping production camera streaming, texture import,
OpenXR eye views, and layer submission client-owned. The composite-layer
example can also run a broker H.264 consumer probe that requests the broker
stream over localhost, consumes the binary packets in the composite app
process, and decodes them with platform MediaCodec into a Java-owned
SurfaceTexture external texture or into hardware buffers that the existing
Vulkan GPU-buffer probe imports and draws into the OpenXR projection layer. The
hardware-buffer mode also latches selected Camera2 intrinsics and pose metadata
from the broker stream-start ack when the broker can report it, so the native
projection-readiness logs can distinguish "metadata available" from "missing
projection inputs" before a stereo projected path is selected.
The public
Unity comparison target is
[The Big Red Button Institute](https://github.com/MesmerPrism/the-big-red-button-institute),
which drives one visible Quest button through direct Unity OSC/BLE input and
broker-routed stream events. A source-only Rust broker client probe is included
so Rust tools can exercise the same status, command, stream-list,
subscription, console-open, and latency-sample path without adopting a specific
async runtime:
[examples/quest-broker-apk/README.md](examples/quest-broker-apk/README.md).
[examples/broker-client-probe/README.md](examples/broker-client-probe/README.md).

The particle crate includes a Rust-native dynamic mesh coordinate sampler that
keeps an even coordinate set stable across deformed mesh updates, carries
same-surface and cross-surface neighborhoods, and rebuilds only when mesh
topology changes. The hand-mesh path consumes public `HandMeshSnapshot` frames
from a native provider, so Meta/OpenXR hand mesh data can be adapted without
putting platform calls in the core crate. See
[docs/DYNAMIC_MESH_COORDINATE_SAMPLING.md](docs/DYNAMIC_MESH_COORDINATE_SAMPLING.md)
and
[docs/HAND_MESH_PARTICLE_RUNTIME.md](docs/HAND_MESH_PARTICLE_RUNTIME.md).
An optional source-only ADB shell helper example can be built as a dex jar and
launched with `adb shell app_process` to report shell-helper status to the
broker, including optional bounded MediaCodec and shell-visible camera metadata
probes, plus a guarded Camera2 open/one-frame capture feasibility probe and
metadata-only synthetic encoded-stream events. It can also emit a bounded
synthetic encoded-packet stream over ADB-forwarded TCP so binary payloads stay
off the broker JSON path, and includes a guarded MediaCodec synthetic-Surface
encoder probe plus a guarded shell `screenrecord` display-source probe behind
the same framing. It is Developer Mode tooling, not an installed APK permission:
[examples/quest-broker-shell-helper/README.md](examples/quest-broker-shell-helper/README.md).
Quest ADB input smoke-test limits are documented in
[docs/QUEST_ADB_INPUT_WORKFLOW.md](docs/QUEST_ADB_INPUT_WORKFLOW.md).
Headset-local app launching and the boundary between normal PackageManager
launches and ADB-launched shell helpers is documented in
[docs/QUEST_APP_LAUNCHING_AND_SHELL_HELPERS.md](docs/QUEST_APP_LAUNCHING_AND_SHELL_HELPERS.md).

The contracts examples and minimal APK can be run without headset hardware or
downstream app code:

```powershell
cargo run -p rusty-xr-contracts --example plain_stereo_feedback_layout --features serde
cargo run -p rusty-xr-contracts --example composite_feedback_session --features serde
cargo run -p rusty-xr-contracts --example meta_passthrough_style_catalog --features serde
cargo run -p rusty-xr-contracts --example audio_reactive_passthrough_style --features serde
cargo run -p rusty-xr-contracts --example visual_strobe_profiles --features serde
cargo run -p rusty-xr-particles --example dynamic_mesh_coordinates
powershell -ExecutionPolicy Bypass -File .\examples\quest-minimal-apk\tools\Build-QuestMinimalApk.ps1
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-apk\tools\Build-QuestBrokerApk.ps1
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-shell-helper\tools\Build-BrokerShellHelper.ps1
cargo run -p rusty-xr-broker-client-probe -- status
```

The immersive Quest example requires a Quest-compatible OpenXR loader and
hardware validation:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-composite-layer-apk\tools\Build-QuestCompositeLayerApk.ps1 -OpenXrLoaderPath C:\path\to\libopenxr_loader.so
```

Quest raw camera, platform passthrough, environment depth, MediaProjection, and
operator casting sources are intentionally distinct. See
[docs/QUEST_VISUAL_SOURCE_TAXONOMY.md](docs/QUEST_VISUAL_SOURCE_TAXONOMY.md)
before interpreting camera-composite diagnostics. Native compositor passthrough
style contracts are documented in
[docs/META_PASSTHROUGH_LAYER.md](docs/META_PASSTHROUGH_LAYER.md).
Intentional visual strobe profiles, 120 Hz display-refresh constraints, and
photoepilepsy warnings are documented in
[docs/VISUAL_STROBE_PROFILES.md](docs/VISUAL_STROBE_PROFILES.md).

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
[docs/MEDIA_PIPELINE_AND_PERMISSIONS.md](docs/MEDIA_PIPELINE_AND_PERMISSIONS.md),
including the boundary for Companion's optional managed FFmpeg preview runtime:
FFmpeg remains an external/user-managed media sidecar, while Quest-side
encoded-video examples use Android platform MediaCodec.

BLE, LSL, and Polar H10 data-pipeline guidance is documented in
[docs/BLE_LSL_POLAR_PIPELINE.md](docs/BLE_LSL_POLAR_PIPELINE.md).
OSC live-control and sensor-ingress guidance is documented in
[docs/OSC_ADAPTER.md](docs/OSC_ADAPTER.md).

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
