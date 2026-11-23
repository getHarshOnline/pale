//! Reconstructible process-local resident-session mechanics.
//!
//! [`GenerationLifecycle`] remains the semantic and durable oracle. This module
//! stores each exact admitted [`SessionAdmissionLeaseV1`] once in a bounded
//! [`RuntimeArena`]. Two mechanical indexes provide unique lookup by logical
//! session and reverse lookup by retained compiled generation; neither index
//! duplicates the session body.
//!
//! For lifecycle state `L`, working projection `P`, session identity `s`, and compiled
//! generation digest `g`, the session projection and reverse-index laws are:

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/session-pin-working-set.display.mathml"
))]
//!
//! Both indexes reserve their complete declared bound before construction
//! succeeds. They deliberately use sorted vectors: measured evidence at the
//! baseline, tenfold, and hundredfold workloads must earn a more complex
//! representation. This module carries no authority, serialization, public
//! handle, session admission, revocation, liveness, or generation-reclamation
//! decision.
use std::collections::BTreeMap;

use pale_core::records::{
    CompiledGenerationDigest, HarnessActivityIdentity, NativeSessionCorrelationV1,
    SessionAdmissionIdV1, SessionAdmissionLeaseDigest, SessionAdmissionLeaseV1,
};
use pale_kernel::{ArenaError, ArenaHandle, RuntimeArena};

use super::lifecycle::{GenerationLifecycle, SessionGenerationPin, SessionGenerationPinSnapshot};

/// One unique session lookup into the body arena.
#[derive(Debug)]
struct SessionIndexEntry {
    session_admission_id: SessionAdmissionIdV1,
    handle: ArenaHandle<SessionAdmissionLeaseV1>,
}

/// One generation-to-session reverse lookup into the same body arena.
#[derive(Debug)]
struct GenerationIndexEntry {
    generation: CompiledGenerationDigest,
    session_admission_id: SessionAdmissionIdV1,
    handle: ArenaHandle<SessionAdmissionLeaseV1>,
}

/// Bounded mutable projection of admitted resident sessions.
///
/// Each body is the exact [`SessionAdmissionLeaseV1`] admitted by the semantic
/// lifecycle. Bodies live for one process-local store incarnation and may refer
/// to several immutable compiled generations. Handles remain private to this
/// projection; reconstruction creates a fresh arena identity and invalidates
/// every prior handle by construction.
pub(super) struct ResidentSessionWorkingSet {
    capacity: usize,
    bodies: RuntimeArena<SessionAdmissionLeaseV1>,
    by_session: Vec<SessionIndexEntry>,
    by_generation: Vec<GenerationIndexEntry>,
}

/// Fully checked mechanical edit corresponding to one semantic pin transition.
///
/// Every owned key, index position, capacity condition, body coordinate, and
/// removal-generation fence is resolved before durable append. Applying this
/// value performs no semantic decision and allocates no additional capacity.
pub(super) enum PreparedResidentSessionMutation {
    /// Insert one admitted lease at both precomputed index positions.
    Insert {
        lease: Box<SessionAdmissionLeaseV1>,
        session_position: usize,
        generation_position: usize,
        session_admission_id: SessionAdmissionIdV1,
        generation_session_admission_id: SessionAdmissionIdV1,
        generation: CompiledGenerationDigest,
    },
    /// Remove one exact lease and its two precomputed index memberships.
    Remove {
        handle: ArenaHandle<SessionAdmissionLeaseV1>,
        session_position: usize,
        generation_position: usize,
    },
}

/// Ready resident-session projection or a closed recovery-required state.
///
/// Capacity refusal is non-mutating and leaves the projection ready. Any
/// structural contradiction, foreign/stale handle, or unexpected apply failure
/// closes all later projection mutation until cold reconstruction replaces the
/// complete process-local store.
pub(super) enum ResidentSessionProjection {
    /// The mechanical image agrees with its semantic lifecycle oracle.
    Ready(ResidentSessionWorkingSet),
    /// Projection integrity is lost; durable authority must be reconstructed.
    RecoveryRequired(ResidentSessionWorkingSetError),
}

