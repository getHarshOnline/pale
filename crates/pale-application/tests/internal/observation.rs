//! Behavioral laws for the bounded observation algebra.

use std::collections::BTreeMap;

use pale_core::records::{
    CompositionReasonStepV1, ControlIncarnationIdV1, ControlStoreIdV1, DiagnosticEffectCertaintyV1,
    ExperienceCompositionPlanDigest, InstanceDigest, JournalSequenceV1, ProfileDigest,
};
use pale_ports::AuthorityStoreCoordinate;

use crate::observation::{ObservationCategory, ObservationFilter};

use super::{
    index_subjects, insert_fact, readiness_meet, truncate_facts, ObservationBackpressureRecovery,
    ObservationCancellation, ObservationCausalPath, ObservationCausalSelector,
    ObservationCausalStep, ObservationCursor, ObservationDeltaOutcome, ObservationDeltaRequest,
    ObservationFact, ObservationFreshness, ObservationGeneration, ObservationHealthState,
    ObservationLensDisposition, ObservationLensRequest, ObservationPage,
    ObservationPageDisposition, ObservationPageRequest, ObservationReadinessState,
    ObservationResumeCoordinate, ObservationResynchronizationReason, ObservationRevision,
    ObservationSnapshot, ObservationStepCompletion, ObservationStepLimit, ObservationSubject,
    MAX_OBSERVATION_FACTS,
};

fn snapshot(
    generation: ObservationGeneration,
    revision: ObservationRevision,
    facts: Vec<ObservationFact>,
    causal_paths: Vec<ObservationCausalPath>,
) -> ObservationSnapshot {
    let mut admitted = BTreeMap::new();
    for fact in facts {
        insert_fact(&mut admitted, fact);
    }
    let omitted_fact_count = truncate_facts(&mut admitted);
    let subjects = index_subjects(&admitted);
    let causal_paths = causal_paths
        .into_iter()
        .map(|path| (path.selector.subject.clone(), path))
        .collect();
    ObservationSnapshot {
        profile_origin: None,
        cursor: ObservationCursor {
            generation,
            revision,
        },
        facts: admitted,
        subjects,
        causal_paths,
        omitted_fact_count,
    }
}

fn ready_page(disposition: ObservationPageDisposition) -> ObservationPage {
    let ObservationPageDisposition::Ready(page) = disposition else {
        panic!("active request must return a page");
    };
    page
}

fn generation(seed: char) -> ObservationGeneration {
    ObservationGeneration::EffectiveProfile {
        profile_digest: ProfileDigest::parse(std::iter::repeat_n(seed, 64).collect::<String>())
            .unwrap(),
        instance_digest: InstanceDigest::parse(std::iter::repeat_n(seed, 64).collect::<String>())
            .unwrap(),
        composition_plan_digest: ExperienceCompositionPlanDigest::parse(
            std::iter::repeat_n(seed, 64).collect::<String>(),
        )
        .unwrap(),
    }
}

fn resident_generation() -> ObservationGeneration {
    ObservationGeneration::Resident {
        authority_store: AuthorityStoreCoordinate::new(
            ControlStoreIdV1::parse("control-store").unwrap(),
            ControlIncarnationIdV1::parse("incarnation-a").unwrap(),
        ),
        active_generation: None,
    }
}

fn effective_generation(profile_seed: char, instance_seed: char) -> ObservationGeneration {
    ObservationGeneration::EffectiveProfile {
        profile_digest: ProfileDigest::parse(
            std::iter::repeat_n(profile_seed, 64).collect::<String>(),
        )
        .unwrap(),
        instance_digest: InstanceDigest::parse(
            std::iter::repeat_n(instance_seed, 64).collect::<String>(),
        )
        .unwrap(),
        composition_plan_digest: ExperienceCompositionPlanDigest::parse("c".repeat(64)).unwrap(),
    }
}

fn resident_generation_in(incarnation: &str) -> ObservationGeneration {
    ObservationGeneration::Resident {
        authority_store: AuthorityStoreCoordinate::new(
            ControlStoreIdV1::parse("control-store").unwrap(),
            ControlIncarnationIdV1::parse(incarnation).unwrap(),
        ),
        active_generation: None,
    }
}

#[test]
fn resident_snapshot_has_no_profile_association_source() {
    let snapshot = snapshot(
        resident_generation(),
        ObservationRevision::ResidentJournal(None),
        Vec::new(),
        Vec::new(),
    );
    assert_eq!(
        snapshot.profile_source_owner().unwrap_err(),
        crate::observation::ObservationProfileError::NoProfileSource
    );
}

