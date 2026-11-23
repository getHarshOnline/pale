//! Checked fixed-point helpers shared by score reducers.
//!
//! Ratios use `MICRO_SCALE` and round down through integer division.

use pale_core::records::{ActionError, ActionErrorCode};

use super::{error::action_error, MICRO_SCALE};

/// Compute a fixed-point ratio, rejecting a zero denominator and overflow.
pub(super) fn ratio_micros(
    action_id: &str,
    numerator: u64,
    denominator: u64,
) -> Result<u32, ActionError> {
    if denominator == 0 || numerator > denominator {
        return Err(action_error(
            Some(action_id),
            ActionErrorCode::MalformedInput,
            "ratio requires 0 <= numerator <= a non-zero denominator",
        ));
    }
    let scaled = u128::from(numerator)
        .checked_mul(u128::from(MICRO_SCALE))
        .and_then(|value| value.checked_div(u128::from(denominator)))
        .ok_or_else(|| {
            action_error(
                Some(action_id),
                ActionErrorCode::NumericOverflow,
                "fixed-point ratio calculation overflowed",
            )
        })?;
    u32::try_from(scaled).map_err(|_| {
        action_error(
            Some(action_id),
            ActionErrorCode::NumericOverflow,
            "fixed-point ratio does not fit in u32",
        )
    })
}
