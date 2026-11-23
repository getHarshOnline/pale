//! Capability record contract tests.

use super::*;

fn resource(resource_id: &str) -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: crate::records::PackageIdV1::parse("example.capabilities")
            .expect("synthetic Package id must be valid"),
        resource_id: crate::records::PackageResourceIdV1::parse(resource_id)
            .expect("synthetic resource id must be valid"),
    }
}

#[test]
fn admitted_capability_projection_contains_no_authored_resource_links() {
    fn references_authored_link(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Object(object) => object.iter().any(|(key, value)| {
                (key == "$ref"
                    && value
                        .as_str()
                        .is_some_and(|reference| reference.ends_with("PackageResourceLinkV1")))
                    || references_authored_link(value)
            }),
            serde_json::Value::Array(values) => values.iter().any(references_authored_link),
            _ => false,
        }
    }

    let schema = serde_json::to_value(schemars::schema_for!(CapabilityBindingsV1))
        .expect("generated schema must serialize");
    assert!(schema.pointer("/$defs/PackageResourceRefV1").is_some());
    assert!(schema.pointer("/$defs/PackageResourceLinkV1").is_none());
    assert!(!references_authored_link(&schema));
}

#[test]
fn one_content_bearing_recipe_can_back_multiple_requirement_refs() {
    let recipe_ref = resource("recipe");
    let capability = CapabilityIdV1::parse("inspect").expect("test capability must be valid");
    let requirement = |id| ResolvedCapabilityUseV1 {
        kind: CapabilityUseKind::OperationRequirement,
        owner: resource("operation"),
        requirement_id: CapabilityRequirementIdV1::parse(id)
            .expect("test requirement id must be valid"),
        capability_id: capability.clone(),
        recipe: Some(recipe_ref.clone()),
        effect_class: EffectClass::Read,
        policy: ResolvedCapabilityRequirementPolicyV1::RequiredBlock,
        failure_posture: FailurePosture::FailClosed,
    };
    let projection = CapabilityBindingsV1 {
        schema: SchemaId::parse("pale.agent-surface.capability-bindings.v1")
            .expect("test schema id must be valid"),
        schema_sha256: crate::records::SchemaDigest::parse("0".repeat(64))
            .expect("test digest must be valid"),
        schema_version: 1,
        profile_digest: ProfileDigest::parse("1".repeat(64)).expect("test digest must be valid"),
        recipes: vec![ResolvedToolRecipeV1 {
            resource: recipe_ref.clone(),
            procedure: ToolRecipe {
                description: "Inspect one bounded target.".into(),
                capability: capability.clone(),
                tool_contract: ToolContractIdV1::parse("inspection")
                    .expect("test contract id must be valid"),
                tool_operation: ToolOperationIdV1::parse("inspect")
                    .expect("test operation id must be valid"),
                effect_class: EffectClass::Read,
                input_mapping: BTreeMap::new(),
                evidence: ToolEvidenceContract {
                    required_fields: Vec::new(),
                    proof_ceiling: ProofCeilingIdV1::parse("observed")
                        .expect("test proof id must be valid"),
                    repository_verifier: false,
                },
            },
        }],
        providers: Vec::new(),
        requirements: vec![requirement("primary"), requirement("secondary")],
    };

    let value = serde_json::to_value(&projection).expect("projection must serialize");
    assert_eq!(value["recipes"].as_array().map(Vec::len), Some(1));
    assert!(value["requirements"]
        .as_array()
        .expect("requirements must be an array")
        .iter()
        .all(|use_| use_["recipe"] == value["recipes"][0]["resource"]));
}
