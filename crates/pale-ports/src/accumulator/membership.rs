//! Bounded membership at one exact current checkpoint, without tree mutation.

use pale_core::records::ArtifactDigest;

use super::preparation::{AccumulatorImplementation, OperationAccumulatorCheckpoint};

/// A present control-state leaf expected by its application projection owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccumulatorMembershipLeaf {
    key: ArtifactDigest,
    value: ArtifactDigest,
}

impl AccumulatorMembershipLeaf {
    /// Bind one expected present leaf. The selected trie reserves zero for absence.
    ///
    /// # Errors
    /// Rejects the absent-value sentinel as a present value.
    pub fn new(
        key: ArtifactDigest,
        value: ArtifactDigest,
    ) -> Result<Self, AccumulatorMembershipInputError> {
        if value.as_str().bytes().all(|byte| byte == b'0') {
            return Err(AccumulatorMembershipInputError::PresentZero);
        }
        Ok(Self { key, value })
    }

    /// Borrow the full hashed logical key.
    #[must_use]
    pub const fn key(&self) -> &ArtifactDigest {
        &self.key
    }

    /// Borrow the exact expected present value.
    #[must_use]
    pub const fn value(&self) -> &ArtifactDigest {
        &self.value
    }
}

/// Trie-position attempt and retained proof ceilings for read-only membership.
/// Fixed store/header/head/frontier/record queries run outside the trie-read
/// ceiling. These limits do not bound total SQL work, `SQLite` cache pages,
/// allocator overhead or I/O latency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccumulatorMembershipLimits {
    maximum_leaves: usize,
    maximum_reads: usize,
    maximum_proof_bytes: usize,
}

impl AccumulatorMembershipLimits {
    /// Preserve all raw ceilings, without a normalization or fallback quota.
    ///
    /// # Errors
    /// Rejects a zero ceiling.
    pub fn new(
        maximum_leaves: usize,
        maximum_reads: usize,
        maximum_proof_bytes: usize,
    ) -> Result<Self, AccumulatorMembershipInputError> {
        if [maximum_leaves, maximum_reads, maximum_proof_bytes].contains(&0) {
            return Err(AccumulatorMembershipInputError::Bounds);
        }
        Ok(Self {
            maximum_leaves,
            maximum_reads,
            maximum_proof_bytes,
        })
    }
    /// Maximum selected present leaves.
    #[must_use]
    pub const fn maximum_leaves(self) -> usize {
        self.maximum_leaves
    }
    /// Maximum attempted trie-position reads, including misses. Fixed custody
    /// metadata and immutable-record checks are outside this count.
    #[must_use]
    pub const fn maximum_reads(self) -> usize {
        self.maximum_reads
    }
    /// Maximum compiled proof bytes, excluding allocator spare capacity.
    #[must_use]
    pub const fn maximum_proof_bytes(self) -> usize {
        self.maximum_proof_bytes
    }
}

/// Invalid finite membership input, rejected before custody access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AccumulatorMembershipInputError {
    /// An input ceiling is zero or exceeded, or no leaves were selected.
    #[error("invalid accumulator membership bound")]
    Bounds,
    /// A present leaf cannot use the absent-value sentinel.
    #[error("present accumulator membership value is zero")]
    PresentZero,
    /// Keys must be strictly ordered, with no repeated position.
    #[error("accumulator membership keys are not strictly ordered")]
    Order,
}

/// Complete read request under one checkpoint and one closed implementation.
/// Checkpoint construction, signatures and independent anchor truth are not
/// established by constructing this value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationAccumulatorMembership {
    checkpoint: OperationAccumulatorCheckpoint,
    leaves: Box<[AccumulatorMembershipLeaf]>,
    limits: AccumulatorMembershipLimits,
    implementation: AccumulatorImplementation,
}

impl OperationAccumulatorMembership {
    /// Retain exact state-tree expectations and all raw bounds.
    ///
    /// # Errors
    /// Rejects empty/excessive leaves before scanning or boxing, then unordered
    /// or repeated keys. This cannot undo the caller's input allocation.
    pub fn new(
        checkpoint: OperationAccumulatorCheckpoint,
        leaves: Vec<AccumulatorMembershipLeaf>,
        limits: AccumulatorMembershipLimits,
        implementation: AccumulatorImplementation,
    ) -> Result<Self, AccumulatorMembershipInputError> {
        if leaves.is_empty() || leaves.len() > limits.maximum_leaves() {
            return Err(AccumulatorMembershipInputError::Bounds);
        }
        if leaves.windows(2).any(|pair| pair[0].key >= pair[1].key) {
            return Err(AccumulatorMembershipInputError::Order);
        }
        Ok(Self {
            checkpoint,
            leaves: leaves.into_boxed_slice(),
            limits,
            implementation,
        })
    }
    /// Borrow the complete selected checkpoint, including both roots.
    #[must_use]
    pub const fn checkpoint(&self) -> &OperationAccumulatorCheckpoint {
        &self.checkpoint
    }
    /// Borrow the selected state leaves in hashed-key order.
    #[must_use]
    pub fn leaves(&self) -> &[AccumulatorMembershipLeaf] {
        &self.leaves
    }
    /// Return the exact raw attempt and proof ceilings.
    #[must_use]
    pub const fn limits(&self) -> AccumulatorMembershipLimits {
        self.limits
    }
    /// Return supported native mechanics, not profile authority.
    #[must_use]
    pub const fn implementation(&self) -> AccumulatorImplementation {
        self.implementation
    }
}

/// Adapter report that the complete request matched one current read snapshot.
/// Application equality checks establish request/result correlation, not an
/// independent proof that an arbitrary port implementation is correct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedOperationAccumulatorMembership(OperationAccumulatorMembership);

impl VerifiedOperationAccumulatorMembership {
    /// Retain a request after the adapter has actually verified its proof.
    /// This constructor performs no signature, root or anchor verification.
    #[must_use]
    pub const fn new(request: OperationAccumulatorMembership) -> Self {
        Self(request)
    }
    /// Borrow the complete request that the adapter reports verifying.
    #[must_use]
    pub const fn request(&self) -> &OperationAccumulatorMembership {
        &self.0
    }
}
