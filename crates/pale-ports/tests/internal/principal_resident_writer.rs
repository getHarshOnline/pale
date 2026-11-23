//! Physical resident-writer closure tests.

use super::*;
use crate::{
    ControlRecordBytes, ControlRecordDigest, PrincipalResidentWriterRecordSetSnapshot,
    PrincipalResidentWriterRecordSetSnapshotError, PrincipalResidentWriterStoredRecord,
};

use pale_core::records::{
    ControlCheckpointDomainIdV1, ControlDomainEpochV1, ControlDomainSequenceV1,
    ControlIncarnationIdV1, ControlStoreIdV1, ControlTransactionSequenceV1,
    PrincipalResidentWriterFenceIdV1, PrincipalResidentWriterFenceStateV1,
    PrincipalResidentWriterGenerationV1, ResidentPrincipalIdV1, ResidentRecordRefV1, SchemaDigest,
    SchemaId, CONTROL_STATE_CHECKPOINT_V1_SCHEMA, PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA,
    PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SCHEMA,
};

fn digest(byte: char) -> ControlRecordDigest {
    ControlRecordDigest::parse(std::iter::repeat_n(byte, 64).collect::<String>()).unwrap()
}

fn coordinate(reference: &str, byte: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(ResidentRecordRefV1::parse(reference).unwrap(), digest(byte))
}

fn record(coordinate: ControlRecordCoordinate) -> ControlRecord {
    record_with_schema(coordinate, PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA)
}

