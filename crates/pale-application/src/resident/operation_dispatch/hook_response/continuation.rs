//! Exact signed response phases retained by the original occurrence's owner.

use super::*;
use crate::resident::operation_dispatch::checkpoint_authority::RetainedCheckpointAuthority;
use crate::resident::CompletedOperationObservationCustody;

pub(in crate::resident) struct NativeHookResponseContinuation {
    pub(super) request: CommonRequestEnvelopeV1,
    pub(super) response: ResidentHookResponseRecord,
    pub(super) response_canonical: String,
    pub(super) record: ControlRecord,
    pub(super) preparation: ResponsePhase,
    pub(super) dispatch: Option<ResponsePhase>,
    pub(super) dispatch_ref: ResidentRecordRefV1,
    pub(super) dispatch_checkpoint: OperationCheckpointInput,
    pub(super) attempt_id: CapabilityAttemptIdV1,
    pub(super) returned: bool,
    pub(super) signing_authority: SigningAuthorityBindingV1,
    pub(super) observation_schema: SchemaDigest,
    pub(super) checkpoint_schema: SchemaDigest,
    pub(super) checkpoint_authority: RetainedCheckpointAuthority,
    pub(super) limits: AccumulatorPreparationLimits,
    pub(super) maximum_record_bytes: usize,
}

impl std::fmt::Debug for NativeHookResponseContinuation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeHookResponseContinuation")
            .field("returned", &self.returned)
            .finish_non_exhaustive()
    }
}

impl NativeHookResponseContinuation {
    /// A lender may not replace the saved identities, bounds or signer selection.
    /// Current permission still requires fresh admission on every continuation.
    pub(super) fn matches(
        &self,
        turn: &NativeHookResponseTurn<'_>,
        inputs: &NativeHookResponseInputs<'_>,
    ) -> bool {
        self.request == *turn.request
            && self.response.handle_sha256
                == *turn
                    .correlated
                    .operation()
                    .handle()
                    .1
                    .canonical_operation_handle_sha256()
            && self.record.coordinate().record_ref() == &inputs.response_ref
            && self.preparation.observation.0 == inputs.preparation_ref
            && self.dispatch_ref == inputs.dispatch_ref
            && self.attempt_id == inputs.attempt_id
            && self.preparation.identity.reference == inputs.preparation_checkpoint.reference
            && self.preparation.identity.id == inputs.preparation_checkpoint.id
            && self.dispatch_checkpoint.reference == inputs.dispatch_checkpoint.reference
            && self.dispatch_checkpoint.id == inputs.dispatch_checkpoint.id
            && self.signing_authority == *inputs.signing.signing_authority
            && self.observation_schema == *inputs.signing.expected_schema_sha256
            && self.checkpoint_schema == *inputs.checkpoint_schema
            && self.checkpoint_authority
                == RetainedCheckpointAuthority::new(inputs.checkpoint_authority)
            && self.limits == inputs.accumulator_limits
            && self.maximum_record_bytes == inputs.maximum_record_bytes
    }
}

pub(in crate::resident) struct ResponsePhase {
    pub(in crate::resident) predecessor: (ResidentRecordRefV1, OperationObservationV1),
    pub(in crate::resident) observation: (ResidentRecordRefV1, OperationObservationV1),
    pub(super) prior_checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    pub(super) identity: OperationCheckpointInput,
    pub(in crate::resident) checkpoint: Option<ControlStateCheckpointV1>,
    pending: Option<OperationAppendFailure>,
    pub(in crate::resident) completed: Option<CompletedOperationObservationCustody>,
}

impl ResponsePhase {
    pub(in crate::resident) fn new(
        predecessor: (&ResidentRecordRefV1, &OperationObservationV1),
        observation: (ResidentRecordRefV1, OperationObservationV1),
        prior_checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
        identity: OperationCheckpointInput,
    ) -> Self {
        Self {
            predecessor: (predecessor.0.clone(), predecessor.1.clone()),
            observation,
            prior_checkpoint: (prior_checkpoint.0.clone(), prior_checkpoint.1.clone()),
            identity,
            checkpoint: None,
            pending: None,
            completed: None,
        }
    }

