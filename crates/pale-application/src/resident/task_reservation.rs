//! Aggregate process-local occupancy for resident operation work.
//!
//! This ledger counts provider tasks, A2A delegate sends and recoveries, and
//! outstanding Consult child work against one caller-supplied ceiling. Opening
//! a Consult batch consumes only its separate batch bound. This ledger stores
//! no work or output and grants no replay.

use std::num::NonZeroUsize;

use pale_core::records::{
    A2aTaskBindingDigest, A2aTaskBindingIdV1, A2aTaskBindingV1, OperationHandleDigest,
    ResidentRecordRefV1,
};
use pale_ports::AuthorityStoreCoordinate;

use super::consultation::ResidentConsultationBatchCoordinate;
use super::task_owner::{ResidentOperationTaskKey, ResidentOperationTaskOwnerInstance};

/// Exact positive aggregate occupancy ceiling supplied by the process owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) struct ResidentAggregateTaskLimits {
    maximum_outstanding: NonZeroUsize,
}

impl ResidentAggregateTaskLimits {
    /// Bind the process-local ledger to one explicit positive ceiling.
    #[must_use]
    #[allow(
        dead_code,
        reason = "the production resident process constructor is deliberately not selected yet"
    )]
    pub(in crate::resident) const fn new(maximum_outstanding: NonZeroUsize) -> Self {
        Self {
            maximum_outstanding,
        }
    }

    /// Return the exact aggregate ceiling.
    #[must_use]
    pub(in crate::resident) const fn maximum_outstanding(self) -> NonZeroUsize {
        self.maximum_outstanding
    }
}

/// Typed process-local owner of one aggregate reservation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentTaskReservationRoute {
    /// Provider work retained by the Invoke owner.
    Provider,
    /// One accepted A2A `Delegate` send or its exact restart recovery.
    A2aDelegate,
    /// One authorized A2A `GetTask` read through durable history disposition.
    A2aGetTask,
    /// One authorized A2A `CancelTask` call through durable disposition.
    A2aCancelTask,
    /// Child work retained by one exact finite Consult batch.
    Consultation(ResidentConsultationBatchCoordinate),
}

/// Binding identity shared by remote Task request classes.
///
/// The route class remains separate. This key only prevents two request classes
/// from concurrently acting on the same admitted remote Task binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::resident) struct ResidentA2aTaskWorkKey {
    pub(in crate::resident) store: AuthorityStoreCoordinate,
    pub(in crate::resident) operation_handle_ref: ResidentRecordRefV1,
    pub(in crate::resident) operation_handle_sha256: OperationHandleDigest,
    pub(in crate::resident) task_binding_id: A2aTaskBindingIdV1,
    pub(in crate::resident) task_binding_ref: ResidentRecordRefV1,
    pub(in crate::resident) task_binding_sha256: A2aTaskBindingDigest,
}

impl ResidentA2aTaskWorkKey {
    /// Return whether one admitted binding is the exact binding in this key.
    #[must_use]
    pub(in crate::resident) fn matches_binding(
        &self,
        binding: (&ResidentRecordRefV1, &A2aTaskBindingV1),
    ) -> bool {
        self.task_binding_ref == *binding.0
            && self.task_binding_sha256 == *binding.1.canonical_a2a_task_binding_sha256()
            && self.task_binding_id == *binding.1.task_binding_id()
            && self.operation_handle_ref == *binding.1.operation_handle_ref()
            && self.operation_handle_sha256 == *binding.1.operation_handle_sha256()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ResidentAggregateTaskKey {
    Operation(ResidentOperationTaskKey),
    A2aTask(ResidentA2aTaskWorkKey),
}

/// Process-local phase of one outstanding reservation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentTaskReservationPhase {
    /// An exact task is being offered to its typed executor.
    Offered,
    /// The typed executor may own the task.
    Accepted,
    /// Returned output still requires its typed durable handling.
    CompletionPendingDurableHandling,
    /// Lost work still requires its typed durable recovery.
    LossPendingDurableHandling,
}

struct ResidentTaskReservation {
    key: ResidentAggregateTaskKey,
    route: ResidentTaskReservationRoute,
    phase: ResidentTaskReservationPhase,
}

/// Linear handle for one entry in the transient `Offered` phase.
#[must_use = "settle or return the exact aggregate task offer"]
pub(in crate::resident) struct ResidentTaskReservationOffer {
    owner_instance: ResidentOperationTaskOwnerInstance,
    key: Box<ResidentAggregateTaskKey>,
    route: ResidentTaskReservationRoute,
}

/// Linear handle for one completion or loss awaiting durable handling.
#[must_use = "complete exact durable handling before releasing this reservation"]
pub(in crate::resident) struct ResidentTaskReservationSettlement {
    owner_instance: ResidentOperationTaskOwnerInstance,
    key: Box<ResidentAggregateTaskKey>,
    route: ResidentTaskReservationRoute,
    phase: ResidentTaskReservationPhase,
}

/// Payload-free aggregate task ledger for one process lifetime.
pub(in crate::resident) struct ResidentAggregateTaskReservations {
    owner_instance: ResidentOperationTaskOwnerInstance,
    limits: ResidentAggregateTaskLimits,
    entries: Vec<ResidentTaskReservation>,
}

impl ResidentAggregateTaskReservations {
    /// Reserve the complete aggregate table before any work can be offered.
    pub(in crate::resident) fn new(
        owner_instance: ResidentOperationTaskOwnerInstance,
        limits: ResidentAggregateTaskLimits,
    ) -> Result<Self, ResidentAggregateTaskReservationBuildError> {
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(limits.maximum_outstanding().get())
            .map_err(|_| ResidentAggregateTaskReservationBuildError::Allocation)?;
        Ok(Self {
            owner_instance,
            limits,
            entries,
        })
    }

