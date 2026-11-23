//! Shared custody phases after semantic recovery signs `Dispatched`.

use super::*;
use crate::resident::{
    DatabaseCommittedOperationObservationAnchorPending, OperationObservationCustodyAnchorFailure,
    OperationObservationCustodyCommitFailure, PreparedOperationObservationCheckpoint,
    PreparedOperationObservationCustody,
};
use pale_ports::{AccumulatorPreparationLimits, AntiRollbackAnchorPort, OperationCustodyPort};
use std::sync::Arc;

#[cfg(test)]
#[path = "../../../tests/internal/resident/operation_dispatch/prepared_recovery.rs"]
mod test_support;

/// Signed `Dispatched` successor reconstructed from one retained preparation.
///
/// Semantic owners construct this value only after rebuilding their exact
/// native action or provider request. It has no handler or carrier method.
#[must_use = "retain the dispatch through checkpoint custody or discard it"]
pub(in crate::resident) struct PreparedRecoveredOperationDispatch {
    operation: IssuedLocalOperation,
    prepared_ref: ResidentRecordRefV1,
    prepared: OperationObservationV1,
    selection_ref: ResidentRecordRefV1,
    request_expires_at: UnixTimeV1,
    observation_ref: ResidentRecordRefV1,
    observation: OperationObservationV1,
    prior_checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
}

impl PreparedRecoveredOperationDispatch {
    pub(super) fn new(
        operation: &IssuedLocalOperation,
        prepared: (&ResidentRecordRefV1, &OperationObservationV1),
        selection_ref: ResidentRecordRefV1,
        request_expires_at: UnixTimeV1,
        observation_ref: ResidentRecordRefV1,
        observation: OperationObservationV1,
        prior_checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    ) -> Self {
        Self {
            operation: operation.clone_for_recovery_continuation(),
            prepared_ref: prepared.0.clone(),
            prepared: prepared.1.clone(),
            selection_ref,
            request_expires_at,
            observation_ref,
            observation,
            prior_checkpoint: (prior_checkpoint.0.clone(), prior_checkpoint.1.clone()),
        }
    }

    pub(in crate::resident) const fn operation(&self) -> &IssuedLocalOperation {
        &self.operation
    }

    pub(in crate::resident) fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation_ref, &self.observation)
    }

    /// Borrow the exact retained preparation that this dispatch succeeds.
    pub(in crate::resident) fn prepared_observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.prepared_ref, &self.prepared)
    }

    pub(in crate::resident) fn checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.prior_checkpoint.0, &self.prior_checkpoint.1)
    }

    pub(in crate::resident) const fn selection_ref(&self) -> &ResidentRecordRefV1 {
        &self.selection_ref
    }

    pub(in crate::resident) const fn request_expires_at(&self) -> UnixTimeV1 {
        self.request_expires_at
    }

    pub(in crate::resident) fn prepare_append(
        &self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
    ) -> Result<PreparedOperationObservationAppend<'_>, OperationObservationCustodyProjectionError>
    {
        let (handle_ref, handle) = self.operation.handle();
        prepare_operation_observation_append(
            OperationObservationTransition::new(
                handle_ref,
                handle,
                &self.prepared_ref,
                &self.prepared,
                &self.observation_ref,
                &self.observation,
            ),
            resolve_store,
        )
    }

    /// Compute the exact successor roots without signing or committing them.
    pub(in crate::resident) fn prepare_checkpoint<'dispatch>(
        self: &'dispatch Arc<Self>,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        limits: AccumulatorPreparationLimits,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<PreparedRecoveredDispatchCheckpoint<'dispatch>, OperationAppendFailure> {
        let prepared = self.prepare_append(resolve_store)?.prepare_checkpoint(
            self.operation.checkpoint_selection(),
            &self.prior_checkpoint.0,
            &self.prior_checkpoint.1,
            limits,
            custody,
        )?;
        Ok(PreparedRecoveredDispatchCheckpoint {
            dispatch: self,
            prepared,
        })
    }
}

/// Exact prepared roots for one recovered dispatch checkpoint.
#[must_use = "issue this exact checkpoint or discard the recovered dispatch"]
pub(in crate::resident) struct PreparedRecoveredDispatchCheckpoint<'a> {
    dispatch: &'a Arc<PreparedRecoveredOperationDispatch>,
    prepared: PreparedOperationObservationCheckpoint<'a>,
}

