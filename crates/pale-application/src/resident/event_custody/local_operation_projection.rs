//! Exact projection of one admitted local operation observation.
//!
//! The projector translates already admitted Pale-owned operation evidence into
//! the existing Event, interaction, Genealogy, and reconciliation contracts.
//! It does not read storage, append custody, deliver an Event, or infer a native

//! effect or settlement from an operation state.

use pale_core::{canon, records::*, sha256_hex};
use pale_ports::{
    AuthorityStoreCoordinate, EventIndexKey, OperationPublicationSource,
    OperationPublicationSourceKind,
};
use serde::Serialize;

use crate::resident::{
    IssuedLocalOperation, RecoveredLocalOperation, RecoveredOperationObservation,
};

const PALE_OWNER: &str = "pale";
const EVENT_TYPE: &str = "pale.operation.observation.committed";
const LIFECYCLE_NAMESPACE: &str = "pale.operation.observation";
const SUCCESSOR_RELATION: &str = "successor";
const INVOKE_RELATION: &str = "invoke";
const CONSULT_RELATION: &str = "consult";
const DELEGATE_RELATION: &str = "delegate";

/// Untrusted policy candidate for one local operation publication.
pub struct LocalOperationPublicationPolicyInput<'a> {
    /// Exact admitted target for this publication.
    pub target_scope: &'a EventScopeV1,
    /// Source classification retained by the Event and occurrence.
    pub classification: &'a DataClassificationV1,
    /// Owner-selected interaction visibility.
    pub visibility: &'a InteractionVisibilityV1,
    /// Maximum disclosure allowed for the interaction projection.
    pub disclosure_ceiling: &'a InteractionDisclosureCeilingV1,
    /// Policy governing later payload resolution.
    pub payload_resolution_policy: &'a PayloadResolutionPolicyV1,
    /// Exact owner record admitted as the interaction actor.
    pub actor_provenance: &'a OwnerRecordCoordinateV1,
}

/// Application-admitted disclosure and placement policy for one publication.
///
/// Operation custody does not determine who may receive the publication, who
/// the interaction actor is, or how a referenced payload may be resolved. The
/// private fields prevent an unreviewed struct literal from widening those
/// meanings at the projector boundary.
#[derive(Clone)]
pub struct LocalOperationPublicationPolicy {
    target_scope: EventScopeV1,
    classification: DataClassificationV1,
    visibility: InteractionVisibilityV1,
    disclosure_ceiling: InteractionDisclosureCeilingV1,
    payload_resolution_policy: PayloadResolutionPolicyV1,
    actor_provenance: OwnerRecordCoordinateV1,
}

impl LocalOperationPublicationPolicy {
    /// Seal policy facts recovered through their application-owned admission path.
    ///
    /// This constructor is restricted to the resident application. It does not
    /// make untrusted wire input admissible and performs no storage or effect.
    pub(in crate::resident) fn from_admitted_input(
        input: LocalOperationPublicationPolicyInput<'_>,
    ) -> Self {
        Self {
            target_scope: input.target_scope.clone(),
            classification: input.classification.clone(),
            visibility: input.visibility.clone(),
            disclosure_ceiling: input.disclosure_ceiling.clone(),
            payload_resolution_policy: input.payload_resolution_policy.clone(),
            actor_provenance: input.actor_provenance.clone(),
        }
    }

    /// Borrow the sealed policy through the existing projection input shape.
    pub(in crate::resident) const fn as_input(&self) -> LocalOperationPublicationPolicyInput<'_> {
        LocalOperationPublicationPolicyInput {
            target_scope: &self.target_scope,
            classification: &self.classification,
            visibility: &self.visibility,
            disclosure_ceiling: &self.disclosure_ceiling,
            payload_resolution_policy: &self.payload_resolution_policy,
            actor_provenance: &self.actor_provenance,
        }
    }

    pub(in crate::resident) const fn target_scope(&self) -> &EventScopeV1 {
        &self.target_scope
    }

    pub(in crate::resident) const fn classification(&self) -> &DataClassificationV1 {
        &self.classification
    }

    pub(in crate::resident) const fn visibility(&self) -> &InteractionVisibilityV1 {
        &self.visibility
    }

    pub(in crate::resident) const fn disclosure_ceiling(&self) -> &InteractionDisclosureCeilingV1 {
        &self.disclosure_ceiling
    }

    pub(in crate::resident) const fn payload_resolution_policy(
        &self,
    ) -> &PayloadResolutionPolicyV1 {
        &self.payload_resolution_policy
    }

    pub(in crate::resident) const fn actor_provenance(&self) -> &OwnerRecordCoordinateV1 {
        &self.actor_provenance
    }
}

/// Deterministic immutable references for one observation's public projection.
///
/// The references are derived from the exact authority store and observation.
/// Restarted publication therefore cannot allocate another identity for the
/// same source. They remain locators only and prove no custody or delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalOperationPublicationReferences {
    event: ResidentRecordRefV1,
    occurrence: ResidentRecordRefV1,
    occurrence_retention: ResidentRecordRefV1,
    edge: Option<ResidentRecordRefV1>,
    edge_retention: Option<ResidentRecordRefV1>,
    frontier: ResidentRecordRefV1,
    frontier_retention: ResidentRecordRefV1,
}

impl LocalOperationPublicationReferences {
    /// Derive the complete reference set from immutable source evidence.
    ///
    /// # Errors
    /// Returns an encoding or coordinate error when the exact source cannot
    /// produce canonical initial-v1 reference identities.
    pub fn derive(
        store: &AuthorityStoreCoordinate,
        observation_ref: &ResidentRecordRefV1,
        observation: &OperationObservationV1,
    ) -> Result<Self, LocalOperationPublicationProjectionError> {
        Self::derive_for_source(
            "pale-local-operation",
            "operation-publication",
            store,
            observation_ref,
            observation
                .canonical_operation_observation_sha256()
                .as_str(),
            observation.predecessor_ref().is_some(),
        )
    }

    /// Derive references for an observation and its admitted operation handle.
    ///
    /// A child operation's revision-one observation owns one structural lineage
    /// edge even though the observation itself has no predecessor. Successor
    /// revisions retain the existing one-edge predecessor projection.
    pub(super) fn derive_for_operation(
        store: &AuthorityStoreCoordinate,
        observation_ref: &ResidentRecordRefV1,
        observation: &OperationObservationV1,
        handle: &OperationHandleV1,
    ) -> Result<Self, LocalOperationPublicationProjectionError> {
        let has_edge = observation.predecessor_ref().is_some()
            || (observation.revision().get() == 1 && handle.parent_operation().is_some());
        Self::derive_for_source(
            "pale-local-operation",
            "operation-publication",
            store,
            observation_ref,
            observation
                .canonical_operation_observation_sha256()
                .as_str(),
            has_edge,
        )
    }

    pub(super) fn derive_for_source(
        identity_domain: &str,
        reference_namespace: &str,
        store: &AuthorityStoreCoordinate,
        source_ref: &ResidentRecordRefV1,
        source_sha256: &str,
        has_predecessor: bool,
    ) -> Result<Self, LocalOperationPublicationProjectionError> {
        let identity = |kind: &str| {
            derived_id_from_digest(
                &format!("{identity_domain}-{kind}-v1"),
                store,
                source_ref,
                source_sha256,
            )
        };
        let record_ref = |kind: &str, value: &str| {
            ResidentRecordRefV1::parse(format!("pale/{reference_namespace}/{kind}/{value}"))
        };
        let event_id = identity("event")?;
        let occurrence_id = identity("occurrence")?;
        let occurrence_retention_id = identity("occurrence-retention")?;
        let edge_id = identity("edge")?;
        let edge_retention_id = identity("edge-retention")?;
        let frontier_id = identity("frontier")?;
        let frontier_retention_id = identity("frontier-retention")?;
        Ok(Self {
            event: record_ref("event", &event_id)?,
            occurrence: record_ref("occurrence", &occurrence_id)?,
            occurrence_retention: record_ref("occurrence-retention", &occurrence_retention_id)?,
            edge: has_predecessor
                .then(|| record_ref("edge", &edge_id))
                .transpose()?,
            edge_retention: has_predecessor
                .then(|| record_ref("edge-retention", &edge_retention_id))
                .transpose()?,
            frontier: record_ref("frontier", &frontier_id)?,
            frontier_retention: record_ref("frontier-retention", &frontier_retention_id)?,
        })
    }

    pub(super) const fn event(&self) -> &ResidentRecordRefV1 {
        &self.event
    }

    pub(super) const fn occurrence(&self) -> &ResidentRecordRefV1 {
        &self.occurrence
    }

    pub(super) const fn occurrence_retention(&self) -> &ResidentRecordRefV1 {
        &self.occurrence_retention
    }

    pub(super) fn edge(&self) -> Option<&ResidentRecordRefV1> {
        self.edge.as_ref()
    }

    pub(super) fn edge_retention(&self) -> Option<&ResidentRecordRefV1> {
        self.edge_retention.as_ref()
    }

    pub(super) const fn frontier(&self) -> &ResidentRecordRefV1 {
        &self.frontier
    }

    pub(super) const fn frontier_retention(&self) -> &ResidentRecordRefV1 {
        &self.frontier_retention
    }
}

