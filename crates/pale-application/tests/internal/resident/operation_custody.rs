//! Behavioral laws for application-owned operation-start custody.

use std::collections::VecDeque;

use pale_core::records::{
    ControlCheckpointDomainIdV1, ControlDomainEpochV1, ControlDomainSequenceV1,
    ControlIncarnationIdV1, ControlStoreIdV1, ResidentRecordRefV1, SchemaDigest, SchemaId,
    COMMON_REQUEST_ENVELOPE_V1_SCHEMA, CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
    OPERATION_HANDLE_V1_SCHEMA, OPERATION_OBSERVATION_V1_SCHEMA,
};
use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError,
    AntiRollbackAnchorErrorKind, AntiRollbackAnchorObservation, AntiRollbackAnchorPort,
    AuthorityStoreCoordinate, ControlCheckpointHead, ControlDomainAntiRollbackAnchor,
    ControlRecord, ControlRecordBytes, ControlRecordCoordinate, ControlRecordDigest,
    ControlStoreError, ControlStoreErrorKind, OperationCustodyCandidate,
    OperationCustodyCheckpoint, OperationCustodyCommitOutcome, OperationCustodyIndexKey,
    OperationCustodyIndexMutation, OperationCustodyIndexObservation,
    OperationCustodyIndexPredecessor, OperationCustodyIndexQuery, OperationCustodyPort,
    OperationCustodyReadBackOutcome, OperationCustodyRecord, OperationCustodyTransitionKind,
    PreparedAntiRollbackAnchorTransition,
};

use super::*;

fn digest(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn coordinate(reference: &str, byte: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(reference).expect("record reference"),
        ControlRecordDigest::parse(digest(byte)).expect("record digest"),
    )
}

fn record(reference: &str, byte: char, schema: &str) -> ControlRecord {
    ControlRecord::new(
        coordinate(reference, byte),
        SchemaId::parse(schema).expect("schema id"),
        SchemaDigest::parse(digest('f')).expect("schema digest"),
        ControlRecordBytes::new(format!(r#"{{"record":"{reference}"}}"#).into_bytes())
            .expect("record bytes"),
    )
}

fn key(byte: char) -> OperationCustodyIndexKey {
    OperationCustodyIndexKey::parse(digest(byte)).expect("index key")
}

fn store_coordinate() -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("control-store").expect("store id"),
        ControlIncarnationIdV1::parse("incarnation-1").expect("incarnation id"),
    )
}

fn checkpoint(sequence: u64, reference: &str, byte: char) -> ControlCheckpointHead {
    ControlCheckpointHead::new(
        ControlCheckpointDomainIdV1::parse("operation:principal-1").expect("checkpoint domain"),
        ControlDomainEpochV1::new(3).expect("domain epoch"),
        ControlDomainSequenceV1::new(sequence).expect("domain sequence"),
        pale_core::records::ControlTransactionSequenceV1::new(sequence + 20).unwrap(),
        coordinate(reference, byte),
    )
}

fn start_records() -> Vec<OperationCustodyRecord> {
    vec![
        OperationCustodyRecord::CommonRequest(record(
            "request-1",
            '1',
            COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
        )),
        OperationCustodyRecord::OperationHandle(record(
            "handle-1",
            '2',
            OPERATION_HANDLE_V1_SCHEMA,
        )),
        OperationCustodyRecord::OperationObservation(record(
            "observation-1",
            '3',
            OPERATION_OBSERVATION_V1_SCHEMA,
        )),
    ]
}

fn start_indexes() -> Vec<OperationCustodyIndexMutation> {
    let records = start_records();
    let request = records[0].record().coordinate().clone();
    let handle = records[1].record().coordinate().clone();
    let observation = records[2].record().coordinate().clone();
    vec![
        OperationCustodyIndexMutation::RequestEvidence {
            key: key('a'),
            expected: OperationCustodyIndexPredecessor::Absent,
            successor: request,
        },
        OperationCustodyIndexMutation::RequestOperation {
            key: key('a'),
            expected: OperationCustodyIndexPredecessor::Absent,
            successor: handle.clone(),
        },
        OperationCustodyIndexMutation::OperationHandle {
            key: key('b'),
            expected: OperationCustodyIndexPredecessor::Absent,
            successor: handle,
        },
        OperationCustodyIndexMutation::LatestObservation {
            key: key('b'),
            expected: OperationCustodyIndexPredecessor::Absent,
            successor: observation,
        },
    ]
}

// Physical phase fixture only: contextual producers own the private encoding.
fn session_basis() -> pale_ports::OperationSessionBasis {
    pale_ports::OperationSessionBasis::new(
        handle_record().coordinate().clone(),
        request_record().coordinate().clone(),
        pale_core::records::SessionAdmissionLeaseDigest::parse(digest('a')).unwrap(),
        b"{}".to_vec(),
        4096,
    )
    .unwrap()
}

