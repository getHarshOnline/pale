//! Cold-restart laws for provider calls that may already have happened.

use super::start::CountingAuthority;
use super::*;
use crate::local_operation_start_fixture::{record_ref, schema_digest, signature};
use crate::resident::operation_observation_custody::tests::local_recovery::{
    with_process_host_provider_recovery_fixtures, with_process_host_restart_fixtures,
    ProcessHostRestartFixture, RecoveryStore,
};
use crate::resident::process_host::provider_recovery::*;
use crate::resident::process_host::restart::*;
use crate::resident::{
    OperationAppendFailure, OperationAttemptError, OperationCheckpointInput,
    OperationObservationAuthorityStoreRejection, OperationObservationAuthorityStoreResolver,
    ProviderOperationUncertainty,
};
use crate::session_admission_fixture::ProvingAuthority;
use pale_core::canon;
use pale_core::records::{
    ControlCheckpointIdV1, ControlTransactionSequenceV1, ReconciliationFrontierDigest,
    ReconciliationFrontierReferenceV1, SignedResidentAdmissionContextV1, UnixTimeV1,
    CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID, OPERATION_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::{
    AccumulatorMutation, AccumulatorPosition, AccumulatorPreparationLimits, AntiRollbackAnchor,
    AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError, AntiRollbackAnchorObservation,
    AntiRollbackAnchorPort, AuthorityStoreCoordinate, ControlRecord, ControlStoreError,
    ControlStoreErrorKind, ControlTransactionPreparation, OperationAccumulatorPreparation,
    OperationAccumulatorPreparationError, OperationCustodyCandidate, OperationCustodyPort,
    OperationCustodyReadBackOutcome, PreparedAntiRollbackAnchorTransition,
    PreparedOperationAccumulator, StateAccumulatorBatch, UnresolvedCustodyAccumulatorBatch,
    WallClockSourceError, WallClockSourcePort,
};
use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

pub(in crate::resident::process_host) fn host_with_generation<'store>(
    port: &'store MemoryAuthorityStore,
    selector: &ResidentDeploymentAuthoritySelector,
    selected_store: &AuthorityStoreCoordinate,
    generation: crate::resident::AdmittedCompiledGeneration,
) -> ResidentProcessHost<'store, UnprovedHeldWriter> {
    let mut application = ready_application(port, selected_store.clone(), 91);
    application.fixture_retain_generation(generation);
    let deployment =
        unproved_deployment_for_selector(selector, selected_store.clone(), application);
    ResidentProcessHost::new(
        unproved_writer(selector.principal_id.clone(), vec![selected_store.clone()]),
        vec![deployment],
        NonZeroUsize::new(1).expect("one deployment"),
        ResidentOperationHost::new(task_state()),
    )
    .expect("one exact provider-recovery deployment")
}

pub(in crate::resident::process_host) fn restart_input<'a>(
    recovery: OperationControlRecoveryInput<'a>,
) -> ResidentProcessOperationRestartInput<'a> {
    ResidentProcessOperationRestartInput {
        committed_observation_limits: None,
        operation: recovery.operation.context,
        maximum_parent_depth: recovery.operation.maximum_parent_depth,
        current_head: ResidentProcessRestartReadLimits {
            observation: recovery.observation_limits,
            checkpoints: recovery.checkpoint_limits,
            membership: recovery.membership_limits,
        },
        maximum_selection_bytes: 1024 * 1024,
        a2a_task: None,
        a2a_cancel_task: None,
    }
}

pub(in crate::resident::process_host) fn association<'a>(
    signing: &'a SigningAuthorityBindingV1,
    input: &ResidentProcessOperationRestartInput<'a>,
) -> ResidentOperationControlAssociation<'a> {
    ResidentOperationControlAssociation {
        current_request_authority: signing,
        historical_handle_authority: input.operation.handle_signing_authority,
        recovery_store: input.operation.store,
        checkpoint_authority: input.operation.checkpoint_authority,
    }
}

pub(in crate::resident::process_host) struct SelectedStore(
    pub(in crate::resident::process_host) AuthorityStoreCoordinate,
);
impl OperationObservationAuthorityStoreResolver for SelectedStore {
    fn resolve(
        &self,
        _: &pale_core::records::OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        Ok(self.0.clone())
    }
}

struct RejectingStore;
impl OperationObservationAuthorityStoreResolver for RejectingStore {
    fn resolve(
        &self,
        _: &pale_core::records::OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        Err(OperationObservationAuthorityStoreRejection)
    }
}

pub(in crate::resident::process_host) struct IncreasingClock(
    pub(in crate::resident::process_host) AtomicU64,
);
impl WallClockSourcePort for IncreasingClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        Ok(UnixTimeV1::new(self.0.fetch_add(1, Ordering::SeqCst)).expect("positive modeled time"))
    }
}

pub(in crate::resident::process_host) struct AppendCustody<'a> {
    inner: &'a mut RecoveryStore,
    transaction: ControlTransactionSequenceV1,
    commits: usize,
    read_backs: usize,
    commit_results: VecDeque<Result<pale_ports::OperationCustodyCommitOutcome, ControlStoreError>>,
    read_back_results: VecDeque<Result<OperationCustodyReadBackOutcome, ControlStoreError>>,
    committed: Option<OperationCustodyCandidate>,
}

