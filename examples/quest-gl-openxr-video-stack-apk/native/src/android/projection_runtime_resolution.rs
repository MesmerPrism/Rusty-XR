use super::openxr_gles_config::{
    OesCameraProjectionMode, OesPeripheralStretchBlendMode, OesPeripheralStretchConfig,
    OesPeripheralStretchCornerMode, OesPeripheralStretchDebug, OesPeripheralStretchMode,
    OesProcessingLayer, OesProjectionAlphaMode, OesProjectionBorderPolicy,
    OesProjectionRuntimeState, OesProjectionTuning,
};
use rusty_xr_runtime_config as rxrc;

pub(super) use super::projection_runtime_layers::{
    oes_projection_runtime_resolution_enabled, oes_projection_runtime_resolution_from_state,
};

pub(super) fn oes_projection_runtime_state_from_resolution(
    fallback: OesProjectionRuntimeState,
    resolution: &rxrc::RuntimeConfigResolution,
) -> OesProjectionRuntimeState {
    let projection_area_offset_uv =
        oes_projection_area_offset_from_resolution(fallback.projection_area_offset_uv, resolution);
    OesProjectionRuntimeState {
        tuning: oes_projection_tuning_from_resolution(fallback.tuning, resolution),
        projection_area_offset_uv,
        projection_area_eye_offset_uv: oes_projection_area_eye_offset_from_resolution(
            fallback.projection_area_eye_offset_uv,
            projection_area_offset_uv,
            resolution,
        ),
        projection_area_scale: oes_projection_area_scale_from_resolution(
            fallback.projection_area_scale,
            resolution,
        ),
        projection_area_radius: oes_projection_area_radius_from_resolution(
            fallback.projection_area_radius,
            resolution,
        ),
        projection_area_corner_radius_uv: oes_projection_runtime_float(
            resolution,
            rxrc::KEY_PROJECTION_AREA_CORNER_RADIUS_UV,
            fallback.projection_area_corner_radius_uv,
            0.0,
            0.5,
        ),
        projection_area_opacity: oes_projection_runtime_float(
            resolution,
            rxrc::KEY_PROJECTION_AREA_OPACITY,
            fallback.projection_area_opacity,
            0.0,
            1.0,
        ),
        projection_border_opacity: oes_projection_runtime_float(
            resolution,
            rxrc::KEY_PROJECTION_BORDER_OPACITY,
            fallback.projection_border_opacity,
            0.0,
            1.0,
        ),
        projection_alpha_mode: oes_projection_runtime_text(
            resolution,
            rxrc::KEY_PROJECTION_ALPHA_MODE,
        )
        .and_then(OesProjectionAlphaMode::parse)
        .unwrap_or(fallback.projection_alpha_mode),
        projection_alpha_scale: oes_projection_runtime_float(
            resolution,
            rxrc::KEY_PROJECTION_ALPHA_SCALE,
            fallback.projection_alpha_scale,
            0.0,
            4.0,
        ),
        projection_alpha_bias: oes_projection_runtime_float(
            resolution,
            rxrc::KEY_PROJECTION_ALPHA_BIAS,
            fallback.projection_alpha_bias,
            -1.0,
            1.0,
        ),
        camera_projection_mode: oes_projection_runtime_text(
            resolution,
            rxrc::KEY_CAMERA_PROJECTION_MODE,
        )
        .and_then(OesCameraProjectionMode::parse)
        .unwrap_or(fallback.camera_projection_mode),
        projection_border_policy: oes_projection_runtime_text(
            resolution,
            rxrc::KEY_PROJECTION_BORDER_POLICY,
        )
        .and_then(OesProjectionBorderPolicy::parse)
        .unwrap_or(fallback.projection_border_policy),
        processing_layer: oes_projection_runtime_text(resolution, rxrc::KEY_PROCESSING_LAYER)
            .and_then(OesProcessingLayer::parse)
            .unwrap_or(fallback.processing_layer),
        blur_radius_px: oes_projection_runtime_float(
            resolution,
            rxrc::KEY_CAMERA_BLUR_RADIUS_PX,
            fallback.blur_radius_px,
            0.0,
            16.0,
        ),
        peripheral_stretch: oes_peripheral_stretch_from_resolution(
            fallback.peripheral_stretch,
            resolution,
        ),
    }
}

