//! Public confinement, complete-read, mode, and no-follow publication guarantees.

#[cfg(unix)]
use pale_adapter_fs::{ConfinedEntryKind, DirectoryLimit, ReadLimit, TraversalLimit};
use pale_adapter_fs::{ConfinedRoot, FsError};

#[cfg(unix)]
/// Deliberately tiny read ceiling used to prove complete-payload rejection.
const FOUR_BYTES: ReadLimit = ReadLimit::new(4);

#[test]
fn platform_envelope_is_explicit_at_root_admission() {
    let tmp = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    assert!(ConfinedRoot::open(tmp.path()).is_ok());
    #[cfg(not(unix))]
    assert!(matches!(
        ConfinedRoot::open(tmp.path()),
        Err(FsError::UnsupportedPlatform { .. })
    ));
}

#[cfg(unix)]
#[test]
fn bounded_file_reads_redact_content_and_enforce_the_hard_limit() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("record"), b"four").unwrap();
    let root = ConfinedRoot::open(tmp.path()).unwrap();
    let record = root.location("record").unwrap();

    let file = record.read_file(FOUR_BYTES).unwrap().unwrap();
    let debug = format!("{file:?}");
    assert!(debug.contains("redacted:4 bytes"));
    assert!(
        !debug.contains("four"),
        "debug output must not expose file bytes"
    );
    let metadata = record.inspect_file(FOUR_BYTES).unwrap().unwrap();
    assert_eq!(file.bytes, b"four");
    assert_eq!(metadata.len, 4);
    assert!(matches!(
        record.read_file(ReadLimit::new(3)),
        Err(FsError::FileReadLimit {
            ref path,
            max_bytes: 3
        }) if path == "record"
    ));
}

#[cfg(unix)]
#[test]
fn metadata_inspection_rejects_a_fifo_replacement_without_waiting_for_a_writer() {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    const PROBE_ROOT: &str = "PALE_CONFINED_FIFO_PROBE_ROOT";
    if let Some(path) = std::env::var_os(PROBE_ROOT) {
        let root = ConfinedRoot::open(std::path::Path::new(&path)).unwrap();
        assert!(matches!(
            root.location("record").unwrap().inspect_file(FOUR_BYTES),
            Err(FsError::Collision(ref path, _)) if path == "record"
        ));
        return;
    }

    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("record");
    std::fs::write(&path, b"four").unwrap();
    let root = ConfinedRoot::open(temporary.path()).unwrap();
    assert_eq!(
        root.location("record")
            .unwrap()
            .inspect_file(FOUR_BYTES)
            .unwrap()
            .unwrap()
            .len,
        4
    );
    std::fs::remove_file(&path).unwrap();
    assert!(Command::new("mkfifo")
        .arg(&path)
        .status()
        .unwrap()
        .success());

    // A child gives a blocking-open regression a deadline and can be reaped.
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "metadata_inspection_rejects_a_fifo_replacement_without_waiting_for_a_writer",
        ])
        .env(PROBE_ROOT, temporary.path())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "FIFO inspection was not rejected");
            break;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("metadata inspection waited for a FIFO writer");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(unix)]
#[test]
fn publication_rejects_mode_bits_outside_the_admitted_mask() {
    let tmp = tempfile::tempdir().unwrap();
    let root = ConfinedRoot::open(tmp.path()).unwrap();
    let record = root.location("record").unwrap();

    assert!(matches!(
        record.write_atomic(b"content", 0o10_000),
        Err(FsError::InvalidMode {
            ref path,
            mode: 0o10_000
        }) if path == "record"
    ));
    assert!(!tmp.path().join("record").exists());
}

#[cfg(unix)]
#[test]
fn replacement_preserves_a_symlink_leaf_and_its_target() {
    use std::os::unix::fs::symlink;

    let tmp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("target");
    std::fs::write(&target, b"outside").unwrap();
    symlink(&target, tmp.path().join("record")).unwrap();
    let root = ConfinedRoot::open(tmp.path()).unwrap();

    assert!(matches!(
        root.location("record").unwrap().write_atomic(b"new", 0o600),
        Err(FsError::Collision(ref path, _)) if path == "record"
    ));
    assert_eq!(std::fs::read(&target).unwrap(), b"outside");
    assert!(std::fs::symlink_metadata(tmp.path().join("record"))
        .unwrap()
        .file_type()
        .is_symlink());
}

