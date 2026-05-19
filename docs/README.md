# Rusty XR Docs

This is the public navigation index for Rusty XR. Use it as the first stop
before reading chronological plans or investigation logs.

## Current Architecture

- [Module and crate map](MODULE_CRATE_MAP.md): current public foundation by
  crate, category, and dependency direction.
- [API, CLI, and MCP entrypoints](API_CLI_MCP_ENTRYPOINTS.md): public routes
  agents can use for Rust APIs, schemas, broker APIs, CLIs, and optional MCP.
- [Feature and adapter policy](FEATURE_AND_ADAPTER_POLICY.md): what belongs in
  core crates, optional features, adapters, and examples.
- [Serialization and schema policy](SERIALIZATION_AND_SCHEMA_POLICY.md): serde,
  schema export, and compatibility rules.
- [Provenance](PROVENANCE.md): public-safe provenance notes for extracted
  utility work.
- [API surface review](API_SURFACE_REVIEW.md): active review notes for public
  contracts.

## Public Examples

- [Examples matrix](EXAMPLES_MATRIX.md): examples, modules used, hardware
  needs, validation commands, and sanitized downstream pressure.
- [Contracts examples](../crates/rusty-xr-contracts/examples/README.md):
  source-only contract demos.
- [Minimal Quest APK](../examples/quest-minimal-apk/README.md): smallest
  Android smoke test.
- [Composite layer Quest APK](../examples/quest-composite-layer-apk/README.md):
  larger OpenXR/Vulkan diagnostic example.
- [Broker sidecar APK](../examples/quest-broker-apk/README.md): broker
  status, stream, clock, launcher, and diagnostic sidecar example.
- [Broker shell helper](../examples/quest-broker-shell-helper/README.md):
  source-only Developer Mode shell-helper example.
- [Broker client probe](../examples/broker-client-probe/README.md):
  source-only Rust client probe for broker status and commands.
- [Makepad comparison shell](../examples/makepad-q2q-camera-shell/README.md):
  standalone Makepad-first comparison lane.

## Extraction Gate

- [Public extraction workflow](PUBLIC_EXTRACTION_WORKFLOW.md): how reusable
  pieces move from downstream pressure into this public repo.
- [API, CLI, and MCP entrypoints](API_CLI_MCP_ENTRYPOINTS.md): consistency
  rules for command surfaces and provider routes.
- [Effect-stack diagnostics](EFFECT_STACK_DIAGNOSTICS.md): data-only visual
  pass descriptors and comparison reports.
- [Mesh fixture manifest](MESH_FIXTURE_MANIFEST.md): public synthetic mesh
  fixture manifests for topology, sampling, SDF/depth, particle, collider, and
  render-payload tests.
- [Dynamic mesh coordinate sampling](DYNAMIC_MESH_COORDINATE_SAMPLING.md):
  topology-stable mesh coordinates and neighborhoods.
- [Dynamic mesh colliders](DYNAMIC_MESH_COLLIDERS.md): collider-ready dynamic
  mesh surfaces and diagnostic shells.
- [Dynamic mesh to SDF](DYNAMIC_MESH_TO_SDF.md): mesh snapshot to packed SDF
  conversion and particle attraction examples.
- [Quest developer-home menu](QUEST_DEVELOPER_HOME_MENU.md): data contracts for
  developer-home panels, launcher entries, settings shortcuts, helper state,
  focus recovery, and command evidence.
- [Broker clock and timebase](BROKER_CLOCK_AND_TIMEBASE.md): broker-owned
  elapsed-realtime clock, stamps, sync probes, and health snapshots.

## Validation

- [Validation commands](VALIDATION.md): command sets by change type.
- [Android and Quest APK building](ANDROID_QUEST_APK_BUILDING.md): Android
  toolchain and Quest source-build notes.
- [Companion integration](RUSTY_XR_COMPANION_INTEGRATION.md): shared catalog
  schema and companion-tool boundary.
- [Media pipeline and permissions](MEDIA_PIPELINE_AND_PERMISSIONS.md): Android
  media permissions, platform codecs, and optional external sidecars.

## Current Runbooks

