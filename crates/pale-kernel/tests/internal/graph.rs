//! Compares packed-graph algorithms with independent graph references and deterministic laws.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::atomic::AtomicU64;

use petgraph::algo::{kosaraju_scc, toposort};
use petgraph::graph::DiGraph;
use petgraph::visit::{EdgeRef as _, IntoEdgeReferences as _, IntoEdgesDirected as _};

use super::*;
use crate::AdaptiveNodeSetEncoding;

type TestGraph = PackedGraph<&'static str, (), u8, (), &'static str>;

fn policy() -> AdaptiveNodeSetPolicy {
    AdaptiveNodeSetPolicy::new(2, 5_000).unwrap()
}

fn graph_with_policy(reverse_input: bool, set_policy: AdaptiveNodeSetPolicy) -> TestGraph {
    let mut nodes = vec![
        GraphNode {
            key: "d",
            payload: (),
        },
        GraphNode {
            key: "a",
            payload: (),
        },
        GraphNode {
            key: "c",
            payload: (),
        },
        GraphNode {
            key: "b",
            payload: (),
        },
    ];
    let mut edges = vec![
        GraphEdge {
            id: 3,
            source: "b",
            target: "c",
            payload: (),
            reason: "bc",
        },
        GraphEdge {
            id: 1,
            source: "a",
            target: "b",
            payload: (),
            reason: "ab-first",
        },
        GraphEdge {
            id: 2,
            source: "a",
            target: "b",
            payload: (),
            reason: "ab-parallel",
        },
    ];
    if reverse_input {
        nodes.reverse();
        edges.reverse();
    }
    TestGraph::build(nodes, edges, set_policy).unwrap()
}

fn graph(reverse_input: bool) -> TestGraph {
    graph_with_policy(reverse_input, policy())
}

#[derive(Debug, PartialEq, Eq)]
struct ReferenceProof {
    reached: BTreeSet<u8>,
    predecessor: BTreeMap<u8, (u8, u16)>,
}

impl ReferenceProof {
    fn edge_path_to(&self, target: u8) -> Option<Vec<u16>> {
        if !self.reached.contains(&target) {
            return None;
        }
        let mut edges = Vec::new();
        let mut cursor = target;
        while let Some(&(prior, edge)) = self.predecessor.get(&cursor) {
            edges.push(edge);
            cursor = prior;
        }
        edges.reverse();
        Some(edges)
    }
}

fn reference_closure(
    node_count: u8,
    edges: &[(u16, u8, u8)],
    origins: &[u8],
    direction: Direction,
) -> ReferenceProof {
    let mut canonical_edges = edges.to_vec();
    canonical_edges.sort_unstable_by_key(|&(id, source, target)| (source, target, id));
    let mut reached = origins.iter().copied().collect::<BTreeSet<_>>();
    assert!(reached.iter().all(|node| *node < node_count));
    let mut queue = reached.iter().copied().collect::<VecDeque<_>>();
    let mut predecessor = BTreeMap::new();
    while let Some(current) = queue.pop_front() {
        for &(edge, source, target) in &canonical_edges {
            let neighbor = match direction {
                Direction::Outgoing if source == current => Some(target),
                Direction::Incoming if target == current => Some(source),
                _ => None,
            };
            if let Some(neighbor) = neighbor.filter(|neighbor| reached.insert(*neighbor)) {
                predecessor.insert(neighbor, (current, edge));
                queue.push_back(neighbor);
            }
        }
    }
    ReferenceProof {
        reached,
        predecessor,
    }
}

fn canonical_components(mut components: Vec<Vec<u32>>) -> Vec<Vec<u32>> {
    for component in &mut components {
        component.sort_unstable();
    }
    components.sort_unstable();
    components
}

