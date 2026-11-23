//! Host-neutral hook-policy composition, protocol encoding, and state transitions.
//!
//! Authority-path journeys remain in the separate untrusted-event boundary test
//! target so policy tests do not impersonate filesystem admission evidence.

use pale_application::hooks::*;
use pale_application::session_state::{SessionRuntimeState, StateEffect};
use pale_application::terminal_adapters::{for_exact_binding, TerminalAdapterDescriptor};
use pale_core::engine_wire::DecisionDispositionV1;
use pale_core::records::{
    CapabilityIdV1, ChildPropagation, FailurePosture, LifecycleAction, LifecycleBehavior,
    LifecycleBehaviorIdV1, LifecycleEvent, LifecycleFallbackV1, ModeAxis, PackageIdV1,
    PackageResourceIdV1, PackageResourceRefV1, ParticipantRefV1, PredicateKind,
    ProviderBindingRefV1, ResponsibilityV1, RunPolicyEntry, RunPolicyIdV1, StateScope,
};

#[path = "support/provider_binding.rs"]
mod provider_binding_fixtures;

fn adapter(plugin_id: &str) -> (TerminalAdapterDescriptor, ProviderBindingRefV1) {
    let mut binding = provider_binding_fixtures::provider_binding(ParticipantRefV1::TerminalLeaf {
        terminal_leaf_id: "terminal-owner".into(),
    });
    binding.responsibility = ResponsibilityV1::TerminalHarnessExecution;
    binding.adapter_plugin.adapter_plugin_id = plugin_id.into();
    binding.binding_identity = binding.recompute_identity().unwrap();
    let reference = ProviderBindingRefV1 {
        provider_binding_id: binding.id.clone(),
        provider_binding_identity: binding.binding_identity.clone(),
    };
    (for_exact_binding(&reference, &binding).unwrap(), reference)
}

fn claude() -> TerminalAdapterDescriptor {
    adapter("pale.adapter.claude-code.v1").0
}

fn codex() -> TerminalAdapterDescriptor {
    adapter("pale.adapter.codex.v1").0
}

fn adapters() -> [TerminalAdapterDescriptor; 2] {
    [claude(), codex()]
}

fn resource(id: &str) -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse("test").expect("test Package identity is valid"),
        resource_id: PackageResourceIdV1::parse(id).expect("test resource identity is valid"),
    }
}

fn selected_mode(state: &SessionRuntimeState, axis: ModeAxis) -> Option<&PackageResourceRefV1> {
    state.active_modes().get(&axis)
}

fn behaviour(
    id: &str,
    event: LifecycleEvent,
    predicate_kind: PredicateKind,
    matchers: &[&str],
    action: LifecycleAction,
) -> LifecycleBehavior {
    LifecycleBehavior {
        id: LifecycleBehaviorIdV1::parse(id).expect("test behavior identity is valid"),
        description: format!("{id} description"),
        guidance: None,
        event,
        predicate_kind,
        matchers: matchers.iter().map(|s| s.to_string()).collect(),
        case_insensitive: false,
        existing_targets_only: false,
        allow_admitted_operation: false,
        action,
        activate_mode: None,
        modes: vec![],
        tool_reqs: vec![],
        failure_posture: FailurePosture::FailClosed,
        state_scope: StateScope::Stateless,
        governed_by: None,
        timeout_ms: None,
        fallback: None,
    }
}

fn empty_context(adapter: TerminalAdapterDescriptor) -> HookContext<'static> {
    HookContext {
        adapter,
        tool_name: None,
        tool_command: None,
        message: None,
        target_facts: &[],
        protected_write_admitted: false,
        mode_changed: false,
        rebaseline_pending: false,
        child_context_admitted: false,
        active_modes: &[],
        mode_axes: &[],
        mode_defaults: &[],
        available_tool_requirements: &[],
    }
}

fn render_behaviour_outcome(
    event: HookEvent,
    behaviours: &[LifecycleBehavior],
    injection: &str,
    context: &HookContext<'_>,
) -> HookOutcome {
    let plan = LifecyclePlan::evaluate(event, behaviours, context, None).unwrap();
    let guidance = std::collections::BTreeMap::new();
    let payloads = plan.bind_payloads(injection, "", &guidance).unwrap();
    payloads.render()
}

fn render_behaviour_outcome_with_state(
    event: HookEvent,
    behaviours: &[LifecycleBehavior],
    injection: &str,
    context: &HookContext<'_>,
    state: Option<&SessionRuntimeState>,
) -> HookOutcome {
    let plan = LifecyclePlan::evaluate(event, behaviours, context, state).unwrap();
    let guidance = std::collections::BTreeMap::new();
    let payloads = plan.bind_payloads(injection, "", &guidance).unwrap();
    payloads.render()
}

fn render_behaviour_outcome_with_mode_context(
    event: HookEvent,
    behaviours: &[LifecycleBehavior],
    injection: &str,
    mode_context: &str,
    context: &HookContext<'_>,
    state: Option<&SessionRuntimeState>,
) -> HookOutcome {
    let plan = LifecyclePlan::evaluate(event, behaviours, context, state).unwrap();
    let guidance = std::collections::BTreeMap::new();
    let payloads = plan
        .bind_payloads(injection, mode_context, &guidance)
        .unwrap();
    payloads.render()
}

fn behavior_requires_injection(
    event: HookEvent,
    behaviours: &[LifecycleBehavior],
    context: &HookContext<'_>,
) -> bool {
    LifecyclePlan::evaluate(event, behaviours, context, None)
        .unwrap()
        .requires_injection()
}

fn apply_state_actions(
    state: &mut SessionRuntimeState,
    event: HookEvent,
    behaviours: &[LifecycleBehavior],
    context: &HookContext<'_>,
) -> Result<Vec<StateEffect>, pale_application::session_state::SessionStateError> {
    let plan = LifecyclePlan::evaluate(event, behaviours, context, Some(&*state))
        .expect("test lifecycle bindings are coherent");
    plan.apply_state_actions(state)
}

fn render_policy_output(
    host: TerminalAdapterDescriptor,
    event: HookEvent,
    policy: &RunPolicyEntry,
    injection: &str,
    tool_name: Option<&str>,
    tool_command: Option<&str>,
) -> HookOutput {
    RunPolicyPlan::evaluate(
        event,
        policy,
        RunPolicyFacts {
            child_type: None,
            tool_name,
            tool_command,
            exact_authorization: None,
        },
    )
    .render(injection)
    .unwrap()
    .into_output(host, event)
}

