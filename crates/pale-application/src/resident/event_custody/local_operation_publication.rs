//! Restart-safe bounded publication of one local operation observation.
//!
//! One call readmits retained stages, then signs and commits at most the first

//! absent stage. Historical bytes remain dependencies after key rotation.

use pale_core::{canon, records::*};
use pale_ports::{
    AuthorityStoreCoordinate, ControlRecord, ControlTransactionPort, EventCustodyPort,
    EventIndexQuery,
};

use super::local_operation_projection::admit_local_operation_predecessor_publication_records;
use super::projection::{
    genealogy_edge_key_from_parts, occurrence_key, owner_source_key_from_parts,
    reconciliation_frontier_key_from_subject,
};
use super::{
    AdmittedA2aTaskPublicationPlan, AdmittedEventRecord, AdmittedLocalOperationPublicationPlan,
    AdmittedOwnerPublication, EventCustodyTransitionInput, EventIdentityLookupError,
    EventPublicationRaceVerifier, EventPublicationState, LocalOperationPublicationProjectionError,
    LocalOperationPublicationSigningContexts,
};
use crate::resident::{
    IssuedLocalOperation, RecoveredA2aTaskHistory, RecoveredLocalOperation,
    RecoveredOperationObservation,
};
use crate::resident::{ResidentApplication, ResidentApplicationError, ResidentCommitReceipt};

type OccurrenceSigner<'a> = dyn FnMut(
        &[u8],
        &InteractionOccurrencePreimageV1,
        &InteractionOccurrenceDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
    + 'a;
type EdgeSigner<'a> = dyn FnMut(
        &[u8],
        &GenealogyEdgePreimageV1,
        &GenealogyEdgeDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
    + 'a;
type FrontierSigner<'a> = dyn FnMut(
        &[u8],
        &ReconciliationFrontierPreimageV1,
        &ReconciliationFrontierDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>
    + 'a;
type AuthorityResolver<'a> =
    dyn FnMut(&ControlRecord) -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1> + 'a;
type EventSourceAdmission<'a> =
    dyn FnMut(&EventReferencePreimageV1) -> Result<(), InvalidResidentRecordV1> + 'a;
type OccurrenceVerifier<'a> = dyn FnMut(
        &[u8],
        &InteractionOccurrencePreimageV1,
        &InteractionOccurrenceDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;
type EdgeVerifier<'a> = dyn FnMut(
        &[u8],
        &GenealogyEdgePreimageV1,
        &GenealogyEdgeDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;
type FrontierVerifier<'a> = dyn FnMut(
        &[u8],
        &ReconciliationFrontierPreimageV1,
        &ReconciliationFrontierDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;

/// Current signers, used only for a stage proved absent.
pub struct LocalOperationPublicationIssuance<'a> {
    /// Sign a newly issued occurrence.
    pub sign_occurrence: &'a mut OccurrenceSigner<'a>,
    /// Sign a newly issued Genealogy edge.
    pub sign_edge: &'a mut EdgeSigner<'a>,
    /// Sign a newly issued reconciliation frontier.
    pub sign_frontier: &'a mut FrontierSigner<'a>,
}

/// Family-specific authority resolution and verification for retained stages.
pub struct LocalOperationPublicationReadmission<'a> {
    /// Re-admit the historical Event's owner publication and producer evidence.
    pub admit_event_source: &'a mut EventSourceAdmission<'a>,
    /// Resolve the historical occurrence authority.
    pub resolve_occurrence_authority: &'a mut AuthorityResolver<'a>,
    /// Verify the retained occurrence signature.
    pub verify_occurrence_signature: &'a mut OccurrenceVerifier<'a>,
    /// Resolve the historical edge authority.
    pub resolve_edge_authority: &'a mut AuthorityResolver<'a>,
    /// Verify the retained edge signature.
    pub verify_edge_signature: &'a mut EdgeVerifier<'a>,
    /// Resolve the historical frontier authority.
    pub resolve_frontier_authority: &'a mut AuthorityResolver<'a>,
    /// Verify the retained frontier signature.
    pub verify_frontier_signature: &'a mut FrontierVerifier<'a>,
}

