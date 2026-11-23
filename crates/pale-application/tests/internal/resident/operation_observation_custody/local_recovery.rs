//! Real contextual start records and phase joins, with explicit custody/proof
//! test doubles. Actual SQLite/SMT proof and reopen tests are adapter-owned.
//! These tests do not qualify a production signer, anchor backend or one combined
//! native start/restart assembly.

use super::local_start::{source_generation, with_start, RecoveryParents, StartStore};
use super::*;
use crate::local_operation_start_fixture::StartAdmission;
use crate::resident::operation_service::{
    ResidentOperationCancellationAdmission, ResidentOperationCancellationFailure,
    ResidentOperationCancellationInput, ResidentOperationCancellationOutcome,
    ResidentOperationService, ResidentOperationTaskState,
};
use crate::resident::task_owner::ResidentOperationTaskLimits;
use crate::resident::task_reservation::ResidentAggregateTaskLimits;
use crate::resident::{
    recover_local_operation, recover_local_operation_from_handle,
    LocalOperationHandleRecoveryInput, LocalOperationObservationAdmission,
    LocalOperationObservationError, LocalOperationObservationInput,
    LocalOperationRecoveryAdmission, LocalOperationRecoveryContext, LocalOperationRecoveryError,
    OperationAppendFailure, OperationCheckpointAncestryLimits, OperationCheckpointInput,
    OperationControlAuthorizationContext, OperationControlError, OperationControlObservation,
    OperationControlRecoveryAdmission, OperationControlRecoveryInput,
    OPERATION_CANCEL_CAPABILITY_ID, OPERATION_OBSERVE_CAPABILITY_ID,
};
use crate::resident_fixture::RecoveryResolver;
use crate::session_admission_fixture::ProvingAuthority;
use pale_ports::*;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{cell::RefCell, rc::Rc};

type RecoveryTrace = Rc<RefCell<Vec<&'static str>>>;

/// One real application-Observe fixture lent to the private process-host tests.
///
/// The fixture retains the existing recovery and admission owners. It does not
/// provide a second Observe implementation or a permissive storage shortcut.
pub(in crate::resident) struct ProcessHostObserveFixture<'a> {
    /// Exact input used by the application-owned Observe journey.
    pub(in crate::resident) input: LocalOperationObservationInput<'a>,
    /// Existing historical and present authority callbacks.
    pub(in crate::resident) admission: LocalOperationObservationAdmission<'a>,
    /// Historical signing authority retained by the operation handle.
    pub(in crate::resident) signing_authority: &'a SigningAuthorityBindingV1,
    /// In-memory custody owner with the real protected-recovery records.
    pub(in crate::resident) custody: &'a mut dyn OperationCustodyPort,
    /// In-memory historical session owner used by protected recovery.
    pub(in crate::resident) sessions: &'a dyn SessionLeaseCustodyPort,
    /// In-memory independent-anchor owner used by protected recovery.
    pub(in crate::resident) anchor: &'a mut dyn AntiRollbackAnchorPort,
}

type ProcessHostObserveRunner =
    dyn for<'a> FnMut(
        ProcessHostObserveFixture<'a>,
    ) -> Result<OperationControlObservation, LocalOperationObservationError>;

/// One real Observe fixture with retained authority for `ResidentProgram` tests.
pub(in crate::resident) struct ResidentProgramObserveFixture<'a> {
    pub(in crate::resident) observation: ProcessHostObserveFixture<'a>,
    pub(in crate::resident) session: &'a crate::resident::AdmittedAmbientSession,
    pub(in crate::resident) current: &'a crate::resident::AmbientSessionAdmissionContext<'a>,
    pub(in crate::resident) generation: crate::resident::AdmittedCompiledGeneration,
    /// Exact byte-identical source input for cold reconstruction of this generation.
    pub(in crate::resident) reconstruction: crate::resident::ResidentGenerationReconstructionInput,
}

type ResidentProgramObserveRunner = dyn for<'a> FnMut(ResidentProgramObserveFixture<'a>);

/// One protected-recovery fixture lent to private cold-restart classification.
pub(in crate::resident) struct ProcessHostRestartFixture<'a> {
    pub(in crate::resident) recovery: OperationControlRecoveryInput<'a>,
    pub(in crate::resident) admission: OperationControlRecoveryAdmission<'a>,
    pub(in crate::resident) signing_authority: &'a SigningAuthorityBindingV1,
    pub(in crate::resident) session: &'a crate::resident::AdmittedAmbientSession,
    pub(in crate::resident) current: &'a crate::resident::AmbientSessionAdmissionContext<'a>,
    pub(in crate::resident) request: &'a CommonRequestEnvelopeV1,
    pub(in crate::resident) native_registry: &'a crate::actions::NativeActionRegistry,
    pub(in crate::resident) evidence:
        crate::resident::operation_custody::OperationStartRecoveryEvidence,
    pub(in crate::resident) generation: crate::resident::AdmittedCompiledGeneration,
    pub(in crate::resident) rediscovery_generation:
        Option<crate::resident::AdmittedCompiledGeneration>,
    pub(in crate::resident) custody: &'a mut RecoveryStore,
    pub(in crate::resident) sessions: &'a dyn SessionLeaseCustodyPort,
    pub(in crate::resident) anchor: &'a mut ReadAnchor,
}

type ProcessHostRestartRunner = dyn for<'a> FnMut(ProcessHostRestartFixture<'a>);

/// One retained request presented to root-Invoke request resolution.
pub(in crate::resident) struct ProcessHostInvokeRequestFixture<'a> {
    pub(in crate::resident) recovery: OperationControlRecoveryInput<'a>,
    pub(in crate::resident) admission: OperationControlRecoveryAdmission<'a>,
    pub(in crate::resident) request: &'a CommonRequestEnvelopeV1,
    pub(in crate::resident) session: &'a crate::resident::AdmittedAmbientSession,
    pub(in crate::resident) current: &'a crate::resident::AmbientSessionAdmissionContext<'a>,
    pub(in crate::resident) signing_authority: &'a SigningAuthorityBindingV1,
    pub(in crate::resident) generation: crate::resident::AdmittedCompiledGeneration,
    pub(in crate::resident) custody: &'a mut RecoveryStore,
    pub(in crate::resident) sessions: &'a dyn SessionLeaseCustodyPort,
    pub(in crate::resident) anchor: &'a mut dyn AntiRollbackAnchorPort,
}

type ProcessHostInvokeRequestRunner = dyn for<'a> FnMut(ProcessHostInvokeRequestFixture<'a>);

/// One real application-Cancel fixture lent to the private process-host tests.
///
/// The fixture retains the existing recovery, present-authority, custody, and
/// signing owners. It does not provide another cancellation implementation.
pub(in crate::resident) struct ProcessHostCancellationFixture<'a> {
    /// Exact input used by the application-owned Cancel journey.
    pub(in crate::resident) input: ResidentOperationCancellationInput<'a>,
    /// Existing historical, present-authority, store, and signing callbacks.
    pub(in crate::resident) admission: ResidentOperationCancellationAdmission<'a>,
    /// Historical signing authority retained by the operation handle.
    pub(in crate::resident) signing_authority: &'a SigningAuthorityBindingV1,
    /// Exact recovered operation handle used by collection fixtures.
    pub(in crate::resident) operation_handle: (ResidentRecordRefV1, OperationHandleV1),
    /// In-memory custody owner with the real protected-recovery records.
    pub(in crate::resident) custody: &'a mut dyn OperationCustodyPort,
    /// In-memory historical session owner used by protected recovery.
    pub(in crate::resident) sessions: &'a dyn SessionLeaseCustodyPort,
    /// In-memory independent-anchor owner used by recovery and append custody.
    pub(in crate::resident) anchor: &'a mut dyn AntiRollbackAnchorPort,
}

/// One real Cancel fixture with the retained session and generation needed by
/// the private resident-program boundary.
pub(in crate::resident) struct ResidentProgramCancellationFixture<'a> {
    pub(in crate::resident) cancellation: ProcessHostCancellationFixture<'a>,
    pub(in crate::resident) session: &'a crate::resident::AdmittedAmbientSession,
    pub(in crate::resident) current: &'a crate::resident::AmbientSessionAdmissionContext<'a>,
    pub(in crate::resident) generation: crate::resident::AdmittedCompiledGeneration,
}

type ProcessHostCancellationRunner = dyn for<'a> FnMut(
    ProcessHostCancellationFixture<'a>,
) -> Result<
    ResidentOperationCancellationOutcome,
    ResidentOperationCancellationFailure,
>;

type ResidentProgramCancellationRunner = dyn for<'a> FnMut(ResidentProgramCancellationFixture<'a>);

#[derive(Default)]
struct LocatorRecoveryRunners<'a> {
    process_host_observe: Option<&'a mut ProcessHostObserveRunner>,
    process_host_cancellation: Option<&'a mut ProcessHostCancellationRunner>,
    process_host_restart: Option<&'a mut ProcessHostRestartRunner>,
    process_host_invoke_request: Option<&'a mut ProcessHostInvokeRequestRunner>,
    resident_program_cancellation: Option<&'a mut ResidentProgramCancellationRunner>,
    resident_program_observe: Option<&'a mut ResidentProgramObserveRunner>,
    resident_program_observe_authorities:
        Option<crate::session_admission_fixture::LocalStartExistingAuthorities<'a>>,
}

#[path = "observation_readmission.rs"]
mod observation_readmission;

#[path = "handle_recovery.rs"]
mod handle_recovery;

#[path = "session_basis.rs"]
mod session_basis;

struct SessionRead(SessionLeaseBody, AtomicUsize);
impl SessionLeaseCustodyPort for SessionRead {
    fn compare_and_retain(
        &self,
        _: &SessionLeaseBody,
    ) -> Result<SessionLeaseBodyRetention, ControlStoreError> {
        panic!("locator recovery must not retain or renew a session")
    }
    fn read(
        &self,
        query: &SessionLeaseBodyRead,
    ) -> Result<Option<SessionLeaseBody>, ControlStoreError> {
        self.1.fetch_add(1, Ordering::Relaxed);
        assert_eq!(query.store(), self.0.store());
        assert_eq!(query.digest(), self.0.digest());
        assert!(self.0.canonical_wire_jcs().as_slice().len() <= query.maximum_body_bytes());
        Ok(Some(self.0.clone()))
    }
}

struct ObservationStore<'a>(&'a SigningAuthorityBindingV1);

impl OperationObservationAuthorityStoreResolver for ObservationStore<'_> {
    fn resolve(
        &self,
        observation: &OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        let authority = self.0.integrated_runtime_authority().unwrap();
        assert_eq!(
            observation.deployment_runtime_authority_ref(),
            &authority.deployment_runtime_authority_ref
        );
        assert_eq!(
            observation.deployment_runtime_authority_sha256(),
            &authority.deployment_runtime_authority_sha256
        );
        Ok(store_coordinate())
    }
}

fn replace_observation(
    issued: &crate::resident::IssuedLocalOperation,
    original: &ControlRecord,
    checkpoint: &ControlStateCheckpointV1,
    signing: &SigningAuthorityBindingV1,
    session: &crate::resident::AdmittedAmbientSession,
) -> (
    ControlCheckpointHead,
    ControlRecord,
    ControlStateCheckpointV1,
) {
    let (handle_ref, handle) = issued.handle();
    let first = issue_observation(
        signing,
        handle_ref,
        handle,
        schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
        OperationObservationRevisionV1::new(1).unwrap(),
        None,
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Resolving,
        },
        handle.created_at().get(),
    );
    assert_eq!(
        pale_core::canon::to_canonical_string(&first)
            .unwrap()
            .as_bytes(),
        original.canonical_record_jcs().as_slice()
    );
    append_observation(issued, original, &first, checkpoint, signing, session)
}

fn append_observation(
    issued: &crate::resident::IssuedLocalOperation,
    original: &ControlRecord,
    predecessor: &OperationObservationV1,
    checkpoint: &ControlStateCheckpointV1,
    signing: &SigningAuthorityBindingV1,
    session: &crate::resident::AdmittedAmbientSession,
) -> (
    ControlCheckpointHead,
    ControlRecord,
    ControlStateCheckpointV1,
) {
    let (head, record, checkpoint, _, _) = append_observation_with_state(
        issued,
        original,
        predecessor,
        checkpoint,
        signing,
        session,
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Resolving,
        },
    );
    (head, record, checkpoint)
}

fn append_observation_with_state(
    issued: &crate::resident::IssuedLocalOperation,
    original: &ControlRecord,
    predecessor: &OperationObservationV1,
    checkpoint: &ControlStateCheckpointV1,
    signing: &SigningAuthorityBindingV1,
    _session: &crate::resident::AdmittedAmbientSession,
    state: OperationObservationStateV1,
) -> (
    ControlCheckpointHead,
    ControlRecord,
    ControlStateCheckpointV1,
    ResidentRecordRefV1,
    OperationObservationV1,
) {
    let (handle_ref, handle) = issued.handle();
    let revision = predecessor.revision().get() + 1;
    let sequence = checkpoint.domain_sequence().get() + 1;
    let next = issue_observation(
        signing,
        handle_ref,
        handle,
        schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
        OperationObservationRevisionV1::new(revision).unwrap(),
        Some((original.coordinate().record_ref(), predecessor)),
        state,
        predecessor.observed_at().get() + 1,
    );
    let next_ref = record_ref(&format!("recovery/observation-{revision}"));
    let prior_ref = record_ref(&format!("operation/checkpoint-{}", sequence - 1));
    let append = prepare_operation_observation_append(
        OperationObservationTransition::new(
            handle_ref,
            handle,
            original.coordinate().record_ref(),
            predecessor,
            &next_ref,
            &next,
        ),
        &ObservationStore(signing),
    )
    .unwrap();
    let mut native = super::checkpoint_preparation::PreparationOnly::new(
        |request: &OperationAccumulatorPreparation| {
            let AccumulatorPreparationBasis::Successor(prior) = request.basis() else {
                panic!("actual observation is a successor")
            };
            assert_eq!(request.state().changes().len(), 1);
            assert!(request.unresolved().changes().is_empty());
            Ok(PreparedOperationAccumulator::new(
                request.clone(),
                StateAccumulatorBatch::new(
                    prior.state_root().clone(),
                    artifact_digest(match revision {
                        2 => 'c',
                        3 => 'd',
                        4 => 'e',
                        _ => panic!("fixture supports at most four observation revisions"),
                    }),
                    vec![AccumulatorMutation::new(
                        AccumulatorPosition::Leaf([1; 32]),
                        Some(vec![2; 32]),
                        Some(vec![3; 32]),
                        32,
                    )
                    .unwrap()],
                    1,
                )
                .unwrap(),
                UnresolvedCustodyAccumulatorBatch::new(
                    prior.unresolved_root().clone(),
                    prior.unresolved_root().clone(),
                    vec![],
                    1,
                )
                .unwrap(),
            )
            .unwrap())
        },
        *checkpoint.committed_transaction_sequence(),
    );
    let prepared = append
        .prepare_checkpoint(
            issued.checkpoint_selection(),
            &prior_ref,
            checkpoint,
            AccumulatorPreparationLimits::new(1, 10_000, 10_000, 4096, 100_000).unwrap(),
            &mut native,
        )
        .unwrap();
    let mut next_checkpoint = checkpoint_preimage(
        sequence,
        Some((
            prior_ref.clone(),
            checkpoint.canonical_control_checkpoint_sha256().clone(),
        )),
    );
    next_checkpoint.checkpoint_domain_id = checkpoint.checkpoint_domain_id().clone();
    next_checkpoint.checkpoint_profile_sha256 = checkpoint.checkpoint_profile_sha256().clone();
    next_checkpoint.committed_transaction_sequence = prepared.transaction_sequence();
    next_checkpoint.state_root_sha256 = prepared.state_root().clone();
    next_checkpoint.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
    next_checkpoint.changed_leaf_keys = vec![prepared.observation_key().clone()];
    next_checkpoint.leaf_count = *checkpoint.leaf_count();
    next_checkpoint.issued_at = UnixTimeV1::new(checkpoint.issued_at().get() + 1).unwrap();
    let signing_source = next_checkpoint.clone();
    let signing_calls = Cell::new(0);
    let (prepared, next_checkpoint) = prepared
        .issue_checkpoint(
            &record_ref(&format!("operation/checkpoint-{sequence}")),
            next_checkpoint,
            &signing_source.schema_sha256,
            &checkpoint_authority(&signing_source),
            |domain, actual, digest| {
                assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                assert_eq!(actual, &signing_source);
                assert_eq!(
                    digest,
                    &ControlStateCheckpointDigest::hash_canonical(actual).unwrap()
                );
                signing_calls.set(signing_calls.get() + 1);
                Ok(signature())
            },
        )
        .unwrap();
    let mut custody = RecordingStore::committing(OperationCustodyCommitOutcome::Committed);
    let mut anchor = RecordingAnchor::advancing();
    let pending = prepared.commit(&mut custody, &mut anchor).unwrap();
    let head = pending.candidate().checkpoint().successor().clone();
    let record = pending.candidate().checkpoint().successor_record().clone();
    assert_eq!(
        pale_core::canon::to_canonical_string(&next_checkpoint)
            .unwrap()
            .as_bytes(),
        record.canonical_record_jcs().as_slice()
    );
    let _completed = pending.advance_anchor(&mut anchor).unwrap();
    assert_eq!((custody.commits, anchor.advances), (1, 1));
    assert_eq!(signing_calls.get(), 1);
    (head, record, next_checkpoint, next_ref, next)
}

pub(in crate::resident) struct ReadAnchor {
    replies: std::collections::VecDeque<Option<AntiRollbackAnchor>>,
    reads: usize,
    unavailable: bool,
    trace: Option<RecoveryTrace>,
}

impl ReadAnchor {
    /// Make subsequent recovery reads return an unavailable observation.
    pub(in crate::resident) fn make_unavailable(&mut self) {
        self.unavailable = true;
    }

    /// Replace bounded recovery replies with one newly anchored control head.
    pub(in crate::resident) fn replace_with_control_head(
        &mut self,
        store: &AuthorityStoreCoordinate,
        head: &ControlCheckpointHead,
        replies: usize,
    ) {
        let anchor = AntiRollbackAnchor::ControlDomain(ControlDomainAntiRollbackAnchor::new(
            store.store_id().clone(),
            store.incarnation_id().clone(),
            head.checkpoint_domain_id().clone(),
            head.domain_epoch(),
            head.domain_sequence(),
            head.coordinate().clone(),
        ));
        self.replies = std::iter::repeat_n(Some(anchor), replies).collect();
        self.unavailable = false;
    }
}

impl AntiRollbackAnchorPort for ReadAnchor {
    fn read(
        &mut self,
        _: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        self.reads += 1;
        if let Some(trace) = &self.trace {
            trace.borrow_mut().push("anchor");
        }
        if self.unavailable {
            return Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::Unavailable,
            ));
        }
        Ok(self.replies.pop_front().expect("bounded anchor reads"))
    }
    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        panic!("locator recovery must not advance an anchor")
    }
    fn observe(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        panic!("locator recovery must not fabricate a proposed transition")
    }
}

pub(in crate::resident) struct RecoveryStore {
    handle_lookup: Option<handle_recovery::Lookup>,
    key: OperationCustodyIndexKey,
    request: Option<ControlRecord>,
    handle: Option<ControlRecord>,
    lease: Option<ControlRecord>,
    session_basis: Option<OperationSessionBasis>,
    discovered_session_basis: Option<OperationSessionBasis>,
    head: ControlCheckpointHead,
    checkpoint: ControlRecord,
    initial: OperationAccumulatorPreparation,
    start_observation: Option<ControlRecord>,
    start_checkpoint: Option<ControlRecord>,
    selected_observation: Option<ControlRecord>,
    observation_checkpoint: Option<ControlRecord>,
    observation_history: std::collections::BTreeMap<ResidentRecordRefV1, ControlRecord>,
    checkpoint_history: std::collections::BTreeMap<ResidentRecordRefV1, ControlRecord>,
    missing_operation_record: Option<ControlRecordCoordinate>,
    attempt_selection: Option<ControlRecord>,
    retained_cancellation_request: Option<CommonRequestEnvelopeV1>,
    observation_leaf: Option<AccumulatorMembershipLeaf>,
    collection_snapshot: Option<OperationCollectionSnapshot>,
    trace: Option<RecoveryTrace>,
    index_reads: usize,
    lease_reads: usize,
    observation_body_reads: usize,
    selection_reads: usize,
    head_reads: usize,
    membership_calls: usize,
    start_reads: usize,
    repeat_start_source_once: bool,
    start_through_override: Option<ControlTransactionSequenceV1>,
    change_result_limit: bool,
    change_result_leaf: bool,
    head_changed: bool,
    head_changed_after_memberships: Option<usize>,
    start_changed_after_memberships: Option<usize>,
}

