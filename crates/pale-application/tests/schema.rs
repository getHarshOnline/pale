//! Initial-v1 schema-boundary proofs owned by the application layer.

use pale_contracts::{record_schema_identity, validate_definition, validate_value, SchemaError};

#[test]
fn obsolete_standalone_package_contracts_have_no_registry_identity() {
    for id in [
        "connector.v1",
        "lifecycle-behaviour.v1",
        "package-selection.v1",
        "run-policies.v1",
        "tool.v1",
    ] {
        assert!(matches!(
            record_schema_identity(id),
            Err(SchemaError::UnknownId(_))
        ));
        assert!(matches!(
            validate_value(&serde_json::Value::Null, id),
            Err(SchemaError::UnknownId(_))
        ));
    }
}

#[test]
fn manifest_owned_capability_definitions_are_closed_without_new_schema_roots() {
    let policy = serde_json::json!({
        "permitted_capabilities": ["repository.read"],
        "forbidden_capabilities": [],
        "permitted_effects": ["read"],
        "forbidden_effects": [],
    });
    validate_definition(&policy, "capability-procedure.v1", "CapabilityPolicy")
        .expect("the named Package-owned policy definition is admissible");

    let mut unknown = policy;
    unknown["provider"] = serde_json::json!("private");
    assert!(matches!(
        validate_definition(&unknown, "capability-procedure.v1", "CapabilityPolicy"),
        Err(SchemaError::Invalid(_))
    ));
    assert!(matches!(
        record_schema_identity("capability-policy.v1"),
        Err(SchemaError::UnknownId(_))
    ));
}

#[test]
fn private_semantic_selection_requires_owner_qualified_resources() {
    let valid = serde_json::json!({
        "semantic": {
            "operations": [{
                "selection": "exact",
                "resource": {
                    "package_id": "example.engine",
                    "resource_id": "review"
                }
            }]
        },
        "run_policies": [{
            "id": "standard",
            "description": "Portable test policy.",
            "session_inject": false,
            "child_propagation": "propagate",
            "action_gate": false,
            "gated_tool_matchers": []
        }]
    });
    validate_value(&valid, "capabilities.v1").expect("qualified semantic selection is valid");

    let mut stringly = valid;
    stringly["semantic"]["operations"] = serde_json::json!(["review"]);
    assert!(matches!(
        validate_value(&stringly, "capabilities.v1"),
        Err(SchemaError::Invalid(_))
    ));
}
