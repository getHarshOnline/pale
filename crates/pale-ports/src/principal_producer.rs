//! Physical closure for principal-wide producer-incarnation transitions.
//!
//! Application policy supplies already-admitted allocation or retirement,
//! principal-trust checkpoint, and control-checkpoint records. This module
//! proves only their finite physical closure before one adapter transaction
//! compares and advances all three heads. It does not generate randomness,
//! prove non-reuse across restore, advance external anchors, or authorize an
//! owner outbox.

use std::collections::BTreeSet;

use pale_core::records::{
    ProducerEpochV1, ProducerIdV1, ProducerIncarnationHeadStateV1, ResidentPrincipalIdV1,
    MAX_RESIDENT_SET_MEMBERS, PRODUCER_INCARNATION_RETIREMENT_V1_SCHEMA,
    PRODUCER_INCARNATION_V1_SCHEMA,
};

use crate::{
    ControlCheckpointHead, ControlRecord, ControlRecordCoordinate, ControlStoreError,
    PrincipalTrustCheckpointHead, PrincipalTrustSnapshot, PrincipalTrustStoreBinding,
};

/// Exact current physical head for one `(principal, producer)` allocation key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalProducerIncarnationHead {
    origin_principal_id: ResidentPrincipalIdV1,
    producer_id: ProducerIdV1,
    producer_epoch: ProducerEpochV1,
    state: ProducerIncarnationHeadStateV1,
    allocation: ControlRecordCoordinate,
    retirement: Option<ControlRecordCoordinate>,
}

impl PrincipalProducerIncarnationHead {
    /// Bind one active or retired allocation head to its immutable records.
    ///
    /// # Errors
    ///
    /// Returns [`PrincipalProducerIncarnationCandidateError::ProducerHead`]
    /// unless active state omits retirement and retired state names one.
    pub fn new(
        origin_principal_id: ResidentPrincipalIdV1,
        producer_id: ProducerIdV1,
        producer_epoch: ProducerEpochV1,
        state: ProducerIncarnationHeadStateV1,
        allocation: ControlRecordCoordinate,
        retirement: Option<ControlRecordCoordinate>,
    ) -> Result<Self, PrincipalProducerIncarnationCandidateError> {
        if matches!(state, ProducerIncarnationHeadStateV1::Active) != retirement.is_none() {
            return Err(PrincipalProducerIncarnationCandidateError::ProducerHead);
        }
        Ok(Self {
            origin_principal_id,
            producer_id,
            producer_epoch,
            state,
            allocation,
            retirement,
        })
    }

    /// Borrow the principal-wide allocation namespace.
    #[must_use]
    pub const fn origin_principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.origin_principal_id
    }

    /// Borrow the operational producer identity.
    #[must_use]
    pub const fn producer_id(&self) -> &ProducerIdV1 {
        &self.producer_id
    }

    /// Borrow the principal-wide unique epoch of this allocation.
    #[must_use]
    pub const fn producer_epoch(&self) -> &ProducerEpochV1 {
        &self.producer_epoch
    }

    /// Return whether the current allocation is active or retired.
    #[must_use]
    pub const fn state(&self) -> ProducerIncarnationHeadStateV1 {
        self.state
    }

    /// Borrow the current immutable allocation record coordinate.
    #[must_use]
    pub const fn allocation(&self) -> &ControlRecordCoordinate {
        &self.allocation
    }

    /// Borrow the append-only retirement coordinate when the head is retired.
    #[must_use]
    pub const fn retirement(&self) -> Option<&ControlRecordCoordinate> {
        self.retirement.as_ref()
    }
}

/// Closed producer-incarnation transition selected by application admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalProducerIncarnationTransitionKind {
    /// Install the first active allocation for one producer key.
    InitialAllocation,
    /// Append retirement to the exact active allocation.
    Retirement,
    /// Replace an exact retired head with a fresh active allocation.
    SuccessorAllocation,
}

/// Exact physical predecessor frontier consumed by one producer transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalProducerIncarnationExpectedFrontier {
    producer: Option<(PrincipalProducerIncarnationHead, Vec<ControlRecord>)>,
    trust: (PrincipalTrustCheckpointHead, ControlRecord),
    checkpoint: (ControlCheckpointHead, ControlRecord),
}

