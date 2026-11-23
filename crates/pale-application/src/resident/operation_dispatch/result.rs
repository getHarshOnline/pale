//! Compare admitted result evidence with the exact selection retained before dispatch.

use super::*;
use crate::actions::{NativeActionRegistry, PreparedNativeAction};
use crate::provider_invocation::AdmittedProviderInvocationOutcome;
use crate::resident::RecoveredOperationObservation;
use pale_ports::ProviderInvocationRequest;

#[cfg(test)]
#[path = "../../../tests/internal/resident/operation_dispatch/result.rs"]
mod test_support;

mod failure;
mod native;
pub use failure::{admit_native_operation_failure, NativeOperationFailureEvidenceInput};
pub use native::{admit_native_operation_result, NativeOperationResultInput};

/// Evidence linking a native outcome to its original dispatch.
/// Neither form grants inspection permission or permits another invocation.
pub enum NativeOperationTerminalEvidence<'a> {
    /// An independently admitted outcome immediately follows the dispatch.
    Direct {
        /// Exact stored dispatch reference, not a caller-selected alias.
        dispatched_ref: &'a ResidentRecordRefV1,
        /// Admitted conclusive observation with that dispatch as its predecessor.
        terminal: &'a OperationObservationV1,
    },
    /// One protected recovery admitted both the selected dispatch and its head.
    /// Intervening expiry or uncertainty remains part of that checked history.
    Recovered(&'a RecoveredOperationObservation),
}

/// Historical inputs for checking a result against its original attempt.
///
/// The caller must recover and contextually admit the dispatched observation
/// through the existing checkpoint/anchor owner. A record read or matching hash
/// alone does not establish that history. This comparison grants no current
/// read permission, new invocation, retry or completion observation.
pub struct OperationAttemptHistoryInput<'a> {
    /// Durable operation locator selecting the original request location.
    pub operation: &'a IssuedLocalOperation,
    /// Original contextually admitted request.
    pub request: &'a CommonRequestEnvelopeV1,
    /// Original admitted session; it need not permit new execution now.
    pub session: &'a SessionAdmissionLeaseV1,
    /// Original child lease named by the handle.
    pub lease: &'a OperationLeaseV1,
    /// Retained definitions for the operation's compiled generation.
    pub generation: &'a AdmittedCompiledGeneration,
    /// Admitted dispatched observation naming the private attempt selection.
    pub dispatched: &'a OperationObservationV1,
    /// Exact private record resolved from that observation's evidence reference.
    pub selection: &'a ControlRecord,
    /// Maximum private record bytes; applies before parsing.
    pub maximum_selection_bytes: usize,
}

/// Historical inputs for one retained `DispatchPrepared` attempt.
///
/// The preparation observation selects the private record. The caller must
/// already have admitted that observation through its checkpoint and anchor.
/// This value grants no dispatch, execution, provider or carrier authority.
pub(in crate::resident) struct OperationPreparationHistoryInput<'a> {
    pub(in crate::resident) operation: &'a IssuedLocalOperation,
    pub(in crate::resident) request: &'a CommonRequestEnvelopeV1,
    pub(in crate::resident) session: &'a SessionAdmissionLeaseV1,
    pub(in crate::resident) lease: &'a OperationLeaseV1,
    pub(in crate::resident) generation: &'a AdmittedCompiledGeneration,
    pub(in crate::resident) prepared: &'a OperationObservationV1,
    pub(in crate::resident) selection: &'a ControlRecord,
    pub(in crate::resident) maximum_selection_bytes: usize,
}

/// Semantic owner encoded by one exact retained attempt selection.
///
/// This private classification does not reconstruct a call, resolve a route,
/// or grant execution authority. It only distinguishes the two existing
/// attempt owners after the selection has been decoded and joined to its
/// immutable parents.
pub(in crate::resident) enum OperationAttemptOwner {
    /// The retained attempt belongs to the built-in native-action owner.
    Native,
    /// The retained attempt belongs to an external provider binding.
    Provider,
}

/// Historical attempt inputs and an independently admitted external-provider result.
pub struct OperationAttemptResultInput<'a> {
    /// Original request, compiled selection and dispatched observation.
    pub history: OperationAttemptHistoryInput<'a>,
    /// Result already admitted by the existing provider and signature owners.
    pub success: &'a AdmittedProviderInvocationSuccess,
}

/// Recovered provider selection joined to its immutable historical parents.
///
/// This value reconstructs the exact provider request used by the original
/// dispatch. It grants no current provider, route, transport, or disclosure
/// permission and cannot start another invocation.
pub(in crate::resident) struct RecoveredProviderAttempt<'a> {
    selection: AttemptSelection,
    parents: AttemptSelectionContext<'a>,
}

