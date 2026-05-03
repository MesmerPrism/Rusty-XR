# Rusty XR Quest Broker APK

Public proof-of-concept for a minimal Quest sidecar broker. The broker is a
separate Android APK/service that stays focused on messages, status,
diagnostics, latency samples, LSL forwarding when the native dependency is
supplied, OSC transport probes, and generic stream-event publishing for
diagnostic adapters.

This is not the final runtime architecture and does not touch camera, depth,
textures, OpenXR frame timing, MediaProjection, H.264/H.265, or rendering.

## Endpoints

- HTTP status: `http://127.0.0.1:8765/status`
- WebSocket samples/events: `ws://127.0.0.1:8765/rustyxr/v1/events`
- LSL stream when native LSL is packaged: `rusty_xr_broker_latency`
- OSC latency egress when enabled: `/rusty-xr/broker/latency`
- OSC control ingress when enabled: `/rusty-xr/drive/radius`
- Bio diagnostic stream IDs: `bio:polar_hr_rr`, `bio:polar_ecg`,
  `bio:polar_acc`

The status payload reports broker uptime, accepted sample counts, active
capabilities, stream descriptors, command counters, LSL availability, and OSC
ingress/egress settings. WebSocket clients can send legacy `status_request`,
`hello`, and `latency_sample` JSON messages. They can also use the first
versioned command envelope for status, capability, stream, subscription,
runtime OSC ingress configuration, generic stream publication, and console
requests:

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
