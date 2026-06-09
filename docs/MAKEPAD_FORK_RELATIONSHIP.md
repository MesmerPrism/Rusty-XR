# Makepad Fork Relationship

Rusty XR and the maintained Makepad fork branch have different jobs.

For the public-facing rationale, see
[Makepad Strategy For Rusty XR](MAKEPAD_STRATEGY.md). This file focuses on the
fork relationship and validation ladder.

Rusty XR remains the framework-neutral public core. It owns contracts,
runtime-profile keys, diagnostic schemas, scorecard helpers, camera/depth data
models, and source examples that can be understood without adopting a specific
application framework.

The Makepad fork branch is a narrow app-shell dependency for the Makepad-first
Quest lane. It exists so the Makepad Android, Quest, OpenXR, Live, and renderer
tooling can be tested against the same Rusty XR contracts without moving those
contracts into Makepad or making the Rusty XR core depend on Makepad.

## Dependency Direction

The dependency direction is intentionally one-way:

```text
Rusty XR core crates
  -> current custom Quest APK examples
  -> Makepad-first Quest example
       -> maintained Makepad fork branch
```

Rusty XR core crates must not depend on Makepad. Makepad-specific code belongs
in the standalone Makepad example, optional adapters, or the maintained Makepad
fork branch.

Repo-family boundary:

- Hostess Makepad shell crates may depend on Makepad.
- Studio Makepad/UI shell crates may depend on Makepad.
- Public Rusty Quest Makepad examples may depend on Makepad.
- Manifold, Manifold packages, Rusty core/CLI crates, descriptor repos, and
  schema/fixture workspaces stay Makepad-free.

Makepad can prove app-shell, rendering, Android, OpenXR, Vulkan, and
generated-package behavior. It does not define Manifold command/session/stream
authority.

## Upstream Alignment Status

The maintained branch is now an upstream-alignment branch as well as a Rusty XR
stress lane. Recent Makepad integration work brought in or reconciled selected
upstream video, Android rendering, text wrapping, Android lifecycle, manifest,
minimum-SDK, tooling-default, and Android App Bundle packaging changes while
preserving the Rusty XR camera-lane diagnostics.

That changes the interpretation of Makepad evidence: a Makepad lane result
should be read as "upstream Makepad plus a narrow Rusty XR camera/XR patch
queue", not as an old isolated fork. Before deeper shader, descriptor, or
external-resource changes, refresh the upstream diff and decide whether the
change belongs in generic Makepad terms or in the Rusty XR adapter layer.

## Fork Patch Queue

The maintained Makepad branch should stay a shallow patch queue. Its current
scope is limited to:

- Selected upstream Android, text, and video fixes needed to keep the tested
  Makepad branch close to current Makepad behavior.
- Android packaging fixes needed for the tested Windows-to-Quest build lane.
- Android manifest, minimum-SDK, tooling-default, and App Bundle packaging
  alignment that keeps the Rusty XR wrapper from becoming a competing packager.
- Android lifecycle shutdown safety needed by mobile, XR, and external-resource
  apps.
- A targeted Android Vulkan window-swapchain frame-fence wait for the
  Quest/Horizon OS suboptimal or out-of-date recreation path.
- Public-safe Android activity and native bootstrap markers used to validate
  launcher/generated-XR startup without publishing raw device logs.
- Quest manifest camera permission and optional camera feature declarations
  needed by public examples that exercise Android NDK Camera2 diagnostics.
- A Makepad `Video` widget camera-permission option used by public examples
  where headset raw-camera sources are gated separately from ordinary app
  cameras.
- An Android-only broker H.264 video source used by public examples to request
  broker-managed left/right streams through the same command and `RXYRVID1`
  framing as the non-Makepad broker examples. The Quest Vulkan/XR path can hand
  decoded MediaCodec output to Makepad as CPU-YUV planes when no GL external
  texture handle exists; broker-camera requests can carry a camera ID and source
  frame-rate target, while zero-copy surface texture transport remains a
  separate performance target.
- A video-source metadata event so public examples can consume broker
  stream-header projection metadata before deriving homography-stage rows.
- Video texture update metadata and throttled frame-flow markers so public
  examples can correlate producer updates, Makepad texture updates, and XR
  frame submission without parsing renderer-specific path names.
- A small `xr_view_id()` shader builtin that exposes Makepad's existing XR
  multiview index to application shaders for per-eye texture selection without
  hardcoding backend-specific symbols.
