//! Production `SQLite` producer-incarnation custody tests.

use super::*;

#[path = "producer/readback.rs"]
mod readback;

#[path = "principal_trust_read.rs"]
mod trust_read;

#[path = "producer/custody_anchors.rs"]
mod custody_anchors;

#[path = "producer/accumulator.rs"]
mod accumulator_custody;

use pale_core::records::{
    ControlCheckpointDomainIdV1, ControlDomainEpochV1, ControlDomainSequenceV1,
    ControlIncarnationIdV1, ControlStoreIdV1, PrincipalTrustCheckpointEpochV1, ProducerEpochV1,
    ProducerIdV1, ResidentPrincipalIdV1, ResidentRecordRefV1, SchemaDigest, SchemaId,
    CONTROL_STATE_CHECKPOINT_V1_SCHEMA, PRODUCER_INCARNATION_RETIREMENT_V1_SCHEMA,
    PRODUCER_INCARNATION_V1_SCHEMA,
};
use pale_ports::{
    ControlCheckpointHead, ControlRecord, ControlRecordBytes, ControlRecordCoordinate,
    ControlRecordDigest, PrincipalProducerIncarnationExpectedFrontier,
    PrincipalProducerIncarnationSuccessorFrontier, PrincipalTrustCheckpointHead,
    PrincipalTrustSnapshot, PrincipalTrustStoreBinding,
};

const MIGRATION: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/principal-trust/0001_initial_v1.sql"
));

fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn coordinate(reference: &str, byte: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(reference).expect("record ref"),
        ControlRecordDigest::parse(sha(byte)).expect("record digest"),
    )
}

