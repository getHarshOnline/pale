//! Test-only mutation of an initial operation session-basis leaf.

use super::*;

impl SessionBasisLeaf {
    pub(in crate::resident) fn replace_basis_for_test(&mut self, basis: &OperationSessionBasis) {
        assert_eq!(&self.handle, basis.handle());
        assert_eq!(&self.request, basis.request());
        assert_eq!(&self.session_lease_digest, basis.session_lease_digest());
        self.basis_sha256 = basis.basis_sha256().clone();
    }
}
