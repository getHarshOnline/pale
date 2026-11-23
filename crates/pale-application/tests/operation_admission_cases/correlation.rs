//! Tool-correlation proofs for host identity, turn isolation, and confirmation leases.

use super::support::{
    apply_pre, apply_success, current_lease, instance_digest, tool, Fixture, TestAdapter,
    NATIVE_DIGEST_TOOL,
};
use pale_application::operation_admission::{
    ConfirmationRegistrationState, ConfirmationRevocation, ObservedToolInvocation,
    OperationAdmissionError, ToolInvocation,
};

#[test]
fn host_identity_correlates_parallel_calls_once_without_payload_confusion() {
    for adapter in [TestAdapter::ClaudeCode, TestAdapter::Codex] {
        assert_host_identity_correlation(adapter);
    }
}

fn assert_host_identity_correlation(adapter: TestAdapter) {
    let fixture = Fixture::direct(adapter);
    let mut state = fixture.activate("").unwrap().unwrap();
    let input = serde_json::json!({"path": "release.tar"});

    apply_pre(
        &fixture,
        &mut state,
        tool(
            &fixture,
            "tool-a",
            NATIVE_DIGEST_TOOL,
            Some(&input),
            instance_digest(),
            Some("turn-a"),
        ),
    );
    assert!(matches!(
        fixture.admit_tool(
            &current_lease(&state),
            tool(
                &fixture,
                "tool-a",
                NATIVE_DIGEST_TOOL,
                Some(&input),
                instance_digest(),
                Some("turn-a")
            )
        ),
        Err(OperationAdmissionError::ToolInvocationReplay)
    ));
    apply_pre(
        &fixture,
        &mut state,
        tool(
            &fixture,
            "tool-b",
            NATIVE_DIGEST_TOOL,
            Some(&input),
            instance_digest(),
            Some("turn-a"),
        ),
    );

    let other_provider = Fixture::direct(match adapter {
        TestAdapter::ClaudeCode => TestAdapter::Codex,
        TestAdapter::Codex => TestAdapter::ClaudeCode,
    });
    let substituted_completion = ToolInvocation::try_from_host(ObservedToolInvocation {
        provider_binding: &other_provider.provider_binding,
        adapter: fixture.adapter,
        tool_name: NATIVE_DIGEST_TOOL,
        tool_use_id: Some("tool-a"),
        tool_input: Some(&input),
        command: None,
        instance_digest: instance_digest(),
        session_id: Some("session-a"),
        turn_id: Some("turn-a"),
    })
    .unwrap();
    assert!(matches!(
        fixture.complete(&current_lease(&state), substituted_completion),
        Err(OperationAdmissionError::InvocationOriginMismatch)
    ));

    apply_success(
        &fixture,
        &mut state,
        tool(
            &fixture,
            "tool-a",
            NATIVE_DIGEST_TOOL,
            Some(&input),
            instance_digest(),
            Some("turn-a"),
        ),
    );
    assert!(matches!(
        fixture.complete(
            &current_lease(&state),
            tool(
                &fixture,
                "tool-a",
                NATIVE_DIGEST_TOOL,
                Some(&input),
                instance_digest(),
                Some("turn-a")
            )
        ),
        Err(OperationAdmissionError::ToolInvocationNotAdmitted)
    ));

    let changed = serde_json::json!({"path": "other.tar"});
    assert!(matches!(
        fixture.complete(
            &current_lease(&state),
            tool(
                &fixture,
                "tool-b",
                NATIVE_DIGEST_TOOL,
                Some(&changed),
                instance_digest(),
                Some("turn-a")
            )
        ),
        Err(OperationAdmissionError::ToolInvocationMismatch)
    ));
    apply_success(
        &fixture,
        &mut state,
        tool(
            &fixture,
            "tool-b",
            NATIVE_DIGEST_TOOL,
            Some(&input),
            instance_digest(),
            Some("turn-a"),
        ),
    );
}

