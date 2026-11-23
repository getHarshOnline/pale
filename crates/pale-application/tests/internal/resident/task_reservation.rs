//! Behavioral laws for aggregate resident task reservations.

use super::*;
use pale_core::records::{
    A2aTaskBindingDigest, A2aTaskBindingIdV1, ArtifactDigest, ControlIncarnationIdV1,
    ControlStoreIdV1, ImmutableRecordReferenceV1, OperationHandleDigest, ResidentRecordRefV1,
};
use pale_ports::AuthorityStoreCoordinate;

fn a2a_task_key(store: AuthorityStoreCoordinate, name: &str) -> ResidentA2aTaskWorkKey {
    let digest = ArtifactDigest::hash_bytes(name.as_bytes()).to_string();
    ResidentA2aTaskWorkKey {
        store,
        operation_handle_ref: ResidentRecordRefV1::parse(format!("operation/{name}"))
            .expect("operation reference"),
        operation_handle_sha256: OperationHandleDigest::parse(digest.clone())
            .expect("operation digest"),
        task_binding_id: A2aTaskBindingIdV1::parse(format!("task-binding-{name}"))
            .expect("task-binding identity"),
        task_binding_ref: ResidentRecordRefV1::parse(format!("task-binding/{name}"))
            .expect("task-binding reference"),
        task_binding_sha256: A2aTaskBindingDigest::parse(digest).expect("task-binding digest"),
    }
}

impl ResidentAggregateTaskReservations {
    pub(in crate::resident) fn set_a2a_task_phase_for_test(
        &mut self,
        key: &ResidentA2aTaskWorkKey,
        route: &ResidentTaskReservationRoute,
        phase: ResidentTaskReservationPhase,
    ) {
        self.entry_mut(&ResidentAggregateTaskKey::A2aTask(key.clone()), route)
            .expect("hostile fixture must name an existing A2A task reservation")
            .phase = phase;
    }

    pub(in crate::resident) fn get_task_release_failure_for_test(
        &mut self,
        key: ResidentA2aTaskWorkKey,
    ) -> ResidentTaskReservationSettlementTransitionFailure {
        let route = ResidentTaskReservationRoute::A2aGetTask;
        let offer = self
            .begin_a2a_task_offer(key.clone(), route.clone())
            .expect("hostile fixture needs a distinct GetTask reservation");
        self.accept_offer(offer)
            .expect("hostile fixture accepts its exact GetTask offer");
        let settlement = self
            .mark_a2a_task_completion_pending(&key, &route)
            .expect("hostile fixture retains completion capacity");
        let mut replacement = ResidentAggregateTaskReservations::new(
            ResidentOperationTaskOwnerInstance::new(),
            self.limits,
        )
        .expect("replacement hostile ledger");
        replacement
            .release_completion(settlement)
            .expect_err("a replacement ledger must preserve the exact settlement")
    }

    pub(in crate::resident) fn retry_get_task_release_for_test(
        &mut self,
        failure: ResidentTaskReservationSettlementTransitionFailure,
    ) {
        self.release_completion(*failure.settlement)
            .expect("the original ledger releases the restored exact settlement");
    }

    pub(in crate::resident) fn a2a_offer_for_test(
        &self,
        key: &ResidentOperationTaskKey,
    ) -> ResidentTaskReservationOffer {
        ResidentTaskReservationOffer {
            owner_instance: self.owner_instance.clone(),
            key: Box::new(ResidentAggregateTaskKey::Operation(key.clone())),
            route: ResidentTaskReservationRoute::A2aDelegate,
        }
    }

    pub(in crate::resident) fn set_phase_for_test(
        &mut self,
        key: &ResidentOperationTaskKey,
        route: &ResidentTaskReservationRoute,
        phase: ResidentTaskReservationPhase,
    ) {
        self.entry_mut(&ResidentAggregateTaskKey::Operation(key.clone()), route)
            .expect("hostile fixture must name an existing reservation")
            .phase = phase;
    }

    pub(in crate::resident) fn replace_key_for_test(
        &mut self,
        current: &ResidentOperationTaskKey,
        replacement: ResidentOperationTaskKey,
    ) {
        self.entries
            .iter_mut()
            .find(|entry| entry.key == ResidentAggregateTaskKey::Operation(current.clone()))
            .expect("hostile fixture must name an existing reservation")
            .key = ResidentAggregateTaskKey::Operation(replacement);
    }

