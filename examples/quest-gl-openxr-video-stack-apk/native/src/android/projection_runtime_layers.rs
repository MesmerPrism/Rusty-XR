use rusty_xr_runtime_config as rxrc;

use crate::current_android_projection_property_config;

use super::{
    openxr_gles_config::{
        OesCameraProjectionMode, OesPeripheralStretchConfig, OesProcessingLayer,
        OesProjectionAlphaMode, OesProjectionBorderPolicy, OesProjectionRuntimeState,
        OesProjectionTuning,
    },
    projection_runtime_config_layers::{
        oes_projection_runtime_config, oes_projection_runtime_default_config,
    },
    projection_runtime_property_readback::oes_current_android_projection_property_values,
};

pub(super) use super::projection_runtime_property_readback::oes_projection_runtime_resolution_enabled;

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
        state.processing_layer,
        state.blur_radius_px,
        state.peripheral_stretch,
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
    processing_layer: OesProcessingLayer,
    blur_radius_px: f32,
    peripheral_stretch: OesPeripheralStretchConfig,
) -> rxrc::ProjectionRuntimeConfigResolution {
    let defaults = oes_projection_runtime_default_config();
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
        processing_layer,
        blur_radius_px,
        peripheral_stretch,
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
