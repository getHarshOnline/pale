//! Exact GraphBLAS-to-reference reachability equivalence over cyclic and parallel graphs.

use std::collections::BTreeSet;

use super::*;

#[test]
fn graphblas_support_algebra_matches_canonical_cpu_sets() {
    let endpoints = [
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
            .into_iter()
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
    let seeds = [vec![], vec![0], vec![1, 4], vec![7], vec![0, 0]];

    for direction in [Direction::Outgoing, Direction::Incoming] {
        let handles = seeds
            .iter()
            .map(|seeds| {
                seeds
                    .iter()
                    .map(|&index| graph.node_at(index).expect("test node"))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut accelerated = graph.batch_reachability();
        assert_eq!(
            accelerated.backend(),
            BatchReachabilityBackend::GraphblasCpu,
            "the feature-gated proof must execute GraphBLAS rather than fallback"
        );
        let actual = accelerated
            .query(direction, handles.clone())
            .expect("accelerated batch query");
        assert_eq!(
            accelerated.backend(),
            BatchReachabilityBackend::GraphblasCpu,
            "a GraphBLAS failure must not be hidden by the CPU fallback in this proof"
        );

        let mut reference = graph.batch_reachability();
        reference.accelerator = None;
        let expected = reference
            .query(direction, handles)
            .expect("canonical CPU batch query");
        assert_eq!(
            actual
                .iter()
                .map(|set| set.iter().map(NodeHandle::index).collect::<BTreeSet<_>>())
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(|set| set.iter().map(NodeHandle::index).collect::<BTreeSet<_>>())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn graphblas_boundary_preserves_empty_and_edgeless_graph_semantics() {
    type TestGraph = PackedGraph<u32, (), u32, (), ()>;

    let policy = AdaptiveNodeSetPolicy::new(2, 5_000).expect("valid test policy");
    let empty = TestGraph::build(Vec::new(), Vec::new(), policy).expect("valid empty graph");
    let mut empty_batch = empty.batch_reachability();
    let empty_result = empty_batch
        .query(Direction::Outgoing, [Vec::<NodeHandle>::new()])
        .expect("empty graph closure");
    assert_eq!(empty_result.len(), 1);
    assert!(empty_result[0].is_empty());

    let edgeless = TestGraph::build(
        (0..3).map(|key| GraphNode { key, payload: () }).collect(),
        Vec::new(),
        policy,
    )
    .expect("valid edgeless graph");
    let seed = edgeless.node_at(1).expect("middle node");
    let mut accelerated = edgeless.batch_reachability();
    assert_eq!(
        accelerated.backend(),
        BatchReachabilityBackend::GraphblasCpu,
        "a nonempty zero-edge matrix must remain a valid accelerator"
    );
    for direction in [Direction::Outgoing, Direction::Incoming] {
        let sets = accelerated
            .query(direction, [vec![seed], Vec::new()])
            .expect("edgeless graph closure");
        assert_eq!(
            sets[0].iter().map(NodeHandle::index).collect::<Vec<_>>(),
            vec![1]
        );
        assert!(sets[1].is_empty());
    }
}

#[test]
fn repeated_accelerator_construction_shares_process_runtime_safely() {
    let graph = PackedGraph::build(
        (0..3).map(|key| GraphNode { key, payload: () }).collect(),
        vec![GraphEdge {
            id: 1_u32,
            source: 0,
            target: 1,
            payload: (),
            reason: (),
        }],
        AdaptiveNodeSetPolicy::new(2, 5_000).expect("valid test policy"),
    )
    .expect("valid test graph");
    let seed = graph.node_at(0).expect("origin");
    let mut first = graph.batch_reachability();
    let mut second = graph.batch_reachability();
    assert_eq!(first.backend(), BatchReachabilityBackend::GraphblasCpu);
    assert_eq!(second.backend(), BatchReachabilityBackend::GraphblasCpu);
    assert_eq!(
        first
            .query(Direction::Outgoing, [vec![seed]])
            .expect("first closure"),
        second
            .query(Direction::Outgoing, [vec![seed]])
            .expect("second closure")
    );
}
