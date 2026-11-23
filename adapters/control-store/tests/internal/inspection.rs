//! Behavioral proof for the public read-only control-store inspection seam.

use super::*;
use crate::test_support as support;
use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

fn request(directory: &tempfile::TempDir, suffix: &str) -> ControlStoreInspectionRequest {
    let identity = support::identity(ControlStoreKind::PrincipalTrust, suffix);
    let principal_id = match identity.scope() {
        ControlStoreScope::Principal(scope) => scope.principal_id().clone(),
        ControlStoreScope::Deployment(_) => unreachable!("principal fixture scope"),
    };
    ControlStoreInspectionRequest::principal(
        directory.path().to_path_buf(),
        principal_id,
        identity.logical_store_id().clone(),
        identity.control_incarnation_id().clone(),
    )
}

fn sidecar(main: &Path, suffix: &str) -> PathBuf {
    let mut path = OsString::from(main.as_os_str());
    path.push(suffix);
    PathBuf::from(path)
}

fn optional_bytes(path: &Path) -> Option<Vec<u8>> {
    path.exists().then(|| std::fs::read(path).unwrap())
}

#[test]
fn missing_inspection_creates_nothing() {
    let directory = support::private_tempdir();
    let observed = inspect_control_store(request(&directory, "missing")).unwrap();

    assert_eq!(observed.kind(), ControlStoreKind::PrincipalTrust);
    assert_eq!(observed.state(), &ControlStoreInspectionState::Missing);
    assert!(!directory.path().join("pale").exists());
}

#[test]
fn initialized_store_retains_the_complete_ordered_unselected_profile() {
    let directory = support::private_tempdir();
    support::initialize(&directory, ControlStoreKind::PrincipalTrust, "profile");

    let observed = inspect_control_store(request(&directory, "profile")).unwrap();
    assert_eq!(
        observed.state(),
        &ControlStoreInspectionState::ProfileUnselected {
            unresolved: ControlStorageProfileRequirement::ALL.to_vec(),
        }
    );
}

#[test]
fn inspection_does_not_create_or_update_wal_sidecars() {
    let directory = support::private_tempdir();
    let (location, _, _) =
        support::initialize(&directory, ControlStoreKind::PrincipalTrust, "immutable");
    let writer = rusqlite::Connection::open(location.path()).unwrap();
    writer.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
    let sqlite_version: String = writer
        .query_row(
            "SELECT sqlite_version FROM pale_control_store_profile_observation WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    writer
        .execute(
            "UPDATE pale_control_store_profile_observation SET sqlite_version = 'inspection-probe' WHERE singleton = 1",
            [],
        )
        .unwrap();
    writer
        .execute(
            "UPDATE pale_control_store_profile_observation SET sqlite_version = ?1 WHERE singleton = 1",
            [&sqlite_version],
        )
        .unwrap();

    let main = location.path();
    let wal = sidecar(&main, "-wal");
    let shared_memory = sidecar(&main, "-shm");
    assert!(wal.exists() && shared_memory.exists());
    let before = [
        optional_bytes(&main),
        optional_bytes(&wal),
        optional_bytes(&shared_memory),
    ];

    let observed = inspect_control_store(request(&directory, "immutable")).unwrap();

    assert!(matches!(
        observed.state(),
        ControlStoreInspectionState::ProfileUnselected { .. }
    ));
    assert_eq!(
        before,
        [
            optional_bytes(&main),
            optional_bytes(&wal),
            optional_bytes(&shared_memory),
        ]
    );
    drop(writer);
}

#[test]
fn empty_store_remains_distinct_from_missing_and_rejected() {
    let directory = support::private_tempdir();
    let location = support::location(&directory, ControlStoreKind::PrincipalTrust, "empty");
    std::fs::File::create(location.path()).expect("empty candidate leaf is created");
    std::fs::set_permissions(location.path(), std::fs::Permissions::from_mode(0o600))
        .expect("empty candidate leaf is private");

    let observed = inspect_control_store(request(&directory, "empty")).unwrap();
    assert_eq!(observed.state(), &ControlStoreInspectionState::Empty);
}

#[test]
fn ready_classification_is_not_collapsed_into_profile_selection_or_rejection() {
    assert_eq!(
        ControlStoreInspectionState::from(StoreOpenClassification::Ready),
        ControlStoreInspectionState::Ready
    );
}

#[test]
fn expected_identity_mismatch_is_a_physical_rejection() {
    let directory = support::private_tempdir();
    support::initialize(&directory, ControlStoreKind::PrincipalTrust, "identity");
    let mut request = request(&directory, "identity");
    request.logical_store_id = ControlStoreIdV1::parse("different-logical-store").unwrap();

    let observed = inspect_control_store(request).unwrap();
    assert_eq!(
        observed.state(),
        &ControlStoreInspectionState::Rejected(
            ControlStoreInspectionRejection::WrongLogicalStoreId,
        )
    );
}