fn fact(
    category: ObservationCategory,
    machine: &str,
    readiness: ObservationReadinessState,
) -> ObservationFact {
    let subject = ObservationSubject::Machine(machine.to_owned());
    match category {
        ObservationCategory::OperationExposure
        | ObservationCategory::CapabilityRequirementDisposition => {
            panic!("fixture category requires a repository-scoped semantic subject")
        }
        ObservationCategory::Health => ObservationFact::Health {
            subject,
            state: ObservationHealthState::Unknown,
            freshness: ObservationFreshness::Unknown,
        },
        ObservationCategory::Readiness => ObservationFact::Readiness {
            subject,
            state: readiness,
            freshness: ObservationFreshness::Fresh,
        },
        ObservationCategory::Effect => ObservationFact::Effect {
            subject,
            certainty: DiagnosticEffectCertaintyV1::NotStarted,
            freshness: ObservationFreshness::Fresh,
        },
        ObservationCategory::NativeOwner
        | ObservationCategory::Reconciliation
        | ObservationCategory::ProcessIntent => panic!("fixture category is not used"),
    }
}

fn causal_path(
    generation: ObservationGeneration,
    subject: ObservationSubject,
    identifiers: &[&str],
) -> ObservationCausalPath {
    ObservationCausalPath {
        selector: ObservationCausalSelector {
            generation,
            subject,
        },
        steps: identifiers
            .iter()
            .map(|identifier| {
                ObservationCausalStep::Composition(CompositionReasonStepV1::EntryContract {
                    contract_id: (*identifier).to_owned(),
                })
            })
            .collect(),
    }
}

#[test]
fn snapshot_pagination_binds_continuation_to_snapshot_and_filter() {
    let snapshot = snapshot(
        generation('a'),
        ObservationRevision::Static,
        vec![
            fact(
                ObservationCategory::Readiness,
                "machine-a",
                ObservationReadinessState::Ready,
            ),
            fact(
                ObservationCategory::Readiness,
                "machine-b",
                ObservationReadinessState::Unproven,
            ),
            fact(
                ObservationCategory::Health,
                "machine-a",
                ObservationReadinessState::Unproven,
            ),
        ],
        Vec::new(),
    );
    let filter = ObservationFilter::all().with_category(ObservationCategory::Readiness);
    let first = ready_page(
        snapshot
            .page(ObservationPageRequest::new(filter.clone(), 1))
            .unwrap(),
    );
    assert_eq!(first.facts.len(), 1);
    let cursor = first.continuation.expect("one matching fact remains");
    let second = ready_page(
        snapshot
            .page(ObservationPageRequest::new(filter, 1).after(cursor.clone()))
            .unwrap(),
    );
    assert_eq!(second.facts.len(), 1);
    let terminal = ready_page(
        snapshot
            .page(
                ObservationPageRequest::new(
                    ObservationFilter::all().with_category(ObservationCategory::Readiness),
                    1,
                )
                .after(
                    second
                        .continuation
                        .expect("bounded work does not perform an uncharged exhaustion scan"),
                ),
            )
            .unwrap(),
    );
    assert!(terminal.facts.is_empty());
    assert_eq!(terminal.completion, ObservationStepCompletion::Complete);

    let mismatch = snapshot
        .page(
            ObservationPageRequest::new(
                ObservationFilter::all().with_category(ObservationCategory::Health),
                1,
            )
            .after(cursor),
        )
        .expect_err("a cursor cannot cross filters");
    assert_eq!(
        mismatch.reason,
        ObservationResynchronizationReason::ContinuationMismatch
    );
}

#[test]
fn page_bounds_inspection_and_resumes_without_rescanning() {
    let snapshot = snapshot(
        generation('0'),
        ObservationRevision::Static,
        vec![
            fact(
                ObservationCategory::Health,
                "machine-a",
                ObservationReadinessState::Unproven,
            ),
            fact(
                ObservationCategory::Readiness,
                "machine-a",
                ObservationReadinessState::Ready,
            ),
            fact(
                ObservationCategory::Readiness,
                "machine-b",
                ObservationReadinessState::Ready,
            ),
        ],
        Vec::new(),
    );
    let filter = ObservationFilter::all().with_category(ObservationCategory::Readiness);
    let request = || {
        ObservationPageRequest::bounded(
            filter.clone(),
            1,
            8,
            1_024,
            ObservationCancellation::Active,
        )
    };

    let first = ready_page(snapshot.page(request()).unwrap());
    assert!(first.facts.is_empty());
    assert_eq!(first.inspected, 1);
    assert_eq!(
        first.completion,
        ObservationStepCompletion::Stopped(ObservationStepLimit::Inspected)
    );
    let first_cursor = first
        .continuation
        .expect("the readiness facts were not inspected");
    assert_eq!(first_cursor.inspected_position(), 1);

    let second = ready_page(snapshot.page(request().after(first_cursor)).unwrap());
    assert_eq!(second.facts.len(), 1);
    assert_eq!(second.inspected, 1);
    let second_cursor = second.continuation.expect("one readiness fact remains");
    assert_eq!(second_cursor.inspected_position(), 2);

    let third = ready_page(snapshot.page(request().after(second_cursor)).unwrap());
    assert_eq!(third.facts.len(), 1);
    let third_cursor = third
        .continuation
        .expect("bounded work does not perform an uncharged exhaustion scan");
    assert_eq!(third_cursor.inspected_position(), 3);

    let terminal = ready_page(snapshot.page(request().after(third_cursor)).unwrap());
    assert!(terminal.facts.is_empty());
    assert_eq!(terminal.inspected, 0);
    assert_eq!(terminal.completion, ObservationStepCompletion::Complete);
    assert!(terminal.continuation.is_none());
}

