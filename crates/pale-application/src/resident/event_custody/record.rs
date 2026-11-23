//! Typed application-owned private Event custody records.

use pale_core::records::{
    ArtifactDigest, CanonicalEventIdentityV1, ControlIncarnationIdV1, ControlStoreIdV1,
    DeliveryAttemptIdV1, EventAcknowledgementDigest, EventAcknowledgementDispositionV1,
    EventReferenceDigest, EventReferenceV1, EventSubscriptionIdV1, EventTypeV1, JournalSequenceV1,
    OwnerEventSequenceV1, ResidentNodeIdV1, ResidentPrincipalIdV1, ResidentRecordRefV1,
    SourceOwnerV1, UnixTimeV1,
};
use serde::{Deserialize, Serialize};

use pale_ports::{ControlRecordCoordinate, EventCustodyTransitionKind, EventTransitionMemberKind};

use super::{require, EventCustodyProjectionError};

/// Lifecycle of one admitted local subscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SubscriptionLifecycle {
    /// The subscription may select events strictly beyond its ingress frontier.
    Active,
    /// The subscription is terminal and selects no new delivery.
    Retired {
        /// Exact retirement time.
        retired_at: UnixTimeV1,
    },
}

/// One active or retired subscription admitted by application policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdmittedEventSubscription {
    #[serde(skip)]
    pub(super) record_ref: ResidentRecordRefV1,
    pub(super) subscription_id: EventSubscriptionIdV1,
    pub(super) source_owner: SourceOwnerV1,
    pub(super) event_types: Box<[EventTypeV1]>,
    pub(super) destination_principal_id: ResidentPrincipalIdV1,
    pub(super) destination_node_id: Option<ResidentNodeIdV1>,
    pub(super) ingress_frontier: JournalSequenceV1,
    pub(super) lifecycle: SubscriptionLifecycle,
}

/// Unadmitted semantic declaration for one Event subscription.
///
/// [`AdmittedEventSubscription::admit`] validates the only non-structural
/// invariant: `event_types` must be nonempty and strictly sorted without
/// duplicates. Keeping the declaration distinct prevents a partially checked
/// subscription from entering physical custody.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventSubscriptionDeclaration {
    /// Stable subscription identity.
    pub subscription_id: EventSubscriptionIdV1,
    /// Native owner whose events may enter the subscription.
    pub source_owner: SourceOwnerV1,
    /// Exact accepted event-type vocabulary in canonical order.
    pub event_types: Vec<EventTypeV1>,
    /// Resident principal receiving deliveries.
    pub destination_principal_id: ResidentPrincipalIdV1,
    /// Optional resident node narrowing the destination.
    pub destination_node_id: Option<ResidentNodeIdV1>,
    /// Fixed Pale journal frontier at admission.
    pub ingress_frontier: JournalSequenceV1,
}

impl AdmittedEventSubscription {
    /// Admit one subscription declaration and lifecycle state.
    ///
    /// # Errors
    ///
    /// Returns [`EventCustodyProjectionError::Lineage`] when event types are
    /// empty, repeated, or not strictly sorted.
    pub fn admit(
        record_ref: ResidentRecordRefV1,
        declaration: EventSubscriptionDeclaration,
        lifecycle: SubscriptionLifecycle,
    ) -> Result<Self, EventCustodyProjectionError> {
        require(
            !declaration.event_types.is_empty()
                && declaration
                    .event_types
                    .windows(2)
                    .all(|pair| pair[0] < pair[1]),
        )?;
        Ok(Self {
            record_ref,
            subscription_id: declaration.subscription_id,
            source_owner: declaration.source_owner,
            event_types: declaration.event_types.into_boxed_slice(),
            destination_principal_id: declaration.destination_principal_id,
            destination_node_id: declaration.destination_node_id,
            ingress_frontier: declaration.ingress_frontier,
            lifecycle,
        })
    }

    /// Borrow the immutable state-record reference.
    #[must_use]
    pub const fn record_ref(&self) -> &ResidentRecordRefV1 {
        &self.record_ref
    }

    /// Borrow the stable subscription identity.
    #[must_use]
    pub const fn subscription_id(&self) -> &EventSubscriptionIdV1 {
        &self.subscription_id
    }

