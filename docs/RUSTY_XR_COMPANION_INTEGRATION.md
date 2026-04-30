# Rusty XR Companion Integration

Rusty XR Companion Apps is the public Windows operator repository for install,
launch, device profiles, casting, diagnostics, and future public example APK
distribution metadata.

Rusty XR core owns:

- reusable Rust contracts
- public JSON schema shapes
- synthetic examples that do not require APKs or headset hardware
- public documentation for adapter boundaries

Rusty XR Companion Apps owns:

- WPF and CLI operator UX
- ADB, optional Quest tooling, and optional casting tool orchestration
- local APK install/launch workflows
- public catalog files that point to local APKs or release assets
- Windows release packaging

## Shared Catalog Shape

The current shared connection point is `quest-app-catalog.schema.json`.

Catalogs should use:

```json
{
  "schemaVersion": "rusty.xr.quest-app-catalog.v1",
  "apps": [],
  "deviceProfiles": [],
  "runtimeProfiles": []
}
```

This lets future public example APK metadata be shared with the Companion app
without making either repository depend on the other's build system.

The synthetic `composite_feedback_session` Rust example emits a catalog hint
using this schema version, but it does not publish an APK or require the
Companion app to run.

The first APK-producing public example is `examples/quest-minimal-apk/`. Its
catalog is:

```text
examples/quest-minimal-apk/catalog/rusty-xr-quest-minimal.catalog.json
```

The catalog points at the local debug APK output path under the example's
ignored `build/` directory. Rusty XR core owns the source and metadata;
Companion owns install, launch, stop, device profile, cast, and verification
flows.

The first immersive public example is
`examples/quest-composite-layer-apk/`. Its catalog declares separate runtime
profiles for `synthetic-composite-layer`, `camera-diagnostic-cpu-copy`,
`camera-source-diagnostics`, `camera-gpu-buffer-probe`,
`camera-stereo-gpu-composite`, and optional `media-projection-stream`.
Runtime-profile `values` are passed as Activity launch extras by the Companion
app; MediaProjection profile values must not be interpreted as the render
source for the custom layer.

The `camera-stereo-gpu-composite` profile is the accepted public raw-camera
reference path for the tested Quest Camera2 provider: paired `PRIVATE`
hardware buffers, platform intrinsics/pose metadata, display-eye
screen-to-camera homographies, `rotate0` per-source texture orientation, and
explicit manual visual acceptance. New device/runtime variants should rerun
`camera-source-diagnostics` and keep the manual visual gate until orientation,
eye mapping, head-motion stability, and border behavior are inspected.

Validate a catalog with:

```powershell
python tools/schema/check_quest_app_catalog.py tools/schema/fixtures/quest-app-catalog.example.json
```

When both repositories are checked out as siblings on a development machine,
the same validator can be pointed at the Companion sample catalog:

```powershell
python tools/schema/check_quest_app_catalog.py ..\Rusty-XR-Companion-Apps\samples\quest-session-kit\apk-catalog.example.json
```

## Boundary

The catalog may contain public package names and public release-asset URLs when
examples are intentionally published. It should not contain local machine paths,
signing material, private package names, generated captures, or private runtime
profiles.
