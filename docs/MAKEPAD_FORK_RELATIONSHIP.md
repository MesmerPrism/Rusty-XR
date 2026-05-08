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
- Cargo workspace metadata exclusions for standalone CSG leaf crates that are
  outside Makepad's main workspace validation path.
- Ignore rules for local generated Android-control target folders.
- Branch-local notes that document how Rusty XR consumes the fork.

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

## Validation Ladder

Use the current custom APK lane as the diagnostic baseline and the Makepad lane
as the ergonomic app-shell lane. A Makepad fork update should be validated in
this order:

1. Targeted formatting for the Makepad files changed by the branch.
2. Cargo metadata checks for any workspace or manifest changes.
3. `cargo-makepad` check and release build.
4. Quest/Vulkan smoke for the minimal Makepad Android surface.
5. Rusty XR Makepad example launcher and direct-XR smoke.
6. Camera, broker, or stream adapters only after the renderer smoke path is
   stable enough to trust measurements.

The current isolation log is tracked in
[MAKEPAD_XR_GPU_PAGE_FAULT_INVESTIGATION.md](MAKEPAD_XR_GPU_PAGE_FAULT_INVESTIGATION.md),
and the lane-level tradeoff ledger is tracked in
[MAKEPAD_Q2Q_PARALLEL_APPROACH_COMPARISON.md](MAKEPAD_Q2Q_PARALLEL_APPROACH_COMPARISON.md).
