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