#[test]
fn injection_is_loaded_only_when_an_event_or_behaviour_needs_it() {
    let plain = empty_context(claude());
    assert!(!behavior_requires_injection(
        HookEvent::SessionStart,
        &[],
        &plain,
    ));
    assert!(!behavior_requires_injection(
        HookEvent::SubagentStart,
        &[],
        &plain
    ));
    let admitted_child = HookContext {
        child_context_admitted: true,
        ..empty_context(claude())
    };
    assert!(!behavior_requires_injection(
        HookEvent::SubagentStart,
        &[],
        &admitted_child
    ));
    assert!(!behavior_requires_injection(
        HookEvent::PreToolUse,
        &[],
        &plain
    ));

    let inject_on_edit = [behaviour(
        "reorient",
        LifecycleEvent::PreToolUse,
        PredicateKind::ToolMatch,
        &["Edit"],
        LifecycleAction::Inject,
    )];
    let editing = HookContext {
        tool_name: Some("Edit"),
        ..empty_context(claude())
    };
    assert!(
        behavior_requires_injection(HookEvent::PreToolUse, &inject_on_edit, &editing),
        "an inject-behaviour that holds on PreToolUse must force the payload to load"
    );
    // Presence alone is not enough — the behaviour's predicate must actually hold for this event.
    let reading = HookContext {
        tool_name: Some("Read"),
        ..empty_context(claude())
    };
    assert!(!behavior_requires_injection(
        HookEvent::PreToolUse,
        &inject_on_edit,
        &reading
    ));

    let standard = policy("standard", true, ChildPropagation::Propagate, false, &[]);
    let start = RunPolicyPlan::evaluate(
        HookEvent::SessionStart,
        &standard,
        RunPolicyFacts {
            child_type: None,
            tool_name: None,
            tool_command: None,
            exact_authorization: None,
        },
    );
    assert!(start.requires_injection());
    assert!(
        matches!(start.render(""), Err(RunPolicyPayloadError)),
        "a required payload cannot degrade silently into empty context"
    );
}

#[test]
fn lifecycle_requirements_use_the_route_sealed_capability_set() {
    let mut gated = behaviour(
        "browser-check",
        LifecycleEvent::PostToolUse,
        PredicateKind::Always,
        &[],
        LifecycleAction::Guide,
    );
    gated.tool_reqs =
        vec![CapabilityIdV1::parse("browser-qa").expect("test capability identity is valid")];
    gated.failure_posture = FailurePosture::FailOpen;

    assert_eq!(
        render_behaviour_outcome(
            HookEvent::PostToolUse,
            std::slice::from_ref(&gated),
            "",
            &empty_context(claude()),
        ),
        HookOutcome::Context(String::new()),
        "an unavailable optional mechanic must not manufacture guidance"
    );

    let available =
        vec![CapabilityIdV1::parse("browser-qa").expect("test capability identity is valid")];
    let context = HookContext {
        available_tool_requirements: &available,
        ..empty_context(claude())
    };
    assert_eq!(
        render_behaviour_outcome(
            HookEvent::PostToolUse,
            std::slice::from_ref(&gated),
            "",
            &context,
        ),
        HookOutcome::Context(gated.description.clone())
    );

    gated.failure_posture = FailurePosture::FailClosed;
    assert!(matches!(
        render_behaviour_outcome(
            HookEvent::PostToolUse,
            std::slice::from_ref(&gated),
            "",
            &empty_context(claude()),
        ),
        HookOutcome::Deny(_)
    ));
}

#[test]
fn lifecycle_support_comes_from_the_selected_adapter_descriptor() {
    let mut unavailable = behaviour(
        "host-only-check",
        LifecycleEvent::UserPromptExpansion,
        PredicateKind::Always,
        &[],
        LifecycleAction::Guide,
    );
    let replacement = "Use the native tool result and the ordinary recovery doctrine.";
    unavailable.fallback = Some(LifecycleFallbackV1 {
        guidance: replacement.into(),
        carrier_event: LifecycleEvent::SessionStart,
    });
    assert_eq!(
        render_behaviour_outcome(
            HookEvent::SessionStart,
            std::slice::from_ref(&unavailable),
            "",
            &empty_context(codex()),
        ),
        HookOutcome::Context(replacement.into())
    );
    assert_eq!(
        render_behaviour_outcome(
            HookEvent::SessionStart,
            std::slice::from_ref(&unavailable),
            "",
            &empty_context(claude()),
        ),
        HookOutcome::Context(String::new()),
        "a supported native event must not activate its generic fallback"
    );
}

#[test]
fn lifecycle_requirements_exclude_only_semantically_inert_records() {
    let disabled = policy("disabled", false, ChildPropagation::None, false, &[]);
    let housekeeping = behaviour(
        "session-summary",
        LifecycleEvent::SessionEnd,
        PredicateKind::Always,
        &[],
        LifecycleAction::NoOp,
    );
    let inert = behaviour(
        "completion-notify",
        LifecycleEvent::Stop,
        PredicateKind::Never,
        &[],
        LifecycleAction::NoOp,
    );

    let mut fallback = behaviour(
        "host-fallback",
        LifecycleEvent::UserPromptExpansion,
        PredicateKind::Never,
        &[],
        LifecycleAction::NoOp,
    );
    fallback.fallback = Some(LifecycleFallbackV1 {
        guidance: "Use the session-start carrier.".into(),
        carrier_event: LifecycleEvent::SessionStart,
    });
    let cases = [
        (inert, Vec::new()),
        (housekeeping, vec![HookEvent::SessionEnd]),
        (fallback, vec![HookEvent::SessionStart]),
    ];

    for (behaviour, expected) in cases {
        let requirements =
            HookRequirements::resolve(codex(), &disabled, &[behaviour], Default::default());
        assert_eq!(requirements.events(), expected);
    }

    let active = policy(
        "active",
        true,
        ChildPropagation::Propagate,
        true,
        &["Bash(git push*)"],
    );
    let cases = [
        (
            claude(),
            vec![
                HookEvent::SessionStart,
                HookEvent::UserPromptSubmit,
                HookEvent::UserPromptExpansion,
                HookEvent::PermissionRequest,
                HookEvent::PreToolUse,
                HookEvent::PostToolUse,
                HookEvent::PostToolUseFailure,
                HookEvent::SubagentStart,
                HookEvent::Stop,
            ],
        ),
        (
            codex(),
            vec![
                HookEvent::SessionStart,
                HookEvent::UserPromptSubmit,
                HookEvent::PermissionRequest,
                HookEvent::PreToolUse,
                HookEvent::PostToolUse,
                HookEvent::SubagentStart,
                HookEvent::Stop,
            ],
        ),
    ];
    for (adapter, expected) in cases {
        let requirements = HookRequirements::resolve(
            adapter,
            &active,
            &[],
            pale_application::hooks::HookRequirementFacts {
                write_boundary: true,
                operations: true,
                modes: true,
                repository_verifiers: true,
            },
        );
        assert_eq!(requirements.events(), expected, "adapter `{adapter}`");
    }

    for adapter in [claude(), codex()] {
        let requirements = HookRequirements::resolve(
            adapter,
            &disabled,
            &[],
            pale_application::hooks::HookRequirementFacts {
                write_boundary: true,
                ..Default::default()
            },
        );
        assert_eq!(
            requirements.events(),
            [HookEvent::PermissionRequest, HookEvent::PreToolUse],
            "the write boundary must cover every pre-effect event for `{adapter}`"
        );
    }
}