/// Admitted predecessor evidence required to publish a successor edge.
///
/// The predecessor observation proves the handle-local signed lineage. The
/// Event and occurrence prove that the exact predecessor was already projected.
pub struct LocalOperationPredecessorPublication {
    /// Exact predecessor observation and immutable reference.
    observation: (ResidentRecordRefV1, OperationObservationV1),
    /// Event previously derived from that observation.
    event: (ResidentRecordRefV1, EventReferenceV1),
    /// Occurrence previously derived from that Event.
    occurrence: (ResidentRecordRefV1, InteractionOccurrenceV1),
}

impl LocalOperationPredecessorPublication {
    /// Re-admit one exact predecessor projection before it can source a new edge.
    ///
    /// This admission verifies the predecessor's source Event, its complete
    /// local-operation projection meaning, and its historical runtime signature.
    /// It performs no storage write, signing, delivery, or native effect.
    ///
    /// # Errors
    /// Returns [`LocalOperationPublicationProjectionError`] when the recovered
    /// source, deterministic references, policy, runtime authority, signature,
    /// or complete Event/occurrence meaning differs.
    #[allow(clippy::too_many_arguments)]
    pub fn admit<EventAdmission, OccurrenceVerification>(
        store: &AuthorityStoreCoordinate,
        recovered: &RecoveredOperationObservation,
        operation: &IssuedLocalOperation,
        event: (&ResidentRecordRefV1, &EventReferenceV1),
        occurrence: (&ResidentRecordRefV1, &InteractionOccurrenceV1),
        occurrence_context: SignedResidentAdmissionContextV1<'_>,
        admit_event_source: EventAdmission,
        verify_occurrence_signature: OccurrenceVerification,
    ) -> Result<Self, LocalOperationPublicationProjectionError>
    where
        EventAdmission: FnOnce(&EventReferencePreimageV1) -> Result<(), InvalidResidentRecordV1>,
        OccurrenceVerification: FnOnce(
            &[u8],
            &InteractionOccurrencePreimageV1,
            &InteractionOccurrenceDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        admit_local_operation_predecessor_publication_from_parts(
            store,
            recovered.store(),
            operation.handle(),
            recovered.selected(),
            recovered.selected_checkpoint_record(),
            operation.publication_policy(),
            event,
            occurrence,
            occurrence_context,
            admit_event_source,
            verify_occurrence_signature,
        )
    }
}

/// Current schema and signer contexts for the three signed derived records.
///
/// These contexts are deliberately absent from
/// [`AdmittedLocalOperationPublicationPlan`]. A restarted publication uses the
/// exact retained predecessor and the current admitted signer only for the
/// first stage that is still absent.
pub struct LocalOperationPublicationSigningContexts<'a> {
    /// Context for the interaction occurrence.
    pub occurrence: SignedResidentAdmissionContextV1<'a>,
    /// Context for the optional Genealogy edge.
    pub edge: SignedResidentAdmissionContextV1<'a>,
    /// Context for the reconciliation frontier.
    pub frontier: SignedResidentAdmissionContextV1<'a>,
}

/// Signer-free publication meaning admitted from one exact operation observation.
///
/// The plan contains no signed derived record. Restart therefore cannot replace
/// a retained predecessor with bytes issued by a newer key. Each missing stage
/// is issued only after all earlier stages have been read and readmitted.
pub struct AdmittedLocalOperationPublicationPlan {
    pub(super) event_meaning: LocalOperationEventMeaning,
    pub(super) owner_source_key: EventIndexKey,
    pub(super) references: LocalOperationPublicationReferences,
    pub(super) deployment_runtime_authority_ref: ResidentRecordRefV1,
    pub(super) deployment_runtime_authority_sha256: ArtifactDigest,
    pub(super) occurrence_id: InteractionOccurrenceIdV1,
    pub(super) handle_coordinate: OwnerRecordCoordinateV1,
    pub(super) parent_placement: OwnerRecordKnowledgeV1,
    pub(super) edge_source_occurrence: Option<(ResidentRecordRefV1, InteractionOccurrenceV1)>,
    pub(super) edge_proof: Option<GenealogyProofV1>,
    pub(super) edge_id: Option<GenealogyEdgeIdV1>,
    pub(super) observation_coordinate: OwnerRecordCoordinateV1,
    pub(super) frontier_id: ReconciliationFrontierIdV1,
    pub(super) checkpoint_ref: ResidentRecordRefV1,
    pub(super) checkpoint_bytes_sha256: ArtifactDigest,
    pub(super) observation_ref: ResidentRecordRefV1,
    pub(super) observation_bytes_sha256: ArtifactDigest,
    pub(super) policy: LocalOperationPublicationPolicy,
}

impl AdmittedLocalOperationPublicationPlan {
    pub(super) const fn event_ref(&self) -> &ResidentRecordRefV1 {
        &self.references.event
    }

    pub(super) const fn owner_source_key(&self) -> &EventIndexKey {
        &self.owner_source_key
    }

    pub(super) const fn references(&self) -> &LocalOperationPublicationReferences {
        &self.references
    }

    pub(super) fn issue_occurrence<F>(
        &self,
        event: &EventReferenceV1,
        context: SignedResidentAdmissionContextV1<'_>,
        sign: F,
    ) -> Result<InteractionOccurrenceV1, LocalOperationPublicationProjectionError>
    where
        F: FnOnce(
            &[u8],
            &InteractionOccurrencePreimageV1,
            &InteractionOccurrenceDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.admit_runtime_context(event.origin_principal_id(), &context)?;
        let schema = pale_contracts::record_schema_identity(INTERACTION_OCCURRENCE_V1_SCHEMA_ID)?;
        Ok(InteractionOccurrenceV1::issue_contextual(
            self.occurrence_preimage(event, schema.schema, schema.schema_sha256, &context)?,
            &context,
            |_| Ok(()),
            sign,
        )?)
    }

    pub(super) fn issue_edge<F>(
        &self,
        destination: &InteractionOccurrenceV1,
        context: SignedResidentAdmissionContextV1<'_>,
        sign: F,
    ) -> Result<Option<GenealogyEdgeV1>, LocalOperationPublicationProjectionError>
    where
        F: FnOnce(
            &[u8],
            &GenealogyEdgePreimageV1,
            &GenealogyEdgeDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.admit_runtime_context(destination.issuer_principal_id(), &context)?;
        let Some(preimage) = self.edge_preimage(destination, &context)? else {
            return Ok(None);
        };
        Ok(Some(GenealogyEdgeV1::issue_contextual(
            preimage,
            &context,
            |_| Ok(()),
            sign,
        )?))
    }

    pub(super) fn issue_frontier<F>(
        &self,
        occurrence: &InteractionOccurrenceV1,
        context: SignedResidentAdmissionContextV1<'_>,
        sign: F,
    ) -> Result<ReconciliationFrontierV1, LocalOperationPublicationProjectionError>
    where
        F: FnOnce(
            &[u8],
            &ReconciliationFrontierPreimageV1,
            &ReconciliationFrontierDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.admit_runtime_context(occurrence.issuer_principal_id(), &context)?;
        let schema = pale_contracts::record_schema_identity(RECONCILIATION_FRONTIER_V1_SCHEMA_ID)?;
        Ok(ReconciliationFrontierV1::issue_contextual(
            self.frontier_preimage(occurrence, schema.schema, schema.schema_sha256, &context)?,
            &context,
            |_| Ok(()),
            sign,
        )?)
    }

    pub(super) fn event_meaning_matches(&self, event: &EventReferenceV1) -> bool {
        self.event_meaning.matches(event)
    }

    pub(super) fn issue_event<F>(
        &self,
        producer: &ProducerIncarnationV1,
        writer_fence: &PrincipalResidentWriterFenceV1,
        admit_source: F,
    ) -> Result<EventReferenceV1, LocalOperationPublicationProjectionError>
    where
        F: FnOnce(&EventReferencePreimageV1) -> Result<(), InvalidResidentRecordV1>,
    {
        if producer.origin_principal_id() != &self.event_meaning.origin_principal_id
            || writer_fence.principal_id() != &self.event_meaning.origin_principal_id
            || writer_fence.state() != &PrincipalResidentWriterFenceStateV1::Active
        {
            return Err(LocalOperationPublicationProjectionError::Association);
        }
        let recorded_at = self
            .event_meaning
            .minimum_recorded_at
            .max(*producer.allocated_at());
        Ok(EventReferenceV1::issue_contextual(
            self.event_meaning.preimage(
                writer_fence.primary_node_id().clone(),
                recorded_at,
                producer.producer_epoch().clone(),
            ),
            &self.event_meaning.schema_sha256,
            admit_source,
        )?)
    }

    pub(super) fn occurrence_preimage(
        &self,
        event: &EventReferenceV1,
        schema_id: SchemaId,
        schema_sha256: SchemaDigest,
        context: &SignedResidentAdmissionContextV1<'_>,
    ) -> Result<InteractionOccurrencePreimageV1, LocalOperationPublicationProjectionError> {
        operation_occurrence_preimage(
            self.occurrence_id.clone(),
            &self.references.event,
            event,
            &self.handle_coordinate,
            self.parent_placement.clone(),
            &self.policy,
            *event.recorded_at(),
            schema_id,
            schema_sha256,
            context,
        )
    }

