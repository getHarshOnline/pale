//! Logical sizing stays read-only and refuses incomplete observations.

use pale_adapter_fs::measure_build_tree;

#[test]
fn counts_file_entries_and_sparse_lengths_without_following_links() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    std::fs::write(root.join("one"), b"12345").unwrap();
    std::fs::hard_link(root.join("one"), root.join("two")).unwrap();
    let sparse = std::fs::File::create(root.join("sparse")).unwrap();
    sparse.set_len(1_048_576).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("/", root.join("outside")).unwrap();
    let result = measure_build_tree(&root);
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    assert_eq!(result.logical_bytes, Some(1_048_586));
    assert_eq!(result.file_count, 3);
    #[cfg(unix)]
    assert_eq!(result.skipped_symlinks, 1);
    assert_eq!(std::fs::read(root.join("one")).unwrap(), b"12345");
}

#[test]
fn distinguishes_empty_missing_and_nondirectory_roots() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    assert_eq!(measure_build_tree(&root).logical_bytes, Some(0));
    std::fs::write(root.join("file"), b"x").unwrap();
    for path in [root.join("missing"), root.join("file")] {
        let result = measure_build_tree(&path);
        assert_eq!(result.logical_bytes, None);
        assert!(!result.errors.is_empty());
    }
}

#[cfg(unix)]
#[test]
fn refuses_symlink_roots_and_ancestors() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    std::fs::create_dir(root.join("real")).unwrap();
    std::fs::create_dir(root.join("real/nested")).unwrap();
    std::os::unix::fs::symlink(root.join("real"), root.join("alias")).unwrap();
    for path in [root.join("alias"), root.join("alias/nested")] {
        let result = measure_build_tree(&path);
        assert_eq!(result.logical_bytes, None);
        assert!(result.errors[0].contains("symlink"));
    }
}

#[cfg(unix)]
#[test]
fn retains_partial_measurement_when_a_special_file_is_present() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    std::fs::write(root.join("artifact"), b"abc").unwrap();
    let _socket = std::os::unix::net::UnixListener::bind(root.join("socket")).unwrap();
    let result = measure_build_tree(&root);
    assert_eq!(result.logical_bytes, Some(3));
    assert_eq!(result.file_count, 1);
    assert_eq!(result.errors.len(), 1);
    assert!(result.errors[0].contains("Unsupported file type"));
}

#[test]
fn refuses_unbounded_depth_and_relative_selection() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let mut nested = root.clone();
    for _ in 0..130 {
        nested.push("d");
        std::fs::create_dir(&nested).unwrap();
    }
    let report = measure_build_tree(&root);
    assert_eq!(report.logical_bytes, None);
    assert!(!report.errors.is_empty());
    let relative = measure_build_tree(std::path::Path::new("target"));
    assert_eq!(relative.logical_bytes, None);
    assert!(relative.errors[0].contains("absolute"));
}