#[test]
fn destructive_shell_uses_the_canonical_command_tool_class_for_each_harness() {
    let gate = behaviour(
        "destructive-shell",
        LifecycleEvent::PreToolUse,
        PredicateKind::DestructiveShell,
        &[],
        LifecycleAction::Deny,
    );
    for (native_harness, tool_name) in [
        (claude(), "Bash"),
        (codex(), "exec_command"),
        (codex(), "shell"),
    ] {
        let context = HookContext {
            adapter: native_harness,
            tool_name: Some(tool_name),
            tool_command: Some("rm -rf build"),
            ..empty_context(claude())
        };
        assert!(matches!(
            render_behaviour_outcome(
                HookEvent::PreToolUse,
                std::slice::from_ref(&gate),
                "",
                &context,
            ),
            HookOutcome::Deny(_)
        ));
    }
}

#[test]
fn shell_safety_predicates_split_commands_and_fail_closed_on_dynamic_execution() {
    for (predicate, commands) in [
        (
            PredicateKind::DestructiveShell,
            [
                "cat harmless\nrm -rf protected",
                "cat harmless & rm -rf protected",
                "echo $(rm -rf protected)",
            ],
        ),
        (
            PredicateKind::GitHookBypass,
            [
                "git status\ngit push --no-verify",
                "git status & git push --no-verify",
                "echo $(git push --no-verify)",
            ],
        ),
    ] {
        let gate = behaviour(
            "shell-safety",
            LifecycleEvent::PreToolUse,
            predicate,
            &[],
            LifecycleAction::Deny,
        );
        for command in commands {
            let context = HookContext {
                adapter: claude(),
                tool_name: Some("Bash"),
                tool_command: Some(command),
                ..empty_context(claude())
            };
            assert!(matches!(
                render_behaviour_outcome(
                    HookEvent::PreToolUse,
                    std::slice::from_ref(&gate),
                    "",
                    &context,
                ),
                HookOutcome::Deny(_)
            ));
        }
    }
}

#[test]
fn git_hook_bypass_uses_typed_shell_structure_not_substring_matching() {
    let gate = behaviour(
        "git-hook-bypass",
        LifecycleEvent::PreToolUse,
        PredicateKind::GitHookBypass,
        &[],
        LifecycleAction::Deny,
    );
    for (native_harness, tool_name) in [(claude(), "Bash"), (codex(), "exec_command")] {
        for (command, denied) in [
            ("git commit --no-verify -m safe", true),
            ("git -c core.hooksPath=/dev/null push", true),
            ("git config --local core.hooksPath /dev/null", true),
            (
                "GIT_CONFIG_KEY_0=core.hooksPath GIT_CONFIG_VALUE_0=/dev/null GIT_CONFIG_COUNT=1 git commit -m safe",
                true,
            ),
            ("git commit -m 'document --no-verify'", false),
            ("git config --get core.hooksPath", false),
            ("git config --unset core.hooksPath", false),
            ("echo --no-verify", false),
        ] {
            let context = HookContext {
                adapter: native_harness,
                tool_name: Some(tool_name),
                tool_command: Some(command),
                ..empty_context(claude())
            };
            let outcome = render_behaviour_outcome(
                HookEvent::PreToolUse,
                std::slice::from_ref(&gate),
                "",
                &context,
            );
            assert_eq!(
                matches!(outcome, HookOutcome::Deny(_)),
                denied,
                "{native_harness}: {command}"
            );
        }
    }
}

#[test]
fn exact_message_matches_whole_command_not_substring() {
    // A standalone deactivation command matches; the same words inside a larger
    // request must NOT (the substring bug this replaces would have deactivated on
    // "add a normal mode toggle"). Trailing punctuation/case are ignored.
    let deact = behaviour(
        "deactivate",
        LifecycleEvent::UserPromptSubmit,
        PredicateKind::ExactMessage,
        &["stop", "normal mode"],
        LifecycleAction::Guide,
    );
    let fires = |msg: &str| {
        let ctx = HookContext {
            message: Some(msg),
            ..empty_context(claude())
        };
        matches!(
            render_behaviour_outcome(
                HookEvent::UserPromptSubmit,
                std::slice::from_ref(&deact),
                "",
                &ctx,
            ),
            HookOutcome::Context(c) if !c.is_empty()
        )
    };
    assert!(fires("Normal mode."), "standalone command matches");
    assert!(fires("  stop  "), "trim + case normalized");
    assert!(
        !fires("add a normal mode toggle"),
        "the command as a substring must NOT match"
    );
}

#[test]
fn command_prefix_matches_leading_token_with_sigil_alias() {
    let cmd = behaviour(
        "mode-command",
        LifecycleEvent::UserPromptSubmit,
        PredicateKind::CommandPrefix,
        &["/review"],
        LifecycleAction::Guide,
    );
    let fires = |msg: &str| {
        let ctx = HookContext {
            message: Some(msg),
            ..empty_context(claude())
        };
        matches!(
            render_behaviour_outcome(
                HookEvent::UserPromptSubmit,
                std::slice::from_ref(&cmd),
                "",
                &ctx,
            ),
            HookOutcome::Context(c) if !c.is_empty()
        )
    };
    assert!(
        fires("/review the changes"),
        "leading command token matches"
    );
    assert!(fires("@review the changes"), "`@` sigil aliases to `/`");
    assert!(
        !fires("/reviewer"),
        "a longer command token must not collide"
    );
    assert!(
        !fires("let's use review here"),
        "a mention that is not a leading command must not match"
    );
}

#[test]
fn all_applicable_guidance_composes_not_first_wins() {
    // Two guidance behaviours both fire on the same event: BOTH must surface, in
    // catalog order — the previous engine collapsed to the first.
    let a = behaviour(
        "first",
        LifecycleEvent::UserPromptSubmit,
        PredicateKind::Always,
        &[],
        LifecycleAction::Guide,
    );
    let b = behaviour(
        "second",
        LifecycleEvent::UserPromptSubmit,
        PredicateKind::Always,
        &[],
        LifecycleAction::Warn,
    );
    match render_behaviour_outcome(
        HookEvent::UserPromptSubmit,
        &[a, b],
        "",
        &empty_context(claude()),
    ) {
        HookOutcome::Context(c) => assert!(
            c.contains("first description") && c.contains("second description"),
            "both guidance payloads compose: {c}"
        ),
        other => panic!("expected composed guidance context, got {other:?}"),
    }
}

