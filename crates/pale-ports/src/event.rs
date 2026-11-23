//! Atomic physical custody for Event delivery and Genealogy projections.
//!
//! Application policy admits source publications, derives full canonical event
//! identity keys, constructs signed carrier records, and prepares the authority
//! journal append. This module closes only finite physical transaction shapes,
//! typed private indexes, bounded reads, and exact post-commit observation.
//! It stores owner references rather than owner bodies and grants no native
//! effect, acceptance, settlement, or cross-owner authority.

use std::collections::BTreeSet;

use pale_core::records::{
    JournalSequenceV1, EVENT_ACKNOWLEDGEMENT_V1_SCHEMA, EVENT_REFERENCE_V1_SCHEMA,
    GENEALOGY_EDGE_V1_SCHEMA, INTERACTION_OCCURRENCE_V1_SCHEMA, RECONCILIATION_FRONTIER_V1_SCHEMA,
};

use crate::{
    AuthorityStoreCoordinate, ControlRecord, ControlRecordCoordinate, ControlRecordDigest,
    ControlStoreError, ControlTransactionPreparation, PreparedAuthorityAppend,
};

mod history;
pub use history::{
    EventCustodyHistoryMember, EventCustodyHistoryObservation, EventCustodyHistoryRead,
};

/// Quarantined conflicts between unlike records claiming one canonical Event identity.
const EVENT_IDENTITY_CONFLICT_STATE_SCHEMA: &str = "pale/internal-event-identity-conflict.v1";
/// Private proof of the exact journal sequence that admitted one Event.
const EVENT_COMMIT_WITNESS_STATE_SCHEMA: &str = "pale/internal-event-commit-witness.v1";
/// Admitted subscription lifecycle state owned by the local Event custodian.
const EVENT_SUBSCRIPTION_STATE_SCHEMA: &str = "pale/internal-event-subscription-state.v1";
/// Per-consumer inbox state for one durable delivery or replay attempt.
const EVENT_INBOX_DELIVERY_STATE_SCHEMA: &str = "pale/internal-event-inbox-delivery-state.v1";
/// Immutable evidence for one bounded delivery attempt.
const EVENT_DELIVERY_ATTEMPT_STATE_SCHEMA: &str = "pale/internal-event-delivery-attempt.v1";
/// Current ordinal allocation head for one subscription delivery stream.
const EVENT_DELIVERY_STREAM_STATE_SCHEMA: &str = "pale/internal-event-delivery-stream.v1";
/// Monotone delivery position maintained independently for each consumer.
const EVENT_DELIVERY_CURSOR_STATE_SCHEMA: &str = "pale/internal-event-delivery-cursor.v1";
/// Monotone publication position maintained independently for each source.
const EVENT_SOURCE_FRONTIER_STATE_SCHEMA: &str = "pale/internal-event-source-frontier.v1";
/// Retained Pale-derived publications awaiting their own settlement.
const EVENT_DERIVED_OUTBOX_STATE_SCHEMA: &str = "pale/internal-event-derived-outbox.v1";
/// Private proof that selected records were inserted by one Event transition.
const EVENT_TRANSITION_WITNESS_STATE_SCHEMA: &str = "pale/internal-event-transition-witness.v1";

/// Non-interchangeable publication roles at the Pale Event boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPublicationRole {
    /// An external semantic owner exposes a committed outbox row or durable log.
    OwnerIngress,
    /// Pale retains its own committed derived record before downstream exposure.
    PaleDerivedOutbox,
}

/// One finite physical transition in local Event/Genealogy custody.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventCustodyTransitionKind {
    /// Admit one exact external owner-publication reference.
    ///
    /// Application admission must prove the immutable source outbox row was
    /// committed while its producer incarnation could publish. The deployment
    /// transaction may occur after retirement when draining that pre-existing
    /// row; current retired state alone neither invalidates history nor permits
    /// a new source publication.
    AdmitOwnerReference,
    /// Quarantine reuse of one canonical identity with different event bytes.
    QuarantineIdentityConflict,
    /// Install an admitted durable subscription.
    AdmitSubscription,
    /// Replace a subscription head with its terminal state.
    RetireSubscription,
    /// Enqueue the first delivery attempt in one durable consumer inbox.
    EnqueueDelivery,
    /// Enqueue a distinct replay attempt without changing event identity.
    EnqueueReplay,
    /// Retain one signed delivery-only acknowledgement and inbox successor.
    RecordAcknowledgement,
    /// Advance one consumer's delivery cursor after exact acknowledgement.
    AdvanceDeliveryCursor,
    /// Advance one source's independently typed publication frontier.
    AdvanceSourceFrontier,
    /// Atomically retain an occurrence and its Pale-derived publication state.
    ProjectInteractionOccurrence,
    /// Atomically retain a directed edge and its Pale-derived publication state.
    ProjectGenealogyEdge,
    /// Advance a reconciliation vector and retain its exact publication state.
    AdvanceReconciliationFrontier,
    /// Settle one retained Pale-derived publication.
    SettlePaleDerivedPublication,
}

impl EventCustodyTransitionKind {
    /// Return the publication role touched by this transition, when any.
    #[must_use]
    pub const fn publication_role(self) -> Option<EventPublicationRole> {
        match self {
            Self::AdmitOwnerReference => Some(EventPublicationRole::OwnerIngress),
            Self::ProjectInteractionOccurrence
            | Self::ProjectGenealogyEdge
            | Self::AdvanceReconciliationFrontier
            | Self::SettlePaleDerivedPublication => Some(EventPublicationRole::PaleDerivedOutbox),
            Self::QuarantineIdentityConflict
            | Self::AdmitSubscription
            | Self::RetireSubscription
            | Self::EnqueueDelivery
            | Self::EnqueueReplay
            | Self::RecordAcknowledgement
            | Self::AdvanceDeliveryCursor
            | Self::AdvanceSourceFrontier => None,
        }
    }
}

/// Semantic role of one immutable record in an Event custody transition.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum EventTransitionMemberKind {
    /// Public [`pale_core::records::EventReferenceV1`] carrier.
    EventReference,
    /// Private Event-to-journal admission witness.
    EventCommitWitness,
    /// Private transition-to-journal membership witness.
    TransitionWitness,
    /// Private same-identity/different-digest quarantine record.
    IdentityConflict,
    /// Private admitted subscription state.
    Subscription,
    /// Private durable consumer-inbox entry or successor.
    InboxDelivery,
    /// Private immutable delivery-attempt record.
    DeliveryAttempt,
    /// Private per-subscription delivery-stream allocation head.
    DeliveryStream,
    /// Public [`pale_core::records::EventAcknowledgementV1`] carrier.
    EventAcknowledgement,
    /// Private per-consumer delivery cursor.
    DeliveryCursor,
    /// Private per-source publication frontier.
    SourceFrontier,
    /// Public [`pale_core::records::InteractionOccurrenceV1`] projection.
    InteractionOccurrence,
    /// Public [`pale_core::records::GenealogyEdgeV1`] projection.
    GenealogyEdge,
    /// Public [`pale_core::records::ReconciliationFrontierV1`] projection.
    ReconciliationFrontier,
    /// Private Pale-derived publication state.
    PaleDerivedOutbox,
}

