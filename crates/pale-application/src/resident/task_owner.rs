//! Reconstructible process-local ownership of resident operation work.
//!
//! Durable operation custody remains authoritative. This module only retains
//! the application basis for work that a host-supplied mechanical executor has
//! accepted. It selects no thread count, queue, fairness rule, runtime, IPC
//! carrier, provider, or cancellation mechanism.

use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use pale_core::records::ImmutableRecordReferenceV1;
use pale_ports::{AuthorityStoreCoordinate, ProviderInvocationObservation, ProviderInvocationPort};

use super::operation_dispatch::{
    invoke_owned_provider_invocation, OperationAttemptError, OwnedProviderInvocationRequest,
    PreparedProviderOperationCall, PreparedRecoveredProviderOperationCall,
    ProviderOperationCallBasis, ProviderOperationContinuationCoordinate,
};
use super::task_reservation::{
    ResidentAggregateTaskReservations, ResidentTaskReservationOfferFailure,
    ResidentTaskReservationOfferTransitionFailure, ResidentTaskReservationRoute,
    ResidentTaskReservationSettlement, ResidentTaskReservationTransitionFailure,
};
use crate::provider_invocation::ProviderInvocationAdmissionError;

/// Exact positive process-local occupancy ceiling supplied by the resident host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) struct ResidentOperationTaskLimits {
    maximum_in_flight: NonZeroUsize,
}

impl ResidentOperationTaskLimits {
    /// Bind the task owner to one explicit positive occupancy ceiling.
    #[must_use]
    pub(in crate::resident) const fn new(maximum_in_flight: NonZeroUsize) -> Self {
        Self { maximum_in_flight }
    }

    /// Return the exact configured in-flight ceiling.
    #[must_use]
    pub(in crate::resident) const fn maximum_in_flight(self) -> NonZeroUsize {
        self.maximum_in_flight
    }
}

/// Identity of one process-lifetime operation-task state.
///
/// Equality is allocation identity. The marker is neither a durable coordinate
/// nor authority and disappears when its resident task state is dropped.
#[derive(Clone)]
pub(in crate::resident) struct ResidentOperationTaskOwnerInstance(
    Arc<ResidentOperationTaskOwnerInstanceMarker>,
);

struct ResidentOperationTaskOwnerInstanceMarker {
    last_offer_id: AtomicU64,
}

impl ResidentOperationTaskOwnerInstance {
    /// Allocate one identity shared by all task subowners in one state.
    #[must_use]
    pub(in crate::resident) fn new() -> Self {
        Self(Arc::new(ResidentOperationTaskOwnerInstanceMarker {
            last_offer_id: AtomicU64::new(0),
        }))
    }

    /// Return whether two handles name the same process-owner allocation.
    #[must_use]
    pub(in crate::resident) fn is_same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    /// Allocate one offer identity across every subowner in this task state.
    fn next_offer_id(&self) -> Option<ResidentOperationTaskOfferId> {
        let previous = self
            .0
            .last_offer_id
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .ok()?;
        previous
            .checked_add(1)
            .and_then(NonZeroU64::new)
            .map(ResidentOperationTaskOfferId)
    }
}

impl std::fmt::Debug for ResidentOperationTaskOwnerInstance {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResidentOperationTaskOwnerInstance")
            .finish_non_exhaustive()
    }
}

/// Store-qualified identity of one process-local mechanical task.
///
/// A record reference is only meaningful in the exact authority-store
/// incarnation that owns it. This key is private and never serialized.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::resident) struct ResidentOperationTaskKey {
    store: AuthorityStoreCoordinate,
    dispatch: ImmutableRecordReferenceV1,
}

impl ResidentOperationTaskKey {
    /// Bind one durable dispatch reference to its exact physical store.
    #[must_use]
    pub(in crate::resident) const fn new(
        store: AuthorityStoreCoordinate,
        dispatch: ImmutableRecordReferenceV1,
    ) -> Self {
        Self { store, dispatch }
    }

    /// Borrow the exact store incarnation that owns the dispatch.
    #[must_use]
    pub(in crate::resident) const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Borrow the immutable durable dispatch reference.
    #[must_use]
    pub(in crate::resident) const fn dispatch(&self) -> &ImmutableRecordReferenceV1 {
        &self.dispatch
    }
}

/// One mechanical task whose immutable key names its durable dispatch basis.
pub(in crate::resident) struct ResidentOperationTask<Work> {
    owner_instance: ResidentOperationTaskOwnerInstance,
    offer_id: ResidentOperationTaskOfferId,
    key: ResidentOperationTaskKey,
    work: Work,
}

/// Process-local identity of one exact executor offer.
///
/// The monotone value detects an executor that returns a different task with
/// the same durable key. It is neither serialized nor an authority coordinate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) struct ResidentOperationTaskOfferId(NonZeroU64);

impl<Work> ResidentOperationTask<Work> {
    /// Construct work that has not yet been accepted by an executor.
    #[must_use]
    const fn new(
        owner_instance: ResidentOperationTaskOwnerInstance,
        offer_id: ResidentOperationTaskOfferId,
        key: ResidentOperationTaskKey,
        work: Work,
    ) -> Self {
        Self {
            owner_instance,
            offer_id,
            key,
            work,
        }
    }

    /// Borrow the exact durable dispatch coordinate.
    #[must_use]
    pub(in crate::resident) const fn key(&self) -> &ResidentOperationTaskKey {
        &self.key
    }

    /// Borrow the process-owner instance that accepted this task.
    #[must_use]
    pub(in crate::resident) const fn owner_instance(&self) -> &ResidentOperationTaskOwnerInstance {
        &self.owner_instance
    }

    /// Borrow the mechanical work without granting a second acceptance.
    #[must_use]
    pub(in crate::resident) const fn work(&self) -> &Work {
        &self.work
    }

    /// Transform the linear work without changing task ownership or identity.
    #[must_use]
    pub(in crate::resident) fn map_work<Mapped>(
        self,
        map: impl FnOnce(Work) -> Mapped,
    ) -> ResidentOperationTask<Mapped> {
        ResidentOperationTask {
            owner_instance: self.owner_instance,
            offer_id: self.offer_id,
            key: self.key,
            work: map(self.work),
        }
    }