/// Read and admit the exact predecessor Event and occurrence for one successor.
///
/// Both lookups are point reads derived from the predecessor observation. The
/// returned value owns its admitted evidence so a later plan cannot outlive the
/// historical recovery that selected it.
pub(super) fn read_local_operation_predecessor_publication(
    port: &mut dyn EventCustodyPort,
    store: &AuthorityStoreCoordinate,
    recovered: &RecoveredOperationObservation,
    operation: &RecoveredLocalOperation,
    readmission: &mut LocalOperationPublicationReadmission<'_>,
    maximum_record_bytes: usize,
) -> Result<super::LocalOperationPredecessorPublication, LocalOperationPublicationAdvanceError> {
    if maximum_record_bytes == 0 {
        return Err(LocalOperationPublicationAdvanceError::Bounds);
    }
    let (observation_ref, observation) = recovered.selected();
    let observation_jcs = canon::to_canonical_string(observation)
        .map_err(LocalOperationPublicationProjectionError::from)?;
    let observation_digest =
        ArtifactDigest::parse(pale_core::sha256_hex(observation_jcs.as_bytes()))
            .map_err(LocalOperationPublicationProjectionError::from)?;
    let owner =
        SourceOwnerV1::parse("pale").map_err(LocalOperationPublicationProjectionError::from)?;
    let version = SourceVersionV1::parse(observation.revision().get().to_string())
        .map_err(LocalOperationPublicationProjectionError::from)?;
    let owner_source = owner_source_key_from_parts(
        observation.issuer_principal_id(),
        &owner,
        observation_ref,
        observation.schema_id(),
        observation.schema_sha256(),
        &observation_digest,
        &version,
    )?;
    let event_query = EventIndexQuery::OwnerSource(owner_source);
    let event = read_index(port, store, &event_query, maximum_record_bytes)?
        .ok_or(LocalOperationPublicationAdvanceError::MissingPredecessor)?;
    let event_digest = EventReferenceDigest::parse(event.coordinate().record_sha256().as_str())
        .map_err(LocalOperationPublicationProjectionError::from)?;
    let references =
        super::LocalOperationPublicationReferences::derive(store, observation_ref, observation)?;
    if event.coordinate().record_ref() != references.event() {
        return Err(LocalOperationPublicationAdvanceError::Conflict);
    }
    let occurrence_query =
        EventIndexQuery::InteractionOccurrence(occurrence_key(references.event(), &event_digest)?);
    let occurrence = read_index(port, store, &occurrence_query, maximum_record_bytes)?
        .ok_or(LocalOperationPublicationAdvanceError::MissingPredecessor)?;
    admit_local_operation_predecessor_publication_records(
        store,
        recovered,
        operation,
        &event,
        &occurrence,
        |record| (readmission.resolve_occurrence_authority)(record),
        |preimage| (readmission.admit_event_source)(preimage),
        |domain, preimage, digest, signature| {
            (readmission.verify_occurrence_signature)(domain, preimage, digest, signature)
        },
    )
    .map_err(Into::into)
}

