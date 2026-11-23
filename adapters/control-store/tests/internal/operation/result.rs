//! Physical co-commit of response records and results with observations.
//!
//! Application tests own signed result and terminal-state admission. These
//! fixtures exercise the native transaction, immutable bytes, and replay only.

use super::*;
use pale_core::records::{
    ADAPTER_FAILURE_V1_SCHEMA, CAPABILITY_RESULT_V1_SCHEMA, DIAGNOSTIC_V1_SCHEMA,
};
use pale_ports::{
    OperationCustodyRecordKind, PRIVATE_OPERATION_ATTEMPT_SELECTION_V1_SCHEMA,
    PRIVATE_OPERATION_HOOK_RESPONSE_V1_SCHEMA,
};

const MEMBER_SCHEMAS: [&str; 4] = [
    CAPABILITY_RESULT_V1_SCHEMA,
    PRIVATE_OPERATION_ATTEMPT_SELECTION_V1_SCHEMA,
    DIAGNOSTIC_V1_SCHEMA,
    PRIVATE_OPERATION_HOOK_RESPONSE_V1_SCHEMA,
];

fn member_record(reference: &str, byte: char, schema: &str) -> ControlRecord {
    if schema != DIAGNOSTIC_V1_SCHEMA {
        return record(reference, byte, schema);
    }
    // The generated conformance record owns these diagnostic bytes; this
    // transaction test does not infer the failure's semantic association.
    let fixture = pale_core::canon::parse_strict(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/conformance/cross-leaf.v1/contract-diagnostic.json"
    )))
    .unwrap();
    let descriptor = pale_contracts::record_schema_identity("diagnostic.v1").unwrap();
    assert_eq!(
        fixture["schema"]["schema_sha256"].as_str().unwrap(),
        descriptor.schema_sha256.as_str()
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
        descriptor.schema,
        descriptor.schema_sha256,
        ControlRecordBytes::new(bytes).unwrap(),
    )
}

fn with_member(
    start: &OperationCustodyCandidate,
    result: ControlRecord,
) -> OperationCustodyCandidate {
    let observation = observation_candidate(
        start
            .record(OperationCustodyRecordKind::OperationObservation)
            .unwrap()
            .clone(),
        "one",
        '4',
    );
    let mut records = observation.records().to_vec();
    records.push(match result.schema_id().as_str() {
        CAPABILITY_RESULT_V1_SCHEMA => OperationCustodyRecord::CapabilityResult(result),
        PRIVATE_OPERATION_ATTEMPT_SELECTION_V1_SCHEMA => {
            OperationCustodyRecord::AttemptSelection(result)
        }
        DIAGNOSTIC_V1_SCHEMA => OperationCustodyRecord::Diagnostic(result),
        PRIVATE_OPERATION_HOOK_RESPONSE_V1_SCHEMA => OperationCustodyRecord::HookResponse(result),
        _ => panic!("fixture needs a selected optional member"),
    });
    OperationCustodyCandidate::new(
        store(),
        OperationCustodyTransitionKind::Observe,
        records,
        observation
            .indexes()
            .iter()
            .filter(|index| {
                matches!(
                    index,
                    OperationCustodyIndexMutation::LatestObservation { .. }
                )
            })
            .cloned()
            .collect(),
        observation.checkpoint().clone(),
        observation.accumulator().clone(),
    )
    .unwrap()
}

fn with_safe_failure_atom(start: &OperationCustodyCandidate) -> OperationCustodyCandidate {
    let observation = observation_candidate(
        start
            .record(OperationCustodyRecordKind::OperationObservation)
            .unwrap()
            .clone(),
        "one",
        '4',
    );
    let mut records = observation.records().to_vec();
    records.extend([
        OperationCustodyRecord::AdapterFailure(member_record(
            "adapter-failure-1",
            '5',
            ADAPTER_FAILURE_V1_SCHEMA,
        )),
        OperationCustodyRecord::CapabilityResult(member_record(
            "result-1",
            '6',
            CAPABILITY_RESULT_V1_SCHEMA,
        )),
        OperationCustodyRecord::Diagnostic(member_record(
            "diagnostic-1",
            '7',
            DIAGNOSTIC_V1_SCHEMA,
        )),
    ]);
    OperationCustodyCandidate::new(
        store(),
        OperationCustodyTransitionKind::Observe,
        records,
        observation
            .indexes()
            .iter()
            .filter(|index| {
                matches!(
                    index,
                    OperationCustodyIndexMutation::LatestObservation { .. }
                )
            })
            .cloned()
            .collect(),
        observation.checkpoint().clone(),
        observation.accumulator().clone(),
    )
    .unwrap()
}

