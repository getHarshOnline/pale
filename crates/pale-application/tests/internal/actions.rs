//! Native-action registry proofs for exact dispatch, computation lineage, and wire admission.

use pale_core::canon;
use pale_core::records::{
    ActionBoundary, ActionEffect, ActionError, ActionErrorCode, ActionExecution, ActionInput,
    PathInventoryInput, PathStyle, RankingCandidate, RankingScore, RankingWeight,
    WeightedRankingInput, COMPLIANCE_TRACE_GRADE_ACTION_ID, CONFIG_INVARIANT_SCAN_ACTION_ID,
    COST_AGGREGATE_ACTION_ID, EVIDENCE_SELECTION_ACTION_ID, LOOP_STATUS_REDUCE_ACTION_ID,
    PATH_INVENTORY_ACTION_ID, REVIEW_CONVERGENCE_REDUCE_ACTION_ID, RUBRIC_EVALUATION_ACTION_ID,
    SERVICE_DEFINITION_NORMALIZE_ACTION_ID, SKILL_HEALTH_AGGREGATE_ACTION_ID,
    WEIGHTED_RANKING_ACTION_ID,
};

use super::{NativeActionInvocationError, NativeActionRegistry};

fn registry() -> NativeActionRegistry {
    NativeActionRegistry::new().expect("native registry must be internally coherent")
}

fn execute(action_id: &str, input: ActionInput) -> ActionExecution {
    registry()
        .execute_value(
            action_id,
            serde_json::to_value(input).expect("typed action input serializes"),
        )
        .expect("valid action executes")
}

fn rejection(action_id: &str, input: ActionInput) -> ActionError {
    match registry()
        .execute_value(
            action_id,
            serde_json::to_value(input).expect("typed action input serializes"),
        )
        .expect_err("action input must be rejected")
    {
        NativeActionInvocationError::Action(error) => error,
        internal => panic!("expected a domain rejection, got {internal}"),
    }
}

fn digest_without_field(record: &impl serde::Serialize, field: &str) -> String {
    let mut value = serde_json::to_value(record).expect("record serializes");
    let object = value.as_object_mut().expect("sealed record is an object");
    object.remove(field).expect("digest field is present");
    canon::digest(&value).expect("record preimage is canonical")
}

#[test]
fn registry_is_a_sealed_exact_bijection() {
    let registry = registry();
    let contracts = registry.contracts().collect::<Vec<_>>();
    assert_eq!(
        contracts
            .iter()
            .map(|contract| contract.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            COMPLIANCE_TRACE_GRADE_ACTION_ID,
            CONFIG_INVARIANT_SCAN_ACTION_ID,
            COST_AGGREGATE_ACTION_ID,
            EVIDENCE_SELECTION_ACTION_ID,
            LOOP_STATUS_REDUCE_ACTION_ID,
            PATH_INVENTORY_ACTION_ID,
            REVIEW_CONVERGENCE_REDUCE_ACTION_ID,
            RUBRIC_EVALUATION_ACTION_ID,
            SERVICE_DEFINITION_NORMALIZE_ACTION_ID,
            SKILL_HEALTH_AGGREGATE_ACTION_ID,
            WEIGHTED_RANKING_ACTION_ID,
        ]
    );
    for contract in contracts {
        assert_eq!(contract.effect, ActionEffect::Compute);
        assert_eq!(contract.boundary, ActionBoundary::Pure);
        assert_eq!(
            contract.contract_digest.as_str(),
            digest_without_field(contract, "contract_digest")
        );
        let schema = registry
            .payload_schema(&contract.id)
            .expect("every contract has one sealed payload projection");
        assert!(
            !schema.contains_key("$ref"),
            "the published tool schema must inline its action-specific root"
        );
        assert_eq!(schema.get("type"), Some(&serde_json::json!("object")));
        assert_local_schema_references_resolve(&schema);
    }
}

/// Prove that every retained local reference resolves inside one tool schema.
fn assert_local_schema_references_resolve(schema: &serde_json::Map<String, serde_json::Value>) {
    fn visit(root: &serde_json::Value, value: &serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                for (key, value) in object {
                    if key == "$ref" {
                        let reference = value.as_str().expect("schema $ref is a string");
                        let pointer = reference
                            .strip_prefix('#')
                            .expect("tool schema carries only local references");
                        assert!(
                            root.pointer(pointer).is_some(),
                            "tool schema reference `{reference}` resolves locally"
                        );
                    } else {
                        visit(root, value);
                    }
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    visit(root, value);
                }
            }
            serde_json::Value::Null
            | serde_json::Value::Bool(_)
            | serde_json::Value::Number(_)
            | serde_json::Value::String(_) => {}
        }
    }

    let root = serde_json::Value::Object(schema.clone());
    visit(&root, &root);
}

