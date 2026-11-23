//! Protected recovery of one operation and its current observation for control requests.

use super::{
    authorize_operation_cancellation, authorize_operation_observation,
    AuthorizedOperationCancellation, AuthorizedOperationObservation,
    OperationControlAuthorizationContext, OperationControlError, OperationControlInput,
};
use crate::resident::{
    recover_local_child_operation_from_handle, recover_local_operation_from_handle,
    recover_operation_observation, LocalChildOperationHandleRecoveryInput,
    LocalOperationHandleRecoveryInput, LocalOperationRecoveryAdmission,
    LocalOperationRecoveryError, OperationCheckpointAncestryLimits,
    OperationObservationReadmissionError, OperationObservationRecoveryAdmission,
    OperationObservationRecoveryInput, OperationObservationRecoveryLimits,
    OperationObservationSelection, RecoveredLocalOperation, RecoveredOperationObservation,
};
use pale_core::records::*;
use pale_ports::{
    AccumulatorMembershipLimits, AntiRollbackAnchorPort, OperationCustodyPort,
    SessionLeaseCustodyPort,
};

type RequestAdmission<'a> = dyn FnMut(
        &ResidentRecordRefV1,
        CommonRequestEnvelopeWireV1,
    ) -> Result<CommonRequestEnvelopeV1, InvalidResidentRecordV1>
    + 'a;
type HandleAuthorityAdmission<'a> = dyn FnMut(
        &OperationHandlePreimageV1,
        Option<&OperationHandleV1>,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;
type HandleSignatureVerification<'a> = dyn FnMut(
        &[u8],
        &OperationHandlePreimageV1,
        &OperationHandleDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;
type CheckpointAdmission<'a> = dyn FnMut(
        &ResidentRecordRefV1,
        ControlStateCheckpointWireV1,
    ) -> Result<ControlStateCheckpointV1, InvalidResidentRecordV1>
    + 'a;
type ObservationSignerResolution<'a> = dyn FnMut(
        &ResidentRecordRefV1,
        &OperationObservationWireV1,
    ) -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1>
    + 'a;
type ObservationStateAdmission<'a> =
    dyn FnMut(&OperationObservationStateV1) -> Result<(), InvalidResidentRecordV1> + 'a;
type AuthorityRotationAdmission<'a> = dyn FnMut(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;
type ObservationSignatureVerification<'a> = dyn FnMut(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;
type CheckpointSignatureVerification<'a> = dyn FnMut(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>
    + 'a;

/// Bounded inputs for recovering the operation selected by one saved handle.
pub struct OperationControlRecoveryInput<'a> {
    /// Saved handle, parent-depth limit, store, authority, and record ceilings.
    pub operation: LocalOperationHandleRecoveryInput<'a>,
    /// Current head or exact historical observation requested by the caller.
    pub observation: OperationObservationSelection<'a>,
    /// Observation revision, aggregate-byte, and per-record ceilings.
    pub observation_limits: OperationObservationRecoveryLimits,
    /// Checkpoint ancestry count, byte, and admission ceilings.
    pub checkpoint_limits: OperationCheckpointAncestryLimits,
    /// Native accumulator membership ceilings for the selected observation.
    pub membership_limits: AccumulatorMembershipLimits,
}

/// Bounded recovery of one child against its already protected parent.
pub(in crate::resident) struct ChildOperationControlRecoveryInput<'a> {
    /// Exact child handle, parent, parent lease and historical recovery context.
    pub(in crate::resident) operation: LocalChildOperationHandleRecoveryInput<'a>,
    /// Current head or exact historical observation requested by the owner.
    pub(in crate::resident) observation: OperationObservationSelection<'a>,
    /// Observation revision, aggregate-byte, and per-record ceilings.
    pub(in crate::resident) observation_limits: OperationObservationRecoveryLimits,
    /// Checkpoint ancestry count, byte, and admission ceilings.
    pub(in crate::resident) checkpoint_limits: OperationCheckpointAncestryLimits,
    /// Native accumulator membership ceilings for the selected observation.
    pub(in crate::resident) membership_limits: AccumulatorMembershipLimits,
}

/// Existing historical-owner callbacks used by protected recovery.
///
/// These callbacks admit retained evidence. They do not authenticate the live
/// client or grant current observation or cancellation authority.
pub struct OperationControlRecoveryAdmission<'a> {
    /// Admit each original request against authority at its issuance.
    pub admit_request: &'a mut RequestAdmission<'a>,
    /// Admit each recovered handle against its historical authority and parent.
    pub admit_handle_authority: &'a mut HandleAuthorityAdmission<'a>,
    /// Verify each recovered handle signature.
    pub verify_handle_signature: &'a mut HandleSignatureVerification<'a>,
    /// Admit checkpoints encountered while reading observation ancestry.
    pub admit_checkpoint: &'a mut CheckpointAdmission<'a>,
    /// Resolve the signer that owned each historical observation.
    pub resolve_observation_signer: &'a mut ObservationSignerResolution<'a>,
    /// Admit owner-specific evidence referenced by each observation state.
    pub admit_observation_state: &'a mut ObservationStateAdmission<'a>,
    /// Admit any historical runtime-authority rotation.
    pub admit_rotation: &'a mut AuthorityRotationAdmission<'a>,
    /// Verify each historical observation signature.
    pub verify_observation_signature: &'a mut ObservationSignatureVerification<'a>,
    /// Verify current and historical checkpoint signatures.
    pub verify_checkpoint_signature: &'a mut CheckpointSignatureVerification<'a>,
}

