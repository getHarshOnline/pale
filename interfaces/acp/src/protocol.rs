//! Binding-safe ACP session projections.

use std::path::PathBuf;
use std::{error::Error, fmt};

use agent_client_protocol::schema::v1::{
    ContentBlock, Plan, SessionUpdate, StopReason, ToolCall, ToolCallUpdate,
};

/// Maximum UTF-8 bytes admitted in one text-only prompt.
pub const ACP_PROMPT_BYTE_LIMIT: usize = 1 << 20;
/// Maximum bytes admitted before one incoming newline frame delimiter.
///
/// The bound is enforced before the official SDK allocates and parses the
/// complete JSON-RPC line. It includes JSON syntax and every encoded payload,
/// not only text projected through [`AcpUpdate::text`].
pub const ACP_INCOMING_FRAME_BYTE_LIMIT: usize = 2 << 20;
/// Maximum ordered updates accepted during one prompt turn.
pub const ACP_TURN_UPDATE_LIMIT: u32 = 4_096;
/// Maximum newline-delimited JSON-RPC messages admitted during one turn.
///
/// This aggregate ceiling includes negotiation, session control, permission
/// requests, updates, and the terminal response. The fixed control allowance
/// keeps the independently documented update ceiling reachable.
pub const ACP_TURN_INBOUND_EVENT_LIMIT: u32 = ACP_TURN_UPDATE_LIMIT + 64;
/// Maximum aggregate bytes admitted from the peer during one turn.
///
/// Every byte, including newline delimiters and permission/control traffic,
/// counts toward this ceiling independently of the per-frame bound.
pub const ACP_TURN_INBOUND_BYTE_LIMIT: usize = 8 * ACP_INCOMING_FRAME_BYTE_LIMIT;
/// Maximum UTF-8 bytes projected from one streamed text chunk.
pub const ACP_UPDATE_TEXT_BYTE_LIMIT: usize = 1 << 20;

/// Closed lifecycle state of one bounded ACP client turn.
///
/// In the relation below, `S` denotes this enum's complete state set.
/// Transport failure or caller-deadline expiry terminates any nonterminal
/// in-flight state as [`Self::TransportLost`].
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/session-transition.display.mathml"
))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcpSessionState {
    /// No protocol negotiation has completed.
    Disconnected,
    /// The peer accepted exact stable wire version 1.
    InitializedV1,
    /// One native ACP session is open with no active prompt.
    SessionOpen,
    /// The sole prompt turn is active.
    PromptActive,
    /// `session/cancel` was sent and terminal confirmation is pending.
    CancellationRequested,
    /// The agent returned one terminal stop reason.
    TurnEnded,
    /// The carrier was lost or abandoned at its deadline, or the peer failed
    /// before a terminal stop reason arrived.
    TransportLost,
}

/// Closed event accepted by [`AcpSessionState::advance`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcpSessionEvent {
    /// Exact ACP wire version 1 was negotiated.
    ProtocolV1Accepted,
    /// The peer created one native session.
    SessionOpened,
    /// The sole prompt was sent.
    PromptStarted,
    /// Cooperative cancellation was forwarded to the peer.
    CancellationRequested,
    /// The peer returned a terminal stop reason.
    TurnStopped,
    /// The byte carrier or protocol peer failed.
    TransportFailed,
}

/// Rejected ACP session-state transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AcpTransitionError {
    state: AcpSessionState,
    event: AcpSessionEvent,
}

impl AcpTransitionError {
    /// Return the state from which the rejected transition was attempted.
    #[must_use]
    pub const fn state(self) -> AcpSessionState {
        self.state
    }

    /// Return the event that was invalid for [`Self::state`].
    #[must_use]
    pub const fn event(self) -> AcpSessionEvent {
        self.event
    }
}

impl fmt::Display for AcpTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ACP session event is invalid for the current state")
    }
}

impl Error for AcpTransitionError {}

