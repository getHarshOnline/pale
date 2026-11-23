//! Repository-hook carrier boundary tests.

use super::*;

use pale_core::records::{
    ArtifactDigest, ChildPropagation, LifecycleEvent, LogicalRepositoryIdV1,
    ProviderBindingIdentityV1, ProviderBindingRefV1, RunPolicyEntry, RunPolicyIdV1, WorkspaceId,
};

#[test]
fn hook_artifacts_are_pale_owned_and_scoped_to_the_terminal_harness() {
    for (harness, harness_name) in [
        (TerminalAdapterDescriptor::claude_code(), "claude-code"),
        (TerminalAdapterDescriptor::codex(), "codex"),
    ] {
        let intent = activation_intent_path(harness);
        let contribution = repository_hook_contribution_path(harness);

        assert_eq!(
            intent,
            format!(".pale/hooks/{harness_name}/activation-intent.json")
        );
        assert_eq!(
            contribution,
            format!(".pale/hooks/{harness_name}/repository-hook-contribution.json")
        );
        for target in [&intent, &contribution] {
            assert!(pale_core::paths::SurfaceTarget::parse(target).is_some());
            assert!(!target.contains("provider-bindings"));
            assert!(!target.contains(".claude"));
            assert!(!target.contains(".codex"));
            assert!(!target.contains(".mcp.json"));
            assert!(!target.ends_with("settings.json"));
            assert!(!target.ends_with("config.toml"));
        }
    }
}

#[test]
fn unverified_machine_admission_renders_only_inert_activation_intent() {
    let policy = RunPolicyEntry {
        id: RunPolicyIdV1::parse("standard").unwrap(),
        description: "test activation policy".into(),
        session_inject: true,
        child_propagation: ChildPropagation::None,
        child_type_selectors: Vec::new(),
        action_gate: false,
        gated_tool_matchers: Vec::new(),
    };

    for (label, admission) in [
        ("portable", crate::render::MachineAdmission::portable()),
        (
            "reconstructed",
            crate::render::MachineAdmission::reconstructed(Vec::new(), BTreeMap::new()),
        ),
    ] {
        let note = activation_block_reason(admission.is_verified(), true, true, true, true)
            .expect("unverified admission must block runnable activation");
        for harness in [
            TerminalAdapterDescriptor::claude_code(),
            TerminalAdapterDescriptor::codex(),
        ] {
            let requirements = HookRequirements::resolve(
                harness,
                &policy,
                &[],
                crate::hooks::HookRequirementFacts::default(),
            );
            let surface =
                activation_intent_surface(harness, &requirements, Vec::new(), Vec::new(), note)
                    .unwrap();
            let content: serde_json::Value = serde_json::from_str(surface.content()).unwrap();

            assert_eq!(surface.kind, "hook-activation-intent", "{label}: {harness}");
            assert!(surface.rel_path.ends_with("/activation-intent.json"));
            assert_eq!(content["runnable"], false, "{label}: {harness}");
            assert!(
                surface
                    .target_note
                    .as_deref()
                    .is_some_and(|value| value.contains("portable or removal reconstruction")),
                "{label}: {harness}"
            );
        }
    }
}

