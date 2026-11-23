//! Closed physical record shapes for observation and result co-commit.

use super::*;
use pale_core::records::{
    ADAPTER_FAILURE_V1_SCHEMA, CAPABILITY_RESULT_V1_SCHEMA, DIAGNOSTIC_V1_SCHEMA,
};

fn diagnostic(reference: &str) -> ControlRecord {
    // Reuse the generated diagnostic and its actual schema descriptor. This
    // physical fixture does not claim semantic failure-state admission.
    let fixture = pale_core::canon::parse_strict(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/conformance/cross-leaf.v1/contract-diagnostic.json"
    )))
    .unwrap();
    let schema = &fixture["schema"];
    assert_eq!(
        schema["wire_schema_id"].as_str().unwrap(),
        DIAGNOSTIC_V1_SCHEMA
    );
    assert_eq!(
        schema["schema_sha256"].as_str().unwrap(),
        pale_core::sha256_hex(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../contracts/bundle/agent-surface.v1/schemas/pale/agent-surface/diagnostic.v1.json"
        )))
    );
    let bytes = fixture["canonical_diagnostic"]
        .as_str()
        .unwrap()
        .as_bytes()
        .to_vec();
    ControlRecord::new(
        ControlRecordCoordinate::new(
            ResidentRecordRefV1::parse(reference).unwrap(),
            ControlRecordDigest::parse(pale_core::sha256_hex(&bytes)).unwrap(),
        ),
        SchemaId::parse(DIAGNOSTIC_V1_SCHEMA).unwrap(),
        SchemaDigest::parse(schema["schema_sha256"].as_str().unwrap()).unwrap(),
        ControlRecordBytes::new(bytes).unwrap(),
    )
}

fn observe(
    records: Vec<OperationCustodyRecord>,
) -> Result<OperationCustodyCandidate, OperationCustodyCandidateError> {
    OperationCustodyCandidate::new(
        store(),
        OperationCustodyTransitionKind::Observe,
        records,
        vec![OperationCustodyIndexMutation::LatestObservation {
            key: key('b'),
            expected: OperationCustodyIndexPredecessor::Exact(record(
                "observation-1",
                '3',
                OPERATION_OBSERVATION_V1_SCHEMA,
            )),
            successor: record("observation-2", '4', OPERATION_OBSERVATION_V1_SCHEMA)
                .coordinate()
                .clone(),
        }],
        checkpoint_transition(),
        accumulator(&checkpoint_transition()),
    )
}

#[test]
fn observation_accepts_one_optional_result_without_an_additional_index() {
    let observation = OperationCustodyRecord::OperationObservation(record(
        "observation-2",
        '4',
        OPERATION_OBSERVATION_V1_SCHEMA,
    ));
    let result = record("result-1", '5', CAPABILITY_RESULT_V1_SCHEMA);
    for records in [
        vec![observation.clone()],
        vec![
            observation.clone(),
            OperationCustodyRecord::CapabilityResult(result.clone()),
        ],
        vec![
            OperationCustodyRecord::CapabilityResult(result.clone()),
            observation,
        ],
    ] {
        let has_result = records.len() == 2;
        let candidate = observe(records).unwrap();
        assert_eq!(candidate.indexes().len(), 2);
        assert_eq!(
            candidate.record(OperationCustodyRecordKind::CapabilityResult),
            has_result.then_some(&result)
        );
        assert_eq!(
            candidate.accumulator(),
            &accumulator(&checkpoint_transition())
        );
    }
}

#[test]
fn safe_provider_failure_is_one_closed_three_record_atom() {
    let observation = OperationCustodyRecord::OperationObservation(record(
        "observation-2",
        '4',
        OPERATION_OBSERVATION_V1_SCHEMA,
    ));
    let failure = OperationCustodyRecord::AdapterFailure(record(
        "adapter-failure-1",
        '5',
        ADAPTER_FAILURE_V1_SCHEMA,
    ));
    let result = OperationCustodyRecord::CapabilityResult(record(
        "result-1",
        '6',
        CAPABILITY_RESULT_V1_SCHEMA,
    ));
    let diagnostic = OperationCustodyRecord::Diagnostic(diagnostic("diagnostic-1"));

    let candidate = observe(vec![
        observation.clone(),
        failure.clone(),
        result.clone(),
        diagnostic.clone(),
    ])
    .expect("the complete safe-failure atom is accepted");
    assert_eq!(
        candidate.record(OperationCustodyRecordKind::AdapterFailure),
        Some(failure.record())
    );
    assert_eq!(
        candidate.record(OperationCustodyRecordKind::CapabilityResult),
        Some(result.record())
    );
    assert_eq!(
        candidate.record(OperationCustodyRecordKind::Diagnostic),
        Some(diagnostic.record())
    );

    for incomplete in [
        vec![observation.clone(), failure.clone(), result.clone()],
        vec![observation.clone(), failure.clone(), diagnostic.clone()],
        vec![observation.clone(), result.clone(), diagnostic.clone()],
        vec![
            observation.clone(),
            failure.clone(),
            result.clone(),
            diagnostic.clone(),
            failure.clone(),
        ],
    ] {
        assert_eq!(
            observe(incomplete).expect_err("partial or duplicate atoms are rejected"),
            OperationCustodyCandidateError::RecordShape
        );
    }
}

