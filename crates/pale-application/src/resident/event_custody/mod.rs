//! Application-owned Event, delivery, Genealogy, and frontier custody.
//!
//! This module is the sole semantic projection into
//! [`pale_ports::EventCustodyPort`]. It derives domain-separated canonical
//! keys, validates cross-record lineage, and exposes affine continuation only
//! after a new transaction or a same-call exact recovery. Ordinary replay is
//! custody evidence and grants no second projection, notification, or send.
//!
//! Owner bodies remain outside Pale. The producer-retirement race is closed
//! by [`EventPublicationRaceVerifier`] under the exact active resident-writer
//! fence; neither a principal-store point read nor the deployment transaction
//! alone can establish that cross-store ordering. In the continuation law,
//! `E_commit` is the event-custody commit disposition and `K` is the selected
//! transition kind. `C_continue` exists only for a new commit or an exact
//! same-call recovery and one of the four named continuation-bearing kinds.

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/event-custody-continuation.display.mathml"
))]
use pale_core::canon;
use pale_core::records::{
    EventAcknowledgementV1, EventReferenceV1, GenealogyEdgeV1, InteractionOccurrenceV1,
    InvalidResidentRecordV1, PrincipalResidentWriterFenceV1, ProducerIncarnationRetirementV1,
    ProducerIncarnationV1, ReconciliationFrontierV1, ResidentRecordRefV1,
};
use pale_ports::{
    AuthorityStoreCoordinate, ControlRecord, ControlRecordError, EventCustodyCandidateError,
};

use super::record_projection::ControlRecordProjectionError;

mod a2a_task_projection;
mod initial_delivery;
pub(in crate::resident) mod journal;
pub(in crate::resident) mod local_operation_projection;
mod local_operation_publication;
mod local_operation_sweep;
mod phase;
pub(in crate::resident) mod projection;
mod record;
mod recovery;
mod replay;

pub use a2a_task_projection::{
    admit_a2a_task_predecessor_publication, admit_a2a_task_publication_plan,
    A2aTaskPredecessorPublication, A2aTaskPublicationProjectionError, A2aTaskPublicationReferences,
    AdmittedA2aTaskPublicationPlan,
};
pub use initial_delivery::{
    advance_initial_event_delivery, advance_initial_event_delivery_cursor,
    record_initial_event_acknowledgement, recover_initial_event_delivery,
    InitialEventAcknowledgementAdvance, InitialEventAcknowledgementAuthority,
    InitialEventAcknowledgementInput, InitialEventDeliveryAdvance,
    InitialEventDeliveryCursorAdvance, InitialEventDeliveryError, InitialEventDeliveryPlan,
    InitialEventDeliveryRecovery, InitialEventDeliveryRecoveryLimits, RecordedEventAcknowledgement,
};
pub use local_operation_projection::{
    admit_local_operation_publication_plan, AdmittedLocalOperationPublicationPlan,
    LocalOperationPredecessorPublication, LocalOperationPublicationPolicy,
    LocalOperationPublicationPolicyInput, LocalOperationPublicationProjectionError,
    LocalOperationPublicationReferences, LocalOperationPublicationSigningContexts,
};
pub use local_operation_publication::{
    advance_a2a_task_publication, advance_local_operation_publication,
    LocalOperationPublicationAdvance, LocalOperationPublicationAdvanceError,
    LocalOperationPublicationIssuance, LocalOperationPublicationReadmission,
    LocalOperationPublicationStage,
};
pub use local_operation_sweep::{
    sweep_local_operation_publications, A2aTaskPublicationRecoveryLimits,
    A2aTaskPublicationRecoveryRequest, A2aTaskPublicationSourceRecoveryError,
    LocalOperationPublicationCurrent, LocalOperationPublicationRecoveryLimits,
    LocalOperationPublicationRecoveryRequest, LocalOperationPublicationSourceRecoveryError,
    LocalOperationPublicationSweepAttempt, LocalOperationPublicationSweepError,
    LocalOperationPublicationSweepLimits, LocalOperationPublicationSweepProgress,
    OperationPublicationRevision, RecoveredA2aTaskPublicationSource,
};
pub use phase::{
    CompletedEventCustodyTransition, EventCustodyCommitEvidence, EventCustodyCommitFailure,
    EventCustodyCommitFailureReason, EventCustodyContinuation, EventCustodyContinuationKind,
    PreparedEventCustodyTransition,
};
pub use record::{
    AdmittedEventSubscription, EventDeliveryAttempt, EventDeliveryCursor, EventDeliveryDisposition,
    EventDeliveryOrdinal, EventDeliveryState, EventDeliveryStream, EventSourceFrontier,
    EventSubscriptionDeclaration, PaleDerivedPublicationRetention, SubscriptionDeliveryTarget,
    SubscriptionLifecycle,
};
pub use recovery::{
    read_pending_pale_derived_publications, EventCustodyRecoveryError,
    PendingPaleDerivedPublication, PendingPaleDerivedPublicationPage,
};
pub use replay::{
    classify_event_identity, classify_genealogy_edge, classify_interaction_occurrence,
    classify_reconciliation_frontier, DerivedEventRecordDisposition, EventIdentityDisposition,
    EventIdentityLookupError, ReconciliationFrontierDisposition,
};

