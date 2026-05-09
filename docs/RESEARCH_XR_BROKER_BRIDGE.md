# Research XR Broker Bridge

Status: public-safe interoperability analysis and example proposal. This is not
an endorsement, affiliation, or compatibility claim by EDIA, RCAS, or Rusty XR.
It is a source-only architecture page for deciding where broker contracts and
thin adapters should live.

## Why This Exists

Unity-centered research XR toolboxes and a sidecar broker solve adjacent
problems. A Unity experiment framework is the right place for participant flow,
scene state, trial semantics, and UXF-style logging. A broker is the right place
for stream routing, timing normalization, drop/jitter visibility, replay,
protocol fan-out, and operator diagnostics.

The practical goal is a thin bridge:

```text
Unity / Godot / Unreal / Rust client
  -> app-local acquisition, scene meaning, and control hooks

Rusty XR broker
  -> stream registry, timing, validation, processing, replay, LSL/OSC/WebSocket fan-out

Research XR toolbox
  -> experiment semantics, trial flow, Unity workflow, and analysis integration
```

The maintainer-facing collaboration track is kept in
[EDIA_COLLABORATION_TRACK.md](EDIA_COLLABORATION_TRACK.md).
The minimal Unity-side adapter contract is kept in
[UNITY_BROKER_ADAPTER_CONTRACT.md](UNITY_BROKER_ADAPTER_CONTRACT.md).

## Complementary Roles

EDIA describes itself as a Unity XR toolbox for research, with modules for
experiment structure, config files, eye tracking, remote control and streaming,
logging, and Lab Streaming Layer synchronization. EDIA RCAS is the remote
control and streaming module. Its public README frames the controller side as
the experimenter interface and the executer side as the VR-device experiment
runtime.

Rusty XR broker is intentionally narrower at the scene layer. It should not
become a Unity experiment framework. It should provide typed stream contracts,
status, timing, transport negotiation, recording, replay, and protocol adapters
that more than one engine can consume.

| Area | Keep In Unity / Research Toolbox | Move Or Mirror In Broker |
| --- | --- | --- |
| Experiment structure | session, block, trial, scene flow | broker session metadata and external markers |
| Remote control | experiment-specific commands | generic command envelope, ack, auth, status |
| Eye data | SDK bridge and scene-local gaze hits | normalized streams, validation, replay, LSL, processors |
| Preview video | operator preview UI | stream manifests, sample metadata, drop/jitter counters |
| Logging | participant-facing experiment logs | transport logs, JSONL/CSV export, replay fixtures |

## Transport Lesson

EDIA RCAS is useful as a design reference because it separates reliable control
from low-latency stream traffic. Rusty XR should keep that split but avoid
copying RCAS-specific assumptions into the broker model.

Rusty XR target shape:

```text
Reliable control path:
  WebSocket or TCP-like broker control channel

Loss-tolerant data path:
  UDP lane negotiated by reliable control

Development fallback:
  WebSocket text/binary stream for localhost, ADB-forwarded, or NAT-hostile runs
```

The broker stream manifest must make loss visible. High-rate streams should
advertise sequence numbers, monotonic timestamps, source timestamps where
available, payload schema, reliability class, ordered/unordered behavior,
maximum datagram bytes, drop counters, late packet counters, heartbeat state,
and any required auth/session token.

## Example Stream Manifest

```json
{
  "manifest_schema": "rusty.xr.broker.stream_manifest.v1",
  "stream_id": "eye.screen.gaze_point",
  "session_id": "session-001",
  "source_id": "desktop-tracker",
  "payload_kind": "Json",
  "payload_schema": "rusty.xr.eye.screen.gaze_point.v1",
  "sequence_start": 0,
  "recommended_rate_hz": 120.0,
  "max_datagram_bytes": 1200,
  "reliability": "LossTolerant",
  "ordered": false,
  "endpoint": {
    "transport": "Udp",
    "host": "127.0.0.1",
    "port": 47777,
    "path": null,
    "channel_id": null,
    "max_datagram_bytes": 1200,
    "auth_required": false
  },
  "heartbeat": {
    "last_heartbeat_elapsed_ns": null,
    "timeout_after_ns": 1000000000
  },
  "drop_counters": {
    "received_samples": 0,
    "emitted_samples": 0,
    "dropped_samples": 0,
    "late_samples": 0,
    "duplicate_samples": 0,
    "out_of_order_samples": 0,
    "queue_overflow_count": 0
  }
}
```

