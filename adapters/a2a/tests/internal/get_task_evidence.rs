//! Exact-byte protocol qualification through the same official decoder as transport.

use super::*;
use pale_core::records::MAX_RESIDENT_ID_BYTES;
use serde_json::{json, Value};

fn request() -> Value {
    json!({"jsonrpc":"2.0","id":"rpc-1","method":"GetTask","params":{"id":"task-1","historyLength":0}})
}

fn response(state: a2a::TaskState) -> Value {
    json!({"jsonrpc":"2.0","id":"rpc-1","result":{"id":"task-1","contextId":"context-1","status":{"state":state}}})
}

fn bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec_pretty(value).unwrap()
}

fn read(
    request: &[u8],
    response: &[u8],
    maximum: usize,
) -> Result<A2aRemoteTaskStateV1, A2aExchangeError> {
    A2aGetTaskDecoder.read_task_state(
        request,
        response,
        &A2aRemoteTaskIdV1::parse("task-1".to_owned()).unwrap(),
        &A2aRemoteContextIdV1::parse("context-1".to_owned()).unwrap(),
        NonZeroUsize::new(maximum).unwrap(),
    )
}

#[test]
fn official_task_states_and_noncanonical_object_order_are_preserved() {
    let request = br#" { "params":{"historyLength":0,"id":"task-1"}, "method":"GetTask", "id":"\u0072pc-1", "jsonrpc":"2.0" } "#;
    for (wire, expected) in [
        (
            a2a::TaskState::Unspecified,
            A2aRemoteTaskStateV1::Unspecified,
        ),
        (a2a::TaskState::Submitted, A2aRemoteTaskStateV1::Submitted),
        (a2a::TaskState::Working, A2aRemoteTaskStateV1::Working),
        (
            a2a::TaskState::InputRequired,
            A2aRemoteTaskStateV1::InputRequired,
        ),
        (
            a2a::TaskState::AuthRequired,
            A2aRemoteTaskStateV1::AuthRequired,
        ),
        (a2a::TaskState::Completed, A2aRemoteTaskStateV1::Completed),
        (a2a::TaskState::Canceled, A2aRemoteTaskStateV1::Canceled),
        (a2a::TaskState::Failed, A2aRemoteTaskStateV1::Failed),
        (a2a::TaskState::Rejected, A2aRemoteTaskStateV1::Rejected),
    ] {
        let response = serde_json::to_string_pretty(&response(wire)).unwrap();
        assert_eq!(
            read(request, response.as_bytes(), request.len() + response.len()),
            Ok(expected)
        );
    }
}

#[test]
fn aggregate_limit_precedes_parsing_and_has_an_exact_boundary() {
    let request = bytes(&request());
    let response = bytes(&response(a2a::TaskState::Working));
    let total = request.len() + response.len();
    assert_eq!(
        read(&request, &response, total),
        Ok(A2aRemoteTaskStateV1::Working)
    );
    assert_eq!(
        read(&request, &response, total - 1),
        Err(A2aExchangeError::LimitExceeded)
    );
    // Each buffer individually fits this ceiling; the pair does not.
    assert_eq!(
        read(&request, &response, request.len().max(response.len())),
        Err(A2aExchangeError::LimitExceeded)
    );
    assert_eq!(
        read(b"not JSON", b"also invalid", 1),
        Err(A2aExchangeError::LimitExceeded)
    );
    assert_eq!(
        read(b"", &response, response.len()),
        Err(A2aExchangeError::Response)
    );
    assert_eq!(
        read(&request, b"", request.len()),
        Err(A2aExchangeError::Response)
    );
    let mut deep = vec![b'['; 150];
    deep.extend(vec![b']'; 150]);
    assert_eq!(
        read(&deep, &response, deep.len() + response.len()),
        Err(A2aExchangeError::Response)
    );
}

#[test]
fn request_profile_rejects_unselected_methods_parameters_and_identity_types() {
    let response = bytes(&response(a2a::TaskState::Working));
    let exact = request();
    let changes = [
        ("/jsonrpc", json!("1.0")),
        ("/method", json!("SendMessage")),
        ("/method", json!("CancelTask")),
        ("/method", json!("GetTask ")),
        ("/id", json!(null)),
        ("/id", json!(1)),
        ("/id", json!(false)),
        ("/id", json!("")),
        ("/id", json!("bad\nidentity")),
        ("/params/historyLength", json!(null)),
        ("/params/historyLength", json!(1)),
        ("/params/historyLength", json!(-1)),
        ("/params/historyLength", json!("0")),
        ("/params/id", json!(null)),
        ("/params/id", json!(7)),
        ("/params", json!([])),
    ];
    for (pointer, value) in changes {
        let mut changed = exact.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        assert_eq!(
            read(&bytes(&changed), &response, 8192),
            Err(A2aExchangeError::Response),
            "{pointer}"
        );
    }
    for key in ["jsonrpc", "method", "id", "params"] {
        let mut changed = exact.clone();
        changed.as_object_mut().unwrap().remove(key);
        assert_eq!(
            read(&bytes(&changed), &response, 8192),
            Err(A2aExchangeError::Response)
        );
    }
    for key in ["id", "historyLength"] {
        let mut changed = exact.clone();
        changed["params"].as_object_mut().unwrap().remove(key);
        assert_eq!(
            read(&bytes(&changed), &response, 8192),
            Err(A2aExchangeError::Response)
        );
    }
    for key in ["tenant", "metadata", "extra"] {
        let mut changed = exact.clone();
        changed["params"][key] = Value::Null;
        assert_eq!(
            read(&bytes(&changed), &response, 8192),
            Err(A2aExchangeError::Response)
        );
        let mut changed = exact.clone();
        changed[key] = Value::Null;
        assert_eq!(
            read(&bytes(&changed), &response, 8192),
            Err(A2aExchangeError::Response)
        );
    }
    let mut oversized = exact;
    oversized["id"] = json!("x".repeat(MAX_RESIDENT_ID_BYTES + 1));
    assert_eq!(
        read(&bytes(&oversized), &response, 8192),
        Err(A2aExchangeError::LimitExceeded)
    );
}

