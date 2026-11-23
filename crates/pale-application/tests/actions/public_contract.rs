//! Public action-contract proofs for sealed records, structured failures, and redacted debug output.

use pale_application::actions::{NativeActionInvocationError, NativeActionRegistry};
use pale_core::records::{
    ActionError, ActionErrorCode, ActionInput, ConfigTextRecord, PathInventoryInput, PathStyle,
    PATH_INVENTORY_ACTION_ID,
};

fn registry() -> NativeActionRegistry {
    NativeActionRegistry::new().expect("the compiled action catalog is coherent")
}

fn action_rejection<T>(result: Result<T, NativeActionInvocationError>) -> ActionError {
    match result {
        Err(NativeActionInvocationError::Action(error)) => error,
        Err(internal) => panic!("expected a domain rejection, got {internal}"),
        Ok(_) => panic!("expected the action boundary to reject the input"),
    }
}

fn assert_seal_is_required<T>(record: &T, field: &str)
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let mut value = serde_json::to_value(record).expect("sealed record serializes");
    value
        .as_object_mut()
        .expect("sealed record is an object")
        .remove(field);
    assert!(serde_json::from_value::<T>(value).is_err());
}

#[test]
fn execution_failures_retain_the_structured_action_envelope() {
    let unknown = action_rejection(
        registry().execute_payload("pale.compute.unknown", serde_json::Map::new()),
    );
    assert_eq!(unknown.code, ActionErrorCode::UnknownAction);
    assert_eq!(unknown.action_id.as_deref(), Some("pale.compute.unknown"));

    let unknown = action_rejection(registry().payload_schema("pale.compute.unknown"));
    assert_eq!(unknown.code, ActionErrorCode::UnknownAction);
    assert_eq!(unknown.action_id.as_deref(), Some("pale.compute.unknown"));

    let unknown =
        action_rejection(registry().execute_value("pale.compute.unknown", serde_json::Value::Null));
    assert_eq!(unknown.code, ActionErrorCode::UnknownAction);

    let malformed = registry()
        .execute_value(
            PATH_INVENTORY_ACTION_ID,
            serde_json::json!({
                "kind": "path_inventory",
                "payload": { "style": "posix_relative" }
            }),
        )
        .unwrap_err();
    let NativeActionInvocationError::MalformedInput(pale_contracts::SchemaError::Invalid(
        violations,
    )) = malformed
    else {
        panic!("malformed input must preserve the structured schema rejection");
    };
    assert_eq!(violations.violations()[0].instance_path(), "/payload");
    assert_eq!(violations.violations()[0].keyword(), "required");
}

#[test]
fn successful_wire_records_cannot_be_deserialized_without_their_seals() {
    let registry = registry();
    let execution = registry
        .execute_value(
            PATH_INVENTORY_ACTION_ID,
            serde_json::to_value(ActionInput::PathInventory(PathInventoryInput {
                style: PathStyle::PosixRelative,
                paths: vec!["src/./lib.rs".into()],
            }))
            .expect("typed input serializes"),
        )
        .expect("valid action executes");

    assert_seal_is_required(
        registry
            .contract(PATH_INVENTORY_ACTION_ID)
            .expect("registered contract"),
        "contract_digest",
    );
    assert_seal_is_required(&execution.request, "request_digest");
    assert_seal_is_required(&execution.result, "result_digest");
    assert_seal_is_required(&execution.receipt, "receipt_digest");

    let debug = format!("{execution:?}");
    assert!(debug.contains("PathInventory"));
    assert!(!debug.contains("src/./lib.rs"));
    assert!(!debug.contains("src/lib.rs"));
}

#[test]
fn configuration_text_is_redacted_from_debug_output() {
    let record = ConfigTextRecord {
        id: "settings".into(),
        text: "access_token = secret-value".into(),
    };

    let debug = format!("{record:?}");
    assert!(debug.contains("text_bytes"));
    assert!(!debug.contains("secret-value"));
}
