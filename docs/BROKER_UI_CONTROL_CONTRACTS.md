# Broker UI And Control Contracts

Rusty XR models broker-facing UI and command authority as data contracts in
`rusty-xr-broker-model`. UI frameworks, device APIs, sockets, and renderer
lifecycles stay outside the model crate.

## Contract Families

- `BrokerPanelDescriptorDocument` describes broker-owned panels and widget
  primitives that a Makepad, web, companion, or command-line surface can render.
- `BrokerTelemetryChartDescriptor` describes a stream metric that can be drawn
  on an x/y chart without hard-coding a UI framework.
- `BrokerStreamRegistrySnapshot` describes providers, streams, adapters,
  subscribers, command clients, and active leases at one broker revision.
- `BrokerCommandAuthorityRequirement`, `BrokerControlScope`,
  `BrokerCommandPrecondition`, and `BrokerControlLease` describe the authority
  checks needed before mutating commands.

The contracts deliberately do not grant authority. A broker implementation must
still validate capability, lease, revision, and operator-confirmation rules at
command execution time.

## UI Rules

Panel descriptors are safe to expose to read-only clients when their advertised
data sensitivity and command scopes match the client capability set. A command
button that is not read-only must be lease-aware before a client treats it as
actionable.

Telemetry charts bind to stream ids and metric names from the stream registry.
Low-rate telemetry can be retained in local UI history. High-rate or media-like
streams should use the registry `rate_class` and `retention_policy` fields to
decide whether a UI should draw, downsample, or refuse a direct chart.
The public registry helpers expose this same distinction through chartable
streams and UI auto-subscribe candidates so clients do not need to duplicate
basic media-vs-telemetry filtering rules.

## Registry Snapshot Entrypoints

Brokers can expose `BrokerStreamRegistrySnapshot` through the read-only
`stream_registry.snapshot` command and the optional
`/stream_registry/snapshot` HTTP path. Both surfaces should return the same
schema id, broker id, revision, providers, streams, adapters, subscribers,
command clients, and active leases. A UI may render this topology without
receiving mutation authority. The registry `revision` is a topology and stream
state witness; read-only status, capability, stream-list, or registry queries
should not advance it merely because a command counter changed.

## Public Fixtures

Synthetic fixtures live under `fixtures/broker-ui`:

- `synthetic-panel-descriptor.json`
- `synthetic-stream-registry-snapshot.json`

They are intentionally generic. Downstream apps can keep app-specific package
identity, private streams, assets, and runtime behavior in their own repos while
still validating against these public shapes.

## Validation

Recommended checks for this contract surface:

```powershell
cargo test -p rusty-xr-broker-model --features serde
python tools/schema/export_schemas.py --check
python tools/boundary-scan/rusty_xr_boundary_scan.py --repo-root .
```
