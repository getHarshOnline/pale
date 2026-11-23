//! Host-faithful output shaping for the run-policy triad.
//!
//! Native Claude reads raw stdout at [`HookEvent::SessionStart`]; other events and
//! hosts need the JSON `hookSpecificOutput` envelope (`additionalContext`) or the text is dropped. Codex
//! Tool decisions use `permissionDecision`; prompt decisions use the host's
//! top-level `decision` shape. `systemMessage` is deliberately reserved for
//! exceptional user-visible warnings rather than repeated branding.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use super::protocol::{event_contract, ContextChannel, DecisionChannel};
use super::{host_event_name, HookEvent, HookOutcome, HookOutput};

/// Emit context in the host's required shape.
pub(super) fn inject_output(
    host: TerminalAdapterDescriptor,
    event: HookEvent,
    injection: &str,
) -> String {
    match event_contract(host, event).map(|contract| contract.context) {
        Some(ContextChannel::RawStdout) => injection.to_string(),
        Some(ContextChannel::AdditionalContext) => serde_json::json!({
            "hookSpecificOutput": {
                "hookEventName": host_event_name(event),
                "additionalContext": injection
            }
        })
        .to_string(),
        Some(ContextChannel::None) | None => String::new(),
    }
}

/// Emit an `ask` (confirm-before-proceeding) decision in the host's required shape.
pub(super) fn ask_output(
    host: TerminalAdapterDescriptor,
    event: HookEvent,
    reason: &str,
) -> String {
    if event == HookEvent::PreToolUse {
        // Codex currently parses but does not implement `ask` for PreToolUse; it
        // treats the hook as failed and continues the tool. Preserve the intended
        // confirmation boundary by denying this invocation instead of emitting a
        // misleading unsupported decision.
        if host == TerminalAdapterDescriptor::codex() {
            return deny_output(
                host,
                event,
                &format!("{reason} Codex cannot request confirmation from PreToolUse; retry after explicit authorization."),
            );
        }
        let inner = serde_json::json!({
            "hookEventName": host_event_name(event),
            "permissionDecision": "ask",
            "permissionDecisionReason": reason
        });
        return serde_json::json!({ "hookSpecificOutput": inner }).to_string();
    }

    // Prompt events have no host-level `ask` decision. Blocking is the only faithful way to
    // require confirmation without letting the current invocation proceed as if it were approved.
    block_output(host, event, reason)
}

/// Emit a block for events whose protocol uses a top-level decision.
fn block_output(_host: TerminalAdapterDescriptor, event: HookEvent, reason: &str) -> String {
    let output = serde_json::json!({
        "decision": "block",
        "reason": reason,
    });
    debug_assert_ne!(event, HookEvent::PreToolUse);
    output.to_string()
}

fn stop_output(reason: &str) -> String {
    serde_json::json!({
        "continue": false,
        "stopReason": reason,
    })
    .to_string()
}

fn notification_output(reason: &str) -> String {
    serde_json::json!({ "systemMessage": reason }).to_string()
}

/// Encode a concrete refusal using the strongest channel the host event offers.
/// Events without a decision channel can only receive an explicit notification;
/// the caller must not mistake that fallback for rollback or host enforcement.
pub fn deny_event(host: TerminalAdapterDescriptor, event: HookEvent, reason: &str) -> HookOutput {
    HookOutcome::Deny(reason.to_string()).into_output(host, event)
}

/// Emit the strongest honest response when hook evaluation itself could not be
/// trusted. Pre-effect and completion events use their native decision channel;
/// context-only events receive a built-in degraded-mode instruction; events
/// with neither affordance can only notify the user.
pub fn failure_event(
    host: TerminalAdapterDescriptor,
    event: HookEvent,
    reason: &str,
) -> HookOutput {
    let message = format!(
        "Pale could not verify this `{}` lifecycle invocation: {reason}. Treat Pale context and safeguards as unavailable for this event; do not perform consequential actions until project-local activation is repaired.",
        event.as_str()
    );
    let contract = event_contract(host, event);
    let stdout = match contract.map(|item| item.decision) {
        Some(DecisionChannel::ToolPermission)
        | Some(DecisionChannel::PermissionRequest)
        | Some(DecisionChannel::TopLevelBlock)
        | Some(DecisionChannel::UniversalStop) => deny_output(host, event, &message),
        Some(DecisionChannel::None) => match contract.map(|item| item.context) {
            Some(ContextChannel::RawStdout | ContextChannel::AdditionalContext) => {
                inject_output(host, event, &message)
            }
            Some(ContextChannel::None) | None => notification_output(&message),
        },
        None => notification_output(&message),
    };
    HookOutput::ok(stdout)
}

/// Emit a deny decision in the shape required by the event. Claude tool decisions use
/// `hookSpecificOutput.permissionDecision`; prompt submit/expansion and the other blockable
/// lifecycle events use a top-level `decision: block`.
pub(super) fn deny_output(
    host: TerminalAdapterDescriptor,
    event: HookEvent,
    reason: &str,
) -> String {
    match event_contract(host, event).map(|contract| contract.decision) {
        Some(DecisionChannel::PermissionRequest) => serde_json::json!({
            "hookSpecificOutput": {
                "hookEventName": host_event_name(event),
                "decision": { "behavior": "deny", "message": reason }
            }
        })
        .to_string(),
        Some(DecisionChannel::ToolPermission) => {
            let inner = serde_json::json!({
                "hookEventName": host_event_name(event),
                "permissionDecision": "deny",
                "permissionDecisionReason": reason
            });
            serde_json::json!({ "hookSpecificOutput": inner }).to_string()
        }
        Some(DecisionChannel::TopLevelBlock) => block_output(host, event, reason),
        Some(DecisionChannel::UniversalStop) => stop_output(reason),
        Some(DecisionChannel::None) => match event_contract(host, event).map(|item| item.context) {
            Some(ContextChannel::RawStdout | ContextChannel::AdditionalContext) => {
                inject_output(host, event, reason)
            }
            Some(ContextChannel::None) | None => notification_output(reason),
        },
        None => notification_output(reason),
    }
}
