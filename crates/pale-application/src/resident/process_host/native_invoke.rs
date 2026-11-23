//! Run one admitted native Invoke through its selected deployment.

use super::*;
use crate::actions::NativeActionRegistry;
use crate::resident::operation_dispatch::{
    execute_native_operation_work, invoke_native_operation, prepare_native_operation_work,
    record_native_operation_outcome, record_recovered_native_operation_outcome,
    recover_native_operation_outcome_phase, AdmittedStagedNativeOperationOutcome,
    CompletedNativeOperation, IssuedNativeOperationStageCheckpoint, NativeOperationAttemptInput,
    NativeOperationInvokeFailure, NativeOperationInvokeInput, NativeOperationOutcomeRecordingInput,
    NativeOperationOutcomeRecoveryInput, NativeOperationRecordingFailure,
    NativeOperationRecoveryContext, NativeOperationStageFinalization,
    NativeOperationStagePreparationContinuation, NativeOperationStagingAnchorPending,
    NativeOperationStagingCommitFailure, NativeOperationTerminalRecordingInput,
    OperationAttemptContext, OperationAttemptError, OperationAttemptHistoryInput,
    OperationCheckpointInput, PreparedNativeOperationStageCustody, PreparedNativeOperationWork,
};
use crate::resident::process_host::consult_child::ResidentProcessConsultChildDriver;
use crate::resident::process_host::operation_resolution::{
    validate_attempt_refs, ResidentProcessOperationResolutionFailure,
    ResidentProcessOperationResolutionInput,
};
use crate::resident::process_host::restart::{
    ResidentProcessAttemptObservation, ResidentProcessOperationLineage,
    ResidentProcessOperationResolutionPhase, ResidentProcessRecoveredAttempt,
    ResidentProcessRecoveredOperationResolution, ResidentProcessRecoveredOperationStart,
};
use crate::resident::ConsultationChildDispatchBatch;
use pale_core::records::{
    OperationActivePhaseV1, OperationObservationAdmissionContextV1, OperationObservationStateV1,
    OperationObservationV1,
};

/// One exact continuation rejected before any phase-specific effect.
///
/// The caller retains the continuation and can retry it only after correcting
/// the process-host association. A selection rejection never consumes custody.
pub(super) struct NativeContinuationSelectionFailure<Continuation> {
    continuation: Box<Continuation>,
    reason: ResidentProcessHostSelectionError,
}

impl<Continuation> std::fmt::Debug for NativeContinuationSelectionFailure<Continuation> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeContinuationSelectionFailure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

impl<Continuation> NativeContinuationSelectionFailure<Continuation> {
    pub(super) fn into_parts(self) -> (Continuation, ResidentProcessHostSelectionError) {
        (*self.continuation, self.reason)
    }
}

/// Fresh recording coordinates for one recomputed native outcome.
pub(super) struct ResidentProcessNativeRecoveryRecordingInput<'a> {
    pub(super) outcome_ref: ResidentRecordRefV1,
    pub(super) outcome_observation_ref: ResidentRecordRefV1,
    pub(super) result_ref: ResidentRecordRefV1,
    pub(super) diagnostic_ref: ResidentRecordRefV1,
    pub(super) terminal_ref: ResidentRecordRefV1,
    pub(super) outcome_checkpoint: super::super::operation_dispatch::OperationCheckpointInput,
    pub(super) terminal_checkpoint: super::super::operation_dispatch::OperationCheckpointInput,
    pub(super) clock: &'a dyn pale_ports::WallClockSourcePort,
    pub(super) resolve_store: &'a dyn super::super::OperationObservationAuthorityStoreResolver,
    pub(super) checkpoint_schema: &'a pale_core::records::SchemaDigest,
    pub(super) checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    pub(super) signing: pale_core::records::SignedResidentAdmissionContextV1<'a>,
    pub(super) maximum_diagnostic_bytes: usize,
    pub(super) maximum_outcome_bytes: usize,
    pub(super) accumulator_limits: pale_ports::AccumulatorPreparationLimits,
}

/// Current authority and fresh custody inputs for one original native dispatch.
pub(super) struct ResidentProcessNativeRecoveryInput<'operation, 'call, 'registry> {
    pub(super) attempt: &'operation ResidentProcessRecoveredAttempt,
    pub(super) current: &'call super::super::AmbientSessionAdmissionContext<'call>,
    pub(super) registry: &'registry NativeActionRegistry,
    pub(super) contract_set: &'operation pale_core::records::ContractSetV1,
    pub(super) maximum_selection_bytes: usize,
    pub(super) maximum_record_bytes: usize,
    pub(super) recording: ResidentProcessNativeRecoveryRecordingInput<'call>,
}

/// Fresh terminal coordinates for one already staged native outcome.
pub(super) struct ResidentProcessStagedNativeRecoveryRecordingInput<'a> {
    pub(super) result_ref: ResidentRecordRefV1,
    pub(super) diagnostic_ref: ResidentRecordRefV1,
    pub(super) terminal_ref: ResidentRecordRefV1,
    pub(super) terminal_checkpoint: super::super::operation_dispatch::OperationCheckpointInput,
    pub(super) clock: &'a dyn pale_ports::WallClockSourcePort,
    pub(super) resolve_store: &'a dyn super::super::OperationObservationAuthorityStoreResolver,
    pub(super) checkpoint_schema: &'a pale_core::records::SchemaDigest,
    pub(super) checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    pub(super) signing: pale_core::records::SignedResidentAdmissionContextV1<'a>,
    pub(super) maximum_diagnostic_bytes: usize,
    pub(super) accumulator_limits: pale_ports::AccumulatorPreparationLimits,
}

/// Protected history and fresh terminal custody for one staged native outcome.
pub(super) struct ResidentProcessStagedNativeRecoveryInput<'operation, 'call, 'registry> {
    pub(super) attempt: &'operation ResidentProcessRecoveredAttempt,
    pub(super) current: &'call super::super::AmbientSessionAdmissionContext<'call>,
    pub(super) registry: &'registry NativeActionRegistry,
    pub(super) contract_set: &'operation pale_core::records::ContractSetV1,
    pub(super) maximum_selection_bytes: usize,
    pub(super) maximum_record_bytes: usize,
    pub(super) maximum_outcome_bytes: usize,
    pub(super) recording: ResidentProcessStagedNativeRecoveryRecordingInput<'call>,
}

/// Why cold native recomputation stopped before terminal evidence existed.
#[derive(Debug)]
pub(super) enum ResidentProcessNativeRecoveryFailure {
    /// Process deployment or port selection differed before native admission.
    Selection(ResidentProcessHostSelectionError),
    /// Protected dispatch reconstruction or current release checks failed.
    Admission(OperationAttemptError),
    /// A staged outcome exists and must use outcome recovery without a handler call.
    StagedOutcomeRequired,
    /// Cancellation already follows the dispatch and prohibits recomputation.
    CancellationRequested,
    /// The handler ran once and recording owns the only continuation.
    Recording(NativeOperationRecordingFailure),
}

/// Why staged native recovery stopped before terminal custody completed.
#[derive(Debug)]
pub(super) enum ResidentProcessStagedNativeRecoveryFailure {
    /// Process deployment or authority selection differed.
    Selection(ResidentProcessHostSelectionError),
    /// Protected history, custody, or the staged outcome differed.
    Admission(OperationAttemptError),
    /// Terminal recording owns the only remaining continuation.
    Recording(NativeOperationRecordingFailure),
}

/// Fresh native identities and custody inputs not proved by Invoke recovery.
pub(super) struct ResidentProcessNativeInvokeBasis<'operation, 'call, 'registry> {
    pub(super) registry: &'registry NativeActionRegistry,
    pub(super) attempt_id: &'operation pale_core::records::CapabilityAttemptIdV1,
    pub(super) result_admission:
        pale_core::records::CapabilitySuccessAdmissionContextV1<'operation>,
    pub(super) checkpoint_schema: &'call pale_core::records::SchemaDigest,
    pub(super) checkpoint_authority:
        &'call ControlCheckpointSigningAuthorityAdmissionContextV1<'call>,
    pub(super) preparation_checkpoint: OperationCheckpointInput,
    pub(super) dispatch_checkpoint: OperationCheckpointInput,
    pub(super) outcome_checkpoint: OperationCheckpointInput,
    pub(super) terminal_checkpoint: OperationCheckpointInput,
    pub(super) dispatch_ref: ResidentRecordRefV1,
    pub(super) outcome_ref: ResidentRecordRefV1,
    pub(super) outcome_observation_ref: ResidentRecordRefV1,
    pub(super) result_ref: ResidentRecordRefV1,
    pub(super) diagnostic_ref: ResidentRecordRefV1,
    pub(super) terminal_ref: ResidentRecordRefV1,
    pub(super) clock: &'call dyn pale_ports::WallClockSourcePort,
    pub(super) resolve_store: &'call dyn super::super::OperationObservationAuthorityStoreResolver,
    pub(super) maximum_diagnostic_bytes: usize,
    pub(super) maximum_outcome_bytes: usize,
    pub(super) accumulator_limits: pale_ports::AccumulatorPreparationLimits,
}