    /// Return the fixed Pale journal frontier present at admission.
    #[must_use]
    pub const fn ingress_frontier(&self) -> JournalSequenceV1 {
        self.ingress_frontier
    }

    /// Borrow the principal authorized to receive this subscription.
    #[must_use]
    pub const fn destination_principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.destination_principal_id
    }

    /// Borrow the optional destination-node restriction.
    #[must_use]
    pub const fn destination_node_id(&self) -> Option<&ResidentNodeIdV1> {
        self.destination_node_id.as_ref()
    }

    pub(super) fn selects(
        &self,
        event: &EventReferenceV1,
        committed_at: JournalSequenceV1,
    ) -> bool {
        matches!(self.lifecycle, SubscriptionLifecycle::Active)
            && committed_at > self.ingress_frontier
            && &self.source_owner == event.source_owner()
            && self.event_types.binary_search(event.event_type()).is_ok()
    }
}

/// One deterministic fanout target retaining the admitted subscription.
#[derive(Clone, Copy)]
pub struct SubscriptionDeliveryTarget<'a> {
    subscription: &'a AdmittedEventSubscription,
}

impl<'a> SubscriptionDeliveryTarget<'a> {
    pub(super) const fn new(subscription: &'a AdmittedEventSubscription) -> Self {
        Self { subscription }
    }

    /// Borrow the selected subscription.
    #[must_use]
    pub const fn subscription(&self) -> &'a AdmittedEventSubscription {
        self.subscription
    }

    /// Borrow the stable subscription identity used for deterministic order.
    #[must_use]
    pub const fn subscription_id(&self) -> &EventSubscriptionIdV1 {
        self.subscription.subscription_id()
    }
}

/// Positive ordinal in one subscription's delivery stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct EventDeliveryOrdinal(u64);

impl EventDeliveryOrdinal {
    /// Construct a positive exact-JCS integer ordinal.
    ///
    /// # Errors
    ///
    /// Returns [`EventCustodyProjectionError::Lineage`] for zero or a value
    /// outside the inclusive [`pale_core::canon::JSON_SAFE_INTEGER_MAX`]
    /// ceiling.
    pub const fn new(value: u64) -> Result<Self, EventCustodyProjectionError> {
        if value == 0 || value > pale_core::canon::JSON_SAFE_INTEGER_MAX {
            Err(EventCustodyProjectionError::Lineage)
        } else {
            Ok(Self(value))
        }
    }

    /// Return the positive ordinal.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    pub(super) fn checked_successor(self) -> Option<Self> {
        self.0
            .checked_add(1)
            .and_then(|value| Self::new(value).ok())
    }
}

/// Private proof of the journal position that admitted one exact Event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct EventCommitWitness {
    #[serde(skip)]
    pub(super) record_ref: ResidentRecordRefV1,
    pub(super) store_id: ControlStoreIdV1,
    pub(super) store_incarnation_id: ControlIncarnationIdV1,
    pub(super) event_ref: ResidentRecordRefV1,
    pub(super) event_sha256: EventReferenceDigest,
    pub(super) event_identity: CanonicalEventIdentityV1,
    pub(super) committed_at: JournalSequenceV1,
}

/// Private proof that one subject record entered custody in a selected transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct EventCustodyTransitionWitness {
    #[serde(skip)]
    pub(super) record_ref: ResidentRecordRefV1,
    pub(super) store_id: ControlStoreIdV1,
    pub(super) store_incarnation_id: ControlIncarnationIdV1,
    pub(super) transition_kind: EventCustodyTransitionKind,
    pub(super) subject_kind: EventTransitionMemberKind,
    pub(super) subject: ControlRecordCoordinate,
    pub(super) committed_at: JournalSequenceV1,
}

/// Current predecessor-bound ordinal allocation for one subscription stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventDeliveryStream {
    #[serde(skip)]
    pub(super) record_ref: ResidentRecordRefV1,
    pub(super) subscription_id: EventSubscriptionIdV1,
    pub(super) through_ordinal: EventDeliveryOrdinal,
    pub(super) last_attempt_id: DeliveryAttemptIdV1,
}

