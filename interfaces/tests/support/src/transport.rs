//! Bounded in-memory framing between an outer interface and the resident.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::Arc;
use std::time::Instant;

use pale_application::resident::ResidentFrameService;
use pale_ports::{
    LocalIpcCarrierError, LocalIpcCarrierOperation, LocalIpcFramePort, LocalIpcPeerCredentials,
};

/// Failure of one bounded development-only frame exchange.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum BoundedFrameClientError {
    /// The request did not enter the bounded channel.
    #[error("resident request was not sent")]
    NotSent,
    /// The request entered the channel, but no response was established.
    #[error("resident request outcome is unknown")]
    OutcomeUnknown,
    /// No service owns the receiving side of the channel.
    #[error("resident service is unavailable")]
    ServiceUnavailable,
}

/// One request waiting for exactly one resident response.
pub struct PendingResidentExchange {
    request: Box<[u8]>,
    deadline: Instant,
    response: SyncSender<Result<Box<[u8]>, BoundedFrameClientError>>,
}

impl PendingResidentExchange {
    /// Borrow the exact outer request bytes.
    #[must_use]
    pub fn request(&self) -> &[u8] {
        &self.request
    }

    /// Return the unchanged absolute deadline.
    #[must_use]
    pub const fn deadline(&self) -> Instant {
        self.deadline
    }

    /// Complete the waiting exchange without interpreting resident bytes.
    pub fn complete(self, response: Result<Box<[u8]>, BoundedFrameClientError>) {
        let _ = self.response.send(response);
    }

    fn into_frame_port(self, peer: LocalIpcPeerCredentials) -> ExchangeFramePort {
        ExchangeFramePort {
            exchange: Some(self),
            peer,
            request_received: false,
            response: None,
            fail_response: false,
        }
    }
}

/// Bounded raw-frame client used by SDK, command, and MCP conformance tests.
#[derive(Clone)]
pub struct BoundedFrameClient {
    requests: SyncSender<PendingResidentExchange>,
    handoffs: Arc<AtomicUsize>,
}

impl BoundedFrameClient {
    /// Create a bounded client and the service-side receiver.
    ///
    /// # Panics
    ///
    /// Panics when `capacity` is zero because a zero-capacity synchronous
    /// channel would make the non-blocking send contract misleading.
    #[must_use]
    pub fn new(capacity: usize) -> (Self, Receiver<PendingResidentExchange>) {
        assert!(
            capacity > 0,
            "resident test channel capacity must be positive"
        );
        let (requests, receiver) = std::sync::mpsc::sync_channel(capacity);
        (
            Self {
                requests,
                handoffs: Arc::new(AtomicUsize::new(0)),
            },
            receiver,
        )
    }

    /// Share the count of requests successfully handed to the service side.
    #[must_use]
    pub fn handoff_counter(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.handoffs)
    }

    /// Exchange one exact frame before an absolute deadline.
    ///
    /// # Errors
    ///
    /// Returns `NotSent` before successful channel handoff,
    /// `ServiceUnavailable` when the receiver has closed, or `OutcomeUnknown`
    /// after handoff when no response is established before the deadline.
    pub fn exchange(
        &mut self,
        request: &[u8],
        deadline: Instant,
    ) -> Result<Box<[u8]>, BoundedFrameClientError> {
        if Instant::now() >= deadline {
            return Err(BoundedFrameClientError::NotSent);
        }
        let (response, result) = std::sync::mpsc::sync_channel(1);
        let exchange = PendingResidentExchange {
            request: request.to_vec().into_boxed_slice(),
            deadline,
            response,
        };
        match self.requests.try_send(exchange) {
            Ok(()) => {
                self.handoffs.fetch_add(1, Ordering::SeqCst);
            }
            Err(TrySendError::Full(_)) => return Err(BoundedFrameClientError::NotSent),
            Err(TrySendError::Disconnected(_)) => {
                return Err(BoundedFrameClientError::ServiceUnavailable);
            }
        }
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            return Err(BoundedFrameClientError::OutcomeUnknown);
        };
        match result.recv_timeout(remaining) {
            Ok(response) => response,
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {
                Err(BoundedFrameClientError::OutcomeUnknown)
            }
        }
    }
}

