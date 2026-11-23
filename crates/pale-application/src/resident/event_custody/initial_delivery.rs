//! Deterministic first delivery custody for one admitted Event subscription.
//!
//! This module retains a private pending attempt and can join one signed
//! destination acknowledgement to it. It advances only the contiguous
//! delivery cursor. It does not send a carrier or settle an outbox record.

use pale_core::{canon, records::*};
use pale_ports::{
    AuthorityStoreCoordinate, ControlRecord, ControlRecordCoordinate, ControlRecordDigest,
    ControlStoreError, ControlTransactionPort, EventCustodyHistoryMember, EventCustodyPort,
    EventCustodyTransitionKind, EventInboxCursor, EventInboxPageBoundary, EventInboxPageRead,
    EventIndexQuery, EventTransitionMemberKind,
};
use serde::{Deserialize, Serialize};

use super::projection::{
    acknowledgement_key, delivery_attempt_key, delivery_cursor_key, delivery_stream_key,
    event_commit_witness_key, event_identity, event_identity_key, inbox_key, private_record,
    public_record, subscription_key, transition_witness_key,
};
use super::record::{
    DeliveryAttemptKind, EventCommitWitness, EventCustodyTransitionWitness, EventDeliveryStream,
};
use super::{
    plan_event_fanout, AdmittedEventRecord, AdmittedEventSubscription, EventCustodyCommitEvidence,
    EventCustodyContinuation, EventCustodyContinuationKind, EventCustodyProjectionError,
    EventCustodyTransitionInput, EventDeliveryAttempt, EventDeliveryCursor,
    EventDeliveryDisposition, EventDeliveryOrdinal, EventDeliveryState,
};
use crate::resident::{ResidentApplication, ResidentApplicationError, ResidentCommitReceipt};

const INITIAL_DELIVERY_IDENTITY_DOMAIN: &[u8] = b"PALE_INITIAL_EVENT_DELIVERY_IDENTITY_V1\0";
const DELIVERY_STREAM_SUCCESSOR_DOMAIN: &[u8] = b"PALE_EVENT_DELIVERY_STREAM_SUCCESSOR_V1\0";
const DELIVERY_CURSOR_SUCCESSOR_DOMAIN: &[u8] = b"PALE_EVENT_DELIVERY_CURSOR_SUCCESSOR_V1\0";

#[derive(Serialize)]
struct InitialDeliveryIdentity<'a> {
    initial_v1_domain: &'static str,
    event_identity: &'a CanonicalEventIdentityV1,
    event_sha256: &'a EventReferenceDigest,
    subscription_id: &'a EventSubscriptionIdV1,
}

#[derive(Serialize)]
struct DeliveryStreamSuccessorIdentity<'a> {
    initial_v1_domain: &'static str,
    subscription_id: &'a EventSubscriptionIdV1,
    ordinal: EventDeliveryOrdinal,
    attempt_id: &'a DeliveryAttemptIdV1,
}

#[derive(Serialize)]
struct DeliveryCursorSuccessorIdentity<'a> {
    initial_v1_domain: &'static str,
    subscription_id: &'a EventSubscriptionIdV1,
    through_ordinal: EventDeliveryOrdinal,
    acknowledgement_sha256: &'a EventAcknowledgementDigest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EventCommitWitnessWire {
    store_id: ControlStoreIdV1,
    store_incarnation_id: ControlIncarnationIdV1,
    event_ref: ResidentRecordRefV1,
    event_sha256: EventReferenceDigest,
    event_identity: CanonicalEventIdentityV1,
    committed_at: JournalSequenceV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EventDeliveryStreamWire {
    subscription_id: EventSubscriptionIdV1,
    through_ordinal: u64,
    last_attempt_id: DeliveryAttemptIdV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EventDeliveryStateWire {
    subscription_id: EventSubscriptionIdV1,
    event_identity: CanonicalEventIdentityV1,
    event_sha256: EventReferenceDigest,
    attempt_id: DeliveryAttemptIdV1,
    ordinal: u64,
    disposition: EventDeliveryDisposition,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EventDeliveryAttemptWire {
    attempt_id: DeliveryAttemptIdV1,
    subscription_id: EventSubscriptionIdV1,
    event_identity: CanonicalEventIdentityV1,
    event_sha256: EventReferenceDigest,
    ordinal: u64,
    kind: DeliveryAttemptKind,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EventDeliveryCursorWire {
    subscription_id: EventSubscriptionIdV1,
    through_ordinal: u64,
    acknowledgement_ref: ResidentRecordRefV1,
    acknowledgement_sha256: EventAcknowledgementDigest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EventCustodyTransitionWitnessWire {
    store_id: ControlStoreIdV1,
    store_incarnation_id: ControlIncarnationIdV1,
    transition_kind: EventCustodyTransitionKind,
    subject_kind: EventTransitionMemberKind,
    subject: ControlRecordCoordinate,
    committed_at: JournalSequenceV1,
}

/// Deterministic identity inputs for one first delivery attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitialEventDeliveryPlan {
    event_ref: ResidentRecordRefV1,
    event: EventReferenceV1,
    subscription: AdmittedEventSubscription,
    attempt_id: DeliveryAttemptIdV1,
    attempt_ref: ResidentRecordRefV1,
    inbox_ref: ResidentRecordRefV1,
}

impl InitialEventDeliveryPlan {
    /// Bind one exact Event and active subscription.
    ///
    /// Event commit time is recovered from custody. The caller cannot claim it.
    ///
    /// # Errors
    ///
    /// Returns an error when canonical identity encoding fails or the derived
    /// private record and attempt identifiers are invalid.
    pub fn admit(
        event_ref: ResidentRecordRefV1,
        event: EventReferenceV1,
        subscription: AdmittedEventSubscription,
    ) -> Result<Self, InitialEventDeliveryError> {
        let identity = event_identity(&event);
        let canonical = canon::to_canonical_string(&InitialDeliveryIdentity {
            initial_v1_domain: "initial_event_delivery",
            event_identity: &identity,
            event_sha256: event.canonical_event_sha256(),
            subscription_id: subscription.subscription_id(),
        })?;
        let mut bytes =
            Vec::with_capacity(INITIAL_DELIVERY_IDENTITY_DOMAIN.len() + canonical.len());
        bytes.extend_from_slice(INITIAL_DELIVERY_IDENTITY_DOMAIN);
        bytes.extend_from_slice(canonical.as_bytes());
        let digest = pale_core::sha256_hex(&bytes);
        Ok(Self {
            event_ref,
            event,
            subscription,
            attempt_id: DeliveryAttemptIdV1::parse(digest.clone())
                .map_err(|_| InitialEventDeliveryError::PrivateIdentity)?,
            attempt_ref: ResidentRecordRefV1::parse(format!(
                "initial-event-delivery-attempt-{digest}"
            ))
            .map_err(|_| InitialEventDeliveryError::PrivateIdentity)?,
            inbox_ref: ResidentRecordRefV1::parse(format!("initial-event-delivery-inbox-{digest}"))
                .map_err(|_| InitialEventDeliveryError::PrivateIdentity)?,
        })
    }

    /// Borrow the deterministic identity of this initial delivery attempt.
    #[must_use]
    pub const fn attempt_id(&self) -> &DeliveryAttemptIdV1 {
        &self.attempt_id
    }

    fn event(&self) -> AdmittedEventRecord<'_, EventReferenceV1> {
        AdmittedEventRecord::new(&self.event_ref, &self.event)
    }

    fn attempt(&self, ordinal: EventDeliveryOrdinal) -> EventDeliveryAttempt {
        EventDeliveryAttempt::initial(
            self.attempt_ref.clone(),
            self.attempt_id.clone(),
            self.subscription.subscription_id().clone(),
            event_identity(&self.event),
            self.event.canonical_event_sha256().clone(),
            ordinal,
        )
    }

    fn delivery(&self, ordinal: EventDeliveryOrdinal) -> EventDeliveryState {
        EventDeliveryState::new(
            self.inbox_ref.clone(),
            self.subscription.subscription_id().clone(),
            event_identity(&self.event),
            self.event.canonical_event_sha256().clone(),
            self.attempt_id.clone(),
            ordinal,
            EventDeliveryDisposition::Pending,
        )
    }
}

/// Positive limits for one restart-recovery pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InitialEventDeliveryRecoveryLimits {
    maximum_pages: usize,
    maximum_items_per_page: usize,
    maximum_private_record_bytes: usize,
    maximum_page_record_bytes: usize,
}

impl InitialEventDeliveryRecoveryLimits {
    /// Construct explicit page, row, and aggregate retained-body bounds.
    ///
    /// # Errors
    ///
    /// Returns [`InitialEventDeliveryError::ZeroBound`] when any bound is zero.
    pub const fn new(
        maximum_pages: usize,
        maximum_items_per_page: usize,
        maximum_private_record_bytes: usize,
        maximum_page_record_bytes: usize,
    ) -> Result<Self, InitialEventDeliveryError> {
        if maximum_pages == 0
            || maximum_items_per_page == 0
            || maximum_private_record_bytes == 0
            || maximum_page_record_bytes == 0
        {
            Err(InitialEventDeliveryError::ZeroBound)
        } else {
            Ok(Self {
                maximum_pages,
                maximum_items_per_page,
                maximum_private_record_bytes,
                maximum_page_record_bytes,
            })
        }
    }
}

/// Strict result of reading the deterministic private attempt after restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitialEventDeliveryRecovery {
    /// No inbox state or immutable attempt exists.
    Absent,
    /// Both private records retain the same pending ordinal.
    Pending {
        /// Allocated position in the subscription delivery stream.
        ordinal: EventDeliveryOrdinal,
    },
}

