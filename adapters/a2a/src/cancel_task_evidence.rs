//! Pure qualification of the selected unary task-cancellation protocol, not cancellation proof.

use std::num::NonZeroUsize;

use pale_core::records::A2aJsonRpcRequestIdV1;
use pale_core::records::{A2aRemoteContextIdV1, A2aRemoteTaskIdV1, A2aRemoteTaskStateV1};
use pale_ports::{A2aCancelTaskEvidencePort, A2aCancelTaskRequestEvidencePort, A2aExchangeError};

use crate::{
    task_evidence::{read_task_request, read_task_state_response},
    A2aMethod,
};

/// Stateless official-protocol reader for retained unary `CancelTask` evidence.
///
/// No service, connection, credential, route admission or clock is created.
/// Decoded values prove neither authorized exchange, local cancellation,
/// remote settlement, nor safe replay.
#[derive(Clone, Copy, Debug, Default)]
pub struct A2aCancelTaskDecoder;

impl A2aCancelTaskEvidencePort for A2aCancelTaskDecoder {
    fn read_task_state_after_cancel(
        &self,
        request: &[u8],
        response: &[u8],
        expected_task: &A2aRemoteTaskIdV1,
        expected_context: &A2aRemoteContextIdV1,
        maximum_bytes: NonZeroUsize,
    ) -> Result<A2aRemoteTaskStateV1, A2aExchangeError> {
        if request
            .len()
            .checked_add(response.len())
            .is_none_or(|length| length > maximum_bytes.get())
        {
            return Err(A2aExchangeError::LimitExceeded);
        }
        let (request_id, params) = read_task_request(request, A2aMethod::CancelTask)?;
        if params.len() != 1 || params.keys().any(|key| key != "id") {
            return Err(A2aExchangeError::Response);
        }
        let params: a2a::CancelTaskRequest =
            serde_json::from_value(params.into()).map_err(|_| A2aExchangeError::Response)?;
        if params.id != expected_task.as_str() {
            return Err(A2aExchangeError::CoordinateMismatch);
        }
        read_task_state_response(
            A2aMethod::CancelTask,
            &request_id,
            response,
            expected_task,
            expected_context,
            maximum_bytes,
        )
    }
}

impl A2aCancelTaskRequestEvidencePort for A2aCancelTaskDecoder {
    fn read_cancel_task_request(
        &self,
        request: &[u8],
        expected_task: &A2aRemoteTaskIdV1,
        maximum_bytes: NonZeroUsize,
    ) -> Result<A2aJsonRpcRequestIdV1, A2aExchangeError> {
        if request.len() > maximum_bytes.get() {
            return Err(A2aExchangeError::LimitExceeded);
        }
        let (request_id, params) = read_task_request(request, A2aMethod::CancelTask)?;
        if params.len() != 1 || params.keys().any(|key| key != "id") {
            return Err(A2aExchangeError::Response);
        }
        let params: a2a::CancelTaskRequest =
            serde_json::from_value(params.into()).map_err(|_| A2aExchangeError::Response)?;
        if params.id != expected_task.as_str() {
            return Err(A2aExchangeError::CoordinateMismatch);
        }
        Ok(request_id)
    }
}

#[cfg(test)]
#[path = "../tests/internal/cancel_task_evidence.rs"]
mod tests;
