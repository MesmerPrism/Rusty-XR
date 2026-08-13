//! Broker module manifest and runtime-state contracts.
//!
//! These data-only contracts let a broker advertise optional providers,
//! processors, sinks, bridges, control adapters, diagnostics, and supervisors
//! without making broker core depend on any runtime adapter, UI framework, or
//! external protocol implementation.

use crate::{
    BrokerChartPolicy, BrokerCommandAuthorityRequirement, BrokerControlScope,
    BrokerDataSensitivity, BrokerPanelDescriptor, BrokerPayloadKind, BrokerStreamKind,
    BrokerStreamRateClass, BrokerStreamRetentionPolicy, BrokerTimestampDomain,
    BrokerUiSubscriptionPolicy,
};

/// Versioned JSON schema id for broker module manifests.
pub const BROKER_MODULE_MANIFEST_SCHEMA: &str = "rusty.xr.broker.module_manifest.v1";

/// Versioned JSON schema id for broker module runtime-state snapshots.
pub const BROKER_MODULE_RUNTIME_STATE_SCHEMA: &str = "rusty.xr.broker.module_runtime_state.v1";

/// Broker-managed runtime capability category.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerModuleKind {
    Provider,
    Processor,
    Sink,
    Bridge,
    ControlAdapter,
    Diagnostic,
    Supervisor,
}

impl BrokerModuleKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Provider => "provider",
            Self::Processor => "processor",
            Self::Sink => "sink",
            Self::Bridge => "bridge",
            Self::ControlAdapter => "control_adapter",
            Self::Diagnostic => "diagnostic",
            Self::Supervisor => "supervisor",
        }
    }
}

/// Broker-visible lifecycle for one optional module.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerModuleLifecycleState {
    Discovered,
    Configured,
    Starting,
    Active,
    Idle,
    Degraded,
    Failed,
    Stopping,
    Stopped,
    Unavailable,
}

impl BrokerModuleLifecycleState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Discovered => "discovered",
            Self::Configured => "configured",
            Self::Starting => "starting",
            Self::Active => "active",
            Self::Idle => "idle",
            Self::Degraded => "degraded",
            Self::Failed => "failed",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Unavailable => "unavailable",
        }
    }

    pub const fn is_available(self) -> bool {
        matches!(
            self,
            Self::Configured | Self::Starting | Self::Active | Self::Idle | Self::Degraded
        )
    }
}

/// Kind of permission or gate a module requires before it can run.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerModulePermissionKind {
    BrokerCapability,
    OperatorApproval,
    AndroidPermission,
    DevicePermission,
    NetworkAccess,
    FileSystemAccess,
    ExternalCredential,
    PairingToken,
    Unknown,
}

/// Platform family where a module can be hosted.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerModulePlatform {
    Any,
    Android,
    Quest,
    Windows,
    Macos,
    Linux,
    Web,
    ExternalSidecar,
    Unknown,
}

/// Broker response when a module fails or degrades.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerModuleFailureAction {
    ReportOnly,
    DegradeStreams,
    DisableModule,
    RestartModule,
    StopSession,
    OperatorInterventionRequired,
}

/// Runtime health state for one advertised module metric.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrokerModuleHealthState {
    Healthy,
    Warning,
    Critical,
    Unavailable,
    #[default]
    Unknown,
}

/// Stream binding exposed or consumed by a broker module.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerModuleStreamBinding {
    pub stream_id: String,
    pub label: String,
    pub stream_kind: BrokerStreamKind,
    pub payload_kind: BrokerPayloadKind,
    pub payload_schema: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub required: bool,
    pub recommended_rate_hz: Option<f32>,
    pub rate_class: BrokerStreamRateClass,
    pub data_sensitivity: BrokerDataSensitivity,
    pub retention_policy: BrokerStreamRetentionPolicy,
    #[cfg_attr(feature = "serde", serde(default))]
    pub ui_subscription_policy: BrokerUiSubscriptionPolicy,
    #[cfg_attr(feature = "serde", serde(default))]
    pub chart_policy: BrokerChartPolicy,
}

