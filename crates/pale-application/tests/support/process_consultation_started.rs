//! Test-only construction of a consultation start phase.

use super::*;

impl ResidentProcessConsultationStarted {
    pub(in crate::resident::process_host) fn fixture(
        selector: ResidentDeploymentAuthoritySelector,
        store: AuthorityStoreCoordinate,
        parent: crate::resident::IssuedLocalOperation,
    ) -> Self {
        let maximum_children = parent
            .consultation_start_intent()
            .expect("fixture consultation intent")
            .fresh_child_count();
        Self {
            selector,
            store,
            parent,
            children: Vec::with_capacity(maximum_children),
            maximum_children,
        }
    }

    pub(in crate::resident::process_host) fn replace_selector_for_test(
        &mut self,
        selector: ResidentDeploymentAuthoritySelector,
    ) {
        self.selector = selector;
    }

    pub(in crate::resident::process_host) fn replace_store_for_test(
        &mut self,
        store: AuthorityStoreCoordinate,
    ) {
        self.store = store;
    }
}