impl EventDeliveryStream {
    /// Construct one immutable subscription-stream allocation head.
    #[must_use]
    pub fn new(
        record_ref: ResidentRecordRefV1,
        subscription_id: EventSubscriptionIdV1,
        through_ordinal: EventDeliveryOrdinal,
        last_attempt_id: DeliveryAttemptIdV1,
    ) -> Self {
        Self {
            record_ref,
            subscription_id,
            through_ordinal,
            last_attempt_id,
        }
    }
}

/// Whether one immutable delivery attempt is initial or explicit replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum DeliveryAttemptKind {
    /// First delivery for one subscription/Event pair.
    Initial,
    /// Explicit replay with a distinct attempt identity and ordinal.
    Replay,
}

/// One immutable delivery attempt, distinct from canonical Event identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventDeliveryAttempt {
    #[serde(skip)]
    pub(super) record_ref: ResidentRecordRefV1,
    pub(super) attempt_id: DeliveryAttemptIdV1,
    pub(super) subscription_id: EventSubscriptionIdV1,
    pub(super) event_identity: CanonicalEventIdentityV1,
    pub(super) event_sha256: EventReferenceDigest,
    pub(super) ordinal: EventDeliveryOrdinal,
    pub(super) kind: DeliveryAttemptKind,
}

impl EventDeliveryAttempt {
    /// Construct the first attempt for one subscription/Event pair.
    #[must_use]
    pub fn initial(
        record_ref: ResidentRecordRefV1,
        attempt_id: DeliveryAttemptIdV1,
        subscription_id: EventSubscriptionIdV1,
        event_identity: CanonicalEventIdentityV1,
        event_sha256: EventReferenceDigest,
        ordinal: EventDeliveryOrdinal,
    ) -> Self {
        Self {
            record_ref,
            attempt_id,
            subscription_id,
            event_identity,
            event_sha256,
            ordinal,
            kind: DeliveryAttemptKind::Initial,
        }
    }

    /// Construct a distinct explicit replay attempt.
    #[must_use]
    pub fn replay(
        record_ref: ResidentRecordRefV1,
        attempt_id: DeliveryAttemptIdV1,
        subscription_id: EventSubscriptionIdV1,
        event_identity: CanonicalEventIdentityV1,
        event_sha256: EventReferenceDigest,
        ordinal: EventDeliveryOrdinal,
    ) -> Self {
        Self {
            record_ref,
            attempt_id,
            subscription_id,
            event_identity,
            event_sha256,
            ordinal,
            kind: DeliveryAttemptKind::Replay,
        }
    }

    /// Borrow the attempt identity.
    #[must_use]
    pub const fn attempt_id(&self) -> &DeliveryAttemptIdV1 {
        &self.attempt_id
    }
}

/// Current delivery disposition; every acknowledgement is delivery-only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum EventDeliveryDisposition {
    /// The attempt awaits a signed acknowledgement.
    Pending,
    /// The destination signed one terminal delivery disposition.
    Acknowledged {
        /// Immutable acknowledgement reference.
        acknowledgement_ref: ResidentRecordRefV1,
        /// Canonical acknowledgement digest.
        acknowledgement_sha256: EventAcknowledgementDigest,
        /// Exact delivery-only disposition.
        disposition: EventAcknowledgementDispositionV1,
    },
}

/// Current immutable inbox state for one subscription/Event pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventDeliveryState {
    #[serde(skip)]
    pub(super) record_ref: ResidentRecordRefV1,
    pub(super) subscription_id: EventSubscriptionIdV1,
    pub(super) event_identity: CanonicalEventIdentityV1,
    pub(super) event_sha256: EventReferenceDigest,
    pub(super) attempt_id: DeliveryAttemptIdV1,
    pub(super) ordinal: EventDeliveryOrdinal,
    pub(super) disposition: EventDeliveryDisposition,
}

impl EventDeliveryState {
    /// Construct one immutable inbox state.
    #[must_use]
    pub fn new(
        record_ref: ResidentRecordRefV1,
        subscription_id: EventSubscriptionIdV1,
        event_identity: CanonicalEventIdentityV1,
        event_sha256: EventReferenceDigest,
        attempt_id: DeliveryAttemptIdV1,
        ordinal: EventDeliveryOrdinal,
        disposition: EventDeliveryDisposition,
    ) -> Self {
        Self {
            record_ref,
            subscription_id,
            event_identity,
            event_sha256,
            attempt_id,
            ordinal,
            disposition,
        }
    }
}

