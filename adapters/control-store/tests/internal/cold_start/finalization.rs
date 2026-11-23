//! Historical deployment-finalization discovery over a reopened principal store.

use super::{coordinate, error_kind, hash, PRINCIPAL_MIGRATION};
use pale_core::records::{
    ArtifactDigest, CheckpointProfileDigest, ControlCheckpointDomainIdV1, ControlDomainEpochV1,
    ControlDomainSequenceV1, ControlIncarnationIdV1, ControlStoreIdV1,
    ControlTransactionSequenceV1, LocalAdminCommandIdV1, LocalAdminIdempotencyKeyV1,
    LocalAdminReceiptIdV1, PrincipalTrustCheckpointEpochV1, ResidentPrincipalIdV1,
    ResidentRecordRefV1, SchemaDigest, SchemaId, CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA,
    CONTROL_STATE_CHECKPOINT_V1_SCHEMA, DEPLOYMENT_BINDING_V1_SCHEMA,
    DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA, LOCAL_ADMIN_AUTHORITY_V1_SCHEMA,
    LOCAL_ADMIN_COMMAND_V1_SCHEMA, LOCAL_ADMIN_RECEIPT_V1_SCHEMA,
    PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA,
};
use pale_ports::{
    AccumulatorCheckpointBinding, AccumulatorCheckpointPhase, AccumulatorMutation,
    AccumulatorPosition, AuthorityStoreCoordinate, ControlCheckpointHead, ControlRecord,
    ControlRecordBytes, ControlStoreErrorKind, DeploymentFinalizationRecords,
    DeploymentFinalizationReplayIdentity, DeploymentFinalizationSetReadLimits,
    DeploymentFinalizationSetReadPort, DeploymentFinalizationSetSnapshot,
    DeploymentFinalizationSnapshot, OperationAccumulatorBatch, PreparedDeploymentFinalizationAtom,
    PrincipalTrustCheckpointHead, PrincipalTrustStoreBinding, StateAccumulatorBatch,
    UnresolvedCustodyAccumulatorBatch,
};
use rusqlite::{params, Connection};

const ADMIN_BODY: &[u8] = br#"{"record":"admin"}"#;
const PREDECESSOR_TRUST_BODY: &[u8] = br#"{"record":"prior-trust"}"#;
const PREDECESSOR_CONTROL_BODY: &[u8] = br#"{"record":"prior-control"}"#;

fn record(schema: &str, reference: &str, character: char, body: &[u8]) -> ControlRecord {
    ControlRecord::new(
        coordinate(reference, character),
        SchemaId::parse(schema).unwrap(),
        SchemaDigest::parse(hash('f')).unwrap(),
        ControlRecordBytes::new(body.to_vec()).unwrap(),
    )
}

fn checkpoint(
    sequence: u64,
    transaction: u64,
    reference: &str,
    character: char,
) -> ControlCheckpointHead {
    ControlCheckpointHead::new(
        ControlCheckpointDomainIdV1::parse("principal-head").unwrap(),
        ControlDomainEpochV1::new(1).unwrap(),
        ControlDomainSequenceV1::new(sequence).unwrap(),
        ControlTransactionSequenceV1::new(transaction).unwrap(),
        coordinate(reference, character),
    )
}

