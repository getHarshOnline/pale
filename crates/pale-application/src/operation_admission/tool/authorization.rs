//! Explicit owner authorization and native confirmation policy.

use pale_core::records::{
    Approval, CapabilityRequirementIdV1, OperationCapabilityContractV1, ToolAuthorizationDigest,
};
use serde::Serialize;

use crate::session_state::ActiveOperationLease;

use super::super::{AdmittedOperation, OperationAdmissionError, ToolInvocation};

#[derive(Serialize)]
struct ToolAuthorizationPreimage<'a> {
    schema: &'static str,
    operation_binding_digest: &'a pale_core::records::OperationBindingDigest,
    operation_contract_digest: &'a pale_core::records::OperationContractDigest,
    instance_digest: &'a pale_core::records::InstanceDigest,
    requirement_id: &'a CapabilityRequirementIdV1,
    provider_binding: &'a pale_core::records::ProviderBindingRefV1,
    tool_name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_input: Option<&'a serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<&'a str>,
}

/// Opaque proof that the active operation carried exact owner authorization for
/// this tool request.
///
/// Only successful operation admission can mint the value. The run-policy gate
/// may borrow it but cannot reconstruct or persist its authorization material.
pub struct ExactToolAuthorization {
    _private: (),
}

impl ExactToolAuthorization {
    pub(crate) fn from_resident_permission(_: &crate::resident::NativeHookPermission) -> Self {
        Self { _private: () }
    }
}

impl std::fmt::Debug for ExactToolAuthorization {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ExactToolAuthorization(<redacted>)")
    }
}

/// Native owner confirmation requested for one admitted tool call.
pub struct ToolConfirmationRequest {
    requirement: CapabilityRequirementIdV1,
    reason: String,
}

impl ToolConfirmationRequest {
    /// Capability requirement awaiting confirmation.
    pub fn requirement(&self) -> &CapabilityRequirementIdV1 {
        &self.requirement
    }

    /// Bounded explanation for the native confirmation UI.
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// Result of applying the selected approval policy to one exact tool call.
pub(super) enum ApprovalDecision {
    Proceed,
    Exact(ExactToolAuthorization),
    Confirm(ToolConfirmationRequest),
}

fn authorization_digest(
    operation: &AdmittedOperation<'_>,
    lease: &ActiveOperationLease,
    capability: &OperationCapabilityContractV1,
    invocation: &ToolInvocation<'_>,
) -> Result<ToolAuthorizationDigest, OperationAdmissionError> {
    let scope = ToolAuthorizationPreimage {
        schema: "pale/tool-authorization-scope.v1",
        operation_binding_digest: lease.binding_digest(),
        operation_contract_digest: lease.contract_digest(),
        instance_digest: invocation.instance_digest,
        requirement_id: &capability.requirement_id,
        provider_binding: &operation.bundle.catalog().provider_binding,
        tool_name: invocation.tool_name,
        tool_input: invocation.tool_input,
        command: invocation.command,
    };
    ToolAuthorizationDigest::hash_canonical(&scope).map_err(OperationAdmissionError::from)
}

fn require_authorization(
    operation: &AdmittedOperation<'_>,
    lease: &ActiveOperationLease,
    capability: &OperationCapabilityContractV1,
    invocation: &ToolInvocation<'_>,
) -> Result<ApprovalDecision, OperationAdmissionError> {
    let scope_digest = authorization_digest(operation, lease, capability, invocation)?;
    if lease.authorized_actions().get(&capability.requirement_id) != Some(&scope_digest) {
        return Err(OperationAdmissionError::ToolAuthorizationRequired {
            operation: operation.id().clone(),
            requirement: capability.requirement_id.clone(),
            scope_digest,
        });
    }
    Ok(ApprovalDecision::Exact(ExactToolAuthorization {
        _private: (),
    }))
}

/// Resolve approval without weakening `ask` when native confirmation is unavailable.
pub(super) fn approval_decision(
    operation: &AdmittedOperation<'_>,
    lease: &ActiveOperationLease,
    capability: &OperationCapabilityContractV1,
    invocation: &ToolInvocation<'_>,
) -> Result<ApprovalDecision, OperationAdmissionError> {
    match capability.provider_state.approval() {
        None => Err(OperationAdmissionError::Unavailable {
            operation: operation.contract.id.clone(),
        }),
        Some(Approval::None) => Ok(ApprovalDecision::Proceed),
        Some(Approval::Ask)
            if crate::hooks::tool_confirmation(invocation.adapter)
                == crate::hooks::ToolConfirmation::NativeAsk =>
        {
            Ok(ApprovalDecision::Confirm(ToolConfirmationRequest {
                requirement: capability.requirement_id.clone(),
                reason: format!(
                    "Pale operation `{}` requires confirmation for `{}`.",
                    operation.contract.id, capability.requirement_id
                ),
            }))
        }
        Some(Approval::Ask | Approval::Gated) => {
            require_authorization(operation, lease, capability, invocation)
        }
    }
}
