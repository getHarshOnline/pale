//! Private composition of durable A2A `CancelTask` request and response phases.
//!
//! Aggregate capacity is accepted before the carrier can start. A carrier-started
//! request never returns to a replayable phase. Remote state remains history.

use pale_core::records::{
    ArtifactDigest, CanonicalSignatureV1, ControlCheckpointSigningAuthorityAdmissionContextV1,
    ControlStateCheckpointDigest, ControlStateCheckpointPreimageV1, InvalidResidentRecordV1,
    OperationObservationDigest, OperationObservationPreimageV1, ResidentRecordRefV1, SchemaDigest,
    UnixTimeV1,
};
use pale_ports::{
    A2aCancelTaskEvidencePort, A2aExchangePort, AccumulatorPreparationLimits, WallClockSourcePort,
};

use super::ResidentOperationService;
use crate::resident::a2a_cancel_task_owner::{
    a2a_cancel_task_key, ResidentA2aCancelTaskAggregateAcceptanceFailure,
    ResidentA2aCancelTaskAggregateAccepted, ResidentA2aCancelTaskAggregateTransitionRejection,
    ResidentA2aCancelTaskAwaitingCustody, ResidentA2aCancelTaskDispositionAssociationFailure,
    ResidentA2aCancelTaskDispositionCustodyToken, ResidentA2aCancelTaskDurableCustody,
    ResidentA2aCancelTaskOwnerPoll, ResidentA2aCancelTaskReadbackAssociationFailure,
    ResidentA2aCancelTaskReadbackCustodyToken, ResidentA2aCancelTaskReleaseFailure,
    ResidentA2aCancelTaskReservation, ResidentA2aCancelTaskReservationReleaseFailure,
    ResidentA2aCancelTaskStart, ResidentA2aCancelTaskStartContext,
    ResidentA2aCancelTaskStartTransitionFailure, ResidentRetainedA2aCancelTaskOutcome,
};
use crate::resident::a2a_task_custody::{
    commit_a2a_cancel_task_disposition_continuable, commit_a2a_cancel_task_readback_continuable,
    A2aCancelTaskDispositionCommitInput, A2aCancelTaskDispositionPhase,
    A2aCancelTaskReadbackCommitInput, ContinuableA2aCancelTaskCustodyFailure,
    ContinuableA2aCancelTaskDispositionCommitFailure,
    ContinuableA2aCancelTaskReadbackCommitFailure,
};
use crate::resident::task_reservation::ResidentA2aTaskWorkKey;
use crate::resident::task_reservation::ResidentTaskReservationTransitionFailure;
use crate::resident::{
    prepare_a2a_cancel_task_custody, A2aCancelTaskAuthorizationContext, A2aCancelTaskError,
    A2aCancelTaskInput, OperationCheckpointInput, PreparedA2aCancelTaskCustody,
    RetainedA2aCancelTask,
};

/// Prepared local cancellation request that still owns its aggregate offer.
#[must_use = "complete physical custody or release the exact unopened reservation"]
pub(in crate::resident) struct ResidentPreparedA2aCancelTask<'a> {
    pub(in crate::resident) reservation: ResidentA2aCancelTaskReservation,
    prepared: Box<PreparedA2aCancelTaskCustody<'a>>,
}

/// Initial request preparation stopped before carrier or physical write authority.
#[must_use = "preserve an unresolved aggregate release"]
pub(in crate::resident) enum ResidentA2aCancelTaskPreparationFailure {
    Reservation(Box<crate::resident::task_reservation::ResidentA2aTaskReservationOfferFailure>),
    Closed(Box<A2aCancelTaskError>),
    Release {
        failure: Box<ResidentA2aCancelTaskReservationReleaseFailure>,
        reason: Box<A2aCancelTaskError>,
    },
}

/// Physical request custody stopped while aggregate capacity remains offered.
#[must_use = "continue exact physical custody or classify non-new evidence"]
pub(in crate::resident) enum ResidentA2aCancelTaskRequestCustodyFailure<'a> {
    /// A prepared phase failed its owner or offered-reservation fence.
    PreparedAssociation {
        prepared: Box<ResidentPreparedA2aCancelTask<'a>>,
        reason: ResidentTaskReservationTransitionFailure,
    },
    /// An unresolved physical phase failed its owner or reservation fence.
    ContinuingAssociation {
        reservation: ResidentA2aCancelTaskReservation,
        failure: ContinuableA2aCancelTaskCustodyFailure<'a>,
        reason: ResidentTaskReservationTransitionFailure,
    },
    /// Database or anchor custody remains unresolved under the exact owner.
    Custody {
        reservation: ResidentA2aCancelTaskReservation,
        failure: ContinuableA2aCancelTaskCustodyFailure<'a>,
    },
}

