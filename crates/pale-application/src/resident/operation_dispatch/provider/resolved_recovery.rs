//! Rebuild one provider attempt after an anchored `ProviderResolved` head.

use super::*;
use crate::provider_invocation::ProviderInvocationAttemptAdmission;
use crate::resident::operation_dispatch::custody::append_checkpoint_for_selection;
use crate::resident::{
    CompletedOperationObservationCustody, DatabaseCommittedOperationObservationAnchorPending,
    OperationObservationCustodyAnchorFailure, OperationObservationCustodyCommitFailure,
    RecoveredOperationObservation,
};
use pale_ports::{
    AccumulatorPreparationLimits, AntiRollbackAnchorPort, OperationCustodyPort,
    ProviderInvocationCoordinate,
};

/// Source-owned result meaning and implementation coordinates for one attempt.
#[derive(Clone, Copy)]
pub(in crate::resident) struct RecoveredProviderAttemptMeaning<'a> {
    pub(in crate::resident) implementation_kind: &'a BoundedResidentTextV1,
    pub(in crate::resident) implementation_version: &'a BoundedResidentTextV1,
    pub(in crate::resident) admission: ProviderInvocationAttemptAdmission<'a>,
}

/// Exact history and fresh identities needed to sign `DispatchPrepared`.
pub(in crate::resident) struct RecoveredProviderAttemptInput<'a, 'identity> {
    pub(in crate::resident) operation: &'a IssuedLocalOperation,
    pub(in crate::resident) request: &'a CommonRequestEnvelopeV1,
    pub(in crate::resident) session: &'a AdmittedAmbientSession,
    pub(in crate::resident) lease: &'a OperationLeaseV1,
    pub(in crate::resident) generation: &'a AdmittedCompiledGeneration,
    pub(in crate::resident) recovered: &'a RecoveredOperationObservation,
    pub(in crate::resident) current: &'a AmbientSessionAdmissionContext<'a>,
    pub(in crate::resident) provider_binding: &'a ProviderBindingRefV1,
    pub(in crate::resident) attempt_id: &'identity CapabilityAttemptIdV1,
    pub(in crate::resident) selection_ref: ResidentRecordRefV1,
    pub(in crate::resident) observation_ref: ResidentRecordRefV1,
    pub(in crate::resident) maximum_selection_bytes: usize,
    pub(in crate::resident) signing: SignedResidentAdmissionContextV1<'a>,
    pub(in crate::resident) meaning: RecoveredProviderAttemptMeaning<'a>,
}

/// Signed `DispatchPrepared` successor with its private selection.
#[must_use = "retain this exact provider attempt or discard it"]
pub(in crate::resident) struct PreparedRecoveredProviderAttempt {
    operation: IssuedLocalOperation,
    predecessor: (ResidentRecordRefV1, OperationObservationV1),
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
    selection: ControlRecord,
    prior_checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
}

impl PreparedRecoveredProviderAttempt {
    pub(in crate::resident) fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    fn prepare_append(
        &self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    ) -> Result<PreparedOperationObservationAppend<'_>, OperationObservationCustodyProjectionError>
    {
        let (handle_ref, handle) = self.operation.handle();
        let append = prepare_operation_observation_append(
            OperationObservationTransition::new(
                handle_ref,
                handle,
                &self.predecessor.0,
                &self.predecessor.1,
                &self.observation_ref,
                &self.observation,
            ),
            resolve_store,
        )?;
        Ok(
            append.with_associated_record(OperationCustodyRecord::AttemptSelection(
                self.selection.clone(),
            )),
        )
    }
}

/// Anchored custody for one rebuilt `DispatchPrepared` successor.
pub(in crate::resident) struct CompletedRecoveredProviderAttempt {
    attempt: PreparedRecoveredProviderAttempt,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    custody: CompletedOperationObservationCustody,
}

impl CompletedRecoveredProviderAttempt {
    pub(in crate::resident) fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.attempt.observation()
    }

    pub(in crate::resident) fn checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.checkpoint.0, &self.checkpoint.1)
    }

    pub(in crate::resident) const fn custody(&self) -> &CompletedOperationObservationCustody {
        &self.custody
    }
}

/// Exact database ambiguity for one rebuilt provider attempt.
#[must_use = "reconcile only this exact provider-attempt candidate"]
pub(in crate::resident) struct RecoveredProviderAttemptCommitFailure {
    attempt: PreparedRecoveredProviderAttempt,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    failure: OperationObservationCustodyCommitFailure,
}

/// Exact database custody awaiting only its independent anchor.
#[must_use = "continue only this exact provider-attempt anchor transition"]
pub(in crate::resident) struct RecoveredProviderAttemptAnchorPending {
    attempt: PreparedRecoveredProviderAttempt,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    custody: RecoveredProviderAttemptAnchorCustody,
}

enum RecoveredProviderAttemptAnchorCustody {
    Pending(Box<DatabaseCommittedOperationObservationAnchorPending>),
    Failed(OperationObservationCustodyAnchorFailure),
}

