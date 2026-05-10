# Quest Streaming Diagnostics Workflow

This workflow covers headset streaming and camera-composite performance
diagnosis for the public Quest examples. It is meant for Quest-to-Quest-style
streaming, broker H.264 receive/decode paths, direct in-app Camera2 projection,
and adjacent headset diagnostics where the goal is to find the dominant cost
instead of only proving that a profile launches.

Keep generated logs, screenshots, traces, encoded payloads, scorecards, APKs,
and device-specific notes under ignored `artifacts/` folders. Do not commit
headset serials, private package names, local paths, raw captures, or
downstream visual-effect details.

## What The Current Evidence Shows

The reusable cost matrix now separates four concerns:

- synthetic compositor/OpenXR submission cost
- direct in-app Camera2 `PRIVATE` hardware-buffer projection
- broker existing-stream receive/decode/hardware-buffer handoff
- broker live Camera2-to-H.264-to-MediaCodec-to-projected-stereo handoff

A representative validation pass on the public composite-layer example showed:

| Lane | Scale | Result |
| --- | --- | --- |
| Synthetic compositor only | `0.75` and `0.65` | Held display cadence with low app and CPU+GPU time. |
| Direct in-app projected Camera2 | `0.75` | Missed display cadence and accumulated high app/CPU+GPU time. |
| Direct in-app projected Camera2 | `0.65` | Recovered display cadence with lower app/CPU+GPU time. |
| Broker existing-stream receive/decode | `0.75` and `0.65` | Stayed stable as a receiver/decode isolation lane, but remained `flat-probe` without projection metadata. |
| Broker live projected stereo | `0.75` | Matched the direct projected path's cadence problem. |
| Broker live projected stereo | `0.65` | Recovered similarly to the direct projected path. |

Follow-up stage timing around Java image acquisition, `HardwareBuffer`
extraction, decoded-frame image waits, and native bridge calls found those
handoff stages averaging below roughly `0.15 ms` in both direct and broker live
projected runs. That puts the current high-cost area after transport/decode and
after Java/native handoff: the next attribution target is the metadata-backed
projected draw/render path.

This conclusion is narrower than saying "the renderer is the bottleneck." The
empty synthetic compositor path is not the expensive path in the measured
matrix. The expensive path is the projected per-eye camera render path that is
shared by direct Camera2 and broker live projected streaming.

The sanitized parity target map for this projected path is tracked in
[CAMERA_STEREO_PROJECTION_PARITY_WORKPLAN.md](CAMERA_STEREO_PROJECTION_PARITY_WORKPLAN.md).
Use that note when comparing the public accepted profile against downstream
custom stereo camera evidence or depth-alignment work.

## Render Scale Semantics

`rustyxr.xrRenderScale` is a linear OpenXR render-target scale. A profile at
`0.65` renders each eye at 65 percent of the baseline linear dimensions; `0.75`
renders at 75 percent.

Pixel area scales with the square of the linear scale:

```text
(0.65 / 0.75)^2 = 0.751...
```

So `0.65` is about 25 percent fewer render-target pixels than `0.75`. Use the
`0.75` profile as a visual-quality baseline and the `0.65` profile as the
current performance comparison profile while projected draw cost is being
optimized.

The public catalog profile
`camera-stereo-gpu-composite-performance-065` explicitly sets
`rustyxr.xrRenderScale=0.65`. Keep profile names and actual catalog values in
sync whenever adding new performance variants.

## Matrix Lanes

Run one variable per lane. Do not mix acquisition, transport, decode,
projection mode, color mode, border mode, foveation, render scale, and stream
metadata in a single comparison.

### Synthetic Compositor

Use the synthetic profile to prove OpenXR lifecycle, swapchain submission, and
baseline compositor cost without Camera2, MediaCodec, broker traffic,
environment depth, MediaProjection, or visual-effect work.

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile synthetic-composite-layer `
  -Override 'rustyxr.xrRenderScale=0.75' `
  -WarmupSeconds 35
```

Repeat with `rustyxr.xrRenderScale=0.65`.

### Direct In-App Projected Camera

Use the direct Camera2 projected profiles to measure the in-process camera
path: Camera2 `PRIVATE` buffers, Vulkan hardware-buffer import, projection
metadata, and the submitted OpenXR projection layer.

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite `
  -WarmupSeconds 35 `
  -CaptureHzdbScreencap

powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-performance-065 `
  -WarmupSeconds 35 `
  -CaptureHzdbScreencap

powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-fast075 `
  -WarmupSeconds 35 `
  -CaptureHzdbScreencap
```

Use `camera-stereo-gpu-composite-fast075` when comparing the direct in-app
Camera2 projection renderer against the Q2Q fast profile. It keeps direct
Camera2 stereo capture and projection metadata, but selects the same fast public
raw-projection shader.

### Broker Live Projected Stream

