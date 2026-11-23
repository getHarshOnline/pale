//! Review lifecycle, ownership, and diagnostic parity through the dispatcher.

use pale_sdk::{
    ConfigurationImprovementCandidateRequest, ConfigurationImprovementDisposition,
    ConfigurationImprovementExplanation, ConfigurationImprovementPhase,
    ConfigurationImprovementProposalRequest, PaleApi,
};

use crate::command::{
    CommandDispatcher, CommandOutcome, CommandRequest, ConfigurationImprovementCommand as Review,
    ConfigurationImprovementOutcome as Outcome, ConfigurationImprovementSources,
};
use crate::test_fixtures::{
    captured_profile_sources, captured_workspace, effective_profile_request,
};

fn dispatch(dispatcher: &CommandDispatcher, command: Review) -> Outcome {
    match dispatcher
        .dispatch(CommandRequest::ConfigurationImprovement(command))
        .unwrap()
    {
        CommandOutcome::ConfigurationImprovement(outcome) => outcome,
        _ => panic!("review command returned a different operation's outcome"),
    }
}

fn candidate(project: &str) -> ConfigurationImprovementProposalRequest {
    ConfigurationImprovementProposalRequest::new(
        project.to_owned(),
        "default".to_owned(),
        "machine".to_owned(),
        captured_workspace("read"),
        captured_profile_sources().0,
    )
}

fn candidate_profile(project: &str) -> ConfigurationImprovementCandidateRequest {
    ConfigurationImprovementCandidateRequest::new(
        project.to_owned(),
        "default".to_owned(),
        "machine".to_owned(),
        captured_profile_sources().0,
    )
}

fn review_sources() -> Box<ConfigurationImprovementSources> {
    let (workspace, profile) = effective_profile_request();
    Box::new(ConfigurationImprovementSources::new(
        workspace,
        profile,
        candidate("demo"),
    ))
}

#[test]
fn source_impact_queries_check_the_review_and_delegate_paging_to_the_sdk() {
    let dispatcher = CommandDispatcher::new().unwrap();
    let api = PaleApi::new().unwrap();
    let preview = api
        .preview_configuration_improvement_sources(*review_sources())
        .unwrap();
    let summary = preview.summary().canonical_bytes().to_vec();
    for (offset, limit) in [(0, 0), (0, 1), (1, 1), (0, u16::MAX), (u32::MAX, 0)] {
        let Outcome::Impacts(page) = dispatch(
            &dispatcher,
            Review::ImpactsSources {
                sources: review_sources(),
                presented_summary: summary.clone(),
                offset,
                limit,
            },
        ) else {
            panic!("impact query returned an unrelated review outcome");
        };
        assert_eq!(page, preview.comparison().impacts(offset, limit));
    }

    let subject = preview.comparison().impacts(0, 1).entries()[0]
        .subject()
        .selector();
    let Outcome::SourceDetails {
        summary: checked_summary,
        explanation,
        source,
        source_page_state,
    } = dispatch(
        &dispatcher,
        Review::InspectSources {
            sources: review_sources(),
            presented_summary: summary.clone(),
            subject: subject.clone(),
            input_limit: 64,
            maximum_inspected: 64,
            maximum_returned: 64,
        },
    )
    else {
        panic!("source inspection returned an unrelated review outcome");
    };
    assert_eq!(checked_summary.canonical_bytes(), summary);
    assert_eq!(explanation.subject().selector(), subject);
    assert_eq!(source.subject().selector(), subject);
    assert_eq!(
        source_page_state,
        pale_sdk::ConfigurationImprovementSourcePageState::RetainedReviewRequired,
    );
    let source_projection =
        serde_json::from_slice::<serde_json::Value>(source.canonical_bytes()).unwrap();
    assert_eq!(
        source_projection,
        serde_json::to_value(&source).unwrap(),
        "direct serialization must preserve the canonical source projection",
    );
    assert_eq!(
        source_projection["dependencyCopiedStringBytes"],
        source.dependency_copied_string_bytes()
    );
    assert_eq!(
        source_projection["sourceCopiedStringBytes"],
        source.source_copied_string_bytes()
    );

    let error = dispatcher
        .dispatch(CommandRequest::ConfigurationImprovement(
            Review::ImpactsSources {
                sources: review_sources(),
                presented_summary: b"{}".to_vec(),
                offset: 1,
                limit: 1,
            },
        ))
        .unwrap_err();
    assert_eq!(
        error.diagnostic().code(),
        "configuration_improvement_summary_mismatch"
    );

    let error = dispatcher
        .dispatch(CommandRequest::ConfigurationImprovement(
            Review::InspectSources {
                sources: review_sources(),
                presented_summary: b"{}".to_vec(),
                subject,
                input_limit: 64,
                maximum_inspected: 64,
                maximum_returned: 64,
            },
        ))
        .unwrap_err();
    assert_eq!(
        error.diagnostic().code(),
        "configuration_improvement_summary_mismatch"
    );
}

