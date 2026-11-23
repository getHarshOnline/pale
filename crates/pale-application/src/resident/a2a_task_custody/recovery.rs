//! Bounded known-binding recovery; no task enumeration or current access grant.

use super::*;
use crate::resident::{
    checkpoint_profile::{a2a_evidence_leaf, a2a_record_leaf, OperationLeafProjectionError},
    operation_custody::read_index,
    operation_observation_custody::recover_with_checkpoint,
    IssuedLocalOperation, LocalOperationRecoveryError, OperationCheckpointAncestryLimits,
    OperationObservationReadmissionError, OperationObservationRecoveryAdmission,
    OperationObservationRecoveryInput, OperationObservationRecoveryLimits,
    OperationObservationSelection, OperationRecoveryError, RecoveredOperationObservation,
};
use std::collections::BTreeMap;

mod read;
use read::{Evidence, History};
mod discovery;
pub use discovery::{
    discover_a2a_task_bindings, DiscoveredA2aTaskBinding, DiscoveredA2aTaskBindings,
};

/// Independent finite ceilings for the remote history and its exact protocol bytes.
#[derive(Debug, Clone, Copy)]
pub struct A2aTaskRecoveryLimits {
    maximum_revisions: usize,
    maximum_record_bytes: usize,
    maximum_history_bytes: usize,
    maximum_evidence_records: usize,
    maximum_evidence_bytes: usize,
}

/// Which remote observation to return after admitting the complete selected history.
#[derive(Debug, Clone, Copy)]
pub enum A2aTaskObservationSelection<'a> {
    /// Return the remote head selected by the store at the start of this read.
    SelectedHead,
    /// Return this exact member only if it belongs to the selected head's history.
    HistoryMember {
        /// Existing remote-observation reference.
        reference: &'a ResidentRecordRefV1,
        /// Canonical observation digest paired with the reference.
        digest: &'a A2aTaskObservationDigest,
    },
}

impl A2aTaskRecoveryLimits {
    /// Bounds include binding, route, initial/head index bodies, and every remote
    /// predecessor body; repeated physical reads are charged again. Evidence is
    /// deduplicated by exact coordinate and includes all SSE metadata bytes.
    /// Local history and checkpoint ancestry have separate explicit ceilings.
    /// # Errors
    /// Rejects zero ceilings; there is no implicit quota or unbounded fallback.
    pub fn new(
        maximum_revisions: usize,
        maximum_record_bytes: usize,
        maximum_history_bytes: usize,
        maximum_evidence_records: usize,
        maximum_evidence_bytes: usize,
    ) -> Result<Self, A2aTaskRecoveryError> {
        if [
            maximum_revisions,
            maximum_record_bytes,
            maximum_history_bytes,
            maximum_evidence_records,
            maximum_evidence_bytes,
        ]
        .contains(&0)
        {
            return Err(A2aTaskRecoveryError::Bounds);
        }
        Ok(Self {
            maximum_revisions,
            maximum_record_bytes,
            maximum_history_bytes,
            maximum_evidence_records,
            maximum_evidence_bytes,
        })
    }
}

/// Existing local locator plus a known binding identity, not permission to inspect it.
pub struct A2aTaskRecoveryInput<'a> {
    /// Exact admitted store incarnation.
    pub store: &'a AuthorityStoreCoordinate,
    /// Custody-issued or protected-recovered local operation.
    pub operation: &'a IssuedLocalOperation,
    /// Caller-known identity; no one-task-per-attempt assumption or enumeration.
    pub task_binding_id: &'a A2aTaskBindingIdV1,
    /// Requested remote head or exact historical member.
    pub observation: A2aTaskObservationSelection<'a>,
    /// Owner-resolved authority for the current signed checkpoint.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// One local history walk, shared by dispatch and cancellation witnesses.
    pub local_limits: OperationObservationRecoveryLimits,
    /// Applied to each of two fixed ancestry checks, never once per remote revision.
    pub checkpoint_limits: OperationCheckpointAncestryLimits,
    /// Native membership proof ceilings, used for local and remote selected leaves.
    pub membership_limits: AccumulatorMembershipLimits,
    /// Remote canonical-body and deduplicated evidence ceilings.
    pub limits: A2aTaskRecoveryLimits,
}