/// One typed admitted record paired with its immutable resident reference.
pub struct AdmittedEventRecord<'a, R> {
    record_ref: &'a ResidentRecordRefV1,
    record: &'a R,
}

impl<R> Copy for AdmittedEventRecord<'_, R> {}

impl<R> Clone for AdmittedEventRecord<'_, R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, R> AdmittedEventRecord<'a, R> {
    /// Bind one already-admitted value to its exact immutable reference.
    #[must_use]
    pub const fn new(record_ref: &'a ResidentRecordRefV1, record: &'a R) -> Self {
        Self { record_ref, record }
    }

    /// Borrow the immutable resident reference.
    #[must_use]
    pub const fn record_ref(&self) -> &ResidentRecordRefV1 {
        self.record_ref
    }

    /// Borrow the admitted typed value.
    #[must_use]
    pub const fn record(&self) -> &R {
        self.record
    }
}

/// Producer state under which one immutable owner publication was created.
#[derive(Clone, Copy)]
pub enum EventPublicationState<'a> {
    /// The exact allocation remains the active producer head.
    Active,
    /// The row was committed before this separately appended retirement.
    CommittedBeforeRetirement {
        /// Immutable retirement reference.
        retirement_ref: &'a ResidentRecordRefV1,
        /// Admitted retirement record closing the allocation.
        retirement: &'a ProducerIncarnationRetirementV1,
    },
}

/// Exact cross-store evidence presented to the source-publication owner.
pub struct EventPublicationRaceClosure<'a> {
    event: AdmittedEventRecord<'a, EventReferenceV1>,
    allocation_ref: &'a ResidentRecordRefV1,
    allocation: &'a ProducerIncarnationV1,
    writer_fence_ref: &'a ResidentRecordRefV1,
    writer_fence: &'a PrincipalResidentWriterFenceV1,
    state: EventPublicationState<'a>,
}

impl EventPublicationRaceClosure<'_> {
    /// Borrow the admitted event reference.
    #[must_use]
    pub const fn event(&self) -> &EventReferenceV1 {
        self.event.record
    }

    /// Borrow the producer allocation and immutable reference.
    #[must_use]
    pub const fn allocation(&self) -> (&ResidentRecordRefV1, &ProducerIncarnationV1) {
        (self.allocation_ref, self.allocation)
    }

    /// Borrow the resident-writer fence serializing publication and retirement.
    #[must_use]
    pub const fn writer_fence(&self) -> (&ResidentRecordRefV1, &PrincipalResidentWriterFenceV1) {
        (self.writer_fence_ref, self.writer_fence)
    }

    /// Return whether the source row is active-head or pre-retirement evidence.
    #[must_use]
    pub const fn publication_state(&self) -> EventPublicationState<'_> {
        self.state
    }
}