    pub(in crate::resident) fn replace_route_for_test(
        &mut self,
        key: &ResidentOperationTaskKey,
        replacement: ResidentTaskReservationRoute,
    ) {
        self.entries
            .iter_mut()
            .find(|entry| entry.key == ResidentAggregateTaskKey::Operation(key.clone()))
            .expect("hostile fixture must name an existing reservation")
            .route = replacement;
    }

    pub(in crate::resident) fn accepted_provider_offer_failure_for_test(
        &mut self,
        key: &ResidentOperationTaskKey,
    ) -> ResidentTaskReservationOfferTransitionFailure {
        let route = ResidentTaskReservationRoute::Provider;
        let offer = ResidentTaskReservationOffer {
            owner_instance: self.owner_instance.clone(),
            key: Box::new(ResidentAggregateTaskKey::Operation(key.clone())),
            route: route.clone(),
        };
        let failure = self
            .accept_offer(offer)
            .expect_err("accepted fixture phase rejects a second acceptance");
        self.set_phase_for_test(key, &route, ResidentTaskReservationPhase::Offered);
        failure
    }

    pub(in crate::resident) fn refused_provider_offer_failure_for_test(
        &mut self,
        key: &ResidentOperationTaskKey,
    ) -> ResidentTaskReservationOfferTransitionFailure {
        let route = ResidentTaskReservationRoute::Provider;
        let offer = self
            .begin_offer(key.clone(), route.clone())
            .expect("hostile fixture needs one aggregate offer");
        self.set_phase_for_test(key, &route, ResidentTaskReservationPhase::Accepted);
        let failure = self
            .refuse_offer(offer)
            .expect_err("accepted fixture phase rejects refusal");
        self.set_phase_for_test(key, &route, ResidentTaskReservationPhase::Offered);
        failure
    }

    pub(in crate::resident) fn provider_release_failure_for_test(
        &mut self,
        key: &ResidentOperationTaskKey,
    ) -> ResidentTaskReservationSettlementTransitionFailure {
        let route = ResidentTaskReservationRoute::Provider;
        let offer = self
            .begin_offer(key.clone(), route.clone())
            .expect("hostile fixture needs one provider reservation");
        self.accept_offer(offer)
            .expect("hostile fixture must accept its own provider reservation");
        let settlement = self
            .mark_completion_pending(key, &route)
            .expect("hostile fixture must enter provider completion handling");
        let mut replacement = ResidentAggregateTaskReservations::new(
            ResidentOperationTaskOwnerInstance::new(),
            self.limits,
        )
        .expect("replacement hostile ledger");
        replacement
            .release_completion(settlement)
            .expect_err("replacement owner must reject the exact provider settlement")
    }
}

fn limit(value: usize) -> ResidentAggregateTaskLimits {
    ResidentAggregateTaskLimits::new(
        NonZeroUsize::new(value).expect("positive aggregate test limit"),
    )
}

fn store(name: &str) -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse(format!("store-{name}")).expect("store identity"),
        ControlIncarnationIdV1::parse("incarnation-1").expect("store incarnation"),
    )
}

fn reference(name: &str) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: ResidentRecordRefV1::parse(name).expect("record reference"),
        record_sha256: ArtifactDigest::hash_bytes(name.as_bytes()),
    }
}

fn key(store: AuthorityStoreCoordinate, name: &str) -> ResidentOperationTaskKey {
    ResidentOperationTaskKey::new(store, reference(name))
}

#[test]
fn offer_capability_cannot_cross_identical_replacement_ledgers() {
    let first_owner = ResidentOperationTaskOwnerInstance::new();
    let second_owner = ResidentOperationTaskOwnerInstance::new();
    let task_key = key(store("primary"), "reservation/replacement");
    let mut first =
        ResidentAggregateTaskReservations::new(first_owner, limit(1)).expect("first ledger");
    let mut second =
        ResidentAggregateTaskReservations::new(second_owner, limit(1)).expect("second ledger");
    let offer = first
        .begin_offer(task_key, ResidentTaskReservationRoute::Provider)
        .expect("first offer");

    let failure = second
        .accept_offer(offer)
        .expect_err("a replacement ledger must reject the capability");
    assert_eq!(
        failure.reason,
        ResidentTaskReservationTransitionFailure::OwnerMismatch
    );
    assert_eq!(first.outstanding(), 1);
    assert_eq!(second.outstanding(), 0);
    first
        .accept_offer(*failure.offer)
        .expect("the original capability remains usable by its owner");
    assert_eq!(first.outstanding(), 1);
}

