//! Test-only access to provider-recovery continuations.

use super::*;
use crate::resident::operation_dispatch::provider::execution::continue_prepared_recovered_provider_unknown_with_attachment;
use pale_core::records::OperationObservationV1;

impl ResidentProcessProviderRecoveryPrepared {
    pub(in crate::resident) const fn selection_failure(
        &self,
    ) -> Option<&ResidentProcessHostSelectionError> {
        self.selection_failure.as_ref()
    }

    pub(in crate::resident) const fn recording_failure(&self) -> Option<&OperationAppendFailure> {
        self.pending.failure()
    }

    pub(in crate::resident) fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.pending.observation()
    }
}

impl ResidentProcessProviderRecoveryIssued {
    pub(in crate::resident) const fn selection_failure(
        &self,
    ) -> Option<&ResidentProcessHostSelectionError> {
        self.selection_failure.as_ref()
    }

    pub(in crate::resident) const fn recording_failure(&self) -> Option<&OperationAppendFailure> {
        self.pending.failure()
    }

    pub(in crate::resident) fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.pending.observation()
    }

    pub(in crate::resident) const fn checkpoint_value(&self) -> &ControlStateCheckpointV1 {
        self.pending.checkpoint_value()
    }
}

impl<'store, WriterGuard> ResidentProcessHost<'store, WriterGuard> {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident::process_host) fn continue_prepared_provider_recovery_with_test_attachment<
        CheckpointSign,
        Attach,
    >(
        &self,
        pending: ResidentProcessProviderRecoveryPrepared,
        access: ResidentDeploymentOperationAccess<'_>,
        resolve_store: &dyn crate::resident::OperationObservationAuthorityStoreResolver,
        checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign_checkpoint: CheckpointSign,
        attach: Attach,
    ) -> Result<ProviderOperationInvokeResult, ResidentProcessProviderRecoveryFailure>
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
        let ResidentProcessProviderRecoveryPrepared {
            selector,
            store,
            pending: phase,
            selection_failure: _,
        } = pending;
        if let Err(reason) =
            self.validate_provider_recovery_continuation(&selector, &store, access.store)
        {
            return Err(ResidentProcessProviderRecoveryFailure::Prepared(Box::new(
                ResidentProcessProviderRecoveryPrepared {
                    selector,
                    store,
                    pending: phase,
                    selection_failure: Some(reason),
                },
            )));
        }
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        map_provider_recovery_continuation(
            selector,
            store,
            continue_prepared_recovered_provider_unknown_with_attachment(
                phase,
                resolve_store,
                custody,
                anchor,
                checkpoint_authority,
                sign_checkpoint,
                attach,
            ),
        )
    }
}