#[test]
fn canonical_build_preserves_parallel_edges_and_petgraph_view() {
    let left = graph(false);
    let right = graph(true);
    assert_ne!(left.generation(), right.generation());
    assert_eq!(left.edge_count(), 3);
    assert_eq!(
        (0..u32::try_from(left.edge_count()).expect("bounded edge count"))
            .map(|index| *left.edge_identity(left.edge_at(index).unwrap()).unwrap().0)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(
        (0..u32::try_from(left.node_count()).expect("bounded node count"))
            .map(|index| left
                .node_key(left.node_at(index).unwrap())
                .copied()
                .unwrap())
            .collect::<Vec<_>>(),
        (0..u32::try_from(right.node_count()).expect("bounded node count"))
            .map(|index| right
                .node_key(right.node_at(index).unwrap())
                .copied()
                .unwrap())
            .collect::<Vec<_>>()
    );
    let view = left.petgraph_view();
    assert!(toposort(&view, None).is_ok());
    assert_eq!(kosaraju_scc(&view).len(), 4);
}

#[test]
fn generation_exhaustion_is_a_typed_build_error() {
    let exhausted = AtomicU64::new(u64::MAX);
    let result = TestGraph::build_with_generation_counter(
        vec![GraphNode {
            key: "a",
            payload: (),
        }],
        Vec::new(),
        policy(),
        &exhausted,
    );
    assert_eq!(result, Err(GraphBuildError::GenerationExhausted));
}

#[test]
fn graph_owned_policy_is_the_only_public_set_construction_authority() {
    let set_policy = AdaptiveNodeSetPolicy::new(1, 8_000).unwrap();
    let graph = graph_with_policy(false, set_policy);
    let a = graph.find(&"a").unwrap();
    let b = graph.find(&"b").unwrap();
    let c = graph.find(&"c").unwrap();
    let d = graph.find(&"d").unwrap();

    assert_eq!(graph.set_policy(), set_policy);
    let singleton = graph.node_set([a]).unwrap();
    assert_eq!(singleton.encoding(), AdaptiveNodeSetEncoding::SmallSorted);
    assert_eq!(
        graph.node_set([a, b]).unwrap().encoding(),
        AdaptiveNodeSetEncoding::Roaring
    );
    assert_eq!(
        graph.node_set([a, b, c, d]).unwrap().encoding(),
        AdaptiveNodeSetEncoding::Dense
    );
    assert_eq!(
        singleton
            .union(&graph.node_set([b]).unwrap())
            .unwrap()
            .encoding(),
        AdaptiveNodeSetEncoding::Roaring
    );
    assert_eq!(
        graph
            .node_set([a, b, c, d])
            .unwrap()
            .difference(&graph.node_set([d]).unwrap())
            .unwrap()
            .encoding(),
        AdaptiveNodeSetEncoding::Roaring
    );
    assert_eq!(
        graph.reachability(a).unwrap().reached().encoding(),
        AdaptiveNodeSetEncoding::Roaring
    );
}

#[test]
fn adjacency_resume_preserves_parallel_edges_and_remaining_length() {
    for reverse_input in [false, true] {
        let graph = graph_with_policy(reverse_input, policy());
        for (node, direction, expected_ids) in [
            ("a", Direction::Outgoing, vec![1, 2]),
            ("b", Direction::Incoming, vec![1, 2]),
            ("b", Direction::Outgoing, vec![3]),
            ("c", Direction::Incoming, vec![3]),
            ("d", Direction::Outgoing, vec![]),
        ] {
            let node = graph.find(&node).unwrap();
            for offset in (0..=expected_ids.len() + 1).chain([usize::MAX]) {
                let mut edges = graph.edges_directed(node, direction).unwrap();
                assert_eq!(
                    edges.size_hint(),
                    (expected_ids.len(), Some(expected_ids.len()))
                );
                let resumed = edges.nth(offset);
                assert_eq!(
                    resumed.map(|edge| *graph.edge_identity(edge).unwrap().0),
                    expected_ids.get(offset).copied()
                );
                let consumed = offset.saturating_add(1).min(expected_ids.len());
                let remaining = &expected_ids[consumed..];
                assert_eq!(edges.len(), remaining.len());
                assert_eq!(edges.size_hint(), (remaining.len(), Some(remaining.len())));
                let rest = edges.by_ref().collect::<Vec<_>>();
                assert!(rest
                    .iter()
                    .all(|edge| edge.generation() == graph.generation()));
                assert_eq!(
                    rest.into_iter()
                        .map(|edge| *graph.edge_identity(edge).unwrap().0)
                        .collect::<Vec<_>>(),
                    remaining
                );
                assert_eq!(edges.len(), 0);
                assert_eq!(edges.next(), None);
                assert_eq!(edges.nth(usize::MAX), None);
            }
            let mut pages = Vec::new();
            for offset in 0..expected_ids.len() {
                pages.extend(
                    graph
                        .edges_directed(node, direction)
                        .unwrap()
                        .skip(offset)
                        .take(1)
                        .map(|edge| *graph.edge_identity(edge).unwrap().0),
                );
            }
            assert_eq!(pages, expected_ids);
        }
    }
}

#[test]
fn typed_edge_filtering_and_petgraph_edge_views_preserve_parallel_edges() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum EdgeKind {
        Propagates,
        Annotation,
    }

    let graph = PackedGraph::build(
        ["a", "b", "c"]
            .into_iter()
            .map(|key| GraphNode { key, payload: () })
            .collect(),
        vec![
            GraphEdge {
                id: 2_u8,
                source: "a",
                target: "b",
                payload: EdgeKind::Annotation,
                reason: "annotation",
            },
            GraphEdge {
                id: 1,
                source: "a",
                target: "b",
                payload: EdgeKind::Propagates,
                reason: "dependency",
            },
            GraphEdge {
                id: 3,
                source: "b",
                target: "c",
                payload: EdgeKind::Propagates,
                reason: "dependency",
            },
        ],
        policy(),
    )
    .unwrap();
    let a = graph.find(&"a").unwrap();
    let b = graph.find(&"b").unwrap();

    let outgoing = graph
        .edges_directed(a, Direction::Outgoing)
        .unwrap()
        .collect::<Vec<_>>();
    assert_eq!(outgoing.len(), 2);
    assert_eq!(
        outgoing
            .iter()
            .filter(|&&edge| graph.edge_payload(edge).unwrap().0 == &EdgeKind::Propagates)
            .map(|edge| *graph.edge_identity(*edge).unwrap().0)
            .collect::<Vec<_>>(),
        vec![1]
    );
    assert_eq!(
        graph
            .edges_directed(b, Direction::Incoming)
            .unwrap()
            .count(),
        2
    );

    let view = graph.petgraph_view();
    let all_edges = (&view).edge_references().collect::<Vec<_>>();
    assert_eq!(all_edges.len(), 3);
    assert_eq!(
        all_edges
            .iter()
            .map(|edge| (edge.id(), edge.source(), edge.target(), *edge.weight()))
            .collect::<Vec<_>>(),
        vec![
            (0, 0, 1, EdgeKind::Propagates),
            (1, 0, 1, EdgeKind::Annotation),
            (2, 1, 2, EdgeKind::Propagates),
        ]
    );
    assert_eq!((&view).edges_directed(0, Direction::Outgoing).count(), 2);
}