/// Read and admit the exact predecessor Event and occurrence for one A2A successor.
pub(super) fn read_a2a_task_predecessor_publication(
    port: &mut dyn EventCustodyPort,
    store: &AuthorityStoreCoordinate,
    recovered: &RecoveredA2aTaskHistory,
    operation: &IssuedLocalOperation,
    readmission: &mut LocalOperationPublicationReadmission<'_>,
    maximum_record_bytes: usize,
) -> Result<super::A2aTaskPredecessorPublication, LocalOperationPublicationAdvanceError> {
    if maximum_record_bytes == 0 {
        return Err(LocalOperationPublicationAdvanceError::Bounds);
    }
    let (observation_ref, observation) = recovered.selected_observation();
    let observation_jcs = canon::to_canonical_string(observation)
        .map_err(LocalOperationPublicationProjectionError::from)?;
    let observation_digest =
        ArtifactDigest::parse(pale_core::sha256_hex(observation_jcs.as_bytes()))
            .map_err(LocalOperationPublicationProjectionError::from)?;
    let owner =
        SourceOwnerV1::parse("pale").map_err(LocalOperationPublicationProjectionError::from)?;
    let version = SourceVersionV1::parse(observation.revision().get().to_string())
        .map_err(LocalOperationPublicationProjectionError::from)?;
    let owner_source = owner_source_key_from_parts(
        observation.issuer_principal_id(),
        &owner,
        observation_ref,
        observation.schema_id(),
        observation.schema_sha256(),
        &observation_digest,
        &version,
    )?;
    let event = read_index(
        port,
        store,
        &EventIndexQuery::OwnerSource(owner_source),
        maximum_record_bytes,
    )?
    .ok_or(LocalOperationPublicationAdvanceError::MissingPredecessor)?;
    let event_schema = schema(EVENT_REFERENCE_V1_SCHEMA_ID)?;
    let event_value = EventReferenceV1::admit_contextual_wire(
        decode_wire(&event)?,
        &event_schema.schema_sha256,
        |_| Ok(()),
    )?;
    let event_digest = EventReferenceDigest::parse(event.coordinate().record_sha256().as_str())
        .map_err(LocalOperationPublicationProjectionError::from)?;
    let references =
        super::A2aTaskPublicationReferences::derive(store, observation_ref, observation)?;
    if event.coordinate().record_ref() != references.inner().event() {
        return Err(LocalOperationPublicationAdvanceError::Conflict);
    }
    let occurrence = read_index(
        port,
        store,
        &EventIndexQuery::InteractionOccurrence(occurrence_key(
            references.inner().event(),
            &event_digest,
        )?),
        maximum_record_bytes,
    )?
    .ok_or(LocalOperationPublicationAdvanceError::MissingPredecessor)?;
    let occurrence_schema = schema(INTERACTION_OCCURRENCE_V1_SCHEMA_ID)?;
    let authority = (readmission.resolve_occurrence_authority)(&occurrence)?;
    let occurrence_context = context(&occurrence_schema, &authority);
    let occurrence_value = InteractionOccurrenceV1::admit_contextual_wire(
        decode_wire(&occurrence)?,
        &occurrence_context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )?;
    super::admit_a2a_task_predecessor_publication(
        store,
        recovered,
        operation,
        (event.coordinate().record_ref(), &event_value),
        (occurrence.coordinate().record_ref(), &occurrence_value),
        occurrence_context,
        |preimage| (readmission.admit_event_source)(preimage),
        |domain, preimage, digest, signature| {
            (readmission.verify_occurrence_signature)(domain, preimage, digest, signature)
        },
    )
    .map_err(Into::into)
}

/// Ordered stages of one local operation publication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalOperationPublicationStage {
    /// Immutable Event reference.
    Event,
    /// Event placement in interaction context.
    Occurrence,
    /// Structural parent-child or same-operation successor relationship.
    GenealogyEdge,
    /// Owner-publication reconciliation evidence.
    ReconciliationFrontier,
}

/// Result of one bounded publication advance.
pub enum LocalOperationPublicationAdvance {
    /// One stage was established by a new commit or exact replay.
    Established {
        /// Stage established by this call.
        stage: LocalOperationPublicationStage,
        /// How exact Event custody was established for this stage.
        evidence: super::EventCustodyCommitEvidence,
        /// Durable resident journal coordinate.
        receipt: Box<ResidentCommitReceipt>,
    },
    /// Every stage exists and passed contextual readmission.
    Complete,
}

