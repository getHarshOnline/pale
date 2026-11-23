//! Bounded structured denials for native-harness string channels.

use pale_core::engine_wire::OperationDenialV1;

use super::OperationAdmissionError;

// Denials cross a native string channel: bound the complete actionable message
// separately from each identity/detail field before canonical encoding.
/// Maximum UTF-8 bytes retained in the complete host-visible denial.
const MAX_MESSAGE_BYTES: usize = 1_024;
/// Maximum UTF-8 bytes retained in one structured denial identity.
const MAX_FIELD_BYTES: usize = 256;

/// Failure to encode an admitted denial for a native harness response channel.
#[derive(Debug, thiserror::Error)]
pub enum DenialEncodingError {
    /// The computed denial no longer satisfies its canonical wire schema.
    #[error("operation denial does not satisfy its schema: {0}")]
    Schema(#[from] pale_contracts::SchemaError),
    /// Canonical JSON encoding of the validated denial failed.
    #[error("operation denial could not be canonicalized: {0}")]
    Canon(#[from] pale_core::canon::CanonError),
}

fn operation_denial_host_reason(denial: &OperationDenialV1) -> Result<String, DenialEncodingError> {
    pale_contracts::validate_record(denial, "operation-denial.v1")?;
    let record = pale_core::canon::to_canonical_string(denial)?;
    Ok(format!("Pale operation admission denied: {record}"))
}

impl OperationAdmissionError {
    /// Encode the stable denial class and actionable fields into the native
    /// harness's required string channel without asking callers to parse prose.
    ///
    /// # Errors
    ///
    /// Returns [`DenialEncodingError`] if the bounded structured record violates
    /// its pinned schema or cannot be encoded as canonical JSON.
    pub fn host_reason(&self) -> Result<String, DenialEncodingError> {
        operation_denial_host_reason(&self.denial()?)
    }

    fn bounded_field(value: &str) -> String {
        crate::bounded_text::digest_preview(value, MAX_FIELD_BYTES)
    }

    fn denial(&self) -> Result<OperationDenialV1, pale_contracts::SchemaError> {
        let schema_identity = pale_contracts::record_schema_identity("operation-denial.v1")?;
        let mut denial = OperationDenialV1 {
            schema: schema_identity.schema,
            schema_sha256: schema_identity.schema_sha256,
            code: self.code().into(),
            message: crate::bounded_text::digest_preview(&self.to_string(), MAX_MESSAGE_BYTES),
            operation: None,
            requirement: None,
            tool: None,
            detail: None,
            scope_digest: None,
        };
        match self {
            Self::ArtifactDigestMismatch { artifact }
            | Self::MalformedArtifact { artifact }
            | Self::SemanticClosureMismatch { artifact } => {
                denial.detail = Some(Self::bounded_field(artifact));
            }
            Self::EntryContractMismatch { operation } | Self::Unavailable { operation } => {
                denial.operation = Some(Self::bounded_field(&operation.to_string()));
            }
            Self::UnknownOperation { operation } => {
                denial.operation = Some(Self::bounded_field(operation));
            }
            Self::InvalidOperationRecord { family, .. } => {
                denial.detail = Some(Self::bounded_field(family));
            }
            Self::DuplicateIdentity { identity } => {
                denial.detail = Some(Self::bounded_field(identity));
            }
            Self::UnknownAuthorization {
                operation,
                requirement,
            }
            | Self::OperationHalted {
                operation,
                requirement,
            }
            | Self::OperationEscalated {
                operation,
                requirement,
            }
            | Self::FailureObservationUnavailable {
                operation,
                requirement,
                ..
            } => {
                denial.operation = Some(Self::bounded_field(&operation.to_string()));
                denial.requirement = Some(Self::bounded_field(requirement.as_str()));
            }
            Self::ToolOutsideContract { operation, tool } => {
                denial.operation = Some(Self::bounded_field(&operation.to_string()));
                denial.tool = Some(Self::bounded_field(tool));
            }
            Self::ToolAuthorizationRequired {
                operation,
                requirement,
                scope_digest,
            } => {
                denial.operation = Some(Self::bounded_field(&operation.to_string()));
                denial.requirement = Some(Self::bounded_field(requirement.as_str()));
                denial.scope_digest = Some(scope_digest.clone());
            }
            Self::AmbiguousToolContract {
                operation,
                tool,
                requirements,
            } => {
                denial.operation = Some(Self::bounded_field(&operation.to_string()));
                denial.tool = Some(Self::bounded_field(tool));
                denial.detail = Some(Self::bounded_field(requirements));
            }
            Self::OperationSteering {
                operation,
                requirement,
                effect,
            } => {
                denial.operation = Some(Self::bounded_field(&operation.to_string()));
                denial.requirement = Some(Self::bounded_field(requirement.as_str()));
                denial.detail = Some(Self::bounded_field(&effect.to_string()));
            }
            Self::OperationRetrying {
                operation,
                requirement,
                attempted,
            } => {
                denial.operation = Some(Self::bounded_field(&operation.to_string()));
                denial.requirement = Some(Self::bounded_field(requirement.as_str()));
                denial.detail = Some(Self::bounded_field(attempted));
            }
            Self::BindingCatalogMismatch
            | Self::RouteContextMismatch
            | Self::InvocationOriginMismatch
            | Self::InvocationPathMismatch
            | Self::MissingToolUseIdentity
            | Self::InvalidToolUseIdentity
            | Self::MissingActivityIdentity
            | Self::HostIdentity(_)
            | Self::ToolInvocationReplay
            | Self::ToolInvocationNotAdmitted
            | Self::ToolInvocationMismatch
            | Self::FailureTransitionMissing
            | Self::MalformedAuthorization
            | Self::StaleLease
            | Self::CrossTurnLease
            | Self::Canon(_)
            | Self::SessionState(_) => {}
        }
        Ok(denial)
    }

    /// Return the stable denial code shared by native-host and diagnostic projections.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ArtifactDigestMismatch { .. } => "artifact_digest_mismatch",
            Self::MalformedArtifact { .. } => "malformed_artifact",
            Self::SemanticClosureMismatch { .. } => "semantic_closure_mismatch",
            Self::InvalidOperationRecord { .. } => "invalid_operation_record",
            Self::DuplicateIdentity { .. } => "duplicate_identity",
            Self::BindingCatalogMismatch => "binding_catalog_mismatch",
            Self::RouteContextMismatch => "route_context_mismatch",
            Self::EntryContractMismatch { .. } => "entry_contract_mismatch",
            Self::UnknownOperation { .. } => "unknown_operation",
            Self::InvocationOriginMismatch => "invocation_origin_mismatch",
            Self::InvocationPathMismatch => "invocation_path_mismatch",
            Self::MissingToolUseIdentity => "missing_tool_use_identity",
            Self::InvalidToolUseIdentity => "invalid_tool_use_identity",
            Self::MissingActivityIdentity => "missing_activity_identity",
            Self::HostIdentity(_) => "invalid_host_identity",
            Self::Unavailable { .. } => "operation_unavailable",
            Self::MalformedAuthorization => "malformed_authorization",
            Self::UnknownAuthorization { .. } => "unknown_authorization",
            Self::StaleLease => "stale_lease",
            Self::CrossTurnLease => "cross_turn_lease",
            Self::ToolInvocationReplay => "tool_invocation_replay",
            Self::ToolInvocationNotAdmitted => "tool_invocation_not_admitted",
            Self::ToolInvocationMismatch => "tool_invocation_mismatch",
            Self::FailureTransitionMissing => "failure_transition_missing",
            Self::ToolOutsideContract { .. } => "tool_outside_contract",
            Self::ToolAuthorizationRequired { .. } => "tool_authorization_required",
            Self::AmbiguousToolContract { .. } => "ambiguous_tool_contract",
            Self::OperationHalted { .. } => "operation_halted",
            Self::OperationSteering { .. } => "operation_steering",
            Self::OperationRetrying { .. } => "operation_retrying",
            Self::OperationEscalated { .. } => "operation_escalated",
            Self::FailureObservationUnavailable { .. } => "failure_observation_unavailable",
            Self::Canon(_) => "canonicalization_failed",
            Self::SessionState(_) => "session_state_rejected",
        }
    }
}