#[test]
fn deny_beats_ask_and_guide_regardless_of_order() {
    // Strength order (not source order): a deny anywhere wins over an ask/guide, and an ask
    // wins over a guide — so a weaker action can never mask a stronger one on the same event.
    let guide = behaviour(
        "g",
        LifecycleEvent::PreToolUse,
        PredicateKind::Always,
        &[],
        LifecycleAction::Guide,
    );
    let ask = behaviour(
        "a",
        LifecycleEvent::PreToolUse,
        PredicateKind::Always,
        &[],
        LifecycleAction::Ask,
    );
    let deny = behaviour(
        "d",
        LifecycleEvent::PreToolUse,
        PredicateKind::Always,
        &[],
        LifecycleAction::Deny,
    );
    // Deny declared LAST still wins over an earlier ask + guide.
    assert!(matches!(
        render_behaviour_outcome(
            HookEvent::PreToolUse,
            &[guide.clone(), ask.clone(), deny.clone()],
            "",
            &empty_context(claude())
        ),
        HookOutcome::Deny(_)
    ));
    // Ask declared after a guide still wins over the guide (but below any deny).
    assert!(matches!(
        render_behaviour_outcome(
            HookEvent::PreToolUse,
            &[guide, ask],
            "",
            &empty_context(claude())
        ),
        HookOutcome::Ask(_)
    ));
}

#[test]
fn blocking_outcomes_stably_deduplicate_non_adjacent_repeats() {
    let render = |action| {
        let mut first = behaviour(
            "first",
            LifecycleEvent::PreToolUse,
            PredicateKind::Always,
            &[],
            action,
        );
        first.description = "shared".into();
        let mut second = behaviour(
            "second",
            LifecycleEvent::PreToolUse,
            PredicateKind::Always,
            &[],
            action,
        );
        second.description = "distinct".into();
        let mut repeated = behaviour(
            "repeated",
            LifecycleEvent::PreToolUse,
            PredicateKind::Always,
            &[],
            action,
        );
        repeated.description = "shared".into();
        render_behaviour_outcome(
            HookEvent::PreToolUse,
            &[first, second, repeated],
            "",
            &empty_context(claude()),
        )
    };

    assert_eq!(
        render(LifecycleAction::Deny),
        HookOutcome::Deny("shared\n\ndistinct".into())
    );
    assert_eq!(
        render(LifecycleAction::Ask),
        HookOutcome::Ask("shared\n\ndistinct".into())
    );
}

#[test]
fn typed_outcome_composition_preserves_permission_request_denial_shape() {
    let outcome = HookOutcome::Context("run-policy context".into())
        .merge(HookOutcome::Deny("lifecycle denial".into()));
    assert_eq!(
        outcome.disposition(claude(), HookEvent::PermissionRequest),
        DecisionDispositionV1::Deny
    );

    let output = outcome.into_output(claude(), HookEvent::PermissionRequest);
    let value: serde_json::Value = serde_json::from_str(&output.stdout).unwrap();
    assert_eq!(value["hookSpecificOutput"]["decision"]["behavior"], "deny");
    assert_eq!(
        value["hookSpecificOutput"]["hookEventName"],
        "PermissionRequest"
    );
}

#[test]
fn state_actions_activate_deactivate_and_capture() {
    // A prompt command activates its declared target mode; a standalone command clears it.
    let mut on = behaviour(
        "enter-review",
        LifecycleEvent::UserPromptSubmit,
        PredicateKind::CommandPrefix,
        &["/review"],
        LifecycleAction::Activate,
    );
    on.activate_mode = Some(resource("review"));
    let off = behaviour(
        "leave",
        LifecycleEvent::UserPromptSubmit,
        PredicateKind::ExactMessage,
        &["/mode reset"],
        LifecycleAction::Deactivate,
    );
    let cat = [on, off];

    let mut state = SessionRuntimeState::default();
    let mode_axes = vec![
        (resource("full"), ModeAxis::Intensity),
        (resource("dev"), ModeAxis::TaskStance),
        (resource("review"), ModeAxis::TaskStance),
    ];
    let defaults = vec![
        (ModeAxis::Intensity, resource("full")),
        (ModeAxis::TaskStance, resource("dev")),
    ];
    state.ensure_defaults(&defaults).unwrap();
    let initial_modes = state.active_modes().values().cloned().collect::<Vec<_>>();
    let enter = HookContext {
        message: Some("/review please"),
        active_modes: &initial_modes,
        mode_axes: &mode_axes,
        mode_defaults: &defaults,
        ..empty_context(claude())
    };
    let fx = apply_state_actions(&mut state, HookEvent::UserPromptSubmit, &cat, &enter).unwrap();
    assert_eq!(
        selected_mode(&state, ModeAxis::TaskStance),
        Some(&resource("review"))
    );
    assert_eq!(
        fx,
        vec![StateEffect::Activated {
            axis: ModeAxis::TaskStance,
            mode: resource("review")
        }]
    );

    let review_modes = state.active_modes().values().cloned().collect::<Vec<_>>();
    let leave = HookContext {
        message: Some("/mode reset"),
        active_modes: &review_modes,
        mode_axes: &mode_axes,
        mode_defaults: &defaults,
        ..empty_context(claude())
    };
    let fx2 = apply_state_actions(&mut state, HookEvent::UserPromptSubmit, &cat, &leave).unwrap();
    assert_eq!(
        selected_mode(&state, ModeAxis::TaskStance),
        Some(&resource("dev")),
        "the standalone command restored the default stance"
    );
    assert_eq!(
        selected_mode(&state, ModeAxis::Intensity),
        Some(&resource("full"))
    );
    assert_eq!(fx2, vec![StateEffect::DefaultsRestored]);
}

#[test]
fn lifecycle_plan_rejects_incoherent_mode_bindings_before_state_or_output() {
    let mut activate = behaviour(
        "enter-review",
        LifecycleEvent::UserPromptSubmit,
        PredicateKind::Always,
        &[],
        LifecycleAction::Activate,
    );
    activate.activate_mode = Some(resource("review"));
    let defaults = vec![(ModeAxis::TaskStance, resource("review"))];
    let duplicate_modes = vec![
        (resource("review"), ModeAxis::TaskStance),
        (resource("review"), ModeAxis::Intensity),
    ];
    let context = HookContext {
        mode_axes: &duplicate_modes,
        mode_defaults: &defaults,
        ..empty_context(claude())
    };

    assert!(matches!(
        LifecyclePlan::evaluate(
            HookEvent::UserPromptSubmit,
            std::slice::from_ref(&activate),
            &context,
            None,
        ),
        Err(LifecyclePlanError::DuplicateMode(mode)) if mode == resource("review")
    ));
}

