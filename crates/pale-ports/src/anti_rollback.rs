//! Neutral custody boundary for independently retained anti-rollback anchors.
//!
//! The application supplies an already-admitted signed checkpoint frontier.
//! This module proves the constant-size monotone transition and classifies an
//! observed external value relative to it. A concrete adapter owns persistence
//! outside the restorable control-store failure domain; this port neither
//! selects that mechanism nor claims that an anchor advance made a database
//! transaction, owner call, or other effect safe.

use std::cmp::Ordering;

use pale_core::records::{
    ControlCheckpointDomainIdV1, ControlDomainEpochV1, ControlDomainSequenceV1,
    ControlIncarnationIdV1, ControlStoreIdV1, PrincipalTrustCheckpointEpochV1,
    ResidentPrincipalIdV1,
};

use crate::ControlRecordCoordinate;

/// Independently retained minimum accepted principal-trust checkpoint.
///
/// This is deliberately distinct from the mutable database head represented by
/// [`crate::PrincipalTrustCheckpointHead`]. Equality says only that the
/// independently retained minimum and one admitted checkpoint coordinate agree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalTrustMinimumAnchor {
    principal_id: ResidentPrincipalIdV1,
    checkpoint_epoch: PrincipalTrustCheckpointEpochV1,
    coordinate: ControlRecordCoordinate,
}

impl PrincipalTrustMinimumAnchor {
    /// Bind one principal minimum to an admitted signed-checkpoint coordinate.
    #[must_use]
    pub const fn new(
        principal_id: ResidentPrincipalIdV1,
        checkpoint_epoch: PrincipalTrustCheckpointEpochV1,
        coordinate: ControlRecordCoordinate,
    ) -> Self {
        Self {
            principal_id,
            checkpoint_epoch,
            coordinate,
        }
    }

    /// Borrow the principal whose rollback minimum is retained.
    #[must_use]
    pub const fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.principal_id
    }

    /// Return the minimum accepted principal-trust epoch.
    #[must_use]
    pub const fn checkpoint_epoch(&self) -> PrincipalTrustCheckpointEpochV1 {
        self.checkpoint_epoch
    }

    /// Borrow the exact signed-checkpoint reference and canonical digest.
    #[must_use]
    pub const fn coordinate(&self) -> &ControlRecordCoordinate {
        &self.coordinate
    }

    /// Test whether this minimum advances `predecessor` exactly once.
    #[must_use]
    pub fn is_exact_successor_of(&self, predecessor: &Self) -> bool {
        self.principal_id == predecessor.principal_id
            && predecessor.checkpoint_epoch.checked_successor() == Some(self.checkpoint_epoch)
            && self.coordinate != predecessor.coordinate
    }
}

/// Independently retained checkpoint of one control-store conflict domain.
///
/// Store and incarnation are part of the typed value so restored bytes cannot
/// be compared as though they belonged to the live store. The domain epoch is
/// stable within one successor chain; the sequence advances exactly once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlDomainAntiRollbackAnchor {
    control_store_id: ControlStoreIdV1,
    control_incarnation_id: ControlIncarnationIdV1,
    checkpoint_domain_id: ControlCheckpointDomainIdV1,
    domain_epoch: ControlDomainEpochV1,
    domain_sequence: ControlDomainSequenceV1,
    coordinate: ControlRecordCoordinate,
}

impl ControlDomainAntiRollbackAnchor {
    /// Bind one external domain anchor to an admitted signed checkpoint.
    #[must_use]
    pub const fn new(
        control_store_id: ControlStoreIdV1,
        control_incarnation_id: ControlIncarnationIdV1,
        checkpoint_domain_id: ControlCheckpointDomainIdV1,
        domain_epoch: ControlDomainEpochV1,
        domain_sequence: ControlDomainSequenceV1,
        coordinate: ControlRecordCoordinate,
    ) -> Self {
        Self {
            control_store_id,
            control_incarnation_id,
            checkpoint_domain_id,
            domain_epoch,
            domain_sequence,
            coordinate,
        }
    }