impl<'a> AppendCustody<'a> {
    pub(in crate::resident::process_host) fn committing(inner: &'a mut RecoveryStore) -> Self {
        let transaction = inner.current_transaction_sequence();
        Self {
            inner,
            transaction,
            commits: 0,
            read_backs: 0,
            commit_results: VecDeque::new(),
            read_back_results: VecDeque::new(),
            committed: None,
        }
    }

    pub(in crate::resident::process_host) fn counts(&self) -> (usize, usize) {
        (self.commits, self.read_backs)
    }

    pub(in crate::resident::process_host) fn fail_next_commit(
        &mut self,
        kind: ControlStoreErrorKind,
    ) {
        self.commit_results
            .push_back(Err(ControlStoreError::new(kind)));
    }

    pub(in crate::resident::process_host) fn push_read_back(
        &mut self,
        outcome: OperationCustodyReadBackOutcome,
    ) {
        self.read_back_results.push_back(Ok(outcome));
    }

    pub(in crate::resident::process_host) fn fail_next_read_back(
        &mut self,
        kind: ControlStoreErrorKind,
    ) {
        self.read_back_results
            .push_back(Err(ControlStoreError::new(kind)));
    }
}

impl pale_ports::ControlTransactionPort for AppendCustody<'_> {
    fn prepare_transaction(
        &mut self,
        store: &AuthorityStoreCoordinate,
    ) -> Result<ControlTransactionPreparation, ControlStoreError> {
        ControlTransactionPreparation::new(store.clone(), self.transaction)
            .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired))
    }
}

impl OperationCustodyPort for AppendCustody<'_> {
    fn read_collection_snapshot(
        &mut self,
        store: &AuthorityStoreCoordinate,
        handle: &pale_ports::ControlRecordCoordinate,
        maximum_frontier_bytes: usize,
        maximum_journal_payload_bytes: usize,
    ) -> Result<Option<pale_ports::OperationCollectionSnapshot>, ControlStoreError> {
        self.inner.read_collection_snapshot(
            store,
            handle,
            maximum_frontier_bytes,
            maximum_journal_payload_bytes,
        )
    }
    fn read_operation_session_basis(
        &mut self,
        store: &AuthorityStoreCoordinate,
        handle: &pale_ports::ControlRecordCoordinate,
        maximum_bytes: usize,
    ) -> Result<Option<pale_ports::OperationSessionBasis>, ControlStoreError> {
        self.inner
            .read_operation_session_basis(store, handle, maximum_bytes)
    }
    fn read_a2a_task_bindings(
        &mut self,
        store: &AuthorityStoreCoordinate,
        scope: &pale_ports::OperationA2aBindingScope,
        cursor: Option<&pale_ports::OperationA2aBindingCursor>,
        maximum_bindings: usize,
        maximum_body_bytes: usize,
    ) -> Result<pale_ports::OperationA2aBindings, ControlStoreError> {
        self.inner.read_a2a_task_bindings(
            store,
            scope,
            cursor,
            maximum_bindings,
            maximum_body_bytes,
        )
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
    ) -> Result<Option<(pale_ports::ControlCheckpointHead, ControlRecord)>, ControlStoreError> {
        if let Some(candidate) = &self.committed {
            let checkpoint = candidate.checkpoint();
            if checkpoint.successor().checkpoint_domain_id() == domain {
                return Ok(Some((
                    checkpoint.successor().clone(),
                    checkpoint.successor_record().clone(),
                )));
            }
        }
        self.inner
            .read_checkpoint_head(store, domain, maximum_record_bytes)
    }
    fn verify_accumulator_membership(
        &mut self,
        request: &pale_ports::OperationAccumulatorMembership,
    ) -> Result<
        pale_ports::VerifiedOperationAccumulatorMembership,
        OperationAccumulatorPreparationError,
    > {
        self.inner.verify_accumulator_membership(request)
    }
    fn prepare_accumulator(
        &mut self,
        request: &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
        let pale_ports::AccumulatorPreparationBasis::Successor(predecessor) = request.basis()
        else {
            panic!("provider recovery appends to an admitted checkpoint")
        };
        let marker = u8::try_from(self.commits + 1).expect("modeled append marker");
        PreparedOperationAccumulator::new(
            request.clone(),
            StateAccumulatorBatch::new(
                predecessor.state_root().clone(),
                ArtifactDigest::hash_bytes(&[marker; 32]),
                vec![AccumulatorMutation::new(
                    AccumulatorPosition::Leaf([marker; 32]),
                    Some(vec![marker; 32]),
                    Some(vec![marker + 1; 32]),
                    32,
                )
                .expect("modeled accumulator mutation")],
                1,
            )
            .expect("modeled state accumulator"),
            UnresolvedCustodyAccumulatorBatch::new(
                predecessor.unresolved_root().clone(),
                predecessor.unresolved_root().clone(),
                vec![],
                1,
            )
            .expect("unchanged unresolved accumulator"),
        )
        .map_err(|_| OperationAccumulatorPreparationError::Limit)
    }
    fn compare_and_retain_operation_lease(
        &mut self,
        store: &AuthorityStoreCoordinate,
        record: &ControlRecord,
    ) -> Result<pale_ports::OperationCustodyCommitOutcome, ControlStoreError> {
        self.inner.compare_and_retain_operation_lease(store, record)
    }
    fn read_operation_record(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &pale_ports::ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError> {
        self.inner
            .read_operation_record(store, coordinate, maximum_body_bytes)
    }
    fn compare_and_commit(
        &mut self,
        candidate: &pale_ports::OperationCustodyCandidate,
    ) -> Result<pale_ports::OperationCustodyCommitOutcome, ControlStoreError> {
        self.commits += 1;
        self.transaction = candidate
            .checkpoint()
            .successor()
            .committed_transaction_sequence();
        let result = match self.commit_results.pop_front() {
            Some(result) => result,
            None => Ok(pale_ports::OperationCustodyCommitOutcome::Committed),
        };
        if matches!(
            result,
            Ok(pale_ports::OperationCustodyCommitOutcome::Committed)
        ) {
            self.committed = Some(candidate.clone());
        }
        result
    }
    fn observe_exact(
        &mut self,
        _: &pale_ports::OperationCustodyCandidate,
    ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
        self.read_backs += 1;
        self.read_back_results
            .pop_front()
            .unwrap_or(Ok(OperationCustodyReadBackOutcome::Predecessor))
    }
    fn read_index(
        &mut self,
        store: &AuthorityStoreCoordinate,
        query: &pale_ports::OperationCustodyIndexQuery,
        maximum_record_bytes: usize,
    ) -> Result<pale_ports::OperationCustodyIndexObservation, ControlStoreError> {
        if let Some(record) = self.committed.as_ref().and_then(|candidate| {
            crate::resident::operation_observation_custody::tests::candidate_indexed_record(
                candidate, query,
            )
        }) {
            return pale_ports::OperationCustodyIndexObservation::exact(query.clone(), record)
                .map_err(|_| ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired));
        }
        self.inner.read_index(store, query, maximum_record_bytes)
    }
}