/// Owner boundary that closes producer allocation/publication/retirement order.
pub trait EventPublicationRaceVerifier {
    /// Verify one source publication under an uninterrupted active-writer operation.
    ///
    /// The implementation must prove that allocation/head admission and source
    /// publication shared one uninterrupted sole-active-writer critical
    /// section. Retirement is a separate operation under that same fence. A
    /// drained immutable row is admissible only with proof that its commit
    /// preceded the bound retirement. None of these relations may be inferred
    /// from timestamps or a deployment-store transaction.
    ///
    /// # Errors
    ///
    /// Returns [`EventPublicationRaceRejection`] when that ordering is absent,
    /// stale, or ambiguous.
    fn verify(
        &self,
        closure: &EventPublicationRaceClosure<'_>,
    ) -> Result<(), EventPublicationRaceRejection>;
}

impl<F> EventPublicationRaceVerifier for F
where
    F: Fn(&EventPublicationRaceClosure<'_>) -> Result<(), EventPublicationRaceRejection>,
{
    fn verify(
        &self,
        closure: &EventPublicationRaceClosure<'_>,
    ) -> Result<(), EventPublicationRaceRejection> {
        self(closure)
    }
}

/// The owner could not prove publication before producer retirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("owner publication is not serialized with producer retirement")]
pub struct EventPublicationRaceRejection;

/// Admitted owner-publication input with its external race verifier.
pub struct AdmittedOwnerPublication<'a> {
    closure: EventPublicationRaceClosure<'a>,
    verifier: &'a dyn EventPublicationRaceVerifier,
}

impl<'a> AdmittedOwnerPublication<'a> {
    /// Bind admitted records to the owner of cross-store serialization proof.
    #[must_use]
    pub const fn new(
        event: AdmittedEventRecord<'a, EventReferenceV1>,
        allocation_ref: &'a ResidentRecordRefV1,
        allocation: &'a ProducerIncarnationV1,
        writer_fence_ref: &'a ResidentRecordRefV1,
        writer_fence: &'a PrincipalResidentWriterFenceV1,
        state: EventPublicationState<'a>,
        verifier: &'a dyn EventPublicationRaceVerifier,
    ) -> Self {
        Self {
            closure: EventPublicationRaceClosure {
                event,
                allocation_ref,
                allocation,
                writer_fence_ref,
                writer_fence,
                state,
            },
            verifier,
        }
    }
}