    /// Recover unaccepted work from an executor refusal.
    #[must_use]
    pub(in crate::resident) fn into_parts(self) -> (ResidentOperationTaskKey, Work) {
        (self.key, self.work)
    }

    /// Convert lost accepted work into an owner-bound completion observation.
    #[must_use]
    pub(in crate::resident) fn into_lost<Output>(
        self,
    ) -> ResidentOperationTaskCompletion<Work, Output> {
        ResidentOperationTaskCompletion::Lost {
            owner_instance: self.owner_instance,
            offer_id: self.offer_id,
            key: self.key,
        }
    }
}

/// One non-blocking executor completion observation.
pub(in crate::resident) enum ResidentOperationTaskCompletion<Work, Output> {
    /// The executor returned the exact owned work and its raw output.
    Completed {
        /// Original accepted task.
        task: ResidentOperationTask<Work>,
        /// Untrusted mechanical output.
        output: Output,
    },
    /// The executor lost an accepted task without a trustworthy output.
    Lost {
        /// Process-owner instance that accepted the missing task.
        owner_instance: ResidentOperationTaskOwnerInstance,
        /// Exact process-local offer accepted by the executor.
        offer_id: ResidentOperationTaskOfferId,
        /// Durable dispatch coordinate of the lost task.
        key: ResidentOperationTaskKey,
    },
}

impl<Work, Output> ResidentOperationTaskCompletion<Work, Output> {
    /// Borrow the exact store-qualified dispatch completed by the executor.
    #[must_use]
    pub(in crate::resident) const fn key(&self) -> &ResidentOperationTaskKey {
        match self {
            Self::Completed { task, .. } => task.key(),
            Self::Lost { key, .. } => key,
        }
    }
}

/// Host-supplied non-blocking acceptance and completion boundary.
///
/// `try_accept` must return promptly. Acceptance means the host owns the only
/// ordinary execution attempt; it may not ask Pale to submit the work again.
/// Completion is observational and carries no application authority.
pub(in crate::resident) trait ResidentOperationTaskExecutor<Work, Output> {
    /// Accept work without blocking the resident, or return it unopened.
    fn try_accept(
        &mut self,
        task: ResidentOperationTask<Work>,
    ) -> Result<(), ResidentOperationTask<Work>>;

    /// Poll one completion without waiting.
    fn try_completion(&mut self) -> Option<ResidentOperationTaskCompletion<Work, Output>>;
}

struct ActiveOperationTask<Basis> {
    offer_id: ResidentOperationTaskOfferId,
    key: ResidentOperationTaskKey,
    basis: Basis,
    cancellation: ResidentOperationTaskCancellationState,
}

/// Process-local application basis for mechanically accepted operation work.
///
/// Entries are hints reconstructed from durable operation custody after a
/// restart. Losing this table never grants replay or changes an operation.
pub(in crate::resident) struct ResidentOperationTaskOwner<Basis> {
    owner_instance: ResidentOperationTaskOwnerInstance,
    limits: ResidentOperationTaskLimits,
    active: Vec<ActiveOperationTask<Basis>>,
}

impl<Basis> ResidentOperationTaskOwner<Basis> {
    /// Reserve the complete configured occupancy before accepting any work.
    ///
    /// # Errors
    /// Returns an allocation failure without constructing a partially usable owner.
    pub(in crate::resident) fn new_in(
        owner_instance: ResidentOperationTaskOwnerInstance,
        limits: ResidentOperationTaskLimits,
    ) -> Result<Self, ResidentOperationTaskOwnerBuildError> {
        let mut active = Vec::new();
        active
            .try_reserve_exact(limits.maximum_in_flight.get())
            .map_err(|_| ResidentOperationTaskOwnerBuildError::Allocation)?;
        Ok(Self {
            owner_instance,
            limits,
            active,
        })
    }

    /// Return the exact host-supplied ceiling.
    #[must_use]
    pub(in crate::resident) const fn limits(&self) -> ResidentOperationTaskLimits {
        self.limits
    }

    /// Return current reconstructible occupancy.
    #[must_use]
    pub(in crate::resident) fn in_flight(&self) -> usize {
        self.active.len()
    }

    /// Return whether this resident lifetime owns the exact accepted task.
    #[must_use]
    pub(in crate::resident) fn owns(&self, key: &ResidentOperationTaskKey) -> bool {
        self.active.iter().any(|task| &task.key == key)
    }

    /// Borrow the process-owner instance shared by this task owner.
    #[must_use]
    pub(in crate::resident) const fn owner_instance(&self) -> &ResidentOperationTaskOwnerInstance {
        &self.owner_instance
    }

    /// Offer one operation task to the mechanical executor exactly once.
    ///
    /// Duplicate and saturated work is rejected before the executor is called.
    /// Executor refusal returns the original application basis and unopened work.
    pub(in crate::resident) fn try_accept<Work, Output, Executor>(
        &mut self,
        key: ResidentOperationTaskKey,
        basis: Basis,
        work: Work,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, ResidentOperationTaskRefusal<Basis, Work>>
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        if self.active.iter().any(|task| task.key == key) {
            return Err(ResidentOperationTaskRefusal::Duplicate { basis, work });
        }
        if self.active.len() == self.limits.maximum_in_flight.get() {
            return Err(ResidentOperationTaskRefusal::Saturated { basis, work });
        }

        let Some(offer_id) = self.owner_instance.next_offer_id() else {
            return Err(ResidentOperationTaskRefusal::OfferIdentityExhausted { basis, work });
        };
        let task =
            ResidentOperationTask::new(self.owner_instance.clone(), offer_id, key.clone(), work);
        if let Err(returned) = executor.try_accept(task) {
            if !self.matches_offer(&returned, offer_id, &key) {
                self.active.push(ActiveOperationTask {
                    offer_id,
                    key: key.clone(),
                    basis,
                    cancellation: ResidentOperationTaskCancellationState::Unnotified,
                });
                return Err(ResidentOperationTaskRefusal::ExecutorProtocolLoss {
                    unopened_basis: None,
                    expected_key: key,
                    returned: Box::new(returned),
                });
            }
            let (_, work) = returned.into_parts();
            return Err(ResidentOperationTaskRefusal::Executor { basis, work });
        }
        self.active.push(ActiveOperationTask {
            offer_id,
            key: key.clone(),
            basis,
            cancellation: ResidentOperationTaskCancellationState::Unnotified,
        });
        Ok(key)
    }

