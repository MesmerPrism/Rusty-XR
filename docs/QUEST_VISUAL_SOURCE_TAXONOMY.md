# Quest Visual Source Taxonomy

Rusty XR keeps raw headset-camera compositing, runtime environment depth,
native passthrough styling, rendered strobe stimuli, final-display inspection,
and operator streaming paths separate. These sources are related, but they are
not interchangeable.

The GitHub Pages summary for this boundary is
[passthrough.html](passthrough.html). Use this Markdown file for the detailed
technical taxonomy and keep public examples explicit about which source class
they use.

## Native Platform Passthrough Compositor

The platform passthrough compositor is the system-provided view users see in
mixed reality. It is useful and often the best user-facing background layer,
but it is not exposed to this public example as a sampleable app camera
texture. A custom camera projection layer must use camera frames that the app
can receive, timestamp, and sample.

Public style descriptors for reconstruction, projected passthrough, opacity,
edge rendering, mono color maps, brightness/contrast/saturation, and color LUT
bindings live in [META_PASSTHROUGH_LAYER.md](META_PASSTHROUGH_LAYER.md). Those
descriptors are adapter input, not proof that the app can sample the runtime's
final passthrough image.

## Rendered Strobe Stimuli

Intentional strobing is an app-rendered or compositor-style-parameter stimulus,
not a camera source. Public descriptors for full-field red/black flicker and
phase-inverted passthrough LUT flicker live in
[VISUAL_STROBE_PROFILES.md](VISUAL_STROBE_PROFILES.md). These profiles are
safety-gated research stimuli and must not be treated as normal UI animation,
camera diagnostics, or medical/wellness features.

## Raw Camera Sources

Camera2, headset-camera, or passthrough-camera style APIs are the relevant
input class for custom camera compositing when they expose frames and metadata.
For a useful projection path, the app needs GPU-importable image buffers,
timestamps, delivered image size, sensor orientation, intrinsics, and pose or
extrinsics.

The current `camera-gpu-buffer-probe` profile proves that Camera2 `PRIVATE`
`HardwareBuffer` frames can be imported into Vulkan and sampled without CPU
RGBA staging. If that provider resolves to a mono stream, that is a limitation
of the current provider path. It is not proof that custom stereo camera
composition is impossible on every runtime or API surface.

Live public Quest validations on this workspace found Camera2 sources `1`,
`50`, and `51`. Sources `50` and `51` were the useful back-facing concurrent
pair: each exposed GPU-importable `PRIVATE` output, intrinsic calibration, lens
pose fields, and up to `60 fps` Camera2 ranges. Delivered sizes can vary by
runtime and model; both `1280x1280` and `1280x960` have been observed. Treat
those IDs and sizes as diagnostics from that runtime, not as portable
requirements.

## Environment Depth

Environment depth is a runtime-generated depth texture exposed through the
public Depth API / `XR_META_environment_depth` style contract. It is not raw
depth-sensor data, raw infrared camera data, projector output, or a low-level
sensor-fusion feed. Public app code should treat it as a black-box depth image
with metadata supplied by the runtime.

The native OpenXR shape is provider-based: create the provider, start it,
acquire the latest available environment-depth image during the XR frame, use
the returned swapchain image and per-eye metadata, then release it with the
frame. A runtime may report that no depth image is available for the current
frame. Public code should model that as normal runtime state instead of a hard
failure.

The returned depth image metadata includes the depth swapchain index, `nearZ`,
`farZ`, and per-eye view pose/FOV data. Swapchain width and height are reported
separately. The texture should be treated as depth-buffer-style data: metric
reconstruction needs the supplied near/far values, per-eye FOV/pose, and the
adapter's projection or inverse-projection math. Do not assume raw samples are
already linear meters unless an adapter explicitly documents that conversion.
Some runtimes can report an infinite far plane; public contracts should treat
that as a valid runtime range and use an explicit visualization cutoff when
displaying the texture.

Quest device models can differ internally in how the runtime estimates depth,
but the public app-facing contract remains an environment-depth texture. Public
Rusty XR docs and contracts should not rely on raw depth hardware access,
infrared tracking frames, projector frames, or private compositor
representations.

Confidence should be optional in public contracts. The public Meta
environment-depth image structure does not require a separate confidence image,
so an absent confidence payload is a valid state. If an adapter derives a local
confidence signal from neighborhood consistency, edge checks, or temporal
stability, label it as app-derived rather than runtime-supplied.

The API exposes acquisition at most once per XR frame and the cadence of new
depth images is runtime-controlled. When an adapter needs latency or rate
measurement, store an explicit runtime capture timestamp when the platform
provides one. Do not infer the runtime's depth cadence from exported dataset
stride, display-stream cadence, or operator capture rate.

Observed depth resolutions and formats are useful diagnostics, not portable
requirements. A public frame descriptor should record width, height, format
such as `depth_u16le` or `D16_UNORM`, eye/layer identity, byte length,
near/far, per-eye view metadata, optional runtime capture time, and whether a
confidence payload exists.

