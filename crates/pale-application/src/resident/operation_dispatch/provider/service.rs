//! Retain provider resolution, attempt preparation, and dispatch before release.

use super::*;
use crate::resident::operation_dispatch::custody::{append_checkpoint, sample_time};
use crate::resident::CompletedOperationObservationCustody;
use pale_ports::{
    AccumulatorPreparationLimits, AntiRollbackAnchorPort, OperationCustodyPort, WallClockSourcePort,
};

/// Resolved inputs for retaining one provider dispatch without releasing it.
pub struct ProviderOperationDispatchInput<'operation, 'call> {
    /// Original operation and exact source-selected provider attempt.
    pub operation: ProviderOperationInput<'operation>,
    /// Checkpoint containing the admitted `Resolving` or `ContextBound` predecessor.
    pub checkpoint: (&'call ResidentRecordRefV1, &'call ControlStateCheckpointV1),
    /// Independently selected checkpoint schema.
    pub checkpoint_schema: &'call SchemaDigest,
    /// Resolved checkpoint signer for each newly issued checkpoint.
    pub checkpoint_authority: &'call ControlCheckpointSigningAuthorityAdmissionContextV1<'call>,
    /// Resolved operation-observation signer.
    pub signing: SignedResidentAdmissionContextV1<'call>,
    /// Checkpoint for provider-resolution custody.
    pub resolution_checkpoint: OperationCheckpointInput,
    /// Checkpoint for private attempt-selection custody.
    pub preparation_checkpoint: OperationCheckpointInput,
    /// Checkpoint for dispatch custody.
    pub dispatch_checkpoint: OperationCheckpointInput,
    /// Fresh dispatch-observation reference.
    pub dispatch_ref: ResidentRecordRefV1,
    /// Selected time source; it does not establish clock trust.
    pub clock: &'call dyn WallClockSourcePort,
    /// Existing owner of the observation's store coordinate.
    pub resolve_store: &'call dyn OperationObservationAuthorityStoreResolver,
    /// Existing finite accumulator-preparation limits.
    pub accumulator_limits: AccumulatorPreparationLimits,
}

/// Exact newly retained provider dispatch, still unopened by any carrier.
pub struct RetainedProviderOperationDispatch<'operation> {
    pub(super) dispatch: PreparedProviderOperationDispatch<'operation>,
    pub(super) custody: CompletedOperationObservationCustody,
    pub(super) checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
}

impl<'operation> RetainedProviderOperationDispatch<'operation> {
    /// Borrow the original issued operation with its source lifetime preserved.
    #[must_use]
    pub(in crate::resident) const fn operation(&self) -> &'operation IssuedLocalOperation {
        self.dispatch.attempt.context.operation
    }

    /// Borrow the exact original operation context without renewing authority.
    #[must_use]
    pub const fn context(&self) -> &OperationAttemptContext<'_> {
        self.dispatch.context()
    }

    /// Borrow the exact provider request without releasing it.
    ///
    /// # Errors
    ///
    /// Returns an error if the retained selection no longer matches its frozen
    /// operation, request, generation, or provider coordinates.
    pub fn provider_request(&self) -> Result<ProviderInvocationRequest<'_>, OperationAttemptError> {
        self.dispatch.provider_request()
    }

    /// Borrow the committed dispatch observation.
    #[must_use]
    pub fn observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.dispatch.observation()
    }

    /// Borrow the exact completed database-and-anchor custody.
    #[must_use]
    pub const fn custody(&self) -> &CompletedOperationObservationCustody {
        &self.custody
    }

    /// Borrow the checkpoint that committed the dispatch.
    #[must_use]
    pub fn checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.checkpoint.0, &self.checkpoint.1)
    }

    /// Verify fresh `New` custody before a later carrier consumes this phase.
    ///
    /// This performs bounded reads only. It does not call a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the physical dispatch, checkpoint, database evidence,
    /// or independent anchor differs from the retained phase.
    pub fn verify_release_custody(
        &self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
        maximum_record_bytes: usize,
    ) -> Result<(), OperationAttemptError> {
        self.dispatch
            .prepare_append(resolve_store)?
            .verify_fresh_dispatch_custody(&self.custody, custody, anchor, maximum_record_bytes)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "../../../../tests/internal/resident/operation_dispatch/provider_service.rs"]
pub(in crate::resident) mod test_support;