impl PrincipalProducerIncarnationExpectedFrontier {
    /// Group the three independently typed predecessor heads and their bytes.
    #[must_use]
    pub fn new(
        producer: Option<(PrincipalProducerIncarnationHead, Vec<ControlRecord>)>,
        trust: (PrincipalTrustCheckpointHead, ControlRecord),
        checkpoint: (ControlCheckpointHead, ControlRecord),
    ) -> Self {
        Self {
            producer,
            trust,
            checkpoint,
        }
    }
}

/// Three intended successor heads closed by one producer transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalProducerIncarnationSuccessorFrontier {
    producer: PrincipalProducerIncarnationHead,
    trust: PrincipalTrustCheckpointHead,
    checkpoint: ControlCheckpointHead,
}

impl PrincipalProducerIncarnationSuccessorFrontier {
    /// Group producer, principal-trust, and hard-domain successor heads.
    #[must_use]
    pub const fn new(
        producer: PrincipalProducerIncarnationHead,
        trust: PrincipalTrustCheckpointHead,
        checkpoint: ControlCheckpointHead,
    ) -> Self {
        Self {
            producer,
            trust,
            checkpoint,
        }
    }
}

/// Closed physical transition awaiting one atomic trust-store comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPrincipalProducerIncarnationTransition {
    binding: PrincipalTrustStoreBinding,
    kind: PrincipalProducerIncarnationTransitionKind,
    expected_producer: Option<PrincipalProducerIncarnationHead>,
    expected_producer_records: Vec<ControlRecord>,
    expected_trust: PrincipalTrustCheckpointHead,
    expected_trust_record: ControlRecord,
    expected_checkpoint: ControlCheckpointHead,
    expected_checkpoint_record: ControlRecord,
    records: Vec<ControlRecord>,
    next_producer: PrincipalProducerIncarnationHead,
    next_trust: PrincipalTrustCheckpointHead,
    next_checkpoint: ControlCheckpointHead,
    accumulator: Option<crate::OperationAccumulatorBatch>,
}