#[test]
fn lifecycle_plan_rejects_session_modes_outside_the_sealed_selection() {
    let behavior = behaviour(
        "guidance",
        LifecycleEvent::UserPromptSubmit,
        PredicateKind::Always,
        &[],
        LifecycleAction::Guide,
    );
    let mode_axes = vec![
        (resource("full"), ModeAxis::Intensity),
        (resource("dev"), ModeAxis::TaskStance),
        (resource("review"), ModeAxis::TaskStance),
    ];
    let defaults = vec![
        (ModeAxis::Intensity, resource("full")),
        (ModeAxis::TaskStance, resource("dev")),
    ];

    let unknown = vec![resource("full"), resource("invented")];
    let context = HookContext {
        active_modes: &unknown,
        mode_axes: &mode_axes,
        mode_defaults: &defaults,
        ..empty_context(claude())
    };
    assert!(matches!(
        LifecyclePlan::evaluate(
            HookEvent::UserPromptSubmit,
            std::slice::from_ref(&behavior),
            &context,
            None,
        ),
        Err(LifecyclePlanError::UnknownActiveMode(mode)) if mode == resource("invented")
    ));

    let conflicting = vec![resource("full"), resource("dev"), resource("review")];
    let context = HookContext {
        active_modes: &conflicting,
        mode_axes: &mode_axes,
        mode_defaults: &defaults,
        ..empty_context(claude())
    };
    assert!(matches!(
        LifecyclePlan::evaluate(
            HookEvent::UserPromptSubmit,
            std::slice::from_ref(&behavior),
            &context,
            None,
        ),
        Err(LifecyclePlanError::MultipleActiveModes {
            axis: ModeAxis::TaskStance,
            ..
        })
    ));

    let missing_axis = vec![resource("full")];
    let context = HookContext {
        active_modes: &missing_axis,
        mode_axes: &mode_axes,
        mode_defaults: &defaults,
        ..empty_context(claude())
    };
    assert_eq!(
        LifecyclePlan::evaluate(
            HookEvent::UserPromptSubmit,
            std::slice::from_ref(&behavior),
            &context,
            None,
        )
        .unwrap_err(),
        LifecyclePlanError::MissingActiveMode(ModeAxis::TaskStance)
    );

    let active = vec![resource("full"), resource("dev")];
    let partial_defaults = vec![(ModeAxis::Intensity, resource("full"))];
    let context = HookContext {
        active_modes: &active,
        mode_axes: &mode_axes,
        mode_defaults: &partial_defaults,
        ..empty_context(claude())
    };
    assert_eq!(
        LifecyclePlan::evaluate(
            HookEvent::UserPromptSubmit,
            std::slice::from_ref(&behavior),
            &context,
            None,
        )
        .unwrap_err(),
        LifecyclePlanError::MissingDefault(ModeAxis::TaskStance)
    );

    let mut unknown_scope = behavior;
    unknown_scope.modes = vec![resource("invented")];
    let context = HookContext {
        active_modes: &active,
        mode_axes: &mode_axes,
        mode_defaults: &defaults,
        ..empty_context(claude())
    };
    assert!(matches!(
        LifecyclePlan::evaluate(
            HookEvent::UserPromptSubmit,
            std::slice::from_ref(&unknown_scope),
            &context,
            None,
        ),
        Err(LifecyclePlanError::BehaviorModeUnknown { behavior, mode })
            if behavior == LifecycleBehaviorIdV1::parse("guidance").unwrap()
                && mode == resource("invented")
    ));
}

#[test]
fn capture_state_action_records_an_entry() {
    let cap = behaviour(
        "audit",
        LifecycleEvent::PostToolUse,
        PredicateKind::Always,
        &[],
        LifecycleAction::Capture,
    );
    let mut state = SessionRuntimeState::default();
    let fx = apply_state_actions(
        &mut state,
        HookEvent::PostToolUse,
        std::slice::from_ref(&cap),
        &empty_context(claude()),
    )
    .unwrap();
    assert_eq!(
        fx,
        vec![StateEffect::Captured {
            entry: "audit description".into()
        }]
    );
    assert_eq!(
        serde_json::to_value(&state).unwrap()["captures"],
        serde_json::json!(["audit description"])
    );
}

#[test]
fn first_touch_guidance_uses_precommit_state_then_flips_after_commit() {
    let ft = behaviour(
        "first-edit-guard",
        LifecycleEvent::PreToolUse,
        PredicateKind::FirstTouch,
        &[],
        LifecycleAction::Guide,
    );
    let cat = std::slice::from_ref(&ft);
    let mut state = SessionRuntimeState::default();
    let targets = ["src/main.rs", "src/lib.rs"].map(|path| TargetPathFact {
        path: path.into(),
        presence: TargetPresence::Unknown,
    });
    let ctx = HookContext {
        target_facts: &targets,
        ..empty_context(claude())
    };
    assert!(matches!(
        render_behaviour_outcome_with_state(
            HookEvent::PreToolUse,
            cat,
            "",
            &ctx,
            Some(&state)
        ),
        HookOutcome::Context(context) if context.contains("first-edit-guard")
    ));
    let fx1 = apply_state_actions(&mut state, HookEvent::PreToolUse, cat, &ctx).unwrap();
    assert!(fx1
        .iter()
        .any(|effect| matches!(effect, StateEffect::FirstTouch { .. })));
    let persisted = serde_json::to_value(&state).unwrap();
    let touched = persisted["touched"].as_array().unwrap();
    assert!(
        touched.iter().any(|value| value == "src/main.rs"),
        "the first touch is recorded"
    );
    assert!(touched.iter().any(|value| value == "src/lib.rs"));
    assert!(matches!(
        render_behaviour_outcome_with_state(
            HookEvent::PreToolUse,
            cat,
            "",
            &ctx,
            Some(&state)
        ),
        HookOutcome::Context(context) if context.is_empty()
    ));
}

