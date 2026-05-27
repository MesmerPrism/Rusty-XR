# Camera Texture Lane Parity Plan

This plan makes the direct Vulkan/HWB, direct GLES/OES, Makepad CPU-YUV, and
Makepad HWB external paths comparable without forcing them to share one renderer
implementation. The target is modular parity: every lane reports the same
diagnostic contract while each adapter keeps its native resource ownership.

## Canonical Lane Vocabulary

Use these lane-family identifiers when mapping runner profiles, log markers,
schema records, and summaries:

| Lane family | Stable id | Source handoff | Resource/shader shape |
| --- | --- | --- | --- |
| Direct HWB | `vulkan-hwb-direct-camera2-raw` | Camera2 `ImageReader.PRIVATE` / `AHardwareBuffer` | Vulkan hardware-buffer import; direct renderer owns descriptor contract |
| Direct OES | `gles-oes-direct-camera2-raw` | Camera2 `SurfaceTexture` / `GL_TEXTURE_EXTERNAL_OES` | GLES external-OES sampler with `SurfaceTexture` transform |
| Makepad CPU-YUV | `makepad-cpuyuv-direct-camera2-raw` | Camera2 CPU YUV planes | Makepad Y/U/V plane textures; accepted visual color reference |
| Makepad HWB external | `makepad-hwb-external-direct-camera2-raw` | Camera2 `AHardwareBuffer` | Makepad Vulkan video texture; current shader shape remains sampled image plus sampler |

Broker and synthetic source variants should map into the same lane contract by
changing source fields, not by inventing new resource-family names unless the
texture/resource architecture changes.

## Shared Contract

The public Rust contract is `CameraTextureLaneContract` in
`rusty-xr-contracts`. Its schema id is
`rusty.xr.camera-texture-lane-contract.v1`, exported as
`camera-texture-lane-contract.schema.json`. Host-side log evidence can be
converted into `camera-texture-lane-contracts.jsonl` with:

```powershell
python tools\quest-camera-profile\Build-CameraTextureLaneContracts.py <run-root>
```

The builder scans existing public HWB, OES, and Makepad marker lines. It does
not change renderer behavior.

For Makepad lanes, `VideoTextureUpdated` now carries optional texture-update
metadata for camera source identity, camera frame identity,
acquire/upload/import timing, resource path, descriptor shape, Vulkan format
facts, and fallback state. The Rusty XR Makepad adapter should prefer those
event fields when emitting markers or lane-contract artifacts, and use older
marker inference only as a compatibility fallback for old evidence bundles.

Each record separates these concerns:

| Section | Required facts |
| --- | --- |
| `source` | Source kind, public source label, delivered size, handoff label, source-eye mapping, optional camera input/format ids |
| `resource` | Texture resource kind, descriptor shape, texture label, optional buffer/import-cache identity, shader interface label |
| `transform` | Visible source UV rect, transform stage, transform owner, OES matrix or HWB flags or YUV rotation when applicable |
| `color` | Accepted/experimental/diagnostic status, color reference, matrix/range/transfer labels |
| `timing` | Camera frame identity, acquire/upload/import timestamps, texture update/submit ids, optional `xrEndFrame` stamp |
| `lifecycle` | First-frame state, fallback state, frame reuse policy, resource release policy, optional focus state |
| `projection` | Border policy, processing layer, projection surface label, status label |

Color acceptance is intentionally separate from resource correctness. Makepad
CPU-YUV can remain the accepted visual reference while Makepad HWB external is
resource-cadence comparable but color-experimental.

## Perfetto Deep-Trace Tier

Perfetto capture belongs above the lane contract as an optional explanation
tool. The default gate should stay lightweight: camera texture lane summaries,
freshness screenshots, Meta stale counters, and focused log markers are the
normal pass/fail evidence. Use Perfetto only when those lighter signals need a
cause attribution pass, such as CPU-YUV raw versus blur at the same render
scale, Makepad CPU-YUV versus HWB external under the same clean settings, or a
GPU/CPU scheduling question that cannot be answered from lane timing fields.

Build a host-side plan artifact before capturing:

```powershell
python tools\quest-camera-profile\Build-PerfettoTracePlan.py `
  --mode capture `
  --provider hzdb `
  --preset lightweight `
  --intended-use effect-layer-ab `
  --artifact-dir artifacts\quest-camera-profile-runs\<run>\perfetto `
  --out artifacts\quest-camera-profile-runs\<run>\perfetto\perfetto-trace-plan.json
```

The emitted plan uses `rusty.xr.camera-perfetto-trace-plan.v1`, records the
provider and overhead policy, and keeps raw `.pftrace` files in ignored
artifact folders. Commit only normalized contracts, summaries, or docs.

## Implementation Sequence

1. Freeze all runner summaries and new marker work on the four lane-family
   names above. Keep older profile ids as aliases only at adapter boundaries.
2. Teach direct Vulkan/HWB diagnostics to emit or export
   `CameraTextureLaneContract` records from existing hardware-buffer import,
   transform-flag, render-frame, and projection-status facts.
3. Teach direct GLES/OES diagnostics to emit the same contract from existing
   `SurfaceTexture` timestamp, update count, transform matrix, color-transfer,
   swapchain-format, and projection-status facts.
4. Promote Makepad CPU-YUV frame identity into the adapter contract. The
   Makepad event side should expose camera input/format ids, camera frame
   id/timestamp, and upload id/timestamp so the Rusty XR shell does not infer
   freshness only from `VideoTextureUpdated` plus `yuv.enabled`.
5. Promote Makepad HWB external resource identity the same way. Carry camera
   input/format ids, AHB frame timestamp, import/update id, Vulkan
   format/external format, fallback state, and descriptor-shape evidence as
   contract fields.
6. Replace scattered Makepad shell path inference with one adapter that converts
   Makepad events into `CameraTextureLaneContract`.
7. Align focused Makepad gate summaries with direct HWB/OES summaries:
   freshness, strict stale, acquire-to-upload/import, upload/import-to-submit,
   frame reuse, color status, and descriptor shape.
8. Document resource lifetime rules by adapter:
   HWB acquire/release/import cache, OES frame-available/updateTexImage/transform
   ownership, CPU-YUV latest-frame/upload/reuse, and Makepad HWB
   import/reimport/fallback.
9. Only after the contract records agree, change shader or descriptor
   architecture. In particular, a Makepad HWB combined-image-sampler path must
   change shader/SPIR-V resource lowering and Vulkan descriptor layout together.

## Validation Posture

Contract-only changes should use host validation: focused crate tests, schema
export checks, and workflow safety checks when scripts change. Device gates are
reserved for adapter or renderer changes that affect actual acquisition,
upload/import, submission, or visual output.
