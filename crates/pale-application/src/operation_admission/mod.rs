//! Decide whether an agent's tool call may run: check the route, the operation's
//! contract, its capability bindings and the caller's authorization.
//!
//! This module builds on `bounded_text`, `hooks`, `host_tool`, `resident`,
//! `session_state` and `terminal_adapters`; `decision_evidence`, `failure`, `hooks`
//! and `resident` build on it.
//!
//! Prompt prose remains model-owned. This module verifies the sealed route,
//! catalog, selected contract, capability bindings, and bounded authorization
//! identities without interpreting Markdown.

mod activation;
mod denial;
mod error;
mod invocation;
mod proof;
mod tool;
mod verify;

pub use activation::{parse_authorization_values, OperationActivation, VerifiedOperationSelection};
pub use denial::DenialEncodingError;
pub use error::OperationAdmissionError;
pub use invocation::{ObservedToolInvocation, ToolInvocation};
pub(crate) use invocation::{ToolInvocationDigest, ToolUseIdentityDigest};
pub use proof::{ActivationReadyOperation, AdmittedOperation, VerifiedOperationBundle};
pub use tool::{
    AdmittedToolCall, CompletedToolCall, ConfirmationRegistrationState, ConfirmationRevocation,
    ExactToolAuthorization, RecordedToolFailure, ToolConfirmationRequest,
};
