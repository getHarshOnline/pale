//! Remote task history under existing operation custody, never local settlement.
//! Evidence owners must establish an actual authorized exchange. A retained
//! dispatched observation alone is not a send receipt or current access grant.

use super::{record_projection, OperationObservationCustodyProjectionError};
use pale_core::{canon, records::*};
use pale_ports::*;

mod known_task;
pub(super) use known_task::KnownA2aTaskBasis;

mod cancel_task;
pub(in crate::resident) use cancel_task::ContinuableA2aCancelTaskCustodyFailure;
pub(in crate::resident) use cancel_task::{
    commit_a2a_cancel_task_disposition_continuable, commit_a2a_cancel_task_readback_continuable,
    A2aCancelTaskDispositionCommitInput, A2aCancelTaskDispositionPhase,
    A2aCancelTaskReadbackCommitInput, CompletedA2aCancelTaskDispositionCustody,
    CompletedA2aCancelTaskReadbackCustody, ContinuableA2aCancelTaskDispositionCommitFailure,
    ContinuableA2aCancelTaskReadbackCommitFailure,
};
pub use cancel_task::{
    prepare_a2a_cancel_task_custody, start_a2a_cancel_task, A2aCancelTaskAuthorizationContext,
    A2aCancelTaskDispositionFailure, A2aCancelTaskDispositionInput, A2aCancelTaskError,
    A2aCancelTaskInput, A2aCancelTaskNotStarted, A2aCancelTaskObservationInput,
    A2aCancelTaskOutcomeUnknown, A2aCancelTaskPoll, A2aCancelTaskStartFailure,
    A2aCancelTaskUncertainty, CompletedA2aCancelTaskReadback,
    DatabaseCommittedA2aCancelTaskAnchorPending, InFlightA2aCancelTask,
    PreparedA2aCancelTaskCustody, PreparedA2aCancelTaskDisposition, ReceivedA2aCancelTaskResponse,
    RetainedA2aCancelTask,
};

mod get_task;
pub(in crate::resident) use get_task::custody::{
    prepare_a2a_get_task_custody, recover_a2a_get_task_custody, A2aGetTaskCustodyFailure,
    A2aGetTaskCustodyRecoveryError, A2aGetTaskLocalDispositionBasis,
    A2aGetTaskLocalDispositionKind, A2aGetTaskPreparationInput, A2aGetTaskStartFailure,
    PreparedA2aGetTaskCustody, RecoveredA2aGetTaskCustody, RetainedA2aGetTask,
};
pub use get_task::{
    admit_a2a_get_task_evidence, commit_a2a_get_task_readback, start_a2a_get_task_readback,
    A2aGetTaskCommitFailure, A2aGetTaskCommitInput, A2aGetTaskObservationInput,
    A2aGetTaskReadbackAuthorizationContext, A2aGetTaskReadbackError, A2aGetTaskReadbackInput,
    A2aGetTaskReadbackPoll, CompletedA2aGetTaskReadback, CompletedA2aGetTaskReadbackCustody,
    InFlightA2aGetTaskReadback, ReceivedA2aGetTaskReadback,
};
pub(in crate::resident) use get_task::{
    commit_a2a_get_task_readback_continuable, ContinuableA2aGetTaskCommitFailure,
};
mod checkpoint;
pub use checkpoint::PreparedA2aTaskCheckpoint;
mod service;
pub use service::{
    commit_a2a_initial_task_response, A2aInitialTaskCommitFailure, A2aInitialTaskCommitInput,
    CompletedA2aInitialTaskCustody,
};
pub(in crate::resident) use service::{
    commit_a2a_initial_task_response_continuable, ContinuableA2aInitialTaskCommitFailure,
};
mod recovery;
pub use recovery::{
    discover_a2a_task_bindings, recover_a2a_task_history, A2aTaskObservationSelection,
    A2aTaskRecoveryAdmission, A2aTaskRecoveryError, A2aTaskRecoveryInput, A2aTaskRecoveryLimits,
    DiscoveredA2aTaskBinding, DiscoveredA2aTaskBindings, RecoveredA2aTaskHistory,
};

/// Association of an admitted route with this operation's checkpoint domain.
/// Physical retention of the signed route is an independent coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum A2aRouteAssociation {
    /// Prove absence and insert this domain's first route association leaf.
    Insert,
    /// Prove the exact route leaf already belongs to this domain.
    Retained,
}

