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

The companion release can install or update the operator-side tools that are
reasonable to manage in a per-user cache: Android platform-tools / `adb`, Meta
`hzdb`, and `scrcpy`. Rust/Cargo, Android SDK/NDK/JDK, OpenXR loader binaries,
and signing identity remain explicit local build inputs for machines that build
APK bytes from source.

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

The first public broker proof-of-concept is
`examples/quest-broker-apk/`. Its catalog is:

```text
examples/quest-broker-apk/catalog/rusty-xr-quest-broker.catalog.json
```

It declares runtime profiles for the localhost WebSocket/HTTP status path,
optional broker-to-laptop LSL forwarding when a compliant Android `liblsl.so`
is supplied at build time, optional OSC latency egress, and OSC drive ingress
from a laptop to localhost WebSocket clients. The broker has been validated
with a Unity client on Quest, but this iteration intentionally does not publish
a Unity project. A dedicated public Unity example is planned after the broker
API shape settles.

Validate a catalog with:

```powershell
python tools/schema/check_quest_app_catalog.py tools/schema/fixtures/quest-app-catalog.example.json
```

When both repositories are checked out as siblings on a development machine,
the same validator can be pointed at the Companion sample catalog:

```powershell
python tools/schema/check_quest_app_catalog.py ..\Rusty-XR-Companion-Apps\samples\quest-session-kit\apk-catalog.example.json
```

## Source Workspace

The recommended local source layout is:

```text
<workspace>\Rusty-XR
<workspace>\Rusty-XR-Companion-Apps
```

From `Rusty-XR-Companion-Apps`, run:

```powershell
dotnet run --project .\src\RustyXr.Companion.Cli -- workspace guide --root <workspace>
```

That command prints the expected catalog paths, APK output paths, and install /
launch / verification commands for the public Rusty XR minimal,
composite-layer, and broker examples.

Minimal APK loop:

```powershell
cd <workspace>\Rusty-XR
powershell -ExecutionPolicy Bypass -File .\examples\quest-minimal-apk\tools\Build-QuestMinimalApk.ps1
cd <workspace>\Rusty-XR-Companion-Apps
dotnet run --project .\src\RustyXr.Companion.Cli -- catalog verify --path ..\Rusty-XR\examples\quest-minimal-apk\catalog\rusty-xr-quest-minimal.catalog.json --app rusty-xr-quest-minimal --serial <serial> --install --launch --device-profile perf-smoke-test --runtime-profile minimal-contract-log --settle-ms 4000 --out .\artifacts\verify
```

Composite-layer APK loop:

```powershell
cd <workspace>\Rusty-XR
powershell -ExecutionPolicy Bypass -File .\examples\quest-composite-layer-apk\tools\Build-QuestCompositeLayerApk.ps1 -OpenXrLoaderPath <path-to-libopenxr_loader.so>
cd <workspace>\Rusty-XR-Companion-Apps
dotnet run --project .\src\RustyXr.Companion.Cli -- catalog verify --path ..\Rusty-XR\examples\quest-composite-layer-apk\catalog\rusty-xr-quest-composite-layer.catalog.json --app rusty-xr-quest-composite-layer --serial <serial> --stop-catalog-apps --install --launch --device-profile xr-composite-smoke-test --runtime-profile camera-stereo-gpu-composite --settle-ms 9000 --logcat-lines 1400 --out .\artifacts\verify
```

Broker APK loop:

```powershell
cd <workspace>\Rusty-XR
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-apk\tools\Build-QuestBrokerApk.ps1
cd <workspace>\Rusty-XR-Companion-Apps
dotnet run --project .\src\RustyXr.Companion.Cli -- catalog verify --path ..\Rusty-XR\examples\quest-broker-apk\catalog\rusty-xr-quest-broker.catalog.json --app rusty-xr-quest-broker --serial <serial> --stop-catalog-apps --install --launch --device-profile broker-smoke-test --runtime-profile broker-latency-websocket-lsl --settle-ms 5000 --logcat-lines 1000 --out .\artifacts\verify
dotnet run --project .\src\RustyXr.Companion.Cli -- osc send --host <quest-lan-ip> --port 9000 --address /rusty-xr/drive/radius --arg float:0.75
```

## Boundary

The catalog may contain public package names and public release-asset URLs when
examples are intentionally published. It should not contain local machine paths,
signing material, private package names, generated captures, or private runtime
profiles.
