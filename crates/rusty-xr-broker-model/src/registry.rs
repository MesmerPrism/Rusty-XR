//! Broker stream registry and topology snapshot contracts.
//!
//! The registry snapshot gives UI hosts a framework-neutral view of providers,
//! streams, adapters, subscribers, command clients, and active leases.

use std::collections::BTreeMap;

use crate::{
    BrokerControlLease, BrokerDataSensitivity, BrokerPayloadKind, BrokerReliabilityClass,
    BrokerStatus, BrokerStreamKind, BrokerStreamManifest, BrokerTransportKind, STREAM_BIO_BREATH,
    STREAM_BIO_HEART, STREAM_LATENCY_SAMPLE, STREAM_SYNTHETIC_WAVE,
};

/// Versioned JSON schema id for broker stream registry snapshots.
pub const BROKER_STREAM_REGISTRY_SNAPSHOT_SCHEMA: &str =
    "rusty.xr.broker.stream_registry_snapshot.v1";

/// Read-only command name for requesting a broker stream registry snapshot.
pub const BROKER_STREAM_REGISTRY_SNAPSHOT_COMMAND: &str = "stream_registry.snapshot";

/// Public HTTP path for a broker stream registry snapshot.
pub const BROKER_STREAM_REGISTRY_SNAPSHOT_HTTP_PATH: &str = "/stream_registry/snapshot";

/// Coarse update rate class for UI and storage decisions.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerStreamRateClass {
    LowRateTelemetry,
    FrameRateTelemetry,
    Media,
    Burst,
    MetadataOnly,
    Unknown,
}

impl BrokerStreamRateClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LowRateTelemetry => "low_rate_telemetry",
            Self::FrameRateTelemetry => "frame_rate_telemetry",
            Self::Media => "media",
            Self::Burst => "burst",
            Self::MetadataOnly => "metadata_only",
            Self::Unknown => "unknown",
        }
    }
}

/// Retention expectation advertised by a broker stream.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerStreamRetentionPolicy {
    None,
    RollingWindow,
    SessionReplay,
    DownstreamOwned,
}

impl BrokerStreamRetentionPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::RollingWindow => "rolling_window",
            Self::SessionReplay => "session_replay",
            Self::DownstreamOwned => "downstream_owned",
        }
    }
}

/// Shared lifecycle state for registry nodes.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerRegistryNodeState {
    Starting,
    Active,
    Idle,
    Stopped,
    Degraded,
    Failed,
    Unknown,
}

impl BrokerRegistryNodeState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Active => "active",
            Self::Idle => "idle",
            Self::Stopped => "stopped",
            Self::Degraded => "degraded",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }
}

/// Broker-owned topology snapshot.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerStreamRegistrySnapshot {
    pub schema: String,
    pub broker_id: String,
    pub revision: u64,
    pub captured_elapsed_ns: Option<u64>,
    pub providers: Vec<BrokerStreamProviderDescriptor>,
    pub streams: Vec<BrokerRegisteredStreamDescriptor>,
    pub adapters: Vec<BrokerStreamAdapterDescriptor>,
    pub subscribers: Vec<BrokerStreamSubscriberDescriptor>,
    pub command_clients: Vec<BrokerCommandClientDescriptor>,
    pub active_leases: Vec<BrokerControlLease>,
}

impl BrokerStreamRegistrySnapshot {
    pub fn new(broker_id: impl Into<String>, revision: u64) -> Self {
        Self {
            schema: BROKER_STREAM_REGISTRY_SNAPSHOT_SCHEMA.to_string(),
            broker_id: broker_id.into(),
            revision,
            captured_elapsed_ns: None,
            providers: Vec::new(),
            streams: Vec::new(),
            adapters: Vec::new(),
            subscribers: Vec::new(),
            command_clients: Vec::new(),
            active_leases: Vec::new(),
        }
    }

    pub fn from_status(status: &BrokerStatus, revision: u64) -> Self {
        let mut provider_streams = BTreeMap::<String, Vec<String>>::new();
        let mut snapshot = Self::new(status.broker_id.clone(), revision);
        for manifest in &status.streams {
            provider_streams
                .entry(manifest.source_id.clone())
                .or_default()
                .push(manifest.stream_id.clone());
            snapshot
                .streams
                .push(BrokerRegisteredStreamDescriptor::from_manifest(manifest));
        }
        snapshot.providers = provider_streams
            .into_iter()
            .map(|(provider_id, stream_ids)| {
                let mut provider = BrokerStreamProviderDescriptor::new(
                    provider_id.clone(),
                    provider_id,
                    BrokerDataSensitivity::Unknown,
                )
                .with_state(BrokerRegistryNodeState::Active);
                provider.stream_ids = stream_ids;
                provider
            })
            .collect();
        snapshot
    }

