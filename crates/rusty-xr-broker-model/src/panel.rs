//! Broker-described UI panel contracts.
//!
//! These contracts let a broker describe safe read-only panels and future
//! lease-aware controls without depending on a UI framework.

use crate::{BrokerCommandAuthorityRequirement, BrokerCommandMutationClass, BrokerControlScope};

/// Versioned JSON schema id for broker panel descriptor documents.
pub const BROKER_PANEL_DESCRIPTOR_DOCUMENT_SCHEMA: &str = "rusty.xr.broker.panel_descriptor_set.v1";

/// High-level broker panel category.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerPanelKind {
    StateCard,
    CommandGroup,
    StreamList,
    TelemetryChart,
    DomainStatus,
    Custom,
}

/// Public sensitivity category for broker-visible data.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerDataSensitivity {
    Public,
    Diagnostic,
    Mixed,
    Physiology,
    DerivedPhysiology,
    Restricted,
    Unknown,
}

impl BrokerDataSensitivity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Diagnostic => "diagnostic",
            Self::Mixed => "mixed",
            Self::Physiology => "physiology",
            Self::DerivedPhysiology => "derived_physiology",
            Self::Restricted => "restricted",
            Self::Unknown => "unknown",
        }
    }
}

/// Versioned set of broker-described panels.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerPanelDescriptorDocument {
    pub schema: String,
    pub version: String,
    pub panels: Vec<BrokerPanelDescriptor>,
}

impl BrokerPanelDescriptorDocument {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            schema: BROKER_PANEL_DESCRIPTOR_DOCUMENT_SCHEMA.to_string(),
            version: version.into(),
            panels: Vec::new(),
        }
    }

    pub fn with_panel(mut self, panel: BrokerPanelDescriptor) -> Self {
        self.panels.push(panel);
        self
    }

    pub fn read_only_command_count(&self) -> usize {
        self.panels
            .iter()
            .flat_map(|panel| panel.widgets.iter())
            .filter(|widget| {
                matches!(
                    widget,
                    BrokerPanelWidgetDescriptor::CommandButton {
                        read_only: true,
                        lease_required: false,
                        ..
                    }
                )
            })
            .count()
    }

    pub fn stream_list_line(&self) -> String {
        self.panels
            .iter()
            .flat_map(|panel| panel.widgets.iter())
            .find_map(|widget| match widget {
                BrokerPanelWidgetDescriptor::StreamList { stream_ids, .. } => Some(stream_ids),
                _ => None,
            })
            .map(|streams| streams.join(" / "))
            .unwrap_or_else(|| "no descriptor stream list".to_string())
    }

    pub fn telemetry_charts(&self) -> Vec<BrokerTelemetryChartDescriptor> {
        self.panels
            .iter()
            .flat_map(|panel| panel.widgets.iter())
            .filter_map(BrokerPanelWidgetDescriptor::telemetry_chart_descriptor)
            .collect()
    }

    pub fn default_chart(&self) -> Option<BrokerTelemetryChartDescriptor> {
        self.telemetry_charts().into_iter().next()
    }

    pub fn command_authority_requirements(&self) -> Vec<BrokerCommandAuthorityRequirement> {
        self.panels
            .iter()
            .flat_map(|panel| panel.widgets.iter())
            .filter_map(BrokerPanelWidgetDescriptor::command_authority_requirement)
            .collect()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "{} / {} panels / {} read-only commands / {} telemetry chart(s)",
            self.version,
            self.panels.len(),
            self.read_only_command_count(),
            self.telemetry_charts().len()
        )
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_PANEL_DESCRIPTOR_DOCUMENT_SCHEMA
            && !self.version.trim().is_empty()
            && !self.panels.is_empty()
            && self.panels.iter().all(BrokerPanelDescriptor::is_valid)
    }
}

/// One broker-described panel.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerPanelDescriptor {
    pub id: String,
    pub title: String,
    pub kind: BrokerPanelKind,
    pub data_sensitivity: BrokerDataSensitivity,
    pub command_scope: String,
    pub required_capability: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub lease_required: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub widgets: Vec<BrokerPanelWidgetDescriptor>,
}

impl BrokerPanelDescriptor {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        kind: BrokerPanelKind,
        data_sensitivity: BrokerDataSensitivity,
        command_scope: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            kind,
            data_sensitivity,
            command_scope: command_scope.into(),
            required_capability: None,
            lease_required: false,
            widgets: Vec::new(),
        }
    }

    pub fn with_required_capability(mut self, capability: impl Into<String>) -> Self {
        self.required_capability = Some(capability.into());
        self
    }

    pub const fn with_lease_required(mut self, lease_required: bool) -> Self {
        self.lease_required = lease_required;
        self
    }

    pub fn with_widget(mut self, widget: BrokerPanelWidgetDescriptor) -> Self {
        self.widgets.push(widget);
        self
    }

    pub fn is_valid(&self) -> bool {
        !self.id.trim().is_empty()
            && !self.title.trim().is_empty()
            && !self.command_scope.trim().is_empty()
            && self
                .required_capability
                .as_deref()
                .map(|capability| !capability.trim().is_empty())
                .unwrap_or(true)
            && self
                .widgets
                .iter()
                .all(BrokerPanelWidgetDescriptor::is_valid)
    }
}

