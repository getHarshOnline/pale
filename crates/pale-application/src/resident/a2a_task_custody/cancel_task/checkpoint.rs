//! Checkpoint, database, and independent-anchor phases for one `CancelTask` request.

use super::service::{
    A2aCancelTaskAuthorizationContext, A2aCancelTaskError, A2aCancelTaskNotStarted,
    A2aCancelTaskOutcomeUnknown, A2aCancelTaskRelease, A2aCancelTaskResponseContext,
    A2aCancelTaskStartFailure, A2aCancelTaskUncertainty, InFlightA2aCancelTask,
    PreparedA2aCancelTask,
};
use crate::resident::{
    operation_dispatch::observation_checkpoint_preimage, prepare_operation_observation_append,
    validate_recovered_operation_cancellation, CompletedOperationObservationCustody,
    DatabaseCommittedOperationObservationAnchorPending, OperationCheckpointInput,
    OperationCustodyDatabaseEvidence, OperationObservationAuthorityStoreResolver,
    OperationObservationTransition, PreparedOperationObservationCustody,
};
use pale_core::records::*;
use pale_ports::{
    A2aExchangePort, AccumulatorPreparationLimits, AntiRollbackAnchorPort, OperationCustodyPort,
    WallClockSourcePort,
};

/// Closed cancellation-request custody awaiting one database commit.
#[must_use = "commit and anchor this cancellation request before carrier release"]
pub struct PreparedA2aCancelTaskCustody<'a> {
    physical: PreparedOperationObservationCustody,
    release: A2aCancelTaskRelease<'a>,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
}

/// Database-proven cancellation request awaiting independent anchoring.
#[must_use = "advance the independent anchor before carrier release"]
pub struct DatabaseCommittedA2aCancelTaskAnchorPending<'a> {
    physical: DatabaseCommittedOperationObservationAnchorPending,
    release: A2aCancelTaskRelease<'a>,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
}

/// Newly committed and anchored cancellation request, still unopened by the carrier.
#[must_use = "recheck current authority and recovered custody before starting the request"]
pub struct RetainedA2aCancelTask<'a> {
    custody: CompletedOperationObservationCustody,
    release: A2aCancelTaskRelease<'a>,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
}

pub(in crate::resident) struct A2aCancelTaskCustodyFinalizer<'a> {
    release: A2aCancelTaskRelease<'a>,
    checkpoint: (ResidentRecordRefV1, ControlStateCheckpointV1),
}

/// Exact request-custody phase retained after a database or anchor failure.
pub(in crate::resident) enum ContinuableA2aCancelTaskCustodyFailure<'a> {
    /// Database custody is unresolved and still owns the complete prepared request.
    Commit {
        finalizer: Box<A2aCancelTaskCustodyFinalizer<'a>>,
        failure: crate::resident::OperationObservationCustodyCommitFailure,
    },
    /// Database custody exists, but independent anchoring remains unresolved.
    Anchor {
        finalizer: Box<A2aCancelTaskCustodyFinalizer<'a>>,
        failure: crate::resident::OperationObservationCustodyAnchorFailure,
    },
    /// Exact physical custody exists, but this call did not make the database commit.
    NotNew {
        _finalizer: Box<A2aCancelTaskCustodyFinalizer<'a>>,
        custody: Box<CompletedOperationObservationCustody>,
    },
}

impl<'a> A2aCancelTaskCustodyFinalizer<'a> {
    fn finish(self, custody: CompletedOperationObservationCustody) -> RetainedA2aCancelTask<'a> {
        RetainedA2aCancelTask {
            custody,
            release: self.release,
            checkpoint: self.checkpoint,
        }
    }
}