#[test]
fn failed_phase_transition_returns_the_same_linear_offer() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let task_key = key(store("primary"), "reservation/phase");
    let route = ResidentTaskReservationRoute::Provider;
    let mut reservations = ResidentAggregateTaskReservations::new(owner, limit(1)).expect("ledger");
    let offer = reservations
        .begin_offer(task_key.clone(), route.clone())
        .expect("offer");
    reservations.set_phase_for_test(&task_key, &route, ResidentTaskReservationPhase::Accepted);

    let failure = reservations
        .accept_offer(offer)
        .expect_err("the wrong phase must reject the transition");
    assert!(matches!(
        failure.reason,
        ResidentTaskReservationTransitionFailure::Phase {
            expected: ResidentTaskReservationPhase::Offered,
            actual: ResidentTaskReservationPhase::Accepted,
        }
    ));
    assert_eq!(reservations.outstanding(), 1);
    reservations.set_phase_for_test(&task_key, &route, ResidentTaskReservationPhase::Offered);
    reservations
        .accept_offer(*failure.offer)
        .expect("the returned capability remains usable");
}

#[test]
fn consult_route_rejects_foreign_owner_and_store_before_insertion() {
    let first_owner = ResidentOperationTaskOwnerInstance::new();
    let second_owner = ResidentOperationTaskOwnerInstance::new();
    let primary = store("primary");
    let coordinate = ResidentConsultationBatchCoordinate::hostile_fixture(
        first_owner.clone(),
        primary.clone(),
        reference("consult/parent"),
        reference("consult/frontier"),
    );
    let mut foreign =
        ResidentAggregateTaskReservations::new(second_owner, limit(1)).expect("foreign ledger");
    assert!(matches!(
        foreign.begin_offer(
            key(primary.clone(), "consult/child"),
            ResidentTaskReservationRoute::Consultation(coordinate.clone()),
        ),
        Err(ResidentTaskReservationOfferFailure::OwnerMismatch)
    ));
    assert_eq!(foreign.outstanding(), 0);

    let mut owning =
        ResidentAggregateTaskReservations::new(first_owner, limit(1)).expect("owning ledger");
    assert!(matches!(
        owning.begin_offer(
            key(store("other"), "consult/child"),
            ResidentTaskReservationRoute::Consultation(coordinate),
        ),
        Err(ResidentTaskReservationOfferFailure::StoreMismatch)
    ));
    assert_eq!(owning.outstanding(), 0);
}

#[test]
fn settlement_capability_cannot_release_a_replacement_ledger() {
    let first_owner = ResidentOperationTaskOwnerInstance::new();
    let second_owner = ResidentOperationTaskOwnerInstance::new();
    let task_key = key(store("primary"), "reservation/settlement");
    let route = ResidentTaskReservationRoute::Provider;
    let mut first =
        ResidentAggregateTaskReservations::new(first_owner, limit(1)).expect("first ledger");
    let mut second =
        ResidentAggregateTaskReservations::new(second_owner, limit(1)).expect("second ledger");
    let offer = first
        .begin_offer(task_key.clone(), route.clone())
        .expect("offer");
    first.accept_offer(offer).expect("accept offer");
    let settlement = first
        .mark_completion_pending(&task_key, &route)
        .expect("completion settlement");

    let failure = second
        .release_completion(settlement)
        .expect_err("a replacement ledger must reject settlement");
    assert_eq!(
        failure.reason,
        ResidentTaskReservationTransitionFailure::OwnerMismatch
    );
    assert_eq!(first.outstanding(), 1);
    assert_eq!(second.outstanding(), 0);
    first
        .release_completion(*failure.settlement)
        .expect("the original ledger releases the returned capability");
    assert_eq!(first.outstanding(), 0);
}

