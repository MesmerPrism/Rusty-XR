# API, CLI, And MCP Entrypoints

This map keeps Rusty XR usable by agents without forcing every task through one
tool. The project should expose stable Rust APIs first, schemas and source
examples second, CLIs for operator workflows third, and MCP only behind the same
safety and audit rules.

## Current Entrypoint Layers

| Layer | Owned by this repo | Purpose | Machine-readable surface |
| --- | --- | --- | --- |
| Rust APIs | Yes | Public contracts and deterministic helpers in `crates/`. | Rust types, tests, and serde examples. |
| JSON schemas | Yes | Stable interchange shapes for catalogs, provider snapshots, broker records, kiosk records, and diagnostics. | `python tools\schema\export_schemas.py --check` and generated schema files. |
| Source examples | Yes | No-hardware contract proofs and source-build examples. | `cargo run ... --features serde`, example JSON, public catalogs. |
| Broker HTTP/WebSocket API | Example-owned | Runtime status, clock, stream, command, registry, and kiosk control-plane reporting. | `/status`, `/stream_registry/snapshot`, `/kiosk/status`, `/clock/now`, `/clock/health`, WebSocket commands, broker streams. |
| Rust probe CLI | Yes | Source-only broker status and command probes. | `cargo run -p rusty-xr-broker-client-probe -- <command>`. |
| PowerShell/Python tools | Yes | Repo-local validation, schema checks, scorecards, and source builds. | `--json`, `--json-out`, schema files, scorecard JSON, boundary reports. |
| Companion CLI | Sibling public repo | Install, launch, catalog verify, shell-helper orchestration, and operator reports. | `dotnet run --project ...RustyXr.Companion.Cli -- ... --json`. |
| MCP provider | Optional external provider | Read-only status, docs/API lookup, and gated Quest operation planning. | `McpServerConfig`, provider snapshots, blocked plans for gated side effects. |

## Kiosk Run Record

Rusty Kiosk operations should converge on
`rusty.xr.kiosk.command_run_record.v1`, exported as
`home-kiosk-command-run-record.schema.json`.

Use this record when a tool observes or changes kiosk state through any of
these paths:

- Rust API helper or source example.
- Broker HTTP endpoint such as `GET /kiosk/status`.
- Broker WebSocket command such as `kiosk.get_status`.
- Companion CLI command with `--json`.
- Shell helper CLI command.
- Direct ADB fallback.
- `hzdb` CLI.
- `hzdb` MCP server.
- Manual operator note.

Each run record should include:

- command goal;
- surface intent;
- primary provider evidence;
- fallback provider evidence, when used;
- before and after `KioskControlPlaneStatus`, when available;
- outcome;
- issue codes and notes.

This keeps API, CLI, and MCP routes comparable without making any one provider
mandatory.

## Agent Task Routing

| Agent task | First public entrypoint | Fallback or companion path | Notes |
| --- | --- | --- | --- |
| Understand crate roles | [Module and crate map](MODULE_CRATE_MAP.md), Cargo metadata | Rust docs and crate tests | Do not infer current architecture from long investigation logs first. |
| Validate docs and public boundary | [Validation commands](VALIDATION.md) | CI command list | Boundary scan is required for public/private-sensitive changes. |
| Emit a synthetic kiosk record | `cargo run -p rusty-xr-contracts --example kiosk_command_run_record --features serde` | Schema check | No headset or provider is used. |
| Query broker state | `cargo run -p rusty-xr-broker-client-probe -- status` | `GET /status`, `GET /kiosk/status` | Requires a broker only when querying live state. |
| Query broker stream topology | `cargo run -p rusty-xr-broker-client-probe -- registry-summary` | `GET /stream_registry/snapshot`, WebSocket `stream_registry.snapshot` | Read-only module/provider/stream topology; parsed summaries validate module links and do not grant command authority. |
| Exercise broker control leases | `cargo run -p rusty-xr-broker-client-probe -- lease-request` | WebSocket `control_lease.request` / `control_lease.release` | Grants temporary broker-side authority only after the broker accepts holder, scope, revision, and conflict checks. |
| Query broker clock | Broker `/clock/now` and `/clock/health` | Companion broker report | Use clock epoch IDs in kiosk records when available. |
| Verify app catalog | Companion `catalog verify ... --json` | Source catalog schema checks | Device validation requires operator/resource workflow. |
| Inspect Quest provider readiness | `rusty-xr-quest-diagnostics` provider snapshot | Companion or `hzdb`/ADB probe | Read-only by default. |
| Generate schemas | `python tools\schema\export_schemas.py --check` | `python tools\schema\export_schemas.py --out <dir>` | Generated files stay out of default source unless intentionally committed. |
| Generate scorecards | Tool-specific `--json-out` command | Public scorecard contracts | Raw captures and local artifacts stay ignored. |
| Use MCP | Provider snapshot and safety layer first | CLI fallback with same run record | Mutating MCP calls should return a blocked plan unless explicitly gated. |

## Consistency Rules

- Every durable task surface should have a Rust type or documented schema.
- Every live provider path should have a source-only or synthetic validation
  path.
- Every CLI/MCP side effect should map to the same safety class as the
  equivalent API operation.
- Every provider route should record the command goal, provider, fallback,
  foreground before/after, broker clock/status when available, and surface
  intent.
- Read-only status APIs may be easy to call; app lifecycle, file mutation,
  device setting, shell command, network forward, and root operations require
  operator gates in the invoking tool.

## Current Gap

This repo does not own a first-party MCP server. It owns provider-neutral MCP
configuration models and the evidence envelope that an MCP provider should
emit. The MCP execution layer belongs in companion/operator tooling until the
safety layer and operation catalog are stable.
