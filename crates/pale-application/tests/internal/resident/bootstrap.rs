//! Principal-bootstrap database-state transition tests.

use super::*;

use pale_core::records::{
    ArtifactDigest, ControlCheckpointDomainIdV1, ControlDomainEpochV1, ControlDomainSequenceV1,
    ControlIncarnationIdV1, ControlStoreIdV1, PrincipalBootstrapIdV1,
    PrincipalTrustCheckpointEpochV1, ResidentPrincipalIdV1, ResidentRecordRefV1, SchemaDigest,
    SchemaId, UnixTimeV1,
};
use pale_ports::{ControlRecord, ControlRecordBytes, ControlRecordCoordinate, ControlRecordDigest};

struct FakeStore {
    commit: Option<Result<PrincipalBootstrapCommitDisposition, ControlStoreError>>,
    observation: Option<Result<PrincipalBootstrapDatabaseObservation, ControlStoreError>>,
    observation_calls: usize,
}

impl pale_ports::ControlTransactionPort for FakeStore {
    fn prepare_transaction(
        &mut self,
        _: &pale_ports::AuthorityStoreCoordinate,
    ) -> Result<pale_ports::ControlTransactionPreparation, pale_ports::ControlStoreError> {
        panic!("this physical-custody/readback fixture does not prepare a new checkpoint")
    }
}

impl PrincipalBootstrapCustodyPort for FakeStore {
    fn prepare_accumulator(
        &mut self,
        _: &pale_ports::OperationAccumulatorPreparation,
    ) -> Result<
        pale_ports::PreparedOperationAccumulator,
        pale_ports::OperationAccumulatorPreparationError,
    > {
        panic!("this physical-custody/readback fixture does not prepare checkpoint trees")
    }

    fn compare_and_commit(
        &mut self,
        _: &PreparedPrincipalBootstrapAtom,
    ) -> Result<PrincipalBootstrapCommitDisposition, ControlStoreError> {
        self.commit.take().expect("one commit call")
    }

    fn observe_exact(
        &mut self,
        _: &PreparedPrincipalBootstrapAtom,
    ) -> Result<PrincipalBootstrapDatabaseObservation, ControlStoreError> {
        self.observation_calls += 1;
        self.observation.take().expect("one observation call")
    }
}

fn digest(byte: char) -> ControlRecordDigest {
    ControlRecordDigest::parse(std::iter::repeat_n(byte, 64).collect::<String>()).unwrap()
}

fn coordinate(reference: &str, byte: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(ResidentRecordRefV1::parse(reference).unwrap(), digest(byte))
}