fn finalization_candidate_for_transaction(
    expected_transaction: u64,
) -> PreparedDeploymentFinalizationAtom {
    let principal = ResidentPrincipalIdV1::parse("principal-1").unwrap();
    let store = AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("trust-store-1").unwrap(),
        ControlIncarnationIdV1::parse("trust-incarnation-1").unwrap(),
    );
    let records = DeploymentFinalizationRecords::new(
        record(
            LOCAL_ADMIN_COMMAND_V1_SCHEMA,
            "admin-command",
            '1',
            br#"{"record":"command"}"#,
        ),
        record(
            DEPLOYMENT_BINDING_V1_SCHEMA,
            "deployment-binding",
            '2',
            br#"{"record":"binding"}"#,
        ),
        record(
            CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA,
            "deployment-checkpoint-signer",
            '3',
            br#"{"record":"signer"}"#,
        ),
        record(
            DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA,
            "deployment-runtime-authority",
            '4',
            br#"{"record":"runtime"}"#,
        ),
        record(
            PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA,
            "trust-successor",
            '5',
            br#"{"record":"trust"}"#,
        ),
        record(
            CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
            "principal-control-successor",
            '6',
            br#"{"record":"control"}"#,
        ),
        record(
            LOCAL_ADMIN_RECEIPT_V1_SCHEMA,
            "admin-receipt",
            '7',
            br#"{"record":"receipt"}"#,
        ),
    )
    .unwrap();
    let predecessor_control = checkpoint(1, 1, "principal-control-predecessor", '8');
    let successor_control = checkpoint(
        2,
        expected_transaction + 1,
        "principal-control-successor",
        '6',
    );
    let preparation = pale_ports::ControlTransactionPreparation::new(
        store.clone(),
        ControlTransactionSequenceV1::new(expected_transaction).unwrap(),
    )
    .unwrap();
    let accumulator_binding = AccumulatorCheckpointBinding::new(
        store.clone(),
        ResidentRecordRefV1::parse("checkpoint-profile").unwrap(),
        CheckpointProfileDigest::parse(hash('e')).unwrap(),
        AccumulatorCheckpointPhase::Successor(predecessor_control.clone()),
        successor_control.clone(),
        preparation,
    )
    .unwrap();
    let zero = ArtifactDigest::parse(hash('0')).unwrap();
    let accumulator = OperationAccumulatorBatch::new(
        accumulator_binding,
        StateAccumulatorBatch::new(
            zero.clone(),
            ArtifactDigest::parse(hash('b')).unwrap(),
            vec![AccumulatorMutation::new(
                AccumulatorPosition::Leaf([b'b'; 32]),
                None,
                Some(vec![b'b'; 32]),
                32,
            )
            .unwrap()],
            1,
        )
        .unwrap(),
        UnresolvedCustodyAccumulatorBatch::new(zero.clone(), zero, Vec::new(), 1).unwrap(),
    )
    .unwrap();
    PreparedDeploymentFinalizationAtom::new(
        PrincipalTrustStoreBinding::new(
            store.store_id().clone(),
            store.incarnation_id().clone(),
            principal.clone(),
        ),
        DeploymentFinalizationReplayIdentity::new(
            LocalAdminCommandIdV1::parse("finalization-command").unwrap(),
            LocalAdminIdempotencyKeyV1::parse("finalization-replay").unwrap(),
            coordinate("admin-authority", 'a'),
            records.command().coordinate().clone(),
            LocalAdminReceiptIdV1::parse("finalization-receipt").unwrap(),
        ),
        records,
        PrincipalTrustCheckpointHead::new(
            principal.clone(),
            PrincipalTrustCheckpointEpochV1::new(1).unwrap(),
            coordinate("trust-predecessor", '9'),
        ),
        PrincipalTrustCheckpointHead::new(
            principal,
            PrincipalTrustCheckpointEpochV1::new(2).unwrap(),
            coordinate("trust-successor", '5'),
        ),
        predecessor_control,
        successor_control,
        accumulator,
    )
    .unwrap()
}

fn finalization_candidate() -> PreparedDeploymentFinalizationAtom {
    finalization_candidate_for_transaction(1)
}

