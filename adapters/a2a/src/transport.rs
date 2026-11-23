//! One attempt: no retry, redirect, reconnect, or remote-state inference.

use std::num::NonZeroUsize;

use pale_ports::{
    A2aExchangeError, A2aExchangeEvidence, A2aExchangeFacts, A2aExchangeRequest,
    A2aPreparedExchange,
};
use reqwest::header::{ACCEPT, ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_TYPE};
use tokio::{sync::mpsc, time::timeout};

use crate::{
    decode_response, decoding::decode_error, wire, A2aEventDecoder, A2aEventLimits, A2aMethod,
};

pub(crate) async fn run(
    prepared: A2aPreparedExchange,
    client: reqwest::Client,
    sender: mpsc::Sender<A2aExchangeEvidence>,
    submitted: std::time::Instant,
) -> Result<(), A2aExchangeError> {
    let limits = prepared.request().limits();
    // Queue residence and every wait on a slow consumer spend the same budget.
    let remaining = limits
        .total_timeout
        .checked_sub(submitted.elapsed())
        .filter(|duration| !duration.is_zero())
        .ok_or(A2aExchangeError::Timeout)?;
    timeout(remaining, exchange(prepared, client, sender))
        .await
        .map_err(|_| A2aExchangeError::Timeout)?
}

async fn exchange(
    prepared: A2aPreparedExchange,
    client: reqwest::Client,
    sender: mpsc::Sender<A2aExchangeEvidence>,
) -> Result<(), A2aExchangeError> {
    let (request, _, bytes) = prepared.into_parts();
    let limits = request.limits();
    let method = wire::method(&request);
    let streaming = method == A2aMethod::SendStreamingMessage;
    let response = client
        .post(request.endpoint().as_str())
        .header(a2a::SVC_PARAM_VERSION, a2a::VERSION)
        .header(CONTENT_TYPE, "application/json")
        .header(
            ACCEPT,
            if streaming {
                "text/event-stream"
            } else {
                "application/json"
            },
        )
        .header(ACCEPT_ENCODING, "identity")
        .body(bytes)
        .send();
    let mut response = timeout(limits.connect_timeout, response)
        .await
        .map_err(|_| A2aExchangeError::Timeout)?
        .map_err(connection_error)?;
    if !response.status().is_success()
        || response.headers().contains_key(CONTENT_ENCODING)
        || !media_type(
            &response,
            if streaming {
                "text/event-stream"
            } else {
                "application/json"
            },
        )
        || response
            .content_length()
            .is_some_and(|length| length > limits.response_bytes.get() as u64)
    {
        return Err(A2aExchangeError::HttpResponse);
    }
    let mut evidence_bytes = 0usize;
    if streaming {
        let mut decoder = A2aEventDecoder::new(
            request.coordinate().request_id().as_str().into(),
            A2aEventLimits {
                line_bytes: limits.line_bytes,
                data_bytes: limits.event_bytes,
                stream_bytes: limits.response_bytes,
                events: limits.events,
            },
        )
        .map_err(decode_error)?;
        while let Some(chunk) = next_chunk(&mut response, &request).await? {
            let mut remaining = chunk.as_ref();
            while let Some(event) = decoder.next_event(&mut remaining).map_err(decode_error)? {
                let facts = wire::facts(event.response, &request)?;
                let evidence = A2aExchangeEvidence {
                    facts,
                    bytes: event.data,
                    sse: Some((event.event_id, event.event_type)),
                };
                charge(&evidence, &mut evidence_bytes, limits.evidence_bytes)?;
                sender
                    .send(evidence)
                    .await
                    .map_err(|_| A2aExchangeError::Cancelled)?;
            }
        }
        decoder.finish().map_err(decode_error)?;
    } else {
        let mut bytes = Vec::new();
        while let Some(chunk) = next_chunk(&mut response, &request).await? {
            let chunk = chunk.as_ref();
            if bytes
                .len()
                .checked_add(chunk.len())
                .is_none_or(|length| length > limits.response_bytes.get())
            {
                return Err(A2aExchangeError::LimitExceeded);
            }
            bytes.extend_from_slice(chunk);
        }
        let response = decode_response(
            method,
            &request.coordinate().request_id().as_str().into(),
            &bytes,
            limits.response_bytes,
        )
        .map_err(decode_error)?;
        let evidence = A2aExchangeEvidence {
            facts: wire::facts(response, &request)?,
            bytes,
            sse: None,
        };
        charge(&evidence, &mut evidence_bytes, limits.evidence_bytes)?;
        sender
            .send(evidence)
            .await
            .map_err(|_| A2aExchangeError::Cancelled)?;
    }
    Ok(())
}

fn media_type(response: &reqwest::Response, expected: &str) -> bool {
    let mut values = response.headers().get_all(CONTENT_TYPE).iter();
    let Some(value) = values.next() else {
        return false;
    };
    values.next().is_none()
        && value.to_str().ok().is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|essence| essence.trim().eq_ignore_ascii_case(expected))
        })
}

async fn next_chunk(
    response: &mut reqwest::Response,
    request: &A2aExchangeRequest,
) -> Result<Option<impl AsRef<[u8]>>, A2aExchangeError> {
    timeout(request.limits().read_timeout, response.chunk())
        .await
        .map_err(|_| A2aExchangeError::Timeout)?
        .map_err(connection_error)
}

fn connection_error(error: reqwest::Error) -> A2aExchangeError {
    // Neither Debug nor Display is used: URLs and peer details can be private.
    if error.is_timeout() {
        A2aExchangeError::Timeout
    } else {
        A2aExchangeError::Connection
    }
}

fn charge(
    evidence: &A2aExchangeEvidence,
    used: &mut usize,
    maximum: NonZeroUsize,
) -> Result<(), A2aExchangeError> {
    let mut length = evidence.bytes.len();
    let mut add = |bytes: usize| -> Result<(), A2aExchangeError> {
        length = length
            .checked_add(bytes)
            .ok_or(A2aExchangeError::LimitExceeded)?;
        Ok(())
    };
    if let Some((id, kind)) = &evidence.sse {
        add(id.len())?;
        add(kind.len())?;
    }
    match &evidence.facts {
        A2aExchangeFacts::Task { task, context, .. }
        | A2aExchangeFacts::Status { task, context, .. }
        | A2aExchangeFacts::Artifact { task, context } => {
            add(task.as_str().len())?;
            add(context.as_str().len())?;
        }
        A2aExchangeFacts::RemoteError { .. } => add(std::mem::size_of::<i64>())?,
        A2aExchangeFacts::DirectMessage => {}
    }
    *used = used
        .checked_add(length)
        .filter(|length| *length <= maximum.get())
        .ok_or(A2aExchangeError::LimitExceeded)?;
    Ok(())
}

#[cfg(test)]
#[path = "../tests/internal/transport.rs"]
mod tests;
