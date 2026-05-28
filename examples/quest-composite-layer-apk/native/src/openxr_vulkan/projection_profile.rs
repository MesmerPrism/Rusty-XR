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

pub(super) fn frame_source_sampling_mode(frame: &HeadsetCameraGpuFrame) -> &'static str {
    if let Some(mode) = frame.diagnostics.source_sampling_mode.as_deref() {
        match mode.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "target-local-raster"
            | "target-local"
            | "target-raster"
            | "local-raster"
            | "raster"
            | "default" => return "target-local-raster",
            "screen-to-camera-homography"
            | "screen-camera-homography"
            | "screen-to-source-homography"
            | "camera-homography"
            | "camera-projection"
            | "homography" => return "screen-to-camera-homography",
            _ => {}
        }
    }
    let mapping_intent = frame.diagnostics.content_mapping_intent.as_deref();
    match mapping_intent {
        Some(
            "map-camera-frame-through-screen-to-camera-homography"
            | "map-stimulus-raster-through-camera-projection",
        ) => "screen-to-camera-homography",
        Some(
            "map-camera-frame-to-full-frame-projection-area"
            | "map-camera-frame-to-full-frame-projection-surface"
            | "map-full-frame-stimulus-to-projection-area"
            | "map-full-frame-stimulus-to-projection-surface"
            | "map-full-frame-content-to-projection-area"
            | "map-full-frame-content-to-projection-surface",
        ) => "target-local-raster",
        _ => {
            if frame
                .diagnostics
                .projection_geometry_profile
                .as_deref()
                .is_some_and(|value| value == "camera-projection")
            {
                "screen-to-camera-homography"
            } else {
                "target-local-raster"
            }
        }
    }
}

pub(super) fn frame_requests_target_local_raster_sampling(frame: &HeadsetCameraGpuFrame) -> bool {
    frame_source_sampling_mode(frame) == "target-local-raster"
}
