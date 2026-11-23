//! Protected recovery of provider dispatch for carrier-specific continuation.

use super::*;
use crate::provider_invocation::PreparedProviderInvocation;
use crate::resident::RecoveredOperationObservation;
use pale_ports::{AntiRollbackAnchorPort, OperationCustodyPort};
use std::sync::Arc;

/// Provider-specific meaning retained beside a reconstructed dispatch.
///
/// The shared dispatch value owns checkpoint and custody progression. This
/// wrapper additionally retains the exact historical provider selection and
/// response admission needed for a one-time same-process release after that
/// dispatch reaches completed custody. It is never reconstructed from a
/// historical `Dispatched` observation.
#[must_use = "retain this provider dispatch through custody or discard it"]
pub(in crate::resident) struct PreparedRecoveredProviderDispatch {
    dispatch: Arc<PreparedRecoveredOperationDispatch>,
    request: CommonRequestEnvelopeV1,
    provider_request: OwnedProviderInvocationRequest,
    admission: crate::provider_invocation::OwnedProviderInvocationAdmission,
}

impl PreparedRecoveredProviderDispatch {
    pub(in crate::resident) fn dispatch(&self) -> &Arc<PreparedRecoveredOperationDispatch> {
        &self.dispatch
    }

    pub(in crate::resident) fn provider_request(
        &self,
    ) -> Result<ProviderInvocationRequest<'_>, OperationAttemptError> {
        self.provider_request
            .borrowed()
            .map_err(|_| OperationAttemptError::Association)
    }

    pub(in crate::resident) fn admits_recording_context(
        &self,
        admission: &ProviderInvocationAdmissionContext<'_>,
    ) -> bool {
        self.admission.borrowed().is_same_selection(admission)
    }

    pub(super) fn into_parts(
        self,
    ) -> (
        Arc<PreparedRecoveredOperationDispatch>,
        CommonRequestEnvelopeV1,
        OwnedProviderInvocationRequest,
        crate::provider_invocation::OwnedProviderInvocationAdmission,
    ) {
        (
            self.dispatch,
            self.request,
            self.provider_request,
            self.admission,
        )
    }

    pub(in crate::resident) fn validate_current_at<Authority: AmbientSessionAuthorityPort>(
        &self,
        current: &AmbientSessionAdmissionContext<'_>,
        authority: &mut Authority,
    ) -> Result<(), OperationAttemptError> {
        let operation = self.dispatch.operation();
        let session = operation.revalidate_session(current, authority)?;
        super::super::validate_operation_current_basis(
            operation,
            &self.request,
            &session,
            operation.historical_operation_lease(),
            current,
        )?;
        let request = self.provider_request()?;
        self.admission
            .borrowed()
            .validate_for(request)
            .map_err(|_| OperationAttemptError::Association)
    }

    pub(in crate::resident) fn result_authority(&self) -> &SigningAuthorityBindingV1 {
        self.admission.result_signing_authority()
    }

    pub(in crate::resident) fn validate_prepared_call(&self) -> Result<(), OperationAttemptError> {
        let request = self.provider_request()?;
        PreparedProviderInvocation::prepare(request, EffectClass::Compute)
            .and_then(|prepared| prepared.bind(self.admission.borrowed()))
            .map(|_| ())
            .map_err(|_| OperationAttemptError::Association)
    }
}

#[cfg(test)]
#[path = "../../../../tests/support/provider_dispatch_recovery.rs"]
pub(in crate::resident) mod provider_recovery_test_support;

impl std::ops::Deref for PreparedRecoveredProviderDispatch {
    type Target = PreparedRecoveredOperationDispatch;

    fn deref(&self) -> &Self::Target {
        &self.dispatch
    }
}

/// Rebuild and sign `Dispatched` for one protected provider preparation.
///
/// The retained selection reconstructs the exact provider request. A freshly
/// resolved admission context must accept that request before signing. The
/// request is then discarded. No provider, carrier or route is invoked.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn recover_provider_operation_preparation<
    'operation,
    Authority,
    Rotation,
    Sign,
