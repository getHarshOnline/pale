//! Bounded private bytes describing the session basis consumed by one operation.

use pale_core::records::{ArtifactDigest, SessionAdmissionLeaseDigest};

use super::{ControlRecordCoordinate, OperationCustodyCandidateError};

/// Hard physical ceiling; callers may impose a smaller byte bound.
pub const MAX_OPERATION_SESSION_BASIS_BYTES: usize = 64 * 1024;

/// Exact operation-scoped private bytes, not a public record or session authority.
/// The application owns canonical encoding, content admission and checkpoint
/// membership. This type checks only finite bytes and physical association.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationSessionBasis {
    handle: ControlRecordCoordinate,
    request: ControlRecordCoordinate,
    session_lease_digest: SessionAdmissionLeaseDigest,
    canonical_basis_jcs: crate::ControlRecordBytes,
    basis_sha256: ArtifactDigest,
}

impl OperationSessionBasis {
    /// Retain bounded opaque bytes and compute their exact-byte hash.
    /// This constructor does not establish JSON canonicality or session admission.
    ///
    /// # Errors
    /// Rejects empty or oversized bytes and aliased request/handle references.
    pub fn new(
        handle: ControlRecordCoordinate,
        request: ControlRecordCoordinate,
        session_lease_digest: SessionAdmissionLeaseDigest,
        canonical_basis_jcs: Vec<u8>,
        maximum_bytes: usize,
    ) -> Result<Self, OperationCustodyCandidateError> {
        if canonical_basis_jcs.is_empty()
            || canonical_basis_jcs.len() > maximum_bytes.min(MAX_OPERATION_SESSION_BASIS_BYTES)
            || handle.record_ref() == request.record_ref()
        {
            return Err(OperationCustodyCandidateError::SessionBasis);
        }
        let basis_sha256 = ArtifactDigest::hash_bytes(&canonical_basis_jcs);
        Ok(Self {
            handle,
            request,
            session_lease_digest,
            canonical_basis_jcs: crate::ControlRecordBytes::new(canonical_basis_jcs)
                .map_err(|_| OperationCustodyCandidateError::SessionBasis)?,
            basis_sha256,
        })
    }

    /// Exact operation handle coordinate.
    #[must_use]
    pub const fn handle(&self) -> &ControlRecordCoordinate {
        &self.handle
    }

    /// Exact original request coordinate.
    #[must_use]
    pub const fn request(&self) -> &ControlRecordCoordinate {
        &self.request
    }

    /// Exact lease digest selected by the application session owner.
    #[must_use]
    pub const fn session_lease_digest(&self) -> &SessionAdmissionLeaseDigest {
        &self.session_lease_digest
    }

    /// Caller-produced canonical bytes, opaque to physical custody.
    #[must_use]
    pub fn canonical_basis_jcs(&self) -> &[u8] {
        self.canonical_basis_jcs.as_slice()
    }

    /// Digest computed over exactly the retained bytes.
    #[must_use]
    pub const fn basis_sha256(&self) -> &ArtifactDigest {
        &self.basis_sha256
    }
}
