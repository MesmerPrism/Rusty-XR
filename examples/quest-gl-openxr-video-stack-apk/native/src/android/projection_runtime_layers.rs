use jni::{objects::JObject, sys::jobject, JavaVM};
use rusty_xr_runtime_config as rxrc;

use crate::current_android_projection_property_config;

use super::openxr_gles_config::{
    activity_string_extra, android_system_property_value, OesCameraProjectionMode,
    OesProjectionAlphaMode, OesProjectionBorderPolicy, OesProjectionRuntimeState,
    OesProjectionTuning, DEFAULT_PROJECTION_TARGET_DEPTH_METERS,
    OES_PROJECTION_RUNTIME_RESOLUTION_ENABLED_EXTRA,
    OES_PROJECTION_RUNTIME_RESOLUTION_ENABLED_PROP, PROJECTION_PREVIEW_FOV_Y_DEGREES,
    PROJECTION_RAW_OVERSCAN,
};

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
