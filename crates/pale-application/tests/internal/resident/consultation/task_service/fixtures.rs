//! Shared test executors, task builders and hostile process-local setup.

use super::*;

impl ResidentConsultationTaskService {
    pub(in crate::resident) fn new(
        planned: ConsultationChildDispatchBatch,
        limits: ResidentConsultationTaskLimits,
    ) -> Result<Self, ResidentConsultationTaskBuildError> {
        Self::new_in(planned, limits, ResidentOperationTaskOwnerInstance::new())
    }

    pub(super) fn legacy_reservations(&self) -> ResidentAggregateTaskReservations {
        ResidentAggregateTaskReservations::new(
            self.tasks.owner_instance().clone(),
            ResidentAggregateTaskLimits::new(
                NonZeroUsize::new(1).expect("positive focused-test aggregate limit"),
            ),
        )
        .expect("focused-test aggregate ledger")
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn retain_and_accept<
        Work,
        Output,
        LookupError,
        RetentionFailure,
        Lookup,
        RecoverChild,
        Retain,
        TaskExecutor,
    >(
        &mut self,
        candidate: &ConsultationChildDispatchCandidate,
        clock: &dyn WallClockSourcePort,
        lookup: Lookup,
        recover_child: RecoverChild,
        retain: Retain,
        executor: &mut TaskExecutor,
    ) -> Result<
        ResidentOperationTaskKey,
        Box<ResidentConsultationTaskAcceptanceFailure<Work, LookupError, RetentionFailure>>,
    >
    where
        Lookup: FnOnce() -> Result<RecoveredConsultationCollection, LookupError>,
        RecoverChild: FnOnce(
            &ConsultationChildDispatchCandidate,
        ) -> Result<RecoveredOperationObservation, LookupError>,
        Retain: FnOnce(
            &ConsultationChildDispatchCandidate,
            RecoveredOperationObservation,
        ) -> ConsultationChildRetentionOutcome<Work, RetentionFailure>,
        TaskExecutor: ResidentOperationTaskExecutor<Work, Output>,
    {
        let mut reservations = self.legacy_reservations();
        self.retain_and_accept_reserved(
            candidate,
            &mut reservations,
            clock,
            lookup,
            recover_child,
            retain,
            executor,
        )
    }

    pub(in crate::resident) fn retry_accept<Work, Output, TaskExecutor>(
        &mut self,
        coordinate: &ResidentConsultationBatchCoordinate,
        retry: ResidentConsultationTaskRetry<Work>,
        executor: &mut TaskExecutor,
    ) -> Result<ResidentOperationTaskKey, ResidentConsultationTaskRetryFailure<Work>>
    where
        TaskExecutor: ResidentOperationTaskExecutor<Work, Output>,
    {
        let mut reservations = self.legacy_reservations();
        self.retry_accept_reserved(coordinate, retry, &mut reservations, executor)
    }

    pub(in crate::resident) fn try_completion<Work, Output, TaskExecutor>(
        &mut self,
        executor: &mut TaskExecutor,
    ) -> Option<ResidentConsultationTaskPoll<Work, Output>>
    where
        TaskExecutor: ResidentOperationTaskExecutor<Work, Output>,
    {
        executor
            .try_completion()
            .map(|completion| self.join_completion(completion))
    }

    pub(in crate::resident) fn try_completion_and_join<
        Work,
        Output,
        TaskExecutor,
        LookupError,
        JoinOutcome,
        Lookup,
        Join,
    >(
        &mut self,
        executor: &mut TaskExecutor,
        lookup: Lookup,
        join: Join,
    ) -> Option<ResidentConsultationTaskJoinPoll<Work, Output, JoinOutcome, LookupError>>
    where
        TaskExecutor: ResidentOperationTaskExecutor<Work, Output>,
        Lookup: FnOnce() -> Result<RecoveredConsultationCollection, LookupError>,
        Join: FnOnce(
            &RecoveredConsultationCollection,
            &ConsultationChildDispatchCandidate,
            &Output,
        ) -> JoinOutcome,
    {
        self.try_completion(executor)
            .map(|completion| match completion {
                ResidentConsultationTaskPoll::Completed {
                    candidate,
                    dispatch,
                    work,
                    output,
                    cancellation,
                } => match lookup() {
                    Ok(current) => {
                        let joined = join(&current, &candidate, &output);
                        ResidentConsultationTaskJoinPoll::Joined {
                            candidate,
                            dispatch,
                            work,
                            output,
                            joined,
                            cancellation,
                        }
                    }
                    Err(reason) => ResidentConsultationTaskJoinPoll::JoinRecoveryFailed {
                        candidate,
                        dispatch,
                        work,
                        output,
                        cancellation,
                        reason,
                    },
                },
                ResidentConsultationTaskPoll::Lost(lost) => {
                    ResidentConsultationTaskJoinPoll::Lost(lost)
                }
                ResidentConsultationTaskPoll::Stale(completion) => {
                    ResidentConsultationTaskJoinPoll::Stale(completion)
                }
            })
    }

    pub(super) fn try_accept_retained<Work, Output, TaskExecutor>(
        &mut self,
        retained: RetainedConsultationTask<Work>,
        executor: &mut TaskExecutor,
    ) -> Result<ResidentOperationTaskKey, ResidentConsultationTaskRefusal<Work>>
    where
        TaskExecutor: ResidentOperationTaskExecutor<Work, Output>,
    {
        let mut reservations = self.legacy_reservations();
        self.try_accept_retained_reserved(retained, &mut reservations, executor)
    }
}

impl ResidentConsultationBatchCoordinate {
    pub(in crate::resident) fn hostile_fixture(
        owner_instance: ResidentOperationTaskOwnerInstance,
        store: AuthorityStoreCoordinate,
        parent: ImmutableRecordReferenceV1,
        collection_frontier: ImmutableRecordReferenceV1,
    ) -> Self {
        Self {
            owner_instance,
            store,
            parent,
            collection_frontier,
        }
    }
}

impl ResidentConsultationTaskOwner {
    pub(in crate::resident) fn new(
        limits: ResidentConsultationTaskOwnerLimits,
    ) -> Result<Self, ResidentConsultationTaskOwnerBuildError> {
        Self::new_in(ResidentOperationTaskOwnerInstance::new(), limits)
    }

