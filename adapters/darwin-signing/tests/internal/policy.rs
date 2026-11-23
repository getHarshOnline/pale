//! A duplicate or unbounded native policy never silently broadens a key slot.
use super::*;
use serde_json::json;

fn policy() -> serde_json::Value {
    json!({"schema_version":1,"client_requirement":"fixture-only-requirement",
        "client_uid":501,"maximum_connections":2,"slots":[{
            "alias":"fixture","service":"fixture-service","account":"fixture-account",
            "public_key":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","allowed_kinds":["operation_handle"]}]})
}

#[test]
fn policy_is_closed_bounded_and_has_unique_slots_and_kinds() {
    let check = |value: &serde_json::Value| valid_policy(&serde_json::to_vec(value).unwrap());
    let good = policy();
    assert!(check(&good));
    for (field, value) in [
        ("schema_version", json!(2)),
        ("maximum_connections", json!(0)),
        ("maximum_connections", json!(65)),
        ("fallback", json!("forbidden")),
    ] {
        let mut bad = good.clone();
        bad[field] = value;
        assert!(!check(&bad), "{field}");
    }
    let mut bad = good.clone();
    let slot = bad["slots"][0].clone();
    bad["slots"].as_array_mut().unwrap().push(slot);
    assert!(!check(&bad));
    for (field, value) in [
        (
            "allowed_kinds",
            json!(["operation_handle", "operation_handle"]),
        ),
        ("allowed_kinds", json!(["arbitrary_domain"])),
        ("public_key", json!("not-a-key")),
        ("seed", json!("forbidden")),
    ] {
        let mut bad = good.clone();
        bad["slots"][0][field] = value;
        assert!(!check(&bad), "{field}");
    }
}
