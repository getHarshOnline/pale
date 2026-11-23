//! Cold-restart laws for a retained A2A `CancelTask` request.

use super::a2a_cancel_task::{
    cancel_input, cancellation_request, checkpoint_signer, CountingCancelPort, SelectedStore,
};
use super::a2a_delegate::{
    with_retained_process_a2a_task, with_retained_process_a2a_task_and_one_completed_append,
    with_retained_process_a2a_task_and_two_completed_appends, RetainedProcessA2aTaskFixture,
};
use super::*;
use crate::local_operation_start_fixture::{record_ref, schema_digest, signature, RecoveryParents};
use crate::resident::a2a_task_custody::A2aCancelTaskDispositionCommitInput;
use crate::resident::a2a_task_custody::A2aGetTaskPreparationInput;
use crate::resident::operation_dispatch::selection::tests::a2a_readback_service::{
    exchange_limits, recover_retained_a2a_task_history_after_local_commit, Clock, GetTaskPort,
};
use crate::resident::process_host::a2a_cancel_task::ResidentProcessRecoveredA2aCancelTaskFailure;
use crate::resident::process_host::a2a_delegate_recovery_owner::{
    ResidentA2aDelegateRecoveryOwner, ResidentA2aDelegateRecoveryPort,
};
use crate::resident::process_host::a2a_get_task::ResidentProcessRecoveredA2aGetTaskDisposition;
use crate::resident::process_host::delegate_recovery_sweep::{
    ResidentProcessDelegateRecoveryError, ResidentProcessDelegateRecoveryTurn,
};
use crate::resident::process_host::operation_recovery_sweep::{
    ResidentProcessRootProviderRecoveryOwner, ResidentProcessRootProviderResolution,
};
use crate::resident::process_host::restart::{
    ResidentProcessA2aCancelTaskRestartInput, ResidentProcessA2aTaskRestartInput,
    ResidentProcessAttemptObservation, ResidentProcessOperationRestartDisposition,
    ResidentProcessOperationRestartError, ResidentProcessOperationRestartInput,
    ResidentProcessRecoveredA2aCancelTaskRequest, ResidentProcessRecoveredA2aTask,
    ResidentProcessRecoveredAttempt, ResidentProcessRecoveredLocalCancellation,
    ResidentProcessRecoveredOperationResolution, ResidentProcessRecoveredPreparedAttempt,
    ResidentProcessRecoveredProviderResolution, ResidentProcessRestartReadLimits,
};
use crate::resident::{
    A2aCancelTaskDispositionInput, A2aGetTaskReadbackInput, A2aSendRecoveryLimits,
    A2aTaskRecoveryAdmission, A2aTaskRecoveryLimits, IssuedLocalOperation,
    LocalOperationRecoveryContext, OperationAttemptHistoryInput, OperationCheckpointAncestryLimits,
    OperationCheckpointInput, OperationControlRecoveryAdmission,
    OperationObservationRecoveryLimits, RecoveredA2aTaskHistory,
    ResidentServiceA2aCancelRecoveryInputs, ResidentServiceA2aDelegateLaunchInputs,
    ResidentServiceA2aPreTaskRecoveryInputs, ResidentServiceA2aTaskObservationInputs,
    ResidentServiceDelegateInputPort, ResidentServiceDeploymentSelector,
};
use crate::resident_fixture::RecoveryResolver;
use crate::session_admission_fixture::ProvingAuthority;
use pale_core::records::{
    A2aJsonRpcRequestIdV1, A2aRemoteTaskIdV1, A2aRouteAdmissionContextV1, A2aRouteAdmissionV1,
    A2aRouteAdmissionWireV1, A2aTaskBindingDigest, A2aTaskBindingPreimageV1, A2aTaskBindingV1,
    A2aTaskBindingWireV1, A2aTaskObservationDigest, A2aTaskObservationPreimageV1,
    A2aTaskObservationSourceV1, A2aTaskObservationWireV1, ArtifactDigest, CanonicalSignatureV1,
    CommonRequestAdmissionContextV1, CommonRequestEnvelopePreimageV1, CommonRequestEnvelopeV1,
    CommonRequestEnvelopeWireV1, ControlCheckpointIdV1, ControlStateCheckpointDigest,
    ControlStateCheckpointPreimageV1, ControlStateCheckpointV1, ControlStateCheckpointWireV1,
    ImmutableRecordReferenceV1, InvalidResidentRecordV1, OperationHandleDigest,
    OperationHandlePreimageV1, OperationObservationDigest, OperationObservationPreimageV1,
    OperationObservationStateV1, OperationObservationWireV1, ReconciliationFrontierDigest,
    ReconciliationFrontierReferenceV1, ResidentRecordRefV1, SessionAdmissionLeaseWireV1,
    SignedResidentAdmissionContextV1, UnixTimeV1, OPERATION_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::{
    A2aCancelTaskRequestEvidencePort, A2aExchangeError, AccumulatorMembershipLimits,
    AccumulatorPreparationLimits, AntiRollbackAnchorPort, AuthorityStoreCoordinate,
    ControlStoreError, OperationA2aEvidence, OperationCustodyIndexPredecessor,
    OperationCustodyPort, SessionLeaseBody, SessionLeaseBodyRead, SessionLeaseBodyRetention,
    SessionLeaseCustodyPort,
};
use std::{cell::Cell, num::NonZeroUsize, rc::Rc};

#[derive(Debug, Clone, Copy, Default)]
enum RestartCustodyFault {
    #[default]
    None,
    A2aSendPreparationAbsent,
    A2aSendPreparationReadUnavailable,
    A2aSendPreparationMalformed,
    A2aSendCheckpointAbsent,
    A2aRouteRecordMissing,
    A2aEvidenceRowMissing,
    A2aEvidenceReadUnavailable,
    A2aEvidenceCoordinateUnlike,
    A2aEvidenceMembershipUnavailable,
    BindingDiscoveryUnavailable,
    BindingAbsent,
    DuplicateBindingCandidate,
    IncompleteBindingPage,
    RepeatedStartEvidenceMissing,
    FinalOperationHeadMissing,
    FinalCheckpointReadUnavailable,
    AppendCommitUnknownOnce,
}

struct RestartCustody<'a> {
    inner: &'a mut dyn OperationCustodyPort,
    fault: RestartCustodyFault,
    protocol_request_qualified: Rc<Cell<bool>>,
    a2a_evidence_read: Rc<Cell<bool>>,
    append_fault_issued: bool,
    append_read_back_blocked: bool,
    a2a_send_preparation_reads: usize,
    accumulator_preparations: usize,
    commits: usize,
}

impl<'a> RestartCustody<'a> {
    fn new(
        inner: &'a mut dyn OperationCustodyPort,
        fault: RestartCustodyFault,
        protocol_request_qualified: Rc<Cell<bool>>,
        a2a_evidence_read: Rc<Cell<bool>>,
    ) -> Self {
        Self {
            inner,
            fault,
            protocol_request_qualified,
            a2a_evidence_read,
            append_fault_issued: false,
            append_read_back_blocked: false,
            a2a_send_preparation_reads: 0,
            accumulator_preparations: 0,
            commits: 0,
        }
    }

    fn writes(&self) -> (usize, usize) {
        (self.accumulator_preparations, self.commits)
    }

    fn a2a_send_preparation_reads(&self) -> usize {
        self.a2a_send_preparation_reads
    }
}

impl pale_ports::ControlTransactionPort for RestartCustody<'_> {
    fn prepare_transaction(
        &mut self,
        store: &AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, ControlStoreError> {
        self.inner.prepare_transaction(store)
    }
}

impl OperationCustodyPort for RestartCustody<'_> {
    fn read_operation_session_basis(
        &mut self,
        store: &AuthorityStoreCoordinate,
        handle: &pale_ports::ControlRecordCoordinate,
        maximum_bytes: usize,
    ) -> Result<Option<pale_ports::OperationSessionBasis>, ControlStoreError> {
        self.inner
            .read_operation_session_basis(store, handle, maximum_bytes)
    }

    fn read_a2a_evidence(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &pale_ports::ControlRecordCoordinate,
        maximum_bytes: usize,
    ) -> Result<Option<OperationA2aEvidence>, ControlStoreError> {
        let evidence = match self.fault {
            RestartCustodyFault::A2aEvidenceRowMissing => Ok(None),
            RestartCustodyFault::A2aEvidenceReadUnavailable => Err(ControlStoreError::new(
                pale_ports::ControlStoreErrorKind::Storage,
            )),
            _ => self
                .inner
                .read_a2a_evidence(store, coordinate, maximum_bytes),
        }?;
        self.a2a_evidence_read.set(evidence.is_some());
        if matches!(self.fault, RestartCustodyFault::A2aEvidenceCoordinateUnlike) {
            return evidence
                .map(|evidence| {
                    OperationA2aEvidence::new(
                        ImmutableRecordReferenceV1 {
                            record_ref: record_ref("cancel-task/unlike-evidence"),
                            record_sha256: ArtifactDigest::hash_bytes(evidence.bytes()),
                        },
                        evidence.bytes().to_vec(),
                        None,
                        maximum_bytes,
                    )
                    .map_err(|_| {
                        ControlStoreError::new(pale_ports::ControlStoreErrorKind::RecoveryRequired)
                    })
                })
                .transpose();
        }
        Ok(evidence)
    }

    fn read_a2a_evidence_inserted_at(
        &mut self,
        store: &AuthorityStoreCoordinate,
        transaction: pale_core::records::ControlTransactionSequenceV1,
        maximum_items: usize,
        maximum_bytes: usize,
    ) -> Result<Vec<OperationA2aEvidence>, ControlStoreError> {
        self.inner
            .read_a2a_evidence_inserted_at(store, transaction, maximum_items, maximum_bytes)
    }

    fn read_a2a_task_bindings(
        &mut self,
        store: &AuthorityStoreCoordinate,
        scope: &pale_ports::OperationA2aBindingScope,
        cursor: Option<&pale_ports::OperationA2aBindingCursor>,
        maximum_bindings: usize,
        maximum_body_bytes: usize,
    ) -> Result<pale_ports::OperationA2aBindings, ControlStoreError> {
        if matches!(self.fault, RestartCustodyFault::BindingDiscoveryUnavailable) {
            return Err(ControlStoreError::new(
                pale_ports::ControlStoreErrorKind::Storage,
            ));
        }
        let mut bindings = self.inner.read_a2a_task_bindings(
            store,
            scope,
            cursor,
            maximum_bindings,
            maximum_body_bytes,
        )?;
        if matches!(self.fault, RestartCustodyFault::BindingAbsent) {
            bindings.bindings.clear();
            bindings.continuation = None;
            return Ok(bindings);
        }
        if matches!(self.fault, RestartCustodyFault::DuplicateBindingCandidate) {
            let duplicate = bindings.bindings.first().cloned().ok_or_else(|| {
                ControlStoreError::new(pale_ports::ControlStoreErrorKind::RecoveryRequired)
            })?;
            bindings.bindings.push(duplicate);
        }
        if matches!(self.fault, RestartCustodyFault::IncompleteBindingPage) {
            let last = bindings.bindings.last().ok_or_else(|| {
                ControlStoreError::new(pale_ports::ControlStoreErrorKind::RecoveryRequired)
            })?;
            bindings.continuation = Some(pale_ports::OperationA2aBindingCursor::new(
                store.clone(),
                scope.clone(),
                last.transaction(),
                last.transaction(),
                last.binding().coordinate().record_ref().clone(),
            )?);
        }
        Ok(bindings)
    }

    fn read_publication_sources(
        &mut self,
        store: &AuthorityStoreCoordinate,
        cursor: Option<&pale_ports::OperationPublicationCursor>,
        maximum_sources: usize,
        maximum_body_bytes: usize,
    ) -> Result<pale_ports::OperationPublicationSources, pale_ports::OperationPublicationReadError>
    {
        self.inner
            .read_publication_sources(store, cursor, maximum_sources, maximum_body_bytes)
    }

    fn read_checkpoint_head(
        &mut self,
        store: &AuthorityStoreCoordinate,
        domain: &pale_core::records::ControlCheckpointDomainIdV1,
        maximum_record_bytes: usize,
    ) -> Result<
        Option<(pale_ports::ControlCheckpointHead, pale_ports::ControlRecord)>,
        ControlStoreError,
    > {
        if matches!(
            self.fault,
            RestartCustodyFault::FinalCheckpointReadUnavailable
        ) && self.protocol_request_qualified.get()
        {
            return Err(ControlStoreError::new(
                pale_ports::ControlStoreErrorKind::Storage,
            ));
        }
        self.inner
            .read_checkpoint_head(store, domain, maximum_record_bytes)
    }

    fn verify_accumulator_membership(
        &mut self,
        request: &pale_ports::OperationAccumulatorMembership,
    ) -> Result<
        pale_ports::VerifiedOperationAccumulatorMembership,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        if matches!(
            self.fault,
            RestartCustodyFault::A2aEvidenceMembershipUnavailable
        ) && self.a2a_evidence_read.get()
        {
            return Err(pale_ports::OperationAccumulatorPreparationError::Limit);
        }
        self.inner.verify_accumulator_membership(request)
    }

    fn prepare_accumulator(
        &mut self,
        request: &pale_ports::OperationAccumulatorPreparation,
    ) -> Result<
        pale_ports::PreparedOperationAccumulator,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        self.accumulator_preparations += 1;
        self.inner.prepare_accumulator(request)
    }

    fn compare_and_retain_operation_lease(
        &mut self,
        store: &AuthorityStoreCoordinate,
        record: &pale_ports::ControlRecord,
    ) -> Result<pale_ports::OperationCustodyCommitOutcome, ControlStoreError> {
        self.inner.compare_and_retain_operation_lease(store, record)
    }

    fn read_operation_record(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &pale_ports::ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<Option<pale_ports::ControlRecord>, ControlStoreError> {
        let record = self
            .inner
            .read_operation_record(store, coordinate, maximum_body_bytes)?;
        if matches!(self.fault, RestartCustodyFault::A2aRouteRecordMissing)
            && record.as_ref().is_some_and(|record| {
                record.schema_id().as_str() == pale_core::records::A2A_ROUTE_ADMISSION_V1_SCHEMA
            })
        {
            return Ok(None);
        }
        Ok(record)
    }

    fn compare_and_commit(
        &mut self,
        candidate: &pale_ports::OperationCustodyCandidate,
    ) -> Result<pale_ports::OperationCustodyCommitOutcome, ControlStoreError> {
        self.commits += 1;
        if matches!(self.fault, RestartCustodyFault::AppendCommitUnknownOnce)
            && self.protocol_request_qualified.get()
            && !self.append_fault_issued
        {
            self.append_fault_issued = true;
            let _ = self.inner.compare_and_commit(candidate)?;
            return Err(ControlStoreError::new(
                pale_ports::ControlStoreErrorKind::OutcomeUnknown,
            ));
        }
        self.inner.compare_and_commit(candidate)
    }

    fn observe_exact(
        &mut self,
        candidate: &pale_ports::OperationCustodyCandidate,
    ) -> Result<pale_ports::OperationCustodyReadBackOutcome, ControlStoreError> {
        if matches!(self.fault, RestartCustodyFault::AppendCommitUnknownOnce)
            && self.append_fault_issued
        {
            if !self.append_read_back_blocked {
                self.append_read_back_blocked = true;
                return Err(ControlStoreError::new(
                    pale_ports::ControlStoreErrorKind::Storage,
                ));
            }
            return Ok(pale_ports::OperationCustodyReadBackOutcome::ExactSuccessor);
        }
        self.inner.observe_exact(candidate)
    }

    fn read_index(
        &mut self,
        store: &AuthorityStoreCoordinate,
        query: &pale_ports::OperationCustodyIndexQuery,
        maximum_record_bytes: usize,
    ) -> Result<pale_ports::OperationCustodyIndexObservation, ControlStoreError> {
        if matches!(
            query,
            pale_ports::OperationCustodyIndexQuery::A2aSendPreparation(_)
        ) {
            self.a2a_send_preparation_reads += 1;
        }
        if matches!(
            self.fault,
            RestartCustodyFault::A2aSendPreparationReadUnavailable
        ) && matches!(
            query,
            pale_ports::OperationCustodyIndexQuery::A2aSendPreparation(_)
        ) {
            return Err(ControlStoreError::new(
                pale_ports::ControlStoreErrorKind::Storage,
            ));
        }
        if matches!(self.fault, RestartCustodyFault::A2aSendPreparationAbsent)
            && matches!(
                query,
                pale_ports::OperationCustodyIndexQuery::A2aSendPreparation(_)
            )
        {
            return Ok(pale_ports::OperationCustodyIndexObservation::absent(
                query.clone(),
            ));
        }
        if matches!(self.fault, RestartCustodyFault::A2aSendCheckpointAbsent)
            && matches!(
                query,
                pale_ports::OperationCustodyIndexQuery::A2aSendPreparationCheckpoint(_)
            )
        {
            return Ok(pale_ports::OperationCustodyIndexObservation::absent(
                query.clone(),
            ));
        }
        if matches!(
            self.fault,
            RestartCustodyFault::RepeatedStartEvidenceMissing
        ) && self.protocol_request_qualified.get()
            && matches!(
                query,
                pale_ports::OperationCustodyIndexQuery::RequestOperation(_)
                    | pale_ports::OperationCustodyIndexQuery::RequestEvidence(_)
            )
        {
            return Ok(pale_ports::OperationCustodyIndexObservation::absent(
                query.clone(),
            ));
        }
        if matches!(self.fault, RestartCustodyFault::FinalOperationHeadMissing)
            && self.protocol_request_qualified.get()
            && matches!(
                query,
                pale_ports::OperationCustodyIndexQuery::LatestObservation(_)
            )
        {
            return Ok(pale_ports::OperationCustodyIndexObservation::absent(
                query.clone(),
            ));
        }
        let observed = self.inner.read_index(store, query, maximum_record_bytes)?;
        if matches!(self.fault, RestartCustodyFault::A2aSendPreparationMalformed)
            && matches!(
                query,
                pale_ports::OperationCustodyIndexQuery::A2aSendPreparation(_)
            )
        {
            let record = observed.into_record().ok_or_else(|| {
                ControlStoreError::new(pale_ports::ControlStoreErrorKind::RecoveryRequired)
            })?;
            let malformed = pale_ports::ControlRecord::new(
                record.coordinate().clone(),
                record.schema_id().clone(),
                record.schema_sha256().clone(),
                pale_ports::ControlRecordBytes::new(b"{}".to_vec()).expect("bounded malformed row"),
            );
            return pale_ports::OperationCustodyIndexObservation::exact(query.clone(), malformed)
                .map_err(|_| {
                    ControlStoreError::new(pale_ports::ControlStoreErrorKind::RecoveryRequired)
                });
        }
        Ok(observed)
    }
}

