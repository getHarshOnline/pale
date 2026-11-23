//! Read-only preparation of the two trees before a checkpoint is signed.
//!
//! The application supplies an initial-domain selection or an admitted
//! predecessor and logical changes. The adapter checks the selected phase
//! against one read snapshot and returns owned, finite
//! changes. Preparation neither commits a checkpoint nor grants authority.

use pale_core::records::{
    ArtifactDigest, CheckpointProfileDigest, ControlCheckpointDomainIdV1, ControlDomainEpochV1,
    ResidentRecordRefV1,
};

use super::{
    AccumulatorBatchError, AccumulatorCheckpointBinding, AccumulatorCheckpointPhase,
    OperationAccumulatorBatch, StateAccumulatorBatch, UnresolvedCustodyAccumulatorBatch,
};
use crate::{
    AuthorityStoreCoordinate, ControlCheckpointHead, ControlStoreError,
    ControlTransactionPreparation,
};

/// Closed native accumulator mechanics, independent of logical profile authority.
/// An implementation change requires a new explicit adapter match and vectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccumulatorImplementation {
    /// Pinned trie merge rules, SHA-256, and Pale's finite node encoding.
    SparseMerkleTree061Sha256PaleV1,
}

impl AccumulatorImplementation {
    /// Source-owned implementation identity included by logical profile content.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SparseMerkleTree061Sha256PaleV1 => {
                "sparse-merkle-tree/0.6.1/trie/sha256/pale-node-v1"
            }
        }
    }
}

/// Exact physical checkpoint supplied by the checkpoint admission owner.
/// Constructing this value does not verify its signature or independent anchor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationAccumulatorCheckpoint {
    store: AuthorityStoreCoordinate,
    profile_ref: ResidentRecordRefV1,
    profile_sha256: CheckpointProfileDigest,
    checkpoint: ControlCheckpointHead,
    state_root: ArtifactDigest,
    unresolved_root: ArtifactDigest,
}

impl OperationAccumulatorCheckpoint {
    /// Retain the complete checkpoint coordinates without selecting defaults.
    #[must_use]
    pub const fn new(
        store: AuthorityStoreCoordinate,
        profile_ref: ResidentRecordRefV1,
        profile_sha256: CheckpointProfileDigest,
        checkpoint: ControlCheckpointHead,
        state_root: ArtifactDigest,
        unresolved_root: ArtifactDigest,
    ) -> Self {
        Self {
            store,
            profile_ref,
            profile_sha256,
            checkpoint,
            state_root,
            unresolved_root,
        }
    }

    /// Borrow the selected store incarnation.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Borrow the immutable profile reference.
    #[must_use]
    pub const fn profile_ref(&self) -> &ResidentRecordRefV1 {
        &self.profile_ref
    }

    /// Borrow the immutable profile digest.
    #[must_use]
    pub const fn profile_sha256(&self) -> &CheckpointProfileDigest {
        &self.profile_sha256
    }

    /// Borrow the signed checkpoint's physical coordinate.
    #[must_use]
    pub const fn checkpoint(&self) -> &ControlCheckpointHead {
        &self.checkpoint
    }

    /// Borrow the authenticated full-state root.
    #[must_use]
    pub const fn state_root(&self) -> &ArtifactDigest {
        &self.state_root
    }

    /// Borrow the independently classified unresolved-custody root.
    #[must_use]
    pub const fn unresolved_root(&self) -> &ArtifactDigest {
        &self.unresolved_root
    }
}

/// Exact application-selected scope for the first operation checkpoint.
/// No record, root, tree absence, signer or deployment admission is fabricated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccumulatorInitialDomain {
    store: AuthorityStoreCoordinate,
    profile_ref: ResidentRecordRefV1,
    profile_sha256: CheckpointProfileDigest,
    domain: ControlCheckpointDomainIdV1,
    epoch: ControlDomainEpochV1,
}

impl AccumulatorInitialDomain {
    /// Retain independently selected coordinates; native preparation must prove
    /// whole-domain absence across other epoch/profile partitions as well.
    #[must_use]
    pub const fn new(
        store: AuthorityStoreCoordinate,
        profile_ref: ResidentRecordRefV1,
        profile_sha256: CheckpointProfileDigest,
        domain: ControlCheckpointDomainIdV1,
        epoch: ControlDomainEpochV1,
    ) -> Self {
        Self {
            store,
            profile_ref,
            profile_sha256,
            domain,
            epoch,
        }
    }
}

