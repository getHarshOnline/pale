//! Exercises generation, runtime, and scratch arena ownership, staleness, and reset boundaries.

use super::*;
use crate::{AdaptiveNodeSetPolicy, GraphNode, PackedGraph};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn graph() -> PackedGraph<u8, (), u8, (), ()> {
    PackedGraph::<u8, (), u8, (), ()>::build(
        vec![GraphNode {
            key: 0,
            payload: (),
        }],
        Vec::new(),
        AdaptiveNodeSetPolicy::new(0, 0).unwrap(),
    )
    .unwrap()
}

fn generation() -> GraphGeneration {
    graph().generation()
}

struct DropProbe(Arc<AtomicUsize>);

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn immutable_handles_reject_cross_arena_and_generation() {
    let graph = PackedGraph::<u8, (), u8, (), ()>::build(
        vec![GraphNode {
            key: 0,
            payload: (),
        }],
        Vec::new(),
        AdaptiveNodeSetPolicy::new(0, 0).unwrap(),
    )
    .unwrap();
    let left = GenerationArena::seal_for_graph(&graph, vec!["left"]).unwrap();
    assert!(matches!(
        GenerationArena::<&str>::seal_for_graph(&graph, Vec::new()),
        Err(ArenaError::AlignmentMismatch)
    ));
    let right = GenerationArena::seal(generation(), vec!["right"]).unwrap();
    assert_eq!(left.get(left.handle(0).unwrap()), Ok(&"left"));
    assert_eq!(right.get(left.handle(0).unwrap()), Err(ArenaError::Foreign));
}

#[test]
fn runtime_reuse_rejects_stale_aba_handle() {
    let mut arena = RuntimeArena::new(1).unwrap();
    let old = arena.insert("old").unwrap();
    *arena.get_mut(old).unwrap() = "updated";
    assert_eq!(arena.get(old), Ok(&"updated"));
    assert_eq!(arena.remove(old), Ok("updated"));
    assert!(matches!(arena.get_mut(old), Err(ArenaError::Stale)));
    let new = arena.insert("new").unwrap();
    assert_eq!(arena.get(old), Err(ArenaError::Stale));
    assert_eq!(arena.get(new), Ok(&"new"));
    assert_eq!(arena.insert("overflow"), Err(ArenaError::Exhausted));

    let mut foreign = RuntimeArena::new(1).unwrap();
    assert!(matches!(foreign.get_mut(new), Err(ArenaError::Foreign)));
}

#[test]
fn removal_validation_proves_generation_advance_without_mutation() {
    let mut arena = RuntimeArena::new(1).unwrap();
    let handle = arena.insert("retained").unwrap();
    assert_eq!(arena.validate_removal(handle), Ok(()));
    assert_eq!(arena.get(handle), Ok(&"retained"));

    arena.slots[handle.slot as usize].generation = u32::MAX;
    let exhausted = ArenaHandle {
        slot_generation: u32::MAX,
        ..handle
    };
    assert_eq!(
        arena.validate_removal(exhausted),
        Err(ArenaError::SlotGenerationExhausted)
    );
    assert_eq!(
        arena.remove(exhausted),
        Err(ArenaError::SlotGenerationExhausted)
    );
    assert_eq!(arena.get(exhausted), Ok(&"retained"));
}

#[test]
fn runtime_store_is_not_falsely_affiliated_with_one_compiled_graph() {
    let first = graph().generation();
    let second = graph().generation();
    assert_ne!(first, second);

    let mut arena = RuntimeArena::new(2).unwrap();
    let first_handle = arena.insert(first).unwrap();
    let second_handle = arena.insert(second).unwrap();
    assert_eq!(arena.get(first_handle), Ok(&first));
    assert_eq!(arena.get(second_handle), Ok(&second));
}

#[test]
fn scratch_reset_rejects_prior_epoch_and_cross_kind_handles() {
    let graph = graph();
    let mut scratch = ScratchArena::new(1).unwrap();
    let old = scratch.insert(7).unwrap();
    scratch.reset().unwrap();
    assert_eq!(scratch.get(old), Err(ArenaError::Foreign));
    let immutable = GenerationArena::seal_for_graph(&graph, vec![7]).unwrap();
    assert_eq!(
        scratch.get(immutable.handle(0).unwrap()),
        Err(ArenaError::Foreign)
    );
}