pub(in crate::resident::process_host) struct AppendAnchor<'a> {
    inner: &'a mut dyn AntiRollbackAnchorPort,
    advances: usize,
    observations: usize,
    advance_results:
        VecDeque<Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError>>,
    observation_results: VecDeque<Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError>>,
    retained: Option<AntiRollbackAnchor>,
}

impl<'a> AppendAnchor<'a> {
    pub(in crate::resident::process_host) fn available(
        inner: &'a mut dyn AntiRollbackAnchorPort,
    ) -> Self {
        Self {
            inner,
            advances: 0,
            observations: 0,
            advance_results: VecDeque::new(),
            observation_results: VecDeque::new(),
            retained: None,
        }
    }

    pub(in crate::resident::process_host) fn counts(&self) -> (usize, usize) {
        (self.advances, self.observations)
    }

    pub(in crate::resident::process_host) fn fail_next_advance(&mut self) {
        self.advance_results
            .push_back(Err(AntiRollbackAnchorError::new(
                pale_ports::AntiRollbackAnchorErrorKind::Unavailable,
            )));
    }
}
impl AntiRollbackAnchorPort for AppendAnchor<'_> {
    fn read(
        &mut self,
        key: &pale_ports::AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        if self
            .retained
            .as_ref()
            .is_some_and(|anchor| &anchor.key() == key)
        {
            return Ok(self.retained.clone());
        }
        self.inner.read(key)
    }
    fn compare_and_advance(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        self.advances += 1;
        let result = match self.advance_results.pop_front() {
            Some(result) => result,
            None => Ok(AntiRollbackAnchorAdvanceDisposition::Advanced),
        };
        if matches!(result, Ok(AntiRollbackAnchorAdvanceDisposition::Advanced)) {
            self.retained = Some(transition.next().clone());
        }
        result
    }
    fn observe(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        self.observations += 1;
        self.observation_results
            .pop_front()
            .unwrap_or(Ok(AntiRollbackAnchorObservation::ExactPredecessor))
    }
}

pub(in crate::resident::process_host) struct ProviderRecoveryInvocation<'a> {
    pub(in crate::resident::process_host) current:
        &'a crate::resident::AmbientSessionAdmissionContext<'a>,
    pub(in crate::resident::process_host) signing: &'a SigningAuthorityBindingV1,
    pub(in crate::resident::process_host) checkpoint_authority:
        &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    pub(in crate::resident::process_host) observation_schema: &'a SchemaDigest,
    pub(in crate::resident::process_host) checkpoint_schema: &'a SchemaDigest,
    pub(in crate::resident::process_host) clock: &'a dyn WallClockSourcePort,
    pub(in crate::resident::process_host) resolver:
        &'a dyn OperationObservationAuthorityStoreResolver,
}

impl<'a> ProviderRecoveryInvocation<'a> {
    pub(in crate::resident::process_host) fn input(
        &self,
        suffix: &str,
    ) -> ProviderOperationInvokeInput<'a> {
        ProviderOperationInvokeInput {
            current: self.current,
            signing: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: self.observation_schema,
                signing_authority: self.signing,
            },
            terminal_checkpoint: OperationCheckpointInput {
                reference: record_ref(&format!("provider-restart/checkpoint-{suffix}")),
                id: ControlCheckpointIdV1::parse(format!("provider-restart-{suffix}"))
                    .expect("provider recovery checkpoint identity"),
            },
            result_ref: record_ref(&format!("provider-restart/result-{suffix}")),
            diagnostic_ref: record_ref(&format!("provider-restart/diagnostic-{suffix}")),
            reconciliation: ReconciliationFrontierReferenceV1 {
                record_ref: record_ref(&format!("provider-restart/reconciliation-{suffix}")),
                record_sha256: ReconciliationFrontierDigest::parse("d".repeat(64))
                    .expect("reconciliation digest"),
            },
            terminal_ref: record_ref(&format!("provider-restart/terminal-{suffix}")),
            checkpoint_schema: self.checkpoint_schema,
            checkpoint_authority: self.checkpoint_authority,
            clock: self.clock,
            resolve_store: self.resolver,
            maximum_record_bytes: 1024 * 1024,
            maximum_diagnostic_bytes: 1024 * 1024,
            accumulator_limits: AccumulatorPreparationLimits::new(1, 1024, 1024, 512, 65_536)
                .expect("provider recovery accumulator limits"),
        }
    }
}

