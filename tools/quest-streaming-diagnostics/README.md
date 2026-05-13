# Quest Streaming Diagnostics Tools

These helpers turn Quest camera, broker H.264, and live stereo projection run
artifacts into a single scorecard. They are public-safe: generated logs,
screenshots, APKs, and scorecards stay under ignored `artifacts/` folders.

Use the existing camera-profile harness to create the run folders:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-performance-065 `
  -WarmupSeconds 35 `
  -CaptureHzdbScreencap
```

Then summarize one or more artifacts:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-streaming-diagnostics\Invoke-QuestStreamingScorecard.ps1 `
  -ArtifactDirs .\artifacts\quest-camera-profile-runs\<run-a>,.\artifacts\quest-camera-profile-runs\<run-b>
```

The wrapper writes:

- `scorecard.md`: compact comparison table for notes and pull requests.
- `scorecard.json`: full parsed fields for deeper analysis.
- `scorecard-table.txt`: console table capture.

The parser also works directly:

```powershell
python .\tools\quest-streaming-diagnostics\Parse-QuestStreamingArtifact.py `
  .\artifacts\quest-camera-profile-runs\<run-a> `
  .\artifacts\quest-camera-profile-runs\<run-b> `
  --markdown-out .\artifacts\quest-streaming-diagnostics-scorecards\manual\scorecard.md `
  --json-out .\artifacts\quest-streaming-diagnostics-scorecards\manual\scorecard.json
```

## Scorecard Signals

The parser extracts:

- broker H.264 source, wire, decode, pair, queue-drop, and native-bridge
  counters from `Rusty XR broker H.264 consumer probe` reports.
- direct Camera2 acquire, `HardwareBuffer`, pair-search, and native-bridge
  timings from `Stereo headset camera pair` log lines.
- OpenXR frame cadence, render scale, import-cache counters, and frame timing.
- Makepad direct-XR cadence samples from `RUSTY_XR_MAKEPAD_CADENCE` markers,
  including Makepad `NextFrame` callback rate and paired left/right camera
  texture-update rate. Newer markers also include Makepad `XrUpdate` and
  draw-event rates for cadence-source isolation. Use `VrApi` rows, when
  present, for runtime display cadence.
- Horizon launch-state hints, including volumetric-window launches, immersive
  transition/focus events, loading-complete events, launch-blocked events, and
  permission-controller dialogs. These are guards against treating app-process
  markers as full headset-presentation evidence when the device is still on a
  loading screen.
- final projection status, active tier, shader path, and alignment state.
- temporal projection and frame-adoption fields when log lines expose them,
  including target projection motion, applied projection motion, residual
  projection motion, visual lag, pose-clamp angular/linear limits when present,
  adoption mode, held/adopted decision state, candidate motion p95, held-frame
  count/duration, crossfade count, edge-fill/invalid-UV percentages, and
  optional space-warp counters.
- schema-3 stream-header projection metadata readiness and the session metadata
  source chosen by the receiver. This remains visible even when the long
  consumer JSON report is truncated by logcat.
- camera-to-display consumption fields when log lines expose them, including
  requested/active display refresh, `VrApi` target FPS, consumed camera-frame
  Hz, projection-render Hz, and how many projection frames reuse each camera
  frame.
- `VrApi` app, CPU+GPU, timewarp, tear, stale, CPU, and GPU rows when present.
- optional pre/post battery, thermal, process CPU, and meminfo snapshots when a
  run harness captured them.

## Matrix Shape

A useful live-streaming cost matrix keeps one variable per lane:

1. Synthetic compositor only at `0.75` and `0.65`.
2. Direct in-app Camera2 projected stereo at `0.75` and `0.65`.
3. Broker existing-stream receive/decode at `0.75` and `0.65`.
4. Broker live Camera2-to-H.264-to-decode projected stereo at `0.75` and
   `0.65`.

The existing-stream lane is receiver-side isolation. It is only a projected
path comparison when the incoming stream carries the same projection metadata
as the live broker-camera profile.

## Interpretation Rules

Do not call a run successful just because FPS is high. Check whether the path
was actually `gpu-projected`, whether `alignedProjection=true`, whether
left/right decoded counts and native accepted counts are nonzero, and whether
queue drops or `VrApi` tear/stale counters moved.

For Makepad-generated Android/XR shells, also check the Horizon launch-state
columns. Cadence markers and camera texture updates can be useful while the app
process is alive, but they are not proof that the headset has left the loading
screen or that the runtime is presenting the app as the immersive foreground
client.

Treat `0.65` and `0.75` as linear XR render scales. The `0.65` profile renders
about 25 percent fewer render-target pixels than `0.75`, because pixel area
scales with the square of the linear scale.

When direct Camera2 projected stereo and broker live projected stereo both
miss cadence at `0.75` and both recover at `0.65`, while synthetic compositor
and broker receive/decode remain stable, the next target is projected draw or
shader/render attribution rather than transport, MediaCodec, or Java
hardware-buffer handoff.

Do not infer camera smoothness from display FPS alone. For refresh-normalized
comparisons, run the same profile at explicit display refresh requests and
compare camera delivery/update Hz against consumed camera-frame Hz. A renderer
can submit at 72 Hz or 90 Hz while reusing the latest roughly 50 Hz camera pair;
the scorecard should show both the display cadence and the camera-frame reuse
ratio.
