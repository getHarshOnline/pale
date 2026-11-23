//! Stable-v1 ACP conductor over one supplied byte connection.

use std::sync::{Arc, Mutex};

use agent_client_protocol::schema::v1::{
    CancelNotification, ClientCapabilities, Implementation, InitializeRequest,
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SelectedPermissionOutcome, SessionNotification,
};
use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::util::MatchDispatch;
use agent_client_protocol::{ActiveSession, Agent, ByteStreams, ConnectionTo, SessionMessage};
use futures::{pin_mut, select_biased, AsyncRead, AsyncWrite, Future, FutureExt};
use pale_sdk::{ApiInitializationError, DiagnosticProjector};

use crate::cancellation::AcpCancellation;
use crate::error::{AcpError, AcpErrorKind};
use crate::framing::{BoundedIncoming, InboundLimitExceeded, InboundLimitStatus};
use crate::protocol::{
    project_stop_reason, project_update, AcpRequestError, AcpSessionEvent, AcpSessionId,
    AcpSessionState, AcpTurnOutcome, AcpTurnRequest, AcpUpdate, StopReasonProjectionError,
    UpdateProjectionError, ACP_TURN_UPDATE_LIMIT,
};

/// Stable ACP-v1 conductor over one caller-supplied byte connection.
///
/// Each [`Self::conduct_turn`] call consumes exactly one caller-supplied byte
/// connection, creates one native ACP session, and permits one active prompt.
/// The conductor does not launch an executable, retain the session afterward,
/// or proxy unrelated Pale SDK operations. Callers compose the canonical SDK
/// separately when an agent turn participates in a Pale review.
pub struct AcpConductor {
    diagnostics: DiagnosticProjector,
}

impl AcpConductor {
    /// Construct the conductor with canonical Pale diagnostic projection.
    ///
    /// # Errors
    ///
    /// Returns [`ApiInitializationError`] when the embedded diagnostic contract
    /// is incoherent.
    pub fn new() -> Result<Self, ApiInitializationError> {
        DiagnosticProjector::new().map(|diagnostics| Self { diagnostics })
    }

