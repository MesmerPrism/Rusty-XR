//! Broker host manifest contracts.
//!
//! Host manifests describe where the authoritative broker is running and how a
//! UI or companion may reach it. They do not grant authority; command
//! capabilities, leases, revisions, and operator gates still belong to command
//! execution.

use crate::{
    BrokerSecurityMode, BrokerTimestampDomain, BrokerTransportEndpoint, BrokerTransportKind,
    BrokerTransportSecurityPolicy,
};

/// Versioned JSON schema id for broker host manifests.
pub const BROKER_HOST_MANIFEST_SCHEMA: &str = "rusty.xr.broker.host_manifest.v1";

/// Read-only command name for requesting a broker host manifest.
pub const BROKER_HOST_MANIFEST_COMMAND: &str = "broker.host_manifest";

/// Public HTTP path for a broker host manifest.
pub const BROKER_HOST_MANIFEST_HTTP_PATH: &str = "/broker/host_manifest";

/// Deployment role for the broker host that owns command authority.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerHostAuthorityRole {
    HeadsetLocalPrimary,
    DesktopPrimary,
    EmbeddedInProcessPrimary,
    RelayPrimary,
    Observer,
    Unknown,
}

impl BrokerHostAuthorityRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HeadsetLocalPrimary => "headset_local_primary",
            Self::DesktopPrimary => "desktop_primary",
            Self::EmbeddedInProcessPrimary => "embedded_in_process_primary",
            Self::RelayPrimary => "relay_primary",
            Self::Observer => "observer",
            Self::Unknown => "unknown",
        }
    }
}

/// Visibility class for an advertised broker endpoint.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerEndpointVisibility {
    Loopback,
    AdbForwarded,
    PairedLan,
    PublicRelay,
    ExternalSidecar,
    Hidden,
    Unknown,
}

impl BrokerEndpointVisibility {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Loopback => "loopback",
            Self::AdbForwarded => "adb_forwarded",
            Self::PairedLan => "paired_lan",
            Self::PublicRelay => "public_relay",
            Self::ExternalSidecar => "external_sidecar",
            Self::Hidden => "hidden",
            Self::Unknown => "unknown",
        }
    }

    pub const fn requires_non_loopback_security(self) -> bool {
        matches!(self, Self::PairedLan | Self::PublicRelay)
    }
}

/// One endpoint exposed by a broker host.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerHostEndpointDescriptor {
    pub endpoint_id: String,
    pub label: String,
    pub endpoint: BrokerTransportEndpoint,
    pub visibility: BrokerEndpointVisibility,
    pub command_scope: String,
    pub primary: bool,
}

impl BrokerHostEndpointDescriptor {
    pub fn new(
        endpoint_id: impl Into<String>,
        label: impl Into<String>,
        endpoint: BrokerTransportEndpoint,
        visibility: BrokerEndpointVisibility,
        command_scope: impl Into<String>,
    ) -> Self {
        Self {
            endpoint_id: endpoint_id.into(),
            label: label.into(),
            endpoint,
            visibility,
            command_scope: command_scope.into(),
            primary: false,
        }
    }

    pub const fn with_primary(mut self, primary: bool) -> Self {
        self.primary = primary;
        self
    }

    pub fn is_visible_to_ui(&self) -> bool {
        !matches!(self.visibility, BrokerEndpointVisibility::Hidden)
    }

    pub fn is_valid_for_security(&self, security: &BrokerTransportSecurityPolicy) -> bool {
        if !self.endpoint.is_valid() || !security.allows_endpoint(&self.endpoint) {
            return false;
        }
        match self.visibility {
            BrokerEndpointVisibility::Loopback => self.endpoint.is_loopback(),
            BrokerEndpointVisibility::AdbForwarded => {
                self.endpoint.transport == BrokerTransportKind::AdbForwardedTcp
                    || self.endpoint.is_loopback()
            }
            BrokerEndpointVisibility::PairedLan | BrokerEndpointVisibility::PublicRelay => {
                security.non_loopback_allowed
                    && !matches!(security.mode, BrokerSecurityMode::LoopbackOnly)
            }
            BrokerEndpointVisibility::ExternalSidecar => {
                matches!(
                    self.endpoint.transport,
                    BrokerTransportKind::ExternalSidecar | BrokerTransportKind::MetadataOnly
                ) || security.non_loopback_allowed
            }
            BrokerEndpointVisibility::Hidden | BrokerEndpointVisibility::Unknown => true,
        }
    }

