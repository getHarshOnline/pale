//! Principal control-checkpoint physical snapshot law tests.

use super::*;
use crate::{ControlRecordBytes, ControlRecordCoordinate, ControlRecordDigest};
use pale_core::records::{
    ControlCheckpointDomainIdV1, ControlDomainEpochV1, ControlDomainSequenceV1,
    ControlIncarnationIdV1, ControlStoreIdV1, ResidentPrincipalIdV1, ResidentRecordRefV1,
    SchemaDigest, SchemaId,
};

fn coordinate(reference: &str, byte: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(reference).expect("record ref"),
        ControlRecordDigest::parse(std::iter::repeat_n(byte, 64).collect::<String>())
            .expect("record digest"),
    )
}

fn head(sequence: u64, reference: &str, byte: char) -> ControlCheckpointHead {
    ControlCheckpointHead::new(
        ControlCheckpointDomainIdV1::parse("principal-trust").unwrap(),
        ControlDomainEpochV1::new(1).unwrap(),
        ControlDomainSequenceV1::new(sequence).unwrap(),
        ControlTransactionSequenceV1::new(sequence + 1).unwrap(),
        coordinate(reference, byte),
    )
}

fn record(reference: &str, byte: char) -> ControlRecord {
    ControlRecord::new(
        coordinate(reference, byte),
        SchemaId::parse(pale_core::records::CONTROL_STATE_CHECKPOINT_V1_SCHEMA).unwrap(),
        SchemaDigest::parse("8".repeat(64)).unwrap(),
        ControlRecordBytes::new(b"opaque".to_vec()).unwrap(),
    )
}

fn binding() -> PrincipalTrustStoreBinding {
    PrincipalTrustStoreBinding::new(
        ControlStoreIdV1::parse("store-1").unwrap(),
        ControlIncarnationIdV1::parse("incarnation-1").unwrap(),
        ResidentPrincipalIdV1::parse("principal-1").unwrap(),
    )
}

#[test]
fn snapshot_requires_exact_initial_current_and_physical_bounds() {
    let store_binding = binding();
    let initial = head(1, "control-1", 'a');
    let current = head(2, "control-2", 'b');
    let through = ControlTransactionSequenceV1::new(3).unwrap();
    let records = vec![
        PrincipalControlCheckpointStoredRecord::new(
            ControlTransactionSequenceV1::new(2).unwrap(),
            record("control-1", 'a'),
        ),
        PrincipalControlCheckpointStoredRecord::new(through, record("control-2", 'b')),
    ];
    let snapshot = PrincipalControlCheckpointChainSnapshot::new(
        store_binding.clone(),
        initial.clone(),
        current.clone(),
        through,
        records.clone().into_boxed_slice(),
    )
    .unwrap();
    assert_eq!(snapshot.binding(), &store_binding);
    assert_eq!(snapshot.initial(), &initial);
    assert_eq!(snapshot.through(), &current);
    assert_eq!(snapshot.observed_through(), through);
    assert_eq!(snapshot.records().len(), 2);

    assert_eq!(
        PrincipalControlCheckpointChainSnapshot::new(
            store_binding.clone(),
            initial.clone(),
            current.clone(),
            through,
            Box::new([]),
        ),
        Err(PrincipalControlCheckpointChainSnapshotError::Empty)
    );
    assert_eq!(
        PrincipalControlCheckpointChainSnapshot::new(
            store_binding.clone(),
            initial.clone(),
            current.clone(),
            through,
            vec![records[1].clone()].into_boxed_slice(),
        ),
        Err(PrincipalControlCheckpointChainSnapshotError::InitialRecord)
    );
    assert_eq!(
        PrincipalControlCheckpointChainSnapshot::new(
            store_binding.clone(),
            initial.clone(),
            current.clone(),
            through,
            vec![records[0].clone()].into_boxed_slice(),
        ),
        Err(PrincipalControlCheckpointChainSnapshotError::CurrentRecord)
    );
    assert_eq!(
        PrincipalControlCheckpointChainSnapshot::new(
            store_binding.clone(),
            initial.clone(),
            current,
            through,
            vec![records[0].clone(), records[0].clone()].into_boxed_slice(),
        ),
        Err(PrincipalControlCheckpointChainSnapshotError::RepeatedCoordinate)
    );
    assert_eq!(
        PrincipalControlCheckpointChainSnapshot::new(
            store_binding,
            initial.clone(),
            head(1, "control-1", 'a'),
            through,
            vec![PrincipalControlCheckpointStoredRecord::new(
                ControlTransactionSequenceV1::new(4).unwrap(),
                record("control-1", 'a'),
            )]
            .into_boxed_slice(),
        ),
        Err(PrincipalControlCheckpointChainSnapshotError::FutureRecord)
    );

    let future_head = head(3, "control-3", 'c');
    assert_eq!(
        PrincipalControlCheckpointChainSnapshot::new(
            binding(),
            initial,
            future_head.clone(),
            through,
            vec![
                records[0].clone(),
                PrincipalControlCheckpointStoredRecord::new(through, record("control-3", 'c'),),
            ]
            .into_boxed_slice(),
        ),
        Err(PrincipalControlCheckpointChainSnapshotError::HeadRelation)
    );
}
