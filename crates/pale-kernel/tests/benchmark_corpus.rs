//! Correctness envelope for the synthetic resident-kernel benchmark corpus.
//!
//! These tests validate generators, reference mechanics, and the exact public
//! operations timed by the feature-gated benchmark. They establish algorithmic
//! parity only; they are not performance evidence or a policy selection.

#[path = "support/resident_kernel_benchmark.rs"]
mod resident_kernel_benchmark;

#[path = "support/resident_kernel_measurement.rs"]
mod resident_kernel_measurement;

#[path = "internal/benchmark_measurement.rs"]
mod benchmark_measurement;

use std::collections::BTreeSet;

use pale_kernel::{ArenaError, Direction, GenerationArena, RuntimeArena, ScratchArena};
use petgraph::algo::{kosaraju_scc, toposort};

use resident_kernel_benchmark::{
    characterization_set_cases, expected_benchmark_rows, reference_closure,
    synthetic_runtime_bodies, synthetic_set_policy, Cardinality, GraphFixture, GraphPattern,
    SetFixture, SetPattern, CHARACTERIZATION_SET_CARDINALITIES, CHARACTERIZATION_SET_FIXTURE_COUNT,
    CHARACTERIZATION_SET_OVERLAPS, CHARACTERIZATION_SET_PATTERNS, CHARACTERIZATION_SET_UNIVERSES,
};

fn indices(set: &pale_kernel::AdaptiveNodeSet) -> BTreeSet<u32> {
    set.iter().map(pale_kernel::NodeHandle::index).collect()
}

#[test]
fn synthetic_set_grid_obeys_reference_algebra_across_encodings() {
    let cases = [
        (256, Cardinality::Absolute(1), SetPattern::Clustered, 0),
        (256, Cardinality::Absolute(32), SetPattern::Strided, 5_000),
        (
            4_096,
            Cardinality::Permyriad(1_000),
            SetPattern::FixedScatter,
            9_000,
        ),
        (
            65_536,
            Cardinality::Permyriad(5_000),
            SetPattern::Strided,
            5_000,
        ),
        (
            262_144,
            Cardinality::Permyriad(9_000),
            SetPattern::Clustered,
            0,
        ),
    ];

    for (universe, cardinality, pattern, overlap) in cases {
        let fixture = SetFixture::new(
            universe,
            cardinality,
            pattern,
            overlap,
            synthetic_set_policy(),
        );
        let left_reference = fixture
            .members
            .left
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let right_reference = fixture
            .members
            .right
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let left = fixture
            .graph
            .node_set(fixture.handles(&fixture.members.left))
            .unwrap();
        let right = fixture
            .graph
            .node_set(fixture.handles(&fixture.members.right))
            .unwrap();

        assert_eq!(indices(&left), left_reference);
        assert_eq!(indices(&right), right_reference);
        assert_eq!(left.len(), left_reference.len());
        assert!(!left.is_empty());
        assert!(left_reference.iter().all(|index| left
            .contains(fixture.graph.node_at(*index).unwrap())
            .unwrap()));
        assert_eq!(
            indices(&left.union(&right).unwrap()),
            left_reference.union(&right_reference).copied().collect()
        );
        assert_eq!(
            indices(&left.intersection(&right).unwrap()),
            left_reference
                .intersection(&right_reference)
                .copied()
                .collect()
        );
        assert_eq!(
            indices(&left.difference(&right).unwrap()),
            left_reference
                .difference(&right_reference)
                .copied()
                .collect()
        );
        assert_eq!(
            left.is_subset(&right).unwrap(),
            left_reference.is_subset(&right_reference)
        );
        assert_eq!(
            left.is_disjoint(&right).unwrap(),
            left_reference.is_disjoint(&right_reference)
        );
        let chained = left
            .union(&right)
            .unwrap()
            .difference(&right)
            .unwrap()
            .intersection(&left)
            .unwrap();
        assert_eq!(
            indices(&chained),
            left_reference
                .difference(&right_reference)
                .copied()
                .collect()
        );

        let actual_overlap = left_reference.intersection(&right_reference).count();
        assert_eq!(
            actual_overlap, fixture.members.effective_overlap as usize,
            "requested overlap {} permyriad must be reported separately from its feasible value",
            fixture.members.requested_overlap_permyriad
        );
    }
}

#[test]
fn synthetic_graph_grid_matches_naive_closure_and_reason_paths() {
    let patterns = [
        GraphPattern::Chain,
        GraphPattern::Layered,
        GraphPattern::Fanout,
        GraphPattern::Disjoint,
        GraphPattern::SccBand,
        GraphPattern::Parallel,
    ];
    for pattern in patterns {
        let fixture = GraphFixture::new(64, pattern, synthetic_set_policy());
        let forward_reference = reference_closure(64, &fixture.edges, &[0], Direction::Outgoing);
        let forward = fixture.graph.reachability(fixture.origin()).unwrap();
        assert_eq!(indices(forward.reached()), forward_reference.reached);

        let reverse_reference = reference_closure(64, &fixture.edges, &[63], Direction::Incoming);
        let reverse = fixture
            .graph
            .reverse_impact([fixture.impact_seed()])
            .unwrap();
        assert_eq!(indices(reverse.affected()), reverse_reference.reached);
        assert!(reverse
            .affected()
            .is_disjoint(reverse.unaffected())
            .unwrap());
        assert_eq!(
            reverse
                .affected()
                .union(reverse.unaffected())
                .unwrap()
                .len(),
            fixture.graph.node_count()
        );

        for target in 0..64 {
            let target_handle = fixture.graph.node_at(target).unwrap();
            let actual = forward.path_to(target_handle).unwrap().map(|path| {
                path.edges
                    .into_iter()
                    .map(|edge| *fixture.graph.edge_payload(edge).unwrap().1)
                    .collect::<Vec<_>>()
            });
            assert_eq!(actual, forward_reference.edge_path_to(target));
        }

        let view = fixture.graph.petgraph_view();
        let components = kosaraju_scc(&view);
        assert!(!components.is_empty());
        assert_eq!(
            toposort(&view, None).is_ok(),
            pattern != GraphPattern::SccBand
        );
    }
}

