use std::{
    ffi::{CStr, CString},
    os::raw::{c_char, c_int},
};

use jni::{
    objects::{JObject, JString, JValue},
    sys::jobject,
    JNIEnv, JavaVM,
};

use rusty_xr_camera_model::{ColorRgba, ProjectionBorderDescriptor, ProjectionBorderFillPolicy};
use rusty_xr_contracts::InvalidProjectionFillPolicy;

pub(super) const DEFAULT_PROJECTION_TARGET_DEPTH_METERS: f32 = 1.0;
pub(super) const PROJECTION_PREVIEW_FOV_Y_DEGREES: f32 = 60.0;
pub(super) const PROJECTION_RAW_OVERSCAN: f32 = 1.06;
pub(super) const OES_PROJECTION_RUNTIME_RESOLUTION_ENABLED_PROP: &str =
    "debug.rustyxr.oes.projection.runtime.resolution.enabled";
pub(super) const OES_PROJECTION_RUNTIME_RESOLUTION_ENABLED_EXTRA: &str =
    "rustyxr.projectionRuntimeResolutionEnabled";

const OES_TUNING_PROP_PROJECTION_DEPTH_METERS: &str = "debug.rustyxr.projection.depth.meters";
const OES_TUNING_PROP_CAMERA_PREVIEW_FOV_Y_DEGREES: &str =
    "debug.rustyxr.camera.preview.fov.y.degrees";
const OES_TUNING_PROP_CAMERA_PREVIEW_OFFSET_Y_METERS: &str =
    "debug.rustyxr.camera.preview.offset.y.meters";
const OES_TUNING_PROP_CAMERA_RAW_OVERLAY_OVERSCAN: &str =
    "debug.rustyxr.camera.raw.overlay.overscan";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum OesProjectionBorderPolicy {
    #[default]
    SolidRed,
    PassthroughUnderlay,
}

