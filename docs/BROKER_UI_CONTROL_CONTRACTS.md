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
still validate role, capability, lease, revision, expiry, holder identity, and
operator-confirmation rules at command execution time.

## UI Rules

Panel descriptors are safe to expose to read-only clients when their advertised
data sensitivity and command scopes match the client capability set. A command
button that is not read-only must be lease-aware before a client treats it as
actionable.

Panel-level capability is an authority default, not only a visibility hint. The
effective command requirement for a command button uses the widget
`required_capability` when present, otherwise it inherits the panel
`required_capability`. Clients should use
`BrokerPanelDescriptorDocument::command_authority_requirements()` or
`BrokerPanelDescriptor::command_authority_requirements()` when deciding what a
rendered command would require, so Makepad, web, and CLI surfaces do not
interpret panel visibility differently from command authority.

Mutating, exclusive-lease, and external-gate command requirements are valid only
when they carry a capability gate and a revision gate. Exclusive-lease commands
must also name a valid lease scope. `BrokerControlLease::is_active_for` checks
the holder client id, exact scope, current registry revision, active state, and
expiry; `matches_scope_at_revision` is the non-authoritative helper for
descriptor/topology matching.

Telemetry charts bind to stream ids and metric names from the stream registry.
Low-rate telemetry can be retained in local UI history. High-rate or media-like
streams should advertise explicit `ui_subscription_policy` and `chart_policy`
fields instead of relying only on inferred `rate_class` and `retention_policy`.
The public registry helpers expose this distinction through chartable streams
and UI auto-subscribe candidates. `AutoSubscribeLowRate` is the only policy that
enters the default auto-subscribe set; `AutoSubscribeWhenSelected`,
`ManualOnly`, and `NeverSubscribeFromUi` require an explicit UI decision.
Likewise, `LowRateDirect` and `DownsampleRequired` can enter chart catalogs,
`DedicatedViewRequired` keeps specialized streams out of generic charts, and
`NotChartable` keeps unknown or metadata-only streams out.

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
python tools/schema/check_broker_ui_fixtures.py --repo-root .
python tools/boundary-scan/rusty_xr_boundary_scan.py --repo-root .
```