#[test]
fn small_page_work_does_not_scale_with_snapshot_cardinality() {
    let facts = (0..MAX_OBSERVATION_FACTS)
        .map(|index| {
            fact(
                ObservationCategory::Readiness,
                &format!("machine-{index:04}"),
                ObservationReadinessState::Ready,
            )
        })
        .collect();
    let snapshot = snapshot(
        generation('d'),
        ObservationRevision::Static,
        facts,
        Vec::new(),
    );
    let request = || {
        ObservationPageRequest::bounded(
            ObservationFilter::all(),
            1,
            1,
            1_024,
            ObservationCancellation::Active,
        )
    };

    let first = ready_page(snapshot.page(request()).unwrap());
    assert_eq!(first.inspected, 1);
    assert_eq!(first.facts.len(), 1);
    let cursor = first.continuation.expect("the snapshot has more facts");
    assert_eq!(cursor.inspected_position(), 1);

    let second = ready_page(snapshot.page(request().after(cursor)).unwrap());
    assert_eq!(second.inspected, 1);
    assert_eq!(second.facts.len(), 1);
    assert_eq!(
        second
            .continuation
            .expect("the snapshot has more facts")
            .inspected_position(),
        2
    );
}

#[test]
fn page_separates_copy_stops_from_unrepresentable_items() {
    let first = fact(
        ObservationCategory::Readiness,
        "machine-a",
        ObservationReadinessState::Ready,
    );
    let second = fact(
        ObservationCategory::Readiness,
        "machine-b",
        ObservationReadinessState::Ready,
    );
    let first_bytes = first.copied_string_bytes();
    let second_bytes = second.copied_string_bytes();
    assert_eq!(first_bytes, second_bytes);
    let snapshot = snapshot(
        generation('1'),
        ObservationRevision::Static,
        vec![first.clone(), second],
        Vec::new(),
    );

    let stopped = ready_page(
        snapshot
            .page(ObservationPageRequest::bounded(
                ObservationFilter::all(),
                8,
                8,
                first_bytes,
                ObservationCancellation::Active,
            ))
            .unwrap(),
    );
    assert_eq!(stopped.facts, vec![first]);
    assert_eq!(stopped.copied_string_bytes, first_bytes);
    assert_eq!(
        stopped.completion,
        ObservationStepCompletion::Stopped(ObservationStepLimit::CopiedStringBytes {
            minimum_required: second_bytes,
        })
    );
    assert!(stopped.continuation.is_some());

    let oversized = ready_page(
        snapshot
            .page(ObservationPageRequest::bounded(
                ObservationFilter::all(),
                8,
                8,
                first_bytes - 1,
                ObservationCancellation::Active,
            ))
            .unwrap(),
    );
    assert!(oversized.facts.is_empty());
    assert_eq!(oversized.inspected, 1);
    assert_eq!(
        oversized.completion,
        ObservationStepCompletion::Incomplete(ObservationStepLimit::CopiedStringBytes {
            minimum_required: first_bytes,
        })
    );
    assert!(oversized.continuation.is_none());
}

