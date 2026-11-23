//! Canonical key derivation and semantic-to-physical Event projection.

use pale_core::canon;
use pale_core::records::{
    ArtifactDigest, CanonicalEventIdentityV1, DeliveryAttemptIdV1, EventReferenceV1,
    EventSubscriptionIdV1, GenealogyEdgeIdV1, GenealogyEdgeV1, GenealogyProofV1, JournalSequenceV1,
    PrincipalResidentWriterFenceStateV1, ReconciliationFrontierV1, ReconciliationSubjectV1,
    ResidentPrincipalIdV1, ResidentRecordRefV1, SchemaDigest, SchemaId, SourceOwnerV1,
    SourceVersionV1,
};
use pale_ports::{
    AuthorityStoreCoordinate, ControlRecord, ControlRecordBytes, ControlRecordCoordinate,
    ControlRecordDigest, EventCustodyTransition, EventCustodyTransitionKind, EventIndexAssertion,
    EventIndexKey, EventIndexMutation, EventIndexPredecessor, EventIndexQuery,
    EventTransitionMember, EventTransitionMemberKind, PreparedAuthorityAppend,
};
use serde::Serialize;

use crate::resident::record_projection;

use super::record::{DeliveryAttemptKind, RetentionDisposition};
use super::{
    require, AdmittedEventRecord, EventCustodyProjectionError, EventCustodyTransitionInput,
    EventPublicationState,
};

use super::record::EventCustodyTransitionWitness;

/// Separates the full producer-scoped Event identity from every other key domain.
const EVENT_IDENTITY_KEY_DOMAIN: &[u8] = b"PALE_EVENT_IDENTITY_INDEX_KEY_V1\0";
/// Separates the Event's admission-sequence witness from semantic identity.
const EVENT_COMMIT_WITNESS_KEY_DOMAIN: &[u8] = b"PALE_EVENT_COMMIT_WITNESS_INDEX_KEY_V1\0";
/// Separates one immutable owner source from producer-scoped Event identity.
const OWNER_SOURCE_KEY_DOMAIN: &[u8] = b"PALE_EVENT_OWNER_SOURCE_INDEX_KEY_V1\0";
/// Separates identity-conflict custody from the incumbent Event mapping.
const EVENT_IDENTITY_CONFLICT_KEY_DOMAIN: &[u8] = b"PALE_EVENT_IDENTITY_CONFLICT_INDEX_KEY_V1\0";
/// Separates subscription lifecycle state.
const SUBSCRIPTION_KEY_DOMAIN: &[u8] = b"PALE_EVENT_SUBSCRIPTION_INDEX_KEY_V1\0";
/// Separates one subscription/Event inbox state.
const INBOX_DELIVERY_KEY_DOMAIN: &[u8] = b"PALE_EVENT_INBOX_DELIVERY_INDEX_KEY_V1\0";
/// Separates one immutable delivery attempt.
const DELIVERY_ATTEMPT_KEY_DOMAIN: &[u8] = b"PALE_EVENT_DELIVERY_ATTEMPT_INDEX_KEY_V1\0";
/// Separates each subscription's delivery-stream allocation head.
const DELIVERY_STREAM_KEY_DOMAIN: &[u8] = b"PALE_EVENT_DELIVERY_STREAM_INDEX_KEY_V1\0";
/// Separates one signed delivery acknowledgement.
const ACKNOWLEDGEMENT_KEY_DOMAIN: &[u8] = b"PALE_EVENT_ACKNOWLEDGEMENT_INDEX_KEY_V1\0";
/// Separates each subscription's contiguous terminal cursor.
const DELIVERY_CURSOR_KEY_DOMAIN: &[u8] = b"PALE_EVENT_DELIVERY_CURSOR_INDEX_KEY_V1\0";
/// Separates independently advancing source frontiers.
const SOURCE_FRONTIER_KEY_DOMAIN: &[u8] = b"PALE_EVENT_SOURCE_FRONTIER_INDEX_KEY_V1\0";
/// Separates occurrence placement from Event identity custody.
const OCCURRENCE_KEY_DOMAIN: &[u8] = b"PALE_INTERACTION_OCCURRENCE_INDEX_KEY_V1\0";
/// Separates directed truth-classed Genealogy edges.
const GENEALOGY_EDGE_KEY_DOMAIN: &[u8] = b"PALE_GENEALOGY_EDGE_INDEX_KEY_V1\0";
/// Separates each reconciliation subject's immutable snapshot head.
const RECONCILIATION_FRONTIER_KEY_DOMAIN: &[u8] = b"PALE_RECONCILIATION_FRONTIER_INDEX_KEY_V1\0";
/// Separates Pale-derived outbox retention from owner ingress.
const PALE_DERIVED_OUTBOX_KEY_DOMAIN: &[u8] = b"PALE_DERIVED_OUTBOX_INDEX_KEY_V1\0";
/// Separates exact transition-membership witnesses from all semantic indexes.
const TRANSITION_WITNESS_KEY_DOMAIN: &[u8] = b"PALE_EVENT_TRANSITION_WITNESS_KEY_V1\0";

#[derive(Serialize)]
struct DomainKeyPreimage<'a, T> {
    initial_v1_domain: &'static str,
    value: &'a T,
}

#[derive(Serialize)]
struct PairKeyPreimage<'a, L, R> {
    initial_v1_domain: &'static str,
    left: &'a L,
    right: &'a R,
}

#[derive(Serialize)]
struct IdentityConflictState<'a> {
    event_identity: &'a CanonicalEventIdentityV1,
    incumbent_ref: &'a ResidentRecordRefV1,
    incumbent_sha256: &'a str,
    candidate_ref: &'a ResidentRecordRefV1,
    candidate_sha256: &'a str,
}

#[derive(Serialize)]
struct OwnerSourceIdentity<'a> {
    origin_principal_id: &'a ResidentPrincipalIdV1,
    source_owner: &'a SourceOwnerV1,
    source_record_ref: &'a ResidentRecordRefV1,
    source_record_schema_id: &'a SchemaId,
    source_record_schema_sha256: &'a SchemaDigest,
    source_record_content_sha256: &'a ArtifactDigest,
    source_version: &'a SourceVersionV1,
}

