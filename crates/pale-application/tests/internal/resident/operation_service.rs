//! Behavioral checks for private resident operation-service composition.

use super::*;
use crate::effect_case_lineage_fixture as request_fixture;
use crate::local_operation_start_fixture::{
    artifact_digest, checkpoint_preimage, issue_observation, record_ref, schema_digest, signature,
    store_coordinate,
};
use crate::resident::a2a_delegate_task_service::ResidentA2aDelegateCompletion;
use crate::resident::consultation::tests::consultation_task_batch_fixture;
use crate::resident::consultation::tests::consultation_task_batch_fixture_in_store;
use crate::resident::consultation::{
    coordinator::ResidentConsultationCoordinator,
    service::task::{
        ResidentConsultationContinuationToken, ResidentConsultationTaskOwnerResolution,
        ResidentConsultationTaskRefusal, ResidentConsultationTaskService, RetainedConsultationTask,
    },
    ConsultationChildDispatchBatch,
};
use crate::resident::operation_dispatch::selection::tests::provider_dispatch::{
    request_composed_provider_cancellation, with_retained_delegate_dispatch,
    A2aTaskOwnerCustodyModel,
};
use crate::resident::operation_observation_custody::tests::{
    a2a_exchange, local_start, native_execution as custody,
};
use crate::resident::{
    retain_a2a_outcome, A2aDelegateLaunchInput, A2aFailedSendObservationInput,
    A2aInitialTaskCommitInput, A2aInitialTaskCustodyInput, A2aNoTaskOutcomeRetention,
    A2aNotStartedOutcomeRetention, A2aOutcomeRetentionInput, A2aOutcomeUnknownObservationInput,
    A2aRecoveredUnknownOutcomeRetention, A2aRouteAssociation, A2aSendDispatch, A2aSendInput,
    A2aSendObservationCommitInput, A2aSendRecoveryLimits, A2aTaskOutcomeRetention,
    A2aUnknownOutcomeRetention, AuthorizedOperationCancellation, OperationCheckpointAncestryLimits,
    OperationCheckpointInput, RecoveredLocalOperation, OPERATION_CANCEL_CAPABILITY_ID,
};
use crate::session_admission_fixture::ProvingAuthority;
use pale_ports::{
    A2aExchange, A2aExchangeCoordinate, A2aExchangeError, A2aExchangeEvidence, A2aExchangeFacts,
    A2aExchangePort, A2aExchangeRequest, A2aPreparedExchange, A2aSendMessageEvidencePort,
    AuthorityStoreCoordinate, OperationA2aEvidence, OperationCustodyRecordKind,
    WallClockSourceError, WallClockSourcePort,
};
use std::{
    cell::Cell,
    collections::VecDeque,
    num::NonZeroUsize,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll},
};

impl ResidentOperationTaskState {
    pub(in crate::resident) fn seed_consultation_dispatch_for_test<Work, Output, Executor>(
        &mut self,
        batch: ConsultationChildDispatchBatch,
        dispatch: ResidentOperationTaskKey,
        work: Work,
        executor: &mut Executor,
    ) -> ResidentConsultationBatchCoordinate
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        let candidate = batch
            .candidates()
            .first()
            .expect("focused batch has one child")
            .clone();
        let coordinate = self
            .consultation_tasks
            .open_batch(
                batch,
                ResidentConsultationTaskLimits::new(
                    NonZeroUsize::new(1).expect("one in-flight child"),
                    NonZeroUsize::new(1).expect("one roster member"),
                ),
            )
            .expect("focused Consult batch opens");
        self.consultation_tasks
            .seed_retained_for_supervision_test(
                &coordinate,
                candidate,
                dispatch,
                work,
                &mut self.reservations,
                executor,
            )
            .expect("focused Consult child enters its exact task owner");
        coordinate
    }

    pub(in crate::resident) fn set_consultation_reservation_phase_for_test(
        &mut self,
        key: &ResidentOperationTaskKey,
        coordinate: &crate::resident::consultation::ResidentConsultationBatchCoordinate,
        phase: crate::resident::task_reservation::ResidentTaskReservationPhase,
    ) {
        self.reservations.set_phase_for_test(
            key,
            &crate::resident::task_reservation::ResidentTaskReservationRoute::Consultation(
                coordinate.clone(),
            ),
            phase,
        );
    }

    pub(in crate::resident) fn set_a2a_get_task_reservation_phase_for_test(
        &mut self,
        key: &crate::resident::task_reservation::ResidentA2aTaskWorkKey,
        phase: crate::resident::task_reservation::ResidentTaskReservationPhase,
    ) {
        self.reservations.set_a2a_task_phase_for_test(
            key,
            &crate::resident::task_reservation::ResidentTaskReservationRoute::A2aGetTask,
            phase,
        );
    }

    pub(in crate::resident) fn accepted_provider_offer_failure_for_test(
        &mut self,
        key: &crate::resident::task_owner::ResidentOperationTaskKey,
    ) -> crate::resident::task_reservation::ResidentTaskReservationOfferTransitionFailure {
        self.reservations
            .accepted_provider_offer_failure_for_test(key)
    }

    pub(in crate::resident) fn refused_provider_offer_failure_for_test(
        &mut self,
        key: &crate::resident::task_owner::ResidentOperationTaskKey,
    ) -> crate::resident::task_reservation::ResidentTaskReservationOfferTransitionFailure {
        self.reservations
            .refused_provider_offer_failure_for_test(key)
    }

    pub(in crate::resident) fn provider_release_failure_for_test(
        &mut self,
        key: &crate::resident::task_owner::ResidentOperationTaskKey,
    ) -> crate::resident::task_reservation::ResidentTaskReservationSettlementTransitionFailure {
        self.reservations.provider_release_failure_for_test(key)
    }

    pub(in crate::resident) fn set_provider_reservation_phase_for_test(
        &mut self,
        key: &ResidentOperationTaskKey,
        phase: crate::resident::task_reservation::ResidentTaskReservationPhase,
    ) {
        self.reservations.set_phase_for_test(
            key,
            &crate::resident::task_reservation::ResidentTaskReservationRoute::Provider,
            phase,
        );
    }

    pub(in crate::resident) fn accept_provider_reservation_for_test(
        &mut self,
        key: ResidentOperationTaskKey,
    ) {
        let offer = self
            .reservations
            .begin_offer(
                key,
                crate::resident::task_reservation::ResidentTaskReservationRoute::Provider,
            )
            .expect("provider reservation test fixture must have capacity");
        self.reservations
            .accept_offer(offer)
            .expect("provider reservation test fixture must accept its own offer");
    }

    pub(in crate::resident) fn accept_a2a_reservation_for_test(
        &mut self,
        key: ResidentOperationTaskKey,
    ) {
        let offer = self
            .reservations
            .begin_offer(
                key,
                crate::resident::task_reservation::ResidentTaskReservationRoute::A2aDelegate,
            )
            .expect("A2A reservation test fixture must have capacity");
        self.reservations
            .accept_offer(offer)
            .expect("A2A reservation test fixture must accept its own offer");
    }

    pub(in crate::resident) fn consultation_batch_mut(
        &mut self,
        coordinate: &ResidentConsultationBatchCoordinate,
    ) -> Option<&mut ResidentConsultationTaskService> {
        self.consultation_tasks.batch_mut(coordinate)
    }

    pub(in crate::resident) fn accept_retained_consultation_for_test<Work, Output, Executor>(
        &mut self,
        coordinate: &ResidentConsultationBatchCoordinate,
        retained: RetainedConsultationTask<Work>,
        executor: &mut Executor,
    ) -> Option<Result<ResidentOperationTaskKey, ResidentConsultationTaskRefusal<Work>>>
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        self.consultation_tasks.accept_retained_for_test(
            coordinate,
            retained,
            &mut self.reservations,
            executor,
        )
    }

    pub(in crate::resident) fn resolve_consultation_continuation(
        &mut self,
        continuation: ResidentConsultationContinuationToken,
    ) -> ResidentConsultationTaskOwnerResolution {
        self.consultation_tasks
            .resolve_continuation_reserved(continuation, &mut self.reservations)
    }
}

impl<Authority> ResidentOperationService<'_, Authority> {
    pub(in crate::resident) fn set_consultation_reservation_phase_for_test(
        &mut self,
        key: &ResidentOperationTaskKey,
        coordinate: &crate::resident::consultation::ResidentConsultationBatchCoordinate,
        phase: crate::resident::task_reservation::ResidentTaskReservationPhase,
    ) {
        self.tasks
            .set_consultation_reservation_phase_for_test(key, coordinate, phase);
    }
}

impl<'a, Authority> ResidentOperationService<'a, Authority> {
    pub(in crate::resident) fn reject_a2a_cancel_task_release_with_replacement_for_test(
        &mut self,
        retained: crate::resident::a2a_cancel_task_owner::ResidentRetainedA2aCancelTaskOutcome,
    ) -> crate::resident::a2a_cancel_task_owner::ResidentA2aCancelTaskReleaseFailure {
        let mut replacement = ResidentOperationTaskState::new(
            ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
            NonZeroUsize::new(1).unwrap(),
            ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
        )
        .expect("replacement CancelTask owner");
        match replacement
            .a2a_cancel_task_owner
            .finish_retained(retained, &mut replacement.reservations)
        {
            Ok(_) => panic!("a replacement owner cannot release CancelTask capacity"),
            Err(failure) => failure,
        }
    }
}

impl<'ports, Authority> ResidentOperationService<'ports, Authority> {
    pub(in crate::resident) fn begin_a2a_get_task_offer_for_test(
        &mut self,
        key: crate::resident::task_reservation::ResidentA2aTaskWorkKey,
    ) -> crate::resident::task_reservation::ResidentTaskReservationOffer {
        self.tasks
            .reservations
            .begin_a2a_task_offer(
                key,
                crate::resident::task_reservation::ResidentTaskReservationRoute::A2aGetTask,
            )
            .expect("hostile fixture needs one exact GetTask offer")
    }

    pub(in crate::resident) fn reject_a2a_get_task_acceptance_for_test(
        &mut self,
        key: &crate::resident::task_reservation::ResidentA2aTaskWorkKey,
        offer: crate::resident::task_reservation::ResidentTaskReservationOffer,
    ) -> crate::resident::task_reservation::ResidentTaskReservationOfferTransitionFailure {
        let route = crate::resident::task_reservation::ResidentTaskReservationRoute::A2aGetTask;
        self.tasks.reservations.set_a2a_task_phase_for_test(
            key,
            &route,
            crate::resident::task_reservation::ResidentTaskReservationPhase::Accepted,
        );
        let failure = self
            .tasks
            .reservations
            .accept_offer(offer)
            .expect_err("an already accepted GetTask offer must preserve its capability");
        self.tasks.reservations.set_a2a_task_phase_for_test(
            key,
            &route,
            crate::resident::task_reservation::ResidentTaskReservationPhase::Offered,
        );
        failure
    }

    pub(in crate::resident) fn reject_a2a_get_task_rollback_for_test(
        &mut self,
        key: &crate::resident::task_reservation::ResidentA2aTaskWorkKey,
        offer: crate::resident::task_reservation::ResidentTaskReservationOffer,
    ) -> crate::resident::task_reservation::ResidentTaskReservationOfferTransitionFailure {
        let route = crate::resident::task_reservation::ResidentTaskReservationRoute::A2aGetTask;
        self.tasks.reservations.set_a2a_task_phase_for_test(
            key,
            &route,
            crate::resident::task_reservation::ResidentTaskReservationPhase::Accepted,
        );
        let failure = self
            .tasks
            .reservations
            .refuse_offer(offer)
            .expect_err("an accepted GetTask offer must preserve rollback custody");
        self.tasks.reservations.set_a2a_task_phase_for_test(
            key,
            &route,
            crate::resident::task_reservation::ResidentTaskReservationPhase::Offered,
        );
        failure
    }

    pub(in crate::resident) fn set_a2a_get_task_reservation_phase_for_test(
        &mut self,
        key: &crate::resident::task_reservation::ResidentA2aTaskWorkKey,
        phase: crate::resident::task_reservation::ResidentTaskReservationPhase,
    ) {
        self.tasks
            .set_a2a_get_task_reservation_phase_for_test(key, phase);
    }

    pub(in crate::resident) fn get_task_release_failure_for_test(
        &mut self,
        key: crate::resident::task_reservation::ResidentA2aTaskWorkKey,
    ) -> crate::resident::task_reservation::ResidentTaskReservationSettlementTransitionFailure {
        self.tasks
            .reservations
            .get_task_release_failure_for_test(key)
    }

    pub(in crate::resident) fn retry_get_task_release_for_test(
        &mut self,
        failure: crate::resident::task_reservation::ResidentTaskReservationSettlementTransitionFailure,
    ) {
        self.tasks
            .reservations
            .retry_get_task_release_for_test(failure);
    }

    pub(in crate::resident) fn outstanding_get_task_work_for_test(&self) -> usize {
        self.tasks.reservations.outstanding()
    }
}

impl<'host, Authority> ResidentOperationService<'host, Authority> {
    pub(in crate::resident) fn active_a2a_send_checkpoint_for_test(
        &self,
        key: &ResidentOperationTaskKey,
    ) -> Option<(ResidentRecordRefV1, ControlStateCheckpointV1)> {
        self.tasks
            .a2a_delegate_tasks
            .active_send_checkpoint_for_test(key)
    }

    pub(in crate::resident) fn a2a_started_association_failure_for_test(
        &mut self,
        key: &ResidentOperationTaskKey,
    ) -> ResidentA2aDelegateLaunchFailure<'static, 'static> {
        self.tasks
            .a2a_delegate_tasks
            .started_association_failure_for_test(key, &mut self.tasks.reservations)
    }

    pub(in crate::resident) fn a2a_offered_completion_failure_for_test(
        &mut self,
        key: &ResidentOperationTaskKey,
    ) -> ResidentA2aDelegateLaunchFailure<'static, 'static> {
        self.tasks
            .a2a_delegate_tasks
            .offered_completion_failure_for_test(key, &mut self.tasks.reservations)
    }

    pub(in crate::resident) fn a2a_rollback_failure_from_not_started_for_test(
        &mut self,
        completion: ResidentA2aDelegateCompletion,
    ) -> (
        ResidentOperationTaskKey,
        ResidentA2aDelegateLaunchFailure<'static, 'static>,
    ) {
        self.tasks
            .a2a_delegate_tasks
            .rollback_failure_from_not_started_for_test(completion, &mut self.tasks.reservations)
    }

    pub(in crate::resident) fn set_a2a_reservation_phase_for_test(
        &mut self,
        key: &ResidentOperationTaskKey,
        phase: crate::resident::task_reservation::ResidentTaskReservationPhase,
    ) {
        self.tasks.reservations.set_phase_for_test(
            key,
            &crate::resident::task_reservation::ResidentTaskReservationRoute::A2aDelegate,
            phase,
        );
    }

    pub(in crate::resident) fn reject_a2a_finalization_with_replacement_for_test(
        &mut self,
        finalization: ResidentA2aDelegateFinalization,
        retained: RetainedA2aOutcome,
    ) -> ResidentA2aDelegateFinalizationRejection {
        let mut replacement = ResidentOperationTaskState::new(
            ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
            NonZeroUsize::new(1).unwrap(),
            ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
        )
        .expect("replacement A2A task owner");
        match ResidentOperationService::new(
            self.authority,
            self.custody,
            self.anchor,
            &mut replacement,
        )
        .finish_a2a_delegate(finalization, retained)
        {
            Ok(_) => panic!("a replacement owner cannot release A2A capacity"),
            Err(rejection) => rejection,
        }
    }

