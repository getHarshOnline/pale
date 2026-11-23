//! Inspect one structural `Delegate` identity without acquiring resident authority.

use std::fmt;

use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1,
    ResidentRecordRefV1,
};

use crate::error::{ApiError, FailureSpecification};
use crate::resident_observation::{
    json_string, parse_canonical_record, verified_operation_handle_digest,
};
use crate::PaleApi;

/// Exact untrusted bytes and a claimed custody reference for one operation handle.
///
/// Construction performs no validation. Pass this value to
/// [`PaleApi::inspect_a2a_delegate_handle`] before using its contents at an
/// outward A2A boundary.
pub struct A2aDelegateHandleRequest {
    record_ref: String,
    canonical_handle: Vec<u8>,
}

impl A2aDelegateHandleRequest {
    /// Assemble one owned inspection request.
    #[must_use]
    pub fn new(record_ref: String, canonical_handle: Vec<u8>) -> Self {
        Self {
            record_ref,
            canonical_handle,
        }
    }

    /// Borrow the claimed immutable custody reference.
    #[must_use]
    pub fn record_ref(&self) -> &str {
        &self.record_ref
    }

    /// Borrow the claimed canonical `operation-handle.v1` bytes.
    #[must_use]
    pub fn canonical_handle(&self) -> &[u8] {
        &self.canonical_handle
    }
}

impl fmt::Debug for A2aDelegateHandleRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("A2aDelegateHandleRequest")
            .field("record_ref", &self.record_ref)
            .field("canonical_handle_byte_len", &self.canonical_handle.len())
            .finish()
    }
}

/// Structurally verified identity of one `Delegate` operation.
///
/// This SDK-owned projection proves exact canonical bytes, the selected public
/// contract, the handle's self-carried digest, and the `Delegate` operation
/// class. Its record reference has passed only public syntax validation; proving
/// that the reference retains these bytes requires resident custody. This type
/// does not verify the signature, grant caller or disclosure authority, recover
/// the operation, select an A2A route, or permit a send.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct A2aDelegateHandle {
    record_ref: String,
    operation_handle_id: String,
    operation_handle_sha256: String,
    canonical_handle: Vec<u8>,
}

impl A2aDelegateHandle {
    /// Borrow the immutable custody reference supplied by the caller.
    #[must_use]
    pub fn record_ref(&self) -> &str {
        &self.record_ref
    }

    /// Borrow the operation identity carried by the verified handle bytes.
    #[must_use]
    pub fn operation_handle_id(&self) -> &str {
        &self.operation_handle_id
    }

    /// Borrow the reproduced canonical operation-handle digest.
    #[must_use]
    pub fn operation_handle_sha256(&self) -> &str {
        &self.operation_handle_sha256
    }

    /// Borrow the exact canonical signed-handle bytes.
    #[must_use]
    pub fn canonical_handle(&self) -> &[u8] {
        &self.canonical_handle
    }
}

impl PaleApi {
    /// Inspect one exact `Delegate` operation handle for an outward A2A owner.
    ///
    /// This effect-free boundary gives an interface a typed structural identity
    /// plus a syntax-checked caller-supplied record reference, without exporting
    /// application records or adapter wire DTOs. A live delegation still requires
    /// authenticated resident attachment, current authority, production signing
    /// and anti-rollback custody, and a separately qualified HTTPS carrier.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the record reference is invalid, the bytes are
    /// absent, non-canonical, outside the selected contract, carry the wrong
    /// schema identity or digest, or describe an operation other than
    /// `Delegate`.
    pub fn inspect_a2a_delegate_handle(
        &self,
        request: A2aDelegateHandleRequest,
    ) -> Result<A2aDelegateHandle, ApiError> {
        inspect_delegate_handle(request)
            .map_err(|error| invalid_delegate_handle(&self.diagnostic_identity, error))
    }
}

#[derive(Debug, thiserror::Error)]
enum A2aDelegateHandleError {
    #[error("operation handle record reference is invalid")]
    RecordReference,
    #[error("operation handle is not an exact canonical contract record")]
    Contract,
    #[error("operation handle does not reproduce its claimed digest")]
    Digest,
    #[error("operation handle does not identify a Delegate operation")]
    OperationClass,
    #[error("operation handle identity is absent")]
    OperationIdentity,
}

fn inspect_delegate_handle(
    request: A2aDelegateHandleRequest,
) -> Result<A2aDelegateHandle, A2aDelegateHandleError> {
    ResidentRecordRefV1::parse(request.record_ref.clone())
        .map_err(|_| A2aDelegateHandleError::RecordReference)?;
    let handle = parse_canonical_record(
        &request.canonical_handle,
        "operation-handle.v1",
        "schema_sha256",
    )
    .map_err(|_| A2aDelegateHandleError::Contract)?;
    let operation_handle_sha256 = verified_operation_handle_digest(&handle)
        .ok_or(A2aDelegateHandleError::Digest)?
        .to_owned();
    if json_string(&handle, "operation_class") != Some("delegate") {
        return Err(A2aDelegateHandleError::OperationClass);
    }
    let operation_handle_id = json_string(&handle, "operation_handle_id")
        .ok_or(A2aDelegateHandleError::OperationIdentity)?
        .to_owned();
    Ok(A2aDelegateHandle {
        record_ref: request.record_ref,
        operation_handle_id,
        operation_handle_sha256,
        canonical_handle: request.canonical_handle,
    })
}

fn invalid_delegate_handle(
    identity: &pale_core::records::RecordSchemaIdentity,
    error: A2aDelegateHandleError,
) -> ApiError {
    ApiError::new_with_opaque_source(
        identity,
        error,
        FailureSpecification {
            code: "a2a_delegate_handle_invalid",
            stage: DiagnosticStageV1::InputAdmission,
            disposition: DiagnosticDispositionV1::InvalidInput,
            summary: "A2A delegate handle is not an exact canonical Delegate identity",
            subject: None,
            schema_coordinate: None,
            location: None,
            retry: DiagnosticRetryV1::AfterCorrection,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: vec![
                "No resident recovery, route selection, disclosure, or carrier activity started.",
                "Signature and current-authority verification remain with the resident attachment.",
            ],
            help: Vec::new(),
            facts: Vec::new(),
        },
    )
}

#[cfg(test)]
#[path = "../tests/internal/a2a.rs"]
mod tests;
