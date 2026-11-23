//! Native principal trust reads, using the existing producer custody fixture.

use super::*;
use crate::SqlitePrincipalTrustReader;
use pale_ports::{
    PrincipalControlCheckpointChainReadLimits, PrincipalControlCheckpointChainReadPort,
    PrincipalControlCheckpointChainSnapshot, PrincipalTrustCheckpointSetReadLimits,
    PrincipalTrustCheckpointSetReadPort, PrincipalTrustCheckpointSetSnapshot,
    PrincipalTrustReadPort,
};

fn control_checkpoint_chain(
    connection: &mut Connection,
    maximum_checkpoints: usize,
    maximum_bytes: usize,
) -> Result<Option<PrincipalControlCheckpointChainSnapshot>, ControlStoreError> {
    SqlitePrincipalTrustReader { connection }.read_control_checkpoint_chain(
        &binding(),
        &checkpoint(1),
        PrincipalControlCheckpointChainReadLimits::new(maximum_checkpoints, maximum_bytes),
    )
}

fn read(
    connection: &mut Connection,
    maximum_bytes: usize,
) -> Result<Option<PrincipalTrustSnapshot>, ControlStoreError> {
    SqlitePrincipalTrustReader { connection }.read_current_checkpoint(&binding(), maximum_bytes)
}

