//! Deterministic synthetic corpus and reference mechanics for resident-kernel benchmarks.
//!
//! This module intentionally contains no test bodies and no timing policy. The
//! integration test and feature-gated benchmark include it independently so
//! both exercise the same generated declarations and naive reference model.

use std::collections::{BTreeSet, VecDeque};

use pale_kernel::{
    AdaptiveNodeSetPolicy, Direction, GraphEdge, GraphNode, NodeHandle, PackedGraph,
};

/// Fixed corpus seed; it is reproducibility data, not an authority coordinate.
pub(crate) const SYNTHETIC_SEED: u64 = 0x4a41_5256_4953_0001;

/// Full characterization universes, independent of any owner-qualified corpus.
pub(crate) const CHARACTERIZATION_SET_UNIVERSES: [u32; 4] = [256, 4_096, 65_536, 262_144];

/// Full characterization cardinalities crossed with every universe and shape.
pub(crate) const CHARACTERIZATION_SET_CARDINALITIES: [Cardinality; 8] = [
    Cardinality::Absolute(1),
    Cardinality::Absolute(4),
    Cardinality::Absolute(32),
    Cardinality::Permyriad(10),
    Cardinality::Permyriad(100),
    Cardinality::Permyriad(1_000),
    Cardinality::Permyriad(5_000),
    Cardinality::Permyriad(9_000),
];

/// Full characterization membership shapes.
pub(crate) const CHARACTERIZATION_SET_PATTERNS: [SetPattern; 3] = [
    SetPattern::Clustered,
    SetPattern::Strided,
    SetPattern::FixedScatter,
];

/// Full characterization requested overlaps in parts per ten thousand.
pub(crate) const CHARACTERIZATION_SET_OVERLAPS: [u16; 3] = [0, 5_000, 9_000];

/// Exact Cartesian fixture count emitted by [`characterization_set_cases`].
pub(crate) const CHARACTERIZATION_SET_FIXTURE_COUNT: usize = CHARACTERIZATION_SET_UNIVERSES.len()
    * CHARACTERIZATION_SET_CARDINALITIES.len()
    * CHARACTERIZATION_SET_PATTERNS.len()
    * CHARACTERIZATION_SET_OVERLAPS.len();

/// Number of operations recorded for each adaptive-set fixture.
pub(crate) const SET_OPERATIONS_PER_FIXTURE: usize = 10;

/// Number of operations recorded for each graph fixture before acceleration.
pub(crate) const GRAPH_OPERATIONS_PER_FIXTURE: usize = 15;

/// Number of operations recorded for each typed-arena fixture.
pub(crate) const ARENA_OPERATIONS_PER_FIXTURE: usize = 8;

/// Number of operations recorded for each process-local wake-ring fixture.
pub(crate) const WAKE_RING_OPERATIONS_PER_FIXTURE: usize = 6;

/// Exact benchmark row count for the selected corpus and feature set.
///
/// The count is an evidence-integrity invariant, not a performance result. A
/// full run crosses 288 set fixtures, 18 graph fixtures, three arena fixtures,
/// and three wake-ring fixtures. The graph rows include matched cold, cached,
/// topologically rejected, and full-reference point queries. `GraphBLAS` adds
/// one derived-index
/// construction, six accelerated batch-query rows, and six matched canonical-
/// CPU rows per full graph fixture. Smoke selects one fixture per family and
/// only the eight-seed forward/reverse pair for each backend.
pub(crate) const fn expected_benchmark_rows(
    smoke: bool,
    graphblas: bool,
    graph_only: bool,
) -> usize {
    let (set_fixtures, graph_fixtures, arena_fixtures, wake_ring_fixtures, graphblas_operations) =
        if smoke {
            (1, 1, 1, 1, if graphblas { 5 } else { 0 })
        } else {
            (
                CHARACTERIZATION_SET_FIXTURE_COUNT,
                3 * 6,
                3,
                3,
                if graphblas { 13 } else { 0 },
            )
        };
    if graph_only {
        return graph_fixtures * (GRAPH_OPERATIONS_PER_FIXTURE + graphblas_operations);
    }
    set_fixtures * SET_OPERATIONS_PER_FIXTURE
        + graph_fixtures * (GRAPH_OPERATIONS_PER_FIXTURE + graphblas_operations)
        + arena_fixtures * ARENA_OPERATIONS_PER_FIXTURE
        + wake_ring_fixtures * WAKE_RING_OPERATIONS_PER_FIXTURE
}

