# Quest Stereo Alignment Workflow

This workflow is for iterating on public Quest custom stereo camera projection
alignment across the Rusty XR custom APK path and the Makepad-first camera
shell. Keep raw screenshots, logs, APKs, local paths, headset serials, and
private target details out of committed files.

For cross-architecture raw lane naming and the full public verification suite
covering Vulkan/HWB, OpenGL/OES, and Makepad CPU-YUV direct and broker paths,
see
[QUEST_RAW_CAMERA_STACK_ALIGNMENT_WORKFLOW.md](QUEST_RAW_CAMERA_STACK_ALIGNMENT_WORKFLOW.md).

## Source Boundaries

Keep these witnesses separate:

- raw Camera2 frames are the app-owned stereo source
- the Rusty XR custom APK is the current projection-math reference
- the Makepad-first shell is the comparison implementation
- ADB/HzDB screenshots are final submitted-surface stills
- MediaProjection is an optional consented final-display stream, not direct
  access to Meta's protected passthrough layer
- browser physical-screen stimuli provide repeatable targets and timing logs,
  not headset capture by themselves
- headset review remains the final binocular-comfort gate

Do not treat one witness as another. A screenshot can falsify a surface or
edge-stripe regression, but it cannot by itself prove headset-comfort parity.

## Iteration Packet

Every alignment iteration should write one ignored artifact root:

```text
artifacts/quest-stereo-alignment/<iteration-id>/
  manifest.json
  rusty-xr-target/
  makepad-candidate/
  optional-mediaprojection/
  analysis/
  operator-notes.md
```

The manifest should record public-safe values only:

- iteration id and hypothesis
- Rusty XR runtime profile
- Makepad slice marker
- launch route
- refresh rate, render scale, CPU/GPU level, foveation
- capture methods used: ADB, HzDB screencap, HzDB metacam, MediaProjection
- stimulus method, when used: browser sync stimulus, static screen target, or
  synthetic projection-area diagnostic render
- screenshot freshness result
- analyzer report paths
- whether headset review accepted or rejected the candidate

## Capture Order

1. Confirm the headset/resource lease and passive awake/proximity state.
2. Launch the Rusty XR target profile and capture ADB plus HzDB screenshots.
3. Launch the Makepad candidate with the same declared refresh/device state.
4. Capture the same screenshot set and a multi-frame freshness sequence.
5. If MediaProjection consent is already active, record it as an optional
   final-display witness. If consent is blocked, record that state and continue
   with ADB/HzDB evidence.
6. Run `tools/quest-stereo-alignment/Analyze-StereoAlignment.py` against the
   Rusty XR target and Makepad candidate.
7. When both logs contain homography tokens, run
   `tools/quest-stereo-alignment/Compare-HomographyStages.py` before tuning a
   visual warp. Compare `screen_to_surface`, `surface_to_camera`, and
   `screen_to_camera` separately so the first divergent coordinate stage is
   explicit.
8. Update `docs/MAKEPAD_STEREO_COMPARISON_ITERATION.md` with the next slice
   row before changing code again.

For physical laptop-screen runs, serve
`tools/quest-visual-stimulus/run-sync-stimulus.py` and keep its session files
next to the screenshots or display-capture evidence. Do not merge those timing
conclusions with synthetic projection-area footprint results unless the
manifest names both stimulus lanes explicitly.

## Scoring

Score these separately:

- center/marker alignment against the real-world black target
- edge-stripe regression at the left and right window borders
- source-eye mapping and roll/orientation by headset review
- camera freshness and frame uniqueness
- fatal/GPU-fault counters
- performance/cadence only after visual alignment is accepted

Do not select a scalar because it matches one disparity number if it introduces
edge stripes or fails headset review.

When using an app-owned colored border as the witness, record which coordinate
domain produced the border. A marker around a centered in-surface camera window
is not equivalent to a marker around the homography-projected camera footprint.
For Makepad live-camera alignment, prefer markers that explicitly log
`liveCameraWindowDomain=projected_camera_uv`.

## Makepad Slice Rules

Keep the S103 architecture unless a later slice explicitly falsifies it:

- full submitted OpenXR surface remains active
- camera coverage stays inside the shader-owned window
- matte/border are shader-owned, not a resized layer
- horizontal alignment tuning must not sample outside valid camera content and
  repeat edge pixels

Each Makepad slice needs:

- a unique marker such as `s106SafeHorizontalWindowSampling=true`
- stale-marker absence in the fresh native library or logs
- six-frame freshness evidence when possible
- an explicit operator classification: accepted, rejected, or objective-only
