//! Test-only mutations for Consult recovery association refusals.

use super::*;

impl ResidentProcessConsultationPolicyHandoff {
    pub(in crate::resident) fn replace_started_selector_for_test(
        &mut self,
        selector: ResidentDeploymentAuthoritySelector,
    ) {
        self.started.replace_selector_for_test(selector);
    }

    pub(in crate::resident) fn replace_started_store_for_test(
        &mut self,
        store: AuthorityStoreCoordinate,
    ) {
        self.started.replace_store_for_test(store);
    }

    pub(in crate::resident) fn replace_frontier_for_test(
        &mut self,
        frontier: Box<ResidentConsultationPolicyFrontier>,
    ) {
        self.frontier = frontier;
    }
}

#[test]
fn skipped_owners_remain_explicit() {
    assert_ne!(
        ResidentProcessConsultRecoveryOtherOwner::Invoke,
        ResidentProcessConsultRecoveryOtherOwner::Child
    );
}
