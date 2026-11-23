//! Inspect preparation bytes before custody in resident tests.

use super::{IssuedLocalOperation, PreparedLocalOperationStart};
use crate::resident::consultation::ConsultationStartIntent;
use pale_core::records::{OperationHandleV1, ResidentRecordRefV1};
use pale_ports::OperationCustodyRecord;

impl IssuedLocalOperation {
    /// Clone an issued locator only for test composition.
    pub(in crate::resident) fn fixture_clone(&self) -> Self {
        Self {
            checkpoint_selection: self.checkpoint_selection.clone(),
            request_coordinate: self.request_coordinate.clone(),
            handle_ref: self.handle_ref.clone(),
            handle: self.handle.clone(),
            initial_observation_ref: self.initial_observation_ref.clone(),
            initial_observation: self.initial_observation.clone(),
            retained_session: self.retained_session.clone(),
            operation_lease: self.operation_lease.clone(),
            publication_policy: self.publication_policy.clone(),
            consultation_start_intent: self.consultation_start_intent.clone(),
        }
    }
}

impl PreparedLocalOperationStart {
    /// Borrow signed preparation bytes for before-and-after custody assertions.
    /// These bytes carry no issued application locator or dispatch permission.
    #[must_use]
    pub(in crate::resident) fn records(&self) -> &[OperationCustodyRecord] {
        &self.records
    }

    /// Borrow the signed locator before custody for association-only tests.
    /// This does not construct an issued operation or grant continuation.
    #[must_use]
    pub(in crate::resident) fn prepared_handle(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationHandleV1) {
        (&self.handle_ref, &self.handle)
    }

    /// Replace only the private Consult intent before its checkpoint is prepared.
    pub(in crate::resident) fn with_test_consultation_start_intent(
        mut self,
        intent: ConsultationStartIntent,
    ) -> Self {
        intent
            .validate_signed_parent(&self.store, &self.handle_ref, &self.handle)
            .expect("test Consult intent matches the signed parent");
        let basis = super::session_basis::test_support::with_test_consultation_start_intent(
            &self.session_basis,
            intent.clone(),
        );
        self.leaves.replace_session_basis_for_test(&basis);
        self.session_basis = basis;
        self.consultation_start_intent = Some(intent);
        self
    }
}