    /// Conduct one stable-v1 prompt turn over an established byte connection.
    ///
    /// Updates are delivered synchronously to `observe` in exact receipt order.
    /// The observer must return promptly: it runs inline so the client can avoid
    /// a second queue or transcript, and time spent inside caller code cannot be
    /// preempted by `deadline`. The client retains only the count and terminal
    /// reason. The caller's async runtime supplies a future that becomes ready
    /// at a finite deadline; this crate owns no clock or executor. A never-ready
    /// future would deliberately remove the temporal bound. Pre-prompt
    /// cancellation drops the connection without fabricating `session/cancel`;
    /// after a prompt starts, cancellation is cooperative and this method waits
    /// for the agent's terminal response after forwarding `session/cancel`.
    /// Neither path treats the request as settlement. Per-frame and aggregate
    /// inbound ceilings include negotiation, permission, control, update, and
    /// terminal messages. A peer that exceeds a resource ceiling is failed
    /// closed immediately after the client sends its best-effort cancellation.
    ///
    /// # Errors
    ///
    /// Returns [`AcpError`] for invalid caller input, non-v1 negotiation,
    /// bounded-input failure, an unsupported stable-v1 message, transport
    /// termination before an agent stop reason, or expiry of the caller-supplied
    /// deadline future. Failures after session creation retain the native
    /// session correlation. Deadline expiry drops the connection; it does not
    /// prove that the native agent stopped or that any effect settled.
    ///
    /// # Panics
    ///
    /// A panic from `observe` propagates after the in-flight connection is
    /// dropped. The client never converts caller failure into an ACP result.
    pub async fn conduct_turn<Outgoing, Incoming, Deadline, Observe>(
        &self,
        outgoing: Outgoing,
        incoming: Incoming,
        request: AcpTurnRequest,
        cancellation: AcpCancellation,
        deadline: Deadline,
        mut observe: Observe,
    ) -> Result<AcpTurnOutcome, AcpError>
    where
        Outgoing: AsyncWrite + Send + 'static,
        Incoming: AsyncRead + Send + 'static,
        Deadline: Future<Output = ()> + Send,
        Observe: FnMut(AcpUpdate) + Send,
    {
        request.validate().map_err(|error| {
            let summary = match error {
                AcpRequestError::RelativeWorkingDirectory => {
                    "ACP working-directory correlation must be absolute"
                }
                AcpRequestError::EmptyPrompt => "ACP prompt must not be empty",
                AcpRequestError::PromptTooLarge => "ACP prompt exceeds the bounded byte ceiling",
            };
            AcpError::new(
                &self.diagnostics,
                AcpErrorKind::InvalidRequest,
                AcpSessionState::Disconnected,
                None,
                summary,
                None,
            )
        })?;
        if cancellation.is_cancelled() {
            return Err(AcpError::new(
                &self.diagnostics,
                AcpErrorKind::CancelledBeforePrompt,
                AcpSessionState::Disconnected,
                None,
                "ACP turn was cancelled before protocol negotiation",
                None,
            ));
        }

        let (cwd, prompt) = request.into_parts();
        let inbound_status = InboundLimitStatus::default();
        let bounded_incoming = BoundedIncoming::new(incoming, inbound_status.clone());
        let progress = TurnProgress::default();
        let active_progress = progress.clone();
        let turn = agent_client_protocol::Client
            .builder()
            .on_receive_request(
                async move |request: RequestPermissionRequest, responder, _connection| {
                    let rejection = request
                        .options
                        .iter()
                        .find(|option| {
                            matches!(
                                option.kind,
                                agent_client_protocol::schema::v1::PermissionOptionKind::RejectOnce
                            )
                        })
                        .map(|option| option.option_id.clone());
                    let outcome = rejection.map_or(RequestPermissionOutcome::Cancelled, |id| {
                        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(id))
                    });
                    responder.respond(RequestPermissionResponse::new(outcome))
                },
                agent_client_protocol::on_receive_request!(),
            )
            .connect_with(
                ByteStreams::new(outgoing, bounded_incoming),
                async move |connection: ConnectionTo<Agent>| {
                    let initialize = connection
                        .send_request(
                            InitializeRequest::new(ProtocolVersion::V1)
                                .client_capabilities(ClientCapabilities::new())
                                .client_info(Implementation::new(
                                    "pale-acp",
                                    env!("CARGO_PKG_VERSION"),
                                )),
                        )
                        .block_task()
                        .fuse();
                    let cancelled = cancellation.cancelled().fuse();
                    pin_mut!(initialize, cancelled);
                    let initialized = select_biased! {
                        () = cancelled => return Ok(Err(active_progress.failure(
                            TurnFailureKind::CancelledBeforePrompt,
                        ))),
                        result = initialize => result?,
                    };
                    if initialized.protocol_version != ProtocolVersion::V1 {
                        return Ok(Err(
                            active_progress.failure(TurnFailureKind::UnsupportedVersion)
                        ));
                    }
                    if let Err(error) = active_progress.advance(AcpSessionEvent::ProtocolV1Accepted)
                    {
                        return Ok(Err(error));
                    }

                    let session_start = connection
                        .build_session(&cwd)
                        .block_task()
                        .start_session()
                        .fuse();
                    let cancelled = cancellation.cancelled().fuse();
                    pin_mut!(session_start, cancelled);
                    let mut session = select_biased! {
                        () = cancelled => return Ok(Err(active_progress.failure(
                            TurnFailureKind::CancelledBeforePrompt,
                        ))),
                        result = session_start => result?,
                    };
                    let session_id = AcpSessionId::from_native(session.session_id());
                    if let Err(error) = active_progress.establish_session(session_id.clone()) {
                        return Ok(Err(error));
                    }
                    if cancellation.is_cancelled() {
                        return Ok(Err(
                            active_progress.failure(TurnFailureKind::CancelledBeforePrompt)
                        ));
                    }
                    session.send_prompt(prompt)?;
                    if let Err(error) = active_progress.advance(AcpSessionEvent::PromptStarted) {
                        return Ok(Err(error));
                    }
                    let mut update_count = 0_u32;
                    loop {
                        let message =
                            if active_progress.state() == AcpSessionState::CancellationRequested {
                                session.read_update().await?
                            } else {
                                let connection = session.connection().clone();
                                let cancel_session_id = session.session_id().clone();
                                let update = session.read_update().fuse();
                                let cancelled = cancellation.cancelled().fuse();
                                pin_mut!(update, cancelled);
                                select_biased! {
                                    () = cancelled => {
                                        connection.send_notification(
                                            CancelNotification::new(cancel_session_id)
                                        )?;
                                        if let Err(error) = active_progress
                                            .advance(AcpSessionEvent::CancellationRequested)
                                        {
                                            return Ok(Err(error));
                                        }
                                        continue;
                                    },
                                    message = update => message?,
                                }
                            };
                        match message {
                            SessionMessage::SessionMessage(dispatch) => {
                                let Some(notification) = notification(dispatch).await? else {
                                    return Ok(Err(TurnFailure::new(
                                        active_progress.snapshot(),
                                        TurnFailureKind::UnknownSessionMessage,
                                    )));
                                };
                                let Some(sequence) = update_count.checked_add(1) else {
                                    return Ok(Err(TurnFailure::new(
                                        active_progress.snapshot(),
                                        TurnFailureKind::UpdateLimit,
                                    )));
                                };
                                if sequence > ACP_TURN_UPDATE_LIMIT {
                                    send_best_effort_cancel(&session);
                                    return Ok(Err(TurnFailure::new(
                                        active_progress.snapshot(),
                                        TurnFailureKind::UpdateLimit,
                                    )));
                                }
                                let update = match project_update(sequence, notification.update) {
                                    Ok(update) => update,
                                    Err(error) => {
                                        send_best_effort_cancel(&session);
                                        return Ok(Err(TurnFailure::new(
                                            active_progress.snapshot(),
                                            TurnFailureKind::UpdateProjection(error),
                                        )));
                                    }
                                };
                                observe(update);
                                update_count = sequence;
                            }
                            SessionMessage::StopReason(reason) => {
                                let cancellation_requested = active_progress.state()
                                    == AcpSessionState::CancellationRequested;
                                if let Err(error) =
                                    active_progress.advance(AcpSessionEvent::TurnStopped)
                                {
                                    return Ok(Err(error));
                                }
                                let stop_reason = match project_stop_reason(reason) {
                                    Ok(reason) => reason,
                                    Err(error) => {
                                        return Ok(Err(TurnFailure::new(
                                            active_progress.snapshot(),
                                            TurnFailureKind::StopReasonProjection(error),
                                        )));
                                    }
                                };
                                return Ok(Ok(AcpTurnOutcome::new(
                                    session_id,
                                    stop_reason,
                                    update_count,
                                    cancellation_requested,
                                )));
                            }
                            _ => {
                                return Ok(Err(TurnFailure::new(
                                    active_progress.snapshot(),
                                    TurnFailureKind::UnknownSessionMessage,
                                )))
                            }
                        }
                    }
                },
            )
            .fuse();
        let deadline = deadline.fuse();
        pin_mut!(turn, deadline);
        select_biased! {
            result = turn => self.project_result(
                result,
                inbound_status.exceeded(),
                progress.snapshot(),
            ),
            () = deadline => {
                let failure = progress.transport_failure();
                Err(AcpError::new(
                    &self.diagnostics,
                    AcpErrorKind::DeadlineExceeded,
                    failure.state,
                    failure.session_id,
                    "ACP turn deadline elapsed before terminal confirmation",
                    None,
                ))
            },
        }
    }

    fn project_result(
        &self,
        result: Result<Result<AcpTurnOutcome, TurnFailure>, agent_client_protocol::Error>,
        inbound_limit: Option<InboundLimitExceeded>,
        progress: TurnProgressState,
    ) -> Result<AcpTurnOutcome, AcpError> {
        match result {
            Ok(Ok(outcome)) => Ok(outcome),
            Ok(Err(TurnFailure {
                progress: TurnProgressState { state, session_id },
                kind: TurnFailureKind::UnsupportedVersion,
            })) => Err(AcpError::new(
                &self.diagnostics,
                AcpErrorKind::UnsupportedProtocolVersion,
                state,
                session_id,
                "ACP peer selected a protocol version other than stable v1",
                None,
            )),
            Ok(Err(TurnFailure {
                progress: TurnProgressState { state, session_id },
                kind: TurnFailureKind::CancelledBeforePrompt,
            })) => Err(AcpError::new(
                &self.diagnostics,
                AcpErrorKind::CancelledBeforePrompt,
                state,
                session_id,
                "ACP turn was cancelled before its prompt started",
                None,
            )),
            Ok(Err(TurnFailure {
                progress: TurnProgressState { state, session_id },
                kind:
                    TurnFailureKind::UpdateLimit
                    | TurnFailureKind::UpdateProjection(UpdateProjectionError::TextTooLarge),
            })) => Err(AcpError::new(
                &self.diagnostics,
                AcpErrorKind::UpdateLimitExceeded,
                state,
                session_id,
                "ACP peer exceeded the bounded update ceiling",
                None,
            )),
            Ok(Err(TurnFailure {
                progress: TurnProgressState { state, session_id },
                kind:
                    TurnFailureKind::UpdateProjection(
                        UpdateProjectionError::UnsupportedContent
                        | UpdateProjectionError::UnknownStableV1Update,
                    )
                    | TurnFailureKind::StopReasonProjection(StopReasonProjectionError)
                    | TurnFailureKind::UnknownSessionMessage
                    | TurnFailureKind::Transition,
            })) => Err(AcpError::new(
                &self.diagnostics,
                AcpErrorKind::UnsupportedStableV1Message,
                state,
                session_id,
                "ACP peer emitted a stable-v1 message this pinned conductor cannot interpret",
                None,
            )),
            Err(source) => {
                let (kind, summary) = match inbound_limit {
                    Some(InboundLimitExceeded::FrameBytes) => (
                        AcpErrorKind::FrameLimitExceeded,
                        "ACP peer exceeded the newline-delimited frame byte ceiling",
                    ),
                    Some(InboundLimitExceeded::TurnBytes) => (
                        AcpErrorKind::InboundByteLimitExceeded,
                        "ACP peer exceeded the aggregate inbound byte ceiling",
                    ),
                    Some(InboundLimitExceeded::TurnEvents) => (
                        AcpErrorKind::InboundEventLimitExceeded,
                        "ACP peer exceeded the aggregate inbound event ceiling",
                    ),
                    None => (
                        AcpErrorKind::Transport,
                        "ACP transport ended before the prompt reached a terminal response",
                    ),
                };
                let failure = progress.transport_failure();
                Err(AcpError::new(
                    &self.diagnostics,
                    kind,
                    failure.state,
                    failure.session_id,
                    summary,
                    Some(Box::new(source)),
                ))
            }
        }
    }
}