    pub(super) fn edge_preimage(
        &self,
        destination: &InteractionOccurrenceV1,
        context: &SignedResidentAdmissionContextV1<'_>,
    ) -> Result<Option<GenealogyEdgePreimageV1>, LocalOperationPublicationProjectionError> {
        let (source_ref, source) = match &self.edge_source_occurrence {
            Some(source) => source,
            None => return Ok(None),
        };
        let edge_id = self
            .edge_id
            .clone()
            .ok_or(LocalOperationPublicationProjectionError::MissingPredecessor)?;
        let schema = pale_contracts::record_schema_identity(GENEALOGY_EDGE_V1_SCHEMA_ID)?;
        let runtime_authority = context
            .signing_authority
            .integrated_runtime_authority()
            .ok_or(LocalOperationPublicationProjectionError::Association)?;
        Ok(Some(GenealogyEdgePreimageV1 {
            schema_id: schema.schema,
            schema_sha256: schema.schema_sha256,
            deployment_runtime_authority_ref: runtime_authority
                .deployment_runtime_authority_ref
                .clone(),
            deployment_runtime_authority_sha256: runtime_authority
                .deployment_runtime_authority_sha256
                .clone(),
            edge_id,
            source_occurrence_ref: source_ref.clone(),
            source_occurrence_sha256: source.canonical_occurrence_sha256().clone(),
            destination_occurrence_ref: self.references.occurrence.clone(),
            destination_occurrence_sha256: destination.canonical_occurrence_sha256().clone(),
            proof: self
                .edge_proof
                .clone()
                .ok_or(LocalOperationPublicationProjectionError::MissingPredecessor)?,
            admitted_at: *destination.projected_at(),
            issuer_principal_id: context.signing_authority.principal_id().clone(),
            issuer_key_id: ResidentKeyIdV1::parse(context.signing_authority.signer_key_id())?,
            issuer_key_epoch: *context.signing_authority.signer_key_epoch(),
        }))
    }

    pub(super) fn frontier_preimage(
        &self,
        occurrence: &InteractionOccurrenceV1,
        schema_id: SchemaId,
        schema_sha256: SchemaDigest,
        context: &SignedResidentAdmissionContextV1<'_>,
    ) -> Result<ReconciliationFrontierPreimageV1, LocalOperationPublicationProjectionError> {
        let occurrence_bytes_sha256 = ArtifactDigest::parse(sha256_hex(
            canon::to_canonical_string(occurrence)?.as_bytes(),
        ))?;
        let runtime_authority = context
            .signing_authority
            .integrated_runtime_authority()
            .ok_or(LocalOperationPublicationProjectionError::Association)?;
        Ok(ReconciliationFrontierPreimageV1 {
            schema_id,
            schema_sha256,
            deployment_runtime_authority_ref: runtime_authority
                .deployment_runtime_authority_ref
                .clone(),
            deployment_runtime_authority_sha256: runtime_authority
                .deployment_runtime_authority_sha256
                .clone(),
            frontier_id: self.frontier_id.clone(),
            subject: ReconciliationSubjectV1::Source {
                coordinate: self.observation_coordinate.clone(),
            },
            dimensions: vec![ReconciliationDimensionEntryV1 {
                dimension: ReconciliationDimensionV1::OwnerPublication,
                status: ReconciliationDimensionStatusV1::Complete,
                evidence: Some(AuthorityEvidenceV1::new(
                    self.checkpoint_ref.clone(),
                    self.checkpoint_bytes_sha256.clone(),
                )),
            }],
            source_occurrence_refs: vec![ImmutableRecordReferenceV1 {
                record_ref: self.references.occurrence.clone(),
                record_sha256: occurrence_bytes_sha256,
            }],
            owner_record_revisions: vec![FrontierOwnerRevisionV1 {
                owner_system_id: OwnerSystemIdV1::parse(PALE_OWNER)?,
                record_ref: self.observation_ref.clone(),
                record_sha256: self.observation_bytes_sha256.clone(),
                native_revision: self.observation_coordinate.native_revision.clone(),
            }],
            generated_at: *occurrence.projected_at(),
            issuer_principal_id: context.signing_authority.principal_id().clone(),
            issuer_key_id: ResidentKeyIdV1::parse(context.signing_authority.signer_key_id())?,
            issuer_key_epoch: *context.signing_authority.signer_key_epoch(),
        })
    }

    pub(super) fn edge_source_occurrence(
        &self,
    ) -> Option<(&ResidentRecordRefV1, &InteractionOccurrenceV1)> {
        self.edge_source_occurrence
            .as_ref()
            .map(|(reference, value)| (reference, value))
    }

    pub(super) fn edge_identity(
        &self,
    ) -> Option<(
        &GenealogyEdgeIdV1,
        &ResidentRecordRefV1,
        &ResidentRecordRefV1,
        GenealogyProofV1,
    )> {
        let (source_ref, _) = self.edge_source_occurrence.as_ref()?;
        Some((
            self.edge_id.as_ref()?,
            source_ref,
            &self.references.occurrence,
            self.edge_proof.clone()?,
        ))
    }

    pub(super) fn frontier_subject(&self) -> ReconciliationSubjectV1 {
        ReconciliationSubjectV1::Source {
            coordinate: self.observation_coordinate.clone(),
        }
    }

    pub(super) fn admit_runtime_context(
        &self,
        origin_principal_id: &ResidentPrincipalIdV1,
        context: &SignedResidentAdmissionContextV1<'_>,
    ) -> Result<(), LocalOperationPublicationProjectionError> {
        admit_runtime_context(
            origin_principal_id,
            &self.deployment_runtime_authority_ref,
            &self.deployment_runtime_authority_sha256,
            context,
        )
    }
}

/// Producer-independent Event meaning retained by an admitted publication plan.
///
/// The writer node, producer epoch, and recording time are issuance evidence.
/// They are deliberately absent so a restarted producer can discover and
/// readmit an Event issued by its predecessor.
pub(super) struct LocalOperationEventMeaning {
    pub(super) schema_id: SchemaId,
    pub(super) schema_sha256: SchemaDigest,
    pub(super) event_id: EventIdV1,
    pub(super) event_type: EventTypeV1,
    pub(super) event_version: u32,
    pub(super) lifecycle_namespace: LifecycleNamespaceV1,
    pub(super) origin_principal_id: ResidentPrincipalIdV1,
    pub(super) source_owner: SourceOwnerV1,
    pub(super) source_record_ref: ResidentRecordRefV1,
    pub(super) source_record_schema_id: SchemaId,
    pub(super) source_record_schema_sha256: SchemaDigest,
    pub(super) source_record_content_sha256: ArtifactDigest,
    pub(super) source_version: SourceVersionV1,
    pub(super) target_scope: EventScopeV1,
    pub(super) classification: DataClassificationV1,
    pub(super) ordering_key: EventOrderingKeyV1,
    pub(super) sequence: OwnerEventSequenceV1,
    pub(super) occurred_at: UnixTimeV1,
    pub(super) minimum_recorded_at: UnixTimeV1,
    pub(super) correlation_id: EventCorrelationIdV1,
    pub(super) causation_id: Option<EventCausationIdV1>,
}

impl LocalOperationEventMeaning {
    pub(super) fn preimage(
        &self,
        origin_node_id: ResidentNodeIdV1,
        recorded_at: UnixTimeV1,
        producer_epoch: ProducerEpochV1,
    ) -> EventReferencePreimageV1 {
        EventReferencePreimageV1 {
            schema_id: self.schema_id.clone(),
            schema_sha256: self.schema_sha256.clone(),
            event_id: self.event_id.clone(),
            event_type: self.event_type.clone(),
            event_version: self.event_version,
            lifecycle_namespace: self.lifecycle_namespace.clone(),
            origin_principal_id: self.origin_principal_id.clone(),
            origin_node_id,
            source_owner: self.source_owner.clone(),
            source_record_ref: self.source_record_ref.clone(),
            source_record_schema_id: self.source_record_schema_id.clone(),
            source_record_schema_sha256: self.source_record_schema_sha256.clone(),
            source_record_content_sha256: self.source_record_content_sha256.clone(),
            source_version: self.source_version.clone(),
            target_scope: self.target_scope.clone(),
            payload_ref: None,
            payload_sha256: None,
            classification: self.classification.clone(),
            ordering_key: Some(self.ordering_key.clone()),
            sequence: Some(self.sequence),
            occurred_at: self.occurred_at,
            recorded_at,
            correlation_id: self.correlation_id.clone(),
            causation_id: self.causation_id.clone(),
            producer_epoch,
            expires_at: None,
        }
    }

