//! One immutable packed directed-multigraph authority.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

use petgraph::visit::{
    Data, EdgeCount, EdgeIndexable, EdgeRef, GraphBase, GraphProp, IntoEdgeReferences, IntoEdges,
    IntoEdgesDirected, IntoNeighbors, IntoNeighborsDirected, IntoNodeIdentifiers,
    NodeCompactIndexable, NodeCount, NodeIndexable, VisitMap, Visitable,
};
use petgraph::{Directed, Direction};

use crate::{AdaptiveNodeSet, AdaptiveNodeSetPolicy, NodeSetError};

#[cfg(all(feature = "graphblas", not(target_family = "wasm")))]
mod graphblas;
mod query;

pub use query::{GraphTopology, ReachabilityIndex};

/// Monotone allocator for process-local graph identities.
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

fn dense_index(index: usize) -> u32 {
    u32::try_from(index).expect("packed graph cardinality is checked at construction")
}

/// Process-local identity of one immutable packed graph generation.
///
/// Values are deliberately opaque and non-serializable. Owning kernel
/// constructors allocate nonzero identities; exhaustion fails closed rather
/// than wrapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphGeneration(NonZeroU64);

/// Process-local graph-generation allocation failed closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
enum GraphGenerationError {
    /// The nonzero `u64` identity space is exhausted.
    #[error("process-local graph-generation identities are exhausted")]
    Exhausted,
}

impl GraphGeneration {
    /// Allocate through an owning counter without wrapping at exhaustion.
    fn fresh(counter: &AtomicU64) -> Result<Self, GraphGenerationError> {
        let value = counter
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| GraphGenerationError::Exhausted)?;
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(GraphGenerationError::Exhausted)
    }
}

/// Dense node handle valid only in one [`GraphGeneration`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeHandle {
    generation: GraphGeneration,
    index: u32,
}

impl NodeHandle {
    pub(crate) const fn new(generation: GraphGeneration, index: u32) -> Self {
        Self { generation, index }
    }

    /// Return the dense generation-local index.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }

    /// Return the owning packed generation.
    #[must_use]
    pub const fn generation(self) -> GraphGeneration {
        self.generation
    }
}

/// Dense edge handle valid only in one [`GraphGeneration`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeHandle {
    generation: GraphGeneration,
    index: u32,
}

impl EdgeHandle {
    /// Return the dense generation-local index.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }

    /// Return the owning packed generation.
    #[must_use]
    pub const fn generation(self) -> GraphGeneration {
        self.generation
    }
}

/// One semantic node supplied to the consuming builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNode<K, N> {
    /// Stable semantic key used for construction and lookup.
    pub key: K,
    /// Typed immutable node body.
    pub payload: N,
}

/// One typed directed edge supplied to the consuming builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdge<I, K, E, R> {
    /// Canonical identity that distinguishes lawful parallel edges.
    pub id: I,
    /// Stable semantic source key.
    pub source: K,
    /// Stable semantic target key.
    pub target: K,
    /// Typed edge body.
    pub payload: E,
    /// Typed causal reason retained for explanation.
    pub reason: R,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PackedEdge<I, E, R> {
    id: I,
    source: u32,
    target: u32,
    payload: E,
    reason: R,
}

/// Construction rejected an ambiguous or incomplete semantic graph.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GraphBuildError<K, I> {
    /// A semantic node key occurs more than once.
    #[error("packed graph repeats node key")]
    DuplicateNode(K),
    /// A canonical edge identity occurs more than once.
    #[error("packed graph repeats edge identity")]
    DuplicateEdge(I),
    /// An edge endpoint has no admitted node.
    #[error("packed graph edge references a missing node")]
    MissingNode(K),
    /// Cardinality exceeds the dense `u32` handle space.
    #[error("packed graph exceeds its dense handle space")]
    HandleSpace,
    /// Process-local graph-generation identities are exhausted.
    #[error("process-local graph-generation identities are exhausted")]
    GenerationExhausted,
}

/// A public graph query received a foreign or absent handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GraphAccessError {
    /// The handle belongs to another immutable generation.
    #[error("graph handle belongs to another generation")]
    ForeignGeneration,
    /// The handle index is outside the graph's packed range.
    #[error("graph handle index is outside the packed range")]
    OutOfRange,
    /// Adaptive-set construction failed for a graph-internal index.
    #[error(transparent)]
    NodeSet(#[from] NodeSetError),
}

/// Mechanical backend currently serving one [`BatchReachability`] index.
///
/// Both variants implement the same finite reachability algebra. This value is
/// performance evidence only: it is not graph identity, admission, authority,
/// activation, or proof that an edge caused a runtime occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchReachabilityBackend {
    /// Canonical deterministic traversal over [`PackedGraph`] CSR indexes.
    CanonicalCpu,
    /// SuiteSparse:GraphBLAS support-algebra evaluation on the CPU.
    ///
    /// This variant can be selected only when the crate is built with the
    /// `graphblas` feature and the derived matrix initializes successfully.
    GraphblasCpu,
}

/// One deterministic traversal-origin-to-reached-node reason path.
///
/// [`PackedGraph::reachability`] follows each edge from source to target, so
/// its edge orientation matches adjacent nodes. [`PackedGraph::reverse_impact`]
/// follows dependency edges from target to source; each returned edge therefore
/// points from the next node in the path back to the previous node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasonPath {
    /// Nodes in traversal order from origin through reached destination.
    pub nodes: Vec<NodeHandle>,
    /// Exact edges connecting adjacent [`Self::nodes`].
    pub edges: Vec<EdgeHandle>,
}

