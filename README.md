# Rusty XR

Rusty XR is a public Rust workspace for reusable XR contracts, utility modules,
and workflow helpers used by Rust-native XR applications.

The project is intended to provide a clean shared foundation for app shells and
experiments that need common XR data models, runtime configuration helpers,
device diagnostics, passthrough-camera abstractions, depth and SDF contracts,
plain stereo/feedback layer descriptors and tuning hints, native
passthrough-layer style descriptors, safety-gated visual strobe descriptors,
effect-stack diagnostic descriptors, LSL utilities, and general particle or
animation primitives.
The research-streaming surface now also includes source-only broker stream and
engine-neutral eye-data contracts before native provider adapters.

Rusty XR is not a Quest application, not an APK distribution repo, and not a
replacement for an app-specific shell. Application repositories remain
responsible for package identity, platform integration, rendering policy,
assets, product behavior, signing, release payloads, and validation on target
hardware.
Rusty Kiosk developer-home and kiosk-like launcher concepts are modeled as data
contracts only: panels, launcher entries, settings shortcuts, helper state, and
bounded focus-recovery events. They do not make Rusty XR a system UI replacement
or an MDM/device-owner kiosk implementation.

## Workspace

For the current public documentation map, start with
[docs/README.md](docs/README.md). The short route for agents and contributors
is:

- [Module and crate map](docs/MODULE_CRATE_MAP.md)
- [Examples matrix](docs/EXAMPLES_MATRIX.md)
- [API, CLI, and MCP entrypoints](docs/API_CLI_MCP_ENTRYPOINTS.md)
- [Public extraction workflow](docs/PUBLIC_EXTRACTION_WORKFLOW.md)
- [Validation commands](docs/VALIDATION.md)
- [Feature and adapter policy](docs/FEATURE_AND_ADAPTER_POLICY.md)
- [Serialization and schema policy](docs/SERIALIZATION_AND_SCHEMA_POLICY.md)

Initial crate layout:

- `rusty-xr-contracts`: shared XR data contracts.
- `rusty-xr-runtime-config`: runtime configuration and launch-property helpers.
- `rusty-xr-ble`: framework-neutral BLE and Android Bluetooth contracts.
- `rusty-xr-debug-canvas`: normalized debug/test canvas and diagnostic HUD
  state primitives.
- `rusty-xr-lsl`: Lab Streaming Layer models and utilities.
- `rusty-xr-osc`: Open Sound Control packet and UDP helpers.
- `rusty-xr-zmq`: optional pure-Rust ZeroMQ adapter helpers.
- `rusty-xr-broker-model`: broker command, stream-manifest, sample-header, and
  transport-lane contracts, plus synthetic stream and replay payload shapes.
- `rusty-xr-eye-model`: screen-space gaze, XR gaze-ray, AOI, processor-event,
  and synthetic eye-data contracts.
- `rusty-xr-polar`: Polar H10 data contracts and protocol helpers.
- `rusty-xr-quest-diagnostics`: reusable Quest diagnostic status, tooling
  provider, `hzdb`/ADB/MCP, health, app, log, screenshot, file, Perfetto,
  docs/API, and artifact-report models.
- `rusty-xr-camera-model`: camera metadata and projection helpers.
- `rusty-xr-depth-model`: depth-frame and environment-depth contracts.
- `rusty-xr-sdf`: signed-distance-field, mesh snapshot, sparse TSDF, and
  dynamic mesh-to-SDF reference utilities.
