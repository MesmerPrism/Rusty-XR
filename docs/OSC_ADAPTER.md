# OSC Adapter

Rusty XR includes a small Open Sound Control utility crate,
`rusty-xr-osc`, for app-neutral live control and sensor ingress.

The crate provides:

- OSC message and bundle data models.
- Encoding and decoding for common OSC 1.0 argument types.
- A standard-library UDP socket helper for desktop and Android app shells.
- Optional `serde` support for decoded packet records and endpoint status.

It does not define app-specific address trees, sensor semantics, renderer
behavior, Android activity lifecycle, or visual effects. A downstream app should
map addresses such as `/runtime/command`, `/sensor/pose`, or
`/operator/probe` into its own typed state.

## Quest Example Listener

The public Quest composite-layer example has an explicit `osc-udp-listener`
runtime profile. It starts a UDP listener, writes packet summaries to logcat,
and draws a headset diagnostics panel built from the `rusty-xr-debug-canvas`
logical layout primitives. The panel shows listener status, bind/local address,
packet count, last peer, byte count, last packet summary, and any receiver
error. The profile keeps camera, MediaProjection, and environment depth off so
OSC transport can be tested by itself.

The companion `osc-udp-listener-no-overlay` runtime profile starts the same UDP
listener but leaves the diagnostics panel disabled. Use it as an A/B isolation
profile when a live test needs to separate OSC ingress cost from headset canvas
rendering cost.

The panel is now controlled through the generic diagnostic HUD state in
`rusty-xr-debug-canvas`, not through OSC-specific state. In APK shells, ADB,
controller, LSL, OSC, or app code can all map into the same command vocabulary:
`show`, `hide`, `toggle`, `next`, `previous`, and `page:N`. The Quest example
currently exposes the ADB/runtime-config path:

```powershell
adb shell am start -n com.example.rustyxr.composite/.CompositeLayerActivity --ez rustyxr.diagnosticHudVisible false
adb shell am start -n com.example.rustyxr.composite/.CompositeLayerActivity --es rustyxr.diagnosticHudCommand toggle
```

Launch the profile through Rusty XR Companion Apps:

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- catalog launch --path .\examples\quest-composite-layer-apk\catalog\rusty-xr-quest-composite-layer.catalog.json --app rusty-xr-quest-composite-layer --serial <serial> --runtime-profile osc-udp-listener
```

Send a probe packet from the companion CLI over the local network:

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- osc send --host <quest-lan-ip> --port 9000 --address /rusty-xr/probe --arg string:hello
```

Then inspect logcat for:

```text
Rusty XR OSC packet received
```

In headset, the overlay should switch from `STARTING` to `LISTENING`, then
increment `PACKETS` and update the `LAST PACKET` section after each probe
message.

The headset overlay renderer batches the canvas draw list into one instanced
Vulkan draw using a small host-updated storage buffer. Avoid adding per-glyph
or per-primitive draw calls in this path; live transport tests should remain
limited by the app under test, not the diagnostics panel.

For headset readability, the diagnostics panel is rendered as a shared
head-anchored stereo surface instead of independent per-eye screen-space text.
See
[DIAGNOSTIC_HUD_STEREO_RENDERING.md](DIAGNOSTIC_HUD_STEREO_RENDERING.md)
for the rendering options and tradeoffs.

OSC commonly uses UDP. ADB `forward` and `reverse` are TCP-oriented, so use the
headset LAN IP for this probe unless the target app implements a separate TCP
bridge.

## Broker OSC Drive Proof

The public broker APK example adds a non-rendering OSC ingress path for
sidecar validation. The `broker-osc-drive-ingress` runtime profile listens for
`/rusty-xr/drive/radius` on UDP port `9000` and rebroadcasts accepted values to
localhost WebSocket clients as:

```json
{
  "type": "osc_drive",
  "schema": "rusty.xr.osc.drive.v1",
  "address": "/rusty-xr/drive/radius",
  "value": 0.75,
  "sequence_id": 1
}
```

Launch and probe it through the companion CLI:

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- catalog launch --path .\examples\quest-broker-apk\catalog\rusty-xr-quest-broker.catalog.json --app rusty-xr-quest-broker --serial <serial> --runtime-profile broker-osc-drive-ingress
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- osc send --host <quest-lan-ip> --port 9000 --address /rusty-xr/drive/radius --arg float:0.75
```

This path has been validated with a Unity client on Quest consuming broker
WebSocket events to drive a live scene parameter. A dedicated public Unity
example will be added separately; the current public surface is the broker APK
source, catalog, and companion workflow.

## Local Rust Probe

Without headset hardware, the crate example can test loopback transport:

```powershell
cargo run -p rusty-xr-osc --example osc_udp_probe -- listen --bind 127.0.0.1:9000 --count 1
cargo run -p rusty-xr-osc --example osc_udp_probe -- send --to 127.0.0.1:9000 --address /rusty-xr/probe --arg string:hello
```

Use this only as a transport probe. Stable public schemas for specific sensor
families should be added separately after their address and value contracts are
clear.
