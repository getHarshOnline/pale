//! Pre-separation initial-v1 stores are refused intact, never reinterpreted.

use super::*;

const PRE_SEPARATION_MANIFEST: &str =
    "1a76d941f8310fa4010c435648b8a9500ee3a0482fffb6cc85754a9cca1339ed";
const PRE_SEPARATION_DEPLOYMENT_SCHEMA: &str =
    "1b219ec15ab1759114c372d2e2e054c22a4877828ee42d8f0d32d9ef8b32e6e6";
const PRE_SEPARATION_DEPLOYMENT_MIGRATION: &str =
    "ec5d3144812dc83adbd7eb6a0b61285fdd44c7452e778384e8ed53bd245e2363";

#[test]
fn pre_separation_stores_are_refused_without_changing_saved_history() {
    for (kind, suffix) in [
        (ControlStoreKind::PrincipalTrust, "old-trust"),
        (ControlStoreKind::DeploymentControl, "old-deployment"),
    ] {
        let directory = support::private_tempdir();
        let (location, identity, _) = support::initialize(&directory, kind, suffix);
        let payloads = [
            b"retained authority payload".as_slice(),
            b"retained collection payload".as_slice(),
        ];
        {
            let connection = Connection::open(location.path()).unwrap();
            if kind == ControlStoreKind::DeploymentControl {
                // Reconstruct the exact previous schema in this scratch fixture.
                // Its known resultant digest guards against approximating an old store.
                connection
                    .execute_batch(
                        "DROP TABLE pale_deployment_collection_head;
                     DROP TABLE pale_deployment_collection_journal;",
                    )
                    .unwrap();
                assert_eq!(
                    crate::schema::resultant_schema_sha256(&connection).unwrap(),
                    PRE_SEPARATION_DEPLOYMENT_SCHEMA
                );
                connection.execute(
                    "UPDATE pale_control_store_header SET store_schema_sha256=?1, migration_head_sha256=?2 WHERE singleton=1",
                    params![PRE_SEPARATION_DEPLOYMENT_SCHEMA, PRE_SEPARATION_DEPLOYMENT_MIGRATION],
                ).unwrap();
                connection.execute(
                    "UPDATE pale_control_store_migration_receipt SET migration_sha256=?1 WHERE migration_ordinal=1",
                    [PRE_SEPARATION_DEPLOYMENT_MIGRATION],
                ).unwrap();
                // Physical custody is opaque here. These two different retained
                // payloads model the old mixed journal, not semantic admission.
                for (offset, payload) in payloads.iter().enumerate() {
                    let frame =
                        crate::frame::encode(payload, pale_ports::AuthorityByteClass::JournalEntry)
                            .unwrap();
                    connection.execute(
                        "INSERT INTO pale_deployment_authority_journal (journal_sequence,physical_frame) VALUES (?1,?2)",
                        params![i64::try_from(offset+1).unwrap(), frame],
                    ).unwrap();
                }
                connection.execute("UPDATE pale_deployment_authority_head SET journal_sequence=2 WHERE singleton=1", []).unwrap();
            }
            connection.execute(
                "UPDATE pale_control_store_header SET storage_manifest_sha256=?1 WHERE singleton=1",
                [PRE_SEPARATION_MANIFEST],
            ).unwrap();
            connection
                .execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
                .unwrap();
        }
        let before = fs::read(location.path()).unwrap();
        assert_eq!(
            classify_store(&location, &identity).unwrap(),
            StoreOpenClassification::Rejected(StoreOpenRejection::SourceIdentityMismatch)
        );
        assert!(initialize_unadmitted_store(&location, identity.clone()).is_err());
        if kind == ControlStoreKind::DeploymentControl {
            let coordinate = pale_ports::AuthorityStoreCoordinate::new(
                identity.logical_store_id().clone(),
                identity.control_incarnation_id().clone(),
            );
            assert!(matches!(
                crate::authority::DeploymentControlSqliteStore::open_unadmitted_candidate(
                    &location, coordinate
                ),
                Err(ControlSqliteError::SourceInvariant(
                    "deployment authority candidate classification"
                )),
            ));
            let connection =
                Connection::open_with_flags(location.path(), OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .unwrap();
            for (offset, expected) in payloads.iter().enumerate() {
                let frame: Vec<u8> = connection.query_row(
                    "SELECT physical_frame FROM pale_deployment_authority_journal WHERE journal_sequence=?1",
                    [i64::try_from(offset+1).unwrap()], |row| row.get(0),
                ).unwrap();
                assert_eq!(
                    crate::frame::decode(&frame, 64, pale_ports::AuthorityByteClass::JournalEntry)
                        .unwrap(),
                    *expected
                );
            }
            assert_eq!(
                crate::schema::resultant_schema_sha256(&connection).unwrap(),
                PRE_SEPARATION_DEPLOYMENT_SCHEMA
            );
        }
        assert_eq!(fs::read(location.path()).unwrap(), before);
    }
}