/// Protected operation identity and observation history from one store.
///
/// Recovery grants no live disclosure or cancellation authority. Call
/// [`Self::authorize_observation`] or [`Self::authorize_cancellation`] with a
/// fresh, contextually admitted control request before using the retained facts.
pub struct RecoveredOperationControl {
    operation: RecoveredLocalOperation,
    observation: RecoveredOperationObservation,
}

impl RecoveredOperationControl {
    /// Borrow the protected operation identity and original request.
    #[must_use]
    pub const fn operation(&self) -> &RecoveredLocalOperation {
        &self.operation
    }

    /// Borrow the admitted observation member and current anchored head.
    #[must_use]
    pub const fn observation(&self) -> &RecoveredOperationObservation {
        &self.observation
    }

    /// Consume this protected recovery after its final process-level checks.
    ///
    /// This moves the authenticated history into one semantic continuation. It
    /// does not renew authority or repeat recovery.
    pub(in crate::resident) fn into_parts(
        self,
    ) -> (RecoveredLocalOperation, RecoveredOperationObservation) {
        (self.operation, self.observation)
    }

    /// Select the immediate predecessor retained by the original protected read.
    ///
    /// This is available only for a selected cancellation head. It performs no
    /// second custody, checkpoint, anchor, handle, lease, or start read.
    pub(in crate::resident) fn select_immediate_predecessor(
        self,
    ) -> Result<Self, OperationControlRecoveryError> {
        Ok(Self {
            operation: self.operation,
            observation: self.observation.into_immediate_predecessor_selection()?,
        })
    }

    /// Establish current read authority for this recovered observation.
    ///
    /// # Errors
    ///
    /// Returns the existing control-admission error when request identity,
    /// timing, current head, or caller authority differs.
    pub fn authorize_observation<'a, Authorize>(
        &'a self,
        request: &'a CommonRequestEnvelopeV1,
        authorized_at: UnixTimeV1,
        authorize: Authorize,
    ) -> Result<AuthorizedOperationObservation<'a>, OperationControlError>
    where
        Authorize:
            FnOnce(OperationControlAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
    {
        authorize_operation_observation(
            OperationControlInput {
                request,
                operation: &self.operation,
                observation: &self.observation,
                authorized_at,
            },
            authorize,
        )
    }

    /// Establish current cancellation authority without sending or writing.
    ///
    /// # Errors
    ///
    /// Returns the existing control-admission error when request identity,
    /// timing, current head, or caller authority differs.
    pub fn authorize_cancellation<'a, Authorize>(
        &'a self,
        request: &'a CommonRequestEnvelopeV1,
        authorized_at: UnixTimeV1,
        authorize: Authorize,
    ) -> Result<AuthorizedOperationCancellation<'a>, OperationControlError>
    where
        Authorize:
            FnOnce(OperationControlAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
    {
        authorize_operation_cancellation(
            OperationControlInput {
                request,
                operation: &self.operation,
                observation: &self.observation,
                authorized_at,
            },
            authorize,
        )
    }
}

#[cfg(test)]
#[path = "../../../tests/support/resident/operation_control_recovery.rs"]
mod operation_control_recovery_fixture;

