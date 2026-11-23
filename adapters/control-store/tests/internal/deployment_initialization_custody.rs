//! Production `SQLite` custody qualification for deployment initialization.

use super::*;
use std::sync::{Arc, Barrier};
use std::time::Duration;

use pale_core::records::{
    ArtifactDigest, CheckpointProfileDigest, ControlCheckpointDomainIdV1, ControlDomainEpochV1,
    ControlDomainSequenceV1, ControlIncarnationIdV1, ControlStoreIdV1,
    ControlTransactionSequenceV1, DeploymentControlStoreGenesisIdV1, LocalAdminCommandIdV1,
    LocalAdminIdempotencyKeyV1, LocalAdminReceiptIdV1, PrincipalTrustCheckpointEpochV1,
    ResidentPrincipalIdV1, ResidentRecordRefV1, SchemaDigest, SchemaId, WorkspaceId,
    CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA, CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
    DEPLOYMENT_BINDING_V1_SCHEMA, DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA,
    DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA, LOCAL_ADMIN_COMMAND_V1_SCHEMA,
    LOCAL_ADMIN_RECEIPT_V1_SCHEMA, PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA,
};
use pale_ports::{
    AccumulatorCheckpointBinding, AccumulatorCheckpointPhase, AccumulatorMutation,
    AccumulatorPosition, AntiRollbackAnchor, AntiRollbackAnchorAdvanceDisposition,
    AntiRollbackAnchorError, AntiRollbackAnchorKey, AntiRollbackAnchorObservation,
    AntiRollbackAnchorPort, ControlRecordBytes, ControlRecordCoordinate, ControlRecordDigest,
    ControlTransactionPort, DeploymentControlStoreBinding,
    DeploymentControlStoreGenesisCommitEvidence, DeploymentControlStoreGenesisRecords,
    DeploymentFinalizationCommitEvidence, DeploymentFinalizationRecords,
    DeploymentFinalizationReplayIdentity, PreparedAntiRollbackAnchorTransition,
    PrincipalTrustCheckpointHead, PrincipalTrustStoreBinding, StateAccumulatorBatch,
    UnresolvedCustodyAccumulatorBatch,
};
use rusqlite::Connection;

const PRINCIPAL_MIGRATION: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/principal-trust/0001_initial_v1.sql"
));
const DEPLOYMENT_MIGRATION: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/migrations/deployment-control/0001_initial_v1.sql"
));