#[cfg(unix)]
#[test]
fn lock_acquisition_does_not_adopt_or_repair_an_existing_file() {
    use pale_adapter_fs::{acquire_advisory_lock, AdvisoryLockEffect, FsError};
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::time::Duration;

    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("lock"), b"user bytes").unwrap();
    std::fs::set_permissions(
        tmp.path().join("lock"),
        std::fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    let root = ConfinedRoot::open(tmp.path()).unwrap();

    let failure = acquire_advisory_lock(&root, "lock", Duration::from_millis(10), "test")
        .expect_err("foreign lock contents are rejected");
    assert_eq!(failure.effect(), AdvisoryLockEffect::Untouched);
    assert!(matches!(
        failure.source_error(),
        FsError::Collision(path, _) if path == "lock"
    ));
    assert_eq!(
        std::fs::read(tmp.path().join("lock")).unwrap(),
        b"user bytes"
    );
    assert_eq!(
        std::fs::metadata(tmp.path().join("lock")).unwrap().mode() & 0o7777,
        0o644
    );
}

#[cfg(unix)]
#[test]
fn lock_acquisition_times_out_while_a_prior_guard_is_held() {
    use pale_adapter_fs::{acquire_advisory_lock, AdvisoryLockEffect, FsError};
    use std::os::unix::fs::MetadataExt;
    use std::time::Duration;

    let tmp = tempfile::tempdir().unwrap();
    let root = ConfinedRoot::open(tmp.path()).unwrap();
    let (held, first_effect) =
        acquire_advisory_lock(&root, "lock", Duration::from_millis(10), "test")
            .expect("first caller holds the confined lock");
    assert_eq!(first_effect, AdvisoryLockEffect::Confirmed);
    assert_eq!(
        std::fs::metadata(tmp.path().join("lock")).unwrap().mode() & 0o7777,
        0o600
    );

    let timeout = acquire_advisory_lock(&root, "lock", Duration::from_millis(10), "test")
        .expect_err("the held lock enforces bounded contention");
    assert_eq!(timeout.effect(), AdvisoryLockEffect::Untouched);
    assert!(matches!(
        timeout.source_error(),
        FsError::Timeout {
            operation: "test",
            ..
        }
    ));

    drop(held);
    let (reacquired, repeated_effect) =
        acquire_advisory_lock(&root, "lock", Duration::from_millis(10), "test")
            .expect("dropping the guard releases the operating-system lock");
    assert_eq!(repeated_effect, AdvisoryLockEffect::Untouched);
    drop(reacquired);
}

#[cfg(unix)]
#[test]
fn directory_enumeration_is_sorted_and_fails_closed_at_its_entry_bound() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join("items")).unwrap();
    std::fs::write(tmp.path().join("items/z"), b"").unwrap();
    std::fs::write(tmp.path().join("items/a"), b"").unwrap();
    let root = ConfinedRoot::open(tmp.path()).unwrap();
    let items = root.location("items").unwrap();

    let entries = items
        .read_directory(DirectoryLimit::new(2))
        .unwrap()
        .unwrap();
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>(),
        ["a", "z"]
    );
    assert!(matches!(
        items.read_directory(DirectoryLimit::new(1)),
        Err(FsError::DirectoryEntryLimit {
            ref path,
            max_entries: 1
        }) if path == "items"
    ));
}

