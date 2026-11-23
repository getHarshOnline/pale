//! Neutral physical custody boundary for one admitted principal bootstrap atom.
//!
//! The application projects already-admitted signed records into
//! [`PreparedPrincipalBootstrapAtom`]. A concrete store may compare and commit
//! that closed atom, but it cannot construct semantic trust, reinterpret record
//! bytes, advance an external anti-rollback anchor, or assemble Pale Runtime.
//!
//! The physical closure law is

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/principal-bootstrap-closure.display.mathml"
))]
//!
//! The seven fixed records are the principal, recovery policy, three initial
//! authorities, trust checkpoint, and bootstrap receipt. A failed commit whose
//! outcome is unknown must be resolved through [`PrincipalBootstrapCustodyPort::observe_exact`];
//! retrying the mutation cannot establish which atom owns the store.
use std::collections::BTreeSet;

use crate::{
    ControlRecord, ControlRecordCoordinate, ControlRecordError, ControlStoreError,
    PrincipalTrustCheckpointHead, PrincipalTrustStoreBinding,
};
use pale_core::records::{
    ControlCheckpointDomainIdV1, ControlDomainEpochV1, ControlDomainSequenceV1,
    ControlTransactionSequenceV1, PrincipalBootstrapIdV1, UnixTimeV1,
};

/// One ordered hard-domain checkpoint index row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalBootstrapControlCheckpoint {
    ordinal: u32,
    checkpoint_domain_id: ControlCheckpointDomainIdV1,
    domain_epoch: ControlDomainEpochV1,
    domain_sequence: ControlDomainSequenceV1,
    committed_transaction_sequence: ControlTransactionSequenceV1,
    coordinate: ControlRecordCoordinate,
}

impl PrincipalBootstrapControlCheckpoint {
    /// Construct one already-admitted checkpoint index member.
    ///
    /// # Errors
    ///
    /// Returns [`PrincipalBootstrapCandidateError::InvalidCheckpointOrdinal`]
    /// when `ordinal` is zero. Contiguity is checked by
    /// [`PreparedPrincipalBootstrapAtom::new`].
    pub fn new(
        ordinal: u32,
        checkpoint_domain_id: ControlCheckpointDomainIdV1,
        domain_epoch: ControlDomainEpochV1,
        domain_sequence: ControlDomainSequenceV1,
        committed_transaction_sequence: ControlTransactionSequenceV1,
        coordinate: ControlRecordCoordinate,
    ) -> Result<Self, PrincipalBootstrapCandidateError> {
        if ordinal == 0 {
            return Err(PrincipalBootstrapCandidateError::InvalidCheckpointOrdinal);
        }
        Ok(Self {
            ordinal,
            checkpoint_domain_id,
            domain_epoch,
            domain_sequence,
            committed_transaction_sequence,
            coordinate,
        })
    }

    /// Return the one-based canonical checkpoint ordinal.
    #[must_use]
    pub const fn ordinal(&self) -> u32 {
        self.ordinal
    }

    /// Borrow the exact hard-conflict domain.
    #[must_use]
    pub const fn checkpoint_domain_id(&self) -> &ControlCheckpointDomainIdV1 {
        &self.checkpoint_domain_id
    }

    /// Return the admitted domain epoch.
    #[must_use]
    pub const fn domain_epoch(&self) -> ControlDomainEpochV1 {
        self.domain_epoch
    }

    /// Return the admitted domain sequence.
    #[must_use]
    pub const fn domain_sequence(&self) -> ControlDomainSequenceV1 {
        self.domain_sequence
    }

    /// Return the separately admitted checkpoint's claimed store transaction.
    /// All bootstrap checkpoint members must bind the same actual commit.
    #[must_use]
    pub const fn committed_transaction_sequence(&self) -> ControlTransactionSequenceV1 {
        self.committed_transaction_sequence
    }

    /// Borrow the indexed immutable record coordinate.
    #[must_use]
    pub const fn coordinate(&self) -> &ControlRecordCoordinate {
        &self.coordinate
    }
}

/// Six non-checkpoint, non-frontier records closed by every bootstrap head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalBootstrapFixedRecords {
    principal_record: ControlRecordCoordinate,
    recovery_policy: ControlRecordCoordinate,
    admin_authority: ControlRecordCoordinate,
    producer_authority: ControlRecordCoordinate,
    checkpoint_authority: ControlRecordCoordinate,
    receipt: ControlRecordCoordinate,
}

impl PrincipalBootstrapFixedRecords {
    /// Group the six semantically distinct fixed coordinates without flattening them.
    #[must_use]
    pub const fn new(
        principal_record: ControlRecordCoordinate,
        recovery_policy: ControlRecordCoordinate,
        admin_authority: ControlRecordCoordinate,
        producer_authority: ControlRecordCoordinate,
        checkpoint_authority: ControlRecordCoordinate,
        receipt: ControlRecordCoordinate,
    ) -> Self {
        Self {
            principal_record,
            recovery_policy,
            admin_authority,
            producer_authority,
            checkpoint_authority,
            receipt,
        }
    }