fn second_finalization_candidate() -> PreparedDeploymentFinalizationAtom {
    let principal = ResidentPrincipalIdV1::parse("principal-1").unwrap();
    let store = AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("trust-store-1").unwrap(),
        ControlIncarnationIdV1::parse("trust-incarnation-1").unwrap(),
    );
    let records = DeploymentFinalizationRecords::new(
        record(
            LOCAL_ADMIN_COMMAND_V1_SCHEMA,
            "admin-command-two",
            'c',
            br#"{"record":"command-two"}"#,
        ),
        record(
            DEPLOYMENT_BINDING_V1_SCHEMA,
            "deployment-binding-two",
            'd',
            br#"{"record":"binding-two"}"#,
        ),
        record(
            CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA,
            "deployment-checkpoint-signer-two",
            'e',
            br#"{"record":"signer-two"}"#,
        ),
        record(
            DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA,
            "deployment-runtime-authority-two",
            'f',
            br#"{"record":"runtime-two"}"#,
        ),
        record(
            PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA,
            "trust-successor-two",
            'b',
            br#"{"record":"trust-two"}"#,
        ),
        record(
            CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
            "principal-control-successor-two",
            'c',
            br#"{"record":"control-two"}"#,
        ),
        record(
            LOCAL_ADMIN_RECEIPT_V1_SCHEMA,
            "admin-receipt-two",
            'd',
            br#"{"record":"receipt-two"}"#,
        ),
    )
    .unwrap();
    let predecessor_control = checkpoint(2, 2, "principal-control-successor", '6');
    let successor_control = checkpoint(3, 3, "principal-control-successor-two", 'c');
    let preparation = pale_ports::ControlTransactionPreparation::new(
        store.clone(),
        ControlTransactionSequenceV1::new(2).unwrap(),
    )
    .unwrap();
    let accumulator_binding = AccumulatorCheckpointBinding::new(
        store.clone(),
        ResidentRecordRefV1::parse("checkpoint-profile").unwrap(),
        CheckpointProfileDigest::parse(hash('e')).unwrap(),
        AccumulatorCheckpointPhase::Successor(predecessor_control.clone()),
        successor_control.clone(),
        preparation,
    )
    .unwrap();
    let zero = ArtifactDigest::parse(hash('0')).unwrap();
    let accumulator = OperationAccumulatorBatch::new(
        accumulator_binding,
        StateAccumulatorBatch::new(
            ArtifactDigest::parse(hash('b')).unwrap(),
            ArtifactDigest::parse(hash('c')).unwrap(),
            vec![AccumulatorMutation::new(
                AccumulatorPosition::Leaf([b'b'; 32]),
                Some(vec![b'b'; 32]),
                Some(vec![b'c'; 32]),
                32,
            )
            .unwrap()],
            1,
        )
        .unwrap(),
        UnresolvedCustodyAccumulatorBatch::new(zero.clone(), zero, Vec::new(), 1).unwrap(),
    )
    .unwrap();
    PreparedDeploymentFinalizationAtom::new(
        PrincipalTrustStoreBinding::new(
            store.store_id().clone(),
            store.incarnation_id().clone(),
            principal.clone(),
        ),
        DeploymentFinalizationReplayIdentity::new(
            LocalAdminCommandIdV1::parse("finalization-command-two").unwrap(),
            LocalAdminIdempotencyKeyV1::parse("finalization-replay-two").unwrap(),
            coordinate("admin-authority", 'a'),
            records.command().coordinate().clone(),
            LocalAdminReceiptIdV1::parse("finalization-receipt-two").unwrap(),
        ),
        records,
        PrincipalTrustCheckpointHead::new(
            principal.clone(),
            PrincipalTrustCheckpointEpochV1::new(2).unwrap(),
            coordinate("trust-successor", '5'),
        ),
        PrincipalTrustCheckpointHead::new(
            principal,
            PrincipalTrustCheckpointEpochV1::new(3).unwrap(),
            coordinate("trust-successor-two", 'b'),
        ),
        predecessor_control,
        successor_control,
        accumulator,
    )
    .unwrap()
}

fn copied_record_bytes(
    candidate: &PreparedDeploymentFinalizationAtom,
    predecessor_trust_body: &[u8],
    predecessor_control_body: &[u8],
) -> usize {
    ADMIN_BODY.len()
        + predecessor_trust_body.len()
        + predecessor_control_body.len()
        + [
            candidate.records().command(),
            candidate.records().binding(),
            candidate.records().checkpoint_signing_authority(),
            candidate.records().runtime_authority(),
            candidate.records().trust_checkpoint(),
            candidate.records().control_checkpoint(),
            candidate.records().receipt(),
        ]
        .into_iter()
        .map(|record| record.canonical_record_jcs().as_slice().len())
        .sum::<usize>()
}

