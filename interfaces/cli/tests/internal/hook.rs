//! Thin CLI hook-ingress tests.

use clap::Parser as _;

fn request(payload: &[u8]) -> crate::command::ResidentHookRequest {
    crate::command::ResidentHookRequest::new(
        "session-start".to_string(),
        "workspace-a".to_string(),
        "repository-a".to_string(),
        "provider-binding-terminal".to_string(),
        "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        payload.to_vec(),
    )
}

#[test]
fn exact_static_coordinate_reaches_the_api_owned_carrier_gate() {
    let error = crate::command::CommandDispatcher::new()
        .unwrap()
        .dispatch(crate::command::CommandRequest::ResidentHook(request(
            br#"{"hook_event_name":"SessionStart","session_id":"native-session"}"#,
        )))
        .expect_err("this build has no authenticated resident carrier");

    assert_eq!(
        error.diagnostic().code(),
        "resident_hook_transport_unavailable"
    );
    assert_eq!(
        error.diagnostic().effect(),
        pale_sdk::DiagnosticEffectCertainty::NotStarted
    );
}

#[test]
fn malformed_native_input_keeps_the_api_diagnostic() {
    let error = crate::command::CommandDispatcher::new()
        .unwrap()
        .dispatch(crate::command::CommandRequest::ResidentHook(request(b"{")))
        .expect_err("malformed native input must fail API admission");

    assert_eq!(error.diagnostic().code(), "resident_hook_payload_malformed");
    assert_eq!(
        error.diagnostic().effect(),
        pale_sdk::DiagnosticEffectCertainty::NotStarted
    );
}

#[test]
fn cli_accepts_only_the_static_repository_hook_coordinate() {
    let parsed = crate::cli::Cli::try_parse_from([
        "pale",
        "hook",
        "session-start",
        "--workspace",
        "workspace-a",
        "--repository",
        "repository-a",
        "--provider-binding",
        "provider-binding-terminal",
        "--provider-binding-identity",
        "0000000000000000000000000000000000000000000000000000000000000000",
    ])
    .expect("parse canonical repository-hook command");
    let crate::cli::Cmd::Hook {
        resident_hook_config,
        event,
        workspace,
        repository,
        provider_binding,
        provider_binding_identity,
    } = parsed.cmd
    else {
        panic!("canonical hook arguments selected another command");
    };
    assert!(resident_hook_config.is_none());
    assert_eq!(
        (
            event,
            workspace,
            repository,
            provider_binding,
            provider_binding_identity,
        ),
        (
            "session-start".to_owned(),
            "workspace-a".to_owned(),
            "repository-a".to_owned(),
            "provider-binding-terminal".to_owned(),
            "0000000000000000000000000000000000000000000000000000000000000000".to_owned(),
        )
    );

    assert!(crate::cli::Cli::try_parse_from([
        "pale",
        "hook",
        "session-start",
        "--route",
        "retired-route",
        "--root",
        "/tmp/repository",
    ])
    .is_err());
}