impl OesProjectionBorderPolicy {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "solid-red" => Some(Self::SolidRed),
            "passthrough-underlay" => Some(Self::PassthroughUnderlay),
            _ => None,
        }
    }

    pub(super) const fn stable_id(self) -> &'static str {
        match self {
            Self::SolidRed => "solid-red",
            Self::PassthroughUnderlay => "passthrough-underlay",
        }
    }

    pub(super) const fn shader_id(self) -> c_int {
        match self {
            Self::SolidRed => 0,
            Self::PassthroughUnderlay => 1,
        }
    }

    pub(super) const fn uses_source_alpha(self) -> bool {
        matches!(self, Self::PassthroughUnderlay)
    }

    pub(super) fn needs_source_alpha(
        self,
        projection_area_opacity: f32,
        projection_border_opacity: f32,
        projection_alpha_mode: OesProjectionAlphaMode,
    ) -> bool {
        self.uses_source_alpha()
            || projection_area_opacity < 0.999
            || projection_border_opacity < 0.999
            || projection_alpha_mode.uses_dynamic_alpha()
    }

    pub(super) const fn clear_color(self) -> (f32, f32, f32, f32) {
        match self {
            Self::SolidRed => (1.0, 0.0, 0.0, 1.0),
            Self::PassthroughUnderlay => (0.0, 0.0, 0.0, 0.0),
        }
    }

    pub(super) const fn shared_fill_policy(self) -> ProjectionBorderFillPolicy {
        match self {
            Self::SolidRed => ProjectionBorderFillPolicy::SolidColor,
            Self::PassthroughUnderlay => ProjectionBorderFillPolicy::PassthroughUnderlay,
        }
    }

    pub(super) fn shared_descriptor(self, opacity: f32) -> ProjectionBorderDescriptor {
        let (r, g, b, a) = self.clear_color();
        ProjectionBorderDescriptor::new(
            self.shared_fill_policy(),
            ColorRgba::new(r, g, b, a),
            opacity.clamp(0.0, 1.0),
        )
    }

    pub(super) const fn invalid_source_uv_fill_policy(self) -> InvalidProjectionFillPolicy {
        match self {
            Self::SolidRed => InvalidProjectionFillPolicy::SolidRed,
            Self::PassthroughUnderlay => InvalidProjectionFillPolicy::Transparent,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum OesSourceColorTransfer {
    Identity,
    #[default]
    SrgbToLinear,
}

impl OesSourceColorTransfer {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "identity" => Some(Self::Identity),
            "srgb-to-linear" => Some(Self::SrgbToLinear),
            _ => None,
        }
    }

    pub(super) const fn stable_id(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::SrgbToLinear => "srgb-to-linear",
        }
    }

    pub(super) const fn shader_id(self) -> c_int {
        match self {
            Self::Identity => 0,
            Self::SrgbToLinear => 1,
        }
    }

    pub(super) const fn input_encoding(self) -> &'static str {
        match self {
            Self::Identity => "linear-or-renderer-native-rgb",
            Self::SrgbToLinear => "external-oes-srgb-nonlinear-rgb",
        }
    }

    pub(super) const fn output_encoding(self) -> &'static str {
        match self {
            Self::Identity => "unchanged-rgb",
            Self::SrgbToLinear => "linear-rgb",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum OesProjectionAlphaMode {
    #[default]
    Fixed,
    Red,
    Green,
    Blue,
    Luma,
    InverseRed,
    InverseGreen,
    InverseBlue,
    InverseLuma,
    RedDominance,
    GreenDominance,
    BlueDominance,
    Saturation,
    InverseSaturation,
}

impl OesProjectionAlphaMode {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "fixed" | "none" | "constant" | "area-opacity" | "opacity" => Some(Self::Fixed),
            "red" | "r" | "channel-r" => Some(Self::Red),
            "green" | "g" | "channel-g" => Some(Self::Green),
            "blue" | "b" | "channel-b" => Some(Self::Blue),
            "luma" | "luminance" | "brightness" | "value" => Some(Self::Luma),
            "inverse-red" | "red-inverse" | "inv-red" | "one-minus-red" | "1-red" | "1-r" => {
                Some(Self::InverseRed)
            }
            "inverse-green" | "green-inverse" | "inv-green" | "one-minus-green" | "1-green"
            | "1-g" => Some(Self::InverseGreen),
            "inverse-blue" | "blue-inverse" | "inv-blue" | "one-minus-blue" | "1-blue" | "1-b" => {
                Some(Self::InverseBlue)
            }
            "inverse-luma" | "luma-inverse" | "inv-luma" | "inverse-brightness"
            | "one-minus-luma" | "1-luma" | "1-brightness" => Some(Self::InverseLuma),
            "red-dominance" | "dominant-red" | "red-key" | "red-chroma" | "red-minus-max" => {
                Some(Self::RedDominance)
            }
            "green-dominance" | "dominant-green" | "green-key" | "green-chroma"
            | "green-minus-max" | "screen-green" => Some(Self::GreenDominance),
            "blue-dominance" | "dominant-blue" | "blue-key" | "blue-chroma" | "blue-minus-max" => {
                Some(Self::BlueDominance)
            }
            "saturation" | "chroma" | "max-min" | "colorfulness" => Some(Self::Saturation),
            "inverse-saturation"
            | "saturation-inverse"
            | "inverse-chroma"
            | "inv-chroma"
            | "one-minus-saturation"
            | "1-saturation" => Some(Self::InverseSaturation),
            _ => None,
        }
    }

    pub(super) const fn stable_id(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Red => "red",
            Self::Green => "green",
            Self::Blue => "blue",
            Self::Luma => "luma",
            Self::InverseRed => "inverse-red",
            Self::InverseGreen => "inverse-green",
            Self::InverseBlue => "inverse-blue",
            Self::InverseLuma => "inverse-luma",
            Self::RedDominance => "red-dominance",
            Self::GreenDominance => "green-dominance",
            Self::BlueDominance => "blue-dominance",
            Self::Saturation => "saturation",
            Self::InverseSaturation => "inverse-saturation",
        }
    }

    pub(super) const fn shader_id(self) -> c_int {
        match self {
            Self::Fixed => 0,
            Self::Red => 1,
            Self::Green => 2,
            Self::Blue => 3,
            Self::Luma => 4,
            Self::InverseRed => 5,
            Self::InverseGreen => 6,
            Self::InverseBlue => 7,
            Self::InverseLuma => 8,
            Self::RedDominance => 9,
            Self::GreenDominance => 10,
            Self::BlueDominance => 11,
            Self::Saturation => 12,
            Self::InverseSaturation => 13,
        }
    }

    pub(super) const fn uses_dynamic_alpha(self) -> bool {
        !matches!(self, Self::Fixed)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum OesCameraProjectionMode {
    #[default]
    DisplayScreenHomography,
    WorldCanvas,
}

impl OesCameraProjectionMode {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            ""
            | "display-screen-homography"
            | "screen-homography"
            | "display-eye-homography"
            | "fullscreen"
            | "custom"
            | "default" => Some(Self::DisplayScreenHomography),
            "world-canvas" | "worldCanvas" | "world-space-canvas" | "world-space-quad"
            | "mesh-quad" | "actual-quad" | "canvas" => Some(Self::WorldCanvas),
            _ => None,
        }
    }

    pub(super) const fn stable_id(self) -> &'static str {
        match self {
            Self::DisplayScreenHomography => "display-screen-homography",
            Self::WorldCanvas => "world-canvas",
        }
    }

    pub(super) const fn uses_world_canvas(self) -> bool {
        matches!(self, Self::WorldCanvas)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum OesContentMappingMode {
    #[default]
    CameraProjection,
    #[allow(dead_code)]
    FullFrameStimulusToProjectionArea,
    FullFrameStimulusToSurfaceHomography,
}

impl OesContentMappingMode {
    pub(super) const fn shader_id(self) -> c_int {
        match self {
            Self::CameraProjection => 0,
            Self::FullFrameStimulusToProjectionArea => 1,
            Self::FullFrameStimulusToSurfaceHomography => 0,
        }
    }

    pub(super) const fn stable_id(self) -> &'static str {
        match self {
            Self::CameraProjection => "camera-projection-homography",
            Self::FullFrameStimulusToProjectionArea => "full-frame-stimulus-to-projection-area",
            Self::FullFrameStimulusToSurfaceHomography => {
                "full-frame-stimulus-to-surface-homography"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum OesProcessingLayer {
    #[default]
    Raw,
    Blur,
}

impl OesProcessingLayer {
    pub(super) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "raw" => Some(Self::Raw),
            "blur" => Some(Self::Blur),
            _ => None,
        }
    }

    pub(super) const fn stable_id(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Blur => "blur",
        }
    }

    pub(super) const fn shader_id(self) -> c_int {
        match self {
            Self::Raw => 0,
            Self::Blur => 1,
        }
    }
}

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
    pub(super) processing_layer: OesProcessingLayer,
    pub(super) blur_radius_px: f32,
    pub(super) base_projection_tuning: OesProjectionTuning,
    pub(super) projection_state: OesProjectionRuntimeState,
    pub(super) camera_color_controls: OesColorControls,
}

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

