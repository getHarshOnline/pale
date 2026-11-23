//! Resolve one user message against the route-sealed run-mode catalog.

use pale_core::records::{ModeAxis, PackageResourceRefV1, SealedRunMode};

use super::prompt::{command_prefix_matches, normalized_whole_message};

/// A session transition resolved from the route-sealed mode vocabulary.
#[must_use = "a resolved mode command must be applied or deliberately withheld"]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModeCommand {
    /// Activate one declared mode without disturbing independent axes.
    Activate {
        /// Independent mode axis to update.
        axis: ModeAxis,
        /// Declared mode identity to activate.
        id: PackageResourceRefV1,
    },
}

/// Resolve canonical invocations, declared command aliases, and exact natural
/// triggers from one sealed catalog. No lifecycle record repeats this
/// vocabulary.
pub fn resolve_mode_command(message: &str, modes: &[SealedRunMode]) -> Option<ModeCommand> {
    let exact = normalized_whole_message(message);
    modes.iter().find_map(|mode| {
        let canonical = format!(
            "/pale-mode-{}",
            mode.id.resource_id.as_str().replace('_', "-")
        );
        let command_match = std::iter::once(canonical.as_str())
            .chain(mode.aliases.iter().map(String::as_str))
            .any(|alias| command_prefix_matches(message, alias));
        let exact_match = mode
            .exact_triggers
            .iter()
            .any(|trigger| exact == normalized_whole_message(trigger));
        (command_match || exact_match).then(|| ModeCommand::Activate {
            axis: mode.axis,
            id: mode.id.clone(),
        })
    })
}