impl<'a> PreparedRecoveredDispatchCheckpoint<'a> {
    pub(in crate::resident) fn dispatch(&self) -> &PreparedRecoveredOperationDispatch {
        self.dispatch
    }

    /// Sign once, then attach the same issued value without another signer.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn issue<Sign>(
        self,
        identity: OperationCheckpointInput,
        now: UnixTimeV1,
        schema: &SchemaDigest,
        authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        sign: Sign,
    ) -> Result<IssuedRecoveredDispatchCheckpoint, OperationAppendFailure>
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let preimage = observation_checkpoint_preimage(
            &self.prepared,
            self.dispatch.checkpoint(),
            &identity,
            now,
            schema,
            authority,
        )?;
        let checkpoint = self
            .prepared
            .issue_checkpoint_value(preimage, schema, authority, sign)?;
        Ok(IssuedRecoveredDispatchCheckpoint {
            dispatch: Arc::clone(self.dispatch),
            identity,
            checkpoint,
        })
    }
}

/// Checkpoint signed once for a recovered dispatch, before physical attachment.
#[must_use = "attach this exact checkpoint without signing another value"]
pub(in crate::resident) struct IssuedRecoveredDispatchCheckpoint {
    dispatch: Arc<PreparedRecoveredOperationDispatch>,
    identity: OperationCheckpointInput,
    checkpoint: ControlStateCheckpointV1,
}

impl IssuedRecoveredDispatchCheckpoint {
    pub(in crate::resident) fn dispatch(&self) -> &PreparedRecoveredOperationDispatch {
        &self.dispatch
    }

    pub(in crate::resident) fn checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.identity.reference, &self.checkpoint)
    }

    /// Rebuild only the deterministic roots, attach the issued checkpoint and commit once.
    pub(in crate::resident) fn attach_and_commit(
        self,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        limits: AccumulatorPreparationLimits,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<CompletedRecoveredDispatchCustody, RecoveredDispatchCustodyFailure> {
        let prepared = match self
            .dispatch
            .prepare_checkpoint(resolve_store, limits, custody)
        {
            Ok(prepared) => prepared,
            Err(failure) => {
                return Err(RecoveredDispatchCustodyFailure::Issued {
                    continuation: Box::new(self),
                    failure: Box::new(failure),
                });
            }
        };
        let physical = match prepared
            .prepared
            .attach_issued_checkpoint(&self.identity.reference, &self.checkpoint)
        {
            Ok(physical) => physical,
            Err(failure) => {
                return Err(RecoveredDispatchCustodyFailure::Issued {
                    continuation: Box::new(self),
                    failure: Box::new(OperationAppendFailure::Projection(failure)),
                });
            }
        };
        commit_recovered_dispatch(
            self.dispatch,
            self.identity,
            self.checkpoint,
            physical,
            custody,
            anchor,
        )
    }
}

/// One recovered dispatch whose database outcome is still ambiguous.
#[must_use = "reconcile only this retained exact candidate"]
pub(in crate::resident) struct RecoveredDispatchCommitFailure {
    dispatch: Arc<PreparedRecoveredOperationDispatch>,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    failure: OperationObservationCustodyCommitFailure,
}

impl RecoveredDispatchCommitFailure {
    pub(in crate::resident) fn dispatch(&self) -> &PreparedRecoveredOperationDispatch {
        &self.dispatch
    }

    pub(in crate::resident) fn reconcile_exact(
        self,
        custody: &mut dyn OperationCustodyPort,
    ) -> Result<RecoveredDispatchAnchorPending, Box<Self>> {
        match self.failure.reconcile_exact(custody) {
            Ok(pending) => Ok(RecoveredDispatchAnchorPending {
                dispatch: self.dispatch,
                checkpoint: self.checkpoint,
                custody: RecoveredDispatchAnchorCustody::Pending(Box::new(pending)),
            }),
            Err(failure) => Err(Box::new(Self { failure, ..self })),
        }
    }
}