#[test]
fn request_correlation_and_both_task_coordinates_are_independent_fences() {
    let exact_request = request();
    let exact_response = response(a2a::TaskState::Working);
    let mut changed = exact_request.clone();
    changed["id"] = json!("rpc-2");
    assert_eq!(
        read(&bytes(&changed), &bytes(&exact_response), 8192),
        Err(A2aExchangeError::CoordinateMismatch)
    );
    changed = exact_request.clone();
    changed["params"]["id"] = json!("other-task");
    assert_eq!(
        read(&bytes(&changed), &bytes(&exact_response), 8192),
        Err(A2aExchangeError::CoordinateMismatch)
    );
    for (pointer, value) in [
        ("/id", json!("rpc-2")),
        ("/id", json!(1)),
        ("/result/id", json!("other-task")),
        ("/result/contextId", json!("other-context")),
    ] {
        let mut changed = exact_response.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        assert_eq!(
            read(&bytes(&exact_request), &bytes(&changed), 8192),
            Err(A2aExchangeError::CoordinateMismatch),
            "{pointer}"
        );
    }
}

#[test]
fn response_union_and_selected_payload_cannot_fabricate_a_task_state() {
    let request = bytes(&request());
    let exact = response(a2a::TaskState::Working);
    let mut error = exact.clone();
    error.as_object_mut().unwrap().remove("result");
    error["error"] = json!({"code":-32000,"message":"secret-peer-token"});
    let mut both = exact.clone();
    both["error"] = error["error"].clone();
    let mut extra = exact.clone();
    extra["extra"] = Value::Null;
    let mut metadata = exact.clone();
    metadata["result"]["metadata"] = json!({"secret":"peer"});
    let mut unknown_state = exact.clone();
    unknown_state["result"]["status"]["state"] = json!("TASK_STATE_NOT_SELECTED");
    for response in [
        error,
        both,
        extra,
        metadata,
        unknown_state,
        json!({"jsonrpc":"2.0","id":"rpc-1","result":null}),
        json!({"jsonrpc":"2.0","id":"rpc-1","result":{"task":exact["result"]}}),
        json!({"jsonrpc":"2.0","id":"rpc-1","result":{"message":{"messageId":"m-1"}}}),
        json!({"jsonrpc":"2.0","id":"rpc-1","result":{"statusUpdate":{"taskId":"task-1"}}}),
    ] {
        let error = read(&request, &bytes(&response), 8192).unwrap_err();
        assert_eq!(error, A2aExchangeError::Response);
        assert!(!format!("{error:?} {error}").contains("secret-peer-token"));
    }
}

#[test]
fn ambiguous_request_or_response_json_is_never_normalized_into_evidence() {
    let request = bytes(&request());
    let response = bytes(&response(a2a::TaskState::Working));
    for changed in [
        br#"{"jsonrpc":"2.0","id":"rpc-1","i\u0064":"rpc-1","method":"GetTask","params":{"id":"task-1","historyLength":0}}"#.as_slice(),
        br#"{"jsonrpc":"2.0","id":"rpc-1","method":"GetTask","params":{"id":"task-1","id":"task-1","historyLength":0}}"#,
        br#"{"jsonrpc":"2.0","id":"rpc-1","method":"GetTask","params":{"id":"task-1","historyLength":0,"historyLength":0}}"#,
        b"{} {}", b"\xff",
    ] {
        assert_eq!(read(changed, &response, 8192), Err(A2aExchangeError::Response));
    }
    for changed in [
        br#"{"jsonrpc":"2.0","id":"rpc-1","i\u0064":"rpc-1","result":{}}"#.as_slice(),
        br#"{"jsonrpc":"2.0","id":"rpc-1","result":{"id":"task-1","contextId":"context-1","status":{"state":"TASK_STATE_WORKING","state":"TASK_STATE_COMPLETED"}}}"#,
        b"{} {}", b"\xff",
    ] {
        assert_eq!(read(&request, changed, 8192), Err(A2aExchangeError::Response));
    }
}
