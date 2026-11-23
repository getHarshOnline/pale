//! Equivalence laws for topology rejection, exact caching, and point witnesses.

use super::*;

fn policy() -> crate::AdaptiveNodeSetPolicy {
    crate::AdaptiveNodeSetPolicy::new(2, 5_000).expect("valid test policy")
}

fn graph(node_count: u8, edge_mask: u64) -> PackedGraph<u8, (), u16, (), u16> {
    let mut edges = Vec::new();
    let mut identity = 0_u16;
    for source in 0..node_count {
        for target in 0..node_count {
            let bit = u64::from(source) * u64::from(node_count) + u64::from(target);
            if edge_mask & (1_u64 << bit) != 0 {
                edges.push(super::super::GraphEdge {
                    id: identity,
                    source,
                    target,
                    payload: (),
                    reason: identity,
                });
                identity += 1;
            }
        }
    }
    if edge_mask & 2 != 0 {
        edges.push(super::super::GraphEdge {
            id: identity,
            source: 0,
            target: 1,
            payload: (),
            reason: identity,
        });
    }
    PackedGraph::build(
        (0..node_count)
            .rev()
            .map(|key| super::super::GraphNode { key, payload: () })
            .collect(),
        edges,
        policy(),
    )
    .expect("valid generated graph")
}

fn edge_indices(path: Option<ReasonPath>) -> Option<Vec<u32>> {
    path.map(|path| path.edges.into_iter().map(EdgeHandle::index).collect())
}

fn has_cached<K, N, I, E, R>(
    index: &ReachabilityIndex<'_, K, N, I, E, R>,
    source: NodeHandle,
    target: NodeHandle,
) -> bool {
    index.cached.as_ref().is_some_and(|cached| {
        cached.generation == index.topology.generation
            && cached.source == source.index()
            && cached.target == target.index()
    })
}

#[test]
fn point_index_matches_canonical_breadth_first_witness_exhaustively() {
    const NODES: u8 = 3;
    for edge_mask in 0_u64..(1_u64 << (NODES * NODES)) {
        let graph = graph(NODES, edge_mask);
        let mut index = graph.reachability_index();
        for source in 0..NODES {
            let source = graph.find(&source).expect("source exists");
            let reference = graph.reachability(source).expect("reference traversal");
            for target in 0..NODES {
                let target = graph.find(&target).expect("target exists");
                assert_eq!(
                    edge_indices(index.path(source, target).expect("point query")),
                    edge_indices(reference.path_to(target).expect("reference path")),
                    "edge_mask={edge_mask:#011b} source={} target={}",
                    source.index(),
                    target.index()
                );
            }
        }
    }
}

#[test]
fn topology_classification_preserves_general_multigraph_edges() {
    let dag = graph(3, (1 << 1) | (1 << 5));
    assert_eq!(
        dag.reachability_index().topology(),
        GraphTopology::DirectedAcyclic
    );

    let cyclic = graph(3, (1 << 1) | (1 << 3));
    assert_eq!(
        cyclic.reachability_index().topology(),
        GraphTopology::StronglyConnectedCondensation { components: 2 }
    );
    assert_eq!(
        cyclic.edge_count(),
        3,
        "parallel edges remain canonical facts"
    );
}

#[test]
fn cache_is_exact_and_foreign_handles_still_fail_closed() {
    let packed = graph(3, (1 << 1) | (1 << 5));
    let foreign = graph(3, (1 << 1) | (1 << 5));
    let source = packed.find(&0).expect("source exists");
    let target = packed.find(&2).expect("target exists");
    let mut index = packed.reachability_index();
    let first = index.path(source, target).expect("cold path");
    assert!(has_cached(&index, source, target));
    assert_eq!(index.path(source, target).expect("warm path"), first);
    assert_eq!(
        index.path(source, foreign.find(&2).expect("foreign target")),
        Err(GraphAccessError::ForeignGeneration)
    );
    assert!(has_cached(&index, source, target));
}

#[test]
fn rank_rejection_is_exact_for_reverse_dag_queries() {
    let graph = graph(4, (1 << 1) | (1 << 6) | (1 << 11));
    let mut index = graph.reachability_index();
    let first = graph.find(&0).expect("first node");
    let last = graph.find(&3).expect("last node");
    assert!(index.path(first, last).expect("forward query").is_some());
    assert_eq!(index.path(last, first).expect("reverse query"), None);
}