/// Closed basis for bounded initial insertion or an authenticated replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccumulatorPreparationBasis {
    /// No checkpoint predecessor exists; absence must be proved, not assumed.
    Initial(AccumulatorInitialDomain),
    /// One exact admitted stored checkpoint and its two authenticated roots.
    Successor(OperationAccumulatorCheckpoint),
}

impl AccumulatorPreparationBasis {
    /// Borrow the independently admitted store incarnation.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        match self {
            Self::Initial(value) => &value.store,
            Self::Successor(value) => &value.store,
        }
    }
    /// Borrow the exact immutable profile reference.
    #[must_use]
    pub const fn profile_ref(&self) -> &ResidentRecordRefV1 {
        match self {
            Self::Initial(value) => &value.profile_ref,
            Self::Successor(value) => &value.profile_ref,
        }
    }
    /// Borrow the exact immutable profile digest.
    #[must_use]
    pub const fn profile_sha256(&self) -> &CheckpointProfileDigest {
        match self {
            Self::Initial(value) => &value.profile_sha256,
            Self::Successor(value) => &value.profile_sha256,
        }
    }
    /// Borrow the domain, independently of its epoch/profile partition.
    #[must_use]
    pub fn domain(&self) -> &ControlCheckpointDomainIdV1 {
        match self {
            Self::Initial(value) => &value.domain,
            Self::Successor(value) => value.checkpoint.checkpoint_domain_id(),
        }
    }
    /// Return the already-admitted epoch; this grants no epoch reset authority.
    #[must_use]
    pub fn epoch(&self) -> ControlDomainEpochV1 {
        match self {
            Self::Initial(value) => value.epoch,
            Self::Successor(value) => value.checkpoint.domain_epoch(),
        }
    }
}

/// Replacement at one hashed logical key; `None` alone denotes absence.
/// Terminal and uncertain operation states are values, not implicit removals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccumulatorLeafChange {
    key: ArtifactDigest,
    predecessor: Option<ArtifactDigest>,
    successor: Option<ArtifactDigest>,
}

impl AccumulatorLeafChange {
    /// Retain one actual change without conflating zero with a present value.
    ///
    /// # Errors
    /// Rejects unchanged values and present all-zero hashes, which the selected
    /// sparse tree reserves for absence. The profile must not encode a present
    /// logical value as that sentinel.
    pub fn new(
        key: ArtifactDigest,
        predecessor: Option<ArtifactDigest>,
        successor: Option<ArtifactDigest>,
    ) -> Result<Self, AccumulatorPreparationInputError> {
        if predecessor
            .iter()
            .chain(successor.iter())
            .any(|hash| hash.as_str().bytes().all(|byte| byte == b'0'))
        {
            return Err(AccumulatorPreparationInputError::PresentZero);
        }
        if predecessor == successor {
            return Err(AccumulatorPreparationInputError::NoChange);
        }
        Ok(Self {
            key,
            predecessor,
            successor,
        })
    }

    /// Return the exact hashed logical key.
    #[must_use]
    pub const fn key(&self) -> &ArtifactDigest {
        &self.key
    }

    /// Return the expected prior value, or explicit absence.
    #[must_use]
    pub const fn predecessor(&self) -> Option<&ArtifactDigest> {
        self.predecessor.as_ref()
    }

    /// Return the replacement value, or explicit removal.
    #[must_use]
    pub const fn successor(&self) -> Option<&ArtifactDigest> {
        self.successor.as_ref()
    }
}

/// A request cannot be prepared because its finite input is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AccumulatorPreparationInputError {
    /// A finite producer-selected ceiling is zero or exceeded.
    #[error("accumulator preparation limit is zero or exceeded")]
    Bounds,
    /// The tree's absent-value sentinel was supplied as a present hash.
    #[error("a present accumulator leaf cannot have an all-zero hash")]
    PresentZero,
    /// A supplied leaf has identical prior and replacement values.
    #[error("accumulator leaf does not change")]
    NoChange,
    /// Leaf keys are duplicated or not in ascending byte order.
    #[error("accumulator leaf keys must be strictly ordered")]
    Order,
    /// A successor request contains no logical change.
    #[error("successor accumulator preparation has no logical changes")]
    Empty,
    /// An initial insertion asserted present logical predecessor values.
    #[error("initial accumulator preparation contains predecessor leaves")]
    InitialPredecessor,
    /// The proposed mutation belongs to another store/incarnation, or its
    /// observed transaction head predates the retained checkpoint.
    #[error("accumulator preparation transaction does not match its store")]
    TransactionFence,
}

