//! Reference laws for set-only batch reachability and generation fencing.

use std::collections::{BTreeSet, VecDeque};

use super::*;

type TestGraph = PackedGraph<u32, (), u32, (), ()>;

fn graph_fixture() -> (TestGraph, Vec<(u32, u32)>) {
    let endpoints = vec![
        (0, 1),
        (0, 1),
        (1, 2),
        (2, 0),
        (2, 3),
        (4, 5),
        (5, 6),
        (6, 4),
    ];
    let graph = PackedGraph::build(
        (0..8)
            .rev()
            .map(|key| GraphNode { key, payload: () })
            .collect(),
        endpoints
            .iter()
            .copied()
            .enumerate()
            .map(|(index, (source, target))| GraphEdge {
                id: u32::try_from(index).expect("bounded test edge"),
                source,
                target,
                payload: (),
                reason: (),
            })
            .collect(),
        AdaptiveNodeSetPolicy::new(2, 5_000).expect("valid test policy"),
    )
    .expect("valid test graph");
    (graph, endpoints)
}

fn reference(endpoints: &[(u32, u32)], direction: Direction, seeds: &[u32]) -> BTreeSet<u32> {
    let mut adjacency = vec![Vec::new(); 8];
    for &(source, target) in endpoints {
        let (owner, neighbor) = match direction {
            Direction::Outgoing => (source, target),
            Direction::Incoming => (target, source),
        };
        adjacency[owner as usize].push(neighbor);
    }
    let mut reached = seeds.iter().copied().collect::<BTreeSet<_>>();
    let mut queue = reached.iter().copied().collect::<VecDeque<_>>();
    while let Some(node) = queue.pop_front() {
        for &neighbor in &adjacency[node as usize] {
            if reached.insert(neighbor) {
                queue.push_back(neighbor);
            }
        }
    }
    reached
}

fn reached(set: &AdaptiveNodeSet) -> BTreeSet<u32> {
    set.iter().map(NodeHandle::index).collect()
}

#[test]
fn cpu_batch_matches_independent_fixed_point_in_both_directions() {
    let (graph, endpoints) = graph_fixture();
    let seeds = vec![vec![], vec![0], vec![1, 4], vec![7], vec![0, 0]];
    let handles = |batch: &[Vec<u32>]| {
        batch
            .iter()
            .map(|seeds| {
                seeds
                    .iter()
                    .map(|&index| graph.node_at(index).expect("test node"))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };

    for direction in [Direction::Outgoing, Direction::Incoming] {
        let mut index = graph.batch_reachability();
        #[cfg(all(feature = "graphblas", not(target_family = "wasm")))]
        {
            index.accelerator = None;
        }
        assert_eq!(index.backend(), BatchReachabilityBackend::CanonicalCpu);
        let actual = index
            .query(direction, handles(&seeds))
            .expect("canonical batch query");
        let expected = seeds
            .iter()
            .map(|seeds| reference(&endpoints, direction, seeds))
            .collect::<Vec<_>>();
        assert_eq!(actual.iter().map(reached).collect::<Vec<_>>(), expected);
    }
}

#[test]
fn every_batch_handle_is_validated_before_query_execution() {
    let (graph, _) = graph_fixture();
    let (foreign, _) = graph_fixture();
    let mut index = graph.batch_reachability();
    let backend = index.backend();
    assert_eq!(
        index.query(
            Direction::Outgoing,
            [vec![
                graph.node_at(0).expect("local node"),
                foreign.node_at(0).expect("foreign node"),
            ]],
        ),
        Err(GraphAccessError::ForeignGeneration)
    );
    assert_eq!(index.backend(), backend);
}

#[test]
fn canonical_constructor_is_the_exact_cpu_baseline() {
    let (graph, endpoints) = graph_fixture();
    let handles = [
        vec![graph.node_at(0).expect("origin")],
        vec![graph.node_at(7).expect("isolated node")],
    ];
    let mut canonical = graph.canonical_batch_reachability();
    assert_eq!(canonical.backend(), BatchReachabilityBackend::CanonicalCpu);
    let actual = canonical
        .query(Direction::Outgoing, handles.clone())
        .expect("canonical batch");
    let expected = [vec![0], vec![7]]
        .iter()
        .map(|seeds| reference(&endpoints, Direction::Outgoing, seeds))
        .collect::<Vec<_>>();
    assert_eq!(actual.iter().map(reached).collect::<Vec<_>>(), expected);
}