impl PreparedPrincipalProducerIncarnationTransition {
    /// Validate the three-head transition and complete immutable record closure.
    ///
    /// Every transition appends exactly one allocation or retirement, one
    /// principal-trust checkpoint, and one control checkpoint. The successor
    /// trust and control heads must each advance once. Signature, root, and
    /// operation semantics remain application-owned.
    ///
    /// # Errors
    ///
    /// Returns [`PrincipalProducerIncarnationCandidateError`] for a principal,
    /// predecessor, append-set, producer-state, or checkpoint contradiction.
    pub fn new(
        binding: PrincipalTrustStoreBinding,
        kind: PrincipalProducerIncarnationTransitionKind,
        expected: PrincipalProducerIncarnationExpectedFrontier,
        records: Vec<ControlRecord>,
        next: PrincipalProducerIncarnationSuccessorFrontier,
    ) -> Result<Self, PrincipalProducerIncarnationCandidateError> {
        let (expected_producer, expected_producer_records) = expected
            .producer
            .map_or((None, Vec::new()), |(head, records)| (Some(head), records));
        let (expected_trust, expected_trust_record) = expected.trust;
        let (expected_checkpoint, expected_checkpoint_record) = expected.checkpoint;
        let PrincipalProducerIncarnationSuccessorFrontier {
            producer: next_producer,
            trust: next_trust,
            checkpoint: next_checkpoint,
        } = next;

        if next_producer.origin_principal_id != *binding.principal_id()
            || expected_producer
                .as_ref()
                .is_some_and(|head| head.origin_principal_id != *binding.principal_id())
            || expected_trust.principal_id() != binding.principal_id()
            || next_trust.principal_id() != binding.principal_id()
        {
            return Err(PrincipalProducerIncarnationCandidateError::PrincipalBinding);
        }
        if expected_trust.coordinate() != expected_trust_record.coordinate()
            || !next_trust.is_exact_successor_of(&expected_trust)
        {
            return Err(PrincipalProducerIncarnationCandidateError::TrustCheckpointClosure);
        }
        if expected_checkpoint.coordinate() != expected_checkpoint_record.coordinate()
            || !next_checkpoint.is_exact_successor_of(&expected_checkpoint)
        {
            return Err(PrincipalProducerIncarnationCandidateError::ControlCheckpointClosure);
        }
        if !expected_producer_closes(expected_producer.as_ref(), &expected_producer_records) {
            return Err(PrincipalProducerIncarnationCandidateError::ProducerHead);
        }

        let unique = records
            .iter()
            .map(|record| record.coordinate().record_ref())
            .collect::<BTreeSet<_>>();
        let operation_coordinate = match kind {
            PrincipalProducerIncarnationTransitionKind::InitialAllocation
            | PrincipalProducerIncarnationTransitionKind::SuccessorAllocation => {
                next_producer.allocation()
            }
            PrincipalProducerIncarnationTransitionKind::Retirement => next_producer
                .retirement()
                .ok_or(PrincipalProducerIncarnationCandidateError::Transition)?,
        };
        let closes = records.len() == 3
            && unique.len() == records.len()
            && records
                .iter()
                .any(|record| record.coordinate() == operation_coordinate)
            && records
                .iter()
                .any(|record| record.coordinate() == next_trust.coordinate())
            && records
                .iter()
                .any(|record| record.coordinate() == next_checkpoint.coordinate())
            && records.iter().all(|record| {
                record.coordinate() != expected_trust.coordinate()
                    && record.coordinate() != expected_checkpoint.coordinate()
                    && expected_producer_records
                        .iter()
                        .all(|expected| expected.coordinate() != record.coordinate())
            });
        if !closes {
            return Err(PrincipalProducerIncarnationCandidateError::RecordClosure);
        }

        let valid = match (kind, expected_producer.as_ref()) {
            (PrincipalProducerIncarnationTransitionKind::InitialAllocation, None) => {
                next_producer.state == ProducerIncarnationHeadStateV1::Active
                    && next_producer.retirement.is_none()
            }
            (PrincipalProducerIncarnationTransitionKind::Retirement, Some(expected)) => {
                same_producer(expected, &next_producer)
                    && expected.state == ProducerIncarnationHeadStateV1::Active
                    && next_producer.state == ProducerIncarnationHeadStateV1::Retired
                    && expected.producer_epoch == next_producer.producer_epoch
                    && expected.allocation == next_producer.allocation
            }
            (PrincipalProducerIncarnationTransitionKind::SuccessorAllocation, Some(expected)) => {
                same_producer(expected, &next_producer)
                    && expected.state == ProducerIncarnationHeadStateV1::Retired
                    && next_producer.state == ProducerIncarnationHeadStateV1::Active
                    && expected.producer_epoch != next_producer.producer_epoch
                    && expected.allocation != next_producer.allocation
                    && next_producer.retirement.is_none()
            }
            _ => false,
        };
        if !valid {
            return Err(PrincipalProducerIncarnationCandidateError::Transition);
        }

        Ok(Self {
            binding,
            kind,
            expected_producer,
            expected_producer_records,
            expected_trust,
            expected_trust_record,
            expected_checkpoint,
            expected_checkpoint_record,
            records,
            next_producer,
            next_trust,
            next_checkpoint,
            accumulator: None,
        })
    }

    /// Retain tree changes for this exact store and checkpoint transition.
    ///
    /// The application owns root construction and checkpoint admission. This
    /// attachment checks physical identity only; custody must still compare
    /// the prepared nodes and transaction against the database.
    ///
    /// # Errors
    /// Rejects repeated attachment or a different store, phase or checkpoint pair.
    pub fn with_accumulator(
        mut self,
        accumulator: crate::OperationAccumulatorBatch,
    ) -> Result<Self, PrincipalProducerIncarnationCandidateError> {
        let binding = accumulator.binding();
        if self.accumulator.is_some()
            || binding.store().store_id() != self.binding.trust_store_id()
            || binding.store().incarnation_id() != self.binding.trust_store_incarnation_id()
            || binding.successor() != &self.next_checkpoint
            || !matches!(binding.phase(), crate::AccumulatorCheckpointPhase::Successor(prior)
                if prior == &self.expected_checkpoint)
        {
            return Err(PrincipalProducerIncarnationCandidateError::AccumulatorClosure);
        }
        self.accumulator = Some(accumulator);
        Ok(self)
    }

    /// Borrow prepared tree custody. Absence does not prove root construction.
    #[must_use]
    pub const fn accumulator(&self) -> Option<&crate::OperationAccumulatorBatch> {
        self.accumulator.as_ref()
    }

    /// Borrow the exact principal trust-store binding.
    #[must_use]
    pub const fn binding(&self) -> &PrincipalTrustStoreBinding {
        &self.binding
    }

