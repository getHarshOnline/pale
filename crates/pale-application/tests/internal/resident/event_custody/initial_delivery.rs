//! Deterministic first-delivery identity and admission laws.

use super::*;

#[test]
fn initial_delivery_identity_is_stable_and_event_digest_sensitive() {
    let subscription = subscription("delivery-subscription", "delivery-subscription-active", 1);
    let first = InitialEventDeliveryPlan::admit(
        record_ref("event-record-1"),
        event("event-1", "revision-1"),
        subscription.clone(),
    )
    .expect("first plan");
    let repeated = InitialEventDeliveryPlan::admit(
        record_ref("event-record-1"),
        event("event-1", "revision-1"),
        subscription.clone(),
    )
    .expect("repeated plan");
    let changed = InitialEventDeliveryPlan::admit(
        record_ref("event-record-2"),
        event("event-1", "revision-2"),
        subscription,
    )
    .expect("changed plan");

    assert_eq!(first, repeated);
    assert_ne!(first, changed);
}
