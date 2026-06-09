# Camera Stereo Projection Parity Workplan

This note tracks the public Rusty XR path for bringing custom stereo Camera2
projection from "correct and visible" to "smooth with real GPU headroom." It is
intentionally public-safe: downstream app names, package IDs, raw local
artifacts, visual-effect stacks, generated APK identities, and private tuning
remain outside this repository.

## Target Map

Rusty XR's public baseline is the Quest composite-layer profile
`camera-stereo-gpu-composite-performance-065`.

The comparison target is a validated downstream custom stereo camera run that
uses the same broad class of app-owned stereo camera projection: headset camera
frames are acquired by the app shell, imported as GPU-readable textures, mapped
per eye, and submitted through an OpenXR presentation path. The downstream run
is not a source for public visual effects, exact tuning, package identity, or
private launch behavior. It is evidence that the public projection path should
be able to keep correct stereo while leaving more frame budget.

## Public Baseline

The accepted public `0.65` profile demonstrates:

- visible in-app stereo Camera2 projection
- selected concurrent left/right Camera2 sources with `1280x1280` square
  GPU-importable buffers
- `activeTier=gpu-projected`
- `alignedProjection=true`
- `stereoLayout=Separate`
- paired left/right GPU buffers
- `cpuUploadCount=0`
- stable Vulkan hardware-buffer imports with no continuing import failures or
  cache evictions after warm-up
- public camera projection metadata, per-eye homographies, and platform pose
  metadata flowing into the projection shader

The remaining issue is performance headroom. The representative accepted run
held roughly `70.7/72 Hz` near the end of the capture, but `VrApi` GPU
utilization was effectively saturated and the validation bundle still contained
sleep/tear warnings. That makes the run useful as a correctness baseline, not
as the final smoothness target.

## Current Alignment State

The accepted screen-space alignment state is S119. The earlier S91-S118 slices
remain useful history for why the current tools exist, but coordinate parity is
no longer the active bottleneck for the Brave fixed-screen path: the Rusty XR
reference and borderless Makepad candidate stayed within roughly
`0.003-0.004 px` dense homography delta, and edge-stripe scores were zero in
both Makepad eyes. Treat remaining differences as presentation/effect/temporal
policy unless new evidence shows projection-row drift.

## Current Fast Public Profiles

The current renderer-parity profiles are:

- `camera-stereo-gpu-composite-scale075`: direct in-app Camera2 stereo
  projection with the same Quest custom stereo geometry and the direct
  raw-projection shader at render scale `0.75`.
- `camera-stereo-gpu-composite-scale065`: the same direct raw-projection renderer path at
  render scale `0.65`.
- `broker-h264-stereo-live-openxr-projection-scale075-probe`: broker-owned
  Camera2 `50`/`51` capture, square `1280x1280` H.264 frames, hardware-buffer
  decode/import, frame-order live stereo pairing, and the direct
  raw-projection shader at render scale `0.75`.
- `broker-h264-stereo-live-openxr-projection-scale065-probe`: the same broker
  direct raw-projection path at render scale `0.65`.

Use the fast `0.75` profiles as the public renderer-parity lane. They hold
stereo geometry, GPU import, decode mode, and camera IDs constant while
removing the heavier soft-border shader work from the measurement. Minor
motion artifacts during head movement are tracked as stream/reprojection
compensation work, not as a stereo orientation failure.

The Makepad comparison lane is not ready for performance comparison yet. It has
reached paired per-eye camera sampling with a no-swap limited-BT.601 CPU-YUV
control, but the current blocker is app-panel placement/isolation. Recent
head-locked placement attempts produced live passthrough screenshots without a
visible app-owned panel. The maintained Makepad fork can now disable native
OpenXR passthrough, but a clean install must grant both Scene Access and Headset
Camera before the Makepad permission flow enters XR presenting. With both
permissions granted, the passthrough-off gate reaches active XR and emits the
native-passthrough-disabled marker, but S66 still showed no non-black clear
color, solid app panel, or black border. The active Makepad blocker is now
projection-layer output isolation, not Camera2 acquisition or shader color
conversion. Loading/preflight states are failed launches and must not be
counted as passthrough-off success. This does not invalidate the earlier
Makepad panel proof: the panel and live camera feed already rendered visibly in
the headset, and the remaining parity issue was that the surface was anchored
in world space instead of camera/head space. The S67a passthrough-on rerun
confirmed active XR and clean end-frame submission but still did not show the
panel. S62 artifact review narrowed the recovery target to the exact visible
source traits: plain world-space vertex path, the higher S62 panel pose
(`pos.y=0.92`), dark clear color, direct per-eye YUV sampling, and the thin
pale border. S67a2 restored those traits on the maintained fork and passed both
launcher-path and direct generated-XR visual gates: the app-owned live camera
panel and thin pale border are visible again, both screenshot sequences were
byte-distinct, end-frame markers were present, stale head-locked markers were
absent, and GPU-fault/fatal/small-hardware-buffer counters stayed at zero. The
next Makepad step was S67b: change only the native passthrough/background
isolation setting and iterate until a solid app-owned background plus the same
visible panel is present. S67b passed that ordered milestone in both launcher
and direct generated-XR runs: native passthrough was disabled, OpenXR submitted
an opaque projection layer, the app-owned live camera panel and pale border were
visible against a solid black background, and both screenshot sequences were
byte-distinct. Two follow-up watches carry forward: the requested non-black
clear color did not appear visually despite marker presence, and the small 4x4
hardware-buffer warning class plus environment-depth runtime logs remained
visible without GPU faults, fatal signals, or visible depth occlusion. S68 is
now the active Makepad gate: it keeps the S67b passthrough-off live camera
sampling and border, but replaces the S62 world-space vertex transform with
active-eye `draw_pass.camera_inv` placement. Passing S68 requires the panel to
remain visible and fresh with native passthrough disabled; classifying it as
non-world-space still requires headset-motion inspection and is not yet final
metadata-backed projection parity. The first S68 static gate passed the
visibility/freshness side in both launcher and direct generated-XR paths: the
app-owned live camera panel and pale border remained visible with native
passthrough disabled, six-frame screenshot sequences were byte-distinct,
end-frame markers stayed successful, and app/global GPU-fault and fatal
counters stayed at zero. The small 4x4 hardware-buffer warning class and
environment-depth runtime logs remain tracked; no visible depth occlusion was
seen in the captured frames. A passive eight-frame motion-review capture found
the pale border at a stable screen-space bounding box, but frame deltas were
small, so that sequence is not conclusive unless the headset was deliberately
moved during capture. Follow-up operator headset inspection completed that
classification: the panels are now cleanly projected per eye and no longer
behave like the earlier world-space surface. The Makepad lane is still not at
stereo projection parity because the camera feeds are swapped left/right
between eyes and the panels need alignment tuning for a good stereo effect.
The next ordered gates are therefore source-eye mapping correction, horizontal
mirror correction, then stereo alignment using the existing custom Rusty XR
target alignment notes. S69 kept the S68 per-eye `camera_inv` placement, S59
no-swap limited-BT.601 color path, and vertical-only UV orientation, but
changed the Makepad display source-eye selector to `inverted_xr_view_id` /
`display-left-from-right-source`. Operator review reported coherent eye
alignment but remaining horizontal mirroring, so S69b keeps the source-eye
mapping and adds a horizontal UV flip. The Makepad lane should not advance to
performance comparison until source-eye mapping, mirror orientation, and
alignment have all passed.