- Public-safe Android Vulkan video import markers that show whether external
  camera `AHardwareBuffer` frames are imported with a YCbCr resource sampler
  while the current shader path still binds separate sampled-image and sampler
  descriptors.
- An Android OpenXR native-passthrough composition-layer option, currently
  disabled by default on the maintained branch so camera-panel diagnostics can
  distinguish app-owned projection geometry from runtime passthrough imagery.
- Cargo workspace metadata exclusions for standalone CSG leaf crates that are
  outside Makepad's main workspace validation path.
- Ignore rules for local generated Android-control target folders.
- Branch-local notes that document how Rusty XR consumes the fork.

Rejected fork experiments should remain in the public iteration ledger, not in
the active patch queue. The Android video cleanup-completion experiment was
tested in the Rusty XR visual gate and reverted after it reintroduced the Quest
app-process GPU page-fault class when exercised by native `Video` widgets.

Do not use the fork as a place to copy Rusty XR app behavior, generated APKs,
device logs, local SDK caches, package identities, private paths, or
downstream-specific tuning.

## Promotion Policy

Keep every Makepad-side change small enough to review independently. A patch is
a good upstream candidate when it improves Makepad portability, packaging,
workspace metadata, or renderer correctness without relying on Rusty XR-specific
application behavior.

Rusty XR can offer Makepad useful generic pressure in these areas:

- External frame/resource descriptors for Android camera, decoder, and XR media
  paths.
- Video texture lifecycle events covering acquire, update, stale/reuse, and
  release.
- Android lifecycle and packaging hardening for source builds, CI, and mobile
  distribution.
- Shader-resource reflection and descriptor diagnostics for backend parity.
- XR panel DPI, text, scaling, focus, and input evidence.

Rusty XR should pin and document the Makepad revision or branch used for the
Makepad-first lane. When the fork branch changes, update the comparison ledger
and rerun the validation ladder before interpreting camera, streaming, or
renderer measurements from the Makepad lane.

There are two pins to keep in sync. The standalone Makepad example's
`Cargo.lock` pins the Rust crates used by host checks. APK evidence generation
should use `Build-MakepadStereoAlignmentApk.ps1` with `-MakepadSourceRoot` or
`RUSTY_QUEST_MAKEPAD_SOURCE_ROOT` so the `cargo-makepad` tool and app Makepad
dependencies both come from the same maintained checkout. The wrapper requires
that source root by default. Use an installed `cargo-makepad` binary only for
an explicit upstream or portability comparison.

## Validation Ladder

Use the current custom APK lane as the diagnostic baseline and the Makepad lane
as the ergonomic app-shell lane. A Makepad fork update should be validated in
this order:

1. Targeted formatting for the Makepad files changed by the branch, using the
   maintained fork's metadata-driven formatter (`tools/rusty_xr_format.py`)
   rather than `cargo fmt --all`. Cargo's `--all` formatter route walks local
   path dependencies, including vendored crates outside the Makepad patch
   surface.
2. Cargo metadata checks for any workspace or manifest changes.
3. Android Java compile checks when Java bridge or generated template code is
   touched.
4. `cargo-makepad` check and release build.
5. Rebuild the Rusty XR APK through `Build-MakepadStereoAlignmentApk.ps1` with
   `-MakepadSourceRoot` or `RUSTY_QUEST_MAKEPAD_SOURCE_ROOT`; this source-builds
   `cargo-makepad` and source-patches the app Makepad dependency for that
   build.
6. Quest/Vulkan smoke for the minimal Makepad Android surface.
7. Rusty Quest Makepad example launcher and generated-XR startup/liveness smoke,
   with short startup marker capture separated from longer fault-counter
   capture.
8. Camera2 metadata/acquisition through the Rusty XR-owned Android NDK probe.
9. Hardware-buffer import, broker, or stream adapters only after the renderer
   smoke and camera acquisition paths are stable enough to trust measurements.

The current isolation log is tracked in
[MAKEPAD_XR_GPU_PAGE_FAULT_INVESTIGATION.md](MAKEPAD_XR_GPU_PAGE_FAULT_INVESTIGATION.md),
and the lane-level tradeoff ledger is tracked in
[MAKEPAD_CAMERA_PARALLEL_APPROACH_COMPARISON.md](MAKEPAD_CAMERA_PARALLEL_APPROACH_COMPARISON.md).