impl RecoveryStore {
    /// Make one completed observation append visible to a fresh recovery read.
    pub(in crate::resident) fn adopt_completed_observation(
        &mut self,
        store: &AuthorityStoreCoordinate,
        candidate: &OperationCustodyCandidate,
        observation_ref: &ResidentRecordRefV1,
        observation: &OperationObservationV1,
        checkpoint: &ControlStateCheckpointV1,
    ) {
        let frozen_start_through = self.head.committed_transaction_sequence();
        // Operation-start discovery freezes the committed-start index, not the
        // successor checkpoint installed below. Preserve the prior prefix while
        // making the newly committed terminal head visible to protected reads.
        self.start_through_override = Some(frozen_start_through);
        if let Some(previous) = self.selected_observation.take() {
            self.observation_history
                .insert(previous.coordinate().record_ref().clone(), previous);
        }
        if let Some(previous) = self.observation_checkpoint.take() {
            self.checkpoint_history
                .insert(previous.coordinate().record_ref().clone(), previous);
        }
        let observation_record = candidate
            .record(OperationCustodyRecordKind::OperationObservation)
            .expect("completed append retains its terminal observation")
            .clone();
        assert_eq!(
            observation_record.coordinate().record_ref(),
            observation_ref
        );
        self.observation_history
            .insert(observation_ref.clone(), observation_record.clone());
        self.checkpoint_history.insert(
            candidate
                .checkpoint()
                .successor_record()
                .coordinate()
                .record_ref()
                .clone(),
            candidate.checkpoint().successor_record().clone(),
        );
        self.head = candidate.checkpoint().successor().clone();
        self.checkpoint = candidate.checkpoint().successor_record().clone();
        self.selected_observation = Some(observation_record);
        self.observation_checkpoint = Some(self.checkpoint.clone());
        self.observation_leaf = Some(
            crate::resident::checkpoint_profile::operation_observation_membership(
                store,
                checkpoint,
                observation_ref,
                observation,
            )
            .expect("completed observation has one admitted membership leaf"),
        );
    }

    /// Fail the first membership check after this many further successful checks.
    pub(in crate::resident) fn change_head_after_memberships(&mut self, successful_calls: usize) {
        self.head_changed_after_memberships = Some(
            self.membership_calls
                .checked_add(successful_calls)
                .expect("membership fault threshold"),
        );
    }

    /// Substitute one start record after this many further membership checks.
    pub(in crate::resident) fn change_start_after_memberships(&mut self, successful_calls: usize) {
        self.start_changed_after_memberships = Some(
            self.membership_calls
                .checked_add(successful_calls)
                .expect("start-record fault threshold"),
        );
    }

    /// Number of exact attempt-selection point reads.
    pub(in crate::resident) const fn selection_reads(&self) -> usize {
        self.selection_reads
    }

    /// Borrow the exact retained attempt selection, when this is a dispatch fixture.
    pub(in crate::resident) const fn attempt_selection(&self) -> Option<&ControlRecord> {
        self.attempt_selection.as_ref()
    }

    /// Replace private selection storage to model a hostile point-read result.
    pub(in crate::resident) fn replace_attempt_selection(&mut self, record: ControlRecord) {
        self.attempt_selection = Some(record);
    }

    /// Borrow the exact request referenced by a retained cancellation head.
    pub(in crate::resident) const fn retained_cancellation_request(
        &self,
    ) -> Option<&CommonRequestEnvelopeV1> {
        self.retained_cancellation_request.as_ref()
    }

    /// Count exact recovery reads that a continuation can compare.
    pub(in crate::resident) const fn access_counts(&self) -> (usize, usize, usize, usize) {
        (
            self.index_reads,
            self.observation_body_reads,
            self.head_reads,
            self.membership_calls,
        )
    }

    /// Number of bounded committed-start discovery reads.
    pub(in crate::resident) const fn start_reads(&self) -> usize {
        self.start_reads
    }

    /// Repeat the admitted start once to exercise a two-row frozen prefix.
    pub(in crate::resident) fn repeat_start_source_once(&mut self) {
        self.repeat_start_source_once = true;
    }

    /// Remove the request body selected by the committed start source.
    pub(in crate::resident) fn remove_discovered_request(&mut self) {
        self.request = None;
    }

    /// Remove the operation lease needed after exact start admission.
    pub(in crate::resident) fn remove_operation_lease(&mut self) {
        self.lease = None;
    }

    /// Substitute only the enumerated private basis, not the exact point-read basis.
    pub(in crate::resident) fn substitute_discovered_session_basis(&mut self) {
        let original = self
            .session_basis
            .as_ref()
            .expect("restart fixture retains its session basis");
        self.discovered_session_basis = Some(
            OperationSessionBasis::new(
                original.handle().clone(),
                original.request().clone(),
                SessionAdmissionLeaseDigest::parse(sha('f')).unwrap(),
                original.canonical_basis_jcs().to_vec(),
                MAX_OPERATION_SESSION_BASIS_BYTES,
            )
            .unwrap(),
        );
    }

    /// Return the current transaction fence for one modeled successor append.
    pub(in crate::resident) fn current_transaction_sequence(&self) -> ControlTransactionSequenceV1 {
        self.head.committed_transaction_sequence()
    }

    /// Remove the latest-observation index after semantic recovery has completed.
    pub(in crate::resident) fn remove_current_observation(&mut self) {
        self.selected_observation = None;
    }

    /// Substitute an older admitted checkpoint after semantic recovery.
    pub(in crate::resident) fn substitute_current_checkpoint(&mut self) {
        self.checkpoint = self
            .start_checkpoint
            .clone()
            .expect("fixture retains its initial checkpoint");
    }
}

impl pale_ports::ControlTransactionPort for RecoveryStore {
    fn prepare_transaction(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, pale_ports::ControlStoreError> {
        panic!("this physical-custody/readback fixture does not prepare a new checkpoint")
    }
}

impl OperationCustodyPort for RecoveryStore {
    fn read_operation_starts(
        &mut self,
        store: &AuthorityStoreCoordinate,
        cursor: Option<&OperationStartCursor>,
        maximum_sources: usize,
        maximum_session_basis_bytes: usize,
    ) -> Result<OperationStartSources, OperationStartReadError> {
        self.start_reads += 1;
        assert_eq!(store, &store_coordinate());
        assert_eq!(maximum_sources, 1);
        let basis = self
            .discovered_session_basis
            .as_ref()
            .or(self.session_basis.as_ref())
            .expect("restart fixture retains its session basis");
        assert!(basis.canonical_basis_jcs().len() <= maximum_session_basis_bytes);
        let through = self
            .start_through_override
            .unwrap_or_else(|| self.head.committed_transaction_sequence());
        let first_transaction = self.initial.transaction().successor();
        let second_transaction = ControlTransactionSequenceV1::new(first_transaction.get() + 1)
            .expect("the repeated source uses the next transaction sequence");
        let (start_transaction, continuation) = match cursor {
            None => {
                let continuation = self.repeat_start_source_once.then(|| {
                    OperationStartCursor::new(
                        store.clone(),
                        through,
                        first_transaction,
                        basis.handle().clone(),
                        basis.request().clone(),
                        basis.session_lease_digest().clone(),
                        basis.basis_sha256().clone(),
                    )
                    .expect("the repeated fixture cursor remains inside its frozen prefix")
                });
                (first_transaction, continuation)
            }
            Some(cursor) if self.repeat_start_source_once => {
                assert_eq!(cursor.store(), store);
                assert_eq!(cursor.through(), through);
                assert_eq!(cursor.after_transaction(), first_transaction);
                assert_eq!(cursor.handle(), basis.handle());
                assert_eq!(cursor.request(), basis.request());
                assert_eq!(cursor.session_lease_digest(), basis.session_lease_digest());
                assert_eq!(cursor.session_basis_digest(), basis.basis_sha256());
                (second_transaction, None)
            }
            Some(_) => panic!("one-source fixture exhausts its prefix"),
        };
        assert!(
            start_transaction.get() <= through.get(),
            "the repeated fixture source remains inside its frozen prefix"
        );
        Ok(OperationStartSources::new(
            through,
            vec![OperationStartSource::new(start_transaction, basis.clone())],
            continuation,
        ))
    }

    fn read_collection_snapshot(
        &mut self,
        store: &AuthorityStoreCoordinate,
        handle: &ControlRecordCoordinate,
        maximum_frontier_bytes: usize,
        maximum_journal_payload_bytes: usize,
    ) -> Result<Option<OperationCollectionSnapshot>, ControlStoreError> {
        assert_eq!(store, &store_coordinate());
        if let Some(snapshot) = &self.collection_snapshot {
            assert_eq!(handle, snapshot.evidence().operation_handle());
            assert!(
                snapshot.frontier().canonical_record_jcs().as_slice().len()
                    <= maximum_frontier_bytes
            );
            assert!(snapshot.journal_payload().as_slice().len() <= maximum_journal_payload_bytes);
        }
        Ok(self.collection_snapshot.clone())
    }

    fn read_a2a_task_bindings(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_ports::OperationA2aBindingScope,
        _: Option<&pale_ports::OperationA2aBindingCursor>,
        _: usize,
        _: usize,
    ) -> Result<pale_ports::OperationA2aBindings, pale_ports::ControlStoreError> {
        panic!("remote-task discovery is outside this fixture's tested journey")
    }

    fn read_operation_session_basis(
        &mut self,
        store: &AuthorityStoreCoordinate,
        handle: &ControlRecordCoordinate,
        maximum: usize,
    ) -> Result<Option<OperationSessionBasis>, ControlStoreError> {
        assert_eq!(store, &store_coordinate());
        assert_eq!(handle, self.handle.as_ref().unwrap().coordinate());
        if let Some(basis) = &self.session_basis {
            assert!(basis.canonical_basis_jcs().len() <= maximum);
        }
        Ok(self.session_basis.clone())
    }
    fn read_publication_sources(
        &mut self,
        _store: &pale_ports::AuthorityStoreCoordinate,
        _cursor: Option<&pale_ports::OperationPublicationCursor>,
        _maximum_sources: usize,
        _maximum_body_bytes: usize,
    ) -> Result<pale_ports::OperationPublicationSources, pale_ports::OperationPublicationReadError>
    {
        panic!("publication discovery is outside this fixture's tested journey")
    }

    fn read_checkpoint_head(
        &mut self,
        store: &AuthorityStoreCoordinate,
        domain: &ControlCheckpointDomainIdV1,
        maximum: usize,
    ) -> Result<Option<(ControlCheckpointHead, ControlRecord)>, ControlStoreError> {
        self.head_reads += 1;
        assert_eq!(store, &store_coordinate());
        assert_eq!(domain, self.head.checkpoint_domain_id());
        assert!(self.checkpoint.canonical_record_jcs().as_slice().len() <= maximum);
        Ok(Some((self.head.clone(), self.checkpoint.clone())))
    }
    fn verify_accumulator_membership(
        &mut self,
        request: &OperationAccumulatorMembership,
    ) -> Result<VerifiedOperationAccumulatorMembership, OperationAccumulatorPreparationError> {
        self.membership_calls += 1;
        if let Some(trace) = &self.trace {
            trace.borrow_mut().push("membership");
        }
        assert_eq!(request.checkpoint().checkpoint(), &self.head);
        assert!(matches!(request.leaves().len(), 1 | 3));
        // Compare recovery's identity and basis against the four actual logical changes
        // captured before the original checkpoint was issued, not a second
        // invocation of the new recovery projection.
        for leaf in request.leaves() {
            let initial = self.initial.state().changes().iter().any(|change| {
                change.key() == leaf.key() && change.successor() == Some(leaf.value())
            });
            if !initial && self.observation_leaf.as_ref() != Some(leaf) {
                return Err(ControlStoreError::new(ControlStoreErrorKind::RecoveryRequired).into());
            }
        }
        if self.head_changed
            || self
                .head_changed_after_memberships
                .is_some_and(|threshold| self.membership_calls > threshold)
        {
            return Err(ControlStoreError::new(ControlStoreErrorKind::CandidateConflict).into());
        }
        let returned = if self.change_result_limit {
            OperationAccumulatorMembership::new(
                request.checkpoint().clone(),
                request.leaves().to_vec(),
                AccumulatorMembershipLimits::new(
                    request.limits().maximum_leaves() + 1,
                    request.limits().maximum_reads(),
                    request.limits().maximum_proof_bytes(),
                )
                .unwrap(),
                request.implementation(),
            )
            .unwrap()
        } else if self.change_result_leaf {
            let mut leaves = request.leaves().to_vec();
            leaves[0] =
                AccumulatorMembershipLeaf::new(leaves[0].key().clone(), artifact_digest('e'))
                    .unwrap();
            OperationAccumulatorMembership::new(
                request.checkpoint().clone(),
                leaves,
                request.limits(),
                request.implementation(),
            )
            .unwrap()
        } else {
            request.clone()
        };
        Ok(VerifiedOperationAccumulatorMembership::new(returned))
    }
    fn prepare_accumulator(
        &mut self,
        _: &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
        panic!("read-only recovery must not prepare an update")
    }
    fn compare_and_retain_operation_lease(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlRecord,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        panic!("read-only recovery must not retain a lease")
    }
    fn read_operation_record(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
        maximum: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError> {
        if self
            .discovered_session_basis
            .as_ref()
            .or(self.session_basis.as_ref())
            .is_some_and(|basis| coordinate == basis.request())
        {
            assert_eq!(store, &store_coordinate());
            if let Some(record) = &self.request {
                assert!(record.canonical_record_jcs().as_slice().len() <= maximum);
            }
            return Ok(self.request.clone());
        }
        if let Some(record) = self
            .request
            .as_ref()
            .filter(|record| record.coordinate() == coordinate)
        {
            assert_eq!(store, &store_coordinate());
            assert!(record.canonical_record_jcs().as_slice().len() <= maximum);
            return Ok(Some(record.clone()));
        }
        if let Some(record) = self
            .attempt_selection
            .as_ref()
            .filter(|record| record.coordinate() == coordinate)
        {
            self.selection_reads += 1;
            assert_eq!(store, &store_coordinate());
            assert!(record.canonical_record_jcs().as_slice().len() <= maximum);
            return Ok(Some(record.clone()));
        }
        if let Some(lookup) = &mut self.handle_lookup {
            if coordinate == &lookup.expected {
                assert_eq!(store, &store_coordinate());
                lookup.handle_reads += 1;
                assert_eq!(maximum, lookup.maximum);
                return Ok(lookup.handle.clone());
            }
        }
        if let Some(record) = self
            .observation_history
            .get(coordinate.record_ref())
            .filter(|record| record.coordinate() == coordinate)
        {
            self.observation_body_reads += 1;
            assert!(record.canonical_record_jcs().as_slice().len() <= maximum);
            return Ok(Some(record.clone()));
        }
        if let Some(record) = self
            .checkpoint_history
            .get(coordinate.record_ref())
            .filter(|record| record.coordinate() == coordinate)
        {
            assert!(record.canonical_record_jcs().as_slice().len() <= maximum);
            return Ok(Some(record.clone()));
        }
        if self.missing_operation_record.as_ref() == Some(coordinate) {
            return Ok(None);
        }
        self.lease_reads += 1;
        assert_eq!(store, &store_coordinate());
        if let Some(lease) = &self.lease {
            assert_eq!(coordinate, lease.coordinate());
            assert!(lease.canonical_record_jcs().as_slice().len() <= maximum);
        }
        Ok(self.lease.clone())
    }
    fn compare_and_commit(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        panic!("recovery must not commit")
    }
    fn observe_exact(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
        panic!("recovery must not replay an old candidate")
    }
    fn read_index(
        &mut self,
        store: &AuthorityStoreCoordinate,
        query: &OperationCustodyIndexQuery,
        maximum: usize,
    ) -> Result<OperationCustodyIndexObservation, ControlStoreError> {
        if let OperationCustodyIndexQuery::RequestForHandle(key) = query {
            assert_eq!(store, &store_coordinate());
            let lookup = self.handle_lookup.as_mut().expect("handle lookup selected");
            assert_eq!(key, &lookup.key);
            assert_eq!(maximum, lookup.maximum);
            lookup.index_reads += 1;
            let returned_query = if lookup.wrong_query {
                OperationCustodyIndexQuery::RequestForHandle(
                    OperationCustodyIndexKey::parse(sha('f')).unwrap(),
                )
            } else {
                query.clone()
            };
            return Ok(match &lookup.request {
                Some(record) => {
                    OperationCustodyIndexObservation::exact(returned_query, record.clone()).unwrap()
                }
                None => OperationCustodyIndexObservation::absent(returned_query),
            });
        }
        self.index_reads += 1;
        assert_eq!(store, &store_coordinate());
        let record = match query {
            OperationCustodyIndexQuery::RequestEvidence(key) => {
                assert_eq!(key, &self.key);
                self.request.clone()
            }
            OperationCustodyIndexQuery::RequestOperation(key) => {
                assert_eq!(key, &self.key);
                self.handle.clone()
            }
            OperationCustodyIndexQuery::OperationHandle(_) => self.handle.clone(),
            OperationCustodyIndexQuery::StartObservation(key) => {
                assert_eq!(key, &self.key);
                self.start_observation.as_ref().map(|record| {
                    if self
                        .start_changed_after_memberships
                        .is_some_and(|threshold| self.membership_calls > threshold)
                    {
                        without_signature(record)
                    } else {
                        record.clone()
                    }
                })
            }
            OperationCustodyIndexQuery::LatestObservation(_) => self.selected_observation.clone(),
            OperationCustodyIndexQuery::ObservationCheckpoint(key) => {
                let start = self.start_observation.as_ref().map(|observation| {
                    OperationCustodyIndexKey::for_observation(observation.coordinate())
                });
                if start.as_ref() == Some(key) {
                    self.start_checkpoint.clone()
                } else {
                    self.observation_checkpoint.clone()
                }
            }
            _ => panic!("operation recovery requested an unselected index role"),
        };
        if let Some(record) = record {
            assert!(record.canonical_record_jcs().as_slice().len() <= maximum);
            Ok(OperationCustodyIndexObservation::exact(query.clone(), record).unwrap())
        } else {
            Ok(OperationCustodyIndexObservation::absent(query.clone()))
        }
    }
}

pub(in crate::resident) struct RecoveryAppendStore<'a> {
    pub(in crate::resident) recovery: &'a mut RecoveryStore,
    pub(in crate::resident) append: &'a mut RecordingStore,
}

impl pale_ports::ControlTransactionPort for RecoveryAppendStore<'_> {
    fn prepare_transaction(
        &mut self,
        store: &AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, ControlStoreError> {
        self.append.prepare_transaction(store)
    }
}

impl OperationCustodyPort for RecoveryAppendStore<'_> {
    fn read_collection_snapshot(
        &mut self,
        store: &AuthorityStoreCoordinate,
        handle: &ControlRecordCoordinate,
        maximum_frontier_bytes: usize,
        maximum_journal_payload_bytes: usize,
    ) -> Result<Option<OperationCollectionSnapshot>, ControlStoreError> {
        self.recovery.read_collection_snapshot(
            store,
            handle,
            maximum_frontier_bytes,
            maximum_journal_payload_bytes,
        )
    }

    fn read_a2a_task_bindings(
        &mut self,
        store: &AuthorityStoreCoordinate,
        scope: &pale_ports::OperationA2aBindingScope,
        cursor: Option<&pale_ports::OperationA2aBindingCursor>,
        maximum_bindings: usize,
        maximum_body_bytes: usize,
    ) -> Result<pale_ports::OperationA2aBindings, ControlStoreError> {
        self.recovery.read_a2a_task_bindings(
            store,
            scope,
            cursor,
            maximum_bindings,
            maximum_body_bytes,
        )
    }

    fn read_operation_session_basis(
        &mut self,
        store: &AuthorityStoreCoordinate,
        handle: &ControlRecordCoordinate,
        maximum: usize,
    ) -> Result<Option<OperationSessionBasis>, ControlStoreError> {
        self.recovery
            .read_operation_session_basis(store, handle, maximum)
    }

    fn read_publication_sources(
        &mut self,
        store: &AuthorityStoreCoordinate,
        cursor: Option<&pale_ports::OperationPublicationCursor>,
        maximum_sources: usize,
        maximum_body_bytes: usize,
    ) -> Result<pale_ports::OperationPublicationSources, pale_ports::OperationPublicationReadError>
    {
        self.recovery
            .read_publication_sources(store, cursor, maximum_sources, maximum_body_bytes)
    }

    fn read_checkpoint_head(
        &mut self,
        store: &AuthorityStoreCoordinate,
        domain: &ControlCheckpointDomainIdV1,
        maximum: usize,
    ) -> Result<Option<(ControlCheckpointHead, ControlRecord)>, ControlStoreError> {
        self.recovery.read_checkpoint_head(store, domain, maximum)
    }

    fn verify_accumulator_membership(
        &mut self,
        request: &OperationAccumulatorMembership,
    ) -> Result<VerifiedOperationAccumulatorMembership, OperationAccumulatorPreparationError> {
        self.recovery.verify_accumulator_membership(request)
    }

    fn prepare_accumulator(
        &mut self,
        request: &OperationAccumulatorPreparation,
    ) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
        self.append.prepare_accumulator(request)
    }

    fn compare_and_retain_operation_lease(
        &mut self,
        store: &AuthorityStoreCoordinate,
        lease: &ControlRecord,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        self.recovery
            .compare_and_retain_operation_lease(store, lease)
    }

    fn read_operation_record(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
        maximum: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError> {
        self.recovery
            .read_operation_record(store, coordinate, maximum)
    }

    fn compare_and_commit(
        &mut self,
        candidate: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        self.append.compare_and_commit(candidate)
    }

    fn observe_exact(
        &mut self,
        candidate: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
        self.append.observe_exact(candidate)
    }

    fn read_index(
        &mut self,
        store: &AuthorityStoreCoordinate,
        query: &OperationCustodyIndexQuery,
        maximum: usize,
    ) -> Result<OperationCustodyIndexObservation, ControlStoreError> {
        self.recovery.read_index(store, query, maximum)
    }
}