/// Canonical predecessor selected for one non-origin reached node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Predecessor {
    prior: u32,
    edge: u32,
}

/// Total per-node state in one compact reachability proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReachabilityState {
    Unreached,
    Origin,
    Via(Predecessor),
}

/// Compact proof of canonical reachability from one or more origins.
///
/// The proof-storage and requested-witness bounds are rendered on
/// [`PackedGraph`]. [`Self::path_to`] reconstructs only the requested witness.
#[derive(Debug, PartialEq, Eq)]
pub struct ReachabilityProof {
    generation: GraphGeneration,
    reached: AdaptiveNodeSet,
    state: Box<[ReachabilityState]>,
}

impl ReachabilityProof {
    /// Borrow the exact reached-node set.
    #[must_use]
    pub const fn reached(&self) -> &AdaptiveNodeSet {
        &self.reached
    }

    /// Iterate the canonical predecessor edge for every non-origin reached node.
    ///
    /// The iterator exposes the compact proof forest without reconstructing one
    /// complete [`ReasonPath`] per reached node. It contains no edge for an
    /// origin and does not enumerate alternate paths, so it remains evidence of
    /// the deterministic traversal proof rather than a second adjacency view.
    pub fn witness_edges(&self) -> impl Iterator<Item = EdgeHandle> + '_ {
        self.state.iter().filter_map(|state| match state {
            ReachabilityState::Via(predecessor) => Some(EdgeHandle {
                generation: self.generation,
                index: predecessor.edge,
            }),
            ReachabilityState::Unreached | ReachabilityState::Origin => None,
        })
    }

    /// Reconstruct the canonical shortest witness to `target` when reached.
    ///
    /// # Errors
    ///
    /// Rejects a foreign or out-of-range target handle.
    pub fn path_to(&self, target: NodeHandle) -> Result<Option<ReasonPath>, GraphAccessError> {
        if target.generation != self.generation {
            return Err(GraphAccessError::ForeignGeneration);
        }
        if target.index as usize >= self.state.len() {
            return Err(GraphAccessError::OutOfRange);
        }
        let mut nodes = vec![target];
        let mut edges = Vec::new();
        let mut cursor = target;
        loop {
            match self.state[cursor.index as usize] {
                ReachabilityState::Unreached => return Ok(None),
                ReachabilityState::Origin => break,
                ReachabilityState::Via(predecessor) => {
                    edges.push(EdgeHandle {
                        generation: self.generation,
                        index: predecessor.edge,
                    });
                    cursor = NodeHandle::new(self.generation, predecessor.prior);
                    nodes.push(cursor);
                }
            }
        }
        nodes.reverse();
        edges.reverse();
        Ok(Some(ReasonPath { nodes, edges }))
    }
}

/// Exact affected/unaffected partition plus compact causal witnesses.
#[derive(Debug, PartialEq, Eq)]
pub struct ImpactClosure {
    unaffected: AdaptiveNodeSet,
    proof: ReachabilityProof,
}

impl ImpactClosure {
    /// Borrow seeds and every node that transitively depends on them.
    ///
    /// The reached set inside [`Self::proof`] is the single affected-membership
    /// authority; the closure does not clone it into a parallel set.
    #[must_use]
    pub const fn affected(&self) -> &AdaptiveNodeSet {
        self.proof.reached()
    }

    /// Borrow the exact unaffected complement in this generation.
    #[must_use]
    pub const fn unaffected(&self) -> &AdaptiveNodeSet {
        &self.unaffected
    }

    /// Borrow the compact multi-source causal proof.
    #[must_use]
    pub const fn proof(&self) -> &ReachabilityProof {
        &self.proof
    }
}

/// One immutable packed directed multigraph.
///
/// Nodes sort by semantic key and receive a dense bijection. In the rendered
/// laws, `K` is the semantic-key set, `V` the node set, `I` the edge-identity
/// set, and `E` the edge set; `d(v)` is a node's selected-direction degree and
/// `ell` a reconstructed witness length:
///
/// Edges sort by `(source, target, edge identity)` and receive the analogous
/// dense edge bijection.
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/math/packed-graph-laws.display.mathml"))]
///
/// This is the deterministic tie-break for adjacency and shortest-path choice.
/// Forward and reverse offsets are derived indexes into the same edge arena, so
/// lawful parallel edges remain distinct without creating a second topology.
///
/// The construction, traversal, lookup, neighbor-iteration, proof-storage, and
/// witness-reconstruction bounds are rendered above. Dense handles and the
/// [`PetgraphView`] are process-local mechanics, not semantic identities,
/// authority, admission, activation, or native truth.
#[derive(Debug, PartialEq, Eq)]
pub struct PackedGraph<K, N, I, E, R> {
    generation: GraphGeneration,
    set_policy: AdaptiveNodeSetPolicy,
    nodes: Box<[GraphNode<K, N>]>,
    edges: Box<[PackedEdge<I, E, R>]>,
    forward_offsets: Box<[u32]>,
    forward_edges: Box<[u32]>,
    reverse_offsets: Box<[u32]>,
    reverse_edges: Box<[u32]>,
}

