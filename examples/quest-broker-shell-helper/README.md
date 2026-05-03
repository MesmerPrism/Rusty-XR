# Rusty XR Broker Shell Helper

This is a source-only Developer Mode helper for the Quest broker example. It is
not an APK and it is not installed as an app. The helper is pushed to
`/data/local/tmp` and launched with `adb shell app_process`, so it runs with the
Android `shell` UID when the user has enabled and authorized ADB debugging.

The first implementation is intentionally small: it connects to the broker's
localhost WebSocket endpoint and sends `shell_helper.report_status` with its
reported UID, helper version, and basic diagnostic capabilities. With
`--probe-codecs`, it also reports a bounded Android MediaCodec summary for
H.264, H.265, and AV1 codecs so later encoded-video work can choose a platform
codec path before any frame transport is attempted. With
`--probe-cameras`, it runs a bounded `dumpsys media.camera` probe as Android
`shell` and reports parsed camera counts, API1 mappings, per-device lens
pose/intrinsics, FPS rows, and stream-configuration rows into broker status.
With `--probe-camera-open`, it attempts bounded Camera2 open plus a tiny
YUV_420_888 one-frame capture through the shell-launched helper and reports
success/failure per attempted Camera2 id; add `--camera-open-id <id>` to limit
the probe to one source. With
`--emit-synthetic-video-metadata`, it registers a metadata-only synthetic H.264
stream and sends a bounded set of encoded-sample metadata events through the
broker video-lab commands. With `--emit-synthetic-video-binary`, it also opens
a bounded device-local TCP stream for deterministic synthetic encoded packets;
use `adb forward` or the Companion `binary-probe` command to receive that stream
on the host. With `--emit-mediacodec-synthetic-video`, it encodes a tiny
synthetic Surface source through Android MediaCodec and sends the resulting
H.264 packets over the same ADB-forwarded binary framing. That path also reports
a video-lab metric sample for helper encode/write timing and drop/stale/queue
counters. With `--emit-screenrecord-video`, it runs the shell-only Android
`screenrecord --output-format=h264 -` display capture path, chunks stdout H.264
bytes into the same framing, and reports the same broker metadata/metric shape.
Frame bytes are still kept off the broker JSON/WebSocket path.

The camera open/capture path is diagnostic only. It does not keep a camera
session alive or provide frame transport to clients yet. Future helper slices
can add lower-level buffer transport after the launch, broker-handshake,
metadata contract, binary side-channel, synthetic encoder, shell screenrecord,
and one-frame Camera2 feasibility paths are stable.

## Build

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-shell-helper\tools\Build-BrokerShellHelper.ps1
```

Output:

```text
examples/quest-broker-shell-helper/build/outputs/rusty-xr-broker-shell-helper.jar
```

Generated jars and build output are ignored by git.

## Run

Start the broker APK first, then run:

```powershell
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-shell-helper\tools\Start-BrokerShellHelper.ps1 -Serial <serial>
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-shell-helper\tools\Start-BrokerShellHelper.ps1 -Serial <serial> -ProbeCodecs -EmitSyntheticVideoMetadata
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-shell-helper\tools\Start-BrokerShellHelper.ps1 -Serial <serial> -ProbeCameras
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-shell-helper\tools\Start-BrokerShellHelper.ps1 -Serial <serial> -ProbeCameras -ProbeCameraOpen
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-shell-helper\tools\Start-BrokerShellHelper.ps1 -Serial <serial> -EmitSyntheticVideoBinary -BinaryVideoPackets 3 -BinaryVideoPacketBytes 1024
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-shell-helper\tools\Start-BrokerShellHelper.ps1 -Serial <serial> -EmitMediaCodecSyntheticVideo -EncodedVideoFrames 4 -EncodedVideoWidth 320 -EncodedVideoHeight 180
powershell -ExecutionPolicy Bypass -File .\examples\quest-broker-shell-helper\tools\Start-BrokerShellHelper.ps1 -Serial <serial> -EmitScreenrecordVideo -EncodedVideoWidth 320 -EncodedVideoHeight 180 -EncodedVideoBitrate 500000 -ScreenrecordTimeLimit 1 -BinaryVideoPackets 30 -BinaryVideoPacketBytes 16384
```

From the sibling Rusty XR Companion Apps source checkout, the same lifecycle can
be managed through the public CLI:

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- broker shell-helper start --serial <serial> --rusty-xr-root . --json
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- broker shell-helper start --serial <serial> --rusty-xr-root . --no-build --probe-codecs --json
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- broker shell-helper start --serial <serial> --rusty-xr-root . --probe-cameras --json
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- broker shell-helper start --serial <serial> --rusty-xr-root . --probe-cameras --probe-camera-open --json
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- broker shell-helper start --serial <serial> --rusty-xr-root . --no-build --probe-codecs --emit-synthetic-video-metadata --json
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- broker shell-helper binary-probe --serial <serial> --rusty-xr-root . --json
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- broker shell-helper binary-probe --serial <serial> --rusty-xr-root . --mediacodec-synthetic --encoded-video-frames 4 --encoded-video-width 320 --encoded-video-height 180 --json
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- broker shell-helper binary-probe --serial <serial> --rusty-xr-root . --screenrecord-source --encoded-video-width 320 --encoded-video-height 180 --encoded-video-bitrate 500000 --screenrecord-time-limit 1 --json
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- broker shell-helper status --serial <serial> --json
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- broker shell-helper stop --serial <serial> --rusty-xr-root . --no-build --json
```

Manual shape:

```powershell
adb push .\examples\quest-broker-shell-helper\build\outputs\rusty-xr-broker-shell-helper.jar /data/local/tmp/rusty-xr-broker-shell-helper.jar
adb shell CLASSPATH=/data/local/tmp/rusty-xr-broker-shell-helper.jar app_process / com.example.rustyxr.shell.Helper --broker-host 127.0.0.1 --broker-port 8765 --probe-codecs --emit-synthetic-video-metadata
```

Expected result:

- helper logs its UID
- broker receives `shell_helper.report_status`
- when camera probing is enabled, broker status includes
  `diagnostics.camera_probe` with bounded parsed shell-visible camera metadata
- when Camera2 open probing is enabled, broker status includes
  `diagnostics.camera_open_probe` and camera-provider status summarizes
  shell metadata/open/capture feasibility for projection-profile selection
- when synthetic video metadata is enabled, broker receives
  `video_lab.register_encoded_stream_manifest` and
  `video_lab.record_encoded_sample_metadata`
- when synthetic binary video is enabled, the host receives a
  `rusty.xr.video_lab.binary_stream.v1` stream with magic `RXYRVID1`, bounded
  packet headers, and deterministic payload bytes over ADB-forwarded TCP
- when MediaCodec synthetic video is enabled, the host receives variable-size
  H.264 packets generated from a synthetic Surface source over the same binary
  framing
- when MediaCodec synthetic video is enabled, broker receives one
  `video_lab.record_metric_sample` for helper encode/write timing and
  drop/stale/queue counters
- when screenrecord video is enabled, the host receives bounded H.264 chunks
  captured from the display by the Android shell `screenrecord` command over
  the same binary framing
- `GET http://127.0.0.1:8765/status` shows `shellHelper.connected=true`

## Boundary

This helper is Developer Mode / ADB tooling. The installed broker APK does not
inherit shell privileges. Do not describe this as a normal app permission or a
store-style runtime capability.