    /// Offer work while retaining a different owned basis after acceptance.
    ///
    /// The unopened basis is returned with work on every refusal. Only the
    /// smaller accepted basis enters the process-local task table.
    fn try_accept_with<Work, Output, Executor, UnopenedBasis>(
        &mut self,
        key: ResidentOperationTaskKey,
        accepted_basis: Basis,
        unopened_basis: UnopenedBasis,
        work: Work,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, ResidentOperationTaskRefusal<UnopenedBasis, Work>>
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        if self.active.iter().any(|task| task.key == key) {
            return Err(ResidentOperationTaskRefusal::Duplicate {
                basis: unopened_basis,
                work,
            });
        }
        if self.active.len() == self.limits.maximum_in_flight.get() {
            return Err(ResidentOperationTaskRefusal::Saturated {
                basis: unopened_basis,
                work,
            });
        }

        let Some(offer_id) = self.owner_instance.next_offer_id() else {
            return Err(ResidentOperationTaskRefusal::OfferIdentityExhausted {
                basis: unopened_basis,
                work,
            });
        };
        let task =
            ResidentOperationTask::new(self.owner_instance.clone(), offer_id, key.clone(), work);
        if let Err(returned) = executor.try_accept(task) {
            if !self.matches_offer(&returned, offer_id, &key) {
                self.active.push(ActiveOperationTask {
                    offer_id,
                    key: key.clone(),
                    basis: accepted_basis,
                    cancellation: ResidentOperationTaskCancellationState::Unnotified,
                });
                return Err(ResidentOperationTaskRefusal::ExecutorProtocolLoss {
                    unopened_basis: Some(unopened_basis),
                    expected_key: key,
                    returned: Box::new(returned),
                });
            }
            let (_, work) = returned.into_parts();
            return Err(ResidentOperationTaskRefusal::Executor {
                basis: unopened_basis,
                work,
            });
        }
        self.active.push(ActiveOperationTask {
            offer_id,
            key: key.clone(),
            basis: accepted_basis,
            cancellation: ResidentOperationTaskCancellationState::Unnotified,
        });
        Ok(key)
    }

    fn matches_offer<Work>(
        &self,
        returned: &ResidentOperationTask<Work>,
        offer_id: ResidentOperationTaskOfferId,
        key: &ResidentOperationTaskKey,
    ) -> bool {
        self.owner_instance.is_same(returned.owner_instance())
            && returned.offer_id == offer_id
            && returned.key() == key
    }

    /// Record a process-local cancellation notification for accepted work.
    ///
    /// The caller must establish any durable cancellation head separately.
    /// This method never calls the executor and proves neither durable
    /// cancellation nor that the accepted work stopped.
    #[must_use]
    pub(in crate::resident) fn note_cancellation_notification(
        &mut self,
        key: &ResidentOperationTaskKey,
    ) -> ResidentOperationTaskCancellationNotification {
        let Some(task) = self.active.iter_mut().find(|task| &task.key == key) else {
            return ResidentOperationTaskCancellationNotification::NotAccepted;
        };
        match task.cancellation {
            ResidentOperationTaskCancellationState::Notified => {
                ResidentOperationTaskCancellationNotification::AlreadyNotified
            }
            ResidentOperationTaskCancellationState::Unnotified => {
                task.cancellation = ResidentOperationTaskCancellationState::Notified;
                ResidentOperationTaskCancellationNotification::Notified
            }
        }
    }

    /// Poll one completion and join it only to its exact accepted basis.
    ///
    /// Stale or duplicate completions carry no basis and therefore cannot
    /// advance any operation.
    pub(in crate::resident) fn try_completion<Work, Output, Executor>(
        &mut self,
        executor: &mut Executor,
    ) -> Option<ResidentOperationTaskJoin<Basis, Work, Output>>
    where
        Executor: ResidentOperationTaskExecutor<Work, Output>,
    {
        executor
            .try_completion()
            .map(|completion| self.join_completion(completion))
    }

    pub(in crate::resident) fn join_completion<Work, Output>(
        &mut self,
        completion: ResidentOperationTaskCompletion<Work, Output>,
    ) -> ResidentOperationTaskJoin<Basis, Work, Output> {
        let (owner_instance, offer_id, key) = match &completion {
            ResidentOperationTaskCompletion::Completed { task, .. } => {
                (task.owner_instance(), task.offer_id, task.key())
            }
            ResidentOperationTaskCompletion::Lost {
                owner_instance,
                offer_id,
                key,
            } => (owner_instance, *offer_id, key),
        };
        if !self.owner_instance.is_same(owner_instance) {
            return ResidentOperationTaskJoin::Stale(completion);
        }
        let Some(index) = self
            .active
            .iter()
            .position(|task| task.offer_id == offer_id && &task.key == key)
        else {
            return ResidentOperationTaskJoin::Stale(completion);
        };
        let active = self.active.remove(index);
        match completion {
            ResidentOperationTaskCompletion::Completed { task, output } => {
                ResidentOperationTaskJoin::Completed {
                    basis: active.basis,
                    task,
                    output,
                    cancellation: active.cancellation,
                }
            }
            ResidentOperationTaskCompletion::Lost { key, .. } => ResidentOperationTaskJoin::Lost {
                basis: active.basis,
                key,
                cancellation: active.cancellation,
            },
        }
    }

