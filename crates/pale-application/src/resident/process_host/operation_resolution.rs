//! Continue one recovered Invoke or pairwise Delegate before implementation selection.

use super::*;
use crate::resident::operation_dispatch::{
    observation_checkpoint_preimage, prepare_operation_context_binding,
    CompletedOperationContextBinding, OperationAttemptContext, OperationAttemptError,
    OperationCheckpointInput, OperationContextBindingInput, PreparedOperationContextBinding,
};
use crate::resident::process_host::restart::{
    ResidentProcessOperationResolutionPhase, ResidentProcessRecoveredOperationResolution,
};
use crate::resident::{
    CompletedOperationObservationCustody, DatabaseCommittedOperationObservationAnchorPending,
    OperationObservationAuthorityStoreResolver, OperationObservationCustodyAnchorFailure,
    OperationObservationCustodyCommitFailure, PreparedOperationObservationCheckpoint,
};
use pale_core::records::{
    ControlStateCheckpointV1, ImmutableRecordReferenceV1, OperationActivePhaseV1,
    OperationObservationAdmissionContextV1, OperationObservationStateV1,
    SignedResidentAdmissionContextV1,
};
use pale_ports::AccumulatorPreparationLimits;

/// Current authority and fresh attempt references for one pre-attempt handoff.
#[derive(Clone)]
pub(super) struct ResidentProcessOperationResolutionInput<'a> {
    pub(super) session: &'a super::super::AdmittedAmbientSession,
    pub(super) current: &'a super::super::AmbientSessionAdmissionContext<'a>,
    pub(super) signing: SignedResidentAdmissionContextV1<'a>,
    pub(super) selection_ref: ResidentRecordRefV1,
    pub(super) observation_ref: ResidentRecordRefV1,
    pub(super) maximum_selection_bytes: usize,
    pub(super) maximum_record_bytes: usize,
}

/// Exact context evidence and custody inputs for one recovered `Resolving` head.
pub(super) struct ResidentProcessOperationContextInput<'a> {
    pub(super) session: &'a super::super::AdmittedAmbientSession,
    pub(super) current: &'a super::super::AmbientSessionAdmissionContext<'a>,
    pub(super) evidence: ImmutableRecordReferenceV1,
    pub(super) observation_ref: ResidentRecordRefV1,
    pub(super) signing: SignedResidentAdmissionContextV1<'a>,
    pub(super) checkpoint: OperationCheckpointInput,
    pub(super) checkpoint_issued_at: UnixTimeV1,
    pub(super) checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    pub(super) maximum_record_bytes: usize,
}

/// Signed `ContextBound` successor awaiting one exact checkpoint.
#[must_use = "retain this exact context binding or discard it"]
pub(super) struct ResidentProcessPreparedOperationContext<'a> {
    selector: ResidentDeploymentAuthoritySelector,
    lineage: super::restart::ResidentProcessOperationLineage,
    binding: PreparedOperationContextBinding<'a>,
    prior_checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    checkpoint: OperationCheckpointInput,
    checkpoint_issued_at: UnixTimeV1,
}

/// Prepared roots for one recovered context-binding checkpoint.
#[must_use = "issue this exact checkpoint or discard the context binding"]
pub(super) struct ResidentProcessPreparedOperationContextCheckpoint<'a> {
    context: &'a ResidentProcessPreparedOperationContext<'a>,
    prepared: PreparedOperationObservationCheckpoint<'a>,
}

/// One context checkpoint signed once and not yet attached.
#[must_use = "attach this exact checkpoint without signing another value"]
pub(super) struct ResidentProcessIssuedOperationContextCheckpoint<'a> {
    context: &'a ResidentProcessPreparedOperationContext<'a>,
    checkpoint: ControlStateCheckpointV1,
}

/// One exact context-binding database candidate awaiting read-back.
#[must_use = "reconcile only this retained exact candidate"]
pub(super) struct ResidentProcessOperationContextCommitFailure<'a> {
    context: &'a ResidentProcessPreparedOperationContext<'a>,
    checkpoint: ControlStateCheckpointV1,
    failure: OperationObservationCustodyCommitFailure,
}

/// Database-proven context custody awaiting only its independent anchor.
#[must_use = "continue only this retained anchor transition"]
pub(super) struct ResidentProcessOperationContextAnchorPending<'a> {
    context: &'a ResidentProcessPreparedOperationContext<'a>,
    checkpoint: ControlStateCheckpointV1,
    custody: ResidentProcessOperationContextAnchorCustody,
}

enum ResidentProcessOperationContextAnchorCustody {
    Pending(Box<DatabaseCommittedOperationObservationAnchorPending>),
    Failed(OperationObservationCustodyAnchorFailure),
}

/// Durable context custody that can be lent to implementation selection.
pub(in crate::resident) struct CompletedResidentProcessOperationContext<'a> {
    binding: CompletedOperationContextBinding<'a>,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    custody: CompletedOperationObservationCustody,
}

impl CompletedResidentProcessOperationContext<'_> {
    /// Borrow the exact durable `ContextBound` observation.
    pub(super) fn observation(
        &self,
    ) -> (
        &ResidentRecordRefV1,
        &pale_core::records::OperationObservationV1,
    ) {
        self.binding.observation()
    }

    /// Borrow the database and anchor custody proof.
    pub(super) fn custody(&self) -> &CompletedOperationObservationCustody {
        &self.custody
    }
}

