//! File transaction locking and exhaustive recovery boundaries.

use std::fs;
use std::time::Duration;

use crate::confine::ConfinedRoot;
use crate::errors::FsError;

#[test]
fn apply_lock_serializes_same_target_and_releases() {
    use super::acquire_apply_lock;
    use crate::lock::acquire_location;

    let tmp = tempfile::tempdir().unwrap();
    let root = ConfinedRoot::open(tmp.path()).unwrap();
    let lock = root.location(".pale/apply.lock").unwrap();
    let held = acquire_apply_lock(&root).unwrap();
    assert!(acquire_location(&lock, Duration::from_millis(50), "probe").is_err());
    drop(held);
    assert!(acquire_location(&lock, Duration::from_millis(500), "probe").is_ok());
}

#[test]
fn restore_attempts_every_backup_after_an_individual_failure() {
    use super::{backup_of, restore, Backup};

    let tmp = tempfile::tempdir().unwrap();
    let restored = tmp.path().join("restored.txt");
    let invalid_leaf = tmp.path().join("directory-not-file");
    fs::write(&restored, b"original\n").unwrap();
    fs::create_dir(&invalid_leaf).unwrap();
    let root = ConfinedRoot::open(tmp.path()).unwrap();

    let restored_location = root.location("restored.txt").unwrap();
    let mut restored_backup = Backup::write(
        restored_location.clone(),
        backup_of(&restored_location).unwrap(),
        b"generated\n",
        0o644,
    );
    restored_location
        .write_atomic(b"generated\n", 0o644)
        .unwrap();
    let _published_identity = restored_backup.verify_published().unwrap();

    let backups = vec![
        restored_backup,
        Backup::delete(root.location("directory-not-file").unwrap(), None),
    ];
    let failures = restore(&backups).unwrap_err();
    assert!(matches!(
        failures.failures(),
        [FsError::Collision(path, _)] if path == "directory-not-file"
    ));
    assert_eq!(fs::read(restored).unwrap(), b"original\n");
}