#[test]
fn provider_cold_restart_records_uncertainty_without_a_process_route() {
    let cases = Rc::new(AtomicUsize::new(0));
    let counted_cases = Rc::clone(&cases);
    with_process_host_provider_recovery_fixtures(&mut move |fixture: ProcessHostRestartFixture<
        '_,
    >| {
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            current,
            evidence,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let checkpoint_authority = recovery.operation.context.checkpoint_authority;
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_with_generation(&port, &selector, &selected_store, generation);
        let restart = restart_input(recovery);
        let disposition = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector: selector.clone(),
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &restart),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                restart,
                admission,
            )
            .expect("protected provider classification");
        let ResidentProcessOperationRestartDisposition::Provider(attempt) = disposition else {
            return;
        };
        counted_cases.fetch_add(1, Ordering::SeqCst);
        let predecessor = attempt
            .start
            .control
            .observation()
            .selected_head()
            .0
            .clone();
        let transaction = *attempt
            .start
            .control
            .observation()
            .current_checkpoint()
            .1
            .committed_transaction_sequence();
        let mut append = AppendCustody {
            inner: custody,
            transaction,
            commits: 0,
            read_backs: 0,
            commit_results: VecDeque::new(),
            read_back_results: VecDeque::new(),
            committed: None,
        };
        let mut append_anchor = AppendAnchor {
            inner: anchor,
            advances: 0,
            observations: 0,
            advance_results: VecDeque::new(),
            observation_results: VecDeque::new(),
            retained: None,
        };
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let clock = IncreasingClock(AtomicU64::new(current.now.get() + 20));
        let resolver = SelectedStore(selected_store.clone());
        let suffix = match attempt.dispatch {
            ResidentProcessAttemptObservation::CurrentHead => "ordinary",
            ResidentProcessAttemptObservation::CancellationPredecessor => "cancelled",
        };
        let result = host
            .recover_provider(
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append,
                    sessions,
                    anchor: &mut append_anchor,
                },
                &mut ProvingAuthority,
                ResidentProcessProviderRecoveryInput {
                    maximum_selection_bytes: 1024 * 1024,
                    recording: ProviderRecoveryInvocation {
                        current,
                        signing: signing_authority,
                        checkpoint_authority,
                        observation_schema: &observation_schema,
                        checkpoint_schema: &checkpoint_schema,
                        clock: &clock,
                        resolver: &resolver,
                    }
                    .input(suffix),
                    attempt: &attempt,
                },
                |_| Ok(()),
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(signature()),
                |_, _, _| Ok(signature()),
            )
            .expect("cold provider uncertainty custody");
        let ProviderOperationInvokeResult::OutcomeUnknown(unknown) = result else {
            panic!("cold restart cannot claim a conclusive provider outcome")
        };
        assert!(matches!(
            unknown.uncertainty(),
            ProviderOperationUncertainty::ColdRestartAfterDispatch
        ));
        assert_eq!(
            unknown.observation().1.predecessor_ref(),
            &Some(predecessor)
        );
        assert_eq!((append.commits, append_anchor.advances), (1, 1));
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(host.provider_routes.checked_out, 0);
        assert!(host.provider_routes.active.is_empty());
    });
    assert_eq!(cases.load(Ordering::SeqCst), 2);
}

