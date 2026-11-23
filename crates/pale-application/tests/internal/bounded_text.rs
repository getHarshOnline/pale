//! Bounded-text proofs for UTF-8-safe previews and complete-source digest evidence.

use super::digest_preview;

#[test]
fn preview_preserves_utf8_bounds_and_complete_digest_when_space_allows() {
    let value = "alpha-βeta-".repeat(20);
    let preview = digest_preview(&value, 84);

    assert!(preview.len() <= 84);
    assert!(preview.starts_with("alpha-"));
    assert!(preview.ends_with(&format!(
        "[sha256:{}]",
        pale_core::sha256_hex(value.as_bytes())
    )));
}

#[test]
fn tiny_ceiling_returns_a_valid_bounded_marker_prefix() {
    assert!(digest_preview("not hashed when no output is possible", 0).is_empty());
    let preview = digest_preview("larger than three bytes", 3);

    assert_eq!(preview, "[sh");
    assert_eq!(preview.len(), 3);
}
