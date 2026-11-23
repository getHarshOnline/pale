//! Real `SQLite` qualification for bounded physical cold-start reads.

use super::*;
use pale_core::records::{
    ArtifactDigest, CheckpointProfileDigest, ControlCheckpointDomainIdV1, ControlDomainEpochV1,
    ControlDomainSequenceV1, ControlIncarnationIdV1, ControlStoreIdV1,
    ControlTransactionSequenceV1, DeploymentControlStoreGenesisIdV1, ResidentPrincipalIdV1,
    ResidentRecordRefV1, SchemaDigest, SchemaId, WorkspaceId, CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
    DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA, PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA,
    PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SCHEMA,
};
use pale_ports::{
    AccumulatorCheckpointBinding, AccumulatorCheckpointPhase, AccumulatorMutation,
    AccumulatorPosition, ControlCheckpointHead, ControlRecord, ControlRecordBytes,
    ControlRecordCoordinate, ControlRecordDigest, ControlTransactionPreparation,
    DeploymentControlStoreGenesisReadLimits, DeploymentControlStoreGenesisReadPort,
    DeploymentControlStoreGenesisRecords, DeploymentControlStoreGenesisSnapshot,
    OperationAccumulatorBatch, PrincipalResidentWriterReadPort,
    PrincipalResidentWriterRecordSetReadLimits, PrincipalResidentWriterRecordSetReadPort,
    PrincipalTrustReadPort, StateAccumulatorBatch, UnresolvedCustodyAccumulatorBatch,
};
use rusqlite::{params, Connection};

#[path = "cold_start/finalization.rs"]
mod finalization;

const PRINCIPAL_MIGRATION: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/principal-trust/0001_initial_v1.sql"
));
const DEPLOYMENT_MIGRATION: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/deployment-control/0001_initial_v1.sql"
));
const WRITER_BODY: &[u8] = br#"{"writer":"active"}"#;
const CHECKPOINT_BODY: &[u8] = br#"{"checkpoint":1}"#;
const SECOND_CHECKPOINT_BODY: &[u8] = br#"{"checkpoint":2}"#;
const RECEIPT_BODY: &[u8] = br#"{"genesis":"receipt"}"#;

fn hash(character: char) -> String {
    character.to_string().repeat(64)
}

fn coordinate(reference: &str, digest: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(reference).unwrap(),
        ControlRecordDigest::parse(hash(digest)).unwrap(),
    )
}

fn principal_binding() -> PrincipalTrustStoreBinding {
    PrincipalTrustStoreBinding::new(
        ControlStoreIdV1::parse("trust-store-1").unwrap(),
        ControlIncarnationIdV1::parse("trust-incarnation-1").unwrap(),
        ResidentPrincipalIdV1::parse("principal-1").unwrap(),
    )
}

fn deployment_binding() -> DeploymentControlStoreBinding {
    DeploymentControlStoreBinding::new(
        ControlStoreIdV1::parse("deployment-store-1").unwrap(),
        ControlIncarnationIdV1::parse("deployment-incarnation-1").unwrap(),
        ResidentPrincipalIdV1::parse("principal-1").unwrap(),
        WorkspaceId::parse("workspace-1").unwrap(),
        coordinate("deployment-binding", '2'),
        coordinate("checkpoint-signer", '3'),
        coordinate("runtime-authority", '4'),
    )
}

fn initialize_principal(path: &std::path::Path) -> Connection {
    let connection = Connection::open(path).unwrap();
    connection.execute_batch(PRINCIPAL_MIGRATION).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    connection.execute("INSERT INTO pale_control_store_header (singleton,store_kind,logical_store_id,control_incarnation_id,logical_transaction_sequence,store_schema_id,store_schema_sha256,store_schema_version,storage_manifest_sha256,migration_head_sha256) VALUES (1,'principal_trust','trust-store-1','trust-incarnation-1',2,'schema',?1,1,?2,?3)", params![hash('1'),hash('2'),hash('3')]).unwrap();
    connection
        .execute(
            "INSERT INTO pale_principal_trust_store_binding VALUES (1,'principal-1')",
            [],
        )
        .unwrap();
    connection
}

fn insert_principal_record(
    connection: &Connection,
    coordinate: &ControlRecordCoordinate,
    schema: &str,
    body: &[u8],
) {
    insert_principal_record_at(connection, coordinate, schema, body, 2);
}

fn insert_principal_record_at(
    connection: &Connection,
    coordinate: &ControlRecordCoordinate,
    schema: &str,
    body: &[u8],
    transaction: u64,
) {
    connection.execute("INSERT INTO pale_principal_trust_record (record_ref,record_sha256,schema_id,schema_sha256,principal_id,canonical_record_jcs,inserted_transaction_sequence) VALUES (?1,?2,?3,?4,'principal-1',?5,?6)", params![coordinate.record_ref().as_str(),coordinate.record_sha256().as_str(),schema,hash('f'),body,i64::try_from(transaction).unwrap()]).unwrap();
}

fn retain_writer(connection: &Connection) {
    let writer = coordinate("writer-active-1", 'a');
    let checkpoint = coordinate("writer-checkpoint-1", 'b');
    insert_principal_record(
        connection,
        &writer,
        PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA,
        WRITER_BODY,
    );
    insert_principal_record(
        connection,
        &checkpoint,
        CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
        CHECKPOINT_BODY,
    );
    connection.execute("INSERT INTO pale_principal_control_checkpoint_head (checkpoint_domain_id,domain_epoch,domain_sequence,control_checkpoint_ref,control_checkpoint_sha256) VALUES ('principal-resident-writer',1,1,?1,?2)", params![checkpoint.record_ref().as_str(),checkpoint.record_sha256().as_str()]).unwrap();
    connection.execute("INSERT INTO pale_principal_resident_writer_head (singleton,writer_fence_id,principal_id,writer_generation,writer_state,writer_fence_ref,writer_fence_sha256,checkpoint_domain_id,checkpoint_domain_epoch,checkpoint_domain_sequence,control_checkpoint_ref,control_checkpoint_sha256) VALUES (1,'writer-lineage-1','principal-1',1,'active',?1,?2,'principal-resident-writer',1,1,?3,?4)", params![writer.record_ref().as_str(),writer.record_sha256().as_str(),checkpoint.record_ref().as_str(),checkpoint.record_sha256().as_str()]).unwrap();
}