#[test]
fn provider_settlement_validation_is_borrowed_exact_and_releasable_after_repair() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let primary = store("primary");
    let task_key = key(primary.clone(), "reservation/provider-settlement");
    let route = ResidentTaskReservationRoute::Provider;
    let mut reservations =
        ResidentAggregateTaskReservations::new(owner.clone(), limit(1)).expect("ledger");
    let offer = reservations
        .begin_offer(task_key.clone(), route.clone())
        .expect("provider offer");
    reservations.accept_offer(offer).expect("accept offer");
    let settlement = reservations
        .mark_completion_pending(&task_key, &route)
        .expect("completion settlement");

    let replacement =
        ResidentAggregateTaskReservations::new(ResidentOperationTaskOwnerInstance::new(), limit(1))
            .expect("replacement ledger");
    assert_eq!(
        replacement.require_provider_completion(&settlement, &task_key),
        Err(ResidentTaskReservationTransitionFailure::OwnerMismatch)
    );
    assert_eq!(
        reservations.require_provider_completion(
            &settlement,
            &key(primary.clone(), "reservation/other-provider"),
        ),
        Err(ResidentTaskReservationTransitionFailure::NotFound)
    );

    let consultation_route = ResidentTaskReservationRoute::Consultation(
        ResidentConsultationBatchCoordinate::hostile_fixture(
            owner,
            primary,
            reference("consult/parent"),
            reference("consult/frontier"),
        ),
    );
    reservations.replace_route_for_test(&task_key, consultation_route);
    assert_eq!(
        reservations.require_provider_completion(&settlement, &task_key),
        Err(ResidentTaskReservationTransitionFailure::RouteMismatch)
    );
    reservations.replace_route_for_test(&task_key, route.clone());

    reservations.set_phase_for_test(&task_key, &route, ResidentTaskReservationPhase::Accepted);
    assert_eq!(
        reservations.require_provider_completion(&settlement, &task_key),
        Err(ResidentTaskReservationTransitionFailure::Phase {
            expected: ResidentTaskReservationPhase::CompletionPendingDurableHandling,
            actual: ResidentTaskReservationPhase::Accepted,
        })
    );
    reservations.set_phase_for_test(
        &task_key,
        &route,
        ResidentTaskReservationPhase::CompletionPendingDurableHandling,
    );

    reservations
        .require_provider_completion(&settlement, &task_key)
        .expect("the exact repaired association validates");
    reservations
        .release_completion(settlement)
        .expect("the same capability releases exactly once");
    assert_eq!(reservations.outstanding(), 0);
}

#[test]
fn dropping_provider_settlement_never_releases_aggregate_capacity() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let primary = store("primary");
    let task_key = key(primary.clone(), "reservation/provider-drop");
    let route = ResidentTaskReservationRoute::Provider;
    let mut reservations = ResidentAggregateTaskReservations::new(owner, limit(1)).expect("ledger");
    let offer = reservations
        .begin_offer(task_key.clone(), route.clone())
        .expect("provider offer");
    reservations.accept_offer(offer).expect("accept offer");
    let settlement = reservations
        .mark_completion_pending(&task_key, &route)
        .expect("completion settlement");

    drop(settlement);

    assert_eq!(reservations.outstanding(), 1);
    assert!(matches!(
        reservations.begin_offer(
            key(primary, "reservation/provider-after-drop"),
            ResidentTaskReservationRoute::Provider,
        ),
        Err(ResidentTaskReservationOfferFailure::Saturated)
    ));
}

#[test]
fn provider_and_consult_routes_share_one_ceiling_and_release_exact_slots() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let primary = store("primary");
    let coordinate = ResidentConsultationBatchCoordinate::hostile_fixture(
        owner.clone(),
        primary.clone(),
        reference("consult/parent"),
        reference("consult/frontier"),
    );
    let consult_key = key(primary.clone(), "consult/child");
    let consult_route = ResidentTaskReservationRoute::Consultation(coordinate);
    let mut reservations = ResidentAggregateTaskReservations::new(owner, limit(1)).expect("ledger");
    let consult_offer = reservations
        .begin_offer(consult_key.clone(), consult_route.clone())
        .expect("Consult offer");
    reservations
        .accept_offer(consult_offer)
        .expect("Consult acceptance");

    assert!(matches!(
        reservations.begin_offer(
            key(primary, "provider/blocked"),
            ResidentTaskReservationRoute::Provider,
        ),
        Err(ResidentTaskReservationOfferFailure::Saturated)
    ));
    assert_eq!(reservations.outstanding(), 1);

    let settlement = reservations
        .mark_completion_pending(&consult_key, &consult_route)
        .expect("Consult completion settlement");
    reservations
        .release_completion(settlement)
        .expect("exact Consult settlement releases capacity");
    let provider_offer = reservations
        .begin_offer(
            key(store("primary"), "provider/after-consult"),
            ResidentTaskReservationRoute::Provider,
        )
        .expect("provider uses the released aggregate slot");
    reservations
        .accept_offer(provider_offer)
        .expect("provider acceptance");
    assert_eq!(reservations.outstanding(), 1);
}