/// Widget primitive inside a broker-described panel.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(tag = "kind", rename_all = "snake_case")
)]
#[derive(Clone, Debug, PartialEq)]
pub enum BrokerPanelWidgetDescriptor {
    StateCard {
        id: String,
        label: String,
        value_path: String,
    },
    CommandButton {
        id: String,
        label: String,
        command: String,
        #[cfg_attr(feature = "serde", serde(default))]
        read_only: bool,
        command_scope: String,
        required_capability: Option<String>,
        #[cfg_attr(feature = "serde", serde(default))]
        lease_required: bool,
    },
    StreamList {
        id: String,
        label: String,
        stream_ids: Vec<String>,
        data_sensitivity: BrokerDataSensitivity,
    },
    TelemetryChart {
        id: String,
        title: String,
        stream_id: String,
        metric: String,
        x_axis: String,
        y_axis: String,
        max_points: usize,
        data_sensitivity: BrokerDataSensitivity,
        command_scope: String,
        high_rate_policy: String,
    },
}

impl BrokerPanelWidgetDescriptor {
    pub fn telemetry_chart_descriptor(&self) -> Option<BrokerTelemetryChartDescriptor> {
        match self {
            Self::TelemetryChart {
                id,
                title,
                stream_id,
                metric,
                x_axis,
                y_axis,
                max_points,
                data_sensitivity,
                command_scope,
                high_rate_policy,
            } => Some(BrokerTelemetryChartDescriptor {
                id: id.clone(),
                title: title.clone(),
                stream_id: stream_id.clone(),
                metric: metric.clone(),
                x_axis: x_axis.clone(),
                y_axis: y_axis.clone(),
                max_points: *max_points,
                data_sensitivity: *data_sensitivity,
                command_scope: command_scope.clone(),
                high_rate_policy: high_rate_policy.clone(),
            }),
            _ => None,
        }
    }

    pub fn command_authority_requirement(&self) -> Option<BrokerCommandAuthorityRequirement> {
        match self {
            Self::CommandButton {
                command,
                read_only,
                command_scope,
                required_capability,
                lease_required,
                ..
            } => {
                let mut requirement = if *read_only {
                    BrokerCommandAuthorityRequirement::read_only(
                        command.clone(),
                        command_scope.clone(),
                    )
                } else {
                    BrokerCommandAuthorityRequirement {
                        schema: crate::BROKER_COMMAND_AUTHORITY_REQUIREMENT_SCHEMA.to_string(),
                        command: command.clone(),
                        command_scope: command_scope.clone(),
                        mutation_class: if *lease_required {
                            BrokerCommandMutationClass::ExclusiveLease
                        } else {
                            BrokerCommandMutationClass::Mutating
                        },
                        required_capability: None,
                        lease_required: *lease_required,
                        required_lease_scope: lease_required.then(|| {
                            BrokerControlScope::new(command_scope.clone(), command_scope.clone())
                        }),
                        required_revision: None,
                        operator_confirm_required: !*read_only,
                    }
                };
                requirement.required_capability = required_capability.clone();
                Some(requirement)
            }
            _ => None,
        }
    }

    pub fn is_valid(&self) -> bool {
        match self {
            Self::StateCard {
                id,
                label,
                value_path,
            } => non_empty(id) && non_empty(label) && non_empty(value_path),
            Self::CommandButton {
                id,
                label,
                command,
                read_only,
                command_scope,
                required_capability,
                lease_required,
            } => {
                non_empty(id)
                    && non_empty(label)
                    && non_empty(command)
                    && non_empty(command_scope)
                    && required_capability
                        .as_deref()
                        .map(non_empty)
                        .unwrap_or(true)
                    && (*read_only || *lease_required)
            }
            Self::StreamList {
                id,
                label,
                stream_ids,
                ..
            } => {
                non_empty(id)
                    && non_empty(label)
                    && !stream_ids.is_empty()
                    && stream_ids.iter().all(|stream_id| non_empty(stream_id))
            }
            Self::TelemetryChart {
                id,
                title,
                stream_id,
                metric,
                x_axis,
                y_axis,
                max_points,
                command_scope,
                ..
            } => {
                non_empty(id)
                    && non_empty(title)
                    && non_empty(stream_id)
                    && non_empty(metric)
                    && non_empty(x_axis)
                    && non_empty(y_axis)
                    && *max_points > 0
                    && non_empty(command_scope)
            }
        }
    }
}

/// Extracted chart primitive for Makepad, companion, or web renderers.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BrokerTelemetryChartDescriptor {
    pub id: String,
    pub title: String,
    pub stream_id: String,
    pub metric: String,
    pub x_axis: String,
    pub y_axis: String,
    pub max_points: usize,
    pub data_sensitivity: BrokerDataSensitivity,
    pub command_scope: String,
    pub high_rate_policy: String,
}

