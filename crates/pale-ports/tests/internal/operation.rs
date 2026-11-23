//! Behavioral tests for the generic operation-custody boundary.

use super::*;

#[path = "operation/a2a.rs"]
mod a2a;
#[path = "operation/accumulator_preparation.rs"]
mod accumulator_preparation;
#[path = "operation/collection.rs"]
mod collection;
#[path = "operation/result.rs"]
mod result;
use crate::{
    AccumulatorBatchError, AccumulatorCheckpointBinding, AccumulatorCheckpointPhase,
    AccumulatorMutation, AccumulatorPosition, ControlRecordBytes, ControlRecordDigest,
    OperationAccumulatorBatch, OperationCheckpointPhase, StateAccumulatorBatch,
    UnresolvedCustodyAccumulatorBatch,
};
use pale_core::records::{
    ArtifactDigest, CheckpointProfileDigest, ControlCheckpointDomainIdV1, ControlDomainEpochV1,
    ControlDomainSequenceV1, ControlIncarnationIdV1, ControlStoreIdV1, ResidentRecordRefV1,
    SchemaDigest, SchemaId,
};

#[derive(Default)]
struct DeterministicCustody {
    committed: Option<OperationCustodyCandidate>,
}

impl crate::ControlTransactionPort for DeterministicCustody {
    fn prepare_transaction(
        &mut self,
        _: &crate::AuthorityStoreCoordinate,
    ) -> Result<crate::ControlTransactionPreparation, crate::ControlStoreError> {
        panic!("this physical-custody/readback fixture does not prepare a new checkpoint")
    }
}

impl OperationCustodyPort for DeterministicCustody {
    fn read_a2a_task_bindings(
        &mut self,
        _: &crate::AuthorityStoreCoordinate,
        _: &crate::OperationA2aBindingScope,
        _: Option<&crate::OperationA2aBindingCursor>,
        _: usize,
        _: usize,
    ) -> Result<crate::OperationA2aBindings, crate::ControlStoreError> {
        panic!("remote-task discovery is outside this fixture's tested journey")
    }

    fn read_publication_sources(
        &mut self,
        _store: &crate::AuthorityStoreCoordinate,
        _cursor: Option<&crate::OperationPublicationCursor>,
        _maximum_sources: usize,
        _maximum_body_bytes: usize,
    ) -> Result<crate::OperationPublicationSources, crate::OperationPublicationReadError> {
        panic!("publication discovery is outside this fixture's tested journey")
    }

    fn read_checkpoint_head(
        &mut self,
        _: &crate::AuthorityStoreCoordinate,
        _: &pale_core::records::ControlCheckpointDomainIdV1,
        _: usize,
    ) -> Result<
        Option<(crate::ControlCheckpointHead, crate::ControlRecord)>,
        crate::ControlStoreError,
    > {
        panic!("this owning phase test does not perform locator recovery")
    }

    fn verify_accumulator_membership(
        &mut self,
        _: &crate::OperationAccumulatorMembership,
    ) -> Result<
        crate::VerifiedOperationAccumulatorMembership,
        crate::OperationAccumulatorPreparationError,
    > {
        panic!("this owning phase test does not perform membership recovery")
    }

    fn prepare_accumulator(
        &mut self,
        _: &crate::OperationAccumulatorPreparation,
    ) -> Result<crate::PreparedOperationAccumulator, crate::OperationAccumulatorPreparationError>
    {
        panic!("candidate-only fixture must not prepare authenticated roots")
    }

    fn compare_and_retain_operation_lease(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlRecord,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        panic!("candidate-only fixture does not retain operation leases")
    }

