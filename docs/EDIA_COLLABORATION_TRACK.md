# EDIA Collaboration Track

Status: public collaboration note for maintainers and contributors. This is
not an endorsement, affiliation, or compatibility claim by EDIA or Rusty XR.

## Purpose

This track explains what Rusty XR is doing around EDIA-style research XR
workflows, why the work exists, and where maintainer feedback would be most
useful before adapter work starts.

The goal is collaboration clarity:

- keep EDIA and Unity-centered research workflows in charge of experiment
  semantics, participant flow, scenes, and UXF-style logging
- keep Rusty XR focused on reusable broker contracts, stream metadata,
  validation, replay, diagnostics, and protocol fan-out
- avoid copying EDIA source code unless a future adapter deliberately ports
  something with license notices preserved
- avoid implying that Rusty XR replaces EDIA or that either project has
  endorsed the other

## Current Implementation State

Implemented in public Rusty XR core:

- `rusty-xr-broker-model`
  - command, acknowledgement, and client-hello envelopes
  - stream manifests and sample headers
  - transport metadata for WebSocket, TCP-like, UDP, ADB-forwarded TCP, and
    metadata-only lanes
  - reliability classes for reliable, loss-tolerant, best-effort, and
    metadata-only streams
  - heartbeat state, drop counters, session manifests, replay records, and
    deterministic synthetic wave samples
- `rusty-xr-eye-model`
  - screen-space gaze points
  - headset-local and world-space XR gaze rays
  - screen-space AOI hits
  - derived fixation, dwell, and blink event shapes
  - validity flags, confidence, provenance, and deterministic synthetic eye
    samples
- public schema export coverage for these contracts
- a public architecture page:
  [RESEARCH_XR_BROKER_BRIDGE.md](RESEARCH_XR_BROKER_BRIDGE.md)

Also implemented as public example and operator tooling:

- a Quest broker proof-of-concept APK with localhost HTTP/WebSocket status and
  events, OSC ingress/egress probes, generic stream-event publication, optional
  native LSL forwarding through a user-supplied Android `liblsl.so`, and a
  no-display sidecar launch activity for catalog automation
- Companion catalog verification for the broker APK, including process/API
  health checks instead of fragile foreground-activity checks
- a public Unity comparison path using the Big Red Button example: direct Unity
  OSC acknowledgements and broker-routed OSC/WebSocket stream events exercise
  the same visible target for side-by-side diagnostics

Not implemented yet:

- no live broker UDP socket or TCP socket runtime in the contract crates
- no EDIA package, Unity package, or UXF integration
- no Tobii, headset, or other native eye-tracker provider
- no LSL/OSC/WebSocket real eye-data forwarder for proprietary tracker data
- no claim that a desktop tracker validates headset-local XR gaze semantics

The TCP/UDP lesson from RCAS is implemented as explicit public contracts:
stream manifests can describe reliable control paths and loss-tolerant data
lanes, and sample headers carry sequence/timing/drop metadata. Actual network
runtimes and adapters remain example- or downstream-adapter work rather than
contract-crate behavior.

## Agent Readiness Notes

Reviewed public materials on 2026-05-09:

- [EDIA website](https://edia-toolbox.github.io/)
- [edia_core](https://github.com/edia-toolbox/edia_core)
- [edia_eye](https://github.com/edia-toolbox/edia_eye)
- [edia_lsl](https://github.com/edia-toolbox/edia_lsl)
- [edia_rcas](https://github.com/edia-toolbox/edia_rcas)
- [edia_installer](https://github.com/edia-toolbox/edia_installer)
- Rusty XR public core and public Companion docs in this repo family

This section is written as collaboration support, not as a maintainer claim
about EDIA internals. "Not visible" means "not found in the public materials
reviewed for this note."

### Current EDIA Public Posture

EDIA already has useful human-facing project surfaces:

- a public website describing EDIA as a modular Unity XR toolbox for research
- Unity package repositories for Core, Eye, LSL, RCAS, and device-specific eye
  modules
- Unity Package Manager install instructions in package READMEs
- sample scenes and package documentation paths
- Doxygen-generated API documentation in the Core, Eye, and LSL repos
- an installer repo that provides a Unity Editor script for installing EDIA
  modules and core XR dependencies
- GitHub Discussions and issue trackers for maintainer feedback

That is a good human contributor posture. The part that was not visible in the
reviewed public materials is a deliberate agent-facing operational layer:

| Surface | Public EDIA status from reviewed materials |
| --- | --- |
| Root agent-notes files in Core, Eye, LSL, RCAS, Installer | Not visible in root repo listings reviewed |
| Portable AI-agent skill such as `skills/edia-toolbox-builder/SKILL.md` | Not visible |
| Machine-readable module catalog | Not visible |
| Standalone CLI command surface | Not visible |
| CLI operation planner | Not visible |
| MCP server or MCP tool schema | Not visible |
| Generated API surface JSON | Not visible |
| Scriptable Unity batchmode installer entry point | Not visible in public installer README reviewed |

EDIA repositories do expose `.github/workflows` folders in several root
listings, so this note should not be read as claiming there is no CI. The
specific missing piece is a documented CI target for an agent-readable module
catalog, generated API surface, or operation catalog.

### Current Rusty XR Agent Support

Rusty XR core is already structured for agent-assisted public work:

- root agent notes with public-boundary rules, orientation files, source
  workspace guidance, and validation commands
- `skills/rusty-xr-builder/SKILL.md` as a portable raw AI-agent skill
- docs that distinguish public contracts from downstream app behavior
- schema export checks for public contract crates
- public-boundary scans before pushing from this machine

Rusty XR Companion extends that support into an operational surface:

- documented API, CLI, and MCP split
- shared operation catalog for API, CLI, and MCP names
- `api surface`, `api surface --json`, and `api surface --mcp-tools`
- `api plan --operation ...` to return inspectable dispatch plans without
  executing them
- a stdio MCP server that executes read-only calls and returns blocked plans
  for side-effecting operations
- release-agent onboarding files bundled into app, CLI, and MCP release zips

For EDIA collaboration, the relevant pattern is not "build MCP first." The
useful sequence is:

```text
agent notes
-> module catalog
-> generated API/event surface
-> CLI operation planner
-> read-only MCP wrappers
-> controlled side-effect execution
```

That order keeps the work useful to human maintainers while making it safer for
agents and automation.

## EDIA Agent-Readiness Suggestions

These suggestions are intentionally incremental. They can be adopted by EDIA
maintainers independently of Rusty XR, and Rusty XR should treat them as
collaboration notes rather than requirements.

### Add Root Agent Notes

Add a root coding-agent notes file to each main EDIA repo, using EDIA
maintainers' preferred filename convention:

```text
edia_core/<agent-notes-file>
edia_eye/<agent-notes-file>
edia_lsl/<agent-notes-file>
edia_rcas/<agent-notes-file>
edia_installer/<agent-notes-file>
```

Each file can stay short. The goal is to give coding agents a deterministic
local playbook:

```md
# EDIA Agent Notes

## Purpose

Describe what this repo owns.

## First Steps

1. Read `README.md`.
2. Read the package manifest under `Assets/com.edia.*`.
3. Read the main runtime entry points for this module.
4. For docs/API work, inspect the Doxygen configuration.
5. For install behavior, inspect `edia_installer`.

## Unity Version

State the expected Unity editor version and whether the repo is a package-only
repo or a development Unity project.

## Package Path

State which `Assets/com.edia.*` path is the package source of truth.

## Dependencies

List required EDIA modules, Unity packages, external SDKs, and optional
device-specific integrations.

## Safe Changes

Describe edits that are normally safe for contributors and agents.

## Unsafe Changes

Do not commit Unity `Library/`, build output, device logs, participant data,
session data, EEG/LSL captures, headset serials, SDK binaries that cannot be
redistributed, private experiment configs, signing material, local package cache
paths, screenshots, videos, or local editor settings.

## Validation

List package manifest checks, Doxygen generation checks, Unity batchmode checks,
and sample-scene checks that maintainers expect before a PR.

## Review Checklist

- Public APIs are documented.
- Event names and payload expectations are updated.
- Generated artifacts are either current or ignored.
- No private research, device, SDK, or local-machine artifacts are committed.
```

EDIA Core should be the canonical overview because it is the central public
entry point for the toolbox.

### Add A Portable EDIA Skill

A portable skill would let Codex-style, Claude-style, and similar agents start
from the same public rules:

```text
skills/edia-toolbox-builder/SKILL.md
```

Suggested front matter:

```yaml
---
name: edia-toolbox-builder
description: Build, document, review, or extend EDIA Unity XR Toolbox packages for research, including Core, Eye, LSL, RCAS, package installation, Doxygen docs, Unity samples, and broker-bridge planning without committing participant data, generated Unity artifacts, device logs, SDK binaries, or private experiment configs.
---
```

Suggested sections:

- First Steps
- Module Map
- Unity Package Rules
- Public Boundary
- Doxygen/API Rules
- Installer Rules
- Eye/LSL/RCAS Rules
- Validation Commands
- Review Checklist

### Add A Machine-Readable Module Catalog

A repo-local catalog would let agents and tools stop scraping READMEs:

```text
tools/edia-catalog/edia-modules.schema.json
tools/edia-catalog/edia-modules.json
```

Example shape:

```json
{
  "schema": "edia.modules.v1",
  "modules": [
    {
      "id": "com.edia.core",
      "repo": "edia-toolbox/edia_core",
      "package_path": "Assets/com.edia.core",
      "unity": "6000.x",
      "dependencies": [
        "com.unity.inputsystem",
        "com.unity.xr.interaction.toolkit",
        "com.unity.xr.management",
        "com.unity.xr.hands",
        "com.unity.render-pipelines.universal",
        "com.unity.nuget.newtonsoft-json",
        "com.unity.textmeshpro"
      ],
      "samples": [
        "Starter Kit",
        "Demo Tasks",
        "Starter Room"
      ],
      "docs": {
        "doxygen_config": "DoxyGen/...",
        "generated_api": "Assets/com.edia.core/Documentation~"
      }
    }
  ]
}
```

The catalog should include optional modules, device-specific eye packages, and
external SDK requirements as explicit fields rather than prose.

### Generate API And Event Surfaces

EDIA's generated Doxygen docs are already the strongest starting point. The
next step is a compact machine-readable export:

```text
artifacts/api/edia-core.surface.json
artifacts/api/edia-eye.surface.json
artifacts/api/edia-lsl.surface.json
artifacts/api/edia-rcas.surface.json
```

Useful fields:

- package id and version
- Unity version constraints
- public namespaces, classes, methods, events, and constants
- sample names and package paths
- prefabs and scenes when extractable without launching Unity
- external SDK requirements
- Doxygen source path and generated docs path

EDIA Core's event constants are especially useful as a first export target.
Broker and automation tooling need stable event names, payload notes, and
ownership boundaries before they can safely bridge into a Unity project.

Example event surface:

```json
{
  "schema": "edia.events.v1",
  "module": "com.edia.core",
  "events": [
    {
      "group": "state-machine",
      "name": "EvStartExperiment",
      "payload": "none",
      "description": "Request experiment start through the EDIA event layer."
    },
    {
      "group": "data-handlers",
      "name": "EvStoreMarker",
      "payload": "string marker",
      "description": "Request marker storage or forwarding through configured handlers."
    }
  ]
}
```

The exact names and payload descriptions should come from EDIA source and
maintainer review. The important part is making the event bus visible as data,
not only as C# source.

### Add Batchmode Installer Entry Points

The public installer README describes a Unity Editor script and menu flow. For
agents and CI, a thin batchmode layer would be enough:

```csharp
public static class EdiaInstallerBatch
{
    public static void ExportPackageStatus();
    public static void InstallCore();
    public static void InstallProfileCoreEyeRcas();
    public static void ImportSamples();
}
```

Documented command examples could then look like:

```powershell
Unity.exe -batchmode -projectPath <project> -executeMethod Edia.Installer.EdiaInstallerBatch.ExportPackageStatus -quit
Unity.exe -batchmode -projectPath <project> -executeMethod Edia.Installer.EdiaInstallerBatch.InstallProfileCoreEyeRcas -quit
```

The first method should be read-only. Mutating methods should require explicit
operator intent and write a clear report.

### Add A Small CLI Planner

The first EDIA CLI does not need to embed Unity or mutate projects. It can
start as a read-only planner over the module catalog and API/event surfaces:

```powershell
edia doctor
edia modules list
edia modules graph
edia install plan --profile core-eye-rcas
edia samples list --module core
edia api surface --module core --json
edia api events --module core --json
edia docs status --module core
edia project validate --project <unity-project>
edia broker bridge plan --json
```

Later, controlled commands can call Unity batchmode:

```powershell
edia install run --profile core-eye-rcas --project <unity-project>
edia samples import --module lsl --project <unity-project>
edia docs doxygen --module core
```

The key safety rule is that `plan` commands should be useful before `run`
commands exist.

### Define Operation Safety Categories

An operation catalog can feed both CLI and MCP wrappers:

```json
{
  "operation": "edia.modules.list",
  "owner": "core",
  "safety": "read-only",
  "description": "List known EDIA modules from the module catalog."
}
```

Side-effecting operations should return blocked plans by default:

```json
{
  "operation": "edia.install.profile",
  "owner": "installer",
  "safety": "unity-project-mutating",
  "description": "Install EDIA packages into a Unity project through Unity batchmode."
}
```

Planner output:

```json
{
  "operation": "edia.install.profile",
  "blocked": true,
  "reason": "Unity-project-mutating operation requires explicit user approval.",
  "planned_command": "Unity.exe -batchmode -projectPath ... -executeMethod ..."
}
```

This is the same safety shape Rusty XR Companion uses: read-only operations can
execute, while install, launch, write, capture, and device-changing operations
remain behind explicit plans.

### Add Read-Only MCP Wrappers Last

Once EDIA has a module catalog and operation planner, an MCP layer can stay
small:

| MCP tool | Purpose |
| --- | --- |
| `edia_modules_list` | List EDIA modules and dependencies |
| `edia_api_surface` | Return generated API surface |
| `edia_events_catalog` | Return event constants and payload notes |
| `edia_install_plan` | Plan an install without executing |
| `edia_project_status` | Inspect a Unity project manifest |
| `edia_docs_status` | Check Doxygen generated docs presence |
| `edia_rcas_summary` | Summarize RCAS transport/config surfaces |
| `edia_broker_bridge_plan` | Summarize possible bridge boundaries |

The MCP server should not mutate Unity projects by default. It should return
blocked plans for package install, sample import, Doxygen generation, scene
changes, build output, device access, log capture, and participant-data paths.

### Add Agent-Oriented CI Checks

Useful first checks:

- validate every package manifest
- validate the module catalog against its schema
- check generated API/event surfaces are current
- check Doxygen config paths exist
- check repo-local docs links
- scan for generated Unity folders and local artifacts
- scan for participant/session data, device logs, local paths, and SDK binaries
  that should not be committed

Unity test execution can remain a later or opt-in target. Metadata and docs
checks are cheaper and still help contributors immediately.

## Broker Bridge Agent Surface

If EDIA maintainers are interested in broker collaboration, the agent-readable
surface should start with planning operations, not runtime mutation:

```text
edia.broker_bridge.plan
edia.broker_bridge.events_surface
edia.broker_bridge.eye_surface
edia.broker_bridge.lsl_surface
edia.broker_bridge.rcas_surface
```

Read-only CLI examples:

```powershell
edia broker bridge plan --json
edia api events --module core --json
edia api eye-schema --module eye --json
edia api rcas-transport --json
```

Future side-effecting examples should stay blocked by default:

```powershell
edia broker bridge add-unity-client --project <path>
edia broker bridge import-sample --project <path>
```

For Rusty XR, the matching public commitment is:

- keep the broker contracts engine-neutral and source-only
- publish schemas for stream manifests, sample headers, eye samples, commands,
  acknowledgements, and replay records
- keep preview-media streams on explicit binary lanes that expose codec config,
  keyframe state, source timestamps, queue/drop counters, and decode/import
  timing as telemetry instead of hiding them inside a renderer
- make Unity adapter proposals reviewable as docs before writing adapter code
- use EDIA maintainers' preferred names and boundaries for EDIA-owned concepts
- avoid implying EDIA endorsement or replacing EDIA's Unity experiment model
- keep proprietary SDK access and participant data outside public Rusty XR core

## Why This Split

EDIA is strongest when a researcher needs a Unity-centered experiment
workflow. Rusty XR's broker work is useful when multiple engines or tools need
to consume the same time-stamped stream shape, replay a session, inspect
drop/jitter behavior, or publish to lab/creative protocols from a sidecar.

That means the bridge should stay thin:

```text
Unity / EDIA side
  - experiment flow
  - scene object meaning
  - trial/block/session semantics
  - local SDK access when Unity is the right SDK host

Rusty XR broker side
  - stream registry
  - timing and sequence metadata
  - validation
  - replay fixtures
  - diagnostics
  - optional protocol fan-out
```

## Feedback Requested From EDIA Maintainers

The most useful early feedback would be:

- whether the role split above matches how EDIA maintainers would want an
  external broker to interact with EDIA
- whether `eye.screen.gaze_point`, `eye.xr.local_ray`, `eye.xr.world_ray`,
  `eye.screen.aoi_hit`, and derived fixation/dwell/blink streams are useful
  names and boundaries
- whether broker session metadata should mirror any EDIA/UXF concepts by name,
  or keep those concepts app-owned and only carry external markers
- what a minimal Unity adapter should accept and emit before it becomes too
  large
- what attribution, non-endorsement language, and contribution routing would
  make collaboration comfortable for EDIA maintainers

## Proposed Next Public Steps

1. Expand JSONL replay fixtures from synthetic broker and eye streams.
2. Add small processor tests for validation, blink/dropout detection, AOI hit
   counting, and deterministic replay output.
3. Add sanitized preview-stream fixture metadata that mirrors the broker
   H.264 control/data split without including headset captures or app-specific
   renderer behavior.
4. Keep the minimal Unity adapter shape reviewable in
   [UNITY_BROKER_ADAPTER_CONTRACT.md](UNITY_BROKER_ADAPTER_CONTRACT.md).
5. Ask EDIA maintainers to review the bridge boundary and stream naming before
   a public adapter spike.
6. Only after that, build source-only adapter prototypes with explicit license
   and attribution checks.

## PR-Ready Maintainer Checklist

Before opening or reviewing a broker-bridge pull request, check:

- The PR states that it is not an endorsement, affiliation, or compatibility
  claim.
- The PR links to the bridge boundary and Unity adapter contract docs.
- New stream names, event names, and payload fields are reviewable as schemas
  or examples, not only as code.
- Live-device, tracker-SDK, package-install, scene-mutation, and participant
  data paths are out of scope unless explicitly requested.
- Synthetic replay fixtures pass the repo-local fixture checker.
- Screen-space gaze, XR gaze rays, AOI hits, fixation/dwell/blink events, and
  broker command envelopes remain separate concepts.
- Unity-side code keeps scene semantics and experiment flow inside Unity.
- Rusty XR code stays source-only and engine-neutral unless the PR is clearly
  an optional adapter.
- Third-party license and attribution notes are updated if any upstream source
  is copied or ported.
- Public docs do not include local paths, device identifiers, participant data,
  generated captures, private package names, signing material, or tracker SDK
  binaries.

## Boundaries

Public Rusty XR core should not include proprietary SDKs, private package
identities, generated captures, participant data, local validation logs, or
device-specific license keys. Provider work that forwards or records eye data
requires a current license and field-of-use review for the target hardware and
SDK before it becomes public code.
