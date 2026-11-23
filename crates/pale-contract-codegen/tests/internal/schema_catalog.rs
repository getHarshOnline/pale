//! Verifies the schema producer catalog has one complete, orthogonal classification.

use std::collections::BTreeSet;

use super::*;

#[test]
fn convergence_families_replace_only_the_approved_predecessors() {
    let schema_ids = contract_producers()
        .iter()
        .map(|producer| producer.schema_id())
        .collect::<BTreeSet<_>>();

    for current in [
        "capability-revocation-checkpoint.v1",
        "capability-session-grant-revocation.v1",
        "capability-session-grant.v1",
        "event-acknowledgement.v1",
        "event-reference.v1",
        "execution-owner-fence.v1",
        "genealogy-edge.v1",
        "interaction-occurrence.v1",
        "local-route-binding.v1",
        "reconciliation-frontier.v1",
    ] {
        assert!(
            schema_ids.contains(current),
            "missing current family {current}"
        );
    }

    for predecessor in [
        "dispatch-evidence-reservation.v1",
        "execution-route.v1",
        "orchestration-child-evidence-ref.v1",
        "orchestration-completion.v1",
        "orchestration-dispatch-receipt.v1",
        "orchestration-dispatch-request.v1",
        "orchestration-envelope.v1",
        "orchestration-native-run-result.v1",
        "orchestration-owner-evidence-ref.v1",
        "orchestration-prepare-request.v1",
        "orchestration-prepare-response.v1",
        "orchestration-technical-receipt.v1",
        "orchestrator-adapter-projection.v1",
        "owner-event-reference.v1",
        "project-set.v1",
        "repositories.v1",
        "route-activation-index.v1",
    ] {
        assert!(
            !schema_ids.contains(predecessor),
            "predecessor family remains active: {predecessor}"
        );
    }
}

#[test]
fn every_producer_has_one_orthogonal_projection_classification() {
    let producers = contract_producers();

    let definition_sets = producers
        .iter()
        .filter(|producer| producer.projection_kind() == ProjectionKind::DefinitionSet)
        .map(|producer| producer.schema_id())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        definition_sets,
        BTreeSet::from(["capability-procedure.v1", "composition-primitives.v1"])
    );
    assert_eq!(
        producers
            .iter()
            .filter(|producer| producer.projection_kind() == ProjectionKind::Root)
            .count(),
        136
    );

    let structurally_complete = producers
        .iter()
        .filter(|producer| producer.admission_route() == AdmissionRoute::StructuralOnly)
        .map(|producer| producer.schema_id())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        structurally_complete,
        BTreeSet::from([
            "deferred-effect.v1",
            "lifecycle-event.v1",
            "operation-denial.v1",
            "sha256-digest.v1",
            "surface-ownership.v1",
            "tool-call-matcher.v1",
        ])
    );
    assert_eq!(
        producers
            .iter()
            .filter(|producer| producer.admission_route() == AdmissionRoute::Contextual)
            .count(),
        123
    );
    assert_eq!(
        producers
            .iter()
            .filter(|producer| producer.admission_route() == AdmissionRoute::ContextFree)
            .count(),
        0
    );

    let generated = producers
        .iter()
        .filter(|producer| producer.admission_route() == AdmissionRoute::Generated)
        .map(|producer| producer.schema_id())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        generated,
        BTreeSet::from([
            "capability-bindings.v1",
            "deferred-surface-manifest.v1",
            "generated-manifest.v1",
            "harness-projection.v1",
            "hook-context.v1",
            "operation-catalog.v1",
            "operation-contract.v1",
            "operation-route-binding.v1",
            "repository-native-action-catalog.v1",
        ])
    );
}
