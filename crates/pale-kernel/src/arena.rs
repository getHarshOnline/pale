//! Typed arenas with explicit compiled-generation and local-lifetime identity.
//!
//! Arena handles are capabilities for process-local memory, not serialized
//! semantic identities or proof of admission, activation, release, or native
//! effect. Dropping an arena releases every owned body; no hidden global body
//! registry survives its lifetime.

use crate::{GraphGeneration, PackedGraph};
use std::marker::PhantomData;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

/// Monotone allocator for process-local arena identities.
static NEXT_ARENA: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum ArenaScope {
    /// Immutable bodies aligned with one compiled graph.
    Generation(GraphGeneration),
    /// Mutable bodies retained by one process-local store incarnation.
    Runtime,
    /// Temporary bodies invalidated together by an epoch reset.
    Scratch,
}

/// Opaque process-local identity of one typed arena.
///
/// Scope is part of equality: immutable stores retain their graph generation,
/// while runtime and scratch stores receive independent local incarnations.
/// Caller-chosen integers cannot forge this value, and it is deliberately
/// non-serializable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ArenaIdentity {
    /// Lifetime class and optional immutable-graph fence of this store.
    scope: ArenaScope,
    /// Monotone identity distinguishing same-kind arenas in this process.
    local: NonZeroU64,
}

impl ArenaIdentity {
    /// Allocate a non-zero process-local identity for one arena kind.
    ///
    /// # Errors
    ///
    /// Returns [`ArenaError::IdentityExhausted`] after the monotone allocator
    /// can no longer advance without wrapping.
    fn fresh(scope: ArenaScope) -> Result<Self, ArenaError> {
        let value = NEXT_ARENA
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| ArenaError::IdentityExhausted)?;
        Ok(Self {
            scope,
            local: NonZeroU64::new(value).ok_or(ArenaError::IdentityExhausted)?,
        })
    }
}

/// Typed handle into exactly one local arena generation.
///
/// Equality includes arena scope, local store identity, reset epoch, slot, and
/// slot generation. Immutable scope additionally carries the compiled graph
/// generation. A handle therefore cannot be replayed across rebuild, arena
/// replacement, scratch reset, or runtime-slot reuse.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArenaHandle<T> {
    /// Exact arena identity that issued the handle.
    arena: ArenaIdentity,
    /// Reset epoch, used by scratch arenas and zero elsewhere.
    epoch: u64,
    /// Dense slot within the issuing arena.
    slot: u32,
    /// Reuse generation of the selected runtime slot.
    slot_generation: u32,
    /// Invariant marker preventing handles for different body types from mixing.
    marker: PhantomData<fn() -> T>,
}

impl<T> Copy for ArenaHandle<T> {}
impl<T> Clone for ArenaHandle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

/// A bounded arena operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ArenaError {
    /// Configured capacity exceeds the dense handle space.
    #[error("arena capacity exceeds the dense handle space")]
    InvalidCapacity,
    /// The process allocator cannot reserve the configured bounded capacity.
    #[error("arena capacity cannot be reserved")]
    CapacityUnavailable,
    /// Immutable body count differs from its aligned packed graph.
    #[error("immutable arena body count does not match its packed graph")]
    AlignmentMismatch,
    /// A bounded runtime or scratch arena has no vacant slot.
    #[error("arena is at its configured capacity")]
    Exhausted,
    /// Process-local arena identities are exhausted.
    #[error("process-local arena identities are exhausted")]
    IdentityExhausted,
    /// A runtime slot exhausted its reuse generation.
    #[error("runtime slot generation is exhausted")]
    SlotGenerationExhausted,
    /// A scratch arena exhausted its reset epoch.
    #[error("scratch reset epoch is exhausted")]
    ResetEpochExhausted,
    /// The handle belongs to another arena kind, generation, or epoch.
    #[error("arena handle belongs to another arena generation")]
    Foreign,
    /// The slot is absent, removed, or has since been reused.
    #[error("arena handle is stale")]
    Stale,
}

