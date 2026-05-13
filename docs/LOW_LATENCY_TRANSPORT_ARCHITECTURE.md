# Low-Latency Transport Architecture

Rusty XR should support low-latency media, control, telemetry, and XR input
diagnostics without becoming a vendor SDK wrapper or an app-specific streaming
product. This document defines the clean-room public shape.

## Purpose

The immediate target is a negotiated transport/session model around the
existing broker diagnostics:

- control commands and acknowledgements
- stream manifests and stream events
- bounded and live-bounded binary media payloads
- timing, drop, queue, and network-quality metrics
- optional LAN experiments with explicit pairing/security policy
- operator scorecards in companion tools

The concrete Quest-to-Quest online streaming plan is tracked in
[QUEST_TO_QUEST_ONLINE_STREAMING_ROADMAP.md](QUEST_TO_QUEST_ONLINE_STREAMING_ROADMAP.md).
This architecture note stays transport-neutral; that roadmap applies the same
contracts to the current public Quest broker/composite examples.

The public core owns contracts, validation, schemas, docs, and synthetic tests.
Quest broker APKs, OpenXR clients, Windows tools, browser experiments, and
vendor SDKs remain adapters, sidecars, or downstream shells.

## Non-Goals

This is not:

- a Kyber-compatible protocol
- a copied vendor packet format
- a bundled external SDK runtime
- a WebRTC, FFmpeg, libVLC, QUIC, or WebTransport implementation in core
- a production camera-streaming service
- an OpenXR layer submission path
- a replacement for app-owned rendering, decode, texture import, or frame
  pairing

## Clean-Room And License Boundary

The working policy is conservative. External low-latency systems can inform
requirements, but public Rusty XR code must be authored independently under the
repo license.

Allowed in public Rusty XR:

- independent session and stream contracts
- codec, transport, and decoder capability descriptors
- timing and network-quality sample contracts
- security-policy and pairing-policy descriptors
- packet parser/writer code for Rusty XR-owned diagnostic formats
- optional adapter hooks that do not pull large SDKs into core

Not allowed in public Rusty XR core:

- Kyber source code, SDK headers, SDK examples, or binaries
- copied Kyber packet layouts or proprietary wire details
- bundled native media SDKs or codec payloads
- app package identities, signing material, generated captures, or device logs
- downstream visual-effect, scene, study, or product behavior

If an external low-latency SDK is evaluated later, it should be user-supplied,
separately licensed, and launched or probed as an external sidecar. Companion
tools may record version/license/path metadata and compare scorecards, but the
MIT core must not link or redistribute that SDK.

## Clean-Room Low-Latency Style

The useful architecture lesson from low-latency media systems is the style, not
the vendor protocol. Rusty XR should keep:

- small typed messages for session, stream, capability, timing, and security
  state
- explicit separation of control, media, telemetry, and XR-input planes
- bounded queues with visible drop reasons
- clock and timestamp domains named in manifests
- codec configuration and keyframe recovery represented as first-class state
- adapters that can be swapped without changing app-owned rendering
- scorecards that attribute source, encode, transport, decode, import,
  projected render, and OpenXR submit costs separately

That is the Kyber-inspired clean-room boundary for this repo: independent
contracts and metrics, no Kyber compatibility claim, no copied packet layout,
and no linked or redistributed external low-latency SDK payload.

## Relationship To The Broker

The existing broker model is the first contract home. It already contains
command envelopes, acknowledgements, stream manifests, sample headers, session
manifests, transport endpoints, timing stamps, heartbeat state, drop counters,
replay records, and synthetic stream payloads.

The transport architecture should extend that model only where the existing
contracts are too weak for negotiated low-latency sessions. A separate crate is
justified only if the transport surface grows beyond broker-neutral contracts.

## Planes

Control plane:

- capability discovery
- session offer/answer
- stream start/stop
- pairing approval and expiry
- command acknowledgements

Media plane:

- stream descriptors
- codec and stereo layout descriptors
- binary endpoint descriptors
- packet sequence, key-frame, and payload-size metadata
- compatibility with existing diagnostic framing while a v2 format stabilizes

Telemetry plane:

- source, encode, send, receive, decode, import, and submit timestamps
- queue depth, jitter, drop, and late-packet counters
- clock-sync quality
- per-session summaries

XR input/control plane:

- pose/control stream descriptors
- input sample timestamps
- reliability and ordering policy
- app-owned semantic mapping

## Session Lifecycle

Use an explicit state machine:

```text
Created -> Offered -> Accepted -> Starting -> Streaming -> Draining -> Closed
                                             -> Failed
```

Each session should track:

- session id
- client id
- selected transport
- stream descriptors
- security policy
- creation time and last heartbeat
- binary endpoint references
- per-stream counters
- close/failure reason

The first implementation can stay in memory. Persistent storage is not needed
until replay/export requirements are stable.

## Stream Descriptors

Stream descriptors should make transport assumptions explicit:

- stream id
- stream kind: media, audio, telemetry, control, XR input, bio, synthetic
- codec id or payload schema
- direction
- reliability and ordering
- stereo layout when relevant
- nominal rate, target latency, and max payload size
- selected endpoint or metadata-only status

Stream ids should be public, generic, and app-neutral. Downstream apps own
semantic names and private stream maps.

## Timing Model

Low-latency diagnostics are only useful when each stage is separately visible.
Use independent optional timestamps rather than one opaque "latency" number:

- source capture time
- encode start/done
- packet send/receive
- decode start/done
- texture import
- XR submit
- present estimate

Every timestamp should carry a domain such as elapsed realtime, camera sensor,
media PTS, Unix, OpenXR predicted display time, relay receive time, or unknown.
Timestamp-nearest stereo pairing across processes or devices should not become
an acceptance path until the sender and receiver manifest their domains and the
run records a clock-sync estimate or declares pairing as frame-order only.

Scorecards should separate transport, decode, texture import, projected render,
and OpenXR submit costs so renderer bottlenecks are not hidden by network
averages.

## Network Quality Model

Public contracts should model, not prematurely implement, recovery behavior:

- packet loss estimate
- late packet count
- decode gap count
- jitter-buffer depth
- target and observed latency
- repair attempt/success counters
- drop reason classification

Forward-error correction and retransmission strategies belong in later
adapters or clean-room implementations after the contract shape is stable.

## Security And Pairing

Loopback remains the default. Non-loopback media/control exposure must be
explicitly gated:

- disabled by default
- explicit runtime flag for LAN binds
- pairing token or operator approval
- expiration
- capability-scoped permissions
- status payloads that report active exposure

This keeps diagnostic LAN work possible without normalizing always-open control
or media endpoints.

## Adapter Boundaries

Core contracts may name transport families such as TCP, ADB-forwarded TCP,
QUIC, WebTransport, WebRTC diagnostic, or external sidecar. Implementations
belong in separate tools, examples, adapter crates, or downstream shells unless
they remain small and dependency-light.

Recommended order:

1. Existing TCP/ADB-forwarded diagnostic paths.
2. Rusty XR-owned v2 diagnostic packet format.
3. Companion inspection and scorecards.
4. Secure non-loopback pairing policy.
5. Mediated WebSocket/TLS relay experiments using the existing diagnostic
   framing.
6. Runtime H.264 media controls such as keyframe request and bitrate changes.
7. Optional WebRTC adapter after the current H.264 access units, projection
   metadata, and hardware-buffer decode path can be preserved.
8. Optional QUIC/WebTransport experiments after relay MVP measurements.
9. Optional external sidecar comparison lanes after license review.

The current Rusty XR-owned diagnostic video format is the bounded `RXYRVID1`
schema-3 stream framing used by the public broker example. Its fixed-width
big-endian header contains magic, binary schema version, codec id, width,
height, packet count, and a stream-header metadata byte count. Schema-3 streams
then write bounded UTF-8 JSON metadata before packet headers; live Quest stereo
streams use that metadata for per-eye projection bootstrap. Each v2/v3 packet
header carries presentation time, flags, payload size, source elapsed time, and
source Unix time before the encoded payload. This format is only a public
diagnostic contract for Rusty XR examples; it is not a vendor packet format or
an external SDK compatibility layer.