/// Context custody stopped after the context observation was signed once.
pub(super) enum ResidentProcessOperationContextCustodyFailure<'a> {
    /// Deterministic roots or checkpoint attachment failed; the signed checkpoint remains exact.
    Issued {
        continuation: Box<ResidentProcessIssuedOperationContextCheckpoint<'a>>,
        failure: crate::resident::operation_dispatch::OperationAppendFailure,
    },
    /// The database outcome needs exact candidate read-back.
    Commit(Box<ResidentProcessOperationContextCommitFailure<'a>>),
    /// Database custody exists and only the independent anchor remains.
    Anchor(Box<ResidentProcessOperationContextAnchorPending<'a>>),
}

/// Why recovered operation resolution stopped before owner-specific preparation.
#[derive(Debug)]
pub(super) enum ResidentProcessOperationResolutionFailure {
    /// Deployment, store, operation, authority, signer, or fresh reference differed.
    Selection(ResidentProcessHostSelectionError),
    /// The existing operation owner rejected continuation or context evidence.
    Admission(OperationAttemptError),
}

impl From<ResidentProcessHostSelectionError> for ResidentProcessOperationResolutionFailure {
    fn from(value: ResidentProcessHostSelectionError) -> Self {
        Self::Selection(value)
    }
}

impl From<OperationAttemptError> for ResidentProcessOperationResolutionFailure {
    fn from(value: OperationAttemptError) -> Self {
        Self::Admission(value)
    }
}

impl<WriterGuard> ResidentProcessHost<'_, WriterGuard> {
    /// Admit one self-contained resolving Invoke for built-in selection.
    ///
    /// The returned context borrows only protected owners that outlive the
    /// native continuation. Context-bearing operations keep the separate
    /// binding path because its recovered witness is local to that loan.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn admit_self_contained_invoke_attempt<'operation, Authority>(
        &'operation self,
        recovered: &'operation ResidentProcessRecoveredOperationResolution,
        access: &mut ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessOperationResolutionInput<'operation>,
        authority: &mut Authority,
    ) -> Result<
        (
            OperationAttemptContext<'operation>,
            (
                &'operation ResidentRecordRefV1,
                &'operation ControlStateCheckpointV1,
            ),
        ),
        ResidentProcessOperationResolutionFailure,
    >
    where
        Authority: super::super::AmbientSessionAuthorityPort,
    {
        if recovered.phase != ResidentProcessOperationResolutionPhase::Resolving {
            return Err(OperationAttemptError::Association.into());
        }
        let operation = recovered.start.control.operation().operation();
        if operation.handle().1.operation_class() != &OperationClassV1::Invoke
            || !operation
                .historical_operation_lease()
                .evidence_obligations()
                .is_empty()
        {
            return Err(OperationAttemptError::ContextRequired.into());
        }
        self.validate_operation_resolution(
            recovered,
            access,
            input.session,
            input.current,
            input.signing,
            input.maximum_record_bytes,
            true,
            authority,
        )?;
        let generation = self.deployments
            [self.deployment_authority_index(&recovered.start.selector)?]
        .application
        .generation(&recovered.start.generation_digest)
        .ok_or(ResidentProcessHostSelectionError::OperationResolutionOperationAuthority)?;
        let predecessor = recovered.start.control.observation().selected_head();
        let checkpoint = recovered.start.control.observation().current_checkpoint();
        validate_attempt_refs(
            operation,
            predecessor,
            checkpoint,
            &input.selection_ref,
            &input.observation_ref,
            input.maximum_selection_bytes,
        )?;
        let observation = OperationObservationAdmissionContextV1 {
            signed: input.signing,
            handle_ref: operation.handle().0,
            handle: operation.handle().1,
            predecessor: Some(predecessor),
        };
        Ok((
            OperationAttemptContext {
                operation,
                request: recovered.start.control.operation().request(),
                session: input.session,
                current: input.current,
                lease: operation.historical_operation_lease(),
                context_binding: None,
                generation,
                selection_ref: input.selection_ref,
                maximum_selection_bytes: input.maximum_selection_bytes,
                observation_ref: input.observation_ref,
                observation,
            },
            checkpoint,
        ))
    }

    /// Lend one self-contained or already-bound operation to its next semantic owner.
    ///
    /// The closure receives the exact private attempt context. This method does
    /// not choose an implementation, sign a selection, dispatch, or call a carrier.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn with_recovered_operation_resolution<Output, OwnerError, Authority, Prepare>(
        &self,
        recovered: &ResidentProcessRecoveredOperationResolution,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessOperationResolutionInput<'_>,
        authority: &mut Authority,
        prepare: Prepare,
    ) -> Result<Result<Output, OwnerError>, ResidentProcessOperationResolutionFailure>
    where
        Authority: super::super::AmbientSessionAuthorityPort,
        Prepare: FnOnce(
            OperationAttemptContext<'_>,
            (&ResidentRecordRefV1, &ControlStateCheckpointV1),
        ) -> Result<Output, OwnerError>,
    {
        self.with_operation_resolution_context(recovered, access, input, authority, None, prepare)
    }

    /// Lend one admitted self-contained resolution with its already-fenced ports.
    ///
    /// The owner closure may retain exactly one signed successor. It receives no
    /// discovery, retry, or replay authority beyond the selected operation and
    /// physical ports already validated by this host.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn with_recovered_operation_resolution_access<
        Output,
        OwnerError,
        Authority,
        Prepare,
    >(
        &self,
        recovered: &ResidentProcessRecoveredOperationResolution,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessOperationResolutionInput<'_>,
        authority: &mut Authority,
        prepare: Prepare,
    ) -> Result<Result<Output, OwnerError>, ResidentProcessOperationResolutionFailure>
    where
        Authority: super::super::AmbientSessionAuthorityPort,
        Prepare: FnOnce(
            OperationAttemptContext<'_>,
            (&ResidentRecordRefV1, &ControlStateCheckpointV1),
            ResidentDeploymentOperationAccess<'_>,
            &mut Authority,
        ) -> Result<Output, OwnerError>,
    {
        let selector = &recovered.start.selector;
        let index = self.deployment_authority_index(selector)?;
        Self::with_operation_resolution_context_in(
            self.deployments[index].application.as_ref(),
            selector,
            recovered,
            access,
            input,
            authority,
            None,
            prepare,
        )
    }

    /// Lend one newly completed context binding to its next semantic owner.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn with_completed_operation_context<Output, OwnerError, Authority, Prepare>(
        &self,
        recovered: &ResidentProcessRecoveredOperationResolution,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessOperationResolutionInput<'_>,
        completed: &CompletedResidentProcessOperationContext<'_>,
        authority: &mut Authority,
        prepare: Prepare,
    ) -> Result<Result<Output, OwnerError>, ResidentProcessOperationResolutionFailure>
    where
        Authority: super::super::AmbientSessionAuthorityPort,
        Prepare: FnOnce(
            OperationAttemptContext<'_>,
            (&ResidentRecordRefV1, &ControlStateCheckpointV1),
        ) -> Result<Output, OwnerError>,
    {
        Self::validate_completed_operation_context(recovered, completed)?;
        self.with_operation_resolution_context(
            recovered,
            access,
            input,
            authority,
            Some((
                &completed.binding,
                (&completed.checkpoint.0, &completed.checkpoint.1),
            )),
            prepare,
        )
    }

    pub(super) fn validate_completed_operation_context(
        recovered: &ResidentProcessRecoveredOperationResolution,
        completed: &CompletedResidentProcessOperationContext<'_>,
    ) -> Result<(), ResidentProcessOperationResolutionFailure> {
        if recovered.phase != ResidentProcessOperationResolutionPhase::Resolving
            || recovered
                .start
                .control
                .operation()
                .operation()
                .historical_operation_lease()
                .evidence_obligations()
                .is_empty()
            || !completed.binding.matches(
                recovered.start.control.operation().operation(),
                completed.binding.observation(),
            )
            || completed.binding.observation().1.predecessor_ref().as_ref()
                != Some(recovered.start.control.observation().selected_head().0)
            || completed
                .binding
                .observation()
                .1
                .predecessor_sha256()
                .as_ref()
                != Some(
                    recovered
                        .start
                        .control
                        .observation()
                        .selected_head()
                        .1
                        .canonical_operation_observation_sha256(),
                )
            || completed
                .checkpoint
                .1
                .previous_control_checkpoint_ref()
                .as_ref()
                != Some(recovered.start.control.observation().current_checkpoint().0)
            || completed
                .checkpoint
                .1
                .previous_control_checkpoint_sha256()
                .as_ref()
                != Some(
                    recovered
                        .start
                        .control
                        .observation()
                        .current_checkpoint()
                        .1
                        .canonical_control_checkpoint_sha256(),
                )
        {
            return Err(OperationAttemptError::Association.into());
        }
        Ok(())
    }

    pub(super) fn completed_operation_context_parts<'a>(
        completed: &'a CompletedResidentProcessOperationContext<'a>,
    ) -> (
        &'a CompletedOperationContextBinding<'a>,
        (&'a ResidentRecordRefV1, &'a ControlStateCheckpointV1),
    ) {
        (
            &completed.binding,
            (&completed.checkpoint.0, &completed.checkpoint.1),
        )
    }

    /// Admit and sign the exact context witness for one recovered `Resolving` head.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_recovered_operation_context<'a, Authority, Evidence, Rotation, Sign>(
        &'a self,
        recovered: &'a ResidentProcessRecoveredOperationResolution,
        mut access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessOperationContextInput<'a>,
        authority: &mut Authority,
        admit_evidence: Evidence,
        admit_rotation: Rotation,
        sign: Sign,
    ) -> Result<
        ResidentProcessPreparedOperationContext<'a>,
        ResidentProcessOperationResolutionFailure,
    >
    where
        Authority: super::super::AmbientSessionAuthorityPort,
        Evidence: FnOnce(
            &ImmutableRecordReferenceV1,
            &[ResidentRecordRefV1],
        ) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Sign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        if recovered.phase != ResidentProcessOperationResolutionPhase::Resolving {
            return Err(OperationAttemptError::Association.into());
        }
        let operation = recovered.start.control.operation().operation();
        if operation
            .historical_operation_lease()
            .evidence_obligations()
            .is_empty()
        {
            return Err(OperationAttemptError::ContextNotRequired.into());
        }
        let store = self.validate_operation_resolution(
            recovered,
            &mut access,
            input.session,
            input.current,
            input.signing,
            input.maximum_record_bytes,
            true,
            authority,
        )?;
        validate_context_checkpoint(&recovered.start.selector, store, recovered, &input)?;
        if recovered
            .start
            .lineage
            .consult_child()
            .and_then(|lineage| lineage.context())
            .is_some_and(|expected| expected != &input.evidence)
        {
            return Err(OperationAttemptError::Association.into());
        }
        let head = recovered.start.control.observation().selected_head();
        let admission = OperationObservationAdmissionContextV1 {
            signed: input.signing,
            handle_ref: operation.handle().0,
            handle: operation.handle().1,
            predecessor: Some(head),
        };
        let binding = prepare_operation_context_binding(
            OperationContextBindingInput {
                operation,
                request: recovered.start.control.operation().request(),
                session: input.session,
                current: input.current,
                lease: operation.historical_operation_lease(),
                evidence: input.evidence,
                observation_ref: input.observation_ref,
                observation: &admission,
            },
            authority,
            admit_evidence,
            admit_rotation,
            sign,
        )?;
        Ok(ResidentProcessPreparedOperationContext {
            selector: recovered.start.selector.clone(),
            lineage: recovered.start.lineage.clone(),
            binding,
            prior_checkpoint: (
                recovered
                    .start
                    .control
                    .observation()
                    .current_checkpoint()
                    .0
                    .clone(),
                recovered
                    .start
                    .control
                    .observation()
                    .current_checkpoint()
                    .1
                    .clone(),
            ),
            checkpoint: input.checkpoint,
            checkpoint_issued_at: input.checkpoint_issued_at,
        })
    }

    /// Compute the exact context-binding roots without signing or committing.
    pub(super) fn prepare_operation_context_checkpoint<'a>(
        &self,
        context: &'a ResidentProcessPreparedOperationContext<'a>,
        access: ResidentDeploymentOperationAccess<'_>,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        limits: AccumulatorPreparationLimits,
    ) -> Result<
        Result<ResidentProcessPreparedOperationContextCheckpoint<'a>, OperationAttemptError>,
        ResidentProcessHostSelectionError,
    > {
        self.validate_prepared_operation_context(context, &access)?;
        let result = context
            .binding
            .prepare_append(resolve_store)
            .map_err(OperationAttemptError::Projection)
            .and_then(|append| {
                append
                    .prepare_checkpoint(
                        context.binding.operation().checkpoint_selection(),
                        &context.prior_checkpoint.0,
                        &context.prior_checkpoint.1,
                        limits,
                        access.custody,
                    )
                    .map_err(OperationAttemptError::Projection)
            })
            .map(
                |prepared| ResidentProcessPreparedOperationContextCheckpoint { context, prepared },
            );
        Ok(result)
    }

    /// Sign one exact prepared context checkpoint.
    pub(super) fn issue_operation_context_checkpoint<'a, Sign>(
        &self,
        access: ResidentDeploymentOperationAccess<'_>,
        prepared: ResidentProcessPreparedOperationContextCheckpoint<'a>,
        schema: &pale_core::records::SchemaDigest,
        authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign: Sign,
    ) -> Result<
        Result<ResidentProcessIssuedOperationContextCheckpoint<'a>, OperationAttemptError>,
        ResidentProcessHostSelectionError,
    >
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let store = self.validate_prepared_operation_context(prepared.context, &access)?;
        let expected_schema = pale_contracts::record_schema_identity(
            CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID,
        )
        .map_err(|_| ResidentProcessHostSelectionError::OperationResolutionCheckpointAuthority)?;
        if schema != &expected_schema.schema_sha256
            || !checkpoint_authority_matches(&prepared.context.selector, store, authority)
        {
            return Err(ResidentProcessHostSelectionError::OperationResolutionCheckpointAuthority);
        }
        let preimage = match observation_checkpoint_preimage(
            &prepared.prepared,
            (
                &prepared.context.prior_checkpoint.0,
                &prepared.context.prior_checkpoint.1,
            ),
            &prepared.context.checkpoint,
            prepared.context.checkpoint_issued_at,
            schema,
            authority,
        ) {
            Ok(preimage) => preimage,
            Err(reason) => return Ok(Err(reason)),
        };
        Ok(prepared
            .prepared
            .issue_checkpoint_value(preimage, schema, authority, sign)
            .map(
                |checkpoint| ResidentProcessIssuedOperationContextCheckpoint {
                    context: prepared.context,
                    checkpoint,
                },
            )
            .map_err(OperationAttemptError::Projection))
    }

    /// Attach, commit and anchor one already issued context checkpoint.
    pub(super) fn retain_operation_context_checkpoint<'a>(
        &self,
        access: ResidentDeploymentOperationAccess<'_>,
        issued: ResidentProcessIssuedOperationContextCheckpoint<'a>,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        limits: AccumulatorPreparationLimits,
    ) -> Result<
        Result<
            CompletedResidentProcessOperationContext<'a>,
            ResidentProcessOperationContextCustodyFailure<'a>,
        >,
        ResidentProcessHostSelectionError,
    > {
        self.validate_prepared_operation_context(issued.context, &access)?;
        let prepared = match issued
            .context
            .binding
            .prepare_append(resolve_store)
            .map_err(crate::resident::operation_dispatch::OperationAppendFailure::Projection)
            .and_then(|append| {
                append
                    .prepare_checkpoint(
                        issued.context.binding.operation().checkpoint_selection(),
                        &issued.context.prior_checkpoint.0,
                        &issued.context.prior_checkpoint.1,
                        limits,
                        access.custody,
                    )
                    .map_err(
                        crate::resident::operation_dispatch::OperationAppendFailure::Projection,
                    )
            }) {
            Ok(prepared) => prepared,
            Err(failure) => {
                return Ok(Err(ResidentProcessOperationContextCustodyFailure::Issued {
                    continuation: Box::new(issued),
                    failure,
                }));
            }
        };
        let physical = match prepared
            .attach_issued_checkpoint(&issued.context.checkpoint.reference, &issued.checkpoint)
        {
            Ok(physical) => physical,
            Err(reason) => {
                return Ok(Err(ResidentProcessOperationContextCustodyFailure::Issued {
                    continuation: Box::new(issued),
                    failure:
                        crate::resident::operation_dispatch::OperationAppendFailure::Projection(
                            reason,
                        ),
                }));
            }
        };
        let pending = match physical.commit(access.custody, access.anchor) {
            Ok(pending) => pending,
            Err(failure) => {
                return Ok(Err(ResidentProcessOperationContextCustodyFailure::Commit(
                    Box::new(ResidentProcessOperationContextCommitFailure {
                        context: issued.context,
                        checkpoint: issued.checkpoint,
                        failure,
                    }),
                )));
            }
        };
        match pending.advance_anchor(access.anchor) {
            Ok(custody) => Ok(complete_context(
                issued.context,
                issued.checkpoint.clone(),
                custody,
                resolve_store,
            )
            .map_err(
                |failure| ResidentProcessOperationContextCustodyFailure::Issued {
                    continuation: Box::new(issued),
                    failure: crate::resident::operation_dispatch::OperationAppendFailure::Operation(
                        failure,
                    ),
                },
            )),
            Err(failure) => Ok(Err(ResidentProcessOperationContextCustodyFailure::Anchor(
                Box::new(ResidentProcessOperationContextAnchorPending {
                    context: issued.context,
                    checkpoint: issued.checkpoint,
                    custody: ResidentProcessOperationContextAnchorCustody::Failed(failure),
                }),
            ))),
        }
    }

    /// Resolve only the retained exact context candidate after commit ambiguity.
    pub(super) fn reconcile_operation_context_checkpoint<'a>(
        &self,
        access: ResidentDeploymentOperationAccess<'_>,
        failure: ResidentProcessOperationContextCommitFailure<'a>,
    ) -> Result<
        Result<
            ResidentProcessOperationContextAnchorPending<'a>,
            Box<ResidentProcessOperationContextCommitFailure<'a>>,
        >,
        ResidentProcessHostSelectionError,
    > {
        self.validate_prepared_operation_context(failure.context, &access)?;
        match failure.failure.reconcile_exact(access.custody) {
            Ok(pending) => Ok(Ok(ResidentProcessOperationContextAnchorPending {
                context: failure.context,
                checkpoint: failure.checkpoint,
                custody: ResidentProcessOperationContextAnchorCustody::Pending(Box::new(pending)),
            })),
            Err(lower) => Ok(Err(Box::new(
                ResidentProcessOperationContextCommitFailure {
                    failure: lower,
                    ..failure
                },
            ))),
        }
    }

    /// Continue only the retained independent-anchor transition.
    pub(super) fn continue_operation_context_anchor<'a>(
        &self,
        access: ResidentDeploymentOperationAccess<'_>,
        pending: ResidentProcessOperationContextAnchorPending<'a>,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    ) -> Result<
        Result<
            CompletedResidentProcessOperationContext<'a>,
            Box<ResidentProcessOperationContextAnchorPending<'a>>,
        >,
        ResidentProcessHostSelectionError,
    > {
        self.validate_prepared_operation_context(pending.context, &access)?;
        pending
            .context
            .binding
            .prepare_append(resolve_store)
            .map(|_| ())
            .map_err(|_| {
                ResidentProcessHostSelectionError::OperationResolutionOperationAuthority
            })?;
        let ResidentProcessOperationContextAnchorPending {
            context,
            checkpoint,
            custody,
        } = pending;
        let result = match custody {
            ResidentProcessOperationContextAnchorCustody::Pending(pending) => {
                pending.advance_anchor(access.anchor)
            }
            ResidentProcessOperationContextAnchorCustody::Failed(failure) => {
                match failure.try_into_retryable() {
                    Ok(pending) => pending.advance_anchor(access.anchor),
                    Err(failure) => failure.continue_after_failed_observation(access.anchor),
                }
            }
        };
        match result {
            Ok(custody) => complete_context(context, checkpoint, custody, resolve_store)
                .map(Ok)
                .map_err(|_| {
                    ResidentProcessHostSelectionError::OperationResolutionOperationAuthority
                }),
            Err(failure) => Ok(Err(Box::new(
                ResidentProcessOperationContextAnchorPending {
                    context,
                    checkpoint,
                    custody: ResidentProcessOperationContextAnchorCustody::Failed(failure),
                },
            ))),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn with_operation_resolution_context<Output, OwnerError, Authority, Prepare>(
        &self,
        recovered: &ResidentProcessRecoveredOperationResolution,
        access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessOperationResolutionInput<'_>,
        authority: &mut Authority,
        completed: Option<(
            &CompletedOperationContextBinding<'_>,
            (&ResidentRecordRefV1, &ControlStateCheckpointV1),
        )>,
        prepare: Prepare,
    ) -> Result<Result<Output, OwnerError>, ResidentProcessOperationResolutionFailure>
    where
        Authority: super::super::AmbientSessionAuthorityPort,
        Prepare: FnOnce(
            OperationAttemptContext<'_>,
            (&ResidentRecordRefV1, &ControlStateCheckpointV1),
        ) -> Result<Output, OwnerError>,
    {
        let selector = &recovered.start.selector;
        let index = self.deployment_authority_index(selector)?;
        Self::with_operation_resolution_context_in(
            self.deployments[index].application.as_ref(),
            selector,
            recovered,
            access,
            input,
            authority,
            completed,
            |context, checkpoint, _, _| prepare(context, checkpoint),
        )
    }

    /// Lend recovered operation meaning together with its already-fenced ports.
    ///
    /// Owner-specific composition uses this form after splitting the immutable
    /// application from process-local task owners. The closure receives no new
    /// authority: every association and the current recovered head are checked
    /// before the store, session, or anchor capability is lent.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn with_operation_resolution_context_in<Output, OwnerError, Authority, Prepare>(
        application: &ResidentApplication<'_>,
        selector: &ResidentDeploymentAuthoritySelector,
        recovered: &ResidentProcessRecoveredOperationResolution,
        mut access: ResidentDeploymentOperationAccess<'_>,
        input: ResidentProcessOperationResolutionInput<'_>,
        authority: &mut Authority,
        completed: Option<(
            &CompletedOperationContextBinding<'_>,
            (&ResidentRecordRefV1, &ControlStateCheckpointV1),
        )>,
        prepare: Prepare,
    ) -> Result<Result<Output, OwnerError>, ResidentProcessOperationResolutionFailure>
    where
        Authority: super::super::AmbientSessionAuthorityPort,
        Prepare: FnOnce(
            OperationAttemptContext<'_>,
            (&ResidentRecordRefV1, &ControlStateCheckpointV1),
            ResidentDeploymentOperationAccess<'_>,
            &mut Authority,
        ) -> Result<Output, OwnerError>,
    {
        let verify_recovered_head = completed.is_none();
        Self::validate_operation_resolution_in(
            application,
            selector,
            recovered,
            &mut access,
            input.session,
            input.current,
            input.signing,
            input.maximum_record_bytes,
            verify_recovered_head,
            authority,
        )?;
        let operation = recovered.start.control.operation().operation();
        let generation = application
            .generation(&recovered.start.generation_digest)
            .ok_or(ResidentProcessHostSelectionError::OperationResolutionOperationAuthority)?;
        let obligations_empty = operation
            .historical_operation_lease()
            .evidence_obligations()
            .is_empty();
        let recovered_binding;
        let (binding, predecessor, checkpoint) =
            match (recovered.phase, obligations_empty, completed) {
                (ResidentProcessOperationResolutionPhase::Resolving, true, None) => (
                    None,
                    recovered.start.control.observation().selected_head(),
                    recovered.start.control.observation().current_checkpoint(),
                ),
                (ResidentProcessOperationResolutionPhase::ContextBound, false, None) => {
                    recovered_binding = CompletedOperationContextBinding::recover(
                        operation,
                        recovered.start.control.observation(),
                    )?;
                    let predecessor = recovered_binding.observation();
                    (
                        Some(&recovered_binding),
                        predecessor,
                        recovered.start.control.observation().current_checkpoint(),
                    )
                }
                (
                    ResidentProcessOperationResolutionPhase::Resolving,
                    false,
                    Some((binding, checkpoint)),
                ) => (Some(binding), binding.observation(), checkpoint),
                _ => return Err(OperationAttemptError::Association.into()),
            };
        validate_attempt_refs(
            operation,
            predecessor,
            checkpoint,
            &input.selection_ref,
            &input.observation_ref,
            input.maximum_selection_bytes,
        )?;
        let admission = OperationObservationAdmissionContextV1 {
            signed: input.signing,
            handle_ref: operation.handle().0,
            handle: operation.handle().1,
            predecessor: Some(predecessor),
        };
        let result = prepare(
            OperationAttemptContext {
                operation,
                request: recovered.start.control.operation().request(),
                session: input.session,
                current: input.current,
                lease: operation.historical_operation_lease(),
                context_binding: binding,
                generation,
                selection_ref: input.selection_ref,
                maximum_selection_bytes: input.maximum_selection_bytes,
                observation_ref: input.observation_ref,
                observation: admission,
            },
            checkpoint,
            access,
            authority,
        );
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    fn validate_operation_resolution<Authority>(
        &self,
        recovered: &ResidentProcessRecoveredOperationResolution,
        access: &mut ResidentDeploymentOperationAccess<'_>,
        session: &super::super::AdmittedAmbientSession,
        current: &super::super::AmbientSessionAdmissionContext<'_>,
        signing: SignedResidentAdmissionContextV1<'_>,
        maximum_record_bytes: usize,
        verify_recovered_head: bool,
        authority: &mut Authority,
    ) -> Result<&AuthorityStoreCoordinate, ResidentProcessOperationResolutionFailure>
    where
        Authority: super::super::AmbientSessionAuthorityPort,
    {
        let selector = &recovered.start.selector;
        let index = self.deployment_authority_index(selector)?;
        let application = self.deployments[index].application.as_ref();
        Self::validate_operation_resolution_in(
            application,
            selector,
            recovered,
            access,
            session,
            current,
            signing,
            maximum_record_bytes,
            verify_recovered_head,
            authority,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn validate_operation_resolution_in<'application, Authority>(
        application: &'application ResidentApplication<'_>,
        selector: &ResidentDeploymentAuthoritySelector,
        recovered: &ResidentProcessRecoveredOperationResolution,
        access: &mut ResidentDeploymentOperationAccess<'_>,
        session: &super::super::AdmittedAmbientSession,
        current: &super::super::AmbientSessionAdmissionContext<'_>,
        signing: SignedResidentAdmissionContextV1<'_>,
        maximum_record_bytes: usize,
        verify_recovered_head: bool,
        authority: &mut Authority,
    ) -> Result<&'application AuthorityStoreCoordinate, ResidentProcessOperationResolutionFailure>
    where
        Authority: super::super::AmbientSessionAuthorityPort,
    {
        let store = application.store();
        let control = &recovered.start.control;
        let operation = control.operation().operation();
        let (_, handle) = operation.handle();
        let historical_session = operation.historical_session_lease();
        let historical_lease = operation.historical_operation_lease();
        if access.store != store
            || control.operation().store() != store
            || control.observation().store() != store
            || !recovered.start.lineage.admits(operation.handle().0, handle)
            || !selector.matches_operation(operation)
            || handle.compiled_generation_sha256() != &recovered.start.generation_digest
            || application
                .generation(&recovered.start.generation_digest)
                .is_none()
            || maximum_record_bytes == 0
        {
            return Err(
                ResidentProcessHostSelectionError::OperationResolutionOperationAuthority.into(),
            );
        }
        if !selector.matches_current_session(current)
            || !selector.matches_signing_authority(current.generation.signed.signing_authority)
            || current
                .generation
                .compiled_generation
                .compiled_generation_digest()
                != &recovered.start.generation_digest
            || current.generation.active_head.compiled_generation_digest()
                != &recovered.start.generation_digest
            || current
                .generation
                .activation_fence
                .activation_fence_digest()
                != handle.activation_fence_sha256()
        {
            return Err(
                ResidentProcessHostSelectionError::OperationResolutionCurrentAuthority.into(),
            );
        }
        if session.lease() != historical_session
            || historical_session.compiled_generation_digest()
                != handle.compiled_generation_sha256()
            || historical_session.activation_fence_digest() != handle.activation_fence_sha256()
            || historical_session.instance_digest() != handle.instance_sha256()
            || historical_session.deployment_runtime_authority_ref()
                != &selector.runtime_authority_ref
            || historical_session
                .deployment_runtime_authority_sha256()
                .as_str()
                != selector.runtime_authority_sha256.as_str()
            || historical_session.issuer_principal_id() != &selector.principal_id
            || historical_lease.operation_lease_digest() != handle.operation_lease_sha256()
        {
            return Err(
                ResidentProcessHostSelectionError::OperationResolutionHistoricalAuthority.into(),
            );
        }
        if !selector.matches_signing_authority(
            &control
                .operation()
                .request()
                .to_preimage()
                .signing_authority_binding,
        ) || !selector.matches_signing_authority(signing.signing_authority)
        {
            return Err(
                ResidentProcessHostSelectionError::OperationResolutionSigningAuthority.into(),
            );
        }
        let expected_observation_schema = pale_contracts::record_schema_identity(
            OPERATION_OBSERVATION_V1_SCHEMA_ID,
        )
        .map_err(|_| ResidentProcessHostSelectionError::OperationResolutionSigningAuthority)?;
        if signing.expected_schema_sha256 != &expected_observation_schema.schema_sha256 {
            return Err(
                ResidentProcessHostSelectionError::OperationResolutionSigningAuthority.into(),
            );
        }
        let head = control.observation().selected_head();
        if !observation_matches_resolution_selector(selector, head.1)
            || current.now < *head.1.observed_at()
            || current.now >= *handle.expires_at()
            || current.now >= control.operation().request().expires_at()
            || current.now >= historical_lease.expires_at()
        {
            return Err(
                ResidentProcessHostSelectionError::OperationResolutionObservationAuthority.into(),
            );
        }
        match (recovered.phase, head.1.state()) {
            (
                ResidentProcessOperationResolutionPhase::Resolving,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::Resolving,
                },
            )
            | (
                ResidentProcessOperationResolutionPhase::ContextBound,
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::ContextBound { .. },
                },
            ) => {}
            _ => return Err(OperationAttemptError::Association.into()),
        }
        if let Some(lineage) = recovered.start.lineage.consult_child() {
            match (lineage.context(), head.1.state()) {
                (
                    Some(expected),
                    OperationObservationStateV1::Active {
                        phase: OperationActivePhaseV1::ContextBound { evidence },
                    },
                ) if expected == evidence => {}
                (
                    Some(_),
                    OperationObservationStateV1::Active {
                        phase: OperationActivePhaseV1::Resolving,
                    },
                ) => {}
                (
                    None,
                    OperationObservationStateV1::Active {
                        phase: OperationActivePhaseV1::Resolving,
                    },
                ) => {}
                _ => return Err(OperationAttemptError::Association.into()),
            }
        }
        let checkpoint = control.observation().current_checkpoint().1;
        if checkpoint.control_store_id() != store.store_id()
            || checkpoint.control_incarnation_id() != store.incarnation_id()
            || checkpoint.issuer_principal_id() != &selector.principal_id
        {
            return Err(
                ResidentProcessHostSelectionError::OperationResolutionCheckpointAuthority.into(),
            );
        }
        session
            .revalidate(current, authority)
            .map_err(OperationAttemptError::from)?;
        if verify_recovered_head {
            crate::resident::operation_dispatch::verify_recovered_operation_head_custody(
                operation,
                control.observation(),
                access.custody,
                access.anchor,
                maximum_record_bytes,
            )?;
        }
        Ok(store)
    }

    fn validate_prepared_operation_context<'a>(
        &'a self,
        context: &ResidentProcessPreparedOperationContext<'_>,
        access: &ResidentDeploymentOperationAccess<'_>,
    ) -> Result<&'a AuthorityStoreCoordinate, ResidentProcessHostSelectionError> {
        let index = self.deployment_authority_index(&context.selector)?;
        let store = self.deployments[index].store();
        let operation = context.binding.operation();
        let (_, handle) = operation.handle();
        let observation = context.binding.observation();
        if access.store != store
            || !context.lineage.admits(operation.handle().0, handle)
            || !context.selector.matches_operation(operation)
            || !operation.checkpoint_selection().matches_store(store)
            || context.prior_checkpoint.1.control_store_id() != store.store_id()
            || context.prior_checkpoint.1.control_incarnation_id() != store.incarnation_id()
            || context.prior_checkpoint.1.issuer_principal_id() != &context.selector.principal_id
            || !matches!(
                observation.1.state(),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::ContextBound { .. }
                }
            )
        {
            return Err(ResidentProcessHostSelectionError::OperationResolutionOperationAuthority);
        }
        Ok(store)
    }
}