/// Inputs to the atomic binding and revision-one history producer.
pub struct A2aTaskBindingCustodyInput<'a> {
    /// Existing anchored recovery of the exact historical dispatched observation.
    /// This is custody proof, not evidence that a network send occurred.
    pub operation_custody: &'a super::RecoveredOperationObservation,
    /// Fresh immutable binding reference.
    pub binding_ref: ResidentRecordRefV1,
    /// Candidate signed by the existing core owner after evidence admission.
    pub binding: A2aTaskBindingPreimageV1,
    /// Admitted historical handle, dispatched observation, route and signer.
    pub binding_context: A2aTaskBindingAdmissionContextV1<'a>,
    /// Fresh immutable initial history reference.
    pub observation_ref: ResidentRecordRefV1,
    /// Exact initial response/event and binding time.
    pub observation: A2aTaskObservationPreimageV1,
    /// Independently resolved observation schema and runtime signer.
    pub observation_signing: SignedResidentAdmissionContextV1<'a>,
    /// Absent means co-insert the route; Exact requires those already retained bytes.
    pub route_predecessor: OperationCustodyIndexPredecessor,
    /// Whether this operation domain already authenticates the route association.
    pub route_association: A2aRouteAssociation,
    /// Exact protocol evidence to insert in this atom.
    pub new_evidence: Vec<OperationA2aEvidence>,
    /// Exact evidence already authenticated under the predecessor checkpoint.
    pub retained_evidence: Vec<OperationA2aEvidence>,
    /// Aggregate raw and metadata bytes across both evidence collections.
    pub maximum_evidence_bytes: usize,
    /// Maximum canonical bytes of each signed record before custody preparation.
    pub maximum_record_bytes: usize,
}

/// Inputs to one remote-history successor, independently of Pale operation state.
pub struct A2aTaskObservationCustodyInput<'a> {
    /// Existing anchored recovery of the exact local observation context.
    /// A cancellation-requested context may be a historical member after progression.
    pub operation_custody: &'a super::RecoveredOperationObservation,
    /// Exact immutable reference of the context's selected Pale observation.
    /// This can differ from the historical dispatch retained by the binding.
    pub operation_observation_ref: &'a ResidentRecordRefV1,
    /// Fresh immutable successor reference.
    pub observation_ref: ResidentRecordRefV1,
    /// Candidate whose laws are owned by the existing core issuer.
    pub observation: A2aTaskObservationPreimageV1,
    /// Exact retained parents, predecessor, and independently resolved signer.
    pub context: A2aTaskObservationAdmissionContextV1<'a>,
    /// Raw evidence inserted atomically with this successor.
    pub new_evidence: Vec<OperationA2aEvidence>,
    /// Exact pre-existing evidence; it is never upserted.
    pub retained_evidence: Vec<OperationA2aEvidence>,
    /// Aggregate bytes including SSE metadata, across both collections.
    pub maximum_evidence_bytes: usize,
    /// Maximum canonical bytes of each projected signed record.
    pub maximum_record_bytes: usize,
}