    fn read_operation_record(
        &mut self,
        _: &AuthorityStoreCoordinate,
        _: &ControlRecordCoordinate,
        _: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError> {
        panic!("candidate-only fixture does not read operation leases")
    }

    fn compare_and_commit(
        &mut self,
        candidate: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyCommitOutcome, ControlStoreError> {
        match &self.committed {
            Some(committed) if committed == candidate => {
                Ok(OperationCustodyCommitOutcome::ExactReplay)
            }
            Some(_) => Err(ControlStoreError::new(
                crate::ControlStoreErrorKind::CandidateConflict,
            )),
            None => {
                self.committed = Some(candidate.clone());
                Ok(OperationCustodyCommitOutcome::Committed)
            }
        }
    }

    fn observe_exact(
        &mut self,
        candidate: &OperationCustodyCandidate,
    ) -> Result<OperationCustodyReadBackOutcome, ControlStoreError> {
        Ok(match &self.committed {
            Some(committed) if committed == candidate => {
                OperationCustodyReadBackOutcome::ExactSuccessor
            }
            None => OperationCustodyReadBackOutcome::Predecessor,
            Some(_) => OperationCustodyReadBackOutcome::ConflictingOrPartial,
        })
    }

    fn read_index(
        &mut self,
        store: &AuthorityStoreCoordinate,
        query: &OperationCustodyIndexQuery,
        _maximum_record_bytes: usize,
    ) -> Result<OperationCustodyIndexObservation, ControlStoreError> {
        let Some(candidate) = self
            .committed
            .as_ref()
            .filter(|candidate| candidate.store() == store)
        else {
            return Ok(OperationCustodyIndexObservation::absent(query.clone()));
        };
        let successor = candidate.indexes().iter().find_map(|index| {
            let (key, successor) = match index {
                OperationCustodyIndexMutation::RequestForHandle { key, successor }
                    if matches!(query, OperationCustodyIndexQuery::RequestForHandle(_)) =>
                {
                    (key, successor)
                }
                OperationCustodyIndexMutation::RequestEvidence { key, successor, .. }
                    if matches!(query, OperationCustodyIndexQuery::RequestEvidence(_)) =>
                {
                    (key, successor)
                }
                OperationCustodyIndexMutation::RequestOperation { key, successor, .. }
                    if matches!(query, OperationCustodyIndexQuery::RequestOperation(_)) =>
                {
                    (key, successor)
                }
                OperationCustodyIndexMutation::OperationHandle { key, successor, .. }
                    if matches!(query, OperationCustodyIndexQuery::OperationHandle(_)) =>
                {
                    (key, successor)
                }
                OperationCustodyIndexMutation::ParentChildMembership { key, successor, .. }
                    if matches!(query, OperationCustodyIndexQuery::ParentChildMembership(_)) =>
                {
                    (key, successor)
                }
                OperationCustodyIndexMutation::LatestObservation { key, successor, .. }
                    if matches!(query, OperationCustodyIndexQuery::LatestObservation(_)) =>
                {
                    (key, successor)
                }
                _ => return None,
            };
            (key == query.key()).then_some(successor)
        });
        let Some(successor) = successor else {
            return Ok(OperationCustodyIndexObservation::absent(query.clone()));
        };
        let record = candidate
            .records()
            .iter()
            .map(OperationCustodyRecord::record)
            .find(|record| record.coordinate() == successor)
            .expect("validated successor record")
            .clone();
        OperationCustodyIndexObservation::exact(query.clone(), record).map_err(|error| {
            ControlStoreError::with_source(crate::ControlStoreErrorKind::Storage, error)
        })
    }
}

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

#[test]
fn taking_an_index_record_preserves_its_buffer_and_absence() {
    let query = OperationCustodyIndexQuery::RequestEvidence(key('a'));
    assert!(OperationCustodyIndexObservation::absent(query.clone())
        .into_record()
        .is_none());
    let request = record("request", '1', COMMON_REQUEST_ENVELOPE_V1_SCHEMA);
    let original = request.canonical_record_jcs().as_slice().as_ptr();
    let observed = OperationCustodyIndexObservation::exact(query, request)
        .unwrap()
        .into_record()
        .unwrap();
    assert_eq!(
        observed.canonical_record_jcs().as_slice().as_ptr(),
        original
    );
}

fn store() -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("control-store").expect("store id"),
        ControlIncarnationIdV1::parse("incarnation-1").expect("incarnation id"),
    )
}

fn checkpoint(sequence: u64, reference: &str, byte: char) -> ControlCheckpointHead {
    ControlCheckpointHead::new(
        ControlCheckpointDomainIdV1::parse("operation:principal-1").expect("domain id"),
        ControlDomainEpochV1::new(3).expect("domain epoch"),
        ControlDomainSequenceV1::new(sequence).expect("domain sequence"),
        pale_core::records::ControlTransactionSequenceV1::new(sequence + 20).unwrap(),
        coordinate(reference, byte),
    )
}