fn send_best_effort_cancel(session: &ActiveSession<'_, Agent>) {
    let _result = session
        .connection()
        .send_notification(CancelNotification::new(session.session_id().clone()));
}

async fn notification(
    dispatch: agent_client_protocol::Dispatch,
) -> Result<Option<SessionNotification>, agent_client_protocol::Error> {
    let mut notification = None;
    MatchDispatch::new(dispatch)
        .if_notification(async |value: SessionNotification| {
            notification = Some(value);
            Ok::<(), agent_client_protocol::Error>(())
        })
        .await
        .otherwise_ignore()?;
    Ok(notification)
}

/// Internal turn failure paired with the last coherent progress witness.
#[derive(Debug)]
struct TurnFailure {
    progress: TurnProgressState,
    kind: TurnFailureKind,
}

impl TurnFailure {
    const fn new(progress: TurnProgressState, kind: TurnFailureKind) -> Self {
        Self { progress, kind }
    }
}

/// Closed failure domain produced inside the protocol task.
#[derive(Debug)]
enum TurnFailureKind {
    /// The peer negotiated a protocol version other than stable v1.
    UnsupportedVersion,
    /// Cancellation won before the sole prompt began.
    CancelledBeforePrompt,
    /// The turn exceeded its retained-update ceiling.
    UpdateLimit,
    /// One native update could not enter the bounded text-only projection.
    UpdateProjection(UpdateProjectionError),
    /// The native terminal reason has no stable-v1 projection.
    StopReasonProjection(StopReasonProjectionError),
    /// The peer emitted a session message outside the supported stable-v1 set.
    UnknownSessionMessage,
    /// An internal event contradicted the closed [`AcpSessionState`] lifecycle.
    Transition,
}