fn initialize_principal(path: &std::path::Path) -> Connection {
    let connection = Connection::open(path).unwrap();
    connection.execute_batch(PRINCIPAL_MIGRATION).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    connection.execute("INSERT INTO pale_control_store_header (singleton,store_kind,logical_store_id,control_incarnation_id,logical_transaction_sequence,store_schema_id,store_schema_sha256,store_schema_version,storage_manifest_sha256,migration_head_sha256) VALUES (1,'principal_trust','trust-store-1','trust-incarnation-1',1,'schema',?1,1,?2,?3)",params![hash('1'),hash('2'),hash('3')]).unwrap();
    connection
        .execute(
            "INSERT INTO pale_principal_trust_store_binding VALUES (1,'principal-1')",
            [],
        )
        .unwrap();
    for (reference, digest, schema, body) in [
        (
            "admin-authority",
            'a',
            LOCAL_ADMIN_AUTHORITY_V1_SCHEMA,
            ADMIN_BODY,
        ),
        (
            "trust-predecessor",
            '9',
            PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA,
            PREDECESSOR_TRUST_BODY,
        ),
        (
            "principal-control-predecessor",
            '8',
            CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
            PREDECESSOR_CONTROL_BODY,
        ),
    ] {
        connection.execute("INSERT INTO pale_principal_trust_record (record_ref,record_sha256,schema_id,schema_sha256,principal_id,canonical_record_jcs,inserted_transaction_sequence) VALUES (?1,?2,?3,?4,'principal-1',?5,1)",params![reference,hash(digest),schema,hash('f'),body]).unwrap();
    }
    connection.execute("INSERT INTO pale_principal_trust_checkpoint_head VALUES (1,'principal-1',1,'trust-predecessor',?1)",[hash('9')]).unwrap();
    connection.execute("INSERT INTO pale_principal_control_checkpoint_head (checkpoint_domain_id,domain_epoch,domain_sequence,control_checkpoint_ref,control_checkpoint_sha256) VALUES ('principal-head',1,1,'principal-control-predecessor',?1)",[hash('8')]).unwrap();
    connection.execute("INSERT INTO pale_principal_control_accumulator_frontier (checkpoint_domain_id,domain_epoch,checkpoint_profile_ref,checkpoint_profile_sha256,control_checkpoint_ref,control_checkpoint_sha256,state_root_sha256,unresolved_root_sha256) VALUES ('principal-head',1,'checkpoint-profile',?1,'principal-control-predecessor',?2,?3,?4)",params![hash('e'),hash('8'),hash('0'),hash('0')]).unwrap();
    connection
}

fn binding() -> PrincipalTrustStoreBinding {
    PrincipalTrustStoreBinding::new(
        ControlStoreIdV1::parse("trust-store-1").unwrap(),
        ControlIncarnationIdV1::parse("trust-incarnation-1").unwrap(),
        ResidentPrincipalIdV1::parse("principal-1").unwrap(),
    )
}

fn commit(path: &std::path::Path) -> (PreparedDeploymentFinalizationAtom, usize) {
    let mut connection = initialize_principal(path);
    let candidate = finalization_candidate();
    let copied_bytes =
        copied_record_bytes(&candidate, PREDECESSOR_TRUST_BODY, PREDECESSOR_CONTROL_BODY);
    let _committed = candidate
        .clone()
        .commit(&mut crate::SqliteDeploymentFinalizationCustody {
            connection: &mut connection,
            maximum_record_bytes: 1_024,
        })
        .unwrap();
    (candidate, copied_bytes)
}

fn commit_after_unrelated_transaction(
    path: &std::path::Path,
) -> (PreparedDeploymentFinalizationAtom, usize) {
    let mut connection = initialize_principal(path);
    connection
        .execute(
            "UPDATE pale_control_store_header SET logical_transaction_sequence=2 WHERE singleton=1",
            [],
        )
        .unwrap();
    let candidate = finalization_candidate_for_transaction(2);
    let copied_bytes =
        copied_record_bytes(&candidate, PREDECESSOR_TRUST_BODY, PREDECESSOR_CONTROL_BODY);
    let _committed = candidate
        .clone()
        .commit(&mut crate::SqliteDeploymentFinalizationCustody {
            connection: &mut connection,
            maximum_record_bytes: 1_024,
        })
        .unwrap();
    (candidate, copied_bytes)
}

fn commit_two_across_reopen(
    path: &std::path::Path,
) -> (
    PreparedDeploymentFinalizationAtom,
    PreparedDeploymentFinalizationAtom,
    usize,
) {
    let (first, first_bytes) = commit(path);
    let second = second_finalization_candidate();
    let second_bytes = copied_record_bytes(
        &second,
        first
            .records()
            .trust_checkpoint()
            .canonical_record_jcs()
            .as_slice(),
        first
            .records()
            .control_checkpoint()
            .canonical_record_jcs()
            .as_slice(),
    );
    let mut reopened = Connection::open(path).unwrap();
    let _committed = second
        .clone()
        .commit(&mut crate::SqliteDeploymentFinalizationCustody {
            connection: &mut reopened,
            maximum_record_bytes: 1_024,
        })
        .unwrap();
    drop(reopened);
    (first, second, first_bytes + second_bytes)
}

fn read(
    connection: &mut Connection,
    maximum_record_bytes: usize,
    maximum_mutations: usize,
) -> Result<Option<DeploymentFinalizationSetSnapshot>, pale_ports::ControlStoreError> {
    read_with_limits(connection, 8, maximum_record_bytes, maximum_mutations)
}