fn validate_context_checkpoint(
    selector: &ResidentDeploymentAuthoritySelector,
    store: &AuthorityStoreCoordinate,
    recovered: &ResidentProcessRecoveredOperationResolution,
    input: &ResidentProcessOperationContextInput<'_>,
) -> Result<(), ResidentProcessHostSelectionError> {
    let prior = recovered.start.control.observation().current_checkpoint();
    let (handle_ref, handle) = recovered.start.control.operation().operation().handle();
    let request_ref = recovered
        .start
        .control
        .operation()
        .operation()
        .request_coordinate()
        .record_ref();
    if !checkpoint_authority_matches(selector, store, input.checkpoint_authority)
        || input.checkpoint_authority.authority_ref != prior.1.checkpoint_signing_authority_ref()
        || input.checkpoint_authority.authority_sha256
            != prior.1.checkpoint_signing_authority_sha256()
        || input.checkpoint_authority.checkpoint_domain_id != prior.1.checkpoint_domain_id()
        || input.checkpoint_authority.checkpoint_profile_ref != prior.1.checkpoint_profile_ref()
        || input.checkpoint_authority.checkpoint_profile_sha256
            != prior.1.checkpoint_profile_sha256()
        || input.checkpoint_authority.subject_key_id != prior.1.issuer_key_id()
        || input.checkpoint_authority.subject_key_epoch != *prior.1.issuer_key_epoch()
        || input.checkpoint_issued_at < *prior.1.issued_at()
        || input.checkpoint.reference == *handle_ref
        || input.checkpoint.reference == *handle.operation_lease_ref()
        || input.checkpoint.reference == *request_ref
        || input.checkpoint.reference == *prior.0
        || input.checkpoint.reference == input.observation_ref
        || input.checkpoint.reference == input.evidence.record_ref
        || input.observation_ref == *prior.0
        || input.evidence.record_ref == *prior.0
    {
        return Err(ResidentProcessHostSelectionError::OperationResolutionCheckpointAuthority);
    }
    Ok(())
}

