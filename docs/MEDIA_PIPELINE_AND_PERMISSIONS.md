# Media Pipeline, Windows Streaming, And Permissions

Rusty XR should make media-pipeline integration easier without becoming an
app-specific Quest shell. This document records the public, reusable shape for
Quest media capture, Windows streaming, and permission handling.

## Media Sources

Keep these sources separate:

- Native passthrough layer: compositor-owned passthrough. It is not an app
  texture and should not be documented as sampleable app media.
- Rendered strobe stimulus: app-owned projection-layer flicker or documented
  passthrough-style parameter switching. It is not a camera source and requires
  an explicit safety gate before use.
- Passthrough Camera API / Android Camera2: raw forward-facing camera frames for
  CV/ML and custom app processing. On supported Quest devices this requires
  headset camera permission and camera metadata handling.
- Environment depth: runtime-generated depth texture for occlusion,
  diagnostics, mapping, or readback. It is not a raw depth sensor stream, not a
  raw RGB camera source, and not final-display capture.
- MediaProjection: final display or selected app-window capture. Use this when
  the goal is to represent what the user sees, including app UI and overlays.
  It is not the camera source for a camera-driven custom projection layer.
- App render payloads: app-owned frames, particles, depth summaries, counters,
  or synthetic debug visuals.
- OSC control/sensor datagrams: app-owned live command or sensor packets over
  UDP. They are not media frames and should be mapped into typed app state
  before driving rendering or simulation.

Public Rusty XR crates should model metadata, timestamps, frame descriptors,
runtime counters, control packets, and stream status. The Android app shell
owns the actual MediaProjection, Camera2, OpenXR, Vulkan, encoder, socket, or
ADB integration.

### Native Passthrough Style Shape

`rusty-xr-contracts` includes data-only native passthrough style descriptors
for platform adapters. They cover reconstruction and projected passthrough,
underlay/overlay placement, opacity, edge color, mono color maps,
brightness/contrast/saturation, and color-LUT bindings. See
[META_PASSTHROUGH_LAYER.md](META_PASSTHROUGH_LAYER.md).

These descriptors are not media frames. They do not make the compositor's
passthrough image sampleable, and they do not replace raw camera APIs when an
app needs pixels, timestamps, intrinsics, or camera poses.

### Environment Depth Capture Shape

Public environment-depth tooling should describe what an adapter received from
the runtime, not claim access to private sensors or compositor internals. A
portable depth frame descriptor should include:

- width, height, and byte length
- format, such as `depth_u16le`, `D16_UNORM`, or an adapter-specific public
  enum
- eye index, array-layer index, or stereo layer count
- near/far depth range supplied by the runtime
- per-eye view pose/FOV metadata when available
- frame index and app timestamp
- optional runtime capture timestamp for latency or cadence measurement
- optional confidence payload only when a provider actually supplies or derives
  one

Raw environment-depth samples should not be treated as direct meters by
default. Store the raw format and projection metadata, then reconstruct metric
depth through the adapter's documented near/far and projection path. If a
capture uses a `u16` payload, the metadata must still say whether those values
are normalized depth, millimeters, or another documented encoding.

Runtime depth cadence is not a fixed app-controlled rate. An app can acquire
the latest available depth image during an XR frame, but a runtime may return
no new image. For cadence and latency diagnostics, prefer a runtime-provided
depth capture timestamp over counts from strided exports or display-capture
streams. Count unique runtime capture timestamps separately from acquired
frames because a runtime can return the same depth image across multiple XR
frames.

The public composite-layer example exposes this as the
`environment-depth-diagnostics` runtime profile. That profile keeps app camera
and MediaProjection paths off, acquires at most one environment-depth image per
OpenXR frame, and logs provider support, swapchain size, near/far range,
runtime capture timestamp progression, repeated capture timestamps, observed
acquire/depth rates, average acquire CPU cost, hand-removal state, and explicit
confidence source/payload availability. Its visual mode samples the runtime
stereo depth texture as per-eye grayscale and records the texture transform used
for inspection.

Keep CPU readback, TSDF integration, mesh extraction, and physics/query use
separate from GPU-only depth visualization. Readback or mapping adapters should
be explicitly enabled, throttled by a public policy, and measured separately
from provider start/acquire and fragment-shader visualization cost.

Keep final-display capture and environment-depth capture separate. A
MediaProjection stream can show what the headset presented after app UI and
overlays are composited, but it cannot recover the runtime's raw
environment-depth image or the raw camera frames used by a custom projection
layer.

## Plain Stereo And Feedback Surface Layout

`rusty-xr-contracts` includes public layout contracts for app-owned projected
media surfaces:

- `StereoMediaLayout` selects mono, side-by-side, top/bottom, or separate-eye
  source UV layout.