fn projection_border_policy_from_activity(
    app: &android_activity::AndroidApp,
) -> OesProjectionBorderPolicy {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return OesProjectionBorderPolicy::default();
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return OesProjectionBorderPolicy::default();
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    let requested = activity_string_extra(&mut env, &activity, "rustyxr.projectionBorderPolicy");
    requested
        .as_deref()
        .and_then(OesProjectionBorderPolicy::parse)
        .unwrap_or_default()
}

fn processing_layer_from_activity(app: &android_activity::AndroidApp) -> OesProcessingLayer {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return OesProcessingLayer::default();
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return OesProcessingLayer::default();
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    let requested = activity_string_extra(&mut env, &activity, "rustyxr.processingLayer");
    requested
        .as_deref()
        .and_then(OesProcessingLayer::parse)
        .unwrap_or_default()
}

fn camera_projection_mode_from_activity(
    app: &android_activity::AndroidApp,
) -> OesCameraProjectionMode {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return OesCameraProjectionMode::default();
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return OesCameraProjectionMode::default();
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    let requested = activity_string_extra(&mut env, &activity, "rustyxr.cameraProjectionMode");
    requested
        .as_deref()
        .and_then(OesCameraProjectionMode::parse)
        .unwrap_or_default()
}

fn blur_radius_px_from_activity(app: &android_activity::AndroidApp) -> f32 {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return 2.0;
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return 2.0;
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    activity_string_extra(&mut env, &activity, "rustyxr.cameraBlurRadiusPx")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(2.0)
        .clamp(0.0, 16.0)
}