The `environment-depth-diagnostics` profile in the public composite-layer
example is the first hardware-facing diagnostic for this source class. It
checks extension support, provider start, swapchain creation, acquire status,
runtime capture timestamp progression, repeated capture timestamps, observed
depth cadence, acquire CPU cost, near/far range, hand-removal support, and
explicit confidence-source reporting. Its current headset visual samples the
runtime `D16_UNORM` stereo depth texture directly as per-eye grayscale, maps
layer `0` to the left eye and layer `1` to the right eye, applies the same
rotate/flip UV transform semantics used by the camera projection shader, and
uses an explicit maximum-meter cutoff for infinite or very distant depth.

## Paired Stereo Camera Provider

The desired public Tier 2 path is a paired stereo provider:

- left and right GPU-importable buffers
- timestamps tracked against a soft pairing target
- per-eye delivered image domains
- per-eye intrinsics
- sensor orientation
- platform pose/extrinsics, or an explicit user-supplied public calibration
  profile

The public Camera2 bridge keeps the latest available left/right pair even when
separate streams exceed the soft timestamp target. That avoids starving the
OpenXR renderer with stale buffers; release validation must still inspect the
logged pair deltas, `softPairOverMax` count, and headset comfort before
claiming the provider is good enough for a device build.

Temporal smoothing is a separate layer on top of this provider. It may hold a
previous accepted stereo pair, clamp the visible projection toward a newer
target, or crossfade pairs for a short window, but it must not make the two
eyes advance independently. If smoothing is enabled, the renderer should keep
one shared stereo smoothing coefficient and report both target motion and
applied visual motion in scorecards.

The `camera-stereo-gpu-composite` profile is the public reference example for
this path on the tested Quest Camera2 provider. It must not pass verification
unless logs show `activeTier=gpu-projected`,
`alignedProjection=true`, `stereoLayout=Separate`, paired left/right GPU
buffers, and `poseSource=platform` or `poseSource=estimated-profile`.

The public renderer uses Camera2 platform pose first. Its coordinate convention
is recorded as `android-camera2-lens-pose-reference-from-camera`:
`LENS_POSE_TRANSLATION` is treated as the camera optical center in meters in
the `LENS_POSE_REFERENCE` frame. Camera2's `LENS_POSE_ROTATION` quaternion maps
that reference frame into the camera-aligned frame, so Rusty XR normalizes and
inverts it before storing `CameraExtrinsics.world_from_camera`.

For `LENS_POSE_REFERENCE_GYROSCOPE`, the origin is the primary gyroscope or
device reference reported by Android. It is useful camera metadata, but it is
not automatically an OpenXR display-eye or view-space pose. The public
projection chain is:

1. Camera2 pose-reference frame, such as gyroscope/device reference.
2. Camera-aligned frame from Camera2 lens pose rotation/translation.
3. Current OpenXR head/tracking basis, where Android sensor axes are resolved
   against the current head right/up/forward vectors instead of treated as
   already being display-eye space.
4. Per-eye view/surface frame from the current OpenXR view/FOV.
5. Source camera UV after intrinsics scaling and texture-orientation transform.

The renderer first projects the shared head-anchored camera-content surface
through each current OpenXR display-eye view/FOV into fullscreen display UV,
then composes that mapping with the selected Camera2 source projection. The
shader therefore receives one display-eye screen-to-camera homography per eye.
Source-eye mapping still controls which imported left/right camera texture is
sampled; homography selection is based on the display eye. Fullscreen shader UV
uses `y=0` at the top of the screen, so OpenXR tangent-space `+Y` maps to
screen UV `y=0`.

The visible full-view surface can be larger than the camera-content surface.
The shader expands full-view UV into content UV for the public projection
border while using display UV for camera projection. This keeps the
camera-covered region and the projection exterior fill as separate public
concepts, which is important when the camera feed is 4:3 but the eye swapchain
is not.

The public example currently exposes two projected stereo render mappings. The
default `display-screen-homography` mapping is the accepted public baseline:
it draws a fullscreen multiview Vulkan pass and maps each display-eye pixel
through the head-anchored content surface into the selected Camera2 source. The
`quad-surface` mapping is an A/B comparison profile that reconstructs the
content-surface coordinates a real head-anchored quad would rasterize before
performing the same camera projection. Both modes now share the same paired
Camera2 buffers and camera-driven projection-border coordinates. The
quad-surface mode is intentionally still marked as visually gated because
performance and final color parity with optimized downstream renderers remain
open work.

Camera color and renderer-performance experiments are separate from projection
mapping. The baseline projected profile uses Vulkan `PRIVATE` hardware-buffer
import, `external-rgb`, fixed foveation off, render scale `0.75`, and the same
public border and projection path. The `0.65` performance profile changes only
OpenXR render scale to test fragment-load headroom. The shader-side YCbCr
profile changes only the color decode assumption to `Cr/Y/Cb` BT.601 narrow
range, and should be used only when hardware-buffer diagnostics show the
sampler is exposing channel-packed data instead of converted RGB. The
fixed-foveation profile changes only the OpenXR fragment-density-map path and
remains experimental until headset logs show a created foveated swapchain,
valid foveation image handles, no framebuffer or driver failure, and stable
frame cadence.

