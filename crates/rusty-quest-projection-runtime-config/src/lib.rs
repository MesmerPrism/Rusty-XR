//! Runtime configuration helpers for Rusty Quest projection lanes.
//!
//! This crate models generic launch/runtime settings. Downstream apps can map
//! their private environment variables, Android properties, or config files
//! onto these public keys without treating app-specific setting names as
//! canonical keys.
//!
//! Enable the `serde` feature when runtime profiles or operator tools need to
//! serialize these public settings.
//!
//! ```
//! use rusty_quest_projection_runtime_config::{RuntimeConfig, RuntimeConfigSource, RuntimeValue};
//!
//! let mut config = RuntimeConfig::new();
//! config
//!     .set("render_scale", RuntimeValue::Float(0.8), RuntimeConfigSource::Synthetic)
//!     .expect("key should be public-safe");
//! assert_eq!(config.get("render_scale"), Some(&RuntimeValue::Float(0.8)));
//! ```

use std::{collections::BTreeMap, fmt, str::FromStr};

/// Crate version exposed for lightweight smoke checks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Stable runtime setting key.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeKey(String);

impl RuntimeKey {
    pub fn new(value: impl Into<String>) -> Result<Self, RuntimeConfigError> {
        let value = value.into();
        validate_key(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn android_property(&self, prefix: &AndroidPropertyPrefix) -> String {
        let suffix = self.as_str().replace(['_', '-'], ".");
        format!("{}.{}", prefix.as_str(), suffix)
    }
}

impl fmt::Display for RuntimeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RuntimeKey {
    type Err = RuntimeConfigError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

/// Broad area that owns a runtime key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeKeyDomain {
    Projection,
}

/// Projection sub-contract that owns a runtime key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionRuntimeKeyOwner {
    Geometry,
    ProjectionArea,
    TargetFootprint,
    SourceSampling,
    Alpha,
    RendererPolicy,
}

/// Expected value shape for a runtime key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeValueKind {
    Bool,
    Integer,
    Float,
    Text,
}

/// Public runtime-key registry entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeKeyDefinition {
    pub key: &'static str,
    pub domain: RuntimeKeyDomain,
    pub owner: ProjectionRuntimeKeyOwner,
    pub value_kind: RuntimeValueKind,
    pub description: &'static str,
}

impl RuntimeKeyDefinition {
    pub fn runtime_key(&self) -> RuntimeKey {
        RuntimeKey::new(self.key).expect("registered runtime keys should be valid")
    }
}

/// Source spelling accepted for a runtime-key input.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeKeyInputSource {
    Canonical,
    LaunchExtra,
    AndroidProperty,
    EnvironmentVariable,
}

/// Value transformation needed when an input transport encodes a value
/// differently than its canonical runtime key.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeKeyInputValueTransform {
    Identity,
    NegateNumber,
}

impl RuntimeKeyInputValueTransform {
    fn apply(
        self,
        input: &str,
        raw_value: &str,
        value: RuntimeValue,
    ) -> Result<RuntimeValue, RuntimeConfigError> {
        match self {
            Self::Identity => Ok(value),
            Self::NegateNumber => match value {
                RuntimeValue::Integer(value) => value
                    .checked_neg()
                    .map(RuntimeValue::Integer)
                    .ok_or_else(|| RuntimeConfigError::InvalidRuntimeInputValue {
                        input: input.to_string(),
                        value: raw_value.to_string(),
                    }),
                RuntimeValue::Float(value) => Ok(RuntimeValue::Float(-value)),
                RuntimeValue::Bool(_) | RuntimeValue::Text(_) => {
                    Err(RuntimeConfigError::InvalidRuntimeInputValue {
                        input: input.to_string(),
                        value: raw_value.to_string(),
                    })
                }
            },
        }
    }
}

/// Exact input accepted for a canonical runtime key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeKeyInputBinding {
    pub input: &'static str,
    pub canonical_key: &'static str,
    pub source: RuntimeKeyInputSource,
    pub value_transform: RuntimeKeyInputValueTransform,
}

impl RuntimeKeyInputBinding {
    pub fn canonical_runtime_key(&self) -> RuntimeKey {
        RuntimeKey::new(self.canonical_key).expect("registered inputs should target valid keys")
    }
}

/// Input evidence recorded when an input key is canonicalized.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeKeyInputRecord {
    pub input_key: String,
    pub canonical_key: RuntimeKey,
    pub source: RuntimeKeyInputSource,
    pub value_transform: RuntimeKeyInputValueTransform,
}

/// Parsed runtime config plus key-input evidence.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeConfigInputParse {
    pub config: RuntimeConfig,
    pub inputs: Vec<RuntimeKeyInputRecord>,
}

impl RuntimeConfigInputParse {
    pub fn into_config(self) -> RuntimeConfig {
        self.config
    }
}

/// Public Android property prefix. Keep app-specific prefixes in app repos.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AndroidPropertyPrefix(String);

impl AndroidPropertyPrefix {
    pub fn new(value: impl Into<String>) -> Result<Self, RuntimeConfigError> {
        let value = value.into();
        if value.is_empty()
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'.')
        {
            return Err(RuntimeConfigError::InvalidAndroidPropertyPrefix(value));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for AndroidPropertyPrefix {
    fn default() -> Self {
        Self("debug.rustyquest".to_string())
    }
}

/// Public owner label for a runtime-config layer.
///
/// Owners describe where a group of settings came from, such as a launch
/// profile, Android property readback, file profile, or backend default table.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeConfigOwner(String);

impl RuntimeConfigOwner {
    pub fn new(value: impl Into<String>) -> Result<Self, RuntimeConfigError> {
        let value = value.into();
        validate_owner(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RuntimeConfigOwner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RuntimeConfigOwner {
    type Err = RuntimeConfigError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

/// Generic runtime setting value.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeValue {
    Bool(bool),
    Integer(i64),
    Float(f64),
    Text(String),
}

impl RuntimeValue {
    pub fn parse_typed(raw: &str) -> Self {
        let trimmed = raw.trim();
        if let Some(value) = parse_bool(trimmed) {
            return Self::Bool(value);
        }
        if let Ok(value) = trimmed.parse::<i64>() {
            return Self::Integer(value);
        }
        if let Ok(value) = trimmed.parse::<f64>() {
            if value.is_finite() {
                return Self::Float(value);
            }
        }
        Self::Text(trimmed.to_string())
    }

    pub fn parse_for_kind(raw: &str, kind: RuntimeValueKind) -> Option<Self> {
        let trimmed = raw.trim();
        match kind {
            RuntimeValueKind::Bool => parse_bool(trimmed).map(Self::Bool),
            RuntimeValueKind::Integer => trimmed.parse::<i64>().ok().map(Self::Integer),
            RuntimeValueKind::Float => trimmed
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map(Self::Float),
            RuntimeValueKind::Text => Some(Self::Text(trimmed.to_string())),
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(value) => Some(*value),
            Self::Integer(value) => Some(*value as f64),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value),
            _ => None,
        }
    }
}

/// One parsed runtime setting with source metadata.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSetting {
    pub key: RuntimeKey,
    pub value: RuntimeValue,
    pub source: RuntimeConfigSource,
}

impl RuntimeSetting {
    pub fn new(key: RuntimeKey, value: RuntimeValue, source: RuntimeConfigSource) -> Self {
        Self { key, value, source }
    }
}

/// Source of a runtime setting.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeConfigSource {
    Default,
    Environment,
    AndroidProperty,
    File,
    CommandLine,
    Synthetic,
}

/// Ordered map of runtime settings.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RuntimeConfig {
    settings: BTreeMap<RuntimeKey, RuntimeSetting>,
}

impl RuntimeConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, setting: RuntimeSetting) -> Option<RuntimeSetting> {
        self.settings.insert(setting.key.clone(), setting)
    }

    pub fn set(
        &mut self,
        key: impl Into<String>,
        value: RuntimeValue,
        source: RuntimeConfigSource,
    ) -> Result<Option<RuntimeSetting>, RuntimeConfigError> {
        let key = RuntimeKey::new(key)?;
        Ok(self.insert(RuntimeSetting::new(key, value, source)))
    }

    pub fn get(&self, key: &str) -> Option<&RuntimeValue> {
        self.settings
            .get(&RuntimeKey::new(key).ok()?)
            .map(|setting| &setting.value)
    }

    pub fn parse_pairs<'a>(
        source: RuntimeConfigSource,
        pairs: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Result<Self, RuntimeConfigError> {
        let mut config = Self::new();
        for (key, raw_value) in pairs {
            config.set(key, RuntimeValue::parse_typed(raw_value), source.clone())?;
        }
        Ok(config)
    }

    pub fn iter(&self) -> impl Iterator<Item = &RuntimeSetting> {
        self.settings.values()
    }
}

pub const KEY_CAMERA_PROJECTION_MODE: &str = "camera_projection_mode";
pub const KEY_PROJECTION_GEOMETRY_PROFILE: &str = "projection_geometry_profile";
pub const KEY_SYNTHETIC_PROJECTION_PROFILE: &str = "synthetic_projection_profile";
pub const KEY_PROJECTION_SCALE: &str = "projection_scale";
pub const KEY_PROJECTION_DEPTH_METERS: &str = "projection_depth_meters";
pub const KEY_CAMERA_PROJECTION_FOV_Y_DEGREES: &str = "camera_projection_fov_y_degrees";
pub const KEY_CAMERA_PREVIEW_FOV_Y_DEGREES: &str = "camera_preview_fov_y_degrees";
pub const KEY_CAMERA_PREVIEW_OFFSET_Y_METERS: &str = "camera_preview_offset_y_meters";
pub const KEY_CAMERA_RAW_OVERLAY_OVERSCAN: &str = "camera_raw_overlay_overscan";
pub const KEY_PROJECTION_AREA_SCALE_UV: &str = "projection_area_scale_uv";
pub const KEY_PROJECTION_AREA_SCALE_X: &str = "projection_area_scale_x";
pub const KEY_PROJECTION_AREA_SCALE_Y: &str = "projection_area_scale_y";
pub const KEY_PROJECTION_AREA_OFFSET_X_UV: &str = "projection_area_offset_x_uv";
pub const KEY_PROJECTION_AREA_OFFSET_Y_UV: &str = "projection_area_offset_y_uv";
pub const KEY_PROJECTION_AREA_LEFT_OFFSET_X_UV: &str = "projection_area_left_offset_x_uv";
pub const KEY_PROJECTION_AREA_LEFT_OFFSET_Y_UV: &str = "projection_area_left_offset_y_uv";
pub const KEY_PROJECTION_AREA_RIGHT_OFFSET_X_UV: &str = "projection_area_right_offset_x_uv";
pub const KEY_PROJECTION_AREA_RIGHT_OFFSET_Y_UV: &str = "projection_area_right_offset_y_uv";
pub const KEY_PROJECTION_AREA_RADIUS_X_UV: &str = "projection_area_radius_x_uv";
pub const KEY_PROJECTION_AREA_RADIUS_Y_UV: &str = "projection_area_radius_y_uv";
pub const KEY_PROJECTION_AREA_CORNER_RADIUS_UV: &str = "projection_area_corner_radius_uv";
pub const KEY_PROJECTION_AREA_OPACITY: &str = "projection_area_opacity";
pub const KEY_PROJECTION_BORDER_OPACITY: &str = "projection_border_opacity";
pub const KEY_PROJECTION_BORDER_POLICY: &str = "projection_border_policy";
pub const KEY_PROJECTION_TARGET_OFFSET_X_UV: &str = "projection_target_offset_x_uv";
pub const KEY_PROJECTION_TARGET_OFFSET_Y_UV: &str = "projection_target_offset_y_uv";
pub const KEY_PROJECTION_TARGET_SCALE: &str = "projection_target_scale";
pub const KEY_PROJECTION_TARGET_JOYSTICK_CONTROLS: &str = "projection_target_joystick_controls";
pub const KEY_PROJECTION_TARGET_BREATH_CONTROLS: &str = "projection_target_breath_controls";
pub const KEY_PROJECTION_TARGET_BREATH_STREAM: &str = "projection_target_breath_stream";
pub const KEY_PROJECTION_TARGET_BREATH_MIN_SCALE: &str = "projection_target_breath_min_scale";
pub const KEY_PROJECTION_TARGET_BREATH_MAX_SCALE: &str = "projection_target_breath_max_scale";
pub const KEY_PROJECTION_TARGET_BREATH_SMOOTHING_ALPHA: &str =
    "projection_target_breath_smoothing_alpha";