#[test]
fn foreign_store_and_stale_authority_are_rejected_before_recovery_io() {
    let tested = Rc::new(AtomicUsize::new(0));
    let counted_tests = Rc::clone(&tested);
    with_process_host_provider_recovery_fixtures(&mut move |fixture: ProcessHostRestartFixture<
        '_,
    >| {
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            current,
            evidence,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let checkpoint_authority = recovery.operation.context.checkpoint_authority;
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_with_generation(&port, &selector, &selected_store, generation);
        let restart = restart_input(recovery);
        let disposition = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector: selector.clone(),
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &restart),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                restart,
                admission,
            )
            .expect("protected provider classification");
        let ResidentProcessOperationRestartDisposition::Provider(attempt) = disposition else {
            return;
        };
        if counted_tests.fetch_add(1, Ordering::SeqCst) != 0 {
            return;
        }
        let transaction = *attempt
            .start
            .control
            .observation()
            .current_checkpoint()
            .1
            .committed_transaction_sequence();
        let mut append = AppendCustody {
            inner: custody,
            transaction,
            commits: 0,
            read_backs: 0,
            commit_results: VecDeque::new(),
            read_back_results: VecDeque::new(),
            committed: None,
        };
        let mut append_anchor = AppendAnchor {
            inner: anchor,
            advances: 0,
            observations: 0,
            advance_results: VecDeque::new(),
            observation_results: VecDeque::new(),
            retained: None,
        };
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let clock = IncreasingClock(AtomicU64::new(current.now.get() + 20));
        let resolver = SelectedStore(selected_store.clone());
        let other_store = store("provider-restart-other", "provider-restart-other-1");
        let sign_calls = AtomicUsize::new(0);
        let result = host.recover_provider(
            ResidentDeploymentOperationAccess {
                store: &other_store,
                custody: &mut append,
                sessions,
                anchor: &mut append_anchor,
            },
            &mut ProvingAuthority,
            ResidentProcessProviderRecoveryInput {
                attempt: &attempt,
                recording: ProviderRecoveryInvocation {
                    current,
                    signing: signing_authority,
                    checkpoint_authority,
                    observation_schema: &observation_schema,
                    checkpoint_schema: &checkpoint_schema,
                    clock: &clock,
                    resolver: &resolver,
                }
                .input("foreign"),
                maximum_selection_bytes: 1024 * 1024,
            },
            |_| panic!("foreign store reached reconciliation"),
            |_, _, _, _| panic!("foreign store reached rotation"),
            |_, _, _| {
                sign_calls.fetch_add(1, Ordering::SeqCst);
                Ok(signature())
            },
            |_, _, _| {
                sign_calls.fetch_add(1, Ordering::SeqCst);
                Ok(signature())
            },
        );
        assert!(matches!(
            result,
            Err(ResidentProcessProviderRecoveryFailure::Selection(
                ResidentProcessHostSelectionError::AccessStoreAssociation
            ))
        ));
        assert_eq!((append.commits, append.read_backs), (0, 0));
        assert_eq!((append_anchor.advances, append_anchor.observations), (0, 0));
        assert_eq!(sign_calls.load(Ordering::SeqCst), 0);

        let mut authority = CountingAuthority::rejecting_signature();
        let result = host.recover_provider(
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut append,
                sessions,
                anchor: &mut append_anchor,
            },
            &mut authority,
            ResidentProcessProviderRecoveryInput {
                attempt: &attempt,
                recording: ProviderRecoveryInvocation {
                    current,
                    signing: signing_authority,
                    checkpoint_authority,
                    observation_schema: &observation_schema,
                    checkpoint_schema: &checkpoint_schema,
                    clock: &clock,
                    resolver: &resolver,
                }
                .input("authority-rejection"),
                maximum_selection_bytes: 1024 * 1024,
            },
            |_| panic!("rejected authority reached reconciliation"),
            |_, _, _, _| panic!("rejected authority reached rotation"),
            |_, _, _| {
                sign_calls.fetch_add(1, Ordering::SeqCst);
                Ok(signature())
            },
            |_, _, _| {
                sign_calls.fetch_add(1, Ordering::SeqCst);
                Ok(signature())
            },
        );
        assert!(matches!(
            result,
            Err(ResidentProcessProviderRecoveryFailure::Preparation(failure))
                if matches!(
                    *failure,
                    OperationAppendFailure::Operation(OperationAttemptError::Session(_))
                )
        ));
        assert!(authority.calls > 0);
        assert_eq!((append.commits, append.read_backs), (0, 0));
        assert_eq!((append_anchor.advances, append_anchor.observations), (0, 0));
        assert_eq!(sign_calls.load(Ordering::SeqCst), 0);
    });
    assert_eq!(tested.load(Ordering::SeqCst), 2);
}