/// Exact phase retained when provider-attempt custody stops.
pub(in crate::resident) enum RecoveredProviderAttemptCustodyFailure {
    /// Semantic or checkpoint preparation failed before database mutation.
    Prepared {
        attempt: Box<PreparedRecoveredProviderAttempt>,
        checkpoint: OperationCheckpointInput,
        failure: Box<OperationAppendFailure>,
    },
    /// Database outcome remains ambiguous and owns the exact candidate.
    Commit(Box<RecoveredProviderAttemptCommitFailure>),
    /// Database custody is exact and only the anchor remains.
    Anchor(Box<RecoveredProviderAttemptAnchorPending>),
}

impl RecoveredProviderAttemptCommitFailure {
    pub(in crate::resident) fn reconcile_exact(
        self,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<RecoveredProviderAttemptAnchorPending, Box<Self>> {
        match self.failure.reconcile_exact(custody) {
            Ok(pending) => Ok(RecoveredProviderAttemptAnchorPending {
                attempt: self.attempt,
                checkpoint: self.checkpoint,
                custody: RecoveredProviderAttemptAnchorCustody::Pending(Box::new(pending)),
            }),
            Err(failure) => Err(Box::new(Self { failure, ..self })),
        }
    }
}

impl RecoveredProviderAttemptAnchorPending {
    pub(in crate::resident) fn continue_anchor(
        self,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<CompletedRecoveredProviderAttempt, Box<Self>> {
        let Self {
            attempt,
            checkpoint,
            custody,
        } = self;
        let completed = match custody {
            RecoveredProviderAttemptAnchorCustody::Pending(pending) => {
                pending.advance_anchor(anchor)
            }
            RecoveredProviderAttemptAnchorCustody::Failed(failure) => {
                match failure.try_into_retryable() {
                    Ok(pending) => pending.advance_anchor(anchor),
                    Err(failure) => failure.continue_after_failed_observation(anchor),
                }
            }
        };
        match completed {
            Ok(custody) => Ok(CompletedRecoveredProviderAttempt {
                attempt,
                checkpoint,
                custody,
            }),
            Err(failure) => Err(Box::new(Self {
                attempt,
                checkpoint,
                custody: RecoveredProviderAttemptAnchorCustody::Failed(failure),
            })),
        }
    }
}

/// Rebuild and sign one exact provider attempt without calling a carrier.
pub(in crate::resident) fn prepare_recovered_provider_attempt<
    'a,
    'identity,
    Authority,
    Rotation,
    Sign,
>(
    input: RecoveredProviderAttemptInput<'a, 'identity>,
    authority: &mut Authority,
    admit_rotation: Rotation,
    sign: Sign,
) -> Result<PreparedRecoveredProviderAttempt, OperationAttemptError>
where
    Authority: AmbientSessionAuthorityPort,
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
    let selected = input.recovered.selected_head();
    let OperationObservationStateV1::Active {
        phase: OperationActivePhaseV1::ProviderResolved { provider_binding },
    } = selected.1.state()
    else {
        return Err(OperationAttemptError::Association);
    };
    if provider_binding != input.provider_binding {
        return Err(OperationAttemptError::Association);
    }

    let predecessor = input
        .recovered
        .selected_predecessor()
        .ok_or(OperationAttemptError::Association)?;
    if input.lease.evidence_obligations().is_empty() {
        if !matches!(
            predecessor.1.state(),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Resolving,
            }
        ) {
            return Err(OperationAttemptError::Association);
        }
    } else {
        let binding = CompletedOperationContextBinding::recover(input.operation, input.recovered)?;
        if binding.observation() != predecessor {
            return Err(OperationAttemptError::Association);
        }
    }