pub const KEY_PROJECTION_TARGET_BREATH_INVERT: &str = "projection_target_breath_invert";
pub const KEY_PROJECTION_TARGET_BREATH_MIN_QUALITY: &str = "projection_target_breath_min_quality";
pub const KEY_PROCESSING_LAYER: &str = "processing_layer";
pub const KEY_CAMERA_BLUR_RADIUS_PX: &str = "camera_blur_radius_px";
pub const KEY_PERIPHERAL_STRETCH_MODE: &str = "peripheral_stretch_mode";
pub const KEY_PERIPHERAL_STRETCH_CORE_SCALE: &str = "peripheral_stretch_core_scale";
pub const KEY_PERIPHERAL_STRETCH_EDGE_INSET_UV: &str = "peripheral_stretch_edge_inset_uv";
pub const KEY_PERIPHERAL_STRETCH_MAX_INSET_UV: &str = "peripheral_stretch_max_inset_uv";
pub const KEY_PERIPHERAL_STRETCH_CURVE: &str = "peripheral_stretch_curve";
pub const KEY_PERIPHERAL_STRETCH_INNER_BLEND_UV: &str = "peripheral_stretch_inner_blend_uv";
pub const KEY_PERIPHERAL_STRETCH_BLEND_CURVE: &str = "peripheral_stretch_blend_curve";
pub const KEY_PERIPHERAL_STRETCH_BLEND_MODE: &str = "peripheral_stretch_blend_mode";
pub const KEY_PERIPHERAL_STRETCH_CORNER_MODE: &str = "peripheral_stretch_corner_mode";
pub const KEY_PERIPHERAL_STRETCH_DEBUG: &str = "peripheral_stretch_debug";
pub const KEY_PROJECTION_ALPHA_MODE: &str = "projection_alpha_mode";
pub const KEY_PROJECTION_ALPHA_SCALE: &str = "projection_alpha_scale";
pub const KEY_PROJECTION_ALPHA_BIAS: &str = "projection_alpha_bias";
pub const KEY_SOURCE_EYE_MAPPING: &str = "source_eye_mapping";
pub const KEY_SOURCE_TEXTURE_ROTATION: &str = "source_texture_rotation";
pub const KEY_SOURCE_TEXTURE_FLIP_X: &str = "source_texture_flip_x";
pub const KEY_SOURCE_TEXTURE_FLIP_Y: &str = "source_texture_flip_y";
pub const KEY_SOURCE_TEXTURE_MIRROR: &str = "source_texture_mirror";
pub const KEY_SOURCE_TEXTURE_TRANSFORM_SOURCE: &str = "source_texture_transform_source";
pub const KEY_SOURCE_TEXTURE_TRANSFORM_REASON: &str = "source_texture_transform_reason";
pub const KEY_LEFT_SOURCE_TEXTURE_TRANSFORM_SOURCE: &str = "left_source_texture_transform_source";
pub const KEY_RIGHT_SOURCE_TEXTURE_TRANSFORM_SOURCE: &str = "right_source_texture_transform_source";
pub const KEY_SOURCE_VISIBLE_RECT_X_UV: &str = "source_visible_rect_x_uv";
pub const KEY_SOURCE_VISIBLE_RECT_Y_UV: &str = "source_visible_rect_y_uv";
pub const KEY_SOURCE_VISIBLE_RECT_WIDTH_UV: &str = "source_visible_rect_width_uv";
pub const KEY_SOURCE_VISIBLE_RECT_HEIGHT_UV: &str = "source_visible_rect_height_uv";

/// Canonical projection runtime keys.
pub const PROJECTION_RUNTIME_KEY_DEFINITIONS: &[RuntimeKeyDefinition] = &[
    projection_key(
        KEY_CAMERA_PROJECTION_MODE,
        ProjectionRuntimeKeyOwner::Geometry,
        RuntimeValueKind::Text,
        "Projection placement mode selected by the runtime profile.",
    ),
    projection_key(
        KEY_PROJECTION_GEOMETRY_PROFILE,
        ProjectionRuntimeKeyOwner::Geometry,
        RuntimeValueKind::Text,
        "Named projection geometry profile used for renderer and analyzer parity.",
    ),
    projection_key(
        KEY_SYNTHETIC_PROJECTION_PROFILE,
        ProjectionRuntimeKeyOwner::Geometry,
        RuntimeValueKind::Text,
        "Named projection profile embedded in synthetic source metadata.",
    ),
    projection_key(
        KEY_PROJECTION_SCALE,
        ProjectionRuntimeKeyOwner::Geometry,
        RuntimeValueKind::Float,
        "Projection scale retained as an explicit non-depth tuning key.",
    ),
    projection_key(
        KEY_PROJECTION_DEPTH_METERS,
        ProjectionRuntimeKeyOwner::Geometry,
        RuntimeValueKind::Float,
        "Head-anchored projection surface depth in meters.",
    ),
    projection_key(
        KEY_CAMERA_PROJECTION_FOV_Y_DEGREES,
        ProjectionRuntimeKeyOwner::Geometry,
        RuntimeValueKind::Float,
        "Projection camera vertical field of view in degrees.",
    ),
    projection_key(
        KEY_CAMERA_PREVIEW_FOV_Y_DEGREES,
        ProjectionRuntimeKeyOwner::Geometry,
        RuntimeValueKind::Float,
        "Preview camera vertical field of view in degrees.",
    ),
    projection_key(
        KEY_CAMERA_PREVIEW_OFFSET_Y_METERS,
        ProjectionRuntimeKeyOwner::Geometry,
        RuntimeValueKind::Float,
        "Preview camera vertical offset in meters.",
    ),
    projection_key(
        KEY_CAMERA_RAW_OVERLAY_OVERSCAN,
        ProjectionRuntimeKeyOwner::Geometry,
        RuntimeValueKind::Float,
        "Raw camera overlay overscan factor.",
    ),
    projection_key(
        KEY_PROJECTION_AREA_SCALE_UV,
        ProjectionRuntimeKeyOwner::ProjectionArea,
        RuntimeValueKind::Float,
        "Uniform projection-area scale in display-eye screen UV.",
    ),
    projection_key(
        KEY_PROJECTION_AREA_SCALE_X,
        ProjectionRuntimeKeyOwner::ProjectionArea,
        RuntimeValueKind::Float,
        "Horizontal projection-area scale in display-eye screen UV.",
    ),
    projection_key(
        KEY_PROJECTION_AREA_SCALE_Y,
        ProjectionRuntimeKeyOwner::ProjectionArea,
        RuntimeValueKind::Float,
        "Vertical projection-area scale in display-eye screen UV.",
    ),
    projection_key(
        KEY_PROJECTION_AREA_OFFSET_X_UV,
        ProjectionRuntimeKeyOwner::ProjectionArea,
        RuntimeValueKind::Float,
        "Global projection-area horizontal offset; positive X moves right.",
    ),
    projection_key(
        KEY_PROJECTION_AREA_OFFSET_Y_UV,
        ProjectionRuntimeKeyOwner::ProjectionArea,
        RuntimeValueKind::Float,
        "Global projection-area vertical offset; positive Y moves down.",
    ),
    projection_key(
        KEY_PROJECTION_AREA_LEFT_OFFSET_X_UV,
        ProjectionRuntimeKeyOwner::ProjectionArea,
        RuntimeValueKind::Float,
        "Left-eye projection-area horizontal offset; positive X moves right.",
    ),
    projection_key(
        KEY_PROJECTION_AREA_LEFT_OFFSET_Y_UV,
        ProjectionRuntimeKeyOwner::ProjectionArea,
        RuntimeValueKind::Float,
        "Left-eye projection-area vertical offset; positive Y moves down.",
    ),
    projection_key(
        KEY_PROJECTION_AREA_RIGHT_OFFSET_X_UV,
        ProjectionRuntimeKeyOwner::ProjectionArea,
        RuntimeValueKind::Float,
        "Right-eye projection-area horizontal offset; positive X moves right.",
    ),
    projection_key(
        KEY_PROJECTION_AREA_RIGHT_OFFSET_Y_UV,
        ProjectionRuntimeKeyOwner::ProjectionArea,
        RuntimeValueKind::Float,
        "Right-eye projection-area vertical offset; positive Y moves down.",
    ),
    projection_key(
        KEY_PROJECTION_AREA_RADIUS_X_UV,
        ProjectionRuntimeKeyOwner::ProjectionArea,
        RuntimeValueKind::Float,
        "Projection-area horizontal radius in display-eye screen UV.",
    ),
    projection_key(
        KEY_PROJECTION_AREA_RADIUS_Y_UV,
        ProjectionRuntimeKeyOwner::ProjectionArea,
        RuntimeValueKind::Float,
        "Projection-area vertical radius in display-eye screen UV.",
    ),
    projection_key(
        KEY_PROJECTION_AREA_CORNER_RADIUS_UV,
        ProjectionRuntimeKeyOwner::ProjectionArea,
        RuntimeValueKind::Float,
        "Projection-area corner radius in display-eye screen UV.",
    ),
    projection_key(
        KEY_PROJECTION_AREA_OPACITY,
        ProjectionRuntimeKeyOwner::ProjectionArea,
        RuntimeValueKind::Float,
        "Projection-area source opacity.",
    ),
    projection_key(
        KEY_PROJECTION_BORDER_OPACITY,
        ProjectionRuntimeKeyOwner::ProjectionArea,
        RuntimeValueKind::Float,
        "Projection-area border opacity.",
    ),
    projection_key(
        KEY_PROJECTION_BORDER_POLICY,
        ProjectionRuntimeKeyOwner::ProjectionArea,
        RuntimeValueKind::Text,
        "Projection-area border fill policy.",
    ),
    projection_key(
        KEY_PROJECTION_TARGET_OFFSET_X_UV,
        ProjectionRuntimeKeyOwner::TargetFootprint,
        RuntimeValueKind::Float,
        "Runtime horizontal offset applied to the metadata or fallback target footprint.",
    ),
    projection_key(
        KEY_PROJECTION_TARGET_OFFSET_Y_UV,
        ProjectionRuntimeKeyOwner::TargetFootprint,
        RuntimeValueKind::Float,
        "Runtime vertical offset applied to the metadata or fallback target footprint.",
    ),
    projection_key(
        KEY_PROJECTION_TARGET_SCALE,
        ProjectionRuntimeKeyOwner::TargetFootprint,
        RuntimeValueKind::Float,
        "Runtime uniform scale applied around the target-footprint center.",
    ),
    projection_key(
        KEY_PROJECTION_TARGET_JOYSTICK_CONTROLS,
        ProjectionRuntimeKeyOwner::TargetFootprint,
        RuntimeValueKind::Text,
        "OpenXR controller target-footprint control mode: off, offset-scale, or horizontal-offset.",
    ),
    projection_key(
        KEY_PROJECTION_TARGET_BREATH_CONTROLS,
        ProjectionRuntimeKeyOwner::TargetFootprint,
        RuntimeValueKind::Text,
        "Broker breath target-footprint control mode.",
    ),
    projection_key(
        KEY_PROJECTION_TARGET_BREATH_STREAM,
        ProjectionRuntimeKeyOwner::TargetFootprint,
        RuntimeValueKind::Text,
        "Broker stream id that provides normalized breath volume for target-footprint scale control.",
    ),
    projection_key(
        KEY_PROJECTION_TARGET_BREATH_MIN_SCALE,
        ProjectionRuntimeKeyOwner::TargetFootprint,
        RuntimeValueKind::Float,
        "Projection-target scale mapped from breath volume 0.0 before optional inversion.",
    ),
    projection_key(
        KEY_PROJECTION_TARGET_BREATH_MAX_SCALE,
        ProjectionRuntimeKeyOwner::TargetFootprint,
        RuntimeValueKind::Float,
        "Projection-target scale mapped from breath volume 1.0 before optional inversion.",
    ),
    projection_key(
        KEY_PROJECTION_TARGET_BREATH_SMOOTHING_ALPHA,
        ProjectionRuntimeKeyOwner::TargetFootprint,
        RuntimeValueKind::Float,
        "Exponential smoothing alpha applied to broker breath target-scale updates.",
    ),
    projection_key(
        KEY_PROJECTION_TARGET_BREATH_INVERT,
        ProjectionRuntimeKeyOwner::TargetFootprint,
        RuntimeValueKind::Bool,
        "Whether breath volume is inverted before mapping it to projection-target scale.",
    ),
    projection_key(
        KEY_PROJECTION_TARGET_BREATH_MIN_QUALITY,
        ProjectionRuntimeKeyOwner::TargetFootprint,
        RuntimeValueKind::Float,
        "Minimum broker breath quality01 accepted for projection-target scale updates.",
    ),
    projection_key(
        KEY_PROCESSING_LAYER,
        ProjectionRuntimeKeyOwner::RendererPolicy,
        RuntimeValueKind::Text,
        "Source-agnostic processing layer applied after target-footprint placement.",
    ),
    projection_key(
        KEY_CAMERA_BLUR_RADIUS_PX,
        ProjectionRuntimeKeyOwner::RendererPolicy,
        RuntimeValueKind::Float,
        "Diagnostic blur radius in source pixels for the blur processing layer.",
    ),
    projection_key(
        KEY_PERIPHERAL_STRETCH_MODE,
        ProjectionRuntimeKeyOwner::RendererPolicy,
        RuntimeValueKind::Text,
        "Peripheral-stretch exterior fill mode.",
    ),
    projection_key(
        KEY_PERIPHERAL_STRETCH_CORE_SCALE,
        ProjectionRuntimeKeyOwner::RendererPolicy,
        RuntimeValueKind::Float,
        "Scale of the coherent target-footprint core used by peripheral stretch.",
    ),
    projection_key(
        KEY_PERIPHERAL_STRETCH_EDGE_INSET_UV,
        ProjectionRuntimeKeyOwner::RendererPolicy,
        RuntimeValueKind::Float,
        "Optional source sample inset from the target-footprint edge for peripheral stretch.",
    ),
    projection_key(
        KEY_PERIPHERAL_STRETCH_MAX_INSET_UV,
        ProjectionRuntimeKeyOwner::RendererPolicy,
        RuntimeValueKind::Float,
        "Maximum permitted target-edge inset for peripheral stretch.",
    ),
    projection_key(
        KEY_PERIPHERAL_STRETCH_CURVE,
        ProjectionRuntimeKeyOwner::RendererPolicy,
        RuntimeValueKind::Float,
        "Curve parameter reserved for non-linear peripheral stretch falloff.",
    ),
    projection_key(
        KEY_PERIPHERAL_STRETCH_INNER_BLEND_UV,
        ProjectionRuntimeKeyOwner::RendererPolicy,
        RuntimeValueKind::Float,
        "Width of the target-footprint inner transition band for peripheral stretch.",
    ),
    projection_key(
        KEY_PERIPHERAL_STRETCH_BLEND_CURVE,
        ProjectionRuntimeKeyOwner::RendererPolicy,
        RuntimeValueKind::Float,
        "Curve parameter for peripheral-stretch target inner-band blending.",
    ),
    projection_key(
        KEY_PERIPHERAL_STRETCH_BLEND_MODE,
        ProjectionRuntimeKeyOwner::RendererPolicy,
        RuntimeValueKind::Text,
        "Peripheral-stretch target-footprint blend mode.",
    ),
    projection_key(
        KEY_PERIPHERAL_STRETCH_CORNER_MODE,
        ProjectionRuntimeKeyOwner::RendererPolicy,
        RuntimeValueKind::Text,
        "Corner handling mode for target-footprint peripheral stretch.",
    ),
    projection_key(
        KEY_PERIPHERAL_STRETCH_DEBUG,
        ProjectionRuntimeKeyOwner::RendererPolicy,
        RuntimeValueKind::Text,
        "Debug overlay mode for peripheral-stretch exterior samples.",
    ),
    projection_key(
        KEY_PROJECTION_ALPHA_MODE,
        ProjectionRuntimeKeyOwner::Alpha,
        RuntimeValueKind::Text,
        "Projection alpha interpretation mode.",
    ),
    projection_key(
        KEY_PROJECTION_ALPHA_SCALE,
        ProjectionRuntimeKeyOwner::Alpha,
        RuntimeValueKind::Float,
        "Projection alpha scale.",
    ),
    projection_key(
        KEY_PROJECTION_ALPHA_BIAS,
        ProjectionRuntimeKeyOwner::Alpha,
        RuntimeValueKind::Float,
        "Projection alpha bias.",
    ),
    projection_key(
        KEY_SOURCE_EYE_MAPPING,
        ProjectionRuntimeKeyOwner::SourceSampling,
        RuntimeValueKind::Text,
        "Mapping from display eye to sampled source eye.",
    ),
    projection_key(
        KEY_SOURCE_TEXTURE_ROTATION,
        ProjectionRuntimeKeyOwner::SourceSampling,
        RuntimeValueKind::Text,
        "Source texture rotation before projection sampling.",
    ),
    projection_key(
        KEY_SOURCE_TEXTURE_FLIP_X,
        ProjectionRuntimeKeyOwner::SourceSampling,
        RuntimeValueKind::Bool,
        "Source texture horizontal flip before projection sampling.",
    ),
    projection_key(
        KEY_SOURCE_TEXTURE_FLIP_Y,
        ProjectionRuntimeKeyOwner::SourceSampling,
        RuntimeValueKind::Bool,
        "Source texture vertical flip before projection sampling.",
    ),
    projection_key(
        KEY_SOURCE_TEXTURE_MIRROR,
        ProjectionRuntimeKeyOwner::SourceSampling,
        RuntimeValueKind::Bool,
        "Source texture mirror operation before projection sampling.",
    ),
    projection_key(
        KEY_SOURCE_TEXTURE_TRANSFORM_SOURCE,
        ProjectionRuntimeKeyOwner::SourceSampling,
        RuntimeValueKind::Text,
        "Owner that supplied the source texture transform.",
    ),
    projection_key(
        KEY_SOURCE_TEXTURE_TRANSFORM_REASON,
        ProjectionRuntimeKeyOwner::SourceSampling,
        RuntimeValueKind::Text,
        "Reason for the selected source texture transform.",
    ),
    projection_key(
        KEY_LEFT_SOURCE_TEXTURE_TRANSFORM_SOURCE,
        ProjectionRuntimeKeyOwner::SourceSampling,
        RuntimeValueKind::Text,
        "Owner that supplied the left-eye source texture transform.",
    ),
    projection_key(
        KEY_RIGHT_SOURCE_TEXTURE_TRANSFORM_SOURCE,
        ProjectionRuntimeKeyOwner::SourceSampling,
        RuntimeValueKind::Text,
        "Owner that supplied the right-eye source texture transform.",
    ),
    projection_key(
        KEY_SOURCE_VISIBLE_RECT_X_UV,
        ProjectionRuntimeKeyOwner::SourceSampling,
        RuntimeValueKind::Float,
        "Source-visible rectangle X coordinate in normalized source UV.",
    ),
    projection_key(
        KEY_SOURCE_VISIBLE_RECT_Y_UV,
        ProjectionRuntimeKeyOwner::SourceSampling,
        RuntimeValueKind::Float,
        "Source-visible rectangle Y coordinate in normalized source UV.",
    ),
    projection_key(
        KEY_SOURCE_VISIBLE_RECT_WIDTH_UV,
        ProjectionRuntimeKeyOwner::SourceSampling,
        RuntimeValueKind::Float,
        "Source-visible rectangle width in normalized source UV.",
    ),
    projection_key(
        KEY_SOURCE_VISIBLE_RECT_HEIGHT_UV,
        ProjectionRuntimeKeyOwner::SourceSampling,
        RuntimeValueKind::Float,
        "Source-visible rectangle height in normalized source UV.",
    ),
];