#[derive(Clone, Copy, Default)]
enum RestartAnchorFault {
    #[default]
    None,
    FinalReadUnavailable,
    AppendUnavailableOnce,
}

struct RestartAnchor<'a> {
    inner: &'a mut dyn AntiRollbackAnchorPort,
    fault: RestartAnchorFault,
    protocol_request_qualified: Rc<Cell<bool>>,
    append_fault_issued: bool,
}

impl<'a> RestartAnchor<'a> {
    fn new(
        inner: &'a mut dyn AntiRollbackAnchorPort,
        fault: RestartAnchorFault,
        protocol_request_qualified: Rc<Cell<bool>>,
    ) -> Self {
        Self {
            inner,
            fault,
            protocol_request_qualified,
            append_fault_issued: false,
        }
    }
}

impl AntiRollbackAnchorPort for RestartAnchor<'_> {
    fn read(
        &mut self,
        key: &pale_ports::AntiRollbackAnchorKey,
    ) -> Result<Option<pale_ports::AntiRollbackAnchor>, pale_ports::AntiRollbackAnchorError> {
        if matches!(self.fault, RestartAnchorFault::FinalReadUnavailable)
            && self.protocol_request_qualified.get()
        {
            return Err(pale_ports::AntiRollbackAnchorError::new(
                pale_ports::AntiRollbackAnchorErrorKind::Unavailable,
            ));
        }
        self.inner.read(key)
    }

    fn compare_and_advance(
        &mut self,
        transition: &pale_ports::PreparedAntiRollbackAnchorTransition,
    ) -> Result<pale_ports::AntiRollbackAnchorAdvanceDisposition, pale_ports::AntiRollbackAnchorError>
    {
        if matches!(self.fault, RestartAnchorFault::AppendUnavailableOnce)
            && self.protocol_request_qualified.get()
            && !self.append_fault_issued
        {
            self.append_fault_issued = true;
            return Err(pale_ports::AntiRollbackAnchorError::new(
                pale_ports::AntiRollbackAnchorErrorKind::Unavailable,
            ));
        }
        self.inner.compare_and_advance(transition)
    }

    fn observe(
        &mut self,
        transition: &pale_ports::PreparedAntiRollbackAnchorTransition,
    ) -> Result<pale_ports::AntiRollbackAnchorObservation, pale_ports::AntiRollbackAnchorError>
    {
        self.inner.observe(transition)
    }
}

struct LatchingCancelTaskDecoder {
    protocol_request_qualified: Rc<Cell<bool>>,
}

impl A2aCancelTaskRequestEvidencePort for LatchingCancelTaskDecoder {
    fn read_cancel_task_request(
        &self,
        request: &[u8],
        expected_task: &A2aRemoteTaskIdV1,
        maximum_bytes: NonZeroUsize,
    ) -> Result<A2aJsonRpcRequestIdV1, A2aExchangeError> {
        let request_id = pale_adapter_a2a::A2aCancelTaskDecoder.read_cancel_task_request(
            request,
            expected_task,
            maximum_bytes,
        )?;
        self.protocol_request_qualified.set(true);
        Ok(request_id)
    }
}

struct RestartSessions(SessionLeaseBody);

impl SessionLeaseCustodyPort for RestartSessions {
    fn compare_and_retain(
        &self,
        _: &SessionLeaseBody,
    ) -> Result<SessionLeaseBodyRetention, ControlStoreError> {
        panic!("restart recovery cannot retain or renew a session")
    }

    fn read(
        &self,
        query: &SessionLeaseBodyRead,
    ) -> Result<Option<SessionLeaseBody>, ControlStoreError> {
        assert_eq!(query.store(), self.0.store());
        Ok(Some(self.0.clone()))
    }
}

fn restart_parents(
    parents: RecoveryParents<'_>,
    store: &AuthorityStoreCoordinate,
) -> (RestartSessions, RecoveryResolver) {
    let session = parents.session.lease();
    let body = SessionLeaseBody::new(
        store.clone(),
        session.session_admission_lease_digest().clone(),
        session.schema_id().clone(),
        session.schema_sha256().clone(),
        pale_ports::ControlRecordBytes::new(
            pale_core::canon::to_canonical_string(&SessionAdmissionLeaseWireV1::from(session))
                .expect("session lease wire")
                .into_bytes(),
        )
        .expect("bounded session lease"),
    );
    let mut resolver = RecoveryResolver::default();
    resolver.insert_generation(parents.current.generation.compiled_generation);
    resolver.insert_active(parents.current.generation.active_head);
    resolver.insert_fence(parents.current.generation.activation_fence);
    resolver.insert_effective_profile(parents.current.generation.effective_profile);
    (RestartSessions(body), resolver)
}

struct RetainedCancelTaskRestartFixture<'a, 'store> {
    host: &'a mut ResidentProcessHost<'store, UnprovedHeldWriter>,
    selector: ResidentDeploymentAuthoritySelector,
    store: AuthorityStoreCoordinate,
    sessions: &'a super::a2a_delegate::UnusedA2aSessions,
    previous_history: RecoveredA2aTaskHistory,
    history: RecoveredA2aTaskHistory,
    attempt: OperationAttemptHistoryInput<'a>,
    signing: &'a SigningAuthorityBindingV1,
    checkpoint: ControlStateCheckpointV1,
    authority: &'a mut ProvingAuthority,
    custody: &'a mut dyn OperationCustodyPort,
    anchor: &'a mut dyn AntiRollbackAnchorPort,
    parents: RecoveryParents<'a>,
    parent_request: (pale_ports::ControlRecordCoordinate, CommonRequestEnvelopeV1),
    send_preparations: usize,
    send_starts: usize,
    cancel_starts: usize,
}

struct RestartClassificationRun {
    disposition:
        Result<ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartError>,
    expected_binding: (ResidentRecordRefV1, A2aTaskBindingV1),
    send_preparations: usize,
    send_starts: usize,
    cancel_starts: usize,
    protocol_request_qualified: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct RestartHostOccupancy {
    provider_reserved: usize,
    provider_checked_out: usize,
    provider_active: usize,
    a2a_delegate_reserved: usize,
    a2a_delegate_checked_out: usize,
    a2a_delegate_active: usize,
    a2a_task_active: usize,
    a2a_task_checked_out: usize,
    task_reservations: usize,
}

fn restart_host_occupancy(
    host: &ResidentProcessHost<'_, UnprovedHeldWriter>,
) -> RestartHostOccupancy {
    RestartHostOccupancy {
        provider_reserved: host.provider_routes.reserved,
        provider_checked_out: host.provider_routes.checked_out,
        provider_active: host.provider_routes.active.len(),
        a2a_delegate_reserved: host.a2a_delegate_routes.reserved_count(),
        a2a_delegate_checked_out: host.a2a_delegate_routes.checked_out_count(),
        a2a_delegate_active: host.a2a_delegate_routes.active_count(),
        a2a_task_active: host.a2a_task_routes.active_count(),
        a2a_task_checked_out: host.a2a_task_routes.checked_out_count(),
        task_reservations: host.operations.outstanding_task_reservations(),
    }
}

impl RetainedCancelTaskRestartFixture<'_, '_> {
    fn classify(
        self,
        bounds: RestartClassifierBounds,
        custody_fault: RestartCustodyFault,
        anchor_fault: RestartAnchorFault,
    ) -> RestartClassificationRun {
        self.classify_with_a2a_input(Some(bounds), custody_fault, anchor_fault)
    }

    fn classify_without_a2a_input(self) -> RestartClassificationRun {
        self.classify_with_a2a_input(None, RestartCustodyFault::None, RestartAnchorFault::None)
    }

