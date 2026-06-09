#[cfg(target_os = "android")]
use super::log_info;
use super::{
    CameraImageRotation, CameraPeripheralStretchBlendMode, CameraPeripheralStretchCornerMode,
    CameraPeripheralStretchDebug, CameraPeripheralStretchMode, CameraProcessingLayer,
    CameraProjectionAlphaMode, CameraProjectionBorderPolicy, CameraProjectionMode,
    ProjectionTargetJoystickControls, RuntimeConfig, StereoSourceEyeMapping,
};
use rusty_quest_projection_runtime_config as rxrc;

#[cfg(target_os = "android")]
pub(super) fn log_projection_runtime_manifest(
    phase: &str,
    config: &RuntimeConfig,
    include_effective: bool,
) {
    let runtime = hwb_projection_runtime_resolution(config, include_effective);
    for line in runtime.manifest_marker_lines("hwb", phase) {
        log_info(line);
    }
    log_info(format!(
        "RUSTY_QUEST_HWB_PROJECTION_RUNTIME schema=rusty.quest.hwb-projection-runtime.v1 phase={} mode={} resolvedManifestConsumptionEnabled={}",
        phase,
        if config.projection_runtime_resolution_enabled {
            "resolved-manifest"
        } else {
            "direct"
        },
        config.projection_runtime_resolution_enabled
    ));
}

pub(super) fn hwb_projection_runtime_resolution(
    config: &RuntimeConfig,
    include_effective: bool,
) -> rxrc::ProjectionRuntimeConfigResolution {
    let defaults = public_projection_runtime_config(
        &RuntimeConfig::default(),
        rxrc::RuntimeConfigSource::Default,
    );
    let (property_config, inputs) = hwb_projection_runtime_android_property_config();
    let mut builder = rxrc::ProjectionRuntimeConfigBuilder::new();
    builder
        .push_layer("hwb-defaults", 0, defaults)
        .expect("manifest owner should be valid");
    if include_effective {
        builder
            .push_layer(
                "hwb-launch-effective",
                10,
                public_projection_runtime_config(config, rxrc::RuntimeConfigSource::CommandLine),
            )
            .expect("manifest owner should be valid");
    }
    builder
        .push_layer("hwb-android-properties", 20, property_config)
        .expect("manifest owner should be valid");
    builder.with_inputs(inputs).resolve()
}