    /// Join one completion while moving its aggregate reservation to a linear
    /// durable-handling capability before process-local task state is removed.
    pub(in crate::resident) fn join_reserved_completion<Work, Output>(
        &mut self,
        completion: ResidentOperationTaskCompletion<Work, Output>,
        reservations: &mut ResidentAggregateTaskReservations,
        route: ResidentTaskReservationRoute,
    ) -> ResidentReservedOperationTaskJoin<Basis, Work, Output> {
        let (owner_instance, offer_id, key) = match &completion {
            ResidentOperationTaskCompletion::Completed { task, .. } => {
                (task.owner_instance(), task.offer_id, task.key())
            }
            ResidentOperationTaskCompletion::Lost {
                owner_instance,
                offer_id,
                key,
            } => (owner_instance, *offer_id, key),
        };
        if !self.owner_instance.is_same(owner_instance) {
            return ResidentReservedOperationTaskJoin::Stale(completion);
        }
        let Some(index) = self
            .active
            .iter()
            .position(|task| task.offer_id == offer_id && &task.key == key)
        else {
            return ResidentReservedOperationTaskJoin::Stale(completion);
        };
        let settlement = match &completion {
            ResidentOperationTaskCompletion::Completed { .. } => {
                reservations.mark_completion_pending(key, &route)
            }
            ResidentOperationTaskCompletion::Lost { .. } => {
                reservations.mark_loss_pending(key, &route)
            }
        };
        let settlement = match settlement {
            Ok(settlement) => settlement,
            Err(reason) => {
                return ResidentReservedOperationTaskJoin::ReservationRejected {
                    completion,
                    reason,
                };
            }
        };
        let active = self.active.remove(index);
        match completion {
            ResidentOperationTaskCompletion::Completed { task, output } => {
                let key = task.key().clone();
                ResidentReservedOperationTaskJoin::Completed {
                    basis: active.basis,
                    key,
                    task,
                    output,
                    cancellation: active.cancellation,
                    settlement,
                }
            }
            ResidentOperationTaskCompletion::Lost { key, .. } => {
                ResidentReservedOperationTaskJoin::Lost {
                    basis: active.basis,
                    key,
                    cancellation: active.cancellation,
                    settlement,
                }
            }
        }
    }
}

/// Construction stopped before the task owner could accept work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentOperationTaskOwnerBuildError {
    /// The configured occupancy could not be reserved.
    Allocation,
}

/// Why one executor offer did not produce an ordinary accepted-task result.
///
/// Most variants prove that the work remained unopened and return a retry-safe
/// basis. Protocol loss instead keeps the offered task active because the
/// executor may have retained it; only non-replayable mismatch evidence returns.
pub(in crate::resident) enum ResidentOperationTaskRefusal<Basis, Work> {
    /// The exact durable dispatch already has accepted work.
    Duplicate { basis: Basis, work: Work },
    /// The explicit process-local occupancy ceiling is full.
    Saturated { basis: Basis, work: Work },
    /// This owner cannot allocate another exact process-local offer identity.
    OfferIdentityExhausted { basis: Basis, work: Work },
    /// The host executor refused the task without opening it.
    Executor { basis: Basis, work: Work },
    /// The host retained the offered task but returned a different value.
    ExecutorProtocolLoss {
        /// Original unopened basis without the consumed work or continuation.
        unopened_basis: Option<Basis>,
        expected_key: ResidentOperationTaskKey,
        returned: Box<ResidentOperationTask<Work>>,
    },
}

/// Process-local notification that cancellation may race with accepted work.
///
/// This state is not evidence of a durable request, a committed cancellation
/// head, or a stopped provider call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentOperationTaskCancellationNotification {
    /// No task with this exact dispatch key has been accepted.
    NotAccepted,
    /// The process-local notification is now retained beside accepted work.
    Notified,
    /// The same process-local notification was already retained.
    AlreadyNotified,
}

/// Whether process-local accepted work received an advisory cancellation notice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentOperationTaskCancellationState {
    /// No process-local cancellation notice was retained before completion.
    Unnotified,
    /// A process-local cancellation notice was retained before completion.
    Notified,
}

/// Result of joining one mechanical completion to accepted application basis.
pub(in crate::resident) enum ResidentOperationTaskJoin<Basis, Work, Output> {
    /// Exact accepted work returned with untrusted output.
    Completed {
        basis: Basis,
        task: ResidentOperationTask<Work>,
        output: Output,
        cancellation: ResidentOperationTaskCancellationState,
    },
    /// The executor lost accepted work; protected recovery must classify it.
    Lost {
        basis: Basis,
        key: ResidentOperationTaskKey,
        cancellation: ResidentOperationTaskCancellationState,
    },
    /// No active task owns this completion.
    Stale(ResidentOperationTaskCompletion<Work, Output>),
}

/// Result of atomically routing an executor completion into aggregate custody.
pub(in crate::resident) enum ResidentReservedOperationTaskJoin<Basis, Work, Output> {
    /// Exact accepted work returned and its aggregate slot awaits durable handling.
    Completed {
        basis: Basis,
        key: ResidentOperationTaskKey,
        task: ResidentOperationTask<Work>,
        output: Output,
        cancellation: ResidentOperationTaskCancellationState,
        settlement: ResidentTaskReservationSettlement,
    },
    /// Accepted work was lost and its aggregate slot awaits durable recovery.
    Lost {
        basis: Basis,
        key: ResidentOperationTaskKey,
        cancellation: ResidentOperationTaskCancellationState,
        settlement: ResidentTaskReservationSettlement,
    },
    /// No accepted task owns this completion.
    Stale(ResidentOperationTaskCompletion<Work, Output>),
    /// The task remains active because aggregate custody did not transition.
    ReservationRejected {
        completion: ResidentOperationTaskCompletion<Work, Output>,
        reason: ResidentTaskReservationTransitionFailure,
    },
}

type ProviderTaskOutput = Result<ProviderInvocationObservation, ProviderInvocationAdmissionError>;

/// Reconstructible owner of accepted provider work for one resident lifetime.
///
/// Durable dispatch and operation observations remain authoritative. This
/// owner cannot create a call, replay one after restart, or claim that a
/// process-local cancellation notification stopped accepted work.
pub(in crate::resident) struct ResidentProviderTaskOwner {
    tasks: ResidentOperationTaskOwner<ProviderOperationContinuationCoordinate>,
}