/// Reusable set-only batch reachability over one immutable [`PackedGraph`].
///
/// The borrowed graph remains the sole owner of adjacency, generation
/// fencing, typed bodies, and canonical reason paths. Default and portable
/// builds use the pure-Rust [`PackedGraph`] traversal. The optional `graphblas`
/// feature uses the permissively licensed `issundb-graphblas` safe wrapper over
/// the `SuiteSparse` C implementation to derive one reusable matrix. Its
/// process-global runtime initializes exactly once, permits concurrent index
/// handles, and is deliberately never finalized. Any initialization or
/// operation failure drops that disposable projection and recomputes through
/// the canonical CPU path.
///
/// Building that feature requires `CMake` plus a C/`OpenMP` toolchain. A prebuilt
/// binary does not require `CMake`, although its platform `OpenMP` runtime remains
/// a packaging concern; WebAssembly targets exclude the native dependency even
/// under all features. A network-free first build additionally requires
/// `ISSUNDB_GRAPHBLAS_SRC` to name a checksum-verified `GraphBLAS` 10.3.1 source
/// root containing `Include/GraphBLAS.h`; Cargo's `--offline` flag alone does not
/// suppress the dependency build script's pinned source download.
///
/// `graphblas_sparse_linear_algebra` was declined because its non-commercial
/// licence is unsuitable, and no mature equivalent pure-Rust `GraphBLAS`
/// backend currently earns replacement. Pale therefore does not grow a
/// home-built numerical backend: equivalence and benchmark evidence gate this
/// optimization, never graph meaning.
///
/// In the batch law, `V` is the node universe, `E_d` the edges in direction
/// `d`, `S_i` seed set `i`, `b` the batch size, and `R_d` exact reachability.
/// The law is evaluated independently for every seed set:
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/batch-reachability.display.mathml"
))]
///
/// Results carry exact reached sets only. Use [`PackedGraph::reachability`] or
/// [`PackedGraph::reverse_impact`] when the caller needs canonical reason paths
/// or an unaffected complement.
pub struct BatchReachability<'a, K, N, I, E, R> {
    graph: &'a PackedGraph<K, N, I, E, R>,
    #[cfg(all(feature = "graphblas", not(target_family = "wasm")))]
    accelerator: Option<graphblas::ReachabilityIndex>,
}

impl<K: Ord + Clone, N, I: Ord + Clone, E, R> PackedGraph<K, N, I, E, R> {
    /// Consume semantic inputs into one canonical packed generation.
    ///
    /// # Errors
    ///
    /// Rejects duplicate node keys or edge identities, missing endpoints, and
    /// cardinality outside dense `u32` handles. Distinct edge identities retain
    /// lawful parallel edges even when their payloads and reasons are equal.
    pub fn build(
        nodes: Vec<GraphNode<K, N>>,
        edges: Vec<GraphEdge<I, K, E, R>>,
        set_policy: AdaptiveNodeSetPolicy,
    ) -> Result<Self, GraphBuildError<K, I>> {
        Self::build_with_generation_counter(nodes, edges, set_policy, &NEXT_GENERATION)
    }

    /// Canonical build implementation parameterized only by its identity counter.
    fn build_with_generation_counter(
        mut nodes: Vec<GraphNode<K, N>>,
        edges: Vec<GraphEdge<I, K, E, R>>,
        set_policy: AdaptiveNodeSetPolicy,
        generation_counter: &AtomicU64,
    ) -> Result<Self, GraphBuildError<K, I>> {
        if nodes.len() > u32::MAX as usize || edges.len() > u32::MAX as usize {
            return Err(GraphBuildError::HandleSpace);
        }
        nodes.sort_by(|left, right| left.key.cmp(&right.key));
        if let Some(pair) = nodes.windows(2).find(|pair| pair[0].key == pair[1].key) {
            return Err(GraphBuildError::DuplicateNode(pair[0].key.clone()));
        }
        let node_index = nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.key.clone(), dense_index(index)))
            .collect::<BTreeMap<_, _>>();

        let mut edge_ids = BTreeSet::new();
        let mut packed = Vec::with_capacity(edges.len());
        for edge in edges {
            if !edge_ids.insert(edge.id.clone()) {
                return Err(GraphBuildError::DuplicateEdge(edge.id));
            }
            let source = node_index
                .get(&edge.source)
                .copied()
                .ok_or_else(|| GraphBuildError::MissingNode(edge.source.clone()))?;
            let target = node_index
                .get(&edge.target)
                .copied()
                .ok_or_else(|| GraphBuildError::MissingNode(edge.target.clone()))?;
            packed.push(PackedEdge {
                id: edge.id,
                source,
                target,
                payload: edge.payload,
                reason: edge.reason,
            });
        }
        packed.sort_by(|left, right| {
            (&left.source, &left.target, &left.id).cmp(&(&right.source, &right.target, &right.id))
        });
        let (forward_offsets, forward_edges) =
            pack_adjacency(nodes.len(), packed.len(), |edge| packed[edge].source);
        let (reverse_offsets, reverse_edges) =
            pack_adjacency(nodes.len(), packed.len(), |edge| packed[edge].target);
        let generation = GraphGeneration::fresh(generation_counter)
            .map_err(|GraphGenerationError::Exhausted| GraphBuildError::GenerationExhausted)?;
        Ok(Self {
            generation,
            set_policy,
            nodes: nodes.into(),
            edges: packed.into(),
            forward_offsets: forward_offsets.into(),
            forward_edges: forward_edges.into(),
            reverse_offsets: reverse_offsets.into(),
            reverse_edges: reverse_edges.into(),
        })
    }
}

