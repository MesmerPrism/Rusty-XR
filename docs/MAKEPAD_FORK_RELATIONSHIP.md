# Makepad Fork Relationship

Rusty XR and the maintained Makepad fork branch have different jobs.

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

## Fork Patch Queue

The maintained Makepad branch should stay a shallow patch queue. Its current
scope is limited to:

- Android packaging fixes needed for the tested Windows-to-Quest build lane.
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
  texture handle exists; zero-copy surface texture transport remains a separate
  performance target.
- A video-source metadata event so public examples can consume broker
  stream-header projection metadata before deriving homography-stage rows.
- A small `xr_view_id()` shader builtin that exposes Makepad's existing XR
  multiview index to application shaders for per-eye texture selection without
  hardcoding backend-specific symbols.
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

Rusty XR should pin and document the Makepad revision or branch used for the
Makepad-first lane. When the fork branch changes, update the comparison ledger
and rerun the validation ladder before interpreting camera, streaming, or
renderer measurements from the Makepad lane.

There are two pins to keep in sync. The standalone Makepad example's
`Cargo.lock` pins the Rust crates used by `cargo check`, while APK generation
uses the installed `cargo-makepad` binary. Reinstall `cargo-makepad` from the
same maintained checkout after fork changes that affect generated Java,
packaging, Android platform bridges, or native bootstrap code.

## Validation Ladder

Use the current custom APK lane as the diagnostic baseline and the Makepad lane
as the ergonomic app-shell lane. A Makepad fork update should be validated in
this order:

1. Targeted formatting for the Makepad files changed by the branch.
2. Cargo metadata checks for any workspace or manifest changes.
3. Android Java compile checks when Java bridge or generated template code is
   touched.
4. `cargo-makepad` check and release build.
5. Refresh the installed `cargo-makepad` binary from the maintained fork before
   rebuilding a Rusty XR APK that depends on new Makepad Android code.
6. Quest/Vulkan smoke for the minimal Makepad Android surface.
7. Rusty XR Makepad example launcher and generated-XR startup/liveness smoke,
   with short startup marker capture separated from longer fault-counter
   capture.
8. Camera2 metadata/acquisition through the Rusty XR-owned Android NDK probe.
9. Hardware-buffer import, broker, or stream adapters only after the renderer
   smoke and camera acquisition paths are stable enough to trust measurements.

The current isolation log is tracked in
[MAKEPAD_XR_GPU_PAGE_FAULT_INVESTIGATION.md](MAKEPAD_XR_GPU_PAGE_FAULT_INVESTIGATION.md),
and the lane-level tradeoff ledger is tracked in
[MAKEPAD_Q2Q_PARALLEL_APPROACH_COMPARISON.md](MAKEPAD_Q2Q_PARALLEL_APPROACH_COMPARISON.md).