fn hash(character: char) -> String {
    character.to_string().repeat(64)
}
fn coordinate(reference: &str, character: char) -> ControlRecordCoordinate {
    ControlRecordCoordinate::new(
        ResidentRecordRefV1::parse(reference).unwrap(),
        ControlRecordDigest::parse(hash(character)).unwrap(),
    )
}
fn record(schema: &str, reference: &str, character: char, body: &str) -> ControlRecord {
    ControlRecord::new(
        coordinate(reference, character),
        SchemaId::parse(schema).unwrap(),
        SchemaDigest::parse(hash('f')).unwrap(),
        ControlRecordBytes::new(body.as_bytes().to_vec()).unwrap(),
    )
}
fn checkpoint(
    domain: &str,
    sequence: u64,
    transaction: u64,
    reference: &str,
    character: char,
) -> ControlCheckpointHead {
    ControlCheckpointHead::new(
        ControlCheckpointDomainIdV1::parse(domain).unwrap(),
        ControlDomainEpochV1::new(1).unwrap(),
        ControlDomainSequenceV1::new(sequence).unwrap(),
        ControlTransactionSequenceV1::new(transaction).unwrap(),
        coordinate(reference, character),
    )
}
fn batch(
    store: AuthorityStoreCoordinate,
    phase: AccumulatorCheckpointPhase,
    head: ControlCheckpointHead,
    character: u8,
) -> OperationAccumulatorBatch {
    let preparation = pale_ports::ControlTransactionPreparation::new(
        store.clone(),
        ControlTransactionSequenceV1::new(1).unwrap(),
    )
    .unwrap();
    let binding = AccumulatorCheckpointBinding::new(
        store,
        ResidentRecordRefV1::parse("checkpoint-profile").unwrap(),
        CheckpointProfileDigest::parse(hash('e')).unwrap(),
        phase,
        head,
        preparation,
    )
    .unwrap();
    let zero = ArtifactDigest::parse(hash('0')).unwrap();
    OperationAccumulatorBatch::new(
        binding,
        StateAccumulatorBatch::new(
            zero.clone(),
            ArtifactDigest::parse(hash(char::from(character))).unwrap(),
            vec![AccumulatorMutation::new(
                AccumulatorPosition::Leaf([character; 32]),
                None,
                Some(vec![character; 32]),
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
fn finalization(receipt_body: &str, key: &str) -> PreparedDeploymentFinalizationAtom {
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
            r#"{"record":"command"}"#,
        ),
        record(
            DEPLOYMENT_BINDING_V1_SCHEMA,
            "deployment-binding",
            '2',
            r#"{"record":"binding"}"#,
        ),
        record(
            CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA,
            "deployment-checkpoint-signer",
            '3',
            r#"{"record":"signer"}"#,
        ),
        record(
            DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA,
            "deployment-runtime-authority",
            '4',
            r#"{"record":"runtime"}"#,
        ),
        record(
            PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA,
            "trust-successor",
            '5',
            r#"{"record":"trust"}"#,
        ),
        record(
            CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
            "principal-control-successor",
            '6',
            r#"{"record":"control"}"#,
        ),
        record(
            LOCAL_ADMIN_RECEIPT_V1_SCHEMA,
            "admin-receipt",
            '7',
            receipt_body,
        ),
    )
    .unwrap();
    let predecessor_control =
        checkpoint("principal-head", 1, 1, "principal-control-predecessor", '8');
    let successor_control = checkpoint("principal-head", 2, 2, "principal-control-successor", '6');
    PreparedDeploymentFinalizationAtom::new(
        PrincipalTrustStoreBinding::new(
            store.store_id().clone(),
            store.incarnation_id().clone(),
            principal.clone(),
        ),
        DeploymentFinalizationReplayIdentity::new(
            LocalAdminCommandIdV1::parse("finalization-command").unwrap(),
            LocalAdminIdempotencyKeyV1::parse(key).unwrap(),
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
        predecessor_control.clone(),
        successor_control.clone(),
        batch(
            store,
            AccumulatorCheckpointPhase::Successor(predecessor_control),
            successor_control,
            b'b',
        ),
    )
    .unwrap()
}
fn initialize_principal(connection: Connection) -> Connection {
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    connection.execute_batch(PRINCIPAL_MIGRATION).unwrap();
    connection.execute("INSERT INTO pale_control_store_header (singleton,store_kind,logical_store_id,control_incarnation_id,logical_transaction_sequence,store_schema_id,store_schema_sha256,store_schema_version,storage_manifest_sha256,migration_head_sha256) VALUES (1,'principal_trust','trust-store-1','trust-incarnation-1',1,'schema',?1,1,?2,?3)",[hash('1'),hash('2'),hash('3')]).unwrap();
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
            "pale/local-admin-authority.v1",
            "admin",
        ),
        (
            "trust-predecessor",
            '9',
            PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA,
            "trust",
        ),
        (
            "principal-control-predecessor",
            '8',
            CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
            "control",
        ),
    ] {
        connection.execute("INSERT INTO pale_principal_trust_record (record_ref,record_sha256,schema_id,schema_sha256,principal_id,canonical_record_jcs,inserted_transaction_sequence) VALUES (?1,?2,?3,?4,'principal-1',?5,1)",rusqlite::params![reference,hash(digest),schema,hash('f'),body.as_bytes()]).unwrap();
    }
    connection.execute("INSERT INTO pale_principal_trust_checkpoint_head VALUES (1,'principal-1',1,'trust-predecessor',?1)",[hash('9')]).unwrap();
    connection.execute("INSERT INTO pale_principal_control_checkpoint_head VALUES ('principal-head',1,1,'principal-control-predecessor',?1)",[hash('8')]).unwrap();
    connection.execute("INSERT INTO pale_principal_control_accumulator_frontier VALUES ('principal-head',1,'checkpoint-profile',?1,'principal-control-predecessor',?2,?3,?3)",rusqlite::params![hash('e'),hash('8'),hash('0')]).unwrap();
    connection
}
fn principal_memory() -> Connection {
    initialize_principal(Connection::open_in_memory().unwrap())
}

#[derive(Default)]
struct CommitOnly;
impl DeploymentFinalizationCustodyPort for CommitOnly {
    fn compare_and_commit(
        &mut self,
        _: &PreparedDeploymentFinalizationAtom,
    ) -> Result<DeploymentFinalizationCommitDisposition, ControlStoreError> {
        Ok(DeploymentFinalizationCommitDisposition::New)
    }
    fn observe_exact(
        &mut self,
        _: &PreparedDeploymentFinalizationAtom,
    ) -> Result<DeploymentFinalizationDatabaseObservation, ControlStoreError> {
        Ok(DeploymentFinalizationDatabaseObservation::Exact)
    }
}
#[derive(Default)]
struct Anchors;
impl AntiRollbackAnchorPort for Anchors {
    fn read(
        &mut self,
        _: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        unreachable!()
    }
    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        Ok(AntiRollbackAnchorAdvanceDisposition::Advanced)
    }
    fn observe(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        Ok(AntiRollbackAnchorObservation::ExactSuccessor)
    }
}
fn anchored() -> pale_ports::AnchoredDeploymentFinalizationCapability {
    finalization("receipt", "finalization-replay")
        .commit(&mut CommitOnly)
        .unwrap()
        .anchors_pending()
        .complete(&mut Anchors)
        .unwrap()
}
fn genesis(
    capability: &pale_ports::AnchoredDeploymentFinalizationCapability,
    count: usize,
) -> PreparedDeploymentControlStoreGenesisAtom {
    genesis_with_receipt(capability, count, "receipt")
}
fn genesis_with_receipt(
    capability: &pale_ports::AnchoredDeploymentFinalizationCapability,
    count: usize,
    receipt_body: &str,
) -> PreparedDeploymentControlStoreGenesisAtom {
    let store = AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("deployment-store-1").unwrap(),
        ControlIncarnationIdV1::parse("deployment-incarnation-1").unwrap(),
    );
    let binding = DeploymentControlStoreBinding::new(
        store.store_id().clone(),
        store.incarnation_id().clone(),
        capability.principal_id().clone(),
        WorkspaceId::parse("workspace-1").unwrap(),
        capability.binding().clone(),
        capability.checkpoint_signing_authority().clone(),
        capability.runtime_authority().clone(),
    );
    let mut records = Vec::with_capacity(count);
    let mut heads = Vec::with_capacity(count);
    let mut accumulators = Vec::with_capacity(count);
    for ordinal in 0..count {
        let domain = format!("deployment-{ordinal:04}");
        let reference = format!("deployment-checkpoint-{ordinal:04}");
        let head = checkpoint(&domain, 1, 2, &reference, 'c');
        records.push(record(
            CONTROL_STATE_CHECKPOINT_V1_SCHEMA,
            &reference,
            'c',
            &format!(r#"{{"checkpoint":{ordinal}}}"#),
        ));
        heads.push(head.clone());
        accumulators.push(batch(
            store.clone(),
            AccumulatorCheckpointPhase::Initial,
            head,
            b'c',
        ));
    }
    PreparedDeploymentControlStoreGenesisAtom::new(
        capability,
        binding,
        DeploymentControlStoreGenesisIdV1::parse("deployment-genesis-1").unwrap(),
        DeploymentControlStoreGenesisRecords::new(
            records,
            record(
                DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA,
                "deployment-genesis-receipt",
                'd',
                receipt_body,
            ),
        )
        .unwrap(),
        heads,
        accumulators,
    )
    .unwrap()
}
fn initialize_deployment(connection: Connection) -> Connection {
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    connection.execute_batch(DEPLOYMENT_MIGRATION).unwrap();
    connection.execute("INSERT INTO pale_control_store_header (singleton,store_kind,logical_store_id,control_incarnation_id,logical_transaction_sequence,store_schema_id,store_schema_sha256,store_schema_version,storage_manifest_sha256,migration_head_sha256) VALUES (1,'deployment_control','deployment-store-1','deployment-incarnation-1',1,'schema',?1,1,?2,?3)",[hash('1'),hash('2'),hash('3')]).unwrap();
    connection.execute("INSERT INTO pale_deployment_control_store_binding VALUES (1,'workspace-1','deployment-binding',?1)",[hash('2')]).unwrap();
    connection
}
fn deployment_memory() -> Connection {
    initialize_deployment(Connection::open_in_memory().unwrap())
}

#[test]
fn finalization_commit_replay_changed_key_and_corrupt_closure() {
    let candidate = finalization("receipt", "finalization-replay");
    let mut connection = principal_memory();
    let mut custody = SqliteDeploymentFinalizationCustody {
        connection: &mut connection,
        maximum_record_bytes: 1024,
    };
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap(),
        DeploymentFinalizationCommitDisposition::New
    );
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap(),
        DeploymentFinalizationCommitDisposition::ExactReplay
    );
    let changed = finalization("changed", "finalization-replay");
    assert_eq!(
        custody.compare_and_commit(&changed).unwrap_err().kind(),
        ControlStoreErrorKind::CandidateConflict
    );
    custody
        .connection
        .execute(
            "UPDATE pale_principal_deployment_finalization SET receipt_id='corrupt'",
            [],
        )
        .unwrap();
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    custody
        .connection
        .execute(
            "DELETE FROM pale_principal_deployment_finalization_mutation",
            [],
        )
        .unwrap();
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

