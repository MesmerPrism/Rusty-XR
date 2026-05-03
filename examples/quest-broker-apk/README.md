# Rusty XR Quest Broker APK

Public proof-of-concept for a minimal Quest sidecar broker. The broker is a
separate Android APK/service that stays focused on messages, status,
diagnostics, latency samples, LSL forwarding when the native dependency is
supplied, OSC transport probes, and generic stream-event publishing for
diagnostic adapters. It also exposes the first camera-projection metadata
provider shape so XR clients can discover a projection profile while keeping
actual rendering inside the active XR app.

This is not the final runtime architecture and does not own camera buffers,
depth textures, OpenXR frame timing, MediaProjection, H.264/H.265 transport,
or rendering. The broker reports those boundaries explicitly so clients do not
mistake metadata support for cross-app layer injection.

For the bounded app-camera H.264 side channel, the stream-start command returns
best-effort selected Camera2 projection metadata alongside the binary endpoint
when the broker has camera permission. A consuming XR app can use that metadata
to tag decoded hardware buffers with intrinsics and lens pose while still
owning decode, Vulkan import, projection, and OpenXR layer submission itself.

## Endpoints

- HTTP status: `http://127.0.0.1:8765/status`
- WebSocket samples/events: `ws://127.0.0.1:8765/rustyxr/v1/events`
- LSL stream when native LSL is packaged: `rusty_xr_broker_latency`
- OSC latency egress when enabled: `/rusty-xr/broker/latency`
- OSC control ingress when enabled: `/rusty-xr/drive/radius`
- Bio diagnostic stream IDs: `bio:polar_hr_rr`, `bio:polar_ecg`,
  `bio:polar_acc`
- Camera provider stream IDs: `camera_provider.status`,
  `camera_provider.projection_profile`, `camera_provider.visual_acceptance`
- Shell helper status stream ID: `shell_helper.status`
- Video-lab metric stream ID: `video_lab.metric_sample`
- Video-lab encoded stream manifest ID: `video_lab.encoded_stream_manifest`
- Video-lab encoded sample metadata ID: `video_lab.encoded_sample_metadata`

The status payload reports broker uptime, accepted sample counts, active
capabilities, stream descriptors, command counters, LSL availability, and OSC
ingress/egress settings. WebSocket clients can send legacy `status_request`,
`hello`, and `latency_sample` JSON messages. They can also use the first
versioned command envelope for status, capability, stream, subscription,
runtime OSC ingress configuration, generic stream publication, and console
requests. Camera-provider metadata and shell-helper status commands are also
available:

- `camera_provider.get_status`
- `camera_provider.get_projection_profile`
- `camera_provider.run_app_camera_probe`
- `camera_provider.start_app_camera_luma_stream`
- `camera_provider.start_app_camera_h264_stream`
- `camera_provider.run_app_camera_h264_decode_probe`
- `camera_provider.set_source_eye_mapping`
- `camera_provider.set_texture_transform`
- `camera_provider.record_visual_acceptance`
- `shell_helper.get_status`
- `shell_helper.report_status`
- `video_lab.get_status`
- `video_lab.register_encoded_stream_manifest`
- `video_lab.record_encoded_sample_metadata`
- `video_lab.record_metric_sample`

```json
{
  "type": "command",
  "schema": "rusty.xr.broker.command.v1",
  "request_id": "req-001",
  "client_id": "quest-client",
  "command": "subscribe",
  "params": {
    "stream": "osc:/rusty-xr/drive/radius"
  }
}
```

Command replies use `rusty.xr.broker.command_ack.v1`. Subscribed clients
receive generic `stream_event` messages such as:

```json
{
  "type": "stream_event",
  "schema": "rusty.xr.broker.stream_event.v1",
  "stream": "osc:/rusty-xr/drive/radius",
  "sequence_id": 42,
  "payload": {
    "value01": 0.75
  }
}
```

The legacy `osc_drive` broadcast remains in place for compatibility while
newer clients move to explicit subscriptions. The broker acknowledges accepted
latency samples and logs transport diagnostics to logcat with the
`RustyXrBroker` tag.

