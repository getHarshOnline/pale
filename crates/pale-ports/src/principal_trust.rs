//! Principal-specific bindings and mutable trust-frontier coordinates.
//!
//! Neutral physical records and domain checkpoints live in
//! [`crate::ControlRecord`] and [`crate::ControlCheckpointHead`]. This module
//! retains only the principal identity and succession semantics that must not
//! be erased at the physical store boundary.

use pale_core::records::{
    ControlIncarnationIdV1, ControlStoreIdV1, ControlTransactionSequenceV1,
    PrincipalTrustCheckpointEpochV1, ResidentPrincipalIdV1,
};

use crate::{ControlRecord, ControlRecordCoordinate, ControlStoreError};

/// Copy limits for one principal trust checkpoint-set snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrincipalTrustCheckpointSetReadLimits {
    maximum_checkpoints: usize,
    maximum_record_bytes: usize,
}

impl PrincipalTrustCheckpointSetReadLimits {
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

    /// Return the maximum combined body bytes owned by the returned set.
    #[must_use]
    pub const fn maximum_record_bytes(self) -> usize {
        self.maximum_record_bytes
    }
}

/// Exact mutable head of the principal-wide signed trust frontier.
///
/// The signed checkpoint remains an immutable [`crate::ControlRecord`]. This
/// projection retains only the coordinates required for atomic physical
/// compare-and-set beside an affected control-domain head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalTrustCheckpointHead {
    principal_id: ResidentPrincipalIdV1,
    checkpoint_epoch: PrincipalTrustCheckpointEpochV1,
    coordinate: ControlRecordCoordinate,
}

impl PrincipalTrustCheckpointHead {
    /// Bind one admitted trust checkpoint to its physical principal frontier.
    #[must_use]
    pub const fn new(
        principal_id: ResidentPrincipalIdV1,
        checkpoint_epoch: PrincipalTrustCheckpointEpochV1,
        coordinate: ControlRecordCoordinate,
    ) -> Self {
        Self {
            principal_id,
            checkpoint_epoch,
            coordinate,
        }
    }

    /// Borrow the principal whose complete trust frontier is represented.
    #[must_use]
    pub const fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.principal_id
    }

    /// Return the positive principal-trust succession epoch.
    #[must_use]
    pub const fn checkpoint_epoch(&self) -> PrincipalTrustCheckpointEpochV1 {
        self.checkpoint_epoch
    }

    /// Borrow the immutable signed-checkpoint coordinate.
    #[must_use]
    pub const fn coordinate(&self) -> &ControlRecordCoordinate {
        &self.coordinate
    }

    /// Test the exact monotone relation from `predecessor` to this head.
    ///
    /// A successor preserves the principal, advances the trust epoch exactly
    /// once, and names a different immutable checkpoint coordinate.
    #[must_use]
    pub fn is_exact_successor_of(&self, predecessor: &Self) -> bool {
        predecessor.principal_id == self.principal_id
            && predecessor.checkpoint_epoch.checked_successor() == Some(self.checkpoint_epoch)
            && predecessor.coordinate != self.coordinate
    }
}

/// Exact physical store and principal binding required by one candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalTrustStoreBinding {
    trust_store_id: ControlStoreIdV1,
    trust_store_incarnation_id: ControlIncarnationIdV1,
    principal_id: ResidentPrincipalIdV1,
}

impl PrincipalTrustStoreBinding {
    /// Bind one principal to one exact trust-store incarnation.
    #[must_use]
    pub const fn new(
        trust_store_id: ControlStoreIdV1,
        trust_store_incarnation_id: ControlIncarnationIdV1,
        principal_id: ResidentPrincipalIdV1,
    ) -> Self {
        Self {
            trust_store_id,
            trust_store_incarnation_id,
            principal_id,
        }
    }

    /// Borrow the stable trust-store identity.
    #[must_use]
    pub const fn trust_store_id(&self) -> &ControlStoreIdV1 {
        &self.trust_store_id
    }

