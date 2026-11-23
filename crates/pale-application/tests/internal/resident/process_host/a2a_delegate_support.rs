//! Test-only fault injection for process-host A2A retention.

use super::*;
use pale_core::records::{ControlStateCheckpointV1, ResidentRecordRefV1};

pub(in crate::resident::process_host) fn cross_a2a_retention_failures_for_test<'a>(
    first: ResidentProcessA2aDelegateRetentionFailure<'a>,
    second: ResidentProcessA2aDelegateRetentionFailure<'a>,
) -> (
    ResidentProcessA2aDelegateRetentionFailure<'a>,
    ResidentProcessA2aDelegateRetentionFailure<'a>,
) {
    let ResidentProcessA2aDelegateRetentionFailure::Retention {
        route: first_route,
        key: first_key,
        kind: first_kind,
        finalization: first_finalization,
        failure: first_failure,
        reason: first_reason,
    } = first
    else {
        panic!("the first fixture must retain physical outcome custody")
    };
    let ResidentProcessA2aDelegateRetentionFailure::Retention {
        route: second_route,
        key: second_key,
        kind: second_kind,
        finalization: second_finalization,
        failure: second_failure,
        reason: second_reason,
    } = second
    else {
        panic!("the second fixture must retain physical outcome custody")
    };
    assert_eq!(first_kind, second_kind);
    (
        ResidentProcessA2aDelegateRetentionFailure::Retention {
            route: first_route,
            key: first_key,
            kind: first_kind,
            finalization: first_finalization,
            failure: second_failure,
            reason: first_reason,
        },
        ResidentProcessA2aDelegateRetentionFailure::Retention {
            route: second_route,
            key: second_key,
            kind: second_kind,
            finalization: second_finalization,
            failure: first_failure,
            reason: second_reason,
        },
    )
}

pub(in crate::resident::process_host) fn cross_a2a_retained_outcomes_for_test<'first, 'second>(
    first: ResidentProcessA2aDelegateRetentionFailure<'first>,
    second: ResidentProcessA2aDelegateRetentionFailure<'second>,
) -> (
    ResidentProcessA2aDelegateRetentionFailure<'second>,
    ResidentProcessA2aDelegateRetentionFailure<'first>,
) {
    let ResidentProcessA2aDelegateRetentionFailure::Route {
        route: first_route,
        key: first_key,
        kind: first_kind,
        retained: first_retained,
    } = first
    else {
        panic!("the first fixture must retain a completed outcome")
    };
    let ResidentProcessA2aDelegateRetentionFailure::Route {
        route: second_route,
        key: second_key,
        kind: second_kind,
        retained: second_retained,
    } = second
    else {
        panic!("the second fixture must retain a completed outcome")
    };
    assert_eq!(first_kind, second_kind);
    (
        ResidentProcessA2aDelegateRetentionFailure::Route {
            route: first_route,
            key: first_key,
            kind: first_kind,
            retained: second_retained,
        },
        ResidentProcessA2aDelegateRetentionFailure::Route {
            route: second_route,
            key: second_key,
            kind: second_kind,
            retained: first_retained,
        },
    )
}