#[test]
fn signed_unknown_and_checkpoint_survive_phase_failures_without_replay() {
    let cases = Rc::new(AtomicUsize::new(0));
    let counted_cases = Rc::clone(&cases);
    with_process_host_provider_recovery_fixtures(&mut move |fixture: ProcessHostRestartFixture<
        '_,
    >| {
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            current,
            evidence,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let checkpoint_authority = recovery.operation.context.checkpoint_authority;
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_with_generation(&port, &selector, &selected_store, generation);
        let restart = restart_input(recovery);
        let disposition = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector: selector.clone(),
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &restart),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                restart,
                admission,
            )
            .expect("protected provider classification");
        let ResidentProcessOperationRestartDisposition::Provider(attempt) = disposition else {
            return;
        };
        counted_cases.fetch_add(1, Ordering::SeqCst);
        let transaction = *attempt
            .start
            .control
            .observation()
            .current_checkpoint()
            .1
            .committed_transaction_sequence();
        let mut append = AppendCustody::committing(custody);
        append.transaction = transaction;
        let mut append_anchor = AppendAnchor::available(anchor);
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let clock = IncreasingClock(AtomicU64::new(current.now.get() + 20));
        let resolver = SelectedStore(selected_store.clone());
        let observation_signs = AtomicUsize::new(0);
        let checkpoint_signs = AtomicUsize::new(0);
        let suffix = match attempt.dispatch {
            ResidentProcessAttemptObservation::CurrentHead => "phase-current",
            ResidentProcessAttemptObservation::CancellationPredecessor => "phase-cancelled",
        };
        let failure = match host.recover_provider(
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut append,
                sessions,
                anchor: &mut append_anchor,
            },
            &mut ProvingAuthority,
            ResidentProcessProviderRecoveryInput {
                attempt: &attempt,
                recording: ProviderRecoveryInvocation {
                    current,
                    signing: signing_authority,
                    checkpoint_authority,
                    observation_schema: &observation_schema,
                    checkpoint_schema: &checkpoint_schema,
                    clock: &clock,
                    resolver: &resolver,
                }
                .input(suffix),
                maximum_selection_bytes: 1024 * 1024,
            },
            |_| Ok(()),
            |_, _, _, _| Ok(()),
            |_, _, _| {
                observation_signs.fetch_add(1, Ordering::SeqCst);
                Ok(signature())
            },
            |_, _, _| {
                checkpoint_signs.fetch_add(1, Ordering::SeqCst);
                Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "control state checkpoint",
                    invariant: "modeled checkpoint signer refusal",
                })
            },
        ) {
            Ok(_) => panic!("checkpoint signer refusal must not complete"),
            Err(failure) => failure,
        };
        let ResidentProcessProviderRecoveryFailure::Prepared(prepared) = failure else {
            panic!("checkpoint signer refusal must retain Prepared")
        };
        let terminal_bytes = canon::to_canonical_string(prepared.observation().1)
            .expect("terminal observation is canonical");
        assert_eq!(observation_signs.load(Ordering::SeqCst), 1);
        assert_eq!(checkpoint_signs.load(Ordering::SeqCst), 1);
        assert_eq!(append.counts(), (0, 0));

        let foreign_store = AuthorityStoreCoordinate::new(
            pale_core::records::ControlStoreIdV1::parse("foreign-provider-recovery-store")
                .expect("foreign store"),
            selected_store.incarnation_id().clone(),
        );
        let failure = match host.continue_prepared_provider_recovery(
            *prepared,
            ResidentDeploymentOperationAccess {
                store: &foreign_store,
                custody: &mut append,
                sessions,
                anchor: &mut append_anchor,
            },
            &resolver,
            checkpoint_authority,
            |_, _, _| -> Result<_, InvalidResidentRecordV1> {
                panic!("an unlike store must fail before checkpoint signing")
            },
        ) {
            Ok(_) => panic!("an unlike store must retain Prepared"),
            Err(failure) => failure,
        };
        let ResidentProcessProviderRecoveryFailure::Prepared(prepared) = failure else {
            panic!("an unlike store must retain Prepared")
        };
        assert!(matches!(
            prepared.selection_failure(),
            Some(ResidentProcessHostSelectionError::AccessStoreAssociation)
        ));
        assert_eq!(observation_signs.load(Ordering::SeqCst), 1);
        assert_eq!(checkpoint_signs.load(Ordering::SeqCst), 1);
        assert_eq!(append.counts(), (0, 0));

        let failure = match host.continue_prepared_provider_recovery(
            *prepared,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut append,
                sessions,
                anchor: &mut append_anchor,
            },
            &RejectingStore,
            checkpoint_authority,
            |_, _, _| {
                checkpoint_signs.fetch_add(1, Ordering::SeqCst);
                Ok(signature())
            },
        ) {
            Ok(_) => panic!("store projection refusal must not complete"),
            Err(failure) => failure,
        };
        let ResidentProcessProviderRecoveryFailure::Prepared(prepared) = failure else {
            panic!("store projection refusal must retain Prepared")
        };
        assert_eq!(
            canon::to_canonical_string(prepared.observation().1)
                .expect("retained observation is canonical"),
            terminal_bytes
        );
        assert_eq!(observation_signs.load(Ordering::SeqCst), 1);
        assert_eq!(checkpoint_signs.load(Ordering::SeqCst), 1);
        assert_eq!(append.counts(), (0, 0));

        let substituted_epoch = pale_core::records::SigningKeyEpochV1::new(
            checkpoint_authority
                .subject_key_epoch
                .get()
                .checked_add(1)
                .expect("fixture key epoch has a successor"),
        )
        .expect("positive substituted key epoch");
        let substituted_authority =
            pale_core::records::ControlCheckpointSigningAuthorityAdmissionContextV1 {
                subject_key_epoch: substituted_epoch,
                ..*checkpoint_authority
            };
        let failure = match host.continue_prepared_provider_recovery(
            *prepared,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut append,
                sessions,
                anchor: &mut append_anchor,
            },
            &resolver,
            &substituted_authority,
            |_, _, _| -> Result<_, InvalidResidentRecordV1> {
                panic!("an unlike authority must fail before checkpoint signing")
            },
        ) {
            Ok(_) => panic!("an unlike authority must retain Prepared"),
            Err(failure) => failure,
        };
        let ResidentProcessProviderRecoveryFailure::Prepared(prepared) = failure else {
            panic!("an unlike authority must retain Prepared")
        };
        assert!(matches!(
            prepared.recording_failure(),
            Some(OperationAppendFailure::Operation(
                OperationAttemptError::Association
            ))
        ));
        assert_eq!(observation_signs.load(Ordering::SeqCst), 1);
        assert_eq!(checkpoint_signs.load(Ordering::SeqCst), 1);
        assert_eq!(append.counts(), (0, 0));

        let failure = match host.continue_prepared_provider_recovery_with_test_attachment(
            *prepared,
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut append,
                sessions,
                anchor: &mut append_anchor,
            },
            &resolver,
            checkpoint_authority,
            |_, _, _| {
                checkpoint_signs.fetch_add(1, Ordering::SeqCst);
                Ok(signature())
            },
            |_, _, _| Err(crate::resident::OperationObservationCustodyProjectionError::Lineage),
        ) {
            Ok(_) => panic!("post-sign attachment failure must not complete"),
            Err(failure) => failure,
        };
        let ResidentProcessProviderRecoveryFailure::Issued(issued) = failure else {
            panic!("attachment refusal must retain Issued")
        };
        let checkpoint_bytes = canon::to_canonical_string(issued.checkpoint_value())
            .expect("issued checkpoint is canonical");
        assert_eq!(
            canon::to_canonical_string(issued.observation().1)
                .expect("retained observation is canonical"),
            terminal_bytes
        );
        assert_eq!(observation_signs.load(Ordering::SeqCst), 1);
        assert_eq!(checkpoint_signs.load(Ordering::SeqCst), 2);

        let failure = match host.continue_issued_provider_recovery(
            *issued,
            ResidentDeploymentOperationAccess {
                store: &foreign_store,
                custody: &mut append,
                sessions,
                anchor: &mut append_anchor,
            },
            &resolver,
        ) {
            Ok(_) => panic!("an unlike store must retain Issued"),
            Err(failure) => failure,
        };
        let ResidentProcessProviderRecoveryFailure::Issued(issued) = failure else {
            panic!("an unlike store must retain Issued")
        };
        assert!(matches!(
            issued.selection_failure(),
            Some(ResidentProcessHostSelectionError::AccessStoreAssociation)
        ));
        assert_eq!(observation_signs.load(Ordering::SeqCst), 1);
        assert_eq!(checkpoint_signs.load(Ordering::SeqCst), 2);
        assert_eq!(append.counts(), (0, 0));

        let ProviderOperationInvokeResult::OutcomeUnknown(unknown) = host
            .continue_issued_provider_recovery(
                *issued,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append,
                    sessions,
                    anchor: &mut append_anchor,
                },
                &resolver,
            )
            .expect("issued checkpoint attaches and reaches exact custody")
        else {
            panic!("cold provider recovery remains outcome unknown")
        };
        assert_eq!(
            canon::to_canonical_string(unknown.observation().1)
                .expect("completed observation is canonical"),
            terminal_bytes
        );
        assert_eq!(
            append
                .committed
                .as_ref()
                .expect("exact candidate committed")
                .checkpoint()
                .successor_record()
                .canonical_record_jcs()
                .as_slice(),
            checkpoint_bytes.as_bytes()
        );
        assert_eq!(observation_signs.load(Ordering::SeqCst), 1);
        assert_eq!(checkpoint_signs.load(Ordering::SeqCst), 2);
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(host.provider_routes.checked_out, 0);
        assert!(host.provider_routes.active.is_empty());
    });
    assert_eq!(cases.load(Ordering::SeqCst), 2);
}

