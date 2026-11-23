//! One-source cold-start recovery for committed root provider Invokes.
//!
//! This sweep owns no scheduler. It advances one deployment cursor only after
//! the discovered start is fully re-admitted and receives a closed disposition.
//! Unsupported operation owners are reported explicitly and never dispatched.

use super::*;
use crate::resident::checkpoint_recovery::check_record;
use crate::resident::local_operation_start::{
    discover_local_operation_start, LocalOperationStartDiscoveryError,
    LocalOperationStartDiscoveryTurn,
};
use crate::resident::process_host::restart::{
    ResidentProcessRecoveredLocalCancellation, ResidentProcessRecoveredOperationResolution,
    ResidentProcessRecoveredOperationStart, ResidentProcessRecoveredPreparedAttempt,
    ResidentProcessRecoveredProviderResolution,
};
use crate::resident::LocalOperationRecoveryContext;
use pale_core::{
    canon,
    records::{CommonRequestEnvelopeWireV1, COMMON_REQUEST_ENVELOPE_V1_SCHEMA_ID},
};
use pale_ports::OperationStartSource;

/// Closed result from pre-attempt implementation resolution.
pub(super) enum ResidentProcessRootProviderResolution<Output> {
    /// The existing provider-resolution owner accepted this exact operation.
    Provider(Output),
    /// The built-in owner durably accepted this exact operation.
    Native(Output),
}

/// Application dispatch seam for one supported cold-start phase.
///
/// This interface does not select a runtime program, signer, executor, clock,
/// record identity, or scheduling policy. A resident program-level composition
/// supplies those inputs and implements each method by calling the existing
/// phase owner. Separate methods prevent one recovered phase from being passed
/// to another owner. A future concrete implementer must return `Ok` only after
/// that existing owner has durably closed the phase, or has durably handed it to
/// its separately retained successor. Only then may the frozen-prefix cursor
/// advance. An error keeps the source selected for another bounded turn.
pub(super) trait ResidentProcessRootProviderRecoveryOwner<WriterGuard> {
    /// Phase-owned result returned after a closed recovery disposition.
    type Output;
    /// Phase-owned failure that preserves any linear continuation it needs.
    type Error;

    /// Complete one retained pre-dispatch cancellation without a provider call.
    fn complete_local_cancellation<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredLocalCancellation>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error>;

    /// Continue one exact root Invoke through implementation resolution.
    fn recover_operation_resolution<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredOperationResolution>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<ResidentProcessRootProviderResolution<Self::Output>, Self::Error>;

    /// Retain the first exact provider attempt after durable resolution.
    fn recover_provider_resolved<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredProviderResolution>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error>;

    /// Complete and release one exact retained provider preparation.
    fn recover_provider_prepared<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredPreparedAttempt>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error>;

    /// Record durable uncertainty without replaying a crossed provider call.
    fn recover_provider_dispatched<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredAttempt>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error>;

    /// Complete or durably retain one prepared built-in dispatch.
    fn recover_native_prepared<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredPreparedAttempt>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error>;

    /// Record or durably retain one built-in attempt that crossed dispatch.
    fn recover_native_dispatched<'store>(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        recovered: Box<ResidentProcessRecoveredAttempt>,
        access: ResidentDeploymentOperationAccess<'_>,
    ) -> Result<Self::Output, Self::Error>;
}

/// Unsupported owner that the 0.0.1 root-provider sweep will not run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResidentProcessOperationRecoveryDeferred {
    /// Consultation policy and fan-in remain with the consultation owner.
    Consult,
    /// Child delegation requires explicit parent-lineage discovery.
    Delegate,
    /// A2A custody and carrier recovery remain with the A2A owner.
    A2a,
    /// Native actions and hook responses remain with their separate semantic owners.
    Native,
}

/// One completed recovery-sweep turn.
pub(super) enum ResidentProcessOperationRecoveryTurn<Output> {
    /// Every source in this deployment's frozen prefix has been processed.
    Exhausted,
    /// A terminal operation was observed and no work was scheduled.
    ReportedTerminal,
    /// A recovery-only operation was observed and no work was scheduled.
    ReportedRecoveryOnly(Box<ResidentProcessRecoveredOperationStart>),
    /// Another semantic owner was identified without running its work.
    DeferredUnsupported {
        owner: ResidentProcessOperationRecoveryDeferred,
        disposition: Option<Box<ResidentProcessOperationRestartDisposition>>,
    },
    /// The existing root-provider owner accepted this source.
    Scheduled(Output),
}

