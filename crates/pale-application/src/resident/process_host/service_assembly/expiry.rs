//! Select retained preparations for expiry without releasing saved hook output.

use super::{restart, OperationActivePhaseV1, OperationObservationStateV1, RetainedInvokePhase};
use crate::resident::operation_dispatch::hook_response::recovery::RecoveredHookResponsePhase;

impl RetainedInvokePhase {
    /// A saved hook preparation may expire, but cannot resume ordinary dispatch.
    /// Cancellation and dispatched history wait for their reconciliation owners.
    pub(super) fn native_hook_expiry_start(
        &self,
    ) -> Option<&restart::ResidentProcessRecoveredOperationStart> {
        let Self::Deferred(disposition) = self else {
            return None;
        };
        let restart::ResidentProcessOperationRestartDisposition::NativeHookResponse(response) =
            disposition.as_ref()
        else {
            return None;
        };
        (response.response.phase == RecoveredHookResponsePhase::Prepared
            && matches!(
                response
                    .start
                    .control
                    .observation()
                    .selected_head()
                    .1
                    .state(),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::DispatchPrepared { .. }
                }
            ))
        .then_some(&response.start)
    }

    pub(super) fn pre_dispatch_expiry_start(
        &self,
    ) -> Option<&restart::ResidentProcessRecoveredOperationStart> {
        match self {
            Self::OperationResolution(recovered) => Some(&recovered.start),
            Self::ProviderResolved(recovered) => Some(&recovered.start),
            Self::ProviderPrepared(recovered) | Self::NativePrepared(recovered) => {
                Some(&recovered.start)
            }
            _ => self.native_hook_expiry_start(),
        }
    }
}
