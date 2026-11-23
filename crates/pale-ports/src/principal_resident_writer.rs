//! Pure physical closure for the clean resident-writer lineage.
//!
//! The application supplies already-admitted records, the exact predecessor,
//! and the intended successor. This module proves the finite clean-transition
//! shape and exposes one narrow custody port that atomically compares both the
//! writer and control-checkpoint heads. It grants no OS lock, liveness,
//! takeover, authentication, or runtime authority.

use std::collections::BTreeSet;

use pale_core::records::{
    PrincipalResidentWriterFenceIdV1, PrincipalResidentWriterFenceStateV1,
    PrincipalResidentWriterGenerationV1, ResidentPrincipalIdV1,
};

/// Held physical exclusion for one principal's resident writer.
///
/// This port exposes only the principal coordinate needed to join a native
/// guard to admitted writer evidence. Implementations retain their operating-
/// system resource for the value's lifetime. The guard does not prove a writer
/// fence, process health, database state, or runtime readiness.
pub trait PrincipalWriterGuardPort {
    /// Borrow the principal protected by the held native guard.
    fn principal_id(&self) -> &pale_core::records::ResidentPrincipalIdV1;
}

use crate::{
    ControlCheckpointHead, ControlRecord, ControlRecordCoordinate, PrincipalTrustStoreBinding,
};

/// Exact current resident-writer head carried by one hard-frontier atom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalResidentWriterHead {
    writer_fence_id: PrincipalResidentWriterFenceIdV1,
    principal_id: ResidentPrincipalIdV1,
    writer_generation: PrincipalResidentWriterGenerationV1,
    state: PrincipalResidentWriterFenceStateV1,
    coordinate: ControlRecordCoordinate,
    checkpoint: ControlCheckpointHead,
}

impl PrincipalResidentWriterHead {
    /// Bind the semantic lineage coordinates to one immutable fence record.
    #[must_use]
    pub const fn new(
        writer_fence_id: PrincipalResidentWriterFenceIdV1,
        principal_id: ResidentPrincipalIdV1,
        writer_generation: PrincipalResidentWriterGenerationV1,
        state: PrincipalResidentWriterFenceStateV1,
        coordinate: ControlRecordCoordinate,
        checkpoint: ControlCheckpointHead,
    ) -> Self {
        Self {
            writer_fence_id,
            principal_id,
            writer_generation,
            state,
            coordinate,
            checkpoint,
        }
    }

    /// Borrow the stable writer-fence lineage identity.
    #[must_use]
    pub const fn writer_fence_id(&self) -> &PrincipalResidentWriterFenceIdV1 {
        &self.writer_fence_id
    }

    /// Borrow the principal protected by this writer lineage.
    #[must_use]
    pub const fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.principal_id
    }

    /// Return the positive writer generation.
    #[must_use]
    pub const fn writer_generation(&self) -> PrincipalResidentWriterGenerationV1 {
        self.writer_generation
    }

    /// Return whether this exact fence is active or cleanly retired.
    #[must_use]
    pub const fn state(&self) -> PrincipalResidentWriterFenceStateV1 {
        self.state
    }

    /// Borrow the immutable current fence coordinate.
    #[must_use]
    pub const fn coordinate(&self) -> &ControlRecordCoordinate {
        &self.coordinate
    }

    /// Borrow the control-checkpoint head committed with this writer head.
    #[must_use]
    pub const fn checkpoint(&self) -> &ControlCheckpointHead {
        &self.checkpoint
    }
}

/// Clean writer transition shape closed by application preparation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalResidentWriterTransitionKind {
    /// Install generation one when no writer head exists.
    InitialActive,
    /// Append a terminal receipt and same-generation retired fence.
    CleanRetirement,
    /// Advance a cleanly retired head to the next active generation.
    CleanSuccessor,
}

/// Closed comparison and append inputs awaiting hard-frontier composition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPrincipalResidentWriterTransition {
    binding: PrincipalTrustStoreBinding,
    kind: PrincipalResidentWriterTransitionKind,
    expected_checkpoint: ControlCheckpointHead,
    expected_checkpoint_record: ControlRecord,
    expected_head: Option<PrincipalResidentWriterHead>,
    expected_record: Option<ControlRecord>,
    records: Vec<ControlRecord>,
    next_head: PrincipalResidentWriterHead,
}