- `PlainStereoLayer` describes a projected source surface, content fitting mode,
  pose, opacity, and optional border.
- `VisualFeedbackBorder` computes four simple rectangular border segments around
  the fitted content rectangle.
- `FeedbackBorderTuning` carries public border-only tuning values from custom
  stereo camera work.
- `VisualFeedbackLayerTuning` carries public scalar knobs for optional
  screen-composite feedback surfaces and border adapters.
- `StereoLayerPerformanceHints` records adapter performance levers for custom
  stereo and feedback layers.

Use these for raw camera overlays, optional screen-feedback insets, debug
surfaces, or future renderer adapters. They are intentionally layout-only. They
do not implement compositor passthrough, OpenXR composition submission, Camera2
or Passthrough Camera API acquisition, Android hardware-buffer import, Vulkan
external-format sampling, or downstream guide/effect stacks.

The public border baseline is:

| Knob | Public default | Meaning |
| --- | --- | --- |
| `inner_coverage` | `0.18` | Coverage value where border feedback is still fully mixed. |
| `outer_coverage` | `0.82` | Coverage value where border feedback fades out. |
| `feedback_mix` | `1.0` | Maximum feedback contribution for the border. |
| `pullback` | `0.14` | Adapter hint for pulling feedback samples inward from the edge. |
| `swirl_strength` | `0.58` | Adapter hint for border-only recursive motion. |
| `zoom` | `0.22` | Adapter hint for border-only recursive zoom. |
| `edge_boost` | `0.46` | Adapter hint for strengthening edge-derived border response. |
| `rounded_radius` | `(0.50, 0.39)` | Rounded-rectangle border shape radius in normalized content space. |
| `rounded_feather` | `0.12` | Border shape feather. |
| `corner_radius` | `0.08` | Rounded-corner radius. |
| `dark_edge_bleed_inset` | `0.25` | How far dark edge-adjacent feedback may bleed inward. |
| `dark_edge_cutoff` | `0.22` | Luma cutoff for dark edge-adjacent feedback. |
| `dark_edge_feather` | `0.16` | Feather around the dark-edge cutoff. |

The public soft raw-camera border preset is intended for the Quest stereo
camera overlay example when the visible layer remains a direct raw camera
sample and the border should blend gently back into the projection:

| Knob | Public soft raw-camera value | Meaning |
| --- | --- | --- |
| `inner_coverage` | `0.30` | Coverage value where the soft border is still fully mixed. |
| `outer_coverage` | `0.88` | Coverage value where the border contribution fades out. |
| `feedback_mix` | `0.62` | Maximum border blend contribution. |
| `pullback` | `0.16` | Inward camera-sample pullback for the border bleed. |
| `swirl_strength` | `0.18` | Mild public border-only motion hint. |
| `zoom` | `0.12` | Mild public border-only zoom hint. |
| `edge_boost` | `0.50` | Edge-derived border response boost. |
| `rounded_radius` | `(0.47, 0.36)` | Rounded/oval border shape radius in normalized content space. |
| `rounded_feather` | `0.10` | Border shape feather. |
| `corner_radius` | `0.08` | Rounded-corner radius. |
| `dark_edge_bleed_inset` | `0.16` | How far dark edge-adjacent feedback may bleed inward. |
| `dark_edge_cutoff` | `0.25` | Luma cutoff for dark edge-adjacent feedback. |
| `dark_edge_feather` | `0.14` | Feather around the dark-edge cutoff. |

Do not confuse this border tuning with downstream effect layers. The public
repo may carry these border scalar values and layout helpers; it should not
carry private image-processing passes, effect maps, geometric-effect
implementations, scene behavior, or project-specific shader code.

The public composite feedback baseline is:

| Knob | Public default | Meaning |
| --- | --- | --- |
| `feedback_intensity` | `1.10` | Overall recursive feedback intensity. |
| `border_gain` | `1.25` | Border-region feedback gain. |
| `fill_gain` | `0.72` | Interior/fill feedback gain. |
| `feedback_floor` | `0.0` | Minimum source feedback response. |
| `feedback_gain` | `1.45` | Screen-composite feedback amplification. |
| `feedback_warp` | `0.085` | Adapter hint for border-feedback UV perturbation; Rusty XR does not implement the shader. |
| `feedback_zoom` | `0.095` | Adapter hint for recursive zoom on the feedback surface. |
| `organic_noise_scale` | `5.0` | Adapter hint for coarse animated border variation. |
| `organic_noise_amount` | `0.16` | Amount of organic variation. |
| `low_confidence_threshold` | `0.25` | Optional confidence threshold for adapters that have a confidence signal. |
| `low_confidence_softness` | `0.12` | Optional confidence feather. |

