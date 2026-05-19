# Quest-To-Quest Online Streaming Roadmap

This roadmap describes the public implementation path from the current
diagnostic Quest camera streaming examples toward online two-way stereo camera
projection between two Quest devices.

Keep this document public-safe. Do not commit headset serials, local IPs,
local paths, raw screenshots, generated captures, private package names, or
downstream visual-effect behavior. Store run artifacts under ignored
`artifacts/` folders.

## Current Proven Baseline

Rusty XR now has a validated one-device laptop-loop foundation:

```text
Quest Camera2 stereo capture
  -> broker H.264 MediaCodec encoders
  -> laptop relay / isolation hop
  -> broker receiver proxy
  -> composite existing-stream H.264 consumers
  -> MediaCodec hardware-buffer decode
  -> native AHardwareBuffer import
  -> Vulkan/OpenXR projected stereo draw
```

The current public example path supports:

- stereo Camera2 IDs selected by runtime profile
- Android platform H.264 encode/decode
- `RXYRVID1` binary stream headers and packet timing
- existing-stream receiver mode
- schema-3 stream-header projection metadata for existing streams, with launch
  metadata as a fallback
- hardware-buffer decode into the existing GPU-import path
- metadata-backed projected stereo rendering
- multi-screenshot visual freshness checks
- scorecards that separate packet, decode, native handoff, GPU import, and
  OpenXR projection evidence

The important result is not just that the screen changes. A valid pass should
show projected shader use, aligned projection, nonzero decoded frames, nonzero
accepted stereo pairs, zero native rejects, zero queue drops, zero GPU import
failures, and non-byte-identical visible screenshots.

## 2026-05-13 Planning Update

The next Q2Q slice should keep the current `RXYRVID1` H.264 diagnostic stream
and Android platform MediaCodec path. The missing production shape is not a new
codec stack first; it is stronger session metadata, camera/source capability
manifests, bounded queues, runtime media controls, temporal smoothing, and
projection metadata that can travel with the session.

Updated priority:

1. Re-validate current `main` with no-smoothing temporal metrics.
2. Add camera/source capability and timestamp-domain manifests.
3. Promote H.264 stream invariants into scorecard gates.
4. Add runtime media controls for keyframes and bitrate.
5. Implement temporal projection smoothing before hiding render issues behind
   more network work.
6. Make projection metadata session-native.
7. Add one-way LAN Q2Q profiles.
8. Add mediated LAN relay with hard backpressure.
9. Add reduced-quality two-way LAN.
10. Add online TLS relay.
11. Add ASW, WebRTC, and WebTransport only as measured adapter lanes.

This order is deliberate. The current diagnostics already show that direct
Camera2 projection and broker-live projection share the projected draw/render
bottleneck, while encode/decode/handoff are not the dominant measured cost.
Online Q2Q should therefore improve and measure the renderer path before
adding a more opaque transport layer.

As of the current diagnostic implementation, items 1-6 are implemented for the
single-headset laptop-loop proof, and receiver-side frame-adoption/edge metrics
are visible in the projected path. The next public milestone is one-way LAN Q2Q
using the same stream-header metadata path, with additional motion stress tests
used to tune the adoption thresholds.

Validation status after the 2026-05-13 streaming pass:

- E1 direct in-app Camera2 to the Rusty XR projected screen passed.
- E2 on-device broker H.264 to the Rusty XR projected screen passed.
- E3 Quest camera to laptop relay to Quest receiver to the Rusty XR projected
  screen passed.
- E4 one-way LAN Quest-to-Quest, E5 reverse direction, and E6 reduced-quality
  two-way LAN are still pending a second headset on the same network.

## 2026-05-16 Remote Relay Update

The first public-internet rehearsal proved that DNS, relay reachability, TLS
verification, token auth, and relay byte forwarding can work across remote
sites. It also showed that PC-mediated bridge mode creates failure modes that
are not inherent to Quest-to-Quest streaming: sender PC to Quest LAN
reachability, receiver PC firewall rules, ADB forward/reverse freshness, and
one-shot stream sources consumed by probes.

Bridge mode should therefore remain a fallback and diagnostic scaffold. The
primary online milestone is now a Quest-native relay client in the broker path:
the sender broker writes `RXYRVID1` output to outbound TLS relay sockets, and
the receiver broker reads outbound TLS relay sockets into device-local
existing-stream inputs for the composite. Agent PCs should coordinate through
ADB, broker commands, screenshots, logcat, and scorecard extraction only.

