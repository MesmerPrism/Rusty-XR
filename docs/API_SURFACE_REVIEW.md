# API Surface Review

This review records the current public utility surface before hardware and
native-adapter examples are added.

## Current Decision

The public core is stable enough for synthetic examples, schema alignment, and
a minimal Rust-native Android APK smoke test. It is not yet stable enough for a
first public tag or for native Quest OpenXR, Vulkan, Makepad, BLE-native, or
LSL-native adapters.

## Surface That Can Be Used By Examples

- `rusty-xr-contracts`: math, pose, timing, camera/depth metadata, temporal
  camera projection policy/state/metric contracts, hands, interaction rays,
  developer-home panel/session/launcher/settings/recovery contracts, plain
  media layers, effect-stack diagnostic descriptors, native passthrough style
  descriptors, safety-gated visual strobe descriptors, render payloads,
  runtime counters, room mesh source state, semantic room mesh snapshots, and
  capture lifecycle state.
- `rusty-quest-makepad-runtime-config`: generic runtime keys, values, and Android property
  naming helpers.
- `rusty-xr-ble`, `rusty-xr-lsl`, and `rusty-xr-polar`: protocol/data models
  without native transport backends.
- `rusty-xr-broker-model`: broker command envelopes, stream/session metadata,
  transport-lane contracts, timing stamps, stream replay records, and
  broker-clock snapshot/stamp/correlation/health/sync-probe models.
- `rusty-xr-camera-model`, `rusty-xr-depth-model`, `rusty-xr-sdf`, and
  `rusty-xr-particles`: deterministic camera projection and temporal
  homography-motion helpers, dynamic mesh coordinate sampling, same-surface and
  cross-surface neighborhoods, live hand-mesh snapshot anchors, dynamic mesh
  collider surfaces with diagnostic shells, dynamic mesh-to-SDF conversion,
  public SDF particle attraction helpers, and public snapshots that can be
  demonstrated with synthetic data or public examples.

## Stabilization Notes

- Keep `serde` opt-in and default features dependency-light.
- Keep generated schemas ignored until each schema has a consumer and review.
- Prefer adding schemas for cross-repo metadata before adding native adapters.
- Keep APK payloads, signing, install scripts, local captures, and package
  identity in downstream or companion repositories.
- Keep Rusty Kiosk custom-home rendering, ADB helper lifecycle, focus recovery
  execution, and managed kiosk policy in app shells or companion tooling.
  Public core should only model the data contracts.
- Keep effect-stack render passes as descriptors and comparison reports.
  Shader implementation, concrete visual tuning, native texture ownership, and
  generated captures stay in downstream app shells.
- Use `quest-app-catalog.schema.json` as the shared metadata shape for public
  example APK listings when example APKs are later published.
- Keep session-level capture/OpenXR examples public-authored and adapter-owned:
  platform calls may live in examples, while reusable data contracts and
  deterministic mesh/SDF/particle helpers stay in core crates.
- Keep the first APK example limited to native-library loading, public contract
  serialization, launch, stop, snapshot, and basic frame-callback diagnostics.

## Deferred

- Native Quest passthrough or environment-depth adapters.
- Native passthrough handle creation, frame submission, mesh upload, and LUT
  allocation.
- Presentation of intentional strobing stimuli without an explicit safety gate.
- Native camera acquisition or platform texture import.
- Runtime implementation of temporal camera projection smoothing, depth-aware
  reprojection, or space-warp submission.
- Runtime permission prompt implementations.
- Hardware APK examples beyond the minimal smoke-test shell.
- Downstream app integration or consumer repo rewiring.
