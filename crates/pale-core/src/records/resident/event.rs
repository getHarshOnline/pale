//! Owner-reference delivery contracts.
//!
//! [`EventReferenceV1`] carries one committed owner reference; it does not copy
//! or reinterpret the owner's record. [`EventAcknowledgementV1`] records only a
//! consumer delivery disposition. Neither record implies a response, effect,
//! outcome, native acceptance, or settlement.

use serde::{Deserialize, Serialize};

use super::*;
use crate::records::*;

/// Public schema root for event-reference wires.
pub const EVENT_REFERENCE_V1_SCHEMA_ID: &str = "event-reference.v1";
/// Wire schema carried by every event reference.
pub const EVENT_REFERENCE_V1_SCHEMA: &str = "pale/event-reference.v1";
/// Public schema root for delivery-only event acknowledgements.
pub const EVENT_ACKNOWLEDGEMENT_V1_SCHEMA_ID: &str = "event-acknowledgement.v1";
/// Wire schema carried by every event acknowledgement.
pub const EVENT_ACKNOWLEDGEMENT_V1_SCHEMA: &str = "pale/event-acknowledgement.v1";
/// Signature domain for [`EventAcknowledgementV1`].
pub const EVENT_ACKNOWLEDGEMENT_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_EVENT_ACKNOWLEDGEMENT_V1\0";

/// Exact cross-system target scope of one event reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EventScopeV1 {
    /// Principal scope selected for delivery.
    pub principal_scope: ResidentPrincipalIdV1,
    /// Controller scope selected for delivery.
    pub controller_scope: ResidentControllerIdV1,
    /// Optional relationship scope selected for delivery.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relationship_scope: Option<RelationshipScopeV1>,
    /// Owner-defined environment coordinate.
    pub environment: DeploymentEnvironmentV1,
}