#[test]
fn result_cannot_replace_observation_duplicate_a_role_or_enter_a_start() {
    let observation = OperationCustodyRecord::OperationObservation(record(
        "observation-2",
        '4',
        OPERATION_OBSERVATION_V1_SCHEMA,
    ));
    let result = OperationCustodyRecord::CapabilityResult(record(
        "result-1",
        '5',
        CAPABILITY_RESULT_V1_SCHEMA,
    ));
    for records in [
        vec![result.clone()],
        vec![observation.clone(), result.clone(), result.clone()],
        vec![observation, result.clone(), start_records()[0].clone()],
    ] {
        assert_eq!(
            observe(records).unwrap_err(),
            OperationCustodyCandidateError::RecordShape
        );
    }
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
        let mut records = start_records();
        records.push(result.clone());
        assert_eq!(
            OperationCustodyCandidate::new_start(
                store(),
                kind,
                records,
                indexes,
                checkpoint_transition(),
                accumulator(&checkpoint_transition()),
                start_session_basis(),
            )
            .unwrap_err(),
            OperationCustodyCandidateError::RecordShape
        );
    }
}

#[test]
fn result_schema_and_immutable_reference_are_checked_without_parsing_semantics() {
    for (result, expected) in [
        (
            record("result-1", '5', OPERATION_OBSERVATION_V1_SCHEMA),
            OperationCustodyCandidateError::RecordSchema,
        ),
        (
            record("observation-2", '5', CAPABILITY_RESULT_V1_SCHEMA),
            OperationCustodyCandidateError::DuplicateRecordReference,
        ),
        (
            record("observation-1", '5', CAPABILITY_RESULT_V1_SCHEMA),
            OperationCustodyCandidateError::DuplicateRecordReference,
        ),
        (
            record("checkpoint-9", '5', CAPABILITY_RESULT_V1_SCHEMA),
            OperationCustodyCandidateError::DuplicateRecordReference,
        ),
        (
            record("checkpoint-8", '5', CAPABILITY_RESULT_V1_SCHEMA),
            OperationCustodyCandidateError::DuplicateRecordReference,
        ),
    ] {
        assert_eq!(
            observe(vec![
                OperationCustodyRecord::OperationObservation(record(
                    "observation-2",
                    '4',
                    OPERATION_OBSERVATION_V1_SCHEMA
                )),
                OperationCustodyRecord::CapabilityResult(result)
            ])
            .unwrap_err(),
            expected
        );
    }
}

#[test]
fn private_attempt_selection_is_an_alternative_to_a_result_not_an_extra_role() {
    let observation = OperationCustodyRecord::OperationObservation(record(
        "observation-2",
        '4',
        OPERATION_OBSERVATION_V1_SCHEMA,
    ));
    let selection = OperationCustodyRecord::AttemptSelection(record(
        "attempt-selection",
        '5',
        PRIVATE_OPERATION_ATTEMPT_SELECTION_V1_SCHEMA,
    ));
    for records in [
        vec![observation.clone(), selection.clone()],
        vec![selection.clone(), observation.clone()],
    ] {
        let candidate = observe(records).unwrap();
        assert_eq!(candidate.indexes().len(), 2);
        assert_eq!(
            candidate.record(OperationCustodyRecordKind::AttemptSelection),
            Some(selection.record())
        );
        assert!(candidate
            .record(OperationCustodyRecordKind::CapabilityResult)
            .is_none());
    }
    for records in [
        vec![selection.clone()],
        vec![observation.clone(), selection.clone(), selection.clone()],
        vec![
            observation.clone(),
            selection.clone(),
            OperationCustodyRecord::CapabilityResult(record(
                "result",
                '6',
                CAPABILITY_RESULT_V1_SCHEMA,
            )),
        ],
    ] {
        assert_eq!(
            observe(records).unwrap_err(),
            OperationCustodyCandidateError::RecordShape
        );
    }
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
        let mut records = start_records();
        records.push(selection.clone());
        assert_eq!(
            OperationCustodyCandidate::new_start(
                store(),
                kind,
                records,
                indexes,
                checkpoint_transition(),
                accumulator(&checkpoint_transition()),
                start_session_basis(),
            )
            .unwrap_err(),
            OperationCustodyCandidateError::RecordShape
        );
    }
    for (reference, schema, expected) in [
        (
            "attempt-selection",
            CAPABILITY_RESULT_V1_SCHEMA,
            OperationCustodyCandidateError::RecordSchema,
        ),
        (
            "observation-2",
            PRIVATE_OPERATION_ATTEMPT_SELECTION_V1_SCHEMA,
            OperationCustodyCandidateError::DuplicateRecordReference,
        ),
        (
            "observation-1",
            PRIVATE_OPERATION_ATTEMPT_SELECTION_V1_SCHEMA,
            OperationCustodyCandidateError::DuplicateRecordReference,
        ),
        (
            "checkpoint-9",
            PRIVATE_OPERATION_ATTEMPT_SELECTION_V1_SCHEMA,
            OperationCustodyCandidateError::DuplicateRecordReference,
        ),
    ] {
        assert_eq!(
            observe(vec![
                observation.clone(),
                OperationCustodyRecord::AttemptSelection(record(reference, '5', schema))
            ])
            .unwrap_err(),
            expected
        );
    }
}

