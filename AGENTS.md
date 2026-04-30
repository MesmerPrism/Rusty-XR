# Agent Notes

This is the public Rusty XR core repository. Keep committed files public,
portable, and useful without access to downstream app repositories.

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

## Orientation

Start with:

- `README.md`
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

## Validation

Run the relevant subset for the change:

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

If only docs changed, still run the public-boundary scan before pushing.
