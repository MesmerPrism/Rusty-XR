# Rusty XR Broker Client Probe

This is a small Rust-side client for the public Quest broker proof-of-concept.
It is intended as a source-level compatibility example for Rust-native tools and
app shells. It uses only `std::net` plus `serde_json`, so it can be read and
ported without adopting a specific async runtime or WebSocket crate.

Typical local flow after the broker APK is installed and running on the Quest:

```powershell
dotnet run --project ..\Rusty-XR-Companion-Apps\src\RustyXr.Companion.Cli -- broker forward --serial <serial>
cargo run -p rusty-xr-broker-client-probe -- status
cargo run -p rusty-xr-broker-client-probe -- streams
cargo run -p rusty-xr-broker-client-probe -- registry
cargo run -p rusty-xr-broker-client-probe -- registry-summary
cargo run -p rusty-xr-broker-client-probe -- registry-http
cargo run -p rusty-xr-broker-client-probe -- registry-http-summary
cargo run -p rusty-xr-broker-client-probe -- host-manifest
cargo run -p rusty-xr-broker-client-probe -- host-manifest-http
cargo run -p rusty-xr-broker-client-probe -- lease-request --scope session.lifecycle --duration-ms 60000
cargo run -p rusty-xr-broker-client-probe -- lease-release --lease control-lease-1 --reason operator_done
cargo run -p rusty-xr-broker-client-probe -- camera-provider
cargo run -p rusty-xr-broker-client-probe -- projection-profile
cargo run -p rusty-xr-broker-client-probe -- app-camera-probe
cargo run -p rusty-xr-broker-client-probe -- app-camera-probe --camera-id 50 --persist-frame --frame-output-dir /sdcard/Android/data/com.example.rustyxr.broker/files/camera-frame-capture/run-001
cargo run -p rusty-xr-broker-client-probe -- synthetic-h264-stream --session frozen-left --device-port 8879 --host-port 18879 --synthetic-image-path /sdcard/Android/data/com.example.rustyxr.broker/files/camera-frame-capture/camera-50.jpg
cargo run -p rusty-xr-broker-client-probe -- app-camera-h264-decode-probe --session probe-h264-decode-session
cargo run -p rusty-xr-broker-client-probe -- shell-helper-status
cargo run -p rusty-xr-broker-client-probe -- video-lab-status
cargo run -p rusty-xr-broker-client-probe -- video-lab-scorecard
cargo run -p rusty-xr-broker-client-probe -- video-manifest-stub
cargo run -p rusty-xr-broker-client-probe -- video-sample-meta-stub
cargo run -p rusty-xr-broker-client-probe -- video-metric-stub
cargo run -p rusty-xr-broker-client-probe -- h264-proxy-probe
cargo run -p rusty-xr-broker-client-probe -- transport-capabilities
cargo run -p rusty-xr-broker-client-probe -- transport-create-session
cargo run -p rusty-xr-broker-client-probe -- transport-list-sessions
cargo run -p rusty-xr-broker-client-probe -- transport-get-session --session probe-transport-session
cargo run -p rusty-xr-broker-client-probe -- transport-close-session --session probe-transport-session
cargo run -p rusty-xr-broker-client-probe -- sample --subscribe
cargo run -p rusty-xr-broker-client-probe -- open-ui
cargo run -p rusty-xr-broker-client-probe -- close-ui
```

Commands:

- `status`: read `GET /status`.
- `capabilities`: send `list_capabilities` over WebSocket.
- `streams`: send `list_streams` over WebSocket.
- `registry`: send `stream_registry.snapshot` and print the broker topology
  snapshot.
- `registry-summary`: send `stream_registry.snapshot`, parse it as public
  `BrokerStreamRegistrySnapshot`, validate module links, and print compact
  module/provider/stream topology lines.
- `registry-http`: read `GET /stream_registry/snapshot` and print the broker
  topology snapshot.
- `registry-http-summary`: parse the HTTP registry snapshot the same way as
  `registry-summary`.
- `host-manifest`: send `broker.host_manifest` and print the broker host role,
  endpoint visibility, security policy, clock domain, and capabilities.
- `host-manifest-http`: read `GET /broker/host_manifest` and print the same
  broker host manifest over HTTP.
- `lease-request`: send `control_lease.request` with a public control-scope
  payload. Defaults to `session.lifecycle`; use `--scope`, `--command-scope`,
  `--resource`, `--duration-ms`, `--expected-revision`, and
  `--operator-confirmed` to exercise stricter broker gates.
- `lease-release --lease <id>`: send `control_lease.release`. Use `--scope`
  to require the released lease to match a specific scope and `--reason` to
  tag the release.
