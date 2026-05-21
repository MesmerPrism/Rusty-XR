# Rusty XR Gargoyle Unity Package

`com.rustyxr.gargoyle` is the small Unity-side client for the Rusty XR
Gargoyle broker identity. It owns broker communication only: WebSocket
connection, client hello, commands, command acknowledgements, stream-event
parsing, and scene-local routing.

It does not include direct OSC, direct LSL, BLE, camera, MediaCodec, WebRTC,
or Unity texture ownership. Those belong in examples or optional packages.

## Install

Use Unity Package Manager with a Git URL:

```json
"com.rustyxr.gargoyle": "https://github.com/MesmerPrism/Rusty-XR.git?path=/unity/com.rustyxr.gargoyle"
```

For local development from a sibling checkout:

```json
"com.rustyxr.gargoyle": "file:../Rusty-XR/unity/com.rustyxr.gargoyle"
```

## Runtime Shape

Add a `GargoyleClient` component to a scene object. By default it connects to:

```text
ws://127.0.0.1:8765/rustyxr/v1/events
```

Then either subscribe from code:

```csharp
client.Subscribe("synthetic:wave");
```

or add a `GargoyleStreamRouter` with `GargoyleStreamReceiver` components.

## Package Boundary

Keep project-specific scene behavior outside this package. A Unity project
should decide what a stream means for objects, trials, visuals, logging, and
participant flow. Gargoyle exposes declared edges: streams, commands, clocks,
diagnostics, source labels, and consented data paths.
