//! Native-harness lifecycle contracts used by validation and output encoding.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use pale_core::records::LifecycleEvent;

/// General command-hook ceiling shared by host events without a tighter limit.
const COMMAND_TIMEOUT_CEILING_SECS: u32 = 600;
/// Claude Code's tighter terminal-event command ceiling.
const CLAUDE_SESSION_END_TIMEOUT_CEILING_SECS: u32 = 60;
/// Codex's fail-fast terminal-event command ceiling.
const CODEX_SESSION_END_TIMEOUT_CEILING_SECS: u32 = 3;

/// How model-visible context is delivered for one exact native event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContextChannel {
    None,
    RawStdout,
    AdditionalContext,
}

/// The strongest event-specific control accepted by the native harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DecisionChannel {
    None,
    TopLevelBlock,
    UniversalStop,
    ToolPermission,
    PermissionRequest,
}

/// Exact lifecycle affordances for one host/event pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EventContract {
    pub context: ContextChannel,
    pub decision: DecisionChannel,
}

impl EventContract {
    const fn new(context: ContextChannel, decision: DecisionChannel) -> Self {
        Self { context, decision }
    }
}

/// How a native harness can enforce a capability whose approval is `ask`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolConfirmation {
    NativeAsk,
    ExactAuthorization,
}

/// Return the exact output affordances Pale relies on for this host event.
///
/// A missing contract means the native harness does not expose the event. A
/// context-free contract may still block or record state; a decision-free
/// contract may still carry context. Post-effect decisions steer the agent but
/// never claim to roll back an effect that already happened.
pub(crate) fn event_contract(
    host: TerminalAdapterDescriptor,
    event: LifecycleEvent,
) -> Option<EventContract> {
    use ContextChannel::{AdditionalContext, None as NoContext, RawStdout};
    use DecisionChannel::{
        None as NoDecision, PermissionRequest, ToolPermission, TopLevelBlock, UniversalStop,
    };
    use LifecycleEvent as Event;

    let contract = match (host.is_claude_code(), event) {
        (true, Event::SessionStart) => EventContract::new(RawStdout, NoDecision),
        (true, Event::UserPromptSubmit | Event::UserPromptExpansion) => {
            EventContract::new(AdditionalContext, TopLevelBlock)
        }
        (true, Event::PermissionRequest) => EventContract::new(NoContext, PermissionRequest),
        (true, Event::PreToolUse) => EventContract::new(AdditionalContext, ToolPermission),
        (true, Event::PostToolUse) => EventContract::new(AdditionalContext, TopLevelBlock),
        (true, Event::PostToolUseFailure) => EventContract::new(AdditionalContext, NoDecision),
        (true, Event::SubagentStart) => EventContract::new(AdditionalContext, NoDecision),
        (true, Event::SubagentStop | Event::Stop) => {
            EventContract::new(AdditionalContext, TopLevelBlock)
        }
        (true, Event::PreCompact) => EventContract::new(NoContext, TopLevelBlock),
        (true, Event::PostCompact | Event::SessionEnd) => EventContract::new(NoContext, NoDecision),

        (false, Event::SessionStart | Event::SubagentStart) => {
            EventContract::new(AdditionalContext, NoDecision)
        }
        (false, Event::UserPromptSubmit) => EventContract::new(AdditionalContext, TopLevelBlock),
        (false, Event::PermissionRequest) => EventContract::new(NoContext, PermissionRequest),
        (false, Event::PreToolUse) => EventContract::new(AdditionalContext, ToolPermission),
        (false, Event::PostToolUse) => EventContract::new(AdditionalContext, TopLevelBlock),
        (false, Event::SubagentStop | Event::Stop) => EventContract::new(NoContext, TopLevelBlock),
        (false, Event::PreCompact) => EventContract::new(NoContext, UniversalStop),
        (false, Event::PostCompact | Event::SessionEnd) => {
            EventContract::new(NoContext, NoDecision)
        }
        (false, Event::UserPromptExpansion | Event::PostToolUseFailure) => {
            return None;
        }
    };
    Some(contract)
}

/// Whether the native harness exposes this lifecycle event.
pub(crate) fn supports_event(host: TerminalAdapterDescriptor, event: LifecycleEvent) -> bool {
    event_contract(host, event).is_some()
}

/// Maximum command-hook timeout admitted for one host/event pair.
///
/// The ceiling is a native protocol limit, not Package policy. Callers may
/// select a smaller timeout, but a generated contribution must never exceed
/// the host-specific bound. `None` means the event itself is unsupported.
#[must_use]
pub(crate) fn command_timeout_ceiling_secs(
    host: TerminalAdapterDescriptor,
    event: LifecycleEvent,
) -> Option<u32> {
    event_contract(host, event)?;
    Some(match (host.is_codex(), event) {
        (true, LifecycleEvent::SessionEnd) => CODEX_SESSION_END_TIMEOUT_CEILING_SECS,
        (false, LifecycleEvent::SessionEnd) => CLAUDE_SESSION_END_TIMEOUT_CEILING_SECS,
        _ => COMMAND_TIMEOUT_CEILING_SECS,
    })
}

/// Return the strongest confirmation mechanism the host can enforce safely.
pub(crate) fn tool_confirmation(host: TerminalAdapterDescriptor) -> ToolConfirmation {
    if host.is_claude_code() {
        ToolConfirmation::NativeAsk
    } else {
        ToolConfirmation::ExactAuthorization
    }
}

#[cfg(test)]
#[path = "../../tests/internal/hooks/protocol.rs"]
mod tests;