fn pack_adjacency(
    node_count: usize,
    edge_count: usize,
    owner: impl Fn(usize) -> u32,
) -> (Vec<u32>, Vec<u32>) {
    let mut offsets = vec![0_u32; node_count + 1];
    for edge in 0..edge_count {
        offsets[owner(edge) as usize + 1] += 1;
    }
    for index in 1..offsets.len() {
        offsets[index] += offsets[index - 1];
    }
    let mut cursors = offsets[..node_count].to_vec();
    let mut packed = vec![0_u32; edge_count];
    for edge in 0..edge_count {
        let node = owner(edge) as usize;
        packed[cursors[node] as usize] = dense_index(edge);
        cursors[node] += 1;
    }
    (offsets, packed)
}

impl<K, N, I, E, R> PackedGraph<K, N, I, E, R> {
    /// Return this graph's immutable generation.
    #[must_use]
    pub const fn generation(&self) -> GraphGeneration {
        self.generation
    }

    /// Return this generation's sole adaptive-set representation policy.
    #[must_use]
    pub const fn set_policy(&self) -> AdaptiveNodeSetPolicy {
        self.set_policy
    }

    /// Prepare a reusable set-only batch reachability index.
    ///
    /// Construction never changes the owning graph. When `GraphBLAS` is enabled,
    /// its sparse matrix is a generation-local derived index built from the
    /// canonical edge arena; failure to build it selects the CPU implementation
    /// without changing query meaning.
    #[must_use]
    pub fn batch_reachability(&self) -> BatchReachability<'_, K, N, I, E, R> {
        BatchReachability {
            graph: self,
            #[cfg(all(feature = "graphblas", not(target_family = "wasm")))]
            accelerator: graphblas::ReachabilityIndex::build(self).ok(),
        }
    }

    /// Prepare the canonical CPU batch implementation as a reference baseline.
    ///
    /// This constructor shares [`BatchReachability::query`] and its exact set
    /// semantics with [`Self::batch_reachability`], but deliberately omits any
    /// disposable accelerator. It exists for matched measurement and
    /// equivalence evidence; it creates no alternate adjacency, proof-path, or
    /// graph-meaning owner.
    #[must_use]
    pub const fn canonical_batch_reachability(&self) -> BatchReachability<'_, K, N, I, E, R> {
        BatchReachability {
            graph: self,
            #[cfg(all(feature = "graphblas", not(target_family = "wasm")))]
            accelerator: None,
        }
    }

    /// Return the node count.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Return the edge count, including parallel edges.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Build a finite set through this graph generation's sole policy.
    ///
    /// The method validates every [`NodeHandle`] before selecting storage. It is
    /// the public construction seam for query sets, which prevents callers from
    /// choosing physical encodings independently of the generation.
    ///
    /// # Errors
    ///
    /// Rejects a foreign handle or a handle outside this graph's node universe.
    pub fn node_set(
        &self,
        nodes: impl IntoIterator<Item = NodeHandle>,
    ) -> Result<AdaptiveNodeSet, NodeSetError> {
        AdaptiveNodeSet::from_nodes(
            self.generation,
            dense_index(self.nodes.len()),
            nodes,
            self.set_policy,
        )
    }

    /// Borrow the dense compatibility view used by `petgraph` algorithms.
    ///
    /// The view deliberately uses generation-free dense `u32` identifiers.
    /// Pale [`NodeHandle`] values therefore cannot cross into an algorithm
    /// that has no error channel for rejecting a foreign graph generation.
    /// Convert any selected dense index back through [`Self::node_at`] before
    /// using it with generation-checked Pale APIs.
    #[must_use]
    pub const fn petgraph_view(&self) -> PetgraphView<'_, K, N, I, E, R> {
        PetgraphView { graph: self }
    }

    /// Resolve a dense algorithm-view index to a generation-bound handle.
    ///
    /// # Errors
    ///
    /// Rejects an index outside this graph's packed node range.
    pub fn node_at(&self, index: u32) -> Result<NodeHandle, GraphAccessError> {
        if index as usize >= self.nodes.len() {
            Err(GraphAccessError::OutOfRange)
        } else {
            Ok(self.node(index as usize))
        }
    }

    /// Resolve a packed edge index to a generation-bound handle.
    ///
    /// Iterating `0..edge_count` through this method exposes every canonical
    /// edge, including parallel edges that no selected reason path uses.
    ///
    /// # Errors
    ///
    /// Rejects an index outside this graph's packed edge range.
    pub fn edge_at(&self, index: u32) -> Result<EdgeHandle, GraphAccessError> {
        if index as usize >= self.edges.len() {
            Err(GraphAccessError::OutOfRange)
        } else {
            Ok(EdgeHandle {
                generation: self.generation,
                index,
            })
        }
    }

    /// Look up one canonical semantic node.
    #[must_use]
    pub fn find(&self, key: &K) -> Option<NodeHandle>
    where
        K: Ord,
    {
        self.nodes
            .binary_search_by(|node| node.key.cmp(key))
            .ok()
            .map(|index| self.node(index))
    }

    /// Find a semantic node through a borrowed total-order comparison.
    ///
    /// The callback compares each canonical stored key with one borrowed query
    /// using the same total order as `K`. This preserves the lookup bound
    /// rendered on [`PackedGraph`] without allocating an owned `K` merely to
    /// enter binary search. The
    /// returned capability remains fenced to this graph generation.
    #[must_use]
    pub fn find_by(&self, mut compare: impl FnMut(&K) -> std::cmp::Ordering) -> Option<NodeHandle> {
        self.nodes
            .binary_search_by(|node| compare(&node.key))
            .ok()
            .map(|index| self.node(index))
    }

    /// Borrow a node body after generation validation.
    ///
    /// # Errors
    ///
    /// Rejects a foreign or out-of-range handle.
    pub fn node_payload(&self, node: NodeHandle) -> Result<&N, GraphAccessError> {
        Ok(&self.nodes[self.validate_node(node)?].payload)
    }

    /// Borrow the canonical semantic key for a node handle.
    ///
    /// # Errors
    ///
    /// Rejects a foreign or out-of-range handle.
    pub fn node_key(&self, node: NodeHandle) -> Result<&K, GraphAccessError> {
        Ok(&self.nodes[self.validate_node(node)?].key)
    }

    /// Borrow an edge payload and reason after generation validation.
    ///
    /// # Errors
    ///
    /// Rejects a foreign or out-of-range handle.
    pub fn edge_payload(&self, edge: EdgeHandle) -> Result<(&E, &R), GraphAccessError> {
        if edge.generation != self.generation {
            return Err(GraphAccessError::ForeignGeneration);
        }
        let edge = self
            .edges
            .get(edge.index as usize)
            .ok_or(GraphAccessError::OutOfRange)?;
        Ok((&edge.payload, &edge.reason))
    }

    /// Borrow an edge's canonical identity and exact endpoints.
    ///
    /// # Errors
    ///
    /// Rejects a foreign or out-of-range handle.
    pub fn edge_identity(
        &self,
        edge: EdgeHandle,
    ) -> Result<(&I, NodeHandle, NodeHandle), GraphAccessError> {
        if edge.generation != self.generation {
            return Err(GraphAccessError::ForeignGeneration);
        }
        let edge = self
            .edges
            .get(edge.index as usize)
            .ok_or(GraphAccessError::OutOfRange)?;
        Ok((
            &edge.id,
            self.node(edge.source as usize),
            self.node(edge.target as usize),
        ))
    }

    /// Iterate exact incident edge handles without copying edge bodies.
    ///
    /// `Outgoing` yields edges whose source is `node`; `Incoming` yields edges
    /// whose target is `node`. The iterator walks a borrowed index slice into
    /// the graph's single canonical edge arena. Callers can filter closed typed
    /// payload variants through [`Self::edge_payload`] while parallel edges
    /// remain independently addressable.
    ///
    /// # Errors
    ///
    /// Rejects a foreign or out-of-range node handle.
    pub fn edges_directed(
        &self,
        node: NodeHandle,
        direction: Direction,
    ) -> Result<DirectedEdgeHandles<'_>, GraphAccessError> {
        self.validate_node(node)?;
        Ok(DirectedEdgeHandles {
            generation: self.generation,
            edges: self.edge_ids_unchecked(node, direction).iter(),
        })
    }

    /// Compute forward reachability with canonical shortest witnesses.
    ///
    /// Reachability is the least fixed point of the monotone operator rendered
    /// on [`BatchReachability`]. Because the vertex set is
    /// finite, breadth-first iteration terminates after at most one admission
    /// per vertex. The proof retains one deterministic shortest predecessor; it
    /// does not claim that the selected edge caused a runtime event.
    ///
    /// # Errors
    ///
    /// Rejects a foreign or out-of-range origin.
    pub fn reachability(&self, origin: NodeHandle) -> Result<ReachabilityProof, GraphAccessError> {
        self.traverse([origin], Direction::Outgoing)
    }

    /// Compute exact reverse impact and its unrelated complement.
    ///
    /// For node universe `V`, owner-to-dependency edges `E`, and initial seed
    /// set `S_0`, affected nodes are the least fixed point
    ///
    #[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/math/reverse-impact-closure.display.mathml"))]
    ///
    /// The operator is monotone over the finite powerset lattice.
    /// [`ImpactClosure::unaffected`] is the exact complement in this graph
    /// generation; neither partition is an authority or release decision.
    ///
    /// # Errors
    ///
    /// Rejects any foreign or out-of-range seed, or an impossible internal set
    /// construction failure.
    pub fn reverse_impact(
        &self,
        seeds: impl IntoIterator<Item = NodeHandle>,
    ) -> Result<ImpactClosure, GraphAccessError> {
        let proof = self.traverse(seeds, Direction::Incoming)?;
        let unaffected = proof.reached().complement()?;
        Ok(ImpactClosure { unaffected, proof })
    }

    fn traverse(
        &self,
        origins: impl IntoIterator<Item = NodeHandle>,
        direction: Direction,
    ) -> Result<ReachabilityProof, GraphAccessError> {
        let mut origin_mask = vec![false; self.nodes.len()];
        for origin in origins {
            origin_mask[self.validate_node(origin)?] = true;
        }
        let mut state = vec![ReachabilityState::Unreached; self.nodes.len()];
        let mut queue = VecDeque::new();
        for (index, is_origin) in origin_mask.into_iter().enumerate() {
            if is_origin {
                state[index] = ReachabilityState::Origin;
                queue.push_back(dense_index(index));
            }
        }
        while let Some(index) = queue.pop_front() {
            let node = self.node(index as usize);
            for &edge_index in self.edge_ids_unchecked(node, direction) {
                let neighbor = self.neighbor_for(edge_index, direction);
                if state[neighbor.index as usize] == ReachabilityState::Unreached {
                    state[neighbor.index as usize] = ReachabilityState::Via(Predecessor {
                        prior: index,
                        edge: edge_index,
                    });
                    queue.push_back(neighbor.index);
                }
            }
        }
        let reached = AdaptiveNodeSet::from_canonical(
            self.generation,
            dense_index(self.nodes.len()),
            state
                .iter()
                .enumerate()
                .filter_map(|(index, state)| {
                    (*state != ReachabilityState::Unreached).then_some(dense_index(index))
                })
                .collect(),
            self.set_policy,
        )?;
        Ok(ReachabilityProof {
            generation: self.generation,
            reached,
            state: state.into(),
        })
    }

    fn validate_node(&self, node: NodeHandle) -> Result<usize, GraphAccessError> {
        if node.generation != self.generation {
            return Err(GraphAccessError::ForeignGeneration);
        }
        let index = node.index as usize;
        if index >= self.nodes.len() {
            return Err(GraphAccessError::OutOfRange);
        }
        Ok(index)
    }

    fn node(&self, index: usize) -> NodeHandle {
        NodeHandle::new(self.generation, dense_index(index))
    }

    fn edge_ids_unchecked(&self, node: NodeHandle, direction: Direction) -> &[u32] {
        let index = node.index as usize;
        let (offsets, edges) = if direction == Direction::Outgoing {
            (&self.forward_offsets, &self.forward_edges)
        } else {
            (&self.reverse_offsets, &self.reverse_edges)
        };
        &edges[offsets[index] as usize..offsets[index + 1] as usize]
    }

    fn neighbor_for(&self, edge: u32, direction: Direction) -> NodeHandle {
        let edge = &self.edges[edge as usize];
        self.node(if direction == Direction::Outgoing {
            edge.target as usize
        } else {
            edge.source as usize
        })
    }
}

