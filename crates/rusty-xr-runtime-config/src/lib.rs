//! Runtime configuration helpers for Rusty XR.
//!
//! This crate models generic launch/runtime settings. Downstream apps can map
//! their private environment variables, Android properties, or config files
//! onto these public keys without publishing app-specific aliases.
//!
//! Enable the `serde` feature when runtime profiles or operator tools need to
//! serialize these public settings.
//!
//! ```
//! use rusty_xr_runtime_config::{RuntimeConfig, RuntimeConfigSource, RuntimeValue};
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
        Self("debug.rustyxr".to_string())
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

/// Runtime configuration parsing error.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeConfigError {
    InvalidKey(String),
    InvalidAndroidPropertyPrefix(String),
}

impl fmt::Display for RuntimeConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKey(value) => write!(f, "invalid runtime config key: {value}"),
            Self::InvalidAndroidPropertyPrefix(value) => {
                write!(f, "invalid Android property prefix: {value}")
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

        assert_eq!(key.android_property(&prefix), "debug.rustyxr.render.scale");
    }

    #[test]
    fn android_property_normalizes_public_key_separators() {
        let key = RuntimeKey::new("render-scale").expect("key should be valid");
        let prefix = AndroidPropertyPrefix::default();

        assert_eq!(key.android_property(&prefix), "debug.rustyxr.render.scale");
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
