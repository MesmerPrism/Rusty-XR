# Diagnostic HUD Stereo Rendering

Rusty XR diagnostic panels should pick a stereo rendering path intentionally.
The wrong path can make text look doubled or slightly detached even when the
content is otherwise sharp.

## Recommended Headset Path

Use a shared stereo surface for in-headset HUDs.

The Quest composite example now anchors one logical HUD surface in front of the
head and projects each HUD rectangle or glyph through the current left and right
OpenXR display views. This follows the same camera ownership model as
framework-native XR panels: the panel has one world/head-space surface, then the
stereo camera system decides how each eye sees it.

The implementation derives that surface from the same head-anchored
camera-preview envelope used by the custom projection path, applies the same
normalized inset, and then projects each primitive into multiview clip space.
That keeps the visible coverage aligned with the previous direct per-eye
projection while avoiding two independent per-eye text layouts.

## Rendering Options

### Monoscopic Screen-Space Overlay

Render one 2D rectangle with identical clip coordinates for both eyes.

This is useful for desktop mirrors, screenshots, or non-XR debugging. It is not
a good headset HUD default because the panel has no stable stereo depth.

### Direct Per-Eye Screen-Space Overlay

Project a desired envelope into each eye, collapse the projected corners into
left and right screen rectangles, and render text independently inside those
rectangles.

This can match a target screen-space coverage exactly and is cheap to implement,
but it does not preserve a single 3D panel surface. If the left and right
rectangles diverge, text can feel like it has odd stereo separation because the
glyphs are effectively placed in two separate screen-space layouts.

### Shared Stereo Surface

Create one head-anchored or world-anchored panel surface, place the normalized
HUD layout on that surface, and project the same surface through each display
eye.

This is the preferred general-purpose diagnostic HUD mode for headset builds.
It keeps the panel depth and convergence coherent. In the public Quest example,
the surface is head anchored so it stays in front of the user, and its size is
derived from the same projection envelope as the camera-preview overlay so the
coverage remains comparable to the direct per-eye path.

### Framework-Native XR View

Use a UI framework's XR view or panel primitive when the app already depends on
that framework.

This usually gives the best text formatting, DPI handling, and input behavior:
the framework lays out a 2D view, attaches it to a world transform, and lets the
XR camera stack render it per eye. Keep this as an adapter decision rather than
a core Rusty XR dependency unless the downstream app has already chosen that UI
stack.

## Public Example Contract

The reusable debug-canvas crate owns normalized documents, sections, badges,
rows, text runs, tones, and HUD command state. It does not own stereo rendering.

The Quest example renderer owns:

- the head-anchored HUD surface placement
- conversion from normalized draw-list rectangles into surface points
- left/right OpenXR projection into multiview clip coordinates
- Vulkan pipeline, blending, scissor, and glyph drawing

Other app shells can keep the same document/draw-list contract while swapping
in a framework-native XR panel, a world-space mesh, or a temporary screen-space
debug overlay.

## Screenshot Alignment Checks

Quest raw-surface screenshots are useful but have a narrow interpretation.

ADB `screencap` and HzDB `screencap` can return the full stereo surface, so a
tooling pass can split the image into left and right halves and locate HUD
features in each eye. This is good for checking whether a HUD is present,
whether the raw left/right surface positions changed, and whether one launch
path produced a stale or unrelated frame.

These captures do not necessarily reproduce headset-visible stereo comfort. In
the Makepad comparison work, the public target and the Makepad shell produced
nearly identical raw Meta performance-HUD positions in ADB/HzDB stereo
captures, even though headset inspection reported a Makepad-only HUD alignment
problem. Treat that as a limitation of the capture path: raw stereo screenshots
measure the submitted/composited surface, not every perceptual or optical state
that the headset user experiences.

HzDB `metacam` is a separate witness. In the current Quest setup it produced a
single `1024x1024` camera view rather than a left/right stereo pair, so it can
confirm that content is visible but should not be used as a disparity metric.

A direct generated-XR activity launch with the Oculus VR category is a useful
Makepad control because it removes the normal launcher handoff. It did not
resolve the headset-visible HUD misalignment in the comparison shell, and the
public target and direct Makepad launch both reported the same raw Horizon
window class and stereo surface shape. Keep that launch mode in the matrix, but
do not treat it as sufficient evidence that the presentation path now matches
the public target.

An upstream Makepad XR example is only a valid HUD baseline after proving that
the generated XR activity stays foreground. In the S96 control, the upstream
example entered the generated XR activity and then toggled back to the normal
Android activity, producing a 2D screen rather than the original XR scene. That
matches the known symmetric-activity-toggle failure. Apply only the minimal
directional XR handoff guard before using upstream Makepad scene-selection or
hand-panel UI as a third HUD alignment baseline.