const fn projection_key(
    key: &'static str,
    owner: ProjectionRuntimeKeyOwner,
    value_kind: RuntimeValueKind,
    description: &'static str,
) -> RuntimeKeyDefinition {
    RuntimeKeyDefinition {
        key,
        domain: RuntimeKeyDomain::Projection,
        owner,
        value_kind,
        description,
    }
}

/// External input spellings for projection runtime keys.
pub const PROJECTION_RUNTIME_KEY_INPUTS: &[RuntimeKeyInputBinding] = &[
    launch_input(
        "rustyquest.cameraProjectionMode",
        KEY_CAMERA_PROJECTION_MODE,
    ),
    launch_input(
        "rustyquest.cameraProjectionGeometryProfile",
        KEY_PROJECTION_GEOMETRY_PROFILE,
    ),
    launch_input(
        "rustyquest.brokerH264ProjectionGeometryProfile",
        KEY_PROJECTION_GEOMETRY_PROFILE,
    ),
    launch_input(
        "rustyquest.brokerH264SyntheticProjectionProfile",
        KEY_SYNTHETIC_PROJECTION_PROFILE,
    ),
    launch_input("rustyquest.cameraProjectionScale", KEY_PROJECTION_SCALE),
    launch_input(
        "rustyquest.projectionDepthMeters",
        KEY_PROJECTION_DEPTH_METERS,
    ),
    launch_input(
        "rustyquest.cameraProjectionFovYDegrees",
        KEY_CAMERA_PROJECTION_FOV_Y_DEGREES,
    ),
    launch_input(
        "rustyquest.cameraPreviewFovYDegrees",
        KEY_CAMERA_PREVIEW_FOV_Y_DEGREES,
    ),
    launch_input(
        "rustyquest.cameraPreviewOffsetYMeters",
        KEY_CAMERA_PREVIEW_OFFSET_Y_METERS,
    ),
    launch_input(
        "rustyquest.cameraRawOverlayOverscan",
        KEY_CAMERA_RAW_OVERLAY_OVERSCAN,
    ),
    launch_input(
        "rustyquest.projectionAreaScaleUv",
        KEY_PROJECTION_AREA_SCALE_UV,
    ),
    launch_input(
        "rustyquest.projectionAreaScaleX",
        KEY_PROJECTION_AREA_SCALE_X,
    ),
    launch_input(
        "rustyquest.projectionAreaScaleY",
        KEY_PROJECTION_AREA_SCALE_Y,
    ),
    launch_input(
        "rustyquest.projectionAreaOffsetXUv",
        KEY_PROJECTION_AREA_OFFSET_X_UV,
    ),
    launch_input(
        "rustyquest.projectionAreaOffsetYUv",
        KEY_PROJECTION_AREA_OFFSET_Y_UV,
    ),
    launch_input(
        "rustyquest.projectionAreaLeftOffsetXUv",
        KEY_PROJECTION_AREA_LEFT_OFFSET_X_UV,
    ),
    launch_input(
        "rustyquest.projectionAreaLeftOffsetYUv",
        KEY_PROJECTION_AREA_LEFT_OFFSET_Y_UV,
    ),
    launch_input(
        "rustyquest.projectionAreaRightOffsetXUv",
        KEY_PROJECTION_AREA_RIGHT_OFFSET_X_UV,
    ),
    launch_input(
        "rustyquest.projectionAreaRightOffsetYUv",
        KEY_PROJECTION_AREA_RIGHT_OFFSET_Y_UV,
    ),
    launch_input(
        "rustyquest.projectionAreaRadiusXUv",
        KEY_PROJECTION_AREA_RADIUS_X_UV,
    ),
    launch_input(
        "rustyquest.projectionAreaRadiusYUv",
        KEY_PROJECTION_AREA_RADIUS_Y_UV,
    ),
    launch_input(
        "rustyquest.projectionAreaCornerRadiusUv",
        KEY_PROJECTION_AREA_CORNER_RADIUS_UV,
    ),
    launch_input(
        "rustyquest.projectionAreaOpacity",
        KEY_PROJECTION_AREA_OPACITY,
    ),
    launch_input(
        "rustyquest.projectionBorderOpacity",
        KEY_PROJECTION_BORDER_OPACITY,
    ),
    launch_input(
        "rustyquest.projectionBorderPolicy",
        KEY_PROJECTION_BORDER_POLICY,
    ),
    launch_input(
        "rustyquest.projectionTargetOffsetXUv",
        KEY_PROJECTION_TARGET_OFFSET_X_UV,
    ),
    launch_input(
        "rustyquest.projectionTargetOffsetYUv",
        KEY_PROJECTION_TARGET_OFFSET_Y_UV,
    ),
    launch_input(
        "rustyquest.projectionTargetScale",
        KEY_PROJECTION_TARGET_SCALE,
    ),
    launch_input(
        "rustyquest.projectionTargetJoystickControls",
        KEY_PROJECTION_TARGET_JOYSTICK_CONTROLS,
    ),
    launch_input(
        "rustyquest.projectionTargetBreathControls",
        KEY_PROJECTION_TARGET_BREATH_CONTROLS,
    ),
    launch_input(
        "rustyquest.projectionTargetBreathStream",
        KEY_PROJECTION_TARGET_BREATH_STREAM,
    ),
    launch_input(
        "rustyquest.projectionTargetBreathMinScale",
        KEY_PROJECTION_TARGET_BREATH_MIN_SCALE,
    ),
    launch_input(
        "rustyquest.projectionTargetBreathMaxScale",
        KEY_PROJECTION_TARGET_BREATH_MAX_SCALE,
    ),
    launch_input(
        "rustyquest.projectionTargetBreathSmoothingAlpha",
        KEY_PROJECTION_TARGET_BREATH_SMOOTHING_ALPHA,
    ),
    launch_input(
        "rustyquest.projectionTargetBreathInvert",
        KEY_PROJECTION_TARGET_BREATH_INVERT,
    ),
    launch_input(
        "rustyquest.projectionTargetBreathMinQuality",
        KEY_PROJECTION_TARGET_BREATH_MIN_QUALITY,
    ),
    launch_input("rustyquest.processingLayer", KEY_PROCESSING_LAYER),
    launch_input("rustyquest.cameraBlurRadiusPx", KEY_CAMERA_BLUR_RADIUS_PX),
    launch_input(
        "rustyquest.peripheralStretchMode",
        KEY_PERIPHERAL_STRETCH_MODE,
    ),
    launch_input(
        "rustyquest.peripheralStretchCoreScale",
        KEY_PERIPHERAL_STRETCH_CORE_SCALE,
    ),
    launch_input(
        "rustyquest.peripheralStretchEdgeInsetUv",
        KEY_PERIPHERAL_STRETCH_EDGE_INSET_UV,
    ),
    launch_input(
        "rustyquest.peripheralStretchMaxInsetUv",
        KEY_PERIPHERAL_STRETCH_MAX_INSET_UV,
    ),
    launch_input(
        "rustyquest.peripheralStretchCurve",
        KEY_PERIPHERAL_STRETCH_CURVE,
    ),
    launch_input(
        "rustyquest.peripheralStretchInnerBlendUv",
        KEY_PERIPHERAL_STRETCH_INNER_BLEND_UV,
    ),
    launch_input(
        "rustyquest.peripheralStretchBlendCurve",
        KEY_PERIPHERAL_STRETCH_BLEND_CURVE,
    ),
    launch_input(
        "rustyquest.peripheralStretchBlendMode",
        KEY_PERIPHERAL_STRETCH_BLEND_MODE,
    ),
    launch_input(
        "rustyquest.peripheralStretchCornerMode",
        KEY_PERIPHERAL_STRETCH_CORNER_MODE,
    ),
    launch_input(
        "rustyquest.peripheralStretchDebug",
        KEY_PERIPHERAL_STRETCH_DEBUG,
    ),
    launch_input("rustyquest.projectionAlphaMode", KEY_PROJECTION_ALPHA_MODE),
    launch_input(
        "rustyquest.projectionAlphaScale",
        KEY_PROJECTION_ALPHA_SCALE,
    ),
    launch_input("rustyquest.projectionAlphaBias", KEY_PROJECTION_ALPHA_BIAS),
    launch_input("rustyquest.cameraSourceEyeMapping", KEY_SOURCE_EYE_MAPPING),
    launch_input(
        "rustyquest.cameraTextureRotation",
        KEY_SOURCE_TEXTURE_ROTATION,
    ),
    launch_input("rustyquest.cameraTextureFlipX", KEY_SOURCE_TEXTURE_FLIP_X),
    launch_input("rustyquest.cameraTextureFlipY", KEY_SOURCE_TEXTURE_FLIP_Y),
    launch_input("rustyquest.cameraTextureMirror", KEY_SOURCE_TEXTURE_MIRROR),
    launch_input(
        "rustyquest.cameraTextureTransformSource",
        KEY_SOURCE_TEXTURE_TRANSFORM_SOURCE,
    ),
    launch_input(
        "rustyquest.cameraTextureTransformReason",
        KEY_SOURCE_TEXTURE_TRANSFORM_REASON,
    ),
    launch_input(
        "rustyquest.leftCameraTextureTransformSource",
        KEY_LEFT_SOURCE_TEXTURE_TRANSFORM_SOURCE,
    ),
    launch_input(
        "rustyquest.rightCameraTextureTransformSource",
        KEY_RIGHT_SOURCE_TEXTURE_TRANSFORM_SOURCE,
    ),
    property_input(
        "debug.rustyquest.camera.projection.mode",
        KEY_CAMERA_PROJECTION_MODE,
    ),
    property_input(
        "debug.rustyquest.projection.geometry.profile",
        KEY_PROJECTION_GEOMETRY_PROFILE,
    ),
    property_input("debug.rustyquest.projection.scale", KEY_PROJECTION_SCALE),
    property_input(
        "debug.rustyquest.projection.depth.meters",
        KEY_PROJECTION_DEPTH_METERS,
    ),
    property_input(
        "debug.rustyquest.camera.projection.fov.y.degrees",
        KEY_CAMERA_PROJECTION_FOV_Y_DEGREES,
    ),
    property_input(
        "debug.rustyquest.camera.preview.fov.y.degrees",
        KEY_CAMERA_PREVIEW_FOV_Y_DEGREES,
    ),
    property_input(
        "debug.rustyquest.camera.preview.offset.y.meters",
        KEY_CAMERA_PREVIEW_OFFSET_Y_METERS,
    ),
    property_input(
        "debug.rustyquest.camera.raw.overlay.overscan",
        KEY_CAMERA_RAW_OVERLAY_OVERSCAN,
    ),
    property_input(
        "debug.rustyquest.projection.area.scale.uv",
        KEY_PROJECTION_AREA_SCALE_UV,
    ),
    property_input(
        "debug.rustyquest.projection.area.scale.x",
        KEY_PROJECTION_AREA_SCALE_X,
    ),
    property_input(
        "debug.rustyquest.projection.area.scale.y",
        KEY_PROJECTION_AREA_SCALE_Y,
    ),
    property_input(
        "debug.rustyquest.projection.area.offset.x.uv",
        KEY_PROJECTION_AREA_OFFSET_X_UV,
    ),
    property_input(
        "debug.rustyquest.projection.area.offset.y.uv",
        KEY_PROJECTION_AREA_OFFSET_Y_UV,
    ),
    property_input(
        "debug.rustyquest.projection.area.left.offset.x.uv",
        KEY_PROJECTION_AREA_LEFT_OFFSET_X_UV,
    ),
    property_input(
        "debug.rustyquest.projection.area.left.offset.y.uv",
        KEY_PROJECTION_AREA_LEFT_OFFSET_Y_UV,
    ),
    property_input(
        "debug.rustyquest.projection.area.right.offset.x.uv",
        KEY_PROJECTION_AREA_RIGHT_OFFSET_X_UV,
    ),
    property_input(
        "debug.rustyquest.projection.area.right.offset.y.uv",
        KEY_PROJECTION_AREA_RIGHT_OFFSET_Y_UV,
    ),
    property_input(
        "debug.rustyquest.projection.area.radius.x.uv",
        KEY_PROJECTION_AREA_RADIUS_X_UV,
    ),
    property_input(
        "debug.rustyquest.projection.area.radius.y.uv",
        KEY_PROJECTION_AREA_RADIUS_Y_UV,
    ),
    property_input(
        "debug.rustyquest.projection.area.corner.radius.uv",
        KEY_PROJECTION_AREA_CORNER_RADIUS_UV,
    ),
    property_input(
        "debug.rustyquest.projection.area.opacity",
        KEY_PROJECTION_AREA_OPACITY,
    ),
    property_input(
        "debug.rustyquest.projection.border.opacity",
        KEY_PROJECTION_BORDER_OPACITY,
    ),
    property_input(
        "debug.rustyquest.projection.border.policy",
        KEY_PROJECTION_BORDER_POLICY,
    ),
    property_input(
        "debug.rustyquest.projection.target.offset.x.uv",
        KEY_PROJECTION_TARGET_OFFSET_X_UV,
    ),
    property_input(
        "debug.rustyquest.projection.target.offset.y.uv",
        KEY_PROJECTION_TARGET_OFFSET_Y_UV,
    ),
    property_input(
        "debug.rustyquest.projection.target.scale",
        KEY_PROJECTION_TARGET_SCALE,
    ),
    property_input(
        "debug.rustyquest.projection.target.joystick.controls",
        KEY_PROJECTION_TARGET_JOYSTICK_CONTROLS,
    ),
    property_input(
        "debug.rustyquest.projection.target.breath.controls",
        KEY_PROJECTION_TARGET_BREATH_CONTROLS,
    ),
    property_input(
        "debug.rustyquest.projection.target.breath.stream",
        KEY_PROJECTION_TARGET_BREATH_STREAM,
    ),
    property_input(
        "debug.rustyquest.projection.target.breath.min.scale",
        KEY_PROJECTION_TARGET_BREATH_MIN_SCALE,
    ),
    property_input(
        "debug.rustyquest.projection.target.breath.max.scale",
        KEY_PROJECTION_TARGET_BREATH_MAX_SCALE,
    ),
    property_input(
        "debug.rustyquest.projection.target.breath.smoothing.alpha",
        KEY_PROJECTION_TARGET_BREATH_SMOOTHING_ALPHA,
    ),
    property_input(
        "debug.rustyquest.projection.target.breath.invert",
        KEY_PROJECTION_TARGET_BREATH_INVERT,
    ),
    property_input(
        "debug.rustyquest.projection.target.breath.min.quality",
        KEY_PROJECTION_TARGET_BREATH_MIN_QUALITY,
    ),
    property_input("debug.rustyquest.processing.layer", KEY_PROCESSING_LAYER),
    property_input(
        "debug.rustyquest.camera.blur.radius.px",
        KEY_CAMERA_BLUR_RADIUS_PX,
    ),
    property_input(
        "debug.rustyquest.peripheral.stretch.mode",
        KEY_PERIPHERAL_STRETCH_MODE,
    ),
    property_input(
        "debug.rustyquest.peripheral.stretch.core.scale",
        KEY_PERIPHERAL_STRETCH_CORE_SCALE,
    ),
    property_input(
        "debug.rustyquest.peripheral.stretch.edge.inset.uv",
        KEY_PERIPHERAL_STRETCH_EDGE_INSET_UV,
    ),
    property_input(
        "debug.rustyquest.peripheral.stretch.max.inset.uv",
        KEY_PERIPHERAL_STRETCH_MAX_INSET_UV,
    ),
    property_input(
        "debug.rustyquest.peripheral.stretch.curve",
        KEY_PERIPHERAL_STRETCH_CURVE,
    ),
    property_input(
        "debug.rustyquest.peripheral.stretch.inner.blend.uv",
        KEY_PERIPHERAL_STRETCH_INNER_BLEND_UV,
    ),
    property_input(
        "debug.rustyquest.peripheral.stretch.blend.curve",
        KEY_PERIPHERAL_STRETCH_BLEND_CURVE,
    ),
    property_input(
        "debug.rustyquest.peripheral.stretch.blend.mode",
        KEY_PERIPHERAL_STRETCH_BLEND_MODE,
    ),
    property_input(
        "debug.rustyquest.peripheral.stretch.corner.mode",
        KEY_PERIPHERAL_STRETCH_CORNER_MODE,
    ),
    property_input(
        "debug.rustyquest.peripheral.stretch.debug",
        KEY_PERIPHERAL_STRETCH_DEBUG,
    ),
    property_input(
        "debug.rustyquest.projection.alpha.mode",
        KEY_PROJECTION_ALPHA_MODE,
    ),
    property_input(
        "debug.rustyquest.projection.alpha.scale",
        KEY_PROJECTION_ALPHA_SCALE,
    ),
    property_input(
        "debug.rustyquest.projection.alpha.bias",
        KEY_PROJECTION_ALPHA_BIAS,
    ),
    property_input(
        "debug.rustyquest.source.eye.mapping",
        KEY_SOURCE_EYE_MAPPING,
    ),
    property_input(
        "debug.rustyquest.source.texture.transform.source",
        KEY_SOURCE_TEXTURE_TRANSFORM_SOURCE,
    ),
    property_input(
        "debug.rustyquest.source.visible.rect.x.uv",
        KEY_SOURCE_VISIBLE_RECT_X_UV,
    ),
    property_input(
        "debug.rustyquest.source.visible.rect.y.uv",
        KEY_SOURCE_VISIBLE_RECT_Y_UV,
    ),
    property_input(
        "debug.rustyquest.source.visible.rect.width.uv",
        KEY_SOURCE_VISIBLE_RECT_WIDTH_UV,
    ),
    property_input(
        "debug.rustyquest.source.visible.rect.height.uv",
        KEY_SOURCE_VISIBLE_RECT_HEIGHT_UV,
    ),
    env_input(
        "RUSTY_QUEST_MAKEPAD_PROJECTION_DEPTH_METERS",
        KEY_PROJECTION_DEPTH_METERS,
    ),
    env_input(
        "RUSTY_QUEST_MAKEPAD_CAMERA_PREVIEW_FOV_Y_DEGREES",
        KEY_CAMERA_PREVIEW_FOV_Y_DEGREES,
    ),
    env_input(
        "RUSTY_QUEST_MAKEPAD_CAMERA_PREVIEW_OFFSET_Y_METERS",
        KEY_CAMERA_PREVIEW_OFFSET_Y_METERS,
    ),
    env_input(
        "RUSTY_QUEST_MAKEPAD_CAMERA_RAW_OVERLAY_OVERSCAN",
        KEY_CAMERA_RAW_OVERLAY_OVERSCAN,
    ),
];