    fn legacy_reservations(&self) -> ResidentAggregateTaskReservations {
        ResidentAggregateTaskReservations::new(
            self.owner_instance.clone(),
            ResidentAggregateTaskLimits::new(
                NonZeroUsize::new(1).expect("positive focused-test aggregate limit"),
            ),
        )
        .expect("focused-test aggregate ledger")
    }

    pub(in crate::resident) fn accept_retained_for_test<Work, Output, TaskExecutor>(
        &mut self,
        coordinate: &ResidentConsultationBatchCoordinate,
        retained: RetainedConsultationTask<Work>,
        reservations: &mut ResidentAggregateTaskReservations,
        executor: &mut TaskExecutor,
    ) -> Option<Result<ResidentOperationTaskKey, ResidentConsultationTaskRefusal<Work>>>
    where
        TaskExecutor: ResidentOperationTaskExecutor<Work, Output>,
    {
        self.batches
            .iter_mut()
            .find(|batch| &batch.coordinate == coordinate)
            .map(|batch| {
                batch
                    .tasks
                    .try_accept_retained_reserved(retained, reservations, executor)
            })
    }

    pub(in crate::resident) fn seed_retained_for_supervision_test<Work, Output, TaskExecutor>(
        &mut self,
        coordinate: &ResidentConsultationBatchCoordinate,
        candidate: ConsultationChildDispatchCandidate,
        dispatch: ResidentOperationTaskKey,
        work: Work,
        reservations: &mut ResidentAggregateTaskReservations,
        executor: &mut TaskExecutor,
    ) -> Result<ResidentOperationTaskKey, ResidentConsultationTaskRefusal<Work>>
    where
        TaskExecutor: ResidentOperationTaskExecutor<Work, Output>,
    {
        self.accept_retained_for_test(
            coordinate,
            RetainedConsultationTask {
                basis: ConsultationTaskBasis {
                    candidate,
                    dispatch,
                },
                work,
            },
            reservations,
            executor,
        )
        .expect("the exact supervision-test batch is retained")
    }

