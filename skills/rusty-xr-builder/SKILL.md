---
name: rusty-xr-builder
description: Build, extend, document, or review Rusty XR public crates and examples. Use when working in the Rusty XR repo or when an agent needs to create framework-neutral XR contracts, runtime config helpers, LSL models, Quest diagnostics, camera/depth utilities, SDF contracts, particle primitives, or public Android/Quest shell guidance without leaking app-specific package identity, private rendering behavior, signing data, generated artifacts, or downstream project details.
---

# Rusty XR Builder

Use this skill to work on the public Rusty XR repository.

## First Steps

1. Read `README.md`.
2. Read `docs/IMPLEMENTATION_PLAN.md`.
3. For Android / Quest APK questions, read `docs/ANDROID_QUEST_APK_BUILDING.md`.
4. For media-pipeline, Windows streaming, or APK permission questions, read `docs/MEDIA_PIPELINE_AND_PERMISSIONS.md`.
5. For BLE, LSL, or Polar H10 data-pipeline questions, read `docs/BLE_LSL_POLAR_PIPELINE.md`.
6. For general Makepad/Rusty-XR-family tool imports, read `docs/GENERAL_TOOL_IMPORT_AUDIT.md`.
7. For broader machine-wide Quest/tooling candidates, read `docs/MACHINE_REPO_TOOLING_AUDIT.md`.
8. For serialization or schema work, read `docs/SERIALIZATION_AND_SCHEMA_POLICY.md`.
9. For optional native/framework adapters, read `docs/FEATURE_AND_ADAPTER_POLICY.md`.
10. For extracted utility provenance, read `docs/PROVENANCE.md`.
11. For public API review status, read `docs/API_SURFACE_REVIEW.md`.
12. For Rusty XR Companion Apps catalog alignment, read `docs/RUSTY_XR_COMPANION_INTEGRATION.md`.
13. Inspect the relevant crate before editing.
14. Keep changes public, dependency-light, and testable without downstream app repos.

## Public Boundary

Rusty XR may contain:

- Framework-neutral XR contracts.
- XR canvas/ray interaction contracts and hand-menu anchors.
- Runtime configuration models and helpers.
- BLE UUID, GATT, notification, scan-result, and Android Bluetooth permission models.
- Pure LSL stream, status, staleness, roundtrip, and telemetry models.
- Polar H10 GATT IDs, HR/RR decoding, ECG/ACC frame contracts, PMD command builders, and sanitized LSL schemas.
- Generic Quest diagnostic status models.
- Camera metadata, intrinsics scaling, projection, and timestamp helpers.
- Plain mono/stereo media layer descriptors, border tuning, visual feedback tuning, and performance hints.
- Room mesh source state, semantic room mesh snapshots, and capture lifecycle metadata.
- Depth metadata, readiness, and payload summaries.
- Packed SDF, mesh snapshot, and sampling contracts.
- Sparse TSDF scan snapshots, scan surface samples, and scan-fusion stats.
- General particle, fixed-step, animation, and render-payload primitives.
- Opt-in `serde` support for stable public contracts.
- Public schema exports generated to ignored locations until reviewed.
- Public provenance metadata and boundary scanning utilities.
- Public docs and examples authored specifically for this repo.
- Public schema shapes for companion/operator metadata such as Quest app catalogs.

Do not add:

- App-specific package names, launch activities, signing, release payloads, headset serials, or local paths.
- Private rendering stacks, exact tuning constants, product parity details, or private scene behavior.
- Generated captures, screenshots, datasets, performance artifacts, or device logs.
- Framework-specific code unless it is a thin optional adapter.
- Native `liblsl`, Android, OpenXR, Vulkan, Makepad, Unity, or Meta SDK dependencies in pure model crates.
- App-specific simulation logic, private biofeedback tuning, or project-specific LSL stream names.

If a candidate is ambiguous, add the smaller contract first and leave concrete behavior in the app shell.

