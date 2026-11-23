//! Checks the finite semilattice laws for session verification requirements.

use super::VerificationRequirement::{ProtectedConfiguration, Standard};

#[test]
fn strongest_is_idempotent_commutative_and_associative() {
    let domain = [Standard, ProtectedConfiguration];
    for left in domain {
        assert_eq!(left.strongest(left), left);
        for right in domain {
            assert_eq!(left.strongest(right), right.strongest(left));
            for third in domain {
                assert_eq!(
                    left.strongest(right).strongest(third),
                    left.strongest(right.strongest(third))
                );
            }
        }
    }
}
