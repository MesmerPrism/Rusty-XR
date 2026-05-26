# Makepad camera Parallel Approach Comparison

This document tracks the Makepad-first Quest lane beside the current Rusty XR
custom Android APK lane. The goal is not to pick a winner yet. The goal is to
make both paths runnable, comparable, and explicit about affordances, costs, and
dependencies.

## Current Decision

Run a Makepad-first fork lane in parallel with the existing custom APK lane,
with both lanes pointing back to the same framework-neutral Rusty XR core. Keep
the public Rusty XR repository MIT-accessible, keep generated artifacts out of
source, and keep Makepad coupling isolated to a standalone example or optional
adapter until the affordance tradeoffs are proven on device.

The ownership boundary between Rusty XR core and the maintained Makepad fork
branch is documented in [MAKEPAD_FORK_RELATIONSHIP.md](MAKEPAD_FORK_RELATIONSHIP.md).
The implementation ledger for the direct stereo-comparison APK path is tracked
in [MAKEPAD_STEREO_COMPARISON_ITERATION.md](MAKEPAD_STEREO_COMPARISON_ITERATION.md).

## Decided For Now

- Use a branch or fork lane for Makepad work instead of replacing the current APK
  examples.
- Pin Makepad dependencies by public git revision in the example package.
- Use `cargo-makepad android --variant=quest` as the Makepad-owned Android,
  OpenXR, signing, install, and run surface.
- Start with a synthetic `makepad-xr` OpenXR smoke shell before adding
  headset-camera transport or broker integration.
- Track generated manifests and APK outputs with `build-manifest.public.json`
  even when the manifest is produced by Makepad instead of kept in source.
- Keep the current custom APK examples intact while the Makepad lane proves its
  device behavior, dependency cost, and update cadence.
- Treat the current custom APK lane as the diagnostic baseline and the Makepad
  lane as the ergonomic app-shell lane, not as two independent products.
- Route Makepad lane profile values through `rusty-xr-runtime-config` first;
  add camera metadata, stream framing, and scorecard contracts through core
  crates before adding Makepad-specific adapters.
- Treat performance comparison as a controlled device-profile test. Current
  source-provenance target evidence shows accepted stereo camera projection at
  72Hz, render scale `0.75`, no stale frames, and roughly half GPU utilization
  under explicit Quest CPU/GPU level `4` / `4`. Future Makepad-vs-custom runs
  must either use the same declared device performance levels or label the
  difference as an intentional variable.

## Still Needs A Decision

- Dependency policy: stay pinned to Makepad `dev` revisions, vendor a tested
  fork, or consume tagged releases when available.
- Tooling patch policy: upstream the Windows shared-library packaging fix, carry
  the generated-wrapper path fix, carry a short-lived Makepad fork, or wait for
  a released Makepad tool update before expecting Windows contributors to build
  this lane.
- Release policy: whether Rusty XR should publish Makepad-built Quest examples,
  keep them as developer-only examples, or expose them only as comparison
  fixtures.
- Runtime profile injection: whether to patch Makepad's Java/Rust bridge for
  arbitrary Android intent extras or keep a Rusty XR adapter outside Makepad.
- Camera path: whether the Makepad lane should use Makepad headset camera APIs,
  Rusty XR camera contracts, or a thin adapter that can report both surfaces.
- Permission timing: whether `XrPermissionsFlow` should request headset camera
  during synthetic smoke startup or whether the first lane should defer camera
  permissions until camera transport is enabled.
- Manifest customization: whether Rusty XR should accept Makepad-generated
  Quest permissions wholesale or add a post-generation manifest patch step.
- CI expectations: whether this lane should be source validation only, desktop
  `cargo check`, Android build, or install-on-Quest smoke testing.
