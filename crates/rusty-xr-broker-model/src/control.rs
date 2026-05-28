//! Broker command authority and lease contracts.
//!
//! These types describe the broker-side checks a UI or companion should expect
//! before issuing mutating commands. They are data contracts only; enforcement
//! belongs to the broker implementation that owns the runtime state.

/// Versioned JSON schema id for broker control scopes.
pub const BROKER_CONTROL_SCOPE_SCHEMA: &str = "rusty.xr.broker.control_scope.v1";

/// Versioned JSON schema id for broker control leases.
pub const BROKER_CONTROL_LEASE_SCHEMA: &str = "rusty.xr.broker.control_lease.v1";

/// Versioned JSON schema id for broker command authority requirements.
pub const BROKER_COMMAND_AUTHORITY_REQUIREMENT_SCHEMA: &str =
    "rusty.xr.broker.command_authority_requirement.v1";

/// Versioned JSON schema id for broker command preconditions.
pub const BROKER_COMMAND_PRECONDITION_SCHEMA: &str = "rusty.xr.broker.command_precondition.v1";

/// How a broker command may affect broker-owned or target-owned state.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerCommandMutationClass {
    ReadOnly,
    Mutating,
    ExclusiveLease,
    ExternalGate,
}

/// Broker-visible lifecycle for a control lease.
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(rename_all = "snake_case")
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrokerControlLeaseState {
    Offered,
    Active,
    Expired,
    Revoked,
    Released,
    Denied,
}

/// Broker-owned control surface or resource that can be protected by a lease.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerControlScope {
    pub schema: String,
    pub scope_id: String,
    pub command_scope: String,
    pub resource_id: Option<String>,
}

impl BrokerControlScope {
    pub fn new(scope_id: impl Into<String>, command_scope: impl Into<String>) -> Self {
        Self {
            schema: BROKER_CONTROL_SCOPE_SCHEMA.to_string(),
            scope_id: scope_id.into(),
            command_scope: command_scope.into(),
            resource_id: None,
        }
    }

    pub fn with_resource_id(mut self, resource_id: impl Into<String>) -> Self {
        self.resource_id = Some(resource_id.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_CONTROL_SCOPE_SCHEMA
            && !self.scope_id.trim().is_empty()
            && !self.command_scope.trim().is_empty()
    }
}

/// Revision and lease preconditions attached to a command request.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerCommandPrecondition {
    pub schema: String,
    pub expected_revision: Option<u64>,
    pub lease_id: Option<String>,
    pub holder_client_id: Option<String>,
}

impl BrokerCommandPrecondition {
    pub fn new() -> Self {
        Self {
            schema: BROKER_COMMAND_PRECONDITION_SCHEMA.to_string(),
            expected_revision: None,
            lease_id: None,
            holder_client_id: None,
        }
    }

    pub const fn with_expected_revision(mut self, expected_revision: u64) -> Self {
        self.expected_revision = Some(expected_revision);
        self
    }

    pub fn with_lease_id(mut self, lease_id: impl Into<String>) -> Self {
        self.lease_id = Some(lease_id.into());
        self
    }

    pub fn with_holder_client_id(mut self, holder_client_id: impl Into<String>) -> Self {
        self.holder_client_id = Some(holder_client_id.into());
        self
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_COMMAND_PRECONDITION_SCHEMA
            && self
                .lease_id
                .as_deref()
                .map(|lease_id| !lease_id.trim().is_empty())
                .unwrap_or(true)
            && self
                .holder_client_id
                .as_deref()
                .map(|client_id| !client_id.trim().is_empty())
                .unwrap_or(true)
    }
}

impl Default for BrokerCommandPrecondition {
    fn default() -> Self {
        Self::new()
    }
}

/// Broker-advertised authority requirements for one command.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerCommandAuthorityRequirement {
    pub schema: String,
    pub command: String,
    pub command_scope: String,
    pub mutation_class: BrokerCommandMutationClass,
    pub required_capability: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub required_capabilities: Vec<String>,
    pub required_role: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub allowed_roles: Vec<String>,
    pub lease_required: bool,
    pub required_lease_scope: Option<BrokerControlScope>,
    pub required_revision: Option<u64>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub revision_required: bool,
    pub operator_confirm_required: bool,
}

impl BrokerCommandAuthorityRequirement {
    pub fn read_only(command: impl Into<String>, command_scope: impl Into<String>) -> Self {
        Self {
            schema: BROKER_COMMAND_AUTHORITY_REQUIREMENT_SCHEMA.to_string(),
            command: command.into(),
            command_scope: command_scope.into(),
            mutation_class: BrokerCommandMutationClass::ReadOnly,
            required_capability: None,
            required_capabilities: Vec::new(),
            required_role: None,
            allowed_roles: Vec::new(),
            lease_required: false,
            required_lease_scope: None,
            required_revision: None,
            revision_required: false,
            operator_confirm_required: false,
        }
    }