/// One exact Consult child retained at durable native dispatch.
///
/// The resident task owner holds this linear value. Its handler has not run.
pub(super) struct ResidentProcessConsultNativeChildWork<'operation, 'call, 'registry> {
    store: AuthorityStoreCoordinate,
    lineage: ResidentProcessOperationLineage,
    dispatch_ref: ResidentRecordRefV1,
    dispatch: OperationObservationV1,
    prepared: Option<Box<PreparedNativeOperationWork<'operation, 'call, 'registry>>>,
}

/// Why a Consult child did not reach one retained native dispatch.
#[derive(Debug)]
pub(super) enum ResidentProcessConsultNativeChildRetentionFailure<'operation, 'registry> {
    /// Native selection or durable dispatch stopped in its existing phase.
    Native(ResidentProcessSelfContainedNativeInvokeFailure<'operation, 'registry>),
    /// Protected recovery did not admit the dispatched child head.
    Recovery(ResidentProcessOperationRestartError),
}

pub(super) type ResidentProcessConsultNativeChildOutput<'operation, 'registry> = Result<
    CompletedNativeOperation,
    ResidentProcessSelfContainedNativeInvokeFailure<'operation, 'registry>,
>;

impl ResidentProcessConsultNativeChildWork<'_, '_, '_> {
    /// Borrow the exact durable dispatch offered to the task owner.
    pub(super) fn dispatch(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.dispatch_ref, &self.dispatch)
    }
}

/// Why one newly resolved self-contained native Invoke stopped.
#[derive(Debug)]
pub(super) enum ResidentProcessSelfContainedNativeInvokeFailure<'operation, 'registry> {
    /// Resolution or deployment selection failed before owner-specific work.
    Resolution(ResidentProcessOperationResolutionFailure),
    /// The existing native state machine retains the exact stopped phase.
    Native(NativeOperationInvokeFailure<'operation, 'registry>),
}

impl<'host, 'store> ResidentProcessConsultChildDriver<'host, 'store> {
    /// Poll one retained Consult-native child without selecting another child.
    pub(super) fn poll_consultation_native_child_turn<'operation, 'call, 'registry, Executor>(
        &mut self,
        task_limits: ResidentConsultationTaskLimits,
        observed_at: UnixTimeV1,
        current: RecoveredConsultationCollection,
        executor: &mut Executor,
    ) -> ResidentConsultationTurnDisposition<
        ResidentProcessConsultNativeChildWork<'operation, 'call, 'registry>,
        ResidentProcessConsultNativeChildOutput<'operation, 'registry>,
        std::convert::Infallible,
        std::convert::Infallible,
        std::convert::Infallible,
        (),
    >
    where
        Executor: ResidentOperationTaskExecutor<
            ResidentProcessConsultNativeChildWork<'operation, 'call, 'registry>,
            ResidentProcessConsultNativeChildOutput<'operation, 'registry>,
        >,
    {
        self.operations.supervise_recovered_consultation_turn(
            task_limits,
            observed_at,
            current,
            executor,
            |_, _| ResidentConsultationSupervisionDecision::Continue(None),
            |_| unreachable!("a ready completion is polled before child recovery"),
            |_, _| unreachable!("a completion-only turn never retains child work"),
        )
    }

    /// Offer one protected Consult child after its native dispatch is durable.
    ///
    /// This turn rechecks the exact roster child, retains `Dispatched` custody,
    /// and gives the unopened handler to the resident task owner. It neither
    /// runs the handler nor admits a child result into the parent.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub(super) fn supervise_consultation_native_child_turn<
        'operation,
        'call,
        'registry,
        Authority,
        Restart,
        Rotation,
        ObservationSign,
        CheckpointSign,
        Executor,
    >(
        &mut self,
        association: ResidentOperationControlAssociation<'_>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        task_limits: ResidentConsultationTaskLimits,
        observed_at: UnixTimeV1,
        current: RecoveredConsultationCollection,
        planned: &ConsultationChildDispatchBatch,
        candidate: ConsultationChildDispatchCandidate,
        parent: &'operation ResidentProcessRecoveredOperationStart,
        recovered: &'operation ResidentProcessRecoveredOperationResolution,
        restart: &mut Restart,
        admission: &mut OperationControlRecoveryAdmission<'_>,
        resolution: ResidentProcessOperationResolutionInput<'operation>,
        basis: ResidentProcessNativeInvokeBasis<'operation, 'call, 'registry>,
        executor: &mut Executor,
        mut admit_rotation: Rotation,
        mut sign_observation: ObservationSign,
        mut sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentConsultationTurnDisposition<
            ResidentProcessConsultNativeChildWork<'operation, 'call, 'registry>,
            ResidentProcessConsultNativeChildOutput<'operation, 'registry>,
            std::convert::Infallible,
            ResidentProcessOperationRestartError,
            ResidentProcessConsultNativeChildRetentionFailure<'operation, 'registry>,
            (),
        >,
        ResidentProcessOperationRestartError,
    >
    where
        'host: 'operation,
        'operation: 'call,
        Authority: super::super::AmbientSessionAuthorityPort,
        Restart: FnMut() -> ResidentProcessOperationRestartInput<'operation>,
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Executor: ResidentOperationTaskExecutor<
            ResidentProcessConsultNativeChildWork<'operation, 'call, 'registry>,
            ResidentProcessConsultNativeChildOutput<'operation, 'registry>,
        >,
    {
        let selector = &self.selector;
        let selected_store = self.store.clone();
        let ResidentOperationControlAssociation {
            current_request_authority,
            historical_handle_authority,
            recovery_store,
            checkpoint_authority,
        } = association;
        let ResidentDeploymentOperationAccess {
            store,
            custody,
            sessions,
            anchor,
        } = access;
        let preflight = ResidentProcessHost::<()>::classify_consultation_child_operation_in(
            self.application,
            selector,
            ResidentOperationControlAssociation {
                current_request_authority,
                historical_handle_authority,
                recovery_store,
                checkpoint_authority,
            },
            ResidentDeploymentOperationAccess {
                store,
                custody: &mut *custody,
                sessions,
                anchor: &mut *anchor,
            },
            restart(),
            admission,
            parent,
            &current,
            planned,
            &candidate,
            None,
            observed_at,
        )?;
        let ResidentProcessOperationRestartDisposition::OperationResolution(preflight) = preflight
        else {
            return Err(ResidentProcessOperationRestartError::StartEvidence);
        };
        if preflight.start.selector != recovered.start.selector
            || preflight.start.generation_digest != recovered.start.generation_digest
            || preflight.start.lineage != recovered.start.lineage
        {
            return Err(ResidentProcessOperationRestartError::StartEvidence);
        }
        let (_, preflight_observation) = preflight.start.control.into_parts();

        let application = self.application;
        let operations = &mut *self.operations;
        debug_assert_eq!(application.store(), &selected_store);
        let current_for_release = current.clone();
        let planned_for_release = planned.clone();
        let parent_for_release = parent;
        let selector_for_release = selector.clone();
        let mut preflight_observation = Some(preflight_observation);
        Ok(operations.supervise_recovered_consultation_turn(
            task_limits,
            observed_at,
            current,
            executor,
            move |_, _| {
                ResidentConsultationSupervisionDecision::Continue(Some(Box::new(candidate)))
            },
            move |_| {
                Ok(preflight_observation
                    .take()
                    .expect("one candidate recovery"))
            },
            move |current_candidate, _| {
                let prepared =
                    match ResidentProcessHost::<()>::prepare_consultation_child_native_work_in(
                        application,
                        recovered,
                        ResidentDeploymentOperationAccess {
                            store,
                            custody: &mut *custody,
                            sessions,
                            anchor: &mut *anchor,
                        },
                        resolution,
                        basis,
                        authority,
                        &mut admit_rotation,
                        &mut sign_observation,
                        &mut sign_checkpoint,
                    ) {
                        Ok(prepared) => prepared,
                        Err(reason) => {
                            return ConsultationChildRetentionOutcome::Unopened(
                                ResidentProcessConsultNativeChildRetentionFailure::Native(reason),
                            );
                        }
                    };
                let dispatched =
                    ResidentProcessHost::<()>::classify_consultation_child_operation_in(
                        application,
                        &selector_for_release,
                        ResidentOperationControlAssociation {
                            current_request_authority,
                            historical_handle_authority,
                            recovery_store,
                            checkpoint_authority,
                        },
                        ResidentDeploymentOperationAccess {
                            store,
                            custody: &mut *custody,
                            sessions,
                            anchor: &mut *anchor,
                        },
                        restart(),
                        admission,
                        parent_for_release,
                        &current_for_release,
                        &planned_for_release,
                        current_candidate,
                        None,
                        observed_at,
                    );
                let dispatched = match dispatched {
                    Ok(ResidentProcessOperationRestartDisposition::Native(dispatched)) => {
                        dispatched.start.control.into_parts().1
                    }
                    Ok(_) => {
                        return ConsultationChildRetentionOutcome::Uncertain(
                            ResidentProcessConsultNativeChildRetentionFailure::Recovery(
                                ResidentProcessOperationRestartError::StartEvidence,
                            ),
                        );
                    }
                    Err(reason) => {
                        return ConsultationChildRetentionOutcome::Uncertain(
                            ResidentProcessConsultNativeChildRetentionFailure::Recovery(reason),
                        );
                    }
                };
                ConsultationChildRetentionOutcome::Retained {
                    child: Box::new(dispatched),
                    work: prepared,
                }
            },
        ))
    }
}

