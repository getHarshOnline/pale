//! Initial root or child operation leaves and their derived primary lookups.
//! No physical checkpoint or observation-checkpoint lookup is hashed into itself.

use pale_core::{canon, records::*};
use pale_ports::{
    ControlRecordCoordinate, OperationCustodyIndexKey, OperationCustodyIndexMutation,
    OperationCustodyIndexPredecessor, OperationSessionBasis,
};
use serde::Serialize;

use super::{
    canonical_hash, control_key, observation_hash, OperationCheckpointSelection,
    OperationLeafProjectionError, HANDLE_VALUE_DOMAIN, KEY_DOMAIN, REQUEST_VALUE_DOMAIN,
};
use crate::resident::{
    operation_custody::{operation_request_key, OperationRecoveryError},
    operation_observation_custody::operation_handle_key,
    record_projection,
};

#[derive(Serialize)]
struct RequestAssociationLeaf {
    domain: &'static str,
    key: ChangedControlLeafKeyV1,
    schema_id: SchemaId,
    request_ref: ResidentRecordRefV1,
    request_sha256: CommonRequestEnvelopeDigest,
    handle_ref: ResidentRecordRefV1,
    handle_sha256: OperationHandleDigest,
    signer_key_epoch: SigningKeyEpochV1,
    disposition: &'static str,
}

#[derive(Serialize)]
struct HandleLeaf {
    domain: &'static str,
    key: ChangedControlLeafKeyV1,
    schema_id: SchemaId,
    record_ref: ResidentRecordRefV1,
    record_sha256: OperationHandleDigest,
    compiled_generation_sha256: CompiledGenerationDigest,
    operation_class: OperationClassV1,
    disposition: &'static str,
}

/// Immutable identity leaves shared by root/child start and protected locator recovery.
/// Retains bounded coordinates, never another copy of the request body.
pub(in crate::resident) struct OperationIdentityLeaves {
    request_key: OperationCustodyIndexKey,
    handle_key: OperationCustodyIndexKey,
    request: RequestAssociationLeaf,
    handle: HandleLeaf,
}

/// Initial insertion retains the same identity projection used by recovery.
pub(in crate::resident) struct InitialOperationLeaves {
    identity: OperationIdentityLeaves,
    session_basis: super::session_basis::SessionBasisLeaf,
    observation_key: ChangedControlLeafKeyV1,
    observation: ControlRecordCoordinate,
    values: [ArtifactDigest; 3],
}

