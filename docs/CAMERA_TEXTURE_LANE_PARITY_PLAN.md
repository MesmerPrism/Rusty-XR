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
`camera-texture-lane-contract.schema.json`.

Each record separates these concerns:

| Section | Required facts |
| --- | --- |
| `source` | Source kind, public source label, delivered size, handoff label, source-eye mapping |
| `resource` | Texture resource kind, descriptor shape, texture label, optional buffer/import-cache identity, shader interface label |
| `transform` | Visible source UV rect, transform stage, transform owner, OES matrix or HWB flags or YUV rotation when applicable |
| `color` | Accepted/experimental/diagnostic status, color reference, matrix/range/transfer labels |
| `timing` | Camera frame identity, acquire/upload/import timestamps, texture update/submit ids, optional `xrEndFrame` stamp |
| `lifecycle` | First-frame state, fallback state, frame reuse policy, resource release policy, optional focus state |
| `projection` | Border policy, processing layer, projection surface label, status label |

Color acceptance is intentionally separate from resource correctness. Makepad
CPU-YUV can remain the accepted visual reference while Makepad HWB external is
resource-cadence comparable but color-experimental.

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
   Makepad event side should expose camera frame id/timestamp and upload
   id/timestamp so the Rusty XR shell does not infer freshness only from
   `VideoTextureUpdated` plus `yuv.enabled`.
5. Promote Makepad HWB external resource identity the same way. Carry AHB
   frame timestamp, import/update id, Vulkan format/external format, fallback
   state, and descriptor-shape evidence as contract fields.
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