    /// Return the number of entries that still consume aggregate capacity.
    #[must_use]
    pub(in crate::resident) fn outstanding(&self) -> usize {
        self.entries.len()
    }

    /// Return the aggregate process-local occupancy ceiling.
    #[must_use]
    pub(in crate::resident) const fn limits(&self) -> ResidentAggregateTaskLimits {
        self.limits
    }

    /// Refuse work before any caller-owned lookup, clock, retention, or executor call.
    pub(in crate::resident) fn require_capacity(
        &self,
    ) -> Result<(), ResidentTaskReservationOfferFailure> {
        if self.entries.len() == self.limits.maximum_outstanding().get() {
            Err(ResidentTaskReservationOfferFailure::Saturated)
        } else {
            Ok(())
        }
    }

    /// Retain one exact transient offer after its store-qualified key is known.
    pub(in crate::resident) fn begin_offer(
        &mut self,
        key: ResidentOperationTaskKey,
        route: ResidentTaskReservationRoute,
    ) -> Result<ResidentTaskReservationOffer, ResidentTaskReservationOfferFailure> {
        self.can_begin_offer(&key, &route)?;
        self.begin_offer_inner(ResidentAggregateTaskKey::Operation(key), route)
    }

    /// Purely check one operation offer before caller-owned work is constructed.
    pub(in crate::resident) fn can_begin_offer(
        &self,
        key: &ResidentOperationTaskKey,
        route: &ResidentTaskReservationRoute,
    ) -> Result<(), ResidentTaskReservationOfferFailure> {
        if let ResidentTaskReservationRoute::Consultation(coordinate) = route {
            if !coordinate.belongs_to(&self.owner_instance) {
                return Err(ResidentTaskReservationOfferFailure::OwnerMismatch);
            }
            if coordinate.store() != key.store() {
                return Err(ResidentTaskReservationOfferFailure::StoreMismatch);
            }
        }
        let key = ResidentAggregateTaskKey::Operation(key.clone());
        if self.entries.iter().any(|entry| entry.key == key) {
            return Err(ResidentTaskReservationOfferFailure::Duplicate);
        }
        self.require_capacity()
    }

    /// Retain one remote-Task request class against one exact binding.
    ///
    /// The binding key is independent of the request class. A later request
    /// class therefore cannot acquire the same binding while this one is live.
    pub(in crate::resident) fn begin_a2a_task_offer(
        &mut self,
        key: ResidentA2aTaskWorkKey,
        route: ResidentTaskReservationRoute,
    ) -> Result<ResidentTaskReservationOffer, ResidentA2aTaskReservationOfferFailure> {
        if !matches!(
            route,
            ResidentTaskReservationRoute::A2aGetTask | ResidentTaskReservationRoute::A2aCancelTask
        ) {
            return Err(ResidentA2aTaskReservationOfferFailure::Route);
        }
        self.begin_offer_inner(ResidentAggregateTaskKey::A2aTask(key), route)
            .map_err(ResidentA2aTaskReservationOfferFailure::Offer)
    }