    /// Return the admitted transition class.
    #[must_use]
    pub const fn kind(&self) -> PrincipalProducerIncarnationTransitionKind {
        self.kind
    }

    /// Borrow the current producer head, absent only for initial allocation.
    #[must_use]
    pub const fn expected_producer(&self) -> Option<&PrincipalProducerIncarnationHead> {
        self.expected_producer.as_ref()
    }

    /// Borrow all immutable records closing the expected producer head.
    #[must_use]
    pub fn expected_producer_records(&self) -> &[ControlRecord] {
        &self.expected_producer_records
    }

    /// Borrow the exact predecessor trust-checkpoint head.
    #[must_use]
    pub const fn expected_trust(&self) -> &PrincipalTrustCheckpointHead {
        &self.expected_trust
    }

    /// Borrow the canonical predecessor trust-checkpoint bytes.
    #[must_use]
    pub const fn expected_trust_record(&self) -> &ControlRecord {
        &self.expected_trust_record
    }

    /// Borrow the exact predecessor control-checkpoint head.
    #[must_use]
    pub const fn expected_checkpoint(&self) -> &ControlCheckpointHead {
        &self.expected_checkpoint
    }

    /// Borrow the canonical predecessor control-checkpoint bytes.
    #[must_use]
    pub const fn expected_checkpoint_record(&self) -> &ControlRecord {
        &self.expected_checkpoint_record
    }

    /// Borrow the complete three-record append set.
    #[must_use]
    pub fn records(&self) -> &[ControlRecord] {
        &self.records
    }

    /// Borrow the intended producer successor head.
    #[must_use]
    pub const fn next_producer(&self) -> &PrincipalProducerIncarnationHead {
        &self.next_producer
    }

    /// Borrow the intended principal-trust successor head.
    #[must_use]
    pub const fn next_trust(&self) -> &PrincipalTrustCheckpointHead {
        &self.next_trust
    }

    /// Borrow the intended control-checkpoint successor head.
    #[must_use]
    pub const fn next_checkpoint(&self) -> &ControlCheckpointHead {
        &self.next_checkpoint
    }
}

/// Physical producer candidate construction failed before storage access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PrincipalProducerIncarnationCandidateError {
    /// The tree batch belongs to another store or checkpoint transition.
    #[error("producer accumulator does not match its checkpoint transition")]
    AccumulatorClosure,
    /// A producer or trust head differs from the store's principal binding.
    #[error("producer-incarnation principal does not match the trust-store binding")]
    PrincipalBinding,
    /// Expected producer records do not close over one valid active or retired head.
    #[error("producer-incarnation predecessor records do not close over its head")]
    ProducerHead,
    /// Appended records do not close over the three intended successor heads.
    #[error("producer-incarnation records do not form one exact append set")]
    RecordClosure,
    /// The signed trust checkpoint does not advance exactly once.
    #[error("principal trust checkpoint does not advance the exact predecessor")]
    TrustCheckpointClosure,
    /// The hard-domain checkpoint does not advance exactly once.
    #[error("producer control checkpoint does not advance the exact predecessor")]
    ControlCheckpointClosure,
    /// Producer state contradicts the selected allocation or retirement transition.
    #[error("producer-incarnation head transition is not an initial-v1 transition")]
    Transition,
}

/// Successful durable comparison of one prepared producer transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalProducerIncarnationCommitDisposition {
    /// All three expected heads advanced to the prepared successor.
    Committed,
    /// The store already contains the complete transition byte-for-byte.
    ExactReplay,
}

/// Exact physical observation after restart or ambiguous commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalProducerIncarnationDatabaseObservation {
    /// All expected heads remain current and no proposed append exists.
    Predecessor,
    /// All successor heads, predecessor bytes, and appended bytes match exactly.
    ExactSuccessor,
    /// Another head, partial append, or contradictory closure exists.
    ConflictingOrPartial,
}

/// Narrow physical custody boundary for principal-wide producer transitions.
pub trait PrincipalProducerIncarnationCustodyPort: crate::ControlTransactionPort {
    /// Compute bounded successor trees from one read-only store snapshot.
    ///
    /// This does not sign a checkpoint or commit records.
    ///
    /// # Errors
    /// Rejects stale stores, checkpoints or transactions, invalid tree state,
    /// and work that exceeds the request's limits.
    fn prepare_accumulator(
        &mut self,
        request: &crate::OperationAccumulatorPreparation,
    ) -> Result<crate::PreparedOperationAccumulator, crate::OperationAccumulatorPreparationError>;

