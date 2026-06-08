# Agent Notes

This is the public Rusty XR core repository. Keep committed files public,
portable, and useful without access to downstream app repositories.

Rusty Morphospace is the top-level project/platform umbrella for the clean
refactor. Do not rename this public repository, crate/package ids, public APIs,
or compatibility diagnostics as part of that umbrella decision. New generic
contracts should still land in their concrete lanes such as Matter, Lattice,
Manifold, Optics, GUI, Studio, or Quest.

The clean refactor now uses `Rusty Lattice` for generic situated relation
contracts: spaces, transforms, tracked poses, view sets, spatial input roles,
frame-state binding, calibration, validity, confidence, and runtime capability
snapshots. Do not rename this public repository or break existing public
`Rusty-XR` APIs as part of that decision. New generic relation extractions
should be planned as scoped migrations with compatibility notes.

## Public Boundary

Do not commit:

- private/local repository names or paths
- app-specific package identities, launch activities, signing config, release
  payload paths, headset serials, or generated artifacts
- private visual-effect behavior, product parity details, project-specific
  tuning constants, study logic, or private stream names
- APK/AAB bytes, loader binaries, keystores, screenshots, captures, logcat
  dumps, media frames, diagnostics bundles, or cache folders

Use generic language such as "downstream app", "target app", "public example",
"runtime profile", and "device profile".

## Publishing And Dependencies

Before adding dependencies, optional adapters, generated APK release assets, or
codec/media stack code:

- Keep core crates contract-first and framework-neutral.
- Prefer permissive source dependencies with generated notices.
- Keep native SDKs, WebRTC/NDI payloads, FFmpeg/GStreamer/libx264/libx265,
  signing material, generated APKs, captures, and tool caches out of core.
- Use source-only adapters or user-supplied SDK/runtime paths for sensitive
  integrations.
- For distributed APKs, publish source commit/tag, APK SHA-256, signing mode,
  included native libraries, permissions, and third-party notices.
- Prefer Android platform MediaCodec for H.264/H.265 before bundling codec
  libraries.

## Orientation

Start with:

- `README.md`
- `docs/README.md`
- `docs/MODULE_CRATE_MAP.md`
- `docs/EXAMPLES_MATRIX.md`
- `docs/UNITY_EXAMPLE_INTEGRATION.md`
- `docs/API_CLI_MCP_ENTRYPOINTS.md`
- `docs/PUBLIC_EXTRACTION_WORKFLOW.md`
- `docs/VALIDATION.md`
- `docs/IMPLEMENTATION_PLAN.md`
- `docs/ANDROID_QUEST_APK_BUILDING.md`
- `docs/RUSTY_XR_COMPANION_INTEGRATION.md`
- `docs/MEDIA_PIPELINE_AND_PERMISSIONS.md`
- `docs/SERIALIZATION_AND_SCHEMA_POLICY.md`
- `docs/FEATURE_AND_ADAPTER_POLICY.md`
- `docs/PROVENANCE.md`
- `skills/rusty-xr-builder/SKILL.md`

For code changes, inspect the owning crate before editing. Prefer the smallest
public contract or helper that solves the problem, then add synthetic tests.

## Source Workspace With Companion

When Rusty XR Companion Apps is installed for local Quest operations, keep the
two public repos as siblings:

```text
<workspace>\Rusty-XR
<workspace>\Rusty-XR-Companion-Apps
```

Rusty XR owns public Rust contracts, schemas, examples, and APK source.
Companion owns Windows UX, managed Quest operator tooling, install, launch,
cast, diagnostics, and catalog verification.

From the companion repo, ask the CLI for the current workspace guide:

```powershell
dotnet run --project .\src\RustyXr.Companion.Cli -- workspace guide --root <workspace>
```

Build the minimal Android smoke-test APK from this repo:

```powershell
rustup target add aarch64-linux-android
powershell -ExecutionPolicy Bypass -File .\examples\quest-minimal-apk\tools\Build-QuestMinimalApk.ps1
```

Then verify it from the companion repo:

```powershell
dotnet run --project .\src\RustyXr.Companion.Cli -- catalog verify --path ..\Rusty-XR\examples\quest-minimal-apk\catalog\rusty-xr-quest-minimal.catalog.json --app rusty-xr-quest-minimal --serial <serial> --install --launch --device-profile perf-smoke-test --runtime-profile minimal-contract-log --settle-ms 4000 --out .\artifacts\verify
```

Build the immersive composite-layer example only on machines with the Android
tooling and Quest-compatible OpenXR loader:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-composite-layer-apk\tools\Build-QuestCompositeLayerApk.ps1 -OpenXrLoaderPath <path-to-libopenxr_loader.so>
```

Then verify it from the companion repo:

```powershell
dotnet run --project .\src\RustyXr.Companion.Cli -- catalog verify --path ..\Rusty-XR\examples\quest-composite-layer-apk\catalog\rusty-xr-quest-composite-layer.catalog.json --app rusty-xr-quest-composite-layer --serial <serial> --stop-catalog-apps --install --launch --device-profile xr-composite-smoke-test --runtime-profile camera-stereo-gpu-composite --settle-ms 9000 --logcat-lines 1400 --out .\artifacts\verify
```

The companion-managed tooling cache covers `adb`, `hzdb`, and `scrcpy`.
Rust/Cargo, Android SDK/NDK/JDK, OpenXR loader binaries, and signing material
remain explicit local build inputs.

Treat `hzdb` as an optional Meta Quest provider, not a required core
dependency. Before adding `hzdb`, MCP, docs/API search, Perfetto, device
health, app lifecycle, file, log, screenshot, or proximity workflows, read
`docs/META_QUEST_HZDB_PROVIDER_PLAN.md`. Mutating operations such as shell
commands, file deletion, app clear/uninstall, proximity changes, port
forwarding, `setprop`, root, and MCP config writes need an explicit operator
gate in the invoking tool.
For Rusty Kiosk work, provider evidence is part of the default tracking setup:
record the command goal, provider used, fallback command, foreground before and
after, broker clock/status, and whether any Meta menu/settings entry was
intentional.

## Local Quest Baseline

For local headset validation, prefer a broker/developer-home operator baseline
when it is available. The public source name for this kind of surface is Rusty
Kiosk: a normal broker/developer home used to launch targets, inspect status,
record clock evidence, and recover focus during development.

This is a workflow baseline, not a platform claim. Do not describe it as a
system Home replacement, Home/Menu interceptor, arbitrary Android app window
manager, or managed-device kiosk. If a test needs raw Horizon OS behavior,
first-run prompts, Store/SideQuest behavior, or managed-device policy, state
that the broker/developer-home baseline was intentionally skipped.

## Validation

Run the relevant subset for the change:

```powershell
cargo fmt --all --check
cargo test --workspace
cargo test --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
cargo test --doc --workspace --all-features
python tools/docs/check_links.py --repo-root .
python tools/workflow/check_powershell_workflow_safety.py --repo-root .
python tools/schema/export_schemas.py --check
python tools/quest-camera-profile/Validate-CanvasCustomParityArtifacts.py --self-test
python tools/schema/check_quest_app_catalog.py tools/schema/fixtures/quest-app-catalog.example.json
python tools/boundary-scan/rusty_xr_boundary_scan.py --repo-root .
```

If only docs changed, still run the public-boundary scan before pushing.
