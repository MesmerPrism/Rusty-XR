use super::openxr_gles_config::{
    OesColorControls, OesProcessingLayer, OesProjectionAlphaMode, OesProjectionBorderPolicy,
    OesProjectionRuntimeState,
};

#[derive(Clone, Copy, Debug)]
pub(super) struct OesRenderTuning {
    pub(super) projection_border_policy: OesProjectionBorderPolicy,
    pub(super) processing_layer: OesProcessingLayer,
    pub(super) blur_radius_px: f32,
    pub(super) projection_area_eye_offset_uv: [[f32; 2]; 2],
    pub(super) projection_area_scale: [f32; 2],
    pub(super) projection_area_radius: [f32; 2],
    pub(super) projection_area_corner_radius_uv: f32,
    pub(super) projection_area_opacity: f32,
    pub(super) projection_border_opacity: f32,
    pub(super) projection_alpha_mode: OesProjectionAlphaMode,
    pub(super) projection_alpha_scale: f32,
    pub(super) projection_alpha_bias: f32,
    pub(super) camera_color_controls: OesColorControls,
}

impl OesRenderTuning {
    pub(super) fn from_projection_state(
        projection_state: OesProjectionRuntimeState,
        processing_layer: OesProcessingLayer,
        blur_radius_px: f32,
        camera_color_controls: OesColorControls,
    ) -> Self {
        Self {
            projection_border_policy: projection_state.projection_border_policy,
            processing_layer,
            blur_radius_px,
            projection_area_eye_offset_uv: projection_state.projection_area_eye_offset_uv,
            projection_area_scale: projection_state.projection_area_scale,
            projection_area_radius: projection_state.projection_area_radius,
            projection_area_corner_radius_uv: projection_state.projection_area_corner_radius_uv,
            projection_area_opacity: projection_state.projection_area_opacity,
            projection_border_opacity: projection_state.projection_border_opacity,
            projection_alpha_mode: projection_state.projection_alpha_mode,
            projection_alpha_scale: projection_state.projection_alpha_scale,
            projection_alpha_bias: projection_state.projection_alpha_bias,
            camera_color_controls,
        }
    }
}