impl ResidentProviderTaskOwner {
    /// Reserve the complete configured provider occupancy.
    ///
    /// # Errors
    /// Returns an allocation failure before accepting provider work.
    pub(in crate::resident) fn new(
        owner_instance: ResidentOperationTaskOwnerInstance,
        limits: ResidentOperationTaskLimits,
    ) -> Result<Self, ResidentOperationTaskOwnerBuildError> {
        ResidentOperationTaskOwner::new_in(owner_instance, limits).map(|tasks| Self { tasks })
    }

    /// Return the current process-local provider occupancy.
    #[must_use]
    pub(in crate::resident) fn in_flight(&self) -> usize {
        self.tasks.in_flight()
    }

    /// Return the exact process-local provider occupancy ceiling.
    #[must_use]
    pub(in crate::resident) const fn limits(&self) -> ResidentOperationTaskLimits {
        self.tasks.limits()
    }

    /// Return whether this resident lifetime owns the exact accepted provider task.
    #[must_use]
    pub(in crate::resident) fn owns(&self, key: &ResidentOperationTaskKey) -> bool {
        self.tasks.owns(key)
    }

    /// Borrow the process-owner instance that owns provider retry capabilities.
    #[must_use]
    pub(in crate::resident) const fn owner_instance(&self) -> &ResidentOperationTaskOwnerInstance {
        self.tasks.owner_instance()
    }

    /// Offer one fully preflighted provider call to the host executor.
    ///
    /// Every refusal retains the unopened call. Executor acceptance consumes
    /// the only ordinary call attempt; the resident must not submit it again.
    pub(in crate::resident) fn try_accept<'operation, Executor>(
        &mut self,
        call: PreparedProviderOperationCall<'operation>,
        reservations: &mut ResidentAggregateTaskReservations,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, ResidentProviderTaskRefusal<'operation>>
    where
        Executor: ResidentOperationTaskExecutor<OwnedProviderInvocationRequest, ProviderTaskOutput>,
    {
        let (store, dispatch, continuation, basis, request) = call
            .into_task_parts()
            .map_err(|(call, reason)| ResidentProviderTaskRefusal::Preparation { call, reason })?;
        let key = ResidentOperationTaskKey::new(store, dispatch);
        let offer =
            match reservations.begin_offer(key.clone(), ResidentTaskReservationRoute::Provider) {
                Ok(offer) => offer,
                Err(reason) => {
                    return Err(match reason {
                        ResidentTaskReservationOfferFailure::Duplicate => {
                            ResidentProviderTaskRefusal::Duplicate(Box::new(
                                PreparedProviderOperationCall::from_task_parts(basis, request),
                            ))
                        }
                        ResidentTaskReservationOfferFailure::Saturated => {
                            ResidentProviderTaskRefusal::Saturated {
                                owner_instance: self.tasks.owner_instance().clone(),
                                call: Box::new(PreparedProviderOperationCall::from_task_parts(
                                    basis, request,
                                )),
                            }
                        }
                        ResidentTaskReservationOfferFailure::OwnerMismatch
                        | ResidentTaskReservationOfferFailure::StoreMismatch => {
                            unreachable!("provider reservations have no Consult association")
                        }
                    });
                }
            };
        let result = self
            .tasks
            .try_accept_with(key, continuation, basis, request, executor);
        match result {
            Ok(key) => match reservations.accept_offer(offer) {
                Ok(()) => Ok(key),
                Err(transition) => Err(ResidentProviderTaskRefusal::AggregateTransition {
                    transition,
                    outcome: Box::new(ResidentProviderTaskAggregateTransitionOutcome::Accepted {
                        key,
                    }),
                }),
            },
            Err(refusal) => {
                let protocol_loss = matches!(
                    refusal,
                    ResidentOperationTaskRefusal::ExecutorProtocolLoss { .. }
                );
                let transition = if protocol_loss {
                    reservations.accept_offer(offer)
                } else {
                    reservations.refuse_offer(offer)
                };
                let refusal = match refusal {
                    ResidentOperationTaskRefusal::Duplicate { basis, work } => {
                        ResidentProviderTaskRefusal::Duplicate(Box::new(
                            PreparedProviderOperationCall::from_task_parts(basis, work),
                        ))
                    }
                    ResidentOperationTaskRefusal::Saturated { basis, work } => {
                        ResidentProviderTaskRefusal::Saturated {
                            owner_instance: self.tasks.owner_instance().clone(),
                            call: Box::new(PreparedProviderOperationCall::from_task_parts(
                                basis, work,
                            )),
                        }
                    }
                    ResidentOperationTaskRefusal::OfferIdentityExhausted { basis, work } => {
                        ResidentProviderTaskRefusal::OfferIdentityExhausted(Box::new(
                            PreparedProviderOperationCall::from_task_parts(basis, work),
                        ))
                    }
                    ResidentOperationTaskRefusal::Executor { basis, work } => {
                        ResidentProviderTaskRefusal::Executor {
                            owner_instance: self.tasks.owner_instance().clone(),
                            call: Box::new(PreparedProviderOperationCall::from_task_parts(
                                basis, work,
                            )),
                        }
                    }
                    ResidentOperationTaskRefusal::ExecutorProtocolLoss {
                        unopened_basis,
                        expected_key,
                        returned,
                    } => ResidentProviderTaskRefusal::ExecutorProtocolLoss {
                        unopened_call_basis: unopened_basis.map(Box::new),
                        expected_key,
                        returned,
                    },
                };
                match transition {
                    Ok(()) => Err(refusal),
                    Err(transition) => Err(ResidentProviderTaskRefusal::AggregateTransition {
                        transition,
                        outcome: Box::new(ResidentProviderTaskAggregateTransitionOutcome::Refused(
                            refusal,
                        )),
                    }),
                }
            }
        }
    }

