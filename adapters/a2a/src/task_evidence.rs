//! Shared exact-byte mechanics for known-task unary read-backs.

use std::num::NonZeroUsize;

use pale_core::records::{
    A2aJsonRpcRequestIdV1, A2aRemoteContextIdV1, A2aRemoteTaskIdV1, A2aRemoteTaskStateV1,
    MAX_RESIDENT_ID_BYTES,
};
use pale_ports::A2aExchangeError;
use serde_json::{Map, Value};

use crate::{
    decode_response,
    decoding::{decode_error, UniqueJson},
    wire, A2aMethod, A2aResponse,
};

pub(super) fn read_task_request(
    bytes: &[u8],
    method: A2aMethod,
) -> Result<(A2aJsonRpcRequestIdV1, Map<String, Value>), A2aExchangeError> {
    let UniqueJson(Value::Object(mut envelope)) =
        serde_json::from_slice(bytes).map_err(|_| A2aExchangeError::Response)?
    else {
        return Err(A2aExchangeError::Response);
    };
    if envelope.len() != 4
        || envelope.remove("jsonrpc") != Some(Value::String("2.0".into()))
        || envelope.remove("method") != Some(Value::String(method.as_str().into()))
    {
        return Err(A2aExchangeError::Response);
    }
    let Some(Value::String(request_id)) = envelope.remove("id") else {
        return Err(A2aExchangeError::Response);
    };
    if request_id.len() > MAX_RESIDENT_ID_BYTES {
        return Err(A2aExchangeError::LimitExceeded);
    }
    let request_id =
        A2aJsonRpcRequestIdV1::parse(request_id).map_err(|_| A2aExchangeError::Response)?;
    let Some(Value::Object(params)) = envelope.remove("params") else {
        return Err(A2aExchangeError::Response);
    };
    Ok((request_id, params))
}

pub(super) fn read_task_state_response(
    method: A2aMethod,
    request_id: &A2aJsonRpcRequestIdV1,
    response: &[u8],
    expected_task: &A2aRemoteTaskIdV1,
    expected_context: &A2aRemoteContextIdV1,
    maximum_bytes: NonZeroUsize,
) -> Result<A2aRemoteTaskStateV1, A2aExchangeError> {
    let response = decode_response(method, &request_id.as_str().into(), response, maximum_bytes)
        .map_err(decode_error)?;
    let A2aResponse::Task(task) = response else {
        return Err(A2aExchangeError::Response);
    };
    let (task, context, state) = wire::task_facts(task)?;
    if &task != expected_task || &context != expected_context {
        return Err(A2aExchangeError::CoordinateMismatch);
    }
    Ok(state)
}