    pub(in crate::resident) fn continue_provider_custody_with_finalization_mismatch_for_test(
        &mut self,
        custody: ResidentProviderRecordingCustody,
        substituted_store: AuthorityStoreCoordinate,
    ) -> Result<ResidentProviderTaskPoll<'static>, ResidentProviderRecordingCustodyRejection> {
        let Self {
            authority,
            custody: operation_custody,
            anchor,
            tasks,
        } = self;
        tasks
            .provider_tasks
            .continue_custody_with_finalization_mismatch_for_test(
                &mut ResidentLocalOperationCoordinator::new(
                    *authority,
                    *operation_custody,
                    *anchor,
                ),
                custody,
                &mut tasks.reservations,
                substituted_store,
            )
    }

    pub(in crate::resident) fn restore_provider_finalization_for_test(
        &mut self,
        finalization: ResidentProviderRecordingFinalization,
    ) -> Result<ResidentProviderTaskPoll<'static>, ResidentProviderRecordingFinalizationRejection>
    {
        self.tasks
            .provider_tasks
            .restore_finalization_for_test(finalization, &mut self.tasks.reservations)
    }

    pub(in crate::resident) fn recover_unresolved_a2a_delegate_with_rejected_loss_for_test<
        'operation,
        'call,
        'a,
    >(
        &mut self,
        unresolved: ResidentA2aDelegateUnresolvedLaunch<'operation, 'call>,
        input: A2aSendRecoveryInput<'a>,
        admission: A2aSendRecoveryAdmission<'_>,
    ) -> ResidentA2aDelegateRecoveredOfferRejection<'a> {
        self.tasks
            .a2a_delegate_tasks
            .recover_unresolved_with_rejected_loss_for_test(
                unresolved,
                input,
                self.custody,
                self.anchor,
                admission,
                &mut self.tasks.reservations,
            )
    }

    fn recover_a2a_delegate_with_rejected_loss_for_test<'a>(
        &mut self,
        input: A2aSendRecoveryInput<'a>,
        admission: A2aSendRecoveryAdmission<'_>,
    ) -> ResidentA2aDelegateRecoveredOfferRejection<'a> {
        self.tasks
            .a2a_delegate_tasks
            .recover_with_rejected_loss_for_test(
                input,
                self.custody,
                self.anchor,
                admission,
                &mut self.tasks.reservations,
            )
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn commit_consultation_terminal_without_release_for_test<
        'operation,
        Work,
        Output,
        AdmitEvidence,
        AdmitCollection,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        ready: ResidentConsultationCompletionReady<Work, Output>,
        input: ConsultationChildJoinInput<'operation>,
        checkpoint: ConsultationSuccessorCommitInput<'_>,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        admit_evidence: AdmitEvidence,
        admit_collection: AdmitCollection,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentConsultationJoinedTerminal<Work, Output>,
        ResidentConsultationTerminalJoinFailure<Work, Output>,
    >
    where
        AdmitEvidence: FnOnce(
            &crate::resident::consultation::ConsultationRosterSlot,
            &ImmutableRecordReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        AdmitCollection: FnOnce(
            &crate::resident::consultation::ConsultationCollection,
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
        ResidentConsultationJoinedTerminal::commit_without_aggregate_release_for_test(
            &self.tasks.consultation_tasks,
            &self.tasks.reservations,
            self.custody,
            self.anchor,
            ready,
            input,
            checkpoint,
            resolve_store,
            admit_evidence,
            admit_collection,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn join_consultation_terminal_with_optional_release_rejection_for_test<
        'operation,
        Work,
        Output,
        AdmitEvidence,
        AdmitCollection,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        &mut self,
        reject_release: bool,
        ready: ResidentConsultationCompletionReady<Work, Output>,
        input: ConsultationChildJoinInput<'operation>,
        checkpoint: ConsultationSuccessorCommitInput<'_>,
        resolve_store: &dyn OperationObservationAuthorityStoreResolver,
        admit_evidence: AdmitEvidence,
        admit_collection: AdmitCollection,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        CompletedResidentConsultationTerminal<Work, Output>,
        ResidentConsultationTerminalJoinFailure<Work, Output>,
    >
    where
        AdmitEvidence: FnOnce(
            &crate::resident::consultation::ConsultationRosterSlot,
            &ImmutableRecordReferenceV1,
        ) -> Result<(), InvalidResidentRecordV1>,
        AdmitCollection: FnOnce(
            &crate::resident::consultation::ConsultationCollection,
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
        if !reject_release {
            return self.join_consultation_terminal(
                ready,
                input,
                checkpoint,
                resolve_store,
                admit_evidence,
                admit_collection,
                admit_rotation,
                sign_observation,
                sign_checkpoint,
            );
        }
        let joined = self.commit_consultation_terminal_without_release_for_test(
            ready,
            input,
            checkpoint,
            resolve_store,
            admit_evidence,
            admit_collection,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )?;
        let mut replacement = ResidentOperationTaskState::new(
            ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
            NonZeroUsize::new(1).unwrap(),
            ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
        )
        .expect("replacement Consult task state");
        retry_consultation_terminal_release(
            &mut replacement.consultation_tasks,
            &mut replacement.reservations,
            joined,
        )
    }
}

impl<Authority> ResidentOperationService<'_, Authority> {
    pub(in crate::resident) fn consultation_batch_mut(
        &mut self,
        coordinate: &ResidentConsultationBatchCoordinate,
    ) -> Option<&mut ResidentConsultationTaskService> {
        self.tasks.consultation_batch_mut(coordinate)
    }

    pub(in crate::resident) fn accept_retained_consultation_for_test<Work, Output, Executor>(
        &mut self,
        coordinate: &ResidentConsultationBatchCoordinate,
        retained: RetainedConsultationTask<Work>,
        executor: &mut Executor,
    ) -> Option<Result<ResidentOperationTaskKey, ResidentConsultationTaskRefusal<Work>>>
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        self.tasks
            .accept_retained_consultation_for_test(coordinate, retained, executor)
    }

    pub(in crate::resident) fn resolve_consultation_continuation(
        &mut self,
        continuation: ResidentConsultationContinuationToken,
    ) -> ResidentConsultationTaskOwnerResolution {
        self.tasks.resolve_consultation_continuation(continuation)
    }

    pub(in crate::resident) fn consultation_coordinator(
        &mut self,
    ) -> ResidentConsultationCoordinator<'_> {
        ResidentConsultationCoordinator::new(self.custody, self.anchor)
    }
}

struct A2aTaskOwnerClock {
    values: std::sync::Mutex<VecDeque<Result<UnixTimeV1, WallClockSourceError>>>,
    calls: AtomicUsize,
}

impl A2aTaskOwnerClock {
    fn new(values: impl IntoIterator<Item = Result<u64, WallClockSourceError>>) -> Self {
        Self {
            values: std::sync::Mutex::new(
                values
                    .into_iter()
                    .map(|value| value.map(|value| UnixTimeV1::new(value).unwrap()))
                    .collect(),
            ),
            calls: AtomicUsize::new(0),
        }
    }
}

impl WallClockSourcePort for A2aTaskOwnerClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.values
            .lock()
            .unwrap()
            .pop_front()
            .expect("one modeled A2A task-owner time")
    }
}

struct A2aTaskOwnerExchange {
    coordinate: A2aExchangeCoordinate,
    response: Option<A2aExchangeEvidence>,
    pending: bool,
}

impl A2aExchange for A2aTaskOwnerExchange {
    fn coordinate(&self) -> &A2aExchangeCoordinate {
        &self.coordinate
    }

    fn poll_next(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<A2aExchangeEvidence, A2aExchangeError>>> {
        if self.pending {
            return Poll::Pending;
        }
        match self.response.take() {
            Some(response) => Poll::Ready(Some(Ok(response))),
            None => Poll::Ready(None),
        }
    }

    fn cancel(&mut self) {}
}

#[derive(Clone, Default)]
enum A2aTaskOwnerExchangeMode {
    #[default]
    Pending,
    Response(A2aExchangeFacts),
    Reject(A2aExchangeError),
}

#[derive(Default)]
struct A2aTaskOwnerExchangePort {
    preparations: Cell<usize>,
    starts: Cell<usize>,
    mode: A2aTaskOwnerExchangeMode,
}

impl A2aTaskOwnerExchangePort {
    fn response(facts: A2aExchangeFacts) -> Self {
        Self {
            preparations: Cell::new(0),
            starts: Cell::new(0),
            mode: A2aTaskOwnerExchangeMode::Response(facts),
        }
    }

    fn reject(reason: A2aExchangeError) -> Self {
        Self {
            preparations: Cell::new(0),
            starts: Cell::new(0),
            mode: A2aTaskOwnerExchangeMode::Reject(reason),
        }
    }
}

impl A2aExchangePort for A2aTaskOwnerExchangePort {
    fn prepare(
        &self,
        request: A2aExchangeRequest,
    ) -> Result<A2aPreparedExchange, A2aExchangeError> {
        self.preparations.set(self.preparations.get() + 1);
        A2aPreparedExchange::from_encoded(
            request,
            br#"{"jsonrpc":"2.0","id":"rpc-1","method":"message/send"}"#.to_vec(),
            ArtifactDigest::hash_bytes(b"task-owner TLS trust"),
        )
    }