#[test]
fn fresh_review_finalization_preserves_sdk_comparison_and_diagnostics() {
    let dispatcher = CommandDispatcher::new().unwrap();
    let api = PaleApi::new().unwrap();
    let Outcome::SourcePreviewed { preview, impacts } =
        dispatch(&dispatcher, Review::PreviewSources(review_sources()))
    else {
        panic!("source review did not return a preview");
    };
    assert_eq!(impacts, preview.comparison().impacts(0, 0));
    let summary = preview.summary().canonical_bytes().to_vec();
    for disposition in [
        ConfigurationImprovementDisposition::Accepted,
        ConfigurationImprovementDisposition::Rejected,
        ConfigurationImprovementDisposition::Cancelled,
        ConfigurationImprovementDisposition::Stale,
    ] {
        let Outcome::Finished(terminal) = dispatch(
            &dispatcher,
            Review::FinishSources {
                sources: review_sources(),
                presented_summary: summary.clone(),
                disposition,
            },
        ) else {
            panic!("matching review did not return its decision");
        };
        assert_eq!(terminal.disposition(), disposition);
        assert_eq!(terminal.preview().canonical_bytes(), summary);
    }
    for input in [
        b"{}".to_vec(),
        b"{\"private-marker\":1,\"private-marker\":2}".to_vec(),
    ] {
        let Outcome::SourcePreviewed { preview, .. } =
            dispatch(&dispatcher, Review::PreviewSources(review_sources()))
        else {
            panic!("source review did not return a preview");
        };
        let direct = api
            .finish_presented_configuration_improvement(
                *preview,
                &input,
                ConfigurationImprovementDisposition::Accepted,
            )
            .err()
            .unwrap();
        let projected = dispatcher
            .dispatch(CommandRequest::ConfigurationImprovement(
                Review::FinishSources {
                    sources: review_sources(),
                    presented_summary: input,
                    disposition: ConfigurationImprovementDisposition::Accepted,
                },
            ))
            .err()
            .unwrap();
        assert_eq!(
            direct.diagnostic().canonical_json(),
            projected.diagnostic().canonical_json()
        );
        assert!(!projected
            .diagnostic()
            .canonical_json()
            .contains("private-marker"));
    }
}

fn explained(api: &PaleApi, dispatcher: &CommandDispatcher) -> ConfigurationImprovementExplanation {
    let (workspace, profile) = effective_profile_request();
    let baseline = api.compile_portable_workspace(workspace).unwrap();
    let profile = api.compile_effective_profile(&baseline, profile).unwrap();
    let Outcome::Inspected(inspection) = dispatch(dispatcher, Review::Inspect(Box::new(baseline)))
    else {
        panic!("inspection was not retained");
    };
    assert_eq!(inspection.phase(), ConfigurationImprovementPhase::Inspected);
    let Outcome::Explained(explanation) = dispatch(
        dispatcher,
        Review::Explain {
            inspection,
            profile: Box::new(profile),
        },
    ) else {
        panic!("explanation was not retained");
    };
    assert_eq!(
        explanation.phase(),
        ConfigurationImprovementPhase::Explained
    );
    *explanation
}

#[test]
fn staged_and_source_first_reviews_retain_the_same_proof_and_explicit_decisions() {
    let api = PaleApi::new().unwrap();
    let dispatcher = CommandDispatcher::new().unwrap();
    let explanation = explained(&api, &dispatcher);
    let Outcome::Proposed(proposal) = dispatch(
        &dispatcher,
        Review::Propose {
            explanation,
            candidate: Box::new(candidate("demo")),
        },
    ) else {
        panic!("proposal was not retained");
    };
    assert_eq!(proposal.phase(), ConfigurationImprovementPhase::Proposed);
    let Outcome::Evaluated(evaluation) = dispatch(&dispatcher, Review::Evaluate(proposal)) else {
        panic!("evaluation was not retained");
    };
    assert_eq!(evaluation.phase(), ConfigurationImprovementPhase::Evaluated);
    let Outcome::Diagnosed(diagnosis) = dispatch(&dispatcher, Review::Diagnose(evaluation)) else {
        panic!("diagnosis was not retained");
    };
    assert_eq!(diagnosis.phase(), ConfigurationImprovementPhase::Diagnosed);
    let Outcome::Previewed(preview) = dispatch(&dispatcher, Review::Preview(diagnosis)) else {
        panic!("preview was not retained");
    };
    assert_eq!(preview.phase(), ConfigurationImprovementPhase::Previewed);
    let expected_summary = preview.summary().canonical_bytes().to_vec();
    let expected_impacts = preview.comparison().impacts(0, 2);
    let expected_initial_impacts = preview.comparison().impacts(0, 0);
    assert_ne!(
        preview.summary().profile_digest(),
        preview.summary().candidate_profile_digest()
    );
    assert_eq!(
        preview.baseline_profile().profile_digest(),
        preview.summary().profile_digest(),
    );
    let Outcome::Finished(terminal) = dispatch(
        &dispatcher,
        Review::Finish {
            preview,
            disposition: ConfigurationImprovementDisposition::Rejected,
        },
    ) else {
        panic!("rejection did not consume the preview");
    };
    assert_eq!(
        terminal.disposition(),
        ConfigurationImprovementDisposition::Rejected
    );
    assert_eq!(terminal.preview().canonical_bytes(), expected_summary);

    for disposition in [
        ConfigurationImprovementDisposition::Accepted,
        ConfigurationImprovementDisposition::Rejected,
        ConfigurationImprovementDisposition::Cancelled,
        ConfigurationImprovementDisposition::Stale,
    ] {
        let (workspace, profile) = effective_profile_request();
        let Outcome::SourcePreviewed { preview, impacts } = dispatch(
            &dispatcher,
            Review::PreviewSources(Box::new(ConfigurationImprovementSources::new(
                workspace,
                profile,
                candidate("demo"),
            ))),
        ) else {
            panic!("source-first review must stop at preview");
        };
        assert_eq!(preview.summary().canonical_bytes(), expected_summary);
        assert_eq!(impacts, expected_initial_impacts);
        assert_eq!(preview.comparison().impacts(0, 2), expected_impacts);
        // This checks preservation of the caller's disposition, not proof of
        // an editor or native revision change. That proof belongs to the carrier.
        let Outcome::Finished(terminal) = dispatch(
            &dispatcher,
            Review::Finish {
                preview,
                disposition,
            },
        ) else {
            panic!("explicit decision did not consume the preview");
        };
        assert_eq!(terminal.disposition(), disposition);
        assert_eq!(terminal.preview().canonical_bytes(), expected_summary);
    }
}

