//! Composite semantic state folded by the resident authority journal.
//!
//! Generation lifecycle, desired process state, and the last Event custody
//! basis share one journal. A sparse delta changes one component and retains
//! the others in the successor fingerprint. The law and its symbols are owned
//! by [`crate::resident::ResidentApplication`]. Custody describes retained
//! records; it does not grant fresh publication permission.
//!
//! This module owns no storage, process, sentinel, or notification effect. It
//! only derives and reapplies typed transitions for the journal coordinator.

use pale_core::records::{ArtifactDigest, JournalSequenceV1};
use serde::{Deserialize, Serialize};

use super::event_custody::journal::{EventCustodyBasis, EventCustodyFrontier};
use super::lifecycle::{
    GenerationLifecycle, GenerationLifecycleDelta, GenerationLifecycleError,
    GenerationLifecycleRecoveryResolver, GenerationLifecycleRecoveryStateV1,
    GenerationLifecycleSnapshot, GenerationLifecycleTransition,
};
use super::process_reconciliation::DesiredProcessState;
use super::session_lease_custody::SessionLeaseRecoveryContext;

/// Domain separator for the complete private resident-authority fingerprint.
const RESIDENT_AUTHORITY_STATE_DIGEST_DOMAIN_V1: &str = "pale-resident-authority-state.v1";

/// Complete handle-free semantic state at one journal frontier.
#[derive(Debug, Clone)]
pub(super) struct ResidentAuthorityState {
    lifecycle: GenerationLifecycle,
    desired_process_state: Option<DesiredProcessState>,
    event_custody: EventCustodyFrontier,
}

impl ResidentAuthorityState {
    /// Begin with admitted lifecycle semantics and no invented desired state.
    pub(super) fn new(lifecycle: GenerationLifecycle) -> Self {
        Self {
            lifecycle,
            desired_process_state: None,
            event_custody: EventCustodyFrontier::Empty,
        }
    }

    /// Rebuild the complete state from checkpoint custody.
    pub(super) fn from_recovery(
        lifecycle: GenerationLifecycle,
        desired_process_state: Option<DesiredProcessState>,
        event_custody: EventCustodyFrontier,
    ) -> Self {
        Self {
            lifecycle,
            desired_process_state,
            event_custody,
        }
    }

    /// Borrow the generation-lifecycle projection.
    pub(super) const fn lifecycle(&self) -> &GenerationLifecycle {
        &self.lifecycle
    }

    /// Return the persisted desired process state, when one was explicitly set.
    pub(super) const fn desired_process_state(&self) -> Option<DesiredProcessState> {
        self.desired_process_state
    }

    /// Project the complete semantic state used by journal fingerprints.
    pub(super) fn snapshot(&self) -> ResidentAuthoritySnapshot {
        ResidentAuthoritySnapshot {
            lifecycle: self.lifecycle.snapshot(),
            desired_process_state: self.desired_process_state,
            event_custody: self.event_custody.clone(),
        }
    }

    /// Wrap one already-derived generation transition for the shared journal.
    pub(super) fn lifecycle_transition(
        transition: GenerationLifecycleTransition,
    ) -> ResidentAuthorityTransition {
        ResidentAuthorityTransition::GenerationLifecycle(Box::new(transition))
    }

    /// Derive an idempotent desired-state transition.
    ///
    /// `None` means the requested state is already the durably installed
    /// projection and must not consume another journal coordinate.
    pub(super) fn desired_process_state_transition(
        &self,
        desired: DesiredProcessState,
    ) -> Option<ResidentAuthorityTransition> {
        (self.desired_process_state != Some(desired)).then_some(
            ResidentAuthorityTransition::DesiredProcessState {
                expected: self.desired_process_state,
                desired,
            },
        )
    }

    pub(super) fn event_custody(&self) -> &EventCustodyFrontier {
        &self.event_custody
    }

    pub(super) fn event_custody_transition(
        &self,
        basis: EventCustodyBasis,
    ) -> ResidentAuthorityTransition {
        ResidentAuthorityTransition::EventCustody {
            expected: self.event_custody.clone(),
            basis: Box::new(basis),
        }
    }
}

/// Serializable complete semantic image used only for private state hashing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct ResidentAuthoritySnapshot {
    lifecycle: GenerationLifecycleSnapshot,
    desired_process_state: Option<DesiredProcessState>,
    event_custody: EventCustodyFrontier,
}