impl PreparedPrincipalResidentWriterTransition {
    /// Validate the finite physical transition and its exact record closure.
    ///
    /// The accepted relation is
    #[doc = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/math/principal-writer-transitions.display.mathml"
    ))]
    ///
    /// where `A_g` is an active writer at generation `g` and `R_g` its cleanly
    /// retired state. Clean retirement preserves the generation and clean
    /// succession advances it exactly once. Signature and relationship
    /// admission remain application-owned.
    ///
    /// # Errors
    ///
    /// Returns [`PrincipalResidentWriterCandidateError`] for a mismatched
    /// principal, transition shape, record set, or generation.
    pub fn new(
        binding: PrincipalTrustStoreBinding,
        kind: PrincipalResidentWriterTransitionKind,
        expected_checkpoint: (ControlCheckpointHead, ControlRecord),
        expected_writer: Option<(PrincipalResidentWriterHead, ControlRecord)>,
        records: Vec<ControlRecord>,
        next_head: PrincipalResidentWriterHead,
    ) -> Result<Self, PrincipalResidentWriterCandidateError> {
        let (expected_checkpoint, expected_checkpoint_record) = expected_checkpoint;
        let (expected_head, expected_record) = match expected_writer {
            Some((head, record)) => (Some(head), Some(record)),
            None => (None, None),
        };
        if next_head.principal_id != *binding.principal_id() {
            return Err(PrincipalResidentWriterCandidateError::PrincipalBinding);
        }
        if expected_checkpoint.coordinate() != expected_checkpoint_record.coordinate()
            || expected_head
                .as_ref()
                .is_some_and(|head| head.checkpoint != expected_checkpoint)
            || !next_head
                .checkpoint
                .is_exact_successor_of(&expected_checkpoint)
        {
            return Err(PrincipalResidentWriterCandidateError::CheckpointClosure);
        }
        let unique = records
            .iter()
            .map(|record| record.coordinate().record_ref())
            .collect::<BTreeSet<_>>();
        let expected_closes = match (expected_head.as_ref(), expected_record.as_ref()) {
            (None, None) => true,
            (Some(head), Some(record)) => head.coordinate() == record.coordinate(),
            (None, Some(_)) | (Some(_), None) => false,
        };
        if !expected_closes
            || unique.len() != records.len()
            || !records
                .iter()
                .any(|record| record.coordinate() == next_head.coordinate())
            || !records
                .iter()
                .any(|record| record.coordinate() == next_head.checkpoint.coordinate())
            || records.iter().any(|record| {
                record.coordinate() == expected_checkpoint.coordinate()
                    || expected_head
                        .as_ref()
                        .is_some_and(|head| record.coordinate() == head.coordinate())
            })
        {
            return Err(PrincipalResidentWriterCandidateError::RecordClosure);
        }

        let valid = match (kind, expected_head.as_ref()) {
            (PrincipalResidentWriterTransitionKind::InitialActive, None) => {
                records.len() == 2
                    && next_head.state == PrincipalResidentWriterFenceStateV1::Active
                    && next_head.writer_generation.get() == 1
            }
            (PrincipalResidentWriterTransitionKind::CleanRetirement, Some(expected)) => {
                records.len() == 3
                    && same_lineage(expected, &next_head)
                    && expected.state == PrincipalResidentWriterFenceStateV1::Active
                    && next_head.state == PrincipalResidentWriterFenceStateV1::Retired
                    && expected.writer_generation == next_head.writer_generation
            }
            (PrincipalResidentWriterTransitionKind::CleanSuccessor, Some(expected)) => {
                records.len() == 2
                    && same_lineage(expected, &next_head)
                    && expected.state == PrincipalResidentWriterFenceStateV1::Retired
                    && next_head.state == PrincipalResidentWriterFenceStateV1::Active
                    && expected.writer_generation.checked_successor()
                        == Some(next_head.writer_generation)
            }
            _ => false,
        };
        if !valid {
            return Err(PrincipalResidentWriterCandidateError::Transition);
        }
        Ok(Self {
            binding,
            kind,
            expected_checkpoint,
            expected_checkpoint_record,
            expected_head,
            expected_record,
            records,
            next_head,
        })
    }

    /// Borrow the exact store and principal binding.
    #[must_use]
    pub const fn binding(&self) -> &PrincipalTrustStoreBinding {
        &self.binding
    }

    /// Return the admitted clean-transition class.
    #[must_use]
    pub const fn kind(&self) -> PrincipalResidentWriterTransitionKind {
        self.kind
    }

    /// Borrow the exact control-checkpoint head required before the transition.
    #[must_use]
    pub const fn expected_checkpoint(&self) -> &ControlCheckpointHead {
        &self.expected_checkpoint
    }

    /// Borrow the canonical predecessor checkpoint bytes used for exact read-back.
    #[must_use]
    pub const fn expected_checkpoint_record(&self) -> &ControlRecord {
        &self.expected_checkpoint_record
    }

    /// Borrow the exact expected head, absent only for generation one.
    #[must_use]
    pub const fn expected_head(&self) -> Option<&PrincipalResidentWriterHead> {
        self.expected_head.as_ref()
    }

    /// Borrow the canonical predecessor bytes paired with [`Self::expected_head`].
    #[must_use]
    pub const fn expected_record(&self) -> Option<&ControlRecord> {
        self.expected_record.as_ref()
    }

    /// Borrow the complete proposed append set.
    #[must_use]
    pub fn records(&self) -> &[ControlRecord] {
        &self.records
    }

    /// Borrow the exact intended successor head.
    #[must_use]
    pub const fn next_head(&self) -> &PrincipalResidentWriterHead {
        &self.next_head
    }
}

