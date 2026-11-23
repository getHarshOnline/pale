//! Test-only semantic-finalization substitutions.

use super::*;

impl OwnedRecoveredProviderUnknownTerminal {
    fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }
}

impl PreparedRecoveredProviderUnknown {
    pub(in crate::resident) fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.terminal.observation()
    }
}

impl IssuedRecoveredProviderUnknownCheckpoint {
    pub(in crate::resident) const fn checkpoint_value(&self) -> &ControlStateCheckpointV1 {
        &self.checkpoint_value
    }

    pub(in crate::resident) fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.prepared.observation()
    }
}

impl ProviderOperationRecordingFinalization {
    pub(in crate::resident) fn substitute_expected_store_for_test(
        mut self,
        store: AuthorityStoreCoordinate,
    ) -> Self {
        self.finalizer.expected_store = store;
        self
    }

    pub(in crate::resident) fn restore_expected_store_for_test(mut self) -> Self {
        self.finalizer.expected_store = self.custody.candidate().store().clone();
        self
    }
}