#[test]
fn protected_existing_path_denies_unless_the_exact_tool_call_was_admitted() {
    let mut protection = behaviour(
        "standards-config",
        LifecycleEvent::PreToolUse,
        PredicateKind::PathMatch,
        &["*.eslintrc.*"],
        LifecycleAction::Deny,
    );
    protection.case_insensitive = true;
    protection.existing_targets_only = true;
    protection.allow_admitted_operation = true;
    let path = "config/.ESLINTRC.JS";
    let present = [TargetPathFact {
        path: path.into(),
        presence: TargetPresence::Present,
    }];
    let blocked = HookContext {
        target_facts: &present,
        ..empty_context(claude())
    };
    assert!(matches!(
        render_behaviour_outcome(
            HookEvent::PreToolUse,
            std::slice::from_ref(&protection),
            "",
            &blocked,
        ),
        HookOutcome::Deny(_)
    ));

    let missing = [TargetPathFact {
        path: path.into(),
        presence: TargetPresence::Missing,
    }];
    let new_target = HookContext {
        target_facts: &missing,
        ..empty_context(claude())
    };
    assert!(matches!(
        render_behaviour_outcome(
            HookEvent::PreToolUse,
            std::slice::from_ref(&protection),
            "",
            &new_target,
        ),
        HookOutcome::Context(context) if context.is_empty()
    ));

    let paths = vec![path.to_string()];
    assert!(protected_write_requires_verification(
        std::slice::from_ref(&protection),
        HookEvent::PreToolUse,
        &paths,
        true,
    ));
    assert!(!protected_write_requires_verification(
        std::slice::from_ref(&protection),
        HookEvent::PreToolUse,
        &paths,
        false,
    ));
    assert!(protected_write_requires_verification(
        std::slice::from_ref(&protection),
        HookEvent::PostToolUse,
        &paths,
        false,
    ));

    let admitted = HookContext {
        target_facts: &present,
        protected_write_admitted: true,
        available_tool_requirements: &protection.tool_reqs,
        ..empty_context(claude())
    };
    assert!(matches!(
        render_behaviour_outcome(
            HookEvent::PreToolUse,
            std::slice::from_ref(&protection),
            "",
            &admitted,
        ),
        HookOutcome::Context(context) if context.is_empty()
    ));
}

#[test]
fn behaviour_engine_mode_gate_and_tool_deny_precedence() {
    let modes = [resource("intensity")];
    let mode_axes = [
        (resource("standard"), ModeAxis::Intensity),
        (resource("intensity"), ModeAxis::Intensity),
    ];
    let mode_defaults = [(ModeAxis::Intensity, resource("standard"))];
    let mut activate = behaviour(
        "guide",
        LifecycleEvent::UserPromptSubmit,
        PredicateKind::Always,
        &[],
        LifecycleAction::Guide,
    );
    activate.modes = vec![resource("intensity")];
    let cat = [activate];
    let ctx = HookContext {
        active_modes: &modes,
        mode_axes: &mode_axes,
        mode_defaults: &mode_defaults,
        ..empty_context(claude())
    };
    let out = render_behaviour_outcome(HookEvent::UserPromptSubmit, &cat, "DOCTRINE", &ctx)
        .into_output(claude(), HookEvent::UserPromptSubmit);
    assert!(
        out.stdout.contains("additionalContext"),
        "an in-mode behaviour surfaces guidance"
    );
    // Same behaviour, but the required mode is not active -> skipped (no output).
    let standard_mode = [resource("standard")];
    let ctx_no_mode = HookContext {
        active_modes: &standard_mode,
        mode_axes: &mode_axes,
        mode_defaults: &mode_defaults,
        ..empty_context(claude())
    };
    let out2 =
        render_behaviour_outcome(HookEvent::UserPromptSubmit, &cat, "DOCTRINE", &ctx_no_mode)
            .into_output(claude(), HookEvent::UserPromptSubmit);
    assert_eq!(
        out2.stdout, "",
        "a mode-scoped behaviour is inert outside its mode"
    );

    // ToolMatch deny on PreToolUse: a gated command denies (blocking) and wins.
    let deny = behaviour(
        "boundary",
        LifecycleEvent::PreToolUse,
        PredicateKind::ToolMatch,
        &["Bash(git push*)"],
        LifecycleAction::Deny,
    );
    let ctx_push = HookContext {
        tool_name: Some("Bash"),
        tool_command: Some("git push origin main"),
        ..empty_context(claude())
    };
    let out3 = render_behaviour_outcome(
        HookEvent::PreToolUse,
        std::slice::from_ref(&deny),
        "",
        &ctx_push,
    )
    .into_output(claude(), HookEvent::PreToolUse);
    assert!(out3.stdout.contains("\"permissionDecision\":\"deny\""));
    // A harmless command under the same behaviour is allowed.
    let ctx_ls = HookContext {
        tool_name: Some("Bash"),
        tool_command: Some("ls"),
        ..empty_context(claude())
    };
    assert_eq!(
        render_behaviour_outcome(HookEvent::PreToolUse, &[deny], "", &ctx_ls)
            .into_output(claude(), HookEvent::PreToolUse)
            .stdout,
        ""
    );
}

#[test]
fn structured_write_predicates_scope_guidance_to_the_proven_effect() {
    let quality = behaviour(
        "quality",
        LifecycleEvent::PostToolUse,
        PredicateKind::WriteEffect,
        &[],
        LifecycleAction::Guide,
    );
    let design = behaviour(
        "design",
        LifecycleEvent::PostToolUse,
        PredicateKind::PathMatch,
        &["*.html", "*.tsx"],
        LifecycleAction::Guide,
    );
    let behaviours = [quality, design];

    let no_write = empty_context(claude());
    assert!(matches!(
        render_behaviour_outcome(HookEvent::PostToolUse, &behaviours, "", &no_write),
        HookOutcome::Context(context) if context.is_empty()
    ));

    let rust_target = [TargetPathFact {
        path: "src/lib.rs".into(),
        presence: TargetPresence::Unknown,
    }];
    let rust_write = HookContext {
        target_facts: &rust_target,
        ..empty_context(claude())
    };
    assert!(matches!(
        render_behaviour_outcome(HookEvent::PostToolUse, &behaviours, "", &rust_write),
        HookOutcome::Context(context)
            if context.contains("quality description") && !context.contains("design description")
    ));

    let ui_target = [TargetPathFact {
        path: "src/view.tsx".into(),
        presence: TargetPresence::Unknown,
    }];
    let ui_write = HookContext {
        target_facts: &ui_target,
        ..empty_context(claude())
    };
    assert!(matches!(
        render_behaviour_outcome(HookEvent::PostToolUse, &behaviours, "", &ui_write),
        HookOutcome::Context(context)
            if context.contains("quality description") && context.contains("design description")
    ));
}