    pub const fn with_captured_elapsed_ns(mut self, captured_elapsed_ns: u64) -> Self {
        self.captured_elapsed_ns = Some(captured_elapsed_ns);
        self
    }

    pub fn with_provider(mut self, provider: BrokerStreamProviderDescriptor) -> Self {
        self.providers.push(provider);
        self
    }

    pub fn with_stream(mut self, stream: BrokerRegisteredStreamDescriptor) -> Self {
        self.streams.push(stream);
        self
    }

    pub fn with_adapter(mut self, adapter: BrokerStreamAdapterDescriptor) -> Self {
        self.adapters.push(adapter);
        self
    }

    pub fn with_subscriber(mut self, subscriber: BrokerStreamSubscriberDescriptor) -> Self {
        self.subscribers.push(subscriber);
        self
    }

    pub fn with_command_client(mut self, client: BrokerCommandClientDescriptor) -> Self {
        self.command_clients.push(client);
        self
    }

    pub fn with_active_lease(mut self, lease: BrokerControlLease) -> Self {
        self.active_leases.push(lease);
        self
    }

    pub fn stream(&self, stream_id: &str) -> Option<&BrokerRegisteredStreamDescriptor> {
        self.streams
            .iter()
            .find(|stream| stream.stream_id == stream_id)
    }

    pub fn subscriber(&self, subscriber_id: &str) -> Option<&BrokerStreamSubscriberDescriptor> {
        self.subscribers
            .iter()
            .find(|subscriber| subscriber.subscriber_id == subscriber_id)
    }

    pub fn chartable_streams(&self) -> Vec<&BrokerRegisteredStreamDescriptor> {
        self.streams
            .iter()
            .filter(|stream| stream.is_chartable())
            .collect()
    }

    pub fn chartable_metric_count(&self) -> usize {
        self.chartable_streams()
            .iter()
            .map(|stream| stream.metrics.len())
            .sum()
    }

    pub fn auto_subscribe_streams(&self) -> Vec<&BrokerRegisteredStreamDescriptor> {
        self.streams
            .iter()
            .filter(|stream| stream.is_ui_auto_subscribe_candidate())
            .collect()
    }

    pub fn auto_subscribe_stream_ids(&self) -> Vec<String> {
        self.auto_subscribe_streams()
            .iter()
            .map(|stream| stream.stream_id.clone())
            .collect()
    }