/// Domain-separated fingerprint of one complete [`ResidentAuthoritySnapshot`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct ResidentAuthorityStateDigest(ArtifactDigest);

impl ResidentAuthorityStateDigest {
    /// Hash the canonical product state without treating it as a public record.
    pub(super) fn from_state(
        state: &ResidentAuthorityState,
    ) -> Result<Self, pale_core::canon::CanonError> {
        let canonical = pale_core::canon::to_canonical_string(&(
            RESIDENT_AUTHORITY_STATE_DIGEST_DOMAIN_V1,
            state.snapshot(),
        ))?;
        Ok(Self(ArtifactDigest::hash_bytes(canonical.as_bytes())))
    }
}

/// One pure sparse transition awaiting journal sequence binding.
pub(super) enum ResidentAuthorityTransition {
    /// Retain the latest Event transaction without copying record bodies.
    EventCustody {
        expected: EventCustodyFrontier,
        basis: Box<EventCustodyBasis>,
    },
    /// Change only generation-lifecycle semantics.
    GenerationLifecycle(Box<GenerationLifecycleTransition>),
    /// Change only the explicit desired process state.
    DesiredProcessState {
        /// Desired state observed when the transition was derived.
        expected: Option<DesiredProcessState>,
        /// Explicit state to install.
        desired: DesiredProcessState,
    },
}

impl ResidentAuthorityTransition {
    /// Bind one transition to the current product state and journal sequence.
    pub(super) fn into_journal_parts(
        self,
        current: &ResidentAuthorityState,
        sequence: JournalSequenceV1,
    ) -> Result<(ResidentAuthorityDelta, ResidentAuthorityState), ResidentAuthorityTransitionError>
    {
        match self {
            Self::EventCustody { expected, basis } => {
                if current.event_custody != expected
                    || !expected.accepts(&basis)
                    || basis.sequence() != sequence
                {
                    return Err(ResidentAuthorityTransitionError::PredecessorMismatch);
                }
                Ok((
                    ResidentAuthorityDelta::EventCustody {
                        basis: basis.clone(),
                    },
                    ResidentAuthorityState {
                        lifecycle: current.lifecycle.clone(),
                        desired_process_state: current.desired_process_state,
                        event_custody: EventCustodyFrontier::Retained(basis),
                    },
                ))
            }
            Self::GenerationLifecycle(transition) => {
                let (expected, delta, successor) = (*transition).into_journal_parts();
                if current.lifecycle.snapshot() != expected {
                    return Err(ResidentAuthorityTransitionError::PredecessorMismatch);
                }
                successor.validate_envelope(sequence, &expected)?;
                delta.validate_envelope(sequence)?;
                Ok((
                    ResidentAuthorityDelta::GenerationLifecycle {
                        delta: Box::new(delta),
                    },
                    ResidentAuthorityState {
                        lifecycle: successor,
                        desired_process_state: current.desired_process_state,
                        event_custody: current.event_custody.clone(),
                    },
                ))
            }
            Self::DesiredProcessState { expected, desired } => {
                if current.desired_process_state != expected || expected == Some(desired) {
                    return Err(ResidentAuthorityTransitionError::PredecessorMismatch);
                }
                Ok((
                    ResidentAuthorityDelta::DesiredProcessState { desired },
                    ResidentAuthorityState {
                        lifecycle: current.lifecycle.clone(),
                        desired_process_state: Some(desired),
                        event_custody: current.event_custody.clone(),
                    },
                ))
            }
        }
    }
}

/// Closed sparse mutation families admitted by the private journal decoder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "body", rename_all = "snake_case")]
pub(super) enum ResidentAuthorityDelta {
    /// Coordinates of the members committed with this journal entry.
    EventCustody { basis: Box<EventCustodyBasis> },
    /// Reapply one generation-lifecycle delta without changing desired state.
    GenerationLifecycle {
        /// Minimal lifecycle mutation re-admitted through its semantic owner.
        delta: Box<GenerationLifecycleDelta>,
    },
    /// Install one explicit desired process state without changing lifecycle.
    DesiredProcessState {
        /// Desired state selected by an authenticated later caller.
        desired: DesiredProcessState,
    },
}

impl ResidentAuthorityDelta {
    /// Validate any journal-sequence coordinate embedded by this sparse delta.
    pub(super) fn validate_envelope(
        &self,
        sequence: JournalSequenceV1,
    ) -> Result<(), ResidentAuthorityTransitionError> {
        match self {
            Self::EventCustody { basis } => {
                if basis.sequence() != sequence {
                    return Err(ResidentAuthorityTransitionError::PredecessorMismatch);
                }
            }
            Self::GenerationLifecycle { delta } => delta.validate_envelope(sequence)?,
            Self::DesiredProcessState { .. } => {}
        }
        Ok(())
    }

