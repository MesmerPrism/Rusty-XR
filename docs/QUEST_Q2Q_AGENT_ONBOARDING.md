# Quest-To-Quest Agent Onboarding

This note is for an upstream XR collaborator or an agent working with that
collaborator on a proper Quest-to-Quest streaming setup from the public Rusty
XR repository. It assumes the developer may be working on macOS and may have
multiple Quest headsets available.

## Brief Message For A Collaborator

Rusty XR now has enough public Quest infrastructure to make a serious
Quest-to-Quest streaming test practical. The repo has a Rust/OpenXR/Vulkan
composite-layer APK, a separate broker APK, Camera2 `PRIVATE`
hardware-buffer import, metadata-backed stereo projection, Android MediaCodec
H.264 encode/decode probes, live stereo broker streaming profiles, and reusable
diagnostic scorecards.

The most useful recent finding is that the broker receive/decode path and
Java/native hardware-buffer handoff can be isolated from projected-render cost.
In current public-example tests, synthetic compositor and broker receive/decode
lanes stayed stable, while both direct in-app Camera2 projection and broker
live projected stereo hit the same render-scale-sensitive projected draw path.
That makes the next collaboration target clear: turn the existing diagnostic
broker/composite path into a proper two-headset setup, then optimize and
profile the shared projected render path with repeatable scorecards.

## Goal For The Agent

Build and run a public, reproducible Quest-to-Quest streaming validation using
two or more Quest headsets:

- one headset acts as the camera/H.264 sender through the public broker APK
- another headset runs the public composite-layer APK and receives/decodes the
  stereo H.264 stream
- both sides emit enough timing and status logs to separate capture, encode,
  network, decode, hardware-buffer handoff, import, projection, and OpenXR
  render cost
- all artifacts remain local under ignored `artifacts/` folders
- any changes proposed for Rusty XR stay public, portable, and free of
  downstream app identities, private assets, serial numbers, local paths, or
  generated captures

The current one-device broker/composite live path is a diagnostic foundation,
not the final Q2Q product. A strong agent should first prove the existing
catalog profiles locally, then split the sender and receiver across two
headsets.

## Repository Map

Start at the public repo root:

```text
Rusty-XR/
```

Read these first:

- the repo-local agent notes: public boundary, validation commands, and
  source-workspace rules.
- `README.md`: high-level project shape and examples.
- `docs/QUEST_STREAMING_DIAGNOSTICS_WORKFLOW.md`: the current streaming cost
  matrix and scorecard interpretation rules.
- `docs/MEDIA_PIPELINE_AND_PERMISSIONS.md`: source taxonomy, Camera2,
  MediaCodec, broker, MediaProjection, and permissions boundaries.
- `examples/quest-composite-layer-apk/README.md`: composite-layer APK,
  runtime profiles, launch extras, expected log signals, and current known
  gaps.
- `examples/quest-broker-apk/README.md`: broker APK capabilities and H.264
  side-channel behavior.
- `tools/quest-streaming-diagnostics/README.md`: scorecard parser and wrapper.
- `tools/quest-camera-profile/README.md`: profile-run harness and screenshot /
  log capture helpers.

Important source files:

- `examples/quest-composite-layer-apk/src/com/example/rustyxr/composite/BrokerH264ConsumerProbe.java`
  receives broker H.264 streams, decodes with Android MediaCodec, records live
  stereo timing, and forwards decoded hardware buffers.
- `examples/quest-composite-layer-apk/src/com/example/rustyxr/composite/HeadsetCameraService.java`
  owns the direct Camera2 camera path and direct stereo stage timing.
- `examples/quest-composite-layer-apk/native/src/native_camera.rs` receives
  Java hardware-buffer and camera metadata records on the Rust/native side.
- `examples/quest-composite-layer-apk/native/src/openxr_vulkan.rs` owns the
  OpenXR/Vulkan projection draw path and import/render timing signals.
- `examples/quest-broker-apk/src/com/example/rustyxr/broker/BrokerAppCameraH264StreamSession.java`
  owns broker app-context Camera2-to-H.264 streaming.
- `examples/quest-broker-apk/src/com/example/rustyxr/broker/BrokerH264TcpProxySession.java`
  proxies incoming H.264 streams for receiver-side isolation.