    pub(super) fn matches(&self, event: &EventReferenceV1) -> bool {
        event.schema_id() == &self.schema_id
            && event.schema_sha256() == &self.schema_sha256
            && event.event_id() == &self.event_id
            && event.event_type() == &self.event_type
            && *event.event_version() == self.event_version
            && event.lifecycle_namespace() == &self.lifecycle_namespace
            && event.origin_principal_id() == &self.origin_principal_id
            && event.source_owner() == &self.source_owner
            && event.source_record_ref() == &self.source_record_ref
            && event.source_record_schema_id() == &self.source_record_schema_id
            && event.source_record_schema_sha256() == &self.source_record_schema_sha256
            && event.source_record_content_sha256() == &self.source_record_content_sha256
            && event.source_version() == &self.source_version
            && event.target_scope() == &self.target_scope
            && event.payload_ref().is_none()
            && event.payload_sha256().is_none()
            && event.classification() == &self.classification
            && event.ordering_key().as_ref() == Some(&self.ordering_key)
            && event.sequence().as_ref() == Some(&self.sequence)
            && event.occurred_at() == &self.occurred_at
            && event.recorded_at() >= &self.minimum_recorded_at
            && event.correlation_id() == &self.correlation_id
            && event.causation_id() == &self.causation_id
            && event.expires_at().is_none()
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn operation_occurrence_preimage(
    occurrence_id: InteractionOccurrenceIdV1,
    event_reference_ref: &ResidentRecordRefV1,
    event: &EventReferenceV1,
    handle_coordinate: &OwnerRecordCoordinateV1,
    parent_placement: OwnerRecordKnowledgeV1,
    policy: &LocalOperationPublicationPolicy,
    projected_at: UnixTimeV1,
    schema_id: SchemaId,
    schema_sha256: SchemaDigest,
    context: &SignedResidentAdmissionContextV1<'_>,
) -> Result<InteractionOccurrencePreimageV1, LocalOperationPublicationProjectionError> {
    let runtime_authority = context
        .signing_authority
        .integrated_runtime_authority()
        .ok_or(LocalOperationPublicationProjectionError::Association)?;
    Ok(InteractionOccurrencePreimageV1 {
        schema_id,
        schema_sha256,
        deployment_runtime_authority_ref: runtime_authority
            .deployment_runtime_authority_ref
            .clone(),
        deployment_runtime_authority_sha256: runtime_authority
            .deployment_runtime_authority_sha256
            .clone(),
        occurrence_id,
        event_reference_ref: event_reference_ref.clone(),
        event_reference_sha256: event.canonical_event_sha256().clone(),
        native_container: handle_coordinate.clone(),
        container_genre: NativeContainerGenreV1::parse("operation")?,
        root_placement: OwnerRecordKnowledgeV1::Unknown,
        thread_placement: OwnerRecordKnowledgeV1::Unknown,
        parent_placement,
        referenced_records: Vec::new(),
        actor_provenance: policy.actor_provenance.clone(),
        participant_snapshot: OwnerRecordKnowledgeV1::Unknown,
        audience_snapshot: OwnerRecordKnowledgeV1::Unknown,
        visibility: policy.visibility.clone(),
        disclosure_ceiling: policy.disclosure_ceiling.clone(),
        mention_and_cc_records: Vec::new(),
        response_expectation: InteractionResponseExpectationV1::Unknown,
        lifecycle: InteractionLifecycleProjectionV1::Unknown,
        classification: policy.classification.clone(),
        payload_resolution_policy: policy.payload_resolution_policy.clone(),
        projected_at,
        issuer_principal_id: context.signing_authority.principal_id().clone(),
        issuer_key_id: ResidentKeyIdV1::parse(context.signing_authority.signer_key_id())?,
        issuer_key_epoch: *context.signing_authority.signer_key_epoch(),
    })
}

fn operation_handle_coordinate(
    handle_ref: &ResidentRecordRefV1,
    handle: &OperationHandleV1,
) -> Result<OwnerRecordCoordinateV1, LocalOperationPublicationProjectionError> {
    let handle_jcs = canon::to_canonical_string(handle)?;
    Ok(OwnerRecordCoordinateV1 {
        owner_system_id: OwnerSystemIdV1::parse(PALE_OWNER)?,
        record_ref: handle_ref.clone(),
        record_schema_id: handle.schema_id().clone(),
        record_schema_sha256: handle.schema_sha256().clone(),
        record_content_sha256: ArtifactDigest::parse(sha256_hex(handle_jcs.as_bytes()))?,
        native_revision: NativeRevisionV1 {
            native_revision_owner: NativeRevisionOwnerV1::parse(PALE_OWNER)?,
            native_revision_token: NativeRevisionTokenV1::parse(format!(
                "operation-handle:{}",
                handle.canonical_operation_handle_sha256().as_str()
            ))?,
        },
    })
}

pub(super) fn recovered_operation_parent_placement(
    recovered: &RecoveredLocalOperation,
) -> Result<OwnerRecordKnowledgeV1, LocalOperationPublicationProjectionError> {
    let (_, handle) = recovered.operation().handle();
    match (handle.parent_operation(), recovered.recovered_parent()) {
        (None, None) => Ok(OwnerRecordKnowledgeV1::Unknown),
        (Some(expected), Some(parent)) => {
            let (parent_ref, parent_handle) = parent.operation().handle();
            if expected.operation_handle_id != *parent_handle.operation_handle_id()
                || expected.operation_handle_ref != *parent_ref
                || expected.operation_handle_sha256
                    != *parent_handle.canonical_operation_handle_sha256()
            {
                return Err(LocalOperationPublicationProjectionError::Association);
            }
            Ok(OwnerRecordKnowledgeV1::Known {
                coordinate: operation_handle_coordinate(parent_ref, parent_handle)?,
            })
        }
        _ => Err(LocalOperationPublicationProjectionError::Association),
    }
}

fn operation_class_relation(operation_class: OperationClassV1) -> &'static str {
    match operation_class {
        OperationClassV1::Invoke => INVOKE_RELATION,
        OperationClassV1::Consult => CONSULT_RELATION,
        OperationClassV1::Delegate => DELEGATE_RELATION,
    }
}

fn validate_parent_start(
    store: &AuthorityStoreCoordinate,
    child_handle: &OperationHandleV1,
    parent: &LocalOperationPredecessorPublication,
    parent_placement: &OwnerRecordKnowledgeV1,
    child_handle_coordinate: &OwnerRecordCoordinateV1,
    output_references: [Option<&ResidentRecordRefV1>; 7],
) -> Result<(), LocalOperationPublicationProjectionError> {
    let expected_parent = child_handle
        .parent_operation()
        .as_ref()
        .ok_or(LocalOperationPublicationProjectionError::Association)?;
    let OwnerRecordKnowledgeV1::Known {
        coordinate: parent_coordinate,
    } = parent_placement
    else {
        return Err(LocalOperationPublicationProjectionError::Association);
    };
    let (observation_ref, observation) = (&parent.observation.0, &parent.observation.1);
    let (event_ref, event) = (&parent.event.0, &parent.event.1);
    let (occurrence_ref, occurrence) = (&parent.occurrence.0, &parent.occurrence.1);
    observation.verify()?;
    event.verify()?;
    occurrence.verify()?;
    let observation_jcs = canon::to_canonical_string(observation)?;
    let observation_digest = ArtifactDigest::parse(sha256_hex(observation_jcs.as_bytes()))?;
    let expected_event_id = derived_id(
        "pale-local-operation-event-v1",
        store,
        observation_ref,
        observation,
    )?;
    let expected_occurrence_id = derived_id(
        "pale-local-operation-occurrence-v1",
        store,
        observation_ref,
        observation,
    )?;
    if observation.revision().get() != 1
        || observation.predecessor_ref().is_some()
        || observation.predecessor_sha256().is_some()
        || expected_parent.operation_handle_id != *observation.operation_handle_id()
        || expected_parent.operation_handle_ref != *observation.operation_handle_ref()
        || expected_parent.operation_handle_sha256 != *observation.operation_handle_sha256()
        || parent_coordinate.record_ref != expected_parent.operation_handle_ref
        || parent_coordinate
            .native_revision
            .native_revision_token
            .as_str()
            != format!(
                "operation-handle:{}",
                expected_parent.operation_handle_sha256.as_str()
            )
        || occurrence.native_container() != parent_coordinate
        || parent_coordinate == child_handle_coordinate
        || event.source_record_ref() != observation_ref
        || event.source_record_content_sha256() != &observation_digest
        || event.event_id().as_str() != expected_event_id
        || event.source_version().as_str() != "1"
        || occurrence.event_reference_ref() != event_ref
        || occurrence.event_reference_sha256() != event.canonical_event_sha256()
        || occurrence.occurrence_id().as_str() != expected_occurrence_id
        || output_references.into_iter().flatten().any(|output| {
            output == observation_ref || output == event_ref || output == occurrence_ref
        })
    {
        return Err(LocalOperationPublicationProjectionError::Association);
    }
    Ok(())
}

pub(super) fn admit_runtime_context(
    origin_principal_id: &ResidentPrincipalIdV1,
    deployment_runtime_authority_ref: &ResidentRecordRefV1,
    deployment_runtime_authority_sha256: &ArtifactDigest,
    context: &SignedResidentAdmissionContextV1<'_>,
) -> Result<(), LocalOperationPublicationProjectionError> {
    let Some(runtime_authority) = context.signing_authority.integrated_runtime_authority() else {
        return Err(LocalOperationPublicationProjectionError::Association);
    };
    if &runtime_authority.deployment_runtime_authority_ref != deployment_runtime_authority_ref
        || &runtime_authority.deployment_runtime_authority_sha256
            != deployment_runtime_authority_sha256
        || context.signing_authority.principal_id() != origin_principal_id
    {
        return Err(LocalOperationPublicationProjectionError::Association);
    }
    Ok(())
}

/// Projection failed before any signing callback could authorize the result.
#[derive(Debug, thiserror::Error)]
pub enum LocalOperationPublicationProjectionError {
    /// The source family is not a local operation observation.
    #[error("publication source is not a local operation observation")]
    UnsupportedSource,
    /// The physical source, recovered evidence, operation, or authority differs.
    #[error("local operation publication evidence does not form one exact lineage")]
    Association,
    /// A successor is missing its already admitted predecessor publication.
    #[error("successor publication is missing exact predecessor projection evidence")]
    MissingPredecessor,
    /// Revision one incorrectly supplied predecessor projection evidence.
    #[error("initial operation publication cannot carry predecessor projection evidence")]
    UnexpectedPredecessor,
    /// A public record rejected its structure, dependency, or signing context.
    #[error(transparent)]
    Admission(#[from] InvalidResidentRecordV1),
    /// The generated contract registry could not supply an exact schema identity.
    #[error(transparent)]
    Schema(#[from] pale_contracts::SchemaError),
    /// Canonical record encoding failed.
    #[error(transparent)]
    Canonical(#[from] canon::CanonError),
    /// A locally computed SHA-256 value could not enter its nominal digest type.
    #[error(transparent)]
    Digest(#[from] InvalidDigest),
    /// The producer-independent source lookup could not be projected.
    #[error(transparent)]
    EventProjection(#[from] super::EventCustodyProjectionError),
}

#[derive(Serialize)]
struct DerivedIdentityPreimage<'a> {
    initial_v1_domain: &'a str,
    store_id: &'a str,
    store_incarnation_id: &'a str,
    observation_ref: &'a str,
    observation_sha256: &'a str,
}

#[allow(clippy::too_many_arguments)]
fn admit_local_operation_predecessor_publication_with_parent<
    EventAdmission,
    OccurrenceVerification,
>(
    store: &AuthorityStoreCoordinate,
    recovered_store: &AuthorityStoreCoordinate,
    operation: (&ResidentRecordRefV1, &OperationHandleV1),
    observation: (&ResidentRecordRefV1, &OperationObservationV1),
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    policy: &LocalOperationPublicationPolicy,
    parent_placement: OwnerRecordKnowledgeV1,
    event: (&ResidentRecordRefV1, &EventReferenceV1),
    occurrence: (&ResidentRecordRefV1, &InteractionOccurrenceV1),
    occurrence_context: SignedResidentAdmissionContextV1<'_>,
    admit_event_source: EventAdmission,
    verify_occurrence_signature: OccurrenceVerification,
) -> Result<LocalOperationPredecessorPublication, LocalOperationPublicationProjectionError>
where
    EventAdmission: FnOnce(&EventReferencePreimageV1) -> Result<(), InvalidResidentRecordV1>,
    OccurrenceVerification: FnOnce(
        &[u8],
        &InteractionOccurrencePreimageV1,
        &InteractionOccurrenceDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    let (handle_ref, handle) = operation;
    let (observation_ref, observation_value) = observation;
    let (_, checkpoint_value) = checkpoint;
    let expected_references =
        LocalOperationPublicationReferences::derive(store, observation_ref, observation_value)?;
    let observation_schema =
        pale_contracts::record_schema_identity(OPERATION_OBSERVATION_V1_SCHEMA_ID)?;
    let checkpoint_schema =
        pale_contracts::record_schema_identity(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID)?;
    let observation_jcs = canon::to_canonical_string(observation_value)?;
    let handle_jcs = canon::to_canonical_string(handle)?;

    if recovered_store != store
        || observation_value.operation_handle_id() != handle.operation_handle_id()
        || observation_value.operation_handle_ref() != handle_ref
        || observation_value.operation_handle_sha256() != handle.canonical_operation_handle_sha256()
        || checkpoint_value.control_store_id() != store.store_id()
        || checkpoint_value.control_incarnation_id() != store.incarnation_id()
        || event.0 != expected_references.event()
        || occurrence.0 != expected_references.occurrence()
        || observation_value.schema_id() != &observation_schema.schema
        || observation_value.schema_sha256() != &observation_schema.schema_sha256
        || checkpoint_value.schema_id() != &checkpoint_schema.schema
        || checkpoint_value.schema_sha256() != &checkpoint_schema.schema_sha256
    {
        return Err(LocalOperationPublicationProjectionError::Association);
    }

    let event_schema = pale_contracts::record_schema_identity(EVENT_REFERENCE_V1_SCHEMA_ID)?;
    let admitted_event = EventReferenceV1::admit_contextual_wire(
        canonical_wire(event.1)?,
        &event_schema.schema_sha256,
        admit_event_source,
    )?;
    let observation_bytes_sha256 = ArtifactDigest::parse(sha256_hex(observation_jcs.as_bytes()))?;
    let minimum_recorded_at = projection_time(observation_value, checkpoint_value);
    let causation_id = match (
        observation_value.predecessor_ref(),
        observation_value.predecessor_sha256(),
    ) {
        (None, None) => None,
        (Some(reference), Some(digest)) => {
            Some(EventCausationIdV1::parse(derived_id_from_digest(
                "pale-local-operation-event-v1",
                store,
                reference,
                digest.as_str(),
            )?)?)
        }
        _ => return Err(LocalOperationPublicationProjectionError::Association),
    };
    let expected_event = LocalOperationEventMeaning {
        schema_id: event_schema.schema,
        schema_sha256: event_schema.schema_sha256,
        event_id: EventIdV1::parse(derived_id(
            "pale-local-operation-event-v1",
            store,
            observation_ref,
            observation_value,
        )?)?,
        event_type: EventTypeV1::parse(EVENT_TYPE)?,
        event_version: 1,
        lifecycle_namespace: LifecycleNamespaceV1::parse(LIFECYCLE_NAMESPACE)?,
        origin_principal_id: observation_value.issuer_principal_id().clone(),
        source_owner: SourceOwnerV1::parse(PALE_OWNER)?,
        source_record_ref: observation_ref.clone(),
        source_record_schema_id: observation_value.schema_id().clone(),
        source_record_schema_sha256: observation_value.schema_sha256().clone(),
        source_record_content_sha256: observation_bytes_sha256,
        source_version: SourceVersionV1::parse(observation_value.revision().get().to_string())?,
        target_scope: policy.target_scope.clone(),
        classification: policy.classification.clone(),
        ordering_key: EventOrderingKeyV1::parse(handle.operation_handle_id().as_str())?,
        sequence: OwnerEventSequenceV1::new(observation_value.revision().get())?,
        occurred_at: *observation_value.observed_at(),
        minimum_recorded_at,
        correlation_id: EventCorrelationIdV1::parse(handle.request_binding().correlation_id())?,
        causation_id,
    };
    if !expected_event.matches(&admitted_event) {
        return Err(LocalOperationPublicationProjectionError::Association);
    }

    let handle_coordinate = OwnerRecordCoordinateV1 {
        owner_system_id: OwnerSystemIdV1::parse(PALE_OWNER)?,
        record_ref: handle_ref.clone(),
        record_schema_id: handle.schema_id().clone(),
        record_schema_sha256: handle.schema_sha256().clone(),
        record_content_sha256: ArtifactDigest::parse(sha256_hex(handle_jcs.as_bytes()))?,
        native_revision: NativeRevisionV1 {
            native_revision_owner: NativeRevisionOwnerV1::parse(PALE_OWNER)?,
            native_revision_token: NativeRevisionTokenV1::parse(format!(
                "operation-handle:{}",
                handle.canonical_operation_handle_sha256().as_str()
            ))?,
        },
    };
    admit_runtime_context(
        admitted_event.origin_principal_id(),
        handle.deployment_runtime_authority_ref(),
        handle.deployment_runtime_authority_sha256(),
        &occurrence_context,
    )?;
    let occurrence_schema =
        pale_contracts::record_schema_identity(INTERACTION_OCCURRENCE_V1_SCHEMA_ID)?;
    let expected_occurrence = operation_occurrence_preimage(
        InteractionOccurrenceIdV1::parse(derived_id(
            "pale-local-operation-occurrence-v1",
            store,
            observation_ref,
            observation_value,
        )?)?,
        event.0,
        &admitted_event,
        &handle_coordinate,
        parent_placement,
        policy,
        *admitted_event.recorded_at(),
        occurrence_schema.schema.clone(),
        occurrence_schema.schema_sha256.clone(),
        &occurrence_context,
    )?;
    let admitted_occurrence = InteractionOccurrenceV1::admit_contextual_wire(
        canonical_wire(occurrence.1)?,
        &occurrence_context,
        |actual| {
            if actual == &expected_occurrence {
                Ok(())
            } else {
                Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "local operation predecessor occurrence",
                    invariant: "exact admitted publication meaning",
                })
            }
        },
        verify_occurrence_signature,
    )?;
    if &admitted_occurrence != occurrence.1 {
        return Err(LocalOperationPublicationProjectionError::Association);
    }

    Ok(LocalOperationPredecessorPublication {
        observation: (observation.0.clone(), observation.1.clone()),
        event: (event.0.clone(), event.1.clone()),
        occurrence: (occurrence.0.clone(), occurrence.1.clone()),
    })
}

#[allow(clippy::too_many_arguments)]
fn admit_local_operation_predecessor_publication_from_parts<
    EventAdmission,
    OccurrenceVerification,
>(
    store: &AuthorityStoreCoordinate,
    recovered_store: &AuthorityStoreCoordinate,
    operation: (&ResidentRecordRefV1, &OperationHandleV1),
    observation: (&ResidentRecordRefV1, &OperationObservationV1),
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    policy: &LocalOperationPublicationPolicy,
    event: (&ResidentRecordRefV1, &EventReferenceV1),
    occurrence: (&ResidentRecordRefV1, &InteractionOccurrenceV1),
    occurrence_context: SignedResidentAdmissionContextV1<'_>,
    admit_event_source: EventAdmission,
    verify_occurrence_signature: OccurrenceVerification,
) -> Result<LocalOperationPredecessorPublication, LocalOperationPublicationProjectionError>
where
    EventAdmission: FnOnce(&EventReferencePreimageV1) -> Result<(), InvalidResidentRecordV1>,
    OccurrenceVerification: FnOnce(
        &[u8],
        &InteractionOccurrencePreimageV1,
        &InteractionOccurrenceDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    admit_local_operation_predecessor_publication_with_parent(
        store,
        recovered_store,
        operation,
        observation,
        checkpoint,
        policy,
        OwnerRecordKnowledgeV1::Unknown,
        event,
        occurrence,
        occurrence_context,
        admit_event_source,
        verify_occurrence_signature,
    )
}

/// Admit predecessor records returned by exact Event indexes.
///
/// The first decode establishes only the closed record shape needed to build
/// the exact expected preimages. The existing predecessor admission then
/// performs the owner-source and signature checks before any value is returned.
#[allow(clippy::too_many_arguments)]
pub(super) fn admit_local_operation_predecessor_publication_records<
    Authority,
    EventAdmission,
    OccurrenceVerification,
>(
    store: &AuthorityStoreCoordinate,
    recovered: &RecoveredOperationObservation,
    operation: &RecoveredLocalOperation,
    event_record: &pale_ports::ControlRecord,
    occurrence_record: &pale_ports::ControlRecord,
    resolve_occurrence_authority: Authority,
    admit_event_source: EventAdmission,
    verify_occurrence_signature: OccurrenceVerification,
) -> Result<LocalOperationPredecessorPublication, LocalOperationPublicationProjectionError>
where
    Authority: FnOnce(
        &pale_ports::ControlRecord,
    ) -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1>,
    EventAdmission: FnOnce(&EventReferencePreimageV1) -> Result<(), InvalidResidentRecordV1>,
    OccurrenceVerification: FnOnce(
        &[u8],
        &InteractionOccurrencePreimageV1,
        &InteractionOccurrenceDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    let event_schema = pale_contracts::record_schema_identity(EVENT_REFERENCE_V1_SCHEMA_ID)?;
    let occurrence_schema =
        pale_contracts::record_schema_identity(INTERACTION_OCCURRENCE_V1_SCHEMA_ID)?;
    if event_record.schema_id() != &event_schema.schema
        || event_record.schema_sha256() != &event_schema.schema_sha256
        || occurrence_record.schema_id() != &occurrence_schema.schema
        || occurrence_record.schema_sha256() != &occurrence_schema.schema_sha256
    {
        return Err(LocalOperationPublicationProjectionError::Association);
    }

    // These temporary typed values are not returned. Their owner-specific
    // admission and signature checks occur in the shared exact path below.
    let event_wire = serde_json::from_value(canon::parse_strict(
        event_record.canonical_record_jcs().as_slice(),
    )?)
    .map_err(|_| {
        LocalOperationPublicationProjectionError::Admission(InvalidResidentRecordV1::NonCanonical {
            relation: "local operation predecessor Event wire",
        })
    })?;
    let event =
        EventReferenceV1::admit_contextual_wire(event_wire, &event_schema.schema_sha256, |_| {
            Ok(())
        })?;
    let occurrence_authority = resolve_occurrence_authority(occurrence_record)?;
    let occurrence_context = SignedResidentAdmissionContextV1 {
        expected_schema_sha256: &occurrence_schema.schema_sha256,
        signing_authority: &occurrence_authority,
    };
    let occurrence_wire = serde_json::from_value(canon::parse_strict(
        occurrence_record.canonical_record_jcs().as_slice(),
    )?)
    .map_err(|_| {
        LocalOperationPublicationProjectionError::Admission(InvalidResidentRecordV1::NonCanonical {
            relation: "local operation predecessor occurrence wire",
        })
    })?;
    let occurrence = InteractionOccurrenceV1::admit_contextual_wire(
        occurrence_wire,
        &occurrence_context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )?;
    let admitted = admit_local_operation_predecessor_publication_with_parent(
        store,
        recovered.store(),
        operation.operation().handle(),
        recovered.selected(),
        recovered.selected_checkpoint_record(),
        operation.operation().publication_policy(),
        recovered_operation_parent_placement(operation)?,
        (event_record.coordinate().record_ref(), &event),
        (occurrence_record.coordinate().record_ref(), &occurrence),
        occurrence_context,
        admit_event_source,
        verify_occurrence_signature,
    )?;
    let event_jcs = canon::to_canonical_string(&event)?;
    let occurrence_jcs = canon::to_canonical_string(&occurrence)?;
    if event_record.coordinate().record_ref() != &admitted.event.0
        || event_record.coordinate().record_sha256().as_str()
            != event.canonical_event_sha256().as_str()
        || event_record.canonical_record_jcs().as_slice() != event_jcs.as_bytes()
        || occurrence_record.coordinate().record_ref() != &admitted.occurrence.0
        || occurrence_record.coordinate().record_sha256().as_str()
            != occurrence.canonical_occurrence_sha256().as_str()
        || occurrence_record.canonical_record_jcs().as_slice() != occurrence_jcs.as_bytes()
    {
        return Err(LocalOperationPublicationProjectionError::Association);
    }
    Ok(admitted)
}

fn canonical_wire<W, R>(record: &R) -> Result<W, LocalOperationPublicationProjectionError>
where
    W: serde::de::DeserializeOwned,
    R: Serialize,
{
    serde_json::from_str(&canon::to_canonical_string(record)?).map_err(|_| {
        LocalOperationPublicationProjectionError::Admission(InvalidResidentRecordV1::NonCanonical {
            relation: "local operation predecessor wire",
        })
    })
}

/// Admit the signer-free publication meaning of one local operation observation.
///
/// All physical source bytes must equal the separately recovered and admitted
/// observation and checkpoint. A root's first revision has no Genealogy edge.
/// A child's first revision requires its recovered parent's admitted start
/// occurrence. A later revision requires the same operation's exact predecessor
/// observation and occurrence. Signed records are issued one missing stage at a
/// time after every retained source has been readmitted.
///
/// # Errors
/// Returns [`LocalOperationPublicationProjectionError`] for a foreign source,
/// unlike evidence, missing predecessor publication, schema failure, or record
/// admission/signing failure.
#[allow(clippy::too_many_arguments)]
pub fn admit_local_operation_publication_plan(
    store: &AuthorityStoreCoordinate,
    source: &OperationPublicationSource,
    recovered: &RecoveredOperationObservation,
    operation: &IssuedLocalOperation,
    references: &LocalOperationPublicationReferences,
    predecessor: Option<LocalOperationPredecessorPublication>,
) -> Result<AdmittedLocalOperationPublicationPlan, LocalOperationPublicationProjectionError> {
    admit_local_operation_publication_plan_from_parts(
        store,
        source,
        recovered.store(),
        operation.handle(),
        recovered.selected(),
        recovered.selected_checkpoint_record(),
        operation.publication_policy(),
        references,
        predecessor,
    )
}

/// Admit one recovered operation publication with its exact retained parent.
#[allow(clippy::too_many_arguments)]
pub(super) fn admit_recovered_local_operation_publication_plan(
    store: &AuthorityStoreCoordinate,
    source: &OperationPublicationSource,
    recovered: &RecoveredOperationObservation,
    operation: &RecoveredLocalOperation,
    references: &LocalOperationPublicationReferences,
    predecessor: Option<LocalOperationPredecessorPublication>,
    parent_start: Option<LocalOperationPredecessorPublication>,
) -> Result<AdmittedLocalOperationPublicationPlan, LocalOperationPublicationProjectionError> {
    admit_local_operation_publication_plan_with_parent(
        store,
        source,
        recovered.store(),
        operation.operation().handle(),
        recovered.selected(),
        recovered.selected_checkpoint_record(),
        operation.operation().publication_policy(),
        references,
        recovered_operation_parent_placement(operation)?,
        predecessor,
        parent_start,
    )
}

#[allow(clippy::too_many_arguments)]
fn admit_local_operation_publication_plan_with_parent(
    store: &AuthorityStoreCoordinate,
    source: &OperationPublicationSource,
    recovered_store: &AuthorityStoreCoordinate,
    operation: (&ResidentRecordRefV1, &OperationHandleV1),
    observation: (&ResidentRecordRefV1, &OperationObservationV1),
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    policy: &LocalOperationPublicationPolicy,
    references: &LocalOperationPublicationReferences,
    parent_placement: OwnerRecordKnowledgeV1,
    predecessor: Option<LocalOperationPredecessorPublication>,
    parent_start: Option<LocalOperationPredecessorPublication>,
) -> Result<AdmittedLocalOperationPublicationPlan, LocalOperationPublicationProjectionError> {
    let evidence = admit_projection_evidence(
        store,
        source,
        recovered_store,
        operation,
        observation,
        checkpoint,
        policy,
        references,
        predecessor.as_ref(),
        parent_start.as_ref(),
        &parent_placement,
    )?;

    let event_schema = pale_contracts::record_schema_identity(EVENT_REFERENCE_V1_SCHEMA_ID)?;
    let event_meaning = LocalOperationEventMeaning {
        schema_id: event_schema.schema,
        schema_sha256: event_schema.schema_sha256.clone(),
        event_id: EventIdV1::parse(evidence.event_id.clone())?,
        event_type: EventTypeV1::parse(EVENT_TYPE)?,
        event_version: 1,
        lifecycle_namespace: LifecycleNamespaceV1::parse(LIFECYCLE_NAMESPACE)?,
        origin_principal_id: evidence.observation.issuer_principal_id().clone(),
        source_owner: SourceOwnerV1::parse(PALE_OWNER)?,
        source_record_ref: evidence.observation_ref.clone(),
        source_record_schema_id: source.observation().schema_id().clone(),
        source_record_schema_sha256: source.observation().schema_sha256().clone(),
        source_record_content_sha256: evidence.observation_bytes_sha256.clone(),
        source_version: SourceVersionV1::parse(evidence.observation.revision().get().to_string())?,
        target_scope: policy.target_scope.clone(),
        classification: policy.classification.clone(),
        ordering_key: EventOrderingKeyV1::parse(evidence.handle.operation_handle_id().as_str())?,
        sequence: OwnerEventSequenceV1::new(evidence.observation.revision().get())?,
        occurred_at: *evidence.observation.observed_at(),
        minimum_recorded_at: evidence.minimum_recorded_at,
        correlation_id: EventCorrelationIdV1::parse(
            evidence.handle.request_binding().correlation_id(),
        )?,
        causation_id: predecessor
            .as_ref()
            .map(|prior| EventCausationIdV1::parse(prior.event.1.event_id().as_str()))
            .transpose()?,
    };
    let owner_source_key = super::projection::owner_source_key_from_parts(
        &event_meaning.origin_principal_id,
        &event_meaning.source_owner,
        &event_meaning.source_record_ref,
        &event_meaning.source_record_schema_id,
        &event_meaning.source_record_schema_sha256,
        &event_meaning.source_record_content_sha256,
        &event_meaning.source_version,
    )?;

    let edge_proof = references
        .edge
        .as_ref()
        .map(
            |_| -> Result<GenealogyProofV1, LocalOperationPublicationProjectionError> {
                Ok(GenealogyProofV1::NativeStructural {
                    relation: GenealogyRelationV1::parse(if predecessor.is_some() {
                        SUCCESSOR_RELATION
                    } else {
                        operation_class_relation(*evidence.handle.operation_class())
                    })?,
                    native_owner_evidence: if predecessor.is_some() {
                        evidence.observation_coordinate.clone()
                    } else {
                        evidence.handle_coordinate.clone()
                    },
                })
            },
        )
        .transpose()?;
    Ok(AdmittedLocalOperationPublicationPlan {
        event_meaning,
        owner_source_key,
        references: references.clone(),
        deployment_runtime_authority_ref: evidence
            .handle
            .deployment_runtime_authority_ref()
            .clone(),
        deployment_runtime_authority_sha256: evidence
            .handle
            .deployment_runtime_authority_sha256()
            .clone(),
        occurrence_id: InteractionOccurrenceIdV1::parse(evidence.occurrence_id)?,
        handle_coordinate: evidence.handle_coordinate.clone(),
        parent_placement,
        edge_source_occurrence: predecessor
            .as_ref()
            .or(parent_start.as_ref())
            .map(|prior| (prior.occurrence.0.clone(), prior.occurrence.1.clone())),
        edge_proof,
        edge_id: references
            .edge
            .as_ref()
            .map(|_| GenealogyEdgeIdV1::parse(evidence.edge_id))
            .transpose()?,
        observation_coordinate: evidence.observation_coordinate,
        frontier_id: ReconciliationFrontierIdV1::parse(evidence.frontier_id)?,
        checkpoint_ref: evidence.checkpoint_ref,
        checkpoint_bytes_sha256: evidence.checkpoint_bytes_sha256,
        observation_ref: evidence.observation_ref,
        observation_bytes_sha256: evidence.observation_bytes_sha256,
        policy: policy.clone(),
    })
}

#[allow(clippy::too_many_arguments)]
fn admit_local_operation_publication_plan_from_parts(
    store: &AuthorityStoreCoordinate,
    source: &OperationPublicationSource,
    recovered_store: &AuthorityStoreCoordinate,
    operation: (&ResidentRecordRefV1, &OperationHandleV1),
    observation: (&ResidentRecordRefV1, &OperationObservationV1),
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    policy: &LocalOperationPublicationPolicy,
    references: &LocalOperationPublicationReferences,
    predecessor: Option<LocalOperationPredecessorPublication>,
) -> Result<AdmittedLocalOperationPublicationPlan, LocalOperationPublicationProjectionError> {
    admit_local_operation_publication_plan_with_parent(
        store,
        source,
        recovered_store,
        operation,
        observation,
        checkpoint,
        policy,
        references,
        OwnerRecordKnowledgeV1::Unknown,
        predecessor,
        None,
    )
}

struct AdmittedProjectionEvidence<'a> {
    handle: &'a OperationHandleV1,
    observation: &'a OperationObservationV1,
    observation_ref: ResidentRecordRefV1,
    observation_bytes_sha256: ArtifactDigest,
    checkpoint_ref: ResidentRecordRefV1,
    checkpoint_bytes_sha256: ArtifactDigest,
    handle_coordinate: OwnerRecordCoordinateV1,
    observation_coordinate: OwnerRecordCoordinateV1,
    event_id: String,
    occurrence_id: String,
    edge_id: String,
    frontier_id: String,
    minimum_recorded_at: UnixTimeV1,
}

#[allow(clippy::too_many_arguments)]
fn admit_projection_evidence<'a>(
    store: &AuthorityStoreCoordinate,
    source: &OperationPublicationSource,
    recovered_store: &AuthorityStoreCoordinate,
    operation: (&'a ResidentRecordRefV1, &'a OperationHandleV1),
    selected_observation: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    selected_checkpoint: (&'a ResidentRecordRefV1, &'a ControlStateCheckpointV1),
    policy: &LocalOperationPublicationPolicy,
    references: &LocalOperationPublicationReferences,
    predecessor: Option<&LocalOperationPredecessorPublication>,
    parent_start: Option<&LocalOperationPredecessorPublication>,
    parent_placement: &OwnerRecordKnowledgeV1,
) -> Result<AdmittedProjectionEvidence<'a>, LocalOperationPublicationProjectionError> {
    if source.kind() != OperationPublicationSourceKind::OperationObservation {
        return Err(LocalOperationPublicationProjectionError::UnsupportedSource);
    }
    let (handle_ref, handle) = operation;
    let (observation_ref, observation) = selected_observation;
    let (checkpoint_ref, checkpoint) = selected_checkpoint;

