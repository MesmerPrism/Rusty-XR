//! Lab Streaming Layer models and utilities for Rusty XR.
//!
//! This crate currently contains pure models only. Native `liblsl` discovery,
//! inlet, and outlet backends should remain optional so tests do not require a
//! system LSL runtime.
//!
//! Enable the `serde` feature when stream descriptors or telemetry packets need
//! to cross process boundaries.
//!
//! ```
//! use rusty_xr_lsl::{LslChannelFormat, LslStreamDescriptor};
//!
//! let descriptor = LslStreamDescriptor::new("Example", "Telemetry", 2, LslChannelFormat::Float32);
//! assert!(descriptor.is_valid());
//! ```

/// Crate version exposed for lightweight smoke checks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Public LSL channel format model.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LslChannelFormat {
    Float32,
    Double64,
    Int32,
    Int16,
    Int8,
    String,
}

/// Sanitized stream roles used by Rusty XR examples and adapters.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LslStreamRole {
    Biofeedback,
    ClockProbe,
    ClockEcho,
    ParticleTelemetry,
    PolarHeartRate,
    PolarEcg,
    PolarAccelerometer,
    Custom,
}

/// Public HRV/biofeedback stream name used by the PolarH10 workflow docs.
pub const HRV_BIOFEEDBACK_STREAM_NAME: &str = "HRV_Biofeedback";

/// Public HRV/biofeedback stream type used by the PolarH10 workflow docs.
pub const HRV_BIOFEEDBACK_STREAM_TYPE: &str = "HRV";

/// Public stream type for Polar heart-rate and RR data.
pub const POLAR_HEART_RATE_STREAM_TYPE: &str = "rusty.xr.polar.heart_rate";

/// Public stream type for Polar ECG data.
pub const POLAR_ECG_STREAM_TYPE: &str = "rusty.xr.polar.ecg";

/// Public stream type for Polar accelerometer data.
pub const POLAR_ACC_STREAM_TYPE: &str = "rusty.xr.polar.acc";

/// App-neutral stream descriptor.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct LslStreamDescriptor {
    pub name: String,
    pub stream_type: String,
    pub source_id: Option<String>,
    pub channel_count: u32,
    pub nominal_srate_hz: Option<f64>,
    pub channel_format: LslChannelFormat,
    pub role: Option<LslStreamRole>,
}

impl LslStreamDescriptor {
    pub fn new(
        name: impl Into<String>,
        stream_type: impl Into<String>,
        channel_count: u32,
        channel_format: LslChannelFormat,
    ) -> Self {
        Self {
            name: name.into(),
            stream_type: stream_type.into(),
            source_id: None,
            channel_count,
            nominal_srate_hz: None,
            channel_format,
            role: None,
        }
    }

    pub fn with_source_id(mut self, source_id: impl Into<String>) -> Self {
        self.source_id = Some(source_id.into());
        self
    }

    pub fn with_nominal_srate_hz(mut self, nominal_srate_hz: f64) -> Self {
        self.nominal_srate_hz = Some(nominal_srate_hz);
        self
    }

    pub fn with_role(mut self, role: LslStreamRole) -> Self {
        self.role = Some(role);
        self
    }

    pub fn is_valid(&self) -> bool {
        !self.name.trim().is_empty()
            && !self.stream_type.trim().is_empty()
            && self.channel_count > 0
            && self
                .nominal_srate_hz
                .map(|value| value.is_finite() && value >= 0.0)
                .unwrap_or(true)
    }
}

/// Human-readable channel labels and units for an LSL stream.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LslChannelSchema {
    pub labels: Vec<String>,
    pub unit: Option<String>,
}

impl LslChannelSchema {
    pub fn new(labels: Vec<String>, unit: Option<String>) -> Self {
        Self { labels, unit }
    }

    pub fn is_valid_for(&self, descriptor: &LslStreamDescriptor) -> bool {
        self.labels.len() == descriptor.channel_count as usize
            && self.labels.iter().all(|label| !label.trim().is_empty())
    }
}