S69b built a fresh Makepad APK from the documented standalone lane and
validated it with the guarded device-gate harness. The harness reached active
XR on the first launcher attempt, recorded six distinct screenshots, preserved
zero GPU-fault/fatal counters, and found no stale S69/S68 path labels in the
extracted native library. Follow-up operator review clarified that the
horizontal flip should be retained; the remaining issues are overlap alignment
and horizontal stretch. S70 therefore keeps the S69/S69b source-eye and texture
orientation path, then changes only geometry toward the Rusty XR target:
head-centered plane, about `0.75m` depth, `60deg` preview FOV, `1.06` raw
overscan, square `1280x1280` source aspect, and an approximate `0.92m x 0.92m`
content surface. Operator review accepted the S70 aspect-ratio correction and
reported that real-world geometry is no longer horizontally stretched, but the
remaining stereo mismatch is depth dependent: strong at close range and nearly
imperceptible around `1m`. S71 therefore keeps the S70 square surface and S69b
mirror/source-eye mapping, but moves the panel back to active-eye placement so
the Makepad quad does not add a shared physical convergence plane on top of the
camera-pair disparity. The S71 automated launcher gate passed with fresh
camera screenshots, S71 marker presence, zero stale S70/S69/S68 path labels,
and zero app/global GPU-fault or fatal counters, but operator headset
inspection reported that close-range stereo alignment was slightly worse than
S70. That negative split makes further panel-placement changes a lower-value
path. S72 returns to the S70 head-centered square visual basis and ports the
important Rusty XR custom-path difference directly: source camera
intrinsics/pose must affect the sampled UVs through the shared
`rusty-xr-camera-model` projection primitive, not through another panel
placement tweak. The Makepad pass now computes per-source
`surface_to_camera_uv_homography` rows from Camera2 intrinsics, lens pose, the
stereo reference center, and the same head-anchored preview surface constants.
The shader selects the matching homography with the same source-eye selector
used for the YUV texture pair, then applies the accepted texture flip. S72 and
S73 were both useful negative splits rather than visual passes: device gates
reached active XR, emitted homography-ready markers, captured byte-distinct
frames, and kept app/global GPU-fault and fatal counters at zero, but the
visible panel collapsed to a monocolored camera-reactive surface instead of a
spatial camera image. CPU-side YUV probes still showed live nonzero left/right
camera content, so the current blocker is not acquisition. S74 hard-coded the
logged homography rows in the shader and restored the camera feed, with the
known parallax issue still visible. That makes the dynamic Makepad field
handoff the current blocker for the metadata-backed path. S75 then tried the
same dynamic rows with pre-draw `set_dyn_instance` / `set_uniform` writes plus
existing area patching. The app stayed active, fresh, and fault-clean, but the
visible panel again lost the spatial camera image. That narrows the binding
fault to the draw-instance root: the panel still draws through nested
`DrawCube` draw-vars, so the shader instance slice is not guaranteed to include
the wrapper's custom homography fields. The refreshed public direct fast target
still reports projected stereo, `cpuUploadCount=0`, and app VrApi 72 FPS /
no-tear / no-stale lines under the comparison device profile; global stale
lines in that brief run came from another runtime process and are not
app-process target evidence. S76 moved the Makepad camera panel to direct
draw-vars ownership and restored live camera pixels through dynamic metadata
rows. That closed the binding blocker. S77 then ported the target shader's
invalid-projected-UV fallback so out-of-range projected samples fall back to
oriented content UVs instead of clamping to a dark edge. S78 removed the
remaining physical-panel convergence variable by drawing the same YUV and
homography path as a fullscreen clip-space surface, and S79 matched the public
target's `display-left-from-left-source` mapping. S80 was a useful negative
split: applying the target raw-content scale directly before the Makepad
surface-to-camera homography shifted/zoomed the surface instead of fixing
alignment. A fresh public fast `0.75` target recheck still reports projected
stereo, paired buffers, `cpuUploadCount=0`, app-process 72Hz no-tear/no-stale
VrApi lines, and byte-distinct screenshots. The active S81 Makepad split now
tests the closer target order: reconstruct display-screen UV to a head-anchored
preview-surface UV, then apply the Camera2 homography. If S81 remains visually
off, the next public-safe requirement is exposing exact OpenXR per-eye view/FOV
state to the Makepad app path rather than tuning another simplified panel
constant. S81 confirmed that this direction is active and performant, but the
shader-side ray reconstruction still left a striped invalid/fallback region.
S82 therefore moves the composition closer to the public target: the Camera2
plan now carries CPU-collapsed screen-to-camera homography rows plus separate
screen-to-surface rows for fallback fill, so the shader samples projected
camera UV directly from display-screen UV. S82 validated as an active,
fault-clean, byte-distinct device run with the new rows and stale path markers
absent, but it did not close the visual gap: live camera content remained, while
invalid/fallback striping persisted. That makes the next parity requirement
more concrete: the Makepad path needs to expose the exact per-eye OpenXR
view/FOV state used for the display surface, so the public target's
`surface_to_screen` -> `screen_to_camera` chain can be reproduced instead of
approximated from static display constants.

