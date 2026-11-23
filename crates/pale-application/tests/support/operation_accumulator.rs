//! Finite physical tree batches for application custody tests.
//!
//! These synthetic node bytes do not prove Merkle roots or checkpoint authority.
//! Tests supply their existing signed checkpoint records and retain the separate
//! construction verifier. Native accumulator tests own real proof computation.

use pale_core::records::{ArtifactDigest, ControlStateCheckpointV1, ResidentRecordRefV1};
use pale_ports::{
    AccumulatorCheckpointBinding, AccumulatorMutation, AccumulatorPosition,
    AuthorityStoreCoordinate, ControlCheckpointHead, ControlRecordCoordinate, ControlRecordDigest,
    OperationAccumulatorBatch, StateAccumulatorBatch, UnresolvedCustodyAccumulatorBatch,
};

/// Match every physical coordinate to the test's existing admitted pair.
pub(crate) fn for_checkpoints(
    store: AuthorityStoreCoordinate,
    predecessor: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    successor: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
) -> OperationAccumulatorBatch {
    // A synthetic optimistic frontier, not a read from native storage.
    let transaction = pale_ports::ControlTransactionPreparation::new(
        store.clone(),
        pale_core::records::ControlTransactionSequenceV1::new(
            successor.1.committed_transaction_sequence().get() - 1,
        )
        .expect("positive predecessor transaction"),
    )
    .expect("bounded fixture transaction");
    let binding = AccumulatorCheckpointBinding::new(
        store,
        predecessor.1.checkpoint_profile_ref().clone(),
        predecessor.1.checkpoint_profile_sha256().clone(),
        pale_ports::AccumulatorCheckpointPhase::Successor(checkpoint_head(predecessor)),
        checkpoint_head(successor),
        transaction,
    )
    .expect("fixture checkpoint pair");
    physical_batch(
        binding,
        predecessor.1.state_root_sha256().clone(),
        successor.1.state_root_sha256().clone(),
        predecessor.1.unresolved_custody_root_sha256().clone(),
        successor.1.unresolved_custody_root_sha256().clone(),
    )
}

/// Represent explicit roots without claiming that synthetic positions prove them.
/// Kept separate so physical-custody and negative root-binding tests can reuse it.
pub(crate) fn physical_batch(
    binding: AccumulatorCheckpointBinding,
    state_predecessor: ArtifactDigest,
    state_successor: ArtifactDigest,
    unresolved_predecessor: ArtifactDigest,
    unresolved_successor: ArtifactDigest,
) -> OperationAccumulatorBatch {
    let unresolved_changes = if unresolved_predecessor == unresolved_successor {
        Vec::new()
    } else {
        vec![synthetic_mutation(2)]
    };
    OperationAccumulatorBatch::new(
        binding,
        StateAccumulatorBatch::new(
            state_predecessor,
            state_successor,
            vec![synthetic_mutation(1)],
            1,
        )
        .expect("finite state batch"),
        UnresolvedCustodyAccumulatorBatch::new(
            unresolved_predecessor,
            unresolved_successor,
            unresolved_changes,
            1,
        )
        .expect("finite unresolved-custody batch"),
    )
    .expect("physical fixture batch")
}

fn synthetic_mutation(key: u8) -> AccumulatorMutation {
    AccumulatorMutation::new(
        AccumulatorPosition::Leaf([key; 32]),
        Some(vec![3; 32]),
        Some(vec![4; 32]),
        32,
    )
    .expect("one explicit synthetic leaf replacement")
}

fn checkpoint_head(
    (reference, checkpoint): (&ResidentRecordRefV1, &ControlStateCheckpointV1),
) -> ControlCheckpointHead {
    ControlCheckpointHead::new(
        checkpoint.checkpoint_domain_id().clone(),
        *checkpoint.domain_epoch(),
        *checkpoint.domain_sequence(),
        *checkpoint.committed_transaction_sequence(),
        ControlRecordCoordinate::new(
            reference.clone(),
            ControlRecordDigest::parse(checkpoint.canonical_control_checkpoint_sha256().as_str())
                .expect("checkpoint digest"),
        ),
    )
}
