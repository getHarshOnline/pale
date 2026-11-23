//! Preparation must retain one selected call without invoking its handler.

use std::cell::RefCell;

use pale_core::records::{
    ActionError, ActionInput, ActionOutput, PATH_INVENTORY_ACTION_ID, WEIGHTED_RANKING_ACTION_ID,
};

use super::NativeActionRegistry;
use crate::actions::NativeActionInvocationError;

#[path = "native_ingress.rs"]
mod native_ingress;
#[path = "recorded.rs"]
mod recorded;

thread_local! {
    // Per-test-thread state records actual dispatch, not a production hook.
    static HANDLER_INPUTS: RefCell<Vec<ActionInput>> = const { RefCell::new(Vec::new()) };
}

fn record_path_call(input: &ActionInput) -> Result<ActionOutput, ActionError> {
    HANDLER_INPUTS.with_borrow_mut(|inputs| inputs.push(input.clone()));
    let original = super::execution::NATIVE_ACTION_DEFINITIONS
        .iter()
        .find(|definition| definition.id == PATH_INVENTORY_ACTION_ID)
        .expect("the path action is compiled");
    (original.execute)(input)
}

fn observed_registry() -> NativeActionRegistry {
    HANDLER_INPUTS.with_borrow_mut(Vec::clear);
    let mut registry = NativeActionRegistry::new().expect("coherent registry");
    registry
        .actions
        .get_mut(PATH_INVENTORY_ACTION_ID)
        .expect("the path action is registered")
        .execute = record_path_call;
    registry
}

fn value() -> serde_json::Value {
    serde_json::json!({
        "kind": "path_inventory",
        "payload": {"style": "posix_relative", "paths": ["src/./lib.rs"]},
    })
}

#[test]
fn preparation_and_drop_do_not_call_the_handler() {
    let registry = observed_registry();
    let call = registry
        .prepare_value(PATH_INVENTORY_ACTION_ID, value())
        .unwrap();
    assert!(HANDLER_INPUTS.with_borrow(Vec::is_empty));
    drop(call);
    assert!(HANDLER_INPUTS.with_borrow(Vec::is_empty));
}

#[test]
fn execution_consumes_the_exact_prepared_input_and_selected_handler() {
    let registry = observed_registry();
    let input: ActionInput = serde_json::from_value(value()).unwrap();
    let call = registry
        .prepare_value(PATH_INVENTORY_ACTION_ID, value())
        .unwrap();
    assert!(std::ptr::eq(
        call.contract(),
        registry.contract(PATH_INVENTORY_ACTION_ID).unwrap(),
    ));
    assert_eq!(call.request().action_id, PATH_INVENTORY_ACTION_ID);
    assert_eq!(call.request().input, input);
    assert_eq!(
        call.request().contract_digest,
        call.contract().contract_digest
    );
    let request = call.request().clone();
    let execution = call.execute().unwrap();
    assert_eq!(execution.request, request);
    HANDLER_INPUTS.with_borrow(|inputs| assert_eq!(inputs.as_slice(), &[input]));
}

#[test]
fn unknown_malformed_and_mismatched_inputs_do_not_call_the_handler() {
    let registry = observed_registry();
    assert!(matches!(
        registry.prepare_value("pale.compute.unknown", serde_json::Value::Null),
        Err(NativeActionInvocationError::Action(error))
            if error.code == pale_core::records::ActionErrorCode::UnknownAction
    ));
    assert!(matches!(
        registry.prepare_value(PATH_INVENTORY_ACTION_ID, serde_json::Value::Null),
        Err(NativeActionInvocationError::MalformedInput(_))
    ));
    assert!(matches!(
        registry.prepare_value(WEIGHTED_RANKING_ACTION_ID, value()),
        Err(NativeActionInvocationError::Action(error))
            if error.code == pale_core::records::ActionErrorCode::InputMismatch
    ));
    assert!(HANDLER_INPUTS.with_borrow(Vec::is_empty));
}