S83 tests the first exact-display-input step without changing the Makepad fork.
The fork already copies the active OpenXR per-eye projection and inverse-view
matrices into Makepad draw-pass uniforms before rendering. The Makepad shader
now reconstructs the display ray from `inverse(draw_pass.camera_projection)`
and `draw_pass.camera_inv`, intersects it with the same head-centered preview
surface, and applies the proven Camera2 `surface_to_camera` rows. This keeps
the S82 CPU-collapsed rows visible in logs as comparison evidence, but the
active sample path no longer depends on synthetic display-FOV constants. S83
was a negative visual split: it reached active XR, stayed fault-clean, emitted
fresh S83 markers, captured byte-distinct screenshots, and still had nonzero
CPU YUV probes, but the projected camera surface was black/blank. That points
to a Makepad shader projection-matrix convention or matrix-inverse issue, not
to Camera2 acquisition. A same-session refreshed public fast `0.75` target run
under the comparison device profile still reports projected stereo, paired GPU
buffers, `cpuUploadCount=0`, target source-eye mapping, app-process `VrApi`
72/72 to 73/72 with `Tear=0` and `Stale=0`, and six actually distinct frame
hashes. Its generic ROI validator can be black-like when the headset pose puts
the camera content low in the capture, so rely on the final projection markers
and per-frame hashes before declaring a target run stale.

The next Makepad split should preserve the recovered-feed lineage from
S82/S76/S77 while closing the exact-display-state gap. The lower-risk path is a
short shader diagnostic that uses near/far projection rays and an explicit
fallback to the S82 recovered feed when projected UVs go invalid. The more
structural path is a maintained Makepad fork exposure of the active OpenXR
per-eye view/FOV/head-center state so the public target's
`surface_to_screen` -> `screen_to_camera` CPU homography chain can be
reproduced directly instead of inferred in the fragment shader.

S84 implements that lower-risk split. The shader now keeps two projection
paths live: an exact-display candidate from near/far inverse-projection rays
through `draw_pass.camera_projection` / `camera_inv`, and an S82
CPU-collapsed `screen_to_camera` fallback. This should keep live camera pixels
visible even if the exact branch remains invalid, while making the next
headset sample decisive: improvement over S82 means the shader-side convention
can be fixed in the app; S82-like output means the maintained Makepad fork
should expose the exact per-eye OpenXR view/FOV state to app Rust for the same
CPU-side homography chain as the public target.

S84 did not recover the feed visually. The device run still reached active XR,
kept native passthrough disabled, emitted fresh S84 and homography-ready
markers, captured byte-distinct frames, and retained live Camera2/YUV probe
evidence with no app GPU-fault or fatal signatures. The headset screenshot was
black apart from the runtime HUD. Treat this as a shader sample-selection split:
the near/far exact branch can choose an in-range but visually black sample, and
the fallback branch did not get decisive ownership of the rendered pixel.

S85 is therefore a control build, not a parity claim. It should force the
S82-style `screen_to_camera` rows to own sampling while logging the S84 exact
branch as disabled/stale. Restored live feed means the fallback rows and YUV
textures are still correct and the remaining problem is the Makepad
draw-pass-projection convention. Continued black output means the S84 refactor
regressed fallback row binding or shader branch wiring and that must be fixed
before exposing exact per-eye state through the maintained Makepad fork.

S85 stayed black even with the exact branch disabled. The logs still prove live
Camera2/YUV content, bound left/right YUV textures, and the expected
screen-to-camera rows, so the next split should not tune projection constants.
S86 should bypass homography sampling and render direct fullscreen YUV UVs with
the same clip-space surface. Direct-feed recovery would put the fault in the
`screen_to_camera` UV domain or row composition. Continued black output would
mean the current fullscreen shader route itself has diverged from the earlier
visible camera controls and should be repaired before parity math resumes.

