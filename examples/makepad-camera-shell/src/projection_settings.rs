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
    PeripheralStretch,
}

impl MakepadProcessingLayer {
    pub(crate) fn current() -> Self {
        let value = hotload_text_any(
            &[rxrc::KEY_PROCESSING_LAYER, KEY_MAKEPAD_PROCESSING_LAYER],
            "raw",
        );
        Self::from_stable_id(&value)
    }

    pub(crate) fn from_stable_id(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "blur" => Self::Blur,
            "stretch"
            | "peripheral-stretch"
            | "border-stretch"
            | "projection-border-stretch"
            | "edge-stretch" => Self::PeripheralStretch,
            _ => Self::Raw,
        }
    }

    pub(crate) fn stable_id(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Blur => "blur",
            Self::PeripheralStretch => "peripheral-stretch",
        }
    }

    pub(crate) fn shader_code(self) -> f32 {
        match self {
            Self::Raw => 0.0,
            Self::Blur => 1.0,
            Self::PeripheralStretch => 2.0,
        }
    }

    pub(crate) fn from_shader_code(value: f32) -> Self {
        match value.round() as i32 {
            1 => Self::Blur,
            2 => Self::PeripheralStretch,
            _ => Self::Raw,
        }
    }

    pub(crate) fn consumes_projection_exterior(self) -> bool {
        matches!(self, Self::PeripheralStretch)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MakepadPeripheralStretchMode {
    EdgeStretch,
}

impl MakepadPeripheralStretchMode {
    pub(crate) fn current() -> Self {
        let value = hotload_text(rxrc::KEY_PERIPHERAL_STRETCH_MODE, "edge-stretch");
        Self::from_stable_id(&value)
    }

    pub(crate) fn from_stable_id(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            ""
            | "edge-stretch"
            | "stretch"
            | "peripheral-stretch"
            | "border-stretch"
            | "projection-border-stretch" => Self::EdgeStretch,
            _ => Self::EdgeStretch,
        }
    }

    pub(crate) fn stable_id(self) -> &'static str {
        match self {
            Self::EdgeStretch => "edge-stretch",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MakepadPeripheralStretchBlendMode {
    Off,
    TargetInnerBand,
}

impl MakepadPeripheralStretchBlendMode {
    pub(crate) fn current() -> Self {
        let value = hotload_text(rxrc::KEY_PERIPHERAL_STRETCH_BLEND_MODE, "target-inner-band");
        Self::from_stable_id(&value)
    }

    pub(crate) fn from_stable_id(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "0" | "false" | "no" | "off" | "disabled" => Self::Off,
            _ => Self::TargetInnerBand,
        }
    }

    pub(crate) fn stable_id(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::TargetInnerBand => "target-inner-band",
        }
    }

    pub(crate) fn shader_code(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::TargetInnerBand => 1.0,
        }
    }

    pub(crate) fn from_shader_code(value: f32) -> Self {
        if value.round() as i32 == 0 {
            Self::Off
        } else {
            Self::TargetInnerBand
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MakepadPeripheralStretchCornerMode {
    TargetFootprint,
}

impl MakepadPeripheralStretchCornerMode {
    pub(crate) fn current() -> Self {
        let value = hotload_text(rxrc::KEY_PERIPHERAL_STRETCH_CORNER_MODE, "target-footprint");
        Self::from_stable_id(&value)
    }

    pub(crate) fn from_stable_id(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            ""
            | "target-footprint"
            | "projection-area"
            | "projection-area-rect"
            | "rect"
            | "rectangle" => Self::TargetFootprint,
            _ => Self::TargetFootprint,
        }
    }

    pub(crate) fn stable_id(self) -> &'static str {
        match self {
            Self::TargetFootprint => "target-footprint",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MakepadPeripheralStretchDebug {
    Off,
    Regions,
    SampleUv,
}

impl MakepadPeripheralStretchDebug {
    pub(crate) fn current() -> Self {
        let value = hotload_text(rxrc::KEY_PERIPHERAL_STRETCH_DEBUG, "off");
        Self::from_stable_id(&value)
    }

    pub(crate) fn from_stable_id(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "1" | "true" | "yes" | "on" | "enabled" | "regions" | "region" => Self::Regions,
            "2" | "sample-uv" | "sampleuv" | "uv" => Self::SampleUv,
            _ => Self::Off,
        }
    }

    pub(crate) fn stable_id(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Regions => "regions",
            Self::SampleUv => "sample-uv",
        }
    }

    pub(crate) fn shader_code(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Regions => 1.0,
            Self::SampleUv => 2.0,
        }
    }

    pub(crate) fn from_shader_code(value: f32) -> Self {
        match value.round() as i32 {
            1 => Self::Regions,
            2 => Self::SampleUv,
            _ => Self::Off,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MakepadPeripheralStretchConfig {
    pub(crate) mode: MakepadPeripheralStretchMode,
    pub(crate) core_scale: f32,
    pub(crate) edge_inset_uv: f32,
    pub(crate) max_inset_uv: f32,
    pub(crate) curve: f32,
    pub(crate) inner_blend_uv: f32,
    pub(crate) blend_curve: f32,
    pub(crate) blend_mode: MakepadPeripheralStretchBlendMode,
    pub(crate) corner_mode: MakepadPeripheralStretchCornerMode,
    pub(crate) debug: MakepadPeripheralStretchDebug,
}

impl Default for MakepadPeripheralStretchConfig {
    fn default() -> Self {
        Self {
            mode: MakepadPeripheralStretchMode::EdgeStretch,
            core_scale: 1.0,
            edge_inset_uv: 0.015,
            max_inset_uv: 0.14,
            curve: 1.6,
            inner_blend_uv: 0.040,
            blend_curve: 1.6,
            blend_mode: MakepadPeripheralStretchBlendMode::TargetInnerBand,
            corner_mode: MakepadPeripheralStretchCornerMode::TargetFootprint,
            debug: MakepadPeripheralStretchDebug::Off,
        }
    }
}

impl MakepadPeripheralStretchConfig {
    pub(crate) fn current() -> Self {
        let defaults = Self::default();
        let edge_inset_uv = hotload_f32(
            rxrc::KEY_PERIPHERAL_STRETCH_EDGE_INSET_UV,
            defaults.edge_inset_uv,
            0.0,
            0.49,
        );
        Self {
            mode: MakepadPeripheralStretchMode::current(),
            core_scale: hotload_f32(
                rxrc::KEY_PERIPHERAL_STRETCH_CORE_SCALE,
                defaults.core_scale,
                0.05,
                1.0,
            ),
            edge_inset_uv,
            max_inset_uv: hotload_f32(
                rxrc::KEY_PERIPHERAL_STRETCH_MAX_INSET_UV,
                defaults.max_inset_uv,
                edge_inset_uv,
                0.49,
            ),
            curve: hotload_f32(
                rxrc::KEY_PERIPHERAL_STRETCH_CURVE,
                defaults.curve,
                0.25,
                6.0,
            ),
            inner_blend_uv: hotload_f32(
                rxrc::KEY_PERIPHERAL_STRETCH_INNER_BLEND_UV,
                defaults.inner_blend_uv,
                0.0,
                0.25,
            ),
            blend_curve: hotload_f32(
                rxrc::KEY_PERIPHERAL_STRETCH_BLEND_CURVE,
                defaults.blend_curve,
                0.25,
                6.0,
            ),
            blend_mode: MakepadPeripheralStretchBlendMode::current(),
            corner_mode: MakepadPeripheralStretchCornerMode::current(),
            debug: MakepadPeripheralStretchDebug::current(),
        }
    }

    pub(crate) fn marker_fields(self, processing_layer: MakepadProcessingLayer) -> String {
        let consumes_projection_exterior = processing_layer.consumes_projection_exterior();
        let transition_active = !matches!(self.blend_mode, MakepadPeripheralStretchBlendMode::Off)
            && self.inner_blend_uv > 0.0001;
        let (core_region, transition_region, transition_space, transition_semantics) =
            if transition_active {
                (
                    "target-footprint-minus-inner-transition-band",
                    "target-footprint-inner-edge-band",
                    "target-local-raster-uv",
                    "canonical-sample-to-stretch-sample-remap",
                )
            } else {
                (
                    "target-footprint",
                    "off",
                    "off",
                    "hard-edge-preblend-reference",
                )
            };
        let projection_exterior_mode = if consumes_projection_exterior && transition_active {
            "target-edge-stretch-with-inner-band-blend"
        } else if consumes_projection_exterior {
            "target-edge-stretch-hard-edge"
        } else {
            "projection-border-policy-fallback"
        };
        format!(
            "peripheralStretchMode={} peripheralStretchCoreScale={:.3} peripheralStretchEdgeInsetUv={:.3} peripheralStretchMaxInsetUv={:.3} peripheralStretchCurve={:.3} peripheralStretchInnerBlendUv={:.3} peripheralStretchBlendCurve={:.3} peripheralStretchBlendMode={} peripheralStretchCornerMode={} peripheralStretchDebug={} peripheralStretchActive={} peripheralStretchTransitionActive={} peripheralStretchConsumesProjectionExterior={} peripheralStretchCoreRegion={} peripheralStretchTransitionRegion={} peripheralStretchExteriorRegion=visible-render-surface-minus-target-footprint peripheralStretchTransitionSpace={} peripheralStretchTransitionSemantics={} peripheralStretchProjectionExteriorMode={} peripheralStretchBorderSource=projection-edge-sample peripheralStretchExteriorSource=target-edge-sample peripheralStretchReference=pure-hwb-target-inner-band",
            self.mode.stable_id(),
            self.core_scale,
            self.edge_inset_uv,
            self.max_inset_uv,
            self.curve,
            self.inner_blend_uv,
            self.blend_curve,
            self.blend_mode.stable_id(),
            self.corner_mode.stable_id(),
            self.debug.stable_id(),
            processing_layer == MakepadProcessingLayer::PeripheralStretch,
            transition_active,
            consumes_projection_exterior,
            core_region,
            transition_region,
            transition_space,
            transition_semantics,
            projection_exterior_mode,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MakepadProjectionSampleMode {
    Camera,
    SolidColor,
    SolidNoTexture,
    ClearOnly,
}

impl MakepadProjectionSampleMode {
    pub(crate) fn current() -> Self {
        let value = hotload_text(KEY_MAKEPAD_PROJECTION_SAMPLE_MODE, "camera");
        Self::from_stable_id(&value)
    }

    pub(crate) fn from_stable_id(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "solid" | "solid-color" | "no-camera" | "sample-off" => Self::SolidColor,
            "solid-no-texture" | "solid-notexture" | "no-texture" | "no-camera-texture" => {
                Self::SolidNoTexture
            }
            "clear" | "clear-only" | "no-draw" | "panel-off" => Self::ClearOnly,
            _ => Self::Camera,
        }
    }

    pub(crate) fn stable_id(self) -> &'static str {
        match self {
            Self::Camera => "camera",
            Self::SolidColor => "solid-color",
            Self::SolidNoTexture => "solid-no-texture",
            Self::ClearOnly => "clear-only",
        }
    }

    pub(crate) fn shader_code(self) -> f32 {
        match self {
            Self::Camera => 0.0,
            Self::SolidColor | Self::SolidNoTexture | Self::ClearOnly => 1.0,
        }
    }

    pub(crate) fn binds_camera_textures(self) -> bool {
        match self {
            Self::Camera | Self::SolidColor => true,
            Self::SolidNoTexture | Self::ClearOnly => false,
        }
    }

    pub(crate) fn draws_projection_panel(self) -> bool {
        match self {
            Self::Camera | Self::SolidColor | Self::SolidNoTexture => true,
            Self::ClearOnly => false,
        }
    }
}

pub(crate) fn makepad_blur_radius_px() -> f32 {
    hotload_f32_any(
        &[rxrc::KEY_CAMERA_BLUR_RADIUS_PX, KEY_MAKEPAD_BLUR_RADIUS_PX],
        2.0,
        0.0,
        16.0,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        MakepadPeripheralStretchBlendMode, MakepadPeripheralStretchConfig, MakepadProcessingLayer,
        MakepadProjectionSampleMode,
    };

    #[test]
    fn peripheral_stretch_default_matches_pure_hwb_target_inner_band_reference() {
        let config = MakepadPeripheralStretchConfig::default();

        assert_eq!(config.inner_blend_uv, 0.040);
        assert_eq!(
            config.blend_mode,
            MakepadPeripheralStretchBlendMode::TargetInnerBand
        );

        let fields = config.marker_fields(MakepadProcessingLayer::PeripheralStretch);

        assert!(fields.contains("peripheralStretchInnerBlendUv=0.040"));
        assert!(fields.contains("peripheralStretchBlendMode=target-inner-band"));
        assert!(fields.contains("peripheralStretchActive=true"));
        assert!(fields.contains("peripheralStretchTransitionActive=true"));
        assert!(fields.contains("peripheralStretchConsumesProjectionExterior=true"));
        assert!(fields
            .contains("peripheralStretchCoreRegion=target-footprint-minus-inner-transition-band"));
        assert!(
            fields.contains("peripheralStretchTransitionRegion=target-footprint-inner-edge-band")
        );
        assert!(fields.contains(
            "peripheralStretchProjectionExteriorMode=target-edge-stretch-with-inner-band-blend"
        ));
        assert!(fields.contains("peripheralStretchReference=pure-hwb-target-inner-band"));
    }

    #[test]
    fn solid_no_texture_is_solid_shader_without_camera_binding() {
        let mode = MakepadProjectionSampleMode::from_stable_id("solid-no-texture");

        assert_eq!(mode.stable_id(), "solid-no-texture");
        assert_eq!(mode.shader_code(), 1.0);
        assert!(!mode.binds_camera_textures());
    }

    #[test]
    fn solid_color_keeps_camera_texture_binding() {
        let mode = MakepadProjectionSampleMode::from_stable_id("solid-color");

        assert_eq!(mode.stable_id(), "solid-color");
        assert_eq!(mode.shader_code(), 1.0);
        assert!(mode.binds_camera_textures());
        assert!(mode.draws_projection_panel());
    }

    #[test]
    fn clear_only_skips_camera_binding_and_panel_draw() {
        let mode = MakepadProjectionSampleMode::from_stable_id("clear-only");

        assert_eq!(mode.stable_id(), "clear-only");
        assert_eq!(mode.shader_code(), 1.0);
        assert!(!mode.binds_camera_textures());
        assert!(!mode.draws_projection_panel());
    }
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
            TARGET_PROJECTION_PREVIEW_OFFSET_Y_METERS,
            -2.0,
            2.0,
        );
    }
    makepad_legacy_projection_preview_offset_y_meters()
}

fn makepad_legacy_projection_preview_offset_y_meters() -> f32 {
    hotload_f32(
        KEY_CAMERA_PREVIEW_OFFSET_Y_METERS,
        TARGET_PROJECTION_PREVIEW_OFFSET_Y_METERS,
        -2.0,
        2.0,
    )
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