### Custom Stereo Layer Performance Levers

Public adapters should expose these levers rather than baking private scene
behavior into the core:

- Prefer opaque GPU-sampled camera buffers for the visible stereo layer. On the
  validated Quest path this meant Android hardware buffers and Vulkan
  external-format / YCbCr-aware sampling. CPU-readable YUV is useful for
  capture/debug, but it should not be the default visible path.
- Track the OpenGL ES `SurfaceTexture` / external-OES lane as a separate
  public implementation candidate for video-backed multilayer effects. Its
  implementation order and comparison gates are documented in
  [OPENGL_OPENXR_MULTILAYER_STACK_PLAN.md](OPENGL_OPENXR_MULTILAYER_STACK_PLAN.md).
- Keep the native compositor passthrough layer, raw camera overlay, and
  optional MediaProjection screen-capture surface separate. They have different
  permissions, timing, and sampling rules.
- In a camera-driven custom composite example, the visible layer source should
  be the headset camera path. MediaProjection should be reserved for
  final-screen capture to Windows so operators and test harnesses can inspect
  what the headset is showing.
- For optional final-screen surfaces sourced from MediaProjection, treat the
  source as monoscopic RGBA unless the app explicitly publishes stereo
  metadata. Sample the full UV range for both eyes and preserve the source
  aspect for the inset.
- Keep environment depth off for ordinary border/stereo feedback. Start/acquire
  depth only for explicit visual-debug, mapping, or readback modes.
- Keep environment cubes and physics workers off unless the current scene
  consumes them.
- Keep the final mobile XR shader interface small. In local Quest validation,
  the compact interface budget was `11` vertex attributes and `7` descriptors;
  larger projection-heavy shaders hit Adreno link limits.
- Reuse descriptor pools for offscreen feedback or guide passes. The first
  multi-pass validation rendered, but descriptor-pool churn was the main
  avoidable cost.
- Coalesce camera-frame-ready and render-loop wakeups so headset camera
  callbacks do not produce unbounded main-loop churn.

The current public Quest composite-layer example exposes three camera path
tiers:

- Tier 0: synthetic OpenXR/Vulkan smoke test, no camera.
- Tier 1: CPU diagnostic flat camera copy. It converts `YUV_420_888` to RGBA,
  preserves the raw camera source aspect, stages below source resolution, and
  copies the same mono image into both eye swapchain array layers. It does not
  claim camera/view alignment.
- Tier 2: intended GPU-projected headset-camera path. The public example now
  has an Android hardware-buffer bridge for paired Camera2 `PRIVATE` frames.
  It reports `activeTier=gpu-projected` only when imported left/right buffers,
  scaled per-eye intrinsics, valid per-eye pose/extrinsics, and the projection
  shader are active together with explicit per-eye camera texture orientation,
  source-eye mapping, and the visual-inspection release gate.

The Java bridge passes public metadata to Rust for diagnostics and projection:
source label, camera ID, delivered size, timestamp, optional sensor
orientation, optional active-array or sensor pixel domains, optional focal
length and principal point, Camera2 lens-pose translation/rotation/reference
when available, requested/active tier labels, transport labels, GPU-buffer
descriptors when available, and explicit flags for missing intrinsics or
missing pose.

That CPU path is useful for bring-up and diagnostics, but it is not the
preferred performance path. The performant shape is:

- Import camera frames as GPU-sampled hardware buffers instead of converting
  `YUV_420_888` to RGBA on CPU.
- If a CPU diagnostic path is kept, throttle it at the ImageReader/acquisition
  boundary; throttling only after CPU conversion still burns the frame budget.
- Keep CPU diagnostic staging previews below source resolution. Full-resolution
  per-eye projection belongs in shader space or a GPU camera texture path.
- Keep projection and per-eye selection in shader space; do not CPU-resample
  into full-eye staging buffers at camera frame rate.
- Keep render/buffer scale configurable for full-view custom projection.
  Keep a lower-scale performance profile available when a device can import
  camera buffers cleanly but the full projected shader path cannot hold display
  cadence at the baseline scale.
- Keep fixed foveation off for edge-sensitive projection paths unless a device
  test proves the fragment-density map does not introduce visible edge or tile
  artifacts.
- Keep the OpenXR submit path pipelined. Submit Vulkan work to an in-flight
  command/fence slot, release the swapchain image, and wait on that fence only
  when the same slot is reused; an immediate post-submit fence wait removes
  useful CPU/GPU overlap.
- Keep final XR shader interfaces small. Move guide work to offscreen passes,
  bake constants where possible, and avoid adding live draw variables casually.
- Reuse descriptor pools and external-buffer imports; repeated allocation per
  camera frame or per guide pass can dominate memory and CPU.
