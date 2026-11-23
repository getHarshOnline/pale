//! Generation-bound adaptive finite sets.

use crate::{GraphGeneration, NodeHandle};
use roaring::RoaringBitmap;

/// Physical encoding selected by one [`AdaptiveNodeSetPolicy`].
///
/// This value describes storage only. It carries no Package meaning, authority,
/// admission, or activation state, and callers cannot select it on graph-query
/// APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdaptiveNodeSetEncoding {
    /// Sorted, duplicate-free dense indices.
    SmallSorted,
    /// Generation-sized packed words.
    Dense,
    /// A compressed [`RoaringBitmap`].
    Roaring,
}

/// Explicit representation policy owned by one packed graph generation.
///
/// Sets with at most [`Self::small_cardinality_limit`] members use
/// [`AdaptiveNodeSetEncoding::SmallSorted`]. Larger sets use
/// [`AdaptiveNodeSetEncoding::Dense`] when their density reaches
/// [`Self::dense_min_density_permyriad`], and
/// [`AdaptiveNodeSetEncoding::Roaring`] otherwise. The density comparison is
/// exact integer arithmetic. In the rendered laws, `U` is the node universe,
/// `S` a member set, `delta` the permyriad density threshold, and `P` the
/// selected physical encoding; `A` and `B` are compatible logical sets:
///
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/math/adaptive-node-set-laws.display.mathml"))]
///
/// Thresholds are constructor inputs rather than guessed defaults. They select
/// a replaceable physical representation; the rendered law guarantees that
/// every policy preserves the logical member set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdaptiveNodeSetPolicy {
    small_cardinality_limit: u32,
    dense_min_density_permyriad: u16,
}

impl AdaptiveNodeSetPolicy {
    /// Construct an explicit generation-owned representation policy.
    ///
    /// `dense_min_density_permyriad` is a density in the inclusive range
    /// `0..=10_000`. No value is inferred from machine state or workspace
    /// configuration.
    ///
    /// # Errors
    ///
    /// Returns [`NodeSetError::InvalidDensityThreshold`] when the density is
    /// greater than `10_000`.
    pub const fn new(
        small_cardinality_limit: u32,
        dense_min_density_permyriad: u16,
    ) -> Result<Self, NodeSetError> {
        if dense_min_density_permyriad > 10_000 {
            return Err(NodeSetError::InvalidDensityThreshold {
                permyriad: dense_min_density_permyriad,
            });
        }
        Ok(Self {
            small_cardinality_limit,
            dense_min_density_permyriad,
        })
    }

    /// Return the inclusive cardinality bound for sorted storage.
    #[must_use]
    pub const fn small_cardinality_limit(self) -> u32 {
        self.small_cardinality_limit
    }

    /// Return the minimum dense-set density in parts per ten thousand.
    #[must_use]
    pub const fn dense_min_density_permyriad(self) -> u16 {
        self.dense_min_density_permyriad
    }