impl AcpSessionState {
    /// Apply one event to the closed monotone turn lifecycle.
    ///
    /// # Errors
    ///
    /// Returns [`AcpTransitionError`] when the event would skip, repeat, or
    /// reverse a protocol state. Transport failure is terminal from every
    /// nonterminal state.
    pub fn advance(self, event: AcpSessionEvent) -> Result<Self, AcpTransitionError> {
        let next = match (self, event) {
            (Self::Disconnected, AcpSessionEvent::ProtocolV1Accepted) => Self::InitializedV1,
            (Self::InitializedV1, AcpSessionEvent::SessionOpened) => Self::SessionOpen,
            (Self::SessionOpen, AcpSessionEvent::PromptStarted) => Self::PromptActive,
            (Self::PromptActive, AcpSessionEvent::CancellationRequested) => {
                Self::CancellationRequested
            }
            (Self::PromptActive | Self::CancellationRequested, AcpSessionEvent::TurnStopped) => {
                Self::TurnEnded
            }
            (
                Self::Disconnected
                | Self::InitializedV1
                | Self::SessionOpen
                | Self::PromptActive
                | Self::CancellationRequested,
                AcpSessionEvent::TransportFailed,
            ) => Self::TransportLost,
            _ => return Err(AcpTransitionError { state: self, event }),
        };
        Ok(next)
    }
}

/// One text-only prompt over an already-established ACP byte connection.
///
/// Its [`fmt::Debug`] projection reports only structural sizes; it never emits
/// the working directory or prompt text.
#[derive(Clone, Eq, PartialEq)]
pub struct AcpTurnRequest {
    cwd: PathBuf,
    prompt: String,
}

impl fmt::Debug for AcpTurnRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AcpTurnRequest")
            .field("cwd_component_count", &self.cwd.components().count())
            .field("prompt_byte_len", &self.prompt.len())
            .finish()
    }
}

impl AcpTurnRequest {
    /// Bind one absolute working-directory correlation to one prompt.
    ///
    /// The directory is sent to the native agent as session context. It grants
    /// no filesystem authority because this client advertises no filesystem or
    /// terminal capability and implements no corresponding handler.
    #[must_use]
    pub fn new(cwd: PathBuf, prompt: String) -> Self {
        Self { cwd, prompt }
    }

    pub(crate) fn validate(&self) -> Result<(), AcpRequestError> {
        if !self.cwd.is_absolute() {
            return Err(AcpRequestError::RelativeWorkingDirectory);
        }
        if self.prompt.trim().is_empty() {
            return Err(AcpRequestError::EmptyPrompt);
        }
        if self.prompt.len() > ACP_PROMPT_BYTE_LIMIT {
            return Err(AcpRequestError::PromptTooLarge);
        }
        Ok(())
    }

    pub(crate) fn into_parts(self) -> (PathBuf, String) {
        (self.cwd, self.prompt)
    }
}

/// Caller-input condition rejected before ACP negotiation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AcpRequestError {
    /// The native working-directory correlation is not absolute.
    RelativeWorkingDirectory,
    /// The prompt is empty after trimming whitespace.
    EmptyPrompt,
    /// The prompt exceeds [`ACP_PROMPT_BYTE_LIMIT`].
    PromptTooLarge,
}

/// Stable kind of one ordered ACP session update.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcpUpdateKind {
    /// Echoed or transformed user-message content.
    UserMessage,
    /// Agent response content.
    AgentMessage,
    /// Agent reasoning content selected for protocol delivery.
    AgentThought,
    /// A native agent tool call began.
    ToolCall,
    /// A native agent tool call changed state.
    ToolCallUpdate,
    /// The native agent published a plan projection.
    Plan,
    /// The native agent's available commands changed.
    AvailableCommands,
    /// The native agent's active mode changed.
    CurrentMode,
    /// The native agent's session configuration changed.
    Configuration,
    /// The native agent's session metadata changed.
    SessionInformation,
    /// The native agent published usage information.
    Usage,
}

/// Structured native content carried by an ACP session update.
///
/// These values retain the exact stable-v1 types decoded by the pinned ACP
/// SDK. Pale does not reinterpret a tool call, tool-call update, or plan as a
/// Pale operation, effect, authority grant, or settlement.
#[derive(Clone, PartialEq)]
pub enum AcpStructuredUpdate {
    /// A native agent tool call began.
    ToolCall(Box<ToolCall>),
    /// A native agent tool call changed state.
    ToolCallUpdate(Box<ToolCallUpdate>),
    /// The native agent published a complete plan projection.
    Plan(Plan),
}