impl EventTransitionMemberKind {
    /// Return the exact public-carrier or application-private schema identity.
    ///
    /// Internal identities type private custody rows without creating a public
    /// contract family or allowing another domain's record into an Event index.
    #[must_use]
    pub const fn required_schema_id(self) -> &'static str {
        match self {
            Self::EventReference => EVENT_REFERENCE_V1_SCHEMA,
            Self::EventCommitWitness => EVENT_COMMIT_WITNESS_STATE_SCHEMA,
            Self::TransitionWitness => EVENT_TRANSITION_WITNESS_STATE_SCHEMA,
            Self::IdentityConflict => EVENT_IDENTITY_CONFLICT_STATE_SCHEMA,
            Self::Subscription => EVENT_SUBSCRIPTION_STATE_SCHEMA,
            Self::InboxDelivery => EVENT_INBOX_DELIVERY_STATE_SCHEMA,
            Self::DeliveryAttempt => EVENT_DELIVERY_ATTEMPT_STATE_SCHEMA,
            Self::DeliveryStream => EVENT_DELIVERY_STREAM_STATE_SCHEMA,
            Self::EventAcknowledgement => EVENT_ACKNOWLEDGEMENT_V1_SCHEMA,
            Self::DeliveryCursor => EVENT_DELIVERY_CURSOR_STATE_SCHEMA,
            Self::SourceFrontier => EVENT_SOURCE_FRONTIER_STATE_SCHEMA,
            Self::InteractionOccurrence => INTERACTION_OCCURRENCE_V1_SCHEMA,
            Self::GenealogyEdge => GENEALOGY_EDGE_V1_SCHEMA,
            Self::ReconciliationFrontier => RECONCILIATION_FRONTIER_V1_SCHEMA,
            Self::PaleDerivedOutbox => EVENT_DERIVED_OUTBOX_STATE_SCHEMA,
        }
    }
}

/// One role-tagged immutable record in a prepared Event transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventTransitionMember {
    kind: EventTransitionMemberKind,
    record: ControlRecord,
}

impl EventTransitionMember {
    /// Bind one already-admitted record to its closed physical role.
    #[must_use]
    pub const fn new(kind: EventTransitionMemberKind, record: ControlRecord) -> Self {
        Self { kind, record }
    }

    /// Return the record's role in this transition.
    #[must_use]
    pub const fn kind(&self) -> EventTransitionMemberKind {
        self.kind
    }

    /// Borrow the exact immutable record.
    #[must_use]
    pub const fn record(&self) -> &ControlRecord {
        &self.record
    }
}

/// Digest of an application-derived canonical Event private-index key.
///
/// For [`EventIndexQuery::EventIdentity`], the preimage is the canonical full
/// `(origin principal, producer epoch, event identity)` tuple. For
/// [`EventIndexQuery::OwnerSource`], the application derives a stable owner
/// source and projection key without the producer epoch. The digest is a
/// physical key only; this port neither accepts a bare event ID nor invents a
/// universal correlation identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct EventIndexKey(ControlRecordDigest);

impl EventIndexKey {
    /// Retain one already validated lowercase SHA-256 key digest.
    #[must_use]
    pub const fn new(digest: ControlRecordDigest) -> Self {
        Self(digest)
    }

    /// Borrow the exact lowercase digest spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Closed point-query family for Event restart, replay, and projection recovery.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventIndexQuery {
    /// Resolve the full canonical event identity to its immutable reference.
    EventIdentity(EventIndexKey),
    /// Resolve one Event identity to its exact admission sequence witness.
    EventCommitWitness(EventIndexKey),
    /// Resolve one transition subject to its exact journal-membership witness.
    TransitionWitness(EventIndexKey),
    /// Resolve one admitted owner source and projection to its immutable Event.
    OwnerSource(EventIndexKey),
    /// Resolve a canonical event identity to retained conflict evidence.
    IdentityConflict(EventIndexKey),
    /// Resolve one subscription identity to its current state.
    Subscription(EventIndexKey),
    /// Resolve one subscription/event pair to its current inbox state.
    InboxDelivery(EventIndexKey),
    /// Resolve one immutable delivery attempt.
    DeliveryAttempt(EventIndexKey),
    /// Resolve one subscription to its current delivery-stream allocation head.
    DeliveryStream(EventIndexKey),
    /// Resolve one attempt to its delivery-only acknowledgement.
    Acknowledgement(EventIndexKey),
    /// Resolve one consumer/subscription pair to its delivery cursor.
    DeliveryCursor(EventIndexKey),
    /// Resolve one source subscription to its publication frontier.
    SourceFrontier(EventIndexKey),
    /// Resolve one event reference to its deterministic occurrence.
    InteractionOccurrence(EventIndexKey),
    /// Resolve one exact directed/truth-classed edge identity.
    GenealogyEdge(EventIndexKey),
    /// Resolve one subject to its current independent frontier vector.
    ReconciliationFrontier(EventIndexKey),
    /// Resolve one Pale-derived publication identity to retained outbox state.
    PaleDerivedOutbox(EventIndexKey),
}

impl EventIndexQuery {
    /// Borrow the exact application-derived key.
    #[must_use]
    pub const fn key(&self) -> &EventIndexKey {
        match self {
            Self::EventIdentity(key)
            | Self::EventCommitWitness(key)
            | Self::TransitionWitness(key)
            | Self::OwnerSource(key)
            | Self::IdentityConflict(key)
            | Self::Subscription(key)
            | Self::InboxDelivery(key)
            | Self::DeliveryAttempt(key)
            | Self::DeliveryStream(key)
            | Self::Acknowledgement(key)
            | Self::DeliveryCursor(key)
            | Self::SourceFrontier(key)
            | Self::InteractionOccurrence(key)
            | Self::GenealogyEdge(key)
            | Self::ReconciliationFrontier(key)
            | Self::PaleDerivedOutbox(key) => key,
        }
    }

