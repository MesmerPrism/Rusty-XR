use super::{
    openxr_gles_activity_env::{activity_float_extra, with_activity_env},
    openxr_gles_config::{
        activity_string_extra, android_system_property_f32, OesCameraProjectionMode,
        OesProjectionAlphaMode, OesProjectionBorderPolicy, OesProjectionTuning,
        DEFAULT_PROJECTION_TARGET_DEPTH_METERS, PROJECTION_PREVIEW_FOV_Y_DEGREES,
        PROJECTION_RAW_OVERSCAN,
    },
};

const OES_TUNING_PROP_PROJECTION_DEPTH_METERS: &str = "debug.rustyxr.projection.depth.meters";
const OES_TUNING_PROP_CAMERA_PREVIEW_FOV_Y_DEGREES: &str =
    "debug.rustyxr.camera.preview.fov.y.degrees";
const OES_TUNING_PROP_CAMERA_PREVIEW_OFFSET_Y_METERS: &str =
    "debug.rustyxr.camera.preview.offset.y.meters";
const OES_TUNING_PROP_CAMERA_RAW_OVERLAY_OVERSCAN: &str =
    "debug.rustyxr.camera.raw.overlay.overscan";

pub(super) fn projection_tuning_with_legacy_system_properties(
    tuning: OesProjectionTuning,
) -> OesProjectionTuning {
    OesProjectionTuning {
        projection_depth_meters: android_system_property_f32(
            OES_TUNING_PROP_PROJECTION_DEPTH_METERS,
            tuning.projection_depth_meters,
            0.05,
            10.0,
        ),
        camera_preview_fov_y_degrees: android_system_property_f32(
            OES_TUNING_PROP_CAMERA_PREVIEW_FOV_Y_DEGREES,
            tuning.camera_preview_fov_y_degrees,
            1.0,
            175.0,
        ),
        camera_preview_offset_y_meters: android_system_property_f32(
            OES_TUNING_PROP_CAMERA_PREVIEW_OFFSET_Y_METERS,
            tuning.camera_preview_offset_y_meters,
            -2.0,
            2.0,
        ),
        camera_raw_overlay_overscan: android_system_property_f32(
            OES_TUNING_PROP_CAMERA_RAW_OVERLAY_OVERSCAN,
            tuning.camera_raw_overlay_overscan,
            1.0,
            16.0,
        ),
    }
}

pub(super) fn projection_tuning_from_activity(
    app: &android_activity::AndroidApp,
) -> OesProjectionTuning {
    OesProjectionTuning {
        projection_depth_meters: projection_depth_meters_from_activity(app),
        camera_preview_fov_y_degrees: projection_preview_fov_y_degrees_from_activity(app),
        camera_preview_offset_y_meters: projection_preview_offset_y_meters_from_activity(app),
        camera_raw_overlay_overscan: projection_raw_overscan_from_activity(app),
    }
}

pub(super) fn projection_border_policy_from_activity(
    app: &android_activity::AndroidApp,
) -> OesProjectionBorderPolicy {
    with_activity_env(
        app,
        OesProjectionBorderPolicy::default(),
        |env, activity| {
            let requested = activity_string_extra(env, activity, "rustyxr.projectionBorderPolicy");
            requested
                .as_deref()
                .and_then(OesProjectionBorderPolicy::parse)
                .unwrap_or_default()
        },
    )
}

pub(super) fn camera_projection_mode_from_activity(
    app: &android_activity::AndroidApp,
) -> OesCameraProjectionMode {
    with_activity_env(app, OesCameraProjectionMode::default(), |env, activity| {
        let requested = activity_string_extra(env, activity, "rustyxr.cameraProjectionMode");
        requested
            .as_deref()
            .and_then(OesCameraProjectionMode::parse)
            .unwrap_or_default()
    })
}

fn projection_depth_meters_from_activity(app: &android_activity::AndroidApp) -> f32 {
    with_activity_env(
        app,
        DEFAULT_PROJECTION_TARGET_DEPTH_METERS,
        |env, activity| {
            activity_string_extra(env, activity, "rustyxr.projectionDepthMeters")
                .and_then(|value| value.parse::<f32>().ok())
                .filter(|value| value.is_finite())
                .unwrap_or(DEFAULT_PROJECTION_TARGET_DEPTH_METERS)
                .clamp(0.05, 10.0)
        },
    )
}

fn projection_preview_fov_y_degrees_from_activity(app: &android_activity::AndroidApp) -> f32 {
    with_activity_env(app, PROJECTION_PREVIEW_FOV_Y_DEGREES, |env, activity| {
        activity_string_extra(env, activity, "rustyxr.cameraPreviewFovYDegrees")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(PROJECTION_PREVIEW_FOV_Y_DEGREES)
            .clamp(1.0, 175.0)
    })
}

fn projection_preview_offset_y_meters_from_activity(app: &android_activity::AndroidApp) -> f32 {
    with_activity_env(app, 0.0, |env, activity| {
        activity_string_extra(env, activity, "rustyxr.cameraPreviewOffsetYMeters")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
            .clamp(-2.0, 2.0)
    })
}

fn projection_raw_overscan_from_activity(app: &android_activity::AndroidApp) -> f32 {
    with_activity_env(app, PROJECTION_RAW_OVERSCAN, |env, activity| {
        activity_string_extra(env, activity, "rustyxr.cameraRawOverlayOverscan")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(PROJECTION_RAW_OVERSCAN)
            .max(1.0)
    })
}