    let observation_jcs = canon::to_canonical_string(observation)?;
    let checkpoint_jcs = canon::to_canonical_string(checkpoint)?;
    let handle_jcs = canon::to_canonical_string(handle)?;
    let observation_schema =
        pale_contracts::record_schema_identity(OPERATION_OBSERVATION_V1_SCHEMA_ID)?;
    let checkpoint_schema =
        pale_contracts::record_schema_identity(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID)?;

    let output_references = [
        Some(&references.event),
        Some(&references.occurrence),
        Some(&references.occurrence_retention),
        references.edge.as_ref(),
        references.edge_retention.as_ref(),
        Some(&references.frontier),
        Some(&references.frontier_retention),
    ];
    let references_are_distinct =
        output_references
            .into_iter()
            .flatten()
            .enumerate()
            .all(|(index, reference)| {
                output_references
                    .into_iter()
                    .flatten()
                    .skip(index + 1)
                    .all(|other| reference != other)
            });

    if recovered_store != store
        || source.observation().coordinate().record_ref() != observation_ref
        || source.observation().coordinate().record_sha256().as_str()
            != observation
                .canonical_operation_observation_sha256()
                .as_str()
        || source.observation().schema_id() != &observation_schema.schema
        || source.observation().schema_sha256() != &observation_schema.schema_sha256
        || source.observation().canonical_record_jcs().as_slice() != observation_jcs.as_bytes()
        || source.checkpoint().coordinate().record_ref() != checkpoint_ref
        || source.checkpoint().coordinate().record_sha256().as_str()
            != checkpoint.canonical_control_checkpoint_sha256().as_str()
        || source.checkpoint().schema_id() != &checkpoint_schema.schema
        || source.checkpoint().schema_sha256() != &checkpoint_schema.schema_sha256
        || source.checkpoint().canonical_record_jcs().as_slice() != checkpoint_jcs.as_bytes()
        || source.transaction() != *checkpoint.committed_transaction_sequence()
        || checkpoint.control_store_id() != store.store_id()
        || checkpoint.control_incarnation_id() != store.incarnation_id()
        || observation.operation_handle_id() != handle.operation_handle_id()
        || observation.operation_handle_ref() != handle_ref
        || observation.operation_handle_sha256() != handle.canonical_operation_handle_sha256()
        || references.edge.is_some() != references.edge_retention.is_some()
        || !references_are_distinct
    {
        return Err(LocalOperationPublicationProjectionError::Association);
    }