    /// Seed an intentionally conflicting process-local dispatch owner.
    ///
    /// This hostile test path bypasses the shared aggregate ledger so routing
    /// tests can prove that duplicate exact ownership fails before custody.
    pub(in crate::resident) fn seed_conflicting_dispatch_owner_for_test<
        Work,
        Output,
        TaskExecutor,
    >(
        &mut self,
        coordinate: &ResidentConsultationBatchCoordinate,
        candidate: ConsultationChildDispatchCandidate,
        dispatch: ResidentOperationTaskKey,
        work: Work,
        executor: &mut TaskExecutor,
    ) -> Result<ResidentOperationTaskKey, ResidentConsultationTaskRefusal<Work>>
    where
        TaskExecutor: ResidentOperationTaskExecutor<Work, Output>,
    {
        self.batch_mut(coordinate)
            .expect("the hostile test batch exists")
            .try_accept_retained(
                RetainedConsultationTask {
                    basis: ConsultationTaskBasis {
                        candidate,
                        dispatch,
                    },
                    work,
                },
                executor,
            )
    }

    pub(in crate::resident) fn batch_mut(
        &mut self,
        coordinate: &ResidentConsultationBatchCoordinate,
    ) -> Option<&mut ResidentConsultationTaskService> {
        self.batches
            .iter_mut()
            .find(|batch| &batch.coordinate == coordinate)
            .map(|batch| &mut batch.tasks)
    }

    pub(in crate::resident) fn retry_accept<Work, Output, TaskExecutor>(
        &mut self,
        coordinate: &ResidentConsultationBatchCoordinate,
        retry: ResidentConsultationTaskRetry<Work>,
        executor: &mut TaskExecutor,
    ) -> Result<ResidentOperationTaskKey, ResidentConsultationTaskRetryFailure<Work>>
    where
        TaskExecutor: ResidentOperationTaskExecutor<Work, Output>,
    {
        let mut reservations = self.legacy_reservations();
        self.retry_accept_reserved(coordinate, retry, &mut reservations, executor)
    }

    pub(in crate::resident) fn try_completion<Work, Output, TaskExecutor>(
        &mut self,
        executor: &mut TaskExecutor,
    ) -> Option<
        Result<
            ResidentConsultationTaskOwnerPoll<Work, Output>,
            ResidentConsultationTaskOwnerPollError<Work, Output>,
        >,
    >
    where
        TaskExecutor: ResidentOperationTaskExecutor<Work, Output>,
    {
        let completion = executor.try_completion()?;
        let dispatch = match &completion {
            ResidentOperationTaskCompletion::Completed { task, .. } => task.key(),
            ResidentOperationTaskCompletion::Lost { key, .. } => key,
        };
        let mut batch_index = None;
        for (index, batch) in self.batches.iter().enumerate() {
            if batch.tasks.owns_dispatch(dispatch) {
                if batch_index.is_some() {
                    return Some(Err(ResidentConsultationTaskOwnerPollError::Ambiguous(
                        completion,
                    )));
                }
                batch_index = Some(index);
            }
        }
        let Some(batch_index) = batch_index else {
            return Some(Err(ResidentConsultationTaskOwnerPollError::Stale(
                completion,
            )));
        };
        let coordinate = self.batches[batch_index].coordinate.clone();
        let outcome = self.batches[batch_index].tasks.join_completion(completion);
        let (dispatch, kind) = match &outcome {
            ResidentConsultationTaskPoll::Completed { dispatch, .. } => (
                dispatch.clone(),
                ResidentConsultationContinuationKind::Completed,
            ),
            ResidentConsultationTaskPoll::Lost(lost) => (
                lost.dispatch().clone(),
                ResidentConsultationContinuationKind::Lost,
            ),
            ResidentConsultationTaskPoll::Stale(_) => {
                let ResidentConsultationTaskPoll::Stale(completion) = outcome else {
                    unreachable!("the matched outcome is stale")
                };
                return Some(Err(ResidentConsultationTaskOwnerPollError::Stale(
                    completion,
                )));
            }
        };
        Some(Ok(ResidentConsultationTaskOwnerPoll {
            coordinate: coordinate.clone(),
            continuation: ResidentConsultationContinuationToken {
                coordinate,
                dispatch,
                kind,
                settlement: None,
            },
            outcome,
        }))
    }