/// Exact replay or recovered database custody that cannot authorize carrier start.
#[must_use = "release only the aggregate offer; never start CancelTask"]
pub(in crate::resident) struct ResidentA2aCancelTaskNonDispatchableReplayCustody {
    pub(in crate::resident) reservation: ResidentA2aCancelTaskReservation,
    pub(in crate::resident) custody: Box<crate::resident::CompletedOperationObservationCustody>,
}

/// Retained request and still-offered aggregate capacity before local acceptance.
#[must_use = "accept aggregate capacity before fresh preflight and carrier start"]
pub(in crate::resident) struct ResidentRetainedUnopenedA2aCancelTask<'a> {
    pub(in crate::resident) reservation: ResidentA2aCancelTaskReservation,
    pub(in crate::resident) retained: Box<RetainedA2aCancelTask<'a>>,
}

/// Complete carrier read-back selected for durable remote-history custody.
#[must_use = "retain predecessor-linked remote history before aggregate release"]
pub(in crate::resident) struct ResidentSelectedA2aCancelTaskReadback {
    token: Box<ResidentA2aCancelTaskReadbackCustodyToken>,
    completed: Box<crate::resident::CompletedA2aCancelTaskReadback>,
}

enum ResidentA2aCancelTaskReadbackRetentionPhase {
    Selected(Box<ResidentSelectedA2aCancelTaskReadback>),
    Continuing {
        token: Box<ResidentA2aCancelTaskReadbackCustodyToken>,
        failure: Box<ContinuableA2aCancelTaskReadbackCommitFailure>,
    },
    Association(Box<ResidentA2aCancelTaskReadbackAssociationFailure>),
}

/// Remote-history retention stopped while exact aggregate capacity remains held.
#[must_use = "retry pre-physical admission or continue exact physical custody"]
pub(in crate::resident) struct ResidentA2aCancelTaskReadbackRetentionFailure {
    phase: ResidentA2aCancelTaskReadbackRetentionPhase,
}

/// Incomplete carrier outcome selected for durable local disposition.
#[must_use = "retain exact local disposition before aggregate release"]
pub(in crate::resident) struct ResidentSelectedA2aCancelTaskDisposition {
    token: Box<ResidentA2aCancelTaskDispositionCustodyToken>,
    phase: Box<A2aCancelTaskDispositionPhase>,
}

enum ResidentA2aCancelTaskDispositionRetentionPhase {
    Selected(Box<ResidentSelectedA2aCancelTaskDisposition>),
    Continuing {
        token: Box<ResidentA2aCancelTaskDispositionCustodyToken>,
        failure: Box<ContinuableA2aCancelTaskDispositionCommitFailure>,
    },
    Association(Box<ResidentA2aCancelTaskDispositionAssociationFailure>),
}

/// Local disposition retention stopped while aggregate capacity remains held.
#[must_use = "retry pre-physical admission or continue exact physical custody"]
pub(in crate::resident) struct ResidentA2aCancelTaskDispositionRetentionFailure {
    phase: ResidentA2aCancelTaskDispositionRetentionPhase,
}

impl ResidentSelectedA2aCancelTaskReadback {
    /// Borrow the binding-qualified key retained with the complete read-back.
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        self.token.key()
    }
}

impl ResidentA2aCancelTaskReadbackRetentionFailure {
    /// Borrow the key retained by this exact terminal phase.
    pub(in crate::resident) fn key(&self) -> &ResidentA2aTaskWorkKey {
        match &self.phase {
            ResidentA2aCancelTaskReadbackRetentionPhase::Selected(selected) => selected.key(),
            ResidentA2aCancelTaskReadbackRetentionPhase::Continuing { token, .. } => token.key(),
            ResidentA2aCancelTaskReadbackRetentionPhase::Association(failure) => {
                failure.token.key()
            }
        }
    }
}

impl ResidentSelectedA2aCancelTaskDisposition {
    /// Borrow the binding-qualified key retained with the incomplete outcome.
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        self.token.key()
    }
}

impl ResidentA2aCancelTaskDispositionRetentionFailure {
    /// Borrow the key retained by this exact terminal phase.
    pub(in crate::resident) fn key(&self) -> &ResidentA2aTaskWorkKey {
        match &self.phase {
            ResidentA2aCancelTaskDispositionRetentionPhase::Selected(selected) => selected.key(),
            ResidentA2aCancelTaskDispositionRetentionPhase::Continuing { token, .. } => token.key(),
            ResidentA2aCancelTaskDispositionRetentionPhase::Association(failure) => {
                failure.token.key()
            }
        }
    }
}