#[test]
fn safe_provider_failure_atom_commits_and_replays_as_one_sqlite_transaction() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_store(&connection);
    let start = start_candidate("one", ['1', '2', '3', '9']);
    let failure = with_safe_failure_atom(&start);
    let mut custody = SqliteOperationCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        custody.compare_and_commit(&start).unwrap(),
        OperationCustodyCommitOutcome::Committed
    );
    assert_eq!(
        custody.compare_and_commit(&failure).unwrap(),
        OperationCustodyCommitOutcome::Committed
    );
    assert_eq!(
        custody.observe_exact(&failure).unwrap(),
        OperationCustodyReadBackOutcome::ExactSuccessor
    );
    assert_eq!(
        custody.compare_and_commit(&failure).unwrap(),
        OperationCustodyCommitOutcome::ExactReplay
    );
    assert!(failure
        .record(OperationCustodyRecordKind::AdapterFailure)
        .is_some());
    assert!(failure
        .record(OperationCustodyRecordKind::CapabilityResult)
        .is_some());
    assert!(failure
        .record(OperationCustodyRecordKind::Diagnostic)
        .is_some());
}

#[test]
fn optional_member_and_observation_reopen_replay_and_share_the_checkpoint_transaction() {
    for schema in MEMBER_SCHEMAS {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("result.sqlite");
        let start = start_candidate("one", ['1', '2', '3', '9']);
        let result = member_record("result-1", '5', schema);
        let candidate = with_member(&start, result.clone());
        {
            let mut connection = Connection::open(&path).unwrap();
            initialize_store(&connection);
            let mut custody = SqliteOperationCustody {
                connection: &mut connection,
                coordinate: store(),
            };
            assert_eq!(
                custody.compare_and_commit(&start).unwrap(),
                OperationCustodyCommitOutcome::Committed
            );
            assert_eq!(
                custody.compare_and_commit(&candidate).unwrap(),
                OperationCustodyCommitOutcome::Committed
            );
        }
        let mut connection = Connection::open(&path).unwrap();
        connection
            .pragma_update(None, "foreign_keys", true)
            .unwrap();
        let before = connection.total_changes();
        {
            let mut custody = SqliteOperationCustody {
                connection: &mut connection,
                coordinate: store(),
            };
            assert_eq!(
                custody.observe_exact(&candidate).unwrap(),
                OperationCustodyReadBackOutcome::ExactSuccessor
            );
            assert_eq!(
                custody.compare_and_commit(&candidate).unwrap(),
                OperationCustodyCommitOutcome::ExactReplay
            );
            for member in candidate.records() {
                let expected = member.record();
                assert_eq!(
                    custody
                        .read_operation_record(
                            &store(),
                            expected.coordinate(),
                            expected.canonical_record_jcs().as_slice().len()
                        )
                        .unwrap(),
                    Some(expected.clone())
                );
            }
            assert!(custody
                .read_operation_record(
                    &store(),
                    result.coordinate(),
                    result.canonical_record_jcs().as_slice().len() - 1
                )
                .is_err());
        }
        for member in candidate
            .records()
            .iter()
            .map(OperationCustodyRecord::record)
            .chain(std::iter::once(candidate.checkpoint().successor_record()))
        {
            let stamp: i64 = connection.query_row("SELECT inserted_transaction_sequence FROM pale_deployment_control_record WHERE record_ref=?1", [member.coordinate().record_ref().as_str()], |row| row.get(0)).unwrap();
            assert_eq!(
                u64::try_from(stamp).unwrap(),
                candidate
                    .checkpoint()
                    .successor()
                    .committed_transaction_sequence()
                    .get()
            );
        }
        assert_eq!(candidate.indexes().len(), 2);
        assert_eq!(connection.total_changes(), before);
    }
}