fn read_with_limits(
    connection: &mut Connection,
    maximum_finalizations: usize,
    maximum_record_bytes: usize,
    maximum_mutations: usize,
) -> Result<Option<DeploymentFinalizationSetSnapshot>, pale_ports::ControlStoreError> {
    crate::SqliteDeploymentFinalizationSetReader { connection }.read_finalization_set(
        &binding(),
        DeploymentFinalizationSetReadLimits::new(
            maximum_finalizations,
            maximum_record_bytes,
            maximum_mutations,
        ),
    )
}

fn one_finalization(set: &DeploymentFinalizationSetSnapshot) -> &DeploymentFinalizationSnapshot {
    let [snapshot] = set.finalizations() else {
        panic!("expected exactly one deployment finalization")
    };
    snapshot
}

#[test]
fn complete_finalization_survives_reopen_with_exact_physical_evidence() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("principal.sqlite");
    let (candidate, copied_bytes) = commit(&path);

    let mut connection = Connection::open(&path).unwrap();
    let set = read(&mut connection, copied_bytes, 1).unwrap().unwrap();
    let snapshot = one_finalization(&set);
    assert_eq!(snapshot.binding(), candidate.binding());
    assert_eq!(snapshot.replay(), candidate.replay());
    assert_eq!(snapshot.records(), candidate.records());
    assert_eq!(snapshot.predecessor_trust(), candidate.predecessor_trust());
    assert_eq!(snapshot.successor_trust(), candidate.successor_trust());
    assert_eq!(
        snapshot.predecessor_control(),
        candidate.predecessor_control()
    );
    assert_eq!(snapshot.successor_control(), candidate.successor_control());
    assert_eq!(snapshot.accumulator(), candidate.accumulator());
    assert_eq!(snapshot.committed_transaction().get(), 2);
    assert_eq!(
        snapshot
            .admin_authority_record()
            .canonical_record_jcs()
            .as_slice(),
        ADMIN_BODY
    );
    assert_eq!(
        snapshot
            .predecessor_trust_record()
            .canonical_record_jcs()
            .as_slice(),
        PREDECESSOR_TRUST_BODY
    );
    assert_eq!(
        snapshot
            .predecessor_control_record()
            .canonical_record_jcs()
            .as_slice(),
        PREDECESSOR_CONTROL_BODY
    );
    assert!(connection.is_autocommit());
}

#[test]
fn sequential_finalizations_survive_reopen_in_exact_transaction_order() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("principal.sqlite");
    let (first, second, copied_bytes) = commit_two_across_reopen(&path);

    let mut connection = Connection::open(&path).unwrap();
    let first_read = read_with_limits(&mut connection, 2, copied_bytes, 2)
        .unwrap()
        .unwrap();
    assert_eq!(first_read.binding(), &binding());
    assert_eq!(first_read.observed_through().get(), 3);
    let [observed_first, observed_second] = first_read.finalizations() else {
        panic!("expected both deployment finalizations")
    };
    assert_eq!(observed_first.committed_transaction().get(), 2);
    assert_eq!(observed_second.committed_transaction().get(), 3);
    assert_eq!(observed_first.replay(), first.replay());
    assert_eq!(observed_first.records(), first.records());
    assert_eq!(observed_first.accumulator(), first.accumulator());
    assert_eq!(
        crate::deployment_initialization::finalization_snapshot_identity(observed_first),
        observed_first.candidate_identity_sha256().as_str()
    );
    assert_eq!(observed_second.replay(), second.replay());
    assert_eq!(observed_second.records(), second.records());
    assert_eq!(observed_second.accumulator(), second.accumulator());
    assert_eq!(
        crate::deployment_initialization::finalization_snapshot_identity(observed_second),
        observed_second.candidate_identity_sha256().as_str()
    );
    assert_eq!(
        observed_second
            .predecessor_trust_record()
            .canonical_record_jcs(),
        first.records().trust_checkpoint().canonical_record_jcs()
    );
    assert_eq!(
        observed_second
            .predecessor_control_record()
            .canonical_record_jcs(),
        first.records().control_checkpoint().canonical_record_jcs()
    );

    let second_read = read_with_limits(&mut connection, 2, copied_bytes, 2)
        .unwrap()
        .unwrap();
    assert_eq!(second_read, first_read);
    assert!(connection.is_autocommit());
}

