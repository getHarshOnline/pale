//! Resident-hook API admission and unavailable-carrier diagnostics.

use pale_sdk::{
    DiagnosticDisposition, DiagnosticEffectCertainty, DiagnosticRetry, DiagnosticStage, PaleApi,
    ResidentHookRequest,
};

fn request(event: &str, payload: &[u8]) -> ResidentHookRequest {
    ResidentHookRequest::new(
        event.to_string(),
        "workspace-a".to_string(),
        "repository-a".to_string(),
        "provider-binding-terminal".to_string(),
        "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        payload.to_vec(),
    )
}

#[test]
fn admitted_native_event_still_requires_an_authenticated_carrier() {
    let api = PaleApi::new().unwrap();
    let admitted = api
        .admit_resident_hook(request(
            "session-start",
            br#"{"hook_event_name":"SessionStart","session_id":"native-session"}"#,
        ))
        .unwrap();
    let error = api.resident_hook_transport_unavailable(&admitted);
    let diagnostic = error.diagnostic();

    assert_eq!(diagnostic.code(), "resident_hook_transport_unavailable");
    assert_eq!(diagnostic.stage(), DiagnosticStage::ResidentAdmission);
    assert_eq!(diagnostic.disposition(), DiagnosticDisposition::Blocked);
    assert_eq!(diagnostic.retry(), DiagnosticRetry::AfterStateChange);
    assert_eq!(diagnostic.effect(), DiagnosticEffectCertainty::NotStarted);
    assert!(diagnostic
        .help()
        .iter()
        .any(|help| help.contains("authenticated resident carrier")));
}

#[test]
fn routing_and_payload_families_have_exhaustive_stable_diagnostics() {
    let api = PaleApi::new().unwrap();
    let cases = [
        (
            request(
                "not-a-lifecycle-event",
                br#"{"hook_event_name":"SessionStart"}"#,
            ),
            "resident_hook_event_unknown",
        ),
        (
            request("session-start", b"{}"),
            "resident_hook_event_mismatch",
        ),
        (
            request("session-start", b"{"),
            "resident_hook_payload_malformed",
        ),
        (
            request("session-start", b""),
            "resident_hook_payload_absent",
        ),
    ];

    for (request, expected_code) in cases {
        let error = api
            .admit_resident_hook(request)
            .expect_err("fixture must fail resident-hook admission");
        let diagnostic = error.diagnostic();
        assert_eq!(diagnostic.code(), expected_code);
        assert_eq!(diagnostic.stage(), DiagnosticStage::ResidentAdmission);
        assert_eq!(
            diagnostic.disposition(),
            DiagnosticDisposition::InvalidInput
        );
        assert_eq!(diagnostic.retry(), DiagnosticRetry::AfterCorrection);
        assert_eq!(diagnostic.effect(), DiagnosticEffectCertainty::NotStarted);
        assert!(std::error::Error::source(&error).is_none());
    }
}

fn command_items() -> (ResidentHookRequest, serde_json::Value, serde_json::Value) {
    use serde_json::json;
    let original = request(
        "pre-tool-use",
        &serde_json::to_vec(&json!({
            "hook_event_name":"PreToolUse", "session_id":"private-session-marker",
            "turn_id":"private-turn-marker", "tool_use_id":"private-tool-marker",
            "tool_name":"Bash", "tool_input":{"command":"cargo check"},
            "cwd":"/private/project-marker"
        }))
        .unwrap(),
    );
    let item = json!({"type":"commandExecution", "id":"private-tool-marker",
        "command":"/bin/zsh -lc 'cargo check'", "cwd":"/private/project-marker",
        "source":"unifiedExecStartup", "status":"inProgress", "commandActions":[],
        "aggregatedOutput":null, "exitCode":null, "durationMs":null});
    let start = json!({"method":"item/started", "params":{
        "threadId":"private-session-marker", "turnId":"private-turn-marker",
        "startedAtMs":123, "item":item
    }});
    let mut end = start.clone();
    end["method"] = json!("item/completed");
    end["params"].as_object_mut().unwrap().remove("startedAtMs");
    end["params"]["completedAtMs"] = json!(124);
    end["params"]["item"]["status"] = json!("completed");
    end["params"]["item"]["exitCode"] = json!(0);
    end["params"]["item"]["durationMs"] = json!(1);
    end["params"]["item"]["aggregatedOutput"] = json!("Private-output-marker: tests failed");
    (original, start, end)
}