This example deliberately uses a screen-space eye stream. A desktop tracker can
validate timing, recording, replay, LSL forwarding, and screen-space AOI logic,
but it does not validate headset-local gaze rays or XR scene-hit semantics.
The public Rusty XR core therefore keeps screen-space gaze, XR gaze-ray, and
derived eye-processor contracts separate.

## Replay Fixtures

Desktop-only replay fixtures now live under
[`fixtures/replay`](../fixtures/replay):

- `synthetic-broker-wave.session.json`
- `synthetic-broker-wave.jsonl`
- `synthetic-eye-screen-gaze.session.json`
- `synthetic-eye-screen-gaze.jsonl`

The broker fixture carries deterministic `synthetic:wave` records. The eye
fixture carries broker replay records whose payloads are
`eye.screen.gaze_point` samples, including one explicit blink/dropout sample.
These files are intentionally metadata-only and do not require Unity, a headset,
a tracker SDK, or a broker runtime.

Validate them with:

```powershell
python tools\replay\check_replay_fixtures.py --fixtures fixtures\replay
```

The checker verifies the session manifest, stream manifest, replay record,
sample header, sequence, timing, and payload consistency for each JSONL line.

## Thin Adapter Duties

A Unity bridge should stay small:

- connect to the broker
- send a client hello and status
- subscribe to selected stream manifests
- forward generic remote-control commands into local Unity events
- publish trial or scene markers back to the broker
- optionally display a broker-provided diagnostic or operator preview stream

It should not become a second experiment framework, own headset-specific
rendering for the broker, or hide LAN control without explicit safety gates.
The source-level contract for this first Unity adapter is tracked in
[UNITY_BROKER_ADAPTER_CONTRACT.md](UNITY_BROKER_ADAPTER_CONTRACT.md).

## License And Attribution

License posture checked on May 9, 2026:

- EDIA Core GitHub page and license file report MIT license:
  <https://github.com/edia-toolbox/edia_core>
- EDIA RCAS GitHub page and license file report MIT license:
  <https://github.com/edia-toolbox/edia_rcas>
- EDIA Eye GitHub page and license file report MIT license:
  <https://github.com/edia-toolbox/edia_eye>
- Rusty XR is MIT licensed:
  <https://github.com/MesmerPrism/Rusty-XR>

This page does not copy EDIA source code. If a future adapter copies or ports
any code, recheck upstream license files at that time, preserve required MIT
copyright notices, and add third-party notices to any distributed artifact.

## Non-Goals

- No claim that Rusty XR replaces EDIA or UXF.
- No unauthenticated LAN control by default.
- No bundled proprietary SDKs, native eye-tracker SDKs, `liblsl`, FFmpeg,
  WebRTC, NDI, or codec payloads in public core.
- No public eye-tracker forwarding or logging provider without a current
  license and field-of-use review for the target hardware and SDK.
- No private app package IDs, generated captures, validation logs, or device
  artifacts in public docs.

## Next Implementation Steps

1. Keep broker protocol, stream-manifest, replay, and synthetic stream
   contracts in public Rusty XR core.
2. Keep screen-space eye-data contracts separate from headset-local gaze-ray
   contracts.
3. Expand broker processors and JSONL replay fixtures before device-specific
   providers.
4. Prototype optional source-only providers and Unity bridges only after the
   contracts pass serialization, schema, test, and boundary scans and the
   provider license path is explicit.
