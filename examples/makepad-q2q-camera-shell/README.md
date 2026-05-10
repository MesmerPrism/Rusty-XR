# Rusty XR Makepad Q2Q Camera Shell

This is a standalone Makepad-first Quest lane. It exists so Rusty XR can compare
the current custom Android APK workflow with Makepad's generated Android/OpenXR
packaging without replacing either path too early.

The first pass was intentionally a synthetic OpenXR smoke app. It proved the
Makepad Quest APK path, emitted Rusty XR log markers, exercised Makepad's XR
root, and documented the integration gaps before adding camera transport,
stream, or broker behavior. The current source now keeps that synthetic stereo
comparison scene and adds a bounded Android NDK Camera2 metadata/acquisition
probe plus a delayed Makepad-owned paired hardware-buffer import probe so
renderer smoke runs can be lined up against the custom APK camera-stereo
baseline before performance parity is measured.

This example consumes the maintained Makepad fork branch as an app-shell
dependency only. Rusty XR core stays Makepad-independent; the relationship and
fork-patch policy are documented in
[../../docs/MAKEPAD_FORK_RELATIONSHIP.md](../../docs/MAKEPAD_FORK_RELATIONSHIP.md).

## Current Scope

- Uses `cargo-makepad android --variant=quest`.
- Uses the maintained Makepad fork branch
  `rusty-xr/android-libstd-packaging`; the current documented branch head is
  `7ea477d0f168`.
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
  records the first hardware-buffer descriptor.
- On Android, after that bounded acquisition window, selects a left/right source
  pair, starts two Makepad-owned camera playbacks with distinct
  `VideoExternal` textures, and emits
  `RUSTY_XR_MAKEPAD_HARDWARE_BUFFER_IMPORT` plus
  `RUSTY_XR_MAKEPAD_STEREO_PROJECTION` as Makepad's Android/Vulkan video
  texture path enumerates, starts, prepares, and accepts both camera hardware
  buffers.
- Emits `RUSTY_XR_MAKEPAD_CADENCE` samples every five seconds, using Makepad
  `NextFrame` events for callback cadence and left/right
  `VideoTextureUpdated` events for Makepad camera texture progression. The
  marker also carries Makepad `XrUpdate` and draw-event rates so a low camera
  texture cadence can be separated from OpenXR loop cadence and app callback
  cadence.
- The maintained Makepad fork also emits public-safe Java activity and native
  bootstrap phase markers for Android activity creation, native handoff,
  EGL/GL setup, Vulkan readiness, and main-loop handoff.
- Keeps Android SDK, generated Java, generated manifest, native library, and APK
  output under ignored local build folders.
- The current source includes Makepad's `XrPermissionsFlow` so normal launcher
  startup can enter active XR presentation. Earlier isolation variants
  confirmed that the original GPU fault also appeared when the permission flow
  was removed.
