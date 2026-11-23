//! Behavioral laws for fresh-child Consult roster preparation.

use super::*;
use crate::local_operation_start_fixture as local;
use crate::resident::consultation::{
    ConsultationRequirement, ConsultationRosterTemplate, ConsultationRosterTemplateSlot,
    ConsultationRosterTemplateSource, ConsultationSlotId,
};
use pale_core::records::*;
use std::cell::{Cell, RefCell};

fn source_generation() -> (
    crate::resident::AdmittedCompiledGeneration,
    EffectiveProjectProfileV1,
) {
    let (compilation, materializations, profile) =
        local::source_generation_inputs_with(|_| {}, |_| {});
    let generation = crate::resident::AdmittedCompiledGeneration::admit(
        compilation,
        materializations,
        crate::resident_fixture::synthetic_set_policy(),
    )
    .expect("admitted source generation");
    (generation, profile)
}

fn immutable(value: &str) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: local::record_ref(value),
        record_sha256: ArtifactDigest::hash_bytes(value.as_bytes()),
    }
}

fn slot<Child>(
    id: &str,
    source: ConsultationRosterTemplateSource<Child>,
) -> ConsultationRosterTemplateSlot<Child> {
    ConsultationRosterTemplateSlot::new(
        ConsultationSlotId::parse(id).expect("slot identity"),
        ConsultationRequirement::Required,
        immutable(&format!("requirement/{id}")),
        ParticipantRefV1::Application {
            application_id: id.to_owned(),
        },
        None,
        Some(immutable(&format!("context/{id}"))),
        source,
    )
}

fn roster<Child>(
    slots: Vec<ConsultationRosterTemplateSlot<Child>>,
) -> ConsultationRosterTemplate<Child> {
    ConsultationRosterTemplate::new(
        immutable("policy/fresh-child-roster"),
        UnixTimeV1::new(61).expect("open time"),
        UnixTimeV1::new(75).expect("deadline"),
        slots,
        crate::resident::ConsultationCollectionLimits::new(4, 65_536, 65_536)
            .expect("bounded collection"),
    )
    .expect("bounded roster template")
}

#[test]
fn parent_failure_retains_the_unallocated_roster_template() {
    let template = roster(vec![slot::<()>(
        "existing",
        ConsultationRosterTemplateSource::EvidenceResolved {
            evidence: immutable("evidence/existing"),
        },
    )]);
    let result = prepare_consultation_start(
        template,
        || Err::<SignedConsultationParentStart, _>("parent unavailable"),
        |_,
         _: &()|
         -> Result<
            ResidentConsultationAllocatedChild<()>,
            ResidentConsultationChildAllocationFailure<(), ()>,
        > { panic!("child allocation cannot precede its parent") },
        |_, _, _| Ok(local::signature()),
    );
    let Err(ResidentConsultationStartPreparationFailure::Parent { template, reason }) = result
    else {
        panic!("parent failure retains the declarative roster")
    };

    assert_eq!(reason, "parent unavailable");
    assert_eq!(template.slots.len(), 1);
    assert_eq!(template.fresh_child_count(), 0);
}

#[test]
fn parent_identity_precedes_child_capture_and_preserves_evidence_slots() {
    let (generation, profile) = source_generation();
    let store = local::store_coordinate();
    local::with_operation_start_input_at_store(
        &generation,
        &profile,
        "fresh-child-roster",
        |_| {},
        OperationClassV1::Consult,
        &store,
        |semantic, request, _checkpoint, parents| {
            let events = RefCell::new(Vec::new());
            let signatures = Cell::new(0usize);
            let template = roster(vec![
                slot(
                    "existing",
                    ConsultationRosterTemplateSource::EvidenceResolved {
                        evidence: immutable("evidence/existing"),
                    },
                ),
                slot(
                    "fresh",
                    ConsultationRosterTemplateSource::FreshChild { child: "fresh" },
                ),
            ]);
            events.borrow_mut().push("parent");
            let parent = crate::resident::local_operation_start::issue_local_consult_parent_start(
                semantic,
                &mut crate::session_admission_fixture::ProvingAuthority,
                |_, _, _, _, _, _, _, _| Ok(()),
                |_, _, _| {
                    signatures.set(signatures.get() + 1);
                    Ok(local::signature())
                },
            )
            .expect("signed Consult parent");
            let pending = ResidentConsultationRosterPending::new(parent, template);
            assert_eq!(events.borrow().as_slice(), ["parent"]);
            assert_eq!(pending.fresh_child_count(), 1);
            assert_eq!(
                pending.parent().parent().2.operation_class(),
                &OperationClassV1::Consult
            );
            let prepared = pending
                .prepare(
                    |(_, parent_ref, parent_handle, parent_lease),
                     suffix|
                     -> Result<
                        ResidentConsultationAllocatedChild<OperationHandleIdV1>,
                        ResidentConsultationChildAllocationFailure<OperationHandleIdV1, ()>,
                    > {
                        events.borrow_mut().push(*suffix);
                        crate::resident::consultation::tests::with_test_child_start_input(
                            (parent_ref, parent_handle),
                            parent_lease,
                            None,
                            request,
                            &generation,
                            &parents,
                            OperationClassV1::Invoke,
                            suffix,
                            |input, _| {
                                let child_id = input.semantic.handle.operation_handle_id.clone();
                                ResidentConsultationAllocatedChild::capture(child_id, &input)
                                    .map_err(ResidentConsultationChildAllocationFailure::Capture)
                            },
                        )
                    },
                    |_, _, _| {
                        signatures.set(signatures.get() + 1);
                        Ok(local::signature())
                    },
                )
                .unwrap_or_else(|_| panic!("parent and fresh-child roster prepare"));

            assert_eq!(events.into_inner(), vec!["parent", "fresh"]);
            assert_eq!(signatures.get(), 2, "one parent handle and observation");
            assert_eq!(prepared.children.len(), 1);
            assert_eq!(prepared.children[0].as_str(), "child-operation-fresh");
            let intent = prepared
                .parent
                .consultation_start_intent()
                .expect("prepared Consult retains its private roster");
            assert_eq!(intent.slot_count(), 2);
            assert_eq!(intent.fresh_child_count(), 1);
            assert_eq!(intent.evidence_resolved_count(), 1);
            assert_eq!(
                prepared.parent.prepared_handle().1.operation_class(),
                &OperationClassV1::Consult
            );
        },
    );
}

