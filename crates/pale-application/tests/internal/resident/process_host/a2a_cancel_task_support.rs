//! Hostile support for routed `CancelTask` terminal release.

use super::*;

impl<'store, WriterGuard> ResidentProcessHost<'store, WriterGuard> {
    pub(in crate::resident::process_host) fn reject_a2a_cancel_task_aggregate_release_with_replacement_for_test<
        Authority,
    >(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessRetainedA2aCancelTaskOutcome,
    ) -> ResidentProcessA2aCancelTaskReleaseFailure {
        let ResidentProcessRetainedA2aCancelTaskOutcome { route, lower } = pending;
        assert!(self.a2a_task_routes.owns_custody(&route));
        let failure = self
            .with_selected_operation_service(
                &route.selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.reject_a2a_cancel_task_release_with_replacement_for_test(*lower)
                },
            )
            .expect("the exact CancelTask deployment remains selected");
        ResidentProcessA2aCancelTaskReleaseFailure::Lower {
            route,
            lower: Box::new(failure),
        }
    }

    pub(in crate::resident::process_host) fn release_a2a_cancel_task_aggregate_without_route_for_test<
        Authority,
    >(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessRetainedA2aCancelTaskOutcome,
    ) -> (
        ResidentProcessA2aTaskRouteCustody,
        ResidentA2aCancelTaskDurableCustody,
    ) {
        let ResidentProcessRetainedA2aCancelTaskOutcome { route, lower } = pending;
        assert!(self.a2a_task_routes.owns_custody(&route));
        let custody = self
            .with_selected_operation_service(
                &route.selector,
                access,
                authority,
                |_application, service, _sessions| service.finish_a2a_cancel_task(*lower),
            )
            .expect("the exact CancelTask deployment remains selected")
            .unwrap_or_else(|_| panic!("exact aggregate capacity release"));
        (route, custody)
    }
}