- GPU fault disposition: whether to isolate and fix the current Makepad XR page
  fault before camera work, or temporarily split renderer smoke from permission
  and camera bring-up. The latest splits rule out depth provider/swapchain
  creation, passthrough creation/submission, and composition-layer submission as
  required causes. Later splits also rule out OpenXR color swapchain creation,
  the OpenXR frame loop, OpenXR session creation, OpenXR instance creation, and
  the generated XR activity as required causes. Default Android/GLES-only
  controls did not reproduce the fault in short runs. A plain upstream Makepad
  counter app then reproduced the fault when built with the Quest/Vulkan backend
  and stayed clean when the same Quest-shaped control was forced through GLES.
  A Quest/Vulkan control that skipped Vulkan window draw/present also stayed
  clean. Further splits showed that suppressing suboptimal-triggered swapchain
  recreation stayed clean, a same-toolchain baseline still faulted, and waiting
  either device idle or the current window-frame fence before suboptimal
  recreation stayed clean. The current local Makepad fork state promotes that
  frame-fence wait to a persistent source patch, the no-diagnostic Quest/Vulkan
  counter run stayed clean, and the Rusty XR Makepad shell now passes launcher
  plus generated-XR startup/liveness validation against that fork state,
  including Android NDK Camera2 metadata enumeration, bounded first `PRIVATE`
  buffer acquisition, and direct generated-XR paired Makepad `VideoExternal`
  import/projection-mapping markers. The remaining lead is to harden Makepad's
  Android Vulkan window-swapchain recreation on the acquire/present suboptimal
  path on Horizon OS and keep repeated small hardware-buffer warnings visible
  during paired import/performance comparison work.
- Launch surface: whether automation should use Makepad's launcher `run` path,
  direct adb launch of the generated XR activity, or both as separate checks.

## Affordance Matrix

| Area | Current custom APK lane | Makepad-first lane |
| --- | --- | --- |
| Android manifest | Source-owned manifest per example. | Generated by `cargo-makepad` from CLI options and Quest variant. |
| Package identity | Source manifest and build scripts own it. | CLI `--package-name` owns it at build time. |
| Java shell | Rusty XR examples own Java activity/service code. | Makepad generates `MakepadActivity`, launcher activity, and XR activity. |
| OpenXR loader | Explicit build input or local Android dependency. | Bundled by Makepad Quest variant into the APK. |
| Quest permissions | Explicit source manifest permissions. | Quest variant generates OpenXR, passthrough, scene, anchor, and headset camera permissions. |
| Runtime configuration | Intent extras and Rusty XR config code are directly controlled. | Startup marker values now pass through `rusty-xr-runtime-config`; Android intent extras still need a Makepad adapter. |
| Shared core usage | Uses Rusty XR crates directly for config, diagnostics, camera, broker, and scorecard behavior. | First shared-core bridge uses `rusty-xr-runtime-config`; next bridges should add stream, camera, and scorecard contracts. |
| UI/runtime | Minimal Android shell plus Rust examples. | Full Makepad UI, live/studio ecosystem, OpenXR render loop, and Quest shell. |
| XR scene/root | Rusty XR example owns OpenXR/Vulkan setup directly. | `makepad-xr` provides `XrRoot`, scene nodes, XR permission flow, passthrough hooks, and Makepad draw abstractions. |
| Camera affordance | Camera2 `PRIVATE`, MediaCodec, and projection probes are explicit in example code. | The Makepad shell now has a Rusty XR-owned Android NDK Camera2 metadata/acquisition probe and a direct generated-XR paired Makepad `VideoExternal` import/projection-mapping marker path. Q2Q transport remains a later gate. |
| Accessibility for contributors | Uses common Android concepts but more custom scripts. | One Makepad command path, but contributors must understand Makepad tooling. |
| Maintenance burden | Rusty XR owns Android packaging details. | Rusty XR tracks Makepad tool changes and pins revisions. |
| Debug/install flow | ADB and Rusty XR scripts. | `cargo makepad android run`, with device selection through `--devices`; direct `adb -s` remains useful when multiple devices are connected. |
| Log hygiene | Rusty XR can control public marker formats directly. | Makepad log lines include source-location prefixes, so shared logs need scrubbing before publication. |

## Cost And Dependency Ledger

