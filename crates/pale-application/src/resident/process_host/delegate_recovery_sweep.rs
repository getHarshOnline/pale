//! One-source cold-start recovery for committed pairwise Delegate children.
//!
//! One turn protects one retained start, proves its exact immediate parent from
//! the bounded recovered handle chain, and hands the child to the owner of its
//! retained phase. It never runs, accepts, appends to, or settles the parent.

use super::*;
use crate::resident::local_operation_start::{
    discover_local_operation_start, LocalOperationStartDiscoveryError,
    LocalOperationStartDiscoveryTurn,
};
use crate::resident::process_host::operation_recovery_sweep::{
    admit_discovered_start, ResidentProcessOperationRecoverySourceError,
    ResidentProcessRootProviderRecoveryOwner,
};
use crate::resident::process_host::restart::{
    ResidentProcessOperationLineage, ResidentProcessRecoveredA2aCancelTaskRequest,
    ResidentProcessRecoveredA2aTask, ResidentProcessRecoveredOperationResolution,
    ResidentProcessRecoveredOperationStart,
};

/// Exact A2A phase owners added to the shared provider-recovery owner.
///
/// Each method receives a phase already admitted against the Delegate's exact
/// parent. Returning success means the existing owner has durably closed the
/// phase or retained its exact successor. None may replay `SendMessage`.
pub(super) trait ResidentProcessDelegateRecoveryOwner<WriterGuard>:
    ResidentProcessRootProviderRecoveryOwner<WriterGuard>
{
    /// Continue one point-addressed pairwise Delegate before dispatch.
    ///
    /// Unlike root-Invoke recovery, this method receives the current admitted
    /// Delegate facts. It must not select or issue a route, and the input gives
    /// it no reusable carrier with which to replay `SendMessage`.
    fn recover_delegate_operation_resolution<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredOperationResolution>,
        access: ResidentDeploymentOperationAccess<'_>,
        input: super::program::ResidentProgramDelegateResolutionInput<'_, '_>,
    ) -> Result<Self::Output, Self::Error>;

    /// Continue one admitted A2A send whose remote Task is not yet identified.
    fn recover_a2a_pre_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredAttempt>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error>;

    /// Continue observation of one exact already-identified remote Task.
    fn recover_a2a_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredA2aTask>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error>;

    /// Continue read-back for one exact retained remote cancellation request.
    fn recover_a2a_cancel_task<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredA2aCancelTaskRequest>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error>;
}

/// Another exact recovery owner selected before Delegate lineage admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResidentProcessDelegateRecoveryOtherOwner {
    /// An unparented Invoke or Consult belongs to a root recovery sweep.
    Root,
    /// A parented non-Delegate belongs to its consultation parent owner.
    ConsultChild,
}

/// One closed or exactly handed-off Delegate recovery turn.
#[must_use = "preserve the reported or handed-off Delegate phase"]
pub(super) enum ResidentProcessDelegateRecoveryTurn<Output> {
    /// Every source in this deployment's frozen prefix has been processed.
    Exhausted,
    /// This source belongs to another exact recovery owner.
    SkippedOtherOwner(ResidentProcessDelegateRecoveryOtherOwner),
    /// The child already has a terminal observation; the parent is unchanged.
    ReportedTerminal(Box<ResidentProcessRecoveredOperationStart>),
    /// Initial dispatch needs an admitted point-addressed session turn.
    ///
    /// Background recovery retains this exact phase but cannot construct the
    /// caller and generation evidence required to send the Delegate request.
    DeferredResolution(Box<ResidentProcessRecoveredOperationResolution>),
    /// Only a separately retained recovery owner can advance this child.
    ReportedRecoveryOnly(Box<ResidentProcessRecoveredOperationStart>),
    /// The existing phase owner accepted the exact child handoff.
    Scheduled(Output),
}