/// Reconstructed pure native call joined to its immutable historical parents.
///
/// This phase proves only that the closed registry still produces the exact
/// retained selection. It carries no current execution or storage authority.
pub(super) struct RecoveredNativeAttempt<'a, 'registry> {
    selection: AttemptSelection,
    parents: AttemptSelectionContext<'a>,
    call: PreparedNativeAction<'registry>,
    contract_set: &'a ContractSetV1,
}

impl<'a, 'registry> RecoveredNativeAttempt<'a, 'registry> {
    pub(super) const fn operation(&self) -> &'a IssuedLocalOperation {
        self.parents.operation
    }

    pub(super) const fn selection(&self) -> &AttemptSelection {
        &self.selection
    }

    pub(super) fn result_authority(&self) -> &SigningAuthorityBindingV1 {
        self.selection.result_authority()
    }

    pub(super) const fn timeout_millis(&self) -> u64 {
        self.parents.lease.timeout_millis().get()
    }

    pub(super) fn validate_current_at<Authority: AmbientSessionAuthorityPort>(
        &self,
        current: &AmbientSessionAdmissionContext<'_>,
        authority: &mut Authority,
    ) -> Result<(), OperationAttemptError> {
        let session = self
            .parents
            .operation
            .revalidate_session(current, authority)?;
        super::validate_operation_current_basis(
            self.parents.operation,
            self.parents.request,
            &session,
            self.parents.lease,
            current,
        )?;
        self.selection.validate(&self.parents)?;
        Ok(())
    }

    pub(super) fn into_parts(
        self,
    ) -> (
        AttemptSelection,
        PreparedNativeAction<'registry>,
        &'a ContractSetV1,
    ) {
        (self.selection, self.call, self.contract_set)
    }
}

impl<'a> RecoveredProviderAttempt<'a> {
    pub(in crate::resident) fn attempt_id(&self) -> &CapabilityAttemptIdV1 {
        self.selection.attempt_id()
    }

    pub(super) fn result_authority(&self) -> &SigningAuthorityBindingV1 {
        self.selection.result_authority()
    }

    /// Borrow the original provider request for correlation and request building.
    pub(in crate::resident) fn provider_request(
        &self,
    ) -> Result<ProviderInvocationRequest<'_>, OperationAttemptError> {
        self.selection
            .provider_request(self.parents)
            .map_err(OperationAttemptError::from)
    }

    pub(super) fn validate_outcome(
        &self,
        outcome: &AdmittedProviderInvocationOutcome,
    ) -> Result<(), OperationAttemptError> {
        match outcome {
            AdmittedProviderInvocationOutcome::Succeeded(success) => self
                .selection
                .validate_success(success, &self.parents)
                .map_err(OperationAttemptError::from),
            AdmittedProviderInvocationOutcome::Failed(failure) => self
                .selection
                .validate_failure(failure, &self.parents)
                .map_err(OperationAttemptError::from),
            AdmittedProviderInvocationOutcome::Unavailable(_) => Ok(()),
        }
    }

    /// Borrow the durable operation joined to this historical selection.
    pub(in crate::resident) const fn operation(&self) -> &'a IssuedLocalOperation {
        self.parents.operation
    }

    /// Recheck the exact historical operation against current authority.
    pub(in crate::resident) fn validate_current_at<Authority: AmbientSessionAuthorityPort>(
        &self,
        current: &AmbientSessionAdmissionContext<'_>,
        authority: &mut Authority,
    ) -> Result<(), OperationAttemptError> {
        let session = self
            .parents
            .operation
            .revalidate_session(current, authority)?;
        super::validate_operation_current_basis(
            self.parents.operation,
            self.parents.request,
            &session,
            self.parents.lease,
            current,
        )?;
        self.provider_request()?;
        Ok(())
    }
}

/// Reject results from a different request, provider, implementation or signer.
///
/// The signed dispatch reference selects the private record; result fields do
/// not select their own expected implementation or signing authority. Historical
/// lease expiry does not prevent this evidence check, and success does not renew
/// that lease. The function performs no storage reads, writes or provider calls.
///
/// # Errors
/// Rejects malformed or oversized preparation, a substituted operation or
/// dispatch attempt, or a result that differs from the retained selection.
pub fn validate_operation_attempt_result(
    input: OperationAttemptResultInput<'_>,
) -> Result<(), OperationAttemptError> {
    let selection = input.history.selection()?;
    selection.validate_success(input.success, &input.history.parents())?;
    Ok(())
}

