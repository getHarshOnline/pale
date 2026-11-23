//! Shared lexical bounds for pure action inputs.

use pale_core::records::{ActionError, ActionErrorCode};

use super::{error::action_error, MAX_CANONICAL_INTEGER};

/// Validate one bounded protocol token and retain the owning field in failures.
pub(super) fn validate_token(
    action_id: &str,
    kind: &str,
    value: &str,
    max_bytes: usize,
) -> Result<(), ActionError> {
    if value.is_empty()
        || value.len() > max_bytes
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(action_error(
            Some(action_id),
            ActionErrorCode::MalformedInput,
            format!(
                "{kind} must contain 1..={max_bytes} control-free bytes with no surrounding whitespace"
            ),
        ));
    }
    Ok(())
}

/// Reject integers outside the exact JSON-safe range used by action records.
pub(super) fn validate_safe_integer(
    action_id: &str,
    kind: &str,
    value: u64,
) -> Result<(), ActionError> {
    if value > MAX_CANONICAL_INTEGER {
        return Err(action_error(
            Some(action_id),
            ActionErrorCode::NumericOverflow,
            format!("{kind} exceeds the canonical JSON safe-integer bound"),
        ));
    }
    Ok(())
}

/// Add two action counters without silently wrapping the declared ceiling.
pub(super) fn checked_add(
    action_id: &str,
    kind: &str,
    left: u64,
    right: u64,
) -> Result<u64, ActionError> {
    let value = left.checked_add(right).ok_or_else(|| {
        action_error(
            Some(action_id),
            ActionErrorCode::NumericOverflow,
            format!("{kind} overflowed u64"),
        )
    })?;
    validate_safe_integer(action_id, kind, value)?;
    Ok(value)
}
