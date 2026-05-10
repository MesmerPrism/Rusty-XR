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
cargo run -p rusty-xr-broker-client-probe -- camera-provider
cargo run -p rusty-xr-broker-client-probe -- projection-profile
cargo run -p rusty-xr-broker-client-probe -- app-camera-probe
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
- `camera-provider`: send `camera_provider.get_status`.
- `projection-profile`: send `camera_provider.get_projection_profile`.
- `app-camera-probe`: send `camera_provider.run_app_camera_probe`; the broker
  APK must have runtime camera permission for capture attempts to succeed.
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