fn oes_peripheral_stretch_from_resolution(
    fallback: OesPeripheralStretchConfig,
    resolution: &rxrc::RuntimeConfigResolution,
) -> OesPeripheralStretchConfig {
    let edge_inset_uv = oes_projection_runtime_float(
        resolution,
        rxrc::KEY_PERIPHERAL_STRETCH_EDGE_INSET_UV,
        fallback.edge_inset_uv,
        0.0,
        0.49,
    );
    OesPeripheralStretchConfig {
        mode: oes_projection_runtime_text(resolution, rxrc::KEY_PERIPHERAL_STRETCH_MODE)
            .and_then(OesPeripheralStretchMode::parse)
            .unwrap_or(fallback.mode),
        core_scale: oes_projection_runtime_float(
            resolution,
            rxrc::KEY_PERIPHERAL_STRETCH_CORE_SCALE,
            fallback.core_scale,
            0.05,
            1.0,
        ),
        edge_inset_uv,
        max_inset_uv: oes_projection_runtime_float(
            resolution,
            rxrc::KEY_PERIPHERAL_STRETCH_MAX_INSET_UV,
            fallback.max_inset_uv,
            edge_inset_uv,
            0.49,
        ),
        curve: oes_projection_runtime_float(
            resolution,
            rxrc::KEY_PERIPHERAL_STRETCH_CURVE,
            fallback.curve,
            0.25,
            6.0,
        ),
        inner_blend_uv: oes_projection_runtime_float(
            resolution,
            rxrc::KEY_PERIPHERAL_STRETCH_INNER_BLEND_UV,
            fallback.inner_blend_uv,
            0.0,
            0.25,
        ),
        blend_curve: oes_projection_runtime_float(
            resolution,
            rxrc::KEY_PERIPHERAL_STRETCH_BLEND_CURVE,
            fallback.blend_curve,
            0.25,
            6.0,
        ),
        blend_mode: oes_projection_runtime_text(
            resolution,
            rxrc::KEY_PERIPHERAL_STRETCH_BLEND_MODE,
        )
        .and_then(OesPeripheralStretchBlendMode::parse)
        .unwrap_or(fallback.blend_mode),
        corner_mode: oes_projection_runtime_text(
            resolution,
            rxrc::KEY_PERIPHERAL_STRETCH_CORNER_MODE,
        )
        .and_then(OesPeripheralStretchCornerMode::parse)
        .unwrap_or(fallback.corner_mode),
        debug: oes_projection_runtime_text(resolution, rxrc::KEY_PERIPHERAL_STRETCH_DEBUG)
            .and_then(OesPeripheralStretchDebug::parse)
            .unwrap_or(fallback.debug),
    }
    .sanitized()
}

fn oes_projection_tuning_from_resolution(
    fallback: OesProjectionTuning,
    resolution: &rxrc::RuntimeConfigResolution,
) -> OesProjectionTuning {
    OesProjectionTuning {
        projection_depth_meters: oes_projection_runtime_float(
            resolution,
            rxrc::KEY_PROJECTION_DEPTH_METERS,
            fallback.projection_depth_meters,
            0.05,
            10.0,
        ),
        camera_preview_fov_y_degrees: oes_projection_runtime_float(
            resolution,
            rxrc::KEY_CAMERA_PREVIEW_FOV_Y_DEGREES,
            fallback.camera_preview_fov_y_degrees,
            1.0,
            175.0,
        ),
        camera_preview_offset_y_meters: oes_projection_runtime_float(
            resolution,
            rxrc::KEY_CAMERA_PREVIEW_OFFSET_Y_METERS,
            fallback.camera_preview_offset_y_meters,
            -2.0,
            2.0,
        ),
        camera_raw_overlay_overscan: oes_projection_runtime_float(
            resolution,
            rxrc::KEY_CAMERA_RAW_OVERLAY_OVERSCAN,
            fallback.camera_raw_overlay_overscan,
            1.0,
            16.0,
        ),
    }
}

fn oes_projection_area_offset_from_resolution(
    fallback: [f32; 2],
    resolution: &rxrc::RuntimeConfigResolution,
) -> [f32; 2] {
    [
        oes_projection_runtime_float(
            resolution,
            rxrc::KEY_PROJECTION_AREA_OFFSET_X_UV,
            fallback[0],
            -0.5,
            0.5,
        ),
        oes_projection_runtime_float(
            resolution,
            rxrc::KEY_PROJECTION_AREA_OFFSET_Y_UV,
            fallback[1],
            -0.5,
            0.5,
        ),
    ]
}

