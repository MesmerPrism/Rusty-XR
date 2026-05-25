use super::openxr_gles_config::{
    activity_string_extra, android_system_property_f32, OesActivityConfig, OesCameraProjectionMode,
    OesColorControls, OesProcessingLayer, OesProjectionAlphaMode, OesProjectionBorderPolicy,
    OesProjectionRuntimeState, OesProjectionTuning, OesSourceColorTransfer,
    DEFAULT_PROJECTION_TARGET_DEPTH_METERS, PROJECTION_PREVIEW_FOV_Y_DEGREES,
    PROJECTION_RAW_OVERSCAN,
};
use jni::{objects::JObject, sys::jobject, JNIEnv, JavaVM};

const OES_TUNING_PROP_PROJECTION_DEPTH_METERS: &str = "debug.rustyxr.projection.depth.meters";
const OES_TUNING_PROP_CAMERA_PREVIEW_FOV_Y_DEGREES: &str =
    "debug.rustyxr.camera.preview.fov.y.degrees";
const OES_TUNING_PROP_CAMERA_PREVIEW_OFFSET_Y_METERS: &str =
    "debug.rustyxr.camera.preview.offset.y.meters";
const OES_TUNING_PROP_CAMERA_RAW_OVERLAY_OVERSCAN: &str =
    "debug.rustyxr.camera.raw.overlay.overscan";
impl OesActivityConfig {
    pub(super) fn from_activity(app: &android_activity::AndroidApp) -> Self {
        let processing_layer = processing_layer_from_activity(app);
        let blur_radius_px = blur_radius_px_from_activity(app);
        let base_projection_tuning = OesProjectionTuning::from_activity(app);
        let projection_area_offset_x_uv = projection_area_offset_x_uv_from_activity(app);
        let projection_area_offset_y_uv = projection_area_offset_y_uv_from_activity(app);
        let projection_area_offset_uv = [projection_area_offset_x_uv, projection_area_offset_y_uv];
        let projection_state = OesProjectionRuntimeState {
            tuning: base_projection_tuning,
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
        };
        let camera_color_controls = camera_color_controls_from_activity(app);

        Self {
            processing_layer,
            blur_radius_px,
            base_projection_tuning,
            projection_state,
            camera_color_controls,
        }
    }
}

impl OesProjectionTuning {
    fn from_activity(app: &android_activity::AndroidApp) -> Self {
        Self {
            projection_depth_meters: projection_depth_meters_from_activity(app),
            camera_preview_fov_y_degrees: projection_preview_fov_y_degrees_from_activity(app),
            camera_preview_offset_y_meters: projection_preview_offset_y_meters_from_activity(app),
            camera_raw_overlay_overscan: projection_raw_overscan_from_activity(app),
        }
    }

    fn with_system_properties(self) -> Self {
        Self {
            projection_depth_meters: android_system_property_f32(
                OES_TUNING_PROP_PROJECTION_DEPTH_METERS,
                self.projection_depth_meters,
                0.05,
                10.0,
            ),
            camera_preview_fov_y_degrees: android_system_property_f32(
                OES_TUNING_PROP_CAMERA_PREVIEW_FOV_Y_DEGREES,
                self.camera_preview_fov_y_degrees,
                1.0,
                175.0,
            ),
            camera_preview_offset_y_meters: android_system_property_f32(
                OES_TUNING_PROP_CAMERA_PREVIEW_OFFSET_Y_METERS,
                self.camera_preview_offset_y_meters,
                -2.0,
                2.0,
            ),
            camera_raw_overlay_overscan: android_system_property_f32(
                OES_TUNING_PROP_CAMERA_RAW_OVERLAY_OVERSCAN,
                self.camera_raw_overlay_overscan,
                1.0,
                16.0,
            ),
        }
    }
}

impl OesProjectionRuntimeState {
    pub(super) fn with_legacy_system_properties(self) -> Self {
        Self {
            tuning: self.tuning.with_system_properties(),
            ..self
        }
    }
}

fn with_activity_env<R>(
    app: &android_activity::AndroidApp,
    fallback: R,
    read: impl FnOnce(&mut JNIEnv<'_>, &JObject<'_>) -> R,
) -> R {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return fallback;
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return fallback;
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    read(&mut env, &activity)
}

