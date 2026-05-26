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
It also owns public developer-home menu contracts for app-owned panels,
launcher entries, settings shortcuts, helper state, bounded supervisor policy,
and focus-recovery event logging.

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

### OSC Utilities

Reusable Open Sound Control packet models, common OSC 1.0 encoding/decoding,
and a small UDP helper for live control and sensor ingress.

OSC address trees and value semantics remain app-owned. The public crate should
stay transport-oriented until a stable sensor contract is ready.

### Broker Model

Reusable broker command, acknowledgement, client hello, stream manifest, sample
header, session manifest, timing, drop counter, replay record, synthetic stream,
transport-lane, and broker-clock contracts. These models keep reliable control,
loss-tolerant streams, replay, WebSocket/UDP fallbacks, and timestamp-domain
correlations explicit without implementing sockets or engine adapters in core.
The research-XR bridge split is documented in
[RESEARCH_XR_BROKER_BRIDGE.md](RESEARCH_XR_BROKER_BRIDGE.md).
The broker-owned timebase and Quest broker Clock page are documented in
[BROKER_CLOCK_AND_TIMEBASE.md](BROKER_CLOCK_AND_TIMEBASE.md).

The clean-room low-latency transport direction is documented in
[LOW_LATENCY_TRANSPORT_ARCHITECTURE.md](LOW_LATENCY_TRANSPORT_ARCHITECTURE.md).
The Quest-to-Quest online streaming roadmap is documented in
[QUEST_TO_QUEST_ONLINE_STREAMING_ROADMAP.md](QUEST_TO_QUEST_ONLINE_STREAMING_ROADMAP.md).
Rusty XR may model session negotiation, stream descriptors, timing metrics,
network-quality samples, security policy, and optional external-sidecar
comparison lanes, but public core must not copy vendor SDK source, packet
layouts, SDK headers, binaries, or proprietary wire details.
The next broker-model slice should strengthen the existing H.264 diagnostic
path before adding new transports: camera/source capability manifests,
timestamp-domain fields, H.264 stream invariants, bounded relay metrics,
runtime keyframe/bitrate commands, session-native projection metadata, and
privacy/security policy for LAN and online relays. Clock follow-up work should
add OpenXR frame-timeline samples, companion sync-probe reporting, and session
manifest clock summaries.

### ZeroMQ Adapter

The optional `rusty-xr-zmq` crate turns broker ZeroMQ bridge manifests into
runtime-ready receiver configuration, bounded app-drain queues, and an opt-in
pure Rust PUB/SUB receiver. The crate is not part of Rusty XR core and default
builds do not open sockets or pull runtime ZeroMQ dependencies.

The first public slices should stay focused on receiver-side diagnostics:
manifest validation, explicit bind/connect selection, topic-prefix handling,
drop counters, decode-error counters, receive timestamps, desktop checks, and
Android target compilation. A local PUB/SUB loopback example now exercises the
runtime path without hardware. Later slices can add broker example wiring,
sender helpers, richer socket-pattern coverage, and device runtime validation.

### Eye Model

Engine-neutral eye-data contracts for screen-space gaze points, XR gaze rays,
screen-space AOI hits, derived processor events, validity flags, and provenance.
The screen-space and XR-ray shapes stay separate so desktop tracker samples can
validate timing, recording, replay, and AOI processors without pretending to
validate headset-local gaze vectors or scene-hit semantics. Native tracker SDKs,
provider licenses, calibration UX, and LSL/OSC/WebSocket forwarding stay in
optional adapters or downstream shells.

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
device readiness. The crate now also owns provider-neutral contracts for
Meta Quest development tooling: `hzdb`/ADB/MCP provider capabilities, safety
classes, device health, controller info, app metadata, foreground state, log
filters, screenshot capture manifests, file-operation plans, Perfetto trace
sessions and metrics, docs/API search results, optional asset-search results,
MCP server config, agent-skill metadata, and combined provider snapshots.
The provider architecture and implementation plan are documented in
[META_QUEST_HZDB_PROVIDER_PLAN.md](META_QUEST_HZDB_PROVIDER_PLAN.md).

