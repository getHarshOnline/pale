//! One-shot host invocation correlation and lease replacement.

use pale_core::records::CapabilityRequirementIdV1;
use serde::Serialize;

use crate::session_state::{
    ActiveOperationLease, InflightAdmissionStatus, SessionRuntimeState, SessionStateError,
};

use super::super::{
    invocation::{ToolInvocationDigest, ToolUseIdentityDigest},
    AdmittedOperation, OperationAdmissionError, ToolInvocation,
};

#[derive(Serialize)]
struct ToolInvocationPreimage<'a> {
    schema: &'static str,
    operation_binding_digest: &'a pale_core::records::OperationBindingDigest,
    operation_contract_digest: &'a pale_core::records::OperationContractDigest,
    profile_digest: &'a pale_core::records::ProfileDigest,
    repository_id: &'a pale_core::records::LogicalRepositoryIdV1,
    execution_path_witness: &'a pale_core::records::ExecutionPathWitnessV1,
    activity: &'a pale_core::records::HarnessActivityIdentity,
    instance_digest: &'a pale_core::records::InstanceDigest,
    tool_use_id_digest: &'a ToolUseIdentityDigest,
    provider_binding: &'a pale_core::records::ProviderBindingRefV1,
    tool_name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_input: Option<&'a serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<&'a str>,
}

/// Candidate lease replacement kept separate until its owning evidence write succeeds.
pub(super) struct ToolStateTransition {
    replacement: ActiveOperationLease,
    confirmed: Option<ActiveOperationLease>,
}

impl ToolStateTransition {
    /// Construct a transition that needs no later confirmation promotion.
    pub(super) fn new(replacement: ActiveOperationLease) -> Self {
        Self {
            replacement,
            confirmed: None,
        }
    }

    fn pending_confirmation(
        replacement: ActiveOperationLease,
        confirmed: ActiveOperationLease,
    ) -> Self {
        Self {
            replacement,
            confirmed: Some(confirmed),
        }
    }

    /// Replace only the exact predecessor lease observed during admission.
    pub(super) fn apply_to(
        &self,
        state: &mut SessionRuntimeState,
        expected: &ActiveOperationLease,
    ) -> Result<bool, SessionStateError> {
        state.replace_operation_if_current(Some(expected), Some(&self.replacement))
    }

    /// Classify persisted state without mutating an unrelated successor lease.
    pub(super) fn confirmation_state(
        &self,
        state: &SessionRuntimeState,
        predecessor: &ActiveOperationLease,
    ) -> ConfirmationRegistrationState {
        if state.active_operation() == Some(predecessor) {
            return ConfirmationRegistrationState::Predecessor;
        }
        if state.active_operation() == Some(&self.replacement) {
            return ConfirmationRegistrationState::Provisional;
        }
        if self
            .confirmed
            .as_ref()
            .is_some_and(|confirmed| state.active_operation() == Some(confirmed))
        {
            return ConfirmationRegistrationState::Finalized;
        }
        ConfirmationRegistrationState::Different
    }

    /// Promote the provisional lease only after confirmation evidence is durable.
    pub(super) fn confirm_after_evidence(
        &self,
        state: &mut SessionRuntimeState,
    ) -> Result<bool, SessionStateError> {
        let Some(confirmed) = self.confirmed.as_ref() else {
            return Ok(false);
        };
        state.replace_operation_if_current(Some(&self.replacement), Some(confirmed))
    }