#[test]
fn unevaluable_predicate_follows_failure_posture() {
    // A first-touch gate with no session state cannot be evaluated: fail-closed ->
    // the deny still fires (safe), fail-open -> it is skipped.
    let mut closed = behaviour(
        "first-touch",
        LifecycleEvent::PreToolUse,
        PredicateKind::FirstTouch,
        &[],
        LifecycleAction::Deny,
    );
    closed.failure_posture = FailurePosture::FailClosed;
    let out = render_behaviour_outcome(
        HookEvent::PreToolUse,
        std::slice::from_ref(&closed),
        "",
        &empty_context(claude()),
    )
    .into_output(claude(), HookEvent::PreToolUse);
    assert!(
        out.stdout.contains("\"permissionDecision\":\"deny\""),
        "fail-closed gate denies when it cannot be evaluated"
    );
    let mut open = closed;
    open.failure_posture = FailurePosture::FailOpen;
    assert_eq!(
        render_behaviour_outcome(HookEvent::PreToolUse, &[open], "", &empty_context(claude()),)
            .into_output(claude(), HookEvent::PreToolUse)
            .stdout,
        "",
        "fail-open behaviour is skipped when it cannot be evaluated"
    );

    let mut missing_argument = behaviour(
        "command-gate",
        LifecycleEvent::PreToolUse,
        PredicateKind::ToolMatch,
        &["Bash(git push*)"],
        LifecycleAction::Deny,
    );
    let context = HookContext {
        tool_name: Some("Bash"),
        tool_command: None,
        ..empty_context(claude())
    };
    assert!(matches!(
        render_behaviour_outcome(
            HookEvent::PreToolUse,
            std::slice::from_ref(&missing_argument),
            "",
            &context
        ),
        HookOutcome::Deny(_)
    ));
    missing_argument.action = LifecycleAction::Guide;
    assert!(matches!(
        render_behaviour_outcome(
            HookEvent::PreToolUse,
            std::slice::from_ref(&missing_argument),
            "",
            &context
        ),
        HookOutcome::Deny(_)
    ));
    missing_argument.failure_posture = FailurePosture::FailOpen;
    assert!(matches!(
        render_behaviour_outcome(
            HookEvent::PreToolUse,
            &[missing_argument],
            "",
            &context
        ),
        HookOutcome::Context(context) if context.is_empty()
    ));
}

#[test]
fn direct_prompt_expansion_gate_uses_prompt_block_protocol() {
    let gate = behaviour(
        "protected-command",
        LifecycleEvent::UserPromptExpansion,
        PredicateKind::CommandPrefix,
        &["/deploy"],
        LifecycleAction::Deny,
    );
    let context = HookContext {
        message: Some("/deploy"),
        ..empty_context(claude())
    };
    let output = render_behaviour_outcome(HookEvent::UserPromptExpansion, &[gate], "", &context)
        .into_output(claude(), HookEvent::UserPromptExpansion);
    let wire: serde_json::Value = serde_json::from_str(&output.stdout).unwrap();
    assert_eq!(wire["decision"], "block");
    assert!(wire.get("reason").is_some());
    assert!(wire.get("hookSpecificOutput").is_none());
}

fn policy(
    id: &str,
    inject: bool,
    child_propagation: ChildPropagation,
    gate: bool,
    matchers: &[&str],
) -> RunPolicyEntry {
    RunPolicyEntry {
        id: RunPolicyIdV1::parse(id).expect("test run-policy identity is valid"),
        description: "d".into(),
        session_inject: inject,
        child_propagation,
        child_type_selectors: vec![],
        action_gate: gate,
        gated_tool_matchers: matchers
            .iter()
            .map(|matcher| matcher.parse().expect("valid test tool matcher"))
            .collect(),
    }
}

#[test]
fn session_start_injects_raw_for_claude_and_json_envelope_for_codex() {
    let p = policy("standard", true, ChildPropagation::Propagate, false, &[]);
    // Native Claude consumes raw stdout: the injection is emitted verbatim.
    let claude = render_policy_output(
        claude(),
        HookEvent::SessionStart,
        &p,
        "DOCTRINE",
        None,
        None,
    );
    assert_eq!(claude.stdout, "DOCTRINE");
    assert_eq!(claude.exit_code, 0);
    // Codex requires a JSON envelope; the injection must round-trip through the
    // `additionalContext` field. Parsed structurally, so key order / whitespace
    // can change without breaking the test — only a real wiring regression does.
    let codex = render_policy_output(codex(), HookEvent::SessionStart, &p, "DOCTRINE", None, None);
    let v: serde_json::Value =
        serde_json::from_str(&codex.stdout).expect("Codex SessionStart output must be valid JSON");
    // The injection must sit under `hookSpecificOutput.additionalContext` with
    // the matching event name — the host drops it at any other location. A
    // substring check would miss this required nesting; asserting the actual
    // structure catches a wiring regression that a string match cannot.
    assert_eq!(
        v["hookSpecificOutput"]["additionalContext"],
        serde_json::json!("DOCTRINE")
    );
    assert_eq!(
        v["hookSpecificOutput"]["hookEventName"],
        serde_json::json!("SessionStart")
    );
    assert!(
        v.get("systemMessage").is_none(),
        "ordinary context must not consume Codex's user-visible warning channel"
    );
}

