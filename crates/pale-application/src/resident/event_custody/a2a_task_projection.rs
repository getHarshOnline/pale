//! Exact projection of one admitted A2A task observation.
//!
//! Remote task state remains owner evidence. This projector records that Pale
//! admitted and retained the evidence; it does not turn remote completion into

//! a Pale result, effect, acceptance, or settlement.

use pale_core::{canon, records::*, sha256_hex};
use pale_ports::{
    AuthorityStoreCoordinate, OperationPublicationSource, OperationPublicationSourceKind,
};

use super::local_operation_projection::{
    admit_runtime_context, derived_id_from_digest, operation_occurrence_preimage,
    AdmittedLocalOperationPublicationPlan, LocalOperationEventMeaning,
    LocalOperationPublicationProjectionError, LocalOperationPublicationReferences,
};
use super::projection::owner_source_key_from_parts;
use crate::resident::{IssuedLocalOperation, RecoveredA2aTaskHistory};

const PALE_OWNER: &str = "pale";
const EVENT_TYPE: &str = "pale.a2a.task.observation.committed";
const LIFECYCLE_NAMESPACE: &str = "pale.a2a.task.observation";

/// Deterministic references for one admitted remote-task observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct A2aTaskPublicationReferences(LocalOperationPublicationReferences);

impl A2aTaskPublicationReferences {
    /// Derive every output identity from the exact source record and store.
    ///
    /// # Errors
    /// Returns an encoding or coordinate error when a canonical identity cannot
    /// be formed.
    pub fn derive(
        store: &AuthorityStoreCoordinate,
        observation_ref: &ResidentRecordRefV1,
        observation: &A2aTaskObservationV1,
    ) -> Result<Self, A2aTaskPublicationProjectionError> {
        Ok(Self(
            LocalOperationPublicationReferences::derive_for_source(
                "pale-a2a-task-observation",
                "a2a-task-publication",
                store,
                observation_ref,
                observation.canonical_a2a_task_observation_sha256().as_str(),
                observation.predecessor_ref().is_some(),
            )?,
        ))
    }

    pub(super) const fn inner(&self) -> &LocalOperationPublicationReferences {
        &self.0
    }
}

/// Admitted predecessor Event and occurrence for one A2A successor.
pub struct A2aTaskPredecessorPublication {
    observation: (ResidentRecordRefV1, A2aTaskObservationV1),
    event: (ResidentRecordRefV1, EventReferenceV1),
    occurrence: (ResidentRecordRefV1, InteractionOccurrenceV1),
}

/// Signer-free A2A publication meaning over the shared publication stages.
pub struct AdmittedA2aTaskPublicationPlan {
    pub(super) inner: AdmittedLocalOperationPublicationPlan,
}

