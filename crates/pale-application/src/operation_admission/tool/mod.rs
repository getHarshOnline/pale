//! Tool-call admission under one active operation lease.

mod authorization;
mod correlation;
mod matching;

use pale_core::records::{CapabilityRequirementIdV1, FailurePosture};

use crate::session_state::{ActiveOperationLease, SessionRuntimeState, SessionStateError};

use super::{AdmittedOperation, OperationAdmissionError, ToolInvocation};

use authorization::{approval_decision, ApprovalDecision};
pub use authorization::{ExactToolAuthorization, ToolConfirmationRequest};
use correlation::{consume_invocation, register_invocation, verify_lease, ToolStateTransition};
pub use correlation::{ConfirmationRegistrationState, ConfirmationRevocation};
use matching::{enforce_prior_failure, enforce_terminal_failure, match_tool, MatchedTool};

/// One pre-tool call admitted under an exact operation lease and registered as
/// an unconsumed in-flight invocation.
pub struct AdmittedToolCall {
    transition: ToolStateTransition,
    exact_authorization: Option<ExactToolAuthorization>,
    confirmation: Option<ToolConfirmationRequest>,
}

impl std::fmt::Debug for AdmittedToolCall {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdmittedToolCall")
            .field("exact_authorization", &self.exact_authorization.is_some())
            .field("confirmation", &self.confirmation.is_some())
            .finish_non_exhaustive()
    }
}

impl AdmittedToolCall {
    /// Borrow exact owner authorization for the strict run-policy gate.
    pub fn exact_authorization(&self) -> Option<&ExactToolAuthorization> {
        self.exact_authorization.as_ref()
    }

    /// Borrow the native confirmation request, when approval remains pending.
    pub fn confirmation(&self) -> Option<&ToolConfirmationRequest> {
        self.confirmation.as_ref()
    }

    /// Register this invocation if the persisted lease still matches the
    /// snapshot under which it was admitted.
    ///
    /// # Errors
    ///
    /// Returns an error when the replacement violates the bounded lease
    /// contract. A `false` result reports a concurrent lease change.
    pub fn apply_to(
        &self,
        state: &mut SessionRuntimeState,
        expected: &ActiveOperationLease,
    ) -> Result<bool, SessionStateError> {
        self.transition.apply_to(state, expected)
    }

    /// Classify the exact persisted confirmation registration.
    pub fn confirmation_state(
        &self,
        state: &SessionRuntimeState,
        predecessor: &ActiveOperationLease,
    ) -> Option<ConfirmationRegistrationState> {
        self.confirmation.as_ref()?;
        Some(self.transition.confirmation_state(state, predecessor))
    }

    /// Promote a provisional confirmation only after its decision evidence is durable.
    ///
    /// # Errors
    ///
    /// Returns [`SessionStateError`] when the persisted lease cannot perform the
    /// confirmation transition. A `false` result means no matching confirmation
    /// was registered or the lease changed concurrently.
    pub fn confirm_after_evidence(
        &self,
        state: &mut SessionRuntimeState,
    ) -> Result<bool, SessionStateError> {
        if self.confirmation.is_none() {
            return Ok(false);
        }
        self.transition.confirm_after_evidence(state)
    }

    /// Revoke the exact provisional or finalized confirmation registration.
    ///
    /// # Errors
    ///
    /// Returns [`SessionStateError`] when the persisted lease cannot perform the
    /// revocation transition.
    pub fn revoke_confirmation(
        &self,
        state: &mut SessionRuntimeState,
        predecessor: &ActiveOperationLease,
    ) -> Result<ConfirmationRevocation, SessionStateError> {
        if self.confirmation.is_none() {
            return Ok(ConfirmationRevocation::Different);
        }
        self.transition.revoke_confirmation(state, predecessor)
    }
}

/// A matched post-tool failure and its exact one-shot lease transition.
pub struct RecordedToolFailure {
    transition: ToolStateTransition,
    requirement: Option<CapabilityRequirementIdV1>,
    posture: Option<FailurePosture>,
}

impl std::fmt::Debug for RecordedToolFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RecordedToolFailure")
            .field("capability_matched", &self.requirement.is_some())
            .field("posture", &self.posture)
            .finish_non_exhaustive()
    }
}

impl RecordedToolFailure {
    /// Failure posture now attached to the matched capability, if any.
    pub fn posture(&self) -> Option<(&CapabilityRequirementIdV1, FailurePosture)> {
        self.requirement.as_ref().zip(self.posture)
    }

    /// Consume the matched invocation if the persisted lease still matches the
    /// snapshot used to derive this transition.
    ///
    /// # Errors
    ///
    /// Returns an error when the replacement violates the bounded lease
    /// contract. A `false` result reports a concurrent lease change.
    pub fn apply_to(
        &self,
        state: &mut SessionRuntimeState,
        expected: &ActiveOperationLease,
    ) -> Result<bool, SessionStateError> {
        self.transition.apply_to(state, expected)
    }
}

/// Successful completion of one exact in-flight tool invocation.
pub struct CompletedToolCall {
    transition: ToolStateTransition,
}

impl std::fmt::Debug for CompletedToolCall {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CompletedToolCall(<redacted>)")
    }
}

