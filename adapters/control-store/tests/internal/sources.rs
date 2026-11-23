//! Source-verification tests that separate migration and resultant identities.

use super::*;

#[path = "sources/regeneration.rs"]
mod regeneration;

fn principal_store(schema_sha256: String) -> StoreManifest {
    StoreManifest {
        application_id: 1_346_456_660,
        schema_id: "pale/control-store/principal-trust.v1".to_owned(),
        schema_sha256,
        target_schema_version: 1,
    }
}

fn principal_migration(sql: &str) -> Vec<MigrationManifest> {
    vec![MigrationManifest {
        id: "0001_initial_v1".to_owned(),
        ordinal: 1,
        path: "adapters/control-store/migrations/principal-trust/0001_initial_v1.sql".to_owned(),
        sha256: sha256_hex(sql.as_bytes()),
    }]
}

#[test]
fn migration_digest_cannot_masquerade_as_resultant_schema_identity() {
    let migration_sha256 = sha256_hex(PRINCIPAL_TRUST_MIGRATION_V1.as_bytes());
    let error = verify_store_source(
        ControlStoreKind::PrincipalTrust,
        principal_store(migration_sha256),
        principal_migration(PRINCIPAL_TRUST_MIGRATION_V1),
        PRINCIPAL_TRUST_MIGRATION_V1,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ControlSqliteError::SourceInvariant("resultant physical schema hash")
    ));
}

#[test]
fn exact_migration_bytes_and_resultant_schema_are_independent_proofs() {
    const CHANGED_SCHEMA: &str = "PRAGMA application_id = 1346456660; PRAGMA user_version = 1; \
        CREATE TABLE different (id INTEGER PRIMARY KEY) STRICT;";
    let sources = verify_embedded_storage_sources().unwrap();
    let expected_schema = sources
        .store(ControlStoreKind::PrincipalTrust)
        .schema_sha256()
        .to_owned();
    let error = verify_store_source(
        ControlStoreKind::PrincipalTrust,
        principal_store(expected_schema),
        principal_migration(CHANGED_SCHEMA),
        CHANGED_SCHEMA,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ControlSqliteError::SourceInvariant("resultant physical schema hash")
    ));
}

#[test]
fn migration_must_realize_its_declared_headers() {
    const WRONG_HEADERS: &str = "PRAGMA application_id = 1; PRAGMA user_version = 9; \
        CREATE TABLE item (id INTEGER PRIMARY KEY) STRICT;";
    let error = verify_store_source(
        ControlStoreKind::PrincipalTrust,
        principal_store("0".repeat(64)),
        principal_migration(WRONG_HEADERS),
        WRONG_HEADERS,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ControlSqliteError::SourceInvariant("migration application ID")
    ));
}

#[test]
fn pale_application_ids_reject_old_brand_and_cross_store_headers() {
    const OLD_BRAND_HEADER: &str = "PRAGMA application_id = 1246904916; \
        PRAGMA user_version = 1; CREATE TABLE item (id INTEGER PRIMARY KEY) STRICT;";
    let old_brand_error = verify_store_source(
        ControlStoreKind::PrincipalTrust,
        principal_store("0".repeat(64)),
        principal_migration(OLD_BRAND_HEADER),
        OLD_BRAND_HEADER,
    )
    .unwrap_err();
    assert!(matches!(
        old_brand_error,
        ControlSqliteError::SourceInvariant("migration application ID")
    ));

    let cross_store_error = verify_store_source(
        ControlStoreKind::PrincipalTrust,
        principal_store("0".repeat(64)),
        principal_migration(DEPLOYMENT_CONTROL_MIGRATION_V1),
        DEPLOYMENT_CONTROL_MIGRATION_V1,
    )
    .unwrap_err();
    assert!(matches!(
        cross_store_error,
        ControlSqliteError::SourceInvariant("migration application ID")
    ));
}
