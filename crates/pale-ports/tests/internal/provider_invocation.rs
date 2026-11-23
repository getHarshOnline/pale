//! Proves that provider invocation unavailability preserves its closed distinctions.

use super::*;

#[test]
fn provider_invocation_unavailability_states_are_pairwise_distinct() {
    let states = [
        ProviderInvocationUnavailability::OwnerContractUnavailable,
        ProviderInvocationUnavailability::Down,
        ProviderInvocationUnavailability::Partitioned,
        ProviderInvocationUnavailability::Removed,
        ProviderInvocationUnavailability::EvidenceUnknown,
    ];

    assert_eq!(
        states
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        5
    );
}