pub(super) fn public_projection_runtime_config(
    config: &RuntimeConfig,
    source: rxrc::RuntimeConfigSource,
) -> rxrc::RuntimeConfig {
    let mut public = rxrc::RuntimeConfig::new();
    set_public_text(
        &mut public,
        rxrc::KEY_CAMERA_PROJECTION_MODE,
        config.camera_projection_mode.stable_id(),
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_CAMERA_PROJECTION_FOV_Y_DEGREES,
        config.camera_projection_fov_y_degrees,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_CAMERA_PREVIEW_FOV_Y_DEGREES,
        config.camera_preview_fov_y_degrees,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_CAMERA_PREVIEW_OFFSET_Y_METERS,
        config.camera_preview_offset_y_meters,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PROJECTION_SCALE,
        config.camera_projection_scale,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PROJECTION_DEPTH_METERS,
        config.camera_projection_depth_meters,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PROJECTION_AREA_SCALE_UV,
        config.camera_projection_area_scale_uv,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PROJECTION_AREA_OFFSET_X_UV,
        config.camera_projection_area_offset_x_uv,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PROJECTION_AREA_OFFSET_Y_UV,
        config.camera_projection_area_offset_y_uv,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PROJECTION_AREA_LEFT_OFFSET_X_UV,
        config.camera_projection_area_left_offset_x_uv,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PROJECTION_AREA_LEFT_OFFSET_Y_UV,
        config.camera_projection_area_left_offset_y_uv,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PROJECTION_AREA_RIGHT_OFFSET_X_UV,
        config.camera_projection_area_right_offset_x_uv,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PROJECTION_AREA_RIGHT_OFFSET_Y_UV,
        config.camera_projection_area_right_offset_y_uv,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PROJECTION_AREA_RADIUS_X_UV,
        config.camera_projection_area_radius_x_uv,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PROJECTION_AREA_RADIUS_Y_UV,
        config.camera_projection_area_radius_y_uv,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PROJECTION_AREA_CORNER_RADIUS_UV,
        config.camera_projection_area_corner_radius_uv,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PROJECTION_AREA_OPACITY,
        config.camera_projection_area_opacity,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PROJECTION_BORDER_OPACITY,
        config.camera_projection_border_opacity,
        source.clone(),
    );
    set_public_text(
        &mut public,
        rxrc::KEY_PROJECTION_BORDER_POLICY,
        config.camera_projection_border_policy.stable_id(),
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PROJECTION_TARGET_OFFSET_X_UV,
        config.projection_target_offset_x_uv,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PROJECTION_TARGET_OFFSET_Y_UV,
        config.projection_target_offset_y_uv,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PROJECTION_TARGET_SCALE,
        config.projection_target_scale,
        source.clone(),
    );
    set_public_text(
        &mut public,
        rxrc::KEY_PROJECTION_TARGET_JOYSTICK_CONTROLS,
        config.projection_target_joystick_controls.stable_id(),
        source.clone(),
    );
    set_public_text(
        &mut public,
        rxrc::KEY_PROCESSING_LAYER,
        config.camera_processing_layer.stable_id(),
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_CAMERA_BLUR_RADIUS_PX,
        config.camera_blur_radius_px,
        source.clone(),
    );
    let peripheral_stretch = config.camera_peripheral_stretch.sanitized();
    set_public_text(
        &mut public,
        rxrc::KEY_PERIPHERAL_STRETCH_MODE,
        peripheral_stretch.mode.stable_id(),
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PERIPHERAL_STRETCH_CORE_SCALE,
        peripheral_stretch.core_scale,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PERIPHERAL_STRETCH_EDGE_INSET_UV,
        peripheral_stretch.edge_inset_uv,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PERIPHERAL_STRETCH_MAX_INSET_UV,
        peripheral_stretch.max_inset_uv,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PERIPHERAL_STRETCH_CURVE,
        peripheral_stretch.curve,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PERIPHERAL_STRETCH_INNER_BLEND_UV,
        peripheral_stretch.inner_blend_uv,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PERIPHERAL_STRETCH_BLEND_CURVE,
        peripheral_stretch.blend_curve,
        source.clone(),
    );
    set_public_text(
        &mut public,
        rxrc::KEY_PERIPHERAL_STRETCH_BLEND_MODE,
        peripheral_stretch.blend_mode.stable_id(),
        source.clone(),
    );
    set_public_text(
        &mut public,
        rxrc::KEY_PERIPHERAL_STRETCH_CORNER_MODE,
        peripheral_stretch.corner_mode.stable_id(),
        source.clone(),
    );
    set_public_text(
        &mut public,
        rxrc::KEY_PERIPHERAL_STRETCH_DEBUG,
        peripheral_stretch.debug.stable_id(),
        source.clone(),
    );
    set_public_text(
        &mut public,
        rxrc::KEY_PROJECTION_ALPHA_MODE,
        config.camera_projection_alpha_mode.stable_id(),
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PROJECTION_ALPHA_SCALE,
        config.camera_projection_alpha_scale,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_PROJECTION_ALPHA_BIAS,
        config.camera_projection_alpha_bias,
        source.clone(),
    );
    set_public_float(
        &mut public,
        rxrc::KEY_CAMERA_RAW_OVERLAY_OVERSCAN,
        config.camera_raw_overlay_overscan,
        source.clone(),
    );
    set_public_text(
        &mut public,
        rxrc::KEY_SOURCE_EYE_MAPPING,
        config.source_eye_mapping.stable_id(),
        source.clone(),
    );
    set_public_text(
        &mut public,
        rxrc::KEY_SOURCE_TEXTURE_ROTATION,
        config.camera_texture_transform.rotation.stable_id(),
        source.clone(),
    );
    set_public_bool(
        &mut public,
        rxrc::KEY_SOURCE_TEXTURE_FLIP_X,
        config.camera_texture_transform.flip_x,
        source.clone(),
    );
    set_public_bool(
        &mut public,
        rxrc::KEY_SOURCE_TEXTURE_FLIP_Y,
        config.camera_texture_transform.flip_y,
        source.clone(),
    );
    set_public_bool(
        &mut public,
        rxrc::KEY_SOURCE_TEXTURE_MIRROR,
        config.camera_texture_transform.mirror,
        source.clone(),
    );
    set_public_text(
        &mut public,
        rxrc::KEY_SOURCE_TEXTURE_TRANSFORM_SOURCE,
        config.camera_texture_transform.source_label.as_str(),
        source.clone(),
    );
    set_public_text(
        &mut public,
        rxrc::KEY_LEFT_SOURCE_TEXTURE_TRANSFORM_SOURCE,
        config.left_camera_texture_transform.source_label.as_str(),
        source.clone(),
    );
    set_public_text(
        &mut public,
        rxrc::KEY_RIGHT_SOURCE_TEXTURE_TRANSFORM_SOURCE,
        config.right_camera_texture_transform.source_label.as_str(),
        source.clone(),
    );
    set_public_text(
        &mut public,
        rxrc::KEY_SOURCE_TEXTURE_TRANSFORM_REASON,
        config.camera_texture_transform.reason.as_str(),
        source,
    );
    public
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

fn set_public_bool(
    config: &mut rxrc::RuntimeConfig,
    key: &'static str,
    value: bool,
    source: rxrc::RuntimeConfigSource,
) {
    config
        .set(key, rxrc::RuntimeValue::Bool(value), source)
        .expect("projection manifest keys should be public-safe");
}

pub(super) fn apply_hwb_projection_runtime_resolution(
    config: &mut RuntimeConfig,
    resolution: &rxrc::RuntimeConfigResolution,
) {
    config.camera_projection_mode =
        hwb_projection_runtime_text(resolution, rxrc::KEY_CAMERA_PROJECTION_MODE)
            .and_then(CameraProjectionMode::parse)
            .unwrap_or(config.camera_projection_mode);
    config.camera_projection_fov_y_degrees = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_CAMERA_PROJECTION_FOV_Y_DEGREES,
        config.camera_projection_fov_y_degrees,
        1.0,
        175.0,
    );
    config.camera_preview_fov_y_degrees = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_CAMERA_PREVIEW_FOV_Y_DEGREES,
        config.camera_preview_fov_y_degrees,
        1.0,
        175.0,
    );
    config.camera_preview_offset_y_meters = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_CAMERA_PREVIEW_OFFSET_Y_METERS,
        config.camera_preview_offset_y_meters,
        -2.0,
        2.0,
    );
    config.camera_projection_scale = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_SCALE,
        config.camera_projection_scale,
        0.05,
        4.0,
    );
    config.camera_projection_depth_meters = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_DEPTH_METERS,
        config.camera_projection_depth_meters,
        0.05,
        10.0,
    );
    config.camera_projection_area_scale_uv = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_SCALE_UV,
        config.camera_projection_area_scale_uv,
        0.05,
        4.0,
    );

    let global_offset_x = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_OFFSET_X_UV,
        config.camera_projection_area_offset_x_uv,
        -0.5,
        0.5,
    );
    let global_offset_y = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_OFFSET_Y_UV,
        config.camera_projection_area_offset_y_uv,
        -0.5,
        0.5,
    );
    config.camera_projection_area_offset_x_uv = global_offset_x;
    config.camera_projection_area_offset_y_uv = global_offset_y;
    config.camera_projection_area_left_offset_x_uv = hwb_projection_runtime_eye_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_LEFT_OFFSET_X_UV,
        rxrc::KEY_PROJECTION_AREA_OFFSET_X_UV,
        global_offset_x,
        config.camera_projection_area_left_offset_x_uv,
        -0.5,
        0.5,
    );
    config.camera_projection_area_left_offset_y_uv = hwb_projection_runtime_eye_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_LEFT_OFFSET_Y_UV,
        rxrc::KEY_PROJECTION_AREA_OFFSET_Y_UV,
        global_offset_y,
        config.camera_projection_area_left_offset_y_uv,
        -0.5,
        0.5,
    );
    config.camera_projection_area_right_offset_x_uv = hwb_projection_runtime_eye_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_RIGHT_OFFSET_X_UV,
        rxrc::KEY_PROJECTION_AREA_OFFSET_X_UV,
        global_offset_x,
        config.camera_projection_area_right_offset_x_uv,
        -0.5,
        0.5,
    );
    config.camera_projection_area_right_offset_y_uv = hwb_projection_runtime_eye_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_RIGHT_OFFSET_Y_UV,
        rxrc::KEY_PROJECTION_AREA_OFFSET_Y_UV,
        global_offset_y,
        config.camera_projection_area_right_offset_y_uv,
        -0.5,
        0.5,
    );

    config.camera_projection_area_radius_x_uv = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_RADIUS_X_UV,
        config.camera_projection_area_radius_x_uv,
        0.05,
        0.5,
    );
    config.camera_projection_area_radius_y_uv = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_RADIUS_Y_UV,
        config.camera_projection_area_radius_y_uv,
        0.05,
        0.5,
    );
    config.camera_projection_area_corner_radius_uv = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_CORNER_RADIUS_UV,
        config.camera_projection_area_corner_radius_uv,
        0.0,
        0.5,
    );
    config.camera_projection_area_opacity = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_OPACITY,
        config.camera_projection_area_opacity,
        0.0,
        1.0,
    );
    config.camera_projection_border_opacity = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_BORDER_OPACITY,
        config.camera_projection_border_opacity,
        0.0,
        1.0,
    );
    config.camera_projection_border_policy =
        hwb_projection_runtime_text(resolution, rxrc::KEY_PROJECTION_BORDER_POLICY)
            .and_then(CameraProjectionBorderPolicy::parse)
            .unwrap_or(config.camera_projection_border_policy);
    config.projection_target_offset_x_uv = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_TARGET_OFFSET_X_UV,
        config.projection_target_offset_x_uv,
        -0.5,
        0.5,
    );
    config.projection_target_offset_y_uv = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_TARGET_OFFSET_Y_UV,
        config.projection_target_offset_y_uv,
        -0.5,
        0.5,
    );
    config.projection_target_scale = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_TARGET_SCALE,
        config.projection_target_scale,
        0.05,
        1.5,
    );
    config.projection_target_joystick_controls =
        hwb_projection_runtime_text(resolution, rxrc::KEY_PROJECTION_TARGET_JOYSTICK_CONTROLS)
            .and_then(ProjectionTargetJoystickControls::parse)
            .unwrap_or(config.projection_target_joystick_controls);
    config.camera_processing_layer =
        hwb_projection_runtime_text(resolution, rxrc::KEY_PROCESSING_LAYER)
            .and_then(CameraProcessingLayer::parse)
            .unwrap_or(config.camera_processing_layer);
    config.camera_blur_radius_px = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_CAMERA_BLUR_RADIUS_PX,
        config.camera_blur_radius_px,
        0.0,
        16.0,
    );
    config.camera_peripheral_stretch.mode =
        hwb_projection_runtime_text(resolution, rxrc::KEY_PERIPHERAL_STRETCH_MODE)
            .and_then(CameraPeripheralStretchMode::parse)
            .unwrap_or(config.camera_peripheral_stretch.mode);
    config.camera_peripheral_stretch.core_scale = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PERIPHERAL_STRETCH_CORE_SCALE,
        config.camera_peripheral_stretch.core_scale,
        0.05,
        1.0,
    );
    config.camera_peripheral_stretch.edge_inset_uv = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PERIPHERAL_STRETCH_EDGE_INSET_UV,
        config.camera_peripheral_stretch.edge_inset_uv,
        0.0,
        0.49,
    );
    config.camera_peripheral_stretch.max_inset_uv = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PERIPHERAL_STRETCH_MAX_INSET_UV,
        config
            .camera_peripheral_stretch
            .max_inset_uv
            .max(config.camera_peripheral_stretch.edge_inset_uv),
        config.camera_peripheral_stretch.edge_inset_uv,
        0.49,
    );
    config.camera_peripheral_stretch.curve = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PERIPHERAL_STRETCH_CURVE,
        config.camera_peripheral_stretch.curve,
        0.25,
        6.0,
    );
    config.camera_peripheral_stretch.inner_blend_uv = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PERIPHERAL_STRETCH_INNER_BLEND_UV,
        config.camera_peripheral_stretch.inner_blend_uv,
        0.0,
        0.25,
    );
    config.camera_peripheral_stretch.blend_curve = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PERIPHERAL_STRETCH_BLEND_CURVE,
        config.camera_peripheral_stretch.blend_curve,
        0.25,
        6.0,
    );
    config.camera_peripheral_stretch.blend_mode =
        hwb_projection_runtime_text(resolution, rxrc::KEY_PERIPHERAL_STRETCH_BLEND_MODE)
            .and_then(CameraPeripheralStretchBlendMode::parse)
            .unwrap_or(config.camera_peripheral_stretch.blend_mode);
    config.camera_peripheral_stretch.corner_mode =
        hwb_projection_runtime_text(resolution, rxrc::KEY_PERIPHERAL_STRETCH_CORNER_MODE)
            .and_then(CameraPeripheralStretchCornerMode::parse)
            .unwrap_or(config.camera_peripheral_stretch.corner_mode);
    config.camera_peripheral_stretch.debug =
        hwb_projection_runtime_text(resolution, rxrc::KEY_PERIPHERAL_STRETCH_DEBUG)
            .and_then(CameraPeripheralStretchDebug::parse)
            .unwrap_or(config.camera_peripheral_stretch.debug);
    config.camera_peripheral_stretch = config.camera_peripheral_stretch.sanitized();
    config.camera_projection_alpha_mode =
        hwb_projection_runtime_text(resolution, rxrc::KEY_PROJECTION_ALPHA_MODE)
            .and_then(CameraProjectionAlphaMode::parse)
            .unwrap_or(config.camera_projection_alpha_mode);
    config.camera_projection_alpha_scale = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_ALPHA_SCALE,
        config.camera_projection_alpha_scale,
        0.0,
        4.0,
    );
    config.camera_projection_alpha_bias = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_ALPHA_BIAS,
        config.camera_projection_alpha_bias,
        -1.0,
        1.0,
    );
    config.camera_raw_overlay_overscan = hwb_projection_runtime_float(
        resolution,
        rxrc::KEY_CAMERA_RAW_OVERLAY_OVERSCAN,
        config.camera_raw_overlay_overscan,
        1.0,
        16.0,
    );
    config.source_eye_mapping =
        hwb_projection_runtime_text(resolution, rxrc::KEY_SOURCE_EYE_MAPPING)
            .and_then(StereoSourceEyeMapping::parse)
            .unwrap_or(config.source_eye_mapping);

    if let Some(rotation) =
        hwb_projection_runtime_text(resolution, rxrc::KEY_SOURCE_TEXTURE_ROTATION)
            .and_then(CameraImageRotation::parse)
    {
        config.camera_texture_transform.rotation = rotation;
    }
    if let Some(value) = hwb_projection_runtime_bool(resolution, rxrc::KEY_SOURCE_TEXTURE_FLIP_X) {
        config.camera_texture_transform.flip_x = value;
    }
    if let Some(value) = hwb_projection_runtime_bool(resolution, rxrc::KEY_SOURCE_TEXTURE_FLIP_Y) {
        config.camera_texture_transform.flip_y = value;
    }
    if let Some(value) = hwb_projection_runtime_bool(resolution, rxrc::KEY_SOURCE_TEXTURE_MIRROR) {
        config.camera_texture_transform.mirror = value;
    }
    if let Some(value) =
        hwb_projection_runtime_text(resolution, rxrc::KEY_SOURCE_TEXTURE_TRANSFORM_SOURCE)
    {
        config.camera_texture_transform.source_label = value.to_string();
    }
    if let Some(value) =
        hwb_projection_runtime_text(resolution, rxrc::KEY_LEFT_SOURCE_TEXTURE_TRANSFORM_SOURCE)
    {
        config.left_camera_texture_transform.source_label = value.to_string();
    }
    if let Some(value) =
        hwb_projection_runtime_text(resolution, rxrc::KEY_RIGHT_SOURCE_TEXTURE_TRANSFORM_SOURCE)
    {
        config.right_camera_texture_transform.source_label = value.to_string();
    }
    if let Some(value) =
        hwb_projection_runtime_text(resolution, rxrc::KEY_SOURCE_TEXTURE_TRANSFORM_REASON)
    {
        config.camera_texture_transform.reason = value.to_string();
    }
}

