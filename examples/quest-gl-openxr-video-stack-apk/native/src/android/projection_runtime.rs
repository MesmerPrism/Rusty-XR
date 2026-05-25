use super::{
    log_info,
    openxr_gles_config::{
        activity_string_extra, android_system_property_value, OesCameraProjectionMode,
        OesProjectionAlphaMode, OesProjectionBorderPolicy, OesProjectionRuntimeState,
        OesProjectionTuning,
    },
    DEFAULT_PROJECTION_TARGET_DEPTH_METERS, OES_PROJECTION_RUNTIME_RESOLUTION_ENABLED_EXTRA,
    OES_PROJECTION_RUNTIME_RESOLUTION_ENABLED_PROP, PROJECTION_PREVIEW_FOV_Y_DEGREES,
    PROJECTION_RAW_OVERSCAN,
};
use crate::current_android_projection_property_config;
use jni::{objects::JObject, sys::jobject, JavaVM};
use rusty_xr_runtime_config as rxrc;

pub(super) fn log_oes_projection_runtime_manifest(
    phase: &str,
    runtime: &rxrc::ProjectionRuntimeConfigResolution,
    resolved_manifest_consumption_enabled: bool,
) {
    for line in runtime.manifest_marker_lines("oes", phase) {
        log_info(line);
    }
    log_info(format!(
            "RUSTY_XR_OES_PROJECTION_RUNTIME schema=rusty.xr.oes-projection-runtime.v1 phase={} mode={} resolvedManifestConsumptionEnabled={}",
            phase,
            if resolved_manifest_consumption_enabled {
                "resolved-manifest"
            } else {
                "legacy"
            },
            resolved_manifest_consumption_enabled
        ));
}

pub(super) fn oes_projection_tuning_hotload_log_message(
    tuning_source: &str,
    frame_count: u64,
    tuning: OesProjectionTuning,
) -> String {
    format!(
        "Rusty XR OpenXR GLES projection tuning hotload source={} frame={} projectionDepthMeters={:.6} cameraPreviewFovYDegrees={:.6} cameraPreviewOffsetYMeters={:.6} cameraRawOverlayOverscan={:.6} propertyPrefix=debug.rustyxr",
        tuning_source,
        frame_count,
        tuning.projection_depth_meters,
        tuning.camera_preview_fov_y_degrees,
        tuning.camera_preview_offset_y_meters,
        tuning.camera_raw_overlay_overscan
    )
}

pub(super) fn oes_projection_runtime_hotload_log_message(
    tuning_source: &str,
    frame_count: u64,
    projection_state: OesProjectionRuntimeState,
) -> String {
    format!(
        "Rusty XR OpenXR GLES projection runtime hotload source={} frame={} projectionDepthMeters={:.6} cameraPreviewFovYDegrees={:.6} cameraPreviewOffsetYMeters={:.6} cameraRawOverlayOverscan={:.6} projectionAreaOffsetUv={:.6},{:.6} projectionAreaScale={:.6},{:.6} projectionAreaRadiusUv={:.6},{:.6} projectionAreaOpacity={:.3} projectionBorderOpacity={:.3} projectionAlphaMode={} projectionAlphaScale={:.3} projectionAlphaBias={:.3} cameraProjectionMode={} projectionBorderPolicy={} propertyPrefix=debug.rustyxr",
        tuning_source,
        frame_count,
        projection_state.tuning.projection_depth_meters,
        projection_state.tuning.camera_preview_fov_y_degrees,
        projection_state.tuning.camera_preview_offset_y_meters,
        projection_state.tuning.camera_raw_overlay_overscan,
        projection_state.projection_area_offset_uv[0],
        projection_state.projection_area_offset_uv[1],
        projection_state.projection_area_scale[0],
        projection_state.projection_area_scale[1],
        projection_state.projection_area_radius[0],
        projection_state.projection_area_radius[1],
        projection_state.projection_area_opacity,
        projection_state.projection_border_opacity,
        projection_state.projection_alpha_mode.stable_id(),
        projection_state.projection_alpha_scale,
        projection_state.projection_alpha_bias,
        projection_state.camera_projection_mode.stable_id(),
        projection_state.projection_border_policy.stable_id()
    )
}

fn oes_current_android_projection_property_values() -> Vec<(&'static str, String)> {
    rxrc::PROJECTION_RUNTIME_KEY_ALIASES
        .iter()
        .filter(|alias| {
            alias.source == rxrc::RuntimeKeyAliasSource::AndroidProperty
                && alias.status == rxrc::RuntimeKeyAliasStatus::Current
        })
        .filter_map(|alias| {
            android_system_property_value(alias.alias).map(|value| (alias.alias, value))
        })
        .collect()
}