fn checkpoint_transition() -> OperationCustodyCheckpoint {
    OperationCustodyCheckpoint::new(
        OperationCheckpointPhase::Successor {
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
    .expect("checkpoint transition")
}

// Physical shape fixture only: these port tests do not admit a root or signer.
fn accumulator(checkpoint: &OperationCustodyCheckpoint) -> OperationAccumulatorBatch {
    OperationAccumulatorBatch::new(
        AccumulatorCheckpointBinding::new(
            store(),
            ResidentRecordRefV1::parse("profile").unwrap(),
            CheckpointProfileDigest::parse(digest('a')).unwrap(),
            match checkpoint.phase() {
                OperationCheckpointPhase::Initial => AccumulatorCheckpointPhase::Initial,
                OperationCheckpointPhase::Successor { predecessor, .. } => {
                    AccumulatorCheckpointPhase::Successor(predecessor.clone())
                }
            },
            checkpoint.successor().clone(),
            crate::ControlTransactionPreparation::new(
                store(),
                pale_core::records::ControlTransactionSequenceV1::new(
                    checkpoint
                        .successor()
                        .committed_transaction_sequence()
                        .get()
                        - 1,
                )
                .unwrap(),
            )
            .unwrap(),
        )
        .unwrap(),
        StateAccumulatorBatch::new(
            ArtifactDigest::parse(digest(
                if matches!(checkpoint.phase(), OperationCheckpointPhase::Initial) {
                    '0'
                } else {
                    '1'
                },
            ))
            .unwrap(),
            ArtifactDigest::parse(digest('2')).unwrap(),
            vec![AccumulatorMutation::new(
                AccumulatorPosition::Leaf([1; 32]),
                None,
                Some(vec![2; 32]),
                132,
            )
            .unwrap()],
            1,
        )
        .unwrap(),
        UnresolvedCustodyAccumulatorBatch::new(
            ArtifactDigest::parse(digest('0')).unwrap(),
            ArtifactDigest::parse(digest('0')).unwrap(),
            vec![],
            1,
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn accumulator_construction_rejects_empty_unbounded_unordered_and_noop_mutations() {
    let position = AccumulatorPosition::Leaf([1; 32]);
    assert_eq!(
        AccumulatorMutation::new(position, None, Some(vec![1; 32]), 0).unwrap_err(),
        AccumulatorBatchError::Bounds
    );
    assert_eq!(
        AccumulatorMutation::new(position, None, Some(vec![1; 32]), 31).unwrap_err(),
        AccumulatorBatchError::Bounds
    );
    assert_eq!(
        AccumulatorMutation::new(position, None, Some(vec![1; 31]), 132).unwrap_err(),
        AccumulatorBatchError::ByteLength
    );
    assert_eq!(
        AccumulatorMutation::new(position, None, None, 132).unwrap_err(),
        AccumulatorBatchError::NoChange
    );
    assert_eq!(
        AccumulatorMutation::new(position, Some(vec![1; 32]), Some(vec![1; 32]), 132).unwrap_err(),
        AccumulatorBatchError::NoChange
    );
    let root = ArtifactDigest::parse(digest('0')).unwrap();
    let mutation = AccumulatorMutation::new(position, None, Some(vec![1; 32]), 132).unwrap();
    assert_eq!(
        StateAccumulatorBatch::new(root.clone(), root.clone(), vec![mutation.clone()], 0)
            .unwrap_err(),
        AccumulatorBatchError::Bounds
    );
    assert_eq!(
        StateAccumulatorBatch::new(
            root.clone(),
            root.clone(),
            vec![mutation.clone(), mutation.clone()],
            1
        )
        .unwrap_err(),
        AccumulatorBatchError::Bounds
    );
    assert_eq!(
        StateAccumulatorBatch::new(
            root.clone(),
            root.clone(),
            vec![mutation.clone(), mutation],
            2
        )
        .unwrap_err(),
        AccumulatorBatchError::Order
    );
    assert_eq!(
        UnresolvedCustodyAccumulatorBatch::new(
            root.clone(),
            ArtifactDigest::parse(digest('1')).unwrap(),
            vec![],
            1
        )
        .unwrap_err(),
        AccumulatorBatchError::EmptyRootChange
    );
    let binding = accumulator(&checkpoint_transition()).binding().clone();
    assert_eq!(
        OperationAccumulatorBatch::new(
            binding,
            StateAccumulatorBatch::new(root.clone(), root.clone(), vec![], 1).unwrap(),
            UnresolvedCustodyAccumulatorBatch::new(root.clone(), root, vec![], 1).unwrap()
        )
        .unwrap_err(),
        AccumulatorBatchError::EmptyOperation
    );
}

#[test]
fn only_an_initial_checkpoint_can_retain_two_canonical_empty_trees() {
    let zero = ArtifactDigest::parse(digest('0')).unwrap();
    let state = StateAccumulatorBatch::new(zero.clone(), zero.clone(), vec![], 1).unwrap();
    let unresolved =
        UnresolvedCustodyAccumulatorBatch::new(zero.clone(), zero.clone(), vec![], 1).unwrap();
    let binding = AccumulatorCheckpointBinding::new(
        store(),
        ResidentRecordRefV1::parse("profile").unwrap(),
        CheckpointProfileDigest::parse(digest('a')).unwrap(),
        AccumulatorCheckpointPhase::Initial,
        checkpoint(1, "initial", '1'),
        crate::ControlTransactionPreparation::new(
            store(),
            pale_core::records::ControlTransactionSequenceV1::new(20).unwrap(),
        )
        .unwrap(),
    )
    .unwrap();

    let batch = OperationAccumulatorBatch::new(binding.clone(), state, unresolved.clone()).unwrap();
    assert_eq!(
        batch.binding().phase(),
        &AccumulatorCheckpointPhase::Initial
    );
    assert!(batch.state().mutations().is_empty());
    assert!(batch.unresolved().mutations().is_empty());
    assert_eq!(batch.state().predecessor_root(), &zero);
    assert_eq!(batch.state().successor_root(), &zero);
    assert_eq!(batch.unresolved().predecessor_root(), &zero);
    assert_eq!(batch.unresolved().successor_root(), &zero);

    let nonzero = ArtifactDigest::parse(digest('2')).unwrap();
    let nonzero_state = StateAccumulatorBatch::new(nonzero.clone(), nonzero, vec![], 1).unwrap();
    assert_eq!(
        OperationAccumulatorBatch::new(binding.clone(), nonzero_state, unresolved.clone()),
        Err(AccumulatorBatchError::InitialPredecessor)
    );

    let predecessor_state = StateAccumulatorBatch::new(
        zero.clone(),
        ArtifactDigest::parse(digest('2')).unwrap(),
        vec![AccumulatorMutation::new(
            AccumulatorPosition::Leaf([2; 32]),
            Some(vec![1; 32]),
            Some(vec![2; 32]),
            32,
        )
        .unwrap()],
        1,
    )
    .unwrap();
    assert_eq!(
        OperationAccumulatorBatch::new(binding, predecessor_state, unresolved),
        Err(AccumulatorBatchError::InitialPredecessor)
    );
}

#[test]
fn operation_rejects_unlike_accumulator_checkpoint_before_storage() {
    let unlike = OperationCustodyCheckpoint::new(
        OperationCheckpointPhase::Successor {
            predecessor: checkpoint(9, "checkpoint-9", '9'),
            predecessor_record: Box::new(record(
                "checkpoint-9",
                '9',
                CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
            )),
        },
        checkpoint(10, "checkpoint-10", 'a'),
        record("checkpoint-10", 'a', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
    )
    .unwrap();
    assert_eq!(
        OperationCustodyCandidate::new_start(
            store(),
            OperationCustodyTransitionKind::StartRoot,
            start_records(),
            root_start_indexes(),
            checkpoint_transition(),
            accumulator(&unlike),
            start_session_basis(),
        )
        .unwrap_err(),
        OperationCustodyCandidateError::AccumulatorBinding
    );
}

#[test]
fn initial_checkpoint_is_mandatory_sequence_one_and_only_a_root_start() {
    assert_eq!(
        OperationCustodyCheckpoint::new(
            OperationCheckpointPhase::Initial,
            checkpoint(2, "initial", '1'),
            record("initial", '1', CONTROL_STATE_CHECKPOINT_V1_SCHEMA)
        ),
        Err(OperationCustodyCandidateError::CheckpointSuccession)
    );
    let checkpoint = OperationCustodyCheckpoint::new(
        OperationCheckpointPhase::Initial,
        checkpoint(1, "initial", '1'),
        record("initial", '1', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
    )
    .unwrap();
    let batch = accumulator(&checkpoint);
    OperationCustodyCandidate::new_start(
        store(),
        OperationCustodyTransitionKind::StartRoot,
        start_records(),
        root_start_indexes(),
        checkpoint.clone(),
        batch.clone(),
        start_session_basis(),
    )
    .unwrap();
    assert_eq!(
        OperationCustodyCandidate::new_start(
            store(),
            OperationCustodyTransitionKind::StartChild,
            start_records(),
            root_start_indexes(),
            checkpoint.clone(),
            batch.clone(),
            start_session_basis(),
        ),
        Err(OperationCustodyCandidateError::AccumulatorBinding)
    );
    assert_eq!(
        OperationCustodyCandidate::new(
            store(),
            OperationCustodyTransitionKind::Observe,
            start_records(),
            root_start_indexes(),
            checkpoint,
            batch,
        ),
        Err(OperationCustodyCandidateError::AccumulatorBinding)
    );
}

// Bounded opaque physical fixture, not a public record or admitted session.
fn start_session_basis() -> crate::OperationSessionBasis {
    crate::OperationSessionBasis::new(
        coordinate("handle-1", '2'),
        coordinate("request-1", '1'),
        pale_core::records::SessionAdmissionLeaseDigest::parse(digest('a')).unwrap(),
        br#"{"physical_fixture":"session-basis"}"#.to_vec(),
        4096,
    )
    .unwrap()
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

fn root_start_indexes() -> Vec<OperationCustodyIndexMutation> {
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
            successor: handle.clone(),
        },
        OperationCustodyIndexMutation::LatestObservation {
            key: key('b'),
            expected: OperationCustodyIndexPredecessor::Absent,
            successor: observation,
        },
    ]
}

fn child_start_indexes() -> Vec<OperationCustodyIndexMutation> {
    let handle = start_records()[1].record().coordinate().clone();
    let mut indexes = root_start_indexes();
    indexes.push(OperationCustodyIndexMutation::ParentChildMembership {
        key: key('c'),
        expected: OperationCustodyIndexPredecessor::Absent,
        successor: handle,
    });
    indexes
}

#[test]
fn root_start_closes_the_exact_request_handle_and_initial_observation() {
    let candidate = OperationCustodyCandidate::new_start(
        store(),
        OperationCustodyTransitionKind::StartRoot,
        start_records(),
        root_start_indexes(),
        checkpoint_transition(),
        accumulator(&checkpoint_transition()),
        start_session_basis(),
    )
    .expect("root start");

    assert_eq!(candidate.records().len(), 3);
    assert_eq!(candidate.indexes().len(), 7);
    assert_eq!(
        candidate
            .record(OperationCustodyRecordKind::OperationHandle)
            .expect("handle")
            .coordinate(),
        &coordinate("handle-1", '2')
    );
}

#[test]
fn child_membership_is_an_additional_exact_parent_child_tuple_index() {
    let candidate = OperationCustodyCandidate::new_start(
        store(),
        OperationCustodyTransitionKind::StartChild,
        start_records(),
        child_start_indexes(),
        checkpoint_transition(),
        accumulator(&checkpoint_transition()),
        start_session_basis(),
    )
    .expect("child start");

    assert_eq!(candidate.indexes().len(), 8);
    assert!(candidate.indexes().iter().any(|index| matches!(
        index,
        OperationCustodyIndexMutation::ParentChildMembership { key: observed, .. }
            if observed == &key('c')
    )));
}

#[test]
fn child_start_rejects_an_absent_membership_index() {
    let error = OperationCustodyCandidate::new_start(
        store(),
        OperationCustodyTransitionKind::StartChild,
        start_records(),
        root_start_indexes(),
        checkpoint_transition(),
        accumulator(&checkpoint_transition()),
        start_session_basis(),
    )
    .expect_err("missing membership must fail");

    assert_eq!(error, OperationCustodyCandidateError::IndexShape);
}

#[test]
fn start_requires_request_and_handle_indexes_to_share_their_identity_keys() {
    for role in [
        IndexRoleToCorrupt::RequestOperation,
        IndexRoleToCorrupt::LatestObservation,
    ] {
        let mut indexes = root_start_indexes();
        match role {
            IndexRoleToCorrupt::RequestOperation => {
                let OperationCustodyIndexMutation::RequestOperation { key: index_key, .. } =
                    &mut indexes[1]
                else {
                    panic!("fixture preserves request-operation position");
                };
                *index_key = key('d');
            }
            IndexRoleToCorrupt::LatestObservation => {
                let OperationCustodyIndexMutation::LatestObservation { key: index_key, .. } =
                    &mut indexes[3]
                else {
                    panic!("fixture preserves latest-observation position");
                };
                *index_key = key('d');
            }
        }
        let error = OperationCustodyCandidate::new_start(
            store(),
            OperationCustodyTransitionKind::StartRoot,
            start_records(),
            indexes,
            checkpoint_transition(),
            accumulator(&checkpoint_transition()),
            start_session_basis(),
        )
        .expect_err("split request or handle index keys must fail");

        assert_eq!(error, OperationCustodyCandidateError::IndexKeyPair);
    }
}

#[derive(Clone, Copy)]
enum IndexRoleToCorrupt {
    RequestOperation,
    LatestObservation,
}

#[test]
fn observation_requires_the_exact_prior_observation_and_only_advances_latest() {
    let predecessor = record("observation-1", '3', OPERATION_OBSERVATION_V1_SCHEMA);
    let successor = record("observation-2", '4', OPERATION_OBSERVATION_V1_SCHEMA);
    let candidate = OperationCustodyCandidate::new(
        store(),
        OperationCustodyTransitionKind::Observe,
        vec![OperationCustodyRecord::OperationObservation(
            successor.clone(),
        )],
        vec![OperationCustodyIndexMutation::LatestObservation {
            key: key('b'),
            expected: OperationCustodyIndexPredecessor::Exact(predecessor),
            successor: successor.coordinate().clone(),
        }],
        checkpoint_transition(),
        accumulator(&checkpoint_transition()),
    )
    .expect("successor observation");

    assert_eq!(candidate.records().len(), 1);
    assert_eq!(candidate.indexes().len(), 2);
}

#[test]
fn observation_derives_one_exact_checkpoint_index_and_rejects_supplied_substitutes() {
    let predecessor = record("observation-1", '3', OPERATION_OBSERVATION_V1_SCHEMA);
    let successor = record("observation-2", '4', OPERATION_OBSERVATION_V1_SCHEMA);
    let checkpoint = checkpoint_transition();
    let latest = OperationCustodyIndexMutation::LatestObservation {
        key: key('b'),
        expected: OperationCustodyIndexPredecessor::Exact(predecessor),
        successor: successor.coordinate().clone(),
    };
    let candidate = OperationCustodyCandidate::new(
        store(),
        OperationCustodyTransitionKind::Observe,
        vec![OperationCustodyRecord::OperationObservation(
            successor.clone(),
        )],
        vec![latest.clone()],
        checkpoint.clone(),
        accumulator(&checkpoint),
    )
    .expect("successor observation");
    assert!(candidate.indexes().iter().any(|index| matches!(
        index,
        OperationCustodyIndexMutation::ObservationCheckpoint { key, successor: indexed }
            if key == &OperationCustodyIndexKey::for_observation(successor.coordinate())
                && indexed == checkpoint.successor_record().coordinate()
    )));

    let supplied = OperationCustodyIndexMutation::ObservationCheckpoint {
        key: OperationCustodyIndexKey::for_observation(successor.coordinate()),
        successor: coordinate("substituted-checkpoint", 'e'),
    };
    assert_eq!(
        OperationCustodyCandidate::new(
            store(),
            OperationCustodyTransitionKind::Observe,
            vec![OperationCustodyRecord::OperationObservation(successor)],
            vec![latest, supplied],
            checkpoint.clone(),
            accumulator(&checkpoint),
        )
        .expect_err("the application cannot supply or substitute the derived checkpoint index"),
        OperationCustodyCandidateError::IndexShape
    );
}

#[test]
fn observation_can_retain_one_fresh_control_request_without_a_second_index() {
    let predecessor = record("observation-1", '3', OPERATION_OBSERVATION_V1_SCHEMA);
    let successor = record("observation-2", '4', OPERATION_OBSERVATION_V1_SCHEMA);
    let request = record("control-request", '5', COMMON_REQUEST_ENVELOPE_V1_SCHEMA);
    let candidate = OperationCustodyCandidate::new(
        store(),
        OperationCustodyTransitionKind::Observe,
        vec![
            OperationCustodyRecord::OperationObservation(successor.clone()),
            OperationCustodyRecord::CommonRequest(request.clone()),
        ],
        vec![OperationCustodyIndexMutation::LatestObservation {
            key: key('b'),
            expected: OperationCustodyIndexPredecessor::Exact(predecessor),
            successor: successor.coordinate().clone(),
        }],
        checkpoint_transition(),
        accumulator(&checkpoint_transition()),
    )
    .expect("one observation may retain the exact request it references");

    assert_eq!(
        candidate.record(OperationCustodyRecordKind::CommonRequest),
        Some(&request)
    );
    assert_eq!(
        candidate
            .indexes()
            .iter()
            .filter(|index| matches!(
                index,
                OperationCustodyIndexMutation::LatestObservation { .. }
            ))
            .count(),
        1,
        "a continuation request does not create another start index"
    );
}

#[test]
fn observation_rejects_blind_retry_predecessors() {
    let successor = record("observation-2", '4', OPERATION_OBSERVATION_V1_SCHEMA);
    let error = OperationCustodyCandidate::new(
        store(),
        OperationCustodyTransitionKind::Observe,
        vec![OperationCustodyRecord::OperationObservation(
            successor.clone(),
        )],
        vec![OperationCustodyIndexMutation::LatestObservation {
            key: key('b'),
            expected: OperationCustodyIndexPredecessor::Absent,
            successor: successor.coordinate().clone(),
        }],
        checkpoint_transition(),
        accumulator(&checkpoint_transition()),
    )
    .expect_err("blind observation retry must fail");

    assert_eq!(error, OperationCustodyCandidateError::IndexPredecessor);
}

#[test]
fn immutable_record_references_cannot_be_reused_with_another_digest() {
    let predecessor = record("observation-1", '3', OPERATION_OBSERVATION_V1_SCHEMA);
    let successor = record("observation-1", '4', OPERATION_OBSERVATION_V1_SCHEMA);
    let error = OperationCustodyCandidate::new(
        store(),
        OperationCustodyTransitionKind::Observe,
        vec![OperationCustodyRecord::OperationObservation(
            successor.clone(),
        )],
        vec![OperationCustodyIndexMutation::LatestObservation {
            key: key('b'),
            expected: OperationCustodyIndexPredecessor::Exact(predecessor),
            successor: successor.coordinate().clone(),
        }],
        checkpoint_transition(),
        accumulator(&checkpoint_transition()),
    )
    .expect_err("one immutable record reference cannot identify two byte identities");

    assert_eq!(
        error,
        OperationCustodyCandidateError::DuplicateRecordReference
    );
}

#[test]
fn checkpoint_successor_requires_a_fresh_immutable_record_reference() {
    let predecessor = record("observation-1", '3', OPERATION_OBSERVATION_V1_SCHEMA);
    let successor = record("observation-2", '4', OPERATION_OBSERVATION_V1_SCHEMA);
    let checkpoint = OperationCustodyCheckpoint::new(
        OperationCheckpointPhase::Successor {
            predecessor: checkpoint(8, "checkpoint", '8'),
            predecessor_record: Box::new(record(
                "checkpoint",
                '8',
                CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
            )),
        },
        checkpoint(9, "checkpoint", '9'),
        record("checkpoint", '9', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
    )
    .expect("checkpoint coordinates otherwise form an exact successor");
    let error = OperationCustodyCandidate::new(
        store(),
        OperationCustodyTransitionKind::Observe,
        vec![OperationCustodyRecord::OperationObservation(
            successor.clone(),
        )],
        vec![OperationCustodyIndexMutation::LatestObservation {
            key: key('b'),
            expected: OperationCustodyIndexPredecessor::Exact(predecessor),
            successor: successor.coordinate().clone(),
        }],
        checkpoint.clone(),
        accumulator(&checkpoint),
    )
    .expect_err("checkpoint succession cannot recycle one immutable reference");

    assert_eq!(
        error,
        OperationCustodyCandidateError::DuplicateRecordReference
    );
}

#[test]
fn checkpoint_body_substitution_is_rejected_before_candidate_custody() {
    let predecessor = checkpoint(8, "checkpoint-8", '8');
    let predecessor_record = record("checkpoint-8", '8', CONTROL_STATE_CHECKPOINT_V1_SCHEMA);
    let successor = checkpoint(9, "checkpoint-9", '9');
    let substituted_successor = record("checkpoint-9", 'e', CONTROL_STATE_CHECKPOINT_V1_SCHEMA);
    assert_eq!(
        OperationCustodyCheckpoint::new(
            OperationCheckpointPhase::Successor {
                predecessor,
                predecessor_record: Box::new(predecessor_record),
            },
            successor,
            substituted_successor,
        ),
        Err(OperationCustodyCandidateError::SuccessorCheckpoint)
    );
}

#[test]
fn point_observation_keeps_request_evidence_and_handle_domains_distinct() {
    let query = OperationCustodyIndexQuery::RequestEvidence(key('a'));
    let wrong = record("handle-1", '2', OPERATION_HANDLE_V1_SCHEMA);
    let error = OperationCustodyIndexObservation::exact(query, wrong)
        .expect_err("a handle cannot answer request evidence");

    assert_eq!(error, OperationCustodyObservationError::RecordSchema);
}

#[test]
fn recovery_indexes_are_derived_once_and_keep_their_record_domains() {
    let first = coordinate("observation-a", '3');
    let alias = coordinate("observation-b", '3');
    let changed = coordinate("observation-a", '4');
    let first_key = OperationCustodyIndexKey::for_observation(&first);
    assert_eq!(first_key, OperationCustodyIndexKey::for_observation(&first));
    assert_ne!(first_key, OperationCustodyIndexKey::for_observation(&alias));
    assert_ne!(
        first_key,
        OperationCustodyIndexKey::for_observation(&changed)
    );
    let candidate = OperationCustodyCandidate::new_start(
        store(),
        OperationCustodyTransitionKind::StartRoot,
        start_records(),
        root_start_indexes(),
        checkpoint_transition(),
        accumulator(&checkpoint_transition()),
        start_session_basis(),
    )
    .expect("start");
    let initial = candidate
        .record(OperationCustodyRecordKind::OperationObservation)
        .expect("initial");
    let request = candidate
        .record(OperationCustodyRecordKind::CommonRequest)
        .unwrap();
    assert!(candidate.indexes().iter().any(|index| matches!(index,
        OperationCustodyIndexMutation::RequestForHandle { key: observed, successor }
            if observed == &key('b') && successor == request.coordinate()
    )));
    assert!(candidate.indexes().iter().any(|index| matches!(index,
        OperationCustodyIndexMutation::StartObservation { key: observed, successor }
            if observed == &key('a') && successor == initial.coordinate()
    )));
    assert!(candidate.indexes().iter().any(|index| matches!(index,
        OperationCustodyIndexMutation::ObservationCheckpoint { key: observed, successor }
            if observed == &OperationCustodyIndexKey::for_observation(initial.coordinate())
                && successor == candidate.checkpoint().successor_record().coordinate()
    )));
    for query in [
        OperationCustodyIndexQuery::RequestForHandle(key('b')),
        OperationCustodyIndexQuery::StartObservation(key('a')),
        OperationCustodyIndexQuery::ObservationCheckpoint(key('3')),
    ] {
        assert_eq!(
            OperationCustodyIndexObservation::exact(
                query,
                record("handle", '2', OPERATION_HANDLE_V1_SCHEMA)
            )
            .expect_err("wrong record role"),
            OperationCustodyObservationError::RecordSchema
        );
    }
    let mut indexes = root_start_indexes();
    indexes.push(OperationCustodyIndexMutation::StartObservation {
        key: key('a'),
        successor: initial.coordinate().clone(),
    });
    assert_eq!(
        OperationCustodyCandidate::new_start(
            store(),
            OperationCustodyTransitionKind::StartRoot,
            start_records(),
            indexes,
            checkpoint_transition(),
            accumulator(&checkpoint_transition()),
            start_session_basis(),
        )
        .expect_err("caller cannot supply generated index"),
        OperationCustodyCandidateError::IndexShape
    );
}

#[test]
fn deterministic_reference_store_distinguishes_new_commit_replay_and_readback() {
    let candidate = OperationCustodyCandidate::new_start(
        store(),
        OperationCustodyTransitionKind::StartRoot,
        start_records(),
        root_start_indexes(),
        checkpoint_transition(),
        accumulator(&checkpoint_transition()),
        start_session_basis(),
    )
    .expect("root start");
    let mut custody = DeterministicCustody::default();

    assert_eq!(
        custody.compare_and_commit(&candidate).expect("new commit"),
        OperationCustodyCommitOutcome::Committed
    );
    assert_eq!(
        custody
            .compare_and_commit(&candidate)
            .expect("exact replay"),
        OperationCustodyCommitOutcome::ExactReplay
    );
    assert_eq!(
        custody.observe_exact(&candidate).expect("readback"),
        OperationCustodyReadBackOutcome::ExactSuccessor
    );
    let handle = custody
        .read_index(
            candidate.store(),
            &OperationCustodyIndexQuery::RequestOperation(key('a')),
            16 * 1024,
        )
        .expect("request operation lookup");
    assert_eq!(
        handle.record().expect("durable handle").coordinate(),
        &coordinate("handle-1", '2')
    );
    let request = custody
        .read_index(
            candidate.store(),
            &OperationCustodyIndexQuery::RequestForHandle(key('b')),
            16 * 1024,
        )
        .expect("handle request lookup");
    assert_eq!(
        request.record().unwrap().coordinate(),
        &coordinate("request-1", '1')
    );
    assert!(custody
        .read_index(
            candidate.store(),
            &OperationCustodyIndexQuery::RequestForHandle(key('a')),
            16 * 1024
        )
        .unwrap()
        .record()
        .is_none());
}

#[test]
fn handle_request_index_is_derived_for_both_start_kinds_and_cannot_be_injected() {
    for (kind, indexes) in [
        (
            OperationCustodyTransitionKind::StartRoot,
            root_start_indexes(),
        ),
        (
            OperationCustodyTransitionKind::StartChild,
            child_start_indexes(),
        ),
    ] {
        let checkpoint = checkpoint_transition();
        let candidate = OperationCustodyCandidate::new_start(
            store(),
            kind,
            start_records(),
            indexes.clone(),
            checkpoint.clone(),
            accumulator(&checkpoint),
            start_session_basis(),
        )
        .unwrap();
        let derived = OperationCustodyIndexMutation::RequestForHandle {
            key: key('b'),
            successor: coordinate("request-1", '1'),
        };
        assert_eq!(
            candidate
                .indexes()
                .iter()
                .filter(|entry| matches!(
                    entry,
                    OperationCustodyIndexMutation::RequestForHandle { .. }
                ))
                .count(),
            1
        );
        assert!(candidate.indexes().contains(&derived));
        for injected in [
            derived,
            OperationCustodyIndexMutation::RequestForHandle {
                key: key('c'),
                successor: coordinate("foreign-request", 'e'),
            },
        ] {
            let mut supplied = indexes.clone();
            supplied.push(injected);
            assert_eq!(
                OperationCustodyCandidate::new_start(
                    store(),
                    kind,
                    start_records(),
                    supplied,
                    checkpoint.clone(),
                    accumulator(&checkpoint),
                    start_session_basis(),
                )
                .unwrap_err(),
                OperationCustodyCandidateError::IndexShape
            );
        }
    }
}