- [Quest visual source taxonomy](QUEST_VISUAL_SOURCE_TAXONOMY.md): raw camera,
  passthrough, environment depth, MediaProjection, and casting source
  boundaries.
- [Quest app launching and shell helpers](QUEST_APP_LAUNCHING_AND_SHELL_HELPERS.md):
  normal app launches, shell helpers, and package visibility.
- [Quest distribution and ADB boundary](QUEST_DISTRIBUTION_AND_ADB_BOUNDARY.md):
  Store-style apps, developer builds, ADB hosts, Wi-Fi ADB, and shell helpers.
- [Quest tracking access boundary](QUEST_TRACKING_ACCESS_BOUNDARY.md):
  foreground OpenXR pose sampling, Android sensor limits, and ADB diagnostics.
- [Quest streaming diagnostics workflow](QUEST_STREAMING_DIAGNOSTICS_WORKFLOW.md):
  diagnostic streaming scorecards and projection paths.
- [Quest stereo alignment workflow](QUEST_STEREO_ALIGNMENT_WORKFLOW.md):
  headset screenshot and alignment pass.
- [Quest camera profile workflow](QUEST_CAMERA_PROFILE_WORKFLOW.md): runtime
  camera profiles and reusable run tooling.
- [Screen-space and blur alignment workflow](SCREEN_SPACE_AND_BLUR_ALIGNMENT_WORKFLOW.md):
  public diagnostic blur and projection-area workflow.
- [Projection coordinate space ledger](PROJECTION_COORDINATE_SPACE_LEDGER.md):
  coordinate domains, source-of-truth rules, and three-lane contract before
  blur.
- [Environment depth particle anchoring](ENVIRONMENT_DEPTH_PARTICLE_ANCHORING.md):
  world-space depth mesh, retained particle, and scene particle-map contract
  before camera/depth comparison.
- [World-space quad and direct shader reconciliation](WORLD_SPACE_QUAD_DIRECT_SHADER_RECONCILIATION.md):
  equivalence rules for the reference-space quad path and collapsed per-eye
  shader path.
- [Synthetic projection coordinate alignment plan](SYNTHETIC_PROJECTION_COORDINATE_ALIGNMENT_PLAN.md):
  active synthetic-first gate before blur, live camera, or passthrough
  alignment.

## Plans, Investigations, And History

These files are useful evidence, but they are not the shortest route to current
architecture.

- [Implementation plan](IMPLEMENTATION_PLAN.md): milestone plan and active
  roadmap.
- [Camera stereo projection parity workplan](CAMERA_STEREO_PROJECTION_PARITY_WORKPLAN.md):
  public projection parity target.
- [OpenGL/OpenXR multilayer stack plan](OPENGL_OPENXR_MULTILAYER_STACK_PLAN.md):
  SurfaceTexture/OES and multilayer diagnostics plan.
- [Quest-to-Quest online streaming roadmap](QUEST_TO_QUEST_ONLINE_STREAMING_ROADMAP.md):
  staged online streaming roadmap.
- [Quest-to-Quest native relay session, 2026-05-19](QUEST_TO_QUEST_NATIVE_RELAY_SESSION_2026_05_19.md):
  public-safe two-way native relay retrospective and next-run diagnostic plan.
- [Makepad Android build compatibility plan](MAKEPAD_ANDROID_BUILD_COMPATIBILITY_PLAN.md):
  Makepad packaging and hotload compatibility plan.
- [Makepad stereo comparison iteration](MAKEPAD_STEREO_COMPARISON_ITERATION.md):
  large chronological implementation ledger.
- [Makepad XR GPU page fault investigation](MAKEPAD_XR_GPU_PAGE_FAULT_INVESTIGATION.md):
  historical GPU fault isolation log.
- [Machine repo tooling audit](MACHINE_REPO_TOOLING_AUDIT.md): sanitized
  machine-wide repository and tooling audit.
- [General tool import audit](GENERAL_TOOL_IMPORT_AUDIT.md): public-safe import
  candidates and reference tooling review.

## GitHub Pages

The Pages layer starts at [index.html](index.html). Keep Pages focused on the
public-facing summary; keep agent routing and command detail in Markdown.
