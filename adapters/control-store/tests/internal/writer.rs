//! Production `SQLite` resident-writer hard-frontier tests.

use pale_core::records::{
    ControlCheckpointDomainIdV1, ControlDomainEpochV1, ControlDomainSequenceV1,
    ControlIncarnationIdV1, ControlStoreIdV1, PrincipalResidentWriterFenceIdV1,
    PrincipalResidentWriterFenceStateV1, PrincipalResidentWriterGenerationV1,
    ResidentPrincipalIdV1, ResidentRecordRefV1, SchemaDigest, SchemaId,
};
use pale_ports::{
    ControlCheckpointHead, ControlRecord, ControlRecordBytes, ControlRecordCoordinate,
    ControlRecordDigest, ControlStoreErrorKind, PreparedPrincipalResidentWriterTransition,
    PrincipalResidentWriterCommitDisposition, PrincipalResidentWriterCustodyPort,
    PrincipalResidentWriterDatabaseObservation, PrincipalResidentWriterHead,
    PrincipalResidentWriterTransitionKind, PrincipalTrustStoreBinding,
};
use rusqlite::{params, Connection};

use crate::principal_trust::insert_record;
use crate::writer::SqlitePrincipalResidentWriterCustody;

const MIGRATION: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/principal-trust/0001_initial_v1.sql"
));
const CHECKPOINT_DOMAIN: &str = "principal-resident-writer";

fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn coordinate(reference: &str, byte: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(reference).unwrap(),
        ControlRecordDigest::parse(sha(byte)).unwrap(),
    )
}

fn record(coordinate: ControlRecordCoordinate, schema: &str, content: &str) -> ControlRecord {
    ControlRecord::new(
        coordinate,
        SchemaId::parse(schema).unwrap(),
        SchemaDigest::parse(sha('a')).unwrap(),
        ControlRecordBytes::new(content.as_bytes().to_vec()).unwrap(),
    )
}

fn checkpoint(sequence: u64, byte: char) -> ControlCheckpointHead {
    ControlCheckpointHead::new(
        ControlCheckpointDomainIdV1::parse(CHECKPOINT_DOMAIN).unwrap(),
        ControlDomainEpochV1::new(1).unwrap(),
        ControlDomainSequenceV1::new(sequence).unwrap(),
        pale_core::records::ControlTransactionSequenceV1::new(sequence + 20).unwrap(),
        coordinate(&format!("control-checkpoint-{sequence}"), byte),
    )
}