Use the live broker H.264 profiles to test broker-owned Camera2 capture,
Android platform encoder output, device-local binary H.264 transport,
MediaCodec decode into `ImageReader` `PRIVATE` buffers, native stereo handoff,
and the same projected OpenXR draw path.

The Quest custom stereo profiles pin Camera2 IDs `50` and `51` for the outside
front camera pair. Override `rustyxr.brokerH264LeftCameraId` and
`rustyxr.brokerH264RightCameraId` only when validating a different device
profile or camera topology.

Start the broker APK first, then launch the composite profile:

```powershell
adb shell am start -n com.example.rustyxr.broker/.MainActivity

powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile broker-h264-stereo-live-openxr-projection-probe `
  -WarmupSeconds 35

powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile broker-h264-stereo-live-openxr-projection-scale065-probe `
  -WarmupSeconds 35

powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile broker-h264-stereo-live-openxr-projection-fast075-probe `
  -WarmupSeconds 35
```

Use the `fast075` profile as the renderer-parity run. It keeps the same broker
capture ID pair, MediaCodec decode, and GPU-import path; uses square
`1280x1280` broker frames and frame-order live stereo pairing; and swaps the
projection draw to the fast public raw-projection shader at render scale `0.75`.

For longer timing windows, override capture and packet limits in the launch:

```powershell
-Override 'rustyxr.brokerH264CaptureMs=30000,rustyxr.brokerH264MaxPackets=1500'
```

### Broker Existing Stream

Use existing-stream mode to isolate receiver-side costs when another provider
has already exposed `RXYRVID1` H.264 streams. This lane validates incoming
stream receive, MediaCodec decode, hardware-buffer handoff, import, and draw.
It is not a true projected-path comparison unless the incoming stream carries
projection metadata equivalent to the live broker-camera profile.

## Scorecard Tooling

Use `tools/quest-streaming-diagnostics` to turn one or more run folders into a
single table:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-streaming-diagnostics\Invoke-QuestStreamingScorecard.ps1 `
  -ArtifactDirs .\artifacts\quest-camera-profile-runs\<run-a>,.\artifacts\quest-camera-profile-runs\<run-b>
```

The parser extracts:

- OpenXR observed FPS, frame time, render scale, and GPU import counters
- `VrApi` app, CPU+GPU, timewarp, tear, stale, CPU, and GPU fields
- broker source packet, wire packet, decode, pair, queue-drop, and native
  accepted counts
- direct Camera2 acquire, get-buffer, pair-search, and native bridge timings
- broker decoded-frame image wait, get-buffer, and native bridge timings
- final projection status, active tier, shader path, and alignment state
- optional battery, thermal, process CPU, and meminfo snapshots

Use the generated `scorecard.md` in working notes and PR summaries. Keep
`scorecard.json` and raw artifacts local unless a sanitized public report is
explicitly needed.

## Required Stage Timings

When adding or changing streaming paths, keep these timing windows available:

- Direct Camera2: `acquireLatestImage`, `Image.getHardwareBuffer`, stereo pair
  search, and native stereo bridge.
- Broker decode: decoded `ImageReader.acquireNextImage`, decoded
  `Image.getHardwareBuffer`, per-eye hardware-buffer native bridge, and stereo
  pair native bridge.
- OpenXR: record CPU time, submit CPU time, observed FPS, average frame time,
  render scale, import-cache hits/misses/evictions, and GPU import failures.
- Runtime: `VrApi` app time, CPU+GPU time, tear/stale counts, timewarp time,
  CPU percentage, and GPU percentage.

Sub-millisecond handoff timings do not prove the profile is fast; they only
rule out that stage. Compare them against full frame time and `VrApi` rows.

## Reject Or Downgrade Runs

Reject a run before interpreting the matrix when:

- the active tier is not the intended tier
- `alignedProjection=true` is expected but the logs show `flat-probe`
- left/right decoded counts, native accepted counts, or source packet rates are
  zero in a broker stereo lane
- direct Camera2 falls back to CPU uploads during a GPU-projected run
- GPU import failures or cache evictions continue after warm-up
- the headset entered sleep, an interstitial, or a consent panel during the
  measured window
- existing-stream mode is compared as a projected run without projection
  metadata

Downgrade a run to "transport/decode only" when it receives and decodes frames
correctly but lacks projection metadata or final `gpu-projected` status.

## Current Next Target

The next public implementation slice is deeper projected draw attribution:
separate projection shader cost, border/perimeter work, descriptor/import
reuse, command recording, and submit behavior from the already-measured
transport, decode, image-acquire, `HardwareBuffer`, and native bridge stages.
Keep that attribution tied to the acceptance gate in
[CAMERA_STEREO_PROJECTION_PARITY_WORKPLAN.md](CAMERA_STEREO_PROJECTION_PARITY_WORKPLAN.md).