fn retain_clean_writer_history(connection: &Connection) {
    retain_writer(connection);
    let terminal = coordinate("writer-terminal-1", 'c');
    let retired = coordinate("writer-retired-1", 'd');
    let retirement_checkpoint = coordinate("writer-checkpoint-2", 'e');
    insert_principal_record_at(
        connection,
        &terminal,
        pale_core::records::PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SCHEMA,
        br#"{"terminal":1}"#,
        3,
    );
    insert_principal_record_at(
        connection,
        &retired,
        PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA,
        br#"{"writer":"retired"}"#,
        3,
    );
    insert_principal_record_at(
        connection,
        &retirement_checkpoint,
        CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
        br#"{"checkpoint":2}"#,
        3,
    );
    let transaction = connection.unchecked_transaction().unwrap();
    transaction.execute("UPDATE pale_principal_control_checkpoint_head SET domain_sequence=2,control_checkpoint_ref=?1,control_checkpoint_sha256=?2 WHERE checkpoint_domain_id='principal-resident-writer'", params![retirement_checkpoint.record_ref().as_str(),retirement_checkpoint.record_sha256().as_str()]).unwrap();
    transaction.execute("UPDATE pale_principal_resident_writer_head SET writer_state='retired',writer_fence_ref=?1,writer_fence_sha256=?2,checkpoint_domain_sequence=2,control_checkpoint_ref=?3,control_checkpoint_sha256=?4", params![retired.record_ref().as_str(),retired.record_sha256().as_str(),retirement_checkpoint.record_ref().as_str(),retirement_checkpoint.record_sha256().as_str()]).unwrap();
    transaction.commit().unwrap();

    let successor = coordinate("writer-active-2", '6');
    let successor_checkpoint = coordinate("writer-checkpoint-3", '7');
    insert_principal_record_at(
        connection,
        &successor,
        PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA,
        br#"{"writer":"active-2"}"#,
        4,
    );
    insert_principal_record_at(
        connection,
        &successor_checkpoint,
        CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
        br#"{"checkpoint":3}"#,
        4,
    );
    let transaction = connection.unchecked_transaction().unwrap();
    transaction.execute("UPDATE pale_principal_control_checkpoint_head SET domain_sequence=3,control_checkpoint_ref=?1,control_checkpoint_sha256=?2 WHERE checkpoint_domain_id='principal-resident-writer'", params![successor_checkpoint.record_ref().as_str(),successor_checkpoint.record_sha256().as_str()]).unwrap();
    transaction.execute("UPDATE pale_principal_resident_writer_head SET writer_generation=2,writer_state='active',writer_fence_ref=?1,writer_fence_sha256=?2,checkpoint_domain_sequence=3,control_checkpoint_ref=?3,control_checkpoint_sha256=?4", params![successor.record_ref().as_str(),successor.record_sha256().as_str(),successor_checkpoint.record_ref().as_str(),successor_checkpoint.record_sha256().as_str()]).unwrap();
    transaction
        .execute(
            "UPDATE pale_control_store_header SET logical_transaction_sequence=4",
            [],
        )
        .unwrap();
    transaction.commit().unwrap();
}

fn initialize_deployment(path: &std::path::Path) -> Connection {
    let connection = Connection::open(path).unwrap();
    connection.execute_batch(DEPLOYMENT_MIGRATION).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    connection.execute("INSERT INTO pale_control_store_header (singleton,store_kind,logical_store_id,control_incarnation_id,logical_transaction_sequence,store_schema_id,store_schema_sha256,store_schema_version,storage_manifest_sha256,migration_head_sha256) VALUES (1,'deployment_control','deployment-store-1','deployment-incarnation-1',2,'schema',?1,1,?2,?3)", params![hash('1'),hash('2'),hash('3')]).unwrap();
    connection.execute("INSERT INTO pale_deployment_control_store_binding VALUES (1,'workspace-1','deployment-binding',?1)",[hash('2')]).unwrap();
    connection
}

fn insert_deployment_record(
    connection: &Connection,
    coordinate: &ControlRecordCoordinate,
    schema: &str,
    body: &[u8],
) {
    connection.execute("INSERT INTO pale_deployment_control_record (record_ref,inserted_transaction_sequence,record_sha256,schema_id,schema_sha256,canonical_record_jcs) VALUES (?1,2,?2,?3,?4,?5)",params![coordinate.record_ref().as_str(),coordinate.record_sha256().as_str(),schema,hash('f'),body]).unwrap();
}

fn genesis_limits(maximum_record_bytes: usize) -> DeploymentControlStoreGenesisReadLimits {
    DeploymentControlStoreGenesisReadLimits::new(8, maximum_record_bytes, 32)
}

fn genesis_record(coordinate: ControlRecordCoordinate, schema: &str, body: &[u8]) -> ControlRecord {
    ControlRecord::new(
        coordinate,
        SchemaId::parse(schema).unwrap(),
        SchemaDigest::parse(hash('f')).unwrap(),
        ControlRecordBytes::new(body.to_vec()).unwrap(),
    )
}