| Cost | Current custom APK lane | Makepad-first lane |
| --- | --- | --- |
| Required Rust target | `aarch64-linux-android`. | `aarch64-linux-android` through Makepad toolchain install. |
| Android SDK/NDK/JDK | External or locally configured through the shared Rusty XR Android toolchain resolver; Unity `AndroidPlayerRoot` remains supported, and split SDK/NDK/JDK roots are supported when a bundled JDK is missing or broken. | Downloaded and managed by `cargo-makepad` unless an SDK path is supplied. |
| Generated source | Limited to Java classes/dex/build outputs. | Generated manifest, Java activity sources, dex, resources, shared library packaging, and signed APK. |
| Public dependency | Mostly Rusty XR crates and Android tools. | Adds pinned Makepad git dependency and `cargo-makepad` tool while still depending on Rusty XR core crates. |
| XR dependency surface | Custom shell owns OpenXR/Vulkan dependencies directly. | `makepad-xr` pulls in Makepad's XR, rendering, physics/math, asset, and UI dependency graph. |
| Output location | Example `build/` folders. | Makepad `target/android/makepad-android-apk/...` folders. |
| Licensing | Rusty XR MIT plus Android/OpenXR inputs. | Rusty XR MIT plus Makepad MIT OR Apache-2.0 and Android/OpenXR inputs. |
| Current blocker cost | Existing lane already has measured camera/stream diagnostics and now has a portable SDK/NDK/JDK resolver, but its explicit scripts still exercise a different Android shell than Makepad. | The maintained Makepad fork currently needs local Windows packaging patches, the Android Vulkan frame-fence wait, the small `xr_view_id()` shader builtin needed for per-eye texture selection, the `env_cube=false` camera gate, and a native-passthrough layer switch for app-panel visibility diagnostics. The Makepad shell is clean for paired import/projection markers and launcher-path app/fault cadence when Scene Access and Headset Camera are both granted. S49-S59 proved that the app-owned Makepad panel can visibly render live camera content, but S62 showed it was still world-space anchored instead of mapped into the per-eye camera/head space. S65/S66 then showed that loading/preflight states are failed launches, and that the passthrough-off isolation branch can lose visible app output entirely. The active cost is S67: restore the known-visible panel positive control, isolate the passthrough-off regression, then return to head-locked projection and performance comparison. |

## Immediate Validation Ladder

1. Source validation: manifest validator accepts both source-owned and
   Makepad-generated Android manifests.
2. Desktop smoke: the standalone Makepad package passes `cargo check`.
3. Android build: `cargo makepad android --variant=quest build` produces a Quest
   APK and bundles required native shared libraries.
4. Launcher startup smoke: install/run on a selected Quest and confirm Java
   activity, native bootstrap, `RUSTY_XR_MAKEPAD_CAMERA_STATUS`, and
   `RUSTY_XR_MAKEPAD_STEREO_COMPARISON` markers in a short log window.
5. Generated-XR startup smoke: launch the generated XR activity directly and
   confirm the same marker chain, focused immersive activity, and absence of
   native crashes.
6. Liveness/fault window: run a separate longer capture for app-process GPU
   page-fault, fatal, and small hardware-buffer counters.
7. Camera2 metadata/acquisition: confirm source enumeration and one bounded
   hardware-buffer-backed `PRIVATE` frame without importing it into Makepad.
8. Hardware-buffer import: import the acquired buffers into the Makepad/Vulkan
   texture path before attempting projection parity.
9. Visible projection gate: prove that paired left/right camera textures are the
   visible XR content, not a synthetic fallback, debug overlay, protected-content
   capture artifact, native compositor passthrough background, or loading state.
10. Normalized performance batch: run current custom and Makepad samples in the
    same session with matching device performance levels, runtime scale, power
    state, screenshot capture, and scorecard parsing.
11. Target comparison: compare against the accepted source-provenance target
    using the same counters: `VrApi` FPS/`Stale`/`Tear`/`GPU%`, app-process
    fatal/GPU-fault counts, camera progression, CPU upload count, and small
    hardware-buffer warning class.

## Current Device Findings

- Desktop `cargo check` passes for the standalone Makepad package.
- Quest APK build and install pass with the tested Makepad tooling plus local
  Windows packaging fixes for generated wrapper paths and dependent Rust shared
  libraries.
- The generated launcher activity starts and emits Java activity, native
  bootstrap, `RUSTY_XR_MAKEPAD_CAMERA_STATUS`, and
  `RUSTY_XR_MAKEPAD_STEREO_COMPARISON` markers in a short startup capture.
- The generated XR activity can be launched directly, becomes the focused
  headset activity, emits the same marker chain, and reaches Vulkan ready /
  before main loop in a short startup capture.
- Makepad's XR permission flow requests `horizonos.permission.HEADSET_CAMERA`
  even though Q2Q camera transport is not wired yet.
- S62 reached paired, fresh, per-eye Makepad camera sampling with clean
  app-process fault counters, but the panel was still world-space. S63 and S64
  kept the app stable while attempting head-locked placement, yet operator
  review showed no app-owned panel in the headset view; native passthrough was
  still visible and made screenshot interpretation ambiguous. S65 disabled the
  maintained fork's native OpenXR passthrough layer and also showed that clean
  installs need Scene Access grants before the Makepad permission flow enters
  XR presenting. With permissions repaired, passthrough-off active XR emits the
  expected marker chain, but S66 still showed black output instead of the
  non-black clear color or bordered solid panel. That is a regression in the
  passthrough-off isolation branch, not a denial of the earlier visible
  world-space panel proof. S67 therefore restores the known-visible panel
  positive control before projection math resumes.