#[test]
fn exhaustive_petgraph_topology_and_scc_match_owned_reference_graphs() {
    const NODE_COUNT: u8 = 3;
    for edge_mask in 0_u16..(1 << (NODE_COUNT * NODE_COUNT)) {
        let mut edges = Vec::new();
        for source in 0..NODE_COUNT {
            for target in 0..NODE_COUNT {
                let bit = source * NODE_COUNT + target;
                if edge_mask & (1 << bit) != 0 {
                    edges.push(GraphEdge {
                        id: u16::from(bit),
                        source,
                        target,
                        payload: (),
                        reason: (),
                    });
                }
            }
        }
        let graph = PackedGraph::build(
            (0..NODE_COUNT)
                .map(|key| GraphNode { key, payload: () })
                .collect(),
            edges,
            policy(),
        )
        .unwrap();

        let mut reference = DiGraph::<(), ()>::new();
        let nodes = (0..NODE_COUNT)
            .map(|_| reference.add_node(()))
            .collect::<Vec<_>>();
        for source in (0..NODE_COUNT).rev() {
            for target in (0..NODE_COUNT).rev() {
                let bit = source * NODE_COUNT + target;
                if edge_mask & (1 << bit) != 0 {
                    reference.add_edge(nodes[source as usize], nodes[target as usize], ());
                }
            }
        }

        let view = graph.petgraph_view();
        let actual_topology = toposort(&view, None)
            .map_err(|cycle| cycle.node_id())
            .map(|nodes| nodes.into_iter().collect::<Vec<_>>());
        let reference_topology = toposort(&reference, None)
            .map_err(|cycle| u32::try_from(cycle.node_id().index()).expect("bounded node index"))
            .map(|nodes| {
                nodes
                    .into_iter()
                    .map(|node| u32::try_from(node.index()).expect("bounded node index"))
                    .collect::<Vec<_>>()
            });
        assert_eq!(
            actual_topology, reference_topology,
            "topology edge mask {edge_mask:#05x}"
        );

        let actual_components = canonical_components(kosaraju_scc(&view));
        let reference_components = canonical_components(
            kosaraju_scc(&reference)
                .into_iter()
                .map(|component| {
                    component
                        .into_iter()
                        .map(|node| u32::try_from(node.index()).expect("bounded node index"))
                        .collect()
                })
                .collect(),
        );
        assert_eq!(
            actual_components, reference_components,
            "SCC edge mask {edge_mask:#05x}"
        );
    }
}