- Size the imported hardware-buffer cache to the camera producer's retained
  buffer pool. If the stereo `ImageReader` can rotate through several buffers
  per eye but the Vulkan cache is smaller than the combined left/right pool,
  the renderer can re-import buffers every frame and lose the intended GPU
  fast path.
- Track OpenXR frame cadence, camera delivery FPS, and import churn as separate
  counters. A renderer can hold display cadence while the Camera2 producer
  delivers below display cadence, and stale-camera-frame diagnosis needs both
  numbers in the same run.
- Coalesce camera-frame wakeups so callbacks cannot queue unbounded render-loop
  messages.
- For separate left/right Camera2 streams, treat timestamp matching as a soft
  quality target. Publish the latest available pair to avoid starving the
  renderer, close older queued buffers promptly, and log pair deltas plus
  over-target counts for validation.
- Size opaque stereo `ImageReader` queues for the imported-buffer lifetime, not
  just the Java pending-pair queue. Vulkan memory and descriptor caches can
  retain a few `AHardwareBuffer` images after Java has closed its `Image`, so a
  too-small `maxImages` value can starve Camera2 and produce stale frames.
- Keep acquisition implementations modular. A Java Camera2 concurrent-stereo
  path is useful as a public Vulkan example, but lower-level/native
  hardware-buffer readers can have different queue, timestamp, and AE-policy
  behavior. Compare those as explicit modules or runtime profiles instead of
  silently changing the camera path under an existing profile.
- Log camera topology before treating an acquisition path as equivalent. Native
  probes should record all visible NDK camera IDs, logical multi-camera
  capability, physical camera IDs, sensor sync type, `PRIVATE` sizes, and the
  selected synthetic or explicit stereo side source. When testing lifecycle
  timing, prefer a launch-extra delay such as `rustyxr.cameraStartDelayMs`
  instead of changing projection, border, or shader code in the same run.
- Keep OpenXR passthrough-client state separate from raw camera acquisition.
  Optional passthrough features and scene permissions can affect whether a
  Quest runtime exposes `XR_FB_passthrough`, but creating a passthrough
  client/layer is not the same thing as delivering fresh raw camera buffers.
  Always measure camera-frame progression and runtime camera-compute load when
  this probe is enabled.
- Keep environment depth, environment cubes, TSDF/readback, physics workers,
  and MediaProjection disabled unless the active runtime mode consumes them.

### Temporal Projection Smoothing

Temporal smoothing for the opaque custom stereo camera layer belongs between
camera/decode/import and the final per-eye projection shader. It should not
change which source is authoritative:

- raw Camera2/Passthrough Camera frames remain the visible camera source
- MediaProjection remains final-display inspection only
- environment depth is an optional depth witness
- native passthrough remains compositor-owned and unsampled by the app

The first public contract slice records `TemporalProjectionPolicy`,
`ProjectionTargetState`, `VisualProjectionState`, `StereoCameraFramePair`, and
`TemporalProjectionMetrics`. These are data contracts for adapters and
scorecards. They do not enable smoothing by themselves.

The implementation order is metrics-only first, then pose-delta clamp,
screen-motion clamp, frame adoption smoothing, edge handling, depth-aware
fallbacks, and optional space-warp probes. Keep all temporal profiles explicit
and leave the current no-smoothing profile available for comparison.

For a successful temporal profile, scorecards should prove that
`applied_projection_motion_px_p95` is bounded by policy while target motion and
residual lag remain visible in the metrics.

The same temporal policy should apply to direct Camera2 projection,
broker-decoded existing-stream projection, and Makepad stereo projection. Meta
native passthrough may be used as a compositor-owned visual reference, but it
is not the source texture and should not be treated as a shortcut around
app-owned projection smoothing.

Production adapters should use GPU-sampled camera buffers, Android
hardware-buffer import, external-format or YCbCr-aware Vulkan sampling, and a
small projection shader. A Tier 2 launch must not be reported as aligned unless
the GPU texture, intrinsics, source camera pose/extrinsics, explicit per-eye
camera texture orientation, source-eye mapping, and visual inspection gate are
all active.

On the current opaque Camera2 hardware-buffer path, the Vulkan sampler-YCbCr
conversion presents normalized RGB at the projection shader boundary. Keep
manual BT.601-style channel decode as a diagnostic switch for devices that
expose channel-packed values, but do not stack that decode on top of the
external sampler's conversion. Log the external format, Vulkan format,
suggested YCbCr model/range, component mapping, and active camera color mode
with each import test so RGB-sampler and shader-decode paths can be compared
without changing projection or border behavior.

Current public Quest profile findings:

- The combined immutable-sampler `external-rgb` path is the usable baseline for
  the projected stereo example on the tested runtime.