Recent public profile testing narrowed the next useful axis. On the current
combined immutable-sampler Vulkan path, `external-rgb` is the usable public
baseline and shader-side `Cr/Y/Cb` decode can produce a green/discolored image
because the sampler is already presenting RGB-like values. Follow-up
acquisition probes kept projection, border, sampler, and color controls stable
while changing one Camera2 parameter at a time. No explicit AE FPS target,
separate-eye `ImageReader` max images reduced to `3`, a wider stereo-pair
window, and a lower `1280x960` separate-eye size did not fix stale progression
in the concurrent-separate Java Camera2 stereo path on the tested runtime. A
mono Camera2 `PRIVATE` GPU-buffer probe at `1280x960` continued to deliver live
frames, so the next public split should compare Java Camera2 concurrent stereo
against a lower-level/native hardware-buffer reader path without changing the
projection or border surface.

That lower-level path is now represented as an opt-in native acquisition
probe, not as a replacement for the Java Camera2 example. It uses Android NDK
`ACamera*` sessions and `AImageReader` `PRIVATE` GPU-sampled buffers, logs the
native camera sources it sees, and publishes the latest stereo pair into the
same Vulkan projection path. Early headset runs showed that this ownership
shape alone does not guarantee fresh stereo progression; the remaining
acquisition diff is the effective source/session/timestamp behavior.

A second native profile, `single-back-mirror`, intentionally opens only one
native camera source and mirrors the same hardware buffer into both display
eyes. It is not a valid stereo alignment path, but it is an important taxonomy
entry because it isolates acquisition source cadence from the Vulkan import
and OpenXR renderer. When mirror mode receives live frames but the full native
stereo profile does not, the next investigation should stay on camera
source/provider policy and side-camera timestamp behavior rather than
projection or border geometry.

OpenXR passthrough-client probing belongs in a separate bucket. Optional
passthrough and scene manifest declarations can make `XR_FB_passthrough`
available, and `client` / `warmup` probes can verify runtime client state, but
they do not change the raw-camera color model or prove that camera buffers are
fresh.

The renderer then applies an explicit `CameraTextureTransform` after projection
UV calculation and before sampling imported camera textures. That transform is
separate from Camera2 `SENSOR_ORIENTATION`; opaque GPU camera buffers can need a
texture-space rotate/flip even when sensor orientation is `0`.

On the current public Quest Camera2 hardware-buffer projection path, the
projected stereo profile uses `rotate0` with no post-projection flip for both
eyes. `FlipY` remains available as a diagnostic override for other devices or
driver paths, but it is not the default release-candidate orientation because
the projected shader path already receives camera UVs in the expected vertical
orientation.

The public profile supports independent left/right texture transforms and a
source-eye mapping knob. Use `rustyquest.cameraSourceEyeMapping=left-right` or
`right-left` to test which Camera2 source belongs to each display eye. Use
`rustyxr.cameraOrientationDiagnosticMode=cycle-source-eye-mapping`,
`cycle-left-texture-transform`, `cycle-right-texture-transform`, or `cycle-all`
only for live visual diagnosis; a cycling profile is not a release-ready
orientation proof.

The example rejects non-finite or undefined pose values and logs
`poseSource=platform` only when both stereo eyes have valid pose. It logs
`alignedProjection=true` only when paired GPU buffers, per-eye projection
metadata, the projected shader path, and explicit per-eye texture orientation
are active. Release verification still requires `visualReleaseAccepted=true`
with the manual acceptance token in the same final status line. A public
estimated calibration profile may be supplied through launch extras, but the
repo does not bake in device-private calibration constants.

Logs alone are not a release gate. The final stereo profile still requires
manual headset or cast inspection: the feed must be upright, the soft border
must be visible and camera-driven, and the two eyes must not be obviously
swapped or divergent. If a display capture is unavoidable for debugging, keep it
local under ignored artifact roots. Screenshots, log bundles, APKs, and
`artifacts/**` must not be staged or published.

## MediaProjection

MediaProjection is a final app/display composite stream. It is user-consent
gated and may be monoscopic, cropped, scaled, encoded, or include already
composited overlays. It is useful for diagnostics and Windows/operator
streaming. It is not a raw camera source and cannot provide the two camera
images required for custom stereo projection.

## scrcpy, Casting, Screenrecord

scrcpy, headset casting, and screenrecord are operator inspection routes for
the final display/compositor output. They are useful for checking what the
headset is presenting, but they are not replacements for raw left/right camera
streams and do not provide the metadata needed for camera/view alignment.

## Diagnostic Order

Before declaring a stereo blocker, run the `camera-source-diagnostics` runtime
profile. It enumerates camera IDs, physical camera IDs, logical multi-camera
capability, concurrent-camera exposure, `PRIVATE` and YUV output sizes, FPS
ranges, lens facing, active-array and sensor-pixel domains, calibration fields,
pose fields, selected stereo-pair score/reason, and stereo candidate
accept/reject reasons. Companion verification pulls the APK's app-private diagnostics payload as
`camera-source-diagnostics.json` when an output bundle is enabled.
