# Screen-Space And Blur Alignment Workflow

This is the public Rusty XR workflow for aligning the reusable camera projection
area and the public diagnostic blur layer before downstream apps add
app-specific visual effects.

Rusty XR is the canonical place to finish this base work. Downstream apps should
return to their custom effect stacks only after the public raw lanes agree on
screen-space coverage, invalid-region policy, stereo orientation, and the
generic blur diagnostic.

The active synthetic-first coordinate gate is tracked in
[SYNTHETIC_PROJECTION_COORDINATE_ALIGNMENT_PLAN.md](SYNTHETIC_PROJECTION_COORDINATE_ALIGNMENT_PLAN.md).
Use that plan before starting physical camera or passthrough-underlay
alignment.

## Scope

This workflow owns:

- raw stereo projection area alignment across Vulkan/HWB, OpenGL/OES, and
  Makepad CPU-YUV lanes;
- the two-area render convention: full submitted XR surface, with camera pixels
  only inside the projected camera region;
- invalid-region policy: solid diagnostic color for segmentation or transparent
  passthrough underlay for operator alignment;
- independent opacity controls for the projected camera area and the surrounding
  border/matte region, so the same full submitted surface can be compared
  against native passthrough without changing geometry;
- broker-synthetic H.264 stimuli for deterministic projection, color/luma,
  temporal, and blur checks;
- the public 9-tap diagnostic blur layer as a processing-stack witness;
- the later physical-screen stimulus procedure used to compare custom camera
  projection against native passthrough.

This workflow does not own downstream color remaps, displacement, product
effects, private tuning constants, generated APK identities, or local artifact
payloads.

## Lane Set

Use the canonical lane names from
[QUEST_RAW_CAMERA_STACK_ALIGNMENT_WORKFLOW.md](QUEST_RAW_CAMERA_STACK_ALIGNMENT_WORKFLOW.md):

| Lane | Natural input path | Render path |
| --- | --- | --- |
| `vulkan-hwb-direct-camera2-raw` | Camera2 -> `ImageReader.PRIVATE` / `HardwareBuffer` | Vulkan + OpenXR |
| `vulkan-hwb-broker-h264-raw` | Broker Camera2 or synthetic H.264 -> MediaCodec hardware buffer | Vulkan + OpenXR |
| `gles-oes-direct-camera2-raw` | Camera2 -> `SurfaceTexture` / `GL_TEXTURE_EXTERNAL_OES` | OpenGL ES + OpenXR |
| `gles-oes-broker-h264-raw` | Broker Camera2 or synthetic H.264 -> MediaCodec -> `SurfaceTexture` / OES | OpenGL ES + OpenXR |
| `makepad-cpuyuv-direct-camera2-raw` | Camera2 -> CPU YUV planes | Makepad/OpenXR |
| `makepad-cpuyuv-broker-h264-raw` | Broker Camera2 or synthetic H.264 -> MediaCodec CPU YUV planes | Makepad/OpenXR |

Do not collapse these into a single transport. The comparison is useful because
each lane uses the ingest path that matches its real architecture.

## Synthetic First

Start with broker-synthetic H.264 before using the physical camera. Synthetic
input removes room lighting, exposure, autofocus, headset pose, display
brightness, and physical-screen placement from the first-pass diagnosis.

Use these public patterns:

- `diagnostic-grid`: static color bars, luma ramp, checkerboard structure, and a
  checker-anchored thin-line stimulus for projection and blur checks.
- `motion-bar`: the same base grid plus a moving marker driven by frame index,
  for repeated-frame, stale-frame, adoption, and left/right temporal-sync
  checks.

Recommended strict target:

```text
source mode: broker-synthetic
pattern: diagnostic-grid or motion-bar
projection profile: camera-matched for camera-shape parity, full-frame-diagnostic for projection-space coverage
width/height: 1280x1280
bitrate: 6000000
left/right ports: 8879 / 8880
max packets: 0
capture duration: live/unbounded
requested source fps: 50 when testing camera-realistic cadence
```

Record observed packet, decode, texture-update, and render cadence. The
requested source fps is not proof; the observed cadence is the result.

Synthetic source geometry is explicit:

- `head-anchored-virtual-camera` is the backwards-compatible diagnostic mode.
  It models a synthetic camera with a public 60 degree vertical field of view.
