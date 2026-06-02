use super::*;
use rusty_xr_runtime_config as rxrc;

pub(super) fn makepad_projection_runtime_resolution_enabled() -> bool {
    hotload_bool(KEY_MAKEPAD_PROJECTION_RUNTIME_RESOLUTION_ENABLED, false)
}

pub(super) fn makepad_projection_runtime_mode_token() -> &'static str {
    if makepad_projection_runtime_resolution_enabled() {
        "resolved-manifest"
    } else {
        "legacy"
    }
}

pub(super) fn makepad_projection_runtime_manifest_lines(
    phase: &str,
    config: &rxrc::RuntimeConfig,
    tuning: HorizontalAlignmentTuning,
) -> Vec<String> {
    let runtime = makepad_projection_runtime_resolution(config, tuning);
    let mut lines = runtime.manifest_marker_lines("makepad", phase);
    lines.push(format!(
        "RUSTY_XR_MAKEPAD_PROJECTION_RUNTIME schema=rusty.xr.makepad-projection-runtime.v1 phase={} mode={} resolvedManifestConsumptionEnabled={}",
        marker_token(phase),
        makepad_projection_runtime_mode_token(),
        makepad_projection_runtime_resolution_enabled()
    ));
    lines
}

pub(super) fn makepad_projection_runtime_resolution(
    config: &rxrc::RuntimeConfig,
    tuning: HorizontalAlignmentTuning,
) -> rxrc::ProjectionRuntimeConfigResolution {
    let mut builder = rxrc::ProjectionRuntimeConfigBuilder::new();
    builder
        .push_layer(
            "makepad-defaults",
            0,
            makepad_projection_runtime_config_defaults(),
        )
        .expect("manifest owner should be valid");
    builder
        .push_layer(
            "makepad-effective",
            10,
            makepad_projection_runtime_config_effective(config, tuning),
        )
        .expect("manifest owner should be valid");

    let env_config = makepad_projection_runtime_env_config();
    builder
        .push_layer("makepad-env", 20, env_config)
        .expect("manifest owner should be valid");

    let current_property_config =
        makepad_projection_runtime_android_property_config(rxrc::RuntimeKeyAliasStatus::Current);
    builder
        .push_layer("makepad-android-properties", 30, current_property_config)
        .expect("manifest owner should be valid");

    builder
        .with_aliases(makepad_projection_alias_records())
        .resolve()
}

fn makepad_projection_runtime_env_config() -> rxrc::RuntimeConfig {
    let values = rxrc::PROJECTION_RUNTIME_KEY_ALIASES
        .iter()
        .filter(|alias| alias.source == rxrc::RuntimeKeyAliasSource::EnvironmentVariable)
        .filter_map(|alias| {
            std::env::var(alias.alias)
                .ok()
                .map(|value| (alias.alias, value))
        })
        .collect::<Vec<_>>();
    makepad_projection_runtime_alias_config(rxrc::RuntimeConfigSource::Environment, values)
}

fn makepad_projection_runtime_android_property_config(
    status: rxrc::RuntimeKeyAliasStatus,
) -> rxrc::RuntimeConfig {
    let values = rxrc::PROJECTION_RUNTIME_KEY_ALIASES
        .iter()
        .filter(|alias| {
            alias.source == rxrc::RuntimeKeyAliasSource::AndroidProperty && alias.status == status
        })
        .filter_map(|alias| {
            android_system_property_value(alias.alias).map(|value| (alias.alias, value))
        })
        .collect::<Vec<_>>();
    makepad_projection_runtime_alias_config(rxrc::RuntimeConfigSource::AndroidProperty, values)
}