    fn encoding_for(self, universe: u32, cardinality: usize) -> AdaptiveNodeSetEncoding {
        if cardinality <= self.small_cardinality_limit as usize {
            return AdaptiveNodeSetEncoding::SmallSorted;
        }
        let cardinality = cardinality as u64;
        let dense = cardinality * 10_000
            >= u64::from(universe) * u64::from(self.dense_min_density_permyriad);
        if dense {
            AdaptiveNodeSetEncoding::Dense
        } else {
            AdaptiveNodeSetEncoding::Roaring
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Representation {
    SmallSorted(Box<[u32]>),
    Dense(Box<[u64]>),
    Roaring(RoaringBitmap),
}

/// Return the exact word count for a dense exclusive universe bound.
fn dense_word_count(universe: u32) -> Result<usize, NodeSetError> {
    usize::try_from(universe.div_ceil(64)).map_err(|_| NodeSetError::UniverseTooLarge)
}

/// A finite-set policy, construction, or algebra operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NodeSetError {
    /// A density threshold exceeds one complete universe.
    #[error("dense node-set density threshold {permyriad} exceeds 10000 permyriad")]
    InvalidDensityThreshold {
        /// Rejected density in parts per ten thousand.
        permyriad: u16,
    },
    /// A member lies outside the exclusive universe bound.
    #[error("node index {index} is outside universe {universe}")]
    OutOfUniverse {
        /// Rejected dense index.
        index: u32,
        /// Exclusive universe bound.
        universe: u32,
    },
    /// An operand belongs to another immutable graph generation.
    #[error("node-set algebra crosses graph generations")]
    CrossGeneration,
    /// Operands declare different node universes.
    #[error("node-set algebra crosses universe bounds")]
    CrossUniverse,
    /// Dense storage could not reserve the declared universe.
    #[error("dense node-set universe cannot be allocated")]
    UniverseTooLarge,
}

/// A finite node set bound to one graph generation and universe.
///
/// All encodings iterate in ascending order and obey the same algebra. Physical
/// representation is observationally irrelevant: equality and every operation
/// are defined over members, not bytes. For compatible sets `A` and `B`,
/// intersection is their meet:
///
/// These meet laws are rendered once on [`AdaptiveNodeSetPolicy`].
///
/// The set's policy is inherited from its owning [`crate::PackedGraph`]. Set
/// algebra reapplies that policy to the result, so representation does not
/// depend on a query caller's choice or on the left operand's current encoding.
#[derive(Debug, Clone)]
pub struct AdaptiveNodeSet {
    generation: GraphGeneration,
    universe: u32,
    policy: AdaptiveNodeSetPolicy,
    cardinality: usize,
    representation: Representation,
}

impl PartialEq for AdaptiveNodeSet {
    fn eq(&self, other: &Self) -> bool {
        self.generation == other.generation
            && self.universe == other.universe
            && self.iter().eq(other.iter())
    }
}

impl Eq for AdaptiveNodeSet {}

impl AdaptiveNodeSet {
    pub(crate) fn from_nodes(
        generation: GraphGeneration,
        universe: u32,
        nodes: impl IntoIterator<Item = NodeHandle>,
        policy: AdaptiveNodeSetPolicy,
    ) -> Result<Self, NodeSetError> {
        let mut indices = Vec::new();
        for node in nodes {
            if node.generation() != generation {
                return Err(NodeSetError::CrossGeneration);
            }
            indices.push(node.index());
        }
        Self::from_indices(generation, universe, indices, policy)
    }

    pub(crate) fn from_indices(
        generation: GraphGeneration,
        universe: u32,
        indices: impl IntoIterator<Item = u32>,
        policy: AdaptiveNodeSetPolicy,
    ) -> Result<Self, NodeSetError> {
        let mut indices = indices.into_iter().collect::<Vec<_>>();
        indices.sort_unstable();
        indices.dedup();
        if let Some(&index) = indices.iter().find(|&&index| index >= universe) {
            return Err(NodeSetError::OutOfUniverse { index, universe });
        }
        Self::from_canonical(generation, universe, indices, policy)
    }

    /// Return the owning graph generation.
    #[must_use]
    pub const fn generation(&self) -> GraphGeneration {
        self.generation
    }

    /// Return the exclusive node-universe bound.
    #[must_use]
    pub const fn universe(&self) -> u32 {
        self.universe
    }

    /// Return the selected physical encoding.
    #[must_use]
    pub fn encoding(&self) -> AdaptiveNodeSetEncoding {
        match &self.representation {
            Representation::SmallSorted(_) => AdaptiveNodeSetEncoding::SmallSorted,
            Representation::Dense(_) => AdaptiveNodeSetEncoding::Dense,
            Representation::Roaring(_) => AdaptiveNodeSetEncoding::Roaring,
        }
    }

    /// Return cardinality.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cardinality
    }

    /// Report whether the set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Test generation-bound membership.
    ///
    /// # Errors
    ///
    /// Returns [`NodeSetError::CrossGeneration`] for a foreign handle.
    pub fn contains(&self, node: NodeHandle) -> Result<bool, NodeSetError> {
        if node.generation() != self.generation {
            return Err(NodeSetError::CrossGeneration);
        }
        Ok(node.index() < self.universe && self.contains_index(node.index()))
    }

    /// Iterate present handles in ascending order.
    ///
    /// Small and Roaring encodings visit stored members. Dense encoding scans
    /// packed words while skipping zero bits rather than visiting each index.
    #[must_use]
    pub fn iter(&self) -> NodeSetIter<'_> {
        let inner = match &self.representation {
            Representation::SmallSorted(values) => NodeSetIterInner::Small(values.iter()),
            Representation::Dense(words) => NodeSetIterInner::Dense(DenseIter::new(words)),
            Representation::Roaring(bits) => NodeSetIterInner::Roaring(bits.iter()),
        };
        NodeSetIter {
            generation: self.generation,
            inner,
        }
    }

