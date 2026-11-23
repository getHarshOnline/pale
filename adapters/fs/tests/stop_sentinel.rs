//! Behavioral proof for confined service-manager stop-sentinel custody.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};

use pale_adapter_fs::{StopSentinelCustody, StopSentinelDisposition, StopSentinelObservation};

/// Fixture-relative deployment coordinate; it carries no authority by itself.
const SENTINEL: &str = "supervisor/stopped";

fn custody(root: &std::path::Path) -> StopSentinelCustody {
    fs::create_dir_all(root.join("supervisor")).expect("create fixture instance directory");
    StopSentinelCustody::new(root, SENTINEL).expect("bind confined stop sentinel")
}

#[test]
fn present_and_absent_transitions_are_idempotent() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let sentinel = custody(temporary.path());

    assert_eq!(sentinel.observe().unwrap(), StopSentinelObservation::Absent);
    assert_eq!(
        sentinel.ensure_absent().unwrap(),
        StopSentinelDisposition::AlreadyAbsent
    );
    assert_eq!(
        sentinel.ensure_present().unwrap(),
        StopSentinelDisposition::Created
    );
    assert_eq!(
        fs::metadata(temporary.path().join(SENTINEL))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o600
    );
    assert_eq!(
        sentinel.ensure_present().unwrap(),
        StopSentinelDisposition::AlreadyPresent
    );
    assert_eq!(
        sentinel.observe().unwrap(),
        StopSentinelObservation::Present
    );
    assert_eq!(
        sentinel.ensure_absent().unwrap(),
        StopSentinelDisposition::Removed
    );
    assert_eq!(
        sentinel.ensure_absent().unwrap(),
        StopSentinelDisposition::AlreadyAbsent
    );
    assert_eq!(sentinel.observe().unwrap(), StopSentinelObservation::Absent);
}

#[test]
fn a_published_stop_guard_survives_the_boundary_before_later_directives() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let sentinel = custody(temporary.path());

    assert_eq!(
        sentinel.ensure_present().unwrap(),
        StopSentinelDisposition::Created
    );
    drop(sentinel);

    let reopened = custody(temporary.path());
    assert_eq!(
        reopened.observe().unwrap(),
        StopSentinelObservation::Present,
        "dropping the adapter before any process directive must not roll back the durable guard"
    );
}

#[test]
fn a_foreign_file_is_neither_adopted_nor_removed() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let path = temporary.path().join(SENTINEL);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"owned elsewhere\n").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let sentinel = StopSentinelCustody::new(temporary.path(), SENTINEL).unwrap();

    assert!(sentinel.observe().is_err());
    assert!(sentinel.ensure_present().is_err());
    assert!(sentinel.ensure_absent().is_err());
    assert_eq!(fs::read(&path).unwrap(), b"owned elsewhere\n");
}

#[test]
fn a_marker_with_foreign_permissions_is_preserved() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let sentinel = custody(temporary.path());
    sentinel.ensure_present().unwrap();
    let path = temporary.path().join(SENTINEL);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();

    assert!(sentinel.observe().is_err());
    assert!(sentinel.ensure_absent().is_err());
    assert!(path.exists(), "mode drift must not authorize removal");
}

#[test]
fn a_symlink_leaf_is_rejected_without_touching_its_target() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let outside = tempfile::tempdir().expect("outside root");
    let target = outside.path().join("target");
    fs::write(&target, b"outside\n").unwrap();
    let path = temporary.path().join(SENTINEL);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    symlink(&target, &path).unwrap();
    let sentinel = StopSentinelCustody::new(temporary.path(), SENTINEL).unwrap();

    assert!(sentinel.observe().is_err());
    assert!(sentinel.ensure_present().is_err());
    assert!(sentinel.ensure_absent().is_err());
    assert_eq!(fs::read(&target).unwrap(), b"outside\n");
    assert!(fs::symlink_metadata(&path)
        .unwrap()
        .file_type()
        .is_symlink());
}

#[test]
fn a_symlink_ancestor_cannot_redirect_the_sentinel() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let outside = tempfile::tempdir().expect("outside root");
    symlink(outside.path(), temporary.path().join("supervisor")).unwrap();

    assert!(StopSentinelCustody::new(temporary.path(), SENTINEL).is_err());
    assert!(fs::read_dir(outside.path()).unwrap().next().is_none());
}

#[test]
fn a_replaced_root_path_does_not_rebind_custody() {
    let temporary = tempfile::tempdir().expect("temporary parent");
    let root = temporary.path().join("instance");
    let moved = temporary.path().join("moved-instance");
    fs::create_dir_all(root.join("supervisor")).unwrap();
    let sentinel = StopSentinelCustody::new(&root, SENTINEL).unwrap();

    fs::rename(&root, &moved).unwrap();
    fs::create_dir_all(root.join("supervisor")).unwrap();

    assert!(sentinel.ensure_present().is_err());
    assert!(!root.join(SENTINEL).exists());
    assert!(!moved.join(SENTINEL).exists());
}
