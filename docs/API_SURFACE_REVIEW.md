# API Surface Review

This review records the current public utility surface before hardware,
APK, or native-adapter examples are added.

## Current Decision

The public core is stable enough for synthetic examples and schema
alignment work. It is not yet stable enough for a first public tag or for
native Quest, Android, OpenXR, Vulkan, Makepad, BLE-native, or LSL-native
adapters.

## Surface That Can Be Used By Examples

- `rusty-xr-contracts`: math, pose, timing, camera/depth metadata, hands,
  interaction rays, plain media layers, render payloads, runtime counters, room
  mesh source state, semantic room mesh snapshots, and capture lifecycle state.
- `rusty-xr-runtime-config`: generic runtime keys, values, and Android property
  naming helpers.
- `rusty-xr-ble`, `rusty-xr-lsl`, and `rusty-xr-polar`: protocol/data models
  without native transport backends.
- `rusty-xr-camera-model`, `rusty-xr-depth-model`, `rusty-xr-sdf`, and
  `rusty-xr-particles`: deterministic helpers and public snapshots that can be
  demonstrated with synthetic data.

## Stabilization Notes

- Keep `serde` opt-in and default features dependency-light.
- Keep generated schemas ignored until each schema has a consumer and review.
- Prefer adding schemas for cross-repo metadata before adding native adapters.
- Keep APK payloads, signing, install scripts, local captures, and package
  identity in downstream or companion repositories.
- Use `quest-app-catalog.schema.json` as the shared metadata shape for public
  example APK listings when example APKs are later published.
- Keep session-level examples synthetic until an adapter can prove permission,
  capture, transport, and launch behavior without downstream private wiring.

## Deferred

- Native Quest passthrough or environment-depth adapters.
- Native camera acquisition or platform texture import.
- Runtime permission prompt implementations.
- Hardware/APK examples.
- Downstream app integration or consumer repo rewiring.