#[derive(Serialize)]
struct TransitionWitnessIdentity<'a> {
    transition_kind: EventCustodyTransitionKind,
    subject_kind: EventTransitionMemberKind,
    subject: &'a ControlRecordCoordinate,
}

type TransitionProjection = (
    EventCustodyTransitionKind,
    Vec<EventTransitionMember>,
    Vec<EventIndexMutation>,
    Vec<EventIndexAssertion>,
);

pub(super) fn event_identity(event: &EventReferenceV1) -> CanonicalEventIdentityV1 {
    CanonicalEventIdentityV1 {
        origin_principal_id: event.origin_principal_id().clone(),
        producer_epoch: event.producer_epoch().clone(),
        event_id: event.event_id().clone(),
    }
}

pub(super) fn event_identity_key(
    event: &EventReferenceV1,
) -> Result<EventIndexKey, EventCustodyProjectionError> {
    derive_key(
        EVENT_IDENTITY_KEY_DOMAIN,
        "event_identity",
        &event_identity(event),
    )
}

pub(super) fn event_commit_witness_key(
    event: &EventReferenceV1,
) -> Result<EventIndexKey, EventCustodyProjectionError> {
    derive_key(
        EVENT_COMMIT_WITNESS_KEY_DOMAIN,
        "event_commit_witness",
        &event_identity(event),
    )
}

pub(super) fn delivery_cursor_key(
    subscription_id: &EventSubscriptionIdV1,
) -> Result<EventIndexKey, EventCustodyProjectionError> {
    derive_key(
        DELIVERY_CURSOR_KEY_DOMAIN,
        "delivery_cursor",
        subscription_id,
    )
}

pub(super) fn transition_witness_key(
    transition_kind: EventCustodyTransitionKind,
    subject_kind: EventTransitionMemberKind,
    subject: &ControlRecordCoordinate,
) -> Result<EventIndexKey, EventCustodyProjectionError> {
    derive_key(
        TRANSITION_WITNESS_KEY_DOMAIN,
        "transition_witness",
        &TransitionWitnessIdentity {
            transition_kind,
            subject_kind,
            subject,
        },
    )
}

fn transition_witness(
    store: &AuthorityStoreCoordinate,
    committed_at: JournalSequenceV1,
    transition_kind: EventCustodyTransitionKind,
    subject_kind: EventTransitionMemberKind,
    subject: &ControlRecordCoordinate,
) -> Result<(EventTransitionMember, EventIndexMutation), EventCustodyProjectionError> {
    let key = transition_witness_key(transition_kind, subject_kind, subject)?;
    let witness = EventCustodyTransitionWitness {
        record_ref: ResidentRecordRefV1::parse(format!(
            "event-transition-witness-{}",
            key.as_str()
        ))
        .map_err(|_| EventCustodyProjectionError::Lineage)?,
        store_id: store.store_id().clone(),
        store_incarnation_id: store.incarnation_id().clone(),
        transition_kind,
        subject_kind,
        subject: subject.clone(),
        committed_at,
    };
    let record = private_record(
        &witness.record_ref,
        EventTransitionMemberKind::TransitionWitness,
        &witness,
    )?;
    Ok((
        member(EventTransitionMemberKind::TransitionWitness, record.clone()),
        mutation(
            EventIndexQuery::TransitionWitness(key),
            EventIndexPredecessor::Absent,
            &record,
        ),
    ))
}

/// Derive the producer-independent lookup for one exact immutable owner source.
///
/// This key identifies the source represented by an Event. It deliberately
/// excludes producer, node, recording-time, delivery, and policy fields. A
/// later producer must therefore find and readmit the existing Event instead
/// of allocating a second Event for the same source.
pub(super) fn owner_source_key(
    event: &EventReferenceV1,
) -> Result<EventIndexKey, EventCustodyProjectionError> {
    owner_source_key_from_parts(
        event.origin_principal_id(),
        event.source_owner(),
        event.source_record_ref(),
        event.source_record_schema_id(),
        event.source_record_schema_sha256(),
        event.source_record_content_sha256(),
        event.source_version(),
    )
}

/// Derive the producer-independent lookup before an Event is issued.
///
/// A publication plan knows its exact owner source before it knows which
/// admitted producer and writer will issue the Event. Keeping this projection
/// beside [`owner_source_key`] prevents that lookup from acquiring a second
/// hash domain or preimage shape.
#[allow(clippy::too_many_arguments)]
pub(super) fn owner_source_key_from_parts(
    origin_principal_id: &ResidentPrincipalIdV1,
    source_owner: &SourceOwnerV1,
    source_record_ref: &ResidentRecordRefV1,
    source_record_schema_id: &SchemaId,
    source_record_schema_sha256: &SchemaDigest,
    source_record_content_sha256: &ArtifactDigest,
    source_version: &SourceVersionV1,
) -> Result<EventIndexKey, EventCustodyProjectionError> {
    derive_key(
        OWNER_SOURCE_KEY_DOMAIN,
        "owner_source",
        &OwnerSourceIdentity {
            origin_principal_id,
            source_owner,
            source_record_ref,
            source_record_schema_id,
            source_record_schema_sha256,
            source_record_content_sha256,
            source_version,
        },
    )
}

fn derive_key<T: Serialize>(
    domain: &[u8],
    label: &'static str,
    value: &T,
) -> Result<EventIndexKey, EventCustodyProjectionError> {
    let canonical = canon::to_canonical_string(&DomainKeyPreimage {
        initial_v1_domain: label,
        value,
    })?;
    let mut bytes = Vec::with_capacity(domain.len() + canonical.len());
    bytes.extend_from_slice(domain);
    bytes.extend_from_slice(canonical.as_bytes());
    Ok(EventIndexKey::new(ControlRecordDigest::parse(
        pale_core::sha256_hex(&bytes),
    )?))
}

fn derive_pair_key<L: Serialize, R: Serialize>(
    domain: &[u8],
    label: &'static str,
    left: &L,
    right: &R,
) -> Result<EventIndexKey, EventCustodyProjectionError> {
    derive_key(
        domain,
        label,
        &PairKeyPreimage {
            initial_v1_domain: label,
            left,
            right,
        },
    )
}