pub(super) fn validate_attempt_refs(
    operation: &crate::resident::IssuedLocalOperation,
    predecessor: (
        &ResidentRecordRefV1,
        &pale_core::records::OperationObservationV1,
    ),
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    selection_ref: &ResidentRecordRefV1,
    observation_ref: &ResidentRecordRefV1,
    maximum_selection_bytes: usize,
) -> Result<(), OperationAttemptError> {
    let (handle_ref, handle) = operation.handle();
    let request_ref = operation.request_coordinate().record_ref();
    if maximum_selection_bytes == 0 {
        return Err(OperationAttemptError::BoundExceeded);
    }
    if selection_ref == observation_ref
        || selection_ref == handle_ref
        || selection_ref == handle.operation_lease_ref()
        || selection_ref == request_ref
        || selection_ref == predecessor.0
        || selection_ref == checkpoint.0
        || observation_ref == handle_ref
        || observation_ref == handle.operation_lease_ref()
        || observation_ref == request_ref
        || observation_ref == predecessor.0
        || observation_ref == checkpoint.0
        || matches!(
            predecessor.1.state(),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::ContextBound { evidence }
            } if selection_ref == &evidence.record_ref || observation_ref == &evidence.record_ref
        )
    {
        return Err(OperationAttemptError::Association);
    }
    Ok(())
}

fn observation_matches_resolution_selector(
    selector: &ResidentDeploymentAuthoritySelector,
    observation: &pale_core::records::OperationObservationV1,
) -> bool {
    observation.deployment_runtime_authority_ref() == &selector.runtime_authority_ref
        && observation.deployment_runtime_authority_sha256().as_str()
            == selector.runtime_authority_sha256.as_str()
        && observation.issuer_principal_id() == &selector.principal_id
}

fn complete_context<'a>(
    context: &'a ResidentProcessPreparedOperationContext<'a>,
    checkpoint: ControlStateCheckpointV1,
    custody: CompletedOperationObservationCustody,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
) -> Result<CompletedResidentProcessOperationContext<'a>, OperationAttemptError> {
    let binding = context.binding.finish_borrowed(&custody, resolve_store)?;
    Ok(CompletedResidentProcessOperationContext {
        binding,
        checkpoint: (context.checkpoint.reference.clone(), checkpoint),
        custody,
    })
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/process_host/operation_resolution_support.rs"]
mod test_support;