    let observation_bytes_sha256 = ArtifactDigest::parse(sha256_hex(observation_jcs.as_bytes()))?;
    let checkpoint_bytes_sha256 = ArtifactDigest::parse(sha256_hex(checkpoint_jcs.as_bytes()))?;
    let handle_bytes_sha256 = ArtifactDigest::parse(sha256_hex(handle_jcs.as_bytes()))?;
    let native_revision = NativeRevisionV1 {
        native_revision_owner: NativeRevisionOwnerV1::parse(PALE_OWNER)?,
        native_revision_token: NativeRevisionTokenV1::parse(format!(
            "operation-observation:{}:{}",
            observation.revision().get(),
            observation
                .canonical_operation_observation_sha256()
                .as_str()
        ))?,
    };
    let handle_coordinate = OwnerRecordCoordinateV1 {
        owner_system_id: OwnerSystemIdV1::parse(PALE_OWNER)?,
        record_ref: handle_ref.clone(),
        record_schema_id: handle.schema_id().clone(),
        record_schema_sha256: handle.schema_sha256().clone(),
        record_content_sha256: handle_bytes_sha256,
        native_revision: NativeRevisionV1 {
            native_revision_owner: NativeRevisionOwnerV1::parse(PALE_OWNER)?,
            native_revision_token: NativeRevisionTokenV1::parse(format!(
                "operation-handle:{}",
                handle.canonical_operation_handle_sha256().as_str()
            ))?,
        },
    };
    let observation_coordinate = OwnerRecordCoordinateV1 {
        owner_system_id: OwnerSystemIdV1::parse(PALE_OWNER)?,
        record_ref: observation_ref.clone(),
        record_schema_id: observation.schema_id().clone(),
        record_schema_sha256: observation.schema_sha256().clone(),
        record_content_sha256: observation_bytes_sha256.clone(),
        native_revision,
    };