/// Attempt and retained-data ceilings supplied by the admitted producer profile.
/// These are validation inputs, not a new persisted policy or default quota.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccumulatorPreparationLimits {
    maximum_leaf_changes: usize,
    maximum_reads: usize,
    maximum_writes: usize,
    maximum_positions: usize,
    maximum_proof_bytes: usize,
}

impl AccumulatorPreparationLimits {
    /// Bound both trees together; repeated attempts still consume their budget.
    ///
    /// # Errors
    /// Rejects a zero limit. The adapter must enforce these during preparation,
    /// before performing the excess attempt or retaining excess output.
    pub fn new(
        maximum_leaf_changes: usize,
        maximum_reads: usize,
        maximum_writes: usize,
        maximum_positions: usize,
        maximum_proof_bytes: usize,
    ) -> Result<Self, AccumulatorPreparationInputError> {
        if [
            maximum_leaf_changes,
            maximum_reads,
            maximum_writes,
            maximum_positions,
            maximum_proof_bytes,
        ]
        .contains(&0)
        {
            return Err(AccumulatorPreparationInputError::Bounds);
        }
        Ok(Self {
            maximum_leaf_changes,
            maximum_reads,
            maximum_writes,
            maximum_positions,
            maximum_proof_bytes,
        })
    }

    /// Maximum logical replacements across both trees.
    #[must_use]
    pub const fn maximum_leaf_changes(self) -> usize {
        self.maximum_leaf_changes
    }
    /// Maximum attempted node/leaf reads, including overlay hits.
    #[must_use]
    pub const fn maximum_reads(self) -> usize {
        self.maximum_reads
    }
    /// Maximum attempted node/leaf writes, including overwritten intermediate values.
    #[must_use]
    pub const fn maximum_writes(self) -> usize {
        self.maximum_writes
    }
    /// Maximum distinct retained changed physical positions across both trees.
    #[must_use]
    pub const fn maximum_positions(self) -> usize {
        self.maximum_positions
    }
    /// Maximum compiled-proof bytes across both trees, excluding spare capacity
    /// and allocator overhead. A conservative encoded-size check before
    /// compilation can reject a proof whose eventual encoding would be smaller.
    #[must_use]
    pub const fn maximum_proof_bytes(self) -> usize {
        self.maximum_proof_bytes
    }
}

/// Changes to authenticated control state, not unresolved-custody classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateLeafChanges(Box<[AccumulatorLeafChange]>);

/// Changes supplied by the reconciliation-custody owner, never inferred from
/// an operation's terminal flag. An empty set preserves the admitted prior root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedCustodyLeafChanges(Box<[AccumulatorLeafChange]>);

fn ordered_changes(
    changes: Vec<AccumulatorLeafChange>,
    maximum_leaf_changes: usize,
) -> Result<Box<[AccumulatorLeafChange]>, AccumulatorPreparationInputError> {
    if maximum_leaf_changes == 0 || changes.len() > maximum_leaf_changes {
        return Err(AccumulatorPreparationInputError::Bounds);
    }
    if changes.windows(2).any(|pair| pair[0].key >= pair[1].key) {
        return Err(AccumulatorPreparationInputError::Order);
    }
    Ok(changes.into_boxed_slice())
}

impl StateLeafChanges {
    /// Retain ordered unique changes; the complete request enforces total bounds.
    ///
    /// # Errors
    /// Rejects excessive length before scanning or boxing, then duplicate or
    /// unordered keys. The limit does not undo the caller's allocation.
    pub fn new(
        changes: Vec<AccumulatorLeafChange>,
        maximum_leaf_changes: usize,
    ) -> Result<Self, AccumulatorPreparationInputError> {
        ordered_changes(changes, maximum_leaf_changes).map(Self)
    }
    /// Borrow the logical replacements in byte-key order.
    #[must_use]
    pub fn changes(&self) -> &[AccumulatorLeafChange] {
        &self.0
    }
}

impl UnresolvedCustodyLeafChanges {
    /// Retain only changes supplied by the unresolved-custody owner.
    ///
    /// # Errors
    /// Rejects excessive length before scanning or boxing, then duplicate or
    /// unordered keys. The limit does not undo the caller's allocation.
    pub fn new(
        changes: Vec<AccumulatorLeafChange>,
        maximum_leaf_changes: usize,
    ) -> Result<Self, AccumulatorPreparationInputError> {
        ordered_changes(changes, maximum_leaf_changes).map(Self)
    }
    /// Borrow the independently classified replacements in byte-key order.
    #[must_use]
    pub fn changes(&self) -> &[AccumulatorLeafChange] {
        &self.0
    }
}