fn makepad_projection_runtime_alias_config(
    source: rxrc::RuntimeConfigSource,
    values: Vec<(&'static str, String)>,
) -> rxrc::RuntimeConfig {
    let mut config = rxrc::RuntimeConfig::new();
    for (key, value) in values {
        let Ok(parsed) =
            rxrc::parse_projection_runtime_pairs(source.clone(), [(key, value.as_str())])
        else {
            continue;
        };
        for setting in parsed.config.iter() {
            config.insert(setting.clone());
        }
    }
    config
}

pub(super) fn makepad_current_projection_runtime_float(
    key: &str,
    fallback: f32,
    min: f32,
    max: f32,
) -> f32 {
    let config = App::runtime_config();
    let legacy = App::legacy_horizontal_alignment_tuning();
    let runtime = makepad_projection_runtime_resolution(&config, legacy);
    makepad_projection_runtime_float(&runtime.resolution, key, fallback, min, max)
}

fn makepad_projection_runtime_float(
    resolution: &rxrc::RuntimeConfigResolution,
    key: &str,
    fallback: f32,
    min: f32,
    max: f32,
) -> f32 {
    makepad_projection_runtime_optional_float(resolution, key, min, max).unwrap_or(fallback)
}

fn makepad_projection_runtime_optional_float(
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

fn makepad_projection_runtime_text<'a>(
    resolution: &'a rxrc::RuntimeConfigResolution,
    key: &str,
) -> Option<&'a str> {
    resolution
        .resolved()
        .get(key)
        .and_then(rxrc::RuntimeValue::as_text)
}