#[test]
fn finalization_fault_boundaries_roll_back_to_predecessor() {
    for (action, table) in [
        ("INSERT", "pale_principal_trust_record"),
        ("UPDATE", "pale_principal_trust_checkpoint_head"),
        ("UPDATE", "pale_principal_control_checkpoint_head"),
        ("UPDATE", "pale_principal_control_accumulator_frontier"),
        ("INSERT", "pale_principal_deployment_finalization_mutation"),
        ("INSERT", "pale_principal_deployment_finalization"),
    ] {
        let candidate = finalization("receipt", "finalization-replay");
        let mut connection = principal_memory();
        let sql=format!("CREATE TEMP TRIGGER stop BEFORE {action} ON {table} BEGIN SELECT RAISE(ABORT,'fault'); END");
        connection.execute_batch(&sql).unwrap();
        let mut custody = SqliteDeploymentFinalizationCustody {
            connection: &mut connection,
            maximum_record_bytes: 1024,
        };
        assert!(custody.compare_and_commit(&candidate).is_err());
        custody.connection.execute("DROP TRIGGER stop", []).unwrap();
        assert_eq!(
            custody.observe_exact(&candidate).unwrap(),
            DeploymentFinalizationDatabaseObservation::Predecessor
        );
    }
}

#[test]
fn finalization_reopen_later_frontier_and_exhaustion_preserve_exact_replay() {
    let file = tempfile::NamedTempFile::new().unwrap();
    {
        let mut connection = initialize_principal(Connection::open(file.path()).unwrap());
        let candidate = finalization("receipt", "finalization-replay");
        let mut custody = SqliteDeploymentFinalizationCustody {
            connection: &mut connection,
            maximum_record_bytes: 1024,
        };
        custody.compare_and_commit(&candidate).unwrap();
        for (reference, digest, schema) in [
            ("trust-later", 'b', PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA),
            ("control-later", 'c', CONTROL_STATE_CHECKPOINT_V1_SCHEMA),
        ] {
            custody.connection.execute("INSERT INTO pale_principal_trust_record (record_ref,record_sha256,schema_id,schema_sha256,principal_id,canonical_record_jcs,inserted_transaction_sequence) VALUES (?1,?2,?3,?4,'principal-1',x'01',3)",rusqlite::params![reference,hash(digest),schema,hash('f')]).unwrap();
        }
        custody.connection.execute("UPDATE pale_principal_trust_checkpoint_head SET checkpoint_epoch=3,trust_checkpoint_ref='trust-later',trust_checkpoint_sha256=?1",[hash('b')]).unwrap();
        custody.connection.execute("UPDATE pale_principal_control_checkpoint_head SET domain_sequence=3,control_checkpoint_ref='control-later',control_checkpoint_sha256=?1",[hash('c')]).unwrap();
        custody.connection.execute("UPDATE pale_principal_control_accumulator_frontier SET control_checkpoint_ref='control-later',control_checkpoint_sha256=?1,state_root_sha256=?2",rusqlite::params![hash('c'),hash('d')]).unwrap();
        custody.connection.execute("UPDATE pale_control_store_header SET logical_transaction_sequence=9007199254740991",[]).unwrap();
        assert_eq!(
            custody.compare_and_commit(&candidate).unwrap(),
            DeploymentFinalizationCommitDisposition::ExactReplay
        );
    }
    let mut connection = Connection::open(file.path()).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    let candidate = finalization("receipt", "finalization-replay");
    let mut custody = SqliteDeploymentFinalizationCustody {
        connection: &mut connection,
        maximum_record_bytes: 1024,
    };
    assert_eq!(
        custody.observe_exact(&candidate).unwrap(),
        DeploymentFinalizationDatabaseObservation::Exact
    );
    assert_eq!(
        custody
            .prepare_transaction(&AuthorityStoreCoordinate::new(
                ControlStoreIdV1::parse("trust-store-1").unwrap(),
                ControlIncarnationIdV1::parse("trust-incarnation-1").unwrap()
            ))
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::TransactionExhausted
    );
}