#[test]
fn failed_optional_member_observation_or_checkpoint_write_rolls_back_the_whole_append() {
    for schema in MEMBER_SCHEMAS {
        for reference in ["result-1", "observation-one-2", "checkpoint-10-one"] {
            let mut connection = Connection::open_in_memory().unwrap();
            initialize_store(&connection);
            let start = start_candidate("one", ['1', '2', '3', '9']);
            let result = member_record("result-1", '5', schema);
            let candidate = with_member(&start, result.clone());
            SqliteOperationCustody {
                connection: &mut connection,
                coordinate: store(),
            }
            .compare_and_commit(&start)
            .unwrap();
            connection.execute_batch(&format!("CREATE TRIGGER abort_result BEFORE INSERT ON pale_deployment_control_record WHEN NEW.record_ref='{reference}' BEGIN SELECT RAISE(ABORT,'injected record write'); END;")).unwrap();
            {
                let mut custody = SqliteOperationCustody {
                    connection: &mut connection,
                    coordinate: store(),
                };
                assert!(custody.compare_and_commit(&candidate).is_err());
                assert_eq!(
                    custody.observe_exact(&candidate).unwrap(),
                    OperationCustodyReadBackOutcome::Predecessor
                );
                assert_eq!(
                    custody.observe_exact(&start).unwrap(),
                    OperationCustodyReadBackOutcome::ExactSuccessor
                );
                assert_eq!(
                    custody
                        .read_operation_record(&store(), result.coordinate(), 1024)
                        .unwrap(),
                    None
                );
            }
            assert_eq!(
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM pale_deployment_control_record",
                        [],
                        |row| row.get::<_, i64>(0)
                    )
                    .unwrap(),
                5
            );
            connection
                .execute_batch("DROP TRIGGER abort_result")
                .unwrap();
            assert_eq!(
                SqliteOperationCustody {
                    connection: &mut connection,
                    coordinate: store()
                }
                .compare_and_commit(&candidate)
                .unwrap(),
                OperationCustodyCommitOutcome::Committed
            );
        }
    }
}

#[test]
fn missing_substituted_or_wrong_transaction_member_is_not_exact_replay() {
    for schema in MEMBER_SCHEMAS {
        for corruption in [
            "DELETE FROM pale_deployment_control_record WHERE record_ref='result-1'",
            "UPDATE pale_deployment_control_record SET canonical_record_jcs=x'7b7d' WHERE record_ref='result-1'",
            "UPDATE pale_deployment_control_record SET schema_id='pale/operation-observation.v1' WHERE record_ref='result-1'",
            "UPDATE pale_deployment_control_record SET schema_sha256=printf('%064d',0) WHERE record_ref='result-1'",
            "UPDATE pale_deployment_control_record SET record_sha256=printf('%064d',0) WHERE record_ref='result-1'",
            "UPDATE pale_deployment_control_record SET inserted_transaction_sequence=29 WHERE record_ref='result-1'",
        ] {
            let mut connection = Connection::open_in_memory().unwrap();
            initialize_store(&connection);
            let start = start_candidate("one", ['1', '2', '3', '9']);
            let candidate = with_member(&start, member_record("result-1", '5', schema));
            {
                let mut custody = SqliteOperationCustody {
                    connection: &mut connection,
                    coordinate: store(),
                };
                custody.compare_and_commit(&start).unwrap();
                custody.compare_and_commit(&candidate).unwrap();
            }
            connection.execute_batch(corruption).unwrap();
            let before = connection.total_changes();
            let mut custody = SqliteOperationCustody {
                connection: &mut connection,
                coordinate: store(),
            };
            assert_eq!(
                custody.observe_exact(&candidate).unwrap(),
                OperationCustodyReadBackOutcome::ConflictingOrPartial
            );
            assert_eq!(
                custody.compare_and_commit(&candidate).unwrap_err().kind(),
                ControlStoreErrorKind::RecoveryRequired
            );
            assert_eq!(connection.total_changes(), before);
        }
    }
}