fn hwb_projection_runtime_android_property_config(
) -> (rxrc::RuntimeConfig, Vec<rxrc::RuntimeKeyInputRecord>) {
    let values = rxrc::PROJECTION_RUNTIME_KEY_INPUTS
        .iter()
        .filter(|input| input.source == rxrc::RuntimeKeyInputSource::AndroidProperty)
        .filter_map(|input| {
            hwb_android_system_property_value(input.input).map(|value| (input.input, value))
        })
        .collect::<Vec<_>>();
    let mut config = rxrc::RuntimeConfig::new();
    let mut inputs = Vec::new();
    for (key, value) in values {
        let Ok(parsed) = rxrc::parse_projection_runtime_pairs(
            rxrc::RuntimeConfigSource::AndroidProperty,
            [(key, value.as_str())],
        ) else {
            continue;
        };
        for setting in parsed.config.iter() {
            config.insert(setting.clone());
        }
        inputs.extend(parsed.inputs);
    }
    (config, inputs)
}

#[cfg(target_os = "android")]
fn hwb_android_system_property_value(name: &str) -> Option<String> {
    use std::{
        ffi::{CStr, CString},
        os::raw::{c_char, c_int},
    };

    #[link(name = "c")]
    unsafe extern "C" {
        fn __system_property_get(name: *const c_char, value: *mut c_char) -> c_int;
    }

    let name = CString::new(name).ok()?;
    let mut value = [0 as c_char; 128];
    let len = unsafe { __system_property_get(name.as_ptr(), value.as_mut_ptr()) };
    if len <= 0 {
        return None;
    }
    let value = unsafe { CStr::from_ptr(value.as_ptr()) }
        .to_string_lossy()
        .trim()
        .to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(not(target_os = "android"))]