fn prepared() -> PreparedPrincipalBootstrapCustody {
    let coordinates = [
        coordinate("principal", '1'),
        coordinate("recovery", '2'),
        coordinate("admin", '3'),
        coordinate("producer", '4'),
        coordinate("checkpoint-authority", '5'),
        coordinate("trust", '6'),
        coordinate("receipt", '7'),
        coordinate("control", '8'),
    ];
    let records = coordinates
        .iter()
        .cloned()
        .map(|coordinate| {
            ControlRecord::new(
                coordinate,
                SchemaId::parse("pale/test.v1").unwrap(),
                SchemaDigest::parse("a".repeat(64)).unwrap(),
                ControlRecordBytes::new(br#"{"record":"test"}"#.to_vec()).unwrap(),
            )
        })
        .collect();
    let checkpoints = vec![PrincipalBootstrapControlCheckpoint::new(
        1,
        ControlCheckpointDomainIdV1::parse("principal-head").unwrap(),
        ControlDomainEpochV1::new(1).unwrap(),
        ControlDomainSequenceV1::new(1).unwrap(),
        pale_core::records::ControlTransactionSequenceV1::new(2).unwrap(),
        coordinates[7].clone(),
    )
    .unwrap()];
    let head = PrincipalBootstrapHead::new(
        PrincipalBootstrapIdV1::parse("bootstrap-1").unwrap(),
        PrincipalBootstrapFixedRecords::new(
            coordinates[0].clone(),
            coordinates[1].clone(),
            coordinates[2].clone(),
            coordinates[3].clone(),
            coordinates[4].clone(),
            coordinates[6].clone(),
        ),
        PrincipalTrustCheckpointHead::new(
            ResidentPrincipalIdV1::parse("principal-1").unwrap(),
            PrincipalTrustCheckpointEpochV1::new(1).unwrap(),
            coordinates[5].clone(),
        ),
        1,
        UnixTimeV1::new(50).unwrap(),
    );
    let binding = PrincipalTrustStoreBinding::new(
        ControlStoreIdV1::parse("store-1").unwrap(),
        ControlIncarnationIdV1::parse("incarnation-1").unwrap(),
        ResidentPrincipalIdV1::parse("principal-1").unwrap(),
    );
    let store = pale_ports::AuthorityStoreCoordinate::new(
        binding.trust_store_id().clone(),
        binding.trust_store_incarnation_id().clone(),
    );
    let transaction = pale_ports::ControlTransactionPreparation::new(
        store.clone(),
        pale_core::records::ControlTransactionSequenceV1::new(1).unwrap(),
    )
    .unwrap();
    let zero = ArtifactDigest::parse("0".repeat(64)).unwrap();
    let accumulator = pale_ports::OperationAccumulatorBatch::new(
        pale_ports::AccumulatorCheckpointBinding::new(
            store,
            ResidentRecordRefV1::parse("physical-profile").unwrap(),
            pale_core::records::CheckpointProfileDigest::parse("a".repeat(64)).unwrap(),
            pale_ports::AccumulatorCheckpointPhase::Initial,
            pale_ports::ControlCheckpointHead::new(
                checkpoints[0].checkpoint_domain_id().clone(),
                checkpoints[0].domain_epoch(),
                checkpoints[0].domain_sequence(),
                checkpoints[0].committed_transaction_sequence(),
                checkpoints[0].coordinate().clone(),
            ),
            transaction,
        )
        .unwrap(),
        pale_ports::StateAccumulatorBatch::new(
            zero.clone(),
            ArtifactDigest::parse("b".repeat(64)).unwrap(),
            vec![pale_ports::AccumulatorMutation::new(
                pale_ports::AccumulatorPosition::Leaf([1; 32]),
                None,
                Some(vec![2; 32]),
                32,
            )
            .unwrap()],
            1,
        )
        .unwrap(),
        pale_ports::UnresolvedCustodyAccumulatorBatch::new(zero.clone(), zero, vec![], 1).unwrap(),
    )
    .unwrap();
    PreparedPrincipalBootstrapCustody::from_candidate(
        PreparedPrincipalBootstrapAtom::new(binding, records, checkpoints, head, vec![accumulator])
            .unwrap(),
        vec![
            PreparedAntiRollbackAnchorTransition::initial(
                PrincipalTrustMinimumAnchor::new(
                    ResidentPrincipalIdV1::parse("principal-1").unwrap(),
                    PrincipalTrustCheckpointEpochV1::new(1).unwrap(),
                    coordinates[5].clone(),
                )
                .into(),
            )
            .unwrap(),
            PreparedAntiRollbackAnchorTransition::initial(
                ControlDomainAntiRollbackAnchor::new(
                    ControlStoreIdV1::parse("principal-control-1").unwrap(),
                    ControlIncarnationIdV1::parse("control-incarnation-1").unwrap(),
                    ControlCheckpointDomainIdV1::parse("principal-head").unwrap(),
                    ControlDomainEpochV1::new(1).unwrap(),
                    ControlDomainSequenceV1::new(1).unwrap(),
                    coordinates[7].clone(),
                )
                .into(),
            )
            .unwrap(),
        ],
    )
}

#[path = "bootstrap/anchors.rs"]
mod anchors;

#[test]
fn ordinary_commit_advances_only_to_database_committed_anchors_pending() {
    let mut store = FakeStore {
        commit: Some(Ok(PrincipalBootstrapCommitDisposition::New)),
        observation: None,
        observation_calls: 0,
    };
    let committed = prepared().commit(&mut store).unwrap();
    assert_eq!(
        committed.phase(),
        PrincipalBootstrapCustodyPhase::DatabaseCommittedAnchorsPending
    );
    assert_eq!(committed.evidence(), PrincipalBootstrapCommitEvidence::New);
    assert_eq!(store.observation_calls, 0);
}

#[test]
fn ambiguous_commit_requires_and_accepts_exact_read_back() {
    let mut store = FakeStore {
        commit: Some(Err(ControlStoreError::new(
            ControlStoreErrorKind::OutcomeUnknown,
        ))),
        observation: Some(Ok(PrincipalBootstrapDatabaseObservation::Exact)),
        observation_calls: 0,
    };
    let committed = prepared().commit(&mut store).unwrap();
    assert_eq!(
        committed.evidence(),
        PrincipalBootstrapCommitEvidence::RecoveredExactAfterAmbiguousCommit
    );
    assert_eq!(store.observation_calls, 1);
}

#[test]
fn empty_read_back_retains_the_prepared_state_instead_of_retrying() {
    let mut store = FakeStore {
        commit: Some(Err(ControlStoreError::new(
            ControlStoreErrorKind::OutcomeUnknown,
        ))),
        observation: Some(Ok(PrincipalBootstrapDatabaseObservation::Empty)),
        observation_calls: 0,
    };
    let failure = prepared().commit(&mut store).unwrap_err();
    assert!(matches!(
        failure.reason(),
        PrincipalBootstrapCommitFailureReason::CommitExcludedByExactReadBack { .. }
    ));
    assert_eq!(
        failure.into_prepared().phase(),
        PrincipalBootstrapCustodyPhase::Prepared
    );
    assert_eq!(store.observation_calls, 1);
}

#[test]
fn conflicting_read_back_fails_closed_for_explicit_recovery() {
    let mut store = FakeStore {
        commit: Some(Err(ControlStoreError::new(
            ControlStoreErrorKind::OutcomeUnknown,
        ))),
        observation: Some(Ok(
            PrincipalBootstrapDatabaseObservation::ConflictingOrPartial,
        )),
        observation_calls: 0,
    };
    let failure = prepared().commit(&mut store).unwrap_err();
    assert!(matches!(
        failure.reason(),
        PrincipalBootstrapCommitFailureReason::RecoveryRequiredAfterExactReadBack { .. }
    ));
    assert_eq!(store.observation_calls, 1);
}

#[test]
fn failed_read_back_preserves_uncertainty_and_the_exact_prepared_state() {
    let original = prepared();
    let mut store = FakeStore {
        commit: Some(Err(ControlStoreError::with_source(
            ControlStoreErrorKind::OutcomeUnknown,
            std::io::Error::other("commit outcome remains unknown"),
        ))),
        observation: Some(Err(ControlStoreError::with_source(
            ControlStoreErrorKind::RecoveryRequired,
            std::io::Error::other("read-back store binding does not match"),
        ))),
        observation_calls: 0,
    };
    let failure = original.clone().commit(&mut store).unwrap_err();
    let PrincipalBootstrapCommitFailureReason::ExactReadBackFailed {
        uncertain,
        read_back,
    } = failure.reason()
    else {
        panic!("failed read-back must not resolve the uncertain commit");
    };
    assert_eq!(uncertain.kind(), ControlStoreErrorKind::OutcomeUnknown);
    assert_eq!(read_back.kind(), ControlStoreErrorKind::RecoveryRequired);
    assert_eq!(
        std::error::Error::source(uncertain).unwrap().to_string(),
        "commit outcome remains unknown"
    );
    assert_eq!(
        std::error::Error::source(read_back).unwrap().to_string(),
        "read-back store binding does not match"
    );
    assert_eq!(failure.into_prepared(), original);
    assert_eq!(store.observation_calls, 1);
    // Each one-shot result was consumed once; a retry would panic in FakeStore.
    assert!(store.commit.is_none());
    assert!(store.observation.is_none());
}