/// Bounded lookup of one live lease by its privacy-preserving host activity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ResidentNativeSessionLookup {
    /// No live session exposes this native activity identity.
    Missing,
    /// Exactly one live session exposes this activity identity.
    Exact(SessionAdmissionLeaseDigest),
    /// More than one live session exposes the identity, so no lease is selected.
    Ambiguous,
}

impl ResidentSessionProjection {
    /// Start from one completely rebuilt projection.
    pub(super) const fn ready(working_set: ResidentSessionWorkingSet) -> Self {
        Self::Ready(working_set)
    }

    /// Reject mutation while cold reconstruction is required.
    pub(super) const fn require_ready(&self) -> Result<(), ResidentSessionWorkingSetError> {
        match self {
            Self::Ready(_) => Ok(()),
            Self::RecoveryRequired(reason) => Err(*reason),
        }
    }

    /// Close projection mutation after an owning semantic/projection invariant fails.
    pub(super) fn require_recovery(
        &mut self,
        reason: ResidentSessionWorkingSetError,
    ) -> ResidentSessionWorkingSetError {
        *self = Self::RecoveryRequired(reason);
        reason
    }

    /// Prepare a semantic pin insertion without changing projection state.
    pub(super) fn prepare_pin(
        &mut self,
        pin: SessionGenerationPin,
    ) -> Result<PreparedResidentSessionMutation, ResidentSessionWorkingSetError> {
        let result = match self {
            Self::Ready(working_set) => working_set.prepare_pin(pin),
            Self::RecoveryRequired(reason) => return Err(*reason),
        };
        self.close_on_integrity_failure(result)
    }

    /// Prepare an exact drain without changing projection state.
    pub(super) fn prepare_drain(
        &mut self,
        session_admission_id: &SessionAdmissionIdV1,
        lease_digest: &SessionAdmissionLeaseDigest,
    ) -> Result<PreparedResidentSessionMutation, ResidentSessionWorkingSetError> {
        let result = match self {
            Self::Ready(working_set) => {
                working_set.prepare_drain(session_admission_id, lease_digest)
            }
            Self::RecoveryRequired(reason) => return Err(*reason),
        };
        self.close_on_integrity_failure(result)
    }

    /// Apply a previously prepared edit or close the projection on contradiction.
    pub(super) fn apply(
        &mut self,
        prepared: PreparedResidentSessionMutation,
    ) -> Result<(), ResidentSessionWorkingSetError> {
        let result = match self {
            Self::Ready(working_set) => working_set.apply(prepared),
            Self::RecoveryRequired(reason) => return Err(*reason),
        };
        if let Err(reason) = result {
            *self = Self::RecoveryRequired(reason);
        }
        result
    }

    /// Return the exact owned semantic image when the projection is coherent.
    pub(super) fn snapshot(
        &self,
    ) -> Result<
        BTreeMap<SessionAdmissionIdV1, SessionGenerationPinSnapshot>,
        ResidentSessionWorkingSetError,
    > {
        match self {
            Self::Ready(working_set) => working_set.snapshot(),
            Self::RecoveryRequired(reason) => Err(*reason),
        }
    }

    /// Return deterministic reverse lookup only from a coherent projection.
    pub(super) fn sessions_retaining(
        &self,
        generation: &CompiledGenerationDigest,
    ) -> Result<Vec<SessionAdmissionIdV1>, ResidentSessionWorkingSetError> {
        match self {
            Self::Ready(working_set) => working_set.sessions_retaining(generation),
            Self::RecoveryRequired(reason) => Err(*reason),
        }
    }