/// Admit one exact A2A task observation for Event and Genealogy projection.
///
/// The source checkpoint must already have passed its record-owner admission.
/// This function then proves exact physical bytes, store association, operation
/// and task binding, predecessor projection, deterministic identities, and the
/// retained publication policy before producing a plan. It performs no write,
/// signing, delivery, network request, or native effect.
///
/// # Errors
/// Rejects unlike source, checkpoint, operation, binding, predecessor, or
/// deterministic projection evidence.
#[allow(clippy::too_many_arguments)]
pub fn admit_a2a_task_publication_plan(
    store: &AuthorityStoreCoordinate,
    source: &OperationPublicationSource,
    recovered: &RecoveredA2aTaskHistory,
    operation: &IssuedLocalOperation,
    source_checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    references: &A2aTaskPublicationReferences,
    predecessor: Option<A2aTaskPredecessorPublication>,
) -> Result<AdmittedA2aTaskPublicationPlan, A2aTaskPublicationProjectionError> {
    if source.kind() != OperationPublicationSourceKind::A2aTaskObservation {
        return Err(A2aTaskPublicationProjectionError::UnsupportedSource);
    }
    let (observation_ref, observation) = recovered.selected_observation();
    let (binding_ref, binding) = recovered.binding();
    let (handle_ref, handle) = operation.handle();
    let (checkpoint_ref, checkpoint) = source_checkpoint;
    let observation_jcs = canon::to_canonical_string(observation)?;
    let handle_jcs = canon::to_canonical_string(handle)?;
    let checkpoint_jcs = canon::to_canonical_string(checkpoint)?;
    let observation_digest = ArtifactDigest::parse(sha256_hex(observation_jcs.as_bytes()))?;
    let handle_digest = ArtifactDigest::parse(sha256_hex(handle_jcs.as_bytes()))?;
    let checkpoint_digest = ArtifactDigest::parse(sha256_hex(checkpoint_jcs.as_bytes()))?;
    let observation_schema =
        pale_contracts::record_schema_identity(A2A_TASK_OBSERVATION_V1_SCHEMA_ID)?;
    let checkpoint_schema =
        pale_contracts::record_schema_identity(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID)?;
    let output = references.inner();
    let output_references = [
        Some(output.event()),
        Some(output.occurrence()),
        Some(output.occurrence_retention()),
        output.edge(),
        output.edge_retention(),
        Some(output.frontier()),
        Some(output.frontier_retention()),
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
    if recovered.local_observation().store() != store
        || source.observation().coordinate().record_ref() != observation_ref
        || source.observation().coordinate().record_sha256().as_str()
            != observation.canonical_a2a_task_observation_sha256().as_str()
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
        || observation.task_binding_id() != binding.task_binding_id()
        || observation.task_binding_ref() != binding_ref
        || observation.task_binding_sha256() != binding.canonical_a2a_task_binding_sha256()
        || binding.operation_handle_id() != handle.operation_handle_id()
        || binding.operation_handle_ref() != handle_ref
        || binding.operation_handle_sha256() != handle.canonical_operation_handle_sha256()
        || recovered.local_observation().handle.record_ref() != handle_ref
        || recovered
            .local_observation()
            .handle
            .record_sha256()
            .as_str()
            != handle.canonical_operation_handle_sha256().as_str()
        || output.edge().is_some() != output.edge_retention().is_some()
        || !references_are_distinct
    {
        return Err(A2aTaskPublicationProjectionError::Association);
    }

    let handle_coordinate = owner_coordinate(
        handle_ref,
        handle.schema_id(),
        handle.schema_sha256(),
        handle_digest,
        format!(
            "operation-handle:{}",
            handle.canonical_operation_handle_sha256().as_str()
        ),
    )?;
    let observation_coordinate = owner_coordinate(
        observation_ref,
        observation.schema_id(),
        observation.schema_sha256(),
        observation_digest.clone(),
        format!(
            "a2a-task-observation:{}:{}",
            observation.revision().get(),
            observation.canonical_a2a_task_observation_sha256().as_str()
        ),
    )?;
    let known_references = [handle_ref, binding_ref, observation_ref, checkpoint_ref];
    if output_references
        .into_iter()
        .flatten()
        .any(|reference| known_references.contains(&reference))
    {
        return Err(A2aTaskPublicationProjectionError::Association);
    }

    let occurrence_id = derived(
        "occurrence",
        store,
        observation_ref,
        observation.canonical_a2a_task_observation_sha256().as_str(),
    )?;
    let edge_id = derived(
        "edge",
        store,
        observation_ref,
        observation.canonical_a2a_task_observation_sha256().as_str(),
    )?;
    let frontier_id = derived(
        "frontier",
        store,
        observation_ref,
        observation.canonical_a2a_task_observation_sha256().as_str(),
    )?;
    let event_meaning = event_meaning(
        store,
        observation_ref,
        observation,
        handle,
        operation.publication_policy(),
        observation_digest.clone(),
        (*observation.observed_at()).max(*checkpoint.issued_at()),
    )?;

    match (observation.predecessor_ref(), predecessor.as_ref()) {
        (None, None)
            if observation.revision().get() == 1
                && output.edge().is_none()
                && output.edge_retention().is_none() => {}
        (None, Some(_)) => return Err(A2aTaskPublicationProjectionError::UnexpectedPredecessor),
        (None, None) => return Err(A2aTaskPublicationProjectionError::Association),
        (Some(_), None) => return Err(A2aTaskPublicationProjectionError::MissingPredecessor),
        (Some(expected_ref), Some(prior)) => {
            validate_predecessor(
                store,
                operation,
                observation,
                expected_ref,
                &handle_coordinate,
                operation.publication_policy(),
                output,
                prior,
            )?;
        }
    }

    let owner_source_key = owner_source_key_from_parts(
        &event_meaning.origin_principal_id,
        &event_meaning.source_owner,
        &event_meaning.source_record_ref,
        &event_meaning.source_record_schema_id,
        &event_meaning.source_record_schema_sha256,
        &event_meaning.source_record_content_sha256,
        &event_meaning.source_version,
    )?;
    let edge_proof = output
        .edge()
        .map(
            |_| -> Result<GenealogyProofV1, A2aTaskPublicationProjectionError> {
                Ok(GenealogyProofV1::NativeStructural {
                    relation: GenealogyRelationV1::parse("successor")?,
                    native_owner_evidence: observation_coordinate.clone(),
                })
            },
        )
        .transpose()?;
    Ok(AdmittedA2aTaskPublicationPlan {
        inner: AdmittedLocalOperationPublicationPlan {
            event_meaning,
            owner_source_key,
            references: output.clone(),
            deployment_runtime_authority_ref: handle.deployment_runtime_authority_ref().clone(),
            deployment_runtime_authority_sha256: handle
                .deployment_runtime_authority_sha256()
                .clone(),
            occurrence_id: InteractionOccurrenceIdV1::parse(occurrence_id)?,
            handle_coordinate,
            parent_placement: OwnerRecordKnowledgeV1::Unknown,
            edge_source_occurrence: predecessor
                .map(|value| (value.occurrence.0, value.occurrence.1)),
            edge_proof,
            edge_id: output
                .edge()
                .map(|_| GenealogyEdgeIdV1::parse(edge_id))
                .transpose()?,
            observation_coordinate,
            frontier_id: ReconciliationFrontierIdV1::parse(frontier_id)?,
            checkpoint_ref: checkpoint_ref.clone(),
            checkpoint_bytes_sha256: checkpoint_digest,
            observation_ref: observation_ref.clone(),
            observation_bytes_sha256: observation_digest,
            policy: operation.publication_policy().clone(),
        },
    })
}

/// Re-admit the Event and occurrence already projected for an A2A predecessor.
///
/// # Errors
/// Rejects an unlike deterministic identity, source meaning, policy, signer, or
/// retained canonical record.
#[allow(clippy::too_many_arguments)]
pub fn admit_a2a_task_predecessor_publication<EventAdmission, OccurrenceVerification>(
    store: &AuthorityStoreCoordinate,
    recovered: &RecoveredA2aTaskHistory,
    operation: &IssuedLocalOperation,
    event: (&ResidentRecordRefV1, &EventReferenceV1),
    occurrence: (&ResidentRecordRefV1, &InteractionOccurrenceV1),
    occurrence_context: SignedResidentAdmissionContextV1<'_>,
    admit_event_source: EventAdmission,
    verify_occurrence_signature: OccurrenceVerification,
) -> Result<A2aTaskPredecessorPublication, A2aTaskPublicationProjectionError>
where
    EventAdmission: FnOnce(&EventReferencePreimageV1) -> Result<(), InvalidResidentRecordV1>,
    OccurrenceVerification: FnOnce(
        &[u8],
        &InteractionOccurrencePreimageV1,
        &InteractionOccurrenceDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    let (observation_ref, observation) = recovered.selected_observation();
    let (handle_ref, handle) = operation.handle();
    let references = A2aTaskPublicationReferences::derive(store, observation_ref, observation)?;
    let observation_jcs = canon::to_canonical_string(observation)?;
    let handle_jcs = canon::to_canonical_string(handle)?;
    let event_schema = pale_contracts::record_schema_identity(EVENT_REFERENCE_V1_SCHEMA_ID)?;
    let admitted_event = EventReferenceV1::admit_contextual_wire(
        canonical_wire(event.1)?,
        &event_schema.schema_sha256,
        admit_event_source,
    )?;
    let expected_event = event_meaning(
        store,
        observation_ref,
        observation,
        handle,
        operation.publication_policy(),
        ArtifactDigest::parse(sha256_hex(observation_jcs.as_bytes()))?,
        *observation.observed_at(),
    )?;
    if event.0 != references.inner().event() || !expected_event.matches(&admitted_event) {
        return Err(A2aTaskPublicationProjectionError::Association);
    }
    admit_runtime_context(
        admitted_event.origin_principal_id(),
        handle.deployment_runtime_authority_ref(),
        handle.deployment_runtime_authority_sha256(),
        &occurrence_context,
    )?;
    let occurrence_schema =
        pale_contracts::record_schema_identity(INTERACTION_OCCURRENCE_V1_SCHEMA_ID)?;
    let handle_coordinate = owner_coordinate(
        handle_ref,
        handle.schema_id(),
        handle.schema_sha256(),
        ArtifactDigest::parse(sha256_hex(handle_jcs.as_bytes()))?,
        format!(
            "operation-handle:{}",
            handle.canonical_operation_handle_sha256().as_str()
        ),
    )?;
    let expected_occurrence = operation_occurrence_preimage(
        InteractionOccurrenceIdV1::parse(derived(
            "occurrence",
            store,
            observation_ref,
            observation.canonical_a2a_task_observation_sha256().as_str(),
        )?)?,
        event.0,
        &admitted_event,
        &handle_coordinate,
        OwnerRecordKnowledgeV1::Unknown,
        operation.publication_policy(),
        *admitted_event.recorded_at(),
        occurrence_schema.schema,
        occurrence_schema.schema_sha256,
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
                    record: "A2A task predecessor occurrence",
                    invariant: "exact admitted publication meaning",
                })
            }
        },
        verify_occurrence_signature,
    )?;
    if occurrence.0 != references.inner().occurrence() || &admitted_occurrence != occurrence.1 {
        return Err(A2aTaskPublicationProjectionError::Association);
    }
    Ok(A2aTaskPredecessorPublication {
        observation: (observation_ref.clone(), observation.clone()),
        event: (event.0.clone(), event.1.clone()),
        occurrence: (occurrence.0.clone(), occurrence.1.clone()),
    })
}