/// Synthetic sorted-set cutoff used only to exercise representation mechanics.
const SYNTHETIC_SMALL_CARDINALITY_LIMIT: u32 = 32;

/// Synthetic dense-set threshold used only to exercise representation mechanics.
const SYNTHETIC_DENSE_MIN_DENSITY_PERMYRIAD: u16 = 5_000;

pub(crate) type SyntheticPackedGraph = PackedGraph<u32, u32, u64, EdgeKind, u64>;

/// Deterministic multi-field runtime body used only for synthetic arena characterization.
///
/// The shape exercises populated storage and mutation without selecting a
/// resident application body layout or assigning authority to benchmark data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SyntheticRuntimeBody {
    /// Stable synthetic identity for one generated body.
    pub(crate) identity: u64,
    /// Synthetic compiled-generation affinity used to vary the body shape.
    pub(crate) generation_affinity: u32,
    /// Mutable revision advanced by the churn case.
    pub(crate) revision: u64,
    /// Synthetic deadline coordinate; it carries no scheduling policy.
    pub(crate) deadline: u64,
    /// Fixed-width membership coordinates used only to model populated bodies.
    pub(crate) memberships: [u32; 4],
}

impl SyntheticRuntimeBody {
    /// Derive one deterministic body from its dense index and fixture size.
    pub(crate) fn for_index(index: u32, universe: u32) -> Self {
        Self {
            identity: SYNTHETIC_SEED ^ u64::from(index),
            generation_affinity: universe,
            revision: u64::from(index),
            deadline: u64::from(universe) + u64::from(index),
            memberships: [
                index,
                index.wrapping_add(1) % universe,
                index.wrapping_add(7) % universe,
                index.wrapping_mul(17) % universe,
            ],
        }
    }