/// Provider-dispatch retention stopped at one exact recoverable phase.
pub enum ProviderOperationRetentionFailure<'operation> {
    /// Nothing was signed or retained.
    Admission(OperationAttemptError),
    /// Signed provider resolution remains unopened with its append failure.
    Resolution {
        /// Exact signed provider resolution.
        resolution: Box<PreparedProviderResolution<'operation>>,
        /// Exact projection, commit, or anchor failure.
        failure: Box<OperationAppendFailure>,
    },
    /// Resolution is durable, but attempt preparation was rejected.
    AttemptPreparation {
        /// Exact signed resolution, retained by the failed transition.
        failure: Box<ProviderOperationPreparationFailure<'operation>>,
        /// Completed resolution custody.
        completed: Box<CompletedOperationObservationCustody>,
        /// Exact checkpoint that committed the resolution.
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
    },
    /// Signed private attempt remains unopened with its append failure.
    Attempt {
        /// Exact signed attempt preparation.
        attempt: Box<PreparedProviderOperationAttempt<'operation>>,
        /// Exact projection, commit, or anchor failure.
        failure: Box<OperationAppendFailure>,
    },
    /// Attempt preparation is durable, but dispatch signing was rejected.
    DispatchPreparation {
        /// Exact unopened attempt, retained by the failed transition.
        failure: Box<ProviderOperationDispatchFailure<'operation>>,
        /// Completed attempt custody.
        completed: Box<CompletedOperationObservationCustody>,
        /// Exact checkpoint that committed the attempt.
        checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
    },
    /// Signed dispatch remains unopened with its append failure.
    Dispatch {
        /// Exact signed dispatch.
        dispatch: Box<PreparedProviderOperationDispatch<'operation>>,
        /// Exact projection, commit, or anchor failure.
        failure: Box<OperationAppendFailure>,
    },
}

impl std::fmt::Debug for ProviderOperationRetentionFailure<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Admission(_) => "Admission",
            Self::Resolution { .. } => "Resolution",
            Self::AttemptPreparation { .. } => "AttemptPreparation",
            Self::Attempt { .. } => "Attempt",
            Self::DispatchPreparation { .. } => "DispatchPreparation",
            Self::Dispatch { .. } => "Dispatch",
        };
        formatter
            .debug_struct("ProviderOperationRetentionFailure")
            .field("phase", &phase)
            .finish_non_exhaustive()
    }
}

/// Retain one external-provider dispatch through database and anchor custody.
///
/// The function performs no provider, A2A, context, or effect call. A later
/// carrier-specific phase must recheck current authority and fresh `New`
/// dispatch custody before it can consume the returned value.
///
/// # Errors
///
/// Returns the exact stopped phase when admission, signing, checkpoint
/// preparation, database custody, or independent anchoring fails.
#[allow(clippy::too_many_arguments)]
pub fn retain_provider_operation_dispatch<
    'operation,
    'call,
    Authority,
    Rotation,
    ObservationSign,
    CheckpointSign,
>(
    input: ProviderOperationDispatchInput<'operation, 'call>,
    authority: &mut Authority,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
    mut admit_rotation: Rotation,
    mut sign_observation: ObservationSign,
    mut sign_checkpoint: CheckpointSign,
) -> Result<
    RetainedProviderOperationDispatch<'operation>,
    ProviderOperationRetentionFailure<'operation>,