- The combined-sampler `external-cr-y-cb-bt601-narrow` path can be strongly
  green/discolored when the external sampler already presents RGB-like values.
  Keep it as a diagnostic for raw-channel exposure, not as a default public
  profile.
- Render cadence and camera cadence must be measured separately. A profile can
  submit OpenXR frames at `72 Hz` while Camera2 delivers paired camera buffers
  below display cadence.
- The simple Java Camera2 acquisition probes did not close the stale-frame gap
  on the tested runtime: no explicit AE FPS request, a smaller stereo
  `ImageReader` max-image count, a wider pair window, and a `1280x960`
  separate-eye size all still produced stale concurrent-stereo progression.
- The live-bounded broker H.264 stereo profile proves that paired broker
  Camera2 streams can be encoded, decoded, paired, imported as hardware buffers,
  and submitted through the `gpu-projected` OpenXR stereo path. It remains a
  diagnostic path until stream lifetime, timestamp-based pair/drop policy,
  remote-device validation, and release-grade projected performance are solved.
- In current projected diagnostics, lowering `rustyxr.xrRenderScale` from
  `0.75` to `0.65` is the useful performance comparison knob. Keep the `0.75`
  profile as the visual-quality baseline and use the `0.65` profile to isolate
  transport/decode/pairing from projected render cost.
- The direct-versus-broker streaming cost matrix narrows the current
  bottleneck: synthetic compositor-only profiles are stable, broker
  existing-stream receive/decode is stable as a receiver/decode isolation lane,
  and both direct Camera2 projected stereo and broker live projected stereo show
  the same render-scale-sensitive behavior. When sender projection metadata is
  supplied, existing-stream can exercise the same metadata-backed projected
  path; without that metadata, treat it as transport/decode evidence only.
  Stage timing puts Java acquire/wait, `HardwareBuffer` extraction, and native
  bridge calls below roughly sub-millisecond scale, so the next optimization
  target is the shared metadata-backed projected draw/render path.
- A mono `PRIVATE` GPU-buffer probe at `1280x960` continued to deliver live
  frames, so the next useful comparison is Java Camera2 concurrent stereo
  against a lower-level/native hardware-buffer reader module.
- The first native-reader probe reproduced the intended lower-level ownership
  shape with `ACamera*`, `AImageReader` `PRIVATE` GPU-sampled buffers,
  acquire-latest handling, immediate `AImage` deletion after taking an
  `AHardwareBuffer` reference, and latest-pair publication. Direct side-camera
  sessions still showed stale progression on the tested runtime, so the next
  acquisition comparison is source/session shape and timestamp behavior, not
  another Java queue-depth tweak.
- The native single-camera mirror probe is a useful isolation module: it opens
  one native camera source and mirrors the same acquired hardware buffer into
  both display eyes. On one tested runtime, one side-camera ID delivered live
  frames in this mode while the other side-camera ID remained sparse even when
  opened alone. Treat exact IDs as run-local diagnostics. A successful mirror
  run indicates that the renderer/import path can keep up, while a failing
  stereo-native run points back at effective source/provider policy or
  concurrent side-camera session behavior.
- The native probe logs source topology and supports delayed camera startup.
  Treat those as acquisition-lifecycle diagnostics: they should explain whether
  two runs are using the same effective stereo source before color or stale
  frame conclusions are compared.
- Optional `XR_FB_passthrough` client/layer probing exposed a real runtime
  capability boundary, but did not by itself fix camera progression. Keep
  `client` and `warmup` as diagnostics for extension exposure and runtime
  state, not as default color/performance profiles.

### Autonomous Quest Camera Profile Runs

`tools/quest-camera-profile` contains generic public helpers for repeatable
Quest camera profile runs:

- `Invoke-QuestCameraProfileRun.ps1` launches a catalog runtime profile,
  captures power/wake/VR-power snapshots, records logcat and screenshots, and
  writes a run manifest under ignored `artifacts/`.
- `Validate-QuestCameraRun.py` rejects black-camera screenshots and log
  windows that contain sleep, screen-off, session-exit, or automation-disable
  signals. Native acquisition reports also include side-frame counts, selected
  camera IDs, timestamp deltas, and the single-camera mirror flag when those
  log lines are present.
- `Compare-QuestCameraImages.py` compares two local screenshots by ROIs and
  writes RGB/luma/saturation metrics plus a contact sheet.

Use these tools to keep profile comparisons systematic. A useful run records
the active runtime profile, camera color mode, requested/applied Camera2 AE FPS
range, observed camera-pair FPS, CPU upload count, hardware-buffer import cache
state, OpenXR frame cadence, render scale, foveation state, power state, and
VR-power/proximity state. Reject the run before comparing color if the headset
entered standby or the camera ROIs are black.