impl<'a> OperationAttemptHistoryInput<'a> {
    /// Classify the owner of this exact historical attempt without executing it.
    pub(in crate::resident) fn classify_owner(
        &self,
    ) -> Result<OperationAttemptOwner, OperationAttemptError> {
        let selection = self.selection()?;
        selection.validate(&self.parents())?;
        Ok(if selection.provider().is_some() {
            OperationAttemptOwner::Provider
        } else {
            OperationAttemptOwner::Native
        })
    }

    /// Recover the exact provider reference retained by this selection.
    ///
    /// `None` identifies a native attempt. The returned reference is copied
    /// from the admitted private selection after it rejoins its immutable
    /// operation, request, session, lease, and generation.
    pub(in crate::resident) fn provider_binding(
        &self,
    ) -> Result<Option<ProviderBindingRefV1>, OperationAttemptError> {
        let selection = self.selection()?;
        selection.validate(&self.parents())?;
        Ok(selection.provider().cloned())
    }

    /// Reconstruct the exact historical provider selection without renewing it.
    pub(in crate::resident) fn recover_provider_attempt(
        self,
    ) -> Result<RecoveredProviderAttempt<'a>, OperationAttemptError> {
        let selection = self.selection()?;
        let parents = self.parents();
        Ok(RecoveredProviderAttempt { selection, parents })
    }

    /// Reconstruct the exact pure native call without executing it.
    pub(super) fn recover_native_attempt<'registry>(
        self,
        registry: &'registry NativeActionRegistry,
        contract_set: &'a ContractSetV1,
    ) -> Result<RecoveredNativeAttempt<'a, 'registry>, OperationAttemptError> {
        let selection = self.selection()?;
        let parents = self.parents();
        let binding = self.operation.handle().1.request_binding();
        let (schema, schema_sha256) = selection.result_schema();
        let admission = CapabilitySuccessAdmissionContextV1 {
            result: CapabilityResultAdmissionContextV1 {
                expected_schema_sha256: schema_sha256,
                expected_request_binding: binding,
                expected_attempt_id: selection.attempt_id(),
                expected_implementation_kind: selection.implementation_kind(),
                expected_implementation_version: selection.implementation_version(),
                signing_authority: selection.result_authority(),
            },
            contract_set_ref: binding.contract_set_ref(),
            contract_set,
        };
        if schema.as_str() != CAPABILITY_RESULT_V1_SCHEMA {
            return Err(OperationAttemptError::Association);
        }
        let (reconstructed, call) =
            AttemptSelection::capture_native(parents, registry, selection.attempt_id(), admission)?;
        if reconstructed != selection {
            return Err(OperationAttemptError::Association);
        }
        Ok(RecoveredNativeAttempt {
            selection,
            parents,
            call,
            contract_set,
        })
    }

    fn validate_terminal<'evidence>(
        &self,
        evidence: NativeOperationTerminalEvidence<'evidence>,
    ) -> Result<&'evidence OperationObservationV1, OperationAttemptError> {
        let (handle_ref, handle) = self.operation.handle();
        match evidence {
            NativeOperationTerminalEvidence::Direct {
                dispatched_ref,
                terminal,
            } => {
                terminal
                    .verify_admitted_successor_of(
                        handle_ref,
                        handle,
                        dispatched_ref,
                        self.dispatched,
                    )
                    .map_err(|_| OperationAttemptError::Association)?;
                Ok(terminal)
            }
            NativeOperationTerminalEvidence::Recovered(recovered) => {
                let (_, dispatched) = recovered.selected();
                let (_, terminal) = recovered.selected_head();
                if !self
                    .operation
                    .checkpoint_selection()
                    .matches_store(recovered.store())
                    || dispatched != self.dispatched
                    || dispatched.operation_handle_ref() != handle_ref
                    || dispatched.operation_handle_sha256()
                        != handle.canonical_operation_handle_sha256()
                    || terminal.revision() <= dispatched.revision()
                    || !terminal.state().is_terminal()
                {
                    return Err(OperationAttemptError::Association);
                }
                // Recovery already admitted every intervening predecessor and
                // checked this head against its checkpoint and independent anchor.
                // Comparing two independently recovered endpoints is not sufficient.
                Ok(terminal)
            }
        }
    }

    fn parents(&self) -> AttemptSelectionContext<'a> {
        AttemptSelectionContext {
            operation: self.operation,
            request_ref: self.operation.request_coordinate().record_ref(),
            request: self.request,
            session: self.session,
            lease: self.lease,
            generation: self.generation,
        }
    }

    fn selection(&self) -> Result<AttemptSelection, OperationAttemptError> {
        let (handle_ref, handle) = self.operation.handle();
        if self.dispatched.operation_handle_ref() != handle_ref
            || self.dispatched.operation_handle_sha256()
                != handle.canonical_operation_handle_sha256()
            || self.dispatched.operation_handle_id() != handle.operation_handle_id()
        {
            return Err(OperationAttemptError::Association);
        }
        let OperationObservationStateV1::Active {
            phase:
                OperationActivePhaseV1::Dispatched {
                    attempt_id,
                    evidence,
                },
        } = self.dispatched.state()
        else {
            return Err(OperationAttemptError::Association);
        };
        let selection =
            AttemptSelection::decode(self.selection, evidence, self.maximum_selection_bytes)?;
        if selection.attempt_id() != attempt_id {
            return Err(OperationAttemptError::Association);
        }
        Ok(selection)
    }
}

