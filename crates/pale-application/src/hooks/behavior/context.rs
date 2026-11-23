//! Invocation facts and fail-closed predicate evaluation.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use pale_core::records::{
    CapabilityIdV1, FailurePosture, LifecycleBehavior, ModeAxis, PackageResourceRefV1,
    PredicateKind,
};

use crate::hooks::matcher::{lifecycle_tool_call_matches, star_pattern_matches};
use crate::hooks::prompt::{command_prefix_matches, normalized_whole_message};
use crate::hooks::shell::{git_hook_bypass, recursive_force_delete};
use crate::hooks::{HookEvent, TargetPathFact, TargetPresence};
use crate::host_tool::{protocol_tool_kind, ProtocolToolKind};
use crate::session_state::SessionRuntimeState;

/// The runtime facts a lifecycle behavior's predicate is evaluated against. Facts not
/// carried here (or session state at a call site that lacks it) are absent; a
/// predicate that needs an absent fact is UNEVALUABLE and resolves per the behavior's
/// `failure_posture` (fail-closed blocks; other postures skip until the fact exists).
#[derive(Debug, Clone, Copy)]
pub struct HookContext<'a> {
    /// Application adapter that decoded the provider-bound event.
    pub adapter: TerminalAdapterDescriptor,
    /// Native host tool identity observed at this event boundary.
    pub tool_name: Option<&'a str>,
    /// Parsed command text for a command-oriented host tool.
    pub tool_command: Option<&'a str>,
    /// The whole user message (for `command_prefix` and `exact_message` activation).
    pub message: Option<&'a str>,
    /// Pre-action facts for every structured target. This is the single target
    /// source for write, path, and first-touch predicates.
    pub target_facts: &'a [TargetPathFact],
    /// Whether a sealed operation contract admitted this exact tool call.
    pub protected_write_admitted: bool,
    /// This prompt resolved to a mode activation or reset in the sealed run-mode catalog.
    pub mode_changed: bool,
    /// A sealed post-compaction refresh is waiting for this context-bearing prompt.
    pub rebaseline_pending: bool,
    /// The sealed run policy admits project and mode context for this child.
    pub child_context_admitted: bool,
    /// The active run-modes (a behavior scoped to modes fires only when one is active).
    pub active_modes: &'a [PackageResourceRefV1],
    /// Selected mode id to axis bindings from the sealed context.
    pub mode_axes: &'a [(PackageResourceRefV1, ModeAxis)],
    /// One declared default per selected axis, also from the seal.
    pub mode_defaults: &'a [(ModeAxis, PackageResourceRefV1)],
    /// Exact requirement ids whose sealed project binding is available for this
    /// native-harness route.
    pub available_tool_requirements: &'a [CapabilityIdV1],
}

fn path_matches(behavior: &LifecycleBehavior, path: &str) -> bool {
    if behavior.case_insensitive {
        let path = path.to_lowercase();
        behavior
            .matchers
            .iter()
            .any(|pattern| star_pattern_matches(&pattern.to_lowercase(), &path))
    } else {
        behavior
            .matchers
            .iter()
            .any(|pattern| star_pattern_matches(pattern, path))
    }
}

/// Whether these exact paths fall under an admitted exception to a selected,
/// fail-closed path guard that also binds a verifier requirement.
///
/// Pre-tool evaluation requires proof that an operation admitted the call;
/// post-tool evaluation covers a newly-created protected target as well.
pub fn protected_write_requires_verification(
    behaviors: &[LifecycleBehavior],
    event: HookEvent,
    paths: &[String],
    operation_admitted: bool,
) -> bool {
    matches!(event, HookEvent::PreToolUse | HookEvent::PostToolUse)
        && (event == HookEvent::PostToolUse || operation_admitted)
        && behaviors.iter().any(|behavior| {
            admitted_path_exception_requires_verification(behavior)
                && paths.iter().any(|path| path_matches(behavior, path))
        })
}

/// Structural contract for a protected-path exception whose successful use
/// must be followed by a repository verifier.
pub(crate) fn admitted_path_exception_requires_verification(behavior: &LifecycleBehavior) -> bool {
    behavior.event == HookEvent::PreToolUse
        && behavior.predicate_kind == PredicateKind::PathMatch
        && behavior.action == pale_core::records::LifecycleAction::Deny
        && behavior.failure_posture == FailurePosture::FailClosed
        && behavior.allow_admitted_operation
}