    /// Borrow the stable logical control-store identity.
    #[must_use]
    pub const fn control_store_id(&self) -> &ControlStoreIdV1 {
        &self.control_store_id
    }

    /// Borrow the exact control-store incarnation.
    #[must_use]
    pub const fn control_incarnation_id(&self) -> &ControlIncarnationIdV1 {
        &self.control_incarnation_id
    }

    /// Borrow the independently progressing conflict-domain identity.
    #[must_use]
    pub const fn checkpoint_domain_id(&self) -> &ControlCheckpointDomainIdV1 {
        &self.checkpoint_domain_id
    }

    /// Return the epoch preserved by this domain's successor chain.
    #[must_use]
    pub const fn domain_epoch(&self) -> ControlDomainEpochV1 {
        self.domain_epoch
    }

    /// Return the monotone sequence within [`Self::domain_epoch`].
    #[must_use]
    pub const fn domain_sequence(&self) -> ControlDomainSequenceV1 {
        self.domain_sequence
    }

    /// Borrow the exact checkpoint reference and canonical digest.
    #[must_use]
    pub const fn coordinate(&self) -> &ControlRecordCoordinate {
        &self.coordinate
    }

    /// Test whether this domain anchor advances `predecessor` exactly once.
    #[must_use]
    pub fn is_exact_successor_of(&self, predecessor: &Self) -> bool {
        self.control_store_id == predecessor.control_store_id
            && self.control_incarnation_id == predecessor.control_incarnation_id
            && self.checkpoint_domain_id == predecessor.checkpoint_domain_id
            && self.domain_epoch == predecessor.domain_epoch
            && predecessor.domain_sequence.checked_successor() == Some(self.domain_sequence)
            && self.coordinate != predecessor.coordinate
    }
}

/// Deterministic physical key for one independently retained anchor.
///
/// Derived ordering places principal minima before control-domain anchors, then
/// orders each variant lexicographically by its typed identity fields. Epoch,
/// sequence, record reference, and digest are values rather than key material.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AntiRollbackAnchorKey {
    /// One principal-wide trust minimum.
    PrincipalTrustMinimum(ResidentPrincipalIdV1),
    /// One independently advancing control-store conflict domain.
    ControlDomain {
        /// Stable logical control-store identity.
        control_store_id: ControlStoreIdV1,
        /// Exact control-store incarnation.
        control_incarnation_id: ControlIncarnationIdV1,
        /// Independently advancing conflict-domain identity.
        checkpoint_domain_id: ControlCheckpointDomainIdV1,
    },
}

/// Closed value family retained through [`AntiRollbackAnchorPort`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AntiRollbackAnchor {
    /// Principal-wide minimum accepted trust checkpoint.
    PrincipalTrustMinimum(PrincipalTrustMinimumAnchor),
    /// Per-store, per-incarnation control-domain checkpoint.
    ControlDomain(ControlDomainAntiRollbackAnchor),
}

impl AntiRollbackAnchor {
    /// Project the deterministic physical key without flattening anchor kinds.
    #[must_use]
    pub fn key(&self) -> AntiRollbackAnchorKey {
        match self {
            Self::PrincipalTrustMinimum(anchor) => {
                AntiRollbackAnchorKey::PrincipalTrustMinimum(anchor.principal_id.clone())
            }
            Self::ControlDomain(anchor) => AntiRollbackAnchorKey::ControlDomain {
                control_store_id: anchor.control_store_id.clone(),
                control_incarnation_id: anchor.control_incarnation_id.clone(),
                checkpoint_domain_id: anchor.checkpoint_domain_id.clone(),
            },
        }
    }
}

impl From<PrincipalTrustMinimumAnchor> for AntiRollbackAnchor {
    fn from(value: PrincipalTrustMinimumAnchor) -> Self {
        Self::PrincipalTrustMinimum(value)
    }
}

impl From<ControlDomainAntiRollbackAnchor> for AntiRollbackAnchor {
    fn from(value: ControlDomainAntiRollbackAnchor) -> Self {
        Self::ControlDomain(value)
    }
}

