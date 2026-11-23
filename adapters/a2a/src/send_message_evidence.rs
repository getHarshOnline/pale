//! Pure qualification of the selected unary send protocol, not transport authority.

use std::num::NonZeroUsize;

use pale_core::records::{A2aJsonRpcRequestIdV1, MAX_RESIDENT_ID_BYTES};
use pale_ports::{A2aExchangeError, A2aExchangeFacts, A2aSendMessageEvidencePort};
use serde_json::Value;

use crate::{
    decode_response,
    decoding::{decode_error, UniqueJson},
    wire, A2aMethod,
};

/// Stateless official-protocol reader for retained unary `SendMessage` evidence.
#[derive(Clone, Copy, Debug, Default)]
pub struct A2aSendMessageDecoder;

impl A2aSendMessageEvidencePort for A2aSendMessageDecoder {
    fn read_initial_response(
        &self,
        request: &[u8],
        response: &[u8],
        expected_request: &A2aJsonRpcRequestIdV1,
        maximum_bytes: NonZeroUsize,
    ) -> Result<A2aExchangeFacts, A2aExchangeError> {
        if request
            .len()
            .checked_add(response.len())
            .is_none_or(|length| length > maximum_bytes.get())
        {
            return Err(A2aExchangeError::LimitExceeded);
        }
        read_request(request, expected_request)?;
        let response = decode_response(
            A2aMethod::SendMessage,
            &expected_request.as_str().into(),
            response,
            maximum_bytes,
        )
        .map_err(decode_error)?;
        wire::unbound_facts(response)
    }
}

fn read_request(
    bytes: &[u8],
    expected_request: &A2aJsonRpcRequestIdV1,
) -> Result<(), A2aExchangeError> {
    let UniqueJson(Value::Object(mut envelope)) =
        serde_json::from_slice(bytes).map_err(|_| A2aExchangeError::Response)?
    else {
        return Err(A2aExchangeError::Response);
    };
    if envelope.len() != 4
        || envelope.remove("jsonrpc") != Some(Value::String("2.0".into()))
        || envelope.remove("method") != Some(Value::String(a2a::methods::SEND_MESSAGE.into()))
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
    if &request_id != expected_request {
        return Err(A2aExchangeError::CoordinateMismatch);
    }
    let Some(Value::Object(params)) = envelope.remove("params") else {
        return Err(A2aExchangeError::Response);
    };
    let params: a2a::SendMessageRequest =
        serde_json::from_value(Value::Object(params)).map_err(|_| A2aExchangeError::Response)?;
    let configuration = params.configuration.ok_or(A2aExchangeError::Response)?;
    if params.tenant.is_some()
        || params.metadata.is_some()
        || params.message.message_id.is_empty()
        || params.message.message_id.len() > MAX_RESIDENT_ID_BYTES
        || params.message.role != a2a::Role::User
        || params.message.context_id.is_some()
        || params.message.task_id.is_some()
        || params.message.metadata.is_some()
        || params.message.extensions.is_some()
        || params.message.reference_task_ids.is_some()
        || params.message.parts.len() != 1
        || configuration.accepted_output_modes.as_deref() != Some(&["text/plain".to_owned()])
        || configuration.task_push_notification_config.is_some()
        || configuration.history_length != Some(0)
        || configuration.return_immediately != Some(true)
    {
        return Err(A2aExchangeError::Response);
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/internal/send_message_evidence.rs"]
mod tests;