fn event_meaning(
    store: &AuthorityStoreCoordinate,
    observation_ref: &ResidentRecordRefV1,
    observation: &A2aTaskObservationV1,
    handle: &OperationHandleV1,
    policy: &super::LocalOperationPublicationPolicy,
    observation_digest: ArtifactDigest,
    minimum_recorded_at: UnixTimeV1,
) -> Result<LocalOperationEventMeaning, A2aTaskPublicationProjectionError> {
    let schema = pale_contracts::record_schema_identity(EVENT_REFERENCE_V1_SCHEMA_ID)?;
    let causation_id = match (
        observation.predecessor_ref(),
        observation.predecessor_sha256(),
    ) {
        (None, None) => None,
        (Some(reference), Some(digest)) => {
            Some(EventCausationIdV1::parse(derived_id_from_digest(
                "pale-a2a-task-observation-event-v1",
                store,
                reference,
                digest.as_str(),
            )?)?)
        }
        _ => return Err(A2aTaskPublicationProjectionError::Association),
    };
    Ok(LocalOperationEventMeaning {
        schema_id: schema.schema,
        schema_sha256: schema.schema_sha256,
        event_id: EventIdV1::parse(derived(
            "event",
            store,
            observation_ref,
            observation.canonical_a2a_task_observation_sha256().as_str(),
        )?)?,
        event_type: EventTypeV1::parse(EVENT_TYPE)?,
        event_version: 1,
        lifecycle_namespace: LifecycleNamespaceV1::parse(LIFECYCLE_NAMESPACE)?,
        origin_principal_id: observation.issuer_principal_id().clone(),
        source_owner: SourceOwnerV1::parse(PALE_OWNER)?,
        source_record_ref: observation_ref.clone(),
        source_record_schema_id: observation.schema_id().clone(),
        source_record_schema_sha256: observation.schema_sha256().clone(),
        source_record_content_sha256: observation_digest,
        source_version: SourceVersionV1::parse(observation.revision().get().to_string())?,
        target_scope: policy.target_scope().clone(),
        classification: policy.classification().clone(),
        ordering_key: EventOrderingKeyV1::parse(observation.task_binding_id().as_str())?,
        sequence: OwnerEventSequenceV1::new(observation.revision().get())?,
        occurred_at: *observation.observed_at(),
        minimum_recorded_at,
        correlation_id: EventCorrelationIdV1::parse(handle.request_binding().correlation_id())?,
        causation_id,
    })
}