pub(super) fn public_record<R>(
    value: AdmittedEventRecord<'_, R>,
    digest: &str,
) -> Result<ControlRecord, EventCustodyProjectionError>
where
    R: Serialize + record_projection::ResidentRecordSchema,
{
    Ok(record_projection::record(
        record_projection::coordinate(value.record_ref, digest)?,
        value.record,
    )?)
}

/// Project one typed private row into its closed physical custody role.
///
/// [`EventTransitionMemberKind::required_schema_id`] is an application-private role
/// identity, not a published JSON Schema family. Its digest slot therefore
/// carries the SHA-256 fingerprint of that role identity. Structural and
/// cross-record admission remain owned by this typed application projection;
/// this fingerprint alone is not structural-schema validation.
pub(super) fn private_record<T: Serialize>(
    record_ref: &ResidentRecordRefV1,
    kind: EventTransitionMemberKind,
    value: &T,
) -> Result<ControlRecord, EventCustodyProjectionError> {
    let bytes = canon::to_canonical_string(value)?.into_bytes();
    let digest = ControlRecordDigest::parse(pale_core::sha256_hex(&bytes))?;
    let schema_id = SchemaId::parse(kind.required_schema_id())
        .map_err(|_| EventCustodyProjectionError::Lineage)?;
    let schema_sha256 =
        SchemaDigest::parse(pale_core::sha256_hex(kind.required_schema_id().as_bytes()))
            .map_err(|_| EventCustodyProjectionError::Lineage)?;
    Ok(ControlRecord::new(
        ControlRecordCoordinate::new(record_ref.clone(), digest),
        schema_id,
        schema_sha256,
        ControlRecordBytes::new(bytes)?,
    ))
}

fn exact(record: ControlRecord) -> EventIndexPredecessor {
    EventIndexPredecessor::Exact(record)
}

fn member(kind: EventTransitionMemberKind, record: ControlRecord) -> EventTransitionMember {
    EventTransitionMember::new(kind, record)
}

fn mutation(
    query: EventIndexQuery,
    expected: EventIndexPredecessor,
    successor: &ControlRecord,
) -> EventIndexMutation {
    EventIndexMutation::new(query, expected, successor.coordinate().clone())
}

fn assertion(query: EventIndexQuery, record: ControlRecord) -> EventIndexAssertion {
    EventIndexAssertion::new(query, exact(record))
}

/// Projected content awaiting the application journal's exact append.
/// No authority state or physical custody changes during this preparation.
pub(in crate::resident) struct EventCustodyContent {
    pub(super) store: AuthorityStoreCoordinate,
    pub(super) kind: EventCustodyTransitionKind,
    pub(super) members: Vec<EventTransitionMember>,
    pub(super) mutations: Vec<EventIndexMutation>,
    pub(super) assertions: Vec<EventIndexAssertion>,
}

impl EventCustodyContent {
    /// Validate the complete physical shape once its journal append is available.
    pub(in crate::resident) fn attach(
        self,
        transaction: pale_ports::ControlTransactionPreparation,
        journal_append: PreparedAuthorityAppend,
    ) -> Result<EventCustodyTransition, EventCustodyProjectionError> {
        Ok(EventCustodyTransition::new(
            self.store,
            transaction,
            self.kind,
            journal_append,
            self.members,
            self.mutations,
            self.assertions,
        )?)
    }
}

