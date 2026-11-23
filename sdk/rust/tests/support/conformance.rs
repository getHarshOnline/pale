//! Explicit regeneration of API-owned conformance outputs; ordinary runs only compare.

use std::path::PathBuf;

/// Resolve one named canonical fixture, never a leaf projection.
fn canonical_path(member: &str) -> PathBuf {
    assert!(
        !member.starts_with('/') && !member.split('/').any(|part| part == ".."),
        "fixture member must remain within its canonical owner"
    );
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/conformance/cross-leaf.v1")
        .join(member)
}

/// Compare normal test evidence, or explicitly regenerate the named owner output.
pub(super) fn assert_json(member: &str, actual: &serde_json::Value, expected: &serde_json::Value) {
    if std::env::var_os("PALE_UPDATE_CONFORMANCE").as_deref() == Some("1".as_ref()) {
        let mut bytes = serde_json::to_vec_pretty(actual).expect("fixture serializes");
        bytes.push(b'\n');
        std::fs::write(canonical_path(member), bytes).expect("canonical fixture writes");
    } else {
        assert_eq!(actual, expected, "canonical fixture {member}");
    }
}
