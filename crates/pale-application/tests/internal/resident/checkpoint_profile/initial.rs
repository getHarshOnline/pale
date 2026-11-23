//! Test-only access to initial operation-checkpoint leaves.

use super::*;

impl InitialOperationLeaves {
    pub(in crate::resident) fn replace_session_basis_for_test(
        &mut self,
        basis: &OperationSessionBasis,
    ) {
        self.session_basis.replace_basis_for_test(basis);
    }
}
