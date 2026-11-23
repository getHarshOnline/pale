//! Finite physical accumulator mutations accompanying operation custody.
//!
//! Application construction owns authenticated roots and profile meaning.
//! These values retain exact physical preimages; they do not admit signatures.

use pale_core::records::{ArtifactDigest, CheckpointProfileDigest, ResidentRecordRefV1};

use crate::{AuthorityStoreCoordinate, ControlCheckpointHead, ControlTransactionPreparation};

pub(crate) mod membership;
pub(crate) mod preparation;

/// One typed physical position in a selected accumulator tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccumulatorPosition {
    /// A branch at an exact height and path.
    Branch {
        /// Height within the fixed-width trie.
        height: u8,
        /// Exact hashed branch path.
        path: [u8; 32],
    },
    /// A leaf at an exact hashed key.
    Leaf([u8; 32]),
}

/// Physical batch rejection before storage access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AccumulatorBatchError {
    /// A selected validation ceiling is zero or exceeded.
    #[error("accumulator validation bound is invalid or exceeded")]
    Bounds,
    /// Present bytes cannot represent the selected position.
    #[error("accumulator position has invalid byte length")]
    ByteLength,
    /// A mutation repeats its exact predecessor.
    #[error("accumulator mutation does not change its position")]
    NoChange,
    /// Positions are not strictly ordered and unique.
    #[error("accumulator positions are not strictly ordered")]
    Order,
    /// An untouched tree cannot change its root.
    #[error("empty accumulator mutation set changes its root")]
    EmptyRootChange,
    /// No tree contributes a physical change.
    #[error("operation accumulator batch is empty")]
    EmptyOperation,
    /// Prepared trees do not retain the selected predecessor roots.
    #[error("accumulator predecessor roots do not match the selected checkpoint")]
    PredecessorRoots,
    /// Checkpoint heads do not form one exact domain succession.
    #[error("accumulator checkpoint pair is not an exact successor")]
    CheckpointPair,
    /// Initial custody cannot replace a prior root or a present position.
    #[error("initial accumulator batch contains predecessor state")]
    InitialPredecessor,
    /// A prepared write or signed checkpoint names a different transaction.
    #[error("accumulator transaction fence does not match its checkpoint")]
    TransactionFence,
}

/// Exact before/after bytes for one touched position; absence is explicit.
#[derive(Clone, PartialEq, Eq)]
pub struct AccumulatorMutation {
    position: AccumulatorPosition,
    predecessor: Option<Box<[u8]>>,
    successor: Option<Box<[u8]>>,
}

impl std::fmt::Debug for AccumulatorMutation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AccumulatorMutation")
            .field("position", &self.position)
            .field(
                "predecessor_bytes",
                &self.predecessor.as_ref().map(|bytes| bytes.len()),
            )
            .field(
                "successor_bytes",
                &self.successor.as_ref().map(|bytes| bytes.len()),
            )
            .finish()
    }
}