fn record_with_schema(coordinate: ControlRecordCoordinate, schema: &str) -> ControlRecord {
    ControlRecord::new(
        coordinate,
        SchemaId::parse(schema).unwrap(),
        SchemaDigest::parse("a".repeat(64)).unwrap(),
        ControlRecordBytes::new(br#"{"record":"writer"}"#.to_vec()).unwrap(),
    )
}

fn stored(record: ControlRecord, inserted_in: u64) -> PrincipalResidentWriterStoredRecord {
    PrincipalResidentWriterStoredRecord::new(
        ControlTransactionSequenceV1::new(inserted_in).unwrap(),
        record,
    )
}

fn checkpoint(reference: &str, byte: char, sequence: u64) -> ControlCheckpointHead {
    checkpoint_with_transaction(reference, byte, sequence, sequence + 20)
}

fn checkpoint_with_transaction(
    reference: &str,
    byte: char,
    sequence: u64,
    transaction: u64,
) -> ControlCheckpointHead {
    ControlCheckpointHead::new(
        ControlCheckpointDomainIdV1::parse("principal-resident-writer").unwrap(),
        ControlDomainEpochV1::new(1).unwrap(),
        ControlDomainSequenceV1::new(sequence).unwrap(),
        ControlTransactionSequenceV1::new(transaction).unwrap(),
        coordinate(reference, byte),
    )
}

fn binding() -> PrincipalTrustStoreBinding {
    PrincipalTrustStoreBinding::new(
        ControlStoreIdV1::parse("trust-store-1").unwrap(),
        ControlIncarnationIdV1::parse("trust-incarnation-1").unwrap(),
        ResidentPrincipalIdV1::parse("principal-1").unwrap(),
    )
}

fn head(
    reference: &str,
    byte: char,
    generation: u64,
    state: PrincipalResidentWriterFenceStateV1,
    checkpoint: ControlCheckpointHead,
) -> PrincipalResidentWriterHead {
    PrincipalResidentWriterHead::new(
        PrincipalResidentWriterFenceIdV1::parse("writer-1").unwrap(),
        ResidentPrincipalIdV1::parse("principal-1").unwrap(),
        PrincipalResidentWriterGenerationV1::new(generation).unwrap(),
        state,
        coordinate(reference, byte),
        checkpoint,
    )
}

#[test]
fn clean_writer_candidates_form_the_only_three_physical_transitions() {
    let checkpoint_1 = checkpoint("control-checkpoint-1", 'a', 1);
    let checkpoint_2 = checkpoint("control-checkpoint-2", 'b', 2);
    let initial = head(
        "writer-active-1",
        '1',
        1,
        PrincipalResidentWriterFenceStateV1::Active,
        checkpoint_2.clone(),
    );
    let initial_candidate = PreparedPrincipalResidentWriterTransition::new(
        binding(),
        PrincipalResidentWriterTransitionKind::InitialActive,
        (
            checkpoint_1.clone(),
            record(checkpoint_1.coordinate().clone()),
        ),
        None,
        vec![
            record(initial.coordinate().clone()),
            record(checkpoint_2.coordinate().clone()),
        ],
        initial.clone(),
    )
    .unwrap();
    assert_eq!(
        initial_candidate.kind(),
        PrincipalResidentWriterTransitionKind::InitialActive
    );

    let checkpoint_3 = checkpoint("control-checkpoint-3", 'c', 3);
    let retired = head(
        "writer-retired-1",
        '2',
        1,
        PrincipalResidentWriterFenceStateV1::Retired,
        checkpoint_3.clone(),
    );
    PreparedPrincipalResidentWriterTransition::new(
        binding(),
        PrincipalResidentWriterTransitionKind::CleanRetirement,
        (
            checkpoint_2.clone(),
            record(checkpoint_2.coordinate().clone()),
        ),
        Some((initial.clone(), record(initial.coordinate().clone()))),
        vec![
            record(coordinate("writer-terminal-1", '3')),
            record(retired.coordinate().clone()),
            record(checkpoint_3.coordinate().clone()),
        ],
        retired.clone(),
    )
    .unwrap();

    let checkpoint_4 = checkpoint("control-checkpoint-4", 'd', 4);
    let successor = head(
        "writer-active-2",
        '4',
        2,
        PrincipalResidentWriterFenceStateV1::Active,
        checkpoint_4.clone(),
    );
    PreparedPrincipalResidentWriterTransition::new(
        binding(),
        PrincipalResidentWriterTransitionKind::CleanSuccessor,
        (
            checkpoint_3.clone(),
            record(checkpoint_3.coordinate().clone()),
        ),
        Some((retired.clone(), record(retired.coordinate().clone()))),
        vec![
            record(successor.coordinate().clone()),
            record(checkpoint_4.coordinate().clone()),
        ],
        successor,
    )
    .unwrap();
}

#[test]
fn skipped_generation_duplicate_record_and_wrong_principal_fail_closed() {
    let checkpoint_1 = checkpoint("control-checkpoint-1", 'a', 1);
    let checkpoint_2 = checkpoint("control-checkpoint-2", 'b', 2);
    let retired = head(
        "writer-retired-1",
        '1',
        1,
        PrincipalResidentWriterFenceStateV1::Retired,
        checkpoint_1.clone(),
    );
    let skipped = head(
        "writer-active-3",
        '2',
        3,
        PrincipalResidentWriterFenceStateV1::Active,
        checkpoint_2.clone(),
    );
    assert_eq!(
        PreparedPrincipalResidentWriterTransition::new(
            binding(),
            PrincipalResidentWriterTransitionKind::CleanSuccessor,
            (
                checkpoint_1.clone(),
                record(checkpoint_1.coordinate().clone()),
            ),
            Some((retired.clone(), record(retired.coordinate().clone()),)),
            vec![
                record(skipped.coordinate().clone()),
                record(checkpoint_2.coordinate().clone()),
            ],
            skipped,
        ),
        Err(PrincipalResidentWriterCandidateError::Transition)
    );

    let initial = head(
        "writer-active-1",
        '3',
        1,
        PrincipalResidentWriterFenceStateV1::Active,
        checkpoint_2.clone(),
    );
    assert_eq!(
        PreparedPrincipalResidentWriterTransition::new(
            binding(),
            PrincipalResidentWriterTransitionKind::InitialActive,
            (
                checkpoint_1.clone(),
                record(checkpoint_1.coordinate().clone()),
            ),
            None,
            vec![
                record(initial.coordinate().clone()),
                record(initial.coordinate().clone()),
                record(checkpoint_2.coordinate().clone()),
            ],
            initial,
        ),
        Err(PrincipalResidentWriterCandidateError::RecordClosure)
    );

    let other = PrincipalResidentWriterHead::new(
        PrincipalResidentWriterFenceIdV1::parse("writer-1").unwrap(),
        ResidentPrincipalIdV1::parse("principal-2").unwrap(),
        PrincipalResidentWriterGenerationV1::new(1).unwrap(),
        PrincipalResidentWriterFenceStateV1::Active,
        coordinate("writer-other", '4'),
        checkpoint_2.clone(),
    );
    assert_eq!(
        PreparedPrincipalResidentWriterTransition::new(
            binding(),
            PrincipalResidentWriterTransitionKind::InitialActive,
            (
                checkpoint_1.clone(),
                record(checkpoint_1.coordinate().clone()),
            ),
            None,
            vec![
                record(other.coordinate().clone()),
                record(checkpoint_2.coordinate().clone()),
            ],
            other,
        ),
        Err(PrincipalResidentWriterCandidateError::PrincipalBinding)
    );
}

#[test]
fn predecessor_head_requires_its_exact_comparison_record() {
    let checkpoint_1 = checkpoint("control-checkpoint-1", 'a', 1);
    let checkpoint_2 = checkpoint("control-checkpoint-2", 'b', 2);
    let retired = head(
        "writer-retired-1",
        '1',
        1,
        PrincipalResidentWriterFenceStateV1::Retired,
        checkpoint_1.clone(),
    );
    let successor = head(
        "writer-active-2",
        '2',
        2,
        PrincipalResidentWriterFenceStateV1::Active,
        checkpoint_2.clone(),
    );
    assert_eq!(
        PreparedPrincipalResidentWriterTransition::new(
            binding(),
            PrincipalResidentWriterTransitionKind::CleanSuccessor,
            (
                checkpoint_1.clone(),
                record(checkpoint_1.coordinate().clone()),
            ),
            Some((retired, record(coordinate("writer-other", '4')))),
            vec![
                record(successor.coordinate().clone()),
                record(checkpoint_2.coordinate().clone()),
            ],
            successor,
        ),
        Err(PrincipalResidentWriterCandidateError::RecordClosure)
    );
}

#[test]
fn physical_record_set_keeps_surplus_rows_without_admitting_them() {
    let current_checkpoint = checkpoint("control-checkpoint-1", 'a', 1);
    let current_head = head(
        "writer-active-1",
        '1',
        1,
        PrincipalResidentWriterFenceStateV1::Active,
        current_checkpoint.clone(),
    );
    let takeover = coordinate("writer-takeover-surplus", '2');
    let snapshot = PrincipalResidentWriterRecordSetSnapshot::new(
        binding(),
        current_head.clone(),
        ControlTransactionSequenceV1::new(30).unwrap(),
        vec![
            stored(record(current_head.coordinate().clone()), 21),
            stored(
                record_with_schema(
                    takeover.clone(),
                    PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SCHEMA,
                ),
                21,
            ),
        ]
        .into_boxed_slice(),
        vec![stored(
            record_with_schema(
                current_checkpoint.coordinate().clone(),
                CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
            ),
            21,
        )]
        .into_boxed_slice(),
    )
    .unwrap();

    assert_eq!(snapshot.head(), &current_head);
    assert_eq!(snapshot.writer_records().len(), 2);
    assert!(snapshot
        .writer_records()
        .iter()
        .any(|retained| retained.record().coordinate() == &takeover));
}

#[test]
fn physical_record_set_rejects_malformed_local_closure() {
    let current_checkpoint = checkpoint("control-checkpoint-1", 'a', 1);
    let current_head = head(
        "writer-active-1",
        '1',
        1,
        PrincipalResidentWriterFenceStateV1::Active,
        current_checkpoint.clone(),
    );
    let checkpoint_record = stored(
        record_with_schema(
            current_checkpoint.coordinate().clone(),
            CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
        ),
        21,
    );
    let writer_record = stored(record(current_head.coordinate().clone()), 21);

    assert_eq!(
        PrincipalResidentWriterRecordSetSnapshot::new(
            binding(),
            current_head.clone(),
            ControlTransactionSequenceV1::new(30).unwrap(),
            vec![writer_record.clone(), writer_record.clone()].into_boxed_slice(),
            vec![checkpoint_record.clone()].into_boxed_slice(),
        ),
        Err(PrincipalResidentWriterRecordSetSnapshotError::RepeatedCoordinate)
    );
    assert_eq!(
        PrincipalResidentWriterRecordSetSnapshot::new(
            binding(),
            current_head.clone(),
            ControlTransactionSequenceV1::new(30).unwrap(),
            vec![
                stored(record(coordinate("writer-z", '9')), 21),
                writer_record.clone(),
            ]
            .into_boxed_slice(),
            vec![checkpoint_record.clone()].into_boxed_slice(),
        ),
        Err(PrincipalResidentWriterRecordSetSnapshotError::RecordOrder)
    );
    assert_eq!(
        PrincipalResidentWriterRecordSetSnapshot::new(
            binding(),
            current_head.clone(),
            ControlTransactionSequenceV1::new(20).unwrap(),
            vec![writer_record.clone()].into_boxed_slice(),
            vec![checkpoint_record.clone()].into_boxed_slice(),
        ),
        Err(PrincipalResidentWriterRecordSetSnapshotError::FutureRecord)
    );
    assert_eq!(
        PrincipalResidentWriterRecordSetSnapshot::new(
            binding(),
            current_head.clone(),
            ControlTransactionSequenceV1::new(30).unwrap(),
            vec![writer_record.clone()].into_boxed_slice(),
            vec![stored(checkpoint_record.record().clone(), 22)].into_boxed_slice(),
        ),
        Err(PrincipalResidentWriterRecordSetSnapshotError::CheckpointTransaction)
    );

    let mismatched_checkpoint = checkpoint_with_transaction("control-checkpoint-1", 'a', 1, 22);
    let mismatched_head = head(
        "writer-active-1",
        '1',
        1,
        PrincipalResidentWriterFenceStateV1::Active,
        mismatched_checkpoint,
    );
    assert_eq!(
        PrincipalResidentWriterRecordSetSnapshot::new(
            binding(),
            mismatched_head,
            ControlTransactionSequenceV1::new(30).unwrap(),
            vec![writer_record.clone()].into_boxed_slice(),
            vec![checkpoint_record.clone()].into_boxed_slice(),
        ),
        Err(PrincipalResidentWriterRecordSetSnapshotError::CurrentRecord)
    );

    let future_checkpoint = checkpoint_with_transaction("control-checkpoint-1", 'a', 1, 31);
    let future_head = head(
        "writer-active-1",
        '1',
        1,
        PrincipalResidentWriterFenceStateV1::Active,
        future_checkpoint,
    );
    assert_eq!(
        PrincipalResidentWriterRecordSetSnapshot::new(
            binding(),
            future_head,
            ControlTransactionSequenceV1::new(30).unwrap(),
            vec![writer_record.clone()].into_boxed_slice(),
            vec![checkpoint_record.clone()].into_boxed_slice(),
        ),
        Err(PrincipalResidentWriterRecordSetSnapshotError::FutureRecord)
    );
    assert_eq!(
        PrincipalResidentWriterRecordSetSnapshot::new(
            binding(),
            current_head.clone(),
            ControlTransactionSequenceV1::new(30).unwrap(),
            vec![stored(
                record_with_schema(
                    current_head.coordinate().clone(),
                    CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
                ),
                21,
            )]
            .into_boxed_slice(),
            vec![checkpoint_record.clone()].into_boxed_slice(),
        ),
        Err(PrincipalResidentWriterRecordSetSnapshotError::RecordFamily)
    );

    let other_principal_head = PrincipalResidentWriterHead::new(
        PrincipalResidentWriterFenceIdV1::parse("writer-1").unwrap(),
        ResidentPrincipalIdV1::parse("principal-2").unwrap(),
        PrincipalResidentWriterGenerationV1::new(1).unwrap(),
        PrincipalResidentWriterFenceStateV1::Active,
        current_head.coordinate().clone(),
        current_checkpoint.clone(),
    );
    assert_eq!(
        PrincipalResidentWriterRecordSetSnapshot::new(
            binding(),
            other_principal_head,
            ControlTransactionSequenceV1::new(30).unwrap(),
            vec![writer_record].into_boxed_slice(),
            vec![checkpoint_record.clone()].into_boxed_slice(),
        ),
        Err(PrincipalResidentWriterRecordSetSnapshotError::PrincipalBinding)
    );

    let takeover_head = head(
        "writer-takeover-current",
        '3',
        1,
        PrincipalResidentWriterFenceStateV1::Active,
        current_checkpoint,
    );
    assert_eq!(
        PrincipalResidentWriterRecordSetSnapshot::new(
            binding(),
            takeover_head.clone(),
            ControlTransactionSequenceV1::new(30).unwrap(),
            vec![stored(
                record_with_schema(
                    takeover_head.coordinate().clone(),
                    PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SCHEMA,
                ),
                21,
            )]
            .into_boxed_slice(),
            vec![checkpoint_record].into_boxed_slice(),
        ),
        Err(PrincipalResidentWriterRecordSetSnapshotError::CurrentRecord)
    );
}