    pub fn is_valid(&self) -> bool {
        non_empty(&self.endpoint_id)
            && non_empty(&self.label)
            && self.endpoint.is_valid()
            && non_empty(&self.command_scope)
    }
}

/// Deployment and endpoint manifest for one broker host.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerHostManifest {
    pub schema: String,
    pub host_id: String,
    pub label: String,
    pub authority_role: BrokerHostAuthorityRole,
    pub endpoints: Vec<BrokerHostEndpointDescriptor>,
    pub capabilities: Vec<String>,
    pub security: BrokerTransportSecurityPolicy,
    pub broker_clock_domain: BrokerTimestampDomain,
    pub session_manifest_required: bool,
    pub observed_elapsed_ns: Option<u64>,
    pub notes: Vec<String>,
}

impl BrokerHostManifest {
    pub fn new(
        host_id: impl Into<String>,
        label: impl Into<String>,
        authority_role: BrokerHostAuthorityRole,
        security: BrokerTransportSecurityPolicy,
    ) -> Self {
        Self {
            schema: BROKER_HOST_MANIFEST_SCHEMA.to_string(),
            host_id: host_id.into(),
            label: label.into(),
            authority_role,
            endpoints: Vec::new(),
            capabilities: Vec::new(),
            security,
            broker_clock_domain: BrokerTimestampDomain::ElapsedRealtime,
            session_manifest_required: true,
            observed_elapsed_ns: None,
            notes: Vec::new(),
        }
    }

    pub fn headset_local(host_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(
            host_id,
            label,
            BrokerHostAuthorityRole::HeadsetLocalPrimary,
            BrokerTransportSecurityPolicy::loopback_only(),
        )
    }