S86 recovered real camera detail with byte-distinct frames and no app fault
signatures, so the fullscreen Makepad shader route and CPU-YUV texture sampling
are healthy. The remaining projection issue is the CPU-computed
`screen_to_camera` homography produced from static Makepad display-eye
approximations. The public target's fast shader applies `screen_to_camera` to
display-surface UV and uses content UV mainly for fallback/border behavior, so
the next parity step should expose the active OpenXR `XrView` pose/FOV state
from the maintained Makepad path and compute the same CPU homography chain with
real runtime view data.

S87 implements that structural step. The maintained Makepad fork now carries
the active per-eye OpenXR local-space pose and FOV through `XrState`, and the
Makepad example consumes that fork revision instead of inferring display-eye
state from fixed constants or fragment-shader matrix inverses. The app stores
the selected Camera2 stereo sources, waits for runtime XR view state, then
recomputes the same public target-style `surface_to_screen` ->
`screen_to_camera` homography chain with runtime left/right eye pose and FOV.
The pre-device source gate passes standalone `cargo fmt` and `cargo check`
with only the known Makepad duplicate-dependency warning. The next device gate
must prove three things separately: the new `runtimeXrViewStateReady` marker
appears, stale S86/S85 projection paths are absent from the fresh APK and logs,
and the visual result moves from S86 direct-YUV proof back toward projected
camera parity without reintroducing app-process GPU-fault or fatal signatures.

S87 passed that structural gate. The fresh APK contained the S87 strings and no
stale S86/S85 strings, launcher startup reached active XR on the first attempt,
runtime XR view state and homography-ready markers appeared, six screenshots
were byte-distinct, and app/global GPU-fault plus fatal counters stayed at
zero. Visual review showed a live projected camera surface in the same
low-in-frame headset-pose class as the refreshed public fast `0.75` target.
The next visual delta is no longer activity handoff, texture binding, or
Makepad fork eye-state exposure; it is shader policy for invalid projected UVs.
The public direct shader falls back to an oriented content-surface sample and
dims it, while S87 returned black. S88 ports that fallback policy while keeping
the S87 runtime OpenXR view/FOV rows, source-eye mapping, and device-gate
counters unchanged.

S88 passed the target-shader-policy gate. The fresh Makepad APK contained S88
markers and no stale S87/S86/S85 path strings, reached active XR through the
guarded launcher path, emitted runtime-view and homography-ready markers, kept
six screenshot frames byte-distinct, and stayed app/global GPU-fault plus fatal
clean while the small hardware-buffer warning class remained visible. Visual
review kept live projected camera content and made invalid/edge regions follow
the public direct shader policy more closely. This is still not a performance
parity claim because the Makepad sample presented near 90Hz while the refreshed
public fast target sample presented near 72Hz. The next gate is S89: compare
the S88 runtime-view homography and sampling chain directly against the
validated target implementation and fix the remaining close-range
parallax/projection-geometry mismatch before any refresh-normalized performance
comparison.

S89 source prep removes one Makepad-only variable before tuning projection
math: the visible panel shader had been flattening Makepad `CubeGeom` into a
fullscreen surface, which can overdraw several cube faces into the same plane
with geometry-provided UVs. The S89 patch switches to a single fullscreen
`QuadGeom` and derives target-style screen UV directly from quad position while
preserving the S87/S88 runtime-view homography rows, source-eye mapping, and
invalid-UV fallback policy. Standalone formatting and compile checks pass; the
fresh release APK also contains S89 strings and no stale S88/S87/S86 path
strings. The device gate is pending because ADB transport was unavailable
during the first S89 attempt; that is a workflow/transport blocker, not a
projection result. Once the Quest is visible to ADB again, the gate must decide
whether this deterministic screen-UV domain improves, preserves, or worsens the
observed close-range parallax.

S90 closes a source-binding delta from the validated target diff before the
next headset run. The target path chooses the stereo pair by Camera2 pose X and
tracks display camera IDs, but the Makepad lane previously correlated Camera2
metadata to Makepad video streams by source index while Makepad Android named
all back cameras identically. The maintained fork now exposes `cameraId=` in
Android video descriptors. The Makepad example now carries Camera2 IDs through
the stereo projection plan, orders the selected Camera2 pair by physical pose
X, parses Makepad descriptor camera IDs, and chooses Makepad video streams by
camera ID before falling back to index. The guarded launcher device gate now
confirms `s90CameraIdSourceBinding=true` and `sourceBindingMode=camera-id`, so
the remaining parallax result is a projection-math question rather than a
possible metadata/texture mismatch. A fresh S90 APK build has passed the native
string gate: S90, `cameraId=`, and pose-X source-selection strings are present,
while stale S89/S88/S87/S86 path strings are absent. The launcher run reached
active XR on the first attempt, emitted runtime-view and homography-ready
markers, captured six byte-distinct screenshots, and kept app/global GPU-fault
plus fatal counters at zero while preserving the known small hardware-buffer
warning counter separately. The S90 source state is pushed, and a static re-diff
against the validated public fast `0.75` target found no further source-only
mismatch in source ordering, runtime view/FOV inputs, the shared homography
helper chain, projection scale, preview FOV, raw overscan, or `left-right`
source-eye mapping. Operator parallax inspection rejected S90 as visual parity:
the headset still showed depth-dependent stereo misalignment/parallax, an
apparent left/right source-eye flip, and a roll/orientation defect where a
horizontal real-world surface rotated toward vertical on screen. S91 therefore
keeps the S90 acquisition and camera-ID binding result but changes projection
math: display-eye homography rows stay display-indexed, source-eye texture
selection is inverted as a visual correction candidate, and the active camera
texture orientation returns from the 180-degree `flip-x-and-y` transform to a
vertical-only correction. The fresh S91 device gate reached active XR, emitted
the S91 display/source mapping markers with stale S90 path counters at zero,
captured six byte-distinct screenshots, and stayed fault-clean while preserving
the small hardware-buffer warning class separately. The S91 result must be
treated as best-effort until headset review, but it is the right state for
objective performance diagnostics because the frame transport path is already
live and fault-clean.
Focused host tests now cover descriptor `cameraId=` parsing and camera-ID pair
binding, including the case where source indices are misleading and the
descriptor IDs must win.