#[test]
fn page_continuation_binds_raw_limits_and_survives_cancellation() {
    let snapshot = snapshot(
        generation('2'),
        ObservationRevision::Static,
        vec![fact(
            ObservationCategory::Readiness,
            "machine-a",
            ObservationReadinessState::Ready,
        )],
        Vec::new(),
    );
    let request = || {
        ObservationPageRequest::bounded(
            ObservationFilter::all(),
            1,
            8,
            1_024,
            ObservationCancellation::Active,
        )
    };
    let first = ready_page(snapshot.page(request()).unwrap());
    let cursor = first
        .continuation
        .expect("the bounded step issues a cursor");

    for (maximum_inspected, maximum_returned, maximum_copied_string_bytes) in
        [(2, 8, 1_024), (1, 9, 1_024), (1, 8, 1_025)]
    {
        let mismatch = snapshot
            .page(
                ObservationPageRequest::bounded(
                    ObservationFilter::all(),
                    maximum_inspected,
                    maximum_returned,
                    maximum_copied_string_bytes,
                    ObservationCancellation::Active,
                )
                .after(cursor.clone()),
            )
            .expect_err("a cursor cannot cross raw bounds");
        assert_eq!(
            mismatch.reason,
            ObservationResynchronizationReason::ContinuationMismatch
        );
    }

    let mismatch = snapshot
        .page(
            ObservationPageRequest::bounded(
                ObservationFilter::all().with_category(ObservationCategory::Health),
                1,
                8,
                1_024,
                ObservationCancellation::Active,
            )
            .after(cursor.clone()),
        )
        .expect_err("a cursor cannot cross filters");
    assert_eq!(
        mismatch.reason,
        ObservationResynchronizationReason::ContinuationMismatch
    );

    let ObservationPageDisposition::Cancelled { continuation } = snapshot
        .page(
            ObservationPageRequest::bounded(
                ObservationFilter::all(),
                1,
                8,
                1_024,
                ObservationCancellation::Cancelled,
            )
            .after(cursor.clone()),
        )
        .unwrap()
    else {
        panic!("cancellation must remain a successful disposition");
    };
    assert_eq!(continuation, Some(cursor));
}

#[test]
fn delta_requires_same_generation_and_adjacent_revision() {
    let prior = snapshot(
        resident_generation(),
        ObservationRevision::ResidentJournal(Some(JournalSequenceV1::new(1).unwrap())),
        vec![fact(
            ObservationCategory::Readiness,
            "machine-a",
            ObservationReadinessState::Unproven,
        )],
        Vec::new(),
    );
    let current = snapshot(
        resident_generation(),
        ObservationRevision::ResidentJournal(Some(JournalSequenceV1::new(2).unwrap())),
        vec![fact(
            ObservationCategory::Readiness,
            "machine-a",
            ObservationReadinessState::Ready,
        )],
        Vec::new(),
    );
    let request = ObservationDeltaRequest {
        resume: prior.resume_coordinate(),
        maximum_changes: 8,
        cancellation: ObservationCancellation::Active,
    };
    let ObservationDeltaOutcome::Delta(delta) = current.delta_from(&prior, request) else {
        panic!("adjacent revisions must produce one complete delta");
    };
    assert_eq!(delta.changes.len(), 1);
    assert_eq!(delta.from, *prior.cursor());
    assert_eq!(delta.to, *current.cursor());

    let skipped = snapshot(
        resident_generation(),
        ObservationRevision::ResidentJournal(Some(JournalSequenceV1::new(4).unwrap())),
        Vec::new(),
        Vec::new(),
    );
    let outcome = skipped.delta_from(
        &prior,
        ObservationDeltaRequest {
            resume: prior.resume_coordinate(),
            maximum_changes: 8,
            cancellation: ObservationCancellation::Active,
        },
    );
    assert!(matches!(
        outcome,
        ObservationDeltaOutcome::Resynchronize(ref value)
            if matches!(value.reason, ObservationResynchronizationReason::RevisionGap { .. })
    ));

    let changed_generation = snapshot(
        generation('b'),
        ObservationRevision::Static,
        Vec::new(),
        Vec::new(),
    );
    let outcome = changed_generation.delta_from(
        &prior,
        ObservationDeltaRequest {
            resume: prior.resume_coordinate(),
            maximum_changes: 8,
            cancellation: ObservationCancellation::Active,
        },
    );
    assert!(matches!(
        outcome,
        ObservationDeltaOutcome::Resynchronize(ref value)
            if matches!(value.reason, ObservationResynchronizationReason::GenerationMismatch { .. })
    ));
}

#[test]
fn generation_binds_machine_instance_and_authority_store_incarnation() {
    let profile_a = snapshot(
        effective_generation('a', 'b'),
        ObservationRevision::Static,
        Vec::new(),
        Vec::new(),
    );
    let profile_b = snapshot(
        effective_generation('a', 'c'),
        ObservationRevision::Static,
        Vec::new(),
        Vec::new(),
    );
    assert!(matches!(
        profile_b.delta_from(
            &profile_a,
            ObservationDeltaRequest {
                resume: profile_a.resume_coordinate(),
                maximum_changes: 8,
                cancellation: ObservationCancellation::Active,
            },
        ),
        ObservationDeltaOutcome::Resynchronize(ref value)
            if matches!(value.reason, ObservationResynchronizationReason::GenerationMismatch { .. })
    ));

    let resident_a = snapshot(
        resident_generation_in("incarnation-a"),
        ObservationRevision::ResidentJournal(Some(JournalSequenceV1::new(1).unwrap())),
        Vec::new(),
        Vec::new(),
    );
    let resident_b = snapshot(
        resident_generation_in("incarnation-b"),
        ObservationRevision::ResidentJournal(Some(JournalSequenceV1::new(2).unwrap())),
        Vec::new(),
        Vec::new(),
    );
    assert!(matches!(
        resident_b.delta_from(
            &resident_a,
            ObservationDeltaRequest {
                resume: resident_a.resume_coordinate(),
                maximum_changes: 8,
                cancellation: ObservationCancellation::Active,
            },
        ),
        ObservationDeltaOutcome::Resynchronize(ref value)
            if matches!(value.reason, ObservationResynchronizationReason::GenerationMismatch { .. })
    ));
}