#[test]
fn path_inventory_tool_schema_is_the_exact_self_contained_payload_shape() {
    let schema = registry()
        .payload_schema(PATH_INVENTORY_ACTION_ID)
        .expect("path inventory has a payload projection");
    assert_eq!(
        schema.get("required"),
        Some(&serde_json::json!(["style", "paths"]))
    );
    let schema = serde_json::Value::Object(schema);
    assert_eq!(
        schema["$defs"]
            .as_object()
            .expect("path inventory carries its reachable definition registry")
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["PathStyle"],
        "tool projection must not leak unrelated action definitions"
    );
    assert_eq!(
        schema.pointer("/$defs/PathStyle/oneOf/0/const"),
        Some(&serde_json::json!("posix_relative"))
    );
}

#[test]
fn execution_seals_the_complete_computation_chain() {
    let execution = execute(
        PATH_INVENTORY_ACTION_ID,
        ActionInput::PathInventory(PathInventoryInput {
            style: PathStyle::PosixRelative,
            paths: vec!["src/./lib.rs".into()],
        }),
    );

    assert_eq!(
        execution.request.request_digest.as_str(),
        digest_without_field(&execution.request, "request_digest")
    );
    assert_eq!(
        execution.result.result_digest.as_str(),
        digest_without_field(&execution.result, "result_digest")
    );
    assert_eq!(
        execution.receipt.receipt_digest.as_str(),
        digest_without_field(&execution.receipt, "receipt_digest")
    );
    assert_eq!(
        execution.result.request_digest,
        execution.request.request_digest
    );
    assert_eq!(
        execution.receipt.request_digest,
        execution.request.request_digest
    );
    assert_eq!(
        execution.receipt.result_digest,
        execution.result.result_digest
    );
}

fn ranking_input() -> WeightedRankingInput {
    WeightedRankingInput {
        weights: vec![RankingWeight {
            id: "quality".into(),
            weight_micros: 1_000_000,
        }],
        candidates: vec![RankingCandidate {
            id: "candidate".into(),
            scores: vec![RankingScore {
                weight_id: "quality".into(),
                value_micros: 1_000_000,
            }],
        }],
    }
}

#[test]
fn public_admission_rejects_mismatched_kinds_and_unsafe_numbers() {
    assert_eq!(
        rejection(
            PATH_INVENTORY_ACTION_ID,
            ActionInput::WeightedRanking(ranking_input())
        )
        .code,
        ActionErrorCode::InputMismatch
    );

    let mut unsafe_integer = ranking_input();
    unsafe_integer.candidates[0].scores[0].value_micros = i64::MAX;
    assert!(matches!(
        registry().execute_value(
            WEIGHTED_RANKING_ACTION_ID,
            serde_json::to_value(ActionInput::WeightedRanking(unsafe_integer)).unwrap(),
        ),
        Err(NativeActionInvocationError::MalformedInput(
            pale_contracts::SchemaError::Invalid(_)
        ))
    ));
}

#[test]
fn malformed_payload_explains_its_actual_tag_before_action_kind_matching() {
    let registry = registry();
    let input = serde_json::json!({
        "kind": "path_inventory",
        "payload": {"style": "PRIVATE-REJECTED-VALUE", "paths": []},
    });
    for action in [PATH_INVENTORY_ACTION_ID, WEIGHTED_RANKING_ACTION_ID] {
        let error = registry.execute_value(action, input.clone()).unwrap_err();
        let NativeActionInvocationError::MalformedInput(pale_contracts::SchemaError::Invalid(
            violations,
        )) = &error
        else {
            panic!("input schema admission must precede action-kind matching");
        };
        assert_eq!(violations.violations()[0].instance_path(), "/payload/style");
        assert!(violations.violations()[0]
            .expectation()
            .contains("posix_relative"));
        let source = std::error::Error::source(&error)
            .and_then(|source| source.downcast_ref::<pale_contracts::SchemaError>())
            .expect("schema evidence remains a typed source");
        assert_eq!(error.to_string(), source.to_string());
        assert!(!format!("{error:?} {error}").contains("PRIVATE-REJECTED-VALUE"));
    }
    let mut unknown_tag = input;
    unknown_tag["kind"] = serde_json::json!("unknown");
    let NativeActionInvocationError::MalformedInput(pale_contracts::SchemaError::Invalid(
        violations,
    )) = registry
        .execute_value(PATH_INVENTORY_ACTION_ID, unknown_tag)
        .unwrap_err()
    else {
        panic!("an unknown tag must remain a schema rejection");
    };
    assert_eq!(violations.violations()[0].instance_path(), "");
}