    fn classify_with_a2a_input(
        self,
        bounds: Option<RestartClassifierBounds>,
        custody_fault: RestartCustodyFault,
        anchor_fault: RestartAnchorFault,
    ) -> RestartClassificationRun {
        let occupancy_before = restart_host_occupancy(self.host);
        let expected_binding = (
            self.history.binding().0.clone(),
            self.history.binding().1.clone(),
        );
        let protocol_request_qualified = Rc::new(Cell::new(false));
        let a2a_evidence_read = Rc::new(Cell::new(false));
        let protocol = LatchingCancelTaskDecoder {
            protocol_request_qualified: Rc::clone(&protocol_request_qualified),
        };
        let mut custody = RestartCustody::new(
            self.custody,
            custody_fault,
            Rc::clone(&protocol_request_qualified),
            a2a_evidence_read,
        );
        let mut anchor = RestartAnchor::new(
            self.anchor,
            anchor_fault,
            Rc::clone(&protocol_request_qualified),
        );
        let disposition = classify_retained_pairwise_delegate(RestartClassificationInput {
            host: self.host,
            selector: self.selector,
            store: &self.store,
            history: &self.history,
            attempt: self.attempt,
            signing: self.signing,
            checkpoint: &self.checkpoint,
            custody: &mut custody,
            anchor: &mut anchor,
            parents: self.parents,
            parent_request: &self.parent_request,
            protocol: &protocol,
            mode: RestartClassifierMode::CancelTask(bounds),
        });
        let protocol_request_qualified = protocol_request_qualified.get();
        assert_eq!(restart_host_occupancy(self.host), occupancy_before);
        RestartClassificationRun {
            disposition,
            expected_binding,
            send_preparations: self.send_preparations,
            send_starts: self.send_starts,
            cancel_starts: self.cancel_starts,
            protocol_request_qualified,
        }
    }
}

fn prove_cancel_task_delegate_routing(fixture: RetainedCancelTaskRestartFixture<'_, '_>) {
    let RetainedCancelTaskRestartFixture {
        host,
        selector,
        store,
        sessions,
        previous_history: _,
        history,
        attempt,
        signing,
        checkpoint,
        authority: _,
        custody,
        anchor,
        parents,
        parent_request,
        send_preparations,
        send_starts,
        cancel_starts,
    } = fixture;
    let protocol_request_qualified = Rc::new(Cell::new(false));
    let protocol = LatchingCancelTaskDecoder {
        protocol_request_qualified: Rc::clone(&protocol_request_qualified),
    };
    let mut custody = RestartCustody::new(
        custody,
        RestartCustodyFault::None,
        Rc::clone(&protocol_request_qualified),
        Rc::new(Cell::new(false)),
    );
    let mut anchor = RestartAnchor::new(
        anchor,
        RestartAnchorFault::None,
        Rc::clone(&protocol_request_qualified),
    );
    let disposition = classify_retained_pairwise_delegate(RestartClassificationInput {
        host,
        selector,
        store: &store,
        history: &history,
        attempt,
        signing,
        checkpoint: &checkpoint,
        custody: &mut custody,
        anchor: &mut anchor,
        parents,
        parent_request: &parent_request,
        protocol: &protocol,
        mode: RestartClassifierMode::CancelTask(Some(RestartClassifierBounds::standard())),
    })
    .expect("one exact retained CancelTask disposition");
    let mut a2a = A2aRouteOwner::default();
    let mut owner =
        crate::resident::process_host::service_assembly::resident_service_delegate_recovery_owner::<
            _,
            UnprovedHeldWriter,
        >(&mut a2a);
    let routed = host
        .route_delegate_restart_disposition(
            disposition,
            ResidentDeploymentOperationAccess {
                store: &store,
                custody: &mut custody,
                sessions,
                anchor: &mut anchor,
            },
            &mut owner,
        )
        .expect("the CancelTask read-back owner accepts the exact child");
    assert!(matches!(
        routed,
        ResidentProcessDelegateRecoveryTurn::Scheduled(DelegateRouteCall::A2aCancelTask)
    ));
    assert_eq!(a2a.calls, [DelegateRouteCall::A2aCancelTask]);
    assert_eq!(send_preparations, 1);
    assert_eq!(send_starts, 1, "routing must not replay SendMessage");
    assert_eq!(cancel_starts, 0, "routing must not replay CancelTask");
    assert!(protocol_request_qualified.get());
}

fn prove_public_cancel_task_input_selection(fixture: RetainedCancelTaskRestartFixture<'_, '_>) {
    let RetainedCancelTaskRestartFixture {
        host,
        selector,
        store,
        sessions,
        previous_history: _,
        history,
        attempt,
        signing,
        checkpoint,
        authority: _,
        custody,
        anchor,
        parents,
        parent_request,
        send_preparations,
        send_starts,
        cancel_starts,
    } = fixture;
    let protocol_request_qualified = Rc::new(Cell::new(false));
    let protocol = LatchingCancelTaskDecoder {
        protocol_request_qualified: Rc::clone(&protocol_request_qualified),
    };
    let mut custody = RestartCustody::new(
        custody,
        RestartCustodyFault::None,
        Rc::clone(&protocol_request_qualified),
        Rc::new(Cell::new(false)),
    );
    let mut anchor = RestartAnchor::new(
        anchor,
        RestartAnchorFault::None,
        Rc::clone(&protocol_request_qualified),
    );
    let disposition = classify_retained_pairwise_delegate(RestartClassificationInput {
        host,
        selector,
        store: &store,
        history: &history,
        attempt,
        signing,
        checkpoint: &checkpoint,
        custody: &mut custody,
        anchor: &mut anchor,
        parents,
        parent_request: &parent_request,
        protocol: &protocol,
        mode: RestartClassifierMode::CancelTask(Some(RestartClassifierBounds::standard())),
    })
    .expect("one exact retained CancelTask disposition");
    let calls = Rc::new(PublicDelegateInputCalls::default());
    let mut public = crate::resident::process_host::service_assembly::
        ResidentServiceDelegateRecoveryAdapter::new(
            RefusingPublicDelegateInputs(Rc::clone(&calls)),
            1,
        );
    let mut owner =
        crate::resident::process_host::service_assembly::resident_service_delegate_recovery_owner::<
            _,
            UnprovedHeldWriter,
        >(&mut public);
    let routed = host.route_delegate_restart_disposition(
        disposition,
        ResidentDeploymentOperationAccess {
            store: &store,
            custody: &mut custody,
            sessions,
            anchor: &mut anchor,
        },
        &mut owner,
    );
    assert!(matches!(
        routed,
        Err(ResidentProcessDelegateRecoveryError::Route(
            pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable
        ))
    ));
    assert_eq!(calls.cancel_recovery.get(), 1);
    assert_eq!(calls.initial_launch.get(), 0);
    assert_eq!(calls.pre_task.get(), 0);
    assert_eq!(calls.task_observation.get(), 0);
    assert_eq!(send_preparations, 1);
    assert_eq!(
        send_starts, 1,
        "public recovery must not replay SendMessage"
    );
    assert_eq!(
        cancel_starts, 0,
        "public recovery must not replay CancelTask"
    );
    assert!(protocol_request_qualified.get());
}

fn with_retained_cancel_task_restart(
    request_bytes: Option<Vec<u8>>,
    run: impl for<'a, 'store> FnOnce(RetainedCancelTaskRestartFixture<'a, 'store>),
) {
    with_retained_process_a2a_task_and_one_completed_append(|fixture| {
        prepare_retained_cancel_task_restart(request_bytes, fixture, run);
    });
}

fn with_retained_cancel_task_restart_and_two_completed_appends(
    request_bytes: Option<Vec<u8>>,
    run: impl for<'a, 'store> FnOnce(RetainedCancelTaskRestartFixture<'a, 'store>),
) {
    with_retained_process_a2a_task_and_two_completed_appends(|fixture| {
        prepare_retained_cancel_task_restart(request_bytes, fixture, run);
    });
}

fn prepare_retained_cancel_task_restart<'a, 'store>(
    request_bytes: Option<Vec<u8>>,
    fixture: RetainedProcessA2aTaskFixture<'a, 'store>,
    run: impl FnOnce(RetainedCancelTaskRestartFixture<'a, 'store>),
) {
    let RetainedProcessA2aTaskFixture {
        host,
        selector,
        store,
        sessions,
        history,
        provider_recovery: _,
        attempt,
        signing,
        checkpoint,
        checkpoint_ref: _,
        authority,
        custody,
        anchor,
        parents,
        parent_request,
        send_preparations,
        send_starts,
    } = fixture;
    let port = request_bytes.map_or_else(CountingCancelPort::default, |bytes| {
        CountingCancelPort::with_request_bytes(bytes)
    });
    let request = cancellation_request(attempt.request, attempt.operation.handle().1);
    let request_id = A2aJsonRpcRequestIdV1::parse("process-cancel-rpc").unwrap();
    let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
    let checkpoint_authority = checkpoint_signer(&checkpoint);
    let resolver = SelectedStore(store.clone());
    let prepared = match host.prepare_a2a_cancel_task(
                    &selector,
                    ResidentDeploymentOperationAccess {
                        store: &store,
                        custody,
                        sessions,
                        anchor,
                    },
                    authority,
                    cancel_input(
                        &request,
                        &history,
                        &attempt,
                        &request_id,
                        &observation_schema,
                        signing,
                    ),
                    &port,
                    &Clock::at(71),
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                    OperationCheckpointInput {
                        reference: record_ref("process/cancel-checkpoint"),
                        id: ControlCheckpointIdV1::parse("process-cancel-checkpoint").unwrap(),
                    },
                    UnixTimeV1::new(72).unwrap(),
                    checkpoint.schema_sha256(),
                    &checkpoint_authority,
                    AccumulatorPreparationLimits::new(8, 20_000, 20_000, 8_192, 200_000)
                        .unwrap(),
                    &resolver,
                    |_, _, _| Ok(signature()),
                ) {
                Ok(prepared) => prepared,
                Err(crate::resident::process_host::a2a_cancel_task::ResidentProcessA2aCancelTaskPreparationFailure::Selection(_)) => panic!("CancelTask deployment selection"),
                Err(crate::resident::process_host::a2a_cancel_task::ResidentProcessA2aCancelTaskPreparationFailure::RouteConflict) => panic!("CancelTask route conflict"),
                Err(crate::resident::process_host::a2a_cancel_task::ResidentProcessA2aCancelTaskPreparationFailure::RouteSaturated) => panic!("CancelTask route saturation"),
                Err(crate::resident::process_host::a2a_cancel_task::ResidentProcessA2aCancelTaskPreparationFailure::Rejected(_)) => panic!("CancelTask preflight rejection"),
                Err(crate::resident::process_host::a2a_cancel_task::ResidentProcessA2aCancelTaskPreparationFailure::Closed(reason)) => panic!("CancelTask preparation closed: {reason:?}"),
                Err(crate::resident::process_host::a2a_cancel_task::ResidentProcessA2aCancelTaskPreparationFailure::Release { .. }) => panic!("CancelTask preparation release"),
            };
    let retained = host
        .retain_prepared_a2a_cancel_task(
            ResidentDeploymentOperationAccess {
                store: &store,
                custody,
                sessions,
                anchor,
            },
            authority,
            prepared,
        )
        .unwrap_or_else(|_| panic!("CancelTask request custody"));
    let cancellation_checkpoint = retained.request_checkpoint().1.clone();
    let previous_history = history;
    let history = recover_retained_a2a_task_history_after_local_commit(
        &previous_history,
        attempt.operation,
        &cancellation_checkpoint,
        signing,
        authority,
        custody,
        anchor,
    );
    let cancel_starts = port.starts();
    run(RetainedCancelTaskRestartFixture {
        host,
        selector,
        store,
        sessions,
        previous_history,
        history,
        attempt,
        signing,
        checkpoint: cancellation_checkpoint,
        authority,
        custody,
        anchor,
        parents,
        parent_request,
        send_preparations,
        send_starts,
        cancel_starts,
    });
    assert_eq!(
        port.starts(),
        cancel_starts,
        "restart cannot start CancelTask"
    );
}

fn request_admission_context(
    request: &CommonRequestEnvelopePreimageV1,
) -> CommonRequestAdmissionContextV1<'_> {
    CommonRequestAdmissionContextV1 {
        expected_schema_sha256: &request.envelope_schema_sha256,
        expected_body_schema_id: &request.body.schema_id,
        expected_body_schema_sha256: &request.body.schema_sha256,
        caller: &request.caller,
        signing_authority: &request.signing_authority_binding,
        route: &request.route,
        session_admission: &request.session_admission,
        operation_target: request.operation_target.as_ref(),
        execution_owner_fence: None,
        federation_claim: None,
        workspace_id: &request.workspace_id,
        capability_id: &request.capability_id,
        operation: &request.operation,
        scope: &request.scope,
        risk_class: &request.risk_class,
        resource_constraints: &request.resource_constraints,
        expected_precondition: None,
        idempotency_key: &request.idempotency_key,
        issued_at: request.issued_at,
        expires_at: request.expires_at,
        generation: request.generation.as_ref(),
        fence_token: request.fence_token.as_ref(),
    }
}

#[derive(Clone, Copy)]
struct RestartClassifierBounds {
    request_bytes: NonZeroUsize,
    bindings: usize,
    binding_bytes: usize,
}

impl RestartClassifierBounds {
    fn standard() -> Self {
        Self {
            request_bytes: NonZeroUsize::new(8_192).unwrap(),
            bindings: 8,
            binding_bytes: 65_536,
        }
    }
}

struct RestartLocalAdmission<'a> {
    operation: &'a IssuedLocalOperation,
    child_request: &'a CommonRequestEnvelopeV1,
    parent_request: &'a (pale_ports::ControlRecordCoordinate, CommonRequestEnvelopeV1),
    checkpoint: &'a ControlStateCheckpointV1,
    checkpoint_authority:
        &'a pale_core::records::ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
}

impl RestartLocalAdmission<'_> {
    fn admit_request(
        &self,
        reference: &ResidentRecordRefV1,
        wire: CommonRequestEnvelopeWireV1,
    ) -> Result<CommonRequestEnvelopeV1, InvalidResidentRecordV1> {
        let request = if reference == self.operation.request_coordinate().record_ref() {
            self.child_request
        } else if reference == self.parent_request.0.record_ref() {
            &self.parent_request.1
        } else {
            return Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "common request envelope",
                invariant: "known retained request coordinate",
            });
        };
        let preimage = request.to_preimage();
        CommonRequestEnvelopeV1::admit_contextual_wire(
            wire,
            &request_admission_context(&preimage),
            |_, _, actual| {
                let expected_signature = request.caller_signature();
                if actual != expected_signature {
                    return Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "common request envelope",
                        invariant: "exact retained caller signature",
                    });
                }
                Ok(())
            },
        )
    }

    fn admit_handle_authority(
        &self,
        preimage: &OperationHandlePreimageV1,
        parent: Option<&OperationHandleV1>,
    ) -> Result<(), InvalidResidentRecordV1> {
        let child_handle = self.operation.handle().1;
        let expected_parent = child_handle
            .parent_operation()
            .as_ref()
            .expect("Delegate restart has one Consult parent");
        if preimage.operation_handle_id == *child_handle.operation_handle_id() {
            let parent = parent.ok_or(InvalidResidentRecordV1::ContextualAdmission {
                record: "operation handle",
                invariant: "exact recovered Delegate parent",
            })?;
            if parent.operation_handle_id() != &expected_parent.operation_handle_id
                || parent.canonical_operation_handle_sha256()
                    != &expected_parent.operation_handle_sha256
            {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "operation handle",
                    invariant: "exact recovered Delegate parent",
                });
            }
        } else if preimage.operation_handle_id != expected_parent.operation_handle_id
            || parent.is_some()
        {
            return Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "operation handle",
                invariant: "one admitted Consult parent",
            });
        }
        Ok(())
    }

    fn verify_signature(&self, actual: &CanonicalSignatureV1) {
        assert_eq!(actual, &signature());
    }

    fn admit_checkpoint(
        &self,
        wire: ControlStateCheckpointWireV1,
    ) -> Result<ControlStateCheckpointV1, InvalidResidentRecordV1> {
        ControlStateCheckpointV1::admit_signed_wire(
            wire,
            self.checkpoint.schema_sha256(),
            self.checkpoint_authority,
            |_, _, _, actual| {
                self.verify_signature(actual);
                Ok(())
            },
        )
    }
}

struct RestartA2aAdmission<'a> {
    route: (&'a ResidentRecordRefV1, &'a A2aRouteAdmissionV1),
    signing: &'a SigningAuthorityBindingV1,
}

