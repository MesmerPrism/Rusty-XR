use rusty_xr_runtime_config as rxrc;

use super::*;

#[derive(Clone, Copy)]
pub(crate) struct ProjectionPanelGeometry {
    pub(crate) width_meters: f32,
    pub(crate) height_meters: f32,
    pub(crate) depth_meters: f32,
    pub(crate) offset_y_meters: f32,
    pub(crate) z_meters: f32,
}

impl ProjectionPanelGeometry {
    pub(crate) fn size(self) -> Vec3f {
        vec3f(self.width_meters, self.height_meters, 0.010)
    }

    pub(crate) fn pos(self) -> Vec3f {
        vec3f(0.0, self.offset_y_meters, self.z_meters)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MakepadProjectionBorderPolicy {
    SolidRed,
    PassthroughUnderlay,
}

impl MakepadProjectionBorderPolicy {
    pub(crate) fn current() -> Self {
        let value = hotload_text(KEY_MAKEPAD_PROJECTION_BORDER_POLICY, "solid-red");
        Self::from_stable_id(&value)
    }

    pub(crate) fn from_stable_id(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "passthrough-underlay" => Self::PassthroughUnderlay,
            _ => Self::SolidRed,
        }
    }

    pub(crate) fn from_shader_code(value: f32) -> Self {
        if value >= 0.5 {
            Self::PassthroughUnderlay
        } else {
            Self::SolidRed
        }
    }

    pub(crate) fn stable_id(self) -> &'static str {
        match self {
            Self::SolidRed => "solid-red",
            Self::PassthroughUnderlay => "passthrough-underlay",
        }
    }

