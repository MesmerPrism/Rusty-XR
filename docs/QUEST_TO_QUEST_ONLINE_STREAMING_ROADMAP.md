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
- launch-provided projection metadata for existing streams
- hardware-buffer decode into the existing GPU-import path
- metadata-backed projected stereo rendering
- multi-screenshot visual freshness checks
- scorecards that separate packet, decode, native handoff, GPU import, and
  OpenXR projection evidence

The important result is not just that the screen changes. A valid pass should
show projected shader use, aligned projection, nonzero decoded frames, nonzero
accepted stereo pairs, zero native rejects, zero queue drops, zero GPU import
failures, and non-byte-identical visible screenshots.

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

### Phase 0: Keep The Laptop Loop As The Gate

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

### Phase 1: One-Way LAN Quest-To-Quest

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

### Phase 2: One-Way Mediated LAN

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

### Phase 3: Two-Way LAN

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

### Phase 4: Online Relay MVP

For a first internet-capable version, keep the current H.264 framing and add an
authenticated relay that both Quests connect to with outbound TLS connections.
This avoids relying on inbound connections through phone hotspots, routers, or
carrier NAT.

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

### Phase 5: WebRTC Adapter

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

### Phase 6: Adaptive Quality And Comfort

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
| TURN relay | Reliable fallback for remote networks | Adds bandwidth cost and latency, but is required for real-world reliability. |

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

- `StreamDirection`: incoming, outgoing, bidirectional
- `MediaTrackKind`: video, audio, metadata, control
- `VideoCodecPreference`: H.264 first, others optional
- `StereoMediaLayout`: separate streams, side-by-side, mono fallback
- `SignalingSessionState`
- `IceConnectionState` / `RelayConnectionState`
- `NetworkQualitySample`
- `RemoteStreamHealth`
- `ProjectionMetadataEnvelope`
- `PairingPolicy` and `RemoteSessionSecurityPolicy`

Adapters then map those contracts onto TCP diagnostics, WebSocket relay, or
WebRTC without moving native SDKs or generated media artifacts into core.

## Validation Gates

Do not promote a network session unless all of these are captured:

- sender stream start acknowledgements
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

The run can be visually promising and still fail the gate if it only proves
that pixels changed. The scorecard must show that the pixels came through the
intended stereo media path and were projected by the intended XR renderer.