pub(in crate::resident) fn project_content(
    store: AuthorityStoreCoordinate,
    journal_sequence: JournalSequenceV1,
    input: EventCustodyTransitionInput<'_>,
) -> Result<EventCustodyContent, EventCustodyProjectionError> {
    let (kind, mut members, mut mutations, assertions) = match input {
        EventCustodyTransitionInput::AdmitOwnerReference(publication) => {
            let closure = &publication.closure;
            closure.event.record.verify()?;
            closure.allocation.verify()?;
            closure.writer_fence.verify()?;
            require(
                closure.event.record.origin_principal_id()
                    == closure.allocation.origin_principal_id()
                    && closure.event.record.producer_epoch() == closure.allocation.producer_epoch()
                    && closure.writer_fence.principal_id()
                        == closure.allocation.origin_principal_id()
                    && closure.writer_fence.state() == &PrincipalResidentWriterFenceStateV1::Active,
            )?;
            if let EventPublicationState::CommittedBeforeRetirement { retirement, .. } =
                closure.state
            {
                retirement.verify()?;
                require(
                    retirement.incarnation_ref() == closure.allocation_ref
                        && retirement.incarnation_sha256()
                            == closure.allocation.canonical_incarnation_sha256()
                        && retirement.origin_principal_id()
                            == closure.allocation.origin_principal_id()
                        && retirement.producer_id() == closure.allocation.producer_id()
                        && retirement.producer_epoch() == closure.allocation.producer_epoch(),
                )?;
            }
            publication.verifier.verify(closure)?;
            let event = public_record(
                closure.event,
                closure.event.record.canonical_event_sha256().as_str(),
            )?;
            let identity_key = event_identity_key(closure.event.record)?;
            let witness_key = event_commit_witness_key(closure.event.record)?;
            let owner_source_key = owner_source_key(closure.event.record)?;
            let witness_ref = ResidentRecordRefV1::parse(format!(
                "event-commit-witness-{}",
                witness_key.as_str()
            ))
            .map_err(|_| EventCustodyProjectionError::Lineage)?;
            let witness = super::record::EventCommitWitness {
                record_ref: witness_ref,
                store_id: store.store_id().clone(),
                store_incarnation_id: store.incarnation_id().clone(),
                event_ref: closure.event.record_ref.clone(),
                event_sha256: closure.event.record.canonical_event_sha256().clone(),
                event_identity: event_identity(closure.event.record),
                committed_at: journal_sequence,
            };
            let witness_record = private_record(
                &witness.record_ref,
                EventTransitionMemberKind::EventCommitWitness,
                &witness,
            )?;
            (
                EventCustodyTransitionKind::AdmitOwnerReference,
                vec![
                    member(EventTransitionMemberKind::EventReference, event.clone()),
                    member(
                        EventTransitionMemberKind::EventCommitWitness,
                        witness_record.clone(),
                    ),
                ],
                vec![
                    mutation(
                        EventIndexQuery::EventIdentity(identity_key),
                        EventIndexPredecessor::Absent,
                        &event,
                    ),
                    mutation(
                        EventIndexQuery::OwnerSource(owner_source_key),
                        EventIndexPredecessor::Absent,
                        &event,
                    ),
                    mutation(
                        EventIndexQuery::EventCommitWitness(witness_key),
                        EventIndexPredecessor::Absent,
                        &witness_record,
                    ),
                ],
                Vec::new(),
            )
        }
        EventCustodyTransitionInput::QuarantineIdentityConflict {
            incumbent,
            candidate,
            conflict_ref,
        } => {
            candidate.record.verify()?;
            let candidate_record = public_record(
                candidate,
                candidate.record.canonical_event_sha256().as_str(),
            )?;
            let identity = event_identity(candidate.record);
            let identity_key = event_identity_key(candidate.record)?;
            require(
                incumbent.schema_id().as_str()
                    == EventTransitionMemberKind::EventReference.required_schema_id()
                    && incumbent.coordinate().record_sha256().as_str()
                        != candidate.record.canonical_event_sha256().as_str(),
            )?;
            let conflict = private_record(
                conflict_ref,
                EventTransitionMemberKind::IdentityConflict,
                &IdentityConflictState {
                    event_identity: &identity,
                    incumbent_ref: incumbent.coordinate().record_ref(),
                    incumbent_sha256: incumbent.coordinate().record_sha256().as_str(),
                    candidate_ref: candidate_record.coordinate().record_ref(),
                    candidate_sha256: candidate_record.coordinate().record_sha256().as_str(),
                },
            )?;
            let conflict_key = derive_key(
                EVENT_IDENTITY_CONFLICT_KEY_DOMAIN,
                "event_identity_conflict",
                &identity,
            )?;
            (
                EventCustodyTransitionKind::QuarantineIdentityConflict,
                vec![member(
                    EventTransitionMemberKind::IdentityConflict,
                    conflict.clone(),
                )],
                vec![mutation(
                    EventIndexQuery::IdentityConflict(conflict_key),
                    EventIndexPredecessor::Absent,
                    &conflict,
                )],
                vec![assertion(
                    EventIndexQuery::EventIdentity(identity_key),
                    incumbent.clone(),
                )],
            )
        }
        EventCustodyTransitionInput::AdmitSubscription(subscription) => {
            require(matches!(
                subscription.lifecycle,
                super::SubscriptionLifecycle::Active
            ))?;
            let record = private_record(
                &subscription.record_ref,
                EventTransitionMemberKind::Subscription,
                subscription,
            )?;
            let key = derive_key(
                SUBSCRIPTION_KEY_DOMAIN,
                "subscription",
                &subscription.subscription_id,
            )?;
            (
                EventCustodyTransitionKind::AdmitSubscription,
                vec![member(
                    EventTransitionMemberKind::Subscription,
                    record.clone(),
                )],
                vec![mutation(
                    EventIndexQuery::Subscription(key),
                    EventIndexPredecessor::Absent,
                    &record,
                )],
                Vec::new(),
            )
        }
        EventCustodyTransitionInput::RetireSubscription {
            predecessor,
            successor,
        } => {
            require(
                matches!(predecessor.lifecycle, super::SubscriptionLifecycle::Active)
                    && matches!(
                        successor.lifecycle,
                        super::SubscriptionLifecycle::Retired { .. }
                    )
                    && predecessor.subscription_id == successor.subscription_id
                    && predecessor.source_owner == successor.source_owner
                    && predecessor.event_types == successor.event_types
                    && predecessor.destination_principal_id == successor.destination_principal_id
                    && predecessor.destination_node_id == successor.destination_node_id
                    && predecessor.ingress_frontier == successor.ingress_frontier,
            )?;
            let prior = private_record(
                &predecessor.record_ref,
                EventTransitionMemberKind::Subscription,
                predecessor,
            )?;
            let next = private_record(
                &successor.record_ref,
                EventTransitionMemberKind::Subscription,
                successor,
            )?;
            let key = derive_key(
                SUBSCRIPTION_KEY_DOMAIN,
                "subscription",
                &successor.subscription_id,
            )?;
            (
                EventCustodyTransitionKind::RetireSubscription,
                vec![member(
                    EventTransitionMemberKind::Subscription,
                    next.clone(),
                )],
                vec![mutation(
                    EventIndexQuery::Subscription(key),
                    exact(prior),
                    &next,
                )],
                Vec::new(),
            )
        }
        EventCustodyTransitionInput::EnqueueDelivery {
            event,
            subscription,
            delivery,
            attempt,
            stream_predecessor,
            stream_successor,
        } => project_delivery(
            event,
            subscription,
            None,
            delivery,
            attempt,
            stream_predecessor,
            stream_successor,
        )?,
        EventCustodyTransitionInput::EnqueueReplay {
            event,
            subscription,
            predecessor,
            successor,
            attempt,
            stream_predecessor,
            stream_successor,
        } => project_delivery(
            event,
            subscription,
            Some(predecessor),
            successor,
            attempt,
            Some(stream_predecessor),
            stream_successor,
        )?,
        EventCustodyTransitionInput::RecordAcknowledgement {
            attempt,
            predecessor,
            successor,
            acknowledgement,
        } => {
            acknowledgement.record.verify()?;
            let acknowledgement_identity = acknowledgement.record.canonical_event_identity();
            require(
                matches!(
                    predecessor.disposition,
                    super::EventDeliveryDisposition::Pending
                ) && predecessor.subscription_id == attempt.subscription_id
                    && predecessor.event_identity == attempt.event_identity
                    && predecessor.event_sha256 == attempt.event_sha256
                    && predecessor.attempt_id == attempt.attempt_id
                    && predecessor.ordinal == attempt.ordinal
                    && successor.subscription_id == predecessor.subscription_id
                    && successor.event_identity == predecessor.event_identity
                    && successor.event_sha256 == predecessor.event_sha256
                    && successor.attempt_id == predecessor.attempt_id
                    && successor.ordinal == predecessor.ordinal
                    && acknowledgement.record.delivery_attempt_id() == &attempt.attempt_id
                    && acknowledgement.record.subscription_id() == &attempt.subscription_id
                    && acknowledgement_identity == &attempt.event_identity
                    && acknowledgement.record.canonical_event_sha256() == &attempt.event_sha256
                    && matches!(
                        successor.disposition,
                        super::EventDeliveryDisposition::Acknowledged { .. }
                    ),
            )?;
            if let super::EventDeliveryDisposition::Acknowledged {
                acknowledgement_ref,
                acknowledgement_sha256,
                disposition,
            } = &successor.disposition
            {
                require(
                    acknowledgement_ref == acknowledgement.record_ref
                        && acknowledgement_sha256
                            == acknowledgement.record.canonical_acknowledgement_sha256()
                        && disposition == acknowledgement.record.disposition(),
                )?;
            }
            let attempt_record = private_record(
                &attempt.record_ref,
                EventTransitionMemberKind::DeliveryAttempt,
                attempt,
            )?;
            let prior = private_record(
                &predecessor.record_ref,
                EventTransitionMemberKind::InboxDelivery,
                predecessor,
            )?;
            let next = private_record(
                &successor.record_ref,
                EventTransitionMemberKind::InboxDelivery,
                successor,
            )?;
            let ack = public_record(
                acknowledgement,
                acknowledgement
                    .record
                    .canonical_acknowledgement_sha256()
                    .as_str(),
            )?;
            let inbox_key = inbox_key(&successor.subscription_id, &successor.event_identity)?;
            let ack_key = derive_key(
                ACKNOWLEDGEMENT_KEY_DOMAIN,
                "acknowledgement",
                &attempt.attempt_id,
            )?;
            let attempt_key = derive_key(
                DELIVERY_ATTEMPT_KEY_DOMAIN,
                "delivery_attempt",
                &attempt.attempt_id,
            )?;
            (
                EventCustodyTransitionKind::RecordAcknowledgement,
                vec![
                    member(EventTransitionMemberKind::EventAcknowledgement, ack.clone()),
                    member(EventTransitionMemberKind::InboxDelivery, next.clone()),
                ],
                vec![
                    mutation(
                        EventIndexQuery::Acknowledgement(ack_key),
                        EventIndexPredecessor::Absent,
                        &ack,
                    ),
                    mutation(
                        EventIndexQuery::InboxDelivery(inbox_key),
                        exact(prior),
                        &next,
                    ),
                ],
                vec![assertion(
                    EventIndexQuery::DeliveryAttempt(attempt_key),
                    attempt_record,
                )],
            )
        }
        EventCustodyTransitionInput::AdvanceDeliveryCursor {
            attempt,
            acknowledgement,
            predecessor,
            successor,
        } => {
            acknowledgement.record.verify()?;
            require(
                attempt.subscription_id == successor.subscription_id
                    && attempt.attempt_id == *acknowledgement.record.delivery_attempt_id()
                    && attempt.event_identity == *acknowledgement.record.canonical_event_identity()
                    && attempt.event_sha256 == *acknowledgement.record.canonical_event_sha256()
                    && attempt.ordinal == successor.through_ordinal
                    && successor.subscription_id == *acknowledgement.record.subscription_id()
                    && successor.acknowledgement_ref == *acknowledgement.record_ref
                    && successor.acknowledgement_sha256
                        == *acknowledgement.record.canonical_acknowledgement_sha256()
                    && predecessor.map_or(successor.through_ordinal.get() == 1, |prior| {
                        prior.subscription_id == successor.subscription_id
                            && prior.through_ordinal.checked_successor()
                                == Some(successor.through_ordinal)
                    }),
            )?;
            let ack = public_record(
                acknowledgement,
                acknowledgement
                    .record
                    .canonical_acknowledgement_sha256()
                    .as_str(),
            )?;
            let next = private_record(
                &successor.record_ref,
                EventTransitionMemberKind::DeliveryCursor,
                successor,
            )?;
            let prior = predecessor
                .map(|value| {
                    private_record(
                        &value.record_ref,
                        EventTransitionMemberKind::DeliveryCursor,
                        value,
                    )
                })
                .transpose()?;
            let cursor_key = delivery_cursor_key(&successor.subscription_id)?;
            let ack_key = derive_key(
                ACKNOWLEDGEMENT_KEY_DOMAIN,
                "acknowledgement",
                acknowledgement.record.delivery_attempt_id(),
            )?;
            (
                EventCustodyTransitionKind::AdvanceDeliveryCursor,
                vec![member(
                    EventTransitionMemberKind::DeliveryCursor,
                    next.clone(),
                )],
                vec![mutation(
                    EventIndexQuery::DeliveryCursor(cursor_key),
                    prior.map_or(EventIndexPredecessor::Absent, exact),
                    &next,
                )],
                vec![assertion(EventIndexQuery::Acknowledgement(ack_key), ack)],
            )
        }
        EventCustodyTransitionInput::AdvanceSourceFrontier {
            predecessor,
            successor,
        } => {
            require(predecessor.is_none_or(|prior| {
                prior.subscription_id == successor.subscription_id
                    && prior.source_owner == successor.source_owner
                    && match (&prior.owner_sequence, &successor.owner_sequence) {
                        (Some(left), Some(right)) => left < right,
                        (None, None) => {
                            prior.event_identity != successor.event_identity
                                || prior.event_sha256 != successor.event_sha256
                        }
                        _ => false,
                    }
            }))?;
            let next = private_record(
                &successor.record_ref,
                EventTransitionMemberKind::SourceFrontier,
                successor,
            )?;
            let prior = predecessor
                .map(|value| {
                    private_record(
                        &value.record_ref,
                        EventTransitionMemberKind::SourceFrontier,
                        value,
                    )
                })
                .transpose()?;
            let key = derive_pair_key(
                SOURCE_FRONTIER_KEY_DOMAIN,
                "source_frontier",
                &successor.subscription_id,
                &successor.source_owner,
            )?;
            (
                EventCustodyTransitionKind::AdvanceSourceFrontier,
                vec![member(
                    EventTransitionMemberKind::SourceFrontier,
                    next.clone(),
                )],
                vec![mutation(
                    EventIndexQuery::SourceFrontier(key),
                    prior.map_or(EventIndexPredecessor::Absent, exact),
                    &next,
                )],
                Vec::new(),
            )
        }
        EventCustodyTransitionInput::ProjectInteractionOccurrence {
            event,
            occurrence,
            retention_record_ref,
            maximum_publication_bytes,
        } => {
            event.record.verify()?;
            occurrence.record.verify()?;
            require(
                occurrence.record.event_reference_ref() == event.record_ref
                    && occurrence.record.event_reference_sha256()
                        == event.record.canonical_event_sha256(),
            )?;
            let event_record =
                public_record(event, event.record.canonical_event_sha256().as_str())?;
            let occurrence_record = public_record(
                occurrence,
                occurrence.record.canonical_occurrence_sha256().as_str(),
            )?;
            let event_key = event_identity_key(event.record)?;
            let occurrence_key =
                occurrence_key(event.record_ref, event.record.canonical_event_sha256())?;
            let (retention, retention_index) = retain_publication(
                &occurrence_record,
                retention_record_ref,
                maximum_publication_bytes,
            )?;
            (
                EventCustodyTransitionKind::ProjectInteractionOccurrence,
                vec![
                    member(
                        EventTransitionMemberKind::InteractionOccurrence,
                        occurrence_record.clone(),
                    ),
                    retention,
                ],
                vec![
                    mutation(
                        EventIndexQuery::InteractionOccurrence(occurrence_key),
                        EventIndexPredecessor::Absent,
                        &occurrence_record,
                    ),
                    retention_index,
                ],
                vec![assertion(
                    EventIndexQuery::EventIdentity(event_key),
                    event_record,
                )],
            )
        }
        EventCustodyTransitionInput::ProjectGenealogyEdge {
            source,
            destination,
            edge,
            retention_record_ref,
            maximum_publication_bytes,
        } => {
            source.record.verify()?;
            destination.record.verify()?;
            edge.record.verify()?;
            require(
                edge.record.source_occurrence_ref() == source.record_ref
                    && edge.record.source_occurrence_sha256()
                        == source.record.canonical_occurrence_sha256()
                    && edge.record.destination_occurrence_ref() == destination.record_ref
                    && edge.record.destination_occurrence_sha256()
                        == destination.record.canonical_occurrence_sha256(),
            )?;
            let source_record =
                public_record(source, source.record.canonical_occurrence_sha256().as_str())?;
            let destination_record = public_record(
                destination,
                destination.record.canonical_occurrence_sha256().as_str(),
            )?;
            let edge_record =
                public_record(edge, edge.record.canonical_genealogy_edge_sha256().as_str())?;
            let edge_key = genealogy_edge_key(edge.record)?;
            let source_key = occurrence_key(
                source.record.event_reference_ref(),
                source.record.event_reference_sha256(),
            )?;
            let destination_key = occurrence_key(
                destination.record.event_reference_ref(),
                destination.record.event_reference_sha256(),
            )?;
            let (retention, retention_index) = retain_publication(
                &edge_record,
                retention_record_ref,
                maximum_publication_bytes,
            )?;
            (
                EventCustodyTransitionKind::ProjectGenealogyEdge,
                vec![
                    member(
                        EventTransitionMemberKind::GenealogyEdge,
                        edge_record.clone(),
                    ),
                    retention,
                ],
                vec![
                    mutation(
                        EventIndexQuery::GenealogyEdge(edge_key),
                        EventIndexPredecessor::Absent,
                        &edge_record,
                    ),
                    retention_index,
                ],
                vec![
                    assertion(
                        EventIndexQuery::InteractionOccurrence(source_key),
                        source_record,
                    ),
                    assertion(
                        EventIndexQuery::InteractionOccurrence(destination_key),
                        destination_record,
                    ),
                ],
            )
        }
        EventCustodyTransitionInput::AdvanceReconciliationFrontier {
            predecessor,
            successor,
            retention_record_ref,
            maximum_publication_bytes,
        } => {
            successor.record.verify()?;
            if let Some(prior) = predecessor {
                prior.record.verify()?;
                require(
                    prior.record.subject() == successor.record.subject()
                        && prior.record.canonical_reconciliation_frontier_sha256()
                            != successor.record.canonical_reconciliation_frontier_sha256(),
                )?;
            }
            let next = public_record(
                successor,
                successor
                    .record
                    .canonical_reconciliation_frontier_sha256()
                    .as_str(),
            )?;
            let prior = predecessor
                .map(|value| {
                    public_record(
                        value,
                        value
                            .record
                            .canonical_reconciliation_frontier_sha256()
                            .as_str(),
                    )
                })
                .transpose()?;
            let key = reconciliation_frontier_key(successor.record)?;
            let (retention, retention_index) =
                retain_publication(&next, retention_record_ref, maximum_publication_bytes)?;
            (
                EventCustodyTransitionKind::AdvanceReconciliationFrontier,
                vec![
                    member(
                        EventTransitionMemberKind::ReconciliationFrontier,
                        next.clone(),
                    ),
                    retention,
                ],
                vec![
                    mutation(
                        EventIndexQuery::ReconciliationFrontier(key),
                        prior.map_or(EventIndexPredecessor::Absent, exact),
                        &next,
                    ),
                    retention_index,
                ],
                Vec::new(),
            )
        }
        EventCustodyTransitionInput::SettlePaleDerivedPublication {
            predecessor,
            successor,
        } => {
            let (retention, index) = project_pale_derived_retention(successor, Some(predecessor))?;
            (
                EventCustodyTransitionKind::SettlePaleDerivedPublication,
                vec![retention],
                vec![index],
                Vec::new(),
            )
        }
    };
    let witness_subject_kind = match kind {
        EventCustodyTransitionKind::EnqueueDelivery | EventCustodyTransitionKind::EnqueueReplay => {
            Some(EventTransitionMemberKind::DeliveryAttempt)
        }
        EventCustodyTransitionKind::RecordAcknowledgement => {
            Some(EventTransitionMemberKind::EventAcknowledgement)
        }
        EventCustodyTransitionKind::AdvanceDeliveryCursor => {
            Some(EventTransitionMemberKind::DeliveryCursor)
        }
        _ => None,
    };
    if let Some(subject_kind) = witness_subject_kind {
        let subject = members
            .iter()
            .find(|member| member.kind() == subject_kind)
            .ok_or(EventCustodyProjectionError::Lineage)?
            .record()
            .coordinate()
            .clone();
        let (witness, witness_index) =
            transition_witness(&store, journal_sequence, kind, subject_kind, &subject)?;
        members.push(witness);
        mutations.push(witness_index);
    }
    Ok(EventCustodyContent {
        store,
        kind,
        members,
        mutations,
        assertions,
    })
}

