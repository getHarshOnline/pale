//! Canonical diagnostic projection for ACP boundary failures.

use std::error::Error;
use std::fmt;

use pale_sdk::{
    DiagnosticDisposition, DiagnosticEffectCertainty, DiagnosticProjection, DiagnosticProjector,
    DiagnosticRetry, DiagnosticStage, DiagnosticV1,
};

use crate::{AcpSessionId, AcpSessionState};

/// Stable machine-readable category for one failed ACP client turn.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcpErrorKind {
    /// The caller supplied an invalid working-directory or prompt value.
    InvalidRequest,
    /// Cancellation was already requested before a prompt could start.
    CancelledBeforePrompt,
    /// The peer selected a protocol version other than stable ACP v1.
    UnsupportedProtocolVersion,
    /// The peer exceeded the bounded update or text ceiling.
    UpdateLimitExceeded,
    /// The peer exceeded the aggregate inbound event ceiling.
    InboundEventLimitExceeded,
    /// The peer exceeded the aggregate inbound byte ceiling.
    InboundByteLimitExceeded,
    /// The caller-supplied turn deadline elapsed before terminal confirmation.
    DeadlineExceeded,
    /// One newline-delimited JSON-RPC frame exceeded the byte ceiling.
    FrameLimitExceeded,
    /// The pinned stable-v1 SDK yielded a message this leaf cannot interpret.
    UnsupportedStableV1Message,
    /// The byte carrier or protocol peer failed before a terminal response.
    Transport,
}

/// One ACP boundary failure carrying the canonical Pale diagnostic.
pub struct AcpError {
    kind: AcpErrorKind,
    state: AcpSessionState,
    session_id: Option<AcpSessionId>,
    diagnostic: Box<DiagnosticV1>,
    source: Option<Box<dyn Error + Send + Sync>>,
}

impl AcpError {
    pub(crate) fn new(
        diagnostics: &DiagnosticProjector,
        kind: AcpErrorKind,
        state: AcpSessionState,
        session_id: Option<AcpSessionId>,
        summary: &'static str,
        source: Option<Box<dyn Error + Send + Sync>>,
    ) -> Self {
        let (stage, disposition, retry) = match kind {
            AcpErrorKind::InvalidRequest | AcpErrorKind::UnsupportedProtocolVersion => (
                DiagnosticStage::InputAdmission,
                DiagnosticDisposition::InvalidInput,
                DiagnosticRetry::AfterCorrection,
            ),
            AcpErrorKind::CancelledBeforePrompt
            | AcpErrorKind::UpdateLimitExceeded
            | AcpErrorKind::InboundEventLimitExceeded
            | AcpErrorKind::InboundByteLimitExceeded => (
                DiagnosticStage::InputAdmission,
                DiagnosticDisposition::Blocked,
                DiagnosticRetry::AfterStateChange,
            ),
            AcpErrorKind::FrameLimitExceeded => (
                DiagnosticStage::InputAdmission,
                DiagnosticDisposition::InvalidInput,
                DiagnosticRetry::AfterCorrection,
            ),
            AcpErrorKind::DeadlineExceeded | AcpErrorKind::Transport => (
                DiagnosticStage::HostObservation,
                DiagnosticDisposition::OperationalFailure,
                DiagnosticRetry::AfterStateChange,
            ),
            AcpErrorKind::UnsupportedStableV1Message => (
                DiagnosticStage::Internal,
                DiagnosticDisposition::InternalFailure,
                DiagnosticRetry::NotIndicated,
            ),
        };
        let mut facts = vec![("acp_session_state".to_owned(), state.as_str().to_owned())];
        if let Some(session_id) = session_id.as_ref() {
            facts.push(("acp_session_id".to_owned(), session_id.as_str().to_owned()));
        }
        let projection = DiagnosticProjection::new(
            kind.code().to_owned(),
            stage,
            disposition,
            summary.to_owned(),
            retry,
            DiagnosticEffectCertainty::NotApplicable,
        )
        .with_facts(facts);
        match diagnostics.project(projection) {
            Ok(diagnostic) => Self {
                kind,
                state,
                session_id,
                diagnostic: Box::new(diagnostic),
                source,
            },
            Err(api_error) => Self {
                kind,
                state,
                session_id,
                diagnostic: Box::new(api_error.diagnostic().clone()),
                source: Some(Box::new(api_error)),
            },
        }
    }

    /// Return the stable ACP failure category.
    #[must_use]
    pub const fn kind(&self) -> AcpErrorKind {
        self.kind
    }

    /// Return the closed state classification assigned to this failure.
    ///
    /// [`AcpSessionState::TransportLost`] classifies carrier loss or
    /// caller-deadline abandonment; it is not a claim that the peer observed
    /// every preceding transition.
    #[must_use]
    pub const fn state(&self) -> AcpSessionState {
        self.state
    }

    /// Borrow the native session correlation when session creation completed.
    #[must_use]
    pub const fn session_id(&self) -> Option<&AcpSessionId> {
        self.session_id.as_ref()
    }

    /// Borrow the canonical Pale diagnostic.
    #[must_use]
    pub fn diagnostic(&self) -> &DiagnosticV1 {
        self.diagnostic.as_ref()
    }
}

impl AcpErrorKind {
    const fn code(self) -> &'static str {
        match self {
            Self::InvalidRequest => "acp_invalid_request",
            Self::CancelledBeforePrompt => "acp_cancelled_before_prompt",
            Self::UnsupportedProtocolVersion => "acp_unsupported_protocol_version",
            Self::UpdateLimitExceeded => "acp_update_limit_exceeded",
            Self::InboundEventLimitExceeded => "acp_inbound_event_limit_exceeded",
            Self::InboundByteLimitExceeded => "acp_inbound_byte_limit_exceeded",
            Self::DeadlineExceeded => "acp_deadline_exceeded",
            Self::FrameLimitExceeded => "acp_frame_limit_exceeded",
            Self::UnsupportedStableV1Message => "acp_unsupported_stable_v1_message",
            Self::Transport => "acp_transport_failure",
        }
    }
}

impl AcpSessionState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Disconnected => "disconnected",
            Self::InitializedV1 => "initialized_v1",
            Self::SessionOpen => "session_open",
            Self::PromptActive => "prompt_active",
            Self::CancellationRequested => "cancellation_requested",
            Self::TurnEnded => "turn_ended",
            Self::TransportLost => "transport_lost",
        }
    }
}

impl fmt::Debug for AcpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AcpError")
            .field("kind", &self.kind)
            .field("state", &self.state)
            .field("session_id", &self.session_id)
            .field("diagnostic", &self.diagnostic.identity().as_str())
            .field("source", &self.source.as_ref().map(|_| "<preserved>"))
            .finish()
    }
}

impl fmt::Display for AcpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.diagnostic.summary())
    }
}

impl Error for AcpError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn Error + 'static))
    }
}
