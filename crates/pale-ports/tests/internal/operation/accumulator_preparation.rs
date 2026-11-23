//! Input, identity and consuming-binding laws, not Merkle-proof qualification.

use super::*;
use crate::{
    AccumulatorImplementation, AccumulatorLeafChange, AccumulatorPreparationBasis,
    AccumulatorPreparationInputError, AccumulatorPreparationLimits, OperationAccumulatorCheckpoint,
    OperationAccumulatorPreparation, PreparedOperationAccumulator, StateLeafChanges,
    UnresolvedCustodyLeafChanges,
};

fn hash(byte: char) -> ArtifactDigest {
    ArtifactDigest::parse(digest(byte)).expect("digest")
}

fn change(byte: char) -> AccumulatorLeafChange {
    AccumulatorLeafChange::new(hash(byte), Some(hash('a')), Some(hash('b'))).unwrap()
}

fn limits(leaves: usize) -> AccumulatorPreparationLimits {
    AccumulatorPreparationLimits::new(leaves, 512, 512, 512, 16_384).unwrap()
}

fn initial_basis() -> AccumulatorPreparationBasis {
    AccumulatorPreparationBasis::Initial(crate::AccumulatorInitialDomain::new(
        store(),
        ResidentRecordRefV1::parse("profile").unwrap(),
        CheckpointProfileDigest::parse(digest('a')).unwrap(),
        checkpoint(1, "initial", '1').checkpoint_domain_id().clone(),
        checkpoint(1, "initial", '1').domain_epoch(),
    ))
}