/// Serve one pending exchange through the public resident frame service.
///
/// A failed turn that produced no response remains outcome-unknown. This
/// helper does not synthesize a response or retry the request.
pub fn serve_exchange(
    service: &mut ResidentFrameService<'_>,
    exchange: PendingResidentExchange,
    peer: LocalIpcPeerCredentials,
) {
    serve_exchange_with_response(service, exchange, peer, |_| {});
}

/// Serve one exchange and inspect its exact response before client admission.
///
/// The observer receives `None` when the service established no response. It
/// cannot replace bytes, complete the exchange, or ask the service to retry.
/// Interface tests use this hook to register application-produced evidence
/// with an independent response-admission model before the waiting SDK reads
/// those bytes.
pub fn serve_exchange_with_response(
    service: &mut ResidentFrameService<'_>,
    exchange: PendingResidentExchange,
    peer: LocalIpcPeerCredentials,
    inspect: impl FnOnce(Option<&[u8]>),
) {
    let deadline = exchange.deadline();
    let mut frame = exchange.into_frame_port(peer);
    let _ = service.serve_frame(&mut frame, deadline, deadline);
    inspect(frame.response.as_deref());
    frame.complete();
}

/// Serve one request while the physical response write fails.
///
/// The application still receives the exact request and may commit its
/// bounded result. The waiting client receives `OutcomeUnknown`; no response
/// bytes are synthesized or retained by this transport model.
pub fn serve_exchange_with_failed_response(
    service: &mut ResidentFrameService<'_>,
    exchange: PendingResidentExchange,
    peer: LocalIpcPeerCredentials,
) {
    let deadline = exchange.deadline();
    let mut frame = exchange.into_frame_port(peer);
    frame.fail_response = true;
    let _ = service.serve_frame(&mut frame, deadline, deadline);
    frame.complete();
}

struct ExchangeFramePort {
    exchange: Option<PendingResidentExchange>,
    peer: LocalIpcPeerCredentials,
    request_received: bool,
    response: Option<Box<[u8]>>,
    fail_response: bool,
}

impl ExchangeFramePort {
    fn complete(mut self) {
        let response = self
            .response
            .take()
            .ok_or(BoundedFrameClientError::OutcomeUnknown);
        if let Some(exchange) = self.exchange.take() {
            exchange.complete(response);
        }
    }

    fn deadline_error(operation: LocalIpcCarrierOperation) -> LocalIpcCarrierError {
        LocalIpcCarrierError::DeadlineElapsed {
            operation,
            frame_bytes_transferred: 0,
        }
    }
}

impl LocalIpcFramePort for ExchangeFramePort {
    fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
        Ok(self.peer)
    }

    fn send_frame(
        &mut self,
        payload: &[u8],
        deadline: Instant,
    ) -> Result<(), LocalIpcCarrierError> {
        if Instant::now() >= deadline {
            return Err(Self::deadline_error(LocalIpcCarrierOperation::SendFrame));
        }
        if self.response.is_some() {
            return Err(LocalIpcCarrierError::Io {
                operation: LocalIpcCarrierOperation::SendFrame,
                frame_bytes_transferred: 0,
                source: std::io::Error::other("resident attempted two responses for one frame"),
            });
        }
        if self.fail_response {
            return Err(LocalIpcCarrierError::Io {
                operation: LocalIpcCarrierOperation::SendFrame,
                frame_bytes_transferred: 0,
                source: std::io::Error::other("injected resident response failure"),
            });
        }
        self.response = Some(payload.to_vec().into_boxed_slice());
        Ok(())
    }

    fn receive_frame(&mut self, deadline: Instant) -> Result<Box<[u8]>, LocalIpcCarrierError> {
        if Instant::now() >= deadline {
            return Err(Self::deadline_error(LocalIpcCarrierOperation::ReceiveFrame));
        }
        if self.request_received {
            return Err(LocalIpcCarrierError::EndOfStream);
        }
        let exchange = self
            .exchange
            .as_ref()
            .ok_or(LocalIpcCarrierError::EndOfStream)?;
        if Instant::now() >= exchange.deadline() {
            return Err(Self::deadline_error(LocalIpcCarrierOperation::ReceiveFrame));
        }
        self.request_received = true;
        Ok(exchange.request().to_vec().into_boxed_slice())
    }
}