#[test]
fn a2a_delegate_shares_the_ceiling_and_rejects_duplicate_dispatches() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let primary = store("primary");
    let a2a_key = key(primary.clone(), "a2a/dispatch");
    let mut reservations = ResidentAggregateTaskReservations::new(owner, limit(1)).expect("ledger");
    let offer = reservations
        .begin_offer(a2a_key.clone(), ResidentTaskReservationRoute::A2aDelegate)
        .expect("A2A offer");

    assert!(matches!(
        reservations.begin_offer(a2a_key, ResidentTaskReservationRoute::A2aDelegate,),
        Err(ResidentTaskReservationOfferFailure::Duplicate)
    ));
    assert!(matches!(
        reservations.begin_offer(
            key(primary, "provider/blocked-by-a2a"),
            ResidentTaskReservationRoute::Provider,
        ),
        Err(ResidentTaskReservationOfferFailure::Saturated)
    ));
    assert_eq!(reservations.outstanding(), 1);
    reservations
        .accept_offer(offer)
        .expect("the original A2A offer remains exact");
}

#[test]
fn a2a_offer_borrowed_proof_rejects_every_substitution_without_releasing_capacity() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let foreign_owner = ResidentOperationTaskOwnerInstance::new();
    let exact_key = key(store("primary"), "a2a/exact-offer");
    let substituted_key = key(store("primary"), "a2a/substituted-offer");
    let mut reservations =
        ResidentAggregateTaskReservations::new(owner.clone(), limit(2)).expect("aggregate ledger");
    let offer = reservations
        .begin_offer(exact_key.clone(), ResidentTaskReservationRoute::A2aDelegate)
        .expect("exact A2A offer");

    assert!(reservations.require_a2a_offer(&offer, &exact_key).is_ok());
    assert_eq!(
        reservations.require_a2a_offer(&offer, &substituted_key),
        Err(ResidentTaskReservationTransitionFailure::NotFound)
    );
    assert_eq!(reservations.outstanding(), 1);

    let foreign = ResidentAggregateTaskReservations::new(foreign_owner, limit(2))
        .expect("replacement ledger");
    assert_eq!(
        foreign.require_a2a_offer(&offer, &exact_key),
        Err(ResidentTaskReservationTransitionFailure::OwnerMismatch)
    );
    assert_eq!(foreign.outstanding(), 0);

    reservations.set_phase_for_test(
        &exact_key,
        &ResidentTaskReservationRoute::A2aDelegate,
        ResidentTaskReservationPhase::Accepted,
    );
    assert!(matches!(
        reservations.require_a2a_offer(&offer, &exact_key),
        Err(ResidentTaskReservationTransitionFailure::Phase {
            expected: ResidentTaskReservationPhase::Offered,
            actual: ResidentTaskReservationPhase::Accepted,
        })
    ));
    assert_eq!(reservations.outstanding(), 1);
    reservations.set_phase_for_test(
        &exact_key,
        &ResidentTaskReservationRoute::A2aDelegate,
        ResidentTaskReservationPhase::Offered,
    );

    reservations.replace_route_for_test(&exact_key, ResidentTaskReservationRoute::Provider);
    assert_eq!(
        reservations.require_a2a_offer(&offer, &exact_key),
        Err(ResidentTaskReservationTransitionFailure::RouteMismatch)
    );
    assert_eq!(reservations.outstanding(), 1);
    reservations.replace_route_for_test(&exact_key, ResidentTaskReservationRoute::A2aDelegate);

    drop(offer);
    assert_eq!(
        reservations.outstanding(),
        1,
        "dropping a linear offer cannot release aggregate capacity"
    );
    assert_eq!(
        reservations.require_capacity(),
        Ok(()),
        "one additional slot remains, but the dropped slot stays occupied"
    );
}

