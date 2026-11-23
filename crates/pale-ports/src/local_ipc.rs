//! Neutral framed local-IPC boundary.
//!
//! This port carries one bounded opaque byte string per frame. The carrier
//! establishes neither the bytes' schema nor the caller's identity, authority,
//! replay status, or semantic admission. A host authenticates and admits the
//! decoded request separately after observing [`LocalIpcPeerCredentials`].
//!
//! The selected framing law is intentionally small. For payload length `n` and
//! caller-selected ceiling `L`:

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/local-ipc-frame-bound.display.mathml"
))]
//! Each frame is the unsigned 32-bit big-endian encoding of `n`, followed by
//! exactly `n` opaque bytes. A timeout after partial progress or an oversized
//! inbound declaration leaves the stream out of frame alignment; the carrier
//! must then reject every subsequent operation on that connection.
use std::io;
use std::num::NonZeroU32;
use std::time::Instant;

use serde::{Deserialize, Serialize};

mod authentication;
pub use authentication::{LocalConnectionSignerPort, LocalConnectionSigningError};

mod resident_operation;
pub use resident_operation::{
    ResidentOperationFrameCancellationDisposition, ResidentOperationFrameFailure,
    ResidentOperationFrameLocator, ResidentOperationFrameRecords, ResidentOperationFrameRequest,
    ResidentOperationFrameResponse, ResidentOperationFrameUnsupportedReason,
};

/// Positive payload ceiling applied before allocation or transmission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalIpcFrameLimits {
    max_payload_bytes: NonZeroU32,
}

impl LocalIpcFrameLimits {
    /// Select a positive maximum opaque payload size.
    #[must_use]
    pub const fn new(max_payload_bytes: NonZeroU32) -> Self {
        Self { max_payload_bytes }
    }

    /// Return the maximum payload size, excluding the four-byte length prefix.
    #[must_use]
    pub const fn max_payload_bytes(self) -> NonZeroU32 {
        self.max_payload_bytes
    }
}

/// Operating-system credentials observed for one connected local peer.
///
/// These coordinates can contribute to an authentication decision, but they
/// are not a Pale principal, caller, role, capability, or session proof. In
/// particular, equal user identifiers do not authenticate two processes to one
/// another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalIpcPeerCredentials {
    user_id: u32,
    group_id: u32,
    process_id: Option<u32>,
}

impl LocalIpcPeerCredentials {
    /// Preserve the peer coordinates returned by the host kernel.
    #[must_use]
    pub const fn new(user_id: u32, group_id: u32, process_id: Option<u32>) -> Self {
        Self {
            user_id,
            group_id,
            process_id,
        }
    }

    /// Return the kernel-observed effective user identifier.
    #[must_use]
    pub const fn user_id(self) -> u32 {
        self.user_id
    }

    /// Return the kernel-observed effective group identifier.
    #[must_use]
    pub const fn group_id(self) -> u32 {
        self.group_id
    }

    /// Return the kernel-observed process identifier when the host exposes it.
    #[must_use]
    pub const fn process_id(self) -> Option<u32> {
        self.process_id
    }
}

/// Physical carrier operation associated with a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalIpcCarrierOperation {
    /// Bind a caller-selected local endpoint.
    Bind,
    /// Connect to a caller-selected local endpoint.
    Connect,
    /// Accept one incoming local connection.
    Accept,
    /// Query host credentials for a connected peer.
    ObservePeerCredentials,
    /// Receive one complete bounded frame.
    ReceiveFrame,
    /// Send one complete bounded frame.
    SendFrame,
}

/// Frame section that ended before its declared size was received.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalIpcFrameSection {
    /// Four-byte unsigned big-endian payload length.
    LengthPrefix,
    /// Opaque payload governed by the decoded length.
    Payload,
}