## 2026-05-19 Native Relay Update

The first two-way Quest-native relay session moved live Camera2 H.264 payload
in both directions through broker-owned outbound TLS relay clients. Side A to
Side B copied tens of megabytes per eye before a connection reset. Side B to
Side A copied more than one hundred megabytes per eye over roughly two minutes,
with relay and receiver broker byte counters agreeing exactly.

This promotes the online relay milestone from "relay reachable" to "native
two-way payload proven." It does not yet prove stable long-session quality or
frame-level stereo sync. The first hardening pass adds per-lane `RXYRVID1`
stream counters to `q2q_relay.get_status`, preserves relay byte counters on
exception close paths, reports receiver-side complete frame-set gate counters
before native stereo texture commit, and adds offline scorecard/session-plan
tools. Remaining gating work is an executing receiver-first orchestrator,
final-status-before-stop capture, reduced-quality public-relay validation, and
another measured two-Quest remote run.

The public-safe retrospective and next-run checklist live in
[Quest-to-Quest native relay session, 2026-05-19](QUEST_TO_QUEST_NATIVE_RELAY_SESSION_2026_05_19.md).

## Target Shape

The online target should remain native Quest-to-Quest streaming. Browser and
web pages are useful for pairing, dashboards, and operator views, but the core
camera and projection runtime should stay in native Quest APKs because it needs
Camera2, MediaCodec, hardware buffers, Vulkan/OpenXR texture import, and XR
layer submission.

```text
Quest A sender
  broker APK
  Camera2 left/right sources
  MediaCodec H.264 encoders
  projection metadata and timing
  network media sender

Internet / LAN / relay
  signaling and pairing
  optional TURN or relay fallback
  media packets
  metadata and health telemetry

Quest B receiver
  composite-layer APK
  network media receiver
  MediaCodec H.264 decoders
  hardware-buffer handoff
  Vulkan/OpenXR projected stereo draw

Repeat the same path in reverse for two-way sessions.
```

## Recommended Phases

### Phase 0: Baseline And Scorecard Hardening

Before every network change, keep a short local loopback gate working with the
same receiver profile:

- `camera-stereo-gpu-composite-fast075`
- `rustyxr.camera=false`
- `rustyxr.brokerH264Consumer=true`
- `rustyxr.brokerH264Stereo=true`
- `rustyxr.brokerH264SourceMode=existing-stream`
- `rustyxr.brokerH264DecodeOutputMode=hardware-buffer`
- `rustyxr.brokerH264StereoPairingMode=frame-order`

The sender should provide left and right projection metadata to the receiver,
preferably as session metadata rather than manual launch-only state. The
diagnostic harness should capture multiple screenshots during the live window,
then parse logcat and relay/proxy counters into one scorecard.

The baseline manifest should include:

- commit SHA
- role and direction
- session id, peer id, track id, and stream id
- runtime profile
- render scale
- projection shader path
- temporal mode and temporal policy values
- target/applied/residual temporal metrics
- camera, encoder, decoder, import, relay, and OpenXR cadence fields

While temporal smoothing is off, the no-smoothing gate should prove:

- `target_projection_motion_px_p95` is present
- `applied_projection_motion_px_p95` is present
- target and applied projection motion are equal
- residual, held-frame, crossfade, edge-fill, and ASW counters are zero
- frame-adoption mode, held/adopted decision state, candidate motion p95, max
  hold duration, invalid-UV percentage, and edge-fill percentage are visible
  before treating smoothing as a valid online-streaming improvement
- camera frame age is reported or explicitly unavailable due to timestamp
  domain mismatch

### Phase 1: Capability And Stream Invariants

Add a public camera/source capability manifest before relying on fixed source
IDs or fixed stream timing. A first contract can be named
`CameraSourceCapabilities` and should describe:

- source family and API path
- OS/runtime version observed by the app
- camera permission and headset camera permission state
- selected logical camera id and physical camera ids when exposed
- vendor camera-position/source tags when available
- supported `PRIVATE` and YUV sizes
- supported and selected frame-rate ranges
- selected stream size, stream minimum frame duration, and selected reason
- timestamp source and timestamp domain

Add a stream manifest gate for each H.264 eye stream:

- session id, role, direction, peer id, track id, stream id, and eye
- codec `h264` with Annex-B bitstream semantics
- encoder and decoder names
- bitrate, bitrate mode, I-frame interval, requested/applied latency modes
- SPS/PPS presence and codec-config packet counts before first frame
- keyframe/sync-frame count and recovery state
- decoder output mode, including hardware-buffer import state
- close reason for every stream lane