impl BrokerModuleStreamBinding {
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
            stream_kind,
            payload_kind,
            payload_schema: payload_schema.into(),
            required: false,
            recommended_rate_hz: None,
            rate_class: BrokerStreamRateClass::Unknown,
            data_sensitivity,
            retention_policy: BrokerStreamRetentionPolicy::None,
            ui_subscription_policy: BrokerUiSubscriptionPolicy::ManualOnly,
            chart_policy: BrokerChartPolicy::NotChartable,
        }
    }

    pub const fn with_required(mut self, required: bool) -> Self {
        self.required = required;
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

    pub const fn with_ui_subscription_policy(
        mut self,
        ui_subscription_policy: BrokerUiSubscriptionPolicy,
    ) -> Self {
        self.ui_subscription_policy = ui_subscription_policy;
        self
    }

    pub const fn with_chart_policy(mut self, chart_policy: BrokerChartPolicy) -> Self {
        self.chart_policy = chart_policy;
        self
    }

    pub fn is_valid(&self) -> bool {
        non_empty(&self.stream_id)
            && non_empty(&self.label)
            && non_empty(&self.payload_schema)
            && self
                .recommended_rate_hz
                .map(|rate| rate.is_finite() && rate > 0.0)
                .unwrap_or(true)
    }
}

/// Command accepted by a module through the broker authority path.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerModuleCommandDescriptor {
    pub command: String,
    pub label: String,
    pub authority: BrokerCommandAuthorityRequirement,
    pub notes: Vec<String>,
}

impl BrokerModuleCommandDescriptor {
    pub fn new(
        command: impl Into<String>,
        label: impl Into<String>,
        authority: BrokerCommandAuthorityRequirement,
    ) -> Self {
        Self {
            command: command.into(),
            label: label.into(),
            authority,
            notes: Vec::new(),
        }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        non_empty(&self.command)
            && non_empty(&self.label)
            && self.authority.is_valid()
            && self.authority.command == self.command
            && self.notes.iter().all(|note| non_empty(note))
    }
}

/// Permission, consent, or capability gate needed by a module.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerModulePermissionRequirement {
    pub permission_id: String,
    pub label: String,
    pub requirement_kind: BrokerModulePermissionKind,
    #[cfg_attr(feature = "serde", serde(default))]
    pub required: bool,
    pub operator_message: Option<String>,
}

impl BrokerModulePermissionRequirement {
    pub fn new(
        permission_id: impl Into<String>,
        label: impl Into<String>,
        requirement_kind: BrokerModulePermissionKind,
    ) -> Self {
        Self {
            permission_id: permission_id.into(),
            label: label.into(),
            requirement_kind,
            required: true,
            operator_message: None,
        }
    }

    pub const fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn with_operator_message(mut self, operator_message: impl Into<String>) -> Self {
        self.operator_message = Some(operator_message.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        non_empty(&self.permission_id)
            && non_empty(&self.label)
            && self
                .operator_message
                .as_deref()
                .map(non_empty)
                .unwrap_or(true)
    }
}

/// External executable, service, or sidecar expected by a module.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerModuleExternalToolRequirement {
    pub tool_id: String,
    pub label: String,
    pub version_requirement: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub user_supplied: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub required: bool,
}

impl BrokerModuleExternalToolRequirement {
    pub fn new(tool_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            tool_id: tool_id.into(),
            label: label.into(),
            version_requirement: None,
            user_supplied: true,
            required: true,
        }
    }

    pub fn with_version_requirement(mut self, version_requirement: impl Into<String>) -> Self {
        self.version_requirement = Some(version_requirement.into());
        self
    }

    pub const fn with_user_supplied(mut self, user_supplied: bool) -> Self {
        self.user_supplied = user_supplied;
        self
    }

    pub const fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn is_valid(&self) -> bool {
        non_empty(&self.tool_id)
            && non_empty(&self.label)
            && self
                .version_requirement
                .as_deref()
                .map(non_empty)
                .unwrap_or(true)
    }
}

/// Resource a module needs to hold or coordinate through the broker.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerModuleResourceLock {
    pub resource_id: String,
    pub label: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub exclusive: bool,
    pub lease_scope: Option<BrokerControlScope>,
    pub notes: Vec<String>,
}