impl ResidentPreparedA2aCancelTask<'_> {
    /// Borrow the binding-qualified key retained with the prepared request.
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        &self.reservation.key
    }
}

impl ResidentA2aCancelTaskPreparationFailure {
    /// Borrow the key only when unopened-offer release remains unresolved.
    pub(in crate::resident) fn unresolved_key(&self) -> Option<&ResidentA2aTaskWorkKey> {
        match self {
            Self::Release { failure, .. } => Some(failure.key()),
            Self::Reservation(_) | Self::Closed(_) => None,
        }
    }
}

impl ResidentA2aCancelTaskRequestCustodyFailure<'_> {
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

impl ResidentA2aCancelTaskNonDispatchableReplayCustody {
    /// Borrow the key retained with physical replay custody that cannot dispatch.
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        &self.reservation.key
    }
}

impl ResidentA2aCancelTaskNonDispatchableReplayReleaseFailure {
    /// Borrow the key retained with failed aggregate-offer release.
    pub(in crate::resident) fn key(&self) -> &ResidentA2aTaskWorkKey {
        self.failure.key()
    }
}

impl ResidentRetainedUnopenedA2aCancelTask<'_> {
    /// Borrow the key of this retained unopened cancellation request.
    pub(in crate::resident) const fn key(&self) -> &ResidentA2aTaskWorkKey {
        &self.reservation.key
    }
}

impl<'ports, Authority> ResidentOperationService<'ports, Authority> {
    /// Reserve binding-wide capacity, then prepare one local cancellation request.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn prepare_owned_a2a_cancel_task<
        'a,
        Authorize,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        input: A2aCancelTaskInput<'a>,
        port: &dyn A2aExchangePort,
        clock: &dyn WallClockSourcePort,
        authorize: Authorize,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        checkpoint: OperationCheckpointInput,
        checkpoint_issued_at: UnixTimeV1,
        checkpoint_schema: &SchemaDigest,
        checkpoint_authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        accumulator_limits: AccumulatorPreparationLimits,
        resolve_store: &dyn crate::resident::OperationObservationAuthorityStoreResolver,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ResidentPreparedA2aCancelTask<'a>, ResidentA2aCancelTaskPreparationFailure>
    where
        Authorize:
            FnOnce(A2aCancelTaskAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
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
    {
        let key = a2a_cancel_task_key(input.history);
        let reservation = self
            .tasks
            .a2a_cancel_task_owner
            .reserve(key, &mut self.tasks.reservations)
            .map_err(|failure| {
                ResidentA2aCancelTaskPreparationFailure::Reservation(Box::new(failure))
            })?;
        match prepare_a2a_cancel_task_custody(
            input,
            port,
            clock,
            authorize,
            admit_rotation,
            sign_observation,
            checkpoint,
            checkpoint_issued_at,
            checkpoint_schema,
            checkpoint_authority,
            accumulator_limits,
            resolve_store,
            self.custody,
            sign_checkpoint,
        ) {
            Ok(prepared) => Ok(ResidentPreparedA2aCancelTask {
                reservation,
                prepared: Box::new(prepared),
            }),
            Err(reason) => match self
                .tasks
                .a2a_cancel_task_owner
                .release_unopened(reservation, &mut self.tasks.reservations)
            {
                Ok(()) => Err(ResidentA2aCancelTaskPreparationFailure::Closed(Box::new(
                    reason,
                ))),
                Err(failure) => Err(ResidentA2aCancelTaskPreparationFailure::Release {
                    failure: Box::new(failure),
                    reason: Box::new(reason),
                }),
            },
        }
    }

    /// Commit and anchor the prepared local request without carrier access.
    pub(in crate::resident) fn retain_prepared_a2a_cancel_task<'a>(
        &mut self,
        prepared: ResidentPreparedA2aCancelTask<'a>,
    ) -> Result<
        ResidentRetainedUnopenedA2aCancelTask<'a>,
        ResidentA2aCancelTaskRequestCustodyFailure<'a>,
    > {
        if let Err(reason) = self
            .tasks
            .a2a_cancel_task_owner
            .require_unopened(&prepared.reservation, &self.tasks.reservations)
        {
            return Err(
                ResidentA2aCancelTaskRequestCustodyFailure::PreparedAssociation {
                    prepared: Box::new(prepared),
                    reason,
                },
            );
        }
        match prepared
            .prepared
            .retain_continuable(self.custody, self.anchor)
        {
            Ok(retained) => Ok(ResidentRetainedUnopenedA2aCancelTask {
                reservation: prepared.reservation,
                retained: Box::new(retained),
            }),
            Err(failure) => Err(ResidentA2aCancelTaskRequestCustodyFailure::Custody {
                reservation: prepared.reservation,
                failure,
            }),
        }
    }