fn retained_genesis_accumulator(
    binding: &DeploymentControlStoreBinding,
    checkpoint: ControlCheckpointHead,
    profile: &str,
    profile_digest: char,
    state_root: char,
    key: u8,
    value: u8,
) -> OperationAccumulatorBatch {
    let preparation = ControlTransactionPreparation::new(
        binding.store().clone(),
        ControlTransactionSequenceV1::new(1).unwrap(),
    )
    .unwrap();
    let accumulator_binding = AccumulatorCheckpointBinding::new(
        binding.store().clone(),
        ResidentRecordRefV1::parse(profile).unwrap(),
        CheckpointProfileDigest::parse(hash(profile_digest)).unwrap(),
        AccumulatorCheckpointPhase::Initial,
        checkpoint,
        preparation,
    )
    .unwrap();
    let zero = ArtifactDigest::parse(hash('0')).unwrap();
    OperationAccumulatorBatch::new(
        accumulator_binding,
        StateAccumulatorBatch::new(
            zero.clone(),
            ArtifactDigest::parse(hash(state_root)).unwrap(),
            vec![AccumulatorMutation::new(
                AccumulatorPosition::Leaf([key; 32]),
                None,
                Some(vec![value; 32]),
                32,
            )
            .unwrap()],
            1,
        )
        .unwrap(),
        UnresolvedCustodyAccumulatorBatch::new(zero.clone(), zero, Vec::new(), 1).unwrap(),
    )
    .unwrap()
}

fn retained_genesis_snapshot() -> DeploymentControlStoreGenesisSnapshot {
    let checkpoint_coordinate = coordinate("genesis-checkpoint", 'd');
    let checkpoint = ControlCheckpointHead::new(
        ControlCheckpointDomainIdV1::parse("operation").unwrap(),
        ControlDomainEpochV1::new(1).unwrap(),
        ControlDomainSequenceV1::new(1).unwrap(),
        ControlTransactionSequenceV1::new(2).unwrap(),
        checkpoint_coordinate.clone(),
    );
    let binding = deployment_binding();
    let accumulator =
        retained_genesis_accumulator(&binding, checkpoint.clone(), "profile", '7', '8', 1, 2);
    DeploymentControlStoreGenesisSnapshot::new(
        binding,
        DeploymentControlStoreGenesisIdV1::parse("genesis-1").unwrap(),
        AuthorityStoreCoordinate::new(
            ControlStoreIdV1::parse("trust-store-1").unwrap(),
            ControlIncarnationIdV1::parse("trust-incarnation-1").unwrap(),
        ),
        ControlTransactionSequenceV1::new(2).unwrap(),
        ArtifactDigest::parse(hash('e')).unwrap(),
        DeploymentControlStoreGenesisRecords::new(
            vec![genesis_record(
                checkpoint_coordinate,
                CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
                CHECKPOINT_BODY,
            )],
            genesis_record(
                coordinate("genesis-receipt", 'c'),
                DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA,
                RECEIPT_BODY,
            ),
        )
        .unwrap(),
        vec![checkpoint],
        vec![accumulator],
    )
    .unwrap()
}

fn retained_two_checkpoint_genesis_snapshot() -> DeploymentControlStoreGenesisSnapshot {
    let first_coordinate = coordinate("genesis-checkpoint", 'd');
    let second_coordinate = coordinate("genesis-checkpoint-2", '5');
    let first = ControlCheckpointHead::new(
        ControlCheckpointDomainIdV1::parse("operation").unwrap(),
        ControlDomainEpochV1::new(1).unwrap(),
        ControlDomainSequenceV1::new(1).unwrap(),
        ControlTransactionSequenceV1::new(2).unwrap(),
        first_coordinate.clone(),
    );
    let second = ControlCheckpointHead::new(
        ControlCheckpointDomainIdV1::parse("secondary").unwrap(),
        ControlDomainEpochV1::new(1).unwrap(),
        ControlDomainSequenceV1::new(1).unwrap(),
        ControlTransactionSequenceV1::new(2).unwrap(),
        second_coordinate.clone(),
    );
    let binding = deployment_binding();
    let accumulators = vec![
        retained_genesis_accumulator(&binding, first.clone(), "profile", '7', '8', 1, 2),
        retained_genesis_accumulator(&binding, second.clone(), "profile-2", '7', '8', 3, 4),
    ];
    DeploymentControlStoreGenesisSnapshot::new(
        binding,
        DeploymentControlStoreGenesisIdV1::parse("genesis-1").unwrap(),
        AuthorityStoreCoordinate::new(
            ControlStoreIdV1::parse("trust-store-1").unwrap(),
            ControlIncarnationIdV1::parse("trust-incarnation-1").unwrap(),
        ),
        ControlTransactionSequenceV1::new(2).unwrap(),
        ArtifactDigest::parse(hash('e')).unwrap(),
        DeploymentControlStoreGenesisRecords::new(
            vec![
                genesis_record(
                    first_coordinate,
                    CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
                    CHECKPOINT_BODY,
                ),
                genesis_record(
                    second_coordinate,
                    CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
                    SECOND_CHECKPOINT_BODY,
                ),
            ],
            genesis_record(
                coordinate("genesis-receipt", 'c'),
                DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA,
                RECEIPT_BODY,
            ),
        )
        .unwrap(),
        vec![first, second],
        accumulators,
    )
    .unwrap()
}

