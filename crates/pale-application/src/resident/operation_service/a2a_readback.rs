//! Private composition of retained A2A task read-back.
//!
//! This module does not select a route, allocate record identities, grant
//! disclosure, or interpret remote task state as a Pale result. It only joins
//! the existing recovery, one-shot `GetTask`, and durable custody owners to the
//! resident operation service.

use pale_core::records::{
    ArtifactDigest, CanonicalSignatureV1, ControlCheckpointSigningAuthorityAdmissionContextV1,
    ControlStateCheckpointDigest, ControlStateCheckpointPreimageV1, ControlStateCheckpointV1,
    InvalidResidentRecordV1, OperationObservationDigest, OperationObservationPreimageV1,
    OperationObservationStateV1, OperationObservationWireV1, ResidentRecordRefV1, SchemaDigest,
    SigningAuthorityBindingV1, UnixTimeV1,
};
use pale_ports::{
    A2aExchangePort, A2aGetTaskEvidencePort, AccumulatorMembershipLimits,
    AccumulatorPreparationLimits, AuthorityStoreCoordinate, WallClockSourcePort,
};

use super::ResidentOperationService;
use crate::resident::a2a_get_task_owner::{
    a2a_get_task_key, ResidentA2aGetTaskAggregateAcceptanceFailure,
    ResidentA2aGetTaskAggregateAccepted, ResidentA2aGetTaskAggregateTransitionRejection,
    ResidentA2aGetTaskAwaitingCustody, ResidentA2aGetTaskClosedDisposition,
    ResidentA2aGetTaskDurableStart, ResidentA2aGetTaskDurableStartFailure,
    ResidentA2aGetTaskFinishFailure, ResidentA2aGetTaskPoll,
    ResidentA2aGetTaskReadStopFinishFailure, ResidentA2aGetTaskReadStoppedDisposition,
    ResidentA2aGetTaskReservation, ResidentA2aGetTaskReservationReleaseFailure,
};
use crate::resident::a2a_task_custody::{
    prepare_a2a_get_task_custody, recover_a2a_get_task_custody, A2aGetTaskCustodyFailure,
    A2aGetTaskCustodyRecoveryError, A2aGetTaskLocalDispositionKind, A2aGetTaskPreparationInput,
    PreparedA2aGetTaskCustody, RecoveredA2aGetTaskCustody, RetainedA2aGetTask,
};
use crate::resident::task_reservation::{
    ResidentA2aTaskReservationOfferFailure, ResidentA2aTaskWorkKey,
    ResidentTaskReservationSettlement, ResidentTaskReservationSettlementTransitionFailure,
    ResidentTaskReservationTransitionFailure,
};
use crate::resident::{
    commit_a2a_get_task_readback as commit_readback,
    commit_a2a_get_task_readback_continuable as commit_readback_continuable,
    recover_a2a_task_history as recover_history, start_a2a_get_task_readback as start_readback,
    A2aGetTaskCommitFailure, A2aGetTaskCommitInput, A2aGetTaskReadbackAuthorizationContext,
    A2aGetTaskReadbackError, A2aGetTaskReadbackInput, A2aTaskRecoveryAdmission,
    A2aTaskRecoveryError, A2aTaskRecoveryInput, CompletedA2aGetTaskReadback,
    CompletedA2aGetTaskReadbackCustody, ContinuableA2aGetTaskCommitFailure,
    InFlightA2aGetTaskReadback, IssuedLocalOperation, OperationCheckpointInput,
    OperationObservationCustodyAnchorFailure, OperationObservationCustodyCommitFailure,
    OperationObservationRecoveryAdmission, RecoveredA2aTaskHistory,
};

/// Fresh private identities and bounds for one local terminal disposition.
pub(in crate::resident) struct ResidentA2aGetTaskLocalDispositionInput<'a> {
    pub disposition_ref: ResidentRecordRefV1,
    pub recorded_at: UnixTimeV1,
    pub checkpoint: OperationCheckpointInput,
    pub checkpoint_issued_at: UnixTimeV1,
    pub checkpoint_schema: &'a SchemaDigest,
    pub checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    pub accumulator_limits: AccumulatorPreparationLimits,
}

pub(in crate::resident) enum ResidentA2aGetTaskLocalTerminal {
    Closed(Box<ResidentA2aGetTaskClosedDisposition>),
    ReadStopped(Box<ResidentA2aGetTaskReadStoppedDisposition>),
}

/// Durable local disposition that still owns aggregate capacity.
#[must_use = "release aggregate capacity only after this custody exists"]
pub(in crate::resident) struct ResidentRetainedA2aGetTaskLocalDisposition {
    terminal: ResidentA2aGetTaskLocalTerminal,
    custody: Box<crate::resident::CompletedOperationObservationCustody>,
}

/// Exact database or anchor phase retained after local-disposition custody stopped.
#[must_use = "continue exact local-disposition custody without carrier access"]
pub(in crate::resident) enum ResidentA2aGetTaskLocalDispositionCustodyFailure {
    Association {
        terminal: ResidentA2aGetTaskLocalTerminal,
        reason: Box<A2aGetTaskReadbackError>,
    },
    Commit {
        terminal: ResidentA2aGetTaskLocalTerminal,
        failure: Box<OperationObservationCustodyCommitFailure>,
    },
    Anchor {
        terminal: ResidentA2aGetTaskLocalTerminal,
        failure: Box<OperationObservationCustodyAnchorFailure>,
    },
}

/// Released terminal outcome paired with its exact durable local custody.
pub(in crate::resident) enum ResidentReleasedA2aGetTaskLocalDisposition {
    Closed {
        reason: A2aGetTaskReadbackError,
        custody: Box<crate::resident::CompletedOperationObservationCustody>,
    },
    ReadStopped {
        custody: Box<crate::resident::CompletedOperationObservationCustody>,
    },
}

/// Aggregate release failed after local-disposition custody completed.
#[must_use = "retry only aggregate release; never call the carrier again"]
pub(in crate::resident) enum ResidentA2aGetTaskLocalDispositionReleaseFailure {
    Closed {
        custody: Box<crate::resident::CompletedOperationObservationCustody>,
        failure: ResidentA2aGetTaskFinishFailure,
    },
    ReadStopped {
        custody: Box<crate::resident::CompletedOperationObservationCustody>,
        failure: ResidentA2aGetTaskReadStopFinishFailure,
    },
}