The S97 guarded upstream control passed that requirement. With only the
directional handoff guard applied, the upstream scene-selection style UI stayed
in the generated XR activity and operator review reported no Meta performance
HUD stereo misalignment. Use this as the Makepad-side HUD baseline when
diffing the maintained fork and the public example. Keep its scope narrow:
upstream still logged GPU page-fault warnings, so this is evidence about
presentation/HUD alignment, not proof that the upstream renderer is
GPU-fault-clean on the device.

The S98 maintained-example control restores native passthrough while keeping
the same camera/projection shader path. Its objective gate proves the maintained
camera example can submit the guarded-upstream-style two-layer frame again:
`nativePassthrough=true`, `projectionBlendSourceAlpha=true`, and `layerCount=2`
with live camera content, distinct screenshots, and no fatal or GPU-fault
counters. Treat its raw screenshots as a content/freshness witness only. Live
headset review reported that S98 still misaligned the Meta performance HUD, so
the defect is not explained by the camera example's passthrough-off one-layer
submission alone. Test the original Makepad scene picker on the maintained fork
before changing camera projection math again.

S99 tested that original scene-picker path on the maintained fork. Operator
review reported that the Makepad canvas was present and the Meta performance
HUD was not stereo-misaligned. This rules out the maintained fork, manifest,
direct VR-category launch, native passthrough, and two-layer OpenXR submission
as broad causes. The next suspect is camera-example-specific presentation
state: S99 used the fork's high default XR render target, while S98 used the
camera example's explicit `0.75` Makepad XR render scale.

S100 tested that render-scale suspect directly. The high/default render-scale
camera example kept the HUD aligned through launch and the green camera-arming
placeholder, but the HUD misalignment appeared when live camera content began.
It also regressed CPU load, stale frames, and 90 FPS stability. For HUD work,
the important split is now camera content versus acquisition/import; for
performance work, further camera-example tests should return to `0.75`.

S101 suppressed live camera sampling after arming while leaving acquisition and
texture updates active at `0.75`. Operator review reported that HUD alignment
was good, so acquisition/import alone is not the trigger. The remaining suspect
is the live camera projection surface itself: coverage, valid-region masking,
per-eye sampling bounds, or the way the live camera pixels occupy the app-owned
surface. The next HUD test should keep live sampling active but force a
full-surface coverage path.

S102 confirmed that live camera sampling can stay HUD-aligned when the shader
forces full-surface identity coverage and bypasses the bounded valid-region
mask. S103 then reintroduced camera coverage as an in-shader content window
with matte and border while keeping the submitted layer full. The S103 launch
reached active XR and produced live camera-window screenshots with S103 markers.
A stable-link rerun and headset review accepted this architecture: the Meta
performance HUD stayed aligned, and the prior distance-dependent camera
alignment defect did not return. Keep that full-layer/in-shader-window shape as
the HUD-safe baseline while the remaining camera work focuses on horizontal
eye alignment.

The S105 horizontal tuning pass reused raw stereo screenshots for a narrower
camera-content metric: feature matching on left/right camera crops compared
Makepad strength candidates against the public fast `0.75` target and selected
`Strength=0.425` as the current image-derived default. Treat that metric as an
alignment aid only; headset inspection remains the final acceptance gate.

When investigating HUD alignment regressions:

- use ADB or HzDB full-surface screenshots to detect raw-position changes
- keep the launch command and activity metadata next to the screenshot result
- use headset inspection or a true binocular through-lens capture for final
  stereo-alignment acceptance
- avoid treating a near-zero raw-position delta as proof that the headset view
  is comfortable

## Text Rendering

The public Quest example uses a generated real-font atlas for headset-visible
diagnostics. Its native build script rasterizes the bundled
`JetBrainsMono-Regular.ttf` asset into a small printable-ASCII SDF atlas, then
the Vulkan fragment shader samples that atlas with derivative-based smoothing
scaled to the glyph's on-screen size. This
keeps the APK example independent of a UI framework while avoiding the blocky
look of procedural 5x7 debug text and reducing eye-dependent subpixel breakup.

The reference UI framework path uses a DPI-scaled 2D view with a full font
system. The public example approximates the useful parts by keeping the HUD
grid less dense, using a higher-resolution SDF atlas, clamping samples inside
each glyph cell, and deriving the coverage ramp from atlas texels per screen
pixel. It also uses most of the camera-projection envelope for diagnostics so
each fixed-width text cell receives enough display pixels to stay legible.

This atlas path is intentionally narrower than a full UI text engine:

- it supports printable ASCII diagnostics, not shaping or rich text
- the debug-canvas layout remains a fixed-column diagnostic panel
- the renderer can embolden text roles slightly, but it does not implement
  font fallback, ligatures, cursor editing, or paragraph typography

Framework-native adapters can still use their own higher-level text stacks.
The reusable boundary is the normalized canvas document and draw list; font
selection, glyph caching, DPI policy, atlas format, and shader strategy belong
to the app shell or renderer.
