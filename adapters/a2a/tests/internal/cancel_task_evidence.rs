//! Exact-byte cancellation read-back qualification through the official decoder.

use super::*;
use serde_json::{json, Value};

fn request() -> Value {
    json!({"jsonrpc":"2.0","id":"cancel-1","method":"CancelTask","params":{"id":"task-1"}})
}

fn response(state: a2a::TaskState) -> Value {
    json!({"jsonrpc":"2.0","id":"cancel-1","result":{"id":"task-1","contextId":"context-1","status":{"state":state}}})
}

fn bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec_pretty(value).unwrap()
}

fn read(
    request: &[u8],
    response: &[u8],
    maximum: usize,
) -> Result<A2aRemoteTaskStateV1, A2aExchangeError> {
    A2aCancelTaskDecoder.read_task_state_after_cancel(
        request,
        response,
        &A2aRemoteTaskIdV1::parse("task-1".to_owned()).unwrap(),
        &A2aRemoteContextIdV1::parse("context-1".to_owned()).unwrap(),
        NonZeroUsize::new(maximum).unwrap(),
    )
}

fn read_request(request: &[u8], maximum: usize) -> Result<A2aJsonRpcRequestIdV1, A2aExchangeError> {
    A2aCancelTaskDecoder.read_cancel_task_request(
        request,
        &A2aRemoteTaskIdV1::parse("task-1".to_owned()).unwrap(),
        NonZeroUsize::new(maximum).unwrap(),
    )
}

#[test]
fn request_only_reader_preserves_the_official_cancel_task_identity() {
    let request =
        br#" { "params":{"id":"task-1"}, "method":"CancelTask", "id":"cancel-1", "jsonrpc":"2.0" } "#;
    assert_eq!(
        read_request(request, request.len()),
        Ok(A2aJsonRpcRequestIdV1::parse("cancel-1".to_owned()).unwrap())
    );
}

#[test]
fn request_only_reader_rejects_other_json_rpc_roles_and_methods() {
    for rejected in [
        json!([request()]),
        json!({"jsonrpc":"2.0","method":"CancelTask","params":{"id":"task-1"}}),
        json!({"jsonrpc":"2.0","id":"cancel-1","result":{"id":"task-1"}}),
        json!({"jsonrpc":"2.0","id":"cancel-1","method":"SendMessage","params":{"id":"task-1"}}),
        json!({"jsonrpc":"2.0","id":"cancel-1","method":"GetTask","params":{"id":"task-1"}}),
    ] {
        assert!(read_request(&bytes(&rejected), 8192).is_err());
    }
}

#[test]
fn request_only_reader_rejects_unlike_or_extended_cancel_requests() {
    let mut wrong_task = request();
    wrong_task["params"]["id"] = json!("other-task");
    assert_eq!(
        read_request(&bytes(&wrong_task), 8192),
        Err(A2aExchangeError::CoordinateMismatch)
    );
    for (key, value) in [
        ("metadata", json!({})),
        ("contextId", json!("context-1")),
        ("tenant", json!("tenant-1")),
    ] {
        let mut extended = request();
        extended["params"][key] = value;
        assert_eq!(
            read_request(&bytes(&extended), 8192),
            Err(A2aExchangeError::Response)
        );
    }
}

#[test]
fn request_only_reader_bounds_bytes_before_protocol_qualification() {
    let request = bytes(&request());
    assert_eq!(
        read_request(&request, request.len() - 1),
        Err(A2aExchangeError::LimitExceeded)
    );
    let duplicate = br#"{"jsonrpc":"2.0","id":"cancel-1","method":"CancelTask","params":{"id":"task-1","id":"task-1"}}"#;
    assert_eq!(
        read_request(duplicate, duplicate.len()),
        Err(A2aExchangeError::Response)
    );
}

#[test]
fn exact_cancel_request_and_task_readback_are_preserved() {
    let request = br#" { "params":{"id":"task-1"}, "method":"CancelTask", "id":"cancel-1", "jsonrpc":"2.0" } "#;
    for (wire, expected) in [
        (a2a::TaskState::Working, A2aRemoteTaskStateV1::Working),
        (a2a::TaskState::Canceled, A2aRemoteTaskStateV1::Canceled),
        (a2a::TaskState::Completed, A2aRemoteTaskStateV1::Completed),
    ] {
        let response = bytes(&response(wire));
        assert_eq!(
            read(request, &response, request.len() + response.len()),
            Ok(expected)
        );
    }
}

#[test]
fn get_task_substitution_and_extra_cancel_parameters_are_rejected() {
    let response = bytes(&response(a2a::TaskState::Canceled));
    let mut changed = request();
    changed["method"] = json!("GetTask");
    assert_eq!(
        read(&bytes(&changed), &response, 8192),
        Err(A2aExchangeError::Response)
    );
    for (key, value) in [
        ("metadata", json!({})),
        ("tenant", json!("tenant")),
        ("historyLength", json!(0)),
    ] {
        let mut changed = request();
        changed["params"][key] = value;
        assert_eq!(
            read(&bytes(&changed), &response, 8192),
            Err(A2aExchangeError::Response)
        );
    }
}

#[test]
fn request_response_task_and_context_coordinates_are_independent_fences() {
    let exact_request = request();
    let exact_response = response(a2a::TaskState::Canceled);
    for (pointer, value) in [
        ("/id", json!("cancel-2")),
        ("/params/id", json!("other-task")),
    ] {
        let mut changed = exact_request.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        assert_eq!(
            read(&bytes(&changed), &bytes(&exact_response), 8192),
            Err(A2aExchangeError::CoordinateMismatch)
        );
    }
    for (pointer, value) in [
        ("/id", json!("cancel-2")),
        ("/result/id", json!("other-task")),
        ("/result/contextId", json!("other-context")),
    ] {
        let mut changed = exact_response.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        assert_eq!(
            read(&bytes(&exact_request), &bytes(&changed), 8192),
            Err(A2aExchangeError::CoordinateMismatch)
        );
    }
}

#[test]
fn limits_remote_errors_and_ambiguous_json_cannot_become_task_state() {
    let request = bytes(&request());
    let response = bytes(&response(a2a::TaskState::Canceled));
    assert_eq!(
        read(&request, &response, request.len() + response.len() - 1),
        Err(A2aExchangeError::LimitExceeded)
    );
    let error =
        bytes(&json!({"jsonrpc":"2.0","id":"cancel-1","error":{"code":-32000,"message":"secret"}}));
    assert_eq!(
        read(&request, &error, 8192),
        Err(A2aExchangeError::Response)
    );
    let duplicate = br#"{"jsonrpc":"2.0","id":"cancel-1","method":"CancelTask","params":{"id":"task-1","id":"task-1"}}"#;
    assert_eq!(
        read(duplicate, &response, 8192),
        Err(A2aExchangeError::Response)
    );
}