pub(super) fn subscription_key(
    subscription: &pale_core::records::EventSubscriptionIdV1,
) -> Result<EventIndexKey, EventCustodyProjectionError> {
    derive_key(SUBSCRIPTION_KEY_DOMAIN, "subscription", subscription)
}

pub(super) fn inbox_key(
    subscription: &pale_core::records::EventSubscriptionIdV1,
    identity: &CanonicalEventIdentityV1,
) -> Result<EventIndexKey, EventCustodyProjectionError> {
    derive_pair_key(
        INBOX_DELIVERY_KEY_DOMAIN,
        "inbox_delivery",
        subscription,
        identity,
    )
}

pub(super) fn delivery_attempt_key(
    attempt_id: &DeliveryAttemptIdV1,
) -> Result<EventIndexKey, EventCustodyProjectionError> {
    derive_key(DELIVERY_ATTEMPT_KEY_DOMAIN, "delivery_attempt", attempt_id)
}

pub(super) fn acknowledgement_key(
    attempt_id: &DeliveryAttemptIdV1,
) -> Result<EventIndexKey, EventCustodyProjectionError> {
    derive_key(ACKNOWLEDGEMENT_KEY_DOMAIN, "acknowledgement", attempt_id)
}

pub(super) fn delivery_stream_key(
    subscription: &pale_core::records::EventSubscriptionIdV1,
) -> Result<EventIndexKey, EventCustodyProjectionError> {
    derive_key(DELIVERY_STREAM_KEY_DOMAIN, "delivery_stream", subscription)
}