impl BrokerModuleResourceLock {
    pub fn new(resource_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            resource_id: resource_id.into(),
            label: label.into(),
            exclusive: false,
            lease_scope: None,
            notes: Vec::new(),
        }
    }

    pub const fn with_exclusive(mut self, exclusive: bool) -> Self {
        self.exclusive = exclusive;
        self
    }

    pub fn with_lease_scope(mut self, lease_scope: BrokerControlScope) -> Self {
        self.lease_scope = Some(lease_scope);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        non_empty(&self.resource_id)
            && non_empty(&self.label)
            && self
                .lease_scope
                .as_ref()
                .map(BrokerControlScope::is_valid)
                .unwrap_or(true)
            && self.notes.iter().all(|note| non_empty(note))
    }
}

/// Platform support advertised by a module manifest.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerModulePlatformSupport {
    pub platform: BrokerModulePlatform,
    #[cfg_attr(feature = "serde", serde(default))]
    pub supported: bool,
    pub required_features: Vec<String>,
    pub notes: Vec<String>,
}

impl BrokerModulePlatformSupport {
    pub fn new(platform: BrokerModulePlatform, supported: bool) -> Self {
        Self {
            platform,
            supported,
            required_features: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn with_required_feature(mut self, feature: impl Into<String>) -> Self {
        self.required_features.push(feature.into());
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        self.required_features
            .iter()
            .all(|feature| non_empty(feature))
            && self.notes.iter().all(|note| non_empty(note))
    }
}

/// Timestamp behavior expected for streams emitted or consumed by a module.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerModuleTimestampBehavior {
    pub source_timestamp_domain: BrokerTimestampDomain,
    #[cfg_attr(feature = "serde", serde(default))]
    pub source_timestamp_required: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub broker_receive_timestamp_required: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub preserves_source_timestamps: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub emits_chunk_timestamps: bool,
    pub max_clock_error_ns: Option<u64>,
}

impl BrokerModuleTimestampBehavior {
    pub fn new(source_timestamp_domain: BrokerTimestampDomain) -> Self {
        Self {
            source_timestamp_domain,
            source_timestamp_required: false,
            broker_receive_timestamp_required: true,
            preserves_source_timestamps: true,
            emits_chunk_timestamps: false,
            max_clock_error_ns: None,
        }
    }

    pub const fn with_source_timestamp_required(mut self, required: bool) -> Self {
        self.source_timestamp_required = required;
        self
    }

    pub const fn with_broker_receive_timestamp_required(mut self, required: bool) -> Self {
        self.broker_receive_timestamp_required = required;
        self
    }

    pub const fn with_preserves_source_timestamps(mut self, preserves: bool) -> Self {
        self.preserves_source_timestamps = preserves;
        self
    }

    pub const fn with_emits_chunk_timestamps(mut self, emits: bool) -> Self {
        self.emits_chunk_timestamps = emits;
        self
    }

    pub const fn with_max_clock_error_ns(mut self, max_clock_error_ns: u64) -> Self {
        self.max_clock_error_ns = Some(max_clock_error_ns);
        self
    }

    pub fn is_valid(&self) -> bool {
        self.max_clock_error_ns
            .map(|error_ns| error_ns > 0)
            .unwrap_or(true)
    }
}

/// Clock-correlation policy advertised by a module.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerModuleClockPolicy {
    pub broker_clock_domain: BrokerTimestampDomain,
    #[cfg_attr(feature = "serde", serde(default))]
    pub correlation_required: bool,
    pub allowed_source_domains: Vec<BrokerTimestampDomain>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub clock_health_required: bool,
}

impl BrokerModuleClockPolicy {
    pub fn broker_elapsed() -> Self {
        Self {
            broker_clock_domain: BrokerTimestampDomain::ElapsedRealtime,
            correlation_required: false,
            allowed_source_domains: vec![BrokerTimestampDomain::ElapsedRealtime],
            clock_health_required: true,
        }
    }

    pub const fn with_correlation_required(mut self, required: bool) -> Self {
        self.correlation_required = required;
        self
    }

    pub fn with_allowed_source_domain(mut self, domain: BrokerTimestampDomain) -> Self {
        if !self.allowed_source_domains.contains(&domain) {
            self.allowed_source_domains.push(domain);
        }
        self
    }

    pub const fn with_clock_health_required(mut self, required: bool) -> Self {
        self.clock_health_required = required;
        self
    }

    pub fn is_valid(&self) -> bool {
        !self.allowed_source_domains.is_empty()
            && (!self.correlation_required
                || !self
                    .allowed_source_domains
                    .contains(&BrokerTimestampDomain::Unknown))
    }
}

