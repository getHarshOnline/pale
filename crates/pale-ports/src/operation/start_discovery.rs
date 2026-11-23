//! Bounded physical discovery of committed operation starts.

use pale_core::records::{
    ArtifactDigest, ControlTransactionSequenceV1, SessionAdmissionLeaseDigest,
};

use crate::{
    AuthorityStoreCoordinate, ControlRecordCoordinate, ControlStoreError, ControlStoreErrorKind,
};

use super::OperationSessionBasis;

/// Maximum committed starts returned by one page.
pub const MAX_OPERATION_START_SOURCES: usize = 128;
/// Maximum combined session-basis bytes returned by one page.
pub const MAX_OPERATION_START_SOURCE_BYTES: usize = 8 * 1024 * 1024;

/// Position in one frozen prefix of committed operation starts.
///
/// This storage cursor does not grant access or admit the retained session basis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationStartCursor {
    store: AuthorityStoreCoordinate,
    through: ControlTransactionSequenceV1,
    after_transaction: ControlTransactionSequenceV1,
    handle: ControlRecordCoordinate,
    request: ControlRecordCoordinate,
    session_lease_digest: SessionAdmissionLeaseDigest,
    session_basis_digest: ArtifactDigest,
}

impl OperationStartCursor {
    /// Retain the exact last row and the first page's transaction frontier.
    ///
    /// # Errors
    /// Rejects a row position newer than the frozen frontier.
    pub fn new(
        store: AuthorityStoreCoordinate,
        through: ControlTransactionSequenceV1,
        after_transaction: ControlTransactionSequenceV1,
        handle: ControlRecordCoordinate,
        request: ControlRecordCoordinate,
        session_lease_digest: SessionAdmissionLeaseDigest,
        session_basis_digest: ArtifactDigest,
    ) -> Result<Self, ControlStoreError> {
        if after_transaction.get() > through.get() {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        }
        Ok(Self {
            store,
            through,
            after_transaction,
            handle,
            request,
            session_lease_digest,
            session_basis_digest,
        })
    }

    /// Borrow the exact store incarnation.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Return the inclusive transaction frontier frozen by the first page.
    #[must_use]
    pub const fn through(&self) -> ControlTransactionSequenceV1 {
        self.through
    }

    /// Return the insertion transaction of the prior page's last row.
    #[must_use]
    pub const fn after_transaction(&self) -> ControlTransactionSequenceV1 {
        self.after_transaction
    }

    /// Borrow the exact handle coordinate of the prior page's last row.
    #[must_use]
    pub const fn handle(&self) -> &ControlRecordCoordinate {
        &self.handle
    }

    /// Borrow the exact request coordinate of the prior page's last row.
    #[must_use]
    pub const fn request(&self) -> &ControlRecordCoordinate {
        &self.request
    }

    /// Borrow the retained session-lease digest.
    #[must_use]
    pub const fn session_lease_digest(&self) -> &SessionAdmissionLeaseDigest {
        &self.session_lease_digest
    }

    /// Borrow the digest of the retained session-basis bytes.
    #[must_use]
    pub const fn session_basis_digest(&self) -> &ArtifactDigest {
        &self.session_basis_digest
    }
}

/// One committed operation start and its opaque session-basis bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationStartSource {
    transaction: ControlTransactionSequenceV1,
    session_basis: OperationSessionBasis,
}

impl OperationStartSource {
    /// Retain a physically complete start source.
    #[must_use]
    pub const fn new(
        transaction: ControlTransactionSequenceV1,
        session_basis: OperationSessionBasis,
    ) -> Self {
        Self {
            transaction,
            session_basis,
        }
    }

    /// Return the transaction that committed the start atom.
    #[must_use]
    pub const fn transaction(&self) -> ControlTransactionSequenceV1 {
        self.transaction
    }

    /// Borrow the unadmitted session basis retained with the start.
    #[must_use]
    pub const fn session_basis(&self) -> &OperationSessionBasis {
        &self.session_basis
    }
}

/// One bounded page from a frozen committed-start prefix.
#[derive(Debug)]
pub struct OperationStartSources {
    through: ControlTransactionSequenceV1,
    /// Sources in increasing transaction and handle-reference order.
    pub sources: Vec<OperationStartSource>,
    /// Continuation only when one more indexed row exists in the frozen prefix.
    pub continuation: Option<OperationStartCursor>,
}

impl OperationStartSources {
    /// Construct a page after its physical checks have completed.
    #[must_use]
    pub fn new(
        through: ControlTransactionSequenceV1,
        sources: Vec<OperationStartSource>,
        continuation: Option<OperationStartCursor>,
    ) -> Self {
        Self {
            through,
            sources,
            continuation,
        }
    }

    /// Return the inclusive transaction frontier frozen by the first page.
    #[must_use]
    pub const fn through(&self) -> ControlTransactionSequenceV1 {
        self.through
    }
}

/// Failure to read one complete committed-start page.
#[derive(Debug, thiserror::Error)]
pub enum OperationStartReadError {
    /// The requested limits are invalid or the page exceeds its byte budget.
    #[error("operation start source page exceeds its read limits")]
    Limit,
    /// Storage is unavailable, changed, rolled back, or inconsistent.
    #[error(transparent)]
    Storage(#[from] ControlStoreError),
}