/// Descriptor filter for discovery results.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LslStreamFilter {
    pub name: Option<String>,
    pub stream_type: Option<String>,
    pub source_id: Option<String>,
}

impl LslStreamFilter {
    pub fn matches(&self, descriptor: &LslStreamDescriptor) -> bool {
        self.name
            .as_deref()
            .map(|name| name == descriptor.name)
            .unwrap_or(true)
            && self
                .stream_type
                .as_deref()
                .map(|stream_type| stream_type == descriptor.stream_type)
                .unwrap_or(true)
            && self
                .source_id
                .as_deref()
                .map(|source_id| descriptor.source_id.as_deref() == Some(source_id))
                .unwrap_or(true)
    }
}

/// Inlet/outlet connection state.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LslConnectionState {
    Disconnected,
    Resolving,
    Connected,
    Stale,
    Error,
}

/// Status for a discovered stream inlet or local outlet.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct LslEndpointStatus {
    pub descriptor: LslStreamDescriptor,
    pub state: LslConnectionState,
    pub last_sample_time_ns: Option<u64>,
    pub last_resolve_time_ns: Option<u64>,
    pub sample_count: u64,
    pub last_error: Option<String>,
}

impl LslEndpointStatus {
    pub fn new(descriptor: LslStreamDescriptor) -> Self {
        Self {
            descriptor,
            state: LslConnectionState::Disconnected,
            last_sample_time_ns: None,
            last_resolve_time_ns: None,
            sample_count: 0,
            last_error: None,
        }
    }

    pub fn sample_age_ns(&self, now_ns: u64) -> Option<u64> {
        let last_sample_time_ns = self.last_sample_time_ns?;
        now_ns.checked_sub(last_sample_time_ns)
    }

    pub fn is_stale(&self, now_ns: u64, stale_after_ns: u64) -> bool {
        self.sample_age_ns(now_ns)
            .map(|age_ns| age_ns > stale_after_ns)
            .unwrap_or(true)
    }
}

/// Roundtrip timing probe summary.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LslRoundtripProbe {
    pub sequence: u64,
    pub sent_time_ns: u64,
    pub received_time_ns: Option<u64>,
}

impl LslRoundtripProbe {
    pub const fn new(sequence: u64, sent_time_ns: u64) -> Self {
        Self {
            sequence,
            sent_time_ns,
            received_time_ns: None,
        }
    }

    pub const fn with_received_time_ns(mut self, received_time_ns: u64) -> Self {
        self.received_time_ns = Some(received_time_ns);
        self
    }

    pub fn latency_ns(self) -> Option<u64> {
        self.received_time_ns?.checked_sub(self.sent_time_ns)
    }
}

/// Generic telemetry sample that can be routed through LSL or another backend.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct TelemetrySample {
    pub timestamp_ns: u64,
    pub labels: Vec<String>,
    pub values: Vec<f32>,
}

impl TelemetrySample {
    pub fn new(timestamp_ns: u64, labels: Vec<String>, values: Vec<f32>) -> Self {
        Self {
            timestamp_ns,
            labels,
            values,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.labels.len() == self.values.len() && self.values.iter().all(|value| value.is_finite())
    }
}

/// Normalized one-channel biofeedback value suitable for particles or UI.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct LslBiofeedbackReading {
    pub sequence: u64,
    pub received_time_ns: u64,
    pub value01: f32,
    pub packet_interval_ns: Option<u64>,
    pub source_stream_name: Option<String>,
    pub source_stream_type: Option<String>,
}

impl LslBiofeedbackReading {
    pub fn new(sequence: u64, received_time_ns: u64, value01: f32) -> Self {
        Self {
            sequence,
            received_time_ns,
            value01: value01.clamp(0.0, 1.0),
            packet_interval_ns: None,
            source_stream_name: None,
            source_stream_type: None,
        }
    }

    pub fn age_ns(&self, now_ns: u64) -> Option<u64> {
        now_ns.checked_sub(self.received_time_ns)
    }

