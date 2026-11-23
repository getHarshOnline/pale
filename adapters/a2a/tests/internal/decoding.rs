//! Wire-boundary laws using the selected official A2A DTOs.

use std::num::NonZeroUsize;

use crate::sse::A2aStreamEvent;
use crate::{
    decode_response, A2aDecodeError, A2aEventDecoder, A2aEventLimits, A2aMethod, A2aResponse,
};
use a2a::{JsonRpcId, SendMessageResponse, StreamResponse};
use serde_json::{json, Value};

fn bound(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}

#[test]
fn pull_decoder_drains_a_comment_chunk_without_inventing_an_event() {
    let mut decoder = A2aEventDecoder::new(
        "request-1".into(),
        A2aEventLimits {
            line_bytes: bound(4096),
            data_bytes: bound(16_384),
            stream_bytes: bound(65_536),
            events: bound(32),
        },
    )
    .unwrap();
    let mut chunk = b": keep-alive\r\n\r\n".as_slice();
    assert_eq!(decoder.next_event(&mut chunk).unwrap(), None);
    assert!(chunk.is_empty());
    decoder.finish().unwrap();
}

fn task() -> Value {
    json!({"id":"task-native", "contextId":"context-native", "status":{"state":"TASK_STATE_WORKING"}})
}

fn message() -> Value {
    json!({"messageId":"message-native", "role":"ROLE_AGENT", "parts":[{"text":"héllo 🌍"}]})
}

fn response(result: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({"jsonrpc":"2.0", "id":"request-native", "result":result})).unwrap()
}

#[test]
fn selected_payload_rejects_nontext_parts_even_when_upstream_would_choose_text() {
    for part in [
        json!({"raw":"c2VjcmV0"}),
        json!({"url":"file:///private/input"}),
        json!({"data":{"choice":1}}),
        json!({"text":"visible","raw":"c2VjcmV0"}),
        json!({"text":"visible","metadata":{}}),
        json!({"text":"visible","extensions":[]}),
    ] {
        let mut value = message();
        value["parts"] = json!([part]);
        assert_eq!(
            decode(A2aMethod::SendMessage, &response(json!({"message":value}))),
            Err(A2aDecodeError::InvalidPayload)
        );
    }
    let mut value = task();
    value["artifacts"] =
        json!([{"artifactId":"a","parts":[{"text":"safe","url":"https://example.invalid/file"}]}]);
    assert_eq!(
        decode(A2aMethod::GetTask, &response(value)),
        Err(A2aDecodeError::InvalidPayload)
    );
}

#[test]
fn selected_payload_does_not_confuse_reserved_words_in_text_with_fields() {
    let mut value = message();
    value["parts"] = json!([{"text":"raw url data metadata extensions"}]);
    assert!(decode(A2aMethod::SendMessage, &response(json!({"message":value}))).is_ok());
}

fn decode(method: A2aMethod, bytes: &[u8]) -> Result<A2aResponse, A2aDecodeError> {
    decode_response(
        method,
        &JsonRpcId::from("request-native"),
        bytes,
        bound(16_384),
    )
}

fn limits() -> A2aEventLimits {
    A2aEventLimits {
        line_bytes: bound(2048),
        data_bytes: bound(4096),
        stream_bytes: bound(16_384),
        events: bound(16),
    }
}

fn event(result: Value) -> Vec<u8> {
    let mut bytes = b"data: ".to_vec();
    bytes.extend(response(result));
    bytes.extend(b"\n\n");
    bytes
}

fn collect(
    chunks: &[&[u8]],
    limits: A2aEventLimits,
) -> (Vec<A2aStreamEvent>, Result<(), A2aDecodeError>) {
    let mut decoder = A2aEventDecoder::new(JsonRpcId::from("request-native"), limits).unwrap();
    let mut events = Vec::new();
    for chunk in chunks {
        let mut input = *chunk;
        loop {
            match decoder.next_event(&mut input) {
                Ok(Some(event)) => events.push(event),
                Ok(None) => break,
                Err(error) => return (events, Err(error)),
            }
        }
    }
    (events, decoder.finish())
}