impl<K, N, I, E, R> BatchReachability<'_, K, N, I, E, R> {
    /// Report the currently selected mechanical backend.
    ///
    /// A `GraphBLAS` operation error permanently changes this index to
    /// [`BatchReachabilityBackend::CanonicalCpu`] before the failed query is
    /// recomputed. Callers must not use this value as a semantic branch.
    #[must_use]
    pub const fn backend(&self) -> BatchReachabilityBackend {
        #[cfg(all(feature = "graphblas", not(target_family = "wasm")))]
        if self.accelerator.is_some() {
            return BatchReachabilityBackend::GraphblasCpu;
        }
        BatchReachabilityBackend::CanonicalCpu
    }

    /// Compute exact reached sets for a batch of independent seed sets.
    ///
    /// All handles are validated before either backend runs. Output order
    /// matches input batch order, while members inside each
    /// [`AdaptiveNodeSet`] follow the owning graph's deterministic iteration
    /// order and sole [`AdaptiveNodeSetPolicy`]. An empty seed set reaches
    /// nothing.
    ///
    /// # Errors
    ///
    /// Rejects a foreign or out-of-range seed handle, or an impossible
    /// graph-internal adaptive-set construction failure. `GraphBLAS` mechanics do
    /// not escape this semantic boundary: backend failures select and recompute
    /// through the canonical CPU path.
    pub fn query<B, S>(
        &mut self,
        direction: Direction,
        batches: B,
    ) -> Result<Vec<AdaptiveNodeSet>, GraphAccessError>
    where
        B: IntoIterator<Item = S>,
        S: IntoIterator<Item = NodeHandle>,
    {
        let batches = batches
            .into_iter()
            .map(|seeds| {
                seeds
                    .into_iter()
                    .map(|seed| self.graph.validate_node(seed).map(dense_index))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;

        #[cfg(all(feature = "graphblas", not(target_family = "wasm")))]
        if let Some(accelerator) = &self.accelerator {
            match accelerator.closures(direction, &batches) {
                Ok(closures) => return self.graph.sets_from_dense(closures),
                Err(_) => self.accelerator = None,
            }
        }

        self.graph.cpu_batch_reachability(direction, &batches)
    }
}

impl<K, N, I, E, R> PackedGraph<K, N, I, E, R> {
    #[cfg(all(feature = "graphblas", not(target_family = "wasm")))]
    fn sets_from_dense(
        &self,
        closures: Vec<Vec<u32>>,
    ) -> Result<Vec<AdaptiveNodeSet>, GraphAccessError> {
        closures
            .into_iter()
            .map(|members| {
                AdaptiveNodeSet::from_canonical(
                    self.generation,
                    dense_index(self.nodes.len()),
                    members,
                    self.set_policy,
                )
                .map_err(GraphAccessError::from)
            })
            .collect()
    }

    fn cpu_batch_reachability(
        &self,
        direction: Direction,
        batches: &[Vec<u32>],
    ) -> Result<Vec<AdaptiveNodeSet>, GraphAccessError> {
        batches
            .iter()
            .map(|seeds| {
                self.traverse(
                    seeds.iter().map(|&index| self.node(index as usize)),
                    direction,
                )
                .map(|proof| proof.reached)
            })
            .collect()
    }
}

/// Zero-copy iterator over exact generation-bound edge handles.
///
/// The iterator borrows one packed adjacency slice and yields in canonical edge
/// order. Its creation and degree-relative iteration bounds are rendered on
/// [`PackedGraph`].
/// [`Iterator::nth`] skips directly within that slice, and [`ExactSizeIterator::len`]
/// reports its remaining edges without traversal. An ordinal is meaningful only
/// within the same graph generation, node and direction; it is not a semantic
/// identity or an admitted query continuation.
pub struct DirectedEdgeHandles<'a> {
    generation: GraphGeneration,
    edges: std::slice::Iter<'a, u32>,
}

