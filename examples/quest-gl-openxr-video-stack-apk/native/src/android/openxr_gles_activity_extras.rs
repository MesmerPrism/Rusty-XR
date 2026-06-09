use super::{
    openxr_gles_activity_color::camera_color_controls_from_activity,
    openxr_gles_activity_env::{activity_float_extra, with_activity_env},
    openxr_gles_activity_projection::{
        camera_projection_mode_from_activity, projection_alpha_bias_from_activity,
        projection_alpha_mode_from_activity, projection_alpha_scale_from_activity,
        projection_area_corner_radius_uv_from_activity,
        projection_area_eye_offset_uv_from_activity, projection_area_offset_uv_from_activity,
        projection_area_opacity_from_activity, projection_area_radius_from_activity,
        projection_area_scale_from_activity, projection_border_opacity_from_activity,
        projection_border_policy_from_activity, projection_tuning_from_activity,
    },
    openxr_gles_config::{
        activity_string_extra, OesCameraProjectionMode, OesColorControls,
        OesPeripheralStretchBlendMode, OesPeripheralStretchConfig, OesPeripheralStretchCornerMode,
        OesPeripheralStretchDebug, OesPeripheralStretchMode, OesProcessingLayer,
        OesProjectionAlphaMode, OesProjectionBorderPolicy, OesProjectionTuning,
    },
};

pub(super) use super::openxr_gles_activity_projection::projection_tuning_with_legacy_system_properties;

pub(super) struct OesActivityExtras {
    pub(super) processing_layer: OesProcessingLayer,
    pub(super) blur_radius_px: f32,
    pub(super) base_projection_tuning: OesProjectionTuning,
    pub(super) projection_area_offset_uv: [f32; 2],
    pub(super) projection_area_eye_offset_uv: [[f32; 2]; 2],
    pub(super) projection_area_scale: [f32; 2],
    pub(super) projection_area_radius: [f32; 2],
    pub(super) projection_area_corner_radius_uv: f32,
    pub(super) projection_area_opacity: f32,
    pub(super) projection_border_opacity: f32,
    pub(super) projection_alpha_mode: OesProjectionAlphaMode,
    pub(super) projection_alpha_scale: f32,
    pub(super) projection_alpha_bias: f32,
    pub(super) camera_projection_mode: OesCameraProjectionMode,
    pub(super) projection_border_policy: OesProjectionBorderPolicy,
    pub(super) peripheral_stretch: OesPeripheralStretchConfig,
    pub(super) camera_color_controls: OesColorControls,
}

pub(super) fn read_oes_activity_extras(app: &android_activity::AndroidApp) -> OesActivityExtras {
    let base_projection_tuning = projection_tuning_from_activity(app);
    let projection_area_offset_uv = projection_area_offset_uv_from_activity(app);
    OesActivityExtras {
        processing_layer: processing_layer_from_activity(app),
        blur_radius_px: blur_radius_px_from_activity(app),
        base_projection_tuning,
        projection_area_offset_uv,
        projection_area_eye_offset_uv: projection_area_eye_offset_uv_from_activity(
            app,
            projection_area_offset_uv,
        ),
        projection_area_scale: projection_area_scale_from_activity(app),
        projection_area_radius: projection_area_radius_from_activity(app),
        projection_area_corner_radius_uv: projection_area_corner_radius_uv_from_activity(app),
        projection_area_opacity: projection_area_opacity_from_activity(app),
        projection_border_opacity: projection_border_opacity_from_activity(app),
        projection_alpha_mode: projection_alpha_mode_from_activity(app),
        projection_alpha_scale: projection_alpha_scale_from_activity(app),
        projection_alpha_bias: projection_alpha_bias_from_activity(app),
        camera_projection_mode: camera_projection_mode_from_activity(app),
        projection_border_policy: projection_border_policy_from_activity(app),
        peripheral_stretch: peripheral_stretch_from_activity(app),
        camera_color_controls: camera_color_controls_from_activity(app),
    }
}

fn processing_layer_from_activity(app: &android_activity::AndroidApp) -> OesProcessingLayer {
    with_activity_env(app, OesProcessingLayer::default(), |env, activity| {
        let requested = activity_string_extra(env, activity, "rustyquest.processingLayer");
        requested
            .as_deref()
            .and_then(OesProcessingLayer::parse)
            .unwrap_or_default()
    })
}

fn blur_radius_px_from_activity(app: &android_activity::AndroidApp) -> f32 {
    with_activity_env(app, 2.0, |env, activity| {
        activity_string_extra(env, activity, "rustyquest.cameraBlurRadiusPx")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(2.0)
            .clamp(0.0, 16.0)
    })
}

fn peripheral_stretch_from_activity(
    app: &android_activity::AndroidApp,
) -> OesPeripheralStretchConfig {
    let defaults = OesPeripheralStretchConfig::default();
    with_activity_env(app, defaults, |env, activity| {
        OesPeripheralStretchConfig {
            mode: activity_string_extra(env, activity, "rustyquest.peripheralStretchMode")
                .as_deref()
                .and_then(OesPeripheralStretchMode::parse)
                .unwrap_or(defaults.mode),
            core_scale: activity_float_extra(
                env,
                activity,
                &["rustyquest.peripheralStretchCoreScale"],
            )
            .unwrap_or(defaults.core_scale),
            edge_inset_uv: activity_float_extra(
                env,
                activity,
                &["rustyquest.peripheralStretchEdgeInsetUv"],
            )
            .unwrap_or(defaults.edge_inset_uv),
            max_inset_uv: activity_float_extra(
                env,
                activity,
                &["rustyquest.peripheralStretchMaxInsetUv"],
            )
            .unwrap_or(defaults.max_inset_uv),
            curve: activity_float_extra(env, activity, &["rustyquest.peripheralStretchCurve"])
                .unwrap_or(defaults.curve),
            inner_blend_uv: activity_float_extra(
                env,
                activity,
                &["rustyquest.peripheralStretchInnerBlendUv"],
            )
            .unwrap_or(defaults.inner_blend_uv),
            blend_curve: activity_float_extra(
                env,
                activity,
                &["rustyquest.peripheralStretchBlendCurve"],
            )
            .unwrap_or(defaults.blend_curve),
            blend_mode: activity_string_extra(
                env,
                activity,
                "rustyquest.peripheralStretchBlendMode",
            )
            .as_deref()
            .and_then(OesPeripheralStretchBlendMode::parse)
            .unwrap_or(defaults.blend_mode),
            corner_mode: activity_string_extra(
                env,
                activity,
                "rustyquest.peripheralStretchCornerMode",
            )
            .as_deref()
            .and_then(OesPeripheralStretchCornerMode::parse)
            .unwrap_or(defaults.corner_mode),
            debug: activity_string_extra(env, activity, "rustyquest.peripheralStretchDebug")
                .as_deref()
                .and_then(OesPeripheralStretchDebug::parse)
                .unwrap_or(defaults.debug),
        }
        .sanitized()
    })
}