/// Immutable bodies belonging to one compiled generation.
///
/// The shared arena law rendered on [`RuntimeArena`] distinguishes this type's
/// linear seal from the mutable arena's constant-time operations. Dropping the
/// arena reclaims all bodies together; it performs no external effect and
/// changes no graph topology.
#[derive(Debug)]
pub struct GenerationArena<T> {
    /// Process-local identity aligned with the owning packed graph.
    identity: ArenaIdentity,
    /// Immutable bodies in the same dense order as graph nodes.
    values: Box<[T]>,
}

impl<T> GenerationArena<T> {
    /// Consume bodies into the exact generation of one immutable packed graph.
    ///
    /// This is the external sealing seam for aligned graph/body stores. It
    /// prevents a caller from supplying an unrelated process-local generation
    /// while preserving the graph's opaque, non-serializable identity.
    ///
    /// # Errors
    ///
    /// Rejects more bodies than dense `u32` handles or exhausted local identity.
    pub fn seal_for_graph<K, N, I, E, R>(
        graph: &PackedGraph<K, N, I, E, R>,
        values: Vec<T>,
    ) -> Result<Self, ArenaError> {
        if values.len() != graph.node_count() {
            return Err(ArenaError::AlignmentMismatch);
        }
        Self::seal(graph.generation(), values)
    }

    /// Consume bodies into an immutable arena.
    ///
    /// # Errors
    ///
    /// Rejects more bodies than dense `u32` handles or exhausted local identity.
    pub(crate) fn seal(
        compiled_generation: GraphGeneration,
        values: Vec<T>,
    ) -> Result<Self, ArenaError> {
        if values.len() > u32::MAX as usize {
            return Err(ArenaError::InvalidCapacity);
        }
        Ok(Self {
            identity: ArenaIdentity::fresh(ArenaScope::Generation(compiled_generation))?,
            values: values.into(),
        })
    }

    /// Return a handle for one present body.
    #[must_use]
    pub fn handle(&self, index: usize) -> Option<ArenaHandle<T>> {
        let slot = u32::try_from(index).ok()?;
        (index < self.values.len()).then_some(ArenaHandle {
            arena: self.identity,
            epoch: 0,
            slot,
            slot_generation: 0,
            marker: PhantomData,
        })
    }

    /// Resolve a handle in its exact immutable arena.
    ///
    /// # Errors
    ///
    /// Rejects foreign and out-of-range handles.
    pub fn get(&self, handle: ArenaHandle<T>) -> Result<&T, ArenaError> {
        if handle.arena != self.identity || handle.epoch != 0 {
            return Err(ArenaError::Foreign);
        }
        self.values
            .get(handle.slot as usize)
            .ok_or(ArenaError::Stale)
    }
}

/// One runtime slot and the reuse generation fencing its current occupant.
#[derive(Debug)]
struct RuntimeSlot<T> {
    /// Generation advanced before a removed slot may be reused.
    generation: u32,
    /// Current body, or `None` while the slot is on the free list.
    value: Option<T>,
}

/// Bounded mutable arena whose slot generations reject stale and ABA handles.
///
/// A successful removal advances the slot generation before the slot can be
/// reused. In the shared law, the linear seal is
/// [`GenerationArena::seal_for_graph`], where `n` is the consumed immutable
/// body count. Its constant terms name [`Self::get`], [`Self::get_mut`],
/// [`Self::insert`], and [`Self::remove`]. Within runtime arena `A`, handle `h`
/// resolves slot `s` exactly when:
///
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/math/arena-liveness.display.mathml"))]
///
/// The pre-reserved insert, access, and removal bound is part of the rendered
/// arena law above.
#[derive(Debug)]
pub struct RuntimeArena<T> {
    /// Process-local runtime-store identity.
    identity: ArenaIdentity,
    /// Maximum number of simultaneously retained bodies.
    capacity: usize,
    /// Dense slots, reserved to `capacity` before construction succeeds.
    slots: Vec<RuntimeSlot<T>>,
    /// Vacant slot coordinates, also pre-reserved to the arena bound.
    free: Vec<u32>,
}

