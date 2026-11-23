//! Caller-driven Unix listener and retained connection ownership.

use std::{collections::VecDeque, num::NonZeroUsize, path::Path, time::Instant};

use pale_adapter_uds::{
    begin_authenticated_request, AuthenticatedLocalRequest, EndpointConfigurationError,
    LocalAuthenticationError, LocalEndpointConfiguration, PendingLocalAuthentication,
    UnixSocketCarrier, UnixSocketListener,
};
use pale_application::resident::{
    ResidentFrameService, ResidentReadyFrame, ResidentWorkTurnCoordinator, ResidentWorkTurnError,
    ResidentWorkTurnInput, ResidentWorkTurnOutcome,
};
use pale_ports::{LocalIpcCarrierError, LocalIpcFrameLimits};

mod authentication;
use authentication::AuthenticatedFrame;
pub use authentication::ResidentConnectionAuthentication;

/// Absolute deadlines retained for one accepted connection.
///
/// A later host turn cannot renew these deadlines. Read readiness may expose
/// only part of a request; the frame deadline also bounds that subsequent read.
#[derive(Clone, Copy, Debug)]
pub struct ConnectionDeadlines {
    frame: Instant,
    response: Instant,
}

impl ConnectionDeadlines {
    /// Preserve the caller's existing frame and operation-response ceilings.
    #[must_use]
    pub const fn new(frame: Instant, response: Instant) -> Self {
        Self { frame, response }
    }

    fn expired(self) -> bool {
        let now = Instant::now();
        now >= self.frame || now >= self.response
    }
}

struct PendingConnection {
    carrier: UnixSocketCarrier,
    deadlines: ConnectionDeadlines,
    authentication: ConnectionAuthentication,
}

enum ConnectionAuthentication {
    Hello,
    Challenged(PendingLocalAuthentication),
    Verified(Option<AuthenticatedLocalRequest>),
}

/// Outcome of one bounded native host turn.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidentHostTurn {
    /// The application ran one frame, retained phase or recovery turn.
    Work(ResidentWorkTurnOutcome),
    /// A connection is awaiting its hello, authenticated request or next turn.
    AwaitingFrame,
    /// One pending connection reached its original deadline and was closed.
    /// This neither cancels an operation nor discards application recovery.
    ConnectionExpired,
    /// No ready connection or application work was offered.
    Idle,
}

