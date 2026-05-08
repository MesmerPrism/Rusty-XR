# Quest Render Artifact Diagnostics

This workflow is for intermittent Quest OpenXR/Vulkan visual artifacts such as
white pixel pops, short tears, stale frames, or corruption that is visible in
headset even when the on-screen metrics overlay looks broadly healthy.

Keep all screenshots, logcat dumps, traces, captures, and comparison reports in
ignored artifact folders. Do not commit device serials, package identities,
captures, APKs, or app-specific launch commands to this public repo.

## Direct-Device First

Start with headset-native evidence before moving to video capture. Video is
useful for transient symptoms, but it can add encoder/compositor artifacts and
it often makes the runtime metrics harder to attribute.

1. Confirm that the intended app is the focused immersive app:

   ```powershell
   adb devices
   adb shell pidof <package>
   adb shell dumpsys window | Select-String "<package>|mCurrentFocus|mFocusedApp"
   ```

2. Clear logcat, let the scene run in the headset, then dump a focused log:

   ```powershell
   adb logcat -c
   Start-Sleep -Seconds 25
   adb logcat -d -v time > artifacts\quest-render-artifacts\<run>\logcat.txt
   ```

3. Parse `VrApi` or OpenXR runtime rows for the target app PID. Prioritize
   `FPS`, `Tear`, `Stale`, app time, `CPU&GPU`, GPU utilization, `SF`, and
   timewarp/app GPU fields over the visible overlay alone. One-second averages
   can look acceptable while `Tear` or Perfetto GPU spikes still identify the
   artifact path.

4. Capture a single headset frame after the metrics sample:

   ```powershell
   hzdb capture screenshot --method metacam --width 1832 --height 1920 -o artifacts\quest-render-artifacts\<run>\hmd.png
   adb shell screencap -p /sdcard/quest-render-artifact.png
   adb pull /sdcard/quest-render-artifact.png artifacts\quest-render-artifacts\<run>\adb-screencap.png
   ```

   Prefer direct HMD screenshots over cast or video when checking a specific
   pixel artifact. If an OVR Metrics overlay is visible, treat its text and
   graph pixels as an intentional compositor layer and do not classify them as
   app-rendered corruption.

5. Run one-variable A/B captures. Keep every run to one changed parameter and
   record the exact extras in the artifact manifest:

   - render scale, for example `rustyxr.xrRenderScale`
   - app-owned buffer scale or internal render target scale
   - fixed foveation level, for example `rustyxr.xrFixedFoveationLevel`
   - MSAA or sample count
   - particle count, alpha blend path, or diagnostic shader strip
   - display refresh target, when the app exposes one

   A useful A/B result is a lower `Tear` or `Stale` count, lower Perfetto
   `app_gpu_ms` spikes, or a repeatable screenshot difference. If the visual
   symptom changes but `VrApi`/Perfetto metrics do not, preserve the capture and
   move to shader or swapchain diagnostics.

## OVR Metrics Tool

When OVR Metrics Tool is installed, discover the broadcast surface from the
device instead of assuming a fixed tool version:

```powershell
adb shell dumpsys package com.oculus.ovrmonitormetricsservice
```

Useful actions commonly include enabling the advanced GPU preset, enabling
stats, enabling CSV output, taking screenshots, and logging state. The exact
CSV location is tool-version dependent, so treat the logcat `VrApi` rows as the
portable baseline and keep any CSV as an optional artifact.

The overlay is useful, but it is not enough for intermittent artifacts. A
healthy-looking overlay can still hide single-frame GPU spikes, nonzero
`Tear`, stale-frame events, or scheduler gaps that only show up in logcat or
Perfetto.

## Perfetto

Use Perfetto when the artifact is correlated with `Tear`, `Stale`, timewarp
stress, scheduler gaps, render-thread stalls, or GPU spikes. If the Rusty XR
Companion tooling is available, prefer `hzdb` because it can capture traces
without relying on host `trace_processor_shell` being on `PATH`:

```powershell
hzdb perf capture --mode custom `
  --cpu-scheduling `
  --xr-runtime `
  --gpu-render-stage `
  --duration 15000 `
  --app <package> `
  --output artifacts\quest-render-artifacts\<run>\trace

hzdb perf analyze-trace artifacts\quest-render-artifacts\<run>\trace --focus gpu
hzdb perf analyze-trace artifacts\quest-render-artifacts\<run>\trace --focus threads
```

If host analysis tools are missing, keep the `.pftrace` anyway. It can be
opened later through `hzdb perf open`, Perfetto UI, Android Studio, Meta Quest
Developer Hub, or a separately installed `trace_processor_shell`.