>(
    history: OperationPreparationHistoryInput<'operation>,
    recovered: &RecoveredOperationObservation,
    admission: ProviderInvocationAdmissionContext<'operation>,
    current: &AmbientSessionAdmissionContext<'_>,
    signing: SignedResidentAdmissionContextV1<'_>,
    observation_ref: ResidentRecordRefV1,
    authority: &mut Authority,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    maximum_record_bytes: usize,
    admit_rotation: Rotation,
    sign: Sign,
) -> Result<PreparedRecoveredProviderDispatch, OperationAttemptError>
where
    Authority: AmbientSessionAuthorityPort,
    Rotation: FnOnce(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
    Sign: FnOnce(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let request_expires_at = history.request.expires_at();
    let selected = recovered.selected();
    if selected != recovered.selected_head()
        || selected.1 != history.prepared
        || !history
            .operation
            .checkpoint_selection()
            .matches_store(recovered.store())
        || history.session != history.operation.historical_session_lease()
        || history.lease != history.operation.historical_operation_lease()
    {
        return Err(OperationAttemptError::Association);
    }
    if !history.lease.evidence_obligations().is_empty() {
        CompletedOperationContextBinding::recover(history.operation, recovered)?;
    }
    let historical_request = history.request.clone();
    let attempt = history.recover_provider_attempt()?;
    let request = attempt.provider_request()?;
    admission
        .validate_for(request)
        .map_err(|_| OperationAttemptError::Association)?;
    attempt.validate_current_at(current, authority)?;
    super::super::verify_recovered_operation_head_custody(
        attempt.operation(),
        recovered,
        custody,
        anchor,
        maximum_record_bytes,
    )?;
    attempt.validate_current_at(current, authority)?;
    signing::validate_runtime_signer(
        signing.signing_authority,
        current,
        OPERATION_OBSERVATION_V1_SCHEMA,
    )?;
    signing::validate_runtime_signer(
        attempt.result_authority(),
        current,
        CAPABILITY_RESULT_V1_SCHEMA,
    )?;
    let (prepared_ref, prepared) = selected;
    let OperationObservationStateV1::Active {
        phase:
            OperationActivePhaseV1::DispatchPrepared {
                attempt_id,
                evidence,
            },
    } = prepared.state()
    else {
        return Err(OperationAttemptError::Association);
    };
    if attempt.attempt_id() != attempt_id
        || evidence.record_ref == observation_ref
        || observation_ref == *prepared_ref
        || observation_ref == *attempt.operation().handle().0
        || observation_ref == *attempt.operation().handle().1.operation_lease_ref()
        || observation_ref == *attempt.operation().request_coordinate().record_ref()
        || prepared.predecessor_ref().as_ref() == Some(&observation_ref)
    {
        return Err(OperationAttemptError::Association);
    }
    let (handle_ref, handle) = attempt.operation().handle();
    let observation = signing::issue_observation(
        &OperationObservationAdmissionContextV1 {
            signed: signing,
            handle_ref,
            handle,
            predecessor: Some((prepared_ref, prepared)),
        },
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Dispatched {
                attempt_id: attempt_id.clone(),
                evidence: evidence.clone(),
            },
        },
        current.now,
        admit_rotation,
        sign,
    )?;
    let prior = recovered.current_checkpoint();
    let provider_request = OwnedProviderInvocationRequest::capture(request);
    Ok(PreparedRecoveredProviderDispatch {
        dispatch: Arc::new(PreparedRecoveredOperationDispatch::new(
            attempt.operation(),
            (prepared_ref, prepared),
            evidence.record_ref.clone(),
            request_expires_at,
            observation_ref,
            observation,
            prior,
        )),
        request: historical_request,
        provider_request,
        admission: admission.to_owned_selection(),
    })
}

/// Protected historical provider dispatch for carrier-specific continuation.
///
/// This value rejoins an anchored `Dispatched` observation with its exact
/// private attempt selection and immutable parents. It can prepare a new,
/// separately retained carrier request. It cannot invoke the provider again.
pub(in crate::resident) struct RecoveredProviderOperationDispatch<'a> {
    attempt: RecoveredProviderAttempt<'a>,
    observation: &'a RecoveredOperationObservation,
    selection_ref: ResidentRecordRefV1,
    preparation_observation_ref: ResidentRecordRefV1,
}

impl<'a> RecoveredProviderOperationDispatch<'a> {
    pub(in crate::resident) fn attempt_id(&self) -> &CapabilityAttemptIdV1 {
        self.attempt.attempt_id()
    }

    pub(in crate::resident) const fn operation(&self) -> &'a IssuedLocalOperation {
        self.attempt.operation()
    }

    pub(in crate::resident) fn provider_request(
        &self,
    ) -> Result<ProviderInvocationRequest<'_>, OperationAttemptError> {
        self.attempt.provider_request()
    }

    pub(in crate::resident) fn validate_outcome(
        &self,
        outcome: &crate::provider_invocation::AdmittedProviderInvocationOutcome,
    ) -> Result<(), OperationAttemptError> {
        self.attempt.validate_outcome(outcome)
    }

    pub(in crate::resident) fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.observation.selected()
    }

    pub(in crate::resident) fn recording_predecessor(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.observation.selected_head()
    }

    pub(in crate::resident) fn checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        self.observation.current_checkpoint()
    }

    pub(in crate::resident) const fn store(&self) -> &pale_ports::AuthorityStoreCoordinate {
        self.observation.store()
    }

    pub(in crate::resident) const fn recovered_observation(
        &self,
    ) -> &'a RecoveredOperationObservation {
        self.observation
    }

    pub(in crate::resident) const fn selection_ref(&self) -> &ResidentRecordRefV1 {
        &self.selection_ref
    }

    pub(in crate::resident) const fn preparation_observation_ref(&self) -> &ResidentRecordRefV1 {
        &self.preparation_observation_ref
    }

    /// Recheck the original operation against current session authority.
    pub(in crate::resident) fn validate_current_at<Authority: AmbientSessionAuthorityPort>(
        &self,
        current: &AmbientSessionAdmissionContext<'_>,
        authority: &mut Authority,
    ) -> Result<(), OperationAttemptError> {
        self.attempt.validate_current_at(current, authority)
    }

    /// Re-read the recovered head, current checkpoint, and independent anchor.
    ///
    /// Historical database custody is valid evidence here. The subsequent A2A
    /// preparation still needs a new database commit and exact anchor before
    /// any carrier can start.
    pub(in crate::resident) fn verify_preparation_custody(
        &self,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        maximum_record_bytes: usize,
    ) -> Result<(), OperationAttemptError> {
        super::super::verify_recovered_operation_head_custody(
            self.operation(),
            self.observation,
            custody,
            anchor,
            maximum_record_bytes,
        )
    }
}

