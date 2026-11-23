//! Complete configuration-review laws at the MCP-to-dispatcher boundary.

use pale_cli::command::{
    CommandDispatcher, CommandOutcome, CommandRequest, ConfigurationImprovementCommand,
    ConfigurationImprovementOutcome,
};
use serde_json::{json, Value};

use super::review_fixture;
use crate::tools::FacadeTool;

fn command(input: Value) -> pale_cli::command::CommandRequest {
    FacadeTool::ReviewConfiguration
        .command(input.as_object().unwrap().clone())
        .unwrap()
}

#[test]
fn configuration_review_projects_owned_preview_and_all_checked_terminals() {
    let dispatcher = CommandDispatcher::new().unwrap();
    let outcome = dispatcher
        .dispatch(command(review_fixture::preview()))
        .unwrap();
    let CommandOutcome::ConfigurationImprovement(
        ConfigurationImprovementOutcome::SourcePreviewed { preview, impacts },
    ) = &outcome
    else {
        panic!("review returns its exact preview variant");
    };
    let expected_summary: Value =
        serde_json::from_slice(preview.summary().canonical_bytes()).unwrap();
    assert_eq!(expected_summary, review_fixture::expected()["summary"]);
    let expected_impacts: Value = serde_json::from_slice(impacts.canonical_bytes()).unwrap();
    let projected = FacadeTool::ReviewConfiguration
        .project(outcome)
        .unwrap()
        .structured_content
        .unwrap();
    assert_eq!(
        projected,
        json!({
            "state": "preview",
            "summary": expected_summary,
            "impacts": expected_impacts,
        })
    );
    let repeated = dispatcher
        .dispatch(command(review_fixture::preview()))
        .unwrap();
    let repeated = FacadeTool::ReviewConfiguration
        .project(repeated)
        .unwrap()
        .structured_content
        .unwrap();
    assert_eq!(
        serde_json::to_vec(&projected).unwrap(),
        serde_json::to_vec(&repeated).unwrap(),
        "the same typed dispatcher result must have stable MCP JSON"
    );
    let subject = projected["impacts"]["entries"][0]["subject"].clone();
    let details = dispatcher
        .dispatch(command(review_fixture::inspect(
            expected_summary.clone(),
            subject.clone(),
        )))
        .unwrap();
    let details = FacadeTool::ReviewConfiguration
        .project(details)
        .unwrap()
        .structured_content
        .unwrap();
    assert_eq!(details["state"], "details");
    assert_eq!(details["summary"], expected_summary);
    assert_eq!(details["explanation"]["subject"], subject);
    assert_eq!(details["source"]["subject"], subject);
    assert!(details["source"]["occurrences"].is_array());
    assert!(details["source"]["inputsComplete"].is_boolean());
    assert!(details["source"]["sourcesComplete"].is_boolean());
    assert_eq!(details["sourcePageState"], "retained_review_required");
    assert!(
        !details["source"]["inputsComplete"].as_bool().unwrap()
            || !details["source"]["sourcesComplete"].as_bool().unwrap(),
        "the stateless limitation must describe genuinely unfinished evidence",
    );
    assert!(details["source"].get("recordAnchor").is_some());
    for disposition in ["accepted", "rejected", "cancelled", "stale"] {
        let outcome = dispatcher
            .dispatch(command(review_fixture::finish(
                expected_summary.clone(),
                disposition,
            )))
            .unwrap();
        assert_eq!(
            FacadeTool::ReviewConfiguration
                .project(outcome)
                .unwrap()
                .structured_content
                .unwrap(),
            json!({"state": "finished", "summary": expected_summary, "disposition": disposition})
        );
    }
}

