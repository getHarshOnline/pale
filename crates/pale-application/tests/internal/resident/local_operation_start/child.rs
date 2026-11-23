//! Test-only construction of an already-proved local parent.

use super::*;

impl PreparedLocalChildParent {
    /// Assemble the parent phase used by application tests.
    ///
    /// The tests still issue the child through the real semantic, checkpoint,
    /// database, and anchor phases. Recovery tests prove that these parent
    /// fields can only come from protected current-head recovery.
    pub(in crate::resident) fn fixture(
        parent: &crate::resident::IssuedLocalOperation,
        checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
        transaction: ControlTransactionPreparation,
        observed_at: UnixTimeV1,
    ) -> Self {
        Self {
            selection: parent.checkpoint_selection(),
            checkpoint: (checkpoint.0.clone(), checkpoint.1.clone()),
            transaction,
            reference: parent.handle().0.clone(),
            handle: parent.handle().1.clone(),
            lease: parent.historical_operation_lease().clone(),
            observed_at,
        }
    }
}