    /// Offer one same-process recovered provider call to the host executor.
    ///
    /// The call exists only after a newly committed recovered dispatch passed
    /// release preflight. Refusal keeps the unopened call; acceptance retains
    /// only its durable continuation coordinate.
    pub(in crate::resident) fn try_accept_recovered<Executor>(
        &mut self,
        call: PreparedRecoveredProviderOperationCall,
        reservations: &mut ResidentAggregateTaskReservations,
        executor: &mut Executor,
    ) -> Result<ResidentOperationTaskKey, ResidentRecoveredProviderTaskRefusal>
    where
        Executor: ResidentOperationTaskExecutor<OwnedProviderInvocationRequest, ProviderTaskOutput>,
    {
        let (store, dispatch, continuation, request) = call.into_task_parts();
        let key = ResidentOperationTaskKey::new(store, dispatch);
        let offer =
            match reservations.begin_offer(key.clone(), ResidentTaskReservationRoute::Provider) {
                Ok(offer) => offer,
                Err(reason) => {
                    let call = Box::new(PreparedRecoveredProviderOperationCall::from_task_parts(
                        continuation,
                        request,
                    ));
                    return Err(match reason {
                        ResidentTaskReservationOfferFailure::Duplicate => {
                            ResidentRecoveredProviderTaskRefusal::Duplicate(call)
                        }
                        ResidentTaskReservationOfferFailure::Saturated => {
                            ResidentRecoveredProviderTaskRefusal::Saturated {
                                owner_instance: self.tasks.owner_instance().clone(),
                                call,
                            }
                        }
                        ResidentTaskReservationOfferFailure::OwnerMismatch
                        | ResidentTaskReservationOfferFailure::StoreMismatch => {
                            unreachable!("provider reservations have no Consult association")
                        }
                    });
                }
            };
        let result =
            self.tasks
                .try_accept_with(key, continuation.clone(), continuation, request, executor);
        match result {
            Ok(key) => match reservations.accept_offer(offer) {
                Ok(()) => Ok(key),
                Err(transition) => Err(ResidentRecoveredProviderTaskRefusal::AggregateTransition {
                    transition,
                    outcome: Box::new(
                        ResidentRecoveredProviderTaskAggregateTransitionOutcome::Accepted { key },
                    ),
                }),
            },
            Err(refusal) => {
                let protocol_loss = matches!(
                    refusal,
                    ResidentOperationTaskRefusal::ExecutorProtocolLoss { .. }
                );
                let transition = if protocol_loss {
                    reservations.accept_offer(offer)
                } else {
                    reservations.refuse_offer(offer)
                };
                let refusal = match refusal {
                    ResidentOperationTaskRefusal::Duplicate { basis, work } => {
                        ResidentRecoveredProviderTaskRefusal::Duplicate(Box::new(
                            PreparedRecoveredProviderOperationCall::from_task_parts(basis, work),
                        ))
                    }
                    ResidentOperationTaskRefusal::Saturated { basis, work } => {
                        ResidentRecoveredProviderTaskRefusal::Saturated {
                            owner_instance: self.tasks.owner_instance().clone(),
                            call: Box::new(
                                PreparedRecoveredProviderOperationCall::from_task_parts(
                                    basis, work,
                                ),
                            ),
                        }
                    }
                    ResidentOperationTaskRefusal::OfferIdentityExhausted { basis, work } => {
                        ResidentRecoveredProviderTaskRefusal::OfferIdentityExhausted(Box::new(
                            PreparedRecoveredProviderOperationCall::from_task_parts(basis, work),
                        ))
                    }
                    ResidentOperationTaskRefusal::Executor { basis, work } => {
                        ResidentRecoveredProviderTaskRefusal::Executor {
                            owner_instance: self.tasks.owner_instance().clone(),
                            call: Box::new(
                                PreparedRecoveredProviderOperationCall::from_task_parts(
                                    basis, work,
                                ),
                            ),
                        }
                    }
                    ResidentOperationTaskRefusal::ExecutorProtocolLoss {
                        unopened_basis,
                        expected_key,
                        returned,
                    } => ResidentRecoveredProviderTaskRefusal::ExecutorProtocolLoss {
                        unopened_continuation: Box::new(unopened_basis),
                        expected_key: Box::new(expected_key),
                        returned,
                    },
                };
                match transition {
                    Ok(()) => Err(refusal),
                    Err(transition) => {
                        Err(ResidentRecoveredProviderTaskRefusal::AggregateTransition {
                            transition,
                            outcome: Box::new(
                                ResidentRecoveredProviderTaskAggregateTransitionOutcome::Refused(
                                    refusal,
                                ),
                            ),
                        })
                    }
                }
            }
        }
    }

    /// Retry only the aggregate Offered-to-Accepted transition after the
    /// executor already consumed the ordinary call attempt.
    ///
    /// The task owner must still contain the exact accepted key. This method
    /// never calls the executor and never reconstructs provider work.
    pub(in crate::resident) fn continue_accepted_aggregate_transition<'operation>(
        &mut self,
        refusal: ResidentProviderTaskRefusal<'operation>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<
        ResidentProviderAcceptedAggregateTransition<'operation>,
        ResidentProviderTaskRefusal<'operation>,
    > {
        let ResidentProviderTaskRefusal::AggregateTransition {
            transition,
            outcome,
        } = refusal
        else {
            return Err(refusal);
        };
        let accepted_key = match outcome.as_ref() {
            ResidentProviderTaskAggregateTransitionOutcome::Accepted { key } => key,
            ResidentProviderTaskAggregateTransitionOutcome::Refused(
                ResidentProviderTaskRefusal::ExecutorProtocolLoss { expected_key, .. },
            ) => expected_key,
            ResidentProviderTaskAggregateTransitionOutcome::Refused(_) => {
                return Err(ResidentProviderTaskRefusal::AggregateTransition {
                    transition,
                    outcome,
                });
            }
        };
        if !self.tasks.owns(accepted_key) {
            return Err(ResidentProviderTaskRefusal::AggregateTransition {
                transition,
                outcome,
            });
        }
        if let Err(transition) = reservations.retry_accept_offer(transition) {
            return Err(ResidentProviderTaskRefusal::AggregateTransition {
                transition,
                outcome,
            });
        }
        match *outcome {
            ResidentProviderTaskAggregateTransitionOutcome::Accepted { key } => {
                Ok(ResidentProviderAcceptedAggregateTransition::Accepted(key))
            }
            ResidentProviderTaskAggregateTransitionOutcome::Refused(refusal) => {
                let key = match &refusal {
                    ResidentProviderTaskRefusal::ExecutorProtocolLoss { expected_key, .. } => {
                        expected_key.clone()
                    }
                    _ => unreachable!("only accepted protocol loss reaches this branch"),
                };
                Ok(
                    ResidentProviderAcceptedAggregateTransition::AcceptedWithFailure {
                        key,
                        refusal: Box::new(refusal),
                    },
                )
            }
        }
    }