fn oes_projection_area_eye_offset_from_resolution(
    fallback: [[f32; 2]; 2],
    global_offset_uv: [f32; 2],
    resolution: &rxrc::RuntimeConfigResolution,
) -> [[f32; 2]; 2] {
    let resolved = [
        [
            oes_projection_runtime_optional_float(
                resolution,
                rxrc::KEY_PROJECTION_AREA_LEFT_OFFSET_X_UV,
                -0.5,
                0.5,
            )
            .unwrap_or(global_offset_uv[0]),
            oes_projection_runtime_optional_float(
                resolution,
                rxrc::KEY_PROJECTION_AREA_LEFT_OFFSET_Y_UV,
                -0.5,
                0.5,
            )
            .unwrap_or(global_offset_uv[1]),
        ],
        [
            oes_projection_runtime_optional_float(
                resolution,
                rxrc::KEY_PROJECTION_AREA_RIGHT_OFFSET_X_UV,
                -0.5,
                0.5,
            )
            .unwrap_or(global_offset_uv[0]),
            oes_projection_runtime_optional_float(
                resolution,
                rxrc::KEY_PROJECTION_AREA_RIGHT_OFFSET_Y_UV,
                -0.5,
                0.5,
            )
            .unwrap_or(global_offset_uv[1]),
        ],
    ];
    [
        sanitize_projection_eye_offset(resolved[0], fallback[0]),
        sanitize_projection_eye_offset(resolved[1], fallback[1]),
    ]
}

fn sanitize_projection_eye_offset(value: [f32; 2], fallback: [f32; 2]) -> [f32; 2] {
    if value[0].is_finite() && value[1].is_finite() {
        [value[0].clamp(-0.5, 0.5), value[1].clamp(-0.5, 0.5)]
    } else {
        fallback
    }
}

fn oes_projection_area_scale_from_resolution(
    fallback: [f32; 2],
    resolution: &rxrc::RuntimeConfigResolution,
) -> [f32; 2] {
    let uniform_scale = oes_projection_runtime_optional_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_SCALE_UV,
        0.05,
        4.0,
    );
    [
        oes_projection_runtime_optional_float(
            resolution,
            rxrc::KEY_PROJECTION_AREA_SCALE_X,
            0.05,
            4.0,
        )
        .or(uniform_scale)
        .unwrap_or(fallback[0]),
        oes_projection_runtime_optional_float(
            resolution,
            rxrc::KEY_PROJECTION_AREA_SCALE_Y,
            0.05,
            4.0,
        )
        .or(uniform_scale)
        .unwrap_or(fallback[1]),
    ]
}

fn oes_projection_area_radius_from_resolution(
    fallback: [f32; 2],
    resolution: &rxrc::RuntimeConfigResolution,
) -> [f32; 2] {
    [
        oes_projection_runtime_float(
            resolution,
            rxrc::KEY_PROJECTION_AREA_RADIUS_X_UV,
            fallback[0],
            0.05,
            0.5,
        ),
        oes_projection_runtime_float(
            resolution,
            rxrc::KEY_PROJECTION_AREA_RADIUS_Y_UV,
            fallback[1],
            0.05,
            0.5,
        ),
    ]
}

fn oes_projection_runtime_float(
    resolution: &rxrc::RuntimeConfigResolution,
    key: &str,
    fallback: f32,
    min: f32,
    max: f32,
) -> f32 {
    oes_projection_runtime_optional_float(resolution, key, min, max).unwrap_or(fallback)
}

fn oes_projection_runtime_optional_float(
    resolution: &rxrc::RuntimeConfigResolution,
    key: &str,
    min: f32,
    max: f32,
) -> Option<f32> {
    resolution
        .resolved()
        .get(key)
        .and_then(rxrc::RuntimeValue::as_float)
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(f64::from(min), f64::from(max)) as f32)
}

fn oes_projection_runtime_text<'a>(
    resolution: &'a rxrc::RuntimeConfigResolution,
    key: &str,
) -> Option<&'a str> {
    resolution
        .resolved()
        .get(key)
        .and_then(rxrc::RuntimeValue::as_text)
}
