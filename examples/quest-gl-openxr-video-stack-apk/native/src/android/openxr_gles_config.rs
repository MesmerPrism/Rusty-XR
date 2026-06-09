use std::{
    ffi::{CStr, CString},
    os::raw::{c_char, c_int},
};

use jni::{
    objects::{JObject, JString, JValue},
    JNIEnv,
};

pub(super) const DEFAULT_PROJECTION_TARGET_DEPTH_METERS: f32 = 1.0;
pub(super) const PROJECTION_PREVIEW_FOV_Y_DEGREES: f32 = 60.0;
pub(super) const PROJECTION_RAW_OVERSCAN: f32 = 1.06;
pub(super) const OES_PROJECTION_RUNTIME_RESOLUTION_ENABLED_PROP: &str =
    "debug.rustyquest.makepad.oes.projection.runtime.resolution.enabled";
pub(super) const OES_PROJECTION_RUNTIME_RESOLUTION_ENABLED_EXTRA: &str =
    "rustyquest.makepad.projectionRuntimeResolutionEnabled";

pub(super) use super::openxr_gles_contracts::{
    OesCameraProjectionMode, OesContentMappingMode, OesPeripheralStretchBlendMode,
    OesPeripheralStretchConfig, OesPeripheralStretchCornerMode, OesPeripheralStretchDebug,
    OesPeripheralStretchMode, OesProcessingLayer, OesProjectionAlphaMode,
    OesProjectionBorderPolicy, OesSourceColorTransfer,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct OesProjectionTuning {
    pub(super) projection_depth_meters: f32,
    pub(super) camera_preview_fov_y_degrees: f32,
    pub(super) camera_preview_offset_y_meters: f32,
    pub(super) camera_raw_overlay_overscan: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct OesProjectionRuntimeState {
    pub(super) tuning: OesProjectionTuning,
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
    pub(super) processing_layer: OesProcessingLayer,
    pub(super) blur_radius_px: f32,
    pub(super) peripheral_stretch: OesPeripheralStretchConfig,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct OesColorControls {
    pub(super) matrix: [[f32; 3]; 3],
    pub(super) offset: [f32; 3],
    pub(super) contrast: f32,
    pub(super) brightness: f32,
    pub(super) saturation: f32,
    pub(super) source_transfer: OesSourceColorTransfer,
}

impl Default for OesColorControls {
    fn default() -> Self {
        Self {
            matrix: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            offset: [0.0, 0.0, 0.0],
            contrast: 1.0,
            brightness: 0.0,
            saturation: 1.0,
            source_transfer: OesSourceColorTransfer::default(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct OesActivityConfig {
    pub(super) base_projection_tuning: OesProjectionTuning,
    pub(super) projection_state: OesProjectionRuntimeState,
    pub(super) camera_color_controls: OesColorControls,
}

pub(super) fn activity_string_extra(
    env: &mut JNIEnv<'_>,
    activity: &JObject<'_>,
    key: &str,
) -> Option<String> {
    let intent = env
        .call_method(activity, "getIntent", "()Landroid/content/Intent;", &[])
        .and_then(|value| value.l())
        .ok()?;
    if intent.is_null() {
        return None;
    }
    let key = env.new_string(key).ok()?;
    let key_object = JObject::from(key);
    let extras = env
        .call_method(&intent, "getExtras", "()Landroid/os/Bundle;", &[])
        .and_then(|value| value.l())
        .ok()?;
    if extras.is_null() {
        return None;
    }
    let value = env
        .call_method(
            &extras,
            "get",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&key_object)],
        )
        .and_then(|value| value.l())
        .ok()?;
    if value.is_null() {
        return None;
    }
    let value_string = env
        .call_method(&value, "toString", "()Ljava/lang/String;", &[])
        .and_then(|value| value.l())
        .ok()?;
    if value_string.is_null() {
        return None;
    }
    env.get_string(&JString::from(value_string))
        .map(|value| value.to_string_lossy().into_owned())
        .ok()
}

pub(super) fn android_system_property_value(name: &str) -> Option<String> {
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

pub(super) fn android_system_property_f32(name: &str, default: f32, min: f32, max: f32) -> f32 {
    android_system_property_value(name)
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(min, max))
        .unwrap_or(default)
}