#[test]
fn diagnostic_is_a_single_optional_observation_member_not_a_start_or_combined_result() {
    let observation = OperationCustodyRecord::OperationObservation(record(
        "observation-2",
        '4',
        OPERATION_OBSERVATION_V1_SCHEMA,
    ));
    let diagnostic = OperationCustodyRecord::Diagnostic(diagnostic("diagnostic"));
    for records in [
        vec![observation.clone(), diagnostic.clone()],
        vec![diagnostic.clone(), observation.clone()],
    ] {
        let candidate = observe(records).unwrap();
        assert_eq!(
            candidate.record(OperationCustodyRecordKind::Diagnostic),
            Some(diagnostic.record())
        );
        assert_eq!(candidate.indexes().len(), 2);
        assert_eq!(
            candidate.accumulator(),
            &accumulator(&checkpoint_transition())
        );
    }
    for records in [
        vec![diagnostic.clone()],
        vec![observation.clone(), diagnostic.clone(), diagnostic.clone()],
        vec![
            observation.clone(),
            diagnostic.clone(),
            OperationCustodyRecord::CapabilityResult(record(
                "result",
                '5',
                CAPABILITY_RESULT_V1_SCHEMA,
            )),
        ],
        vec![
            observation.clone(),
            diagnostic.clone(),
            OperationCustodyRecord::AttemptSelection(record(
                "selection",
                '6',
                PRIVATE_OPERATION_ATTEMPT_SELECTION_V1_SCHEMA,
            )),
        ],
    ] {
        assert_eq!(
            observe(records).unwrap_err(),
            OperationCustodyCandidateError::RecordShape
        );
    }
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
        let mut records = start_records();
        records.push(diagnostic.clone());
        assert_eq!(
            OperationCustodyCandidate::new_start(
                store(),
                kind,
                records,
                indexes,
                checkpoint_transition(),
                accumulator(&checkpoint_transition()),
                start_session_basis(),
            )
            .unwrap_err(),
            OperationCustodyCandidateError::RecordShape
        );
    }
}

#[test]
fn diagnostic_role_rejects_other_record_schemas_and_reserved_reference_aliases() {
    let observation = OperationCustodyRecord::OperationObservation(record(
        "observation-2",
        '4',
        OPERATION_OBSERVATION_V1_SCHEMA,
    ));
    for record in [
        OperationCustodyRecord::CapabilityResult(diagnostic("diagnostic")),
        OperationCustodyRecord::AttemptSelection(diagnostic("diagnostic")),
        OperationCustodyRecord::Diagnostic(record("result", '5', CAPABILITY_RESULT_V1_SCHEMA)),
        OperationCustodyRecord::Diagnostic(record(
            "selection",
            '6',
            PRIVATE_OPERATION_ATTEMPT_SELECTION_V1_SCHEMA,
        )),
        OperationCustodyRecord::Diagnostic(record(
            "other-observation",
            '7',
            OPERATION_OBSERVATION_V1_SCHEMA,
        )),
    ] {
        assert_eq!(
            observe(vec![observation.clone(), record]).unwrap_err(),
            OperationCustodyCandidateError::RecordSchema
        );
    }
    for reference in [
        "observation-1",
        "observation-2",
        "checkpoint-8",
        "checkpoint-9",
    ] {
        assert_eq!(
            observe(vec![
                observation.clone(),
                OperationCustodyRecord::Diagnostic(diagnostic(reference))
            ])
            .unwrap_err(),
            OperationCustodyCandidateError::DuplicateRecordReference
        );
    }
}
