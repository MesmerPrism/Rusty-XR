//! Synthetic composite feedback session example.
//!
//! This models the public contracts that a downstream app and companion tool
//! can exchange around a display-composite feedback session. It does not build
//! an APK, request headset permissions, acquire frames, submit native layers,
//! stream pixels, or implement downstream visual effects.

use rusty_xr_contracts::{
    CaptureLifecycleState, CapturePermissionState, CaptureSourceKind, CaptureSourceState,
    ColorRgba, EnvironmentDepthState, ImageSize, PlainStereoLayer, Rect2, RoomMeshSourceKind,
    RoomMeshSourceState, StereoLayerContentMode, StereoLayerPerformanceHints, StereoMediaLayout,
    Vec2, VisualFeedbackBorder, VisualFeedbackBorderLayout, VisualFeedbackLayerTuning,
};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CompanionCatalogHint {
    schema_version: &'static str,
    app_id: &'static str,
    package_name: &'static str,
    apk_file: Option<&'static str>,
    runtime_profile_id: &'static str,
    device_profile_id: &'static str,
}

#[derive(serde::Serialize)]
struct SessionDiagnostics {
    capture_sources: Vec<CaptureSourceState>,
    room_mesh: RoomMeshSourceState,
    environment_depth: EnvironmentDepthState,
}

#[derive(serde::Serialize)]
struct ExampleOutput {
    description: &'static str,
    companion_catalog_hint: CompanionCatalogHint,
    feedback_layer: PlainStereoLayer,
    content_rect: Rect2,
    border_layout: VisualFeedbackBorderLayout,
    diagnostics: SessionDiagnostics,
    boundary_notes: [&'static str; 5],
}

fn main() -> Result<(), serde_json::Error> {
    let feedback_layer = PlainStereoLayer::new(ImageSize::new(1920, 1080), Vec2::new(1.0, 1.0))
        .with_source_layout(StereoMediaLayout::Mono)
        .with_content_mode(StereoLayerContentMode::Fit)
        .with_border(VisualFeedbackBorder::new(0.018, ColorRgba::WHITE).with_opacity(0.82))
        .with_visual_feedback_tuning(VisualFeedbackLayerTuning::MEDIA_PROJECTION_BORDER_FEEDBACK)
        .with_performance_hints(StereoLayerPerformanceHints::MEDIA_PROJECTION_FEEDBACK_BASELINE);

    assert!(feedback_layer.is_valid());

    let content_rect = feedback_layer
        .content_rect()
        .expect("valid feedback layer should produce a content rectangle");
    let border_layout = feedback_layer
        .border_layout()
        .expect("valid feedback layer should produce border geometry");

    assert!(content_rect.is_valid());
    assert!(border_layout.is_valid());

    let media_projection_source = CaptureSourceState::new(CaptureSourceKind::MediaProjection)
        .with_lifecycle(CaptureLifecycleState::Running)
        .with_permission(CapturePermissionState::Granted)
        .with_counts(240, 3)
        .with_last_frame_time_ns(8_333_333_000);
    let app_render_source = CaptureSourceState::new(CaptureSourceKind::AppRender)
        .with_lifecycle(CaptureLifecycleState::Running)
        .with_permission(CapturePermissionState::NotRequired)
        .with_counts(480, 0)
        .with_last_frame_time_ns(8_333_333_000);
    let room_mesh = RoomMeshSourceState::new(RoomMeshSourceKind::Synthetic)
        .with_lifecycle(CaptureLifecycleState::Idle)
        .with_permission(CapturePermissionState::NotRequired)
        .with_mesh_counts(1, 4, 2, 1)
        .with_last_update_time_ns(8_000_000_000);
    let environment_depth = EnvironmentDepthState {
        supported: true,
        permission_granted: true,
        provider_created: false,
        provider_running: false,
        frame_available: false,
    };

    assert!(media_projection_source.is_capturing());
    assert!(app_render_source.is_capturing());
    assert!(room_mesh.has_mesh());
    assert!(!environment_depth.is_active());

    let output = ExampleOutput {
        description: "Synthetic composite feedback session contract with no native capture.",
        companion_catalog_hint: CompanionCatalogHint {
            schema_version: "rusty.xr.quest-app-catalog.v1",
            app_id: "synthetic-composite-feedback",
            package_name: "com.example.rustyxr.feedback",
            apk_file: None,
            runtime_profile_id: "feedback-demo",
            device_profile_id: "balanced-dev",
        },
        feedback_layer,
        content_rect,
        border_layout,
        diagnostics: SessionDiagnostics {
            capture_sources: vec![media_projection_source, app_render_source],
            room_mesh,
            environment_depth,
        },
        boundary_notes: [
            "MediaProjection consent and foreground service setup stay in an app shell.",
            "Companion catalogs describe public APK metadata but do not carry APK bytes by default.",
            "Native OpenXR composition and platform texture import stay in optional adapters.",
            "Room mesh and depth values here are diagnostic state, not live provider calls.",
            "Downstream visual effects are intentionally outside this public example.",
        ],
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