fn projection_border_policy_from_activity(
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

fn processing_layer_from_activity(app: &android_activity::AndroidApp) -> OesProcessingLayer {
    with_activity_env(app, OesProcessingLayer::default(), |env, activity| {
        let requested = activity_string_extra(env, activity, "rustyxr.processingLayer");
        requested
            .as_deref()
            .and_then(OesProcessingLayer::parse)
            .unwrap_or_default()
    })
}

fn camera_projection_mode_from_activity(
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

fn blur_radius_px_from_activity(app: &android_activity::AndroidApp) -> f32 {
    with_activity_env(app, 2.0, |env, activity| {
        activity_string_extra(env, activity, "rustyxr.cameraBlurRadiusPx")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(2.0)
            .clamp(0.0, 16.0)
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

fn activity_float_extra(
    env: &mut JNIEnv<'_>,
    activity: &JObject<'_>,
    keys: &[&str],
) -> Option<f32> {
    keys.iter()
        .find_map(|key| activity_string_extra(env, activity, key))
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
}

fn projection_area_eye_offset_uv_from_activity(
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

fn projection_area_scale_from_activity(app: &android_activity::AndroidApp) -> [f32; 2] {
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

fn projection_area_radius_from_activity(app: &android_activity::AndroidApp) -> [f32; 2] {
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

fn projection_area_corner_radius_uv_from_activity(app: &android_activity::AndroidApp) -> f32 {
    with_activity_env(app, 0.08, |env, activity| {
        activity_string_extra(env, activity, "rustyxr.projectionAreaCornerRadiusUv")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(0.08)
            .clamp(0.0, 0.5)
    })
}

fn projection_area_opacity_from_activity(app: &android_activity::AndroidApp) -> f32 {
    with_activity_env(app, 1.0, |env, activity| {
        activity_string_extra(env, activity, "rustyxr.projectionAreaOpacity")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(1.0)
            .clamp(0.0, 1.0)
    })
}

fn projection_border_opacity_from_activity(app: &android_activity::AndroidApp) -> f32 {
    with_activity_env(app, 1.0, |env, activity| {
        activity_string_extra(env, activity, "rustyxr.projectionBorderOpacity")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(1.0)
            .clamp(0.0, 1.0)
    })
}

fn projection_alpha_mode_from_activity(
    app: &android_activity::AndroidApp,
) -> OesProjectionAlphaMode {
    with_activity_env(app, OesProjectionAlphaMode::default(), |env, activity| {
        activity_string_extra(env, activity, "rustyxr.projectionAlphaMode")
            .as_deref()
            .and_then(OesProjectionAlphaMode::parse)
            .unwrap_or_default()
    })
}

fn projection_alpha_scale_from_activity(app: &android_activity::AndroidApp) -> f32 {
    with_activity_env(app, 1.0, |env, activity| {
        activity_string_extra(env, activity, "rustyxr.projectionAlphaScale")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(1.0)
            .clamp(0.0, 4.0)
    })
}

fn projection_alpha_bias_from_activity(app: &android_activity::AndroidApp) -> f32 {
    with_activity_env(app, 0.0, |env, activity| {
        activity_string_extra(env, activity, "rustyxr.projectionAlphaBias")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
            .clamp(-1.0, 1.0)
    })
}

fn camera_color_controls_from_activity(app: &android_activity::AndroidApp) -> OesColorControls {
    let defaults = OesColorControls::default();
    with_activity_env(app, defaults, |env, activity| {
        let matrix = activity_string_extra(env, activity, "rustyxr.cameraColorMatrix")
            .as_deref()
            .map(parse_color_matrix)
            .unwrap_or(defaults.matrix);
        let offset = activity_string_extra(env, activity, "rustyxr.cameraColorOffset")
            .as_deref()
            .map(parse_color_offset)
            .unwrap_or(defaults.offset);
        let contrast = activity_string_extra(env, activity, "rustyxr.cameraColorContrast")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(defaults.contrast)
            .clamp(0.0, 4.0);
        let brightness = activity_string_extra(env, activity, "rustyxr.cameraColorBrightness")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(defaults.brightness)
            .clamp(-1.0, 1.0);
        let saturation = activity_string_extra(env, activity, "rustyxr.cameraColorSaturation")
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(defaults.saturation)
            .clamp(0.0, 4.0);
        let source_transfer =
            activity_string_extra(env, activity, "rustyxr.oesSourceColorTransfer")
                .as_deref()
                .and_then(OesSourceColorTransfer::parse)
                .unwrap_or(defaults.source_transfer);
        OesColorControls {
            matrix,
            offset,
            contrast,
            brightness,
            saturation,
            source_transfer,
        }
    })
}

fn parse_color_components(value: &str) -> Vec<f32> {
    value
        .split([';', ',', ' '])
        .filter(|item| !item.trim().is_empty())
        .filter_map(|item| item.trim().parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .collect()
}

fn parse_color_matrix(value: &str) -> [[f32; 3]; 3] {
    let values = parse_color_components(value);
    if values.len() != 9 {
        return OesColorControls::default().matrix;
    }
    [
        [values[0], values[1], values[2]],
        [values[3], values[4], values[5]],
        [values[6], values[7], values[8]],
    ]
}

fn parse_color_offset(value: &str) -> [f32; 3] {
    let values = parse_color_components(value);
    if values.len() != 3 {
        return OesColorControls::default().offset;
    }
    [
        values[0].clamp(-1.0, 1.0),
        values[1].clamp(-1.0, 1.0),
        values[2].clamp(-1.0, 1.0),
    ]
}