pub(in crate::resident) struct SequencedAppendAnchor {
    advances: VecDeque<Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError>>,
    observations: VecDeque<Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError>>,
    advance_count: usize,
    observation_count: usize,
}

impl SequencedAppendAnchor {
    pub(in crate::resident) fn advancing() -> Self {
        Self {
            advances: VecDeque::from([Ok(AntiRollbackAnchorAdvanceDisposition::Advanced)]),
            observations: VecDeque::from([Ok(AntiRollbackAnchorObservation::ExactPredecessor)]),
            advance_count: 0,
            observation_count: 0,
        }
    }

    pub(in crate::resident) fn unavailable() -> Self {
        Self {
            advances: VecDeque::from([Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::Unavailable,
            ))]),
            observations: VecDeque::from([Ok(AntiRollbackAnchorObservation::ExactPredecessor)]),
            advance_count: 0,
            observation_count: 0,
        }
    }

    pub(in crate::resident) fn unavailable_then_advancing() -> Self {
        Self {
            advances: VecDeque::from([
                Err(AntiRollbackAnchorError::new(
                    AntiRollbackAnchorErrorKind::Unavailable,
                )),
                Ok(AntiRollbackAnchorAdvanceDisposition::Advanced),
            ]),
            observations: VecDeque::from([Ok(AntiRollbackAnchorObservation::ExactPredecessor)]),
            advance_count: 0,
            observation_count: 0,
        }
    }

    pub(in crate::resident) fn advancing_after_database_recovery() -> Self {
        Self {
            advances: VecDeque::from([Ok(AntiRollbackAnchorAdvanceDisposition::Advanced)]),
            observations: VecDeque::from([
                Ok(AntiRollbackAnchorObservation::ExactPredecessor),
                Ok(AntiRollbackAnchorObservation::ExactPredecessor),
            ]),
            advance_count: 0,
            observation_count: 0,
        }
    }

    /// Admit one independently observed and advanced successor transition.
    pub(in crate::resident) fn with_advancing_follow_up(mut self) -> Self {
        self.advances
            .push_back(Ok(AntiRollbackAnchorAdvanceDisposition::Advanced));
        self.observations
            .push_back(Ok(AntiRollbackAnchorObservation::ExactPredecessor));
        self
    }

    pub(in crate::resident) const fn access_counts(&self) -> (usize, usize) {
        (self.advance_count, self.observation_count)
    }
}

impl AntiRollbackAnchorPort for SequencedAppendAnchor {
    fn read(
        &mut self,
        _: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        panic!("append-only anchor fixture must not serve recovery reads")
    }

    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        self.advance_count += 1;
        self.advances
            .pop_front()
            .expect("one configured anchor-advance outcome per attempt")
    }

    fn observe(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        self.observation_count += 1;
        self.observations
            .pop_front()
            .expect("one configured anchor observation per attempt")
    }
}

pub(in crate::resident) struct RecoveryAppendAnchor<'a, A> {
    pub(in crate::resident) recovery: &'a mut ReadAnchor,
    pub(in crate::resident) append: &'a mut A,
}

impl<A: AntiRollbackAnchorPort> AntiRollbackAnchorPort for RecoveryAppendAnchor<'_, A> {
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        self.recovery.read(key)
    }

    fn compare_and_advance(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        self.append.compare_and_advance(transition)
    }

    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        self.append.observe(transition)
    }
}

pub(in crate::resident) fn cancellation_accumulator_response(
    request: &OperationAccumulatorPreparation,
) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
    let AccumulatorPreparationBasis::Successor(prior) = request.basis() else {
        panic!("cancellation appends to an admitted checkpoint")
    };
    assert_eq!(request.state().changes().len(), 1);
    assert!(request.unresolved().changes().is_empty());
    Ok(PreparedOperationAccumulator::new(
        request.clone(),
        StateAccumulatorBatch::new(
            prior.state_root().clone(),
            artifact_digest('9'),
            vec![AccumulatorMutation::new(
                AccumulatorPosition::Leaf([9; 32]),
                Some(vec![8; 32]),
                Some(vec![9; 32]),
                32,
            )
            .unwrap()],
            1,
        )
        .unwrap(),
        UnresolvedCustodyAccumulatorBatch::new(
            prior.unresolved_root().clone(),
            prior.unresolved_root().clone(),
            vec![],
            1,
        )
        .unwrap(),
    )
    .unwrap())
}

fn reject_cancellation_accumulator_preparation(
    _: &OperationAccumulatorPreparation,
) -> Result<PreparedOperationAccumulator, OperationAccumulatorPreparationError> {
    Err(OperationAccumulatorPreparationError::Limit)
}

fn parents(value: &RecoveryParents<'_>) -> (SessionRead, RecoveryResolver, ControlRecord) {
    let session = value.session.lease();
    let body = SessionLeaseBody::new(
        store_coordinate(),
        session.session_admission_lease_digest().clone(),
        session.schema_id().clone(),
        session.schema_sha256().clone(),
        ControlRecordBytes::new(
            pale_core::canon::to_canonical_string(&SessionAdmissionLeaseWireV1::from(session))
                .unwrap()
                .into_bytes(),
        )
        .unwrap(),
    );
    let mut resolver = RecoveryResolver::default();
    resolver.insert_generation(value.current.generation.compiled_generation);
    resolver.insert_active(value.current.generation.active_head);
    resolver.insert_fence(value.current.generation.activation_fence);
    resolver.insert_effective_profile(value.current.generation.effective_profile);
    let lease = crate::resident::record_projection::record(
        crate::resident::record_projection::coordinate(
            &record_ref("start-lease"),
            value.lease.operation_lease_digest().as_str(),
        )
        .unwrap(),
        value.lease,
    )
    .unwrap();
    (SessionRead(body, AtomicUsize::new(0)), resolver, lease)
}

fn observation_request(
    original: &CommonRequestEnvelopeV1,
    handle: &OperationHandleV1,
) -> CommonRequestEnvelopeV1 {
    let issued_at = handle.created_at().get() + 3;
    let mut preimage = original.to_preimage();
    preimage.request_id =
        ResidentRequestIdV1::parse("control-observe-application").expect("request id");
    preimage.correlation_id =
        ResidentCorrelationIdV1::parse("control-observe-application").expect("correlation id");
    preimage.capability_id =
        CapabilityIdV1::parse(OPERATION_OBSERVE_CAPABILITY_ID).expect("capability");
    preimage.idempotency_key =
        RequestIdempotencyKeyV1::parse("control-observe-application").expect("idempotency key");
    preimage.issued_at = UnixTimeV1::new(issued_at).expect("issue time");
    preimage.expires_at = *handle.retain_until();
    preimage.generation = Some(handle.compiled_generation_sha256().clone());
    preimage.fence_token = Some(handle.activation_fence_sha256().clone());
    preimage.body.schema_id =
        SchemaId::parse(OPERATION_HANDLE_V1_SCHEMA).expect("operation handle schema");
    preimage.body.schema_sha256 = handle.schema_sha256().clone();
    preimage.body.payload = serde_json::to_value(handle).expect("operation handle payload");
    preimage.body.payload_sha256 = ArtifactDigest::hash_bytes(
        pale_core::canon::to_canonical_string(&preimage.body.payload)
            .expect("canonical handle payload")
            .as_bytes(),
    );
    crate::effect_case_lineage_fixture::request(&preimage)
}

fn cancellation_request(
    original: &CommonRequestEnvelopeV1,
    handle: &OperationHandleV1,
) -> CommonRequestEnvelopeV1 {
    let mut preimage = observation_request(original, handle).to_preimage();
    preimage.request_id =
        ResidentRequestIdV1::parse("control-cancel-application").expect("request id");
    preimage.correlation_id =
        ResidentCorrelationIdV1::parse("control-cancel-application").expect("correlation id");
    preimage.capability_id =
        CapabilityIdV1::parse(OPERATION_CANCEL_CAPABILITY_ID).expect("capability");
    preimage.idempotency_key =
        RequestIdempotencyKeyV1::parse("control-cancel-application").expect("idempotency key");
    crate::effect_case_lineage_fixture::request(&preimage)
}

fn resident_program_cancellation_request(
    original: &CommonRequestEnvelopeV1,
    handle: &OperationHandleV1,
    session: &crate::resident::AdmittedAmbientSession,
    current: &crate::resident::AmbientSessionAdmissionContext<'_>,
) -> CommonRequestEnvelopeV1 {
    resident_program_current_request(cancellation_request(original, handle), session, current)
}

fn resident_program_observation_request(
    original: &CommonRequestEnvelopeV1,
    handle: &OperationHandleV1,
    session: &crate::resident::AdmittedAmbientSession,
    current: &crate::resident::AmbientSessionAdmissionContext<'_>,
) -> CommonRequestEnvelopeV1 {
    resident_program_current_request(observation_request(original, handle), session, current)
}

fn resident_program_current_request(
    request: CommonRequestEnvelopeV1,
    session: &crate::resident::AdmittedAmbientSession,
    current: &crate::resident::AmbientSessionAdmissionContext<'_>,
) -> CommonRequestEnvelopeV1 {
    let mut preimage = request.to_preimage();
    let checked_at = UnixTimeV1::new(preimage.issued_at.get() + 1).expect("check time");
    let grant = current.grant.expect("resident Program fixture grant").grant;
    let capability_revocations = current
        .capability_revocations
        .expect("resident Program capability-revocation frontier");
    preimage.session_admission = CommonRequestSessionAdmissionV1 {
        session_admission_id: session.lease().session_admission_id().clone(),
        session_admission_lease_digest: session.lease().session_admission_lease_digest().clone(),
        session_grant_id: session.grant_id().clone(),
        session_grant_sha256: ArtifactDigest::parse(session.grant_sha256().as_str())
            .expect("session grant artifact digest"),
        deployment_binding_ref: session.lease().deployment_binding_ref().clone(),
        deployment_binding_sha256: session.lease().deployment_binding_sha256().clone(),
        session_floor_epoch: *grant.session_floor_epoch(),
        grant_issuer_principal_id: grant.issuer_principal_id().clone(),
        grant_issuer_key_id: grant.issuer_key_id().clone(),
        grant_issuer_key_epoch: PrincipalKeyEpochV1::new(grant.issuer_key_epoch().get())
            .expect("grant issuer key epoch"),
        checked_revocation_epoch: current.session_revocations.epoch,
        checked_revocation_ref: current.session_revocations.revocation_ref.cloned(),
        checked_revocation_sha256: current.session_revocations.revocation_sha256.cloned(),
        checked_revocation_checkpoint_ref: capability_revocations.checkpoint_ref.clone(),
        checked_revocation_checkpoint_sha256: ArtifactDigest::parse(
            capability_revocations
                .checkpoint
                .canonical_checkpoint_sha256()
                .as_str(),
        )
        .expect("capability checkpoint artifact digest"),
        checked_at,
    };
    preimage.generation = Some(session.lease().compiled_generation_digest().clone());
    preimage.expires_at = UnixTimeV1::new(checked_at.get() + 10).expect("expiry time");
    crate::effect_case_lineage_fixture::request(&preimage)
}

fn retained_pre_call_cancellation_request(
    original: &CommonRequestEnvelopeV1,
    handle: &OperationHandleV1,
) -> CommonRequestEnvelopeV1 {
    let mut preimage = cancellation_request(original, handle).to_preimage();
    preimage.issued_at = UnixTimeV1::new(handle.created_at().get() + 1).expect("issue time");
    crate::effect_case_lineage_fixture::request(&preimage)
}

