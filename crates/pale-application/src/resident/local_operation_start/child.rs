//! Protected current parent selection before a child start.

use super::{
    require, LocalOperationStartError, LocalOperationStartInput, PreparedLocalOperationStart,
    RetainedLocalOperationStartFailure, StartBasis,
};
use crate::resident::{
    checkpoint_profile::OperationCheckpointSelection,
    operation_observation_custody::{
        recover_with_checkpoint, OperationObservationRecoveryAdmission,
        OperationObservationRecoveryInput, OperationObservationSelection,
    },
    session_admission::AmbientSessionAuthorityPort,
    RecoveredOperationControl,
};
use pale_core::{canon, records::*};
use pale_ports::{
    AntiRollbackAnchorPort, ControlTransactionPreparation, OperationCustodyIndexKey,
    OperationCustodyIndexMutation, OperationCustodyIndexPredecessor, OperationCustodyPort,
};

/// A protected active parent and its retained transaction predecessor.
/// This is preparation evidence, not a reservation or permission to dispatch.
/// No caller can replace the selected parent, domain, checkpoint or transaction.
#[must_use = "parent selection still requires child admission and atomic custody"]
pub struct PreparedLocalChildParent {
    pub(super) selection: OperationCheckpointSelection,
    pub(super) checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    pub(super) transaction: ControlTransactionPreparation,
    reference: ResidentRecordRefV1,
    handle: OperationHandleV1,
    lease: OperationLeaseV1,
    observed_at: UnixTimeV1,
}

/// Read and admit the selected current parent before preparing child records.
/// Capture the store transaction first and retain it through child commitment.
/// Existing bounded observation recovery checks history, current membership and
/// both independent-anchor reads. These reads are not an atomic snapshot: the
/// later native accumulator and custody transaction must compare this same fence.
/// The parent must be active; current time and narrowed child authority are
/// checked by child admission, not inferred from this historical evidence.
/// # Errors
/// Rejects historical selection before I/O, wrong lease association, inactive
/// parent, failed bounded recovery or unlike physical transaction coordinates.
pub fn prepare_local_child_parent<
    Signer,
    State,
    Rotation,
    Signature,
    Checkpoint,
    CheckpointSignature,
>(
    input: OperationObservationRecoveryInput<'_>,
    parent_lease: &OperationLeaseV1,
    port: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    admissions: OperationObservationRecoveryAdmission<
        Signer,
        State,
        Rotation,
        Signature,
        Checkpoint,
    >,
    verify_checkpoint_signature: CheckpointSignature,
) -> Result<PreparedLocalChildParent, LocalOperationStartError>
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
    require(matches!(
        input.observation,
        OperationObservationSelection::SelectedHead
    ))?;
    let (reference, handle) = input.operation.handle();
    require(handle.operation_lease_sha256() == parent_lease.operation_lease_digest())?;
    let selection = input.operation.checkpoint_selection();
    require(selection.matches_store(input.store))?;
    let transaction = port
        .prepare_transaction(input.store)
        .map_err(pale_ports::OperationAccumulatorPreparationError::from)?;
    require(transaction.store() == input.store)?;
    let reference = reference.clone();
    let handle = handle.clone();
    let recovered = recover_with_checkpoint(
        input,
        port,
        anchor,
        admissions,
        verify_checkpoint_signature,
        &mut |_, _| {},
    )?;
    require(matches!(
        recovered.selected().1.state(),
        OperationObservationStateV1::Active { .. }
    ))?;
    let observed_at = *recovered.selected().1.observed_at();
    Ok(PreparedLocalChildParent {
        selection,
        checkpoint: recovered.current.into_record(),
        transaction,
        reference,
        handle,
        lease: parent_lease.clone(),
        observed_at,
    })
}

/// Bind an exact point-recovered parent to a transaction prepared before its read.
///
/// The caller must prepare the transaction from the selected deployment store
/// before protected recovery. This adapter consumes that already-admitted
/// recovery without repeating its custody, checkpoint, or anchor reads.
pub(in crate::resident) fn prepare_local_child_parent_from_recovered(
    store: &pale_ports::AuthorityStoreCoordinate,
    transaction: ControlTransactionPreparation,
    recovered: RecoveredOperationControl,
) -> Result<PreparedLocalChildParent, LocalOperationStartError> {
    require(transaction.store() == store)?;
    let (operation, observation) = recovered.into_parts();
    require(operation.store() == store && observation.store() == store)?;
    let issued = operation.operation();
    let (reference, handle) = issued.handle();
    let lease = issued.historical_operation_lease();
    require(handle.operation_lease_sha256() == lease.operation_lease_digest())?;
    let selection = issued.checkpoint_selection();
    require(selection.matches_store(store))?;
    require(matches!(
        observation.selected().1.state(),
        OperationObservationStateV1::Active { .. }
    ))?;
    let observed_at = *observation.selected().1.observed_at();
    Ok(PreparedLocalChildParent {
        selection,
        checkpoint: observation.current.into_record(),
        transaction,
        reference: reference.clone(),
        handle: handle.clone(),
        lease: lease.clone(),
        observed_at,
    })
}