/// Validated initial or exact-successor anchor transition.
///
/// The admitted transition relation is
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/anti-rollback-anchor-transition.display.mathml"
))]
///
/// where `e` is the optional existing anchor, `n` the proposed next anchor, and
/// `position` the principal checkpoint epoch or control-domain sequence.
/// Control successors additionally preserve store, incarnation,
/// domain, and domain epoch. This type proves only transition shape; checkpoint
/// signatures, database durability, and external persistence remain with their
/// respective owners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedAntiRollbackAnchorTransition {
    key: AntiRollbackAnchorKey,
    expected: Option<AntiRollbackAnchor>,
    next: AntiRollbackAnchor,
}

impl PreparedAntiRollbackAnchorTransition {
    /// Validate the first anchor for one previously absent key.
    ///
    /// Principal minima begin at checkpoint epoch `1`; control-domain anchors
    /// begin at sequence `1` within their already-admitted domain epoch.
    ///
    /// # Errors
    ///
    /// Returns [`AntiRollbackAnchorTransitionError::InitialPosition`] when
    /// `next` does not name the required initial coordinate.
    pub fn initial(next: AntiRollbackAnchor) -> Result<Self, AntiRollbackAnchorTransitionError> {
        let initial = match &next {
            AntiRollbackAnchor::PrincipalTrustMinimum(anchor) => anchor.checkpoint_epoch.get() == 1,
            AntiRollbackAnchor::ControlDomain(anchor) => anchor.domain_sequence.get() == 1,
        };
        if !initial {
            return Err(AntiRollbackAnchorTransitionError::InitialPosition);
        }
        Ok(Self {
            key: next.key(),
            expected: None,
            next,
        })
    }

    /// Validate one exact monotone successor for an existing key.
    ///
    /// # Errors
    ///
    /// Returns [`AntiRollbackAnchorTransitionError::AnchorKind`] when the two
    /// values have different anchor kinds,
    /// [`AntiRollbackAnchorTransitionError::Key`] when their typed keys differ,
    /// or [`AntiRollbackAnchorTransitionError::Successor`] unless `next`
    /// advances the applicable coordinate exactly once while preserving the
    /// lineage fields required by that anchor kind.
    pub fn successor(
        expected: AntiRollbackAnchor,
        next: AntiRollbackAnchor,
    ) -> Result<Self, AntiRollbackAnchorTransitionError> {
        let same_kind = matches!(
            (&expected, &next),
            (
                AntiRollbackAnchor::PrincipalTrustMinimum(_),
                AntiRollbackAnchor::PrincipalTrustMinimum(_)
            ) | (
                AntiRollbackAnchor::ControlDomain(_),
                AntiRollbackAnchor::ControlDomain(_)
            )
        );
        if !same_kind {
            return Err(AntiRollbackAnchorTransitionError::AnchorKind);
        }
        let key = expected.key();
        if next.key() != key {
            return Err(AntiRollbackAnchorTransitionError::Key);
        }
        let valid = match (&expected, &next) {
            (
                AntiRollbackAnchor::PrincipalTrustMinimum(expected),
                AntiRollbackAnchor::PrincipalTrustMinimum(next),
            ) => next.is_exact_successor_of(expected),
            (
                AntiRollbackAnchor::ControlDomain(expected),
                AntiRollbackAnchor::ControlDomain(next),
            ) => next.is_exact_successor_of(expected),
            _ => false,
        };
        if !valid {
            return Err(AntiRollbackAnchorTransitionError::Successor);
        }
        Ok(Self {
            key,
            expected: Some(expected),
            next,
        })
    }

    /// Borrow the deterministic physical key compared by the adapter.
    #[must_use]
    pub const fn key(&self) -> &AntiRollbackAnchorKey {
        &self.key
    }

    /// Borrow the exact predecessor, absent only for an initial transition.
    #[must_use]
    pub const fn expected(&self) -> Option<&AntiRollbackAnchor> {
        self.expected.as_ref()
    }

