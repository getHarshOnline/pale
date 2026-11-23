//! Mechanical serialization laws, not admitted application fixtures.

use super::*;
use serde_json::{json, Value};

fn text_input() -> A2aExchangeInput<'static> {
    A2aExchangeInput::Text {
        mode: A2aTaskRequestModeV1::SendMessage,
        message_id: "message-native",
        text: "quoted \"value\"\nα",
    }
}

#[test]
fn selected_request_has_exact_ids_and_no_unselected_input() {
    let mut bytes = Vec::new();
    encode_input("request-native", text_input(), &mut bytes).unwrap();
    assert!(bytes.starts_with(b"{\n  \"jsonrpc\": \"2.0\","));
    let actual: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        actual,
        json!({
            "jsonrpc":"2.0", "id":"request-native", "method":a2a::methods::SEND_MESSAGE,
            "params": {
                "message": {"messageId":"message-native", "role":"ROLE_USER", "parts":[{"text":"quoted \"value\"\nα"}]},
                "configuration": {"acceptedOutputModes":["text/plain"], "historyLength":0, "returnImmediately":true}
            }
        })
    );
    let mut again = Vec::new();
    encode_input("request-native", text_input(), &mut again).unwrap();
    assert_eq!(bytes, again);
}

#[test]
fn retained_bytes_cannot_be_substituted_or_extended() {
    let mut bytes = Vec::new();
    encode_input("request-native", text_input(), &mut bytes).unwrap();
    let mut writer = MatchingWriter { remaining: &bytes };
    encode_input("request-native", text_input(), &mut writer).unwrap();
    assert!(writer.remaining.is_empty());

    for end in 0..bytes.len() {
        let mut writer = MatchingWriter {
            remaining: &bytes[..end],
        };
        assert!(encode_input("request-native", text_input(), &mut writer).is_err());
    }
    for changed in 0..bytes.len() {
        let mut substituted = bytes.clone();
        substituted[changed] ^= 1;
        let mut writer = MatchingWriter {
            remaining: &substituted,
        };
        assert!(encode_input("request-native", text_input(), &mut writer).is_err());
    }
    let mut writer = MatchingWriter { remaining: &bytes };
    assert!(encode_input("request-other", text_input(), &mut writer).is_err());
    bytes.push(b' ');
    let mut writer = MatchingWriter { remaining: &bytes };
    encode_input("request-native", text_input(), &mut writer).unwrap();
    assert!(!writer.remaining.is_empty());
}

#[test]
fn escaping_is_charged_before_the_output_buffer_grows() {
    let mut expected = Vec::new();
    encode_input("request-native", text_input(), &mut expected).unwrap();
    for maximum in 0..expected.len() {
        let mut writer = LimitedWriter {
            bytes: Vec::new(),
            maximum,
        };
        assert!(encode_input("request-native", text_input(), &mut writer).is_err());
        assert!(writer.bytes.len() <= maximum);
    }
    let mut writer = LimitedWriter {
        bytes: Vec::new(),
        maximum: expected.len(),
    };
    encode_input("request-native", text_input(), &mut writer).unwrap();
    assert_eq!(writer.bytes, expected);
}

#[test]
fn task_queries_keep_task_and_request_identity_distinct() {
    let task = A2aRemoteTaskIdV1::parse("task-native".to_owned()).unwrap();
    let context = A2aRemoteContextIdV1::parse("context-native".to_owned()).unwrap();
    for (input, method, params) in [
        (
            A2aExchangeInput::GetTask {
                task: &task,
                context: &context,
            },
            a2a::methods::GET_TASK,
            json!({"id":"task-native","historyLength":0}),
        ),
        (
            A2aExchangeInput::CancelTask {
                task: &task,
                context: &context,
            },
            a2a::methods::CANCEL_TASK,
            json!({"id":"task-native"}),
        ),
    ] {
        let mut bytes = Vec::new();
        encode_input("request-native", input, &mut bytes).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&bytes).unwrap(),
            json!({"jsonrpc":"2.0","id":"request-native","method":method,"params":params})
        );
        if method == a2a::methods::GET_TASK {
            use pale_ports::A2aGetTaskEvidencePort;
            let response = serde_json::to_vec_pretty(&json!({
                "jsonrpc":"2.0", "id":"request-native", "result": {
                    "id":"task-native", "contextId":"context-native",
                    "status":{"state":"TASK_STATE_WORKING"}
                }
            }))
            .unwrap();
            assert_eq!(
                crate::A2aGetTaskDecoder.read_task_state(
                    &bytes,
                    &response,
                    &task,
                    &context,
                    std::num::NonZeroUsize::new(bytes.len() + response.len()).unwrap(),
                ),
                Ok(A2aRemoteTaskStateV1::Working),
            );
        }
    }
}