#[test]
fn canonical_identity_and_endpoints_resolve_reason_handles() {
    let graph = graph(false);
    let a = graph.find(&"a").unwrap();
    let b = graph.find(&"b").unwrap();
    let proof = graph.reachability(a).unwrap();
    assert_eq!(graph.node_key(a), Ok(&"a"));
    let edge = proof.path_to(b).unwrap().unwrap().edges[0];
    assert_eq!(graph.edge_identity(edge), Ok((&1, a, b)));
    let witness_ids = proof
        .witness_edges()
        .map(|edge| *graph.edge_identity(edge).unwrap().0)
        .collect::<Vec<_>>();
    assert_eq!(witness_ids, [1, 3]);
}

#[test]
fn duplicate_identity_missing_endpoint_and_foreign_handles_fail_closed() {
    let nodes = vec![GraphNode {
        key: "a",
        payload: (),
    }];
    let duplicate = vec![
        GraphEdge {
            id: 1,
            source: "a",
            target: "a",
            payload: (),
            reason: "x",
        },
        GraphEdge {
            id: 1,
            source: "a",
            target: "a",
            payload: (),
            reason: "y",
        },
    ];
    assert_eq!(
        TestGraph::build(nodes.clone(), duplicate, policy()),
        Err(GraphBuildError::DuplicateEdge(1))
    );
    assert!(matches!(
        TestGraph::build(
            nodes,
            vec![GraphEdge {
                id: 1,
                source: "a",
                target: "z",
                payload: (),
                reason: "x"
            }],
            policy()
        ),
        Err(GraphBuildError::MissingNode("z"))
    ));
    let foreign_graph = graph(false);
    let graph = graph(false);
    let foreign = foreign_graph.find(&"a").unwrap();
    assert_eq!(
        graph.reachability(foreign),
        Err(GraphAccessError::ForeignGeneration)
    );
    assert_eq!(graph.node_at(99), Err(GraphAccessError::OutOfRange));
    assert_eq!(graph.edge_at(99), Err(GraphAccessError::OutOfRange));
}

#[test]
fn independently_built_graphs_reject_every_foreign_handle_surface() {
    let left = graph(false);
    let right = graph(false);
    let foreign_node = left.find(&"a").unwrap();
    let foreign_edge = left
        .edge_at(0)
        .expect("the fixture has a canonical packed edge");

    assert_eq!(
        right.node_key(foreign_node),
        Err(GraphAccessError::ForeignGeneration)
    );
    assert_eq!(
        right.node_payload(foreign_node),
        Err(GraphAccessError::ForeignGeneration)
    );
    assert_eq!(
        right.reachability(foreign_node),
        Err(GraphAccessError::ForeignGeneration)
    );
    assert_eq!(
        right.reverse_impact([foreign_node]),
        Err(GraphAccessError::ForeignGeneration)
    );
    assert_eq!(
        right.edge_payload(foreign_edge),
        Err(GraphAccessError::ForeignGeneration)
    );
    assert_eq!(
        right.edge_identity(foreign_edge),
        Err(GraphAccessError::ForeignGeneration)
    );
}

#[test]
fn seed_order_does_not_change_reverse_impact_witnesses() {
    let graph = graph(false);
    let b = graph.find(&"b").unwrap();
    let c = graph.find(&"c").unwrap();
    let left = graph.reverse_impact([b, c]).unwrap();
    let right = graph.reverse_impact([c, b, c]).unwrap();
    assert_eq!(left, right);
    assert!(left.affected().is_disjoint(left.unaffected()).unwrap());
    assert_eq!(
        left.affected().len() + left.unaffected().len(),
        graph.node_count()
    );
    let a = graph.find(&"a").unwrap();
    let witness = left.proof().path_to(a).unwrap().unwrap();
    assert_eq!(witness.nodes.last(), Some(&a));
    assert_eq!(witness.edges.len(), 1);
}

