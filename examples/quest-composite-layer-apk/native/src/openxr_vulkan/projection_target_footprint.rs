use rusty_xr_camera_model::{Rect2, TargetScreenFootprint, Vec2};

use crate::{HeadsetCameraFrameDiagnostics, RuntimeConfig, StereoGpuCameraFrame};

#[derive(Clone, Copy, Debug)]
pub(super) struct HwbTargetFootprintParams {
    pub(super) from_metadata: bool,
    pub(super) area_params: [f32; 4],
    pub(super) area_offset_params: [f32; 4],
    pub(super) area_radius_params: [f32; 4],
}

impl HwbTargetFootprintParams {
    pub(super) fn from_config(config: &RuntimeConfig) -> Self {
        let area_params = config.camera_area_params_push();
        Self {
            from_metadata: false,
            area_params,
            area_offset_params: config.camera_area_offset_params_push(),
            area_radius_params: [
                area_params[0],
                area_params[1],
                area_params[0],
                area_params[1],
            ],
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
    let left_radius = rect_radius(left_rect);
    let right_radius = rect_radius(right_rect);
    let radius_x = ((left_radius.x + right_radius.x) * 0.5).clamp(0.001, 0.5);
    let radius_y = ((left_radius.y + right_radius.y) * 0.5).clamp(0.001, 0.5);
    HwbTargetFootprintParams {
        from_metadata: true,
        area_params: [radius_x, radius_y, 0.0, 1.0],
        area_offset_params: [
            left_center.x - 0.5,
            left_center.y - 0.5,
            right_center.x - 0.5,
            right_center.y - 0.5,
        ],
        area_radius_params: [left_radius.x, left_radius.y, right_radius.x, right_radius.y],
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

fn rect_radius(rect: Rect2) -> Vec2 {
    Vec2::new(
        (rect.size.x * 0.5).clamp(0.001, 0.5),
        (rect.size.y * 0.5).clamp(0.001, 0.5),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn footprint(x: f32, y: f32, width: f32, height: f32) -> TargetScreenFootprint {
        TargetScreenFootprint::from_display_eye_screen_uv_rect(Rect2::new(
            Vec2::new(x, y),
            Vec2::new(width, height),
        ))
        .expect("valid target footprint")
    }

    #[test]
    fn metadata_target_footprint_preserves_per_eye_size() {
        let params = params_from_targets(
            footprint(0.171875, 0.21875, 0.75, 0.65625),
            footprint(0.078125, 0.21875, 0.75, 0.671875),
        );

        assert!(params.from_metadata);
        assert_eq!(
            params.area_offset_params,
            [0.046875, 0.046875, -0.046875, 0.0546875]
        );
        assert_eq!(
            params.area_radius_params,
            [0.375, 0.328125, 0.375, 0.3359375]
        );
        assert_eq!(params.area_params, [0.375, 0.33203125, 0.0, 1.0]);
    }
}