impl<WriterGuard> ResidentProcessHost<'_, WriterGuard> {
    /// Retain one protected Consult child at native dispatch without running it.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_consultation_child_native_work_in<
        'operation,
        'call,
        'registry,
        Authority,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        application: &'operation ResidentApplication<'_>,
        recovered: &'operation ResidentProcessRecoveredOperationResolution,
        mut access: ResidentDeploymentOperationAccess<'_>,
        resolution: ResidentProcessOperationResolutionInput<'operation>,
        basis: ResidentProcessNativeInvokeBasis<'operation, 'call, 'registry>,
        authority: &mut Authority,
        mut admit_rotation: Rotation,
        mut sign_observation: ObservationSign,
        mut sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentProcessConsultNativeChildWork<'operation, 'call, 'registry>,
        ResidentProcessSelfContainedNativeInvokeFailure<'operation, 'registry>,
    >
    where
        'operation: 'call,
        Authority: super::super::AmbientSessionAuthorityPort,
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        if recovered.start.lineage.consult_child().is_none() {
            return Err(ResidentProcessSelfContainedNativeInvokeFailure::Resolution(
                ResidentProcessOperationResolutionFailure::Selection(
                    ResidentProcessHostSelectionError::ConsultationChildAssociation,
                ),
            ));
        }
        let signing = resolution.signing;
        let maximum_record_bytes = resolution.maximum_record_bytes;
        if recovered.phase != ResidentProcessOperationResolutionPhase::Resolving {
            return Err(ResidentProcessSelfContainedNativeInvokeFailure::Native(
                NativeOperationInvokeFailure::Admission(OperationAttemptError::Association),
            ));
        }
        let operation = recovered.start.control.operation().operation();
        if !operation
            .historical_operation_lease()
            .evidence_obligations()
            .is_empty()
        {
            return Err(ResidentProcessSelfContainedNativeInvokeFailure::Native(
                NativeOperationInvokeFailure::Admission(OperationAttemptError::ContextRequired),
            ));
        }
        Self::validate_operation_resolution_in(
            application,
            &recovered.start.selector,
            recovered,
            &mut access,
            resolution.session,
            resolution.current,
            resolution.signing,
            resolution.maximum_record_bytes,
            true,
            authority,
        )
        .map_err(ResidentProcessSelfContainedNativeInvokeFailure::Resolution)?;
        let generation = application
            .generation(&recovered.start.generation_digest)
            .ok_or_else(|| {
                ResidentProcessSelfContainedNativeInvokeFailure::Resolution(
                    ResidentProcessOperationResolutionFailure::Selection(
                        ResidentProcessHostSelectionError::OperationResolutionOperationAuthority,
                    ),
                )
            })?;
        let predecessor = recovered.start.control.observation().selected_head();
        let checkpoint = recovered.start.control.observation().current_checkpoint();
        validate_attempt_refs(
            operation,
            predecessor,
            checkpoint,
            &resolution.selection_ref,
            &resolution.observation_ref,
            resolution.maximum_selection_bytes,
        )
        .map_err(|reason| {
            ResidentProcessSelfContainedNativeInvokeFailure::Native(
                NativeOperationInvokeFailure::Admission(reason),
            )
        })?;
        let context = OperationAttemptContext {
            operation,
            request: recovered.start.control.operation().request(),
            session: resolution.session,
            current: resolution.current,
            lease: operation.historical_operation_lease(),
            context_binding: None,
            generation,
            selection_ref: resolution.selection_ref,
            maximum_selection_bytes: resolution.maximum_selection_bytes,
            observation_ref: resolution.observation_ref,
            observation: OperationObservationAdmissionContextV1 {
                signed: resolution.signing,
                handle_ref: operation.handle().0,
                handle: operation.handle().1,
                predecessor: Some(predecessor),
            },
        };
        let input = NativeOperationInvokeInput {
            attempt: NativeOperationAttemptInput {
                context,
                registry: basis.registry,
                attempt_id: basis.attempt_id,
                result_admission: basis.result_admission,
            },
            checkpoint,
            checkpoint_schema: basis.checkpoint_schema,
            checkpoint_authority: basis.checkpoint_authority,
            signing,
            preparation_checkpoint: basis.preparation_checkpoint,
            dispatch_checkpoint: basis.dispatch_checkpoint,
            outcome_checkpoint: basis.outcome_checkpoint,
            terminal_checkpoint: basis.terminal_checkpoint,
            dispatch_ref: basis.dispatch_ref,
            outcome_ref: basis.outcome_ref,
            outcome_observation_ref: basis.outcome_observation_ref,
            result_ref: basis.result_ref,
            diagnostic_ref: basis.diagnostic_ref,
            terminal_ref: basis.terminal_ref,
            clock: basis.clock,
            resolve_store: basis.resolve_store,
            maximum_record_bytes,
            maximum_diagnostic_bytes: basis.maximum_diagnostic_bytes,
            maximum_outcome_bytes: basis.maximum_outcome_bytes,
            accumulator_limits: basis.accumulator_limits,
        };
        let selected_store = application.store();
        if selected_store != access.store {
            return Err(ResidentProcessSelfContainedNativeInvokeFailure::Resolution(
                ResidentProcessOperationResolutionFailure::Selection(
                    ResidentProcessHostSelectionError::AccessStoreAssociation,
                ),
            ));
        }
        validate_native_invoke_associations(
            &recovered.start.selector,
            &recovered.start.lineage,
            selected_store,
            &input,
        )
        .map_err(|reason| {
            ResidentProcessSelfContainedNativeInvokeFailure::Resolution(
                ResidentProcessOperationResolutionFailure::Selection(reason),
            )
        })?;
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        let prepared = prepare_native_operation_work(
            input,
            authority,
            custody,
            anchor,
            &mut admit_rotation,
            &mut sign_observation,
            &mut sign_checkpoint,
        )
        .map_err(ResidentProcessSelfContainedNativeInvokeFailure::Native)?;
        let (dispatch_ref, dispatch) = prepared.dispatch();
        Ok(ResidentProcessConsultNativeChildWork {
            store: selected_store.clone(),
            lineage: recovered.start.lineage.clone(),
            dispatch_ref: dispatch_ref.clone(),
            dispatch: dispatch.clone(),
            prepared: Some(Box::new(prepared)),
        })
    }

    /// Execute one task-owned Consult child without constructing another call.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn execute_consultation_child_native_work<
        'operation,
        'call,
        'registry,
        Authority,
        Rotation,
        ObservationSign,
        CheckpointSign,
        ResultSign,
    >(
        work: ResidentProcessConsultNativeChildWork<'operation, 'call, 'registry>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
        sign_result: ResultSign,
    ) -> (
        ResidentProcessConsultNativeChildWork<'operation, 'call, 'registry>,
        ResidentProcessConsultNativeChildOutput<'operation, 'registry>,
    )
    where
        Authority: super::super::AmbientSessionAuthorityPort,
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ResultSign: FnMut(
            &[u8],
            &pale_core::records::CapabilityResultPreimageV1,
            &pale_core::records::CapabilityResultDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let mut work = work;
        if &work.store != access.store {
            return (
                work,
                Err(ResidentProcessSelfContainedNativeInvokeFailure::Resolution(
                    ResidentProcessOperationResolutionFailure::Selection(
                        ResidentProcessHostSelectionError::AccessStoreAssociation,
                    ),
                )),
            );
        }
        debug_assert!(work.lineage.consult_child().is_some());
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        let Some(prepared) = work.prepared.take() else {
            return (
                work,
                Err(ResidentProcessSelfContainedNativeInvokeFailure::Native(
                    NativeOperationInvokeFailure::Admission(OperationAttemptError::Association),
                )),
            );
        };
        let completed = execute_native_operation_work(
            *prepared,
            authority,
            custody,
            anchor,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
            sign_result,
        )
        .map_err(ResidentProcessSelfContainedNativeInvokeFailure::Native);
        (work, completed)
    }

    /// Progress one protected self-contained Invoke through the native state machine.
    ///
    /// The recovered resolving head supplies all historical meaning. This method
    /// adds only fresh attempt and recording identities, then returns every
    /// stopped native phase unchanged. It owns no retry or background task.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn invoke_self_contained_native<
        'operation,
        'call,
        'registry,
        Authority,
        Rotation,
        ObservationSign,
        CheckpointSign,
        ResultSign,
    >(
        &'operation self,
        recovered: &'operation ResidentProcessRecoveredOperationResolution,
        mut access: ResidentDeploymentOperationAccess<'_>,
        resolution: ResidentProcessOperationResolutionInput<'operation>,
        basis: ResidentProcessNativeInvokeBasis<'operation, 'call, 'registry>,
        authority: &mut Authority,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
        sign_result: ResultSign,
    ) -> Result<
        CompletedNativeOperation,
        ResidentProcessSelfContainedNativeInvokeFailure<'operation, 'registry>,
    >
    where
        'operation: 'call,
        Authority: super::super::AmbientSessionAuthorityPort,
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ResultSign: FnMut(
            &[u8],
            &pale_core::records::CapabilityResultPreimageV1,
            &pale_core::records::CapabilityResultDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let signing = resolution.signing;
        let maximum_record_bytes = resolution.maximum_record_bytes;
        let (context, checkpoint) = self
            .admit_self_contained_invoke_attempt(recovered, &mut access, resolution, authority)
            .map_err(ResidentProcessSelfContainedNativeInvokeFailure::Resolution)?;
        let input = NativeOperationInvokeInput {
            attempt: NativeOperationAttemptInput {
                context,
                registry: basis.registry,
                attempt_id: basis.attempt_id,
                result_admission: basis.result_admission,
            },
            checkpoint,
            checkpoint_schema: basis.checkpoint_schema,
            checkpoint_authority: basis.checkpoint_authority,
            signing,
            preparation_checkpoint: basis.preparation_checkpoint,
            dispatch_checkpoint: basis.dispatch_checkpoint,
            outcome_checkpoint: basis.outcome_checkpoint,
            terminal_checkpoint: basis.terminal_checkpoint,
            dispatch_ref: basis.dispatch_ref,
            outcome_ref: basis.outcome_ref,
            outcome_observation_ref: basis.outcome_observation_ref,
            result_ref: basis.result_ref,
            diagnostic_ref: basis.diagnostic_ref,
            terminal_ref: basis.terminal_ref,
            clock: basis.clock,
            resolve_store: basis.resolve_store,
            maximum_record_bytes,
            maximum_diagnostic_bytes: basis.maximum_diagnostic_bytes,
            maximum_outcome_bytes: basis.maximum_outcome_bytes,
            accumulator_limits: basis.accumulator_limits,
        };
        self.invoke_native_with_lineage(
            &recovered.start.selector,
            &recovered.start.lineage,
            access,
            authority,
            input,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
            sign_result,
        )
        .map_err(|reason| {
            ResidentProcessSelfContainedNativeInvokeFailure::Resolution(
                ResidentProcessOperationResolutionFailure::Selection(reason),
            )
        })?
        .map_err(ResidentProcessSelfContainedNativeInvokeFailure::Native)
    }

    /// Run the existing native service after selecting its exact deployment.
    ///
    /// Selection finishes before custody, anchor, authority, clock, signing, or
    /// handler work. The inner result is the existing service result unchanged,
    /// including every post-handler continuation that prohibits another call.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn invoke_native<
        'operation,
        'call,
        'registry,
        Authority,
        Rotation,
        ObservationSign,
        CheckpointSign,
        ResultSign,
    >(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: NativeOperationInvokeInput<'operation, 'call, 'registry>,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
        sign_result: ResultSign,
    ) -> Result<
        Result<CompletedNativeOperation, NativeOperationInvokeFailure<'operation, 'registry>>,
        ResidentProcessHostSelectionError,
    >
    where
        Authority: super::super::AmbientSessionAuthorityPort,
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ResultSign: FnMut(
            &[u8],
            &pale_core::records::CapabilityResultPreimageV1,
            &pale_core::records::CapabilityResultDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.invoke_native_with_lineage(
            selector,
            &ResidentProcessOperationLineage::Root,
            access,
            authority,
            input,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
            sign_result,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn invoke_native_with_lineage<
        'operation,
        'call,
        'registry,
        Authority,
        Rotation,
        ObservationSign,
        CheckpointSign,
        ResultSign,
    >(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        lineage: &ResidentProcessOperationLineage,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: NativeOperationInvokeInput<'operation, 'call, 'registry>,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
        sign_result: ResultSign,
    ) -> Result<
        Result<CompletedNativeOperation, NativeOperationInvokeFailure<'operation, 'registry>>,
        ResidentProcessHostSelectionError,
    >
    where
        Authority: super::super::AmbientSessionAuthorityPort,
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ResultSign: FnMut(
            &[u8],
            &pale_core::records::CapabilityResultPreimageV1,
            &pale_core::records::CapabilityResultDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let index = self.deployment_authority_index(selector)?;
        let selected_store = self.deployments[index].store();
        if selected_store != access.store {
            return Err(ResidentProcessHostSelectionError::AccessStoreAssociation);
        }
        validate_native_invoke_associations(selector, lineage, selected_store, &input)?;
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        Ok(invoke_native_operation(
            input,
            authority,
            custody,
            anchor,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
            sign_result,
        ))
    }

    /// Retry one fixed post-handler stage preparation under the same deployment.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn continue_native_stage_preparation<Rotation, ObservationSign>(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        continuation: NativeOperationStagePreparationContinuation,
        signing: pale_core::records::SignedResidentAdmissionContextV1<'_>,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
    ) -> Result<
        Result<PreparedNativeOperationStageCustody, NativeOperationRecordingFailure>,
        NativeContinuationSelectionFailure<NativeOperationStagePreparationContinuation>,
    >
    where
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.continue_native_stage_preparation_with_lineage(
            selector,
            &ResidentProcessOperationLineage::Root,
            access,
            continuation,
            signing,
            admit_rotation,
            sign_observation,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn continue_native_stage_preparation_with_lineage<Rotation, ObservationSign>(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        lineage: &ResidentProcessOperationLineage,
        access: ResidentDeploymentOperationAccess<'_>,
        continuation: NativeOperationStagePreparationContinuation,
        signing: pale_core::records::SignedResidentAdmissionContextV1<'_>,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
    ) -> Result<
        Result<PreparedNativeOperationStageCustody, NativeOperationRecordingFailure>,
        NativeContinuationSelectionFailure<NativeOperationStagePreparationContinuation>,
    >
    where
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let store = match self.select_native_continuation_store(selector, &access) {
            Ok(store) => store,
            Err(reason) => {
                return Err(NativeContinuationSelectionFailure {
                    continuation: Box::new(continuation),
                    reason,
                });
            }
        };
        if let Err(reason) =
            validate_native_stage_preparation(selector, lineage, store, &continuation, signing)
        {
            return Err(NativeContinuationSelectionFailure {
                continuation: Box::new(continuation),
                reason,
            });
        }
        Ok(continuation.continue_with(signing, admit_rotation, sign_observation))
    }

    /// Continue fixed checkpoint preparation and custody for one signed stage.
    pub(super) fn continue_native_prepared_stage<CheckpointSign>(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        continuation: PreparedNativeOperationStageCustody,
        resolve_store: &dyn super::super::OperationObservationAuthorityStoreResolver,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        Result<NativeOperationStageFinalization, NativeOperationRecordingFailure>,
        NativeContinuationSelectionFailure<PreparedNativeOperationStageCustody>,
    >
    where
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.continue_native_prepared_stage_with_lineage(
            selector,
            &ResidentProcessOperationLineage::Root,
            access,
            continuation,
            resolve_store,
            sign_checkpoint,
        )
    }

    pub(super) fn continue_native_prepared_stage_with_lineage<CheckpointSign>(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        lineage: &ResidentProcessOperationLineage,
        access: ResidentDeploymentOperationAccess<'_>,
        continuation: PreparedNativeOperationStageCustody,
        resolve_store: &dyn super::super::OperationObservationAuthorityStoreResolver,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        Result<NativeOperationStageFinalization, NativeOperationRecordingFailure>,
        NativeContinuationSelectionFailure<PreparedNativeOperationStageCustody>,
    >
    where
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let store = match self.select_native_continuation_store(selector, &access) {
            Ok(store) => store,
            Err(reason) => {
                return Err(NativeContinuationSelectionFailure {
                    continuation: Box::new(continuation),
                    reason,
                });
            }
        };
        if let Err(reason) = validate_native_prepared_stage(selector, lineage, store, &continuation)
        {
            return Err(NativeContinuationSelectionFailure {
                continuation: Box::new(continuation),
                reason,
            });
        }
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        Ok(continuation.continue_custody(resolve_store, custody, anchor, sign_checkpoint))
    }

    /// Attach and retain one checkpoint that was already signed exactly once.
    pub(super) fn continue_native_issued_stage(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        continuation: IssuedNativeOperationStageCheckpoint,
        resolve_store: &dyn super::super::OperationObservationAuthorityStoreResolver,
    ) -> Result<
        Result<NativeOperationStageFinalization, NativeOperationRecordingFailure>,
        NativeContinuationSelectionFailure<IssuedNativeOperationStageCheckpoint>,
    > {
        self.continue_native_issued_stage_with_lineage(
            selector,
            &ResidentProcessOperationLineage::Root,
            access,
            continuation,
            resolve_store,
        )
    }

    pub(super) fn continue_native_issued_stage_with_lineage(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        lineage: &ResidentProcessOperationLineage,
        access: ResidentDeploymentOperationAccess<'_>,
        continuation: IssuedNativeOperationStageCheckpoint,
        resolve_store: &dyn super::super::OperationObservationAuthorityStoreResolver,
    ) -> Result<
        Result<NativeOperationStageFinalization, NativeOperationRecordingFailure>,
        NativeContinuationSelectionFailure<IssuedNativeOperationStageCheckpoint>,
    > {
        let store = match self.select_native_continuation_store(selector, &access) {
            Ok(store) => store,
            Err(reason) => {
                return Err(NativeContinuationSelectionFailure {
                    continuation: Box::new(continuation),
                    reason,
                });
            }
        };
        if let Err(reason) = validate_native_issued_stage(selector, lineage, store, &continuation) {
            return Err(NativeContinuationSelectionFailure {
                continuation: Box::new(continuation),
                reason,
            });
        }
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        Ok(continuation.continue_custody(resolve_store, custody, anchor))
    }

    /// Read back one exact stage candidate after an ambiguous database commit.
    pub(super) fn reconcile_native_stage_commit(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        continuation: NativeOperationStagingCommitFailure,
    ) -> Result<
        Result<NativeOperationStagingAnchorPending, NativeOperationStagingCommitFailure>,
        NativeContinuationSelectionFailure<NativeOperationStagingCommitFailure>,
    > {
        self.reconcile_native_stage_commit_with_lineage(
            selector,
            &ResidentProcessOperationLineage::Root,
            access,
            continuation,
        )
    }

    pub(super) fn reconcile_native_stage_commit_with_lineage(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        lineage: &ResidentProcessOperationLineage,
        access: ResidentDeploymentOperationAccess<'_>,
        continuation: NativeOperationStagingCommitFailure,
    ) -> Result<
        Result<NativeOperationStagingAnchorPending, NativeOperationStagingCommitFailure>,
        NativeContinuationSelectionFailure<NativeOperationStagingCommitFailure>,
    > {
        let store = match self.select_native_continuation_store(selector, &access) {
            Ok(store) => store,
            Err(reason) => {
                return Err(NativeContinuationSelectionFailure {
                    continuation: Box::new(continuation),
                    reason,
                });
            }
        };
        if let Err(reason) = validate_native_committed_stage(
            selector,
            lineage,
            store,
            continuation.operation(),
            continuation.observation().1,
            continuation.checkpoint().1,
        ) {
            return Err(NativeContinuationSelectionFailure {
                continuation: Box::new(continuation),
                reason,
            });
        }
        Ok(continuation.reconcile_exact(access.custody))
    }

    /// Continue only the retained independent-anchor transition for one stage.
    pub(super) fn continue_native_stage_anchor(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        continuation: NativeOperationStagingAnchorPending,
    ) -> Result<
        Result<NativeOperationStageFinalization, NativeOperationStagingAnchorPending>,
        NativeContinuationSelectionFailure<NativeOperationStagingAnchorPending>,
    > {
        self.continue_native_stage_anchor_with_lineage(
            selector,
            &ResidentProcessOperationLineage::Root,
            access,
            continuation,
        )
    }

    pub(super) fn continue_native_stage_anchor_with_lineage(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        lineage: &ResidentProcessOperationLineage,
        access: ResidentDeploymentOperationAccess<'_>,
        continuation: NativeOperationStagingAnchorPending,
    ) -> Result<
        Result<NativeOperationStageFinalization, NativeOperationStagingAnchorPending>,
        NativeContinuationSelectionFailure<NativeOperationStagingAnchorPending>,
    > {
        let store = match self.select_native_continuation_store(selector, &access) {
            Ok(store) => store,
            Err(reason) => {
                return Err(NativeContinuationSelectionFailure {
                    continuation: Box::new(continuation),
                    reason,
                });
            }
        };
        if let Err(reason) = validate_native_committed_stage(
            selector,
            lineage,
            store,
            continuation.operation(),
            continuation.observation().1,
            continuation.checkpoint().1,
        ) {
            return Err(NativeContinuationSelectionFailure {
                continuation: Box::new(continuation),
                reason,
            });
        }
        Ok(continuation.continue_anchor(access.anchor))
    }

    /// Admit the exact completed stage or fail closed without another effect path.
    pub(super) fn admit_native_stage(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        finalization: NativeOperationStageFinalization,
    ) -> Result<
        Result<AdmittedStagedNativeOperationOutcome, OperationAttemptError>,
        NativeContinuationSelectionFailure<NativeOperationStageFinalization>,
    > {
        self.admit_native_stage_with_lineage(
            selector,
            &ResidentProcessOperationLineage::Root,
            access,
            finalization,
        )
    }

    pub(super) fn admit_native_stage_with_lineage(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        lineage: &ResidentProcessOperationLineage,
        access: ResidentDeploymentOperationAccess<'_>,
        finalization: NativeOperationStageFinalization,
    ) -> Result<
        Result<AdmittedStagedNativeOperationOutcome, OperationAttemptError>,
        NativeContinuationSelectionFailure<NativeOperationStageFinalization>,
    > {
        let store = match self.select_native_continuation_store(selector, &access) {
            Ok(store) => store,
            Err(reason) => {
                return Err(NativeContinuationSelectionFailure {
                    continuation: Box::new(finalization),
                    reason,
                });
            }
        };
        if let Err(reason) = validate_native_committed_stage(
            selector,
            lineage,
            store,
            finalization.operation(),
            finalization.observation().1,
            finalization.checkpoint().1,
        ) {
            return Err(NativeContinuationSelectionFailure {
                continuation: Box::new(finalization),
                reason,
            });
        }
        Ok(finalization
            .finish()
            .map_err(|failure| failure.into_reason()))
    }

    fn select_native_continuation_store<'a>(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: &ResidentDeploymentOperationAccess<'a>,
    ) -> Result<&AuthorityStoreCoordinate, ResidentProcessHostSelectionError> {
        let index = self.deployment_authority_index(selector)?;
        let selected = self.deployments[index].store();
        if selected != access.store {
            return Err(ResidentProcessHostSelectionError::AccessStoreAssociation);
        }
        Ok(selected)
    }

    /// Recompute one protected native dispatch after current checks.
    ///
    /// An already-staged outcome and a cancellation predecessor are rejected
    /// before the handler. A post-handler recording failure returns its exact
    /// recording continuation. That continuation cannot reach the handler,
    /// but the caller retains the borrowed recovered attempt and this method
    /// does not establish process-wide exactly-once execution.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn recover_native<
        'operation,
        'call,
        'registry,
        Authority,
        Rotation,
        ObservationSign,
        CheckpointSign,
        ResultSign,
    >(
        &'operation self,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: ResidentProcessNativeRecoveryInput<'operation, 'call, 'registry>,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
        sign_result: ResultSign,
    ) -> Result<CompletedNativeOperation, ResidentProcessNativeRecoveryFailure>
    where
        Authority: super::super::AmbientSessionAuthorityPort,
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnMut(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ResultSign: FnMut(
            &[u8],
            &pale_core::records::CapabilityResultPreimageV1,
            &pale_core::records::CapabilityResultDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let attempt = input.attempt;
        let selector = &attempt.start.selector;
        let index = self
            .deployment_authority_index(selector)
            .map_err(ResidentProcessNativeRecoveryFailure::Selection)?;
        let application = self.deployments[index].application.as_ref();
        let selected_store = application.store();
        if access.store != selected_store {
            return Err(ResidentProcessNativeRecoveryFailure::Selection(
                ResidentProcessHostSelectionError::AccessStoreAssociation,
            ));
        }
        let generation = application
            .generation(&attempt.start.generation_digest)
            .ok_or(ResidentProcessNativeRecoveryFailure::Admission(
                OperationAttemptError::Association,
            ))?;
        validate_native_recovery_associations(selector, selected_store, &input)
            .map_err(ResidentProcessNativeRecoveryFailure::Selection)?;
        if attempt.dispatch == ResidentProcessAttemptObservation::CancellationPredecessor {
            return if has_staged_native_outcome(attempt.start.control.observation())
                .map_err(ResidentProcessNativeRecoveryFailure::Admission)?
            {
                Err(ResidentProcessNativeRecoveryFailure::StagedOutcomeRequired)
            } else {
                Err(ResidentProcessNativeRecoveryFailure::CancellationRequested)
            };
        }
        require_original_native_dispatch(attempt)?;
        validate_native_recovery_references(&input)
            .map_err(ResidentProcessNativeRecoveryFailure::Admission)?;

        let recovered = &attempt.start.control;
        let operation = recovered.operation();
        let history = OperationAttemptHistoryInput {
            operation: operation.operation(),
            request: operation.request(),
            session: operation.operation().historical_session_lease(),
            lease: operation.operation().historical_operation_lease(),
            generation,
            dispatched: recovered.observation().selected().1,
            selection: &attempt.selection,
            maximum_selection_bytes: input.maximum_selection_bytes,
        };
        let dispatch = crate::resident::operation_dispatch::recover_native_operation_dispatch(
            history,
            recovered.observation(),
            input.registry,
            input.contract_set,
        )
        .map_err(ResidentProcessNativeRecoveryFailure::Admission)?;
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        let outcome = dispatch
            .recompute(
                NativeOperationRecoveryContext {
                    current: input.current,
                    clock: input.recording.clock,
                    maximum_record_bytes: input.maximum_record_bytes,
                },
                authority,
                custody,
                anchor,
            )
            .map_err(ResidentProcessNativeRecoveryFailure::Admission)?;
        let checkpoint = recovered.observation().current_checkpoint();
        let recording = input.recording;
        record_native_operation_outcome(
            outcome,
            NativeOperationOutcomeRecordingInput {
                checkpoint,
                outcome_ref: recording.outcome_ref,
                outcome_observation_ref: recording.outcome_observation_ref,
                result_ref: recording.result_ref,
                diagnostic_ref: recording.diagnostic_ref,
                terminal_ref: recording.terminal_ref,
                outcome_checkpoint: recording.outcome_checkpoint,
                terminal_checkpoint: recording.terminal_checkpoint,
                clock: recording.clock,
                resolve_store: recording.resolve_store,
                checkpoint_schema: recording.checkpoint_schema,
                checkpoint_authority: recording.checkpoint_authority,
                signing: recording.signing,
                maximum_diagnostic_bytes: recording.maximum_diagnostic_bytes,
                maximum_outcome_bytes: recording.maximum_outcome_bytes,
                accumulator_limits: recording.accumulator_limits,
            },
            custody,
            anchor,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
            sign_result,
        )
        .map_err(ResidentProcessNativeRecoveryFailure::Recording)
    }

    /// Finish one staged native outcome without running or staging it again.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn recover_staged_native<
        'operation,
        'call,
        'registry,
        Rotation,
        ObservationSign,
        CheckpointSign,
        ResultSign,
    >(
        &'operation self,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessStagedNativeRecoveryInput<'operation, 'call, 'registry>,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
        sign_result: ResultSign,
    ) -> Result<CompletedNativeOperation, ResidentProcessStagedNativeRecoveryFailure>
    where
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ResultSign: FnOnce(
            &[u8],
            &pale_core::records::CapabilityResultPreimageV1,
            &pale_core::records::CapabilityResultDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let attempt = input.attempt;
        let selector = &attempt.start.selector;
        let index = self
            .deployment_authority_index(selector)
            .map_err(ResidentProcessStagedNativeRecoveryFailure::Selection)?;
        let application = self.deployments[index].application.as_ref();
        let selected_store = application.store();
        if access.store != selected_store {
            return Err(ResidentProcessStagedNativeRecoveryFailure::Selection(
                ResidentProcessHostSelectionError::AccessStoreAssociation,
            ));
        }
        let generation = application
            .generation(&attempt.start.generation_digest)
            .ok_or(ResidentProcessStagedNativeRecoveryFailure::Admission(
                OperationAttemptError::Association,
            ))?;
        validate_staged_native_recovery_associations(selector, selected_store, &input)
            .map_err(ResidentProcessStagedNativeRecoveryFailure::Selection)?;
        require_staged_native_dispatch(attempt)
            .map_err(ResidentProcessStagedNativeRecoveryFailure::Admission)?;

        let recovered = &attempt.start.control;
        let operation = recovered.operation();
        let history = OperationAttemptHistoryInput {
            operation: operation.operation(),
            request: operation.request(),
            session: operation.operation().historical_session_lease(),
            lease: operation.operation().historical_operation_lease(),
            generation,
            dispatched: staged_native_dispatch(recovered.observation())
                .map_err(ResidentProcessStagedNativeRecoveryFailure::Admission)?
                .1,
            selection: &attempt.selection,
            maximum_selection_bytes: input.maximum_selection_bytes,
        };
        let ResidentDeploymentOperationAccess {
            custody, anchor, ..
        } = access;
        let phase = recover_native_operation_outcome_phase(
            NativeOperationOutcomeRecoveryInput {
                history,
                maximum_outcome_bytes: input.maximum_outcome_bytes,
            },
            recovered.observation(),
            input.registry,
            input.contract_set,
            custody,
        )
        .map_err(ResidentProcessStagedNativeRecoveryFailure::Admission)?;
        if !selector.matches_signing_authority(phase.result_authority()) {
            return Err(ResidentProcessStagedNativeRecoveryFailure::Selection(
                ResidentProcessHostSelectionError::NativeSigningAuthority,
            ));
        }
        validate_staged_native_recovery_references(&input, phase.outcome_coordinate())
            .map_err(ResidentProcessStagedNativeRecoveryFailure::Admission)?;
        crate::resident::operation_dispatch::verify_recovered_operation_head_custody(
            operation.operation(),
            recovered.observation(),
            custody,
            anchor,
            input.maximum_record_bytes,
        )
        .map_err(ResidentProcessStagedNativeRecoveryFailure::Admission)?;

        let checkpoint = recovered.observation().current_checkpoint();
        let recording = input.recording;
        record_recovered_native_operation_outcome(
            phase,
            recovered.observation(),
            checkpoint,
            NativeOperationTerminalRecordingInput {
                result_ref: recording.result_ref,
                diagnostic_ref: recording.diagnostic_ref,
                terminal_ref: recording.terminal_ref,
                terminal_checkpoint: recording.terminal_checkpoint,
                clock: recording.clock,
                resolve_store: recording.resolve_store,
                checkpoint_schema: recording.checkpoint_schema,
                checkpoint_authority: recording.checkpoint_authority,
                signing: recording.signing,
                maximum_diagnostic_bytes: recording.maximum_diagnostic_bytes,
                accumulator_limits: recording.accumulator_limits,
            },
            custody,
            anchor,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
            sign_result,
        )
        .map_err(ResidentProcessStagedNativeRecoveryFailure::Recording)
    }
}