    fn start(
        &self,
        prepared: A2aPreparedExchange,
    ) -> Result<Box<dyn A2aExchange>, A2aExchangeError> {
        self.starts.set(self.starts.get() + 1);
        if let A2aTaskOwnerExchangeMode::Reject(reason) = self.mode {
            return Err(reason);
        }
        let response = match &self.mode {
            A2aTaskOwnerExchangeMode::Pending => None,
            A2aTaskOwnerExchangeMode::Response(facts) => Some(A2aExchangeEvidence {
                facts: facts.clone(),
                bytes: br#"{"jsonrpc":"2.0","id":"rpc-1","result":{}}"#.to_vec(),
                sse: None,
            }),
            A2aTaskOwnerExchangeMode::Reject(_) => unreachable!("handled before exchange creation"),
        };
        Ok(Box::new(A2aTaskOwnerExchange {
            coordinate: prepared.coordinate().clone(),
            response,
            pending: matches!(self.mode, A2aTaskOwnerExchangeMode::Pending),
        }))
    }
}

struct NoA2aProtocol;

impl A2aSendMessageEvidencePort for NoA2aProtocol {
    fn read_initial_response(
        &self,
        _: &[u8],
        _: &[u8],
        _: &A2aJsonRpcRequestIdV1,
        _: NonZeroUsize,
    ) -> Result<pale_ports::A2aExchangeFacts, A2aExchangeError> {
        panic!("a pending or cancelled task-owner exchange has no response to decode")
    }
}

struct ExactA2aProtocol(A2aExchangeFacts);

impl A2aSendMessageEvidencePort for ExactA2aProtocol {
    fn read_initial_response(
        &self,
        _: &[u8],
        _: &[u8],
        _: &A2aJsonRpcRequestIdV1,
        _: NonZeroUsize,
    ) -> Result<pale_ports::A2aExchangeFacts, A2aExchangeError> {
        Ok(self.0.clone())
    }
}

#[derive(Clone, Copy)]
enum A2aUnopenedFault {
    ReleaseClock,
    ReservationSubstitutions,
    DroppedUnopened,
    DiagnosticRecovery,
    DiagnosticStillInadmissible,
    Preflight,
    AnchorContinuation,
    AnchorLaterSuccessor,
    AnchorLaterPredecessor,
    AnchorLaterUnlike,
    AnchorLaterUnavailable,
    CommitRecovery,
    CommitRecoveryLossRetry,
    GenericRecoveryLossRetry,
    Duplicate,
    Saturated,
}

fn retry_substituted_unopened<'operation, 'call, Authority>(
    service: &mut ResidentOperationService<'_, Authority>,
    unopened: Box<
        super::super::a2a_delegate_task_service::ResidentA2aDelegateUnopenedLaunch<
            'operation,
            'call,
        >,
    >,
    exchange: &dyn A2aExchangePort,
    clock: &dyn WallClockSourcePort,
) -> (
    Box<
        super::super::a2a_delegate_task_service::ResidentA2aDelegateUnopenedLaunch<
            'operation,
            'call,
        >,
    >,
    super::super::task_reservation::ResidentTaskReservationTransitionFailure,
)
where
    Authority: AmbientSessionAuthorityPort,
{
    match service.retry_unopened_a2a_delegate(*unopened, exchange, clock, |_, _, _| Ok(())) {
        Err(ResidentA2aDelegateUnopenedRetryFailure::Reservation { unopened, reason }) => {
            (unopened, reason)
        }
        Ok(_) => panic!("substituted aggregate state cannot start SendMessage"),
        Err(_) => panic!("aggregate substitution must fail before A2A retry I/O"),
    }
}

fn a2a_unopened_retry_cancel_and_finish_case(fault: A2aUnopenedFault) {
    with_retained_delegate_dispatch(
        A2aTaskOwnerCustodyModel {
            clock_times: vec![63, 64, 65],
            expected_preparations: if matches!(
                fault,
                A2aUnopenedFault::Duplicate | A2aUnopenedFault::Saturated
            ) {
                4
            } else if matches!(fault, A2aUnopenedFault::DroppedUnopened) {
                5
            } else {
                6
            },
            expected_commits: if matches!(
                fault,
                A2aUnopenedFault::Duplicate | A2aUnopenedFault::Saturated
            ) {
                4
            } else if matches!(fault, A2aUnopenedFault::DroppedUnopened) {
                5
            } else {
                6
            },
            expected_read_backs: if matches!(
                fault,
                A2aUnopenedFault::Duplicate | A2aUnopenedFault::Saturated
            ) {
                4
            } else if matches!(fault, A2aUnopenedFault::DroppedUnopened) {
                5
            } else if matches!(
                fault,
                A2aUnopenedFault::CommitRecovery
                    | A2aUnopenedFault::CommitRecoveryLossRetry
                    | A2aUnopenedFault::GenericRecoveryLossRetry
            ) {
                7
            } else {
                6
            },
            expected_anchor_advances: if matches!(
                fault,
                A2aUnopenedFault::AnchorContinuation | A2aUnopenedFault::AnchorLaterPredecessor
            ) {
                7
            } else if matches!(
                fault,
                A2aUnopenedFault::Duplicate | A2aUnopenedFault::Saturated
            ) {
                4
            } else if matches!(fault, A2aUnopenedFault::DroppedUnopened) {
                5
            } else {
                6
            },
            expected_anchor_observations: if matches!(
                fault,
                A2aUnopenedFault::Duplicate | A2aUnopenedFault::Saturated
            ) {
                4
            } else if matches!(fault, A2aUnopenedFault::DroppedUnopened) {
                5
            } else if matches!(
                fault,
                A2aUnopenedFault::AnchorLaterSuccessor | A2aUnopenedFault::AnchorLaterPredecessor
            ) {
                8
            } else if matches!(
                fault,
                A2aUnopenedFault::AnchorLaterUnlike | A2aUnopenedFault::AnchorLaterUnavailable
            ) {
                9
            } else {
                6
            },
            unknown_exact_commits: if matches!(
                fault,
                A2aUnopenedFault::CommitRecovery
                    | A2aUnopenedFault::CommitRecoveryLossRetry
                    | A2aUnopenedFault::GenericRecoveryLossRetry
            ) {
                &[5]
            } else {
                &[]
            },
            unavailable_read_backs: &[],
            unavailable_anchor_advances: if matches!(fault, A2aUnopenedFault::AnchorContinuation) {
                &[5]
            } else {
                &[]
            },
            unknown_exact_anchor_advances: &[],
            unknown_failed_observation_anchor_advances: if matches!(
                fault,
                A2aUnopenedFault::AnchorLaterSuccessor
                    | A2aUnopenedFault::AnchorLaterPredecessor
                    | A2aUnopenedFault::AnchorLaterUnlike
                    | A2aUnopenedFault::AnchorLaterUnavailable
            ) {
                &[5]
            } else {
                &[]
            },
            later_anchor_observations: match fault {
                A2aUnopenedFault::AnchorLaterSuccessor => {
                    vec![custody::ServiceAnchorLaterObservation::ExactSuccessor]
                }
                A2aUnopenedFault::AnchorLaterPredecessor => {
                    vec![custody::ServiceAnchorLaterObservation::ExactPredecessor]
                }
                A2aUnopenedFault::AnchorLaterUnlike => vec![
                    custody::ServiceAnchorLaterObservation::Missing,
                    custody::ServiceAnchorLaterObservation::ExactSuccessor,
                ],
                A2aUnopenedFault::AnchorLaterUnavailable => vec![
                    custody::ServiceAnchorLaterObservation::Unavailable,
                    custody::ServiceAnchorLaterObservation::ExactSuccessor,
                ],
                _ => Vec::new(),
            },
            fail_a2a_preflight_once: matches!(fault, A2aUnopenedFault::Preflight),
        },
        |retained,
         current,
         signing,
         checkpoint_schema,
         checkpoint_authority,
         _provider_clock,
         resolve_store,
         authority,
         custody,
         anchor,
         _parents,
         _parent_request,
         _restart_generation,
         _initial_candidate| {
            let provider = retained
                .provider_request()
                .unwrap()
                .provider_binding()
                .clone();
            let route = a2a_exchange::route_for_handle_at_with_authority(
                &provider,
                retained.operation().handle().1,
                "https://agent.example.test/a2a",
                false,
                10,
                100,
                110,
                signing.signing_authority,
            );
            let route_ref = record_ref("a2a-task-owner/route");
            let request_id = A2aJsonRpcRequestIdV1::parse("rpc-1").unwrap();
            let clock = match fault {
                A2aUnopenedFault::ReleaseClock
                | A2aUnopenedFault::ReservationSubstitutions
                | A2aUnopenedFault::DroppedUnopened
                | A2aUnopenedFault::DiagnosticRecovery
                | A2aUnopenedFault::DiagnosticStillInadmissible => A2aTaskOwnerClock::new([
                    Ok(66),
                    Err(WallClockSourceError::from_source(std::io::Error::other(
                        "release clock unavailable",
                    ))),
                    Ok(67),
                ]),
                A2aUnopenedFault::Preflight => A2aTaskOwnerClock::new([Ok(66), Ok(67), Ok(68)]),
                A2aUnopenedFault::AnchorContinuation
                | A2aUnopenedFault::AnchorLaterSuccessor
                | A2aUnopenedFault::AnchorLaterPredecessor
                | A2aUnopenedFault::AnchorLaterUnlike
                | A2aUnopenedFault::AnchorLaterUnavailable
                | A2aUnopenedFault::CommitRecovery
                | A2aUnopenedFault::CommitRecoveryLossRetry
                | A2aUnopenedFault::GenericRecoveryLossRetry
                | A2aUnopenedFault::Duplicate
                | A2aUnopenedFault::Saturated => A2aTaskOwnerClock::new([Ok(66), Ok(67), Ok(68)]),
            };
            let exchange = A2aTaskOwnerExchangePort::default();
            let send_checkpoint_signs = Cell::new(0_usize);
            let mut tasks = ResidentOperationTaskState::new(
                ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                NonZeroUsize::new(1).unwrap(),
                ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
            )
            .unwrap();
            let launch = A2aDelegateLaunchInput {
                send: A2aSendInput {
                    dispatch: A2aSendDispatch::fresh(retained),
                    current,
                    route: (&route_ref, &route),
                    route_predecessor: pale_ports::OperationCustodyIndexPredecessor::Absent,
                    route_association: A2aRouteAssociation::Insert,
                    request_id: &request_id,
                    message_id: "message-1",
                    text: "Inspect the selected workspace.",
                    request_evidence_ref: record_ref("a2a-task-owner/request"),
                    preparation_ref: record_ref("a2a-task-owner/preparation"),
                    exchange_limits: a2a_exchange::limits(),
                    maximum_evidence_bytes: 8_192,
                    maximum_record_bytes: 65_536,
                    maximum_dispatch_record_bytes: 65_536,
                    resolve_store,
                },
                checkpoint: OperationCheckpointInput {
                    reference: record_ref("a2a-task-owner/send-checkpoint"),
                    id: ControlCheckpointIdV1::parse("a2a-task-owner-send-checkpoint").unwrap(),
                },
                checkpoint_schema,
                checkpoint_authority,
                response_ref: record_ref("a2a-task-owner/response"),
                clock: &clock,
                accumulator_limits: AccumulatorPreparationLimits::new(
                    8, 20_000, 20_000, 8_192, 200_000,
                )
                .unwrap(),
                membership_limits: pale_ports::AccumulatorMembershipLimits::new(8, 20_000, 200_000)
                    .unwrap(),
            };
            let exact_key = launch.send.dispatch.task_key().unwrap();
            if matches!(
                fault,
                A2aUnopenedFault::Duplicate | A2aUnopenedFault::Saturated
            ) {
                if matches!(fault, A2aUnopenedFault::Duplicate) {
                    tasks.accept_a2a_reservation_for_test(exact_key.clone());
                } else {
                    tasks.accept_provider_reservation_for_test(
                        super::super::task_owner::ResidentOperationTaskKey::new(
                            store_coordinate(),
                            ImmutableRecordReferenceV1 {
                                record_ref: record_ref("a2a-task-owner/saturated"),
                                record_sha256: artifact_digest('9'),
                            },
                        ),
                    );
                }
            }
            let mut service = ResidentOperationService::new(authority, custody, anchor, &mut tasks);
            let launch_result = service.launch_a2a_delegate(
                launch,
                &exchange,
                |_, _, _| Ok(()),
                |_, _, _| Ok(()),
                |_, _, _| {
                    send_checkpoint_signs.set(send_checkpoint_signs.get() + 1);
                    Ok(signature())
                },
            );
            if matches!(
                fault,
                A2aUnopenedFault::Duplicate | A2aUnopenedFault::Saturated
            ) {
                let reason = match launch_result {
                    Err(ResidentA2aDelegateLaunchFailure::Reservation { reason, .. }) => reason,
                    _ => panic!("shared capacity must reject before A2A preparation"),
                };
                assert!(matches!(
                    (fault, reason),
                    (
                        A2aUnopenedFault::Duplicate,
                        super::super::task_reservation::ResidentTaskReservationOfferFailure::Duplicate
                    ) | (
                        A2aUnopenedFault::Saturated,
                        super::super::task_reservation::ResidentTaskReservationOfferFailure::Saturated
                    )
                ));
                assert_eq!(service.outstanding_task_reservations(), 1);
                assert_eq!(exchange.preparations.get(), 0);
                assert_eq!(exchange.starts.get(), 0);
                assert_eq!(clock.calls.load(Ordering::SeqCst), 0);
                assert_eq!(send_checkpoint_signs.get(), 0);
                return;
            }
            assert_eq!(send_checkpoint_signs.get(), 1);
            let failure = match launch_result {
                Ok(_) => panic!("the second release-clock sample is unavailable"),
                Err(failure) => failure,
            };
            let mut unopened = match failure {
                ResidentA2aDelegateLaunchFailure::Unopened(unopened) => {
                    assert!(!matches!(
                        fault,
                        A2aUnopenedFault::AnchorContinuation
                            | A2aUnopenedFault::AnchorLaterSuccessor
                            | A2aUnopenedFault::AnchorLaterPredecessor
                            | A2aUnopenedFault::AnchorLaterUnlike
                            | A2aUnopenedFault::AnchorLaterUnavailable
                    ));
                    assert_eq!(service.outstanding_task_reservations(), 1);
                    unopened
                }
                ResidentA2aDelegateLaunchFailure::Association { .. } => {
                    panic!("unexpected A2A launch association failure")
                }
                ResidentA2aDelegateLaunchFailure::Reservation { .. } => {
                    panic!("unexpected A2A launch reservation failure")
                }
                ResidentA2aDelegateLaunchFailure::Launch(launch) => match *launch {
                    crate::resident::A2aDelegateLaunchFailure::Preparation(reason) => {
                        panic!("unexpected A2A preparation failure: {reason:?}")
                    }
                    other => panic!("unexpected pre-custody A2A launch failure: {other:?}"),
                },
                ResidentA2aDelegateLaunchFailure::Rollback { .. } => {
                    panic!("unexpected A2A launch rollback failure")
                }
                ResidentA2aDelegateLaunchFailure::Custody(unresolved) => {
                    assert!(matches!(
                        fault,
                        A2aUnopenedFault::AnchorContinuation
                            | A2aUnopenedFault::AnchorLaterSuccessor
                            | A2aUnopenedFault::AnchorLaterPredecessor
                            | A2aUnopenedFault::AnchorLaterUnlike
                            | A2aUnopenedFault::AnchorLaterUnavailable
                            | A2aUnopenedFault::CommitRecovery
                            | A2aUnopenedFault::CommitRecoveryLossRetry
                            | A2aUnopenedFault::GenericRecoveryLossRetry
                    ));
                    assert_eq!(service.outstanding_task_reservations(), 1);
                    let mut replacement_tasks = ResidentOperationTaskState::new(
                        ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                        NonZeroUsize::new(1).unwrap(),
                        ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                    )
                    .unwrap();
                    let mut replacement = ResidentOperationService::new(
                        authority,
                        custody,
                        anchor,
                        &mut replacement_tasks,
                    );
                    let rejected = match replacement.continue_a2a_delegate_custody(*unresolved) {
                        Ok(_) => panic!("a replacement owner cannot continue exact anchor custody"),
                        Err(rejected) => rejected,
                    };
                    assert!(rejected.reservation.is_some());
                    assert_eq!(exchange.starts.get(), 0);
                    let mut service =
                        ResidentOperationService::new(authority, custody, anchor, &mut tasks);
                    let continued = service.continue_a2a_delegate_custody(*rejected.unresolved);
                    let continued = if matches!(
                        fault,
                        A2aUnopenedFault::AnchorLaterUnlike
                            | A2aUnopenedFault::AnchorLaterUnavailable
                    ) {
                        let unresolved = match continued {
                            Ok(_) => panic!("unlike later anchor evidence cannot complete custody"),
                            Err(rejected) => {
                                assert!(rejected.reservation.is_none());
                                rejected.unresolved
                            }
                        };
                        assert_eq!(exchange.starts.get(), 0);
                        service.continue_a2a_delegate_custody(*unresolved)
                    } else {
                        continued
                    };
                    if matches!(
                        fault,
                        A2aUnopenedFault::CommitRecovery
                            | A2aUnopenedFault::CommitRecoveryLossRetry
                            | A2aUnopenedFault::GenericRecoveryLossRetry
                    ) {
                        let unresolved = match continued {
                            Ok(_) => {
                                panic!("an ambiguous recovered commit cannot regain SendMessage")
                            }
                            Err(rejected) => {
                                assert!(rejected.reservation.is_none());
                                rejected.unresolved
                            }
                        };
                        let fixture = unresolved
                            .recovery_fixture()
                            .expect("exact read-back must preserve a protected recovery fixture");
                        let unresolved = match service.continue_a2a_delegate_custody(*unresolved) {
                            Ok(_) => panic!("recovery-only custody cannot become an unopened send"),
                            Err(rejected) => {
                                assert!(rejected.reservation.is_none());
                                rejected.unresolved
                            }
                        };
                        assert_eq!(exchange.starts.get(), 0);
                        let recovered_observation = fixture.recovered();
                        let recovery_limits = A2aSendRecoveryLimits::new(
                            65_536,
                            8_192,
                            OperationCheckpointAncestryLimits::new(16, 1 << 20, 16).unwrap(),
                            pale_ports::AccumulatorMembershipLimits::new(32, 100_000, 1_000_000)
                                .unwrap(),
                        )
                        .unwrap();
                        let mut replacement = ResidentOperationService::new(
                            authority,
                            custody,
                            anchor,
                            &mut replacement_tasks,
                        );
                        let unresolved = match replacement.recover_unresolved_a2a_delegate(
                            *unresolved,
                            A2aSendRecoveryInput {
                                history: fixture.history(),
                                recovered: &recovered_observation,
                                limits: recovery_limits,
                            },
                            A2aSendRecoveryAdmission {
                                admit_checkpoint: &mut |_, _| {
                                    panic!("replacement owner must be rejected before checkpoint admission")
                                },
                                admit_route: &mut |_, _| {
                                    panic!("replacement owner must be rejected before route admission")
                                },
                            },
                        ) {
                            Ok(_) => panic!("a replacement owner cannot recover the held offer"),
                            Err(super::super::a2a_delegate_task_service::ResidentA2aDelegateUnresolvedRecoveryFailure::Reservation { unresolved, .. }) => unresolved,
                            Err(_) => panic!("replacement rejection must precede protected recovery"),
                        };
                        assert_eq!(exchange.starts.get(), 0);
                        let mut admit_checkpoint =
                            |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                                ControlStateCheckpointV1::admit_signed_wire(
                                    wire,
                                    checkpoint_schema,
                                    checkpoint_authority,
                                    |_, _, _, actual| {
                                        assert_eq!(actual, &signature());
                                        Ok(())
                                    },
                                )
                            };
                        let mut admit_route =
                            |reference: &ResidentRecordRefV1, wire: A2aRouteAdmissionWireV1| {
                                assert_eq!(reference, &route_ref);
                                A2aRouteAdmissionV1::admit_contextual_wire(
                                    wire,
                                    &A2aRouteAdmissionContextV1 {
                                        signed: SignedResidentAdmissionContextV1 {
                                            expected_schema_sha256: route.schema_sha256(),
                                            signing_authority: signing.signing_authority,
                                        },
                                        provider_binding: route.provider_binding(),
                                        route: route.route(),
                                    },
                                    |_| Ok(()),
                                    |_, _, _, actual| {
                                        assert_eq!(actual, &signature());
                                        Ok(())
                                    },
                                )
                            };
                        let generic_recovery =
                            matches!(fault, A2aUnopenedFault::GenericRecoveryLossRetry);
                        let mut restarted_tasks = ResidentOperationTaskState::new(
                            ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                            NonZeroUsize::new(1).unwrap(),
                            ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                        )
                        .unwrap();
                        let recovered = if generic_recovery {
                            let mut restarted = ResidentOperationService::new(
                                authority,
                                custody,
                                anchor,
                                &mut restarted_tasks,
                            );
                            let rejected = restarted
                                .recover_a2a_delegate_with_rejected_loss_for_test(
                                    A2aSendRecoveryInput {
                                        history: fixture.history(),
                                        recovered: &recovered_observation,
                                        limits: recovery_limits,
                                    },
                                    A2aSendRecoveryAdmission {
                                        admit_checkpoint: &mut admit_checkpoint,
                                        admit_route: &mut admit_route,
                                    },
                                );
                            assert_eq!(restarted.outstanding_task_reservations(), 1);
                            restarted_tasks.reservations.set_phase_for_test(
                                &exact_key,
                                &crate::resident::task_reservation::ResidentTaskReservationRoute::A2aDelegate,
                                crate::resident::task_reservation::ResidentTaskReservationPhase::Offered,
                            );
                            let mut retry = ResidentOperationService::new(
                                authority,
                                custody,
                                anchor,
                                &mut restarted_tasks,
                            );
                            retry
                                .retry_recovered_a2a_delegate_loss(rejected)
                                .unwrap_or_else(|_| {
                                    panic!(
                                        "generic recovery retries only its rejected aggregate loss"
                                    )
                                })
                        } else {
                            let mut service = ResidentOperationService::new(
                                authority, custody, anchor, &mut tasks,
                            );
                            if matches!(fault, A2aUnopenedFault::CommitRecoveryLossRetry) {
                                let rejected = service
                                    .recover_unresolved_a2a_delegate_with_rejected_loss_for_test(
                                        *unresolved,
                                        A2aSendRecoveryInput {
                                            history: fixture.history(),
                                            recovered: &recovered_observation,
                                            limits: recovery_limits,
                                        },
                                        A2aSendRecoveryAdmission {
                                            admit_checkpoint: &mut admit_checkpoint,
                                            admit_route: &mut admit_route,
                                        },
                                    );
                                tasks.reservations.set_phase_for_test(
                                    &exact_key,
                                    &super::super::task_reservation::ResidentTaskReservationRoute::A2aDelegate,
                                    super::super::task_reservation::ResidentTaskReservationPhase::Offered,
                                );
                                let mut retry_service = ResidentOperationService::new(
                                    authority, custody, anchor, &mut tasks,
                                );
                                retry_service
                                    .retry_recovered_a2a_delegate_loss(rejected)
                                    .unwrap_or_else(|_| {
                                        panic!("the exact recovered loss token retries only its aggregate transition")
                                    })
                            } else {
                                match service.recover_unresolved_a2a_delegate(
                                *unresolved,
                                A2aSendRecoveryInput {
                                    history: fixture.history(),
                                    recovered: &recovered_observation,
                                    limits: recovery_limits,
                                },
                                A2aSendRecoveryAdmission {
                                    admit_checkpoint: &mut admit_checkpoint,
                                    admit_route: &mut admit_route,
                                },
                            ) {
                            Ok(recovered) => recovered,
                            Err(super::super::a2a_delegate_task_service::ResidentA2aDelegateUnresolvedRecoveryFailure::Reservation { reason, .. }) => panic!("exact recovery reservation: {reason:?}"),
                            Err(super::super::a2a_delegate_task_service::ResidentA2aDelegateUnresolvedRecoveryFailure::Custody(_)) => panic!("exact recovery remained outside recovery-only custody"),
                            Err(super::super::a2a_delegate_task_service::ResidentA2aDelegateUnresolvedRecoveryFailure::Association { reason, .. }) => panic!("exact recovery association: {reason:?}"),
                            Err(super::super::a2a_delegate_task_service::ResidentA2aDelegateUnresolvedRecoveryFailure::Recovery { reason, .. }) => panic!("exact protected A2A recovery: {reason:?}"),
                            Err(super::super::a2a_delegate_task_service::ResidentA2aDelegateUnresolvedRecoveryFailure::LossReservation(_)) => panic!("exact recovered offer did not enter loss custody"),
                            }
                            }
                        };
                        assert_eq!(exchange.starts.get(), 0);
                        let super::super::a2a_delegate_task_service::ResidentRecoveredA2aDelegate {
                            outcome,
                            finalization,
                        } = recovered;
                        let retention_clock = A2aTaskOwnerClock::new([Ok(68), Ok(69)]);
                        let observation_signs = Cell::new(0_usize);
                        let outcome_checkpoint_signs = Cell::new(0_usize);
                        let retained = retain_a2a_outcome(
                            A2aOutcomeRetentionInput::RecoveredOutcomeUnknown(
                                A2aRecoveredUnknownOutcomeRetention::new(
                                    outcome,
                                    A2aOutcomeUnknownObservationInput {
                                        diagnostic_ref: record_ref("a2a-task-owner/recovered-diagnostic"),
                                        observation_ref: record_ref("a2a-task-owner/recovered-unknown"),
                                        reconciliation: ReconciliationFrontierReferenceV1 {
                                            record_ref: record_ref("a2a-task-owner/recovered-reconciliation"),
                                            record_sha256: ReconciliationFrontierDigest::parse(
                                                "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
                                            )
                                            .unwrap(),
                                        },
                                        clock: &retention_clock,
                                        signing,
                                        maximum_diagnostic_bytes: 65_536,
                                    },
                                    A2aSendObservationCommitInput {
                                        checkpoint: OperationCheckpointInput {
                                            reference: record_ref("a2a-task-owner/recovered-checkpoint"),
                                            id: ControlCheckpointIdV1::parse(
                                                "a2a-task-owner-recovered-checkpoint",
                                            )
                                            .unwrap(),
                                        },
                                        clock: &retention_clock,
                                        checkpoint_schema,
                                        checkpoint_authority,
                                        accumulator_limits: AccumulatorPreparationLimits::new(
                                            8, 20_000, 20_000, 8_192, 200_000,
                                        )
                                        .unwrap(),
                                    },
                                    resolve_store,
                                    |_| Ok(()),
                                    |_, _, _, _| Ok(()),
                                    |_, _, _| {
                                        observation_signs.set(observation_signs.get() + 1);
                                        Ok(signature())
                                    },
                                    |_, _, _| {
                                        outcome_checkpoint_signs
                                            .set(outcome_checkpoint_signs.get() + 1);
                                        Ok(signature())
                                    },
                                ),
                            ),
                            custody,
                            anchor,
                        )
                        .expect("recovered uncertainty custody");
                        assert_eq!(observation_signs.get(), 1);
                        assert_eq!(outcome_checkpoint_signs.get(), 1);
                        let finishing_tasks = if generic_recovery {
                            &mut restarted_tasks
                        } else {
                            &mut tasks
                        };
                        let mut service = ResidentOperationService::new(
                            authority,
                            custody,
                            anchor,
                            finishing_tasks,
                        );
                        let _retained = service
                            .finish_a2a_delegate(finalization, retained)
                            .unwrap_or_else(|_| {
                                panic!("exact recovered finalization releases capacity")
                            });
                        assert_eq!(service.outstanding_task_reservations(), 0);
                        assert_eq!(exchange.preparations.get(), 1);
                        assert_eq!(exchange.starts.get(), 0);
                        return;
                    }
                    let unopened = match continued {
                        Ok(unopened) => unopened,
                        Err(_) => panic!("the exact owner continues the unresolved anchor"),
                    };
                    assert_eq!(service.outstanding_task_reservations(), 1);
                    assert_eq!(exchange.starts.get(), 0);
                    Box::new(unopened)
                }
                ResidentA2aDelegateLaunchFailure::StartedAssociation { .. } => {
                    panic!("unexpected started A2A association failure")
                }
                ResidentA2aDelegateLaunchFailure::OfferSettlement(_) => {
                    panic!("unexpected A2A offer settlement failure")
                }
            };
            assert_eq!(exchange.starts.get(), 0);

            if matches!(fault, A2aUnopenedFault::DroppedUnopened) {
                drop(unopened);
                assert_eq!(tasks.outstanding_task_reservations(), 1);
                assert_eq!(exchange.preparations.get(), 1);
                assert_eq!(exchange.starts.get(), 0);
                assert_eq!(clock.calls.load(Ordering::SeqCst), 2);
                assert_eq!(send_checkpoint_signs.get(), 1);
                return;
            }

            if matches!(fault, A2aUnopenedFault::ReservationSubstitutions) {
                let expected_preparations = exchange.preparations.get();
                let expected_starts = exchange.starts.get();
                let expected_clock_calls = clock.calls.load(Ordering::SeqCst);

                let substituted_store_key = ResidentOperationTaskKey::new(
                    AuthorityStoreCoordinate::new(
                        ControlStoreIdV1::parse("control-store").unwrap(),
                        ControlIncarnationIdV1::parse("incarnation-substituted").unwrap(),
                    ),
                    exact_key.dispatch().clone(),
                );
                tasks
                    .reservations
                    .replace_key_for_test(&exact_key, substituted_store_key.clone());
                let mut service =
                    ResidentOperationService::new(authority, custody, anchor, &mut tasks);
                let (returned, reason) =
                    retry_substituted_unopened(&mut service, unopened, &exchange, &clock);
                assert_eq!(
                    reason,
                    super::super::task_reservation::ResidentTaskReservationTransitionFailure::NotFound
                );
                assert_eq!(service.outstanding_task_reservations(), 1);
                tasks
                    .reservations
                    .replace_key_for_test(&substituted_store_key, exact_key.clone());
                unopened = returned;

                let substituted_key = ResidentOperationTaskKey::new(
                    exact_key.store().clone(),
                    ImmutableRecordReferenceV1 {
                        record_ref: record_ref("a2a-task-owner/substituted-dispatch"),
                        record_sha256: artifact_digest('7'),
                    },
                );
                tasks
                    .reservations
                    .replace_key_for_test(&exact_key, substituted_key.clone());
                let mut service =
                    ResidentOperationService::new(authority, custody, anchor, &mut tasks);
                let (returned, reason) =
                    retry_substituted_unopened(&mut service, unopened, &exchange, &clock);
                assert_eq!(
                    reason,
                    super::super::task_reservation::ResidentTaskReservationTransitionFailure::NotFound
                );
                assert_eq!(service.outstanding_task_reservations(), 1);
                tasks
                    .reservations
                    .replace_key_for_test(&substituted_key, exact_key.clone());
                unopened = returned;

                tasks.reservations.replace_route_for_test(
                    &exact_key,
                    super::super::task_reservation::ResidentTaskReservationRoute::Provider,
                );
                let mut service =
                    ResidentOperationService::new(authority, custody, anchor, &mut tasks);
                let (returned, reason) =
                    retry_substituted_unopened(&mut service, unopened, &exchange, &clock);
                assert_eq!(
                    reason,
                    super::super::task_reservation::ResidentTaskReservationTransitionFailure::RouteMismatch
                );
                assert_eq!(service.outstanding_task_reservations(), 1);
                tasks.reservations.replace_route_for_test(
                    &exact_key,
                    super::super::task_reservation::ResidentTaskReservationRoute::A2aDelegate,
                );
                unopened = returned;

                tasks.reservations.set_phase_for_test(
                    &exact_key,
                    &super::super::task_reservation::ResidentTaskReservationRoute::A2aDelegate,
                    super::super::task_reservation::ResidentTaskReservationPhase::Accepted,
                );
                let mut service =
                    ResidentOperationService::new(authority, custody, anchor, &mut tasks);
                let (returned, reason) =
                    retry_substituted_unopened(&mut service, unopened, &exchange, &clock);
                assert!(matches!(
                    reason,
                    super::super::task_reservation::ResidentTaskReservationTransitionFailure::Phase {
                        expected: super::super::task_reservation::ResidentTaskReservationPhase::Offered,
                        actual: super::super::task_reservation::ResidentTaskReservationPhase::Accepted,
                    }
                ));
                assert_eq!(service.outstanding_task_reservations(), 1);
                tasks.reservations.set_phase_for_test(
                    &exact_key,
                    &super::super::task_reservation::ResidentTaskReservationRoute::A2aDelegate,
                    super::super::task_reservation::ResidentTaskReservationPhase::Offered,
                );
                unopened = returned;

                assert_eq!(exchange.preparations.get(), expected_preparations);
                assert_eq!(exchange.starts.get(), expected_starts);
                assert_eq!(clock.calls.load(Ordering::SeqCst), expected_clock_calls);
            }

            let mut replacement_tasks = ResidentOperationTaskState::new(
                ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                NonZeroUsize::new(1).unwrap(),
                ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
            )
            .unwrap();
            let mut replacement =
                ResidentOperationService::new(authority, custody, anchor, &mut replacement_tasks);
            let failure = match replacement.retry_unopened_a2a_delegate(
                *unopened,
                &exchange,
                &clock,
                |_, _, _| Ok(()),
            ) {
                Ok(_) => panic!("a replacement owner cannot consume the original offer"),
                Err(failure) => failure,
            };
            let ResidentA2aDelegateUnopenedRetryFailure::Reservation { unopened, .. } = failure
            else {
                panic!("replacement-owner rejection must retain the unopened token")
            };
            assert_eq!(exchange.starts.get(), 0);
            assert_eq!(
                clock.calls.load(Ordering::SeqCst),
                if matches!(
                    fault,
                    A2aUnopenedFault::AnchorContinuation
                        | A2aUnopenedFault::AnchorLaterSuccessor
                        | A2aUnopenedFault::AnchorLaterPredecessor
                        | A2aUnopenedFault::AnchorLaterUnlike
                        | A2aUnopenedFault::AnchorLaterUnavailable
                ) {
                    1
                } else {
                    2
                }
            );

            let mut service = ResidentOperationService::new(authority, custody, anchor, &mut tasks);
            let launch = match service.retry_unopened_a2a_delegate(
                *unopened,
                &exchange,
                &clock,
                |_, _, _| Ok(()),
            ) {
                Ok(launch) => launch,
                Err(_) => panic!("the exact owner may retry while the carrier is still unopened"),
            };
            let ResidentA2aDelegateLaunch::Awaiting(key) = launch else {
                panic!("the exact unopened retry must start one exchange")
            };
            assert_eq!(exchange.starts.get(), 1);
            assert_eq!(service.in_flight_a2a_delegate_tasks(), 1);
            let waker = std::task::Waker::noop();
            let mut context = std::task::Context::from_waker(waker);
            assert!(matches!(
                service.poll_a2a_delegate(&key, &mut context, &NoA2aProtocol, &clock),
                ResidentA2aDelegatePoll::Pending
            ));
            assert_eq!(service.in_flight_a2a_delegate_tasks(), 1);
            let completion = service.cancel_a2a_delegate_reads(&key);
            let ResidentA2aDelegatePoll::Completed(
                super::super::a2a_delegate_task_service::ResidentA2aDelegateCompletion::OutcomeUnknown {
                    outcome,
                    finalization,
                },
            ) = completion
            else {
                panic!("local cancellation must retain explicit outcome uncertainty")
            };
            assert_eq!(service.in_flight_a2a_delegate_tasks(), 0);
            assert_eq!(service.outstanding_task_reservations(), 1);

            let (outcome, finalization) = if matches!(
                fault,
                A2aUnopenedFault::DiagnosticRecovery
                    | A2aUnopenedFault::DiagnosticStillInadmissible
            ) {
                let persistent = matches!(fault, A2aUnopenedFault::DiagnosticStillInadmissible);
                let diagnostic = if persistent {
                    super::super::a2a_delegate_task_service::ResidentA2aDelegateDiagnostic::persistent_fixture_from_outcome_unknown(
                        outcome,
                        finalization,
                    )
                } else {
                    super::super::a2a_delegate_task_service::ResidentA2aDelegateDiagnostic::fixture_from_outcome_unknown(
                        outcome,
                        finalization,
                    )
                };
                let replacement = ResidentOperationService::new(
                    authority,
                    custody,
                    anchor,
                    &mut replacement_tasks,
                );
                let diagnostic = match replacement.recover_a2a_delegate_diagnostic(diagnostic) {
                    Ok(_) => panic!("a replacement owner cannot recover the pending diagnostic"),
                    Err(super::super::a2a_delegate_task_service::ResidentA2aDelegateDiagnosticRecoveryFailure::Reservation {
                        diagnostic,
                        ..
                    }) => diagnostic,
                    Err(super::super::a2a_delegate_task_service::ResidentA2aDelegateDiagnosticRecoveryFailure::StillInadmissible(_)) => {
                        panic!("the replacement owner must be rejected before diagnostic recovery")
                    }
                };
                assert_eq!(replacement.outstanding_task_reservations(), 0);
                assert_eq!(exchange.starts.get(), 1);
                let diagnostic = if persistent {
                    let service =
                        ResidentOperationService::new(authority, custody, anchor, &mut tasks);
                    match service.recover_a2a_delegate_diagnostic(*diagnostic) {
                        Err(super::super::a2a_delegate_task_service::ResidentA2aDelegateDiagnosticRecoveryFailure::StillInadmissible(diagnostic)) => {
                            assert_eq!(service.outstanding_task_reservations(), 1);
                            assert_eq!(exchange.starts.get(), 1);
                            Box::new(diagnostic.restore_outcome_unknown_fixture())
                        }
                        Ok(_) => panic!("the incomplete diagnostic fixture cannot recover"),
                        Err(_) => panic!("the exact owner must reach diagnostic admission"),
                    }
                } else {
                    diagnostic
                };
                let service = ResidentOperationService::new(authority, custody, anchor, &mut tasks);
                match service.recover_a2a_delegate_diagnostic(*diagnostic) {
                    Ok(super::super::a2a_delegate_task_service::ResidentA2aDelegateCompletion::OutcomeUnknown {
                        outcome,
                        finalization,
                    }) => (outcome, finalization),
                    Ok(_) => panic!("diagnostic recovery changed the carrier outcome"),
                    Err(_) => panic!("the exact owner must recover the retained diagnostic"),
                }
            } else {
                (outcome, finalization)
            };

            let retention_clock = A2aTaskOwnerClock::new([Ok(68), Ok(69)]);
            let observation_signs = Cell::new(0_usize);
            let outcome_checkpoint_signs = Cell::new(0_usize);
            let retained = retain_a2a_outcome(
                A2aOutcomeRetentionInput::OutcomeUnknown(A2aUnknownOutcomeRetention::new(
                    outcome,
                    A2aOutcomeUnknownObservationInput {
                        diagnostic_ref: record_ref("a2a-task-owner/diagnostic"),
                        observation_ref: record_ref("a2a-task-owner/outcome-unknown"),
                        reconciliation: ReconciliationFrontierReferenceV1 {
                            record_ref: record_ref("a2a-task-owner/reconciliation"),
                            record_sha256: ReconciliationFrontierDigest::parse(
                                "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
                            )
                            .unwrap(),
                        },
                        clock: &retention_clock,
                        signing,
                        maximum_diagnostic_bytes: 65_536,
                    },
                    A2aSendObservationCommitInput {
                        checkpoint: OperationCheckpointInput {
                            reference: record_ref("a2a-task-owner/outcome-checkpoint"),
                            id: ControlCheckpointIdV1::parse("a2a-task-owner-outcome-checkpoint")
                                .unwrap(),
                        },
                        clock: &retention_clock,
                        checkpoint_schema,
                        checkpoint_authority,
                        accumulator_limits: AccumulatorPreparationLimits::new(
                            8, 20_000, 20_000, 8_192, 200_000,
                        )
                        .unwrap(),
                    },
                    resolve_store,
                    |_| Ok(()),
                    |_, _, _, _| Ok(()),
                    |_, _, _| {
                        observation_signs.set(observation_signs.get() + 1);
                        Ok(signature())
                    },
                    |_, _, _| {
                        outcome_checkpoint_signs.set(outcome_checkpoint_signs.get() + 1);
                        Ok(signature())
                    },
                )),
                custody,
                anchor,
            )
            .expect("uncertain cancellation custody");
            assert_eq!(observation_signs.get(), 1);
            assert_eq!(outcome_checkpoint_signs.get(), 1);

            let mut replacement =
                ResidentOperationService::new(authority, custody, anchor, &mut replacement_tasks);
            let rejected = match replacement.finish_a2a_delegate(finalization, retained) {
                Ok(_) => panic!("a replacement owner cannot release the original capacity"),
                Err(rejected) => rejected,
            };
            assert_eq!(replacement.outstanding_task_reservations(), 0);
            let mut service = ResidentOperationService::new(authority, custody, anchor, &mut tasks);
            if service
                .finish_a2a_delegate(*rejected.finalization, *rejected.retained)
                .is_err()
            {
                panic!("the original owner releases only after exact durable custody");
            }
            assert_eq!(service.outstanding_task_reservations(), 0);
            assert_eq!(exchange.preparations.get(), 1);
            assert_eq!(exchange.starts.get(), 1);
            assert_eq!(
                clock.calls.load(Ordering::SeqCst),
                if matches!(
                    fault,
                    A2aUnopenedFault::AnchorContinuation
                        | A2aUnopenedFault::AnchorLaterSuccessor
                        | A2aUnopenedFault::AnchorLaterPredecessor
                        | A2aUnopenedFault::AnchorLaterUnlike
                        | A2aUnopenedFault::AnchorLaterUnavailable
                ) {
                    2
                } else {
                    3
                }
            );
        },
    );
}