## Sanitized Parity Target

The downstream target proves the desired behavior class:

- custom stereo Camera2 projection visible in headset
- no app fatal or GPU-fault signatures in the validated window
- no final stale/tear behavior under operator inspection
- materially lower GPU load than the public `0.65` run
- device performance state controlled during the run
- camera permission handled before the measurement window

Treat refresh rate, render scale, buffer scale, multisampling, foveation,
permission state, and power/device levels as normalization axes before
declaring any implementation delta. The public objective is not to clone the
downstream shell. The objective is to make Rusty XR's reusable projection
contracts, diagnostics, and public example path expose enough of the same
low-cost shape.

The first S92 transport comparison now gives a normalized reference point,
even though Makepad projection math still needs headset acceptance. With both
stacks at CPU/GPU level `4` / `4` and `VrApi` scale factor `0.75`, the public
fast target held about `72.9/72Hz` with `Tear=0`, `Stale=0`, `App=1.73ms`,
`CPU&GPU=1.48ms`, `GPU%=0.20`, `CPU%=0.21`, low app-process CPU in `top`,
paired GPU buffers, and `cpuUploadCount=0`. The Makepad S91 lane held about
`90.5/90Hz` with `Tear=0`, `Stale=0`, `App=2.06ms`, `CPU&GPU=7.56ms`,
`GPU%=0.28`, `CPU%=0.47`, app/XR/draw cadence near `90Hz`, paired camera
texture updates near `50Hz`, and substantially higher app-process CPU. Both
runs produced six byte-distinct screenshots and stayed GPU-fault/fatal clean.
This suggests the Makepad loop has useful presentation headroom, while the
current Makepad camera path is CPU-expensive and should not become the default
public performance host until the camera import path moves closer to the
zero-copy public target.

S93 normalized the public target to explicit `90Hz` and separated display
render cadence from camera-delivery cadence. The public fast `0.75` target
requested and activated `90.000Hz`, held `OpenXR` about `90Hz`, consumed
camera pairs at `50.001Hz`, rendered projection frames at `90.007Hz`, and
averaged `1.800` renders per distinct camera frame. This confirms that the
public path can redraw at display rate with fresh head pose while reusing a
roughly 50 Hz camera pair cadence. The Makepad S91 lane likewise held about
`90Hz` app/`XrUpdate`/draw cadence with paired texture updates around `50Hz`,
so raw camera cadence alone does not explain the visible smoothness or
alignment differences.

The decisive S93 visual split is presentation-level: while Makepad S91 was
active, the Meta performance HUD itself appeared stereo-misaligned along with
the camera projection. Switching back to the public Rusty XR target under the
same `90Hz` / CPU-GPU level-4 device state made the HUD stable/aligned again.
That makes the next Makepad blocker a Makepad XR presentation/view/layer-state
issue, or a closely related runtime configuration issue, before it is merely a
camera shader homography issue. The `alignedProjection=true` marker is therefore
objective transport state, not visual acceptance.

The S94 screenshot follow-up narrows the evidence boundary. ADB `screencap` and
HzDB `screencap` both exposed the full raw stereo surface and allowed automated
green-HUD feature detection, but the measured Meta performance-HUD disparity was
effectively identical between the public target and Makepad S91. HzDB
`metacam` produced a single camera view in this setup rather than a stereo
pair. A direct Makepad generated-XR activity launch with the Oculus VR category
also reached S91 markers and kept the same raw HUD positions, so the normal
launcher hop is not sufficient to explain the headset-visible HUD misalignment.
Use these captures as raw-surface witnesses only; final HUD stereo acceptance
still needs headset review or a true binocular through-lens capture.

S95 repeated the direct generated-XR activity launch as a headset visual control.
The run reached the XR activity, emitted S91 runtime/projection markers, stayed
fault-clean, and still showed the Meta performance HUD misaligned to the
operator. The public target and the direct Makepad XR launch also shared the
same Horizon raw-window class and stereo surface shape, so the issue should not
be reduced to a launcher hop or generic volumetric-window routing difference.

S96 attempted that upstream Makepad baseline and found a launch-state blocker
before any HUD conclusion could be drawn. A clean upstream `dev` XR example
build using only a local Windows packaging shim entered the generated XR
activity, then toggled back to Makepad's normal Android activity and displayed a
2D screen. This matches the earlier Makepad recovery notes: direct generated-XR
launches require a directional activity handoff, not the symmetric
`switchActivity()` method, because `XrStartPresenting` called from an activity
that is already the XR activity must stay there and create or retry the OpenXR
session. The next baseline should therefore be upstream Makepad plus only the
minimal Quest launch guard and local Windows build shim.