#[test]
fn delta_cancellation_and_backpressure_never_emit_partial_state() {
    let prior = snapshot(
        resident_generation(),
        ObservationRevision::ResidentJournal(None),
        Vec::new(),
        Vec::new(),
    );
    let current = snapshot(
        resident_generation(),
        ObservationRevision::ResidentJournal(Some(JournalSequenceV1::new(1).unwrap())),
        vec![
            fact(
                ObservationCategory::Readiness,
                "machine-a",
                ObservationReadinessState::Ready,
            ),
            fact(
                ObservationCategory::Readiness,
                "machine-b",
                ObservationReadinessState::Ready,
            ),
        ],
        Vec::new(),
    );
    let resume = prior.resume_coordinate();
    assert!(matches!(
        current.delta_from(
            &prior,
            ObservationDeltaRequest {
                resume: resume.clone(),
                maximum_changes: 8,
                cancellation: ObservationCancellation::Cancelled,
            }
        ),
        ObservationDeltaOutcome::Cancelled(ObservationResumeCoordinate(_))
    ));

    let outcome = current.delta_from(
        &prior,
        ObservationDeltaRequest {
            resume,
            maximum_changes: 1,
            cancellation: ObservationCancellation::Active,
        },
    );
    assert!(matches!(
        outcome,
        ObservationDeltaOutcome::Backpressured {
            pending_changes: 2,
            maximum_changes: 1,
            recovery: ObservationBackpressureRecovery::RequestSnapshot,
            ..
        }
    ));
}

#[test]
fn visualization_lens_is_bounded_deterministic_and_non_geometric() {
    let snapshot = snapshot(
        generation('c'),
        ObservationRevision::Static,
        vec![
            fact(
                ObservationCategory::Readiness,
                "machine-b",
                ObservationReadinessState::Ready,
            ),
            fact(
                ObservationCategory::Health,
                "machine-a",
                ObservationReadinessState::Unproven,
            ),
        ],
        Vec::new(),
    );
    let request = ObservationLensRequest {
        filter: ObservationFilter::all(),
        maximum_inspected: 0,
        maximum_nodes: 1,
        maximum_copied_string_bytes: 0,
        include_causal_paths: true,
        maximum_causal_path_lookups: 0,
        maximum_causal_steps: 0,
        after: None,
        cancellation: ObservationCancellation::Active,
    };
    let ObservationLensDisposition::Ready(first) = snapshot.visual_lens(request.clone()) else {
        panic!("active request must yield a lens");
    };
    let ObservationLensDisposition::Ready(second) = snapshot.visual_lens(request) else {
        panic!("active request must yield a lens");
    };
    assert_eq!(first, second);
    assert_eq!(first.nodes.len(), 1);
    assert!(first.continuation.is_some());
    assert_eq!(first.cursor, snapshot.cursor().clone());
    assert_eq!(first.nodes[0].facts.len(), 1);
    assert_eq!(
        first.nodes[0].facts[0].category(),
        ObservationCategory::Health
    );
    assert_eq!(first.snapshot_omitted_count, 0);
}