    pub fn mutating(
        command: impl Into<String>,
        command_scope: impl Into<String>,
        required_lease_scope: BrokerControlScope,
    ) -> Self {
        Self {
            schema: BROKER_COMMAND_AUTHORITY_REQUIREMENT_SCHEMA.to_string(),
            command: command.into(),
            command_scope: command_scope.into(),
            mutation_class: BrokerCommandMutationClass::ExclusiveLease,
            required_capability: None,
            required_capabilities: Vec::new(),
            required_role: None,
            allowed_roles: Vec::new(),
            lease_required: true,
            required_lease_scope: Some(required_lease_scope),
            required_revision: None,
            revision_required: true,
            operator_confirm_required: true,
        }
    }

    pub fn with_required_capability(mut self, capability: impl Into<String>) -> Self {
        self.required_capability = Some(capability.into());
        self
    }

    pub fn with_required_capabilities<I, S>(mut self, capabilities: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.required_capabilities = capabilities.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_required_role(mut self, role: impl Into<String>) -> Self {
        self.required_role = Some(role.into());
        self
    }

    pub fn with_allowed_role(mut self, role: impl Into<String>) -> Self {
        self.allowed_roles.push(role.into());
        self
    }

    pub const fn with_required_revision(mut self, revision: u64) -> Self {
        self.required_revision = Some(revision);
        self
    }

    pub const fn with_revision_required(mut self, required: bool) -> Self {
        self.revision_required = required;
        self
    }

    pub const fn with_operator_confirm_required(mut self, required: bool) -> Self {
        self.operator_confirm_required = required;
        self
    }

    pub fn precondition(&self, lease_id: Option<String>) -> BrokerCommandPrecondition {
        let mut precondition = BrokerCommandPrecondition::new();
        precondition.expected_revision = self.required_revision;
        precondition.lease_id = lease_id;
        precondition
    }

    pub fn precondition_for_revision(
        &self,
        lease_id: Option<String>,
        current_revision: u64,
    ) -> BrokerCommandPrecondition {
        let mut precondition = self.precondition(lease_id);
        if self.revision_required && precondition.expected_revision.is_none() {
            precondition.expected_revision = Some(current_revision);
        }
        precondition
    }

    pub fn is_read_only(&self) -> bool {
        matches!(self.mutation_class, BrokerCommandMutationClass::ReadOnly) && !self.lease_required
    }

    pub fn requires_revision(&self) -> bool {
        self.revision_required || self.required_revision.is_some()
    }

    pub fn required_capability_names(&self) -> Vec<&str> {
        let mut capabilities = Vec::new();
        if let Some(capability) = self.required_capability.as_deref() {
            capabilities.push(capability);
        }
        for capability in &self.required_capabilities {
            let capability = capability.as_str();
            if !capabilities.iter().any(|existing| *existing == capability) {
                capabilities.push(capability);
            }
        }
        capabilities
    }

    pub fn is_valid(&self) -> bool {
        let mutating_requires_authority = matches!(
            self.mutation_class,
            BrokerCommandMutationClass::Mutating
                | BrokerCommandMutationClass::ExclusiveLease
                | BrokerCommandMutationClass::ExternalGate
        );

        self.schema == BROKER_COMMAND_AUTHORITY_REQUIREMENT_SCHEMA
            && !self.command.trim().is_empty()
            && !self.command_scope.trim().is_empty()
            && self
                .required_capability
                .as_deref()
                .map(|capability| !capability.trim().is_empty())
                .unwrap_or(true)
            && self
                .required_capabilities
                .iter()
                .all(|capability| !capability.trim().is_empty())
            && self
                .required_role
                .as_deref()
                .map(|role| !role.trim().is_empty())
                .unwrap_or(true)
            && self
                .allowed_roles
                .iter()
                .all(|role| !role.trim().is_empty())
            && self
                .required_lease_scope
                .as_ref()
                .map(BrokerControlScope::is_valid)
                .unwrap_or(!self.lease_required)
            && (!matches!(
                self.mutation_class,
                BrokerCommandMutationClass::ExclusiveLease
            ) || self.lease_required)
            && (!mutating_requires_authority
                || (!self.required_capability_names().is_empty() && self.requires_revision()))
    }
}

/// Broker-issued temporary authority for one control scope.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerControlLease {
    pub schema: String,
    pub lease_id: String,
    pub holder_client_id: String,
    pub scope: BrokerControlScope,
    pub granted_revision: u64,
    pub expires_elapsed_ns: Option<u64>,
    pub state: BrokerControlLeaseState,
}

impl BrokerControlLease {
    pub fn new(
        lease_id: impl Into<String>,
        holder_client_id: impl Into<String>,
        scope: BrokerControlScope,
        granted_revision: u64,
    ) -> Self {
        Self {
            schema: BROKER_CONTROL_LEASE_SCHEMA.to_string(),
            lease_id: lease_id.into(),
            holder_client_id: holder_client_id.into(),
            scope,
            granted_revision,
            expires_elapsed_ns: None,
            state: BrokerControlLeaseState::Active,
        }
    }