#[test]
fn packed_reachability_matches_pure_reference_interpreter() {
    let graph = graph(false);
    let origin = graph.find(&"a").unwrap();
    let proof = graph.reachability(origin).unwrap();
    let reference = reference_closure(
        4,
        &[(1, 0, 1), (2, 0, 1), (3, 1, 2)],
        &[0],
        Direction::Outgoing,
    );
    let resolved = proof
        .reached()
        .iter()
        .map(|node| match *graph.node_key(node).unwrap() {
            "a" => 0,
            "b" => 1,
            "c" => 2,
            "d" => 3,
            unexpected => panic!("unexpected fixture key {unexpected}"),
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(resolved, reference.reached);

    let lengths = proof
        .reached()
        .iter()
        .map(|node| {
            let path = proof.path_to(node).unwrap().unwrap();
            (*graph.node_key(node).unwrap(), path.edges.len())
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(lengths, BTreeMap::from([("a", 0), ("b", 1), ("c", 2)]));
    let c = graph.find(&"c").unwrap();
    let edge_ids = proof
        .path_to(c)
        .unwrap()
        .unwrap()
        .edges
        .into_iter()
        .map(|edge| u16::from(*graph.edge_identity(edge).unwrap().0))
        .collect::<Vec<_>>();
    assert_eq!(edge_ids, reference.edge_path_to(2).unwrap());
}

#[test]
fn reference_witness_order_is_independent_of_edge_identity_order() {
    let facts = [(100_u16, 0_u8, 1_u8), (1, 0, 2), (50, 1, 3), (2, 2, 3)];
    let graph = PackedGraph::build(
        (0_u8..4)
            .map(|key| GraphNode { key, payload: () })
            .collect(),
        facts
            .iter()
            .rev()
            .map(|&(id, source, target)| GraphEdge {
                id,
                source,
                target,
                payload: (),
                reason: id,
            })
            .collect(),
        policy(),
    )
    .unwrap();

    let outgoing = graph.reachability(graph.find(&0).unwrap()).unwrap();
    let outgoing_edges = outgoing
        .path_to(graph.find(&3).unwrap())
        .unwrap()
        .unwrap()
        .edges
        .into_iter()
        .map(|edge| *graph.edge_identity(edge).unwrap().0)
        .collect::<Vec<_>>();
    assert_eq!(
        outgoing_edges,
        reference_closure(4, &facts, &[0], Direction::Outgoing)
            .edge_path_to(3)
            .unwrap()
    );

    let incoming = graph.reverse_impact([graph.find(&3).unwrap()]).unwrap();
    let incoming_edges = incoming
        .proof
        .path_to(graph.find(&0).unwrap())
        .unwrap()
        .unwrap()
        .edges
        .into_iter()
        .map(|edge| *graph.edge_identity(edge).unwrap().0)
        .collect::<Vec<_>>();
    assert_eq!(
        incoming_edges,
        reference_closure(4, &facts, &[3], Direction::Incoming)
            .edge_path_to(0)
            .unwrap()
    );
}

#[test]
fn exhaustive_small_multigraphs_refine_the_pure_reference() {
    const NODE_COUNT: u8 = 3;

    for edge_mask in 0_u16..(1 << (NODE_COUNT * NODE_COUNT)) {
        let nodes = (0..NODE_COUNT)
            .map(|key| GraphNode { key, payload: () })
            .collect::<Vec<_>>();
        let mut facts = Vec::new();
        for source in 0..NODE_COUNT {
            for target in 0..NODE_COUNT {
                let bit = source * NODE_COUNT + target;
                if edge_mask & (1 << bit) != 0 {
                    facts.push((1_000 - u16::from(bit), source, target));
                }
            }
        }
        if edge_mask & (1 << 1) != 0 {
            facts.push((2_000, 0, 1));
        }
        let mut authored_edges = facts
            .iter()
            .map(|&(id, source, target)| GraphEdge {
                id,
                source,
                target,
                payload: (),
                reason: id,
            })
            .collect::<Vec<_>>();
        if edge_mask % 2 == 1 {
            authored_edges.reverse();
        }
        let graph = PackedGraph::build(nodes, authored_edges, policy()).unwrap();

        for seed_mask in 1_u8..(1 << NODE_COUNT) {
            let seeds = (0..NODE_COUNT)
                .filter(|node| seed_mask & (1 << node) != 0)
                .collect::<Vec<_>>();
            let reference = reference_closure(NODE_COUNT, &facts, &seeds, Direction::Incoming);
            let handles = seeds
                .iter()
                .map(|seed| graph.find(seed).unwrap())
                .collect::<Vec<_>>();
            let impact = graph.reverse_impact(handles.clone()).unwrap();
            let affected = impact
                .affected()
                .iter()
                .map(|node| *graph.node_key(node).unwrap())
                .collect::<BTreeSet<_>>();
            let unaffected = impact
                .unaffected()
                .iter()
                .map(|node| *graph.node_key(node).unwrap())
                .collect::<BTreeSet<_>>();
            assert_eq!(
                affected, reference.reached,
                "edge={edge_mask:#05x} seed={seed_mask:#05b}"
            );
            assert_eq!(
                unaffected,
                (0..NODE_COUNT)
                    .filter(|node| !reference.reached.contains(node))
                    .collect(),
                "edge={edge_mask:#05x} seed={seed_mask:#05b}"
            );
            for target in &reference.reached {
                let edge_path = impact
                    .proof()
                    .path_to(graph.find(target).unwrap())
                    .unwrap()
                    .unwrap()
                    .edges
                    .into_iter()
                    .map(|edge| *graph.edge_identity(edge).unwrap().0)
                    .collect::<Vec<_>>();
                assert_eq!(
                    edge_path,
                    reference.edge_path_to(*target).unwrap(),
                    "edge={edge_mask:#05x} seed={seed_mask:#05b} target={target}"
                );
            }
            let permuted = handles
                .iter()
                .rev()
                .copied()
                .chain(handles.first().copied())
                .collect::<Vec<_>>();
            assert_eq!(graph.reverse_impact(permuted).unwrap(), impact);
        }
    }
}

#[test]
fn deterministic_multigraph_simulation_matches_reverse_reference_and_witnesses() {
    for case in 0_u8..12 {
        let nodes = (0_u8..6)
            .map(|key| GraphNode { key, payload: () })
            .collect::<Vec<_>>();
        let mut facts = Vec::new();
        let mut edges = Vec::new();
        let mut id = 0_u16;
        for source in 0_u8..6 {
            for target in 0_u8..6 {
                if (source * 7 + target * 3 + case) % 5 == 0 {
                    facts.push((source, target));
                    edges.push(GraphEdge {
                        id,
                        source,
                        target,
                        payload: (),
                        reason: id,
                    });
                    id += 1;
                }
            }
        }
        let graph = PackedGraph::build(nodes, edges, policy()).unwrap();
        let seed = graph.find(&(case % 6)).unwrap();
        let impact = graph.reverse_impact([seed]).unwrap();

        let mut reference = BTreeSet::from([case % 6]);
        let mut queue = VecDeque::from([case % 6]);
        while let Some(target) = queue.pop_front() {
            for (source, _) in facts.iter().filter(|(_, candidate)| *candidate == target) {
                if reference.insert(*source) {
                    queue.push_back(*source);
                }
            }
        }
        let actual = impact
            .affected()
            .iter()
            .map(|node| *graph.node_key(node).unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, reference);
        assert_eq!(
            impact.affected().len() + impact.unaffected().len(),
            graph.node_count()
        );
        for node in impact.affected() {
            let path = impact.proof().path_to(node).unwrap().unwrap();
            for (pair, edge) in path.nodes.windows(2).zip(path.edges.iter()) {
                let (_, source, target) = graph.edge_identity(*edge).unwrap();
                assert_eq!((source, target), (pair[1], pair[0]));
            }
        }
    }
}

#[test]
fn borrowed_comparator_lookup_needs_no_owned_query() {
    let graph = PackedGraph::build(
        vec![
            GraphNode {
                key: "alpha".to_owned(),
                payload: (),
            },
            GraphNode {
                key: "beta".to_owned(),
                payload: (),
            },
        ],
        Vec::<GraphEdge<u8, String, (), u8>>::new(),
        policy(),
    )
    .unwrap();
    let borrowed = "beta";
    let handle = graph
        .find_by(|candidate| candidate.as_str().cmp(borrowed))
        .expect("borrowed key resolves");
    assert_eq!(graph.node_key(handle).unwrap(), borrowed);
    assert!(graph
        .find_by(|candidate| candidate.as_str().cmp("missing"))
        .is_none());
}