/// Typed failure at the framed local carrier boundary.
#[derive(Debug, thiserror::Error)]
pub enum LocalIpcCarrierError {
    /// A prior connection-fatal frame error made continued use unsafe.
    #[error("local IPC connection is unusable after a prior framing failure")]
    ConnectionUnusable,
    /// The absolute monotonic deadline elapsed before the operation completed.
    #[error(
        "local IPC {operation:?} deadline elapsed after {frame_bytes_transferred} frame bytes"
    )]
    DeadlineElapsed {
        /// Operation interrupted by the deadline.
        operation: LocalIpcCarrierOperation,
        /// Length-prefix plus payload bytes already transferred for this frame.
        frame_bytes_transferred: usize,
    },
    /// The peer closed cleanly before any byte of the next frame.
    #[error("local IPC peer closed before the next frame")]
    EndOfStream,
    /// The peer closed after beginning, but before completing, one frame.
    #[error(
        "local IPC frame ended inside {section:?}: expected {expected_bytes} bytes, received {received_bytes}"
    )]
    TruncatedFrame {
        /// Incomplete section of the frame.
        section: LocalIpcFrameSection,
        /// Bytes required by that section.
        expected_bytes: usize,
        /// Bytes received in that section before end-of-stream.
        received_bytes: usize,
    },
    /// A declared or supplied payload exceeds the selected connection ceiling.
    #[error(
        "local IPC {operation:?} payload length {declared_bytes} exceeds ceiling {limit_bytes}"
    )]
    FrameTooLarge {
        /// Send or receive operation that observed the violation.
        operation: LocalIpcCarrierOperation,
        /// Supplied or decoded payload length.
        declared_bytes: usize,
        /// Maximum payload length selected for this connection.
        limit_bytes: usize,
    },
    /// A bounded inbound payload could not be allocated.
    #[error("local IPC could not allocate declared payload of {declared_bytes} bytes")]
    FrameAllocationFailed {
        /// Validated inbound payload length whose allocation failed.
        declared_bytes: usize,
    },
    /// The host rejected or failed a physical carrier operation.
    #[error("local IPC {operation:?} failed after {frame_bytes_transferred} frame bytes")]
    Io {
        /// Physical operation attempted at the host boundary.
        operation: LocalIpcCarrierOperation,
        /// Length-prefix plus payload bytes already transferred for this frame.
        frame_bytes_transferred: usize,
        /// Original operating-system failure without endpoint or payload text.
        #[source]
        source: io::Error,
    },
}

impl LocalIpcCarrierError {
    /// Construct a redacted carrier error while preserving its typed OS source.
    #[must_use]
    pub fn io(operation: LocalIpcCarrierOperation, source: io::Error) -> Self {
        Self::io_after_progress(operation, 0, source)
    }

    /// Preserve partial frame progress with a redacted typed OS failure.
    #[must_use]
    pub fn io_after_progress(
        operation: LocalIpcCarrierOperation,
        frame_bytes_transferred: usize,
        source: io::Error,
    ) -> Self {
        Self::Io {
            operation,
            frame_bytes_transferred,
            source,
        }
    }

    /// Whether this failure makes every later operation on the connection unsafe.
    ///
    /// Preflight send rejection and a deadline before any frame byte crosses
    /// the carrier preserve alignment. Receiving an oversize declaration has
    /// already consumed its prefix and is therefore connection-fatal.
    #[must_use]
    pub const fn invalidates_connection(&self) -> bool {
        match self {
            Self::ConnectionUnusable
            | Self::EndOfStream
            | Self::TruncatedFrame { .. }
            | Self::FrameAllocationFailed { .. } => true,
            Self::Io { operation, .. } => matches!(
                operation,
                LocalIpcCarrierOperation::ReceiveFrame | LocalIpcCarrierOperation::SendFrame
            ),
            Self::DeadlineElapsed {
                frame_bytes_transferred,
                ..
            } => *frame_bytes_transferred != 0,
            Self::FrameTooLarge { operation, .. } => {
                matches!(*operation, LocalIpcCarrierOperation::ReceiveFrame)
            }
        }
    }
}

/// Connected opaque-frame transport used by resident application composition.
///
/// Implementations must apply their configured payload ceiling before sending
/// or allocating, perform no unbounded blocking beyond the supplied monotonic
/// deadline, preserve partial progress in [`LocalIpcCarrierError`], and become
/// permanently unusable after any error for which
/// [`LocalIpcCarrierError::invalidates_connection`] is true.
pub trait LocalIpcFramePort: Send {
    /// Return the caller key that authenticated this port's exact request frame.
    ///
    /// Plain carriers return `None`. An authentication adapter may return a key
    /// only after verifying the request bytes, and must keep that association
    /// through any prefetched frame wrapper. This proves a connection credential,
    /// not a principal, session, grant or operation permission.
    fn authenticated_caller_key(&self) -> Option<[u8; 32]> {
        None
    }