S97 completed that guarded upstream baseline. With only the directional XR
handoff added to upstream Makepad, the old upstream scene-picker / XR example
UI stayed in the generated XR activity and operator review did not see the Meta
performance HUD stereo misalignment. The log still carried upstream GPU
page-fault warning lines, so this is a presentation/HUD baseline rather than a
GPU-fault-clean renderer baseline. S99 then repeated the scene-picker control
from the maintained fork itself. That maintained-fork scene picker also stayed
in the generated XR activity, submitted native passthrough plus a two-layer
OpenXR frame, and operator review reported no Meta performance-HUD stereo
misalignment. The HUD split is now narrower: upstream Makepad and the
maintained fork's original scene content are comfortable, while the Rusty XR
camera example is not. Next work should diff the Rusty XR example against the
scene-picker baseline, prioritizing explicit render-scale changes, the
fullscreen clip-space camera surface, runtime-view homography state, and the
example's panel/projection path. For raw Makepad panel lineage, use the
S62/S67/S68 visible-panel states as the closest reference rather than the later
S91 fullscreen projection experiment.

S98 made the first maintained-example split against that baseline by restoring
native passthrough in the camera example while leaving the S91 camera/projection
shader path otherwise intact. The direct generated-XR activity reached active
XR on the first attempt, emitted S98 markers, submitted a two-layer OpenXR frame
with `nativePassthrough=true`, `projectionBlendSourceAlpha=true`, and
`layerCount=2`, captured six byte-distinct frames, and stayed GPU-fault/fatal
clean while preserving the small hardware-buffer warning class. Raw screenshots
showed live camera content and the runtime HUD. Operator headset review then
reported that the Meta performance HUD was still stereo-misaligned in S98. That
rules out the simple passthrough-off / missing-source-alpha explanation: the
maintained camera example can submit the guarded-upstream-style two-layer frame
and still reproduce the binocular HUD defect. The next split is therefore the
original Makepad scene picker built from the maintained fork itself, to isolate
fork/manifest/OpenXR state from the Rusty XR camera example path. S99 completed
that split and was headset-aligned. Its end-frame markers also exposed a new
smallest suspect: the aligned scene picker used the fork's high default XR
target size (`2352x2464` for `1680x1760` recommended), while the misaligned
camera example used the explicit `0.75` target size (`1260x1320`). The next
camera-example split should therefore restore the scene-picker/default Makepad
XR scale before changing projection math again.

S100 performed that render-scale split. The camera example at the scene-picker
default scale reached active XR and used the high/default image rect, but
operator review reported that the HUD stayed aligned only during launch and
the green camera-arming placeholder. The misalignment appeared when live camera
content replaced the placeholder. The high/default scale also regressed the
performance target: stale frames returned, 90 FPS was not sustained, and CPU
load was visibly red. Treat render scale as a ruled-out primary cause for the
HUD trigger and return to `0.75` for further camera-path isolation. The next
split should leave Camera2/Makepad acquisition running while suppressing live
camera sampling in the shader after arming.

S101 completed that camera-feed-suppressed control at the performant `0.75`
scale. Camera acquisition/import stayed active and the low image rect returned,
but the shader rendered a controlled diagnostic surface instead of sampling
live YUV once the streams armed. Operator review reported that HUD alignment
was good. This rules out acquisition/import and low render scale as the direct
HUD trigger. The active suspect is now live camera pixel/projection rendering.
The operator also observed that the diagnostic surface appeared to cover a
larger area than the normal camera projection, so the next split should keep
live camera sampling enabled while forcing a full-surface valid/coverage path.

S102 made that split decisive. Live YUV sampling at `0.75` stayed HUD-aligned
when the shader forced full-surface identity coverage and disabled the
projection-valid dim/mask branch. The camera feed was intentionally full-screen
and therefore not yet geometrically correct, but the architecture direction is
now clear: keep the submitted OpenXR surface full and perform camera coverage,
matte, crop, and border inside the shader. Do not resize or shrink the layer or
the app-owned surface to make the projection window.

S103 implements the first version of that architecture. The Makepad example now
keeps the full submitted surface active while rendering live camera pixels only
inside a shader-owned content window, with dark matte outside and a black border
around the camera-covered region. The window size is derived from the public
target's full-view/content overscan ratio rather than by resizing the layer.
The fresh APK reached active XR and emitted the S103/full-layer/in-shader
coverage markers with zero GPU-fault or fatal counters in the ready sample. A
stable-link rerun then produced six byte-distinct freshness hashes and operator
headset review accepted S103 as the new render-stack baseline: the Meta
performance HUD stayed aligned, the earlier distance-dependent parallax issue
was gone, and rotation/aspect were correct. The remaining Makepad visual
defect is now only horizontal alignment between the two eye images. S104 should
preserve S103's full submitted surface, in-shader camera window, matte, border,
rotation, and aspect behavior, and tune only the horizontal sample offset
against the public target's stereo projection rows.

S104 was the first horizontal-only pass on top of that accepted baseline. It
kept the S103 layer/window architecture and evaluated the display-mapped
`surface_to_camera` homography at the camera-window center, then applied only
the X center delta to the live camera sample. Objective device validation
passed with the S104 marker, six distinct frame hashes, low `0.75` image-rect
markers, and zero GPU-fault/fatal counters, but operator review still saw a
large horizontal offset. The useful diff pointer is that the public target's
projected shader path samples via `screen_to_camera(v_surface_uv)` for the full
submitted surface.

S105 applied that pointer and added a tuning workflow. The automatic X
correction used the `screen_to_camera` center delta, and additive manual
left/right UV offsets were read from `debug.rustyxr` Android properties while
the Makepad app was running. The first screenshot-driven sweep compared the
Makepad `Strength=0` baseline and a running strength sweep against the public
fast `0.75` target. `Strength=0` removed the visible edge-striping class but
left the eye images too far apart, while larger strengths moved the eye images
together and eventually reintroduced side artifacts. Feature matching on the
left/right camera-content crops put `Strength=0.425` closest to the public
target's normalized horizontal disparity, but operator headset review rejected
that result as still not close to the Rusty XR target.

