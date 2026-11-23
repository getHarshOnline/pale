//! Immutable operation-session provenance leaf, independent of current admission.

use pale_core::records::*;
use pale_ports::{AccumulatorMembershipLeaf, OperationSessionBasis};
use serde::Serialize;

use super::{
    canonical_hash, control_key, OperationCheckpointSelection, OperationLeafProjectionError,
    KEY_DOMAIN,
};

pub(super) const SESSION_BASIS_VALUE_DOMAIN: &str = "PALE_OPERATION_SESSION_BASIS_LEAF_V1";

#[derive(Serialize)]
pub(in crate::resident) struct SessionBasisLeaf {
    domain: &'static str,
    key: ChangedControlLeafKeyV1,
    handle: pale_ports::ControlRecordCoordinate,
    request: pale_ports::ControlRecordCoordinate,
    session_lease_digest: SessionAdmissionLeaseDigest,
    basis_sha256: ArtifactDigest,
}

pub(in crate::resident) fn operation_session_basis_leaf(
    selection: &OperationCheckpointSelection,
    (handle_ref, handle): (&ResidentRecordRefV1, &OperationHandleV1),
    (request_ref, request): (&ResidentRecordRefV1, &CommonRequestEnvelopeV1),
    session: &SessionAdmissionLeaseV1,
    basis: &OperationSessionBasis,
) -> Result<SessionBasisLeaf, super::InitialOperationLeafError> {
    let scope = selection.initial_basis()?;
    if basis.handle().record_ref() != handle_ref
        || basis.handle().record_sha256().as_str()
            != handle.canonical_operation_handle_sha256().as_str()
        || basis.request().record_ref() != request_ref
        || basis.request().record_sha256().as_str() != request.canonical_request_sha256().as_str()
        || basis.session_lease_digest() != session.session_admission_lease_digest()
    {
        return Err(super::OperationCheckpointProfileError::Association.into());
    }
    Ok(SessionBasisLeaf {
        domain: SESSION_BASIS_VALUE_DOMAIN,
        key: control_key(
            scope.store(),
            scope.domain(),
            scope.epoch(),
            "operation-session-basis",
            handle.operation_handle_id().as_str(),
        )?,
        handle: basis.handle().clone(),
        request: basis.request().clone(),
        session_lease_digest: basis.session_lease_digest().clone(),
        basis_sha256: basis.basis_sha256().clone(),
    })
}

impl SessionBasisLeaf {
    pub(super) fn change(
        &self,
    ) -> Result<
        (ChangedControlLeafKeyV1, ArtifactDigest, ArtifactDigest),
        pale_core::canon::CanonError,
    > {
        Ok((
            self.key.clone(),
            canonical_hash(&(KEY_DOMAIN, &self.key))?,
            canonical_hash(self)?,
        ))
    }

    pub(in crate::resident) fn membership(
        &self,
    ) -> Result<AccumulatorMembershipLeaf, super::InitialOperationLeafError> {
        let (_, key, value) = self.change().map_err(OperationLeafProjectionError::from)?;
        AccumulatorMembershipLeaf::new(key, value)
            .map_err(|_| super::OperationCheckpointProfileError::Association.into())
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/checkpoint_profile/session_basis.rs"]
mod tests;