/// Exact database custody awaiting only the independent anchor transition.
#[must_use = "continue only this retained anchor transition"]
pub(in crate::resident) struct RecoveredDispatchAnchorPending {
    dispatch: Arc<PreparedRecoveredOperationDispatch>,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    custody: RecoveredDispatchAnchorCustody,
}

enum RecoveredDispatchAnchorCustody {
    Pending(Box<DatabaseCommittedOperationObservationAnchorPending>),
    Failed(OperationObservationCustodyAnchorFailure),
}

impl RecoveredDispatchAnchorPending {
    pub(in crate::resident) fn dispatch(&self) -> &PreparedRecoveredOperationDispatch {
        &self.dispatch
    }

    pub(in crate::resident) fn continue_anchor(
        self,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<CompletedRecoveredDispatchCustody, Box<Self>> {
        let Self {
            dispatch,
            checkpoint,
            custody,
        } = self;
        let completed = match custody {
            RecoveredDispatchAnchorCustody::Pending(pending) => pending.advance_anchor(anchor),
            RecoveredDispatchAnchorCustody::Failed(failure) => match failure.try_into_retryable() {
                Ok(pending) => pending.advance_anchor(anchor),
                Err(failure) => failure.continue_after_failed_observation(anchor),
            },
        };
        match completed {
            Ok(custody) => Ok(CompletedRecoveredDispatchCustody {
                observation: (
                    dispatch.observation().0.clone(),
                    dispatch.observation().1.clone(),
                ),
                checkpoint,
                custody,
            }),
            Err(failure) => Err(Box::new(Self {
                dispatch,
                checkpoint,
                custody: RecoveredDispatchAnchorCustody::Failed(failure),
            })),
        }
    }
}

/// Durable `Dispatched` custody. No handler or provider was released.
pub(in crate::resident) struct CompletedRecoveredDispatchCustody {
    observation: (ResidentRecordRefV1, OperationObservationV1),
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
    custody: crate::resident::CompletedOperationObservationCustody,
}

impl CompletedRecoveredDispatchCustody {
    pub(in crate::resident) fn observation(
        &self,
    ) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.observation.0, &self.observation.1)
    }

    pub(in crate::resident) fn checkpoint(
        &self,
    ) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.checkpoint.0, &self.checkpoint.1)
    }

    pub(in crate::resident) const fn custody(
        &self,
    ) -> &crate::resident::CompletedOperationObservationCustody {
        &self.custody
    }
}

/// Exact custody phase that remains after a recovered dispatch stopped.
pub(in crate::resident) enum RecoveredDispatchCustodyFailure {
    /// The checkpoint was signed once. This continuation accepts no signer.
    Issued {
        continuation: Box<IssuedRecoveredDispatchCheckpoint>,
        failure: Box<OperationAppendFailure>,
    },
    /// Commit ambiguity retains the exact physical candidate.
    Commit(Box<RecoveredDispatchCommitFailure>),
    /// Database custody is exact and only the anchor remains.
    Anchor(Box<RecoveredDispatchAnchorPending>),
}

fn commit_recovered_dispatch(
    dispatch: Arc<PreparedRecoveredOperationDispatch>,
    identity: OperationCheckpointInput,
    checkpoint: ControlStateCheckpointV1,
    physical: PreparedOperationObservationCustody,
    custody: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> Result<CompletedRecoveredDispatchCustody, RecoveredDispatchCustodyFailure> {
    let checkpoint = (identity.reference, checkpoint);
    let pending = match physical.commit(custody, anchor) {
        Ok(pending) => pending,
        Err(failure) => {
            return Err(RecoveredDispatchCustodyFailure::Commit(Box::new(
                RecoveredDispatchCommitFailure {
                    dispatch,
                    checkpoint,
                    failure,
                },
            )));
        }
    };
    match pending.advance_anchor(anchor) {
        Ok(custody) => Ok(CompletedRecoveredDispatchCustody {
            observation: (
                dispatch.observation().0.clone(),
                dispatch.observation().1.clone(),
            ),
            checkpoint,
            custody,
        }),
        Err(failure) => Err(RecoveredDispatchCustodyFailure::Anchor(Box::new(
            RecoveredDispatchAnchorPending {
                dispatch,
                checkpoint,
                custody: RecoveredDispatchAnchorCustody::Failed(failure),
            },
        ))),
    }
}