/// Complete typed semantic input for one Event custody transition.
pub enum EventCustodyTransitionInput<'a> {
    /// Admit one externally owned immutable publication reference.
    AdmitOwnerReference(AdmittedOwnerPublication<'a>),
    /// Retain a same-identity/different-digest conflict without replacing the incumbent.
    QuarantineIdentityConflict {
        /// Exact incumbent event already mapped at the identity key.
        incumbent: &'a ControlRecord,
        /// Unlike admitted candidate claiming the same full identity.
        candidate: AdmittedEventRecord<'a, EventReferenceV1>,
        /// Immutable private conflict-record reference.
        conflict_ref: &'a ResidentRecordRefV1,
    },
    /// Install one active admitted subscription.
    AdmitSubscription(&'a AdmittedEventSubscription),
    /// Replace one active subscription with its terminal successor.
    RetireSubscription {
        /// Current active state.
        predecessor: &'a AdmittedEventSubscription,
        /// Terminal immutable successor.
        successor: &'a AdmittedEventSubscription,
    },
    /// Enqueue one first delivery attempt.
    EnqueueDelivery {
        /// Exact admitted event.
        event: AdmittedEventRecord<'a, EventReferenceV1>,
        /// Active subscription selecting the event.
        subscription: &'a AdmittedEventSubscription,
        /// Initial pending inbox state.
        delivery: &'a EventDeliveryState,
        /// Immutable first attempt.
        attempt: &'a EventDeliveryAttempt,
        /// Current stream head, absent only for the first subscription delivery.
        stream_predecessor: Option<&'a EventDeliveryStream>,
        /// Exact next subscription-stream allocation.
        stream_successor: &'a EventDeliveryStream,
    },
    /// Enqueue one explicit replay attempt without changing event identity.
    EnqueueReplay {
        /// Exact admitted event.
        event: AdmittedEventRecord<'a, EventReferenceV1>,
        /// Active subscription selecting the event.
        subscription: &'a AdmittedEventSubscription,
        /// Current inbox state.
        predecessor: &'a EventDeliveryState,
        /// Pending replay successor.
        successor: &'a EventDeliveryState,
        /// Distinct immutable replay attempt.
        attempt: &'a EventDeliveryAttempt,
        /// Current subscription delivery-stream allocation head.
        stream_predecessor: &'a EventDeliveryStream,
        /// Exact successor allocating the replay ordinal.
        stream_successor: &'a EventDeliveryStream,
    },
    /// Record one signed delivery disposition and advance only its inbox state.
    RecordAcknowledgement {
        /// Immutable attempt receiving the acknowledgement.
        attempt: &'a EventDeliveryAttempt,
        /// Current pending inbox state.
        predecessor: &'a EventDeliveryState,
        /// Terminal inbox successor.
        successor: &'a EventDeliveryState,
        /// Signed delivery-only acknowledgement.
        acknowledgement: AdmittedEventRecord<'a, EventAcknowledgementV1>,
    },
    /// Advance a consumer cursor by exactly one contiguous terminal delivery.
    AdvanceDeliveryCursor {
        /// Immutable attempt whose terminal acknowledgement advances the cursor.
        attempt: &'a EventDeliveryAttempt,
        /// Exact acknowledgement authorizing the terminal step.
        acknowledgement: AdmittedEventRecord<'a, EventAcknowledgementV1>,
        /// Current cursor, absent only at ordinal one.
        predecessor: Option<&'a EventDeliveryCursor>,
        /// Exact contiguous successor.
        successor: &'a EventDeliveryCursor,
    },
    /// Advance one source's independently typed publication frontier.
    AdvanceSourceFrontier {
        /// Current frontier, when one exists.
        predecessor: Option<&'a EventSourceFrontier>,
        /// Monotone successor.
        successor: &'a EventSourceFrontier,
    },
    /// Atomically retain deterministic placement and its exact outbox entry.
    ProjectInteractionOccurrence {
        /// Admitted event being placed.
        event: AdmittedEventRecord<'a, EventReferenceV1>,
        /// Signed body-free occurrence projection.
        occurrence: AdmittedEventRecord<'a, InteractionOccurrenceV1>,
        /// Immutable reference for the new outbox state; not the publication identity.
        retention_record_ref: &'a ResidentRecordRefV1,
        /// Combined encoded publication and retention bytes permitted for storage.
        /// Checked after canonicalization; not a serialization memory/work bound.
        maximum_publication_bytes: usize,
    },
    /// Atomically retain a directed relation and its exact outbox entry.
    ProjectGenealogyEdge {
        /// Exact source occurrence.
        source: AdmittedEventRecord<'a, InteractionOccurrenceV1>,
        /// Exact destination occurrence.
        destination: AdmittedEventRecord<'a, InteractionOccurrenceV1>,
        /// Signed directed edge.
        edge: AdmittedEventRecord<'a, GenealogyEdgeV1>,
        /// Immutable reference for the new outbox state; not the publication identity.
        retention_record_ref: &'a ResidentRecordRefV1,
        /// Combined encoded publication and retention bytes permitted for storage.
        /// Checked after canonicalization; not a serialization memory/work bound.
        maximum_publication_bytes: usize,
    },
    /// Append a frontier snapshot, move its head and retain its exact outbox entry.
    AdvanceReconciliationFrontier {
        /// Current immutable snapshot, when one exists.
        predecessor: Option<AdmittedEventRecord<'a, ReconciliationFrontierV1>>,
        /// Immutable successor snapshot.
        successor: AdmittedEventRecord<'a, ReconciliationFrontierV1>,
        /// Immutable reference for the new outbox state; not the publication identity.
        retention_record_ref: &'a ResidentRecordRefV1,
        /// Combined encoded publication and retention bytes permitted for storage.
        /// Checked after canonicalization; not a serialization memory/work bound.
        maximum_publication_bytes: usize,
    },
    /// Settle one exact retained Pale-derived publication.
    SettlePaleDerivedPublication {
        /// Current retained state.
        predecessor: &'a PaleDerivedPublicationRetention,
        /// Terminal successor.
        successor: &'a PaleDerivedPublicationRetention,
    },
}

/// Semantic or physical projection failure before database access.
#[derive(Debug, thiserror::Error)]
pub enum EventCustodyProjectionError {
    /// An admitted public record no longer verifies.
    #[error(transparent)]
    Semantic(#[from] InvalidResidentRecordV1),
    /// Cross-record identity, lifecycle, ordering, or proof lineage is inexact.
    #[error("event custody records do not form the selected exact lineage")]
    Lineage,
    /// The encoded publication and retention exceed the selected storage ceiling.
    #[error("derived publication exceeds its retained byte bound")]
    PublicationBytes,
    /// The source-publication owner rejected cross-store serialization evidence.
    #[error(transparent)]
    PublicationRace(#[from] EventPublicationRaceRejection),
    /// Canonical key or private-record encoding failed.
    #[error(transparent)]
    Canonicalize(#[from] canon::CanonError),
    /// A physical immutable record could not be represented.
    #[error(transparent)]
    Record(#[from] ControlRecordError),
    /// Shared admitted-record projection failed before storage access.
    #[error("project admitted Event record into physical custody")]
    PhysicalProjection,
    /// The closed physical port rejected the projected transition shape.
    #[error(transparent)]
    Candidate(#[from] EventCustodyCandidateError),
}

impl From<ControlRecordProjectionError> for EventCustodyProjectionError {
    fn from(_: ControlRecordProjectionError) -> Self {
        Self::PhysicalProjection
    }
}

/// Project one semantic transition into the closed physical Event port shape.
///
/// # Errors
///
/// Returns [`EventCustodyProjectionError`] before storage access when admitted
/// records, cross-record lineage, canonical keys, or the exact port shape fail.
pub fn prepare_event_custody_transition(
    store: AuthorityStoreCoordinate,
    transaction: pale_ports::ControlTransactionPreparation,
    journal_append: pale_ports::PreparedAuthorityAppend,
    input: EventCustodyTransitionInput<'_>,
) -> Result<PreparedEventCustodyTransition, EventCustodyProjectionError> {
    let sequence = journal_append.entry().sequence();
    projection::project_content(store, sequence, input)?
        .attach(transaction, journal_append)
        .map(PreparedEventCustodyTransition::new)
}

/// Select matching active subscriptions in stable identity order.
///
/// The subscription's fixed ingress frontier excludes earlier events. Replay
/// of excluded history requires the explicit replay transition; this function
/// never makes admission retroactive.
#[must_use]
pub fn plan_event_fanout<'a>(
    event: &EventReferenceV1,
    committed_at: pale_core::records::JournalSequenceV1,
    subscriptions: &'a [AdmittedEventSubscription],
) -> Vec<SubscriptionDeliveryTarget<'a>> {
    let mut selected = subscriptions
        .iter()
        .filter(|subscription| subscription.selects(event, committed_at))
        .map(SubscriptionDeliveryTarget::new)
        .collect::<Vec<_>>();
    selected.sort_unstable_by(|left, right| left.subscription_id().cmp(right.subscription_id()));
    selected
}

fn require(exact: bool) -> Result<(), EventCustodyProjectionError> {
    exact
        .then_some(())
        .ok_or(EventCustodyProjectionError::Lineage)
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/event_custody.rs"]
mod tests;