    /// Borrow the exact trust-store incarnation.
    #[must_use]
    pub const fn trust_store_incarnation_id(&self) -> &ControlIncarnationIdV1 {
        &self.trust_store_incarnation_id
    }

    /// Borrow the principal whose immutable records the store retains.
    #[must_use]
    pub const fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.principal_id
    }
}

/// One observed principal trust head and its exact immutable stored bytes.
///
/// Bounded trust discovery and producer recovery share this exact
/// store binding, signed trust-checkpoint head, and immutable checkpoint bytes.
/// A matching tuple proves only a stable physical observation boundary; record
/// decoding, signature admission, historical uniqueness, and root closure
/// remain application work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalTrustSnapshot {
    binding: PrincipalTrustStoreBinding,
    through: PrincipalTrustCheckpointHead,
    checkpoint_record: ControlRecord,
}

impl PrincipalTrustSnapshot {
    /// Bind one principal trust-store incarnation to its exact checkpoint bytes.
    ///
    /// # Errors
    ///
    /// Returns [`PrincipalTrustSnapshotError::PrincipalBinding`]
    /// when the checkpoint belongs to another principal, or
    /// [`PrincipalTrustSnapshotError::CheckpointRecord`] when the
    /// supplied record does not reproduce the checkpoint coordinate.
    pub fn new(
        binding: PrincipalTrustStoreBinding,
        through: PrincipalTrustCheckpointHead,
        checkpoint_record: ControlRecord,
    ) -> Result<Self, PrincipalTrustSnapshotError> {
        if binding.principal_id() != through.principal_id() {
            return Err(PrincipalTrustSnapshotError::PrincipalBinding);
        }
        if checkpoint_record.coordinate() != through.coordinate() {
            return Err(PrincipalTrustSnapshotError::CheckpointRecord);
        }
        Ok(Self {
            binding,
            through,
            checkpoint_record,
        })
    }

    /// Borrow the exact trust-store incarnation and principal binding.
    #[must_use]
    pub const fn binding(&self) -> &PrincipalTrustStoreBinding {
        &self.binding
    }

    /// Borrow the stored trust-checkpoint head fencing selected reads.
    #[must_use]
    pub const fn through(&self) -> &PrincipalTrustCheckpointHead {
        &self.through
    }

    /// Borrow the exact stored signed-checkpoint bytes fencing every read.
    #[must_use]
    pub const fn checkpoint_record(&self) -> &ControlRecord {
        &self.checkpoint_record
    }
}

/// A physical trust snapshot contradicts its exact supplied coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PrincipalTrustSnapshotError {
    /// The checkpoint and store binding name different principals.
    #[error("trust checkpoint does not match the principal binding")]
    PrincipalBinding,
    /// The record coordinate differs from the stored head coordinate.
    #[error("trust checkpoint record does not match its head")]
    CheckpointRecord,
}

/// One exact stored checkpoint and the transaction that inserted it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalTrustCheckpointRecord {
    inserted_in: ControlTransactionSequenceV1,
    record: ControlRecord,
}

impl PrincipalTrustCheckpointRecord {
    /// Retain one immutable checkpoint row with its positive insertion stamp.
    #[must_use]
    pub const fn new(inserted_in: ControlTransactionSequenceV1, record: ControlRecord) -> Self {
        Self {
            inserted_in,
            record,
        }
    }

    /// Return the store transaction that inserted this row.
    #[must_use]
    pub const fn inserted_in(&self) -> ControlTransactionSequenceV1 {
        self.inserted_in
    }

    /// Borrow the exact opaque checkpoint record.
    #[must_use]
    pub const fn record(&self) -> &ControlRecord {
        &self.record
    }
}