    fn begin_offer_inner(
        &mut self,
        key: ResidentAggregateTaskKey,
        route: ResidentTaskReservationRoute,
    ) -> Result<ResidentTaskReservationOffer, ResidentTaskReservationOfferFailure> {
        if self.entries.iter().any(|entry| entry.key == key) {
            return Err(ResidentTaskReservationOfferFailure::Duplicate);
        }
        self.require_capacity()?;
        self.entries.push(ResidentTaskReservation {
            key: key.clone(),
            route: route.clone(),
            phase: ResidentTaskReservationPhase::Offered,
        });
        Ok(ResidentTaskReservationOffer {
            owner_instance: self.owner_instance.clone(),
            key: Box::new(key),
            route,
        })
    }

    /// Commit an offered reservation after acceptance or executor protocol loss.
    pub(in crate::resident) fn accept_offer(
        &mut self,
        offer: ResidentTaskReservationOffer,
    ) -> Result<(), ResidentTaskReservationOfferTransitionFailure> {
        let result = self.require_owner(&offer.owner_instance).and_then(|()| {
            self.transition(
                &offer.key,
                &offer.route,
                ResidentTaskReservationPhase::Offered,
                ResidentTaskReservationPhase::Accepted,
            )
        });
        result.map_err(|reason| ResidentTaskReservationOfferTransitionFailure {
            offer: Box::new(offer),
            reason,
        })
    }

    /// Retry the exact Offered-to-Accepted transition after mechanical
    /// acceptance already consumed the ordinary call attempt.
    ///
    /// This method neither reconstructs an offer nor changes its route. A
    /// rejected retry returns the same linear offer capability with the latest
    /// transition reason.
    pub(in crate::resident) fn retry_accept_offer(
        &mut self,
        failure: ResidentTaskReservationOfferTransitionFailure,
    ) -> Result<(), ResidentTaskReservationOfferTransitionFailure> {
        self.accept_offer(*failure.offer)
    }

    /// Retry the exact Offered removal after an unopened refusal already won.
    ///
    /// This method neither reconstructs an offer nor changes its route. A
    /// rejected retry returns the same linear offer capability with the latest
    /// transition reason.
    pub(in crate::resident) fn retry_refuse_offer(
        &mut self,
        failure: ResidentTaskReservationOfferTransitionFailure,
    ) -> Result<(), ResidentTaskReservationOfferTransitionFailure> {
        self.refuse_offer(*failure.offer)
    }

    /// Move an offered task directly into completion handling.
    ///
    /// This is used when a carrier start itself returns the terminal local
    /// disposition, leaving no accepted process-local task to poll.
    pub(in crate::resident) fn complete_offer(
        &mut self,
        offer: ResidentTaskReservationOffer,
    ) -> Result<ResidentTaskReservationSettlement, ResidentTaskReservationOfferTransitionFailure>
    {
        let result = self.require_owner(&offer.owner_instance).and_then(|()| {
            self.transition(
                &offer.key,
                &offer.route,
                ResidentTaskReservationPhase::Offered,
                ResidentTaskReservationPhase::CompletionPendingDurableHandling,
            )
        });
        match result {
            Ok(()) => Ok(ResidentTaskReservationSettlement {
                owner_instance: offer.owner_instance,
                key: offer.key,
                route: offer.route,
                phase: ResidentTaskReservationPhase::CompletionPendingDurableHandling,
            }),
            Err(reason) => Err(ResidentTaskReservationOfferTransitionFailure {
                offer: Box::new(offer),
                reason,
            }),
        }
    }

    /// Move a restart-recovered offered task directly into loss handling.
    pub(in crate::resident) fn lose_offer(
        &mut self,
        offer: ResidentTaskReservationOffer,
    ) -> Result<ResidentTaskReservationSettlement, ResidentTaskReservationOfferTransitionFailure>
    {
        let result = self.require_owner(&offer.owner_instance).and_then(|()| {
            self.transition(
                &offer.key,
                &offer.route,
                ResidentTaskReservationPhase::Offered,
                ResidentTaskReservationPhase::LossPendingDurableHandling,
            )
        });
        match result {
            Ok(()) => Ok(ResidentTaskReservationSettlement {
                owner_instance: offer.owner_instance,
                key: offer.key,
                route: offer.route,
                phase: ResidentTaskReservationPhase::LossPendingDurableHandling,
            }),
            Err(reason) => Err(ResidentTaskReservationOfferTransitionFailure {
                offer: Box::new(offer),
                reason,
            }),
        }
    }

    /// Return aggregate capacity after a proven unopened refusal.
    pub(in crate::resident) fn refuse_offer(
        &mut self,
        offer: ResidentTaskReservationOffer,
    ) -> Result<(), ResidentTaskReservationOfferTransitionFailure> {
        let result = self.require_owner(&offer.owner_instance).and_then(|()| {
            self.remove(
                &offer.key,
                &offer.route,
                ResidentTaskReservationPhase::Offered,
            )
        });
        result.map_err(|reason| ResidentTaskReservationOfferTransitionFailure {
            offer: Box::new(offer),
            reason,
        })
    }

