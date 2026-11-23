//! Pure qualification of the selected unary task-read protocol, not transport authority.

use std::num::NonZeroUsize;

use pale_core::records::{A2aRemoteContextIdV1, A2aRemoteTaskIdV1, A2aRemoteTaskStateV1};
use pale_ports::{A2aExchangeError, A2aGetTaskEvidencePort};

use crate::{
    task_evidence::{read_task_request, read_task_state_response},
    A2aMethod,
};

/// Stateless official-protocol reader for retained unary `GetTask` evidence.
///
/// No service, connection, credential, route admission or clock is created.
/// The returned remote state proves neither authorized exchange nor Pale success.
#[derive(Clone, Copy, Debug, Default)]
pub struct A2aGetTaskDecoder;

impl A2aGetTaskEvidencePort for A2aGetTaskDecoder {
    fn read_task_state(
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
        let (request_id, params) = read_task_request(request, A2aMethod::GetTask)?;
        if params.len() != 2
            || params
                .keys()
                .any(|key| !matches!(key.as_str(), "id" | "historyLength"))
        {
            return Err(A2aExchangeError::Response);
        }
        let params: a2a::GetTaskRequest =
            serde_json::from_value(params.into()).map_err(|_| A2aExchangeError::Response)?;
        if params.history_length != Some(0) {
            return Err(A2aExchangeError::Response);
        }
        if params.id != expected_task.as_str() {
            return Err(A2aExchangeError::CoordinateMismatch);
        }
        read_task_state_response(
            A2aMethod::GetTask,
            &request_id,
            response,
            expected_task,
            expected_context,
            maximum_bytes,
        )
    }
}

#[cfg(test)]
#[path = "../tests/internal/get_task_evidence.rs"]
mod tests;