fn hwb_android_system_property_value(_name: &str) -> Option<String> {
    None
}

fn hwb_projection_runtime_float(
    resolution: &rxrc::RuntimeConfigResolution,
    key: &str,
    fallback: f32,
    min: f32,
    max: f32,
) -> f32 {
    hwb_projection_runtime_optional_float(resolution, key, min, max).unwrap_or(fallback)
}

fn hwb_projection_runtime_eye_float(
    resolution: &rxrc::RuntimeConfigResolution,
    eye_key: &str,
    global_key: &str,
    global_value: f32,
    fallback: f32,
    min: f32,
    max: f32,
) -> f32 {
    let eye = hwb_projection_runtime_optional_float(resolution, eye_key, min, max);
    let eye_precedence = resolution.get(eye_key).map(|setting| setting.precedence);
    let global_precedence = resolution
        .get(global_key)
        .map(|setting| setting.precedence)
        .unwrap_or(0);
    if eye_precedence.is_some_and(|precedence| precedence >= global_precedence) {
        eye.unwrap_or(fallback)
    } else {
        global_value
    }
}

fn hwb_projection_runtime_optional_float(
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

fn hwb_projection_runtime_bool(
    resolution: &rxrc::RuntimeConfigResolution,
    key: &str,
) -> Option<bool> {
    resolution
        .resolved()
        .get(key)
        .and_then(rxrc::RuntimeValue::as_bool)
}

fn hwb_projection_runtime_text<'a>(
    resolution: &'a rxrc::RuntimeConfigResolution,
    key: &str,
) -> Option<&'a str> {
    resolution
        .resolved()
        .get(key)
        .and_then(rxrc::RuntimeValue::as_text)
}