fn record(reference: &str, byte: char) -> ControlRecord {
    let schema = if reference.starts_with("allocation-") {
        PRODUCER_INCARNATION_V1_SCHEMA
    } else if reference.starts_with("retirement-") {
        PRODUCER_INCARNATION_RETIREMENT_V1_SCHEMA
    } else if reference.starts_with("trust-") {
        pale_core::records::PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA
    } else if reference.starts_with("control-") {
        CONTROL_STATE_CHECKPOINT_V1_SCHEMA
    } else {
        "pale/test.v1"
    };
    let body = if reference.starts_with("control-") {
        format!(
            r#"{{"checkpoint_domain_id":"producer:producer-1","domain_sequence":{}}}"#,
            reference.trim_start_matches("control-")
        )
    } else {
        format!(r#"{{"record":"{reference}"}}"#)
    };
    ControlRecord::new(
        coordinate(reference, byte),
        SchemaId::parse(schema).expect("schema"),
        SchemaDigest::parse(sha('f')).expect("schema digest"),
        ControlRecordBytes::new(body.into_bytes()).expect("record bytes"),
    )
}

fn binding() -> PrincipalTrustStoreBinding {
    PrincipalTrustStoreBinding::new(
        ControlStoreIdV1::parse("trust-store-1").expect("store"),
        ControlIncarnationIdV1::parse("trust-incarnation-1").expect("incarnation"),
        ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
    )
}

fn trust_head(epoch: u64) -> PrincipalTrustCheckpointHead {
    PrincipalTrustCheckpointHead::new(
        binding().principal_id().clone(),
        PrincipalTrustCheckpointEpochV1::new(epoch).expect("trust epoch"),
        coordinate(&format!("trust-{epoch}"), digit(epoch)),
    )
}

fn checkpoint(sequence: u64) -> ControlCheckpointHead {
    ControlCheckpointHead::new(
        ControlCheckpointDomainIdV1::parse("producer:producer-1").expect("domain"),
        ControlDomainEpochV1::new(1).expect("domain epoch"),
        ControlDomainSequenceV1::new(sequence).expect("domain sequence"),
        pale_core::records::ControlTransactionSequenceV1::new(sequence + 20).unwrap(),
        coordinate(&format!("control-{sequence}"), digit(sequence + 4)),
    )
}

fn digit(value: u64) -> char {
    char::from(b'0' + u8::try_from(value).expect("single test digit"))
}

fn producer_head(
    epoch: char,
    state: ProducerIncarnationHeadStateV1,
    retirement: Option<ControlRecordCoordinate>,
) -> PrincipalProducerIncarnationHead {
    PrincipalProducerIncarnationHead::new(
        binding().principal_id().clone(),
        ProducerIdV1::parse("producer-1").expect("producer"),
        ProducerEpochV1::parse(std::iter::repeat_n(epoch, 32).collect::<String>())
            .expect("producer epoch"),
        state,
        coordinate(&format!("allocation-{epoch}"), epoch),
        retirement,
    )
    .expect("producer head")
}

fn expected_frontier(
    producer: Option<(PrincipalProducerIncarnationHead, Vec<ControlRecord>)>,
    trust_epoch: u64,
    control_sequence: u64,
) -> PrincipalProducerIncarnationExpectedFrontier {
    PrincipalProducerIncarnationExpectedFrontier::new(
        producer,
        (
            trust_head(trust_epoch),
            record(&format!("trust-{trust_epoch}"), digit(trust_epoch)),
        ),
        (
            checkpoint(control_sequence),
            record(
                &format!("control-{control_sequence}"),
                digit(control_sequence + 4),
            ),
        ),
    )
}

fn successor_frontier(
    producer: PrincipalProducerIncarnationHead,
    trust_epoch: u64,
    control_sequence: u64,
) -> PrincipalProducerIncarnationSuccessorFrontier {
    PrincipalProducerIncarnationSuccessorFrontier::new(
        producer,
        trust_head(trust_epoch),
        checkpoint(control_sequence),
    )
}

fn initial_candidate() -> PreparedPrincipalProducerIncarnationTransition {
    PreparedPrincipalProducerIncarnationTransition::new(
        binding(),
        PrincipalProducerIncarnationTransitionKind::InitialAllocation,
        expected_frontier(None, 1, 1),
        vec![
            record("allocation-a", 'a'),
            record("trust-2", '2'),
            record("control-2", '6'),
        ],
        successor_frontier(
            producer_head('a', ProducerIncarnationHeadStateV1::Active, None),
            2,
            2,
        ),
    )
    .expect("initial candidate")
}

fn retirement_candidate() -> PreparedPrincipalProducerIncarnationTransition {
    let active = producer_head('a', ProducerIncarnationHeadStateV1::Active, None);
    let retired = producer_head(
        'a',
        ProducerIncarnationHeadStateV1::Retired,
        Some(coordinate("retirement-a", 'b')),
    );
    PreparedPrincipalProducerIncarnationTransition::new(
        binding(),
        PrincipalProducerIncarnationTransitionKind::Retirement,
        expected_frontier(Some((active, vec![record("allocation-a", 'a')])), 2, 2),
        vec![
            record("retirement-a", 'b'),
            record("trust-3", '3'),
            record("control-3", '7'),
        ],
        successor_frontier(retired, 3, 3),
    )
    .expect("retirement candidate")
}

fn successor_candidate() -> PreparedPrincipalProducerIncarnationTransition {
    let retired = producer_head(
        'a',
        ProducerIncarnationHeadStateV1::Retired,
        Some(coordinate("retirement-a", 'b')),
    );
    PreparedPrincipalProducerIncarnationTransition::new(
        binding(),
        PrincipalProducerIncarnationTransitionKind::SuccessorAllocation,
        expected_frontier(
            Some((
                retired,
                vec![record("allocation-a", 'a'), record("retirement-a", 'b')],
            )),
            3,
            3,
        ),
        vec![
            record("allocation-c", 'c'),
            record("trust-4", '4'),
            record("control-4", '8'),
        ],
        successor_frontier(
            producer_head('c', ProducerIncarnationHeadStateV1::Active, None),
            4,
            4,
        ),
    )
    .expect("successor candidate")
}

fn initialize_store() -> Connection {
    initialize_connection(Connection::open_in_memory().expect("connection"))
}

fn initialize_connection(connection: Connection) -> Connection {
    initialize_candidate_connection(connection, &initial_candidate())
}

fn initialize_candidate_connection(
    connection: Connection,
    candidate: &PreparedPrincipalProducerIncarnationTransition,
) -> Connection {
    let binding = candidate.binding();
    let trust = candidate.expected_trust_record();
    let control = candidate.expected_checkpoint_record();
    let trust_head = candidate.expected_trust();
    let control_head = candidate.expected_checkpoint();
    connection
        .pragma_update(None, "foreign_keys", true)
        .expect("foreign keys");
    connection
        .pragma_update(None, "trusted_schema", false)
        .expect("untrusted schema");
    connection.execute_batch(MIGRATION).expect("migration");
    connection
        .execute(
            "INSERT INTO pale_control_store_header \
             (singleton, store_kind, logical_store_id, control_incarnation_id, store_schema_id, \
              store_schema_sha256, store_schema_version, storage_manifest_sha256, migration_head_sha256, logical_transaction_sequence) \
             VALUES (1, 'principal_trust', ?4, ?5, \
                     'pale/control-store/principal-trust.v1', ?1, 1, ?2, ?3, ?6)",
            params![
                sha('1'), sha('2'), sha('3'),
                binding.trust_store_id().as_str(),
                binding.trust_store_incarnation_id().as_str(),
                crate::error::sqlite_integer(control_head.committed_transaction_sequence().get())
                    .expect("fixture transaction sequence")
            ],
        )
        .expect("header");
    connection
        .execute(
            "INSERT INTO pale_principal_trust_store_binding (singleton, principal_id) \
             VALUES (1, ?1)",
            params![binding.principal_id().as_str()],
        )
        .expect("binding");
    insert_record(&connection, binding, trust).expect("trust record");
    insert_record(&connection, binding, control).expect("control record");
    connection
        .execute(
            "INSERT INTO pale_principal_trust_checkpoint_head \
             (singleton, principal_id, checkpoint_epoch, trust_checkpoint_ref, \
              trust_checkpoint_sha256) VALUES (1, ?3, ?4, ?1, ?2)",
            params![
                trust.coordinate().record_ref().as_str(),
                trust.coordinate().record_sha256().as_str(),
                binding.principal_id().as_str(),
                crate::error::sqlite_integer(trust_head.checkpoint_epoch().get())
                    .expect("fixture trust checkpoint epoch")
            ],
        )
        .expect("trust head");
    connection
        .execute(
            "INSERT INTO pale_principal_control_checkpoint_head \
             (checkpoint_domain_id, domain_epoch, domain_sequence, control_checkpoint_ref, \
              control_checkpoint_sha256) VALUES (?3, ?4, ?5, ?1, ?2)",
            params![
                control.coordinate().record_ref().as_str(),
                control.coordinate().record_sha256().as_str(),
                control_head.checkpoint_domain_id().as_str(),
                crate::error::sqlite_integer(control_head.domain_epoch().get())
                    .expect("fixture control domain epoch"),
                crate::error::sqlite_integer(control_head.domain_sequence().get())
                    .expect("fixture control domain sequence")
            ],
        )
        .expect("control head");
    connection
}

fn transition_row_counts(connection: &Connection) -> (i64, i64, i64, i64) {
    connection
        .query_row(
            "SELECT \
             (SELECT COUNT(*) FROM pale_principal_trust_record), \
             (SELECT COUNT(*) FROM pale_principal_producer_incarnation), \
             (SELECT COUNT(*) FROM pale_principal_producer_incarnation_retirement), \
             (SELECT COUNT(*) FROM pale_principal_producer_incarnation_head)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("producer transition row counts")
}

fn assert_injected_failure_rolls_back(
    mut connection: Connection,
    candidate: &PreparedPrincipalProducerIncarnationTransition,
    trigger: &str,
) {
    let before = transition_row_counts(&connection);
    let coordinate = crate::principal_trust::store_coordinate(candidate.binding());
    let transaction_before = crate::store::transaction::read(&connection, &coordinate).unwrap();
    connection
        .execute_batch(trigger)
        .expect("install producer transition failure trigger");
    let mut store = SqlitePrincipalProducerIncarnationCustody {
        connection: &mut connection,
    };
    assert_eq!(
        store.compare_and_commit(candidate).unwrap_err().kind(),
        ControlStoreErrorKind::Storage
    );
    assert_eq!(
        store.observe_exact(candidate).unwrap(),
        PrincipalProducerIncarnationDatabaseObservation::Predecessor
    );
    assert_eq!(transition_row_counts(&connection), before);
    assert_eq!(
        crate::store::transaction::read(&connection, &coordinate).unwrap(),
        transaction_before
    );
}

fn commit(
    connection: &mut Connection,
    candidate: &PreparedPrincipalProducerIncarnationTransition,
) -> PrincipalProducerIncarnationCommitDisposition {
    SqlitePrincipalProducerIncarnationCustody { connection }
        .compare_and_commit(candidate)
        .expect("producer transition commits")
}

#[test]
fn initial_allocation_commits_replays_and_reads_back_exactly() {
    let mut connection = initialize_store();
    let candidate = initial_candidate();
    let mut store = SqlitePrincipalProducerIncarnationCustody {
        connection: &mut connection,
    };
    assert_eq!(
        store.observe_exact(&candidate).unwrap(),
        PrincipalProducerIncarnationDatabaseObservation::Predecessor
    );
    assert_eq!(
        store.compare_and_commit(&candidate).unwrap(),
        PrincipalProducerIncarnationCommitDisposition::Committed
    );
    let coordinate = crate::principal_trust::store_coordinate(candidate.binding());
    assert_eq!(
        crate::store::transaction::read(store.connection, &coordinate)
            .unwrap()
            .get(),
        22
    );
    assert_eq!(
        store.observe_exact(&candidate).unwrap(),
        PrincipalProducerIncarnationDatabaseObservation::ExactSuccessor
    );
    assert_eq!(
        store.compare_and_commit(&candidate).unwrap(),
        PrincipalProducerIncarnationCommitDisposition::ExactReplay
    );
    assert_eq!(
        crate::store::transaction::read(store.connection, &coordinate)
            .unwrap()
            .get(),
        22
    );
    let state: (String, String) = connection
        .query_row(
            "SELECT producer_state, producer_epoch \
             FROM pale_principal_producer_incarnation_head",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(state, ("active".to_owned(), "a".repeat(32)));
}

#[test]
fn committed_transition_survives_reopen_and_replays_without_writes() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("principal-trust.sqlite3");
    let candidate = initial_candidate();
    {
        let mut connection =
            initialize_connection(Connection::open(&path).expect("file-backed connection"));
        assert_eq!(
            commit(&mut connection, &candidate),
            PrincipalProducerIncarnationCommitDisposition::Committed
        );
    }

    let mut reopened = Connection::open(&path).expect("reopen principal trust store");
    reopened
        .pragma_update(None, "foreign_keys", true)
        .expect("foreign keys");
    let changes_before = reopened.total_changes();
    let mut store = SqlitePrincipalProducerIncarnationCustody {
        connection: &mut reopened,
    };
    assert_eq!(
        store.observe_exact(&candidate).unwrap(),
        PrincipalProducerIncarnationDatabaseObservation::ExactSuccessor
    );
    assert_eq!(
        store.compare_and_commit(&candidate).unwrap(),
        PrincipalProducerIncarnationCommitDisposition::ExactReplay
    );
    assert_eq!(reopened.total_changes(), changes_before);
}

#[test]
fn every_allocation_transaction_boundary_rolls_back_to_the_exact_predecessor() {
    let candidate = initial_candidate();
    let record_triggers = candidate.records().iter().map(|record| {
        format!(
            "CREATE TEMP TRIGGER reject_boundary BEFORE INSERT ON \
             pale_principal_trust_record WHEN NEW.record_ref = '{}' \
             BEGIN SELECT RAISE(ABORT, 'injected producer record failure'); END;",
            record.coordinate().record_ref().as_str()
        )
    });
    let boundary_triggers = [
        "CREATE TEMP TRIGGER reject_boundary BEFORE INSERT ON \
         pale_principal_producer_incarnation \
         BEGIN SELECT RAISE(ABORT, 'injected allocation-index failure'); END;",
        "CREATE TEMP TRIGGER reject_boundary BEFORE INSERT ON \
         pale_principal_producer_incarnation_head \
         BEGIN SELECT RAISE(ABORT, 'injected producer-head failure'); END;",
        "CREATE TEMP TRIGGER reject_boundary BEFORE UPDATE ON \
         pale_principal_trust_checkpoint_head \
         BEGIN SELECT RAISE(ABORT, 'injected trust-head failure'); END;",
        "CREATE TEMP TRIGGER reject_boundary BEFORE UPDATE ON \
         pale_principal_control_checkpoint_head \
         BEGIN SELECT RAISE(ABORT, 'injected control-head failure'); END;",
    ];

    for trigger in record_triggers.chain(boundary_triggers.map(str::to_owned)) {
        assert_injected_failure_rolls_back(initialize_store(), &candidate, &trigger);
    }
}

#[test]
fn retirement_index_failure_rolls_back_to_the_exact_active_predecessor() {
    let candidate = retirement_candidate();
    let mut connection = initialize_store();
    assert_eq!(
        commit(&mut connection, &initial_candidate()),
        PrincipalProducerIncarnationCommitDisposition::Committed
    );
    assert_injected_failure_rolls_back(
        connection,
        &candidate,
        "CREATE TEMP TRIGGER reject_boundary BEFORE INSERT ON \
         pale_principal_producer_incarnation_retirement \
         BEGIN SELECT RAISE(ABORT, 'injected retirement-index failure'); END;",
    );
}

#[test]
fn retirement_and_successor_allocation_preserve_history_and_advance_all_heads() {
    let mut connection = initialize_store();
    assert_eq!(
        commit(&mut connection, &initial_candidate()),
        PrincipalProducerIncarnationCommitDisposition::Committed
    );
    assert_eq!(
        commit(&mut connection, &retirement_candidate()),
        PrincipalProducerIncarnationCommitDisposition::Committed
    );
    let retired_state: String = connection
        .query_row(
            "SELECT producer_state FROM pale_principal_producer_incarnation_head",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(retired_state, "retired");

    assert_eq!(
        commit(&mut connection, &successor_candidate()),
        PrincipalProducerIncarnationCommitDisposition::Committed
    );
    let counts: (i64, i64, i64) = connection
        .query_row(
            "SELECT \
             (SELECT COUNT(*) FROM pale_principal_producer_incarnation), \
             (SELECT COUNT(*) FROM pale_principal_producer_incarnation_retirement), \
             (SELECT COUNT(*) FROM pale_principal_producer_incarnation_head)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(counts, (2, 1, 1));
    let frontier: (String, i64, i64) = connection
        .query_row(
            "SELECT producer_state, \
             (SELECT checkpoint_epoch FROM pale_principal_trust_checkpoint_head), \
             (SELECT domain_sequence FROM pale_principal_control_checkpoint_head \
              WHERE checkpoint_domain_id = 'producer:producer-1') \
             FROM pale_principal_producer_incarnation_head",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(frontier, ("active".to_owned(), 4, 4));
}

#[test]
fn partial_append_and_corrupt_predecessor_require_recovery() {
    let mut partial = initialize_store();
    commit(&mut partial, &initial_candidate());
    let retirement = retirement_candidate();
    insert_record(&partial, retirement.binding(), &retirement.records()[0])
        .expect("partial proposed record");
    let mut store = SqlitePrincipalProducerIncarnationCustody {
        connection: &mut partial,
    };
    assert_eq!(
        store.compare_and_commit(&retirement).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );

    let mut corrupt = initialize_store();
    commit(&mut corrupt, &initial_candidate());
    corrupt
        .execute(
            "UPDATE pale_principal_trust_record SET canonical_record_jcs = X'7b7d' \
             WHERE record_ref = 'allocation-a'",
            [],
        )
        .unwrap();
    let mut store = SqlitePrincipalProducerIncarnationCustody {
        connection: &mut corrupt,
    };
    assert_eq!(
        store
            .compare_and_commit(&retirement_candidate())
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

#[test]
fn stale_complete_frontier_and_cross_producer_epoch_reuse_conflict() {
    let mut stale = initialize_store();
    commit(&mut stale, &initial_candidate());
    stale
        .execute(
            "UPDATE pale_principal_trust_checkpoint_head SET checkpoint_epoch = 9",
            [],
        )
        .unwrap();
    let mut store = SqlitePrincipalProducerIncarnationCustody {
        connection: &mut stale,
    };
    assert_eq!(
        store
            .compare_and_commit(&retirement_candidate())
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::CandidateConflict
    );

    let mut reused = initialize_store();
    commit(&mut reused, &initial_candidate());
    let allocation = record("other-allocation", 'd');
    insert_record(&reused, &binding(), &allocation).unwrap();
    reused
        .execute(
            "INSERT INTO pale_principal_producer_incarnation \
             (allocation_ref, allocation_sha256, origin_principal_id, producer_id, \
              producer_epoch, previous_allocation_ref, previous_allocation_sha256) \
             VALUES (?1, ?2, 'principal-1', 'other-producer', ?3, NULL, NULL)",
            params![
                allocation.coordinate().record_ref().as_str(),
                allocation.coordinate().record_sha256().as_str(),
                "c".repeat(32)
            ],
        )
        .unwrap();
    commit(&mut reused, &retirement_candidate());
    let mut store = SqlitePrincipalProducerIncarnationCustody {
        connection: &mut reused,
    };
    assert_eq!(
        store
            .compare_and_commit(&successor_candidate())
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::CandidateConflict
    );
}

#[test]
fn point_reads_preserve_retired_history_and_current_successor_state() {
    let mut connection = initialize_store();
    commit(&mut connection, &initial_candidate());
    commit(&mut connection, &retirement_candidate());
    commit(&mut connection, &successor_candidate());
    let mut store = SqlitePrincipalProducerIncarnationCustody {
        connection: &mut connection,
    };

    let retired_epoch = ProducerEpochV1::parse("a".repeat(32)).unwrap();
    let retired = store
        .read_by_epoch(&binding(), &retired_epoch)
        .expect("retired epoch read");
    let PrincipalProducerIncarnationReadObservation::Exact(retired) = retired else {
        panic!("retired epoch must remain exactly readable");
    };
    assert_eq!(
        retired.head().state(),
        ProducerIncarnationHeadStateV1::Retired
    );
    assert!(retired.retirement().is_some());

    let current = store
        .read_head(
            &binding(),
            &ProducerIdV1::parse("producer-1").expect("producer"),
        )
        .expect("current producer head read");
    let PrincipalProducerIncarnationReadObservation::Exact(current) = current else {
        panic!("successor epoch must be the exact current head");
    };
    assert_eq!(current.head().producer_epoch().as_str(), "c".repeat(32));
    assert_eq!(
        current.head().state(),
        ProducerIncarnationHeadStateV1::Active
    );
    assert!(current.retirement().is_none());
    assert_eq!(
        current.previous_allocation(),
        Some(&coordinate("allocation-a", 'a'))
    );

    let snapshot = PrincipalTrustSnapshot::new(binding(), trust_head(4), record("trust-4", '4'))
        .expect("current producer snapshot");
    let predecessor = store
        .read_by_allocation(&snapshot, &coordinate("allocation-a", 'a'))
        .expect("historical allocation read");
    let PrincipalProducerIncarnationReadObservation::Exact(predecessor) = predecessor else {
        panic!("historical allocation remains exactly readable");
    };
    assert_eq!(
        predecessor.head().state(),
        ProducerIncarnationHeadStateV1::Retired
    );
    assert!(predecessor.previous_allocation().is_none());

    let stale_snapshot =
        PrincipalTrustSnapshot::new(binding(), trust_head(3), record("trust-3", '3'))
            .expect("stale producer snapshot shape remains valid");
    assert_eq!(
        store
            .read_by_allocation(&stale_snapshot, &coordinate("allocation-a", 'a'))
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

#[test]
fn producer_head_page_is_fenced_by_the_exact_trust_checkpoint() {
    let mut connection = initialize_store();
    commit(&mut connection, &initial_candidate());
    let snapshot = pale_ports::PrincipalTrustReadPort::read_current_checkpoint(
        &mut crate::SqlitePrincipalTrustReader {
            connection: &mut connection,
        },
        &binding(),
        1024,
    )
    .expect("discover native trust head")
    .expect("stored trust head");
    let request =
        PrincipalProducerIncarnationHeadPageRead::new(snapshot, None, 1).expect("page request");
    let mut store = SqlitePrincipalProducerIncarnationCustody {
        connection: &mut connection,
    };
    let page = store
        .read_heads_page(&request)
        .expect("stable producer-head page");
    assert_eq!(page.through(), request.through());
    assert_eq!(page.entries().len(), 1);
    assert_eq!(
        page.entries()[0].cursor().producer_id().as_str(),
        "producer-1"
    );
    assert_eq!(
        page.boundary(),
        &PrincipalProducerIncarnationHeadPageBoundary::Complete
    );

    let stale = PrincipalProducerIncarnationHeadPageRead::new(
        PrincipalTrustSnapshot::new(binding(), trust_head(1), record("trust-1", '1'))
            .expect("stale snapshot shape remains valid"),
        None,
        1,
    )
    .expect("stale request shape remains valid");
    assert_eq!(
        store.read_heads_page(&stale).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

#[test]
fn producer_point_read_rejects_record_role_substitution() {
    let mut connection = initialize_store();
    commit(&mut connection, &initial_candidate());
    connection
        .execute(
            "UPDATE pale_principal_trust_record SET schema_id = ?1 \
             WHERE record_ref = 'allocation-a'",
            [PRODUCER_INCARNATION_RETIREMENT_V1_SCHEMA],
        )
        .unwrap();
    let mut store = SqlitePrincipalProducerIncarnationCustody {
        connection: &mut connection,
    };
    assert_eq!(
        store
            .read_by_epoch(&binding(), &ProducerEpochV1::parse("a".repeat(32)).unwrap(),)
            .unwrap(),
        PrincipalProducerIncarnationReadObservation::ConflictingOrPartial
    );

    let mut retired = initialize_store();
    commit(&mut retired, &initial_candidate());
    commit(&mut retired, &retirement_candidate());
    commit(&mut retired, &successor_candidate());
    retired
        .execute(
            "UPDATE pale_principal_producer_incarnation_retirement \
             SET producer_id = 'substituted-producer'",
            [],
        )
        .unwrap();
    let mut store = SqlitePrincipalProducerIncarnationCustody {
        connection: &mut retired,
    };
    assert_eq!(
        store
            .read_by_epoch(&binding(), &ProducerEpochV1::parse("a".repeat(32)).unwrap(),)
            .unwrap(),
        PrincipalProducerIncarnationReadObservation::ConflictingOrPartial
    );
}