    const fn role(&self) -> EventIndexRole {
        match self {
            Self::EventIdentity(_) => EventIndexRole::EventIdentity,
            Self::EventCommitWitness(_) => EventIndexRole::EventCommitWitness,
            Self::TransitionWitness(_) => EventIndexRole::TransitionWitness,
            Self::OwnerSource(_) => EventIndexRole::OwnerSource,
            Self::IdentityConflict(_) => EventIndexRole::IdentityConflict,
            Self::Subscription(_) => EventIndexRole::Subscription,
            Self::InboxDelivery(_) => EventIndexRole::InboxDelivery,
            Self::DeliveryAttempt(_) => EventIndexRole::DeliveryAttempt,
            Self::DeliveryStream(_) => EventIndexRole::DeliveryStream,
            Self::Acknowledgement(_) => EventIndexRole::Acknowledgement,
            Self::DeliveryCursor(_) => EventIndexRole::DeliveryCursor,
            Self::SourceFrontier(_) => EventIndexRole::SourceFrontier,
            Self::InteractionOccurrence(_) => EventIndexRole::InteractionOccurrence,
            Self::GenealogyEdge(_) => EventIndexRole::GenealogyEdge,
            Self::ReconciliationFrontier(_) => EventIndexRole::ReconciliationFrontier,
            Self::PaleDerivedOutbox(_) => EventIndexRole::PaleDerivedOutbox,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EventIndexRole {
    EventIdentity,
    EventCommitWitness,
    TransitionWitness,
    OwnerSource,
    IdentityConflict,
    Subscription,
    InboxDelivery,
    DeliveryAttempt,
    DeliveryStream,
    Acknowledgement,
    DeliveryCursor,
    SourceFrontier,
    InteractionOccurrence,
    GenealogyEdge,
    ReconciliationFrontier,
    PaleDerivedOutbox,
}

impl EventIndexRole {
    const fn required_schema_id(self) -> &'static str {
        match self {
            Self::EventIdentity => EVENT_REFERENCE_V1_SCHEMA,
            Self::EventCommitWitness => EVENT_COMMIT_WITNESS_STATE_SCHEMA,
            Self::TransitionWitness => EVENT_TRANSITION_WITNESS_STATE_SCHEMA,
            Self::OwnerSource => EVENT_REFERENCE_V1_SCHEMA,
            Self::IdentityConflict => EVENT_IDENTITY_CONFLICT_STATE_SCHEMA,
            Self::Subscription => EVENT_SUBSCRIPTION_STATE_SCHEMA,
            Self::InboxDelivery => EVENT_INBOX_DELIVERY_STATE_SCHEMA,
            Self::DeliveryAttempt => EVENT_DELIVERY_ATTEMPT_STATE_SCHEMA,
            Self::DeliveryStream => EVENT_DELIVERY_STREAM_STATE_SCHEMA,
            Self::Acknowledgement => EVENT_ACKNOWLEDGEMENT_V1_SCHEMA,
            Self::DeliveryCursor => EVENT_DELIVERY_CURSOR_STATE_SCHEMA,
            Self::SourceFrontier => EVENT_SOURCE_FRONTIER_STATE_SCHEMA,
            Self::InteractionOccurrence => INTERACTION_OCCURRENCE_V1_SCHEMA,
            Self::GenealogyEdge => GENEALOGY_EDGE_V1_SCHEMA,
            Self::ReconciliationFrontier => RECONCILIATION_FRONTIER_V1_SCHEMA,
            Self::PaleDerivedOutbox => EVENT_DERIVED_OUTBOX_STATE_SCHEMA,
        }
    }
}

/// Exact absent or immutable predecessor required by one typed index operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventIndexPredecessor {
    /// No row may exist at the typed key.
    Absent,
    /// The key must contain this exact immutable record.
    Exact(ControlRecord),
}

/// One typed private-index compare-and-set mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventIndexMutation {
    query: EventIndexQuery,
    expected: EventIndexPredecessor,
    successor: ControlRecordCoordinate,
}

impl EventIndexMutation {
    /// Bind a typed index key, exact predecessor, and candidate-owned successor.
    #[must_use]
    pub const fn new(
        query: EventIndexQuery,
        expected: EventIndexPredecessor,
        successor: ControlRecordCoordinate,
    ) -> Self {
        Self {
            query,
            expected,
            successor,
        }
    }

    /// Borrow the exact typed query and key.
    #[must_use]
    pub const fn query(&self) -> &EventIndexQuery {
        &self.query
    }

    /// Borrow the exact compare-and-set predecessor.
    #[must_use]
    pub const fn expected(&self) -> &EventIndexPredecessor {
        &self.expected
    }

    /// Borrow the immutable record that becomes the index head.
    #[must_use]
    pub const fn successor(&self) -> &ControlRecordCoordinate {
        &self.successor
    }
}

/// One typed private-index read asserted without mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventIndexAssertion {
    query: EventIndexQuery,
    expected: EventIndexPredecessor,
}

impl EventIndexAssertion {
    /// Require one exact typed private-index observation.
    #[must_use]
    pub const fn new(query: EventIndexQuery, expected: EventIndexPredecessor) -> Self {
        Self { query, expected }
    }

    /// Borrow the exact typed query and key.
    #[must_use]
    pub const fn query(&self) -> &EventIndexQuery {
        &self.query
    }

    /// Borrow the exact required predecessor.
    #[must_use]
    pub const fn expected(&self) -> &EventIndexPredecessor {
        &self.expected
    }
}

/// Closed Event transition prepared for one atomic physical commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventCustodyTransition {
    store: AuthorityStoreCoordinate,
    transaction: ControlTransactionPreparation,
    kind: EventCustodyTransitionKind,
    journal_append: PreparedAuthorityAppend,
    members: Box<[EventTransitionMember]>,
    index_mutations: Box<[EventIndexMutation]>,
    index_assertions: Box<[EventIndexAssertion]>,
}

impl EventCustodyTransition {
    /// Prove one finite transaction shape against the single authority journal.
    ///
    /// # Errors
    ///
    /// Returns [`EventCustodyCandidateError`] for store disagreement, wrong
    /// record/index shape, schema mismatch, repeated coordinates or keys, a
    /// successor outside the candidate, or an ill-typed predecessor.
    pub fn new(
        store: AuthorityStoreCoordinate,
        transaction: ControlTransactionPreparation,
        kind: EventCustodyTransitionKind,
        journal_append: PreparedAuthorityAppend,
        members: Vec<EventTransitionMember>,
        index_mutations: Vec<EventIndexMutation>,
        index_assertions: Vec<EventIndexAssertion>,
    ) -> Result<Self, EventCustodyCandidateError> {
        if journal_append.store() != &store || transaction.store() != &store {
            return Err(EventCustodyCandidateError::AuthorityJournal);
        }
        validate_member_shape(kind, &members)?;
        validate_mutation_shape(kind, &members, &index_mutations)?;
        validate_assertion_shape(kind, &index_assertions)?;

        let coordinates = members
            .iter()
            .map(|member| member.record().coordinate())
            .collect::<BTreeSet<_>>();
        if coordinates.len() != members.len() {
            return Err(EventCustodyCandidateError::DuplicateRecordCoordinate);
        }

        Ok(Self {
            store,
            transaction,
            kind,
            journal_append,
            members: members.into_boxed_slice(),
            index_mutations: index_mutations.into_boxed_slice(),
            index_assertions: index_assertions.into_boxed_slice(),
        })
    }

    /// Borrow the exact authority-store incarnation.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Borrow the exact control transaction selected before journal preparation.
    #[must_use]
    pub const fn transaction(&self) -> &ControlTransactionPreparation {
        &self.transaction
    }

    /// Return the finite transition class.
    #[must_use]
    pub const fn kind(&self) -> EventCustodyTransitionKind {
        self.kind
    }

    /// Borrow the exact append to the shared authority journal.
    #[must_use]
    pub const fn journal_append(&self) -> &PreparedAuthorityAppend {
        &self.journal_append
    }

    /// Borrow every immutable transition record.
    #[must_use]
    pub const fn members(&self) -> &[EventTransitionMember] {
        &self.members
    }

    /// Borrow every typed compare-and-set index mutation.
    #[must_use]
    pub const fn index_mutations(&self) -> &[EventIndexMutation] {
        &self.index_mutations
    }

    /// Borrow every co-evaluated typed index assertion.
    #[must_use]
    pub const fn index_assertions(&self) -> &[EventIndexAssertion] {
        &self.index_assertions
    }
}