    pub fn subscriber_stream_ids(&self, subscriber_id: &str) -> Vec<String> {
        self.subscriber(subscriber_id)
            .map(|subscriber| {
                subscriber
                    .stream_ids
                    .iter()
                    .filter(|stream_id| self.stream(stream_id).is_some())
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "{} rev {} / {} provider(s) / {} stream(s) / {} adapter(s) / {} subscriber(s) / {} command client(s) / {} active lease(s)",
            self.broker_id,
            self.revision,
            self.providers.len(),
            self.streams.len(),
            self.adapters.len(),
            self.subscribers.len(),
            self.command_clients.len(),
            self.active_leases.len()
        )
    }

    pub fn providers_line(&self) -> String {
        if self.providers.is_empty() {
            return "no providers".to_string();
        }
        self.providers
            .iter()
            .map(BrokerStreamProviderDescriptor::line)
            .collect::<Vec<_>>()
            .join(" / ")
    }

    pub fn streams_line(&self) -> String {
        if self.streams.is_empty() {
            return "no streams".to_string();
        }
        self.streams
            .iter()
            .map(BrokerRegisteredStreamDescriptor::line)
            .collect::<Vec<_>>()
            .join(" / ")
    }

    pub fn adapters_line(&self) -> String {
        if self.adapters.is_empty() {
            return "no adapters".to_string();
        }
        self.adapters
            .iter()
            .map(BrokerStreamAdapterDescriptor::line)
            .collect::<Vec<_>>()
            .join(" / ")
    }

    pub fn subscribers_line(&self) -> String {
        if self.subscribers.is_empty() {
            return "no subscribers".to_string();
        }
        self.subscribers
            .iter()
            .map(BrokerStreamSubscriberDescriptor::line)
            .collect::<Vec<_>>()
            .join(" / ")
    }

    pub fn leases_line(&self) -> String {
        if self.active_leases.is_empty() {
            return "no active leases".to_string();
        }
        self.active_leases
            .iter()
            .map(|lease| {
                format!(
                    "{} / holder {} / scope {} / rev {} / {:?}",
                    lease.lease_id,
                    lease.holder_client_id,
                    lease.scope.scope_id,
                    lease.granted_revision,
                    lease.state
                )
            })
            .collect::<Vec<_>>()
            .join(" / ")
    }

    pub fn streams_for_provider(
        &self,
        provider_id: &str,
    ) -> Vec<&BrokerRegisteredStreamDescriptor> {
        self.streams
            .iter()
            .filter(|stream| stream.provider_id.as_deref() == Some(provider_id))
            .collect()
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_STREAM_REGISTRY_SNAPSHOT_SCHEMA
            && !self.broker_id.trim().is_empty()
            && self
                .providers
                .iter()
                .all(BrokerStreamProviderDescriptor::is_valid)
            && self
                .streams
                .iter()
                .all(BrokerRegisteredStreamDescriptor::is_valid)
            && self
                .adapters
                .iter()
                .all(BrokerStreamAdapterDescriptor::is_valid)
            && self
                .subscribers
                .iter()
                .all(BrokerStreamSubscriberDescriptor::is_valid)
            && self
                .command_clients
                .iter()
                .all(BrokerCommandClientDescriptor::is_valid)
            && self.active_leases.iter().all(BrokerControlLease::is_valid)
    }
}

/// Provider that owns one or more streams.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerStreamProviderDescriptor {
    pub provider_id: String,
    pub label: String,
    pub state: BrokerRegistryNodeState,
    pub data_sensitivity: BrokerDataSensitivity,
    pub stream_ids: Vec<String>,
}

impl BrokerStreamProviderDescriptor {
    pub fn new(
        provider_id: impl Into<String>,
        label: impl Into<String>,
        data_sensitivity: BrokerDataSensitivity,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            label: label.into(),
            state: BrokerRegistryNodeState::Unknown,
            data_sensitivity,
            stream_ids: Vec::new(),
        }
    }

    pub const fn with_state(mut self, state: BrokerRegistryNodeState) -> Self {
        self.state = state;
        self
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_ids.push(stream_id.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        non_empty(&self.provider_id)
            && non_empty(&self.label)
            && self.stream_ids.iter().all(|stream_id| non_empty(stream_id))
    }

    pub fn line(&self) -> String {
        format!(
            "{} / {} / {} stream(s) / {}",
            self.label,
            self.provider_id,
            self.stream_ids.len(),
            self.state.as_str()
        )
    }
}

/// Stream descriptor optimized for registry and UI discovery.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerRegisteredStreamDescriptor {
    pub stream_id: String,
    pub label: String,
    pub provider_id: Option<String>,
    pub stream_kind: BrokerStreamKind,
    pub payload_kind: BrokerPayloadKind,
    pub payload_schema: String,
    pub metrics: Vec<BrokerStreamMetricDescriptor>,
    pub recommended_rate_hz: Option<f32>,
    pub rate_class: BrokerStreamRateClass,
    pub data_sensitivity: BrokerDataSensitivity,
    pub retention_policy: BrokerStreamRetentionPolicy,
}

impl BrokerRegisteredStreamDescriptor {
    pub fn new(
        stream_id: impl Into<String>,
        label: impl Into<String>,
        stream_kind: BrokerStreamKind,
        payload_kind: BrokerPayloadKind,
        payload_schema: impl Into<String>,
        data_sensitivity: BrokerDataSensitivity,
    ) -> Self {
        Self {
            stream_id: stream_id.into(),
            label: label.into(),
            provider_id: None,
            stream_kind,
            payload_kind,
            payload_schema: payload_schema.into(),
            metrics: Vec::new(),
            recommended_rate_hz: None,
            rate_class: BrokerStreamRateClass::Unknown,
            data_sensitivity,
            retention_policy: BrokerStreamRetentionPolicy::None,
        }
    }