>
where
    Authority: AmbientSessionAuthorityPort,
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
    validate_references(&input).map_err(ProviderOperationRetentionFailure::Admission)?;
    let ProviderOperationDispatchInput {
        operation,
        checkpoint,
        checkpoint_schema,
        checkpoint_authority,
        signing,
        resolution_checkpoint,
        preparation_checkpoint,
        dispatch_checkpoint,
        dispatch_ref,
        clock,
        resolve_store,
        accumulator_limits,
    } = input;
    let issued = operation.context.operation;
    let original_current = operation.context.current;
    let now = sample_time(clock, original_current.now.max(*checkpoint.1.issued_at()))
        .map_err(ProviderOperationRetentionFailure::Admission)?;
    let current = original_current.at_time(now);
    let resolution = prepare_provider_resolution_at(
        operation,
        &current,
        authority,
        &mut admit_rotation,
        &mut sign_observation,
    )
    .map_err(ProviderOperationRetentionFailure::Admission)?;
    let retained_resolution = resolution
        .prepare_append(resolve_store)
        .map_err(OperationAppendFailure::from)
        .and_then(|append| {
            append_checkpoint(
                append,
                issued,
                checkpoint,
                resolution_checkpoint,
                now,
                checkpoint_schema,
                checkpoint_authority,
                accumulator_limits,
                custody,
                anchor,
                &mut sign_checkpoint,
            )
        });
    let (resolution_done, resolution_checkpoint) = match retained_resolution {
        Ok(value) => value,
        Err(failure) => {
            return Err(ProviderOperationRetentionFailure::Resolution {
                resolution: Box::new(resolution),
                failure: Box::new(failure),
            });
        }
    };

    let now = match sample_time(clock, now) {
        Ok(value) => value,
        Err(reason) => {
            return Err(ProviderOperationRetentionFailure::AttemptPreparation {
                failure: Box::new(ProviderOperationPreparationFailure {
                    resolution: Box::new(resolution),
                    reason,
                }),
                completed: Box::new(resolution_done),
                checkpoint: Box::new(resolution_checkpoint),
            });
        }
    };
    let current = original_current.at_time(now);
    let attempt = match resolution.prepare_attempt(
        &resolution_done,
        &current,
        resolve_store,
        authority,
        &mut admit_rotation,
        &mut sign_observation,
    ) {
        Ok(value) => value,
        Err(failure) => {
            return Err(ProviderOperationRetentionFailure::AttemptPreparation {
                failure: Box::new(failure),
                completed: Box::new(resolution_done),
                checkpoint: Box::new(resolution_checkpoint),
            });
        }
    };
    let retained_attempt = attempt
        .prepare_append(resolve_store)
        .map_err(OperationAppendFailure::from)
        .and_then(|append| {
            append_checkpoint(
                append,
                issued,
                (&resolution_checkpoint.0, &resolution_checkpoint.1),
                preparation_checkpoint,
                now,
                checkpoint_schema,
                checkpoint_authority,
                accumulator_limits,
                custody,
                anchor,
                &mut sign_checkpoint,
            )
        });
    let (attempt_done, attempt_checkpoint) = match retained_attempt {
        Ok(value) => value,
        Err(failure) => {
            return Err(ProviderOperationRetentionFailure::Attempt {
                attempt: Box::new(attempt),
                failure: Box::new(failure),
            });
        }
    };

    let now = match sample_time(clock, now) {
        Ok(value) => value,
        Err(reason) => {
            return Err(ProviderOperationRetentionFailure::DispatchPreparation {
                failure: Box::new(ProviderOperationDispatchFailure {
                    attempt: Box::new(attempt),
                    reason,
                }),
                completed: Box::new(attempt_done),
                checkpoint: Box::new(attempt_checkpoint),
            });
        }
    };
    let current = original_current.at_time(now);
    let dispatch = match attempt.prepare_dispatch(
        ProviderOperationDispatchContext {
            completed: &attempt_done,
            current: &current,
            signing,
            observation_ref: dispatch_ref,
            resolve_store,
        },
        authority,
        &mut admit_rotation,
        &mut sign_observation,
    ) {
        Ok(value) => value,
        Err(failure) => {
            return Err(ProviderOperationRetentionFailure::DispatchPreparation {
                failure: Box::new(failure),
                completed: Box::new(attempt_done),
                checkpoint: Box::new(attempt_checkpoint),
            });
        }
    };
    let retained_dispatch = dispatch
        .prepare_append(resolve_store)
        .map_err(OperationAppendFailure::from)
        .and_then(|append| {
            append_checkpoint(
                append,
                issued,
                (&attempt_checkpoint.0, &attempt_checkpoint.1),
                dispatch_checkpoint,
                now,
                checkpoint_schema,
                checkpoint_authority,
                accumulator_limits,
                custody,
                anchor,
                &mut sign_checkpoint,
            )
        });
    match retained_dispatch {
        Ok((custody, checkpoint)) => Ok(RetainedProviderOperationDispatch {
            dispatch,
            custody,
            checkpoint,
        }),
        Err(failure) => Err(ProviderOperationRetentionFailure::Dispatch {
            dispatch: Box::new(dispatch),
            failure: Box::new(failure),
        }),
    }
}

fn validate_references(
    input: &ProviderOperationDispatchInput<'_, '_>,
) -> Result<(), OperationAttemptError> {
    let context = &input.operation.context;
    context.validate_original_session()?;
    let planned = [
        &input.operation.resolution_ref,
        &context.selection_ref,
        &context.observation_ref,
        &input.dispatch_ref,
        &input.resolution_checkpoint.reference,
        &input.preparation_checkpoint.reference,
        &input.dispatch_checkpoint.reference,
    ];
    let original = [
        context.operation.handle().0,
        context.operation.request_coordinate().record_ref(),
        context.operation.handle().1.operation_lease_ref(),
        input.checkpoint.0,
    ];
    for (index, reference) in planned.iter().enumerate() {
        if planned[..index].contains(reference)
            || original.contains(reference)
            || context
                .observation
                .predecessor
                .is_some_and(|(prior, _)| prior == *reference)
        {
            return Err(OperationAttemptError::Association);
        }
    }
    let ids = [
        &input.resolution_checkpoint.id,
        &input.preparation_checkpoint.id,
        &input.dispatch_checkpoint.id,
    ];
    if ids.iter().enumerate().any(|(index, id)| {
        ids[..index].contains(id) || *id == input.checkpoint.1.control_checkpoint_id()
    }) {
        return Err(OperationAttemptError::Association);
    }
    Ok(())
}