    /// Advance the synthetic revision for a remove-and-reinsert operation.
    pub(crate) fn advance(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
}

/// Generate the complete deterministic body population for one arena fixture.
pub(crate) fn synthetic_runtime_bodies(size: u32) -> Vec<SyntheticRuntimeBody> {
    assert!(size > 0, "runtime benchmark arenas must be nonempty");
    (0..size)
        .map(|index| SyntheticRuntimeBody::for_index(index, size))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SetPattern {
    Clustered,
    Strided,
    FixedScatter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Cardinality {
    Absolute(u32),
    Permyriad(u16),
}

impl Cardinality {
    pub(crate) fn resolve(self, universe: u32) -> u32 {
        match self {
            Self::Absolute(value) => value.min(universe),
            Self::Permyriad(value) => {
                assert!(value <= 10_000, "density must be expressed in permyriad");
                let scaled = (u64::from(universe) * u64::from(value)).div_ceil(10_000);
                u32::try_from(scaled).expect("scaled cardinality cannot exceed its u32 universe")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SetPairMembers {
    pub(crate) left: Vec<u32>,
    pub(crate) right: Vec<u32>,
    pub(crate) requested_overlap_permyriad: u16,
    pub(crate) effective_overlap: u32,
}

pub(crate) struct SetFixture {
    pub(crate) graph: SyntheticPackedGraph,
    pub(crate) members: SetPairMembers,
}

impl SetFixture {
    pub(crate) fn new(
        universe: u32,
        cardinality: Cardinality,
        pattern: SetPattern,
        requested_overlap_permyriad: u16,
        policy: AdaptiveNodeSetPolicy,
    ) -> Self {
        let graph = PackedGraph::build(
            (0..universe)
                .map(|key| GraphNode { key, payload: key })
                .collect(),
            Vec::<GraphEdge<u64, u32, EdgeKind, u64>>::new(),
            policy,
        )
        .expect("synthetic set universe is a valid packed graph");
        let members = paired_set_members(
            universe,
            cardinality.resolve(universe),
            pattern,
            requested_overlap_permyriad,
        );
        Self { graph, members }
    }

    pub(crate) fn handles(&self, members: &[u32]) -> Vec<NodeHandle> {
        members
            .iter()
            .map(|&index| {
                self.graph
                    .node_at(index)
                    .expect("generated set member is inside its universe")
            })
            .collect()
    }
}

pub(crate) fn paired_set_members(
    universe: u32,
    cardinality: u32,
    pattern: SetPattern,
    requested_overlap_permyriad: u16,
) -> SetPairMembers {
    assert!(cardinality <= universe, "cardinality must fit its universe");
    assert!(
        requested_overlap_permyriad <= 10_000,
        "overlap must be expressed in permyriad"
    );
    let left = patterned_members(universe, cardinality, pattern, SYNTHETIC_SEED);
    let requested = u64::from(cardinality) * u64::from(requested_overlap_permyriad) / 10_000;
    let minimum = u32::try_from((u64::from(cardinality) * 2).saturating_sub(u64::from(universe)))
        .expect("minimum overlap cannot exceed one u32 cardinality");
    let effective_overlap = u32::try_from(requested)
        .expect("requested overlap is bounded by u32 cardinality")
        .max(minimum)
        .min(cardinality);

    let left_set = left.iter().copied().collect::<BTreeSet<_>>();
    let mut right = left
        .iter()
        .take(effective_overlap as usize)
        .copied()
        .collect::<BTreeSet<_>>();
    for candidate in patterned_indices(universe, SetPattern::FixedScatter, SYNTHETIC_SEED ^ 0x9e37)
    {
        if right.len() == cardinality as usize {
            break;
        }
        if !left_set.contains(&candidate) {
            right.insert(candidate);
        }
    }
    assert_eq!(right.len(), cardinality as usize);
    let right = right.into_iter().collect::<Vec<_>>();
    assert_eq!(
        left_set
            .intersection(&right.iter().copied().collect())
            .count(),
        effective_overlap as usize
    );
    SetPairMembers {
        left,
        right,
        requested_overlap_permyriad,
        effective_overlap,
    }
}

/// Enumerate the deterministic full Cartesian synthetic set grid.
///
/// The iterator crosses every declared universe, cardinality, membership shape,
/// and requested overlap. It characterizes mechanics only and cannot select an
/// [`AdaptiveNodeSetPolicy`] for an owner-qualified resident generation.
pub(crate) fn characterization_set_cases(
) -> impl Iterator<Item = (u32, Cardinality, SetPattern, u16)> {
    CHARACTERIZATION_SET_UNIVERSES
        .into_iter()
        .flat_map(|universe| {
            CHARACTERIZATION_SET_CARDINALITIES
                .into_iter()
                .flat_map(move |cardinality| {
                    CHARACTERIZATION_SET_PATTERNS
                        .into_iter()
                        .flat_map(move |pattern| {
                            CHARACTERIZATION_SET_OVERLAPS
                                .into_iter()
                                .map(move |overlap| (universe, cardinality, pattern, overlap))
                        })
                })
        })
}

pub(crate) fn patterned_members(
    universe: u32,
    cardinality: u32,
    pattern: SetPattern,
    seed: u64,
) -> Vec<u32> {
    patterned_indices(universe, pattern, seed)
        .take(cardinality as usize)
        .collect()
}

fn patterned_indices(
    universe: u32,
    pattern: SetPattern,
    seed: u64,
) -> Box<dyn Iterator<Item = u32>> {
    if universe == 0 {
        return Box::new(std::iter::empty());
    }
    match pattern {
        SetPattern::Clustered => {
            let offset = u32::try_from(seed % u64::from(universe)).expect("offset is in universe");
            Box::new((0..universe).map(move |index| (offset + index) % universe))
        }
        SetPattern::Strided => {
            let step = coprime_step(universe, 17);
            let offset = u32::try_from(seed % u64::from(universe)).expect("offset is in universe");
            Box::new((0..universe).map(move |index| {
                u32::try_from(
                    (u64::from(offset) + u64::from(index) * u64::from(step)) % u64::from(universe),
                )
                .expect("strided member remains in universe")
            }))
        }
        SetPattern::FixedScatter => {
            let step_seed = u32::try_from((seed >> 32) | 1).unwrap_or(u32::MAX);
            let step = coprime_step(universe, step_seed);
            let offset = u32::try_from(seed % u64::from(universe)).expect("offset is in universe");
            Box::new((0..universe).map(move |index| {
                u32::try_from(
                    (u64::from(offset) + u64::from(index) * u64::from(step)) % u64::from(universe),
                )
                .expect("scattered member remains in universe")
            }))
        }
    }
}

fn coprime_step(universe: u32, seed: u32) -> u32 {
    if universe <= 1 {
        return 1;
    }
    let mut step = (seed % universe).max(1);
    while greatest_common_divisor(step, universe) != 1 {
        step = step
            .checked_add(1)
            .filter(|value| *value < universe)
            .unwrap_or(1);
    }
    step
}

fn greatest_common_divisor(mut left: u32, mut right: u32) -> u32 {
    while right != 0 {
        (left, right) = (right, left % right);
    }
    left
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GraphPattern {
    Chain,
    Layered,
    Fanout,
    Disjoint,
    SccBand,
    Parallel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EdgeKind {
    Dependency,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SyntheticEdge {
    pub(crate) id: u64,
    pub(crate) source: u32,
    pub(crate) target: u32,
}

pub(crate) struct GraphFixture {
    pub(crate) graph: SyntheticPackedGraph,
    pub(crate) edges: Vec<SyntheticEdge>,
}

impl GraphFixture {
    pub(crate) fn new(
        node_count: u32,
        pattern: GraphPattern,
        policy: AdaptiveNodeSetPolicy,
    ) -> Self {
        let edges = graph_edges(node_count, pattern);
        let graph = PackedGraph::build(
            (0..node_count)
                .rev()
                .map(|key| GraphNode { key, payload: key })
                .collect(),
            edges
                .iter()
                .rev()
                .map(|edge| GraphEdge {
                    id: edge.id,
                    source: edge.source,
                    target: edge.target,
                    payload: EdgeKind::Dependency,
                    reason: edge.id,
                })
                .collect(),
            policy,
        )
        .expect("synthetic graph declaration is valid");
        Self { graph, edges }
    }

    pub(crate) fn origin(&self) -> NodeHandle {
        self.graph
            .node_at(0)
            .expect("benchmark graphs are nonempty")
    }

    pub(crate) fn impact_seed(&self) -> NodeHandle {
        let last =
            u32::try_from(self.graph.node_count() - 1).expect("benchmark graph fits dense handles");
        self.graph
            .node_at(last)
            .expect("last benchmark node exists")
    }
}

pub(crate) fn graph_edges(node_count: u32, pattern: GraphPattern) -> Vec<SyntheticEdge> {
    assert!(node_count > 0, "benchmark graphs must be nonempty");
    let mut endpoints = Vec::new();
    match pattern {
        GraphPattern::Chain => {
            endpoints.extend((0..node_count - 1).map(|source| (source, source + 1)));
        }
        GraphPattern::Layered => {
            let width = if node_count < 256 { 8 } else { 32 };
            for source in 0..node_count {
                let next = source.saturating_add(width);
                if next < node_count {
                    endpoints.push((source, next));
                    let adjacent = next + 1;
                    if adjacent < node_count && adjacent / width == next / width {
                        endpoints.push((source, adjacent));
                    }
                }
            }
        }
        GraphPattern::Fanout => {
            endpoints.extend((1..node_count).map(|target| (0, target)));
        }
        GraphPattern::Disjoint => {
            let band = 16;
            endpoints.extend(
                (0..node_count - 1)
                    .filter(|source| (source + 1) % band != 0)
                    .map(|source| (source, source + 1)),
            );
        }
        GraphPattern::SccBand => {
            let band = 8;
            let mut start = 0;
            while start < node_count {
                let end = (start + band).min(node_count);
                if end - start > 1 {
                    endpoints.extend((start..end - 1).map(|source| (source, source + 1)));
                    endpoints.push((end - 1, start));
                }
                if end < node_count {
                    endpoints.push((start, end));
                }
                start = end;
            }
        }
        GraphPattern::Parallel => {
            for source in 0..node_count - 1 {
                endpoints.push((source, source + 1));
                endpoints.push((source, source + 1));
            }
        }
    }
    endpoints
        .into_iter()
        .enumerate()
        .map(|(index, (source, target))| SyntheticEdge {
            id: u64::try_from(index).expect("benchmark edge count fits u64") + 1,
            source,
            target,
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReferenceClosure {
    pub(crate) reached: BTreeSet<u32>,
    pub(crate) predecessor: Vec<Option<(u32, u64)>>,
}

impl ReferenceClosure {
    pub(crate) fn edge_path_to(&self, target: u32) -> Option<Vec<u64>> {
        if !self.reached.contains(&target) {
            return None;
        }
        let mut cursor = target;
        let mut path = Vec::new();
        while let Some((prior, edge)) = self.predecessor[cursor as usize] {
            path.push(edge);
            cursor = prior;
        }
        path.reverse();
        Some(path)
    }
}

pub(crate) fn reference_closure(
    node_count: u32,
    edges: &[SyntheticEdge],
    origins: &[u32],
    direction: Direction,
) -> ReferenceClosure {
    let mut canonical = edges.to_vec();
    canonical.sort_unstable_by_key(|edge| (edge.source, edge.target, edge.id));
    let mut adjacency = vec![Vec::new(); node_count as usize];
    for edge in canonical {
        let (owner, neighbor) = match direction {
            Direction::Outgoing => (edge.source, edge.target),
            Direction::Incoming => (edge.target, edge.source),
        };
        adjacency[owner as usize].push((neighbor, edge.id));
    }
    let mut reached = origins.iter().copied().collect::<BTreeSet<_>>();
    assert!(reached.iter().all(|node| *node < node_count));
    let mut queue = reached.iter().copied().collect::<VecDeque<_>>();
    let mut predecessor = vec![None; node_count as usize];
    while let Some(current) = queue.pop_front() {
        for &(neighbor, edge) in &adjacency[current as usize] {
            if reached.insert(neighbor) {
                predecessor[neighbor as usize] = Some((current, edge));
                queue.push_back(neighbor);
            }
        }
    }
    ReferenceClosure {
        reached,
        predecessor,
    }
}

/// Return the synthetic mechanics policy shared by benchmark and reference tests.
///
/// These fixed thresholds deliberately exercise all three encodings. They are
/// neither measured selection nor a default for an admitted Pale generation.
pub(crate) fn synthetic_set_policy() -> AdaptiveNodeSetPolicy {
    AdaptiveNodeSetPolicy::new(
        SYNTHETIC_SMALL_CARDINALITY_LIMIT,
        SYNTHETIC_DENSE_MIN_DENSITY_PERMYRIAD,
    )
    .expect("synthetic policy is valid")
}