#[test]
fn missing_host_identity_and_cross_turn_events_fail_before_admission() {
    let input = serde_json::json!({"path": "release.tar"});
    let fixture = Fixture::direct(TestAdapter::ClaudeCode);
    let observed = |tool_use_id, session_id, turn_id| ObservedToolInvocation {
        provider_binding: &fixture.provider_binding,
        adapter: fixture.adapter,
        tool_name: NATIVE_DIGEST_TOOL,
        tool_use_id,
        tool_input: Some(&input),
        command: None,
        instance_digest: instance_digest(),
        session_id,
        turn_id,
    };
    assert!(matches!(
        ToolInvocation::try_from_host(observed(None, Some("session-a"), Some("turn-a"))),
        Err(OperationAdmissionError::MissingToolUseIdentity)
    ));
    for invalid in ["", "tool\nidentity"] {
        assert!(matches!(
            ToolInvocation::try_from_host(observed(
                Some(invalid),
                Some("session-a"),
                Some("turn-a")
            )),
            Err(OperationAdmissionError::InvalidToolUseIdentity)
        ));
    }
    let oversized = "x".repeat(1_025);
    assert!(matches!(
        ToolInvocation::try_from_host(observed(
            Some(&oversized),
            Some("session-a"),
            Some("turn-a")
        )),
        Err(OperationAdmissionError::InvalidToolUseIdentity)
    ));
    assert!(matches!(
        ToolInvocation::try_from_host(observed(Some("tool-a"), None, None)),
        Err(OperationAdmissionError::MissingActivityIdentity)
    ));

    let state = fixture.activate("").unwrap().unwrap();
    assert!(matches!(
        fixture.admit_tool(
            &current_lease(&state),
            tool(
                &fixture,
                "tool-a",
                NATIVE_DIGEST_TOOL,
                Some(&input),
                instance_digest(),
                Some("turn-b")
            )
        ),
        Err(OperationAdmissionError::CrossTurnLease)
    ));
}

#[test]
fn native_confirmation_is_non_consumable_until_evidence_then_exactly_promoted() {
    let fixture = Fixture::direct(TestAdapter::ClaudeCode);
    let mut state = fixture.activate("").unwrap().unwrap();
    let predecessor = current_lease(&state);
    let input = serde_json::json!({"query": "release evidence"});
    let invocation = tool(
        &fixture,
        "web-search-1",
        "WebSearch",
        Some(&input),
        instance_digest(),
        Some("turn-a"),
    );
    let admission = fixture.admit_tool(&predecessor, invocation).unwrap();
    assert!(admission.confirmation().is_some());
    assert!(admission.apply_to(&mut state, &predecessor).unwrap());
    assert_eq!(
        admission.confirmation_state(&state, &predecessor),
        Some(ConfirmationRegistrationState::Provisional)
    );
    assert!(matches!(
        fixture.complete(
            &current_lease(&state),
            tool(
                &fixture,
                "web-search-1",
                "WebSearch",
                Some(&input),
                instance_digest(),
                Some("turn-a")
            )
        ),
        Err(OperationAdmissionError::ToolInvocationNotAdmitted)
    ));

    assert!(admission.confirm_after_evidence(&mut state).unwrap());
    assert_eq!(
        admission.confirmation_state(&state, &predecessor),
        Some(ConfirmationRegistrationState::Finalized)
    );
    apply_success(
        &fixture,
        &mut state,
        tool(
            &fixture,
            "web-search-1",
            "WebSearch",
            Some(&input),
            instance_digest(),
            Some("turn-a"),
        ),
    );
}

#[test]
fn confirmation_revocation_restores_only_its_exact_registration() {
    let fixture = Fixture::direct(TestAdapter::ClaudeCode);
    let mut state = fixture.activate("").unwrap().unwrap();
    let predecessor = current_lease(&state);
    let input = serde_json::json!({"query": "release evidence"});
    let admission = fixture
        .admit_tool(
            &predecessor,
            tool(
                &fixture,
                "web-search-1",
                "WebSearch",
                Some(&input),
                instance_digest(),
                Some("turn-a"),
            ),
        )
        .unwrap();
    assert!(admission.apply_to(&mut state, &predecessor).unwrap());
    assert_eq!(
        admission
            .revoke_confirmation(&mut state, &predecessor)
            .unwrap(),
        ConfirmationRevocation::RestoredProvisional
    );
    assert_eq!(current_lease(&state), predecessor);

    assert!(admission.apply_to(&mut state, &predecessor).unwrap());
    let pending = current_lease(&state);
    apply_pre(
        &fixture,
        &mut state,
        tool(
            &fixture,
            "digest-1",
            NATIVE_DIGEST_TOOL,
            Some(&serde_json::json!({"path": "release.tar"})),
            instance_digest(),
            Some("turn-a"),
        ),
    );
    let different = state.clone();
    assert_ne!(current_lease(&state), pending);
    assert_eq!(
        admission
            .revoke_confirmation(&mut state, &predecessor)
            .unwrap(),
        ConfirmationRevocation::Different
    );
    assert_eq!(state, different);
}