impl RestartA2aAdmission<'_> {
    fn admit_route(
        &self,
        reference: &ResidentRecordRefV1,
        wire: A2aRouteAdmissionWireV1,
    ) -> Result<A2aRouteAdmissionV1, InvalidResidentRecordV1> {
        assert_eq!(reference, self.route.0);
        A2aRouteAdmissionV1::admit_contextual_wire(
            wire,
            &A2aRouteAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: self.route.1.schema_sha256(),
                    signing_authority: self.signing,
                },
                provider_binding: self.route.1.provider_binding(),
                route: self.route.1.route(),
            },
            |_| Ok(()),
            |_, _, _, actual| {
                self.verify_signature(actual);
                Ok(())
            },
        )
    }

    fn admit_binding_evidence(
        &self,
        preimage: &A2aTaskBindingPreimageV1,
        evidence: &[&OperationA2aEvidence],
    ) {
        let coordinates = evidence
            .iter()
            .map(|item| item.coordinate())
            .collect::<Vec<_>>();
        for expected in [&preimage.request_evidence, &preimage.initial_task_evidence] {
            assert!(coordinates.iter().any(|coordinate| {
                coordinate.record_ref() == &expected.record_ref
                    && coordinate.record_sha256().as_str() == expected.record_sha256.as_str()
            }));
        }
        assert_eq!(coordinates.len(), 2);
    }

    fn admit_observation_evidence(
        &self,
        preimage: &A2aTaskObservationPreimageV1,
        evidence: &[&OperationA2aEvidence],
    ) {
        let expected = match &preimage.source {
            A2aTaskObservationSourceV1::InitialSendMessageResponse { evidence } => evidence,
            _ => panic!("restart fixture retains one initial SendMessage observation"),
        };
        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].coordinate().record_ref(), &expected.record_ref);
        assert_eq!(
            evidence[0].coordinate().record_sha256().as_str(),
            expected.record_sha256.as_str()
        );
    }

    fn verify_signature(&self, actual: &CanonicalSignatureV1) {
        assert_eq!(actual, &signature());
    }
}

struct RestartClassificationInput<'call, 'operation, 'store> {
    host: &'call ResidentProcessHost<'store, UnprovedHeldWriter>,
    selector: ResidentDeploymentAuthoritySelector,
    store: &'call AuthorityStoreCoordinate,
    history: &'call RecoveredA2aTaskHistory,
    attempt: OperationAttemptHistoryInput<'operation>,
    signing: &'operation SigningAuthorityBindingV1,
    checkpoint: &'call ControlStateCheckpointV1,
    custody: &'call mut dyn OperationCustodyPort,
    anchor: &'call mut dyn AntiRollbackAnchorPort,
    parents: RecoveryParents<'operation>,
    parent_request: &'call (pale_ports::ControlRecordCoordinate, CommonRequestEnvelopeV1),
    protocol: &'call dyn A2aCancelTaskRequestEvidencePort,
    mode: RestartClassifierMode,
}

#[derive(Clone, Copy)]
enum RestartClassifierMode {
    CancelTask(Option<RestartClassifierBounds>),
    KnownTask(RestartClassifierBounds),
    NoA2aAdmission,
}

fn classify_retained_pairwise_delegate(
    input: RestartClassificationInput<'_, '_, '_>,
) -> Result<ResidentProcessOperationRestartDisposition, ResidentProcessOperationRestartError> {
    let RestartClassificationInput {
        host,
        selector,
        store,
        history,
        attempt,
        signing,
        checkpoint,
        custody,
        anchor,
        parents,
        parent_request,
        protocol,
        mode,
    } = input;
    let (sessions, resolver) = restart_parents(parents, store);
    let profile = crate::local_operation_start_fixture::profile();
    let checkpoint_authority = checkpoint_signer(checkpoint);
    let evidence = crate::resident::operation_custody::read_operation_start_for_request(
        custody,
        store,
        attempt.request,
        1 << 20,
    )
    .expect("start evidence read")
    .expect("retained start evidence");
    let local_admission = RestartLocalAdmission {
        operation: attempt.operation,
        child_request: attempt.request,
        parent_request,
        checkpoint,
        checkpoint_authority: &checkpoint_authority,
    };
    let remote_admission = RestartA2aAdmission {
        route: history.route(),
        signing,
    };
    let mut admit_request = |reference: &ResidentRecordRefV1, wire: CommonRequestEnvelopeWireV1| {
        local_admission.admit_request(reference, wire)
    };
    let mut admit_handle_authority =
        |preimage: &OperationHandlePreimageV1, parent: Option<&OperationHandleV1>| {
            local_admission.admit_handle_authority(preimage, parent)
        };
    let mut verify_handle_signature =
        |_: &[u8],
         _: &OperationHandlePreimageV1,
         _: &OperationHandleDigest,
         actual: &CanonicalSignatureV1| {
            local_admission.verify_signature(actual);
            Ok(())
        };
    let mut admit_checkpoint = |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
        local_admission.admit_checkpoint(wire)
    };
    let mut resolve_observation_signer =
        |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| Ok(signing.clone());
    let mut admit_observation_state = |_: &OperationObservationStateV1| Ok(());
    let mut admit_rotation = |_: &ResidentRecordRefV1,
                              _: &ArtifactDigest,
                              _: &ResidentRecordRefV1,
                              _: &ArtifactDigest| { Ok(()) };
    let mut verify_observation_signature =
        |_: &[u8],
         _: &OperationObservationPreimageV1,
         _: &OperationObservationDigest,
         actual: &CanonicalSignatureV1| {
            local_admission.verify_signature(actual);
            Ok(())
        };
    let mut verify_checkpoint_signature =
        |_: &[u8],
         _: &ControlStateCheckpointPreimageV1,
         _: &ControlStateCheckpointDigest,
         actual: &CanonicalSignatureV1| {
            local_admission.verify_signature(actual);
            Ok(())
        };
    let mut admit_route = |reference: &ResidentRecordRefV1, wire: A2aRouteAdmissionWireV1| {
        remote_admission.admit_route(reference, wire)
    };
    let mut resolve_binding_signer =
        |_: &ResidentRecordRefV1, _: &A2aTaskBindingWireV1| Ok(signing.clone());
    let mut admit_binding_evidence =
        |preimage: &A2aTaskBindingPreimageV1, evidence: &[&OperationA2aEvidence]| {
            remote_admission.admit_binding_evidence(preimage, evidence);
            Ok(())
        };
    let mut verify_binding_signature =
        |_: &[u8],
         _: &A2aTaskBindingPreimageV1,
         _: &A2aTaskBindingDigest,
         actual: &CanonicalSignatureV1| {
            remote_admission.verify_signature(actual);
            Ok(())
        };
    let mut resolve_remote_observation_signer =
        |_: &ResidentRecordRefV1, _: &A2aTaskObservationWireV1| Ok(signing.clone());
    let mut admit_remote_observation_evidence =
        |preimage: &A2aTaskObservationPreimageV1, evidence: &[&OperationA2aEvidence]| {
            remote_admission.admit_observation_evidence(preimage, evidence);
            Ok(())
        };
    let mut admit_remote_rotation =
        |_: &ResidentRecordRefV1,
         _: &ArtifactDigest,
         _: &ResidentRecordRefV1,
         _: &ArtifactDigest| { Ok(()) };
    let mut verify_remote_observation_signature =
        |_: &[u8],
         _: &A2aTaskObservationPreimageV1,
         _: &A2aTaskObservationDigest,
         actual: &CanonicalSignatureV1| {
            remote_admission.verify_signature(actual);
            Ok(())
        };
    let observation_limits = OperationObservationRecoveryLimits::new(16, 1 << 20, 1 << 20)
        .expect("bounded observation recovery");
    let checkpoint_limits = OperationCheckpointAncestryLimits::new(16, 1 << 20, 16)
        .expect("bounded checkpoint recovery");
    let membership_limits = AccumulatorMembershipLimits::new(64, 100_000, 1_000_000)
        .expect("bounded membership recovery");
    let local_limits = ResidentProcessRestartReadLimits {
        observation: observation_limits,
        checkpoints: checkpoint_limits,
        membership: membership_limits,
    };
    let recovery_context = LocalOperationRecoveryContext {
        store,
        profile: &profile,
        handle_signing_authority: signing,
        checkpoint_authority: &checkpoint_authority,
        historical: &resolver,
        maximum_record_bytes: 1 << 20,
        maximum_operation_lease_bytes: 1 << 20,
        maximum_session_lease_bytes: 1 << 20,
        membership_limits,
        observation_limits,
        checkpoint_limits,
    };
    let parent_evidence = crate::resident::operation_custody::read_operation_start_for_request(
        custody,
        store,
        &parent_request.1,
        1 << 20,
    )
    .expect("parent start evidence read")
    .expect("retained parent start evidence");
    let parent = host.classify_existing_operation(
        ResidentProcessExistingOperationStart {
            selector: selector.clone(),
            evidence: Box::new(parent_evidence),
        },
        ResidentOperationControlAssociation {
            current_request_authority: signing,
            historical_handle_authority: signing,
            recovery_store: store,
            checkpoint_authority: &checkpoint_authority,
        },
        ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions: &sessions,
            anchor,
        },
        ResidentProcessOperationRestartInput {
            committed_observation_limits: None,
            operation: recovery_context,
            maximum_parent_depth: 0,
            current_head: local_limits,
            maximum_selection_bytes: 1 << 20,
            a2a_task: None,
            a2a_cancel_task: None,
        },
        OperationControlRecoveryAdmission {
            admit_request: &mut admit_request,
            admit_handle_authority: &mut admit_handle_authority,
            verify_handle_signature: &mut verify_handle_signature,
            admit_checkpoint: &mut admit_checkpoint,
            resolve_observation_signer: &mut resolve_observation_signer,
            admit_observation_state: &mut admit_observation_state,
            admit_rotation: &mut admit_rotation,
            verify_observation_signature: &mut verify_observation_signature,
            verify_checkpoint_signature: &mut verify_checkpoint_signature,
        },
    )?;
    let parent = match parent {
        ResidentProcessOperationRestartDisposition::Consult(parent) => parent,
        ResidentProcessOperationRestartDisposition::OperationResolution(parent) => {
            Box::new(parent.start)
        }
        _ => return Err(ResidentProcessOperationRestartError::State),
    };
    macro_rules! classify_child {
        ($a2a_task:expr, $a2a_cancel_task:expr) => {
            host.classify_delegate_child_operation(
                ResidentProcessExistingOperationStart {
                    selector,
                    evidence: Box::new(evidence),
                },
                ResidentOperationControlAssociation {
                    current_request_authority: signing,
                    historical_handle_authority: signing,
                    recovery_store: store,
                    checkpoint_authority: &checkpoint_authority,
                },
                ResidentDeploymentOperationAccess {
                    store,
                    custody,
                    sessions: &sessions,
                    anchor,
                },
                ResidentProcessOperationRestartInput {
                    committed_observation_limits: None,
                    operation: recovery_context,
                    maximum_parent_depth: 0,
                    current_head: local_limits,
                    maximum_selection_bytes: 1 << 20,
                    a2a_task: $a2a_task,
                    a2a_cancel_task: $a2a_cancel_task,
                },
                &mut OperationControlRecoveryAdmission {
                    admit_request: &mut admit_request,
                    admit_handle_authority: &mut admit_handle_authority,
                    verify_handle_signature: &mut verify_handle_signature,
                    admit_checkpoint: &mut admit_checkpoint,
                    resolve_observation_signer: &mut resolve_observation_signer,
                    admit_observation_state: &mut admit_observation_state,
                    admit_rotation: &mut admit_rotation,
                    verify_observation_signature: &mut verify_observation_signature,
                    verify_checkpoint_signature: &mut verify_checkpoint_signature,
                },
                &parent,
            )
        };
    }
    match mode {
        RestartClassifierMode::NoA2aAdmission => classify_child!(None, None),
        RestartClassifierMode::KnownTask(bounds) => classify_child!(
            Some(ResidentProcessA2aTaskRestartInput {
                maximum_bindings: bounds.bindings,
                maximum_binding_bytes: bounds.binding_bytes,
                send: A2aSendRecoveryLimits::new(
                    1 << 20,
                    8_192,
                    checkpoint_limits,
                    membership_limits,
                )
                .unwrap(),
                task: A2aTaskRecoveryLimits::new(1, 65_536, 1 << 20, 2, 8_192).unwrap(),
                remote: A2aTaskRecoveryAdmission {
                    admit_route: &mut admit_route,
                    resolve_binding_signer: &mut resolve_binding_signer,
                    admit_binding_evidence: &mut admit_binding_evidence,
                    verify_binding_signature: &mut verify_binding_signature,
                    resolve_observation_signer: &mut resolve_remote_observation_signer,
                    admit_observation_evidence: &mut admit_remote_observation_evidence,
                    admit_rotation: &mut admit_remote_rotation,
                    verify_observation_signature: &mut verify_remote_observation_signature,
                },
            }),
            None
        ),
        RestartClassifierMode::CancelTask(bounds) => classify_child!(
            None,
            bounds.map(|bounds| ResidentProcessA2aCancelTaskRestartInput {
                protocol,
                maximum_request_bytes: bounds.request_bytes,
                maximum_bindings: bounds.bindings,
                maximum_binding_bytes: bounds.binding_bytes,
                send: A2aSendRecoveryLimits::new(
                    1 << 20,
                    8_192,
                    checkpoint_limits,
                    membership_limits,
                )
                .unwrap(),
                task: A2aTaskRecoveryLimits::new(1, 65_536, 1 << 20, 2, 8_192).unwrap(),
                remote: A2aTaskRecoveryAdmission {
                    admit_route: &mut admit_route,
                    resolve_binding_signer: &mut resolve_binding_signer,
                    admit_binding_evidence: &mut admit_binding_evidence,
                    verify_binding_signature: &mut verify_binding_signature,
                    resolve_observation_signer: &mut resolve_remote_observation_signer,
                    admit_observation_evidence: &mut admit_remote_observation_evidence,
                    admit_rotation: &mut admit_remote_rotation,
                    verify_observation_signature: &mut verify_remote_observation_signature,
                },
            })
        ),
    }
}

fn copied_attempt<'a>(
    input: &OperationAttemptHistoryInput<'a>,
) -> OperationAttemptHistoryInput<'a> {
    OperationAttemptHistoryInput {
        operation: input.operation,
        request: input.request,
        session: input.session,
        lease: input.lease,
        generation: input.generation,
        dispatched: input.dispatched,
        selection: input.selection,
        maximum_selection_bytes: input.maximum_selection_bytes,
    }
}

fn known_task_read_input<'a>(
    history: &'a RecoveredA2aTaskHistory,
    attempt: &OperationAttemptHistoryInput<'a>,
    request_id: &'a A2aJsonRpcRequestIdV1,
    name: &str,
) -> A2aGetTaskReadbackInput<'a> {
    A2aGetTaskReadbackInput {
        history,
        attempt: copied_attempt(attempt),
        request_id,
        request_record_ref: record_ref(&format!("process/{name}-request")),
        response_record_ref: record_ref(&format!("process/{name}-response")),
        limits: exchange_limits(),
        maximum_evidence_bytes: 8_192,
    }
}

