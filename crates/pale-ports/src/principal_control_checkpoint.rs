//! Physical recovery evidence for a principal control-checkpoint chain.
//!
//! These values preserve exact bounded rows from the admitted bootstrap
//! checkpoint through one selected physical head. They do not admit checkpoint
//! semantics, signer authority, profile content, accumulator construction, or
//! an independent anti-rollback minimum.

use pale_core::records::ControlTransactionSequenceV1;

use crate::{ControlCheckpointHead, ControlRecord, ControlStoreError, PrincipalTrustStoreBinding};

/// Copy limits for one principal control-checkpoint chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrincipalControlCheckpointChainReadLimits {
    maximum_checkpoints: usize,
    maximum_record_bytes: usize,
}

impl PrincipalControlCheckpointChainReadLimits {
    /// Set checkpoint-count and aggregate record-body ceilings.
    #[must_use]
    pub const fn new(maximum_checkpoints: usize, maximum_record_bytes: usize) -> Self {
        Self {
            maximum_checkpoints,
            maximum_record_bytes,
        }
    }
    /// Return the maximum number of copied checkpoint records.
    #[must_use]
    pub const fn maximum_checkpoints(self) -> usize {
        self.maximum_checkpoints
    }
    /// Return the maximum combined bytes copied for checkpoint records.
    #[must_use]
    pub const fn maximum_record_bytes(self) -> usize {
        self.maximum_record_bytes
    }
}

/// One opaque control checkpoint and its physical insertion transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalControlCheckpointStoredRecord {
    inserted_in: ControlTransactionSequenceV1,
    record: ControlRecord,
}

impl PrincipalControlCheckpointStoredRecord {
    /// Retain one immutable row without admitting its checkpoint meaning.
    #[must_use]
    pub const fn new(inserted_in: ControlTransactionSequenceV1, record: ControlRecord) -> Self {
        Self {
            inserted_in,
            record,
        }
    }
    /// Return the store transaction that inserted the row.
    #[must_use]
    pub const fn inserted_in(&self) -> ControlTransactionSequenceV1 {
        self.inserted_in
    }
    /// Borrow the exact stored record.
    #[must_use]
    pub const fn record(&self) -> &ControlRecord {
        &self.record
    }
}

/// Bounded physical evidence for one principal control-checkpoint domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalControlCheckpointChainSnapshot {
    binding: PrincipalTrustStoreBinding,
    initial: ControlCheckpointHead,
    through: ControlCheckpointHead,
    observed_through: ControlTransactionSequenceV1,
    records: Box<[PrincipalControlCheckpointStoredRecord]>,
}

impl PrincipalControlCheckpointChainSnapshot {
    /// Close physical evidence around exact initial and current coordinates.
    ///
    /// # Errors
    ///
    /// Rejects an unlike domain or epoch, a non-initial bootstrap head, an
    /// empty set, another record family, repeated coordinates, future rows, or
    /// missing exact initial/current records.
    pub fn new(
        binding: PrincipalTrustStoreBinding,
        initial: ControlCheckpointHead,
        through: ControlCheckpointHead,
        observed_through: ControlTransactionSequenceV1,
        records: Box<[PrincipalControlCheckpointStoredRecord]>,
    ) -> Result<Self, PrincipalControlCheckpointChainSnapshotError> {
        if initial.domain_sequence().get() != 1
            || initial.checkpoint_domain_id() != through.checkpoint_domain_id()
            || initial.domain_epoch() != through.domain_epoch()
            || initial.domain_sequence() > through.domain_sequence()
            || initial.committed_transaction_sequence() > observed_through
            || through.committed_transaction_sequence() > observed_through
            || (initial.domain_sequence() != through.domain_sequence()
                && initial.coordinate() == through.coordinate())
        {
            return Err(PrincipalControlCheckpointChainSnapshotError::HeadRelation);
        }
        if records.is_empty() {
            return Err(PrincipalControlCheckpointChainSnapshotError::Empty);
        }
        let mut coordinates = std::collections::BTreeSet::new();
        let mut has_initial = false;
        let mut has_current = false;
        for retained in &records {
            if retained.inserted_in() > observed_through {
                return Err(PrincipalControlCheckpointChainSnapshotError::FutureRecord);
            }
            let record = retained.record();
            if record.schema_id().as_str() != pale_core::records::CONTROL_STATE_CHECKPOINT_V1_SCHEMA
            {
                return Err(PrincipalControlCheckpointChainSnapshotError::RecordFamily);
            }
            if !coordinates.insert(record.coordinate().clone()) {
                return Err(PrincipalControlCheckpointChainSnapshotError::RepeatedCoordinate);
            }
            has_initial |= record.coordinate() == initial.coordinate();
            has_current |= record.coordinate() == through.coordinate();
        }
        if !has_initial {
            return Err(PrincipalControlCheckpointChainSnapshotError::InitialRecord);
        }
        if !has_current {
            return Err(PrincipalControlCheckpointChainSnapshotError::CurrentRecord);
        }
        Ok(Self {
            binding,
            initial,
            through,
            observed_through,
            records,
        })
    }