S106 keeps the S103 full submitted surface and shader-owned camera window, but
changes the shifted-sample policy before further tuning. The default strength
returns to `0.0`, and shifted `aligned_window_uv` samples are validity-tested
separately so out-of-range corrections show matte instead of repeating clamped
camera-edge pixels as linear side stripes. New alignment tooling under
`tools/quest-stereo-alignment/` scores black-target disparity and edge-stripe
regression separately, so a future scalar or manual offset cannot be accepted
only because it matches one raw screenshot disparity number.

S106 device evidence falsified strength-only tuning: safe invalid matte removed
the clamped edge-repeat failure mode, but camera framing still did not match
the Rusty XR target. S107 therefore added a hotloadable camera-window content
scale and showed that the previous framing was too small, while also exposing
that the diagnostic guide and inner window border were contaminating visual
review. S108 removes those border/guide artifacts, uses black invalid matte,
and resets the review baseline to `contentUvScale=1.60`, `Strength=0`, and
zero manual offsets. Symmetric per-eye offsets remain available for evidence,
but the first sweep showed that larger offsets trade horizontal convergence for
a central black invalid wedge. Do not bake those offsets until headset review
accepts the tradeoff.

The current dedicated alignment workflow is
`docs/QUEST_STEREO_ALIGNMENT_WORKFLOW.md`. Iterations should keep one ignored
artifact packet with the Rusty XR target, Makepad candidate, optional
MediaProjection/final-display witness, analyzer reports, and operator
classification. MediaProjection can witness the final display when consent is
already active, but it is still not direct access to Meta's protected
passthrough compositor buffer.

## Cross-Stack Alignment Update

The next parity work should treat Rusty XR direct projection, Rusty XR
broker/existing-stream projection, Makepad projection, and Meta native
passthrough as separate evidence lanes with a shared metadata vocabulary.

Rusty XR direct projection remains the authoritative public target for:

- Camera2 source capability logging
- source-eye mapping
- texture transform
- intrinsics/extrinsics use
- screen-to-camera projection rows
- temporal target/applied/residual metrics
- projected shader path and render-scale scorecards

The broker/existing-stream path should be considered equivalent only when it
receives session-native projection metadata instead of relying on manual launch
extras. Its scorecard must prove that decoded hardware buffers, source-eye
mapping, texture transforms, and projection metadata reached the same projected
shader path as direct Camera2.

The Makepad lane should consume the same public contracts rather than growing a
separate calibration model. Its useful comparison state is:

- full submitted surface kept stable for XR presentation comfort
- camera coverage, matte, crop, and border owned inside the shader
- same source-eye mapping vocabulary as the Rusty XR target
- same projection metadata fields and timestamp domains
- same temporal policy values and scorecard fields once smoothing is enabled

Meta native passthrough is a reference for user-visible comfort, room context,
and visual alignment, not a raw camera texture. Use it as a separate witness:

- compare native passthrough against custom projection with fixed pose and a
  known physical-screen stimulus when available
- record whether the run used native passthrough underlay/overlay, app-owned
  raw camera projection, broker-decoded projection, or MediaProjection witness
- do not infer raw camera freshness from native passthrough visibility
- do not make native passthrough a dependency for custom projection smoothness

Temporal smoothing should be introduced only after the no-smoothing target and
candidate are both measured. The first acceptance split is not whether the
camera looks "more native"; it is whether `applied_projection_motion_px_p95`
is bounded while target motion, residual lag, stereo lockstep state, and
invalid-UV/edge-fill costs remain visible in the scorecard.

## Absorbable Public Work

Rusty XR can absorb these public-safe lessons:

- Keep the direct Camera2 performance profile separate from broker, depth,
  MediaProjection, and visual-effect profiles.
- Expand the scorecard so parity comparisons always include source/build
  provenance, launch method, runtime settings, projection path, `VrApi` app and
  CPU+GPU time, GPU percentage, tear/stale counts, GPU import counters,
  camera-frame progression, app fatal signatures, and GPU fault signatures.
- Normalize device-level and permission setup in run manifests so one run is
  not compared against another with hidden runtime-state differences.
- Capture a short multi-frame screenshot freshness sequence for visual gates
  and record whether any frames are byte-identical before treating a screenshot
  as live camera evidence.
- Attribute projected draw cost into shader work, border/perimeter work,
  descriptor/import reuse, command recording, frame-buffer/render-pass churn,
  and submit behavior.
- Keep sampler and external-format handling explicit. Public logs should show
  whether camera imports use the expected external format path, sampler binding
  mode, import-cache size, descriptor-cache size, failures, misses, and
  evictions.
- Keep projection math authoritative in the public target until Makepad headset
  review passes source-eye mapping, close-range parallax, and roll/orientation.
  Makepad should absorb the same contracts rather than grow an independent
  calibration path.
- Treat Makepad as a strong candidate for UI-heavy or app-shell experiments,
  but move camera projection and future temporal smoothing through
  framework-neutral contracts so the same transport/effect assumptions can be
  compared across both stacks.
- Preserve public source taxonomy. Platform passthrough, raw Camera2, OpenXR
  environment depth, MediaProjection, and operator casting are separate witness
  streams, not interchangeable camera sources.
- Move only generic math, metadata, runtime-profile, scorecard, and optional
  adapter contracts into public crates. Keep downstream effect behavior and
  exact tuning downstream.

