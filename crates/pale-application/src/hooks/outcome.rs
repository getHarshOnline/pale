//! Typed hook outcome composition before host-specific encoding.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use std::collections::BTreeSet;

use pale_core::engine_wire::DecisionDispositionV1;

use super::output::{ask_output, deny_output, inject_output};
use super::protocol::{event_contract, DecisionChannel};
use super::{HookEvent, HookOutput};

/// One policy result before host encoding.
///
/// Composition is ordered by authority: deny, then ask, then context.
/// Use [`Self::merge`] to preserve that order before calling
/// [`Self::into_output`]; encoding policy layers independently can let a weaker
/// response obscure a stronger one.
#[must_use = "a hook policy outcome must be composed or encoded for the native harness"]
#[derive(Clone, PartialEq, Eq)]
pub enum HookOutcome {
    /// Require the current invocation to be refused with the composed reason.
    Deny(String),
    /// Pause the invocation for native owner confirmation.
    Ask(String),
    /// Permit the invocation while adding bounded model-facing context.
    Context(String),
}

impl std::fmt::Debug for HookOutcome {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (kind, bytes) = match self {
            Self::Deny(text) => ("deny", text.len()),
            Self::Ask(text) => ("ask", text.len()),
            Self::Context(text) => ("context", text.len()),
        };
        formatter
            .debug_struct("HookOutcome")
            .field("kind", &kind)
            .field("bytes", &bytes)
            .finish()
    }
}

impl HookOutcome {
    /// Return decision text that must be recorded with its event-aware disposition.
    pub fn decision_reason(&self) -> Option<&str> {
        match self {
            Self::Deny(reason) | Self::Ask(reason) => Some(reason),
            Self::Context(_) => None,
        }
    }

    /// Classify selected response intent for this host event, before delivery.
    pub fn disposition(
        &self,
        host: TerminalAdapterDescriptor,
        event: HookEvent,
    ) -> DecisionDispositionV1 {
        if matches!(
            event,
            HookEvent::PostToolUse | HookEvent::PostToolUseFailure
        ) {
            return DecisionDispositionV1::PostEffectSteer;
        }
        if matches!(
            event,
            HookEvent::SubagentStop | HookEvent::Stop | HookEvent::PreCompact
        ) && !matches!(self, Self::Context(_))
        {
            return DecisionDispositionV1::CompletionSteer;
        }
        match self {
            Self::Context(_) => DecisionDispositionV1::Notification,
            Self::Ask(_)
                if event == HookEvent::PreToolUse
                    && host == TerminalAdapterDescriptor::claude_code() =>
            {
                DecisionDispositionV1::RequestConfirmation
            }
            Self::Deny(_) | Self::Ask(_)
                if event_contract(host, event)
                    .is_some_and(|contract| contract.decision != DecisionChannel::None) =>
            {
                DecisionDispositionV1::Deny
            }
            Self::Deny(_) | Self::Ask(_) => DecisionDispositionV1::Notification,
        }
    }

    /// Merge independently evaluated policy layers by strength.
    ///
    /// Equal-strength text composes in stable order with exact duplicates
    /// removed. Denial always dominates confirmation and context.
    ///
    /// # Examples
    ///
    /// ```
    /// use pale_application::hooks::HookOutcome;
    ///
    /// let outcome = HookOutcome::Context("advice".into())
    ///     .merge(HookOutcome::Deny("protected path".into()));
    /// assert!(matches!(
    ///     outcome,
    ///     HookOutcome::Deny(reason) if reason == "protected path"
    /// ));
    /// ```
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Deny(left), Self::Deny(right)) => Self::Deny(compose(&left, &right)),
            (Self::Deny(reason), _) | (_, Self::Deny(reason)) => Self::Deny(reason),
            (Self::Ask(left), Self::Ask(right)) => Self::Ask(compose(&left, &right)),
            (Self::Ask(reason), _) | (_, Self::Ask(reason)) => Self::Ask(reason),
            (Self::Context(left), Self::Context(right)) => Self::Context(compose(&left, &right)),
        }
    }

    /// Encode once, after every policy layer has been resolved.
    pub fn into_output(self, host: TerminalAdapterDescriptor, event: HookEvent) -> HookOutput {
        let stdout = match self {
            Self::Deny(reason) => deny_output(host, event, &reason),
            Self::Ask(reason) => ask_output(host, event, &reason),
            Self::Context(context) if context.is_empty() => String::new(),
            Self::Context(context) => inject_output(host, event, &context),
        };
        HookOutput::ok(stdout)
    }
}

fn compose(left: &str, right: &str) -> String {
    let mut seen = BTreeSet::new();
    [left, right]
        .into_iter()
        .filter(|part| !part.is_empty() && seen.insert(*part))
        .collect::<Vec<_>>()
        .join("\n\n")
}