impl<'a> ContinuableA2aCancelTaskCustodyFailure<'a> {
    /// Continue only the unresolved physical custody phase.
    pub(in crate::resident) fn continue_custody(
        self,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<RetainedA2aCancelTask<'a>, Self> {
        let (finalizer, pending) = match self {
            Self::Commit { finalizer, failure } => {
                let pending = match failure.reconcile_exact(custody) {
                    Ok(pending) => pending,
                    Err(failure) => match failure.try_into_retryable() {
                        Ok(prepared) => match prepared.commit(custody, anchor) {
                            Ok(pending) => pending,
                            Err(failure) => return Err(Self::Commit { finalizer, failure }),
                        },
                        Err(failure) => return Err(Self::Commit { finalizer, failure }),
                    },
                };
                (finalizer, pending)
            }
            Self::Anchor { finalizer, failure } => match failure.try_into_retryable() {
                Ok(pending) => (finalizer, pending),
                Err(failure) => match failure.continue_after_failed_observation(anchor) {
                    Ok(completed)
                        if completed.database_evidence()
                            == OperationCustodyDatabaseEvidence::New =>
                    {
                        return Ok(finalizer.finish(completed));
                    }
                    Ok(completed) => {
                        return Err(Self::NotNew {
                            _finalizer: finalizer,
                            custody: Box::new(completed),
                        });
                    }
                    Err(failure) => return Err(Self::Anchor { finalizer, failure }),
                },
            },
            Self::NotNew { .. } => return Err(self),
        };
        let database_evidence = pending.database_evidence();
        match pending.advance_anchor(anchor) {
            Ok(completed) if database_evidence == OperationCustodyDatabaseEvidence::New => {
                Ok(finalizer.finish(completed))
            }
            Ok(completed) => Err(Self::NotNew {
                _finalizer: finalizer,
                custody: Box::new(completed),
            }),
            Err(failure) => Err(Self::Anchor { finalizer, failure }),
        }
    }

    /// Recover physical replay custody without creating dispatch authority.
    pub(in crate::resident) fn into_not_new_custody(
        self,
    ) -> Result<CompletedOperationObservationCustody, Self> {
        match self {
            Self::NotNew { custody, .. } => Ok(*custody),
            other => Err(other),
        }
    }
}

impl<'a> PreparedA2aCancelTask<'a> {
    /// Prepare roots, issue the successor checkpoint, and close its custody candidate.
    ///
    /// These adjacent phases stay in one consuming call because the prepared
    /// append borrows the cancellation observation owned by `self`. No database
    /// write or independent-anchor transition occurs here.
    ///
    /// # Errors
    ///
    /// Returns an error when the observation append, checkpoint roots, signing
    /// authority, or canonical checkpoint cannot be admitted.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_custody<Sign>(
        self,
        identity: OperationCheckpointInput,
        issued_at: UnixTimeV1,
        schema: &SchemaDigest,
        authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        limits: AccumulatorPreparationLimits,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        custody: &mut dyn OperationCustodyPort,
        sign: Sign,
    ) -> Result<PreparedA2aCancelTaskCustody<'a>, A2aCancelTaskError>
    where
        Sign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        if issued_at < *self.release.cancellation.1.observed_at() {
            return Err(A2aCancelTaskError::Time);
        }
        let prior = self
            .release
            .history
            .local_observation()
            .current_checkpoint();
        let local_head = self.release.history.local_observation().selected_head();
        let (handle_ref, handle) = self.release.operation.handle();
        let append = prepare_operation_observation_append(
            OperationObservationTransition::new(
                handle_ref,
                handle,
                local_head.0,
                local_head.1,
                &self.release.cancellation.0,
                &self.release.cancellation.1,
            ),
            resolve_store,
        )?
        .with_a2a_evidence_items(vec![self.release.request_evidence.clone()]);
        let prepared = append.prepare_checkpoint(
            self.release.operation.checkpoint_selection(),
            prior.0,
            prior.1,
            limits,
            custody,
        )?;
        let preimage = observation_checkpoint_preimage(
            &prepared, prior, &identity, issued_at, schema, authority,
        )?;
        let (physical, checkpoint) =
            prepared.issue_checkpoint(&identity.reference, preimage, schema, authority, sign)?;
        Ok(PreparedA2aCancelTaskCustody {
            physical,
            release: self.release,
            checkpoint: (identity.reference, checkpoint),
        })
    }
}

impl<'a> PreparedA2aCancelTaskCustody<'a> {
    /// Attempt one database commit without retrying an uncertain outcome.
    ///
    /// # Errors
    ///
    /// Returns an error when the database commit fails or its outcome requires
    /// recovery before another write is safe.
    pub fn commit(
        self,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<DatabaseCommittedA2aCancelTaskAnchorPending<'a>, A2aCancelTaskError> {
        let physical = self.physical.commit(custody, anchor)?;
        Ok(DatabaseCommittedA2aCancelTaskAnchorPending {
            physical,
            release: self.release,
            checkpoint: self.checkpoint,
        })
    }