#[test]
fn methods_preserve_the_official_task_message_and_four_stream_variants() {
    assert!(
        matches!(decode(A2aMethod::SendMessage, &response(json!({"task":task()}))).unwrap(), A2aResponse::Send(SendMessageResponse::Task(task)) if task.id == "task-native" && task.context_id == "context-native")
    );
    assert!(
        matches!(decode(A2aMethod::SendMessage, &response(json!({"message":message()}))).unwrap(), A2aResponse::Send(SendMessageResponse::Message(message)) if message.task_id.is_none())
    );
    for method in [A2aMethod::GetTask, A2aMethod::CancelTask] {
        assert!(
            matches!(decode(method, &response(task())).unwrap(), A2aResponse::Task(task) if task.status.state == a2a::TaskState::Working)
        );
    }
    let cases = [
        json!({"task":task()}),
        json!({"message":message()}),
        json!({"statusUpdate":{"taskId":"task-native","contextId":"context-native","status":{"state":"TASK_STATE_WORKING"}}}),
        json!({"artifactUpdate":{"taskId":"task-native","contextId":"context-native","artifact":{"artifactId":"artifact-native","parts":[]},"append":true,"lastChunk":false}}),
    ];
    for (index, value) in cases.into_iter().enumerate() {
        let expected: StreamResponse = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(
            decode(A2aMethod::SendStreamingMessage, &response(value)).unwrap(),
            A2aResponse::Stream(expected),
            "variant {index}"
        );
    }
}

#[test]
fn exact_request_identity_is_checked_even_for_remote_errors() {
    let error = json!({"jsonrpc":"2.0","id":"request-native","error":{"code":-32000,"message":"native error","data":null}});
    assert!(
        matches!(decode(A2aMethod::GetTask, &serde_json::to_vec(&error).unwrap()).unwrap(), A2aResponse::RemoteError(error) if error.code == -32000)
    );
    for id in [json!(null), json!("another"), json!(1)] {
        let mut value = error.clone();
        value["id"] = id;
        assert_eq!(
            decode(A2aMethod::GetTask, &serde_json::to_vec(&value).unwrap()),
            Err(A2aDecodeError::RequestIdentityMismatch)
        );
    }
    let numeric = serde_json::to_vec(&json!({"jsonrpc":"2.0","id":1,"result":task()})).unwrap();
    assert!(decode_response(
        A2aMethod::GetTask,
        &JsonRpcId::Number(1),
        &numeric,
        bound(1000)
    )
    .is_ok());
    assert_eq!(
        decode_response(
            A2aMethod::GetTask,
            &JsonRpcId::String("1".into()),
            &numeric,
            bound(1000)
        ),
        Err(A2aDecodeError::RequestIdentityMismatch)
    );
    assert_eq!(
        decode_response(A2aMethod::GetTask, &JsonRpcId::Null, &numeric, bound(1000)),
        Err(A2aDecodeError::NullRequestIdentity)
    );
}

#[test]
fn result_error_presence_does_not_normalize_explicit_null_into_absence() {
    let error = json!({"code":-32000,"message":"native"});
    for fields in [
        json!({}),
        json!({"result":null}),
        json!({"error":null}),
        json!({"result":task(),"error":null}),
        json!({"result":null,"error":error}),
        json!({"result":task(),"error":error}),
    ] {
        let mut envelope = json!({"jsonrpc":"2.0","id":"request-native"});
        envelope
            .as_object_mut()
            .unwrap()
            .extend(fields.as_object().unwrap().clone());
        assert_eq!(
            decode(A2aMethod::GetTask, &serde_json::to_vec(&envelope).unwrap()),
            Err(A2aDecodeError::InvalidResponseUnion)
        );
    }
}