/// One read-snapshot preparation request, before any successor has been signed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationAccumulatorPreparation {
    implementation: AccumulatorImplementation,
    basis: AccumulatorPreparationBasis,
    state: StateLeafChanges,
    unresolved: UnresolvedCustodyLeafChanges,
    limits: AccumulatorPreparationLimits,
    transaction: ControlTransactionPreparation,
}

impl OperationAccumulatorPreparation {
    /// Check combined bounds while preserving the two classification owners.
    /// An initial frontier may contain no leaves; every successor must change
    /// at least one logical leaf.
    ///
    /// # Errors
    /// Rejects an empty successor, count overflow, excessive logical changes,
    /// or a transaction proposal outside the retained store/checkpoint lineage.
    pub fn new(
        basis: AccumulatorPreparationBasis,
        state: StateLeafChanges,
        unresolved: UnresolvedCustodyLeafChanges,
        limits: AccumulatorPreparationLimits,
        implementation: AccumulatorImplementation,
        transaction: ControlTransactionPreparation,
    ) -> Result<Self, AccumulatorPreparationInputError> {
        if transaction.store() != basis.store()
            || matches!(&basis, AccumulatorPreparationBasis::Successor(prior)
                if prior.checkpoint().committed_transaction_sequence().get() > transaction.expected().get())
        {
            return Err(AccumulatorPreparationInputError::TransactionFence);
        }
        if matches!(&basis, AccumulatorPreparationBasis::Initial(_))
            && state
                .0
                .iter()
                .chain(&unresolved.0)
                .any(|change| change.predecessor().is_some())
        {
            return Err(AccumulatorPreparationInputError::InitialPredecessor);
        }
        let total = state
            .0
            .len()
            .checked_add(unresolved.0.len())
            .ok_or(AccumulatorPreparationInputError::Bounds)?;
        if total == 0 && matches!(&basis, AccumulatorPreparationBasis::Successor(_)) {
            return Err(AccumulatorPreparationInputError::Empty);
        }
        if total > limits.maximum_leaf_changes {
            return Err(AccumulatorPreparationInputError::Bounds);
        }
        Ok(Self {
            implementation,
            basis,
            state,
            unresolved,
            limits,
            transaction,
        })
    }
    /// Borrow the closed initial-domain or admitted predecessor basis.
    #[must_use]
    pub const fn basis(&self) -> &AccumulatorPreparationBasis {
        &self.basis
    }

    /// Borrow the full optimistic store fence covered by request/result equality.
    #[must_use]
    pub const fn transaction(&self) -> &ControlTransactionPreparation {
        &self.transaction
    }
    /// Borrow full-state changes.
    #[must_use]
    pub const fn state(&self) -> &StateLeafChanges {
        &self.state
    }
    /// Borrow independently classified unresolved-custody changes.
    #[must_use]
    pub const fn unresolved(&self) -> &UnresolvedCustodyLeafChanges {
        &self.unresolved
    }
    /// Read the combined preparation budgets.
    #[must_use]
    pub const fn limits(&self) -> AccumulatorPreparationLimits {
        self.limits
    }

    /// Return the exact mechanics requested after application profile admission.
    #[must_use]
    pub const fn implementation(&self) -> AccumulatorImplementation {
        self.implementation
    }
}

