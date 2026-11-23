//! Select a descendant expiry through its bounded exact chain to the discovered root.
//! Historical identity and membership prove only this repair, never execution.

use super::*;
use crate::resident::{
    checkpoint_profile::{operation_identity_leaves, operation_session_basis_leaf},
    operation_custody::read_index,
    operation_observation_custody::operation_handle_key,
    OperationControlRecoveryAdmission, OperationStartRecoveryEvidence,
};
use pale_ports::{AccumulatorMembershipLeaf, OperationCustodyIndexQuery};

pub(super) struct RootIdentity<'a> {
    pub(super) request: &'a CommonRequestEnvelopeV1,
    pub(super) start: &'a OperationStartRecoveryEvidence,
    pub(super) handle: &'a OperationHandleV1,
    pub(super) lease: &'a OperationLeaseV1,
    pub(super) maximum_parent_depth: usize,
}

pub(super) struct ExpiryIdentity {
    pub(super) start: Option<OperationStartRecoveryEvidence>,
    pub(super) handle: OperationHandleV1,
    pub(super) selection: OperationCheckpointSelection,
    pub(super) leaves: Vec<AccumulatorMembershipLeaf>,
}

pub(super) fn select_expiry_identity(
    input: LocalOperationRecoveryContext<'_>,
    root: RootIdentity<'_>,
    checkpoint: &ControlStateCheckpointV1,
    custody: &mut dyn OperationCustodyPort,
    sessions: &dyn SessionLeaseCustodyPort,
    admission: &mut OperationControlRecoveryAdmission<'_>,
) -> Result<ExpiryIdentity, LocalOperationRecoveryError> {
    let selection = input.profile.select(input.store, root.request)?;
    selection.verify(input.store, root.handle, checkpoint)?;
    let mut leaves = operation_identity_leaves(
        &selection,
        (root.start.request().coordinate().record_ref(), root.request),
        (root.start.handle().coordinate().record_ref(), root.handle),
    )
    .and_then(|leaves| leaves.membership())
    .map_err(|_| LocalOperationRecoveryError::Association)?;
    let [changed] = checkpoint.changed_leaf_keys().as_slice() else {
        return Err(LocalOperationRecoveryError::Association);
    };
    if changed.record_class.as_str() != "operation-observation" {
        return Err(LocalOperationRecoveryError::Association);
    }
    let target = OperationHandleIdV1::parse(changed.logical_key.as_str())?;
    if &target == root.handle.operation_handle_id() {
        return Ok(ExpiryIdentity {
            start: None,
            handle: root.handle.clone(),
            selection,
            leaves,
        });
    }
    if root.maximum_parent_depth == 0 {
        return Err(LocalOperationRecoveryError::Bounds);
    }
    let key = operation_handle_key(&target).map_err(OperationRecoveryError::from)?;
    let handle_record = read_index(
        custody,
        input.store,
        OperationCustodyIndexQuery::OperationHandle(key),
        input.maximum_record_bytes,
    )?
    .ok_or(LocalOperationRecoveryError::Missing)?;
    let mut next = handle_record.coordinate().clone();
    let mut chain = Vec::new();
    while &next != root.start.handle().coordinate() {
        if chain.len() == root.maximum_parent_depth {
            return Err(LocalOperationRecoveryError::Bounds);
        }
        if chain
            .iter()
            .any(|candidate: &by_handle::RecoveredHandleInput| {
                candidate.handle_record.coordinate() == &next
            })
        {
            return Err(LocalOperationRecoveryError::Association);
        }
        let candidate = by_handle::read_handle_input(
            input,
            &next,
            &pale_contracts::record_schema_identity(OPERATION_HANDLE_V1_SCHEMA_ID)?,
            &pale_contracts::record_schema_identity(COMMON_REQUEST_ENVELOPE_V1_SCHEMA_ID)?,
            custody,
            &mut admission.admit_request,
        )?;
        let parent = candidate
            .handle_hint
            .parent_operation()
            .as_ref()
            .ok_or(LocalOperationRecoveryError::Association)?;
        next = record_projection::coordinate(
            &parent.operation_handle_ref,
            parent.operation_handle_sha256.as_str(),
        )
        .map_err(|_| LocalOperationRecoveryError::Association)?;
        chain.push(candidate);
    }
    let mut parent_ref = root.start.handle().coordinate().record_ref().clone();
    let mut parent_handle = root.handle.clone();
    let mut parent_lease = root.lease.clone();
    let mut selection = selection;
    let mut target_start = None;
    for candidate in chain.into_iter().rev() {
        let start = read_operation_start_for_request(
            custody,
            input.store,
            &candidate.request,
            input.maximum_record_bytes,
        )?
        .ok_or(LocalOperationRecoveryError::Missing)?;
        if start.request() != &candidate.request_record
            || start.handle() != &candidate.handle_record
        {
            return Err(LocalOperationRecoveryError::Association);
        }
        let (handle, lease, session) = identity::admit_handle(
            input,
            &candidate.request,
            Some((&parent_ref, &parent_handle, &parent_lease)),
            &candidate.handle_record,
            custody,
            sessions,
            (
                &mut *admission.admit_handle_authority,
                &mut *admission.verify_handle_signature,
            ),
        )?;
        selection = selection.child(input.profile, &candidate.request)?;
        selection.verify(input.store, &handle, checkpoint)?;
        leaves.extend(
            operation_identity_leaves(
                &selection,
                (
                    start.request().coordinate().record_ref(),
                    &candidate.request,
                ),
                (start.handle().coordinate().record_ref(), &handle),
            )
            .and_then(|leaves| leaves.membership())
            .map_err(|_| LocalOperationRecoveryError::Association)?,
        );
        let basis = custody
            .read_operation_session_basis(
                input.store,
                start.handle().coordinate(),
                input
                    .maximum_record_bytes
                    .min(pale_ports::MAX_OPERATION_SESSION_BASIS_BYTES),
            )?
            .ok_or(LocalOperationRecoveryError::Missing)?;
        let (_, _, intent) = super::super::session_basis::decode(
            &basis,
            &selection,
            start.handle().coordinate(),
            start.request().coordinate(),
            &session,
            input.maximum_record_bytes,
        )?;
        if let Some(intent) = intent {
            intent
                .validate_signed_parent(
                    input.store,
                    start.handle().coordinate().record_ref(),
                    &handle,
                )
                .map_err(map_consultation_start_intent_error)?;
        }
        leaves.push(
            operation_session_basis_leaf(
                &selection,
                (start.handle().coordinate().record_ref(), &handle),
                (
                    start.request().coordinate().record_ref(),
                    &candidate.request,
                ),
                &session,
                &basis,
            )
            .and_then(|leaf| leaf.membership())
            .map_err(|_| LocalOperationRecoveryError::Association)?,
        );
        parent_ref = start.handle().coordinate().record_ref().clone();
        parent_handle = handle;
        parent_lease = lease;
        target_start = Some(start);
    }
    if parent_handle.operation_handle_id() != &target
        || !matches!(
            parent_handle.operation_class(),
            OperationClassV1::Invoke | OperationClassV1::Delegate
        )
    {
        return Err(LocalOperationRecoveryError::Association);
    }
    let start = Some(target_start.ok_or(LocalOperationRecoveryError::Missing)?);
    let handle = parent_handle;
    leaves.sort_unstable_by(|left, right| left.key().cmp(right.key()));
    Ok(ExpiryIdentity {
        start,
        handle,
        selection,
        leaves,
    })
}