#[allow(clippy::too_many_arguments)]
fn validate_predecessor(
    store: &AuthorityStoreCoordinate,
    operation: &IssuedLocalOperation,
    observation: &A2aTaskObservationV1,
    expected_ref: &ResidentRecordRefV1,
    handle_coordinate: &OwnerRecordCoordinateV1,
    policy: &super::LocalOperationPublicationPolicy,
    references: &LocalOperationPublicationReferences,
    prior: &A2aTaskPredecessorPublication,
) -> Result<(), A2aTaskPublicationProjectionError> {
    let (handle_ref, handle) = operation.handle();
    let prior_observation = &prior.observation.1;
    let prior_event = &prior.event.1;
    let prior_occurrence = &prior.occurrence.1;
    let prior_jcs = canon::to_canonical_string(prior_observation)?;
    let prior_digest = ArtifactDigest::parse(sha256_hex(prior_jcs.as_bytes()))?;
    let prior_event_id = derived(
        "event",
        store,
        &prior.observation.0,
        prior_observation
            .canonical_a2a_task_observation_sha256()
            .as_str(),
    )?;
    let prior_occurrence_id = derived(
        "occurrence",
        store,
        &prior.observation.0,
        prior_observation
            .canonical_a2a_task_observation_sha256()
            .as_str(),
    )?;
    if expected_ref != &prior.observation.0
        || observation.predecessor_sha256().as_ref()
            != Some(prior_observation.canonical_a2a_task_observation_sha256())
        || prior_observation.task_binding_id() != observation.task_binding_id()
        || prior_event.source_record_ref() != &prior.observation.0
        || prior_event.origin_principal_id() != observation.issuer_principal_id()
        || prior_event.event_id().as_str() != prior_event_id
        || prior_event.event_type().as_str() != EVENT_TYPE
        || prior_event.lifecycle_namespace().as_str() != LIFECYCLE_NAMESPACE
        || prior_event.source_owner().as_str() != PALE_OWNER
        || prior_event.source_record_schema_id() != prior_observation.schema_id()
        || prior_event.source_record_schema_sha256() != prior_observation.schema_sha256()
        || prior_event.source_record_content_sha256() != &prior_digest
        || prior_event.source_version().as_str() != prior_observation.revision().get().to_string()
        || prior_event
            .ordering_key()
            .as_ref()
            .map(EventOrderingKeyV1::as_str)
            != Some(observation.task_binding_id().as_str())
        || prior_event.sequence().as_ref().map(|value| value.get())
            != Some(prior_observation.revision().get())
        || prior_event.occurred_at() != prior_observation.observed_at()
        || prior_event.correlation_id().as_str() != handle.request_binding().correlation_id()
        || prior_occurrence.event_reference_ref() != &prior.event.0
        || prior_occurrence.event_reference_sha256() != prior_event.canonical_event_sha256()
        || prior_occurrence.occurrence_id().as_str() != prior_occurrence_id
        || prior_occurrence.native_container() != handle_coordinate
        || prior_occurrence.container_genre().as_str() != "operation"
        || prior_occurrence.actor_provenance() != policy.actor_provenance()
        || &prior.occurrence.0 == references.occurrence()
        || handle_ref == &prior.observation.0
    {
        return Err(A2aTaskPublicationProjectionError::Association);
    }
    Ok(())
}

