use ash::vk;
use openxr as xr;

use crate::HeadsetCameraGpuFrame;

use super::projection_view_basis::fov_aspect;

pub(super) fn full_target_canvas_aspect(
    display_view: &xr::View,
    resolution: vk::Extent2D,
) -> (f32, &'static str) {
    if let Some(aspect) = fov_aspect(display_view.fov) {
        return (aspect.clamp(0.25, 4.0), "display-eye-fov");
    }
    if resolution.height > 0 {
        return (
            (resolution.width as f32 / resolution.height as f32).clamp(0.25, 4.0),
            "swapchain-resolution-fallback",
        );
    }
    (1.0, "square-fallback")
}

pub(super) fn content_surface_aspect(
    width: f32,
    height: f32,
    resolution: vk::Extent2D,
) -> (f32, &'static str) {
    if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
        return ((width / height).clamp(0.25, 4.0), "camera-content-size");
    }
    if resolution.height > 0 {
        return (
            (resolution.width as f32 / resolution.height as f32).clamp(0.25, 4.0),
            "swapchain-resolution-fallback",
        );
    }
    (1.0, "square-fallback")
}

pub(super) fn frame_requests_full_frame_stimulus_mapping(frame: &HeadsetCameraGpuFrame) -> bool {
    let full_frame_profile = frame
        .diagnostics
        .synthetic_projection_profile
        .as_deref()
        .is_some_and(|value| value == "full-frame-diagnostic")
        || frame
            .diagnostics
            .projection_geometry_profile
            .as_deref()
            .is_some_and(|value| value == "full-frame-diagnostic");
    if !full_frame_profile {
        return false;
    }
    let Some(mapping_intent) = frame.diagnostics.content_mapping_intent.as_deref() else {
        return false;
    };
    matches!(
        mapping_intent,
        "map-full-frame-stimulus-to-projection-area"
            | "map-full-frame-stimulus-to-projection-surface"
            | "map-full-frame-content-to-projection-area"
            | "map-full-frame-content-to-projection-surface"
    )
}
