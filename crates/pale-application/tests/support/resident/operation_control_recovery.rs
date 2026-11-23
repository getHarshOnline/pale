//! Recovered operation-control values used by resident behavior tests.

use super::*;

impl RecoveredOperationControl {
    pub(in crate::resident) const fn fixture(
        operation: RecoveredLocalOperation,
        observation: RecoveredOperationObservation,
    ) -> Self {
        Self {
            operation,
            observation,
        }
    }
}