The first public examples are synthetic contracts-only demos:
`cargo run -p rusty-xr-contracts --example plain_stereo_feedback_layout --features serde`.
`cargo run -p rusty-xr-contracts --example composite_feedback_session --features serde`.
The first public APK-producing example is a minimal Rust-native Android smoke
test:
`powershell -ExecutionPolicy Bypass -File .\examples\quest-minimal-apk\tools\Build-QuestMinimalApk.ps1`.
Hardware/OpenXR/native-adapter examples remain deferred until the utility
surface review passes.

## Crate Map

- `rusty-xr-contracts`: shared math, pose, timing, eye-view, camera/depth metadata, hand snapshots, plain stereo/feedback layer descriptors, border tuning, performance hints, render payloads, counters, room mesh source state, semantic room mesh snapshots, and capture lifecycle metadata.
- `rusty-xr-contracts` also owns small interaction contracts such as `InteractionRay`, `XrCanvasSurface`, `HandMenuAnchor`, and `HandInfluencePoint`.
- `rusty-xr-runtime-config`: generic runtime keys, typed values, config maps, and public Android property naming helpers.
- `rusty-xr-ble`: pure BLE UUID, scan result, GATT path, notification, operation, and Android Bluetooth permission models.
- `rusty-xr-lsl`: pure LSL descriptor, role, channel schema, filter, endpoint status, staleness, roundtrip, biofeedback, and telemetry models.
- `rusty-xr-polar`: Polar H10 GATT IDs, HR/RR decoder, uncompressed ECG/ACC PMD decoders, PMD command builders, and public LSL schemas.
- `rusty-xr-quest-diagnostics`: generic device readiness, package launch, frame-rate, and runtime status models.
- `rusty-xr-camera-model`: intrinsics scaling, projection/back-projection, and timestamp matching.
- `rusty-xr-depth-model`: depth readiness classification and frame summaries.
- `rusty-xr-sdf`: packed SDF grid/sample contracts, bounds, sampling, and triangle mesh snapshots.
- `rusty-xr-sdf` also owns sparse TSDF scan snapshots and scan-fusion stats.
- `rusty-xr-particles`: particle state/set, fixed-step clock, and render payload generation.

## Design Rules

- Prefer plain Rust data at crate boundaries.
- Keep APIs deterministic and easy to test with synthetic data.
- Use `std` only unless a dependency is clearly justified.
- Keep default crate features dependency-light.
- Put `serde` behind an explicit feature and add round-trip tests.
- Keep app shell responsibilities out of the core.
- Keep adapters optional and downstream-facing.
- Add unit tests for every nontrivial helper.
- Update public docs when a crate becomes materially more useful.

## Android / Quest APK Rule

Rusty XR supports Rust-based Android / Quest app shells, but it is not itself an APK repo.

The public core owns contracts and reusable helpers. The app shell owns:

- Android package identity and signing.
- Manifest permissions and activity declarations.
- Android lifecycle and permission prompts.
- OpenXR loader/runtime integration.
- Renderer backend, swapchains, frame loop, foveation, and platform timing.
- Install, launch, log capture, visual validation, and release payloads.

When asked to add APK support, first document or define the public shell boundary. Do not add private package names or shell scripts unless they are clean public examples.

Rusty XR Companion Apps is the public operator repo for future example APK
catalog metadata. Keep the shared shape aligned through
`quest-app-catalog.schema.json`; do not copy Windows app shell code or APK
payloads into this core repo.

## Media Pipeline And Permissions

Keep media sources separate:

- Native passthrough layer: compositor-owned, not an app-sampleable texture.
- Passthrough Camera API / Camera2: raw camera frames for CV/ML or app processing.
- MediaProjection: final display or app-window capture, including app UI/overlays.
- App render payloads: app-owned frames, particles, depth summaries, counters, or synthetic debug visuals.
- Plain feedback borders: public layout geometry, approved border tuning, approved visual-feedback scalar knobs, and performance hints only; no private downstream image-processing passes, effect-map implementations, geometric-effect implementation, scene behavior, or project-specific shader code.