#[test]
fn native_command_join_preserves_typed_exit_and_separate_command_identities() {
    use pale_sdk::CodexCommandTerminalStatus;
    let api = PaleApi::new().unwrap();
    let (original, start, mut end) = command_items();
    let admitted = api.admit_resident_hook(original).unwrap();
    for exit in [0, 101] {
        end["params"]["item"]["exitCode"] = exit.into();
        let result = api
            .observe_codex_command_completion(
                &admitted,
                &serde_json::to_vec(&start).unwrap(),
                &serde_json::to_vec(&end).unwrap(),
            )
            .unwrap();
        assert_eq!(result.exit_code(), Some(exit));
        assert_eq!(result.status(), CodexCommandTerminalStatus::Completed);
        assert_eq!(result.duration_millis(), Some(1));
        assert_ne!(result.hook_command_sha256(), result.native_command_sha256());
        assert!(result.output_sha256().is_some());
        for rendered in [
            serde_json::to_string(&result).unwrap(),
            format!("{result:?}"),
        ] {
            for marker in [
                "private-session-marker",
                "private-turn-marker",
                "private-tool-marker",
                "/private/project-marker",
                "cargo check",
                "Private-output-marker",
            ] {
                assert!(!rendered.contains(marker), "{marker}");
            }
        }
        assert_ne!(result.native_session_sha256(), result.native_turn_sha256());
        assert_ne!(result.native_turn_sha256(), result.native_tool_use_sha256());
    }
}

#[test]
fn native_command_absent_exit_stays_absent_for_every_terminal_state() {
    use pale_sdk::CodexCommandTerminalStatus;
    let api = PaleApi::new().unwrap();
    let (original, start, mut end) = command_items();
    let admitted = api.admit_resident_hook(original).unwrap();
    for (native, expected) in [
        ("completed", CodexCommandTerminalStatus::Completed),
        ("failed", CodexCommandTerminalStatus::Failed),
        ("declined", CodexCommandTerminalStatus::Declined),
    ] {
        end["params"]["item"]["status"] = native.into();
        end["params"]["item"]["exitCode"] = serde_json::Value::Null;
        let result = api
            .observe_codex_command_completion(
                &admitted,
                &serde_json::to_vec(&start).unwrap(),
                &serde_json::to_vec(&end).unwrap(),
            )
            .unwrap();
        assert_eq!(result.exit_code(), None);
        assert_eq!(result.status(), expected);
    }
}

#[test]
fn native_command_join_refuses_mismatch_malformed_phase_and_duplicate_keys() {
    use serde_json::json;
    let api = PaleApi::new().unwrap();
    let (original, start, end) = command_items();
    let admitted = api.admit_resident_hook(original).unwrap();
    let changes = [
        ("/params/threadId", json!("other")),
        ("/params/turnId", json!("other")),
        ("/params/item/id", json!("other")),
        ("/params/item/cwd", json!("/other")),
        ("/params/item/command", json!("different command")),
        ("/params/item/source", json!("userShell")),
        ("/params/item/type", json!("agentMessage")),
        ("/params/item/status", json!("inProgress")),
        ("/params/item/exitCode", json!("0")),
        ("/params/item/exitCode", json!(2147483648_i64)),
        ("/params/item/aggregatedOutput", json!({"exit_code":0})),
        ("/params/item/commandActions", json!(null)),
        ("/params/completedAtMs", json!(null)),
        ("/method", json!("item/started")),
    ];
    for (pointer, value) in changes {
        let mut wrong = end.clone();
        *wrong.pointer_mut(pointer).unwrap() = value;
        let error = api
            .observe_codex_command_completion(
                &admitted,
                &serde_json::to_vec(&start).unwrap(),
                &serde_json::to_vec(&wrong).unwrap(),
            )
            .unwrap_err();
        assert_eq!(
            error.diagnostic().code(),
            "codex_command_observation_rejected"
        );
        assert_eq!(
            error.diagnostic().effect(),
            DiagnosticEffectCertainty::NotApplicable
        );
        assert_eq!(
            error.diagnostic().disposition(),
            DiagnosticDisposition::InvalidInput
        );
        assert!(!format!("{error:?}").contains("private-session-marker"));
    }
    let encoded = serde_json::to_string(&end).unwrap();
    let duplicate = encoded.replacen("\"exitCode\":0", "\"exitCode\":0,\"exitCode\":101", 1);
    assert_ne!(encoded, duplicate);
    let huge = vec![b' '; ResidentHookRequest::MAX_BYTES + 1];
    for wrong in [b"{".to_vec(), duplicate.into_bytes(), huge] {
        assert!(api
            .observe_codex_command_completion(
                &admitted,
                &serde_json::to_vec(&start).unwrap(),
                &wrong
            )
            .is_err());
    }
    let mut already_done = start.clone();
    already_done["params"]["item"]["exitCode"] = json!(0);
    assert!(api
        .observe_codex_command_completion(
            &admitted,
            &serde_json::to_vec(&already_done).unwrap(),
            &serde_json::to_vec(&end).unwrap()
        )
        .is_err());
}