/// Rejoin one protected `Dispatched` history member to its exact provider input.
///
/// The selected observation must also be the recovered operation head. Its
/// immediate predecessor must be the matching `DispatchPrepared` observation.
/// Success grants no retry, provider call, transport start, or disclosure.
pub(in crate::resident) fn recover_provider_operation_dispatch<'a>(
    input: OperationAttemptHistoryInput<'a>,
    recovered: &'a RecoveredOperationObservation,
) -> Result<RecoveredProviderOperationDispatch<'a>, OperationAttemptError> {
    recover_provider_operation_dispatch_for(input, recovered, ProviderRecoveryHead::Dispatched)
}

/// Rejoin one provider dispatch beneath a direct cancellation-request head.
///
/// The returned history proves the original provider selection and exact
/// dispatch. It does not grant provider replay, start a carrier, or prove that
/// cancellation completed.
pub(in crate::resident) fn recover_provider_dispatch_under_cancellation_request<'a>(
    input: OperationAttemptHistoryInput<'a>,
    recovered: &'a RecoveredOperationObservation,
) -> Result<RecoveredProviderOperationDispatch<'a>, OperationAttemptError> {
    recover_provider_operation_dispatch_for(
        input,
        recovered,
        ProviderRecoveryHead::DirectCancellationRequest,
    )
}

enum ProviderRecoveryHead {
    Dispatched,
    DirectCancellationRequest,
}

fn recover_provider_operation_dispatch_for<'a>(
    input: OperationAttemptHistoryInput<'a>,
    recovered: &'a RecoveredOperationObservation,
    recovery_head: ProviderRecoveryHead,
) -> Result<RecoveredProviderOperationDispatch<'a>, OperationAttemptError> {
    if !input
        .operation
        .checkpoint_selection()
        .matches_store(recovered.store())
        || input.session != input.operation.historical_session_lease()
        || input.lease != input.operation.historical_operation_lease()
    {
        return Err(OperationAttemptError::Association);
    }
    let selected = recovered.selected();
    if selected.1 != input.dispatched {
        return Err(OperationAttemptError::Association);
    }
    let selected_head = recovered.selected_head();
    if selected != selected_head {
        let direct_cancellation = matches!(
            recovery_head,
            ProviderRecoveryHead::DirectCancellationRequest
        ) && matches!(
            selected_head.1.state(),
            OperationObservationStateV1::CancellationRequested { .. }
        ) && selected_head
            .1
            .verify_admitted_successor_of(
                input.operation.handle().0,
                input.operation.handle().1,
                selected.0,
                selected.1,
            )
            .is_ok();
        if !direct_cancellation {
            return Err(OperationAttemptError::Association);
        }
    }
    let Some(predecessor) = recovered.selected_predecessor() else {
        return Err(OperationAttemptError::Association);
    };
    let (
        OperationObservationStateV1::Active {
            phase:
                OperationActivePhaseV1::Dispatched {
                    attempt_id,
                    evidence,
                },
        },
        OperationObservationStateV1::Active {
            phase:
                OperationActivePhaseV1::DispatchPrepared {
                    attempt_id: prepared_attempt_id,
                    evidence: prepared_evidence,
                },
        },
    ) = (selected.1.state(), predecessor.1.state())
    else {
        return Err(OperationAttemptError::Association);
    };
    if attempt_id != prepared_attempt_id || evidence != prepared_evidence {
        return Err(OperationAttemptError::Association);
    }
    if !input.lease.evidence_obligations().is_empty() {
        CompletedOperationContextBinding::recover(input.operation, recovered)?;
    }
    let attempt = input.recover_provider_attempt()?;
    let request = attempt.provider_request()?;
    if request.attempt_id() != attempt_id {
        return Err(OperationAttemptError::Association);
    }
    Ok(RecoveredProviderOperationDispatch {
        attempt,
        observation: recovered,
        selection_ref: evidence.record_ref.clone(),
        preparation_observation_ref: predecessor.0.clone(),
    })
}