    /// Retain accepted occupancy while returned output awaits durable handling.
    pub(in crate::resident) fn mark_completion_pending(
        &mut self,
        key: &ResidentOperationTaskKey,
        route: &ResidentTaskReservationRoute,
    ) -> Result<ResidentTaskReservationSettlement, ResidentTaskReservationTransitionFailure> {
        self.mark_completion_pending_inner(&ResidentAggregateTaskKey::Operation(key.clone()), route)
    }

    /// Hold remote-Task request capacity through durable history handling.
    pub(in crate::resident) fn mark_a2a_task_completion_pending(
        &mut self,
        key: &ResidentA2aTaskWorkKey,
        route: &ResidentTaskReservationRoute,
    ) -> Result<ResidentTaskReservationSettlement, ResidentTaskReservationTransitionFailure> {
        self.mark_completion_pending_inner(&ResidentAggregateTaskKey::A2aTask(key.clone()), route)
    }

    fn mark_completion_pending_inner(
        &mut self,
        key: &ResidentAggregateTaskKey,
        route: &ResidentTaskReservationRoute,
    ) -> Result<ResidentTaskReservationSettlement, ResidentTaskReservationTransitionFailure> {
        self.transition(
            key,
            route,
            ResidentTaskReservationPhase::Accepted,
            ResidentTaskReservationPhase::CompletionPendingDurableHandling,
        )?;
        Ok(ResidentTaskReservationSettlement {
            owner_instance: self.owner_instance.clone(),
            key: Box::new(key.clone()),
            route: route.clone(),
            phase: ResidentTaskReservationPhase::CompletionPendingDurableHandling,
        })
    }

    /// Retain accepted occupancy while lost work awaits durable recovery.
    pub(in crate::resident) fn mark_loss_pending(
        &mut self,
        key: &ResidentOperationTaskKey,
        route: &ResidentTaskReservationRoute,
    ) -> Result<ResidentTaskReservationSettlement, ResidentTaskReservationTransitionFailure> {
        self.transition(
            &ResidentAggregateTaskKey::Operation(key.clone()),
            route,
            ResidentTaskReservationPhase::Accepted,
            ResidentTaskReservationPhase::LossPendingDurableHandling,
        )?;
        Ok(ResidentTaskReservationSettlement {
            owner_instance: self.owner_instance.clone(),
            key: Box::new(ResidentAggregateTaskKey::Operation(key.clone())),
            route: route.clone(),
            phase: ResidentTaskReservationPhase::LossPendingDurableHandling,
        })
    }

    /// Release an exact completion only after its typed durable handler succeeds.
    pub(in crate::resident) fn release_completion(
        &mut self,
        settlement: ResidentTaskReservationSettlement,
    ) -> Result<(), ResidentTaskReservationSettlementTransitionFailure> {
        let result = self
            .require_owner(&settlement.owner_instance)
            .and_then(|()| {
                if settlement.phase
                    != ResidentTaskReservationPhase::CompletionPendingDurableHandling
                {
                    return Err(ResidentTaskReservationTransitionFailure::Phase {
                        expected: ResidentTaskReservationPhase::CompletionPendingDurableHandling,
                        actual: settlement.phase,
                    });
                }
                self.remove(
                    &settlement.key,
                    &settlement.route,
                    ResidentTaskReservationPhase::CompletionPendingDurableHandling,
                )
            });
        result.map_err(
            |reason| ResidentTaskReservationSettlementTransitionFailure {
                settlement: Box::new(settlement),
                reason,
            },
        )
    }

    /// Release an exact loss only after its typed durable recovery succeeds.
    pub(in crate::resident) fn release_loss(
        &mut self,
        settlement: ResidentTaskReservationSettlement,
    ) -> Result<(), ResidentTaskReservationSettlementTransitionFailure> {
        let result = self
            .require_owner(&settlement.owner_instance)
            .and_then(|()| {
                if settlement.phase != ResidentTaskReservationPhase::LossPendingDurableHandling {
                    return Err(ResidentTaskReservationTransitionFailure::Phase {
                        expected: ResidentTaskReservationPhase::LossPendingDurableHandling,
                        actual: settlement.phase,
                    });
                }
                self.remove(
                    &settlement.key,
                    &settlement.route,
                    ResidentTaskReservationPhase::LossPendingDurableHandling,
                )
            });
        result.map_err(
            |reason| ResidentTaskReservationSettlementTransitionFailure {
                settlement: Box::new(settlement),
                reason,
            },
        )
    }

