use rusty_xr_camera_model::{Rect2, TargetScreenFootprint, Vec2};

use crate::{HeadsetCameraFrameDiagnostics, RuntimeConfig, StereoGpuCameraFrame};

#[derive(Clone, Copy, Debug)]
pub(super) struct HwbTargetFootprintParams {
    pub(super) from_metadata: bool,
    pub(super) area_params: [f32; 4],
    pub(super) area_offset_params: [f32; 4],
}

impl HwbTargetFootprintParams {
    pub(super) fn from_config(config: &RuntimeConfig) -> Self {
        Self {
            from_metadata: false,
            area_params: config.camera_area_params_push(),
            area_offset_params: config.camera_area_offset_params_push(),
        }
    }
}

pub(super) fn target_footprint_params_from_mono_frame(
    diagnostics: &HeadsetCameraFrameDiagnostics,
    config: &RuntimeConfig,
) -> HwbTargetFootprintParams {
    let Some(target) = target_footprint_from_diagnostics(diagnostics) else {
        return HwbTargetFootprintParams::from_config(config);
    };
    params_from_targets(target, target)
}

pub(super) fn target_footprint_params_from_stereo_frame(
    frame: &StereoGpuCameraFrame,
    config: &RuntimeConfig,
) -> HwbTargetFootprintParams {
    let (Some(left), Some(right)) = (
        target_footprint_from_diagnostics(&frame.left.diagnostics),
        target_footprint_from_diagnostics(&frame.right.diagnostics),
    ) else {
        return HwbTargetFootprintParams::from_config(config);
    };
    params_from_targets(left, right)
}

pub(super) fn diagnostics_has_target_footprint(
    diagnostics: &HeadsetCameraFrameDiagnostics,
) -> bool {
    target_footprint_from_diagnostics(diagnostics).is_some()
}

fn params_from_targets(
    left: TargetScreenFootprint,
    right: TargetScreenFootprint,
) -> HwbTargetFootprintParams {
    let left_rect = left.visible_screen_uv_rect;
    let right_rect = right.visible_screen_uv_rect;
    let left_center = rect_center(left_rect);
    let right_center = rect_center(right_rect);
    let radius_x = ((left_rect.size.x + right_rect.size.x) * 0.25).clamp(0.001, 0.5);
    let radius_y = ((left_rect.size.y + right_rect.size.y) * 0.25).clamp(0.001, 0.5);
    HwbTargetFootprintParams {
        from_metadata: true,
        area_params: [radius_x, radius_y, 0.0, 1.0],
        area_offset_params: [
            left_center.x - 0.5,
            left_center.y - 0.5,
            right_center.x - 0.5,
            right_center.y - 0.5,
        ],
    }
}

fn target_footprint_from_diagnostics(
    diagnostics: &HeadsetCameraFrameDiagnostics,
) -> Option<TargetScreenFootprint> {
    let [x, y, width, height] = diagnostics.target_screen_uv_rect?;
    TargetScreenFootprint::from_display_eye_screen_uv_rect(Rect2::new(
        Vec2::new(x, y),
        Vec2::new(width, height),
    ))
}

fn rect_center(rect: Rect2) -> Vec2 {
    rect.origin + rect.size * 0.5
}