impl<'store, WriterGuard> ResidentProcessHost<'store, WriterGuard> {
    pub(in crate::resident::process_host) fn a2a_send_checkpoint_for_test<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        key: &ResidentOperationTaskKey,
    ) -> (ResidentRecordRefV1, ControlStateCheckpointV1) {
        let selector = self
            .a2a_delegate_routes
            .selector(key)
            .expect("the fixture needs one active A2A route")
            .clone();
        self.with_selected_operation_service(
            &selector,
            access,
            authority,
            |_application, service, _sessions| {
                service
                    .active_a2a_send_checkpoint_for_test(key)
                    .expect("the fixture needs one active A2A send")
            },
        )
        .expect("the fixture route and access must select one deployment")
    }

    pub(in crate::resident::process_host) fn recover_unresolved_a2a_delegate_with_rejected_loss_for_test<
        'operation,
        'call,
        'recovery,
        Authority,
    >(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aDelegateLaunchFailure<'operation, 'call>,
        input: A2aSendRecoveryInput<'recovery>,
        admission: A2aSendRecoveryAdmission<'_>,
    ) -> ResidentProcessA2aDelegateRecoveryFailure<'operation, 'call, 'recovery> {
        let ResidentProcessA2aDelegateLaunchFailure::Custody {
            key,
            selector,
            failure,
        } = pending
        else {
            panic!("the fixture requires unresolved send custody")
        };
        let ResidentA2aDelegateLaunchFailure::Custody(unresolved) = *failure else {
            panic!("the routed custody must retain lower unresolved custody")
        };
        assert!(self.validate_active_a2a_route(&key, &selector, access.store));
        let rejected = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.recover_unresolved_a2a_delegate_with_rejected_loss_for_test(
                        *unresolved,
                        input,
                        admission,
                    )
                },
            )
            .expect("the fixture route and access must select one deployment");
        ResidentProcessA2aDelegateRecoveryFailure::Loss {
            key,
            selector,
            failure: Box::new(
                ResidentA2aDelegateUnresolvedRecoveryFailure::LossReservation(Box::new(rejected)),
            ),
        }
    }

    pub(in crate::resident::process_host) fn a2a_started_association_failure_for_test<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        key: ResidentOperationTaskKey,
    ) -> ResidentProcessA2aDelegateLaunchFailure<'static, 'static> {
        let selector = self
            .a2a_delegate_routes
            .selector(&key)
            .expect("the fixture needs one active process route")
            .clone();
        let failure = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.a2a_started_association_failure_for_test(&key)
                },
            )
            .expect("the fixture route and access must select one deployment");
        routed_launch_failure(key, selector, failure)
    }

    pub(in crate::resident::process_host) fn a2a_offered_completion_failure_for_test<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        key: ResidentOperationTaskKey,
    ) -> ResidentProcessA2aDelegateLaunchFailure<'static, 'static> {
        let selector = self
            .a2a_delegate_routes
            .selector(&key)
            .expect("the fixture needs one active process route")
            .clone();
        let failure = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.a2a_offered_completion_failure_for_test(&key)
                },
            )
            .expect("the fixture route and access must select one deployment");
        routed_launch_failure(key, selector, failure)
    }

    pub(in crate::resident::process_host) fn a2a_rollback_failure_for_test<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        completed: ResidentProcessA2aDelegateCompletion,
    ) -> ResidentProcessA2aDelegateLaunchFailure<'static, 'static> {
        let ResidentProcessA2aDelegateCompletion { route, completion } = completed;
        let selector = route.selector.clone();
        let (key, failure) = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.a2a_rollback_failure_from_not_started_for_test(completion)
                },
            )
            .expect("the fixture route and access must select one deployment");
        assert_eq!(key, route.key);
        assert!(route
            .owner_instance
            .is_same(&self.a2a_delegate_routes.owner_instance));
        self.a2a_delegate_routes.checked_out -= 1;
        self.a2a_delegate_routes
            .active
            .push(ResidentProcessA2aDelegateRoute {
                key: key.clone(),
                selector: selector.clone(),
            });
        routed_launch_failure(key, selector, failure)
    }

    pub(in crate::resident::process_host) fn set_a2a_reservation_phase_for_test<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        key: &ResidentOperationTaskKey,
        phase: crate::resident::task_reservation::ResidentTaskReservationPhase,
    ) {
        let selector = self
            .a2a_delegate_routes
            .selector(key)
            .expect("the fixture needs one active process route")
            .clone();
        self.with_selected_operation_service(
            &selector,
            access,
            authority,
            |_application, service, _sessions| {
                service.set_a2a_reservation_phase_for_test(key, phase);
            },
        )
        .expect("the fixture route and access must select one deployment");
    }

    pub(in crate::resident::process_host) fn a2a_diagnostic_from_unknown_for_test(
        &mut self,
        completed: ResidentProcessA2aDelegateCompletion,
    ) -> ResidentProcessA2aDelegateDiagnostic {
        let ResidentProcessA2aDelegateCompletion { route, completion } = completed;
        let ResidentA2aDelegateCompletion::OutcomeUnknown {
            outcome,
            finalization,
        } = completion
        else {
            panic!("the diagnostic fixture requires a real uncertain outcome")
        };
        ResidentProcessA2aDelegateDiagnostic {
            route,
            diagnostic: ResidentA2aDelegateDiagnostic::fixture_from_outcome_unknown(
                outcome,
                finalization,
            ),
        }
    }

    pub(in crate::resident::process_host) fn retain_a2a_delegate_with_rejected_finalization_for_test<
        'a,
        Authority,
    >(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aDelegateRetention<'a>,
    ) -> ResidentProcessA2aDelegateRetentionFailure<'static> {
        assert!(pending.process_association().is_ok());
        let route = match &pending {
            ResidentProcessA2aDelegateRetention::Task { route, .. }
            | ResidentProcessA2aDelegateRetention::NoTask { route, .. }
            | ResidentProcessA2aDelegateRetention::NotStarted { route, .. }
            | ResidentProcessA2aDelegateRetention::OutcomeUnknown { route, .. }
            | ResidentProcessA2aDelegateRetention::RecoveredOutcomeUnknown { route, .. } => route,
        };
        assert!(self.validate_checked_out_a2a_route(route, access.store));
        let selector = route.selector.clone();
        let (route, finalization, input) = pending.into_parts();
        let rejection = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    let retained = service
                        .retain_a2a_delegate_outcome(input)
                        .unwrap_or_else(|_| panic!("test fixture must retain its A2A outcome"));
                    service
                        .reject_a2a_finalization_with_replacement_for_test(finalization, retained)
                },
            )
            .expect("the checked-out route and access remain exact");
        ResidentProcessA2aDelegateRetentionFailure::Finalization {
            route,
            rejection: Box::new(rejection),
        }
    }

    pub(in crate::resident::process_host) fn retain_a2a_delegate_with_rejected_route_release_for_test<
        'a,
        Authority,
    >(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aDelegateRetention<'a>,
    ) -> ResidentProcessA2aDelegateRetentionFailure<'static> {
        let (key, kind) = pending
            .process_association()
            .expect("test fixture must retain the process association");
        let route = match &pending {
            ResidentProcessA2aDelegateRetention::Task { route, .. }
            | ResidentProcessA2aDelegateRetention::NoTask { route, .. }
            | ResidentProcessA2aDelegateRetention::NotStarted { route, .. }
            | ResidentProcessA2aDelegateRetention::OutcomeUnknown { route, .. }
            | ResidentProcessA2aDelegateRetention::RecoveredOutcomeUnknown { route, .. } => route,
        };
        assert!(self.validate_checked_out_a2a_route(route, access.store));
        let selector = route.selector.clone();
        let (route, finalization, input) = pending.into_parts();
        let retained = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    let retained = service
                        .retain_a2a_delegate_outcome(input)
                        .unwrap_or_else(|_| panic!("test fixture must retain its A2A outcome"));
                    service
                        .finish_a2a_delegate(finalization, retained)
                        .unwrap_or_else(|_| panic!("test fixture must release aggregate capacity"))
                },
            )
            .expect("the checked-out route and access remain exact");

        let checked_out = self.a2a_delegate_routes.checked_out;
        self.a2a_delegate_routes.checked_out = 0;
        let failure = match self.release_retained_a2a_delegate(route, key, kind, retained) {
            Ok(_) => panic!("the modeled process-route release must fail"),
            Err(failure) => failure,
        };
        self.a2a_delegate_routes.checked_out = checked_out;
        *failure
    }
}
