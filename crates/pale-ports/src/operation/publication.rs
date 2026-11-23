//! Bounded discovery of the committed sources from which Events can be derived.
//!
//! These records prove physical association only. The application must still
//! admit signatures, checkpoint membership, anchors and producer publication.

//! Enumeration does not establish whether an Event is pending or delivered.

use pale_core::records::{
    ControlTransactionSequenceV1, ResidentRecordRefV1, A2A_TASK_OBSERVATION_V1_SCHEMA,
    CONTROL_STATE_CHECKPOINT_V1_SCHEMA, OPERATION_OBSERVATION_V1_SCHEMA,
};

use crate::{AuthorityStoreCoordinate, ControlRecord, ControlStoreError, ControlStoreErrorKind};

/// Maximum source bodies returned in one page across both observation families.
/// Each family's indexed seek may inspect this many coordinates plus one look-ahead.
pub const MAX_OPERATION_PUBLICATION_SOURCES: usize = 128;
/// Maximum combined observation and checkpoint body bytes returned in one page.
pub const MAX_OPERATION_PUBLICATION_BODY_BYTES: usize = 8 * 1024 * 1024;

/// Position in an immutable source prefix of one exact store incarnation.
///
/// This is a private storage cursor, not an authenticated recipient token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationPublicationCursor {
    store: AuthorityStoreCoordinate,
    through: ControlTransactionSequenceV1,
    after_transaction: ControlTransactionSequenceV1,
    after_reference: ResidentRecordRefV1,
}

impl OperationPublicationCursor {
    /// Retain a source position at or below the captured transaction frontier.
    ///
    /// # Errors
    /// Rejects a position newer than the captured frontier. This does not prove
    /// that either position exists or that the caller may read this store.
    pub fn new(
        store: AuthorityStoreCoordinate,
        through: ControlTransactionSequenceV1,
        after_transaction: ControlTransactionSequenceV1,
        after_reference: ResidentRecordRefV1,
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
            after_reference,
        })
    }

    /// Borrow the store and incarnation whose source prefix is being read.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Return the inclusive transaction frontier retained by the first page.
    #[must_use]
    pub const fn through(&self) -> ControlTransactionSequenceV1 {
        self.through
    }

    /// Return the transaction of the last source in the preceding page.
    #[must_use]
    pub const fn after_transaction(&self) -> ControlTransactionSequenceV1 {
        self.after_transaction
    }

    /// Borrow the source reference used to break ties within a transaction.
    #[must_use]
    pub const fn after_reference(&self) -> &ResidentRecordRefV1 {
        &self.after_reference
    }
}

/// Physical source family, derived from the immutable observation's schema.
///
/// Remote task history does not establish a local operation result or acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationPublicationSourceKind {
    /// One local operation history revision.
    OperationObservation,
    /// One independently retained remote task history revision.
    A2aTaskObservation,
}

/// One local or remote observation and its committing checkpoint in one snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationPublicationSource {
    kind: OperationPublicationSourceKind,
    transaction: ControlTransactionSequenceV1,
    observation: ControlRecord,
    checkpoint: ControlRecord,
}

impl OperationPublicationSource {
    /// Retain the two physical records after their same-transaction read-back.
    ///
    /// # Errors
    /// Rejects incorrect record families. The adapter must establish the index
    /// association and insertion stamps; this constructor does not admit them.
    pub fn new(
        transaction: ControlTransactionSequenceV1,
        observation: ControlRecord,
        checkpoint: ControlRecord,
    ) -> Result<Self, ControlStoreError> {
        let kind = match observation.schema_id().as_str() {
            OPERATION_OBSERVATION_V1_SCHEMA => OperationPublicationSourceKind::OperationObservation,
            A2A_TASK_OBSERVATION_V1_SCHEMA => OperationPublicationSourceKind::A2aTaskObservation,
            _ => {
                return Err(ControlStoreError::new(
                    ControlStoreErrorKind::RecoveryRequired,
                ));
            }
        };
        if checkpoint.schema_id().as_str() != CONTROL_STATE_CHECKPOINT_V1_SCHEMA {
            return Err(ControlStoreError::new(
                ControlStoreErrorKind::RecoveryRequired,
            ));
        }
        Ok(Self {
            kind,
            transaction,
            observation,
            checkpoint,
        })
    }

    /// Return the closed source family without interpreting its lifecycle state.
    #[must_use]
    pub const fn kind(&self) -> OperationPublicationSourceKind {
        self.kind
    }

    /// Return the common physical insertion transaction.
    #[must_use]
    pub const fn transaction(&self) -> ControlTransactionSequenceV1 {
        self.transaction
    }

    /// Borrow the immutable observation bytes and their stored identity.
    #[must_use]
    pub const fn observation(&self) -> &ControlRecord {
        &self.observation
    }

    /// Borrow the checkpoint selected by the observation's immutable index.
    #[must_use]
    pub const fn checkpoint(&self) -> &ControlRecord {
        &self.checkpoint
    }
}

/// One bounded source page, with continuation only when another source exists.
#[derive(Debug)]
pub struct OperationPublicationSources {
    through: ControlTransactionSequenceV1,
    /// Sources in increasing transaction/reference order, not causal order.
    pub sources: Vec<OperationPublicationSource>,
    /// Retained prefix position; later commits are excluded from this traversal.
    pub continuation: Option<OperationPublicationCursor>,
}

impl OperationPublicationSources {
    /// Return the inclusive transaction frontier frozen by the first page.
    ///
    /// The value remains available when the page has no continuation. It lets
    /// the application describe the exact prefix it inspected without claiming
    /// that no newer transaction committed after the snapshot.
    #[must_use]
    pub const fn through(&self) -> ControlTransactionSequenceV1 {
        self.through
    }

    /// Construct one bounded page after the adapter fixed its snapshot frontier.
    ///
    /// This retains physical enumeration only. It does not admit any source or
    /// establish that a source has been published.
    #[must_use]
    pub fn new(
        through: ControlTransactionSequenceV1,
        sources: Vec<OperationPublicationSource>,
        continuation: Option<OperationPublicationCursor>,
    ) -> Self {
        Self {
            through,
            sources,
            continuation,
        }
    }
}

/// Failure to read a complete bounded source page.
#[derive(Debug, thiserror::Error)]
pub enum OperationPublicationReadError {
    /// The requested limits are invalid or the selected page exceeds its budget.
    #[error("operation publication source page exceeds its read limits")]
    Limit,
    /// Storage is unavailable, changed incarnation, rolled back or inconsistent.
    #[error(transparent)]
    Storage(#[from] ControlStoreError),
}