/// Whether a behavior's predicate holds: `Some(true/false)` when evaluable, `None` when it
/// needs a fact not available here. `state` carries session runtime facts: with it, a
/// `first_touch` predicate is evaluable (has this target been touched this session?); without
/// it (the effects-free output path passes `None`), first-touch stays unevaluable and resolves
/// via `failure_posture`.
fn predicate_holds(
    b: &LifecycleBehavior,
    ctx: &HookContext,
    state: Option<&SessionRuntimeState>,
) -> Option<bool> {
    match b.predicate_kind {
        PredicateKind::Always => Some(true),
        PredicateKind::Never => Some(false),
        PredicateKind::ToolMatch => ctx
            .tool_name
            .and_then(|tool| lifecycle_tool_call_matches(tool, ctx.tool_command, &b.matchers)),
        PredicateKind::DestructiveShell => match ctx.tool_name {
            Some(tool) if protocol_tool_kind(ctx.adapter, tool) == ProtocolToolKind::Command => {
                ctx.tool_command.and_then(recursive_force_delete)
            }
            Some(_) => Some(false),
            None => None,
        },
        PredicateKind::GitHookBypass => match ctx.tool_name {
            Some(tool) if protocol_tool_kind(ctx.adapter, tool) == ProtocolToolKind::Command => {
                ctx.tool_command.and_then(git_hook_bypass)
            }
            Some(_) => Some(false),
            None => None,
        },
        PredicateKind::WriteEffect => Some(!ctx.target_facts.is_empty()),
        PredicateKind::PathMatch => {
            if b.allow_admitted_operation && ctx.protected_write_admitted {
                return Some(false);
            }
            if !b.existing_targets_only {
                return Some(
                    ctx.target_facts
                        .iter()
                        .any(|fact| path_matches(b, &fact.path)),
                );
            }
            let mut unknown = false;
            for fact in ctx
                .target_facts
                .iter()
                .filter(|fact| path_matches(b, &fact.path))
            {
                match fact.presence {
                    TargetPresence::Present => return Some(true),
                    TargetPresence::Missing => {}
                    TargetPresence::Unknown => unknown = true,
                }
            }
            (!unknown).then_some(false)
        }
        // A prompt command: the normalized message BEGINS with a matcher token.
        PredicateKind::CommandPrefix => ctx.message.map(|message| {
            b.matchers
                .iter()
                .any(|m| command_prefix_matches(message, m))
        }),
        // A standalone command: the normalized WHOLE message equals a matcher literal.
        PredicateKind::ExactMessage => ctx.message.map(|m| {
            let t = normalized_whole_message(m);
            b.matchers.iter().any(|k| t == normalized_whole_message(k))
        }),
        PredicateKind::ModeChanged => Some(ctx.mode_changed),
        PredicateKind::RebaselinePending => Some(ctx.rebaseline_pending),
        // First touch of the target this session: evaluable only when both the target path
        // and the session state are available (true = not yet touched = this IS the first).
        PredicateKind::FirstTouch => state.and_then(|state| {
            (!ctx.target_facts.is_empty()).then(|| {
                ctx.target_facts
                    .iter()
                    .any(|fact| !state.has_touched(&fact.path))
            })
        }),
        PredicateKind::ConnectorBound => Some(
            !b.tool_reqs.is_empty()
                && b.tool_reqs
                    .iter()
                    .all(|requirement| ctx.available_tool_requirements.contains(requirement)),
        ),
    }
}

/// Whether a behavior is active in the current mode set (empty `modes` = every mode).
fn mode_active(b: &LifecycleBehavior, active_modes: &[PackageResourceRefV1]) -> bool {
    b.modes.is_empty() || b.modes.iter().any(|m| active_modes.contains(m))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Outcome of evaluating one lifecycle behaviour against available host facts.
pub(super) enum Evaluation {
    Skip,
    Fire,
    DenyUnevaluable,
}

/// Resolve event/mode/predicate facts without turning uncertainty into the declared action. A
/// fail-closed unknown is a real block even when the record's ordinary action is guidance or state;
/// fail-open and non-blocking recovery postures skip until their required fact exists.
pub(super) fn evaluate_behavior(
    b: &LifecycleBehavior,
    event: HookEvent,
    ctx: &HookContext,
    state: Option<&SessionRuntimeState>,
) -> Evaluation {
    if event == HookEvent::SubagentStart && !ctx.child_context_admitted {
        return Evaluation::Skip;
    }
    if b.event != event || !mode_active(b, ctx.active_modes) {
        return Evaluation::Skip;
    }
    if !b
        .tool_reqs
        .iter()
        .all(|requirement| ctx.available_tool_requirements.contains(requirement))
    {
        return if matches!(b.failure_posture, FailurePosture::FailClosed) {
            Evaluation::DenyUnevaluable
        } else {
            Evaluation::Skip
        };
    }
    match predicate_holds(b, ctx, state) {
        Some(true) => Evaluation::Fire,
        Some(false) => Evaluation::Skip,
        None if matches!(b.failure_posture, FailurePosture::FailClosed) => {
            Evaluation::DenyUnevaluable
        }
        None => Evaluation::Skip,
    }
}