- `camera-matched` keeps synthetic pixels but asks the broker to attach the
  selected Camera2 intrinsics and pose. Use it when broker-synthetic should
  have the same projected shape as the direct camera path.
- `full-frame-diagnostic` treats the synthetic raster as projection-surface
  diagnostic content. Use it when testing full-frame orientation, borders, and
  projection-area separation rather than camera-shaped projection.

All three profiles must still respect the two-area render convention: the
submitted XR surface can be full size, while the camera/projection content is
visible only inside the projection area and the surrounding matte/border remains
under the selected invalid-region policy.

## Projection Area Pass

For screen-space alignment, keep the processing layer set to `raw`.

Use `solid-red` invalid-region policy for automated analysis:

- image-derived active fraction;
- bounding boxes;
- row spans;
- mask or content-envelope IoU;
- visible differences between the projected camera footprint and the full
  submitted layer.

The solid-red policy must be a hard projection-area mask. If the captured
outside-projection region contains feedback color, feathering, or camera samples,
reject the run and fix the border-mode routing before measuring offsets.

Use `passthrough-underlay` invalid-region policy for operator alignment against
native passthrough:

- the submitted XR surface remains full size;
- the camera projection remains a sub-area;
- invalid projected pixels write transparent alpha so the native passthrough
  underlay can show through;
- changing the border policy must not move, scale, or crop the camera
  projection area.

For red-border passthrough alignment, keep the invalid-region policy at
`solid-red`, enable the native passthrough underlay, and sweep projection-area
opacity from `1.0` to `0.0`. The border opacity is a separate control. This mode
is useful when the operator needs to see the exact projected-camera window
against Meta passthrough while retaining a hard red outline/matte.

If a lane appears vertically or horizontally offset, record it as a projection
space finding. Do not compensate with downstream visual-effect parameters.

## Blur Pass

Only after the raw projection area is classified, switch the processing layer to
`blur` and keep every other geometry, border, source, and cadence setting stable.

The public blur layer is a generic diagnostic witness:

- valid camera samples are blurred with a small 9-tap kernel;
- invalid projected pixels keep the selected border policy;
- the blur radius is controlled by the lane's public runtime key;
- the layer is intended for comparing processing-stack behavior, not for
  reproducing a downstream app's effect recipe.

Use the same `diagnostic-grid` static packet for checker/thin-line blur
behavior, then `motion-bar` when checking whether blur processing interacts with
stale/repeated frames.

## Public Control Map

| Renderer family | Raw border control | Projection-area opacity | Border opacity | Blur control |
| --- | --- | --- | --- | --- |
| Vulkan/HWB | `rustyxr.cameraPipelinePreset=raw-projection-solid-red-unorm` or `raw-projection-underlay-unorm` | `rustyxr.cameraProjectionAreaOpacity` | `rustyxr.cameraProjectionBorderOpacity` | `raw-projection-blur-solid-red-unorm` or `raw-projection-blur-underlay-unorm`, plus `rustyxr.cameraBlurRadiusPx` |
| GL/OES | `rustyxr.projectionBorderPolicy=solid-red` or `passthrough-underlay` | `rustyxr.projectionAreaOpacity` | `rustyxr.projectionBorderOpacity` | `rustyxr.processingLayer=blur`, plus `rustyxr.cameraBlurRadiusPx` |
| Makepad CPU-YUV | `debug.rustyxr.makepad.projection.border.policy=solid-red` or `passthrough-underlay` | `debug.rustyxr.makepad.projection.area.opacity` | `debug.rustyxr.makepad.projection.border.opacity` | `debug.rustyxr.makepad.processing.layer=blur`, plus `debug.rustyxr.makepad.blur.radius.px` |

The suite-level `-ProjectionAreaOffsetYUv` parameter forwards the same
screen-space vertical sweep intent to each renderer through its native key. Use
it only after the raw hard-mask evidence is valid.

For alignment runs, do not launch `fast075` HWB profiles. Those names belong to
older performance comparisons and can hide a `cameraProjectionScale=0.75`
footprint. Use the full-feed alignment profiles selected by the suite and keep
any deliberate scale reduction explicit in the command summary.