impl<'a> OperationPreparationHistoryInput<'a> {
    /// Classify the semantic owner without rebuilding or dispatching a call.
    pub(in crate::resident) fn classify_owner(
        &self,
    ) -> Result<OperationAttemptOwner, OperationAttemptError> {
        let selection = self.selection()?;
        selection.validate(&self.parents())?;
        Ok(if selection.provider().is_some() {
            OperationAttemptOwner::Provider
        } else {
            OperationAttemptOwner::Native
        })
    }

    /// Recover the exact provider reference retained by this preparation.
    pub(in crate::resident) fn provider_binding(
        &self,
    ) -> Result<Option<ProviderBindingRefV1>, OperationAttemptError> {
        let selection = self.selection()?;
        selection.validate(&self.parents())?;
        Ok(selection.provider().cloned())
    }

    /// Reconstruct the exact provider request selected before the restart.
    pub(in crate::resident) fn recover_provider_attempt(
        self,
    ) -> Result<RecoveredProviderAttempt<'a>, OperationAttemptError> {
        let selection = self.selection()?;
        let parents = self.parents();
        selection.validate(&parents)?;
        Ok(RecoveredProviderAttempt { selection, parents })
    }

    /// Reconstruct the exact pure native action without running it.
    pub(super) fn recover_native_attempt<'registry>(
        self,
        registry: &'registry NativeActionRegistry,
        contract_set: &'a ContractSetV1,
    ) -> Result<RecoveredNativeAttempt<'a, 'registry>, OperationAttemptError> {
        let selection = self.selection()?;
        let parents = self.parents();
        let binding = self.operation.handle().1.request_binding();
        let (schema, schema_sha256) = selection.result_schema();
        if schema.as_str() != CAPABILITY_RESULT_V1_SCHEMA {
            return Err(OperationAttemptError::Association);
        }
        let admission = CapabilitySuccessAdmissionContextV1 {
            result: CapabilityResultAdmissionContextV1 {
                expected_schema_sha256: schema_sha256,
                expected_request_binding: binding,
                expected_attempt_id: selection.attempt_id(),
                expected_implementation_kind: selection.implementation_kind(),
                expected_implementation_version: selection.implementation_version(),
                signing_authority: selection.result_authority(),
            },
            contract_set_ref: binding.contract_set_ref(),
            contract_set,
        };
        let (reconstructed, call) =
            AttemptSelection::capture_native(parents, registry, selection.attempt_id(), admission)?;
        if reconstructed != selection {
            return Err(OperationAttemptError::Association);
        }
        Ok(RecoveredNativeAttempt {
            selection,
            parents,
            call,
            contract_set,
        })
    }

    fn parents(&self) -> AttemptSelectionContext<'a> {
        AttemptSelectionContext {
            operation: self.operation,
            request_ref: self.operation.request_coordinate().record_ref(),
            request: self.request,
            session: self.session,
            lease: self.lease,
            generation: self.generation,
        }
    }

    fn selection(&self) -> Result<AttemptSelection, OperationAttemptError> {
        let (handle_ref, handle) = self.operation.handle();
        if self.prepared.operation_handle_ref() != handle_ref
            || self.prepared.operation_handle_sha256() != handle.canonical_operation_handle_sha256()
            || self.prepared.operation_handle_id() != handle.operation_handle_id()
        {
            return Err(OperationAttemptError::Association);
        }
        let OperationObservationStateV1::Active {
            phase:
                OperationActivePhaseV1::DispatchPrepared {
                    attempt_id,
                    evidence,
                },
        } = self.prepared.state()
        else {
            return Err(OperationAttemptError::Association);
        };
        let selection =
            AttemptSelection::decode(self.selection, evidence, self.maximum_selection_bytes)?;
        if selection.attempt_id() != attempt_id {
            return Err(OperationAttemptError::Association);
        }
        Ok(selection)
    }
}
