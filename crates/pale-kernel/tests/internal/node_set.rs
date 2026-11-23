//! Checks adaptive node-set encodings against one representation-independent finite-set algebra.

use std::collections::BTreeSet;

use super::*;
use crate::{GraphNode, PackedGraph};

const ENCODINGS: [AdaptiveNodeSetEncoding; 3] = [
    AdaptiveNodeSetEncoding::SmallSorted,
    AdaptiveNodeSetEncoding::Dense,
    AdaptiveNodeSetEncoding::Roaring,
];

fn policy_for(encoding: AdaptiveNodeSetEncoding) -> AdaptiveNodeSetPolicy {
    match encoding {
        AdaptiveNodeSetEncoding::SmallSorted => {
            AdaptiveNodeSetPolicy::new(u32::MAX, 10_000).unwrap()
        }
        AdaptiveNodeSetEncoding::Dense => AdaptiveNodeSetPolicy::new(0, 0).unwrap(),
        AdaptiveNodeSetEncoding::Roaring => AdaptiveNodeSetPolicy::new(0, 10_000).unwrap(),
    }
}

fn generation() -> GraphGeneration {
    PackedGraph::<u8, (), u8, (), ()>::build(
        vec![GraphNode {
            key: 0,
            payload: (),
        }],
        Vec::new(),
        AdaptiveNodeSetPolicy::new(0, 0).unwrap(),
    )
    .unwrap()
    .generation()
}

fn set(
    generation: GraphGeneration,
    universe: u32,
    values: impl IntoIterator<Item = u32>,
    encoding: AdaptiveNodeSetEncoding,
) -> AdaptiveNodeSet {
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort_unstable();
    values.dedup();
    AdaptiveNodeSet::from_canonical_with_encoding(
        generation,
        universe,
        values,
        policy_for(encoding),
        encoding,
    )
    .unwrap()
}

fn indices(set: &AdaptiveNodeSet) -> Vec<u32> {
    set.iter().map(NodeHandle::index).collect()
}

fn members(mask: u8, universe: u32) -> BTreeSet<u32> {
    (0..universe)
        .filter(|index| mask & (1 << index) != 0)
        .collect()
}

#[test]
fn explicit_policy_selects_each_representation_at_exact_boundaries() {
    let policy = AdaptiveNodeSetPolicy::new(2, 6_000).unwrap();
    assert_eq!(policy.small_cardinality_limit(), 2);
    assert_eq!(policy.dense_min_density_permyriad(), 6_000);
    assert_eq!(
        policy.encoding_for(10, 2),
        AdaptiveNodeSetEncoding::SmallSorted
    );
    assert_eq!(policy.encoding_for(10, 3), AdaptiveNodeSetEncoding::Roaring);
    assert_eq!(policy.encoding_for(10, 6), AdaptiveNodeSetEncoding::Dense);
    assert_eq!(policy.encoding_for(10, 10), AdaptiveNodeSetEncoding::Dense);
    assert_eq!(
        AdaptiveNodeSetPolicy::new(0, 10_001),
        Err(NodeSetError::InvalidDensityThreshold { permyriad: 10_001 })
    );
}

#[test]
fn exhaustive_representations_obey_the_same_finite_set_algebra() {
    const UNIVERSE: u32 = 5;
    let generation = generation();
    for left_mask in 0_u8..(1 << UNIVERSE) {
        for right_mask in 0_u8..(1 << UNIVERSE) {
            let expected_left = members(left_mask, UNIVERSE);
            let expected_right = members(right_mask, UNIVERSE);
            let expected_union = expected_left
                .union(&expected_right)
                .copied()
                .collect::<Vec<_>>();
            let expected_intersection = expected_left
                .intersection(&expected_right)
                .copied()
                .collect::<Vec<_>>();
            let expected_difference = expected_left
                .difference(&expected_right)
                .copied()
                .collect::<Vec<_>>();

            for left_encoding in ENCODINGS {
                for right_encoding in ENCODINGS {
                    let left = set(
                        generation,
                        UNIVERSE,
                        expected_left.iter().copied(),
                        left_encoding,
                    );
                    let right = set(
                        generation,
                        UNIVERSE,
                        expected_right.iter().copied(),
                        right_encoding,
                    );
                    assert_eq!(left.len(), expected_left.len());
                    assert_eq!(right.len(), expected_right.len());
                    assert_eq!(indices(&left.union(&right).unwrap()), expected_union);
                    assert_eq!(
                        indices(&left.intersection(&right).unwrap()),
                        expected_intersection
                    );
                    assert_eq!(
                        indices(&left.difference(&right).unwrap()),
                        expected_difference
                    );
                    assert_eq!(left.union(&right).unwrap(), right.union(&left).unwrap());
                    assert_eq!(
                        left.intersection(&right).unwrap(),
                        right.intersection(&left).unwrap()
                    );
                    let meet = left.intersection(&right).unwrap();
                    assert!(meet.is_subset(&left).unwrap());
                    assert!(meet.is_subset(&right).unwrap());
                    assert_eq!(
                        left.is_disjoint(&right).unwrap(),
                        expected_intersection.is_empty()
                    );
                    assert_eq!(left.union(&left).unwrap(), left);
                    assert_eq!(left.intersection(&left).unwrap(), left);
                    let complement = left.complement().unwrap();
                    assert_eq!(
                        indices(&complement),
                        (0..UNIVERSE)
                            .filter(|index| !expected_left.contains(index))
                            .collect::<Vec<_>>()
                    );
                    assert!(left.is_disjoint(&complement).unwrap());
                    assert_eq!(left.union(&complement).unwrap().len(), UNIVERSE as usize);
                }
            }
        }
    }
}

#[test]
fn sparse_encoding_retains_large_declared_universe() {
    let set = set(
        generation(),
        1_000_000,
        [7],
        AdaptiveNodeSetEncoding::SmallSorted,
    );
    assert_eq!(set.universe(), 1_000_000);
    assert_eq!(indices(&set), vec![7]);
}

#[test]
fn dense_word_count_accepts_the_complete_u32_universe() {
    assert_eq!(dense_word_count(0), Ok(0));
    assert_eq!(dense_word_count(1), Ok(1));
    assert_eq!(dense_word_count(64), Ok(1));
    assert_eq!(dense_word_count(65), Ok(2));
    assert_eq!(dense_word_count(u32::MAX), Ok(67_108_864));
}

#[test]
fn cross_generation_universe_and_membership_fail_closed() {
    let left = set(generation(), 2, [1], AdaptiveNodeSetEncoding::Roaring);
    let right = set(generation(), 2, [1], AdaptiveNodeSetEncoding::Roaring);
    assert_eq!(left.union(&right), Err(NodeSetError::CrossGeneration));

    let generation = generation();
    let a = set(generation, 2, [1], AdaptiveNodeSetEncoding::Dense);
    let b = set(generation, 3, [1], AdaptiveNodeSetEncoding::Dense);
    assert_eq!(a.intersection(&b), Err(NodeSetError::CrossUniverse));

    let policy = AdaptiveNodeSetPolicy::new(0, 0).unwrap();
    assert_eq!(
        AdaptiveNodeSet::from_indices(generation, 2, [2], policy),
        Err(NodeSetError::OutOfUniverse {
            index: 2,
            universe: 2,
        })
    );
}