pub(super) fn oes_projection_runtime_resolution_from_state(
    state: OesProjectionRuntimeState,
) -> rxrc::ProjectionRuntimeConfigResolution {
    oes_projection_runtime_resolution(
        state.tuning,
        state.projection_area_offset_uv,
        state.projection_area_eye_offset_uv,
        state.projection_area_scale,
        state.projection_area_radius,
        state.projection_area_corner_radius_uv,
        state.projection_area_opacity,
        state.projection_border_opacity,
        state.projection_alpha_mode,
        state.projection_alpha_scale,
        state.projection_alpha_bias,
        state.camera_projection_mode,
        state.projection_border_policy,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn oes_projection_runtime_resolution(
    base_tuning: OesProjectionTuning,
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
) -> rxrc::ProjectionRuntimeConfigResolution {
    let defaults = oes_projection_runtime_config(
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
        rxrc::RuntimeConfigSource::Default,
    );
    let activity = oes_projection_runtime_config(
        base_tuning,
        projection_area_offset_uv,
        projection_area_eye_offset_uv,
        projection_area_scale,
        projection_area_radius,
        projection_area_corner_radius_uv,
        projection_area_opacity,
        projection_border_opacity,
        projection_alpha_mode,
        projection_alpha_scale,
        projection_alpha_bias,
        camera_projection_mode,
        projection_border_policy,
        rxrc::RuntimeConfigSource::CommandLine,
    );
    let property_values = oes_current_android_projection_property_values();
    let property_parse = current_android_projection_property_config(
        property_values
            .iter()
            .map(|(key, value)| (*key, value.as_str())),
    );

    rxrc::ProjectionRuntimeConfigBuilder::new()
        .with_layer("oes-defaults", 0, defaults)
        .expect("manifest owner should be valid")
        .with_layer("oes-activity-effective", 10, activity)
        .expect("manifest owner should be valid")
        .with_layer("oes-android-properties", 20, property_parse.config)
        .expect("manifest owner should be valid")
        .with_aliases(property_parse.aliases)
        .resolve()
}

pub(super) fn oes_projection_runtime_resolution_enabled(
    app: &android_activity::AndroidApp,
) -> bool {
    if let Some(value) =
        android_system_property_value(OES_PROJECTION_RUNTIME_RESOLUTION_ENABLED_PROP)
            .and_then(|value| oes_projection_runtime_bool(&value))
    {
        return value;
    }
    activity_bool_extra(app, OES_PROJECTION_RUNTIME_RESOLUTION_ENABLED_EXTRA).unwrap_or(false)
}

fn activity_bool_extra(app: &android_activity::AndroidApp, key: &str) -> Option<bool> {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return None;
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return None;
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    activity_string_extra(&mut env, &activity, key)
        .as_deref()
        .and_then(oes_projection_runtime_bool)
}

fn oes_projection_runtime_bool(value: &str) -> Option<bool> {
    rxrc::RuntimeValue::parse_typed(value).as_bool()
}

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
    }
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

#[allow(clippy::too_many_arguments)]
fn oes_projection_runtime_config(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuning_hotload_log_message_keeps_shape() {
        let line = oes_projection_tuning_hotload_log_message(
            "resolved-projection-runtime",
            0,
            OesProjectionTuning {
                projection_depth_meters: 1.25,
                camera_preview_fov_y_degrees: 72.0,
                camera_preview_offset_y_meters: 0.125,
                camera_raw_overlay_overscan: 1.5,
            },
        );

        assert_eq!(
            line,
            "Rusty XR OpenXR GLES projection tuning hotload source=resolved-projection-runtime frame=0 projectionDepthMeters=1.250000 cameraPreviewFovYDegrees=72.000000 cameraPreviewOffsetYMeters=0.125000 cameraRawOverlayOverscan=1.500000 propertyPrefix=debug.rustyxr"
        );
    }

    #[test]
    fn runtime_hotload_log_message_keeps_shape() {
        let line = oes_projection_runtime_hotload_log_message(
            "android-system-property",
            42,
            OesProjectionRuntimeState {
                tuning: OesProjectionTuning {
                    projection_depth_meters: 1.25,
                    camera_preview_fov_y_degrees: 72.0,
                    camera_preview_offset_y_meters: 0.125,
                    camera_raw_overlay_overscan: 1.5,
                },
                projection_area_offset_uv: [0.01, -0.02],
                projection_area_eye_offset_uv: [[0.0, 0.0], [0.0, 0.0]],
                projection_area_scale: [0.95, 0.85],
                projection_area_radius: [0.47, 0.36],
                projection_area_corner_radius_uv: 0.08,
                projection_area_opacity: 0.75,
                projection_border_opacity: 0.5,
                projection_alpha_mode: OesProjectionAlphaMode::Green,
                projection_alpha_scale: 1.25,
                projection_alpha_bias: -0.25,
                camera_projection_mode: OesCameraProjectionMode::WorldCanvas,
                projection_border_policy: OesProjectionBorderPolicy::PassthroughUnderlay,
            },
        );

        assert_eq!(
            line,
            "Rusty XR OpenXR GLES projection runtime hotload source=android-system-property frame=42 projectionDepthMeters=1.250000 cameraPreviewFovYDegrees=72.000000 cameraPreviewOffsetYMeters=0.125000 cameraRawOverlayOverscan=1.500000 projectionAreaOffsetUv=0.010000,-0.020000 projectionAreaScale=0.950000,0.850000 projectionAreaRadiusUv=0.470000,0.360000 projectionAreaOpacity=0.750 projectionBorderOpacity=0.500 projectionAlphaMode=green projectionAlphaScale=1.250 projectionAlphaBias=-0.250 cameraProjectionMode=world-canvas projectionBorderPolicy=passthrough-underlay propertyPrefix=debug.rustyxr"
        );
    }
}
