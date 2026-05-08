# Rusty XR Makepad Q2Q Camera Shell

This is a standalone Makepad-first Quest lane. It exists so Rusty XR can compare
the current custom Android APK workflow with Makepad's generated Android/OpenXR
packaging without replacing either path too early.

The first pass is intentionally a synthetic OpenXR smoke app. It proves the
Makepad Quest APK path, emits a Rusty XR log marker, exercises Makepad's XR
root, and documents the integration gaps before adding camera transport,
stream, or broker behavior.

This example consumes the maintained Makepad fork branch as an app-shell
dependency only. Rusty XR core stays Makepad-independent; the relationship and
fork-patch policy are documented in
[../../docs/MAKEPAD_FORK_RELATIONSHIP.md](../../docs/MAKEPAD_FORK_RELATIONSHIP.md).

## Current Scope

- Uses `cargo-makepad android --variant=quest`.
- Pins Makepad source to `e76019a9f447598ca05697b93e1895a772cbfa8c`.
- Uses `makepad-xr` with a minimal `XrRoot`. Earlier isolation passes tried a
  status panel, a simple cube marker, `XrPermissionsFlow`, and an empty root;
  the current checked-in variant keeps the smallest repro shape.
- Reads its startup marker values through `rusty-xr-runtime-config`, so this
  shell is already attached to a framework-neutral Rusty XR core crate.
- Emits `RUSTY_XR_MAKEPAD_Q2Q_STATUS` on startup.
- Keeps Android SDK, generated Java, generated manifest, native library, and APK
  output under ignored local build folders.
- The current source does not include Makepad's `XrPermissionsFlow`; an earlier
  isolation variant confirmed that the GPU fault also appears when the
  permission flow is removed.

## Build

Install or update the Makepad cargo extension from a Makepad checkout:

```powershell
cargo install --force --path <makepad-checkout>\tools\cargo_makepad
```

Install the Android toolchain into a local cache that is not committed:

```powershell
cargo makepad android --abi=aarch64 --sdk-path <local-makepad-android-sdk> install-toolchain
```

Build the Quest APK from this example directory:

```powershell
cargo makepad android --abi=aarch64 --variant=quest --no-icon --sdk-path <local-makepad-android-sdk> --package-name=io.github.mesmerprism.rustyxr.makepad.q2q --app-label="Rusty XR Makepad Q2Q" build -p rusty-xr-makepad-q2q-camera-shell --release
```

Run on a selected Quest device:

```powershell
cargo makepad android --devices=<quest-serial> --abi=aarch64 --variant=quest --no-icon --sdk-path <local-makepad-android-sdk> --package-name=io.github.mesmerprism.rustyxr.makepad.q2q --app-label="Rusty XR Makepad Q2Q" run -p rusty-xr-makepad-q2q-camera-shell --release
```

The generated APK lands under
`target/android/makepad-android-apk/rusty_xr_makepad_q2q_camera_shell/apk/`.

The Makepad runner starts the generated launcher activity. For direct XR
activity validation, launch the generated XR activity with adb:

```powershell
adb -s <quest-serial> shell am start -n io.github.mesmerprism.rustyxr.makepad.q2q/.MakepadAppXr
```

## Known Affordances And Gaps

- Makepad owns the generated Android manifest, Java activities, OpenXR loader
  packaging, debug signing, install, and launch flow.
- The Quest variant generates both a normal launcher activity and an XR activity.
- The Makepad runner starts the launcher activity. Direct XR smoke validation can
  launch the generated XR activity explicitly.
- Rusty XR runtime profiles are not yet mapped from arbitrary Android intent
  extras into Makepad Rust. This smoke pass reads environment variables for
  desktop/tooling runs and records that Android profile injection still needs an
  adapter.
- The first shared-core bridge is deliberately small: resolved profile values
  pass through `rusty-xr-runtime-config` before logging. Camera metadata,
  stream framing, and scorecard models should be added the same way, through
  core crates first and Makepad adapters second.
- The current lane uses a synthetic status marker only. Camera affordances should
  be added after APK/OpenXR launch is repeatable on device.
- On Windows, the tested Makepad revision required local `cargo-makepad`
  packaging fixes for generated wrapper paths and dependent Rust shared-library
  bundling.
- Current Quest validation reaches the generated XR activity and emits the
  marker, but the device log also reports GPU page faults. Treat that as the
  active Makepad-lane blocker before adding camera transport.
- A control run of Makepad's upstream XR example on the same headset also
  reported GPU page faults, so this is likely not specific to this Rusty XR
  smoke panel.
- The depth-stack comparison showed that Makepad's eager environment-depth path
  differs from the non-Makepad Rusty XR composite stack, but follow-up splits
  still faulted without provider start, per-frame acquire/readback, or depth
  image view creation. Later splits also faulted without passthrough creation,
  without environment-depth provider/swapchain creation, and with zero
  composition layers submitted, without OpenXR color swapchain creation,
  without the OpenXR frame loop, without OpenXR session creation, and without
  Makepad OpenXR instance creation. A same-APK launch of the normal Makepad
  Android activity also reproduced the page-fault class. Later counter-app
  splits moved the active lead into Makepad's Android Vulkan window-swapchain
  recreation after acquire/present reports suboptimal: suppressing that
  recreation stayed clean, the same-toolchain baseline still faulted, and
  waiting the current window-frame fence before recreation stayed clean. That
  wait is now part of the maintained local Makepad fork state and a
  no-diagnostic counter repeat stayed clean. See the repository-level GPU
  investigation note for the current attempt log and the next
  base-Android-renderer isolation steps.

## Validation Status

- `cargo check`: passes for this standalone package.
- Quest APK build: passes with the tested `cargo-makepad` packaging fixes.
- Quest launcher run: installs, starts, emits `RUSTY_XR_MAKEPAD_Q2Q_STATUS`, and
  keeps the app process alive.
- Quest direct XR activity launch: reaches the generated `MakepadAppXr` activity,
  emits the marker, and enters the immersive activity path.
- Known blocker: repeated GPU page fault lines appear in logcat during the
  Makepad XR smoke path. No camera transport or broker integration should be
  judged against this lane until that is isolated or fixed. The same symptom was
  reproduced with Makepad's upstream XR example on the same headset. The
  current isolation target has moved past depth acquire/readback,
  passthrough/composition setup, composition-layer submission, OpenXR color
  swapchain creation, OpenXR session creation, and OpenXR instance creation.
  A same-APK normal-activity launch still reproduced the fault, while fresh
  default Android/GLES-only controls did not. A plain upstream Makepad counter
  app reproduced the fault in the Quest/Vulkan package shape and stayed clean
  when the same Quest-shaped control was forced through GLES. A Quest/Vulkan
  control that skipped Vulkan window draw/present also stayed clean. The active
  lead is Makepad's Android Vulkan suboptimal-triggered swapchain recreation in
  `draw_pass_and_present` on Horizon OS; a targeted wait for the current
  window-frame fence before recreation is now the current local Makepad fork
  candidate patch.