- `rusty-xr-particles`: general particle, animation, mesh-surface sampling,
  live hand-mesh particle anchors, dynamic mesh collider helpers, SDF
  attraction, and neighborhood primitives.

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
tiers. It also includes explicit environment-depth profiles that start the
OpenXR environment-depth provider, log depth resolution, near/far range,
runtime capture timestamps, acquire cost, observed depth cadence, and
confidence availability, and render live depth diagnostics in headset. The
current mapping path includes a generated depth mesh, a retained local-space
particle overlay, and a scene-owned particle map that anchors accepted depth
samples in OpenXR local space, actively clears visible free-space cells from
high-confidence observations, and renders small opaque default-disc particles
over native passthrough:
[examples/quest-composite-layer-apk/README.md](examples/quest-composite-layer-apk/README.md).
The projection-space and scene-map policy are documented in
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
The public parity workplan for moving the accepted custom stereo Camera2
projection path from visible/correct to smooth with GPU headroom is tracked in
[docs/CAMERA_STEREO_PROJECTION_PARITY_WORKPLAN.md](docs/CAMERA_STEREO_PROJECTION_PARITY_WORKPLAN.md).
The public roadmap from the current broker/composite diagnostics toward online
two-way Quest-to-Quest stereo streaming is tracked in
[docs/QUEST_TO_QUEST_ONLINE_STREAMING_ROADMAP.md](docs/QUEST_TO_QUEST_ONLINE_STREAMING_ROADMAP.md).
The current Q2Q plan keeps the Android MediaCodec/`RXYRVID1` diagnostic path
through laptop-loop, LAN, and first relay milestones while adding
camera/source capability manifests, timestamp domains, H.264 invariants,
runtime media controls, temporal smoothing, and session-native projection
metadata before WebRTC or WebTransport adapters.
The first temporal reprojection plan for bounding visible projection motion,
frame adoption, depth-aware fallback, and optional space-warp probes is tracked
in
[docs/CUSTOM_STEREO_CAMERA_TEMPORAL_REPROJECTION.md](docs/CUSTOM_STEREO_CAMERA_TEMPORAL_REPROJECTION.md).
The first broker APK proof-of-concept is a separate Quest sidecar service with
localhost status/WebSocket endpoints, optional LSL forwarding, OSC latency
egress, runtime-configurable OSC-to-WebSocket drive events, generic published
stream events, Polar-compatible broker bio streams, and a 2D broker console that
XR clients can open through the broker command API. The console also includes a
normal-mode Launcher page for named app lists, visible launchable-app search,
PackageManager-based app launching from the headset, plus a Clock page backed
by a broker-owned elapsed-realtime timebase that cooperating apps can query and
use for stream/storage stamps. It now also exposes a
camera-projection metadata provider, a bounded app-context Camera2 capture
probe, bounded app-context raw-luma and H.264 Camera2 side-channel probes,
an Android platform MediaCodec decode-consumption probe, shell-helper status
surface, and video-lab streams for stream manifests, sample metadata, and
timing metrics while keeping release-grade camera streaming, texture import,
OpenXR eye views, and layer submission client-owned. The composite-layer
example can also run broker H.264 consumer probes that request bounded or
live-bounded broker streams over localhost, consume the binary packets in the
composite app process, and decode them with platform MediaCodec into a
Java-owned SurfaceTexture external texture or into hardware buffers that the
existing Vulkan GPU-buffer path imports and draws into the OpenXR projection
layer. The stereo live-bounded probe carries selected Camera2 intrinsics and
pose metadata, decodes paired left/right streams as packets arrive, and can
submit the decoded hardware buffers through the `gpu-projected` OpenXR stereo
path. It remains a diagnostic streaming path; unbounded sessions, timestamp
pairing under jitter, remote-device validation, and release-grade performance
are still future work.
The public
Unity comparison target is
[The Big Red Button Institute](https://github.com/MesmerPrism/the-big-red-button-institute),
which drives one visible Quest button through direct Unity OSC/BLE input and
broker-routed stream events. A source-only Rust broker client probe is included
so Rust tools can exercise the same status, command, stream-list,
subscription, console-open, and latency-sample path without adopting a specific
async runtime:
[examples/quest-broker-apk/README.md](examples/quest-broker-apk/README.md).
[docs/BROKER_CLOCK_AND_TIMEBASE.md](docs/BROKER_CLOCK_AND_TIMEBASE.md).
[examples/broker-client-probe/README.md](examples/broker-client-probe/README.md).

The particle crate includes a Rust-native dynamic mesh coordinate sampler that
keeps an even coordinate set stable across deformed mesh updates, carries
same-surface and cross-surface neighborhoods, and rebuilds only when mesh
topology changes. The hand-mesh path consumes public `HandMeshSnapshot` frames
from a native provider, so Meta/OpenXR hand mesh data can be adapted without
putting platform calls in the core crate. See
[docs/MESH_FIXTURE_MANIFEST.md](docs/MESH_FIXTURE_MANIFEST.md),
[docs/DYNAMIC_MESH_COORDINATE_SAMPLING.md](docs/DYNAMIC_MESH_COORDINATE_SAMPLING.md)
and
[docs/HAND_MESH_PARTICLE_RUNTIME.md](docs/HAND_MESH_PARTICLE_RUNTIME.md).
It also includes a framework-neutral dynamic mesh collider helper that can turn
the current deformed mesh into collider-ready surface geometry plus an optional
diagnostic visual shell for adapter-owned physics and renderer integrations.
See [docs/DYNAMIC_MESH_COLLIDERS.md](docs/DYNAMIC_MESH_COLLIDERS.md).
It also includes a generic dynamic mesh-to-SDF path: `rusty-xr-sdf` can convert
a `TriangleMeshSnapshot` into a `PackedSdfGrid`, and `rusty-xr-particles` can
step public particles toward that SDF as a source-only use case example. See
[docs/DYNAMIC_MESH_TO_SDF.md](docs/DYNAMIC_MESH_TO_SDF.md).
Particle billboard construction, animation-mask lookup tradeoffs, trail
snapshot behavior, and Quest particle-renderer isolation workflows are
documented in
[docs/PARTICLE_BILLBOARD_AND_ANIMATION_PERFORMANCE.md](docs/PARTICLE_BILLBOARD_AND_ANIMATION_PERFORMANCE.md).
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
A standalone Makepad-first Quest comparison lane is available at
[examples/makepad-q2q-camera-shell/README.md](examples/makepad-q2q-camera-shell/README.md).
It uses Makepad's Android/OpenXR packaging surface, exercises `makepad-xr`, and
emits a synthetic Rusty XR status marker before camera or broker behavior is
added. The marker values already route through `rusty-xr-runtime-config`, so
the lane is anchored to the same public core as the custom APK examples.
Current device validation reaches the generated XR activity, but GPU page
faults in the Quest log are still an active Makepad-lane blocker; the same
symptom reproduced with Makepad's upstream XR example on the same headset. The
depth-stack comparison showed useful differences against the non-Makepad
composite example, but provider start, per-frame acquire/readback, and depth
image view creation are not required for the fault. Later splits also faulted
without passthrough creation, without environment-depth provider/swapchain
creation, with zero composition layers submitted, without OpenXR color swapchain
creation, without the OpenXR frame loop, without OpenXR session creation, and
without Makepad OpenXR instance creation. A same-APK launch of the normal
Makepad Android activity also reproduced the page-fault class. The current
isolation target is Makepad's base Android graphics/activity path on Quest /
Horizon OS; the attempt log is tracked in
[docs/MAKEPAD_XR_GPU_PAGE_FAULT_INVESTIGATION.md](docs/MAKEPAD_XR_GPU_PAGE_FAULT_INVESTIGATION.md).
Quest ADB input smoke-test limits are documented in
[docs/QUEST_ADB_INPUT_WORKFLOW.md](docs/QUEST_ADB_INPUT_WORKFLOW.md).
Headset-local app launching and the boundary between normal PackageManager
launches and ADB-launched shell helpers is documented in
[docs/QUEST_APP_LAUNCHING_AND_SHELL_HELPERS.md](docs/QUEST_APP_LAUNCHING_AND_SHELL_HELPERS.md).
Public Rusty Kiosk / developer-home menu contracts for broker panels, launcher
entries, settings shortcuts, helper state, bounded focus-recovery events, and
control-plane status snapshots are
documented in
[docs/QUEST_DEVELOPER_HOME_MENU.md](docs/QUEST_DEVELOPER_HOME_MENU.md).
Generic multi-pass visual pipeline descriptors and layer comparison reports are
documented in
[docs/EFFECT_STACK_DIAGNOSTICS.md](docs/EFFECT_STACK_DIAGNOSTICS.md).
The public implementation plan for an OpenGL ES + OpenXR multilayer video
stack, including SurfaceTexture/OES ingestion, public edge/mask examples,
projection diagnostics, and Vulkan/Makepad comparison gates, is tracked in
[docs/OPENGL_OPENXR_MULTILAYER_STACK_PLAN.md](docs/OPENGL_OPENXR_MULTILAYER_STACK_PLAN.md).
The broker clock/timebase API used by that home surface is documented in
[docs/BROKER_CLOCK_AND_TIMEBASE.md](docs/BROKER_CLOCK_AND_TIMEBASE.md).
The public distribution boundary between Store-style Quest apps, SideQuest or
GitHub developer builds, external ADB hosts, Wi-Fi ADB, and shell helpers is
documented in
[docs/QUEST_DISTRIBUTION_AND_ADB_BOUNDARY.md](docs/QUEST_DISTRIBUTION_AND_ADB_BOUNDARY.md).
The public boundary for headset/controller tracking, Android sensors, and ADB
diagnostics is documented in
[docs/QUEST_TRACKING_ACCESS_BOUNDARY.md](docs/QUEST_TRACKING_ACCESS_BOUNDARY.md).
The optional Meta Quest `hzdb` provider, MCP bridge, docs-first verification,
Perfetto analysis, and structured safety-gated device-operation plan is
documented in
[docs/META_QUEST_HZDB_PROVIDER_PLAN.md](docs/META_QUEST_HZDB_PROVIDER_PLAN.md).
For Rusty Kiosk, that provider loop is part of the default tracking setup:
record command goal, provider, fallback, foreground before/after, broker
clock/status, and whether a Meta menu/settings surface was intentionally opened.
The Quest broker reports that baseline through `rustyKiosk` in `/status`, direct
`/kiosk/status`, WebSocket command `kiosk.get_status`, and stream
`kiosk:control_plane`.

The contracts examples and minimal APK can be run without headset hardware or
downstream app code:

```powershell
cargo run -p rusty-xr-contracts --example plain_stereo_feedback_layout --features serde
cargo run -p rusty-xr-contracts --example composite_feedback_session --features serde
cargo run -p rusty-xr-contracts --example meta_passthrough_style_catalog --features serde
cargo run -p rusty-xr-contracts --example audio_reactive_passthrough_style --features serde
cargo run -p rusty-xr-contracts --example visual_strobe_profiles --features serde
cargo run -p rusty-xr-contracts --example developer_home_manifest --features serde
cargo run -p rusty-xr-contracts --example kiosk_command_run_record --features serde
cargo run -p rusty-xr-contracts --example effect_stack_diagnostic_manifest --features serde
cargo run -p rusty-xr-particles --example dynamic_mesh_coordinates
cargo run -p rusty-xr-particles --example mesh_fixture_manifest --features serde
cargo run -p rusty-xr-particles --example hand_mesh_dynamic_collider
cargo run -p rusty-xr-particles --example hand_mesh_sdf_attraction
cargo run -p rusty-xr-particles --example billboard_performance_patterns
cargo run -p rusty-xr-quest-diagnostics --example quest_provider_snapshot
powershell -ExecutionPolicy Bypass -File .\examples\quest-minimal-apk\tools\Build-QuestMinimalApk.ps1
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-apk\tools\Build-QuestBrokerApk.ps1
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-shell-helper\tools\Build-BrokerShellHelper.ps1
cargo run -p rusty-xr-broker-client-probe -- status
python tools\replay\check_replay_fixtures.py --fixtures fixtures\replay
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
Quest OpenXR shell bring-up notes, including the distinction between stable
scene-space anchoring and Vulkan screen-space viewport conventions, live in
[docs/ANDROID_QUEST_APK_BUILDING.md](docs/ANDROID_QUEST_APK_BUILDING.md).
Direct-device Quest render artifact diagnosis, including `VrApi`, OVR Metrics,
Perfetto, screenshot, and one-variable A/B workflows, is documented in
[docs/QUEST_RENDER_ARTIFACT_DIAGNOSTICS.md](docs/QUEST_RENDER_ARTIFACT_DIAGNOSTICS.md).
`hzdb` is treated as an optional Meta Quest provider for those workflows, not a
core dependency; the provider boundary and MCP safety model live in
[docs/META_QUEST_HZDB_PROVIDER_PLAN.md](docs/META_QUEST_HZDB_PROVIDER_PLAN.md).
Quest streaming and camera-composite cost isolation, including direct in-app
Camera2 versus broker H.264 projected paths, render-scale interpretation, and
the reusable streaming scorecard tooling, is documented in
[docs/QUEST_STREAMING_DIAGNOSTICS_WORKFLOW.md](docs/QUEST_STREAMING_DIAGNOSTICS_WORKFLOW.md).
The current custom stereo projection parity target and depth-alignment impact
plan is documented in
[docs/CAMERA_STEREO_PROJECTION_PARITY_WORKPLAN.md](docs/CAMERA_STEREO_PROJECTION_PARITY_WORKPLAN.md).
An agent-facing onboarding note for a Mac-based collaborator with multiple
Quest headsets is available at
[docs/QUEST_Q2Q_AGENT_ONBOARDING.md](docs/QUEST_Q2Q_AGENT_ONBOARDING.md).
A public plan for migrating the current Quest APK examples toward a
Makepad-compatible Android build and hotload workflow is tracked in
[docs/MAKEPAD_ANDROID_BUILD_COMPATIBILITY_PLAN.md](docs/MAKEPAD_ANDROID_BUILD_COMPATIBILITY_PLAN.md).
The Makepad-first fork-lane comparison, including affordances, costs,
dependencies, and remaining decision points, is tracked in
[docs/MAKEPAD_Q2Q_PARALLEL_APPROACH_COMPARISON.md](docs/MAKEPAD_Q2Q_PARALLEL_APPROACH_COMPARISON.md).
The public ownership boundary between Rusty XR core and the maintained Makepad
fork branch is documented in
[docs/MAKEPAD_FORK_RELATIONSHIP.md](docs/MAKEPAD_FORK_RELATIONSHIP.md).
The active Makepad stereo-comparison implementation ledger is tracked in
[docs/MAKEPAD_STEREO_COMPARISON_ITERATION.md](docs/MAKEPAD_STEREO_COMPARISON_ITERATION.md).
The dedicated Quest stereo alignment workflow, including screenshot analysis
and optional MediaProjection witness handling, is documented in
[docs/QUEST_STEREO_ALIGNMENT_WORKFLOW.md](docs/QUEST_STEREO_ALIGNMENT_WORKFLOW.md).
The raw camera stack alignment workflow names the comparable public direct and
broker lanes across Vulkan/HWB, OpenGL/OES, and Makepad CPU-YUV paths and
documents the full-suite runner:
[docs/QUEST_RAW_CAMERA_STACK_ALIGNMENT_WORKFLOW.md](docs/QUEST_RAW_CAMERA_STACK_ALIGNMENT_WORKFLOW.md).
The current ordered projection-area and public diagnostic blur workflow,
including broker-synthetic `diagnostic-grid` / `motion-bar` inputs and the
later Brave physical-screen stimulus pass, is documented in
[docs/SCREEN_SPACE_AND_BLUR_ALIGNMENT_WORKFLOW.md](docs/SCREEN_SPACE_AND_BLUR_ALIGNMENT_WORKFLOW.md).
The coordinate-space ledger that must pass before more blur work is documented
in
[docs/PROJECTION_COORDINATE_SPACE_LEDGER.md](docs/PROJECTION_COORDINATE_SPACE_LEDGER.md).
Browser-based physical-screen stimulus tooling for camera and final-display
capture alignment runs lives in
[tools/quest-visual-stimulus/README.md](tools/quest-visual-stimulus/README.md).
Active camera-facing-screen runs use that tool's fullscreen foreground
convention: a red dot at top right means the screen is reserved, and `SAFE` or
`STOP` means the browser can be moved or covered again.
The first implementation slices add source-only `build-manifest.public.json`
files beside the public Android examples, the standalone Makepad smoke shell,
and `tools/schema/check_android_build_manifest.py` to validate them.
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
Streaming diagnostics tooling lives in
[tools/quest-streaming-diagnostics/README.md](tools/quest-streaming-diagnostics/README.md)
and produces ignored local scorecards from headset run artifacts.

BLE, LSL, and Polar H10 data-pipeline guidance is documented in
[docs/BLE_LSL_POLAR_PIPELINE.md](docs/BLE_LSL_POLAR_PIPELINE.md).
OSC live-control and sensor-ingress guidance is documented in
[docs/OSC_ADAPTER.md](docs/OSC_ADAPTER.md).
Research-XR broker bridge boundaries and the EDIA/RCAS comparison are
documented in
[docs/RESEARCH_XR_BROKER_BRIDGE.md](docs/RESEARCH_XR_BROKER_BRIDGE.md).
The source-level Unity broker adapter contract is documented in
[docs/UNITY_BROKER_ADAPTER_CONTRACT.md](docs/UNITY_BROKER_ADAPTER_CONTRACT.md).
The public maintainer-facing collaboration track for EDIA-style integration is
documented in
[docs/EDIA_COLLABORATION_TRACK.md](docs/EDIA_COLLABORATION_TRACK.md).

General-purpose XR tool import candidates are tracked in
[docs/GENERAL_TOOL_IMPORT_AUDIT.md](docs/GENERAL_TOOL_IMPORT_AUDIT.md).
The broader sanitized machine-wide repository/tooling audit is tracked in
[docs/MACHINE_REPO_TOOLING_AUDIT.md](docs/MACHINE_REPO_TOOLING_AUDIT.md).
Quest tracking access boundaries for public utilities are tracked in
[docs/QUEST_TRACKING_ACCESS_BOUNDARY.md](docs/QUEST_TRACKING_ACCESS_BOUNDARY.md).
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