#[test]
fn set_limits_fail_closed_without_returning_a_prefix() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("principal.sqlite");
    let (_, _, copied_bytes) = commit_two_across_reopen(&path);
    let mut connection = Connection::open(&path).unwrap();

    error_kind(
        read_with_limits(&mut connection, 1, copied_bytes, 2),
        ControlStoreErrorKind::BoundExceeded,
    );
    error_kind(
        read_with_limits(&mut connection, 2, copied_bytes - 1, 2),
        ControlStoreErrorKind::BoundExceeded,
    );
    error_kind(
        read_with_limits(&mut connection, 2, copied_bytes, 1),
        ControlStoreErrorKind::BoundExceeded,
    );
}

#[test]
fn predecessor_checkpoint_keeps_its_original_transaction_sequence() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("principal.sqlite");
    let (candidate, copied_bytes) = commit_after_unrelated_transaction(&path);

    let mut connection = Connection::open(&path).unwrap();
    let set = read(&mut connection, copied_bytes, 1).unwrap().unwrap();
    let snapshot = one_finalization(&set);
    assert_eq!(
        snapshot.predecessor_control(),
        candidate.predecessor_control()
    );
    assert_eq!(snapshot.successor_control(), candidate.successor_control());
    assert_eq!(snapshot.committed_transaction().get(), 3);
}

#[test]
fn absence_and_copy_limits_are_distinct() {
    let directory = crate::test_support::private_tempdir();
    let absent_path = directory.path().join("absent.sqlite");
    let mut absent = initialize_principal(&absent_path);
    assert!(read(&mut absent, 0, 0).unwrap().is_none());

    let partial_path = directory.path().join("partial.sqlite");
    let mut partial = initialize_principal(&partial_path);
    partial.execute("INSERT INTO pale_principal_trust_record (record_ref,record_sha256,schema_id,schema_sha256,principal_id,canonical_record_jcs,inserted_transaction_sequence) VALUES ('orphan-binding',?1,?2,?3,'principal-1',X'01',1)", params![hash('c'), DEPLOYMENT_BINDING_V1_SCHEMA, hash('f')]).unwrap();
    error_kind(
        read(&mut partial, 1, 0),
        ControlStoreErrorKind::RecoveryRequired,
    );

    let retained_path = directory.path().join("retained.sqlite");
    let (_, copied_bytes) = commit(&retained_path);
    let mut retained = Connection::open(&retained_path).unwrap();
    error_kind(
        read(&mut retained, copied_bytes - 1, 1),
        ControlStoreErrorKind::BoundExceeded,
    );
    error_kind(
        read(&mut retained, copied_bytes, 0),
        ControlStoreErrorKind::BoundExceeded,
    );
}

#[test]
fn unrelated_administration_records_do_not_fabricate_partial_finalization() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("principal.sqlite");
    let mut connection = initialize_principal(&path);
    for (reference, schema, character) in [
        ("other-admin-command", LOCAL_ADMIN_COMMAND_V1_SCHEMA, 'c'),
        ("other-admin-receipt", LOCAL_ADMIN_RECEIPT_V1_SCHEMA, 'd'),
    ] {
        connection.execute(
            "INSERT INTO pale_principal_trust_record \
             (record_ref,record_sha256,schema_id,schema_sha256,principal_id,canonical_record_jcs,inserted_transaction_sequence) \
             VALUES (?1,?2,?3,?4,'principal-1',X'01',1)",
            params![reference, hash(character), schema, hash('f')],
        )
        .unwrap();
    }

    assert!(read(&mut connection, 0, 0).unwrap().is_none());
}

fn corrupt_and_expect_recovery(statement: &str, parameters: impl rusqlite::Params) {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("principal.sqlite");
    let (_, copied_bytes) = commit(&path);
    let mut connection = Connection::open(&path).unwrap();
    connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    connection.execute(statement, parameters).unwrap();
    error_kind(
        read(&mut connection, copied_bytes + 1_024, 8),
        ControlStoreErrorKind::RecoveryRequired,
    );
}

