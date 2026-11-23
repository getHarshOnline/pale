//! Advance an authenticated recovered Invoke head from admitted conclusive evidence.
//!
//! This module owns process-local selection only. The observation owner still
//! admits and signs the terminal state, and the existing custody owner still
//! prepares, commits, reads back, and anchors the exact successor.

use super::*;
use crate::resident::operation_dispatch::{
    verify_recovered_operation_head_custody, OperationAppendFailure, OperationAttemptError,
    OperationCheckpointInput,
};
use crate::resident::operation_observation_custody::reconciliation_append::{
    append_current_head_reconciliation, CompletedCurrentHeadReconciliation,
    CurrentHeadReconciliationAnchorPending, CurrentHeadReconciliationAppendFailure,
    CurrentHeadReconciliationAppendInput, CurrentHeadReconciliationCommitFailure,
    IssuedCurrentHeadReconciliationCheckpoint, PreparedCurrentHeadReconciliation,
    RetryableCurrentHeadReconciliationCommit,
};
use crate::resident::{
    CompletedOperationObservationCustody, OperationObservationAuthorityStoreResolver,
};
use pale_core::records::{
    ControlStateCheckpointV1, OperationObservationStateV1, OperationObservationV1, SchemaDigest,
    SignedResidentAdmissionContextV1,
};
use pale_ports::AccumulatorPreparationLimits;

use super::restart::ResidentProcessRecoveredOperationStart;

/// Exact semantic and custody inputs for one recovered-head reconciliation.
pub(super) struct ResidentProcessReconciliationInput<'a> {
    pub(super) successor_ref: ResidentRecordRefV1,
    pub(super) state: OperationObservationStateV1,
    pub(super) observed_at: UnixTimeV1,
    pub(super) signed: SignedResidentAdmissionContextV1<'a>,
    pub(super) checkpoint: OperationCheckpointInput,
    pub(super) checkpoint_schema: &'a SchemaDigest,
    pub(super) checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    pub(super) accumulator_limits: AccumulatorPreparationLimits,
    pub(super) maximum_record_bytes: usize,
}

/// Selector-bound successful reconciliation.
#[must_use = "preserve the completed reconciliation custody proof"]
pub(super) struct ResidentProcessCompletedReconciliation {
    selector: ResidentDeploymentAuthoritySelector,
    completed: CompletedCurrentHeadReconciliation,
}

impl ResidentProcessCompletedReconciliation {
    pub(super) fn successor(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        self.completed.successor()
    }

    pub(super) fn checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        self.completed.checkpoint()
    }

    pub(super) fn into_custody(self) -> CompletedOperationObservationCustody {
        self.completed.into_custody()
    }
}

/// Signed successor awaiting checkpoint issuance and custody.
#[must_use = "continue this exact reconciliation successor"]
#[derive(Debug)]
pub(super) struct ResidentProcessPreparedReconciliation {
    selector: ResidentDeploymentAuthoritySelector,
    continuation: PreparedCurrentHeadReconciliation,
    maximum_record_bytes: usize,
}

/// Signed successor and checkpoint awaiting attachment and custody.
#[must_use = "continue this exact reconciliation checkpoint"]
#[derive(Debug)]
pub(super) struct ResidentProcessIssuedReconciliation {
    selector: ResidentDeploymentAuthoritySelector,
    continuation: IssuedCurrentHeadReconciliationCheckpoint,
    maximum_record_bytes: usize,
}

/// Candidate whose database commit outcome still requires classification.
#[must_use = "retry or read back this exact reconciliation candidate"]
#[derive(Debug)]
pub(super) struct ResidentProcessReconciliationCommitFailure {
    selector: ResidentDeploymentAuthoritySelector,
    continuation: CurrentHeadReconciliationCommitFailure,
    maximum_record_bytes: usize,
}

/// Candidate proved uncommitted and permitted one exact commit retry.
#[must_use = "commit this exact reconciliation candidate"]
pub(super) struct ResidentProcessRetryableReconciliationCommit {
    selector: ResidentDeploymentAuthoritySelector,
    continuation: RetryableCurrentHeadReconciliationCommit,
    maximum_record_bytes: usize,
}

