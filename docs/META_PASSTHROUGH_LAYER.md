# Meta Passthrough Layer Contracts

Rusty XR models native Meta/OpenXR passthrough as a compositor-owned layer, not
as an app-owned camera texture. A platform adapter creates and starts the native
passthrough feature, creates the native layer, applies styles, and submits an
`XrCompositionLayerPassthroughFB` proxy in the frame layer list. The contract
crate records the portable intent so downstream shells can translate it without
publishing app-specific renderer behavior.

Official references:

- [Meta native Android mobile passthrough](https://developers.meta.com/horizon/documentation/native/android/mobile-passthrough/)
- [Meta passthrough customization](https://developers.meta.com/horizon/documentation/native/android/mobile-passthrough-customization/)
- [Khronos `XrCompositionLayerPassthroughFB`](https://registry.khronos.org/OpenXR/specs/1.1/man/html/XrCompositionLayerPassthroughFB.html)
- [OpenXR `XR_FB_passthrough`](https://registry.khronos.org/OpenXR/specs/1.1/html/xrspec.html#XR_FB_passthrough)
- [OpenXR `XR_META_passthrough_color_lut`](https://registry.khronos.org/OpenXR/specs/1.1/html/xrspec.html#XR_META_passthrough_color_lut)
- [OpenXR `XR_FB_display_refresh_rate`](https://registry.khronos.org/OpenXR/specs/1.0/man/html/XR_FB_display_refresh_rate.html)

## Public Module

The public module is `rusty_xr_contracts::passthrough`. It exports:

- `PlatformPassthroughLayer`: layer purpose, placement, style, start behavior,
  and source-alpha blending intent.
- `PassthroughLayerPurpose`: OpenXR layer-purpose vocabulary.
- `PassthroughStyle`: opacity, edge color, and one color reproduction mode.
- `PassthroughColorReproduction`: native color, mono-to-mono, mono-to-RGBA,
  brightness/contrast/saturation, color LUT, or interpolated color LUT.
- `PassthroughExtensionRequirements`: extension names a platform adapter must
  enable for a descriptor.

This is data only. It does not depend on Android, Meta SDKs, OpenXR loader
bindings, Vulkan, Unity, Makepad, or an application shell.

## Layer Purposes

| Purpose | OpenXR name | Meaning |
| --- | --- | --- |
| `Reconstruction` | `XR_PASSTHROUGH_LAYER_PURPOSE_RECONSTRUCTION_FB` | Runtime-reconstructed environment passthrough. This is the normal full-environment MR background layer. |
| `Projected` | `XR_PASSTHROUGH_LAYER_PURPOSE_PROJECTED_FB` | Passthrough projected onto app-supplied triangle-mesh geometry. This requires `XR_FB_triangle_mesh`. |
| `TrackedKeyboardHands` | `XR_PASSTHROUGH_LAYER_PURPOSE_TRACKED_KEYBOARD_HANDS_FB` | Specialized runtime tracked-keyboard hand passthrough where supported. |
| `TrackedKeyboardMaskedHands` | `XR_PASSTHROUGH_LAYER_PURPOSE_TRACKED_KEYBOARD_MASKED_HANDS_FB` | Specialized tracked-keyboard hand passthrough with keyboard masking where supported. |

Unity-style "reconstructed" and "user defined/projected surface" workflows map
to the same conceptual split: runtime reconstruction versus app-supplied
projection geometry. Neither gives the app a final eye-aligned compositor image
to sample as a texture.

## Placement

`PassthroughLayerPlacement::Underlay` means submit the passthrough proxy before
the app projection layer so virtual content appears on top. `Overlay` means
submit it after app projection content so the passthrough layer can cover that
content. The OpenXR composition layer is a proxy for a runtime-owned layer; the
adapter still owns exact frame submission order.

## Style Parameters

`texture_opacity_factor` is the compositor-layer opacity. `1.0` is fully
visible passthrough and `0.0` is transparent.

`edge_color` controls edge rendering. Alpha `0.0` disables visible edge
rendering. Nonzero alpha lets the runtime draw detected edges using the given
linear RGBA color.

`color_reproduction` is intentionally one mode at a time. Meta/OpenXR exposes
style structs through an extension chain, but Quest runtime validation can
reject incompatible chains. Public Rusty XR therefore keeps color maps, BCS,
and LUTs mutually exclusive at the contract level.

| Mode | Parameters | What It Does |
| --- | --- | --- |
| `None` | No extra color struct | Keeps the runtime's native passthrough color reproduction. |
| `MonoToMono` | 256 `u8` entries | Remaps runtime luminance to another luminance value. Use for grayscale tone curves. |
| `MonoToRgba` | 256 RGBA entries | Indexes runtime luminance into a color map. Use for public gradients, including audio-reactive phase shifts. |
| `BrightnessContrastSaturation` | Brightness, contrast, saturation | Applies runtime BCS adjustment. Rusty XR validates brightness `-100..100`, contrast `>= 0`, and saturation `>= 0`; neutral is `0, 1, 1`. |
| `ColorLut` | Runtime LUT ID, weight | Binds one native 3D color LUT created by an adapter through `XR_META_passthrough_color_lut`. Weight is `0..1`. |
| `InterpolatedColorLut` | Source LUT ID, target LUT ID, weight | Interpolates between two native LUT handles. Weight is `0..1`. |

`PassthroughColorLutSpec` records the data shape for a public 3D LUT. RGB LUTs
use three bytes per element and RGBA LUTs use four. `buffer_size_bytes()` returns
`resolution^3 * channel_bytes` so adapters can validate uploads before creating
native handles.

## Source Boundaries

Native compositor passthrough is separate from these other source classes:

- Raw camera APIs such as Camera2 or a headset-camera API. These can expose
  sampleable frames, timestamps, intrinsics, and pose metadata for custom
  overlays.
- Environment depth. This is a runtime depth texture and metadata path, not a
  passthrough color layer.
- MediaProjection, casting, screenrecord, or operator streaming. These inspect
  the final display/composite output and do not provide raw stereo camera
  frames.

Keep these buckets separate when designing examples. A native passthrough
underlay is usually the right user-facing MR background. A custom camera layer
is the right path only when an app needs sampleable camera pixels and metadata.

## Compositor Access Boundary

Meta passthrough styling is a compositor-owned path. Public OpenXR extensions
let an app create passthrough features and layers, select layer purpose, submit
the passthrough proxy layer with other composition layers, and set documented
style parameters. They do not expose a lower-level shader hook into the Meta
compositor, the stereo-reconstructed passthrough image, or the runtime's
internal camera/depth fusion buffers.

That distinction matters for color effects. A 256-entry mono-to-RGBA map
indexes runtime luminance, so steep parts of a gradient can amplify small
per-eye luminance differences or runtime noise and may produce binocular
rivalry. `XR_META_passthrough_color_lut` is the better compositor-native color
path for smooth RGB remapping because the runtime maps RGB through a 3D LUT and
can interpolate between LUTs, but it is still a documented parameter path, not
a custom compositor shader.

## Audio-Reactive Example Pattern

`audio_reactive_mono_to_rgba_style()` demonstrates a generic public pattern:
map normalized phase to a wrapped 256-entry luminance-to-RGBA gradient and map
normalized amplitude to edge alpha. A real app can derive those normalized
inputs from a microphone, LSL stream, or another public signal source before a
native adapter calls the OpenXR style function.

This example is conceptually inspired by the public mixed-reality paper by
John Desnoyers-Stewart, Noah Miller, and Bernhard E. Riecke,
[DOI 10.1145/3736777](https://doi.org/10.1145/3736777), published in
Proc. ACM Computer Graphics and Interactive Techniques, Volume 8, Issue 3,
Article 42, August 2025. Rusty XR includes only generic public contracts and
synthetic examples; it does not include the artwork's assets, private
application behavior, or downstream visual-effect tuning.

## Running The Examples

```powershell
cargo run -p rusty-xr-contracts --example meta_passthrough_style_catalog --features serde
cargo run -p rusty-xr-contracts --example audio_reactive_passthrough_style --features serde
```

Both examples print JSON descriptors. They do not request permissions, create
native handles, submit OpenXR layers, upload meshes, allocate LUTs, or require a
headset.
