# Quest Camera Profile Workflow Tools

These scripts provide a public, reusable workflow for comparing Quest camera
runtime profiles without committing screenshots, APKs, logcat dumps, or private
reference details.

The tools write into `artifacts/quest-camera-profile-runs/`, which is ignored
by the repository.

## Run A Catalog Profile

From the Rusty XR repo root:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-performance-065 `
  -CaptureHzdbScreencap `
  -FreshnessFrames 6 `
  -FreshnessIntervalMs 1000
```

Useful acquisition probes:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-no-ae-target-065 `
  -CaptureHzdbScreencap

powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-reader-max-3-065 `
  -CaptureHzdbScreencap

powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-native-ndk-065 `
  -CaptureHzdbScreencap

powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-native-single-mirror-065 `
  -CaptureHzdbScreencap
```

Use `-Override key=value` to test variables without adding a new catalog entry.
When invoking the script through `powershell -File`, pass multiple overrides as
a comma-separated list in one `-Override` argument:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-performance-065 `
  -Override 'rustyxr.cameraTargetFps=0,rustyxr.cameraStereoImageReaderMaxImages=3' `
  -CaptureHzdbScreencap
```

For color-pipeline A/B runs in one installed APK, prefer the named pipeline
preset shortcut over repeating the full set of feed, sampler, decode, tone, and
OpenXR color-format extras. Projection geometry is a separate axis, so use
`-CameraProjectionMode display-screen-homography` or
`-CameraProjectionMode quad-surface` when a run needs to compare the fullscreen
display homography against the quad-surface coordinate reconstruction:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -RuntimeProfile camera-stereo-gpu-composite-native-single-mirror-065 `
  -CameraPipelinePreset raw-feed-unorm `
  -CameraProjectionMode display-screen-homography `
  -CaptureHzdbScreencap
```

Current public presets are `projected-srgb`, `raw-feed-unorm`,
`projected-unorm`, `raw-feed-srgb`, `shader-decode-unorm`, and
`separate-decode-unorm`. The `raw-projection-fast-unorm` preset keeps the raw
feed and UNORM swapchain but skips the public border/effect shader for
performance isolation. The `raw-projection-invalid-fill-unorm` preset keeps
that fast path and only undims invalid projected-camera fallback pixels for
perimeter/background A/B tests without enabling the full border composite. The
`raw-projection-perimeter-fill-unorm` preset adds a one-sample geometry rim fill
between that invalid-only probe and the full border composite. The
`raw-projection-soft-border-unorm` preset reuses the public border mask with a
single projected inward sample to test lower/perimeter coverage without the full
border composite. The `raw-projection-strong-border-unorm` preset keeps that
single-sample shape but uses a stronger generic border mix for A/B runs where
the soft variant does not move the lower/perimeter region enough. The
`raw-projection-dynamic-border-unorm` preset keeps the strong-border geometry
and adds a cheap dynamic color-feedback term for border color A/B tests. The
`raw-projection-warm-border-unorm` preset keeps the same geometry while using a
luma-preserving warm feedback term for border-color A/B tests. The
`raw-projection-cycling-border-unorm` preset keeps that geometry and adds a
phase-cycled generic color term for testing temporal border-color feedback; use
`-Override 'rustyxr.cameraBorderCycleHz=<rate>'` to adjust the cycle rate in the
same APK. The
`raw-projection-underlay-unorm` preset submits a public
OpenXR passthrough underlay and alpha-blends the raw projection layer, which is
useful when comparing background composition separately from raw camera
sampling. The app-parsed runtime config log reports both the requested preset
and the resolved feed, sampler, decode, projection-effect, tone, and swapchain
settings.
Projection mode remains independent from those presets so border, sampler, and
color modules can be tested against both public geometry mappings in the same
APK.

## Runtime Control Contract

The Quest example keeps A/B modules switchable through the same public runtime
keys whether they come from this PowerShell harness, another launcher, or a
desktop companion process that can send Android intent extras. Keep companion
integrations on these stable keys instead of duplicating shader-specific state:

| Key | Type | Purpose |
| --- | --- | --- |
| `rustyxr.cameraPipelinePreset` | string | Selects the complete feed/sampler/effect/color-format preset, for example `raw-projection-strong-border-unorm`, `raw-projection-warm-border-unorm`, or `raw-projection-cycling-border-unorm`. |
| `rustyxr.cameraProjectionMode` | string | Selects projection geometry independently from the preset: `display-screen-homography` or `quad-surface`. |
| `rustyxr.cameraBorderCycleHz` | float | Adjusts the generic phase-cycled border-color rate used by `raw-projection-cycling-border-unorm`; ignored by static border presets. |
| `rustyxr.xrRenderScale` | float | Controls OpenXR swapchain scale for performance A/B runs. |
| `rustyxr.openxrPassthroughProbe` | string | Keeps native passthrough checks separate from camera projection: `off`, `warmup`, `client`, or `underlay`. |