#[test]
fn a2a_completion_requires_exact_owner_key_route_and_phase() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let primary = store("primary");
    let a2a_key = key(primary.clone(), "a2a/completion");
    let route = ResidentTaskReservationRoute::A2aDelegate;
    let mut reservations = ResidentAggregateTaskReservations::new(owner, limit(1)).expect("ledger");
    let offer = reservations
        .begin_offer(a2a_key.clone(), route.clone())
        .expect("A2A offer");
    reservations.accept_offer(offer).expect("A2A acceptance");
    let settlement = reservations
        .mark_completion_pending(&a2a_key, &route)
        .expect("A2A completion settlement");

    let replacement =
        ResidentAggregateTaskReservations::new(ResidentOperationTaskOwnerInstance::new(), limit(1))
            .expect("replacement ledger");
    assert_eq!(
        replacement.require_a2a_completion(&settlement, &a2a_key),
        Err(ResidentTaskReservationTransitionFailure::OwnerMismatch)
    );
    assert_eq!(
        reservations.require_a2a_completion(&settlement, &key(primary.clone(), "a2a/substituted"),),
        Err(ResidentTaskReservationTransitionFailure::NotFound)
    );

    reservations.replace_route_for_test(&a2a_key, ResidentTaskReservationRoute::Provider);
    assert_eq!(
        reservations.require_a2a_completion(&settlement, &a2a_key),
        Err(ResidentTaskReservationTransitionFailure::RouteMismatch)
    );
    reservations.replace_route_for_test(&a2a_key, route.clone());
    reservations.set_phase_for_test(&a2a_key, &route, ResidentTaskReservationPhase::Accepted);
    assert_eq!(
        reservations.require_a2a_completion(&settlement, &a2a_key),
        Err(ResidentTaskReservationTransitionFailure::Phase {
            expected: ResidentTaskReservationPhase::CompletionPendingDurableHandling,
            actual: ResidentTaskReservationPhase::Accepted,
        })
    );
    reservations.set_phase_for_test(
        &a2a_key,
        &route,
        ResidentTaskReservationPhase::CompletionPendingDurableHandling,
    );
    reservations
        .require_a2a_completion(&settlement, &a2a_key)
        .expect("the repaired exact A2A settlement validates");
    reservations
        .release_completion(settlement)
        .expect("the exact A2A completion releases once");
    assert_eq!(reservations.outstanding(), 0);
}

#[test]
fn dropped_a2a_loss_settlement_keeps_capacity_reserved() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let primary = store("primary");
    let a2a_key = key(primary.clone(), "a2a/lost");
    let route = ResidentTaskReservationRoute::A2aDelegate;
    let mut reservations = ResidentAggregateTaskReservations::new(owner, limit(1)).expect("ledger");
    let offer = reservations
        .begin_offer(a2a_key.clone(), route.clone())
        .expect("recovered A2A offer");
    let settlement = reservations
        .lose_offer(offer)
        .expect("recovered uncertainty enters loss handling");
    reservations
        .require_a2a_loss(&settlement, &a2a_key)
        .expect("exact A2A loss settlement");

    drop(settlement);

    assert_eq!(reservations.outstanding(), 1);
    assert!(matches!(
        reservations.begin_offer(
            key(primary, "a2a/after-drop"),
            ResidentTaskReservationRoute::A2aDelegate,
        ),
        Err(ResidentTaskReservationOfferFailure::Saturated)
    ));
}

#[test]
fn provider_release_failure_retries_on_the_original_aggregate_owner() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let primary = store("primary");
    let task_key = key(primary, "provider/release-retry");
    let route = ResidentTaskReservationRoute::Provider;
    let mut reservations = ResidentAggregateTaskReservations::new(owner, limit(1)).expect("ledger");
    let offer = reservations
        .begin_offer(task_key.clone(), route.clone())
        .expect("provider offer");
    reservations
        .accept_offer(offer)
        .expect("provider acceptance");
    let settlement = reservations
        .mark_completion_pending(&task_key, &route)
        .expect("provider completion settlement");
    let mut replacement =
        ResidentAggregateTaskReservations::new(ResidentOperationTaskOwnerInstance::new(), limit(1))
            .expect("replacement ledger");

    let failure = replacement
        .release_completion(settlement)
        .expect_err("replacement owner cannot release provider capacity");
    assert_eq!(reservations.outstanding(), 1);
    reservations
        .retry_provider_release(failure)
        .expect("original owner retries only the exact aggregate release");
    assert_eq!(reservations.outstanding(), 0);
}