#[test]
fn a2a_release_clock_retry_cancel_and_finish_preserve_one_linear_owner() {
    a2a_unopened_retry_cancel_and_finish_case(A2aUnopenedFault::ReleaseClock);
}

#[test]
fn a2a_unopened_retry_rejects_store_key_route_and_phase_substitution_without_io() {
    a2a_unopened_retry_cancel_and_finish_case(A2aUnopenedFault::ReservationSubstitutions);
}

#[test]
fn dropping_an_unopened_a2a_token_does_not_release_aggregate_capacity() {
    a2a_unopened_retry_cancel_and_finish_case(A2aUnopenedFault::DroppedUnopened);
}

#[test]
fn a2a_preflight_retry_cancel_and_finish_preserve_one_linear_owner() {
    a2a_unopened_retry_cancel_and_finish_case(A2aUnopenedFault::Preflight);
}

#[test]
fn a2a_diagnostic_recovery_retains_capacity_until_exact_finalization() {
    a2a_unopened_retry_cancel_and_finish_case(A2aUnopenedFault::DiagnosticRecovery);
}

#[test]
fn a2a_inadmissible_diagnostic_retry_returns_its_owner_and_retains_capacity() {
    a2a_unopened_retry_cancel_and_finish_case(A2aUnopenedFault::DiagnosticStillInadmissible);
}

