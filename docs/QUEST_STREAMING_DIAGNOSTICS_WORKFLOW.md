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
| Broker existing-stream receive/decode | `0.75` and `0.65` | Stable as a receiver/decode isolation lane. When sender projection metadata is supplied to the receiver, this lane can exercise the same projected stereo path used by live broker-camera profiles. |
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

## Makepad Device-Gate Guardrails

For the Makepad comparison lane, keep launch, freshness, and artifact hygiene
strict enough that a run cannot be mistaken for a different APK or a native
passthrough/loading state:

- Build from `examples/makepad-q2q-camera-shell`, pass Makepad Android options
  before `build`/`run`, and use `--key=value` syntax for option values.
- Remove or timestamp the expected APK output before a build, then record the
  fresh APK hash. The current generated APK output root is
  `target/android/makepad-android-apk/`; cleaning an older path is not enough.
  A failed build can leave an older APK in place.
- Extract `lib/arm64-v8a/libmakepad.so` when checking diagnostic strings,
  because APK compression can make direct string search unreliable.
- After clean installs, pregrant ordinary runtime camera and scene permissions
  where Android allows it. Treat MediaProjection as a separate consent flow.
- A loading/preflight screen or `XrUpdate=0` is a failed launch, not a
  passthrough-off success. Native compositor passthrough filling the headset is
  not evidence that the app-owned Makepad panel is visible.
- Use explicit Quest serials whenever more than one Android device is attached.
- Capture a short multi-frame screenshot sequence and record unique hashes for
  visual gates. Byte-identical frames must be annotated before using the
  capture as live-camera evidence.
- Record Makepad launch recovery class. A first launcher attempt that remains
  in loading/preflight must be a failed attempt, even when a second launcher
  start succeeds. Direct generated-XR launch is a fallback/control path and
  should not be reported as launcher success.
- Prefer the Makepad device-gate harness in
  `examples/makepad-q2q-camera-shell/tools/Invoke-MakepadQ2QDeviceGate.ps1`
  so launch recovery class, screenshot freshness, stale-marker counts, and
  fault counters are produced from one consistent path.
- Keep proximity/awake state and CPU/GPU levels passive unless that setting is
  the variable under test. Performance comparisons are not comparable if power
  levels or proximity state change mid-run.
- Keep small hardware-buffer warnings visible as their own counter instead of
  merging them with app-process GPU page-faults or fatal signatures.

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

For refresh-normalized comparisons, run the same profile with explicit display
refresh requests instead of relying on runtime defaults:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-fast075 `
  -Override 'rustyxr.xrDisplayRefreshHz=72.0' `
  -WarmupSeconds 35 `
  -CaptureHzdbScreencap

powershell -ExecutionPolicy Bypass -File .\tools\quest-camera-profile\Invoke-QuestCameraProfileRun.ps1 `
  -Serial <serial> `
  -RuntimeProfile camera-stereo-gpu-composite-fast075 `
  -Override 'rustyxr.xrDisplayRefreshHz=90.0' `
  -WarmupSeconds 35 `
  -CaptureHzdbScreencap