/// Failure of one native host turn without exposing endpoint or payload text.
#[derive(Debug, thiserror::Error)]
pub enum ResidentHostError {
    /// Explicit local endpoint configuration or its retained path was refused.
    #[error(transparent)]
    Configuration(#[from] EndpointConfigurationError),
    /// Native accept or readiness failed.
    #[error(transparent)]
    Carrier(#[from] LocalIpcCarrierError),
    /// The selected caller or the exact connection bytes could not be proved.
    #[error(transparent)]
    Authentication(#[from] LocalAuthenticationError),
    /// The existing application frame boundary could not finish the exchange.
    #[error(transparent)]
    Application(#[from] ResidentWorkTurnError),
    /// Fresh connection deadlines already elapsed; no accept was attempted.
    #[error("new resident connection deadlines have elapsed")]
    ExpiredOffer,
}

/// One native listener borrowing the process-lifetime application service.
///
/// The caller bounds retained accepted connections. Each connection carries one
/// exchange and is closed after its frame turn, including an uncertain failure.
/// The application service remains borrowed across turns and keeps its own
/// interrupted phases. Dropping this host closes transport only; it neither
/// cancels resident work nor unlinks a pathname that might have been replaced.
pub struct ResidentLocalHost<'owner, 'service> {
    listener: UnixSocketListener,
    service: &'owner mut ResidentFrameService<'service>,
    coordinator: ResidentWorkTurnCoordinator,
    pending: VecDeque<PendingConnection>,
    maximum_connections: usize,
    authentication: ResidentConnectionAuthentication<'owner>,
}

impl<'owner, 'service> ResidentLocalHost<'owner, 'service> {
    /// Bind the endpoint captured from the selected private machine file.
    ///
    /// Directory identity is rechecked immediately before binding. Configuration
    /// protection does not replace service assembly or peer authentication.
    ///
    /// # Errors
    ///
    /// Refuses changed/unsafe configuration paths or a native bind failure.
    pub fn bind_configured(
        service: &'owner mut ResidentFrameService<'service>,
        configuration: &LocalEndpointConfiguration,
        limits: LocalIpcFrameLimits,
        maximum_connections: NonZeroUsize,
        authentication: ResidentConnectionAuthentication<'owner>,
    ) -> Result<Self, ResidentHostError> {
        Self::bind(
            service,
            configuration.verified_path()?,
            limits,
            maximum_connections,
            authentication,
        )
    }

    /// Bind a fresh endpoint for an already assembled resident service.
    ///
    /// The caller owns directory confinement, permissions and endpoint
    /// lifecycle. Binding never removes a predecessor socket or other file.
    /// The supplied credentials authenticate each connection; application assembly
    /// still owns writer, trust, session and protected request admission.
    ///
    /// # Errors
    ///
    /// Returns the native carrier error when the endpoint cannot be bound.
    pub fn bind(
        service: &'owner mut ResidentFrameService<'service>,
        endpoint: &Path,
        limits: LocalIpcFrameLimits,
        maximum_connections: NonZeroUsize,
        authentication: ResidentConnectionAuthentication<'owner>,
    ) -> Result<Self, ResidentHostError> {
        Ok(Self {
            listener: UnixSocketListener::bind(endpoint, limits)?,
            service,
            coordinator: ResidentWorkTurnCoordinator::new(),
            pending: VecDeque::new(),
            maximum_connections: maximum_connections.get(),
            authentication,
        })
    }

    /// Offer at most one new connection and run one application work turn.
    ///
    /// `accept` supplies deadlines only for a new connection. `None` prevents
    /// accepting another connection while allowing retained application work.
    /// Accepted connections keep their original deadlines even when another
    /// offer is supplied. A quiet connection does not prevent another retained
    /// connection from becoming the ready frame. No accept or readiness check
    /// waits. Each handshake step reads at most one ready frame within its original
    /// deadline. The caller key is selected again before application dispatch.
    /// The existing coordinator fairly arbitrates authenticated frames,
    /// interrupted phases and the caller's recovery opportunity; it also runs
    /// after a handshake step or authentication failure.
    ///
    /// # Errors
    ///
    /// Returns a redacted carrier, authentication or application failure. A failed frame is
    /// closed and never retried; any durable uncertainty stays application-owned.
    pub fn run_one(
        &mut self,
        accept: Option<ConnectionDeadlines>,
        recovery_ready: bool,
    ) -> Result<ResidentHostTurn, ResidentHostError> {
        if let Some(index) = self
            .pending
            .iter()
            .position(|pending| pending.deadlines.expired())
        {
            self.pending.remove(index);
            return Ok(ResidentHostTurn::ConnectionExpired);
        }
        if self.pending.len() < self.maximum_connections {
            if let Some(deadlines) = accept {
                if deadlines.expired() {
                    return Err(ResidentHostError::ExpiredOffer);
                }
                if let Some(carrier) = self.listener.try_accept()? {
                    self.pending.push_back(PendingConnection {
                        carrier,
                        deadlines,
                        authentication: ConnectionAuthentication::Hello,
                    });
                }
            }
        }
        let mut ready = None;
        for index in 0..self.pending.len() {
            if matches!(
                self.pending[index].authentication,
                ConnectionAuthentication::Verified(_)
            ) {
                ready = Some(index);
                break;
            }
            match self.pending[index].carrier.is_readable() {
                Ok(true) => {
                    ready = Some(index);
                    break;
                }
                Ok(false) => {}
                Err(error) => {
                    self.pending.remove(index);
                    return Err(error.into());
                }
            }
        }
        // A handshake takes at most one readable frame. Recovery and retained
        // application phases still get a turn even when authentication fails.
        let mut authentication_error = None;
        if let Some(index) = ready {
            if let Err(error) = self.authenticate(index) {
                self.pending.remove(index);
                ready = None;
                authentication_error = Some(error);
            }
        }
        let mut input = ResidentWorkTurnInput::idle();
        if recovery_ready {
            input = input.with_recovery();
        }
        let mut authenticated_frame = None;
        if let Some(index) = ready {
            if let Some(pending) = self.pending.get_mut(index) {
                if let ConnectionAuthentication::Verified(request) = &mut pending.authentication {
                    authenticated_frame = Some((
                        AuthenticatedFrame {
                            carrier: &mut pending.carrier,
                            signer: self.authentication.signer,
                            request,
                            read: false,
                        },
                        pending.deadlines,
                    ));
                }
            }
        }
        if let Some((frame, deadlines)) = &mut authenticated_frame {
            input = input.with_frame(ResidentReadyFrame::new(
                frame,
                deadlines.frame,
                deadlines.response,
            ));
        }
        let outcome = self.coordinator.run_one(self.service, input);
        if matches!(&outcome, Ok(ResidentWorkTurnOutcome::Frame(_)) | Err(_)) {
            if let Some(index) = ready {
                self.pending.remove(index);
            }
        }
        if let Some(error) = authentication_error {
            return Err(error.into());
        }
        match outcome? {
            ResidentWorkTurnOutcome::Idle if !self.pending.is_empty() => {
                Ok(ResidentHostTurn::AwaitingFrame)
            }
            ResidentWorkTurnOutcome::Idle => Ok(ResidentHostTurn::Idle),
            work => Ok(ResidentHostTurn::Work(work)),
        }
    }

    fn authenticate(&mut self, index: usize) -> Result<(), LocalAuthenticationError> {
        let pending = &mut self.pending[index];
        let state = std::mem::replace(&mut pending.authentication, ConnectionAuthentication::Hello);
        let state = match state {
            ConnectionAuthentication::Hello => {
                ConnectionAuthentication::Challenged(begin_authenticated_request(
                    &mut pending.carrier,
                    self.authentication.signer,
                    self.authentication.random,
                    pending.deadlines.frame.min(pending.deadlines.response),
                    self.authentication.select_caller,
                )?)
            }
            ConnectionAuthentication::Challenged(challenge) => ConnectionAuthentication::Verified(
                Some(challenge.receive_request(&mut pending.carrier)?),
            ),
            state @ ConnectionAuthentication::Verified(_) => state,
        };
        if let ConnectionAuthentication::Verified(request) = &state {
            if !request
                .as_ref()
                .is_some_and(|request| (self.authentication.select_caller)(request.caller_key()))
            {
                return Err(LocalAuthenticationError::Peer);
            }
        }
        pending.authentication = state;
        Ok(())
    }
}