/// Recover one saved operation and an admitted observation from the same store.
///
/// The operation parent chain is recovered first. Observation recovery then
/// uses that exact locator and performs its own bounded checkpoint, membership,
/// and independent-anchor checks. The final association check rejects a
/// substituted store or handle. No write, retry, renewal, disclosure decision,
/// provider call, or cancellation occurs.
///
/// # Errors
///
/// Returns the exact operation or observation recovery failure, or an
/// association error when the two successful reads do not name one operation.
pub fn recover_operation_control(
    input: OperationControlRecoveryInput<'_>,
    operation_port: &mut dyn OperationCustodyPort,
    session_port: &dyn SessionLeaseCustodyPort,
    anchor_port: &mut dyn AntiRollbackAnchorPort,
    admission: OperationControlRecoveryAdmission<'_>,
) -> Result<RecoveredOperationControl, OperationControlRecoveryError> {
    let OperationControlRecoveryInput {
        operation,
        observation,
        observation_limits,
        checkpoint_limits,
        membership_limits,
    } = input;
    let context = operation.context;
    let recovered_operation = recover_local_operation_from_handle(
        operation,
        operation_port,
        session_port,
        anchor_port,
        &mut *admission.admit_request,
        LocalOperationRecoveryAdmission {
            admit_historical_authority: &mut *admission.admit_handle_authority,
            verify_handle_signature: &mut *admission.verify_handle_signature,
            verify_checkpoint_signature: &mut *admission.verify_checkpoint_signature,
            initial_observation: OperationObservationRecoveryAdmission {
                admit_checkpoint: &mut *admission.admit_checkpoint,
                resolve_signer: &mut *admission.resolve_observation_signer,
                admit_state: &mut *admission.admit_observation_state,
                admit_rotation: &mut *admission.admit_rotation,
                verify_signature: &mut *admission.verify_observation_signature,
            },
        },
    )?;
    let recovered_observation = recover_operation_observation(
        OperationObservationRecoveryInput {
            store: context.store,
            operation: recovered_operation.operation(),
            observation,
            checkpoint_authority: context.checkpoint_authority,
            limits: observation_limits,
            checkpoint_limits,
            membership_limits,
        },
        operation_port,
        anchor_port,
        OperationObservationRecoveryAdmission {
            admit_checkpoint: &mut *admission.admit_checkpoint,
            resolve_signer: &mut *admission.resolve_observation_signer,
            admit_state: &mut *admission.admit_observation_state,
            admit_rotation: &mut *admission.admit_rotation,
            verify_signature: &mut *admission.verify_observation_signature,
        },
        &mut *admission.verify_checkpoint_signature,
    )?;
    let recovered_handle = recovered_operation.operation().handle();
    if recovered_observation.store() != recovered_operation.store()
        || recovered_observation.handle.record_ref() != recovered_handle.0
        || recovered_observation.handle.record_sha256().as_str()
            != recovered_handle
                .1
                .canonical_operation_handle_sha256()
                .as_str()
    {
        return Err(OperationControlRecoveryError::Association);
    }
    Ok(RecoveredOperationControl {
        operation: recovered_operation,
        observation: recovered_observation,
    })
}

/// Recover one child and current observation through the existing pairwise proof.
///
/// The protected parent is supplied by the process owner. Child recovery proves
/// the exact parent reference, pairwise membership, common generation, session,
/// activation fence and narrowed lease before observation history is read. This
/// function creates no Consult membership and grants no current authority.
pub(in crate::resident) fn recover_child_operation_control(
    input: ChildOperationControlRecoveryInput<'_>,
    operation_port: &mut dyn OperationCustodyPort,
    session_port: &dyn SessionLeaseCustodyPort,
    anchor_port: &mut dyn AntiRollbackAnchorPort,
    admission: OperationControlRecoveryAdmission<'_>,
) -> Result<RecoveredOperationControl, OperationControlRecoveryError> {
    let ChildOperationControlRecoveryInput {
        operation,
        observation,
        observation_limits,
        checkpoint_limits,
        membership_limits,
    } = input;
    let context = operation.child.context;
    let recovered_operation = recover_local_child_operation_from_handle(
        operation,
        operation_port,
        session_port,
        anchor_port,
        &mut *admission.admit_request,
        LocalOperationRecoveryAdmission {
            admit_historical_authority: &mut *admission.admit_handle_authority,
            verify_handle_signature: &mut *admission.verify_handle_signature,
            verify_checkpoint_signature: &mut *admission.verify_checkpoint_signature,
            initial_observation: OperationObservationRecoveryAdmission {
                admit_checkpoint: &mut *admission.admit_checkpoint,
                resolve_signer: &mut *admission.resolve_observation_signer,
                admit_state: &mut *admission.admit_observation_state,
                admit_rotation: &mut *admission.admit_rotation,
                verify_signature: &mut *admission.verify_observation_signature,
            },
        },
    )?;
    let recovered_observation = recover_operation_observation(
        OperationObservationRecoveryInput {
            store: context.store,
            operation: recovered_operation.operation(),
            observation,
            checkpoint_authority: context.checkpoint_authority,
            limits: observation_limits,
            checkpoint_limits,
            membership_limits,
        },
        operation_port,
        anchor_port,
        OperationObservationRecoveryAdmission {
            admit_checkpoint: &mut *admission.admit_checkpoint,
            resolve_signer: &mut *admission.resolve_observation_signer,
            admit_state: &mut *admission.admit_observation_state,
            admit_rotation: &mut *admission.admit_rotation,
            verify_signature: &mut *admission.verify_observation_signature,
        },
        &mut *admission.verify_checkpoint_signature,
    )?;
    let recovered_handle = recovered_operation.operation().handle();
    if recovered_observation.store() != recovered_operation.store()
        || recovered_observation.handle.record_ref() != recovered_handle.0
        || recovered_observation.handle.record_sha256().as_str()
            != recovered_handle
                .1
                .canonical_operation_handle_sha256()
                .as_str()
    {
        return Err(OperationControlRecoveryError::Association);
    }
    Ok(RecoveredOperationControl {
        operation: recovered_operation,
        observation: recovered_observation,
    })
}

/// Failure before a recovered operation-control view exists.
#[derive(Debug, thiserror::Error)]
pub enum OperationControlRecoveryError {
    /// Saved-handle or parent-chain recovery failed.
    #[error(transparent)]
    Operation(#[from] LocalOperationRecoveryError),
    /// Observation history or current-head recovery failed.
    #[error(transparent)]
    Observation(#[from] OperationObservationReadmissionError),
    /// Successful reads did not identify one store and operation handle.
    #[error("operation control recovery inputs do not identify one operation")]
    Association,
}