impl Iterator for DirectedEdgeHandles<'_> {
    type Item = EdgeHandle;

    fn next(&mut self) -> Option<Self::Item> {
        self.nth(0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.edges.size_hint()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.edges.nth(n).copied().map(|index| EdgeHandle {
            generation: self.generation,
            index,
        })
    }
}

impl ExactSizeIterator for DirectedEdgeHandles<'_> {}

/// Borrowed `petgraph` view over one immutable [`PackedGraph`].
///
/// Dense `u32` node identifiers exist only at this compatibility boundary.
/// The owning graph remains authoritative for generation-checked handles,
/// payloads, causal paths, and impact closures.
#[derive(Debug, Clone, Copy)]
pub struct PetgraphView<'a, K, N, I, E, R> {
    graph: &'a PackedGraph<K, N, I, E, R>,
}

/// Zero-copy dense neighbor iterator for [`PetgraphView`].
pub struct PetgraphNeighbors<'a, K, N, I, E, R> {
    graph: &'a PackedGraph<K, N, I, E, R>,
    edges: std::slice::Iter<'a, u32>,
    direction: Direction,
}

/// Borrowed `petgraph` edge reference into the canonical edge arena.
///
/// [`EdgeRef::weight`] exposes only the typed edge payload. Canonical identity,
/// causal reason, and generation fencing remain available through
/// [`PackedGraph::edge_identity`] and [`PackedGraph::edge_payload`].
pub struct PetgraphEdgeReference<'a, I, E, R> {
    index: u32,
    edge: &'a PackedEdge<I, E, R>,
}