fn require_original_native_dispatch(
    attempt: &ResidentProcessRecoveredAttempt,
) -> Result<(), ResidentProcessNativeRecoveryFailure> {
    let recovered = attempt.start.control.observation();
    let selected = recovered.selected();
    if selected != recovered.selected_head() {
        return Err(ResidentProcessNativeRecoveryFailure::Admission(
            OperationAttemptError::Association,
        ));
    }
    let Some(predecessor) = recovered.selected_predecessor() else {
        return Err(ResidentProcessNativeRecoveryFailure::Admission(
            OperationAttemptError::Association,
        ));
    };
    if has_staged_native_outcome(recovered)
        .map_err(ResidentProcessNativeRecoveryFailure::Admission)?
    {
        return Err(ResidentProcessNativeRecoveryFailure::StagedOutcomeRequired);
    }
    match (selected.1.state(), predecessor.1.state()) {
        (
            OperationObservationStateV1::Active {
                phase:
                    OperationActivePhaseV1::Dispatched {
                        attempt_id,
                        evidence,
                    },
            },
            OperationObservationStateV1::Active {
                phase:
                    OperationActivePhaseV1::DispatchPrepared {
                        attempt_id: prepared_attempt,
                        evidence: prepared_evidence,
                    },
            },
        ) if attempt_id == prepared_attempt && evidence == prepared_evidence => Ok(()),
        _ => Err(ResidentProcessNativeRecoveryFailure::Admission(
            OperationAttemptError::Association,
        )),
    }
}