#[test]
fn different_optional_member_under_the_same_closing_checkpoint_is_not_a_replay() {
    for schema in MEMBER_SCHEMAS {
        let mut connection = Connection::open_in_memory().unwrap();
        initialize_store(&connection);
        let start = start_candidate("one", ['1', '2', '3', '9']);
        let candidate = with_member(&start, member_record("result-1", '5', schema));
        let substitute = with_member(&start, member_record("result-2", '6', schema));
        let mut custody = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        custody.compare_and_commit(&start).unwrap();
        custody.compare_and_commit(&candidate).unwrap();
        assert_eq!(
            custody.observe_exact(&substitute).unwrap(),
            OperationCustodyReadBackOutcome::ConflictingOrPartial
        );
        assert_eq!(
            custody.compare_and_commit(&substitute).unwrap_err().kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
    }
}

#[test]
fn a_committed_optional_member_cannot_be_omitted_from_exact_replay() {
    for schema in MEMBER_SCHEMAS {
        let mut connection = Connection::open_in_memory().unwrap();
        initialize_store(&connection);
        let start = start_candidate("one", ['1', '2', '3', '9']);
        let with = with_member(&start, member_record("result-1", '5', schema));
        let without = observation_candidate(
            start
                .record(OperationCustodyRecordKind::OperationObservation)
                .unwrap()
                .clone(),
            "one",
            '4',
        );
        let mut custody = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        custody.compare_and_commit(&start).unwrap();
        custody.compare_and_commit(&with).unwrap();
        assert_eq!(
            custody.observe_exact(&without).unwrap(),
            OperationCustodyReadBackOutcome::ConflictingOrPartial
        );
        assert_eq!(
            custody.compare_and_commit(&without).unwrap_err().kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
        assert_eq!(
            custody.observe_exact(&with).unwrap(),
            OperationCustodyReadBackOutcome::ExactSuccessor
        );
    }
}

#[test]
fn an_extra_member_of_any_optional_role_prevents_exact_replay() {
    for schema in MEMBER_SCHEMAS {
        for extra_schema in MEMBER_SCHEMAS {
            let mut connection = Connection::open_in_memory().unwrap();
            initialize_store(&connection);
            let start = start_candidate("one", ['1', '2', '3', '9']);
            let candidate = with_member(&start, member_record("result-1", '5', schema));
            {
                let mut custody = SqliteOperationCustody {
                    connection: &mut connection,
                    coordinate: store(),
                };
                custody.compare_and_commit(&start).unwrap();
                custody.compare_and_commit(&candidate).unwrap();
            }
            insert_record(
                &connection,
                &member_record("result-extra", '6', extra_schema),
            );
            let before = connection.total_changes();
            let mut custody = SqliteOperationCustody {
                connection: &mut connection,
                coordinate: store(),
            };
            assert_eq!(
                custody.observe_exact(&candidate).unwrap(),
                OperationCustodyReadBackOutcome::ConflictingOrPartial
            );
            assert_eq!(
                custody.compare_and_commit(&candidate).unwrap_err().kind(),
                ControlStoreErrorKind::RecoveryRequired
            );
            assert_eq!(connection.total_changes(), before);
        }
    }
}

#[test]
fn observation_only_replay_ignores_optional_members_from_other_transactions() {
    for schema in MEMBER_SCHEMAS {
        let mut connection = Connection::open_in_memory().unwrap();
        initialize_store(&connection);
        // This older opaque record is not a member of either following atom.
        insert_record(&connection, &member_record("older-result", '5', schema));
        let start = start_candidate("one", ['1', '2', '3', '9']);
        let candidate = observation_candidate(
            start
                .record(OperationCustodyRecordKind::OperationObservation)
                .unwrap()
                .clone(),
            "one",
            '4',
        );
        let mut custody = SqliteOperationCustody {
            connection: &mut connection,
            coordinate: store(),
        };
        assert_eq!(
            custody.compare_and_commit(&start).unwrap(),
            OperationCustodyCommitOutcome::Committed
        );
        assert_eq!(
            custody.compare_and_commit(&candidate).unwrap(),
            OperationCustodyCommitOutcome::Committed
        );
        assert_eq!(
            custody.observe_exact(&candidate).unwrap(),
            OperationCustodyReadBackOutcome::ExactSuccessor
        );
        assert_eq!(
            custody.compare_and_commit(&candidate).unwrap(),
            OperationCustodyCommitOutcome::ExactReplay
        );
    }
}