/// A prepared Event transition is not physically closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EventCustodyCandidateError {
    /// The combined historical-read limit cannot contain the journal body.
    #[error("event history byte limit is smaller than the journal body")]
    HistoryByteLimit,
    /// The append or transaction targets another authority-store incarnation.
    #[error("event transition does not use the selected authority store")]
    AuthorityJournal,
    /// Record roles do not exactly match the selected transition.
    #[error("event transition has the wrong record shape")]
    MemberShape,
    /// A public carrier or application-private state record uses another schema.
    #[error("event transition member uses the wrong role schema")]
    MemberSchema,
    /// Two members name the same immutable record coordinate.
    #[error("event transition repeats a record coordinate")]
    DuplicateRecordCoordinate,
    /// Index mutations do not exactly match the selected transition.
    #[error("event transition has the wrong index-mutation shape")]
    IndexMutationShape,
    /// A typed index key is mutated more than once.
    #[error("event transition repeats a typed index mutation")]
    DuplicateIndexMutation,
    /// A mutation successor is not a compatible candidate member.
    #[error("event index successor is not closed by the transition")]
    IndexSuccessorClosure,
    /// An index predecessor does not match the selected lifecycle phase.
    #[error("event index predecessor does not match the transition phase")]
    IndexPredecessorShape,
    /// An exact predecessor cannot inhabit the selected typed index.
    #[error("event index predecessor uses the wrong record schema")]
    IndexPredecessorSchema,
    /// Index assertions do not exactly match the selected transition.
    #[error("event transition has the wrong index-assertion shape")]
    IndexAssertionShape,
}

/// Successful physical comparison of one Event transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventCustodyCommitOutcome {
    /// The store committed the complete transition atomically.
    Committed,
    /// The store already contains the complete transition byte-for-byte.
    ExactReplay,
}

/// Exact post-restart or post-ambiguity transition observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventCustodyReadBackOutcome {
    /// Every predecessor remains current and no candidate row exists.
    Predecessor,
    /// Journal entry, records, and indexes equal the candidate byte-for-byte.
    ExactSuccessor,
    /// Observed state is partial, contradictory, or owned by another transition.
    ConflictingOrPartial,
}

/// Exact point-read result for one typed Event private index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventIndexObservation {
    query: EventIndexQuery,
    record: Option<ControlRecord>,
}

impl EventIndexObservation {
    /// Construct an absent result for one exact typed key.
    #[must_use]
    pub const fn absent(query: EventIndexQuery) -> Self {
        Self {
            query,
            record: None,
        }
    }

    /// Bind one stored record to the exact typed key.
    ///
    /// # Errors
    ///
    /// Returns [`EventObservationError::RecordSchema`] when a public index
    /// points at a record outside its carrier family.
    pub fn exact_record(
        query: EventIndexQuery,
        record: ControlRecord,
    ) -> Result<Self, EventObservationError> {
        query_accepts_schema(&query, record.schema_id().as_str())
            .then_some(Self {
                query,
                record: Some(record),
            })
            .ok_or(EventObservationError::RecordSchema)
    }

    /// Borrow the exact typed query.
    #[must_use]
    pub const fn query(&self) -> &EventIndexQuery {
        &self.query
    }

    /// Borrow the exact row, or `None` when the key was absent.
    #[must_use]
    pub const fn record(&self) -> Option<&ControlRecord> {
        self.record.as_ref()
    }
}

/// Exact point-read result for one immutable Event custody record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventRecordObservation {
    coordinate: ControlRecordCoordinate,
    record: Option<ControlRecord>,
}

impl EventRecordObservation {
    /// Construct an absent result for one exact coordinate.
    #[must_use]
    pub const fn absent(coordinate: ControlRecordCoordinate) -> Self {
        Self {
            coordinate,
            record: None,
        }
    }

    /// Bind a stored record to the exact requested coordinate.
    ///
    /// # Errors
    ///
    /// Returns [`EventObservationError::RecordCoordinate`] for a different row.
    pub fn exact(
        coordinate: ControlRecordCoordinate,
        record: ControlRecord,
    ) -> Result<Self, EventObservationError> {
        (record.coordinate() == &coordinate)
            .then_some(Self {
                coordinate,
                record: Some(record),
            })
            .ok_or(EventObservationError::RecordCoordinate)
    }

    /// Borrow the requested immutable coordinate.
    #[must_use]
    pub const fn coordinate(&self) -> &ControlRecordCoordinate {
        &self.coordinate
    }

    /// Borrow the exact row, or `None` when it was absent.
    #[must_use]
    pub const fn record(&self) -> Option<&ControlRecord> {
        self.record.as_ref()
    }
}

/// A physical point observation violated its requested identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EventObservationError {
    /// A public typed index returned a record from another schema family.
    #[error("event index observation uses the wrong record schema")]
    RecordSchema,
    /// A point read returned a different immutable coordinate.
    #[error("event record observation uses the wrong record coordinate")]
    RecordCoordinate,
}

/// Stable total-order cursor for a durable subscription inbox.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventInboxCursor {
    journal_sequence: JournalSequenceV1,
    delivery_key: EventIndexKey,
}

impl EventInboxCursor {
    /// Pair the committing journal sequence with the delivery's exact key.
    #[must_use]
    pub const fn new(journal_sequence: JournalSequenceV1, delivery_key: EventIndexKey) -> Self {
        Self {
            journal_sequence,
            delivery_key,
        }
    }

    /// Return the authority-journal sequence that committed this delivery.
    #[must_use]
    pub const fn journal_sequence(&self) -> JournalSequenceV1 {
        self.journal_sequence
    }

    /// Borrow the exact delivery key used as a stable tie breaker.
    #[must_use]
    pub const fn delivery_key(&self) -> &EventIndexKey {
        &self.delivery_key
    }
}

/// Bounded inbox page request against one fixed authority-journal frontier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventInboxPageRead {
    store: AuthorityStoreCoordinate,
    subscription_key: EventIndexKey,
    after: Option<EventInboxCursor>,
    through: JournalSequenceV1,
    maximum_items: usize,
    maximum_record_bytes: usize,
    maximum_total_record_bytes: usize,
}

impl EventInboxPageRead {
    /// Select one subscription page under an explicit item bound.
    ///
    /// # Errors
    ///
    /// Returns [`EventPageError`] for a zero bound or a cursor beyond `through`.
    pub fn new(
        store: AuthorityStoreCoordinate,
        subscription_key: EventIndexKey,
        after: Option<EventInboxCursor>,
        through: JournalSequenceV1,
        maximum_items: usize,
        maximum_record_bytes: usize,
        maximum_total_record_bytes: usize,
    ) -> Result<Self, EventPageError> {
        if maximum_items == 0 || maximum_record_bytes == 0 || maximum_total_record_bytes == 0 {
            return Err(EventPageError::ZeroBound);
        }
        if after
            .as_ref()
            .is_some_and(|cursor| cursor.journal_sequence() > through)
        {
            return Err(EventPageError::CursorBeyondFrontier);
        }
        Ok(Self {
            store,
            subscription_key,
            after,
            through,
            maximum_items,
            maximum_record_bytes,
            maximum_total_record_bytes,
        })
    }

