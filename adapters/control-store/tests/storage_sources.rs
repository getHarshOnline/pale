//! Embedded manifest, migration, dependency, and unresolved-profile evidence.

use pale_adapter_control_storage::{
    verify_embedded_storage_sources, BackupRestorePolicyV1, ControlStorageProfileRequirement,
    ControlStoreKind,
};

#[test]
fn verifies_both_exact_initial_v1_migration_sets() {
    let sources = verify_embedded_storage_sources().unwrap();
    assert_eq!(sources.manifest_sha256().len(), 64);
    assert_eq!(sources.sqlite_version(), "3.51.3");

    let trust = sources.store(ControlStoreKind::PrincipalTrust);
    let deployment = sources.store(ControlStoreKind::DeploymentControl);
    assert_eq!(trust.kind(), ControlStoreKind::PrincipalTrust);
    assert_eq!(deployment.kind(), ControlStoreKind::DeploymentControl);
    assert_eq!(trust.application_id(), i32::from_be_bytes(*b"PALT"));
    assert_eq!(deployment.application_id(), i32::from_be_bytes(*b"PALD"));
    assert_eq!(trust.target_schema_version(), 1);
    assert_eq!(deployment.target_schema_version(), 1);
    assert_eq!(trust.migrations().len(), 1);
    assert_eq!(deployment.migrations().len(), 1);
    assert_ne!(trust.schema_sha256(), trust.migrations()[0].sha256());
    assert_ne!(
        deployment.schema_sha256(),
        deployment.migrations()[0].sha256()
    );
    assert_ne!(trust.schema_sha256(), deployment.schema_sha256());
    assert_eq!(trust.migrations()[0].id(), "0001_initial_v1");
    assert_eq!(deployment.migrations()[0].id(), "0001_initial_v1");
    assert_ne!(
        trust.migrations()[0].sha256(),
        deployment.migrations()[0].sha256()
    );

    let fixture_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support");
    assert!(fixture_root.is_dir());
    assert!(fixture_root.join("fixtures.rs").is_file());
}

#[test]
fn preserves_every_unresolved_storage_coordinate_as_unselected() {
    let sources = verify_embedded_storage_sources().unwrap();
    assert_eq!(sources.sqlite_profile().journal_mode, "wal");
    assert!(!sources.sqlite_profile().is_selected());
    assert!(!sources.authority_journal_profile().is_selected());
    assert_eq!(
        sources.backup_restore_policy(),
        BackupRestorePolicyV1::NoneInitialV1
    );
    assert!(!sources.backup_restore_policy().is_selected());
    assert!(!sources.is_runtime_profile_selected());
    assert_eq!(
        sources.unresolved_profile_requirements(),
        ControlStorageProfileRequirement::ALL.to_vec()
    );
}