/// Exact stored checkpoint records observed through one current trust head.
///
/// The records are opaque and may be in physical insertion order. The value
/// proves only that one read transaction copied every stored trust-checkpoint
/// row visible through the selected head. It does not admit their schema digest,
/// signatures, lineage, roots, current authority, or an anti-rollback minimum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalTrustCheckpointSetSnapshot {
    binding: PrincipalTrustStoreBinding,
    through: PrincipalTrustCheckpointHead,
    current_record_index: usize,
    observed_through: ControlTransactionSequenceV1,
    checkpoint_records: Box<[PrincipalTrustCheckpointRecord]>,
}

impl PrincipalTrustCheckpointSetSnapshot {
    /// Close a physical checkpoint-set snapshot around its current head.
    ///
    /// # Errors
    ///
    /// Rejects another principal, an empty set, repeated coordinates, a
    /// non-checkpoint record, a future insertion stamp, or an absent current
    /// head record.
    pub fn new(
        binding: PrincipalTrustStoreBinding,
        through: PrincipalTrustCheckpointHead,
        observed_through: ControlTransactionSequenceV1,
        checkpoint_records: Box<[PrincipalTrustCheckpointRecord]>,
    ) -> Result<Self, PrincipalTrustCheckpointSetSnapshotError> {
        if binding.principal_id() != through.principal_id() {
            return Err(PrincipalTrustCheckpointSetSnapshotError::PrincipalBinding);
        }
        if checkpoint_records.is_empty() {
            return Err(PrincipalTrustCheckpointSetSnapshotError::Empty);
        }
        let mut coordinates = std::collections::BTreeSet::new();
        let mut current_record_index = None;
        for (index, retained) in checkpoint_records.iter().enumerate() {
            if retained.inserted_in() > observed_through {
                return Err(PrincipalTrustCheckpointSetSnapshotError::FutureRecord);
            }
            let record = retained.record();
            if record.schema_id().as_str()
                != pale_core::records::PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA
            {
                return Err(PrincipalTrustCheckpointSetSnapshotError::RecordFamily);
            }
            if !coordinates.insert(record.coordinate().clone()) {
                return Err(PrincipalTrustCheckpointSetSnapshotError::RepeatedCoordinate);
            }
            if record.coordinate() == through.coordinate()
                && current_record_index.replace(index).is_some()
            {
                return Err(PrincipalTrustCheckpointSetSnapshotError::CurrentRecord);
            }
        }
        let current_record_index =
            current_record_index.ok_or(PrincipalTrustCheckpointSetSnapshotError::CurrentRecord)?;
        Ok(Self {
            binding,
            through,
            current_record_index,
            observed_through,
            checkpoint_records,
        })
    }

    /// Borrow the exact trust-store incarnation and principal binding.
    #[must_use]
    pub const fn binding(&self) -> &PrincipalTrustStoreBinding {
        &self.binding
    }

    /// Borrow the selected physical current head.
    #[must_use]
    pub const fn through(&self) -> &PrincipalTrustCheckpointHead {
        &self.through
    }

    /// Borrow the selected head's record from the bounded checkpoint set.
    #[must_use]
    pub fn current_checkpoint_record(&self) -> &PrincipalTrustCheckpointRecord {
        &self.checkpoint_records[self.current_record_index]
    }

    /// Return the exact store transaction observed by the physical read.
    #[must_use]
    pub const fn observed_through(&self) -> ControlTransactionSequenceV1 {
        self.observed_through
    }

    /// Borrow all copied checkpoint rows without assigning semantic order.
    #[must_use]
    pub const fn checkpoint_records(&self) -> &[PrincipalTrustCheckpointRecord] {
        &self.checkpoint_records
    }
}