#[test]
fn failed_child_step_retains_the_signed_parent_and_completed_prefix() {
    let (generation, profile) = source_generation();
    let store = local::store_coordinate();
    local::with_operation_start_input_at_store(
        &generation,
        &profile,
        "fresh-child-prefix",
        |_| {},
        OperationClassV1::Consult,
        &store,
        |semantic, request, _checkpoint, parents| {
            let parent_calls = Cell::new(0usize);
            let child_calls = Cell::new(0usize);
            let template = roster(vec![
                slot(
                    "first",
                    ConsultationRosterTemplateSource::FreshChild { child: "first" },
                ),
                slot(
                    "second",
                    ConsultationRosterTemplateSource::FreshChild { child: "second" },
                ),
            ]);
            let result = prepare_consultation_start(
                template,
                || {
                    parent_calls.set(parent_calls.get() + 1);
                    crate::resident::local_operation_start::issue_local_consult_parent_start(
                        semantic,
                        &mut crate::session_admission_fixture::ProvingAuthority,
                        |_, _, _, _, _, _, _, _| Ok(()),
                        |_, _, _| Ok(local::signature()),
                    )
                },
                |(_, parent_ref, parent_handle, parent_lease), suffix| {
                    child_calls.set(child_calls.get() + 1);
                    if *suffix == "second" {
                        return Err(ResidentConsultationChildAllocationFailure::Owner(
                            "second child unavailable",
                        ));
                    }
                    crate::resident::consultation::tests::with_test_child_start_input(
                        (parent_ref, parent_handle),
                        parent_lease,
                        None,
                        request,
                        &generation,
                        &parents,
                        OperationClassV1::Invoke,
                        suffix,
                        |input, _| {
                            ResidentConsultationAllocatedChild::capture(
                                input.semantic.handle.operation_handle_id.clone(),
                                &input,
                            )
                            .map_err(ResidentConsultationChildAllocationFailure::Capture)
                        },
                    )
                },
                |_, _, _| Ok(local::signature()),
            );
            let Err(ResidentConsultationStartPreparationFailure::Roster(
                ResidentConsultationRosterPreparationFailure::Child {
                    parent,
                    children,
                    roster,
                    reason: ResidentConsultationChildAllocationFailure::Owner(reason),
                },
            )) = result
            else {
                panic!("second child failure retains exact preparation state")
            };

            assert_eq!(reason, "second child unavailable");
            assert_eq!(parent_calls.get(), 1);
            assert_eq!(child_calls.get(), 2);
            assert_eq!(
                parent.parent().2.operation_class(),
                &OperationClassV1::Consult
            );
            assert_eq!(children.len(), 1);
            assert_eq!(children[0].as_str(), "child-operation-first");
            assert_eq!(roster.materialized.len(), 1);
            assert_eq!(roster.remaining.len(), 1);
        },
    );
}

