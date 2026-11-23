//! Package snapshot proofs for byte stability, aggregate bounds, and no-follow traversal.

use super::*;

#[cfg(unix)]
use std::fs::File;

#[cfg(unix)]
#[test]
fn captured_bytes_do_not_follow_later_source_changes() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("packages")).unwrap();
    let path = root.path().join("packages/catalog.toml");
    std::fs::write(&path, "version = 1\n").unwrap();

    let snapshot =
        DistributionSnapshot::capture(&pale_adapter_fs::LocalSourceFilesystem, root.path())
            .unwrap();
    std::fs::write(path, "version = 2\n").unwrap();

    assert_eq!(
        snapshot.text(Path::new("catalog.toml")).unwrap(),
        "version = 1\n"
    );
}

#[cfg(unix)]
#[test]
fn snapshot_debug_reports_bounds_without_source_bytes() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("packages")).unwrap();
    let source = "private-package-source";
    std::fs::write(root.path().join("packages/catalog.toml"), source).unwrap();

    let snapshot =
        DistributionSnapshot::capture(&pale_adapter_fs::LocalSourceFilesystem, root.path())
            .unwrap();
    let debug = format!("{snapshot:?}");

    assert!(debug.contains("retained_bytes"));
    assert!(!debug.contains(source));
    assert!(!debug.contains(root.path().to_str().unwrap()));
}

#[cfg(unix)]
#[test]
fn oversized_files_are_rejected_before_parsing() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("packages")).unwrap();
    let path = root.path().join("packages/catalog.toml");
    let file = File::create(path).unwrap();
    file.set_len(MAX_FILE_BYTES + 1).unwrap();

    assert!(matches!(
        DistributionSnapshot::capture(&pale_adapter_fs::LocalSourceFilesystem, root.path()),
        Err(PackageError::SnapshotLimit {
            limit: "file bytes",
            ..
        })
    ));
}

#[cfg(unix)]
#[test]
fn every_descendant_consumes_the_aggregate_entry_budget() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("packages/a/b")).unwrap();
    std::fs::write(root.path().join("packages/a/b/one"), "1").unwrap();
    std::fs::write(root.path().join("packages/two"), "2").unwrap();

    let limits = SourceTreeLimits::new(3, MAX_DEPTH, MAX_FILE_BYTES, MAX_TOTAL_BYTES);
    assert!(matches!(
        DistributionSnapshot::capture_root_with_limits(
            &pale_adapter_fs::LocalSourceFilesystem,
            &root.path().join("packages"),
            limits,
        ),
        Err(PackageError::SnapshotLimit {
            limit: "entry count",
            value: 4,
            maximum: 3,
        })
    ));
}

#[cfg(unix)]
#[test]
fn symlinked_entries_are_rejected() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("packages")).unwrap();
    std::fs::write(root.path().join("target"), "content").unwrap();
    symlink(
        root.path().join("target"),
        root.path().join("packages/catalog.toml"),
    )
    .unwrap();

    assert!(matches!(
        DistributionSnapshot::capture(&pale_adapter_fs::LocalSourceFilesystem, root.path()),
        Err(PackageError::InvalidEntry { .. })
    ));
}

#[cfg(not(unix))]
#[test]
fn capture_fails_before_source_reads_without_no_follow_traversal() {
    let root = tempfile::tempdir().unwrap();
    assert!(matches!(
        DistributionSnapshot::capture(&pale_adapter_fs::LocalSourceFilesystem, root.path()),
        Err(PackageError::UnsupportedPlatform { .. })
    ));
}