    /// Retry release of one already-recorded provider settlement.
    ///
    /// The failure retains the exact linear settlement. A replacement ledger,
    /// wrong route, or changed phase returns the same capability in a fresh
    /// failure; no provider work or durable recording is repeated.
    pub(in crate::resident) fn retry_provider_release(
        &mut self,
        failure: ResidentTaskReservationSettlementTransitionFailure,
    ) -> Result<(), ResidentTaskReservationSettlementTransitionFailure> {
        let settlement = *failure.settlement;
        if settlement.route != ResidentTaskReservationRoute::Provider {
            return Err(ResidentTaskReservationSettlementTransitionFailure {
                settlement: Box::new(settlement),
                reason: ResidentTaskReservationTransitionFailure::RouteMismatch,
            });
        }
        match settlement.phase {
            ResidentTaskReservationPhase::CompletionPendingDurableHandling => {
                self.release_completion(settlement)
            }
            ResidentTaskReservationPhase::LossPendingDurableHandling => {
                self.release_loss(settlement)
            }
            actual => Err(ResidentTaskReservationSettlementTransitionFailure {
                settlement: Box::new(settlement),
                reason: ResidentTaskReservationTransitionFailure::Phase {
                    expected: ResidentTaskReservationPhase::CompletionPendingDurableHandling,
                    actual,
                },
            }),
        }
    }

    /// Prove that this ledger still owns the exact provider completion.
    ///
    /// This borrowed check performs no mutation and lets a resident service
    /// reject a wrapper from a replacement owner before clock, storage,
    /// signing, anchor, reconciliation, or provider I/O.
    pub(in crate::resident) fn require_provider_completion(
        &self,
        settlement: &ResidentTaskReservationSettlement,
        key: &ResidentOperationTaskKey,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        self.require_provider_settlement(
            settlement,
            key,
            ResidentTaskReservationPhase::CompletionPendingDurableHandling,
        )
    }

    /// Prove that this ledger still owns the exact provider loss.
    pub(in crate::resident) fn require_provider_loss(
        &self,
        settlement: &ResidentTaskReservationSettlement,
        key: &ResidentOperationTaskKey,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        self.require_provider_settlement(
            settlement,
            key,
            ResidentTaskReservationPhase::LossPendingDurableHandling,
        )
    }

    /// Prove that this ledger still owns the exact A2A completion.
    pub(in crate::resident) fn require_a2a_completion(
        &self,
        settlement: &ResidentTaskReservationSettlement,
        key: &ResidentOperationTaskKey,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        self.require_settlement(
            settlement,
            &ResidentAggregateTaskKey::Operation(key.clone()),
            &ResidentTaskReservationRoute::A2aDelegate,
            ResidentTaskReservationPhase::CompletionPendingDurableHandling,
        )
    }

    /// Prove that this ledger still owns the exact unopened A2A offer.
    ///
    /// This borrowed check lets the A2A service reject a capability from a
    /// replacement owner, key, route, or phase before it performs clock,
    /// storage, anchor, or carrier I/O.
    pub(in crate::resident) fn require_a2a_offer(
        &self,
        offer: &ResidentTaskReservationOffer,
        key: &ResidentOperationTaskKey,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        self.require_owner(&offer.owner_instance)?;
        let aggregate_key = ResidentAggregateTaskKey::Operation(key.clone());
        if offer.key.as_ref() != &aggregate_key {
            return Err(ResidentTaskReservationTransitionFailure::NotFound);
        }
        if offer.route != ResidentTaskReservationRoute::A2aDelegate {
            return Err(ResidentTaskReservationTransitionFailure::RouteMismatch);
        }
        let Some(entry) = self.entries.iter().find(|entry| entry.key == aggregate_key) else {
            return Err(ResidentTaskReservationTransitionFailure::NotFound);
        };
        if entry.route != ResidentTaskReservationRoute::A2aDelegate {
            return Err(ResidentTaskReservationTransitionFailure::RouteMismatch);
        }
        if entry.phase != ResidentTaskReservationPhase::Offered {
            return Err(ResidentTaskReservationTransitionFailure::Phase {
                expected: ResidentTaskReservationPhase::Offered,
                actual: entry.phase,
            });
        }
        Ok(())
    }