fn retain_genesis(connection: &Connection) {
    let receipt = coordinate("genesis-receipt", 'c');
    let checkpoint = coordinate("genesis-checkpoint", 'd');
    insert_deployment_record(
        connection,
        &receipt,
        DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA,
        RECEIPT_BODY,
    );
    insert_deployment_record(
        connection,
        &checkpoint,
        CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
        CHECKPOINT_BODY,
    );
    connection.execute("INSERT INTO pale_deployment_control_store_genesis (singleton,genesis_id,principal_id,workspace_id,deployment_binding_ref,deployment_binding_sha256,checkpoint_signer_ref,checkpoint_signer_sha256,runtime_authority_ref,runtime_authority_sha256,finalization_store_id,finalization_incarnation_id,receipt_ref,receipt_sha256,receipt_body_sha256,transaction_sequence,checkpoint_count,candidate_identity_sha256) VALUES (1,'genesis-1','principal-1','workspace-1','deployment-binding',?1,'checkpoint-signer',?2,'runtime-authority',?3,'trust-store-1','trust-incarnation-1',?4,?5,?6,2,1,?7)",params![hash('2'),hash('3'),hash('4'),receipt.record_ref().as_str(),receipt.record_sha256().as_str(),pale_core::sha256_hex(RECEIPT_BODY),hash('e')]).unwrap();
    connection.execute("INSERT INTO pale_deployment_control_store_genesis_checkpoint (genesis_id,checkpoint_ordinal,checkpoint_domain_id,domain_epoch,domain_sequence,checkpoint_ref,checkpoint_sha256,checkpoint_body_sha256,checkpoint_profile_ref,checkpoint_profile_sha256,state_root_sha256,unresolved_root_sha256,mutation_count) VALUES ('genesis-1',0,'operation',1,1,?1,?2,?3,'profile',?4,?5,?6,1)",params![checkpoint.record_ref().as_str(),checkpoint.record_sha256().as_str(),pale_core::sha256_hex(CHECKPOINT_BODY),hash('7'),hash('8'),hash('0')]).unwrap();
    let mut position = vec![1_u8; 33];
    position[0] = 0;
    connection.execute("INSERT INTO pale_deployment_control_store_genesis_mutation (genesis_id,checkpoint_ordinal,tree_kind,position_kind,position_key,successor_node_bytes) VALUES ('genesis-1',0,'state','leaf',?1,?2)",params![position,vec![2_u8;32]]).unwrap();
    let identity =
        crate::deployment_initialization::genesis_snapshot_identity(&retained_genesis_snapshot());
    connection
        .execute(
            "UPDATE pale_deployment_control_store_genesis SET candidate_identity_sha256=?1",
            [identity],
        )
        .unwrap();
}

fn retain_second_genesis_checkpoint(
    connection: &Connection,
    first_domain: &str,
    second_domain: &str,
) {
    let checkpoint = coordinate("genesis-checkpoint-2", '5');
    insert_deployment_record(
        connection,
        &checkpoint,
        CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
        SECOND_CHECKPOINT_BODY,
    );
    connection
        .execute(
            "UPDATE pale_deployment_control_store_genesis SET checkpoint_count=2",
            [],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE pale_deployment_control_store_genesis_checkpoint \
             SET checkpoint_domain_id=?1 WHERE checkpoint_ordinal=0",
            [first_domain],
        )
        .unwrap();
    connection.execute("INSERT INTO pale_deployment_control_store_genesis_checkpoint (genesis_id,checkpoint_ordinal,checkpoint_domain_id,domain_epoch,domain_sequence,checkpoint_ref,checkpoint_sha256,checkpoint_body_sha256,checkpoint_profile_ref,checkpoint_profile_sha256,state_root_sha256,unresolved_root_sha256,mutation_count) VALUES ('genesis-1',1,?1,1,1,?2,?3,?4,'profile-2',?5,?6,?7,1)",params![second_domain,checkpoint.record_ref().as_str(),checkpoint.record_sha256().as_str(),pale_core::sha256_hex(SECOND_CHECKPOINT_BODY),hash('7'),hash('8'),hash('0')]).unwrap();
    let mut position = vec![3_u8; 33];
    position[0] = 0;
    connection.execute("INSERT INTO pale_deployment_control_store_genesis_mutation (genesis_id,checkpoint_ordinal,tree_kind,position_kind,position_key,successor_node_bytes) VALUES ('genesis-1',1,'state','leaf',?1,?2)",params![position,vec![4_u8;32]]).unwrap();
}

fn error_kind<T: std::fmt::Debug>(
    result: Result<T, ControlStoreError>,
    expected: ControlStoreErrorKind,
) {
    assert_eq!(result.unwrap_err().kind(), expected);
}

#[test]
fn writer_and_exact_trust_record_survive_a_real_reopen() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("principal.sqlite");
    let connection = initialize_principal(&path);
    retain_writer(&connection);
    drop(connection);

    let mut connection = Connection::open(&path).unwrap();
    let maximum = WRITER_BODY.len() + CHECKPOINT_BODY.len();
    let snapshot = SqlitePrincipalColdStartReader {
        connection: &mut connection,
    }
    .read_current_writer(&principal_binding(), maximum)
    .unwrap()
    .unwrap();
    assert_eq!(snapshot.head().writer_generation().get(), 1);
    assert_eq!(
        snapshot.writer_record().canonical_record_jcs().as_slice(),
        WRITER_BODY
    );
    assert_eq!(
        snapshot
            .checkpoint_record()
            .canonical_record_jcs()
            .as_slice(),
        CHECKPOINT_BODY
    );
    assert!(connection.is_autocommit());

    let exact = coordinate("writer-active-1", 'a');
    let record = crate::SqlitePrincipalTrustReader {
        connection: &mut connection,
    }
    .read_record(&principal_binding(), &exact, WRITER_BODY.len())
    .unwrap()
    .unwrap();
    assert_eq!(record.coordinate(), &exact);
    assert_eq!(record.canonical_record_jcs().as_slice(), WRITER_BODY);
}