const fn launch_input(input: &'static str, canonical_key: &'static str) -> RuntimeKeyInputBinding {
    RuntimeKeyInputBinding {
        input,
        canonical_key,
        source: RuntimeKeyInputSource::LaunchExtra,
        value_transform: RuntimeKeyInputValueTransform::Identity,
    }
}

const fn property_input(
    input: &'static str,
    canonical_key: &'static str,
) -> RuntimeKeyInputBinding {
    RuntimeKeyInputBinding {
        input,
        canonical_key,
        source: RuntimeKeyInputSource::AndroidProperty,
        value_transform: RuntimeKeyInputValueTransform::Identity,
    }
}

const fn env_input(input: &'static str, canonical_key: &'static str) -> RuntimeKeyInputBinding {
    RuntimeKeyInputBinding {
        input,
        canonical_key,
        source: RuntimeKeyInputSource::EnvironmentVariable,
        value_transform: RuntimeKeyInputValueTransform::Identity,
    }
}

pub fn projection_runtime_key_definition(key: &str) -> Option<&'static RuntimeKeyDefinition> {
    PROJECTION_RUNTIME_KEY_DEFINITIONS
        .iter()
        .find(|definition| definition.key == key)
}

pub fn projection_runtime_key_input(input: &str) -> Option<&'static RuntimeKeyInputBinding> {
    PROJECTION_RUNTIME_KEY_INPUTS
        .iter()
        .find(|definition| definition.input == input)
}