The app-parsed runtime config log is the authority for whether a switch was
actually applied. It reports the requested preset and the resolved feed,
projection-effect, sampler, import layout, color format, cycle rate, and render
scale. A companion process should record that parsed log line alongside any
operator note before treating a visual observation as valid.

ADB is still needed for install, cold launch, Android permission grants,
screencaps, and log capture. Lighter control channels can reuse the same key
names for operator-driven A/B switching, but they should not invent new setting
names unless the native runtime config parser is updated at the same time.

Native acquisition and OpenXR passthrough-client state are separate axes. To
test runtime passthrough exposure without adding a catalog profile, use
`-Override 'rustyxr.openxrPassthroughProbe=warmup'` or
`-Override 'rustyxr.openxrPassthroughProbe=client'`. Use
`raw-projection-underlay-unorm` when the passthrough layer should be submitted
as a visible underlay. Always compare those runs
against camera-frame progression; passthrough-client state is not a substitute
for live camera delivery.

To test acquisition lifecycle timing without changing projection or color, use
the native profile with a start delay and a log label:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-native-ndk-065 `
  -Override 'rustyxr.cameraStartDelayMs=2500,rustyxr.nativeSourceMode=delayed-synthetic-dual-back' `
  -CaptureHzdbScreencap
```

To isolate renderer/import progression from concurrent stereo camera delivery,
use `camera-stereo-gpu-composite-native-single-mirror-065`. It opens one native
camera source and mirrors the same hardware buffer into both display eyes. You
can select a specific runtime camera ID for a local diagnostic run:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-native-single-mirror-065 `
  -Override 'rustyxr.nativeLeftCameraId=<camera-id>' `
  -CaptureHzdbScreencap
```

Each run captures battery, power, VR power manager, activity/window state,
logcat, a screen capture, optional `hzdb` screenshots, and a validation report.
When `-FreshnessFrames` is greater than one, the harness also captures a short
sequence of screenshots, writes per-frame SHA-256 hashes, and flags duplicate
hash groups in `<runtime-profile>-freshness-summary.json`. Use that freshness
summary for camera/parity runs so a byte-identical frozen screenshot sequence is
not mistaken for live camera feed. The harness uses a timed `hzdb` proximity
hold when available; it intentionally does not send automation/proximity-disable
broadcasts before launch.

## Validate A Run

The run harness invokes validation automatically when the script is present.
You can also run it manually:

```powershell
python .\tools\quest-camera-profile\Validate-QuestCameraRun.py `
  --image .\artifacts\quest-camera-profile-runs\<run>\<label>-hzdb-screencap.png `
  --logcat .\artifacts\quest-camera-profile-runs\<run>\<label>-logcat-tail.txt `
  --label <label> `
  --out .\artifacts\quest-camera-profile-runs\<run>\<label>-validation.json
```

The validator rejects obvious black-camera screenshots, log windows with
screen-off, power-sleep, session-exit, or automation-disable signals, and runs
where `Rusty XR final projection status` shows stale camera frames while
OpenXR keeps rendering. Meta shell sleep-timeout lines are warnings to compare
against the captured power and VR-power snapshots; by themselves they are not
treated as proof that the headset display entered standby. A run can hold
OpenXR display cadence and still be invalid for color comparison if the camera
ROIs are black or the camera frame counter only advances a few frames across
hundreds of OpenXR frames. For native acquisition runs, the validation report
also includes native side-frame counts, camera IDs, timestamp deltas, and the
single-camera mirror flag when those log lines are present.

## Compare Screenshots

Use the image comparison helper for public A/B runs or local downstream
reference comparisons. Keep the images and reports under ignored artifact
folders.

```powershell
python .\tools\quest-camera-profile\Compare-QuestCameraImages.py `
  --reference .\artifacts\quest-camera-profile-runs\<run-a>\<image-a>.png `
  --candidate .\artifacts\quest-camera-profile-runs\<run-b>\<image-b>.png `
  --out-dir .\artifacts\quest-camera-profile-runs\<comparison>
```

The report includes per-ROI mean RGB, luma, saturation, RMSE, and a simple
candidate-to-reference channel fit. The contact sheet marks the sampled ROIs so
the metrics can be checked visually before drawing conclusions.