#[test]
fn union_ambiguity_and_wrong_method_results_are_rejected_before_upstream_decoding() {
    for result in [
        json!({"task":task(),"message":message()}),
        json!({"task":task(),"statusUpdate":null}),
        json!({"task":null}),
        json!({"message":message(),"extension":{}}),
    ] {
        for method in [A2aMethod::SendMessage, A2aMethod::SendStreamingMessage] {
            assert_eq!(
                decode(method, &response(result.clone())),
                Err(A2aDecodeError::InvalidPayload)
            );
        }
    }
    assert_eq!(
        decode(A2aMethod::GetTask, &response(json!({"task":task()}))),
        Err(A2aDecodeError::InvalidPayload)
    );
    assert_eq!(
        decode(A2aMethod::SendMessage, &response(task())),
        Err(A2aDecodeError::InvalidPayload)
    );
}

#[test]
fn ambiguous_member_names_at_every_depth_and_trailing_json_are_rejected() {
    for bytes in [
        br#"{"jsonrpc":"2.0","jsonrpc":"2.0","id":"request-native","result":{}}"#.as_slice(),
        br#"{"jsonrpc":"2.0","id":"request-native","i\u0064":"request-native","result":{}}"#,
        br#"{"jsonrpc":"2.0","id":"request-native","result":{"task":{"id":"first","id":"second"}}}"#,
        br#"{"jsonrpc":"2.0","id":"request-native","error":{"code":1,"message":"x","data":[{"x":1,"x":2}]}}"#,
        br#"{} {}"#,
        b"\xff",
    ] {
        assert_eq!(decode(A2aMethod::GetTask, bytes), Err(A2aDecodeError::InvalidJson));
    }
}

#[test]
fn envelope_version_shape_and_identity_types_are_closed() {
    for value in [
        json!([]),
        json!({"jsonrpc":"1.0","id":"request-native","result":task()}),
        json!({"id":"request-native","result":task()}),
        json!({"jsonrpc":"2.0","result":task()}),
        json!({"jsonrpc":"2.0","id":1.5,"result":task()}),
        json!({"jsonrpc":"2.0","id":true,"result":task()}),
        json!({"jsonrpc":"2.0","id":"request-native","result":task(),"extra":true}),
    ] {
        assert_eq!(
            decode(A2aMethod::GetTask, &serde_json::to_vec(&value).unwrap()),
            Err(A2aDecodeError::InvalidEnvelope)
        );
    }
}

#[test]
fn unary_byte_ceiling_is_exact_and_errors_do_not_echo_peer_content() {
    let bytes = response(task());
    assert!(decode_response(
        A2aMethod::GetTask,
        &JsonRpcId::from("request-native"),
        &bytes,
        bound(bytes.len())
    )
    .is_ok());
    let error = decode_response(
        A2aMethod::GetTask,
        &JsonRpcId::from("request-native"),
        &bytes,
        bound(bytes.len() - 1),
    )
    .unwrap_err();
    assert_eq!(error, A2aDecodeError::LimitExceeded);
    assert!(!format!("{error:?}: {error}").contains("task-native"));
}

#[test]
fn sse_is_invariant_under_all_utf8_and_delimiter_chunk_boundaries() {
    let payload = String::from_utf8(response(json!({"message":message()}))).unwrap();
    let wire = format!("\u{feff}: comment\r\nid: owner-α\revent: update\ndata: {payload}\r\n\r\n")
        .into_bytes();
    let expected = collect(&[&wire], limits());
    assert_eq!(expected.1, Ok(()));
    assert_eq!(expected.0.len(), 1);
    assert_eq!(expected.0[0].event_id, "owner-α");
    assert_eq!(expected.0[0].event_type, "update");
    for split in 0..=wire.len() {
        assert_eq!(
            collect(&[&wire[..split], &wire[split..]], limits()),
            expected,
            "split {split}"
        );
    }
    let chunks = wire.chunks(1).collect::<Vec<_>>();
    assert_eq!(collect(&chunks, limits()), expected);
}

