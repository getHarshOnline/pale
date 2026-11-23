use super::*;

#[test]
fn outcome_unknown_is_non_terminal_and_does_not_invent_replay_permission() {
    assert_eq!(
        stopped_completion(DiagnosticEffectCertainty::OutcomeUnknown),
        ResultCompletion::NonTerminal
    );

    let diagnostic = serde_json::json!({
        "effect": "outcome_unknown",
        "retry": "reconcile_first",
    });
    let result = serde_json::to_value(publication_result(
        OneShotPlanKind::Adopt,
        &[],
        Some("beta"),
        Some(&diagnostic),
    ))
    .unwrap();
    assert_eq!(result["plan_kind"], "adopt");
    assert_eq!(result["publications"], serde_json::json!([]));
    assert_eq!(result["stopped_repository_id"], "beta");
    assert_eq!(result["diagnostic"], diagnostic);
    assert!(result.get("safe_to_retry").is_none());
}

#[test]
fn conclusive_stop_is_partial_even_before_the_first_write() {
    for effect in [
        DiagnosticEffectCertainty::NotApplicable,
        DiagnosticEffectCertainty::NotStarted,
        DiagnosticEffectCertainty::Confirmed,
    ] {
        assert_eq!(stopped_completion(effect), ResultCompletion::Partial);
    }
}