impl ResidentA2aGetTaskLocalTerminal {
    fn key(&self) -> &ResidentA2aTaskWorkKey {
        match self {
            Self::Closed(closed) => &closed.key,
            Self::ReadStopped(stopped) => &stopped.key,
        }
    }

    fn physical_inputs(
        &self,
    ) -> Option<(
        A2aGetTaskLocalDispositionKind,
        &(ResidentRecordRefV1, ControlStateCheckpointV1),
        &crate::resident::CompletedOperationObservationCustody,
        crate::resident::a2a_task_custody::A2aGetTaskLocalDispositionBasis,
    )> {
        match self {
            Self::Closed(closed) => Some((
                local_disposition_kind(&closed.reason),
                closed.request_checkpoint.as_ref(),
                closed.request_custody.as_deref()?,
                closed.disposition_basis.as_deref()?.clone(),
            )),
            Self::ReadStopped(stopped) => Some((
                A2aGetTaskLocalDispositionKind::ReadStopped,
                stopped.request_checkpoint.as_ref(),
                stopped.request_custody.as_deref()?,
                stopped.disposition_basis.as_deref()?.clone(),
            )),
        }
    }

    fn discard_release_inputs(&mut self) {
        match self {
            Self::Closed(closed) => {
                closed.request_custody = None;
                closed.disposition_basis = None;
            }
            Self::ReadStopped(stopped) => {
                stopped.request_custody = None;
                stopped.disposition_basis = None;
            }
        }
    }
}

impl ResidentRetainedA2aGetTaskLocalDisposition {
    pub(in crate::resident) fn key(&self) -> &ResidentA2aTaskWorkKey {
        self.terminal.key()
    }
}

impl ResidentA2aGetTaskLocalDispositionCustodyFailure {
    pub(in crate::resident) fn key(&self) -> &ResidentA2aTaskWorkKey {
        match self {
            Self::Association { terminal, .. }
            | Self::Commit { terminal, .. }
            | Self::Anchor { terminal, .. } => terminal.key(),
        }
    }
}

impl ResidentA2aGetTaskLocalDispositionReleaseFailure {
    pub(in crate::resident) fn key(&self) -> &ResidentA2aTaskWorkKey {
        match self {
            Self::Closed { failure, .. } => &failure.closed.key,
            Self::ReadStopped { failure, .. } => &failure.stopped.key,
        }
    }
}

fn local_disposition_kind(reason: &A2aGetTaskReadbackError) -> A2aGetTaskLocalDispositionKind {
    match reason {
        A2aGetTaskReadbackError::Clock(_) => A2aGetTaskLocalDispositionKind::ClockFailure,
        A2aGetTaskReadbackError::Exchange(_) => A2aGetTaskLocalDispositionKind::ExchangeFailure,
        A2aGetTaskReadbackError::Evidence(_) | A2aGetTaskReadbackError::Custody(_) => {
            A2aGetTaskLocalDispositionKind::EvidenceFailure
        }
        A2aGetTaskReadbackError::Response => A2aGetTaskLocalDispositionKind::ResponseFailure,
        A2aGetTaskReadbackError::Authorization(_)
        | A2aGetTaskReadbackError::Attempt(_)
        | A2aGetTaskReadbackError::Time
        | A2aGetTaskReadbackError::Association => {
            A2aGetTaskLocalDispositionKind::AssociationFailure
        }
    }
}

/// Prepared `GetTask` request that still owns its unopened aggregate offer.
#[must_use = "complete physical custody or release the exact unopened reservation"]
pub(in crate::resident) struct ResidentPreparedA2aGetTask<'a> {
    pub(in crate::resident) reservation: ResidentA2aGetTaskReservation,
    prepared: Box<PreparedA2aGetTaskCustody<'a>>,
}

/// Initial preparation stopped before carrier release or a physical commit.
#[must_use = "preserve an unresolved aggregate release"]
pub(in crate::resident) enum ResidentA2aGetTaskPreparationFailure {
    Reservation(Box<ResidentA2aTaskReservationOfferFailure>),
    Closed(Box<A2aGetTaskReadbackError>),
    Release {
        failure: Box<ResidentA2aGetTaskReservationReleaseFailure>,
        reason: Box<A2aGetTaskReadbackError>,
    },
}

/// Physical request custody stopped while aggregate capacity remains offered.
#[must_use = "continue exact physical custody or classify non-dispatchable replay"]
pub(in crate::resident) enum ResidentA2aGetTaskRequestCustodyFailure<'a> {
    PreparedAssociation {
        prepared: Box<ResidentPreparedA2aGetTask<'a>>,
        reason: ResidentTaskReservationTransitionFailure,
    },
    ContinuingAssociation {
        reservation: ResidentA2aGetTaskReservation,
        failure: Box<A2aGetTaskCustodyFailure<'a>>,
        reason: ResidentTaskReservationTransitionFailure,
    },
    Custody {
        reservation: ResidentA2aGetTaskReservation,
        failure: Box<A2aGetTaskCustodyFailure<'a>>,
    },
}

/// Recovered physical custody that cannot restore same-process carrier authority.
#[must_use = "release only the aggregate offer; never start GetTask"]
pub(in crate::resident) struct ResidentA2aGetTaskNonDispatchableReplayCustody {
    pub(in crate::resident) reservation: ResidentA2aGetTaskReservation,
    pub(in crate::resident) custody: Box<crate::resident::CompletedOperationObservationCustody>,
}

/// Aggregate release failed for non-dispatchable replay custody.
#[must_use = "retry only the exact unopened aggregate release"]
pub(in crate::resident) struct ResidentA2aGetTaskNonDispatchableReplayReleaseFailure {
    failure: ResidentA2aGetTaskReservationReleaseFailure,
    custody: Box<crate::resident::CompletedOperationObservationCustody>,
}