    /// Borrow the intended monotone successor.
    #[must_use]
    pub const fn next(&self) -> &AntiRollbackAnchor {
        &self.next
    }

    /// Classify one external read relative to this prepared transition.
    ///
    /// This method is the semantic SSOT for adapter observations. A same-position
    /// value with different checkpoint reference or digest is
    /// [`AntiRollbackAnchorObservation::Conflicting`], never a replay. Control
    /// values with another domain epoch are also conflicts because this port has
    /// no authority to infer a cross-epoch transition.
    #[must_use]
    pub fn classify_observed(
        &self,
        observed: Option<&AntiRollbackAnchor>,
    ) -> AntiRollbackAnchorObservation {
        let Some(observed) = observed else {
            return AntiRollbackAnchorObservation::Missing;
        };
        if observed == &self.next {
            return AntiRollbackAnchorObservation::ExactSuccessor;
        }
        if self
            .expected
            .as_ref()
            .is_some_and(|value| value == observed)
        {
            return AntiRollbackAnchorObservation::ExactPredecessor;
        }
        if observed.key() != self.key {
            return AntiRollbackAnchorObservation::Conflicting {
                observed: observed.clone(),
            };
        }

        let lower_bound = self.expected.as_ref().unwrap_or(&self.next);
        match compare_position(observed, lower_bound) {
            Some(Ordering::Less) => AntiRollbackAnchorObservation::Behind {
                observed: observed.clone(),
            },
            Some(Ordering::Equal) | None => AntiRollbackAnchorObservation::Conflicting {
                observed: observed.clone(),
            },
            Some(Ordering::Greater) => match compare_position(observed, &self.next) {
                Some(Ordering::Greater) => AntiRollbackAnchorObservation::Ahead {
                    observed: observed.clone(),
                },
                Some(Ordering::Less | Ordering::Equal) | None => {
                    AntiRollbackAnchorObservation::Conflicting {
                        observed: observed.clone(),
                    }
                }
            },
        }
    }
}

/// Candidate construction failed before any external anchor was accessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AntiRollbackAnchorTransitionError {
    /// Principal and control-domain values cannot share one transition.
    #[error("anti-rollback transition changes anchor kind")]
    AnchorKind,
    /// The predecessor and successor identify different physical keys.
    #[error("anti-rollback transition changes its typed key")]
    Key,
    /// An initial anchor does not begin at its required position.
    #[error("anti-rollback initial transition does not begin at position one")]
    InitialPosition,
    /// A successor skips, rewinds, repeats, or changes a preserved lineage field.
    #[error("anti-rollback transition is not an exact successor")]
    Successor,
}

/// Exact external state relative to one prepared anchor transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AntiRollbackAnchorObservation {
    /// The independently retained key does not exist.
    Missing,
    /// The external value is byte-coordinate equal to the expected predecessor.
    ExactPredecessor,
    /// The external value is byte-coordinate equal to the intended successor.
    ExactSuccessor,
    /// The same lineage is older than the prepared predecessor.
    Behind {
        /// Exact older value returned by the external mechanism.
        observed: AntiRollbackAnchor,
    },
    /// The same lineage is newer than the prepared successor.
    Ahead {
        /// Exact newer value returned by the external mechanism.
        observed: AntiRollbackAnchor,
    },
    /// Key, epoch, position, reference, or digest contradicts the transition.
    Conflicting {
        /// Exact contradictory value returned by the external mechanism.
        observed: AntiRollbackAnchor,
    },
}

/// Result of one external compare-and-advance attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AntiRollbackAnchorAdvanceDisposition {
    /// The adapter atomically replaced the expected external value with `next`.
    Advanced,
    /// The exact intended successor was already retained; no write was needed.
    ExactReplay,
    /// The observed state did not authorize the prepared comparison.
    NotAdvanced(AntiRollbackAnchorObservation),
}

