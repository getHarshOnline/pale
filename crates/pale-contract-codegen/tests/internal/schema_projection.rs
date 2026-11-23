//! Checks schema projection preserves wire fields while removing editorial annotations.

use super::*;

#[test]
fn editorial_stripping_preserves_wire_fields_named_description_and_title() {
    let mut schema = serde_json::json!({
        "description": "root prose",
        "properties": {
            "description": {
                "description": "field prose",
                "type": "string"
            },
            "title": {
                "title": "field prose",
                "type": "string"
            }
        }
    });
    strip_editorial_annotations(&mut schema);
    assert!(schema.get("description").is_none());
    assert_eq!(
        schema.pointer("/properties/description/type"),
        Some(&serde_json::json!("string"))
    );
    assert_eq!(
        schema.pointer("/properties/title/type"),
        Some(&serde_json::json!("string"))
    );
}

#[test]
fn definition_resources_have_a_false_root() {
    let schema = definition_set(|generator| {
        let _ = generator.subschema_for::<String>();
    })
    .to_value();
    assert_eq!(schema.get("not"), Some(&serde_json::json!({})));
    assert!(schema.get("$defs").is_some());
}

#[test]
fn uint64_projection_guard_recurses_and_requires_the_portable_jcs_domain() {
    let maximum = pale_core::canon::JSON_SAFE_INTEGER_MAX;
    let bounded = serde_json::json!({
        "$defs": {
            "Coordinate": {
                "type": "integer",
                "format": "uint64",
                "minimum": 0,
                "maximum": maximum
            }
        }
    });
    validate_portable_uint64_projections(&bounded, "bounded.v1")
        .expect("bounded nested uint64 admits");

    for unsafe_schema in [
        serde_json::json!({"type": "integer", "format": "uint64"}),
        serde_json::json!({
            "type": "integer",
            "format": "uint64",
            "minimum": -1,
            "maximum": maximum
        }),
        serde_json::json!({
            "properties": {
                "coordinate": {
                    "type": "integer",
                    "format": "uint64",
                    "minimum": 0,
                    "maximum": maximum + 1
                }
            }
        }),
    ] {
        let error = validate_portable_uint64_projections(&unsafe_schema, "unsafe.v1")
            .expect_err("unsafe uint64 projection must fail generation");
        assert!(error.contains("unsafe.v1 projects a uint64 outside the portable JCS domain"));
    }
}
