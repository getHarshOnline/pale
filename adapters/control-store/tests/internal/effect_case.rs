//! Behavioral tests for deployment-bound effect-case custody.

use pale_core::records::{
    ControlCheckpointDomainIdV1, ControlDomainEpochV1, ControlDomainSequenceV1,
    ControlIncarnationIdV1, ControlStoreIdV1, ResidentRecordRefV1, SchemaDigest, SchemaId,
    ACTION_ENVELOPE_V1_SCHEMA, COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
    CONTROL_STATE_CHECKPOINT_V1_SCHEMA, EFFECT_ADMISSION_V1_SCHEMA,
    EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
};
use pale_ports::{
    AuthorityStoreCoordinate, ControlCheckpointHead, ControlRecord, ControlRecordBytes,
    ControlRecordCoordinate, ControlRecordDigest, ControlStoreErrorKind,
    EffectCaseActionAdmissionKind, EffectCaseCheckpointMutation, EffectCaseCheckpointTransition,
    EffectCaseCommitOutcome, EffectCaseCustodyPort, EffectCaseIndexAssertion, EffectCaseIndexKey,
    EffectCaseIndexMutation, EffectCaseIndexPredecessor, EffectCaseIndexQuery,
    EffectCaseReadBackOutcome, EffectCaseTransitionCandidate, EffectCaseTransitionKind,
    EffectCaseTransitionMember,
};
use rusqlite::{params, Connection};

use super::SqliteEffectCaseCustody;

const MIGRATION: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/deployment-control/0001_initial_v1.sql"
));

fn digest(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn coordinate(reference: &str, byte: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(reference).expect("record reference"),
        ControlRecordDigest::parse(digest(byte)).expect("record digest"),
    )
}

fn record(reference: &str, byte: char, schema_id: &str) -> ControlRecord {
    ControlRecord::new(
        coordinate(reference, byte),
        SchemaId::parse(schema_id).expect("schema identity"),
        SchemaDigest::parse(digest('f')).expect("schema digest"),
        ControlRecordBytes::new(format!(r#"{{"record":"{reference}"}}"#).into_bytes())
            .expect("record bytes"),
    )
}

fn key(byte: char) -> EffectCaseIndexKey {
    EffectCaseIndexKey::parse(digest(byte)).expect("index key")
}

fn store() -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("deployment-store-1").expect("store identity"),
        ControlIncarnationIdV1::parse("deployment-incarnation-1").expect("store incarnation"),
    )
}

fn checkpoint(sequence: u64, reference: &str, byte: char) -> ControlCheckpointHead {
    ControlCheckpointHead::new(
        ControlCheckpointDomainIdV1::parse("effect:resource-1").expect("checkpoint domain"),
        ControlDomainEpochV1::new(4).expect("domain epoch"),
        ControlDomainSequenceV1::new(sequence).expect("domain sequence"),
        pale_core::records::ControlTransactionSequenceV1::new(sequence + 20).unwrap(),
        coordinate(reference, byte),
    )
}

fn candidate(tag: &str, bytes: [char; 5]) -> EffectCaseTransitionCandidate {
    let request_ref = format!("request-{tag}");
    let action_ref = format!("action-{tag}");
    let claim_ref = format!("claim-{tag}");
    let admission_ref = format!("admission-{tag}");
    let checkpoint_ref = format!("checkpoint-9-{tag}");
    let members = vec![
        EffectCaseTransitionMember::CommonRequest(record(
            &request_ref,
            bytes[0],
            COMMON_REQUEST_ENVELOPE_V1_SCHEMA,
        )),
        EffectCaseTransitionMember::Action(record(
            &action_ref,
            bytes[1],
            ACTION_ENVELOPE_V1_SCHEMA,
        )),
        EffectCaseTransitionMember::ConflictClaim(record(
            &claim_ref,
            bytes[2],
            EFFECT_CONFLICT_CLAIM_V1_SCHEMA,
        )),
        EffectCaseTransitionMember::Admission(record(
            &admission_ref,
            bytes[3],
            EFFECT_ADMISSION_V1_SCHEMA,
        )),
    ];
    let mutations = vec![
        EffectCaseIndexMutation::RequestIdentity {
            key: key('a'),
            expected: EffectCaseIndexPredecessor::Absent,
            successor: coordinate(&request_ref, bytes[0]),
        },
        EffectCaseIndexMutation::LogicalAction {
            key: key('b'),
            expected: EffectCaseIndexPredecessor::Absent,
            successor: coordinate(&admission_ref, bytes[3]),
        },
        EffectCaseIndexMutation::ActionDisposition {
            key: key('b'),
            expected: EffectCaseIndexPredecessor::Absent,
            successor: coordinate(&admission_ref, bytes[3]),
        },
        EffectCaseIndexMutation::ActiveClaim {
            key: key('c'),
            expected: EffectCaseIndexPredecessor::Absent,
            successor: coordinate(&claim_ref, bytes[2]),
        },
    ];
    let checkpoint = EffectCaseCheckpointTransition::new(
        checkpoint(8, "checkpoint-8", '8'),
        record("checkpoint-8", '8', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
        checkpoint(9, &checkpoint_ref, bytes[4]),
        record(
            &checkpoint_ref,
            bytes[4],
            CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
        ),
    )
    .expect("checkpoint transition");
    EffectCaseTransitionCandidate::new(
        store(),
        EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::New),
        members,
        mutations,
        vec![EffectCaseIndexAssertion::ActiveCustody {
            key: key('c'),
            expected: EffectCaseIndexPredecessor::Absent,
        }],
        EffectCaseCheckpointMutation::advance(checkpoint),
    )
    .expect("effect-case candidate")
}

