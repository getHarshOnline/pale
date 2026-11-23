//! Recorded chains must retain the prepared call without rerunning its handler.

use pale_core::records::{
    ActionContractDigest, ActionErrorCode, ActionExecution, ActionReceiptDigest, ActionResultDigest,
};

use super::*;

fn recorded() -> ActionExecution {
    NativeActionRegistry::new()
        .unwrap()
        .execute_value(PATH_INVENTORY_ACTION_ID, value())
        .unwrap()
}

fn other_input() -> serde_json::Value {
    serde_json::json!({
        "kind": "path_inventory",
        "payload": {"style": "posix_relative", "paths": ["README.md"]},
    })
}

fn reseal_receipt(execution: &mut ActionExecution) {
    let mut preimage = serde_json::to_value(&execution.receipt).unwrap();
    preimage.as_object_mut().unwrap().remove("receipt_digest");
    execution.receipt.receipt_digest = ActionReceiptDigest::hash_canonical(&preimage).unwrap();
}

fn reseal_output(execution: &mut ActionExecution) {
    let mut preimage = serde_json::to_value(&execution.result).unwrap();
    preimage.as_object_mut().unwrap().remove("result_digest");
    execution.result.result_digest = ActionResultDigest::hash_canonical(&preimage).unwrap();
    execution.receipt.result_digest = execution.result.result_digest.clone();
    reseal_receipt(execution);
}

fn assert_rejected(
    call: &super::super::PreparedNativeAction<'_>,
    execution: &ActionExecution,
    expected: ActionErrorCode,
) {
    let error = call.validate_recorded_execution(execution).unwrap_err();
    match error {
        NativeActionInvocationError::Action(error) => {
            assert_eq!(error.code, expected);
            assert_eq!(
                error.request_digest.as_ref(),
                Some(&call.request().request_digest)
            );
        }
        other => panic!("expected {expected:?}, got {other:?}"),
    }
    assert!(HANDLER_INPUTS.with_borrow(Vec::is_empty));
}

#[test]
fn intact_recorded_chain_validates_without_dispatch_or_request_mutation() {
    let execution = recorded();
    let registry = observed_registry();
    let call = registry
        .prepare_value(PATH_INVENTORY_ACTION_ID, value())
        .unwrap();
    let request = call.request().clone();
    call.validate_recorded_execution(&execution).unwrap();
    call.validate_recorded_execution(&execution).unwrap();
    assert_eq!(call.request(), &request);
    assert!(HANDLER_INPUTS.with_borrow(Vec::is_empty));
}

#[test]
fn recorded_request_cannot_replace_prepared_input_or_contract() {
    let original = recorded();
    let other = NativeActionRegistry::new()
        .unwrap()
        .execute_value(PATH_INVENTORY_ACTION_ID, other_input())
        .unwrap();
    let registry = observed_registry();
    let call = registry
        .prepare_value(PATH_INVENTORY_ACTION_ID, value())
        .unwrap();

    let mut changed = original.clone();
    changed.request.input = other.request.input.clone();
    assert_rejected(&call, &changed, ActionErrorCode::DigestMismatch);
    // This alternative has internally valid self-digests throughout, but is
    // not the call whose input was retained at dispatch.
    assert_rejected(&call, &other, ActionErrorCode::DigestMismatch);
    changed = original.clone();
    changed.request.request_digest = other.request.request_digest;
    assert_rejected(&call, &changed, ActionErrorCode::DigestMismatch);
    changed = original.clone();
    changed.request.contract_digest = ActionContractDigest::hash_canonical(&"other").unwrap();
    assert_rejected(&call, &changed, ActionErrorCode::ContractMismatch);
    changed = original.clone();
    changed.request.action_id = "pale.compute.unavailable-history".into();
    assert_rejected(&call, &changed, ActionErrorCode::ContractMismatch);
    changed = original;
    changed.request.action_version = 2;
    assert!(matches!(
        call.validate_recorded_execution(&changed),
        Err(NativeActionInvocationError::MalformedInput(
            pale_contracts::SchemaError::Invalid(_)
        ))
    ));
    assert!(HANDLER_INPUTS.with_borrow(Vec::is_empty));
}

