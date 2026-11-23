//! Test-only inspection of the two retained Delegate recovery owners.

use super::*;

pub(in crate::resident::process_host) const fn owners<Provider, A2a>(
    owner: &ResidentA2aDelegateRecoveryOwner<Provider, A2a>,
) -> (&Provider, &A2a) {
    (&owner.provider, &owner.a2a)
}