    /// Prove that this ledger still owns the exact recovered A2A loss.
    pub(in crate::resident) fn require_a2a_loss(
        &self,
        settlement: &ResidentTaskReservationSettlement,
        key: &ResidentOperationTaskKey,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        self.require_settlement(
            settlement,
            &ResidentAggregateTaskKey::Operation(key.clone()),
            &ResidentTaskReservationRoute::A2aDelegate,
            ResidentTaskReservationPhase::LossPendingDurableHandling,
        )
    }

    /// Prove that one completed `GetTask` read still owns aggregate capacity.
    pub(in crate::resident) fn require_a2a_get_task_completion(
        &self,
        settlement: &ResidentTaskReservationSettlement,
        key: &ResidentA2aTaskWorkKey,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        self.require_settlement(
            settlement,
            &ResidentAggregateTaskKey::A2aTask(key.clone()),
            &ResidentTaskReservationRoute::A2aGetTask,
            ResidentTaskReservationPhase::CompletionPendingDurableHandling,
        )
    }

    /// Prove that this ledger still owns the exact unopened `GetTask` offer.
    ///
    /// This check changes no state. The read owner uses it before database,
    /// anchor, or carrier I/O so a phase from a replacement owner cannot write.
    pub(in crate::resident) fn require_a2a_get_task_offer(
        &self,
        offer: &ResidentTaskReservationOffer,
        key: &ResidentA2aTaskWorkKey,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        self.require_owner(&offer.owner_instance)?;
        let aggregate_key = ResidentAggregateTaskKey::A2aTask(key.clone());
        if offer.key.as_ref() != &aggregate_key {
            return Err(ResidentTaskReservationTransitionFailure::NotFound);
        }
        if offer.route != ResidentTaskReservationRoute::A2aGetTask {
            return Err(ResidentTaskReservationTransitionFailure::RouteMismatch);
        }
        let Some(entry) = self.entries.iter().find(|entry| entry.key == aggregate_key) else {
            return Err(ResidentTaskReservationTransitionFailure::NotFound);
        };
        if entry.route != ResidentTaskReservationRoute::A2aGetTask {
            return Err(ResidentTaskReservationTransitionFailure::RouteMismatch);
        }
        if entry.phase != ResidentTaskReservationPhase::Offered {
            return Err(ResidentTaskReservationTransitionFailure::Phase {
                expected: ResidentTaskReservationPhase::Offered,
                actual: entry.phase,
            });
        }
        Ok(())
    }

    /// Prove that this ledger still owns the exact unopened `CancelTask` offer.
    ///
    /// This check changes no state. The cancellation owner uses it before
    /// database or anchor I/O so a phase from a replacement owner cannot write.
    pub(in crate::resident) fn require_a2a_cancel_task_offer(
        &self,
        offer: &ResidentTaskReservationOffer,
        key: &ResidentA2aTaskWorkKey,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        self.require_owner(&offer.owner_instance)?;
        let aggregate_key = ResidentAggregateTaskKey::A2aTask(key.clone());
        if offer.key.as_ref() != &aggregate_key {
            return Err(ResidentTaskReservationTransitionFailure::NotFound);
        }
        if offer.route != ResidentTaskReservationRoute::A2aCancelTask {
            return Err(ResidentTaskReservationTransitionFailure::RouteMismatch);
        }
        let Some(entry) = self.entries.iter().find(|entry| entry.key == aggregate_key) else {
            return Err(ResidentTaskReservationTransitionFailure::NotFound);
        };
        if entry.route != ResidentTaskReservationRoute::A2aCancelTask {
            return Err(ResidentTaskReservationTransitionFailure::RouteMismatch);
        }
        if entry.phase != ResidentTaskReservationPhase::Offered {
            return Err(ResidentTaskReservationTransitionFailure::Phase {
                expected: ResidentTaskReservationPhase::Offered,
                actual: entry.phase,
            });
        }
        Ok(())
    }

    /// Prove that one completed `CancelTask` call still owns aggregate capacity.
    pub(in crate::resident) fn require_a2a_cancel_task_completion(
        &self,
        settlement: &ResidentTaskReservationSettlement,
        key: &ResidentA2aTaskWorkKey,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        self.require_settlement(
            settlement,
            &ResidentAggregateTaskKey::A2aTask(key.clone()),
            &ResidentTaskReservationRoute::A2aCancelTask,
            ResidentTaskReservationPhase::CompletionPendingDurableHandling,
        )
    }

