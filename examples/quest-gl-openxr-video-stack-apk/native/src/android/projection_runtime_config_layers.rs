use rusty_xr_runtime_config as rxrc;

use super::openxr_gles_config::{
    OesCameraProjectionMode, OesPeripheralStretchConfig, OesProcessingLayer,
    OesProjectionAlphaMode, OesProjectionBorderPolicy, OesProjectionTuning,
    DEFAULT_PROJECTION_TARGET_DEPTH_METERS, PROJECTION_PREVIEW_FOV_Y_DEGREES,
    PROJECTION_RAW_OVERSCAN,
};

pub(super) fn oes_projection_runtime_default_config() -> rxrc::RuntimeConfig {
    oes_projection_runtime_config(
        OesProjectionTuning {
            projection_depth_meters: DEFAULT_PROJECTION_TARGET_DEPTH_METERS,
            camera_preview_fov_y_degrees: PROJECTION_PREVIEW_FOV_Y_DEGREES,
            camera_preview_offset_y_meters: 0.0,
            camera_raw_overlay_overscan: PROJECTION_RAW_OVERSCAN,
        },
        [0.0, 0.0],
        [[0.0, 0.0], [0.0, 0.0]],
        [1.0, 1.0],
        [0.47, 0.36],
        0.08,
        1.0,
        1.0,
        OesProjectionAlphaMode::default(),
        1.0,
        0.0,
        OesCameraProjectionMode::default(),
        OesProjectionBorderPolicy::default(),
        OesProcessingLayer::default(),
        2.0,
        OesPeripheralStretchConfig::default(),
        rxrc::RuntimeConfigSource::Default,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn oes_projection_runtime_config(
    tuning: OesProjectionTuning,
    projection_area_offset_uv: [f32; 2],
    projection_area_eye_offset_uv: [[f32; 2]; 2],
    projection_area_scale: [f32; 2],
    projection_area_radius: [f32; 2],
    projection_area_corner_radius_uv: f32,
    projection_area_opacity: f32,
    projection_border_opacity: f32,
    projection_alpha_mode: OesProjectionAlphaMode,
    projection_alpha_scale: f32,
    projection_alpha_bias: f32,
    camera_projection_mode: OesCameraProjectionMode,
    projection_border_policy: OesProjectionBorderPolicy,
    processing_layer: OesProcessingLayer,
    blur_radius_px: f32,
    peripheral_stretch: OesPeripheralStretchConfig,
    source: rxrc::RuntimeConfigSource,
) -> rxrc::RuntimeConfig {
    let mut config = rxrc::RuntimeConfig::new();
    set_public_text(
        &mut config,
        rxrc::KEY_CAMERA_PROJECTION_MODE,
        camera_projection_mode.stable_id(),
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_PROJECTION_DEPTH_METERS,
        tuning.projection_depth_meters,
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_CAMERA_PREVIEW_FOV_Y_DEGREES,
        tuning.camera_preview_fov_y_degrees,
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_CAMERA_PREVIEW_OFFSET_Y_METERS,
        tuning.camera_preview_offset_y_meters,
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_CAMERA_RAW_OVERLAY_OVERSCAN,
        tuning.camera_raw_overlay_overscan,
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_OFFSET_X_UV,
        projection_area_offset_uv[0],
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_OFFSET_Y_UV,
        projection_area_offset_uv[1],
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_LEFT_OFFSET_X_UV,
        projection_area_eye_offset_uv[0][0],
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_LEFT_OFFSET_Y_UV,
        projection_area_eye_offset_uv[0][1],
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_RIGHT_OFFSET_X_UV,
        projection_area_eye_offset_uv[1][0],
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_RIGHT_OFFSET_Y_UV,
        projection_area_eye_offset_uv[1][1],
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_SCALE_X,
        projection_area_scale[0],
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_SCALE_Y,
        projection_area_scale[1],
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_RADIUS_X_UV,
        projection_area_radius[0],
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_RADIUS_Y_UV,
        projection_area_radius[1],
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_CORNER_RADIUS_UV,
        projection_area_corner_radius_uv,
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_OPACITY,
        projection_area_opacity,
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_PROJECTION_BORDER_OPACITY,
        projection_border_opacity,
        source.clone(),
    );
    set_public_text(
        &mut config,
        rxrc::KEY_PROJECTION_BORDER_POLICY,
        projection_border_policy.stable_id(),
        source.clone(),
    );
    set_public_text(
        &mut config,
        rxrc::KEY_PROCESSING_LAYER,
        processing_layer.stable_id(),
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_CAMERA_BLUR_RADIUS_PX,
        blur_radius_px,
        source.clone(),
    );
    let peripheral_stretch = peripheral_stretch.sanitized();
    set_public_text(
        &mut config,
        rxrc::KEY_PERIPHERAL_STRETCH_MODE,
        peripheral_stretch.mode.stable_id(),
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_PERIPHERAL_STRETCH_CORE_SCALE,
        peripheral_stretch.core_scale,
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_PERIPHERAL_STRETCH_EDGE_INSET_UV,
        peripheral_stretch.edge_inset_uv,
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_PERIPHERAL_STRETCH_MAX_INSET_UV,
        peripheral_stretch.max_inset_uv,
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_PERIPHERAL_STRETCH_CURVE,
        peripheral_stretch.curve,
        source.clone(),
    );
    set_public_text(
        &mut config,
        rxrc::KEY_PERIPHERAL_STRETCH_CORNER_MODE,
        peripheral_stretch.corner_mode.stable_id(),
        source.clone(),
    );
    set_public_text(
        &mut config,
        rxrc::KEY_PERIPHERAL_STRETCH_DEBUG,
        peripheral_stretch.debug.stable_id(),
        source.clone(),
    );
    set_public_text(
        &mut config,
        rxrc::KEY_PROJECTION_ALPHA_MODE,
        projection_alpha_mode.stable_id(),
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_PROJECTION_ALPHA_SCALE,
        projection_alpha_scale,
        source.clone(),
    );
    set_public_float(
        &mut config,
        rxrc::KEY_PROJECTION_ALPHA_BIAS,
        projection_alpha_bias,
        source,
    );
    config
}

fn set_public_text(
    config: &mut rxrc::RuntimeConfig,
    key: &'static str,
    value: &str,
    source: rxrc::RuntimeConfigSource,
) {
    config
        .set(key, rxrc::RuntimeValue::Text(value.to_string()), source)
        .expect("projection manifest keys should be public-safe");
}

fn set_public_float(
    config: &mut rxrc::RuntimeConfig,
    key: &'static str,
    value: f32,
    source: rxrc::RuntimeConfigSource,
) {
    config
        .set(key, rxrc::RuntimeValue::Float(f64::from(value)), source)
        .expect("projection manifest keys should be public-safe");
}
