# Interactive Media Route Policy

Rusty XR models interactive media routes as broker-visible contracts, not as a
media runtime. The public `rusty-xr-broker-model` crate can describe what a
route needs, who owns each resource, and what low-rate feedback says about a
session. It does not load codecs, open sockets, import textures, submit OpenXR
layers, or bundle media SDKs.

## Four Plane Model

Every interactive media route manifest separates four planes:

- `control`: JSON command and event traffic for negotiation, permissions,
  leases, selected backend, and low-rate status.
- `media_data`: high-rate payload transport such as encoded video, raw samples,
  or binary diagnostic packets. This plane must not use the JSON command path.
- `render_adoption`: ownership and policy for decode, frame queues, texture or
  buffer import, projection metadata, stale-frame reuse, and XR submission.
- `feedback`: low-rate telemetry, scorecards, issue codes, and timing evidence
  that UI clients and operators can inspect without subscribing to media bytes.

The broker may authorize and observe a route, but the active XR app owns the
render-critical work: decode-to-texture or buffer import, eye/view metadata,
projection, frame pacing, and layer submission. That keeps the broker useful
for orchestration and diagnostics without moving display-frame responsibilities
into a JSON control service.

## Public Contracts

`rusty-xr-broker-model::media` adds these schema-only data contracts:

- `BrokerInteractiveMediaRouteManifest`: stable route id, direction, control
  scope, optional module id, source/output stream ids, plane descriptors,
  latency budget, frame-adoption policy, backend candidates, consent gates,
  command authority, and notes.
- `BrokerInteractiveMediaRouteRuntimeState`: current route lifecycle, revision,
  selected backend, per-plane state, optional feedback sample, optional
  scorecard, and issue codes.
- `BrokerMediaFeedbackSample`: low-rate RTT, jitter, packet loss, frame age,
  queue depth, delivered/dropped/reused frame counts, and issue codes.
- `BrokerMediaPipelineScorecard`: windowed latency, frame counts, stall/import/
  submit miss counters, score, verdict, and notes.

These contracts are deliberately data-only. They can be serialized with the
optional `serde` feature and exported as JSON Schemas, but they do not imply
that any backend exists in broker core.

## Backend And Dependency Policy

Backend descriptors are manifest metadata. A manifest may name a platform
codec, hardware path, reference lane, optimized path, or external sidecar as a
candidate so operators can compare selection and fallback decisions. That
metadata does not add a Rust dependency or copy a backend implementation.

Core crates must not depend on media runtimes or protocol stacks such as
WebRTC, RTSP/RTP, SRT, NDI, FFmpeg, GStreamer, native codec SDKs, OpenXR
renderers, Android lifecycle code, or UI frameworks. Those belong in examples,
adapter crates, tools, sidecars, or downstream app shells after dependency and
license review.

## UI And Registry Use

UI clients should treat media route manifests as discovery and diagnostics:

- Show route ownership and consent state before enabling route controls.
- Keep high-rate streams out of default auto-subscribe paths.
- Use feedback and scorecards for charts, warnings, and comparison tables.
- Require command-authority and lease checks before mutating a route.
- Preserve route revision and selected backend in evidence records.

Broker stream registry snapshots can link streams and providers to modules, but
stream ids remain the data-plane handles. A module link does not imply dynamic
plugin loading, media payload routing through JSON, or permission to run a
mutating command.

## Fixtures And Validation

Synthetic public fixtures live under `fixtures/broker-ui`:

- `synthetic-interactive-media-route-manifest.json`
- `synthetic-interactive-media-route-runtime-state.json`
- `synthetic-media-feedback-sample.json`
- `synthetic-media-pipeline-scorecard.json`

They describe a generic H.264-style diagnostic route shape without loading a
runtime backend or carrying media bytes.

Recommended validation for this contract surface:

```powershell
cargo test -p rusty-xr-broker-model
cargo test -p rusty-xr-broker-model --features serde
python tools\schema\export_schemas.py --check
python tools\schema\check_broker_ui_fixtures.py --repo-root .
python tools\boundary-scan\rusty_xr_boundary_scan.py --repo-root .
```