    /// Borrow the selected authority-store incarnation.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Borrow the exact admitted subscription key.
    #[must_use]
    pub const fn subscription_key(&self) -> &EventIndexKey {
        &self.subscription_key
    }

    /// Borrow the exclusive stable cursor, when continuing a prior page.
    #[must_use]
    pub const fn after(&self) -> Option<&EventInboxCursor> {
        self.after.as_ref()
    }

    /// Return the inclusive journal frontier fixed for this observation pass.
    #[must_use]
    pub const fn through(&self) -> JournalSequenceV1 {
        self.through
    }

    /// Return the hard maximum number of rows in one page.
    #[must_use]
    pub const fn maximum_items(&self) -> usize {
        self.maximum_items
    }

    /// Return the hard maximum body bytes for one row.
    #[must_use]
    pub const fn maximum_record_bytes(&self) -> usize {
        self.maximum_record_bytes
    }

    /// Return the hard aggregate body-byte maximum for the page.
    #[must_use]
    pub const fn maximum_total_record_bytes(&self) -> usize {
        self.maximum_total_record_bytes
    }
}

/// One durable inbox row paired with its stable pagination cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventInboxPageEntry {
    cursor: EventInboxCursor,
    record: ControlRecord,
}

impl EventInboxPageEntry {
    /// Bind one private inbox record to its stable order coordinate.
    #[must_use]
    pub const fn new(cursor: EventInboxCursor, record: ControlRecord) -> Self {
        Self { cursor, record }
    }

    /// Borrow the stable order coordinate.
    #[must_use]
    pub const fn cursor(&self) -> &EventInboxCursor {
        &self.cursor
    }

    /// Borrow the exact private inbox record.
    #[must_use]
    pub const fn record(&self) -> &ControlRecord {
        &self.record
    }
}

/// Completion state of one bounded durable-inbox page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventInboxPageBoundary {
    /// No additional inbox row exists through the fixed frontier.
    Complete,
    /// More rows remain; continue strictly after this exact cursor.
    More {
        /// Exact last cursor returned by this page.
        resume_after: EventInboxCursor,
    },
}

/// One ordered, bounded page of a subscription's durable inbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventInboxPage {
    entries: Box<[EventInboxPageEntry]>,
    boundary: EventInboxPageBoundary,
}

impl EventInboxPage {
    /// Bind rows to the exact request, stable order, and continuation boundary.
    ///
    /// # Errors
    ///
    /// Returns [`EventPageError`] for overflow, a stale/repeated cursor, a row
    /// beyond the fixed journal frontier, or an inexact continuation cursor.
    pub fn new(
        request: &EventInboxPageRead,
        entries: Vec<EventInboxPageEntry>,
        boundary: EventInboxPageBoundary,
    ) -> Result<Self, EventPageError> {
        if entries.len() > request.maximum_items {
            return Err(EventPageError::BoundExceeded);
        }
        let mut total_record_bytes = 0usize;
        for entry in &entries {
            let record_bytes = entry.record.canonical_record_jcs().as_slice().len();
            if record_bytes > request.maximum_record_bytes {
                return Err(EventPageError::RecordBytesExceeded);
            }
            total_record_bytes = total_record_bytes
                .checked_add(record_bytes)
                .ok_or(EventPageError::RecordBytesExceeded)?;
            if total_record_bytes > request.maximum_total_record_bytes {
                return Err(EventPageError::RecordBytesExceeded);
            }
        }
        if entries
            .windows(2)
            .any(|pair| pair[0].cursor >= pair[1].cursor)
            || entries.first().is_some_and(|entry| {
                request
                    .after
                    .as_ref()
                    .is_some_and(|after| entry.cursor <= *after)
            })
        {
            return Err(EventPageError::NonAscending);
        }
        if entries
            .iter()
            .any(|entry| entry.cursor.journal_sequence() > request.through)
        {
            return Err(EventPageError::CursorBeyondFrontier);
        }
        if entries
            .iter()
            .any(|entry| entry.record.schema_id().as_str() != EVENT_INBOX_DELIVERY_STATE_SCHEMA)
        {
            return Err(EventPageError::RecordSchema);
        }
        if let EventInboxPageBoundary::More { resume_after } = &boundary {
            if entries.last().map(EventInboxPageEntry::cursor) != Some(resume_after) {
                return Err(EventPageError::Continuation);
            }
        }
        Ok(Self {
            entries: entries.into_boxed_slice(),
            boundary,
        })
    }

    /// Borrow the stable ascending inbox rows.
    #[must_use]
    pub const fn entries(&self) -> &[EventInboxPageEntry] {
        &self.entries
    }

    /// Borrow the exact completion or continuation state.
    #[must_use]
    pub const fn boundary(&self) -> &EventInboxPageBoundary {
        &self.boundary
    }
}

/// A bounded inbox page is physically inconsistent with its request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EventPageError {
    /// A zero item bound cannot admit even one row.
    #[error("event inbox page bound is zero")]
    ZeroBound,
    /// The page contains more rows than the explicit request bound.
    #[error("event inbox page exceeds its item bound")]
    BoundExceeded,
    /// One row or all returned row bodies exceed their explicit byte ceiling.
    #[error("event inbox page exceeds its record byte bound")]
    RecordBytesExceeded,
    /// A cursor is outside the request's fixed journal frontier.
    #[error("event inbox cursor is beyond the fixed journal frontier")]
    CursorBeyondFrontier,
    /// Rows are repeated, stale, or not strictly ascending.
    #[error("event inbox page is not in strict cursor order")]
    NonAscending,
    /// A page row is not an Event inbox-delivery state record.
    #[error("event inbox page contains a record from another schema domain")]
    RecordSchema,
    /// The continuation coordinate is absent or differs from the last row.
    #[error("event inbox continuation does not equal the final row")]
    Continuation,
}

/// Stable total-order cursor for the current Pale-derived outbox heads.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PaleDerivedOutboxCursor {
    publication_key: EventIndexKey,
}

impl PaleDerivedOutboxCursor {
    /// Bind one application-derived publication key to its recovery order.
    #[must_use]
    pub const fn new(publication_key: EventIndexKey) -> Self {
        Self { publication_key }
    }

    /// Borrow the exact Pale-derived publication key.
    #[must_use]
    pub const fn publication_key(&self) -> &EventIndexKey {
        &self.publication_key
    }
}

/// Bounded current-head read for Pale-derived publication recovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaleDerivedOutboxPageRead {
    store: AuthorityStoreCoordinate,
    after: Option<PaleDerivedOutboxCursor>,
    maximum_items: usize,
}

impl PaleDerivedOutboxPageRead {
    /// Select one recovery page under an explicit item bound.
    ///
    /// # Errors
    ///
    /// Returns [`PaleDerivedOutboxPageError::ZeroBound`] for a zero bound.
    pub fn new(
        store: AuthorityStoreCoordinate,
        after: Option<PaleDerivedOutboxCursor>,
        maximum_items: usize,
    ) -> Result<Self, PaleDerivedOutboxPageError> {
        if maximum_items == 0 {
            return Err(PaleDerivedOutboxPageError::ZeroBound);
        }
        Ok(Self {
            store,
            after,
            maximum_items,
        })
    }