    pub fn from_manifest(manifest: &BrokerStreamManifest) -> Self {
        let rate_class = infer_rate_class(manifest);
        let retention_policy = match rate_class {
            BrokerStreamRateClass::LowRateTelemetry | BrokerStreamRateClass::FrameRateTelemetry => {
                BrokerStreamRetentionPolicy::RollingWindow
            }
            BrokerStreamRateClass::Media | BrokerStreamRateClass::Burst => {
                BrokerStreamRetentionPolicy::DownstreamOwned
            }
            BrokerStreamRateClass::MetadataOnly | BrokerStreamRateClass::Unknown => {
                BrokerStreamRetentionPolicy::None
            }
        };
        let mut descriptor = Self::new(
            manifest.stream_id.clone(),
            manifest.stream_id.clone(),
            infer_stream_kind(&manifest.stream_id, manifest.payload_kind),
            manifest.payload_kind,
            manifest.payload_schema.clone(),
            infer_data_sensitivity(&manifest.stream_id),
        )
        .with_provider_id(manifest.source_id.clone())
        .with_rate_class(rate_class)
        .with_retention_policy(retention_policy);
        descriptor.recommended_rate_hz = manifest.recommended_rate_hz;
        descriptor
    }

    pub fn with_provider_id(mut self, provider_id: impl Into<String>) -> Self {
        self.provider_id = Some(provider_id.into());
        self
    }

    pub fn with_metric(mut self, metric: BrokerStreamMetricDescriptor) -> Self {
        self.metrics.push(metric);
        self
    }

    pub fn with_recommended_rate_hz(mut self, recommended_rate_hz: f32) -> Self {
        self.recommended_rate_hz = Some(recommended_rate_hz);
        self
    }

    pub const fn with_rate_class(mut self, rate_class: BrokerStreamRateClass) -> Self {
        self.rate_class = rate_class;
        self
    }

    pub const fn with_retention_policy(
        mut self,
        retention_policy: BrokerStreamRetentionPolicy,
    ) -> Self {
        self.retention_policy = retention_policy;
        self
    }

    pub fn is_valid(&self) -> bool {
        non_empty(&self.stream_id)
            && non_empty(&self.label)
            && self.provider_id.as_deref().map(non_empty).unwrap_or(true)
            && non_empty(&self.payload_schema)
            && self
                .recommended_rate_hz
                .map(|rate| rate.is_finite() && rate > 0.0)
                .unwrap_or(true)
            && self
                .metrics
                .iter()
                .all(BrokerStreamMetricDescriptor::is_valid)
    }

    pub fn line(&self) -> String {
        let metrics = if self.metrics.is_empty() {
            "no metrics".to_string()
        } else {
            self.metrics
                .iter()
                .map(BrokerStreamMetricDescriptor::line)
                .collect::<Vec<_>>()
                .join(", ")
        };
        format!(
            "{} / {:?} / {} / {} / {}",
            self.stream_id,
            self.stream_kind,
            self.rate_class.as_str(),
            self.data_sensitivity.as_str(),
            metrics
        )
    }

    pub fn is_chartable(&self) -> bool {
        !self.metrics.is_empty()
            && matches!(
                self.payload_kind,
                BrokerPayloadKind::Json | BrokerPayloadKind::Text | BrokerPayloadKind::Custom
            )
            && matches!(
                self.rate_class,
                BrokerStreamRateClass::LowRateTelemetry
                    | BrokerStreamRateClass::FrameRateTelemetry
                    | BrokerStreamRateClass::Unknown
            )
            && !matches!(
                self.retention_policy,
                BrokerStreamRetentionPolicy::DownstreamOwned
            )
    }

    pub fn is_ui_auto_subscribe_candidate(&self) -> bool {
        matches!(
            self.payload_kind,
            BrokerPayloadKind::Json | BrokerPayloadKind::Text | BrokerPayloadKind::Custom
        ) && !matches!(
            self.rate_class,
            BrokerStreamRateClass::Media | BrokerStreamRateClass::Burst
        ) && !matches!(
            self.retention_policy,
            BrokerStreamRetentionPolicy::DownstreamOwned
        )
    }
}

/// Numeric metric carried by a registered stream.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerStreamMetricDescriptor {
    pub metric: String,
    pub label: String,
    pub unit: Option<String>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
}

