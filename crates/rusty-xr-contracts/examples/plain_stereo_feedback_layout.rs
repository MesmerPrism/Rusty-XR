//! Synthetic plain stereo feedback layout example.
//!
//! This demonstrates the public layout contract only. It does not acquire
//! camera frames, request MediaProjection consent, submit OpenXR layers, import
//! platform textures, or implement downstream image-processing effects.

use rusty_xr_contracts::{
    ColorRgba, Eye, ImageSize, PlainStereoLayer, Rect2, StereoLayerContentMode,
    StereoLayerPerformanceHints, StereoMediaLayout, Vec2, VisualFeedbackBorder,
    VisualFeedbackBorderLayout, VisualFeedbackLayerTuning,
};

#[derive(serde::Serialize)]
struct ExampleOutput {
    description: &'static str,
    source_kind: &'static str,
    layer: PlainStereoLayer,
    left_eye_uv: Rect2,
    right_eye_uv: Rect2,
    content_rect: Rect2,
    border_layout: VisualFeedbackBorderLayout,
    notes: [&'static str; 4],
}

fn main() -> Result<(), serde_json::Error> {
    let layer = PlainStereoLayer::new(ImageSize::new(1920, 1080), Vec2::new(1.0, 1.0))
        .with_source_layout(StereoMediaLayout::Mono)
        .with_content_mode(StereoLayerContentMode::Fit)
        .with_border(VisualFeedbackBorder::new(0.02, ColorRgba::WHITE).with_opacity(0.85))
        .with_visual_feedback_tuning(VisualFeedbackLayerTuning::MEDIA_PROJECTION_BORDER_FEEDBACK)
        .with_performance_hints(StereoLayerPerformanceHints::MEDIA_PROJECTION_FEEDBACK_BASELINE);

    assert!(layer.is_valid());

    let content_rect = layer
        .content_rect()
        .expect("valid mono layer should produce a content rectangle");
    let border_layout = layer
        .border_layout()
        .expect("valid bordered layer should produce border geometry");

    assert!(content_rect.is_valid());
    assert!(border_layout.is_valid());

    let output = ExampleOutput {
        description: "Plain mono feedback source fitted into a square projected surface.",
        source_kind: "synthetic monoscopic display-composite source",
        layer,
        left_eye_uv: layer.eye_uv_rect(Eye::Left),
        right_eye_uv: layer.eye_uv_rect(Eye::Right),
        content_rect,
        border_layout,
        notes: [
            "Both eyes sample the full mono source UV rectangle.",
            "The 16:9 source is aspect-fit into a 1m x 1m surface.",
            "Border geometry is computed from the fitted content rectangle.",
            "Native capture, permissions, transport, and compositor submission stay in adapters.",
        ],
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
