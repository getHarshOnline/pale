//! Listen on a local Unix-domain socket and hand each connection to the Pale
//! resident service.
//!
//! This crate builds on `pale-application`, `pale-adapter-uds` and `pale-ports`;
//! the CLI and agent-host tests use it as their resident. The caller supplies an
//! already assembled application service and an exact endpoint. This component
//! owns bounded connection readiness and delegates admission, operation work and
//! recovery to their existing application owners. It chooses no deployment,
//! endpoint-discovery policy, signer or trust anchor.

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix;

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use unix::{
    ConnectionDeadlines, ResidentConnectionAuthentication, ResidentHostError, ResidentHostTurn,
    ResidentLocalHost,
};