#[cfg(unix)]
#[test]
fn root_entry_count_is_repeatable_and_keeps_its_bound() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("first"), b"").unwrap();
    std::fs::write(tmp.path().join("second"), b"").unwrap();
    let root = ConfinedRoot::open(tmp.path()).unwrap();

    assert_eq!(root.entry_count(DirectoryLimit::new(2)).unwrap(), 2);
    assert_eq!(root.entry_count(DirectoryLimit::new(2)).unwrap(), 2);
    assert!(matches!(
        root.entry_count(DirectoryLimit::new(1)),
        Err(FsError::DirectoryEntryLimit { max_entries: 1, .. })
    ));
}

#[cfg(unix)]
#[test]
fn traversal_enforces_aggregate_count_and_depth_without_following_symlinks() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("tree/a/b")).unwrap();
    std::fs::write(tmp.path().join("tree/a/b/value"), b"value").unwrap();
    let root = ConfinedRoot::open(tmp.path()).unwrap();
    let tree = root.location("tree").unwrap();

    let count_error = tree.walk_directory(TraversalLimit::new(2, 8)).unwrap_err();
    assert!(
        matches!(
            count_error,
            FsError::TraversalEntryLimit {
                ref path,
                max_entries: 2
            } if path == "tree"
        ),
        "unexpected traversal count error: {count_error:?}"
    );
    let depth_error = tree.walk_directory(TraversalLimit::new(8, 2)).unwrap_err();
    assert!(
        matches!(
            depth_error,
            FsError::TraversalDepthLimit {
                ref path,
                max_depth: 2
            } if path == "tree/a/b/value"
        ),
        "unexpected traversal depth error: {depth_error:?}"
    );

    let entries = tree
        .walk_directory(TraversalLimit::new(3, 3))
        .unwrap()
        .unwrap();
    assert_eq!(
        entries
            .iter()
            .map(|entry| (entry.relative.as_str(), entry.kind))
            .collect::<Vec<_>>(),
        [
            ("tree/a", ConfinedEntryKind::Directory),
            ("tree/a/b", ConfinedEntryKind::Directory),
            ("tree/a/b/value", ConfinedEntryKind::File),
        ]
    );
}

#[cfg(unix)]
#[test]
fn traversal_reports_but_never_follows_a_symlink() {
    use std::os::unix::fs::symlink;

    let tmp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join("tree")).unwrap();
    std::fs::write(outside.path().join("secret"), b"secret").unwrap();
    symlink(outside.path(), tmp.path().join("tree/link")).unwrap();
    let root = ConfinedRoot::open(tmp.path()).unwrap();

    let entries = root
        .location("tree")
        .unwrap()
        .walk_directory(TraversalLimit::new(2, 2))
        .unwrap()
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].relative, "tree/link");
    assert_eq!(entries[0].kind, ConfinedEntryKind::Symlink);
}

#[cfg(unix)]
#[test]
fn retained_root_rejects_a_pathname_replaced_after_admission() {
    let tmp = tempfile::tempdir().unwrap();
    let root_path = tmp.path().join("root");
    let moved_path = tmp.path().join("moved");
    std::fs::create_dir(&root_path).unwrap();
    let root = ConfinedRoot::open(&root_path).unwrap();

    std::fs::rename(&root_path, &moved_path).unwrap();
    std::fs::create_dir(&root_path).unwrap();

    assert!(matches!(
        root.verify_path_identity(),
        Err(FsError::Collision(_, _))
    ));
}

#[cfg(unix)]
#[test]
fn removal_preserves_symlink_and_directory_leaves() {
    use std::os::unix::fs::symlink;

    let tmp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("target");
    std::fs::write(&target, b"owned elsewhere").unwrap();
    symlink(&target, tmp.path().join("link")).unwrap();
    std::fs::create_dir(tmp.path().join("directory")).unwrap();
    let root = ConfinedRoot::open(tmp.path()).unwrap();

    for relative in ["link", "directory"] {
        assert!(matches!(
            root.location(relative).unwrap().remove_file(),
            Err(FsError::Collision(ref path, _)) if path == relative
        ));
    }
    assert_eq!(std::fs::read(&target).unwrap(), b"owned elsewhere");
    assert!(std::fs::symlink_metadata(tmp.path().join("link"))
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(tmp.path().join("directory").is_dir());
}