/// One publication advance could not prove a safe next step.
#[derive(Debug, thiserror::Error)]
pub enum LocalOperationPublicationAdvanceError {
    /// A canonical key projection failed.
    #[error(transparent)]
    Lookup(#[from] EventIdentityLookupError),
    /// A semantic-to-physical key projection failed.
    #[error(transparent)]
    Key(#[from] super::EventCustodyProjectionError),
    /// Storage failed a point read or transaction preparation.
    #[error(transparent)]
    Store(#[from] pale_ports::ControlStoreError),
    /// Resident authority could not commit the selected stage.
    #[error(transparent)]
    Resident(#[from] ResidentApplicationError),
    /// Publication-plan construction or delayed issuance failed.
    #[error(transparent)]
    Projection(#[from] LocalOperationPublicationProjectionError),
    /// A2A source projection or predecessor admission failed.
    #[error(transparent)]
    A2aProjection(#[from] super::A2aTaskPublicationProjectionError),
    /// A retained public record failed contextual admission.
    #[error(transparent)]
    Admission(#[from] InvalidResidentRecordV1),
    /// A positive point-read bound was not supplied.
    #[error("local operation publication point-read bound is invalid")]
    Bounds,
    /// A successor observation has no admitted predecessor publication.
    #[error("local operation predecessor publication is absent")]
    MissingPredecessor,
    /// Retained bytes are not strict canonical JSON for their record family.
    #[error("retained operation publication bytes are not canonical")]
    InvalidRetainedRecord,
    /// A point read returned another query.
    #[error("operation publication point read returned another query")]
    MisdirectedObservation,
    /// Retained custody and the admitted plan describe unlike meaning.
    #[error("operation publication identity conflicts with retained custody")]
    Conflict,
    /// The plan and deterministic references disagree.
    #[error("operation publication plan does not match its references")]
    ReferenceMismatch,
}

/// Advance one publication by at most one durable stage.
///
/// Retained stages use their historical authorities. Current signers are
/// consulted only after a point read proves their stage absent.
///
/// # Errors
/// Returns [`LocalOperationPublicationAdvanceError`] for invalid retained
/// evidence, conflicting meaning, storage failure, or failed commitment.
#[allow(clippy::too_many_arguments)]
pub fn advance_local_operation_publication<P>(
    application: &mut ResidentApplication<'_>,
    port: &mut P,
    plan: &AdmittedLocalOperationPublicationPlan,
    producer_allocation_ref: &ResidentRecordRefV1,
    producer: &ProducerIncarnationV1,
    writer_fence_ref: &ResidentRecordRefV1,
    writer_fence: &PrincipalResidentWriterFenceV1,
    publication_state: EventPublicationState<'_>,
    race_verifier: &dyn EventPublicationRaceVerifier,
    signing_contexts: LocalOperationPublicationSigningContexts<'_>,
    issuance: &mut LocalOperationPublicationIssuance<'_>,
    readmission: &mut LocalOperationPublicationReadmission<'_>,
    maximum_publication_bytes: usize,
) -> Result<LocalOperationPublicationAdvance, LocalOperationPublicationAdvanceError>
where
    P: EventCustodyPort + ControlTransactionPort,
{
    let store = application.store().clone();
    let references = plan.references();
    let Some(event) = read_event(port, &store, plan, readmission, maximum_publication_bytes)?
    else {
        let candidate =
            plan.issue_event(producer, writer_fence, &mut readmission.admit_event_source)?;
        let owner_publication = AdmittedOwnerPublication::new(
            AdmittedEventRecord::new(plan.event_ref(), &candidate),
            producer_allocation_ref,
            producer,
            writer_fence_ref,
            writer_fence,
            publication_state,
            race_verifier,
        );
        let transaction = port.prepare_transaction(&store)?;
        let (completed, receipt) = application.commit_event_custody(
            port,
            transaction,
            EventCustodyTransitionInput::AdmitOwnerReference(owner_publication),
        )?;
        return committed(
            LocalOperationPublicationStage::Event,
            completed.evidence(),
            receipt,
        );
    };

    let occurrence = match read_occurrence(
        port,
        &store,
        plan,
        &event,
        readmission,
        maximum_publication_bytes,
    )? {
        Some(value) => value,
        None => {
            let value = plan.issue_occurrence(
                &event,
                signing_contexts.occurrence,
                &mut issuance.sign_occurrence,
            )?;
            let transaction = port.prepare_transaction(&store)?;
            let (completed, receipt) = application.commit_event_custody(
                port,
                transaction,
                EventCustodyTransitionInput::ProjectInteractionOccurrence {
                    event: AdmittedEventRecord::new(references.event(), &event),
                    occurrence: AdmittedEventRecord::new(references.occurrence(), &value),
                    retention_record_ref: references.occurrence_retention(),
                    maximum_publication_bytes,
                },
            )?;
            return committed(
                LocalOperationPublicationStage::Occurrence,
                completed.evidence(),
                receipt,
            );
        }
    };

    if references.edge().is_some()
        && read_edge(
            port,
            &store,
            plan,
            &occurrence,
            readmission,
            maximum_publication_bytes,
        )?
        .is_none()
    {
        let value = plan
            .issue_edge(&occurrence, signing_contexts.edge, &mut issuance.sign_edge)?
            .ok_or(LocalOperationPublicationAdvanceError::ReferenceMismatch)?;
        let (source_ref, source) = plan
            .edge_source_occurrence()
            .ok_or(LocalOperationPublicationAdvanceError::ReferenceMismatch)?;
        let edge_ref = references
            .edge()
            .ok_or(LocalOperationPublicationAdvanceError::ReferenceMismatch)?;
        let retention_ref = references
            .edge_retention()
            .ok_or(LocalOperationPublicationAdvanceError::ReferenceMismatch)?;
        let transaction = port.prepare_transaction(&store)?;
        let (completed, receipt) = application.commit_event_custody(
            port,
            transaction,
            EventCustodyTransitionInput::ProjectGenealogyEdge {
                source: AdmittedEventRecord::new(source_ref, source),
                destination: AdmittedEventRecord::new(references.occurrence(), &occurrence),
                edge: AdmittedEventRecord::new(edge_ref, &value),
                retention_record_ref: retention_ref,
                maximum_publication_bytes,
            },
        )?;
        return committed(
            LocalOperationPublicationStage::GenealogyEdge,
            completed.evidence(),
            receipt,
        );
    }

    if read_frontier(
        port,
        &store,
        plan,
        &occurrence,
        readmission,
        maximum_publication_bytes,
    )?
    .is_some()
    {
        return Ok(LocalOperationPublicationAdvance::Complete);
    }
    let value = plan.issue_frontier(
        &occurrence,
        signing_contexts.frontier,
        &mut issuance.sign_frontier,
    )?;
    let transaction = port.prepare_transaction(&store)?;
    let (completed, receipt) = application.commit_event_custody(
        port,
        transaction,
        EventCustodyTransitionInput::AdvanceReconciliationFrontier {
            predecessor: None,
            successor: AdmittedEventRecord::new(references.frontier(), &value),
            retention_record_ref: references.frontier_retention(),
            maximum_publication_bytes,
        },
    )?;
    committed(
        LocalOperationPublicationStage::ReconciliationFrontier,
        completed.evidence(),
        receipt,
    )
}

/// Advance one admitted A2A task publication through the shared durable stages.
///
/// Remote task evidence uses the same Event, occurrence, Genealogy, frontier,
/// replay, and outbox owner as local operation observations. This function does
/// not contact the remote agent or reinterpret its state as Pale completion.
///
/// # Errors
/// Returns the shared publication error for conflicting retained evidence,
/// failed admission, storage failure, or an invalid bound.
#[allow(clippy::too_many_arguments)]
pub fn advance_a2a_task_publication<P>(
    application: &mut ResidentApplication<'_>,
    port: &mut P,
    plan: &AdmittedA2aTaskPublicationPlan,
    producer_allocation_ref: &ResidentRecordRefV1,
    producer: &ProducerIncarnationV1,
    writer_fence_ref: &ResidentRecordRefV1,
    writer_fence: &PrincipalResidentWriterFenceV1,
    publication_state: EventPublicationState<'_>,
    race_verifier: &dyn EventPublicationRaceVerifier,
    signing_contexts: LocalOperationPublicationSigningContexts<'_>,
    issuance: &mut LocalOperationPublicationIssuance<'_>,
    readmission: &mut LocalOperationPublicationReadmission<'_>,
    maximum_publication_bytes: usize,
) -> Result<LocalOperationPublicationAdvance, LocalOperationPublicationAdvanceError>
where
    P: EventCustodyPort + ControlTransactionPort,
{
    advance_local_operation_publication(
        application,
        port,
        &plan.inner,
        producer_allocation_ref,
        producer,
        writer_fence_ref,
        writer_fence,
        publication_state,
        race_verifier,
        signing_contexts,
        issuance,
        readmission,
        maximum_publication_bytes,
    )
}

fn committed(
    stage: LocalOperationPublicationStage,
    evidence: super::EventCustodyCommitEvidence,
    receipt: ResidentCommitReceipt,
) -> Result<LocalOperationPublicationAdvance, LocalOperationPublicationAdvanceError> {
    Ok(LocalOperationPublicationAdvance::Established {
        stage,
        evidence,
        receipt: Box::new(receipt),
    })
}

fn read_event(
    port: &mut dyn EventCustodyPort,
    store: &AuthorityStoreCoordinate,
    plan: &AdmittedLocalOperationPublicationPlan,
    readmission: &mut LocalOperationPublicationReadmission<'_>,
    maximum_record_bytes: usize,
) -> Result<Option<EventReferenceV1>, LocalOperationPublicationAdvanceError> {
    let query = EventIndexQuery::OwnerSource(plan.owner_source_key().clone());
    let Some(record) = read_index(port, store, &query, maximum_record_bytes)? else {
        return Ok(None);
    };
    let schema = schema(EVENT_REFERENCE_V1_SCHEMA_ID)?;
    let event = EventReferenceV1::admit_contextual_wire(
        decode_wire(&record)?,
        &schema.schema_sha256,
        &mut readmission.admit_event_source,
    )?;
    if !plan.event_meaning_matches(&event) {
        return Err(LocalOperationPublicationAdvanceError::Conflict);
    }
    verify_record(
        &record,
        plan.event_ref(),
        &schema,
        event.canonical_event_sha256().as_str(),
        &event,
    )?;
    Ok(Some(event))
}

fn read_occurrence(
    port: &mut dyn EventCustodyPort,
    store: &AuthorityStoreCoordinate,
    plan: &AdmittedLocalOperationPublicationPlan,
    event: &EventReferenceV1,
    readmission: &mut LocalOperationPublicationReadmission<'_>,
    maximum_record_bytes: usize,
) -> Result<Option<InteractionOccurrenceV1>, LocalOperationPublicationAdvanceError> {
    let query = EventIndexQuery::InteractionOccurrence(occurrence_key(
        plan.references().event(),
        event.canonical_event_sha256(),
    )?);
    let Some(record) = read_index(port, store, &query, maximum_record_bytes)? else {
        return Ok(None);
    };
    let schema = schema(INTERACTION_OCCURRENCE_V1_SCHEMA_ID)?;
    let authority = (readmission.resolve_occurrence_authority)(&record)?;
    let context = context(&schema, &authority);
    plan.admit_runtime_context(event.origin_principal_id(), &context)?;
    let expected = plan.occurrence_preimage(
        event,
        schema.schema.clone(),
        schema.schema_sha256.clone(),
        &context,
    )?;
    let value = InteractionOccurrenceV1::admit_contextual_wire(
        decode_wire(&record)?,
        &context,
        |actual| exact_preimage(actual, &expected),
        &mut readmission.verify_occurrence_signature,
    )?;
    verify_record(
        &record,
        plan.references().occurrence(),
        &schema,
        value.canonical_occurrence_sha256().as_str(),
        &value,
    )?;
    Ok(Some(value))
}

fn read_edge(
    port: &mut dyn EventCustodyPort,
    store: &AuthorityStoreCoordinate,
    plan: &AdmittedLocalOperationPublicationPlan,
    destination: &InteractionOccurrenceV1,
    readmission: &mut LocalOperationPublicationReadmission<'_>,
    maximum_record_bytes: usize,
) -> Result<Option<GenealogyEdgeV1>, LocalOperationPublicationAdvanceError> {
    let (edge_id, source_ref, destination_ref, proof) = plan
        .edge_identity()
        .ok_or(LocalOperationPublicationAdvanceError::ReferenceMismatch)?;
    let query = EventIndexQuery::GenealogyEdge(genealogy_edge_key_from_parts(
        edge_id,
        source_ref,
        destination_ref,
        &proof,
    )?);
    let Some(record) = read_index(port, store, &query, maximum_record_bytes)? else {
        return Ok(None);
    };
    let schema = schema(GENEALOGY_EDGE_V1_SCHEMA_ID)?;
    let authority = (readmission.resolve_edge_authority)(&record)?;
    let context = context(&schema, &authority);
    plan.admit_runtime_context(destination.issuer_principal_id(), &context)?;
    let expected = plan
        .edge_preimage(destination, &context)?
        .ok_or(LocalOperationPublicationAdvanceError::ReferenceMismatch)?;
    let value = GenealogyEdgeV1::admit_contextual_wire(
        decode_wire(&record)?,
        &context,
        |actual| exact_preimage(actual, &expected),
        &mut readmission.verify_edge_signature,
    )?;
    verify_record(
        &record,
        plan.references()
            .edge()
            .ok_or(LocalOperationPublicationAdvanceError::ReferenceMismatch)?,
        &schema,
        value.canonical_genealogy_edge_sha256().as_str(),
        &value,
    )?;
    Ok(Some(value))
}

fn read_frontier(
    port: &mut dyn EventCustodyPort,
    store: &AuthorityStoreCoordinate,
    plan: &AdmittedLocalOperationPublicationPlan,
    occurrence: &InteractionOccurrenceV1,
    readmission: &mut LocalOperationPublicationReadmission<'_>,
    maximum_record_bytes: usize,
) -> Result<Option<ReconciliationFrontierV1>, LocalOperationPublicationAdvanceError> {
    let query = EventIndexQuery::ReconciliationFrontier(reconciliation_frontier_key_from_subject(
        &plan.frontier_subject(),
    )?);
    let Some(record) = read_index(port, store, &query, maximum_record_bytes)? else {
        return Ok(None);
    };
    let schema = schema(RECONCILIATION_FRONTIER_V1_SCHEMA_ID)?;
    let authority = (readmission.resolve_frontier_authority)(&record)?;
    let context = context(&schema, &authority);
    plan.admit_runtime_context(occurrence.issuer_principal_id(), &context)?;
    let expected = plan.frontier_preimage(
        occurrence,
        schema.schema.clone(),
        schema.schema_sha256.clone(),
        &context,
    )?;
    let value = ReconciliationFrontierV1::admit_contextual_wire(
        decode_wire(&record)?,
        &context,
        |actual| exact_preimage(actual, &expected),
        &mut readmission.verify_frontier_signature,
    )?;
    verify_record(
        &record,
        plan.references().frontier(),
        &schema,
        value.canonical_reconciliation_frontier_sha256().as_str(),
        &value,
    )?;
    Ok(Some(value))
}

fn read_index(
    port: &mut dyn EventCustodyPort,
    store: &AuthorityStoreCoordinate,
    query: &EventIndexQuery,
    maximum_record_bytes: usize,
) -> Result<Option<ControlRecord>, LocalOperationPublicationAdvanceError> {
    if maximum_record_bytes == 0 {
        return Err(LocalOperationPublicationAdvanceError::Bounds);
    }
    let observed = port.read_index(store, query, maximum_record_bytes)?;
    if observed.query() != query {
        return Err(LocalOperationPublicationAdvanceError::MisdirectedObservation);
    }
    Ok(observed.record().cloned())
}

fn schema(id: &str) -> Result<RecordSchemaIdentity, LocalOperationPublicationAdvanceError> {
    pale_contracts::record_schema_identity(id)
        .map_err(LocalOperationPublicationProjectionError::from)
        .map_err(Into::into)
}

fn context<'a>(
    schema: &'a RecordSchemaIdentity,
    authority: &'a SigningAuthorityBindingV1,
) -> SignedResidentAdmissionContextV1<'a> {
    SignedResidentAdmissionContextV1 {
        expected_schema_sha256: &schema.schema_sha256,
        signing_authority: authority,
    }
}

fn decode_wire<W: serde::de::DeserializeOwned>(
    record: &ControlRecord,
) -> Result<W, LocalOperationPublicationAdvanceError> {
    let value = canon::parse_strict(record.canonical_record_jcs().as_slice())
        .map_err(|_| LocalOperationPublicationAdvanceError::InvalidRetainedRecord)?;
    serde_json::from_value(value)
        .map_err(|_| LocalOperationPublicationAdvanceError::InvalidRetainedRecord)
}

fn exact_preimage<T: PartialEq>(actual: &T, expected: &T) -> Result<(), InvalidResidentRecordV1> {
    if actual == expected {
        Ok(())
    } else {
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "local operation publication",
            invariant: "exact admitted publication meaning",
        })
    }
}

fn verify_record<T: serde::Serialize>(
    record: &ControlRecord,
    expected_ref: &ResidentRecordRefV1,
    expected_schema: &RecordSchemaIdentity,
    expected_digest: &str,
    admitted: &T,
) -> Result<(), LocalOperationPublicationAdvanceError> {
    let canonical = canon::to_canonical_string(admitted)
        .map_err(|_| LocalOperationPublicationAdvanceError::InvalidRetainedRecord)?;
    if record.coordinate().record_ref() != expected_ref
        || record.coordinate().record_sha256().as_str() != expected_digest
        || record.schema_id() != &expected_schema.schema
        || record.schema_sha256() != &expected_schema.schema_sha256
        || record.canonical_record_jcs().as_slice() != canonical.as_bytes()
    {
        return Err(LocalOperationPublicationAdvanceError::Conflict);
    }
    Ok(())
}