    /// Borrow the selected authority-store incarnation.
    #[must_use]
    pub const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Borrow the exclusive cursor, when continuing a prior page.
    #[must_use]
    pub const fn after(&self) -> Option<&PaleDerivedOutboxCursor> {
        self.after.as_ref()
    }

    /// Return the hard maximum number of current heads in one page.
    #[must_use]
    pub const fn maximum_items(&self) -> usize {
        self.maximum_items
    }
}

/// One current Pale-derived outbox head and its stable recovery cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaleDerivedOutboxPageEntry {
    cursor: PaleDerivedOutboxCursor,
    record: ControlRecord,
}

impl PaleDerivedOutboxPageEntry {
    /// Bind one exact private retention head to its publication key.
    #[must_use]
    pub const fn new(cursor: PaleDerivedOutboxCursor, record: ControlRecord) -> Self {
        Self { cursor, record }
    }

    /// Borrow the stable recovery cursor.
    #[must_use]
    pub const fn cursor(&self) -> &PaleDerivedOutboxCursor {
        &self.cursor
    }

    /// Borrow the exact private retention record.
    #[must_use]
    pub const fn record(&self) -> &ControlRecord {
        &self.record
    }
}

/// Completion state of one bounded Pale-derived outbox page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaleDerivedOutboxPageBoundary {
    /// No additional current head exists after this page.
    Complete,
    /// More current heads remain after the final returned cursor.
    More {
        /// Exact final cursor returned by this page.
        resume_after: PaleDerivedOutboxCursor,
    },
}

/// One ordered, bounded page of current Pale-derived outbox heads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaleDerivedOutboxPage {
    entries: Box<[PaleDerivedOutboxPageEntry]>,
    boundary: PaleDerivedOutboxPageBoundary,
}

impl PaleDerivedOutboxPage {
    /// Bind current heads to the exact request and continuation boundary.
    ///
    /// This physical page grants no exposure authority. A retained head may
    /// recur across recovery sweeps until application-owned settlement.
    ///
    /// # Errors
    ///
    /// Returns [`PaleDerivedOutboxPageError`] for overflow, stale or repeated
    /// cursors, a wrong private schema, or an inexact continuation cursor.
    pub fn new(
        request: &PaleDerivedOutboxPageRead,
        entries: Vec<PaleDerivedOutboxPageEntry>,
        boundary: PaleDerivedOutboxPageBoundary,
    ) -> Result<Self, PaleDerivedOutboxPageError> {
        if entries.len() > request.maximum_items {
            return Err(PaleDerivedOutboxPageError::BoundExceeded);
        }
        if entries
            .windows(2)
            .any(|pair| pair[0].cursor >= pair[1].cursor)
            || entries.first().is_some_and(|entry| {
                request
                    .after
                    .as_ref()
                    .is_some_and(|after| entry.cursor <= *after)
            })
        {
            return Err(PaleDerivedOutboxPageError::NonAscending);
        }
        if entries
            .iter()
            .any(|entry| entry.record.schema_id().as_str() != EVENT_DERIVED_OUTBOX_STATE_SCHEMA)
        {
            return Err(PaleDerivedOutboxPageError::RecordSchema);
        }
        if let PaleDerivedOutboxPageBoundary::More { resume_after } = &boundary {
            if entries.last().map(PaleDerivedOutboxPageEntry::cursor) != Some(resume_after) {
                return Err(PaleDerivedOutboxPageError::Continuation);
            }
        }
        Ok(Self {
            entries: entries.into_boxed_slice(),
            boundary,
        })
    }

    /// Borrow the stable ascending current heads.
    #[must_use]
    pub const fn entries(&self) -> &[PaleDerivedOutboxPageEntry] {
        &self.entries
    }

    /// Borrow the exact completion or continuation state.
    #[must_use]
    pub const fn boundary(&self) -> &PaleDerivedOutboxPageBoundary {
        &self.boundary
    }
}

/// A Pale-derived outbox page is physically inconsistent with its request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PaleDerivedOutboxPageError {
    /// A zero item bound cannot admit even one current head.
    #[error("Pale-derived outbox page bound is zero")]
    ZeroBound,
    /// The page contains more current heads than the request permits.
    #[error("Pale-derived outbox page exceeds its item bound")]
    BoundExceeded,
    /// Current heads are repeated, stale, or not strictly ascending.
    #[error("Pale-derived outbox page is not in strict cursor order")]
    NonAscending,
    /// A page row is not a Pale-derived retention-state record.
    #[error("Pale-derived outbox page contains a record from another schema domain")]
    RecordSchema,
    /// The continuation cursor is absent or differs from the final row.
    #[error("Pale-derived outbox continuation does not equal the final row")]
    Continuation,
}

/// Atomic physical custody and bounded reads for Event/Genealogy state.
///
/// This deployment-store boundary cannot close the producer-retirement race in
/// the separate principal trust store. A new owner outbox row and producer
/// retirement must be ordered by the sole active principal resident-writer
/// generation: allocation/head admission and source publication occur in one
/// uninterrupted serialized operation, while retirement is another operation
/// under the same fence. [`crate::PrincipalProducerIncarnationReadPort`]
/// supplies exact records for that application proof. A row committed before
/// retirement remains drainable; no observation here authorizes a new row.
pub trait EventCustodyPort {
    /// Read exact historical journal bytes and stamped immutable members.
    ///
    /// Later journal entries and mutable index changes do not invalidate this read.
    /// The caller must check that the journal delta binds the same transaction
    /// and members. The result is not independent proof of that association,
    /// semantic admission or permission.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] for invalid storage, unavailable reads or an
    /// exceeded byte limit. An absent or partial transaction grants no continuation.
    fn read_committed_members(
        &mut self,
        request: &EventCustodyHistoryRead,
    ) -> Result<EventCustodyHistoryObservation, ControlStoreError>;

    /// Commit journal entry, immutable records, and typed indexes atomically.
    /// The positive journal payload limit also bounds comparison with another
    /// writer's entry. It is an invocation limit, not part of candidate identity.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] for conflict, recovery-required state,
    /// unavailable storage, or an unknown commit outcome. Unknown outcome must
    /// be resolved with [`Self::observe_exact`] before any retry.
    fn compare_and_commit(
        &mut self,
        transition: &EventCustodyTransition,
        maximum_journal_payload_bytes: usize,
    ) -> Result<EventCustodyCommitOutcome, ControlStoreError>;

    /// Observe the complete transition without mutation after restart or ambiguity.
    /// Apply the caller's positive journal payload limit before loading or
    /// hashing stored frame bytes. An exceeded limit does not prove corruption.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] when bounded exact observation cannot finish.
    fn observe_exact(
        &mut self,
        transition: &EventCustodyTransition,
        maximum_journal_payload_bytes: usize,
    ) -> Result<EventCustodyReadBackOutcome, ControlStoreError>;

    /// Read one exact typed private index without a scan.
    ///
    /// `maximum_body_bytes` must be positive and bounds the returned record
    /// body before the adapter copies it into memory. It does not bound index
    /// metadata or establish semantic admission.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] when the limit is zero or exhausted, or
    /// physical integrity or availability cannot be proven.
    fn read_index(
        &mut self,
        store: &AuthorityStoreCoordinate,
        query: &EventIndexQuery,
        maximum_body_bytes: usize,
    ) -> Result<EventIndexObservation, ControlStoreError>;