    /// Resolve one exact live lease without treating an ambiguous host identity as authority.
    pub(super) fn session_by_native_activity(
        &self,
        activity: &HarnessActivityIdentity,
    ) -> Result<ResidentNativeSessionLookup, ResidentSessionWorkingSetError> {
        match self {
            Self::Ready(working_set) => working_set.session_by_native_activity(activity),
            Self::RecoveryRequired(reason) => Err(*reason),
        }
    }

    /// Preserve a capacity refusal but close every integrity failure.
    fn close_on_integrity_failure<T>(
        &mut self,
        result: Result<T, ResidentSessionWorkingSetError>,
    ) -> Result<T, ResidentSessionWorkingSetError> {
        if let Err(reason) = result {
            if reason.requires_recovery() {
                *self = Self::RecoveryRequired(reason);
            }
        }
        result
    }
}

impl ResidentSessionWorkingSet {
    /// Rebuild an exact fresh projection from the semantic lifecycle oracle.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentSessionWorkingSetError`] when the explicit bound cannot
    /// hold the recovered sessions, capacity cannot be reserved, or the semantic
    /// input contradicts the unique session/reverse-index relationship.
    pub(super) fn rebuild(
        lifecycle: &GenerationLifecycle,
        capacity: usize,
    ) -> Result<Self, ResidentSessionWorkingSetError> {
        if lifecycle.session_pins().len() > capacity {
            return Err(ResidentSessionWorkingSetError::CapacityExceeded);
        }
        let bodies = RuntimeArena::new(capacity)?;
        let mut by_session = Vec::new();
        by_session
            .try_reserve_exact(capacity)
            .map_err(|_| ResidentSessionWorkingSetError::CapacityUnavailable)?;
        let mut by_generation = Vec::new();
        by_generation
            .try_reserve_exact(capacity)
            .map_err(|_| ResidentSessionWorkingSetError::CapacityUnavailable)?;
        let mut working = Self {
            capacity,
            bodies,
            by_session,
            by_generation,
        };
        for pin in lifecycle.session_pins() {
            let prepared = working.prepare_pin(pin.clone())?;
            working.apply(prepared)?;
        }
        Ok(working)
    }

    /// Prepare insertion of one session already admitted by [`GenerationLifecycle`].
    ///
    /// This method does not validate a session lease or decide whether the pin
    /// is semantically admissible. Duplicate identity or an inconsistent reverse
    /// index is projection corruption, not an alternative lifecycle verdict.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentSessionWorkingSetError`] without mutation when the
    /// configured bound is full or the supplied pin contradicts current
    /// projection state.
    pub(super) fn prepare_pin(
        &self,
        pin: SessionGenerationPin,
    ) -> Result<PreparedResidentSessionMutation, ResidentSessionWorkingSetError> {
        if self.by_session.len() == self.capacity {
            return Err(ResidentSessionWorkingSetError::CapacityExceeded);
        }
        let session_position = self
            .by_session
            .binary_search_by(|entry| entry.session_admission_id.cmp(pin.session_admission_id()))
            .map_or_else(Ok, |_| Err(ResidentSessionWorkingSetError::Contradiction))?;
        let generation_position = self
            .by_generation
            .binary_search_by(|entry| {
                entry
                    .generation
                    .cmp(pin.generation())
                    .then_with(|| entry.session_admission_id.cmp(pin.session_admission_id()))
            })
            .map_or_else(Ok, |_| Err(ResidentSessionWorkingSetError::Contradiction))?;

        let session_admission_id = pin.session_admission_id().clone();
        let generation_session_admission_id = session_admission_id.clone();
        let generation = pin.generation().clone();
        let lease = Box::new(pin.lease().clone());
        Ok(PreparedResidentSessionMutation::Insert {
            lease,
            session_position,
            generation_position,
            session_admission_id,
            generation_session_admission_id,
            generation,
        })
    }