#[test]
fn generated_declarations_are_deterministic_before_process_local_sealing() {
    assert_eq!(Cardinality::Permyriad(10).resolve(256), 1);
    for pattern in [
        GraphPattern::Chain,
        GraphPattern::Layered,
        GraphPattern::Fanout,
        GraphPattern::Disjoint,
        GraphPattern::SccBand,
        GraphPattern::Parallel,
    ] {
        assert_eq!(
            resident_kernel_benchmark::graph_edges(256, pattern),
            resident_kernel_benchmark::graph_edges(256, pattern)
        );
    }
    assert_eq!(
        resident_kernel_benchmark::paired_set_members(4_096, 410, SetPattern::FixedScatter, 5_000),
        resident_kernel_benchmark::paired_set_members(4_096, 410, SetPattern::FixedScatter, 5_000)
    );
}

#[test]
fn characterization_set_grid_is_the_complete_deterministic_cartesian_product() {
    let cases = characterization_set_cases().collect::<Vec<_>>();
    assert_eq!(cases.len(), CHARACTERIZATION_SET_FIXTURE_COUNT);
    assert_eq!(CHARACTERIZATION_SET_FIXTURE_COUNT, 288);
    assert_eq!(cases.iter().copied().collect::<BTreeSet<_>>().len(), 288);
    assert_eq!(
        cases.first(),
        Some(&(
            CHARACTERIZATION_SET_UNIVERSES[0],
            CHARACTERIZATION_SET_CARDINALITIES[0],
            CHARACTERIZATION_SET_PATTERNS[0],
            CHARACTERIZATION_SET_OVERLAPS[0],
        ))
    );
    assert_eq!(
        cases.last(),
        Some(&(
            *CHARACTERIZATION_SET_UNIVERSES.last().unwrap(),
            *CHARACTERIZATION_SET_CARDINALITIES.last().unwrap(),
            *CHARACTERIZATION_SET_PATTERNS.last().unwrap(),
            *CHARACTERIZATION_SET_OVERLAPS.last().unwrap(),
        ))
    );
}

#[test]
fn benchmark_row_contract_is_exact_and_feature_aware() {
    assert_eq!(expected_benchmark_rows(true, false, false), 39);
    assert_eq!(expected_benchmark_rows(true, true, false), 44);
    assert_eq!(expected_benchmark_rows(false, false, false), 3_192);
    assert_eq!(expected_benchmark_rows(false, true, false), 3_426);
    assert_eq!(expected_benchmark_rows(true, false, true), 15);
    assert_eq!(expected_benchmark_rows(true, true, true), 20);
    assert_eq!(expected_benchmark_rows(false, false, true), 270);
    assert_eq!(expected_benchmark_rows(false, true, true), 504);
    assert_eq!(
        expected_benchmark_rows(
            false,
            cfg!(all(feature = "graphblas", not(target_family = "wasm"))),
            false,
        ),
        if cfg!(all(feature = "graphblas", not(target_family = "wasm"))) {
            3_426
        } else {
            3_192
        }
    );
}

#[test]
fn arena_cases_preserve_alignment_reuse_and_reset_fences() {
    for size in [64_u32, 4_096] {
        let fixture = GraphFixture::new(size, GraphPattern::Chain, synthetic_set_policy());
        let values = (0..size).collect::<Vec<_>>();
        let generation = GenerationArena::seal_for_graph(&fixture.graph, values).unwrap();
        for index in [0, size / 2, size - 1] {
            let handle = generation.handle(index as usize).unwrap();
            assert_eq!(generation.get(handle), Ok(&index));
        }

        let bodies = synthetic_runtime_bodies(size);
        let mut runtime = RuntimeArena::new(size as usize).unwrap();
        let handles = bodies
            .iter()
            .copied()
            .map(|body| runtime.insert(body).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(runtime.insert(bodies[0]), Err(ArenaError::Exhausted));
        let first = handles[(size / 2) as usize];
        let mut replacement = runtime.remove(first).unwrap();
        replacement.advance();
        let second = runtime.insert(replacement).unwrap();
        assert_eq!(runtime.get(first), Err(ArenaError::Stale));
        assert_eq!(runtime.get(second), Ok(&replacement));

        let mut scratch = ScratchArena::new(size as usize).unwrap();
        let old = scratch.insert(7_u32).unwrap();
        scratch.reset().unwrap();
        assert_eq!(scratch.get(old), Err(ArenaError::Foreign));
        let current = scratch.insert(8_u32).unwrap();
        assert_eq!(scratch.get(current), Ok(&8));
    }
}
