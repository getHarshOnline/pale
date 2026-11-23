//! Exercises the source snapshot port as one bounded, relative, internally coherent tree.

use super::*;

fn valid_snapshot() -> SourceTreeSnapshot {
    SourceTreeSnapshot::try_from_parts(
        PathBuf::from("source"),
        BTreeSet::from([PathBuf::new(), PathBuf::from("nested")]),
        BTreeMap::from([(PathBuf::from("nested/file.md"), Box::from(&b"private"[..]))]),
    )
    .unwrap()
}

#[test]
fn snapshot_parts_must_form_one_relative_tree() {
    let invalid = [
        (BTreeSet::new(), PathBuf::from("file.md")),
        (
            BTreeSet::from([PathBuf::new()]),
            PathBuf::from("../file.md"),
        ),
        (
            BTreeSet::from([PathBuf::new()]),
            PathBuf::from("missing/file.md"),
        ),
        (
            BTreeSet::from([PathBuf::new(), PathBuf::from("nested")]),
            PathBuf::from("nested/./file.md"),
        ),
        (
            BTreeSet::from([PathBuf::new(), PathBuf::from("nested")]),
            PathBuf::from("nested//file.md"),
        ),
    ];
    for (directories, file) in invalid {
        assert!(matches!(
            SourceTreeSnapshot::try_from_parts(
                PathBuf::from("source"),
                directories,
                BTreeMap::from([(file, Box::from(&b"bytes"[..]))]),
            ),
            Err(SourceFilesystemError::InvalidEntry { .. })
        ));
    }

    assert!(matches!(
        SourceTreeSnapshot::try_from_parts(
            PathBuf::from("source"),
            BTreeSet::from([PathBuf::new(), PathBuf::from("missing/child")]),
            BTreeMap::new(),
        ),
        Err(SourceFilesystemError::InvalidEntry { .. })
    ));
    assert!(matches!(
        SourceTreeSnapshot::try_from_parts(
            PathBuf::from("source"),
            BTreeSet::from([PathBuf::new(), PathBuf::from("same")]),
            BTreeMap::from([(PathBuf::from("same"), Box::from(&b"bytes"[..]))]),
        ),
        Err(SourceFilesystemError::InvalidEntry { .. })
    ));
    assert!(matches!(
        SourceTreeSnapshot::try_from_parts(
            PathBuf::from("source"),
            BTreeSet::from([PathBuf::new()]),
            BTreeMap::from([(PathBuf::new(), Box::from(&b"bytes"[..]))]),
        ),
        Err(SourceFilesystemError::InvalidEntry { .. })
    ));
    assert!(matches!(
        SourceTreeSnapshot::try_from_parts(
            PathBuf::from("source"),
            BTreeSet::from([PathBuf::new()]),
            BTreeMap::from([(PathBuf::from("/absolute"), Box::from(&b"bytes"[..]))]),
        ),
        Err(SourceFilesystemError::InvalidEntry { .. })
    ));
}

#[test]
fn snapshot_debug_never_renders_retained_bytes() {
    let rendered = format!("{:?}", valid_snapshot());
    assert!(rendered.contains("retained_bytes: 7"));
    assert!(!rendered.contains("private"));
}