/// Greatest contiguous terminal delivery position for one subscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventDeliveryCursor {
    #[serde(skip)]
    pub(super) record_ref: ResidentRecordRefV1,
    pub(super) subscription_id: EventSubscriptionIdV1,
    pub(super) through_ordinal: EventDeliveryOrdinal,
    pub(super) acknowledgement_ref: ResidentRecordRefV1,
    pub(super) acknowledgement_sha256: EventAcknowledgementDigest,
}

impl EventDeliveryCursor {
    /// Construct one cursor snapshot.
    #[must_use]
    pub fn new(
        record_ref: ResidentRecordRefV1,
        subscription_id: EventSubscriptionIdV1,
        through_ordinal: EventDeliveryOrdinal,
        acknowledgement_ref: ResidentRecordRefV1,
        acknowledgement_sha256: EventAcknowledgementDigest,
    ) -> Self {
        Self {
            record_ref,
            subscription_id,
            through_ordinal,
            acknowledgement_ref,
            acknowledgement_sha256,
        }
    }

    /// Return the greatest contiguous terminal ordinal.
    #[must_use]
    pub const fn through_ordinal(&self) -> EventDeliveryOrdinal {
        self.through_ordinal
    }
}

/// Independent owner-publication frontier for one admitted source subscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventSourceFrontier {
    #[serde(skip)]
    pub(super) record_ref: ResidentRecordRefV1,
    pub(super) subscription_id: EventSubscriptionIdV1,
    pub(super) source_owner: SourceOwnerV1,
    pub(super) event_identity: CanonicalEventIdentityV1,
    pub(super) event_sha256: EventReferenceDigest,
    pub(super) owner_sequence: Option<OwnerEventSequenceV1>,
}

impl EventSourceFrontier {
    /// Construct one immutable source-frontier snapshot.
    #[must_use]
    pub fn new(
        record_ref: ResidentRecordRefV1,
        subscription_id: EventSubscriptionIdV1,
        source_owner: SourceOwnerV1,
        event_identity: CanonicalEventIdentityV1,
        event_sha256: EventReferenceDigest,
        owner_sequence: Option<OwnerEventSequenceV1>,
    ) -> Self {
        Self {
            record_ref,
            subscription_id,
            source_owner,
            event_identity,
            event_sha256,
            owner_sequence,
        }
    }
}

/// Retention state of one local outbox record.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum RetentionDisposition {
    /// The payload is durably retained but not yet acknowledged downstream.
    Retained,
    /// Exact downstream acknowledgement closed the retained payload.
    Settled {
        /// Immutable owner-native settlement reference.
        settlement_ref: ResidentRecordRefV1,
        /// Digest of the exact settlement evidence.
        settlement_sha256: ArtifactDigest,
    },
}

/// Application-owned retention for one Pale-derived publication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PaleDerivedPublicationRetention {
    #[serde(skip)]
    pub(super) record_ref: ResidentRecordRefV1,
    pub(super) publication_ref: ResidentRecordRefV1,
    pub(super) publication_sha256: ArtifactDigest,
    pub(super) disposition: RetentionDisposition,
}

impl PaleDerivedPublicationRetention {
    /// Retain one Pale-derived publication before exposure.
    #[must_use]
    pub fn retained(
        record_ref: ResidentRecordRefV1,
        publication_ref: ResidentRecordRefV1,
        publication_sha256: ArtifactDigest,
    ) -> Self {
        Self {
            record_ref,
            publication_ref,
            publication_sha256,
            disposition: RetentionDisposition::Retained,
        }
    }

    /// Construct the terminal successor after exact downstream acknowledgement.
    #[must_use]
    pub fn settled(
        record_ref: ResidentRecordRefV1,
        publication_ref: ResidentRecordRefV1,
        publication_sha256: ArtifactDigest,
        settlement_ref: ResidentRecordRefV1,
        settlement_sha256: ArtifactDigest,
    ) -> Self {
        Self {
            record_ref,
            publication_ref,
            publication_sha256,
            disposition: RetentionDisposition::Settled {
                settlement_ref,
                settlement_sha256,
            },
        }
    }
}