    pub const fn with_expires_elapsed_ns(mut self, expires_elapsed_ns: u64) -> Self {
        self.expires_elapsed_ns = Some(expires_elapsed_ns);
        self
    }

    pub const fn with_state(mut self, state: BrokerControlLeaseState) -> Self {
        self.state = state;
        self
    }

    pub fn matches_scope_at_revision(
        &self,
        scope: &BrokerControlScope,
        current_revision: u64,
    ) -> bool {
        self.state == BrokerControlLeaseState::Active
            && self.scope.scope_id == scope.scope_id
            && self.scope.command_scope == scope.command_scope
            && self.scope.resource_id == scope.resource_id
            && self.granted_revision <= current_revision
    }

    pub fn is_active_for(
        &self,
        scope: &BrokerControlScope,
        holder_client_id: &str,
        current_revision: u64,
        current_elapsed_ns: u64,
    ) -> bool {
        self.matches_scope_at_revision(scope, current_revision)
            && self.holder_client_id == holder_client_id
            && self
                .expires_elapsed_ns
                .map(|expires_elapsed_ns| current_elapsed_ns < expires_elapsed_ns)
                .unwrap_or(true)
    }

    pub fn is_valid(&self) -> bool {
        self.schema == BROKER_CONTROL_LEASE_SCHEMA
            && !self.lease_id.trim().is_empty()
            && !self.holder_client_id.trim().is_empty()
            && self.scope.is_valid()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BrokerCommandAuthorityRequirement, BrokerCommandMutationClass, BrokerCommandPrecondition,
        BrokerControlLease, BrokerControlScope, BROKER_COMMAND_PRECONDITION_SCHEMA,
    };

    #[test]
    fn mutating_authority_requires_valid_lease_scope() {
        let scope =
            BrokerControlScope::new("runtime.bio", "runtime.bio").with_resource_id("bio:breath");
        let authority =
            BrokerCommandAuthorityRequirement::mutating("runtime.bio.pause", "runtime.bio", scope)
                .with_required_capability("broker.bio.control")
                .with_required_role("operator")
                .with_allowed_role("operator")
                .with_required_revision(7);

        assert!(authority.is_valid());
        assert_eq!(
            authority.mutation_class,
            BrokerCommandMutationClass::ExclusiveLease
        );
        assert!(!authority.is_read_only());
        assert_eq!(
            authority.required_capability_names(),
            vec!["broker.bio.control"]
        );
        assert!(authority.requires_revision());

        let precondition = authority.precondition(Some("lease-1".to_string()));
        assert_eq!(precondition.expected_revision, Some(7));
        assert_eq!(precondition.lease_id.as_deref(), Some("lease-1"));
        assert!(precondition.is_valid());
    }

    #[test]
    fn read_only_authority_does_not_require_lease() {
        let authority =
            BrokerCommandAuthorityRequirement::read_only("status_request", "session.lifecycle");

        assert!(authority.is_valid());
        assert!(authority.is_read_only());
        assert!(authority.required_lease_scope.is_none());
    }

    #[test]
    fn mutating_authority_requires_capability_and_revision_gate() {
        let scope = BrokerControlScope::new("runtime.bio", "runtime.bio");
        let missing_capability =
            BrokerCommandAuthorityRequirement::mutating("runtime.bio.pause", "runtime.bio", scope);
        let missing_revision =
            BrokerCommandAuthorityRequirement::read_only("runtime.bio.pause", "runtime.bio")
                .with_required_capability("broker.bio.control")
                .with_operator_confirm_required(true);

        let missing_revision = BrokerCommandAuthorityRequirement {
            mutation_class: BrokerCommandMutationClass::Mutating,
            ..missing_revision
        };

        assert!(!missing_capability.is_valid());
        assert!(!missing_revision.is_valid());
        assert!(missing_capability
            .with_required_capabilities(["broker.bio.control"])
            .with_revision_required(true)
            .is_valid());
    }

    #[test]
    fn active_lease_matches_scope_holder_revision_and_expiry() {
        let scope = BrokerControlScope::new("session.lifecycle", "session.lifecycle");
        let lease = BrokerControlLease::new("lease-1", "client-1", scope.clone(), 3)
            .with_expires_elapsed_ns(10);

        assert!(lease.is_valid());
        assert!(lease.matches_scope_at_revision(&scope, 4));
        assert!(!lease.matches_scope_at_revision(&scope, 2));
        assert!(lease.is_active_for(&scope, "client-1", 4, 9));
        assert!(!lease.is_active_for(&scope, "client-2", 4, 9));
        assert!(!lease.is_active_for(&scope, "client-1", 4, 10));
        assert!(!lease.is_active_for(
            &BrokerControlScope::new("session.lifecycle", "different.scope"),
            "client-1",
            4,
            9
        ));
    }

    #[test]
    fn default_command_precondition_is_schema_valid() {
        let precondition = BrokerCommandPrecondition::default();

        assert_eq!(precondition.schema, BROKER_COMMAND_PRECONDITION_SCHEMA);
        assert!(precondition.is_valid());
    }
}