    let known_references = [handle_ref, observation_ref, checkpoint_ref];
    if output_references
        .into_iter()
        .flatten()
        .any(|output| known_references.contains(&output))
    {
        return Err(LocalOperationPublicationProjectionError::Association);
    }

    let child_start = observation.revision().get() == 1 && handle.parent_operation().is_some();
    match (observation.predecessor_ref(), predecessor, parent_start) {
        (None, None, None)
            if observation.revision().get() == 1
                && references.edge.is_none()
                && references.edge_retention.is_none() => {}
        (None, None, Some(parent)) if child_start && references.edge.is_some() => {
            validate_parent_start(
                store,
                handle,
                parent,
                parent_placement,
                &handle_coordinate,
                output_references,
            )?;
        }
        (None, Some(_), _) | (None, _, Some(_)) => {
            return Err(LocalOperationPublicationProjectionError::UnexpectedPredecessor);
        }
        (None, None, None) if child_start => {
            return Err(LocalOperationPublicationProjectionError::MissingPredecessor);
        }
        (None, None, None) => return Err(LocalOperationPublicationProjectionError::Association),
        (Some(_), None, _) => {
            return Err(LocalOperationPublicationProjectionError::MissingPredecessor);
        }
        (Some(expected_ref), Some(prior), None) => {
            let edge_ref = references
                .edge
                .as_ref()
                .ok_or(LocalOperationPublicationProjectionError::MissingPredecessor)?;
            let prior_observation_ref = &prior.observation.0;
            let prior_observation = &prior.observation.1;
            let prior_event_ref = &prior.event.0;
            let prior_event = &prior.event.1;
            let prior_occurrence_ref = &prior.occurrence.0;
            let prior_occurrence = &prior.occurrence.1;
            prior_observation.verify()?;
            prior_event.verify()?;
            prior_occurrence.verify()?;
            let prior_observation_jcs = canon::to_canonical_string(prior_observation)?;
            let prior_observation_bytes_sha256 =
                ArtifactDigest::parse(sha256_hex(prior_observation_jcs.as_bytes()))?;
            let prior_event_id = derived_id(
                "pale-local-operation-event-v1",
                store,
                prior_observation_ref,
                prior_observation,
            )?;
            let prior_occurrence_id = derived_id(
                "pale-local-operation-occurrence-v1",
                store,
                prior_observation_ref,
                prior_observation,
            )?;
            let prior_source_version = prior_observation.revision().get().to_string();
            if expected_ref != prior_observation_ref
                || observation.predecessor_sha256().as_ref()
                    != Some(prior_observation.canonical_operation_observation_sha256())
                || prior_observation.operation_handle_id() != handle.operation_handle_id()
                || prior_observation.operation_handle_ref() != handle_ref
                || prior_observation.operation_handle_sha256()
                    != handle.canonical_operation_handle_sha256()
                || prior_event.source_record_ref() != prior_observation_ref
                || prior_event.origin_principal_id() != observation.issuer_principal_id()
                || prior_event.event_id().as_str() != prior_event_id
                || prior_event.event_type().as_str() != EVENT_TYPE
                || prior_event.lifecycle_namespace().as_str() != LIFECYCLE_NAMESPACE
                || prior_event.source_owner().as_str() != PALE_OWNER
                || prior_event.source_record_schema_id() != prior_observation.schema_id()
                || prior_event.source_record_schema_sha256() != prior_observation.schema_sha256()
                || prior_event.source_record_content_sha256() != &prior_observation_bytes_sha256
                || prior_event.source_version().as_str() != prior_source_version
                || prior_event
                    .ordering_key()
                    .as_ref()
                    .map(EventOrderingKeyV1::as_str)
                    != Some(handle.operation_handle_id().as_str())
                || prior_event
                    .sequence()
                    .as_ref()
                    .map(|sequence| sequence.get())
                    != Some(prior_observation.revision().get())
                || prior_event.occurred_at() != prior_observation.observed_at()
                || prior_event.correlation_id().as_str()
                    != handle.request_binding().correlation_id()
                || prior_occurrence.event_reference_ref() != prior_event_ref
                || prior_occurrence.event_reference_sha256() != prior_event.canonical_event_sha256()
                || prior_occurrence.occurrence_id().as_str() != prior_occurrence_id
                || prior_occurrence.native_container() != &handle_coordinate
                || prior_occurrence.container_genre().as_str() != "operation"
                || prior_occurrence.actor_provenance() != &policy.actor_provenance
                || prior_occurrence_ref == &references.occurrence
                || edge_ref == prior_occurrence_ref
                || output_references.into_iter().flatten().any(|output| {
                    output == prior_observation_ref
                        || output == prior_event_ref
                        || output == prior_occurrence_ref
                })
            {
                return Err(LocalOperationPublicationProjectionError::Association);
            }
        }
        (Some(_), Some(_), Some(_)) => {
            return Err(LocalOperationPublicationProjectionError::UnexpectedPredecessor);
        }
    }