/// Preparation ended without any database mutation or usable checkpoint.
#[derive(Debug, thiserror::Error)]
pub enum OperationAccumulatorPreparationError {
    /// The requested computation reached a producer-selected work/data ceiling.
    /// This does not imply that stored state is corrupt.
    #[error("accumulator preparation reached its work or data limit")]
    Limit,
    /// Physical comparison, decoding, or authenticated-path verification failed.
    #[error(transparent)]
    Store(#[from] ControlStoreError),
}

/// Owned verified tree changes, with no database lock or signer handle retained.
/// The application must check these roots against the admitted signed successor
/// before consuming this result into its atomic operation candidate.
#[derive(Debug, PartialEq, Eq)]
pub struct PreparedOperationAccumulator {
    request: OperationAccumulatorPreparation,
    state: StateAccumulatorBatch,
    unresolved: UnresolvedCustodyAccumulatorBatch,
}

impl PreparedOperationAccumulator {
    /// Retain adapter-verified output and its complete bounded request.
    /// Request equality is correlation, not independent Merkle verification.
    ///
    /// # Errors
    /// Rejects unlike predecessor roots, nonempty initial predecessor state,
    /// an invented update for an empty initial request, or an empty successor.
    /// This constructor checks representation, not the adapter's Merkle proof.
    pub fn new(
        request: OperationAccumulatorPreparation,
        state: StateAccumulatorBatch,
        unresolved: UnresolvedCustodyAccumulatorBatch,
    ) -> Result<Self, AccumulatorBatchError> {
        let roots_match = match request.basis() {
            AccumulatorPreparationBasis::Initial(_) => {
                [state.predecessor_root(), unresolved.predecessor_root()]
                    .into_iter()
                    .all(|root| root.as_str().bytes().all(|byte| byte == b'0'))
            }
            AccumulatorPreparationBasis::Successor(predecessor) => {
                state.predecessor_root() == predecessor.state_root()
                    && unresolved.predecessor_root() == predecessor.unresolved_root()
            }
        };
        if !roots_match {
            return Err(AccumulatorBatchError::PredecessorRoots);
        }
        if matches!(request.basis(), AccumulatorPreparationBasis::Initial(_))
            && state
                .mutations()
                .iter()
                .chain(unresolved.mutations())
                .any(|mutation| mutation.predecessor().is_some())
        {
            return Err(AccumulatorBatchError::InitialPredecessor);
        }
        let request_is_empty =
            request.state().changes().is_empty() && request.unresolved().changes().is_empty();
        let result_is_empty = state.mutations().is_empty() && unresolved.mutations().is_empty();
        if request_is_empty
            && (!result_is_empty
                || [
                    state.predecessor_root(),
                    state.successor_root(),
                    unresolved.predecessor_root(),
                    unresolved.successor_root(),
                ]
                .into_iter()
                .any(|root| !root.as_str().bytes().all(|byte| byte == b'0')))
        {
            return Err(AccumulatorBatchError::EmptyRootChange);
        }
        if !request_is_empty && result_is_empty {
            return Err(AccumulatorBatchError::EmptyOperation);
        }
        Ok(Self {
            request,
            state,
            unresolved,
        })
    }
    /// Borrow the exact requested changes, tree roles, limits and coordinates.
    /// The caller compares this with its request before accepting the roots.
    #[must_use]
    pub const fn request(&self) -> &OperationAccumulatorPreparation {
        &self.request
    }

    /// Borrow the exact retained phase and coordinates for checkpoint assembly.
    #[must_use]
    pub const fn basis(&self) -> &AccumulatorPreparationBasis {
        self.request.basis()
    }
    /// Borrow the root calculated for the full-state successor.
    #[must_use]
    pub const fn state_root(&self) -> &ArtifactDigest {
        self.state.successor_root()
    }
    /// Borrow the root calculated for unresolved custody.
    #[must_use]
    pub const fn unresolved_root(&self) -> &ArtifactDigest {
        self.unresolved.successor_root()
    }

    /// Bind physical changes to the application's already-admitted checkpoint.
    /// This cannot change the retained store, profile, domain, or prior roots.
    ///
    /// # Errors
    /// Rejects a head outside the selected domain/epoch, outside sequence one
    /// for Initial, or not advancing the retained checkpoint exactly once.
    /// The final custody transaction still compares all predecessor state again.
    pub fn bind_successor(
        self,
        successor: ControlCheckpointHead,
    ) -> Result<OperationAccumulatorBatch, AccumulatorBatchError> {
        let binding = match self.request.basis {
            AccumulatorPreparationBasis::Initial(initial) => {
                if successor.checkpoint_domain_id() != &initial.domain
                    || successor.domain_epoch() != initial.epoch
                {
                    return Err(AccumulatorBatchError::CheckpointPair);
                }
                AccumulatorCheckpointBinding::new(
                    initial.store,
                    initial.profile_ref,
                    initial.profile_sha256,
                    AccumulatorCheckpointPhase::Initial,
                    successor,
                    self.request.transaction,
                )?
            }
            AccumulatorPreparationBasis::Successor(prior) => AccumulatorCheckpointBinding::new(
                prior.store,
                prior.profile_ref,
                prior.profile_sha256,
                AccumulatorCheckpointPhase::Successor(prior.checkpoint),
                successor,
                self.request.transaction,
            )?,
        };
        OperationAccumulatorBatch::new(binding, self.state, self.unresolved)
    }
}