impl AccumulatorMutation {
    /// Validate finite bytes against a producer-selected ceiling, not a stored quota.
    /// The adapter additionally validates its selected concrete branch codec.
    ///
    /// # Errors
    /// Rejects zero/over-limit lengths, non-32-byte leaves, and unchanged values.
    pub fn new(
        position: AccumulatorPosition,
        predecessor: Option<Vec<u8>>,
        successor: Option<Vec<u8>>,
        maximum_node_bytes: usize,
    ) -> Result<Self, AccumulatorBatchError> {
        if maximum_node_bytes == 0 {
            return Err(AccumulatorBatchError::Bounds);
        }
        for bytes in predecessor.iter().chain(successor.iter()) {
            if bytes.len() > maximum_node_bytes {
                return Err(AccumulatorBatchError::Bounds);
            }
            if bytes.is_empty()
                || (matches!(position, AccumulatorPosition::Leaf(_)) && bytes.len() != 32)
            {
                return Err(AccumulatorBatchError::ByteLength);
            }
        }
        if predecessor == successor {
            return Err(AccumulatorBatchError::NoChange);
        }
        Ok(Self {
            position,
            predecessor: predecessor.map(Vec::into_boxed_slice),
            successor: successor.map(Vec::into_boxed_slice),
        })
    }
    /// Return the exact physical position.
    #[must_use]
    pub const fn position(&self) -> AccumulatorPosition {
        self.position
    }
    /// Borrow the asserted predecessor bytes, or explicit absence.
    #[must_use]
    pub fn predecessor(&self) -> Option<&[u8]> {
        self.predecessor.as_deref()
    }
    /// Borrow the replacement bytes, or explicit removal.
    #[must_use]
    pub fn successor(&self) -> Option<&[u8]> {
        self.successor.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TreeBatch {
    predecessor_root: ArtifactDigest,
    successor_root: ArtifactDigest,
    mutations: Box<[AccumulatorMutation]>,
}

impl TreeBatch {
    fn new(
        predecessor_root: ArtifactDigest,
        successor_root: ArtifactDigest,
        mutations: Vec<AccumulatorMutation>,
        maximum_positions: usize,
    ) -> Result<Self, AccumulatorBatchError> {
        if maximum_positions == 0 || mutations.len() > maximum_positions {
            return Err(AccumulatorBatchError::Bounds);
        }
        if mutations
            .windows(2)
            .any(|pair| pair[0].position >= pair[1].position)
        {
            return Err(AccumulatorBatchError::Order);
        }
        if mutations.is_empty() && predecessor_root != successor_root {
            return Err(AccumulatorBatchError::EmptyRootChange);
        }
        Ok(Self {
            predecessor_root,
            successor_root,
            mutations: mutations.into_boxed_slice(),
        })
    }
}

/// Physical mutations in the full control-state tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateAccumulatorBatch(TreeBatch);

impl StateAccumulatorBatch {
    /// Retain one ordered state-tree update and its application-projected roots.
    ///
    /// # Errors
    /// Rejects invalid count ceilings, duplicate/unordered positions, or changed empty roots.
    pub fn new(
        predecessor_root: ArtifactDigest,
        successor_root: ArtifactDigest,
        mutations: Vec<AccumulatorMutation>,
        maximum_positions: usize,
    ) -> Result<Self, AccumulatorBatchError> {
        TreeBatch::new(
            predecessor_root,
            successor_root,
            mutations,
            maximum_positions,
        )
        .map(Self)
    }
    /// Borrow the exact predecessor root.
    #[must_use]
    pub const fn predecessor_root(&self) -> &ArtifactDigest {
        &self.0.predecessor_root
    }
    /// Borrow the exact successor root.
    #[must_use]
    pub const fn successor_root(&self) -> &ArtifactDigest {
        &self.0.successor_root
    }
    /// Borrow ordered unique physical mutations.
    #[must_use]
    pub fn mutations(&self) -> &[AccumulatorMutation] {
        &self.0.mutations
    }
}

/// Physical mutations in the separately qualified unresolved-custody tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedCustodyAccumulatorBatch(TreeBatch);

impl UnresolvedCustodyAccumulatorBatch {
    /// Retain one ordered unresolved-custody update and its projected roots.
    /// An empty set preserves the exact root; it does not assert an absent tree.
    ///
    /// # Errors
    /// Rejects invalid count ceilings, duplicate/unordered positions, or changed empty roots.
    pub fn new(
        predecessor_root: ArtifactDigest,
        successor_root: ArtifactDigest,
        mutations: Vec<AccumulatorMutation>,
        maximum_positions: usize,
    ) -> Result<Self, AccumulatorBatchError> {
        TreeBatch::new(
            predecessor_root,
            successor_root,
            mutations,
            maximum_positions,
        )
        .map(Self)
    }
    /// Borrow the exact predecessor root.
    #[must_use]
    pub const fn predecessor_root(&self) -> &ArtifactDigest {
        &self.0.predecessor_root
    }
    /// Borrow the exact successor root.
    #[must_use]
    pub const fn successor_root(&self) -> &ArtifactDigest {
        &self.0.successor_root
    }
    /// Borrow ordered unique physical mutations.
    #[must_use]
    pub fn mutations(&self) -> &[AccumulatorMutation] {
        &self.0.mutations
    }
}

/// Closed checkpoint phase; initial custody has no invented prior checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccumulatorCheckpointPhase {
    /// First checkpoint in an independently admitted, physically absent domain.
    Initial,
    /// Exact retained checkpoint whose state this batch replaces.
    Successor(ControlCheckpointHead),
}

/// Exact retained physical scope and checkpoint transition of a constructed batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccumulatorCheckpointBinding {
    store: AuthorityStoreCoordinate,
    profile_ref: ResidentRecordRefV1,
    profile_sha256: CheckpointProfileDigest,
    phase: AccumulatorCheckpointPhase,
    successor: ControlCheckpointHead,
    transaction: ControlTransactionPreparation,
}

