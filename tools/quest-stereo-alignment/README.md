# Quest Stereo Alignment Tools

These tools support public-safe comparison of Quest stereo screenshots from the
Rusty XR custom camera path and Makepad-first camera shell.

Use them for evidence, not final acceptance. ADB and HzDB screenshots expose a
raw submitted stereo surface and are useful for regression detection, but
headset review is still required for binocular comfort.

## Analyze A Screenshot

```powershell
python tools\quest-stereo-alignment\Analyze-StereoAlignment.py `
  --candidate artifacts\alignment-run\makepad.png `
  --reference artifacts\alignment-run\rustyxr-target.png `
  --out-dir artifacts\alignment-run\analysis
```

The report separates:

- black-target disparity, when a dark real-world marker is visible in both eyes
- edge-correlation disparity, which is a fallback when no marker is detected
- left/right edge-stripe scores, which catch clamped or repeated edge samples

Optional ROIs are relative to each stereo half when values are between `0` and
`1`, or pixels otherwise:

```powershell
python tools\quest-stereo-alignment\Analyze-StereoAlignment.py `
  --candidate artifacts\alignment-run\makepad.png `
  --reference artifacts\alignment-run\rustyxr-target.png `
  --left-roi 0.18,0.18,0.64,0.64 `
  --right-roi 0.18,0.18,0.64,0.64 `
  --out-dir artifacts\alignment-run\analysis
```

Use candidate/reference-specific ROIs when the two stacks frame the camera
window differently:

```powershell
python tools\quest-stereo-alignment\Analyze-StereoAlignment.py `
  --candidate artifacts\alignment-run\makepad.png `
  --reference artifacts\alignment-run\rustyxr-target.png `
  --candidate-left-roi 0.16,0.18,0.66,0.72 `
  --candidate-right-roi 0.16,0.18,0.66,0.72 `
  --reference-left-roi 0.08,0.12,0.84,0.82 `
  --reference-right-roi 0.08,0.12,0.84,0.82 `
  --max-dark-area-fraction 0.45 `
  --out-dir artifacts\alignment-run\analysis
```

Keep raw screenshots and reports under ignored `artifacts/` folders.

## Compare Homography Stages

When Rusty XR and Makepad logs include homography tokens, compare the coordinate
stages before changing visual warp knobs:

```powershell
python tools\quest-stereo-alignment\Compare-HomographyStages.py `
  --reference-log artifacts\alignment-run\rustyxr.log `
  --candidate-log artifacts\alignment-run\makepad.log `
  --reference-label rusty-xr `
  --candidate-label makepad `
  --width 1680 `
  --height 1760 `
  --out-json artifacts\alignment-run\analysis\homography-stage-summary.json `
  --out-csv artifacts\alignment-run\analysis\homography-stage-samples.csv
```

The comparator keeps these lanes separate:

- `screen_to_surface`
- `surface_to_camera`, when both logs expose it
- `screen_to_camera`
- `surface_to_screen`, when both logs expose it

Use it to identify the first divergent coordinate stage. Screenshot alignment
still needs the visual analyzer and headset review.