    /// Restore the predecessor when confirmation evidence cannot be committed.
    pub(super) fn revoke_confirmation(
        &self,
        state: &mut SessionRuntimeState,
        predecessor: &ActiveOperationLease,
    ) -> Result<ConfirmationRevocation, SessionStateError> {
        match self.confirmation_state(state, predecessor) {
            ConfirmationRegistrationState::Predecessor => {
                Ok(ConfirmationRevocation::AlreadyRestored)
            }
            ConfirmationRegistrationState::Provisional => {
                let restored = state
                    .replace_operation_if_current(Some(&self.replacement), Some(predecessor))?;
                Ok(if restored {
                    ConfirmationRevocation::RestoredProvisional
                } else {
                    ConfirmationRevocation::Different
                })
            }
            ConfirmationRegistrationState::Finalized => {
                let Some(confirmed) = self.confirmed.as_ref() else {
                    return Ok(ConfirmationRevocation::Different);
                };
                let restored =
                    state.replace_operation_if_current(Some(confirmed), Some(predecessor))?;
                Ok(if restored {
                    ConfirmationRevocation::RestoredFinalized
                } else {
                    ConfirmationRevocation::Different
                })
            }
            ConfirmationRegistrationState::Different => Ok(ConfirmationRevocation::Different),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Exact relationship between persisted state and one native confirmation.
pub enum ConfirmationRegistrationState {
    /// The operation lease observed before the tool call is current.
    Predecessor,
    /// The tool call is recorded but cannot be consumed before evidence exists.
    Provisional,
    /// Durable evidence promoted the tool call for one post-tool correlation.
    Finalized,
    /// Another state is current and must not be overwritten.
    Different,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Result of revoking one exact confirmation registration.
pub enum ConfirmationRevocation {
    /// The predecessor was already current and no write was needed.
    AlreadyRestored,
    /// A provisional registration was replaced by its predecessor.
    RestoredProvisional,
    /// A finalized registration was demoted to its predecessor.
    RestoredFinalized,
    /// A different state was preserved without modification.
    Different,
}

/// Verify that a lease still belongs to the admitted bundle, contract, and turn.
pub(super) fn verify_lease(
    operation: &AdmittedOperation<'_>,
    lease: &ActiveOperationLease,
    turn_digest: Option<&pale_core::records::TurnIdentityDigest>,
) -> Result<(), OperationAdmissionError> {
    if operation.id() != lease.operation_id()
        || operation.bundle.binding_digest() != lease.binding_digest()
        || operation.bundle.catalog_digest() != lease.catalog_digest()
        || &operation.contract_digest != lease.contract_digest()
    {
        return Err(OperationAdmissionError::StaleLease);
    }
    if let Some(bound_turn) = lease.turn_digest() {
        if Some(bound_turn) != turn_digest {
            return Err(OperationAdmissionError::CrossTurnLease);
        }
    }
    Ok(())
}

fn invocation_digest(
    operation: &AdmittedOperation<'_>,
    lease: &ActiveOperationLease,
    invocation: &ToolInvocation<'_>,
) -> Result<ToolInvocationDigest, OperationAdmissionError> {
    ToolInvocationDigest::hash_canonical(&ToolInvocationPreimage {
        schema: "pale/tool-invocation.v1",
        operation_binding_digest: lease.binding_digest(),
        operation_contract_digest: lease.contract_digest(),
        profile_digest: operation.bundle.profile_digest(),
        repository_id: operation.bundle.repository_id(),
        execution_path_witness: operation.bundle.execution_path_witness(),
        activity: &invocation.activity,
        instance_digest: invocation.instance_digest,
        tool_use_id_digest: &invocation.tool_use_id_digest,
        provider_binding: &operation.bundle.catalog().provider_binding,
        tool_name: invocation.tool_name,
        tool_input: invocation.tool_input,
        command: invocation.command,
    })
}

/// Register one invocation exactly once and prepare any confirmation transition.
pub(super) fn register_invocation(
    operation: &AdmittedOperation<'_>,
    lease: &ActiveOperationLease,
    invocation: &ToolInvocation<'_>,
    requirement_id: Option<&CapabilityRequirementIdV1>,
    confirmation_pending: bool,
) -> Result<ToolStateTransition, OperationAdmissionError> {
    let invocation_digest = invocation_digest(operation, lease, invocation)?;
    let mut replacement = lease.clone();
    let status = if confirmation_pending {
        InflightAdmissionStatus::EvidencePending
    } else {
        InflightAdmissionStatus::Admitted
    };
    if !replacement.register_inflight(
        invocation.tool_use_id_digest.clone(),
        invocation_digest.clone(),
        requirement_id,
        status,
    )? {
        return Err(OperationAdmissionError::ToolInvocationReplay);
    }
    if !confirmation_pending {
        return Ok(ToolStateTransition::new(replacement));
    }
    let mut confirmed = replacement.clone();
    if !confirmed.confirm_inflight(&invocation.tool_use_id_digest, &invocation_digest) {
        return Err(OperationAdmissionError::ToolInvocationNotAdmitted);
    }
    Ok(ToolStateTransition::pending_confirmation(
        replacement,
        confirmed,
    ))
}

/// Consume the matching admitted invocation at its post-tool boundary.
pub(super) fn consume_invocation(
    operation: &AdmittedOperation<'_>,
    lease: &ActiveOperationLease,
    invocation: &ToolInvocation<'_>,
    requirement_id: Option<&CapabilityRequirementIdV1>,
) -> Result<ActiveOperationLease, OperationAdmissionError> {
    let invocation_digest = invocation_digest(operation, lease, invocation)?;
    let mut replacement = lease.clone();
    match replacement.consume_inflight(&invocation.tool_use_id_digest, &invocation_digest) {
        crate::session_state::InflightConsumption::Missing => {
            Err(OperationAdmissionError::ToolInvocationNotAdmitted)
        }
        crate::session_state::InflightConsumption::Mismatch => {
            Err(OperationAdmissionError::ToolInvocationMismatch)
        }
        crate::session_state::InflightConsumption::NotAdmitted => {
            Err(OperationAdmissionError::ToolInvocationNotAdmitted)
        }
        crate::session_state::InflightConsumption::Matched {
            requirement_id: admitted,
        } if admitted.as_ref() == requirement_id => Ok(replacement),
        crate::session_state::InflightConsumption::Matched { .. } => {
            Err(OperationAdmissionError::ToolInvocationMismatch)
        }
    }
}