/// Health metric descriptor and optional runtime value for one module.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerModuleHealthMetric {
    pub metric: String,
    pub label: String,
    pub unit: Option<String>,
    pub healthy_min: Option<f64>,
    pub healthy_max: Option<f64>,
    pub observed_value: Option<f64>,
    pub state: BrokerModuleHealthState,
}

impl BrokerModuleHealthMetric {
    pub fn new(metric: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            metric: metric.into(),
            label: label.into(),
            unit: None,
            healthy_min: None,
            healthy_max: None,
            observed_value: None,
            state: BrokerModuleHealthState::Unknown,
        }
    }

    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    pub const fn with_healthy_range(mut self, min: f64, max: f64) -> Self {
        self.healthy_min = Some(min);
        self.healthy_max = Some(max);
        self
    }

    pub const fn with_observed_value(mut self, value: f64) -> Self {
        self.observed_value = Some(value);
        self
    }

    pub const fn with_state(mut self, state: BrokerModuleHealthState) -> Self {
        self.state = state;
        self
    }

    pub fn is_valid(&self) -> bool {
        non_empty(&self.metric)
            && non_empty(&self.label)
            && self.unit.as_deref().map(non_empty).unwrap_or(true)
            && self
                .observed_value
                .map(|value| value.is_finite())
                .unwrap_or(true)
            && match (self.healthy_min, self.healthy_max) {
                (Some(min), Some(max)) => min.is_finite() && max.is_finite() && min <= max,
                (Some(value), None) | (None, Some(value)) => value.is_finite(),
                (None, None) => true,
            }
    }
}

/// Failure handling expectation for one module.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerModuleFailurePolicy {
    pub action: BrokerModuleFailureAction,
    pub max_restart_attempts: Option<u32>,
    pub restart_cooldown_elapsed_ns: Option<u64>,
    pub notes: Vec<String>,
}

impl BrokerModuleFailurePolicy {
    pub fn new(action: BrokerModuleFailureAction) -> Self {
        Self {
            action,
            max_restart_attempts: None,
            restart_cooldown_elapsed_ns: None,
            notes: Vec::new(),
        }
    }

    pub const fn with_max_restart_attempts(mut self, attempts: u32) -> Self {
        self.max_restart_attempts = Some(attempts);
        self
    }

    pub const fn with_restart_cooldown_elapsed_ns(mut self, elapsed_ns: u64) -> Self {
        self.restart_cooldown_elapsed_ns = Some(elapsed_ns);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        self.restart_cooldown_elapsed_ns
            .map(|elapsed_ns| elapsed_ns > 0)
            .unwrap_or(true)
            && self.notes.iter().all(|note| non_empty(note))
    }
}

/// Schema-only capability manifest for one broker-managed module.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerModuleManifest {
    pub schema: String,
    pub module_id: String,
    pub module_kind: BrokerModuleKind,
    pub label: String,
    pub version: String,
    pub provided_streams: Vec<BrokerModuleStreamBinding>,
    pub consumed_streams: Vec<BrokerModuleStreamBinding>,
    pub accepted_commands: Vec<BrokerModuleCommandDescriptor>,
    pub required_permissions: Vec<BrokerModulePermissionRequirement>,
    pub required_external_tools: Vec<BrokerModuleExternalToolRequirement>,
    pub platform_support: Vec<BrokerModulePlatformSupport>,
    pub resource_locks: Vec<BrokerModuleResourceLock>,
    pub timestamp_behavior: BrokerModuleTimestampBehavior,
    pub clock_policy: BrokerModuleClockPolicy,
    pub data_sensitivity: BrokerDataSensitivity,
    pub retention_policy: BrokerStreamRetentionPolicy,
    pub ui_subscription_policy: BrokerUiSubscriptionPolicy,
    pub chart_policy: BrokerChartPolicy,
    pub health_metrics: Vec<BrokerModuleHealthMetric>,
    pub failure_policy: BrokerModuleFailurePolicy,
    pub panel_descriptors: Vec<BrokerPanelDescriptor>,
    pub notes: Vec<String>,
}

