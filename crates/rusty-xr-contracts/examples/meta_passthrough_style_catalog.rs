//! Synthetic Meta/OpenXR passthrough style catalog.
//!
//! This demonstrates public descriptors only. It does not create OpenXR
//! handles, submit composition layers, upload meshes, allocate runtime LUTs, or
//! depend on headset hardware.

use rusty_xr_contracts::{
    ColorRgba, PassthroughColorAdjustment, PassthroughColorLutBinding,
    PassthroughColorReproduction, PassthroughExtensionRequirements, PassthroughGradientStop,
    PassthroughLayerPlacement, PassthroughLayerPurpose, PassthroughMonoToMonoMap,
    PassthroughMonoToRgbaMap, PassthroughStyle, PlatformPassthroughLayer,
};

#[derive(serde::Serialize)]
struct CatalogEntry {
    id: &'static str,
    layer: PlatformPassthroughLayer,
    style_mode: &'static str,
    required_extensions: Vec<&'static str>,
    explanation: &'static str,
    adapter_notes: Vec<&'static str>,
}

#[derive(serde::Serialize)]
struct CatalogOutput {
    description: &'static str,
    entries: Vec<CatalogEntry>,
    boundary_notes: [&'static str; 4],
}

fn catalog_entry(
    id: &'static str,
    layer: PlatformPassthroughLayer,
    explanation: &'static str,
    adapter_notes: Vec<&'static str>,
) -> CatalogEntry {
    assert!(layer.is_valid());
    let requirements: PassthroughExtensionRequirements = layer.extension_requirements();
    CatalogEntry {
        id,
        style_mode: layer.style.color_reproduction.stable_id(),
        required_extensions: requirements.extension_names(),
        layer,
        explanation,
        adapter_notes,
    }
}

fn public_gradient() -> Vec<PassthroughGradientStop> {
    vec![
        PassthroughGradientStop::new(0.0, ColorRgba::new(0.03, 0.04, 0.10, 1.0)),
        PassthroughGradientStop::new(0.45, ColorRgba::new(0.10, 0.65, 0.95, 1.0)),
        PassthroughGradientStop::new(1.0, ColorRgba::new(0.95, 0.84, 0.18, 1.0)),
    ]
}

fn main() -> Result<(), serde_json::Error> {
    let neutral_underlay = PlatformPassthroughLayer::reconstruction_underlay();

    let bcs_overlay = PlatformPassthroughLayer::reconstruction_underlay()
        .with_placement(PassthroughLayerPlacement::Overlay)
        .with_style(
            PassthroughStyle::NEUTRAL
                .with_texture_opacity_factor(0.72)
                .with_edge_color(ColorRgba::new(0.25, 0.75, 1.0, 0.38))
                .with_color_reproduction(
                    PassthroughColorReproduction::BrightnessContrastSaturation(
                        PassthroughColorAdjustment::new(4.0, 1.08, 0.92),
                    ),
                ),
        );

    let mono_to_mono_underlay = PlatformPassthroughLayer::reconstruction_underlay().with_style(
        PassthroughStyle::NEUTRAL.with_color_reproduction(
            PassthroughColorReproduction::MonoToMono(PassthroughMonoToMonoMap::inverted()),
        ),
    );

    let projected_color_map = PlatformPassthroughLayer::new(PassthroughLayerPurpose::Projected)
        .with_style(
            PassthroughStyle::NEUTRAL.with_color_reproduction(
                PassthroughColorReproduction::MonoToRgba(
                    PassthroughMonoToRgbaMap::from_gradient(&public_gradient())
                        .expect("public gradient should build"),
                ),
            ),
        );

    let lut_grade = PlatformPassthroughLayer::reconstruction_underlay().with_style(
        PassthroughStyle::NEUTRAL.with_color_reproduction(PassthroughColorReproduction::ColorLut(
            PassthroughColorLutBinding::new("example-warm-grade", 0.65),
        )),
    );

    let output = CatalogOutput {
        description: "Contracts-only catalog for native compositor passthrough styles.",
        entries: vec![
            catalog_entry(
                "reconstruction-underlay-neutral",
                neutral_underlay,
                "Runtime reconstructed passthrough submitted behind app projection layers.",
                vec![
                    "Create and start a native passthrough feature and reconstruction layer.",
                    "Submit the passthrough proxy layer before app projection content.",
                ],
            ),
            catalog_entry(
                "reconstruction-overlay-bcs-edge",
                bcs_overlay,
                "Runtime reconstructed passthrough with opacity, edge rendering, and BCS adjustment.",
                vec![
                    "Submit after app projection content when the passthrough layer should cover it.",
                    "Keep BCS as the only chained color-style extension unless the runtime has been validated otherwise.",
                ],
            ),
            catalog_entry(
                "reconstruction-underlay-inverted-luma",
                mono_to_mono_underlay,
                "Runtime reconstructed passthrough with a 256-entry luminance remap.",
                vec![
                    "Use this shape for grayscale tone curves.",
                    "The adapter copies all 256 entries into the OpenXR mono-to-mono style struct.",
                ],
            ),
            catalog_entry(
                "projected-mesh-color-map",
                projected_color_map,
                "Passthrough projected onto app-supplied mesh geometry with a 256-entry RGBA map.",
                vec![
                    "Projected passthrough needs app geometry and the triangle-mesh extension.",
                    "The color map indexes runtime luminance into public RGBA gradient entries.",
                ],
            ),
            catalog_entry(
                "reconstruction-underlay-lut-binding",
                lut_grade,
                "Runtime reconstructed passthrough referencing a runtime-created 3D color LUT.",
                vec![
                    "This is a descriptor for a LUT handle owned by a native adapter.",
                    "The contract records the intended LUT ID and blend weight, not the platform handle.",
                ],
            ),
        ],
        boundary_notes: [
            "Native passthrough is compositor-owned and not a sampleable camera texture.",
            "OpenXR handle creation, layer submission, mesh upload, and LUT allocation stay in adapters.",
            "Raw camera overlays, environment depth, and final-display capture are separate source classes.",
            "Examples use synthetic public values and avoid downstream visual-effect tuning.",
        ],
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