### Debug Canvas

Reusable normalized layout primitives for headset-visible test panels and
diagnostics displays. The canvas crate owns sections, badges, key/value rows,
wrapping, tones, draw-list generation, and a small diagnostic HUD visibility /
page command state while renderers and input adapters stay app-owned. Headset
renderers should prefer a shared stereo surface for diagnostic HUDs; see
`docs/DIAGNOSTIC_HUD_STEREO_RENDERING.md`.

### Camera Model

Reusable camera contracts and math helpers, including camera metadata,
intrinsics, extrinsics, stereo frame descriptors, and projection-related
utilities.

The active coordinate-space ledger for camera projection, environment-depth
particles, synthetic source geometry, and blur gating is documented in
[PROJECTION_COORDINATE_SPACE_LEDGER.md](PROJECTION_COORDINATE_SPACE_LEDGER.md).
Future camera/projection work should name the domain it reads or writes before
adding renderer tuning.

The custom stereo temporal reprojection plan is tracked in
[CUSTOM_STEREO_CAMERA_TEMPORAL_REPROJECTION.md](CUSTOM_STEREO_CAMERA_TEMPORAL_REPROJECTION.md).
The first public slices now include data-only temporal policy/state/metric
contracts, metrics-only runtime logging, opt-in pose-delta and screen-motion
clamp profiles, frame-adoption controls, stream-header projection metadata,
and edge/invalid-UV scorecard reporting in the composite-layer example.
Direct Camera2 projection, broker H.264 projection, and the single-headset
laptop-relay receiver path have exercised the projected scorecard surface.
Remaining measured work is motion/stress tuning for frame holds, depth-aware
reprojection, optional space-warp probes, and any shader policy that proves
useful without becoming a default.

### Plain Stereo And Feedback Layers

Reusable descriptors for app-owned projected mono/stereo media layers,
side-by-side or separate-eye source layouts, aspect-fit content rectangles, and
simple visual feedback/content borders. Public tuning may include border-only
coverage/radius/edge/feedback scalar values and adapter performance hints for
custom stereo and optional screen-feedback layers.

These contracts do not implement the Passthrough Camera API, MediaProjection,
OpenXR composition submission, Vulkan texture import, or downstream
image-processing, geometric-effect, scene, or product tuning stacks.

### Effect Stack Diagnostics

Reusable data contracts for renderer-owned multi-pass visual pipelines:
ordered pass descriptors, logical intermediate buffers, diagnostic layer taps,
and scalar layer-comparison metrics. These contracts help downstream shells
compare source, guide, edge, mask, displacement, and final-composite layers
without moving shader code, private visual behavior, generated captures, or
native texture ownership into public core. See
[EFFECT_STACK_DIAGNOSTICS.md](EFFECT_STACK_DIAGNOSTICS.md).

### OpenGL/OpenXR Multilayer Stack

Rusty XR should own the reusable OpenGL ES + OpenXR implementation lane for
video-backed multilayer stacks. The public plan keeps the current Vulkan
hardware-buffer path as the performance baseline while adding a separate
SurfaceTexture / external-OES lane for public examples such as luma, edge
detection, masks, simple color maps, and final composites. Downstream apps can
consume this lane for private visual behavior without publishing their recipes
or tuning constants here.

The implementation sequence is documented in
[OPENGL_OPENXR_MULTILAYER_STACK_PLAN.md](OPENGL_OPENXR_MULTILAYER_STACK_PLAN.md):
OpenXR/GLES feasibility, broker synthetic H.264 to OES textures, OES ingest
copy to internal FBOs, projection-policy diagnostic layers, public multilayer
examples, deterministic cross-lane parity, performance/pass-budget comparison,
temporal projection integration, and only then live Camera2.

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