    /// Borrow the principal-record coordinate.
    #[must_use]
    pub const fn principal_record(&self) -> &ControlRecordCoordinate {
        &self.principal_record
    }

    /// Borrow the recovery-policy coordinate.
    #[must_use]
    pub const fn recovery_policy(&self) -> &ControlRecordCoordinate {
        &self.recovery_policy
    }

    /// Borrow the initial local-admin-authority coordinate.
    #[must_use]
    pub const fn admin_authority(&self) -> &ControlRecordCoordinate {
        &self.admin_authority
    }

    /// Borrow the initial producer-authority coordinate.
    #[must_use]
    pub const fn producer_authority(&self) -> &ControlRecordCoordinate {
        &self.producer_authority
    }

    /// Borrow the initial checkpoint-authority coordinate.
    #[must_use]
    pub const fn checkpoint_authority(&self) -> &ControlRecordCoordinate {
        &self.checkpoint_authority
    }

    /// Borrow the root-signed bootstrap-receipt coordinate.
    #[must_use]
    pub const fn receipt(&self) -> &ControlRecordCoordinate {
        &self.receipt
    }

    /// Iterate the six fixed coordinates without erasing their owning fields.
    fn iter(&self) -> impl Iterator<Item = &ControlRecordCoordinate> {
        [
            &self.principal_record,
            &self.recovery_policy,
            &self.admin_authority,
            &self.producer_authority,
            &self.checkpoint_authority,
            &self.receipt,
        ]
        .into_iter()
    }
}

/// Singleton coordinate closing one complete physical bootstrap candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalBootstrapHead {
    bootstrap_id: PrincipalBootstrapIdV1,
    fixed_records: PrincipalBootstrapFixedRecords,
    trust_checkpoint: PrincipalTrustCheckpointHead,
    control_checkpoint_count: u32,
    logical_committed_at: UnixTimeV1,
}

impl PrincipalBootstrapHead {
    /// Bind the grouped fixed roots and variable checkpoint count.
    #[must_use]
    pub const fn new(
        bootstrap_id: PrincipalBootstrapIdV1,
        fixed_records: PrincipalBootstrapFixedRecords,
        trust_checkpoint: PrincipalTrustCheckpointHead,
        control_checkpoint_count: u32,
        logical_committed_at: UnixTimeV1,
    ) -> Self {
        Self {
            bootstrap_id,
            fixed_records,
            trust_checkpoint,
            control_checkpoint_count,
            logical_committed_at,
        }
    }

    /// Borrow the stable one-shot bootstrap identity.
    #[must_use]
    pub const fn bootstrap_id(&self) -> &PrincipalBootstrapIdV1 {
        &self.bootstrap_id
    }

    /// Borrow the principal-record coordinate.
    #[must_use]
    pub const fn principal_record(&self) -> &ControlRecordCoordinate {
        self.fixed_records.principal_record()
    }

    /// Borrow the recovery-policy coordinate.
    #[must_use]
    pub const fn recovery_policy(&self) -> &ControlRecordCoordinate {
        self.fixed_records.recovery_policy()
    }

    /// Borrow the initial local-admin-authority coordinate.
    #[must_use]
    pub const fn admin_authority(&self) -> &ControlRecordCoordinate {
        self.fixed_records.admin_authority()
    }

    /// Borrow the initial producer-authority coordinate.
    #[must_use]
    pub const fn producer_authority(&self) -> &ControlRecordCoordinate {
        self.fixed_records.producer_authority()
    }

    /// Borrow the initial checkpoint-authority coordinate.
    #[must_use]
    pub const fn checkpoint_authority(&self) -> &ControlRecordCoordinate {
        self.fixed_records.checkpoint_authority()
    }

    /// Borrow the epoch-one mutable principal-trust frontier.
    #[must_use]
    pub const fn trust_checkpoint(&self) -> &PrincipalTrustCheckpointHead {
        &self.trust_checkpoint
    }

    /// Borrow the root-signed bootstrap-receipt coordinate.
    #[must_use]
    pub const fn receipt(&self) -> &ControlRecordCoordinate {
        self.fixed_records.receipt()
    }

    /// Return the declared number of hard-domain checkpoints.
    #[must_use]
    pub const fn control_checkpoint_count(&self) -> u32 {
        self.control_checkpoint_count
    }

    /// Return the root-signed logical commit coordinate.
    #[must_use]
    pub const fn logical_committed_at(&self) -> UnixTimeV1 {
        self.logical_committed_at
    }

