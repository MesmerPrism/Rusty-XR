# Rusty XR Minimal Quest APK

This is the first public APK-producing example for Rusty XR. It is deliberately
small: a Java Android activity loads a Rust `cdylib`, asks Rust to serialize
synthetic Rusty XR contracts as JSON, and displays the result with a simple
frame-callback counter.

It does not use OpenXR, Vulkan, MediaProjection, Passthrough Camera API,
environment depth, native compositor layers, or downstream visual effects.

## Build

The build script uses Android SDK, NDK, OpenJDK, `aapt2`, `d8`, `zipalign`, and
`apksigner`. It can use Android tooling from a Unity Android module or another
Android SDK installation.

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-minimal-apk\tools\Build-QuestMinimalApk.ps1
```

The output APK is:

```text
examples/quest-minimal-apk/build/outputs/rusty-xr-quest-minimal-debug.apk
```

Build outputs, debug keystores, and APK bytes are ignored and must not be
committed.

## Companion Catalog

The catalog metadata lives at:

```text
examples/quest-minimal-apk/catalog/rusty-xr-quest-minimal.catalog.json
```

After building the APK, install, launch, and verify it through Rusty XR
Companion Apps:

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- catalog install --path .\examples\quest-minimal-apk\catalog\rusty-xr-quest-minimal.catalog.json --app rusty-xr-quest-minimal --serial <serial>

dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- catalog verify --path .\examples\quest-minimal-apk\catalog\rusty-xr-quest-minimal.catalog.json --app rusty-xr-quest-minimal --serial <serial> --launch --device-profile perf-smoke-test --settle-ms 4000 --out .\artifacts\verify
```

The verifier captures a headset snapshot and app diagnostics through ADB. The
visual truth step is still human: confirm in the headset or through casting that
the activity shows the Rusty XR session JSON and frame counter.