#[test]
#[cfg(any())]
fn cold_restart_continues_one_known_task_without_replaying_send_message() {
    with_retained_process_a2a_task_and_one_completed_append(
        |RetainedProcessA2aTaskFixture {
             host,
             selector,
             store,
             sessions,
             history,
             provider_recovery: _,
             attempt,
             signing,
             checkpoint,
             checkpoint_ref: _,
             authority,
             custody,
             anchor,
             parents,
             parent_request,
             send_preparations,
             send_starts,
         }| {
            let continuation_attempt = copied_attempt(&attempt);
            let protocol_request_qualified = Rc::new(Cell::new(false));
            let protocol = LatchingCancelTaskDecoder {
                protocol_request_qualified: Rc::clone(&protocol_request_qualified),
            };
            let recovered = match classify_retained_pairwise_delegate(RestartClassificationInput {
                host,
                selector: selector.clone(),
                store: &store,
                history: &history,
                attempt,
                signing,
                checkpoint: &checkpoint,
                custody,
                anchor,
                parents,
                parent_request: &parent_request,
                protocol: &protocol,
                mode: RestartClassifierMode::KnownTask(RestartClassifierBounds::standard()),
            })
            .expect("known Task restart classification")
            {
                ResidentProcessOperationRestartDisposition::A2aTask(recovered) => recovered,
                ResidentProcessOperationRestartDisposition::A2aDelegateSendRecoveryOnly(_) => {
                    panic!("one exact retained Task must be selected")
                }
                _ => panic!("known Task reached another restart owner"),
            };
            assert_eq!(send_preparations, 1);
            assert_eq!(send_starts, 1);
            assert!(!protocol_request_qualified.get());

            let port = GetTaskPort::default();
            let request_id =
                A2aJsonRpcRequestIdV1::parse("process-known-task-get").expect("request identity");
            let mut cx = poll_context();
            let authorized = Cell::new(0);

            let mut crossed_selector = selector.clone();
            crossed_selector.runtime_authority_ref = record_ref("runtime/crossed-known-task");
            let crossed = host.continue_recovered_a2a_task_once(
                &recovered,
                &crossed_selector,
                ResidentDeploymentOperationAccess {
                    store: &store,
                    custody,
                    sessions,
                    anchor,
                },
                authority,
                known_task_read_input(
                    &recovered.task,
                    &continuation_attempt,
                    &request_id,
                    "known-task-crossed",
                ),
                &port,
                &Clock::at(100),
                &mut cx,
                |_| {
                    authorized.set(authorized.get() + 1);
                    Ok(())
                },
            );
            assert!(matches!(
                crossed,
                Err(ResidentProcessA2aTaskTurnFailure::Selection(_))
            ));
            assert_eq!(authorized.get(), 0);
            assert_eq!(port.prepares(), 0);
            assert_eq!(port.starts(), 0);

            let denied = host.continue_recovered_a2a_task_once(
                &recovered,
                &selector,
                ResidentDeploymentOperationAccess {
                    store: &store,
                    custody,
                    sessions,
                    anchor,
                },
                authority,
                known_task_read_input(
                    &recovered.task,
                    &continuation_attempt,
                    &request_id,
                    "known-task-denied",
                ),
                &port,
                &Clock::at(100),
                &mut cx,
                |_| {
                    authorized.set(authorized.get() + 1);
                    Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "known Task current read authorization fixture",
                        invariant: "denied",
                    })
                },
            );
            assert!(matches!(
                denied,
                Err(ResidentProcessA2aTaskTurnFailure::Start(_))
            ));
            assert_eq!(authorized.get(), 1);
            assert_eq!(port.prepares(), 0);
            assert_eq!(port.starts(), 0);

            let started = host
                .continue_recovered_a2a_task_once(
                    &recovered,
                    &selector,
                    ResidentDeploymentOperationAccess {
                        store: &store,
                        custody,
                        sessions,
                        anchor,
                    },
                    authority,
                    known_task_read_input(
                        &recovered.task,
                        &continuation_attempt,
                        &request_id,
                        "known-task-start",
                    ),
                    &port,
                    &Clock::at(100),
                    &mut cx,
                    |_| {
                        authorized.set(authorized.get() + 1);
                        Ok(())
                    },
                )
                .unwrap_or_else(|_| panic!("fresh authorized GetTask must start"));
            assert!(matches!(started, ResidentProcessA2aTaskTurn::Started(_)));
            assert_eq!(authorized.get(), 2);
            assert_eq!(port.prepares(), 1);
            assert_eq!(port.starts(), 1);

            for expected in [
                ResidentProcessA2aGetTaskPoll::PendingResponse,
                ResidentProcessA2aGetTaskPoll::ResponseReceived,
                ResidentProcessA2aGetTaskPoll::PendingEof,
            ] {
                let turn = host
                    .continue_recovered_a2a_task_once(
                        &recovered,
                        &selector,
                        ResidentDeploymentOperationAccess {
                            store: &store,
                            custody,
                            sessions,
                            anchor,
                        },
                        authority,
                        known_task_read_input(
                            &recovered.task,
                            &continuation_attempt,
                            &request_id,
                            "known-task-poll",
                        ),
                        &port,
                        &Clock::at(101),
                        &mut cx,
                        |_| panic!("an active read must not authorize or start again"),
                    )
                    .unwrap_or_else(|_| panic!("active GetTask must poll"));
                let ResidentProcessA2aTaskTurn::Polled(actual) = turn else {
                    panic!("an active GetTask must not restart")
                };
                assert_eq!(
                    std::mem::discriminant(&actual),
                    std::mem::discriminant(&expected)
                );
            }
            let awaiting = match host
                .continue_recovered_a2a_task_once(
                    &recovered,
                    &selector,
                    ResidentDeploymentOperationAccess {
                        store: &store,
                        custody,
                        sessions,
                        anchor,
                    },
                    authority,
                    known_task_read_input(
                        &recovered.task,
                        &continuation_attempt,
                        &request_id,
                        "known-task-complete",
                    ),
                    &port,
                    &Clock::at(101),
                    &mut cx,
                    |_| panic!("an active read must not authorize or start again"),
                )
                .unwrap_or_else(|_| panic!("completed GetTask must reach custody"))
            {
                ResidentProcessA2aTaskTurn::Polled(
                    ResidentProcessA2aGetTaskPoll::AwaitingCustody(awaiting),
                ) => awaiting,
                _ => panic!("completed GetTask must await durable custody"),
            };
            assert_eq!(port.starts(), 1);

            let observation_schema = schema_digest(A2A_TASK_OBSERVATION_V1_SCHEMA_ID);
            let checkpoint_authority = checkpoint_signer(&checkpoint);
            let commit_input = || A2aGetTaskCommitInput {
                observation: A2aGetTaskObservationInput {
                    history: &recovered.task,
                    operation: continuation_attempt.operation,
                    observation_ref: record_ref("process/known-task-observation"),
                    signing: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &observation_schema,
                        signing_authority: signing,
                    },
                    maximum_evidence_bytes: 8_192,
                    maximum_record_bytes: 65_536,
                },
                checkpoint: OperationCheckpointInput {
                    reference: record_ref("process/known-task-checkpoint"),
                    id: ControlCheckpointIdV1::parse("process-known-task-checkpoint")
                        .expect("checkpoint identity"),
                },
                issued_at: UnixTimeV1::new(102).expect("fixture time"),
                checkpoint_schema: checkpoint.schema_sha256(),
                checkpoint_authority: &checkpoint_authority,
                accumulator_limits: AccumulatorPreparationLimits::new(
                    8, 20_000, 20_000, 8_192, 200_000,
                )
                .expect("accumulator limits"),
                membership_limits: AccumulatorMembershipLimits::new(8, 20_000, 200_000)
                    .expect("membership limits"),
            };
            let failure = match host.retain_a2a_get_task(
                ResidentDeploymentOperationAccess {
                    store: &store,
                    custody,
                    sessions,
                    anchor,
                },
                authority,
                awaiting,
                commit_input(),
                &pale_adapter_a2a::A2aGetTaskDecoder,
                |_, _, _, _| Ok(()),
                |_, _, _| {
                    Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "known Task observation signer fixture",
                        invariant: "unavailable",
                    })
                },
                |_| Ok(crate::local_operation_start_fixture::store_coordinate()),
                |_, _, _| Ok(signature()),
            ) {
                Err(failure @ ResidentProcessA2aGetTaskRetentionFailure::Lower { .. }) => failure,
                _ => panic!("pre-physical signing failure must preserve custody"),
            };
            let retained = host
                .retry_a2a_get_task_retention(
                    ResidentDeploymentOperationAccess {
                        store: &store,
                        custody,
                        sessions,
                        anchor,
                    },
                    authority,
                    failure,
                    commit_input(),
                    &pale_adapter_a2a::A2aGetTaskDecoder,
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                    |_| Ok(crate::local_operation_start_fixture::store_coordinate()),
                    |_, _, _| Ok(signature()),
                )
                .unwrap_or_else(|_| panic!("exact retry must retain GetTask custody"));
            let completed = host
                .finish_a2a_get_task(
                    ResidentDeploymentOperationAccess {
                        store: &store,
                        custody,
                        sessions,
                        anchor,
                    },
                    authority,
                    retained,
                )
                .unwrap_or_else(|_| panic!("retained GetTask must release exact custody"));
            assert_eq!(completed.binding(), recovered.task.binding());
            assert_eq!(send_starts, 1);
            assert_eq!(port.starts(), 1);
        },
    );
}

#[test]
fn cold_restart_never_replays_a_retained_get_task_preparation() {
    with_retained_process_a2a_task_and_one_completed_append(
        |RetainedProcessA2aTaskFixture {
             host,
             selector,
             store,
             sessions,
             history,
             provider_recovery: _,
             attempt,
             signing,
             checkpoint,
             checkpoint_ref: _,
             authority,
             custody,
             anchor,
             parents,
             parent_request,
             send_preparations,
             send_starts,
         }| {
            let continuation_attempt = copied_attempt(&attempt);
            let protocol_request_qualified = Rc::new(Cell::new(false));
            let protocol = LatchingCancelTaskDecoder {
                protocol_request_qualified: Rc::clone(&protocol_request_qualified),
            };
            let recovered = match classify_retained_pairwise_delegate(RestartClassificationInput {
                host,
                selector: selector.clone(),
                store: &store,
                history: &history,
                attempt,
                signing,
                checkpoint: &checkpoint,
                custody,
                anchor,
                parents,
                parent_request: &parent_request,
                protocol: &protocol,
                mode: RestartClassifierMode::KnownTask(RestartClassifierBounds::standard()),
            })
            .expect("known Task restart classification")
            {
                ResidentProcessOperationRestartDisposition::A2aTask(recovered) => recovered,
                _ => panic!("one exact retained Task must be selected"),
            };
            let port = GetTaskPort::default();
            let request_id = A2aJsonRpcRequestIdV1::parse("process-restart-get-task-preparation")
                .expect("request identity");
            let checkpoint_authority = checkpoint_signer(&checkpoint);
            let mut crossed_selector = selector.clone();
            crossed_selector.runtime_authority_ref =
                record_ref("runtime/crossed-get-task-preparation");
            let crossed = host.prepare_recovered_a2a_get_task(
                &recovered,
                &crossed_selector,
                ResidentDeploymentOperationAccess {
                    store: &store,
                    custody,
                    sessions,
                    anchor,
                },
                authority,
                A2aGetTaskPreparationInput {
                    readback: known_task_read_input(
                        &recovered.task,
                        &continuation_attempt,
                        &request_id,
                        "crossed-get-task-preparation",
                    ),
                    preparation_ref: record_ref("process/crossed-get-task-preparation"),
                    current_predecessor: OperationCustodyIndexPredecessor::Absent,
                    maximum_record_bytes: 65_536,
                },
                &port,
                &Clock::at(100),
                |_| panic!("crossed selection cannot reach authorization"),
                OperationCheckpointInput {
                    reference: record_ref("process/crossed-get-task-checkpoint"),
                    id: ControlCheckpointIdV1::parse("process-crossed-get-task-checkpoint")
                        .expect("checkpoint identity"),
                },
                UnixTimeV1::new(101).expect("checkpoint time"),
                checkpoint.schema_sha256(),
                &checkpoint_authority,
                AccumulatorPreparationLimits::new(8, 20_000, 20_000, 8_192, 200_000)
                    .expect("accumulator limits"),
                AccumulatorMembershipLimits::new(8, 20_000, 200_000).expect("membership limits"),
                |_, _, _| panic!("crossed selection cannot reach checkpoint signing"),
            );
            assert!(matches!(
                crossed,
                Err(crate::resident::process_host::a2a_get_task::ResidentProcessA2aGetTaskPreparationFailure::Selection(
                    ResidentProcessHostSelectionError::A2aOperationAssociation
                ))
            ));
            assert_eq!(port.prepares(), 0);
            assert_eq!(port.starts(), 0);
            let prepared = host
                .prepare_recovered_a2a_get_task(
                    &recovered,
                    &selector,
                    ResidentDeploymentOperationAccess {
                        store: &store,
                        custody,
                        sessions,
                        anchor,
                    },
                    authority,
                    A2aGetTaskPreparationInput {
                        readback: known_task_read_input(
                            &recovered.task,
                            &continuation_attempt,
                            &request_id,
                            "restart-get-task-preparation",
                        ),
                        preparation_ref: record_ref("process/restart-get-task-private-preparation"),
                        current_predecessor: OperationCustodyIndexPredecessor::Absent,
                        maximum_record_bytes: 65_536,
                    },
                    &port,
                    &Clock::at(100),
                    |_| Ok(()),
                    OperationCheckpointInput {
                        reference: record_ref("process/restart-get-task-private-checkpoint"),
                        id: ControlCheckpointIdV1::parse(
                            "process-restart-get-task-private-checkpoint",
                        )
                        .expect("checkpoint identity"),
                    },
                    UnixTimeV1::new(101).expect("checkpoint time"),
                    checkpoint.schema_sha256(),
                    &checkpoint_authority,
                    AccumulatorPreparationLimits::new(8, 20_000, 20_000, 8_192, 200_000)
                        .expect("accumulator limits"),
                    AccumulatorMembershipLimits::new(8, 20_000, 200_000)
                        .expect("membership limits"),
                    |_, _, _| Ok(signature()),
                )
                .unwrap_or_else(|_| panic!("durable GetTask preparation"));
            let _retained = host
                .retain_prepared_a2a_get_task(
                    ResidentDeploymentOperationAccess {
                        store: &store,
                        custody,
                        sessions,
                        anchor,
                    },
                    authority,
                    prepared,
                )
                .unwrap_or_else(|_| panic!("durable GetTask request custody"));
            assert_eq!(port.starts(), 0);

            let disposition = host
                .recover_a2a_get_task_disposition(
                    &recovered,
                    &selector,
                    ResidentDeploymentOperationAccess {
                        store: &store,
                        custody,
                        sessions,
                        anchor,
                    },
                    authority,
                    65_536,
                )
                .unwrap_or_else(|_| panic!("cold GetTask custody classification"));
            assert!(matches!(
                disposition,
                ResidentProcessRecoveredA2aGetTaskDisposition::CarrierStartUnknown
            ));
            assert_eq!(port.starts(), 0, "cold recovery cannot replay the carrier");
            assert_eq!(send_preparations, 1);
            assert_eq!(send_starts, 1);
        },
    );
}