Prefer the full-suite runner for comparable public runs:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\tools\quest-camera-profile\Invoke-RawCameraStackAlignmentSuite.ps1 `
  -Serial <quest-serial> `
  -CompositeApk <composite-apk> `
  -GlesApk <gles-apk> `
  -MakepadApk <makepad-apk> `
  -MakepadPackageName <makepad-package> `
  -MakepadLauncherActivity <launcher-activity> `
  -MakepadXrActivity <xr-activity> `
  -BrokerH264SourceMode broker-synthetic `
  -BrokerH264SyntheticPattern diagnostic-grid `
  -BrokerH264SyntheticProjectionProfile camera-matched `
  -ProjectionBorderPolicy solid-red `
  -ProcessingLayer blur `
  -BlurRadiusPx 2.0
```

Reserve shared device, ADB, build, foreground, and broker-port resources with
the local coordination system before running headset-bound commands. The suite
does not reserve resources and does not change power, stay-awake, or proximity
state unless explicitly requested through its documented guard switches.

## Evidence Packet

For each alignment slice, keep the generated evidence under ignored
`artifacts/` folders and summarize:

- lane name and profile alias;
- source type, synthetic pattern, resolution, bitrate, ports, requested fps,
  and observed cadence;
- border policy and processing layer;
- projection-stage rows when available: `screen_to_surface`,
  `surface_to_camera`, and `screen_to_camera`;
- image-derived footprint rows when available;
- texture update path: hardware-buffer import, OES `updateTexImage()`, or
  CPU-YUV upload;
- OpenXR cadence, repeated render frames, skipped texture updates, and fatal or
  GPU fault markers;
- whether the evidence came from broker synthetic input or physical camera
  input.

For raw-stack suite runs that use `solid-red` invalid fill, derive repeatable
screen-space coordinates from the captured screenshot pixels:

```powershell
python .\tools\quest-camera-profile\Analyze-RawStackScreenSpace.py `
  .\artifacts\raw-stack-suite\<session-id>
```

The analyzer writes `screen-space-analysis\screen-space-summary.md`,
`screen-space-report.json`, and a contact sheet. Coordinates are reported in
the screenshot output space with origin at the top left, `x` increasing right,
and `y` increasing down. Positive vertical center offset means the detected
camera projection area is below the center of that eye's screenshot half.
If solid-red invalid fill is not present in a `solid-red` lane, the analyzer
marks the lane ambiguous. Visible-content fallback is reserved for transparent
underlay/operator runs and must not be treated as a strict valid-mask footprint.
When logcat evidence is present in the lane artifact, the analyzer also records
the latest source fields and available projection-stage rows
(`screen_to_surface`, `surface_to_camera`, and `screen_to_camera`) so
broker-synthetic tri-stack packets can be compared without re-opening the raw
logs by hand.

Do not treat MediaProjection, ADB screenshots, or browser event logs as
submitted per-eye render-target proof. They are useful witnesses, but they need
to be labeled as final-display or correlation evidence.

## Physical-Screen Stimulus Later

After the synthetic projection and blur packets are coherent, use the Brave
browser stimulus workflow for final physical-camera alignment against native
passthrough.

Use:

```powershell
python .\tools\quest-visual-stimulus\run-sync-stimulus.py `
  --session-id <session-id> `
  --server-control
```

Keep Brave fullscreen and foreground during the active interval. The stimulus
session files under `artifacts/sync-stimulus/<session-id>/` are correlation
evidence for color, luma, motion, and timing, not headset-frame proof. Reject or
bracket physical-screen runs where the event log reports `NOFS` or `BG` during
active capture unless the run deliberately allowed windowed operation.

The physical-screen pass is the right time to record a native-passthrough
reference video of the real browser stimulus. That reference should be used
later to judge downstream custom modes. It should not replace the synthetic
screen-space and blur diagnostics.

## Stop Line

Do not start downstream effect tuning until:

1. raw projection area is aligned or explicitly bracketed across the public
   lanes;
2. solid-red and passthrough-underlay border policies are understood for each
   lane;
3. broker-synthetic `diagnostic-grid` and `motion-bar` evidence is recorded for
   the raw layer;
4. the public diagnostic blur layer has been compared with the same geometry
   and source settings;
5. any remaining physical-camera differences are classified as projection,
   source, cadence, capture-witness, or downstream-effect issues.