`configure_osc_ingress` can enable or replace the UDP OSC ingress listener
while the broker service is already running. `publish_stream_event` accepts a
stream id, sequence id, and JSON payload, then broadcasts the payload to clients
subscribed to that stream. The Companion `broker bio-simulate` command uses
that generic path to publish Polar-compatible standard Heart Rate Measurement
and Polar PMD ECG/ACC payloads. This broker path carries GATT-shaped diagnostic
payloads; it is not a Bluetooth peripheral advertiser.

The camera provider is a P0 metadata/config provider. It returns a public
projection profile with `requires_client_eye_views=true` and
`requires_client_layer_submission=true`. The active Unity or Rusty XR app must
own camera permission, texture import/decode, current eye views/FOV, shaders,
and OpenXR layer submission. The broker can run a bounded app-context Camera2
probe with `camera_provider.run_app_camera_probe`; that command uses the
broker APK's normal Android permission model to enumerate Camera2 metadata and
attempt one short YUV capture per selected camera ID. It records only metadata
and capture success state, not frame payloads. For a bounded payload-transport
diagnostic, `camera_provider.start_app_camera_luma_stream` captures a small
number of app-context `YUV_420_888` frames, copies only the luma plane, and
writes `raw_luma8` packets through the public `RXYRVID1` binary framing over
an ADB-forwarded TCP connection. The broker still sends only manifest, sample
metadata, and metric events through WebSocket JSON, and labels this path as a
probe rather than a production encoded-video or compositor provider. When the
next encoded diagnostic is needed, `camera_provider.start_app_camera_h264_stream`
routes app-context Camera2 frames directly into an Android platform
MediaCodec H.264 input surface, then writes encoded packets through the same
`RXYRVID1` framing with codec ID `1`. By default this remains a bounded
capture-then-write probe for regression stability. When clients pass
`live_stream=true`, the broker accepts the binary stream socket before Camera2
capture starts, drains encoder output directly to the stream, and writes
schema-2 packets with per-packet source timestamps. That proves
Camera2-to-encoder payload transport without bundling a codec library. The
follow-up
`camera_provider.run_app_camera_h264_decode_probe` command reuses that same
app-camera encoder path and consumes the resulting H.264 packets inside the
broker process with Android platform MediaCodec byte-buffer output. That
isolates decoder compatibility before a client texture path is attempted, but
texture import, eye views, and OpenXR layer submission still belong to the
active XR client.
When the
optional ADB shell helper reports bounded shell-visible camera metadata or
Camera2 open/capture
feasibility, the broker also summarizes those results in `cameraProvider` and
`projectionProfile` so clients can inspect candidate camera IDs,
pose/intrinsics availability, and the current evidence level without reading
raw helper diagnostics.

The app-context camera probe requires runtime camera permission. For manual
development runs, grant the public example permission before sending the probe:

```powershell
adb shell pm grant com.example.rustyxr.broker android.permission.CAMERA
adb shell pm grant com.example.rustyxr.broker horizonos.permission.HEADSET_CAMERA
```

The shell-helper status path is a control-plane placeholder for a separate
ADB-launched helper. A normal broker APK does not become Android `shell`.
Developer/operator tooling may launch a helper through authorized ADB and have
that helper report its UID, version, capabilities, heartbeat, optional bounded
codec diagnostics, shell-visible camera metadata, and Camera2 open/capture
feasibility through `shell_helper.report_status`. The source-only helper
example lives in `examples/quest-broker-shell-helper`.

`video_lab.register_encoded_stream_manifest` and
`video_lab.record_encoded_sample_metadata` define the control-plane shape for a
future encoded stream: session id, codec/MIME, dimensions, frame rate, payload
transport hint, sample sequence, key-frame flag, encoded byte count, and source
timestamps. `video_lab.record_metric_sample` records the timing/drop/queue
metrics that future encoded-video and texture-import experiments need.
High-rate payload bytes must use a binary transport rather than JSON WebSocket
command payloads. The app-camera luma probe uses that split today for bounded
raw-luma frames, and the app-camera H.264 probe uses Android's platform encoder
for bounded or live-bounded encoded packets. The app-camera H.264 decode probe
verifies platform decoder consumption with byte-buffer output only.
Decode-to-texture and XR layer submission remain separate client/provider work.