fn candidate() -> OperationCustodyCandidate {
    OperationCustodyCandidate::new_start(
        store_coordinate(),
        OperationCustodyTransitionKind::StartRoot,
        start_records(),
        start_indexes(),
        OperationCustodyCheckpoint::new(
            pale_ports::OperationCheckpointPhase::Successor {
                predecessor: checkpoint(8, "checkpoint-8", '8'),
                predecessor_record: Box::new(record(
                    "checkpoint-8",
                    '8',
                    CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
                )),
            },
            checkpoint(9, "checkpoint-9", '9'),
            record("checkpoint-9", '9', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
        )
        .expect("checkpoint transition"),
        crate::operation_accumulator_fixture::physical_batch(
            pale_ports::AccumulatorCheckpointBinding::new(
                store_coordinate(),
                ResidentRecordRefV1::parse("checkpoint-profile").expect("profile reference"),
                pale_core::records::CheckpointProfileDigest::parse(digest('f'))
                    .expect("profile digest"),
                pale_ports::AccumulatorCheckpointPhase::Successor(checkpoint(
                    8,
                    "checkpoint-8",
                    '8',
                )),
                checkpoint(9, "checkpoint-9", '9'),
                pale_ports::ControlTransactionPreparation::new(
                    store_coordinate(),
                    pale_core::records::ControlTransactionSequenceV1::new(28).unwrap(),
                )
                .unwrap(),
            )
            .expect("physical checkpoint binding"),
            pale_core::records::ArtifactDigest::parse(digest('8')).expect("prior root"),
            pale_core::records::ArtifactDigest::parse(digest('9')).expect("next root"),
            pale_core::records::ArtifactDigest::parse(digest('f')).expect("prior custody root"),
            pale_core::records::ArtifactDigest::parse(digest('f')).expect("next custody root"),
        ),
        session_basis(),
    )
    .expect("operation start")
}

fn initial_candidate() -> OperationCustodyCandidate {
    use pale_ports::{
        AccumulatorCheckpointBinding, AccumulatorCheckpointPhase, AccumulatorMutation,
        AccumulatorPosition, OperationAccumulatorBatch, OperationCheckpointPhase,
        StateAccumulatorBatch, UnresolvedCustodyAccumulatorBatch,
    };
    let root = |byte| pale_core::records::ArtifactDigest::parse(digest(byte)).unwrap();
    let head = checkpoint(1, "checkpoint-1", '1');
    OperationCustodyCandidate::new_start(
        store_coordinate(),
        OperationCustodyTransitionKind::StartRoot,
        start_records(),
        start_indexes(),
        OperationCustodyCheckpoint::new(
            OperationCheckpointPhase::Initial,
            head.clone(),
            record("checkpoint-1", '1', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
        )
        .unwrap(),
        OperationAccumulatorBatch::new(
            AccumulatorCheckpointBinding::new(
                store_coordinate(),
                ResidentRecordRefV1::parse("checkpoint-profile").unwrap(),
                pale_core::records::CheckpointProfileDigest::parse(digest('f')).unwrap(),
                AccumulatorCheckpointPhase::Initial,
                head,
                pale_ports::ControlTransactionPreparation::new(
                    store_coordinate(),
                    pale_core::records::ControlTransactionSequenceV1::new(20).unwrap(),
                )
                .unwrap(),
            )
            .unwrap(),
            StateAccumulatorBatch::new(
                root('0'),
                root('9'),
                vec![AccumulatorMutation::new(
                    AccumulatorPosition::Leaf([1; 32]),
                    None,
                    Some(vec![2; 32]),
                    32,
                )
                .unwrap()],
                1,
            )
            .unwrap(),
            UnresolvedCustodyAccumulatorBatch::new(root('0'), root('0'), vec![], 1).unwrap(),
        )
        .unwrap(),
        session_basis(),
    )
    .unwrap()
}

fn request_query() -> OperationCustodyIndexQuery {
    OperationCustodyIndexQuery::RequestEvidence(key('a'))
}

fn operation_query() -> OperationCustodyIndexQuery {
    OperationCustodyIndexQuery::RequestOperation(key('a'))
}

fn exact(
    query: OperationCustodyIndexQuery,
    record: ControlRecord,
) -> OperationCustodyIndexObservation {
    OperationCustodyIndexObservation::exact(query, record).expect("typed point observation")
}

fn request_record() -> ControlRecord {
    record("request-1", '1', COMMON_REQUEST_ENVELOPE_V1_SCHEMA)
}

fn handle_record() -> ControlRecord {
    record("handle-1", '2', OPERATION_HANDLE_V1_SCHEMA)
}

fn wrong_lineage_ahead() -> AntiRollbackAnchorObservation {
    AntiRollbackAnchorObservation::Ahead {
        observed: AntiRollbackAnchor::from(ControlDomainAntiRollbackAnchor::new(
            ControlStoreIdV1::parse("another-store").expect("other store id"),
            ControlIncarnationIdV1::parse("incarnation-1").expect("incarnation id"),
            ControlCheckpointDomainIdV1::parse("operation:principal-1").expect("checkpoint domain"),
            ControlDomainEpochV1::new(3).expect("domain epoch"),
            ControlDomainSequenceV1::new(10).expect("domain sequence"),
            coordinate("checkpoint-10", 'a'),
        )),
    }
}

fn operation_anchor(sequence: u64, reference: &str, byte: char) -> AntiRollbackAnchor {
    AntiRollbackAnchor::from(ControlDomainAntiRollbackAnchor::new(
        store_coordinate().store_id().clone(),
        store_coordinate().incarnation_id().clone(),
        ControlCheckpointDomainIdV1::parse("operation:principal-1").expect("checkpoint domain"),
        ControlDomainEpochV1::new(3).expect("domain epoch"),
        ControlDomainSequenceV1::new(sequence).expect("domain sequence"),
        coordinate(reference, byte),
    ))
}

struct TestStore {
    reads: VecDeque<Result<OperationCustodyIndexObservation, ControlStoreError>>,
    requested_reads: Vec<(AuthorityStoreCoordinate, OperationCustodyIndexQuery)>,
    requested_limits: Vec<usize>,
    commit: Option<Result<OperationCustodyCommitOutcome, ControlStoreError>>,
    read_back: Option<Result<OperationCustodyReadBackOutcome, ControlStoreError>>,
    commits: usize,
    observations: usize,
}

impl TestStore {
    fn with_reads(
        reads: impl IntoIterator<Item = Result<OperationCustodyIndexObservation, ControlStoreError>>,
    ) -> Self {
        Self {
            reads: reads.into_iter().collect(),
            requested_reads: Vec::new(),
            requested_limits: Vec::new(),
            commit: None,
            read_back: None,
            commits: 0,
            observations: 0,
        }
    }

    fn with_commit(
        mut self,
        commit: Result<OperationCustodyCommitOutcome, ControlStoreError>,
    ) -> Self {
        self.commit = Some(commit);
        self
    }
}

impl pale_ports::ControlTransactionPort for TestStore {
    fn prepare_transaction(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, pale_ports::ControlStoreError> {
        panic!("this physical-custody/readback fixture does not prepare a new checkpoint")
    }
}

impl OperationCustodyPort for TestStore {
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
        _: &pale_ports::AuthorityStoreCoordinate,
        _: &pale_core::records::ControlCheckpointDomainIdV1,
        _: usize,
    ) -> Result<
        Option<(pale_ports::ControlCheckpointHead, pale_ports::ControlRecord)>,
        pale_ports::ControlStoreError,
    > {
        panic!("this owning phase test does not perform locator recovery")
    }

    fn verify_accumulator_membership(
        &mut self,
        _: &pale_ports::OperationAccumulatorMembership,
    ) -> Result<
        pale_ports::VerifiedOperationAccumulatorMembership,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        panic!("this owning phase test does not perform membership recovery")
    }

    fn prepare_accumulator(
        &mut self,
        _: &pale_ports::OperationAccumulatorPreparation,
    ) -> Result<
        pale_ports::PreparedOperationAccumulator,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        panic!("unexpected accumulator preparation in custody-only fixture")
    }

    fn compare_and_retain_operation_lease(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlRecord,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        panic!("start-transaction fixture does not retain lease bodies")
    }

    fn read_operation_record(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlRecordCoordinate,
        _: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError> {
        panic!("start-transaction fixture does not read lease bodies")
    }

    fn compare_and_commit(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        self.commits += 1;
        self.commit.take().expect("one operation commit")
    }

    fn observe_exact(
        &mut self,
        _: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
        self.observations += 1;
        self.read_back.take().expect("one operation read-back")
    }

    fn read_index(
        &mut self,
        coordinate: &AuthorityStoreCoordinate,
        query: &OperationCustodyIndexQuery,
        maximum_record_bytes: usize,
    ) -> Result<OperationCustodyIndexObservation, ControlStoreError> {
        self.requested_reads
            .push((coordinate.clone(), query.clone()));
        self.requested_limits.push(maximum_record_bytes);
        self.reads.pop_front().expect("bounded request point read")
    }
}

struct TestAnchor {
    advance: Option<Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError>>,
    observation: Option<Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError>>,
    advances: usize,
    observations: usize,
}

impl TestAnchor {
    fn predecessor() -> Self {
        Self {
            advance: None,
            observation: Some(Ok(AntiRollbackAnchorObservation::ExactPredecessor)),
            advances: 0,
            observations: 0,
        }
    }

    fn advancing() -> Self {
        Self {
            advance: Some(Ok(AntiRollbackAnchorAdvanceDisposition::Advanced)),
            observation: Some(Ok(AntiRollbackAnchorObservation::ExactPredecessor)),
            advances: 0,
            observations: 0,
        }
    }
}

fn commit_start(
    prepared: PreparedOperationStart,
    store: &mut dyn OperationCustodyPort,
) -> Result<DatabaseCommittedOperationStartAnchorPending, OperationStartCommitFailure> {
    let mut anchor = TestAnchor::predecessor();
    prepared.commit(store, &mut anchor, 16 * 1024)
}

#[test]
fn initial_start_requires_missing_anchor_not_a_predecessor_or_existing_successor() {
    for observation in [
        AntiRollbackAnchorObservation::ExactPredecessor,
        AntiRollbackAnchorObservation::ExactSuccessor,
        AntiRollbackAnchorObservation::Conflicting {
            observed: operation_anchor(1, "other-checkpoint", 'a'),
        },
    ] {
        let prepared = prepare_operation_start(initial_candidate()).unwrap();
        let mut store = TestStore::with_reads(absent_request_reads());
        let mut anchor = TestAnchor::predecessor();
        anchor.observation = Some(Ok(observation));
        assert!(prepared.commit(&mut store, &mut anchor, 16 * 1024).is_err());
        assert_eq!(
            (store.commits, store.observations, anchor.advances),
            (0, 0, 0)
        );
    }
    // Conversely Missing is not an exact predecessor for a successor phase.
    let mut store = TestStore::with_reads(absent_request_reads());
    let mut anchor = TestAnchor::predecessor();
    anchor.observation = Some(Ok(AntiRollbackAnchorObservation::Missing));
    assert!(prepare_operation_start(candidate())
        .unwrap()
        .commit(&mut store, &mut anchor, 16 * 1024)
        .is_err());
    assert_eq!(store.commits, 0);
}

#[test]
fn initial_exact_database_replay_establishes_only_its_missing_anchor() {
    let mut store = TestStore::with_reads(absent_request_reads())
        .with_commit(Ok(OperationCustodyCommitOutcome::ExactReplay));
    let mut anchor = TestAnchor::advancing();
    anchor.observation = Some(Ok(AntiRollbackAnchorObservation::Missing));
    let pending = prepare_operation_start(initial_candidate())
        .unwrap()
        .commit(&mut store, &mut anchor, 16 * 1024)
        .unwrap();
    assert_eq!(
        pending.database_evidence(),
        OperationCustodyDatabaseEvidence::ExactReplay
    );
    assert_eq!(anchor.advances, 0);
    anchor.observation = Some(Ok(AntiRollbackAnchorObservation::Missing));
    pending.advance_anchor(&mut anchor).unwrap();
    assert_eq!(
        (store.commits, anchor.advances, anchor.observations),
        (1, 1, 2)
    );
}

#[test]
fn initial_unknown_database_and_anchor_responses_recover_without_second_write() {
    let mut store = TestStore::with_reads(absent_request_reads()).with_commit(Err(
        ControlStoreError::new(ControlStoreErrorKind::OutcomeUnknown),
    ));
    store.read_back = Some(Ok(OperationCustodyReadBackOutcome::ExactSuccessor));
    let mut anchor = TestAnchor::advancing();
    anchor.observation = Some(Ok(AntiRollbackAnchorObservation::Missing));
    let pending = prepare_operation_start(initial_candidate())
        .unwrap()
        .commit(&mut store, &mut anchor, 16 * 1024)
        .unwrap();
    assert_eq!(
        pending.database_evidence(),
        OperationCustodyDatabaseEvidence::RecoveredExactAfterAmbiguousCommit
    );
    anchor.advance = Some(Err(AntiRollbackAnchorError::new(
        AntiRollbackAnchorErrorKind::OutcomeUnknown,
    )));
    anchor.observation = Some(Ok(AntiRollbackAnchorObservation::ExactSuccessor));
    pending.advance_anchor(&mut anchor).unwrap();
    assert_eq!(
        (
            store.commits,
            store.observations,
            anchor.advances,
            anchor.observations
        ),
        (1, 1, 1, 2)
    );
}

#[test]
fn later_exact_start_readback_recovers_without_a_second_commit() {
    let mut store = TestStore::with_reads(absent_request_reads()).with_commit(Err(
        ControlStoreError::new(ControlStoreErrorKind::OutcomeUnknown),
    ));
    store.read_back = Some(Err(ControlStoreError::new(ControlStoreErrorKind::Storage)));
    let mut anchor = TestAnchor::predecessor();
    let failure = prepare_operation_start(candidate())
        .expect("prepared start")
        .commit(&mut store, &mut anchor, 16 * 1024)
        .expect_err("the first exact read-back is unavailable");
    assert_eq!((store.commits, store.observations), (1, 1));

    store.read_back = Some(Ok(OperationCustodyReadBackOutcome::ExactSuccessor));
    let pending = failure
        .reconcile_exact(&mut store)
        .expect("the retained exact candidate is now visible");
    assert_eq!(
        pending.database_evidence(),
        OperationCustodyDatabaseEvidence::RecoveredExactAfterAmbiguousCommit
    );
    assert_eq!((store.commits, store.observations), (1, 2));
}

#[test]
fn later_exact_start_anchor_observation_never_blindly_repeats_the_advance() {
    for later in [
        AntiRollbackAnchorObservation::ExactSuccessor,
        AntiRollbackAnchorObservation::ExactPredecessor,
    ] {
        let mut store = TestStore::with_reads(absent_request_reads())
            .with_commit(Ok(OperationCustodyCommitOutcome::Committed));
        let mut anchor = TestAnchor {
            advance: Some(Ok(AntiRollbackAnchorAdvanceDisposition::Advanced)),
            observation: Some(Ok(AntiRollbackAnchorObservation::Missing)),
            advances: 0,
            observations: 0,
        };
        let pending = prepare_operation_start(initial_candidate())
            .expect("prepared start")
            .commit(&mut store, &mut anchor, 16 * 1024)
            .expect("database commit");
        anchor.advance = Some(Err(AntiRollbackAnchorError::new(
            AntiRollbackAnchorErrorKind::OutcomeUnknown,
        )));
        anchor.observation = Some(Err(AntiRollbackAnchorError::new(
            AntiRollbackAnchorErrorKind::Unavailable,
        )));
        let failure = pending
            .advance_anchor(&mut anchor)
            .expect_err("the first ambiguity observation is unavailable");
        assert_eq!(anchor.advances, 1);

        anchor.observation = Some(Ok(later.clone()));
        if matches!(later, AntiRollbackAnchorObservation::ExactPredecessor) {
            anchor.advance = Some(Ok(AntiRollbackAnchorAdvanceDisposition::Advanced));
        }
        failure
            .continue_after_failed_observation(&mut anchor)
            .expect("later exact anchor evidence closes the retained transition");
        assert_eq!(
            anchor.advances,
            if matches!(later, AntiRollbackAnchorObservation::ExactPredecessor) {
                2
            } else {
                1
            }
        );
    }
}

#[test]
fn later_conflicting_start_evidence_remains_fenced() {
    let mut store = TestStore::with_reads(absent_request_reads()).with_commit(Err(
        ControlStoreError::new(ControlStoreErrorKind::OutcomeUnknown),
    ));
    store.read_back = Some(Err(ControlStoreError::new(ControlStoreErrorKind::Storage)));
    let mut anchor = TestAnchor::predecessor();
    let failure = prepare_operation_start(candidate())
        .expect("prepared start")
        .commit(&mut store, &mut anchor, 16 * 1024)
        .expect_err("the first exact read-back is unavailable");
    store.read_back = Some(Ok(OperationCustodyReadBackOutcome::ConflictingOrPartial));
    assert!(failure.reconcile_exact(&mut store).is_err());
    assert_eq!(store.commits, 1);
}

impl AntiRollbackAnchorPort for TestAnchor {
    fn read(
        &mut self,
        key: &pale_ports::AntiRollbackAnchorKey,
    ) -> Result<Option<pale_ports::AntiRollbackAnchor>, AntiRollbackAnchorError> {
        let _ = key;
        panic!("start transition test must not read a recovery anchor")
    }

    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        self.advances += 1;
        self.advance.take().expect("one anchor advance")
    }

    fn observe(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        self.observations += 1;
        self.observation.take().expect("one anchor observation")
    }
}