- `camera-provider`: send `camera_provider.get_status`.
- `projection-profile`: send `camera_provider.get_projection_profile`.
- `app-camera-probe`: send `camera_provider.run_app_camera_probe`; the broker
  APK must have runtime camera permission for capture attempts to succeed. Add
  `--camera-id`, `--persist-frame`, and `--frame-output-dir` to persist the
  one-frame YUV capture as NV21 raw bytes, a JPEG preview, and a JSON sidecar
  under the broker app's external files area.
- `synthetic-h264-stream`: send `media.start_synthetic_h264_stream`. With
  `--synthetic-image-path`, the broker draws the named device-local image into
  every encoder frame and serves it as a live `RXYRVID1` H.264 stream. This is
  intended for frozen camera-frame replay through the same full-frame broker
  transport used by generated synthetic sources.
- `app-camera-h264-decode-probe [--session <id>]`: run the broker's bounded
  Camera2 to MediaCodec H.264 encode/decode probe and tag the resulting
  manifest/metric with the supplied session id.
- `shell-helper-status`: send `shell_helper.get_status`.
- `shell-helper-report-stub`: send a synthetic `shell_helper.report_status`
  payload so a forwarded broker can exercise the helper-status path before a
  real ADB-launched helper exists.
- `video-lab-status`: send `video_lab.get_status`.
- `video-lab-scorecard`: send `video_lab.get_scorecard` to summarize the
  latest manifest/metric evidence for payload transport, decode, and proxy
  readiness.
- `video-manifest-stub`: register a metadata-only encoded H.264 stream
  manifest with `video_lab.register_encoded_stream_manifest`.
- `video-sample-meta-stub`: record one metadata-only encoded sample event with
  `video_lab.record_encoded_sample_metadata`.
- `video-metric-stub`: send a synthetic
  `video_lab.record_metric_sample` payload so the broker can exercise the
  timing/drop/queue metric path before encoded video packets exist.
- `h264-proxy-probe`: run the in-process synthetic `RXYRVID1` H.264 TCP proxy
  probe without requiring camera permission.
- `transport-capabilities`: send `transport.describe_capabilities`.
- `transport-create-session [--session <id>]`: send a loopback-only
  `transport.create_session` offer for an H.264 media stream.
- `transport-list-sessions`: send `transport.list_sessions`.
- `transport-get-session --session <id>`: send `transport.get_session`.
- `transport-close-session --session <id>`: send `transport.close_session`.
- `subscribe --stream <id>`: subscribe to a broker stream and print the ack.
- `open-ui`: send `open_ui` so the broker Activity brings its 2D console to
  the foreground.
- `close-ui`: send `close_ui` so the active broker console Activity finishes
  while the broker service remains running.
- `sample [--subscribe]`: send a synthetic `latency_sample`; with
  `--subscribe`, first subscribes to `latency:sample` and then prints the
  resulting stream event/ack messages.

Connection options:

- `--host <host>` defaults to `127.0.0.1`.
- `--port <port>` defaults to `8765`.
- `--session <id>` selects a transport session id for transport commands.
- `--lease <id>` selects a control lease id for `lease-release`.
- `--scope <id>`, `--command-scope <scope>`, and `--resource <id>` select the
  broker control scope for lease commands.
- `--duration-ms <ms>`, `--expected-revision <revision>`,
  `--operator-confirmed`, and `--reason <text>` tune lease command payloads.
- `--camera-id <id>` restricts `app-camera-probe` to one Camera2 id.
- `--persist-frame` asks `app-camera-probe` to write the captured frame to
  device storage.
- `--frame-output-dir <path>` sets the device output directory and implies
  `--persist-frame`.
- `--jpeg-quality <1-100>` sets the preview quality for persisted frames.
- `--width <pixels>` and `--height <pixels>` request the app-camera probe's
  preferred YUV capture size.
- `--device-port <port>` and `--host-port <port>` select stream endpoints for
  `synthetic-h264-stream`.
- For `synthetic-h264-stream`, `--width <pixels>` and `--height <pixels>` set
  both encoded and full-frame content dimensions.
- `--capture-ms <ms>`, `--max-packets <count>`, `--bitrate-bps <bps>`,
  `--frame-rate-hz <hz>`, and `--accept-timeout-ms <ms>` tune the live replay
  stream bounds.
- `--synthetic-pattern <name>` defaults to `image-file` when
  `--synthetic-image-path` is present and otherwise defaults to
  `diagnostic-grid`.
- `--projection-profile <name>` defaults to `full-frame-diagnostic`.