    pub(in crate::resident) fn resolve_continuation(
        &mut self,
        continuation: ResidentConsultationContinuationToken,
    ) -> ResidentConsultationTaskOwnerResolution {
        if !continuation.coordinate.belongs_to(&self.owner_instance) {
            return ResidentConsultationTaskOwnerResolution::OwnerMismatch(continuation);
        }
        if continuation.dispatch.store() != continuation.coordinate.store() {
            return ResidentConsultationTaskOwnerResolution::BatchNotFound(continuation);
        }
        let coordinate = continuation.coordinate.clone();
        let Some(batch) = self
            .batches
            .iter_mut()
            .find(|batch| batch.coordinate == coordinate)
        else {
            return ResidentConsultationTaskOwnerResolution::BatchNotFound(continuation);
        };
        let resolution = match batch.tasks.resolve_continuation(continuation) {
            ResidentConsultationLocalResolution::Resolved { .. } => {
                ResidentConsultationContinuationResolution::Resolved
            }
            ResidentConsultationLocalResolution::NotPending(continuation) => {
                ResidentConsultationContinuationResolution::NotPending(continuation)
            }
            ResidentConsultationLocalResolution::KindMismatch {
                continuation,
                actual,
            } => ResidentConsultationContinuationResolution::KindMismatch {
                continuation,
                actual,
            },
        };
        ResidentConsultationTaskOwnerResolution::Continuation(resolution)
    }

    pub(in crate::resident) fn resolve_continuation_reserved(
        &mut self,
        continuation: ResidentConsultationContinuationToken,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentConsultationTaskOwnerResolution {
        self.release_proved_continuation_reserved(continuation, reservations)
    }
}

impl<Work> ResidentConsultationTaskRetryFailure<Work> {
    pub(super) fn into_retry(self) -> Option<ResidentConsultationTaskRetry<Work>> {
        match self {
            Self::OwnerMismatch(retry)
            | Self::BatchMismatch(retry)
            | Self::BatchNotFound(retry)
            | Self::ContinuationUnavailable(retry) => Some(*retry),
            Self::Refused(refusal) => (*refusal).into_retry(),
        }
    }
}

pub(super) fn immutable(value: &str) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: ResidentRecordRefV1::parse(value).expect("record reference"),
        record_sha256: ArtifactDigest::hash_bytes(value.as_bytes()),
    }
}

pub(super) fn store(name: &str) -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse(format!("consultation-store-{name}")).expect("store identity"),
        ControlIncarnationIdV1::parse("consultation-incarnation").expect("store incarnation"),
    )
}

pub(super) fn task_key(value: &str) -> ResidentOperationTaskKey {
    ResidentOperationTaskKey::new(store("primary"), immutable(value))
}

pub(super) fn candidate(name: &str) -> ConsultationChildDispatchCandidate {
    ConsultationChildDispatchCandidate {
        slot_id: ConsultationSlotId::parse(format!("slot-{name}")).expect("slot"),
        child: immutable(&format!("operation/{name}")),
        requirement: ConsultationRequirement::Required,
        requirement_evidence: immutable(&format!("requirement/{name}")),
        participant: ParticipantRefV1::Application {
            application_id: format!("participant-{name}"),
        },
        provider_binding: None,
        context: Some(immutable(&format!("context/{name}"))),
    }
}

pub(super) fn batch(
    candidates: Vec<ConsultationChildDispatchCandidate>,
) -> ConsultationChildDispatchBatch {
    batch_at(
        "operation/consult-parent",
        "collection/frontier",
        candidates,
    )
}

pub(super) fn batch_at(
    parent: &str,
    frontier: &str,
    candidates: Vec<ConsultationChildDispatchCandidate>,
) -> ConsultationChildDispatchBatch {
    ConsultationChildDispatchBatch {
        store: store("primary"),
        parent: immutable(parent),
        collection_frontier: immutable(frontier),
        deadline: pale_core::records::UnixTimeV1::new(100).expect("deadline"),
        candidates,
    }
}

pub(super) fn limits(in_flight: usize, members: usize) -> ResidentConsultationTaskLimits {
    ResidentConsultationTaskLimits::new(
        NonZeroUsize::new(in_flight).expect("in-flight bound"),
        NonZeroUsize::new(members).expect("member bound"),
    )
}

