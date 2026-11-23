//! Exact request/response correlation over bounded, untrusted JSON buffers.

use std::{fmt, num::NonZeroUsize};

use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use serde_json::{Map, Value};

use crate::A2aMethod;

/// A correlated response in the selected upstream method's own vocabulary.
///
/// These values prove wire decoding only. In particular, a [`Self::Send`]
/// message is retained as a message; it never fabricates a remote Task, and a
/// remote error does not classify Pale effect certainty or retry permission.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum A2aResponse {
    /// `SendMessage` returned the upstream Task-or-Message union.
    Send(a2a::SendMessageResponse),
    /// `GetTask` or `CancelTask` returned a Task read-back.
    Task(a2a::Task),
    /// One `SendStreamingMessage` response in its four-variant upstream union.
    Stream(a2a::StreamResponse),
    /// A correlated JSON-RPC error, preserved without semantic reinterpretation.
    RemoteError(a2a::JsonRpcError),
}

/// Redacted rejection at the buffer boundary.
///
/// No variant retains peer input. A rejection proves neither remote inactivity
/// nor safe replay; the operation owner must reconcile any earlier dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum A2aDecodeError {
    /// The caller supplied a null request identity, which cannot correlate this journey.
    NullRequestIdentity,
    /// A response, line, event, stream, or event count exceeded its explicit ceiling.
    LimitExceeded,
    /// JSON syntax, nesting, UTF-8, or object-member uniqueness was invalid.
    InvalidJson,
    /// The response was not one JSON-RPC 2.0 object with the selected fields.
    InvalidEnvelope,
    /// The peer identity differs, including string-versus-number differences.
    RequestIdentityMismatch,
    /// Result/error presence did not form an exclusive union, or a branch was null.
    InvalidResponseUnion,
    /// The result or error failed its selected official A2A wire shape.
    InvalidPayload,
    /// A completed SSE line was not UTF-8.
    InvalidUtf8,
    /// End-of-input left an unterminated SSE line or undispatched data.
    TruncatedEvent,
    /// A previous decoding error closed this stream decoder.
    StreamRejected,
}

impl fmt::Display for A2aDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NullRequestIdentity => "null A2A request identity",
            Self::LimitExceeded => "A2A response limit exceeded",
            Self::InvalidJson => "invalid or ambiguous A2A JSON",
            Self::InvalidEnvelope => "invalid A2A JSON-RPC envelope",
            Self::RequestIdentityMismatch => "A2A response identity mismatch",
            Self::InvalidResponseUnion => "invalid A2A result/error union",
            Self::InvalidPayload => "invalid A2A response payload",
            Self::InvalidUtf8 => "invalid A2A event-stream UTF-8",
            Self::TruncatedEvent => "incomplete A2A event at end of input",
            Self::StreamRejected => "A2A event stream was already rejected",
        })
    }
}

impl std::error::Error for A2aDecodeError {}

/// Decode one bounded JSON-RPC response against the exact request identity.
///
/// The byte ceiling is checked before allocation. Duplicate members are
/// rejected throughout the JSON tree before the official DTO deserializer can
/// erase them. `result` and `error` are checked by presence, so an explicit
/// `null` cannot masquerade as an absent branch. Task/Message and stream unions
/// must contain exactly one variant. HTTP status, TLS, Agent Card admission,
/// dispatch custody and Task admission belong to their respective callers.
///
/// # Errors
///
/// Returns [`A2aDecodeError`] for a null expected identity, exceeded bound,
/// ambiguous JSON, invalid envelope, unlike identity or invalid payload.
pub(crate) fn decode_response(
    method: A2aMethod,
    expected_id: &a2a::JsonRpcId,
    bytes: &[u8],
    max_bytes: NonZeroUsize,
) -> Result<A2aResponse, A2aDecodeError> {
    if matches!(expected_id, a2a::JsonRpcId::Null) {
        return Err(A2aDecodeError::NullRequestIdentity);
    }
    if bytes.len() > max_bytes.get() {
        return Err(A2aDecodeError::LimitExceeded);
    }
    let UniqueJson(value) =
        serde_json::from_slice(bytes).map_err(|_| A2aDecodeError::InvalidJson)?;
    let Value::Object(mut envelope) = value else {
        return Err(A2aDecodeError::InvalidEnvelope);
    };
    if envelope.remove("jsonrpc") != Some(Value::String("2.0".into()))
        || envelope
            .keys()
            .any(|key| !matches!(key.as_str(), "id" | "result" | "error"))
    {
        return Err(A2aDecodeError::InvalidEnvelope);
    }
    let id = envelope
        .remove("id")
        .ok_or(A2aDecodeError::InvalidEnvelope)?;
    let id: a2a::JsonRpcId =
        serde_json::from_value(id).map_err(|_| A2aDecodeError::InvalidEnvelope)?;
    if &id != expected_id {
        return Err(A2aDecodeError::RequestIdentityMismatch);
    }
    match (envelope.remove("result"), envelope.remove("error")) {
        (Some(result), None) if !result.is_null() => decode_result(method, result),
        (None, Some(error)) if !error.is_null() => serde_json::from_value(error)
            .map(A2aResponse::RemoteError)
            .map_err(|_| A2aDecodeError::InvalidPayload),
        _ => Err(A2aDecodeError::InvalidResponseUnion),
    }
}