#[test]
fn genesis_one_commit_replay_corruption_and_wrong_binding() {
    let capability = anchored();
    let candidate = genesis(&capability, 1);
    let mut connection = deployment_memory();
    let mut custody = SqliteDeploymentControlStoreGenesisCustody {
        connection: &mut connection,
        coordinate: candidate.binding().store().clone(),
        maximum_record_bytes: 1024,
    };
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap(),
        DeploymentControlStoreGenesisCommitDisposition::New
    );
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap(),
        DeploymentControlStoreGenesisCommitDisposition::ExactReplay
    );
    let changed = genesis_with_receipt(&capability, 1, "changed");
    assert_eq!(
        custody.compare_and_commit(&changed).unwrap_err().kind(),
        ControlStoreErrorKind::CandidateConflict
    );
    custody
        .connection
        .execute(
            "UPDATE pale_deployment_control_store_genesis SET workspace_id='corrupt'",
            [],
        )
        .unwrap();
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    custody
        .connection
        .execute(
            "DELETE FROM pale_deployment_control_store_genesis_mutation",
            [],
        )
        .unwrap();
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    custody.coordinate = AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("wrong").unwrap(),
        ControlIncarnationIdV1::parse("wrong").unwrap(),
    );
    assert_eq!(
        custody.observe_exact(&candidate).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

#[test]
fn genesis_fault_rolls_back_and_finalization_remains_exact() {
    let finalization = finalization("receipt", "finalization-replay");
    let mut principal = principal_memory();
    let mut transaction_one = SqliteDeploymentFinalizationCustody {
        connection: &mut principal,
        maximum_record_bytes: 1024,
    };
    transaction_one.compare_and_commit(&finalization).unwrap();
    let capability = anchored();
    let genesis = genesis(&capability, 1);
    let mut deployment = deployment_memory();
    deployment.execute_batch("CREATE TEMP TRIGGER stop BEFORE INSERT ON pale_deployment_control_store_genesis BEGIN SELECT RAISE(ABORT,'fault'); END").unwrap();
    let mut transaction_two = SqliteDeploymentControlStoreGenesisCustody {
        connection: &mut deployment,
        coordinate: genesis.binding().store().clone(),
        maximum_record_bytes: 1024,
    };
    assert!(transaction_two.compare_and_commit(&genesis).is_err());
    transaction_two
        .connection
        .execute("DROP TRIGGER stop", [])
        .unwrap();
    assert_eq!(
        transaction_two.observe_exact(&genesis).unwrap(),
        DeploymentControlStoreGenesisDatabaseObservation::Empty
    );
    assert_eq!(
        transaction_one.observe_exact(&finalization).unwrap(),
        DeploymentFinalizationDatabaseObservation::Exact
    );
}

#[test]
fn genesis_statement_faults_leave_the_store_empty() {
    let capability = anchored();
    let candidate = genesis(&capability, 1);
    for table in [
        "pale_deployment_control_record",
        "pale_deployment_operation_checkpoint_head",
        "pale_deployment_operation_accumulator_position",
        "pale_deployment_operation_accumulator_frontier",
        "pale_deployment_control_store_genesis_checkpoint",
        "pale_deployment_control_store_genesis_mutation",
        "pale_deployment_control_store_genesis",
    ] {
        let mut connection = deployment_memory();
        connection
            .execute_batch(&format!(
                "CREATE TEMP TRIGGER stop BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT,'fault'); END"
            ))
            .unwrap();
        let mut custody = SqliteDeploymentControlStoreGenesisCustody {
            connection: &mut connection,
            coordinate: candidate.binding().store().clone(),
            maximum_record_bytes: 1024,
        };
        assert!(custody.compare_and_commit(&candidate).is_err());
        custody.connection.execute("DROP TRIGGER stop", []).unwrap();
        assert_eq!(
            custody.observe_exact(&candidate).unwrap(),
            DeploymentControlStoreGenesisDatabaseObservation::Empty
        );
    }
}

#[test]
fn genesis_1024_members_are_bounded_ordered_and_replayable() {
    let capability = anchored();
    let candidate = genesis(&capability, 1024);
    let mut connection = deployment_memory();
    let mut custody = SqliteDeploymentControlStoreGenesisCustody {
        connection: &mut connection,
        coordinate: candidate.binding().store().clone(),
        maximum_record_bytes: 1024,
    };
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap(),
        DeploymentControlStoreGenesisCommitDisposition::New
    );
    assert_eq!(
        custody.observe_exact(&candidate).unwrap(),
        DeploymentControlStoreGenesisDatabaseObservation::Exact
    );
    let count: i64 = custody
        .connection
        .query_row(
            "SELECT count(*) FROM pale_deployment_control_store_genesis_checkpoint",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1024);
}

