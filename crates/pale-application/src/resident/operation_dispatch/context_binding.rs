//! Bind admitted context to a durable operation before implementation selection.

use super::*;

/// Inputs that join admitted context to one durably started operation.
pub struct OperationContextBindingInput<'operation, 'input> {
    /// Locator exposed after durable start or protected recovery.
    pub operation: &'operation IssuedLocalOperation,
    /// Exact admitted request retained by the operation start.
    pub request: &'input CommonRequestEnvelopeV1,
    /// Existing admitted session; binding rechecks its current authority.
    pub session: &'input AdmittedAmbientSession,
    /// Current, independently resolved session authority and generation.
    pub current: &'input AmbientSessionAdmissionContext<'input>,
    /// Existing child lease whose evidence obligations are being satisfied.
    pub lease: &'input OperationLeaseV1,
    /// Immutable reference to the admitted contextual projection.
    pub evidence: ImmutableRecordReferenceV1,
    /// Fresh reference for the signed `ContextBound` observation.
    pub observation_ref: ResidentRecordRefV1,
    /// Exact admitted `Resolving` predecessor and signing context.
    pub observation: &'input OperationObservationAdmissionContextV1<'input>,
}

/// Signed context binding awaiting checkpoint, database, and anchor custody.
///
/// This value proves preparation only. Dropping it has no effect, and it does
/// not grant implementation selection or dispatch.
#[must_use = "commit the context binding before selecting an implementation"]
pub struct PreparedOperationContextBinding<'a> {
    operation: &'a IssuedLocalOperation,
    predecessor: (ResidentRecordRefV1, OperationObservationV1),
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
}

/// Exact context observation proved to have completed durable custody.
///
/// This token carries no context body and grants no action by itself. Attempt
/// preparation uses it only to prevent an uncommitted `ContextBound`
/// observation from becoming dispatch authority.
pub struct CompletedOperationContextBinding<'a> {
    operation: &'a IssuedLocalOperation,
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
}

impl<'a> PreparedOperationContextBinding<'a> {
    /// Borrow the exact operation whose context was admitted.
    pub(in crate::resident) const fn operation(&self) -> &'a IssuedLocalOperation {
        self.operation
    }

    /// Borrow the signed observation. It is not yet durable.
    #[must_use]
    pub const fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    /// Prepare the existing atomic observation append.
    ///
    /// The context body stays with its semantic owner. Pale stores only the
    /// admitted immutable reference in the observation.
    ///
    /// # Errors
    /// Rejects inconsistent lineage or authority-owned storage selection.
    pub fn prepare_append(
        &self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    ) -> Result<PreparedOperationObservationAppend<'_>, OperationObservationCustodyProjectionError>
    {
        let (handle_ref, handle) = self.operation.handle();
        prepare_operation_observation_append(
            OperationObservationTransition::new(
                handle_ref,
                handle,
                &self.predecessor.0,
                &self.predecessor.1,
                &self.observation_ref,
                &self.observation,
            ),
            resolve_store,
        )
    }

    /// Confirm that this exact context binding completed durable custody.
    ///
    /// # Errors
    /// Rejects a completion for another append or an inconsistent store
    /// projection. This performs no read, write, or external call.
    pub fn finish(
        self,
        completed: &super::super::CompletedOperationObservationCustody,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    ) -> Result<CompletedOperationContextBinding<'a>, OperationAttemptError> {
        self.prepare_append(resolve_store)?
            .verify_completed_custody(completed)?;
        Ok(CompletedOperationContextBinding {
            operation: self.operation,
            observation_ref: self.observation_ref,
            observation: self.observation,
        })
    }

    /// Confirm completed custody while the process owner retains this preparation.
    pub(in crate::resident) fn finish_borrowed(
        &self,
        completed: &super::super::CompletedOperationObservationCustody,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    ) -> Result<CompletedOperationContextBinding<'a>, OperationAttemptError> {
        self.prepare_append(resolve_store)?
            .verify_completed_custody(completed)?;
        Ok(CompletedOperationContextBinding {
            operation: self.operation,
            observation_ref: self.observation_ref.clone(),
            observation: self.observation.clone(),
        })
    }
}