#[test]
fn partial_surplus_and_substituted_closures_require_recovery() {
    corrupt_and_expect_recovery("DELETE FROM pale_principal_deployment_finalization", []);
    corrupt_and_expect_recovery(
        "DELETE FROM pale_principal_trust_record WHERE record_ref='admin-command'",
        [],
    );
    corrupt_and_expect_recovery(
        "INSERT INTO pale_principal_deployment_finalization_mutation \
         SELECT admin_authority_ref,admin_authority_sha256,'orphan-replay',tree_kind,position_kind,position_key,predecessor_node_bytes,successor_node_bytes \
           FROM pale_principal_deployment_finalization_mutation LIMIT 1",
        [],
    );
    corrupt_and_expect_recovery(
        "INSERT INTO pale_principal_deployment_finalization (admin_authority_ref,admin_authority_sha256,idempotency_key,admin_command_id,command_ref,command_sha256,receipt_id,receipt_ref,receipt_sha256,binding_ref,binding_sha256,checkpoint_signer_ref,checkpoint_signer_sha256,runtime_authority_ref,runtime_authority_sha256,trust_checkpoint_ref,trust_checkpoint_sha256,control_checkpoint_ref,control_checkpoint_sha256,transaction_sequence,principal_id,predecessor_trust_epoch,predecessor_trust_ref,predecessor_trust_sha256,successor_trust_epoch,successor_trust_ref,successor_trust_sha256,control_domain_id,predecessor_control_epoch,predecessor_control_sequence,predecessor_control_ref,predecessor_control_sha256,successor_control_epoch,successor_control_sequence,successor_control_ref,successor_control_sha256,checkpoint_profile_ref,checkpoint_profile_sha256,predecessor_state_root_sha256,successor_state_root_sha256,predecessor_unresolved_root_sha256,successor_unresolved_root_sha256,mutation_count,command_body_sha256,binding_body_sha256,checkpoint_signer_body_sha256,runtime_authority_body_sha256,trust_checkpoint_body_sha256,control_checkpoint_body_sha256,receipt_body_sha256,candidate_identity_sha256) SELECT admin_authority_ref,admin_authority_sha256,'surplus-key',admin_command_id,command_ref,command_sha256,receipt_id,receipt_ref,receipt_sha256,binding_ref,binding_sha256,checkpoint_signer_ref,checkpoint_signer_sha256,runtime_authority_ref,runtime_authority_sha256,trust_checkpoint_ref,trust_checkpoint_sha256,control_checkpoint_ref,control_checkpoint_sha256,transaction_sequence,principal_id,predecessor_trust_epoch,predecessor_trust_ref,predecessor_trust_sha256,successor_trust_epoch,successor_trust_ref,successor_trust_sha256,control_domain_id,predecessor_control_epoch,predecessor_control_sequence,predecessor_control_ref,predecessor_control_sha256,successor_control_epoch,successor_control_sequence,successor_control_ref,successor_control_sha256,checkpoint_profile_ref,checkpoint_profile_sha256,predecessor_state_root_sha256,successor_state_root_sha256,predecessor_unresolved_root_sha256,successor_unresolved_root_sha256,mutation_count,command_body_sha256,binding_body_sha256,checkpoint_signer_body_sha256,runtime_authority_body_sha256,trust_checkpoint_body_sha256,control_checkpoint_body_sha256,receipt_body_sha256,candidate_identity_sha256 FROM pale_principal_deployment_finalization",
        [],
    );
    corrupt_and_expect_recovery(
        "UPDATE pale_principal_deployment_finalization SET command_body_sha256=?1",
        [hash('0')],
    );
    corrupt_and_expect_recovery(
        "UPDATE pale_principal_deployment_finalization SET command_ref='deployment-binding',command_sha256=?1",
        [hash('2')],
    );
    corrupt_and_expect_recovery(
        "UPDATE pale_principal_trust_record SET schema_id=?1 WHERE record_ref='admin-command'",
        [DEPLOYMENT_BINDING_V1_SCHEMA],
    );
    corrupt_and_expect_recovery(
        "UPDATE pale_principal_deployment_finalization SET successor_control_sequence=3",
        [],
    );
    corrupt_and_expect_recovery(
        "UPDATE pale_principal_deployment_finalization SET successor_state_root_sha256=?1",
        [hash('1')],
    );
    corrupt_and_expect_recovery(
        "UPDATE pale_principal_deployment_finalization SET mutation_count=1000",
        [],
    );
    corrupt_and_expect_recovery(
        "UPDATE pale_principal_deployment_finalization SET candidate_identity_sha256=?1",
        [hash('1')],
    );
    corrupt_and_expect_recovery(
        "UPDATE pale_principal_deployment_finalization_mutation SET position_key=?1",
        [vec![0_u8; 33]],
    );
}

#[test]
fn foreign_future_and_reversed_close_ownership_require_recovery() {
    corrupt_and_expect_recovery(
        "UPDATE pale_principal_deployment_finalization SET principal_id='another-principal'",
        [],
    );
    corrupt_and_expect_recovery(
        "UPDATE pale_principal_deployment_finalization SET transaction_sequence=3",
        [],
    );
    corrupt_and_expect_recovery(
        "UPDATE pale_principal_deployment_finalization SET transaction_sequence=1",
        [],
    );
}

