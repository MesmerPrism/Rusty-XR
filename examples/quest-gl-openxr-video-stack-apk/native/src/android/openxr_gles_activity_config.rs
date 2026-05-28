use super::{
    openxr_gles_activity_extras::{
        projection_tuning_with_legacy_system_properties, read_oes_activity_extras,
    },
    openxr_gles_config::{OesActivityConfig, OesProjectionRuntimeState},
};

impl OesActivityConfig {
    pub(super) fn from_activity(app: &android_activity::AndroidApp) -> Self {
        let extras = read_oes_activity_extras(app);
        let projection_state = OesProjectionRuntimeState {
            tuning: extras.base_projection_tuning,
            projection_area_offset_uv: extras.projection_area_offset_uv,
            projection_area_eye_offset_uv: extras.projection_area_eye_offset_uv,
            projection_area_scale: extras.projection_area_scale,
            projection_area_radius: extras.projection_area_radius,
            projection_area_corner_radius_uv: extras.projection_area_corner_radius_uv,
            projection_area_opacity: extras.projection_area_opacity,
            projection_border_opacity: extras.projection_border_opacity,
            projection_alpha_mode: extras.projection_alpha_mode,
            projection_alpha_scale: extras.projection_alpha_scale,
            projection_alpha_bias: extras.projection_alpha_bias,
            camera_projection_mode: extras.camera_projection_mode,
            projection_border_policy: extras.projection_border_policy,
            processing_layer: extras.processing_layer,
            blur_radius_px: extras.blur_radius_px,
            peripheral_stretch: extras.peripheral_stretch,
        };

        Self {
            base_projection_tuning: extras.base_projection_tuning,
            projection_state,
            camera_color_controls: extras.camera_color_controls,
        }
    }
}

impl OesProjectionRuntimeState {
    pub(super) fn with_legacy_system_properties(self) -> Self {
        Self {
            tuning: projection_tuning_with_legacy_system_properties(self.tuning),
            ..self
        }
    }
}