impl<I, E, R> Copy for PetgraphEdgeReference<'_, I, E, R> {}

impl<I, E, R> Clone for PetgraphEdgeReference<'_, I, E, R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<I, E, R> EdgeRef for PetgraphEdgeReference<'_, I, E, R> {
    type NodeId = u32;
    type EdgeId = u32;
    type Weight = E;

    fn source(&self) -> Self::NodeId {
        self.edge.source
    }

    fn target(&self) -> Self::NodeId {
        self.edge.target
    }

    fn weight(&self) -> &Self::Weight {
        &self.edge.payload
    }

    fn id(&self) -> Self::EdgeId {
        self.index
    }
}

/// Zero-copy iterator over every canonical edge in a [`PetgraphView`].
pub struct PetgraphEdgeReferences<'a, I, E, R> {
    edges: std::iter::Enumerate<std::slice::Iter<'a, PackedEdge<I, E, R>>>,
}

impl<'a, I, E, R> Iterator for PetgraphEdgeReferences<'a, I, E, R> {
    type Item = PetgraphEdgeReference<'a, I, E, R>;

    fn next(&mut self) -> Option<Self::Item> {
        self.edges
            .next()
            .map(|(index, edge)| PetgraphEdgeReference {
                index: dense_index(index),
                edge,
            })
    }
}

/// Zero-copy iterator over the exact incoming or outgoing edges of one node.
pub struct PetgraphEdges<'a, I, E, R> {
    graph_edges: &'a [PackedEdge<I, E, R>],
    edges: std::slice::Iter<'a, u32>,
}

impl<'a, I, E, R> Iterator for PetgraphEdges<'a, I, E, R> {
    type Item = PetgraphEdgeReference<'a, I, E, R>;

    fn next(&mut self) -> Option<Self::Item> {
        let index = *self.edges.next()?;
        Some(PetgraphEdgeReference {
            index,
            edge: &self.graph_edges[index as usize],
        })
    }
}

impl<K, N, I, E, R> Iterator for PetgraphNeighbors<'_, K, N, I, E, R> {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item> {
        self.edges.next().map(|edge| {
            let edge = &self.graph.edges[*edge as usize];
            if self.direction == Direction::Outgoing {
                edge.target
            } else {
                edge.source
            }
        })
    }
}

/// Dense node-identifier iterator for [`PetgraphView`].
pub struct PetgraphNodeIdentifiers {
    range: std::ops::Range<u32>,
}

impl Iterator for PetgraphNodeIdentifiers {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item> {
        self.range.next()
    }
}

/// Dense visit bitmap scoped to one [`PetgraphView`].
pub struct PetgraphVisitTracker {
    visited: Vec<bool>,
}

impl VisitMap<u32> for PetgraphVisitTracker {
    fn visit(&mut self, node: u32) -> bool {
        let Some(value) = self.visited.get_mut(node as usize) else {
            return false;
        };
        let fresh = !*value;
        *value = true;
        fresh
    }
    fn is_visited(&self, node: &u32) -> bool {
        self.visited.get(*node as usize).copied().unwrap_or(false)
    }

    fn unvisit(&mut self, node: u32) -> bool {
        let Some(value) = self.visited.get_mut(node as usize) else {
            return false;
        };
        let was_visited = *value;
        *value = false;
        was_visited
    }
}