    /// Return the set union under this generation's representation policy.
    ///
    /// # Errors
    ///
    /// Rejects incompatible generations or universes.
    pub fn union(&self, other: &Self) -> Result<Self, NodeSetError> {
        self.combine(other, SetOperation::Union)
    }

    /// Return the set meet under this generation's representation policy.
    ///
    /// # Errors
    ///
    /// Rejects incompatible generations or universes.
    pub fn intersection(&self, other: &Self) -> Result<Self, NodeSetError> {
        self.combine(other, SetOperation::Intersection)
    }

    /// Return the set difference defined on [`AdaptiveNodeSetPolicy`].
    ///
    /// # Errors
    ///
    /// Rejects incompatible generations or universes.
    pub fn difference(&self, other: &Self) -> Result<Self, NodeSetError> {
        self.combine(other, SetOperation::Difference)
    }

    /// Return the exact universe-relative complement defined on
    /// [`AdaptiveNodeSetPolicy`].
    ///
    /// The result is constructed directly in its selected representation. It
    /// does not first materialize a second full-universe set, so reverse-impact
    /// partitioning allocates only the returned complement.
    ///
    /// # Errors
    ///
    /// Returns [`NodeSetError::UniverseTooLarge`] when the selected physical
    /// representation cannot reserve its bounded storage.
    pub(crate) fn complement(&self) -> Result<Self, NodeSetError> {
        let cardinality = self.universe as usize - self.len();
        let encoding = self.policy.encoding_for(self.universe, cardinality);
        let representation = match encoding {
            AdaptiveNodeSetEncoding::SmallSorted => Representation::SmallSorted(
                (0..self.universe)
                    .filter(|&index| !self.contains_index(index))
                    .collect::<Vec<_>>()
                    .into(),
            ),
            AdaptiveNodeSetEncoding::Dense => {
                let count = dense_word_count(self.universe)?;
                let mut words = Vec::new();
                words
                    .try_reserve_exact(count)
                    .map_err(|_| NodeSetError::UniverseTooLarge)?;
                words.resize(count, u64::MAX);
                if let Some(last) = words.last_mut() {
                    let used = self.universe % 64;
                    if used != 0 {
                        *last &= (1_u64 << used) - 1;
                    }
                }
                for member in self {
                    words[member.index() as usize / 64] &= !(1_u64 << (member.index() % 64));
                }
                Representation::Dense(words.into())
            }
            AdaptiveNodeSetEncoding::Roaring => Representation::Roaring(
                (0..self.universe)
                    .filter(|&index| !self.contains_index(index))
                    .collect(),
            ),
        };
        Ok(Self {
            generation: self.generation,
            universe: self.universe,
            policy: self.policy,
            cardinality,
            representation,
        })
    }

    /// Report whether every member is present in `other`.
    ///
    /// # Errors
    ///
    /// Rejects incompatible generations or universes.
    pub fn is_subset(&self, other: &Self) -> Result<bool, NodeSetError> {
        self.ensure_compatible(other)?;
        Ok(self.iter().all(|node| other.contains_index(node.index())))
    }

    /// Report whether the intersection is empty.
    ///
    /// # Errors
    ///
    /// Rejects incompatible generations or universes.
    pub fn is_disjoint(&self, other: &Self) -> Result<bool, NodeSetError> {
        self.ensure_compatible(other)?;
        let (smaller, larger) = if self.len() <= other.len() {
            (self, other)
        } else {
            (other, self)
        };
        Ok(smaller
            .iter()
            .all(|node| !larger.contains_index(node.index())))
    }