impl BrokerStreamMetricDescriptor {
    pub fn new(metric: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            metric: metric.into(),
            label: label.into(),
            unit: None,
            min_value: None,
            max_value: None,
        }
    }

    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    pub const fn with_range(mut self, min_value: f64, max_value: f64) -> Self {
        self.min_value = Some(min_value);
        self.max_value = Some(max_value);
        self
    }

    pub fn is_valid(&self) -> bool {
        non_empty(&self.metric)
            && non_empty(&self.label)
            && self.unit.as_deref().map(non_empty).unwrap_or(true)
            && match (self.min_value, self.max_value) {
                (Some(min), Some(max)) => min.is_finite() && max.is_finite() && min <= max,
                (Some(value), None) | (None, Some(value)) => value.is_finite(),
                (None, None) => true,
            }
    }

    pub fn line(&self) -> String {
        match self.unit.as_deref() {
            Some(unit) => format!("{} {}", self.metric, unit),
            None => self.metric.clone(),
        }
    }
}

/// Adapter that consumes stream input or produces derived streams.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerStreamAdapterDescriptor {
    pub adapter_id: String,
    pub label: String,
    pub state: BrokerRegistryNodeState,
    pub input_stream_ids: Vec<String>,
    pub output_stream_ids: Vec<String>,
}

impl BrokerStreamAdapterDescriptor {
    pub fn new(adapter_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            adapter_id: adapter_id.into(),
            label: label.into(),
            state: BrokerRegistryNodeState::Unknown,
            input_stream_ids: Vec::new(),
            output_stream_ids: Vec::new(),
        }
    }

    pub const fn with_state(mut self, state: BrokerRegistryNodeState) -> Self {
        self.state = state;
        self
    }

    pub fn with_input_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.input_stream_ids.push(stream_id.into());
        self
    }

    pub fn with_output_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.output_stream_ids.push(stream_id.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        non_empty(&self.adapter_id)
            && non_empty(&self.label)
            && self
                .input_stream_ids
                .iter()
                .all(|stream_id| non_empty(stream_id))
            && self
                .output_stream_ids
                .iter()
                .all(|stream_id| non_empty(stream_id))
    }

    pub fn line(&self) -> String {
        format!(
            "{} / {} input(s) -> {} output(s) / {}",
            self.label,
            self.input_stream_ids.len(),
            self.output_stream_ids.len(),
            self.state.as_str()
        )
    }
}

/// Client subscribed to one or more streams.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerStreamSubscriberDescriptor {
    pub subscriber_id: String,
    pub label: String,
    pub transport: BrokerTransportKind,
    pub stream_ids: Vec<String>,
}

impl BrokerStreamSubscriberDescriptor {
    pub fn new(
        subscriber_id: impl Into<String>,
        label: impl Into<String>,
        transport: BrokerTransportKind,
    ) -> Self {
        Self {
            subscriber_id: subscriber_id.into(),
            label: label.into(),
            transport,
            stream_ids: Vec::new(),
        }
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_ids.push(stream_id.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        non_empty(&self.subscriber_id)
            && non_empty(&self.label)
            && self.stream_ids.iter().all(|stream_id| non_empty(stream_id))
    }

    pub fn line(&self) -> String {
        format!(
            "{} / {:?} / {} stream(s)",
            self.label,
            self.transport,
            self.stream_ids.len()
        )
    }
}

/// Client that may issue broker commands.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerCommandClientDescriptor {
    pub client_id: String,
    pub label: String,
    pub command_scopes: Vec<String>,
    pub held_lease_ids: Vec<String>,
}

