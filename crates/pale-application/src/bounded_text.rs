//! Length-limited previews of untrusted text, kept in evidence records.
//!
//! `decision_evidence` and `operation_admission` build on it; it builds on nothing
//! else in this crate. A value that does not fit keeps its start and a digest of
//! the whole where the limit allows, so a record stays small and the full text can
//! still be checked.

/// Retain `value` exactly when it fits; otherwise append its complete digest
/// when the byte ceiling permits, or a bounded digest-marker prefix when it does not.
pub(crate) fn digest_preview(value: &str, limit: usize) -> String {
    if limit == 0 {
        return String::new();
    }
    if value.len() <= limit {
        return value.to_string();
    }
    let marker = format!("[sha256:{}]", pale_core::sha256_hex(value.as_bytes()));
    if marker.len() >= limit {
        return marker[..limit].to_string();
    }
    let mut end = limit - marker.len() - 1;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{} {marker}", &value[..end])
}

#[cfg(test)]
#[path = "../tests/internal/bounded_text.rs"]
mod tests;