/// Why one Delegate recovery turn could not reach an exact handoff.
#[derive(Debug)]
pub(super) enum ResidentProcessDelegateRecoveryError<RouteError> {
    /// Bounded physical discovery failed. The Delegate cursor is unchanged.
    Discovery(LocalOperationStartDiscoveryError),
    /// The request, private basis, handle, or retained start differed.
    Source(ResidentProcessOperationRecoverySourceError),
    /// Protected restart admission or pairwise lineage proof failed.
    Restart(ResidentProcessOperationRestartError),
    /// The protected child and its classified phase contradict Delegate state.
    State(ResidentProcessDelegateRecoveryStateError),
    /// The exact phase owner refused or retained unfinished work.
    Route(RouteError),
}

/// Closed state contradictions that cannot be handed to another owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResidentProcessDelegateRecoveryStateError {
    /// A Delegate handle had no parent coordinate.
    UnparentedDelegate,
    /// Pairwise Delegate admission produced a consultation-only phase.
    ConsultDisposition,
    /// Pairwise admission did not retain the exact child lineage witness.
    PairwiseLineage,
}

impl<'store, WriterGuard> ResidentProcessHost<'store, WriterGuard> {
    /// Recover and route at most one committed pairwise Delegate child.
    ///
    /// Discovery, parent-chain recovery, phase classification and handoff are
    /// bounded. An error leaves the same source selected for a later turn.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn recover_one_delegate_start<Owner>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        restart: ResidentProcessOperationRestartInput<'_>,
        mut admission: OperationControlRecoveryAdmission<'_>,
        maximum_session_basis_bytes: usize,
        owner: &mut Owner,
    ) -> Result<
        ResidentProcessDelegateRecoveryTurn<Owner::Output>,
        ResidentProcessDelegateRecoveryError<Owner::Error>,
    >
    where
        Owner: ResidentProcessDelegateRecoveryOwner<WriterGuard>,
    {
        let index = self.deployment_authority_index(selector).map_err(|error| {
            ResidentProcessDelegateRecoveryError::Restart(
                ResidentProcessOperationRestartError::Selection(error),
            )
        })?;
        if self.deployments[index].store() != access.store {
            return Err(ResidentProcessDelegateRecoveryError::Restart(
                ResidentProcessHostSelectionError::AccessStoreAssociation.into(),
            ));
        }

        let attempt = self.delegate_recovery_attempts[index].clone();
        let operation_context = restart.operation;
        let maximum_record_bytes = operation_context.maximum_record_bytes;
        let ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions,
            anchor,
        } = access;
        let discovered =
            discover_local_operation_start(attempt, &mut *custody, maximum_session_basis_bytes)
                .map_err(ResidentProcessDelegateRecoveryError::Discovery)?;
        let pending = match discovered {
            LocalOperationStartDiscoveryTurn::Exhausted(exhausted) => {
                self.delegate_recovery_attempts[index] = exhausted;
                return Ok(ResidentProcessDelegateRecoveryTurn::Exhausted);
            }
            LocalOperationStartDiscoveryTurn::Pending(pending) => pending,
        };

        let (request, evidence) = admit_discovered_start(
            store,
            pending.source(),
            &mut *custody,
            maximum_record_bytes,
            &mut *admission.admit_request,
        )
        .map_err(ResidentProcessDelegateRecoveryError::Source)?;
        let request_preimage = request.to_preimage();
        let association = ResidentOperationControlAssociation {
            current_request_authority: &request_preimage.signing_authority_binding,
            historical_handle_authority: operation_context.handle_signing_authority,
            recovery_store: operation_context.store,
            checkpoint_authority: operation_context.checkpoint_authority,
        };
        let protected = self
            .protect_existing_operation_start(
                ResidentProcessExistingOperationStart {
                    selector: selector.clone(),
                    evidence: Box::new(evidence),
                },
                association,
                ResidentDeploymentOperationAccess {
                    store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                &restart,
                &mut admission,
            )
            .map_err(ResidentProcessDelegateRecoveryError::Restart)?;

        match (
            protected.operation_class(),
            protected.parent_operation().is_some(),
        ) {
            (OperationClassV1::Delegate, false) => {
                return Err(ResidentProcessDelegateRecoveryError::State(
                    ResidentProcessDelegateRecoveryStateError::UnparentedDelegate,
                ));
            }
            (OperationClassV1::Delegate, true) => {}
            (_, false) => {
                self.delegate_recovery_attempts[index] = pending.advance();
                return Ok(ResidentProcessDelegateRecoveryTurn::SkippedOtherOwner(
                    ResidentProcessDelegateRecoveryOtherOwner::Root,
                ));
            }
            (_, true) => {
                self.delegate_recovery_attempts[index] = pending.advance();
                return Ok(ResidentProcessDelegateRecoveryTurn::SkippedOtherOwner(
                    ResidentProcessDelegateRecoveryOtherOwner::ConsultChild,
                ));
            }
        }

        let disposition = self
            .classify_protected_delegate_child_operation(
                protected,
                ResidentDeploymentOperationAccess {
                    store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
                restart,
                &mut admission,
            )
            .map_err(ResidentProcessDelegateRecoveryError::Restart)?;

        let turn = self.route_delegate_restart_disposition(
            disposition,
            ResidentDeploymentOperationAccess {
                store,
                custody: &mut *custody,
                sessions,
                anchor: &mut *anchor,
            },
            owner,
        )?;

        self.delegate_recovery_attempts[index] = pending.advance();
        Ok(turn)
    }

    /// Hand one admitted Delegate restart disposition to its exact phase owner.
    ///
    /// This seam has no discovery cursor. Its caller advances a cursor only
    /// after this method returns one closed report or durable owner handoff.
    pub(super) fn route_delegate_restart_disposition<Owner>(
        &mut self,
        disposition: ResidentProcessOperationRestartDisposition,
        access: ResidentDeploymentOperationAccess<'_>,
        owner: &mut Owner,
    ) -> Result<
        ResidentProcessDelegateRecoveryTurn<Owner::Output>,
        ResidentProcessDelegateRecoveryError<Owner::Error>,
    >
    where
        Owner: ResidentProcessDelegateRecoveryOwner<WriterGuard>,
    {
        let ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions,
            anchor,
        } = access;
        let turn = match disposition {
            ResidentProcessOperationRestartDisposition::NativeHookResponse(_) => {
                return Err(ResidentProcessDelegateRecoveryError::Restart(
                    ResidentProcessOperationRestartError::State,
                ));
            }
            ResidentProcessOperationRestartDisposition::Terminal(start) => {
                ResidentProcessDelegateRecoveryTurn::ReportedTerminal(start)
            }
            ResidentProcessOperationRestartDisposition::RecoveryOnly(start) => {
                ResidentProcessDelegateRecoveryTurn::ReportedRecoveryOnly(start)
            }
            ResidentProcessOperationRestartDisposition::LocalCancellationRequested(recovered) => {
                ResidentProcessDelegateRecoveryTurn::Scheduled(
                    owner
                        .complete_local_cancellation(
                            self,
                            recovered,
                            ResidentDeploymentOperationAccess {
                                store,
                                custody: &mut *custody,
                                sessions,
                                anchor: &mut *anchor,
                            },
                        )
                        .map_err(ResidentProcessDelegateRecoveryError::Route)?,
                )
            }
            ResidentProcessOperationRestartDisposition::OperationResolution(recovered) => {
                match &recovered.start.lineage {
                    ResidentProcessOperationLineage::PairwiseDelegate(_) => {}
                    ResidentProcessOperationLineage::Root
                    | ResidentProcessOperationLineage::ConsultChild(_) => {
                        return Err(ResidentProcessDelegateRecoveryError::State(
                            ResidentProcessDelegateRecoveryStateError::PairwiseLineage,
                        ));
                    }
                }
                ResidentProcessDelegateRecoveryTurn::DeferredResolution(recovered)
            }
            ResidentProcessOperationRestartDisposition::ProviderResolved(recovered) => {
                ResidentProcessDelegateRecoveryTurn::Scheduled(
                    owner
                        .recover_provider_resolved(
                            self,
                            recovered,
                            ResidentDeploymentOperationAccess {
                                store,
                                custody: &mut *custody,
                                sessions,
                                anchor: &mut *anchor,
                            },
                        )
                        .map_err(ResidentProcessDelegateRecoveryError::Route)?,
                )
            }
            ResidentProcessOperationRestartDisposition::ProviderPrepared(recovered) => {
                ResidentProcessDelegateRecoveryTurn::Scheduled(
                    owner
                        .recover_provider_prepared(
                            self,
                            recovered,
                            ResidentDeploymentOperationAccess {
                                store,
                                custody: &mut *custody,
                                sessions,
                                anchor: &mut *anchor,
                            },
                        )
                        .map_err(ResidentProcessDelegateRecoveryError::Route)?,
                )
            }
            ResidentProcessOperationRestartDisposition::Provider(recovered) => {
                // This owner records outcome uncertainty. It does not replay the
                // already-crossed provider call.
                ResidentProcessDelegateRecoveryTurn::Scheduled(
                    owner
                        .recover_provider_dispatched(
                            self,
                            recovered,
                            ResidentDeploymentOperationAccess {
                                store,
                                custody: &mut *custody,
                                sessions,
                                anchor: &mut *anchor,
                            },
                        )
                        .map_err(ResidentProcessDelegateRecoveryError::Route)?,
                )
            }
            ResidentProcessOperationRestartDisposition::NativePrepared(recovered) => {
                ResidentProcessDelegateRecoveryTurn::Scheduled(
                    owner
                        .recover_native_prepared(
                            self,
                            recovered,
                            ResidentDeploymentOperationAccess {
                                store,
                                custody: &mut *custody,
                                sessions,
                                anchor: &mut *anchor,
                            },
                        )
                        .map_err(ResidentProcessDelegateRecoveryError::Route)?,
                )
            }
            ResidentProcessOperationRestartDisposition::Native(recovered) => {
                ResidentProcessDelegateRecoveryTurn::Scheduled(
                    owner
                        .recover_native_dispatched(
                            self,
                            recovered,
                            ResidentDeploymentOperationAccess {
                                store,
                                custody: &mut *custody,
                                sessions,
                                anchor: &mut *anchor,
                            },
                        )
                        .map_err(ResidentProcessDelegateRecoveryError::Route)?,
                )
            }
            ResidentProcessOperationRestartDisposition::A2aDelegateSendRecoveryOnly(recovered) => {
                ResidentProcessDelegateRecoveryTurn::Scheduled(
                    owner
                        .recover_a2a_pre_task(
                            self,
                            recovered,
                            ResidentDeploymentOperationAccess {
                                store,
                                custody: &mut *custody,
                                sessions,
                                anchor: &mut *anchor,
                            },
                        )
                        .map_err(ResidentProcessDelegateRecoveryError::Route)?,
                )
            }
            ResidentProcessOperationRestartDisposition::A2aTask(recovered) => {
                ResidentProcessDelegateRecoveryTurn::Scheduled(
                    owner
                        .recover_a2a_task(
                            self,
                            recovered,
                            ResidentDeploymentOperationAccess {
                                store,
                                custody: &mut *custody,
                                sessions,
                                anchor: &mut *anchor,
                            },
                        )
                        .map_err(ResidentProcessDelegateRecoveryError::Route)?,
                )
            }
            ResidentProcessOperationRestartDisposition::A2aCancelTaskRequestRecoveryOnly(
                recovered,
            ) => ResidentProcessDelegateRecoveryTurn::Scheduled(
                owner
                    .recover_a2a_cancel_task(
                        self,
                        recovered,
                        ResidentDeploymentOperationAccess {
                            store,
                            custody: &mut *custody,
                            sessions,
                            anchor: &mut *anchor,
                        },
                    )
                    .map_err(ResidentProcessDelegateRecoveryError::Route)?,
            ),
            ResidentProcessOperationRestartDisposition::Consult(_) => {
                return Err(ResidentProcessDelegateRecoveryError::State(
                    ResidentProcessDelegateRecoveryStateError::ConsultDisposition,
                ));
            }
        };

        Ok(turn)
    }
}