#[test]
fn writer_record_set_preserves_initial_and_clean_history_across_reopen() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("principal.sqlite");
    let connection = initialize_principal(&path);
    retain_clean_writer_history(&connection);
    drop(connection);

    let mut connection = Connection::open(&path).unwrap();
    let snapshot = SqlitePrincipalColdStartReader {
        connection: &mut connection,
    }
    .read_writer_record_set(
        &principal_binding(),
        PrincipalResidentWriterRecordSetReadLimits::new(4, 3, 4096),
    )
    .unwrap()
    .unwrap();
    assert_eq!(snapshot.head().writer_generation().get(), 2);
    assert_eq!(snapshot.writer_records().len(), 4);
    assert_eq!(snapshot.checkpoint_records().len(), 3);
    assert_eq!(snapshot.observed_through().get(), 4);
    assert_eq!(
        snapshot.current_writer_record().record().coordinate(),
        snapshot.head().coordinate()
    );
    assert_eq!(
        snapshot.current_checkpoint_record().record().coordinate(),
        snapshot.head().checkpoint().coordinate()
    );
    assert_eq!(
        snapshot
            .writer_records()
            .iter()
            .map(|record| record.inserted_in().get())
            .collect::<Vec<_>>(),
        vec![2, 3, 3, 4]
    );
    assert!(connection.is_autocommit());
}

#[test]
fn writer_record_set_enforces_counts_bytes_and_physical_closure() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("principal.sqlite");
    let mut connection = initialize_principal(&path);
    retain_clean_writer_history(&connection);

    for limits in [
        PrincipalResidentWriterRecordSetReadLimits::new(3, 3, 4096),
        PrincipalResidentWriterRecordSetReadLimits::new(4, 2, 4096),
        PrincipalResidentWriterRecordSetReadLimits::new(4, 3, 1),
    ] {
        error_kind(
            SqlitePrincipalColdStartReader {
                connection: &mut connection,
            }
            .read_writer_record_set(&principal_binding(), limits),
            ControlStoreErrorKind::BoundExceeded,
        );
    }

    connection
        .execute(
            "DELETE FROM pale_principal_trust_record WHERE record_ref='writer-checkpoint-2'",
            [],
        )
        .unwrap();
    error_kind(
        SqlitePrincipalColdStartReader {
            connection: &mut connection,
        }
        .read_writer_record_set(
            &principal_binding(),
            PrincipalResidentWriterRecordSetReadLimits::new(8, 8, 4096),
        ),
        ControlStoreErrorKind::RecoveryRequired,
    );
}

#[test]
fn writer_record_set_rejects_unbound_history_and_material_without_a_head() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("principal.sqlite");
    let mut connection = initialize_principal(&path);
    retain_writer(&connection);
    connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    connection.execute("UPDATE pale_principal_trust_record SET principal_id='other-principal' WHERE record_ref='writer-active-1'", []).unwrap();
    error_kind(
        SqlitePrincipalColdStartReader {
            connection: &mut connection,
        }
        .read_writer_record_set(
            &principal_binding(),
            PrincipalResidentWriterRecordSetReadLimits::new(8, 8, 4096),
        ),
        ControlStoreErrorKind::RecoveryRequired,
    );

    connection
        .execute("DELETE FROM pale_principal_resident_writer_head", [])
        .unwrap();
    error_kind(
        SqlitePrincipalColdStartReader {
            connection: &mut connection,
        }
        .read_writer_record_set(
            &principal_binding(),
            PrincipalResidentWriterRecordSetReadLimits::new(8, 8, 4096),
        ),
        ControlStoreErrorKind::RecoveryRequired,
    );
}

#[test]
fn writer_record_set_preserves_surplus_writer_evidence_for_later_admission() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("principal.sqlite");
    let mut connection = initialize_principal(&path);
    retain_writer(&connection);
    let takeover = coordinate("writer-takeover-surplus", '5');
    insert_principal_record(
        &connection,
        &takeover,
        PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SCHEMA,
        br#"{"takeover":"unadmitted"}"#,
    );

    let snapshot = SqlitePrincipalColdStartReader {
        connection: &mut connection,
    }
    .read_writer_record_set(
        &principal_binding(),
        PrincipalResidentWriterRecordSetReadLimits::new(3, 1, 4096),
    )
    .unwrap()
    .unwrap();
    assert_eq!(snapshot.writer_records().len(), 2);
    assert!(snapshot
        .writer_records()
        .iter()
        .any(|stored| stored.record().coordinate() == &takeover));
}

#[test]
fn writer_record_set_rejects_future_and_wrongly_bound_checkpoint_rows() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("principal.sqlite");
    let mut connection = initialize_principal(&path);
    retain_writer(&connection);
    let future_writer = coordinate("writer-future", '6');
    let future_checkpoint = coordinate("writer-checkpoint-future", '7');
    insert_principal_record_at(
        &connection,
        &future_writer,
        PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA,
        br#"{"writer":"future"}"#,
        3,
    );
    insert_principal_record_at(
        &connection,
        &future_checkpoint,
        CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
        br#"{"checkpoint":"future"}"#,
        3,
    );
    error_kind(
        SqlitePrincipalColdStartReader {
            connection: &mut connection,
        }
        .read_writer_record_set(
            &principal_binding(),
            PrincipalResidentWriterRecordSetReadLimits::new(8, 8, 4096),
        ),
        ControlStoreErrorKind::RecoveryRequired,
    );

    connection
        .execute(
            "DELETE FROM pale_principal_trust_record \
             WHERE record_ref IN ('writer-future','writer-checkpoint-future')",
            [],
        )
        .unwrap();
    connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    connection
        .execute(
            "UPDATE pale_principal_trust_record SET principal_id='other-principal' \
             WHERE record_ref='writer-checkpoint-1'",
            [],
        )
        .unwrap();
    error_kind(
        SqlitePrincipalColdStartReader {
            connection: &mut connection,
        }
        .read_writer_record_set(
            &principal_binding(),
            PrincipalResidentWriterRecordSetReadLimits::new(8, 8, 4096),
        ),
        ControlStoreErrorKind::RecoveryRequired,
    );
}