pub(super) fn makepad_horizontal_alignment_tuning_from_resolution(
    mut tuning: HorizontalAlignmentTuning,
    resolution: &rxrc::RuntimeConfigResolution,
) -> HorizontalAlignmentTuning {
    let global_offset_x_uv = makepad_projection_runtime_optional_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_OFFSET_X_UV,
        -0.5,
        0.5,
    );
    let left_offset_x_uv = makepad_projection_runtime_optional_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_LEFT_OFFSET_X_UV,
        -0.5,
        0.5,
    )
    .or(global_offset_x_uv)
    .unwrap_or(-tuning.projection_area_offset_left_uv);
    let right_offset_x_uv = makepad_projection_runtime_optional_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_RIGHT_OFFSET_X_UV,
        -0.5,
        0.5,
    )
    .or(global_offset_x_uv)
    .unwrap_or(-tuning.projection_area_offset_right_uv);
    tuning.projection_area_offset_left_uv = (-left_offset_x_uv).clamp(-0.5, 0.5);
    tuning.projection_area_offset_right_uv = (-right_offset_x_uv).clamp(-0.5, 0.5);
    tuning.projection_area_offset_vertical_uv = makepad_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_OFFSET_Y_UV,
        tuning.projection_area_offset_vertical_uv,
        -0.5,
        0.5,
    );
    let uniform_scale = makepad_projection_runtime_optional_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_SCALE_UV,
        PROJECTION_AREA_MIN_SCALE,
        PROJECTION_AREA_MAX_SCALE,
    );
    tuning.projection_area_scale_x = makepad_projection_runtime_optional_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_SCALE_X,
        PROJECTION_AREA_MIN_SCALE,
        PROJECTION_AREA_MAX_SCALE,
    )
    .or(uniform_scale)
    .unwrap_or(tuning.projection_area_scale_x);
    tuning.projection_area_scale_y = makepad_projection_runtime_optional_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_SCALE_Y,
        PROJECTION_AREA_MIN_SCALE,
        PROJECTION_AREA_MAX_SCALE,
    )
    .or(uniform_scale)
    .unwrap_or(tuning.projection_area_scale_y);
    tuning.projection_target_offset_x_uv = makepad_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_TARGET_OFFSET_X_UV,
        tuning.projection_target_offset_x_uv,
        -0.5,
        0.5,
    );
    tuning.projection_target_offset_y_uv = makepad_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_TARGET_OFFSET_Y_UV,
        tuning.projection_target_offset_y_uv,
        -0.5,
        0.5,
    );
    tuning.projection_target_scale = makepad_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_TARGET_SCALE,
        tuning.projection_target_scale,
        PROJECTION_TARGET_MIN_SCALE,
        PROJECTION_TARGET_MAX_SCALE,
    );
    tuning.projection_area_radius_x_uv = makepad_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_RADIUS_X_UV,
        tuning.projection_area_radius_x_uv,
        0.05,
        0.5,
    );
    tuning.projection_area_radius_y_uv = makepad_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_RADIUS_Y_UV,
        tuning.projection_area_radius_y_uv,
        0.05,
        0.5,
    );
    tuning.projection_area_corner_radius_uv = makepad_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_CORNER_RADIUS_UV,
        tuning.projection_area_corner_radius_uv,
        0.0,
        0.5,
    );
    tuning.projection_area_opacity = makepad_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_AREA_OPACITY,
        tuning.projection_area_opacity,
        0.0,
        1.0,
    );
    tuning.projection_border_opacity = makepad_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_BORDER_OPACITY,
        tuning.projection_border_opacity,
        0.0,
        1.0,
    );
    if let Some(policy) =
        makepad_projection_runtime_text(resolution, rxrc::KEY_PROJECTION_BORDER_POLICY)
    {
        tuning.projection_border_policy =
            MakepadProjectionBorderPolicy::from_stable_id(policy).shader_code();
    }
    if let Some(processing_layer) =
        makepad_projection_runtime_text(resolution, rxrc::KEY_PROCESSING_LAYER)
    {
        tuning.processing_layer =
            MakepadProcessingLayer::from_stable_id(processing_layer).shader_code();
    }
    tuning.blur_radius_px = makepad_projection_runtime_float(
        resolution,
        rxrc::KEY_CAMERA_BLUR_RADIUS_PX,
        tuning.blur_radius_px,
        0.0,
        16.0,
    );
    tuning.peripheral_stretch_core_scale = makepad_projection_runtime_float(
        resolution,
        rxrc::KEY_PERIPHERAL_STRETCH_CORE_SCALE,
        tuning.peripheral_stretch_core_scale,
        0.05,
        1.0,
    );
    tuning.peripheral_stretch_edge_inset_uv = makepad_projection_runtime_float(
        resolution,
        rxrc::KEY_PERIPHERAL_STRETCH_EDGE_INSET_UV,
        tuning.peripheral_stretch_edge_inset_uv,
        0.0,
        0.49,
    );
    tuning.peripheral_stretch_max_inset_uv = makepad_projection_runtime_float(
        resolution,
        rxrc::KEY_PERIPHERAL_STRETCH_MAX_INSET_UV,
        tuning.peripheral_stretch_max_inset_uv,
        tuning.peripheral_stretch_edge_inset_uv,
        0.49,
    );
    tuning.peripheral_stretch_curve = makepad_projection_runtime_float(
        resolution,
        rxrc::KEY_PERIPHERAL_STRETCH_CURVE,
        tuning.peripheral_stretch_curve,
        0.25,
        6.0,
    );
    tuning.peripheral_stretch_inner_blend_uv = makepad_projection_runtime_float(
        resolution,
        rxrc::KEY_PERIPHERAL_STRETCH_INNER_BLEND_UV,
        tuning.peripheral_stretch_inner_blend_uv,
        0.0,
        0.25,
    );
    tuning.peripheral_stretch_blend_curve = makepad_projection_runtime_float(
        resolution,
        rxrc::KEY_PERIPHERAL_STRETCH_BLEND_CURVE,
        tuning.peripheral_stretch_blend_curve,
        0.25,
        6.0,
    );
    if let Some(blend_mode) =
        makepad_projection_runtime_text(resolution, rxrc::KEY_PERIPHERAL_STRETCH_BLEND_MODE)
    {
        tuning.peripheral_stretch_blend_mode =
            MakepadPeripheralStretchBlendMode::from_stable_id(blend_mode).shader_code();
    }
    if let Some(debug) =
        makepad_projection_runtime_text(resolution, rxrc::KEY_PERIPHERAL_STRETCH_DEBUG)
    {
        tuning.peripheral_stretch_debug =
            MakepadPeripheralStretchDebug::from_stable_id(debug).shader_code();
    }
    if let Some(alpha_mode) =
        makepad_projection_runtime_text(resolution, rxrc::KEY_PROJECTION_ALPHA_MODE)
    {
        tuning.projection_alpha_mode =
            MakepadProjectionAlphaMode::from_stable_id(alpha_mode).shader_code();
    }
    tuning.projection_alpha_scale = makepad_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_ALPHA_SCALE,
        tuning.projection_alpha_scale,
        0.0,
        4.0,
    );
    tuning.projection_alpha_bias = makepad_projection_runtime_float(
        resolution,
        rxrc::KEY_PROJECTION_ALPHA_BIAS,
        tuning.projection_alpha_bias,
        -1.0,
        1.0,
    );
    tuning
}

