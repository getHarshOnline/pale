//! Test construction for recovered-dispatch preflight boundaries.

use super::*;

impl CompletedRecoveredDispatchCustody {
    pub(in crate::resident) fn from_retained_fixture(
        observation: (ResidentRecordRefV1, OperationObservationV1),
        checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
        custody: crate::resident::CompletedOperationObservationCustody,
    ) -> Self {
        Self {
            observation,
            checkpoint,
            custody,
        }
    }
}