#[test]
#[cfg(target_pointer_width = "64")]
fn constructors_reject_capacity_beyond_dense_handle_space() {
    let invalid = u32::MAX as usize + 1;
    assert!(matches!(
        RuntimeArena::<()>::new(invalid),
        Err(ArenaError::InvalidCapacity)
    ));
    assert!(matches!(
        ScratchArena::<()>::new(invalid),
        Err(ArenaError::InvalidCapacity)
    ));
}

#[test]
fn bounded_arenas_reserve_their_declared_capacity_before_admission() {
    let capacity = 64;
    let runtime = RuntimeArena::<u64>::new(capacity).unwrap();
    assert!(runtime.slots.capacity() >= capacity);
    assert!(runtime.free.capacity() >= capacity);

    let scratch = ScratchArena::<u64>::new(capacity).unwrap();
    assert!(scratch.values.capacity() >= capacity);
}

#[test]
fn populated_runtime_churn_preserves_capacity_and_rejects_every_stale_handle() {
    let capacity = 64;
    let mut arena = RuntimeArena::new(capacity).unwrap();
    let original = (0..capacity)
        .map(|value| arena.insert(value).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(arena.insert(capacity), Err(ArenaError::Exhausted));

    let slots_capacity = arena.slots.capacity();
    let free_capacity = arena.free.capacity();
    for handle in original.iter().step_by(2).copied() {
        arena.remove(handle).unwrap();
    }
    let replacements = (0..capacity / 2)
        .map(|value| arena.insert(capacity + value).unwrap())
        .collect::<Vec<_>>();

    assert_eq!(arena.slots.capacity(), slots_capacity);
    assert_eq!(arena.free.capacity(), free_capacity);
    assert!(original
        .iter()
        .step_by(2)
        .all(|handle| arena.get(*handle) == Err(ArenaError::Stale)));
    assert!(replacements.iter().all(|handle| arena.get(*handle).is_ok()));
}

#[test]
fn generation_drop_and_scratch_reset_reclaim_owned_bodies() {
    let graph = PackedGraph::<u8, (), u8, (), ()>::build(
        vec![
            GraphNode {
                key: 0,
                payload: (),
            },
            GraphNode {
                key: 1,
                payload: (),
            },
        ],
        Vec::new(),
        AdaptiveNodeSetPolicy::new(0, 0).unwrap(),
    )
    .unwrap();
    let immutable_drops = Arc::new(AtomicUsize::new(0));
    {
        let arena = GenerationArena::seal_for_graph(
            &graph,
            vec![
                DropProbe(Arc::clone(&immutable_drops)),
                DropProbe(Arc::clone(&immutable_drops)),
            ],
        )
        .unwrap();
        assert_eq!(immutable_drops.load(Ordering::Relaxed), 0);
        drop(arena);
    }
    assert_eq!(immutable_drops.load(Ordering::Relaxed), 2);

    let scratch_drops = Arc::new(AtomicUsize::new(0));
    let mut scratch = ScratchArena::new(2).unwrap();
    scratch
        .insert(DropProbe(Arc::clone(&scratch_drops)))
        .unwrap();
    scratch
        .insert(DropProbe(Arc::clone(&scratch_drops)))
        .unwrap();
    scratch.reset().unwrap();
    assert_eq!(scratch_drops.load(Ordering::Relaxed), 2);

    let runtime_drops = Arc::new(AtomicUsize::new(0));
    let mut runtime = RuntimeArena::new(2).unwrap();
    let removed = runtime
        .insert(DropProbe(Arc::clone(&runtime_drops)))
        .unwrap();
    runtime
        .insert(DropProbe(Arc::clone(&runtime_drops)))
        .unwrap();
    drop(runtime.remove(removed).unwrap());
    assert_eq!(runtime_drops.load(Ordering::Relaxed), 1);
    drop(runtime);
    assert_eq!(runtime_drops.load(Ordering::Relaxed), 2);
}
