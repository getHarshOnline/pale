//! Exercises lexical source-tree capture, exact resource ceilings, and hostile descendants.

use std::fs;
use std::os::unix::fs::symlink;
use std::process::Command;

use super::*;

/// Limits large enough for ordinary fixtures while keeping failure cases explicit.
const GENEROUS: SourceTreeLimits = SourceTreeLimits::new(64, 8, 1_024, 4_096);

#[test]
fn selected_capture_prunes_excluded_trees_and_never_opens_other_files() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    fs::write(root.join("read.md"), "retained").unwrap();
    fs::write(root.join("binary.dat"), vec![0u8; 2_048]).unwrap();
    fs::create_dir(root.join("records")).unwrap();
    symlink("/absent", root.join("records/old.md")).unwrap();
    let snapshot = capture_selected(
        root,
        GENEROUS,
        &|path| path.starts_with("records"),
        &|path| path.extension().is_some_and(|ext| ext == "md"),
    )
    .unwrap();
    assert_eq!(snapshot.files().collect::<Vec<_>>(), [Path::new("read.md")]);
    symlink("/absent", root.join("linked.md")).unwrap();
    assert!(
        capture_selected(root, GENEROUS, &|path| path.starts_with("records"), &|_| {
            true
        })
        .is_err()
    );
}

#[test]
fn capture_is_lexical_and_isolated_from_later_source_mutation() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("source");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("z.md"), b"z-before").unwrap();
    fs::write(root.join("a.md"), b"a-before").unwrap();

    let snapshot = capture(&root, GENEROUS).unwrap();
    let files = snapshot.files().collect::<Vec<_>>();
    assert_eq!(files, [Path::new("a.md"), Path::new("z.md")]);

    fs::write(root.join("a.md"), b"a-after").unwrap();
    assert_eq!(snapshot.file(Path::new("a.md")), Some(&b"a-before"[..]));
}

#[test]
fn every_tree_limit_is_exact_and_fail_closed() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("source");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("a"), b"x").unwrap();
    fs::create_dir(root.join("sub")).unwrap();
    fs::write(root.join("sub/b"), b"y").unwrap();

    assert!(capture(&root, SourceTreeLimits::new(3, 1, 1, 2)).is_ok());
    let cases = [
        (SourceTreeLimits::new(2, 1, 1, 2), "entry count"),
        (SourceTreeLimits::new(3, 0, 1, 2), "directory depth"),
        (SourceTreeLimits::new(3, 1, 0, 2), "file bytes"),
        (SourceTreeLimits::new(3, 1, 1, 1), "total bytes"),
    ];
    for (limits, expected) in cases {
        assert!(matches!(
            capture(&root, limits),
            Err(SourceFilesystemError::Limit { limit, .. }) if limit == expected
        ));
    }
}

#[test]
fn rejects_symlink_and_special_descendants() {
    let temporary = tempfile::tempdir().unwrap();

    let symlink_root = temporary.path().join("symlink-tree");
    fs::create_dir(&symlink_root).unwrap();
    fs::write(temporary.path().join("outside"), b"outside").unwrap();
    symlink(temporary.path().join("outside"), symlink_root.join("link")).unwrap();
    assert!(matches!(
        capture(&symlink_root, GENEROUS),
        Err(SourceFilesystemError::InvalidEntry { .. })
    ));

    let special_root = temporary.path().join("special-tree");
    fs::create_dir(&special_root).unwrap();
    let fifo = special_root.join("fifo");
    assert!(Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .unwrap()
        .success());
    assert!(matches!(
        capture(&special_root, GENEROUS),
        Err(SourceFilesystemError::InvalidEntry { .. })
    ));
}

#[cfg(not(target_os = "macos"))]
#[test]
fn rejects_non_utf8_descendants_where_the_filesystem_can_create_them() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("non-utf8-tree");
    fs::create_dir(&root).unwrap();
    let non_utf8 = OsString::from_vec(vec![b'n', b'o', b'n', 0xff]);
    fs::write(root.join(non_utf8), b"bytes").unwrap();
    assert!(matches!(
        capture(&root, GENEROUS),
        Err(SourceFilesystemError::InvalidEntry { .. })
    ));
}

#[test]
fn rejects_a_root_replaced_after_descriptor_relative_capture() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("source");
    let displaced = temporary.path().join("displaced");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("file"), b"retained").unwrap();

    let result = capture_with(&root, GENEROUS, || {
        fs::rename(&root, &displaced).unwrap();
        fs::create_dir(&root).unwrap();
        fs::write(root.join("file"), b"replacement").unwrap();
    });
    assert!(matches!(
        result,
        Err(SourceFilesystemError::InvalidEntry { .. })
    ));
}

#[test]
fn rejects_a_symlinked_source_root() {
    let temporary = tempfile::tempdir().unwrap();
    let real = temporary.path().join("real");
    let link = temporary.path().join("link");
    fs::create_dir(&real).unwrap();
    symlink(&real, &link).unwrap();
    assert!(matches!(
        capture(&link, GENEROUS),
        Err(SourceFilesystemError::InvalidEntry { .. })
    ));
}
