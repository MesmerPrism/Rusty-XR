# Rusty XR Makepad Q2Q Camera Shell

This is a standalone Makepad-first Quest lane. It exists so Rusty XR can compare
the current custom Android APK workflow with Makepad's generated Android/OpenXR
packaging without replacing either path too early.

The first pass was intentionally a synthetic OpenXR smoke app. It proved the
Makepad Quest APK path, emitted Rusty XR log markers, exercised Makepad's XR
root, and documented the integration gaps before adding camera transport,
stream, or broker behavior. The current source now keeps that synthetic stereo
comparison scene and adds a bounded Android NDK Camera2 metadata/acquisition
probe so renderer smoke runs can be lined up against the custom APK
camera-stereo baseline before hardware-buffer import or projection parity is
claimed.

This example consumes the maintained Makepad fork branch as an app-shell
dependency only. Rusty XR core stays Makepad-independent; the relationship and
fork-patch policy are documented in
[../../docs/MAKEPAD_FORK_RELATIONSHIP.md](../../docs/MAKEPAD_FORK_RELATIONSHIP.md).

## Current Scope

- Uses `cargo-makepad android --variant=quest`.
- Uses the maintained Makepad fork branch
  `rusty-xr/android-libstd-packaging`; the current documented branch head is
  `aebeabf32278`.
- Uses `makepad-xr` with a minimal `XrRoot` plus a small synthetic stereo
  comparison scene. Earlier isolation passes tried a status panel, a simple
  cube marker, `XrPermissionsFlow`, and an empty root.
- Reads its startup marker values through `rusty-xr-runtime-config`, so this
  shell is already attached to a framework-neutral Rusty XR core crate.
- Emits `RUSTY_XR_MAKEPAD_Q2Q_STATUS` and
  `RUSTY_XR_MAKEPAD_STEREO_COMPARISON` on startup.
- On Android, emits those startup markers directly through logcat under a
  Rusty XR tag so device smoke tests have a reliable startup signal.
- On Android, starts one bounded NDK Camera2 diagnostic pass that emits
  `RUSTY_XR_MAKEPAD_CAMERA2_METADATA` after enumeration and
  `RUSTY_XR_MAKEPAD_CAMERA2_ACQUISITION` during setup, first-frame, and
  completion. The pass opens one selected `PRIVATE` `AImageReader` source and
  records the first hardware-buffer descriptor without importing the buffer into
  Makepad or Vulkan.
- The maintained Makepad fork also emits public-safe Java activity and native
  bootstrap phase markers for Android activity creation, native handoff,
  EGL/GL setup, Vulkan readiness, and main-loop handoff.
- Keeps Android SDK, generated Java, generated manifest, native library, and APK
  output under ignored local build folders.
- The current source does not include Makepad's `XrPermissionsFlow`; an earlier
  isolation variant confirmed that the GPU fault also appears when the
  permission flow is removed.
- The current source does not import hardware buffers or claim stereo
  projection parity. It reports `pairedLeftRightGpuBuffers=false` and
  `alignedProjection=false` until those slices are implemented.

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
cargo makepad android --abi=aarch64 --variant=quest --no-icon --sdk-path <local-makepad-android-sdk> --package-name=<public-example-package> --app-label="Rusty XR Makepad Q2Q" build -p rusty-xr-makepad-q2q-camera-shell --release
```

Run on a selected Quest device:

```powershell
cargo makepad android --devices=<quest-serial> --abi=aarch64 --variant=quest --no-icon --sdk-path <local-makepad-android-sdk> --package-name=<public-example-package> --app-label="Rusty XR Makepad Q2Q" run -p rusty-xr-makepad-q2q-camera-shell --release
```

The generated APK lands under
`target/android/makepad-android-apk/rusty_xr_makepad_q2q_camera_shell/apk/`.

The Makepad runner starts the generated launcher activity. For direct XR
activity validation, launch the generated XR activity with adb:

```powershell
adb -s <quest-serial> shell am start -n <public-example-package>/<generated-xr-activity>
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
- The current lane uses synthetic status/stereo-comparison markers plus a
  bounded Camera2 metadata/acquisition probe. Hardware-buffer import and
  projection parity remain separate later gates.
- On Windows, the tested Makepad revision required local `cargo-makepad`
  packaging fixes for generated wrapper paths and dependent Rust shared-library
  bundling.
- Earlier Quest validation reached the generated XR activity and emitted the
  marker, but the device log also reported GPU page faults. That fault class was
  later narrowed below this Rusty XR smoke panel.
- A control run of Makepad's upstream XR example on the same headset also
  reported GPU page faults, so the investigation moved into Makepad's
  Android/Vulkan path rather than this example's scene content.
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
  no-diagnostic counter repeat stayed clean. The current synthetic stereo APK
  built against that fork state now passes the launcher and generated-XR
  startup/liveness gate with no app-process GPU page-fault or fatal lines in
  the 90s windows. Repeated small hardware-buffer warnings remain tracked
  separately before Camera2 hardware-buffer import. See the repository-level
  GPU investigation note for the current attempt log.

## Validation Status

- `cargo check`: passes for this standalone package.
- Quest APK build: passes with the maintained Makepad fork branch, including
  dependent Rust shared-library bundling.
- Quest launcher run: installs, starts, emits Java activity, native bootstrap,
  `RUSTY_XR_MAKEPAD_Q2Q_STATUS`, and
  `RUSTY_XR_MAKEPAD_STEREO_COMPARISON` startup markers in a short capture, then
  keeps the app process alive in the longer liveness window with no app-process
  GPU page-fault or fatal lines.
- Quest generated-XR activity launch: emits the same startup marker set in a
  short capture, reaches Vulkan ready / before main loop, and keeps the app
  process alive in the longer liveness window with no app-process GPU page-fault
  or fatal lines.
- Camera2 metadata/acquisition gate: both Makepad launcher and generated-XR
  startup windows enumerate three Camera2 `PRIVATE` sources, select a
  back-facing 1280x1280 source with intrinsics and pose metadata, receive one
  hardware-buffer-backed frame, and complete the bounded probe with
  `status=ok`. The first-frame descriptor reports native format 35, usage
  131840, one layer, stride 1280, and a present buffer id.
- Current tracked warning: repeated small hardware-buffer lines appear in the
  synthetic stereo device logs. They are not counted as GPU page faults, but
  they should stay visible in the iteration ledger before Camera2
  hardware-buffer import is added.
- Current source/build slice: Camera2 metadata/acquisition is validated against
  the maintained fork branch. The next gate is hardware-buffer import, not
  projection parity.

The current step-by-step implementation ledger is tracked in
[../../docs/MAKEPAD_STEREO_COMPARISON_ITERATION.md](../../docs/MAKEPAD_STEREO_COMPARISON_ITERATION.md).