#[test]
fn genesis_reopen_and_saturated_transaction_still_replay_exactly() {
    let capability = anchored();
    let candidate = genesis(&capability, 1);
    let file = tempfile::NamedTempFile::new().unwrap();
    {
        let mut connection = initialize_deployment(Connection::open(file.path()).unwrap());
        let mut custody = SqliteDeploymentControlStoreGenesisCustody {
            connection: &mut connection,
            coordinate: candidate.binding().store().clone(),
            maximum_record_bytes: 1024,
        };
        custody.compare_and_commit(&candidate).unwrap();
        custody.connection.execute("INSERT INTO pale_deployment_control_record (record_ref,inserted_transaction_sequence,record_sha256,schema_id,schema_sha256,canonical_record_jcs) VALUES ('deployment-later',3,?1,?2,?3,x'01')",rusqlite::params![hash('e'),CONTROL_STATE_CHECKPOINT_V1_SCHEMA,hash('f')]).unwrap();
        custody.connection.execute("UPDATE pale_deployment_operation_checkpoint_head SET domain_sequence=2,control_checkpoint_ref='deployment-later',control_checkpoint_sha256=?1",[hash('e')]).unwrap();
        custody.connection.execute("UPDATE pale_deployment_operation_accumulator_frontier SET control_checkpoint_ref='deployment-later',control_checkpoint_sha256=?1,state_root_sha256=?2",rusqlite::params![hash('e'),hash('d')]).unwrap();
        custody.connection.execute("UPDATE pale_control_store_header SET logical_transaction_sequence=9007199254740991",[]).unwrap();
        assert_eq!(
            custody.compare_and_commit(&candidate).unwrap(),
            DeploymentControlStoreGenesisCommitDisposition::ExactReplay
        );
    }
    let mut connection = Connection::open(file.path()).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    let mut custody = SqliteDeploymentControlStoreGenesisCustody {
        connection: &mut connection,
        coordinate: candidate.binding().store().clone(),
        maximum_record_bytes: 1024,
    };
    assert_eq!(
        custody.observe_exact(&candidate).unwrap(),
        DeploymentControlStoreGenesisDatabaseObservation::Exact
    );
    assert_eq!(
        custody
            .prepare_transaction(candidate.binding().store())
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::TransactionExhausted
    );
}