## Broker-Guided Manual Alignment Loop

Use the broker experiment page plus the ADB-launched shell helper for headset
side tuning when screenshot-derived matching and headset visual judgment
disagree.

Recommended loop:

1. Start the broker shell helper with `--focus-guardian` so it can apply
   whitelisted `debug.rustyquest.makepad.*` tuning properties and report foreground state.
2. Keep the broker console foreground while editing one tuning variable at a
   time.
3. Use `launch_target_guard` for unstable target builds. The helper launches
   the target, gives the operator a bounded visual window, then returns to the
   broker and disables the experiment mode if the guard expires.
4. Record one broker revision per headset judgment: tuning values, guard mode,
   foreground package/activity, recovery count, screenshot freshness hashes,
   and operator verdict.
5. Treat ADB/HzDB screenshots as submitted-surface witnesses. Use
   MediaProjection only as a consented final-display witness, and do not label
   either path as direct access to native runtime passthrough compositor pixels.
6. Compare against the native-runtime passthrough reference as a separate
   witness stream, not as an interchangeable camera source.

This loop is meant to keep manual target inspection recoverable while
projection math is still moving. It is not a kiosk guarantee and it does not
preempt system Home, Guardian, permission, or safety UI.

## Depth And Alignment Impact

Depth work changes how stereo alignment should be validated, but it should not
be folded into the main performance profile by default.

Use depth as an explicit witness or calibration profile:

- raw stereo camera frames remain the source for the custom camera projection
  path
- environment depth can provide a depth-prior proxy for reprojection checks
- MediaProjection can witness final display appearance, but it is not a raw
  camera source
- depth-assisted alignment should record camera timestamps, depth timestamps,
  predicted display time, projection profile, source-eye mapping, and capture
  timing in one scorecard bundle

The activation ladder for depth-informed alignment should stay diagnostic:

1. native runtime passthrough plus environment-depth mesh or particles
2. native runtime passthrough plus custom camera edge overlay
3. native runtime passthrough plus checker or wipe overlay
4. central custom projection window over native passthrough
5. mostly custom projection with a native passthrough border/reference region
6. full custom projection only after scorecard and operator inspection pass

The main parity profile remains depth-off until a depth-assisted correction has
been shown to reduce reprojection error without adding frame-budget risk.

## Projection-Area Diagnostic Gate

Before tuning camera-content sampling, compare both stacks with synthetic
projection-area diagnostics. The Rusty XR reference diagnostic must reuse the
same projection geometry as the accepted camera profile while replacing camera
pixels with a high-contrast target and opaque border. The Makepad diagnostic
must run in a dedicated alignment APK and feed the same synthetic target through
its screen-to-camera footprint.

Acceptance for this gate is geometric, not photographic:

- the diagnostic target is visible in the headset without native passthrough or
  broker UI ambiguity
- the visible border footprint aligns with the Rusty XR reference per eye
- area offsets and screen-domain scale values are recorded as run evidence, not
  baked into public defaults
- camera-content defects inside the accepted area, such as edge gaps or UV crop
  errors, stay deferred until the footprint is accepted

## Implementation Order

1. Freeze the `camera-stereo-gpu-composite-performance-065` evidence as the
   public correctness baseline and keep future public reports tied to explicit
   run manifests.
2. Add scorecard fields for GPU percentage, tear/stale summaries, device-level
   state, camera permission state, import-cache behavior, descriptor-cache
   behavior, projection status, and fatal/GPU-fault signatures.
3. Split projected draw attribution into shader, border/perimeter, descriptor,
   import, command recording, render-pass/frame-buffer, and submit categories.
4. Add a depth-alignment witness profile that records depth/camera/display
   timing without changing the default direct Camera2 performance profile.
5. Add camera/source capability and timestamp-domain manifests before
   accepting fixed-source or timestamp-nearest conclusions across devices.
6. Promote session-native projection metadata so broker/existing-stream and
   Makepad candidates can use the same projection evidence as direct Camera2.
7. Add temporal no-smoothing, pose-clamp, screen-motion-clamp, frame-adoption,
   and edge-mode scorecard gates before Q2Q online transport hides renderer
   smoothness issues.
8. Promote only stable, framework-neutral findings into public contracts:
   projection metadata, timestamp pairing, scorecard schema, calibration
   descriptors, and optional adapter hooks.
9. Re-run the direct Camera2 matrix at `0.75` and `0.65` after each render-path
   slice, and keep the release gate on visible stereo, zero CPU uploads, no
   import churn, no app fatal/GPU-fault signatures, no final stale/tear, and
   clear GPU headroom.
10. Keep the direct raw-projection profiles in the catalog as the stable parity
   reference while future work reintroduces public border/feedback styling or
   adds stream-latency compensation.

## Public Acceptance Gate

Do not call the public profile parity-complete until a new Quest run shows:

- visible paired stereo camera projection in headset
- `activeTier=gpu-projected`
- `alignedProjection=true`
- `stereoLayout=Separate`
- paired left/right GPU buffers
- `cpuUploadCount=0`
- no sustained GPU import failures or cache evictions
- no app fatal or GPU-fault signatures
- no final-window stale/tear behavior
- no byte-identical multi-frame screenshot sequence when the profile is expected
  to show live camera content
- projected frame cadence at the target refresh rate
- materially lower GPU load than the current accepted `0.65` baseline
- a matching scorecard that records all normalization axes

Any report that uses downstream target evidence must remain sanitized before it
is copied into this public repository.
