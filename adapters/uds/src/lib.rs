//! Bounded Unix-domain socket realization of Pale local IPC framing.
//!
//! This adapter implements `LocalIpcFramePort` from `pale-ports` and reuses
//! the protected reader of `pale-adapter-fs`; `pale-resident` and `pale-sdk`
//! build on it. It owns caller-selected pathname binding, deadline-aware local
//! connection mechanics, kernel peer-credential observation, and the framing
//! defined by [`pale_ports::LocalIpcFramePort`]. It treats every payload as
//! opaque bytes. The optional authenticated exchange proves possession of
//! independently selected connection keys; application admission still owns
//! sessions, permissions, replay of operations, readiness and effects.
//!
//! No endpoint pathname is selected here. Deployment composition supplies a
//! confined pathname and remains responsible for its directory permissions,
//! lifecycle, stale-entry handling, and removal.

mod authentication;
pub use authentication::{
    accept_authenticated_request, begin_authenticated_request, exchange_authenticated,
    AuthenticatedLocalClient, AuthenticatedLocalRequest, LocalAuthenticatedExchangeError,
    LocalAuthenticationError, LocalRequestDelivery, PendingLocalAuthentication,
    LOCAL_AUTHENTICATED_FRAME_BYTE_LIMIT, LOCAL_AUTHENTICATED_PAYLOAD_BYTE_LIMIT,
};

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod caller_credential;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod configuration;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix;

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use caller_credential::{FileCallerCredential, FileCallerCredentialError};
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use configuration::{EndpointConfigurationError, LocalEndpointConfiguration};
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use unix::{UnixSocketCarrier, UnixSocketListener};
