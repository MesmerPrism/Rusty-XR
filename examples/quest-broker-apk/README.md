# Rusty XR Quest Broker APK

Public proof-of-concept for a minimal Quest sidecar broker. The broker is a
separate Android APK/service that stays focused on messages, status,
diagnostics, latency samples, LSL forwarding when the native dependency is
supplied, and OSC transport probes.

This is not the final runtime architecture and does not touch camera, depth,
textures, OpenXR frame timing, MediaProjection, H.264/H.265, or rendering.

## Endpoints

- HTTP status: `http://127.0.0.1:8765/status`
- WebSocket samples/events: `ws://127.0.0.1:8765/rustyxr/v1/events`
- LSL stream when native LSL is packaged: `rusty_xr_broker_latency`
- OSC latency egress when enabled: `/rusty-xr/broker/latency`
- OSC control ingress when enabled: `/rusty-xr/drive/radius`

The status payload reports broker uptime, accepted sample counts, active
capabilities, LSL availability, and OSC ingress/egress settings. WebSocket
clients can send `status_request`, `hello`, and `latency_sample` JSON messages.
The broker acknowledges accepted samples and logs transport diagnostics to
logcat with the `RustyXrBroker` tag.

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
`osc_drive` events.

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

## Validation Status

This proof of concept has been validated with a Unity client running on Quest:
the Unity app connected to the broker over localhost WebSocket, sent latency
samples for broker-side LSL forwarding, and consumed OSC-driven broker events
to drive a live scene parameter. A dedicated public Unity example project is
planned for a later iteration; this repository currently publishes the broker
APK source, catalog, and companion workflow only.