    /// Prove that one exact Consult completion still owns aggregate capacity.
    ///
    /// This borrowed check changes no process-local state. The Consult owner
    /// uses it before collection recovery, signing, storage, or anchor work so
    /// a substituted owner, batch, key, route, or phase cannot reach I/O.
    pub(in crate::resident) fn require_consultation_completion(
        &self,
        settlement: &ResidentTaskReservationSettlement,
        key: &ResidentOperationTaskKey,
        coordinate: &ResidentConsultationBatchCoordinate,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        self.require_settlement(
            settlement,
            &ResidentAggregateTaskKey::Operation(key.clone()),
            &ResidentTaskReservationRoute::Consultation(coordinate.clone()),
            ResidentTaskReservationPhase::CompletionPendingDurableHandling,
        )
    }

    /// Prove that one exact lost Consult task still owns aggregate capacity.
    pub(in crate::resident) fn require_consultation_loss(
        &self,
        settlement: &ResidentTaskReservationSettlement,
        key: &ResidentOperationTaskKey,
        coordinate: &ResidentConsultationBatchCoordinate,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        self.require_settlement(
            settlement,
            &ResidentAggregateTaskKey::Operation(key.clone()),
            &ResidentTaskReservationRoute::Consultation(coordinate.clone()),
            ResidentTaskReservationPhase::LossPendingDurableHandling,
        )
    }

    /// Return whether an exact Consult batch still owns aggregate capacity.
    #[must_use]
    pub(in crate::resident) fn has_batch_reservations(
        &self,
        coordinate: &ResidentConsultationBatchCoordinate,
    ) -> bool {
        self.entries.iter().any(|entry| {
            matches!(
                &entry.route,
                ResidentTaskReservationRoute::Consultation(current) if current == coordinate
            )
        })
    }

    fn transition(
        &mut self,
        key: &ResidentAggregateTaskKey,
        route: &ResidentTaskReservationRoute,
        expected: ResidentTaskReservationPhase,
        next: ResidentTaskReservationPhase,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        let entry = self.entry_mut(key, route)?;
        if entry.phase != expected {
            return Err(ResidentTaskReservationTransitionFailure::Phase {
                expected,
                actual: entry.phase,
            });
        }
        entry.phase = next;
        Ok(())
    }

    fn require_provider_settlement(
        &self,
        settlement: &ResidentTaskReservationSettlement,
        key: &ResidentOperationTaskKey,
        phase: ResidentTaskReservationPhase,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        self.require_settlement(
            settlement,
            &ResidentAggregateTaskKey::Operation(key.clone()),
            &ResidentTaskReservationRoute::Provider,
            phase,
        )
    }

    fn require_settlement(
        &self,
        settlement: &ResidentTaskReservationSettlement,
        key: &ResidentAggregateTaskKey,
        route: &ResidentTaskReservationRoute,
        phase: ResidentTaskReservationPhase,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        self.require_owner(&settlement.owner_instance)?;
        if settlement.key.as_ref() != key {
            return Err(ResidentTaskReservationTransitionFailure::NotFound);
        }
        if &settlement.route != route {
            return Err(ResidentTaskReservationTransitionFailure::RouteMismatch);
        }
        if settlement.phase != phase {
            return Err(ResidentTaskReservationTransitionFailure::Phase {
                expected: phase,
                actual: settlement.phase,
            });
        }
        let Some(entry) = self.entries.iter().find(|entry| &entry.key == key) else {
            return Err(ResidentTaskReservationTransitionFailure::NotFound);
        };
        if &entry.route != route {
            return Err(ResidentTaskReservationTransitionFailure::RouteMismatch);
        }
        if entry.phase != phase {
            return Err(ResidentTaskReservationTransitionFailure::Phase {
                expected: phase,
                actual: entry.phase,
            });
        }
        Ok(())
    }

    fn remove(
        &mut self,
        key: &ResidentAggregateTaskKey,
        route: &ResidentTaskReservationRoute,
        expected: ResidentTaskReservationPhase,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        let Some(index) = self.entries.iter().position(|entry| &entry.key == key) else {
            return Err(ResidentTaskReservationTransitionFailure::NotFound);
        };
        if &self.entries[index].route != route {
            return Err(ResidentTaskReservationTransitionFailure::RouteMismatch);
        }
        if self.entries[index].phase != expected {
            return Err(ResidentTaskReservationTransitionFailure::Phase {
                expected,
                actual: self.entries[index].phase,
            });
        }
        self.entries.swap_remove(index);
        Ok(())
    }

    fn entry_mut(
        &mut self,
        key: &ResidentAggregateTaskKey,
        route: &ResidentTaskReservationRoute,
    ) -> Result<&mut ResidentTaskReservation, ResidentTaskReservationTransitionFailure> {
        let Some(entry) = self.entries.iter_mut().find(|entry| &entry.key == key) else {
            return Err(ResidentTaskReservationTransitionFailure::NotFound);
        };
        if &entry.route != route {
            return Err(ResidentTaskReservationTransitionFailure::RouteMismatch);
        }
        Ok(entry)
    }