- The current source imports a Makepad-owned paired camera source after XR
  startup and reports paired-buffer/projection readiness when both textures
  update. The current visible Makepad scene is an app-owned direct CPU-YUV
  camera panel. S59 proves live no-swap YUV color in that panel; S60 is the
  open step that must choose left versus right camera textures per eye before
  parity performance comparison resumes.

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
cargo makepad android --abi=aarch64 --variant=quest --no-icon --sdk-path=<local-makepad-android-sdk> --package-name=<public-example-package> --app-label="Rusty XR Makepad Q2Q" build -p rusty-xr-makepad-q2q-camera-shell --release
```

Run on a selected Quest device:

```powershell
cargo makepad android --devices=<quest-serial> --abi=aarch64 --variant=quest --no-icon --sdk-path=<local-makepad-android-sdk> --package-name=<public-example-package> --app-label="Rusty XR Makepad Q2Q" run -p rusty-xr-makepad-q2q-camera-shell --release
```

Keep the Makepad options before `build` or `run` and use `--key=value` for
paths and package/app values. Before treating an APK as fresh evidence, remove
or timestamp the expected output, record the rebuilt APK hash, and extract
`lib/arm64-v8a/libmakepad.so` for diagnostic string checks. Delete generated
`target/` output before public pushes and public boundary scans.

For unattended camera validation after install, pregrant the declared runtime
camera permissions before the measurement window:

```powershell
adb -s <quest-serial> shell pm grant <public-example-package> android.permission.CAMERA
adb -s <quest-serial> shell pm grant <public-example-package> horizonos.permission.HEADSET_CAMERA
```

MediaProjection is different: it still requires the headset consent flow for
each capture session. A launcher can prepare install, launch, and ordinary
runtime grants, but it should report a blocked MediaProjection consent state
instead of claiming it can bypass the headset prompt.

The generated APK lands under
`target/android/makepad-android-apk/rusty_xr_makepad_q2q_camera_shell/apk/`.

The Makepad runner starts the generated launcher activity. Use the launcher or
normal generated activity for active XR presentation; with `XrPermissionsFlow`
enabled, that path switches into the generated XR activity. Directly launching
the generated XR activity remains useful as a bootstrap/control smoke only,
because the permission flow can switch back to the paired normal activity:

```powershell
adb -s <quest-serial> shell am start -n <public-example-package>/<generated-xr-activity>
```

## Known Affordances And Gaps

- Makepad owns the generated Android manifest, Java activities, OpenXR loader
  packaging, debug signing, install, and launch flow.
- The Quest variant generates both a normal launcher activity and an XR activity.
- The Makepad runner starts the launcher activity. Active XR validation should
  use the launcher path; direct XR launch is now a shell/bootstrap control path
  rather than the primary presentation path.
- Rusty XR runtime profiles are not yet mapped from arbitrary Android intent
  extras into Makepad Rust. This smoke pass reads environment variables for
  desktop/tooling runs and records that Android profile injection still needs an
  adapter.
- The first shared-core bridge is deliberately small: resolved profile values
  pass through `rusty-xr-runtime-config` before logging. Camera metadata,
  stream framing, and scorecard models should be added the same way, through
  core crates first and Makepad adapters second.
- The current lane uses synthetic status/stereo-comparison markers, a bounded
  Camera2 metadata/acquisition probe, and a paired Makepad hardware-buffer
  import/projection-mapping probe.
- The maintained Makepad fork now exposes an explicit native OpenXR passthrough
  toggle plus end-frame diagnostics. After a clean install, grant both Scene
  Access and Headset Camera before classifying XR presentation; otherwise the
  permission flow can remain on the regular/preflight activity while app markers
  still emit. Loading/preflight is a failed launch state, not passthrough-off
  visual success. Earlier gates already proved the app-owned panel can visibly
  render live camera content; the remaining parity gap was world-space
  anchoring rather than per-eye camera/head-space mapping. The current gate
  re-enables native passthrough and restores that known-visible world-space
  panel positive control. S67a stayed active-XR and fault-clean but did not
  visibly recover the panel; artifact review traced the visible S62 target to
  the plain world-space vertex path, the higher panel pose, dark clear color,
  direct per-eye YUV sampling, and the thin pale border. S67a2 restored that
  exact visible-panel control on the maintained fork: launcher and direct-XR
  samples both showed the live app-owned panel, six byte-distinct screenshots,
  clean end-frame markers, and no app-process GPU-fault/fatal/small-buffer
  counters. S67b then disabled native passthrough while preserving that panel:
  launcher and direct-XR samples both showed the live app-owned panel against a
  solid black app background, with opaque OpenXR end-frame submission and
  byte-distinct screenshots. The requested non-black clear color did not appear
  visually, and the small 4x4 hardware-buffer warning class remains a tracked
  counter. The active S68 gate keeps that passthrough-off visible-panel state,
  keeps direct per-eye no-swap limited-BT.601 YUV sampling, and moves the panel
  from the S62 world-space transform to active-eye `draw_pass.camera_inv`
  placement. S68 is a placement/liveness gate, not final projection parity:
  static screenshots can prove the panel is visible and fresh, while headset
  motion is needed to classify whether it follows camera/head space instead of
  staying fixed in world space. The first S68 static gate passed in both
  launcher and direct generated-XR paths: the app-owned live camera panel and
  pale border stayed visible with native passthrough disabled, six-frame
  screenshot sequences were byte-distinct, and app/global GPU-fault and fatal
  counters stayed at zero. The small hardware-buffer warning class and
  environment-depth runtime logs remain tracked separately. Operator headset
  inspection then accepted S68 as clean per-eye panel projection. The remaining
  gaps are narrower: the camera feeds are swapped left/right between eyes, and
  panel alignment is not yet good enough for the intended stereo effect. The
  next gates fix source-eye mapping first, then tune alignment against the
  custom Rusty XR target notes before performance comparison.
- Run-log review after S68 found one important distinction for the next fix:
  S51 already solved horizontal image mirroring with a vertical-only UV flip.
  The S68 issue is source-eye mapping, so S69 should swap the selected
  left/right camera texture sets or active-eye selection without changing the
  accepted no-swap limited-BT.601 color path.
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
  separately through Camera2 acquisition and Makepad hardware-buffer import.
  A later active-presentation gate reached Makepad OpenXR session creation and
  showed that passthrough could start while the runtime rejected environment
  depth provider setup; the maintained fork now treats environment depth as an
  optional feature so a depth-policy failure cannot block projection startup.
  See the repository-level GPU investigation note for the current attempt log.

## Validation Status

- `cargo check`: passes for this standalone package.
- Quest APK build: passes with the maintained Makepad fork branch, including
  dependent Rust shared-library bundling.
- Quest launcher run: installs, starts, emits Java activity, native bootstrap,
  `RUSTY_XR_MAKEPAD_Q2Q_STATUS`, and
  `RUSTY_XR_MAKEPAD_STEREO_COMPARISON` startup markers, switches into active
  XR presentation through `XrPermissionsFlow`, and shows the synthetic stereo
  scene in headset. The S14 launcher pass retained app/`XrUpdate`/draw cadence
  near 90Hz, paired camera texture progression near 50Hz, and no app-process
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
- Makepad hardware-buffer import gate: both Makepad launcher and generated-XR
  short windows enumerate three Makepad camera sources and 66 camera formats,
  select a back-facing 1280x1280 YUV420 source, start the delayed
  `VideoExternal` import path, prepare playback at 1280x1280, and emit
  `makepadVulkanImport=true` on `VideoTextureUpdated`.
- Performance comparison gate: active-presentation comparison reopened after
  S14, but final parity performance is still blocked on visible Makepad camera
  projection. S14 proved launcher-path XR presentation and paired import/cadence
  with the synthetic scene visible. S15 confirmed the custom Rusty XR baseline
  visibly renders proper stereo camera projection, but the current sample was
  performance-degraded. S16 proved marker-level Makepad camera-panel readiness,
  then visual inspection showed the headset still rendering the synthetic
  blue/red debug scene. S17-S20 removed that fallback scene, moved the panel
  under scene-owned `XrNode` routing, and proved a normal Makepad `DrawCube`
  diagnostic panel is visible from the same widget transform. S21 custom-shader
  variants still rendered black, and S22's native `Video` widget surface started
  but did not produce confirmed visible headset-camera playback. S23 added the
  Makepad `Video` headset-camera permission option and removed the unrelated
  custom-shader compile noise, but showed the video widgets must be reset before
  assigning headset-camera sources. S24 added that reset and exposed an Android
  video-cleanup completion gap. S25 tested a Makepad fork cleanup-completion
  patch, but that split was rejected and reverted after it reintroduced the
  Quest app-process GPU page-fault class when the native `Video` widget route
  proceeded. S26 restored the app-fault-clean manual `VideoExternal` route and
  showed the scene-owned cyan diagnostic panel in headset after scene access was
  granted. S27 disabled the solid diagnostic, S28 sampled `sample_video()`
  color directly, and S29 disabled the alignment-guide overlay by default. Use
  the launcher path for Makepad validation, not direct generated-XR launch, and
  verify the paired Makepad textures are the visible XR content before treating
  scorecards as final parity evidence.
- Current comparison target: the accepted source-provenance target run used
  visible stereo Camera2 projection at 72Hz, render scale `0.75`, no stale
  frames, and explicit Quest CPU/GPU level `4` / `4`. Future Makepad parity
  runs must capture the same device performance props, preserve the small
  hardware-buffer warning class separately from GPU-fault counters, and include
  screenshot or headset-cast visual review so a marker-only pass cannot be
  mistaken for projection parity.
- Current cadence probe: rolling `RUSTY_XR_MAKEPAD_CADENCE` samples include
  Makepad `NextFrame`, draw-event, `XrUpdate`, and paired left/right camera
  texture-update counters. The S14 active launcher sample reported
  app/`XrUpdate`/draw cadence near 90Hz and paired texture-update cadence near
  50Hz.
- Current tracked warning: repeated small hardware-buffer lines appear in the
  device logs. They are not counted as GPU page faults, persisted through the
  successful paired import/projection marker gate, and should stay visible in
  the iteration ledger during performance comparison work.
- Current source/build slice: paired Makepad hardware-buffer import and
  projection-mapping markers are validated against the maintained fork branch,
  launcher-path active presentation is validated, the fallback synthetic scene
  has been removed from the visual gate, and the rejected native `Video` widget
  diagnostic is disabled. S32 operator review reclassified the Makepad visual
  evidence as native compositor passthrough plus a low app-owned rectangle, not
  custom projection parity. S33 proved app-owned geometry and improved
  alignment, but the panel still rendered solid split-proof colors rather than
  camera pixels. S34 switched the custom panel to Makepad Y/U/V camera plane
  textures, but device validation still showed proof colors: only one CPU YUV
  stream updated, no paired visual bind completed, and one screenshot exposed an
  unwanted depth-clip/occlusion class where room geometry could cover the app
  panel. S35 keeps panel depth clipping disabled and treats a single updating
  CPU YUV stream as a camera-pixel proof only, not final zero-copy or stereo
  projection parity. The S35 device run did not yet validate that proof because
  the generated XR activity bounced back to the normal activity surface; the
  next slice is an explicit Makepad Android XR activity handoff fix before
  rerunning the YUV proof. The desired visual state for this proof is an
  app-owned panel without runtime depth/environment occlusion; passthrough
  behind the app layer is expected, but real-world geometry covering the panel
  is tracked as a separate depth-clip regression. S36 fixed the activity
  handoff: launcher and direct generated-XR routes now stay in active XR,
  emit YUV-ready/prepared/update and single-stream-proof markers, and remain
  app-fault clean. The visible panel still shows the blue/red proof state
  instead of camera pixels, so S37 focuses on the custom panel's late draw-var
  texture binding/redraw path before paired left/right ownership resumes.
  S37 added an explicit draw-vars-bound marker and kept the same stable/fault
  profile, but the visual result remained blue/red. S38 preferred the actually
  updating YUV stream and removed proof tint, and S39 forced the live
  camera-ready/YUV state onto the active draw area, but both still rendered the
  blue/red proof colors. S40 then made the waiting/default path neutral black
  and set the panel default to camera-ready. The S40 device gate is decisive:
  launcher and direct generated-XR routes stay in active XR, remain
  app-fault clean, keep depth/environment occlusion off, and now render a
  neutral black app-owned panel instead of blue/red. That proves the shader
  edits are active and moves the remaining blocker to Makepad YUV texture
  sampling/content rather than passthrough ambiguity, depth occlusion, activity
  handoff, or stale proof-color state. S41-S45 then proved the camera update
  event carries nonzero CPU-side Y/U/V plane content and kept active OpenXR
  presentation stable through gain-boosted luma, generated R8 Y-slot, generated
  all-slot, and constant shader-bypass controls. S46 proved the visible panel
  executes the edited shader body when the fragment path returns before any
  texture sampling: the panel turned green with the same guide overlay. The
  S47 then visibly sampled a generated R8 texture through the panel's
  `left_tex_y` slot, proving ordinary 2D texture-slot sampling works. The
  S48 disabled that generated replacement and visibly sampled the real Makepad
  camera Y plane through the same direct shader path. S49 removed the earlier
  gain-boost diagnostic and showed a no-gain monochrome camera image in the
  panel. Operator visual review confirmed camera feed visibility and moved the
  next gate to S50: rotate the direct camera-Y sample and center the app-owned
  diagnostic panel for head-forward inspection. S50b passed orientation and
  placement, but operator review identified a remaining left/right mirror. S51
  keeps the centered panel and uses a vertical-only camera-Y flip; the device
  gate retained clean counters and screenshot inspection shows the direct
  camera-Y panel upright, centered, and horizontally corrected. S52 then tried
  direct YUV-to-RGB color conversion in the same early-return shader path. The
  run stayed active-XR and app-fault clean, and CPU-side probes confirmed
  non-empty Y/U/V planes, but the headset view was strongly green/cyan. Per the
  Rusty XR color notes, that result is now treated as sampler/decode-shape
  evidence rather than final color calibration. The S53 gate visualizes the
  sampled Y, U, and V texture slots as separate grayscale bands before channel
  order, range, or matrix tuning resumes. S53 passed that slot-visibility gate:
  the screenshot shows live Y/U/V grayscale bands and clean counters, so the
  remaining color blocker is chroma interpretation. S54 swaps U and V in the
  YUV-to-RGB conversion while keeping the same centered/upright panel; that
  device gate shows real color camera feed instead of the earlier green/cyan
  failure, with the app still active-XR and app-fault clean. S55 keeps that
  plane-order fix and compares swapped-U/V limited/full range plus BT.601/BT.709
  variants in one grid, but its first device screenshot sampled different source
  regions per quadrant. S56 keeps the same formulas and remaps each quadrant to
  the same full camera view; that device gate stayed clean and made the
  comparison fair. The Rusty XR hardware-buffer baseline reports BT.601 narrow
  range for the same camera format, so S57 collapsed the grid into a full-panel
  swapped-U/V limited-BT.601 candidate. S58 removed the center/split diagnostic
  guide and proved the shader edits were active with a border-only overlay, but
  visual review rejected the color claim because skin tones in the panel shifted
  blue/cyan. S59 therefore keeps the border-only panel and vertical flip, samples
  Android YUV_420_888 planes without swapping U/V, and keeps color acceptance
  false until per-eye projection catches up. The S59 run also adds a
  multi-frame screenshot freshness check: future visual gates should record
  whether expected live-camera screenshots are byte-identical before accepting a
  still image as live feed evidence. The first S59 clean install showed why
  launchers should pregrant `horizonos.permission.HEADSET_CAMERA` as well as
  `android.permission.CAMERA`; after that grant, S59 emitted the no-swap marker
  path, stayed app-fault clean, and produced a non-byte-identical screenshot
  sequence.

The current step-by-step implementation ledger is tracked in
[../../docs/MAKEPAD_STEREO_COMPARISON_ITERATION.md](../../docs/MAKEPAD_STEREO_COMPARISON_ITERATION.md).
