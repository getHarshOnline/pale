//! Bounded newline-delimited ACP transport input.

use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::AsyncRead;

use crate::{
    ACP_INCOMING_FRAME_BYTE_LIMIT, ACP_TURN_INBOUND_BYTE_LIMIT, ACP_TURN_INBOUND_EVENT_LIMIT,
};

/// First inbound resource ceiling exceeded by one [`BoundedIncoming`] turn.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InboundLimitExceeded {
    /// One newline-delimited frame exceeded [`ACP_INCOMING_FRAME_BYTE_LIMIT`].
    FrameBytes,
    /// Aggregate bytes exceeded [`ACP_TURN_INBOUND_BYTE_LIMIT`].
    TurnBytes,
    /// Aggregate logical events exceeded [`ACP_TURN_INBOUND_EVENT_LIMIT`].
    TurnEvents,
}

/// Shared first-failure latch for one bounded connection.
///
/// The byte reader records the first exceeded ceiling before returning its
/// transport error. The conductor can therefore preserve the exact resource
/// classification even though the ACP library exposes only the I/O failure.
#[derive(Clone, Debug, Default)]
pub(crate) struct InboundLimitStatus(Arc<AtomicU8>);

impl InboundLimitStatus {
    /// Return the first exceeded ceiling, when the reader recorded one.
    pub(crate) fn exceeded(&self) -> Option<InboundLimitExceeded> {
        match self.0.load(Ordering::Acquire) {
            1 => Some(InboundLimitExceeded::FrameBytes),
            2 => Some(InboundLimitExceeded::TurnBytes),
            3 => Some(InboundLimitExceeded::TurnEvents),
            _ => None,
        }
    }

    /// Record `limit` only when no earlier ceiling owns the failure.
    fn record(&self, limit: InboundLimitExceeded) {
        let code = match limit {
            InboundLimitExceeded::FrameBytes => 1,
            InboundLimitExceeded::TurnBytes => 2,
            InboundLimitExceeded::TurnEvents => 3,
        };
        let _unchanged = self
            .0
            .compare_exchange(0, code, Ordering::AcqRel, Ordering::Acquire);
    }
}

/// Read adapter enforcing per-frame and aggregate turn ceilings.
///
/// Bytes and logical events accumulate for the lifetime of this adapter. A
/// JSON-RPC batch contributes its array cardinality; every other completed
/// frame, including an empty batch, contributes one logical event.
pub(crate) struct BoundedIncoming<Incoming> {
    incoming: Pin<Box<Incoming>>,
    current_frame: Vec<u8>,
    turn_bytes: usize,
    turn_events: u32,
    status: InboundLimitStatus,
}

impl<Incoming> BoundedIncoming<Incoming> {
    /// Wrap `incoming` and bind it to the conductor-visible failure latch.
    pub(crate) fn new(incoming: Incoming, status: InboundLimitStatus) -> Self {
        Self {
            incoming: Box::pin(incoming),
            current_frame: Vec::new(),
            turn_bytes: 0,
            turn_events: 0,
            status,
        }
    }
}

impl<Incoming> AsyncRead for BoundedIncoming<Incoming>
where
    Incoming: AsyncRead,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let read = match self.incoming.as_mut().poll_read(context, buffer) {
            Poll::Ready(Ok(read)) => read,
            Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
            Poll::Pending => return Poll::Pending,
        };
        for byte in &buffer[..read] {
            let Some(next_turn_bytes) = self.turn_bytes.checked_add(1) else {
                return Poll::Ready(Err(self.limit_error(InboundLimitExceeded::TurnBytes)));
            };
            if next_turn_bytes > ACP_TURN_INBOUND_BYTE_LIMIT {
                return Poll::Ready(Err(self.limit_error(InboundLimitExceeded::TurnBytes)));
            }
            self.turn_bytes = next_turn_bytes;
            if *byte == b'\n' {
                let event_count = logical_event_count(&self.current_frame);
                let Some(next_event) = self.turn_events.checked_add(event_count) else {
                    return Poll::Ready(Err(self.limit_error(InboundLimitExceeded::TurnEvents)));
                };
                if next_event > ACP_TURN_INBOUND_EVENT_LIMIT {
                    return Poll::Ready(Err(self.limit_error(InboundLimitExceeded::TurnEvents)));
                }
                self.turn_events = next_event;
                self.current_frame.clear();
                continue;
            }
            if self.current_frame.len() == ACP_INCOMING_FRAME_BYTE_LIMIT {
                return Poll::Ready(Err(self.limit_error(InboundLimitExceeded::FrameBytes)));
            }
            self.current_frame.push(*byte);
        }
        Poll::Ready(Ok(read))
    }
}

/// Count one completed JSON-RPC frame without retaining its decoded body.
fn logical_event_count(frame: &[u8]) -> u32 {
    let Ok(serde_json::Value::Array(events)) = serde_json::from_slice(frame) else {
        return 1;
    };
    u32::try_from(events.len()).unwrap_or(u32::MAX).max(1)
}

impl<Incoming> BoundedIncoming<Incoming> {
    /// Latch `limit` and return the transport error consumed by the ACP peer.
    fn limit_error(&self, limit: InboundLimitExceeded) -> io::Error {
        self.status.record(limit);
        let message = match limit {
            InboundLimitExceeded::FrameBytes => {
                "ACP newline-delimited JSON-RPC frame exceeds the admitted byte ceiling"
            }
            InboundLimitExceeded::TurnBytes => {
                "ACP turn exceeds the aggregate inbound byte ceiling"
            }
            InboundLimitExceeded::TurnEvents => {
                "ACP turn exceeds the aggregate inbound event ceiling"
            }
        };
        io::Error::new(io::ErrorKind::InvalidData, message)
    }
}
