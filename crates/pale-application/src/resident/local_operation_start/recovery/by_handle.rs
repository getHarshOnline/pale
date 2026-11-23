//! Recover the original request selected by a saved operation locator.

use super::{
    check_record, recover_with_parent, IssuedLocalOperation, LocalChildOperationRecoveryContext,
    LocalOperationRecoveryAdmission, LocalOperationRecoveryContext, LocalOperationRecoveryError,
};
use crate::resident::{
    operation_custody::{read_index, OperationRecoveryError},
    operation_observation_custody::{operation_handle_key, OperationObservationRecoveryAdmission},
};
use pale_core::{canon, records::*};
use pale_ports::{
    AntiRollbackAnchorPort, ControlRecordCoordinate, OperationCustodyIndexQuery,
    OperationCustodyPort, SessionLeaseCustodyPort,
};

/// Exact owned request and operation recovered from one saved locator.
///
/// Only successful protected recovery constructs this pair, after the original
/// request's contextual admission, current checkpoint membership, independent
/// anchor and final immutable-record associations have all been checked.
/// Retaining the request does not recover current ambient session admission, current
/// inspection or execution permission, private attempt selection, a prepared
/// call, compiled/provider authority or an authenticated SDK attachment.
pub struct RecoveredLocalOperation {
    operation: IssuedLocalOperation,
    request: CommonRequestEnvelopeV1,
    store: pale_ports::AuthorityStoreCoordinate,
    initial_observation: Option<Box<OperationObservationRecoveryAdmissionEvidence>>,
    recovered_parent: Option<Box<RecoveredLocalOperation>>,
}

/// Revision-one observation evidence retained by protected handle recovery.
///
/// This private wrapper prevents an independently read observation from being
/// substituted for the exact start evidence admitted while the handle chain was
/// recovered. It grants no current inspection or continuation authority.
struct OperationObservationRecoveryAdmissionEvidence {
    observation: crate::resident::RecoveredOperationObservation,
}

/// Bounded protected recovery selected by one saved operation-handle coordinate.
pub struct LocalOperationHandleRecoveryInput<'a> {
    /// Existing store, historical authority, checkpoint profile, and read limits.
    pub context: LocalOperationRecoveryContext<'a>,
    /// Exact saved handle reference and digest selected by the caller.
    pub expected_handle: &'a ControlRecordCoordinate,
    /// Maximum number of parent edges admitted for this recovery.
    pub maximum_parent_depth: usize,
}

/// One saved child handle recovered against an exact protected parent.
pub(in crate::resident) struct LocalChildOperationHandleRecoveryInput<'a> {
    /// Existing child-recovery context, including the protected parent and lease.
    pub(in crate::resident) child: LocalChildOperationRecoveryContext<'a>,
    /// Exact saved child-handle coordinate selected by the process owner.
    pub(in crate::resident) expected_handle: &'a ControlRecordCoordinate,
}

impl RecoveredLocalOperation {
    /// Borrow the custody-proven operation; this does not renew its authority.
    #[must_use]
    pub const fn operation(&self) -> &IssuedLocalOperation {
        &self.operation
    }

    /// Borrow the original contextually admitted request from that same recovery.
    /// The request's body is historical input, not a new dispatch or read grant.
    #[must_use]
    pub const fn request(&self) -> &CommonRequestEnvelopeV1 {
        &self.request
    }

    /// Borrow the exact admitted store incarnation used for protected recovery.
    #[must_use]
    pub const fn store(&self) -> &pale_ports::AuthorityStoreCoordinate {
        &self.store
    }

    pub(in crate::resident) fn into_operation(self) -> IssuedLocalOperation {
        self.operation
    }

    /// Borrow the revision-one observation proved during this handle recovery.
    pub(in crate::resident) fn initial_observation(
        &self,
    ) -> Option<&crate::resident::RecoveredOperationObservation> {
        self.initial_observation
            .as_ref()
            .map(|evidence| &evidence.observation)
    }

    /// Borrow the immediate parent retained by bounded root-to-child recovery.
    pub(in crate::resident) fn recovered_parent(&self) -> Option<&RecoveredLocalOperation> {
        self.recovered_parent.as_deref()
    }
}

#[cfg(test)]
#[path = "../../../../tests/support/resident/recovered_local_operation.rs"]
mod recovered_local_operation_fixture;

