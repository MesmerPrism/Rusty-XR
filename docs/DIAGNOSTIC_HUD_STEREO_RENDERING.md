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
