# Android / Quest APK Building

Rusty XR is a public core workspace, not an APK shell. It is still important
that downstream Rust XR apps can build APKs cleanly, so this document records
the build responsibilities and the intended integration shape.

## Responsibility Split

Rusty XR public crates should own:

- framework-neutral contracts
- runtime configuration models
- camera, depth, SDF, particle, LSL, and diagnostic utility code
- optional thin adapters after the core contracts settle
- public examples that are authored specifically for this repository

The Android / Quest app shell should own:

- Android package identity and signing
- `AndroidManifest.xml` permissions and activity declarations
- OpenXR loader/runtime integration
- Android lifecycle and permission prompts
- Vulkan or renderer backend setup
- swapchains, frame loop, foveation, and platform timing
- headset install, launch, log capture, and visual validation
- app-specific assets, scenes, rendering policy, and release payloads

This split keeps the public core reusable and prevents app-specific package
names, tuning, release metadata, or generated artifacts from leaking into the
public repo.

## Recommended Shell Shape

Use one reusable Rust Android/OpenXR app shell per product family. The shell
depends on Rusty XR crates and converts platform-specific state into plain
public contracts:

```text
Android / OpenXR / renderer shell
  -> Rusty XR contracts and utility crates
  -> app or experiment logic
  -> render payloads, counters, commands
```

Experiment crates should not own APK packaging, OpenXR session setup, Android
permissions, or renderer lifecycle. They should receive snapshots such as poses,
eye views, camera metadata, hand meshes, SDF grids, runtime config, commands,
and frame timing. They should return render payloads, counters, and app-neutral
diagnostics.

## Build Routes

There are two practical public build routes for downstream apps.

### Existing Renderer Shell

Use a Rust renderer/application framework that already owns Android packaging,
activity lifecycle, native library loading, and renderer setup. In that model,
Rusty XR crates are ordinary Rust dependencies and the shell owns the APK.

The shell should expose a small adapter layer that converts framework types to
Rusty XR contracts. Keep that adapter feature-gated and separate from pure
contracts.

### Custom Android Shell

For a custom shell, keep these pieces local to the app repository:

- Android project or packaging tool configuration
- package name and signing config
- manifest permissions
- native entrypoint
- OpenXR loader setup
- Vulkan or other renderer initialization
- install / launch scripts

The shell can build Rust crates with Android targets such as:

```powershell
rustup target add aarch64-linux-android
```

The exact build command depends on the chosen shell/tooling. Common options are
framework-specific wrappers, `cargo-ndk`, or an Android Gradle project that
loads a Rust-produced native library. Do not put app-private package names,
keystore paths, headset serials, or release payload paths in Rusty XR.

## Minimum Quest Shell Checklist

A downstream Quest shell should document and test:

- target ABI: `arm64-v8a` / `aarch64-linux-android`
- Android SDK, NDK, build tools, and JDK versions
- OpenXR loader/runtime setup
- activity used for launch
- required permissions and runtime permission flow
- media-pipeline permission flow, if using camera, display capture, audio, or
  Windows streaming
- renderer backend and swapchain setup
- install command
- launch command
- log capture command
- expected success signals, such as process running, focused activity, frame
  loop active, nonzero draw calls, and headset-visible content

Rusty XR can provide public diagnostic models for those signals, but the actual
commands and app identifiers belong to the shell repo.

For media streaming and permission taxonomy, see
[MEDIA_PIPELINE_AND_PERMISSIONS.md](MEDIA_PIPELINE_AND_PERMISSIONS.md).

## Public Example Policy

Future public examples may include a minimal Rust Android/OpenXR shell, but it
must be authored as a clean example for Rusty XR. It should use synthetic data
where possible, avoid private package names and assets, and avoid copying
private rendering behavior.

The first public APK example is `examples/quest-minimal-apk/`. It is a
Rust-native Android smoke test: a Java activity loads a Rust `cdylib`, displays
synthetic Rusty XR contract JSON, and emits basic frame-callback status. It is
not an OpenXR scene and does not touch passthrough camera, MediaProjection,
environment depth, Vulkan texture import, or native compositor layers.

Build it locally with:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-minimal-apk\tools\Build-QuestMinimalApk.ps1
```

The APK is written under `examples/quest-minimal-apk/build/`, which is ignored.
Use the catalog in `examples/quest-minimal-apk/catalog/` to install, launch,
and verify it through Rusty XR Companion Apps.