fn initialize_store() -> Connection {
    let connection = Connection::open_in_memory().expect("in-memory SQLite");
    connection
        .pragma_update(None, "foreign_keys", true)
        .expect("foreign keys");
    connection.execute_batch(MIGRATION).expect("migration");
    connection
        .execute(
            "INSERT INTO pale_control_store_header \
             (singleton, store_kind, logical_store_id, control_incarnation_id, store_schema_id, \
              store_schema_sha256, store_schema_version, storage_manifest_sha256, migration_head_sha256, logical_transaction_sequence) \
             VALUES (1, 'deployment_control', ?1, ?2, \
                     'pale/control-store/deployment-control.v1', ?3, 1, ?4, ?5, 28)",
            params![
                store().store_id().as_str(),
                store().incarnation_id().as_str(),
                digest('a'),
                digest('b'),
                digest('c'),
            ],
        )
        .expect("store header");
    let predecessor = record("checkpoint-8", '8', CONTROL_STATE_CHECKPOINT_V1_SCHEMA);
    insert_record(&connection, &predecessor);
    connection
        .execute(
            "INSERT INTO pale_deployment_effect_case_checkpoint_head \
             (checkpoint_domain_id, domain_epoch, domain_sequence, \
              control_checkpoint_ref, control_checkpoint_sha256) VALUES (?1, 4, 8, ?2, ?3)",
            params![
                "effect:resource-1",
                predecessor.coordinate().record_ref().as_str(),
                predecessor.coordinate().record_sha256().as_str(),
            ],
        )
        .expect("checkpoint head");
    connection
}

fn insert_record(connection: &Connection, record: &ControlRecord) {
    connection
        .execute(
            "INSERT INTO pale_deployment_control_record \
             (record_ref, record_sha256, schema_id, schema_sha256, canonical_record_jcs, inserted_transaction_sequence) \
             VALUES (?1, ?2, ?3, ?4, ?5, (SELECT logical_transaction_sequence FROM pale_control_store_header WHERE singleton=1))",
            params![
                record.coordinate().record_ref().as_str(),
                record.coordinate().record_sha256().as_str(),
                record.schema_id().as_str(),
                record.schema_sha256().as_str(),
                record.canonical_record_jcs().as_slice(),
            ],
        )
        .expect("seed record");
}

#[test]
fn atomic_commit_exact_replay_and_point_reads_preserve_one_transition() {
    let mut connection = initialize_store();
    let candidate = candidate("one", ['1', '2', '3', '4', '9']);
    let request = candidate.members()[0].record().clone();
    let action = candidate.members()[1].record().clone();
    let mut custody = SqliteEffectCaseCustody {
        connection: &mut connection,
        coordinate: store(),
    };

    assert_eq!(
        custody.compare_and_commit(&candidate).expect("commit"),
        EffectCaseCommitOutcome::Committed
    );
    assert_eq!(
        crate::store::transaction::read(custody.connection, &store())
            .unwrap()
            .get(),
        29
    );
    assert_eq!(
        custody.observe_exact(&candidate).expect("observe"),
        EffectCaseReadBackOutcome::ExactSuccessor
    );
    assert_eq!(
        custody.compare_and_commit(&candidate).expect("replay"),
        EffectCaseCommitOutcome::ExactReplay
    );
    assert_eq!(
        crate::store::transaction::read(custody.connection, &store())
            .unwrap()
            .get(),
        29
    );

    let request_observation = custody
        .read_index(
            candidate.store(),
            &EffectCaseIndexQuery::RequestIdentity(key('a')),
        )
        .expect("request index");
    assert_eq!(request_observation.record(), Some(&request));
    let action_observation = custody
        .read_record(candidate.store(), action.coordinate())
        .expect("action record");
    assert_eq!(action_observation.record(), Some(&action));
    let absent = custody
        .read_index(
            candidate.store(),
            &EffectCaseIndexQuery::RequestIdentity(key('f')),
        )
        .expect("absent request index");
    assert!(absent.record().is_none());
}