fn owner_coordinate(
    record_ref: &ResidentRecordRefV1,
    schema_id: &SchemaId,
    schema_sha256: &SchemaDigest,
    content_sha256: ArtifactDigest,
    revision: String,
) -> Result<OwnerRecordCoordinateV1, InvalidResidentRecordV1> {
    Ok(OwnerRecordCoordinateV1 {
        owner_system_id: OwnerSystemIdV1::parse(PALE_OWNER)?,
        record_ref: record_ref.clone(),
        record_schema_id: schema_id.clone(),
        record_schema_sha256: schema_sha256.clone(),
        record_content_sha256: content_sha256,
        native_revision: NativeRevisionV1 {
            native_revision_owner: NativeRevisionOwnerV1::parse(PALE_OWNER)?,
            native_revision_token: NativeRevisionTokenV1::parse(revision)?,
        },
    })
}

fn derived(
    kind: &str,
    store: &AuthorityStoreCoordinate,
    reference: &ResidentRecordRefV1,
    digest: &str,
) -> Result<String, canon::CanonError> {
    derived_id_from_digest(
        &format!("pale-a2a-task-observation-{kind}-v1"),
        store,
        reference,
        digest,
    )
}

fn canonical_wire<W, R>(record: &R) -> Result<W, A2aTaskPublicationProjectionError>
where
    W: serde::de::DeserializeOwned,
    R: serde::Serialize,
{
    serde_json::from_str(&canon::to_canonical_string(record)?).map_err(|_| {
        A2aTaskPublicationProjectionError::Admission(InvalidResidentRecordV1::NonCanonical {
            relation: "A2A task predecessor wire",
        })
    })
}