XR clients can bring the 2D broker console to the foreground by sending the
`open_ui` broker command. The console has Dashboard, Streams, Commands, and
Diagnostics pages plus a `Return to XR App` button. The button and the
`close_ui` broker command both finish only the broker console Activity while
leaving the broker foreground service running; they do not start or relaunch a
target app.

## Build

Build the APK from the public Rusty XR checkout:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-apk\tools\Build-QuestBrokerApk.ps1
```

Output:

```text
examples/quest-broker-apk/build/outputs/rusty-xr-quest-broker-debug.apk
```

APK bytes, local debug signing material, native build products, and diagnostic
captures are intentionally ignored by git.

## Optional LSL Packaging

The source includes a small JNI publisher for liblsl. Rusty XR does not vendor
or redistribute `liblsl.so` in this example. To build an LSL-capable broker APK,
provide a compliant Android `liblsl.so` and keep matching license notices with
your downstream distribution:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-apk\tools\Build-QuestBrokerApk.ps1 -LslAndroidLibraryPath <path-to-android-arm64-liblsl.so>
```

Without `liblsl.so`, the broker still builds, answers status requests, accepts
WebSocket latency samples, emits logcat diagnostics, and supports OSC ingress
and OSC egress.

## Launch Profiles

The companion catalog is in:

```text
examples/quest-broker-apk/catalog/rusty-xr-quest-broker.catalog.json
```

Use `broker-latency-websocket-lsl` for the basic localhost API and optional LSL
path. Use `broker-osc-drive-ingress` when a laptop sends OSC control values to
the headset and a Unity-side client consumes the resulting WebSocket
`osc_drive` or subscribed `stream_event` events.

Runtime OSC ingress can also be configured over the broker WebSocket command
API, which lets comparison tools start the listener without restarting the
broker Activity.

Direct ADB launch for OSC ingress:

```powershell
adb shell am start -n com.example.rustyxr.broker/.MainActivity `
  --ez rustyxr.oscIngressEnabled true `
  --ei rustyxr.oscIngressPort 9000 `
  --es rustyxr.oscIngressAddress /rusty-xr/drive/radius
```

Send a probe value from the companion CLI:

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- osc send --host <quest-lan-ip> --port 9000 --address /rusty-xr/drive/radius --arg float:0.75
```

After forwarding the broker TCP endpoint with the companion CLI, a Rust-side
probe can exercise the same status, command, stream, subscription, and latency
sample path:

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- broker forward --serial <serial>
cargo run -p rusty-xr-broker-client-probe -- status
cargo run -p rusty-xr-broker-client-probe -- streams
cargo run -p rusty-xr-broker-client-probe -- camera-provider
cargo run -p rusty-xr-broker-client-probe -- projection-profile
cargo run -p rusty-xr-broker-client-probe -- app-camera-probe
cargo run -p rusty-xr-broker-client-probe -- shell-helper-status
cargo run -p rusty-xr-broker-client-probe -- sample --subscribe
cargo run -p rusty-xr-broker-client-probe -- open-ui
cargo run -p rusty-xr-broker-client-probe -- close-ui
```

Companion-side diagnostics can compare direct OSC with broker-routed OSC and
publish Polar-compatible bio payloads through the same broker stream-event
surface:

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- broker compare --quest-host <quest-lan-ip> --serial <serial> --out .\artifacts\broker-compare --json
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- broker bio-simulate --serial <serial> --out .\artifacts\broker-bio-sim --json
```

## Validation Status

This proof of concept has been validated with a Unity client running on Quest:
the Unity app connected to the broker over localhost WebSocket, sent latency
samples for broker-side LSL forwarding, and consumed OSC-driven broker events
to drive a live scene parameter. A broker build with a user-supplied Android
`liblsl.so` has also been validated by resolving `rusty_xr_broker_latency` from
Windows and pulling forwarded string samples. The public Rust client probe now
covers the same broker contract from a Rust-native code path.