    let event_id = derived_id(
        "pale-local-operation-event-v1",
        store,
        observation_ref,
        observation,
    )?;
    let occurrence_id = derived_id(
        "pale-local-operation-occurrence-v1",
        store,
        observation_ref,
        observation,
    )?;
    let edge_id = derived_id(
        "pale-local-operation-edge-v1",
        store,
        observation_ref,
        observation,
    )?;
    let frontier_id = derived_id(
        "pale-local-operation-frontier-v1",
        store,
        observation_ref,
        observation,
    )?;
    let minimum_recorded_at = projection_time(observation, checkpoint);

    Ok(AdmittedProjectionEvidence {
        handle,
        observation,
        observation_ref: observation_ref.clone(),
        observation_bytes_sha256,
        checkpoint_ref: checkpoint_ref.clone(),
        checkpoint_bytes_sha256,
        handle_coordinate,
        observation_coordinate,
        event_id,
        occurrence_id,
        edge_id,
        frontier_id,
        minimum_recorded_at,
    })
}

fn derived_id(
    domain: &'static str,
    store: &AuthorityStoreCoordinate,
    observation_ref: &ResidentRecordRefV1,
    observation: &OperationObservationV1,
) -> Result<String, canon::CanonError> {
    derived_id_from_digest(
        domain,
        store,
        observation_ref,
        observation
            .canonical_operation_observation_sha256()
            .as_str(),
    )
}

pub(super) fn derived_id_from_digest(
    domain: &str,
    store: &AuthorityStoreCoordinate,
    observation_ref: &ResidentRecordRefV1,
    observation_sha256: &str,
) -> Result<String, canon::CanonError> {
    let canonical = canon::to_canonical_string(&DerivedIdentityPreimage {
        initial_v1_domain: domain,
        store_id: store.store_id().as_str(),
        store_incarnation_id: store.incarnation_id().as_str(),
        observation_ref: observation_ref.as_str(),
        observation_sha256,
    })?;
    Ok(sha256_hex(canonical.as_bytes()))
}

fn projection_time(
    observation: &OperationObservationV1,
    checkpoint: &ControlStateCheckpointV1,
) -> UnixTimeV1 {
    [*observation.observed_at(), *checkpoint.issued_at()]
        .into_iter()
        .max()
        .expect("projection evidence always contains two timestamps")
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/event_custody/local_operation_projection.rs"]
pub(in crate::resident) mod tests;