/// Check historical record meaning against its selected protocol evidence.
type EvidenceAdmission<'a, Preimage> =
    dyn FnMut(&Preimage, &[&OperationA2aEvidence]) -> Result<(), InvalidResidentRecordV1> + 'a;

/// Verify an existing record owner's signature domain, preimage and digest.
type SignatureVerification<'a, Preimage, Digest> = dyn FnMut(&[u8], &Preimage, &Digest, &CanonicalSignatureV1) -> Result<(), InvalidResidentRecordV1>
    + 'a;

/// Admit historical authority succession without granting current permission.
type AuthorityRotationAdmission<'a> = dyn FnMut(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;

/// Historical owner callbacks; none confers current transport or disclosure access.
/// Every callback must separately bound its external authority/evidence resolution.
pub struct A2aTaskRecoveryAdmission<'a> {
    /// Invoke the existing route contextual owner with historical signer, exact
    /// provider/route, Agent Card, security and HTTPS evidence. Task custody does
    /// not promise local storage of those separate route-evidence bodies.
    pub admit_route: &'a mut dyn FnMut(
        &ResidentRecordRefV1,
        A2aRouteAdmissionWireV1,
    ) -> Result<A2aRouteAdmissionV1, InvalidResidentRecordV1>,
    /// Resolve binding signer authority at issuance, not from trusted wire fields.
    pub resolve_binding_signer:
        &'a mut dyn FnMut(
            &ResidentRecordRefV1,
            &A2aTaskBindingWireV1,
        ) -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1>,
    /// Prove an actual authorized original request and initial Task/context.
    pub admit_binding_evidence: &'a mut EvidenceAdmission<'a, A2aTaskBindingPreimageV1>,
    /// Verify the exact existing core binding signature domain and preimage.
    pub verify_binding_signature:
        &'a mut SignatureVerification<'a, A2aTaskBindingPreimageV1, A2aTaskBindingDigest>,
    /// Resolve each observation signer at its historical issuance.
    pub resolve_observation_signer:
        &'a mut dyn FnMut(
            &ResidentRecordRefV1,
            &A2aTaskObservationWireV1,
        ) -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1>,
    /// Admit exact request/response or SSE meaning, never remote-to-local success.
    pub admit_observation_evidence: &'a mut EvidenceAdmission<'a, A2aTaskObservationPreimageV1>,
    /// Prove changed runtime authority in the same admitted deployment lineage.
    pub admit_rotation: &'a mut AuthorityRotationAdmission<'a>,
    /// Verify each exact contextual remote observation signature.
    pub verify_observation_signature:
        &'a mut SignatureVerification<'a, A2aTaskObservationPreimageV1, A2aTaskObservationDigest>,
}

/// Fully readmitted selected remote head under one anchored local checkpoint.
/// This is neither a latest-at-return promise nor permission to inspect or send.
pub struct RecoveredA2aTaskHistory {
    route: (ResidentRecordRefV1, A2aRouteAdmissionV1),
    binding: (ResidentRecordRefV1, A2aTaskBindingV1),
    observation: (ResidentRecordRefV1, A2aTaskObservationV1),
    selected_observation: (ResidentRecordRefV1, A2aTaskObservationV1),
    dispatch: (ResidentRecordRefV1, OperationObservationV1),
    local: RecoveredOperationObservation,
}

impl RecoveredA2aTaskHistory {
    /// Exact historically admitted route; freshness does not authorize a new send.
    #[must_use]
    pub fn route(&self) -> (&ResidentRecordRefV1, &A2aRouteAdmissionV1) {
        (&self.route.0, &self.route.1)
    }
    /// Exact known binding associated with the retained local operation.
    #[must_use]
    pub fn binding(&self) -> (&ResidentRecordRefV1, &A2aTaskBindingV1) {
        (&self.binding.0, &self.binding.1)
    }
    /// Selected admitted remote history head, not a Pale result.
    #[must_use]
    pub fn observation(&self) -> (&ResidentRecordRefV1, &A2aTaskObservationV1) {
        (&self.observation.0, &self.observation.1)
    }
    /// Exact requested member, or the selected head when no historical member was requested.
    #[must_use]
    pub fn selected_observation(&self) -> (&ResidentRecordRefV1, &A2aTaskObservationV1) {
        (&self.selected_observation.0, &self.selected_observation.1)
    }
    /// Exact original dispatched observation admitted during the shared local walk.
    /// This is historical association evidence, not current dispatch permission.
    #[must_use]
    pub fn dispatch(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.dispatch.0, &self.dispatch.1)
    }
    /// Existing anchored local head proof from the shared local history walk.
    #[must_use]
    pub fn local_observation(&self) -> &RecoveredOperationObservation {
        &self.local
    }
}