/// Recover an operation and its bounded parent chain from one saved handle.
///
/// The caller must already have permission to inspect this store. The handle's
/// decoded identity selects a private lookup; it is not admitted authority.
/// `admit_request` must resolve each original caller's authority at issuance and
/// use [`CommonRequestEnvelopeV1::admit_contextual_wire`]. It must not renew an
/// expired request. Each handle and request is read and admitted at most once,
/// up to `maximum_parent_depth + 1`. Each callback must separately bound its
/// internal verification work.
///
/// Recovery then uses the same historical lease, handle, current membership and
/// independent-anchor checks as [`super::recover_local_operation`]. Every child
/// is admitted against its exact recovered parent handle and lease. Both request
/// indexes must name the same record, and every admitted handle must match its
/// saved locator and initially read bytes. The returned pair owns the selected
/// operation and request. Success grants no new inspection, execution or
/// continuation permission and proves no historical co-commit.
///
/// # Errors
/// Rejects missing, oversized, misdirected or substituted evidence, historical
/// admission failures, changed membership and anchor races. No writes or retries
/// occur, and no partially recovered operation is returned.
pub fn recover_local_operation_from_handle<
    Request,
    Authority,
    HandleSignature,
    CheckpointSignature,
    ObservationSigner,
    ObservationState,
    AuthorityRotation,
    ObservationSignature,
    HistoricalCheckpoint,
