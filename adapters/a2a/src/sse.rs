//! Decode Server-Sent Events (SSE) incrementally with bounded storage.

use std::num::NonZeroUsize;

use crate::{decode_response, A2aDecodeError, A2aMethod, A2aResponse};

/// Independent receive ceilings selected by the calling transport.
///
/// Total wire bytes include comments and ignored fields, so an endless stream
/// of non-events still exhausts its budget. These are resource ceilings only;
/// they convey no permission to connect, dispatch, or resume a stream.
#[derive(Debug, Clone, Copy)]
pub(crate) struct A2aEventLimits {
    /// Maximum UTF-8 bytes in a line, excluding its CR/LF delimiter.
    pub(crate) line_bytes: NonZeroUsize,
    /// Maximum assembled JSON data bytes, including inserted inter-line LF bytes.
    pub(crate) data_bytes: NonZeroUsize,
    /// Maximum raw bytes consumed over the entire stream, including a leading BOM.
    pub(crate) stream_bytes: NonZeroUsize,
    /// Maximum dispatched data events, including remote error responses.
    pub(crate) events: NonZeroUsize,
}

/// One correlated wire event with its owner-provided SSE coordinates.
///
/// Event IDs may repeat, be inherited or be explicitly reset to empty. No ID
/// is synthesized, deduplicated or promoted into Pale identity. Remote Task
/// state likewise remains upstream observation rather than operation success.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct A2aStreamEvent {
    /// Exact assembled JSON data bytes; not a raw transport transcript.
    pub(crate) data: Vec<u8>,
    /// Last valid `id` field, inherited across events; initially empty.
    pub(crate) event_id: String,
    /// Owner event type, or `message` when absent or empty.
    pub(crate) event_type: String,
    /// Official stream result or correlated remote error from this event's JSON.
    pub(crate) response: A2aResponse,
}

/// A pull decoder for one already-selected `SendStreamingMessage` response.
///
/// [`Self::next_event`] consumes only through the next complete event, leaving
/// the caller's remaining slice untouched. The caller controls backpressure
/// and observes valid events before a later failure regardless of chunking.
/// The decoder holds one line, one data buffer, and the current SSE ID/type;
/// it never collects a stream or opens/reconnects a transport.
///
/// CR, LF and CRLF delimit lines, including across chunks. One leading UTF-8
/// BOM is stripped. Invalid UTF-8 is rejected instead of replacing identity
/// bytes. Comments, unknown fields and `retry` are ignored; `retry` cannot
/// enable reconnection. Any error closes the decoder. [`Self::finish`] reports
/// incomplete trailing data without dispatching it or inferring task completion.
#[derive(Debug)]
pub(crate) struct A2aEventDecoder {
    expected_id: a2a::JsonRpcId,
    limits: A2aEventLimits,
    line: Vec<u8>,
    data: Vec<u8>,
    has_data: bool,
    event_id: String,
    event_type: String,
    first_line: bool,
    after_cr: bool,
    bytes_read: usize,
    events_read: usize,
    state: DecodeState,
}

/// A rejected decoder cannot be resumed with another buffer.
#[derive(Debug)]
enum DecodeState {
    Open,
    Rejected,
}

impl A2aEventDecoder {
    /// Bind one decoder to a non-null caller-owned request identity and bounds.
    ///
    /// # Errors
    ///
    /// Returns [`A2aDecodeError::NullRequestIdentity`] for an uncorrelatable ID.
    pub(crate) fn new(
        expected_id: a2a::JsonRpcId,
        limits: A2aEventLimits,
    ) -> Result<Self, A2aDecodeError> {
        if matches!(expected_id, a2a::JsonRpcId::Null) {
            return Err(A2aDecodeError::NullRequestIdentity);
        }
        Ok(Self {
            expected_id,
            limits,
            line: Vec::new(),
            data: Vec::new(),
            has_data: false,
            event_id: String::new(),
            event_type: String::new(),
            first_line: true,
            after_cr: false,
            bytes_read: 0,
            events_read: 0,
            state: DecodeState::Open,
        })
    }

    /// Consume bytes through the next data event, or all input if none is complete.
    ///
    /// On `Some`, call again with the remaining slice before fetching more bytes.
    /// On `None`, retain this decoder and supply the next chunk. An empty chunk
    /// is not EOF; use [`Self::finish`] when the transport ends. The JSON-RPC
    /// identity is rechecked for every event, including remote errors.
    ///
    /// # Errors
    ///
    /// Returns [`A2aDecodeError`] on exceeded bounds, malformed UTF-8 or JSON,
    /// unlike response identity, or use after rejection. The offending byte
    /// has been consumed, and subsequent calls return
    /// [`A2aDecodeError::StreamRejected`].
    ///
    /// Drain each chunk before reading the next; the decoder retains only an
    /// unfinished event, even when a chunk contains several complete events.
    pub(crate) fn next_event(
        &mut self,
        input: &mut &[u8],
    ) -> Result<Option<A2aStreamEvent>, A2aDecodeError> {
        if matches!(self.state, DecodeState::Rejected) {
            return Err(A2aDecodeError::StreamRejected);
        }
        let result = self.consume(input);
        if result.is_err() {
            self.state = DecodeState::Rejected;
        }
        result
    }