    /// Reapply one sparse delta to the complete accepted product state.
    pub(super) fn apply(
        self,
        current: &ResidentAuthorityState,
        sequence: JournalSequenceV1,
        resolver: &dyn GenerationLifecycleRecoveryResolver,
        session_leases: &SessionLeaseRecoveryContext<'_>,
    ) -> Result<ResidentAuthorityState, ResidentAuthorityTransitionError> {
        match self {
            Self::EventCustody { basis } => {
                if basis.sequence() != sequence || !current.event_custody.accepts(&basis) {
                    return Err(ResidentAuthorityTransitionError::PredecessorMismatch);
                }
                Ok(ResidentAuthorityState {
                    lifecycle: current.lifecycle.clone(),
                    desired_process_state: current.desired_process_state,
                    event_custody: EventCustodyFrontier::Retained(basis),
                })
            }
            Self::GenerationLifecycle { delta } => {
                delta.validate_envelope(sequence)?;
                Ok(ResidentAuthorityState {
                    lifecycle: (*delta).apply(&current.lifecycle, resolver, session_leases)?,
                    desired_process_state: current.desired_process_state,
                    event_custody: current.event_custody.clone(),
                })
            }
            Self::DesiredProcessState { desired } => {
                if current.desired_process_state == Some(desired) {
                    return Err(ResidentAuthorityTransitionError::PredecessorMismatch);
                }
                Ok(ResidentAuthorityState {
                    lifecycle: current.lifecycle.clone(),
                    desired_process_state: Some(desired),
                    event_custody: current.event_custody.clone(),
                })
            }
        }
    }
}

/// Private checkpoint projection re-admitted during cold recovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ResidentAuthorityRecoveryStateV1 {
    lifecycle: GenerationLifecycleRecoveryStateV1,
    desired_process_state: DesiredProcessStateRecoveryV1,
    event_custody: EventCustodyFrontier,
}

/// Explicit checkpoint state that rejects a missing desired-state projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "state", rename_all = "snake_case")]
enum DesiredProcessStateRecoveryV1 {
    /// No authenticated caller has selected a desired process state yet.
    Unset,
    /// An exact desired process state was persisted.
    Set(DesiredProcessState),
}

impl ResidentAuthorityRecoveryStateV1 {
    /// Project the complete admitted product state into a handle-free image.
    pub(super) fn from_authority(authority: &ResidentAuthorityState) -> Self {
        Self {
            lifecycle: GenerationLifecycleRecoveryStateV1::from_lifecycle(&authority.lifecycle),
            event_custody: authority.event_custody.clone(),
            desired_process_state: authority.desired_process_state.map_or(
                DesiredProcessStateRecoveryV1::Unset,
                DesiredProcessStateRecoveryV1::Set,
            ),
        }
    }

    /// Re-admit lifecycle owner context and retain the exact desired state.
    pub(super) fn admit(
        self,
        resolver: &dyn GenerationLifecycleRecoveryResolver,
        session_leases: &SessionLeaseRecoveryContext<'_>,
    ) -> Result<ResidentAuthorityState, GenerationLifecycleError> {
        Ok(ResidentAuthorityState::from_recovery(
            self.lifecycle.admit(resolver, session_leases)?,
            match self.desired_process_state {
                DesiredProcessStateRecoveryV1::Unset => None,
                DesiredProcessStateRecoveryV1::Set(desired) => Some(desired),
            },
            self.event_custody,
        ))
    }

    /// Return the greatest lifecycle or Event custody sequence in this image.
    pub(super) fn maximum_journal_sequence(&self) -> Option<JournalSequenceV1> {
        self.lifecycle
            .maximum_journal_sequence()
            .max(self.event_custody.basis().map(EventCustodyBasis::sequence))
    }
}

/// A sparse transition contradicted its current product state or lifecycle law.
#[derive(Debug, thiserror::Error)]
pub(super) enum ResidentAuthorityTransitionError {
    /// The transition was not derived from the exact currently installed state.
    #[error("resident authority transition predecessor does not match installed state")]
    PredecessorMismatch,
    /// Generation-lifecycle semantics rejected the sparse transition.
    #[error(transparent)]
    Lifecycle(#[from] GenerationLifecycleError),
}