    /// Repair only aggregate occupancy after unopened work was refused.
    ///
    /// The original refusal remains intact. This method never calls the
    /// executor and never reconstructs provider work.
    pub(in crate::resident) fn continue_refused_aggregate_transition<'operation>(
        &mut self,
        refusal: ResidentProviderTaskRefusal<'operation>,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> Result<ResidentProviderTaskRefusal<'operation>, ResidentProviderTaskRefusal<'operation>>
    {
        let ResidentProviderTaskRefusal::AggregateTransition {
            transition,
            outcome,
        } = refusal
        else {
            return Err(refusal);
        };
        let original = match *outcome {
            ResidentProviderTaskAggregateTransitionOutcome::Refused(original) => original,
            accepted @ ResidentProviderTaskAggregateTransitionOutcome::Accepted { .. } => {
                return Err(ResidentProviderTaskRefusal::AggregateTransition {
                    transition,
                    outcome: Box::new(accepted),
                });
            }
        };
        if provider_refusal_has_accepted_work(&original) {
            return Err(ResidentProviderTaskRefusal::AggregateTransition {
                transition,
                outcome: Box::new(ResidentProviderTaskAggregateTransitionOutcome::Refused(
                    original,
                )),
            });
        }
        match reservations.retry_refuse_offer(transition) {
            Ok(()) => Ok(original),
            Err(transition) => Err(ResidentProviderTaskRefusal::AggregateTransition {
                transition,
                outcome: Box::new(ResidentProviderTaskAggregateTransitionOutcome::Refused(
                    original,
                )),
            }),
        }
    }

    /// Record a process-local cancellation notification without stopping work.
    ///
    /// The caller must separately prove any durable cancellation head.
    #[must_use]
    pub(in crate::resident) fn note_cancellation_notification(
        &mut self,
        key: &ResidentOperationTaskKey,
    ) -> ResidentOperationTaskCancellationNotification {
        self.tasks.note_cancellation_notification(key)
    }

    /// Poll and join one provider completion to its exact accepted basis.
    pub(in crate::resident) fn try_completion<Executor>(
        &mut self,
        reservations: &mut ResidentAggregateTaskReservations,
        executor: &mut Executor,
    ) -> Option<ResidentProviderTaskJoin>
    where
        Executor: ResidentOperationTaskExecutor<OwnedProviderInvocationRequest, ProviderTaskOutput>,
    {
        executor
            .try_completion()
            .map(|completion| self.join_completion(completion, reservations))
    }

