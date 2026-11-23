//! Translate only the selected A2A input and observation vocabulary.

use std::io::{self, Write};

use pale_core::records::{
    A2aRemoteContextIdV1, A2aRemoteTaskIdV1, A2aRemoteTaskStateV1, A2aTaskRequestModeV1,
    MAX_RESIDENT_ID_BYTES,
};
use pale_ports::{
    A2aExchangeError, A2aExchangeFacts, A2aExchangeInput, A2aExchangeRequest, A2aPreparedExchange,
};

use crate::{A2aMethod, A2aResponse};

pub(crate) fn method(request: &A2aExchangeRequest) -> A2aMethod {
    input_method(&request.input())
}

fn input_method(input: &A2aExchangeInput<'_>) -> A2aMethod {
    match input {
        A2aExchangeInput::Text {
            mode: A2aTaskRequestModeV1::SendMessage,
            ..
        } => A2aMethod::SendMessage,
        A2aExchangeInput::Text {
            mode: A2aTaskRequestModeV1::SendStreamingMessage,
            ..
        } => A2aMethod::SendStreamingMessage,
        A2aExchangeInput::GetTask { .. } => A2aMethod::GetTask,
        A2aExchangeInput::CancelTask { .. } => A2aMethod::CancelTask,
    }
}

pub(crate) fn prepare(
    request: A2aExchangeRequest,
    tls_trust_input_sha256: pale_core::records::ArtifactDigest,
) -> Result<A2aPreparedExchange, A2aExchangeError> {
    let mut writer = LimitedWriter {
        bytes: Vec::new(),
        maximum: request.limits().request_bytes.get(),
    };
    encode(&request, &mut writer).map_err(|_| A2aExchangeError::LimitExceeded)?;
    A2aPreparedExchange::from_encoded(request, writer.bytes, tls_trust_input_sha256)
}

/// Reject substituted bytes without replacing the already retained body.
pub(crate) fn validate_prepared(prepared: &A2aPreparedExchange) -> Result<(), A2aExchangeError> {
    let mut writer = MatchingWriter {
        remaining: prepared.bytes(),
    };
    encode(prepared.request(), &mut writer).map_err(|_| A2aExchangeError::CoordinateMismatch)?;
    if !writer.remaining.is_empty() {
        return Err(A2aExchangeError::CoordinateMismatch);
    }
    Ok(())
}

fn encode(request: &A2aExchangeRequest, writer: &mut impl Write) -> Result<(), serde_json::Error> {
    encode_input(
        request.coordinate().request_id().as_str(),
        request.input(),
        writer,
    )
}

fn encode_input(
    request_id: &str,
    input: A2aExchangeInput<'_>,
    writer: &mut impl Write,
) -> Result<(), serde_json::Error> {
    let method = input_method(&input);
    let params = match input {
        A2aExchangeInput::Text {
            message_id, text, ..
        } => serde_json::to_value(a2a::SendMessageRequest {
            message: a2a::Message {
                message_id: message_id.to_owned(),
                role: a2a::Role::User,
                parts: vec![a2a::Part::text(text)],
                context_id: None,
                task_id: None,
                metadata: None,
                extensions: None,
                reference_task_ids: None,
            },
            configuration: Some(a2a::SendMessageConfiguration {
                accepted_output_modes: Some(vec!["text/plain".to_owned()]),
                task_push_notification_config: None,
                history_length: Some(0),
                return_immediately: Some(true),
            }),
            metadata: None,
            tenant: None,
        }),
        A2aExchangeInput::GetTask { task, .. } => serde_json::to_value(a2a::GetTaskRequest {
            id: task.as_str().to_owned(),
            history_length: Some(0),
            tenant: None,
        }),
        A2aExchangeInput::CancelTask { task, .. } => serde_json::to_value(a2a::CancelTaskRequest {
            id: task.as_str().to_owned(),
            metadata: None,
            tenant: None,
        }),
    }?;
    let rpc = a2a::JsonRpcRequest::new(request_id.into(), method.as_str(), Some(params));
    serde_json::to_writer_pretty(writer, &rpc)
}

struct MatchingWriter<'a> {
    remaining: &'a [u8],
}

