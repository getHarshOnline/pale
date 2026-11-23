//! Physical principal-bootstrap candidate closure tests.

use super::*;
use crate::{ControlRecordBytes, ControlRecordDigest};

use pale_core::records::{
    ControlCheckpointDomainIdV1, ControlDomainEpochV1, ControlDomainSequenceV1,
    ControlIncarnationIdV1, ControlStoreIdV1, PrincipalBootstrapIdV1,
    PrincipalTrustCheckpointEpochV1, ResidentPrincipalIdV1, ResidentRecordRefV1, SchemaDigest,
    SchemaId, UnixTimeV1,
};

fn digest(byte: char) -> ControlRecordDigest {
    ControlRecordDigest::parse(std::iter::repeat_n(byte, 64).collect::<String>()).expect("digest")
}

fn coordinate(reference: &str, byte: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(reference).expect("record ref"),
        digest(byte),
    )
}

fn record(coordinate: ControlRecordCoordinate) -> ControlRecord {
    ControlRecord::new(
        coordinate,
        SchemaId::parse("pale/test.v1").expect("schema"),
        SchemaDigest::parse("a".repeat(64)).expect("schema digest"),
        ControlRecordBytes::new(br#"{"record":"test"}"#.to_vec()).expect("record bytes"),
    )
}

fn candidate() -> PreparedPrincipalBootstrapAtom {
    let coordinates = [
        coordinate("principal", '1'),
        coordinate("recovery", '2'),
        coordinate("admin", '3'),
        coordinate("producer", '4'),
        coordinate("checkpoint-authority", '5'),
        coordinate("trust-checkpoint", '6'),
        coordinate("receipt", '7'),
        coordinate("control-checkpoint", '8'),
    ];
    let head = PrincipalBootstrapHead::new(
        PrincipalBootstrapIdV1::parse("bootstrap-1").expect("bootstrap id"),
        PrincipalBootstrapFixedRecords::new(
            coordinates[0].clone(),
            coordinates[1].clone(),
            coordinates[2].clone(),
            coordinates[3].clone(),
            coordinates[4].clone(),
            coordinates[6].clone(),
        ),
        PrincipalTrustCheckpointHead::new(
            ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
            PrincipalTrustCheckpointEpochV1::new(1).expect("trust epoch"),
            coordinates[5].clone(),
        ),
        1,
        UnixTimeV1::new(50).expect("time"),
    );
    let checkpoint = PrincipalBootstrapControlCheckpoint::new(
        1,
        ControlCheckpointDomainIdV1::parse("principal-head").expect("domain"),
        ControlDomainEpochV1::new(1).expect("epoch"),
        ControlDomainSequenceV1::new(1).expect("sequence"),
        pale_core::records::ControlTransactionSequenceV1::new(2).unwrap(),
        coordinates[7].clone(),
    )
    .expect("checkpoint");
    let binding = PrincipalTrustStoreBinding::new(
        ControlStoreIdV1::parse("trust-store-1").expect("store"),
        ControlIncarnationIdV1::parse("trust-incarnation-1").expect("incarnation"),
        ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
    );
    let batch = accumulator(
        crate::AuthorityStoreCoordinate::new(
            binding.trust_store_id().clone(),
            binding.trust_store_incarnation_id().clone(),
        ),
        checkpoint_head(&checkpoint),
        crate::AccumulatorCheckpointPhase::Initial,
    );
    PreparedPrincipalBootstrapAtom::new(
        binding,
        coordinates.into_iter().map(record).collect(),
        vec![checkpoint],
        head,
        vec![batch],
    )
    .expect("closed candidate")
}

#[test]
fn candidate_closes_exactly_over_head_and_checkpoint_coordinates() {
    let candidate = candidate();
    assert_eq!(candidate.records().len(), 8);
    assert_eq!(candidate.control_checkpoints().len(), 1);
    assert_eq!(candidate.head().control_checkpoint_count(), 1);
}

fn accumulator_binding(
    store: crate::AuthorityStoreCoordinate,
    checkpoint: crate::ControlCheckpointHead,
    phase: crate::AccumulatorCheckpointPhase,
) -> crate::AccumulatorCheckpointBinding {
    use crate::{AccumulatorCheckpointBinding, ControlTransactionPreparation};
    use pale_core::records::{CheckpointProfileDigest, ControlTransactionSequenceV1};
    let transaction = ControlTransactionPreparation::new(
        store.clone(),
        ControlTransactionSequenceV1::new(checkpoint.committed_transaction_sequence().get() - 1)
            .unwrap(),
    )
    .unwrap();
    AccumulatorCheckpointBinding::new(
        store,
        ResidentRecordRefV1::parse("physical-profile").unwrap(),
        CheckpointProfileDigest::parse("a".repeat(64)).unwrap(),
        phase,
        checkpoint,
        transaction,
    )
    .unwrap()
}

/// Physical attachment fixtures do not assert that these roots are admitted.
fn accumulator(
    store: crate::AuthorityStoreCoordinate,
    checkpoint: crate::ControlCheckpointHead,
    phase: crate::AccumulatorCheckpointPhase,
) -> crate::OperationAccumulatorBatch {
    use crate::{
        AccumulatorMutation, AccumulatorPosition, OperationAccumulatorBatch, StateAccumulatorBatch,
        UnresolvedCustodyAccumulatorBatch,
    };
    use pale_core::records::ArtifactDigest;
    let empty = ArtifactDigest::parse("0".repeat(64)).unwrap();
    OperationAccumulatorBatch::new(
        accumulator_binding(store, checkpoint, phase),
        StateAccumulatorBatch::new(
            empty.clone(),
            ArtifactDigest::parse("b".repeat(64)).unwrap(),
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
        UnresolvedCustodyAccumulatorBatch::new(empty.clone(), empty, vec![], 1).unwrap(),
    )
    .unwrap()
}

fn empty_initial_accumulator(
    store: crate::AuthorityStoreCoordinate,
    checkpoint: crate::ControlCheckpointHead,
) -> crate::OperationAccumulatorBatch {
    use crate::{
        AccumulatorCheckpointPhase, OperationAccumulatorBatch, StateAccumulatorBatch,
        UnresolvedCustodyAccumulatorBatch,
    };
    use pale_core::records::ArtifactDigest;
    let empty = ArtifactDigest::parse("0".repeat(64)).unwrap();
    OperationAccumulatorBatch::new(
        accumulator_binding(store, checkpoint, AccumulatorCheckpointPhase::Initial),
        StateAccumulatorBatch::new(empty.clone(), empty.clone(), vec![], 1).unwrap(),
        UnresolvedCustodyAccumulatorBatch::new(empty.clone(), empty, vec![], 1).unwrap(),
    )
    .unwrap()
}

fn checkpoint_head(
    checkpoint: &PrincipalBootstrapControlCheckpoint,
) -> crate::ControlCheckpointHead {
    crate::ControlCheckpointHead::new(
        checkpoint.checkpoint_domain_id().clone(),
        checkpoint.domain_epoch(),
        checkpoint.domain_sequence(),
        checkpoint.committed_transaction_sequence(),
        checkpoint.coordinate().clone(),
    )
}

fn accumulator_head(
    candidate: &PreparedPrincipalBootstrapAtom,
    index: usize,
) -> crate::ControlCheckpointHead {
    checkpoint_head(&candidate.control_checkpoints()[index])
}

fn rebuild_with_accumulators(
    candidate: &PreparedPrincipalBootstrapAtom,
    accumulators: Vec<crate::OperationAccumulatorBatch>,
) -> Result<PreparedPrincipalBootstrapAtom, PrincipalBootstrapCandidateError> {
    PreparedPrincipalBootstrapAtom::new(
        candidate.binding().clone(),
        candidate.records().to_vec(),
        candidate.control_checkpoints().to_vec(),
        candidate.head().clone(),
        accumulators,
    )
}

#[test]
fn initial_accumulator_set_closes_exact_physical_scope() {
    use crate::{AccumulatorCheckpointPhase, AuthorityStoreCoordinate, ControlCheckpointHead};
    use pale_core::records::ControlTransactionSequenceV1;
    let candidate = candidate();
    let store = AuthorityStoreCoordinate::new(
        candidate.binding().trust_store_id().clone(),
        candidate.binding().trust_store_incarnation_id().clone(),
    );
    let head = accumulator_head(&candidate, 0);
    let batch = accumulator(
        store.clone(),
        head.clone(),
        AccumulatorCheckpointPhase::Initial,
    );
    let attached = rebuild_with_accumulators(&candidate, vec![batch.clone()]).unwrap();
    assert_eq!(attached.accumulators(), std::slice::from_ref(&batch));
    assert_eq!(
        rebuild_with_accumulators(&attached, vec![]),
        Err(PrincipalBootstrapCandidateError::AccumulatorClosure),
    );

    for (changed_store, changed_head) in [
        (
            AuthorityStoreCoordinate::new(
                ControlStoreIdV1::parse("other-store").unwrap(),
                store.incarnation_id().clone(),
            ),
            head.clone(),
        ),
        (
            AuthorityStoreCoordinate::new(
                store.store_id().clone(),
                ControlIncarnationIdV1::parse("other-incarnation").unwrap(),
            ),
            head.clone(),
        ),
        (
            store.clone(),
            ControlCheckpointHead::new(
                ControlCheckpointDomainIdV1::parse("other-domain").unwrap(),
                head.domain_epoch(),
                head.domain_sequence(),
                head.committed_transaction_sequence(),
                head.coordinate().clone(),
            ),
        ),
        (
            store.clone(),
            ControlCheckpointHead::new(
                head.checkpoint_domain_id().clone(),
                ControlDomainEpochV1::new(2).unwrap(),
                head.domain_sequence(),
                head.committed_transaction_sequence(),
                head.coordinate().clone(),
            ),
        ),
        (
            store.clone(),
            ControlCheckpointHead::new(
                head.checkpoint_domain_id().clone(),
                head.domain_epoch(),
                head.domain_sequence(),
                head.committed_transaction_sequence(),
                ControlRecordCoordinate::new(
                    ResidentRecordRefV1::parse("other-checkpoint").unwrap(),
                    head.coordinate().record_sha256().clone(),
                ),
            ),
        ),
        (
            store.clone(),
            ControlCheckpointHead::new(
                head.checkpoint_domain_id().clone(),
                head.domain_epoch(),
                head.domain_sequence(),
                head.committed_transaction_sequence(),
                ControlRecordCoordinate::new(head.coordinate().record_ref().clone(), digest('9')),
            ),
        ),
        (
            store.clone(),
            ControlCheckpointHead::new(
                head.checkpoint_domain_id().clone(),
                head.domain_epoch(),
                head.domain_sequence(),
                ControlTransactionSequenceV1::new(3).unwrap(),
                head.coordinate().clone(),
            ),
        ),
    ] {
        let batch = accumulator(
            changed_store,
            changed_head,
            AccumulatorCheckpointPhase::Initial,
        );
        assert_eq!(
            rebuild_with_accumulators(&candidate, vec![batch]),
            Err(PrincipalBootstrapCandidateError::AccumulatorClosure)
        );
    }
    let successor = ControlCheckpointHead::new(
        head.checkpoint_domain_id().clone(),
        head.domain_epoch(),
        ControlDomainSequenceV1::new(2).unwrap(),
        ControlTransactionSequenceV1::new(3).unwrap(),
        coordinate("successor", '9'),
    );
    let batch = accumulator(
        store,
        successor,
        AccumulatorCheckpointPhase::Successor(head),
    );
    assert_eq!(
        rebuild_with_accumulators(&candidate, vec![batch]),
        Err(PrincipalBootstrapCandidateError::AccumulatorClosure)
    );
}

#[test]
fn duplicate_or_unreferenced_record_fails_closed() {
    let valid = candidate();
    let mut records = valid.records().to_vec();
    records[7] = records[0].clone();
    assert_eq!(
        PreparedPrincipalBootstrapAtom::new(
            valid.binding().clone(),
            records,
            valid.control_checkpoints().to_vec(),
            valid.head().clone(),
            valid.accumulators().to_vec(),
        ),
        Err(PrincipalBootstrapCandidateError::ReferenceClosure)
    );
}

#[test]
fn ordered_accumulators_close_bijectively_over_multiple_checkpoints() {
    let valid = candidate();
    let second = coordinate("second-checkpoint", '9');
    let mut records = valid.records().to_vec();
    records.push(record(second.clone()));
    let mut checkpoints = valid.control_checkpoints().to_vec();
    checkpoints.push(
        PrincipalBootstrapControlCheckpoint::new(
            2,
            ControlCheckpointDomainIdV1::parse("second-domain").unwrap(),
            ControlDomainEpochV1::new(1).unwrap(),
            ControlDomainSequenceV1::new(1).unwrap(),
            checkpoints[0].committed_transaction_sequence(),
            second,
        )
        .unwrap(),
    );
    let head = PrincipalBootstrapHead::new(
        valid.head.bootstrap_id.clone(),
        valid.head.fixed_records.clone(),
        valid.head.trust_checkpoint.clone(),
        2,
        valid.head.logical_committed_at,
    );
    let store = crate::AuthorityStoreCoordinate::new(
        valid.binding().trust_store_id().clone(),
        valid.binding().trust_store_incarnation_id().clone(),
    );
    let first = accumulator(
        store.clone(),
        checkpoint_head(&checkpoints[0]),
        crate::AccumulatorCheckpointPhase::Initial,
    );
    let second = empty_initial_accumulator(store.clone(), checkpoint_head(&checkpoints[1]));
    let multiple = PreparedPrincipalBootstrapAtom::new(
        valid.binding().clone(),
        records,
        checkpoints,
        head,
        vec![first.clone(), second.clone()],
    )
    .unwrap();
    assert_eq!(multiple.accumulators(), &[first.clone(), second.clone()]);

    for batches in [
        vec![],
        vec![first.clone()],
        vec![first.clone(), second.clone(), first.clone()],
        vec![second.clone(), first.clone()],
        vec![first.clone(), first.clone()],
    ] {
        assert_eq!(
            rebuild_with_accumulators(&multiple, batches),
            Err(PrincipalBootstrapCandidateError::AccumulatorClosure)
        );
    }

    let other_transaction = crate::ControlTransactionPreparation::new(
        first.binding().store().clone(),
        pale_core::records::ControlTransactionSequenceV1::new(2).unwrap(),
    )
    .unwrap();
    let other_transaction = crate::OperationAccumulatorBatch::new(
        crate::AccumulatorCheckpointBinding::new(
            second.binding().store().clone(),
            second.binding().profile_ref().clone(),
            second.binding().profile_sha256().clone(),
            crate::AccumulatorCheckpointPhase::Initial,
            crate::ControlCheckpointHead::new(
                second.binding().successor().checkpoint_domain_id().clone(),
                second.binding().successor().domain_epoch(),
                second.binding().successor().domain_sequence(),
                other_transaction.successor(),
                second.binding().successor().coordinate().clone(),
            ),
            other_transaction,
        )
        .unwrap(),
        second.state().clone(),
        second.unresolved().clone(),
    )
    .unwrap();
    assert_eq!(
        rebuild_with_accumulators(&multiple, vec![first, other_transaction]),
        Err(PrincipalBootstrapCandidateError::AccumulatorClosure)
    );
}

#[test]
fn duplicate_physical_identity_fails_before_store_access() {
    let valid = candidate();
    let mut records = valid.records().to_vec();
    let duplicate_ref = records[0].coordinate().record_ref().clone();
    let distinct_coordinate = ControlRecordCoordinate::new(duplicate_ref, digest('9'));
    records[1] = record(distinct_coordinate.clone());
    let head = PrincipalBootstrapHead::new(
        valid.head().bootstrap_id().clone(),
        PrincipalBootstrapFixedRecords::new(
            valid.head().principal_record().clone(),
            distinct_coordinate,
            valid.head().admin_authority().clone(),
            valid.head().producer_authority().clone(),
            valid.head().checkpoint_authority().clone(),
            valid.head().receipt().clone(),
        ),
        valid.head().trust_checkpoint().clone(),
        valid.head().control_checkpoint_count(),
        valid.head().logical_committed_at(),
    );
    assert_eq!(
        PreparedPrincipalBootstrapAtom::new(
            valid.binding().clone(),
            records,
            valid.control_checkpoints().to_vec(),
            head,
            valid.accumulators().to_vec(),
        ),
        Err(PrincipalBootstrapCandidateError::ReferenceClosure)
    );
}

#[test]
fn digest_and_checkpoint_ordinal_are_validated_at_their_owners() {
    assert_eq!(
        ControlRecordDigest::parse("A".repeat(64)),
        Err(ControlRecordError::InvalidDigest)
    );
    assert_eq!(
        PrincipalBootstrapControlCheckpoint::new(
            0,
            ControlCheckpointDomainIdV1::parse("principal-head").unwrap(),
            ControlDomainEpochV1::new(1).unwrap(),
            ControlDomainSequenceV1::new(1).unwrap(),
            pale_core::records::ControlTransactionSequenceV1::new(2).unwrap(),
            coordinate("checkpoint", '1'),
        ),
        Err(PrincipalBootstrapCandidateError::InvalidCheckpointOrdinal)
    );
}

#[test]
fn mutable_trust_head_must_be_epoch_one_for_the_bound_principal() {
    let valid = candidate();
    for trust_checkpoint in [
        PrincipalTrustCheckpointHead::new(
            ResidentPrincipalIdV1::parse("other-principal").unwrap(),
            PrincipalTrustCheckpointEpochV1::new(1).unwrap(),
            valid.head().trust_checkpoint().coordinate().clone(),
        ),
        PrincipalTrustCheckpointHead::new(
            valid.binding().principal_id().clone(),
            PrincipalTrustCheckpointEpochV1::new(2).unwrap(),
            valid.head().trust_checkpoint().coordinate().clone(),
        ),
    ] {
        let head = PrincipalBootstrapHead::new(
            valid.head().bootstrap_id().clone(),
            PrincipalBootstrapFixedRecords::new(
                valid.head().principal_record().clone(),
                valid.head().recovery_policy().clone(),
                valid.head().admin_authority().clone(),
                valid.head().producer_authority().clone(),
                valid.head().checkpoint_authority().clone(),
                valid.head().receipt().clone(),
            ),
            trust_checkpoint,
            valid.head().control_checkpoint_count(),
            valid.head().logical_committed_at(),
        );
        assert_eq!(
            PreparedPrincipalBootstrapAtom::new(
                valid.binding().clone(),
                valid.records().to_vec(),
                valid.control_checkpoints().to_vec(),
                head,
                valid.accumulators().to_vec(),
            ),
            Err(PrincipalBootstrapCandidateError::InvalidTrustCheckpointHead)
        );
    }
}