impl BrokerCommandClientDescriptor {
    pub fn new(client_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            label: label.into(),
            command_scopes: Vec::new(),
            held_lease_ids: Vec::new(),
        }
    }

    pub fn with_command_scope(mut self, command_scope: impl Into<String>) -> Self {
        self.command_scopes.push(command_scope.into());
        self
    }

    pub fn with_held_lease_id(mut self, lease_id: impl Into<String>) -> Self {
        self.held_lease_ids.push(lease_id.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        non_empty(&self.client_id)
            && non_empty(&self.label)
            && self.command_scopes.iter().all(|scope| non_empty(scope))
            && self
                .held_lease_ids
                .iter()
                .all(|lease_id| non_empty(lease_id))
    }

    pub fn line(&self) -> String {
        format!(
            "{} / {} scope(s) / {} lease(s)",
            self.label,
            self.command_scopes.len(),
            self.held_lease_ids.len()
        )
    }
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn infer_stream_kind(stream_id: &str, payload_kind: BrokerPayloadKind) -> BrokerStreamKind {
    if matches!(
        payload_kind,
        BrokerPayloadKind::H264 | BrokerPayloadKind::H265
    ) {
        return BrokerStreamKind::Media;
    }
    if stream_id.starts_with("bio:") {
        return BrokerStreamKind::Bio;
    }
    if stream_id.starts_with("synthetic:") {
        return BrokerStreamKind::Synthetic;
    }
    if stream_id.starts_with("xr:") {
        return BrokerStreamKind::XrInput;
    }
    if stream_id.starts_with("latency:")
        || stream_id.starts_with("clock:")
        || stream_id.starts_with("broker:")
    {
        return BrokerStreamKind::Telemetry;
    }
    if stream_id.contains("control") {
        return BrokerStreamKind::Control;
    }
    BrokerStreamKind::Custom
}

fn infer_data_sensitivity(stream_id: &str) -> BrokerDataSensitivity {
    match stream_id {
        STREAM_LATENCY_SAMPLE => BrokerDataSensitivity::Diagnostic,
        STREAM_BIO_BREATH => BrokerDataSensitivity::DerivedPhysiology,
        STREAM_BIO_HEART => BrokerDataSensitivity::Physiology,
        STREAM_SYNTHETIC_WAVE => BrokerDataSensitivity::Public,
        _ if stream_id.starts_with("bio:") => BrokerDataSensitivity::Physiology,
        _ if stream_id.contains("video") || stream_id.starts_with("camera") => {
            BrokerDataSensitivity::Diagnostic
        }
        _ => BrokerDataSensitivity::Unknown,
    }
}

fn infer_rate_class(manifest: &BrokerStreamManifest) -> BrokerStreamRateClass {
    if manifest.reliability == BrokerReliabilityClass::MetadataOnly {
        return BrokerStreamRateClass::MetadataOnly;
    }
    if matches!(
        manifest.payload_kind,
        BrokerPayloadKind::H264 | BrokerPayloadKind::H265
    ) {
        return BrokerStreamRateClass::Media;
    }
    match manifest.recommended_rate_hz {
        Some(rate) if rate >= 45.0 => BrokerStreamRateClass::FrameRateTelemetry,
        Some(rate) if rate > 0.0 => BrokerStreamRateClass::LowRateTelemetry,
        _ => BrokerStreamRateClass::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BrokerCommandClientDescriptor, BrokerRegisteredStreamDescriptor, BrokerRegistryNodeState,
        BrokerStreamMetricDescriptor, BrokerStreamProviderDescriptor, BrokerStreamRateClass,
        BrokerStreamRegistrySnapshot, BrokerStreamRetentionPolicy,
        BrokerStreamSubscriberDescriptor, BROKER_STREAM_REGISTRY_SNAPSHOT_SCHEMA,
    };
    use crate::{
        BrokerControlLease, BrokerControlScope, BrokerDataSensitivity, BrokerPayloadKind,
        BrokerReliabilityClass, BrokerStatus, BrokerStreamKind, BrokerStreamManifest,
        BrokerTransportKind, SYNTHETIC_WAVE_PAYLOAD_SCHEMA,
    };

    #[test]
    fn registry_snapshot_models_stream_topology_and_leases() {
        let stream = BrokerRegisteredStreamDescriptor::new(
            "bio:breath",
            "Breath",
            BrokerStreamKind::Bio,
            BrokerPayloadKind::Json,
            "rusty.xr.bio.breath.v1",
            BrokerDataSensitivity::DerivedPhysiology,
        )
        .with_provider_id("bio-provider")
        .with_metric(BrokerStreamMetricDescriptor::new("volume01", "Volume").with_range(0.0, 1.0))
        .with_recommended_rate_hz(15.0)
        .with_rate_class(BrokerStreamRateClass::LowRateTelemetry)
        .with_retention_policy(BrokerStreamRetentionPolicy::RollingWindow);
        let scope = BrokerControlScope::new("runtime.bio", "runtime.bio");
        let lease = BrokerControlLease::new("lease-1", "client-1", scope, 5);
        let snapshot = BrokerStreamRegistrySnapshot::new("broker", 5)
            .with_provider(
                BrokerStreamProviderDescriptor::new(
                    "bio-provider",
                    "Bio provider",
                    BrokerDataSensitivity::DerivedPhysiology,
                )
                .with_state(BrokerRegistryNodeState::Active)
                .with_stream_id("bio:breath"),
            )
            .with_stream(stream)
            .with_subscriber(
                BrokerStreamSubscriberDescriptor::new(
                    "makepad-inspector",
                    "Makepad inspector",
                    BrokerTransportKind::WebSocket,
                )
                .with_stream_id("bio:breath"),
            )
            .with_command_client(
                BrokerCommandClientDescriptor::new("makepad-inspector", "Makepad inspector")
                    .with_command_scope("session.lifecycle")
                    .with_held_lease_id("lease-1"),
            )
            .with_active_lease(lease);

        assert!(snapshot.is_valid());
        assert_eq!(snapshot.schema, BROKER_STREAM_REGISTRY_SNAPSHOT_SCHEMA);
        assert!(snapshot.stream("bio:breath").is_some());
        assert_eq!(snapshot.streams_for_provider("bio-provider").len(), 1);
        assert_eq!(snapshot.active_leases.len(), 1);
        assert!(snapshot.summary_line().contains("1 active lease"));
        assert!(snapshot.providers_line().contains("Bio provider"));
        assert!(snapshot.streams_line().contains("bio:breath"));
        assert!(snapshot.subscribers_line().contains("Makepad inspector"));
    }

    #[test]
    fn registry_snapshot_can_be_derived_from_broker_status_manifests() {
        let status = BrokerStatus {
            streams: vec![
                BrokerStreamManifest::new(
                    "synthetic:wave",
                    "synthetic-provider",
                    BrokerPayloadKind::Json,
                    SYNTHETIC_WAVE_PAYLOAD_SCHEMA,
                )
                .with_recommended_rate_hz(30.0),
                BrokerStreamManifest::new(
                    "camera.left.h264",
                    "camera-provider",
                    BrokerPayloadKind::H264,
                    "rusty.xr.video_lab.binary_stream.v1",
                )
                .with_reliability(BrokerReliabilityClass::LossTolerant)
                .with_recommended_rate_hz(60.0),
            ],
            ..BrokerStatus::new("broker", 1_000)
        };

        let snapshot = BrokerStreamRegistrySnapshot::from_status(&status, 12);

        assert!(snapshot.is_valid());
        assert_eq!(snapshot.revision, 12);
        assert_eq!(snapshot.providers.len(), 2);
        assert_eq!(snapshot.streams.len(), 2);
        assert_eq!(
            snapshot
                .stream("camera.left.h264")
                .expect("camera stream expected")
                .rate_class,
            BrokerStreamRateClass::Media
        );
        assert!(snapshot.summary_line().contains("2 stream"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn stream_registry_snapshot_round_trips_with_serde() {
        let snapshot = BrokerStreamRegistrySnapshot::new("broker", 1).with_stream(
            BrokerRegisteredStreamDescriptor::new(
                "latency:sample",
                "Latency",
                BrokerStreamKind::Telemetry,
                BrokerPayloadKind::Json,
                "rusty.xr.broker.latency_sample.v1",
                BrokerDataSensitivity::Diagnostic,
            )
            .with_metric(BrokerStreamMetricDescriptor::new("latency_ms", "Latency").with_unit("ms"))
            .with_rate_class(BrokerStreamRateClass::LowRateTelemetry),
        );

        let encoded = serde_json::to_string(&snapshot).expect("registry should serialize");
        let decoded: BrokerStreamRegistrySnapshot =
            serde_json::from_str(&encoded).expect("registry should deserialize");

        assert_eq!(decoded, snapshot);
        assert!(decoded.is_valid());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn public_stream_registry_fixture_deserializes() {
        let snapshot: BrokerStreamRegistrySnapshot = serde_json::from_str(include_str!(
            "../../../fixtures/broker-ui/synthetic-stream-registry-snapshot.json"
        ))
        .expect("public stream registry fixture should deserialize");

        assert!(snapshot.is_valid());
        assert_eq!(snapshot.broker_id, "synthetic-broker");
        assert_eq!(snapshot.providers.len(), 3);
        assert_eq!(snapshot.streams.len(), 5);
        assert_eq!(snapshot.adapters.len(), 2);
        assert!(snapshot.stream("bio:breath").is_some());
        assert_eq!(snapshot.chartable_streams().len(), 5);
        assert_eq!(snapshot.chartable_metric_count(), 6);
        assert_eq!(snapshot.auto_subscribe_stream_ids().len(), 5);
        assert_eq!(snapshot.active_leases.len(), 0);
    }
}