#[test]
fn unchanged_checkpoint_alias_is_a_new_transaction_but_retained_dependencies_keep_their_stamp() {
    let mut connection = initialize_store();
    let original = candidate("clock", ['1', '2', '3', '4', '9']);
    let admission = original.members()[3].record().clone();
    SqliteEffectCaseCustody {
        connection: &mut connection,
        coordinate: store(),
    }
    .compare_and_commit(&original)
    .unwrap();
    let request = record("alias-request", '6', COMMON_REQUEST_ENVELOPE_V1_SCHEMA);
    let alias = EffectCaseTransitionCandidate::new(
        store(),
        EffectCaseTransitionKind::AdmitAction(EffectCaseActionAdmissionKind::AliasExisting),
        vec![EffectCaseTransitionMember::CommonRequest(request.clone())],
        vec![EffectCaseIndexMutation::RequestIdentity {
            key: key('d'),
            expected: EffectCaseIndexPredecessor::Absent,
            successor: request.coordinate().clone(),
        }],
        vec![
            EffectCaseIndexAssertion::LogicalAction {
                key: key('b'),
                expected: EffectCaseIndexPredecessor::Exact(admission.clone()),
            },
            EffectCaseIndexAssertion::ActionDisposition {
                key: key('b'),
                expected: EffectCaseIndexPredecessor::Exact(admission.clone()),
            },
        ],
        EffectCaseCheckpointMutation::Unchanged,
    )
    .unwrap();
    let mut custody = SqliteEffectCaseCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        custody.compare_and_commit(&alias).unwrap(),
        EffectCaseCommitOutcome::Committed
    );
    assert_eq!(
        crate::store::transaction::read(custody.connection, &store())
            .unwrap()
            .get(),
        30
    );
    assert_eq!(
        crate::store::transaction::record_stamp(
            custody.connection,
            crate::ControlStoreKind::DeploymentControl,
            request.coordinate()
        )
        .unwrap()
        .unwrap()
        .get(),
        30
    );
    assert_eq!(
        crate::store::transaction::record_stamp(
            custody.connection,
            crate::ControlStoreKind::DeploymentControl,
            admission.coordinate()
        )
        .unwrap()
        .unwrap()
        .get(),
        29
    );
    let changes = custody.connection.total_changes();
    assert_eq!(
        custody.compare_and_commit(&alias).unwrap(),
        EffectCaseCommitOutcome::ExactReplay
    );
    assert_eq!(custody.connection.total_changes(), changes);
    assert_eq!(
        custody
            .connection
            .query_row(
                "SELECT domain_sequence FROM pale_deployment_effect_case_checkpoint_head",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        9
    );
}

#[test]
fn stale_complete_candidate_is_a_conflict_not_recovery_state() {
    let mut connection = initialize_store();
    let first = candidate("one", ['1', '2', '3', '4', '9']);
    let stale = candidate("two", ['a', 'b', 'c', 'd', 'e']);
    let mut custody = SqliteEffectCaseCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    custody.compare_and_commit(&first).expect("first commit");

    assert_eq!(
        custody
            .compare_and_commit(&stale)
            .expect_err("stale candidate")
            .kind(),
        ControlStoreErrorKind::CandidateConflict
    );
    assert_eq!(
        custody.observe_exact(&stale).expect("stale observation"),
        EffectCaseReadBackOutcome::ConflictingOrPartial
    );
}

#[test]
fn orphaned_candidate_record_requires_recovery() {
    let mut connection = initialize_store();
    let candidate = candidate("one", ['1', '2', '3', '4', '9']);
    insert_record(&connection, candidate.members()[0].record());
    let mut custody = SqliteEffectCaseCustody {
        connection: &mut connection,
        coordinate: store(),
    };

    assert_eq!(
        custody
            .compare_and_commit(&candidate)
            .expect_err("partial candidate")
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

#[test]
fn failed_index_write_rolls_back_records_and_checkpoint() {
    let mut connection = initialize_store();
    connection
        .execute_batch(
            "CREATE TEMP TRIGGER reject_effect_index BEFORE INSERT \
             ON pale_deployment_effect_case_index \
             BEGIN SELECT RAISE(ABORT, 'injected index failure'); END;",
        )
        .expect("failure trigger");
    let candidate = candidate("one", ['1', '2', '3', '4', '9']);
    let mut custody = SqliteEffectCaseCustody {
        connection: &mut connection,
        coordinate: store(),
    };
    assert_eq!(
        custody
            .compare_and_commit(&candidate)
            .expect_err("injected failure")
            .kind(),
        ControlStoreErrorKind::Storage
    );
    drop(custody);

    let counts = connection
        .query_row(
            "SELECT \
             (SELECT COUNT(*) FROM pale_deployment_control_record), \
             (SELECT COUNT(*) FROM pale_deployment_effect_case_index), \
             (SELECT domain_sequence FROM pale_deployment_effect_case_checkpoint_head \
              WHERE checkpoint_domain_id = 'effect:resource-1')",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .expect("post-failure state");
    assert_eq!(counts, (1, 0, 8));
}