    /// Retain the issued checkpoint before attachment. A later turn can rebuild
    /// roots or finish the exact physical phase, but cannot sign a replacement.
    pub(in crate::resident) fn complete(
        &mut self,
        operation: &IssuedLocalOperation,
        record: Option<&ControlRecord>,
        inputs: &mut ResponsePhaseInputs<'_>,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<(), NativeHookResponseFailure> {
        if self.completed.is_some() {
            return Ok(());
        }
        if let Some(pending) = self.pending.take() {
            return match pending.continue_custody(custody, anchor) {
                Ok((completed, checkpoint)) => {
                    debug_assert_eq!(self.checkpoint.as_ref(), Some(&checkpoint.1));
                    self.completed = Some(completed);
                    Ok(())
                }
                Err(pending) => {
                    self.pending = Some(pending);
                    Err(NativeHookResponseFailure::Custody)
                }
            };
        }
        let append = phase_append(
            operation,
            &self.predecessor,
            &self.observation,
            record,
            inputs.resolve_store,
        )
        .map_err(|_| NativeHookResponseFailure::Custody)?;
        let prepared = append
            .prepare_checkpoint(
                operation.checkpoint_selection(),
                &self.prior_checkpoint.0,
                &self.prior_checkpoint.1,
                inputs.accumulator_limits,
                custody,
            )
            .map_err(|_| NativeHookResponseFailure::Custody)?;
        if self.checkpoint.is_none() {
            let preimage = custody::observation_checkpoint_preimage(
                &prepared,
                (&self.prior_checkpoint.0, &self.prior_checkpoint.1),
                &self.identity,
                *self.observation.1.observed_at(),
                inputs.checkpoint_schema,
                inputs.checkpoint_authority,
            )?;
            self.checkpoint = Some(
                prepared
                    .issue_checkpoint_value(
                        preimage,
                        inputs.checkpoint_schema,
                        inputs.checkpoint_authority,
                        &mut *inputs.sign_checkpoint,
                    )
                    .map_err(|_| NativeHookResponseFailure::Custody)?,
            );
        }
        let checkpoint = self
            .checkpoint
            .as_ref()
            .expect("checkpoint was issued before attachment");
        let physical = prepared
            .attach_issued_checkpoint(&self.identity.reference, checkpoint)
            .map_err(|_| NativeHookResponseFailure::Custody)?;
        match custody::commit_prepared_checkpoint(
            physical,
            (self.identity.reference.clone(), checkpoint.clone()),
            custody,
            anchor,
        ) {
            Ok((completed, _)) => {
                self.completed = Some(completed);
                Ok(())
            }
            Err(pending) => {
                self.pending = Some(pending);
                Err(NativeHookResponseFailure::Custody)
            }
        }
    }
}

pub(in crate::resident) fn phase_append<'a>(
    operation: &'a IssuedLocalOperation,
    predecessor: &'a (ResidentRecordRefV1, OperationObservationV1),
    observation: &'a (ResidentRecordRefV1, OperationObservationV1),
    record: Option<&ControlRecord>,
    resolver: &dyn OperationObservationAuthorityStoreResolver,
) -> Result<PreparedOperationObservationAppend<'a>, OperationObservationCustodyProjectionError> {
    let append = prepare_operation_observation_append(
        OperationObservationTransition::new(
            operation.handle().0,
            operation.handle().1,
            &predecessor.0,
            &predecessor.1,
            &observation.0,
            &observation.1,
        ),
        resolver,
    )?;
    Ok(match record {
        Some(record) => {
            append.with_associated_record(OperationCustodyRecord::HookResponse(record.clone()))
        }
        None => append,
    })
}

/// Physical inputs for one retained response phase; authority is selected by its owner.
pub(in crate::resident) struct ResponsePhaseInputs<'a> {
    /// Checkpoint schema selected before this phase was signed.
    pub checkpoint_schema: &'a SchemaDigest,
    /// Independently admitted checkpoint signing authority.
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    /// Resolver for the signed observation's authority store.
    pub resolve_store: &'a dyn OperationObservationAuthorityStoreResolver,
    /// Finite accumulator work limits.
    pub accumulator_limits: AccumulatorPreparationLimits,
    /// Sign the exact checkpoint once, before retaining it across attachment failures.
    pub sign_checkpoint: &'a mut HookResponseCheckpointSigner<'a>,
}