#[derive(Clone, Copy)]
enum ExpectedDelegateRestart {
    GenericProvider,
    A2aPreTask,
    A2aTask,
    A2aAdmissionUnavailable,
    A2aSendError,
    A2aTaskError,
    A2aTaskIncomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DelegateRouteCall {
    GenericProvider,
    A2aResolution,
    A2aPreTask,
    A2aTask,
    A2aCancelTask,
}

#[derive(Default)]
struct ProviderRouteOwner {
    calls: Vec<DelegateRouteCall>,
}

impl ResidentProcessRootProviderRecoveryOwner<UnprovedHeldWriter> for ProviderRouteOwner {
    type Output = DelegateRouteCall;
    type Error = ();

    fn complete_local_cancellation<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredLocalCancellation>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        panic!("A2A route fixture cannot become local cancellation")
    }

    fn recover_operation_resolution<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredOperationResolution>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<ResidentProcessRootProviderResolution<Self::Output>, Self::Error> {
        panic!("A2A route fixture cannot return to operation resolution")
    }

    fn recover_provider_resolved<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredProviderResolution>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        panic!("A2A route fixture cannot become provider resolution")
    }

    fn recover_provider_prepared<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredPreparedAttempt>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        panic!("A2A route fixture cannot become provider preparation")
    }

    fn recover_provider_dispatched<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredAttempt>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.calls.push(DelegateRouteCall::GenericProvider);
        Ok(DelegateRouteCall::GenericProvider)
    }

    fn recover_native_prepared<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredPreparedAttempt>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        panic!("A2A route fixture cannot become native preparation")
    }

    fn recover_native_dispatched<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredAttempt>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        panic!("A2A route fixture cannot become native dispatch")
    }
}

#[derive(Default)]
struct A2aRouteOwner {
    calls: Vec<DelegateRouteCall>,
}

impl ResidentA2aDelegateRecoveryPort<UnprovedHeldWriter> for A2aRouteOwner {
    type Output = DelegateRouteCall;
    type Error = ();

    fn recover_delegate_resolution<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredOperationResolution>,
        _: ResidentDeploymentOperationAccess<'_>,
        input: crate::resident::process_host::program::ResidentProgramDelegateResolutionInput<
            '_,
            '_,
        >,
    ) -> Result<Self::Output, Self::Error> {
        assert!(input.maximum_selection_bytes() > 0);
        assert!(input.maximum_record_bytes() > 0);
        self.calls.push(DelegateRouteCall::A2aResolution);
        Ok(DelegateRouteCall::A2aResolution)
    }

    fn recover_pre_task<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredAttempt>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.calls.push(DelegateRouteCall::A2aPreTask);
        Ok(DelegateRouteCall::A2aPreTask)
    }

    fn recover_task<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredA2aTask>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.calls.push(DelegateRouteCall::A2aTask);
        Ok(DelegateRouteCall::A2aTask)
    }

    fn recover_cancel_task<'store>(
        &mut self,
        _: &mut ResidentProcessHost<'store, UnprovedHeldWriter>,
        _: Box<ResidentProcessRecoveredA2aCancelTaskRequest>,
        _: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error> {
        self.calls.push(DelegateRouteCall::A2aCancelTask);
        Ok(DelegateRouteCall::A2aCancelTask)
    }
}

#[derive(Default)]
struct PublicDelegateInputCalls {
    initial_launch: Cell<usize>,
    pre_task: Cell<usize>,
    task_observation: Cell<usize>,
    cancel_recovery: Cell<usize>,
}

struct RefusingPublicDelegateInputs(Rc<PublicDelegateInputCalls>);

impl<'service> ResidentServiceDelegateInputPort<'service> for RefusingPublicDelegateInputs {
    type Authority = ProvingAuthority;