#[test]
fn configured_record_ceiling_and_distinct_store_are_enforced() {
    let candidate = finalization("receipt", "finalization-replay");
    let mut connection = principal_memory();
    let mut custody = SqliteDeploymentFinalizationCustody {
        connection: &mut connection,
        maximum_record_bytes: 1,
    };
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap_err().kind(),
        ControlStoreErrorKind::BoundExceeded
    );
    let capability = anchored();
    let genesis = genesis(&capability, 1);
    let mut deployment = deployment_memory();
    let mut custody = SqliteDeploymentControlStoreGenesisCustody {
        connection: &mut deployment,
        coordinate: capability.trust_store().clone(),
        maximum_record_bytes: 1024,
    };
    assert_eq!(
        custody.compare_and_commit(&genesis).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    let mut deployment = deployment_memory();
    deployment
        .execute(
            "UPDATE pale_deployment_control_store_binding SET workspace_id='wrong'",
            [],
        )
        .unwrap();
    let mut custody = SqliteDeploymentControlStoreGenesisCustody {
        connection: &mut deployment,
        coordinate: genesis.binding().store().clone(),
        maximum_record_bytes: 1024,
    };
    assert_eq!(
        custody.observe_exact(&genesis).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    let mut deployment = deployment_memory();
    let mut custody = SqliteDeploymentControlStoreGenesisCustody {
        connection: &mut deployment,
        coordinate: genesis.binding().store().clone(),
        maximum_record_bytes: 1,
    };
    assert_eq!(
        custody.compare_and_commit(&genesis).unwrap_err().kind(),
        ControlStoreErrorKind::BoundExceeded
    );
}

#[test]
fn orphan_closure_material_requires_recovery() {
    let candidate = finalization("receipt", "finalization-replay");
    let mut connection = principal_memory();
    connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    connection.execute("INSERT INTO pale_principal_deployment_finalization_mutation (admin_authority_ref,admin_authority_sha256,idempotency_key,tree_kind,position_kind,position_key,predecessor_node_bytes,successor_node_bytes) VALUES ('admin-authority',?1,'finalization-replay','state','leaf',?2,NULL,?3)",rusqlite::params![hash('a'),vec![0_u8;33],vec![1_u8;32]]).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    let mut custody = SqliteDeploymentFinalizationCustody {
        connection: &mut connection,
        maximum_record_bytes: 1024,
    };
    assert_eq!(
        custody.observe_exact(&candidate).unwrap(),
        DeploymentFinalizationDatabaseObservation::ConflictingOrPartial
    );
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );

    let capability = anchored();
    let candidate = genesis(&capability, 1);
    let mut connection = deployment_memory();
    connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    connection.execute("INSERT INTO pale_deployment_control_store_genesis_checkpoint (genesis_id,checkpoint_ordinal,checkpoint_domain_id,domain_epoch,domain_sequence,checkpoint_ref,checkpoint_sha256,checkpoint_profile_ref,checkpoint_profile_sha256,state_root_sha256,unresolved_root_sha256,mutation_count,checkpoint_body_sha256) VALUES ('deployment-genesis-1',0,'orphan',1,1,'orphan',?1,'profile',?2,?3,?3,0,?4)",rusqlite::params![hash('1'),hash('2'),hash('3'),hash('4')]).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    let mut custody = SqliteDeploymentControlStoreGenesisCustody {
        connection: &mut connection,
        coordinate: candidate.binding().store().clone(),
        maximum_record_bytes: 1024,
    };
    assert_eq!(
        custody.observe_exact(&candidate).unwrap(),
        DeploymentControlStoreGenesisDatabaseObservation::ConflictingOrPartial
    );
    assert_eq!(
        custody.compare_and_commit(&candidate).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

struct AmbiguousFinalization<'connection> {
    inner: SqliteDeploymentFinalizationCustody<'connection>,
}
impl DeploymentFinalizationCustodyPort for AmbiguousFinalization<'_> {
    fn compare_and_commit(
        &mut self,
        candidate: &PreparedDeploymentFinalizationAtom,
    ) -> Result<DeploymentFinalizationCommitDisposition, ControlStoreError> {
        self.inner.compare_and_commit(candidate)?;
        Err(ControlStoreError::new(
            ControlStoreErrorKind::OutcomeUnknown,
        ))
    }
    fn observe_exact(
        &mut self,
        candidate: &PreparedDeploymentFinalizationAtom,
    ) -> Result<DeploymentFinalizationDatabaseObservation, ControlStoreError> {
        self.inner.observe_exact(candidate)
    }
}