#[test]
fn a2a_anchor_continuation_does_not_replay_send_message() {
    a2a_unopened_retry_cancel_and_finish_case(A2aUnopenedFault::AnchorContinuation);
}

#[test]
fn a2a_ambiguous_anchor_later_exact_successor_completes_without_another_advance() {
    a2a_unopened_retry_cancel_and_finish_case(A2aUnopenedFault::AnchorLaterSuccessor);
}

#[test]
fn a2a_ambiguous_anchor_later_exact_predecessor_permits_the_retained_advance() {
    a2a_unopened_retry_cancel_and_finish_case(A2aUnopenedFault::AnchorLaterPredecessor);
}

#[test]
fn a2a_ambiguous_anchor_later_unlike_evidence_returns_the_same_owner() {
    a2a_unopened_retry_cancel_and_finish_case(A2aUnopenedFault::AnchorLaterUnlike);
}

#[test]
fn a2a_ambiguous_anchor_later_unavailable_evidence_returns_the_same_owner() {
    a2a_unopened_retry_cancel_and_finish_case(A2aUnopenedFault::AnchorLaterUnavailable);
}

#[test]
fn a2a_commit_readback_recovers_as_unknown_without_replaying_send_message() {
    a2a_unopened_retry_cancel_and_finish_case(A2aUnopenedFault::CommitRecovery);
}

#[test]
fn a2a_recovered_loss_retries_only_its_rejected_aggregate_transition() {
    a2a_unopened_retry_cancel_and_finish_case(A2aUnopenedFault::CommitRecoveryLossRetry);
}

#[test]
fn a2a_cold_recovery_loss_retry_retains_the_recovered_outcome() {
    a2a_unopened_retry_cancel_and_finish_case(A2aUnopenedFault::GenericRecoveryLossRetry);
}

#[derive(Clone, Copy)]
enum A2aTerminalCase {
    Task,
    NoTask,
    NotStarted,
    LocalReadStopped,
    LocalReadStopCommitUncertain,
    LocalReadStopAnchorUnavailable,
    DuplicateOwnerConflict,
}

impl A2aTerminalCase {
    const fn stops_local_read(self) -> bool {
        matches!(
            self,
            Self::LocalReadStopped
                | Self::LocalReadStopCommitUncertain
                | Self::LocalReadStopAnchorUnavailable
                | Self::DuplicateOwnerConflict
        )
    }

    const fn cancellation_custody_fails(self) -> bool {
        matches!(
            self,
            Self::LocalReadStopCommitUncertain | Self::LocalReadStopAnchorUnavailable
        )
    }
}

fn immutable_a2a_evidence(evidence: &OperationA2aEvidence) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: evidence.coordinate().record_ref().clone(),
        record_sha256: ArtifactDigest::parse(evidence.coordinate().record_sha256().as_str())
            .unwrap(),
    }
}

#[derive(Default)]
struct HeldA2aConflictExecutor {
    tasks: Vec<crate::resident::task_owner::ResidentOperationTask<u8>>,
}

impl ResidentOperationTaskExecutor<u8, u16> for HeldA2aConflictExecutor {
    fn try_accept(
        &mut self,
        task: crate::resident::task_owner::ResidentOperationTask<u8>,
    ) -> Result<(), crate::resident::task_owner::ResidentOperationTask<u8>> {
        self.tasks.push(task);
        Ok(())
    }

    fn try_completion(&mut self) -> Option<ResidentOperationTaskCompletion<u8, u16>> {
        None
    }
}