impl Write for MatchingWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.remaining = self
            .remaining
            .strip_prefix(bytes)
            .ok_or_else(|| io::Error::other("request byte mismatch"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Bounds encoded output length, including escaping and indentation.
/// Upstream DTO and JSON-value allocations are separate retained input costs.
struct LimitedWriter {
    bytes: Vec<u8>,
    maximum: usize,
}

impl Write for LimitedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self
            .bytes
            .len()
            .checked_add(bytes.len())
            .is_none_or(|len| len > self.maximum)
        {
            return Err(io::Error::other("request byte limit"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) fn facts(
    response: A2aResponse,
    request: &A2aExchangeRequest,
) -> Result<A2aExchangeFacts, A2aExchangeError> {
    let facts = unbound_facts(response)?;
    if let A2aExchangeInput::GetTask {
        task: expected_task,
        context: expected_context,
    }
    | A2aExchangeInput::CancelTask {
        task: expected_task,
        context: expected_context,
    } = request.input()
    {
        match &facts {
            A2aExchangeFacts::Task { task, context, .. }
                if task == expected_task && context == expected_context => {}
            A2aExchangeFacts::RemoteError { .. } => {}
            _ => return Err(A2aExchangeError::CoordinateMismatch),
        }
    }
    Ok(facts)
}

pub(crate) fn unbound_facts(response: A2aResponse) -> Result<A2aExchangeFacts, A2aExchangeError> {
    let facts = match response {
        A2aResponse::Send(a2a::SendMessageResponse::Task(task))
        | A2aResponse::Task(task)
        | A2aResponse::Stream(a2a::StreamResponse::Task(task)) => {
            let (task_id, context, state) = task_facts(task)?;
            A2aExchangeFacts::Task {
                task: task_id,
                context,
                state,
            }
        }
        A2aResponse::Send(a2a::SendMessageResponse::Message(_))
        | A2aResponse::Stream(a2a::StreamResponse::Message(_)) => A2aExchangeFacts::DirectMessage,
        A2aResponse::Stream(a2a::StreamResponse::StatusUpdate(update)) => {
            let (task, context) = identity(&update.task_id, &update.context_id)?;
            A2aExchangeFacts::Status {
                task,
                context,
                state: state(update.status.state),
            }
        }
        A2aResponse::Stream(a2a::StreamResponse::ArtifactUpdate(update)) => {
            let (task, context) = identity(&update.task_id, &update.context_id)?;
            A2aExchangeFacts::Artifact { task, context }
        }
        A2aResponse::RemoteError(error) => A2aExchangeFacts::RemoteError {
            code: i64::from(error.code),
        },
    };
    Ok(facts)
}

pub(crate) fn task_facts(
    task: a2a::Task,
) -> Result<
    (
        A2aRemoteTaskIdV1,
        A2aRemoteContextIdV1,
        A2aRemoteTaskStateV1,
    ),
    A2aExchangeError,
> {
    let (task_id, context) = identity(&task.id, &task.context_id)?;
    Ok((task_id, context, state(task.status.state)))
}

fn identity(
    task: &str,
    context: &str,
) -> Result<(A2aRemoteTaskIdV1, A2aRemoteContextIdV1), A2aExchangeError> {
    // Official DTO parsing is byte-bounded. These constructors enforce the
    // existing core identity grammar before the facts cross the port.
    if task.len() > MAX_RESIDENT_ID_BYTES || context.len() > MAX_RESIDENT_ID_BYTES {
        return Err(A2aExchangeError::LimitExceeded);
    }
    Ok((
        A2aRemoteTaskIdV1::parse(task.to_owned()).map_err(|_| A2aExchangeError::Response)?,
        A2aRemoteContextIdV1::parse(context.to_owned()).map_err(|_| A2aExchangeError::Response)?,
    ))
}

fn state(state: a2a::TaskState) -> A2aRemoteTaskStateV1 {
    match state {
        a2a::TaskState::Unspecified => A2aRemoteTaskStateV1::Unspecified,
        a2a::TaskState::Submitted => A2aRemoteTaskStateV1::Submitted,
        a2a::TaskState::Working => A2aRemoteTaskStateV1::Working,
        a2a::TaskState::InputRequired => A2aRemoteTaskStateV1::InputRequired,
        a2a::TaskState::AuthRequired => A2aRemoteTaskStateV1::AuthRequired,
        a2a::TaskState::Completed => A2aRemoteTaskStateV1::Completed,
        a2a::TaskState::Canceled => A2aRemoteTaskStateV1::Canceled,
        a2a::TaskState::Failed => A2aRemoteTaskStateV1::Failed,
        a2a::TaskState::Rejected => A2aRemoteTaskStateV1::Rejected,
    }
}

#[cfg(test)]
#[path = "../tests/internal/wire.rs"]
mod tests;