struct AmbiguousGenesis<'connection> {
    inner: SqliteDeploymentControlStoreGenesisCustody<'connection>,
}
impl DeploymentControlStoreGenesisCustodyPort for AmbiguousGenesis<'_> {
    fn compare_and_commit(
        &mut self,
        candidate: &PreparedDeploymentControlStoreGenesisAtom,
    ) -> Result<DeploymentControlStoreGenesisCommitDisposition, ControlStoreError> {
        self.inner.compare_and_commit(candidate)?;
        Err(ControlStoreError::new(
            ControlStoreErrorKind::OutcomeUnknown,
        ))
    }
    fn observe_exact(
        &mut self,
        candidate: &PreparedDeploymentControlStoreGenesisAtom,
    ) -> Result<DeploymentControlStoreGenesisDatabaseObservation, ControlStoreError> {
        self.inner.observe_exact(candidate)
    }
}

#[test]
fn post_durable_ambiguity_recovers_both_exact_commits() {
    let candidate = finalization("receipt", "finalization-replay");
    let mut principal = principal_memory();
    let mut custody = AmbiguousFinalization {
        inner: SqliteDeploymentFinalizationCustody {
            connection: &mut principal,
            maximum_record_bytes: 1024,
        },
    };
    let committed = candidate.commit(&mut custody).unwrap();
    assert_eq!(
        committed.evidence(),
        DeploymentFinalizationCommitEvidence::RecoveredExactAfterAmbiguousCommit
    );

    let capability = anchored();
    let candidate = genesis(&capability, 1);
    let mut deployment = deployment_memory();
    let mut custody = AmbiguousGenesis {
        inner: SqliteDeploymentControlStoreGenesisCustody {
            connection: &mut deployment,
            coordinate: candidate.binding().store().clone(),
            maximum_record_bytes: 1024,
        },
    };
    let committed = candidate.commit(&mut custody).unwrap();
    assert_eq!(
        committed.evidence(),
        DeploymentControlStoreGenesisCommitEvidence::RecoveredExactAfterAmbiguousCommit
    );
}