The Quest composite-layer example now proves live environment-depth diagnostics,
generated mesh visualization, retained local-space particle visualization,
scene-owned local-space particle mapping, and native passthrough composition in
one public APK source tree. The retained particle path remains documented as a
diagnostic bridge: it writes accepted samples into local scene coordinates, but
the visible particle set is still refreshed from a view-sampled grid. The scene
particle map is the current real-time environment-mapping path: accepted depth
samples are spatially binned in OpenXR local space, confidence-merged by cell,
actively corrected from high-confidence visible free-space observations, and
drawn as small alpha-clipped opaque default-disc particles.
This world-space-first path is also the reference baseline for the coordinate
ledger: runtime depth samples become app reference-space points first, then
render through the current per-eye OpenXR views.

### SDF Model

Reusable signed-distance-field contracts, mesh snapshots, packed grids, sparse
views, dynamic mesh-to-SDF reference conversion, and small reference utilities.
The CPU mesh-to-SDF builder is the public deterministic baseline for examples
and adapter validation; high-throughput workers or GPU kernels remain
adapter-owned until their contracts settle.

### Particle And Animation Primitives

Reusable particle state, simulation, interaction, animation, and render-payload
contracts that can run independently of a specific app shell. The particle
crate also owns source-only mesh coordinate sampling and SDF surface-attraction
helpers so public examples can validate hand-mesh and field workflows without
publishing app-specific simulation behavior.

The public particle guidance also records renderer-facing lessons that remain
general across downstream apps: keep particle centers in the intended OpenXR
reference space, choose center-projected billboards versus world-expanded
billboard vertices deliberately, prefer normal texture-array sampling for
animated billboard masks until profiling proves otherwise, treat trail
particles as frozen snapshots unless a consumer explicitly animates them, and
isolate Quest artifact runs one renderer stage at a time. See
[PARTICLE_BILLBOARD_AND_ANIMATION_PERFORMANCE.md](PARTICLE_BILLBOARD_AND_ANIMATION_PERFORMANCE.md).

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

The Makepad-compatible Android build migration is tracked in
[MAKEPAD_ANDROID_BUILD_COMPATIBILITY_PLAN.md](MAKEPAD_ANDROID_BUILD_COMPATIBILITY_PLAN.md).
That plan keeps current Quest examples usable while moving toward
`cargo-makepad` packaging, Makepad Live/runtime config hotload, and future
dynamic-library hotpatch compatibility.

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
The first Rusty Kiosk / developer-home slice follows this rule:
`rusty-xr-contracts` models home panels, launch rows, settings shortcuts,
helper status, focus-recovery events, and the Rusty Kiosk control-plane status
that distinguishes `BrokerPanel2d`, `BrokerPanelWithShellHelper`, and future
immersive-home phases. Actual 2D UI, immersive rendering, ADB helper lifecycle,
and managed kiosk policy stay in app shells or companion tooling. The Quest
broker APK exposes the current control-plane status through `/status`,
`/kiosk/status`, `kiosk.get_status`, and stream `kiosk:control_plane`.

A broader sanitized machine-wide audit is tracked in
[MACHINE_REPO_TOOLING_AUDIT.md](MACHINE_REPO_TOOLING_AUDIT.md). It covers
additional Quest ADB/`hzdb`, tracking-access boundaries, Windows streaming,
room mesh/depth, PCA/capture, runtime profile, GPU layout,
BLE/LSL/biofeedback, and clean-room import tooling candidates from local
sibling repositories. The foreground-OpenXR-versus-ADB tracking boundary is
documented in
[QUEST_TRACKING_ACCESS_BOUNDARY.md](QUEST_TRACKING_ACCESS_BOUNDARY.md).
The 2026 Meta Horizon agentic tooling update is folded into
[META_QUEST_HZDB_PROVIDER_PLAN.md](META_QUEST_HZDB_PROVIDER_PLAN.md): `hzdb`
is a Meta Quest provider and optional MCP bridge, not a replacement for Rusty
XR contracts or the Companion safety layer.

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
Quest streaming cost isolation is documented in
[QUEST_STREAMING_DIAGNOSTICS_WORKFLOW.md](QUEST_STREAMING_DIAGNOSTICS_WORKFLOW.md),
with public scorecard tooling under `tools/quest-streaming-diagnostics/`.
The custom stereo Camera2 projection parity workplan, including the public
depth-alignment impact gate, is tracked in
[CAMERA_STEREO_PROJECTION_PARITY_WORKPLAN.md](CAMERA_STEREO_PROJECTION_PARITY_WORKPLAN.md).

