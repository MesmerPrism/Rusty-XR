use rusty_xr_contracts::{
    EffectBufferDescriptor, EffectBufferFormat, EffectDiagnosticLayer, EffectLayerComparison,
    EffectLayerComparisonMetrics, EffectLayerMetrics, EffectPassDescriptor, EffectPassInputRole,
    EffectPassKind, EffectStackComparisonReport, EffectStackDescriptor, ImageSize,
    StereoMediaLayout,
};

#[derive(serde::Serialize)]
struct EffectStackDiagnosticManifest {
    stack: EffectStackDescriptor,
    comparison: EffectStackComparisonReport,
}

fn main() {
    let stack = EffectStackDescriptor::new("sample.color_edge_stack", ImageSize::new(1280, 720))
        .with_source_layout(StereoMediaLayout::Separate)
        .with_buffer(
            EffectBufferDescriptor::new(
                "guide.luma",
                ImageSize::new(384, 384),
                EffectBufferFormat::R16Float,
            )
            .persistent(),
        )
        .with_pass(
            EffectPassDescriptor::new("source", EffectPassKind::Source)
                .with_output_buffer("source.color")
                .with_diagnostic_label("Source color"),
        )
        .with_pass(
            EffectPassDescriptor::new("luma", EffectPassKind::LumaTransform)
                .with_input("source.color", EffectPassInputRole::SourceColor)
                .with_output_buffer("guide.luma")
                .offscreen()
                .with_parameter_key("luma.contrast"),
        )
        .with_pass(
            EffectPassDescriptor::new("blur.horizontal", EffectPassKind::Blur)
                .with_input("guide.luma", EffectPassInputRole::Guide)
                .with_output_buffer("guide.blur_h")
                .offscreen()
                .separable()
                .with_parameter_key("blur.radius_px"),
        )
        .with_pass(
            EffectPassDescriptor::new("blur.vertical", EffectPassKind::Blur)
                .with_input("guide.blur_h", EffectPassInputRole::Guide)
                .with_output_buffer("guide.blur")
                .offscreen()
                .separable()
                .with_parameter_key("blur.radius_px"),
        )
        .with_pass(
            EffectPassDescriptor::new("edges", EffectPassKind::EdgeDetection)
                .with_input("guide.blur", EffectPassInputRole::Guide)
                .with_output_buffer("guide.edges")
                .offscreen()
                .with_parameter_key("edge.threshold"),
        )
        .with_pass(
            EffectPassDescriptor::new("final", EffectPassKind::Composite)
                .with_input("source.color", EffectPassInputRole::SourceColor)
                .with_input("guide.edges", EffectPassInputRole::Guide)
                .with_output_buffer("final.color"),
        )
        .with_diagnostic_layer(
            EffectDiagnosticLayer::new("raw", "Raw source")
                .from_pass("source")
                .from_buffer("source.color")
                .with_expected_role(EffectPassInputRole::SourceColor),
        )
        .with_diagnostic_layer(
            EffectDiagnosticLayer::new("blurred-guide", "Blurred guide")
                .from_pass("blur.vertical")
                .from_buffer("guide.blur"),
        )
        .with_diagnostic_layer(
            EffectDiagnosticLayer::new("edge-guide", "Edge guide")
                .from_pass("edges")
                .from_buffer("guide.edges"),
        )
        .with_diagnostic_layer(
            EffectDiagnosticLayer::new("final", "Final composite")
                .from_pass("final")
                .from_buffer("final.color")
                .with_expected_role(EffectPassInputRole::SourceColor),
        );

    assert!(stack.is_valid());

    let comparison = EffectStackComparisonReport::new(
        "report-001",
        "sample.color_edge_stack",
        "reference",
        "candidate",
    )
    .with_layer(
        EffectLayerComparison::new("raw")
            .with_reference(
                EffectLayerMetrics::new(0.82)
                    .with_luma(0.44, 0.18)
                    .with_energy(0.061, 0.027),
            )
            .with_candidate(
                EffectLayerMetrics::new(0.81)
                    .with_luma(0.45, 0.18)
                    .with_energy(0.060, 0.026),
            )
            .with_pair(
                EffectLayerComparisonMetrics::new(0.03, 0.98, 0.96).with_luma_fit(0.01, 0.97),
            )
            .with_note("source projection is close enough for guide comparison"),
    )
    .with_layer(
        EffectLayerComparison::new("blurred-guide")
            .with_pair(
                EffectLayerComparisonMetrics::new(0.11, 1.32, 1.41).with_luma_fit(0.02, 0.83),
            )
            .with_note("candidate guide is sharper than reference"),
    )
    .with_layer(
        EffectLayerComparison::new("edge-guide")
            .with_pair(EffectLayerComparisonMetrics::new(0.08, 1.05, 1.02))
            .with_note("edge detector response is close"),
    );

    assert!(comparison.is_valid());
    assert_eq!(
        comparison.first_layer_outside_blur_tolerance(0.15),
        Some("blurred-guide")
    );

    let manifest = EffectStackDiagnosticManifest { stack, comparison };

    println!(
        "{}",
        serde_json::to_string_pretty(&manifest).expect("manifest should serialize")
    );
}
