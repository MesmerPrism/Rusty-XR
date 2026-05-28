# Broker Host Manifests

`BrokerHostManifest` describes where the authoritative broker is running and
which endpoints a UI, companion, or adapter may use to reach it. The manifest
keeps broker placement separate from UI ownership: Makepad, web, CLI, or
headset panels can render the same broker state while the broker remains the
authority for commands, leases, revisions, and session state.

## Contract

The public model lives in `rusty-xr-broker-model`:

- `BrokerHostManifest`
- `BrokerHostEndpointDescriptor`
- `BrokerHostAuthorityRole`
- `BrokerEndpointVisibility`
- `BROKER_HOST_MANIFEST_COMMAND`
- `BROKER_HOST_MANIFEST_HTTP_PATH`

The manifest records:

- `host_id` and `authority_role`, such as `headset_local_primary` or
  `desktop_primary`;
- advertised endpoints with visibility classes such as `loopback`,
  `adb_forwarded`, `paired_lan`, and `public_relay`;
- a transport security policy;
- broker capabilities;
- the broker clock domain and whether a session manifest is required.

## Security Rule

Endpoint visibility is descriptive, but it must agree with transport security.
Loopback manifests should not advertise LAN or relay endpoints. `paired_lan`
and `public_relay` endpoints require non-loopback security such as a pairing
token or an externally owned sidecar gate.

This manifest does not grant authority. Mutating commands still require the
broker to validate client role, capability, control lease, revision, expiry,
holder identity, and any operator confirmation.

## Entrypoints

Brokers can expose the manifest through:

```text
broker.host_manifest
/broker/host_manifest
```

The command and HTTP response should return the same schema, host id,
authority role, endpoint list, capabilities, security policy, clock domain, and
session-manifest expectation.

## UI Host Selection Flow

A UI host should treat the manifest as the deployment picker before it opens a
long-lived broker session:

1. Read `/broker/host_manifest` over the already-known bootstrap route.
2. Render only visible endpoints and surface their visibility, command scope,
   transport, and security mode to the operator.
3. Prefer the manifest primary endpoint when it is compatible with the UI's
   transport stack; otherwise require an explicit endpoint choice.
4. Open the selected WebSocket or forwarded endpoint and fetch
   `stream_registry.snapshot`.
5. Use the registry revision from that selected broker session for any later
   control-lease request or mutating-command precondition.

This keeps Makepad, web, and companion UIs deployment-aware without making the
UI authoritative. Endpoint selection only chooses a transport route; capability,
lease, holder, expiry, revision, and operator-confirmation checks still happen
inside the broker.

## Fixture

The public synthetic fixture lives at:

```text
fixtures/broker-host/synthetic-host-manifest.json
```

It intentionally advertises only loopback endpoints. LAN, relay, pairing, and
sidecar endpoints should be introduced only by broker implementations that own
the corresponding pairing or security gate.

## Validation

Recommended checks for this contract surface:

```powershell
cargo test -p rusty-xr-broker-model --features serde
python tools/schema/export_schemas.py --check
python tools/boundary-scan/rusty_xr_boundary_scan.py --repo-root .
```
