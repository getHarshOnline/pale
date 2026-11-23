//! Structural projection tests for the closed composition wire model.

use pale_core::records::{ExperienceCompositionPlanV1, WorkspaceCompositionSourceV1};

fn projected_composition_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(ExperienceCompositionPlanV1))
        .expect("composition schema is JSON")
}

fn required_fields_at<'a>(value: &'a serde_json::Value, pointer: &str) -> &'a [serde_json::Value] {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .expect("projected object has a required-field set")
}

#[test]
fn composition_projection_closes_over_every_nested_wire_type() {
    let value = projected_composition_schema();
    let definitions = value["$defs"]
        .as_object()
        .expect("composition projection has named definitions");

    for expected in [
        "CompositionEdgeV1",
        "ContinuityIdentityV1",
        "DeploymentActivationIdentityV1",
        "EvidenceSettlementIdentityV1",
        "InvocationIdentityV1",
        "PaleAdmissionGenerationIdentityV1",
        "NativeDataGenerationIdentityV1",
        "NativeEffectIdentityV1",
        "ProviderBindingIdentityV1",
        "ResponsibilityPathIdentityV1",
    ] {
        assert!(
            definitions.contains_key(expected),
            "composition projection omitted {expected}"
        );
    }
}

#[test]
fn composition_projection_retains_bounded_wire_structure() {
    let value = projected_composition_schema();

    assert_eq!(
        value.pointer("/properties/schema/const"),
        Some(&serde_json::json!("pale/experience-composition-plan.v1"))
    );
    assert_eq!(
        value.pointer("/properties/schema_version/const"),
        Some(&serde_json::json!(1))
    );
    assert_eq!(
        value.pointer("/$defs/PathBudgetV1/properties/max_depth/minimum"),
        Some(&serde_json::json!(1))
    );
    assert_eq!(
        value.pointer("/$defs/PathBudgetV1/properties/max_time_ms/maximum"),
        Some(&serde_json::json!(9_007_199_254_740_991_u64))
    );
    assert!(value
        .pointer("/properties/supported_bootstrap_spines")
        .is_none());
    assert!(value
        .pointer("/properties/selected_bootstrap_spines")
        .is_none());
    assert!(value
        .pointer("/$defs/ExperiencePathV1/properties/bootstrap_spine")
        .is_none());
    assert_eq!(
        value.pointer(
            "/$defs/ExperienceCompositionDeclarationsV1/properties/edge_contracts/maxItems",
        ),
        Some(&serde_json::json!(4_096))
    );
    assert_eq!(
        value.pointer("/properties/provider_compatibility_graph/allOf/1/properties/kind/const"),
        Some(&serde_json::json!("provider_compatibility"))
    );
    assert_eq!(
        value.pointer(
            "/properties/provider_compatibility_graph/allOf/0/properties/nodes/uniqueItems",
        ),
        Some(&serde_json::json!(true))
    );

    for (pointer, nullable_field) in [
        ("/required", "plan_digest"),
        (
            "/$defs/ProviderBindingV1/required",
            "communication_actor_handle",
        ),
        (
            "/$defs/SettlementContractV1/required",
            "native_receipt_contract",
        ),
        ("/$defs/TerminationContractV1/required", "terminal_leaf"),
    ] {
        assert!(
            required_fields_at(&value, pointer)
                .iter()
                .any(|field| field == nullable_field),
            "composition projection made `{nullable_field}` omissible"
        );
    }
}

#[test]
fn workspace_composition_source_owns_one_nested_declaration_inventory() {
    let value = serde_json::to_value(schemars::schema_for!(WorkspaceCompositionSourceV1))
        .expect("workspace composition source schema is JSON");

    assert_eq!(
        value.pointer("/properties/schema/const"),
        Some(&serde_json::json!("pale/workspace-composition-source.v1"))
    );
    assert_eq!(
        value.pointer("/properties/schema_version/const"),
        Some(&serde_json::json!(1))
    );
    assert!(required_fields_at(&value, "/required")
        .iter()
        .any(|field| field == "declarations"));
    for declaration_family in [
        "responsibility_contracts",
        "participant_provider_manifests",
        "provider_bindings",
        "entry_contracts",
        "request_translations",
        "result_translations",
        "termination_contracts",
        "settlement_contracts",
        "continuity_contracts",
        "edge_contracts",
        "selected_experience_paths",
    ] {
        assert!(
            required_fields_at(
                &value,
                "/$defs/ExperienceCompositionDeclarationsV1/required",
            )
            .iter()
            .any(|field| field == declaration_family),
            "workspace composition source made `{declaration_family}` omissible"
        );
    }
}