impl std::fmt::Debug for ResidentProcessRetryableReconciliationCommit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentProcessRetryableReconciliationCommit")
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for ResidentProcessRecoveredOperationStart {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentProcessRecoveredOperationStart")
            .finish_non_exhaustive()
    }
}

/// Database-committed successor awaiting its independent anchor.
#[must_use = "continue this exact independent-anchor transition"]
#[derive(Debug)]
pub(super) struct ResidentProcessReconciliationAnchorPending {
    selector: ResidentDeploymentAuthoritySelector,
    continuation: CurrentHeadReconciliationAnchorPending,
    maximum_record_bytes: usize,
}

/// Exact process-host continuation retained after a selection failure.
#[must_use = "retry only the retained reconciliation phase"]
pub(super) enum ResidentProcessReconciliationContinuation {
    Start(Box<ResidentProcessRecoveredOperationStart>),
    Prepared(Box<ResidentProcessPreparedReconciliation>),
    Issued(Box<ResidentProcessIssuedReconciliation>),
    Commit(Box<ResidentProcessReconciliationCommitFailure>),
    RetryableCommit(Box<ResidentProcessRetryableReconciliationCommit>),
    Anchor(Box<ResidentProcessReconciliationAnchorPending>),
}

/// Failure while advancing one recovered head to admitted terminal evidence.
#[derive(Debug, thiserror::Error)]
pub(super) enum ResidentProcessReconciliationFailure {
    /// Process or deployment selection differed before a phase could continue.
    #[error("reconciliation continuation does not match the selected deployment")]
    Selection {
        continuation: Box<ResidentProcessReconciliationContinuation>,
        reason: ResidentProcessHostSelectionError,
    },
    /// The physical head, checkpoint, or independent anchor changed after recovery.
    #[error("recovered reconciliation head no longer matches physical custody")]
    HeadChanged {
        recovered: Box<ResidentProcessRecoveredOperationStart>,
        reason: OperationAttemptError,
    },
    /// The evidence or successor was rejected before any successor was retained.
    #[error("conclusive reconciliation evidence was rejected")]
    BeforeSuccessor(OperationAppendFailure),
    /// A signed successor exists and must be continued without re-signing it.
    #[error("reconciliation successor requires checkpoint continuation")]
    Prepared {
        continuation: Box<ResidentProcessPreparedReconciliation>,
        reason: Box<OperationAppendFailure>,
    },
    /// The retained predecessor changed before checkpoint signing resumed.
    #[error("reconciliation predecessor changed before checkpoint signing")]
    PreparedHeadChanged {
        continuation: Box<ResidentProcessPreparedReconciliation>,
        reason: OperationAttemptError,
    },
    /// A signed checkpoint exists and must be attached without reissuing it.
    #[error("reconciliation checkpoint requires custody continuation")]
    Issued {
        continuation: Box<ResidentProcessIssuedReconciliation>,
        reason: Box<OperationAppendFailure>,
    },
    /// Database commit status is retained by the existing custody continuation.
    #[error("reconciliation database outcome requires exact continuation")]
    Commit(Box<ResidentProcessReconciliationCommitFailure>),
    /// Only the independent anchor remains for the committed successor.
    #[error("reconciliation independent anchor requires exact continuation")]
    Anchor(Box<ResidentProcessReconciliationAnchorPending>),
}

impl std::fmt::Debug for ResidentProcessReconciliationContinuation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self {
            Self::Start(_) => "Start",
            Self::Prepared(_) => "Prepared",
            Self::Issued(_) => "Issued",
            Self::Commit(_) => "Commit",
            Self::RetryableCommit(_) => "RetryableCommit",
            Self::Anchor(_) => "Anchor",
        };
        formatter
            .debug_tuple("ResidentProcessReconciliationContinuation")
            .field(&phase)
            .finish()
    }
}