    pub fn with_endpoint(mut self, endpoint: BrokerHostEndpointDescriptor) -> Self {
        self.endpoints.push(endpoint);
        self
    }

    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities.push(capability.into());
        self
    }

    pub const fn with_broker_clock_domain(mut self, domain: BrokerTimestampDomain) -> Self {
        self.broker_clock_domain = domain;
        self
    }

    pub const fn with_session_manifest_required(mut self, required: bool) -> Self {
        self.session_manifest_required = required;
        self
    }

    pub const fn with_observed_elapsed_ns(mut self, observed_elapsed_ns: u64) -> Self {
        self.observed_elapsed_ns = Some(observed_elapsed_ns);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn primary_endpoint(&self) -> Option<&BrokerHostEndpointDescriptor> {
        self.endpoints
            .iter()
            .find(|endpoint| endpoint.primary)
            .or_else(|| self.endpoints.first())
    }

    pub fn visible_endpoints(&self) -> Vec<&BrokerHostEndpointDescriptor> {
        self.endpoints
            .iter()
            .filter(|endpoint| endpoint.is_visible_to_ui())
            .collect()
    }

    pub fn supports_capability(&self, capability: &str) -> bool {
        self.capabilities
            .iter()
            .any(|candidate| candidate == capability)
    }

    pub fn endpoint_visibility_line(&self) -> String {
        if self.endpoints.is_empty() {
            return "no endpoints".to_string();
        }
        self.endpoints
            .iter()
            .map(|endpoint| {
                format!(
                    "{}:{}:{}",
                    endpoint.endpoint_id,
                    endpoint.visibility.as_str(),
                    endpoint.command_scope
                )
            })
            .collect::<Vec<_>>()
            .join(" / ")
    }

    pub fn summary_line(&self) -> String {
        format!(
            "{} / {} / {} endpoint(s) / {} capability(ies) / security {:?}",
            self.host_id,
            self.authority_role.as_str(),
            self.endpoints.len(),
            self.capabilities.len(),
            self.security.mode
        )
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_HOST_MANIFEST_SCHEMA
            && non_empty(&self.host_id)
            && non_empty(&self.label)
            && self.security.is_valid()
            && self
                .endpoints
                .iter()
                .all(BrokerHostEndpointDescriptor::is_valid)
            && self
                .endpoints
                .iter()
                .all(|endpoint| endpoint.is_valid_for_security(&self.security))
            && self
                .capabilities
                .iter()
                .all(|capability| non_empty(capability))
            && self.notes.iter().all(|note| non_empty(note))
    }
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::{
        BrokerEndpointVisibility, BrokerHostEndpointDescriptor, BrokerHostManifest,
        BROKER_HOST_MANIFEST_SCHEMA,
    };
    use crate::{
        BrokerSecurityMode, BrokerTimestampDomain, BrokerTransportEndpoint, BrokerTransportKind,
        BrokerTransportSecurityPolicy,
    };

    #[test]
    fn loopback_host_manifest_is_valid_and_summarizable() {
        let manifest = BrokerHostManifest::headset_local("quest-broker", "Quest broker")
            .with_endpoint(
                BrokerHostEndpointDescriptor::new(
                    "ws-loopback",
                    "Loopback WebSocket",
                    BrokerTransportEndpoint::websocket("/rustyxr/v1/events"),
                    BrokerEndpointVisibility::Loopback,
                    "broker.control",
                )
                .with_primary(true),
            )
            .with_capability("broker.status.read")
            .with_capability("broker.stream_registry.read")
            .with_broker_clock_domain(BrokerTimestampDomain::ElapsedRealtime);

        assert!(manifest.is_valid());
        assert_eq!(manifest.schema, BROKER_HOST_MANIFEST_SCHEMA);
        assert!(manifest.supports_capability("broker.status.read"));
        assert!(manifest.summary_line().contains("headset_local_primary"));
        assert!(manifest.endpoint_visibility_line().contains("loopback"));
    }

    #[test]
    fn paired_lan_endpoint_requires_non_loopback_security() {
        let endpoint = BrokerHostEndpointDescriptor::new(
            "lan-ws",
            "Paired LAN WebSocket",
            BrokerTransportEndpoint {
                transport: BrokerTransportKind::WebSocket,
                host: Some("192.0.2.10".to_string()),
                port: Some(8765),
                path: Some("/rustyxr/v1/events".to_string()),
                channel_id: None,
                max_datagram_bytes: None,
                auth_required: true,
            },
            BrokerEndpointVisibility::PairedLan,
            "broker.control",
        );

        let loopback_manifest =
            BrokerHostManifest::headset_local("broker", "Broker").with_endpoint(endpoint.clone());
        assert!(!loopback_manifest.is_valid());

        let paired_security = BrokerTransportSecurityPolicy {
            schema: crate::BROKER_TRANSPORT_SECURITY_POLICY_SCHEMA.to_string(),
            mode: BrokerSecurityMode::PairingToken,
            non_loopback_allowed: true,
            pairing_token_required: true,
            expires_elapsed_ns: Some(10_000),
            capability_scope: vec!["broker.pairing".to_string()],
        };
        let paired_manifest = BrokerHostManifest::new(
            "broker",
            "Broker",
            super::BrokerHostAuthorityRole::HeadsetLocalPrimary,
            paired_security,
        )
        .with_endpoint(endpoint);

        assert!(paired_manifest.is_valid());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn public_host_manifest_fixture_deserializes() {
        let manifest: BrokerHostManifest = serde_json::from_str(include_str!(
            "../../../fixtures/broker-host/synthetic-host-manifest.json"
        ))
        .expect("fixture should deserialize");

        assert!(manifest.is_valid());
        assert_eq!(manifest.host_id, "synthetic-headset-broker");
        assert_eq!(manifest.visible_endpoints().len(), 2);
    }
}