fn empty_initial_request() -> OperationAccumulatorPreparation {
    OperationAccumulatorPreparation::new(
        initial_basis(),
        StateLeafChanges::new(Vec::new(), 1).unwrap(),
        UnresolvedCustodyLeafChanges::new(Vec::new(), 1).unwrap(),
        limits(1),
        AccumulatorImplementation::SparseMerkleTree061Sha256PaleV1,
        crate::ControlTransactionPreparation::new(
            store(),
            pale_core::records::ControlTransactionSequenceV1::new(20).unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}

fn predecessor(batch: &OperationAccumulatorBatch) -> OperationAccumulatorCheckpoint {
    let binding = batch.binding();
    let AccumulatorCheckpointPhase::Successor(head) = binding.phase() else {
        panic!("replacement fixture")
    };
    OperationAccumulatorCheckpoint::new(
        binding.store().clone(),
        binding.profile_ref().clone(),
        binding.profile_sha256().clone(),
        head.clone(),
        batch.state().predecessor_root().clone(),
        batch.unresolved().predecessor_root().clone(),
    )
}

#[test]
fn membership_is_present_ordered_bounded_and_retains_the_entire_exact_request() {
    use crate::{
        AccumulatorMembershipInputError as Error, AccumulatorMembershipLeaf as Leaf,
        AccumulatorMembershipLimits as Limits, OperationAccumulatorMembership as Request,
        VerifiedOperationAccumulatorMembership as Report,
    };
    let batch = accumulator(&checkpoint_transition());
    let selected = predecessor(&batch);
    assert_eq!(Leaf::new(hash('1'), hash('0')), Err(Error::PresentZero));
    for bounds in [(0, 1, 1), (1, 0, 1), (1, 1, 0)] {
        assert_eq!(
            Limits::new(bounds.0, bounds.1, bounds.2),
            Err(Error::Bounds)
        );
    }
    let limit = Limits::new(2, 512, 16384).unwrap();
    let implementation = AccumulatorImplementation::SparseMerkleTree061Sha256PaleV1;
    let leaves = vec![
        Leaf::new(hash('1'), hash('a')).unwrap(),
        Leaf::new(hash('2'), hash('b')).unwrap(),
    ];
    assert_eq!(
        Request::new(selected.clone(), vec![], limit, implementation),
        Err(Error::Bounds)
    );
    assert_eq!(
        Request::new(
            selected.clone(),
            vec![leaves[1].clone(), leaves[0].clone()],
            limit,
            implementation
        ),
        Err(Error::Order)
    );
    assert_eq!(
        Request::new(
            selected.clone(),
            vec![leaves[0].clone(), leaves[0].clone()],
            limit,
            implementation
        ),
        Err(Error::Order)
    );
    assert_eq!(
        Request::new(
            selected.clone(),
            leaves.clone(),
            Limits::new(1, 512, 16384).unwrap(),
            implementation
        ),
        Err(Error::Bounds)
    );
    let request = Request::new(selected.clone(), leaves.clone(), limit, implementation).unwrap();
    let report = Report::new(request.clone());
    assert_eq!(report.request(), &request);
    for alternate in [
        Limits::new(3, 512, 16384).unwrap(),
        Limits::new(2, 513, 16384).unwrap(),
        Limits::new(2, 512, 16385).unwrap(),
    ] {
        assert_ne!(
            report.request(),
            &Request::new(selected.clone(), leaves.clone(), alternate, implementation).unwrap()
        );
    }
    assert_eq!(request.checkpoint(), &selected);
}

#[test]
fn present_zero_is_not_absence_and_noop_changes_are_rejected() {
    for (prior, next) in [(None, Some(hash('0'))), (Some(hash('0')), None)] {
        assert_eq!(
            AccumulatorLeafChange::new(hash('1'), prior, next),
            Err(AccumulatorPreparationInputError::PresentZero)
        );
    }
    for (prior, next) in [(None, None), (Some(hash('a')), Some(hash('a')))] {
        assert_eq!(
            AccumulatorLeafChange::new(hash('1'), prior, next),
            Err(AccumulatorPreparationInputError::NoChange)
        );
    }
    let insertion = AccumulatorLeafChange::new(hash('1'), None, Some(hash('a'))).unwrap();
    let removal = AccumulatorLeafChange::new(hash('1'), Some(hash('a')), None).unwrap();
    assert!(insertion.predecessor().is_none());
    assert!(removal.successor().is_none());
    assert_eq!(insertion.successor(), removal.predecessor());
}

#[test]
fn leaf_bounds_precede_order_validation_and_apply_to_both_trees_together() {
    assert_eq!(
        StateLeafChanges::new(vec![change('2'), change('1')], 1),
        Err(AccumulatorPreparationInputError::Bounds)
    );
    assert_eq!(
        UnresolvedCustodyLeafChanges::new(vec![change('2'), change('1')], 1),
        Err(AccumulatorPreparationInputError::Bounds)
    );
    assert_eq!(
        StateLeafChanges::new(vec![change('2'), change('1')], 2),
        Err(AccumulatorPreparationInputError::Order)
    );
    assert_eq!(
        UnresolvedCustodyLeafChanges::new(vec![change('1'), change('1')], 2),
        Err(AccumulatorPreparationInputError::Order)
    );
    assert_eq!(
        StateLeafChanges::new(Vec::new(), 0),
        Err(AccumulatorPreparationInputError::Bounds)
    );
    for zero_at in 0..5 {
        let mut values = [1; 5];
        values[zero_at] = 0;
        assert_eq!(
            AccumulatorPreparationLimits::new(
                values[0], values[1], values[2], values[3], values[4]
            ),
            Err(AccumulatorPreparationInputError::Bounds)
        );
    }
    let batch = accumulator(&checkpoint_transition());
    let request = OperationAccumulatorPreparation::new(
        AccumulatorPreparationBasis::Successor(predecessor(&batch)),
        StateLeafChanges::new(vec![change('1')], 1).unwrap(),
        UnresolvedCustodyLeafChanges::new(vec![change('1')], 1).unwrap(),
        limits(1),
        AccumulatorImplementation::SparseMerkleTree061Sha256PaleV1,
        batch.binding().transaction().clone(),
    );
    assert_eq!(request, Err(AccumulatorPreparationInputError::Bounds));
    let request = OperationAccumulatorPreparation::new(
        AccumulatorPreparationBasis::Successor(predecessor(&batch)),
        StateLeafChanges::new(Vec::new(), 1).unwrap(),
        UnresolvedCustodyLeafChanges::new(Vec::new(), 1).unwrap(),
        limits(1),
        AccumulatorImplementation::SparseMerkleTree061Sha256PaleV1,
        batch.binding().transaction().clone(),
    );
    assert_eq!(request, Err(AccumulatorPreparationInputError::Empty));
}

#[test]
fn only_an_initial_preparation_can_preserve_an_empty_accumulator() {
    let request = empty_initial_request();
    let zero = hash('0');
    let state = StateAccumulatorBatch::new(zero.clone(), zero.clone(), Vec::new(), 1).unwrap();
    let unresolved =
        UnresolvedCustodyAccumulatorBatch::new(zero.clone(), zero.clone(), Vec::new(), 1).unwrap();
    let prepared =
        PreparedOperationAccumulator::new(request.clone(), state.clone(), unresolved.clone())
            .unwrap();
    assert_eq!(prepared.request(), &request);
    assert_eq!(prepared.state_root(), &zero);
    assert_eq!(prepared.unresolved_root(), &zero);
    assert_eq!(
        request.state().changes().len() + request.unresolved().changes().len(),
        0
    );
    assert!(state.mutations().is_empty());
    assert!(unresolved.mutations().is_empty());
    let batch = prepared
        .bind_successor(checkpoint(1, "empty-initial", '1'))
        .unwrap();
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

    let nonzero = StateAccumulatorBatch::new(hash('1'), hash('1'), Vec::new(), 1).unwrap();
    assert_eq!(
        PreparedOperationAccumulator::new(request.clone(), nonzero, unresolved.clone()),
        Err(AccumulatorBatchError::PredecessorRoots)
    );

    let invented = StateAccumulatorBatch::new(
        zero.clone(),
        hash('2'),
        vec![AccumulatorMutation::new(
            AccumulatorPosition::Leaf([1; 32]),
            None,
            Some(vec![2; 32]),
            32,
        )
        .unwrap()],
        1,
    )
    .unwrap();
    assert_eq!(
        PreparedOperationAccumulator::new(request, invented, unresolved),
        Err(AccumulatorBatchError::EmptyRootChange)
    );
}

#[test]
fn preparation_preserves_unresolved_root_and_consumes_into_the_same_binding() {
    let batch = accumulator(&checkpoint_transition());
    let batch = OperationAccumulatorBatch::new(
        batch.binding().clone(),
        batch.state().clone(),
        UnresolvedCustodyAccumulatorBatch::new(hash('c'), hash('c'), Vec::new(), 1).unwrap(),
    )
    .unwrap();
    let prior = predecessor(&batch);
    let request = OperationAccumulatorPreparation::new(
        AccumulatorPreparationBasis::Successor(prior.clone()),
        StateLeafChanges::new(vec![change('1')], 1).unwrap(),
        UnresolvedCustodyLeafChanges::new(Vec::new(), 1).unwrap(),
        limits(1),
        AccumulatorImplementation::SparseMerkleTree061Sha256PaleV1,
        batch.binding().transaction().clone(),
    )
    .unwrap();
    assert_eq!(
        request.basis(),
        &AccumulatorPreparationBasis::Successor(prior.clone())
    );
    assert!(request.unresolved().changes().is_empty());
    assert_eq!(
        prior.unresolved_root(),
        batch.unresolved().predecessor_root()
    );
    let prepared = PreparedOperationAccumulator::new(
        request.clone(),
        batch.state().clone(),
        batch.unresolved().clone(),
    )
    .unwrap();
    assert_eq!(prepared.request(), &request);
    let later_transaction = crate::ControlTransactionPreparation::new(
        store(),
        pale_core::records::ControlTransactionSequenceV1::new(29).unwrap(),
    )
    .unwrap();
    let different_transaction = OperationAccumulatorPreparation::new(
        request.basis().clone(),
        request.state().clone(),
        request.unresolved().clone(),
        request.limits(),
        request.implementation(),
        later_transaction,
    )
    .unwrap();
    assert_ne!(
        prepared.request(),
        &different_transaction,
        "same roots and raw limits do not erase the store fence"
    );
    assert_eq!(
        OperationAccumulatorPreparation::new(
            request.basis().clone(),
            request.state().clone(),
            request.unresolved().clone(),
            request.limits(),
            request.implementation(),
            crate::ControlTransactionPreparation::new(
                store(),
                pale_core::records::ControlTransactionSequenceV1::new(27).unwrap()
            )
            .unwrap(),
        ),
        Err(AccumulatorPreparationInputError::TransactionFence),
        "a current store head cannot precede its retained checkpoint transaction",
    );
    for changed in [
        [2, 512, 512, 512, 16_384],
        [1, 513, 512, 512, 16_384],
        [1, 512, 513, 512, 16_384],
        [1, 512, 512, 513, 16_384],
        [1, 512, 512, 512, 16_385],
    ] {
        let [leaves, reads, writes, positions, proof] = changed;
        let different = OperationAccumulatorPreparation::new(
            request.basis().clone(),
            request.state().clone(),
            request.unresolved().clone(),
            AccumulatorPreparationLimits::new(leaves, reads, writes, positions, proof).unwrap(),
            request.implementation(),
            request.transaction().clone(),
        )
        .unwrap();
        assert_ne!(
            prepared.request(),
            &different,
            "every raw limit remains associated"
        );
    }
    let other_tree = OperationAccumulatorPreparation::new(
        request.basis().clone(),
        StateLeafChanges::new(vec![], 1).unwrap(),
        UnresolvedCustodyLeafChanges::new(request.state().changes().to_vec(), 1).unwrap(),
        request.limits(),
        request.implementation(),
        request.transaction().clone(),
    )
    .unwrap();
    assert_ne!(
        prepared.request(),
        &other_tree,
        "tree classification cannot be swapped"
    );
    assert_eq!(prepared.state_root(), batch.state().successor_root());
    assert_eq!(
        prepared.unresolved_root(),
        batch.unresolved().successor_root()
    );
    assert_eq!(
        prepared
            .bind_successor(batch.binding().successor().clone())
            .unwrap(),
        batch
    );
}

#[test]
fn prepared_output_rejects_unlike_roots_and_non_successor_heads() {
    let batch = accumulator(&checkpoint_transition());
    let request = OperationAccumulatorPreparation::new(
        AccumulatorPreparationBasis::Successor(predecessor(&batch)),
        StateLeafChanges::new(vec![change('1')], 1).unwrap(),
        UnresolvedCustodyLeafChanges::new(Vec::new(), 1).unwrap(),
        limits(1),
        AccumulatorImplementation::SparseMerkleTree061Sha256PaleV1,
        batch.binding().transaction().clone(),
    )
    .unwrap();
    let wrong = StateAccumulatorBatch::new(
        hash('f'),
        batch.state().successor_root().clone(),
        batch.state().mutations().to_vec(),
        512,
    )
    .unwrap();
    assert_eq!(
        PreparedOperationAccumulator::new(request.clone(), wrong, batch.unresolved().clone(),),
        Err(AccumulatorBatchError::PredecessorRoots)
    );
    let wrong =
        UnresolvedCustodyAccumulatorBatch::new(hash('f'), hash('f'), Vec::new(), 512).unwrap();
    assert_eq!(
        PreparedOperationAccumulator::new(request.clone(), batch.state().clone(), wrong,),
        Err(AccumulatorBatchError::PredecessorRoots)
    );
    for head in [
        predecessor(&batch).checkpoint().clone(),
        checkpoint(10, "checkpoint-10", 'a'),
        ControlCheckpointHead::new(
            ControlCheckpointDomainIdV1::parse("other-domain").unwrap(),
            batch.binding().successor().domain_epoch(),
            batch.binding().successor().domain_sequence(),
            batch.binding().successor().committed_transaction_sequence(),
            batch.binding().successor().coordinate().clone(),
        ),
    ] {
        let prepared = PreparedOperationAccumulator::new(
            request.clone(),
            batch.state().clone(),
            batch.unresolved().clone(),
        )
        .unwrap();
        assert_eq!(
            prepared.bind_successor(head),
            Err(AccumulatorBatchError::CheckpointPair)
        );
    }
}

#[test]
fn initial_preparation_requires_absence_and_binds_only_sequence_one() {
    let basis = initial_basis();
    // Epoch selection is an application-profile obligation, not a global port
    // restriction. This existing fixture deliberately uses a non-one epoch.
    assert_eq!(basis.epoch().get(), 3);
    assert_eq!(
        OperationAccumulatorPreparation::new(
            basis.clone(),
            StateLeafChanges::new(vec![change('1')], 1).unwrap(),
            UnresolvedCustodyLeafChanges::new(vec![], 1).unwrap(),
            limits(1),
            AccumulatorImplementation::SparseMerkleTree061Sha256PaleV1,
            crate::ControlTransactionPreparation::new(
                store(),
                pale_core::records::ControlTransactionSequenceV1::new(20).unwrap()
            )
            .unwrap(),
        ),
        Err(AccumulatorPreparationInputError::InitialPredecessor)
    );
    let request = OperationAccumulatorPreparation::new(
        basis.clone(),
        StateLeafChanges::new(
            vec![AccumulatorLeafChange::new(hash('1'), None, Some(hash('2'))).unwrap()],
            1,
        )
        .unwrap(),
        UnresolvedCustodyLeafChanges::new(vec![], 1).unwrap(),
        limits(1),
        AccumulatorImplementation::SparseMerkleTree061Sha256PaleV1,
        crate::ControlTransactionPreparation::new(
            store(),
            pale_core::records::ControlTransactionSequenceV1::new(20).unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let state = StateAccumulatorBatch::new(
        hash('0'),
        hash('2'),
        vec![AccumulatorMutation::new(
            AccumulatorPosition::Leaf([1; 32]),
            None,
            Some(vec![2; 32]),
            32,
        )
        .unwrap()],
        1,
    )
    .unwrap();
    let unresolved =
        UnresolvedCustodyAccumulatorBatch::new(hash('0'), hash('0'), vec![], 1).unwrap();
    let prepared = || {
        PreparedOperationAccumulator::new(request.clone(), state.clone(), unresolved.clone())
            .unwrap()
    };
    let batch = prepared()
        .bind_successor(checkpoint(1, "initial", '1'))
        .unwrap();
    assert_eq!(
        batch.binding().phase(),
        &AccumulatorCheckpointPhase::Initial
    );
    assert_eq!(
        prepared().bind_successor(checkpoint(2, "not-initial", '2')),
        Err(AccumulatorBatchError::CheckpointPair)
    );
    let stale =
        StateAccumulatorBatch::new(hash('1'), hash('2'), state.mutations().to_vec(), 1).unwrap();
    assert_eq!(
        PreparedOperationAccumulator::new(request.clone(), stale, unresolved.clone(),),
        Err(AccumulatorBatchError::PredecessorRoots)
    );
    let old_position = StateAccumulatorBatch::new(
        hash('0'),
        hash('2'),
        vec![AccumulatorMutation::new(
            AccumulatorPosition::Leaf([1; 32]),
            Some(vec![1; 32]),
            Some(vec![2; 32]),
            32,
        )
        .unwrap()],
        1,
    )
    .unwrap();
    assert_eq!(
        PreparedOperationAccumulator::new(request, old_position, unresolved,),
        Err(AccumulatorBatchError::InitialPredecessor)
    );
}