/// Check union exclusivity before the upstream field-presence decoder chooses a branch.
fn decode_result(method: A2aMethod, result: Value) -> Result<A2aResponse, A2aDecodeError> {
    check_selected_payload(&result)?;
    let members: &[&str] = match method {
        A2aMethod::SendMessage => &["task", "message"],
        A2aMethod::SendStreamingMessage => &["task", "message", "statusUpdate", "artifactUpdate"],
        A2aMethod::GetTask | A2aMethod::CancelTask => &[],
    };
    if !members.is_empty() {
        let object = result.as_object().ok_or(A2aDecodeError::InvalidPayload)?;
        if object.len() != 1
            || !members
                .iter()
                .any(|member| object.get(*member).is_some_and(|value| !value.is_null()))
        {
            return Err(A2aDecodeError::InvalidPayload);
        }
    }
    match method {
        A2aMethod::SendMessage => serde_json::from_value(result).map(A2aResponse::Send),
        A2aMethod::SendStreamingMessage => serde_json::from_value(result).map(A2aResponse::Stream),
        A2aMethod::GetTask | A2aMethod::CancelTask => {
            serde_json::from_value(result).map(A2aResponse::Task)
        }
    }
    .map_err(|_| A2aDecodeError::InvalidPayload)
}

/// Reject unsupported part/extension fields before upstream decoding can
/// discard them. No semantic meaning is inferred from their absence.
fn check_selected_payload(value: &Value) -> Result<(), A2aDecodeError> {
    match value {
        Value::Object(object) => {
            if object.keys().any(|key| {
                matches!(
                    key.as_str(),
                    "raw" | "url" | "data" | "metadata" | "extensions"
                )
            }) {
                return Err(A2aDecodeError::InvalidPayload);
            }
            for child in object.values() {
                check_selected_payload(child)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                check_selected_payload(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// JSON value whose recursive map visitor rejects duplicate decoded member names.
pub(crate) struct UniqueJson(pub(crate) Value);

pub(crate) fn decode_error(error: A2aDecodeError) -> pale_ports::A2aExchangeError {
    use pale_ports::A2aExchangeError;
    match error {
        A2aDecodeError::LimitExceeded => A2aExchangeError::LimitExceeded,
        A2aDecodeError::RequestIdentityMismatch => A2aExchangeError::CoordinateMismatch,
        _ => A2aExchangeError::Response,
    }
}

impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(UniqueJsonVisitor)
    }
}

/// Bounded by the input byte ceiling and `serde_json`'s recursion limit.
struct UniqueJsonVisitor;

impl<'de> Visitor<'de> for UniqueJsonVisitor {
    type Value = UniqueJson;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON with unique object members")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::Null))
    }
    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(UniqueJson(value.into()))
    }
    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(UniqueJson(value.into()))
    }
    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(UniqueJson(value.into()))
    }
    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        serde_json::Number::from_f64(value)
            .map(|number| UniqueJson(Value::Number(number)))
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }
    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(UniqueJson(value.into()))
    }
    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(UniqueJson(value.into()))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(UniqueJson(value)) = sequence.next_element()? {
            values.push(value);
        }
        Ok(UniqueJson(Value::Array(values)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom("duplicate JSON member"));
            }
            let UniqueJson(value) = map.next_value()?;
            values.insert(key, value);
        }
        Ok(UniqueJson(Value::Object(values)))
    }
}