    fn require_owner(
        &self,
        owner_instance: &ResidentOperationTaskOwnerInstance,
    ) -> Result<(), ResidentTaskReservationTransitionFailure> {
        if self.owner_instance.is_same(owner_instance) {
            Ok(())
        } else {
            Err(ResidentTaskReservationTransitionFailure::OwnerMismatch)
        }
    }
}

/// Construction stopped before aggregate reservations were usable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentAggregateTaskReservationBuildError {
    /// The complete configured table could not be reserved.
    Allocation,
}

/// Why a new aggregate offer was not retained.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentTaskReservationOfferFailure {
    /// A Consult coordinate belongs to another process-lifetime task state.
    OwnerMismatch,
    /// The task key and Consult batch name different store incarnations.
    StoreMismatch,
    /// The exact store-qualified task already consumes aggregate capacity.
    Duplicate,
    /// The explicit aggregate ceiling is full.
    Saturated,
}

/// Why a remote-Task request could not reserve its distinct typed route.
pub(in crate::resident) enum ResidentA2aTaskReservationOfferFailure {
    /// The caller supplied a non-A2A task class.
    Route,
    /// The shared aggregate owner rejected capacity or identity.
    Offer(ResidentTaskReservationOfferFailure),
}

impl std::fmt::Debug for ResidentA2aTaskReservationOfferFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Route => formatter.write_str("Route"),
            Self::Offer(reason) => formatter.debug_tuple("Offer").field(reason).finish(),
        }
    }
}

/// Why an exact aggregate transition did not occur.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::resident) enum ResidentTaskReservationTransitionFailure {
    /// The capability belongs to another process-lifetime task state.
    OwnerMismatch,
    /// No outstanding reservation has this exact store-qualified key.
    NotFound,
    /// The key exists, but another typed owner owns it.
    RouteMismatch,
    /// The owner exists in a different process-local phase.
    Phase {
        expected: ResidentTaskReservationPhase,
        actual: ResidentTaskReservationPhase,
    },
}

/// An offered capability rejected without losing its exact reservation owner.
#[must_use = "recover the unchanged aggregate offer capability"]
pub(in crate::resident) struct ResidentTaskReservationOfferTransitionFailure {
    pub(in crate::resident) offer: Box<ResidentTaskReservationOffer>,
    pub(in crate::resident) reason: ResidentTaskReservationTransitionFailure,
}

impl ResidentTaskReservationOfferTransitionFailure {
    /// Return the exact `GetTask` binding key retained by this offer.
    #[must_use]
    pub(in crate::resident) fn a2a_get_task_key(&self) -> Option<&ResidentA2aTaskWorkKey> {
        if self.offer.route != ResidentTaskReservationRoute::A2aGetTask {
            return None;
        }
        match self.offer.key.as_ref() {
            ResidentAggregateTaskKey::A2aTask(key) => Some(key),
            ResidentAggregateTaskKey::Operation(_) => None,
        }
    }
    /// Return the exact `CancelTask` binding key retained by this offer.
    #[must_use]
    pub(in crate::resident) fn a2a_cancel_task_key(&self) -> Option<&ResidentA2aTaskWorkKey> {
        if self.offer.route != ResidentTaskReservationRoute::A2aCancelTask {
            return None;
        }
        match self.offer.key.as_ref() {
            ResidentAggregateTaskKey::A2aTask(key) => Some(key),
            ResidentAggregateTaskKey::Operation(_) => None,
        }
    }
}

impl std::fmt::Debug for ResidentTaskReservationOfferTransitionFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _capability = &self.offer;
        formatter
            .debug_struct("ResidentTaskReservationOfferTransitionFailure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

/// A settlement capability rejected without losing its pending reservation.
#[must_use = "recover the unchanged aggregate settlement capability"]
pub(in crate::resident) struct ResidentTaskReservationSettlementTransitionFailure {
    pub(in crate::resident) settlement: Box<ResidentTaskReservationSettlement>,
    pub(in crate::resident) reason: ResidentTaskReservationTransitionFailure,
}

impl std::fmt::Debug for ResidentTaskReservationSettlementTransitionFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _capability = &self.settlement;
        formatter
            .debug_struct("ResidentTaskReservationSettlementTransitionFailure")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
#[path = "../../tests/internal/resident/task_reservation.rs"]
mod tests;