/// Stable failure classes for the external anchor mechanism itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntiRollbackAnchorErrorKind {
    /// The external mechanism could not be read or compared before ambiguity.
    Unavailable,
    /// An attempted advance may or may not have become externally durable.
    OutcomeUnknown,
}

/// External anchor failure preserving an adapter-native source when available.
#[derive(Debug, thiserror::Error)]
#[error("anti-rollback anchor failure ({kind:?})")]
pub struct AntiRollbackAnchorError {
    kind: AntiRollbackAnchorErrorKind,
    #[source]
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl AntiRollbackAnchorError {
    /// Construct a source-free external-mechanism classification.
    #[must_use]
    pub fn new(kind: AntiRollbackAnchorErrorKind) -> Self {
        Self { kind, source: None }
    }

    /// Retain an adapter-native source behind a stable failure class.
    pub fn with_source<E>(kind: AntiRollbackAnchorErrorKind, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            kind,
            source: Some(Box::new(source)),
        }
    }

    /// Return the stable failure class used by application recovery policy.
    #[must_use]
    pub const fn kind(&self) -> AntiRollbackAnchorErrorKind {
        self.kind
    }
}

/// Constant-size external compare-and-advance boundary for rollback anchors.
///
/// Implementations must keep different [`AntiRollbackAnchorKey`] values
/// independently progressable. An [`AntiRollbackAnchorAdvanceDisposition::Advanced`]
/// result proves only that the external mechanism reports the intended value as
/// durable under its selected contract. Qualification of the mechanism's
/// backup, clone, and failure-domain properties remains outside this trait.
pub trait AntiRollbackAnchorPort {
    /// Read one exact independently retained value without a proposed transition.
    /// This is intended for protected recovery, not ordinary public inspection.
    /// The caller must reject a returned value whose key differs from `key` and
    /// separately admit its signed checkpoint and exact database association.
    /// Reading never initializes or advances an anchor.
    ///
    /// # Errors
    /// Returns an error when the selected external mechanism cannot reliably
    /// answer. Missing state is `None`, never permission to recreate an anchor.
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError>;

    /// Compare the external key and monotonically retain the prepared successor.
    ///
    /// The adapter should return `ExactReplay` only for exact successor equality
    /// and `NotAdvanced` with the result of
    /// [`PreparedAntiRollbackAnchorTransition::classify_observed`] for every
    /// ordinary non-matching state.
    ///
    /// # Errors
    ///
    /// Returns [`AntiRollbackAnchorErrorKind::Unavailable`] when no advance was
    /// attempted or [`AntiRollbackAnchorErrorKind::OutcomeUnknown`] when a write
    /// may have committed. The caller must observe after outcome uncertainty;
    /// blind retry cannot establish the external result.
    fn compare_and_advance(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError>;

    /// Read and classify the exact external value without mutation.
    ///
    /// # Errors
    ///
    /// Returns [`AntiRollbackAnchorError`] when the external mechanism cannot be
    /// read reliably. A successful observation is not proof that the database
    /// head, signature set, or owner effect agrees with the anchor.
    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError>;
}

/// Compare the monotone positions of two values in one admitted lineage.
fn compare_position(left: &AntiRollbackAnchor, right: &AntiRollbackAnchor) -> Option<Ordering> {
    match (left, right) {
        (
            AntiRollbackAnchor::PrincipalTrustMinimum(left),
            AntiRollbackAnchor::PrincipalTrustMinimum(right),
        ) if left.principal_id == right.principal_id => {
            Some(left.checkpoint_epoch.cmp(&right.checkpoint_epoch))
        }
        (AntiRollbackAnchor::ControlDomain(left), AntiRollbackAnchor::ControlDomain(right))
            if left.control_store_id == right.control_store_id
                && left.control_incarnation_id == right.control_incarnation_id
                && left.checkpoint_domain_id == right.checkpoint_domain_id
                && left.domain_epoch == right.domain_epoch =>
        {
            Some(left.domain_sequence.cmp(&right.domain_sequence))
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "../tests/internal/anti_rollback.rs"]
mod tests;