fn recovery(result: Result<Option<PrincipalTrustSnapshot>, ControlStoreError>) {
    assert_eq!(
        result.unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

fn bound(result: Result<Option<PrincipalTrustSnapshot>, ControlStoreError>) {
    assert_eq!(
        result.unwrap_err().kind(),
        ControlStoreErrorKind::BoundExceeded
    );
}

fn checkpoint_set(
    connection: &mut Connection,
    maximum_checkpoints: usize,
    maximum_bytes: usize,
) -> Result<Option<PrincipalTrustCheckpointSetSnapshot>, ControlStoreError> {
    SqlitePrincipalTrustReader { connection }.read_checkpoint_set(
        &binding(),
        PrincipalTrustCheckpointSetReadLimits::new(maximum_checkpoints, maximum_bytes),
    )
}

#[test]
fn trust_discovery_returns_exact_native_head_body_and_releases_transaction() {
    let mut connection = initialize_store();
    connection.pragma_update(None, "query_only", true).unwrap();
    let changes = connection.total_changes();
    let snapshot = read(&mut connection, 1024).unwrap().unwrap();
    assert_eq!(snapshot.binding(), &binding());
    assert_eq!(snapshot.through(), &trust_head(1));
    assert_eq!(snapshot.checkpoint_record(), &record("trust-1", '1'));
    assert!(connection.is_autocommit());
    assert_eq!(connection.total_changes(), changes);
    assert_eq!(
        crate::store::transaction::read(
            &connection,
            &crate::principal_trust::store_coordinate(&binding())
        )
        .unwrap()
        .get(),
        21
    );
}

#[test]
fn trust_discovery_rejects_each_wrong_binding_without_writes() {
    let mut connection = initialize_store();
    let changes = connection.total_changes();
    for (store, incarnation, principal) in [
        ("other", "trust-incarnation-1", "principal-1"),
        ("trust-store-1", "other", "principal-1"),
        ("trust-store-1", "trust-incarnation-1", "other"),
    ] {
        let binding = PrincipalTrustStoreBinding::new(
            ControlStoreIdV1::parse(store).unwrap(),
            ControlIncarnationIdV1::parse(incarnation).unwrap(),
            ResidentPrincipalIdV1::parse(principal).unwrap(),
        );
        recovery(
            SqlitePrincipalTrustReader {
                connection: &mut connection,
            }
            .read_current_checkpoint(&binding, 1024),
        );
        assert!(connection.is_autocommit());
    }
    assert_eq!(connection.total_changes(), changes);
}

#[test]
fn trust_discovery_distinguishes_no_head_from_unresolved_head() {
    let mut connection = initialize_store();
    connection
        .execute("DELETE FROM pale_principal_trust_checkpoint_head", [])
        .unwrap();
    assert_eq!(read(&mut connection, 1024).unwrap(), None);

    let mut connection = initialize_store();
    connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    connection
        .execute(
            "DELETE FROM pale_principal_trust_record WHERE record_ref = 'trust-1'",
            [],
        )
        .unwrap();
    recovery(read(&mut connection, 1024));
    assert!(connection.is_autocommit());
}

#[test]
fn trust_discovery_checks_head_metadata_before_copying_text() {
    for statement in [
        "UPDATE pale_principal_trust_checkpoint_head SET checkpoint_epoch = 0",
        "UPDATE pale_principal_trust_checkpoint_head SET principal_id = 'other'",
        "UPDATE pale_principal_trust_checkpoint_head SET trust_checkpoint_ref = printf('%0513d', 0)",
        "UPDATE pale_principal_trust_checkpoint_head SET trust_checkpoint_ref = CAST(x'ff' AS TEXT)",
        "UPDATE pale_principal_trust_checkpoint_head SET trust_checkpoint_sha256 = 'bad'",
    ] {
        let mut connection = initialize_store();
        connection.pragma_update(None, "foreign_keys", false).unwrap();
        connection.pragma_update(None, "ignore_check_constraints", true).unwrap();
        connection.execute(statement, []).unwrap();
        recovery(read(&mut connection, 1024));
        assert!(connection.is_autocommit());
    }
}

#[test]
fn trust_discovery_rejects_corrupt_record_metadata_and_invalid_stamps() {
    for statement in [
        "UPDATE pale_principal_trust_record SET record_sha256 = 'bad' WHERE record_ref = 'trust-1'",
        "UPDATE pale_principal_trust_record SET schema_id = printf('%0513d', 0) WHERE record_ref = 'trust-1'",
        "UPDATE pale_principal_trust_record SET schema_id = CAST(x'ff' AS TEXT) WHERE record_ref = 'trust-1'",
        "UPDATE pale_principal_trust_record SET schema_sha256 = 'bad' WHERE record_ref = 'trust-1'",
        "UPDATE pale_principal_trust_record SET schema_id = 'pale/producer-incarnation.v1' WHERE record_ref = 'trust-1'",
        "UPDATE pale_principal_trust_record SET canonical_record_jcs = x'' WHERE record_ref = 'trust-1'",
        "UPDATE pale_principal_trust_record SET principal_id = 'other' WHERE record_ref = 'trust-1'",
        "UPDATE pale_principal_trust_record SET inserted_transaction_sequence = 0 WHERE record_ref = 'trust-1'",
        "UPDATE pale_principal_trust_record SET inserted_transaction_sequence = 22 WHERE record_ref = 'trust-1'",
        "UPDATE pale_control_store_header SET logical_transaction_sequence = 0",
        "UPDATE pale_control_store_header SET store_kind = 'deployment_control'",
    ] {
        let mut connection = initialize_store();
        connection.pragma_update(None, "foreign_keys", false).unwrap();
        connection.pragma_update(None, "ignore_check_constraints", true).unwrap();
        connection.execute(statement, []).unwrap();
        recovery(read(&mut connection, 1024));
        assert!(connection.is_autocommit());
    }
}

#[test]
fn trust_discovery_enforces_body_copy_limit_at_the_exact_boundary() {
    let mut connection = initialize_store();
    let expected = record("trust-1", '1');
    let bytes = expected.canonical_record_jcs().as_slice().len();
    assert_eq!(
        read(&mut connection, bytes)
            .unwrap()
            .unwrap()
            .checkpoint_record(),
        &expected
    );
    bound(read(&mut connection, bytes - 1));
    bound(read(&mut connection, 0));
    connection.execute("UPDATE pale_principal_trust_record SET canonical_record_jcs = zeroblob(65536) WHERE record_ref = 'trust-1'", []).unwrap();
    bound(read(&mut connection, 1024));
    assert!(connection.is_autocommit());
}

#[test]
fn trust_discovery_does_not_require_an_unused_transaction_successor() {
    let mut connection = initialize_store();
    connection
        .execute(
            "UPDATE pale_control_store_header SET logical_transaction_sequence = 9007199254740991",
            [],
        )
        .unwrap();
    let changes = connection.total_changes();
    let snapshot = read(&mut connection, 1024).unwrap().unwrap();
    assert_eq!(snapshot.through(), &trust_head(1));
    assert_eq!(
        crate::store::transaction::record_stamp(
            &connection,
            crate::ControlStoreKind::PrincipalTrust,
            snapshot.through().coordinate()
        )
        .unwrap()
        .unwrap()
        .get(),
        21
    );
    assert_eq!(connection.total_changes(), changes);
}

#[test]
fn trust_discovery_preserves_unadmitted_bytes_without_fabricating_hash_admission() {
    let mut connection = initialize_store();
    // Signed record coordinates are not hashes of the full wire body. The
    // physical reader cannot claim signature/schema admission from this row.
    let changed = b"not an admitted signed checkpoint";
    connection.execute("UPDATE pale_principal_trust_record SET canonical_record_jcs = ?1 WHERE record_ref = 'trust-1'", [changed.as_slice()]).unwrap();
    let snapshot = read(&mut connection, changed.len()).unwrap().unwrap();
    assert_eq!(snapshot.through(), &trust_head(1));
    assert_eq!(
        snapshot
            .checkpoint_record()
            .canonical_record_jcs()
            .as_slice(),
        changed
    );
    assert_ne!(snapshot.checkpoint_record(), &record("trust-1", '1'));
}

#[test]
fn trust_discovery_retains_a_coherent_wal_snapshot_during_a_real_writer_commit() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("trust.sqlite");
    let mut reader = initialize_connection(Connection::open(&path).unwrap());
    reader.pragma_update(None, "journal_mode", "WAL").unwrap();
    let mut writer = Connection::open(&path).unwrap();
    writer.pragma_update(None, "foreign_keys", true).unwrap();
    let transaction = reader
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .unwrap();
    let before =
        crate::principal_trust::read::read_current_checkpoint_in(&transaction, &binding(), 1024)
            .unwrap()
            .unwrap();
    commit(&mut writer, &initial_candidate());
    let retained =
        crate::principal_trust::read::read_current_checkpoint_in(&transaction, &binding(), 1024)
            .unwrap()
            .unwrap();
    assert_eq!(retained, before);
    assert_eq!(retained.through(), &trust_head(1));
    transaction.rollback().unwrap();
    let current = read(&mut reader, 1024).unwrap().unwrap();
    assert_eq!(current.through(), &trust_head(2));
    assert_eq!(current.checkpoint_record(), &record("trust-2", '2'));
    let stale_request = PrincipalProducerIncarnationHeadPageRead::new(before, None, 1).unwrap();
    assert_eq!(
        SqlitePrincipalProducerIncarnationCustody {
            connection: &mut reader
        }
        .read_heads_page(&stale_request)
        .unwrap_err()
        .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

#[test]
fn checkpoint_set_read_copies_the_exact_complete_set_without_writes() {
    let mut connection = initialize_store();
    commit(&mut connection, &initial_candidate());
    let changes = connection.total_changes();
    connection.pragma_update(None, "query_only", true).unwrap();
    let checkpoint_set = checkpoint_set(&mut connection, 2, 1024).unwrap().unwrap();
    assert_eq!(checkpoint_set.through(), &trust_head(2));
    assert_eq!(
        checkpoint_set.current_checkpoint_record().record(),
        &record("trust-2", '2')
    );
    assert_eq!(checkpoint_set.observed_through().get(), 22);
    assert_eq!(
        checkpoint_set
            .checkpoint_records()
            .iter()
            .map(|record| record.record().coordinate().clone())
            .collect::<Vec<_>>(),
        vec![coordinate("trust-1", '1'), coordinate("trust-2", '2')]
    );
    assert_eq!(
        checkpoint_set
            .checkpoint_records()
            .iter()
            .map(|record| record.inserted_in().get())
            .collect::<Vec<_>>(),
        vec![21, 22]
    );
    assert!(connection.is_autocommit());
    assert_eq!(connection.total_changes(), changes);
}

#[test]
fn checkpoint_set_read_applies_count_and_byte_bounds_before_returning_rows() {
    let mut connection = initialize_store();
    commit(&mut connection, &initial_candidate());
    assert_eq!(
        checkpoint_set(&mut connection, 1, 1024).unwrap_err().kind(),
        ControlStoreErrorKind::BoundExceeded
    );
    let exact_bytes = record("trust-1", '1')
        .canonical_record_jcs()
        .as_slice()
        .len()
        + record("trust-2", '2')
            .canonical_record_jcs()
            .as_slice()
            .len();
    assert_eq!(
        checkpoint_set(&mut connection, 2, exact_bytes - 1)
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::BoundExceeded
    );
    assert_eq!(
        checkpoint_set(&mut connection, 2, exact_bytes)
            .unwrap()
            .unwrap()
            .checkpoint_records()
            .len(),
        2
    );
}

#[test]
fn checkpoint_set_read_rejects_orphaned_and_cross_principal_rows() {
    let mut connection = initialize_store();
    connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    connection
        .execute("DELETE FROM pale_principal_trust_checkpoint_head", [])
        .unwrap();
    connection
        .execute("DELETE FROM pale_principal_trust_record", [])
        .unwrap();
    assert_eq!(checkpoint_set(&mut connection, 2, 1024).unwrap(), None);

    let mut connection = initialize_store();
    connection
        .execute("DELETE FROM pale_principal_trust_checkpoint_head", [])
        .unwrap();
    assert_eq!(
        checkpoint_set(&mut connection, 2, 1024).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );

    let mut connection = initialize_store();
    commit(&mut connection, &initial_candidate());
    connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    connection
        .execute(
            "UPDATE pale_principal_trust_record SET principal_id='other' WHERE record_ref='trust-1'",
            [],
        )
        .unwrap();
    assert_eq!(
        checkpoint_set(&mut connection, 2, 1024).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );

    let mut connection = initialize_store();
    commit(&mut connection, &initial_candidate());
    connection
        .pragma_update(None, "ignore_check_constraints", true)
        .unwrap();
    connection
        .execute(
            "UPDATE pale_principal_trust_record SET inserted_transaction_sequence=0 WHERE record_ref='trust-1'",
            [],
        )
        .unwrap();
    assert_eq!(
        checkpoint_set(&mut connection, 2, 1024).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
}

#[test]
fn control_checkpoint_chain_read_returns_complete_ordered_domain_without_writes() {
    let mut connection = initialize_store();
    commit(&mut connection, &initial_candidate());
    let changes = connection.total_changes();
    connection.pragma_update(None, "query_only", true).unwrap();
    let snapshot = control_checkpoint_chain(&mut connection, 4, 4096)
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.initial(), &checkpoint(1));
    assert_eq!(snapshot.through(), &checkpoint(2));
    assert_eq!(snapshot.observed_through().get(), 22);
    assert_eq!(
        snapshot
            .records()
            .iter()
            .map(|record| record.record().coordinate().clone())
            .collect::<Vec<_>>(),
        vec![coordinate("control-1", '5'), coordinate("control-2", '6')]
    );
    assert!(connection.is_autocommit());
    assert_eq!(connection.total_changes(), changes);
}

#[test]
fn control_checkpoint_chain_read_distinguishes_absence_partial_and_bounds() {
    let mut connection = initialize_store();
    connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    connection
        .execute("DELETE FROM pale_principal_control_checkpoint_head", [])
        .unwrap();
    connection
        .execute(
            "DELETE FROM pale_principal_trust_record WHERE record_ref='control-1'",
            [],
        )
        .unwrap();
    assert_eq!(
        control_checkpoint_chain(&mut connection, 2, 1024).unwrap(),
        None
    );

    let mut connection = initialize_store();
    connection
        .execute("DELETE FROM pale_principal_control_checkpoint_head", [])
        .unwrap();
    assert_eq!(
        control_checkpoint_chain(&mut connection, 2, 1024)
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::RecoveryRequired
    );

    let mut connection = initialize_store();
    assert_eq!(
        control_checkpoint_chain(&mut connection, 0, 1024)
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::BoundExceeded
    );
    assert_eq!(
        control_checkpoint_chain(&mut connection, 2, 1)
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::BoundExceeded
    );
}

#[test]
fn control_checkpoint_chain_read_rejects_unclassifiable_and_future_rows() {
    for statement in [
        "UPDATE pale_principal_trust_record SET canonical_record_jcs=x'ff' WHERE record_ref='control-1'",
        "UPDATE pale_principal_trust_record SET canonical_record_jcs=CAST('{}' AS BLOB) WHERE record_ref='control-1'",
        "UPDATE pale_principal_trust_record SET canonical_record_jcs=CAST('{\"checkpoint_domain_id\":null}' AS BLOB) WHERE record_ref='control-1'",
        "UPDATE pale_principal_trust_record SET canonical_record_jcs=CAST('{\"checkpoint_domain_id\":1}' AS BLOB) WHERE record_ref='control-1'",
        "UPDATE pale_principal_trust_record SET inserted_transaction_sequence=22 WHERE record_ref='control-1'",
        "UPDATE pale_principal_control_checkpoint_head SET control_checkpoint_ref=printf('%0513d',0) WHERE checkpoint_domain_id='producer:producer-1'",
        "UPDATE pale_principal_control_checkpoint_head SET control_checkpoint_ref=CAST(x'ff' AS TEXT) WHERE checkpoint_domain_id='producer:producer-1'",
        "UPDATE pale_principal_control_checkpoint_head SET domain_sequence=2 WHERE checkpoint_domain_id='producer:producer-1'",
        "UPDATE pale_principal_control_checkpoint_head SET control_checkpoint_sha256='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' WHERE checkpoint_domain_id='producer:producer-1'",
    ] {
        let mut connection = initialize_store();
        connection.pragma_update(None, "foreign_keys", false).unwrap();
        connection
            .pragma_update(None, "ignore_check_constraints", true)
            .unwrap();
        connection.execute(statement, []).unwrap();
        assert_eq!(
            control_checkpoint_chain(&mut connection, 2, 1024)
                .unwrap_err()
                .kind(),
            ControlStoreErrorKind::RecoveryRequired
        );
    }
}
