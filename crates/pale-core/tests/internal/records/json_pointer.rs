//! Exercises the bounded non-root RFC 6901 pointer grammar across parsing, Serde, and schema projection.

use std::collections::BTreeMap;

use super::{NonRootJsonPointer, NON_ROOT_JSON_POINTER_PATTERN};

#[test]
fn admission_uses_rfc_6901_with_the_bounded_non_root_policy() {
    for pointer in [
        "/foo", "/a~1b", "/m~0n", "/foo/", "/雪", "/~01", "/%25", "/a//b",
    ] {
        assert!(
            NonRootJsonPointer::parse(pointer).is_ok(),
            "valid pointer {pointer:?}"
        );
    }
    for pointer in [
        "", "/", "foo", "/~", "/~2", "/a~", "/a\n", "/a\r", "/\u{7f}", "/\u{85}",
    ] {
        assert!(
            NonRootJsonPointer::parse(pointer).is_err(),
            "invalid pointer {pointer:?}"
        );
    }

    let maximum = format!("/{}", "雪".repeat(1_023));
    let oversized = format!("/{}", "雪".repeat(1_024));
    assert!(NonRootJsonPointer::parse(&maximum).is_ok());
    assert!(NonRootJsonPointer::parse(&oversized).is_err());
}

#[test]
fn admitted_spelling_resolves_with_the_standard_json_value_boundary() {
    let value = serde_json::json!({"a/b": {"m~n": 7}, "~1": 8, "%25": 9});
    for (pointer, expected) in [
        ("/a~1b", &value["a/b"]),
        ("/a~1b/m~0n", &value["a/b"]["m~n"]),
        ("/~01", &value["~1"]),
        ("/%25", &value["%25"]),
    ] {
        let pointer = NonRootJsonPointer::parse(pointer).expect("valid test pointer");
        assert_eq!(value.pointer(pointer.as_ref()), Some(expected));
    }
}

#[test]
fn serde_and_schema_project_the_same_owned_invariant() {
    let pointer = NonRootJsonPointer::parse("/a~1b").expect("valid test pointer");
    assert_eq!(serde_json::to_string(&pointer).unwrap(), r#""/a~1b""#);
    assert_eq!(
        serde_json::from_str::<NonRootJsonPointer>(r#""/a~1b""#).unwrap(),
        pointer
    );
    assert!(serde_json::from_str::<NonRootJsonPointer>(r#""/~2""#).is_err());
    assert!(
        serde_json::from_value::<BTreeMap<NonRootJsonPointer, String>>(
            serde_json::json!({"/~2": "rejected before map construction"})
        )
        .is_err()
    );

    let schema = serde_json::to_value(schemars::schema_for!(NonRootJsonPointer)).unwrap();
    assert_eq!(schema["type"], "string");
    assert_eq!(schema["minLength"], 2);
    assert_eq!(schema["maxLength"], 1_024);
    assert_eq!(schema["pattern"], NON_ROOT_JSON_POINTER_PATTERN);
}