    /// Read one immutable record by exact coordinate.
    ///
    /// `maximum_body_bytes` must be positive and bounds the returned record
    /// body before the adapter copies it into memory.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] when the limit is zero or exhausted, or
    /// physical integrity or availability cannot be proven.
    fn read_record(
        &mut self,
        store: &AuthorityStoreCoordinate,
        coordinate: &ControlRecordCoordinate,
        maximum_body_bytes: usize,
    ) -> Result<EventRecordObservation, ControlStoreError>;

    /// Read one stable bounded page from a durable subscription inbox.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] when physical integrity, ordering,
    /// availability, or the requested bound cannot be proven.
    fn read_inbox_page(
        &mut self,
        request: &EventInboxPageRead,
    ) -> Result<EventInboxPage, ControlStoreError>;

    /// Read one bounded page of current Pale-derived outbox heads.
    ///
    /// # Errors
    ///
    /// Returns [`ControlStoreError`] when physical integrity, ordering,
    /// availability, or the requested bound cannot be proven.
    fn read_pale_derived_outbox_page(
        &mut self,
        request: &PaleDerivedOutboxPageRead,
    ) -> Result<PaleDerivedOutboxPage, ControlStoreError>;
}

fn validate_member_shape(
    kind: EventCustodyTransitionKind,
    members: &[EventTransitionMember],
) -> Result<(), EventCustodyCandidateError> {
    validate_member_roles(kind, members.iter().map(EventTransitionMember::kind))?;
    if members
        .iter()
        .any(|member| member.record().schema_id().as_str() != member.kind().required_schema_id())
    {
        return Err(EventCustodyCandidateError::MemberSchema);
    }
    Ok(())
}

fn validate_member_roles(
    kind: EventCustodyTransitionKind,
    members: impl ExactSizeIterator<Item = EventTransitionMemberKind>,
) -> Result<(), EventCustodyCandidateError> {
    let required = required_member_kinds(kind);
    if members.len() != required.len() || members.collect::<BTreeSet<_>>() != required {
        return Err(EventCustodyCandidateError::MemberShape);
    }
    Ok(())
}

fn validate_mutation_shape(
    kind: EventCustodyTransitionKind,
    members: &[EventTransitionMember],
    mutations: &[EventIndexMutation],
) -> Result<(), EventCustodyCandidateError> {
    let keys = mutations
        .iter()
        .map(|mutation| &mutation.query)
        .collect::<BTreeSet<_>>();
    if keys.len() != mutations.len() {
        return Err(EventCustodyCandidateError::DuplicateIndexMutation);
    }
    let observed = mutations
        .iter()
        .map(|mutation| mutation.query.role())
        .collect::<BTreeSet<_>>();
    if observed.len() != mutations.len() || observed != required_mutation_roles(kind) {
        return Err(EventCustodyCandidateError::IndexMutationShape);
    }
    if mutations.iter().any(|mutation| {
        predecessor(&mutation.expected).is_some_and(|record| {
            !query_accepts_schema(&mutation.query, record.schema_id().as_str())
        })
    }) {
        return Err(EventCustodyCandidateError::IndexPredecessorSchema);
    }
    if mutations.iter().any(|mutation| {
        !mutation_predecessor_shape_is_exact(kind, mutation)
            || predecessor(&mutation.expected)
                .is_some_and(|record| record.coordinate() == &mutation.successor)
    }) {
        return Err(EventCustodyCandidateError::IndexPredecessorShape);
    }
    if mutations.iter().any(|mutation| {
        !members.iter().any(|member| {
            member.record().coordinate() == &mutation.successor
                && query_accepts_member(&mutation.query, member.kind())
        })
    }) {
        return Err(EventCustodyCandidateError::IndexSuccessorClosure);
    }
    Ok(())
}

fn validate_assertion_shape(
    kind: EventCustodyTransitionKind,
    assertions: &[EventIndexAssertion],
) -> Result<(), EventCustodyCandidateError> {
    let mut observed = assertions
        .iter()
        .map(|assertion| assertion.query.role())
        .collect::<Vec<_>>();
    observed.sort_unstable();
    if observed != required_assertion_roles(kind) {
        return Err(EventCustodyCandidateError::IndexAssertionShape);
    }
    let queries = assertions
        .iter()
        .map(|assertion| &assertion.query)
        .collect::<BTreeSet<_>>();
    if queries.len() != assertions.len() {
        return Err(EventCustodyCandidateError::IndexAssertionShape);
    }
    if assertions.iter().any(|assertion| {
        predecessor(&assertion.expected).is_some_and(|record| {
            !query_accepts_schema(&assertion.query, record.schema_id().as_str())
        })
    }) {
        return Err(EventCustodyCandidateError::IndexPredecessorSchema);
    }
    if assertions
        .iter()
        .any(|assertion| !matches!(&assertion.expected, EventIndexPredecessor::Exact(_)))
    {
        return Err(EventCustodyCandidateError::IndexPredecessorShape);
    }
    Ok(())
}

fn mutation_predecessor_shape_is_exact(
    kind: EventCustodyTransitionKind,
    mutation: &EventIndexMutation,
) -> bool {
    use EventCustodyTransitionKind as T;
    use EventIndexPredecessor as P;
    use EventIndexRole as I;

    matches!(
        (kind, mutation.query.role(), &mutation.expected),
        (
            T::AdmitOwnerReference,
            I::EventIdentity | I::OwnerSource | I::EventCommitWitness,
            P::Absent
        ) | (
            T::QuarantineIdentityConflict,
            I::IdentityConflict,
            P::Absent
        ) | (T::AdmitSubscription, I::Subscription, P::Absent)
            | (T::RetireSubscription, I::Subscription, P::Exact(_))
            | (
                T::EnqueueDelivery,
                I::InboxDelivery | I::DeliveryAttempt,
                P::Absent
            )
            | (
                T::EnqueueDelivery,
                I::DeliveryStream,
                P::Absent | P::Exact(_)
            )
            | (T::EnqueueReplay, I::InboxDelivery, P::Exact(_))
            | (T::EnqueueReplay, I::DeliveryAttempt, P::Absent)
            | (T::EnqueueReplay, I::DeliveryStream, P::Exact(_))
            | (
                T::EnqueueDelivery | T::EnqueueReplay,
                I::TransitionWitness,
                P::Absent
            )
            | (T::RecordAcknowledgement, I::Acknowledgement, P::Absent)
            | (T::RecordAcknowledgement, I::InboxDelivery, P::Exact(_))
            | (T::RecordAcknowledgement, I::TransitionWitness, P::Absent)
            | (
                T::AdvanceDeliveryCursor,
                I::DeliveryCursor,
                P::Absent | P::Exact(_)
            )
            | (T::AdvanceDeliveryCursor, I::TransitionWitness, P::Absent)
            | (
                T::AdvanceSourceFrontier,
                I::SourceFrontier,
                P::Absent | P::Exact(_)
            )
            | (
                T::ProjectInteractionOccurrence,
                I::InteractionOccurrence,
                P::Absent
            )
            | (T::ProjectGenealogyEdge, I::GenealogyEdge, P::Absent)
            | (
                T::AdvanceReconciliationFrontier,
                I::ReconciliationFrontier,
                P::Absent | P::Exact(_)
            )
            | (
                T::ProjectInteractionOccurrence
                    | T::ProjectGenealogyEdge
                    | T::AdvanceReconciliationFrontier,
                I::PaleDerivedOutbox,
                P::Absent
            )
            | (
                T::SettlePaleDerivedPublication,
                I::PaleDerivedOutbox,
                P::Exact(_)
            )
    )
}