    /// Finish at EOF, rejecting trailing data that lacked an event delimiter.
    ///
    /// A successful finish means only that framing ended cleanly. It does not
    /// establish remote terminal state or authorize reconnect/replay.
    ///
    /// # Errors
    ///
    /// Returns [`A2aDecodeError::TruncatedEvent`] for an unfinished line or
    /// undispatched data, or [`A2aDecodeError::StreamRejected`] after an error.
    pub(crate) fn finish(self) -> Result<(), A2aDecodeError> {
        if matches!(self.state, DecodeState::Rejected) {
            return Err(A2aDecodeError::StreamRejected);
        }
        if !self.line.is_empty() || self.has_data {
            return Err(A2aDecodeError::TruncatedEvent);
        }
        Ok(())
    }

    /// Advance one byte at a time so allocation and work cannot outrun the ceiling.
    fn consume(&mut self, input: &mut &[u8]) -> Result<Option<A2aStreamEvent>, A2aDecodeError> {
        while let Some((&byte, rest)) = input.split_first() {
            *input = rest;
            if self.bytes_read == self.limits.stream_bytes.get() {
                return Err(A2aDecodeError::LimitExceeded);
            }
            self.bytes_read += 1;
            if self.after_cr {
                self.after_cr = false;
                if byte == b'\n' {
                    continue;
                }
            }
            if byte == b'\r' || byte == b'\n' {
                self.after_cr = byte == b'\r';
                if let Some(event) = self.complete_line()? {
                    return Ok(Some(event));
                }
            } else {
                if self.line.len() == self.limits.line_bytes.get() {
                    return Err(A2aDecodeError::LimitExceeded);
                }
                self.line.push(byte);
            }
        }
        Ok(None)
    }

    /// Interpret a complete UTF-8 line without another semantic payload parser.
    fn complete_line(&mut self) -> Result<Option<A2aStreamEvent>, A2aDecodeError> {
        let line = std::str::from_utf8(&self.line).map_err(|_| A2aDecodeError::InvalidUtf8)?;
        let line = if self.first_line {
            self.first_line = false;
            line.strip_prefix('\u{feff}').unwrap_or(line)
        } else {
            line
        };
        if line.is_empty() {
            self.line.clear();
            return self.dispatch();
        }
        if !line.starts_with(':') {
            let (field, value) = line.split_once(':').unwrap_or((line, ""));
            let value = value.strip_prefix(' ').unwrap_or(value);
            match field {
                "data" => {
                    let separator = usize::from(self.has_data);
                    let length = self
                        .data
                        .len()
                        .checked_add(separator)
                        .and_then(|length| length.checked_add(value.len()))
                        .ok_or(A2aDecodeError::LimitExceeded)?;
                    if length > self.limits.data_bytes.get() {
                        return Err(A2aDecodeError::LimitExceeded);
                    }
                    if self.has_data {
                        self.data.push(b'\n');
                    }
                    self.data.extend_from_slice(value.as_bytes());
                    self.has_data = true;
                }
                "id" if !value.contains('\0') => {
                    self.event_id.clear();
                    self.event_id.push_str(value);
                }
                "event" => {
                    self.event_type.clear();
                    self.event_type.push_str(value);
                }
                _ => {}
            }
        }
        self.line.clear();
        Ok(None)
    }

    /// Emit one official stream DTO, preserving SSE ID inheritance and type reset.
    fn dispatch(&mut self) -> Result<Option<A2aStreamEvent>, A2aDecodeError> {
        if !self.has_data {
            self.event_type.clear();
            return Ok(None);
        }
        if self.events_read == self.limits.events.get() {
            return Err(A2aDecodeError::LimitExceeded);
        }
        let response = decode_response(
            A2aMethod::SendStreamingMessage,
            &self.expected_id,
            &self.data,
            self.limits.data_bytes,
        )?;
        self.events_read += 1;
        let data = std::mem::take(&mut self.data);
        self.has_data = false;
        let event_type = if self.event_type.is_empty() {
            "message".into()
        } else {
            std::mem::take(&mut self.event_type)
        };
        Ok(Some(A2aStreamEvent {
            data,
            event_id: self.event_id.clone(),
            event_type,
            response,
        }))
    }
}