    /// Atomically advance producer, principal-trust, and control-checkpoint heads.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ControlStoreErrorKind::CandidateConflict`] for a
    /// complete competing frontier,
    /// [`crate::ControlStoreErrorKind::RecoveryRequired`] for partial or
    /// contradictory state, and
    /// [`crate::ControlStoreErrorKind::OutcomeUnknown`] when commit
    /// failure cannot exclude success.
    fn compare_and_commit(
        &mut self,
        candidate: &PreparedPrincipalProducerIncarnationTransition,
    ) -> Result<PrincipalProducerIncarnationCommitDisposition, ControlStoreError>;

    /// Compare the exact transition without mutation after restart or ambiguity.
    ///
    /// Exact equality proves database custody only. It does not prove CSPRNG
    /// provenance, rollback-safe non-reuse, external anchors, or outbox safety.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] when observation cannot complete.
    fn observe_exact(
        &mut self,
        candidate: &PreparedPrincipalProducerIncarnationTransition,
    ) -> Result<PrincipalProducerIncarnationDatabaseObservation, ControlStoreError>;
}

/// Exact immutable records recovered for one producer incarnation.
///
/// This projection is suitable for contextual producer-incarnation admission after restart. It
/// proves physical record closure only: the application must still decode and
/// admit the signed records, resolve their trust checkpoints, and establish
/// whether an owner publication preceded retirement. A retired incarnation may
/// identify immutable outbox rows already committed under its epoch, but this
/// read grants no authority to append a new row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalProducerIncarnationRecordSet {
    head: PrincipalProducerIncarnationHead,
    previous_allocation: Option<ControlRecordCoordinate>,
    allocation: ControlRecord,
    retirement: Option<ControlRecord>,
}

impl PrincipalProducerIncarnationRecordSet {
    /// Bind an allocation and optional retirement to their exact physical head.
    ///
    /// # Errors
    ///
    /// Returns [`PrincipalProducerIncarnationReadError`] when the records do not
    /// close over the head or use the selected producer schemas.
    pub fn new(
        head: PrincipalProducerIncarnationHead,
        previous_allocation: Option<ControlRecordCoordinate>,
        allocation: ControlRecord,
        retirement: Option<ControlRecord>,
    ) -> Result<Self, PrincipalProducerIncarnationReadError> {
        if allocation.coordinate() != head.allocation()
            || allocation.schema_id().as_str() != PRODUCER_INCARNATION_V1_SCHEMA
        {
            return Err(PrincipalProducerIncarnationReadError::Allocation);
        }
        let retirement_is_exact = match (head.state(), head.retirement(), retirement.as_ref()) {
            (ProducerIncarnationHeadStateV1::Active, None, None) => true,
            (ProducerIncarnationHeadStateV1::Retired, Some(expected), Some(observed)) => {
                observed.coordinate() == expected
                    && observed.schema_id().as_str() == PRODUCER_INCARNATION_RETIREMENT_V1_SCHEMA
            }
            _ => false,
        };
        if !retirement_is_exact {
            return Err(PrincipalProducerIncarnationReadError::Retirement);
        }
        Ok(Self {
            head,
            previous_allocation,
            allocation,
            retirement,
        })
    }

    /// Borrow the producer/incarnation coordinates and physical state.
    #[must_use]
    pub const fn head(&self) -> &PrincipalProducerIncarnationHead {
        &self.head
    }

    /// Borrow the exact prior allocation coordinate retained by physical custody.
    ///
    /// This is `None` only for a producer's first allocation. The coordinate is
    /// traversal evidence, not an admitted predecessor; restart recovery must
    /// still read and contextually admit the referenced allocation and its
    /// retirement before admitting a successor.
    #[must_use]
    pub const fn previous_allocation(&self) -> Option<&ControlRecordCoordinate> {
        self.previous_allocation.as_ref()
    }

    /// Borrow the exact immutable allocation record.
    #[must_use]
    pub const fn allocation(&self) -> &ControlRecord {
        &self.allocation
    }

    /// Borrow the exact immutable retirement record when the incarnation retired.
    #[must_use]
    pub const fn retirement(&self) -> Option<&ControlRecord> {
        self.retirement.as_ref()
    }
}