fn makepad_projection_runtime_config_defaults() -> rxrc::RuntimeConfig {
    let mut config = rxrc::RuntimeConfig::new();
    set_projection_manifest_text(
        &mut config,
        rxrc::KEY_CAMERA_PROJECTION_MODE,
        DEFAULT_CAMERA_PROJECTION_MODE,
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_text(
        &mut config,
        rxrc::KEY_PROJECTION_GEOMETRY_PROFILE,
        DEFAULT_CAMERA_PROJECTION_GEOMETRY_PROFILE,
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PROJECTION_SCALE,
        DEFAULT_PROJECTION_SCALE,
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PROJECTION_DEPTH_METERS,
        DEFAULT_PROJECTION_DEPTH_METERS,
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_CAMERA_PREVIEW_FOV_Y_DEGREES,
        f64::from(TARGET_PROJECTION_PREVIEW_FOV_Y_DEGREES),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_CAMERA_PREVIEW_OFFSET_Y_METERS,
        f64::from(TARGET_PROJECTION_PREVIEW_OFFSET_Y_METERS),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_CAMERA_RAW_OVERLAY_OVERSCAN,
        f64::from(TARGET_PROJECTION_RAW_OVERSCAN),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_LEFT_OFFSET_X_UV,
        f64::from(-TARGET_PROJECTION_AREA_OFFSET_LEFT_UV),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_RIGHT_OFFSET_X_UV,
        f64::from(-TARGET_PROJECTION_AREA_OFFSET_RIGHT_UV),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_OFFSET_Y_UV,
        f64::from(TARGET_PROJECTION_AREA_OFFSET_VERTICAL_UV),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_SCALE_X,
        f64::from(TARGET_PROJECTION_AREA_SCALE_X),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_SCALE_Y,
        f64::from(TARGET_PROJECTION_AREA_SCALE_Y),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PROJECTION_TARGET_OFFSET_X_UV,
        f64::from(TARGET_PROJECTION_TARGET_OFFSET_X_UV),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PROJECTION_TARGET_OFFSET_Y_UV,
        f64::from(TARGET_PROJECTION_TARGET_OFFSET_Y_UV),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PROJECTION_TARGET_SCALE,
        f64::from(TARGET_PROJECTION_TARGET_SCALE),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_RADIUS_X_UV,
        f64::from(TARGET_PROJECTION_AREA_RADIUS_X_UV),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_RADIUS_Y_UV,
        f64::from(TARGET_PROJECTION_AREA_RADIUS_Y_UV),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_CORNER_RADIUS_UV,
        f64::from(TARGET_PROJECTION_AREA_CORNER_RADIUS_UV),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PROJECTION_AREA_OPACITY,
        f64::from(TARGET_PROJECTION_AREA_OPACITY),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PROJECTION_BORDER_OPACITY,
        f64::from(TARGET_PROJECTION_BORDER_OPACITY),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_text(
        &mut config,
        rxrc::KEY_PROJECTION_BORDER_POLICY,
        MakepadProjectionBorderPolicy::SolidRed.stable_id(),
        rxrc::RuntimeConfigSource::Default,
    );
    let default_processing_layer = MakepadProcessingLayer::Raw;
    let default_peripheral_stretch = MakepadPeripheralStretchConfig::default();
    set_projection_manifest_text(
        &mut config,
        rxrc::KEY_PROCESSING_LAYER,
        default_processing_layer.stable_id(),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_CAMERA_BLUR_RADIUS_PX,
        2.0,
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_text(
        &mut config,
        rxrc::KEY_PERIPHERAL_STRETCH_MODE,
        default_peripheral_stretch.mode.stable_id(),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PERIPHERAL_STRETCH_CORE_SCALE,
        f64::from(default_peripheral_stretch.core_scale),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PERIPHERAL_STRETCH_EDGE_INSET_UV,
        f64::from(default_peripheral_stretch.edge_inset_uv),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PERIPHERAL_STRETCH_MAX_INSET_UV,
        f64::from(default_peripheral_stretch.max_inset_uv),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PERIPHERAL_STRETCH_CURVE,
        f64::from(default_peripheral_stretch.curve),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PERIPHERAL_STRETCH_INNER_BLEND_UV,
        f64::from(default_peripheral_stretch.inner_blend_uv),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PERIPHERAL_STRETCH_BLEND_CURVE,
        f64::from(default_peripheral_stretch.blend_curve),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_text(
        &mut config,
        rxrc::KEY_PERIPHERAL_STRETCH_BLEND_MODE,
        default_peripheral_stretch.blend_mode.stable_id(),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_text(
        &mut config,
        rxrc::KEY_PERIPHERAL_STRETCH_CORNER_MODE,
        default_peripheral_stretch.corner_mode.stable_id(),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_text(
        &mut config,
        rxrc::KEY_PERIPHERAL_STRETCH_DEBUG,
        default_peripheral_stretch.debug.stable_id(),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_text(
        &mut config,
        rxrc::KEY_PROJECTION_ALPHA_MODE,
        MakepadProjectionAlphaMode::Fixed.stable_id(),
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PROJECTION_ALPHA_SCALE,
        1.0,
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_float(
        &mut config,
        rxrc::KEY_PROJECTION_ALPHA_BIAS,
        0.0,
        rxrc::RuntimeConfigSource::Default,
    );
    set_projection_manifest_text(
        &mut config,
        rxrc::KEY_SOURCE_EYE_MAPPING,
        DEFAULT_MAKEPAD_DISPLAY_SOURCE_EYE_MAPPING,
        rxrc::RuntimeConfigSource::Default,
    );
    config
}

fn makepad_projection_runtime_config_effective(
    config: &rxrc::RuntimeConfig,
    tuning: HorizontalAlignmentTuning,
) -> rxrc::RuntimeConfig {
    let mut manifest = rxrc::RuntimeConfig::new();
    set_projection_manifest_text(
        &mut manifest,
        rxrc::KEY_CAMERA_PROJECTION_MODE,
        &runtime_text(config, KEY_CAMERA_PROJECTION_MODE),
        rxrc::RuntimeConfigSource::Environment,
    );
    set_projection_manifest_text(
        &mut manifest,
        rxrc::KEY_PROJECTION_GEOMETRY_PROFILE,
        &App::direct_camera_projection_geometry_profile(),
        rxrc::RuntimeConfigSource::Environment,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PROJECTION_SCALE,
        runtime_float(config, KEY_PROJECTION_SCALE),
        rxrc::RuntimeConfigSource::Environment,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PROJECTION_DEPTH_METERS,
        runtime_float(config, KEY_PROJECTION_DEPTH_METERS),
        rxrc::RuntimeConfigSource::Environment,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_CAMERA_PREVIEW_FOV_Y_DEGREES,
        runtime_float(config, KEY_CAMERA_PREVIEW_FOV_Y_DEGREES),
        rxrc::RuntimeConfigSource::Environment,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_CAMERA_PREVIEW_OFFSET_Y_METERS,
        runtime_float(config, KEY_CAMERA_PREVIEW_OFFSET_Y_METERS),
        rxrc::RuntimeConfigSource::Environment,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_CAMERA_RAW_OVERLAY_OVERSCAN,
        runtime_float(config, KEY_CAMERA_RAW_OVERLAY_OVERSCAN),
        rxrc::RuntimeConfigSource::Environment,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PROJECTION_AREA_LEFT_OFFSET_X_UV,
        f64::from(-tuning.projection_area_offset_left_uv),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PROJECTION_AREA_RIGHT_OFFSET_X_UV,
        f64::from(-tuning.projection_area_offset_right_uv),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PROJECTION_AREA_OFFSET_Y_UV,
        f64::from(tuning.projection_area_offset_vertical_uv),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PROJECTION_AREA_SCALE_X,
        f64::from(tuning.projection_area_scale_x),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PROJECTION_AREA_SCALE_Y,
        f64::from(tuning.projection_area_scale_y),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PROJECTION_TARGET_OFFSET_X_UV,
        f64::from(tuning.projection_target_offset_x_uv),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PROJECTION_TARGET_OFFSET_Y_UV,
        f64::from(tuning.projection_target_offset_y_uv),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PROJECTION_TARGET_SCALE,
        f64::from(tuning.projection_target_scale),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PROJECTION_AREA_RADIUS_X_UV,
        f64::from(tuning.projection_area_radius_x_uv),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PROJECTION_AREA_RADIUS_Y_UV,
        f64::from(tuning.projection_area_radius_y_uv),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PROJECTION_AREA_CORNER_RADIUS_UV,
        f64::from(tuning.projection_area_corner_radius_uv),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PROJECTION_AREA_OPACITY,
        f64::from(tuning.projection_area_opacity),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PROJECTION_BORDER_OPACITY,
        f64::from(tuning.projection_border_opacity),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_text(
        &mut manifest,
        rxrc::KEY_PROJECTION_BORDER_POLICY,
        MakepadProjectionBorderPolicy::current().stable_id(),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    let processing_layer = MakepadProcessingLayer::from_shader_code(tuning.processing_layer);
    let stretch_blend_mode =
        MakepadPeripheralStretchBlendMode::from_shader_code(tuning.peripheral_stretch_blend_mode);
    let stretch_debug =
        MakepadPeripheralStretchDebug::from_shader_code(tuning.peripheral_stretch_debug);
    set_projection_manifest_text(
        &mut manifest,
        rxrc::KEY_PROCESSING_LAYER,
        processing_layer.stable_id(),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_CAMERA_BLUR_RADIUS_PX,
        f64::from(tuning.blur_radius_px),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_text(
        &mut manifest,
        rxrc::KEY_PERIPHERAL_STRETCH_MODE,
        MakepadPeripheralStretchMode::EdgeStretch.stable_id(),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PERIPHERAL_STRETCH_CORE_SCALE,
        f64::from(tuning.peripheral_stretch_core_scale),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PERIPHERAL_STRETCH_EDGE_INSET_UV,
        f64::from(tuning.peripheral_stretch_edge_inset_uv),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PERIPHERAL_STRETCH_MAX_INSET_UV,
        f64::from(tuning.peripheral_stretch_max_inset_uv),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PERIPHERAL_STRETCH_CURVE,
        f64::from(tuning.peripheral_stretch_curve),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PERIPHERAL_STRETCH_INNER_BLEND_UV,
        f64::from(tuning.peripheral_stretch_inner_blend_uv),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PERIPHERAL_STRETCH_BLEND_CURVE,
        f64::from(tuning.peripheral_stretch_blend_curve),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_text(
        &mut manifest,
        rxrc::KEY_PERIPHERAL_STRETCH_BLEND_MODE,
        stretch_blend_mode.stable_id(),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_text(
        &mut manifest,
        rxrc::KEY_PERIPHERAL_STRETCH_CORNER_MODE,
        MakepadPeripheralStretchCornerMode::TargetFootprint.stable_id(),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_text(
        &mut manifest,
        rxrc::KEY_PERIPHERAL_STRETCH_DEBUG,
        stretch_debug.stable_id(),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_text(
        &mut manifest,
        rxrc::KEY_PROJECTION_ALPHA_MODE,
        MakepadProjectionAlphaMode::current().stable_id(),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PROJECTION_ALPHA_SCALE,
        f64::from(tuning.projection_alpha_scale),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_float(
        &mut manifest,
        rxrc::KEY_PROJECTION_ALPHA_BIAS,
        f64::from(tuning.projection_alpha_bias),
        rxrc::RuntimeConfigSource::AndroidProperty,
    );
    set_projection_manifest_text(
        &mut manifest,
        rxrc::KEY_SOURCE_EYE_MAPPING,
        makepad_display_source_eye_mapping(),
        rxrc::RuntimeConfigSource::Synthetic,
    );
    manifest
}

fn makepad_projection_alias_records() -> Vec<rxrc::RuntimeKeyAliasRecord> {
    [
        "debug.rustyxr.projection.depth.meters",
        "debug.rustyxr.camera.preview.fov.y.degrees",
        "debug.rustyxr.camera.preview.offset.y.meters",
        "debug.rustyxr.camera.raw.overlay.overscan",
        "debug.rustyxr.projection.area.scale.uv",
        "debug.rustyxr.projection.area.offset.x.uv",
        "debug.rustyxr.projection.area.left.offset.x.uv",
        "debug.rustyxr.projection.area.right.offset.x.uv",
        "debug.rustyxr.projection.area.offset.y.uv",
        "debug.rustyxr.projection.area.scale.x",
        "debug.rustyxr.projection.area.scale.y",
        "debug.rustyxr.projection.target.offset.x.uv",
        "debug.rustyxr.projection.target.offset.y.uv",
        "debug.rustyxr.projection.target.scale",
        "debug.rustyxr.projection.target.joystick.controls",
        "debug.rustyxr.projection.area.radius.x.uv",
        "debug.rustyxr.projection.area.radius.y.uv",
        "debug.rustyxr.projection.area.corner.radius.uv",
        "debug.rustyxr.projection.area.opacity",
        "debug.rustyxr.projection.border.opacity",
        "debug.rustyxr.projection.border.policy",
        "debug.rustyxr.processing.layer",
        "debug.rustyxr.camera.blur.radius.px",
        "debug.rustyxr.peripheral.stretch.mode",
        "debug.rustyxr.peripheral.stretch.core.scale",
        "debug.rustyxr.peripheral.stretch.edge.inset.uv",
        "debug.rustyxr.peripheral.stretch.max.inset.uv",
        "debug.rustyxr.peripheral.stretch.curve",
        "debug.rustyxr.peripheral.stretch.inner.blend.uv",
        "debug.rustyxr.peripheral.stretch.blend.curve",
        "debug.rustyxr.peripheral.stretch.blend.mode",
        "debug.rustyxr.peripheral.stretch.corner.mode",
        "debug.rustyxr.peripheral.stretch.debug",
        "debug.rustyxr.projection.alpha.mode",
        "debug.rustyxr.projection.alpha.scale",
        "debug.rustyxr.projection.alpha.bias",
    ]
    .into_iter()
    .filter_map(|key| rxrc::resolve_projection_runtime_key(key).ok())
    .collect()
}

fn set_projection_manifest_text(
    config: &mut rxrc::RuntimeConfig,
    key: &'static str,
    value: &str,
    source: rxrc::RuntimeConfigSource,
) {
    config
        .set(key, rxrc::RuntimeValue::Text(value.to_string()), source)
        .expect("projection manifest keys should be public-safe");
}

fn set_projection_manifest_float(
    config: &mut rxrc::RuntimeConfig,
    key: &'static str,
    value: f64,
    source: rxrc::RuntimeConfigSource,
) {
    config
        .set(key, rxrc::RuntimeValue::Float(value), source)
        .expect("projection manifest keys should be public-safe");
}