    /// Borrow the exact principal trust-store incarnation read by the adapter.
    #[must_use]
    pub const fn binding(&self) -> &PrincipalTrustStoreBinding {
        &self.binding
    }
    /// Borrow the already admitted bootstrap checkpoint coordinate.
    #[must_use]
    pub const fn initial(&self) -> &ControlCheckpointHead {
        &self.initial
    }
    /// Borrow the selected physical current head.
    #[must_use]
    pub const fn through(&self) -> &ControlCheckpointHead {
        &self.through
    }
    /// Return the store transaction observed by the physical read.
    #[must_use]
    pub const fn observed_through(&self) -> ControlTransactionSequenceV1 {
        self.observed_through
    }
    /// Borrow stored rows without assigning semantic chain order.
    #[must_use]
    pub const fn records(&self) -> &[PrincipalControlCheckpointStoredRecord] {
        &self.records
    }
}

/// A physical control-checkpoint snapshot is incomplete or contradictory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PrincipalControlCheckpointChainSnapshotError {
    /// Initial and current heads do not describe one domain epoch.
    #[error("control checkpoint heads do not describe one bounded physical chain")]
    HeadRelation,
    /// No stored checkpoint row was supplied.
    #[error("control checkpoint chain is empty")]
    Empty,
    /// A row belongs to another record family.
    #[error("control checkpoint chain contains another record family")]
    RecordFamily,
    /// Two rows use one immutable coordinate.
    #[error("control checkpoint chain repeats a record coordinate")]
    RepeatedCoordinate,
    /// A row was inserted after the observed store transaction.
    #[error("control checkpoint chain contains a future record")]
    FutureRecord,
    /// The admitted bootstrap checkpoint row is absent.
    #[error("control checkpoint chain does not contain its initial record")]
    InitialRecord,
    /// The selected current checkpoint row is absent.
    #[error("control checkpoint chain does not contain its current record")]
    CurrentRecord,
}

/// Read bounded physical evidence for one principal control-checkpoint chain.
pub trait PrincipalControlCheckpointChainReadPort {
    /// Copy the selected domain from its admitted bootstrap checkpoint through
    /// the physical current head.
    ///
    /// `None` means neither a head nor a checkpoint record exists for this
    /// domain. A head without rows, rows without a head, another epoch, or
    /// conflicting physical evidence requires recovery. The adapter does not
    /// admit checkpoint semantics or reconstruct accumulator mutations.
    ///
    /// # Errors
    ///
    /// Returns a control-store error for partial or conflicting evidence, an
    /// aggregate limit breach, or a native read failure.
    fn read_control_checkpoint_chain(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        initial: &ControlCheckpointHead,
        limits: PrincipalControlCheckpointChainReadLimits,
    ) -> Result<Option<PrincipalControlCheckpointChainSnapshot>, ControlStoreError>;
}

#[cfg(test)]
#[path = "../tests/internal/principal_control_checkpoint.rs"]
mod tests;
