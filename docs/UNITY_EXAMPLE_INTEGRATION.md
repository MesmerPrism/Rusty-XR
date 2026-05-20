# Unity Example Integration

Status: public repository relationship note for Unity examples that consume
Rusty XR broker contracts.

## Canonical Public Unity Example

The canonical public Unity comparison target is:

```text
https://github.com/MesmerPrism/the-big-red-button-institute
```

Use it when a Rusty XR change needs a Unity-side proof that broker-routed
stream events, replay records, and direct Unity inputs can drive the same scene
behavior. The Unity project owns scene semantics, object behavior, Unity
packages, Quest build settings, and edit-mode tests. Rusty XR owns public Rust
contracts, schema shapes, broker/source examples, companion-facing catalog
metadata, and device/operator diagnostics.

## Boundary

Rusty XR should not absorb Unity scene meaning, Unity package-manager state,
participant workflow, or app-specific visual behavior. Unity examples should
not redefine broker schemas or fork Rusty XR timing/drop/replay contracts.

The shared boundary is source-level and data-level:

- broker stream envelopes
- replay record JSONL shape
- OSC drive stream identity
- latency and acknowledgement timestamps
- companion diagnostics output
- public validation commands and expected failure modes

## Expected Source Layout

When both public repositories are checked out together, keep them as siblings:

```text
<workspace>\Rusty-XR
<workspace>\the-big-red-button-institute
```

That layout is only a convenience for local validation. Public docs and scripts
should continue to use repository-relative paths and public GitHub links.

## Validation Handshake

Use this repo to validate the Rust side first:

```powershell
python tools\schema\check_quest_app_catalog.py tools\schema\fixtures\quest-app-catalog.example.json
cargo test -p rusty-xr-broker-model -p rusty-xr-osc --features serde
```

Use the Unity example to validate the Unity adapter side:

```powershell
powershell -ExecutionPolicy Bypass -File .\Tools\Run-BrokerEditModeTests.ps1
```

For live Quest comparisons, build and launch the Rusty XR broker example with
the companion workflow, then use the Unity example as the visible target scene.
Keep headset serials, local IP addresses, generated reports, screenshots, and
captures out of public commits.

## Related Docs

- [Unity broker adapter contract](UNITY_BROKER_ADAPTER_CONTRACT.md)
- [Companion integration](RUSTY_XR_COMPANION_INTEGRATION.md)
- [Examples matrix](EXAMPLES_MATRIX.md)
- [Broker sidecar APK](../examples/quest-broker-apk/README.md)