    /// Prepare removal of one exact admitted pin from every mechanical access path.
    ///
    /// The session and generation entries and the arena removal fence are fully
    /// checked before mutation. Applying the edit first unlinks both indexes,
    /// then advances the arena slot generation before the vacancy can be reused.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentSessionWorkingSetError::Contradiction`] when the identity,
    /// lease, body, or reverse index differs from the semantic transition.
    pub(super) fn prepare_drain(
        &self,
        session_admission_id: &SessionAdmissionIdV1,
        lease_digest: &SessionAdmissionLeaseDigest,
    ) -> Result<PreparedResidentSessionMutation, ResidentSessionWorkingSetError> {
        let session_position = self
            .by_session
            .binary_search_by(|entry| entry.session_admission_id.cmp(session_admission_id))
            .map_err(|_| ResidentSessionWorkingSetError::Contradiction)?;
        let handle = self.by_session[session_position].handle;
        let body = self.bodies.get(handle)?;
        if body.session_admission_id() != session_admission_id
            || body.session_admission_lease_digest() != lease_digest
        {
            return Err(ResidentSessionWorkingSetError::Contradiction);
        }
        let generation_position = self
            .by_generation
            .binary_search_by(|entry| {
                entry
                    .generation
                    .cmp(body.compiled_generation_digest())
                    .then_with(|| entry.session_admission_id.cmp(session_admission_id))
            })
            .map_err(|_| ResidentSessionWorkingSetError::Contradiction)?;
        if self.by_generation[generation_position].handle != handle {
            return Err(ResidentSessionWorkingSetError::Contradiction);
        }
        self.bodies.validate_removal(handle)?;

        Ok(PreparedResidentSessionMutation::Remove {
            handle,
            session_position,
            generation_position,
        })
    }

    /// Apply one prepared edit after its semantic transition is durably appended.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentSessionWorkingSetError`] only when the private projection
    /// changed after preparation or already contradicted its own invariants.
    pub(super) fn apply(
        &mut self,
        prepared: PreparedResidentSessionMutation,
    ) -> Result<(), ResidentSessionWorkingSetError> {
        match prepared {
            PreparedResidentSessionMutation::Insert {
                lease,
                session_position,
                generation_position,
                session_admission_id,
                generation_session_admission_id,
                generation,
            } => {
                if lease.session_admission_id() != &session_admission_id
                    || lease.session_admission_id() != &generation_session_admission_id
                    || lease.compiled_generation_digest() != &generation
                    || self.by_session.len() == self.capacity
                    || self.by_session.binary_search_by(|entry| {
                        entry.session_admission_id.cmp(&session_admission_id)
                    }) != Err(session_position)
                    || self.by_generation.binary_search_by(|entry| {
                        entry.generation.cmp(&generation).then_with(|| {
                            entry
                                .session_admission_id
                                .cmp(&generation_session_admission_id)
                        })
                    }) != Err(generation_position)
                {
                    return Err(ResidentSessionWorkingSetError::Contradiction);
                }
                let handle = self.bodies.insert(*lease)?;
                self.by_session.insert(
                    session_position,
                    SessionIndexEntry {
                        session_admission_id,
                        handle,
                    },
                );
                self.by_generation.insert(
                    generation_position,
                    GenerationIndexEntry {
                        generation,
                        session_admission_id: generation_session_admission_id,
                        handle,
                    },
                );
            }
            PreparedResidentSessionMutation::Remove {
                handle,
                session_position,
                generation_position,
            } => {
                if self
                    .by_session
                    .get(session_position)
                    .is_none_or(|entry| entry.handle != handle)
                    || self
                        .by_generation
                        .get(generation_position)
                        .is_none_or(|entry| entry.handle != handle)
                {
                    return Err(ResidentSessionWorkingSetError::Contradiction);
                }
                self.bodies.validate_removal(handle)?;
                self.by_generation.remove(generation_position);
                self.by_session.remove(session_position);
                self.bodies.remove(handle)?;
            }
        }
        Ok(())
    }