pub(super) fn occurrence_key(
    event_ref: &ResidentRecordRefV1,
    event_sha256: &pale_core::records::EventReferenceDigest,
) -> Result<EventIndexKey, EventCustodyProjectionError> {
    derive_pair_key(OCCURRENCE_KEY_DOMAIN, "occurrence", event_ref, event_sha256)
}

pub(super) fn genealogy_edge_key(
    edge: &GenealogyEdgeV1,
) -> Result<EventIndexKey, EventCustodyProjectionError> {
    genealogy_edge_key_from_parts(
        edge.edge_id(),
        edge.source_occurrence_ref(),
        edge.destination_occurrence_ref(),
        edge.proof(),
    )
}

pub(super) fn genealogy_edge_key_from_parts(
    edge_id: &GenealogyEdgeIdV1,
    source_occurrence_ref: &ResidentRecordRefV1,
    destination_occurrence_ref: &ResidentRecordRefV1,
    proof: &GenealogyProofV1,
) -> Result<EventIndexKey, EventCustodyProjectionError> {
    derive_key(
        GENEALOGY_EDGE_KEY_DOMAIN,
        "genealogy_edge",
        &(
            edge_id,
            source_occurrence_ref,
            destination_occurrence_ref,
            proof,
        ),
    )
}

pub(super) fn reconciliation_frontier_key(
    frontier: &ReconciliationFrontierV1,
) -> Result<EventIndexKey, EventCustodyProjectionError> {
    reconciliation_frontier_key_from_subject(frontier.subject())
}