pub fn resolve_projection_runtime_input(
    input: &str,
) -> Result<RuntimeKeyInputRecord, RuntimeConfigError> {
    if projection_runtime_key_definition(input).is_some() {
        return Ok(RuntimeKeyInputRecord {
            input_key: input.to_string(),
            canonical_key: RuntimeKey::new(input)?,
            source: RuntimeKeyInputSource::Canonical,
            value_transform: RuntimeKeyInputValueTransform::Identity,
        });
    }

    let binding = projection_runtime_key_input(input)
        .ok_or_else(|| RuntimeConfigError::UnknownRuntimeKeyInput(input.to_string()))?;
    Ok(RuntimeKeyInputRecord {
        input_key: input.to_string(),
        canonical_key: binding.canonical_runtime_key(),
        source: binding.source,
        value_transform: binding.value_transform,
    })
}

pub fn parse_projection_runtime_pairs<'a>(
    source: RuntimeConfigSource,
    pairs: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<RuntimeConfigInputParse, RuntimeConfigError> {
    let mut config = RuntimeConfig::new();
    let mut inputs = Vec::new();
    for (input_key, raw_value) in pairs {
        let input = resolve_projection_runtime_input(input_key)?;
        let definition = projection_runtime_key_definition(input.canonical_key.as_str())
            .expect("resolved projection inputs should target registered keys");
        let value =
            RuntimeValue::parse_for_kind(raw_value, definition.value_kind).ok_or_else(|| {
                RuntimeConfigError::InvalidRuntimeInputValue {
                    input: input_key.to_string(),
                    value: raw_value.to_string(),
                }
            })?;
        let value = input.value_transform.apply(input_key, raw_value, value)?;
        config.insert(RuntimeSetting::new(
            input.canonical_key.clone(),
            value,
            source.clone(),
        ));
        inputs.push(input);
    }

    Ok(RuntimeConfigInputParse { config, inputs })
}

/// One runtime-config layer with explicit precedence.
///
/// Higher precedence wins. If two layers use the same precedence for the same
/// key, the later layer wins so callers can keep deterministic local override
/// behavior.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeConfigLayer {
    pub owner: RuntimeConfigOwner,
    pub precedence: u32,
    pub config: RuntimeConfig,
}

impl RuntimeConfigLayer {
    pub fn new(
        owner: impl Into<String>,
        precedence: u32,
        config: RuntimeConfig,
    ) -> Result<Self, RuntimeConfigError> {
        Ok(Self {
            owner: RuntimeConfigOwner::new(owner)?,
            precedence,
            config,
        })
    }
}

/// Candidate value considered during resolution.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeConfigCandidate {
    pub key: RuntimeKey,
    pub value: RuntimeValue,
    pub source: RuntimeConfigSource,
    pub owner: RuntimeConfigOwner,
    pub precedence: u32,
}

impl RuntimeConfigCandidate {
    fn from_setting(setting: &RuntimeSetting, owner: &RuntimeConfigOwner, precedence: u32) -> Self {
        Self {
            key: setting.key.clone(),
            value: setting.value.clone(),
            source: setting.source.clone(),
            owner: owner.clone(),
            precedence,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct OrderedRuntimeConfigCandidate {
    candidate: RuntimeConfigCandidate,
    layer_order: usize,
}

/// Resolved value for one key, including all candidate values.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeResolvedSetting {
    pub key: RuntimeKey,
    pub value: RuntimeValue,
    pub source: RuntimeConfigSource,
    pub owner: RuntimeConfigOwner,
    pub precedence: u32,
    pub default_value: Option<RuntimeValue>,
    pub candidates: Vec<RuntimeConfigCandidate>,
}

impl RuntimeResolvedSetting {
    pub fn overridden_candidates(&self) -> impl Iterator<Item = &RuntimeConfigCandidate> {
        self.candidates
            .iter()
            .filter(move |candidate| !candidate_matches_resolution(candidate, self))
    }
}

/// Full traced resolution for a set of runtime-config layers.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RuntimeConfigResolution {
    resolved: RuntimeConfig,
    settings: BTreeMap<RuntimeKey, RuntimeResolvedSetting>,
}

impl RuntimeConfigResolution {
    pub fn resolved(&self) -> &RuntimeConfig {
        &self.resolved
    }

    pub fn get(&self, key: &str) -> Option<&RuntimeResolvedSetting> {
        self.settings.get(&RuntimeKey::new(key).ok()?)
    }

    pub fn iter(&self) -> impl Iterator<Item = &RuntimeResolvedSetting> {
        self.settings.values()
    }
}

/// Projection-specific resolved runtime config plus input evidence.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProjectionRuntimeConfigResolution {
    pub resolution: RuntimeConfigResolution,
    pub inputs: Vec<RuntimeKeyInputRecord>,
}

impl ProjectionRuntimeConfigResolution {
    pub fn manifest_marker_lines(&self, backend: &str, phase: &str) -> Vec<String> {
        projection_runtime_manifest_marker_lines(backend, phase, &self.resolution, &self.inputs)
    }
}

/// Builder for traced projection runtime config layers.
#[derive(Clone, Debug, Default)]
pub struct ProjectionRuntimeConfigBuilder {
    resolver: RuntimeConfigResolver,
    inputs: Vec<RuntimeKeyInputRecord>,
}

impl ProjectionRuntimeConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_layer(
        &mut self,
        owner: impl Into<String>,
        precedence: u32,
        config: RuntimeConfig,
    ) -> Result<&mut Self, RuntimeConfigError> {
        if config.iter().next().is_some() {
            self.resolver
                .push_layer(RuntimeConfigLayer::new(owner, precedence, config)?);
        }
        Ok(self)
    }

    pub fn with_layer(
        mut self,
        owner: impl Into<String>,
        precedence: u32,
        config: RuntimeConfig,
    ) -> Result<Self, RuntimeConfigError> {
        self.push_layer(owner, precedence, config)?;
        Ok(self)
    }

    pub fn push_inputs(
        &mut self,
        inputs: impl IntoIterator<Item = RuntimeKeyInputRecord>,
    ) -> &mut Self {
        self.inputs.extend(inputs);
        self
    }

    pub fn with_inputs(mut self, inputs: impl IntoIterator<Item = RuntimeKeyInputRecord>) -> Self {
        self.push_inputs(inputs);
        self
    }

    pub fn resolve(self) -> ProjectionRuntimeConfigResolution {
        ProjectionRuntimeConfigResolution {
            resolution: self.resolver.resolve(),
            inputs: self.inputs,
        }
    }
}

pub fn projection_runtime_manifest_marker_lines(
    backend: &str,
    phase: &str,
    resolution: &RuntimeConfigResolution,
    inputs: &[RuntimeKeyInputRecord],
) -> Vec<String> {
    const INPUTS_PER_LINE: usize = 8;
    const FIELDS_PER_LINE: usize = 4;
    let fields = resolution
        .iter()
        .filter(|setting| projection_runtime_key_definition(setting.key.as_str()).is_some())
        .map(projection_runtime_manifest_field_token)
        .collect::<Vec<_>>();
    let input_tokens = projection_runtime_input_tokens(inputs);
    let field_count = fields.len();
    let input_part_count = input_tokens.len().div_ceil(INPUTS_PER_LINE);
    let field_part_count = field_count.div_ceil(FIELDS_PER_LINE).max(1);
    let part_count = input_part_count + field_part_count;
    let mut lines = Vec::new();
    let backend = sanitize_marker_token(backend);
    let phase = sanitize_marker_token(phase);

    for (index, chunk) in input_tokens.chunks(INPUTS_PER_LINE).enumerate() {
        lines.push(format!(
            "RUSTY_QUEST_MAKEPAD_PROJECTION_RUNTIME_MANIFEST schema=rusty.quest.makepad.projection-runtime-manifest.v1 backend={} phase={} part={}/{} section=inputs fieldCount={} inputCount={} inputs={} fields=none",
            backend,
            phase,
            index + 1,
            part_count,
            field_count,
            inputs.len(),
            chunk.join("|")
        ));
    }

    if fields.is_empty() {
        lines.push(format!(
            "RUSTY_QUEST_MAKEPAD_PROJECTION_RUNTIME_MANIFEST schema=rusty.quest.makepad.projection-runtime-manifest.v1 backend={} phase={} part={}/{} section=fields fieldCount=0 inputCount={} inputs={} fields=none",
            backend,
            phase,
            input_part_count + 1,
            part_count,
            inputs.len(),
            if inputs.is_empty() { "none" } else { "see-section-inputs" }
        ));
        return lines;
    }

    for (index, chunk) in fields.chunks(FIELDS_PER_LINE).enumerate() {
        lines.push(format!(
            "RUSTY_QUEST_MAKEPAD_PROJECTION_RUNTIME_MANIFEST schema=rusty.quest.makepad.projection-runtime-manifest.v1 backend={} phase={} part={}/{} section=fields fieldCount={} inputCount={} inputs={} fields={}",
            backend,
            phase,
            input_part_count + index + 1,
            part_count,
            field_count,
            inputs.len(),
            if inputs.is_empty() { "none" } else { "see-section-inputs" },
            chunk.join(";")
        ));
    }

    lines
}

fn projection_runtime_manifest_field_token(setting: &RuntimeResolvedSetting) -> String {
    let default_value = setting
        .default_value
        .as_ref()
        .map(runtime_value_marker_token)
        .unwrap_or_else(|| "none".to_string());
    let candidates = setting
        .candidates
        .iter()
        .map(|candidate| {
            format!(
                "{}:{}:{}:{}",
                candidate.precedence,
                sanitize_marker_token(candidate.owner.as_str()),
                runtime_source_marker_token(&candidate.source),
                runtime_value_marker_token(&candidate.value)
            )
        })
        .collect::<Vec<_>>()
        .join("|");
    format!(
        "{}[owner={},resolved={},source={},default={},candidates={}]",
        setting.key.as_str(),
        sanitize_marker_token(setting.owner.as_str()),
        runtime_value_marker_token(&setting.value),
        runtime_source_marker_token(&setting.source),
        default_value,
        candidates
    )
}

fn projection_runtime_input_tokens(inputs: &[RuntimeKeyInputRecord]) -> Vec<String> {
    inputs
        .iter()
        .map(|input| {
            format!(
                "{}>{}:{}:{}",
                sanitize_marker_token(&input.input_key),
                input.canonical_key.as_str(),
                input_source_marker_token(input.source),
                input_transform_marker_token(input.value_transform)
            )
        })
        .collect::<Vec<_>>()
}

fn runtime_value_marker_token(value: &RuntimeValue) -> String {
    match value {
        RuntimeValue::Bool(value) => format!("bool:{value}"),
        RuntimeValue::Integer(value) => format!("int:{value}"),
        RuntimeValue::Float(value) => format!("float:{value:.6}"),
        RuntimeValue::Text(value) => format!("text:{}", sanitize_marker_token(value)),
    }
}

fn runtime_source_marker_token(source: &RuntimeConfigSource) -> &'static str {
    match source {
        RuntimeConfigSource::Default => "default",
        RuntimeConfigSource::Environment => "environment",
        RuntimeConfigSource::AndroidProperty => "android-property",
        RuntimeConfigSource::File => "file",
        RuntimeConfigSource::CommandLine => "command-line",
        RuntimeConfigSource::Synthetic => "synthetic",
    }
}

fn input_source_marker_token(source: RuntimeKeyInputSource) -> &'static str {
    match source {
        RuntimeKeyInputSource::Canonical => "canonical",
        RuntimeKeyInputSource::LaunchExtra => "launch-extra",
        RuntimeKeyInputSource::AndroidProperty => "android-property",
        RuntimeKeyInputSource::EnvironmentVariable => "environment-variable",
    }
}