When reading the trace, compare:

- app GPU time and timewarp GPU time
- GPU bus busy, shader busy, fragment shading, texture bandwidth, and memory
  stalls
- render or client-request thread runnable gaps
- frame timeline tracks, if present in the selected trace mode
- runtime app switches, interstitials, or shell activity that overlap the
  artifact window

For alpha-heavy particle scenes, a trace that shows high fragment work or GPU
bus spikes is more actionable than a coarse FPS average. Lower render scale,
reduced overdraw, fewer blended particles, a tighter depth/reject path, or a
dedicated performance profile may be the correct immediate mitigation while
the renderer is optimized.

## Particle Renderer Isolation

For dense billboard particles, avoid treating "reduce particles" as the first
root-cause answer. Reducing count is a useful mitigation, but it does not
identify which renderer stage caused the artifact.

Capture a small matrix where each run changes only one variable:

- keep particle and trail count constant while switching the fragment mask,
  such as texture array, procedural, solid diagnostic, or storage-buffer atlas
- keep the fragment mask constant while switching trail visibility or trail
  density
- keep count and fragment mask constant while switching billboard construction,
  such as center-projected billboards versus world-expanded vertices
- keep the renderer path constant while changing render scale, foveation, MSAA,
  or refresh rate

Add app-side timing windows around the likely stages:

- simulation step
- trail update
- merge of source and trail particles
- billboard instance build and optional depth sort
- buffer map/copy or staging upload
- pipeline or texture/atlas ensure path
- draw command recording
- submit

This separates CPU update spikes from fragment shader cost, transparent
overdraw, vertex projection cost, buffer upload cost, and command submission
gaps. If the artifact disappears when the fragment mask becomes a solid or
procedural diagnostic while visible billboard count remains unchanged, the
fragment lookup path is the next target. If the artifact follows the
world-expanded vertex mode while the fragment shader stays constant, inspect
per-vertex projection and near-plane clipping behavior.

For the reusable particle-side patterns, see
[Particle Billboard And Animation Performance](PARTICLE_BILLBOARD_AND_ANIMATION_PERFORMANCE.md).

For camera-composite and live H.264 streaming performance, run the dedicated
[Quest Streaming Diagnostics Workflow](QUEST_STREAMING_DIAGNOSTICS_WORKFLOW.md)
before changing renderer code. That matrix separates synthetic compositor cost,
direct Camera2 projection, broker receive/decode, broker live projection,
render scale, and Java/native handoff timings.

## Simpleperf

Use Simpleperf when the evidence points to CPU hotspots rather than rendering
or compositor timing. Host Simpleperf is best for symbolized reports, but it is
not required for a first capture:

```powershell
adb shell simpleperf record --app <package> -o /data/local/tmp/perf.data -e cpu-clock -f 4000 -g --duration 30
adb pull /data/local/tmp/perf.data artifacts\quest-render-artifacts\<run>\perf.data
```

If host Simpleperf is not on `PATH`, look in the Android NDK, Android Studio,
or managed companion tooling first. If none is available, use the device-side
`adb shell simpleperf report` output as a limited fallback and preserve
`perf.data` for later host analysis.

## Tool Resolution

Resolve Quest diagnostics tools in this order:

1. Explicit user-supplied paths.
2. Rusty XR Companion managed tooling under
   `%LOCALAPPDATA%\RustyXrCompanion\tooling`, including `platform-tools`,
   `hzdb`, `scrcpy`, and `ffmpeg`.
3. Android SDK, Android NDK, Android Studio, or Meta Quest Developer Hub
   installs.
4. Device-side fallbacks such as `adb shell perfetto`, `adb shell simpleperf`,
   `adb shell screencap`, and `dumpsys`.

`RenderDoc`, host `trace_processor_shell`, and host Simpleperf are optional for
the first pass. Their absence should not block capture. The first pass should
still produce logcat, a direct HMD screenshot, a parameter manifest, and, when
needed, a `.pftrace`.

## Rejecting Bad A/B Runs

Reject a run instead of comparing it when:

- the app PID is not the process producing the relevant `VrApi` rows
- the headset is rendering a shell/interstitial instead of the target app
- the app remains resumed but no native frame loop is active
- fixed foveation was requested but the logs show missing or null
  fragment-density images
- screenshot/cast/video artifacts only overlap an intentional metrics overlay
- logcat contains app setup failure, swapchain failure, graphics driver fault,
  or OpenXR session failure before the measured window

Rejected runs are still useful. Keep the artifact folder and record why the run
was rejected so future agents do not compare it as if it were a valid
performance profile.