pub(super) fn reconciliation_frontier_key_from_subject(
    subject: &ReconciliationSubjectV1,
) -> Result<EventIndexKey, EventCustodyProjectionError> {
    derive_key(
        RECONCILIATION_FRONTIER_KEY_DOMAIN,
        "reconciliation_frontier",
        subject,
    )
}

fn project_delivery(
    event: AdmittedEventRecord<'_, EventReferenceV1>,
    subscription: &super::AdmittedEventSubscription,
    predecessor: Option<&super::EventDeliveryState>,
    successor: &super::EventDeliveryState,
    attempt: &super::EventDeliveryAttempt,
    stream_predecessor: Option<&super::record::EventDeliveryStream>,
    stream_successor: &super::record::EventDeliveryStream,
) -> Result<TransitionProjection, EventCustodyProjectionError> {
    event.record.verify()?;
    let identity = event_identity(event.record);
    let transition_kind = match attempt.kind {
        DeliveryAttemptKind::Initial => EventCustodyTransitionKind::EnqueueDelivery,
        DeliveryAttemptKind::Replay => EventCustodyTransitionKind::EnqueueReplay,
    };
    require(
        matches!(subscription.lifecycle, super::SubscriptionLifecycle::Active)
            && successor.subscription_id == subscription.subscription_id
            && successor.event_identity == identity
            && successor.event_sha256 == *event.record.canonical_event_sha256()
            && successor.attempt_id == attempt.attempt_id
            && successor.ordinal == attempt.ordinal
            && matches!(
                successor.disposition,
                super::EventDeliveryDisposition::Pending
            )
            && attempt.subscription_id == subscription.subscription_id
            && attempt.event_identity == identity
            && attempt.event_sha256 == *event.record.canonical_event_sha256()
            && stream_successor.subscription_id == subscription.subscription_id
            && stream_successor.through_ordinal == attempt.ordinal
            && stream_successor.last_attempt_id == attempt.attempt_id
            && matches!(
                (predecessor, stream_predecessor, attempt.kind),
                (None, None | Some(_), DeliveryAttemptKind::Initial)
                    | (Some(_), Some(_), DeliveryAttemptKind::Replay)
            ),
    )?;
    match stream_predecessor {
        None => require(stream_successor.through_ordinal.get() == 1)?,
        Some(prior) => require(
            prior.subscription_id == stream_successor.subscription_id
                && prior.through_ordinal.checked_successor()
                    == Some(stream_successor.through_ordinal),
        )?,
    }
    if let Some(prior) = predecessor {
        require(
            prior.subscription_id == successor.subscription_id
                && prior.event_identity == successor.event_identity
                && prior.event_sha256 == successor.event_sha256
                && prior.attempt_id != successor.attempt_id
                && prior.ordinal.checked_successor() == Some(successor.ordinal),
        )?;
    }
    let event_record = public_record(event, event.record.canonical_event_sha256().as_str())?;
    let subscription_record = private_record(
        &subscription.record_ref,
        EventTransitionMemberKind::Subscription,
        subscription,
    )?;
    let delivery_record = private_record(
        &successor.record_ref,
        EventTransitionMemberKind::InboxDelivery,
        successor,
    )?;
    let attempt_record = private_record(
        &attempt.record_ref,
        EventTransitionMemberKind::DeliveryAttempt,
        attempt,
    )?;
    let stream_record = private_record(
        &stream_successor.record_ref,
        EventTransitionMemberKind::DeliveryStream,
        stream_successor,
    )?;
    let identity_key = event_identity_key(event.record)?;
    let subscription_key = subscription_key(&subscription.subscription_id)?;
    let inbox_key = inbox_key(&successor.subscription_id, &successor.event_identity)?;
    let attempt_key = delivery_attempt_key(&attempt.attempt_id)?;
    let stream_key = delivery_stream_key(&subscription.subscription_id)?;
    let prior = predecessor
        .map(|value| {
            private_record(
                &value.record_ref,
                EventTransitionMemberKind::InboxDelivery,
                value,
            )
        })
        .transpose()?;
    let prior_stream = stream_predecessor
        .map(|value| {
            private_record(
                &value.record_ref,
                EventTransitionMemberKind::DeliveryStream,
                value,
            )
        })
        .transpose()?;
    Ok((
        transition_kind,
        vec![
            member(
                EventTransitionMemberKind::InboxDelivery,
                delivery_record.clone(),
            ),
            member(
                EventTransitionMemberKind::DeliveryAttempt,
                attempt_record.clone(),
            ),
            member(
                EventTransitionMemberKind::DeliveryStream,
                stream_record.clone(),
            ),
        ],
        vec![
            mutation(
                EventIndexQuery::InboxDelivery(inbox_key),
                prior.map_or(EventIndexPredecessor::Absent, exact),
                &delivery_record,
            ),
            mutation(
                EventIndexQuery::DeliveryAttempt(attempt_key),
                EventIndexPredecessor::Absent,
                &attempt_record,
            ),
            mutation(
                EventIndexQuery::DeliveryStream(stream_key),
                prior_stream.map_or(EventIndexPredecessor::Absent, exact),
                &stream_record,
            ),
        ],
        vec![
            assertion(EventIndexQuery::EventIdentity(identity_key), event_record),
            assertion(
                EventIndexQuery::Subscription(subscription_key),
                subscription_record,
            ),
        ],
    ))
}