#[test]
fn exact_trust_record_distinguishes_absence_conflict_and_bound() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("principal.sqlite");
    let mut connection = initialize_principal(&path);
    retain_writer(&connection);

    let exact = coordinate("writer-active-1", 'a');
    error_kind(
        crate::SqlitePrincipalTrustReader {
            connection: &mut connection,
        }
        .read_record(&principal_binding(), &exact, WRITER_BODY.len() - 1),
        ControlStoreErrorKind::BoundExceeded,
    );
    let conflicting = coordinate("writer-active-1", 'f');
    error_kind(
        crate::SqlitePrincipalTrustReader {
            connection: &mut connection,
        }
        .read_record(&principal_binding(), &conflicting, WRITER_BODY.len()),
        ControlStoreErrorKind::RecoveryRequired,
    );
    let absent = coordinate("absent", '0');
    assert!(crate::SqlitePrincipalTrustReader {
        connection: &mut connection
    }
    .read_record(&principal_binding(), &absent, 0)
    .unwrap()
    .is_none());

    connection
        .execute(
            "UPDATE pale_principal_trust_record SET inserted_transaction_sequence=3 \
             WHERE record_ref='writer-active-1'",
            [],
        )
        .unwrap();
    error_kind(
        crate::SqlitePrincipalTrustReader {
            connection: &mut connection,
        }
        .read_record(&principal_binding(), &exact, WRITER_BODY.len()),
        ControlStoreErrorKind::RecoveryRequired,
    );
}

#[test]
fn principal_reads_distinguish_absence_partial_conflict_and_bounds() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("principal.sqlite");
    let mut connection = initialize_principal(&path);
    assert!(SqlitePrincipalColdStartReader {
        connection: &mut connection
    }
    .read_current_writer(&principal_binding(), 0)
    .unwrap()
    .is_none());

    let writer = coordinate("writer-active-1", 'a');
    insert_principal_record(
        &connection,
        &writer,
        PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA,
        WRITER_BODY,
    );
    error_kind(
        SqlitePrincipalColdStartReader {
            connection: &mut connection,
        }
        .read_current_writer(&principal_binding(), 1024),
        ControlStoreErrorKind::RecoveryRequired,
    );
    connection
        .execute(
            "DELETE FROM pale_principal_trust_record WHERE record_ref='writer-active-1'",
            [],
        )
        .unwrap();
    retain_writer(&connection);
    error_kind(
        SqlitePrincipalColdStartReader {
            connection: &mut connection,
        }
        .read_current_writer(
            &principal_binding(),
            WRITER_BODY.len() + CHECKPOINT_BODY.len() - 1,
        ),
        ControlStoreErrorKind::BoundExceeded,
    );
    connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    connection
        .execute(
            "UPDATE pale_principal_resident_writer_head SET writer_fence_sha256=?1",
            [hash('f')],
        )
        .unwrap();
    error_kind(
        SqlitePrincipalColdStartReader {
            connection: &mut connection,
        }
        .read_current_writer(&principal_binding(), 1024),
        ControlStoreErrorKind::RecoveryRequired,
    );
    let absent = coordinate("absent", '0');
    assert!(crate::SqlitePrincipalTrustReader {
        connection: &mut connection
    }
    .read_record(&principal_binding(), &absent, 0)
    .unwrap()
    .is_none());
}

#[test]
fn principal_writer_rejects_a_substituted_checkpoint_domain() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("principal.sqlite");
    let mut connection = initialize_principal(&path);
    retain_writer(&connection);
    connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    connection
        .execute(
            "UPDATE pale_principal_control_checkpoint_head \
             SET checkpoint_domain_id='another-domain'",
            [],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE pale_principal_resident_writer_head \
             SET checkpoint_domain_id='another-domain'",
            [],
        )
        .unwrap();

    error_kind(
        SqlitePrincipalColdStartReader {
            connection: &mut connection,
        }
        .read_current_writer(&principal_binding(), 1024),
        ControlStoreErrorKind::RecoveryRequired,
    );
}

#[test]
fn deployment_genesis_survives_reopen_and_preserves_exact_closure() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("deployment.sqlite");
    let connection = initialize_deployment(&path);
    retain_genesis(&connection);
    drop(connection);

    let mut connection = Connection::open(&path).unwrap();
    let maximum = RECEIPT_BODY.len() + CHECKPOINT_BODY.len();
    let snapshot = SqliteDeploymentGenesisReader {
        connection: &mut connection,
    }
    .read_genesis(&deployment_binding(), genesis_limits(maximum))
    .unwrap()
    .unwrap();
    assert_eq!(snapshot.genesis_id().as_str(), "genesis-1");
    assert_eq!(snapshot.committed_transaction().get(), 2);
    assert_eq!(
        snapshot.candidate_identity_sha256().as_str(),
        crate::deployment_initialization::genesis_snapshot_identity(&retained_genesis_snapshot())
    );
    assert_eq!(snapshot.checkpoint_heads().len(), 1);
    assert_eq!(snapshot.records().checkpoints().len(), 1);
    assert_eq!(snapshot.accumulators().len(), 1);
    assert_eq!(snapshot.accumulators()[0].state().mutations().len(), 1);
    assert_eq!(
        snapshot
            .records()
            .receipt()
            .canonical_record_jcs()
            .as_slice(),
        RECEIPT_BODY
    );
    assert_eq!(
        snapshot.finalization_trust_store().store_id().as_str(),
        "trust-store-1"
    );
    assert!(connection.is_autocommit());
}