fn projection_depth_meters_from_activity(app: &android_activity::AndroidApp) -> f32 {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return DEFAULT_PROJECTION_TARGET_DEPTH_METERS;
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return DEFAULT_PROJECTION_TARGET_DEPTH_METERS;
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    activity_string_extra(&mut env, &activity, "rustyxr.projectionDepthMeters")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(DEFAULT_PROJECTION_TARGET_DEPTH_METERS)
        .clamp(0.05, 10.0)
}

fn projection_preview_fov_y_degrees_from_activity(app: &android_activity::AndroidApp) -> f32 {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return PROJECTION_PREVIEW_FOV_Y_DEGREES;
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return PROJECTION_PREVIEW_FOV_Y_DEGREES;
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    activity_string_extra(&mut env, &activity, "rustyxr.cameraPreviewFovYDegrees")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(PROJECTION_PREVIEW_FOV_Y_DEGREES)
        .clamp(1.0, 175.0)
}

fn projection_preview_offset_y_meters_from_activity(app: &android_activity::AndroidApp) -> f32 {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return 0.0;
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return 0.0;
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    activity_string_extra(&mut env, &activity, "rustyxr.cameraPreviewOffsetYMeters")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(0.0)
        .clamp(-2.0, 2.0)
}

fn projection_raw_overscan_from_activity(app: &android_activity::AndroidApp) -> f32 {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return PROJECTION_RAW_OVERSCAN;
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return PROJECTION_RAW_OVERSCAN;
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    activity_string_extra(&mut env, &activity, "rustyxr.cameraRawOverlayOverscan")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(PROJECTION_RAW_OVERSCAN)
        .max(1.0)
}

fn projection_area_offset_x_uv_from_activity(app: &android_activity::AndroidApp) -> f32 {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return 0.0;
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return 0.0;
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    activity_string_extra(&mut env, &activity, "rustyxr.projectionAreaOffsetXUv")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(0.0)
        .clamp(-0.5, 0.5)
}

fn projection_area_offset_y_uv_from_activity(app: &android_activity::AndroidApp) -> f32 {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return 0.0;
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return 0.0;
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    activity_string_extra(&mut env, &activity, "rustyxr.projectionAreaOffsetYUv")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(0.0)
        .clamp(-0.5, 0.5)
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
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return [base_offset_uv, base_offset_uv];
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return [base_offset_uv, base_offset_uv];
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    let left_x = activity_float_extra(
        &mut env,
        &activity,
        &["rustyxr.projectionAreaLeftOffsetXUv"],
    )
    .unwrap_or(base_offset_uv[0])
    .clamp(-0.5, 0.5);
    let left_y = activity_float_extra(
        &mut env,
        &activity,
        &["rustyxr.projectionAreaLeftOffsetYUv"],
    )
    .unwrap_or(base_offset_uv[1])
    .clamp(-0.5, 0.5);
    let right_x = activity_float_extra(
        &mut env,
        &activity,
        &["rustyxr.projectionAreaRightOffsetXUv"],
    )
    .unwrap_or(base_offset_uv[0])
    .clamp(-0.5, 0.5);
    let right_y = activity_float_extra(
        &mut env,
        &activity,
        &["rustyxr.projectionAreaRightOffsetYUv"],
    )
    .unwrap_or(base_offset_uv[1])
    .clamp(-0.5, 0.5);
    [[left_x, left_y], [right_x, right_y]]
}

fn projection_area_scale_from_activity(app: &android_activity::AndroidApp) -> [f32; 2] {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return [1.0, 1.0];
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return [1.0, 1.0];
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    let uniform_scale = activity_string_extra(&mut env, &activity, "rustyxr.projectionAreaScaleUv")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(1.0)
        .clamp(0.05, 4.0);
    let scale_x = activity_string_extra(&mut env, &activity, "rustyxr.projectionAreaScaleX")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(uniform_scale)
        .clamp(0.05, 4.0);
    let scale_y = activity_string_extra(&mut env, &activity, "rustyxr.projectionAreaScaleY")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(uniform_scale)
        .clamp(0.05, 4.0);
    [scale_x, scale_y]
}

