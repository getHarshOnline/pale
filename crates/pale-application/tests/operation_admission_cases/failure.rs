//! Failure-transition proofs for matched events, retry debt, and effect-free steering.

use pale_application::operation_admission::OperationAdmissionError;
use pale_core::records::FailurePosture;

use super::support::{
    apply_failure, apply_pre, apply_success, current_lease, instance_digest, tool, Fixture,
    TestAdapter, NATIVE_DIGEST_TOOL,
};

#[test]
fn fail_closed_transition_requires_a_matching_pre_event_and_halts_the_lease() {
    let fixture = Fixture::direct(TestAdapter::ClaudeCode);
    let mut state = fixture.activate("").unwrap().unwrap();
    let input = serde_json::json!({"path": "release.tar"});
    assert!(matches!(
        fixture.record_failure(
            &current_lease(&state),
            tool(
                &fixture,
                "digest-a",
                NATIVE_DIGEST_TOOL,
                Some(&input),
                instance_digest(),
                Some("turn-a")
            )
        ),
        Err(OperationAdmissionError::ToolInvocationNotAdmitted)
    ));
    apply_pre(
        &fixture,
        &mut state,
        tool(
            &fixture,
            "digest-a",
            NATIVE_DIGEST_TOOL,
            Some(&input),
            instance_digest(),
            Some("turn-a"),
        ),
    );
    let failed = apply_failure(
        &fixture,
        &mut state,
        tool(
            &fixture,
            "digest-a",
            NATIVE_DIGEST_TOOL,
            Some(&input),
            instance_digest(),
            Some("turn-a"),
        ),
    );
    assert_eq!(
        failed.posture(),
        Some((
            &super::support::requirement("digest-file"),
            FailurePosture::FailClosed
        ))
    );
    assert!(matches!(
        fixture.admit_tool(
            &current_lease(&state),
            tool(
                &fixture,
                "skill-a",
                "Skill",
                Some(&serde_json::json!({"skill": "pale-doctrine-release"})),
                instance_digest(),
                Some("turn-a")
            )
        ),
        Err(OperationAdmissionError::OperationHalted { .. })
    ));
}

#[test]
fn retry_debt_clears_only_on_the_matching_admitted_success_and_then_escalates() {
    let fixture = Fixture::direct(TestAdapter::ClaudeCode)
        .with_failure_posture("digest-file", FailurePosture::Retry);
    let mut state = fixture.activate("").unwrap().unwrap();
    let input = serde_json::json!({"path": "release.tar"});

    apply_pre(
        &fixture,
        &mut state,
        tool(
            &fixture,
            "digest-a",
            NATIVE_DIGEST_TOOL,
            Some(&input),
            instance_digest(),
            Some("turn-a"),
        ),
    );
    let failed = apply_failure(
        &fixture,
        &mut state,
        tool(
            &fixture,
            "digest-a",
            NATIVE_DIGEST_TOOL,
            Some(&input),
            instance_digest(),
            Some("turn-a"),
        ),
    );
    assert_eq!(
        failed.posture(),
        Some((
            &super::support::requirement("digest-file"),
            FailurePosture::Retry
        ))
    );
    assert!(matches!(
        fixture.admit_tool(
            &current_lease(&state),
            tool(
                &fixture,
                "search-a",
                "WebSearch",
                Some(&serde_json::json!({"query": "diagnose"})),
                instance_digest(),
                Some("turn-a")
            )
        ),
        Err(OperationAdmissionError::OperationRetrying { .. })
    ));

    apply_pre(
        &fixture,
        &mut state,
        tool(
            &fixture,
            "digest-b",
            NATIVE_DIGEST_TOOL,
            Some(&input),
            instance_digest(),
            Some("turn-a"),
        ),
    );
    apply_success(
        &fixture,
        &mut state,
        tool(
            &fixture,
            "digest-b",
            NATIVE_DIGEST_TOOL,
            Some(&input),
            instance_digest(),
            Some("turn-a"),
        ),
    );
    assert!(!matches!(
        fixture.admit_tool(
            &current_lease(&state),
            tool(
                &fixture,
                "search-b",
                "WebSearch",
                Some(&serde_json::json!({"query": "continue"})),
                instance_digest(),
                Some("turn-a")
            )
        ),
        Err(OperationAdmissionError::OperationRetrying { .. })
    ));

    let mut exhausted = fixture.activate("").unwrap().unwrap();
    for tool_id in ["digest-c", "digest-d"] {
        apply_pre(
            &fixture,
            &mut exhausted,
            tool(
                &fixture,
                tool_id,
                NATIVE_DIGEST_TOOL,
                Some(&input),
                instance_digest(),
                Some("turn-a"),
            ),
        );
        apply_failure(
            &fixture,
            &mut exhausted,
            tool(
                &fixture,
                tool_id,
                NATIVE_DIGEST_TOOL,
                Some(&input),
                instance_digest(),
                Some("turn-a"),
            ),
        );
    }
    assert!(matches!(
        fixture.admit_tool(
            &current_lease(&exhausted),
            tool(
                &fixture,
                "digest-e",
                NATIVE_DIGEST_TOOL,
                Some(&input),
                instance_digest(),
                Some("turn-a")
            )
        ),
        Err(OperationAdmissionError::OperationEscalated { .. })
    ));
}

#[test]
fn steer_allows_diagnostics_but_rejects_new_effects() {
    let fixture = Fixture::direct(TestAdapter::ClaudeCode)
        .with_failure_posture("digest-file", FailurePosture::Steer);
    let mut state = fixture.activate("").unwrap().unwrap();
    let input = serde_json::json!({"path": "release.tar"});
    apply_pre(
        &fixture,
        &mut state,
        tool(
            &fixture,
            "digest-a",
            NATIVE_DIGEST_TOOL,
            Some(&input),
            instance_digest(),
            Some("turn-a"),
        ),
    );
    apply_failure(
        &fixture,
        &mut state,
        tool(
            &fixture,
            "digest-a",
            NATIVE_DIGEST_TOOL,
            Some(&input),
            instance_digest(),
            Some("turn-a"),
        ),
    );
    assert!(fixture
        .admit_tool(
            &current_lease(&state),
            tool(
                &fixture,
                "digest-b",
                NATIVE_DIGEST_TOOL,
                Some(&input),
                instance_digest(),
                Some("turn-a")
            )
        )
        .is_ok());
    assert!(matches!(
        fixture.admit_tool(
            &current_lease(&state),
            tool(&fixture,
                "search-a",
                "WebSearch",
                Some(&serde_json::json!({"query": "diagnose"})),
                instance_digest(),
                Some("turn-a")
            )
        ),
        Err(OperationAdmissionError::OperationSteering { effect, .. })
            if effect == pale_core::records::EffectClass::Network
    ));
}