fn input_transform_marker_token(transform: RuntimeKeyInputValueTransform) -> &'static str {
    match transform {
        RuntimeKeyInputValueTransform::Identity => "identity",
        RuntimeKeyInputValueTransform::NegateNumber => "negate-number",
    }
}

fn sanitize_marker_token(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':' | '/') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

/// Incremental resolver for layered runtime configuration.
#[derive(Clone, Debug, Default)]
pub struct RuntimeConfigResolver {
    layers: Vec<RuntimeConfigLayer>,
}

impl RuntimeConfigResolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_layer(&mut self, layer: RuntimeConfigLayer) {
        self.layers.push(layer);
    }

    pub fn with_layer(mut self, layer: RuntimeConfigLayer) -> Self {
        self.push_layer(layer);
        self
    }

    pub fn resolve(&self) -> RuntimeConfigResolution {
        let mut by_key: BTreeMap<RuntimeKey, Vec<OrderedRuntimeConfigCandidate>> = BTreeMap::new();
        for (layer_order, layer) in self.layers.iter().enumerate() {
            for setting in layer.config.iter() {
                by_key.entry(setting.key.clone()).or_default().push(
                    OrderedRuntimeConfigCandidate {
                        candidate: RuntimeConfigCandidate::from_setting(
                            setting,
                            &layer.owner,
                            layer.precedence,
                        ),
                        layer_order,
                    },
                );
            }
        }

        let mut resolved = RuntimeConfig::new();
        let mut settings = BTreeMap::new();
        for (key, mut candidates) in by_key {
            candidates.sort_by(|left, right| {
                right
                    .candidate
                    .precedence
                    .cmp(&left.candidate.precedence)
                    .then_with(|| right.layer_order.cmp(&left.layer_order))
            });

            let winner = candidates
                .first()
                .expect("candidate vector should not be empty")
                .candidate
                .clone();
            let default_value = candidates
                .iter()
                .find(|candidate| candidate.candidate.source == RuntimeConfigSource::Default)
                .map(|candidate| candidate.candidate.value.clone());
            let public_candidates = candidates
                .into_iter()
                .map(|candidate| candidate.candidate)
                .collect::<Vec<_>>();
            let setting = RuntimeResolvedSetting {
                key: key.clone(),
                value: winner.value.clone(),
                source: winner.source.clone(),
                owner: winner.owner.clone(),
                precedence: winner.precedence,
                default_value,
                candidates: public_candidates,
            };

            resolved.insert(RuntimeSetting::new(
                key.clone(),
                winner.value,
                winner.source,
            ));
            settings.insert(key, setting);
        }

        RuntimeConfigResolution { resolved, settings }
    }
}

/// Runtime configuration parsing error.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeConfigError {
    InvalidKey(String),
    InvalidOwner(String),
    InvalidAndroidPropertyPrefix(String),
    UnknownRuntimeKeyInput(String),
    InvalidRuntimeInputValue { input: String, value: String },
}

impl fmt::Display for RuntimeConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKey(value) => write!(f, "invalid runtime config key: {value}"),
            Self::InvalidOwner(value) => write!(f, "invalid runtime config owner: {value}"),
            Self::InvalidAndroidPropertyPrefix(value) => {
                write!(f, "invalid Android property prefix: {value}")
            }
            Self::UnknownRuntimeKeyInput(value) => {
                write!(f, "unknown runtime config key input: {value}")
            }
            Self::InvalidRuntimeInputValue { input, value } => {
                write!(f, "invalid value for runtime config input {input}: {value}")
            }
        }
    }
}

impl std::error::Error for RuntimeConfigError {}

fn validate_key(value: &str) -> Result<(), RuntimeConfigError> {
    if value.is_empty() {
        return Err(RuntimeConfigError::InvalidKey(value.to_string()));
    }

    let mut previous_was_separator = false;
    for byte in value.bytes() {
        let is_valid =
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-';
        if !is_valid {
            return Err(RuntimeConfigError::InvalidKey(value.to_string()));
        }
        let is_separator = byte == b'_' || byte == b'-';
        if is_separator && previous_was_separator {
            return Err(RuntimeConfigError::InvalidKey(value.to_string()));
        }
        previous_was_separator = is_separator;
    }

    Ok(())
}

fn validate_owner(value: &str) -> Result<(), RuntimeConfigError> {
    if value.is_empty() {
        return Err(RuntimeConfigError::InvalidOwner(value.to_string()));
    }

    for byte in value.bytes() {
        let is_valid = byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || byte == b'_'
            || byte == b'-'
            || byte == b'.';
        if !is_valid {
            return Err(RuntimeConfigError::InvalidOwner(value.to_string()));
        }
    }

    Ok(())
}