fn checkpoint_record(head: &ControlCheckpointHead) -> ControlRecord {
    record(
        head.coordinate().clone(),
        "pale/control-state-checkpoint.v1",
        &format!(r#"{{"checkpoint":{}}}"#, head.domain_sequence().get()),
    )
}

fn writer_head(
    reference: &str,
    byte: char,
    generation: u64,
    state: PrincipalResidentWriterFenceStateV1,
    checkpoint: ControlCheckpointHead,
) -> PrincipalResidentWriterHead {
    PrincipalResidentWriterHead::new(
        PrincipalResidentWriterFenceIdV1::parse("writer-lineage-1").unwrap(),
        ResidentPrincipalIdV1::parse("principal-1").unwrap(),
        PrincipalResidentWriterGenerationV1::new(generation).unwrap(),
        state,
        coordinate(reference, byte),
        checkpoint,
    )
}

fn writer_record(head: &PrincipalResidentWriterHead, content: &str) -> ControlRecord {
    record(
        head.coordinate().clone(),
        "pale/principal-resident-writer-fence.v1",
        content,
    )
}

fn binding(incarnation: &str) -> PrincipalTrustStoreBinding {
    PrincipalTrustStoreBinding::new(
        ControlStoreIdV1::parse("trust-store-1").unwrap(),
        ControlIncarnationIdV1::parse(incarnation).unwrap(),
        ResidentPrincipalIdV1::parse("principal-1").unwrap(),
    )
}

fn initial_candidate() -> PreparedPrincipalResidentWriterTransition {
    let checkpoint_1 = checkpoint(1, '1');
    let checkpoint_2 = checkpoint(2, '2');
    let active = writer_head(
        "writer-active-1",
        '3',
        1,
        PrincipalResidentWriterFenceStateV1::Active,
        checkpoint_2.clone(),
    );
    PreparedPrincipalResidentWriterTransition::new(
        binding("trust-incarnation-1"),
        PrincipalResidentWriterTransitionKind::InitialActive,
        (checkpoint_1.clone(), checkpoint_record(&checkpoint_1)),
        None,
        vec![
            writer_record(&active, r#"{"writer":"active-1"}"#),
            checkpoint_record(&checkpoint_2),
        ],
        active,
    )
    .unwrap()
}

fn retirement_candidate() -> PreparedPrincipalResidentWriterTransition {
    let checkpoint_2 = checkpoint(2, '2');
    let checkpoint_3 = checkpoint(3, '4');
    let active = writer_head(
        "writer-active-1",
        '3',
        1,
        PrincipalResidentWriterFenceStateV1::Active,
        checkpoint_2.clone(),
    );
    let retired = writer_head(
        "writer-retired-1",
        '5',
        1,
        PrincipalResidentWriterFenceStateV1::Retired,
        checkpoint_3.clone(),
    );
    PreparedPrincipalResidentWriterTransition::new(
        binding("trust-incarnation-1"),
        PrincipalResidentWriterTransitionKind::CleanRetirement,
        (checkpoint_2.clone(), checkpoint_record(&checkpoint_2)),
        Some((
            active.clone(),
            writer_record(&active, r#"{"writer":"active-1"}"#),
        )),
        vec![
            record(
                coordinate("writer-terminal-1", '6'),
                "pale/principal-resident-writer-terminal-receipt.v1",
                r#"{"writer":"terminal-1"}"#,
            ),
            writer_record(&retired, r#"{"writer":"retired-1"}"#),
            checkpoint_record(&checkpoint_3),
        ],
        retired,
    )
    .unwrap()
}

fn successor_candidate() -> PreparedPrincipalResidentWriterTransition {
    let checkpoint_3 = checkpoint(3, '4');
    let checkpoint_4 = checkpoint(4, '7');
    let retired = writer_head(
        "writer-retired-1",
        '5',
        1,
        PrincipalResidentWriterFenceStateV1::Retired,
        checkpoint_3.clone(),
    );
    let successor = writer_head(
        "writer-active-2",
        '8',
        2,
        PrincipalResidentWriterFenceStateV1::Active,
        checkpoint_4.clone(),
    );
    PreparedPrincipalResidentWriterTransition::new(
        binding("trust-incarnation-1"),
        PrincipalResidentWriterTransitionKind::CleanSuccessor,
        (checkpoint_3.clone(), checkpoint_record(&checkpoint_3)),
        Some((
            retired.clone(),
            writer_record(&retired, r#"{"writer":"retired-1"}"#),
        )),
        vec![
            writer_record(&successor, r#"{"writer":"active-2"}"#),
            checkpoint_record(&checkpoint_4),
        ],
        successor,
    )
    .unwrap()
}

fn initialize_store(connection: Connection) -> Connection {
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    connection.execute_batch(MIGRATION).unwrap();
    connection
        .execute(
            "INSERT INTO pale_control_store_header \
             (singleton, store_kind, logical_store_id, control_incarnation_id, store_schema_id, \
              store_schema_sha256, store_schema_version, storage_manifest_sha256, migration_head_sha256, logical_transaction_sequence) \
             VALUES (1, 'principal_trust', 'trust-store-1', 'trust-incarnation-1', \
                     'pale/control-store/principal-trust.v1', ?1, 1, ?2, ?3, 21)",
            params![sha('a'), sha('b'), sha('c')],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO pale_principal_trust_store_binding (singleton, principal_id) \
             VALUES (1, 'principal-1')",
            [],
        )
        .unwrap();
    let checkpoint = checkpoint(1, '1');
    let checkpoint_record = checkpoint_record(&checkpoint);
    insert_record(
        &connection,
        &binding("trust-incarnation-1"),
        &checkpoint_record,
    )
    .unwrap();
    connection
        .execute(
            "INSERT INTO pale_principal_control_checkpoint_head \
             (checkpoint_domain_id, domain_epoch, domain_sequence, \
              control_checkpoint_ref, control_checkpoint_sha256) \
             VALUES (?1, 1, 1, ?2, ?3)",
            params![
                CHECKPOINT_DOMAIN,
                checkpoint.coordinate().record_ref().as_str(),
                checkpoint.coordinate().record_sha256().as_str(),
            ],
        )
        .unwrap();
    connection
}

fn in_memory_store() -> Connection {
    initialize_store(Connection::open_in_memory().unwrap())
}

fn custody(connection: &mut Connection) -> SqlitePrincipalResidentWriterCustody<'_> {
    SqlitePrincipalResidentWriterCustody { connection }
}

#[test]
fn generation_one_commit_and_exact_replay_advance_both_heads() {
    let mut connection = in_memory_store();
    let candidate = initial_candidate();
    let mut store = custody(&mut connection);
    let coordinate = crate::principal_trust::store_coordinate(candidate.binding());
    assert_eq!(
        crate::store::transaction::read(store.connection, &coordinate)
            .unwrap()
            .get(),
        21
    );
    assert_eq!(
        store.observe_exact(&candidate).unwrap(),
        PrincipalResidentWriterDatabaseObservation::Predecessor
    );
    assert_eq!(
        store.compare_and_commit(&candidate).unwrap(),
        PrincipalResidentWriterCommitDisposition::Committed
    );
    assert_eq!(
        crate::store::transaction::read(store.connection, &coordinate)
            .unwrap()
            .get(),
        22
    );
    assert_eq!(
        store.observe_exact(&candidate).unwrap(),
        PrincipalResidentWriterDatabaseObservation::ExactSuccessor
    );
    assert_eq!(
        store.compare_and_commit(&candidate).unwrap(),
        PrincipalResidentWriterCommitDisposition::ExactReplay
    );
    assert_eq!(
        crate::store::transaction::read(store.connection, &coordinate)
            .unwrap()
            .get(),
        22
    );
}

#[test]
fn clean_retirement_and_successor_preserve_one_atomic_lineage() {
    let mut connection = in_memory_store();
    let initial = initial_candidate();
    let retirement = retirement_candidate();
    let successor = successor_candidate();
    let mut store = custody(&mut connection);
    store.compare_and_commit(&initial).unwrap();
    assert_eq!(
        store.compare_and_commit(&retirement).unwrap(),
        PrincipalResidentWriterCommitDisposition::Committed
    );
    assert_eq!(
        store.compare_and_commit(&successor).unwrap(),
        PrincipalResidentWriterCommitDisposition::Committed
    );
    assert_eq!(
        store.observe_exact(&successor).unwrap(),
        PrincipalResidentWriterDatabaseObservation::ExactSuccessor
    );
}

#[test]
fn stale_writer_compare_and_swap_is_a_candidate_conflict() {
    let mut connection = in_memory_store();
    let candidate = initial_candidate();
    let mut store = custody(&mut connection);
    store.compare_and_commit(&candidate).unwrap();
    store.compare_and_commit(&retirement_candidate()).unwrap();
    assert_eq!(
        store.compare_and_commit(&candidate).unwrap_err().kind(),
        ControlStoreErrorKind::CandidateConflict
    );
    let error = store
        .compare_and_commit(&candidate_with_alternate_successor())
        .unwrap_err();
    assert_eq!(error.kind(), ControlStoreErrorKind::CandidateConflict);
}

#[test]
fn committed_frontier_survives_restart_and_exact_read_back() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("principal-trust.sqlite3");
    let candidate = initial_candidate();
    {
        let mut connection = initialize_store(Connection::open(&path).unwrap());
        custody(&mut connection)
            .compare_and_commit(&candidate)
            .unwrap();
    }
    let mut reopened = Connection::open(&path).unwrap();
    reopened.pragma_update(None, "foreign_keys", true).unwrap();
    let changes_before = reopened.total_changes();
    assert_eq!(
        custody(&mut reopened).observe_exact(&candidate).unwrap(),
        PrincipalResidentWriterDatabaseObservation::ExactSuccessor
    );
    assert_eq!(reopened.total_changes(), changes_before);
}

#[test]
fn missing_predecessor_evidence_prevents_exact_replay() {
    let initial = initial_candidate();
    let mut missing_checkpoint = in_memory_store();
    custody(&mut missing_checkpoint)
        .compare_and_commit(&initial)
        .unwrap();
    missing_checkpoint
        .execute(
            "UPDATE pale_principal_trust_record SET canonical_record_jcs = X'00' \
             WHERE record_ref = ?1",
            [initial
                .expected_checkpoint()
                .coordinate()
                .record_ref()
                .as_str()],
        )
        .unwrap();
    let mut store = custody(&mut missing_checkpoint);
    assert_eq!(
        store.observe_exact(&initial).unwrap(),
        PrincipalResidentWriterDatabaseObservation::ConflictingOrPartial
    );
    assert_eq!(
        store.compare_and_commit(&initial).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );

    let retirement = retirement_candidate();
    let mut missing_writer = in_memory_store();
    custody(&mut missing_writer)
        .compare_and_commit(&initial)
        .unwrap();
    custody(&mut missing_writer)
        .compare_and_commit(&retirement)
        .unwrap();
    missing_writer
        .execute(
            "UPDATE pale_principal_trust_record SET canonical_record_jcs = X'00' \
             WHERE record_ref = ?1",
            [retirement
                .expected_head()
                .unwrap()
                .coordinate()
                .record_ref()
                .as_str()],
        )
        .unwrap();
    let mut store = custody(&mut missing_writer);
    assert_eq!(
        store.observe_exact(&retirement).unwrap(),
        PrincipalResidentWriterDatabaseObservation::ConflictingOrPartial
    );
    assert_eq!(
        store.compare_and_commit(&retirement).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

#[test]
fn cross_incarnation_candidate_is_rejected_without_writes() {
    let mut candidate = initial_candidate();
    candidate = PreparedPrincipalResidentWriterTransition::new(
        binding("trust-incarnation-2"),
        candidate.kind(),
        (
            candidate.expected_checkpoint().clone(),
            candidate.expected_checkpoint_record().clone(),
        ),
        candidate
            .expected_head()
            .cloned()
            .zip(candidate.expected_record().cloned()),
        candidate.records().to_vec(),
        candidate.next_head().clone(),
    )
    .unwrap();
    let mut connection = in_memory_store();
    let error = custody(&mut connection)
        .compare_and_commit(&candidate)
        .unwrap_err();
    assert_eq!(error.kind(), ControlStoreErrorKind::RecoveryRequired);
    assert_eq!(writer_row_count(&connection), 0);
}

#[test]
fn partial_successor_evidence_requires_recovery_and_injected_failure_rolls_back() {
    let candidate = initial_candidate();
    let mut partial = in_memory_store();
    insert_record(&partial, candidate.binding(), &candidate.records()[0]).unwrap();
    let mut partial_store = custody(&mut partial);
    assert_eq!(
        partial_store.observe_exact(&candidate).unwrap(),
        PrincipalResidentWriterDatabaseObservation::ConflictingOrPartial
    );
    assert_eq!(
        partial_store
            .compare_and_commit(&candidate)
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );

    let mut rolled_back = in_memory_store();
    rolled_back
        .execute_batch(
            "CREATE TEMP TRIGGER reject_writer_head BEFORE INSERT ON \
             pale_principal_resident_writer_head \
             BEGIN SELECT RAISE(ABORT, 'injected writer-head failure'); END;",
        )
        .unwrap();
    let mut store = custody(&mut rolled_back);
    assert_eq!(
        store.compare_and_commit(&candidate).unwrap_err().kind(),
        ControlStoreErrorKind::Storage
    );
    assert_eq!(
        store.observe_exact(&candidate).unwrap(),
        PrincipalResidentWriterDatabaseObservation::Predecessor
    );
}

#[test]
fn ambiguous_commit_requires_exact_non_mutating_read_back() {
    let candidate = initial_candidate();
    let mut connection = in_memory_store();
    connection
        .execute_batch(
            "CREATE TEMP TABLE commit_parent (id INTEGER PRIMARY KEY); \
             CREATE TEMP TABLE commit_child (parent_id INTEGER REFERENCES commit_parent(id) \
                 DEFERRABLE INITIALLY DEFERRED); \
             CREATE TEMP TRIGGER reject_writer_commit AFTER INSERT ON \
                 pale_principal_resident_writer_head \
             BEGIN INSERT INTO commit_child(parent_id) VALUES (1); END;",
        )
        .unwrap();
    let mut store = custody(&mut connection);
    assert_eq!(
        store.compare_and_commit(&candidate).unwrap_err().kind(),
        ControlStoreErrorKind::OutcomeUnknown
    );
    assert_eq!(
        store.observe_exact(&candidate).unwrap(),
        PrincipalResidentWriterDatabaseObservation::Predecessor
    );
}

fn candidate_with_alternate_successor() -> PreparedPrincipalResidentWriterTransition {
    let expected = checkpoint(1, '1');
    let next = checkpoint(2, '9');
    let head = writer_head(
        "writer-active-alternate",
        'b',
        1,
        PrincipalResidentWriterFenceStateV1::Active,
        next.clone(),
    );
    PreparedPrincipalResidentWriterTransition::new(
        binding("trust-incarnation-1"),
        PrincipalResidentWriterTransitionKind::InitialActive,
        (expected.clone(), checkpoint_record(&expected)),
        None,
        vec![
            writer_record(&head, r#"{"writer":"alternate"}"#),
            checkpoint_record(&next),
        ],
        head,
    )
    .unwrap()
}

fn writer_row_count(connection: &Connection) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM pale_principal_resident_writer_head",
            [],
            |row| row.get(0),
        )
        .unwrap()
}
