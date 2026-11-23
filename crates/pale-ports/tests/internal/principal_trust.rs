//! Physical principal-trust frontier law tests.

use super::*;
use crate::ControlRecordDigest;
use pale_core::records::ResidentRecordRefV1;

fn coordinate(reference: &str, byte: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(reference).expect("record ref"),
        ControlRecordDigest::parse(std::iter::repeat_n(byte, 64).collect::<String>())
            .expect("record digest"),
    )
}

fn head(principal: &str, epoch: u64, reference: &str, byte: char) -> PrincipalTrustCheckpointHead {
    PrincipalTrustCheckpointHead::new(
        ResidentPrincipalIdV1::parse(principal).expect("principal"),
        PrincipalTrustCheckpointEpochV1::new(epoch).expect("trust epoch"),
        coordinate(reference, byte),
    )
}

#[test]
fn trust_checkpoint_head_requires_one_epoch_same_principal_and_new_coordinate() {
    let predecessor = head("principal-1", 1, "checkpoint-1", 'a');
    assert!(head("principal-1", 2, "checkpoint-2", 'b').is_exact_successor_of(&predecessor));
    assert!(!head("principal-1", 3, "checkpoint-3", 'c').is_exact_successor_of(&predecessor));
    assert!(!head("principal-2", 2, "checkpoint-2", 'b').is_exact_successor_of(&predecessor));
    assert!(!head("principal-1", 2, "checkpoint-1", 'a').is_exact_successor_of(&predecessor));
}

#[test]
fn trust_snapshot_preserves_exact_binding_head_and_record() {
    let binding = PrincipalTrustStoreBinding::new(
        ControlStoreIdV1::parse("store-1").unwrap(),
        ControlIncarnationIdV1::parse("incarnation-1").unwrap(),
        ResidentPrincipalIdV1::parse("principal-1").unwrap(),
    );
    let checkpoint = head("principal-1", 1, "checkpoint-1", 'a');
    let record = ControlRecord::new(
        checkpoint.coordinate().clone(),
        pale_core::records::SchemaId::parse("pale/test.v1").unwrap(),
        pale_core::records::SchemaDigest::parse("b".repeat(64)).unwrap(),
        crate::ControlRecordBytes::new(b"opaque".to_vec()).unwrap(),
    );
    let snapshot =
        PrincipalTrustSnapshot::new(binding.clone(), checkpoint.clone(), record.clone()).unwrap();
    assert_eq!(snapshot.binding(), &binding);
    assert_eq!(snapshot.through(), &checkpoint);
    assert_eq!(snapshot.checkpoint_record(), &record);
    assert_eq!(
        PrincipalTrustSnapshot::new(
            binding.clone(),
            head("principal-2", 1, "checkpoint-1", 'a'),
            record.clone()
        ),
        Err(PrincipalTrustSnapshotError::PrincipalBinding),
    );
    assert_eq!(
        PrincipalTrustSnapshot::new(binding, head("principal-1", 1, "substitution", 'a'), record),
        Err(PrincipalTrustSnapshotError::CheckpointRecord),
    );
}

#[test]
fn checkpoint_set_snapshot_requires_one_exact_current_checkpoint_row() {
    let binding = PrincipalTrustStoreBinding::new(
        ControlStoreIdV1::parse("store-1").unwrap(),
        ControlIncarnationIdV1::parse("incarnation-1").unwrap(),
        ResidentPrincipalIdV1::parse("principal-1").unwrap(),
    );
    let checkpoint = head("principal-1", 1, "checkpoint-1", 'a');
    let record = ControlRecord::new(
        checkpoint.coordinate().clone(),
        pale_core::records::SchemaId::parse(
            pale_core::records::PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA,
        )
        .unwrap(),
        pale_core::records::SchemaDigest::parse("b".repeat(64)).unwrap(),
        crate::ControlRecordBytes::new(b"opaque".to_vec()).unwrap(),
    );
    let current = PrincipalTrustSnapshot::new(binding, checkpoint.clone(), record.clone()).unwrap();
    let transaction = ControlTransactionSequenceV1::new(3).unwrap();
    let retained = PrincipalTrustCheckpointRecord::new(transaction, record.clone());
    assert!(PrincipalTrustCheckpointSetSnapshot::new(
        current.binding().clone(),
        current.through().clone(),
        transaction,
        vec![retained.clone()].into_boxed_slice()
    )
    .is_ok());
    assert_eq!(
        PrincipalTrustCheckpointSetSnapshot::new(
            current.binding().clone(),
            current.through().clone(),
            transaction,
            Box::new([])
        ),
        Err(PrincipalTrustCheckpointSetSnapshotError::Empty)
    );
    assert_eq!(
        PrincipalTrustCheckpointSetSnapshot::new(
            current.binding().clone(),
            current.through().clone(),
            transaction,
            vec![retained.clone(), retained].into_boxed_slice()
        ),
        Err(PrincipalTrustCheckpointSetSnapshotError::RepeatedCoordinate)
    );
    let other_family = ControlRecord::new(
        checkpoint.coordinate().clone(),
        pale_core::records::SchemaId::parse("pale/other.v1").unwrap(),
        pale_core::records::SchemaDigest::parse("b".repeat(64)).unwrap(),
        crate::ControlRecordBytes::new(b"opaque".to_vec()).unwrap(),
    );
    assert_eq!(
        PrincipalTrustCheckpointSetSnapshot::new(
            current.binding().clone(),
            checkpoint,
            transaction,
            vec![PrincipalTrustCheckpointRecord::new(
                transaction,
                other_family
            )]
            .into_boxed_slice(),
        ),
        Err(PrincipalTrustCheckpointSetSnapshotError::RecordFamily)
    );
    assert_eq!(
        PrincipalTrustCheckpointSetSnapshot::new(
            current.binding().clone(),
            current.through().clone(),
            transaction,
            vec![PrincipalTrustCheckpointRecord::new(
                ControlTransactionSequenceV1::new(4).unwrap(),
                record,
            )]
            .into_boxed_slice(),
        ),
        Err(PrincipalTrustCheckpointSetSnapshotError::FutureRecord)
    );
}
