# Rusty XR Broker Registry Simulator

This source-only simulator serves public, synthetic
`BrokerStreamRegistrySnapshot` data over the same read-only registry surfaces as
a broker implementation:

- `GET /status`
- `GET /stream_registry/snapshot`
- WebSocket `/rustyxr/v1/events`
- WebSocket `stream_registry.snapshot`

It is meant for UI, CLI, and schema smoke checks before a live broker is
available. It does not load modules, open adapter sockets, route high-rate
payloads, or grant mutating command authority.

## Run

```powershell
cargo run -p rusty-xr-broker-registry-simulator -- --host 127.0.0.1 --port 8765
cargo run -p rusty-xr-broker-client-probe -- registry-summary
cargo run -p rusty-xr-broker-client-probe -- registry-http-summary
```

Use a degraded module-health profile:

```powershell
cargo run -p rusty-xr-broker-registry-simulator -- --profile degraded
```

Use `--max-connections <count>` when a script should serve a bounded number of
client connections and exit.

## Validation

```powershell
cargo test -p rusty-xr-broker-registry-simulator
cargo test -p rusty-xr-broker-client-probe
```