#[test]
fn provider_recovery_continues_only_its_exact_database_or_anchor_candidate() {
    let cases = Rc::new(AtomicUsize::new(0));
    let counted_cases = Rc::clone(&cases);
    with_process_host_provider_recovery_fixtures(&mut move |fixture: ProcessHostRestartFixture<
        '_,
    >| {
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            current,
            evidence,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let checkpoint_authority = recovery.operation.context.checkpoint_authority;
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_with_generation(&port, &selector, &selected_store, generation);
        let restart = restart_input(recovery);
        let disposition = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector: selector.clone(),
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &restart),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                restart,
                admission,
            )
            .expect("protected provider classification");
        let ResidentProcessOperationRestartDisposition::Provider(attempt) = disposition else {
            return;
        };
        counted_cases.fetch_add(1, Ordering::SeqCst);
        let database_ambiguity = attempt.dispatch == ResidentProcessAttemptObservation::CurrentHead;
        let transaction = *attempt
            .start
            .control
            .observation()
            .current_checkpoint()
            .1
            .committed_transaction_sequence();
        let mut append = AppendCustody {
            inner: custody,
            transaction,
            commits: 0,
            read_backs: 0,
            commit_results: if database_ambiguity {
                VecDeque::from([Err(ControlStoreError::new(
                    ControlStoreErrorKind::OutcomeUnknown,
                ))])
            } else {
                VecDeque::new()
            },
            read_back_results: if database_ambiguity {
                VecDeque::from([
                    Ok(OperationCustodyReadBackOutcome::Predecessor),
                    Err(ControlStoreError::new(ControlStoreErrorKind::Storage)),
                    Ok(OperationCustodyReadBackOutcome::ExactSuccessor),
                ])
            } else {
                VecDeque::new()
            },
            committed: None,
        };
        let mut append_anchor = AppendAnchor {
            inner: anchor,
            advances: 0,
            observations: 0,
            advance_results: if database_ambiguity {
                VecDeque::new()
            } else {
                VecDeque::from([
                    Err(AntiRollbackAnchorError::new(
                        pale_ports::AntiRollbackAnchorErrorKind::Unavailable,
                    )),
                    Ok(AntiRollbackAnchorAdvanceDisposition::Advanced),
                ])
            },
            observation_results: VecDeque::new(),
            retained: None,
        };
        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let checkpoint_schema = schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID);
        let clock = IncreasingClock(AtomicU64::new(current.now.get() + 20));
        let resolver = SelectedStore(selected_store.clone());
        let suffix = if database_ambiguity {
            "database-continuation"
        } else {
            "anchor-continuation"
        };
        let failure = match host.recover_provider(
            ResidentDeploymentOperationAccess {
                store: &selected_store,
                custody: &mut append,
                sessions,
                anchor: &mut append_anchor,
            },
            &mut ProvingAuthority,
            ResidentProcessProviderRecoveryInput {
                attempt: &attempt,
                recording: ProviderRecoveryInvocation {
                    current,
                    signing: signing_authority,
                    checkpoint_authority,
                    observation_schema: &observation_schema,
                    checkpoint_schema: &checkpoint_schema,
                    clock: &clock,
                    resolver: &resolver,
                }
                .input(suffix),
                maximum_selection_bytes: 1024 * 1024,
            },
            |_| Ok(()),
            |_, _, _, _| Ok(()),
            |_, _, _| Ok(signature()),
            |_, _, _| Ok(signature()),
        ) {
            Ok(_) => panic!("modeled physical ambiguity must retain custody"),
            Err(failure) => failure,
        };
        let ResidentProcessProviderRecoveryFailure::Custody(pending) = failure else {
            panic!("physical ambiguity must retain the exact custody phase")
        };

        let foreign_store = store("provider-restart-foreign", "provider-restart-foreign-1");
        let rejected = match host.continue_provider_recovery_custody(
            *pending,
            ResidentDeploymentOperationAccess {
                store: &foreign_store,
                custody: &mut append,
                sessions,
                anchor: &mut append_anchor,
            },
            &mut ProvingAuthority,
        ) {
            Ok(_) => panic!("foreign store cannot consume the continuation"),
            Err(rejected) => rejected,
        };
        assert!(matches!(
            rejected.reason,
            ResidentProcessProviderRecoveryCustodyReason::Selection(
                ResidentProcessHostSelectionError::AccessStoreAssociation
            )
        ));
        let finalization = host
            .continue_provider_recovery_custody(
                *rejected.pending,
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody: &mut append,
                    sessions,
                    anchor: &mut append_anchor,
                },
                &mut ProvingAuthority,
            )
            .expect("exact continuation completes physical custody");
        let foreign_store = store(
            "provider-restart-finalization-foreign",
            "provider-restart-finalization-foreign-1",
        );
        let rejected =
            match host.continue_provider_recovery_finalization(finalization, &foreign_store) {
                Ok(_) => panic!("foreign store cannot consume semantic finalization"),
                Err(rejected) => rejected,
            };
        assert!(matches!(
            rejected.reason,
            ResidentProcessProviderRecoveryFinalizationReason::Selection(
                ResidentProcessHostSelectionError::AccessStoreAssociation
            )
        ));
        let ProviderOperationInvokeResult::OutcomeUnknown(unknown) = host
            .continue_provider_recovery_finalization(*rejected.pending, &selected_store)
            .expect("exact finalization retains uncertainty semantics")
        else {
            panic!("provider restart cannot become conclusive during continuation")
        };
        assert!(matches!(
            unknown.uncertainty(),
            ProviderOperationUncertainty::ColdRestartAfterDispatch
        ));
        assert_eq!(append.commits, 1);
        if database_ambiguity {
            assert_eq!(append.read_backs, 3);
            assert_eq!(append_anchor.advances, 1);
        } else {
            assert_eq!(append.read_backs, 1);
            assert_eq!(append_anchor.advances, 2);
        }
    });
    assert_eq!(cases.load(Ordering::SeqCst), 2);
}

