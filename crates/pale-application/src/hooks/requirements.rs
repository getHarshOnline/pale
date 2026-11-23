//! Execution-path lifecycle requirements shared by validation and rendering.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use pale_core::records::{
    ChildPropagation, LifecycleAction, LifecycleBehavior, PredicateKind, RunPolicyEntry,
};

use super::{harness_supports_event, HookEvent};

/// Semantic features that can require native lifecycle wiring on one path.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HookRequirementFacts {
    /// A pre-effect hook must protect read-only or runtime-owned paths.
    pub write_boundary: bool,
    /// At least one selected operation needs prompt, permission, or stop events.
    pub operations: bool,
    /// Selected run modes need prompt activation events.
    pub modes: bool,
    /// Successful writes can create repository-verification debt.
    pub repository_verifiers: bool,
}

/// Exact hooks and command-tool names required for one neutral execution path.
#[must_use = "resolved hook requirements must constrain validation and rendering"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookRequirements {
    events: Vec<HookEvent>,
    gated_tools: Vec<String>,
}

impl HookRequirements {
    /// Supported lifecycle events in the core-owned closed protocol order.
    pub fn events(&self) -> &[HookEvent] {
        &self.events
    }

    /// Exact command-tool names selected by the run-policy matchers.
    pub fn gated_tools(&self) -> &[String] {
        &self.gated_tools
    }

    /// Resolve policy, semantic, boundary, and behaviour requirements once.
    ///
    /// Unsupported native events are omitted here. Delegation semantics do not
    /// change the terminal native harness's event vocabulary.
    pub fn resolve(
        adapter: TerminalAdapterDescriptor,
        policy: &RunPolicyEntry,
        behaviours: &[LifecycleBehavior],
        facts: HookRequirementFacts,
    ) -> Self {
        let mut gated_tools = policy
            .gated_tool_matchers
            .iter()
            .map(|matcher| matcher.tool().to_string())
            .collect::<Vec<_>>();
        gated_tools.sort();
        gated_tools.dedup();

        let events = HookEvent::vocabulary()
            .filter(|&event| harness_supports_event(adapter, event))
            .filter(|&event| {
                let policy_requires = match event {
                    HookEvent::SessionStart => policy.session_inject,
                    HookEvent::SubagentStart => {
                        policy.session_inject
                            && policy.child_propagation == ChildPropagation::Propagate
                    }
                    HookEvent::PreToolUse => policy.action_gate && !gated_tools.is_empty(),
                    _ => false,
                };
                let behaviour_requires = behaviours.iter().any(|behaviour| {
                    // A placeholder that cannot fire and has no action must not install a hook.
                    // Its explicit host fallback remains independently operative.
                    let inert = behaviour.predicate_kind == PredicateKind::Never
                        && behaviour.action == LifecycleAction::NoOp;
                    (!inert && behaviour.event == event)
                        || (!harness_supports_event(adapter, behaviour.event)
                            && behaviour.fallback_for(event).is_some())
                });
                let boundary_requires =
                    matches!(event, HookEvent::PermissionRequest | HookEvent::PreToolUse)
                        && facts.write_boundary;
                let semantic_requires = match event {
                    HookEvent::UserPromptSubmit => facts.operations || facts.modes,
                    HookEvent::UserPromptExpansion => facts.operations || facts.modes,
                    HookEvent::PreToolUse | HookEvent::PermissionRequest | HookEvent::Stop => {
                        facts.operations
                    }
                    HookEvent::PostToolUse => facts.operations || facts.repository_verifiers,
                    HookEvent::PostToolUseFailure => facts.operations,
                    _ => false,
                };
                policy_requires || behaviour_requires || boundary_requires || semantic_requires
            })
            .collect();

        Self {
            events,
            gated_tools,
        }
    }
}
