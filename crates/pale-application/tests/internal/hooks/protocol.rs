//! Native-hook protocol support proofs over the closed event/action affordance matrix.

use super::{command_timeout_ceiling_secs, event_contract, ContextChannel, DecisionChannel};
use crate::terminal_adapters::TerminalAdapterDescriptor;
use pale_core::records::{LifecycleAction, LifecycleEvent};

fn supports_context(host: TerminalAdapterDescriptor, event: LifecycleEvent) -> bool {
    event_contract(host, event).is_some_and(|contract| contract.context != ContextChannel::None)
}

fn supports_action(
    host: TerminalAdapterDescriptor,
    event: LifecycleEvent,
    action: LifecycleAction,
) -> bool {
    let Some(contract) = event_contract(host, event) else {
        return false;
    };
    match action {
        LifecycleAction::Deny | LifecycleAction::Ask => contract.decision != DecisionChannel::None,
        LifecycleAction::Inject
        | LifecycleAction::InjectModes
        | LifecycleAction::Warn
        | LifecycleAction::Guide
        | LifecycleAction::Notify
        | LifecycleAction::Signal => contract.context != ContextChannel::None,
        LifecycleAction::Activate
        | LifecycleAction::Deactivate
        | LifecycleAction::Capture
        | LifecycleAction::Rebaseline
        | LifecycleAction::NoOp => true,
    }
}

#[test]
fn action_support_tracks_the_exact_host_event_affordance() {
    assert!(supports_action(
        TerminalAdapterDescriptor::claude_code(),
        LifecycleEvent::PreToolUse,
        LifecycleAction::Deny,
    ));
    assert!(!supports_action(
        TerminalAdapterDescriptor::claude_code(),
        LifecycleEvent::SessionStart,
        LifecycleAction::Deny,
    ));
    assert!(supports_action(
        TerminalAdapterDescriptor::codex(),
        LifecycleEvent::SessionStart,
        LifecycleAction::Inject,
    ));
    assert!(!supports_action(
        TerminalAdapterDescriptor::codex(),
        LifecycleEvent::PostCompact,
        LifecycleAction::Guide,
    ));
    assert!(supports_action(
        TerminalAdapterDescriptor::codex(),
        LifecycleEvent::PostCompact,
        LifecycleAction::Rebaseline,
    ));
    assert!(supports_context(
        TerminalAdapterDescriptor::claude_code(),
        LifecycleEvent::PostToolUseFailure,
    ));
    assert!(!supports_context(
        TerminalAdapterDescriptor::codex(),
        LifecycleEvent::PostToolUseFailure,
    ));
}

#[test]
fn command_timeout_ceiling_is_owned_by_each_host_event_contract() {
    assert_eq!(
        command_timeout_ceiling_secs(
            TerminalAdapterDescriptor::codex(),
            LifecycleEvent::SessionEnd
        ),
        Some(3)
    );
    assert_eq!(
        command_timeout_ceiling_secs(
            TerminalAdapterDescriptor::claude_code(),
            LifecycleEvent::SessionEnd
        ),
        Some(60)
    );
    assert_eq!(
        command_timeout_ceiling_secs(
            TerminalAdapterDescriptor::codex(),
            LifecycleEvent::PreToolUse
        ),
        Some(600)
    );
    assert_eq!(
        command_timeout_ceiling_secs(
            TerminalAdapterDescriptor::codex(),
            LifecycleEvent::UserPromptExpansion,
        ),
        None
    );
}