#[test]
fn lens_resumes_by_subject_and_preserves_cancellation_position() {
    let snapshot = snapshot(
        generation('3'),
        ObservationRevision::Static,
        vec![
            fact(
                ObservationCategory::Readiness,
                "machine-a",
                ObservationReadinessState::Ready,
            ),
            fact(
                ObservationCategory::Readiness,
                "machine-b",
                ObservationReadinessState::Ready,
            ),
        ],
        Vec::new(),
    );
    let request = |cancellation, after| ObservationLensRequest {
        filter: ObservationFilter::all(),
        maximum_inspected: 1,
        maximum_nodes: 8,
        maximum_copied_string_bytes: 1_024,
        include_causal_paths: false,
        maximum_causal_path_lookups: 1,
        maximum_causal_steps: 1,
        after,
        cancellation,
    };

    let ObservationLensDisposition::Ready(first) =
        snapshot.visual_lens(request(ObservationCancellation::Active, None))
    else {
        panic!("active lens request must produce a bounded step");
    };
    assert_eq!(first.nodes.len(), 1);
    assert_eq!(first.inspected_subjects, 1);
    assert_eq!(
        first.completion,
        ObservationStepCompletion::Stopped(ObservationStepLimit::Inspected)
    );
    let cursor = first.continuation.expect("one subject remains");
    assert_eq!(cursor.inspected_position(), 1);

    let ObservationLensDisposition::Cancelled { continuation } = snapshot.visual_lens(request(
        ObservationCancellation::Cancelled,
        Some(cursor.clone()),
    )) else {
        panic!("cancellation must not publish a partial lens");
    };
    assert_eq!(continuation, Some(cursor.clone()));

    let ObservationLensDisposition::Ready(second) =
        snapshot.visual_lens(request(ObservationCancellation::Active, Some(cursor)))
    else {
        panic!("the exact continuation must resume the next subject");
    };
    assert_eq!(second.nodes.len(), 1);
    assert_eq!(
        second.nodes[0].subject,
        ObservationSubject::Machine("machine-b".to_owned())
    );
}

#[test]
fn lens_continuation_binds_every_raw_limit() {
    let snapshot = snapshot(
        generation('4'),
        ObservationRevision::Static,
        vec![fact(
            ObservationCategory::Readiness,
            "machine-a",
            ObservationReadinessState::Ready,
        )],
        Vec::new(),
    );
    let first_request = ObservationLensRequest {
        filter: ObservationFilter::all(),
        maximum_inspected: 1,
        maximum_nodes: 8,
        maximum_copied_string_bytes: 1_024,
        include_causal_paths: false,
        maximum_causal_path_lookups: 1,
        maximum_causal_steps: 1,
        after: None,
        cancellation: ObservationCancellation::Active,
    };
    let ObservationLensDisposition::Ready(first) = snapshot.visual_lens(first_request.clone())
    else {
        panic!("active lens request must produce a bounded step");
    };
    let cursor = first
        .continuation
        .expect("the bounded step issues a cursor");

    let mismatches = [
        ObservationLensRequest {
            maximum_inspected: 2,
            after: Some(cursor.clone()),
            ..first_request.clone()
        },
        ObservationLensRequest {
            maximum_nodes: 9,
            after: Some(cursor.clone()),
            ..first_request.clone()
        },
        ObservationLensRequest {
            maximum_copied_string_bytes: 1_025,
            after: Some(cursor.clone()),
            ..first_request.clone()
        },
        ObservationLensRequest {
            include_causal_paths: true,
            after: Some(cursor.clone()),
            ..first_request.clone()
        },
        ObservationLensRequest {
            maximum_causal_path_lookups: 2,
            after: Some(cursor.clone()),
            ..first_request.clone()
        },
        ObservationLensRequest {
            maximum_causal_steps: 2,
            after: Some(cursor.clone()),
            ..first_request.clone()
        },
        ObservationLensRequest {
            filter: ObservationFilter::all().with_category(ObservationCategory::Health),
            after: Some(cursor),
            ..first_request
        },
    ];
    for mismatch in mismatches {
        assert!(matches!(
            snapshot.visual_lens(mismatch),
            ObservationLensDisposition::Resynchronize(ref value)
                if value.reason == ObservationResynchronizationReason::ContinuationMismatch
        ));
    }
}

