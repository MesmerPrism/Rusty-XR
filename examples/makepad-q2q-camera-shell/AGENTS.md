# Rusty XR Makepad Q2Q Agent Notes

This example is the public Makepad-first Quest lane for Rusty XR. It compares
Makepad's generated Android/OpenXR app shell against the existing custom Rusty
XR Quest APK lane while keeping the Rusty XR core crates framework-neutral.

## Required Reading

Before editing this example, read:

- `README.md`
- `../../docs/MAKEPAD_FORK_RELATIONSHIP.md`
- `../../docs/MAKEPAD_ANDROID_BUILD_COMPATIBILITY_PLAN.md`
- `../../docs/MAKEPAD_Q2Q_PARALLEL_APPROACH_COMPARISON.md`
- `../../docs/MAKEPAD_XR_GPU_PAGE_FAULT_INVESTIGATION.md`
- `../../docs/MAKEPAD_STEREO_COMPARISON_ITERATION.md`

## Boundaries

- Keep Rusty XR core crates Makepad-independent.
- Keep Makepad-specific code in this example or optional adapters.
- Do not commit generated Android output, APKs, local SDK paths, logcat dumps,
  device serials, screenshots, captures, or private downstream behavior.
- Use public profile names and public-safe log markers only.
- Treat the custom Rusty XR Quest APK lane as the diagnostic baseline.

## Implementation Ladder

Move in small validated slices:

1. Build the synthetic Makepad OpenXR shell against the maintained Makepad fork.
2. Add synthetic stereo projection markers and scene geometry.
3. Add camera metadata/config logging without opening Camera2.
4. Add Camera2 acquisition diagnostics.
5. Add hardware-buffer import.
6. Add metadata-backed stereo projection parity checks against the custom APK
   `camera-stereo-gpu-composite` profile.
7. Add cadence/performance markers only after active XR presentation is
   confirmed through the launcher path.

Keep future slices separate: active launcher presentation first,
metadata/acquisition second, hardware-buffer import third, projection parity
after those are validated, and performance comparison last.

## Validation

For source changes in this example, run:

```powershell
cargo check --manifest-path examples\makepad-q2q-camera-shell\Cargo.toml
python tools\docs\check_links.py --repo-root .
python tools\schema\check_android_build_manifest.py examples\makepad-q2q-camera-shell\build-manifest.public.json
```

For Android build validation, use `cargo-makepad` from the maintained Makepad
fork and keep the generated `target/` output uncommitted.