/// Retained request and still-offered aggregate capacity before local acceptance.
#[must_use = "accept aggregate capacity before fresh preflight and carrier start"]
pub(in crate::resident) struct ResidentRetainedUnopenedA2aGetTask<'a> {
    pub(in crate::resident) reservation: ResidentA2aGetTaskReservation,
    pub(in crate::resident) retained: Box<RetainedA2aGetTask<'a>>,
}

/// Durable remote-history custody that still owns shared process capacity.
pub(in crate::resident) struct ResidentRetainedA2aGetTask {
    key: Box<ResidentA2aTaskWorkKey>,
    settlement: Box<ResidentTaskReservationSettlement>,
    custody: Box<CompletedA2aGetTaskReadbackCustody>,
}

/// `GetTask` history retention stopped with its exact capacity and custody phase.
pub(in crate::resident) struct ResidentA2aGetTaskRetentionFailure {
    key: Box<ResidentA2aTaskWorkKey>,
    settlement: Box<ResidentTaskReservationSettlement>,
    request_checkpoint: Box<(ResidentRecordRefV1, ControlStateCheckpointV1)>,
    request_custody: Option<Box<crate::resident::CompletedOperationObservationCustody>>,
    failure: Box<ContinuableA2aGetTaskCommitFailure>,
}

/// Capacity release failed after remote history reached physical custody.
pub(in crate::resident) enum ResidentA2aGetTaskReleaseFailure {
    Association(Box<ResidentRetainedA2aGetTask>),
    Settlement {
        key: Box<ResidentA2aTaskWorkKey>,
        custody: Box<CompletedA2aGetTaskReadbackCustody>,
        release: Box<ResidentTaskReservationSettlementTransitionFailure>,
    },
}

impl ResidentPreparedA2aGetTask<'_> {
    /// Borrow the binding-qualified key retained with the prepared request.
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        &self.reservation.key
    }
}

impl ResidentA2aGetTaskPreparationFailure {
    /// Borrow the key only when unopened-offer release remains unresolved.
    pub(in crate::resident) fn unresolved_key(&self) -> Option<&ResidentA2aTaskWorkKey> {
        match self {
            Self::Release { failure, .. } => Some(failure.key()),
            Self::Reservation(_) | Self::Closed(_) => None,
        }
    }
}

impl ResidentA2aGetTaskRequestCustodyFailure<'_> {
    /// Borrow the binding-qualified key retained by this exact phase.
    pub(in crate::resident) fn key(&self) -> &ResidentA2aTaskWorkKey {
        match self {
            Self::PreparedAssociation { prepared, .. } => prepared.key(),
            Self::ContinuingAssociation { reservation, .. } | Self::Custody { reservation, .. } => {
                &reservation.key
            }
        }
    }
}

impl ResidentA2aGetTaskNonDispatchableReplayCustody {
    /// Borrow the key retained with physical replay custody that cannot dispatch.
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        &self.reservation.key
    }
}

impl ResidentA2aGetTaskNonDispatchableReplayReleaseFailure {
    /// Borrow the key retained with failed aggregate-offer release.
    pub(in crate::resident) fn key(&self) -> &ResidentA2aTaskWorkKey {
        self.failure.key()
    }
}

impl ResidentRetainedUnopenedA2aGetTask<'_> {
    /// Borrow the key of this retained unopened read request.
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        &self.reservation.key
    }
}

impl ResidentRetainedA2aGetTask {
    /// Borrow the exact binding-qualified work key retained with custody.
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        &self.key
    }
}

impl ResidentA2aGetTaskRetentionFailure {
    /// Borrow the exact binding-qualified work key retained by this failure.
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        &self.key
    }
}

impl ResidentA2aGetTaskReleaseFailure {
    /// Borrow the exact binding-qualified work key retained by this failure.
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        match self {
            Self::Association(retained) => &retained.key,
            Self::Settlement { key, .. } => key,
        }
    }
}

impl<'ports, Authority> ResidentOperationService<'ports, Authority> {
    /// Reserve binding-wide capacity, then prepare one durable `GetTask` request.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn prepare_owned_a2a_get_task<'a, Authorize, CheckpointSign>(
        &mut self,
        input: A2aGetTaskPreparationInput<'a>,
        port: &dyn A2aExchangePort,
        clock: &dyn WallClockSourcePort,
        authorize: Authorize,
        checkpoint: OperationCheckpointInput,
        checkpoint_issued_at: UnixTimeV1,
        checkpoint_schema: &SchemaDigest,
        checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        accumulator_limits: AccumulatorPreparationLimits,
        membership_limits: AccumulatorMembershipLimits,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ResidentPreparedA2aGetTask<'a>, ResidentA2aGetTaskPreparationFailure>
    where
        Authorize: FnOnce(
            A2aGetTaskReadbackAuthorizationContext<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let key = a2a_get_task_key(&input.readback);
        let reservation = self
            .tasks
            .a2a_get_task_owner
            .reserve(key, &mut self.tasks.reservations)
            .map_err(|failure| {
                ResidentA2aGetTaskPreparationFailure::Reservation(Box::new(failure))
            })?;
        let prepared =
            prepare_a2a_get_task_custody(input, port, clock, authorize).and_then(|prepared| {
                prepared.prepare_custody(
                    checkpoint,
                    checkpoint_issued_at,
                    checkpoint_schema,
                    checkpoint_authority,
                    accumulator_limits,
                    membership_limits,
                    self.custody,
                    sign_checkpoint,
                )
            });
        match prepared {
            Ok(prepared) => Ok(ResidentPreparedA2aGetTask {
                reservation,
                prepared: Box::new(prepared),
            }),
            Err(reason) => match self
                .tasks
                .a2a_get_task_owner
                .release_unopened(reservation, &mut self.tasks.reservations)
            {
                Ok(()) => Err(ResidentA2aGetTaskPreparationFailure::Closed(Box::new(
                    reason,
                ))),
                Err(failure) => Err(ResidentA2aGetTaskPreparationFailure::Release {
                    failure: Box::new(failure),
                    reason: Box::new(reason),
                }),
            },
        }
    }