For Windows streaming, prefer generic tools and protocols:

- `tools/media-pipeline/frame_receiver.py` receives length-prefixed frame packets on Windows.
- Use `adb reverse tcp:<port> tcp:<port>` when the headset app connects to a Windows receiver.
- Use `adb forward tcp:<port> tcp:<port>` when Windows connects to a device-hosted server.
- Keep app-private file export/pull tooling generic and parameterized by package name when added.

Permission rules:

- Manifest/install-time permissions: declare normal permissions such as `INTERNET`, `ACCESS_NETWORK_STATE`, `FOREGROUND_SERVICE`, and `FOREGROUND_SERVICE_MEDIA_PROJECTION`; these do not spawn runtime headset popups.
- Runtime/dangerous permissions: declare and request from a foreground Activity to spawn the headset popup. Camera/headset-camera, microphone, Android 12+ Nearby Devices permissions for Bluetooth scan/connect/advertise, legacy scan-location permission, and notifications belong here when required by target API/platform behavior.
- Development launcher grants: ADB can usually grant declared ordinary runtime permissions with `pm grant` or `install -g`; do not treat this as production UX.
- MediaProjection: request user consent with `MediaProjectionManager.createScreenCaptureIntent()` for each capture session; a launcher cannot bypass this.
- Special permissions: route users to settings or OEM-specific surfaces; avoid them in public examples unless necessary.

When documenting a new media feature, state which permissions are manifest-only, which a launcher can grant for development, and which must be requested in-headset by the app.

## BLE / LSL / Polar Rule

Keep Bluetooth and LSL transport code out of pure crates:

- `rusty-xr-ble` may describe Android permission requirements and GATT operations, but it must not call Android APIs.
- `rusty-xr-polar` may decode standard Heart Rate Measurement payloads and tested PMD ECG/ACC frames, but it must not embed app-specific simulation code or app-specific biofeedback behavior.
- `rusty-xr-lsl` may define public stream descriptors and schemas, but native `liblsl` inlet/outlet backends should be optional adapters.
- Public examples should use sanitized stream types such as `rusty.xr.polar.ecg`, not private project names.
- Attribution for Polar H10 specifics should point to `https://mesmerprism.github.io/PolarH10/` and the Polar BLE SDK MIT project.

## Implementation Workflow

1. Choose the smallest public crate that owns the concept.
2. Add or refine contracts before utility implementations.
3. Avoid reaching into downstream app repos.
4. Add synthetic unit tests.
5. Run examples when they touch the edited surface:

```powershell
cargo run -p rusty-xr-contracts --example plain_stereo_feedback_layout --features serde
cargo run -p rusty-xr-contracts --example composite_feedback_session --features serde
powershell -ExecutionPolicy Bypass -File .\examples\quest-minimal-apk\tools\Build-QuestMinimalApk.ps1
```

6. Run validation:

```powershell
cargo fmt --all --check
cargo test --workspace
cargo test --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
cargo test --doc --workspace --all-features
python tools/docs/check_links.py --repo-root .
python tools/schema/export_schemas.py --check
python tools/schema/check_quest_app_catalog.py tools/schema/fixtures/quest-app-catalog.example.json
python tools/boundary-scan/rusty_xr_boundary_scan.py --repo-root .
```

7. If docs changed only, still run the checks when code is already dirty in the working tree.
8. Summarize boundary decisions in the final response.

## Review Checklist

- The change builds without downstream repos.
- The public docs do not expose private app behavior.
- The crate remains framework-neutral unless explicitly documented as an adapter.
- Tests cover validation, parsing, math, sampling, or payload generation behavior.
- Android / Quest wording preserves the core-vs-shell split.