impl CompletedOperationContextBinding<'_> {
    /// Borrow the exact durably retained context observation.
    #[must_use]
    pub const fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    pub(in crate::resident) fn matches(
        &self,
        operation: &IssuedLocalOperation,
        predecessor: (&ResidentRecordRefV1, &OperationObservationV1),
    ) -> bool {
        self.operation.handle().0 == operation.handle().0
            && self
                .operation
                .handle()
                .1
                .canonical_operation_handle_sha256()
                == operation.handle().1.canonical_operation_handle_sha256()
            && &self.observation_ref == predecessor.0
            && self.observation.canonical_operation_observation_sha256()
                == predecessor.1.canonical_operation_observation_sha256()
    }

    /// Reconstruct context continuation from anchored resident history.
    ///
    /// Observation recovery already admits the selected record, checkpoint
    /// ancestry, accumulator membership, and independent anchor. This method
    /// additionally requires an admitted `ContextBound` member in that history
    /// for the exact issued operation and its retained evidence-bearing lease.
    /// The selected head can be a later phase. It performs no read, write,
    /// context resolution, signing, or provider call.
    ///
    /// # Errors
    /// Rejects a substituted or self-contained operation. A missing required
    /// context witness returns [`OperationAttemptError::ContextRequired`].
    pub fn recover<'a>(
        operation: &'a IssuedLocalOperation,
        recovered: &super::super::RecoveredOperationObservation,
    ) -> Result<CompletedOperationContextBinding<'a>, OperationAttemptError> {
        let (handle_ref, handle) = operation.handle();
        let historical_lease = operation.historical_operation_lease();
        if !operation
            .checkpoint_selection()
            .matches_store(recovered.store())
            || historical_lease.evidence_obligations().is_empty()
            || handle.operation_lease_sha256() != historical_lease.operation_lease_digest()
        {
            return Err(OperationAttemptError::Association);
        }
        let selected = recovered
            .context_binding()
            .ok_or(OperationAttemptError::ContextRequired)?;
        if selected.1.operation_handle_ref() != handle_ref
            || selected.1.operation_handle_sha256() != handle.canonical_operation_handle_sha256()
            || !matches!(
                selected.1.state(),
                OperationObservationStateV1::Active {
                    phase: OperationActivePhaseV1::ContextBound { .. },
                }
            )
        {
            return Err(OperationAttemptError::Association);
        }
        Ok(CompletedOperationContextBinding {
            operation,
            observation_ref: selected.0.clone(),
            observation: selected.1.clone(),
        })
    }
}

/// Admit one exact context projection and sign the next operation observation.
///
/// `admit_evidence` must resolve the projection through its existing owner and
/// prove that it satisfies the exact evidence obligations retained by the
/// operation lease. This function does not read context, write storage, call a
/// provider, or invoke a native handler.
///
/// # Errors
/// Rejects an operation without evidence obligations, stale authority, unlike
/// request or lease data, a non-`Resolving` predecessor, reused references,
/// rejected context evidence, or signing failure.
pub fn prepare_operation_context_binding<'operation, 'input, Authority, Evidence, Rotation, Sign>(
    input: OperationContextBindingInput<'operation, 'input>,
    authority: &mut Authority,
    admit_evidence: Evidence,
    admit_rotation: Rotation,
    sign: Sign,
) -> Result<PreparedOperationContextBinding<'operation>, OperationAttemptError>
where
    Authority: AmbientSessionAuthorityPort,
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
    validate_operation_continuation_at(
        input.operation,
        input.request,
        input.session,
        input.lease,
        input.observation,
        input.current,
        authority,
    )?;
    let obligations = input.lease.evidence_obligations();
    if obligations.is_empty() {
        return Err(OperationAttemptError::ContextNotRequired);
    }
    let predecessor = input
        .observation
        .predecessor
        .ok_or(OperationAttemptError::Association)?;
    if !matches!(
        predecessor.1.state(),
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Resolving,
        }
    ) {
        return Err(OperationAttemptError::Association);
    }
    let (handle_ref, handle) = input.operation.handle();
    if input.observation_ref == *handle_ref
        || input.observation_ref == *handle.operation_lease_ref()
        || input.observation_ref == *input.operation.request_coordinate().record_ref()
        || input.observation_ref == *predecessor.0
        || input.observation_ref == input.evidence.record_ref
        || input.evidence.record_ref == *handle_ref
        || input.evidence.record_ref == *handle.operation_lease_ref()
        || input.evidence.record_ref == *input.operation.request_coordinate().record_ref()
        || input.evidence.record_ref == *predecessor.0
    {
        return Err(OperationAttemptError::Association);
    }
    signing::validate_runtime_signer(
        input.observation.signed.signing_authority,
        input.current,
        OPERATION_OBSERVATION_V1_SCHEMA,
    )?;
    let state = OperationObservationStateV1::Active {
        phase: OperationActivePhaseV1::ContextBound {
            evidence: input.evidence.clone(),
        },
    };
    let evidence = input.evidence;
    let observation = signing::issue_observation_with_state_admission(
        input.observation,
        state.clone(),
        input.current.now,
        |proposed| {
            if proposed != &state {
                return Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "operation context binding",
                    invariant: "exact admitted context transition",
                });
            }
            admit_evidence(&evidence, obligations)
        },
        admit_rotation,
        sign,
    )?;
    Ok(PreparedOperationContextBinding {
        operation: input.operation,
        predecessor: (predecessor.0.clone(), predecessor.1.clone()),
        observation_ref: input.observation_ref,
        observation,
    })
}