#[test]
fn repeated_outcome_unknown_lookup_is_recovery_only_without_provider_replay() {
    let ordinal = Rc::new(AtomicUsize::new(0));
    let observed = Rc::new(AtomicUsize::new(0));
    let next_ordinal = Rc::clone(&ordinal);
    let counted = Rc::clone(&observed);
    with_process_host_restart_fixtures(&mut move |fixture: ProcessHostRestartFixture<'_>| {
        let current = next_ordinal.fetch_add(1, Ordering::SeqCst);
        if current != 2 {
            return;
        }
        let ProcessHostRestartFixture {
            recovery,
            admission,
            signing_authority,
            current: _,
            evidence,
            generation,
            custody,
            sessions,
            anchor,
            ..
        } = fixture;
        let selected_store = recovery.operation.context.store.clone();
        let selector = selector_from_signing_authority(signing_authority);
        let port = MemoryAuthorityStore::default();
        let host = host_with_generation(&port, &selector, &selected_store, generation);
        let input = restart_input(recovery);
        let disposition = host
            .classify_existing_operation(
                ResidentProcessExistingOperationStart {
                    selector,
                    evidence: Box::new(evidence),
                },
                association(signing_authority, &input),
                ResidentDeploymentOperationAccess {
                    store: &selected_store,
                    custody,
                    sessions,
                    anchor,
                },
                input,
                admission,
            )
            .expect("retained outcome-unknown lookup");
        assert!(matches!(
            disposition,
            ResidentProcessOperationRestartDisposition::RecoveryOnly(_)
        ));
        assert_eq!(host.provider_routes.reserved, 0);
        assert_eq!(host.provider_routes.checked_out, 0);
        assert!(host.provider_routes.active.is_empty());
        counted.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(ordinal.load(Ordering::SeqCst), 5);
    assert_eq!(observed.load(Ordering::SeqCst), 1);
}