impl BrokerModuleManifest {
    pub fn new(
        module_id: impl Into<String>,
        module_kind: BrokerModuleKind,
        label: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            schema: BROKER_MODULE_MANIFEST_SCHEMA.to_string(),
            module_id: module_id.into(),
            module_kind,
            label: label.into(),
            version: version.into(),
            provided_streams: Vec::new(),
            consumed_streams: Vec::new(),
            accepted_commands: Vec::new(),
            required_permissions: Vec::new(),
            required_external_tools: Vec::new(),
            platform_support: Vec::new(),
            resource_locks: Vec::new(),
            timestamp_behavior: BrokerModuleTimestampBehavior::new(
                BrokerTimestampDomain::ElapsedRealtime,
            ),
            clock_policy: BrokerModuleClockPolicy::broker_elapsed(),
            data_sensitivity: BrokerDataSensitivity::Unknown,
            retention_policy: BrokerStreamRetentionPolicy::None,
            ui_subscription_policy: BrokerUiSubscriptionPolicy::ManualOnly,
            chart_policy: BrokerChartPolicy::NotChartable,
            health_metrics: Vec::new(),
            failure_policy: BrokerModuleFailurePolicy::new(BrokerModuleFailureAction::ReportOnly),
            panel_descriptors: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn with_provided_stream(mut self, stream: BrokerModuleStreamBinding) -> Self {
        self.provided_streams.push(stream);
        self
    }

    pub fn with_consumed_stream(mut self, stream: BrokerModuleStreamBinding) -> Self {
        self.consumed_streams.push(stream);
        self
    }

    pub fn with_accepted_command(mut self, command: BrokerModuleCommandDescriptor) -> Self {
        self.accepted_commands.push(command);
        self
    }

    pub fn with_required_permission(
        mut self,
        permission: BrokerModulePermissionRequirement,
    ) -> Self {
        self.required_permissions.push(permission);
        self
    }

    pub fn with_required_external_tool(
        mut self,
        tool: BrokerModuleExternalToolRequirement,
    ) -> Self {
        self.required_external_tools.push(tool);
        self
    }

    pub fn with_platform_support(mut self, support: BrokerModulePlatformSupport) -> Self {
        self.platform_support.push(support);
        self
    }

    pub fn with_resource_lock(mut self, lock: BrokerModuleResourceLock) -> Self {
        self.resource_locks.push(lock);
        self
    }

    pub fn with_timestamp_behavior(mut self, behavior: BrokerModuleTimestampBehavior) -> Self {
        self.timestamp_behavior = behavior;
        self
    }

    pub fn with_clock_policy(mut self, policy: BrokerModuleClockPolicy) -> Self {
        self.clock_policy = policy;
        self
    }

    pub const fn with_data_sensitivity(mut self, data_sensitivity: BrokerDataSensitivity) -> Self {
        self.data_sensitivity = data_sensitivity;
        self
    }

    pub const fn with_retention_policy(
        mut self,
        retention_policy: BrokerStreamRetentionPolicy,
    ) -> Self {
        self.retention_policy = retention_policy;
        self
    }

    pub const fn with_ui_subscription_policy(
        mut self,
        ui_subscription_policy: BrokerUiSubscriptionPolicy,
    ) -> Self {
        self.ui_subscription_policy = ui_subscription_policy;
        self
    }

    pub const fn with_chart_policy(mut self, chart_policy: BrokerChartPolicy) -> Self {
        self.chart_policy = chart_policy;
        self
    }

    pub fn with_health_metric(mut self, metric: BrokerModuleHealthMetric) -> Self {
        self.health_metrics.push(metric);
        self
    }

    pub fn with_failure_policy(mut self, policy: BrokerModuleFailurePolicy) -> Self {
        self.failure_policy = policy;
        self
    }

    pub fn with_panel_descriptor(mut self, panel: BrokerPanelDescriptor) -> Self {
        self.panel_descriptors.push(panel);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn provided_stream_ids(&self) -> Vec<&str> {
        self.provided_streams
            .iter()
            .map(|stream| stream.stream_id.as_str())
            .collect()
    }

    pub fn consumed_stream_ids(&self) -> Vec<&str> {
        self.consumed_streams
            .iter()
            .map(|stream| stream.stream_id.as_str())
            .collect()
    }

    pub fn accepted_command_names(&self) -> Vec<&str> {
        self.accepted_commands
            .iter()
            .map(|command| command.command.as_str())
            .collect()
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_MODULE_MANIFEST_SCHEMA
            && valid_module_id(&self.module_id)
            && non_empty(&self.label)
            && non_empty(&self.version)
            && self
                .provided_streams
                .iter()
                .all(BrokerModuleStreamBinding::is_valid)
            && self
                .consumed_streams
                .iter()
                .all(BrokerModuleStreamBinding::is_valid)
            && self
                .accepted_commands
                .iter()
                .all(BrokerModuleCommandDescriptor::is_valid)
            && self
                .required_permissions
                .iter()
                .all(BrokerModulePermissionRequirement::is_valid)
            && self
                .required_external_tools
                .iter()
                .all(BrokerModuleExternalToolRequirement::is_valid)
            && !self.platform_support.is_empty()
            && self
                .platform_support
                .iter()
                .all(BrokerModulePlatformSupport::is_valid)
            && self
                .resource_locks
                .iter()
                .all(BrokerModuleResourceLock::is_valid)
            && self.timestamp_behavior.is_valid()
            && self.clock_policy.is_valid()
            && self
                .health_metrics
                .iter()
                .all(BrokerModuleHealthMetric::is_valid)
            && self.failure_policy.is_valid()
            && self
                .panel_descriptors
                .iter()
                .all(BrokerPanelDescriptor::is_valid)
            && self.notes.iter().all(|note| non_empty(note))
    }
}

/// Runtime state snapshot for one broker-managed module.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerModuleRuntimeState {
    pub schema: String,
    pub module_id: String,
    pub module_kind: BrokerModuleKind,
    pub lifecycle_state: BrokerModuleLifecycleState,
    pub revision: u64,
    pub last_transition_elapsed_ns: Option<u64>,
    pub provided_stream_ids: Vec<String>,
    pub consumed_stream_ids: Vec<String>,
    pub active_resource_locks: Vec<BrokerModuleResourceLock>,
    pub health_metrics: Vec<BrokerModuleHealthMetric>,
    pub issue_codes: Vec<String>,
}

impl BrokerModuleRuntimeState {
    pub fn new(
        module_id: impl Into<String>,
        module_kind: BrokerModuleKind,
        lifecycle_state: BrokerModuleLifecycleState,
    ) -> Self {
        Self {
            schema: BROKER_MODULE_RUNTIME_STATE_SCHEMA.to_string(),
            module_id: module_id.into(),
            module_kind,
            lifecycle_state,
            revision: 0,
            last_transition_elapsed_ns: None,
            provided_stream_ids: Vec::new(),
            consumed_stream_ids: Vec::new(),
            active_resource_locks: Vec::new(),
            health_metrics: Vec::new(),
            issue_codes: Vec::new(),
        }
    }

