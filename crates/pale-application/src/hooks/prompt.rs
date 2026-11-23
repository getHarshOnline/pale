//! Deterministic normalization for prompt commands and exact whole-message triggers.

/// Normalize a complete prompt for exact, punctuation-insensitive matching.
pub(super) fn normalized_whole_message(message: &str) -> String {
    message
        .trim()
        .to_lowercase()
        .trim_end_matches(|character: char| {
            matches!(character, '.' | '!' | '?') || character.is_whitespace()
        })
        .to_string()
}

fn normalized_command(message: &str) -> String {
    let message = message.trim().to_lowercase();
    match message.strip_prefix(['@', '$']) {
        Some(rest) => format!("/{rest}"),
        None => message,
    }
}

/// Match one complete command token, optionally followed by arguments.
pub(super) fn command_prefix_matches(message: &str, matcher: &str) -> bool {
    let message = normalized_command(message);
    let matcher = normalized_command(matcher);
    message == matcher
        || message
            .strip_prefix(&matcher)
            .is_some_and(|rest| rest.starts_with(char::is_whitespace))
}