/// Physical candidate construction failed before storage was touched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PrincipalResidentWriterCandidateError {
    /// The candidate principal differs from its trust-store binding.
    #[error("resident writer principal does not match the trust-store binding")]
    PrincipalBinding,
    /// Appended records do not close exactly over the successor head.
    #[error("resident writer records do not form one exact append set")]
    RecordClosure,
    /// Checkpoint predecessor and successor do not form one exact domain advance.
    #[error("resident writer checkpoint does not advance the exact hard frontier")]
    CheckpointClosure,
    /// Head state or generation contradicts the selected clean transition.
    #[error("resident writer head transition is not an initial-v1 clean transition")]
    Transition,
}

/// Successful durable comparison of one prepared resident-writer transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalResidentWriterCommitDisposition {
    /// The exact predecessor heads advanced to the prepared successor.
    Committed,
    /// The store already contained the complete prepared transition byte-for-byte.
    ExactReplay,
}

/// Exact physical observation after restart or an ambiguous commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalResidentWriterDatabaseObservation {
    /// Both expected predecessor heads remain current and no successor row exists.
    Predecessor,
    /// Both heads and every predecessor and appended record match exactly.
    ExactSuccessor,
    /// Another head, partial append, or contradictory closure is present.
    ConflictingOrPartial,
}

/// Narrow durable custody boundary for the principal resident-writer frontier.
pub trait PrincipalResidentWriterCustodyPort: crate::ControlTransactionPort {
    /// Atomically advance the expected writer and checkpoint heads.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ControlStoreErrorKind::CandidateConflict`] when
    /// another complete head won the compare-and-swap,
    /// [`crate::ControlStoreErrorKind::RecoveryRequired`] for partial or
    /// contradictory state, and
    /// [`crate::ControlStoreErrorKind::OutcomeUnknown`] when commit
    /// failure cannot exclude success.
    fn compare_and_commit(
        &mut self,
        candidate: &PreparedPrincipalResidentWriterTransition,
    ) -> Result<PrincipalResidentWriterCommitDisposition, crate::ControlStoreError>;

    /// Compare current physical state with `candidate` without mutation.
    ///
    /// This is the mandatory recovery route after an outcome-unknown commit.
    /// Exact equality proves database custody only; it grants no OS lock,
    /// liveness, takeover, authentication, or resident-runtime authority.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ControlStoreError`] when observation cannot finish.
    fn observe_exact(
        &mut self,
        candidate: &PreparedPrincipalResidentWriterTransition,
    ) -> Result<PrincipalResidentWriterDatabaseObservation, crate::ControlStoreError>;
}

/// Whether two physical heads belong to one semantic writer lineage.
fn same_lineage(left: &PrincipalResidentWriterHead, right: &PrincipalResidentWriterHead) -> bool {
    left.writer_fence_id == right.writer_fence_id && left.principal_id == right.principal_id
}

#[cfg(test)]
#[path = "../tests/internal/principal_resident_writer.rs"]
mod tests;