For eye alignment, scale camera intrinsics from the source metadata pixel
domain into the delivered per-eye stream domain before projection. A
side-by-side stream halves the delivered width per eye, a top-bottom stream
halves the delivered height per eye, and separate-eye streams use the full
delivered image per eye. A real aligned renderer should, for each eye, compute
the world point on the head-anchored camera surface, transform that point into
the active source camera frame when pose metadata exists, project through the
scaled intrinsics, and sample the camera texture. Camera2
`LENS_POSE_REFERENCE_GYROSCOPE` must first be resolved through the current
head/tracking basis; it is not already OpenXR eye space. If intrinsics or pose
metadata is missing, the renderer should fall back to a visible/logged
diagnostic flat copy. Mono Camera2 fallback should be labeled as mono fallback,
not true stereo alignment. For opaque GPU camera buffers, apply any public
per-eye `CameraTextureTransform` after projection UV calculation and before
sampling; this texture transform is separate from Camera2 sensor orientation.

### Streaming Cost Scorecards

`tools/quest-streaming-diagnostics` contains public helpers for comparing
direct camera projection, broker H.264 live projection, existing-stream
receive/decode, and synthetic compositor baselines. The parser reads the
artifacts produced by the profile harness and writes `scorecard.md` plus
`scorecard.json` under ignored `artifacts/` folders. See
[QUEST_STREAMING_DIAGNOSTICS_WORKFLOW.md](QUEST_STREAMING_DIAGNOSTICS_WORKFLOW.md)
for the matrix lanes, reject rules, and current findings.

### Low-Latency Transport Control/Data Split

Low-latency media work should keep session control, high-rate payload bytes,
and telemetry separate. The broker can negotiate capabilities, stream
descriptors, security policy, and operator approval over JSON commands/events,
while encoded media, raw luma, or future packetized diagnostic payloads stay on
binary endpoints.

This keeps the public broker useful for diagnostics without making it a
vendor-SDK wrapper or a production camera-streaming service. The active XR
client still owns decode-to-texture, hardware-buffer import, eye-view/FOV use,
frame pairing, projected rendering, and OpenXR submission. Timing reports
should flow back through telemetry so scorecards can distinguish transport,
decode, import, projected render, and submit costs.

External low-latency SDKs should be treated as optional sidecars or comparison
lanes. Public Rusty XR may record their version/license/path metadata and
compare measurements, but must not bundle, link, or copy their SDK code,
headers, binaries, or packet formats into the MIT core.

For the Q2Q streaming path, keep the current Android MediaCodec and
`RXYRVID1` H.264 diagnostic format through laptop-loop, LAN, and first relay
milestones. Add session ids, roles, timestamp domains, camera/source
capabilities, H.264 config/keyframe invariants, bounded relay counters, and
session-native projection metadata before replacing the transport. WebRTC and
WebTransport should be adapter lanes after that evidence exists, not the first
implementation dependency.

## Windows Streaming Shape

Use one of these app-shell patterns:

- Device to Windows socket: the Quest app connects to a Windows receiver through
  `adb reverse tcp:<port> tcp:<port>` during development.
- Windows to device socket: the Quest app hosts a local server and Windows
  connects through `adb forward tcp:<port> tcp:<port>`.
- App-private file export: the Quest app writes frames into its app-private
  files directory and a Windows tool pulls files through `adb shell run-as`.
- Network transport: the Quest app sends frames over LAN using an app-owned
  protocol. This requires normal network permissions but still belongs to the
  shell.
- OSC UDP ingress: the Quest app listens on a UDP port and an operator tool,
  phone, or desktop app sends OSC datagrams over the LAN. ADB TCP
  forward/reverse does not provide a UDP tunnel, so same-network addressing is
  the normal public test path.

For Windows-side inspection of saved encoded payloads, Rusty XR Companion may
use an optional FFmpeg executable as an external media sidecar. That runtime is
not part of Rusty XR core and is not bundled into the companion app zip by
default. The companion can install a verified Windows x64 LGPL shared FFmpeg
build into its managed LocalAppData cache, or users can supply their own
`ffmpeg.exe` path. Quest-side encode/decode examples should continue to prefer
Android platform MediaCodec rather than bundled codec libraries.

The public helper at `tools/media-pipeline/frame_receiver.py` implements a
small Windows-side receiver for the first pattern. It is intentionally generic:
it receives length-prefixed frame packets, writes payloads and metadata to a
local folder, and does not know any package name or private visual behavior.

## Frame Receiver Protocol

`frame_receiver.py` listens for TCP connections. Each frame packet is:

```text
u32 little-endian JSON header byte length
UTF-8 JSON header
payload bytes, length from header.byte_len
```

Required JSON header fields:

- `byte_len`: payload byte count