fn require_staged_native_dispatch(
    attempt: &ResidentProcessRecoveredAttempt,
) -> Result<(), OperationAttemptError> {
    let recovered = attempt.start.control.observation();
    let selected = recovered.selected();
    let head = recovered.selected_head();
    match attempt.dispatch {
        ResidentProcessAttemptObservation::CurrentHead if selected != head => {
            return Err(OperationAttemptError::Association);
        }
        ResidentProcessAttemptObservation::CancellationPredecessor
            if selected == head
                || !matches!(
                    head.1.state(),
                    OperationObservationStateV1::CancellationRequested { .. }
                ) =>
        {
            return Err(OperationAttemptError::Association);
        }
        _ => {}
    }
    let dispatch = recovered
        .selected_predecessor()
        .ok_or(OperationAttemptError::Association)?;
    let preparation = recovered
        .selected_predecessor_predecessor()
        .ok_or(OperationAttemptError::Association)?;
    let (
        OperationObservationStateV1::Active {
            phase:
                OperationActivePhaseV1::Dispatched {
                    attempt_id: stage_attempt,
                    evidence: stage_evidence,
                },
        },
        OperationObservationStateV1::Active {
            phase:
                OperationActivePhaseV1::Dispatched {
                    attempt_id: dispatch_attempt,
                    evidence: dispatch_evidence,
                },
        },
        OperationObservationStateV1::Active {
            phase:
                OperationActivePhaseV1::DispatchPrepared {
                    attempt_id: prepared_attempt,
                    evidence: prepared_evidence,
                },
        },
    ) = (
        selected.1.state(),
        dispatch.1.state(),
        preparation.1.state(),
    )
    else {
        return Err(OperationAttemptError::Association);
    };
    let (handle_ref, handle) = attempt.start.control.operation().operation().handle();
    if stage_attempt != dispatch_attempt
        || stage_attempt != prepared_attempt
        || stage_evidence != dispatch_evidence
        || stage_evidence != prepared_evidence
        || selected.0 == dispatch.0
        || dispatch.0 == preparation.0
        || selected
            .1
            .verify_admitted_successor_of(handle_ref, handle, dispatch.0, dispatch.1)
            .is_err()
        || dispatch
            .1
            .verify_admitted_successor_of(handle_ref, handle, preparation.0, preparation.1)
            .is_err()
    {
        return Err(OperationAttemptError::Association);
    }
    Ok(())
}