/// Rejected recovery stage; failures never authorize retry, execution or disclosure.
#[derive(Debug, thiserror::Error)]
pub enum A2aTaskRecoveryError {
    /// A finite record, byte, revision or evidence ceiling was exhausted.
    #[error("A2A task recovery bounds exceeded")]
    Bounds,
    /// Required indexed or immutable evidence is absent.
    #[error("A2A task recovery evidence is missing")]
    Missing,
    /// Exact selected coordinates, schema, bytes or contextual witness differ.
    #[error("A2A task recovery association differs")]
    Association,
    /// Claimed remote predecessor revisions are not contiguous.
    #[error("A2A task recovery history is not contiguous")]
    Lineage,
    /// Existing historical record owner rejected authority, signature or evidence.
    #[error(transparent)]
    Admission(#[from] InvalidResidentRecordV1),
    /// Existing local history admission and its anchored proof failed.
    #[error(transparent)]
    Local(#[from] OperationObservationReadmissionError),
    /// Exact current checkpoint ancestry or membership failed.
    #[error(transparent)]
    Checkpoint(#[from] LocalOperationRecoveryError),
    /// Physical immutable record/evidence lookup failed.
    #[error(transparent)]
    Store(#[from] ControlStoreError),
    /// Typed index resolution failed.
    #[error(transparent)]
    Lookup(#[from] OperationRecoveryError),
    /// Existing projection or key construction failed.
    #[error(transparent)]
    Projection(#[from] A2aTaskCustodyError),
    /// Contract schema identity is unavailable.
    #[error(transparent)]
    Schema(#[from] pale_contracts::SchemaError),
    /// Strict parsing or canonical encoding failed.
    #[error(transparent)]
    Canonical(#[from] canon::CanonError),
    /// Typed wire decoding failed.
    #[error(transparent)]
    Decode(#[from] serde_json::Error),
}

/// Recover a known remote binding without constructing a transport or a second service.
///
/// One bounded local walk admits all revisions, collecting only requested historical
/// witnesses. One remote walk admits every predecessor; exact evidence is read once
/// per coordinate. The final membership proof rechecks the same current checkpoint
/// and independent anchor. No internal retry, write, signing, renewal or network call
/// occurs. The caller must separately establish current inspection permission.
/// # Errors
/// Rejects missing/foreign/corrupt history, exhausted bounds, owner admission failure,
/// missing historical cancellation context, or changed checkpoint/anchor evidence.
pub fn recover_a2a_task_history<
    Signer,
    State,
    Rotation,
    Signature,
    Checkpoint,
    CheckpointSignature,
>(
    input: A2aTaskRecoveryInput<'_>,
    port: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    local_admissions: OperationObservationRecoveryAdmission<
        Signer,
        State,
        Rotation,
        Signature,
        Checkpoint,
    >,
    verify_checkpoint_signature: CheckpointSignature,
    remote: A2aTaskRecoveryAdmission<'_>,
) -> Result<RecoveredA2aTaskHistory, A2aTaskRecoveryError>
where
    Checkpoint: FnMut(
        &ResidentRecordRefV1,
        ControlStateCheckpointWireV1,
    ) -> Result<ControlStateCheckpointV1, InvalidResidentRecordV1>,
    Signer: FnMut(
        &ResidentRecordRefV1,
        &OperationObservationWireV1,
    ) -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1>,
    State: FnMut(&OperationObservationStateV1) -> Result<(), InvalidResidentRecordV1>,
    Rotation: FnMut(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
    Signature: FnMut(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
    CheckpointSignature: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    let history = History::read(&input, port)?;
    let evidence = Evidence::read(&history, &input, port)?;
    let mut cancellations = BTreeMap::new();
    for (_, wire) in &history.revisions {
        if let A2aTaskObservationSourceV1::CancelTaskReadBack { request, .. } = wire.source() {
            cancellations
                .entry(read::coordinate(
                    &request.record_ref,
                    request.record_sha256.as_str(),
                )?)
                .or_insert(None::<(ResidentRecordRefV1, OperationObservationV1)>);
        }
    }
    let mut dispatch = None;
    let mut collector_error = None;
    let OperationObservationRecoveryAdmission {
        mut admit_checkpoint,
        resolve_signer,
        admit_state,
        admit_rotation,
        verify_signature,
    } = local_admissions;
    let local = recover_with_checkpoint(
        OperationObservationRecoveryInput {
            store: input.store,
            operation: input.operation,
            observation: OperationObservationSelection::SelectedHead,
            checkpoint_authority: input.checkpoint_authority,
            limits: input.local_limits,
            checkpoint_limits: input.checkpoint_limits,
            membership_limits: input.membership_limits,
        },
        port,
        anchor,
        OperationObservationRecoveryAdmission {
            admit_checkpoint: &mut admit_checkpoint,
            resolve_signer,
            admit_state,
            admit_rotation,
            verify_signature,
        },
        verify_checkpoint_signature,
        &mut |reference, observation| {
            if reference == history.binding.1.operation_observation_ref()
                && observation.canonical_operation_observation_sha256()
                    == history.binding.1.operation_observation_sha256()
            {
                dispatch = Some((reference.clone(), observation.clone()));
            }
            if let OperationObservationStateV1::CancellationRequested { request } =
                observation.state()
            {
                match read::coordinate(&request.record_ref, request.record_sha256.as_str()) {
                    Ok(key) => {
                        if let Some(witness) = cancellations.get_mut(&key) {
                            // Forward admission visits the earliest matching witness first.
                            // Repeated cancellation evidence must not replace it with a later time.
                            if witness.is_none() {
                                *witness = Some((reference.clone(), observation.clone()));
                            }
                        }
                    }
                    Err(error) => collector_error = Some(error),
                }
            }
        },
    )?;
    // No collected witness or remote owner callback is used before the complete
    // local history, exact checkpoint and independent-anchor proof succeeds.
    if let Some(error) = collector_error {
        return Err(error);
    }
    let dispatch = dispatch.ok_or(A2aTaskRecoveryError::Missing)?;
    if cancellations.values().any(Option::is_none) {
        return Err(A2aTaskRecoveryError::Missing);
    }
    let (handle_ref, handle) = input.operation.handle();
    let History {
        binding: (binding_record, binding_wire),
        route: (route_record, route_wire),
        revisions,
        checkpoint,
        selected_observation,
    } = history;
    let route = (remote.admit_route)(route_record.coordinate().record_ref(), route_wire)?;
    read::check_admitted(
        &route_record,
        &route,
        route.canonical_a2a_route_admission_sha256().as_str(),
    )?;
    let binding_schema = pale_contracts::record_schema_identity(A2A_TASK_BINDING_V1_SCHEMA_ID)?;
    let binding_signer =
        (remote.resolve_binding_signer)(binding_record.coordinate().record_ref(), &binding_wire)?;
    let binding_evidence = evidence.select(&[
        binding_wire.request_evidence(),
        binding_wire.initial_task_evidence(),
    ])?;
    let binding = A2aTaskBindingV1::admit_contextual_wire(
        binding_wire,
        &A2aTaskBindingAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &binding_schema.schema_sha256,
                signing_authority: &binding_signer,
            },
            operation_handle_ref: handle_ref,
            operation_handle: handle,
            operation_observation_ref: &dispatch.0,
            operation_observation: &dispatch.1,
            route_admission_ref: route_record.coordinate().record_ref(),
            route_admission: &route,
        },
        |preimage| (remote.admit_binding_evidence)(preimage, &binding_evidence),
        remote.verify_binding_signature,
    )?;
    read::check_admitted(
        &binding_record,
        &binding,
        binding.canonical_a2a_task_binding_sha256().as_str(),
    )?;
    let schema = pale_contracts::record_schema_identity(A2A_TASK_OBSERVATION_V1_SCHEMA_ID)?;
    let mut admitted: Option<(ResidentRecordRefV1, A2aTaskObservationV1)> = None;
    let mut selected_admitted: Option<(ResidentRecordRefV1, A2aTaskObservationV1)> = None;
    for (record, wire) in revisions.into_iter().rev() {
        let is_selected = record.coordinate() == &selected_observation;
        let signer = (remote.resolve_observation_signer)(record.coordinate().record_ref(), &wire)?;
        let selected_evidence = evidence.select(&source_references(wire.source()))?;
        let local_context = match wire.source() {
            A2aTaskObservationSourceV1::CancelTaskReadBack { request, .. } => {
                let key = read::coordinate(&request.record_ref, request.record_sha256.as_str())?;
                &cancellations
                    .get(&key)
                    .and_then(Option::as_ref)
                    .ok_or(A2aTaskRecoveryError::Missing)?
                    .1
            }
            _ => local.selected().1,
        };
        let observation = A2aTaskObservationV1::admit_contextual_wire(
            wire,
            &A2aTaskObservationAdmissionContextV1 {
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &schema.schema_sha256,
                    signing_authority: &signer,
                },
                task_binding_ref: binding_record.coordinate().record_ref(),
                task_binding: &binding,
                route_admission_ref: route_record.coordinate().record_ref(),
                route_admission: &route,
                operation_handle_ref: handle_ref,
                operation_handle: handle,
                operation_observation: local_context,
                predecessor: admitted
                    .as_ref()
                    .map(|(reference, value)| (reference, value)),
            },
            |preimage| (remote.admit_observation_evidence)(preimage, &selected_evidence),
            &mut *remote.admit_rotation,
            &mut *remote.verify_observation_signature,
        )?;
        read::check_admitted(
            &record,
            &observation,
            observation.canonical_a2a_task_observation_sha256().as_str(),
        )?;
        let admitted_observation = (record.coordinate().record_ref().clone(), observation);
        if is_selected {
            selected_admitted = Some(admitted_observation.clone());
        }
        admitted = Some(admitted_observation);
    }
    let observation = admitted.ok_or(A2aTaskRecoveryError::Missing)?;
    let selected_observation = selected_admitted.ok_or(A2aTaskRecoveryError::Missing)?;
    let key = binding_key(handle, input.task_binding_id)?;
    let mut leaves = Vec::new();
    for (class, logical, coord) in [
        (
            "a2a-route-admission",
            route_record.coordinate().record_ref().as_str(),
            route_record.coordinate().clone(),
        ),
        (
            "a2a-task-binding",
            key.as_str(),
            binding_record.coordinate().clone(),
        ),
        (
            "a2a-task-observation",
            key.as_str(),
            read::coordinate(
                &observation.0,
                observation
                    .1
                    .canonical_a2a_task_observation_sha256()
                    .as_str(),
            )?,
        ),
    ] {
        let leaf = a2a_record_leaf(input.store, &local.current.record, class, logical, &coord)
            .map_err(leaf_error)?;
        leaves.push(
            AccumulatorMembershipLeaf::new(leaf.hashed_key, leaf.value)
                .map_err(A2aTaskCustodyError::from)?,
        );
    }
    for row in evidence.0.values() {
        let leaf =
            a2a_evidence_leaf(input.store, &local.current.record, row).map_err(leaf_error)?;
        leaves.push(
            AccumulatorMembershipLeaf::new(leaf.hashed_key, leaf.value)
                .map_err(A2aTaskCustodyError::from)?,
        );
    }
    leaves.sort_by(|a, b| a.key().cmp(b.key()));
    let _ = local.current.verify_ancestry(
        &checkpoint,
        input.limits.maximum_record_bytes,
        input.checkpoint_limits,
        port,
        &mut admit_checkpoint,
    )?;
    local.current.verify_membership(
        &input.operation.checkpoint_selection(),
        leaves,
        input.membership_limits,
        port,
        anchor,
    )?;
    Ok(RecoveredA2aTaskHistory {
        route: (route_record.coordinate().record_ref().clone(), route),
        binding: (binding_record.coordinate().record_ref().clone(), binding),
        observation,
        selected_observation,
        dispatch,
        local,
    })
}

fn leaf_error(error: OperationLeafProjectionError) -> A2aTaskRecoveryError {
    let error: OperationObservationCustodyProjectionError = match error {
        OperationLeafProjectionError::Store => {
            OperationObservationCustodyProjectionError::StoreScope
        }
        OperationLeafProjectionError::Identity(error) => error.into(),
        OperationLeafProjectionError::Canonicalize(error) => error.into(),
    };
    A2aTaskCustodyError::from(error).into()
}