fn projection_area_radius_from_activity(app: &android_activity::AndroidApp) -> [f32; 2] {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return [0.47, 0.36];
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return [0.47, 0.36];
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    let radius_x = activity_string_extra(&mut env, &activity, "rustyxr.projectionAreaRadiusXUv")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(0.47)
        .clamp(0.05, 0.5);
    let radius_y = activity_string_extra(&mut env, &activity, "rustyxr.projectionAreaRadiusYUv")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(0.36)
        .clamp(0.05, 0.5);
    [radius_x, radius_y]
}

fn projection_area_corner_radius_uv_from_activity(app: &android_activity::AndroidApp) -> f32 {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return 0.08;
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return 0.08;
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    activity_string_extra(&mut env, &activity, "rustyxr.projectionAreaCornerRadiusUv")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(0.08)
        .clamp(0.0, 0.5)
}

fn projection_area_opacity_from_activity(app: &android_activity::AndroidApp) -> f32 {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return 1.0;
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return 1.0;
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    activity_string_extra(&mut env, &activity, "rustyxr.projectionAreaOpacity")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(1.0)
        .clamp(0.0, 1.0)
}

fn projection_border_opacity_from_activity(app: &android_activity::AndroidApp) -> f32 {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return 1.0;
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return 1.0;
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    activity_string_extra(&mut env, &activity, "rustyxr.projectionBorderOpacity")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(1.0)
        .clamp(0.0, 1.0)
}

fn projection_alpha_mode_from_activity(
    app: &android_activity::AndroidApp,
) -> OesProjectionAlphaMode {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return OesProjectionAlphaMode::default();
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return OesProjectionAlphaMode::default();
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    activity_string_extra(&mut env, &activity, "rustyxr.projectionAlphaMode")
        .as_deref()
        .and_then(OesProjectionAlphaMode::parse)
        .unwrap_or_default()
}

fn projection_alpha_scale_from_activity(app: &android_activity::AndroidApp) -> f32 {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return 1.0;
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return 1.0;
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    activity_string_extra(&mut env, &activity, "rustyxr.projectionAlphaScale")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(1.0)
        .clamp(0.0, 4.0)
}

fn projection_alpha_bias_from_activity(app: &android_activity::AndroidApp) -> f32 {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return 0.0;
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return 0.0;
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    activity_string_extra(&mut env, &activity, "rustyxr.projectionAlphaBias")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(0.0)
        .clamp(-1.0, 1.0)
}

fn camera_color_controls_from_activity(app: &android_activity::AndroidApp) -> OesColorControls {
    let Ok(java_vm) = (unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }) else {
        return OesColorControls::default();
    };
    let Ok(mut env) = java_vm.attach_current_thread() else {
        return OesColorControls::default();
    };
    let activity =
        unsafe { JObject::from_raw(app.activity_as_ptr().cast::<std::ffi::c_void>() as jobject) };
    let defaults = OesColorControls::default();
    let matrix = activity_string_extra(&mut env, &activity, "rustyxr.cameraColorMatrix")
        .as_deref()
        .map(parse_color_matrix)
        .unwrap_or(defaults.matrix);
    let offset = activity_string_extra(&mut env, &activity, "rustyxr.cameraColorOffset")
        .as_deref()
        .map(parse_color_offset)
        .unwrap_or(defaults.offset);
    let contrast = activity_string_extra(&mut env, &activity, "rustyxr.cameraColorContrast")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(defaults.contrast)
        .clamp(0.0, 4.0);
    let brightness = activity_string_extra(&mut env, &activity, "rustyxr.cameraColorBrightness")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(defaults.brightness)
        .clamp(-1.0, 1.0);
    let saturation = activity_string_extra(&mut env, &activity, "rustyxr.cameraColorSaturation")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(defaults.saturation)
        .clamp(0.0, 4.0);
    let source_transfer =
        activity_string_extra(&mut env, &activity, "rustyxr.oesSourceColorTransfer")
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