#[test]
fn active_hook_rendering_uses_only_the_stable_resident_route() {
    let policy = RunPolicyEntry {
        id: RunPolicyIdV1::parse("standard").unwrap(),
        description: "test activation policy".into(),
        session_inject: true,
        child_propagation: ChildPropagation::None,
        child_type_selectors: Vec::new(),
        action_gate: false,
        gated_tool_matchers: Vec::new(),
    };
    let workspace_id = WorkspaceId::parse("workspace-a").unwrap();
    let repository_id = LogicalRepositoryIdV1::parse("repository-a").unwrap();
    let provider_binding = ProviderBindingRefV1 {
        provider_binding_id: "provider-binding-terminal".to_owned(),
        provider_binding_identity: ProviderBindingIdentityV1 {
            provider_binding_sha256: ArtifactDigest::hash_bytes(b"provider-binding-terminal"),
        },
    };
    let expected_arguments = [
        "hook".to_owned(),
        "session-start".to_owned(),
        "--workspace".to_owned(),
        workspace_id.to_string(),
        "--repository".to_owned(),
        repository_id.to_string(),
        "--provider-binding".to_owned(),
        provider_binding.provider_binding_id.clone(),
        "--provider-binding-identity".to_owned(),
        provider_binding
            .provider_binding_identity
            .provider_binding_sha256
            .to_string(),
    ];
    let lifecycle = [LifecycleEvent::SessionStart];

    for harness in [
        TerminalAdapterDescriptor::claude_code(),
        TerminalAdapterDescriptor::codex(),
    ] {
        let requirements = HookRequirements::resolve(harness, &policy, &[], Default::default());
        let wiring = ActiveHookWiring {
            exec: "/opt/pale bin/pale",
            workspace_id: &workspace_id,
            repo_id: &repository_id,
            harness,
            provider_binding: &provider_binding,
            lifecycle: &lifecycle,
            requirements: &requirements,
            behaviours: &[],
        };
        let rendered = if harness.is_claude_code() {
            claude_hook_contribution(&wiring).unwrap()
        } else {
            codex_hook_contribution(&wiring).unwrap()
        };
        let rendered: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        let handler = &rendered["hooks"]["SessionStart"][0]["hooks"][0];
        assert_eq!(handler["type"], "command");

        if harness.is_claude_code() {
            assert_eq!(handler["command"], "/opt/pale bin/pale");
            assert_eq!(handler["args"], serde_json::json!(expected_arguments));
        } else {
            let expected_command = std::iter::once("/opt/pale bin/pale".to_owned())
                .chain(expected_arguments.clone())
                .map(|word| crate::host_command::quote_posix_word(&word))
                .collect::<Vec<_>>()
                .join(" ");
            assert_eq!(handler["command"], expected_command);
            assert!(handler.get("args").is_none());
        }
    }
}

#[test]
fn pre_tool_use_goldens_are_exact_application_renderer_output() {
    let policy = RunPolicyEntry {
        id: RunPolicyIdV1::parse("strict").unwrap(),
        description: "test action-gating policy".into(),
        session_inject: false,
        child_propagation: ChildPropagation::None,
        child_type_selectors: Vec::new(),
        action_gate: true,
        gated_tool_matchers: vec!["Read".parse().unwrap()],
    };
    let workspace_id = WorkspaceId::parse("workspace-a").unwrap();
    let repository_id = LogicalRepositoryIdV1::parse("repository-a").unwrap();
    let provider_binding = ProviderBindingRefV1 {
        provider_binding_id: "provider-binding-terminal".to_owned(),
        provider_binding_identity: ProviderBindingIdentityV1 {
            provider_binding_sha256: ArtifactDigest::hash_bytes(b"provider-binding-terminal"),
        },
    };
    let lifecycle = [LifecycleEvent::PreToolUse];

    for (harness, expected) in [
        (
            TerminalAdapterDescriptor::claude_code(),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/hooks/claude-pre-tool-use-contribution.json"
            )),
        ),
        (
            TerminalAdapterDescriptor::codex(),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/hooks/codex-pre-tool-use-contribution.json"
            )),
        ),
    ] {
        let requirements = HookRequirements::resolve(harness, &policy, &[], Default::default());
        let wiring = ActiveHookWiring {
            exec: "pale",
            workspace_id: &workspace_id,
            repo_id: &repository_id,
            harness,
            provider_binding: &provider_binding,
            lifecycle: &lifecycle,
            requirements: &requirements,
            behaviours: &[],
        };
        let rendered = if harness.is_claude_code() {
            claude_hook_contribution(&wiring).unwrap()
        } else {
            codex_hook_contribution(&wiring).unwrap()
        };

        assert_eq!(
            rendered, expected,
            "{harness} outward process golden drifted"
        );
    }
}