/// Shared progress witness between the protocol task and its outer deadline.
///
/// The mutex protects one small state product rather than native message
/// bodies. A poisoned lock retains that last product so failure reporting does
/// not erase an already established [`AcpSessionId`].
#[derive(Clone, Debug, Default)]
struct TurnProgress(Arc<Mutex<TurnProgressState>>);

/// Exact lifecycle state and optional native session correlation for one turn.
#[derive(Clone, Debug)]
struct TurnProgressState {
    state: AcpSessionState,
    session_id: Option<AcpSessionId>,
}

impl Default for TurnProgressState {
    fn default() -> Self {
        Self {
            state: AcpSessionState::Disconnected,
            session_id: None,
        }
    }
}

impl TurnProgress {
    fn snapshot(&self) -> TurnProgressState {
        self.with_state(|progress| progress.clone())
    }

    fn state(&self) -> AcpSessionState {
        self.with_state(|progress| progress.state)
    }

    fn establish_session(&self, session_id: AcpSessionId) -> Result<(), TurnFailure> {
        self.with_state_mut(|progress| {
            progress.session_id = Some(session_id);
            advance_progress(progress, AcpSessionEvent::SessionOpened)
        })
    }

    fn advance(&self, event: AcpSessionEvent) -> Result<(), TurnFailure> {
        self.with_state_mut(|progress| advance_progress(progress, event))
    }

