//! Rust native payload for the public minimal Quest APK example.
//!
//! The library exports one JNI function that returns a synthetic Rusty XR
//! session contract as JSON. It does not call Android capture APIs, OpenXR,
//! Vulkan, or headset-specific services.

use jni::{objects::JClass, sys::jstring, JNIEnv};
use rusty_xr_contracts::{
    CaptureLifecycleState, CapturePermissionState, CaptureSourceKind, CaptureSourceState,
    ColorRgba, EnvironmentDepthState, ImageSize, PlainStereoLayer, Rect2, RoomMeshSourceKind,
    RoomMeshSourceState, StereoLayerContentMode, StereoLayerPerformanceHints, StereoMediaLayout,
    Vec2, VisualFeedbackBorder, VisualFeedbackBorderLayout, VisualFeedbackLayerTuning,
};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MinimalQuestSession {
    schema_version: &'static str,
    app_id: &'static str,
    package_name: &'static str,
    activity_name: &'static str,
    feedback_layer: PlainStereoLayer,
    content_rect: Rect2,
    border_layout: VisualFeedbackBorderLayout,
    capture_sources: [CaptureSourceState; 2],
    room_mesh: RoomMeshSourceState,
    environment_depth: EnvironmentDepthState,
    notes: [&'static str; 4],
}

pub fn session_json() -> String {
    let feedback_layer = PlainStereoLayer::new(ImageSize::new(1920, 1080), Vec2::new(1.0, 1.0))
        .with_source_layout(StereoMediaLayout::Mono)
        .with_content_mode(StereoLayerContentMode::Fit)
        .with_border(VisualFeedbackBorder::new(0.018, ColorRgba::WHITE).with_opacity(0.82))
        .with_visual_feedback_tuning(VisualFeedbackLayerTuning::MEDIA_PROJECTION_BORDER_FEEDBACK)
        .with_performance_hints(StereoLayerPerformanceHints::MEDIA_PROJECTION_FEEDBACK_BASELINE);

    let content_rect = feedback_layer
        .content_rect()
        .expect("minimal example layer should be valid");
    let border_layout = feedback_layer
        .border_layout()
        .expect("minimal example layer should produce border layout");
    let capture_sources = [
        CaptureSourceState::new(CaptureSourceKind::AppRender)
            .with_lifecycle(CaptureLifecycleState::Running)
            .with_permission(CapturePermissionState::NotRequired)
            .with_counts(1, 0),
        CaptureSourceState::new(CaptureSourceKind::Synthetic)
            .with_lifecycle(CaptureLifecycleState::Idle)
            .with_permission(CapturePermissionState::NotRequired),
    ];
    let room_mesh = RoomMeshSourceState::new(RoomMeshSourceKind::Synthetic)
        .with_lifecycle(CaptureLifecycleState::Idle)
        .with_permission(CapturePermissionState::NotRequired)
        .with_mesh_counts(1, 4, 2, 1);
    let environment_depth = EnvironmentDepthState {
        supported: false,
        permission_granted: false,
        provider_created: false,
        provider_running: false,
        frame_available: false,
    };

    let session = MinimalQuestSession {
        schema_version: "rusty.xr.quest-app-catalog.v1",
        app_id: "rusty-xr-quest-minimal",
        package_name: "com.example.rustyxr.minimal",
        activity_name: ".MainActivity",
        feedback_layer,
        content_rect,
        border_layout,
        capture_sources,
        room_mesh,
        environment_depth,
        notes: [
            "This APK proves Rust native loading and public contract serialization.",
            "The first Quest example is a 2D Android smoke test, not an OpenXR scene.",
            "Companion install, launch, stop, snapshot, and verifier flows own device operations.",
            "Native passthrough, MediaProjection, depth, and compositor layers remain deferred.",
        ],
    };

    serde_json::to_string_pretty(&session).expect("minimal session should serialize")
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "system" fn Java_com_example_rustyxr_minimal_MainActivity_sessionJson(
    env: JNIEnv<'_>,
    _class: JClass<'_>,
) -> jstring {
    match env.new_string(session_json()) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::session_json;

    #[test]
    fn session_json_contains_public_catalog_identity() {
        let json = session_json();

        assert!(json.contains("\"appId\": \"rusty-xr-quest-minimal\""));
        assert!(json.contains("\"packageName\": \"com.example.rustyxr.minimal\""));
        assert!(json.contains("\"schemaVersion\": \"rusty.xr.quest-app-catalog.v1\""));
    }
}