fn predecessor(value: &EventIndexPredecessor) -> Option<&ControlRecord> {
    match value {
        EventIndexPredecessor::Absent => None,
        EventIndexPredecessor::Exact(record) => Some(record),
    }
}

fn query_accepts_schema(query: &EventIndexQuery, schema: &str) -> bool {
    schema == query.role().required_schema_id()
}

fn query_accepts_member(query: &EventIndexQuery, member: EventTransitionMemberKind) -> bool {
    matches!(
        (query, member),
        (
            EventIndexQuery::EventIdentity(_),
            EventTransitionMemberKind::EventReference
        ) | (
            EventIndexQuery::EventCommitWitness(_),
            EventTransitionMemberKind::EventCommitWitness
        ) | (
            EventIndexQuery::TransitionWitness(_),
            EventTransitionMemberKind::TransitionWitness
        ) | (
            EventIndexQuery::OwnerSource(_),
            EventTransitionMemberKind::EventReference
        ) | (
            EventIndexQuery::IdentityConflict(_),
            EventTransitionMemberKind::IdentityConflict
        ) | (
            EventIndexQuery::Subscription(_),
            EventTransitionMemberKind::Subscription
        ) | (
            EventIndexQuery::InboxDelivery(_),
            EventTransitionMemberKind::InboxDelivery
        ) | (
            EventIndexQuery::DeliveryAttempt(_),
            EventTransitionMemberKind::DeliveryAttempt
        ) | (
            EventIndexQuery::DeliveryStream(_),
            EventTransitionMemberKind::DeliveryStream
        ) | (
            EventIndexQuery::Acknowledgement(_),
            EventTransitionMemberKind::EventAcknowledgement
        ) | (
            EventIndexQuery::DeliveryCursor(_),
            EventTransitionMemberKind::DeliveryCursor
        ) | (
            EventIndexQuery::SourceFrontier(_),
            EventTransitionMemberKind::SourceFrontier
        ) | (
            EventIndexQuery::InteractionOccurrence(_),
            EventTransitionMemberKind::InteractionOccurrence
        ) | (
            EventIndexQuery::GenealogyEdge(_),
            EventTransitionMemberKind::GenealogyEdge
        ) | (
            EventIndexQuery::ReconciliationFrontier(_),
            EventTransitionMemberKind::ReconciliationFrontier
        ) | (
            EventIndexQuery::PaleDerivedOutbox(_),
            EventTransitionMemberKind::PaleDerivedOutbox
        )
    )
}

fn required_member_kinds(kind: EventCustodyTransitionKind) -> BTreeSet<EventTransitionMemberKind> {
    use EventCustodyTransitionKind as T;
    use EventTransitionMemberKind as M;

    let members: &[M] = match kind {
        T::AdmitOwnerReference => &[M::EventReference, M::EventCommitWitness],
        T::QuarantineIdentityConflict => &[M::IdentityConflict],
        T::AdmitSubscription | T::RetireSubscription => &[M::Subscription],
        T::EnqueueDelivery | T::EnqueueReplay => &[
            M::InboxDelivery,
            M::DeliveryAttempt,
            M::DeliveryStream,
            M::TransitionWitness,
        ],
        T::RecordAcknowledgement => &[
            M::EventAcknowledgement,
            M::InboxDelivery,
            M::TransitionWitness,
        ],
        T::AdvanceDeliveryCursor => &[M::DeliveryCursor, M::TransitionWitness],
        T::AdvanceSourceFrontier => &[M::SourceFrontier],
        T::ProjectInteractionOccurrence => &[M::InteractionOccurrence, M::PaleDerivedOutbox],
        T::ProjectGenealogyEdge => &[M::GenealogyEdge, M::PaleDerivedOutbox],
        T::AdvanceReconciliationFrontier => &[M::ReconciliationFrontier, M::PaleDerivedOutbox],
        T::SettlePaleDerivedPublication => &[M::PaleDerivedOutbox],
    };
    members.iter().copied().collect()
}

fn required_mutation_roles(kind: EventCustodyTransitionKind) -> BTreeSet<EventIndexRole> {
    use EventCustodyTransitionKind as T;
    use EventIndexRole as I;

    let roles: &[I] = match kind {
        T::AdmitOwnerReference => &[I::EventIdentity, I::OwnerSource, I::EventCommitWitness],
        T::QuarantineIdentityConflict => &[I::IdentityConflict],
        T::AdmitSubscription | T::RetireSubscription => &[I::Subscription],
        T::EnqueueDelivery | T::EnqueueReplay => &[
            I::InboxDelivery,
            I::DeliveryAttempt,
            I::DeliveryStream,
            I::TransitionWitness,
        ],
        T::RecordAcknowledgement => &[I::Acknowledgement, I::InboxDelivery, I::TransitionWitness],
        T::AdvanceDeliveryCursor => &[I::DeliveryCursor, I::TransitionWitness],
        T::AdvanceSourceFrontier => &[I::SourceFrontier],
        T::ProjectInteractionOccurrence => &[I::InteractionOccurrence, I::PaleDerivedOutbox],
        T::ProjectGenealogyEdge => &[I::GenealogyEdge, I::PaleDerivedOutbox],
        T::AdvanceReconciliationFrontier => &[I::ReconciliationFrontier, I::PaleDerivedOutbox],
        T::SettlePaleDerivedPublication => &[I::PaleDerivedOutbox],
    };
    roles.iter().copied().collect()
}

fn required_assertion_roles(kind: EventCustodyTransitionKind) -> Vec<EventIndexRole> {
    use EventCustodyTransitionKind as T;
    use EventIndexRole as I;

    let mut roles = match kind {
        T::AdmitOwnerReference
        | T::AdmitSubscription
        | T::RetireSubscription
        | T::AdvanceSourceFrontier
        | T::AdvanceReconciliationFrontier
        | T::SettlePaleDerivedPublication => Vec::new(),
        T::QuarantineIdentityConflict => vec![I::EventIdentity],
        T::EnqueueDelivery | T::EnqueueReplay => vec![I::EventIdentity, I::Subscription],
        T::RecordAcknowledgement => vec![I::DeliveryAttempt],
        T::AdvanceDeliveryCursor => vec![I::Acknowledgement],
        T::ProjectInteractionOccurrence => vec![I::EventIdentity],
        T::ProjectGenealogyEdge => vec![I::InteractionOccurrence, I::InteractionOccurrence],
    };
    roles.sort_unstable();
    roles
}

#[cfg(test)]
#[path = "../tests/internal/event.rs"]
mod tests;