    pub fn is_fresh(&self, now_ns: u64, stale_after_ns: u64) -> bool {
        self.age_ns(now_ns)
            .map(|age_ns| age_ns <= stale_after_ns)
            .unwrap_or(false)
    }
}

/// Returns the public biofeedback stream descriptor used by PolarH10 examples.
pub fn hrv_biofeedback_descriptor() -> LslStreamDescriptor {
    LslStreamDescriptor::new(
        HRV_BIOFEEDBACK_STREAM_NAME,
        HRV_BIOFEEDBACK_STREAM_TYPE,
        1,
        LslChannelFormat::Float32,
    )
    .with_role(LslStreamRole::Biofeedback)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor() -> LslStreamDescriptor {
        LslStreamDescriptor::new("Example", "Telemetry", 2, LslChannelFormat::Float32)
            .with_source_id("source-1")
            .with_nominal_srate_hz(30.0)
            .with_role(LslStreamRole::ParticleTelemetry)
    }

    #[test]
    fn exposes_workspace_version() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn validates_stream_descriptor() {
        assert!(descriptor().is_valid());
        assert_eq!(descriptor().role, Some(LslStreamRole::ParticleTelemetry));
        assert!(
            !LslStreamDescriptor::new("", "Telemetry", 2, LslChannelFormat::Float32).is_valid()
        );
    }

    #[test]
    fn validates_channel_schema_against_descriptor() {
        let schema = LslChannelSchema::new(vec!["x".to_string(), "y".to_string()], None);

        assert!(schema.is_valid_for(&descriptor()));
        assert!(!LslChannelSchema::new(vec!["x".to_string()], None).is_valid_for(&descriptor()));
    }

    #[test]
    fn filters_stream_descriptors() {
        let filter = LslStreamFilter {
            stream_type: Some("Telemetry".to_string()),
            ..LslStreamFilter::default()
        };

        assert!(filter.matches(&descriptor()));
    }

    #[test]
    fn endpoint_status_reports_staleness() {
        let mut status = LslEndpointStatus::new(descriptor());
        status.last_sample_time_ns = Some(100);

        assert_eq!(status.sample_age_ns(150), Some(50));
        assert!(!status.is_stale(150, 100));
        assert!(status.is_stale(250, 100));
    }

    #[test]
    fn roundtrip_probe_reports_latency() {
        let probe = LslRoundtripProbe::new(3, 100).with_received_time_ns(175);

        assert_eq!(probe.latency_ns(), Some(75));
    }

    #[test]
    fn telemetry_sample_requires_label_value_parity() {
        let sample =
            TelemetrySample::new(100, vec!["a".to_string(), "b".to_string()], vec![1.0, 2.0]);

        assert!(sample.is_valid());
        assert!(!TelemetrySample::new(100, vec!["a".to_string()], vec![1.0, 2.0]).is_valid());
    }

    #[test]
    fn biofeedback_reading_clamps_and_reports_freshness() {
        let reading = LslBiofeedbackReading::new(7, 100, 2.0);

        assert_eq!(reading.value01, 1.0);
        assert_eq!(reading.age_ns(150), Some(50));
        assert!(reading.is_fresh(150, 100));
        assert!(!reading.is_fresh(250, 100));
    }

    #[test]
    fn public_biofeedback_descriptor_is_valid() {
        let descriptor = hrv_biofeedback_descriptor();

        assert!(descriptor.is_valid());
        assert_eq!(descriptor.role, Some(LslStreamRole::Biofeedback));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn telemetry_sample_round_trips_with_serde() {
        let sample =
            TelemetrySample::new(100, vec!["a".to_string(), "b".to_string()], vec![1.0, 2.0]);

        let encoded = serde_json::to_string(&sample).expect("sample should serialize");
        let decoded: TelemetrySample =
            serde_json::from_str(&encoded).expect("sample should deserialize");

        assert_eq!(decoded, sample);
    }
}