/// Result of one bounded producer-incarnation point read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrincipalProducerIncarnationReadObservation {
    /// No allocation exists at the exact requested key.
    Absent,
    /// The allocation and optional retirement close over one physical state.
    Exact(Box<PrincipalProducerIncarnationRecordSet>),
    /// Rows exist but do not form one complete incarnation state.
    ConflictingOrPartial,
}

/// A physically returned producer record set did not close over its typed head.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PrincipalProducerIncarnationReadError {
    /// The allocation coordinate or schema differs from the head.
    #[error("producer-incarnation point read contains the wrong allocation")]
    Allocation,
    /// Retirement presence, coordinate, or schema contradicts the head state.
    #[error("producer-incarnation point read contains the wrong retirement state")]
    Retirement,
}

/// Stable exclusive cursor for principal-wide producer-head discovery.
///
/// Producer identities are unique within the principal fixed by
/// [`PrincipalProducerIncarnationHeadPageRead::binding`], so the identity is
/// the complete ordering coordinate. The cursor grants no producer authority.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PrincipalProducerIncarnationHeadCursor(ProducerIdV1);

impl PrincipalProducerIncarnationHeadCursor {
    /// Retain one already validated principal-scoped producer identity.
    #[must_use]
    pub const fn new(producer_id: ProducerIdV1) -> Self {
        Self(producer_id)
    }

    /// Borrow the producer identity used as the stable ordering coordinate.
    #[must_use]
    pub const fn producer_id(&self) -> &ProducerIdV1 {
        &self.0
    }
}

/// Bounded producer-head page request under one exact trust-checkpoint head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalProducerIncarnationHeadPageRead {
    snapshot: PrincipalTrustSnapshot,
    after: Option<PrincipalProducerIncarnationHeadCursor>,
    maximum_items: usize,
}

impl PrincipalProducerIncarnationHeadPageRead {
    /// Select one stable page of the principal's complete producer-head set.
    ///
    /// # Errors
    ///
    /// Returns [`PrincipalProducerIncarnationHeadPageError::ZeroBound`] for an empty
    /// item budget, or
    /// [`PrincipalProducerIncarnationHeadPageError::BoundExceeded`] above the
    /// shared resident collection ceiling.
    pub fn new(
        snapshot: PrincipalTrustSnapshot,
        after: Option<PrincipalProducerIncarnationHeadCursor>,
        maximum_items: usize,
    ) -> Result<Self, PrincipalProducerIncarnationHeadPageError> {
        if maximum_items == 0 {
            return Err(PrincipalProducerIncarnationHeadPageError::ZeroBound);
        }
        if maximum_items > MAX_RESIDENT_SET_MEMBERS {
            return Err(PrincipalProducerIncarnationHeadPageError::BoundExceeded);
        }
        Ok(Self {
            snapshot,
            after,
            maximum_items,
        })
    }

    /// Borrow the fixed physical snapshot shared by pages and lineage reads.
    #[must_use]
    pub const fn snapshot(&self) -> &PrincipalTrustSnapshot {
        &self.snapshot
    }

    /// Borrow the exact trust-store incarnation and principal binding.
    #[must_use]
    pub const fn binding(&self) -> &PrincipalTrustStoreBinding {
        self.snapshot.binding()
    }

    /// Borrow the signed trust-checkpoint head fencing this discovery pass.
    #[must_use]
    pub const fn through(&self) -> &PrincipalTrustCheckpointHead {
        self.snapshot.through()
    }

    /// Borrow the exact stored signed-checkpoint bytes fencing this pass.
    #[must_use]
    pub const fn checkpoint_record(&self) -> &ControlRecord {
        self.snapshot.checkpoint_record()
    }

    /// Borrow the exclusive continuation cursor, when reading a later page.
    #[must_use]
    pub const fn after(&self) -> Option<&PrincipalProducerIncarnationHeadCursor> {
        self.after.as_ref()
    }

    /// Return the hard maximum number of rows in one page.
    #[must_use]
    pub const fn maximum_items(&self) -> usize {
        self.maximum_items
    }
}

/// One physically closed producer head paired with its stable cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalProducerIncarnationHeadPageEntry {
    cursor: PrincipalProducerIncarnationHeadCursor,
    records: PrincipalProducerIncarnationRecordSet,
}