    /// Retry only the aggregate join for an already-returned provider completion.
    ///
    /// This path never polls or invokes the executor again.
    pub(in crate::resident) fn retry_completion(
        &mut self,
        completion: ResidentOperationTaskCompletion<
            OwnedProviderInvocationRequest,
            ProviderTaskOutput,
        >,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentProviderTaskJoin {
        self.join_completion(completion, reservations)
    }

    fn join_completion(
        &mut self,
        completion: ResidentOperationTaskCompletion<
            OwnedProviderInvocationRequest,
            ProviderTaskOutput,
        >,
        reservations: &mut ResidentAggregateTaskReservations,
    ) -> ResidentProviderTaskJoin {
        match self.tasks.join_reserved_completion(
            completion,
            reservations,
            ResidentTaskReservationRoute::Provider,
        ) {
            ResidentReservedOperationTaskJoin::Completed {
                basis,
                key,
                task,
                output,
                cancellation,
                settlement,
            } => ResidentProviderTaskJoin::Completed {
                continuation: basis,
                key,
                request: Box::new(task.into_parts().1),
                output,
                cancellation,
                settlement,
            },
            ResidentReservedOperationTaskJoin::Lost {
                basis,
                key,
                cancellation,
                settlement,
            } => ResidentProviderTaskJoin::Lost {
                continuation: basis,
                key,
                cancellation,
                settlement,
            },
            ResidentReservedOperationTaskJoin::Stale(completion) => {
                ResidentProviderTaskJoin::Stale(completion)
            }
            ResidentReservedOperationTaskJoin::ReservationRejected { completion, reason } => {
                ResidentProviderTaskJoin::ReservationRejected { completion, reason }
            }
        }
    }
}

/// Run the one mechanical provider call contained in accepted work.
///
/// Host executors call this away from the resident's single-writer path. The
/// returned observation remains untrusted until the application records it.
pub(in crate::resident) fn execute_provider_task<Provider: ProviderInvocationPort>(
    task: &ResidentOperationTask<OwnedProviderInvocationRequest>,
    provider: &Provider,
) -> ProviderTaskOutput {
    invoke_owned_provider_invocation(task.work(), provider)
}

/// Why a fully preflighted provider call was not accepted.
pub(in crate::resident) enum ResidentProviderTaskRefusal<'operation> {
    /// The call could not produce its exact durable task key.
    Preparation {
        call: Box<PreparedProviderOperationCall<'operation>>,
        reason: OperationAttemptError,
    },
    /// The same durable dispatch already has accepted work.
    Duplicate(Box<PreparedProviderOperationCall<'operation>>),
    /// The explicit process-local occupancy ceiling is full.
    Saturated {
        owner_instance: ResidentOperationTaskOwnerInstance,
        call: Box<PreparedProviderOperationCall<'operation>>,
    },
    /// The host executor refused unopened work.
    Executor {
        owner_instance: ResidentOperationTaskOwnerInstance,
        call: Box<PreparedProviderOperationCall<'operation>>,
    },
    /// This owner exhausted its process-local executor-offer identity space.
    OfferIdentityExhausted(Box<PreparedProviderOperationCall<'operation>>),
    /// The executor retained the offered call but returned a different task.
    ExecutorProtocolLoss {
        /// The original call basis, without its consumed continuation or request.
        unopened_call_basis: Option<Box<ProviderOperationCallBasis<'operation>>>,
        expected_key: ResidentOperationTaskKey,
        returned: Box<ResidentOperationTask<OwnedProviderInvocationRequest>>,
    },
    /// The mechanical outcome is known, but aggregate occupancy did not make
    /// the matching linear transition. The intact capability remains here.
    AggregateTransition {
        transition: ResidentTaskReservationOfferTransitionFailure,
        outcome: Box<ResidentProviderTaskAggregateTransitionOutcome<'operation>>,
    },
}

/// Why a same-process recovered provider call was not accepted.
pub(in crate::resident) enum ResidentRecoveredProviderTaskRefusal {
    Duplicate(Box<PreparedRecoveredProviderOperationCall>),
    Saturated {
        owner_instance: ResidentOperationTaskOwnerInstance,
        call: Box<PreparedRecoveredProviderOperationCall>,
    },
    Executor {
        owner_instance: ResidentOperationTaskOwnerInstance,
        call: Box<PreparedRecoveredProviderOperationCall>,
    },
    OfferIdentityExhausted(Box<PreparedRecoveredProviderOperationCall>),
    ExecutorProtocolLoss {
        unopened_continuation: Box<Option<ProviderOperationContinuationCoordinate>>,
        expected_key: Box<ResidentOperationTaskKey>,
        returned: Box<ResidentOperationTask<OwnedProviderInvocationRequest>>,
    },
    AggregateTransition {
        transition: ResidentTaskReservationOfferTransitionFailure,
        outcome: Box<ResidentRecoveredProviderTaskAggregateTransitionOutcome>,
    },
}

pub(in crate::resident) enum ResidentRecoveredProviderTaskAggregateTransitionOutcome {
    Accepted { key: ResidentOperationTaskKey },
    Refused(ResidentRecoveredProviderTaskRefusal),
}

/// Mechanical provider outcome retained after an aggregate transition fault.
pub(in crate::resident) enum ResidentProviderTaskAggregateTransitionOutcome<'operation> {
    /// The executor may own the exact task; it must never be offered again.
    Accepted { key: ResidentOperationTaskKey },
    /// The ordinary refusal still owns all retry or mismatch evidence.
    Refused(ResidentProviderTaskRefusal<'operation>),
}

/// Accepted provider work after its exact aggregate transition is repaired.
pub(in crate::resident) enum ResidentProviderAcceptedAggregateTransition<'operation> {
    /// Ordinary executor acceptance and aggregate occupancy now agree.
    Accepted(ResidentOperationTaskKey),
    /// Aggregate occupancy is repaired, but executor protocol loss remains.
    AcceptedWithFailure {
        key: ResidentOperationTaskKey,
        refusal: Box<ResidentProviderTaskRefusal<'operation>>,
    },
}

impl ResidentProviderTaskRefusal<'_> {
    /// Return whether a retry-safe refusal belongs to this process owner.
    #[must_use]
    pub(in crate::resident) fn retry_belongs_to(
        &self,
        expected: &ResidentOperationTaskOwnerInstance,
    ) -> bool {
        match self {
            Self::Saturated { owner_instance, .. } | Self::Executor { owner_instance, .. } => {
                owner_instance.is_same(expected)
            }
            Self::Preparation { .. }
            | Self::Duplicate(_)
            | Self::OfferIdentityExhausted(_)
            | Self::ExecutorProtocolLoss { .. }
            | Self::AggregateTransition { .. } => false,
        }
    }
}

fn provider_refusal_has_accepted_work(refusal: &ResidentProviderTaskRefusal<'_>) -> bool {
    match refusal {
        ResidentProviderTaskRefusal::ExecutorProtocolLoss { .. } => true,
        ResidentProviderTaskRefusal::AggregateTransition { outcome, .. } => {
            match outcome.as_ref() {
                ResidentProviderTaskAggregateTransitionOutcome::Accepted { .. } => true,
                ResidentProviderTaskAggregateTransitionOutcome::Refused(refusal) => {
                    provider_refusal_has_accepted_work(refusal)
                }
            }
        }
        ResidentProviderTaskRefusal::Preparation { .. }
        | ResidentProviderTaskRefusal::Duplicate(_)
        | ResidentProviderTaskRefusal::Saturated { .. }
        | ResidentProviderTaskRefusal::Executor { .. }
        | ResidentProviderTaskRefusal::OfferIdentityExhausted(_) => false,
    }
}

/// One provider completion joined to process-local application basis.
pub(in crate::resident) enum ResidentProviderTaskJoin {
    /// Exact accepted work returned with an untrusted provider observation.
    Completed {
        continuation: ProviderOperationContinuationCoordinate,
        key: ResidentOperationTaskKey,
        request: Box<OwnedProviderInvocationRequest>,
        output: ProviderTaskOutput,
        cancellation: ResidentOperationTaskCancellationState,
        settlement: ResidentTaskReservationSettlement,
    },
    /// Accepted work disappeared without a trustworthy output.
    Lost {
        continuation: ProviderOperationContinuationCoordinate,
        key: ResidentOperationTaskKey,
        cancellation: ResidentOperationTaskCancellationState,
        settlement: ResidentTaskReservationSettlement,
    },
    /// A completion had no accepted basis in this resident lifetime.
    Stale(ResidentOperationTaskCompletion<OwnedProviderInvocationRequest, ProviderTaskOutput>),
    /// The accepted task remains owned because aggregate custody did not advance.
    ReservationRejected {
        completion:
            ResidentOperationTaskCompletion<OwnedProviderInvocationRequest, ProviderTaskOutput>,
        reason: ResidentTaskReservationTransitionFailure,
    },
}

#[cfg(test)]
#[path = "../../tests/internal/resident/task_owner.rs"]
pub(in crate::resident) mod tests;
