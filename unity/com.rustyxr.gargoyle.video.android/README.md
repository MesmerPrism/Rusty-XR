# Rusty XR Gargoyle Video Android

`com.rustyxr.gargoyle.video.android` is the optional video package for Unity
projects that need to talk to Gargoyle video features.

This package currently provides:

- video-lab command helpers;
- synthetic and app-camera H.264 stream start command builders;
- keyframe and bitrate command helpers;
- telemetry receivers for Gargoyle video stream manifests, sample metadata,
  and metric samples;
- a diagnostic `RXYRVID1` H.264 TCP receiver that parses stream headers and
  packets for measurement or future decoder handoff.

It does not yet include a native Android `MediaCodec -> SurfaceTexture ->
Unity texture` bridge. That bridge should remain in this optional package, not
in `com.rustyxr.gargoyle`, because it owns Android media lifecycle, large
payload flow, and native texture concerns.

## Install

Install the core package first, then this package:

```json
"com.rustyxr.gargoyle": "https://github.com/MesmerPrism/Rusty-XR.git?path=/unity/com.rustyxr.gargoyle",
"com.rustyxr.gargoyle.video.android": "https://github.com/MesmerPrism/Rusty-XR.git?path=/unity/com.rustyxr.gargoyle.video.android"
```

For local development from a sibling checkout:

```json
"com.rustyxr.gargoyle": "file:../Rusty-XR/unity/com.rustyxr.gargoyle",
"com.rustyxr.gargoyle.video.android": "file:../Rusty-XR/unity/com.rustyxr.gargoyle.video.android"
```

## First Integration Step

Use `GargoyleVideoController` with a `GargoyleClient` to request
`media.start_synthetic_h264_stream`. Subscribe to video telemetry streams and
show the status in your scene before attempting live camera streams or native
texture decode.