Recommended JSON header fields:

- `frame_index`
- `timestamp_ns`
- `capture_time_ns` for runtime-supplied depth capture time when available
- `width`
- `height`
- `format`, such as `rgba8888`, `png`, `jpeg`, or `depth_u16le`
- `stream`, such as `composite`, `left_camera`, `right_camera`, or `depth`
- `eye` or `layer_index` for per-eye depth or camera payloads
- `near_z` and `far_z` for environment-depth payloads when available

Example development setup:

```powershell
python tools\media-pipeline\frame_receiver.py --port 8787 --output artifacts\media-stream
adb reverse tcp:8787 tcp:8787
```

The app shell can then connect from the headset to `127.0.0.1:8787` and send
packets using the protocol above.

## OSC Probe Protocol

`rusty-xr-osc` provides a common packet codec and UDP helper. The public Quest
example exposes an `osc-udp-listener` runtime profile that logs received packet
summaries. Rusty XR Companion Apps can send a generic probe:

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- osc send --host <quest-lan-ip> --port 9000 --address /rusty-xr/probe --arg string:hello
```

Treat this as transport proof only. App-specific address trees and high-rate
sensor semantics belong in downstream adapters until they stabilize into public
contracts.

The broker proof-of-concept in `examples/quest-broker-apk/` adds a second OSC
test shape. Launching the `broker-osc-drive-ingress` profile starts a
non-rendering sidecar service through a no-display starter activity. The service
listens for `/rusty-xr/drive/radius` on UDP port `9000` and rebroadcasts
accepted values to localhost WebSocket clients as `osc_drive` JSON events. This
was validated with
[The Big Red Button Institute](https://github.com/MesmerPrism/the-big-red-button-institute),
the public Unity Quest example that drives one visible button through direct
Unity input and broker-routed events.

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- catalog launch --path .\examples\quest-broker-apk\catalog\rusty-xr-quest-broker.catalog.json --app rusty-xr-quest-broker --serial <serial> --runtime-profile broker-osc-drive-ingress
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- osc send --host <quest-lan-ip> --port 9000 --address /rusty-xr/drive/radius --arg float:0.75
```

## Permission Taxonomy

Android permissions fall into several categories. The app shell should document
which category each feature uses.

### Manifest / Install-Time Permissions

These are declared in `AndroidManifest.xml` and granted automatically when the
APK is installed if the platform allows them.

Typical media-pipeline examples:

- `android.permission.INTERNET` for socket streaming.
- `android.permission.ACCESS_NETWORK_STATE` for network diagnostics.
- `android.permission.FOREGROUND_SERVICE`.
- `android.permission.FOREGROUND_SERVICE_MEDIA_PROJECTION` when targeting
  Android 14+ and using a foreground service for MediaProjection.

These can be "granted in the APK" only in the sense that declaring them in the
manifest is enough. A launcher does not need to request a headset popup for
normal install-time permissions.

Signature permissions are also install-time, but third-party apps do not get
them unless signed with the defining certificate. Do not design public Rusty XR
features around signature permissions.

### Runtime / Dangerous Permissions

Runtime permissions must be declared in the manifest and requested by a visible
activity before use. Requesting them spawns the system/headset permission popup.

Typical media-pipeline examples:

- `android.permission.CAMERA` for Android camera access.
- `horizonos.permission.HEADSET_CAMERA` for Quest passthrough camera access on
  supported Horizon OS versions.
- `android.permission.RECORD_AUDIO` if the media pipeline captures microphone
  audio.
- `android.permission.POST_NOTIFICATIONS` when target/API behavior requires a
  notification runtime grant.

Request them when the user starts the feature, not at app launch. In an Android
shell this means checking the permission, explaining the need in app UI, then
calling the platform runtime permission request API from the foreground
activity. For example, request camera/headset-camera permission immediately
before opening Camera2 or the Passthrough Camera API.

During development, a Windows launcher using ADB can usually grant ordinary
declared runtime permissions with:

```powershell
adb shell pm grant <package> <permission>
```

or install with:

```powershell
adb install -g <apk>
```

For Quest raw-camera validation, pregrant the declared camera permissions before
the measurement window when the run is meant to be unattended:

```powershell
adb shell pm grant <package> android.permission.CAMERA
adb shell pm grant <package> horizonos.permission.HEADSET_CAMERA
```

Some generated Quest manifests also request adjacent runtime permissions such as
`horizonos.permission.AVATAR_CAMERA` or `android.permission.RECORD_AUDIO`. Grant
only permissions declared by the APK and required by the profile under test, and
record the grant set in the run manifest or artifact summary.

Do not treat ADB grants as a substitute for production UX. Some OEM or
headset-specific privacy surfaces may still require user action in the headset,
and every runtime permission must be checked before use.