    /// Retry only offered-capacity release after preparation already closed.
    pub(in crate::resident) fn retry_a2a_cancel_task_preparation_release(
        &mut self,
        pending: ResidentA2aCancelTaskPreparationFailure,
    ) -> Result<A2aCancelTaskError, ResidentA2aCancelTaskPreparationFailure> {
        let ResidentA2aCancelTaskPreparationFailure::Release { failure, reason } = pending else {
            return Err(pending);
        };
        match self
            .tasks
            .a2a_cancel_task_owner
            .retry_unopened_release(*failure, &mut self.tasks.reservations)
        {
            Ok(()) => Ok(*reason),
            Err(retry) => Err(ResidentA2aCancelTaskPreparationFailure::Release {
                failure: Box::new(retry),
                reason,
            }),
        }
    }

    /// Continue only unresolved request database or anchor custody.
    pub(in crate::resident) fn continue_a2a_cancel_task_request_custody<'a>(
        &mut self,
        pending: ResidentA2aCancelTaskRequestCustodyFailure<'a>,
    ) -> Result<
        ResidentRetainedUnopenedA2aCancelTask<'a>,
        ResidentA2aCancelTaskRequestCustodyFailure<'a>,
    > {
        let (reservation, failure) = match pending {
            ResidentA2aCancelTaskRequestCustodyFailure::PreparedAssociation {
                prepared, ..
            } => return self.retain_prepared_a2a_cancel_task(*prepared),
            ResidentA2aCancelTaskRequestCustodyFailure::ContinuingAssociation {
                reservation,
                failure,
                ..
            }
            | ResidentA2aCancelTaskRequestCustodyFailure::Custody {
                reservation,
                failure,
            } => (reservation, failure),
        };
        if let Err(reason) = self
            .tasks
            .a2a_cancel_task_owner
            .require_unopened(&reservation, &self.tasks.reservations)
        {
            return Err(
                ResidentA2aCancelTaskRequestCustodyFailure::ContinuingAssociation {
                    reservation,
                    failure,
                    reason,
                },
            );
        }
        match failure.continue_custody(self.custody, self.anchor) {
            Ok(retained) => Ok(ResidentRetainedUnopenedA2aCancelTask {
                reservation,
                retained: Box::new(retained),
            }),
            Err(failure) => Err(ResidentA2aCancelTaskRequestCustodyFailure::Custody {
                reservation,
                failure,
            }),
        }
    }

    /// Extract exact replay custody without creating carrier authority.
    pub(in crate::resident) fn classify_non_dispatchable_replay_a2a_cancel_task<'a>(
        &self,
        pending: ResidentA2aCancelTaskRequestCustodyFailure<'a>,
    ) -> Result<
        ResidentA2aCancelTaskNonDispatchableReplayCustody,
        ResidentA2aCancelTaskRequestCustodyFailure<'a>,
    > {
        let ResidentA2aCancelTaskRequestCustodyFailure::Custody {
            reservation,
            failure,
        } = pending
        else {
            return Err(pending);
        };
        match failure.into_not_new_custody() {
            Ok(custody) => Ok(ResidentA2aCancelTaskNonDispatchableReplayCustody {
                reservation,
                custody: Box::new(custody),
            }),
            Err(failure) => Err(ResidentA2aCancelTaskRequestCustodyFailure::Custody {
                reservation,
                failure,
            }),
        }
    }

    /// Release aggregate capacity for replay custody that cannot dispatch.
    pub(in crate::resident) fn finish_non_dispatchable_replay_a2a_cancel_task(
        &mut self,
        closed: ResidentA2aCancelTaskNonDispatchableReplayCustody,
    ) -> Result<
        crate::resident::CompletedOperationObservationCustody,
        ResidentA2aCancelTaskNonDispatchableReplayReleaseFailure,
    > {
        match self
            .tasks
            .a2a_cancel_task_owner
            .release_unopened(closed.reservation, &mut self.tasks.reservations)
        {
            Ok(()) => Ok(*closed.custody),
            Err(failure) => Err(ResidentA2aCancelTaskNonDispatchableReplayReleaseFailure {
                failure,
                custody: closed.custody,
            }),
        }
    }

    /// Retry only aggregate-offer release for non-dispatchable replay custody.
    pub(in crate::resident) fn retry_non_dispatchable_replay_a2a_cancel_task_release(
        &mut self,
        pending: ResidentA2aCancelTaskNonDispatchableReplayReleaseFailure,
    ) -> Result<
        crate::resident::CompletedOperationObservationCustody,
        ResidentA2aCancelTaskNonDispatchableReplayReleaseFailure,
    > {
        match self
            .tasks
            .a2a_cancel_task_owner
            .retry_unopened_release(pending.failure, &mut self.tasks.reservations)
        {
            Ok(()) => Ok(*pending.custody),
            Err(failure) => Err(ResidentA2aCancelTaskNonDispatchableReplayReleaseFailure {
                failure,
                custody: pending.custody,
            }),
        }
    }

    /// Accept aggregate capacity before fresh preflight and carrier start.
    pub(in crate::resident) fn start_owned_a2a_cancel_task<'a, Authorize>(
        &mut self,
        unopened: ResidentRetainedUnopenedA2aCancelTask<'a>,
        start: ResidentA2aCancelTaskStartContext<'_, Authorize>,
    ) -> Result<ResidentA2aCancelTaskStart<'a>, ResidentA2aCancelTaskStartTransitionFailure<'a>>
    where
        Authorize:
            FnOnce(A2aCancelTaskAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
    {
        self.tasks.a2a_cancel_task_owner.start(
            unopened.reservation,
            *unopened.retained,
            start,
            &mut self.tasks.reservations,
        )
    }

    /// Retry only aggregate acceptance before any carrier call.
    pub(in crate::resident) fn retry_a2a_cancel_task_aggregate_acceptance<'a>(
        &mut self,
        failure: ResidentA2aCancelTaskAggregateAcceptanceFailure<'a>,
    ) -> Result<
        ResidentA2aCancelTaskAggregateAccepted<'a>,
        ResidentA2aCancelTaskAggregateAcceptanceFailure<'a>,
    > {
        self.tasks
            .a2a_cancel_task_owner
            .retry_acceptance(failure, &mut self.tasks.reservations)
    }

    /// Retry an unopened start after correcting only its association inputs.
    pub(in crate::resident) fn retry_a2a_cancel_task_start_association<'a, Authorize>(
        &mut self,
        failure: ResidentA2aCancelTaskStartTransitionFailure<'a>,
        start: ResidentA2aCancelTaskStartContext<'_, Authorize>,
    ) -> Result<ResidentA2aCancelTaskStart<'a>, ResidentA2aCancelTaskStartTransitionFailure<'a>>
    where
        Authorize:
            FnOnce(A2aCancelTaskAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
    {
        let ResidentA2aCancelTaskStartTransitionFailure::Association {
            reservation,
            retained,
        } = failure
        else {
            return Err(failure);
        };
        self.tasks.a2a_cancel_task_owner.start(
            reservation,
            *retained,
            start,
            &mut self.tasks.reservations,
        )
    }

    /// Apply fresh preflight to an already aggregate-accepted unopened request.
    pub(in crate::resident) fn start_aggregate_accepted_a2a_cancel_task<'a, Authorize>(
        &mut self,
        accepted: ResidentA2aCancelTaskAggregateAccepted<'a>,
        start: ResidentA2aCancelTaskStartContext<'_, Authorize>,
    ) -> ResidentA2aCancelTaskStart<'a>
    where
        Authorize:
            FnOnce(A2aCancelTaskAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
    {
        self.tasks.a2a_cancel_task_owner.start_accepted(
            accepted,
            start,
            &mut self.tasks.reservations,
        )
    }

    /// Retry only fresh preflight for the same aggregate-accepted request.
    pub(in crate::resident) fn retry_a2a_cancel_task_preflight<'a, Authorize>(
        &mut self,
        failure: crate::resident::a2a_cancel_task_owner::ResidentA2aCancelTaskPreflightFailure<'a>,
        start: ResidentA2aCancelTaskStartContext<'_, Authorize>,
    ) -> ResidentA2aCancelTaskStart<'a>
    where
        Authorize:
            FnOnce(A2aCancelTaskAuthorizationContext<'_>) -> Result<(), InvalidResidentRecordV1>,
    {
        self.tasks.a2a_cancel_task_owner.start_accepted(
            failure.aggregate_accepted,
            start,
            &mut self.tasks.reservations,
        )
    }

    /// Poll one exact active response read without replay or reconnect.
    pub(in crate::resident) fn poll_owned_a2a_cancel_task(
        &mut self,
        key: &ResidentA2aTaskWorkKey,
        cx: &mut std::task::Context<'_>,
        protocol: &dyn A2aCancelTaskEvidencePort,
        clock: &dyn WallClockSourcePort,
    ) -> ResidentA2aCancelTaskOwnerPoll {
        self.tasks.a2a_cancel_task_owner.poll(
            key,
            cx,
            protocol,
            clock,
            &mut self.tasks.reservations,
        )
    }

    /// Stop local response reading without claiming remote or Pale cancellation.
    pub(in crate::resident) fn stop_owned_a2a_cancel_task_response_read(
        &mut self,
        key: &ResidentA2aTaskWorkKey,
    ) -> ResidentA2aCancelTaskOwnerPoll {
        self.tasks
            .a2a_cancel_task_owner
            .stop_response_read(key, &mut self.tasks.reservations)
    }

    /// Retry only the accepted-to-completion aggregate transition.
    pub(in crate::resident) fn retry_owned_a2a_cancel_task_aggregate_transition(
        &mut self,
        rejected: ResidentA2aCancelTaskAggregateTransitionRejection,
    ) -> ResidentA2aCancelTaskOwnerPoll {
        self.tasks
            .a2a_cancel_task_owner
            .retry_aggregate_transition(rejected, &mut self.tasks.reservations)
    }

    /// Select only a complete qualified read-back for remote-history retention.
    pub(in crate::resident) fn select_a2a_cancel_task_readback(
        &self,
        awaiting: ResidentA2aCancelTaskAwaitingCustody,
    ) -> Result<ResidentSelectedA2aCancelTaskReadback, ResidentA2aCancelTaskAwaitingCustody> {
        self.tasks
            .a2a_cancel_task_owner
            .select_readback(awaiting)
            .map(|(token, completed)| ResidentSelectedA2aCancelTaskReadback {
                token: Box::new(token),
                completed: Box::new(completed),
            })
    }

    /// Retain a complete response as predecessor-linked remote Task history.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn retain_a2a_cancel_task_readback<
        Rotation,
        ObservationSign,
        Store,
        CheckpointSign,
    >(
        &mut self,
        selected: ResidentSelectedA2aCancelTaskReadback,
        input: A2aCancelTaskReadbackCommitInput<'_>,
        protocol: &dyn A2aCancelTaskEvidencePort,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        resolve_store: Store,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ResidentRetainedA2aCancelTaskOutcome, ResidentA2aCancelTaskReadbackRetentionFailure>
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
        )
            -> Result<pale_ports::AuthorityStoreCoordinate, InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        if !selected
            .token
            .key()
            .matches_binding(input.observation.history.binding())
            || selected.token.key().store != *input.observation.history.local_observation().store()
            || self
                .tasks
                .a2a_cancel_task_owner
                .require_readback(&selected.token, &self.tasks.reservations)
                .is_err()
        {
            return Err(ResidentA2aCancelTaskReadbackRetentionFailure {
                phase: ResidentA2aCancelTaskReadbackRetentionPhase::Selected(Box::new(selected)),
            });
        }
        let ResidentSelectedA2aCancelTaskReadback { token, completed } = selected;
        match commit_a2a_cancel_task_readback_continuable(
            *completed,
            input,
            protocol,
            self.custody,
            self.anchor,
            admit_rotation,
            sign_observation,
            resolve_store,
            sign_checkpoint,
        ) {
            Ok(custody) => self
                .tasks
                .a2a_cancel_task_owner
                .retain_readback(*token, custody, &self.tasks.reservations)
                .map_err(|failure| ResidentA2aCancelTaskReadbackRetentionFailure {
                    phase: ResidentA2aCancelTaskReadbackRetentionPhase::Association(Box::new(
                        failure,
                    )),
                }),
            Err(failure) => Err(ResidentA2aCancelTaskReadbackRetentionFailure {
                phase: ResidentA2aCancelTaskReadbackRetentionPhase::Continuing {
                    token,
                    failure: Box::new(failure),
                },
            }),
        }
    }

    /// Continue only exact read-back database or independent-anchor custody.
    pub(in crate::resident) fn continue_a2a_cancel_task_readback_custody(
        &mut self,
        pending: ResidentA2aCancelTaskReadbackRetentionFailure,
    ) -> Result<ResidentRetainedA2aCancelTaskOutcome, ResidentA2aCancelTaskReadbackRetentionFailure>
    {
        let ResidentA2aCancelTaskReadbackRetentionPhase::Continuing { token, failure } =
            pending.phase
        else {
            return Err(pending);
        };
        if failure
            .binding()
            .is_none_or(|binding| !token.key().matches_binding(binding))
            || failure.store() != Some(&token.key().store)
            || self
                .tasks
                .a2a_cancel_task_owner
                .require_readback(&token, &self.tasks.reservations)
                .is_err()
        {
            return Err(ResidentA2aCancelTaskReadbackRetentionFailure {
                phase: ResidentA2aCancelTaskReadbackRetentionPhase::Continuing { token, failure },
            });
        }
        match failure.continue_custody(self.custody, self.anchor) {
            Ok(custody) => self
                .tasks
                .a2a_cancel_task_owner
                .retain_readback(*token, custody, &self.tasks.reservations)
                .map_err(|failure| ResidentA2aCancelTaskReadbackRetentionFailure {
                    phase: ResidentA2aCancelTaskReadbackRetentionPhase::Association(Box::new(
                        failure,
                    )),
                }),
            Err(failure) => Err(ResidentA2aCancelTaskReadbackRetentionFailure {
                phase: ResidentA2aCancelTaskReadbackRetentionPhase::Continuing {
                    token,
                    failure: Box::new(failure),
                },
            }),
        }
    }

    /// Recover completed carrier evidence after a pre-physical admission failure.
    ///
    /// This method performs no carrier, signing, storage, or anchor work. The
    /// returned selected phase can be retried only with explicit corrected inputs.
    pub(in crate::resident) fn recover_a2a_cancel_task_readback_prephysical(
        &self,
        pending: ResidentA2aCancelTaskReadbackRetentionFailure,
    ) -> Result<ResidentSelectedA2aCancelTaskReadback, ResidentA2aCancelTaskReadbackRetentionFailure>
    {
        let ResidentA2aCancelTaskReadbackRetentionPhase::Continuing { token, failure } =
            pending.phase
        else {
            return Err(pending);
        };
        if self
            .tasks
            .a2a_cancel_task_owner
            .require_readback(&token, &self.tasks.reservations)
            .is_err()
        {
            return Err(ResidentA2aCancelTaskReadbackRetentionFailure {
                phase: ResidentA2aCancelTaskReadbackRetentionPhase::Continuing { token, failure },
            });
        }
        match failure.into_completed() {
            Ok((completed, _reason)) => Ok(ResidentSelectedA2aCancelTaskReadback {
                token,
                completed: Box::new(completed),
            }),
            Err(failure) => Err(ResidentA2aCancelTaskReadbackRetentionFailure {
                phase: ResidentA2aCancelTaskReadbackRetentionPhase::Continuing {
                    token,
                    failure: Box::new(failure),
                },
            }),
        }
    }

    /// Select only an incomplete carrier outcome for local disposition custody.
    pub(in crate::resident) fn select_a2a_cancel_task_disposition(
        &self,
        awaiting: ResidentA2aCancelTaskAwaitingCustody,
    ) -> Result<ResidentSelectedA2aCancelTaskDisposition, ResidentA2aCancelTaskAwaitingCustody>
    {
        self.tasks
            .a2a_cancel_task_owner
            .select_disposition(awaiting)
            .map(|(token, phase)| ResidentSelectedA2aCancelTaskDisposition {
                token: Box::new(token),
                phase: Box::new(phase),
            })
    }

    /// Retain one incomplete outcome as a local uncertainty disposition.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn retain_a2a_cancel_task_disposition<
        Reconciliation,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        selected: ResidentSelectedA2aCancelTaskDisposition,
        input: A2aCancelTaskDispositionCommitInput<'_>,
        resolve_store: &dyn crate::resident::OperationObservationAuthorityStoreResolver,
        admit_reconciliation: Reconciliation,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentRetainedA2aCancelTaskOutcome,
        ResidentA2aCancelTaskDispositionRetentionFailure,
    >
    where
        Reconciliation: FnOnce(
            &pale_core::records::ReconciliationFrontierReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
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
    {
        let local = input
            .disposition
            .history
            .local_observation()
            .selected_head();
        if !selected
            .token
            .key()
            .matches_binding(input.disposition.history.binding())
            || selected.token.key().store != *input.disposition.history.local_observation().store()
            || selected.token.cancellation()
                != (local.0, local.1.canonical_operation_observation_sha256())
            || self
                .tasks
                .a2a_cancel_task_owner
                .require_disposition(&selected.token, &self.tasks.reservations)
                .is_err()
        {
            return Err(ResidentA2aCancelTaskDispositionRetentionFailure {
                phase: ResidentA2aCancelTaskDispositionRetentionPhase::Selected(Box::new(selected)),
            });
        }
        let ResidentSelectedA2aCancelTaskDisposition { token, phase } = selected;
        match commit_a2a_cancel_task_disposition_continuable(
            *phase,
            input,
            resolve_store,
            self.custody,
            self.anchor,
            admit_reconciliation,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        ) {
            Ok(custody) => self
                .tasks
                .a2a_cancel_task_owner
                .retain_disposition(*token, custody, &self.tasks.reservations)
                .map_err(|failure| ResidentA2aCancelTaskDispositionRetentionFailure {
                    phase: ResidentA2aCancelTaskDispositionRetentionPhase::Association(Box::new(
                        failure,
                    )),
                }),
            Err(failure) => Err(ResidentA2aCancelTaskDispositionRetentionFailure {
                phase: ResidentA2aCancelTaskDispositionRetentionPhase::Continuing {
                    token,
                    failure: Box::new(failure),
                },
            }),
        }
    }

    /// Continue only exact disposition database or independent-anchor custody.
    pub(in crate::resident) fn continue_a2a_cancel_task_disposition_custody(
        &mut self,
        pending: ResidentA2aCancelTaskDispositionRetentionFailure,
    ) -> Result<
        ResidentRetainedA2aCancelTaskOutcome,
        ResidentA2aCancelTaskDispositionRetentionFailure,
    > {
        let ResidentA2aCancelTaskDispositionRetentionPhase::Continuing { token, failure } =
            pending.phase
        else {
            return Err(pending);
        };
        if failure
            .binding()
            .is_none_or(|binding| !token.key().matches_binding(binding))
            || failure.store() != Some(&token.key().store)
            || failure.cancellation() != Some(token.cancellation())
            || self
                .tasks
                .a2a_cancel_task_owner
                .require_disposition(&token, &self.tasks.reservations)
                .is_err()
        {
            return Err(ResidentA2aCancelTaskDispositionRetentionFailure {
                phase: ResidentA2aCancelTaskDispositionRetentionPhase::Continuing {
                    token,
                    failure,
                },
            });
        }
        match failure.continue_custody(self.custody, self.anchor) {
            Ok(custody) => self
                .tasks
                .a2a_cancel_task_owner
                .retain_disposition(*token, custody, &self.tasks.reservations)
                .map_err(|failure| ResidentA2aCancelTaskDispositionRetentionFailure {
                    phase: ResidentA2aCancelTaskDispositionRetentionPhase::Association(Box::new(
                        failure,
                    )),
                }),
            Err(failure) => Err(ResidentA2aCancelTaskDispositionRetentionFailure {
                phase: ResidentA2aCancelTaskDispositionRetentionPhase::Continuing {
                    token,
                    failure: Box::new(failure),
                },
            }),
        }
    }

    /// Recover incomplete carrier evidence after pre-physical admission failure.
    ///
    /// This performs no carrier, signing, storage, database, or anchor work.
    pub(in crate::resident) fn recover_a2a_cancel_task_disposition_prephysical(
        &self,
        pending: ResidentA2aCancelTaskDispositionRetentionFailure,
    ) -> Result<
        ResidentSelectedA2aCancelTaskDisposition,
        ResidentA2aCancelTaskDispositionRetentionFailure,
    > {
        let ResidentA2aCancelTaskDispositionRetentionPhase::Continuing { token, failure } =
            pending.phase
        else {
            return Err(pending);
        };
        if self
            .tasks
            .a2a_cancel_task_owner
            .require_disposition(&token, &self.tasks.reservations)
            .is_err()
        {
            return Err(ResidentA2aCancelTaskDispositionRetentionFailure {
                phase: ResidentA2aCancelTaskDispositionRetentionPhase::Continuing {
                    token,
                    failure,
                },
            });
        }
        match failure.into_phase() {
            Ok((phase, _reason)) => Ok(ResidentSelectedA2aCancelTaskDisposition {
                token,
                phase: Box::new(phase),
            }),
            Err(failure) => Err(ResidentA2aCancelTaskDispositionRetentionFailure {
                phase: ResidentA2aCancelTaskDispositionRetentionPhase::Continuing {
                    token,
                    failure: Box::new(failure),
                },
            }),
        }
    }

    /// Release aggregate capacity only after exact durable terminal custody.
    pub(in crate::resident) fn finish_a2a_cancel_task(
        &mut self,
        retained: ResidentRetainedA2aCancelTaskOutcome,
    ) -> Result<ResidentA2aCancelTaskDurableCustody, ResidentA2aCancelTaskReleaseFailure> {
        self.tasks
            .a2a_cancel_task_owner
            .finish_retained(retained, &mut self.tasks.reservations)
    }

    /// Retry only aggregate release after durable terminal custody already exists.
    pub(in crate::resident) fn retry_a2a_cancel_task_release(
        &mut self,
        failure: ResidentA2aCancelTaskReleaseFailure,
    ) -> Result<ResidentA2aCancelTaskDurableCustody, ResidentA2aCancelTaskReleaseFailure> {
        self.tasks
            .a2a_cancel_task_owner
            .finish_retained(*failure.retained, &mut self.tasks.reservations)
    }
}

/// Aggregate release failure for physical replay custody that cannot dispatch.
#[must_use = "retry only the exact unopened aggregate release"]
pub(in crate::resident) struct ResidentA2aCancelTaskNonDispatchableReplayReleaseFailure {
    pub(in crate::resident) failure: ResidentA2aCancelTaskReservationReleaseFailure,
    pub(in crate::resident) custody: Box<crate::resident::CompletedOperationObservationCustody>,
}