impl<'store, WriterGuard> ResidentProcessHost<'store, WriterGuard> {
    /// Append caller-admitted terminal evidence to a recovered Invoke head.
    ///
    /// Historical recovery is sufficient here. This path does not require a
    /// current ambient session or a retained compiled generation, and it cannot
    /// call an action, provider, A2A route, or other carrier.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn reconcile_recovered_invoke<Evidence, Rotation, SignObservation, SignCheckpoint>(
        &self,
        recovered: ResidentProcessRecoveredOperationStart,
        access: ResidentDeploymentReconciliationAccess<'_>,
        input: ResidentProcessReconciliationInput<'_>,
        store_resolver: &dyn OperationObservationAuthorityStoreResolver,
        admit_evidence: Evidence,
        admit_rotation: Rotation,
        sign_observation: SignObservation,
        sign_checkpoint: SignCheckpoint,
    ) -> Result<ResidentProcessCompletedReconciliation, ResidentProcessReconciliationFailure>
    where
        Evidence: FnOnce(&OperationObservationStateV1) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        SignObservation: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        SignCheckpoint: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let selector = recovered.selector.clone();
        if let Err(reason) = self.validate_reconciliation_start(&recovered, &access, &input) {
            return Err(ResidentProcessReconciliationFailure::Selection {
                continuation: Box::new(ResidentProcessReconciliationContinuation::Start(Box::new(
                    recovered,
                ))),
                reason,
            });
        }
        let operation = recovered.control.operation().operation();
        if let Err(reason) = verify_recovered_operation_head_custody(
            operation,
            recovered.control.observation(),
            access.custody,
            access.anchor,
            input.maximum_record_bytes,
        ) {
            return Err(ResidentProcessReconciliationFailure::HeadChanged {
                recovered: Box::new(recovered),
                reason,
            });
        }

        let (operation, observation) = recovered.control.into_parts();
        let issued = operation.operation();
        let (handle_ref, handle) = issued.handle();
        let result = append_current_head_reconciliation(
            CurrentHeadReconciliationAppendInput {
                recovered: observation,
                handle_ref,
                handle,
                successor_ref: input.successor_ref,
                state: input.state,
                observed_at: input.observed_at,
                signed: input.signed,
                selection: issued.checkpoint_selection(),
                checkpoint: input.checkpoint,
                checkpoint_schema: input.checkpoint_schema,
                checkpoint_authority: input.checkpoint_authority,
                accumulator_limits: input.accumulator_limits,
            },
            store_resolver,
            access.custody,
            access.anchor,
            admit_evidence,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        );
        result
            .map(|completed| ResidentProcessCompletedReconciliation {
                selector: selector.clone(),
                completed,
            })
            .map_err(|failure| wrap_failure(selector, input.maximum_record_bytes, failure))
    }

    /// Continue a signed successor. Only the checkpoint signer can run again.
    pub(super) fn continue_prepared_reconciliation<SignCheckpoint>(
        &self,
        pending: ResidentProcessPreparedReconciliation,
        access: ResidentDeploymentReconciliationAccess<'_>,
        store_resolver: &dyn OperationObservationAuthorityStoreResolver,
        sign_checkpoint: SignCheckpoint,
    ) -> Result<ResidentProcessCompletedReconciliation, ResidentProcessReconciliationFailure>
    where
        SignCheckpoint: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        if let Err(reason) = self.validate_reconciliation_access(&pending.selector, access.store) {
            return Err(selection_failure(
                ResidentProcessReconciliationContinuation::Prepared(Box::new(pending)),
                reason,
            ));
        }
        if let Err(reason) = pending.continuation.verify_predecessor_custody(
            access.custody,
            access.anchor,
            pending.maximum_record_bytes,
        ) {
            return Err(ResidentProcessReconciliationFailure::PreparedHeadChanged {
                continuation: Box::new(pending),
                reason,
            });
        }
        let selector = pending.selector;
        let maximum_record_bytes = pending.maximum_record_bytes;
        pending
            .continuation
            .continue_custody(
                store_resolver,
                access.custody,
                access.anchor,
                sign_checkpoint,
            )
            .map(|completed| ResidentProcessCompletedReconciliation {
                selector: selector.clone(),
                completed,
            })
            .map_err(|failure| wrap_failure(selector, maximum_record_bytes, failure))
    }

    /// Attach the already signed checkpoint and continue physical custody.
    pub(super) fn continue_issued_reconciliation(
        &self,
        pending: ResidentProcessIssuedReconciliation,
        access: ResidentDeploymentReconciliationAccess<'_>,
        store_resolver: &dyn OperationObservationAuthorityStoreResolver,
    ) -> Result<ResidentProcessCompletedReconciliation, ResidentProcessReconciliationFailure> {
        if let Err(reason) = self.validate_reconciliation_access(&pending.selector, access.store) {
            return Err(selection_failure(
                ResidentProcessReconciliationContinuation::Issued(Box::new(pending)),
                reason,
            ));
        }
        let selector = pending.selector;
        let maximum_record_bytes = pending.maximum_record_bytes;
        pending
            .continuation
            .continue_custody(store_resolver, access.custody, access.anchor)
            .map(|completed| ResidentProcessCompletedReconciliation {
                selector: selector.clone(),
                completed,
            })
            .map_err(|failure| wrap_failure(selector, maximum_record_bytes, failure))
    }

    /// Read back one commit whose outcome remained unknown.
    pub(super) fn read_back_reconciliation_commit(
        &self,
        pending: ResidentProcessReconciliationCommitFailure,
        access: ResidentDeploymentReconciliationAccess<'_>,
    ) -> Result<ResidentProcessReconciliationAnchorPending, ResidentProcessReconciliationFailure>
    {
        if let Err(reason) = self.validate_reconciliation_access(&pending.selector, access.store) {
            return Err(selection_failure(
                ResidentProcessReconciliationContinuation::Commit(Box::new(pending)),
                reason,
            ));
        }
        let selector = pending.selector;
        let maximum_record_bytes = pending.maximum_record_bytes;
        pending
            .continuation
            .reconcile_exact(access.custody)
            .map(|continuation| ResidentProcessReconciliationAnchorPending {
                selector: selector.clone(),
                continuation,
                maximum_record_bytes,
            })
            .map_err(|continuation| {
                ResidentProcessReconciliationFailure::Commit(Box::new(
                    ResidentProcessReconciliationCommitFailure {
                        selector,
                        continuation: *continuation,
                        maximum_record_bytes,
                    },
                ))
            })
    }

    /// Commit a candidate only after the custody owner proved non-commit.
    pub(super) fn retry_reconciliation_commit(
        &self,
        pending: ResidentProcessRetryableReconciliationCommit,
        access: ResidentDeploymentReconciliationAccess<'_>,
    ) -> Result<ResidentProcessCompletedReconciliation, ResidentProcessReconciliationFailure> {
        if let Err(reason) = self.validate_reconciliation_access(&pending.selector, access.store) {
            return Err(selection_failure(
                ResidentProcessReconciliationContinuation::RetryableCommit(Box::new(pending)),
                reason,
            ));
        }
        let selector = pending.selector;
        let maximum_record_bytes = pending.maximum_record_bytes;
        pending
            .continuation
            .commit(access.custody, access.anchor)
            .map(|completed| ResidentProcessCompletedReconciliation {
                selector: selector.clone(),
                completed,
            })
            .map_err(|failure| wrap_failure(selector, maximum_record_bytes, failure))
    }

    /// Advance only the retained independent-anchor transition.
    pub(super) fn continue_reconciliation_anchor(
        &self,
        pending: ResidentProcessReconciliationAnchorPending,
        access: ResidentDeploymentReconciliationAccess<'_>,
    ) -> Result<ResidentProcessCompletedReconciliation, ResidentProcessReconciliationFailure> {
        if let Err(reason) = self.validate_reconciliation_access(&pending.selector, access.store) {
            return Err(selection_failure(
                ResidentProcessReconciliationContinuation::Anchor(Box::new(pending)),
                reason,
            ));
        }
        let selector = pending.selector;
        let maximum_record_bytes = pending.maximum_record_bytes;
        pending
            .continuation
            .continue_anchor(access.anchor)
            .map(|completed| ResidentProcessCompletedReconciliation {
                selector: selector.clone(),
                completed,
            })
            .map_err(|continuation| {
                ResidentProcessReconciliationFailure::Anchor(Box::new(
                    ResidentProcessReconciliationAnchorPending {
                        selector,
                        continuation: *continuation,
                        maximum_record_bytes,
                    },
                ))
            })
    }

    fn validate_reconciliation_access(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        store: &AuthorityStoreCoordinate,
    ) -> Result<(), ResidentProcessHostSelectionError> {
        let index = self.deployment_authority_index(selector)?;
        if self.deployments[index].store() != store {
            return Err(ResidentProcessHostSelectionError::AccessStoreAssociation);
        }
        Ok(())
    }

    fn validate_reconciliation_start(
        &self,
        recovered: &ResidentProcessRecoveredOperationStart,
        access: &ResidentDeploymentReconciliationAccess<'_>,
        input: &ResidentProcessReconciliationInput<'_>,
    ) -> Result<(), ResidentProcessHostSelectionError> {
        self.validate_reconciliation_access(&recovered.selector, access.store)?;
        if input.maximum_record_bytes == 0 {
            return Err(ResidentProcessHostSelectionError::ReconciliationBounds);
        }
        let selector = &recovered.selector;
        let control = &recovered.control;
        let operation = control.operation().operation();
        let (handle_ref, handle) = operation.handle();
        let selected = control.observation().selected();
        let head = control.observation().selected_head();
        if control.operation().store() != access.store
            || control.observation().store() != access.store
            || !recovered.lineage.admits(handle_ref, handle)
            || !selector.matches_operation(operation)
            || handle.compiled_generation_sha256() != &recovered.generation_digest
            || selected.0 != head.0
            || selected.1.canonical_operation_observation_sha256()
                != head.1.canonical_operation_observation_sha256()
            || head.1.operation_handle_ref() != handle_ref
            || head.1.operation_handle_sha256() != handle.canonical_operation_handle_sha256()
            || !matches!(
                head.1.state(),
                OperationObservationStateV1::Recovering { .. }
                    | OperationObservationStateV1::OutcomeUnknown { .. }
                    | OperationObservationStateV1::Expired { .. }
            )
            || !matches!(
                input.state,
                OperationObservationStateV1::Succeeded { .. }
                    | OperationObservationStateV1::Failed { .. }
                    | OperationObservationStateV1::Cancelled { .. }
            )
        {
            return Err(ResidentProcessHostSelectionError::ReconciliationOperationAuthority);
        }

        let session = operation.historical_session_lease();
        let lease = operation.historical_operation_lease();
        if lease.operation_lease_digest() != handle.operation_lease_sha256()
            || session.compiled_generation_digest() != handle.compiled_generation_sha256()
            || session.activation_fence_digest() != handle.activation_fence_sha256()
            || session.instance_digest() != handle.instance_sha256()
            || session.deployment_runtime_authority_ref() != &selector.runtime_authority_ref
            || session.deployment_runtime_authority_sha256().as_str()
                != selector.runtime_authority_sha256.as_str()
            || session.issuer_principal_id() != &selector.principal_id
        {
            return Err(ResidentProcessHostSelectionError::ReconciliationHistoricalAuthority);
        }

        let request = control.operation().request().to_preimage();
        if !selector.matches_signing_authority(&request.signing_authority_binding)
            || !operation_observation_matches(selector, head.1)
            || !selector.matches_signing_authority(input.signed.signing_authority)
        {
            return Err(ResidentProcessHostSelectionError::ReconciliationSigningAuthority);
        }
        let expected_observation_schema =
            pale_contracts::record_schema_identity(OPERATION_OBSERVATION_V1_SCHEMA_ID)
                .map_err(|_| ResidentProcessHostSelectionError::ReconciliationSigningAuthority)?;
        if input.signed.expected_schema_sha256 != &expected_observation_schema.schema_sha256 {
            return Err(ResidentProcessHostSelectionError::ReconciliationSigningAuthority);
        }

        let checkpoint = control.observation().current_checkpoint().1;
        let selection = operation.checkpoint_selection();
        if checkpoint.control_store_id() != access.store.store_id()
            || checkpoint.control_incarnation_id() != access.store.incarnation_id()
            || checkpoint.issuer_principal_id() != &selector.principal_id
            || !checkpoint_authority_matches(selector, access.store, input.checkpoint_authority)
            || !selection.matches_authority(input.checkpoint_authority)
            || selection.verify(access.store, handle, checkpoint).is_err()
        {
            return Err(ResidentProcessHostSelectionError::ReconciliationCheckpointAuthority);
        }
        let expected_checkpoint_schema = pale_contracts::record_schema_identity(
            CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID,
        )
        .map_err(|_| ResidentProcessHostSelectionError::ReconciliationCheckpointAuthority)?;
        if input.checkpoint_schema != &expected_checkpoint_schema.schema_sha256 {
            return Err(ResidentProcessHostSelectionError::ReconciliationCheckpointAuthority);
        }
        Ok(())
    }
}

