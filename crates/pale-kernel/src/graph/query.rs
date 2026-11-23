//! Derived point-query mechanics over the canonical packed graph.

use std::collections::{BTreeSet, VecDeque};

use petgraph::algo::kosaraju_scc;

use super::{
    dense_index, EdgeHandle, GraphAccessError, GraphGeneration, NodeHandle, PackedGraph, ReasonPath,
};

/// Mechanical topology class of one immutable packed generation.
///
/// The original [`PackedGraph`] always remains the general directed-multigraph
/// authority. This classification describes only which exact negative filter
/// its derived point-query index can use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphTopology {
    /// Every strongly connected component is a singleton and no self-loop exists.
    DirectedAcyclic,
    /// Cycles were condensed mechanically into an acyclic component order.
    StronglyConnectedCondensation {
        /// Number of strongly connected components in the derived order.
        components: u32,
    },
}

#[derive(Debug)]
struct TopologyIndex {
    generation: GraphGeneration,
    class: GraphTopology,
    component_of: Box<[u32]>,
    component_rank: Box<[u32]>,
}

impl TopologyIndex {
    fn build<K, N, I, E, R>(graph: &PackedGraph<K, N, I, E, R>) -> Self {
        let mut components = kosaraju_scc(&graph.petgraph_view());
        for component in &mut components {
            component.sort_unstable();
        }
        components.sort_unstable_by_key(|component| component.first().copied());

        let mut component_of = vec![0_u32; graph.nodes.len()];
        for (component, nodes) in components.iter().enumerate() {
            let component = dense_index(component);
            for &node in nodes {
                component_of[node as usize] = component;
            }
        }

        let mut component_edges = graph
            .edges
            .iter()
            .filter_map(|edge| {
                let source = component_of[edge.source as usize];
                let target = component_of[edge.target as usize];
                (source != target).then_some((source, target))
            })
            .collect::<Vec<_>>();
        component_edges.sort_unstable();
        component_edges.dedup();

        let mut outgoing = vec![Vec::<u32>::new(); components.len()];
        let mut indegree = vec![0_usize; components.len()];
        for (source, target) in component_edges {
            outgoing[source as usize].push(target);
            indegree[target as usize] = indegree[target as usize]
                .checked_add(1)
                .expect("component indegree cannot exceed packed edge cardinality");
        }
        let mut ready = indegree
            .iter()
            .enumerate()
            .filter_map(|(component, &degree)| (degree == 0).then_some(dense_index(component)))
            .collect::<BTreeSet<_>>();
        let mut component_rank = vec![0_u32; components.len()];
        let mut next_rank = 0_u32;
        while let Some(component) = ready.pop_first() {
            component_rank[component as usize] = next_rank;
            next_rank = next_rank
                .checked_add(1)
                .expect("component count is bounded by the dense u32 node universe");
            for &target in &outgoing[component as usize] {
                indegree[target as usize] = indegree[target as usize]
                    .checked_sub(1)
                    .expect("component indegree was constructed from the same edge set");
                if indegree[target as usize] == 0 {
                    ready.insert(target);
                }
            }
        }
        debug_assert_eq!(next_rank as usize, components.len());

        let has_self_loop = graph.edges.iter().any(|edge| edge.source == edge.target);
        let class = if components.len() == graph.nodes.len() && !has_self_loop {
            GraphTopology::DirectedAcyclic
        } else {
            GraphTopology::StronglyConnectedCondensation {
                components: dense_index(components.len()),
            }
        };
        Self {
            generation: graph.generation,
            class,
            component_of: component_of.into(),
            component_rank: component_rank.into(),
        }
    }

    fn definitely_unreachable(&self, source: u32, target: u32) -> bool {
        let source_component = self.component_of[source as usize];
        let target_component = self.component_of[target as usize];
        source_component != target_component
            && self.component_rank[source_component as usize]
                >= self.component_rank[target_component as usize]
    }
}

#[derive(Debug, Clone)]
struct CachedPath {
    generation: GraphGeneration,
    source: u32,
    target: u32,
    path: Option<ReasonPath>,
}

/// Generation-bound exact point reachability over one [`PackedGraph`].
///
/// The index retains only SCC membership, one topological rank per component,
/// and the last exact query result. It contains no peer adjacency. For
/// component map `c` and condensation rank `tau`, it may reject only when:
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/reachability-query.display.mathml"
))]
///
/// Every possible hit is resolved against the graph's forward and reverse CSR
/// indexes. The result is the same deterministic shortest [`ReasonPath`] as
/// [`PackedGraph::reachability`]. Its one-entry cache is explicitly fenced by
/// [`GraphGeneration`]; neither the index nor a cache hit is semantic evidence,
/// authority, admission, or proof of a runtime occurrence.
#[derive(Debug)]
pub struct ReachabilityIndex<'a, K, N, I, E, R> {
    graph: &'a PackedGraph<K, N, I, E, R>,
    topology: TopologyIndex,
    cached: Option<CachedPath>,
}