#[test]
fn self_consistent_invalid_accumulator_bytes_require_recovery() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("principal.sqlite");
    let (candidate, copied_bytes) = commit(&path);
    let invalid_mutation = AccumulatorMutation::new(
        AccumulatorPosition::Branch {
            height: 1,
            path: [b'b'; 32],
        },
        None,
        Some(vec![0_u8; 66]),
        132,
    )
    .unwrap();
    let invalid_accumulator = OperationAccumulatorBatch::new(
        candidate.accumulator().binding().clone(),
        StateAccumulatorBatch::new(
            candidate.accumulator().state().predecessor_root().clone(),
            candidate.accumulator().state().successor_root().clone(),
            vec![invalid_mutation],
            1,
        )
        .unwrap(),
        candidate.accumulator().unresolved().clone(),
    )
    .unwrap();
    let invalid_snapshot = DeploymentFinalizationSnapshot::new(
        candidate.binding().clone(),
        candidate.replay().clone(),
        record(
            LOCAL_ADMIN_AUTHORITY_V1_SCHEMA,
            "admin-authority",
            'a',
            ADMIN_BODY,
        ),
        candidate.records().clone(),
        candidate.predecessor_trust().clone(),
        record(
            PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA,
            "trust-predecessor",
            '9',
            PREDECESSOR_TRUST_BODY,
        ),
        candidate.successor_trust().clone(),
        candidate.predecessor_control().clone(),
        record(
            CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
            "principal-control-predecessor",
            '8',
            PREDECESSOR_CONTROL_BODY,
        ),
        candidate.successor_control().clone(),
        invalid_accumulator,
        ControlTransactionSequenceV1::new(2).unwrap(),
        ArtifactDigest::parse(hash('0')).unwrap(),
    )
    .unwrap();
    let invalid_identity =
        crate::deployment_initialization::finalization_snapshot_identity(&invalid_snapshot);
    let mut position = vec![b'b'; 33];
    position[0] = 1;

    let mut connection = Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE pale_principal_deployment_finalization_mutation \
             SET position_kind='branch',position_key=?1,successor_node_bytes=?2",
            params![position, vec![0_u8; 66]],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE pale_principal_deployment_finalization SET candidate_identity_sha256=?1",
            [invalid_identity],
        )
        .unwrap();
    error_kind(
        read(&mut connection, copied_bytes, 1),
        ControlStoreErrorKind::RecoveryRequired,
    );
}

#[test]
fn wrong_store_transaction_and_surplus_transaction_records_require_recovery() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("principal.sqlite");
    let (_, copied_bytes) = commit(&path);
    let mut connection = Connection::open(&path).unwrap();
    let wrong = PrincipalTrustStoreBinding::new(
        ControlStoreIdV1::parse("another-store").unwrap(),
        ControlIncarnationIdV1::parse("trust-incarnation-1").unwrap(),
        ResidentPrincipalIdV1::parse("principal-1").unwrap(),
    );
    error_kind(
        crate::SqliteDeploymentFinalizationSetReader {
            connection: &mut connection,
        }
        .read_finalization_set(
            &wrong,
            DeploymentFinalizationSetReadLimits::new(1, copied_bytes, 1),
        ),
        ControlStoreErrorKind::RecoveryRequired,
    );
    connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    connection.execute("INSERT INTO pale_principal_trust_record (record_ref,record_sha256,schema_id,schema_sha256,principal_id,canonical_record_jcs,inserted_transaction_sequence) VALUES ('surplus',?1,'other',?2,'principal-1',X'01',2)",params![hash('1'),hash('2')]).unwrap();
    error_kind(
        read(&mut connection, copied_bytes + 1, 1),
        ControlStoreErrorKind::RecoveryRequired,
    );
}

#[test]
fn native_read_failure_stays_a_storage_failure() {
    let directory = crate::test_support::private_tempdir();
    let path = directory.path().join("principal.sqlite");
    let (_, copied_bytes) = commit(&path);
    let mut connection = Connection::open(&path).unwrap();
    connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    connection
        .execute("DROP TABLE pale_principal_deployment_finalization", [])
        .unwrap();
    error_kind(
        read(&mut connection, copied_bytes, 1),
        ControlStoreErrorKind::Storage,
    );
}