/// Failure before the existing database/anchor custody phases.
#[derive(Debug, thiserror::Error)]
pub enum A2aTaskCustodyError {
    /// Physical time could not be established without violating retained history.
    #[error(transparent)]
    Time(#[from] crate::resident::OperationAttemptError),
    /// Core admission, evidence ownership or runtime-store resolution failed.
    #[error(transparent)]
    Admission(#[from] InvalidResidentRecordV1),
    /// Raw evidence is missing, substituted, aliased or exceeds its total bound.
    #[error("A2A task custody evidence does not close over this history atom")]
    Evidence,
    /// Signed-record projection, checkpoint construction or physical closure failed.
    #[error(transparent)]
    Projection(#[from] OperationObservationCustodyProjectionError),
    /// A bounded predecessor membership request could not be formed.
    #[error(transparent)]
    Membership(#[from] AccumulatorMembershipInputError),
}

/// Admitted history and exact bytes awaiting checkpoint construction.
/// No transport is called; signed fields do not establish physical custody.
#[must_use = "remote history still requires atomic custody and independent anchoring"]
pub struct PreparedA2aTaskAppend<'a> {
    maximum_record_bytes: usize,
    operation_custody: &'a super::RecoveredOperationObservation,
    store: AuthorityStoreCoordinate,
    kind: OperationCustodyTransitionKind,
    handle: &'a OperationHandleV1,
    binding_ref: ResidentRecordRefV1,
    binding: A2aTaskBindingV1,
    observation_ref: ResidentRecordRefV1,
    observation: A2aTaskObservationV1,
    predecessor: Option<(&'a ResidentRecordRefV1, &'a A2aTaskObservationV1)>,
    records: Vec<OperationCustodyRecord>,
    indexes: Vec<OperationCustodyIndexMutation>,
    dependencies: OperationA2aDependencies,
    get_task_preparation: Option<(OperationCustodyIndexKey, ControlRecord)>,
}

/// Issue initial history through the existing core owners and retain its exact atom.
///
/// `admit_evidence` must independently prove the authorized original request,
/// actual Task/context and exact initial response (including SSE framing metadata).
/// It must not infer that a send occurred from a Dispatched observation. The store
/// resolver must resolve the issued observation's runtime authority, permitted
/// schema, key and principal to its admitted deployment store incarnation.
/// Neither callback supplies current network or disclosure permission.
///
/// # Errors
/// Rejects bounds, evidence-reference substitution, contextual/signing failures,
/// unlike retained route bytes, or runtime-store resolution failure.
pub fn prepare_a2a_task_binding_custody<'a, Evidence, BindingSign, ObservationSign, Store>(
    input: A2aTaskBindingCustodyInput<'a>,
    admit_evidence: Evidence,
    sign_binding: BindingSign,
    sign_observation: ObservationSign,
    resolve_store: Store,
) -> Result<PreparedA2aTaskAppend<'a>, A2aTaskCustodyError>
where
    Evidence: FnOnce(
        &A2aTaskBindingPreimageV1,
        &A2aTaskObservationPreimageV1,
        &[OperationA2aEvidence],
        &[OperationA2aEvidence],
    ) -> Result<(), InvalidResidentRecordV1>,
    BindingSign: FnOnce(
        &[u8],
        &A2aTaskBindingPreimageV1,
        &A2aTaskBindingDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    ObservationSign: FnOnce(
        &[u8],
        &A2aTaskObservationPreimageV1,
        &A2aTaskObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    Store:
        FnOnce(&A2aTaskObservationV1) -> Result<AuthorityStoreCoordinate, InvalidResidentRecordV1>,
{
    if input.route_association == A2aRouteAssociation::Retained
        && matches!(
            input.route_predecessor,
            OperationCustodyIndexPredecessor::Absent
        )
    {
        return Err(A2aTaskCustodyError::Evidence);
    }
    check_evidence(
        &input.new_evidence,
        &input.retained_evidence,
        input.maximum_evidence_bytes,
        &[
            &input.binding.request_evidence,
            &input.binding.initial_task_evidence,
        ],
    )?;
    let source = source_references(&input.observation.source);
    if source.len() != 1 || source[0] != &input.binding.initial_task_evidence {
        return Err(A2aTaskCustodyError::Evidence);
    }
    let parents = input.binding_context;
    check_operation_custody(
        input.operation_custody,
        parents.operation_handle_ref,
        parents.operation_handle,
        parents.operation_observation_ref,
        parents.operation_observation,
    )?;
    let binding = A2aTaskBindingV1::issue_contextual(
        input.binding,
        &parents,
        |binding| {
            admit_evidence(
                binding,
                &input.observation,
                &input.new_evidence,
                &input.retained_evidence,
            )
        },
        sign_binding,
    )?;
    let context = A2aTaskObservationAdmissionContextV1 {
        signed: input.observation_signing,
        task_binding_ref: &input.binding_ref,
        task_binding: &binding,
        route_admission_ref: parents.route_admission_ref,
        route_admission: parents.route_admission,
        operation_handle_ref: parents.operation_handle_ref,
        operation_handle: parents.operation_handle,
        operation_observation: parents.operation_observation,
        predecessor: None,
    };
    // The evidence owner above admitted both exact preimages before either signature.
    let observation = A2aTaskObservationV1::issue_contextual(
        input.observation,
        &context,
        |_| Ok(()),
        |_, _, _, _| Err(reject("initial observation has no authority predecessor")),
        sign_observation,
    )?;
    let store = resolve_store(&observation)?;
    if &store != input.operation_custody.store() {
        return Err(A2aTaskCustodyError::Evidence);
    }
    let route = project(
        parents.route_admission_ref,
        parents
            .route_admission
            .canonical_a2a_route_admission_sha256()
            .as_str(),
        parents.route_admission,
        input.maximum_record_bytes,
    )?;
    let binding_record = project(
        &input.binding_ref,
        binding.canonical_a2a_task_binding_sha256().as_str(),
        &binding,
        input.maximum_record_bytes,
    )?;
    let observation_record = project(
        &input.observation_ref,
        observation.canonical_a2a_task_observation_sha256().as_str(),
        &observation,
        input.maximum_record_bytes,
    )?;
    let mut records = vec![
        OperationCustodyRecord::A2aTaskBinding(binding_record.clone()),
        OperationCustodyRecord::A2aTaskObservation(observation_record.clone()),
    ];
    let mut retained = local_parents(
        parents.operation_handle_ref,
        parents.operation_handle,
        parents.operation_observation_ref,
        parents.operation_observation,
        input.maximum_record_bytes,
    )?;
    let mut associated_route = None;
    match input.route_predecessor {
        OperationCustodyIndexPredecessor::Absent => {
            records.push(OperationCustodyRecord::A2aRouteAdmission(route))
        }
        OperationCustodyIndexPredecessor::Exact(expected) if expected == route => {
            if input.route_association == A2aRouteAssociation::Insert {
                associated_route = Some(route);
            } else {
                retained.push(route);
            }
        }
        OperationCustodyIndexPredecessor::Exact(_) => return Err(A2aTaskCustodyError::Evidence),
    }
    let key = binding_key(parents.operation_handle, binding.task_binding_id())?;
    let indexes = vec![
        OperationCustodyIndexMutation::A2aTaskBinding {
            operation_key: super::operation_observation_custody::operation_handle_key(
                parents.operation_handle.operation_handle_id(),
            )
            .map_err(OperationObservationCustodyProjectionError::from)?,
            key: key.clone(),
            expected: OperationCustodyIndexPredecessor::Absent,
            successor: binding_record.coordinate().clone(),
        },
        OperationCustodyIndexMutation::LatestA2aTaskObservation {
            key,
            expected: OperationCustodyIndexPredecessor::Absent,
            successor: observation_record.coordinate().clone(),
        },
    ];
    Ok(PreparedA2aTaskAppend {
        maximum_record_bytes: input.maximum_record_bytes,
        operation_custody: input.operation_custody,
        store,
        kind: OperationCustodyTransitionKind::BindA2aTask,
        handle: parents.operation_handle,
        binding_ref: input.binding_ref,
        binding,
        observation_ref: input.observation_ref,
        observation,
        predecessor: None,
        records,
        indexes,
        dependencies: OperationA2aDependencies {
            associated_route,
            new_evidence: input.new_evidence,
            retained_evidence: input.retained_evidence,
            retained_records: retained,
        },
        get_task_preparation: None,
    })
}

/// Issue an exact remote-history successor without changing the local operation.
///
/// The evidence callback resolves exact request/response or SSE meaning. The
/// core owner retains predecessor, retention, cancellation-source and authority-
/// rotation laws. `resolve_store` has the same authority obligation as initial
/// binding preparation. No physical bytes or current access grant are inferred.
///
/// # Errors
/// Rejects missing predecessor, unlike evidence, limits, contextual admission,
/// signing and store-resolution failures before any custody mutation.
pub fn prepare_a2a_task_observation_custody<'a, Evidence, Rotation, Sign, Store>(
    input: A2aTaskObservationCustodyInput<'a>,
    admit_evidence: Evidence,
    admit_rotation: Rotation,
    sign: Sign,
    resolve_store: Store,
) -> Result<PreparedA2aTaskAppend<'a>, A2aTaskCustodyError>
where
    Evidence: FnOnce(
        &A2aTaskObservationPreimageV1,
        &[OperationA2aEvidence],
        &[OperationA2aEvidence],
    ) -> Result<(), InvalidResidentRecordV1>,
    Rotation: FnOnce(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
    Sign: FnOnce(
        &[u8],
        &A2aTaskObservationPreimageV1,
        &A2aTaskObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    Store:
        FnOnce(&A2aTaskObservationV1) -> Result<AuthorityStoreCoordinate, InvalidResidentRecordV1>,
{
    let context = input.context;
    check_operation_custody(
        input.operation_custody,
        context.operation_handle_ref,
        context.operation_handle,
        input.operation_observation_ref,
        context.operation_observation,
    )?;
    let predecessor = context.predecessor.ok_or(A2aTaskCustodyError::Evidence)?;
    check_evidence(
        &input.new_evidence,
        &input.retained_evidence,
        input.maximum_evidence_bytes,
        &source_references(&input.observation.source),
    )?;
    let observation = A2aTaskObservationV1::issue_contextual(
        input.observation,
        &context,
        |value| admit_evidence(value, &input.new_evidence, &input.retained_evidence),
        admit_rotation,
        sign,
    )?;
    let store = resolve_store(&observation)?;
    if &store != input.operation_custody.store() {
        return Err(A2aTaskCustodyError::Evidence);
    }
    let binding = project(
        context.task_binding_ref,
        context
            .task_binding
            .canonical_a2a_task_binding_sha256()
            .as_str(),
        context.task_binding,
        input.maximum_record_bytes,
    )?;
    let record = project(
        &input.observation_ref,
        observation.canonical_a2a_task_observation_sha256().as_str(),
        &observation,
        input.maximum_record_bytes,
    )?;
    let prior = project(
        predecessor.0,
        predecessor
            .1
            .canonical_a2a_task_observation_sha256()
            .as_str(),
        predecessor.1,
        input.maximum_record_bytes,
    )?;
    // The selected operation observation is retained as exact context, even when
    // it is terminal. Remote history does not advance or reopen that lineage.
    let mut retained = local_parents(
        context.operation_handle_ref,
        context.operation_handle,
        input.operation_observation_ref,
        context.operation_observation,
        input.maximum_record_bytes,
    )?;
    retained.push(binding.clone());
    retained.push(project(
        context.route_admission_ref,
        context
            .route_admission
            .canonical_a2a_route_admission_sha256()
            .as_str(),
        context.route_admission,
        input.maximum_record_bytes,
    )?);
    let key = binding_key(
        context.operation_handle,
        context.task_binding.task_binding_id(),
    )?;
    let indexes = vec![
        OperationCustodyIndexMutation::A2aTaskBinding {
            operation_key: super::operation_observation_custody::operation_handle_key(
                context.operation_handle.operation_handle_id(),
            )
            .map_err(OperationObservationCustodyProjectionError::from)?,
            key: key.clone(),
            expected: OperationCustodyIndexPredecessor::Exact(binding.clone()),
            successor: binding.coordinate().clone(),
        },
        OperationCustodyIndexMutation::LatestA2aTaskObservation {
            key,
            expected: OperationCustodyIndexPredecessor::Exact(prior),
            successor: record.coordinate().clone(),
        },
    ];
    Ok(PreparedA2aTaskAppend {
        store,
        kind: OperationCustodyTransitionKind::ObserveA2aTask,
        maximum_record_bytes: input.maximum_record_bytes,
        operation_custody: input.operation_custody,
        handle: context.operation_handle,
        binding_ref: context.task_binding_ref.clone(),
        binding: context.task_binding.clone(),
        observation_ref: input.observation_ref,
        observation,
        predecessor: Some(predecessor),
        records: vec![OperationCustodyRecord::A2aTaskObservation(record)],
        indexes,
        dependencies: OperationA2aDependencies {
            associated_route: None,
            new_evidence: input.new_evidence,
            retained_evidence: input.retained_evidence,
            retained_records: retained,
        },
        get_task_preparation: None,
    })
}

impl PreparedA2aTaskAppend<'_> {
    /// Bind one successful `GetTask` observation to its retained preparation.
    #[allow(
        dead_code,
        reason = "the process-host GetTask adoption is a separate lane"
    )]
    pub(in crate::resident) fn after_get_task_preparation(
        mut self,
        preparation: ControlRecord,
    ) -> Result<Self, A2aTaskCustodyError> {
        if self.kind != OperationCustodyTransitionKind::ObserveA2aTask
            || !matches!(
                self.observation.source(),
                A2aTaskObservationSourceV1::GetTask { .. }
            )
            || preparation.schema_id().as_str()
                != PRIVATE_OPERATION_A2A_GET_TASK_PREPARATION_V1_SCHEMA
            || self.get_task_preparation.is_some()
        {
            return Err(A2aTaskCustodyError::Evidence);
        }
        self.get_task_preparation = Some((
            binding_key(self.handle, self.binding.task_binding_id())?,
            preparation,
        ));
        Ok(self)
    }
    /// Borrow the contextually admitted binding, not proof of durable custody.
    #[must_use]
    pub fn binding(&self) -> (&ResidentRecordRefV1, &A2aTaskBindingV1) {
        (&self.binding_ref, &self.binding)
    }
    /// Borrow the contextually admitted remote observation, not a local result.
    #[must_use]
    pub fn observation(&self) -> (&ResidentRecordRefV1, &A2aTaskObservationV1) {
        (&self.observation_ref, &self.observation)
    }
}

fn source_references(source: &A2aTaskObservationSourceV1) -> Vec<&ImmutableRecordReferenceV1> {
    match source {
        A2aTaskObservationSourceV1::InitialSendMessageResponse { evidence }
        | A2aTaskObservationSourceV1::InitialStreamingEvent { evidence, .. }
        | A2aTaskObservationSourceV1::StreamingEvent { evidence, .. } => vec![evidence],
        A2aTaskObservationSourceV1::GetTask { request, response }
        | A2aTaskObservationSourceV1::CancelTaskReadBack { request, response } => {
            vec![request, response]
        }
    }
}

fn check_evidence(
    new: &[OperationA2aEvidence],
    retained: &[OperationA2aEvidence],
    maximum: usize,
    expected: &[&ImmutableRecordReferenceV1],
) -> Result<(), A2aTaskCustodyError> {
    if new.len().checked_add(retained.len()) != Some(expected.len())
        || expected.is_empty()
        || expected.len() > 2
    {
        return Err(A2aTaskCustodyError::Evidence);
    }
    let mut total = 0usize;
    let mut references = std::collections::BTreeSet::new();
    for item in new.iter().chain(retained) {
        total = total
            .checked_add(item.byte_len())
            .filter(|n| *n <= maximum)
            .ok_or(A2aTaskCustodyError::Evidence)?;
        if !references.insert(item.coordinate().record_ref())
            || !expected.iter().any(|r| {
                &r.record_ref == item.coordinate().record_ref()
                    && r.record_sha256.as_str() == item.coordinate().record_sha256().as_str()
            })
        {
            return Err(A2aTaskCustodyError::Evidence);
        }
    }
    Ok(())
}

fn local_parents(
    handle_ref: &ResidentRecordRefV1,
    handle: &OperationHandleV1,
    observation_ref: &ResidentRecordRefV1,
    observation: &OperationObservationV1,
    maximum: usize,
) -> Result<Vec<ControlRecord>, A2aTaskCustodyError> {
    Ok(vec![
        project(
            handle_ref,
            handle.canonical_operation_handle_sha256().as_str(),
            handle,
            maximum,
        )?,
        project(
            observation_ref,
            observation
                .canonical_operation_observation_sha256()
                .as_str(),
            observation,
            maximum,
        )?,
    ])
}

fn project<T: serde::Serialize + record_projection::ResidentRecordSchema>(
    reference: &ResidentRecordRefV1,
    digest: &str,
    value: &T,
    maximum: usize,
) -> Result<ControlRecord, A2aTaskCustodyError> {
    let coordinate = record_projection::coordinate(reference, digest)
        .map_err(OperationObservationCustodyProjectionError::from)?;
    let record = record_projection::record(coordinate, value)
        .map_err(OperationObservationCustodyProjectionError::from)?;
    if record.canonical_record_jcs().as_slice().len() > maximum {
        return Err(A2aTaskCustodyError::Evidence);
    }
    Ok(record)
}

fn binding_key(
    handle: &OperationHandleV1,
    binding_id: &A2aTaskBindingIdV1,
) -> Result<OperationCustodyIndexKey, A2aTaskCustodyError> {
    let bytes = canon::to_canonical_string(&(
        "PALE_A2A_TASK_BINDING_INDEX_V1",
        handle.operation_handle_id(),
        binding_id,
    ))
    .map_err(OperationObservationCustodyProjectionError::from)?;
    OperationCustodyIndexKey::parse(ArtifactDigest::hash_bytes(bytes.as_bytes()).as_str())
        .map_err(|e| OperationObservationCustodyProjectionError::from(e).into())
}

fn check_operation_custody(
    proof: &super::RecoveredOperationObservation,
    handle_ref: &ResidentRecordRefV1,
    handle: &OperationHandleV1,
    observation_ref: &ResidentRecordRefV1,
    observation: &OperationObservationV1,
) -> Result<(), A2aTaskCustodyError> {
    if proof.handle.record_ref() != handle_ref
        || proof.handle.record_sha256().as_str()
            != handle.canonical_operation_handle_sha256().as_str()
        || proof.selected().0 != observation_ref
        || proof.selected().1 != observation
    {
        return Err(A2aTaskCustodyError::Evidence);
    }
    Ok(())
}

fn reject(invariant: &'static str) -> InvalidResidentRecordV1 {
    InvalidResidentRecordV1::ContextualAdmission {
        record: "A2A task custody",
        invariant,
    }
}