    /// Iterate the seven fixed coordinates closed by this head.
    fn fixed_coordinates(&self) -> impl Iterator<Item = &ControlRecordCoordinate> {
        self.fixed_records
            .iter()
            .chain(std::iter::once(self.trust_checkpoint.coordinate()))
    }
}

/// Closed physical candidate produced from one admitted bootstrap atom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPrincipalBootstrapAtom {
    binding: PrincipalTrustStoreBinding,
    records: Vec<ControlRecord>,
    control_checkpoints: Vec<PrincipalBootstrapControlCheckpoint>,
    head: PrincipalBootstrapHead,
    accumulators: Vec<crate::OperationAccumulatorBatch>,
}

impl PreparedPrincipalBootstrapAtom {
    /// Number of non-checkpoint records in every initial-v1 bootstrap atom.
    pub const FIXED_RECORD_COUNT: u32 = 7;

    /// Validate record closure and ordered initial accumulator custody.
    ///
    /// # Errors
    ///
    /// Returns [`PrincipalBootstrapCandidateError`] when the atom is empty,
    /// contains duplicate or unreferenced records, contradicts its closing
    /// head, or does not provide exactly one matching initial accumulator batch
    /// per checkpoint. This is physical preparation only; it does not admit
    /// signed bytes.
    pub fn new(
        binding: PrincipalTrustStoreBinding,
        records: Vec<ControlRecord>,
        control_checkpoints: Vec<PrincipalBootstrapControlCheckpoint>,
        head: PrincipalBootstrapHead,
        accumulators: Vec<crate::OperationAccumulatorBatch>,
    ) -> Result<Self, PrincipalBootstrapCandidateError> {
        if control_checkpoints.is_empty()
            || usize::try_from(head.control_checkpoint_count).ok()
                != Some(control_checkpoints.len())
            || usize::try_from(Self::FIXED_RECORD_COUNT)
                .ok()
                .and_then(|fixed| fixed.checked_add(control_checkpoints.len()))
                != Some(records.len())
        {
            return Err(PrincipalBootstrapCandidateError::Cardinality);
        }
        if head.trust_checkpoint.principal_id() != binding.principal_id()
            || head.trust_checkpoint.checkpoint_epoch().get() != 1
        {
            return Err(PrincipalBootstrapCandidateError::InvalidTrustCheckpointHead);
        }
        if control_checkpoints
            .iter()
            .enumerate()
            .any(|(index, checkpoint)| u32::try_from(index + 1).ok() != Some(checkpoint.ordinal))
        {
            return Err(PrincipalBootstrapCandidateError::InvalidCheckpointOrdinal);
        }

        let record_refs = records
            .iter()
            .map(|record| record.coordinate().record_ref())
            .collect::<BTreeSet<_>>();
        let checkpoint_domains = control_checkpoints
            .iter()
            .map(PrincipalBootstrapControlCheckpoint::checkpoint_domain_id)
            .collect::<BTreeSet<_>>();
        if record_refs.len() != records.len()
            || checkpoint_domains.len() != control_checkpoints.len()
        {
            return Err(PrincipalBootstrapCandidateError::ReferenceClosure);
        }

        let observed = records
            .iter()
            .map(|record| record.coordinate().clone())
            .collect::<BTreeSet<_>>();
        let expected = head
            .fixed_coordinates()
            .cloned()
            .chain(
                control_checkpoints
                    .iter()
                    .map(|checkpoint| checkpoint.coordinate.clone()),
            )
            .collect::<BTreeSet<_>>();
        if observed.len() != records.len()
            || expected.len() != records.len()
            || observed != expected
        {
            return Err(PrincipalBootstrapCandidateError::ReferenceClosure);
        }
        let Some(first) = accumulators.first() else {
            return Err(PrincipalBootstrapCandidateError::AccumulatorClosure);
        };
        let transaction = first.binding().transaction();
        if accumulators.len() != control_checkpoints.len()
            || accumulators
                .iter()
                .zip(&control_checkpoints)
                .any(|(accumulator, checkpoint)| {
                    let accumulator_binding = accumulator.binding();
                    let successor = accumulator_binding.successor();
                    !matches!(
                        accumulator_binding.phase(),
                        crate::AccumulatorCheckpointPhase::Initial
                    ) || accumulator_binding.store().store_id() != binding.trust_store_id()
                        || accumulator_binding.store().incarnation_id()
                            != binding.trust_store_incarnation_id()
                        || accumulator_binding.transaction() != transaction
                        || successor.checkpoint_domain_id() != checkpoint.checkpoint_domain_id()
                        || successor.domain_epoch() != checkpoint.domain_epoch()
                        || successor.domain_sequence() != checkpoint.domain_sequence()
                        || successor.coordinate() != checkpoint.coordinate()
                        || successor.committed_transaction_sequence()
                            != checkpoint.committed_transaction_sequence()
                })
        {
            return Err(PrincipalBootstrapCandidateError::AccumulatorClosure);
        }
        Ok(Self {
            binding,
            records,
            control_checkpoints,
            head,
            accumulators,
        })
    }

