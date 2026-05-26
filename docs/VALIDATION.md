# Validation Commands

Run the smallest command set that matches the changed surface, then record what
was skipped and why.

## Docs-Only Changes

```powershell
python tools\workflow\check_powershell_workflow_safety.py --repo-root .
python tools\docs\check_links.py --repo-root .
python tools\boundary-scan\rusty_xr_boundary_scan.py --repo-root .
```

## Rust Source Changes

```powershell
cargo fmt --all --check
cargo test --workspace
python tools\workflow\check_powershell_workflow_safety.py --repo-root .
python tools\schema\export_schemas.py --check
python tools\quest-camera-profile\Validate-CanvasCustomParityArtifacts.py --self-test
python tools\quest-camera-profile\Validate-ProjectionRuntimeReadback.py --self-test
python tools\boundary-scan\rusty_xr_boundary_scan.py --repo-root .
```

Use the broader set before publishing broad API changes:

```powershell
cargo test --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
cargo test --doc --workspace --all-features
```

## Schema And Catalog Changes

```powershell
python tools\workflow\check_powershell_workflow_safety.py --repo-root .
python tools\schema\export_schemas.py --check
python tools\quest-camera-profile\Build-CameraTextureLaneContracts.py --self-test
python tools\quest-camera-profile\Validate-CanvasCustomParityArtifacts.py --self-test
python tools\quest-camera-profile\Validate-ProjectionRuntimeReadback.py --self-test
python tools\schema\check_quest_app_catalog.py tools\schema\fixtures\quest-app-catalog.example.json
python tools\boundary-scan\rusty_xr_boundary_scan.py --repo-root .
```

## Public Example Checks

Source-only examples:

```powershell
cargo run -p rusty-xr-contracts --example effect_stack_diagnostic_manifest --features serde
cargo run -p rusty-xr-contracts --example kiosk_command_run_record --features serde
cargo run -p rusty-xr-particles --example dynamic_mesh_coordinates
cargo run -p rusty-xr-particles --example hand_mesh_fixture_samples
cargo run -p rusty-xr-particles --example hand_mesh_dynamic_collider
cargo run -p rusty-xr-particles --example hand_mesh_sdf_attraction
cargo run -p rusty-xr-quest-diagnostics --example quest_provider_snapshot
cargo run -p rusty-xr-broker-client-probe -- status
cargo test --locked --manifest-path examples\makepad-camera-shell\Cargo.toml
```

The Makepad camera shell is intentionally outside the root workspace, so use
the manifest-path command for source changes in that example.

Android source builds:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-minimal-apk\tools\Build-QuestMinimalApk.ps1
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-apk\tools\Build-QuestBrokerApk.ps1
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-shell-helper\tools\Build-BrokerShellHelper.ps1
powershell -ExecutionPolicy Bypass -File .\examples\quest-composite-layer-apk\tools\Build-QuestCompositeLayerApk.ps1 -OpenXrLoaderPath <path-to-libopenxr_loader.so>
```

Device validation requires an explicit operator workflow and is outside normal
docs/source validation.

## Boundary Discipline

Always run the boundary scan when docs or source touch:

- Quest, Android, broker, kiosk, or diagnostics language.
- Public/private extraction decisions.
- Example names, package descriptions, catalog metadata, or command evidence.
- Any copied or adapted code from a reference project.