impl<K, N, I, E, R> PackedGraph<K, N, I, E, R> {
    /// Build the generation-local point-query projection.
    ///
    /// SCC condensation and ranking happen once, off the query path. The
    /// projection borrows this graph and cannot outlive or mutate it.
    #[must_use]
    pub fn reachability_index(&self) -> ReachabilityIndex<'_, K, N, I, E, R> {
        ReachabilityIndex {
            graph: self,
            topology: TopologyIndex::build(self),
            cached: None,
        }
    }

    fn canonical_path_between(&self, source: u32, target: u32) -> Option<ReasonPath> {
        if source == target {
            return Some(ReasonPath {
                nodes: vec![self.node(source as usize)],
                edges: Vec::new(),
            });
        }

        let mut distance_to_target = vec![u32::MAX; self.nodes.len()];
        distance_to_target[target as usize] = 0;
        let mut queue = VecDeque::from([target]);
        while let Some(current) = queue.pop_front() {
            let distance = distance_to_target[current as usize]
                .checked_add(1)
                .expect("a shortest simple path cannot exceed the u32 node universe");
            for &edge in
                self.edge_ids_unchecked(self.node(current as usize), petgraph::Direction::Incoming)
            {
                let predecessor = self.edges[edge as usize].source;
                if distance_to_target[predecessor as usize] == u32::MAX {
                    distance_to_target[predecessor as usize] = distance;
                    queue.push_back(predecessor);
                }
            }
            if distance_to_target[source as usize] != u32::MAX {
                break;
            }
        }
        if distance_to_target[source as usize] == u32::MAX {
            return None;
        }

        let mut cursor = source;
        let mut nodes = vec![self.node(source as usize)];
        let mut edges = Vec::with_capacity(distance_to_target[source as usize] as usize);
        while cursor != target {
            let expected = distance_to_target[cursor as usize]
                .checked_sub(1)
                .expect("non-target path nodes have positive remaining distance");
            let edge = self
                .edge_ids_unchecked(self.node(cursor as usize), petgraph::Direction::Outgoing)
                .iter()
                .copied()
                .find(|&edge| {
                    let neighbor = self.edges[edge as usize].target;
                    distance_to_target[neighbor as usize] == expected
                })
                .expect("reverse distance labels contain one canonical next edge");
            cursor = self.edges[edge as usize].target;
            edges.push(EdgeHandle {
                generation: self.generation,
                index: edge,
            });
            nodes.push(self.node(cursor as usize));
        }
        Some(ReasonPath { nodes, edges })
    }
}

impl<K, N, I, E, R> ReachabilityIndex<'_, K, N, I, E, R> {
    /// Return the exact mechanical topology classification.
    #[must_use]
    pub const fn topology(&self) -> GraphTopology {
        self.topology.class
    }

    /// Return the canonical shortest path from `source` to `target` when one exists.
    ///
    /// The SCC/topological filter may prove a negative without traversal. Every
    /// other query uses a reverse-distance search over canonical CSR slices,
    /// then selects the first admissible outgoing edge at each distance. This
    /// preserves the same parallel-edge and shortest-path tie-break as the
    /// reference breadth-first proof.
    ///
    /// # Errors
    ///
    /// Rejects a foreign or out-of-range source or target handle before
    /// consulting the cache.
    pub fn path(
        &mut self,
        source: NodeHandle,
        target: NodeHandle,
    ) -> Result<Option<ReasonPath>, GraphAccessError> {
        let source = dense_index(self.graph.validate_node(source)?);
        let target = dense_index(self.graph.validate_node(target)?);
        if let Some(cached) = &self.cached {
            if cached.generation == self.topology.generation
                && cached.source == source
                && cached.target == target
            {
                return Ok(cached.path.clone());
            }
        }
        let path = if self.topology.definitely_unreachable(source, target) {
            None
        } else {
            self.graph.canonical_path_between(source, target)
        };
        self.cached = Some(CachedPath {
            generation: self.topology.generation,
            source,
            target,
            path: path.clone(),
        });
        Ok(path)
    }
}

#[cfg(test)]
#[path = "../../tests/internal/reachability_query.rs"]
mod tests;
