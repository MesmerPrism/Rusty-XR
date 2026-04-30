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
  -CaptureHzdbScreencap
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
```

`-Override key=value` can be repeated to test one variable without adding a
new catalog entry:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-performance-065 `
  -Override rustyxr.cameraTargetFps=0 `
  -Override rustyxr.cameraStereoImageReaderMaxImages=3 `
  -CaptureHzdbScreencap
```

Each run captures battery, power, VR power manager, activity/window state,
logcat, a screen capture, optional `hzdb` screenshots, and a validation report.
The harness uses a timed `hzdb` proximity hold when available; it intentionally
does not send automation/proximity-disable broadcasts before launch.

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
hundreds of OpenXR frames.

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