impl PreparedLocalChildParent {
    /// Duplicate the protected parent for one bounded semantic-start attempt.
    ///
    /// The process owner retains the original only until the attempt returns.
    /// A preparation failure therefore returns the original linear phase, while
    /// success or physical custody failure discards it in favor of the issued
    /// records or the exact custody continuation.
    pub(in crate::resident) fn duplicate_for_start_attempt(&self) -> Self {
        Self {
            selection: self.selection.clone(),
            checkpoint: self.checkpoint.clone(),
            transaction: self.transaction.clone(),
            reference: self.reference.clone(),
            handle: self.handle.clone(),
            lease: self.lease.clone(),
            observed_at: self.observed_at,
        }
    }

    /// Borrow the exact store selected with the protected parent transaction.
    #[must_use]
    pub(in crate::resident) const fn store(&self) -> &pale_ports::AuthorityStoreCoordinate {
        self.transaction.store()
    }

    /// Borrow the protected parent handle coordinate.
    #[must_use]
    pub(in crate::resident) fn operation(&self) -> (&ResidentRecordRefV1, &OperationHandleV1) {
        (&self.reference, &self.handle)
    }

    /// Borrow the protected parent lease admitted with the recovered handle.
    pub(in crate::resident) const fn lease(&self) -> &OperationLeaseV1 {
        &self.lease
    }

    pub(super) fn verify_child(
        &self,
        input: &LocalOperationStartInput<'_>,
    ) -> Result<(), LocalOperationStartError> {
        let parent = input
            .context
            .parent_operation
            .as_ref()
            .ok_or(LocalOperationStartError::Coordinate)?;
        require(
            parent.operation_handle_ref == &self.reference
                && parent.operation_handle == &self.handle
                && parent.operation_lease == &self.lease
                && input.current.now >= self.observed_at
                && input.current.now >= *self.handle.created_at()
                && input.current.now < *self.handle.expires_at()
                && input.current.now < self.lease.expires_at()
                && input.handle.expires_at <= *self.handle.expires_at()
                && input.handle.compiled_generation_sha256
                    == *self.handle.compiled_generation_sha256()
                && input.handle.activation_fence_sha256 == *self.handle.activation_fence_sha256()
                && input.handle.instance_sha256 == *self.handle.instance_sha256(),
        )?;
        self.lease.verify_against_parent(input.session.lease())?;
        input
            .context
            .operation_lease
            .verify_against_parent_operation(&self.lease)?;
        Ok(())
    }
}

pub(in crate::resident) fn prepare_local_child_start_retained<
    Authority,
    HandleSign,
    ObservationSign,
    SessionAuthority,
>(
    input: LocalOperationStartInput<'_>,
    parent: PreparedLocalChildParent,
    expected_class: OperationClassV1,
    session_authority: &mut SessionAuthority,
    admit_compiled_authority: Authority,
    sign_handle: HandleSign,
    sign_observation: ObservationSign,
) -> Result<PreparedLocalOperationStart, RetainedLocalOperationStartFailure>
where
    Authority: FnOnce(
        &CommonRequestEnvelopeV1,
        &crate::resident::AdmittedAmbientSession,
        &crate::resident::AmbientSessionAdmissionContext<'_>,
        &OperationHandlePreimageV1,
        &OperationLeaseV1,
        &crate::semantic::ResolvedOperation,
        &crate::resident::LocalOperationPublicationPolicyInput<'_>,
        &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    ) -> Result<(), InvalidResidentRecordV1>,
    SessionAuthority: AmbientSessionAuthorityPort,
    HandleSign: FnOnce(
        &[u8],
        &OperationHandlePreimageV1,
        &OperationHandleDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    ObservationSign: FnOnce(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    super::prepare_start_retained(
        input,
        (StartBasis::Child(Box::new(parent)), expected_class, None),
        |_, _, _| Ok(None),
        session_authority,
        admit_compiled_authority,
        sign_handle,
        sign_observation,
    )
}

pub(super) fn membership_key(
    parent: &ParentOperationReferenceV1,
    child: &OperationHandleIdV1,
) -> Result<OperationCustodyIndexKey, LocalOperationStartError> {
    let bytes =
        canon::to_canonical_string(&("PALE_OPERATION_PARENT_CHILD_INDEX_KEY_V1", parent, child))?;
    Ok(OperationCustodyIndexKey::parse(pale_core::sha256_hex(
        bytes.as_bytes(),
    ))?)
}

pub(super) fn membership_index(
    parent: &PreparedLocalChildParent,
    reference: &ResidentRecordRefV1,
    handle: &OperationHandleV1,
) -> Result<OperationCustodyIndexMutation, LocalOperationStartError> {
    let exact_parent = ParentOperationReferenceV1 {
        operation_handle_id: parent.handle.operation_handle_id().clone(),
        operation_handle_ref: parent.reference.clone(),
        operation_handle_sha256: parent.handle.canonical_operation_handle_sha256().clone(),
    };
    Ok(OperationCustodyIndexMutation::ParentChildMembership {
        key: membership_key(&exact_parent, handle.operation_handle_id())?,
        expected: OperationCustodyIndexPredecessor::Absent,
        successor: crate::resident::record_projection::coordinate(
            reference,
            handle.canonical_operation_handle_sha256().as_str(),
        )?,
    })
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/local_operation_start/child.rs"]
mod tests;
