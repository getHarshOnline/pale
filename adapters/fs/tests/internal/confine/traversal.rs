//! Ancestor creation journaling and failure recovery.

use super::ConfinedRoot;
use crate::errors::invariant;
use crate::file_transaction::recover;

#[test]
fn ancestor_journal_records_every_created_directory() {
    let temporary = tempfile::tempdir().unwrap();
    let root = ConfinedRoot::open(temporary.path()).unwrap();
    let mut created = Vec::new();

    let parent = root
        .ensure_parent_tracked("one/two/surface", &mut created)
        .unwrap();
    drop(parent);

    assert_eq!(
        created
            .iter()
            .map(|location| location.relative())
            .collect::<Vec<_>>(),
        ["one", "one/two"]
    );
    recover(&[], &created).unwrap();
    assert!(!temporary.path().join("one").exists());
}

#[test]
fn ancestor_sync_failure_keeps_the_visible_directory_rollbackable() {
    let temporary = tempfile::tempdir().unwrap();
    let root = ConfinedRoot::open(temporary.path()).unwrap();
    let mut created = Vec::new();
    let mut reject_sync = |_directory: &std::fs::File, path: &std::path::Path| {
        Err(invariant("inject parent sync failure", path, "test fault"))
    };

    assert!(root
        .ensure_parent_tracked_with_sync("created/child/surface", &mut created, &mut reject_sync,)
        .is_err());
    assert_eq!(
        created
            .iter()
            .map(|directory| directory.relative())
            .collect::<Vec<_>>(),
        ["created"]
    );
    assert!(temporary.path().join("created").is_dir());

    recover(&[], &created).unwrap();
    assert!(!temporary.path().join("created").exists());
}

#[test]
fn recovery_preserves_a_replacement_directory() {
    let temporary = tempfile::tempdir().unwrap();
    let root = ConfinedRoot::open(temporary.path()).unwrap();
    let mut created = Vec::new();

    root.ensure_parent_tracked("owned/surface", &mut created)
        .unwrap();
    std::fs::remove_dir(temporary.path().join("owned")).unwrap();
    std::fs::create_dir(temporary.path().join("owned")).unwrap();

    let failures = recover(&[], &created).unwrap_err();
    assert!(matches!(
        failures.failures(),
        [crate::errors::FsError::Collision(path, _)] if path == "owned"
    ));
    assert!(temporary.path().join("owned").is_dir());
}
