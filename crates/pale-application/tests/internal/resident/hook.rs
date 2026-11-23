//! Resident hook-ingress admission and confinement laws.

use pale_core::records::{
    ArtifactDigest, EffectClass, ExecutionPathSelectionIdV1, HookContextV1, HostToolBinding,
    HostToolEffectBinding, LifecycleEvent, LogicalRepositoryIdV1, ProviderBindingIdentityV1,
    ProviderBindingRefV1, ToolObservability, WorkspaceId,
};

use super::{
    AdmittedResidentHookWritePolicy, HookContextSurfaceCandidate,
    ResidentHookIngressAdmissionError, ResidentHookIngressRequest,
    ResidentHookPolicyAdmissionError, ResidentHookWritePolicyEvaluation,
    RESIDENT_HOOK_EVENT_BYTE_LIMIT,
};
use crate::hooks::RepositoryHookRoutingV1;
use crate::terminal_adapters::TerminalAdapterDescriptor;
use pale_adapter_fs::LocalMachineFilesystem;

fn routing(event: LifecycleEvent) -> RepositoryHookRoutingV1 {
    RepositoryHookRoutingV1::new(
        event,
        WorkspaceId::parse("ws.main").unwrap(),
        LogicalRepositoryIdV1::parse("repo.main").unwrap(),
        ProviderBindingRefV1 {
            provider_binding_id: "terminal-provider".into(),
            provider_binding_identity: ProviderBindingIdentityV1 {
                provider_binding_sha256: ArtifactDigest::hash_bytes(b"terminal-provider"),
            },
        },
    )
    .unwrap()
}

#[test]
fn admits_matching_bounded_event_without_promoting_native_observations() {
    let request = ResidentHookIngressRequest::admit(
        routing(LifecycleEvent::PreToolUse),
        br#"{"hook_event_name":"PreToolUse","session_id":"native-session","tool_name":"Read"}"#,
    )
    .unwrap();

    assert_eq!(request.routing().event(), LifecycleEvent::PreToolUse);
    assert_eq!(request.event().session_id(), Some("native-session"));
    assert_eq!(request.event().tool_name(), Some("Read"));
}

#[test]
fn rejects_absent_malformed_oversized_and_mismatched_events() {
    assert!(matches!(
        ResidentHookIngressRequest::admit(routing(LifecycleEvent::PreToolUse), b""),
        Err(ResidentHookIngressAdmissionError::Absent)
    ));
    assert!(matches!(
        ResidentHookIngressRequest::admit(routing(LifecycleEvent::PreToolUse), b"{"),
        Err(ResidentHookIngressAdmissionError::Malformed(_))
    ));
    assert!(matches!(
        ResidentHookIngressRequest::admit(
            routing(LifecycleEvent::PreToolUse),
            &vec![b' '; RESIDENT_HOOK_EVENT_BYTE_LIMIT + 1],
        ),
        Err(ResidentHookIngressAdmissionError::Malformed(
            "oversized event payload"
        ))
    ));
    assert!(matches!(
        ResidentHookIngressRequest::admit(
            routing(LifecycleEvent::PreToolUse),
            br#"{"hook_event_name":"PostToolUse"}"#,
        ),
        Err(ResidentHookIngressAdmissionError::EventMismatch {
            expected: "PreToolUse"
        })
    ));
    assert!(matches!(
        ResidentHookIngressRequest::admit(
            routing(LifecycleEvent::PreToolUse),
            br#"{"hook_event_name":"PreToolUse","tool_name":"MultiEdit","tool_input":{"edits":{}}}"#,
        ),
        Err(ResidentHookIngressAdmissionError::Malformed(
            "tool_input.edits is not an array"
        ))
    ));
}

#[test]
fn hook_context_selection_rejects_missing_repeated_and_substituted_surfaces() {
    let selected = ExecutionPathSelectionIdV1::parse("path-main").unwrap();
    let other = ExecutionPathSelectionIdV1::parse("path-other").unwrap();
    let candidate = |rel_path, consumers| HookContextSurfaceCandidate {
        rel_path,
        kind: "hook-context",
        consumers,
        content: "{}",
    };

    assert_eq!(
        super::select_hook_context_surface(
            [candidate("ignored", std::slice::from_ref(&other))].into_iter(),
            &selected,
        ),
        Err(ResidentHookPolicyAdmissionError::MissingHookContext)
    );
    assert_eq!(
        super::select_hook_context_surface(
            [
                candidate("first", std::slice::from_ref(&selected)),
                candidate("second", std::slice::from_ref(&selected)),
            ]
            .into_iter(),
            &selected,
        ),
        Err(ResidentHookPolicyAdmissionError::RepeatedHookContext)
    );
    assert_eq!(
        super::select_hook_context_surface(
            [candidate(
                ".pale/execution-paths/path-other/hook-context.json",
                std::slice::from_ref(&selected),
            )]
            .into_iter(),
            &selected,
        ),
        Err(ResidentHookPolicyAdmissionError::HookContextSurfaceSubstitution)
    );
}

fn compiled_hook_context() -> HookContextV1 {
    static CONTEXT: std::sync::OnceLock<HookContextV1> = std::sync::OnceLock::new();
    CONTEXT
        .get_or_init(|| {
            let (compilation, _, _) =
                crate::local_operation_start_fixture::host_source_generation_inputs("Write");
            let surface = compilation.repositories()[0]
                .render()
                .surfaces
                .iter()
                .find(|surface| surface.kind == "hook-context")
                .expect("fixture renders one hook context");
            serde_json::from_str(surface.content()).expect("compiled context is strict JSON")
        })
        .clone()
}