    /// Return the deterministic owned semantic image of all projected pins.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentSessionWorkingSetError::Contradiction`] if an internal index
    /// no longer resolves its exact body.
    pub(super) fn snapshot(
        &self,
    ) -> Result<
        BTreeMap<SessionAdmissionIdV1, SessionGenerationPinSnapshot>,
        ResidentSessionWorkingSetError,
    > {
        self.by_session
            .iter()
            .map(|entry| {
                let lease = self.bodies.get(entry.handle)?;
                if lease.session_admission_id() != &entry.session_admission_id {
                    return Err(ResidentSessionWorkingSetError::Contradiction);
                }
                Ok((
                    entry.session_admission_id.clone(),
                    SessionGenerationPinSnapshot {
                        lease_digest: lease.session_admission_lease_digest().clone(),
                        generation: lease.compiled_generation_digest().clone(),
                    },
                ))
            })
            .collect()
    }

    /// Return the deterministic owned session identities retaining `generation`.
    ///
    /// # Errors
    ///
    /// Returns [`ResidentSessionWorkingSetError::Contradiction`] when a reverse-index
    /// entry no longer resolves the exact body it names.
    pub(super) fn sessions_retaining(
        &self,
        generation: &CompiledGenerationDigest,
    ) -> Result<Vec<SessionAdmissionIdV1>, ResidentSessionWorkingSetError> {
        let start = self
            .by_generation
            .partition_point(|entry| &entry.generation < generation);
        let end = self
            .by_generation
            .partition_point(|entry| &entry.generation <= generation);
        self.by_generation[start..end]
            .iter()
            .map(|entry| {
                let lease = self.bodies.get(entry.handle)?;
                if lease.compiled_generation_digest() != generation
                    || lease.session_admission_id() != &entry.session_admission_id
                {
                    return Err(ResidentSessionWorkingSetError::Contradiction);
                }
                Ok(entry.session_admission_id.clone())
            })
            .collect()
    }

    /// Scan the bounded live set for one exact native activity correlation.
    fn session_by_native_activity(
        &self,
        activity: &HarnessActivityIdentity,
    ) -> Result<ResidentNativeSessionLookup, ResidentSessionWorkingSetError> {
        let mut matched = None;
        for entry in &self.by_session {
            let lease = self.bodies.get(entry.handle)?;
            if lease.session_admission_id() != &entry.session_admission_id {
                return Err(ResidentSessionWorkingSetError::Contradiction);
            }
            let NativeSessionCorrelationV1::Observed {
                native_session_identity,
            } = lease.native_session_correlation()
            else {
                continue;
            };
            if native_session_identity != activity {
                continue;
            }
            if matched.is_some() {
                return Ok(ResidentNativeSessionLookup::Ambiguous);
            }
            matched = Some(lease.session_admission_lease_digest().clone());
        }
        Ok(matched.map_or(
            ResidentNativeSessionLookup::Missing,
            ResidentNativeSessionLookup::Exact,
        ))
    }
}

/// A reconstructible resident-session projection could not preserve its closed law.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ResidentSessionWorkingSetError {
    /// The explicit body bound cannot admit another resident session.
    #[error("resident-session working set is at its configured capacity")]
    CapacityExceeded,
    /// The complete bounded indexes could not be reserved before construction.
    #[error("resident-session working-set capacity cannot be reserved")]
    CapacityUnavailable,
    /// Arena identity, capacity, staleness, or reuse fencing failed.
    #[error(transparent)]
    Arena(#[from] ArenaError),
    /// Arena bodies and their two indexes no longer describe one relation.
    #[error("resident-session working-set projection contradicts its semantic oracle")]
    Contradiction,
}

impl ResidentSessionWorkingSetError {
    /// Return whether this failure invalidates the process-local projection.
    pub(super) const fn requires_recovery(self) -> bool {
        !matches!(self, Self::CapacityExceeded | Self::CapacityUnavailable)
    }
}

#[cfg(test)]
#[path = "../../tests/internal/resident/working_set.rs"]
mod tests;