impl BrokerTelemetryChartDescriptor {
    pub fn label_line(&self) -> String {
        format!(
            "{} / {}.{} / {} points / {}",
            self.title,
            self.stream_id,
            self.metric,
            self.max_points,
            self.data_sensitivity.as_str()
        )
    }

    pub fn is_valid(&self) -> bool {
        non_empty(&self.id)
            && non_empty(&self.title)
            && non_empty(&self.stream_id)
            && non_empty(&self.metric)
            && non_empty(&self.x_axis)
            && non_empty(&self.y_axis)
            && self.max_points > 0
            && non_empty(&self.command_scope)
    }
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::{
        BrokerDataSensitivity, BrokerPanelDescriptor, BrokerPanelDescriptorDocument,
        BrokerPanelKind, BrokerPanelWidgetDescriptor, BROKER_PANEL_DESCRIPTOR_DOCUMENT_SCHEMA,
    };

    #[test]
    fn panel_document_extracts_charts_and_authority_requirements() {
        let document = BrokerPanelDescriptorDocument::new("fixture-panels")
            .with_panel(
                BrokerPanelDescriptor::new(
                    "commands",
                    "Commands",
                    BrokerPanelKind::CommandGroup,
                    BrokerDataSensitivity::Diagnostic,
                    "session.lifecycle",
                )
                .with_widget(BrokerPanelWidgetDescriptor::CommandButton {
                    id: "command.status".to_string(),
                    label: "Status".to_string(),
                    command: "status_request".to_string(),
                    read_only: true,
                    command_scope: "session.lifecycle".to_string(),
                    required_capability: Some("broker.status.read".to_string()),
                    lease_required: false,
                }),
            )
            .with_panel(
                BrokerPanelDescriptor::new(
                    "telemetry",
                    "Telemetry",
                    BrokerPanelKind::TelemetryChart,
                    BrokerDataSensitivity::DerivedPhysiology,
                    "runtime.bio",
                )
                .with_widget(BrokerPanelWidgetDescriptor::TelemetryChart {
                    id: "chart.breath".to_string(),
                    title: "Breath volume".to_string(),
                    stream_id: "bio:breath".to_string(),
                    metric: "volume01".to_string(),
                    x_axis: "time_s".to_string(),
                    y_axis: "volume01".to_string(),
                    max_points: 240,
                    data_sensitivity: BrokerDataSensitivity::DerivedPhysiology,
                    command_scope: "runtime.bio".to_string(),
                    high_rate_policy: "low-rate telemetry only".to_string(),
                }),
            );

        assert!(document.is_valid());
        assert_eq!(document.schema, BROKER_PANEL_DESCRIPTOR_DOCUMENT_SCHEMA);
        assert_eq!(document.read_only_command_count(), 1);
        assert_eq!(document.telemetry_charts().len(), 1);
        assert_eq!(document.command_authority_requirements().len(), 1);
        assert!(document.summary_line().contains("telemetry chart"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn panel_descriptor_fixture_shape_round_trips_with_serde() {
        let fixture = r#"{
          "schema": "rusty.xr.broker.panel_descriptor_set.v1",
          "version": "fixture-panels-20260528",
          "panels": [{
            "id": "telemetry.breath_volume",
            "title": "Breath Volume Telemetry",
            "kind": "telemetry_chart",
            "data_sensitivity": "derived_physiology",
            "command_scope": "runtime.bio",
            "lease_required": false,
            "widgets": [{
              "kind": "telemetry_chart",
              "id": "chart.breath_volume",
              "title": "Breath volume over time",
              "stream_id": "bio:breath",
              "metric": "volume01",
              "x_axis": "time_s",
              "y_axis": "volume01",
              "max_points": 240,
              "data_sensitivity": "derived_physiology",
              "command_scope": "runtime.bio",
              "high_rate_policy": "low-rate telemetry only"
            }]
          }]
        }"#;

        let document: BrokerPanelDescriptorDocument =
            serde_json::from_str(fixture).expect("panel descriptor should deserialize");
        let encoded = serde_json::to_string(&document).expect("panel descriptor should serialize");
        let decoded: BrokerPanelDescriptorDocument =
            serde_json::from_str(&encoded).expect("panel descriptor should round-trip");

        assert_eq!(decoded, document);
        assert!(decoded.is_valid());
        assert_eq!(
            decoded.default_chart().expect("chart expected").stream_id,
            "bio:breath"
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn public_panel_descriptor_fixture_deserializes() {
        let document: BrokerPanelDescriptorDocument = serde_json::from_str(include_str!(
            "../../../fixtures/broker-ui/synthetic-panel-descriptor.json"
        ))
        .expect("public panel descriptor fixture should deserialize");

        assert!(document.is_valid());
        assert_eq!(document.read_only_command_count(), 2);
        assert!(document.stream_list_line().contains("bio:breath"));
        assert_eq!(
            document.default_chart().expect("chart expected").metric,
            "volume01"
        );
    }
}