#[test]
fn get_task_settlements_cannot_cross_binding_keys() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let primary = store("primary");
    let first_key = a2a_task_key(primary.clone(), "first");
    let second_key = a2a_task_key(primary, "second");
    let route = ResidentTaskReservationRoute::A2aGetTask;
    let mut reservations =
        ResidentAggregateTaskReservations::new(owner, limit(2)).expect("aggregate ledger");
    let first_offer = reservations
        .begin_a2a_task_offer(first_key.clone(), route.clone())
        .expect("first GetTask offer");
    let second_offer = reservations
        .begin_a2a_task_offer(second_key.clone(), route.clone())
        .expect("second GetTask offer");
    reservations
        .accept_offer(first_offer)
        .expect("first GetTask acceptance");
    reservations
        .accept_offer(second_offer)
        .expect("second GetTask acceptance");
    let first = reservations
        .mark_a2a_task_completion_pending(&first_key, &route)
        .expect("first GetTask settlement");
    let second = reservations
        .mark_a2a_task_completion_pending(&second_key, &route)
        .expect("second GetTask settlement");

    assert_eq!(
        reservations.require_a2a_get_task_completion(&first, &second_key),
        Err(ResidentTaskReservationTransitionFailure::NotFound)
    );
    assert_eq!(
        reservations.require_a2a_get_task_completion(&second, &first_key),
        Err(ResidentTaskReservationTransitionFailure::NotFound)
    );
    assert_eq!(reservations.outstanding(), 2);
    reservations
        .release_completion(first)
        .expect("first exact settlement releases");
    reservations
        .release_completion(second)
        .expect("second exact settlement releases");
    assert_eq!(reservations.outstanding(), 0);
}

#[test]
fn get_task_and_cancel_task_conflict_on_the_same_binding_key() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let exact = a2a_task_key(store("shared"), "shared-binding");
    let mut reservations =
        ResidentAggregateTaskReservations::new(owner, limit(2)).expect("aggregate ledger");
    let get_offer = reservations
        .begin_a2a_task_offer(exact.clone(), ResidentTaskReservationRoute::A2aGetTask)
        .expect("GetTask reserves the shared binding key");

    assert!(matches!(
        reservations
            .begin_a2a_task_offer(exact.clone(), ResidentTaskReservationRoute::A2aCancelTask,),
        Err(ResidentA2aTaskReservationOfferFailure::Offer(
            ResidentTaskReservationOfferFailure::Duplicate,
        ))
    ));
    assert_eq!(reservations.outstanding(), 1);
    reservations
        .refuse_offer(get_offer)
        .expect("the original GetTask offer releases");

    let cancel_offer = reservations
        .begin_a2a_task_offer(exact.clone(), ResidentTaskReservationRoute::A2aCancelTask)
        .expect("CancelTask reserves the released binding key");
    assert!(matches!(
        reservations.begin_a2a_task_offer(exact, ResidentTaskReservationRoute::A2aGetTask),
        Err(ResidentA2aTaskReservationOfferFailure::Offer(
            ResidentTaskReservationOfferFailure::Duplicate,
        ))
    ));
    reservations
        .refuse_offer(cancel_offer)
        .expect("the exact CancelTask offer releases");
    assert_eq!(reservations.outstanding(), 0);
}

#[test]
fn get_task_and_cancel_task_for_unlike_bindings_can_coexist() {
    let owner = ResidentOperationTaskOwnerInstance::new();
    let selected_store = store("shared");
    let get_key = a2a_task_key(selected_store.clone(), "get-binding");
    let cancel_key = a2a_task_key(selected_store, "cancel-binding");
    let mut reservations =
        ResidentAggregateTaskReservations::new(owner, limit(2)).expect("aggregate ledger");
    let get_offer = reservations
        .begin_a2a_task_offer(get_key, ResidentTaskReservationRoute::A2aGetTask)
        .expect("GetTask offer");
    let cancel_offer = reservations
        .begin_a2a_task_offer(cancel_key, ResidentTaskReservationRoute::A2aCancelTask)
        .expect("unlike CancelTask offer");

    assert_eq!(reservations.outstanding(), 2);
    reservations
        .refuse_offer(get_offer)
        .expect("GetTask offer releases");
    reservations
        .refuse_offer(cancel_offer)
        .expect("CancelTask offer releases");
    assert_eq!(reservations.outstanding(), 0);
}