    /// Complete physical custody while preserving every continuable failure phase.
    pub(in crate::resident) fn retain_continuable(
        self,
        custody: &mut dyn OperationCustodyPort,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<RetainedA2aCancelTask<'a>, ContinuableA2aCancelTaskCustodyFailure<'a>> {
        let Self {
            physical,
            release,
            checkpoint,
        } = self;
        let finalizer = Box::new(A2aCancelTaskCustodyFinalizer {
            release,
            checkpoint,
        });
        let pending = match physical.commit(custody, anchor) {
            Ok(pending) => pending,
            Err(failure) => {
                return Err(ContinuableA2aCancelTaskCustodyFailure::Commit { finalizer, failure });
            }
        };
        let database_evidence = pending.database_evidence();
        match pending.advance_anchor(anchor) {
            Ok(completed) if database_evidence == OperationCustodyDatabaseEvidence::New => {
                Ok(finalizer.finish(completed))
            }
            Ok(completed) => Err(ContinuableA2aCancelTaskCustodyFailure::NotNew {
                _finalizer: finalizer,
                custody: Box::new(completed),
            }),
            Err(failure) => {
                Err(ContinuableA2aCancelTaskCustodyFailure::Anchor { finalizer, failure })
            }
        }
    }
}

impl DatabaseCommittedA2aCancelTaskAnchorPending<'_> {
    /// Report how exact database custody was established.
    #[must_use]
    pub const fn database_evidence(&self) -> OperationCustodyDatabaseEvidence {
        self.physical.database_evidence()
    }
}

impl<'a> DatabaseCommittedA2aCancelTaskAnchorPending<'a> {
    /// Establish the independent anchor, but release only a commit made by this call.
    ///
    /// # Errors
    ///
    /// Returns an error when anchoring fails or the database evidence describes
    /// an exact replay instead of a new commit made by this call.
    pub fn advance_anchor(
        self,
        anchor: &mut dyn AntiRollbackAnchorPort,
    ) -> Result<RetainedA2aCancelTask<'a>, A2aCancelTaskError> {
        let database_evidence = self.physical.database_evidence();
        let custody = self.physical.advance_anchor(anchor)?;
        if database_evidence != OperationCustodyDatabaseEvidence::New {
            return Err(A2aCancelTaskError::NotNewDatabaseCommit);
        }
        Ok(RetainedA2aCancelTask {
            custody,
            release: self.release,
            checkpoint: self.checkpoint,
        })
    }
}

impl RetainedA2aCancelTask<'_> {
    /// Borrow the exact committed cancellation observation.
    #[must_use]
    pub fn cancellation_observation(&self) -> (&ResidentRecordRefV1, &OperationObservationV1) {
        (&self.release.cancellation.0, &self.release.cancellation.1)
    }

    /// Borrow the checkpoint that atomically retained the cancellation request.
    #[must_use]
    pub fn checkpoint(&self) -> (&ResidentRecordRefV1, &ControlStateCheckpointV1) {
        (&self.checkpoint.0, &self.checkpoint.1)
    }
}