impl<T> RuntimeArena<T> {
    /// Create an empty process-local runtime store.
    ///
    /// The fresh store incarnation rejects handles from every other runtime
    /// store, including a reconstruction in the same process. Runtime bodies may
    /// refer to more than one compiled generation, so this arena is deliberately
    /// not aligned to a [`GraphGeneration`].
    ///
    /// # Errors
    ///
    /// Rejects capacity beyond dense `u32` handles, unavailable capacity, or
    /// exhausted local identity. Successful construction reserves storage for
    /// the full bound, so admission within that bound performs no vector growth.
    pub fn new(capacity: usize) -> Result<Self, ArenaError> {
        if capacity > u32::MAX as usize {
            return Err(ArenaError::InvalidCapacity);
        }
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(capacity)
            .map_err(|_| ArenaError::CapacityUnavailable)?;
        let mut free = Vec::new();
        free.try_reserve_exact(capacity)
            .map_err(|_| ArenaError::CapacityUnavailable)?;
        Ok(Self {
            identity: ArenaIdentity::fresh(ArenaScope::Runtime)?,
            capacity,
            slots,
            free,
        })
    }

    /// Insert a value, reusing a vacant slot only after generation advance.
    ///
    /// # Errors
    ///
    /// Returns [`ArenaError::Exhausted`] when no slot can be admitted.
    pub fn insert(&mut self, value: T) -> Result<ArenaHandle<T>, ArenaError> {
        if let Some(slot) = self.free.pop() {
            let slot = slot as usize;
            let generation = self.slots[slot].generation;
            self.slots[slot].value = Some(value);
            return Ok(self.handle(slot, generation));
        }
        if self.slots.len() == self.capacity {
            return Err(ArenaError::Exhausted);
        }
        let slot = self.slots.len();
        self.slots.push(RuntimeSlot {
            generation: 0,
            value: Some(value),
        });
        Ok(self.handle(slot, 0))
    }

    /// Resolve one live handle.
    ///
    /// # Errors
    ///
    /// Rejects foreign, removed, reused, or absent handles.
    pub fn get(&self, handle: ArenaHandle<T>) -> Result<&T, ArenaError> {
        self.slot(handle)?.value.as_ref().ok_or(ArenaError::Stale)
    }

    /// Mutably borrow one live value after exact arena and slot validation.
    ///
    /// Mutation preserves the handle because occupancy and slot generation do
    /// not change. Removing or reusing the slot still invalidates it.
    ///
    /// # Errors
    ///
    /// Rejects foreign, removed, reused, or absent handles.
    pub fn get_mut(&mut self, handle: ArenaHandle<T>) -> Result<&mut T, ArenaError> {
        self.slot(handle)?;
        self.slots[handle.slot as usize]
            .value
            .as_mut()
            .ok_or(ArenaError::Stale)
    }

    /// Validate that removing one live handle cannot exhaust its reuse fence.
    ///
    /// This is the non-mutating preparation step for a caller whose durable
    /// authority boundary must precede the mechanical removal. Exclusive
    /// ownership must preserve the arena between this check and [`Self::remove`].
    ///
    /// # Errors
    ///
    /// Rejects invalid handles and a slot whose generation cannot advance.
    pub fn validate_removal(&self, handle: ArenaHandle<T>) -> Result<(), ArenaError> {
        let slot = self.slot(handle)?;
        slot.generation
            .checked_add(1)
            .ok_or(ArenaError::SlotGenerationExhausted)?;
        Ok(())
    }

    /// Remove a value and advance its slot generation before reuse.
    ///
    /// # Errors
    ///
    /// Rejects invalid handles and fails closed at generation exhaustion.
    pub fn remove(&mut self, handle: ArenaHandle<T>) -> Result<T, ArenaError> {
        self.validate_removal(handle)?;
        let slot = &mut self.slots[handle.slot as usize];
        let next = slot.generation + 1;
        let value = slot.value.take().ok_or(ArenaError::Stale)?;
        slot.generation = next;
        self.free.push(handle.slot);
        Ok(value)
    }

