//! Validated tool-call matching for run policy and lifecycle behavior.

use pale_core::records::ToolCallMatcher;

/// Match Pale's closed star-pattern language without allocating.
///
/// Only `*` is special and matches zero or more Unicode scalar values. `?`,
/// brackets, braces, and backslashes remain literal. Missing leading or trailing
/// stars anchor the corresponding end, so `git push*`, `*deploy*`, and
/// `rm -rf*` retain their declared behavior without acquiring full glob syntax.
pub(crate) fn star_pattern_matches(pattern: &str, value: &str) -> bool {
    let mut pattern = pattern.chars().peekable();
    let mut value = value.chars().peekable();
    let mut retry_pattern = None;
    let mut retry_value = None;

    loop {
        match (pattern.peek().copied(), value.peek().copied()) {
            (Some('*'), _) => {
                while pattern.next_if_eq(&'*').is_some() {}
                if pattern.peek().is_none() {
                    return true;
                }
                retry_pattern = Some(pattern.clone());
                retry_value = Some(value.clone());
            }
            (Some(expected), Some(observed)) if expected == observed => {
                pattern.next();
                value.next();
            }
            (None, None) => return true,
            _ => {
                let (Some(saved_pattern), Some(mut saved_value)) =
                    (retry_pattern.as_ref(), retry_value.take())
                else {
                    return false;
                };
                if saved_value.next().is_none() {
                    return false;
                }
                pattern = saved_pattern.clone();
                value = saved_value.clone();
                retry_value = Some(saved_value);
            }
        }
    }
}

/// Whether an incoming tool call matches a matcher. `None` means a matcher selected the tool but
/// its required command argument was unavailable; callers apply the declared failure posture
/// rather than conflating "unknown" with either a match or a mismatch.
pub(super) fn tool_call_matches(
    tool_name: &str,
    tool_command: Option<&str>,
    matchers: &[ToolCallMatcher],
) -> Option<bool> {
    match_tool_call(
        tool_name,
        tool_command,
        matchers
            .iter()
            .map(|matcher| (matcher.tool(), matcher.argument_glob())),
    )
}

/// Match lifecycle data admitted through the heterogeneous `matchers` field.
/// Invalid tool syntax is unevaluable and therefore follows the behavior's
/// declared failure posture rather than becoming an accidental mismatch.
pub(super) fn lifecycle_tool_call_matches(
    tool_name: &str,
    tool_command: Option<&str>,
    matchers: &[String],
) -> Option<bool> {
    let parsed = matchers
        .iter()
        .map(|matcher| ToolCallMatcher::parse(matcher))
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    tool_call_matches(tool_name, tool_command, &parsed)
}

fn match_tool_call<'a>(
    tool_name: &str,
    tool_command: Option<&str>,
    matchers: impl IntoIterator<Item = (&'a str, Option<&'a str>)>,
) -> Option<bool> {
    let mut required_command = false;
    for (mtool, marg) in matchers {
        let tool_ok = mtool == "*" || mtool == tool_name;
        if !tool_ok {
            continue;
        }
        match marg {
            None => return Some(true),
            Some(glob) => match tool_command {
                Some(command) if star_pattern_matches(glob, command) => return Some(true),
                Some(_) => {}
                None => required_command = true,
            },
        }
    }
    if required_command {
        None
    } else {
        Some(false)
    }
}

#[cfg(test)]
#[path = "../../tests/internal/hooks/matcher.rs"]
mod tests;
