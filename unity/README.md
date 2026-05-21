# Rusty XR Unity Packages

This folder contains source-only Unity Package Manager packages for downstream
Unity projects. Packages are kept separate from Quest APK examples so Unity
projects can consume broker contracts without copying example-app code.

Install from Git with a package path, for example:

```json
"com.rustyxr.gargoyle": "https://github.com/MesmerPrism/Rusty-XR.git?path=/unity/com.rustyxr.gargoyle"
```

Current packages:

- `com.rustyxr.gargoyle`: broker/Gargoyle WebSocket client, command envelopes,
  stream events, routing, and lightweight samples.
- `com.rustyxr.gargoyle.video.android`: optional video command, telemetry, and
  diagnostic `RXYRVID1` H.264 packet receiver components. It depends on the
  core Gargoyle package.

High-rate encoded media payloads should not travel through the generic JSON
WebSocket channel. Use the core package to negotiate and observe streams, then
use a media-specific package for binary receive/decode paths.