pub(super) fn projection_area_offset_uv_from_activity(
    app: &android_activity::AndroidApp,
) -> [f32; 2] {
    [
        projection_area_offset_x_uv_from_activity(app),
        projection_area_offset_y_uv_from_activity(app),
    ]
}

fn projection_area_offset_x_uv_from_activity(app: &android_activity::AndroidApp) -> f32 {
    with_activity_env(app, 0.0, |env, activity| {
        activity_string_extra(env, activity, "rustyxr.projectionAreaOffsetXUv")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
            .clamp(-0.5, 0.5)
    })
}

fn projection_area_offset_y_uv_from_activity(app: &android_activity::AndroidApp) -> f32 {
    with_activity_env(app, 0.0, |env, activity| {
        activity_string_extra(env, activity, "rustyxr.projectionAreaOffsetYUv")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
            .clamp(-0.5, 0.5)
    })
}

pub(super) fn projection_area_eye_offset_uv_from_activity(
    app: &android_activity::AndroidApp,
    base_offset_uv: [f32; 2],
) -> [[f32; 2]; 2] {
    with_activity_env(app, [base_offset_uv, base_offset_uv], |env, activity| {
        let left_x = activity_float_extra(env, activity, &["rustyxr.projectionAreaLeftOffsetXUv"])
            .unwrap_or(base_offset_uv[0])
            .clamp(-0.5, 0.5);
        let left_y = activity_float_extra(env, activity, &["rustyxr.projectionAreaLeftOffsetYUv"])
            .unwrap_or(base_offset_uv[1])
            .clamp(-0.5, 0.5);
        let right_x =
            activity_float_extra(env, activity, &["rustyxr.projectionAreaRightOffsetXUv"])
                .unwrap_or(base_offset_uv[0])
                .clamp(-0.5, 0.5);
        let right_y =
            activity_float_extra(env, activity, &["rustyxr.projectionAreaRightOffsetYUv"])
                .unwrap_or(base_offset_uv[1])
                .clamp(-0.5, 0.5);
        [[left_x, left_y], [right_x, right_y]]
    })
}

pub(super) fn projection_area_scale_from_activity(app: &android_activity::AndroidApp) -> [f32; 2] {
    with_activity_env(app, [1.0, 1.0], |env, activity| {
        let uniform_scale = activity_string_extra(env, activity, "rustyxr.projectionAreaScaleUv")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(1.0)
            .clamp(0.05, 4.0);
        let scale_x = activity_string_extra(env, activity, "rustyxr.projectionAreaScaleX")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(uniform_scale)
            .clamp(0.05, 4.0);
        let scale_y = activity_string_extra(env, activity, "rustyxr.projectionAreaScaleY")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(uniform_scale)
            .clamp(0.05, 4.0);
        [scale_x, scale_y]
    })
}

pub(super) fn projection_area_radius_from_activity(app: &android_activity::AndroidApp) -> [f32; 2] {
    with_activity_env(app, [0.47, 0.36], |env, activity| {
        let radius_x = activity_string_extra(env, activity, "rustyxr.projectionAreaRadiusXUv")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(0.47)
            .clamp(0.05, 0.5);
        let radius_y = activity_string_extra(env, activity, "rustyxr.projectionAreaRadiusYUv")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(0.36)
            .clamp(0.05, 0.5);
        [radius_x, radius_y]
    })
}

pub(super) fn projection_area_corner_radius_uv_from_activity(
    app: &android_activity::AndroidApp,
) -> f32 {
    with_activity_env(app, 0.08, |env, activity| {
        activity_string_extra(env, activity, "rustyxr.projectionAreaCornerRadiusUv")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(0.08)
            .clamp(0.0, 0.5)
    })
}

pub(super) fn projection_area_opacity_from_activity(app: &android_activity::AndroidApp) -> f32 {
    with_activity_env(app, 1.0, |env, activity| {
        activity_string_extra(env, activity, "rustyxr.projectionAreaOpacity")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(1.0)
            .clamp(0.0, 1.0)
    })
}

pub(super) fn projection_border_opacity_from_activity(app: &android_activity::AndroidApp) -> f32 {
    with_activity_env(app, 1.0, |env, activity| {
        activity_string_extra(env, activity, "rustyxr.projectionBorderOpacity")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(1.0)
            .clamp(0.0, 1.0)
    })
}

pub(super) fn projection_alpha_mode_from_activity(
    app: &android_activity::AndroidApp,
) -> OesProjectionAlphaMode {
    with_activity_env(app, OesProjectionAlphaMode::default(), |env, activity| {
        activity_string_extra(env, activity, "rustyxr.projectionAlphaMode")
            .as_deref()
            .and_then(OesProjectionAlphaMode::parse)
            .unwrap_or_default()
    })
}

pub(super) fn projection_alpha_scale_from_activity(app: &android_activity::AndroidApp) -> f32 {
    with_activity_env(app, 1.0, |env, activity| {
        activity_string_extra(env, activity, "rustyxr.projectionAlphaScale")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(1.0)
            .clamp(0.0, 4.0)
    })
}

pub(super) fn projection_alpha_bias_from_activity(app: &android_activity::AndroidApp) -> f32 {
    with_activity_env(app, 0.0, |env, activity| {
        activity_string_extra(env, activity, "rustyxr.projectionAlphaBias")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
            .clamp(-1.0, 1.0)
    })
}