#[test]
fn lens_bounds_causal_lookups_and_complete_paths_independently() {
    let generation = generation('5');
    let first_subject = ObservationSubject::Machine("machine-a".to_owned());
    let second_subject = ObservationSubject::Machine("machine-b".to_owned());
    let bounded = snapshot(
        generation.clone(),
        ObservationRevision::Static,
        vec![
            fact(
                ObservationCategory::Readiness,
                "machine-a",
                ObservationReadinessState::Ready,
            ),
            fact(
                ObservationCategory::Readiness,
                "machine-b",
                ObservationReadinessState::Ready,
            ),
        ],
        vec![
            causal_path(generation.clone(), first_subject, &["entry-a"]),
            causal_path(generation.clone(), second_subject, &["entry-b"]),
        ],
    );
    let request = ObservationLensRequest {
        filter: ObservationFilter::all(),
        maximum_inspected: 8,
        maximum_nodes: 8,
        maximum_copied_string_bytes: 4_096,
        include_causal_paths: true,
        maximum_causal_path_lookups: 1,
        maximum_causal_steps: 8,
        after: None,
        cancellation: ObservationCancellation::Active,
    };
    let ObservationLensDisposition::Ready(lens) = bounded.visual_lens(request) else {
        panic!("active lens request must produce a bounded step");
    };
    assert_eq!(lens.nodes.len(), 1);
    assert_eq!(lens.causal_paths.len(), 1);
    assert_eq!(lens.causal_path_lookups, 1);
    assert_eq!(lens.causal_steps, 1);
    assert_eq!(
        lens.completion,
        ObservationStepCompletion::Stopped(ObservationStepLimit::CausalPathLookups)
    );
    assert!(lens.continuation.is_some());

    let oversized = snapshot(
        generation.clone(),
        ObservationRevision::Static,
        vec![fact(
            ObservationCategory::Readiness,
            "machine-c",
            ObservationReadinessState::Ready,
        )],
        vec![causal_path(
            generation,
            ObservationSubject::Machine("machine-c".to_owned()),
            &["entry-c", "entry-d"],
        )],
    );
    let ObservationLensDisposition::Ready(lens) = oversized.visual_lens(ObservationLensRequest {
        filter: ObservationFilter::all(),
        maximum_inspected: 8,
        maximum_nodes: 8,
        maximum_copied_string_bytes: 4_096,
        include_causal_paths: true,
        maximum_causal_path_lookups: 8,
        maximum_causal_steps: 1,
        after: None,
        cancellation: ObservationCancellation::Active,
    }) else {
        panic!("active lens request must produce a bounded step");
    };
    assert!(lens.nodes.is_empty());
    assert!(lens.causal_paths.is_empty());
    assert_eq!(lens.causal_path_lookups, 1);
    assert_eq!(lens.causal_steps, 0);
    assert_eq!(
        lens.completion,
        ObservationStepCompletion::Incomplete(ObservationStepLimit::CausalSteps {
            minimum_required: 2,
        })
    );
    assert!(lens.continuation.is_none());
}

#[test]
fn lens_retries_the_unreturned_subject_after_causal_resource_stops() {
    let generation = generation('6');
    let first_subject = ObservationSubject::Machine("machine-a".to_owned());
    let second_subject = ObservationSubject::Machine("machine-b".to_owned());
    let bounded = snapshot(
        generation.clone(),
        ObservationRevision::Static,
        vec![
            fact(
                ObservationCategory::Readiness,
                "machine-a",
                ObservationReadinessState::Ready,
            ),
            fact(
                ObservationCategory::Readiness,
                "machine-b",
                ObservationReadinessState::Ready,
            ),
        ],
        vec![
            causal_path(generation.clone(), first_subject.clone(), &["entry-a"]),
            causal_path(generation, second_subject.clone(), &["entry-b"]),
        ],
    );
    let request = |after| ObservationLensRequest {
        filter: ObservationFilter::all(),
        maximum_inspected: 8,
        maximum_nodes: 8,
        maximum_copied_string_bytes: 4_096,
        include_causal_paths: true,
        maximum_causal_path_lookups: 8,
        maximum_causal_steps: 1,
        after,
        cancellation: ObservationCancellation::Active,
    };

    let ObservationLensDisposition::Ready(first) = bounded.visual_lens(request(None)) else {
        panic!("active lens request must produce a bounded step");
    };
    assert_eq!(first.nodes.len(), 1);
    assert_eq!(first.nodes[0].subject, first_subject);
    assert_eq!(first.causal_path_lookups, 2);
    assert_eq!(first.causal_steps, 1);
    assert_eq!(
        first.completion,
        ObservationStepCompletion::Stopped(ObservationStepLimit::CausalSteps {
            minimum_required: 1,
        })
    );
    let continuation = first
        .continuation
        .expect("the second subject fits a fresh bounded step");
    assert_eq!(continuation.inspected_position(), 1);

    let ObservationLensDisposition::Ready(second) =
        bounded.visual_lens(request(Some(continuation)))
    else {
        panic!("the continuation must retry the unreturned subject");
    };
    assert_eq!(second.nodes.len(), 1);
    assert_eq!(second.nodes[0].subject, second_subject);
    assert_eq!(second.causal_path_lookups, 1);
    assert_eq!(second.causal_steps, 1);
}