#[test]
fn sse_multiline_data_ids_resets_and_ignored_fields_follow_wire_rules() {
    let payload = String::from_utf8(response(json!({"task":task()}))).unwrap();
    let split = payload.find(',').unwrap() + 1;
    let wire = format!("id: native\nretry: 1\nunknown: ignored\ndata: {}\ndata: {}\n\nevent: discarded\n\nid: bad\0id\ndata: {payload}\n\nid\nevent\ndata: {payload}\n\n", &payload[..split], &payload[split..]);
    let (events, status) = collect(&[wire.as_bytes()], limits());
    assert_eq!(status, Ok(()));
    assert_eq!(
        events
            .iter()
            .map(|event| event.event_id.as_str())
            .collect::<Vec<_>>(),
        ["native", "native", ""]
    );
    assert!(events.iter().all(|event| event.event_type == "message"));
}

#[test]
fn sse_preserves_valid_prefix_before_a_later_failure_for_any_chunking() {
    let mut wire = event(json!({"task":task()}));
    wire.extend(b"data: {\"jsonrpc\":\"2.0\",\"id\":\"wrong\",\"error\":{\"code\":-1,\"message\":\"x\"}}\n\n");
    let expected = collect(&[&wire], limits());
    assert_eq!(expected.0.len(), 1);
    assert_eq!(expected.1, Err(A2aDecodeError::RequestIdentityMismatch));
    for split in 0..=wire.len() {
        assert_eq!(
            collect(&[&wire[..split], &wire[split..]], limits()),
            expected
        );
    }
}

#[test]
fn sse_limits_bound_lines_assembled_data_total_bytes_and_events() {
    let wire = event(json!({"task":task()}));
    for selected in [
        A2aEventLimits {
            line_bytes: bound(3),
            ..limits()
        },
        A2aEventLimits {
            data_bytes: bound(3),
            ..limits()
        },
        A2aEventLimits {
            stream_bytes: bound(wire.len() - 1),
            ..limits()
        },
    ] {
        assert_eq!(
            collect(&[&wire], selected).1,
            Err(A2aDecodeError::LimitExceeded)
        );
    }
    let selected = A2aEventLimits {
        data_bytes: bound(wire.len() - 8),
        line_bytes: bound(wire.len() - 2),
        stream_bytes: bound(wire.len()),
        events: bound(1),
    };
    assert_eq!(collect(&[&wire], selected).1, Ok(()));
    let mut double = wire.clone();
    double.extend(&wire);
    let result = collect(
        &[&double],
        A2aEventLimits {
            events: bound(1),
            ..limits()
        },
    );
    assert_eq!(result.0.len(), 1);
    assert_eq!(result.1, Err(A2aDecodeError::LimitExceeded));
    assert_eq!(
        collect(
            &[b": x\n: x\n: x\n"],
            A2aEventLimits {
                stream_bytes: bound(8),
                ..limits()
            }
        )
        .1,
        Err(A2aDecodeError::LimitExceeded)
    );
}

#[test]
fn sse_eof_never_dispatches_an_unterminated_event_and_failure_is_sticky() {
    let wire = event(json!({"task":task()}));
    let result = collect(&[&wire[..wire.len() - 1]], limits());
    assert!(result.0.is_empty());
    assert_eq!(result.1, Err(A2aDecodeError::TruncatedEvent));
    let mut decoder = A2aEventDecoder::new(JsonRpcId::from("request-native"), limits()).unwrap();
    let mut invalid = b"data: \xff\n".as_slice();
    assert_eq!(
        decoder.next_event(&mut invalid),
        Err(A2aDecodeError::InvalidUtf8)
    );
    let mut valid = wire.as_slice();
    assert_eq!(
        decoder.next_event(&mut valid),
        Err(A2aDecodeError::StreamRejected)
    );
    assert_eq!(decoder.finish(), Err(A2aDecodeError::StreamRejected));
    assert!(matches!(
        A2aEventDecoder::new(JsonRpcId::Null, limits()),
        Err(A2aDecodeError::NullRequestIdentity)
    ));
}