- Separate 90s launcher and generated-XR liveness captures against the
  maintained fork state showed no app-process GPU page-fault or fatal lines.
  Repeated small hardware-buffer warnings remain tracked separately.
- The Camera2 metadata/acquisition gate passes on both launch paths: each short
  startup capture enumerated three `PRIVATE` sources, selected a back-facing
  1280x1280 source with intrinsics and pose metadata, acquired one
  hardware-buffer-backed frame, and completed with `status=ok`.
- A later source-provenance target correction established a cleaner performance
  target for future comparison: accepted visible stereo camera projection at
  72Hz with no stale frames, render scale `0.75`, and roughly 50% GPU usage when
  the Quest device performance props are explicitly pinned to CPU/GPU level
  `4` / `4`. This does not replace the public custom lane; it sets the current
  comparison envelope and keeps GPU saturation as the leading custom-path
  performance hypothesis.
- A same-session S32 Makepad launcher sample reached the generated XR activity,
  retained paired/projection markers, and stayed app-fault clean, but operator
  visual review reclassified the visible room image as native compositor
  passthrough rather than app-owned camera projection. The next Makepad device
  gate must show the app-owned camera texture panel itself, aligned in the
  headset view, before any performance comparison is meaningful.
- S33 then moved the app-owned panel into view and made it opaque, but the
  visible content remained split proof colors instead of camera pixels. The
  subsequent CPU-YUV camera-pixel proof was useful for color/projection
  debugging and remains the current default. A later Makepad/Vulkan
  `VideoExternal` hardware-buffer diagnostic is available as an opt-in route,
  but stale-frame and color behavior keep it out of the default visual path
  until the Makepad import path is corrected.
- A control run of Makepad's upstream XR example on the same headset reproduced
  the GPU page fault symptom after the Windows tool patches, so the fault is
  likely in the current Makepad/Quest XR stack rather than this Rusty XR smoke
  panel alone.
- The isolation ladder has already ruled out the permission-flow widget,
  Makepad UI surfaces, simple scene content, the environment cube, persistent
  headset-camera permission state, fixed foveation, and a simple app-side
  queue-idle-after-submit patch.
- The source comparison shows Makepad eagerly creates/starts the
  environment-depth provider and attempts per-frame depth acquisition, while the
  non-Makepad Rusty XR composite stack keeps depth mode-gated and reports
  unavailable/error/acquire timing separately. Follow-up splits showed the fault
  still appears when provider start, per-frame acquire/readback, and depth image
  view creation are disabled. Further splits also faulted with passthrough not
  created, with passthrough created but not submitted, with no environment-depth
  provider/swapchain, with zero submitted composition layers, without Makepad's
  OpenXR color swapchain, without the OpenXR frame loop, without OpenXR session
  creation, and without Makepad OpenXR instance creation. A same-APK launch of
  the normal Makepad Android activity also reproduced the page-fault class, but
  fresh default Android/GLES-only controls did not. A plain Makepad counter app
  faulted in the Quest/Vulkan package shape and stayed clean when the same
  Quest-shaped control was forced through GLES. A Quest/Vulkan control that
  returned before Vulkan window draw/present also stayed clean. The next splits
  isolated the suboptimal-present/acquire recreate path: suppressing
  suboptimal-triggered recreation stayed clean, the same-toolchain baseline
  still faulted, and waiting the device or the current window-frame fence before
  recreation stayed clean. The local Makepad fork now carries that frame-fence
  wait as source state and a no-diagnostic Quest/Vulkan counter repeat stayed
  clean. The Rusty XR synthetic stereo shell now also stays clean for the
  current startup/liveness gate against that fork state. The next Makepad tests
  should extend the repeat window, keep small hardware-buffer warnings visible,
  and then start Camera2 metadata/acquisition rather than continuing to focus on
  `makepad-xr` depth, passthrough, or composition-layer ownership.

The active fault-isolation log is tracked in
[MAKEPAD_XR_GPU_PAGE_FAULT_INVESTIGATION.md](MAKEPAD_XR_GPU_PAGE_FAULT_INVESTIGATION.md).