/// Route one already classified root-provider disposition through its owner.
///
/// Point-addressed `AdvanceInvoke` and frozen-prefix recovery share this
/// semantic routing, but not discovery. The caller must establish exact
/// locator or source admission before this function. No cursor is advanced
/// here.
pub(super) fn route_root_provider_disposition<'store, WriterGuard, Owner>(
    host: &mut ResidentProcessHost<'store, WriterGuard>,
    disposition: ResidentProcessOperationRestartDisposition,
    access: ResidentDeploymentOperationAccess<'_>,
    owner: &mut Owner,
) -> Result<ResidentProcessOperationRecoveryTurn<Owner::Output>, Owner::Error>
where
    Owner: ResidentProcessRootProviderRecoveryOwner<WriterGuard>,
{
    let ResidentDeploymentOperationAccess {
        store,
        custody,
        sessions,
        anchor,
    } = access;
    Ok(match classify_root_provider_work(disposition) {
        ClassifiedRecoveryWork::Closed(turn) => turn,
        ClassifiedRecoveryWork::LocalCancellation(recovered) => {
            ResidentProcessOperationRecoveryTurn::Scheduled(owner.complete_local_cancellation(
                host,
                recovered,
                ResidentDeploymentOperationAccess {
                    store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
            )?)
        }
        ClassifiedRecoveryWork::OperationResolution(recovered) => {
            let output = owner.recover_operation_resolution(
                host,
                recovered,
                ResidentDeploymentOperationAccess {
                    store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
            )?;
            ResidentProcessOperationRecoveryTurn::Scheduled(match output {
                ResidentProcessRootProviderResolution::Provider(output)
                | ResidentProcessRootProviderResolution::Native(output) => output,
            })
        }
        ClassifiedRecoveryWork::ProviderResolved(recovered) => {
            ResidentProcessOperationRecoveryTurn::Scheduled(owner.recover_provider_resolved(
                host,
                recovered,
                ResidentDeploymentOperationAccess {
                    store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
            )?)
        }
        ClassifiedRecoveryWork::ProviderPrepared(recovered) => {
            ResidentProcessOperationRecoveryTurn::Scheduled(owner.recover_provider_prepared(
                host,
                recovered,
                ResidentDeploymentOperationAccess {
                    store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
            )?)
        }
        ClassifiedRecoveryWork::ProviderDispatched(recovered) => {
            ResidentProcessOperationRecoveryTurn::Scheduled(owner.recover_provider_dispatched(
                host,
                recovered,
                ResidentDeploymentOperationAccess {
                    store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
            )?)
        }
        ClassifiedRecoveryWork::NativePrepared(recovered) => {
            ResidentProcessOperationRecoveryTurn::Scheduled(owner.recover_native_prepared(
                host,
                recovered,
                ResidentDeploymentOperationAccess {
                    store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
            )?)
        }
        ClassifiedRecoveryWork::NativeDispatched(recovered) => {
            ResidentProcessOperationRecoveryTurn::Scheduled(owner.recover_native_dispatched(
                host,
                recovered,
                ResidentDeploymentOperationAccess {
                    store,
                    custody: &mut *custody,
                    sessions,
                    anchor: &mut *anchor,
                },
            )?)
        }
    })
}

/// Why one recovery-sweep turn could not reach a closed disposition.
#[derive(Debug)]
pub(super) enum ResidentProcessOperationRecoverySweepError<RouteError> {
    /// A committed observation could not establish its exact independent anchor.
    PendingObservation(crate::resident::local_operation_start::PendingRootObservationError),
    /// Bounded physical discovery failed. The deployment cursor is unchanged.
    Discovery(LocalOperationStartDiscoveryError),
    /// The physical source failed exact request, handle, basis, or start admission.
    Source(ResidentProcessOperationRecoverySourceError),
    /// Protected restart classification failed. The deployment cursor is unchanged.
    Restart(ResidentProcessOperationRestartError),
    /// The phase-specific owner refused or retained unfinished work.
    Route(RouteError),
}

/// Why a discovered source could not become an exact existing start.
#[derive(Debug, thiserror::Error)]
pub(super) enum ResidentProcessOperationRecoverySourceError {
    /// The selected store or one immutable coordinate was substituted.
    #[error("operation recovery source association differs")]
    Association,
    /// The request record could not be read exactly.
    #[error(transparent)]
    Store(#[from] pale_ports::ControlStoreError),
    /// The request lookup was incomplete, conflicting, or misdirected.
    #[error(transparent)]
    Start(#[from] super::super::OperationRecoveryError),
    /// The embedded request schema could not be selected.
    #[error(transparent)]
    Schema(#[from] pale_contracts::SchemaError),
    /// Canonical request bytes were malformed or unlike.
    #[error(transparent)]
    Canonical(#[from] canon::CanonError),
    /// The retained wire was not a common request.
    #[error(transparent)]
    Decode(#[from] serde_json::Error),
    /// Historical request admission rejected the retained signer or meaning.
    #[error(transparent)]
    Request(#[from] InvalidResidentRecordV1),
}

impl<'store, WriterGuard> ResidentProcessHost<'store, WriterGuard> {
    /// Recover and route at most one committed start from one deployment.
    ///
    /// The selected phase method receives only its typed root-provider state.
    /// It reports success only after the state is safe to rediscover. An owner
    /// error leaves the deployment cursor unchanged.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn recover_one_root_provider_start<Owner>(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        restart: ResidentProcessOperationRestartInput<'_>,
        mut admission: OperationControlRecoveryAdmission<'_>,
        maximum_session_basis_bytes: usize,
        owner: &mut Owner,
    ) -> Result<
        ResidentProcessOperationRecoveryTurn<Owner::Output>,
        ResidentProcessOperationRecoverySweepError<Owner::Error>,
    >
    where
        Owner: ResidentProcessRootProviderRecoveryOwner<WriterGuard>,
    {
        let index = self.deployment_authority_index(selector).map_err(|error| {
            ResidentProcessOperationRecoverySweepError::Restart(
                ResidentProcessOperationRestartError::Selection(error),
            )
        })?;
        if self.deployments[index].store() != access.store {
            return Err(ResidentProcessOperationRecoverySweepError::Restart(
                ResidentProcessHostSelectionError::AccessStoreAssociation.into(),
            ));
        }
        let attempt = self.operation_recovery_attempts[index].clone();
        let ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions,
            anchor,
        } = access;
        let discovered =
            discover_local_operation_start(attempt, &mut *custody, maximum_session_basis_bytes)
                .map_err(ResidentProcessOperationRecoverySweepError::Discovery)?;
        let pending = match discovered {
            LocalOperationStartDiscoveryTurn::Exhausted(exhausted) => {
                self.operation_recovery_attempts[index] = exhausted;
                return Ok(ResidentProcessOperationRecoveryTurn::Exhausted);
            }
            LocalOperationStartDiscoveryTurn::Pending(pending) => pending,
        };
        let (request, evidence) = admit_discovered_start(
            store,
            pending.source(),
            &mut *custody,
            restart.operation.maximum_record_bytes,
            &mut *admission.admit_request,
        )
        .map_err(ResidentProcessOperationRecoverySweepError::Source)?;
        let request_preimage = request.to_preimage();
        let association = ResidentOperationControlAssociation {
            current_request_authority: &request_preimage.signing_authority_binding,
            historical_handle_authority: restart.operation.handle_signing_authority,
            recovery_store: restart.operation.store,
            checkpoint_authority: restart.operation.checkpoint_authority,
        };
        let existing = ResidentProcessExistingOperationStart {
            selector: selector.clone(),
            evidence: Box::new(evidence),
        };
        super::restart::verify_restart_inputs(
            self.deployments[index].application.as_ref(),
            &existing,
            &association,
            store,
            &restart,
        )
        .map_err(ResidentProcessOperationRecoverySweepError::Restart)?;
        if let Some(limits) = restart.committed_observation_limits {
            crate::resident::local_operation_start::recover_pending_root_observation(
                LocalOperationRecoveryContext {
                    observation_limits: restart.current_head.observation,
                    checkpoint_limits: restart.current_head.checkpoints,
                    membership_limits: restart.current_head.membership,
                    ..restart.operation
                },
                (&request, &existing.evidence, restart.maximum_parent_depth),
                limits,
                custody,
                sessions,
                anchor,
                &mut admission,
                None,
            )
            .map_err(ResidentProcessOperationRecoverySweepError::PendingObservation)?;
        }
        let protected = self
            .protect_existing_operation_start(
                existing,
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
            .map_err(ResidentProcessOperationRecoverySweepError::Restart)?;

        if protected.parent_operation().is_some()
            || protected.operation_class() == &OperationClassV1::Delegate
        {
            self.operation_recovery_attempts[index] = pending.advance();
            return Ok(ResidentProcessOperationRecoveryTurn::DeferredUnsupported {
                owner: ResidentProcessOperationRecoveryDeferred::Delegate,
                disposition: None,
            });
        }

        let disposition = self
            .classify_protected_root_operation(
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
            .map_err(ResidentProcessOperationRecoverySweepError::Restart)?;

        let turn = route_root_provider_disposition(
            self,
            disposition,
            ResidentDeploymentOperationAccess {
                store,
                custody: &mut *custody,
                sessions,
                anchor: &mut *anchor,
            },
            owner,
        )
        .map_err(ResidentProcessOperationRecoverySweepError::Route)?;
        if !matches!(
            turn,
            ResidentProcessOperationRecoveryTurn::DeferredUnsupported {
                owner: ResidentProcessOperationRecoveryDeferred::A2a
                    | ResidentProcessOperationRecoveryDeferred::Native,
                ..
            }
        ) {
            self.operation_recovery_attempts[index] = pending.advance();
        }
        Ok(turn)
    }
}

enum ClassifiedRecoveryWork<Output> {
    Closed(ResidentProcessOperationRecoveryTurn<Output>),
    LocalCancellation(Box<ResidentProcessRecoveredLocalCancellation>),
    OperationResolution(Box<ResidentProcessRecoveredOperationResolution>),
    ProviderResolved(Box<ResidentProcessRecoveredProviderResolution>),
    ProviderPrepared(Box<ResidentProcessRecoveredPreparedAttempt>),
    ProviderDispatched(Box<ResidentProcessRecoveredAttempt>),
    NativePrepared(Box<ResidentProcessRecoveredPreparedAttempt>),
    NativeDispatched(Box<ResidentProcessRecoveredAttempt>),
}

/// Semantic owner identified by the protected restart classifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RestartSemanticOwner {
    Ordinary,
    Consult,
    A2a,
    Native,
}

fn classify_root_provider_work<Output>(
    disposition: ResidentProcessOperationRestartDisposition,
) -> ClassifiedRecoveryWork<Output> {
    let (class, parent) = disposition_operation_class(&disposition);
    if let Some(deferred) = deferred_owner(semantic_owner(&disposition), class, parent) {
        return ClassifiedRecoveryWork::Closed(
            ResidentProcessOperationRecoveryTurn::DeferredUnsupported {
                owner: deferred,
                disposition: Some(Box::new(disposition)),
            },
        );
    }
    match disposition {
        disposition @ ResidentProcessOperationRestartDisposition::NativeHookResponse(_) => {
            ClassifiedRecoveryWork::Closed(ResidentProcessOperationRecoveryTurn::DeferredUnsupported {
                owner: ResidentProcessOperationRecoveryDeferred::Native,
                disposition: Some(Box::new(disposition)),
            })
        }
        ResidentProcessOperationRestartDisposition::Terminal(_) => {
            ClassifiedRecoveryWork::Closed(ResidentProcessOperationRecoveryTurn::ReportedTerminal)
        }
        ResidentProcessOperationRestartDisposition::RecoveryOnly(start) => {
            ClassifiedRecoveryWork::Closed(
                ResidentProcessOperationRecoveryTurn::ReportedRecoveryOnly(start),
            )
        }
        ResidentProcessOperationRestartDisposition::LocalCancellationRequested(value) => {
            ClassifiedRecoveryWork::LocalCancellation(value)
        }
        ResidentProcessOperationRestartDisposition::OperationResolution(value) => {
            ClassifiedRecoveryWork::OperationResolution(value)
        }
        ResidentProcessOperationRestartDisposition::ProviderResolved(value) => {
            ClassifiedRecoveryWork::ProviderResolved(value)
        }
        ResidentProcessOperationRestartDisposition::ProviderPrepared(value) => {
            ClassifiedRecoveryWork::ProviderPrepared(value)
        }
        ResidentProcessOperationRestartDisposition::Provider(value) => {
            ClassifiedRecoveryWork::ProviderDispatched(value)
        }
        ResidentProcessOperationRestartDisposition::NativePrepared(recovered) => {
            ClassifiedRecoveryWork::NativePrepared(recovered)
        }
        ResidentProcessOperationRestartDisposition::Native(recovered) => {
            ClassifiedRecoveryWork::NativeDispatched(recovered)
        }
        ResidentProcessOperationRestartDisposition::Consult(start) => {
            ClassifiedRecoveryWork::Closed(
                ResidentProcessOperationRecoveryTurn::DeferredUnsupported {
                    owner: ResidentProcessOperationRecoveryDeferred::Consult,
                    disposition: Some(Box::new(
                        ResidentProcessOperationRestartDisposition::Consult(start),
                    )),
                },
            )
        }
        disposition @ (ResidentProcessOperationRestartDisposition::A2aDelegateSendRecoveryOnly(_)
        | ResidentProcessOperationRestartDisposition::A2aTask(_)
        | ResidentProcessOperationRestartDisposition::A2aCancelTaskRequestRecoveryOnly(_)) => {
            ClassifiedRecoveryWork::Closed(
                ResidentProcessOperationRecoveryTurn::DeferredUnsupported {
                    owner: a2a_deferred_owner(),
                    disposition: Some(Box::new(disposition)),
                },
            )
        }
    }
}

fn semantic_owner(
    disposition: &ResidentProcessOperationRestartDisposition,
) -> RestartSemanticOwner {
    match disposition {
        ResidentProcessOperationRestartDisposition::Consult(_) => RestartSemanticOwner::Consult,
        ResidentProcessOperationRestartDisposition::A2aDelegateSendRecoveryOnly(_)
        | ResidentProcessOperationRestartDisposition::A2aTask(_)
        | ResidentProcessOperationRestartDisposition::A2aCancelTaskRequestRecoveryOnly(_) => {
            RestartSemanticOwner::A2a
        }
        ResidentProcessOperationRestartDisposition::NativePrepared(_)
        | ResidentProcessOperationRestartDisposition::Native(_)
        | ResidentProcessOperationRestartDisposition::NativeHookResponse(_) => {
            RestartSemanticOwner::Native
        }
        ResidentProcessOperationRestartDisposition::Terminal(_)
        | ResidentProcessOperationRestartDisposition::RecoveryOnly(_)
        | ResidentProcessOperationRestartDisposition::LocalCancellationRequested(_)
        | ResidentProcessOperationRestartDisposition::OperationResolution(_)
        | ResidentProcessOperationRestartDisposition::ProviderPrepared(_)
        | ResidentProcessOperationRestartDisposition::ProviderResolved(_)
        | ResidentProcessOperationRestartDisposition::Provider(_) => RestartSemanticOwner::Ordinary,
    }
}

const fn deferred_owner(
    semantic: RestartSemanticOwner,
    class: OperationClassV1,
    parent: bool,
) -> Option<ResidentProcessOperationRecoveryDeferred> {
    match semantic {
        RestartSemanticOwner::Consult => Some(ResidentProcessOperationRecoveryDeferred::Consult),
        RestartSemanticOwner::A2a => Some(ResidentProcessOperationRecoveryDeferred::A2a),
        RestartSemanticOwner::Native => None,
        RestartSemanticOwner::Ordinary => class_deferred_owner(class, parent),
    }
}

const fn class_deferred_owner(
    class: OperationClassV1,
    parent: bool,
) -> Option<ResidentProcessOperationRecoveryDeferred> {
    if matches!(class, OperationClassV1::Consult) {
        Some(ResidentProcessOperationRecoveryDeferred::Consult)
    } else if matches!(class, OperationClassV1::Delegate) || parent {
        Some(ResidentProcessOperationRecoveryDeferred::Delegate)
    } else {
        None
    }
}

const fn a2a_deferred_owner() -> ResidentProcessOperationRecoveryDeferred {
    ResidentProcessOperationRecoveryDeferred::A2a
}

fn disposition_operation_class(
    disposition: &ResidentProcessOperationRestartDisposition,
) -> (OperationClassV1, bool) {
    let start = match disposition {
        ResidentProcessOperationRestartDisposition::NativeHookResponse(value) => &value.start,
        ResidentProcessOperationRestartDisposition::Terminal(value)
        | ResidentProcessOperationRestartDisposition::RecoveryOnly(value)
        | ResidentProcessOperationRestartDisposition::Consult(value) => value,
        ResidentProcessOperationRestartDisposition::LocalCancellationRequested(value) => {
            &value.start
        }
        ResidentProcessOperationRestartDisposition::OperationResolution(value) => &value.start,
        ResidentProcessOperationRestartDisposition::NativePrepared(value)
        | ResidentProcessOperationRestartDisposition::ProviderPrepared(value) => &value.start,
        ResidentProcessOperationRestartDisposition::ProviderResolved(value) => &value.start,
        ResidentProcessOperationRestartDisposition::Native(value)
        | ResidentProcessOperationRestartDisposition::Provider(value)
        | ResidentProcessOperationRestartDisposition::A2aDelegateSendRecoveryOnly(value) => {
            &value.start
        }
        ResidentProcessOperationRestartDisposition::A2aTask(value) => &value.attempt.start,
        ResidentProcessOperationRestartDisposition::A2aCancelTaskRequestRecoveryOnly(value) => {
            &value.attempt.start
        }
    };
    let handle = start.control.operation().operation().handle().1;
    (
        *handle.operation_class(),
        handle.parent_operation().is_some(),
    )
}

pub(super) fn admit_discovered_start(
    store: &AuthorityStoreCoordinate,
    source: &OperationStartSource,
    custody: &mut dyn OperationCustodyPort,
    maximum_record_bytes: usize,
    admit_request: &mut dyn FnMut(
        &ResidentRecordRefV1,
        CommonRequestEnvelopeWireV1,
    ) -> Result<CommonRequestEnvelopeV1, InvalidResidentRecordV1>,
) -> Result<
    (CommonRequestEnvelopeV1, OperationStartRecoveryEvidence),
    ResidentProcessOperationRecoverySourceError,
> {
    if maximum_record_bytes == 0 {
        return Err(ResidentProcessOperationRecoverySourceError::Association);
    }
    let basis = source.session_basis();
    let request_record = custody
        .read_operation_record(store, basis.request(), maximum_record_bytes)?
        .ok_or(ResidentProcessOperationRecoverySourceError::Association)?;
    if request_record.coordinate() != basis.request() {
        return Err(ResidentProcessOperationRecoverySourceError::Association);
    }
    let schema = pale_contracts::record_schema_identity(COMMON_REQUEST_ENVELOPE_V1_SCHEMA_ID)?;
    check_record(
        &request_record,
        &schema.schema,
        &schema.schema_sha256,
        maximum_record_bytes,
    )
    .map_err(|_| ResidentProcessOperationRecoverySourceError::Association)?;
    let wire: CommonRequestEnvelopeWireV1 = serde_json::from_value(canon::parse_strict(
        request_record.canonical_record_jcs().as_slice(),
    )?)?;
    let request = admit_request(request_record.coordinate().record_ref(), wire)?;
    if request.envelope_schema_id() != &schema.schema
        || request.envelope_schema_sha256() != &schema.schema_sha256
        || request.canonical_request_sha256().as_str()
            != request_record.coordinate().record_sha256().as_str()
        || canon::to_canonical_string(&request)?.as_bytes()
            != request_record.canonical_record_jcs().as_slice()
    {
        return Err(ResidentProcessOperationRecoverySourceError::Association);
    }
    let evidence = super::super::read_operation_start_for_request(
        custody,
        store,
        &request,
        maximum_record_bytes,
    )?
    .ok_or(ResidentProcessOperationRecoverySourceError::Association)?;
    if evidence.request().coordinate() != basis.request()
        || evidence.handle().coordinate() != basis.handle()
    {
        return Err(ResidentProcessOperationRecoverySourceError::Association);
    }
    let exact_basis = custody
        .read_operation_session_basis(
            store,
            basis.handle(),
            maximum_record_bytes.min(pale_ports::MAX_OPERATION_SESSION_BASIS_BYTES),
        )?
        .ok_or(ResidentProcessOperationRecoverySourceError::Association)?;
    if &exact_basis != basis {
        return Err(ResidentProcessOperationRecoverySourceError::Association);
    }
    Ok((request, evidence))
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/process_host/operation_recovery_sweep_classification.rs"]
mod classification_tests;