fn evaluate(
    event_json: &[u8],
    mut context: HookContextV1,
    session_admits_write: bool,
) -> ResidentHookWritePolicyEvaluation {
    let request =
        ResidentHookIngressRequest::admit(routing(LifecycleEvent::PreToolUse), event_json).unwrap();
    context.repository_verifiers.clear();
    let policy = AdmittedResidentHookWritePolicy {
        event: request.event(),
        hook_context: &context,
        adapter: TerminalAdapterDescriptor::claude_code(),
        admits_write: session_admits_write,
    };
    policy.evaluate_write_boundary(&LocalMachineFilesystem)
}

#[test]
fn compiled_write_policy_denies_protected_and_read_only_targets() {
    let temp = tempfile::tempdir().unwrap();
    let protected = temp.path().join("protected");
    let readonly = temp.path().join("readonly");
    std::fs::create_dir_all(&protected).unwrap();
    std::fs::create_dir_all(&readonly).unwrap();

    let mut context = compiled_hook_context();
    context.protected_runtime_dirs = vec![protected.to_string_lossy().into_owned()];
    context.read_only_dirs = vec![readonly.to_string_lossy().into_owned()];
    let protected_target = protected.join("state.json");
    let readonly_target = readonly.join("source.md");

    for target in [protected_target, readonly_target] {
        let event = serde_json::json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Write",
            "tool_input": { "file_path": target },
        });
        assert!(matches!(
            evaluate(&serde_json::to_vec(&event).unwrap(), context.clone(), true),
            ResidentHookWritePolicyEvaluation::Deny { .. }
        ));
    }
}

#[test]
fn unknown_write_binding_is_evaluated_and_session_authority_never_widens() {
    let temp = tempfile::tempdir().unwrap();
    let protected = temp.path().join("protected");
    let outside = temp.path().join("outside");
    std::fs::create_dir_all(&protected).unwrap();
    std::fs::create_dir_all(&outside).unwrap();

    let mut context = compiled_hook_context();
    context.protected_runtime_dirs = vec![protected.to_string_lossy().into_owned()];
    context.read_only_dirs.clear();
    context.host_tool_effects = vec![HostToolEffectBinding {
        effect_class: EffectClass::Write,
        tool: HostToolBinding {
            name: "CustomWrite".into(),
            model_name: None,
            observability: ToolObservability::PreToolObservable,
            commands: vec![],
            command_source: None,
            input_equals: Default::default(),
        },
    }];

    let protected_event = serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "CustomWrite",
        "tool_input": { "file_path": protected.join("state.json") },
    });
    assert!(matches!(
        evaluate(
            &serde_json::to_vec(&protected_event).unwrap(),
            context.clone(),
            true,
        ),
        ResidentHookWritePolicyEvaluation::Deny { .. }
    ));

    let outside_event = serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "CustomWrite",
        "tool_input": { "file_path": outside.join("source.md") },
    });
    assert_eq!(
        evaluate(
            &serde_json::to_vec(&outside_event).unwrap(),
            context.clone(),
            true,
        ),
        ResidentHookWritePolicyEvaluation::NoViolation
    );
    assert!(matches!(
        evaluate(&serde_json::to_vec(&outside_event).unwrap(), context, false,),
        ResidentHookWritePolicyEvaluation::Deny { .. }
    ));

    let mut context_without_protected_roots = compiled_hook_context();
    context_without_protected_roots.read_only_dirs.clear();
    context_without_protected_roots
        .protected_runtime_dirs
        .clear();
    context_without_protected_roots.host_tool_effects = vec![HostToolEffectBinding {
        effect_class: EffectClass::Write,
        tool: HostToolBinding {
            name: "CustomWrite".into(),
            model_name: None,
            observability: ToolObservability::PreToolObservable,
            commands: vec![],
            command_source: None,
            input_equals: Default::default(),
        },
    }];
    assert!(matches!(
        evaluate(
            &serde_json::to_vec(&outside_event).unwrap(),
            context_without_protected_roots,
            false,
        ),
        ResidentHookWritePolicyEvaluation::Deny { .. }
    ));
}

#[test]
fn unbound_unknown_and_unverifiable_command_fail_closed_at_protected_boundaries() {
    let temp = tempfile::tempdir().unwrap();
    let protected = temp.path().join("protected");
    std::fs::create_dir_all(&protected).unwrap();
    let mut context = compiled_hook_context();
    context.protected_runtime_dirs = vec![protected.to_string_lossy().into_owned()];
    context.read_only_dirs.clear();
    context.host_tool_effects.clear();

    for event in [
        serde_json::json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "UnknownWrite",
            "tool_input": { "file_path": protected.join("state.json") },
        }),
        serde_json::json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {},
        }),
    ] {
        assert!(matches!(
            evaluate(&serde_json::to_vec(&event).unwrap(), context.clone(), true),
            ResidentHookWritePolicyEvaluation::Deny { .. }
        ));
    }
}

#[test]
fn missing_or_unbound_tool_authority_fails_closed_without_protected_roots() {
    let mut context = compiled_hook_context();
    context.protected_runtime_dirs.clear();
    context.read_only_dirs.clear();
    context.host_tool_effects.clear();

    for event in [
        serde_json::json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "UnknownWrite",
            "tool_input": { "file_path": "outside.txt" },
        }),
        serde_json::json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": { "command": "touch outside.txt" },
        }),
        serde_json::json!({
            "hook_event_name": "PreToolUse",
        }),
    ] {
        assert!(matches!(
            evaluate(&serde_json::to_vec(&event).unwrap(), context.clone(), true),
            ResidentHookWritePolicyEvaluation::Deny { .. }
        ));
    }
}