    let request_binding = input
        .request
        .local_request_binding()
        .map_err(|_| OperationAttemptError::Association)?;
    let provider = input
        .generation
        .provider_binding(&input.provider_binding.provider_binding_id)
        .ok_or(OperationAttemptError::Association)?;
    if provider.binding_identity != input.provider_binding.provider_binding_identity {
        return Err(OperationAttemptError::Association);
    }
    let invocation = ProviderInvocationRequest::new(
        ProviderInvocationCoordinate::new(
            input.generation.record().compiled_generation_digest(),
            input.generation.record().instance_digest(),
            input.lease.operation(),
            &request_binding,
            input.attempt_id,
        ),
        provider,
        input.meaning.implementation_kind,
        input.meaning.implementation_version,
    )
    .map_err(|_| OperationAttemptError::Association)?;
    let success = CapabilitySuccessAdmissionContextV1 {
        result: CapabilityResultAdmissionContextV1 {
            expected_schema_sha256: input.meaning.admission.result_schema_sha256,
            expected_request_binding: &request_binding,
            expected_attempt_id: input.attempt_id,
            expected_implementation_kind: input.meaning.implementation_kind,
            expected_implementation_version: input.meaning.implementation_version,
            signing_authority: input.meaning.admission.result_signing_authority,
        },
        contract_set_ref: request_binding.contract_set_ref(),
        contract_set: input.meaning.admission.contract_set,
    };
    let failure = AdapterFailureAdmissionContextV1 {
        expected_schema_sha256: input.meaning.admission.failure_schema_sha256,
        expected_request_binding: &request_binding,
        expected_attempt_id: input.attempt_id,
        expected_implementation_kind: input.meaning.implementation_kind,
        expected_implementation_version: input.meaning.implementation_version,
        signing_authority: input.meaning.admission.failure_signing_authority,
    };
    let admission = ProviderInvocationAdmissionContext::new(
        success,
        failure,
        input.meaning.admission.safe_failure_proof,
    );
    let observation_admission = OperationObservationAdmissionContextV1 {
        signed: input.signing,
        handle_ref: input.operation.handle().0,
        handle: input.operation.handle().1,
        predecessor: Some(selected),
    };
    let context = OperationAttemptContext {
        operation: input.operation,
        request: input.request,
        session: input.session,
        current: input.current,
        lease: input.lease,
        context_binding: None,
        generation: input.generation,
        selection_ref: input.selection_ref.clone(),
        maximum_selection_bytes: input.maximum_selection_bytes,
        observation_ref: input.observation_ref.clone(),
        observation: observation_admission,
    };
    context.validate_current_at(input.current, authority)?;
    let selection = AttemptSelection::capture(AttemptSelectionInput {
        parents: context.parents(),
        invocation,
        admission,
    })?;
    if selection.provider() != Some(input.provider_binding)
        || selection
            .provider_request(context.parents())?
            .provider_binding()
            != provider
    {
        return Err(OperationAttemptError::Association);
    }
    let record = selection.record(input.selection_ref, input.maximum_selection_bytes)?;
    let state = OperationObservationStateV1::Active {
        phase: OperationActivePhaseV1::DispatchPrepared {
            attempt_id: selection.attempt_id().clone(),
            evidence: ImmutableRecordReferenceV1 {
                record_ref: record.coordinate().record_ref().clone(),
                record_sha256: ArtifactDigest::parse(record.coordinate().record_sha256().as_str())
                    .map_err(|_| OperationAttemptError::Encoding)?,
            },
        },
    };
    signing::validate_runtime_signer(
        input.signing.signing_authority,
        input.current,
        OPERATION_OBSERVATION_V1_SCHEMA,
    )?;
    let observation = signing::issue_observation(
        &observation_admission,
        state,
        input.current.now,
        admit_rotation,
        sign,
    )?;
    let attempt = PreparedRecoveredProviderAttempt {
        operation: input.operation.clone_for_recovery_continuation(),
        predecessor: (selected.0.clone(), selected.1.clone()),
        observation_ref: input.observation_ref,
        observation,
        selection: record,
        prior_checkpoint: {
            let prior = input.recovered.current_checkpoint();
            (prior.0.clone(), prior.1.clone())
        },
    };
    Ok(attempt)
}

/// Retain one rebuilt attempt through checkpoint, database, and anchor custody.
#[allow(clippy::too_many_arguments)]
pub(in crate::resident) fn retain_recovered_provider_attempt<Sign>(
    attempt: PreparedRecoveredProviderAttempt,
    identity: OperationCheckpointInput,
    now: UnixTimeV1,
    schema: &SchemaDigest,
    authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
    resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    limits: AccumulatorPreparationLimits,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    sign: Sign,
) -> Result<CompletedRecoveredProviderAttempt, RecoveredProviderAttemptCustodyFailure>
where
    Sign: FnOnce(
        &[u8],
        &ControlStateCheckpointPreimageV1,
        &ControlStateCheckpointDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let retry_identity = OperationCheckpointInput {
        reference: identity.reference.clone(),
        id: identity.id.clone(),
    };
    let retained = attempt
        .prepare_append(resolve_store)
        .map_err(OperationAppendFailure::from)
        .and_then(|append| {
            append_checkpoint_for_selection(
                append,
                attempt.operation.checkpoint_selection(),
                (&attempt.prior_checkpoint.0, &attempt.prior_checkpoint.1),
                identity,
                now,
                schema,
                authority,
                limits,
                custody,
                anchor,
                sign,
            )
        });
    match retained {
        Ok((custody, checkpoint)) => Ok(CompletedRecoveredProviderAttempt {
            attempt,
            checkpoint,
            custody,
        }),
        Err(OperationAppendFailure::Commit {
            checkpoint,
            failure,
        }) => Err(RecoveredProviderAttemptCustodyFailure::Commit(Box::new(
            RecoveredProviderAttemptCommitFailure {
                attempt,
                checkpoint: *checkpoint,
                failure,
            },
        ))),
        Err(OperationAppendFailure::Anchor {
            checkpoint,
            failure,
        }) => Err(RecoveredProviderAttemptCustodyFailure::Anchor(Box::new(
            RecoveredProviderAttemptAnchorPending {
                attempt,
                checkpoint: *checkpoint,
                custody: RecoveredProviderAttemptAnchorCustody::Failed(failure),
            },
        ))),
        Err(failure) => Err(RecoveredProviderAttemptCustodyFailure::Prepared {
            attempt: Box::new(attempt),
            checkpoint: retry_identity,
            failure: Box::new(failure),
        }),
    }
}
