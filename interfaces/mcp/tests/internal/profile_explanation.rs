//! Profile explanation transport preserves the shared lookup and diagnostic owners.

use pale_cli::command::{CommandDispatcher, CommandOutcome};
use serde_json::{json, Value};

use super::review_fixture;
use crate::tools::FacadeTool;

#[test]
fn profile_explanation_projects_exact_owned_evidence_and_absence() {
    let dispatcher = CommandDispatcher::new().unwrap();
    let inspect = FacadeTool::InspectEffectiveProfile;
    let arguments = json!({"compilation": review_fixture::baseline(), "section": "machine-realization", "offset": 0, "limit": 1});
    let page = dispatcher
        .dispatch(
            inspect
                .command(arguments.as_object().unwrap().clone())
                .unwrap(),
        )
        .unwrap();
    let value = inspect.project(page).unwrap().structured_content.unwrap();
    let handle = value["entries"][0]["explanationHandle"].as_str().unwrap();
    let explain = FacadeTool::ExplainEffectiveProfileEntry;
    for requested in [handle.to_owned(), "0".repeat(64)] {
        let arguments = json!({"compilation": review_fixture::baseline(), "handle": requested});
        let outcome = dispatcher
            .dispatch(
                explain
                    .command(arguments.as_object().unwrap().clone())
                    .unwrap(),
            )
            .unwrap();
        let CommandOutcome::EffectiveProfileExplanation(explanation) = &outcome else {
            panic!("exact dispatcher outcome");
        };
        assert_eq!(explanation.is_some(), requested == handle);
        let expected = serde_json::to_value(explanation).unwrap();
        assert_eq!(
            explain.project(outcome).unwrap().structured_content,
            Some(json!({"explanation": expected}))
        );
    }
}

#[test]
fn profile_explanation_preserves_compiler_diagnostics_and_rejects_bad_handles() {
    let dispatcher = CommandDispatcher::new().unwrap();
    let mut compilation = review_fixture::baseline();
    compilation["projectId"] = json!("unselected");
    let inspect = json!({"compilation": compilation, "section": "machine-realization", "offset": 0, "limit": 1});
    let explain = json!({"compilation": compilation, "handle": "0".repeat(64)});
    let failure = |route: FacadeTool, input: &Value| {
        dispatcher
            .dispatch(route.command(input.as_object().unwrap().clone()).unwrap())
            .unwrap_err()
    };
    assert_eq!(
        failure(FacadeTool::InspectEffectiveProfile, &inspect)
            .diagnostic()
            .canonical_bytes(),
        failure(FacadeTool::ExplainEffectiveProfileEntry, &explain)
            .diagnostic()
            .canonical_bytes(),
    );
    for handle in [
        json!("private-invalid-handle"),
        json!("A".repeat(64)),
        json!(4),
    ] {
        let input = json!({"compilation": review_fixture::baseline(), "handle": handle});
        assert!(FacadeTool::ExplainEffectiveProfileEntry
            .command(input.as_object().unwrap().clone())
            .is_err());
    }
}