/// A physical checkpoint-set snapshot does not close around its current head.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PrincipalTrustCheckpointSetSnapshotError {
    /// The checkpoint and store binding name different principals.
    #[error("principal trust checkpoint set does not match the principal binding")]
    PrincipalBinding,
    /// No checkpoint record was supplied.
    #[error("principal trust checkpoint set is empty")]
    Empty,
    /// A row belongs to another record family.
    #[error("principal trust checkpoint set contains another record family")]
    RecordFamily,
    /// Two rows use the same immutable coordinate.
    #[error("principal trust checkpoint set repeats a checkpoint coordinate")]
    RepeatedCoordinate,
    /// A row claims an insertion after the observed store transaction.
    #[error("principal trust checkpoint set contains a future checkpoint record")]
    FutureRecord,
    /// The selected current head has no one exact matching row.
    #[error("principal trust checkpoint set does not contain the exact current record")]
    CurrentRecord,
}

/// Read the bounded physical checkpoint set for one principal.
pub trait PrincipalTrustCheckpointSetReadPort {
    /// Copy every stored trust-checkpoint row visible through one current head.
    ///
    /// `None` means no trust head and no trust-checkpoint row exist. Partial,
    /// conflicting, over-limit, or wrongly bound evidence requires recovery.
    /// The returned bytes remain unadmitted and do not establish freshness,
    /// current authority, anchor equality, or resident readiness.
    ///
    /// # Errors
    ///
    /// Returns a control-store error for an inconsistent physical set, an
    /// exceeded aggregate bound, or a native read failure.
    fn read_checkpoint_set(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        limits: PrincipalTrustCheckpointSetReadLimits,
    ) -> Result<Option<PrincipalTrustCheckpointSetSnapshot>, ControlStoreError>;
}

/// Discover the existing principal trust head without admitting its authority.
pub trait PrincipalTrustReadPort {
    /// Read the actual head and exact immutable record in one physical snapshot.
    ///
    /// `maximum_body_bytes` bounds the returned record body, not the database
    /// engine's own page reads or memory. `None` means the bound store has no
    /// trust head. Neither presence nor absence establishes current authority.
    /// A returned snapshot does not keep a read transaction open; later reads
    /// must verify its exact head again. Record decoding, schema and signature
    /// admission, lineage, roots and independent minima remain with their owners.
    ///
    /// # Errors
    /// Returns a control-store error for a wrong store binding, incomplete or
    /// malformed head/record, invalid transaction association, exceeded body
    /// limit, or a native read failure. Never repairs or initializes a head.
    fn read_current_checkpoint(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        maximum_body_bytes: usize,
    ) -> Result<Option<PrincipalTrustSnapshot>, ControlStoreError>;

    /// Read one exact immutable trust record without interpreting its bytes.
    ///
    /// `None` means the immutable reference is absent. Reuse of that reference
    /// with another digest, a wrong store binding, malformed metadata, or a
    /// record beyond the observed store transaction requires recovery. The
    /// copied body cannot exceed `maximum_body_bytes`.
    ///
    /// # Errors
    ///
    /// Returns a control-store error for conflicting physical state, an
    /// exceeded body limit, or a native read failure.
    fn read_record(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        coordinate: &ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<Option<ControlRecord>, ControlStoreError>;
}

/// Read-only preparation of a principal store's next authenticated checkpoint.
///
/// Preparation compares the physical store, transaction and current checkpoint
/// in one read snapshot. It retains no database lock and grants no write or
/// signing authority. Commitment must compare the same predecessor again.
pub trait PrincipalCheckpointPreparationPort: crate::ControlTransactionPort {
    /// Compute bounded successor roots and retain their physical tree changes.
    ///
    /// # Errors
    /// Rejects an initial-domain request, stale transaction or checkpoint,
    /// corrupt state, unsupported encoding, or exceeded work/output limits.
    fn prepare_accumulator(
        &mut self,
        request: &crate::OperationAccumulatorPreparation,
    ) -> Result<crate::PreparedOperationAccumulator, crate::OperationAccumulatorPreparationError>;
}

#[cfg(test)]
#[path = "../tests/internal/principal_trust.rs"]
mod tests;