impl PrincipalProducerIncarnationHeadPageEntry {
    /// Bind one record set to its exact producer ordering coordinate.
    ///
    /// # Errors
    ///
    /// Returns [`PrincipalProducerIncarnationHeadPageError::ProducerCursor`]
    /// when the cursor and physical head identify different producers.
    pub fn new(
        cursor: PrincipalProducerIncarnationHeadCursor,
        records: PrincipalProducerIncarnationRecordSet,
    ) -> Result<Self, PrincipalProducerIncarnationHeadPageError> {
        if cursor.producer_id() != records.head().producer_id() {
            return Err(PrincipalProducerIncarnationHeadPageError::ProducerCursor);
        }
        Ok(Self { cursor, records })
    }

    /// Borrow the stable producer cursor.
    #[must_use]
    pub const fn cursor(&self) -> &PrincipalProducerIncarnationHeadCursor {
        &self.cursor
    }

    /// Borrow the allocation and optional retirement records closing the head.
    #[must_use]
    pub const fn records(&self) -> &PrincipalProducerIncarnationRecordSet {
        &self.records
    }
}

/// Completion state of one bounded producer-head page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrincipalProducerIncarnationHeadPageBoundary {
    /// No additional producer head exists under the fixed checkpoint.
    Complete,
    /// More heads remain; continue strictly after the returned cursor.
    More {
        /// Exact last cursor returned by this page.
        resume_after: PrincipalProducerIncarnationHeadCursor,
    },
}

/// One ordered producer-head page tied to a fixed trust checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalProducerIncarnationHeadPage {
    request: PrincipalProducerIncarnationHeadPageRead,
    entries: Box<[PrincipalProducerIncarnationHeadPageEntry]>,
    boundary: PrincipalProducerIncarnationHeadPageBoundary,
}

impl PrincipalProducerIncarnationHeadPage {
    /// Validate page bounds, principal closure, order, and continuation.
    ///
    /// # Errors
    ///
    /// Returns [`PrincipalProducerIncarnationHeadPageError`] when a row is
    /// outside the request, repeated, unordered, over budget, or paired with
    /// an inexact continuation cursor.
    pub fn new(
        request: &PrincipalProducerIncarnationHeadPageRead,
        entries: Vec<PrincipalProducerIncarnationHeadPageEntry>,
        boundary: PrincipalProducerIncarnationHeadPageBoundary,
    ) -> Result<Self, PrincipalProducerIncarnationHeadPageError> {
        if entries.len() > request.maximum_items {
            return Err(PrincipalProducerIncarnationHeadPageError::BoundExceeded);
        }
        if entries
            .windows(2)
            .any(|pair| pair[0].cursor >= pair[1].cursor)
            || entries.first().is_some_and(|entry| {
                request
                    .after
                    .as_ref()
                    .is_some_and(|after| entry.cursor <= *after)
            })
        {
            return Err(PrincipalProducerIncarnationHeadPageError::NonAscending);
        }
        if entries.iter().any(|entry| {
            entry.records.head().origin_principal_id() != request.binding().principal_id()
        }) {
            return Err(PrincipalProducerIncarnationHeadPageError::PrincipalBinding);
        }
        if let PrincipalProducerIncarnationHeadPageBoundary::More { resume_after } = &boundary {
            if entries
                .last()
                .map(PrincipalProducerIncarnationHeadPageEntry::cursor)
                != Some(resume_after)
            {
                return Err(PrincipalProducerIncarnationHeadPageError::Continuation);
            }
        }
        Ok(Self {
            request: request.clone(),
            entries: entries.into_boxed_slice(),
            boundary,
        })
    }

    /// Borrow the complete observation coordinate and page bound.
    #[must_use]
    pub const fn request(&self) -> &PrincipalProducerIncarnationHeadPageRead {
        &self.request
    }

    /// Borrow the exact trust-store incarnation and principal binding.
    #[must_use]
    pub const fn binding(&self) -> &PrincipalTrustStoreBinding {
        self.request.binding()
    }

    /// Borrow the exact trust checkpoint under which this page was observed.
    #[must_use]
    pub const fn through(&self) -> &PrincipalTrustCheckpointHead {
        self.request.through()
    }

    /// Borrow the stable ascending producer-head rows.
    #[must_use]
    pub const fn entries(&self) -> &[PrincipalProducerIncarnationHeadPageEntry] {
        &self.entries
    }