    fn combine(&self, other: &Self, operation: SetOperation) -> Result<Self, NodeSetError> {
        self.ensure_compatible(other)?;
        let representation = match (&self.representation, &other.representation) {
            (Representation::SmallSorted(left), Representation::SmallSorted(right)) => {
                Representation::SmallSorted(
                    merge_sorted(left.iter().copied(), right.iter().copied(), operation).into(),
                )
            }
            (Representation::Dense(left), Representation::Dense(right)) => Representation::Dense(
                left.iter()
                    .zip(right.iter())
                    .map(|(&left, &right)| operation.apply_word(left, right))
                    .collect::<Vec<_>>()
                    .into(),
            ),
            (Representation::Roaring(left), Representation::Roaring(right)) => {
                let mut result = left.clone();
                match operation {
                    SetOperation::Union => result |= right,
                    SetOperation::Intersection => result &= right,
                    SetOperation::Difference => result -= right,
                }
                Representation::Roaring(result)
            }
            _ => {
                let members = merge_sorted(
                    self.iter().map(NodeHandle::index),
                    other.iter().map(NodeHandle::index),
                    operation,
                );
                return Self::from_canonical(self.generation, self.universe, members, self.policy);
            }
        };
        let provisional = Self {
            generation: self.generation,
            universe: self.universe,
            policy: self.policy,
            cardinality: representation_cardinality(&representation),
            representation,
        };
        if provisional.encoding() == self.policy.encoding_for(self.universe, provisional.len()) {
            return Ok(provisional);
        }
        Self::from_canonical(
            self.generation,
            self.universe,
            provisional.iter().map(NodeHandle::index).collect(),
            self.policy,
        )
    }

    /// Materialize already canonical indices without re-sorting them.
    ///
    /// Callers must supply strictly ascending, duplicate-free indices below
    /// `universe`; debug builds assert that trusted kernel precondition.
    pub(crate) fn from_canonical(
        generation: GraphGeneration,
        universe: u32,
        indices: Vec<u32>,
        policy: AdaptiveNodeSetPolicy,
    ) -> Result<Self, NodeSetError> {
        let encoding = policy.encoding_for(universe, indices.len());
        Self::from_canonical_with_encoding(generation, universe, indices, policy, encoding)
    }

    fn from_canonical_with_encoding(
        generation: GraphGeneration,
        universe: u32,
        indices: Vec<u32>,
        policy: AdaptiveNodeSetPolicy,
        encoding: AdaptiveNodeSetEncoding,
    ) -> Result<Self, NodeSetError> {
        debug_assert!(indices.windows(2).all(|pair| pair[0] < pair[1]));
        debug_assert!(indices.last().is_none_or(|index| *index < universe));
        let cardinality = indices.len();
        match encoding {
            AdaptiveNodeSetEncoding::SmallSorted => Ok(Self {
                generation,
                universe,
                policy,
                cardinality,
                representation: Representation::SmallSorted(indices.into()),
            }),
            AdaptiveNodeSetEncoding::Roaring => Ok(Self {
                generation,
                universe,
                policy,
                cardinality,
                representation: Representation::Roaring(indices.into_iter().collect()),
            }),
            AdaptiveNodeSetEncoding::Dense => {
                let count = dense_word_count(universe)?;
                let mut words = Vec::new();
                words
                    .try_reserve_exact(count)
                    .map_err(|_| NodeSetError::UniverseTooLarge)?;
                words.resize(count, 0);
                for index in indices {
                    words[index as usize / 64] |= 1_u64 << (index % 64);
                }
                Ok(Self {
                    generation,
                    universe,
                    policy,
                    cardinality,
                    representation: Representation::Dense(words.into()),
                })
            }
        }
    }

    fn ensure_compatible(&self, other: &Self) -> Result<(), NodeSetError> {
        if self.generation != other.generation {
            return Err(NodeSetError::CrossGeneration);
        }
        if self.universe != other.universe {
            return Err(NodeSetError::CrossUniverse);
        }
        Ok(())
    }