impl<K, N, I, E, R> GraphBase for PetgraphView<'_, K, N, I, E, R> {
    type NodeId = u32;
    type EdgeId = u32;
}
impl<K, N, I, E, R> Data for PetgraphView<'_, K, N, I, E, R> {
    type NodeWeight = N;
    type EdgeWeight = E;
}
impl<K, N, I, E, R> GraphProp for PetgraphView<'_, K, N, I, E, R> {
    type EdgeType = Directed;
}
impl<K, N, I, E, R> NodeCount for PetgraphView<'_, K, N, I, E, R> {
    fn node_count(&self) -> usize {
        self.graph.nodes.len()
    }
}
impl<K, N, I, E, R> EdgeCount for PetgraphView<'_, K, N, I, E, R> {
    fn edge_count(&self) -> usize {
        self.graph.edges.len()
    }
}
impl<K, N, I, E, R> NodeIndexable for PetgraphView<'_, K, N, I, E, R> {
    fn node_bound(&self) -> usize {
        self.graph.nodes.len()
    }
    fn to_index(&self, node: u32) -> usize {
        node as usize
    }
    fn from_index(&self, index: usize) -> u32 {
        dense_index(index)
    }
}
impl<K, N, I, E, R> NodeCompactIndexable for PetgraphView<'_, K, N, I, E, R> {}
impl<K, N, I, E, R> EdgeIndexable for PetgraphView<'_, K, N, I, E, R> {
    fn edge_bound(&self) -> usize {
        self.graph.edges.len()
    }

    fn to_index(&self, edge: u32) -> usize {
        edge as usize
    }

    fn from_index(&self, index: usize) -> u32 {
        dense_index(index)
    }
}
impl<K, N, I, E, R> Visitable for PetgraphView<'_, K, N, I, E, R> {
    type Map = PetgraphVisitTracker;
    fn visit_map(&self) -> Self::Map {
        PetgraphVisitTracker {
            visited: vec![false; self.graph.nodes.len()],
        }
    }
    fn reset_map(&self, map: &mut Self::Map) {
        map.visited.fill(false);
    }
}
impl<K, N, I, E, R> IntoNodeIdentifiers for &PetgraphView<'_, K, N, I, E, R> {
    type NodeIdentifiers = PetgraphNodeIdentifiers;
    fn node_identifiers(self) -> Self::NodeIdentifiers {
        PetgraphNodeIdentifiers {
            range: 0..dense_index(self.graph.nodes.len()),
        }
    }
}
impl<'a, K, N, I, E, R> IntoNeighbors for &'a PetgraphView<'_, K, N, I, E, R> {
    type Neighbors = PetgraphNeighbors<'a, K, N, I, E, R>;
    fn neighbors(self, node: u32) -> Self::Neighbors {
        let graph = self.graph;
        let start = graph.forward_offsets[node as usize] as usize;
        let end = graph.forward_offsets[node as usize + 1] as usize;
        PetgraphNeighbors {
            graph,
            edges: graph.forward_edges[start..end].iter(),
            direction: Direction::Outgoing,
        }
    }
}
impl<'a, K, N, I, E, R> IntoNeighborsDirected for &'a PetgraphView<'_, K, N, I, E, R> {
    type NeighborsDirected = PetgraphNeighbors<'a, K, N, I, E, R>;
    fn neighbors_directed(self, node: u32, direction: Direction) -> Self::NeighborsDirected {
        let graph = self.graph;
        let (offsets, edges) = if direction == Direction::Outgoing {
            (&graph.forward_offsets, &graph.forward_edges)
        } else {
            (&graph.reverse_offsets, &graph.reverse_edges)
        };
        let start = offsets[node as usize] as usize;
        let end = offsets[node as usize + 1] as usize;
        PetgraphNeighbors {
            graph,
            edges: edges[start..end].iter(),
            direction,
        }
    }
}
impl<'a, K, N, I, E, R> IntoEdgeReferences for &'a PetgraphView<'_, K, N, I, E, R> {
    type EdgeRef = PetgraphEdgeReference<'a, I, E, R>;
    type EdgeReferences = PetgraphEdgeReferences<'a, I, E, R>;

    fn edge_references(self) -> Self::EdgeReferences {
        PetgraphEdgeReferences {
            edges: self.graph.edges.iter().enumerate(),
        }
    }
}
impl<'a, K, N, I, E, R> IntoEdges for &'a PetgraphView<'_, K, N, I, E, R> {
    type Edges = PetgraphEdges<'a, I, E, R>;

    fn edges(self, node: u32) -> Self::Edges {
        self.edges_directed(node, Direction::Outgoing)
    }
}
impl<'a, K, N, I, E, R> IntoEdgesDirected for &'a PetgraphView<'_, K, N, I, E, R> {
    type EdgesDirected = PetgraphEdges<'a, I, E, R>;

    fn edges_directed(self, node: u32, direction: Direction) -> Self::EdgesDirected {
        let graph = self.graph;
        let (offsets, edges) = if direction == Direction::Outgoing {
            (&graph.forward_offsets, &graph.forward_edges)
        } else {
            (&graph.reverse_offsets, &graph.reverse_edges)
        };
        let start = offsets[node as usize] as usize;
        let end = offsets[node as usize + 1] as usize;
        PetgraphEdges {
            graph_edges: &graph.edges,
            edges: edges[start..end].iter(),
        }
    }
}

#[cfg(test)]
#[path = "../tests/internal/graph.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/internal/batch_reachability.rs"]
mod batch_reachability_tests;

#[cfg(all(test, feature = "graphblas", not(target_family = "wasm")))]
#[path = "../tests/internal/graphblas.rs"]
mod graphblas_tests;
