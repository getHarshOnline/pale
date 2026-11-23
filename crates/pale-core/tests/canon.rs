//! Canonicalization tests: RFC-8785-style key sorting, UTF-8 preservation, digest
//! order-independence, and strict parsing (duplicate keys, floats, integer bounds).

use pale_core::canon::*;
use pale_core::sha256_hex;
use serde_json::json;

#[test]
fn canon_sorts_keys_and_compacts() {
    assert_eq!(
        to_canonical_string(&json!({"b":1,"a":2})).unwrap(),
        r#"{"a":2,"b":1}"#
    );
}

#[test]
fn canon_preserves_array_order() {
    assert_eq!(to_canonical_string(&json!([3, 2, 1])).unwrap(), "[3,2,1]");
}

#[test]
fn canon_nested_sorting() {
    let v = json!({"z": {"b": 2, "a": 1}, "a": [2, 1]});
    assert_eq!(
        to_canonical_string(&v).unwrap(),
        r#"{"a":[2,1],"z":{"a":1,"b":2}}"#
    );
}

#[test]
fn canon_keeps_non_ascii_as_utf8_not_escaped() {
    let v = json!({"é": "café"});
    assert_eq!(to_canonical_string(&v).unwrap(), "{\"é\":\"café\"}");
}

#[test]
fn canon_escapes_control_and_quote() {
    let v = json!({"s": "a\"b\nc"});
    assert_eq!(to_canonical_string(&v).unwrap(), r#"{"s":"a\"b\nc"}"#);
}

#[test]
fn digest_is_insertion_order_independent() {
    let a = digest(&json!({"a": 1, "b": 2})).unwrap();
    let b = digest(&json!({"b": 2, "a": 1})).unwrap();
    assert_eq!(a, b);
}

#[test]
fn digest_matches_sha256_of_canonical_bytes() {
    let v = json!({"a": 1});
    assert_eq!(digest(&v).unwrap(), sha256_hex(b"{\"a\":1}"));
}

#[test]
fn pretty_json_documents_preserve_strict_values_and_end_with_one_line_feed() {
    let strict_json = br#"{"z":{"value":true},"a":[2,1]}"#;
    let pretty = pretty_json_document(strict_json).expect("strict JSON renders for files");

    assert_eq!(
        parse_strict(&pretty).unwrap(),
        parse_strict(strict_json).unwrap()
    );
    assert!(pretty.windows(3).any(|window| window == b"\n  "));
    assert!(pretty.ends_with(b"\n"));
    assert!(!pretty.ends_with(b"\n\n"));
}

#[test]
fn typed_canonicalization_obeys_the_strict_number_contract() {
    assert!(to_canonical_string(&json!({"x": 1.5})).is_err());
    assert!(digest(&json!({"x": 1.5})).is_err());
    assert!(to_canonical_string(&json!({"x": 1u64 << 53})).is_err());
    assert!(digest(&json!({"x": 1u64 << 53})).is_err());
}

#[test]
fn strict_parse_accepts_clean_object() {
    let v = parse_strict(br#"{"a":1,"b":[2,3],"c":null,"d":true}"#).unwrap();
    assert_eq!(v, json!({"a":1,"b":[2,3],"c":null,"d":true}));
}

#[test]
fn strict_parse_rejects_duplicate_keys() {
    let err = parse_strict(br#"{"a":1,"a":2}"#).unwrap_err();
    assert!(matches!(err, CanonError::Parse(_)), "{err:?}");
    assert!(err.to_string().contains("duplicate"), "{err}");
}

#[test]
fn strict_parse_rejects_nested_duplicate_keys() {
    let err = parse_strict(br#"{"outer":{"a":1,"a":2}}"#).unwrap_err();
    assert!(err.to_string().contains("duplicate"), "{err}");
}

#[test]
fn strict_parse_rejects_float() {
    assert!(parse_strict(br#"{"x":1.5}"#).is_err());
    // integer written in float form is also rejected
    assert!(parse_strict(br#"{"x":1.0}"#).is_err());
}

#[test]
fn strict_parse_rejects_oversized_integer() {
    // > u64::MAX parses as float in serde_json, so it is rejected.
    assert!(parse_strict(br#"{"x":123456789012345678901234567890}"#).is_err());
}

#[test]
fn strict_parse_enforces_json_safe_integer_bound() {
    // MAX_SAFE_INTEGER (2^53 - 1) is accepted exactly.
    let max_safe = (1u64 << 53) - 1; // 9007199254740991
    let ok = parse_strict(format!(r#"{{"x":{max_safe}}}"#).as_bytes()).unwrap();
    assert_eq!(ok, json!({ "x": max_safe }));
    // 2^53 itself and above are rejected: RFC-8785 canonicalization through the
    // ECMAScript doubles cannot faithfully represent either signed 2^53
    // boundary or the maximum unsigned 64-bit integer.
    assert!(parse_strict(br#"{"x":9007199254740992}"#).is_err());
    assert!(parse_strict(br#"{"x":18446744073709551615}"#).is_err());
    assert!(parse_strict(br#"{"x":-9007199254740992}"#).is_err());
}

#[test]
fn strict_parse_rejects_trailing_bytes() {
    assert!(parse_strict(br#"{"a":1} garbage"#).is_err());
}

#[test]
fn unique_parse_preserves_native_json_numbers() {
    let value = parse_unique(br#"{"ratio":1.5,"epoch":18446744073709551615}"#).unwrap();
    assert_eq!(value["ratio"], json!(1.5));
    assert_eq!(value["epoch"], json!(u64::MAX));
}

#[test]
fn unique_parse_still_rejects_ambiguous_objects() {
    assert!(parse_unique(br#"{"hook":1,"hook":2}"#).is_err());
    assert!(parse_unique(br#"{"hook":1} trailing"#).is_err());
}