impl CompletedToolCall {
    /// Consume the matched invocation if the persisted lease still matches the
    /// snapshot used to derive this transition.
    ///
    /// # Errors
    ///
    /// Returns an error when the replacement violates the bounded lease
    /// contract. A `false` result reports a concurrent lease change.
    pub fn apply_to(
        &self,
        state: &mut SessionRuntimeState,
        expected: &ActiveOperationLease,
    ) -> Result<bool, SessionStateError> {
        self.transition.apply_to(state, expected)
    }
}

impl AdmittedOperation<'_> {
    /// Admit and register one exact native-host pre-tool invocation.
    ///
    /// # Errors
    ///
    /// Returns [`OperationAdmissionError`] when lease, tool, capability,
    /// authorization, failure posture, or invocation registration is invalid.
    pub fn admit_tool(
        &self,
        lease: &ActiveOperationLease,
        invocation: ToolInvocation<'_>,
    ) -> Result<AdmittedToolCall, OperationAdmissionError> {
        if invocation.provider_binding != self.bundle.catalog().provider_binding {
            return Err(OperationAdmissionError::InvocationOriginMismatch);
        }
        verify_lease(self, lease, invocation.turn_digest.as_ref())?;
        enforce_terminal_failure(&self.contract, lease)?;
        let matched = match_tool(&self.contract, &invocation)?;
        let capability = match matched {
            MatchedTool::Auxiliary => None,
            MatchedTool::Capability(capability) => Some(capability),
        };
        enforce_prior_failure(&self.contract, lease, capability)?;
        let decision = match capability {
            Some(capability) => approval_decision(self, lease, capability, &invocation)?,
            None => ApprovalDecision::Proceed,
        };
        let confirmation_pending = matches!(decision, ApprovalDecision::Confirm(_));
        let transition = register_invocation(
            self,
            lease,
            &invocation,
            capability.map(|capability| &capability.requirement_id),
            confirmation_pending,
        )?;
        let (exact_authorization, confirmation) = match decision {
            ApprovalDecision::Proceed => (None, None),
            ApprovalDecision::Exact(proof) => (Some(proof), None),
            ApprovalDecision::Confirm(request) => (None, Some(request)),
        };
        Ok(AdmittedToolCall {
            transition,
            exact_authorization,
            confirmation,
        })
    }

    /// Consume a matching in-flight invocation and record its failure posture.
    ///
    /// # Errors
    ///
    /// Returns [`OperationAdmissionError`] when the lease or invocation does not
    /// match, or the failure transition violates the operation contract.
    pub fn record_tool_failure(
        &self,
        lease: &ActiveOperationLease,
        invocation: ToolInvocation<'_>,
    ) -> Result<RecordedToolFailure, OperationAdmissionError> {
        if invocation.provider_binding != self.bundle.catalog().provider_binding {
            return Err(OperationAdmissionError::InvocationOriginMismatch);
        }
        verify_lease(self, lease, invocation.turn_digest.as_ref())?;
        let matched = match_tool(&self.contract, &invocation)?;
        let requirement_id = matched.requirement_id();
        let mut replacement = consume_invocation(self, lease, &invocation, requirement_id)?;
        let MatchedTool::Capability(capability) = matched else {
            return Ok(RecordedToolFailure {
                transition: ToolStateTransition::new(replacement),
                requirement: None,
                posture: None,
            });
        };

        let may_replace_failure = replacement.failure().is_none_or(|failure| {
            failure.posture() == FailurePosture::FailOpen
                || (failure.posture() == FailurePosture::Retry
                    && failure.requirement_id() == &capability.requirement_id
                    && capability.failure_posture == FailurePosture::Retry)
        });
        if may_replace_failure {
            replacement.record_failure(
                capability.requirement_id.clone(),
                capability.failure_posture,
            )?;
        }
        let failure = replacement
            .failure()
            .ok_or(OperationAdmissionError::FailureTransitionMissing)?;
        let requirement = failure.requirement_id().clone();
        let posture = failure.posture();
        Ok(RecordedToolFailure {
            transition: ToolStateTransition::new(replacement),
            requirement: Some(requirement),
            posture: Some(posture),
        })
    }

    /// Consume a matching in-flight invocation and clear matching retry debt.
    ///
    /// # Errors
    ///
    /// Returns [`OperationAdmissionError`] when the provider, lease, or
    /// invocation does not match the admitted in-flight tool call.
    pub fn complete_tool_success(
        &self,
        lease: &ActiveOperationLease,
        invocation: ToolInvocation<'_>,
    ) -> Result<CompletedToolCall, OperationAdmissionError> {
        if invocation.provider_binding != self.bundle.catalog().provider_binding {
            return Err(OperationAdmissionError::InvocationOriginMismatch);
        }
        verify_lease(self, lease, invocation.turn_digest.as_ref())?;
        let matched = match_tool(&self.contract, &invocation)?;
        let requirement_id = matched.requirement_id();
        let mut replacement = consume_invocation(self, lease, &invocation, requirement_id)?;
        if let MatchedTool::Capability(capability) = matched {
            replacement.clear_retry_failure(&capability.requirement_id);
        }
        Ok(CompletedToolCall {
            transition: ToolStateTransition::new(replacement),
        })
    }
}