fn absent_request_reads() -> [Result<OperationCustodyIndexObservation, ControlStoreError>; 2] {
    [
        Ok(OperationCustodyIndexObservation::absent(operation_query())),
        Ok(OperationCustodyIndexObservation::absent(request_query())),
    ]
}

fn existing_request_reads() -> [Result<OperationCustodyIndexObservation, ControlStoreError>; 4] {
    [
        Ok(exact(operation_query(), handle_record())),
        Ok(exact(request_query(), request_record())),
        Ok(exact(
            OperationCustodyIndexQuery::StartObservation(key('a')),
            record("observation-1", '3', OPERATION_OBSERVATION_V1_SCHEMA),
        )),
        Ok(exact(
            OperationCustodyIndexQuery::ObservationCheckpoint(
                OperationCustodyIndexKey::for_observation(&coordinate("observation-1", '3')),
            ),
            record("checkpoint-9", '9', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
        )),
    ]
}

#[test]
fn every_oversized_recovery_role_is_rejected_even_if_the_port_ignores_its_limit() {
    let limit = 256;
    for oversized_role in 0..4 {
        let reads = existing_request_reads()
            .into_iter()
            .enumerate()
            .map(|(index, read)| {
                let observed = read.expect("fixture observation");
                if index != oversized_role {
                    return Ok(observed);
                }
                let record = observed.record().expect("fixture record");
                Ok(exact(
                    observed.query().clone(),
                    ControlRecord::new(
                        record.coordinate().clone(),
                        record.schema_id().clone(),
                        record.schema_sha256().clone(),
                        ControlRecordBytes::new(vec![b'x'; limit + 1]).expect("oversized body"),
                    ),
                ))
            });
        let mut store = TestStore::with_reads(reads);
        assert!(matches!(
            read_operation_start(
                &mut store,
                &store_coordinate(),
                &key('a'),
                &request_record(),
                limit
            ),
            Err(OperationRecoveryError::RecordBoundExceeded)
        ));
        assert_eq!(store.requested_limits, vec![limit; oversized_role + 1]);
        assert_eq!(store.commits, 0);
        assert_eq!(store.observations, 0);
    }
    let mut absent = TestStore::with_reads(absent_request_reads());
    assert!(read_operation_start(
        &mut absent,
        &store_coordinate(),
        &key('a'),
        &request_record(),
        0
    )
    .expect("absent indexes require no body allocation")
    .is_none());
    assert_eq!(absent.requested_limits, [0, 0]);
}

#[test]
fn request_only_recovery_reads_original_records_without_a_start_candidate() {
    let reads = existing_request_reads();
    let expected_queries: Vec<_> = reads
        .iter()
        .map(|read| {
            (
                store_coordinate(),
                read.as_ref().expect("fixture").query().clone(),
            )
        })
        .collect();
    let mut store = TestStore::with_reads(reads);
    let evidence = read_operation_start(
        &mut store,
        &store_coordinate(),
        &key('a'),
        &request_record(),
        16 * 1024,
    )
    .expect("request lookup")
    .expect("original start");
    assert_eq!(evidence.store(), &store_coordinate());
    assert_eq!(evidence.request(), &request_record());
    assert_eq!(evidence.handle(), &handle_record());
    assert_eq!(
        evidence.initial_observation().coordinate(),
        &coordinate("observation-1", '3')
    );
    assert_eq!(
        evidence.checkpoint().coordinate(),
        &coordinate("checkpoint-9", '9')
    );
    assert_eq!(store.requested_reads, expected_queries);
    assert_eq!(store.commits, 0);
    assert_eq!(store.observations, 0);
}

#[test]
fn absent_request_is_read_only_and_distinct_from_partial_custody() {
    let mut store = TestStore::with_reads(absent_request_reads());
    assert!(read_operation_start(
        &mut store,
        &store_coordinate(),
        &key('a'),
        &request_record(),
        16 * 1024,
    )
    .expect("absent request")
    .is_none());
    assert_eq!(store.requested_reads.len(), 2);
    assert_eq!(store.commits, 0);
    assert_eq!(store.observations, 0);

    for missing in existing_request_reads().map(|read| read.expect("fixture").query().clone()) {
        let reads = existing_request_reads().map(|read| {
            read.map(|observation| {
                if observation.query() == &missing {
                    OperationCustodyIndexObservation::absent(missing.clone())
                } else {
                    observation
                }
            })
        });
        let mut store = TestStore::with_reads(reads);
        assert!(matches!(
            read_operation_start(
                &mut store,
                &store_coordinate(),
                &key('a'),
                &request_record(),
                16 * 1024,
            ),
            Err(OperationRecoveryError::IncompleteRequestIndexes)
        ));
        assert_eq!(store.commits, 0);
        assert_eq!(store.observations, 0);
    }
}

#[test]
fn request_lookup_rejects_misdirected_reads_without_mutation() {
    for misdirected in existing_request_reads().map(|read| read.expect("fixture").query().clone()) {
        let reads = existing_request_reads().map(|read| {
            read.map(|observation| {
                if observation.query() == &misdirected {
                    OperationCustodyIndexObservation::absent(
                        OperationCustodyIndexQuery::RequestOperation(key('f')),
                    )
                } else {
                    observation
                }
            })
        });
        let mut store = TestStore::with_reads(reads);
        assert!(matches!(
            read_operation_start(
                &mut store,
                &store_coordinate(),
                &key('a'),
                &request_record(),
                16 * 1024,
            ),
            Err(OperationRecoveryError::MisdirectedObservation)
        ));
        assert_eq!(store.commits, 0);
        assert_eq!(store.observations, 0);
    }
}

#[test]
fn duplicate_start_is_detected_before_commit_and_requires_exact_recovery() {
    let mut store = TestStore::with_reads(existing_request_reads());
    let error = commit_start(
        prepare_operation_start(candidate()).expect("prepared start"),
        &mut store,
    )
    .expect_err("stored handle has no recovered checkpoint witness");

    assert!(matches!(
        error.reason(),
        OperationStartCommitFailureReason::Lookup(
            OperationRecoveryError::ExistingStartRequiresRecovery { .. }
        )
    ));
    assert_eq!(store.commits, 0);
    assert_eq!(store.observations, 0);
    let OperationStartCommitFailureReason::Lookup(
        OperationRecoveryError::ExistingStartRequiresRecovery { evidence },
    ) = error.reason()
    else {
        panic!("physical recovery evidence")
    };
    assert_eq!(evidence.request(), &request_record());
    assert_eq!(evidence.store(), &store_coordinate());
    assert_eq!(evidence.handle(), &handle_record());
    assert_eq!(
        evidence.initial_observation().coordinate(),
        &coordinate("observation-1", '3')
    );
    assert_eq!(
        evidence.checkpoint().coordinate(),
        &coordinate("checkpoint-9", '9')
    );
}

#[test]
fn missing_recovery_index_prevents_duplicate_start_and_never_replays() {
    for missing in [2, 3] {
        let reads = existing_request_reads()
            .into_iter()
            .enumerate()
            .map(|(index, read)| {
                if index == missing {
                    let query = read.expect("fixture").query().clone();
                    Ok(OperationCustodyIndexObservation::absent(query))
                } else {
                    read
                }
            });
        let mut store = TestStore::with_reads(reads);
        let error = commit_start(
            prepare_operation_start(candidate()).expect("prepare"),
            &mut store,
        )
        .expect_err("incomplete recovery evidence");
        assert!(matches!(
            error.reason(),
            OperationStartCommitFailureReason::Lookup(
                OperationRecoveryError::IncompleteRequestIndexes
            )
        ));
        assert_eq!(store.commits, 0);
        assert_eq!(store.observations, 0);
    }
}

#[test]
fn a_new_start_completes_physical_custody_only_after_hard_domain_anchoring() {
    let mut store = TestStore::with_reads(absent_request_reads())
        .with_commit(Ok(OperationCustodyCommitOutcome::Committed));
    let mut anchor = TestAnchor::advancing();
    let pending = prepare_operation_start(candidate())
        .expect("prepared start")
        .commit(&mut store, &mut anchor, 16 * 1024)
        .expect("database custody");
    assert_eq!(
        pending.database_evidence(),
        OperationCustodyDatabaseEvidence::New
    );

    let completed = pending.advance_anchor(&mut anchor).expect("hard anchor");
    assert_eq!(
        completed.database_evidence(),
        OperationCustodyDatabaseEvidence::New
    );
    assert_eq!(
        completed.anchor_evidence(),
        OperationCustodyAnchorEvidence::Advanced
    );
    assert_eq!(store.commits, 1);
    assert_eq!(anchor.observations, 1);
    assert_eq!(anchor.advances, 1);
}

#[test]
fn restored_database_cannot_recommit_when_the_independent_anchor_is_already_ahead() {
    let mut store = TestStore::with_reads(absent_request_reads())
        .with_commit(Ok(OperationCustodyCommitOutcome::Committed));
    let mut anchor = TestAnchor {
        advance: None,
        observation: Some(Ok(AntiRollbackAnchorObservation::ExactSuccessor)),
        advances: 0,
        observations: 0,
    };
    let failure = prepare_operation_start(candidate())
        .expect("prepared start")
        .commit(&mut store, &mut anchor, 16 * 1024)
        .expect_err("a newer independent anchor rejects the restored database");

    assert!(matches!(
        failure.reason(),
        OperationStartCommitFailureReason::AnchorPrecondition(
            OperationCustodyAnchorFailureReason::NotAdvanced(observation)
        ) if matches!(observation.as_ref(), AntiRollbackAnchorObservation::ExactSuccessor)
    ));
    assert_eq!(store.commits, 0);
    assert_eq!(anchor.observations, 1);
    assert_eq!(anchor.advances, 0);
    let _failure = failure
        .try_into_retryable()
        .expect_err("an ahead anchor is not permission to rewrite restored state");
}

#[test]
fn every_nonpredecessor_anchor_state_rejects_a_new_start_before_database_mutation() {
    let behind = operation_anchor(7, "checkpoint-7", '7');
    let ahead = operation_anchor(10, "checkpoint-10", 'a');
    let conflicting = operation_anchor(8, "checkpoint-8", 'b');
    for (observed, expected) in [
        (
            AntiRollbackAnchorObservation::Missing,
            AntiRollbackAnchorObservation::Missing,
        ),
        (
            AntiRollbackAnchorObservation::Behind {
                observed: behind.clone(),
            },
            AntiRollbackAnchorObservation::Behind { observed: behind },
        ),
        (
            AntiRollbackAnchorObservation::Ahead {
                observed: ahead.clone(),
            },
            AntiRollbackAnchorObservation::Ahead { observed: ahead },
        ),
        (
            AntiRollbackAnchorObservation::Ahead {
                observed: conflicting.clone(),
            },
            AntiRollbackAnchorObservation::Conflicting {
                observed: conflicting,
            },
        ),
    ] {
        let mut store = TestStore::with_reads(absent_request_reads())
            .with_commit(Ok(OperationCustodyCommitOutcome::Committed));
        let mut anchor = TestAnchor {
            advance: None,
            observation: Some(Ok(observed)),
            advances: 0,
            observations: 0,
        };
        let failure = prepare_operation_start(candidate())
            .expect("prepared start")
            .commit(&mut store, &mut anchor, 16 * 1024)
            .expect_err("only the exact predecessor permits mutation");
        let OperationStartCommitFailureReason::AnchorPrecondition(
            OperationCustodyAnchorFailureReason::NotAdvanced(actual),
        ) = failure.reason()
        else {
            panic!("closed anchor precondition failure")
        };
        assert_eq!(actual.as_ref(), &expected);
        assert_eq!(store.commits, 0);
        assert_eq!(anchor.observations, 1);
        assert_eq!(anchor.advances, 0);
        let _failure = failure
            .try_into_retryable()
            .expect_err("a nonpredecessor anchor cannot become commit authority");
    }
}

#[test]
fn unavailable_anchor_preflight_retains_a_retryable_start_without_database_mutation() {
    let mut store = TestStore::with_reads(absent_request_reads())
        .with_commit(Ok(OperationCustodyCommitOutcome::Committed));
    let mut anchor = TestAnchor {
        advance: None,
        observation: Some(Err(AntiRollbackAnchorError::new(
            AntiRollbackAnchorErrorKind::Unavailable,
        ))),
        advances: 0,
        observations: 0,
    };
    let failure = prepare_operation_start(candidate())
        .expect("prepared start")
        .commit(&mut store, &mut anchor, 16 * 1024)
        .expect_err("unavailable anchor cannot permit a database write");
    assert!(matches!(
        failure.reason(),
        OperationStartCommitFailureReason::AnchorPrecondition(
            OperationCustodyAnchorFailureReason::Port(error)
        ) if error.kind() == AntiRollbackAnchorErrorKind::Unavailable
    ));
    let _prepared = failure
        .try_into_retryable()
        .expect("read-only preflight failure retains the original start");
    assert_eq!(store.commits, 0);
    assert_eq!(anchor.observations, 1);
    assert_eq!(anchor.advances, 0);
}

#[test]
fn anchor_change_after_preflight_cannot_complete_a_new_database_commit() {
    let mut store = TestStore::with_reads(absent_request_reads())
        .with_commit(Ok(OperationCustodyCommitOutcome::Committed));
    let mut anchor = TestAnchor {
        advance: Some(Ok(AntiRollbackAnchorAdvanceDisposition::ExactReplay)),
        observation: Some(Ok(AntiRollbackAnchorObservation::ExactPredecessor)),
        advances: 0,
        observations: 0,
    };
    let pending = prepare_operation_start(candidate())
        .expect("prepared start")
        .commit(&mut store, &mut anchor, 16 * 1024)
        .expect("predecessor permits one database commit");
    let failure = pending
        .advance_anchor(&mut anchor)
        .expect_err("a concurrent anchor successor is not this call's completion evidence");

    assert!(matches!(
        failure.reason(),
        OperationCustodyAnchorFailureReason::NotAdvanced(observation)
            if matches!(observation.as_ref(), AntiRollbackAnchorObservation::ExactSuccessor)
    ));
    assert_eq!(store.commits, 1);
    assert_eq!(anchor.observations, 1);
    assert_eq!(anchor.advances, 1);
}

#[test]
fn ambiguous_commit_reads_back_once_and_never_retries() {
    let mut store = TestStore::with_reads(absent_request_reads()).with_commit(Err(
        ControlStoreError::new(ControlStoreErrorKind::OutcomeUnknown),
    ));
    store.read_back = Some(Ok(OperationCustodyReadBackOutcome::ExactSuccessor));
    let pending = commit_start(
        prepare_operation_start(candidate()).expect("prepared start"),
        &mut store,
    )
    .expect("exact read-back");

    assert_eq!(
        pending.database_evidence(),
        OperationCustodyDatabaseEvidence::RecoveredExactAfterAmbiguousCommit
    );
    assert_eq!(store.commits, 1);
    assert_eq!(store.observations, 1);
}

#[test]
fn failed_read_back_after_ambiguous_commit_is_terminal_for_this_attempt() {
    let mut store = TestStore::with_reads(absent_request_reads()).with_commit(Err(
        ControlStoreError::new(ControlStoreErrorKind::OutcomeUnknown),
    ));
    store.read_back = Some(Err(ControlStoreError::new(ControlStoreErrorKind::Storage)));
    let error = commit_start(
        prepare_operation_start(candidate()).expect("prepared start"),
        &mut store,
    )
    .expect_err("failed read-back");

    assert!(matches!(
        error.reason(),
        OperationStartCommitFailureReason::Database(
            OperationCustodyDatabaseFailureReason::ReadBackFailed { .. }
        )
    ));
    assert_eq!(store.commits, 1);
    assert_eq!(store.observations, 1);
}

#[test]
fn concurrent_conflict_gets_one_request_reentry_lookup_and_no_commit_retry() {
    let reads = absent_request_reads()
        .into_iter()
        .chain(existing_request_reads());
    let mut store = TestStore::with_reads(reads).with_commit(Err(ControlStoreError::new(
        ControlStoreErrorKind::CandidateConflict,
    )));
    let error = commit_start(
        prepare_operation_start(candidate()).expect("prepared start"),
        &mut store,
    )
    .expect_err("winner still needs exact checkpoint recovery");

    assert!(matches!(
        error.reason(),
        OperationStartCommitFailureReason::Lookup(
            OperationRecoveryError::ExistingStartRequiresRecovery { .. }
        )
    ));
    assert_eq!(store.commits, 1);
    assert_eq!(store.observations, 0);
    assert_eq!(store.requested_limits, [16 * 1024; 6]);
}

#[test]
fn conflicting_signed_request_evidence_prevents_commit() {
    let conflict = record("request-other", '4', COMMON_REQUEST_ENVELOPE_V1_SCHEMA);
    let mut store = TestStore::with_reads([
        Ok(exact(operation_query(), handle_record())),
        Ok(exact(request_query(), conflict)),
    ]);
    let error = commit_start(
        prepare_operation_start(candidate()).expect("prepared start"),
        &mut store,
    )
    .expect_err("request conflict");

    assert!(matches!(
        error.reason(),
        OperationStartCommitFailureReason::Lookup(OperationRecoveryError::RequestConflict)
    ));
    assert_eq!(store.commits, 0);
}

#[test]
fn anchor_failure_retains_the_database_committed_phase() {
    let mut store = TestStore::with_reads(absent_request_reads())
        .with_commit(Ok(OperationCustodyCommitOutcome::Committed));
    let pending = commit_start(
        prepare_operation_start(candidate()).expect("prepared start"),
        &mut store,
    )
    .expect("database custody");
    let mut anchor = TestAnchor {
        advance: Some(Err(AntiRollbackAnchorError::new(
            AntiRollbackAnchorErrorKind::Unavailable,
        ))),
        observation: None,
        advances: 0,
        observations: 0,
    };
    let failure = pending
        .advance_anchor(&mut anchor)
        .expect_err("anchor unavailable");

    assert!(matches!(
        failure.reason(),
        OperationCustodyAnchorFailureReason::Port(_)
    ));
    assert_eq!(
        failure
            .try_into_retryable()
            .expect("unavailable anchor did not cross an ambiguous boundary")
            .database_evidence(),
        OperationCustodyDatabaseEvidence::New
    );
    assert_eq!(anchor.advances, 1);
    assert_eq!(anchor.observations, 0);
}

#[test]
fn claimed_ahead_from_another_lineage_cannot_complete_start() {
    let mut store = TestStore::with_reads(absent_request_reads())
        .with_commit(Ok(OperationCustodyCommitOutcome::Committed));
    let pending = commit_start(
        prepare_operation_start(candidate()).expect("prepared start"),
        &mut store,
    )
    .expect("database custody");
    let mut anchor = TestAnchor {
        advance: Some(Ok(AntiRollbackAnchorAdvanceDisposition::NotAdvanced(
            wrong_lineage_ahead(),
        ))),
        observation: None,
        advances: 0,
        observations: 0,
    };
    let failure = pending
        .advance_anchor(&mut anchor)
        .expect_err("another anchor lineage cannot prove custody");

    assert!(matches!(
        failure.reason(),
        OperationCustodyAnchorFailureReason::NotAdvanced(observation)
            if matches!(observation.as_ref(), AntiRollbackAnchorObservation::Conflicting { .. })
    ));
    assert_eq!(anchor.advances, 1);
    assert_eq!(anchor.observations, 0);
}

#[test]
fn ambiguous_advance_cannot_accept_a_claimed_ahead_from_another_lineage() {
    let mut store = TestStore::with_reads(absent_request_reads())
        .with_commit(Ok(OperationCustodyCommitOutcome::Committed));
    let pending = commit_start(
        prepare_operation_start(candidate()).expect("prepared start"),
        &mut store,
    )
    .expect("database custody");
    let mut anchor = TestAnchor {
        advance: Some(Err(AntiRollbackAnchorError::new(
            AntiRollbackAnchorErrorKind::OutcomeUnknown,
        ))),
        observation: Some(Ok(wrong_lineage_ahead())),
        advances: 0,
        observations: 0,
    };
    let failure = pending
        .advance_anchor(&mut anchor)
        .expect_err("ambiguous wrong-lineage observation cannot prove custody");

    assert!(matches!(
        failure.reason(),
        OperationCustodyAnchorFailureReason::AmbiguousNotExact { observation, .. }
            if matches!(observation.as_ref(), AntiRollbackAnchorObservation::Conflicting { .. })
    ));
    assert_eq!(anchor.advances, 1);
    assert_eq!(anchor.observations, 1);
}