#[test]
fn lens_separates_copy_stops_from_unrepresentable_nodes() {
    let first = fact(
        ObservationCategory::Readiness,
        "machine-a",
        ObservationReadinessState::Ready,
    );
    let second = fact(
        ObservationCategory::Readiness,
        "machine-b",
        ObservationReadinessState::Ready,
    );
    let node_bytes = ObservationSubject::Machine("machine-a".to_owned())
        .copied_string_bytes()
        .saturating_add(first.copied_string_bytes());
    let bounded = snapshot(
        generation('7'),
        ObservationRevision::Static,
        vec![first, second],
        Vec::new(),
    );
    let request = |maximum_copied_string_bytes| ObservationLensRequest {
        filter: ObservationFilter::all(),
        maximum_inspected: 8,
        maximum_nodes: 8,
        maximum_copied_string_bytes,
        include_causal_paths: false,
        maximum_causal_path_lookups: 0,
        maximum_causal_steps: 0,
        after: None,
        cancellation: ObservationCancellation::Active,
    };

    let ObservationLensDisposition::Ready(stopped) = bounded.visual_lens(request(node_bytes))
    else {
        panic!("active lens request must produce a bounded step");
    };
    assert_eq!(stopped.nodes.len(), 1);
    assert_eq!(stopped.copied_string_bytes, node_bytes);
    assert_eq!(
        stopped.completion,
        ObservationStepCompletion::Stopped(ObservationStepLimit::CopiedStringBytes {
            minimum_required: node_bytes,
        })
    );
    assert!(stopped.continuation.is_some());

    let ObservationLensDisposition::Ready(incomplete) =
        bounded.visual_lens(request(node_bytes - 1))
    else {
        panic!("active lens request must produce a bounded step");
    };
    assert!(incomplete.nodes.is_empty());
    assert_eq!(incomplete.inspected_subjects, 1);
    assert_eq!(
        incomplete.completion,
        ObservationStepCompletion::Incomplete(ObservationStepLimit::CopiedStringBytes {
            minimum_required: node_bytes,
        })
    );
    assert!(incomplete.continuation.is_none());
}

#[test]
fn snapshot_bound_reports_omitted_owner_facts() {
    let requested = MAX_OBSERVATION_FACTS + 7;
    let facts = (0..requested)
        .map(|index| {
            fact(
                ObservationCategory::Readiness,
                &format!("machine-{index:04}"),
                ObservationReadinessState::Unproven,
            )
        })
        .collect();
    let snapshot = snapshot(
        generation('d'),
        ObservationRevision::Static,
        facts,
        Vec::new(),
    );

    assert_eq!(snapshot.fact_count(), MAX_OBSERVATION_FACTS);
    assert_eq!(snapshot.omitted_fact_count(), 7);
    let ObservationLensDisposition::Ready(lens) = snapshot.visual_lens(ObservationLensRequest {
        filter: ObservationFilter::all(),
        maximum_inspected: 0,
        maximum_nodes: 1,
        maximum_copied_string_bytes: 0,
        include_causal_paths: false,
        maximum_causal_path_lookups: 0,
        maximum_causal_steps: 0,
        after: None,
        cancellation: ObservationCancellation::Active,
    }) else {
        panic!("active request must yield a lens");
    };
    assert_eq!(lens.snapshot_omitted_count, 7);
}

#[test]
fn truncated_delta_installs_the_complete_successor_omission_count() {
    let facts = |count| {
        (0..count)
            .map(|index| {
                fact(
                    ObservationCategory::Readiness,
                    &format!("machine-{index:05}"),
                    ObservationReadinessState::Unproven,
                )
            })
            .collect()
    };
    let prior = snapshot(
        resident_generation(),
        ObservationRevision::ResidentJournal(Some(JournalSequenceV1::new(1).unwrap())),
        facts(MAX_OBSERVATION_FACTS + 1),
        Vec::new(),
    );
    let current = snapshot(
        resident_generation(),
        ObservationRevision::ResidentJournal(Some(JournalSequenceV1::new(2).unwrap())),
        facts(MAX_OBSERVATION_FACTS + 2),
        Vec::new(),
    );
    let ObservationDeltaOutcome::Delta(delta) = current.delta_from(
        &prior,
        ObservationDeltaRequest {
            resume: prior.resume_coordinate(),
            maximum_changes: 8,
            cancellation: ObservationCancellation::Active,
        },
    ) else {
        panic!("adjacent bounded snapshots must produce a complete delta");
    };
    assert!(delta.changes.is_empty());
    assert_eq!(delta.successor_omitted_fact_count, 2);
}

#[test]
fn readiness_meet_obeys_its_semilattice_laws_and_precedence() {
    use ObservationReadinessState::{Blocked, Narrowed, Ready, Unavailable, Unproven};

    let states = [Ready, Unproven, Unavailable, Narrowed, Blocked];
    for left in states {
        assert_eq!(readiness_meet(left, Ready), left);
        assert_eq!(readiness_meet(left, left), left);
        for right in states {
            assert_eq!(readiness_meet(left, right), readiness_meet(right, left));
            for third in states {
                assert_eq!(
                    readiness_meet(readiness_meet(left, right), third),
                    readiness_meet(left, readiness_meet(right, third))
                );
            }
        }
    }

    assert_eq!(readiness_meet(Ready, Unproven), Unproven);
    assert_eq!(readiness_meet(Unproven, Unavailable), Unavailable);
    assert_eq!(readiness_meet(Unavailable, Narrowed), Narrowed);
    assert_eq!(readiness_meet(Narrowed, Blocked), Blocked);
}