    /// Borrow ordered initial tree custody.
    #[must_use]
    pub fn accumulators(&self) -> &[crate::OperationAccumulatorBatch] {
        &self.accumulators
    }

    /// Borrow the exact store/principal binding.
    #[must_use]
    pub const fn binding(&self) -> &PrincipalTrustStoreBinding {
        &self.binding
    }

    /// Borrow all canonical custody rows.
    #[must_use]
    pub fn records(&self) -> &[ControlRecord] {
        &self.records
    }

    /// Borrow the ordered hard-domain checkpoint index.
    #[must_use]
    pub fn control_checkpoints(&self) -> &[PrincipalBootstrapControlCheckpoint] {
        &self.control_checkpoints
    }

    /// Borrow the singleton closing coordinate.
    #[must_use]
    pub const fn head(&self) -> &PrincipalBootstrapHead {
        &self.head
    }
}

/// Failure to construct one physically closed bootstrap candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PrincipalBootstrapCandidateError {
    /// Attached tree custody does not close over the physical bootstrap atom.
    #[error("principal bootstrap accumulators do not match the ordered initial checkpoints")]
    AccumulatorClosure,
    /// A shared physical record coordinate or byte string is malformed.
    #[error(transparent)]
    Record(#[from] ControlRecordError),
    /// Record/checkpoint counts contradict the initial-v1 closure law.
    #[error("principal bootstrap record cardinality is not physically closed")]
    Cardinality,
    /// Checkpoint ordinals are not the contiguous one-based sequence.
    #[error("principal bootstrap checkpoint ordinals are not contiguous")]
    InvalidCheckpointOrdinal,
    /// The mutable trust frontier is not epoch one for the bound principal.
    #[error("principal bootstrap trust-checkpoint head is not the bound epoch-one frontier")]
    InvalidTrustCheckpointHead,
    /// Retained records are not uniquely and exactly closed by their references.
    #[error("principal bootstrap references do not close over retained records")]
    ReferenceClosure,
}

/// Successful physical compare-and-commit disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalBootstrapCommitDisposition {
    /// The empty store committed this candidate atom.
    New,
    /// The complete store already contained this exact atom byte-for-byte.
    ExactReplay,
}

/// Exact read-back classification after restart or an ambiguous commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalBootstrapDatabaseObservation {
    /// No bootstrap rows exist.
    Empty,
    /// The store binding and every candidate row match exactly.
    Exact,
    /// Partial rows, another complete candidate, or contradictory closure exists.
    ConflictingOrPartial,
}

/// Physical compare-and-set custody for one admitted principal bootstrap atom.
pub trait PrincipalBootstrapCustodyPort: crate::ControlTransactionPort {
    /// Prepare bounded initial trees in a read snapshot of the physical trust store.
    /// This produces no signature, semantic admission, or database mutation.
    ///
    /// # Errors
    ///
    /// Rejects an existing domain, a stale transaction, unsupported preparation,
    /// invalid stored nodes or work beyond the selected limits.
    fn prepare_accumulator(
        &mut self,
        request: &crate::OperationAccumulatorPreparation,
    ) -> Result<crate::PreparedOperationAccumulator, crate::OperationAccumulatorPreparationError>;

    /// Commit an empty store or prove exact byte-for-byte replay.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ControlStoreErrorKind::CandidateConflict`] for
    /// another complete candidate,
    /// [`crate::ControlStoreErrorKind::RecoveryRequired`] for partial or
    /// contradictory state, and
    /// [`crate::ControlStoreErrorKind::OutcomeUnknown`] whenever a failed
    /// commit cannot exclude success.
    fn compare_and_commit(
        &mut self,
        candidate: &PreparedPrincipalBootstrapAtom,
    ) -> Result<PrincipalBootstrapCommitDisposition, ControlStoreError>;

    /// Compare current physical state with `candidate` without mutating it.
    ///
    /// This is the mandatory recovery route after
    /// [`crate::ControlStoreErrorKind::OutcomeUnknown`]. Exact equality is
    /// physical evidence only; the returned value cannot admit stored bytes as
    /// semantic trust or prove external-anchor state.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] when the physical observation
    /// cannot be completed.
    fn observe_exact(
        &mut self,
        candidate: &PreparedPrincipalBootstrapAtom,
    ) -> Result<PrincipalBootstrapDatabaseObservation, ControlStoreError>;
}

#[cfg(test)]
#[path = "../tests/internal/principal_bootstrap.rs"]
mod tests;
