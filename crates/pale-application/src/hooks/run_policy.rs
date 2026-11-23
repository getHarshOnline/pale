//! Host-neutral evaluation of the sealed run-policy triad.

use pale_core::records::{ChildPropagation, RunPolicyEntry};

use crate::operation_admission::ExactToolAuthorization;

use super::matcher::{star_pattern_matches, tool_call_matches};
use super::{HookEvent, HookOutcome};

/// Whether the sealed run policy permits project context in one child session.
#[must_use = "child-context admission must be enforced before rendering or recording a child launch"]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildContextAdmission {
    /// Propagation is enabled and the child identity satisfies any sealed selector.
    Admitted,
    /// Propagation is disabled or the observed child identity does not satisfy the selector.
    Withheld,
    /// Propagation is selector-bound, but the host omitted the child identity needed to decide.
    IdentityMissing,
}

/// Resolve child propagation once from the sealed policy and host identity.
pub fn child_context_admission(
    policy: &RunPolicyEntry,
    agent_type: Option<&str>,
) -> ChildContextAdmission {
    if policy.child_propagation != ChildPropagation::Propagate {
        return ChildContextAdmission::Withheld;
    }
    if policy.child_type_selectors.is_empty() {
        return ChildContextAdmission::Admitted;
    }
    let Some(agent_type) = agent_type.filter(|value| !value.trim().is_empty()) else {
        return ChildContextAdmission::IdentityMissing;
    };
    let agent_type = agent_type.to_ascii_lowercase();
    if policy
        .child_type_selectors
        .iter()
        .any(|selector| star_pattern_matches(&selector.to_ascii_lowercase(), &agent_type))
    {
        ChildContextAdmission::Admitted
    } else {
        ChildContextAdmission::Withheld
    }
}

/// Host facts consumed by the host-neutral run-policy evaluator.
pub struct RunPolicyFacts<'a> {
    /// Child identity supplied at a child-start boundary.
    pub child_type: Option<&'a str>,
    /// Native tool identity supplied at a pre-effect boundary.
    pub tool_name: Option<&'a str>,
    /// Parsed command text for a command-oriented tool.
    pub tool_command: Option<&'a str>,
    /// Exact authorization proof supplied by operation admission.
    pub exact_authorization: Option<&'a ExactToolAuthorization>,
}

impl std::fmt::Debug for RunPolicyFacts<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RunPolicyFacts")
            .field("child_type_present", &self.child_type.is_some())
            .field("tool_name_present", &self.tool_name.is_some())
            .field("tool_command_present", &self.tool_command.is_some())
            .field("exact_authorization", &self.exact_authorization.is_some())
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PlannedRunPolicyOutcome {
    ContextRequired,
    Resolved(HookOutcome),
}

/// One host-neutral run-policy decision before optional context is attached.
#[must_use = "the run-policy plan must be supplied its required payload and composed"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunPolicyPlan {
    outcome: PlannedRunPolicyOutcome,
}

/// A run-policy plan required context that the adapter did not provide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("run policy requires the sealed project injection payload")]
pub struct RunPolicyPayloadError;

impl RunPolicyPlan {
    /// Evaluate run-policy decisions without loading payload bytes or encoding a host response.
    pub fn evaluate(event: HookEvent, policy: &RunPolicyEntry, facts: RunPolicyFacts<'_>) -> Self {
        let outcome = match event {
            HookEvent::SessionStart if policy.session_inject => {
                PlannedRunPolicyOutcome::ContextRequired
            }
            HookEvent::SubagentStart => match child_context_admission(policy, facts.child_type) {
                ChildContextAdmission::Admitted => PlannedRunPolicyOutcome::ContextRequired,
                ChildContextAdmission::IdentityMissing => PlannedRunPolicyOutcome::Resolved(
                    HookOutcome::Context(
                        "Pale withheld parent project context because this child start omitted the agent identity required by the sealed selector policy. Work only within the explicit parent task and request scoped context before consequential action."
                            .to_string(),
                    ),
                ),
                ChildContextAdmission::Withheld => {
                    PlannedRunPolicyOutcome::Resolved(HookOutcome::Context(String::new()))
                }
            },
            HookEvent::PreToolUse if policy.action_gate => {
                let deny = HookOutcome::Deny;
                let outcome = match facts.tool_name {
                    None => deny(format!(
                        "Pale run policy `{}` gates tool use but no tool name was provided; denying (fail closed).",
                        policy.id
                    )),
                    Some(tool) => match tool_call_matches(
                        tool,
                        facts.tool_command,
                        &policy.gated_tool_matchers,
                    ) {
                        Some(true) if facts.exact_authorization.is_some() => {
                            HookOutcome::Context(String::new())
                        }
                        Some(true) => deny(format!(
                            "Pale run policy `{}` gates this consequential action; complete verification and the review handoff, then obtain owner authorization bound to this exact tool request.",
                            policy.id
                        )),
                        None => deny(format!(
                            "Pale run policy `{}` requires tool arguments that were unavailable; denying (fail closed).",
                            policy.id
                        )),
                        Some(false) => HookOutcome::Context(String::new()),
                    },
                };
                PlannedRunPolicyOutcome::Resolved(outcome)
            }
            // Other lifecycle semantics come from the richer behavior catalog.
            HookEvent::SessionStart
            | HookEvent::UserPromptSubmit
            | HookEvent::UserPromptExpansion
            | HookEvent::PermissionRequest
            | HookEvent::PreToolUse
            | HookEvent::PostToolUse
            | HookEvent::PostToolUseFailure
            | HookEvent::PreCompact
            | HookEvent::PostCompact
            | HookEvent::SubagentStop
            | HookEvent::Stop
            | HookEvent::SessionEnd => {
                PlannedRunPolicyOutcome::Resolved(HookOutcome::Context(String::new()))
            }
        };
        Self { outcome }
    }

    /// Whether rendering this policy decision needs the sealed project injection.
    pub fn requires_injection(&self) -> bool {
        self.outcome == PlannedRunPolicyOutcome::ContextRequired
    }

    /// Attach the adapter-verified injection and return a composable policy outcome.
    ///
    /// # Errors
    ///
    /// Returns [`RunPolicyPayloadError`] when this plan requires project context
    /// and the supplied payload is empty.
    pub fn render(&self, injection: &str) -> Result<HookOutcome, RunPolicyPayloadError> {
        match &self.outcome {
            PlannedRunPolicyOutcome::ContextRequired if injection.is_empty() => {
                Err(RunPolicyPayloadError)
            }
            PlannedRunPolicyOutcome::ContextRequired => {
                Ok(HookOutcome::Context(injection.to_string()))
            }
            PlannedRunPolicyOutcome::Resolved(outcome) => Ok(outcome.clone()),
        }
    }
}