#[test]
fn deployment_genesis_preserves_every_ordered_initial_batch() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("deployment.sqlite");
    let mut connection = initialize_deployment(&path);
    retain_genesis(&connection);
    retain_second_genesis_checkpoint(&connection, "operation", "secondary");
    let expected = retained_two_checkpoint_genesis_snapshot();
    connection
        .execute(
            "UPDATE pale_deployment_control_store_genesis SET candidate_identity_sha256=?1",
            [crate::deployment_initialization::genesis_snapshot_identity(
                &expected,
            )],
        )
        .unwrap();

    let snapshot = SqliteDeploymentGenesisReader {
        connection: &mut connection,
    }
    .read_genesis(&deployment_binding(), genesis_limits(2048))
    .unwrap()
    .unwrap();

    assert_eq!(snapshot.checkpoint_heads(), expected.checkpoint_heads());
    assert_eq!(snapshot.accumulators(), expected.accumulators());
    assert_eq!(snapshot.records(), expected.records());
}

#[test]
fn deployment_genesis_rejects_changed_identity_roots_and_mutations() {
    let directory = crate::test_support::private_tempdir();
    for (name, statement, value) in [
        (
            "identity",
            "UPDATE pale_deployment_control_store_genesis SET candidate_identity_sha256=?1",
            hash('a'),
        ),
        (
            "root",
            "UPDATE pale_deployment_control_store_genesis_checkpoint SET state_root_sha256=?1",
            hash('a'),
        ),
    ] {
        let path = directory.path().join(format!("{name}.sqlite"));
        let mut connection = initialize_deployment(&path);
        retain_genesis(&connection);
        connection.execute(statement, [value]).unwrap();
        error_kind(
            SqliteDeploymentGenesisReader {
                connection: &mut connection,
            }
            .read_genesis(&deployment_binding(), genesis_limits(1024)),
            ControlStoreErrorKind::RecoveryRequired,
        );
    }

    let mutation_path = directory.path().join("mutation.sqlite");
    let mut mutation = initialize_deployment(&mutation_path);
    retain_genesis(&mutation);
    mutation
        .execute(
            "UPDATE pale_deployment_control_store_genesis_mutation \
             SET successor_node_bytes=?1",
            [vec![3_u8; 32]],
        )
        .unwrap();
    error_kind(
        SqliteDeploymentGenesisReader {
            connection: &mut mutation,
        }
        .read_genesis(&deployment_binding(), genesis_limits(1024)),
        ControlStoreErrorKind::RecoveryRequired,
    );

    let position_path = directory.path().join("position.sqlite");
    let mut position = initialize_deployment(&position_path);
    retain_genesis(&position);
    position
        .execute(
            "UPDATE pale_deployment_control_store_genesis_mutation \
             SET position_key=?1",
            [vec![4_u8; 33]],
        )
        .unwrap();
    error_kind(
        SqliteDeploymentGenesisReader {
            connection: &mut position,
        }
        .read_genesis(&deployment_binding(), genesis_limits(1024)),
        ControlStoreErrorKind::RecoveryRequired,
    );
}

#[test]
fn deployment_genesis_applies_whole_closure_limits_before_copying() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("deployment.sqlite");
    let mut connection = initialize_deployment(&path);
    retain_genesis(&connection);
    for limits in [
        DeploymentControlStoreGenesisReadLimits::new(0, 1024, 32),
        DeploymentControlStoreGenesisReadLimits::new(8, 1024, 0),
    ] {
        error_kind(
            SqliteDeploymentGenesisReader {
                connection: &mut connection,
            }
            .read_genesis(&deployment_binding(), limits),
            ControlStoreErrorKind::BoundExceeded,
        );
    }
}

#[test]
fn deployment_genesis_rejects_wrong_transaction_and_surplus_batches() {
    let directory = crate::test_support::private_tempdir();
    let transaction_path = directory.path().join("transaction.sqlite");
    let mut wrong_transaction = initialize_deployment(&transaction_path);
    retain_genesis(&wrong_transaction);
    wrong_transaction
        .execute(
            "UPDATE pale_deployment_control_record SET inserted_transaction_sequence=1 \
             WHERE record_ref='genesis-checkpoint'",
            [],
        )
        .unwrap();
    error_kind(
        SqliteDeploymentGenesisReader {
            connection: &mut wrong_transaction,
        }
        .read_genesis(&deployment_binding(), genesis_limits(1024)),
        ControlStoreErrorKind::RecoveryRequired,
    );

    let surplus_path = directory.path().join("surplus.sqlite");
    let mut surplus = initialize_deployment(&surplus_path);
    retain_genesis(&surplus);
    retain_second_genesis_checkpoint(&surplus, "operation", "surplus");
    surplus
        .execute(
            "UPDATE pale_deployment_control_store_genesis SET checkpoint_count=1",
            [],
        )
        .unwrap();
    error_kind(
        SqliteDeploymentGenesisReader {
            connection: &mut surplus,
        }
        .read_genesis(&deployment_binding(), genesis_limits(2048)),
        ControlStoreErrorKind::RecoveryRequired,
    );
}

#[test]
fn deployment_genesis_rejects_close_beyond_store_frontier() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("future-genesis.sqlite");
    let mut connection = initialize_deployment(&path);
    retain_genesis(&connection);
    connection
        .execute(
            "UPDATE pale_deployment_control_store_genesis SET transaction_sequence=3",
            [],
        )
        .unwrap();

    error_kind(
        SqliteDeploymentGenesisReader {
            connection: &mut connection,
        }
        .read_genesis(&deployment_binding(), genesis_limits(1024)),
        ControlStoreErrorKind::RecoveryRequired,
    );
}