    /// Construct the full fenced handle for one admitted slot.
    fn handle(&self, slot: usize, generation: u32) -> ArenaHandle<T> {
        ArenaHandle {
            arena: self.identity,
            epoch: 0,
            slot: u32::try_from(slot).expect("runtime capacity is bounded by dense handles"),
            slot_generation: generation,
            marker: PhantomData,
        }
    }

    /// Resolve one live slot after checking every arena and reuse fence.
    fn slot(&self, handle: ArenaHandle<T>) -> Result<&RuntimeSlot<T>, ArenaError> {
        if handle.arena != self.identity || handle.epoch != 0 {
            return Err(ArenaError::Foreign);
        }
        let slot = self
            .slots
            .get(handle.slot as usize)
            .ok_or(ArenaError::Stale)?;
        if slot.generation != handle.slot_generation || slot.value.is_none() {
            return Err(ArenaError::Stale);
        }
        Ok(slot)
    }
}

/// Bounded temporary arena invalidated atomically by [`Self::reset`].
///
/// Reset drops all temporary bodies, then advances the epoch. Hence every
/// pre-reset handle fails the equality fence even when a later value occupies
/// the same dense slot. Scratch lifetime is computational only and carries no
/// authority or persistence semantics.
#[derive(Debug)]
pub struct ScratchArena<T> {
    /// Process-local scratch-store identity.
    identity: ArenaIdentity,
    /// Current reset generation.
    epoch: u64,
    /// Maximum bodies admitted before the next reset.
    capacity: usize,
    /// Temporary bodies, reserved to `capacity` before construction succeeds.
    values: Vec<T>,
}

impl<T> ScratchArena<T> {
    /// Create an empty process-local scratch arena.
    ///
    /// # Errors
    ///
    /// Rejects capacity beyond dense `u32` handles, unavailable capacity, or
    /// exhausted local identity. Successful construction reserves storage for
    /// the full bound, so admission within that bound performs no vector growth.
    pub fn new(capacity: usize) -> Result<Self, ArenaError> {
        if capacity > u32::MAX as usize {
            return Err(ArenaError::InvalidCapacity);
        }
        let mut values = Vec::new();
        values
            .try_reserve_exact(capacity)
            .map_err(|_| ArenaError::CapacityUnavailable)?;
        Ok(Self {
            identity: ArenaIdentity::fresh(ArenaScope::Scratch)?,
            epoch: 0,
            capacity,
            values,
        })
    }

    /// Insert one temporary value.
    ///
    /// # Errors
    ///
    /// Returns [`ArenaError::Exhausted`] at the configured bound.
    pub fn insert(&mut self, value: T) -> Result<ArenaHandle<T>, ArenaError> {
        if self.values.len() == self.capacity {
            return Err(ArenaError::Exhausted);
        }
        let slot = u32::try_from(self.values.len()).map_err(|_| ArenaError::InvalidCapacity)?;
        self.values.push(value);
        Ok(ArenaHandle {
            arena: self.identity,
            epoch: self.epoch,
            slot,
            slot_generation: 0,
            marker: PhantomData,
        })
    }

    /// Resolve a handle from the current reset epoch.
    ///
    /// # Errors
    ///
    /// Rejects foreign, reset, or out-of-range handles.
    pub fn get(&self, handle: ArenaHandle<T>) -> Result<&T, ArenaError> {
        if handle.arena != self.identity || handle.epoch != self.epoch {
            return Err(ArenaError::Foreign);
        }
        self.values
            .get(handle.slot as usize)
            .ok_or(ArenaError::Stale)
    }

    /// Drop all values and invalidate every outstanding handle.
    ///
    /// # Errors
    ///
    /// Returns [`ArenaError::ResetEpochExhausted`] if the reset epoch is exhausted.
    pub fn reset(&mut self) -> Result<(), ArenaError> {
        let next = self
            .epoch
            .checked_add(1)
            .ok_or(ArenaError::ResetEpochExhausted)?;
        self.values.clear();
        self.epoch = next;
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/internal/arena.rs"]
mod tests;
