//! Test-only access to local-cancellation recovery phases.

use super::*;

impl ResidentProcessPreparedLocalCancellation {
    pub(in crate::resident) fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.basis.observation()
    }

    pub(in crate::resident) const fn checkpoint_input(&self) -> &OperationCheckpointInput {
        &self.basis.checkpoint
    }
}

impl ResidentProcessIssuedLocalCancellationCheckpoint {
    pub(in crate::resident) fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.basis.observation()
    }

    pub(in crate::resident) const fn issued_checkpoint(&self) -> &ControlStateCheckpointV1 {
        &self.checkpoint
    }
}

impl ResidentProcessLocalCancellationCustodyPending {
    pub(in crate::resident) const fn failure(&self) -> &OperationAppendFailure {
        &self.failure
    }
}

impl<'store, WriterGuard> ResidentProcessHost<'store, WriterGuard> {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident::process_host) fn continue_prepared_local_cancellation_with_test_attachment<
        CheckpointSign,
        Attach,
    >(
        &self,
        pending: ResidentProcessPreparedLocalCancellation,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign_checkpoint: CheckpointSign,
        attach: Attach,
    ) -> Result<
        CompletedResidentProcessLocalCancellation,
        ResidentProcessPreparedLocalCancellationFailure,
    >
    where
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Attach: for<'prepared> FnOnce(
            crate::resident::PreparedOperationObservationCheckpoint<'prepared>,
            &ResidentRecordRefV1,
            &ControlStateCheckpointV1,
        ) -> Result<
            crate::resident::PreparedOperationObservationCustody,
            crate::resident::OperationObservationCustodyProjectionError,
        >,
    {
        self.continue_prepared_local_cancellation_with_attachment(
            pending,
            selector,
            access,
            resolve_store,
            checkpoint_authority,
            sign_checkpoint,
            attach,
        )
    }
}