pub(super) fn retained(
    candidate: ConsultationChildDispatchCandidate,
    dispatch: &str,
    work: u8,
) -> RetainedConsultationTask<u8> {
    RetainedConsultationTask {
        basis: ConsultationTaskBasis {
            candidate,
            dispatch: task_key(dispatch),
        },
        work,
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub(super) enum ExecutorAcceptance {
    #[default]
    Accept,
    Refuse,
}

#[derive(Default)]
pub(super) struct Executor {
    pub(super) accept_calls: usize,
    pub(super) completion_calls: usize,
    pub(super) acceptance: ExecutorAcceptance,
    pub(super) accepted: Vec<ResidentOperationTask<u8>>,
    pub(super) completions: VecDeque<ResidentOperationTaskCompletion<u8, u16>>,
}

impl Executor {
    pub(super) fn refusing() -> Self {
        Self {
            acceptance: ExecutorAcceptance::Refuse,
            ..Self::default()
        }
    }

    pub(super) fn accept_next(&mut self) {
        self.acceptance = ExecutorAcceptance::Accept;
    }

    pub(super) fn complete(&mut self, index: usize, output: u16) {
        let task = self.accepted.remove(index);
        self.completions
            .push_back(ResidentOperationTaskCompletion::Completed { task, output });
    }

    pub(super) fn lose(&mut self, index: usize) {
        let task = self.accepted.remove(index);
        self.completions.push_back(task.into_lost());
    }
}

impl ResidentOperationTaskExecutor<u8, u16> for Executor {
    fn try_accept(
        &mut self,
        task: ResidentOperationTask<u8>,
    ) -> Result<(), ResidentOperationTask<u8>> {
        self.accept_calls += 1;
        if self.acceptance == ExecutorAcceptance::Refuse {
            Err(task)
        } else {
            self.accepted.push(task);
            Ok(())
        }
    }

    fn try_completion(&mut self) -> Option<ResidentOperationTaskCompletion<u8, u16>> {
        self.completion_calls += 1;
        self.completions.pop_front()
    }
}

pub(super) struct SubstitutingExecutor {
    pub(super) accept_calls: usize,
    pub(super) returned: Option<ResidentOperationTask<u8>>,
    pub(super) retained: Vec<ResidentOperationTask<u8>>,
}

pub(super) struct CountingClock<'count>(pub(super) &'count AtomicUsize);

impl WallClockSourcePort for CountingClock<'_> {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        self.0.fetch_add(1, Ordering::Relaxed);
        Ok(UnixTimeV1::new(1).expect("test time"))
    }
}

impl ResidentOperationTaskExecutor<u8, u16> for SubstitutingExecutor {
    fn try_accept(
        &mut self,
        task: ResidentOperationTask<u8>,
    ) -> Result<(), ResidentOperationTask<u8>> {
        self.accept_calls += 1;
        self.retained.push(task);
        Err(self.returned.take().expect("one substituted task"))
    }

    fn try_completion(&mut self) -> Option<ResidentOperationTaskCompletion<u8, u16>> {
        None
    }
}

pub(super) fn owner_limits(batches: usize) -> ResidentConsultationTaskOwnerLimits {
    ResidentConsultationTaskOwnerLimits::new(
        NonZeroUsize::new(batches).expect("active-batch bound"),
    )
}

pub(super) fn task_state_with_aggregate(maximum_outstanding: usize) -> ResidentOperationTaskState {
    ResidentOperationTaskState::new(
        ResidentOperationTaskLimits::new(NonZeroUsize::new(1).expect("provider-task bound")),
        NonZeroUsize::new(1).expect("consultation-batch bound"),
        ResidentAggregateTaskLimits::new(
            NonZeroUsize::new(maximum_outstanding).expect("aggregate-task bound"),
        ),
    )
    .expect("bounded operation task state")
}

pub(super) fn task_state() -> ResidentOperationTaskState {
    task_state_with_aggregate(8)
}

pub(super) fn operation_host() -> ResidentOperationHost {
    ResidentOperationHost::new(task_state())
}

pub(super) fn host_ports() -> (ProvingAuthority, RecordingStore, RecordingAnchor) {
    (
        ProvingAuthority,
        RecordingStore::committing(OperationCustodyCommitOutcome::Committed),
        RecordingAnchor::unavailable_after_preflight(),
    )
}

pub(super) fn assert_continuation_resolved(result: ResidentConsultationTaskOwnerResolution) {
    assert!(matches!(
        result,
        ResidentConsultationTaskOwnerResolution::Continuation(
            ResidentConsultationContinuationResolution::Resolved
        )
    ));
}