#[derive(Debug, thiserror::Error)]
pub(in crate::resident) enum InitialOperationLeafError {
    #[error(transparent)]
    Profile(#[from] super::OperationCheckpointProfileError),
    #[error(transparent)]
    Leaf(#[from] OperationLeafProjectionError),
    #[error(transparent)]
    Lookup(#[from] OperationRecoveryError),
}

pub(in crate::resident) fn operation_identity_leaves(
    selection: &OperationCheckpointSelection,
    (request_ref, request): (&ResidentRecordRefV1, &CommonRequestEnvelopeV1),
    (handle_ref, handle): (&ResidentRecordRefV1, &OperationHandleV1),
) -> Result<OperationIdentityLeaves, InitialOperationLeafError> {
    let basis = selection.initial_basis()?;
    let key =
        |class, logical| control_key(basis.store(), basis.domain(), basis.epoch(), class, logical);
    let request_key = operation_request_key(request)?;
    let handle_key =
        operation_handle_key(handle.operation_handle_id()).map_err(OperationRecoveryError::from)?;
    let request_value = RequestAssociationLeaf {
        domain: REQUEST_VALUE_DOMAIN,
        key: key("operation-request", request_key.as_str())?,
        schema_id: request.envelope_schema_id().clone(),
        request_ref: request_ref.clone(),
        request_sha256: request.canonical_request_sha256().clone(),
        handle_ref: handle_ref.clone(),
        handle_sha256: handle.canonical_operation_handle_sha256().clone(),
        signer_key_epoch: request.caller().signer_key_epoch,
        disposition: "retained",
    };
    let handle_value = HandleLeaf {
        domain: HANDLE_VALUE_DOMAIN,
        key: key("operation-handle", handle.operation_handle_id().as_str())?,
        schema_id: handle.schema_id().clone(),
        record_ref: handle_ref.clone(),
        record_sha256: handle.canonical_operation_handle_sha256().clone(),
        compiled_generation_sha256: handle.compiled_generation_sha256().clone(),
        operation_class: *handle.operation_class(),
        disposition: "retained",
    };
    Ok(OperationIdentityLeaves {
        request_key,
        handle_key,
        request: request_value,
        handle: handle_value,
    })
}

pub(in crate::resident) fn initial_operation_leaves(
    selection: &OperationCheckpointSelection,
    request: (&ResidentRecordRefV1, &CommonRequestEnvelopeV1),
    handle: (&ResidentRecordRefV1, &OperationHandleV1),
    (observation_ref, observation): (&ResidentRecordRefV1, &OperationObservationV1),
    session_basis: &OperationSessionBasis,
    session: &SessionAdmissionLeaseV1,
) -> Result<InitialOperationLeaves, InitialOperationLeafError> {
    let identity = operation_identity_leaves(selection, request, handle)?;
    let session_basis =
        super::operation_session_basis_leaf(selection, handle, request, session, session_basis)?;
    let basis = selection.initial_basis()?;
    let handle = handle.1;
    let observation_key = control_key(
        basis.store(),
        basis.domain(),
        basis.epoch(),
        "operation-observation",
        handle.operation_handle_id().as_str(),
    )?;
    let values = [
        canonical_hash(&identity.request).map_err(OperationLeafProjectionError::from)?,
        canonical_hash(&identity.handle).map_err(OperationLeafProjectionError::from)?,
        observation_hash(&observation_key, observation_ref, observation)
            .map_err(OperationLeafProjectionError::from)?,
    ];
    Ok(InitialOperationLeaves {
        identity,
        session_basis,
        observation_key,
        observation: record_projection::coordinate(
            observation_ref,
            observation
                .canonical_operation_observation_sha256()
                .as_str(),
        )
        .map_err(OperationRecoveryError::from)?,
        values,
    })
}

impl OperationIdentityLeaves {
    pub(in crate::resident) fn membership(
        &self,
    ) -> Result<Vec<pale_ports::AccumulatorMembershipLeaf>, InitialOperationLeafError> {
        let mut leaves = Vec::with_capacity(2);
        for (key, value) in [
            (
                &self.request.key,
                canonical_hash(&self.request).map_err(OperationLeafProjectionError::from)?,
            ),
            (
                &self.handle.key,
                canonical_hash(&self.handle).map_err(OperationLeafProjectionError::from)?,
            ),
        ] {
            let hashed_key =
                canonical_hash(&(KEY_DOMAIN, key)).map_err(OperationLeafProjectionError::from)?;
            leaves.push(
                pale_ports::AccumulatorMembershipLeaf::new(hashed_key, value)
                    .map_err(|_| super::OperationCheckpointProfileError::Association)?,
            );
        }
        leaves.sort_unstable_by(|left, right| left.key().cmp(right.key()));
        Ok(leaves)
    }
}

impl InitialOperationLeaves {
    pub(in crate::resident) fn changes(
        &self,
    ) -> Result<Vec<(ChangedControlLeafKeyV1, ArtifactDigest, ArtifactDigest)>, canon::CanonError>
    {
        let mut changes = [
            &self.identity.request.key,
            &self.identity.handle.key,
            &self.observation_key,
        ]
        .into_iter()
        .zip(&self.values)
        .map(|(key, value)| {
            Ok((
                key.clone(),
                canonical_hash(&(KEY_DOMAIN, key))?,
                value.clone(),
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;
        changes.push(self.session_basis.change()?);
        Ok(changes)
    }

    /// Rebuild the existing four primary lookups from the authenticated leaf
    /// associations. Recovery lookups remain the physical candidate's owner.
    pub(in crate::resident) fn indexes(
        &self,
    ) -> Result<Vec<OperationCustodyIndexMutation>, OperationRecoveryError> {
        let request = record_projection::coordinate(
            &self.identity.request.request_ref,
            self.identity.request.request_sha256.as_str(),
        )?;
        let handle = record_projection::coordinate(
            &self.identity.request.handle_ref,
            self.identity.request.handle_sha256.as_str(),
        )?;
        let indexed_handle = record_projection::coordinate(
            &self.identity.handle.record_ref,
            self.identity.handle.record_sha256.as_str(),
        )?;
        Ok(vec![
            OperationCustodyIndexMutation::RequestEvidence {
                key: self.identity.request_key.clone(),
                expected: OperationCustodyIndexPredecessor::Absent,
                successor: request,
            },
            OperationCustodyIndexMutation::RequestOperation {
                key: self.identity.request_key.clone(),
                expected: OperationCustodyIndexPredecessor::Absent,
                successor: handle,
            },
            OperationCustodyIndexMutation::OperationHandle {
                key: self.identity.handle_key.clone(),
                expected: OperationCustodyIndexPredecessor::Absent,
                successor: indexed_handle,
            },
            OperationCustodyIndexMutation::LatestObservation {
                key: self.identity.handle_key.clone(),
                expected: OperationCustodyIndexPredecessor::Absent,
                successor: self.observation.clone(),
            },
        ])
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/checkpoint_profile/initial.rs"]
mod tests;
