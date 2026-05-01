//! Synthetic audio-reactive native passthrough style example.
//!
//! This models the public control pattern only: normalized audio phase and
//! amplitude drive a passthrough color map and edge opacity. It does not open a
//! microphone, analyze live audio, submit OpenXR layers, or reproduce
//! downstream project-specific visual behavior.

use rusty_xr_contracts::{
    audio_reactive_mono_to_rgba_style, ColorRgba, PassthroughGradientStop, PlatformPassthroughLayer,
};

#[derive(serde::Serialize)]
struct AudioReactiveSnapshot {
    frame_index: u64,
    phase01: f32,
    amplitude01: f32,
    layer: PlatformPassthroughLayer,
}

#[derive(serde::Serialize)]
struct ExampleOutput {
    description: &'static str,
    snapshots: Vec<AudioReactiveSnapshot>,
    implementation_notes: [&'static str; 4],
}

fn public_gradient() -> Vec<PassthroughGradientStop> {
    vec![
        PassthroughGradientStop::new(0.0, ColorRgba::new(0.04, 0.04, 0.09, 1.0)),
        PassthroughGradientStop::new(0.35, ColorRgba::new(0.05, 0.55, 0.88, 1.0)),
        PassthroughGradientStop::new(0.72, ColorRgba::new(0.88, 0.18, 0.58, 1.0)),
        PassthroughGradientStop::new(1.0, ColorRgba::new(0.96, 0.86, 0.24, 1.0)),
    ]
}

fn main() -> Result<(), serde_json::Error> {
    let gradient = public_gradient();
    let controls = [(0_u64, 0.05, 0.08), (1, 0.32, 0.38), (2, 0.69, 0.81)];
    let snapshots: Vec<AudioReactiveSnapshot> = controls
        .into_iter()
        .map(|(frame_index, phase01, amplitude01)| {
            let style = audio_reactive_mono_to_rgba_style(phase01, amplitude01, &gradient)
                .expect("normalized controls should build a public passthrough style");
            let layer = PlatformPassthroughLayer::reconstruction_underlay().with_style(style);
            assert!(layer.is_valid());
            AudioReactiveSnapshot {
                frame_index,
                phase01,
                amplitude01,
                layer,
            }
        })
        .collect();

    let output = ExampleOutput {
        description: "Synthetic audio-reactive passthrough style descriptors.",
        snapshots,
        implementation_notes: [
            "A real app can map microphone, LSL, or other public input streams into phase and amplitude.",
            "The helper shifts a 256-entry luminance-to-RGBA map and scales edge alpha from amplitude.",
            "The native adapter still owns OpenXR session state and xrPassthroughLayerSetStyleFB calls.",
            "Color-map style is kept separate from BCS and LUT style modes for runtime compatibility.",
        ],
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