    pub fn from_manifest(
        manifest: &BrokerModuleManifest,
        lifecycle_state: BrokerModuleLifecycleState,
        revision: u64,
    ) -> Self {
        let mut state = Self::new(
            manifest.module_id.clone(),
            manifest.module_kind,
            lifecycle_state,
        )
        .with_revision(revision);
        state.provided_stream_ids = manifest
            .provided_streams
            .iter()
            .map(|stream| stream.stream_id.clone())
            .collect();
        state.consumed_stream_ids = manifest
            .consumed_streams
            .iter()
            .map(|stream| stream.stream_id.clone())
            .collect();
        state.health_metrics = manifest.health_metrics.clone();
        state
    }

    pub const fn with_revision(mut self, revision: u64) -> Self {
        self.revision = revision;
        self
    }

    pub const fn with_last_transition_elapsed_ns(mut self, elapsed_ns: u64) -> Self {
        self.last_transition_elapsed_ns = Some(elapsed_ns);
        self
    }

    pub fn with_provided_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.provided_stream_ids.push(stream_id.into());
        self
    }

    pub fn with_consumed_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.consumed_stream_ids.push(stream_id.into());
        self
    }

    pub fn with_active_resource_lock(mut self, lock: BrokerModuleResourceLock) -> Self {
        self.active_resource_locks.push(lock);
        self
    }

    pub fn with_health_metric(mut self, metric: BrokerModuleHealthMetric) -> Self {
        self.health_metrics.push(metric);
        self
    }

    pub fn with_issue_code(mut self, issue_code: impl Into<String>) -> Self {
        self.issue_codes.push(issue_code.into());
        self
    }

    pub const fn is_available(&self) -> bool {
        self.lifecycle_state.is_available()
    }

    pub fn health_metric(&self, metric: &str) -> Option<&BrokerModuleHealthMetric> {
        self.health_metrics
            .iter()
            .find(|candidate| candidate.metric == metric)
    }

    pub fn line(&self) -> String {
        format!(
            "{} / {} / {} / {} provided / {} consumed",
            self.module_id,
            self.module_kind.as_str(),
            self.lifecycle_state.as_str(),
            self.provided_stream_ids.len(),
            self.consumed_stream_ids.len()
        )
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_MODULE_RUNTIME_STATE_SCHEMA
            && valid_module_id(&self.module_id)
            && self
                .last_transition_elapsed_ns
                .map(|elapsed_ns| elapsed_ns > 0)
                .unwrap_or(true)
            && self
                .provided_stream_ids
                .iter()
                .all(|stream_id| non_empty(stream_id))
            && self
                .consumed_stream_ids
                .iter()
                .all(|stream_id| non_empty(stream_id))
            && self
                .active_resource_locks
                .iter()
                .all(BrokerModuleResourceLock::is_valid)
            && self
                .health_metrics
                .iter()
                .all(BrokerModuleHealthMetric::is_valid)
            && self.issue_codes.iter().all(|issue| non_empty(issue))
    }
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_module_id(value: &str) -> bool {
    non_empty(value)
        && value.chars().all(|ch| {
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '.' | '_' | '-')
        })
        && value
            .chars()
            .next()
            .map(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
            .unwrap_or(false)
        && value
            .chars()
            .last()
            .map(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{
        BrokerModuleClockPolicy, BrokerModuleCommandDescriptor, BrokerModuleFailureAction,
        BrokerModuleFailurePolicy, BrokerModuleHealthMetric, BrokerModuleHealthState,
        BrokerModuleKind, BrokerModuleLifecycleState, BrokerModuleManifest,
        BrokerModulePermissionKind, BrokerModulePermissionRequirement, BrokerModulePlatform,
        BrokerModulePlatformSupport, BrokerModuleResourceLock, BrokerModuleRuntimeState,
        BrokerModuleStreamBinding, BrokerModuleTimestampBehavior, BROKER_MODULE_MANIFEST_SCHEMA,
        BROKER_MODULE_RUNTIME_STATE_SCHEMA,
    };
    use crate::{
        BrokerChartPolicy, BrokerCommandAuthorityRequirement, BrokerControlScope,
        BrokerDataSensitivity, BrokerPayloadKind, BrokerStreamKind, BrokerStreamRateClass,
        BrokerStreamRetentionPolicy, BrokerTimestampDomain, BrokerUiSubscriptionPolicy,
    };

    fn synthetic_manifest() -> BrokerModuleManifest {
        let stream = BrokerModuleStreamBinding::new(
            "synthetic:wave",
            "Synthetic wave",
            BrokerStreamKind::Synthetic,
            BrokerPayloadKind::Json,
            "rusty.xr.synthetic.wave.v1",
            BrokerDataSensitivity::Public,
        )
        .with_recommended_rate_hz(30.0)
        .with_rate_class(BrokerStreamRateClass::LowRateTelemetry)
        .with_retention_policy(BrokerStreamRetentionPolicy::RollingWindow)
        .with_ui_subscription_policy(BrokerUiSubscriptionPolicy::AutoSubscribeLowRate)
        .with_chart_policy(BrokerChartPolicy::LowRateDirect);
        let authority = BrokerCommandAuthorityRequirement::read_only(
            "synthetic.wave.get_status",
            "runtime.synthetic",
        );

        BrokerModuleManifest::new(
            "synthetic.wave",
            BrokerModuleKind::Provider,
            "Synthetic wave provider",
            "0.1.0",
        )
        .with_provided_stream(stream)
        .with_accepted_command(BrokerModuleCommandDescriptor::new(
            "synthetic.wave.get_status",
            "Get status",
            authority,
        ))
        .with_required_permission(
            BrokerModulePermissionRequirement::new(
                "broker.synthetic.read",
                "Read synthetic status",
                BrokerModulePermissionKind::BrokerCapability,
            )
            .with_operator_message("Allows read-only synthetic status queries."),
        )
        .with_platform_support(BrokerModulePlatformSupport::new(
            BrokerModulePlatform::Any,
            true,
        ))
        .with_resource_lock(
            BrokerModuleResourceLock::new("stream:synthetic:wave", "Synthetic wave stream")
                .with_lease_scope(BrokerControlScope::new(
                    "runtime.synthetic",
                    "runtime.synthetic",
                )),
        )
        .with_timestamp_behavior(
            BrokerModuleTimestampBehavior::new(BrokerTimestampDomain::ElapsedRealtime)
                .with_broker_receive_timestamp_required(true),
        )
        .with_clock_policy(BrokerModuleClockPolicy::broker_elapsed())
        .with_data_sensitivity(BrokerDataSensitivity::Public)
        .with_retention_policy(BrokerStreamRetentionPolicy::RollingWindow)
        .with_ui_subscription_policy(BrokerUiSubscriptionPolicy::AutoSubscribeLowRate)
        .with_chart_policy(BrokerChartPolicy::LowRateDirect)
        .with_health_metric(
            BrokerModuleHealthMetric::new("samples_emitted", "Samples emitted")
                .with_observed_value(120.0)
                .with_state(BrokerModuleHealthState::Healthy),
        )
        .with_failure_policy(BrokerModuleFailurePolicy::new(
            BrokerModuleFailureAction::ReportOnly,
        ))
    }

    #[test]
    fn module_manifest_lists_capabilities_and_policies() {
        let manifest = synthetic_manifest();

        assert!(manifest.is_valid());
        assert_eq!(manifest.schema, BROKER_MODULE_MANIFEST_SCHEMA);
        assert_eq!(manifest.module_kind.as_str(), "provider");
        assert_eq!(manifest.provided_stream_ids(), vec!["synthetic:wave"]);
        assert_eq!(
            manifest.accepted_command_names(),
            vec!["synthetic.wave.get_status"]
        );
        assert_eq!(manifest.platform_support.len(), 1);
    }

    #[test]
    fn runtime_state_can_be_derived_from_manifest() {
        let manifest = synthetic_manifest();
        let runtime = BrokerModuleRuntimeState::from_manifest(
            &manifest,
            BrokerModuleLifecycleState::Active,
            7,
        )
        .with_last_transition_elapsed_ns(1_000_000);

        assert!(runtime.is_valid());
        assert!(runtime.is_available());
        assert_eq!(runtime.schema, BROKER_MODULE_RUNTIME_STATE_SCHEMA);
        assert_eq!(runtime.provided_stream_ids, vec!["synthetic:wave"]);
        assert!(runtime.health_metric("samples_emitted").is_some());
        assert!(runtime.line().contains("synthetic.wave"));
    }

    #[test]
    fn invalid_module_manifest_rejects_private_or_empty_shapes() {
        let invalid_id = BrokerModuleManifest::new(
            "Synthetic Wave",
            BrokerModuleKind::Provider,
            "Synthetic wave provider",
            "0.1.0",
        );
        let no_platform = BrokerModuleManifest::new(
            "synthetic.wave",
            BrokerModuleKind::Provider,
            "Synthetic wave provider",
            "0.1.0",
        );

        assert!(!invalid_id.is_valid());
        assert!(!no_platform.is_valid());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn public_module_manifest_fixture_deserializes() {
        let manifest: BrokerModuleManifest = serde_json::from_str(include_str!(
            "../../../fixtures/broker-ui/synthetic-module-manifest.json"
        ))
        .expect("public module manifest fixture should deserialize");

        assert!(manifest.is_valid());
        assert_eq!(manifest.module_id, "synthetic.wave");
        assert_eq!(manifest.module_kind, BrokerModuleKind::Provider);
        assert_eq!(manifest.provided_stream_ids(), vec!["synthetic:wave"]);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn public_module_runtime_state_fixture_deserializes() {
        let state: BrokerModuleRuntimeState = serde_json::from_str(include_str!(
            "../../../fixtures/broker-ui/synthetic-module-runtime-state.json"
        ))
        .expect("public module runtime fixture should deserialize");

        assert!(state.is_valid());
        assert!(state.is_available());
        assert_eq!(state.module_id, "synthetic.wave");
        assert_eq!(state.lifecycle_state, BrokerModuleLifecycleState::Active);
    }
}