/// Used only after one of the three typed publication owners has verified and
/// projected its record. The coordinate carries that family's canonical digest,
/// not a new raw-byte hash or a substitute for contextual admission.
fn retain_publication(
    publication: &ControlRecord,
    retention_record_ref: &ResidentRecordRefV1,
    maximum_publication_bytes: usize,
) -> Result<(EventTransitionMember, EventIndexMutation), EventCustodyProjectionError> {
    require(retention_record_ref != publication.coordinate().record_ref())?;
    let publication_bytes = publication.canonical_record_jcs().as_slice().len();
    let remaining = maximum_publication_bytes
        .checked_sub(publication_bytes)
        .ok_or(EventCustodyProjectionError::PublicationBytes)?;
    let retained = super::PaleDerivedPublicationRetention::retained(
        retention_record_ref.clone(),
        publication.coordinate().record_ref().clone(),
        pale_core::records::ArtifactDigest::parse(
            publication.coordinate().record_sha256().as_str(),
        )
        .map_err(|_| EventCustodyProjectionError::Lineage)?,
    );
    let (member, index) = project_pale_derived_retention(&retained, None)?;
    if member.record().canonical_record_jcs().as_slice().len() > remaining {
        return Err(EventCustodyProjectionError::PublicationBytes);
    }
    Ok((member, index))
}

fn project_pale_derived_retention(
    successor: &super::PaleDerivedPublicationRetention,
    predecessor: Option<&super::PaleDerivedPublicationRetention>,
) -> Result<(EventTransitionMember, EventIndexMutation), EventCustodyProjectionError> {
    require(match (predecessor, &successor.disposition) {
        (None, RetentionDisposition::Retained) => true,
        (Some(prior), RetentionDisposition::Settled { .. }) => {
            matches!(prior.disposition, RetentionDisposition::Retained)
                && prior.publication_ref == successor.publication_ref
                && prior.publication_sha256 == successor.publication_sha256
        }
        _ => false,
    })?;
    let next = private_record(
        &successor.record_ref,
        EventTransitionMemberKind::PaleDerivedOutbox,
        successor,
    )?;
    let prior = predecessor
        .map(|value| {
            private_record(
                &value.record_ref,
                EventTransitionMemberKind::PaleDerivedOutbox,
                value,
            )
        })
        .transpose()?;
    let key = pale_derived_outbox_key(&successor.publication_ref, &successor.publication_sha256)?;
    Ok((
        member(EventTransitionMemberKind::PaleDerivedOutbox, next.clone()),
        mutation(
            EventIndexQuery::PaleDerivedOutbox(key),
            prior.map_or(EventIndexPredecessor::Absent, exact),
            &next,
        ),
    ))
}

pub(super) fn pale_derived_outbox_key(
    publication_ref: &ResidentRecordRefV1,
    publication_sha256: &pale_core::records::ArtifactDigest,
) -> Result<EventIndexKey, EventCustodyProjectionError> {
    derive_pair_key(
        PALE_DERIVED_OUTBOX_KEY_DOMAIN,
        "outbox_retention",
        publication_ref,
        publication_sha256,
    )
}