impl ResidentProcessReconciliationCommitFailure {
    /// Recover the mutation-capable commit token only after proved non-commit.
    pub(super) fn try_into_retryable(
        self,
    ) -> Result<
        ResidentProcessRetryableReconciliationCommit,
        Box<ResidentProcessReconciliationCommitFailure>,
    > {
        let Self {
            selector,
            continuation,
            maximum_record_bytes,
        } = self;
        continuation
            .try_into_retryable()
            .map(
                |continuation| ResidentProcessRetryableReconciliationCommit {
                    selector: selector.clone(),
                    continuation,
                    maximum_record_bytes,
                },
            )
            .map_err(|continuation| {
                Box::new(ResidentProcessReconciliationCommitFailure {
                    selector,
                    continuation: *continuation,
                    maximum_record_bytes,
                })
            })
    }
}

fn wrap_failure(
    selector: ResidentDeploymentAuthoritySelector,
    maximum_record_bytes: usize,
    failure: CurrentHeadReconciliationAppendFailure,
) -> ResidentProcessReconciliationFailure {
    match failure {
        CurrentHeadReconciliationAppendFailure::BeforeSuccessor(reason) => {
            ResidentProcessReconciliationFailure::BeforeSuccessor(reason)
        }
        CurrentHeadReconciliationAppendFailure::Prepared {
            continuation,
            failure,
        } => ResidentProcessReconciliationFailure::Prepared {
            continuation: Box::new(ResidentProcessPreparedReconciliation {
                selector,
                continuation: *continuation,
                maximum_record_bytes,
            }),
            reason: failure,
        },
        CurrentHeadReconciliationAppendFailure::Issued {
            continuation,
            failure,
        } => ResidentProcessReconciliationFailure::Issued {
            continuation: Box::new(ResidentProcessIssuedReconciliation {
                selector,
                continuation: *continuation,
                maximum_record_bytes,
            }),
            reason: failure,
        },
        CurrentHeadReconciliationAppendFailure::Commit(continuation) => {
            ResidentProcessReconciliationFailure::Commit(Box::new(
                ResidentProcessReconciliationCommitFailure {
                    selector,
                    continuation: *continuation,
                    maximum_record_bytes,
                },
            ))
        }
        CurrentHeadReconciliationAppendFailure::Anchor(continuation) => {
            ResidentProcessReconciliationFailure::Anchor(Box::new(
                ResidentProcessReconciliationAnchorPending {
                    selector,
                    continuation: *continuation,
                    maximum_record_bytes,
                },
            ))
        }
    }
}

fn selection_failure(
    continuation: ResidentProcessReconciliationContinuation,
    reason: ResidentProcessHostSelectionError,
) -> ResidentProcessReconciliationFailure {
    ResidentProcessReconciliationFailure::Selection {
        continuation: Box::new(continuation),
        reason,
    }
}

fn operation_observation_matches(
    selector: &ResidentDeploymentAuthoritySelector,
    observation: &OperationObservationV1,
) -> bool {
    observation.deployment_runtime_authority_ref() == &selector.runtime_authority_ref
        && observation.deployment_runtime_authority_sha256().as_str()
            == selector.runtime_authority_sha256.as_str()
        && observation.issuer_principal_id() == &selector.principal_id
}