Reject Q2Q runs where SPS/PPS are missing, the first decoded frame appears
before config/IDR recovery is known, decoder low-latency state is unknown, or
encoder/decoder names are absent.

Runtime media controls should start as local broker commands:

- `media.request_keyframe`
- `media.set_video_bitrate`
- `media.set_quality_profile`

### Phase 2: Temporal Projection Smoothing

Finish the runtime smoothing lane before making two-headset transport the only
moving part. The first public profiles should be:

- `camera-stereo-temporal-pose-clamp-fast075`
- `camera-stereo-temporal-screen-clamp-fast075`

The screen-motion clamp is the important user-facing smoother. Start with:

```text
rustyxr.cameraTemporalProjectionEnabled=true
rustyxr.cameraTemporalMode=screen-motion-clamp
rustyxr.cameraTemporalMaxPixelsPerFrame=18
rustyxr.cameraTemporalCatchupHalfLifeMs=50
rustyxr.cameraTemporalMaxVisualLagMs=120
rustyxr.cameraTemporalStereoLockstep=true
```

The pose-delta profile uses the same projected path but proves that a single
angular/linear smoothing coefficient can be shared across both eyes:

```text
rustyxr.cameraTemporalMode=pose-delta-clamp
rustyxr.cameraTemporalMaxAngularDegreesPerFrame=1.25
rustyxr.cameraTemporalMaxLinearMetersPerFrame=0.012
```

Acceptance should require `applied_projection_motion_px_p95` to stay under the
configured cap except on explicit reset frames, while
`target_projection_motion_px_p95` and residual lag remain visible. Frame
adoption smoothing and shader-owned edge handling follow the clamp:

- `rustyxr.cameraFrameAdoptionMode=hold-until-smooth`
- `rustyxr.cameraFrameAdoptionMaxJumpPx=24`
- `rustyxr.cameraFrameAdoptionMaxHoldMs=80`
- `rustyxr.cameraTemporalEdgeMode=clamp-soft`

ASW and depth-aware variants remain probes after the planar governor works.

### Phase 3: Session-Native Projection Metadata

Move receiver projection metadata out of manual launch-only state. Add a
session envelope such as:

```rust
ProjectionMetadataEnvelope {
    schema,
    session_id,
    sender_id,
    track_id,
    eye,
    camera_id,
    source_eye_mapping,
    texture_transform,
    delivered_size,
    intrinsics,
    intrinsics_domain,
    extrinsics,
    pose_source,
    timestamp_domain,
    capability_hash,
}
```

The receiver should prefer projection metadata in this order:

1. active session metadata, including `RXYRVID1` schema-3 stream-header
   projection metadata
2. broker status or projection profile
3. explicit launch extra
4. diagnostic fallback or flat-probe downgrade

The public broker/composite diagnostic path now uses the first item for
existing-stream H.264 stereo: the sender writes per-eye projection metadata into
the stream header, TCP proxy hops forward it unchanged, and the receiver logs
the selected session metadata source before projected draw.

### Phase 4: One-Way LAN Quest-To-Quest

Split the current one-device flow across two headsets on the same LAN:

```text
Quest A broker sender
  -> LAN-reachable left/right H.264 endpoints
Quest B composite receiver
  -> existing-stream left/right clients
  -> projected stereo draw
```

Required additions:

- receiver runtime extras for remote sender host and ports
- explicit sender session start/stop lifetime
- sender role and receiver role in run manifests
- stream metadata handoff from sender to receiver
- proxy/readiness waits before launching the receiver
- reconnect and failure reasons for each stereo lane

Acceptance should require the same scorecard fields as the laptop-loop gate,
plus sender and receiver role logs.

The receiver should not launch projection until a readiness check confirms:

- left and right sender endpoints are listening
- projection metadata is available
- receiver proxy or direct receiver is connected
- first headers are validated

### Phase 5: One-Way Mediated LAN

Add a computer or phone relay between the two devices, but keep the media format
unchanged. This proves the session can survive a separate relay process before
adding internet NAT traversal.

The relay should be an explicit role:

```text
Quest A sender -> relay -> Quest B receiver
```

The relay must report:

- accepted client endpoints
- upstream connect success
- bytes forwarded per eye
- close reason
- packet/header validation failures

The relay must never use unbounded media buffers. Record:

- relay buffered bytes and packets
- max buffered bytes
- drop count and drop reason
- slow-peer close count
- sender write-stall timing
- receiver read-gap timing

### Phase 6: Two-Way LAN

Run two one-way sessions at once:

```text
Quest A camera -> Quest B projection
Quest B camera -> Quest A projection
```

This phase should reduce resolution or bitrate before chasing visual quality.
The first useful target is stable bidirectional operation with bounded latency
and no decode/import failures, not maximum resolution.

Track per direction:

- source packet rate
- wire packet rate
- decoded frame rate
- accepted stereo pair rate
- queue drops
- native rejects
- GPU import failures
- OpenXR frame rate
- screenshot freshness

The first two-way target should be conservative: `720x720` or `960x960`, 15 to
30 fps, lower bitrate per eye, and `fast065` before `fast075`. Enable temporal
screen clamp only after the baseline two-way run renders in both directions.

### Phase 7: Online Relay MVP

For a first internet-capable version, keep the current H.264 framing and add an
authenticated relay that both Quests connect to with outbound TLS connections.
This avoids relying on inbound connections through phone hotspots, routers, or
carrier NAT.

The first implementation should be broker-native, not PC-bridge-first:

- sender broker command starts left/right Camera2 H.264 streams and connects
  each eye to the relay as a `sender` lane
- receiver broker command connects each eye to the relay as a `receiver` lane
  and exposes device-local existing-stream ports for the composite
- PCs push short-lived test CA/token material, send broker commands, and
  collect logs, but do not forward media bytes
- PC bridge clients remain available for transport isolation and fallback only

Suggested split:

- WebSocket or HTTP control channel for pairing, offers, health, and metadata
- binary media channel for `RXYRVID1` packets
- short pairing code or QR code
- expiring session token
- per-session TURN/WebRTC configuration placeholder, even before WebRTC is
  implemented

This is not the final media architecture, but it is the fastest way to test
online user flow, authorization, relay bandwidth, and diagnostics with the
stream format already used by the Quest examples.

Add an explicit privacy tier to the contracts:

- `LocalLanDiagnostic`
- `TrustedRelayTransportEncrypted`
- `UntrustedRelayEndToEndEncryptedCandidate`

Debug builds may opt into LAN cleartext for development. Release/online builds
should require TLS, avoid manifest-wide cleartext, keep raw media out of relay
logs by default, expose visible streaming state, and provide an immediate stop
action.

### Phase 8: WebRTC Adapter

WebRTC is the preferred long-term online media transport because it provides
ICE negotiation, congestion control, jitter handling, NAT traversal, and TURN
relay fallback. Rusty XR core should model the session and metrics; the native
Quest APK or a companion sidecar should own the WebRTC implementation.

Use:

- WebSocket/HTTPS signaling for offer, answer, ICE candidates, and pairing
- STUN for direct candidates
- TURN for reliable fallback
- WebRTC data channel for projection metadata, stream health, clock-sync
  samples, and control messages
- H.264 video tracks where supported by the Android/WebRTC stack

Avoid CPU frame copies. The production adapter should prove a decoder path that
can feed the existing native texture-import/projection path before it is
treated as a replacement for the current diagnostic transport.

Do not tunnel opaque `RXYRVID1` packets into a WebRTC video track. The migration
rule is:

```text
RXYRVID1 diagnostic H.264
  -> parse stream headers and packets
  -> extract H.264 access units / NAL units
  -> packetize as RTP/H.264 in the WebRTC adapter
  -> send projection and timing metadata over a data channel
```

Collect WebRTC stats for ICE state, selected candidate pair, RTT, available
bitrate, packets sent/received/lost, jitter, encoded/decoded frames, decoded
keyframes, dropped frames, freezes, decode time, and data-channel buffered
amount.

### Phase 9: WebTransport Investigation

WebTransport over HTTP/3 is an investigation lane after the TLS relay MVP. It
can be useful for multiplexed control, metadata, bidirectional streams, and
datagram experiments, but it should not block the first online Q2Q session.
Keep it only if headset and hotspot tests show better bounded latency,
buffering, or multiplexing than the WebSocket/TLS relay.

### Phase 10: Adaptive Quality And Comfort

After two-way online connectivity works, add adaptation:

- bitrate and resolution profiles
- one-eye fallback for degraded links
- stereo pair drop policy under jitter
- key-frame request policy
- clock sync and one-way latency estimates
- operator-visible privacy and session state
- comfort gates for latency, freezes, and black-screen recovery