    /// Observe host credentials for the connected peer.
    ///
    /// # Errors
    ///
    /// Returns [`LocalIpcCarrierError::Io`] when the host cannot report peer
    /// credentials. A successful observation is still not authentication.
    fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError>;

    /// Send one length-prefixed opaque frame before `deadline`.
    ///
    /// # Errors
    ///
    /// Returns [`LocalIpcCarrierError::FrameTooLarge`] before writing when the
    /// payload exceeds the configured ceiling. Deadline and I/O failures may
    /// follow partial writes; a connection-fatal result makes the carrier
    /// reject every subsequent operation.
    fn send_frame(&mut self, payload: &[u8], deadline: Instant)
        -> Result<(), LocalIpcCarrierError>;

    /// Receive one complete length-prefixed opaque frame before `deadline`.
    ///
    /// # Errors
    ///
    /// Returns a typed clean EOF, truncation, oversize, allocation, deadline,
    /// or I/O failure. Oversize and partial-progress failures make the carrier
    /// reject every subsequent operation because its next boundary is unknown.
    fn receive_frame(&mut self, deadline: Instant) -> Result<Box<[u8]>, LocalIpcCarrierError>;
}

/// One read-only resident Observe request carried by a local frame.
///
/// This transport projection does not authenticate the caller, admit the
/// locator, authorize disclosure, or create a public contract family. The
/// application must admit the signed request before it reads resident custody.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ResidentObserveFrameRequest {
    operation_handle_record_ref: String,
    operation_handle_sha256: String,
    signed_common_request: String,
}

impl ResidentObserveFrameRequest {
    /// Preserve the exact locator and UTF-8 canonical request bytes.
    #[must_use]
    pub fn new(
        operation_handle_record_ref: String,
        operation_handle_sha256: String,
        signed_common_request: String,
    ) -> Self {
        Self {
            operation_handle_record_ref,
            operation_handle_sha256,
            signed_common_request,
        }
    }

    /// Borrow the immutable operation-handle storage reference.
    #[must_use]
    pub fn operation_handle_record_ref(&self) -> &str {
        &self.operation_handle_record_ref
    }

    /// Borrow the canonical operation-handle digest.
    #[must_use]
    pub fn operation_handle_sha256(&self) -> &str {
        &self.operation_handle_sha256
    }

    /// Borrow the exact pre-signed common request as UTF-8 canonical JSON.
    #[must_use]
    pub fn signed_common_request(&self) -> &str {
        &self.signed_common_request
    }
}

/// Stable transport-only failure classes for one resident Observe frame.
///
/// These classes do not replace application diagnostics. They let a framed
/// client distinguish malformed input, denied admission, replay conflict, and
/// resident failure without exposing private records or implementation text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResidentObserveFrameFailure {
    /// The frame or signed request is malformed, oversized, or misbound.
    InvalidRequest,
    /// The same request identity was presented with different signed bytes.
    ReplayConflict,
    /// Caller, session, current authority, or disclosure admission failed.
    AdmissionDenied,
    /// Protected recovery or resident observation did not complete.
    ResidentUnavailable,
}

/// One exact read-only resident Observe response carried by a local frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(
    deny_unknown_fields,
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ResidentObserveFrameResponse {
    /// Exact signed records returned by an authorized current-head observation.
    Observed {
        /// Immutable operation-handle storage reference.
        operation_handle_record_ref: String,
        /// Exact canonical `OperationHandleV1` JSON.
        operation_handle_canonical: String,
        /// Immutable current-observation storage reference.
        operation_observation_record_ref: String,
        /// Exact canonical `OperationObservationV1` JSON.
        operation_observation_canonical: String,
    },
    /// Bounded transport projection of an application-owned rejection.
    Rejected {
        /// Stable transport-only failure class.
        failure: ResidentObserveFrameFailure,
    },
}

#[cfg(test)]
#[path = "../tests/internal/local_ipc.rs"]
mod tests;