impl fmt::Debug for AcpStructuredUpdate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ToolCall(_) => "ToolCall(..)",
            Self::ToolCallUpdate(_) => "ToolCallUpdate(..)",
            Self::Plan(_) => "Plan(..)",
        })
    }
}

/// One bounded update in exact receipt order.
///
/// `sequence` is a process-local ordinal for this turn only. It is not an ACP
/// request, Pale operation, event, Genealogy, generation, or settlement
/// identity. Text-stream updates whose [`ContentBlock`] is not text are
/// rejected rather than silently erased.
/// Tool calls, tool-call updates, and plans preserve their exact native
/// stable-v1 values through [`Self::structured`]. Other non-content updates are
/// represented by [`Self::kind`] alone. This interface does not retain a native
/// transcript. Its [`fmt::Debug`] projection reports structure and text size,
/// never native payload content.
#[derive(Clone, PartialEq)]
pub struct AcpUpdate {
    sequence: u32,
    kind: AcpUpdateKind,
    text: Option<String>,
    structured: Option<AcpStructuredUpdate>,
}

impl fmt::Debug for AcpUpdate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AcpUpdate")
            .field("sequence", &self.sequence)
            .field("kind", &self.kind)
            .field("text_byte_len", &self.text.as_ref().map(String::len))
            .field("has_structured_payload", &self.structured.is_some())
            .finish()
    }
}

impl AcpUpdate {
    /// Return the one-based update ordinal within this turn.
    #[must_use]
    pub const fn sequence(&self) -> u32 {
        self.sequence
    }

    /// Return the stable update kind.
    #[must_use]
    pub const fn kind(&self) -> AcpUpdateKind {
        self.kind
    }

    /// Borrow text carried by a stable text chunk, when present.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    /// Borrow the exact native tool-call or plan payload, when present.
    ///
    /// The payload was admitted under the same per-frame and aggregate-turn
    /// bounds as every other incoming ACP message. Borrowing it does not grant
    /// tool authority or make it Pale-owned state.
    #[must_use]
    pub const fn structured(&self) -> Option<&AcpStructuredUpdate> {
        self.structured.as_ref()
    }
}

/// Project one native stable-v1 update into the bounded Pale view.
///
/// # Errors
///
/// Returns [`UpdateProjectionError`] when the update is outside the supported
/// stable-v1 set, carries non-text content, or its text exceeds
/// [`ACP_UPDATE_TEXT_BYTE_LIMIT`].
pub(crate) fn project_update(
    sequence: u32,
    update: SessionUpdate,
) -> Result<AcpUpdate, UpdateProjectionError> {
    let (kind, content, structured) = match update {
        SessionUpdate::UserMessageChunk(chunk) => {
            (AcpUpdateKind::UserMessage, Some(chunk.content), None)
        }
        SessionUpdate::AgentMessageChunk(chunk) => {
            (AcpUpdateKind::AgentMessage, Some(chunk.content), None)
        }
        SessionUpdate::AgentThoughtChunk(chunk) => {
            (AcpUpdateKind::AgentThought, Some(chunk.content), None)
        }
        SessionUpdate::ToolCall(tool_call) => (
            AcpUpdateKind::ToolCall,
            None,
            Some(AcpStructuredUpdate::ToolCall(Box::new(tool_call))),
        ),
        SessionUpdate::ToolCallUpdate(update) => (
            AcpUpdateKind::ToolCallUpdate,
            None,
            Some(AcpStructuredUpdate::ToolCallUpdate(Box::new(update))),
        ),
        SessionUpdate::Plan(plan) => (
            AcpUpdateKind::Plan,
            None,
            Some(AcpStructuredUpdate::Plan(plan)),
        ),
        SessionUpdate::AvailableCommandsUpdate(_) => (AcpUpdateKind::AvailableCommands, None, None),
        SessionUpdate::CurrentModeUpdate(_) => (AcpUpdateKind::CurrentMode, None, None),
        SessionUpdate::ConfigOptionUpdate(_) => (AcpUpdateKind::Configuration, None, None),
        SessionUpdate::SessionInfoUpdate(_) => (AcpUpdateKind::SessionInformation, None, None),
        SessionUpdate::UsageUpdate(_) => (AcpUpdateKind::Usage, None, None),
        _ => return Err(UpdateProjectionError::UnknownStableV1Update),
    };
    let text = match content {
        Some(ContentBlock::Text(text)) => Some(text.text),
        Some(_) => return Err(UpdateProjectionError::UnsupportedContent),
        None => None,
    };
    if text
        .as_ref()
        .is_some_and(|text| text.len() > ACP_UPDATE_TEXT_BYTE_LIMIT)
    {
        return Err(UpdateProjectionError::TextTooLarge);
    }
    Ok(AcpUpdate {
        sequence,
        kind,
        text,
        structured,
    })
}