#[allow(clippy::too_many_lines)]
fn a2a_terminal_settlement_and_finish_case(case: A2aTerminalCase) {
    with_retained_delegate_dispatch(
        A2aTaskOwnerCustodyModel {
            clock_times: vec![63, 64, 65],
            expected_preparations: if matches!(case, A2aTerminalCase::LocalReadStopped) {
                7
            } else if matches!(case, A2aTerminalCase::DuplicateOwnerConflict) {
                5
            } else {
                6
            },
            expected_commits: if matches!(case, A2aTerminalCase::LocalReadStopped) {
                7
            } else if matches!(
                case,
                A2aTerminalCase::DuplicateOwnerConflict
                    | A2aTerminalCase::LocalReadStopCommitUncertain
            ) {
                5
            } else {
                6
            },
            expected_read_backs: if matches!(case, A2aTerminalCase::LocalReadStopped) {
                7
            } else if matches!(case, A2aTerminalCase::DuplicateOwnerConflict) {
                5
            } else {
                6
            },
            expected_anchor_advances: if matches!(case, A2aTerminalCase::LocalReadStopped) {
                7
            } else if matches!(
                case,
                A2aTerminalCase::DuplicateOwnerConflict
                    | A2aTerminalCase::LocalReadStopCommitUncertain
            ) {
                5
            } else {
                6
            },
            expected_anchor_observations: if matches!(case, A2aTerminalCase::LocalReadStopped) {
                7
            } else if matches!(
                case,
                A2aTerminalCase::DuplicateOwnerConflict
                    | A2aTerminalCase::LocalReadStopCommitUncertain
            ) {
                5
            } else {
                6
            },
            unknown_exact_commits: if matches!(case, A2aTerminalCase::LocalReadStopCommitUncertain)
            {
                &[6]
            } else {
                &[]
            },
            unavailable_read_backs: if matches!(case, A2aTerminalCase::LocalReadStopCommitUncertain)
            {
                &[6]
            } else {
                &[]
            },
            unavailable_anchor_advances: if matches!(
                case,
                A2aTerminalCase::LocalReadStopAnchorUnavailable
            ) {
                &[6]
            } else {
                &[]
            },
            unknown_exact_anchor_advances: &[],
            unknown_failed_observation_anchor_advances: &[],
            later_anchor_observations: Vec::new(),
            fail_a2a_preflight_once: false,
        },
        |retained,
         current,
         signing,
         checkpoint_schema,
         checkpoint_authority,
         _provider_clock,
         resolve_store,
         authority,
         custody,
         anchor,
         parents,
         parent_request,
         _restart_generation,
         initial_candidate| {
            let operation = retained.operation();
            let original_request = retained.request();
            let dispatched = (
                retained.observation().0.clone(),
                retained.observation().1.clone(),
            );
            let dispatched_checkpoint = (
                retained.checkpoint().0.clone(),
                retained.checkpoint().1.clone(),
            );
            let recovery = retained.recovery_fixture();
            let provider = retained
                .provider_request()
                .unwrap()
                .provider_binding()
                .clone();
            let route = a2a_exchange::route_for_handle_at_with_authority(
                &provider,
                operation.handle().1,
                "https://agent.example.test/a2a",
                false,
                10,
                100,
                110,
                signing.signing_authority,
            );
            let route_ref = record_ref("a2a-terminal/route");
            let request_id = A2aJsonRpcRequestIdV1::parse("rpc-1").unwrap();
            let response_facts = match case {
                A2aTerminalCase::Task => A2aExchangeFacts::Task {
                    task: A2aRemoteTaskIdV1::parse("task-1").unwrap(),
                    context: A2aRemoteContextIdV1::parse("context-1").unwrap(),
                    state: A2aRemoteTaskStateV1::Working,
                },
                A2aTerminalCase::NoTask
                | A2aTerminalCase::NotStarted
                | A2aTerminalCase::LocalReadStopped
                | A2aTerminalCase::LocalReadStopCommitUncertain
                | A2aTerminalCase::LocalReadStopAnchorUnavailable
                | A2aTerminalCase::DuplicateOwnerConflict => A2aExchangeFacts::DirectMessage,
            };
            let exchange = match case {
                A2aTerminalCase::Task | A2aTerminalCase::NoTask => {
                    A2aTaskOwnerExchangePort::response(response_facts.clone())
                }
                A2aTerminalCase::NotStarted => {
                    A2aTaskOwnerExchangePort::reject(A2aExchangeError::ServiceUnavailable)
                }
                A2aTerminalCase::LocalReadStopped
                | A2aTerminalCase::LocalReadStopCommitUncertain
                | A2aTerminalCase::LocalReadStopAnchorUnavailable
                | A2aTerminalCase::DuplicateOwnerConflict => A2aTaskOwnerExchangePort::default(),
            };
            let protocol = ExactA2aProtocol(response_facts);
            let response_clock = A2aTaskOwnerClock::new([Ok(66), Ok(67), Ok(68)]);
            let retention_clock = A2aTaskOwnerClock::new([Ok(69), Ok(70)]);
            let send_checkpoint_signs = Cell::new(0_usize);
            let outcome_observation_signs = Cell::new(0_usize);
            let outcome_checkpoint_signs = Cell::new(0_usize);
            let mut tasks = ResidentOperationTaskState::new(
                ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                NonZeroUsize::new(1).unwrap(),
                ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
            )
            .unwrap();
            let launch = A2aDelegateLaunchInput {
                send: A2aSendInput {
                    dispatch: A2aSendDispatch::fresh(retained),
                    current,
                    route: (&route_ref, &route),
                    route_predecessor: pale_ports::OperationCustodyIndexPredecessor::Absent,
                    route_association: A2aRouteAssociation::Insert,
                    request_id: &request_id,
                    message_id: "message-1",
                    text: "Inspect the selected workspace.",
                    request_evidence_ref: record_ref("a2a-terminal/request"),
                    preparation_ref: record_ref("a2a-terminal/preparation"),
                    exchange_limits: a2a_exchange::limits(),
                    maximum_evidence_bytes: 8_192,
                    maximum_record_bytes: 65_536,
                    maximum_dispatch_record_bytes: 65_536,
                    resolve_store,
                },
                checkpoint: OperationCheckpointInput {
                    reference: record_ref("a2a-terminal/send-checkpoint"),
                    id: ControlCheckpointIdV1::parse("a2a-terminal-send-checkpoint").unwrap(),
                },
                checkpoint_schema,
                checkpoint_authority,
                response_ref: record_ref("a2a-terminal/response"),
                clock: &response_clock,
                accumulator_limits: AccumulatorPreparationLimits::new(
                    8, 20_000, 20_000, 8_192, 200_000,
                )
                .unwrap(),
                membership_limits: pale_ports::AccumulatorMembershipLimits::new(8, 20_000, 200_000)
                    .unwrap(),
            };
            let mut service = ResidentOperationService::new(authority, custody, anchor, &mut tasks);
            let launch = service
                .launch_a2a_delegate(
                    launch,
                    &exchange,
                    |_, _, _| Ok(()),
                    |_, _, _| Ok(()),
                    |_, _, _| {
                        send_checkpoint_signs.set(send_checkpoint_signs.get() + 1);
                        Ok(signature())
                    },
                )
                .unwrap_or_else(|_| panic!("the terminal A2A fixture must launch exactly once"));
            let completion = match launch {
                ResidentA2aDelegateLaunch::Awaiting(key) => {
                    let waker = std::task::Waker::noop();
                    let mut context = std::task::Context::from_waker(waker);
                    if case.stops_local_read() {
                        assert!(matches!(
                            service.poll_a2a_delegate(
                                &key,
                                &mut context,
                                &NoA2aProtocol,
                                &response_clock,
                            ),
                            ResidentA2aDelegatePoll::Pending
                        ));
                        assert_eq!(service.in_flight_a2a_delegate_tasks(), 1);
                        let foreign_key = ResidentOperationTaskKey::new(
                            AuthorityStoreCoordinate::new(
                                ControlStoreIdV1::parse("foreign-a2a-cancel-store").unwrap(),
                                key.store().incarnation_id().clone(),
                            ),
                            key.dispatch().clone(),
                        );
                        assert!(matches!(
                            service
                                .tasks
                                .select_dispatched_cancellation_owner(&foreign_key),
                            ResidentDispatchedCancellationOwnerSelection::Missing
                        ));
                        let replacement_tasks = ResidentOperationTaskState::new(
                            ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                            NonZeroUsize::new(1).unwrap(),
                            ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                        )
                        .unwrap();
                        let replacement_owner = replacement_tasks
                            .a2a_delegate_tasks
                            .replacement_dispatch_owner_for_test(key.clone());
                        let replacement_attempt = service
                            .tasks
                            .a2a_delegate_tasks
                            .stop_dispatch_response_read(
                                replacement_owner,
                                &mut service.tasks.reservations,
                            );
                        assert!(matches!(
                            replacement_attempt.into_parts().1,
                            ResidentA2aDelegatePoll::NotFound
                        ));
                        assert_eq!(service.in_flight_a2a_delegate_tasks(), 1);
                        if matches!(case, A2aTerminalCase::DuplicateOwnerConflict) {
                            let batch = consultation_task_batch_fixture_in_store(
                                key.store().clone(),
                                "a2a-cancel/conflict-parent",
                                "a2a-cancel/conflict-frontier",
                                "a2a-cancel-conflicting-child",
                            );
                            let candidate = batch.candidates()[0].clone();
                            let coordinate = service
                                .tasks
                                .consultation_tasks
                                .open_batch(
                                    batch,
                                    ResidentConsultationTaskLimits::new(
                                        NonZeroUsize::new(1).unwrap(),
                                        NonZeroUsize::new(1).unwrap(),
                                    ),
                                )
                                .expect("hostile Consult batch opens");
                            let mut executor = HeldA2aConflictExecutor::default();
                            service
                                .tasks
                                .consultation_tasks
                                .seed_conflicting_dispatch_owner_for_test(
                                    &coordinate,
                                    candidate,
                                    key.clone(),
                                    1,
                                    &mut executor,
                                )
                                .expect("hostile fixture duplicates the A2A dispatch owner");
                            let cancellation = request_composed_provider_cancellation(
                                &mut service,
                                operation,
                                original_request,
                                &parents,
                                Some(&parent_request),
                                initial_candidate,
                                (&dispatched.0, &dispatched.1),
                                (&dispatched_checkpoint.0, &dispatched_checkpoint.1),
                                true,
                            );
                            assert!(matches!(
                                cancellation,
                                Err(ResidentOperationCancellationFailure::DispatchOwnerConflict)
                            ));
                            assert_eq!(service.in_flight_a2a_delegate_tasks(), 1);
                            assert_eq!(service.outstanding_task_reservations(), 1);
                            assert!(matches!(
                                service.poll_a2a_delegate(
                                    &key,
                                    &mut context,
                                    &NoA2aProtocol,
                                    &response_clock,
                                ),
                                ResidentA2aDelegatePoll::Pending
                            ));
                            assert_eq!(exchange.preparations.get(), 1);
                            assert_eq!(exchange.starts.get(), 1);
                            return;
                        }
                        let cancellation = request_composed_provider_cancellation(
                            &mut service,
                            operation,
                            original_request,
                            &parents,
                            Some(&parent_request),
                            initial_candidate,
                            (&dispatched.0, &dispatched.1),
                            (&dispatched_checkpoint.0, &dispatched_checkpoint.1),
                            true,
                        );
                        if case.cancellation_custody_fails() {
                            assert!(matches!(
                                cancellation,
                                Err(ResidentOperationCancellationFailure::DispatchedCustody(_))
                            ));
                            assert_eq!(service.in_flight_a2a_delegate_tasks(), 1);
                            assert_eq!(service.outstanding_task_reservations(), 1);
                            assert!(matches!(
                                service.poll_a2a_delegate(
                                    &key,
                                    &mut context,
                                    &NoA2aProtocol,
                                    &response_clock,
                                ),
                                ResidentA2aDelegatePoll::Pending
                            ));
                            assert_eq!(exchange.preparations.get(), 1);
                            assert_eq!(exchange.starts.get(), 1);
                            return;
                        }
                        let cancellation = cancellation.unwrap_or_else(|failure| match failure {
                            ResidentOperationCancellationFailure::Recovery(reason) => {
                                panic!("durable cancellation recovery failed: {reason:?}")
                            }
                            ResidentOperationCancellationFailure::Authorization(reason) => {
                                panic!("durable cancellation authorization failed: {reason:?}")
                            }
                            ResidentOperationCancellationFailure::Request(reason) => {
                                panic!("durable cancellation projection failed: {reason:?}")
                            }
                            ResidentOperationCancellationFailure::Completion(reason) => {
                                panic!("durable cancellation completion failed: {reason:?}")
                            }
                            ResidentOperationCancellationFailure::Custody(reason) => {
                                panic!("durable cancellation custody failed: {reason:?}")
                            }
                            ResidentOperationCancellationFailure::DispatchedCustody(_) => {
                                panic!("durable cancellation custody remained incomplete")
                            }
                            ResidentOperationCancellationFailure::Association => {
                                panic!("durable cancellation lost its dispatch association")
                            }
                            ResidentOperationCancellationFailure::DispatchOwnerConflict => {
                                panic!("durable cancellation found duplicate dispatch owners")
                            }
                            ResidentOperationCancellationFailure::ConsultationRouteRequired => {
                                panic!("A2A cancellation routed to Consult")
                            }
                        });
                        let ResidentOperationCancellationOutcome::A2aDelegateReadStopped {
                            custody: _,
                            checkpoint: _,
                            continuation,
                        } = cancellation
                        else {
                            panic!("the A2A dispatch owner must return its retained continuation")
                        };
                        let (continued_key, poll) = continuation.into_parts();
                        assert_eq!(continued_key, key);
                        assert_eq!(service.in_flight_a2a_delegate_tasks(), 0);
                        assert!(matches!(
                            service.tasks.select_dispatched_cancellation_owner(&key),
                            ResidentDispatchedCancellationOwnerSelection::Missing
                        ));
                        let ResidentA2aDelegatePoll::Completed(completion) = poll else {
                            panic!("local read cancellation must produce explicit uncertainty")
                        };
                        completion
                    } else {
                        let stale_owner = match service
                            .tasks
                            .select_dispatched_cancellation_owner(&key)
                        {
                            ResidentDispatchedCancellationOwnerSelection::Owned(owner) => *owner,
                            _ => panic!("the active A2A send must have one dispatch owner"),
                        };
                        let completion = match service.poll_a2a_delegate(
                            &key,
                            &mut context,
                            &protocol,
                            &response_clock,
                        ) {
                            ResidentA2aDelegatePoll::Completed(completion) => completion,
                            _ => panic!("the finite unary response must complete in one poll"),
                        };
                        let stale_attempt = match stale_owner {
                            ResidentDispatchedCancellationOwner::A2aDelegate(owner) => service
                                .tasks
                                .a2a_delegate_tasks
                                .stop_dispatch_response_read(
                                    owner,
                                    &mut service.tasks.reservations,
                                ),
                            _ => panic!("the exact dispatch owner changed class"),
                        };
                        assert!(matches!(
                            stale_attempt.into_parts().1,
                            ResidentA2aDelegatePoll::NotFound
                        ));
                        completion
                    }
                }
                ResidentA2aDelegateLaunch::Completed(completion) => completion,
                ResidentA2aDelegateLaunch::Diagnostic(_) => {
                    panic!("the selected terminal branch must admit its diagnostic")
                }
            };
            assert_eq!(service.outstanding_task_reservations(), 1);
            assert_eq!(service.in_flight_a2a_delegate_tasks(), 0);
            let common_commit = || A2aSendObservationCommitInput {
                checkpoint: OperationCheckpointInput {
                    reference: record_ref(match case {
                        A2aTerminalCase::Task => "a2a-terminal/task-checkpoint",
                        A2aTerminalCase::NoTask => "a2a-terminal/no-task-checkpoint",
                        A2aTerminalCase::NotStarted => "a2a-terminal/not-started-checkpoint",
                        A2aTerminalCase::LocalReadStopped => {
                            "a2a-terminal/local-read-stopped-checkpoint"
                        }
                        A2aTerminalCase::LocalReadStopCommitUncertain
                        | A2aTerminalCase::LocalReadStopAnchorUnavailable
                        | A2aTerminalCase::DuplicateOwnerConflict => {
                            unreachable!("failed cancellation custody has no outcome checkpoint")
                        }
                    }),
                    id: ControlCheckpointIdV1::parse(match case {
                        A2aTerminalCase::Task => "a2a-terminal-task-checkpoint",
                        A2aTerminalCase::NoTask => "a2a-terminal-no-task-checkpoint",
                        A2aTerminalCase::NotStarted => "a2a-terminal-not-started-checkpoint",
                        A2aTerminalCase::LocalReadStopped => {
                            "a2a-terminal-local-read-stopped-checkpoint"
                        }
                        A2aTerminalCase::LocalReadStopCommitUncertain
                        | A2aTerminalCase::LocalReadStopAnchorUnavailable
                        | A2aTerminalCase::DuplicateOwnerConflict => {
                            unreachable!("failed cancellation custody has no outcome checkpoint")
                        }
                    })
                    .unwrap(),
                },
                clock: &retention_clock,
                checkpoint_schema,
                checkpoint_authority,
                accumulator_limits: AccumulatorPreparationLimits::new(
                    8, 20_000, 20_000, 8_192, 200_000,
                )
                .unwrap(),
            };
            let common_failed_observation = |kind: A2aTerminalCase| A2aFailedSendObservationInput {
                diagnostic_ref: record_ref(match kind {
                    A2aTerminalCase::NoTask => "a2a-terminal/no-task-diagnostic",
                    A2aTerminalCase::NotStarted => "a2a-terminal/not-started-diagnostic",
                    A2aTerminalCase::Task => unreachable!("Task has no failure diagnostic"),
                    A2aTerminalCase::LocalReadStopped => {
                        "a2a-terminal/local-read-stopped-diagnostic"
                    }
                    A2aTerminalCase::LocalReadStopCommitUncertain
                    | A2aTerminalCase::LocalReadStopAnchorUnavailable
                    | A2aTerminalCase::DuplicateOwnerConflict => {
                        unreachable!("failed cancellation custody has no terminal diagnostic")
                    }
                }),
                observation_ref: record_ref(match kind {
                    A2aTerminalCase::NoTask => "a2a-terminal/no-task-observation",
                    A2aTerminalCase::NotStarted => "a2a-terminal/not-started-observation",
                    A2aTerminalCase::Task => unreachable!("Task has no failure observation"),
                    A2aTerminalCase::LocalReadStopped => {
                        "a2a-terminal/local-read-stopped-observation"
                    }
                    A2aTerminalCase::LocalReadStopCommitUncertain
                    | A2aTerminalCase::LocalReadStopAnchorUnavailable
                    | A2aTerminalCase::DuplicateOwnerConflict => {
                        unreachable!("failed cancellation custody has no terminal observation")
                    }
                }),
                clock: &retention_clock,
                signing,
                maximum_diagnostic_bytes: 65_536,
            };
            let (retained_outcome, finalization) = match (case, completion) {
                (
                    A2aTerminalCase::Task,
                    super::super::a2a_delegate_task_service::ResidentA2aDelegateCompletion::Task {
                        response,
                        finalization,
                    },
                ) => {
                    let operation_custody = recovery.recovered((
                        response.send_checkpoint().0.clone(),
                        response.send_checkpoint().1.clone(),
                    ));
                    let (dispatch_ref, dispatch) = response.operation_observation();
                    assert_eq!(operation_custody.selected(), (dispatch_ref, dispatch));
                    let (task, remote_context, remote_state) = response.task();
                    let (request_evidence, response_evidence) = response.evidence();
                    let binding_ref = record_ref("a2a-terminal/task-binding");
                    let binding_schema = schema_digest(A2A_TASK_BINDING_V1_SCHEMA_ID);
                    let binding = A2aTaskBindingPreimageV1 {
                        schema_id: SchemaId::parse(A2A_TASK_BINDING_V1_SCHEMA).unwrap(),
                        schema_sha256: binding_schema.clone(),
                        task_binding_id: A2aTaskBindingIdV1::parse("a2a-terminal-task").unwrap(),
                        operation_handle_id: operation.handle().1.operation_handle_id().clone(),
                        operation_handle_ref: operation.handle().0.clone(),
                        operation_handle_sha256: operation
                            .handle()
                            .1
                            .canonical_operation_handle_sha256()
                            .clone(),
                        operation_observation_ref: dispatch_ref.clone(),
                        operation_observation_sha256: dispatch
                            .canonical_operation_observation_sha256()
                            .clone(),
                        route_admission_id: route.route_admission_id().clone(),
                        route_admission_ref: route_ref.clone(),
                        route_admission_sha256: route
                            .canonical_a2a_route_admission_sha256()
                            .clone(),
                        attempt_id: dispatch
                            .state()
                            .dispatched_attempt_id()
                            .expect("terminal Task follows a dispatched attempt")
                            .clone(),
                        json_rpc_request_id: request_id.clone(),
                        request_mode: A2aTaskRequestModeV1::SendMessage,
                        remote_task_id: task.clone(),
                        remote_context_id: remote_context.clone(),
                        request_evidence: immutable_a2a_evidence(request_evidence),
                        initial_task_evidence: immutable_a2a_evidence(response_evidence),
                        bound_at: response.observed_at(),
                        deployment_runtime_authority_ref: operation
                            .handle()
                            .1
                            .deployment_runtime_authority_ref()
                            .clone(),
                        deployment_runtime_authority_sha256: operation
                            .handle()
                            .1
                            .deployment_runtime_authority_sha256()
                            .clone(),
                        issuer_principal_id: operation.handle().1.issuer_principal_id().clone(),
                        issuer_key_id: operation.handle().1.issuer_key_id().clone(),
                        issuer_key_epoch: *operation.handle().1.issuer_key_epoch(),
                    };
                    let observation_schema = schema_digest(A2A_TASK_OBSERVATION_V1_SCHEMA_ID);
                    let observation = A2aTaskObservationPreimageV1 {
                        schema_id: SchemaId::parse(A2A_TASK_OBSERVATION_V1_SCHEMA).unwrap(),
                        schema_sha256: observation_schema.clone(),
                        task_binding_id: binding.task_binding_id.clone(),
                        task_binding_ref: binding_ref.clone(),
                        task_binding_sha256: A2aTaskBindingDigest::hash_canonical(&binding)
                            .unwrap(),
                        revision: A2aTaskObservationRevisionV1::new(1).unwrap(),
                        predecessor_ref: None,
                        predecessor_sha256: None,
                        source: A2aTaskObservationSourceV1::InitialSendMessageResponse {
                            evidence: immutable_a2a_evidence(response_evidence),
                        },
                        remote_state: *remote_state,
                        reconciliation: None,
                        observed_at: response.observed_at(),
                        deployment_runtime_authority_ref: operation
                            .handle()
                            .1
                            .deployment_runtime_authority_ref()
                            .clone(),
                        deployment_runtime_authority_sha256: operation
                            .handle()
                            .1
                            .deployment_runtime_authority_sha256()
                            .clone(),
                        issuer_principal_id: operation.handle().1.issuer_principal_id().clone(),
                        issuer_key_id: operation.handle().1.issuer_key_id().clone(),
                        issuer_key_epoch: *operation.handle().1.issuer_key_epoch(),
                    };
                    let retained = retain_a2a_outcome(
                        A2aOutcomeRetentionInput::Task(Box::new(A2aTaskOutcomeRetention::new(
                            response,
                            A2aInitialTaskCommitInput {
                                task: A2aInitialTaskCustodyInput {
                                    operation_custody: &operation_custody,
                                    binding_ref,
                                    binding,
                                    binding_signing: SignedResidentAdmissionContextV1 {
                                        expected_schema_sha256: &binding_schema,
                                        signing_authority: signing.signing_authority,
                                    },
                                    observation_ref: record_ref("a2a-terminal/task-observation"),
                                    observation,
                                    observation_signing: SignedResidentAdmissionContextV1 {
                                        expected_schema_sha256: &observation_schema,
                                        signing_authority: signing.signing_authority,
                                    },
                                    maximum_evidence_bytes: 8_192,
                                    maximum_record_bytes: 65_536,
                                },
                                checkpoint: common_commit().checkpoint,
                                clock: &retention_clock,
                                checkpoint_schema,
                                checkpoint_authority,
                                accumulator_limits: common_commit().accumulator_limits,
                                membership_limits: pale_ports::AccumulatorMembershipLimits::new(
                                    8, 20_000, 200_000,
                                )
                                .unwrap(),
                            },
                            |_, _, new, retained| {
                                assert_eq!(new.len(), 1);
                                assert_eq!(retained.len(), 1);
                                Ok(())
                            },
                            |_, _, _| Ok(signature()),
                            |_, _, _| {
                                outcome_observation_signs
                                    .set(outcome_observation_signs.get() + 1);
                                Ok(signature())
                            },
                            |_| Ok(store_coordinate()),
                            |_, _, _| {
                                outcome_checkpoint_signs.set(outcome_checkpoint_signs.get() + 1);
                                Ok(signature())
                            },
                        ))),
                        custody,
                        anchor,
                    )
                    .expect("actual Task custody");
                    let RetainedA2aOutcome::Task(task) = &retained else {
                        panic!("actual Task changed retention branch")
                    };
                    assert_eq!(task.binding().1.remote_task_id().as_str(), "task-1");
                    (retained, finalization)
                }
                (
                    A2aTerminalCase::NoTask,
                    super::super::a2a_delegate_task_service::ResidentA2aDelegateCompletion::NoTask {
                        response,
                        finalization,
                    },
                ) => {
                    let retained = retain_a2a_outcome(
                        A2aOutcomeRetentionInput::NoTask(A2aNoTaskOutcomeRetention::new(
                            response,
                            common_failed_observation(case),
                            common_commit(),
                            resolve_store,
                            |_, _, _, _| Ok(()),
                            |_, _, _| {
                                outcome_observation_signs
                                    .set(outcome_observation_signs.get() + 1);
                                Ok(signature())
                            },
                            |_, _, _| {
                                outcome_checkpoint_signs.set(outcome_checkpoint_signs.get() + 1);
                                Ok(signature())
                            },
                        )),
                        custody,
                        anchor,
                    )
                    .expect("direct Message no-Task custody");
                    let RetainedA2aOutcome::NoTask(no_task) = &retained else {
                        panic!("direct Message changed retention branch")
                    };
                    assert!(no_task
                        .custody()
                        .candidate()
                        .record(OperationCustodyRecordKind::A2aTaskBinding)
                        .is_none());
                    assert!(no_task
                        .custody()
                        .candidate()
                        .record(OperationCustodyRecordKind::A2aTaskObservation)
                        .is_none());
                    (retained, finalization)
                }
                (
                    A2aTerminalCase::NotStarted,
                    super::super::a2a_delegate_task_service::ResidentA2aDelegateCompletion::NotStarted {
                        response,
                        finalization,
                    },
                ) => {
                    let retained = retain_a2a_outcome(
                        A2aOutcomeRetentionInput::NotStarted(A2aNotStartedOutcomeRetention::new(
                            response,
                            common_failed_observation(case),
                            common_commit(),
                            resolve_store,
                            |_, _, _, _| Ok(()),
                            |_, _, _| {
                                outcome_observation_signs
                                    .set(outcome_observation_signs.get() + 1);
                                Ok(signature())
                            },
                            |_, _, _| {
                                outcome_checkpoint_signs.set(outcome_checkpoint_signs.get() + 1);
                                Ok(signature())
                            },
                        )),
                        custody,
                        anchor,
                    )
                    .expect("not-started custody");
                    (retained, finalization)
                }
                (
                    A2aTerminalCase::LocalReadStopped,
                    super::super::a2a_delegate_task_service::ResidentA2aDelegateCompletion::OutcomeUnknown {
                        outcome,
                        finalization,
                    },
                ) => {
                    assert!(outcome.diagnostic().wire().facts.iter().any(|fact| {
                        fact.key == "uncertainty" && fact.value == "local_read_cancelled"
                    }));
                    let retained = retain_a2a_outcome(
                        A2aOutcomeRetentionInput::OutcomeUnknown(A2aUnknownOutcomeRetention::new(
                            outcome,
                            A2aOutcomeUnknownObservationInput {
                                diagnostic_ref: record_ref(
                                    "a2a-terminal/local-read-stopped-diagnostic",
                                ),
                                observation_ref: record_ref(
                                    "a2a-terminal/local-read-stopped-observation",
                                ),
                                reconciliation: ReconciliationFrontierReferenceV1 {
                                    record_ref: record_ref(
                                        "a2a-terminal/local-read-stopped-reconciliation",
                                    ),
                                    record_sha256: ReconciliationFrontierDigest::parse(
                                        "abababababababababababababababababababababababababababababababab",
                                    )
                                    .unwrap(),
                                },
                                clock: &retention_clock,
                                signing,
                                maximum_diagnostic_bytes: 65_536,
                            },
                            common_commit(),
                            resolve_store,
                            |_| Ok(()),
                            |_, _, _, _| Ok(()),
                            |_, _, _| {
                                outcome_observation_signs
                                    .set(outcome_observation_signs.get() + 1);
                                Ok(signature())
                            },
                            |_, _, _| {
                                outcome_checkpoint_signs.set(outcome_checkpoint_signs.get() + 1);
                                Ok(signature())
                            },
                        )),
                        custody,
                        anchor,
                    )
                    .expect("local read uncertainty custody");
                    (retained, finalization)
                }
                _ => panic!("the terminal carrier branch changed meaning"),
            };
            assert_eq!(outcome_observation_signs.get(), 1);
            assert_eq!(outcome_checkpoint_signs.get(), 1);
            let mut service = ResidentOperationService::new(authority, custody, anchor, &mut tasks);
            assert_eq!(service.outstanding_task_reservations(), 1);
            let _retained = service
                .finish_a2a_delegate(finalization, retained_outcome)
                .unwrap_or_else(|_| panic!("exact durable terminal custody releases capacity"));
            assert_eq!(service.outstanding_task_reservations(), 0);
            assert_eq!(exchange.preparations.get(), 1);
            assert_eq!(exchange.starts.get(), 1);
            assert_eq!(send_checkpoint_signs.get(), 1);
        },
    );
}