#[test]
fn failure_and_context_outputs_follow_each_native_event_contract() {
    let claude_start = failure_event(claude(), HookEvent::SessionStart, "bad seal");
    assert!(claude_start.stdout.contains("could not verify"));
    assert!(serde_json::from_str::<serde_json::Value>(&claude_start.stdout).is_err());

    let codex_start = failure_event(codex(), HookEvent::SessionStart, "bad seal");
    let codex_start: serde_json::Value = serde_json::from_str(&codex_start.stdout).unwrap();
    assert!(codex_start["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .is_some_and(|message| message.contains("could not verify")));

    for host in adapters().into_iter() {
        let tool = failure_event(host, HookEvent::PreToolUse, "bad seal");
        let tool: serde_json::Value = serde_json::from_str(&tool.stdout).unwrap();
        assert_eq!(tool["hookSpecificOutput"]["permissionDecision"], "deny");
    }

    let claude_compact = failure_event(claude(), HookEvent::PreCompact, "bad seal");
    let claude_compact: serde_json::Value = serde_json::from_str(&claude_compact.stdout).unwrap();
    assert_eq!(claude_compact["decision"], "block");

    let codex_compact = failure_event(codex(), HookEvent::PreCompact, "bad seal");
    let codex_compact: serde_json::Value = serde_json::from_str(&codex_compact.stdout).unwrap();
    assert_eq!(codex_compact["continue"], false);
    assert!(codex_compact["stopReason"].is_string());

    for host in adapters().into_iter() {
        let post_compact =
            HookOutcome::Context("must not leak".into()).into_output(host, HookEvent::PostCompact);
        assert!(post_compact.stdout.is_empty());
    }
}

#[test]
fn disabled_policy_injects_nothing() {
    let p = policy("disabled", false, ChildPropagation::None, false, &[]);
    let out = render_policy_output(
        claude(),
        HookEvent::SessionStart,
        &p,
        "DOCTRINE",
        None,
        None,
    );
    assert_eq!(out.stdout, "");
}

#[test]
fn subagent_propagation_axis_is_data_driven() {
    let injection = "DOCTRINE";
    let prop = policy("standard", true, ChildPropagation::Propagate, false, &[]);
    let out = render_policy_output(
        claude(),
        HookEvent::SubagentStart,
        &prop,
        injection,
        None,
        None,
    );
    assert!(out.stdout.contains("SubagentStart"));
    assert!(out.stdout.contains("additionalContext")); // JSON envelope, not raw

    let exempt = policy("gated-parent", true, ChildPropagation::Exempt, false, &[]);
    let out = render_policy_output(
        claude(),
        HookEvent::SubagentStart,
        &exempt,
        injection,
        None,
        None,
    );
    assert_eq!(out.stdout, "", "exempt children are not re-injected");

    let mut selective = prop;
    selective.child_type_selectors = vec!["*explore*".into(), "general".into()];
    let child_outcome = |child_type| {
        RunPolicyPlan::evaluate(
            HookEvent::SubagentStart,
            &selective,
            RunPolicyFacts {
                child_type,
                tool_name: None,
                tool_command: None,
                exact_authorization: None,
            },
        )
        .render(injection)
        .unwrap()
    };
    assert_eq!(
        child_outcome(Some("Repository Explorer")),
        HookOutcome::Context(injection.into())
    );
    assert_eq!(
        child_outcome(Some("reviewer")),
        HookOutcome::Context(String::new())
    );
    assert_eq!(
        child_outcome(None),
        HookOutcome::Context(
            "Pale withheld parent project context because this child start omitted the agent identity required by the sealed selector policy. Work only within the explicit parent task and request scoped context before consequential action."
                .into()
        ),
        "a selector cannot broaden inheritance when child identity is missing"
    );

    let child_modes = behaviour(
        "child-mode-context",
        HookEvent::SubagentStart,
        PredicateKind::Always,
        &[],
        LifecycleAction::InjectModes,
    );
    let scoped_child = HookContext {
        child_context_admitted: false,
        ..empty_context(claude())
    };
    assert_eq!(
        render_behaviour_outcome_with_mode_context(
            HookEvent::SubagentStart,
            &[child_modes],
            "PARENT",
            "MODES",
            &scoped_child,
            None,
        ),
        HookOutcome::Context(String::new()),
        "lifecycle mode guidance cannot bypass child-context admission"
    );
}

#[test]
fn pre_tool_use_gates_on_command_argument_not_just_tool_name() {
    let strict = policy(
        "strict",
        true,
        ChildPropagation::Propagate,
        true,
        &["Bash(git push*)", "Bash(rm -rf*)"],
    );
    // Bash running `git push` -> denied.
    let denied = render_policy_output(
        claude(),
        HookEvent::PreToolUse,
        &strict,
        "",
        Some("Bash"),
        Some("git push origin main"),
    );
    assert!(denied.stdout.contains("\"permissionDecision\":\"deny\""));
    assert!(denied.stdout.contains("strict"));
    // Harmless Bash (`ls`) -> ALLOWED (the whole point: not denied identically).
    let harmless = render_policy_output(
        claude(),
        HookEvent::PreToolUse,
        &strict,
        "",
        Some("Bash"),
        Some("ls -la"),
    );
    assert_eq!(harmless.stdout, "", "harmless Bash is allowed");
    // A different tool -> allowed.
    let other = render_policy_output(
        claude(),
        HookEvent::PreToolUse,
        &strict,
        "",
        Some("Read"),
        None,
    );
    assert_eq!(other.stdout, "");
    // Fail closed: gated Bash but no command available -> deny.
    let noarg = render_policy_output(
        claude(),
        HookEvent::PreToolUse,
        &strict,
        "",
        Some("Bash"),
        None,
    );
    assert!(
        noarg.stdout.contains("\"permissionDecision\":\"deny\""),
        "gated tool without command denies"
    );
    // Fail closed: no tool name at all -> deny.
    let missing = render_policy_output(claude(), HookEvent::PreToolUse, &strict, "", None, None);
    assert!(missing.stdout.contains("\"permissionDecision\":\"deny\""));
    // No gate -> never denies.
    let standard = policy(
        "standard",
        true,
        ChildPropagation::Propagate,
        false,
        &["Bash(git push*)"],
    );
    assert_eq!(
        render_policy_output(
            claude(),
            HookEvent::PreToolUse,
            &standard,
            "",
            Some("Bash"),
            Some("git push")
        )
        .stdout,
        ""
    );
}

#[test]
fn event_wire_and_terminal_adapter_resolution_are_closed() {
    assert_eq!(
        serde_json::from_str::<HookEvent>(r#""session-start""#).unwrap(),
        HookEvent::SessionStart
    );
    assert!(serde_json::from_str::<HookEvent>(r#""bogus""#).is_err());
    assert_eq!(host_event_name(HookEvent::SessionStart), "SessionStart");
    for event in HookEvent::vocabulary() {
        let wire = serde_json::to_string(&event).unwrap();
        assert_eq!(serde_json::from_str::<HookEvent>(&wire).unwrap(), event);
        assert!(!host_event_name(event).is_empty());
    }
    let active = policy("active", true, ChildPropagation::Propagate, false, &[]);
    let expansion = behaviour(
        "expansion",
        LifecycleEvent::UserPromptExpansion,
        PredicateKind::Always,
        &[],
        LifecycleAction::Guide,
    );
    let codex_events = HookRequirements::resolve(
        codex(),
        &active,
        std::slice::from_ref(&expansion),
        Default::default(),
    );
    assert!(!codex_events
        .events()
        .contains(&HookEvent::UserPromptExpansion));
    let claude_events = HookRequirements::resolve(
        claude(),
        &active,
        std::slice::from_ref(&expansion),
        Default::default(),
    );
    assert!(claude_events
        .events()
        .contains(&HookEvent::UserPromptExpansion));

    let mut unknown = provider_binding_fixtures::provider_binding(ParticipantRefV1::TerminalLeaf {
        terminal_leaf_id: "terminal-owner".into(),
    });
    unknown.responsibility = ResponsibilityV1::TerminalHarnessExecution;
    unknown.adapter_plugin.adapter_plugin_id = "pale.adapter.unknown.v1".into();
    unknown.binding_identity = unknown.recompute_identity().unwrap();
    let unknown_ref = ProviderBindingRefV1 {
        provider_binding_id: unknown.id.clone(),
        provider_binding_identity: unknown.binding_identity.clone(),
    };
    assert!(for_exact_binding(&unknown_ref, &unknown).is_err());
    // Unmatched events remain non-blocking; lifecycle data, not an enum arm,
    // supplies any behavior for them.
    let p = policy("standard", true, ChildPropagation::Propagate, false, &[]);
    let out = render_policy_output(claude(), HookEvent::PostToolUse, &p, "DOCTRINE", None, None);
    assert_eq!(out.stdout, "");
    assert_eq!(out.exit_code, 0);
}
