//! Hostile support for routed `GetTask` aggregate-release custody.

use super::*;

impl<'store, WriterGuard> ResidentProcessHost<'store, WriterGuard> {
    pub(in crate::resident::process_host) fn reject_a2a_get_task_release_with_replacement_for_test<
        Authority,
    >(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessRetainedA2aGetTask,
    ) -> ResidentProcessA2aGetTaskReleaseFailure {
        let ResidentProcessRetainedA2aGetTask { route, retained } = pending;
        assert!(self.a2a_task_routes.owns_custody(&route));
        assert_eq!(route.key.as_ref(), retained.key());
        assert_eq!(&route.key.store, access.store);
        let selector = route.selector.as_ref().clone();
        let failure = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.reject_get_task_release_with_replacement_for_test(*retained)
                },
            )
            .expect("the exact test deployment must remain selected");
        ResidentProcessA2aGetTaskReleaseFailure::Lower { route, failure }
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident::process_host) fn start_a2a_get_task<Authority, Authorize>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: A2aGetTaskReadbackInput<'_>,
        exchange: &dyn A2aExchangePort,
        clock: &dyn WallClockSourcePort,
        authorize: Authorize,
    ) -> Result<(), ResidentProcessA2aGetTaskStartFailure>
    where
        Authorize: FnOnce(
            A2aGetTaskReadbackAuthorizationContext<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        let key = a2a_get_task_key(&input);
        let deployment = match self.deployment_authority_index(selector) {
            Ok(deployment) => deployment,
            Err(reason) => {
                return Err(ResidentProcessA2aGetTaskStartFailure::Selection(reason));
            }
        };
        if self.deployments[deployment].store() != access.store || access.store != &key.store {
            return Err(ResidentProcessA2aGetTaskStartFailure::Selection(
                ResidentProcessHostSelectionError::AccessStoreAssociation,
            ));
        }
        if !selector.matches_operation(input.attempt.operation) {
            return Err(ResidentProcessA2aGetTaskStartFailure::Selection(
                ResidentProcessHostSelectionError::HistoricalHandleAuthority,
            ));
        }
        match self.a2a_task_routes.insert(key.clone(), selector.clone()) {
            Ok(()) => {}
            Err(ResidentProcessA2aTaskRouteInsertError::Conflict) => {
                return Err(ResidentProcessA2aGetTaskStartFailure::RouteConflict);
            }
            Err(ResidentProcessA2aTaskRouteInsertError::Saturated) => {
                return Err(ResidentProcessA2aGetTaskStartFailure::RouteSaturated);
            }
        }
        let started = self
            .with_selected_operation_service(
                selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.start_owned_a2a_get_task(key.clone(), input, exchange, clock, authorize)
                },
            )
            .expect("the deployment and access were fenced before GetTask start");
        match started {
            Ok(()) => Ok(()),
            Err(failure @ ResidentA2aGetTaskStartFailure::Acceptance { .. })
            | Err(failure @ ResidentA2aGetTaskStartFailure::Rollback { .. }) => {
                Err(ResidentProcessA2aGetTaskStartFailure::Unresolved {
                    key: Box::new(key),
                    selector: Box::new(selector.clone()),
                    failure: Box::new(failure),
                })
            }
            Err(ResidentA2aGetTaskStartFailure::Closed(reason)) => {
                let removed = self.a2a_task_routes.remove_active(&key, selector);
                debug_assert!(removed);
                Err(ResidentProcessA2aGetTaskStartFailure::Closed(reason))
            }
            Err(failure) => {
                let removed = self.a2a_task_routes.remove_active(&key, selector);
                debug_assert!(removed);
                Err(ResidentProcessA2aGetTaskStartFailure::Rejected(Box::new(
                    failure,
                )))
            }
        }
    }

    pub(in crate::resident::process_host) fn retry_a2a_get_task_start<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aGetTaskStartFailure,
    ) -> Result<ResidentProcessA2aGetTaskStartRetry, ResidentProcessA2aGetTaskStartFailure> {
        let ResidentProcessA2aGetTaskStartFailure::Unresolved {
            key,
            selector,
            failure,
        } = pending
        else {
            return Err(pending);
        };
        let key = *key;
        let selector = *selector;
        let failure = *failure;
        if key.store != *access.store
            || !self.a2a_task_routes.matches(&key, &selector)
            || failure.key() != Some(&key)
        {
            return Err(ResidentProcessA2aGetTaskStartFailure::Unresolved {
                key: Box::new(key),
                selector: Box::new(selector),
                failure: Box::new(failure),
            });
        }
        match failure {
            failure @ ResidentA2aGetTaskStartFailure::Acceptance { .. } => {
                let retried = self
                    .with_selected_operation_service(
                        &selector,
                        access,
                        authority,
                        |_application, service, _sessions| {
                            service.retry_owned_a2a_get_task_acceptance(failure)
                        },
                    )
                    .expect("the active GetTask route was fenced before acceptance retry");
                match retried {
                    Ok(()) => Ok(ResidentProcessA2aGetTaskStartRetry::Started),
                    Err(failure) => Err(ResidentProcessA2aGetTaskStartFailure::Unresolved {
                        key: Box::new(key),
                        selector: Box::new(selector),
                        failure: Box::new(failure),
                    }),
                }
            }
            failure @ ResidentA2aGetTaskStartFailure::Rollback { .. } => {
                let retried = self
                    .with_selected_operation_service(
                        &selector,
                        access,
                        authority,
                        |_application, service, _sessions| {
                            service.retry_owned_a2a_get_task_start_rollback(failure)
                        },
                    )
                    .expect("the active GetTask route was fenced before rollback retry");
                match retried {
                    Ok(reason) => {
                        let removed = self.a2a_task_routes.remove_active(&key, &selector);
                        debug_assert!(removed);
                        Ok(ResidentProcessA2aGetTaskStartRetry::Closed(reason))
                    }
                    Err(failure) => Err(ResidentProcessA2aGetTaskStartFailure::Unresolved {
                        key: Box::new(key),
                        selector: Box::new(selector),
                        failure: Box::new(failure),
                    }),
                }
            }
            failure => Err(ResidentProcessA2aGetTaskStartFailure::Unresolved {
                key: Box::new(key),
                selector: Box::new(selector),
                failure: Box::new(failure),
            }),
        }
    }

    pub(in crate::resident::process_host) fn stop_a2a_get_task_after_acceptance_failure<
        Authority,
    >(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aGetTaskStartFailure,
    ) -> Result<ResidentProcessA2aGetTaskPoll, ResidentProcessA2aGetTaskStartFailure> {
        let ResidentProcessA2aGetTaskStartFailure::Unresolved {
            key,
            selector,
            failure,
        } = pending
        else {
            return Err(pending);
        };
        let key = *key;
        let selector = *selector;
        let failure = *failure;
        if key.store != *access.store
            || !self.a2a_task_routes.matches(&key, &selector)
            || failure.key() != Some(&key)
        {
            return Err(ResidentProcessA2aGetTaskStartFailure::Unresolved {
                key: Box::new(key),
                selector: Box::new(selector),
                failure: Box::new(failure),
            });
        }
        if !matches!(&failure, ResidentA2aGetTaskStartFailure::Acceptance { .. }) {
            return Err(ResidentProcessA2aGetTaskStartFailure::Unresolved {
                key: Box::new(key),
                selector: Box::new(selector),
                failure: Box::new(failure),
            });
        }
        let stopped = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.stop_owned_a2a_get_task_after_acceptance_failure(failure)
                },
            )
            .expect("the active GetTask route was fenced before local stop");
        match stopped {
            Ok(ResidentA2aGetTaskPoll::ReadStopped(stopped)) => {
                let route = self
                    .a2a_task_routes
                    .check_out(&key)
                    .expect("the active GetTask route must remain present");
                Ok(ResidentProcessA2aGetTaskPoll::ReadStopped(
                    ResidentProcessA2aGetTaskReadStopped { route, stopped },
                ))
            }
            Ok(ResidentA2aGetTaskPoll::AggregateTransitionRejected(rejection)) => {
                Ok(ResidentProcessA2aGetTaskPoll::AggregateTransitionRejected {
                    key: Box::new(key),
                    selector: Box::new(selector),
                    rejection,
                })
            }
            Ok(_) => unreachable!("acceptance stop must terminate or retain aggregate capacity"),
            Err(failure) => Err(ResidentProcessA2aGetTaskStartFailure::Unresolved {
                key: Box::new(key),
                selector: Box::new(selector),
                failure: Box::new(failure),
            }),
        }
    }

    pub(in crate::resident::process_host) fn finish_closed_a2a_get_task<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aGetTaskClosed,
    ) -> Result<A2aGetTaskReadbackError, ResidentProcessA2aGetTaskTerminalReleaseFailure> {
        let ResidentProcessA2aGetTaskClosed { route, closed } = pending;
        if !self.a2a_task_routes.owns_custody(&route)
            || route.key.as_ref() != &closed.key
            || route.key.store != *access.store
        {
            return Err(
                ResidentProcessA2aGetTaskTerminalReleaseFailure::SelectionClosed(
                    ResidentProcessA2aGetTaskClosed { route, closed },
                ),
            );
        }
        let selector = route.selector.as_ref().clone();
        let finished = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| service.finish_closed_a2a_get_task(*closed),
            )
            .expect("the checked-out GetTask route was fenced before closed release");
        match finished {
            Ok(reason) => match self.a2a_task_routes.release(route) {
                Ok(()) => Ok(reason),
                Err(route) => Err(
                    ResidentProcessA2aGetTaskTerminalReleaseFailure::RouteClosed {
                        route,
                        reason: Box::new(reason),
                    },
                ),
            },
            Err(reason) => {
                Err(ResidentProcessA2aGetTaskTerminalReleaseFailure::Closed { route, reason })
            }
        }
    }

    pub(in crate::resident::process_host) fn finish_stopped_a2a_get_task_response_read<
        Authority,
    >(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aGetTaskReadStopped,
    ) -> Result<(), ResidentProcessA2aGetTaskTerminalReleaseFailure> {
        let ResidentProcessA2aGetTaskReadStopped { route, stopped } = pending;
        if !self.a2a_task_routes.owns_custody(&route)
            || route.key.as_ref() != &stopped.key
            || route.key.store != *access.store
        {
            return Err(
                ResidentProcessA2aGetTaskTerminalReleaseFailure::SelectionReadStopped(
                    ResidentProcessA2aGetTaskReadStopped { route, stopped },
                ),
            );
        }
        let selector = route.selector.as_ref().clone();
        let finished = self
            .with_selected_operation_service(
                &selector,
                access,
                authority,
                |_application, service, _sessions| {
                    service.finish_stopped_a2a_get_task_response_read(*stopped)
                },
            )
            .expect("the checked-out GetTask route was fenced before read-stop release");
        match finished {
            Ok(()) => match self.a2a_task_routes.release(route) {
                Ok(()) => Ok(()),
                Err(route) => {
                    Err(ResidentProcessA2aGetTaskTerminalReleaseFailure::RouteReadStopped(route))
                }
            },
            Err(reason) => {
                Err(ResidentProcessA2aGetTaskTerminalReleaseFailure::ReadStopped { route, reason })
            }
        }
    }

    pub(in crate::resident::process_host) fn retry_a2a_get_task_terminal_release<Authority>(
        &mut self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        pending: ResidentProcessA2aGetTaskTerminalReleaseFailure,
    ) -> Result<
        ResidentProcessA2aGetTaskTerminalRelease,
        ResidentProcessA2aGetTaskTerminalReleaseFailure,
    > {
        match pending {
            ResidentProcessA2aGetTaskTerminalReleaseFailure::SelectionClosed(pending) => {
                Err(ResidentProcessA2aGetTaskTerminalReleaseFailure::SelectionClosed(pending))
            }
            ResidentProcessA2aGetTaskTerminalReleaseFailure::SelectionReadStopped(pending) => {
                Err(ResidentProcessA2aGetTaskTerminalReleaseFailure::SelectionReadStopped(pending))
            }
            ResidentProcessA2aGetTaskTerminalReleaseFailure::RouteClosed { route, reason } => {
                if !self.a2a_task_routes.owns_custody(&route) || route.key.store != *access.store {
                    return Err(
                        ResidentProcessA2aGetTaskTerminalReleaseFailure::RouteClosed {
                            route,
                            reason,
                        },
                    );
                }
                match self.a2a_task_routes.release(route) {
                    Ok(()) => Ok(ResidentProcessA2aGetTaskTerminalRelease::Closed(*reason)),
                    Err(route) => Err(
                        ResidentProcessA2aGetTaskTerminalReleaseFailure::RouteClosed {
                            route,
                            reason,
                        },
                    ),
                }
            }
            ResidentProcessA2aGetTaskTerminalReleaseFailure::RouteReadStopped(route) => {
                if !self.a2a_task_routes.owns_custody(&route) || route.key.store != *access.store {
                    return Err(
                        ResidentProcessA2aGetTaskTerminalReleaseFailure::RouteReadStopped(route),
                    );
                }
                match self.a2a_task_routes.release(route) {
                    Ok(()) => Ok(ResidentProcessA2aGetTaskTerminalRelease::ReadStopped),
                    Err(route) => Err(
                        ResidentProcessA2aGetTaskTerminalReleaseFailure::RouteReadStopped(route),
                    ),
                }
            }
            ResidentProcessA2aGetTaskTerminalReleaseFailure::Closed { route, reason } => {
                if !self.a2a_task_routes.owns_custody(&route)
                    || route.key.as_ref() != &reason.closed.key
                    || route.key.store != *access.store
                {
                    return Err(ResidentProcessA2aGetTaskTerminalReleaseFailure::Closed {
                        route,
                        reason,
                    });
                }
                let selector = route.selector.as_ref().clone();
                let retried = self
                    .with_selected_operation_service(
                        &selector,
                        access,
                        authority,
                        |_application, service, _sessions| {
                            service.finish_closed_a2a_get_task(*reason.closed)
                        },
                    )
                    .expect("the checked-out GetTask route was fenced before closed retry");
                match retried {
                    Ok(reason) => match self.a2a_task_routes.release(route) {
                        Ok(()) => Ok(ResidentProcessA2aGetTaskTerminalRelease::Closed(reason)),
                        Err(route) => Err(
                            ResidentProcessA2aGetTaskTerminalReleaseFailure::RouteClosed {
                                route,
                                reason: Box::new(reason),
                            },
                        ),
                    },
                    Err(reason) => Err(ResidentProcessA2aGetTaskTerminalReleaseFailure::Closed {
                        route,
                        reason,
                    }),
                }
            }
            ResidentProcessA2aGetTaskTerminalReleaseFailure::ReadStopped { route, reason } => {
                if !self.a2a_task_routes.owns_custody(&route)
                    || route.key.as_ref() != &reason.stopped.key
                    || route.key.store != *access.store
                {
                    return Err(
                        ResidentProcessA2aGetTaskTerminalReleaseFailure::ReadStopped {
                            route,
                            reason,
                        },
                    );
                }
                let selector = route.selector.as_ref().clone();
                let retried = self
                    .with_selected_operation_service(
                        &selector,
                        access,
                        authority,
                        |_application, service, _sessions| {
                            service.finish_stopped_a2a_get_task_response_read(*reason.stopped)
                        },
                    )
                    .expect("the checked-out GetTask route was fenced before read-stop retry");
                match retried {
                    Ok(()) => match self.a2a_task_routes.release(route) {
                        Ok(()) => Ok(ResidentProcessA2aGetTaskTerminalRelease::ReadStopped),
                        Err(route) => Err(
                            ResidentProcessA2aGetTaskTerminalReleaseFailure::RouteReadStopped(
                                route,
                            ),
                        ),
                    },
                    Err(reason) => Err(
                        ResidentProcessA2aGetTaskTerminalReleaseFailure::ReadStopped {
                            route,
                            reason,
                        },
                    ),
                }
            }
        }
    }
}