    /// Commit and anchor one prepared request without carrier access.
    pub(in crate::resident) fn retain_prepared_a2a_get_task<'a>(
        &mut self,
        prepared: ResidentPreparedA2aGetTask<'a>,
    ) -> Result<ResidentRetainedUnopenedA2aGetTask<'a>, ResidentA2aGetTaskRequestCustodyFailure<'a>>
    {
        if let Err(reason) = self
            .tasks
            .a2a_get_task_owner
            .require_unopened(&prepared.reservation, &self.tasks.reservations)
        {
            return Err(
                ResidentA2aGetTaskRequestCustodyFailure::PreparedAssociation {
                    prepared: Box::new(prepared),
                    reason,
                },
            );
        }
        let reservation = prepared.reservation;
        match prepared
            .prepared
            .commit(self.custody, self.anchor)
            .and_then(|pending| pending.advance_anchor(self.anchor))
        {
            Ok(retained) => Ok(ResidentRetainedUnopenedA2aGetTask {
                reservation,
                retained: Box::new(retained),
            }),
            Err(failure) => Err(ResidentA2aGetTaskRequestCustodyFailure::Custody {
                reservation,
                failure,
            }),
        }
    }

    /// Retry only offered-capacity release after preparation already closed.
    pub(in crate::resident) fn retry_a2a_get_task_preparation_release(
        &mut self,
        pending: ResidentA2aGetTaskPreparationFailure,
    ) -> Result<A2aGetTaskReadbackError, ResidentA2aGetTaskPreparationFailure> {
        let ResidentA2aGetTaskPreparationFailure::Release { failure, reason } = pending else {
            return Err(pending);
        };
        match self
            .tasks
            .a2a_get_task_owner
            .retry_unopened_release(*failure, &mut self.tasks.reservations)
        {
            Ok(()) => Ok(*reason),
            Err(retry) => Err(ResidentA2aGetTaskPreparationFailure::Release {
                failure: Box::new(retry),
                reason,
            }),
        }
    }

    /// Continue only unresolved request database or anchor custody.
    pub(in crate::resident) fn continue_a2a_get_task_request_custody<'a>(
        &mut self,
        pending: ResidentA2aGetTaskRequestCustodyFailure<'a>,
    ) -> Result<ResidentRetainedUnopenedA2aGetTask<'a>, ResidentA2aGetTaskRequestCustodyFailure<'a>>
    {
        let (reservation, failure) = match pending {
            ResidentA2aGetTaskRequestCustodyFailure::PreparedAssociation { prepared, .. } => {
                return self.retain_prepared_a2a_get_task(*prepared);
            }
            ResidentA2aGetTaskRequestCustodyFailure::ContinuingAssociation {
                reservation,
                failure,
                ..
            }
            | ResidentA2aGetTaskRequestCustodyFailure::Custody {
                reservation,
                failure,
            } => (reservation, failure),
        };
        if let Err(reason) = self
            .tasks
            .a2a_get_task_owner
            .require_unopened(&reservation, &self.tasks.reservations)
        {
            return Err(
                ResidentA2aGetTaskRequestCustodyFailure::ContinuingAssociation {
                    reservation,
                    failure,
                    reason,
                },
            );
        }
        match (*failure).continue_exact(self.custody, self.anchor) {
            Ok(retained) => Ok(ResidentRetainedUnopenedA2aGetTask {
                reservation,
                retained: Box::new(retained),
            }),
            Err(failure) => Err(ResidentA2aGetTaskRequestCustodyFailure::Custody {
                reservation,
                failure,
            }),
        }
    }

    /// Extract recovered custody without creating carrier authority.
    pub(in crate::resident) fn classify_non_dispatchable_replay_a2a_get_task<'a>(
        &self,
        pending: ResidentA2aGetTaskRequestCustodyFailure<'a>,
    ) -> Result<
        ResidentA2aGetTaskNonDispatchableReplayCustody,
        ResidentA2aGetTaskRequestCustodyFailure<'a>,
    > {
        let ResidentA2aGetTaskRequestCustodyFailure::Custody {
            reservation,
            failure,
        } = pending
        else {
            return Err(pending);
        };
        match (*failure).into_non_dispatchable_custody() {
            Ok(custody) => Ok(ResidentA2aGetTaskNonDispatchableReplayCustody {
                reservation,
                custody: Box::new(custody),
            }),
            Err(failure) => Err(ResidentA2aGetTaskRequestCustodyFailure::Custody {
                reservation,
                failure,
            }),
        }
    }

    /// Release aggregate capacity for recovered custody that cannot dispatch.
    pub(in crate::resident) fn finish_non_dispatchable_replay_a2a_get_task(
        &mut self,
        closed: ResidentA2aGetTaskNonDispatchableReplayCustody,
    ) -> Result<
        crate::resident::CompletedOperationObservationCustody,
        ResidentA2aGetTaskNonDispatchableReplayReleaseFailure,
    > {
        match self
            .tasks
            .a2a_get_task_owner
            .release_unopened(closed.reservation, &mut self.tasks.reservations)
        {
            Ok(()) => Ok(*closed.custody),
            Err(failure) => Err(ResidentA2aGetTaskNonDispatchableReplayReleaseFailure {
                failure,
                custody: closed.custody,
            }),
        }
    }

    /// Retry only aggregate release for non-dispatchable replay custody.
    pub(in crate::resident) fn retry_non_dispatchable_replay_a2a_get_task_release(
        &mut self,
        pending: ResidentA2aGetTaskNonDispatchableReplayReleaseFailure,
    ) -> Result<
        crate::resident::CompletedOperationObservationCustody,
        ResidentA2aGetTaskNonDispatchableReplayReleaseFailure,
    > {
        match self
            .tasks
            .a2a_get_task_owner
            .retry_unopened_release(pending.failure, &mut self.tasks.reservations)
        {
            Ok(()) => Ok(*pending.custody),
            Err(failure) => Err(ResidentA2aGetTaskNonDispatchableReplayReleaseFailure {
                failure,
                custody: pending.custody,
            }),
        }
    }

    /// Accept aggregate capacity before fresh preflight and carrier start.
    pub(in crate::resident) fn start_retained_a2a_get_task<'a, Authorize>(
        &mut self,
        unopened: ResidentRetainedUnopenedA2aGetTask<'a>,
        exchange: &dyn A2aExchangePort,
        clock: &dyn WallClockSourcePort,
        authorize: Authorize,
    ) -> Result<ResidentA2aGetTaskDurableStart<'a>, ResidentA2aGetTaskDurableStartFailure<'a>>
    where
        Authorize: FnOnce(
            A2aGetTaskReadbackAuthorizationContext<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        if unopened.retained.custody().candidate().store() != &unopened.reservation.key.store {
            return Err(ResidentA2aGetTaskDurableStartFailure::Association {
                reservation: unopened.reservation,
                retained: unopened.retained,
            });
        }
        self.tasks.a2a_get_task_owner.start_retained(
            unopened.reservation,
            *unopened.retained,
            exchange,
            clock,
            &mut self.tasks.reservations,
            authorize,
        )
    }

    /// Retry only aggregate acceptance before any carrier call.
    pub(in crate::resident) fn retry_a2a_get_task_aggregate_acceptance<'a>(
        &mut self,
        failure: ResidentA2aGetTaskAggregateAcceptanceFailure<'a>,
    ) -> Result<
        ResidentA2aGetTaskAggregateAccepted<'a>,
        ResidentA2aGetTaskAggregateAcceptanceFailure<'a>,
    > {
        self.tasks
            .a2a_get_task_owner
            .retry_retained_acceptance(failure, &mut self.tasks.reservations)
    }

    /// Apply fresh preflight to one aggregate-accepted unopened request.
    pub(in crate::resident) fn start_aggregate_accepted_a2a_get_task<'a, Authorize>(
        &mut self,
        accepted: ResidentA2aGetTaskAggregateAccepted<'a>,
        exchange: &dyn A2aExchangePort,
        clock: &dyn WallClockSourcePort,
        authorize: Authorize,
    ) -> ResidentA2aGetTaskDurableStart<'a>
    where
        Authorize: FnOnce(
            A2aGetTaskReadbackAuthorizationContext<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        self.tasks.a2a_get_task_owner.start_accepted(
            accepted,
            exchange,
            clock,
            &mut self.tasks.reservations,
            authorize,
        )
    }

    /// Read the exact durable `GetTask` phase without recreating release authority.
    pub(in crate::resident) fn recover_owned_a2a_get_task_custody(
        &mut self,
        history: &RecoveredA2aTaskHistory,
        operation: &IssuedLocalOperation,
        maximum_record_bytes: usize,
    ) -> Result<RecoveredA2aGetTaskCustody, A2aGetTaskCustodyRecoveryError> {
        recover_a2a_get_task_custody(history, operation, maximum_record_bytes, self.custody)
    }

    /// Advance one exact process-owned read by at most one lower phase.
    pub(in crate::resident) fn poll_owned_a2a_get_task(
        &mut self,
        key: &ResidentA2aTaskWorkKey,
        cx: &mut std::task::Context<'_>,
        clock: &dyn WallClockSourcePort,
    ) -> ResidentA2aGetTaskPoll {
        self.tasks
            .a2a_get_task_owner
            .poll(key, cx, clock, &mut self.tasks.reservations)
    }

    /// Stop one local response read without sending remote `CancelTask`.
    pub(in crate::resident) fn stop_owned_a2a_get_task_response_read(
        &mut self,
        key: &ResidentA2aTaskWorkKey,
    ) -> ResidentA2aGetTaskPoll {
        self.tasks
            .a2a_get_task_owner
            .stop_response_read(key, &mut self.tasks.reservations)
    }

    /// Retry only the completed-read aggregate transition.
    pub(in crate::resident) fn retry_owned_a2a_get_task_aggregate_transition(
        &mut self,
        rejected: ResidentA2aGetTaskAggregateTransitionRejection,
    ) -> ResidentA2aGetTaskPoll {
        self.tasks
            .a2a_get_task_owner
            .retry_aggregate_transition(rejected, &mut self.tasks.reservations)
    }

    /// Retain one carrier-start or response-read failure before releasing capacity.
    pub(in crate::resident) fn retain_closed_a2a_get_task_local_disposition<CheckpointSign>(
        &mut self,
        closed: ResidentA2aGetTaskClosedDisposition,
        input: ResidentA2aGetTaskLocalDispositionInput<'_>,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentRetainedA2aGetTaskLocalDisposition,
        ResidentA2aGetTaskLocalDispositionCustodyFailure,
    >
    where
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.retain_a2a_get_task_local_disposition(
            ResidentA2aGetTaskLocalTerminal::Closed(Box::new(closed)),
            input,
            sign_checkpoint,
        )
    }

    /// Retain one explicit local response-read stop before releasing capacity.
    pub(in crate::resident) fn retain_stopped_a2a_get_task_local_disposition<CheckpointSign>(
        &mut self,
        stopped: ResidentA2aGetTaskReadStoppedDisposition,
        input: ResidentA2aGetTaskLocalDispositionInput<'_>,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentRetainedA2aGetTaskLocalDisposition,
        ResidentA2aGetTaskLocalDispositionCustodyFailure,
    >
    where
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.retain_a2a_get_task_local_disposition(
            ResidentA2aGetTaskLocalTerminal::ReadStopped(Box::new(stopped)),
            input,
            sign_checkpoint,
        )
    }

    fn retain_a2a_get_task_local_disposition<CheckpointSign>(
        &mut self,
        mut terminal: ResidentA2aGetTaskLocalTerminal,
        input: ResidentA2aGetTaskLocalDispositionInput<'_>,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentRetainedA2aGetTaskLocalDisposition,
        ResidentA2aGetTaskLocalDispositionCustodyFailure,
    >
    where
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let Some((kind, request_checkpoint, request_custody, basis)) = terminal.physical_inputs()
        else {
            return Err(
                ResidentA2aGetTaskLocalDispositionCustodyFailure::Association {
                    terminal,
                    reason: Box::new(A2aGetTaskReadbackError::Association),
                },
            );
        };
        let physical = basis
            .prepare(input.disposition_ref, kind, input.recorded_at)
            .and_then(|prepared| {
                prepared.prepare_custody(
                    request_checkpoint,
                    request_custody,
                    input.checkpoint,
                    input.checkpoint_issued_at,
                    input.checkpoint_schema,
                    input.checkpoint_authority,
                    input.accumulator_limits,
                    self.custody,
                    sign_checkpoint,
                )
            });
        let physical = match physical {
            Ok(physical) => physical,
            Err(reason) => {
                return Err(
                    ResidentA2aGetTaskLocalDispositionCustodyFailure::Association {
                        terminal,
                        reason: Box::new(reason),
                    },
                );
            }
        };
        terminal.discard_release_inputs();
        let pending = match physical.commit(self.custody, self.anchor) {
            Ok(pending) => pending,
            Err(failure) => {
                return Err(ResidentA2aGetTaskLocalDispositionCustodyFailure::Commit {
                    terminal,
                    failure: Box::new(failure),
                });
            }
        };
        let custody = match pending.advance_anchor(self.anchor) {
            Ok(custody) => custody,
            Err(failure) => {
                return Err(ResidentA2aGetTaskLocalDispositionCustodyFailure::Anchor {
                    terminal,
                    failure: Box::new(failure),
                });
            }
        };
        Ok(ResidentRetainedA2aGetTaskLocalDisposition {
            terminal,
            custody: Box::new(custody),
        })
    }

    /// Continue only the retained database or independent-anchor phase.
    pub(in crate::resident) fn continue_a2a_get_task_local_disposition_custody(
        &mut self,
        failure: ResidentA2aGetTaskLocalDispositionCustodyFailure,
    ) -> Result<
        ResidentRetainedA2aGetTaskLocalDisposition,
        ResidentA2aGetTaskLocalDispositionCustodyFailure,
    > {
        let (terminal, pending) = match failure {
            ResidentA2aGetTaskLocalDispositionCustodyFailure::Association { .. } => {
                return Err(failure);
            }
            ResidentA2aGetTaskLocalDispositionCustodyFailure::Commit { terminal, failure } => {
                let pending = match (*failure).reconcile_exact(self.custody) {
                    Ok(pending) => pending,
                    Err(failure) => match failure.try_into_retryable() {
                        Ok(prepared) => match prepared.commit(self.custody, self.anchor) {
                            Ok(pending) => pending,
                            Err(failure) => {
                                return Err(
                                    ResidentA2aGetTaskLocalDispositionCustodyFailure::Commit {
                                        terminal,
                                        failure: Box::new(failure),
                                    },
                                );
                            }
                        },
                        Err(failure) => {
                            return Err(ResidentA2aGetTaskLocalDispositionCustodyFailure::Commit {
                                terminal,
                                failure: Box::new(failure),
                            });
                        }
                    },
                };
                (terminal, pending)
            }
            ResidentA2aGetTaskLocalDispositionCustodyFailure::Anchor { terminal, failure } => {
                match (*failure).try_into_retryable() {
                    Ok(pending) => (terminal, pending),
                    Err(failure) => match failure.continue_after_failed_observation(self.anchor) {
                        Ok(custody) => {
                            return Ok(ResidentRetainedA2aGetTaskLocalDisposition {
                                terminal,
                                custody: Box::new(custody),
                            });
                        }
                        Err(failure) => {
                            return Err(ResidentA2aGetTaskLocalDispositionCustodyFailure::Anchor {
                                terminal,
                                failure: Box::new(failure),
                            });
                        }
                    },
                }
            }
        };
        match pending.advance_anchor(self.anchor) {
            Ok(custody) => Ok(ResidentRetainedA2aGetTaskLocalDisposition {
                terminal,
                custody: Box::new(custody),
            }),
            Err(failure) => Err(ResidentA2aGetTaskLocalDispositionCustodyFailure::Anchor {
                terminal,
                failure: Box::new(failure),
            }),
        }
    }

    /// Release aggregate capacity only after exact local-disposition custody.
    pub(in crate::resident) fn finish_a2a_get_task_local_disposition(
        &mut self,
        retained: ResidentRetainedA2aGetTaskLocalDisposition,
    ) -> Result<
        ResidentReleasedA2aGetTaskLocalDisposition,
        ResidentA2aGetTaskLocalDispositionReleaseFailure,
    > {
        let ResidentRetainedA2aGetTaskLocalDisposition { terminal, custody } = retained;
        match terminal {
            ResidentA2aGetTaskLocalTerminal::Closed(closed) => match self
                .tasks
                .a2a_get_task_owner
                .finish_closed(*closed, &mut self.tasks.reservations)
            {
                Ok(reason) => {
                    Ok(ResidentReleasedA2aGetTaskLocalDisposition::Closed { reason, custody })
                }
                Err(failure) => Err(ResidentA2aGetTaskLocalDispositionReleaseFailure::Closed {
                    custody,
                    failure,
                }),
            },
            ResidentA2aGetTaskLocalTerminal::ReadStopped(stopped) => match self
                .tasks
                .a2a_get_task_owner
                .finish_stopped_response_read(*stopped, &mut self.tasks.reservations)
            {
                Ok(()) => Ok(ResidentReleasedA2aGetTaskLocalDisposition::ReadStopped { custody }),
                Err(failure) => Err(
                    ResidentA2aGetTaskLocalDispositionReleaseFailure::ReadStopped {
                        custody,
                        failure,
                    },
                ),
            },
        }
    }

    /// Retry only aggregate release after local-disposition custody exists.
    pub(in crate::resident) fn retry_a2a_get_task_local_disposition_release(
        &mut self,
        failure: ResidentA2aGetTaskLocalDispositionReleaseFailure,
    ) -> Result<
        ResidentReleasedA2aGetTaskLocalDisposition,
        ResidentA2aGetTaskLocalDispositionReleaseFailure,
    > {
        match failure {
            ResidentA2aGetTaskLocalDispositionReleaseFailure::Closed { custody, failure } => {
                match self
                    .tasks
                    .a2a_get_task_owner
                    .finish_closed(*failure.closed, &mut self.tasks.reservations)
                {
                    Ok(reason) => {
                        Ok(ResidentReleasedA2aGetTaskLocalDisposition::Closed { reason, custody })
                    }
                    Err(failure) => Err(ResidentA2aGetTaskLocalDispositionReleaseFailure::Closed {
                        custody,
                        failure,
                    }),
                }
            }
            ResidentA2aGetTaskLocalDispositionReleaseFailure::ReadStopped { custody, failure } => {
                match self
                    .tasks
                    .a2a_get_task_owner
                    .finish_stopped_response_read(*failure.stopped, &mut self.tasks.reservations)
                {
                    Ok(()) => {
                        Ok(ResidentReleasedA2aGetTaskLocalDisposition::ReadStopped { custody })
                    }
                    Err(failure) => Err(
                        ResidentA2aGetTaskLocalDispositionReleaseFailure::ReadStopped {
                            custody,
                            failure,
                        },
                    ),
                }
            }
        }
    }

    /// Retain one completed read as predecessor-linked remote Task history.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn retain_owned_a2a_get_task<
        Rotation,
        ObservationSign,
        Store,
        CheckpointSign,
    >(
        &mut self,
        awaiting: ResidentA2aGetTaskAwaitingCustody,
        input: A2aGetTaskCommitInput<'_>,
        protocol: &dyn A2aGetTaskEvidencePort,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        resolve_store: Store,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ResidentRetainedA2aGetTask, ResidentA2aGetTaskRetentionFailure>
    where
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &pale_core::records::A2aTaskObservationPreimageV1,
            &pale_core::records::A2aTaskObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Store: FnOnce(
            &pale_core::records::A2aTaskObservationV1,
        ) -> Result<AuthorityStoreCoordinate, InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let ResidentA2aGetTaskAwaitingCustody {
            key,
            completed,
            request_checkpoint,
            request_custody,
            settlement,
            ..
        } = awaiting;
        let completed = *completed;
        if !key.matches_binding(input.observation.history.binding())
            || key.store != *input.observation.history.local_observation().store()
            || self
                .tasks
                .reservations
                .require_a2a_get_task_completion(&settlement, &key)
                .is_err()
        {
            return Err(ResidentA2aGetTaskRetentionFailure {
                key: Box::new(key),
                settlement,
                request_checkpoint,
                request_custody,
                failure: Box::new(ContinuableA2aGetTaskCommitFailure::PrePhysical(
                    A2aGetTaskCommitFailure::Observation {
                        completed: Box::new(completed),
                        reason: A2aGetTaskReadbackError::Association,
                    },
                )),
            });
        }
        match commit_readback_continuable(
            completed,
            input,
            request_checkpoint.as_ref(),
            request_custody.as_deref(),
            protocol,
            self.custody,
            self.anchor,
            admit_rotation,
            sign_observation,
            resolve_store,
            sign_checkpoint,
        ) {
            Ok(custody) => Ok(ResidentRetainedA2aGetTask {
                key: Box::new(key),
                settlement,
                custody: Box::new(custody),
            }),
            Err(failure) => Err(ResidentA2aGetTaskRetentionFailure {
                key: Box::new(key),
                settlement,
                request_checkpoint,
                request_custody,
                failure: Box::new(failure),
            }),
        }
    }

    /// Retry a pre-physical retention failure with corrected fresh inputs.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn retry_owned_a2a_get_task_retention<
        Rotation,
        ObservationSign,
        Store,
        CheckpointSign,
    >(
        &mut self,
        failure: ResidentA2aGetTaskRetentionFailure,
        input: A2aGetTaskCommitInput<'_>,
        protocol: &dyn A2aGetTaskEvidencePort,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        resolve_store: Store,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ResidentRetainedA2aGetTask, ResidentA2aGetTaskRetentionFailure>
    where
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &pale_core::records::A2aTaskObservationPreimageV1,
            &pale_core::records::A2aTaskObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Store: FnOnce(
            &pale_core::records::A2aTaskObservationV1,
        ) -> Result<AuthorityStoreCoordinate, InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let ResidentA2aGetTaskRetentionFailure {
            key,
            settlement,
            request_checkpoint,
            request_custody,
            failure,
        } = failure;
        if !key.matches_binding(input.observation.history.binding())
            || key.store != *input.observation.history.local_observation().store()
            || self
                .tasks
                .reservations
                .require_a2a_get_task_completion(&settlement, &key)
                .is_err()
        {
            return Err(ResidentA2aGetTaskRetentionFailure {
                key,
                settlement,
                request_checkpoint,
                request_custody,
                failure,
            });
        }
        let completed = match (*failure).into_completed() {
            Ok(completed) => completed,
            Err(failure) => {
                return Err(ResidentA2aGetTaskRetentionFailure {
                    key,
                    settlement,
                    request_checkpoint,
                    request_custody,
                    failure: Box::new(failure),
                });
            }
        };
        match commit_readback_continuable(
            completed,
            input,
            request_checkpoint.as_ref(),
            request_custody.as_deref(),
            protocol,
            self.custody,
            self.anchor,
            admit_rotation,
            sign_observation,
            resolve_store,
            sign_checkpoint,
        ) {
            Ok(custody) => Ok(ResidentRetainedA2aGetTask {
                key,
                settlement,
                custody: Box::new(custody),
            }),
            Err(failure) => Err(ResidentA2aGetTaskRetentionFailure {
                key,
                settlement,
                request_checkpoint,
                request_custody,
                failure: Box::new(failure),
            }),
        }
    }

    /// Continue only an exact unresolved database or anchor custody phase.
    pub(in crate::resident) fn continue_owned_a2a_get_task_custody(
        &mut self,
        failure: ResidentA2aGetTaskRetentionFailure,
    ) -> Result<ResidentRetainedA2aGetTask, ResidentA2aGetTaskRetentionFailure> {
        if failure
            .failure
            .binding()
            .is_none_or(|binding| !failure.key.matches_binding(binding))
            || self
                .tasks
                .reservations
                .require_a2a_get_task_completion(&failure.settlement, &failure.key)
                .is_err()
        {
            return Err(failure);
        }
        let ResidentA2aGetTaskRetentionFailure {
            key,
            settlement,
            request_checkpoint,
            request_custody,
            failure,
        } = failure;
        match (*failure).continue_custody(self.custody, self.anchor) {
            Ok(custody) => Ok(ResidentRetainedA2aGetTask {
                key,
                settlement,
                custody: Box::new(custody),
            }),
            Err(failure) => Err(ResidentA2aGetTaskRetentionFailure {
                key,
                settlement,
                request_checkpoint,
                request_custody,
                failure: Box::new(failure),
            }),
        }
    }

    /// Release shared capacity only after exact remote-history custody.
    pub(in crate::resident) fn finish_owned_a2a_get_task(
        &mut self,
        retained: ResidentRetainedA2aGetTask,
    ) -> Result<CompletedA2aGetTaskReadbackCustody, ResidentA2aGetTaskReleaseFailure> {
        if !retained.key.matches_binding(retained.custody.binding())
            || retained.custody.custody().candidate().store() != &retained.key.store
            || self
                .tasks
                .reservations
                .require_a2a_get_task_completion(&retained.settlement, &retained.key)
                .is_err()
        {
            return Err(ResidentA2aGetTaskReleaseFailure::Association(Box::new(
                retained,
            )));
        }
        let ResidentRetainedA2aGetTask {
            key,
            settlement,
            custody,
        } = retained;
        match self.tasks.reservations.release_completion(*settlement) {
            Ok(()) => Ok(*custody),
            Err(release) => Err(ResidentA2aGetTaskReleaseFailure::Settlement {
                key,
                custody,
                release: Box::new(release),
            }),
        }
    }

    /// Retry only aggregate release after durable remote history already exists.
    pub(in crate::resident) fn retry_owned_a2a_get_task_release(
        &mut self,
        failure: ResidentA2aGetTaskReleaseFailure,
    ) -> Result<CompletedA2aGetTaskReadbackCustody, ResidentA2aGetTaskReleaseFailure> {
        let ResidentA2aGetTaskReleaseFailure::Settlement {
            key,
            custody,
            release,
        } = failure
        else {
            return Err(failure);
        };
        if !key.matches_binding(custody.binding())
            || custody.custody().candidate().store() != &key.store
            || self
                .tasks
                .reservations
                .require_a2a_get_task_completion(&release.settlement, &key)
                .is_err()
        {
            return Err(ResidentA2aGetTaskReleaseFailure::Settlement {
                key,
                custody,
                release,
            });
        }
        match self
            .tasks
            .reservations
            .release_completion(*release.settlement)
        {
            Ok(()) => Ok(*custody),
            Err(release) => Err(ResidentA2aGetTaskReleaseFailure::Settlement {
                key,
                custody,
                release: Box::new(release),
            }),
        }
    }
    /// Recover one exact known task binding under the current anchored local head.
    ///
    /// Recovery proves retained history only. The result does not grant current
    /// disclosure or transport authority and does not enumerate remote tasks.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn recover_a2a_task_history<
        Signer,
        State,
        Rotation,
        Signature,
        Checkpoint,
        CheckpointSignature,
    >(
        &mut self,
        input: A2aTaskRecoveryInput<'_>,
        local_admissions: OperationObservationRecoveryAdmission<
            Signer,
            State,
            Rotation,
            Signature,
            Checkpoint,
        >,
        verify_checkpoint_signature: CheckpointSignature,
        remote: A2aTaskRecoveryAdmission<'_>,
    ) -> Result<RecoveredA2aTaskHistory, A2aTaskRecoveryError>
    where
        Checkpoint: FnMut(
            &ResidentRecordRefV1,
            pale_core::records::ControlStateCheckpointWireV1,
        ) -> Result<ControlStateCheckpointV1, InvalidResidentRecordV1>,
        Signer: FnMut(
            &ResidentRecordRefV1,
            &OperationObservationWireV1,
        ) -> Result<SigningAuthorityBindingV1, InvalidResidentRecordV1>,
        State: FnMut(&OperationObservationStateV1) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnMut(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        Signature: FnMut(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        CheckpointSignature: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        recover_history(
            input,
            self.custody,
            self.anchor,
            local_admissions,
            verify_checkpoint_signature,
            remote,
        )
    }

    /// Start one bounded `GetTask` request for the recovered exact binding.
    ///
    /// The caller supplies fresh record identities and current authorization.
    /// This method cannot replay the historical `SendMessage` request.
    pub(in crate::resident) fn start_a2a_get_task_readback<Authorize>(
        &self,
        input: A2aGetTaskReadbackInput<'_>,
        exchange: &dyn A2aExchangePort,
        clock: &dyn WallClockSourcePort,
        authorize: Authorize,
    ) -> Result<InFlightA2aGetTaskReadback, A2aGetTaskReadbackError>
    where
        Authorize: FnOnce(
            A2aGetTaskReadbackAuthorizationContext<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        start_readback(input, exchange, clock, authorize)
    }

    /// Retain completed request and response bytes before releasing read-back.
    ///
    /// The remote task state remains A2A history. This method neither changes
    /// local operation state nor creates a result, settlement, or effect receipt.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn commit_a2a_get_task_readback<
        Rotation,
        ObservationSign,
        Store,
        CheckpointSign,
    >(
        &mut self,
        completed: CompletedA2aGetTaskReadback,
        input: A2aGetTaskCommitInput<'_>,
        protocol: &dyn A2aGetTaskEvidencePort,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        resolve_store: Store,
        sign_checkpoint: CheckpointSign,
    ) -> Result<CompletedA2aGetTaskReadbackCustody, A2aGetTaskCommitFailure>
    where
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &pale_core::records::A2aTaskObservationPreimageV1,
            &pale_core::records::A2aTaskObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Store: FnOnce(
            &pale_core::records::A2aTaskObservationV1,
        ) -> Result<AuthorityStoreCoordinate, InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        commit_readback(
            completed,
            input,
            protocol,
            self.custody,
            self.anchor,
            admit_rotation,
            sign_observation,
            resolve_store,
            sign_checkpoint,
        )
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/operation_service/a2a_readback.rs"]
mod tests;