/// Reason a native update cannot enter [`AcpUpdate`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UpdateProjectionError {
    /// A text chunk exceeds [`ACP_UPDATE_TEXT_BYTE_LIMIT`].
    TextTooLarge,
    /// A content-bearing update carries a non-text block.
    UnsupportedContent,
    /// The update variant is outside the stable-v1 projection.
    UnknownStableV1Update,
}

/// Stable reason reported by the native agent when one prompt turn ends.
///
/// A stop reason terminates only the ACP prompt. It is not Pale acceptance,
/// Pale Runtime assembly, effect certainty, native read-back, or settlement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcpStopReason {
    /// The agent ended the turn normally.
    EndTurn,
    /// The agent reached its token ceiling.
    MaxTokens,
    /// The agent reached its turn-request ceiling.
    MaxTurnRequests,
    /// The agent refused to continue.
    Refusal,
    /// The agent confirmed cooperative cancellation.
    Cancelled,
}

/// Project one native terminal reason without an unknown fallback.
///
/// # Errors
///
/// Returns [`StopReasonProjectionError`] when `reason` has no stable-v1
/// [`AcpStopReason`] counterpart.
pub(crate) fn project_stop_reason(
    reason: StopReason,
) -> Result<AcpStopReason, StopReasonProjectionError> {
    match reason {
        StopReason::EndTurn => Ok(AcpStopReason::EndTurn),
        StopReason::MaxTokens => Ok(AcpStopReason::MaxTokens),
        StopReason::MaxTurnRequests => Ok(AcpStopReason::MaxTurnRequests),
        StopReason::Refusal => Ok(AcpStopReason::Refusal),
        StopReason::Cancelled => Ok(AcpStopReason::Cancelled),
        _ => Err(StopReasonProjectionError),
    }
}

/// A native stop reason has no stable-v1 [`AcpStopReason`] counterpart.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct StopReasonProjectionError;

/// Terminal summary of one bounded ACP prompt turn.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcpTurnOutcome {
    session_id: AcpSessionId,
    stop_reason: AcpStopReason,
    update_count: u32,
    cancellation_requested: bool,
}

impl AcpTurnOutcome {
    pub(crate) fn new(
        session_id: AcpSessionId,
        stop_reason: AcpStopReason,
        update_count: u32,
        cancellation_requested: bool,
    ) -> Self {
        Self {
            session_id,
            stop_reason,
            update_count,
            cancellation_requested,
        }
    }

    /// Borrow the ACP-native session correlation.
    ///
    /// This value does not substitute for a Pale session, request, operation,
    /// event, generation, or receipt identity.
    #[must_use]
    pub const fn session_id(&self) -> &AcpSessionId {
        &self.session_id
    }

    /// Return the agent-reported terminal reason.
    #[must_use]
    pub const fn stop_reason(&self) -> AcpStopReason {
        self.stop_reason
    }

    /// Return the number of ordered updates delivered to the observer.
    #[must_use]
    pub const fn update_count(&self) -> u32 {
        self.update_count
    }

    /// Report whether `session/cancel` was sent before termination.
    #[must_use]
    pub const fn cancellation_requested(&self) -> bool {
        self.cancellation_requested
    }
}

/// Opaque ACP-native session correlation.
///
/// This newtype deliberately has no conversion into any Pale identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AcpSessionId(String);

impl AcpSessionId {
    pub(crate) fn from_native(value: impl ToString) -> Self {
        Self(value.to_string())
    }

    /// Borrow the native ACP session spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
