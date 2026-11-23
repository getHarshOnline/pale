//! Behavioral boundaries for bounded baseline observations.

#![cfg(unix)]

use pale_adapter_fs::{read_baseline, FsError, ReadLimit, MATERIALIZED_SURFACE_READ_LIMIT};
use pale_core::records::{ArtifactDigest, BaselineStateV1};

/// Production surface-read bound reused so tests exercise the public contract.
const BASELINE_LIMIT: ReadLimit = MATERIALIZED_SURFACE_READ_LIMIT;

#[test]
fn read_baseline_records_existing_and_missing() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("a.md"), "content").unwrap();
    let b = read_baseline(
        tmp.path(),
        &["a.md".into(), "missing.md".into()],
        BASELINE_LIMIT,
    )
    .unwrap();
    let a = b.iter().find(|e| e.rel_path == "a.md").unwrap();
    assert!(matches!(
        &a.observation,
        BaselineStateV1::Present { sha256, .. }
            if sha256 == &ArtifactDigest::hash_bytes(b"content")
    ));
    let m = b.iter().find(|e| e.rel_path == "missing.md").unwrap();
    assert_eq!(m.observation, BaselineStateV1::Absent {});
}

#[test]
fn read_baseline_fails_closed_on_metadata_error() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("not-a-directory"), "x").unwrap();
    let e = read_baseline(
        tmp.path(),
        &["not-a-directory/managed-surface.md".to_string()],
        BASELINE_LIMIT,
    );
    assert!(e.is_err(), "an unsafe ancestor must fail closed: {e:?}");
}

#[test]
fn read_baseline_rejects_duplicate_surface_identity() {
    let tmp = tempfile::tempdir().unwrap();
    let error = read_baseline(
        tmp.path(),
        &["same.md".into(), "same.md".into()],
        BASELINE_LIMIT,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        FsError::DuplicateRequestPath { path } if path == "same.md"
    ));
}
