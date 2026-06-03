# Broker Stereo Combine Plan - 2026-06-03

This note records the Makepad camera matrix result and the implementation
decision for a broker-side stereo-combined H.264 stream. It intentionally keeps
large visual artifacts, local machine paths, and device-specific logs out of the
repository; those remain local run artifacts.

## Matrix Result

The 2026-06-03 Makepad matrix covered the orthogonal direct/broker and YUV/HWB
axes with `target-local-raster`, 72 Hz app cadence, render scale `0.90`, CPU/GPU
`4/4`, and 90 second fixed sample windows.

| Lane | Gate | Visual | Notable result |
| --- | --- | --- | --- |
| Direct YUV | ok | 6 unique screenshots, not fallback green | Cleanest baseline; no stale rows and lowest measured XR GPU time. |
| Direct HWB | failed stale gate | 6 unique screenshots, not fallback green | Live frames reached screen, but recent VrApi stale remained positive. |
| Broker YUV | ok | 6 unique screenshots, not fallback green | Broker camera `50/51` headers and pose-matched adoption were present. |
| Broker HWB | ok | 6 unique screenshots, not fallback green | Broker HWB decode produced hardware-buffer frames and pose-matched adoption. |

The broker paths are now functionally orthogonal to the decode output mode:
`cpu-yuv` and `hardware-buffer` both run against broker camera `50/51`.

## Why A Single Stream Is Attractive

A side-by-side stereo stream would make the pair atomic at the transport layer:
one stream header, one decoder timeline, one presentation timestamp, and one
Makepad video texture update. That directly addresses left/right drift classes
that can appear when two live streams decode and update independently.

It could reduce per-stream overhead in the Makepad app:

- one TCP reader instead of two;
- one MediaCodec decoder instance instead of two;
- one hardware-buffer import path instead of two for HWB decode;
- one stream metadata header carrying both eye rects and camera IDs.

The total image area does not go away. A `1280x1280` per-eye pair becomes a
`2560x1280` atlas, so decode bandwidth, sampling bandwidth, and encoded bitrate
remain roughly proportional to the same total stereo pixels.

## CPU Combine Is Not The Right Implementation

The broker's current camera H.264 path is efficient because Camera2 writes
directly into a MediaCodec encoder input surface. A CPU combiner would replace
that with two CPU-visible `YUV_420_888` camera readers, frame pairing, a
side-by-side YUV copy, and a byte-buffer encoder input.

At `1280x1280` per eye, one combined YUV420 atlas is about 4.9 MB. At 50 Hz,
the broker would need to move roughly 245 MB/s just for the packed output
frames, before accounting for source-plane reads, pixel-stride conversion,
queueing, and encoder input overhead. That would likely erase the benefit of
removing one decoder in the Makepad app and could make thermal or stale-frame
behavior worse.

Decision: do not add a CPU-copy stereo-combine lane as the performance path.

## Recommended Implementation

The version worth implementing is a broker GPU compositor feeding one H.264
encoder input surface.

1. Open camera `50` and camera `51` as separate Camera2 outputs to two
   `SurfaceTexture` or equivalent external texture inputs.
2. Run a broker-local EGL/GLES compositor at the requested camera cadence.
3. Pair the latest left/right camera frames by timestamp before drawing.
4. Render both textures into one MediaCodec encoder input surface using a
   side-by-side atlas layout:
   - left rect: `x=0.0, y=0.0, w=0.5, h=1.0`;
   - right rect: `x=0.5, y=0.0, w=0.5, h=1.0`.
5. Emit one H.264 stream header with stereo atlas metadata:
   - `stereoLayout=side-by-side`;
   - `leftCameraId=50`;
   - `rightCameraId=51`;
   - `leftAtlasUvRect=0,0,0.5,1`;
   - `rightAtlasUvRect=0.5,0,0.5,1`;
   - per-eye camera projection metadata and target footprints;
   - paired camera timestamps and pair delta telemetry.
6. Makepad consumes one broker H.264 stream and samples atlas UVs per eye in
   target-local mode:
   - `leftSampleUv = leftAtlasUvRect.xy + sourceUv * leftAtlasUvRect.zw`;
   - `rightSampleUv = rightAtlasUvRect.xy + sourceUv * rightAtlasUvRect.zw`;
   - bind the same decoded texture or YUV plane set to both eye paths;
   - mark `pairedLeftRightCameraFrames=true` because the stream packet is the
     pair authority.

## Expected Payoff By Lane

| Lane | Expected benefit | Risk |
| --- | --- | --- |
| Broker HWB | Moderate. One decoder and one hardware-buffer import/update path can reduce Makepad-side event/import churn and gives the strongest sync guarantee. | A broker GPU pass at 50 Hz may offset the decoder/import savings. Needs measurement. |
| Broker YUV | Low to moderate. One decoder callback and one YUV plane set can reduce Makepad event churn, but the app still uploads/samples the same total stereo pixels. | If the atlas reaches Makepad as CPU YUV, copy/upload pressure remains substantial. |
| Direct HWB/YUV | None for direct mode. This is a broker transport design and should not affect direct Camera2 acquisition. | N/A |

## Validation Gate

A future implementation should add a fifth experimental matrix lane,
`broker-h264-stereo-atlas`, and compare it against broker YUV and broker HWB:

- `freshnessStatus=ok`;
- `metaPerfStaleStatus=ok`;
- six unique screenshots and no fallback-green classification;
- stream header reports `stereoLayout=side-by-side` and camera `50/51`;
- pair delta telemetry reports bounded left/right timestamp difference;
- Makepad cadence reports one broker stream with paired camera frames;
- XR GPU time and VrApi stale must not regress versus broker HWB at the same
  render scale and performance profile.

Only promote the atlas path if it is at least as stable as broker HWB and shows
measurable Makepad-side overhead reduction or a clear visual alignment benefit
under head motion.
