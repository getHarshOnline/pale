//! Contract-shape guards for the initial-v1 convergence families.

use std::collections::BTreeSet;

use pale_core::records::{
    GenealogyProofV1, InteractionResponseExpectationV1, OwnerRecordKnowledgeV1,
    RESIDENT_PUBLIC_ROOT_SCHEMA_IDS_V1,
};
use serde_json::Value;

fn property_consts<T: schemars::JsonSchema>(property: &str) -> BTreeSet<String> {
    fn visit(value: &Value, property: &str, constants: &mut BTreeSet<String>) {
        match value {
            Value::Object(object) => {
                if let Some(constant) = object
                    .get("properties")
                    .and_then(|properties| properties.get(property))
                    .and_then(|schema| schema.get("const"))
                    .and_then(Value::as_str)
                {
                    constants.insert(constant.to_owned());
                }
                for child in object.values() {
                    visit(child, property, constants);
                }
            }
            Value::Array(array) => {
                for child in array {
                    visit(child, property, constants);
                }
            }
            _ => {}
        }
    }

    let document = serde_json::to_value(schemars::schema_for!(T))
        .expect("serialize generated convergence schema");
    let mut constants = BTreeSet::new();
    visit(&document, property, &mut constants);
    constants
}

#[test]
fn resident_inventory_activates_only_the_new_convergence_roots() {
    let active = RESIDENT_PUBLIC_ROOT_SCHEMA_IDS_V1
        .into_iter()
        .collect::<BTreeSet<_>>();

    for current in [
        "capability-revocation-checkpoint.v1",
        "capability-session-grant-revocation.v1",
        "capability-session-grant.v1",
        "event-acknowledgement.v1",
        "event-reference.v1",
        "execution-owner-fence.v1",
        "genealogy-edge.v1",
        "interaction-occurrence.v1",
        "local-route-binding.v1",
        "reconciliation-frontier.v1",
    ] {
        assert!(active.contains(current), "missing resident root {current}");
    }
    assert!(!active.contains("owner-event-reference.v1"));
}

#[test]
fn genealogy_truth_classes_are_closed_and_non_interchangeable() {
    assert_eq!(
        property_consts::<GenealogyProofV1>("truth_class"),
        BTreeSet::from([
            "pale_consequence_lineage".to_owned(),
            "native_structural".to_owned(),
            "semantic_inferred".to_owned(),
        ])
    );
}

#[test]
fn owner_placement_and_response_expectation_preserve_unknown() {
    assert_eq!(
        property_consts::<OwnerRecordKnowledgeV1>("state"),
        BTreeSet::from([
            "absent".to_owned(),
            "known".to_owned(),
            "unknown".to_owned(),
        ])
    );
    assert_eq!(
        property_consts::<InteractionResponseExpectationV1>("state"),
        BTreeSet::from([
            "expected".to_owned(),
            "none".to_owned(),
            "unknown".to_owned(),
        ])
    );
}
