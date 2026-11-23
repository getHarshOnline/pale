//! Exact unary send evidence through the official protocol decoder.

use super::*;

const REQUEST: &[u8] = br#"{"jsonrpc":"2.0","id":"send-1","method":"SendMessage","params":{"message":{"messageId":"message-1","role":"ROLE_USER","parts":[{"text":"Inspect the selected workspace."}]},"configuration":{"acceptedOutputModes":["text/plain"],"historyLength":0,"returnImmediately":true}}}"#;
const TASK: &[u8] = br#"{"jsonrpc":"2.0","id":"send-1","result":{"task":{"id":"task-1","contextId":"context-1","status":{"state":"TASK_STATE_WORKING"}}}}"#;
const MESSAGE: &[u8] = br#"{"jsonrpc":"2.0","id":"send-1","result":{"message":{"messageId":"message-remote","role":"ROLE_AGENT","parts":[]}}}"#;

#[test]
fn exact_request_and_response_bytes_determine_initial_facts() {
    let decoder = A2aSendMessageDecoder;
    let request = A2aJsonRpcRequestIdV1::parse("send-1").unwrap();
    let maximum = NonZeroUsize::new(8_192).unwrap();
    assert!(matches!(
        decoder
            .read_initial_response(REQUEST, TASK, &request, maximum)
            .unwrap(),
        A2aExchangeFacts::Task { .. }
    ));
    assert_eq!(
        decoder
            .read_initial_response(REQUEST, MESSAGE, &request, maximum)
            .unwrap(),
        A2aExchangeFacts::DirectMessage
    );
}

#[test]
fn unlike_request_identity_or_response_bytes_are_rejected() {
    let decoder = A2aSendMessageDecoder;
    let maximum = NonZeroUsize::new(8_192).unwrap();
    let unlike = A2aJsonRpcRequestIdV1::parse("send-other").unwrap();
    assert_eq!(
        decoder.read_initial_response(REQUEST, TASK, &unlike, maximum),
        Err(A2aExchangeError::CoordinateMismatch)
    );
    assert_eq!(
        decoder.read_initial_response(REQUEST, b"{}", &unlike, maximum),
        Err(A2aExchangeError::CoordinateMismatch)
    );
}