/// An A2A task observation could not enter the shared publication plan.
#[derive(Debug, thiserror::Error)]
pub enum A2aTaskPublicationProjectionError {
    /// The physical source belongs to another record family.
    #[error("publication source is not an A2A task observation")]
    UnsupportedSource,
    /// Recovered source, checkpoint, operation, task, or policy evidence differs.
    #[error("A2A task publication evidence does not form one exact lineage")]
    Association,
    /// A successor has no admitted predecessor projection.
    #[error("A2A task successor is missing predecessor publication evidence")]
    MissingPredecessor,
    /// Revision one incorrectly supplied predecessor projection evidence.
    #[error("initial A2A task publication cannot carry predecessor evidence")]
    UnexpectedPredecessor,
    /// The shared publication plan rejected a derived value.
    #[error(transparent)]
    Shared(#[from] LocalOperationPublicationProjectionError),
    /// A public record rejected its shape or context.
    #[error(transparent)]
    Admission(#[from] InvalidResidentRecordV1),
    /// Contract schema identity is unavailable.
    #[error(transparent)]
    Schema(#[from] pale_contracts::SchemaError),
    /// Canonical encoding failed.
    #[error(transparent)]
    Canonical(#[from] canon::CanonError),
    /// A computed digest was malformed.
    #[error(transparent)]
    Digest(#[from] InvalidDigest),
    /// Event index key projection failed.
    #[error(transparent)]
    EventProjection(#[from] super::EventCustodyProjectionError),
}
