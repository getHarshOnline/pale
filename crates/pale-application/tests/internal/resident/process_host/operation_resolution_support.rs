//! Test-only access to completed operation-context custody.

use super::*;

impl<'a> CompletedResidentProcessOperationContext<'a> {
    pub(in crate::resident) fn fixture(
        binding: CompletedOperationContextBinding<'a>,
        checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
        custody: CompletedOperationObservationCustody,
    ) -> Self {
        Self {
            binding,
            checkpoint,
            custody,
        }
    }

    pub(in crate::resident) const fn binding(&self) -> &CompletedOperationContextBinding<'a> {
        &self.binding
    }
}