/// Result of advancing one deterministic first delivery by at most one commit.
#[derive(Debug)]
pub enum InitialEventDeliveryAdvance {
    /// The exact pending attempt already existed; no continuation was minted.
    Existing {
        /// Previously allocated position in the subscription delivery stream.
        ordinal: EventDeliveryOrdinal,
    },
    /// This call established custody and earned the sole delivery continuation.
    Established {
        /// Allocated position in the subscription delivery stream.
        ordinal: EventDeliveryOrdinal,
        /// How exact custody was established.
        evidence: EventCustodyCommitEvidence,
        /// One-use permission to attempt delivery outside this application step.
        continuation: EventCustodyContinuation,
        /// Resident journal receipt for the durable enqueue.
        receipt: Box<ResidentCommitReceipt>,
    },
}

/// Destination authority required to admit one signed delivery acknowledgement.
pub struct InitialEventAcknowledgementAuthority<'a> {
    /// Exact admitted node that signed the acknowledgement.
    pub acknowledging_node_id: &'a ResidentNodeIdV1,
    /// Active principal-key epoch covering that node's delegation.
    pub principal_key_epoch: PrincipalKeyEpochV1,
    /// Exact active node-delegation reference.
    pub node_delegation_ref: &'a ResidentRecordRefV1,
    /// Digest of the exact node-delegation bytes.
    pub node_delegation_sha256: &'a ArtifactDigest,
}

/// Fresh records and signed wire for one exact delivery acknowledgement.
pub struct InitialEventAcknowledgementInput {
    /// Immutable reference under which the public acknowledgement is retained.
    pub acknowledgement_ref: ResidentRecordRefV1,
    /// Fresh immutable reference for the acknowledged inbox state.
    pub delivery_state_ref: ResidentRecordRefV1,
    /// Untrusted signed acknowledgement from the selected destination.
    pub acknowledgement: EventAcknowledgementWireV1,
}

/// Admitted acknowledgement joined to its exact retained delivery attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordedEventAcknowledgement {
    attempt: EventDeliveryAttempt,
    acknowledgement_ref: ResidentRecordRefV1,
    acknowledgement: EventAcknowledgementV1,
    delivery: EventDeliveryState,
}

impl RecordedEventAcknowledgement {
    /// Borrow the immutable delivery attempt that was acknowledged.
    #[must_use]
    pub const fn attempt(&self) -> &EventDeliveryAttempt {
        &self.attempt
    }

    /// Borrow the public acknowledgement and its immutable record reference.
    #[must_use]
    pub const fn acknowledgement(&self) -> (&ResidentRecordRefV1, &EventAcknowledgementV1) {
        (&self.acknowledgement_ref, &self.acknowledgement)
    }

    /// Borrow the acknowledged inbox state.
    #[must_use]
    pub const fn delivery(&self) -> &EventDeliveryState {
        &self.delivery
    }
}

/// Result of retaining one destination acknowledgement.
pub enum InitialEventAcknowledgementAdvance {
    /// The exact acknowledgement and inbox successor were already retained.
    Existing(RecordedEventAcknowledgement),
    /// This call retained the acknowledgement and inbox successor atomically.
    Established {
        /// Admitted records joined to the exact delivery attempt.
        recorded: RecordedEventAcknowledgement,
        /// How exact custody was established.
        evidence: EventCustodyCommitEvidence,
        /// Resident journal receipt for the durable acknowledgement.
        receipt: Box<ResidentCommitReceipt>,
    },
}

/// Result of advancing the contiguous delivery cursor for one subscription.
pub enum InitialEventDeliveryCursorAdvance {
    /// The exact cursor successor already exists.
    Existing(EventDeliveryCursor),
    /// The cursor already covers a later terminal delivery.
    AlreadyBeyond {
        /// Current contiguous cursor.
        cursor: EventDeliveryCursor,
        /// Terminal delivery supplied by this call.
        acknowledged_ordinal: EventDeliveryOrdinal,
    },
    /// One or more earlier terminal deliveries are still absent.
    WaitingForEarlier {
        /// Current contiguous cursor, or `None` before ordinal one.
        current_through: Option<EventDeliveryOrdinal>,
        /// Terminal delivery that cannot yet advance the cursor.
        acknowledged_ordinal: EventDeliveryOrdinal,
    },
    /// This call atomically retained the next contiguous cursor.
    Established {
        /// Newly retained cursor.
        cursor: EventDeliveryCursor,
        /// How exact custody was established.
        evidence: EventCustodyCommitEvidence,
        /// Resident journal receipt for the cursor advance.
        receipt: Box<ResidentCommitReceipt>,
    },
}