#[test]
fn a2a_task_response_settles_and_finishes_only_after_durable_task_custody() {
    a2a_terminal_settlement_and_finish_case(A2aTerminalCase::Task);
}

#[test]
fn a2a_direct_message_settles_without_creating_a_task_binding_and_finishes() {
    a2a_terminal_settlement_and_finish_case(A2aTerminalCase::NoTask);
}

#[test]
fn a2a_not_started_response_settles_and_finishes_after_durable_failure_custody() {
    a2a_terminal_settlement_and_finish_case(A2aTerminalCase::NotStarted);
}

#[test]
fn durable_cancel_stops_one_exact_a2a_response_read_before_capacity_release() {
    a2a_terminal_settlement_and_finish_case(A2aTerminalCase::LocalReadStopped);
}

#[test]
fn uncertain_cancel_commit_keeps_the_a2a_response_read_active() {
    a2a_terminal_settlement_and_finish_case(A2aTerminalCase::LocalReadStopCommitUncertain);
}

#[test]
fn unavailable_cancel_anchor_keeps_the_a2a_response_read_active() {
    a2a_terminal_settlement_and_finish_case(A2aTerminalCase::LocalReadStopAnchorUnavailable);
}

#[test]
fn duplicate_consult_and_a2a_dispatch_owners_fail_closed_before_custody() {
    a2a_terminal_settlement_and_finish_case(A2aTerminalCase::DuplicateOwnerConflict);
}

#[test]
fn a2a_launch_rejects_duplicate_before_preparation_or_carrier_io() {
    a2a_unopened_retry_cancel_and_finish_case(A2aUnopenedFault::Duplicate);
}

#[test]
fn a2a_launch_rejects_saturation_before_preparation_or_carrier_io() {
    a2a_unopened_retry_cancel_and_finish_case(A2aUnopenedFault::Saturated);
}

fn cancellation_request(
    original: &CommonRequestEnvelopeV1,
    handle: &OperationHandleV1,
) -> CommonRequestEnvelopeV1 {
    let mut preimage = original.to_preimage();
    preimage.request_id = ResidentRequestIdV1::parse("operation-service-cancel").unwrap();
    preimage.correlation_id = ResidentCorrelationIdV1::parse("operation-service-cancel").unwrap();
    preimage.capability_id = CapabilityIdV1::parse(OPERATION_CANCEL_CAPABILITY_ID).unwrap();
    preimage.idempotency_key = RequestIdempotencyKeyV1::parse("operation-service-cancel").unwrap();
    preimage.issued_at = UnixTimeV1::new(handle.created_at().get() + 2).unwrap();
    preimage.expires_at = *handle.retain_until();
    preimage.generation = Some(handle.compiled_generation_sha256().clone());
    preimage.fence_token = Some(handle.activation_fence_sha256().clone());
    preimage.body.schema_id = SchemaId::parse(OPERATION_HANDLE_V1_SCHEMA).unwrap();
    preimage.body.schema_sha256 = handle.schema_sha256().clone();
    preimage.body.payload = serde_json::to_value(handle).unwrap();
    preimage.body.payload_sha256 = ArtifactDigest::hash_bytes(
        pale_core::canon::to_canonical_string(&preimage.body.payload)
            .unwrap()
            .as_bytes(),
    );
    request_fixture::request(&preimage)
}

fn checkpoint_authority(
    checkpoint: &ControlStateCheckpointV1,
) -> ControlCheckpointSigningAuthorityAdmissionContextV1<'_> {
    ControlCheckpointSigningAuthorityAdmissionContextV1 {
        authority_ref: checkpoint.checkpoint_signing_authority_ref(),
        authority_sha256: checkpoint.checkpoint_signing_authority_sha256(),
        principal_id: checkpoint.issuer_principal_id(),
        control_store_id: checkpoint.control_store_id(),
        control_incarnation_id: checkpoint.control_incarnation_id(),
        checkpoint_domain_id: checkpoint.checkpoint_domain_id(),
        checkpoint_profile_ref: checkpoint.checkpoint_profile_ref(),
        checkpoint_profile_sha256: checkpoint.checkpoint_profile_sha256(),
        subject_key_id: checkpoint.issuer_key_id(),
        subject_key_epoch: *checkpoint.issuer_key_epoch(),
    }
}

fn with_authorized_cancellation(
    state: OperationObservationStateV1,
    run: impl FnOnce(
        &mut ProvingAuthority,
        &mut dyn OperationCustodyPort,
        &mut dyn AntiRollbackAnchorPort,
        AuthorizedOperationCancellation<'_>,
        CancellationAppendInput<'_>,
        CancellationAppendAdmission<'_>,
    ),
) {
    with_authorized_cancellation_custody(state, &[], false, run);
}

fn with_authorized_cancellation_custody(
    state: OperationObservationStateV1,
    conflicts_on_commit: &[usize],
    fail_anchor_advance: bool,
    run: impl FnOnce(
        &mut ProvingAuthority,
        &mut dyn OperationCustodyPort,
        &mut dyn AntiRollbackAnchorPort,
        AuthorizedOperationCancellation<'_>,
        CancellationAppendInput<'_>,
        CancellationAppendAdmission<'_>,
    ),
) {
    let expects_append = matches!(
        state,
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Resolving
                | OperationActivePhaseV1::ContextBound { .. }
                | OperationActivePhaseV1::ProviderResolved { .. }
                | OperationActivePhaseV1::DispatchPrepared { .. }
        }
    );
    with_authorized_cancellation_custody_counts(
        state,
        conflicts_on_commit,
        fail_anchor_advance,
        usize::from(expects_append),
        usize::from(expects_append && conflicts_on_commit.is_empty()),
        run,
    );
}

