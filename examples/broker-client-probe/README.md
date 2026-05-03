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
cargo run -p rusty-xr-broker-client-probe -- sample --subscribe
cargo run -p rusty-xr-broker-client-probe -- open-ui
cargo run -p rusty-xr-broker-client-probe -- close-ui
```

Commands:

- `status`: read `GET /status`.
- `capabilities`: send `list_capabilities` over WebSocket.
- `streams`: send `list_streams` over WebSocket.
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
