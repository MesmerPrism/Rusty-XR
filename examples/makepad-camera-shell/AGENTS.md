# Rusty XR Makepad Camera Agent Notes

This example is the public Makepad-first Quest lane for Rusty XR. It compares
Makepad's generated Android/OpenXR app shell against the existing custom Rusty
XR Quest APK lane while keeping the Rusty XR core crates framework-neutral.

## Required Reading

Before editing this example, read:

- `README.md`
- `../../docs/MAKEPAD_FORK_RELATIONSHIP.md`
- `../../docs/MAKEPAD_ANDROID_BUILD_COMPATIBILITY_PLAN.md`
- `../../docs/MAKEPAD_CAMERA_PARALLEL_APPROACH_COMPARISON.md`
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
6. Add broker-managed synthetic H.264 input through the same stream framing and
   Android MediaCodec route used by the public broker examples. A local
   generated texture is useful for renderer smoke only; it is not source parity
   for broker or tri-stack comparisons.
7. Add metadata-backed stereo projection parity checks against the custom APK
   `camera-stereo-gpu-composite` profile.
8. Add cadence/performance markers only after active XR presentation is
   confirmed through the launcher path.

Keep future slices separate: active launcher presentation first,
metadata/acquisition second, hardware-buffer import third, projection parity
after those are validated, and performance comparison last. For broker H.264
runs, also keep source-feed parity, decoded-texture parity, projection-stage
parity, and zero-copy texture performance as separate conclusions.

## Validation

For source changes in this example, run:

```powershell
cargo check --manifest-path examples\makepad-camera-shell\Cargo.toml
cargo test --locked --manifest-path examples\makepad-camera-shell\Cargo.toml
python tools\docs\check_links.py --repo-root .
python tools\schema\check_android_build_manifest.py examples\makepad-camera-shell\build-manifest.public.json
```

This example is intentionally standalone rather than a root-workspace member.
Do not run `cargo check -p rusty-xr-makepad-camera-shell` from the Rusty XR
workspace root; it does not select this package.

For Android build validation, use `cargo-makepad` from the maintained Makepad
fork and keep the generated `target/` output uncommitted.

When editing the maintained Makepad fork for this example, use that fork's
metadata-driven changed-file formatter (`tools/rusty_xr_format.py`). Do not
use `cargo fmt --all` there: Cargo's `--all` formatter route also walks local
path dependencies, including vendored crates outside the Makepad patch surface.

Use `tools/Build-MakepadStereoAlignmentApk.ps1` as the Android/package gate for
this example. Select the SDK for the host that will run `cargo-makepad`:
Windows-host SDKs require `-UseWindowsHost`, while WSL/Linux-host builds must
use a Linux-host Android SDK and NDK prebuilt. When `-MakepadSourceRoot` or
`RUSTY_XR_MAKEPAD_SOURCE_ROOT` is set, the wrapper source-builds
`cargo-makepad` from that checkout and patches the app's Makepad dependency to
the same checkout by default. Use `-NoPatchMakepadXrFromSource` only for an
intentional upstream/pinned-dependency comparison. If a clean WSL/Linux-host
rerun still fails in Makepad packaging while removing a missing bundled font
asset, treat that as a packager-route failure rather than stale staging. Switch
to the Windows-host wrapper lane or state explicitly that Linux-host packaging
itself is under test.

Do not substitute plain `cargo check --target aarch64-linux-android` for the
Makepad Android gate. This app's Android entrypoint is packaged through
Makepad's Android build path, and plain Cargo does not exercise that generated
activity/packager path. The source includes an Android-only binary `main` shim
so direct target checks can compile Android-only Rust modules, but they remain
target-compilation evidence rather than APK/package evidence. For Android-only
Rust edits, an optional `cargo test --target aarch64-linux-android --no-run`
probe may be useful, but label it as partial Android-target Rust compilation
evidence. If it reaches the edited Rust path and then fails only at final test
linking because no target `cc`/NDK linker is configured, do not report "tests
passed" and do not fail the workflow solely on that known linker stop.

The Makepad `cargo_makepad` source used for evidence builds must resolve the
installed SDK platform/build-tools and host executable names. If it still looks
for hardcoded `build-tools/33.0.1/aapt` while the selected Windows SDK contains
a newer `aapt.exe`, do not create a fake SDK shadow or extensionless aliases as
the primary fix. Select or update the maintained Makepad fork/tool so the
packager and wrapper agree on the same SDK profile.

The Makepad revision in this example's `Cargo.lock` controls default Rust
dependency resolution. Keep the locked host test passing and commit intentional
lockfile updates when the maintained fork branch moves. Android APK evidence
builds should pass `-MakepadSourceRoot` so generated Java/native packaging code
and app Rust dependencies both come from the maintained checkout. Do not rely
on an installed `cargo-makepad` binary after editing the fork unless that
installed-binary route is the explicit thing under test.

For broker-synthetic H.264 validation, the guarded device gate should report
stream-header metadata for both eyes, prepared decode state, CPU-YUV texture
readiness, nonzero left/right texture-update cadence, zero decode errors, and
the derived `surface_to_camera`, `screen_to_surface`, and `screen_to_camera`
rows. `max_packets=0` means a live/unbounded broker stream and must not be
clamped to a one-packet stream.
