//! Physical custody for exact session-admission lease bodies.
//!
//! The resident journal retains only a lease digest. This port retains the
//! corresponding canonical wire bytes outside that journal so cold recovery
//! can re-admit them through the application owner. Storage does not make the
//! bytes trusted, current, or executable authority.

use pale_core::records::{ArtifactDigest, SchemaDigest, SchemaId, SessionAdmissionLeaseDigest};

use crate::{AuthorityStoreCoordinate, ControlRecordBytes, ControlStoreError};

/// Result of retaining one exact immutable session-lease body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionLeaseBodyRetention {
    /// This call retained the body for the first time.
    Retained,
    /// The exact same body was already retained.
    ExactReplay,
}

/// Exact opaque session-lease body retained by physical custody.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionLeaseBody {
    store: AuthorityStoreCoordinate,
    digest: SessionAdmissionLeaseDigest,
    schema_id: SchemaId,
    schema_sha256: SchemaDigest,
    canonical_wire_jcs: ControlRecordBytes,
    private_admission_basis_sha256: Option<ArtifactDigest>,
    canonical_private_admission_basis_jcs: Option<ControlRecordBytes>,
}

impl SessionLeaseBody {
    /// Bind one canonical body to its store, digest, and schema coordinates.
    #[must_use]
    pub const fn new(
        store: AuthorityStoreCoordinate,
        digest: SessionAdmissionLeaseDigest,
        schema_id: SchemaId,
        schema_sha256: SchemaDigest,
        canonical_wire_jcs: ControlRecordBytes,
    ) -> Self {
        Self {
            store,
            digest,
            schema_id,
            schema_sha256,
            canonical_wire_jcs,
            private_admission_basis_sha256: None,
            canonical_private_admission_basis_jcs: None,
        }
    }

    /// Attach the private admission basis consumed when this lease was admitted.
    ///
    /// The basis is retained atomically with the lease body. It is not a public
    /// contract and grants no authority until the application decodes it and
    /// revalidates the lease against current owner state.
    #[must_use]
    pub fn with_private_admission_basis(mut self, canonical_jcs: ControlRecordBytes) -> Self {
        self.private_admission_basis_sha256 =
            Some(ArtifactDigest::hash_bytes(canonical_jcs.as_slice()));
        self.canonical_private_admission_basis_jcs = Some(canonical_jcs);
        self
    }

    /// Borrow the exact physical store incarnation.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Borrow the exact retained lease digest named by the journal.
    #[must_use]
    pub const fn digest(&self) -> &SessionAdmissionLeaseDigest {
        &self.digest
    }

    /// Borrow the selected session-lease schema identity.
    #[must_use]
    pub const fn schema_id(&self) -> &SchemaId {
        &self.schema_id
    }

    /// Borrow the exact selected session-lease schema digest.
    #[must_use]
    pub const fn schema_sha256(&self) -> &SchemaDigest {
        &self.schema_sha256
    }

    /// Borrow the complete retained canonical wire bytes.
    #[must_use]
    pub const fn canonical_wire_jcs(&self) -> &ControlRecordBytes {
        &self.canonical_wire_jcs
    }

    /// Borrow the digest of the private admission basis, when one was retained.
    #[must_use]
    pub const fn private_admission_basis_sha256(&self) -> Option<&ArtifactDigest> {
        self.private_admission_basis_sha256.as_ref()
    }

    /// Borrow the private admission-basis bytes, when one was retained.
    #[must_use]
    pub const fn canonical_private_admission_basis_jcs(&self) -> Option<&ControlRecordBytes> {
        self.canonical_private_admission_basis_jcs.as_ref()
    }
}

/// Bounded request for one retained session-lease body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionLeaseBodyRead {
    store: AuthorityStoreCoordinate,
    digest: SessionAdmissionLeaseDigest,
    maximum_body_bytes: usize,
}

impl SessionLeaseBodyRead {
    /// Bind one digest read to an explicit byte ceiling.
    ///
    /// Zero safely excludes every non-empty retained body.
    #[must_use]
    pub const fn new(
        store: AuthorityStoreCoordinate,
        digest: SessionAdmissionLeaseDigest,
        maximum_body_bytes: usize,
    ) -> Self {
        Self {
            store,
            digest,
            maximum_body_bytes,
        }
    }

    /// Borrow the exact physical store incarnation.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Borrow the requested immutable lease digest.
    #[must_use]
    pub const fn digest(&self) -> &SessionAdmissionLeaseDigest {
        &self.digest
    }

    /// Return the maximum retained body bytes the caller will admit.
    #[must_use]
    pub const fn maximum_body_bytes(&self) -> usize {
        self.maximum_body_bytes
    }
}

/// Physical retention and bounded retrieval of session-admission lease bytes.
pub trait SessionLeaseCustodyPort: Send + Sync {
    /// Retain one exact body once, or classify an exact replay.
    ///
    /// A digest collision with different metadata or bytes must return a
    /// [`crate::ControlStoreErrorKind::RecoveryRequired`] failure. A failed
    /// commit whose outcome cannot be proven must return
    /// [`crate::ControlStoreErrorKind::OutcomeUnknown`].
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] when physical custody cannot prove an
    /// exact retained body or exact replay.
    fn compare_and_retain(
        &self,
        body: &SessionLeaseBody,
    ) -> Result<SessionLeaseBodyRetention, ControlStoreError>;

    /// Read at most one lease and private basis without exceeding their shared bound.
    ///
    /// Absence is not permission to recreate historical authority. Oversized,
    /// malformed, or contradictory custody must fail with
    /// [`crate::ControlStoreErrorKind::RecoveryRequired`].
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] when bounded physical observation fails
    /// or retained custody is contradictory.
    fn read(
        &self,
        request: &SessionLeaseBodyRead,
    ) -> Result<Option<SessionLeaseBody>, ControlStoreError>;
}