resident_integrity_record!(
    /// Bounded reference to one owner-committed immutable source record.
    EventReferenceV1, EventReferenceWireV1, EventReferencePreimageV1,
    canonical_event_sha256: EventReferenceDigest,
    EVENT_REFERENCE_V1_SCHEMA, "event reference",
    {
        /// Stable event identity within one producer incarnation.
        event_id: EventIdV1,
        /// Owner-declared semantic event type.
        event_type: EventTypeV1,
        /// Positive owner event-contract version.
        event_version: u32,
        /// Owner lifecycle namespace containing the source transition.
        lifecycle_namespace: LifecycleNamespaceV1,
        /// Principal whose producer incarnation allocated the event identity.
        origin_principal_id: ResidentPrincipalIdV1,
        /// Exact origin node.
        origin_node_id: ResidentNodeIdV1,
        /// Native system that owns the referenced record.
        source_owner: SourceOwnerV1,
        /// Immutable source-record reference.
        source_record_ref: ResidentRecordRefV1,
        /// Exact source-record schema identity.
        source_record_schema_id: SchemaId,
        /// Digest of the exact source-record schema bytes.
        source_record_schema_sha256: SchemaDigest,
        /// Digest of the exact immutable source-record bytes.
        source_record_content_sha256: ArtifactDigest,
        /// Owner-defined immutable source version.
        source_version: SourceVersionV1,
        /// Exact admitted target scope.
        target_scope: EventScopeV1,
        /// Optional immutable payload reference.
        payload_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::payload_ref`].
        payload_sha256: Option<ArtifactDigest>,
        /// Source-owner classification governing retention and disclosure.
        classification: DataClassificationV1,
        /// Optional owner-declared ordering key.
        ordering_key: Option<EventOrderingKeyV1>,
        /// Sequence paired with [`Self::ordering_key`].
        sequence: Option<OwnerEventSequenceV1>,
        /// Time at which the owner says the occurrence happened.
        occurred_at: UnixTimeV1,
        /// Time at which the immutable reference was recorded.
        recorded_at: UnixTimeV1,
        /// Correlation identity scoped to this admitted event lineage.
        correlation_id: EventCorrelationIdV1,
        /// Optional causal predecessor identity.
        causation_id: Option<EventCausationIdV1>,
        /// Principal-wide unique producer incarnation.
        producer_epoch: ProducerEpochV1,
        /// Optional delivery expiry; historical source evidence remains immutable.
        expires_at: Option<UnixTimeV1>
    },
    validate_event_reference,
    wire_transform = event_reference_schema_v1
);

/// Full canonical event identity used for deduplication and acknowledgement.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct CanonicalEventIdentityV1 {
    /// Principal that owns the producer incarnation.
    pub origin_principal_id: ResidentPrincipalIdV1,
    /// Principal-wide unique producer incarnation.
    pub producer_epoch: ProducerEpochV1,
    /// Stable event identity within that incarnation.
    pub event_id: EventIdV1,
}

/// Delivery disposition of one event-reference attempt.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum EventAcknowledgementDispositionV1 {
    /// The consumer durably admitted the event reference.
    Accepted,
    /// The consumer had already admitted the exact event identity and digest.
    Duplicate,
    /// The consumer deterministically rejected delivery.
    Rejected,
    /// The consumer retained a conflict or malformed delivery for adjudication.
    Quarantined,
}

resident_signed_record!(
    /// Signed delivery disposition for one exact event reference.
    EventAcknowledgementV1, EventAcknowledgementWireV1, EventAcknowledgementPreimageV1,
    canonical_acknowledgement_sha256: EventAcknowledgementDigest,
    EVENT_ACKNOWLEDGEMENT_V1_SCHEMA, "event acknowledgement",
    {
        /// Full event identity; bare event IDs are never sufficient.
        canonical_event_identity: CanonicalEventIdentityV1,
        /// Canonical digest of the exact event reference.
        canonical_event_sha256: EventReferenceDigest,
        /// Stable identity of this delivery attempt.
        delivery_attempt_id: DeliveryAttemptIdV1,
        /// Subscription under which delivery was attempted.
        subscription_id: EventSubscriptionIdV1,
        /// Destination principal.
        destination_principal_id: ResidentPrincipalIdV1,
        /// Optional destination node.
        destination_node_id: Option<ResidentNodeIdV1>,
        /// Exact delivery disposition.
        disposition: EventAcknowledgementDispositionV1,
        /// Required only for rejection and quarantine.
        reason_code: Option<ResidentReasonV1>,
        /// Required only for accepted and duplicate delivery.
        consumer_state_version: Option<ConsumerStateVersionV1>,
        /// Original acknowledgement reference for duplicate delivery.
        duplicate_of_ack_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::duplicate_of_ack_ref`].
        duplicate_of_ack_sha256: Option<EventAcknowledgementDigest>,
        /// Quarantine record required only for quarantined delivery.
        quarantine_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::quarantine_ref`].
        quarantine_sha256: Option<ArtifactDigest>,
        /// Unix acknowledgement time.
        acknowledged_at: UnixTimeV1,
        /// Principal whose admitted node signed the acknowledgement.
        acknowledging_principal_id: ResidentPrincipalIdV1,
        /// Admitted node that signed the acknowledgement.
        acknowledging_node_id: ResidentNodeIdV1,
        /// Active principal-key epoch checked for the node delegation.
        acknowledging_principal_key_epoch: PrincipalKeyEpochV1,
        /// Exact active node-delegation reference.
        acknowledging_node_delegation_ref: ResidentRecordRefV1,
        /// Digest of the exact node-delegation bytes.
        acknowledging_node_delegation_sha256: ArtifactDigest
    },
    validate_event_acknowledgement,
    wire_transform = event_acknowledgement_schema_v1
);

/// Owner-resolved signer and route coordinates for one acknowledgement.
#[derive(Clone, Copy)]
pub struct EventAcknowledgementAdmissionContextV1<'a> {
    /// Digest of the exact selected acknowledgement schema.
    pub expected_schema_sha256: &'a SchemaDigest,
    /// Exact admitted destination principal.
    pub destination_principal_id: &'a ResidentPrincipalIdV1,
    /// Destination node when the delivery route names one explicitly.
    pub destination_node_id: Option<&'a ResidentNodeIdV1>,
    /// Exact admitted node that signed the acknowledgement.
    pub acknowledging_node_id: &'a ResidentNodeIdV1,
    /// Active principal key epoch covering the node delegation.
    pub principal_key_epoch: PrincipalKeyEpochV1,
    /// Exact active node-delegation reference.
    pub node_delegation_ref: &'a ResidentRecordRefV1,
    /// Digest of the exact node-delegation bytes.
    pub node_delegation_sha256: &'a ArtifactDigest,
}

impl EventReferenceV1 {
    /// Issue an Event reference after admitting the exact owner publication.
    ///
    /// The source callback must prove the immutable owner record and producer
    /// publication represented by the preimage. This method does not retain,
    /// deliver, acknowledge, or settle the Event.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] for invalid structure, a mismatched
    /// schema coordinate, or rejected source publication evidence.
    pub fn issue_contextual<F>(
        preimage: EventReferencePreimageV1,
        expected_schema_sha256: &SchemaDigest,
        admit_source: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(&EventReferencePreimageV1) -> Result<(), InvalidResidentRecordV1>,
    {
        if preimage.schema_id.as_str() != EVENT_REFERENCE_V1_SCHEMA
            || preimage.schema_sha256 != *expected_schema_sha256
        {
            return Err(InvalidResidentRecordV1::SchemaCoordinate {
                family: "event reference",
            });
        }
        validate_event_reference(&preimage)?;
        admit_source(&preimage)?;
        Self::seal(preimage)
    }

    /// Admit an event reference after resolving its owner-commit publication.
    ///
    /// `admit_source` must verify the exact source schema/ref/hash and producer
    /// incarnation against the native owner's qualified publication path.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] when structure, digest, selected
    /// schema, or owner-publication admission fails.
    pub fn admit_contextual_wire<F>(
        wire: EventReferenceWireV1,
        expected_schema_sha256: &SchemaDigest,
        admit_source: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(&EventReferencePreimageV1) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, _digest| {
            if preimage.schema_sha256 != *expected_schema_sha256 {
                return Err(InvalidResidentRecordV1::SchemaCoordinate {
                    family: "event reference",
                });
            }
            admit_source(preimage)
        })
    }
}

impl EventAcknowledgementV1 {
    /// Admit a delivery acknowledgement against the exact destination node.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] for structural, digest,
    /// node-delegation, destination, or signature failure.
    pub fn admit_contextual_wire<F>(
        wire: EventAcknowledgementWireV1,
        context: &EventAcknowledgementAdmissionContextV1<'_>,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &[u8],
            &EventAcknowledgementPreimageV1,
            &EventAcknowledgementDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            if preimage.schema_sha256 != *context.expected_schema_sha256
                || preimage.destination_principal_id != *context.destination_principal_id
                || preimage.destination_node_id.as_ref() != context.destination_node_id
                || preimage.acknowledging_principal_id != *context.destination_principal_id
                || preimage.acknowledging_node_id != *context.acknowledging_node_id
                || preimage.acknowledging_principal_key_epoch != context.principal_key_epoch
                || preimage.acknowledging_node_delegation_ref != *context.node_delegation_ref
                || preimage.acknowledging_node_delegation_sha256 != *context.node_delegation_sha256
            {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "event acknowledgement",
                    invariant: "exact destination node and active node delegation",
                });
            }
            verify_signature(
                EVENT_ACKNOWLEDGEMENT_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }
}

fn validate_event_reference(
    value: &EventReferencePreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    if value.event_version == 0
        || value.payload_ref.is_some() != value.payload_sha256.is_some()
        || value.ordering_key.is_some() != value.sequence.is_some()
        || value.recorded_at < value.occurred_at
        || value
            .expires_at
            .is_some_and(|expires| expires < value.recorded_at)
    {
        Err(InvalidResidentRecordV1::InvalidCoordinate {
            coordinate: "event reference",
        })
    } else {
        Ok(())
    }
}

fn validate_event_acknowledgement(
    value: &EventAcknowledgementPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    let duplicate_pair =
        value.duplicate_of_ack_ref.is_some() && value.duplicate_of_ack_sha256.is_some();
    let duplicate_absent =
        value.duplicate_of_ack_ref.is_none() && value.duplicate_of_ack_sha256.is_none();
    let quarantine_pair = value.quarantine_ref.is_some() && value.quarantine_sha256.is_some();
    let quarantine_absent = value.quarantine_ref.is_none() && value.quarantine_sha256.is_none();
    if (!duplicate_pair && !duplicate_absent) || (!quarantine_pair && !quarantine_absent) {
        return Err(InvalidResidentRecordV1::InvalidCoordinate {
            coordinate: "event acknowledgement optional evidence",
        });
    }
    let valid = match value.disposition {
        EventAcknowledgementDispositionV1::Accepted => {
            value.consumer_state_version.is_some()
                && value.reason_code.is_none()
                && duplicate_absent
                && quarantine_absent
        }
        EventAcknowledgementDispositionV1::Duplicate => {
            value.consumer_state_version.is_some()
                && value.reason_code.is_none()
                && duplicate_pair
                && quarantine_absent
        }
        EventAcknowledgementDispositionV1::Rejected => {
            value.reason_code.is_some()
                && value.consumer_state_version.is_none()
                && duplicate_absent
                && quarantine_absent
        }
        EventAcknowledgementDispositionV1::Quarantined => {
            value.reason_code.is_some()
                && value.consumer_state_version.is_none()
                && duplicate_absent
                && quarantine_pair
        }
    };
    if valid {
        Ok(())
    } else {
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "event acknowledgement",
            invariant: "delivery-disposition branch",
        })
    }
}

fn event_reference_schema_v1(schema: &mut schemars::Schema) {
    require_all_or_none_properties_v1(schema, &["payload_ref", "payload_sha256"]);
    require_all_or_none_properties_v1(schema, &["ordering_key", "sequence"]);
}

fn event_acknowledgement_schema_v1(schema: &mut schemars::Schema) {
    require_all_or_none_properties_v1(schema, &["duplicate_of_ack_ref", "duplicate_of_ack_sha256"]);
    require_all_or_none_properties_v1(schema, &["quarantine_ref", "quarantine_sha256"]);
}