For the public Quest broker example, the first streaming-grade H.264 metadata
slice stays on Android platform MediaCodec/Camera2 APIs. The broker records
encoder output-format changes, SPS/PPS CSD, requested/applied CBR mode,
sync-frame request status, codec-config packet counts, video packet counts, and
Camera2 capture-start timestamps/frame numbers. Its live-bounded stream path
now drains MediaCodec output into a bounded in-memory queue while a separate
writer thread owns TCP writes, so writer stalls are measured as queue depth and
drop counters instead of directly blocking the codec drain loop. Decoder probes
and composite H.264 consumers request Android decoder low-latency mode
separately from encoder latency hints. Composite stereo H.264 diagnostics
support nearest-source-timestamp pairing and frame-order live pairing. The
validated fast renderer-parity profiles currently pin frame-order pairing so
the camera IDs, square broker frames, decode output mode, GPU import, and
projection shader are the only intended moving parts. Timestamp pairing remains
the next transport-grade mode once capture timestamps are stable across a
remote sender/receiver session. Future Rusty XR packet formats should make
codec-config packets first-class rather than relying on consumers to infer
SPS/PPS from ordinary video samples.

## Quest-To-Quest Online Adapter Rules

The current online Q2Q path should keep `RXYRVID1` as the diagnostic H.264
format for LAN, laptop-loop, and relay milestones. Add typed manifests around
it before replacing it:

- session id, role, direction, peer id, track id, stream id, and eye
- camera/source capability manifest
- timestamp-domain manifest
- encoder and decoder capability/state
- SPS/PPS and codec-config packet presence
- sync-frame request/recovery counters
- relay buffering and slow-peer close counters
- projection metadata envelope and source-eye mapping

A WebSocket/TLS relay is acceptable for the first online MVP only with hard
backpressure: max buffered bytes/packets, drop reasons, close reasons, and
write-stall timing must be visible in the scorecard. Do not use unbounded
message queues for media.

WebRTC should be an adapter, not a rewrite of the renderer. A valid WebRTC
lane must preserve Android MediaCodec decode, HardwareBuffer/AHardwareBuffer
handoff, Vulkan/OpenXR projected stereo draw, projection metadata delivery, and
scorecard visibility. The migration should parse `RXYRVID1`, extract H.264
access units/NAL units, packetize them for the WebRTC video track, and send
projection/timing metadata over a data channel. Do not put opaque `RXYRVID1`
packets directly into a WebRTC video track.

WebTransport is an investigation lane after the TLS relay MVP. Keep it only if
real headset and hotspot tests show lower buffering, bounded latency, or
cleaner multiplexing than the simpler relay.

## FFmpeg Boundary

FFmpeg may be useful as a user-supplied desktop sidecar for inspection,
preview decode, remuxing, thumbnails, saved stream analysis, or PC bridge
experiments. It should not become the Quest runtime path for the current Q2Q
work. Quest sender and receiver examples should continue to prefer Android
platform MediaCodec and hardware buffers before any bundled native codec stack
is considered.

Rusty XR core must not bundle or link FFmpeg, libx264, libx265, GStreamer,
WebRTC, NDI, or comparable native media payloads. If a companion or adapter
uses an external executable, it should record path, version, license class, and
hash/provenance metadata in its own release workflow.

## Validation Matrix

Unit and no-hardware tests:

- contract validation
- serde round trips
- invalid stream/session/security rejection
- packet parser/writer round trips
- malformed packet rejection
- session state transitions
- metric aggregation

Broker tests:

- broker starts with zero transport sessions
- capabilities are reported
- create/list/get/close works
- legacy status and stream-event paths still work
- existing diagnostic media paths are unchanged until explicitly migrated

Quest/client tests:

- loopback session
- bounded H.264 diagnostic stream under a session
- stereo stream with per-eye timing reports
- hardware-buffer import timing report
- frame-pair accepted/dropped metrics

LAN tests:

- LAN disabled by default
- LAN bind rejected without explicit flag
- pairing token/approval required
- token expiry works
- status reports active non-loopback exposure

Regression tests:

- existing diagnostic framing still works
- existing broker WebSocket commands still work
- existing video-lab manifest/sample/metric commands still work
- Companion inspection of existing H.264 payloads still works

## First Implementation Slice

The first code slice should be deliberately small:

1. Audit `rusty-xr-broker-model` against this document.
2. Add missing session offer/answer, security policy, timing sample, and
   network-quality sample types only if they are not already represented.
3. Add validation helpers and focused tests.
4. Add schema export only after the field names pass review.
5. Keep broker APK behavior unchanged until the contracts compile, test, and
   serialize cleanly.