>(
    input: LocalOperationHandleRecoveryInput<'_>,
    operation_port: &mut dyn OperationCustodyPort,
    session_port: &dyn SessionLeaseCustodyPort,
    anchor_port: &mut dyn AntiRollbackAnchorPort,
    admit_request: Request,
    admission: LocalOperationRecoveryAdmission<
        Authority,
        HandleSignature,
        CheckpointSignature,
        ObservationSigner,
        ObservationState,
        AuthorityRotation,
        ObservationSignature,
        HistoricalCheckpoint,
    >,
) -> Result<RecoveredLocalOperation, LocalOperationRecoveryError>
where
    Request: FnMut(
        &ResidentRecordRefV1,
        CommonRequestEnvelopeWireV1,
    ) -> Result<CommonRequestEnvelopeV1, InvalidResidentRecordV1>,
    Authority: FnMut(
        &OperationHandlePreimageV1,
        Option<&OperationHandleV1>,
    ) -> Result<(), InvalidResidentRecordV1>,
    HandleSignature: FnMut(
        &[u8],
        &OperationHandlePreimageV1,
        &OperationHandleDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
    CheckpointSignature: FnMut(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
    HistoricalCheckpoint: FnMut(
        &ResidentRecordRefV1,
        ControlStateCheckpointWireV1,
    ) -> Result<ControlStateCheckpointV1, InvalidResidentRecordV1>,
    ObservationSigner: FnMut(
        &ResidentRecordRefV1,
        &OperationObservationWireV1,
    ) -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1>,
    ObservationState: FnMut(&OperationObservationStateV1) -> Result<(), InvalidResidentRecordV1>,
    AuthorityRotation: FnMut(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
    ObservationSignature: FnMut(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    let context = input.context;
    context.verify_limits()?;
    let recovered_store = context.store.clone();
    let handle_schema = pale_contracts::record_schema_identity(OPERATION_HANDLE_V1_SCHEMA_ID)?;
    let request_schema =
        pale_contracts::record_schema_identity(COMMON_REQUEST_ENVELOPE_V1_SCHEMA_ID)?;
    let mut admit_request = admit_request;
    let mut next = input.expected_handle.clone();
    let mut chain = Vec::new();
    let mut parent_edges = 0usize;
    loop {
        if chain
            .iter()
            .any(|candidate: &RecoveredHandleInput| candidate.handle_record.coordinate() == &next)
        {
            return Err(LocalOperationRecoveryError::Association);
        }
        let candidate = read_handle_input(
            context,
            &next,
            &handle_schema,
            &request_schema,
            operation_port,
            &mut admit_request,
        )?;
        let parent = candidate.handle_hint.parent_operation().clone();
        chain.push(candidate);
        let Some(parent) = parent else {
            break;
        };
        if parent_edges == input.maximum_parent_depth {
            return Err(LocalOperationRecoveryError::Bounds);
        }
        parent_edges += 1;
        next = ControlRecordCoordinate::new(
            parent.operation_handle_ref,
            pale_ports::ControlRecordDigest::parse(parent.operation_handle_sha256.as_str())?,
        );
    }

    let LocalOperationRecoveryAdmission {
        mut admit_historical_authority,
        mut verify_handle_signature,
        mut verify_checkpoint_signature,
        initial_observation,
    } = admission;
    let OperationObservationRecoveryAdmission {
        mut admit_checkpoint,
        mut resolve_signer,
        mut admit_state,
        mut admit_rotation,
        mut verify_signature,
    } = initial_observation;
    let mut recovered_parent: Option<RecoveredLocalOperation> = None;
    for candidate in chain.into_iter().rev() {
        let parent = recovered_parent.as_ref().map(|recovered| {
            (
                &recovered.operation,
                recovered.operation.historical_operation_lease(),
            )
        });
        let recovered = recover_with_parent(
            context,
            &candidate.request,
            parent,
            operation_port,
            session_port,
            anchor_port,
            LocalOperationRecoveryAdmission {
                admit_historical_authority: &mut admit_historical_authority,
                verify_handle_signature: &mut verify_handle_signature,
                verify_checkpoint_signature: &mut verify_checkpoint_signature,
                initial_observation: OperationObservationRecoveryAdmission {
                    admit_checkpoint: &mut admit_checkpoint,
                    resolve_signer: &mut resolve_signer,
                    admit_state: &mut admit_state,
                    admit_rotation: &mut admit_rotation,
                    verify_signature: &mut verify_signature,
                },
            },
        )?;
        if recovered.operation.request_coordinate() != candidate.request_record.coordinate()
            || recovered.handle_record != candidate.handle_record
        {
            return Err(LocalOperationRecoveryError::Association);
        }
        let recovered_parent_chain = recovered_parent.take().map(Box::new);
        recovered_parent = Some(RecoveredLocalOperation {
            operation: recovered.operation,
            request: candidate.request,
            store: recovered_store.clone(),
            initial_observation: Some(Box::new(OperationObservationRecoveryAdmissionEvidence {
                observation: recovered.initial_observation,
            })),
            recovered_parent: recovered_parent_chain,
        });
    }
    recovered_parent.ok_or(LocalOperationRecoveryError::Missing)
}

/// Recover one saved child without rediscovering or independently admitting its parent.
///
/// This is the handle-selected entry to the same pairwise child proof used by
/// [`super::recover_local_child_operation`]. It reads and admits the original
/// child request, then uses the supplied protected parent, parent lease,
/// parent-child membership, session, generation and narrowed lease checks.
/// It performs no current-authority admission, observation transition or dispatch.
pub(in crate::resident) fn recover_local_child_operation_from_handle<
    Request,
    Authority,
    HandleSignature,
    CheckpointSignature,
    ObservationSigner,
    ObservationState,
    AuthorityRotation,
    ObservationSignature,
    HistoricalCheckpoint,
>(
    input: LocalChildOperationHandleRecoveryInput<'_>,
    operation_port: &mut dyn OperationCustodyPort,
    session_port: &dyn SessionLeaseCustodyPort,
    anchor_port: &mut dyn AntiRollbackAnchorPort,
    mut admit_request: Request,
    admission: LocalOperationRecoveryAdmission<
        Authority,
        HandleSignature,
        CheckpointSignature,
        ObservationSigner,
        ObservationState,
        AuthorityRotation,
        ObservationSignature,
        HistoricalCheckpoint,
    >,
) -> Result<RecoveredLocalOperation, LocalOperationRecoveryError>
where
    Request: FnMut(
        &ResidentRecordRefV1,
        CommonRequestEnvelopeWireV1,
    ) -> Result<CommonRequestEnvelopeV1, InvalidResidentRecordV1>,
    Authority: FnMut(
        &OperationHandlePreimageV1,
        Option<&OperationHandleV1>,
    ) -> Result<(), InvalidResidentRecordV1>,
    HandleSignature: FnMut(
        &[u8],
        &OperationHandlePreimageV1,
        &OperationHandleDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
    CheckpointSignature: FnMut(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
    HistoricalCheckpoint: FnMut(
        &ResidentRecordRefV1,
        ControlStateCheckpointWireV1,
    ) -> Result<ControlStateCheckpointV1, InvalidResidentRecordV1>,
    ObservationSigner: FnMut(
        &ResidentRecordRefV1,
        &OperationObservationWireV1,
    ) -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1>,
    ObservationState: FnMut(&OperationObservationStateV1) -> Result<(), InvalidResidentRecordV1>,
    AuthorityRotation: FnMut(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
    ObservationSignature: FnMut(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    input.child.context.verify_limits()?;
    let handle_schema = pale_contracts::record_schema_identity(OPERATION_HANDLE_V1_SCHEMA_ID)?;
    let request_schema =
        pale_contracts::record_schema_identity(COMMON_REQUEST_ENVELOPE_V1_SCHEMA_ID)?;
    let candidate = read_handle_input(
        input.child.context,
        input.expected_handle,
        &handle_schema,
        &request_schema,
        operation_port,
        &mut admit_request,
    )?;
    let expected_parent = input.child.parent.handle();
    let Some(parent) = candidate.handle_hint.parent_operation() else {
        return Err(LocalOperationRecoveryError::Association);
    };
    if parent.operation_handle_ref != *expected_parent.0
        || parent.operation_handle_id != *expected_parent.1.operation_handle_id()
        || parent.operation_handle_sha256 != *expected_parent.1.canonical_operation_handle_sha256()
    {
        return Err(LocalOperationRecoveryError::Association);
    }
    let recovered = recover_with_parent(
        input.child.context,
        &candidate.request,
        Some((input.child.parent, input.child.parent_lease)),
        operation_port,
        session_port,
        anchor_port,
        admission,
    )?;
    if recovered.operation.request_coordinate() != candidate.request_record.coordinate()
        || recovered.handle_record != candidate.handle_record
    {
        return Err(LocalOperationRecoveryError::Association);
    }
    Ok(RecoveredLocalOperation {
        operation: recovered.operation,
        request: candidate.request,
        store: input.child.context.store.clone(),
        initial_observation: Some(Box::new(OperationObservationRecoveryAdmissionEvidence {
            observation: recovered.initial_observation,
        })),
        recovered_parent: None,
    })
}

pub(super) fn read_handle_input<Request>(
    context: LocalOperationRecoveryContext<'_>,
    expected_handle: &ControlRecordCoordinate,
    handle_schema: &RecordSchemaIdentity,
    request_schema: &RecordSchemaIdentity,
    operation_port: &mut dyn OperationCustodyPort,
    admit_request: &mut Request,
) -> Result<RecoveredHandleInput, LocalOperationRecoveryError>
where
    Request: FnMut(
        &ResidentRecordRefV1,
        CommonRequestEnvelopeWireV1,
    ) -> Result<CommonRequestEnvelopeV1, InvalidResidentRecordV1>,
{
    let handle_record = operation_port
        .read_operation_record(context.store, expected_handle, context.maximum_record_bytes)?
        .ok_or(LocalOperationRecoveryError::Missing)?;
    if handle_record.coordinate() != expected_handle {
        return Err(LocalOperationRecoveryError::Association);
    }
    check_record(
        &handle_record,
        &handle_schema.schema,
        &handle_schema.schema_sha256,
        context.maximum_record_bytes,
    )?;
    let handle_hint: OperationHandleWireV1 = serde_json::from_value(canon::parse_strict(
        handle_record.canonical_record_jcs().as_slice(),
    )?)?;
    let key = operation_handle_key(handle_hint.operation_handle_id())
        .map_err(OperationRecoveryError::from)?;
    let request_record = read_index(
        operation_port,
        context.store,
        OperationCustodyIndexQuery::RequestForHandle(key),
        context.maximum_record_bytes,
    )?
    .ok_or(LocalOperationRecoveryError::Missing)?;
    check_record(
        &request_record,
        &request_schema.schema,
        &request_schema.schema_sha256,
        context.maximum_record_bytes,
    )?;
    let wire = serde_json::from_value(canon::parse_strict(
        request_record.canonical_record_jcs().as_slice(),
    )?)?;
    let request = admit_request(request_record.coordinate().record_ref(), wire)?;
    if request.envelope_schema_id() != &request_schema.schema
        || request.envelope_schema_sha256() != &request_schema.schema_sha256
        || request.canonical_request_sha256().as_str()
            != request_record.coordinate().record_sha256().as_str()
        || canon::to_canonical_string(&request)?.as_bytes()
            != request_record.canonical_record_jcs().as_slice()
    {
        return Err(LocalOperationRecoveryError::Association);
    }
    Ok(RecoveredHandleInput {
        handle_record,
        request_record,
        request,
        handle_hint,
    })
}

pub(super) struct RecoveredHandleInput {
    pub(super) handle_record: pale_ports::ControlRecord,
    pub(super) request_record: pale_ports::ControlRecord,
    pub(super) request: CommonRequestEnvelopeV1,
    pub(super) handle_hint: OperationHandleWireV1,
}