    pub(crate) fn shared_fill_policy_id(self) -> &'static str {
        match self {
            Self::SolidRed => "solid-color",
            Self::PassthroughUnderlay => "passthrough-underlay",
        }
    }

    pub(crate) fn shader_code(self) -> f32 {
        match self {
            Self::SolidRed => 0.0,
            Self::PassthroughUnderlay => 1.0,
        }
    }

    pub(crate) fn wants_native_passthrough(self) -> bool {
        matches!(self, Self::PassthroughUnderlay)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MakepadSourceColorTransfer {
    Identity,
}

impl MakepadSourceColorTransfer {
    const fn stable_id(self) -> &'static str {
        match self {
            Self::Identity => "identity",
        }
    }

    const fn input_encoding(self) -> &'static str {
        match self {
            Self::Identity => "makepad-sampled-rgb",
        }
    }

    const fn output_encoding(self) -> &'static str {
        match self {
            Self::Identity => "makepad-renderer-native-rgb",
        }
    }

    const fn transform_applied(self) -> bool {
        match self {
            Self::Identity => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MakepadProjectionAlphaMode {
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

impl MakepadProjectionAlphaMode {
    pub(crate) fn current() -> Self {
        let value = hotload_text(KEY_MAKEPAD_PROJECTION_ALPHA_MODE, "fixed");
        Self::from_stable_id(&value)
    }

    pub(crate) fn from_stable_id(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "red" | "r" | "channel-r" => Self::Red,
            "green" | "g" | "channel-g" => Self::Green,
            "blue" | "b" | "channel-b" => Self::Blue,
            "luma" | "luminance" | "brightness" | "value" => Self::Luma,
            "inverse-red" | "red-inverse" | "inv-red" | "one-minus-red" | "1-red" | "1-r" => {
                Self::InverseRed
            }
            "inverse-green" | "green-inverse" | "inv-green" | "one-minus-green" | "1-green"
            | "1-g" => Self::InverseGreen,
            "inverse-blue" | "blue-inverse" | "inv-blue" | "one-minus-blue" | "1-blue" | "1-b" => {
                Self::InverseBlue
            }
            "inverse-luma" | "luma-inverse" | "inv-luma" | "inverse-brightness"
            | "one-minus-luma" | "1-luma" | "1-brightness" => Self::InverseLuma,
            "red-dominance" | "dominant-red" | "red-key" | "red-chroma" | "red-minus-max" => {
                Self::RedDominance
            }
            "green-dominance" | "dominant-green" | "green-key" | "green-chroma"
            | "green-minus-max" | "screen-green" => Self::GreenDominance,
            "blue-dominance" | "dominant-blue" | "blue-key" | "blue-chroma" | "blue-minus-max" => {
                Self::BlueDominance
            }
            "saturation" | "chroma" | "max-min" | "colorfulness" => Self::Saturation,
            "inverse-saturation"
            | "saturation-inverse"
            | "inverse-chroma"
            | "inv-chroma"
            | "one-minus-saturation"
            | "1-saturation" => Self::InverseSaturation,
            _ => Self::Fixed,
        }
    }

    pub(crate) fn from_shader_code(value: f32) -> Self {
        match value.round() as i32 {
            1 => Self::Red,
            2 => Self::Green,
            3 => Self::Blue,
            4 => Self::Luma,
            5 => Self::InverseRed,
            6 => Self::InverseGreen,
            7 => Self::InverseBlue,
            8 => Self::InverseLuma,
            9 => Self::RedDominance,
            10 => Self::GreenDominance,
            11 => Self::BlueDominance,
            12 => Self::Saturation,
            13 => Self::InverseSaturation,
            _ => Self::Fixed,
        }
    }

    pub(crate) fn stable_id(self) -> &'static str {
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

    pub(crate) fn shader_code(self) -> f32 {
        match self {
            Self::Fixed => 0.0,
            Self::Red => 1.0,
            Self::Green => 2.0,
            Self::Blue => 3.0,
            Self::Luma => 4.0,
            Self::InverseRed => 5.0,
            Self::InverseGreen => 6.0,
            Self::InverseBlue => 7.0,
            Self::InverseLuma => 8.0,
            Self::RedDominance => 9.0,
            Self::GreenDominance => 10.0,
            Self::BlueDominance => 11.0,
            Self::Saturation => 12.0,
            Self::InverseSaturation => 13.0,
        }
    }

    pub(crate) fn uses_dynamic_alpha(self) -> bool {
        !matches!(self, Self::Fixed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MakepadProcessingLayer {
    Raw,
    Blur,
}

impl MakepadProcessingLayer {
    pub(crate) fn current() -> Self {
        let value = hotload_text(KEY_MAKEPAD_PROCESSING_LAYER, "raw");
        match value.trim().to_ascii_lowercase().as_str() {
            "blur" => Self::Blur,
            _ => Self::Raw,
        }
    }

    pub(crate) fn stable_id(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Blur => "blur",
        }
    }

    pub(crate) fn shader_code(self) -> f32 {
        match self {
            Self::Raw => 0.0,
            Self::Blur => 1.0,
        }
    }
}

pub(crate) fn makepad_blur_radius_px() -> f32 {
    hotload_f32(KEY_MAKEPAD_BLUR_RADIUS_PX, 2.0, 0.0, 16.0)
}

fn makepad_source_color_contract_fields(transfer: MakepadSourceColorTransfer) -> String {
    format!(
        "sourceColorInputEncoding={} sourceColorTransformStage=post_makepad_source_sample_pre_processing_layer sourceColorTransform={} sourceColorTransformOwner=makepad-camera-panel-shader sourceColorTransformApplied={} sourceColorOutputEncoding={} cameraColorControlStage=none",
        transfer.input_encoding(),
        transfer.stable_id(),
        transfer.transform_applied(),
        transfer.output_encoding()
    )
}

pub(crate) fn makepad_current_source_color_contract_fields() -> String {
    makepad_source_color_contract_fields(MakepadSourceColorTransfer::Identity)
}

pub(crate) fn makepad_projection_depth_meters() -> f32 {
    if makepad_projection_runtime_resolution_enabled() {
        return makepad_current_projection_runtime_float(
            rxrc::KEY_PROJECTION_DEPTH_METERS,
            TARGET_PROJECTION_DEPTH_METERS,
            0.05,
            10.0,
        );
    }
    makepad_legacy_projection_depth_meters()
}

fn makepad_legacy_projection_depth_meters() -> f32 {
    hotload_f32(
        KEY_PROJECTION_DEPTH_METERS,
        TARGET_PROJECTION_DEPTH_METERS,
        0.05,
        10.0,
    )
}

fn makepad_camera_projection_mode() -> String {
    hotload_text(KEY_CAMERA_PROJECTION_MODE, DEFAULT_CAMERA_PROJECTION_MODE)
        .trim()
        .to_ascii_lowercase()
        .replace('_', "-")
}

pub(crate) fn makepad_camera_projection_mode_is_world_canvas() -> bool {
    matches!(
        makepad_camera_projection_mode().as_str(),
        "world-canvas" | "world-canvas-mode" | "world-space-canvas" | "world-space-quad"
    )
}

pub(crate) fn makepad_projection_preview_fov_y_degrees() -> f32 {
    if makepad_projection_runtime_resolution_enabled() {
        return makepad_current_projection_runtime_float(
            rxrc::KEY_CAMERA_PREVIEW_FOV_Y_DEGREES,
            TARGET_PROJECTION_PREVIEW_FOV_Y_DEGREES,
            1.0,
            175.0,
        );
    }
    makepad_legacy_projection_preview_fov_y_degrees()
}

fn makepad_legacy_projection_preview_fov_y_degrees() -> f32 {
    hotload_f32(
        KEY_CAMERA_PREVIEW_FOV_Y_DEGREES,
        TARGET_PROJECTION_PREVIEW_FOV_Y_DEGREES,
        1.0,
        175.0,
    )
}

pub(crate) fn makepad_projection_preview_offset_y_meters() -> f32 {
    if makepad_projection_runtime_resolution_enabled() {
        return makepad_current_projection_runtime_float(
            rxrc::KEY_CAMERA_PREVIEW_OFFSET_Y_METERS,
            0.0,
            -2.0,
            2.0,
        );
    }
    makepad_legacy_projection_preview_offset_y_meters()
}

fn makepad_legacy_projection_preview_offset_y_meters() -> f32 {
    hotload_f32(KEY_CAMERA_PREVIEW_OFFSET_Y_METERS, 0.0, -2.0, 2.0)
}

pub(crate) fn makepad_projection_raw_overscan() -> f32 {
    if makepad_projection_runtime_resolution_enabled() {
        return makepad_current_projection_runtime_float(
            rxrc::KEY_CAMERA_RAW_OVERLAY_OVERSCAN,
            TARGET_PROJECTION_RAW_OVERSCAN,
            1.0,
            16.0,
        );
    }
    makepad_legacy_projection_raw_overscan()
}

fn makepad_legacy_projection_raw_overscan() -> f32 {
    hotload_f32(
        KEY_CAMERA_RAW_OVERLAY_OVERSCAN,
        TARGET_PROJECTION_RAW_OVERSCAN,
        1.0,
        16.0,
    )
}

pub(crate) fn makepad_projection_panel_geometry() -> ProjectionPanelGeometry {
    let depth_meters = makepad_projection_depth_meters().max(0.05);
    let fov_y_degrees = makepad_projection_preview_fov_y_degrees().clamp(1.0, 175.0);
    let raw_overscan = makepad_projection_raw_overscan().max(1.0);
    let half_height = (fov_y_degrees * 0.5).to_radians().tan() * depth_meters * raw_overscan;
    let height_meters = (half_height * 2.0).max(0.01);
    let width_meters = height_meters * TARGET_DISPLAY_ASPECT.max(0.1);
    let offset_y_meters = makepad_projection_preview_offset_y_meters().clamp(-2.0, 2.0);
    ProjectionPanelGeometry {
        width_meters,
        height_meters,
        depth_meters,
        offset_y_meters,
        z_meters: -depth_meters,
    }
}

pub(crate) fn makepad_projection_area_opacity() -> f32 {
    hotload_f32(
        KEY_MAKEPAD_PROJECTION_AREA_OPACITY,
        TARGET_PROJECTION_AREA_OPACITY,
        0.0,
        1.0,
    )
}

pub(crate) fn makepad_projection_border_opacity() -> f32 {
    hotload_f32(
        KEY_MAKEPAD_PROJECTION_BORDER_OPACITY,
        TARGET_PROJECTION_BORDER_OPACITY,
        0.0,
        1.0,
    )
}

pub(crate) fn makepad_projection_alpha_scale() -> f32 {
    hotload_f32(KEY_MAKEPAD_PROJECTION_ALPHA_SCALE, 1.0, 0.0, 4.0)
}

pub(crate) fn makepad_projection_alpha_bias() -> f32 {
    hotload_f32(KEY_MAKEPAD_PROJECTION_ALPHA_BIAS, 0.0, -1.0, 1.0)
}

pub(crate) fn makepad_native_passthrough_enabled() -> bool {
    let policy = MakepadProjectionBorderPolicy::current();
    let alpha_mode = MakepadProjectionAlphaMode::current();
    let opacity_needs_passthrough =
        makepad_projection_area_opacity() < 0.999 || makepad_projection_border_opacity() < 0.999;
    hotload_bool(
        KEY_MAKEPAD_NATIVE_PASSTHROUGH_ENABLED,
        policy.wants_native_passthrough()
            || opacity_needs_passthrough
            || alpha_mode.uses_dynamic_alpha(),
    )
}