#[test]
fn parent_commit_failure_retains_every_allocated_child_in_roster_order() {
    let (generation, profile) = source_generation();
    let store = local::store_coordinate();
    local::with_operation_start_input_at_store(
        &generation,
        &profile,
        "parent-commit-failure",
        |_| {},
        OperationClassV1::Consult,
        &store,
        |semantic, request, _checkpoint, parents| {
            let template = roster(vec![
                slot(
                    "first",
                    ConsultationRosterTemplateSource::FreshChild { child: "first" },
                ),
                slot(
                    "second",
                    ConsultationRosterTemplateSource::FreshChild { child: "second" },
                ),
            ]);
            let prepared = prepare_consultation_start(
                template,
                || {
                    crate::resident::local_operation_start::issue_local_consult_parent_start(
                        semantic,
                        &mut crate::session_admission_fixture::ProvingAuthority,
                        |_, _, _, _, _, _, _, _| Ok(()),
                        |_, _, _| Ok(local::signature()),
                    )
                },
                |(_, parent_ref, parent_handle, parent_lease), child| {
                    crate::resident::consultation::tests::with_test_child_start_input(
                        (parent_ref, parent_handle),
                        parent_lease,
                        None,
                        request,
                        &generation,
                        &parents,
                        OperationClassV1::Invoke,
                        child,
                        |input, _| {
                            ResidentConsultationAllocatedChild::capture(*child, &input).map_err(
                                ResidentConsultationChildAllocationFailure::<&str, ()>::Capture,
                            )
                        },
                    )
                },
                |_, _, _| Ok(local::signature()),
            )
            .unwrap_or_else(|_| panic!("prepared roster"));
            let pending = prepared
                .into_parent_commit()
                .unwrap_or_else(|_| panic!("matching parent intent"));
            let Err(failure) = pending
                .commit::<(), (), _, _>(|parent| Err((Box::new(parent), "custody unavailable")))
            else {
                panic!("parent custody fails")
            };
            let ResidentConsultationParentCommitFailure::Owner {
                remaining,
                child_count,
                reason: (parent, reason),
            } = failure
            else {
                panic!("parent owner reports its exact failure")
            };

            assert_eq!(reason, "custody unavailable");
            assert_eq!(child_count, 2);
            assert_eq!(
                remaining.into_iter().collect::<Vec<_>>(),
                ["first", "second"]
            );
            assert_eq!(
                parent
                    .consultation_start_intent()
                    .expect("retained parent intent")
                    .fresh_child_count(),
                2
            );
        },
    );
}

#[test]
fn child_schedule_retries_the_same_front_and_only_then_allows_collection() {
    let (generation, profile) = source_generation();
    let store = local::store_coordinate();
    local::with_operation_start_input_at_store(
        &generation,
        &profile,
        "child-schedule",
        |_| {},
        OperationClassV1::Consult,
        &store,
        |semantic, request, _checkpoint, parents| {
            let template = roster(vec![
                slot(
                    "first",
                    ConsultationRosterTemplateSource::FreshChild { child: "first" },
                ),
                slot(
                    "second",
                    ConsultationRosterTemplateSource::FreshChild { child: "second" },
                ),
            ]);
            let prepared = prepare_consultation_start(
                template,
                || {
                    crate::resident::local_operation_start::issue_local_consult_parent_start(
                        semantic,
                        &mut crate::session_admission_fixture::ProvingAuthority,
                        |_, _, _, _, _, _, _, _| Ok(()),
                        |_, _, _| Ok(local::signature()),
                    )
                },
                |(_, parent_ref, parent_handle, parent_lease), child| {
                    crate::resident::consultation::tests::with_test_child_start_input(
                        (parent_ref, parent_handle),
                        parent_lease,
                        None,
                        request,
                        &generation,
                        &parents,
                        OperationClassV1::Invoke,
                        child,
                        |input, _| {
                            ResidentConsultationAllocatedChild::capture(*child, &input).map_err(
                                ResidentConsultationChildAllocationFailure::<&str, ()>::Capture,
                            )
                        },
                    )
                },
                |_, _, _| Ok(local::signature()),
            )
            .unwrap_or_else(|_| panic!("prepared roster"));
            let pending = prepared
                .into_parent_commit()
                .unwrap_or_else(|_| panic!("matching parent intent"));
            let first = pending
                .commit::<_, &'static str, (), _>(|_| Ok("durable-parent"))
                .unwrap_or_else(|_| panic!("parent custody"));
            let ResidentConsultationChildAdvance::Next(first) = first else {
                panic!("first allocated child remains")
            };
            assert_eq!(first.parent(), &"durable-parent");
            assert_eq!(first.next_child(), Some(&"first"));

            let second = first
                .advance::<(), _>(|_, child| {
                    Ok(match *child {
                        "first" => "started-first",
                        _ => unreachable!("first roster child"),
                    })
                })
                .unwrap_or_else(|_| panic!("first child starts"));
            let ResidentConsultationChildAdvance::Next(second) = second else {
                panic!("second allocated child remains")
            };
            assert_eq!(second.started(), ["started-first"]);
            assert_eq!(second.next_child(), Some(&"second"));

            let Err(failure) = second.advance::<_, _>(|_, _| Err("child custody unavailable"))
            else {
                panic!("second child remains retryable")
            };
            assert_eq!(failure.reason, "child custody unavailable");
            assert_eq!(failure.schedule.started(), ["started-first"]);
            assert_eq!(failure.schedule.next_child(), Some(&"second"));

            let completed = failure
                .schedule
                .advance::<(), _>(|_, child| {
                    Ok(match *child {
                        "second" => "started-second",
                        _ => unreachable!("same second roster child"),
                    })
                })
                .unwrap_or_else(|_| panic!("same second child starts"));
            let ResidentConsultationChildAdvance::InitializeCollection(completed) = completed
            else {
                panic!("collection waits for the complete durable prefix")
            };
            assert_eq!(completed.parent, "durable-parent");
            assert_eq!(completed.children, ["started-first", "started-second"]);
        },
    );
}
