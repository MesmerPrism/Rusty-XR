# Public Extraction Workflow

Rusty XR extracts reusable pieces from downstream pressure only when they can
be expressed as public contracts, deterministic helpers, synthetic fixtures, or
source examples.

## Gate

Every extraction must pass this sequence:

1. Interface first: define the public data shape, validation rule, schema, or
   scorecard without downstream package names or private tuning values.
2. Implementation second: add the smallest deterministic helper needed to make
   the interface useful with synthetic inputs.
3. Adapter last: keep native platform calls, app launch identity, renderer
   ownership, device mutation, assets, and release payloads outside core.
4. Proof always: add a synthetic test, source example, schema check, or docs
   matrix entry before treating the extraction as durable.

## Accepted Public Units

- Data contracts and typed enums.
- Parser, validator, layout, math, or scoring helpers.
- Synthetic fixtures and deterministic generators.
- Schema export and compatibility checks.
- Public examples that compose existing crates.
- Sanitized diagnostics with no private payloads.

## Rejected Public Units

- App package identities, launch activities, signing, release payloads, or
  generated APK/AAB files.
- Private visual stacks, exact effect recipes, calibration values, study logic,
  or product-specific tuning constants.
- Headset serials, screenshots, captures, logcat dumps, media frames, or local
  artifact paths.
- SDK calls, device mutation, renderer ownership, and platform lifecycle logic
  that cannot be tested without a specific downstream app.

## Current Extraction Pressure

| Pressure area | Public unit now | Status | Next public step |
| --- | --- | --- | --- |
| Visual effect-stack diagnostics | `rusty-xr-contracts::effect_stack`, [Effect-stack diagnostics](EFFECT_STACK_DIAGNOSTICS.md), and the contracts example `effect_stack_diagnostic_manifest`. | Interface extracted. | Keep expanding scalar reports and synthetic examples before any app-owned behavior moves. |
| Dynamic mesh coordinate and topology fixtures | `rusty-xr-particles` topology keys, dynamic mesh coordinate sampler, [mesh fixture manifest](MESH_FIXTURE_MANIFEST.md), public hand-mesh fixtures, collider helper, SDF attraction helper, and [Dynamic mesh coordinate sampling](DYNAMIC_MESH_COORDINATE_SAMPLING.md). | Manifest and fixtures extracted. | Connect the manifests to the next public mesh/topology utility or downstream pressure-test workflow before adding larger fixture data. |
| Broker, clock, kiosk, and command evidence | `rusty-xr-broker-model` clock/status/stream contracts, `rusty-xr-contracts::home`, `KioskCommandRunRecord`, [Quest developer-home menu](QUEST_DEVELOPER_HOME_MENU.md), [Broker clock and timebase](BROKER_CLOCK_AND_TIMEBASE.md), and [API/CLI/MCP entrypoints](API_CLI_MCP_ENTRYPOINTS.md). | Run-record interface extracted. | Keep command evidence generic: goal, provider, fallback, foreground before/after, broker clock/status, surface intent, and outcome. |
| Agent-state JSONL workflow | Local agent infrastructure outside the public repo. | Stays private/local. | Do not move machine-specific agent ledgers into Rusty XR. If a reusable public pattern appears, extract a schema-only crate or example first. |

## Extraction Record

When extracting, update the public repo and the local planning ledger
separately:

- Public repo: docs, tests, examples, schemas, and crate code.
- Local planning ledger: candidate status, evidence paths, blocker, and next
  action.

Do not hide extraction decisions only in prose logs. The current public source
of truth for future agents is:

- [Module and crate map](MODULE_CRATE_MAP.md)
- [Examples matrix](EXAMPLES_MATRIX.md)
- [Validation commands](VALIDATION.md)
- [API, CLI, and MCP entrypoints](API_CLI_MCP_ENTRYPOINTS.md)
- The owning crate tests and examples
