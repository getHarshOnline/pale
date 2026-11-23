//! Operation record contract tests.

use super::OnDemandOperation;

#[test]
fn doctrine_source_rejects_parent_escape() {
    let value = serde_json::json!({
        "id": "review",
        "description": "Synthetic review operation.",
        "doctrine": "../outside.md"
    });

    assert!(serde_json::from_value::<OnDemandOperation>(value).is_err());
}