## Transport Choices

| Transport | Best Use | Tradeoff |
| --- | --- | --- |
| LAN TCP with `RXYRVID1` | Current diagnostics and LAN validation | Simple and debuggable, but not internet/NAT-ready. |
| Mediated WebSocket/TLS | Fast online MVP using current framing | Easy pairing and firewall behavior, but media quality control is custom. |
| WebRTC | Long-term online Quest-to-Quest media | More integration work, but best fit for low-latency interactive video. |
| WebTransport | Experimental post-relay lane | Promising multiplexing/datagram shape, but not the first MVP dependency. |
| TURN relay | Reliable fallback for remote networks | Adds bandwidth cost and latency, but is required for real-world reliability. |

## FFmpeg And External Runtime Boundary

FFmpeg remains a desktop or companion-side inspection sidecar, not the default
Quest streaming runtime. It is useful for saved H.264 inspection, remuxing,
preview decode, thumbnails, and reference transcode checks on a PC. Quest-side
sender and receiver examples should continue to use Android platform
MediaCodec, ImageReader/HardwareBuffer, AHardwareBuffer import, Vulkan sampling,
and OpenXR projection.

Do not add FFmpeg, libx264, libx265, GStreamer, WebRTC, NDI, or similar native
payloads to Rusty XR core or default Quest APKs without a separate dependency
and release audit.

## Bandwidth Starting Point

Current high-quality stereo diagnostics use square `1280x1280` H.264 streams
with a multi-megabit bitrate per eye. Treat that as a visual-quality baseline,
not the first online default.

Start online tests lower:

- `720x720` or `960x960`
- 15 to 30 fps
- one-way first
- one relay hop first
- explicit bitrate caps

Increase resolution only after packet, decode, import, projection, and
freshness counters stay healthy.

## Security And Privacy Requirements

Quest camera streams are sensitive. Remote streaming must be explicit and
visible:

- disabled by default
- operator-initiated pairing
- expiring pairing code or token
- encrypted signaling and media relay
- clear sender/receiver role state
- explicit camera permission flow
- no default recording
- logs and scorecards without raw media payloads
- status command to list active remote peers
- immediate stop command for both sender and receiver

Do not hide raw camera streaming behind a generic "video out" toggle. The app
should name the source, destination, and active direction.

## Public Contract Work

Core Rusty XR should add framework-neutral models before adopting a native
online media stack:

- `CameraSourceCapabilities`
- `StreamDirection`: incoming, outgoing, bidirectional
- `MediaTrackKind`: video, audio, metadata, control
- `VideoCodecPreference`: H.264 first, others optional
- `StereoMediaLayout`: separate streams, side-by-side, mono fallback
- `SignalingSessionState`
- `IceConnectionState` / `RelayConnectionState`
- `NetworkQualitySample`
- `RemoteStreamHealth`
- `ProjectionMetadataEnvelope`
- `PrivacyTier`
- `PairingPolicy` and `RemoteSessionSecurityPolicy`

Adapters then map those contracts onto TCP diagnostics, WebSocket relay, or
WebRTC without moving native SDKs or generated media artifacts into core.

## Validation Gates

Do not promote a network session unless all of these are captured:

- sender stream start acknowledgements
- camera/source capability manifest
- timestamp-domain manifest
- receiver stream headers for both eyes
- SPS/PPS priming for both eyes
- decoder names and output mode
- decoded frame counts for both eyes
- accepted stereo pair count
- queue drop and native reject counts
- GPU import success/failure counts
- final projection status with `projectionShaderPath=projected`
- aligned projection state
- OpenXR frame cadence summary
- multiple visible non-identical screenshots
- relay or WebRTC connection stats
- close reason for every stream

## Do Not Do Yet

Do not make these the next implementation step:

- full WebRTC replacement of the current diagnostic path
- WebTransport as a required dependency
- ASW default-on for camera projection
- native passthrough as a dependency for custom projection smoothness
- FFmpeg-on-Quest
- opaque `RXYRVID1` packets shoved into RTP/WebRTC video tracks
- online release profiles that rely on cleartext base network config
- timestamp-nearest pairing without timestamp-domain manifests
- two-way `1280x1280` before one-way `720`/`960` is stable

The run can be visually promising and still fail the gate if it only proves
that pixels changed. The scorecard must show that the pixels came through the
intended stereo media path and were projected by the intended XR renderer.