```

Keep `rustyxr.cameraTargetFps` separate from `rustyxr.xrDisplayRefreshHz`.
The former is a Camera2 AE/capture request; the latter is an OpenXR display
refresh request. A camera may deliver roughly 50 Hz while the app still submits
projection frames at 72 Hz or 90 Hz using the latest available camera pair.

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

Use the `fast075` profile only as a performance/compatibility renderer-parity
run. It keeps the same broker capture ID pair, MediaCodec decode, and GPU-import
path; uses square `1280x1280` broker frames and frame-order live stereo pairing;
and swaps the projection draw to the fast public raw-projection shader at render
scale `0.75`. Do not use it for coordinate-alignment gates; use the full-feed
alignment profiles or the raw-stack alignment suite instead.

For longer timing windows, override capture and packet limits in the launch:

```powershell
-Override 'rustyxr.brokerH264CaptureMs=30000,rustyxr.brokerH264MaxPackets=1500'
```

### Broker Existing Stream

Use existing-stream mode to isolate receiver-side costs when another provider
has already exposed `RXYRVID1` H.264 streams. This lane validates incoming
stream receive, MediaCodec decode, hardware-buffer handoff, import, and draw.
It is not a true projected-path comparison unless the receiver has projection
metadata equivalent to the live broker-camera profile.

Existing-stream receiver profiles prefer schema-3 stream-header projection
metadata when the incoming `RXYRVID1` H.264 source provides it. Launch extras
remain as an explicit fallback for older sources and synthetic streams:

- `rustyxr.brokerH264ProjectionMetadataJson`
- `rustyxr.brokerH264LeftProjectionMetadataJson`
- `rustyxr.brokerH264RightProjectionMetadataJson`
- `rustyxr.brokerH264ProjectionMetadataBase64`
- `rustyxr.brokerH264LeftProjectionMetadataBase64`
- `rustyxr.brokerH264RightProjectionMetadataBase64`

For a laptop relay or Quest-to-Quest receiver run, pair those metadata extras
with:

```text
rustyxr.camera=false
rustyxr.brokerH264Consumer=true
rustyxr.brokerH264Stereo=true
rustyxr.brokerH264SourceMode=existing-stream
rustyxr.brokerH264DecodeOutputMode=hardware-buffer
rustyxr.brokerH264StereoPairingMode=frame-order
```

The current renderer-parity target is the `camera-stereo-gpu-composite-fast075`
profile with those existing-stream overrides. It keeps the direct profile's
projection configuration and fast raw-projection shader, while replacing direct
Camera2 capture with the broker/relay/decoder receiver path.

The public online roadmap for extending this into mediated and direct
Quest-to-Quest sessions is
[QUEST_TO_QUEST_ONLINE_STREAMING_ROADMAP.md](QUEST_TO_QUEST_ONLINE_STREAMING_ROADMAP.md).

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
- camera/source capability and timestamp-domain fields when present
- H.264 stream invariants when present, including codec config, SPS/PPS,
  keyframe/sync-frame state, encoder/decoder names, bitrate mode, and
  low-latency request/applied state
- session role, direction, peer id, track id, stream id, and close reasons when
  present
- direct Camera2 acquire, get-buffer, pair-search, and native bridge timings
- broker decoded-frame image wait, get-buffer, and native bridge timings
- final projection status, active tier, shader path, and alignment state
- temporal projection and frame-adoption metrics when present, including camera
  frame age, target/applied projection motion, residual lag, adoption mode,
  pose-clamp angular/linear limits, held/adopted decision state, candidate
  motion p95, held-frame count/duration, invalid UV percentage, edge-fill
  percentage, and optional space-warp counters
- camera consumption metrics when present, including distinct camera frames
  consumed by projected render frames, repeated render frames per camera frame,
  consumed camera-frame Hz, and projection-render Hz
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
- Media stream: record session id, role, direction, track/stream id, eye,
  encoder/decoder names, SPS/PPS/config packet presence, keyframe recovery,
  requested/applied bitrate and latency modes, and close reason.
- Camera/source: record selected source, selected size/fps reason, timestamp
  domain, observed camera timestamp cadence, and selected stream minimum frame
  duration when available.
- Runtime: `VrApi` app time, CPU+GPU time, tear/stale counts, timewarp time,
  CPU percentage, and GPU percentage.
- Temporal projection: camera frame age, stereo pair delta, target projection
  motion, applied projection motion, residual projection motion, visual lag,
  pose-clamp angular/linear limits, frame-adoption mode, held/adopted decision
  state, candidate motion p95, held-frame count/duration, crossfade count,
  invalid-UV percentage, edge-fill percentage, motion-vector max/clamp count,
  and space-warp enabled/skipped frame counts.
- Camera-to-display cadence: requested display refresh, active display refresh,
  `VrApi` target FPS, camera delivery/update Hz, consumed camera-frame Hz,
  projection-render Hz, repeated render frames, and renders per camera frame.

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
- SPS/PPS or codec-config state is missing for a projected H.264 receiver run
- encoder or decoder names are absent from a stream run promoted beyond a
  quick smoke test
- timestamp-nearest pairing is used without explicit timestamp-domain evidence
- the headset entered sleep, an interstitial, or a consent panel during the
  measured window
- existing-stream mode is compared as a projected run without projection
  metadata

Downgrade a run to "transport/decode only" when it receives and decodes frames
correctly but lacks projection metadata or final `gpu-projected` status.

## Current Next Target

The next public implementation slice is one-way LAN Q2Q using the same
schema-3 stream-header projection metadata that passed the single-headset
laptop-loop proof. The direct Camera2 projected path, on-device broker H.264
projected path, and Quest -> laptop relay -> Quest projected path have all
rendered through the XR projection gate with scorecard evidence.

Temporal projection is now opt-in rather than merely planned: no-smoothing
metrics, pose-delta clamp, screen-motion clamp, frame-adoption fields, and
edge/invalid-UV reporting exist in the projected scorecards. The remaining
temporal work is deliberate motion/stress validation for nonzero frame holds,
then depth-aware and optional space-warp profiles. The key scorecard value for
that work remains `applied_projection_motion_px_p95`.

Projected draw attribution still matters: separate projection shader cost,
border/perimeter work, descriptor/import reuse, command recording, and submit
behavior from the already-measured transport, decode, image-acquire,
`HardwareBuffer`, and native bridge stages. Keep that attribution tied to the
acceptance gate in
[CAMERA_STEREO_PROJECTION_PARITY_WORKPLAN.md](CAMERA_STEREO_PROJECTION_PARITY_WORKPLAN.md).
