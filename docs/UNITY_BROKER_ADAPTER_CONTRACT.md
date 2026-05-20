# Unity Broker Adapter Contract

Status: public source-only adapter contract. This is a design target for Unity
examples and downstream Unity projects; it is not a Unity package release,
runtime compatibility claim, or endorsement by any external toolbox.

The canonical public Unity comparison target is tracked in
[Unity example integration](UNITY_EXAMPLE_INTEGRATION.md).

## Purpose

A Unity broker adapter should let a Unity scene exchange typed Rusty XR broker
messages without becoming a second experiment framework. Unity remains the
owner of scene semantics, participant flow, object meaning, input policy, and
package/build settings. The broker side owns stream manifests, sample headers,
command envelopes, replay records, diagnostics, and optional protocol fan-out.

The first adapter should prove these paths with synthetic data:

- broker client hello
- status request
- stream subscription
- stream event parse
- replay-record parse into the same in-memory event shape
- drive-signal receiver
- screen-gaze receiver
- adapter-local routing counters

## Minimal Runtime Components

| Component | Responsibility |
| --- | --- |
| Broker client | WebSocket connection, hello, status request, subscribe/unsubscribe, reconnect status |
| Protocol parser | JSON envelope parsing for commands, acknowledgements, stream events, and replay records |
| Event router | Fan out parsed stream events to registered Unity receivers |
| Drive receiver | Convert `osc:/rusty-xr/drive/radius` or `synthetic:wave` values into a normalized drive value |
| Screen-gaze receiver | Convert `eye.screen.gaze_point` payloads into normalized screen coordinates and validity state |
| Optional visualizer | Render a small local marker from normalized screen gaze for synthetic demos |

These components should be plain C# / Unity scripts with no native plugin,
tracker SDK, headset-specific API, or Unity package-manager mutation in the
first pass.

## Accepted Input Shapes

Live stream event:

```json
{
  "type": "stream_event",
  "schema": "rusty.xr.broker.stream_event.v1",
  "stream": "eye.screen.gaze_point",
  "subscription_id": "sub-eye",
  "header": {
    "schema": "rusty.xr.broker.stream_sample_header.v1",
    "stream_id": "eye.screen.gaze_point",
    "session_id": "session-001",
    "source_id": "synthetic-eye-provider",
    "payload_kind": "Json",
    "payload_schema": "rusty.xr.eye.screen.gaze_point.v1",
    "sequence_number": 5,
    "broker_time_elapsed_ns": 55555555,
    "broker_time_unix_ns": null,
    "source_time_ns": 55555555,
    "source_time_unix_ns": null,
    "dropped_before_sample": 0,
    "late_before_sample": 0
  },
  "payload": {
    "schema": "rusty.xr.eye.screen.gaze_point.v1",
    "base": {
      "provider_id": "synthetic-eye-provider",
      "source_device_id": "desktop-eye-source",
      "sequence_number": 5,
      "sample_time_ns": 55555555,
      "broker_receive_time_ns": 55555555,
      "validity": {
        "sample_valid": false,
        "left_valid": false,
        "right_valid": false,
        "blink": true,
        "tracking_lost": true
      },
      "confidence": 0.0,
      "eye": "Combined",
      "coordinate_space": "ScreenNormalized"
    },
    "display_id": "primary-display",
    "normalized_point": {
      "x": 0.5,
      "y": 0.5
    },
    "screen_pixel": null,
    "pupil_diameter_mm": null
  }
}
```

Replay record:

```json
{
  "type": "replay_record",
  "schema": "rusty.xr.broker.replay_record.v1",
  "session_id": "synthetic-eye-screen-gaze-session",
  "stream": "eye.screen.gaze_point",
  "header": {},
  "payload": {}
}
```

Replay records should normalize into the same in-memory Unity event shape as
live stream events. That lets Unity edit-mode tests consume public fixtures
without requiring a broker runtime.

## Required Validation Behavior

The adapter should reject or ignore:

- unknown envelope types
- unknown schema ids on required envelopes
- empty stream ids
- mismatched stream ids between envelope and sample header
- non-JSON payloads in the first pass
- payload schemas that a receiver does not explicitly support
- non-finite normalized coordinates

The adapter may clamp screen-gaze coordinates to `[0, 1]` for visualization,
but it should preserve the sample validity flags. An invalid blink/dropout
sample should still be delivered to the screen-gaze receiver as an invalid
sample so downstream UI can distinguish "no data" from "not routed."

## Desktop Validation

Desktop-only validation should cover:

```powershell
python tools\replay\check_replay_fixtures.py --fixtures fixtures\replay
cargo test -p rusty-xr-broker-model -p rusty-xr-eye-model --features serde
```

Unity examples should also run edit-mode tests against fixture-equivalent JSON
lines. Those tests should not require a headset, native tracker SDK, broker
runtime, Android build, or Unity play mode.

## Side-Effect Boundary

The minimal adapter should not:

- install Unity packages
- modify scenes automatically outside an explicit editor command
- launch or stop headset apps
- access headset cameras or native eye trackers
- write participant/session data
- emit local diagnostics outside ignored output folders
- claim compatibility with a research toolbox until maintainers review the
  bridge boundary

Adapter installation, sample import, build, device launch, and live capture are
separate operations and should be planned behind explicit user intent.