To force the popup to reappear during tests after a denial:

```powershell
adb shell pm clear-permission-flags <package> <permission> user-set user-fixed
```

Then relaunch the app and request the permission again from the foreground
activity.

### MediaProjection Consent

MediaProjection is not just a manifest permission. The app must request user
consent for each capture session by calling
`MediaProjectionManager.createScreenCaptureIntent()` from a visible activity.
That intent produces the system/headset consent UI. The returned token is
single-use for `createVirtualDisplay()` on Android 14+.

For Android 14+ targets using a foreground service, declare the
MediaProjection foreground-service permissions and service type in the
manifest, but request screen-capture consent before starting or using the
foreground service.

A custom launcher should not try to bypass MediaProjection consent. It can
install, launch, set debug properties, prepare `adb reverse`, and watch logs,
but the app must still trigger the consent flow in headset.

On current Quest system UI, the MediaProjection flow can include an additional
`Select view you want to share` panel after the first consent prompt. Treat this
as a headset/user step: select `Entire view`, then press `Share` in the
headset. ADB shell taps and UIAutomator inspection can see parts of this panel
on some firmware versions, but they cannot reliably select the view or enable
the final share action. A public launcher or validation harness should report
the blocked state instead of claiming it can clear the selector automatically.

When validating a display-composite stream, distinguish app failure from a
receiver that intentionally exits after one frame. A short-lived Windows
receiver can close the socket after proving the first payload, which may make
the app log a broken pipe even though capture and transport were working.

### Special Permissions

Special permissions are neither normal install-time permissions nor runtime
dialogs. They route the user to system settings or an OEM-specific surface.

Examples include drawing over other apps, all-files access, or exact alarms.
Avoid these in the public media pipeline unless there is a strong product need.
If a downstream app uses one, it should present rationale UI, launch the
corresponding settings intent, and re-check state when the user returns.

## Headset Popup Flow

Use this app-side flow for permissions that require the user to approve inside
the headset:

1. User starts a feature such as "Start camera stream" or "Start display
   capture".
2. App checks the required permission or consent state.
3. App shows a short in-app rationale if the user has not already granted it.
4. App invokes the platform request:
   - Runtime permission: request the manifest permission from the foreground
     activity.
   - MediaProjection: launch `createScreenCaptureIntent()`.
   - Special permission: launch the relevant system settings intent.
5. User accepts in headset.
6. App receives the result, opens the camera/projection, and starts streaming.
7. App handles denial by disabling only the feature that needed the permission.

For Quest OpenXR examples, validate the renderer separately from
MediaProjection consent. First launch with display capture disabled and confirm
OpenXR reaches `READY` / `FOCUSED`, camera frames are received, and the custom
layer submits frames. Then repeat with MediaProjection enabled and complete the
headset consent/selector flow. This keeps a blocked consent overlay from being
misdiagnosed as an OpenXR, Camera2, or Vulkan renderer failure.

## Public Tool Boundary

Rusty XR can include generic Windows receivers, frame protocol definitions,
metadata contracts, visual feedback border layout, and diagnostic models. It
should not include private package names, private launch scripts, app-specific
stream defaults, signing keys, device serials, captured frame payloads, or
project-specific visual behavior.

## Source References

- Android permissions overview:
  <https://developer.android.com/guide/topics/permissions/overview>
- Android runtime permissions:
  <https://developer.android.com/training/permissions/requesting>
- Android special permissions:
  <https://developer.android.com/training/permissions/requesting-special>
- Android MediaProjection:
  <https://developer.android.com/media/grow/media-projection>
- Android 14 foreground service types:
  <https://developer.android.com/about/versions/14/changes/fgs-types-required>
- Android 14 MediaProjection behavior changes:
  <https://developer.android.com/about/versions/14/behavior-changes-14>
- Meta Passthrough Camera API overview:
  <https://developers.meta.com/horizon/documentation/unity/unity-pca-overview/>
- Meta Depth API overview:
  <https://developers.meta.com/horizon/documentation/unity/unity-depthapi-overview/>
- OpenXR `xrAcquireEnvironmentDepthImageMETA`:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/xrAcquireEnvironmentDepthImageMETA.html>
- OpenXR `XrEnvironmentDepthImageMETA`:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/XrEnvironmentDepthImageMETA.html>
- OpenXR `XrEnvironmentDepthImageTimestampMETA`:
  <https://registry.khronos.org/OpenXR/specs/1.1/man/html/XrEnvironmentDepthImageTimestampMETA.html>
- Unity Meta OpenXR occlusion platform support:
  <https://docs.unity.cn/Packages/com.unity.xr.meta-openxr%402.1/manual/features/occlusion/platform-support.html>