#[test]
fn failed_preparation_keeps_the_explanation_for_retry_and_commit() {
    let api = PaleApi::new().unwrap();
    let dispatcher = CommandDispatcher::new().unwrap();
    let explanation = explained(&api, &dispatcher);
    let candidate = api
        .compile_portable_workspace(captured_workspace("read"))
        .unwrap();
    let direct = api
        .prepare_configuration_improvement_preview(
            &explanation,
            &candidate,
            candidate_profile("unselected"),
        )
        .err()
        .expect("the alternate context is not admitted");
    let dispatched = dispatcher
        .prepare_configuration_improvement_preview(
            &explanation,
            &candidate,
            candidate_profile("unselected"),
        )
        .err()
        .expect("dispatch must preserve the rejected context");
    assert_eq!(
        dispatched.diagnostic().canonical_bytes(),
        direct.diagnostic().canonical_bytes()
    );
    let prepared = dispatcher
        .prepare_configuration_improvement_preview(
            &explanation,
            &candidate,
            candidate_profile("demo"),
        )
        .expect("the original explanation remains available after failure");
    let expected = prepared.summary().canonical_bytes().to_vec();
    let Outcome::Previewed(preview) = dispatch(
        &dispatcher,
        Review::Commit {
            explanation,
            prepared: Box::new(prepared),
        },
    ) else {
        panic!("the originating explanation must commit its preparation");
    };
    assert_eq!(preview.summary().canonical_bytes(), expected);
}

#[test]
fn equal_coordinates_do_not_replace_the_preparation_owner() {
    let api = PaleApi::new().unwrap();
    let dispatcher = CommandDispatcher::new().unwrap();
    let origin = explained(&api, &dispatcher);
    let replacement = explained(&api, &dispatcher);
    assert_eq!(origin.profile_digest(), replacement.profile_digest());
    assert_eq!(origin.instance_digest(), replacement.instance_digest());
    let candidate = api
        .compile_portable_workspace(captured_workspace("read"))
        .unwrap();
    let prepared = dispatcher
        .prepare_configuration_improvement_preview(&origin, &candidate, candidate_profile("demo"))
        .unwrap();
    let error = dispatcher
        .dispatch(CommandRequest::ConfigurationImprovement(Review::Commit {
            explanation: replacement,
            prepared: Box::new(prepared),
        }))
        .expect_err("matching digests cannot reconstruct an opaque lifecycle");
    super::assert_canonical_failure(
        &error,
        "configuration_improvement_preparation_owner_mismatch",
    );
}

#[test]
fn source_first_review_preserves_candidate_rejection_and_redacts_debug() {
    let dispatcher = CommandDispatcher::new().unwrap();
    let (workspace, profile) = effective_profile_request();
    let request = CommandRequest::ConfigurationImprovement(Review::PreviewSources(Box::new(
        ConfigurationImprovementSources::new(
            workspace,
            profile,
            candidate("private-unselected-project"),
        ),
    )));
    assert!(!format!("{request:?}").contains("private-unselected-project"));
    let error = dispatcher
        .dispatch(request)
        .expect_err("a mismatched candidate cannot produce a preview");
    super::assert_canonical_failure(
        &error,
        "configuration_improvement_candidate_context_mismatch",
    );
}