#[test]
fn result_and_receipt_must_keep_the_registered_contract_and_request() {
    let original = recorded();
    let registry = observed_registry();
    let call = registry
        .prepare_value(PATH_INVENTORY_ACTION_ID, value())
        .unwrap();
    type Change = fn(&mut ActionExecution);
    for change in [
        (|e: &mut ActionExecution| e.result.action_id = "pale.compute.other".into()) as Change,
        |e| e.receipt.action_id = "pale.compute.other".into(),
        |e| e.result.contract_digest = ActionContractDigest::hash_canonical(&"other").unwrap(),
        |e| e.receipt.contract_digest = ActionContractDigest::hash_canonical(&"other").unwrap(),
    ] {
        let mut changed = original.clone();
        change(&mut changed);
        reseal_output(&mut changed);
        assert_rejected(&call, &changed, ActionErrorCode::ContractMismatch);
    }
    for change in [
        (|e: &mut ActionExecution| {
            e.result.request_digest =
                pale_core::records::ActionRequestDigest::hash_canonical(&"other").unwrap();
        }) as Change,
        |e| {
            e.receipt.request_digest =
                pale_core::records::ActionRequestDigest::hash_canonical(&"other").unwrap();
        },
    ] {
        let mut changed = original.clone();
        change(&mut changed);
        reseal_output(&mut changed);
        assert_rejected(&call, &changed, ActionErrorCode::DigestMismatch);
    }
    let mut changed = original;
    changed.receipt.result_digest = ActionResultDigest::hash_canonical(&"unlike result").unwrap();
    reseal_receipt(&mut changed);
    assert_rejected(&call, &changed, ActionErrorCode::DigestMismatch);
}

#[test]
fn altered_output_digest_and_receipt_are_rejected_without_reexecution() {
    let original = recorded();
    let ranking = NativeActionRegistry::new().unwrap().execute_value(
        WEIGHTED_RANKING_ACTION_ID,
        serde_json::json!({"kind":"weighted_ranking","payload":{
            "weights":[{"id":"quality","weight_micros":1_000_000}],
            "candidates":[{"id":"one","scores":[{"weight_id":"quality","value_micros":500_000}]}]
        }}),
    ).unwrap();
    let registry = observed_registry();
    let call = registry
        .prepare_value(PATH_INVENTORY_ACTION_ID, value())
        .unwrap();
    let mut changed = original.clone();
    changed.result.output = ranking.result.output;
    reseal_output(&mut changed);
    assert_rejected(&call, &changed, ActionErrorCode::ContractMismatch);
    changed = original.clone();
    if let ActionOutput::PathInventory(output) = &mut changed.result.output {
        output.entries[0].normalized_path = "another.rs".into();
    } else {
        panic!("path fixture output");
    }
    assert_rejected(&call, &changed, ActionErrorCode::DigestMismatch);
    changed = original.clone();
    changed.result.result_digest = ActionResultDigest::hash_canonical(&"other").unwrap();
    changed.receipt.result_digest = changed.result.result_digest.clone();
    reseal_receipt(&mut changed);
    assert_rejected(&call, &changed, ActionErrorCode::DigestMismatch);
    changed = original;
    changed.receipt.receipt_digest = ActionReceiptDigest::hash_canonical(&"other").unwrap();
    assert_rejected(&call, &changed, ActionErrorCode::DigestMismatch);
}

#[test]
fn recorded_schema_identity_and_nested_versions_have_no_fallback() {
    let original = recorded();
    let registry = observed_registry();
    let call = registry
        .prepare_value(PATH_INVENTORY_ACTION_ID, value())
        .unwrap();
    type Change = fn(&mut ActionExecution);
    for change in [
        (|e: &mut ActionExecution| e.schema_version = 2) as Change,
        |e| e.schema = pale_core::records::SchemaId::parse("pale/unknown-execution.v1").unwrap(),
        |e| e.schema_sha256 = serde_json::from_value(serde_json::json!("0".repeat(64))).unwrap(),
        |e| e.request.schema = "pale/unknown-request.v1".into(),
        |e| e.result.schema_version = 2,
        |e| e.receipt.schema_version = 2,
        |e| e.result.action_version = 2,
        |e| e.receipt.action_version = 2,
    ] {
        let mut changed = original.clone();
        change(&mut changed);
        assert!(matches!(
            call.validate_recorded_execution(&changed),
            Err(NativeActionInvocationError::MalformedInput(
                pale_contracts::SchemaError::Invalid(_)
                    | pale_contracts::SchemaError::IdentityMismatch(_)
            ))
        ));
    }
    assert!(HANDLER_INPUTS.with_borrow(Vec::is_empty));
}

#[test]
fn coherent_output_claim_still_requires_independent_producer_evidence() {
    let mut execution = recorded();
    if let ActionOutput::PathInventory(output) = &mut execution.result.output {
        output.entries[0].normalized_path = "not-the-computed-path.rs".into();
    } else {
        panic!("path fixture output");
    }
    reseal_output(&mut execution);
    let registry = observed_registry();
    let call = registry
        .prepare_value(PATH_INVENTORY_ACTION_ID, value())
        .unwrap();
    // A coherent chain is not a proof of honest computation. The operation
    // owner must authenticate the result and its exact dispatched attempt.
    call.validate_recorded_execution(&execution).unwrap();
    assert!(HANDLER_INPUTS.with_borrow(Vec::is_empty));
}