    fn with_initial_launch_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        _: UseInputs,
    ) -> Result<Output, pale_ports::ResidentOperationFrameFailure>
    where
        UseInputs:
            FnOnce(ResidentServiceA2aDelegateLaunchInputs<'service, Self::Authority>) -> Output,
    {
        self.0.initial_launch.set(self.0.initial_launch.get() + 1);
        Err(pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn with_pre_task_recovery_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        _: UseInputs,
    ) -> Result<Output, pale_ports::ResidentOperationFrameFailure>
    where
        UseInputs:
            FnOnce(ResidentServiceA2aPreTaskRecoveryInputs<'service, Self::Authority>) -> Output,
    {
        self.0.pre_task.set(self.0.pre_task.get() + 1);
        Err(pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn with_task_observation_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        _: UseInputs,
    ) -> Result<Output, pale_ports::ResidentOperationFrameFailure>
    where
        UseInputs:
            FnOnce(ResidentServiceA2aTaskObservationInputs<'service, Self::Authority>) -> Output,
    {
        self.0
            .task_observation
            .set(self.0.task_observation.get() + 1);
        Err(pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable)
    }

    fn with_cancel_recovery_inputs<Output, UseInputs>(
        &mut self,
        _: &ResidentServiceDeploymentSelector,
        _: &pale_ports::ResidentOperationFrameLocator,
        _: UseInputs,
    ) -> Result<Output, pale_ports::ResidentOperationFrameFailure>
    where
        UseInputs: FnOnce(ResidentServiceA2aCancelRecoveryInputs<'service>) -> Output,
    {
        self.0.cancel_recovery.set(self.0.cancel_recovery.get() + 1);
        Err(pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable)
    }
}

fn prove_delegate_restart_custody_classification(
    fault: RestartCustodyFault,
    expected: ExpectedDelegateRestart,
    mode: RestartClassifierMode,
) {
    with_retained_process_a2a_task(
        |RetainedProcessA2aTaskFixture {
             host,
             selector,
             store,
             sessions: _,
             history,
             provider_recovery: _,
             attempt,
             signing,
             checkpoint,
             checkpoint_ref: _,
             authority: _,
             custody,
             anchor,
             parents,
             parent_request,
             send_preparations,
             send_starts,
         }| {
            let protocol_request_qualified = Rc::new(Cell::new(false));
            let a2a_evidence_read = Rc::new(Cell::new(false));
            let protocol = LatchingCancelTaskDecoder {
                protocol_request_qualified: Rc::clone(&protocol_request_qualified),
            };
            let mut custody = RestartCustody::new(
                custody,
                fault,
                Rc::clone(&protocol_request_qualified),
                a2a_evidence_read,
            );
            let mut anchor = RestartAnchor::new(
                anchor,
                RestartAnchorFault::None,
                Rc::clone(&protocol_request_qualified),
            );
            let result = classify_retained_pairwise_delegate(RestartClassificationInput {
                host,
                selector,
                store: &store,
                history: &history,
                attempt,
                signing,
                checkpoint: &checkpoint,
                custody: &mut custody,
                anchor: &mut anchor,
                parents,
                parent_request: &parent_request,
                protocol: &protocol,
                mode,
            });
            match expected {
                ExpectedDelegateRestart::GenericProvider => assert!(matches!(
                    result,
                    Ok(ResidentProcessOperationRestartDisposition::Provider(_))
                )),
                ExpectedDelegateRestart::A2aPreTask => assert!(matches!(
                    result,
                    Ok(ResidentProcessOperationRestartDisposition::A2aDelegateSendRecoveryOnly(_))
                )),
                ExpectedDelegateRestart::A2aTask => assert!(matches!(
                    result,
                    Ok(ResidentProcessOperationRestartDisposition::A2aTask(_))
                )),
                ExpectedDelegateRestart::A2aAdmissionUnavailable => assert!(matches!(
                    result,
                    Err(ResidentProcessOperationRestartError::A2aAdmissionUnavailable)
                )),
                ExpectedDelegateRestart::A2aSendError => match result {
                    Err(ResidentProcessOperationRestartError::A2aSend(_)) => {}
                    Err(error) => panic!("{fault:?} reached another error owner: {error:?}"),
                    Ok(_) => panic!("{fault:?} fell back to a restart disposition"),
                },
                ExpectedDelegateRestart::A2aTaskError => match result {
                    Err(ResidentProcessOperationRestartError::A2aTask(_)) => {}
                    Err(error) => panic!("{fault:?} reached another error owner: {error:?}"),
                    Ok(_) => panic!("{fault:?} fell back to a restart disposition"),
                },
                ExpectedDelegateRestart::A2aTaskIncomplete => match result {
                    Err(ResidentProcessOperationRestartError::A2aTaskCandidatesIncomplete) => {}
                    Err(error) => panic!("{fault:?} reached another error owner: {error:?}"),
                    Ok(_) => panic!("{fault:?} fell back to a restart disposition"),
                },
            }
            assert_eq!(custody.a2a_send_preparation_reads(), 1);
            assert_eq!(send_preparations, 1);
            assert_eq!(send_starts, 1);
            assert!(!protocol_request_qualified.get());
        },
    );
}

fn prove_delegate_restart_routing(
    fault: RestartCustodyFault,
    mode: RestartClassifierMode,
    expected: DelegateRouteCall,
) {
    with_retained_process_a2a_task(
        |RetainedProcessA2aTaskFixture {
             host,
             selector,
             store,
             sessions,
             history,
             provider_recovery: _,
             attempt,
             signing,
             checkpoint,
             checkpoint_ref: _,
             authority: _,
             custody,
             anchor,
             parents,
             parent_request,
             send_preparations,
             send_starts,
         }| {
            let protocol_request_qualified = Rc::new(Cell::new(false));
            let protocol = LatchingCancelTaskDecoder {
                protocol_request_qualified: Rc::clone(&protocol_request_qualified),
            };
            let mut custody = RestartCustody::new(
                custody,
                fault,
                Rc::clone(&protocol_request_qualified),
                Rc::new(Cell::new(false)),
            );
            let mut anchor = RestartAnchor::new(
                anchor,
                RestartAnchorFault::None,
                Rc::clone(&protocol_request_qualified),
            );
            let disposition = classify_retained_pairwise_delegate(RestartClassificationInput {
                host,
                selector,
                store: &store,
                history: &history,
                attempt,
                signing,
                checkpoint: &checkpoint,
                custody: &mut custody,
                anchor: &mut anchor,
                parents,
                parent_request: &parent_request,
                protocol: &protocol,
                mode,
            })
            .expect("the exact pairwise Delegate disposition");
            if expected == DelegateRouteCall::GenericProvider {
                let mut owner = ResidentA2aDelegateRecoveryOwner::new(
                    ProviderRouteOwner::default(),
                    A2aRouteOwner::default(),
                );
                let routed = host
                    .route_delegate_restart_disposition(
                        disposition,
                        ResidentDeploymentOperationAccess {
                            store: &store,
                            custody: &mut custody,
                            sessions,
                            anchor: &mut anchor,
                        },
                        &mut owner,
                    )
                    .expect("the generic provider owner accepts its recovered Delegate");
                assert!(matches!(
                    routed,
                    ResidentProcessDelegateRecoveryTurn::Scheduled(actual) if actual == expected
                ));
                let (provider, a2a) = crate::resident::process_host::a2a_delegate_recovery_owner::a2a_recovery_owner_test_support::owners(&owner);
                assert_eq!(provider.calls, [expected]);
                assert!(a2a.calls.is_empty());
            } else {
                let mut a2a = A2aRouteOwner::default();
                let mut owner = crate::resident::process_host::service_assembly::
                    resident_service_delegate_recovery_owner::<_, UnprovedHeldWriter>(&mut a2a);
                let routed = host
                    .route_delegate_restart_disposition(
                        disposition,
                        ResidentDeploymentOperationAccess {
                            store: &store,
                            custody: &mut custody,
                            sessions,
                            anchor: &mut anchor,
                        },
                        &mut owner,
                    )
                    .expect("the resident service forwards the admitted A2A Delegate phase");
                assert!(matches!(
                    routed,
                    ResidentProcessDelegateRecoveryTurn::Scheduled(actual) if actual == expected
                ));
                assert_eq!(a2a.calls, [expected]);
            }
            assert_eq!(send_preparations, 1);
            assert_eq!(send_starts, 1, "routing must not replay SendMessage");
        },
    );
}

fn prove_public_delegate_input_selection(
    fault: RestartCustodyFault,
    mode: RestartClassifierMode,
    expected: DelegateRouteCall,
) {
    with_retained_process_a2a_task(
        |RetainedProcessA2aTaskFixture {
             host,
             selector,
             store,
             sessions,
             history,
             provider_recovery: _,
             attempt,
             signing,
             checkpoint,
             checkpoint_ref: _,
             authority: _,
             custody,
             anchor,
             parents,
             parent_request,
             send_preparations,
             send_starts,
         }| {
            let protocol_request_qualified = Rc::new(Cell::new(false));
            let protocol = LatchingCancelTaskDecoder {
                protocol_request_qualified: Rc::clone(&protocol_request_qualified),
            };
            let mut custody = RestartCustody::new(
                custody,
                fault,
                Rc::clone(&protocol_request_qualified),
                Rc::new(Cell::new(false)),
            );
            let mut anchor = RestartAnchor::new(
                anchor,
                RestartAnchorFault::None,
                Rc::clone(&protocol_request_qualified),
            );
            let disposition = classify_retained_pairwise_delegate(RestartClassificationInput {
                host,
                selector,
                store: &store,
                history: &history,
                attempt,
                signing,
                checkpoint: &checkpoint,
                custody: &mut custody,
                anchor: &mut anchor,
                parents,
                parent_request: &parent_request,
                protocol: &protocol,
                mode,
            })
            .expect("the exact pairwise Delegate disposition");
            let calls = Rc::new(PublicDelegateInputCalls::default());
            let mut public = crate::resident::process_host::service_assembly::
                ResidentServiceDelegateRecoveryAdapter::new(
                    RefusingPublicDelegateInputs(Rc::clone(&calls)),
                    1,
                );
            let mut owner = crate::resident::process_host::service_assembly::
                resident_service_delegate_recovery_owner::<_, UnprovedHeldWriter>(&mut public);
            let routed = host.route_delegate_restart_disposition(
                disposition,
                ResidentDeploymentOperationAccess {
                    store: &store,
                    custody: &mut custody,
                    sessions,
                    anchor: &mut anchor,
                },
                &mut owner,
            );
            assert!(matches!(
                routed,
                Err(ResidentProcessDelegateRecoveryError::Route(
                    pale_ports::ResidentOperationFrameFailure::ConfigurationUnavailable
                ))
            ));
            match expected {
                DelegateRouteCall::A2aPreTask => {
                    assert_eq!(calls.pre_task.get(), 1);
                    assert_eq!(calls.task_observation.get(), 0);
                }
                DelegateRouteCall::A2aTask => {
                    assert_eq!(calls.pre_task.get(), 0);
                    assert_eq!(calls.task_observation.get(), 1);
                }
                _ => panic!("the public-input fixture covers retained A2A phases"),
            }
            assert_eq!(calls.initial_launch.get(), 0);
            assert_eq!(calls.cancel_recovery.get(), 0);
            assert_eq!(send_preparations, 1);
            assert_eq!(
                send_starts, 1,
                "public recovery must not replay SendMessage"
            );
        },
    );
}

#[test]
fn cold_restart_without_a2a_preparation_uses_generic_provider_uncertainty() {
    prove_delegate_restart_custody_classification(
        RestartCustodyFault::A2aSendPreparationAbsent,
        ExpectedDelegateRestart::GenericProvider,
        RestartClassifierMode::NoA2aAdmission,
    );
}

#[test]
fn cold_restart_with_a2a_preparation_requires_its_admission_owner() {
    prove_delegate_restart_custody_classification(
        RestartCustodyFault::None,
        ExpectedDelegateRestart::A2aAdmissionUnavailable,
        RestartClassifierMode::NoA2aAdmission,
    );
}

#[test]
fn cold_restart_with_exact_send_and_no_task_uses_a2a_pre_task_recovery() {
    prove_delegate_restart_custody_classification(
        RestartCustodyFault::BindingAbsent,
        ExpectedDelegateRestart::A2aPreTask,
        RestartClassifierMode::KnownTask(RestartClassifierBounds::standard()),
    );
}

#[test]
fn public_delegate_pre_task_recovery_selects_only_the_carrier_free_input() {
    prove_public_delegate_input_selection(
        RestartCustodyFault::BindingAbsent,
        RestartClassifierMode::KnownTask(RestartClassifierBounds::standard()),
        DelegateRouteCall::A2aPreTask,
    );
}

#[test]
fn public_delegate_task_recovery_selects_only_read_only_observation_input() {
    prove_public_delegate_input_selection(
        RestartCustodyFault::None,
        RestartClassifierMode::KnownTask(RestartClassifierBounds::standard()),
        DelegateRouteCall::A2aTask,
    );
}

#[test]
fn cold_restart_with_exact_send_and_task_uses_a2a_task_recovery() {
    prove_delegate_restart_custody_classification(
        RestartCustodyFault::None,
        ExpectedDelegateRestart::A2aTask,
        RestartClassifierMode::KnownTask(RestartClassifierBounds::standard()),
    );
}

#[test]
fn delegate_recovery_router_preserves_generic_and_a2a_ownership_without_replaying_send() {
    prove_delegate_restart_routing(
        RestartCustodyFault::A2aSendPreparationAbsent,
        RestartClassifierMode::NoA2aAdmission,
        DelegateRouteCall::GenericProvider,
    );
    prove_delegate_restart_routing(
        RestartCustodyFault::BindingAbsent,
        RestartClassifierMode::KnownTask(RestartClassifierBounds::standard()),
        DelegateRouteCall::A2aPreTask,
    );
    prove_delegate_restart_routing(
        RestartCustodyFault::None,
        RestartClassifierMode::KnownTask(RestartClassifierBounds::standard()),
        DelegateRouteCall::A2aTask,
    );
}

#[test]
fn cold_restart_rejects_untrusted_or_incomplete_a2a_send_evidence() {
    for fault in [
        RestartCustodyFault::A2aSendPreparationReadUnavailable,
        RestartCustodyFault::A2aSendPreparationMalformed,
        RestartCustodyFault::A2aSendCheckpointAbsent,
        RestartCustodyFault::A2aRouteRecordMissing,
        RestartCustodyFault::A2aEvidenceRowMissing,
        RestartCustodyFault::A2aEvidenceReadUnavailable,
        RestartCustodyFault::A2aEvidenceCoordinateUnlike,
        RestartCustodyFault::A2aEvidenceMembershipUnavailable,
    ] {
        prove_delegate_restart_custody_classification(
            fault,
            ExpectedDelegateRestart::A2aSendError,
            RestartClassifierMode::KnownTask(RestartClassifierBounds::standard()),
        );
    }
}

#[test]
fn cold_restart_rejects_conflicting_task_candidates_without_fallback() {
    prove_delegate_restart_custody_classification(
        RestartCustodyFault::DuplicateBindingCandidate,
        ExpectedDelegateRestart::A2aTaskError,
        RestartClassifierMode::KnownTask(RestartClassifierBounds::standard()),
    );
}

#[test]
fn cold_restart_rejects_unavailable_or_incomplete_task_discovery_without_fallback() {
    prove_delegate_restart_custody_classification(
        RestartCustodyFault::BindingDiscoveryUnavailable,
        ExpectedDelegateRestart::A2aTaskError,
        RestartClassifierMode::KnownTask(RestartClassifierBounds::standard()),
    );
    prove_delegate_restart_custody_classification(
        RestartCustodyFault::IncompleteBindingPage,
        ExpectedDelegateRestart::A2aTaskIncomplete,
        RestartClassifierMode::KnownTask(RestartClassifierBounds::standard()),
    );
}

#[test]
fn cold_restart_recovers_one_official_cancel_task_request_without_starting_a_carrier() {
    with_retained_cancel_task_restart(None, |fixture| {
        let run = fixture.classify(
            RestartClassifierBounds::standard(),
            RestartCustodyFault::None,
            RestartAnchorFault::None,
        );
        let recovered = match run
            .disposition
            .expect("read-only cold restart classification")
        {
            ResidentProcessOperationRestartDisposition::A2aCancelTaskRequestRecoveryOnly(
                recovered,
            ) => recovered,
            ResidentProcessOperationRestartDisposition::RecoveryOnly(_) => {
                panic!("CancelTask evidence did not complete the exact recovery join")
            }
            ResidentProcessOperationRestartDisposition::Provider(_) => {
                panic!("CancelTask evidence fell back to provider recovery")
            }
            _ => panic!("retained CancelTask request reached another restart owner"),
        };
        assert_eq!(recovered.request_id.as_str(), "process-cancel-rpc");
        assert_eq!(
            recovered.task.binding(),
            (&run.expected_binding.0, &run.expected_binding.1)
        );
        assert_eq!(
            recovered.attempt.dispatch,
            ResidentProcessAttemptObservation::CancellationPredecessor
        );
        assert_eq!(run.send_preparations, 1);
        assert_eq!(run.send_starts, 1);
        assert_eq!(run.cancel_starts, 0);
        assert!(run.protocol_request_qualified);
    });
}

#[test]
fn delegate_recovery_router_hands_cancel_task_only_to_its_readback_owner() {
    with_retained_cancel_task_restart(None, prove_cancel_task_delegate_routing);
}

#[test]
fn public_delegate_cancel_recovery_selects_only_the_carrier_free_input() {
    with_retained_cancel_task_restart(None, prove_public_cancel_task_input_selection);
}

fn prove_recovered_cancel_task_uncertainty(
    custody_fault: RestartCustodyFault,
    anchor_fault: RestartAnchorFault,
) {
    with_retained_cancel_task_restart_and_two_completed_appends(None, |fixture| {
        prove_recovered_cancel_task_uncertainty_fixture(fixture, custody_fault, anchor_fault);
    });
}

fn prove_recovered_cancel_task_uncertainty_fixture(
    fixture: RetainedCancelTaskRestartFixture<'_, '_>,
    custody_fault: RestartCustodyFault,
    anchor_fault: RestartAnchorFault,
) {
    let RetainedCancelTaskRestartFixture {
        host,
        selector,
        store,
        sessions,
        previous_history: _,
        history,
        attempt,
        signing,
        checkpoint,
        authority: _,
        custody,
        anchor,
        parents,
        parent_request,
        send_preparations,
        send_starts,
        cancel_starts,
    } = fixture;
    let protocol_request_qualified = Rc::new(Cell::new(false));
    let a2a_evidence_read = Rc::new(Cell::new(false));
    let protocol = LatchingCancelTaskDecoder {
        protocol_request_qualified: Rc::clone(&protocol_request_qualified),
    };
    let mut custody = RestartCustody::new(
        custody,
        custody_fault,
        Rc::clone(&protocol_request_qualified),
        a2a_evidence_read,
    );
    let mut anchor =
        RestartAnchor::new(anchor, anchor_fault, Rc::clone(&protocol_request_qualified));
    let recovered = match classify_retained_pairwise_delegate(RestartClassificationInput {
        host,
        selector: selector.clone(),
        store: &store,
        history: &history,
        attempt: copied_attempt(&attempt),
        signing,
        checkpoint: &checkpoint,
        custody: &mut custody,
        anchor: &mut anchor,
        parents,
        parent_request: &parent_request,
        protocol: &protocol,
        mode: RestartClassifierMode::CancelTask(Some(RestartClassifierBounds::standard())),
    })
    .expect("bounded CancelTask restart classification")
    {
        ResidentProcessOperationRestartDisposition::A2aCancelTaskRequestRecoveryOnly(recovered) => {
            recovered
        }
        _ => panic!("one official CancelTask request must reach exact recovery"),
    };
    let predecessor = (
        recovered.task.local_observation().selected_head().0.clone(),
        recovered
            .task
            .local_observation()
            .selected_head()
            .1
            .canonical_operation_observation_sha256()
            .clone(),
    );
    let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
    let checkpoint_authority = checkpoint_signer(&checkpoint);
    let resolver = SelectedStore(store.clone());
    let observation_signatures = Cell::new(0);
    let checkpoint_signatures = Cell::new(0);
    let result = host.recover_restarted_a2a_cancel_task(
        recovered.as_ref(),
        ResidentDeploymentOperationAccess {
            store: &store,
            custody: &mut custody,
            sessions,
            anchor: &mut anchor,
        },
        A2aCancelTaskDispositionCommitInput {
            disposition: A2aCancelTaskDispositionInput {
                history: &history,
                operation: attempt.operation,
                diagnostic_ref: record_ref("process/restart-cancel-unknown-diagnostic"),
                observation_ref: record_ref("process/restart-cancel-unknown-observation"),
                reconciliation: ReconciliationFrontierReferenceV1 {
                    record_ref: record_ref("process/restart-cancel-reconciliation"),
                    record_sha256: ReconciliationFrontierDigest::parse(
                        ArtifactDigest::hash_bytes(b"restart after CancelTask request custody")
                            .to_string(),
                    )
                    .expect("reconciliation digest"),
                },
                signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &observation_schema,
                    signing_authority: signing,
                },
                observed_at: UnixTimeV1::new(100).expect("observation time"),
                maximum_diagnostic_bytes: 65_536,
            },
            checkpoint: OperationCheckpointInput {
                reference: record_ref("process/restart-cancel-unknown-checkpoint"),
                id: ControlCheckpointIdV1::parse("process-restart-cancel-unknown-checkpoint")
                    .expect("checkpoint identity"),
            },
            checkpoint_issued_at: UnixTimeV1::new(101).expect("checkpoint time"),
            checkpoint_schema: checkpoint.schema_sha256(),
            checkpoint_authority: &checkpoint_authority,
            accumulator_limits: AccumulatorPreparationLimits::new(
                8, 20_000, 20_000, 8_192, 200_000,
            )
            .expect("accumulator limits"),
        },
        1 << 20,
        &resolver,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _| {
            observation_signatures.set(observation_signatures.get() + 1);
            Ok(signature())
        },
        |_, _, _| {
            checkpoint_signatures.set(checkpoint_signatures.get() + 1);
            Ok(signature())
        },
    );
    let completed = match result {
        Ok(completed) => completed,
        Err(ResidentProcessRecoveredA2aCancelTaskFailure::Custody(pending)) => host
            .continue_restarted_a2a_cancel_task_custody(
                *pending,
                ResidentDeploymentOperationAccess {
                    store: &store,
                    custody: &mut custody,
                    sessions,
                    anchor: &mut anchor,
                },
            )
            .unwrap_or_else(|_| panic!("exact recovered CancelTask custody continuation")),
        Err(_) => panic!("recovered CancelTask uncertainty preparation"),
    };
    assert_eq!(
        completed.observation.1.predecessor_ref(),
        &Some(predecessor.0)
    );
    assert_eq!(
        completed.observation.1.predecessor_sha256(),
        &Some(predecessor.1)
    );
    match completed.observation.1.state() {
        OperationObservationStateV1::OutcomeUnknown { effect_receipt, .. } => {
            assert!(effect_receipt.is_none())
        }
        _ => panic!("restart after request custody must record OutcomeUnknown"),
    }
    assert_eq!(observation_signatures.get(), 1);
    assert_eq!(checkpoint_signatures.get(), 1);
    assert_eq!(send_preparations, 1);
    assert_eq!(send_starts, 1);
    assert_eq!(cancel_starts, 0);

    let completed_checkpoint = completed.checkpoint.1.clone();
    let second_request_qualified = Rc::new(Cell::new(false));
    let second_protocol = LatchingCancelTaskDecoder {
        protocol_request_qualified: Rc::clone(&second_request_qualified),
    };
    let cold = classify_retained_pairwise_delegate(RestartClassificationInput {
        host,
        selector,
        store: &store,
        history: &history,
        attempt: copied_attempt(&attempt),
        signing,
        checkpoint: &completed_checkpoint,
        custody: &mut custody,
        anchor: &mut anchor,
        parents,
        parent_request: &parent_request,
        protocol: &second_protocol,
        mode: RestartClassifierMode::CancelTask(Some(RestartClassifierBounds::standard())),
    })
    .expect("cold restart after explicit uncertainty");
    assert!(matches!(
        cold,
        ResidentProcessOperationRestartDisposition::Terminal(_)
            | ResidentProcessOperationRestartDisposition::RecoveryOnly(_)
    ));
    assert!(!second_request_qualified.get());
}

#[test]
fn cold_restart_records_one_predecessor_linked_cancel_task_uncertainty_without_replay() {
    prove_recovered_cancel_task_uncertainty(RestartCustodyFault::None, RestartAnchorFault::None);
}

#[test]
fn cold_restart_continues_unknown_database_custody_without_replaying_cancel_task() {
    prove_recovered_cancel_task_uncertainty(
        RestartCustodyFault::AppendCommitUnknownOnce,
        RestartAnchorFault::None,
    );
}

#[test]
fn cold_restart_continues_unavailable_anchor_custody_without_replaying_cancel_task() {
    prove_recovered_cancel_task_uncertainty(
        RestartCustodyFault::None,
        RestartAnchorFault::AppendUnavailableOnce,
    );
}

#[derive(Clone, Copy)]
enum RecoveredCancelTaskAssociationFault {
    Selector,
    Store,
    Generation,
    Selection,
    RequestEvidence,
    OutputReference,
    StaleHead,
}

fn prove_recovered_cancel_task_association_rejection(fault: RecoveredCancelTaskAssociationFault) {
    with_retained_cancel_task_restart(None, |fixture| {
        let RetainedCancelTaskRestartFixture {
            host,
            selector,
            store,
            sessions,
            previous_history,
            history,
            attempt,
            signing,
            checkpoint,
            authority: _,
            custody,
            anchor,
            parents,
            parent_request,
            send_preparations,
            send_starts,
            cancel_starts,
        } = fixture;
        let protocol_request_qualified = Rc::new(Cell::new(false));
        let protocol = LatchingCancelTaskDecoder {
            protocol_request_qualified: Rc::clone(&protocol_request_qualified),
        };
        let mut custody = RestartCustody::new(
            custody,
            RestartCustodyFault::None,
            Rc::clone(&protocol_request_qualified),
            Rc::new(Cell::new(false)),
        );
        let mut anchor = RestartAnchor::new(
            anchor,
            RestartAnchorFault::None,
            Rc::clone(&protocol_request_qualified),
        );
        let mut recovered = match classify_retained_pairwise_delegate(RestartClassificationInput {
            host,
            selector: selector.clone(),
            store: &store,
            history: &history,
            attempt: copied_attempt(&attempt),
            signing,
            checkpoint: &checkpoint,
            custody: &mut custody,
            anchor: &mut anchor,
            parents,
            parent_request: &parent_request,
            protocol: &protocol,
            mode: RestartClassifierMode::CancelTask(Some(RestartClassifierBounds::standard())),
        })
        .expect("bounded CancelTask restart classification")
        {
            ResidentProcessOperationRestartDisposition::A2aCancelTaskRequestRecoveryOnly(
                recovered,
            ) => recovered,
            _ => panic!("one official CancelTask request must reach exact recovery"),
        };
        if matches!(fault, RecoveredCancelTaskAssociationFault::Selector) {
            recovered.attempt.start.selector.runtime_authority_ref =
                record_ref("runtime/crossed-restart-cancel");
        }
        if matches!(fault, RecoveredCancelTaskAssociationFault::Generation) {
            recovered.attempt.start.generation_digest =
                pale_core::records::CompiledGenerationDigest::parse(
                    ArtifactDigest::hash_bytes(b"crossed restart generation").to_string(),
                )
                .expect("crossed generation digest");
        }
        if matches!(fault, RecoveredCancelTaskAssociationFault::Selection) {
            let selection = &recovered.attempt.selection;
            recovered.attempt.selection = pale_ports::ControlRecord::new(
                pale_ports::ControlRecordCoordinate::new(
                    record_ref("process/crossed-restart-cancel-selection"),
                    selection.coordinate().record_sha256().clone(),
                ),
                selection.schema_id().clone(),
                selection.schema_sha256().clone(),
                pale_ports::ControlRecordBytes::new(
                    selection.canonical_record_jcs().as_slice().to_vec(),
                )
                .expect("crossed selection bytes"),
            );
        }
        if matches!(fault, RecoveredCancelTaskAssociationFault::RequestEvidence) {
            let bytes = recovered.request_evidence.bytes().to_vec();
            recovered.request_evidence = OperationA2aEvidence::new(
                ImmutableRecordReferenceV1 {
                    record_ref: record_ref("process/crossed-restart-cancel-request"),
                    record_sha256: ArtifactDigest::hash_bytes(&bytes),
                },
                bytes,
                None,
                1 << 20,
            )
            .expect("bounded crossed request evidence");
        }
        let crossed_store = super::store(
            "crossed-restart-cancel-store",
            "crossed-restart-cancel-incarnation",
        );
        let selected_store = if matches!(fault, RecoveredCancelTaskAssociationFault::Store) {
            &crossed_store
        } else {
            &store
        };
        let selected_history = if matches!(fault, RecoveredCancelTaskAssociationFault::StaleHead) {
            &previous_history
        } else {
            &history
        };
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_authority = checkpoint_signer(&checkpoint);
        let resolver = SelectedStore(store.clone());
        let writes_before = custody.writes();
        let diagnostic_ref =
            if matches!(fault, RecoveredCancelTaskAssociationFault::OutputReference) {
                recovered
                    .attempt
                    .selection
                    .coordinate()
                    .record_ref()
                    .clone()
            } else {
                record_ref("process/rejected-restart-cancel-diagnostic")
            };
        let result = host.recover_restarted_a2a_cancel_task(
            recovered.as_ref(),
            ResidentDeploymentOperationAccess {
                store: selected_store,
                custody: &mut custody,
                sessions,
                anchor: &mut anchor,
            },
            A2aCancelTaskDispositionCommitInput {
                disposition: A2aCancelTaskDispositionInput {
                    history: selected_history,
                    operation: attempt.operation,
                    diagnostic_ref,
                    observation_ref: record_ref("process/rejected-restart-cancel-observation"),
                    reconciliation: ReconciliationFrontierReferenceV1 {
                        record_ref: record_ref("process/rejected-restart-cancel-reconciliation"),
                        record_sha256: ReconciliationFrontierDigest::parse(
                            ArtifactDigest::hash_bytes(b"rejected restart CancelTask").to_string(),
                        )
                        .expect("reconciliation digest"),
                    },
                    signing: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &observation_schema,
                        signing_authority: signing,
                    },
                    observed_at: UnixTimeV1::new(100).expect("observation time"),
                    maximum_diagnostic_bytes: 65_536,
                },
                checkpoint: OperationCheckpointInput {
                    reference: record_ref("process/rejected-restart-cancel-checkpoint"),
                    id: ControlCheckpointIdV1::parse("process-rejected-restart-cancel-checkpoint")
                        .expect("checkpoint identity"),
                },
                checkpoint_issued_at: UnixTimeV1::new(101).expect("checkpoint time"),
                checkpoint_schema: checkpoint.schema_sha256(),
                checkpoint_authority: &checkpoint_authority,
                accumulator_limits: AccumulatorPreparationLimits::new(
                    8, 20_000, 20_000, 8_192, 200_000,
                )
                .expect("accumulator limits"),
            },
            1 << 20,
            &resolver,
            |_| panic!("association rejection cannot admit reconciliation"),
            |_, _, _, _| panic!("association rejection cannot admit rotation"),
            |_, _, _| panic!("association rejection cannot sign an observation"),
            |_, _, _| panic!("association rejection cannot sign a checkpoint"),
        );
        assert!(matches!(
            result,
            Err(ResidentProcessRecoveredA2aCancelTaskFailure::Selection(_))
        ));
        assert_eq!(custody.writes(), writes_before);
        assert_eq!(send_preparations, 1);
        assert_eq!(send_starts, 1);
        assert_eq!(cancel_starts, 0);
    });
}

#[test]
fn recovered_cancel_task_rejects_crossed_selector_store_request_and_head_before_writes() {
    for fault in [
        RecoveredCancelTaskAssociationFault::Selector,
        RecoveredCancelTaskAssociationFault::Store,
        RecoveredCancelTaskAssociationFault::Generation,
        RecoveredCancelTaskAssociationFault::Selection,
        RecoveredCancelTaskAssociationFault::RequestEvidence,
        RecoveredCancelTaskAssociationFault::OutputReference,
        RecoveredCancelTaskAssociationFault::StaleHead,
    ] {
        prove_recovered_cancel_task_association_rejection(fault);
    }
}

fn assert_a2a_request_recovery_only(run: RestartClassificationRun) {
    assert!(matches!(
        run.disposition,
        Ok(ResidentProcessOperationRestartDisposition::RecoveryOnly(_))
    ));
    assert_eq!(run.send_preparations, 1);
    assert_eq!(run.send_starts, 1);
    assert_eq!(run.cancel_starts, 0);
}

#[test]
fn cold_restart_rejects_non_cancel_task_request_shapes_without_starting_a_carrier() {
    let cases = [
        br#"{"#.to_vec(),
        br#"[{"jsonrpc":"2.0","id":"process-cancel-rpc","method":"CancelTask","params":{"id":"task-1"}}]"#.to_vec(),
        br#"{"jsonrpc":"2.0","method":"CancelTask","params":{"id":"task-1"}}"#.to_vec(),
        br#"{"jsonrpc":"2.0","id":"process-cancel-rpc","result":{"id":"task-1"}}"#.to_vec(),
        br#"{"jsonrpc":"2.0","id":"process-cancel-rpc","method":"SendMessage","params":{"id":"task-1"}}"#.to_vec(),
        br#"{"jsonrpc":"2.0","id":"process-cancel-rpc","method":"GetTask","params":{"id":"task-1"}}"#.to_vec(),
        br#"{"jsonrpc":"2.0","id":"process-cancel-rpc","method":"CancelTask","params":{"id":"other-task"}}"#.to_vec(),
        br#"{"jsonrpc":"2.0","id":"process-cancel-rpc","method":"CancelTask","params":{"id":"task-1","metadata":{}}}"#.to_vec(),
    ];
    for bytes in cases {
        with_retained_cancel_task_restart(Some(bytes), |fixture| {
            assert_a2a_request_recovery_only(fixture.classify(
                RestartClassifierBounds::standard(),
                RestartCustodyFault::None,
                RestartAnchorFault::None,
            ));
        });
    }
}

#[test]
fn cold_restart_bounds_cancel_task_request_bytes_before_protocol_decoding() {
    with_retained_cancel_task_restart(None, |fixture| {
        let mut bounds = RestartClassifierBounds::standard();
        bounds.request_bytes = NonZeroUsize::new(16).unwrap();
        let run = fixture.classify(bounds, RestartCustodyFault::None, RestartAnchorFault::None);
        assert!(!run.protocol_request_qualified);
        assert_a2a_request_recovery_only(run);
    });
}

#[test]
fn cold_restart_with_a2a_evidence_but_no_a2a_recovery_input_is_recovery_only() {
    with_retained_cancel_task_restart(None, |fixture| {
        let run = fixture.classify_without_a2a_input();
        assert!(!run.protocol_request_qualified);
        assert_a2a_request_recovery_only(run);
    });
}

#[test]
fn cold_restart_returns_recovery_only_for_unavailable_or_unlike_a2a_evidence() {
    for fault in [
        RestartCustodyFault::A2aEvidenceRowMissing,
        RestartCustodyFault::A2aEvidenceReadUnavailable,
        RestartCustodyFault::A2aEvidenceCoordinateUnlike,
        RestartCustodyFault::A2aEvidenceMembershipUnavailable,
    ] {
        with_retained_cancel_task_restart(None, |fixture| {
            assert_a2a_request_recovery_only(fixture.classify(
                RestartClassifierBounds::standard(),
                fault,
                RestartAnchorFault::None,
            ));
        });
    }
}

#[test]
fn cold_restart_requires_one_complete_bounded_binding_page() {
    for fault in [
        RestartCustodyFault::BindingDiscoveryUnavailable,
        RestartCustodyFault::DuplicateBindingCandidate,
        RestartCustodyFault::IncompleteBindingPage,
    ] {
        with_retained_cancel_task_restart(None, |fixture| {
            assert_a2a_request_recovery_only(fixture.classify(
                RestartClassifierBounds::standard(),
                fault,
                RestartAnchorFault::None,
            ));
        });
    }
    for bounds in [
        RestartClassifierBounds {
            bindings: 0,
            ..RestartClassifierBounds::standard()
        },
        RestartClassifierBounds {
            binding_bytes: 1,
            ..RestartClassifierBounds::standard()
        },
    ] {
        with_retained_cancel_task_restart(None, |fixture| {
            assert_a2a_request_recovery_only(fixture.classify(
                bounds,
                RestartCustodyFault::None,
                RestartAnchorFault::None,
            ));
        });
    }
}

#[test]
fn cold_restart_does_not_repeat_start_evidence_after_protection() {
    with_retained_cancel_task_restart(None, |fixture| {
        let run = fixture.classify(
            RestartClassifierBounds::standard(),
            RestartCustodyFault::RepeatedStartEvidenceMissing,
            RestartAnchorFault::None,
        );
        assert!(matches!(
            run.disposition,
            Ok(ResidentProcessOperationRestartDisposition::A2aCancelTaskRequestRecoveryOnly(_))
        ));
        assert_eq!(run.send_preparations, 1);
        assert_eq!(run.send_starts, 1);
        assert_eq!(run.cancel_starts, 0);
        assert!(run.protocol_request_qualified);
    });
}

#[test]
fn cold_restart_returns_recovery_only_when_final_custody_rereads_are_unavailable() {
    for custody_fault in [
        RestartCustodyFault::FinalOperationHeadMissing,
        RestartCustodyFault::FinalCheckpointReadUnavailable,
    ] {
        with_retained_cancel_task_restart(None, |fixture| {
            assert_a2a_request_recovery_only(fixture.classify(
                RestartClassifierBounds::standard(),
                custody_fault,
                RestartAnchorFault::None,
            ));
        });
    }
    with_retained_cancel_task_restart(None, |fixture| {
        assert_a2a_request_recovery_only(fixture.classify(
            RestartClassifierBounds::standard(),
            RestartCustodyFault::None,
            RestartAnchorFault::FinalReadUnavailable,
        ));
    });
}