/// Failure to plan, recover, or retain one first delivery attempt.
#[derive(Debug, thiserror::Error)]
pub enum InitialEventDeliveryError {
    /// The custody-proved Event commit predates the subscription frontier.
    #[error("subscription does not select the Event at its exact commit frontier")]
    SubscriptionDoesNotSelectEvent,
    /// A domain-separated private identity violates its grammar.
    #[error("deterministic initial-delivery identity is invalid")]
    PrivateIdentity,
    /// A page, row, or aggregate byte bound is zero.
    #[error("initial-delivery recovery bound is zero")]
    ZeroBound,
    /// No resident journal frontier exists for recovery.
    #[error("resident has no journal frontier for an admitted Event")]
    MissingJournalFrontier,
    /// Stable pagination did not terminate inside the page budget.
    #[error("initial-delivery inbox recovery exceeded its page bound")]
    PageBoundExceeded,
    /// Retained state is partial, unlike, or not exactly admitted.
    #[error("stored initial-delivery state conflicts with exact admitted custody")]
    RetainedStateConflict,
    /// The signed acknowledgement failed its contract or destination admission.
    #[error(transparent)]
    Acknowledgement(#[from] InvalidResidentRecordV1),
    /// Canonical private identity encoding failed.
    #[error(transparent)]
    Canonicalize(#[from] canon::CanonError),
    /// Semantic-to-physical Event projection failed.
    #[error(transparent)]
    Projection(#[from] EventCustodyProjectionError),
    /// A bounded physical read failed.
    #[error(transparent)]
    Store(#[from] ControlStoreError),
    /// Durable resident commitment failed.
    #[error(transparent)]
    Resident(#[from] ResidentApplicationError),
}

fn exact_index_record(
    port: &mut dyn EventCustodyPort,
    store: &AuthorityStoreCoordinate,
    query: EventIndexQuery,
    expected: &ControlRecord,
    maximum_body_bytes: usize,
) -> Result<bool, InitialEventDeliveryError> {
    let observed = port.read_index(store, &query, maximum_body_bytes)?;
    if observed.query() != &query {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    match observed.record() {
        None => Ok(false),
        Some(record) if record == expected => Ok(true),
        Some(_) => Err(InitialEventDeliveryError::RetainedStateConflict),
    }
}

fn require_historical_record(
    port: &mut dyn EventCustodyPort,
    store: &AuthorityStoreCoordinate,
    expected: &ControlRecord,
    maximum_body_bytes: usize,
) -> Result<(), InitialEventDeliveryError> {
    let observed = port.read_record(store, expected.coordinate(), maximum_body_bytes)?;
    if observed.coordinate() != expected.coordinate() || observed.record() != Some(expected) {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    Ok(())
}

fn verify_transition_members(
    application: &ResidentApplication<'_>,
    port: &mut dyn EventCustodyPort,
    transition_kind: EventCustodyTransitionKind,
    subject_kind: EventTransitionMemberKind,
    subject: &ControlRecord,
    members: &[EventCustodyHistoryMember],
    maximum_body_bytes: usize,
) -> Result<JournalSequenceV1, InitialEventDeliveryError> {
    let key = transition_witness_key(transition_kind, subject_kind, subject.coordinate())?;
    let query = EventIndexQuery::TransitionWitness(key.clone());
    let observed = port.read_index(application.store(), &query, maximum_body_bytes)?;
    let record = observed
        .record()
        .filter(|_| observed.query() == &query)
        .ok_or(InitialEventDeliveryError::RetainedStateConflict)?;
    let wire: EventCustodyTransitionWitnessWire = decode(record)?;
    if wire.store_id != *application.store().store_id()
        || wire.store_incarnation_id != *application.store().incarnation_id()
        || wire.transition_kind != transition_kind
        || wire.subject_kind != subject_kind
        || wire.subject != *subject.coordinate()
    {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    let witness = EventCustodyTransitionWitness {
        record_ref: ResidentRecordRefV1::parse(format!(
            "event-transition-witness-{}",
            key.as_str()
        ))
        .map_err(|_| InitialEventDeliveryError::PrivateIdentity)?,
        store_id: wire.store_id,
        store_incarnation_id: wire.store_incarnation_id,
        transition_kind: wire.transition_kind,
        subject_kind: wire.subject_kind,
        subject: wire.subject,
        committed_at: wire.committed_at,
    };
    let expected_witness = private_record(
        &witness.record_ref,
        EventTransitionMemberKind::TransitionWitness,
        &witness,
    )?;
    if record != &expected_witness {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    let mut required = Vec::with_capacity(members.len() + 1);
    required.extend_from_slice(members);
    required.push(EventCustodyHistoryMember::new(
        EventTransitionMemberKind::TransitionWitness,
        expected_witness.coordinate().clone(),
    ));
    application.verify_event_custody_members_at(
        port,
        witness.committed_at,
        &required,
        maximum_body_bytes,
    )?;
    Ok(witness.committed_at)
}

fn decode<T: for<'de> Deserialize<'de>>(
    record: &ControlRecord,
) -> Result<T, InitialEventDeliveryError> {
    serde_json::from_slice(record.canonical_record_jcs().as_slice())
        .map_err(|_| InitialEventDeliveryError::RetainedStateConflict)
}

fn decode_delivery_cursor(
    record: &ControlRecord,
) -> Result<EventDeliveryCursor, InitialEventDeliveryError> {
    let wire: EventDeliveryCursorWire = decode(record)?;
    let through_ordinal = EventDeliveryOrdinal::new(wire.through_ordinal)
        .map_err(|_| InitialEventDeliveryError::RetainedStateConflict)?;
    let record_ref = delivery_cursor_record_ref(
        &wire.subscription_id,
        through_ordinal,
        &wire.acknowledgement_sha256,
    )?;
    let cursor = EventDeliveryCursor::new(
        record_ref,
        wire.subscription_id,
        through_ordinal,
        wire.acknowledgement_ref,
        wire.acknowledgement_sha256,
    );
    let expected = private_record(
        record.coordinate().record_ref(),
        EventTransitionMemberKind::DeliveryCursor,
        &cursor,
    )?;
    if &expected != record {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    Ok(cursor)
}

fn delivery_cursor_record_ref(
    subscription_id: &EventSubscriptionIdV1,
    through_ordinal: EventDeliveryOrdinal,
    acknowledgement_sha256: &EventAcknowledgementDigest,
) -> Result<ResidentRecordRefV1, InitialEventDeliveryError> {
    let canonical = canon::to_canonical_string(&DeliveryCursorSuccessorIdentity {
        initial_v1_domain: "event_delivery_cursor_successor",
        subscription_id,
        through_ordinal,
        acknowledgement_sha256,
    })?;
    let mut bytes = Vec::with_capacity(DELIVERY_CURSOR_SUCCESSOR_DOMAIN.len() + canonical.len());
    bytes.extend_from_slice(DELIVERY_CURSOR_SUCCESSOR_DOMAIN);
    bytes.extend_from_slice(canonical.as_bytes());
    ResidentRecordRefV1::parse(format!(
        "event-delivery-cursor-{}",
        pale_core::sha256_hex(&bytes)
    ))
    .map_err(|_| InitialEventDeliveryError::PrivateIdentity)
}

fn delivery_cursor_successor(
    recorded: &RecordedEventAcknowledgement,
) -> Result<EventDeliveryCursor, InitialEventDeliveryError> {
    let acknowledgement_sha256 = recorded.acknowledgement.canonical_acknowledgement_sha256();
    let record_ref = delivery_cursor_record_ref(
        &recorded.attempt.subscription_id,
        recorded.attempt.ordinal,
        acknowledgement_sha256,
    )?;
    Ok(EventDeliveryCursor::new(
        record_ref,
        recorded.attempt.subscription_id.clone(),
        recorded.attempt.ordinal,
        recorded.acknowledgement_ref.clone(),
        acknowledgement_sha256.clone(),
    ))
}

fn read_event_commit_sequence(
    application: &ResidentApplication<'_>,
    port: &mut dyn EventCustodyPort,
    plan: &InitialEventDeliveryPlan,
    maximum_body_bytes: usize,
) -> Result<JournalSequenceV1, InitialEventDeliveryError> {
    let store = application.store();
    let expected_event = public_record(plan.event(), plan.event.canonical_event_sha256().as_str())?;
    let event_query = EventIndexQuery::EventIdentity(event_identity_key(&plan.event)?);
    let observed = port.read_index(store, &event_query, maximum_body_bytes)?;
    if observed.query() != &event_query || observed.record() != Some(&expected_event) {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    let witness_key = event_commit_witness_key(&plan.event)?;
    let witness_query = EventIndexQuery::EventCommitWitness(witness_key.clone());
    let observed = port.read_index(store, &witness_query, maximum_body_bytes)?;
    let record = observed
        .record()
        .filter(|_| observed.query() == &witness_query)
        .ok_or(InitialEventDeliveryError::RetainedStateConflict)?;
    let witness: EventCommitWitnessWire = decode(record)?;
    if witness.store_id != *store.store_id()
        || witness.store_incarnation_id != *store.incarnation_id()
        || witness.event_ref != plan.event_ref
        || witness.event_sha256 != *plan.event.canonical_event_sha256()
        || witness.event_identity != event_identity(&plan.event)
    {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    let witness_ref =
        ResidentRecordRefV1::parse(format!("event-commit-witness-{}", witness_key.as_str()))
            .map_err(|_| InitialEventDeliveryError::PrivateIdentity)?;
    let expected_witness = EventCommitWitness {
        record_ref: witness_ref,
        store_id: witness.store_id.clone(),
        store_incarnation_id: witness.store_incarnation_id.clone(),
        event_ref: witness.event_ref.clone(),
        event_sha256: witness.event_sha256.clone(),
        event_identity: witness.event_identity.clone(),
        committed_at: witness.committed_at,
    };
    let expected_witness_record = private_record(
        &expected_witness.record_ref,
        EventTransitionMemberKind::EventCommitWitness,
        &expected_witness,
    )?;
    if record != &expected_witness_record {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    let required = [
        EventCustodyHistoryMember::new(
            EventTransitionMemberKind::EventReference,
            expected_event.coordinate().clone(),
        ),
        EventCustodyHistoryMember::new(
            EventTransitionMemberKind::EventCommitWitness,
            expected_witness_record.coordinate().clone(),
        ),
    ];
    application.verify_event_custody_members_at(
        port,
        witness.committed_at,
        &required,
        maximum_body_bytes,
    )?;
    if plan_event_fanout(
        &plan.event,
        witness.committed_at,
        std::slice::from_ref(&plan.subscription),
    )
    .is_empty()
    {
        return Err(InitialEventDeliveryError::SubscriptionDoesNotSelectEvent);
    }
    Ok(witness.committed_at)
}

fn expected_subscription_record(
    plan: &InitialEventDeliveryPlan,
) -> Result<ControlRecord, EventCustodyProjectionError> {
    private_record(
        plan.subscription.record_ref(),
        EventTransitionMemberKind::Subscription,
        &plan.subscription,
    )
}

fn recover_subscription_evidence(
    port: &mut dyn EventCustodyPort,
    store: &AuthorityStoreCoordinate,
    plan: &InitialEventDeliveryPlan,
    maximum_body_bytes: usize,
) -> Result<(), InitialEventDeliveryError> {
    let expected = expected_subscription_record(plan)?;
    let observed = port.read_record(store, expected.coordinate(), maximum_body_bytes)?;
    if observed.coordinate() != expected.coordinate() || observed.record() != Some(&expected) {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    Ok(())
}

fn recover_pending(
    application: &ResidentApplication<'_>,
    port: &mut dyn EventCustodyPort,
    store: &AuthorityStoreCoordinate,
    through: JournalSequenceV1,
    plan: &InitialEventDeliveryPlan,
    limits: InitialEventDeliveryRecoveryLimits,
) -> Result<InitialEventDeliveryRecovery, InitialEventDeliveryError> {
    let expected_inbox_key = inbox_key(
        plan.subscription.subscription_id(),
        &event_identity(&plan.event),
    )?;
    let mut after: Option<EventInboxCursor> = None;
    let mut found: Option<(ControlRecord, EventDeliveryStateWire)> = None;
    let mut complete = false;
    for _ in 0..limits.maximum_pages {
        let request = EventInboxPageRead::new(
            store.clone(),
            subscription_key(plan.subscription.subscription_id())?,
            after,
            through,
            limits.maximum_items_per_page,
            limits.maximum_private_record_bytes,
            limits.maximum_page_record_bytes,
        )
        .map_err(|_| InitialEventDeliveryError::RetainedStateConflict)?;
        let page = port.read_inbox_page(&request)?;
        for entry in page.entries() {
            if entry.cursor().delivery_key() == &expected_inbox_key {
                if found.is_some() {
                    return Err(InitialEventDeliveryError::RetainedStateConflict);
                }
                found = Some((entry.record().clone(), decode(entry.record())?));
            }
        }
        match page.boundary() {
            EventInboxPageBoundary::Complete => {
                complete = true;
                break;
            }
            EventInboxPageBoundary::More { resume_after } => after = Some(resume_after.clone()),
        }
    }
    if !complete {
        return Err(InitialEventDeliveryError::PageBoundExceeded);
    }

    let attempt_query = EventIndexQuery::DeliveryAttempt(delivery_attempt_key(&plan.attempt_id)?);
    let attempt_observation =
        port.read_index(store, &attempt_query, limits.maximum_private_record_bytes)?;
    if attempt_observation.query() != &attempt_query {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    match (found, attempt_observation.record()) {
        (None, None) => Ok(InitialEventDeliveryRecovery::Absent),
        (Some((delivery_record, state)), Some(attempt_record)) => {
            if state.subscription_id != *plan.subscription.subscription_id()
                || state.event_identity != event_identity(&plan.event)
                || state.event_sha256 != *plan.event.canonical_event_sha256()
                || state.attempt_id != plan.attempt_id
                || !matches!(state.disposition, EventDeliveryDisposition::Pending)
            {
                return Err(InitialEventDeliveryError::RetainedStateConflict);
            }
            let ordinal = EventDeliveryOrdinal::new(state.ordinal)
                .map_err(|_| InitialEventDeliveryError::RetainedStateConflict)?;
            let expected_delivery = private_record(
                &plan.inbox_ref,
                EventTransitionMemberKind::InboxDelivery,
                &plan.delivery(ordinal),
            )?;
            if delivery_record != expected_delivery {
                return Err(InitialEventDeliveryError::RetainedStateConflict);
            }
            let attempt: EventDeliveryAttemptWire = decode(attempt_record)?;
            if attempt.attempt_id != plan.attempt_id
                || attempt.subscription_id != *plan.subscription.subscription_id()
                || attempt.event_identity != event_identity(&plan.event)
                || attempt.event_sha256 != *plan.event.canonical_event_sha256()
                || attempt.ordinal != state.ordinal
                || attempt.kind != DeliveryAttemptKind::Initial
            {
                return Err(InitialEventDeliveryError::RetainedStateConflict);
            }
            let expected_attempt = private_record(
                &plan.attempt_ref,
                EventTransitionMemberKind::DeliveryAttempt,
                &plan.attempt(ordinal),
            )?;
            if attempt_record != &expected_attempt {
                return Err(InitialEventDeliveryError::RetainedStateConflict);
            }
            verify_initial_delivery_members(
                application,
                port,
                plan,
                ordinal,
                limits.maximum_private_record_bytes,
            )?;
            recover_subscription_evidence(port, store, plan, limits.maximum_private_record_bytes)?;
            let stream = read_stream(port, store, plan, limits.maximum_private_record_bytes)?
                .ok_or(InitialEventDeliveryError::RetainedStateConflict)?;
            if stream.through_ordinal < ordinal
                || (stream.through_ordinal == ordinal && stream.last_attempt_id != plan.attempt_id)
            {
                return Err(InitialEventDeliveryError::RetainedStateConflict);
            }
            Ok(InitialEventDeliveryRecovery::Pending { ordinal })
        }
        _ => Err(InitialEventDeliveryError::RetainedStateConflict),
    }
}

/// Strictly recover one deterministic private pending attempt.
///
/// # Errors
///
/// Returns an error when custody is unavailable, corrupt, partial, outside the
/// fixed journal frontier, or inconsistent with the admitted Event and
/// subscription.
pub fn recover_initial_event_delivery(
    application: &ResidentApplication<'_>,
    port: &mut dyn EventCustodyPort,
    plan: &InitialEventDeliveryPlan,
    limits: InitialEventDeliveryRecoveryLimits,
) -> Result<InitialEventDeliveryRecovery, InitialEventDeliveryError> {
    let store = application.store();
    let through = application
        .journal_head()
        .ok_or(InitialEventDeliveryError::MissingJournalFrontier)?;
    let committed_at =
        read_event_commit_sequence(application, port, plan, limits.maximum_private_record_bytes)?;
    if committed_at > through {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    recover_pending(application, port, store, through, plan, limits)
}

fn read_stream(
    port: &mut dyn EventCustodyPort,
    store: &AuthorityStoreCoordinate,
    plan: &InitialEventDeliveryPlan,
    maximum_body_bytes: usize,
) -> Result<Option<EventDeliveryStream>, InitialEventDeliveryError> {
    let query =
        EventIndexQuery::DeliveryStream(delivery_stream_key(plan.subscription.subscription_id())?);
    let observed = port.read_index(store, &query, maximum_body_bytes)?;
    if observed.query() != &query {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    let Some(record) = observed.record() else {
        return Ok(None);
    };
    let wire: EventDeliveryStreamWire = decode(record)?;
    if wire.subscription_id != *plan.subscription.subscription_id() {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    let through_ordinal = EventDeliveryOrdinal::new(wire.through_ordinal)
        .map_err(|_| InitialEventDeliveryError::RetainedStateConflict)?;
    let expected_ref = stream_record_ref(
        &wire.subscription_id,
        through_ordinal,
        &wire.last_attempt_id,
    )?;
    let stream = EventDeliveryStream::new(
        expected_ref,
        wire.subscription_id,
        through_ordinal,
        wire.last_attempt_id,
    );
    let expected = private_record(
        &stream.record_ref,
        EventTransitionMemberKind::DeliveryStream,
        &stream,
    )?;
    if record != &expected {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    Ok(Some(stream))
}

fn stream_record_ref(
    subscription_id: &EventSubscriptionIdV1,
    ordinal: EventDeliveryOrdinal,
    attempt_id: &DeliveryAttemptIdV1,
) -> Result<ResidentRecordRefV1, InitialEventDeliveryError> {
    let canonical = canon::to_canonical_string(&DeliveryStreamSuccessorIdentity {
        initial_v1_domain: "event_delivery_stream_successor",
        subscription_id,
        ordinal,
        attempt_id,
    })?;
    let mut bytes = Vec::with_capacity(DELIVERY_STREAM_SUCCESSOR_DOMAIN.len() + canonical.len());
    bytes.extend_from_slice(DELIVERY_STREAM_SUCCESSOR_DOMAIN);
    bytes.extend_from_slice(canonical.as_bytes());
    let digest = pale_core::sha256_hex(&bytes);
    ResidentRecordRefV1::parse(format!("event-delivery-stream-{digest}"))
        .map_err(|_| InitialEventDeliveryError::PrivateIdentity)
}

fn verify_initial_delivery_members(
    application: &ResidentApplication<'_>,
    port: &mut dyn EventCustodyPort,
    plan: &InitialEventDeliveryPlan,
    ordinal: EventDeliveryOrdinal,
    maximum_body_bytes: usize,
) -> Result<JournalSequenceV1, InitialEventDeliveryError> {
    let attempt = plan.attempt(ordinal);
    let attempt_record = private_record(
        &attempt.record_ref,
        EventTransitionMemberKind::DeliveryAttempt,
        &attempt,
    )?;
    let delivery = plan.delivery(ordinal);
    let delivery_record = private_record(
        &delivery.record_ref,
        EventTransitionMemberKind::InboxDelivery,
        &delivery,
    )?;
    let stream = EventDeliveryStream::new(
        stream_record_ref(
            plan.subscription.subscription_id(),
            ordinal,
            &plan.attempt_id,
        )?,
        plan.subscription.subscription_id().clone(),
        ordinal,
        plan.attempt_id.clone(),
    );
    let stream_record = private_record(
        &stream.record_ref,
        EventTransitionMemberKind::DeliveryStream,
        &stream,
    )?;
    verify_transition_members(
        application,
        port,
        EventCustodyTransitionKind::EnqueueDelivery,
        EventTransitionMemberKind::DeliveryAttempt,
        &attempt_record,
        &[
            EventCustodyHistoryMember::new(
                EventTransitionMemberKind::InboxDelivery,
                delivery_record.coordinate().clone(),
            ),
            EventCustodyHistoryMember::new(
                EventTransitionMemberKind::DeliveryAttempt,
                attempt_record.coordinate().clone(),
            ),
            EventCustodyHistoryMember::new(
                EventTransitionMemberKind::DeliveryStream,
                stream_record.coordinate().clone(),
            ),
        ],
        maximum_body_bytes,
    )
}

fn verify_recorded_initial_delivery_members(
    application: &ResidentApplication<'_>,
    port: &mut dyn EventCustodyPort,
    recorded: &RecordedEventAcknowledgement,
    maximum_body_bytes: usize,
) -> Result<JournalSequenceV1, InitialEventDeliveryError> {
    if recorded.attempt.kind != DeliveryAttemptKind::Initial {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    let attempt_record = private_record(
        &recorded.attempt.record_ref,
        EventTransitionMemberKind::DeliveryAttempt,
        &recorded.attempt,
    )?;
    let pending = EventDeliveryState::new(
        ResidentRecordRefV1::parse(format!(
            "initial-event-delivery-inbox-{}",
            recorded.attempt.attempt_id.as_str()
        ))
        .map_err(|_| InitialEventDeliveryError::PrivateIdentity)?,
        recorded.attempt.subscription_id.clone(),
        recorded.attempt.event_identity.clone(),
        recorded.attempt.event_sha256.clone(),
        recorded.attempt.attempt_id.clone(),
        recorded.attempt.ordinal,
        EventDeliveryDisposition::Pending,
    );
    let pending_record = private_record(
        &pending.record_ref,
        EventTransitionMemberKind::InboxDelivery,
        &pending,
    )?;
    let stream = EventDeliveryStream::new(
        stream_record_ref(
            &recorded.attempt.subscription_id,
            recorded.attempt.ordinal,
            &recorded.attempt.attempt_id,
        )?,
        recorded.attempt.subscription_id.clone(),
        recorded.attempt.ordinal,
        recorded.attempt.attempt_id.clone(),
    );
    let stream_record = private_record(
        &stream.record_ref,
        EventTransitionMemberKind::DeliveryStream,
        &stream,
    )?;
    verify_transition_members(
        application,
        port,
        EventCustodyTransitionKind::EnqueueDelivery,
        EventTransitionMemberKind::DeliveryAttempt,
        &attempt_record,
        &[
            EventCustodyHistoryMember::new(
                EventTransitionMemberKind::InboxDelivery,
                pending_record.coordinate().clone(),
            ),
            EventCustodyHistoryMember::new(
                EventTransitionMemberKind::DeliveryAttempt,
                attempt_record.coordinate().clone(),
            ),
            EventCustodyHistoryMember::new(
                EventTransitionMemberKind::DeliveryStream,
                stream_record.coordinate().clone(),
            ),
        ],
        maximum_body_bytes,
    )
}

fn verify_recorded_acknowledgement_members(
    application: &ResidentApplication<'_>,
    port: &mut dyn EventCustodyPort,
    recorded: &RecordedEventAcknowledgement,
    acknowledgement: &ControlRecord,
    delivery: &ControlRecord,
    maximum_body_bytes: usize,
) -> Result<JournalSequenceV1, InitialEventDeliveryError> {
    verify_transition_members(
        application,
        port,
        EventCustodyTransitionKind::RecordAcknowledgement,
        EventTransitionMemberKind::EventAcknowledgement,
        acknowledgement,
        &[
            EventCustodyHistoryMember::new(
                EventTransitionMemberKind::EventAcknowledgement,
                acknowledgement.coordinate().clone(),
            ),
            EventCustodyHistoryMember::new(
                EventTransitionMemberKind::InboxDelivery,
                delivery.coordinate().clone(),
            ),
        ],
        maximum_body_bytes,
    )?;
    verify_recorded_initial_delivery_members(application, port, recorded, maximum_body_bytes)
}

fn verify_cursor_members(
    application: &ResidentApplication<'_>,
    port: &mut dyn EventCustodyPort,
    cursor: &EventDeliveryCursor,
    cursor_record: &ControlRecord,
    maximum_body_bytes: usize,
) -> Result<JournalSequenceV1, InitialEventDeliveryError> {
    let acknowledgement_coordinate = ControlRecordCoordinate::new(
        cursor.acknowledgement_ref.clone(),
        ControlRecordDigest::parse(cursor.acknowledgement_sha256.as_str())
            .map_err(|_| InitialEventDeliveryError::RetainedStateConflict)?,
    );
    let observed = port.read_record(
        application.store(),
        &acknowledgement_coordinate,
        maximum_body_bytes,
    )?;
    let acknowledgement = observed
        .record()
        .filter(|record| {
            observed.coordinate() == &acknowledgement_coordinate
                && record.coordinate() == &acknowledgement_coordinate
                && record.schema_id().as_str()
                    == EventTransitionMemberKind::EventAcknowledgement.required_schema_id()
        })
        .ok_or(InitialEventDeliveryError::RetainedStateConflict)?;
    require_historical_record(
        port,
        application.store(),
        acknowledgement,
        maximum_body_bytes,
    )?;
    verify_transition_members(
        application,
        port,
        EventCustodyTransitionKind::AdvanceDeliveryCursor,
        EventTransitionMemberKind::DeliveryCursor,
        cursor_record,
        &[EventCustodyHistoryMember::new(
            EventTransitionMemberKind::DeliveryCursor,
            cursor_record.coordinate().clone(),
        )],
        maximum_body_bytes,
    )
}

fn stream_successor(
    plan: &InitialEventDeliveryPlan,
    predecessor: Option<&EventDeliveryStream>,
) -> Result<EventDeliveryStream, InitialEventDeliveryError> {
    let ordinal = match predecessor {
        None => EventDeliveryOrdinal::new(1)?,
        Some(value) => value
            .through_ordinal
            .checked_successor()
            .ok_or(InitialEventDeliveryError::PrivateIdentity)?,
    };
    Ok(EventDeliveryStream::new(
        stream_record_ref(
            plan.subscription.subscription_id(),
            ordinal,
            &plan.attempt_id,
        )?,
        plan.subscription.subscription_id().clone(),
        ordinal,
        plan.attempt_id.clone(),
    ))
}

/// Retain one deterministic first attempt, or recover its exact pending state.
///
/// # Errors
///
/// Returns an error when retained custody is inconsistent, authority storage
/// fails, or the application cannot commit the exact Event delivery transition.
pub fn advance_initial_event_delivery<P>(
    application: &mut ResidentApplication<'_>,
    port: &mut P,
    plan: &InitialEventDeliveryPlan,
    limits: InitialEventDeliveryRecoveryLimits,
) -> Result<InitialEventDeliveryAdvance, InitialEventDeliveryError>
where
    P: EventCustodyPort + ControlTransactionPort,
{
    let store = application.store().clone();
    let through = application
        .journal_head()
        .ok_or(InitialEventDeliveryError::MissingJournalFrontier)?;
    let committed_at =
        read_event_commit_sequence(application, port, plan, limits.maximum_private_record_bytes)?;
    if committed_at > through {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    if let InitialEventDeliveryRecovery::Pending { ordinal } =
        recover_pending(application, port, &store, through, plan, limits)?
    {
        return Ok(InitialEventDeliveryAdvance::Existing { ordinal });
    }
    recover_subscription_evidence(port, &store, plan, limits.maximum_private_record_bytes)?;
    let stream_predecessor = read_stream(port, &store, plan, limits.maximum_private_record_bytes)?;
    let stream_successor = stream_successor(plan, stream_predecessor.as_ref())?;
    let ordinal = stream_successor.through_ordinal;
    let attempt = plan.attempt(ordinal);
    let delivery = plan.delivery(ordinal);
    let transaction = port.prepare_transaction(&store)?;
    let (mut completed, receipt) = application.commit_event_custody(
        port,
        transaction,
        EventCustodyTransitionInput::EnqueueDelivery {
            event: plan.event(),
            subscription: &plan.subscription,
            delivery: &delivery,
            attempt: &attempt,
            stream_predecessor: stream_predecessor.as_ref(),
            stream_successor: &stream_successor,
        },
    )?;
    let evidence = completed.evidence();
    match evidence {
        EventCustodyCommitEvidence::ExactReplay => {
            if completed.take_continuation().is_some() {
                return Err(InitialEventDeliveryError::RetainedStateConflict);
            }
            Ok(InitialEventDeliveryAdvance::Existing { ordinal })
        }
        EventCustodyCommitEvidence::New
        | EventCustodyCommitEvidence::RecoveredExactAfterAmbiguousCommit => {
            let Some(continuation) = completed.take_continuation() else {
                return Err(InitialEventDeliveryError::RetainedStateConflict);
            };
            if continuation.kind() != EventCustodyContinuationKind::DeliverInboxAttempt {
                return Err(InitialEventDeliveryError::RetainedStateConflict);
            }
            Ok(InitialEventDeliveryAdvance::Established {
                ordinal,
                evidence,
                continuation,
                receipt: Box::new(receipt),
            })
        }
    }
}

/// Admit and retain one signed acknowledgement for an initial delivery.
///
/// The function rechecks the exact Event, subscription, attempt, pending inbox
/// state, and destination authority before it writes. An exact replay returns
/// the retained records without appending another resident-journal entry. The
/// acknowledgement proves delivery only; it does not prove processing,
/// acceptance of work, a native effect, or Pale operation settlement.
///
/// # Errors
///
/// Returns an error when the signed acknowledgement is invalid, the destination
/// authority does not match the retained subscription, any delivery basis is
/// absent or substituted, or durable custody cannot establish one exact result.
#[allow(clippy::too_many_arguments)]
pub fn record_initial_event_acknowledgement<P, Verify>(
    application: &mut ResidentApplication<'_>,
    port: &mut P,
    plan: &InitialEventDeliveryPlan,
    ordinal: EventDeliveryOrdinal,
    input: InitialEventAcknowledgementInput,
    authority: InitialEventAcknowledgementAuthority<'_>,
    maximum_body_bytes: usize,
    verify_signature: Verify,
) -> Result<InitialEventAcknowledgementAdvance, InitialEventDeliveryError>
where
    P: EventCustodyPort + ControlTransactionPort,
    Verify: FnOnce(
        &[u8],
        &EventAcknowledgementPreimageV1,
        &EventAcknowledgementDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    if maximum_body_bytes == 0 {
        return Err(InitialEventDeliveryError::ZeroBound);
    }
    let store = application.store().clone();
    let through = application
        .journal_head()
        .ok_or(InitialEventDeliveryError::MissingJournalFrontier)?;
    let committed_at = read_event_commit_sequence(application, port, plan, maximum_body_bytes)?;
    if committed_at > through {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    recover_subscription_evidence(port, &store, plan, maximum_body_bytes)?;

    if plan
        .subscription
        .destination_node_id()
        .is_some_and(|node| node != authority.acknowledging_node_id)
    {
        return Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "event acknowledgement",
            invariant: "subscription destination node signed acknowledgement",
        }
        .into());
    }

    let schema = pale_contracts::record_schema_identity(EVENT_ACKNOWLEDGEMENT_V1_SCHEMA_ID)
        .map_err(|_| InitialEventDeliveryError::RetainedStateConflict)?;
    let acknowledgement = EventAcknowledgementV1::admit_contextual_wire(
        input.acknowledgement,
        &EventAcknowledgementAdmissionContextV1 {
            expected_schema_sha256: &schema.schema_sha256,
            destination_principal_id: plan.subscription.destination_principal_id(),
            destination_node_id: plan.subscription.destination_node_id(),
            acknowledging_node_id: authority.acknowledging_node_id,
            principal_key_epoch: authority.principal_key_epoch,
            node_delegation_ref: authority.node_delegation_ref,
            node_delegation_sha256: authority.node_delegation_sha256,
        },
        verify_signature,
    )?;

    let attempt = plan.attempt(ordinal);
    let pending = plan.delivery(ordinal);
    let delivery = EventDeliveryState::new(
        input.delivery_state_ref,
        plan.subscription.subscription_id().clone(),
        event_identity(&plan.event),
        plan.event.canonical_event_sha256().clone(),
        attempt.attempt_id().clone(),
        ordinal,
        EventDeliveryDisposition::Acknowledged {
            acknowledgement_ref: input.acknowledgement_ref.clone(),
            acknowledgement_sha256: acknowledgement.canonical_acknowledgement_sha256().clone(),
            disposition: *acknowledgement.disposition(),
        },
    );
    let recorded = RecordedEventAcknowledgement {
        attempt: attempt.clone(),
        acknowledgement_ref: input.acknowledgement_ref,
        acknowledgement,
        delivery,
    };

    let expected_attempt = private_record(
        &recorded.attempt.record_ref,
        EventTransitionMemberKind::DeliveryAttempt,
        &recorded.attempt,
    )?;
    let expected_pending = private_record(
        &pending.record_ref,
        EventTransitionMemberKind::InboxDelivery,
        &pending,
    )?;
    let expected_acknowledgement = public_record(
        AdmittedEventRecord::new(&recorded.acknowledgement_ref, &recorded.acknowledgement),
        recorded
            .acknowledgement
            .canonical_acknowledgement_sha256()
            .as_str(),
    )?;
    let expected_delivery = private_record(
        &recorded.delivery.record_ref,
        EventTransitionMemberKind::InboxDelivery,
        &recorded.delivery,
    )?;

    require_historical_record(port, &store, &expected_attempt, maximum_body_bytes)?;
    require_historical_record(port, &store, &expected_pending, maximum_body_bytes)?;
    verify_initial_delivery_members(application, port, plan, ordinal, maximum_body_bytes)?;
    if !exact_index_record(
        port,
        &store,
        EventIndexQuery::DeliveryAttempt(delivery_attempt_key(recorded.attempt.attempt_id())?),
        &expected_attempt,
        maximum_body_bytes,
    )? {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }

    let inbox_query = EventIndexQuery::InboxDelivery(inbox_key(
        plan.subscription.subscription_id(),
        &event_identity(&plan.event),
    )?);
    let current_inbox = port.read_index(&store, &inbox_query, maximum_body_bytes)?;
    if current_inbox.query() != &inbox_query {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    let acknowledgement_query =
        EventIndexQuery::Acknowledgement(acknowledgement_key(recorded.attempt.attempt_id())?);
    let current_acknowledgement =
        port.read_index(&store, &acknowledgement_query, maximum_body_bytes)?;
    if current_acknowledgement.query() != &acknowledgement_query {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    match (current_inbox.record(), current_acknowledgement.record()) {
        (Some(inbox), Some(acknowledgement))
            if inbox == &expected_delivery && acknowledgement == &expected_acknowledgement =>
        {
            verify_transition_members(
                application,
                port,
                EventCustodyTransitionKind::RecordAcknowledgement,
                EventTransitionMemberKind::EventAcknowledgement,
                &expected_acknowledgement,
                &[
                    EventCustodyHistoryMember::new(
                        EventTransitionMemberKind::EventAcknowledgement,
                        expected_acknowledgement.coordinate().clone(),
                    ),
                    EventCustodyHistoryMember::new(
                        EventTransitionMemberKind::InboxDelivery,
                        expected_delivery.coordinate().clone(),
                    ),
                ],
                maximum_body_bytes,
            )?;
            return Ok(InitialEventAcknowledgementAdvance::Existing(recorded));
        }
        (Some(inbox), None) if inbox == &expected_pending => {}
        _ => return Err(InitialEventDeliveryError::RetainedStateConflict),
    }

    let transaction = port.prepare_transaction(&store)?;
    let (mut completed, receipt) = application.commit_event_custody(
        port,
        transaction,
        EventCustodyTransitionInput::RecordAcknowledgement {
            attempt: &recorded.attempt,
            predecessor: &pending,
            successor: &recorded.delivery,
            acknowledgement: AdmittedEventRecord::new(
                &recorded.acknowledgement_ref,
                &recorded.acknowledgement,
            ),
        },
    )?;
    if completed.take_continuation().is_some() {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    let evidence = completed.evidence();
    match evidence {
        EventCustodyCommitEvidence::ExactReplay => {
            Ok(InitialEventAcknowledgementAdvance::Existing(recorded))
        }
        EventCustodyCommitEvidence::New
        | EventCustodyCommitEvidence::RecoveredExactAfterAmbiguousCommit => {
            Ok(InitialEventAcknowledgementAdvance::Established {
                recorded,
                evidence,
                receipt: Box::new(receipt),
            })
        }
    }
}

/// Advance one subscription cursor through the next contiguous acknowledgement.
///
/// The acknowledgement, attempt, and terminal inbox state must already have
/// exact custody. A later ordinal waits without writing until every earlier
/// ordinal is terminal. Repeating an exact advance is read-only.
///
/// # Errors
///
/// Returns an error when retained acknowledgement evidence is absent or
/// substituted, the cursor record is malformed, or durable custody cannot
/// establish one exact result.
pub fn advance_initial_event_delivery_cursor<P>(
    application: &mut ResidentApplication<'_>,
    port: &mut P,
    recorded: &RecordedEventAcknowledgement,
    maximum_body_bytes: usize,
) -> Result<InitialEventDeliveryCursorAdvance, InitialEventDeliveryError>
where
    P: EventCustodyPort + ControlTransactionPort,
{
    if maximum_body_bytes == 0 {
        return Err(InitialEventDeliveryError::ZeroBound);
    }
    application
        .journal_head()
        .ok_or(InitialEventDeliveryError::MissingJournalFrontier)?;
    let store = application.store().clone();
    let expected_attempt = private_record(
        &recorded.attempt.record_ref,
        EventTransitionMemberKind::DeliveryAttempt,
        &recorded.attempt,
    )?;
    let expected_acknowledgement = public_record(
        AdmittedEventRecord::new(&recorded.acknowledgement_ref, &recorded.acknowledgement),
        recorded
            .acknowledgement
            .canonical_acknowledgement_sha256()
            .as_str(),
    )?;
    let expected_delivery = private_record(
        &recorded.delivery.record_ref,
        EventTransitionMemberKind::InboxDelivery,
        &recorded.delivery,
    )?;
    require_historical_record(port, &store, &expected_attempt, maximum_body_bytes)?;
    require_historical_record(port, &store, &expected_acknowledgement, maximum_body_bytes)?;
    require_historical_record(port, &store, &expected_delivery, maximum_body_bytes)?;
    verify_recorded_acknowledgement_members(
        application,
        port,
        recorded,
        &expected_acknowledgement,
        &expected_delivery,
        maximum_body_bytes,
    )?;
    if !exact_index_record(
        port,
        &store,
        EventIndexQuery::DeliveryAttempt(delivery_attempt_key(recorded.attempt.attempt_id())?),
        &expected_attempt,
        maximum_body_bytes,
    )? || !exact_index_record(
        port,
        &store,
        EventIndexQuery::Acknowledgement(acknowledgement_key(recorded.attempt.attempt_id())?),
        &expected_acknowledgement,
        maximum_body_bytes,
    )? || !exact_index_record(
        port,
        &store,
        EventIndexQuery::InboxDelivery(inbox_key(
            &recorded.attempt.subscription_id,
            &recorded.attempt.event_identity,
        )?),
        &expected_delivery,
        maximum_body_bytes,
    )? {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }

    let cursor_query =
        EventIndexQuery::DeliveryCursor(delivery_cursor_key(&recorded.attempt.subscription_id)?);
    let observed = port.read_index(&store, &cursor_query, maximum_body_bytes)?;
    if observed.query() != &cursor_query {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    let predecessor = observed.record().map(decode_delivery_cursor).transpose()?;
    if let (Some(record), Some(cursor)) = (observed.record(), predecessor.as_ref()) {
        verify_cursor_members(application, port, cursor, record, maximum_body_bytes)?;
    }
    let acknowledged_ordinal = recorded.attempt.ordinal;
    match predecessor.as_ref() {
        Some(cursor) if cursor.through_ordinal() == acknowledged_ordinal => {
            let successor = delivery_cursor_successor(recorded)?;
            let expected = private_record(
                &successor.record_ref,
                EventTransitionMemberKind::DeliveryCursor,
                &successor,
            )?;
            if observed.record() != Some(&expected) {
                return Err(InitialEventDeliveryError::RetainedStateConflict);
            }
            return Ok(InitialEventDeliveryCursorAdvance::Existing(cursor.clone()));
        }
        Some(cursor) if cursor.through_ordinal() > acknowledged_ordinal => {
            return Ok(InitialEventDeliveryCursorAdvance::AlreadyBeyond {
                cursor: cursor.clone(),
                acknowledged_ordinal,
            });
        }
        Some(cursor)
            if cursor.through_ordinal().checked_successor() != Some(acknowledged_ordinal) =>
        {
            return Ok(InitialEventDeliveryCursorAdvance::WaitingForEarlier {
                current_through: Some(cursor.through_ordinal()),
                acknowledged_ordinal,
            });
        }
        None if acknowledged_ordinal.get() != 1 => {
            return Ok(InitialEventDeliveryCursorAdvance::WaitingForEarlier {
                current_through: None,
                acknowledged_ordinal,
            });
        }
        Some(_) | None => {}
    }

    let successor = delivery_cursor_successor(recorded)?;
    let transaction = port.prepare_transaction(&store)?;
    let (mut completed, receipt) = application.commit_event_custody(
        port,
        transaction,
        EventCustodyTransitionInput::AdvanceDeliveryCursor {
            attempt: &recorded.attempt,
            acknowledgement: AdmittedEventRecord::new(
                &recorded.acknowledgement_ref,
                &recorded.acknowledgement,
            ),
            predecessor: predecessor.as_ref(),
            successor: &successor,
        },
    )?;
    if completed.take_continuation().is_some() {
        return Err(InitialEventDeliveryError::RetainedStateConflict);
    }
    let evidence = completed.evidence();
    match evidence {
        EventCustodyCommitEvidence::ExactReplay => {
            Ok(InitialEventDeliveryCursorAdvance::Existing(successor))
        }
        EventCustodyCommitEvidence::New
        | EventCustodyCommitEvidence::RecoveredExactAfterAmbiguousCommit => {
            Ok(InitialEventDeliveryCursorAdvance::Established {
                cursor: successor,
                evidence,
                receipt: Box::new(receipt),
            })
        }
    }
}