    /// Borrow the exact completion or continuation state.
    #[must_use]
    pub const fn boundary(&self) -> &PrincipalProducerIncarnationHeadPageBoundary {
        &self.boundary
    }
}

/// A producer-head discovery page contradicts its fixed request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PrincipalProducerIncarnationHeadPageError {
    /// The signed trust checkpoint and store binding name different principals.
    #[error("producer-head page checkpoint does not match the principal binding")]
    PrincipalBinding,
    /// A zero item bound cannot admit even one producer head.
    #[error("producer-head page bound is zero")]
    ZeroBound,
    /// The page contains more rows than the explicit request bound.
    #[error("producer-head page exceeds its item bound")]
    BoundExceeded,
    /// Rows are repeated, stale, or not strictly ascending by producer identity.
    #[error("producer-head page is not in strict producer order")]
    NonAscending,
    /// A row cursor and its physical head identify different producers.
    #[error("producer-head page cursor does not match its record set")]
    ProducerCursor,
    /// A non-terminal page does not resume after its exact final row.
    #[error("producer-head page continuation cursor is not exact")]
    Continuation,
}

/// Bounded restart-time reads for principal-wide producer incarnations.
///
/// This interface is intentionally separate from
/// [`PrincipalProducerIncarnationCustodyPort`]: reads establish no transition
/// authority and mutation candidates never gain an implicit discovery path.
pub trait PrincipalProducerIncarnationReadPort {
    /// Read one globally unique producer epoch within a principal.
    ///
    /// A retired result remains readable for immutable pre-retirement outbox
    /// drain and replay. It cannot authorize new publication under that epoch.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] when the store cannot prove a complete
    /// bounded point observation.
    fn read_by_epoch(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        producer_epoch: &ProducerEpochV1,
    ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError>;

    /// Read the current head for one principal-scoped producer identity.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] when the store cannot prove a complete
    /// bounded point observation.
    fn read_head(
        &mut self,
        binding: &PrincipalTrustStoreBinding,
        producer_id: &ProducerIdV1,
    ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError>;

    /// Read one historical allocation by its exact coordinate under a fixed snapshot.
    ///
    /// The returned record set retains its predecessor coordinate so the
    /// application can reconstruct and contextually re-admit the lineage. The
    /// implementation must compare the snapshot before and after the point read;
    /// a changed checkpoint fails closed instead of mixing histories.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] unless the store can prove a complete point
    /// observation under `snapshot`.
    fn read_by_allocation(
        &mut self,
        snapshot: &PrincipalTrustSnapshot,
        allocation: &ControlRecordCoordinate,
    ) -> Result<PrincipalProducerIncarnationReadObservation, ControlStoreError>;

    /// Read one stable page of every current head for the selected principal.
    ///
    /// Implementations must compare [`PrincipalProducerIncarnationHeadPageRead::through`]
    /// before and after materializing the page. A changed checkpoint closes the
    /// read with [`crate::ControlStoreErrorKind::RecoveryRequired`] rather than
    /// mixing unlike principal-wide frontiers.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] unless the store can prove one complete,
    /// bounded page under the exact requested trust-checkpoint head.
    fn read_heads_page(
        &mut self,
        request: &PrincipalProducerIncarnationHeadPageRead,
    ) -> Result<PrincipalProducerIncarnationHeadPage, ControlStoreError>;
}

/// Validate immutable record closure for an expected producer head.
fn expected_producer_closes(
    head: Option<&PrincipalProducerIncarnationHead>,
    records: &[ControlRecord],
) -> bool {
    match head {
        None => records.is_empty(),
        Some(head) => {
            let expected = std::iter::once(head.allocation())
                .chain(head.retirement())
                .collect::<BTreeSet<_>>();
            let observed = records
                .iter()
                .map(ControlRecord::coordinate)
                .collect::<BTreeSet<_>>();
            expected.len() == records.len() && expected == observed
        }
    }
}

/// Test whether two physical heads address one producer allocation key.
fn same_producer(
    left: &PrincipalProducerIncarnationHead,
    right: &PrincipalProducerIncarnationHead,
) -> bool {
    left.origin_principal_id == right.origin_principal_id && left.producer_id == right.producer_id
}

#[cfg(test)]
#[path = "../tests/internal/principal_producer.rs"]
mod tests;
