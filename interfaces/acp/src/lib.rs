//! Conduct a coding-agent turn over a caller-supplied stable ACP v1 connection.
//!
//! This crate builds on [`pale_sdk`]; the embedding application starts the agent
//! and calls this crate. It supervises that one connection and owns protocol
//! negotiation and session-turn mechanics only; Pale semantics remain in
//! [`pale_sdk`], while the connected agent remains the native owner of its
//! session updates and tool-call requests.

mod cancellation;
mod conductor;
mod error;
mod framing;
mod protocol;

pub use cancellation::AcpCancellation;
pub use conductor::AcpConductor;
pub use error::{AcpError, AcpErrorKind};
pub use protocol::{
    AcpSessionEvent, AcpSessionId, AcpSessionState, AcpStopReason, AcpStructuredUpdate,
    AcpTransitionError, AcpTurnOutcome, AcpTurnRequest, AcpUpdate, AcpUpdateKind,
    ACP_INCOMING_FRAME_BYTE_LIMIT, ACP_PROMPT_BYTE_LIMIT, ACP_TURN_INBOUND_BYTE_LIMIT,
    ACP_TURN_INBOUND_EVENT_LIMIT, ACP_TURN_UPDATE_LIMIT, ACP_UPDATE_TEXT_BYTE_LIMIT,
};

/// Stable ACP wire protocol selected by this interface.
pub const ACP_PROTOCOL_VERSION: u16 = 1;