- `tools/video/serve_rxyrvid1_h264.py` serves a bounded `RXYRVID1` H.264 test
  stream from host files.

## Current Runtime Profiles

Use the composite-layer catalog:

```text
examples/quest-composite-layer-apk/catalog/rusty-xr-quest-composite-layer.catalog.json
```

Relevant profiles:

- `synthetic-composite-layer`: OpenXR/Vulkan compositor baseline, no camera.
- `camera-stereo-gpu-composite`: direct in-app Camera2 projected stereo at
  `rustyxr.xrRenderScale=0.75`.
- `camera-stereo-gpu-composite-performance-065`: same direct projected path at
  `rustyxr.xrRenderScale=0.65`.
- `broker-h264-stereo-live-openxr-projection-probe`: broker live stereo
  H.264, live decode, hardware-buffer handoff, and projected OpenXR draw.
- `broker-h264-stereo-live-openxr-projection-scale065-probe`: same broker live
  projected path at render scale `0.65`.
- `broker-h264-stereo-openxr-projection-probe`: retained/bounded stereo broker
  H.264 path for regression comparisons.

Use the broker catalog:

```text
examples/quest-broker-apk/catalog/rusty-xr-quest-broker.catalog.json
```

The broker APK can run as a headset-local sidecar with localhost status,
WebSocket events, OSC tests, Camera2 metadata probes, H.264 stream probes, and
binary H.264 side channels. It is still a public proof-of-concept, so a Q2Q
agent should expect to harden stream addressing, remote endpoint selection, and
session lifetime.

## macOS Setup Notes

The public source is portable, but the current APK build PowerShell scripts
were written around Windows paths and Unity's AndroidPlayer layout. On macOS,
an agent should not assume the scripts will run unchanged.

Install or verify:

- Xcode command-line tools
- Rust with `aarch64-linux-android` target
- Android SDK command-line tools and platform tools
- Android build tools, platform `android-35`, and NDK
- JDK compatible with Android build tools
- PowerShell 7 (`pwsh`) if reusing the PowerShell harnesses
- Python 3 for scorecard and helper scripts
- `adb` on `PATH`
- a Quest-compatible `libopenxr_loader.so`, supplied explicitly and not
  committed

The Mac build task is either:

1. Port the existing `Build-QuestCompositeLayerApk.ps1` and
   `Build-QuestBrokerApk.ps1` scripts to resolve macOS SDK/JDK/NDK tool paths,
   `LOCALAPPDATA` replacement paths, `darwin-*` NDK toolchain binaries,
   non-`.exe` / non-`.bat` build tools, and debug keystore locations.
2. Or reproduce the same steps in a small macOS shell script while keeping the
   public catalog APK output paths unchanged.

Do not commit OpenXR loader binaries, generated APKs, keystores, screenshots,
logs, or headset captures.

## Baseline Bring-Up

Before attempting two headsets, prove the examples on one headset.

1. Build and install the broker APK.
2. Build and install the composite-layer APK.
3. Grant camera permissions to both public example packages.
4. Launch `synthetic-composite-layer` and confirm OpenXR reaches focused state.
5. Launch `camera-stereo-gpu-composite-performance-065` and confirm the direct
   projected camera path reports `activeTier=gpu-projected`,
   `alignedProjection=true`, `stereoLayout=Separate`, zero CPU uploads, and
   nonzero OpenXR frame logs.
6. Start the broker APK, then launch
   `broker-h264-stereo-live-openxr-projection-scale065-probe` and confirm
   nonzero source packets, wire packets, decoded frames, native accepted stereo
   pairs, and final projected status.

Use `adb -s <serial>` for every command when more than one headset is attached.
Keep each run's artifacts in a separate ignored folder.

## Q2Q Architecture Target

The intended two-headset shape is:

```text
Quest A sender
  broker APK
  Camera2 PRIVATE left/right sources
  Android MediaCodec H.264 encoders
  RXYRVID1 stereo binary streams
  network-visible stream endpoint

Quest B receiver
  composite-layer APK
  two H.264 stream clients
  Android MediaCodec decoders
  ImageReader PRIVATE hardware buffers
  native stereo AHardwareBuffer bridge
  Vulkan hardware-buffer import
  metadata-backed OpenXR projected stereo draw
```