fn staged_native_dispatch(
    recovered: &crate::resident::RecoveredOperationObservation,
) -> Result<(&ResidentRecordRefV1, &OperationObservationV1), OperationAttemptError> {
    recovered
        .selected_predecessor()
        .ok_or(OperationAttemptError::Association)
}

fn has_staged_native_outcome(
    recovered: &crate::resident::RecoveredOperationObservation,
) -> Result<bool, OperationAttemptError> {
    let selected = recovered.selected();
    let Some(predecessor) = recovered.selected_predecessor() else {
        return Ok(false);
    };
    match (selected.1.state(), predecessor.1.state()) {
        (
            OperationObservationStateV1::Active {
                phase:
                    OperationActivePhaseV1::Dispatched {
                        attempt_id,
                        evidence,
                    },
            },
            OperationObservationStateV1::Active {
                phase:
                    OperationActivePhaseV1::Dispatched {
                        attempt_id: predecessor_attempt,
                        evidence: predecessor_evidence,
                    },
            },
        ) => {
            if attempt_id != predecessor_attempt || evidence != predecessor_evidence {
                return Err(OperationAttemptError::Association);
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn validate_native_recovery_associations(
    selector: &ResidentDeploymentAuthoritySelector,
    store: &pale_ports::AuthorityStoreCoordinate,
    input: &ResidentProcessNativeRecoveryInput<'_, '_, '_>,
) -> Result<(), ResidentProcessHostSelectionError> {
    let control = &input.attempt.start.control;
    let operation = control.operation();
    let issued = operation.operation();
    let (_, handle) = issued.handle();
    if operation.store() != store
        || control.observation().store() != store
        || !input
            .attempt
            .start
            .lineage
            .admits(issued.handle().0, handle)
        || !selector.matches_operation(issued)
    {
        return Err(ResidentProcessHostSelectionError::NativeOperationAuthority);
    }
    if !selector.matches_current_session(input.current)
        || !selector.matches_signing_authority(input.current.generation.signed.signing_authority)
    {
        return Err(ResidentProcessHostSelectionError::NativeCurrentAuthority);
    }
    let historical_session = issued.historical_session_lease();
    if issued.historical_operation_lease().operation_lease_digest()
        != handle.operation_lease_sha256()
        || historical_session.deployment_runtime_authority_ref() != &selector.runtime_authority_ref
        || historical_session
            .deployment_runtime_authority_sha256()
            .as_str()
            != selector.runtime_authority_sha256.as_str()
        || historical_session.issuer_principal_id() != &selector.principal_id
    {
        return Err(ResidentProcessHostSelectionError::HistoricalNativeAuthority);
    }
    let request = operation.request().to_preimage();
    let selected = control.observation().selected();
    let predecessor = control
        .observation()
        .selected_predecessor()
        .ok_or(ResidentProcessHostSelectionError::HistoricalNativeAuthority)?;
    if !selector.matches_signing_authority(&request.signing_authority_binding)
        || !native_observation_matches(selector, selected.1)
        || !native_observation_matches(selector, predecessor.1)
        || !selector.matches_signing_authority(input.recording.signing.signing_authority)
    {
        return Err(ResidentProcessHostSelectionError::NativeSigningAuthority);
    }
    let checkpoint = control.observation().current_checkpoint().1;
    if checkpoint.control_store_id() != store.store_id()
        || checkpoint.control_incarnation_id() != store.incarnation_id()
        || checkpoint.issuer_principal_id() != &selector.principal_id
        || !checkpoint_authority_matches(selector, store, input.recording.checkpoint_authority)
    {
        return Err(ResidentProcessHostSelectionError::NativeCheckpointAuthority);
    }
    Ok(())
}

fn validate_staged_native_recovery_associations(
    selector: &ResidentDeploymentAuthoritySelector,
    store: &pale_ports::AuthorityStoreCoordinate,
    input: &ResidentProcessStagedNativeRecoveryInput<'_, '_, '_>,
) -> Result<(), ResidentProcessHostSelectionError> {
    let control = &input.attempt.start.control;
    let operation = control.operation();
    let issued = operation.operation();
    let (_, handle) = issued.handle();
    if operation.store() != store
        || control.observation().store() != store
        || !input
            .attempt
            .start
            .lineage
            .admits(issued.handle().0, handle)
        || !selector.matches_operation(issued)
    {
        return Err(ResidentProcessHostSelectionError::NativeOperationAuthority);
    }
    if !selector.matches_current_session(input.current)
        || !selector.matches_signing_authority(input.current.generation.signed.signing_authority)
    {
        return Err(ResidentProcessHostSelectionError::NativeCurrentAuthority);
    }
    let historical_session = issued.historical_session_lease();
    if issued.historical_operation_lease().operation_lease_digest()
        != handle.operation_lease_sha256()
        || historical_session.deployment_runtime_authority_ref() != &selector.runtime_authority_ref
        || historical_session
            .deployment_runtime_authority_sha256()
            .as_str()
            != selector.runtime_authority_sha256.as_str()
        || historical_session.issuer_principal_id() != &selector.principal_id
    {
        return Err(ResidentProcessHostSelectionError::HistoricalNativeAuthority);
    }
    let request = operation.request().to_preimage();
    let recovered = control.observation();
    let selected = recovered.selected();
    let predecessor = recovered
        .selected_predecessor()
        .ok_or(ResidentProcessHostSelectionError::HistoricalNativeAuthority)?;
    let second_predecessor = recovered
        .selected_predecessor_predecessor()
        .ok_or(ResidentProcessHostSelectionError::HistoricalNativeAuthority)?;
    let head = recovered.selected_head();
    if !selector.matches_signing_authority(&request.signing_authority_binding)
        || !native_observation_matches(selector, selected.1)
        || !native_observation_matches(selector, predecessor.1)
        || !native_observation_matches(selector, second_predecessor.1)
        || !native_observation_matches(selector, head.1)
        || !selector.matches_signing_authority(input.recording.signing.signing_authority)
    {
        return Err(ResidentProcessHostSelectionError::NativeSigningAuthority);
    }
    let checkpoint = recovered.current_checkpoint().1;
    if checkpoint.control_store_id() != store.store_id()
        || checkpoint.control_incarnation_id() != store.incarnation_id()
        || checkpoint.issuer_principal_id() != &selector.principal_id
        || !checkpoint_authority_matches(selector, store, input.recording.checkpoint_authority)
    {
        return Err(ResidentProcessHostSelectionError::NativeCheckpointAuthority);
    }
    Ok(())
}

fn native_observation_matches(
    selector: &ResidentDeploymentAuthoritySelector,
    observation: &pale_core::records::OperationObservationV1,
) -> bool {
    observation.deployment_runtime_authority_ref() == &selector.runtime_authority_ref
        && observation.deployment_runtime_authority_sha256().as_str()
            == selector.runtime_authority_sha256.as_str()
        && observation.issuer_principal_id() == &selector.principal_id
}

fn validate_native_recovery_references(
    input: &ResidentProcessNativeRecoveryInput<'_, '_, '_>,
) -> Result<(), OperationAttemptError> {
    if input.maximum_selection_bytes == 0
        || input.maximum_record_bytes == 0
        || input.recording.maximum_diagnostic_bytes == 0
        || input.recording.maximum_outcome_bytes == 0
    {
        return Err(OperationAttemptError::BoundExceeded);
    }
    let recovered = &input.attempt.start.control;
    let operation = recovered.operation().operation();
    let observation = recovered.observation();
    let selected = observation.selected();
    let predecessor = observation
        .selected_predecessor()
        .ok_or(OperationAttemptError::Association)?;
    let checkpoint = observation.current_checkpoint();
    let planned = [
        &input.recording.outcome_ref,
        &input.recording.outcome_observation_ref,
        &input.recording.result_ref,
        &input.recording.diagnostic_ref,
        &input.recording.terminal_ref,
        &input.recording.outcome_checkpoint.reference,
        &input.recording.terminal_checkpoint.reference,
    ];
    let reserved = [
        operation.handle().0,
        operation.request_coordinate().record_ref(),
        operation.handle().1.operation_lease_ref(),
        selected.0,
        predecessor.0,
        checkpoint.0,
        input.attempt.selection.coordinate().record_ref(),
    ];
    if planned.iter().enumerate().any(|(index, reference)| {
        planned[..index].contains(reference) || reserved.contains(reference)
    }) || input.recording.outcome_checkpoint.id == input.recording.terminal_checkpoint.id
        || &input.recording.outcome_checkpoint.id == checkpoint.1.control_checkpoint_id()
        || &input.recording.terminal_checkpoint.id == checkpoint.1.control_checkpoint_id()
    {
        return Err(OperationAttemptError::Association);
    }
    Ok(())
}

fn validate_staged_native_recovery_references(
    input: &ResidentProcessStagedNativeRecoveryInput<'_, '_, '_>,
    outcome: &pale_ports::ControlRecordCoordinate,
) -> Result<(), OperationAttemptError> {
    if input.maximum_selection_bytes == 0
        || input.maximum_record_bytes == 0
        || input.maximum_outcome_bytes == 0
        || input.recording.maximum_diagnostic_bytes == 0
    {
        return Err(OperationAttemptError::BoundExceeded);
    }
    let recovered = input.attempt.start.control.observation();
    let operation = input.attempt.start.control.operation().operation();
    let selected = recovered.selected();
    let predecessor = recovered
        .selected_predecessor()
        .ok_or(OperationAttemptError::Association)?;
    let second_predecessor = recovered
        .selected_predecessor_predecessor()
        .ok_or(OperationAttemptError::Association)?;
    let head = recovered.selected_head();
    let current_checkpoint = recovered.current_checkpoint();
    let selected_checkpoint = recovered.selected_checkpoint_record();
    let head_checkpoint = recovered.selected_head_checkpoint_record();
    let cancellation_request_ref = match head.1.state() {
        OperationObservationStateV1::CancellationRequested { request } => Some(&request.record_ref),
        _ => None,
    };
    let planned = [
        &input.recording.result_ref,
        &input.recording.diagnostic_ref,
        &input.recording.terminal_ref,
        &input.recording.terminal_checkpoint.reference,
    ];
    let protected = [
        operation.handle().0,
        operation.request_coordinate().record_ref(),
        operation.handle().1.operation_lease_ref(),
        input.attempt.selection.coordinate().record_ref(),
        selected.0,
        predecessor.0,
        second_predecessor.0,
        head.0,
        current_checkpoint.0,
        selected_checkpoint.0,
        head_checkpoint.0,
        outcome.record_ref(),
    ];
    if planned.iter().enumerate().any(|(index, reference)| {
        planned[..index].contains(reference)
            || protected.contains(reference)
            || cancellation_request_ref.is_some_and(|existing| existing == *reference)
    }) || &input.recording.terminal_checkpoint.id == current_checkpoint.1.control_checkpoint_id()
        || &input.recording.terminal_checkpoint.id == selected_checkpoint.1.control_checkpoint_id()
    {
        return Err(OperationAttemptError::Association);
    }
    Ok(())
}

fn validate_native_stage_preparation(
    selector: &ResidentDeploymentAuthoritySelector,
    lineage: &ResidentProcessOperationLineage,
    store: &pale_ports::AuthorityStoreCoordinate,
    continuation: &NativeOperationStagePreparationContinuation,
    signing: pale_core::records::SignedResidentAdmissionContextV1<'_>,
) -> Result<(), ResidentProcessHostSelectionError> {
    validate_native_continuation_operation(selector, lineage, store, continuation.operation())?;
    if !selector.matches_signing_authority(continuation.signing_authority())
        || !selector.matches_signing_authority(signing.signing_authority)
    {
        return Err(ResidentProcessHostSelectionError::NativeSigningAuthority);
    }
    let checkpoint = continuation.predecessor_checkpoint().1;
    let authority = continuation.checkpoint_authority();
    validate_native_checkpoint(selector, store, checkpoint, &authority)
}

fn validate_native_prepared_stage(
    selector: &ResidentDeploymentAuthoritySelector,
    lineage: &ResidentProcessOperationLineage,
    store: &pale_ports::AuthorityStoreCoordinate,
    continuation: &PreparedNativeOperationStageCustody,
) -> Result<(), ResidentProcessHostSelectionError> {
    validate_native_continuation_operation(selector, lineage, store, continuation.operation())?;
    if !native_observation_matches(selector, continuation.observation().1) {
        return Err(ResidentProcessHostSelectionError::NativeSigningAuthority);
    }
    let checkpoint = continuation.predecessor_checkpoint().1;
    let authority = continuation.checkpoint_authority();
    validate_native_checkpoint(selector, store, checkpoint, &authority)
}

fn validate_native_issued_stage(
    selector: &ResidentDeploymentAuthoritySelector,
    lineage: &ResidentProcessOperationLineage,
    store: &pale_ports::AuthorityStoreCoordinate,
    continuation: &IssuedNativeOperationStageCheckpoint,
) -> Result<(), ResidentProcessHostSelectionError> {
    validate_native_committed_stage(
        selector,
        lineage,
        store,
        continuation.operation(),
        continuation.observation().1,
        continuation.checkpoint().1,
    )
}

fn validate_native_committed_stage(
    selector: &ResidentDeploymentAuthoritySelector,
    lineage: &ResidentProcessOperationLineage,
    store: &pale_ports::AuthorityStoreCoordinate,
    operation: &crate::resident::operation_dispatch::NativeOperationRecordingAuthority,
    observation: &OperationObservationV1,
    checkpoint: &pale_core::records::ControlStateCheckpointV1,
) -> Result<(), ResidentProcessHostSelectionError> {
    validate_native_continuation_operation(selector, lineage, store, operation)?;
    if !native_observation_matches(selector, observation) {
        return Err(ResidentProcessHostSelectionError::NativeSigningAuthority);
    }
    if checkpoint.control_store_id() != store.store_id()
        || checkpoint.control_incarnation_id() != store.incarnation_id()
        || checkpoint.issuer_principal_id() != &selector.principal_id
    {
        return Err(ResidentProcessHostSelectionError::NativeCheckpointAuthority);
    }
    Ok(())
}

fn validate_native_continuation_operation(
    selector: &ResidentDeploymentAuthoritySelector,
    lineage: &ResidentProcessOperationLineage,
    store: &pale_ports::AuthorityStoreCoordinate,
    operation: &crate::resident::operation_dispatch::NativeOperationRecordingAuthority,
) -> Result<(), ResidentProcessHostSelectionError> {
    let (handle_ref, handle) = operation.handle();
    let (historical_runtime_ref, historical_runtime_sha256, historical_principal_id) =
        operation.historical_runtime_authority();
    if !lineage.admits(handle_ref, handle)
        || !selector.matches_handle(handle)
        || !operation.checkpoint_selection().matches_store(store)
        || historical_runtime_ref != &selector.runtime_authority_ref
        || historical_runtime_sha256.as_str() != selector.runtime_authority_sha256.as_str()
        || historical_principal_id != &selector.principal_id
    {
        return Err(ResidentProcessHostSelectionError::HistoricalNativeAuthority);
    }
    Ok(())
}

fn validate_native_checkpoint(
    selector: &ResidentDeploymentAuthoritySelector,
    store: &pale_ports::AuthorityStoreCoordinate,
    checkpoint: &pale_core::records::ControlStateCheckpointV1,
    authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
) -> Result<(), ResidentProcessHostSelectionError> {
    if checkpoint.control_store_id() != store.store_id()
        || checkpoint.control_incarnation_id() != store.incarnation_id()
        || checkpoint.issuer_principal_id() != &selector.principal_id
        || !checkpoint_authority_matches(selector, store, authority)
    {
        return Err(ResidentProcessHostSelectionError::NativeCheckpointAuthority);
    }
    Ok(())
}

fn validate_native_invoke_associations(
    selector: &ResidentDeploymentAuthoritySelector,
    lineage: &ResidentProcessOperationLineage,
    store: &pale_ports::AuthorityStoreCoordinate,
    input: &NativeOperationInvokeInput<'_, '_, '_>,
) -> Result<(), ResidentProcessHostSelectionError> {
    let context = &input.attempt.context;
    let (handle_ref, handle) = context.operation.handle();
    if !lineage.admits(handle_ref, handle)
        || !selector.matches_operation(context.operation)
        || context.observation.handle_ref != handle_ref
        || context.observation.handle != handle
    {
        return Err(ResidentProcessHostSelectionError::NativeOperationAuthority);
    }
    if !selector.matches_current_session(context.current)
        || !selector.matches_signing_authority(context.current.generation.signed.signing_authority)
    {
        return Err(ResidentProcessHostSelectionError::NativeCurrentAuthority);
    }
    let historical_session = context.operation.historical_session_lease();
    if context.session.lease() != historical_session
        || context.lease != context.operation.historical_operation_lease()
        || historical_session.deployment_runtime_authority_ref() != &selector.runtime_authority_ref
        || historical_session
            .deployment_runtime_authority_sha256()
            .as_str()
            != selector.runtime_authority_sha256.as_str()
        || historical_session.issuer_principal_id() != &selector.principal_id
    {
        return Err(ResidentProcessHostSelectionError::HistoricalNativeAuthority);
    }
    let request = context.request.to_preimage();
    if !selector.matches_signing_authority(&request.signing_authority_binding)
        || !selector.matches_signing_authority(context.observation.signed.signing_authority)
        || !selector.matches_signing_authority(input.signing.signing_authority)
        || !selector
            .matches_signing_authority(input.attempt.result_admission.result.signing_authority)
    {
        return Err(ResidentProcessHostSelectionError::NativeSigningAuthority);
    }
    if input.checkpoint.1.control_store_id() != store.store_id()
        || input.checkpoint.1.control_incarnation_id() != store.incarnation_id()
        || input.checkpoint.1.issuer_principal_id() != &selector.principal_id
        || !checkpoint_authority_matches(selector, store, input.checkpoint_authority)
    {
        return Err(ResidentProcessHostSelectionError::NativeCheckpointAuthority);
    }
    Ok(())
}