#[test]
fn configuration_review_preserves_the_sdk_summary_as_direct_bytes() {
    let dispatcher = CommandDispatcher::new().unwrap();
    let outcome = dispatcher
        .dispatch(command(review_fixture::preview()))
        .unwrap();
    let CommandOutcome::ConfigurationImprovement(
        ConfigurationImprovementOutcome::SourcePreviewed { preview, .. },
    ) = outcome
    else {
        panic!("review returns its exact preview variant");
    };
    let summary: Value = serde_json::from_slice(preview.summary().canonical_bytes()).unwrap();
    let command = command(review_fixture::finish(summary, "accepted"));
    let CommandRequest::ConfigurationImprovement(ConfigurationImprovementCommand::FinishSources {
        presented_summary,
        ..
    }) = command
    else {
        panic!("finish input selects the stateless source-review route");
    };
    assert_eq!(presented_summary, preview.summary().canonical_bytes());
}

#[test]
fn configuration_review_keeps_summary_validation_at_the_sdk_owner() {
    let dispatcher = CommandDispatcher::new().unwrap();
    for summary in [json!({}), json!({"private":"REJECTED-PRIVATE-MARKER"})] {
        let error = dispatcher
            .dispatch(command(review_fixture::finish(summary, "accepted")))
            .unwrap_err();
        let diagnostic: Value =
            serde_json::from_slice(error.diagnostic().canonical_bytes()).unwrap();
        assert_eq!(
            diagnostic["code"],
            "configuration_improvement_summary_mismatch"
        );
        assert_eq!(diagnostic["effect"], "not_applicable");
        assert!(!serde_json::to_string(&diagnostic)
            .unwrap()
            .contains("REJECTED-PRIVATE-MARKER"));
    }
    let mut input = review_fixture::preview();
    input["candidate"]["projectId"] = json!("unselected");
    let error = dispatcher.dispatch(command(input)).unwrap_err();
    let actual: Value = serde_json::from_slice(error.diagnostic().canonical_bytes()).unwrap();
    assert_eq!(
        actual,
        review_fixture::expected()["diagnostics"]["context_mismatch"]
    );
}

#[test]
fn configuration_review_rejects_acceptance_after_the_candidate_changes() {
    let dispatcher = CommandDispatcher::new().unwrap();
    let outcome = dispatcher
        .dispatch(command(review_fixture::preview()))
        .unwrap();
    let CommandOutcome::ConfigurationImprovement(
        ConfigurationImprovementOutcome::SourcePreviewed { preview, .. },
    ) = outcome
    else {
        panic!("review returns its exact preview variant");
    };
    let summary: Value = serde_json::from_slice(preview.summary().canonical_bytes()).unwrap();

    let mut changed = review_fixture::finish(summary, "accepted");
    let expected = review_fixture::expected();
    let candidate_root = expected["candidate_machine_root"].as_str().unwrap();
    let binding = &mut changed["candidate"]["privateInputs"]["machineBinding"]["content"];
    *binding = Value::String(
        binding
            .as_str()
            .unwrap()
            .replace(candidate_root, "/captured/changed-after-preview"),
    );

    let error = dispatcher.dispatch(command(changed)).unwrap_err();
    let diagnostic: Value = serde_json::from_slice(error.diagnostic().canonical_bytes()).unwrap();
    assert_eq!(
        diagnostic["code"],
        "configuration_improvement_summary_mismatch"
    );
    assert_eq!(diagnostic["effect"], "not_applicable");
}

#[test]
fn configuration_review_rejects_candidate_package_and_wrong_transport_shapes() {
    let mut injected = review_fixture::preview();
    injected["candidate"]["packageDocuments"] = json!([]);
    let mut unknown_action = review_fixture::preview();
    unknown_action["action"] = json!("apply");
    let mut stringly_summary = review_fixture::finish(json!({}), "accepted");
    stringly_summary["summary"] = json!("{}");
    for input in [
        injected,
        unknown_action,
        stringly_summary,
        review_fixture::finish(json!({}), "publish"),
    ] {
        assert!(FacadeTool::ReviewConfiguration
            .command(input.as_object().unwrap().clone())
            .is_err());
    }
}