The current one-device path uses device-local localhost. For true Q2Q, the
agent should add or configure:

- sender LAN bind/advertise mode
- receiver host/port selection for the sender headset
- explicit stream-session start/stop lifetime
- timeout and reconnect behavior
- projection metadata transport from sender to receiver
- timestamp-based pair/drop policy under network jitter
- run manifests that identify roles as `sender` and `receiver` without
  committing serial numbers

Keep the binary media path separate from JSON/WebSocket status. High-rate H.264
payloads should remain on the `RXYRVID1` binary stream, while manifests,
metrics, capabilities, and operator status can use JSON.

## Diagnostic Matrix To Run

Once the two APKs work, run the matrix from
`docs/QUEST_STREAMING_DIAGNOSTICS_WORKFLOW.md`:

- synthetic compositor only at `0.75` and `0.65`
- direct in-app projected Camera2 at `0.75` and `0.65`
- broker existing-stream receive/decode at `0.75` and `0.65`
- broker live projected stereo at `0.75` and `0.65`
- true Q2Q projected stereo at `0.75` and `0.65`

Expected scorecard command:

```powershell
pwsh -File ./tools/quest-streaming-diagnostics/Invoke-QuestStreamingScorecard.ps1 `
  -ArtifactDirs ./artifacts/<run-a>,./artifacts/<run-b>,./artifacts/<run-c>
```

If PowerShell is not practical on macOS, call the parser directly:

```bash
python3 tools/quest-streaming-diagnostics/Parse-QuestStreamingArtifact.py \
  artifacts/<run-a> \
  artifacts/<run-b> \
  artifacts/<run-c> \
  --markdown-out artifacts/q2q-scorecard/scorecard.md \
  --json-out artifacts/q2q-scorecard/scorecard.json
```

## Signals That Matter

A run is useful only if it records:

- active runtime profile and all launch extras
- sender and receiver role labels
- source packet rate per eye
- wire packet rate per eye
- decoded frame count and decode rate per eye
- stereo pair count, pair delta average/max, native accepted/rejected counts,
  and queue drops
- direct or broker stage timings for image wait/acquire, `HardwareBuffer`, and
  native bridge
- OpenXR observed FPS, average frame time, render scale, import-cache counts,
  and GPU import failures
- `VrApi` app time, CPU+GPU time, tear/stale counts, timewarp time, CPU, and
  GPU fields
- final projection status with `activeTier=gpu-projected`,
  `alignedProjection=true`, and `stereoLayout=Separate`

Do not rely on FPS alone. A receiver can hold display cadence while showing a
flat diagnostic probe, stale camera frames, or a high-drop stream.

## Current Interpretation Rule

The current public evidence says:

- the empty compositor path is not the expensive path
- broker receive/decode alone is not the expensive path
- Java image acquire/wait, `HardwareBuffer` extraction, and native bridge calls
  are not the current dominant cost
- the shared metadata-backed projected stereo render path is the next
  performance target
- `0.65` versus `0.75` is a linear render-scale comparison; `0.65` is about 25
  percent fewer render-target pixels than `0.75`

If a true Q2Q run adds network-related drops or latency, preserve that as a new
network lane instead of mixing it with projected-render conclusions.

## Agent Rules

An agent working on this task should:

- keep the repo-local public/private boundaries
- avoid committing any generated APK, capture, log, trace, device serial, local
  path, or headset-specific artifact
- make one-variable changes and update the scorecard after each lane
- preserve existing public catalog profile semantics
- add new runtime profiles only when a repeated launch shape is useful
- keep macOS script portability separate from Q2Q runtime behavior
- run docs links, schema checks, and the public boundary scan before proposing
  public changes

Useful validation commands from the repo root:

```bash
python3 tools/docs/check_links.py --repo-root .
python3 tools/schema/check_quest_app_catalog.py examples/quest-composite-layer-apk/catalog/rusty-xr-quest-composite-layer.catalog.json
python3 tools/schema/export_schemas.py --check
python3 tools/boundary-scan/rusty_xr_boundary_scan.py --repo-root .
```

Rust validation remains relevant when touching crates:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```