Current Quest media implementation status:

- Public Rusty XR includes camera/depth metadata, permission guidance, a generic
  Windows frame receiver, plain stereo layer descriptors, border tuning,
  composite-feedback tuning, performance hints, visual feedback border layout,
  and native platform passthrough style descriptors.
- Public Rusty XR now includes a clean Quest example APK that demonstrates a
  Camera2/headset-camera custom layer with OpenXR/Vulkan submission, Android
  hardware-buffer import, GPU sampling, paired left/right Camera2 streams,
  public Camera2 intrinsics/pose metadata, display-eye screen-to-camera
  homographies, a quad-surface comparison profile, and the public
  projection-border policy. The fullscreen mapping and quad-surface mapping both
  exercise the core stereo camera stack, but the quad-surface profile remains
  an optimization/color-parity task rather than the final performance
  reference.
- MediaProjection remains a final-screen inspection stream only; it is not the
  raw camera source for the custom layer.
- Public Rusty XR now includes a separate broker APK proof-of-concept for
  localhost WebSocket latency samples, status/capability reporting, optional
  broker-to-laptop LSL forwarding, OSC latency egress, and OSC drive ingress.
  It has been validated with
  [The Big Red Button Institute](https://github.com/MesmerPrism/the-big-red-button-institute),
  the public Unity Quest comparison example. The broker also exposes an initial
  camera-projection metadata provider, a bounded app-context Camera2
  open/one-frame capture probe, bounded app-context raw-luma and H.264 binary
  side-channel probes, a broker-local Android MediaCodec H.264 decode probe,
  and shell-helper status surface. The composite-layer example can now run
  broker H.264 consumer probes that request bounded or live-bounded broker
  streams, consume the device-local binary payload, and decode it in the
  composite app process, including a surface-backed `SurfaceTexture`
  external-texture telemetry mode and a hardware-buffer mode that feeds the
  existing Vulkan/OpenXR GPU-buffer renderer. The stereo live-bounded
  hardware-buffer path carries selected Camera2 intrinsics and platform pose
  metadata from stream start, accepts left/right binary stream sockets before
  capture, decodes paired streams as packets arrive, forwards accepted
  `AHardwareBuffer` pairs through the native stereo bridge, and can submit them
  through the `gpu-projected` OpenXR stereo path. The active XR client still
  owns unbounded production session lifetime, timestamp-based pair/drop policy,
  remote-device transport, eye views/FOV, shaders, and release-grade OpenXR
  performance.
- Public Rusty XR now includes a source-only broker shell-helper example that
  builds a dex jar for `adb shell app_process`. It reports UID/version/basic
  capabilities, optional bounded codec diagnostics, bounded shell-visible
  camera metadata, Camera2 open/one-frame capture feasibility, and optional
  metadata-only synthetic encoded-stream events to the broker over the existing
  WebSocket command API. The broker maps shell camera metadata/open/capture
  evidence into `cameraProvider` and `projectionProfile` status while keeping
  the raw helper diagnostics separate. It can also emit bounded synthetic
  encoded packets over an
  ADB-forwarded TCP side channel with a small public framing contract, plus a
  guarded MediaCodec synthetic-Surface encoder probe that emits real H.264
  packets over the same side channel and records helper encode/write metrics.
  It can also run a guarded shell `screenrecord` display-source probe that
  chunks stdout H.264 bytes into the same framing. It is Developer Mode tooling
  and does not make the installed broker APK run as Android `shell`.
- The broker also exposes metadata-only `video_lab.register_encoded_stream_manifest`,
  `video_lab.record_encoded_sample_metadata`, and
  `video_lab.record_metric_sample` commands with matching video-lab streams for
  encoded-stream contract, sample metadata, and timing/drop/queue diagnostics.
  High-rate frame payloads belong on a binary transport owned by the
  provider/client adapter; the current helper side channel is a synthetic
  MediaCodec proof, shell screenrecord display-source proof, and bounded camera
  feasibility proof for that split. The broker app-context luma probe proves
  bounded raw camera payload delivery with `raw_luma8` packets over
  ADB-forwarded TCP, the broker app-context H.264 probe proves Camera2 frames
  can feed Android's platform encoder, and the composite live stereo H.264
  probe proves the diagnostic decode path can reach Vulkan/OpenXR texture
  submission. These are still diagnostic bridges rather than release streaming
  camera providers.
- A direct-versus-broker streaming matrix now shows that synthetic compositor
  and broker receive/decode lanes can remain stable while both direct Camera2
  projected stereo and broker live projected stereo miss cadence at
  `rustyxr.xrRenderScale=0.75` and recover at `0.65`. Public stage timing also
  keeps Java image acquisition, decoded-image waits, `HardwareBuffer`
  extraction, and native bridge calls below roughly sub-millisecond scale, so
  the next public performance target is projected draw/render attribution
  rather than transport or handoff. The dedicated parity target map is
  documented in
  [CAMERA_STEREO_PROJECTION_PARITY_WORKPLAN.md](CAMERA_STEREO_PROJECTION_PARITY_WORKPLAN.md).
- Public contracts now include temporal projection policy/state/metric shapes.
  The composite-layer example emits no-smoothing metrics, supports opt-in
  pose-delta and screen-motion clamp profiles, carries projection metadata in
  schema-3 stream headers, and reports frame-adoption plus edge/invalid-UV
  fields. Direct, broker-live, and single-headset laptop-loop projected paths
  are validated. Remaining temporal work is motion/stress tuning for nonzero
  frame holds, then depth-aware reprojection and optional space-warp probes as
  separate profiles.
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
- [x] Add particle and animation primitives.
- [x] Add tests for deterministic simulation, dynamic mesh coordinate
  sampling, neighborhoods, and payload generation.

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
| Core contracts | In progress | Initial framework-neutral pose, timing, view, camera, depth, hand, home/menu, render payload, layer, effect-stack diagnostics, and counter contracts added. |
| Runtime config | In progress | Generic key/value parsing and Android property naming helpers added. |
| BLE utilities | In progress | Framework-neutral BLE UUID, GATT path, notification, operation, scan result, and Android permission models added. |
| LSL utilities | In progress | Pure stream descriptor, stream role, channel schema, discovery filter, endpoint status, staleness, roundtrip, biofeedback, and telemetry models added. |
| OSC utilities | In progress | Pure OSC message/bundle codec, UDP helper, loopback probe, and Quest example listener profile added. |
| Broker model | In progress | Public command envelopes, acknowledgements, client hello, stream manifests, sample headers, session manifests, transport endpoints, timing stamps, heartbeat state, drop counters, replay records, synthetic wave streams, transport-session contracts, and Rusty XR diagnostic video headers added for sidecar broker streams. |
| ZeroMQ adapter | In progress | Optional pure-Rust adapter crate added for ZeroMQ bridge manifests, bounded receiver queues, runtime-gated PUB/SUB receiver support, and local loopback example without native `libzmq`. |
| Eye model | In progress | Public screen-space gaze, XR gaze-ray, AOI hit, processor-event, validity, provenance, and deterministic synthetic eye-data contracts added without native tracker SDK dependencies. |
| Debug canvas | In progress | Dependency-light logical canvas crate added for reusable diagnostics panels, with normalized rectangle/text draw lists, input-neutral diagnostic HUD command state, and optional serde. |
| Polar H10 utilities | In progress | Public Polar GATT IDs, HR/RR decoder, uncompressed ECG/ACC PMD decoders, PMD command builders, and LSL schemas added. |
| Quest diagnostics | In progress | Generic readiness, package launch, and frame-rate status models added. |
| Camera model | In progress | Intrinsics scaling, projection, back-projection, and timestamp matching helpers added. |
| Camera temporal projection | In progress | Temporal policy contracts, target/visual projection state, stereo pair timing, pose-delta lockstep clamp, screen-motion clamp, frame-adoption, edge-mode, and scorecard metric fields added. Composite-layer example validates direct and laptop-relay projected paths; physical motion stress tuning remains open. |
| Plain stereo / feedback layers | In progress | Public mono/stereo media layer descriptors, source UV layout helpers, aspect-fit content rectangles, visual feedback border segments, border tuning, composite-feedback tuning, and performance hints added. |
| Effect stack diagnostics | In progress | Public data-only pass graph descriptors, intermediate buffer descriptors, diagnostic layer taps, and scalar layer comparison metrics added for downstream visual pipelines. |
| OpenGL/OpenXR multilayer stack | Planned | Public implementation lane documented for OpenXR/GLES presentation, SurfaceTexture/OES ingestion, internal FBO pass graphs, projection-policy diagnostics, and public edge/mask/composite examples. |
| Native platform passthrough descriptors | In progress | Public Meta/OpenXR layer-purpose, placement, opacity, edge, color-map, BCS, and LUT descriptors added with contracts-only examples. |
| Visual strobe descriptors | In progress | Public full-field and passthrough-LUT strobe profile descriptors, display-frame frequency plans, 120 Hz constraints, and safety warnings added with a no-hardware example. |
| Depth model | In progress | Depth readiness, frame summary, per-view metadata, infinite-far range, cadence, and readback-policy helpers added. |
| SDF model | In progress | Packed SDF grid, sampling, bounds, triangle mesh snapshots, and data-only depth support/impact query contracts added. |
| Particle and animation primitives | In progress | Minimal particle state, fixed-step clock, render payload generation, dynamic mesh coordinate sampling, live hand-mesh sampler updates, same-surface neighbor tiers, cross-surface neighbor links, dynamic mesh collider surfaces with diagnostic shells, SDF attraction helpers, particle visualization helpers, billboard instance packing, trail snapshots, render-budget estimates, generated animation mask fixtures, and billboard/animation performance guidance added. |
| XR canvas and hand interaction | In progress | Public ray/canvas hit-test contracts, hand-menu anchors, activation modes, hand influence points, and normalized debug/test canvas primitives added. |
| Sparse scan / TSDF contracts | In progress | Public sparse TSDF samples, snapshots, scan surface samples, and scan-fusion stats added. |
| Room mesh and capture lifecycle | In progress | Public room mesh source state, semantic room mesh snapshots, and capture lifecycle/source metadata added. |
| Companion app catalog alignment | In progress | `quest-app-catalog` schema version aligned with Rusty XR Companion Apps catalog metadata. |
| General tool import audit | In progress | Makepad/Rust-XR-family candidates plus broader machine-wide Quest/tooling candidates documented with public/downstream boundaries. |
| GitHub Pages docs | In progress | Static public docs layer added under `docs/` with Mermaid diagrams and sanitized architecture guidance. |
| Serialization and schemas | In progress | Opt-in `serde` features, round-trip tests, and custom schema export script added. |
| Boundary scanner and provenance | In progress | Public scanner CLI/config and public utility provenance metadata added. |
| Feature and adapter policy | `[x]` | Adapter feature names, separate-crate rule, and pre-adapter boundary requirements documented. |
| Public examples | In progress | Synthetic layout, composite feedback, passthrough style catalog, audio-reactive passthrough style, and visual strobe profile examples added; minimal Rust-native Android APK smoke test added; first camera-driven Quest OpenXR/Vulkan custom-layer example added with optional MediaProjection screen streaming, an OSC listener diagnostics profile, environment-depth diagnostics, passthrough-backed depth mesh visualization, retained local-space particle visualization, scene-owned environment-depth particle mapping, and Meta/OpenXR hand-mesh particle visualization; broker APK proof added for localhost WebSocket, optional LSL forwarding, and OSC ingress/egress validation; portable hand-mesh sampler, collider, and SDF fixture examples added. |
| Optional adapters | In progress | First adapter slice is `rusty-xr-zmq`; adapters remain separate crates with runtime features disabled by default. |