    fn failure(&self, kind: TurnFailureKind) -> TurnFailure {
        TurnFailure::new(self.snapshot(), kind)
    }

    fn transport_failure(&self) -> TurnProgressState {
        self.snapshot().transport_failure()
    }

    /// Inspect the progress product, retaining it if a prior holder panicked.
    fn with_state<T>(&self, inspect: impl FnOnce(&TurnProgressState) -> T) -> T {
        let guard = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inspect(&guard)
    }

    /// Mutate the progress product, retaining it if a prior holder panicked.
    fn with_state_mut<T>(&self, change: impl FnOnce(&mut TurnProgressState) -> T) -> T {
        let mut guard = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        change(&mut guard)
    }
}

impl TurnProgressState {
    /// Project transport loss without replacing an already terminal state.
    fn transport_failure(mut self) -> Self {
        if let Ok(state) = self.state.advance(AcpSessionEvent::TransportFailed) {
            self.state = state;
        }
        self
    }
}

/// Advance the shared product or retain its exact rejected predecessor.
fn advance_progress(
    progress: &mut TurnProgressState,
    event: AcpSessionEvent,
) -> Result<(), TurnFailure> {
    progress.state = progress
        .state
        .advance(event)
        .map_err(|_| TurnFailure::new(progress.clone(), TurnFailureKind::Transition))?;
    Ok(())
}
