//! Recovered local-operation values used by consultation tests.

use super::*;

impl RecoveredLocalOperation {
    pub(in crate::resident) fn fixture(
        operation: &IssuedLocalOperation,
        request: CommonRequestEnvelopeV1,
        store: pale_ports::AuthorityStoreCoordinate,
    ) -> Self {
        Self {
            operation: operation.fixture_clone(),
            request,
            store,
            initial_observation: None,
            recovered_parent: None,
        }
    }
}