pub(in crate::resident) fn with_owned_dispatched_cancellation_custody(
    conflicts_on_commit: &[usize],
    fail_anchor_advance: bool,
    run: impl FnOnce(
        &mut ProvingAuthority,
        &mut dyn OperationCustodyPort,
        &mut dyn AntiRollbackAnchorPort,
        AuthorizedOperationCancellation<'_>,
        CancellationAppendInput<'_>,
        CancellationAppendAdmission<'_>,
    ),
) {
    with_authorized_cancellation_custody_counts(
        dispatched_state("consult-child"),
        conflicts_on_commit,
        fail_anchor_advance,
        1,
        if conflicts_on_commit.is_empty() {
            if fail_anchor_advance {
                2
            } else {
                1
            }
        } else {
            0
        },
        run,
    );
}

fn with_authorized_cancellation_custody_counts(
    state: OperationObservationStateV1,
    conflicts_on_commit: &[usize],
    fail_anchor_advance: bool,
    expected_appends: usize,
    expected_anchor_advances: usize,
    run: impl FnOnce(
        &mut ProvingAuthority,
        &mut dyn OperationCustodyPort,
        &mut dyn AntiRollbackAnchorPort,
        AuthorizedOperationCancellation<'_>,
        CancellationAppendInput<'_>,
        CancellationAppendAdmission<'_>,
    ),
) {
    let (generation, profile) = local_start::source_generation();
    local_start::with_issued_context(
        &generation,
        &profile,
        "operation-service",
        |_| {},
        crate::session_admission_fixture::LocalStartContext::Inherited,
        |issued, original, parents, checkpoint, _, _| {
            let (handle_ref, handle) = issued.handle();
            let observation_ref = record_ref("operation/service-current");
            let observation = issue_observation(
                parents.signing,
                handle_ref,
                handle,
                schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID),
                OperationObservationRevisionV1::new(1).unwrap(),
                None,
                state,
                handle.created_at().get() + 1,
            );
            let recovered_operation =
                RecoveredLocalOperation::fixture(issued, original.clone(), store_coordinate());
            let recovered_observation = RecoveredOperationObservation::fixture(
                issued,
                (observation_ref.clone(), observation.clone()),
                (observation_ref, observation),
                (record_ref("operation/checkpoint-1"), checkpoint.clone()),
                store_coordinate(),
            );
            let request = cancellation_request(original, handle);
            let authorized = AuthorizedOperationCancellation::fixture(
                &request,
                &recovered_operation,
                &recovered_observation,
                UnixTimeV1::new(request.issued_at().get() + 1).unwrap(),
            );
            let successor_preimage = checkpoint_preimage(
                checkpoint.domain_sequence().get() + 1,
                Some((
                    record_ref("operation/checkpoint-1"),
                    checkpoint.canonical_control_checkpoint_sha256().clone(),
                )),
            );
            let authority = checkpoint_authority(checkpoint);
            let mut rotate = |_: &ResidentRecordRefV1,
                              _: &ArtifactDigest,
                              _: &ResidentRecordRefV1,
                              _: &ArtifactDigest| { Ok(()) };
            let mut sign_observation =
                |_: &[u8], _: &OperationObservationPreimageV1, _: &OperationObservationDigest| {
                    Ok(signature())
                };
            let mut sign_checkpoint =
                |_: &[u8],
                 _: &ControlStateCheckpointPreimageV1,
                 _: &ControlStateCheckpointDigest| { Ok(signature()) };
            let resolve_store = local_start::StartStore(parents.session);
            let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
            custody::with_service_custody_and_handle(
                checkpoint,
                None,
                None,
                expected_appends,
                expected_anchor_advances,
                fail_anchor_advance,
                conflicts_on_commit,
                None,
                Default::default(),
                |store, anchor| {
                    run(
                        &mut ProvingAuthority,
                        store,
                        anchor,
                        authorized,
                        CancellationAppendInput {
                            request_ref: record_ref("request/operation-service-cancel"),
                            observation_ref: record_ref("operation/service-cancellation"),
                            signing: SignedResidentAdmissionContextV1 {
                                expected_schema_sha256: &observation_schema,
                                signing_authority: parents.signing,
                            },
                            checkpoint: OperationCheckpointInput {
                                reference: record_ref("operation/checkpoint-2"),
                                id: successor_preimage.control_checkpoint_id.clone(),
                            },
                            checkpoint_issued_at: UnixTimeV1::new(request.issued_at().get() + 2)
                                .unwrap(),
                            checkpoint_schema: &successor_preimage.schema_sha256,
                            checkpoint_authority: &authority,
                            accumulator_limits: AccumulatorPreparationLimits::new(
                                2, 1024, 1024, 512, 65_536,
                            )
                            .unwrap(),
                        },
                        CancellationAppendAdmission {
                            resolve_store: &resolve_store,
                            admit_rotation: &mut rotate,
                            sign_observation: &mut sign_observation,
                            sign_checkpoint: &mut sign_checkpoint,
                        },
                    )
                },
            );
        },
    );
}

#[path = "operation_service/consult_child_cancellation.rs"]
mod consult_child_cancellation;

fn dispatched_state(owner_fixture: &str) -> OperationObservationStateV1 {
    OperationObservationStateV1::Active {
        phase: OperationActivePhaseV1::Dispatched {
            attempt_id: CapabilityAttemptIdV1::parse("attempt-1").unwrap(),
            evidence: ImmutableRecordReferenceV1 {
                record_ref: record_ref(&format!("selection/{owner_fixture}")),
                record_sha256: artifact_digest('7'),
            },
        },
    }
}

#[test]
fn a2a_and_native_dispatches_return_to_their_owners_without_an_append() {
    for owner_fixture in ["a2a", "native"] {
        with_authorized_cancellation_custody(
            dispatched_state(owner_fixture),
            &[],
            false,
            |authority, store, anchor, authorized, input, mut admission| {
                let mut tasks = ResidentOperationTaskState::new(
                    ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                    NonZeroUsize::new(1).expect("consultation batch bound"),
                    ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                )
                .unwrap_or_else(|_| panic!("one bounded provider task slot must allocate"));
                let mut service =
                    ResidentOperationService::new(authority, store, anchor, &mut tasks);
                let outcome = service
                    .retain_authorized_cancellation(authorized, input, &mut admission)
                    .unwrap_or_else(|_| panic!("owner routing cannot require custody"));
                assert!(matches!(
                    outcome,
                    ResidentOperationCancellationOutcome::DispatchOwnerRequired(_)
                ));
            },
        );
    }
}

#[test]
fn before_dispatch_cancel_retains_a_nonterminal_request_without_task_notification() {
    with_authorized_cancellation(
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Resolving,
        },
        |authority, store, anchor, authorized, input, mut admission| {
            let mut tasks = ResidentOperationTaskState::new(
                ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                NonZeroUsize::new(1).expect("consultation batch bound"),
                ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
            )
            .unwrap();
            let mut service = ResidentOperationService::new(authority, store, anchor, &mut tasks);
            let outcome = service
                .retain_authorized_cancellation(authorized, input, &mut admission)
                .unwrap_or_else(|_| panic!("local cancellation request must reach custody"));
            assert!(matches!(
                outcome,
                ResidentOperationCancellationOutcome::LocalRequestRetained { .. }
            ));
            assert_eq!(service.in_flight_provider_tasks(), 0);
        },
    );
}

#[test]
fn unowned_provider_dispatch_does_not_append_or_fabricate_notification() {
    with_authorized_cancellation(
        dispatched_state("provider"),
        |authority, store, anchor, authorized, input, mut admission| {
            let mut tasks = ResidentOperationTaskState::new(
                ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                NonZeroUsize::new(1).expect("consultation batch bound"),
                ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
            )
            .unwrap();
            let mut service = ResidentOperationService::new(authority, store, anchor, &mut tasks);
            let outcome = service.retain_authorized_cancellation(authorized, input, &mut admission);
            assert!(matches!(
                outcome,
                Ok(ResidentOperationCancellationOutcome::DispatchOwnerRequired(
                    _
                ))
            ));
        },
    );
}

#[test]
fn consultation_terminal_and_uncertain_states_remain_distinct_and_do_not_append() {
    let cases = [
        (
            "collecting",
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::CollectingConsultation {
                    evidence: ImmutableRecordReferenceV1 {
                        record_ref: record_ref("consultation/collecting"),
                        record_sha256: artifact_digest('1'),
                    },
                },
            },
        ),
        (
            "collected",
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::ConsultationCollected {
                    evidence: ImmutableRecordReferenceV1 {
                        record_ref: record_ref("consultation/collected"),
                        record_sha256: artifact_digest('2'),
                    },
                },
            },
        ),
        (
            "terminal",
            OperationObservationStateV1::Succeeded {
                result: CapabilityResultReferenceV1 {
                    record_ref: record_ref("result/terminal"),
                    record_sha256: CapabilityResultDigest::parse(
                        crate::local_operation_start_fixture::sha('3'),
                    )
                    .unwrap(),
                },
                effect_receipt: None,
            },
        ),
        (
            "uncertain",
            OperationObservationStateV1::OutcomeUnknown {
                diagnostic: ImmutableRecordReferenceV1 {
                    record_ref: record_ref("diagnostic/unknown"),
                    record_sha256: artifact_digest('4'),
                },
                reconciliation: ReconciliationFrontierReferenceV1 {
                    record_ref: record_ref("reconciliation/unknown"),
                    record_sha256: ReconciliationFrontierDigest::parse(
                        crate::local_operation_start_fixture::sha('5'),
                    )
                    .unwrap(),
                },
                effect_receipt: None,
            },
        ),
        (
            "duplicate",
            OperationObservationStateV1::CancellationRequested {
                request: ImmutableRecordReferenceV1 {
                    record_ref: record_ref("request/already-cancelled"),
                    record_sha256: artifact_digest('6'),
                },
            },
        ),
    ];
    for (expected, state) in cases {
        with_authorized_cancellation(
            state,
            |authority, store, anchor, authorized, input, mut admission| {
                let mut tasks = ResidentOperationTaskState::new(
                    ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                    NonZeroUsize::new(1).expect("consultation batch bound"),
                    ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                )
                .unwrap();
                let mut service =
                    ResidentOperationService::new(authority, store, anchor, &mut tasks);
                let outcome =
                    service.retain_authorized_cancellation(authorized, input, &mut admission);
                assert!(matches!(
                    (expected, outcome),
                    (
                        "collecting",
                        Err(ResidentOperationCancellationFailure::ConsultationRouteRequired),
                    ) | (
                        "collected",
                        Ok(ResidentOperationCancellationOutcome::ConsultationCollected(
                            _
                        )),
                    ) | (
                        "terminal",
                        Ok(ResidentOperationCancellationOutcome::Established(_))
                    ) | (
                        "uncertain",
                        Ok(ResidentOperationCancellationOutcome::ReconciliationOnly(_))
                    ) | (
                        "duplicate",
                        Ok(ResidentOperationCancellationOutcome::AlreadyRequested(_))
                    )
                ));
            },
        );
    }
}

#[test]
fn short_lived_operation_facades_share_exact_consultation_task_ownership() {
    with_authorized_cancellation(
        OperationObservationStateV1::Recovering {
            reconciliation: ReconciliationFrontierReferenceV1 {
                record_ref: record_ref("reconciliation/consultation-facade"),
                record_sha256: ReconciliationFrontierDigest::parse(
                    crate::local_operation_start_fixture::sha('7'),
                )
                .unwrap(),
            },
        },
        |authority, store, anchor, _, _, _| {
            let provider_limits = ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap());
            let maximum_active_consultation_batches =
                NonZeroUsize::new(2).expect("active consultation bound");
            let child_limits = ResidentConsultationTaskLimits::new(
                NonZeroUsize::new(1).expect("in-flight child bound"),
                NonZeroUsize::new(1).expect("batch member bound"),
            );
            let first = consultation_task_batch_fixture(
                "operation/consult-facade-first",
                "collection/facade-first",
                "facade-first",
            );

            let mut tasks = ResidentOperationTaskState::new(
                provider_limits,
                maximum_active_consultation_batches,
                ResidentAggregateTaskLimits::new(NonZeroUsize::new(8).unwrap()),
            )
            .expect("bounded operation and consultation state");
            let first_coordinate;
            {
                let mut service =
                    ResidentOperationService::new(authority, store, anchor, &mut tasks);
                first_coordinate = service
                    .open_consultation_batch(first, child_limits)
                    .expect("first exact batch");
                assert!(service.consultation_batch_mut(&first_coordinate).is_some());
                assert_eq!(service.active_consultation_batches(), 1);
                assert_eq!(service.in_flight_consultation_tasks(), 0);
                let _coordinator = service.consultation_coordinator();
            }

            {
                let mut service =
                    ResidentOperationService::new(authority, store, anchor, &mut tasks);
                assert!(service.consultation_batch_mut(&first_coordinate).is_some());
                let second = consultation_task_batch_fixture(
                    "operation/consult-facade-second",
                    "collection/facade-second",
                    "facade-second",
                );
                let second_coordinate = service
                    .open_consultation_batch(second, child_limits)
                    .expect("second exact batch");
                assert_ne!(first_coordinate, second_coordinate);
                assert_eq!(service.active_consultation_batches(), 2);
                assert_eq!(
                    service.retire_consultation_batch(&first_coordinate),
                    ResidentConsultationBatchRetirement::Retired
                );
                assert!(service.consultation_batch_mut(&first_coordinate).is_none());
                assert!(service.consultation_batch_mut(&second_coordinate).is_some());
            }
        },
    );
}

#[test]
fn short_lived_facades_preserve_a2a_aggregate_ownership() {
    with_authorized_cancellation(
        OperationObservationStateV1::Recovering {
            reconciliation: ReconciliationFrontierReferenceV1 {
                record_ref: record_ref("reconciliation/a2a-facade"),
                record_sha256: ReconciliationFrontierDigest::parse(
                    crate::local_operation_start_fixture::sha('8'),
                )
                .unwrap(),
            },
        },
        |authority, store, anchor, _, _, _| {
            let mut tasks = ResidentOperationTaskState::new(
                ResidentOperationTaskLimits::new(NonZeroUsize::new(1).unwrap()),
                NonZeroUsize::new(1).expect("consultation batch bound"),
                ResidentAggregateTaskLimits::new(NonZeroUsize::new(1).unwrap()),
            )
            .expect("bounded resident task state");
            tasks.accept_a2a_reservation_for_test(ResidentOperationTaskKey::new(
                store_coordinate(),
                ImmutableRecordReferenceV1 {
                    record_ref: record_ref("operation/a2a-dispatch"),
                    record_sha256: artifact_digest('9'),
                },
            ));

            {
                let service = ResidentOperationService::new(authority, store, anchor, &mut tasks);
                assert_eq!(service.outstanding_task_reservations(), 1);
                assert_eq!(service.in_flight_a2a_delegate_tasks(), 0);
            }
            {
                let service = ResidentOperationService::new(authority, store, anchor, &mut tasks);
                assert_eq!(service.outstanding_task_reservations(), 1);
                assert_eq!(service.in_flight_a2a_delegate_tasks(), 0);
            }
        },
    );
}
