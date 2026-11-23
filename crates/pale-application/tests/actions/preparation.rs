//! Public preparation preserves established wire bytes and typed failures.

use pale_application::actions::{NativeActionInvocationError, NativeActionRegistry};
use pale_core::records::{ActionErrorCode, PATH_INVENTORY_ACTION_ID, WEIGHTED_RANKING_ACTION_ID};

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/conformance/cross-leaf.v1/native-action-journey.json"
    )))
    .expect("the existing cross-leaf action fixture is valid")
}

#[test]
fn prepared_value_and_payload_preserve_existing_canonical_execution_bytes() {
    let registry = NativeActionRegistry::new().unwrap();
    let fixture = fixture();
    let input: serde_json::Value =
        serde_json::from_str(fixture["tagged_input"].as_str().unwrap()).unwrap();
    let payload = serde_json::from_str(fixture["payload_input"].as_str().unwrap()).unwrap();
    let prepared = registry
        .prepare_value(PATH_INVENTORY_ACTION_ID, input.clone())
        .unwrap();
    let request = prepared.request().clone();
    let outputs = [
        prepared.execute().unwrap(),
        registry
            .execute_value(PATH_INVENTORY_ACTION_ID, input)
            .unwrap(),
        registry
            .execute_payload(PATH_INVENTORY_ACTION_ID, payload)
            .unwrap(),
    ];
    for output in outputs {
        assert_eq!(output.request, request);
        let canonical = pale_core::canon::to_canonical_string(&output).unwrap();
        assert_eq!(
            canonical.len() as u64,
            fixture["execution_length"].as_u64().unwrap()
        );
        assert_eq!(
            pale_core::sha256_hex(canonical.as_bytes()),
            fixture["execution_sha256"].as_str().unwrap()
        );
    }
}

#[test]
fn modifying_a_request_copy_cannot_replace_the_retained_input_or_target() {
    let registry = NativeActionRegistry::new().unwrap();
    let input = serde_json::from_str(fixture()["tagged_input"].as_str().unwrap()).unwrap();
    let call = registry
        .prepare_value(PATH_INVENTORY_ACTION_ID, input)
        .unwrap();
    let expected = call.request().clone();
    let mut untrusted_copy = expected.clone();
    untrusted_copy.action_id = WEIGHTED_RANKING_ACTION_ID.into();
    let pale_core::records::ActionInput::PathInventory(input) = &mut untrusted_copy.input else {
        panic!("fixture is a path request");
    };
    input.paths.clear();
    assert_ne!(untrusted_copy, expected);
    assert_eq!(call.execute().unwrap().request, expected);
}

#[test]
fn handler_rejection_is_deferred_and_preserves_its_request_digest() {
    let registry = NativeActionRegistry::new().unwrap();
    let input = serde_json::json!({
        "kind": "path_inventory",
        "payload": {"style": "posix_relative", "paths": ["../escape"]},
    });
    let call = registry
        .prepare_value(PATH_INVENTORY_ACTION_ID, input.clone())
        .unwrap();
    let request_digest = call.request().request_digest.clone();
    let errors = [
        call.execute().unwrap_err(),
        registry
            .execute_value(PATH_INVENTORY_ACTION_ID, input.clone())
            .unwrap_err(),
        registry
            .execute_payload(
                PATH_INVENTORY_ACTION_ID,
                input["payload"].as_object().unwrap().clone(),
            )
            .unwrap_err(),
    ];
    let mut first = None;
    for error in errors {
        let NativeActionInvocationError::Action(error) = error else {
            panic!("lexical root escape is an action failure, not a schema failure");
        };
        assert_eq!(error.code, ActionErrorCode::InvalidPath);
        assert_eq!(error.request_digest.as_ref(), Some(&request_digest));
        let bytes = serde_json::to_vec(&error).unwrap();
        match &first {
            Some(expected) => assert_eq!(&bytes, expected),
            None => first = Some(bytes),
        }
    }
}

#[test]
fn preparation_preserves_schema_rejection_before_action_kind_matching() {
    let registry = NativeActionRegistry::new().unwrap();
    let input = serde_json::json!({
        "kind": "path_inventory",
        "payload": {"style": "PRIVATE-REJECTED-VALUE", "paths": []},
    });
    for target in [PATH_INVENTORY_ACTION_ID, WEIGHTED_RANKING_ACTION_ID] {
        let prepared_error = registry.prepare_value(target, input.clone()).unwrap_err();
        let execution_error = registry.execute_value(target, input.clone()).unwrap_err();
        assert_eq!(
            format!("{prepared_error:?}"),
            format!("{execution_error:?}")
        );
        let NativeActionInvocationError::MalformedInput(pale_contracts::SchemaError::Invalid(
            evidence,
        )) = &prepared_error
        else {
            panic!("input schema validation must precede action kind matching");
        };
        assert_eq!(evidence.violations()[0].instance_path(), "/payload/style");
        assert!(!format!("{prepared_error:?}").contains("PRIVATE-REJECTED-VALUE"));
    }
}