impl AccumulatorCheckpointBinding {
    /// Bind independently admitted profile/checkpoint projections without re-admitting them.
    ///
    /// # Errors
    /// Rejects an initial head outside sequence one, or a pair that changes
    /// domain/epoch or does not advance exactly once. This does not prove absence.
    pub fn new(
        store: AuthorityStoreCoordinate,
        profile_ref: ResidentRecordRefV1,
        profile_sha256: CheckpointProfileDigest,
        phase: AccumulatorCheckpointPhase,
        successor: ControlCheckpointHead,
        transaction: ControlTransactionPreparation,
    ) -> Result<Self, AccumulatorBatchError> {
        let valid = match &phase {
            AccumulatorCheckpointPhase::Initial => successor.domain_sequence().get() == 1,
            AccumulatorCheckpointPhase::Successor(predecessor) => {
                successor.is_exact_successor_of(predecessor)
            }
        };
        if !valid {
            return Err(AccumulatorBatchError::CheckpointPair);
        }
        if transaction.store() != &store
            || matches!(&phase, AccumulatorCheckpointPhase::Successor(prior)
                if prior.committed_transaction_sequence().get() > transaction.expected().get())
            || transaction.successor() != successor.committed_transaction_sequence()
        {
            return Err(AccumulatorBatchError::TransactionFence);
        }
        Ok(Self {
            store,
            profile_ref,
            profile_sha256,
            phase,
            successor,
            transaction,
        })
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
    /// Borrow the exact profile digest.
    #[must_use]
    pub const fn profile_sha256(&self) -> &CheckpointProfileDigest {
        &self.profile_sha256
    }
    /// Borrow the exact closed initial or successor checkpoint phase.
    #[must_use]
    pub const fn phase(&self) -> &AccumulatorCheckpointPhase {
        &self.phase
    }
    /// Borrow the exact successor checkpoint.
    #[must_use]
    pub const fn successor(&self) -> &ControlCheckpointHead {
        &self.successor
    }

    /// Borrow the exact proposed store mutation retained before signing.
    #[must_use]
    pub const fn transaction(&self) -> &ControlTransactionPreparation {
        &self.transaction
    }
}

/// Mandatory physical accumulator contribution to one operation transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationAccumulatorBatch {
    binding: AccumulatorCheckpointBinding,
    state: StateAccumulatorBatch,
    unresolved: UnresolvedCustodyAccumulatorBatch,
}

impl OperationAccumulatorBatch {
    /// Retain both distinct tree projections for the same exact checkpoint pair.
    /// An initial checkpoint may retain two canonical empty trees. A successor
    /// must contain at least one physical mutation.
    ///
    /// # Errors
    /// Rejects an empty successor, an empty initial batch outside its two
    /// canonical empty trees, or initial mutations that assert predecessor bytes.
    pub fn new(
        binding: AccumulatorCheckpointBinding,
        state: StateAccumulatorBatch,
        unresolved: UnresolvedCustodyAccumulatorBatch,
    ) -> Result<Self, AccumulatorBatchError> {
        if matches!(binding.phase(), AccumulatorCheckpointPhase::Initial)
            && (![state.predecessor_root(), unresolved.predecessor_root()]
                .into_iter()
                .all(|root| root.as_str().bytes().all(|byte| byte == b'0'))
                || state
                    .mutations()
                    .iter()
                    .chain(unresolved.mutations())
                    .any(|mutation| mutation.predecessor().is_some()))
        {
            return Err(AccumulatorBatchError::InitialPredecessor);
        }
        if state.mutations().is_empty() && unresolved.mutations().is_empty() {
            if !matches!(binding.phase(), AccumulatorCheckpointPhase::Initial) {
                return Err(AccumulatorBatchError::EmptyOperation);
            }
            if [
                state.predecessor_root(),
                state.successor_root(),
                unresolved.predecessor_root(),
                unresolved.successor_root(),
            ]
            .into_iter()
            .any(|root| !root.as_str().bytes().all(|byte| byte == b'0'))
            {
                return Err(AccumulatorBatchError::EmptyRootChange);
            }
        }
        Ok(Self {
            binding,
            state,
            unresolved,
        })
    }
    /// Borrow the exact physical binding.
    #[must_use]
    pub const fn binding(&self) -> &AccumulatorCheckpointBinding {
        &self.binding
    }
    /// Borrow full-state mutations.
    #[must_use]
    pub const fn state(&self) -> &StateAccumulatorBatch {
        &self.state
    }
    /// Borrow unresolved-custody mutations.
    #[must_use]
    pub const fn unresolved(&self) -> &UnresolvedCustodyAccumulatorBatch {
        &self.unresolved
    }
}