fn request_admission_context(
    request: &pale_core::records::CommonRequestEnvelopePreimageV1,
) -> pale_core::records::CommonRequestAdmissionContextV1<'_> {
    pale_core::records::CommonRequestAdmissionContextV1 {
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

#[test]
fn locator_recovery_after_real_start_phases_is_read_only_and_rejects_authority_and_races() {
    locator_recovery_cases(
        &[
            "none",
            "after-observation",
            "zero-bound",
            "corrupt-handle",
            "corrupt-checkpoint",
            "request-conflict",
            "profile-ref",
            "missing-request",
            "missing-handle",
            "missing-lease",
            "missing-parent",
            "handle-signature",
            "checkpoint-signature",
            "historical-authority",
            "initial-observation-substituted",
            "initial-checkpoint-substituted",
            "initial-observation-signer",
            "initial-observation-signature",
            "initial-observation-rotation",
            "initial-observation-revision-bound",
            "initial-checkpoint-bound",
            "anchor-missing",
            "anchor-unavailable",
            "anchor-wrong-key",
            "anchor-before",
            "anchor-after",
            "head-before-proof",
            "foreign-limit",
            "foreign-leaf",
        ],
        false,
        None,
    );
}

#[test]
fn saved_handle_recovery_joins_exact_request_and_handle_without_renewal() {
    locator_recovery_cases(handle_recovery::CASES, true, None);
}

#[test]
fn inherited_child_store_and_incarnation_reject_before_any_owner_io() {
    use crate::resident::{
        prepare_local_child_parent, recover_local_child_operation,
        LocalChildOperationRecoveryContext, LocalOperationStartError,
        OperationCheckpointAncestryLimits, OperationObservationRecoveryAdmission,
        OperationObservationRecoveryInput, OperationObservationRecoveryLimits,
        OperationObservationSelection, SupportedOperationCheckpointProfile,
    };
    let (generation, profile) = source_generation();
    with_start(
        &generation,
        &profile,
        "foreign-child-store",
        |_| {},
        StartAdmission::AcceptedOwnerModel,
        |prepared, request, _, _, retained| {
            let prepared = super::local_start::prepared_initial(prepared.unwrap());
            let (physical, pending, checkpoint) =
                super::local_start::issue_initial(prepared, request, |_| {}, &Cell::new(0))
                    .unwrap();
            let mut custody = super::local_start::absent_start_store(request);
            let mut initial_anchor = super::local_start::initial_anchor();
            let database = physical
                .commit(&mut custody, &mut initial_anchor, 1_048_576)
                .unwrap();
            let operation = pending
                .finish(database.advance_anchor(&mut initial_anchor).unwrap())
                .unwrap_or_else(|_| panic!("exact parent custody fixture"));
            let (session, historical, _) = parents(&retained);
            let supported = crate::local_operation_start_fixture::profile();
            let mut signer = checkpoint_preimage(1, None);
            signer.checkpoint_domain_id = checkpoint.checkpoint_domain_id().clone();
            signer.checkpoint_profile_sha256 =
                SupportedOperationCheckpointProfile::content_digest().unwrap();
            let authority = checkpoint_authority(&signer);
            // Keep the authority correct for the inherited store so the old
            // authority-only check cannot accidentally make this test pass.
            assert!(operation
                .checkpoint_selection()
                .matches_authority(&authority));
            for foreign in [
                AuthorityStoreCoordinate::new(
                    ControlStoreIdV1::parse("foreign-store").unwrap(),
                    store_coordinate().incarnation_id().clone(),
                ),
                AuthorityStoreCoordinate::new(
                    store_coordinate().store_id().clone(),
                    ControlIncarnationIdV1::parse("foreign-incarnation").unwrap(),
                ),
            ] {
                // Existing fixtures panic on transaction preparation and on any
                // read with no supplied reply. No special permissive test port.
                let mut port = RecordingStore::recovering(vec![]);
                let mut anchor = ReadAnchor {
                    replies: Default::default(),
                    reads: 0,
                    unavailable: false,
                    trace: None,
                };
                let recovered = recover_local_child_operation(
                    LocalChildOperationRecoveryContext {
                        context: LocalOperationRecoveryContext {
                            store: &foreign,
                            profile: &supported,
                            handle_signing_authority: retained.signing,
                            checkpoint_authority: &authority,
                            historical: &historical,
                            maximum_record_bytes: 262_144,
                            maximum_operation_lease_bytes: 262_144,
                            maximum_session_lease_bytes: 262_144,
                            membership_limits: AccumulatorMembershipLimits::new(3, 1024, 1_048_576)
                                .unwrap(),
                            observation_limits: OperationObservationRecoveryLimits::new(
                                1, 262_144, 262_144,
                            )
                            .unwrap(),
                            checkpoint_limits: OperationCheckpointAncestryLimits::new(
                                1, 262_144, 1,
                            )
                            .unwrap(),
                        },
                        parent: &operation,
                        parent_lease: retained.lease,
                    },
                    request,
                    &mut port,
                    &session,
                    &mut anchor,
                    LocalOperationRecoveryAdmission {
                        admit_historical_authority:
                            |_: &OperationHandlePreimageV1, _: Option<&OperationHandleV1>| {
                                panic!("foreign store cannot invoke historical authority")
                            },
                        verify_handle_signature:
                            |_: &[u8],
                             _: &OperationHandlePreimageV1,
                             _: &OperationHandleDigest,
                             _: &CanonicalSignatureV1| {
                                panic!("foreign store cannot verify a handle")
                            },
                        verify_checkpoint_signature:
                            |_: &[u8],
                             _: &ControlStateCheckpointPreimageV1,
                             _: &ControlStateCheckpointDigest,
                             _: &CanonicalSignatureV1| {
                                panic!("foreign store cannot verify a checkpoint")
                            },
                        initial_observation: OperationObservationRecoveryAdmission {
                            admit_checkpoint: |_: &ResidentRecordRefV1,
                                               _: ControlStateCheckpointWireV1|
                             -> Result<
                                ControlStateCheckpointV1,
                                InvalidResidentRecordV1,
                            > {
                                panic!("foreign store cannot admit a historical checkpoint")
                            },
                            resolve_signer: |_: &ResidentRecordRefV1,
                                             _: &OperationObservationWireV1|
                             -> Result<
                                SigningAuthorityBindingV1,
                                InvalidResidentRecordV1,
                            > {
                                panic!("foreign store cannot resolve an observation signer")
                            },
                            admit_state: |_: &OperationObservationStateV1|
                             -> Result<(), InvalidResidentRecordV1> {
                                panic!("foreign store cannot admit observation state")
                            },
                            admit_rotation: |_: &ResidentRecordRefV1,
                                             _: &ArtifactDigest,
                                             _: &ResidentRecordRefV1,
                                             _: &ArtifactDigest|
                             -> Result<(), InvalidResidentRecordV1> {
                                panic!("foreign store cannot admit authority rotation")
                            },
                            verify_signature: |_: &[u8],
                                               _: &OperationObservationPreimageV1,
                                               _: &OperationObservationDigest,
                                               _: &CanonicalSignatureV1|
                             -> Result<(), InvalidResidentRecordV1> {
                                panic!("foreign store cannot verify an observation")
                            },
                        },
                    },
                );
                assert!(matches!(
                    recovered,
                    Err(LocalOperationRecoveryError::Association)
                ));

                let prepared = prepare_local_child_parent(
                    OperationObservationRecoveryInput {
                        store: &foreign,
                        operation: &operation,
                        observation: OperationObservationSelection::SelectedHead,
                        checkpoint_authority: &authority,
                        limits: OperationObservationRecoveryLimits::new(4, 1_048_576, 262_144)
                            .unwrap(),
                        checkpoint_limits: OperationCheckpointAncestryLimits::new(4, 1_048_576, 4)
                            .unwrap(),
                        membership_limits: AccumulatorMembershipLimits::new(1, 1024, 1_048_576)
                            .unwrap(),
                    },
                    retained.lease,
                    &mut port,
                    &mut anchor,
                    OperationObservationRecoveryAdmission {
                        admit_checkpoint: |_: &ResidentRecordRefV1,
                                           _: ControlStateCheckpointWireV1|
                         -> Result<
                            ControlStateCheckpointV1,
                            InvalidResidentRecordV1,
                        > {
                            panic!("foreign checkpoint admission")
                        },
                        resolve_signer: |_: &ResidentRecordRefV1,
                                         _: &OperationObservationWireV1|
                         -> Result<
                            SigningAuthorityBindingV1,
                            InvalidResidentRecordV1,
                        > {
                            panic!("foreign signer resolution")
                        },
                        admit_state: |_: &OperationObservationStateV1| {
                            panic!("foreign state admission")
                        },
                        admit_rotation:
                            |_: &ResidentRecordRefV1,
                             _: &ArtifactDigest,
                             _: &ResidentRecordRefV1,
                             _: &ArtifactDigest| {
                                panic!("foreign rotation admission")
                            },
                        verify_signature:
                            |_: &[u8],
                             _: &OperationObservationPreimageV1,
                             _: &OperationObservationDigest,
                             _: &CanonicalSignatureV1| {
                                panic!("foreign observation signature")
                            },
                    },
                    |_: &[u8],
                     _: &ControlStateCheckpointPreimageV1,
                     _: &ControlStateCheckpointDigest,
                     _: &CanonicalSignatureV1| {
                        panic!("foreign checkpoint signature")
                    },
                );
                assert!(matches!(
                    prepared,
                    Err(LocalOperationStartError::Coordinate)
                ));
                assert!(port.index_queries.is_empty());
                assert!(port.checkpoint_queries.is_empty());
                assert_eq!((port.commits, port.read_backs, anchor.reads), (0, 0, 0));
                assert_eq!(session.1.load(Ordering::Relaxed), 0);
            }
        },
    );
}

#[test]
fn application_observe_recovers_history_before_present_authority_without_mutation() {
    locator_recovery_cases(&["observe-allowed", "observe-denied"], false, None);
}

/// Lend the existing admitted Observe fixture to one process-host test.
pub(in crate::resident) fn with_process_host_observe_fixture(run: &mut ProcessHostObserveRunner) {
    locator_recovery_cases_with_runner(&["observe-allowed"], false, None, Some(run), None, None);
}

/// Lend real terminal, recovery, consultation, and cancellation-predecessor histories.
pub(in crate::resident) fn with_process_host_restart_fixtures(run: &mut ProcessHostRestartRunner) {
    locator_recovery_cases_with_runner(
        &[
            "observe-allowed",
            "cancel-active-allowed",
            "cancel-reconciliation",
            "cancel-collecting",
            "cancel-already-requested",
        ],
        false,
        None,
        None,
        None,
        Some(run),
    );
}

/// Lend one durable root Consult whose rebuilt collection awaits policy.
pub(in crate::resident) fn with_process_host_root_consult_policy_fixture(
    run: &mut ProcessHostRestartRunner,
) {
    locator_recovery_cases_with_runner(
        &["restart-root-consult-policy"],
        false,
        None,
        None,
        None,
        Some(run),
    );
}

/// Lend the three uncertain root-Invoke heads eligible for reconciliation.
pub(in crate::resident) fn with_process_host_reconciliation_fixtures(
    run: &mut ProcessHostRestartRunner,
) {
    locator_recovery_cases_with_runner(
        &[
            "cancel-reconciliation",
            "reconcile-recovering",
            "reconcile-expired",
        ],
        false,
        None,
        None,
        None,
        Some(run),
    );
}

/// Lend each retained pre-dispatch cancellation phase to process-host recovery tests.
pub(in crate::resident) fn with_process_host_local_cancellation_restart_fixtures(
    run: &mut ProcessHostRestartRunner,
) {
    locator_recovery_cases_with_runner(
        &[
            "restart-local-cancellation-resolving",
            "restart-local-cancellation-context-bound",
            "restart-local-cancellation-provider-resolved",
            "restart-local-cancellation-dispatch-prepared",
        ],
        false,
        None,
        None,
        None,
        Some(run),
    );
}

/// Lend exact native, provider, and provider-cancellation dispatch histories.
pub(in crate::resident) fn with_process_host_restart_dispatch_fixtures(
    run: &mut ProcessHostRestartRunner,
) {
    locator_recovery_cases_with_runner(
        &["restart-native-dispatched"],
        false,
        None,
        None,
        None,
        Some(run),
    );
    locator_recovery_cases_with_runner(
        &[
            "restart-provider-dispatched",
            "restart-provider-cancellation-requested",
        ],
        false,
        None,
        None,
        None,
        Some(run),
    );
}

/// Lend exact native and provider preparations before any dispatch call.
pub(in crate::resident) fn with_process_host_prepared_restart_fixtures(
    run: &mut ProcessHostRestartRunner,
) {
    locator_recovery_cases_with_runner(
        &["restart-native-prepared"],
        false,
        None,
        None,
        None,
        Some(&mut *run),
    );
    locator_recovery_cases_with_runner(
        &["restart-provider-prepared"],
        false,
        None,
        None,
        None,
        Some(run),
    );
}

/// Lend one exact provider-resolved head before an attempt identity exists.
pub(in crate::resident) fn with_process_host_provider_resolved_restart_fixture(
    run: &mut ProcessHostRestartRunner,
) {
    locator_recovery_cases_with_runner(
        &[
            "restart-provider-resolved",
            "restart-provider-resolved-evidence",
            "restart-provider-resolved-anchor",
        ],
        false,
        None,
        None,
        None,
        Some(run),
    );
}

/// Lend the pre-attempt root Invoke phases to process-host recovery tests.
pub(in crate::resident) fn with_process_host_operation_resolution_restart_fixtures(
    run: &mut ProcessHostRestartRunner,
) {
    locator_recovery_cases_with_all_runners(
        &[
            "restart-invoke-resolving",
            "restart-invoke-resolving-evidence",
            "restart-invoke-context-bound",
            "restart-invoke-context-bound-self-contained",
        ],
        false,
        None,
        LocatorRecoveryRunners {
            process_host_restart: Some(run),
            ..LocatorRecoveryRunners::default()
        },
    );
}

/// Lend one valid root-Invoke source for hostile sweep-boundary substitutions.
pub(in crate::resident) fn with_process_host_recovery_sweep_fixture(
    run: &mut ProcessHostRestartRunner,
) {
    locator_recovery_cases_with_runner(
        &["restart-invoke-resolving"],
        false,
        None,
        None,
        None,
        Some(run),
    );
}

/// Lend provider-resolved heads whose exact predecessor names another runtime.
pub(in crate::resident) fn with_process_host_foreign_provider_predecessor_restart_fixtures(
    run: &mut ProcessHostRestartRunner,
) {
    locator_recovery_cases_with_runner(
        &[
            "restart-provider-resolved-foreign-resolving",
            "restart-provider-resolved-foreign-context",
        ],
        false,
        None,
        None,
        None,
        Some(run),
    );
}

/// Lend a native operation whose hostile head only looks provider-resolved.
pub(in crate::resident) fn with_process_host_native_provider_resolved_restart_fixture(
    run: &mut ProcessHostRestartRunner,
) {
    locator_recovery_cases_with_runner(
        &["restart-native-provider-resolved"],
        false,
        None,
        None,
        None,
        Some(run),
    );
}

/// Lend provider dispatch histories that can enter uncertainty recovery.
pub(in crate::resident) fn with_process_host_provider_recovery_fixtures(
    run: &mut ProcessHostRestartRunner,
) {
    locator_recovery_cases_with_runner(
        &[
            "restart-provider-dispatched",
            "restart-provider-recoverable-cancellation-requested",
        ],
        false,
        None,
        None,
        None,
        Some(run),
    );
}

/// Lend one exact original native dispatch with its preparation predecessor.
pub(in crate::resident) fn with_process_host_native_recovery_fixtures(
    run: &mut ProcessHostRestartRunner,
) {
    locator_recovery_cases_with_runner(
        &[
            "restart-native-recoverable-dispatched",
            "restart-native-staged",
        ],
        false,
        None,
        None,
        None,
        Some(run),
    );
}

/// Lend retained histories to one root-Invoke request-resolution test.
pub(in crate::resident) fn with_process_host_invoke_request_fixtures(
    run: &mut ProcessHostInvokeRequestRunner,
) {
    locator_recovery_cases_with_all_runners(
        &[
            "observe-allowed",
            "cancel-active-allowed",
            "cancel-reconciliation",
            "cancel-collecting",
            "cancel-already-requested",
        ],
        false,
        None,
        LocatorRecoveryRunners {
            process_host_invoke_request: Some(&mut *run),
            ..LocatorRecoveryRunners::default()
        },
    );
    locator_recovery_cases_with_all_runners(
        &["restart-native-dispatched"],
        false,
        None,
        LocatorRecoveryRunners {
            process_host_invoke_request: Some(&mut *run),
            ..LocatorRecoveryRunners::default()
        },
    );
    locator_recovery_cases_with_all_runners(
        &[
            "restart-provider-dispatched",
            "restart-provider-cancellation-requested",
        ],
        false,
        None,
        LocatorRecoveryRunners {
            process_host_invoke_request: Some(run),
            ..LocatorRecoveryRunners::default()
        },
    );
}

/// Lend the existing denied Observe fixture to one process-host error test.
pub(in crate::resident) fn with_denied_process_host_observe_fixture(
    run: &mut ProcessHostObserveRunner,
) {
    locator_recovery_cases_with_runner(&["observe-denied"], false, None, Some(run), None, None);
}

/// Lend every non-appending Cancel disposition to process-host tests.
pub(in crate::resident) fn with_process_host_cancellation_dispositions(
    run: &mut ProcessHostCancellationRunner,
) {
    locator_recovery_cases_with_runner(
        &[
            "cancel-terminal-allowed",
            "cancel-already-requested",
            "cancel-provider-dispatched",
            "cancel-collecting",
            "cancel-collected",
            "cancel-reconciliation",
        ],
        false,
        None,
        None,
        Some(run),
        None,
    );
}

/// Lend every closed or non-Consult disposition to the Consult-specific route.
pub(in crate::resident) fn with_process_host_consultation_cancellation_dispositions(
    run: &mut ProcessHostCancellationRunner,
) {
    locator_recovery_cases_with_runner(
        &[
            "cancel-terminal-allowed",
            "cancel-already-requested",
            "cancel-provider-dispatched",
            "cancel-collected",
            "cancel-reconciliation",
        ],
        false,
        None,
        None,
        Some(run),
        None,
    );
}

/// Lend one before-dispatch operation to the Consult-specific cancellation route.
pub(in crate::resident) fn with_process_host_before_dispatch_consultation_cancellation_fixture(
    run: &mut ProcessHostCancellationRunner,
) {
    locator_recovery_cases_with_runner(
        &["cancel-before-dispatch-consult"],
        false,
        None,
        None,
        Some(run),
        None,
    );
}

/// Lend one appending Cancel fixture to hostile process-host selection tests.
pub(in crate::resident) fn with_process_host_active_cancellation_fixture(
    run: &mut ProcessHostCancellationRunner,
) {
    locator_recovery_cases_with_runner(
        &["cancel-active-allowed"],
        false,
        None,
        None,
        Some(run),
        None,
    );
}

/// Lend one collecting consultation with exact recovery and append custody.
pub(in crate::resident) fn with_process_host_collecting_cancellation_fixture(
    run: &mut ProcessHostCancellationRunner,
) {
    locator_recovery_cases_with_runner(
        &["cancel-collecting-live"],
        false,
        None,
        None,
        Some(run),
        None,
    );
}

/// Lend one exact Cancel state to a framed resident-program test.
pub(in crate::resident) fn with_resident_program_cancellation_fixture(
    fault: &str,
    run: &mut ResidentProgramCancellationRunner,
) {
    locator_recovery_cases_with_all_runners(
        &[fault],
        false,
        None,
        LocatorRecoveryRunners {
            resident_program_cancellation: Some(run),
            ..LocatorRecoveryRunners::default()
        },
    );
}

/// Lend one exact Observe state to a framed resident-program test.
pub(in crate::resident) fn with_resident_program_observe_fixture(
    run: &mut ResidentProgramObserveRunner,
) {
    locator_recovery_cases_with_all_runners(
        &["observe-allowed"],
        false,
        None,
        LocatorRecoveryRunners {
            resident_program_observe: Some(run),
            ..LocatorRecoveryRunners::default()
        },
    );
}

/// Lend one exact Observe state admitted by an existing deployment trust set.
pub(in crate::resident) fn with_resident_program_observe_fixture_at_authorities(
    authorities: crate::session_admission_fixture::LocalStartExistingAuthorities<'_>,
    run: &mut ResidentProgramObserveRunner,
) {
    locator_recovery_cases_with_all_runners(
        &["observe-allowed"],
        false,
        None,
        LocatorRecoveryRunners {
            resident_program_observe: Some(run),
            resident_program_observe_authorities: Some(authorities),
            ..LocatorRecoveryRunners::default()
        },
    );
}

/// Lend one collecting consultation whose database outcome needs exact read-back.
pub(in crate::resident) fn with_process_host_collecting_cancellation_database_retry_fixture(
    run: &mut ProcessHostCancellationRunner,
) {
    locator_recovery_cases_with_runner(
        &["cancel-collecting-live-database-retry"],
        false,
        None,
        None,
        Some(run),
        None,
    );
}

/// Lend one collecting consultation whose signed cancellation cannot prepare its checkpoint.
pub(in crate::resident) fn with_process_host_collecting_cancellation_preparation_failure_fixture(
    run: &mut ProcessHostCancellationRunner,
) {
    locator_recovery_cases_with_runner(
        &["cancel-collecting-live-preparation-refused"],
        false,
        None,
        None,
        Some(run),
        None,
    );
}

/// Lend one collecting consultation whose retained anchor transition is retried.
pub(in crate::resident) fn with_process_host_collecting_cancellation_anchor_retry_fixture(
    run: &mut ProcessHostCancellationRunner,
) {
    locator_recovery_cases_with_runner(
        &["cancel-collecting-live-anchor-retry"],
        false,
        None,
        None,
        Some(run),
        None,
    );
}

/// Lend one present-authority-denied Cancel fixture to process-host tests.
pub(in crate::resident) fn with_denied_process_host_cancellation_fixture(
    run: &mut ProcessHostCancellationRunner,
) {
    locator_recovery_cases_with_runner(
        &["cancel-terminal-denied"],
        false,
        None,
        None,
        Some(run),
        None,
    );
}

#[test]
fn application_cancel_recovers_history_before_present_authority_without_replaying_terminal_state() {
    locator_recovery_cases(
        &["cancel-terminal-allowed", "cancel-terminal-denied"],
        false,
        None,
    );
}

#[test]
fn application_cancel_recovers_and_authorizes_before_one_active_append() {
    locator_recovery_cases(
        &[
            "cancel-active-allowed",
            "cancel-active-denied",
            "cancel-active-database-failed",
            "cancel-active-anchor-failed",
        ],
        false,
        None,
    );
}

fn locator_recovery_cases(
    faults: &[&str],
    from_handle: bool,
    basis_fault: Option<session_basis::Fault>,
) {
    locator_recovery_cases_with_runner(faults, from_handle, basis_fault, None, None, None);
}

fn locator_recovery_cases_with_runner(
    faults: &[&str],
    from_handle: bool,
    basis_fault: Option<session_basis::Fault>,
    process_host_observe: Option<&mut ProcessHostObserveRunner>,
    process_host_cancellation: Option<&mut ProcessHostCancellationRunner>,
    process_host_restart: Option<&mut ProcessHostRestartRunner>,
) {
    locator_recovery_cases_with_all_runners(
        faults,
        from_handle,
        basis_fault,
        LocatorRecoveryRunners {
            process_host_observe,
            process_host_cancellation,
            process_host_restart,
            ..LocatorRecoveryRunners::default()
        },
    );
}

fn locator_recovery_cases_with_all_runners(
    faults: &[&str],
    from_handle: bool,
    basis_fault: Option<session_basis::Fault>,
    mut runners: LocatorRecoveryRunners<'_>,
) {
    let (generation, profile, mut restart_generations, mut observe_reconstruction) = if runners
        .process_host_restart
        .is_some()
        || runners.process_host_invoke_request.is_some()
        || runners.resident_program_cancellation.is_some()
        || runners.resident_program_observe.is_some()
    {
        let rediscovery_copies = faults
            .iter()
            .filter(|fault| fault.starts_with("restart-local-cancellation-"))
            .count();
        let copies = faults
            .len()
            .checked_add(rediscovery_copies)
            .and_then(|count| count.checked_add(1))
            .expect("restart fixture count");
        let (mut generations, profile, reconstruction) =
            if runners.resident_program_observe.is_some() {
                let (generations, profile, reconstruction) =
                    super::local_start::source_generation_copies_with_reconstruction(copies);
                (generations, profile, Some(reconstruction))
            } else if faults
                .iter()
                .all(|fault| fault.starts_with("restart-provider"))
            {
                let (generations, profile) =
                    super::local_start::host_source_generation_copies(copies);
                (generations, profile, None)
            } else {
                let (generations, profile) = super::local_start::source_generation_copies(copies);
                (generations, profile, None)
            };
        let generation = generations.remove(0);
        (
            generation,
            profile,
            Some(generations.into_iter()),
            reconstruction,
        )
    } else {
        let (generation, profile) = source_generation();
        (generation, profile, None, None)
    };
    let mut domains = std::collections::BTreeSet::new();
    for &fault in faults {
        let application_control = fault.starts_with("cancel-collecting-live")
            || matches!(
                fault,
                "observe-allowed"
                    | "observe-denied"
                    | "cancel-terminal-allowed"
                    | "cancel-terminal-denied"
                    | "cancel-active-allowed"
                    | "cancel-active-denied"
                    | "cancel-active-database-failed"
                    | "cancel-active-anchor-failed"
                    | "cancel-already-requested"
                    | "cancel-provider-dispatched"
                    | "cancel-collecting"
                    | "cancel-collected"
                    | "cancel-reconciliation"
                    | "reconcile-recovering"
                    | "reconcile-expired"
                    | "cancel-before-dispatch-consult"
                    | "restart-native-dispatched"
                    | "restart-native-prepared"
                    | "restart-native-provider-resolved"
                    | "restart-native-recoverable-dispatched"
                    | "restart-native-staged"
                    | "restart-provider-dispatched"
                    | "restart-provider-resolved"
                    | "restart-provider-resolved-evidence"
                    | "restart-provider-resolved-anchor"
                    | "restart-provider-resolved-foreign-resolving"
                    | "restart-provider-resolved-foreign-context"
                    | "restart-provider-prepared"
                    | "restart-provider-cancellation-requested"
                    | "restart-provider-recoverable-cancellation-requested"
            );
        let application_control = application_control
            || fault.starts_with("restart-local-cancellation-")
            || fault.starts_with("restart-invoke-")
            || fault == "restart-root-consult-policy";
        let native_registry =
            crate::actions::NativeActionRegistry::new().expect("closed native action registry");
        let native_restart = matches!(
            fault,
            "restart-native-dispatched"
                | "restart-native-prepared"
                | "restart-native-provider-resolved"
                | "restart-native-recoverable-dispatched"
                | "restart-native-staged"
        );
        crate::local_operation_start_fixture::with_start_context_class(
            &generation,
            &profile,
            fault,
            |request| {
                if native_restart {
                    crate::local_operation_start_fixture::native_request_body(request);
                }
            },
            if native_restart {
                StartAdmission::NativeInput {
                    registry: &native_registry,
                    reject_owner: false,
                }
            } else {
                StartAdmission::AcceptedOwnerModel
            },
            if matches!(
                fault,
                "restart-provider-resolved-evidence"
                    | "restart-provider-resolved-foreign-context"
                    | "restart-invoke-resolving-evidence"
                    | "restart-invoke-context-bound"
            ) {
                crate::session_admission_fixture::LocalStartContext::Inherited
            } else if matches!(
                fault,
                "restart-invoke-resolving"
                    | "restart-invoke-context-bound-self-contained"
                    | "restart-native-recoverable-dispatched"
                    | "restart-native-staged"
            ) || fault.starts_with("restart-provider")
            {
                crate::session_admission_fixture::LocalStartContext::SelfContained
            } else {
                crate::session_admission_fixture::LocalStartContext::Inherited
            },
            if fault.starts_with("cancel-collecting") || fault == "restart-root-consult-policy" {
                OperationClassV1::Consult
            } else {
                OperationClassV1::Invoke
            },
            runners.resident_program_observe.is_some(),
            runners.resident_program_observe_authorities,
            |prepared, request, owners, signatures, retained| {
                assert_eq!((owners, signatures), (1, 2));
                let root_consult_evidence =
                    (fault == "restart-root-consult-policy").then(|| ImmutableRecordReferenceV1 {
                        record_ref: record_ref("evidence/root-consult-policy"),
                        record_sha256: ArtifactDigest::hash_bytes(b"evidence/root-consult-policy"),
                    });
                let mut prepared = prepared.unwrap();
                if let Some(evidence) = root_consult_evidence.as_ref() {
                    let (handle_ref, handle) = prepared.prepared_handle();
                    let intent = crate::resident::consultation::ConsultationStartIntent::admit_signed(
                        &store_coordinate(),
                        handle_ref,
                        handle,
                        crate::resident::consultation::ConsultationStartIntentAdmission {
                            completion_policy: evidence.clone(),
                            opened_at: *handle.created_at(),
                            deadline: *handle.expires_at(),
                            slots: vec![crate::resident::consultation::ConsultationStartIntentSlot::new(
                                crate::resident::ConsultationSlotId::parse("root-policy-evidence")
                                    .expect("fixture slot"),
                                crate::resident::ConsultationRequirement::Required,
                                evidence.clone(),
                                ParticipantRefV1::Application {
                                    application_id: "root-policy-evidence".into(),
                                },
                                None,
                                None,
                                crate::resident::consultation::ConsultationStartIntentSource::EvidenceResolved {
                                    evidence: evidence.clone(),
                                },
                            )],
                            limits: crate::resident::consultation::ConsultationCollectionLimits::new(
                                1, 65_536, 65_536,
                            )
                            .expect("bounded root Consult collection"),
                        },
                    )
                    .expect("root Consult intent matches the signed parent");
                    prepared = prepared.with_test_consultation_start_intent(intent);
                }
                let records = prepared.records().to_vec();
                let mut initial = None;
                let mut native = super::checkpoint_preparation::PreparationOnly::new(
                    |request: &OperationAccumulatorPreparation| {
                        initial = Some(request.clone());
                        super::local_start::initial_response(request)
                    },
                    ControlTransactionSequenceV1::new(1).unwrap(),
                );
                let prepared = prepared
                    .prepare_checkpoint(
                        AccumulatorPreparationLimits::new(4, 10_000, 10_000, 4096, 100_000)
                            .unwrap(),
                        &mut native,
                    )
                    .unwrap();
                let checkpoint_signs = Cell::new(0);
                let (physical, pending, mut checkpoint) =
                    super::local_start::issue_initial(prepared, request, |_| {}, &checkpoint_signs)
                        .unwrap();
                let mut committing = super::local_start::absent_start_store(request);
                let mut initial_anchor = super::local_start::initial_anchor();
                let database = physical
                    .commit(&mut committing, &mut initial_anchor, 1024 * 1024)
                    .unwrap();
                let mut head = database.candidate().checkpoint().successor().clone();
                assert!(
                    domains.insert(head.checkpoint_domain_id().clone()),
                    "different actual request identities have isolated domains"
                );
                let mut checkpoint_record =
                    database.candidate().checkpoint().successor_record().clone();
                let session_basis = database.candidate().session_basis().unwrap().clone();
                let completed = database.advance_anchor(&mut initial_anchor).unwrap();
                let issued = pending
                    .finish(completed)
                    .unwrap_or_else(|_| panic!("exact start"));
                let rotated_signing = SigningAuthorityBindingV1::integrated_runtime(
                    retained.signing.principal_id().clone(),
                    ResidentKeyIdV1::parse("rotated-recovery-key").unwrap(),
                    SigningKeyEpochV1::new(2).unwrap(),
                    IntegratedRuntimeAuthorityV1 {
                        deployment_runtime_authority_ref: record_ref(
                            "rotated-deployment-runtime-authority",
                        ),
                        deployment_runtime_authority_sha256: artifact_digest('a'),
                        deployment_binding_ref: retained
                            .signing
                            .integrated_runtime_authority()
                            .unwrap()
                            .deployment_binding_ref
                            .clone(),
                        deployment_binding_sha256: retained
                            .signing
                            .integrated_runtime_authority()
                            .unwrap()
                            .deployment_binding_sha256
                            .clone(),
                        checked_principal_trust_checkpoint_ref: retained
                            .signing
                            .integrated_runtime_authority()
                            .unwrap()
                            .checked_principal_trust_checkpoint_ref
                            .clone(),
                        checked_principal_trust_checkpoint_sha256: retained
                            .signing
                            .integrated_runtime_authority()
                            .unwrap()
                            .checked_principal_trust_checkpoint_sha256
                            .clone(),
                    },
                )
                .unwrap();
                super::local_start::assert_publication_policy(&issued, request, fault);
                let expected = (issued.handle().0.clone(), issued.handle().1.clone());
                let start_observation = records[2].record().clone();
                let start_checkpoint = checkpoint_record.clone();
                let mut checkpoint_history = std::collections::BTreeMap::from([(
                    start_checkpoint.coordinate().record_ref().clone(),
                    start_checkpoint.clone(),
                )]);
                let mut selected_observation = start_observation.clone();
                let mut observation_history = std::collections::BTreeMap::new();
                let mut observation_leaf = None;
                let mut consultation = None;
                let mut attempt_selection = None;
                let mut retained_cancellation_request = None;
                if fault == "after-observation" {
                    (head, checkpoint_record, checkpoint) = replace_observation(
                        &issued,
                        records[2].record(),
                        &checkpoint,
                        retained.signing,
                        retained.session,
                    );
                    assert_eq!(head.domain_sequence().get(), 2);
                } else if [
                    "initial-observation-substituted",
                    "initial-checkpoint-substituted",
                    "initial-observation-rotation",
                    "initial-observation-revision-bound",
                    "initial-checkpoint-bound",
                ]
                .contains(&fault)
                {
                    let first = issue_observation(
                        retained.signing,
                        &expected.0,
                        &expected.1,
                        schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
                        OperationObservationRevisionV1::new(1).unwrap(),
                        None,
                        OperationObservationStateV1::Active {
                            phase: OperationActivePhaseV1::Resolving,
                        },
                        expected.1.created_at().get(),
                    );
                    let successor_signing = if fault == "initial-observation-rotation" {
                        &rotated_signing
                    } else {
                        retained.signing
                    };
                    let (next_head, next_checkpoint_record, next_checkpoint, reference, next) =
                        append_observation_with_state(
                            &issued,
                            &start_observation,
                            &first,
                            &checkpoint,
                            successor_signing,
                            retained.session,
                            OperationObservationStateV1::Active {
                                phase: OperationActivePhaseV1::Resolving,
                            },
                        );
                    observation_history.insert(
                        start_observation.coordinate().record_ref().clone(),
                        start_observation.clone(),
                    );
                    selected_observation = crate::resident::record_projection::record(
                        crate::resident::record_projection::coordinate(
                            &reference,
                            next.canonical_operation_observation_sha256().as_str(),
                        )
                        .unwrap(),
                        &next,
                    )
                    .unwrap();
                    observation_leaf = Some(
                        crate::resident::checkpoint_profile::operation_observation_membership(
                            &store_coordinate(),
                            &next_checkpoint,
                            &reference,
                            &next,
                        )
                        .unwrap(),
                    );
                    head = next_head;
                    checkpoint_record = next_checkpoint_record;
                    checkpoint = next_checkpoint;
                } else if application_control {
                    let first = issue_observation(
                        retained.signing,
                        &expected.0,
                        &expected.1,
                        schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
                        OperationObservationRevisionV1::new(1).unwrap(),
                        None,
                        OperationObservationStateV1::Active {
                            phase: OperationActivePhaseV1::Resolving,
                        },
                        expected.1.created_at().get(),
                    );
                    assert_eq!(
                        pale_core::canon::to_canonical_string(&first)
                            .unwrap()
                            .as_bytes(),
                        selected_observation.canonical_record_jcs().as_slice()
                    );
                    consultation = if let Some(evidence) = root_consult_evidence.clone() {
                        let (handle_ref, handle) = issued.handle();
                        Some(
                            crate::resident::consultation::tests::consultation_evidence_fixture_for_operation_in_store(
                                handle_ref,
                                handle,
                                evidence,
                                store_coordinate(),
                            ),
                        )
                    } else {
                        fault.starts_with("cancel-collecting").then(|| {
                            let (handle_ref, handle) = issued.handle();
                            let child = crate::resident::operation_observation_custody::tests::
                            OperationFixture::exact();
                            let (child_ref, child_handle) = child.handle();
                            crate::resident::consultation::tests::
                            consultation_supervision_fixture_for_operation_in_store(
                                handle_ref,
                                handle,
                                "cancel-child",
                                ImmutableRecordReferenceV1 {
                                    record_ref: child_ref.clone(),
                                    record_sha256: ArtifactDigest::parse(
                                        child_handle.canonical_operation_handle_sha256().as_str(),
                                    )
                                    .expect("consultation child artifact digest"),
                                },
                                false,
                                store_coordinate(),
                            )
                        })
                    };
                    let (dispatch_attempt, selected_attempt) = match fault {
                        "restart-native-dispatched"
                        | "restart-native-prepared"
                        | "restart-native-recoverable-dispatched"
                        | "restart-native-staged" => {
                            let (attempt, record) = crate::resident::operation_dispatch::selection::tests::restart_native_selection_record(
                                &issued,
                                request,
                                &generation,
                                &retained,
                                &native_registry,
                            );
                            (Some(attempt), Some(record))
                        }
                        "restart-provider-dispatched"
                        | "restart-provider-prepared"
                        | "restart-provider-cancellation-requested"
                        | "restart-provider-recoverable-cancellation-requested" => {
                            let (attempt, record) = crate::resident::operation_dispatch::selection::tests::restart_provider_selection_record(
                                &issued,
                                request,
                                &generation,
                                &retained,
                            );
                            (Some(attempt), Some(record))
                        }
                        _ => (None, None),
                    };
                    attempt_selection = selected_attempt;
                    let dispatch_evidence =
                        attempt_selection
                            .as_ref()
                            .map(|record| ImmutableRecordReferenceV1 {
                                record_ref: record.coordinate().record_ref().clone(),
                                record_sha256: ArtifactDigest::parse(
                                    record.coordinate().record_sha256().as_str(),
                                )
                                .expect("attempt selection artifact digest"),
                            });
                    if fault == "restart-provider-recoverable-cancellation-requested"
                        || fault.starts_with("restart-local-cancellation-")
                    {
                        let cancellation = if fault.starts_with("restart-local-cancellation-") {
                            retained_pre_call_cancellation_request(request, issued.handle().1)
                        } else {
                            cancellation_request(request, issued.handle().1)
                        };
                        let coordinate = crate::resident::record_projection::coordinate(
                            &record_ref(if fault.starts_with("restart-local-cancellation-") {
                                "restart/local-cancellation-request"
                            } else {
                                "restart/cancellation-request"
                            }),
                            cancellation.canonical_request_sha256().as_str(),
                        )
                        .expect("provider cancellation request coordinate");
                        let record =
                            crate::resident::record_projection::record(coordinate, &cancellation)
                                .expect("provider cancellation request record");
                        retained_cancellation_request = Some((cancellation, record));
                    }
                    let control_state = match fault {
                        "restart-invoke-resolving" | "restart-invoke-resolving-evidence" => {
                            OperationObservationStateV1::Active {
                                phase: OperationActivePhaseV1::Resolving,
                            }
                        }
                        "restart-invoke-context-bound"
                        | "restart-invoke-context-bound-self-contained" => {
                            OperationObservationStateV1::Active {
                                phase: OperationActivePhaseV1::ContextBound {
                                    evidence: ImmutableRecordReferenceV1 {
                                        record_ref: record_ref("context/restart-invoke"),
                                        record_sha256: artifact_digest('6'),
                                    },
                                },
                            }
                        }
                        "restart-local-cancellation-resolving" => {
                            OperationObservationStateV1::Active {
                                phase: OperationActivePhaseV1::Resolving,
                            }
                        }
                        "restart-local-cancellation-context-bound" => {
                            OperationObservationStateV1::Active {
                                phase: OperationActivePhaseV1::ContextBound {
                                    evidence: ImmutableRecordReferenceV1 {
                                        record_ref: record_ref(
                                            "context/restart-local-cancellation",
                                        ),
                                        record_sha256: artifact_digest('6'),
                                    },
                                },
                            }
                        }
                        "restart-local-cancellation-provider-resolved" => {
                            OperationObservationStateV1::Active {
                                phase: OperationActivePhaseV1::ProviderResolved {
                                    provider_binding: ProviderBindingRefV1 {
                                        provider_binding_id: "restart-local-cancellation-provider"
                                            .into(),
                                        provider_binding_identity: ProviderBindingIdentityV1 {
                                            provider_binding_sha256: artifact_digest('7'),
                                        },
                                    },
                                },
                            }
                        }
                        "restart-local-cancellation-dispatch-prepared" => {
                            OperationObservationStateV1::Active {
                                phase: OperationActivePhaseV1::DispatchPrepared {
                                    attempt_id: CapabilityAttemptIdV1::parse(
                                        "restart-local-cancellation-attempt",
                                    )
                                    .unwrap(),
                                    evidence: ImmutableRecordReferenceV1 {
                                        record_ref: record_ref(
                                            "dispatch/restart-local-cancellation",
                                        ),
                                        record_sha256: artifact_digest('8'),
                                    },
                                },
                            }
                        }
                        "restart-provider-resolved" | "restart-provider-resolved-anchor" => {
                            let provider = generation
                                .provider_binding("provider-binding-claude-code")
                                .expect("provider-resolved fixture binding");
                            OperationObservationStateV1::Active {
                                phase: OperationActivePhaseV1::ProviderResolved {
                                    provider_binding: ProviderBindingRefV1 {
                                        provider_binding_id: provider.id.clone(),
                                        provider_binding_identity: provider
                                            .binding_identity
                                            .clone(),
                                    },
                                },
                            }
                        }
                        "restart-provider-resolved-evidence"
                        | "restart-provider-resolved-foreign-context" => {
                            OperationObservationStateV1::Active {
                                phase: OperationActivePhaseV1::ContextBound {
                                    evidence: ImmutableRecordReferenceV1 {
                                        record_ref: record_ref("context/restart-provider-resolved"),
                                        record_sha256: artifact_digest('6'),
                                    },
                                },
                            }
                        }
                        "restart-provider-resolved-foreign-resolving" => {
                            OperationObservationStateV1::Active {
                                phase: OperationActivePhaseV1::Resolving,
                            }
                        }
                        "restart-native-provider-resolved" => OperationObservationStateV1::Active {
                            phase: OperationActivePhaseV1::ProviderResolved {
                                provider_binding: ProviderBindingRefV1 {
                                    provider_binding_id: "host-native-cannot-be-provider".into(),
                                    provider_binding_identity: ProviderBindingIdentityV1 {
                                        provider_binding_sha256: artifact_digest('9'),
                                    },
                                },
                            },
                        },
                        value
                            if value.starts_with("cancel-active")
                                || value == "cancel-before-dispatch-consult" =>
                        {
                            OperationObservationStateV1::Active {
                                phase: OperationActivePhaseV1::ContextBound {
                                    evidence: ImmutableRecordReferenceV1 {
                                        record_ref: record_ref("context/retained"),
                                        record_sha256: artifact_digest('f'),
                                    },
                                },
                            }
                        }
                        "cancel-already-requested" => {
                            OperationObservationStateV1::CancellationRequested {
                                request: ImmutableRecordReferenceV1 {
                                    record_ref: record_ref("request/already-cancelled"),
                                    record_sha256: artifact_digest('a'),
                                },
                            }
                        }
                        "cancel-provider-dispatched" => OperationObservationStateV1::Active {
                            phase: OperationActivePhaseV1::Dispatched {
                                attempt_id: CapabilityAttemptIdV1::parse("attempt-process-host")
                                    .unwrap(),
                                evidence: ImmutableRecordReferenceV1 {
                                    record_ref: record_ref("dispatch/process-host"),
                                    record_sha256: artifact_digest('b'),
                                },
                            },
                        },
                        "restart-native-prepared" | "restart-provider-prepared" => {
                            OperationObservationStateV1::Active {
                                phase: OperationActivePhaseV1::ContextBound {
                                    evidence: ImmutableRecordReferenceV1 {
                                        record_ref: record_ref("context/restart-prepared"),
                                        record_sha256: artifact_digest('6'),
                                    },
                                },
                            }
                        }
                        "restart-native-dispatched"
                        | "restart-native-recoverable-dispatched"
                        | "restart-native-staged"
                        | "restart-provider-dispatched"
                        | "restart-provider-cancellation-requested"
                        | "restart-provider-recoverable-cancellation-requested" => {
                            OperationObservationStateV1::Active {
                                phase: if matches!(
                                    fault,
                                    "restart-native-recoverable-dispatched"
                                        | "restart-native-staged"
                                        | "restart-provider-dispatched"
                                        | "restart-provider-recoverable-cancellation-requested"
                                ) {
                                    OperationActivePhaseV1::DispatchPrepared {
                                        attempt_id: dispatch_attempt
                                            .clone()
                                            .expect("restart dispatch attempt"),
                                        evidence: dispatch_evidence
                                            .clone()
                                            .expect("restart dispatch selection"),
                                    }
                                } else {
                                    OperationActivePhaseV1::Dispatched {
                                        attempt_id: dispatch_attempt
                                            .clone()
                                            .expect("restart dispatch attempt"),
                                        evidence: dispatch_evidence
                                            .clone()
                                            .expect("restart dispatch selection"),
                                    }
                                },
                            }
                        }
                        value
                            if value.starts_with("cancel-collecting")
                                || value == "restart-root-consult-policy" =>
                        {
                            OperationObservationStateV1::Active {
                                phase: OperationActivePhaseV1::CollectingConsultation {
                                    evidence: {
                                        let coordinate = consultation
                                            .as_ref()
                                            .expect("collecting consultation fixture")
                                            .snapshot()
                                            .frontier()
                                            .coordinate();
                                        ImmutableRecordReferenceV1 {
                                            record_ref: coordinate.record_ref().clone(),
                                            record_sha256: ArtifactDigest::parse(
                                                coordinate.record_sha256().as_str(),
                                            )
                                            .expect("collection frontier artifact digest"),
                                        }
                                    },
                                },
                            }
                        }
                        "cancel-collected" => OperationObservationStateV1::Active {
                            phase: OperationActivePhaseV1::ConsultationCollected {
                                evidence: ImmutableRecordReferenceV1 {
                                    record_ref: record_ref("consultation/collected"),
                                    record_sha256: artifact_digest('d'),
                                },
                            },
                        },
                        "cancel-reconciliation" => OperationObservationStateV1::OutcomeUnknown {
                            diagnostic: ImmutableRecordReferenceV1 {
                                record_ref: record_ref("diagnostic/outcome-unknown"),
                                record_sha256: artifact_digest('e'),
                            },
                            reconciliation: ReconciliationFrontierReferenceV1 {
                                record_ref: record_ref("reconciliation/process-host"),
                                record_sha256: ReconciliationFrontierDigest::parse(sha('f'))
                                    .unwrap(),
                            },
                            effect_receipt: None,
                        },
                        "reconcile-recovering" => OperationObservationStateV1::Recovering {
                            reconciliation: ReconciliationFrontierReferenceV1 {
                                record_ref: record_ref("reconciliation/recovering"),
                                record_sha256: ReconciliationFrontierDigest::parse(sha('d'))
                                    .unwrap(),
                            },
                        },
                        "reconcile-expired" => OperationObservationStateV1::Expired {
                            diagnostic: ImmutableRecordReferenceV1 {
                                record_ref: record_ref("diagnostic/expired"),
                                record_sha256: artifact_digest('c'),
                            },
                        },
                        _ => OperationObservationStateV1::Failed {
                            result: None,
                            diagnostic: ImmutableRecordReferenceV1 {
                                record_ref: record_ref("diagnostic/retained-failure"),
                                record_sha256: artifact_digest('f'),
                            },
                        },
                    };
                    let predecessor_signing = if matches!(
                        fault,
                        "restart-provider-resolved-foreign-resolving"
                            | "restart-provider-resolved-foreign-context"
                    ) {
                        &rotated_signing
                    } else {
                        retained.signing
                    };
                    let (next_head, next_checkpoint_record, next_checkpoint, reference, next) =
                        append_observation_with_state(
                            &issued,
                            &selected_observation,
                            &first,
                            &checkpoint,
                            predecessor_signing,
                            retained.session,
                            control_state,
                        );
                    observation_history.insert(
                        selected_observation.coordinate().record_ref().clone(),
                        selected_observation.clone(),
                    );
                    selected_observation = crate::resident::record_projection::record(
                        crate::resident::record_projection::coordinate(
                            &reference,
                            next.canonical_operation_observation_sha256().as_str(),
                        )
                        .unwrap(),
                        &next,
                    )
                    .unwrap();
                    observation_leaf = Some(
                        crate::resident::checkpoint_profile::operation_observation_membership(
                            &store_coordinate(),
                            &next_checkpoint,
                            &reference,
                            &next,
                        )
                        .unwrap(),
                    );
                    head = next_head;
                    checkpoint_record = next_checkpoint_record;
                    checkpoint = next_checkpoint;
                    if matches!(
                        fault,
                        "restart-provider-resolved-evidence"
                            | "restart-provider-resolved-foreign-resolving"
                            | "restart-provider-resolved-foreign-context"
                    ) {
                        let context_record = selected_observation.clone();
                        let context = next.clone();
                        checkpoint_history.insert(
                            checkpoint_record.coordinate().record_ref().clone(),
                            checkpoint_record.clone(),
                        );
                        let provider = generation
                            .provider_binding("provider-binding-claude-code")
                            .expect("provider-resolved fixture binding");
                        let (next_head, next_checkpoint_record, next_checkpoint, reference, next) =
                            append_observation_with_state(
                                &issued,
                                &context_record,
                                &context,
                                &checkpoint,
                                retained.signing,
                                retained.session,
                                OperationObservationStateV1::Active {
                                    phase: OperationActivePhaseV1::ProviderResolved {
                                        provider_binding: ProviderBindingRefV1 {
                                            provider_binding_id: provider.id.clone(),
                                            provider_binding_identity: provider
                                                .binding_identity
                                                .clone(),
                                        },
                                    },
                                },
                            );
                        observation_history.insert(
                            context_record.coordinate().record_ref().clone(),
                            context_record,
                        );
                        selected_observation = crate::resident::record_projection::record(
                            crate::resident::record_projection::coordinate(
                                &reference,
                                next.canonical_operation_observation_sha256().as_str(),
                            )
                            .unwrap(),
                            &next,
                        )
                        .unwrap();
                        observation_leaf = Some(
                            crate::resident::checkpoint_profile::operation_observation_membership(
                                &store_coordinate(),
                                &next_checkpoint,
                                &reference,
                                &next,
                            )
                            .unwrap(),
                        );
                        head = next_head;
                        checkpoint_record = next_checkpoint_record;
                        checkpoint = next_checkpoint;
                    }
                    let mut provider_dispatch =
                        (fault == "restart-provider-cancellation-requested").then(|| next.clone());
                    assert_eq!(
                        head.domain_sequence().get(),
                        if matches!(
                            fault,
                            "restart-provider-resolved-evidence"
                                | "restart-provider-resolved-foreign-resolving"
                                | "restart-provider-resolved-foreign-context"
                        ) {
                            3
                        } else {
                            2
                        }
                    );
                    if matches!(
                        fault,
                        "restart-native-prepared" | "restart-provider-prepared"
                    ) {
                        let context_record = selected_observation.clone();
                        let context = next.clone();
                        checkpoint_history.insert(
                            checkpoint_record.coordinate().record_ref().clone(),
                            checkpoint_record.clone(),
                        );
                        let (next_head, next_checkpoint_record, next_checkpoint, reference, next) =
                            append_observation_with_state(
                                &issued,
                                &context_record,
                                &context,
                                &checkpoint,
                                retained.signing,
                                retained.session,
                                OperationObservationStateV1::Active {
                                    phase: OperationActivePhaseV1::DispatchPrepared {
                                        attempt_id: dispatch_attempt
                                            .clone()
                                            .expect("restart preparation attempt"),
                                        evidence: dispatch_evidence
                                            .clone()
                                            .expect("restart preparation selection"),
                                    },
                                },
                            );
                        observation_history.insert(
                            context_record.coordinate().record_ref().clone(),
                            context_record,
                        );
                        selected_observation = crate::resident::record_projection::record(
                            crate::resident::record_projection::coordinate(
                                &reference,
                                next.canonical_operation_observation_sha256().as_str(),
                            )
                            .unwrap(),
                            &next,
                        )
                        .unwrap();
                        observation_leaf = Some(
                            crate::resident::checkpoint_profile::operation_observation_membership(
                                &store_coordinate(),
                                &next_checkpoint,
                                &reference,
                                &next,
                            )
                            .unwrap(),
                        );
                        head = next_head;
                        checkpoint_record = next_checkpoint_record;
                        checkpoint = next_checkpoint;
                        assert_eq!(head.domain_sequence().get(), 3);
                    }
                    if matches!(
                        fault,
                        "restart-native-recoverable-dispatched"
                            | "restart-native-staged"
                            | "restart-provider-dispatched"
                            | "restart-provider-recoverable-cancellation-requested"
                    ) {
                        let preparation_record = selected_observation.clone();
                        let preparation = next.clone();
                        checkpoint_history.insert(
                            checkpoint_record.coordinate().record_ref().clone(),
                            checkpoint_record.clone(),
                        );
                        let (next_head, next_checkpoint_record, next_checkpoint, reference, next) =
                            append_observation_with_state(
                                &issued,
                                &preparation_record,
                                &preparation,
                                &checkpoint,
                                retained.signing,
                                retained.session,
                                OperationObservationStateV1::Active {
                                    phase: OperationActivePhaseV1::Dispatched {
                                        attempt_id: dispatch_attempt
                                            .clone()
                                            .expect("restart dispatch attempt"),
                                        evidence: dispatch_evidence
                                            .clone()
                                            .expect("restart dispatch selection"),
                                    },
                                },
                            );
                        observation_history.insert(
                            preparation_record.coordinate().record_ref().clone(),
                            preparation_record,
                        );
                        selected_observation = crate::resident::record_projection::record(
                            crate::resident::record_projection::coordinate(
                                &reference,
                                next.canonical_operation_observation_sha256().as_str(),
                            )
                            .unwrap(),
                            &next,
                        )
                        .unwrap();
                        observation_leaf = Some(
                            crate::resident::checkpoint_profile::operation_observation_membership(
                                &store_coordinate(),
                                &next_checkpoint,
                                &reference,
                                &next,
                            )
                            .unwrap(),
                        );
                        head = next_head;
                        checkpoint_record = next_checkpoint_record;
                        checkpoint = next_checkpoint;
                        if fault == "restart-provider-recoverable-cancellation-requested" {
                            provider_dispatch = Some(next.clone());
                        }
                        assert_eq!(head.domain_sequence().get(), 3);
                        if fault == "restart-native-staged" {
                            let dispatch_record = selected_observation.clone();
                            let dispatch = next;
                            checkpoint_history.insert(
                                checkpoint_record.coordinate().record_ref().clone(),
                                checkpoint_record.clone(),
                            );
                            let (
                                next_head,
                                next_checkpoint_record,
                                next_checkpoint,
                                reference,
                                next,
                            ) = append_observation_with_state(
                                &issued,
                                &dispatch_record,
                                &dispatch,
                                &checkpoint,
                                retained.signing,
                                retained.session,
                                OperationObservationStateV1::Active {
                                    phase: OperationActivePhaseV1::Dispatched {
                                        attempt_id: dispatch_attempt
                                            .clone()
                                            .expect("restart dispatch attempt"),
                                        evidence: dispatch_evidence
                                            .clone()
                                            .expect("restart dispatch selection"),
                                    },
                                },
                            );
                            observation_history.insert(
                                dispatch_record.coordinate().record_ref().clone(),
                                dispatch_record,
                            );
                            selected_observation = crate::resident::record_projection::record(
                                crate::resident::record_projection::coordinate(
                                    &reference,
                                    next.canonical_operation_observation_sha256().as_str(),
                                )
                                .unwrap(),
                                &next,
                            )
                            .unwrap();
                            observation_leaf = Some(
                                crate::resident::checkpoint_profile::operation_observation_membership(
                                    &store_coordinate(),
                                    &next_checkpoint,
                                    &reference,
                                    &next,
                                )
                                .unwrap(),
                            );
                            head = next_head;
                            checkpoint_record = next_checkpoint_record;
                            checkpoint = next_checkpoint;
                            assert_eq!(head.domain_sequence().get(), 4);
                        }
                    }
                    if fault.starts_with("restart-local-cancellation-") {
                        let predecessor_record = selected_observation.clone();
                        let predecessor = next.clone();
                        checkpoint_history.insert(
                            checkpoint_record.coordinate().record_ref().clone(),
                            checkpoint_record.clone(),
                        );
                        let request = retained_cancellation_request
                            .as_ref()
                            .expect("local cancellation request record");
                        let (next_head, next_checkpoint_record, next_checkpoint, reference, next) =
                            append_observation_with_state(
                                &issued,
                                &predecessor_record,
                                &predecessor,
                                &checkpoint,
                                retained.signing,
                                retained.session,
                                OperationObservationStateV1::CancellationRequested {
                                    request: ImmutableRecordReferenceV1 {
                                        record_ref: request.1.coordinate().record_ref().clone(),
                                        record_sha256: ArtifactDigest::parse(
                                            request.1.coordinate().record_sha256().as_str(),
                                        )
                                        .expect("local cancellation request artifact digest"),
                                    },
                                },
                            );
                        observation_history.insert(
                            predecessor_record.coordinate().record_ref().clone(),
                            predecessor_record,
                        );
                        observation_history.insert(
                            request.1.coordinate().record_ref().clone(),
                            request.1.clone(),
                        );
                        selected_observation = crate::resident::record_projection::record(
                            crate::resident::record_projection::coordinate(
                                &reference,
                                next.canonical_operation_observation_sha256().as_str(),
                            )
                            .unwrap(),
                            &next,
                        )
                        .unwrap();
                        observation_leaf = Some(
                            crate::resident::checkpoint_profile::operation_observation_membership(
                                &store_coordinate(),
                                &next_checkpoint,
                                &reference,
                                &next,
                            )
                            .unwrap(),
                        );
                        head = next_head;
                        checkpoint_record = next_checkpoint_record;
                        checkpoint = next_checkpoint;
                        assert_eq!(head.domain_sequence().get(), 3);
                    }
                    if matches!(
                        fault,
                        "restart-provider-cancellation-requested"
                            | "restart-provider-recoverable-cancellation-requested"
                    ) {
                        let dispatch_record = selected_observation.clone();
                        let dispatch = provider_dispatch
                            .take()
                            .expect("provider cancellation follows the retained dispatch");
                        checkpoint_history.insert(
                            checkpoint_record.coordinate().record_ref().clone(),
                            checkpoint_record.clone(),
                        );
                        let (next_head, next_checkpoint_record, next_checkpoint, reference, next) =
                            append_observation_with_state(
                                &issued,
                                &dispatch_record,
                                &dispatch,
                                &checkpoint,
                                retained.signing,
                                retained.session,
                                OperationObservationStateV1::CancellationRequested {
                                    request: retained_cancellation_request.as_ref().map_or_else(
                                        || ImmutableRecordReferenceV1 {
                                            record_ref: record_ref("restart/cancellation-request"),
                                            record_sha256: artifact_digest('c'),
                                        },
                                        |(_, record)| ImmutableRecordReferenceV1 {
                                            record_ref: record.coordinate().record_ref().clone(),
                                            record_sha256: ArtifactDigest::parse(
                                                record.coordinate().record_sha256().as_str(),
                                            )
                                            .expect(
                                                "provider cancellation request artifact digest",
                                            ),
                                        },
                                    ),
                                },
                            );
                        observation_history.insert(
                            dispatch_record.coordinate().record_ref().clone(),
                            dispatch_record,
                        );
                        if let Some((_, cancellation_record)) =
                            retained_cancellation_request.as_ref()
                        {
                            observation_history.insert(
                                cancellation_record.coordinate().record_ref().clone(),
                                cancellation_record.clone(),
                            );
                        }
                        selected_observation = crate::resident::record_projection::record(
                            crate::resident::record_projection::coordinate(
                                &reference,
                                next.canonical_operation_observation_sha256().as_str(),
                            )
                            .unwrap(),
                            &next,
                        )
                        .unwrap();
                        observation_leaf = Some(
                            crate::resident::checkpoint_profile::operation_observation_membership(
                                &store_coordinate(),
                                &next_checkpoint,
                                &reference,
                                &next,
                            )
                            .unwrap(),
                        );
                        head = next_head;
                        checkpoint_record = next_checkpoint_record;
                        checkpoint = next_checkpoint;
                        assert_eq!(
                            head.domain_sequence().get(),
                            if fault == "restart-provider-cancellation-requested" {
                                3
                            } else {
                                4
                            }
                        );
                    }
                }
                let operation_handle = (issued.handle().0.clone(), issued.handle().1.clone());
                drop(issued);
                assert_eq!(
                    (
                        committing.commits,
                        initial_anchor.advances,
                        checkpoint_signs.get()
                    ),
                    (1, 1, 1)
                );
                let (sessions, mut resolver, lease) = parents(&retained);
                if fault == "missing-parent" {
                    resolver.generations.clear();
                }
                let expected_anchor =
                    AntiRollbackAnchor::ControlDomain(ControlDomainAntiRollbackAnchor::new(
                        store_coordinate().store_id().clone(),
                        store_coordinate().incarnation_id().clone(),
                        head.checkpoint_domain_id().clone(),
                        head.domain_epoch(),
                        head.domain_sequence(),
                        head.coordinate().clone(),
                    ));
                let foreign =
                    AntiRollbackAnchor::ControlDomain(ControlDomainAntiRollbackAnchor::new(
                        store_coordinate().store_id().clone(),
                        store_coordinate().incarnation_id().clone(),
                        if fault == "anchor-wrong-key" {
                            ControlCheckpointDomainIdV1::parse("operation:foreign").unwrap()
                        } else {
                            head.checkpoint_domain_id().clone()
                        },
                        head.domain_epoch(),
                        ControlDomainSequenceV1::new(2).unwrap(),
                        ControlRecordCoordinate::new(
                            record_ref("changed-checkpoint"),
                            ControlRecordDigest::parse(sha('e')).unwrap(),
                        ),
                    ));
                let anchor_replies = if runners.process_host_restart.is_some()
                    || runners.process_host_invoke_request.is_some()
                {
                    vec![Some(expected_anchor.clone()); 20]
                } else if application_control {
                    vec![Some(expected_anchor.clone()); 6]
                } else {
                    vec![
                        if fault == "anchor-missing" {
                            None
                        } else if ["anchor-before", "anchor-wrong-key"].contains(&fault) {
                            Some(foreign.clone())
                        } else {
                            Some(expected_anchor.clone())
                        },
                        Some(if fault == "anchor-after" {
                            foreign
                        } else {
                            expected_anchor.clone()
                        }),
                        Some(expected_anchor.clone()),
                        Some(expected_anchor.clone()),
                    ]
                };
                let recovery_trace = application_control.then(|| Rc::new(RefCell::new(Vec::new())));
                let mut anchor = ReadAnchor {
                    replies: anchor_replies.into(),
                    reads: 0,
                    unavailable: fault == "anchor-unavailable",
                    trace: recovery_trace.clone(),
                };
                let mut store = RecoveryStore {
                    handle_lookup: None,
                    key: crate::resident::operation_custody::operation_request_key(request)
                        .unwrap(),
                    request: (fault != "missing-request").then(|| records[0].record().clone()),
                    handle: (fault != "missing-handle").then(|| records[1].record().clone()),
                    lease: (fault != "missing-lease").then_some(lease),
                    session_basis: Some(session_basis),
                    discovered_session_basis: None,
                    head,
                    checkpoint: checkpoint_record.clone(),
                    initial: initial.unwrap(),
                    start_observation: Some(start_observation),
                    start_checkpoint: Some(start_checkpoint),
                    selected_observation: Some(selected_observation),
                    observation_checkpoint: Some(checkpoint_record),
                    observation_history,
                    checkpoint_history,
                    missing_operation_record: (fault == "restart-provider-cancellation-requested")
                        .then(|| {
                            ControlRecordCoordinate::new(
                                record_ref("restart/cancellation-request"),
                                ControlRecordDigest::parse(sha('c')).unwrap(),
                            )
                        }),
                    attempt_selection,
                    retained_cancellation_request: retained_cancellation_request
                        .as_ref()
                        .map(|(request, _)| request.clone()),
                    observation_leaf,
                    collection_snapshot: consultation
                        .as_ref()
                        .map(|current| current.snapshot().clone()),
                    trace: recovery_trace.clone(),
                    index_reads: 0,
                    lease_reads: 0,
                    observation_body_reads: 0,
                    selection_reads: 0,
                    head_reads: 0,
                    membership_calls: 0,
                    start_reads: 0,
                    repeat_start_source_once: false,
                    start_through_override: None,
                    change_result_limit: fault == "foreign-limit",
                    change_result_leaf: fault == "foreign-leaf",
                    head_changed: fault == "head-before-proof",
                    head_changed_after_memberships: None,
                    start_changed_after_memberships: None,
                };
                if fault == "initial-observation-substituted" {
                    store.start_observation = store.selected_observation.clone();
                }
                if fault == "initial-checkpoint-substituted" {
                    store.start_checkpoint = Some(store.checkpoint.clone());
                }
                if fault == "corrupt-handle" {
                    store.handle = store.handle.as_ref().map(without_signature);
                }
                if let Some(basis_fault) = basis_fault {
                    basis_fault.apply(&mut store);
                }
                if fault == "corrupt-checkpoint" {
                    store.checkpoint = without_signature(&store.checkpoint);
                }
                let supported = if fault == "profile-ref" {
                    crate::resident::SupportedOperationCheckpointProfile::admit(
                        record_ref("other-profile-ref"),
                        &crate::resident::SupportedOperationCheckpointProfile::content_digest()
                            .unwrap(),
                        crate::resident::SupportedOperationCheckpointProfile::canonical_content()
                            .unwrap()
                            .as_bytes(),
                    )
                    .unwrap()
                } else {
                    super::checkpoint_preparation::profile()
                };
                let mut changed = request.to_preimage();
                changed.body.payload = serde_json::json!({"query":"different-content"});
                changed.body.payload_sha256 = ArtifactDigest::hash_bytes(
                    pale_core::canon::to_canonical_string(&changed.body.payload)
                        .unwrap()
                        .as_bytes(),
                );
                let changed = crate::effect_case_lineage_fixture::request(&changed);
                assert_eq!(
                    crate::resident::operation_custody::operation_request_key(&changed).unwrap(),
                    store.key
                );
                assert_eq!(
                    crate::resident::SupportedOperationCheckpointProfile::conflict_domain(&changed)
                        .unwrap(),
                    *checkpoint.checkpoint_domain_id()
                );
                let recovered_request = if fault == "request-conflict" {
                    &changed
                } else {
                    request
                };
                let authority = ControlCheckpointSigningAuthorityAdmissionContextV1 {
                    authority_ref: checkpoint.checkpoint_signing_authority_ref(),
                    authority_sha256: checkpoint.checkpoint_signing_authority_sha256(),
                    principal_id: checkpoint.issuer_principal_id(),
                    control_store_id: checkpoint.control_store_id(),
                    control_incarnation_id: checkpoint.control_incarnation_id(),
                    checkpoint_domain_id: checkpoint.checkpoint_domain_id(),
                    checkpoint_profile_ref: checkpoint.checkpoint_profile_ref(),
                    checkpoint_profile_sha256: checkpoint.checkpoint_profile_sha256(),
                    subject_key_id: checkpoint.issuer_key_id(),
                    subject_key_epoch: *checkpoint.issuer_key_epoch(),
                };
                let verified = Cell::new(0);
                let historical_calls = Cell::new(0);
                if application_control {
                    let trace = recovery_trace
                        .as_ref()
                        .expect("Observe cases carry one shared recovery trace")
                        .clone();
                    let handle_record = store.handle.as_ref().unwrap().clone();
                    let request_record = store.request.as_ref().unwrap().clone();
                    let expected_locator = handle_record.coordinate().clone();
                    store.handle_lookup = Some(handle_recovery::Lookup {
                        expected: expected_locator.clone(),
                        key: crate::resident::operation_observation_custody::operation_handle_key(
                            expected.1.operation_handle_id(),
                        )
                        .unwrap(),
                        handle: Some(handle_record),
                        request: Some(request_record),
                        maximum: 1024 * 1024,
                        wrong_query: false,
                        handle_reads: 0,
                        index_reads: 0,
                    });
                    let control = if runners.resident_program_observe.is_some() {
                        resident_program_observation_request(
                            request,
                            &expected.1,
                            retained.session,
                            retained.current,
                        )
                    } else if runners.resident_program_cancellation.is_some() {
                        resident_program_cancellation_request(
                            request,
                            &expected.1,
                            retained.session,
                            retained.current,
                        )
                    } else if fault.starts_with("cancel-") {
                        cancellation_request(request, &expected.1)
                    } else {
                        observation_request(request, &expected.1)
                    };
                    let historical = request.to_preimage();
                    let request_context = CommonRequestAdmissionContextV1 {
                        expected_schema_sha256: &historical.envelope_schema_sha256,
                        expected_body_schema_id: &historical.body.schema_id,
                        expected_body_schema_sha256: &historical.body.schema_sha256,
                        caller: &historical.caller,
                        signing_authority: &historical.signing_authority_binding,
                        route: &historical.route,
                        session_admission: &historical.session_admission,
                        operation_target: historical.operation_target.as_ref(),
                        execution_owner_fence: None,
                        federation_claim: None,
                        workspace_id: &historical.workspace_id,
                        capability_id: &historical.capability_id,
                        operation: &historical.operation,
                        scope: &historical.scope,
                        risk_class: &historical.risk_class,
                        resource_constraints: &historical.resource_constraints,
                        expected_precondition: None,
                        idempotency_key: &historical.idempotency_key,
                        issued_at: historical.issued_at,
                        expires_at: historical.expires_at,
                        generation: historical.generation.as_ref(),
                        fence_token: historical.fence_token.as_ref(),
                    };
                    let request_calls = Cell::new(0);
                    let caller_signature_calls = Cell::new(0);
                    let handle_authority_calls = Cell::new(0);
                    let handle_signature_calls = Cell::new(0);
                    let checkpoint_signature_calls = Cell::new(0);
                    let observation_signer_calls = Cell::new(0);
                    let observation_state_calls = Cell::new(0);
                    let observation_signature_calls = Cell::new(0);
                    let present_authority_calls = Cell::new(0);
                    let expected_diagnostic = record_ref("diagnostic/retained-failure");
                    let expected_observation_ref = store
                        .selected_observation
                        .as_ref()
                        .unwrap()
                        .coordinate()
                        .record_ref()
                        .clone();

                    let mut admit_request =
                        |reference: &ResidentRecordRefV1, wire: CommonRequestEnvelopeWireV1| {
                            trace.borrow_mut().push("historical-request");
                            request_calls.set(request_calls.get() + 1);
                            if let Some((cancellation, _)) = retained_cancellation_request
                                .as_ref()
                                .filter(|(_, record)| reference == record.coordinate().record_ref())
                            {
                                let preimage = cancellation.to_preimage();
                                return CommonRequestEnvelopeV1::admit_contextual_wire(
                                    wire,
                                    &request_admission_context(&preimage),
                                    |actual_preimage, digest, actual_signature| {
                                        caller_signature_calls
                                            .set(caller_signature_calls.get() + 1);
                                        assert_eq!(actual_preimage, &preimage);
                                        assert_eq!(digest, cancellation.canonical_request_sha256());
                                        assert_eq!(
                                            actual_signature,
                                            cancellation.caller_signature()
                                        );
                                        Ok(())
                                    },
                                );
                            }
                            assert_eq!(reference, records[0].record().coordinate().record_ref());
                            CommonRequestEnvelopeV1::admit_contextual_wire(
                                wire,
                                &request_context,
                                |preimage, digest, actual| {
                                    caller_signature_calls.set(caller_signature_calls.get() + 1);
                                    assert_eq!(preimage, &historical);
                                    assert_eq!(digest, request.canonical_request_sha256());
                                    assert_eq!(actual, request.caller_signature());
                                    Ok(())
                                },
                            )
                        };
                    let mut admit_handle_authority =
                        |_: &OperationHandlePreimageV1, parent: Option<&OperationHandleV1>| {
                            trace.borrow_mut().push("historical-handle");
                            handle_authority_calls.set(handle_authority_calls.get() + 1);
                            assert!(parent.is_none());
                            Ok(())
                        };
                    let mut verify_handle_signature =
                        |domain: &[u8],
                         _: &OperationHandlePreimageV1,
                         digest: &OperationHandleDigest,
                         actual: &CanonicalSignatureV1| {
                            handle_signature_calls.set(handle_signature_calls.get() + 1);
                            assert_eq!(domain, OPERATION_HANDLE_V1_SIGNATURE_DOMAIN);
                            assert_eq!(digest, expected.1.canonical_operation_handle_sha256());
                            assert_eq!(actual, &signature());
                            Ok(())
                        };
                    let mut admit_checkpoint =
                        |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                            ControlStateCheckpointV1::admit_signed_wire(
                                wire,
                                &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
                                &authority,
                                |domain, _, _, actual| {
                                    assert_eq!(
                                        domain,
                                        CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN
                                    );
                                    assert_eq!(actual, &signature());
                                    Ok(())
                                },
                            )
                        };
                    let mut resolve_observation_signer =
                        |_: &ResidentRecordRefV1, wire: &OperationObservationWireV1| {
                            observation_signer_calls.set(observation_signer_calls.get() + 1);
                            if matches!(
                                fault,
                                "restart-provider-resolved-foreign-resolving"
                                    | "restart-provider-resolved-foreign-context"
                            ) && wire.deployment_runtime_authority_ref()
                                == &rotated_signing
                                    .integrated_runtime_authority()
                                    .expect("integrated rotated signing authority")
                                    .deployment_runtime_authority_ref
                            {
                                return Ok(rotated_signing.clone());
                            }
                            assert_eq!(
                                wire.deployment_runtime_authority_ref(),
                                &retained
                                    .signing
                                    .integrated_runtime_authority()
                                    .unwrap()
                                    .deployment_runtime_authority_ref
                            );
                            Ok(retained.signing.clone())
                        };
                    let mut admit_observation_state = |state: &OperationObservationStateV1| {
                        trace.borrow_mut().push("observation");
                        observation_state_calls.set(observation_state_calls.get() + 1);
                        if let OperationObservationStateV1::Failed {
                            result: None,
                            diagnostic,
                        } = state
                        {
                            assert_eq!(&diagnostic.record_ref, &expected_diagnostic);
                        }
                        Ok(())
                    };
                    let mut admit_rotation =
                        |_: &ResidentRecordRefV1,
                         _: &ArtifactDigest,
                         _: &ResidentRecordRefV1,
                         _: &ArtifactDigest| {
                            if matches!(
                                fault,
                                "restart-provider-resolved-foreign-resolving"
                                    | "restart-provider-resolved-foreign-context"
                            ) {
                                Ok(())
                            } else {
                                panic!("fixture does not rotate runtime authority")
                            }
                        };
                    let mut verify_observation_signature =
                        |domain: &[u8],
                         _: &OperationObservationPreimageV1,
                         _: &OperationObservationDigest,
                         actual: &CanonicalSignatureV1| {
                            observation_signature_calls.set(observation_signature_calls.get() + 1);
                            assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
                            assert_eq!(actual, &signature());
                            Ok(())
                        };
                    let mut verify_checkpoint_signature =
                        |domain: &[u8],
                         _: &ControlStateCheckpointPreimageV1,
                         _: &ControlStateCheckpointDigest,
                         actual: &CanonicalSignatureV1| {
                            trace.borrow_mut().push("checkpoint");
                            checkpoint_signature_calls.set(checkpoint_signature_calls.get() + 1);
                            assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                            assert_eq!(actual, &signature());
                            Ok(())
                        };
                    let mut authorize = |context: OperationControlAuthorizationContext<'_>| {
                        present_authority_calls.set(present_authority_calls.get() + 1);
                        let events = trace.borrow();
                        let positions = |name| {
                            events
                                .iter()
                                .enumerate()
                                .filter_map(|(index, event)| (*event == name).then_some(index))
                                .collect::<Vec<_>>()
                        };
                        let requests = positions("historical-request");
                        let handles = positions("historical-handle");
                        let checkpoints = positions("checkpoint");
                        let memberships = positions("membership");
                        let anchors = positions("anchor");
                        let observations = positions("observation");
                        assert_eq!(requests.len(), 1);
                        assert_eq!(handles.len(), 1);
                        assert_eq!(checkpoints.len(), 3);
                        assert_eq!(memberships.len(), 3);
                        assert_eq!(anchors.len(), 6);
                        assert_eq!(observations.len(), 4);
                        assert!(requests[0] < handles[0]);
                        assert!(handles[0] < checkpoints[0]);
                        assert!(checkpoints[0] < anchors[0]);
                        assert!(anchors[0] < memberships[0] && memberships[0] < anchors[1]);
                        assert!(anchors[1] < observations[0]);
                        assert!(observations[1] < checkpoints[1]);
                        assert!(checkpoints[1] < anchors[2]);
                        assert!(anchors[2] < memberships[1] && memberships[1] < anchors[3]);
                        assert!(anchors[3] < observations[2]);
                        assert!(observations[3] < checkpoints[2]);
                        assert!(checkpoints[2] < anchors[4]);
                        assert!(anchors[4] < memberships[2] && memberships[2] < anchors[5]);
                        assert_eq!(events.last(), Some(&"anchor"));
                        drop(events);
                        trace.borrow_mut().push("present-authority");
                        assert_eq!(context.request, &control);
                        assert_eq!(context.original_request, request);
                        assert_eq!(context.operation.0, &expected.0);
                        assert_eq!(context.observation.0, &expected_observation_ref);
                        if matches!(
                            fault,
                            "observe-denied" | "cancel-terminal-denied" | "cancel-active-denied"
                        ) {
                            Err(rejected())
                        } else {
                            Ok(())
                        }
                    };
                    let restart_history_limit = if fault.starts_with("restart-local-cancellation-")
                    {
                        3
                    } else if matches!(
                        fault,
                        "restart-native-prepared"
                            | "restart-provider-prepared"
                            | "restart-provider-resolved-evidence"
                            | "restart-provider-resolved-foreign-resolving"
                            | "restart-provider-resolved-foreign-context"
                            | "restart-native-recoverable-dispatched"
                            | "restart-native-staged"
                            | "restart-provider-dispatched"
                            | "restart-provider-cancellation-requested"
                            | "restart-provider-recoverable-cancellation-requested"
                    ) {
                        if matches!(
                            fault,
                            "restart-native-staged"
                                | "restart-provider-resolved-evidence"
                                | "restart-provider-resolved-foreign-resolving"
                                | "restart-provider-resolved-foreign-context"
                        ) {
                            4
                        } else if fault == "restart-provider-recoverable-cancellation-requested" {
                            5
                        } else if fault == "restart-provider-dispatched" {
                            4
                        } else {
                            3
                        }
                    } else {
                        2
                    };
                    let recovery = OperationControlRecoveryInput {
                        operation: LocalOperationHandleRecoveryInput {
                            context: LocalOperationRecoveryContext {
                                store: &store_coordinate(),
                                profile: &supported,
                                handle_signing_authority: retained.signing,
                                checkpoint_authority: &authority,
                                historical: &resolver,
                                maximum_record_bytes: 1024 * 1024,
                                maximum_operation_lease_bytes: 1024 * 1024,
                                maximum_session_lease_bytes: 1024 * 1024,
                                membership_limits: AccumulatorMembershipLimits::new(
                                    3, 10_000, 100_000,
                                )
                                .unwrap(),
                                observation_limits: OperationObservationRecoveryLimits::new(
                                    restart_history_limit,
                                    1024 * 1024,
                                    1024 * 1024,
                                )
                                .unwrap(),
                                checkpoint_limits: OperationCheckpointAncestryLimits::new(
                                    restart_history_limit,
                                    1024 * 1024,
                                    restart_history_limit,
                                )
                                .unwrap(),
                            },
                            expected_handle: &expected_locator,
                            maximum_parent_depth: 0,
                        },
                        observation: OperationObservationSelection::SelectedHead,
                        observation_limits: OperationObservationRecoveryLimits::new(
                            restart_history_limit,
                            1024 * 1024,
                            1024 * 1024,
                        )
                        .unwrap(),
                        checkpoint_limits: OperationCheckpointAncestryLimits::new(
                            restart_history_limit,
                            1024 * 1024,
                            restart_history_limit,
                        )
                        .unwrap(),
                        membership_limits: AccumulatorMembershipLimits::new(1, 10_000, 100_000)
                            .unwrap(),
                    };
                    let recovery_admission = OperationControlRecoveryAdmission {
                        admit_request: &mut admit_request,
                        admit_handle_authority: &mut admit_handle_authority,
                        verify_handle_signature: &mut verify_handle_signature,
                        admit_checkpoint: &mut admit_checkpoint,
                        resolve_observation_signer: &mut resolve_observation_signer,
                        admit_observation_state: &mut admit_observation_state,
                        admit_rotation: &mut admit_rotation,
                        verify_observation_signature: &mut verify_observation_signature,
                        verify_checkpoint_signature: &mut verify_checkpoint_signature,
                    };
                    if let Some(run) = runners.process_host_invoke_request.as_deref_mut() {
                        run(ProcessHostInvokeRequestFixture {
                            recovery,
                            admission: recovery_admission,
                            request,
                            session: retained.session,
                            current: retained.current,
                            signing_authority: retained.signing,
                            generation: restart_generations
                                .as_mut()
                                .and_then(Iterator::next)
                                .expect("one exact request-resolution generation per fixture"),
                            custody: &mut store,
                            sessions: &sessions,
                            anchor: &mut anchor,
                        });
                        assert_eq!(
                            (
                                committing.commits,
                                initial_anchor.advances,
                                checkpoint_signs.get()
                            ),
                            (1, 1, 1),
                            "request resolution performs no custody write"
                        );
                        return;
                    }
                    if let Some(run) = runners.process_host_restart.as_deref_mut() {
                        let evidence =
                            crate::resident::operation_custody::read_operation_start_for_request(
                                &mut store,
                                &store_coordinate(),
                                request,
                                1024 * 1024,
                            )
                            .expect("restart start-evidence read")
                            .expect("existing start evidence");
                        run(ProcessHostRestartFixture {
                            recovery,
                            admission: recovery_admission,
                            signing_authority: retained.signing,
                            session: retained.session,
                            current: retained.current,
                            request,
                            native_registry: &native_registry,
                            evidence,
                            generation: restart_generations
                                .as_mut()
                                .and_then(Iterator::next)
                                .expect("one exact restart generation per fixture"),
                            rediscovery_generation: fault
                                .starts_with("restart-local-cancellation-")
                                .then(|| {
                                    restart_generations
                                        .as_mut()
                                        .and_then(Iterator::next)
                                        .expect("one exact rediscovery generation per fixture")
                                }),
                            custody: &mut store,
                            sessions: &sessions,
                            anchor: &mut anchor,
                        });
                        assert_eq!(
                            (
                                committing.commits,
                                initial_anchor.advances,
                                checkpoint_signs.get()
                            ),
                            (1, 1, 1),
                            "restart classification performs no custody write"
                        );
                        return;
                    }
                    if fault.starts_with("cancel-")
                        && !fault.starts_with("cancel-active")
                        && !fault.starts_with("cancel-collecting-live")
                    {
                        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
                        let mut reject_rotation =
                            |_: &ResidentRecordRefV1,
                             _: &ArtifactDigest,
                             _: &ResidentRecordRefV1,
                             _: &ArtifactDigest| {
                                panic!("terminal cancellation must not issue an observation")
                            };
                        let mut reject_observation_sign =
                            |_: &[u8],
                             _: &OperationObservationPreimageV1,
                             _: &OperationObservationDigest| {
                                panic!("terminal cancellation must not sign an observation")
                            };
                        let mut reject_checkpoint_sign =
                            |_: &[u8],
                             _: &ControlStateCheckpointPreimageV1,
                             _: &ControlStateCheckpointDigest| {
                                panic!("terminal cancellation must not sign a checkpoint")
                            };
                        let resolve_store = StartStore(retained.session);
                        let input = ResidentOperationCancellationInput {
                            recovery,
                            request: &control,
                            authorized_at: UnixTimeV1::new(control.issued_at().get() + 1).unwrap(),
                            request_ref: record_ref("request/cancel-terminal"),
                            observation_ref: record_ref("observation/cancel-terminal"),
                            signing: SignedResidentAdmissionContextV1 {
                                expected_schema_sha256: &observation_schema,
                                signing_authority: retained.signing,
                            },
                            checkpoint: OperationCheckpointInput {
                                reference: record_ref("checkpoint/cancel-terminal"),
                                id: ControlCheckpointIdV1::parse("cancel-terminal").unwrap(),
                            },
                            checkpoint_issued_at: UnixTimeV1::new(control.issued_at().get() + 2)
                                .unwrap(),
                            checkpoint_schema: checkpoint.schema_sha256(),
                            checkpoint_authority: &authority,
                            accumulator_limits: AccumulatorPreparationLimits::new(
                                1, 10_000, 10_000, 4096, 100_000,
                            )
                            .unwrap(),
                        };
                        let admission = ResidentOperationCancellationAdmission {
                            recovery: recovery_admission,
                            authorize: &mut authorize,
                            resolve_store: &resolve_store,
                            admit_rotation: &mut reject_rotation,
                            sign_observation: &mut reject_observation_sign,
                            sign_checkpoint: &mut reject_checkpoint_sign,
                        };
                        if let Some(run) = runners.resident_program_cancellation.as_deref_mut() {
                            run(ResidentProgramCancellationFixture {
                                cancellation: ProcessHostCancellationFixture {
                                    input,
                                    admission,
                                    signing_authority: retained.signing,
                                    operation_handle: operation_handle.clone(),
                                    custody: &mut store,
                                    sessions: &sessions,
                                    anchor: &mut anchor,
                                },
                                session: retained.session,
                                current: retained.current,
                                generation: restart_generations
                                    .as_mut()
                                    .and_then(Iterator::next)
                                    .expect("one resident-program generation per Cancel fixture"),
                            });
                            return;
                        }
                        let result = match runners.process_host_cancellation.as_deref_mut() {
                            Some(run) => run(ProcessHostCancellationFixture {
                                input,
                                admission,
                                signing_authority: retained.signing,
                                operation_handle: operation_handle.clone(),
                                custody: &mut store,
                                sessions: &sessions,
                                anchor: &mut anchor,
                            }),
                            None => {
                                let mut live_authority = ProvingAuthority;
                                let mut tasks = ResidentOperationTaskState::new(
                                    ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                                    NonZeroUsize::new(1).expect("consultation batch bound"),
                                    ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                                )
                                .unwrap_or_else(|_| panic!("one bounded task slot must allocate"));
                                let mut service = ResidentOperationService::new(
                                    &mut live_authority,
                                    &mut store,
                                    &mut anchor,
                                    &mut tasks,
                                );
                                service.request_cancellation(input, None, &sessions, admission)
                            }
                        };
                        assert!(matches!(
                            (fault, result),
                            (
                                "cancel-terminal-allowed",
                                Ok(ResidentOperationCancellationOutcome::Established(_)),
                            ) | (
                                "cancel-already-requested",
                                Ok(ResidentOperationCancellationOutcome::AlreadyRequested(_)),
                            ) | (
                                "cancel-provider-dispatched",
                                Ok(ResidentOperationCancellationOutcome::DispatchOwnerRequired(
                                    _
                                )),
                            ) | (
                                "cancel-before-dispatch-consult",
                                Ok(ResidentOperationCancellationOutcome::DispatchOwnerRequired(
                                    _
                                )),
                            ) | (
                                "cancel-collecting",
                                Err(
                                    ResidentOperationCancellationFailure::ConsultationRouteRequired
                                ),
                            ) | (
                                "cancel-collected",
                                Ok(ResidentOperationCancellationOutcome::ConsultationCollected(
                                    _
                                )),
                            ) | (
                                "cancel-reconciliation",
                                Ok(ResidentOperationCancellationOutcome::ReconciliationOnly(_)),
                            ) | (
                                "cancel-terminal-denied",
                                Err(ResidentOperationCancellationFailure::Authorization(
                                    OperationControlError::Admission(_),
                                )),
                            )
                        ));
                    } else if fault.starts_with("cancel-active")
                        || fault.starts_with("cancel-collecting-live")
                    {
                        let completes_locally = (runners.process_host_cancellation.is_some()
                            || runners.resident_program_cancellation.is_some())
                            && fault == "cancel-active-allowed";
                        let mut append_store =
                            if fault == "cancel-collecting-live-database-retry" {
                                RecordingStore::failing_commit(
                                    ControlStoreError::new(ControlStoreErrorKind::Storage),
                                    [
                                        Ok(OperationCustodyReadBackOutcome::Predecessor),
                                        Err(ControlStoreError::new(ControlStoreErrorKind::Storage)),
                                        Ok(OperationCustodyReadBackOutcome::ExactSuccessor),
                                    ],
                                )
                            } else if fault == "cancel-active-database-failed" {
                                RecordingStore::failing_commit(
                                    ControlStoreError::new(ControlStoreErrorKind::Storage),
                                    [Ok(OperationCustodyReadBackOutcome::Predecessor)],
                                )
                            } else {
                                RecordingStore::committing(OperationCustodyCommitOutcome::Committed)
                            }
                            .with_preparation(
                                if fault == "cancel-collecting-live-preparation-refused" {
                                    reject_cancellation_accumulator_preparation
                                } else {
                                    cancellation_accumulator_response
                                },
                                *checkpoint.committed_transaction_sequence(),
                            );
                        if completes_locally {
                            append_store = append_store.with_committed_follow_up();
                        }
                        let mut append_anchor = if fault == "cancel-collecting-live-database-retry"
                        {
                            SequencedAppendAnchor::advancing_after_database_recovery()
                        } else if fault == "cancel-collecting-live-anchor-retry" {
                            SequencedAppendAnchor::unavailable_then_advancing()
                        } else if fault == "cancel-active-anchor-failed" {
                            SequencedAppendAnchor::unavailable()
                        } else {
                            SequencedAppendAnchor::advancing()
                        };
                        if completes_locally {
                            append_anchor = append_anchor.with_advancing_follow_up();
                        }
                        let mut joined_store = RecoveryAppendStore {
                            recovery: &mut store,
                            append: &mut append_store,
                        };
                        let mut joined_anchor = RecoveryAppendAnchor {
                            recovery: &mut anchor,
                            append: &mut append_anchor,
                        };
                        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
                        let mut admit_rotation =
                            |_: &ResidentRecordRefV1,
                             _: &ArtifactDigest,
                             _: &ResidentRecordRefV1,
                             _: &ArtifactDigest| { Ok(()) };
                        let mut sign_observation =
                            |_: &[u8],
                             _: &OperationObservationPreimageV1,
                             _: &OperationObservationDigest| {
                                Ok(signature())
                            };
                        let mut sign_checkpoint =
                            |_: &[u8],
                             _: &ControlStateCheckpointPreimageV1,
                             _: &ControlStateCheckpointDigest| {
                                Ok(signature())
                            };
                        let resolve_store = StartStore(retained.session);
                        let input = ResidentOperationCancellationInput {
                            recovery,
                            request: &control,
                            authorized_at: UnixTimeV1::new(control.issued_at().get() + 1).unwrap(),
                            request_ref: record_ref("request/cancel-active"),
                            observation_ref: record_ref("observation/cancel-active"),
                            signing: SignedResidentAdmissionContextV1 {
                                expected_schema_sha256: &observation_schema,
                                signing_authority: retained.signing,
                            },
                            checkpoint: OperationCheckpointInput {
                                reference: record_ref("checkpoint/cancel-active"),
                                id: ControlCheckpointIdV1::parse("cancel-active").unwrap(),
                            },
                            checkpoint_issued_at: UnixTimeV1::new(control.issued_at().get() + 2)
                                .unwrap(),
                            checkpoint_schema: checkpoint.schema_sha256(),
                            checkpoint_authority: &authority,
                            accumulator_limits: AccumulatorPreparationLimits::new(
                                1, 10_000, 10_000, 4096, 100_000,
                            )
                            .unwrap(),
                        };
                        let admission = ResidentOperationCancellationAdmission {
                            recovery: recovery_admission,
                            authorize: &mut authorize,
                            resolve_store: &resolve_store,
                            admit_rotation: &mut admit_rotation,
                            sign_observation: &mut sign_observation,
                            sign_checkpoint: &mut sign_checkpoint,
                        };
                        if let Some(run) = runners.resident_program_cancellation.as_deref_mut() {
                            run(ResidentProgramCancellationFixture {
                                cancellation: ProcessHostCancellationFixture {
                                    input,
                                    admission,
                                    signing_authority: retained.signing,
                                    operation_handle: operation_handle.clone(),
                                    custody: &mut joined_store,
                                    sessions: &sessions,
                                    anchor: &mut joined_anchor,
                                },
                                session: retained.session,
                                current: retained.current,
                                generation: restart_generations
                                    .as_mut()
                                    .and_then(Iterator::next)
                                    .expect("one resident-program generation per Cancel fixture"),
                            });
                            return;
                        }
                        let result = match runners.process_host_cancellation.as_deref_mut() {
                            Some(run) => run(ProcessHostCancellationFixture {
                                input,
                                admission,
                                signing_authority: retained.signing,
                                operation_handle: operation_handle.clone(),
                                custody: &mut joined_store,
                                sessions: &sessions,
                                anchor: &mut joined_anchor,
                            }),
                            None => {
                                let mut live_authority = ProvingAuthority;
                                let mut tasks = ResidentOperationTaskState::new(
                                    ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                                    NonZeroUsize::new(1).expect("consultation batch bound"),
                                    ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                                )
                                .unwrap_or_else(|_| panic!("one bounded task slot must allocate"));
                                let mut service = ResidentOperationService::new(
                                    &mut live_authority,
                                    &mut joined_store,
                                    &mut joined_anchor,
                                    &mut tasks,
                                );
                                service.request_cancellation(input, None, &sessions, admission)
                            }
                        };
                        if fault == "cancel-collecting-live-preparation-refused" {
                            assert!(matches!(
                                result,
                                Err(
                                    ResidentOperationCancellationFailure::ConsultationRouteRequired
                                )
                            ));
                            assert_eq!(append_store.mutation_counts(), (0, 0));
                            assert_eq!(append_anchor.access_counts(), (0, 0));
                        } else if fault == "cancel-active-allowed"
                            || fault == "cancel-collecting-live"
                            || fault.starts_with("cancel-collecting-live-")
                        {
                            if completes_locally {
                                match result {
                                    Ok(
                                        ResidentOperationCancellationOutcome::LocallyCancelled {
                                            ..
                                        },
                                    ) => {}
                                    Err(ResidentOperationCancellationFailure::Completion(
                                        error,
                                    )) => {
                                        panic!("local completion preparation failed: {error:?}")
                                    }
                                    Err(ResidentOperationCancellationFailure::Custody(error)) => {
                                        panic!("local completion custody failed: {error:?}")
                                    }
                                    _ => {
                                        panic!("process-host cancellation did not complete locally")
                                    }
                                }
                            } else {
                                assert!(matches!(
                                    result,
                                    Ok(ResidentOperationCancellationOutcome::LocalRequestRetained {
                                        ..
                                    })
                                ));
                            }
                            let expected_store_counts =
                                if fault == "cancel-collecting-live-database-retry" {
                                    (1, 3)
                                } else if completes_locally {
                                    (2, 2)
                                } else {
                                    (1, 1)
                                };
                            let expected_anchor_counts =
                                if fault == "cancel-collecting-live-anchor-retry" {
                                    (2, 1)
                                } else if fault == "cancel-collecting-live-database-retry" {
                                    (1, 2)
                                } else if completes_locally {
                                    (2, 2)
                                } else {
                                    (1, 1)
                                };
                            assert_eq!(append_store.mutation_counts(), expected_store_counts);
                            assert_eq!(append_anchor.access_counts(), expected_anchor_counts);
                        } else if fault == "cancel-active-denied" {
                            assert!(matches!(
                                result,
                                Err(ResidentOperationCancellationFailure::Authorization(
                                    OperationControlError::Admission(_)
                                ))
                            ));
                            assert_eq!(append_store.mutation_counts(), (0, 0));
                            assert_eq!(append_anchor.access_counts(), (0, 0));
                        } else if fault == "cancel-active-database-failed" {
                            assert!(matches!(
                                result,
                                Err(ResidentOperationCancellationFailure::Custody(
                                    OperationAppendFailure::Commit { .. }
                                ))
                            ));
                            assert_eq!(append_store.mutation_counts(), (1, 1));
                            assert_eq!(append_anchor.access_counts(), (0, 1));
                        } else {
                            assert!(matches!(
                                result,
                                Err(ResidentOperationCancellationFailure::Custody(
                                    OperationAppendFailure::Anchor { .. }
                                ))
                            ));
                            assert_eq!(append_store.mutation_counts(), (1, 1));
                            assert_eq!(append_anchor.access_counts(), (1, 1));
                        }
                    } else {
                        let input = LocalOperationObservationInput {
                            recovery,
                            request: &control,
                            authorized_at: UnixTimeV1::new(control.issued_at().get() + 1).unwrap(),
                        };
                        let admission = LocalOperationObservationAdmission {
                            recovery: recovery_admission,
                            authorize: &mut authorize,
                        };
                        if let Some(run) = runners.resident_program_observe.as_deref_mut() {
                            run(ResidentProgramObserveFixture {
                                observation: ProcessHostObserveFixture {
                                    input,
                                    admission,
                                    signing_authority: retained.signing,
                                    custody: &mut store,
                                    sessions: &sessions,
                                    anchor: &mut anchor,
                                },
                                session: retained.session,
                                current: retained.current,
                                generation: restart_generations
                                    .as_mut()
                                    .and_then(Iterator::next)
                                    .expect("one resident-program generation per Observe fixture"),
                                reconstruction: observe_reconstruction
                                    .take()
                                    .expect("one exact Observe reconstruction input"),
                            });
                            return;
                        }
                        let result = match runners.process_host_observe.as_deref_mut() {
                            Some(run) => run(ProcessHostObserveFixture {
                                input,
                                admission,
                                signing_authority: retained.signing,
                                custody: &mut store,
                                sessions: &sessions,
                                anchor: &mut anchor,
                            }),
                            None => {
                                let mut live_authority = ProvingAuthority;
                                let mut tasks = ResidentOperationTaskState::new(
                                    ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                                    NonZeroUsize::new(1).expect("consultation batch bound"),
                                    ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                                )
                                .unwrap_or_else(|_| panic!("one bounded task slot must allocate"));
                                let mut service = ResidentOperationService::new(
                                    &mut live_authority,
                                    &mut store,
                                    &mut anchor,
                                    &mut tasks,
                                );
                                service.observe(input, &sessions, admission)
                            }
                        };
                        if fault == "observe-allowed" {
                            let observed = result.expect("present authority admitted observation");
                            assert_eq!(observed.operation(), (&expected.0, &expected.1));
                            assert_eq!(
                                observed.observation().0,
                                store
                                    .selected_observation
                                    .as_ref()
                                    .unwrap()
                                    .coordinate()
                                    .record_ref()
                            );
                            assert!(matches!(
                                observed.observation().1.state(),
                                OperationObservationStateV1::Failed {
                                    result: None,
                                    diagnostic
                                } if diagnostic.record_ref == expected_diagnostic
                            ));
                        } else {
                            assert!(matches!(
                                result,
                                Err(LocalOperationObservationError::Authorization(
                                    OperationControlError::Admission(_)
                                ))
                            ));
                        }
                    }
                    assert_eq!(
                        (
                            request_calls.get(),
                            caller_signature_calls.get(),
                            handle_authority_calls.get(),
                            handle_signature_calls.get(),
                            observation_signer_calls.get(),
                            observation_state_calls.get(),
                            observation_signature_calls.get(),
                            checkpoint_signature_calls.get(),
                            present_authority_calls.get(),
                        ),
                        (1, 1, 1, 1, 4, 4, 4, 3, 1)
                    );
                    assert_eq!(
                        (
                            store.index_reads,
                            store.lease_reads,
                            store.observation_body_reads,
                            store.head_reads,
                            store.membership_calls,
                            sessions.1.load(Ordering::Relaxed),
                            anchor.reads,
                        ),
                        (
                            if fault == "cancel-already-requested" {
                                // The one protected cancellation read also authenticates
                                // the immediate predecessor checkpoint for owner handoff.
                                12
                            } else {
                                11
                            },
                            1,
                            2,
                            3,
                            3,
                            1,
                            6,
                        )
                    );
                    assert_eq!(
                        (
                            committing.commits,
                            initial_anchor.advances,
                            checkpoint_signs.get()
                        ),
                        (1, 1, 1),
                        "Observe performs no custody write; counts are only fixture construction"
                    );
                    return;
                }
                if from_handle {
                    handle_recovery::check(
                        handle_recovery::Case {
                            fault,
                            request,
                            substitute: &changed,
                            expected: &expected,
                            retained: &retained,
                            profile: &supported,
                            authority: &authority,
                            resolver: &resolver,
                        },
                        &mut store,
                        &sessions,
                        &mut anchor,
                    );
                    assert_eq!(
                        (
                            committing.commits,
                            initial_anchor.advances,
                            checkpoint_signs.get()
                        ),
                        (1, 1, 1)
                    );
                    return;
                }
                let result = recover_local_operation(
                    LocalOperationRecoveryContext {
                        store: &store_coordinate(),
                        profile: &supported,
                        handle_signing_authority: retained.signing,
                        checkpoint_authority: &authority,
                        historical: &resolver,
                        maximum_record_bytes: if fault == "zero-bound" {
                            0
                        } else {
                            1024 * 1024
                        },
                        maximum_operation_lease_bytes: 1024 * 1024,
                        maximum_session_lease_bytes: 1024 * 1024,
                        membership_limits: AccumulatorMembershipLimits::new(3, 10_000, 100_000)
                            .unwrap(),
                        observation_limits: OperationObservationRecoveryLimits::new(
                            if fault == "initial-observation-revision-bound" {
                                1
                            } else {
                                4
                            },
                            1024 * 1024,
                            1024 * 1024,
                        )
                        .unwrap(),
                        checkpoint_limits: OperationCheckpointAncestryLimits::new(
                            if fault == "initial-checkpoint-bound" {
                                1
                            } else {
                                4
                            },
                            1024 * 1024,
                            4,
                        )
                        .unwrap(),
                    },
                    recovered_request,
                    &mut store,
                    &sessions,
                    &mut anchor,
                    LocalOperationRecoveryAdmission {
                        admit_historical_authority:
                            |_: &OperationHandlePreimageV1, parent: Option<&OperationHandleV1>| {
                                historical_calls.set(historical_calls.get() + 1);
                                assert!(parent.is_none());
                                if fault == "historical-authority" {
                                    Err(rejected())
                                } else {
                                    Ok(())
                                }
                            },
                        verify_handle_signature:
                            |domain: &[u8],
                             _: &OperationHandlePreimageV1,
                             _: &OperationHandleDigest,
                             _: &CanonicalSignatureV1| {
                                assert_eq!(domain, OPERATION_HANDLE_V1_SIGNATURE_DOMAIN);
                                verified.set(verified.get() + 1);
                                if fault == "handle-signature" {
                                    Err(rejected())
                                } else {
                                    Ok(())
                                }
                            },
                        verify_checkpoint_signature:
                            |domain: &[u8],
                             _: &ControlStateCheckpointPreimageV1,
                             _: &ControlStateCheckpointDigest,
                             _: &CanonicalSignatureV1| {
                                assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                                verified.set(verified.get() + 1);
                                if fault == "checkpoint-signature" {
                                    Err(rejected())
                                } else {
                                    Ok(())
                                }
                            },
                        initial_observation: OperationObservationRecoveryAdmission {
                            admit_checkpoint:
                                |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                                    ControlStateCheckpointV1::admit_signed_wire(
                                        wire,
                                        &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
                                        &authority,
                                        |domain, _, _, actual| {
                                            assert_eq!(
                                                domain,
                                                CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN
                                            );
                                            assert_eq!(actual, &signature());
                                            Ok(())
                                        },
                                    )
                                },
                            resolve_signer:
                                |_: &ResidentRecordRefV1, wire: &OperationObservationWireV1| {
                                    if fault == "initial-observation-signer" {
                                        return Err(rejected());
                                    }
                                    if fault == "initial-observation-rotation"
                                        && wire.revision().get() > 1
                                    {
                                        Ok(rotated_signing.clone())
                                    } else {
                                        Ok(retained.signing.clone())
                                    }
                                },
                            admit_state: |_: &OperationObservationStateV1| Ok(()),
                            admit_rotation:
                                |_: &ResidentRecordRefV1,
                                 _: &ArtifactDigest,
                                 _: &ResidentRecordRefV1,
                                 _: &ArtifactDigest| {
                                    assert_eq!(fault, "initial-observation-rotation");
                                    Err(rejected())
                                },
                            verify_signature:
                                |domain: &[u8],
                                 _: &OperationObservationPreimageV1,
                                 _: &OperationObservationDigest,
                                 actual: &CanonicalSignatureV1| {
                                    assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
                                    assert_eq!(actual, &signature());
                                    if fault == "initial-observation-signature" {
                                        Err(rejected())
                                    } else {
                                        Ok(())
                                    }
                                },
                        },
                    },
                );
                if ["none", "after-observation"].contains(&fault) {
                    let recovered = result.unwrap();
                    assert_eq!(recovered.handle(), (&expected.0, &expected.1));
                    super::local_start::assert_publication_policy(&recovered, request, fault);
                    crate::local_operation_start_fixture::assert_original_session_revalidation(
                        &recovered, &retained,
                    );
                    assert_eq!(
                        (store.membership_calls, anchor.reads, verified.get()),
                        (2, 4, 3)
                    );
                    drop(recovered);
                    anchor
                        .replies
                        .extend(vec![Some(expected_anchor.clone()); 4]);
                    let repeated = recover_local_operation(
                        LocalOperationRecoveryContext {
                            store: &store_coordinate(),
                            profile: &supported,
                            handle_signing_authority: retained.signing,
                            checkpoint_authority: &authority,
                            historical: &resolver,
                            maximum_record_bytes: 1024 * 1024,
                            maximum_operation_lease_bytes: 1024 * 1024,
                            maximum_session_lease_bytes: 1024 * 1024,
                            membership_limits: AccumulatorMembershipLimits::new(3, 10_000, 100_000)
                                .unwrap(),
                            observation_limits: OperationObservationRecoveryLimits::new(
                                4,
                                1024 * 1024,
                                1024 * 1024,
                            )
                            .unwrap(),
                            checkpoint_limits: OperationCheckpointAncestryLimits::new(
                                4,
                                1024 * 1024,
                                4,
                            )
                            .unwrap(),
                        },
                        request,
                        &mut store,
                        &sessions,
                        &mut anchor,
                        LocalOperationRecoveryAdmission {
                        admit_historical_authority: |_: &OperationHandlePreimageV1, parent: Option<&OperationHandleV1>| {
                            historical_calls.set(historical_calls.get() + 1);
                            assert!(parent.is_none());
                            Ok(())
                        },
                        verify_handle_signature: |domain: &[u8], _: &OperationHandlePreimageV1, _: &OperationHandleDigest, _: &CanonicalSignatureV1| {
                            assert_eq!(domain, OPERATION_HANDLE_V1_SIGNATURE_DOMAIN);
                            verified.set(verified.get() + 1);
                            Ok(())
                        },
                        verify_checkpoint_signature: |domain: &[u8], _: &ControlStateCheckpointPreimageV1, _: &ControlStateCheckpointDigest, _: &CanonicalSignatureV1| {
                            assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                            verified.set(verified.get() + 1);
                            Ok(())
                        },
                        initial_observation: OperationObservationRecoveryAdmission {
                            admit_checkpoint: |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                                ControlStateCheckpointV1::admit_signed_wire(
                                    wire,
                                    &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
                                    &authority,
                                    |domain, _, _, actual| {
                                        assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                                        assert_eq!(actual, &signature());
                                        Ok(())
                                    },
                                )
                            },
                            resolve_signer: |_: &ResidentRecordRefV1, _: &OperationObservationWireV1| Ok(retained.signing.clone()),
                            admit_state: |_: &OperationObservationStateV1| Ok(()),
                            admit_rotation: |_: &ResidentRecordRefV1,
                                             _: &ArtifactDigest,
                                             _: &ResidentRecordRefV1,
                                             _: &ArtifactDigest| {
                                panic!("recovery fixture does not rotate runtime authority")
                            },
                            verify_signature: |domain: &[u8], _: &OperationObservationPreimageV1, _: &OperationObservationDigest, actual: &CanonicalSignatureV1| {
                                assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
                                assert_eq!(actual, &signature());
                                Ok(())
                            },
                        },
                        },
                    )
                    .unwrap();
                    assert_eq!(repeated.handle(), (&expected.0, &expected.1));
                    super::local_start::assert_publication_policy(&repeated, request, fault);
                    assert_eq!(
                        (store.membership_calls, anchor.reads, verified.get()),
                        (4, 8, 6)
                    );
                    assert_eq!(
                        (
                            store.index_reads,
                            store.lease_reads,
                            sessions.1.load(Ordering::Relaxed),
                            historical_calls.get(),
                            store.head_reads
                        ),
                        (16, 2, 2, 2, 4)
                    );
                } else {
                    let Err(error) = result else {
                        panic!("{fault}: unexpectedly recovered")
                    };
                    // Index, lease, session, historical authority, head, signature,
                    // anchor and membership stages must reach precisely this fence.
                    let expected = if let Some(basis_fault) = basis_fault {
                        basis_fault.expected_stages(&error)
                    } else {
                        match (fault, &error) {
                        ("zero-bound", LocalOperationRecoveryError::Bounds)
                        | ("profile-ref", LocalOperationRecoveryError::Association) => [0; 8],
                        ("request-conflict", LocalOperationRecoveryError::Lookup(
                            crate::resident::OperationRecoveryError::RequestConflict))
                        | ("missing-request" | "missing-handle", LocalOperationRecoveryError::Lookup(
                            crate::resident::OperationRecoveryError::IncompleteRequestIndexes)) => [2, 0, 0, 0, 0, 0, 0, 0],
                        ("corrupt-handle", LocalOperationRecoveryError::Decode(value)) if value.is_data() => [4, 0, 0, 0, 0, 0, 0, 0],
                        ("missing-lease", LocalOperationRecoveryError::Lease(
                            crate::resident::OperationLeaseBodyRecoveryError::Missing)) => [4, 1, 0, 0, 0, 0, 0, 0],
                        ("missing-parent", LocalOperationRecoveryError::Lease(
                            crate::resident::OperationLeaseBodyRecoveryError::ParentSession(
                                crate::resident::GenerationLifecycleError::MissingRecoveryContext("session compiled generation")))) => [4, 1, 1, 0, 0, 0, 0, 0],
                        ("historical-authority" | "handle-signature" | "checkpoint-signature", LocalOperationRecoveryError::Record(
                            InvalidResidentRecordV1::ContextualAdmission {
                                record: "recovery test",
                                invariant: "resolved historical authority or verifier rejected",
                            })) => match fault {
                                "historical-authority" => [4, 1, 1, 1, 0, 0, 0, 0],
                                "handle-signature" => [4, 1, 1, 1, 0, 1, 0, 0],
                                _ => [4, 1, 1, 1, 1, 2, 0, 0],
                            },
                        ("corrupt-checkpoint", LocalOperationRecoveryError::Decode(value)) if value.is_data() => [4, 1, 1, 1, 1, 1, 0, 0],
                        ("anchor-missing", LocalOperationRecoveryError::Missing)
                        | ("anchor-before" | "anchor-wrong-key", LocalOperationRecoveryError::Association) => [4, 1, 1, 1, 1, 2, 1, 0],
                        ("anchor-unavailable", LocalOperationRecoveryError::Anchor(value)) if value.kind() == AntiRollbackAnchorErrorKind::Unavailable => [4, 1, 1, 1, 1, 2, 1, 0],
                        ("anchor-after", LocalOperationRecoveryError::Association) => [4, 1, 1, 1, 1, 2, 2, 1],
                        ("head-before-proof", LocalOperationRecoveryError::Membership(
                            OperationAccumulatorPreparationError::Store(value))) if value.kind() == ControlStoreErrorKind::CandidateConflict => [4, 1, 1, 1, 1, 2, 1, 1],
                        ("foreign-limit" | "foreign-leaf", LocalOperationRecoveryError::Association) => [4, 1, 1, 1, 1, 2, 1, 1],
                        (
                            "initial-observation-substituted" | "initial-checkpoint-substituted",
                            LocalOperationRecoveryError::Association,
                        ) => [8, 1, 1, 1, 2, 3, 4, 2],
                        (
                            "initial-observation-signer"
                            | "initial-observation-signature"
                            | "initial-observation-rotation",
                            LocalOperationRecoveryError::Record(
                                InvalidResidentRecordV1::ContextualAdmission {
                                    record: "recovery test",
                                    invariant:
                                        "resolved historical authority or verifier rejected",
                                },
                            ),
                        ) => [7, 1, 1, 1, 1, 2, 2, 1],
                        (
                            "initial-observation-revision-bound",
                            LocalOperationRecoveryError::Bounds,
                        ) => [7, 1, 1, 1, 1, 2, 2, 1],
                        (
                            "initial-checkpoint-bound",
                            LocalOperationRecoveryError::Bounds,
                        ) => [8, 1, 1, 1, 2, 3, 3, 1],
                        _ => panic!("{fault}: wrong owning disposition: {error:?}"),
                    }
                    };
                    assert_eq!(
                        [
                            store.index_reads,
                            store.lease_reads,
                            sessions.1.load(Ordering::Relaxed),
                            historical_calls.get(),
                            store.head_reads,
                            verified.get(),
                            anchor.reads,
                            store.membership_calls
                        ],
                        expected,
                        "{fault}: evidence stages"
                    );
                }
                assert_eq!(
                    (
                        committing.commits,
                        initial_anchor.advances,
                        checkpoint_signs.get()
                    ),
                    (1, 1, 1)
                );
            },
        );
    }
}

fn rejected() -> InvalidResidentRecordV1 {
    InvalidResidentRecordV1::ContextualAdmission {
        record: "recovery test",
        invariant: "resolved historical authority or verifier rejected",
    }
}

fn without_signature(record: &ControlRecord) -> ControlRecord {
    let mut value =
        pale_core::canon::parse_strict(record.canonical_record_jcs().as_slice()).unwrap();
    assert!(value.as_object_mut().unwrap().remove("signature").is_some());
    ControlRecord::new(
        record.coordinate().clone(),
        record.schema_id().clone(),
        record.schema_sha256().clone(),
        ControlRecordBytes::new(
            pale_core::canon::to_canonical_string(&value)
                .unwrap()
                .into_bytes(),
        )
        .unwrap(),
    )
}