fn candidate_matches_resolution(
    candidate: &RuntimeConfigCandidate,
    setting: &RuntimeResolvedSetting,
) -> bool {
    candidate.key == setting.key
        && candidate.value == setting.value
        && candidate.source == setting.source
        && candidate.owner == setting.owner
        && candidate.precedence == setting.precedence
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" | "enabled" => Some(true),
        "0" | "false" | "no" | "off" | "disabled" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_workspace_version() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn parses_typed_runtime_values() {
        assert_eq!(RuntimeValue::parse_typed("on"), RuntimeValue::Bool(true));
        assert_eq!(RuntimeValue::parse_typed("42"), RuntimeValue::Integer(42));
        assert_eq!(RuntimeValue::parse_typed("0.25"), RuntimeValue::Float(0.25));
        assert_eq!(
            RuntimeValue::parse_typed("balanced"),
            RuntimeValue::Text("balanced".to_string())
        );
    }

    #[test]
    fn rejects_private_or_invalid_key_shapes() {
        assert!(RuntimeKey::new("render_scale").is_ok());
        assert!(RuntimeKey::new("debug.example.render_scale").is_err());
        assert!(RuntimeKey::new("RenderScale").is_err());
    }

    #[test]
    fn builds_generic_android_property_name() {
        let key = RuntimeKey::new("render_scale").expect("key should be valid");
        let prefix = AndroidPropertyPrefix::default();

        assert_eq!(
            key.android_property(&prefix),
            "debug.rustyquest.render.scale"
        );
    }

    #[test]
    fn android_property_normalizes_public_key_separators() {
        let key = RuntimeKey::new("render-scale").expect("key should be valid");
        let prefix = AndroidPropertyPrefix::default();

        assert_eq!(
            key.android_property(&prefix),
            "debug.rustyquest.render.scale"
        );
    }

    #[test]
    fn stores_ordered_runtime_settings() {
        let config = RuntimeConfig::parse_pairs(
            RuntimeConfigSource::Synthetic,
            [("z_value", "9"), ("a_value", "true")],
        )
        .expect("pairs should parse");

        let keys = config
            .iter()
            .map(|setting| setting.key.as_str())
            .collect::<Vec<_>>();

        assert_eq!(keys, ["a_value", "z_value"]);
        assert_eq!(config.get("a_value"), Some(&RuntimeValue::Bool(true)));
    }

    #[test]
    fn projection_runtime_registry_keys_are_valid_and_unique() {
        let mut seen = BTreeMap::new();
        for definition in PROJECTION_RUNTIME_KEY_DEFINITIONS {
            assert_eq!(definition.runtime_key().as_str(), definition.key);
            assert!(
                seen.insert(definition.key, definition.owner).is_none(),
                "duplicate projection key {}",
                definition.key
            );
        }
    }

    #[test]
    fn projection_runtime_inputs_resolve_to_registered_keys() {
        let mut seen = BTreeMap::new();
        for input in PROJECTION_RUNTIME_KEY_INPUTS {
            assert!(
                projection_runtime_key_definition(input.canonical_key).is_some(),
                "input {} targets unregistered key {}",
                input.input,
                input.canonical_key
            );
            assert!(
                seen.insert(input.input, input.canonical_key).is_none(),
                "duplicate input {}",
                input.input
            );
        }

        let launch = resolve_projection_runtime_input("rustyquest.projectionDepthMeters")
            .expect("launch extra input should resolve");
        assert_eq!(launch.canonical_key.as_str(), KEY_PROJECTION_DEPTH_METERS);
        assert_eq!(launch.source, RuntimeKeyInputSource::LaunchExtra);

        let property = resolve_projection_runtime_input("debug.rustyquest.projection.depth.meters")
            .expect("Android property input should resolve");
        assert_eq!(property.canonical_key.as_str(), KEY_PROJECTION_DEPTH_METERS);
        assert_eq!(property.source, RuntimeKeyInputSource::AndroidProperty);
    }

    #[test]
    fn projection_runtime_keeps_source_kind_out_of_projection_math() {
        let source_sampling_keys = PROJECTION_RUNTIME_KEY_DEFINITIONS
            .iter()
            .filter(|definition| definition.owner == ProjectionRuntimeKeyOwner::SourceSampling)
            .map(|definition| definition.key)
            .collect::<Vec<_>>();

        assert_eq!(
            source_sampling_keys,
            [
                KEY_SOURCE_EYE_MAPPING,
                KEY_SOURCE_TEXTURE_ROTATION,
                KEY_SOURCE_TEXTURE_FLIP_X,
                KEY_SOURCE_TEXTURE_FLIP_Y,
                KEY_SOURCE_TEXTURE_MIRROR,
                KEY_SOURCE_TEXTURE_TRANSFORM_SOURCE,
                KEY_SOURCE_TEXTURE_TRANSFORM_REASON,
                KEY_LEFT_SOURCE_TEXTURE_TRANSFORM_SOURCE,
                KEY_RIGHT_SOURCE_TEXTURE_TRANSFORM_SOURCE,
                KEY_SOURCE_VISIBLE_RECT_X_UV,
                KEY_SOURCE_VISIBLE_RECT_Y_UV,
                KEY_SOURCE_VISIBLE_RECT_WIDTH_UV,
                KEY_SOURCE_VISIBLE_RECT_HEIGHT_UV,
            ]
        );
        assert!(projection_runtime_key_definition("source_kind").is_none());
        assert!(projection_runtime_key_definition("camera_source_kind").is_none());
        assert!(projection_runtime_key_definition("synthetic_source_kind").is_none());
    }

    #[test]
    fn parses_projection_runtime_pairs_with_input_evidence() {
        let parsed = parse_projection_runtime_pairs(
            RuntimeConfigSource::CommandLine,
            [
                ("rustyquest.projectionDepthMeters", "1.25"),
                ("source_eye_mapping", "right-left"),
                ("debug.rustyquest.source.visible.rect.width.uv", "0.875"),
            ],
        )
        .expect("projection pairs should parse");

        assert_eq!(
            parsed.config.get(KEY_PROJECTION_DEPTH_METERS),
            Some(&RuntimeValue::Float(1.25))
        );
        assert_eq!(
            parsed.config.get(KEY_SOURCE_EYE_MAPPING),
            Some(&RuntimeValue::Text("right-left".to_string()))
        );
        assert_eq!(
            parsed.config.get(KEY_SOURCE_VISIBLE_RECT_WIDTH_UV),
            Some(&RuntimeValue::Float(0.875))
        );
        assert_eq!(parsed.inputs.len(), 3);
        assert_eq!(parsed.inputs[0].source, RuntimeKeyInputSource::LaunchExtra);
        assert_eq!(parsed.inputs[1].source, RuntimeKeyInputSource::Canonical);
        assert_eq!(
            parsed.inputs[2].source,
            RuntimeKeyInputSource::AndroidProperty
        );
    }

    #[test]
    fn parses_projection_input_values_using_registered_value_kind() {
        let parsed = parse_projection_runtime_pairs(
            RuntimeConfigSource::AndroidProperty,
            [
                ("debug.rustyquest.projection.depth.meters", "1"),
                ("debug.rustyquest.camera.preview.offset.y.meters", "0"),
                ("rustyquest.cameraTextureFlipX", "true"),
                ("debug.rustyquest.projection.alpha.mode", "source-alpha"),
            ],
        )
        .expect("registered projection value kinds should parse");

        assert_eq!(
            parsed.config.get(KEY_PROJECTION_DEPTH_METERS),
            Some(&RuntimeValue::Float(1.0))
        );
        assert_eq!(
            parsed.config.get(KEY_CAMERA_PREVIEW_OFFSET_Y_METERS),
            Some(&RuntimeValue::Float(0.0))
        );
        assert_eq!(
            parsed.config.get(KEY_SOURCE_TEXTURE_FLIP_X),
            Some(&RuntimeValue::Bool(true))
        );
        assert_eq!(
            parsed.config.get(KEY_PROJECTION_ALPHA_MODE),
            Some(&RuntimeValue::Text("source-alpha".to_string()))
        );
    }

    #[test]
    fn rejects_removed_makepad_directional_offset_inputs() {
        let error = parse_projection_runtime_pairs(
            RuntimeConfigSource::AndroidProperty,
            [("debug.rustyquest.projection.area.offset.left.uv", "0.125")],
        )
        .unwrap_err();

        assert_eq!(
            error,
            RuntimeConfigError::UnknownRuntimeKeyInput(
                "debug.rustyquest.projection.area.offset.left.uv".to_string()
            )
        );
    }

    #[test]
    fn rejects_unknown_projection_runtime_inputs() {
        assert_eq!(
            resolve_projection_runtime_input("rustyquest.privateEffectStrength").unwrap_err(),
            RuntimeConfigError::UnknownRuntimeKeyInput(
                "rustyquest.privateEffectStrength".to_string()
            )
        );
    }

    #[test]
    fn projection_manifest_marker_lines_include_resolution_trace() {
        let defaults = parse_projection_runtime_pairs(
            RuntimeConfigSource::Default,
            [("projection_depth_meters", "1.0")],
        )
        .expect("defaults should parse");
        let requested = parse_projection_runtime_pairs(
            RuntimeConfigSource::AndroidProperty,
            [("debug.rustyquest.projection.depth.meters", "1.5")],
        )
        .expect("properties should parse");
        let resolution = RuntimeConfigResolver::new()
            .with_layer(RuntimeConfigLayer::new("backend-defaults", 0, defaults.config).unwrap())
            .with_layer(
                RuntimeConfigLayer::new("android-properties", 20, requested.config).unwrap(),
            )
            .resolve();
        let lines = projection_runtime_manifest_marker_lines(
            "oes",
            "startup",
            &resolution,
            &requested.inputs,
        );

        assert_eq!(lines.len(), 2);
        let joined = lines.join("\n");
        assert!(joined.contains("schema=rusty.quest.makepad.projection-runtime-manifest.v1"));
        assert!(joined.contains("backend=oes"));
        assert!(joined.contains("section=inputs"));
        assert!(joined.contains("section=fields"));
        assert!(joined.contains("projection_depth_meters"));
        assert!(joined.contains("resolved=float:1.500000"));
        assert!(joined.contains("default=float:1.000000"));
        assert!(joined.contains("debug.rustyquest.projection.depth.meters>projection_depth_meters"));
    }

    #[test]
    fn projection_runtime_builder_collects_layers_and_inputs() {
        let defaults = parse_projection_runtime_pairs(
            RuntimeConfigSource::Default,
            [("projection_depth_meters", "1.0")],
        )
        .expect("defaults should parse");
        let requested = parse_projection_runtime_pairs(
            RuntimeConfigSource::AndroidProperty,
            [("debug.rustyquest.projection.depth.meters", "1.25")],
        )
        .expect("properties should parse");
        let runtime = ProjectionRuntimeConfigBuilder::new()
            .with_layer("backend-defaults", 0, defaults.config)
            .expect("default layer should be valid")
            .with_layer("android-properties", 20, requested.config)
            .expect("property layer should be valid")
            .with_inputs(requested.inputs)
            .resolve();

        assert_eq!(
            runtime
                .resolution
                .resolved()
                .get(KEY_PROJECTION_DEPTH_METERS),
            Some(&RuntimeValue::Float(1.25))
        );
        assert_eq!(runtime.inputs.len(), 1);
        let lines = runtime.manifest_marker_lines("oes", "startup");
        let joined = lines.join("\n");
        assert!(joined.contains("backend=oes"));
        assert!(joined.contains("owner=android-properties"));
        assert!(joined.contains("resolved=float:1.250000"));
    }

    fn projection_runtime_golden_snapshot(
        backend: &str,
        launch_pairs: &[(&'static str, &'static str)],
        property_pairs: &[(&'static str, &'static str)],
    ) -> Vec<(&'static str, RuntimeValue)> {
        let defaults = parse_projection_runtime_pairs(
            RuntimeConfigSource::Default,
            [
                (KEY_CAMERA_PROJECTION_MODE, "display-screen-homography"),
                (KEY_PROJECTION_DEPTH_METERS, "1.0"),
                (KEY_CAMERA_PREVIEW_FOV_Y_DEGREES, "60.0"),
                (KEY_CAMERA_PREVIEW_OFFSET_Y_METERS, "0.0"),
                (KEY_CAMERA_RAW_OVERLAY_OVERSCAN, "1.06"),
                (KEY_PROJECTION_AREA_SCALE_X, "1.0"),
                (KEY_PROJECTION_AREA_SCALE_Y, "1.0"),
                (KEY_PROJECTION_AREA_OFFSET_X_UV, "0.0"),
                (KEY_PROJECTION_AREA_OFFSET_Y_UV, "0.0"),
                (KEY_PROJECTION_AREA_LEFT_OFFSET_X_UV, "0.0"),
                (KEY_PROJECTION_AREA_RIGHT_OFFSET_X_UV, "0.0"),
                (KEY_PROJECTION_AREA_RADIUS_X_UV, "0.47"),
                (KEY_PROJECTION_AREA_RADIUS_Y_UV, "0.36"),
                (KEY_PROJECTION_AREA_OPACITY, "1.0"),
                (KEY_PROJECTION_BORDER_OPACITY, "1.0"),
                (KEY_PROJECTION_BORDER_POLICY, "solid-red"),
                (KEY_PROJECTION_TARGET_OFFSET_X_UV, "0.0"),
                (KEY_PROJECTION_TARGET_OFFSET_Y_UV, "0.0"),
                (KEY_PROJECTION_TARGET_SCALE, "1.0"),
                (KEY_PROJECTION_TARGET_JOYSTICK_CONTROLS, "off"),
                (KEY_PROJECTION_TARGET_BREATH_CONTROLS, "off"),
                (KEY_PROJECTION_TARGET_BREATH_STREAM, "bio:breath"),
                (KEY_PROJECTION_TARGET_BREATH_MIN_SCALE, "0.75"),
                (KEY_PROJECTION_TARGET_BREATH_MAX_SCALE, "1.15"),
                (KEY_PROJECTION_TARGET_BREATH_SMOOTHING_ALPHA, "0.25"),
                (KEY_PROJECTION_TARGET_BREATH_INVERT, "false"),
                (KEY_PROJECTION_TARGET_BREATH_MIN_QUALITY, "0.0"),
                (KEY_PROJECTION_ALPHA_MODE, "fixed"),
                (KEY_PROJECTION_ALPHA_SCALE, "1.0"),
                (KEY_PROJECTION_ALPHA_BIAS, "0.0"),
                (KEY_SOURCE_EYE_MAPPING, "left-right"),
                (KEY_SOURCE_TEXTURE_TRANSFORM_SOURCE, "metadata"),
                (KEY_SOURCE_VISIBLE_RECT_X_UV, "0.0"),
                (KEY_SOURCE_VISIBLE_RECT_Y_UV, "0.0"),
                (KEY_SOURCE_VISIBLE_RECT_WIDTH_UV, "1.0"),
                (KEY_SOURCE_VISIBLE_RECT_HEIGHT_UV, "1.0"),
            ],
        )
        .expect("golden defaults should parse");
        let launch = parse_projection_runtime_pairs(
            RuntimeConfigSource::CommandLine,
            launch_pairs.iter().copied(),
        )
        .expect("golden launch pairs should parse");
        let properties = parse_projection_runtime_pairs(
            RuntimeConfigSource::AndroidProperty,
            property_pairs.iter().copied(),
        )
        .expect("golden property pairs should parse");
        let runtime = ProjectionRuntimeConfigBuilder::new()
            .with_layer(format!("{backend}-defaults"), 0, defaults.config)
            .expect("default layer should be valid")
            .with_layer(format!("{backend}-launch"), 10, launch.config)
            .expect("launch layer should be valid")
            .with_layer(format!("{backend}-properties"), 20, properties.config)
            .expect("property layer should be valid")
            .with_inputs(launch.inputs)
            .with_inputs(properties.inputs)
            .resolve();
        [
            KEY_CAMERA_PROJECTION_MODE,
            KEY_PROJECTION_DEPTH_METERS,
            KEY_CAMERA_PREVIEW_FOV_Y_DEGREES,
            KEY_CAMERA_PREVIEW_OFFSET_Y_METERS,
            KEY_CAMERA_RAW_OVERLAY_OVERSCAN,
            KEY_PROJECTION_AREA_SCALE_X,
            KEY_PROJECTION_AREA_SCALE_Y,
            KEY_PROJECTION_AREA_OFFSET_X_UV,
            KEY_PROJECTION_AREA_OFFSET_Y_UV,
            KEY_PROJECTION_AREA_LEFT_OFFSET_X_UV,
            KEY_PROJECTION_AREA_RIGHT_OFFSET_X_UV,
            KEY_PROJECTION_AREA_RADIUS_X_UV,
            KEY_PROJECTION_AREA_RADIUS_Y_UV,
            KEY_PROJECTION_AREA_OPACITY,
            KEY_PROJECTION_BORDER_OPACITY,
            KEY_PROJECTION_BORDER_POLICY,
            KEY_PROJECTION_TARGET_OFFSET_X_UV,
            KEY_PROJECTION_TARGET_OFFSET_Y_UV,
            KEY_PROJECTION_TARGET_SCALE,
            KEY_PROJECTION_TARGET_JOYSTICK_CONTROLS,
            KEY_PROJECTION_TARGET_BREATH_CONTROLS,
            KEY_PROJECTION_TARGET_BREATH_STREAM,
            KEY_PROJECTION_TARGET_BREATH_MIN_SCALE,
            KEY_PROJECTION_TARGET_BREATH_MAX_SCALE,
            KEY_PROJECTION_TARGET_BREATH_SMOOTHING_ALPHA,
            KEY_PROJECTION_TARGET_BREATH_INVERT,
            KEY_PROJECTION_TARGET_BREATH_MIN_QUALITY,
            KEY_PROJECTION_ALPHA_MODE,
            KEY_PROJECTION_ALPHA_SCALE,
            KEY_PROJECTION_ALPHA_BIAS,
            KEY_SOURCE_EYE_MAPPING,
            KEY_SOURCE_TEXTURE_TRANSFORM_SOURCE,
            KEY_SOURCE_VISIBLE_RECT_X_UV,
            KEY_SOURCE_VISIBLE_RECT_Y_UV,
            KEY_SOURCE_VISIBLE_RECT_WIDTH_UV,
            KEY_SOURCE_VISIBLE_RECT_HEIGHT_UV,
        ]
        .into_iter()
        .map(|key| {
            (
                key,
                runtime
                    .resolution
                    .resolved()
                    .get(key)
                    .unwrap_or_else(|| panic!("{key} should resolve"))
                    .clone(),
            )
        })
        .collect()
    }

    #[test]
    fn projection_runtime_golden_matrix_is_backend_neutral_for_equivalent_metadata() {
        let launch_input_snapshot = projection_runtime_golden_snapshot(
            "hwb",
            &[
                (
                    "rustyquest.cameraProjectionMode",
                    "display-screen-homography",
                ),
                ("rustyquest.projectionDepthMeters", "1.25"),
                ("rustyquest.cameraPreviewFovYDegrees", "63.0"),
                ("rustyquest.cameraPreviewOffsetYMeters", "0.08"),
                ("rustyquest.cameraRawOverlayOverscan", "1.12"),
                ("rustyquest.projectionAreaScaleX", "0.82"),
                ("rustyquest.projectionAreaScaleY", "0.74"),
                ("rustyquest.projectionAreaOffsetXUv", "0.03"),
                ("rustyquest.projectionAreaOffsetYUv", "-0.02"),
                ("rustyquest.projectionAreaLeftOffsetXUv", "-0.04"),
                ("rustyquest.projectionAreaRightOffsetXUv", "0.04"),
                ("rustyquest.projectionAreaRadiusXUv", "0.44"),
                ("rustyquest.projectionAreaRadiusYUv", "0.31"),
                ("rustyquest.projectionAreaOpacity", "0.90"),
                ("rustyquest.projectionBorderOpacity", "0.80"),
                ("rustyquest.projectionBorderPolicy", "solid-red"),
                ("rustyquest.projectionTargetOffsetXUv", "0.05"),
                ("rustyquest.projectionTargetOffsetYUv", "-0.03"),
                ("rustyquest.projectionTargetScale", "0.80"),
                (
                    "rustyquest.projectionTargetJoystickControls",
                    "offset-scale",
                ),
                ("rustyquest.projectionTargetBreathControls", "scale"),
                ("rustyquest.projectionTargetBreathStream", "bio:breath"),
                ("rustyquest.projectionTargetBreathMinScale", "0.70"),
                ("rustyquest.projectionTargetBreathMaxScale", "1.20"),
                ("rustyquest.projectionTargetBreathSmoothingAlpha", "0.30"),
                ("rustyquest.projectionTargetBreathInvert", "true"),
                ("rustyquest.projectionTargetBreathMinQuality", "0.20"),
                ("rustyquest.projectionAlphaMode", "fixed"),
                ("rustyquest.projectionAlphaScale", "1.10"),
                ("rustyquest.projectionAlphaBias", "-0.05"),
                ("rustyquest.cameraSourceEyeMapping", "left-right"),
                ("rustyquest.cameraTextureTransformSource", "metadata"),
                (KEY_SOURCE_VISIBLE_RECT_X_UV, "0.10"),
                (KEY_SOURCE_VISIBLE_RECT_Y_UV, "0.20"),
                (KEY_SOURCE_VISIBLE_RECT_WIDTH_UV, "0.80"),
                (KEY_SOURCE_VISIBLE_RECT_HEIGHT_UV, "0.60"),
            ],
            &[],
        );
        let canonical_snapshot = projection_runtime_golden_snapshot(
            "oes",
            &[
                (KEY_CAMERA_PROJECTION_MODE, "display-screen-homography"),
                (KEY_PROJECTION_DEPTH_METERS, "1.25"),
                (KEY_CAMERA_PREVIEW_FOV_Y_DEGREES, "63.0"),
                (KEY_CAMERA_PREVIEW_OFFSET_Y_METERS, "0.08"),
                (KEY_CAMERA_RAW_OVERLAY_OVERSCAN, "1.12"),
                (KEY_PROJECTION_AREA_SCALE_X, "0.82"),
                (KEY_PROJECTION_AREA_SCALE_Y, "0.74"),
                (KEY_PROJECTION_AREA_OFFSET_X_UV, "0.03"),
                (KEY_PROJECTION_AREA_OFFSET_Y_UV, "-0.02"),
                (KEY_PROJECTION_AREA_LEFT_OFFSET_X_UV, "-0.04"),
                (KEY_PROJECTION_AREA_RIGHT_OFFSET_X_UV, "0.04"),
                (KEY_PROJECTION_AREA_RADIUS_X_UV, "0.44"),
                (KEY_PROJECTION_AREA_RADIUS_Y_UV, "0.31"),
                (KEY_PROJECTION_AREA_OPACITY, "0.90"),
                (KEY_PROJECTION_BORDER_OPACITY, "0.80"),
                (KEY_PROJECTION_BORDER_POLICY, "solid-red"),
                (KEY_PROJECTION_TARGET_OFFSET_X_UV, "0.05"),
                (KEY_PROJECTION_TARGET_OFFSET_Y_UV, "-0.03"),
                (KEY_PROJECTION_TARGET_SCALE, "0.80"),
                (KEY_PROJECTION_TARGET_JOYSTICK_CONTROLS, "offset-scale"),
                (KEY_PROJECTION_TARGET_BREATH_CONTROLS, "scale"),
                (KEY_PROJECTION_TARGET_BREATH_STREAM, "bio:breath"),
                (KEY_PROJECTION_TARGET_BREATH_MIN_SCALE, "0.70"),
                (KEY_PROJECTION_TARGET_BREATH_MAX_SCALE, "1.20"),
                (KEY_PROJECTION_TARGET_BREATH_SMOOTHING_ALPHA, "0.30"),
                (KEY_PROJECTION_TARGET_BREATH_INVERT, "true"),
                (KEY_PROJECTION_TARGET_BREATH_MIN_QUALITY, "0.20"),
                (KEY_PROJECTION_ALPHA_MODE, "fixed"),
                (KEY_PROJECTION_ALPHA_SCALE, "1.10"),
                (KEY_PROJECTION_ALPHA_BIAS, "-0.05"),
                (KEY_SOURCE_EYE_MAPPING, "left-right"),
                (KEY_SOURCE_TEXTURE_TRANSFORM_SOURCE, "metadata"),
                (KEY_SOURCE_VISIBLE_RECT_X_UV, "0.10"),
                (KEY_SOURCE_VISIBLE_RECT_Y_UV, "0.20"),
                (KEY_SOURCE_VISIBLE_RECT_WIDTH_UV, "0.80"),
                (KEY_SOURCE_VISIBLE_RECT_HEIGHT_UV, "0.60"),
            ],
            &[],
        );
        let property_snapshot = projection_runtime_golden_snapshot(
            "makepad",
            &[],
            &[
                (
                    "debug.rustyquest.camera.projection.mode",
                    "display-screen-homography",
                ),
                ("debug.rustyquest.projection.depth.meters", "1.25"),
                ("debug.rustyquest.camera.preview.fov.y.degrees", "63.0"),
                ("debug.rustyquest.camera.preview.offset.y.meters", "0.08"),
                ("debug.rustyquest.camera.raw.overlay.overscan", "1.12"),
                ("debug.rustyquest.projection.area.scale.x", "0.82"),
                ("debug.rustyquest.projection.area.scale.y", "0.74"),
                ("debug.rustyquest.projection.area.offset.x.uv", "0.03"),
                ("debug.rustyquest.projection.area.offset.y.uv", "-0.02"),
                ("debug.rustyquest.projection.area.left.offset.x.uv", "-0.04"),
                ("debug.rustyquest.projection.area.right.offset.x.uv", "0.04"),
                ("debug.rustyquest.projection.area.radius.x.uv", "0.44"),
                ("debug.rustyquest.projection.area.radius.y.uv", "0.31"),
                ("debug.rustyquest.projection.area.opacity", "0.90"),
                ("debug.rustyquest.projection.border.opacity", "0.80"),
                ("debug.rustyquest.projection.border.policy", "solid-red"),
                ("debug.rustyquest.projection.target.offset.x.uv", "0.05"),
                ("debug.rustyquest.projection.target.offset.y.uv", "-0.03"),
                ("debug.rustyquest.projection.target.scale", "0.80"),
                (
                    "debug.rustyquest.projection.target.joystick.controls",
                    "offset-scale",
                ),
                (
                    "debug.rustyquest.projection.target.breath.controls",
                    "scale",
                ),
                (
                    "debug.rustyquest.projection.target.breath.stream",
                    "bio:breath",
                ),
                (
                    "debug.rustyquest.projection.target.breath.min.scale",
                    "0.70",
                ),
                (
                    "debug.rustyquest.projection.target.breath.max.scale",
                    "1.20",
                ),
                (
                    "debug.rustyquest.projection.target.breath.smoothing.alpha",
                    "0.30",
                ),
                ("debug.rustyquest.projection.target.breath.invert", "true"),
                (
                    "debug.rustyquest.projection.target.breath.min.quality",
                    "0.20",
                ),
                ("debug.rustyquest.projection.alpha.mode", "fixed"),
                ("debug.rustyquest.projection.alpha.scale", "1.10"),
                ("debug.rustyquest.projection.alpha.bias", "-0.05"),
                ("debug.rustyquest.source.eye.mapping", "left-right"),
                (
                    "debug.rustyquest.source.texture.transform.source",
                    "metadata",
                ),
                ("debug.rustyquest.source.visible.rect.x.uv", "0.10"),
                ("debug.rustyquest.source.visible.rect.y.uv", "0.20"),
                ("debug.rustyquest.source.visible.rect.width.uv", "0.80"),
                ("debug.rustyquest.source.visible.rect.height.uv", "0.60"),
            ],
        );

        assert_eq!(launch_input_snapshot, canonical_snapshot);
        assert_eq!(canonical_snapshot, property_snapshot);
        assert!(
            parse_projection_runtime_pairs(
                RuntimeConfigSource::CommandLine,
                [("source_kind", "synthetic")]
            )
            .is_err(),
            "source kind must not become a projection-runtime key"
        );
    }

    #[test]
    fn projection_runtime_golden_matrix_covers_canvas_and_underlay_profiles() {
        let canvas = projection_runtime_golden_snapshot(
            "hwb",
            &[
                ("rustyquest.cameraProjectionMode", "world-canvas"),
                ("rustyquest.projectionAreaScaleX", "0.70"),
                ("rustyquest.projectionAreaScaleY", "0.55"),
                ("rustyquest.projectionBorderPolicy", "solid-red"),
            ],
            &[],
        );
        assert!(canvas.contains(&(
            KEY_CAMERA_PROJECTION_MODE,
            RuntimeValue::Text("world-canvas".to_string())
        )));
        assert!(canvas.contains(&(
            KEY_PROJECTION_BORDER_POLICY,
            RuntimeValue::Text("solid-red".to_string())
        )));

        let underlay = projection_runtime_golden_snapshot(
            "oes",
            &[
                ("rustyquest.projectionBorderPolicy", "passthrough-underlay"),
                ("rustyquest.projectionAreaOpacity", "0.65"),
                ("rustyquest.projectionBorderOpacity", "0.0"),
                ("rustyquest.projectionAlphaMode", "luma"),
            ],
            &[],
        );
        assert!(underlay.contains(&(
            KEY_PROJECTION_BORDER_POLICY,
            RuntimeValue::Text("passthrough-underlay".to_string())
        )));
        assert!(underlay.contains(&(KEY_PROJECTION_AREA_OPACITY, RuntimeValue::Float(0.65))));
        assert!(underlay.contains(&(KEY_PROJECTION_BORDER_OPACITY, RuntimeValue::Float(0.0))));
        assert!(underlay.contains(&(
            KEY_PROJECTION_ALPHA_MODE,
            RuntimeValue::Text("luma".to_string())
        )));
    }

    #[test]
    fn resolves_layered_runtime_config_with_trace() {
        let defaults = RuntimeConfig::parse_pairs(
            RuntimeConfigSource::Default,
            [
                ("projection_depth_meters", "1.0"),
                ("projection_area_scale_x", "1.0"),
            ],
        )
        .expect("defaults should parse");
        let launch = RuntimeConfig::parse_pairs(
            RuntimeConfigSource::CommandLine,
            [("projection_depth_meters", "1.25")],
        )
        .expect("launch values should parse");
        let properties = RuntimeConfig::parse_pairs(
            RuntimeConfigSource::AndroidProperty,
            [("projection_depth_meters", "1.5")],
        )
        .expect("property values should parse");

        let resolution = RuntimeConfigResolver::new()
            .with_layer(RuntimeConfigLayer::new("backend-defaults", 0, defaults).unwrap())
            .with_layer(RuntimeConfigLayer::new("launch-profile", 10, launch).unwrap())
            .with_layer(RuntimeConfigLayer::new("android-properties", 20, properties).unwrap())
            .resolve();

        assert_eq!(
            resolution.resolved().get("projection_depth_meters"),
            Some(&RuntimeValue::Float(1.5))
        );

        let setting = resolution
            .get("projection_depth_meters")
            .expect("depth should be resolved");
        assert_eq!(setting.source, RuntimeConfigSource::AndroidProperty);
        assert_eq!(setting.owner.as_str(), "android-properties");
        assert_eq!(setting.default_value, Some(RuntimeValue::Float(1.0)));
        assert_eq!(setting.candidates.len(), 3);
        assert_eq!(
            setting
                .overridden_candidates()
                .map(|candidate| candidate.owner.as_str())
                .collect::<Vec<_>>(),
            ["launch-profile", "backend-defaults"]
        );
    }

    #[test]
    fn later_layer_wins_equal_precedence() {
        let first = RuntimeConfig::parse_pairs(
            RuntimeConfigSource::File,
            [("projection_depth_meters", "1.1")],
        )
        .expect("file values should parse");
        let second = RuntimeConfig::parse_pairs(
            RuntimeConfigSource::Environment,
            [("projection_depth_meters", "1.2")],
        )
        .expect("env values should parse");

        let resolution = RuntimeConfigResolver::new()
            .with_layer(RuntimeConfigLayer::new("file-profile", 10, first).unwrap())
            .with_layer(RuntimeConfigLayer::new("env-profile", 10, second).unwrap())
            .resolve();

        let setting = resolution
            .get("projection_depth_meters")
            .expect("depth should be resolved");
        assert_eq!(setting.value, RuntimeValue::Float(1.2));
        assert_eq!(setting.owner.as_str(), "env-profile");
    }

    #[test]
    fn rejects_invalid_owner_labels() {
        let config = RuntimeConfig::new();

        assert_eq!(
            RuntimeConfigLayer::new("Launch Profile", 0, config).unwrap_err(),
            RuntimeConfigError::InvalidOwner("Launch Profile".to_string())
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn runtime_config_round_trips_with_serde() {
        let config = RuntimeConfig::parse_pairs(
            RuntimeConfigSource::Synthetic,
            [("render_scale", "0.8"), ("capture_enabled", "true")],
        )
        .expect("pairs should parse");

        let encoded = serde_json::to_string(&config).expect("config should serialize");
        let decoded: RuntimeConfig =
            serde_json::from_str(&encoded).expect("config should deserialize");

        assert_eq!(decoded, config);
    }
}