impl<'a> RetainedA2aCancelTask<'a> {
    /// Recheck current authorization against recovered post-commit history, then start once.
    ///
    /// A carrier rejection consumes the prepared request. A caller may recover
    /// the durable local request and decide a fresh next step, but cannot replay
    /// this phase after any carrier acceptance or unlike exchange coordinate.
    ///
    /// # Errors
    ///
    /// Returns the unopened retained phase when preflight checks fail. After the
    /// carrier is called, it returns either proved non-acceptance or an uncertain
    /// accepted outcome; neither result permits replay of this phase.
    fn start<Authorize>(
        self,
        current_history: &RecoveredA2aTaskHistory,
        authorized_at: UnixTimeV1,
        carrier: &dyn A2aExchangePort,
        authorize: Authorize,
    ) -> Result<InFlightA2aCancelTask, A2aCancelTaskStartFailure<'a>>
    where
        Authorize:
            FnOnce(A2aCancelTaskAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
    {
        let local_head = current_history.local_observation().selected_head();
        let remote_head = current_history.observation();
        if !self
            .release
            .basis
            .matches_without_local_head(self.release.operation, current_history)
            || local_head != (&self.release.cancellation.0, &self.release.cancellation.1)
            || !matches!(
                local_head.1.state(),
                OperationObservationStateV1::CancellationRequested { .. }
            )
        {
            return Err(A2aCancelTaskStartFailure::Preflight {
                retained: Box::new(self),
                reason: A2aCancelTaskError::Association,
            });
        }
        if authorized_at < *local_head.1.observed_at()
            || authorized_at < *remote_head.1.observed_at()
            || authorized_at > self.release.basis.retain_until()
            || !self.release.basis.admits_exchange_deadline(
                authorized_at,
                self.release.prepared.coordinate().limits().total_timeout,
            )
        {
            return Err(A2aCancelTaskStartFailure::Preflight {
                retained: Box::new(self),
                reason: A2aCancelTaskError::Time,
            });
        }
        if let Err(error) = validate_recovered_operation_cancellation(
            self.release.request,
            self.release.original_request,
            self.release.operation,
            current_history.local_observation(),
            authorized_at,
        ) {
            return Err(A2aCancelTaskStartFailure::Preflight {
                retained: Box::new(self),
                reason: error.into(),
            });
        }
        if let Err(error) = authorize(A2aCancelTaskAuthorizationContext {
            request: self.release.request,
            original_request: self.release.original_request,
            operation: self.release.operation.handle(),
            dispatch: current_history.dispatch(),
            local_head,
            route: current_history.route(),
            binding: current_history.binding(),
            remote_head,
            authorized_at,
        }) {
            return Err(A2aCancelTaskStartFailure::Preflight {
                retained: Box::new(self),
                reason: error.into(),
            });
        }
        let RetainedA2aCancelTask {
            custody,
            release,
            checkpoint: _,
        } = self;
        let A2aCancelTaskRelease {
            request: _,
            original_request: _,
            history: _,
            operation: _,
            basis,
            prepared,
            request_evidence,
            response_record_ref,
            additional_response_record_ref,
            expected_task,
            expected_context,
            cancellation,
            maximum_evidence_bytes,
        } = release;
        let expected = prepared.coordinate().clone();
        let context = A2aCancelTaskResponseContext {
            basis,
            request_evidence,
            response_record_ref,
            additional_response_record_ref,
            expected_task,
            expected_context,
            request_id: expected.context().request_id().clone(),
            cancellation,
            maximum_evidence_bytes,
            start_authorized_at: authorized_at,
        };
        let mut exchange = match carrier.start(prepared) {
            Ok(exchange) => exchange,
            Err(reason) => {
                return Err(A2aCancelTaskStartFailure::NotStarted(Box::new(
                    A2aCancelTaskNotStarted {
                        custody,
                        context,
                        reason,
                    },
                )));
            }
        };
        if exchange.coordinate() != &expected {
            exchange.cancel();
            return Err(A2aCancelTaskStartFailure::OutcomeUnknown(Box::new(
                A2aCancelTaskOutcomeUnknown {
                    custody,
                    context,
                    responses: Vec::new(),
                    reason: A2aCancelTaskUncertainty::UnlikeExchangeCoordinate,
                },
            )));
        }
        Ok(InFlightA2aCancelTask {
            exchange,
            custody,
            context,
            expected,
        })
    }
}

/// Recheck current retained authority and start one `CancelTask` carrier.
///
/// The trusted clock is sampled inside this call immediately before the
/// authorization callback and the sole carrier-start path. A failed clock or
/// denied authorization returns the unopened retained phase. Once the carrier
/// is called, no returned phase can replay these prepared bytes.
///
/// # Errors
///
/// Returns a preflight failure with the unopened retained request, a proved
/// queue rejection, or a non-replayable accepted-but-uncertain outcome.
pub fn start_a2a_cancel_task<'a, Authorize>(
    retained: RetainedA2aCancelTask<'a>,
    current_history: &RecoveredA2aTaskHistory,
    carrier: &dyn A2aExchangePort,
    clock: &dyn WallClockSourcePort,
    authorize: Authorize,
) -> Result<InFlightA2aCancelTask, A2aCancelTaskStartFailure<'a>>
where
    Authorize: FnOnce(A2aCancelTaskAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
{
    let authorized_at = match clock.observe_unix_time() {
        Ok(value) => value,
        Err(reason) => {
            return Err(A2aCancelTaskStartFailure::Preflight {
                retained: Box::new(retained),
                reason: A2aCancelTaskError::Clock(reason),
            });
        }
    };
    retained.start(current_history, authorized_at, carrier, authorize)
}

use super::super::RecoveredA2aTaskHistory;