    fn contains_index(&self, index: u32) -> bool {
        match &self.representation {
            Representation::SmallSorted(values) => values.binary_search(&index).is_ok(),
            Representation::Dense(words) => {
                words[index as usize / 64] & (1_u64 << (index % 64)) != 0
            }
            Representation::Roaring(bits) => bits.contains(index),
        }
    }
}

fn representation_cardinality(representation: &Representation) -> usize {
    match representation {
        Representation::SmallSorted(values) => values.len(),
        Representation::Dense(words) => words
            .iter()
            .map(|word| usize::try_from(word.count_ones()).expect("u32 fits usize"))
            .sum(),
        Representation::Roaring(bits) => usize::try_from(bits.len()).unwrap_or(usize::MAX),
    }
}

#[derive(Clone, Copy)]
enum SetOperation {
    Union,
    Intersection,
    Difference,
}

impl SetOperation {
    fn apply_word(self, left: u64, right: u64) -> u64 {
        match self {
            Self::Union => left | right,
            Self::Intersection => left & right,
            Self::Difference => left & !right,
        }
    }
}

fn merge_sorted(
    left: impl Iterator<Item = u32>,
    right: impl Iterator<Item = u32>,
    operation: SetOperation,
) -> Vec<u32> {
    let mut left = left.peekable();
    let mut right = right.peekable();
    let mut output = Vec::new();
    while left.peek().is_some() || right.peek().is_some() {
        match (left.peek().copied(), right.peek().copied()) {
            (Some(a), Some(b)) if a == b => {
                if !matches!(operation, SetOperation::Difference) {
                    output.push(a);
                }
                left.next();
                right.next();
            }
            (Some(a), Some(b)) if a < b => {
                if !matches!(operation, SetOperation::Intersection) {
                    output.push(a);
                }
                left.next();
            }
            (Some(_) | None, Some(b)) => {
                if matches!(operation, SetOperation::Union) {
                    output.push(b);
                }
                right.next();
            }
            (Some(a), None) => {
                if !matches!(operation, SetOperation::Intersection) {
                    output.push(a);
                }
                left.next();
            }
            (None, None) => break,
        }
    }
    output
}

/// Ascending iterator over generation-bound handles.
///
/// The representation-specific iteration bounds are rendered on
/// [`AdaptiveNodeSetPolicy`]. Dense storage additionally skips zero words.
pub struct NodeSetIter<'a> {
    generation: GraphGeneration,
    inner: NodeSetIterInner<'a>,
}

enum NodeSetIterInner<'a> {
    Small(std::slice::Iter<'a, u32>),
    Dense(DenseIter<'a>),
    Roaring(roaring::bitmap::Iter<'a>),
}

impl Iterator for NodeSetIter<'_> {
    type Item = NodeHandle;
    fn next(&mut self) -> Option<Self::Item> {
        let index = match &mut self.inner {
            NodeSetIterInner::Small(values) => values.next().copied(),
            NodeSetIterInner::Dense(values) => values.next(),
            NodeSetIterInner::Roaring(values) => values.next(),
        }?;
        Some(NodeHandle::new(self.generation, index))
    }
}

impl<'a> IntoIterator for &'a AdaptiveNodeSet {
    type Item = NodeHandle;
    type IntoIter = NodeSetIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

struct DenseIter<'a> {
    words: &'a [u64],
    word_index: usize,
    remaining: u64,
}

impl<'a> DenseIter<'a> {
    fn new(words: &'a [u64]) -> Self {
        Self {
            words,
            word_index: 0,
            remaining: 0,
        }
    }
}

impl Iterator for DenseIter<'_> {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.remaining != 0 {
                let bit = self.remaining.trailing_zeros();
                self.remaining &= self.remaining - 1;
                let base = u32::try_from((self.word_index - 1) * 64)
                    .expect("dense storage is bounded by the u32 node universe");
                return Some(base + bit);
            }
            if self.word_index == self.words.len() {
                return None;
            }
            self.remaining = self.words[self.word_index];
            self.word_index += 1;
        }
    }
}

#[cfg(test)]
#[path = "../tests/internal/node_set.rs"]
mod tests;