fn race_finalization(
    path: &std::path::Path,
    candidate: PreparedDeploymentFinalizationAtom,
    barrier: Arc<Barrier>,
) -> Result<DeploymentFinalizationCommitDisposition, ControlStoreErrorKind> {
    let mut connection = Connection::open(path).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    connection.busy_timeout(Duration::from_secs(5)).unwrap();
    let mut custody = SqliteDeploymentFinalizationCustody {
        connection: &mut connection,
        maximum_record_bytes: 1024,
    };
    barrier.wait();
    custody
        .compare_and_commit(&candidate)
        .map_err(|error| error.kind())
}

fn race_genesis(
    path: &std::path::Path,
    candidate: PreparedDeploymentControlStoreGenesisAtom,
    barrier: Arc<Barrier>,
) -> Result<DeploymentControlStoreGenesisCommitDisposition, ControlStoreErrorKind> {
    let mut connection = Connection::open(path).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    connection.busy_timeout(Duration::from_secs(5)).unwrap();
    let coordinate = candidate.binding().store().clone();
    let mut custody = SqliteDeploymentControlStoreGenesisCustody {
        connection: &mut connection,
        coordinate,
        maximum_record_bytes: 1024,
    };
    barrier.wait();
    custody
        .compare_and_commit(&candidate)
        .map_err(|error| error.kind())
}

#[test]
fn same_and_changed_candidate_races_serialize_exactly() {
    for changed in [false, true] {
        let file = tempfile::NamedTempFile::new().unwrap();
        drop(initialize_principal(Connection::open(file.path()).unwrap()));
        let first = finalization("receipt", "finalization-replay");
        let second = finalization(
            if changed { "changed" } else { "receipt" },
            "finalization-replay",
        );
        let barrier = Arc::new(Barrier::new(2));
        let results = std::thread::scope(|scope| {
            let left = scope.spawn({
                let barrier = Arc::clone(&barrier);
                || race_finalization(file.path(), first, barrier)
            });
            let right = scope.spawn({
                let barrier = Arc::clone(&barrier);
                || race_finalization(file.path(), second, barrier)
            });
            [left.join().unwrap(), right.join().unwrap()]
        });
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, Ok(DeploymentFinalizationCommitDisposition::New)))
                .count(),
            1
        );
        if changed {
            assert_eq!(
                results
                    .iter()
                    .filter(|result| matches!(
                        result,
                        Err(ControlStoreErrorKind::CandidateConflict)
                    ))
                    .count(),
                1
            );
        } else {
            assert_eq!(
                results
                    .iter()
                    .filter(|result| matches!(
                        result,
                        Ok(DeploymentFinalizationCommitDisposition::ExactReplay)
                    ))
                    .count(),
                1
            );
        }
    }

    let capability = anchored();
    for changed in [false, true] {
        let file = tempfile::NamedTempFile::new().unwrap();
        drop(initialize_deployment(
            Connection::open(file.path()).unwrap(),
        ));
        let first = genesis(&capability, 1);
        let second =
            genesis_with_receipt(&capability, 1, if changed { "changed" } else { "receipt" });
        let barrier = Arc::new(Barrier::new(2));
        let results = std::thread::scope(|scope| {
            let left = scope.spawn({
                let barrier = Arc::clone(&barrier);
                || race_genesis(file.path(), first, barrier)
            });
            let right = scope.spawn({
                let barrier = Arc::clone(&barrier);
                || race_genesis(file.path(), second, barrier)
            });
            [left.join().unwrap(), right.join().unwrap()]
        });
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(
                    result,
                    Ok(DeploymentControlStoreGenesisCommitDisposition::New)
                ))
                .count(),
            1
        );
        if changed {
            assert_eq!(
                results
                    .iter()
                    .filter(|result| matches!(
                        result,
                        Err(ControlStoreErrorKind::CandidateConflict)
                    ))
                    .count(),
                1
            );
        } else {
            assert_eq!(
                results
                    .iter()
                    .filter(|result| matches!(
                        result,
                        Ok(DeploymentControlStoreGenesisCommitDisposition::ExactReplay)
                    ))
                    .count(),
                1
            );
        }
    }
}