#[test]
fn deployment_genesis_distinguishes_absence_partial_conflict_and_bounds() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("deployment.sqlite");
    let mut connection = initialize_deployment(&path);
    assert!(SqliteDeploymentGenesisReader {
        connection: &mut connection
    }
    .read_genesis(&deployment_binding(), genesis_limits(0))
    .unwrap()
    .is_none());
    let receipt = coordinate("genesis-receipt", 'c');
    insert_deployment_record(
        &connection,
        &receipt,
        DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA,
        RECEIPT_BODY,
    );
    error_kind(
        SqliteDeploymentGenesisReader {
            connection: &mut connection,
        }
        .read_genesis(&deployment_binding(), genesis_limits(1024)),
        ControlStoreErrorKind::RecoveryRequired,
    );
    connection
        .execute("DELETE FROM pale_deployment_control_record", [])
        .unwrap();
    retain_genesis(&connection);
    error_kind(
        SqliteDeploymentGenesisReader {
            connection: &mut connection,
        }
        .read_genesis(
            &deployment_binding(),
            genesis_limits(RECEIPT_BODY.len() + CHECKPOINT_BODY.len() - 1),
        ),
        ControlStoreErrorKind::BoundExceeded,
    );
    connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    connection
        .execute(
            "DELETE FROM pale_deployment_control_store_genesis_checkpoint",
            [],
        )
        .unwrap();
    error_kind(
        SqliteDeploymentGenesisReader {
            connection: &mut connection,
        }
        .read_genesis(&deployment_binding(), genesis_limits(1024)),
        ControlStoreErrorKind::RecoveryRequired,
    );
}

#[test]
fn deployment_genesis_rejects_noncanonical_or_noninitial_checkpoints() {
    let directory = crate::test_support::private_tempdir();
    let order_path = directory.path().join("unordered.sqlite");
    let mut unordered = initialize_deployment(&order_path);
    retain_genesis(&unordered);
    retain_second_genesis_checkpoint(&unordered, "z-domain", "a-domain");
    error_kind(
        SqliteDeploymentGenesisReader {
            connection: &mut unordered,
        }
        .read_genesis(&deployment_binding(), genesis_limits(1024)),
        ControlStoreErrorKind::RecoveryRequired,
    );

    let epoch_path = directory.path().join("noninitial.sqlite");
    let mut noninitial = initialize_deployment(&epoch_path);
    retain_genesis(&noninitial);
    noninitial
        .execute(
            "UPDATE pale_deployment_control_store_genesis_checkpoint SET domain_epoch=2",
            [],
        )
        .unwrap();
    error_kind(
        SqliteDeploymentGenesisReader {
            connection: &mut noninitial,
        }
        .read_genesis(&deployment_binding(), genesis_limits(1024)),
        ControlStoreErrorKind::RecoveryRequired,
    );
}

#[test]
fn deployment_genesis_rejects_store_reuse_and_unowned_rows() {
    let directory = crate::test_support::private_tempdir();
    let store_path = directory.path().join("same-store.sqlite");
    let mut same_store = initialize_deployment(&store_path);
    retain_genesis(&same_store);
    same_store
        .execute(
            "UPDATE pale_deployment_control_store_genesis \
             SET finalization_store_id='deployment-store-1', \
                 finalization_incarnation_id='deployment-incarnation-1'",
            [],
        )
        .unwrap();
    error_kind(
        SqliteDeploymentGenesisReader {
            connection: &mut same_store,
        }
        .read_genesis(&deployment_binding(), genesis_limits(1024)),
        ControlStoreErrorKind::RecoveryRequired,
    );

    let row_path = directory.path().join("unowned-row.sqlite");
    let mut unowned = initialize_deployment(&row_path);
    retain_genesis(&unowned);
    unowned.pragma_update(None, "foreign_keys", false).unwrap();
    unowned.execute("INSERT INTO pale_deployment_control_store_genesis_mutation (genesis_id,checkpoint_ordinal,tree_kind,position_kind,position_key,successor_node_bytes) VALUES ('unowned-genesis',7,'state','leaf',?1,?2)",params![vec![5_u8;33],vec![6_u8;32]]).unwrap();
    error_kind(
        SqliteDeploymentGenesisReader {
            connection: &mut unowned,
        }
        .read_genesis(&deployment_binding(), genesis_limits(1024)),
        ControlStoreErrorKind::RecoveryRequired,
    );
}

#[test]
fn native_read_failures_remain_storage_failures() {
    let directory = crate::test_support::private_tempdir();
    let principal_path = directory.path().join("principal.sqlite");
    let mut principal = initialize_principal(&principal_path);
    principal
        .execute("DROP TABLE pale_principal_resident_writer_head", [])
        .unwrap();
    error_kind(
        SqlitePrincipalColdStartReader {
            connection: &mut principal,
        }
        .read_current_writer(&principal_binding(), 1024),
        ControlStoreErrorKind::Storage,
    );
    error_kind(
        SqlitePrincipalColdStartReader {
            connection: &mut principal,
        }
        .read_writer_record_set(
            &principal_binding(),
            PrincipalResidentWriterRecordSetReadLimits::new(8, 8, 4096),
        ),
        ControlStoreErrorKind::Storage,
    );
    principal
        .execute("DROP TABLE pale_principal_trust_record", [])
        .unwrap();
    error_kind(
        crate::SqlitePrincipalTrustReader {
            connection: &mut principal,
        }
        .read_record(&principal_binding(), &coordinate("missing", '0'), 1024),
        ControlStoreErrorKind::Storage,
    );

    let deployment_path = directory.path().join("deployment.sqlite");
    let mut deployment = initialize_deployment(&deployment_path);
    deployment
        .execute("DROP TABLE pale_deployment_control_store_genesis", [])
        .unwrap();
    error_kind(
        SqliteDeploymentGenesisReader {
            connection: &mut deployment,
        }
        .read_genesis(&deployment_binding(), genesis_limits(1024)),
        ControlStoreErrorKind::Storage,
    );
}
