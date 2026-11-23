//! Exchange bounded A2A evidence over one verified HTTPS route.
//!
//! This adapter implements the A2A exchange ports of `pale-ports` on top of
//! `pale-core`. The application defines how an exchange is used; so far only the
//! tests of `pale-application` and the control-store adapter use this adapter.
//!
//! The application owns provider selection, route admission, dispatch custody,
//! task binding and settlement. This adapter sends prepared request bytes and
//! returns untrusted native facts. It does not install, activate or publish a
//! service. The first input path carries text and no client credentials.

mod cancel_task_evidence;
mod decoding;
mod dns;
mod get_task_evidence;
mod method;
mod send_message_evidence;
mod service;
mod sse;
mod transport;
mod wire;

pub use cancel_task_evidence::A2aCancelTaskDecoder;
pub use get_task_evidence::A2aGetTaskDecoder;
pub use send_message_evidence::A2aSendMessageDecoder;
pub use service::{A2aService, A2aServiceLimits, A2aServiceState, A2aTlsTrust};

use decoding::{decode_response, A2aDecodeError, A2aResponse};
use method::A2aMethod;
use sse::{A2aEventDecoder, A2aEventLimits};

#[cfg(test)]
#[path = "../tests/internal/decoding.rs"]
mod decoding_tests;
mod task_evidence;
