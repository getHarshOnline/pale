//! Recover and recompute one retained pure native dispatch after restart.

use super::*;
use crate::resident::RecoveredOperationObservation;
use pale_ports::{AntiRollbackAnchorPort, OperationCustodyPort, WallClockSourcePort};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Rebuild and sign `Dispatched` for one protected native preparation.
///
/// The registry reconstructs the exact sealed action before signing. The call
/// is then discarded. No handler, provider, carrier or route is invoked.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn recover_native_operation_preparation<
    'operation,
    Authority,
    Rotation,
    Sign,
>(
    history: OperationPreparationHistoryInput<'operation>,
    recovered: &RecoveredOperationObservation,
    registry: &NativeActionRegistry,
    contract_set: &'operation ContractSetV1,
    current: &AmbientSessionAdmissionContext<'_>,
    signing: SignedResidentAdmissionContextV1<'_>,
    observation_ref: ResidentRecordRefV1,
    authority: &mut Authority,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    maximum_record_bytes: usize,
    admit_rotation: Rotation,
    sign: Sign,
) -> Result<Arc<PreparedRecoveredOperationDispatch>, OperationAttemptError>
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
    let attempt = history.recover_native_attempt(registry, contract_set)?;
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
    if attempt.selection().attempt_id() != attempt_id
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
    Ok(Arc::new(PreparedRecoveredOperationDispatch::new(
        attempt.operation(),
        (prepared_ref, prepared),
        evidence.record_ref.clone(),
        request_expires_at,
        observation_ref,
        observation,
        prior,
    )))
}

/// Current authority, clock and bounded read inputs for pure recomputation.
pub struct NativeOperationRecoveryContext<'a> {
    /// Current authority for the original admitted session and operation.
    pub current: &'a AmbientSessionAdmissionContext<'a>,
    /// Selected record-time source; supplying it does not establish clock trust.
    pub clock: &'a dyn WallClockSourcePort,
    /// Maximum bytes for each recovered custody read.
    pub maximum_record_bytes: usize,
}

/// Exact reconstructed pure call awaiting current recovery checks.
///
/// Construction rejoins protected history and the sealed action registry. It
/// does not run the handler or permit provider, A2A, filesystem or process work.
#[must_use = "recompute this exact pure call after current checks, or discard it"]
pub struct RecoveredNativeOperationDispatch<'operation, 'recovery, 'registry> {
    attempt: RecoveredNativeAttempt<'operation, 'registry>,
    recovered: &'recovery RecoveredOperationObservation,
    selection_ref: ResidentRecordRefV1,
    preparation_observation_ref: ResidentRecordRefV1,
    preparation_predecessor_ref: Option<ResidentRecordRefV1>,
}

/// Rejoin one protected current `Dispatched` head to its exact pure native call.
///
/// The dispatch and its immediate `DispatchPrepared` predecessor must name the
/// same attempt and selection. The closed registry must reproduce that selection
/// byte-for-byte. This function performs no storage read, write or execution;
/// [`RecoveredNativeOperationDispatch::recompute`] owns current release checks.
///
/// # Errors
/// Rejects substituted history, a non-current dispatch, unresolved evidence
/// obligations, a changed action contract or any provider-backed selection.
pub fn recover_native_operation_dispatch<'operation, 'recovery, 'registry>(
    history: OperationAttemptHistoryInput<'operation>,
    recovered: &'recovery RecoveredOperationObservation,
    registry: &'registry NativeActionRegistry,
    contract_set: &'operation ContractSetV1,
) -> Result<RecoveredNativeOperationDispatch<'operation, 'recovery, 'registry>, OperationAttemptError>
{
    if !history
        .operation
        .checkpoint_selection()
        .matches_store(recovered.store())
        || history.session != history.operation.historical_session_lease()
        || history.lease != history.operation.historical_operation_lease()
    {
        return Err(OperationAttemptError::Association);
    }
    let selected = recovered.selected();
    if selected != recovered.selected_head() || selected.1 != history.dispatched {
        return Err(OperationAttemptError::Association);
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
    if !history.lease.evidence_obligations().is_empty() {
        CompletedOperationContextBinding::recover(history.operation, recovered)?;
    }
    let selection_ref = evidence.record_ref.clone();
    let preparation_observation_ref = predecessor.0.clone();
    let preparation_predecessor_ref = predecessor.1.predecessor_ref().clone();
    let attempt = history.recover_native_attempt(registry, contract_set)?;
    if attempt.selection().attempt_id() != attempt_id {
        return Err(OperationAttemptError::Association);
    }
    Ok(RecoveredNativeOperationDispatch {
        attempt,
        recovered,
        selection_ref,
        preparation_observation_ref,
        preparation_predecessor_ref,
    })
}

impl<'operation, 'registry> RecoveredNativeOperationDispatch<'operation, '_, 'registry> {
    /// Recheck current authority and custody, then recompute the same pure call.
    ///
    /// This is a crash-recovery path for the closed `Pure` + `Compute` registry.
    /// It intentionally does not accept a retry flag, replacement input or
    /// carrier. Provider and A2A dispatches cannot enter this type. The resident
    /// must keep custody and this call under its existing single-writer owner.
    /// This is deterministic recomputation, not an exactly-once execution claim:
    /// the same handler can have completed before its in-memory outcome was lost.
    ///
    /// # Errors
    /// Rejects changed authority, time, head, checkpoint, anchor or result signer
    /// before the handler runs. A successful return contains the recomputed
    /// outcome and must be recorded through the existing recovered-history
    /// terminal path.
    pub fn recompute(
        self,
        input: NativeOperationRecoveryContext<'_>,
        authority: &mut impl AmbientSessionAuthorityPort,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<UnstagedNativeOperationOutcome, OperationAttemptError> {
        let started = Instant::now();
        let dispatch = self.recovered.selected();
        let floor = input.current.now.max(*dispatch.1.observed_at());
        let before = observe_time(input.clock, floor, None)?;
        self.attempt
            .validate_current_at(&input.current.at_time(before), authority)?;
        super::super::verify_recovered_operation_head_custody(
            self.attempt.operation(),
            self.recovered,
            custody,
            anchor,
            input.maximum_record_bytes,
        )?;
        let now = observe_time(input.clock, floor, Some(before))?;
        let current = input.current.at_time(now);
        self.attempt.validate_current_at(&current, authority)?;
        signing::validate_runtime_signer(
            self.attempt.selection().result_authority(),
            &current,
            CAPABILITY_RESULT_V1_SCHEMA,
        )?;
        let release_time = observe_time(input.clock, floor, Some(now))?;
        self.attempt
            .validate_current_at(&current.at_time(release_time), authority)?;
        if started.elapsed() >= Duration::from_millis(self.attempt.timeout_millis()) {
            return Err(OperationAttemptError::Deadline);
        }

        let dispatch_ref = dispatch.0.clone();
        let dispatch = dispatch.1.clone();
        Ok(super::execution::execute_recovered_native(
            self.attempt,
            self.selection_ref,
            self.preparation_observation_ref,
            self.preparation_predecessor_ref,
            dispatch_ref,
            dispatch,
            release_time,
        ))
    }
}

fn observe_time(
    clock: &dyn WallClockSourcePort,
    floor: UnixTimeV1,
    previous: Option<UnixTimeV1>,
) -> Result<UnixTimeV1, OperationAttemptError> {
    let observed = clock.observe_unix_time()?;
    if observed < floor || previous.is_some_and(|previous| observed < previous) {
        return Err(OperationAttemptError::TimeOrder);
    }
    Ok(observed)
}
