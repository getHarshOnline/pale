//! Checked aggregation of caller-supplied cumulative cost snapshots.
//!
//! Only the highest snapshot index for each session contributes. Dates are
//! supplied data rather than wall-clock reads, and amounts are integer micro-USD.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use pale_core::records::{
    ActionError, ActionErrorCode, CostAggregateInput, CostAggregateOutput, CostAggregateRow,
    CostSnapshot, COST_AGGREGATE_ACTION_ID,
};

use super::{error::action_error, validation};

/// Maximum caller-supplied cost snapshots aggregated in one invocation.
const MAX_SNAPSHOTS: usize = 100_000;

fn invalid(message: impl Into<String>) -> ActionError {
    action_error(
        Some(COST_AGGREGATE_ACTION_ID),
        ActionErrorCode::MalformedInput,
        message,
    )
}

fn parse_date(date: &str) -> Option<NaiveDate> {
    let bytes = date.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| !matches!(index, 4 | 7) && !byte.is_ascii_digit())
    {
        return None;
    }
    let parse = |start: usize, end: usize| {
        date.get(start..end)
            .and_then(|part| part.parse::<u32>().ok())
    };
    let (Some(year), Some(month), Some(day)) = (parse(0, 4), parse(5, 7), parse(8, 10)) else {
        return None;
    };
    let year = i32::try_from(year).ok()?;
    (year != 0)
        .then(|| NaiveDate::from_ymd_opt(year, month, day))
        .flatten()
}

fn validate_snapshot(snapshot: &CostSnapshot, today: NaiveDate) -> Result<NaiveDate, ActionError> {
    validation::validate_token(
        COST_AGGREGATE_ACTION_ID,
        "session id",
        &snapshot.session_id,
        256,
    )?;
    validation::validate_token(COST_AGGREGATE_ACTION_ID, "model id", &snapshot.model, 256)?;
    let date = parse_date(&snapshot.date).filter(|date| *date <= today);
    let Some(date) = date else {
        return Err(invalid(format!(
            "snapshot for session `{}` has an invalid or future date",
            snapshot.session_id
        )));
    };
    validation::validate_safe_integer(
        COST_AGGREGATE_ACTION_ID,
        "snapshot index",
        snapshot.snapshot_index,
    )?;
    validation::validate_safe_integer(
        COST_AGGREGATE_ACTION_ID,
        "cost in micro-USD",
        snapshot.cost_micro_usd,
    )?;
    Ok(date)
}

/// Aggregate bounded cost entries using checked integer arithmetic.
pub(super) fn execute(input: &CostAggregateInput) -> Result<CostAggregateOutput, ActionError> {
    let today_date =
        parse_date(&input.today).ok_or_else(|| invalid("today must be a valid YYYY-MM-DD date"))?;
    if input.snapshots.len() > MAX_SNAPSHOTS {
        return Err(invalid(format!(
            "cost aggregation exceeds the {MAX_SNAPSHOTS}-snapshot bound"
        )));
    }

    let mut ordered = Vec::with_capacity(input.snapshots.len());
    for snapshot in &input.snapshots {
        let date = validate_snapshot(snapshot, today_date)?;
        ordered.push((snapshot, date));
    }
    ordered.sort_unstable_by(|left, right| {
        left.0
            .session_id
            .cmp(&right.0.session_id)
            .then_with(|| left.0.snapshot_index.cmp(&right.0.snapshot_index))
    });

    let mut latest = BTreeMap::<&str, (&CostSnapshot, NaiveDate)>::new();
    let mut previous: Option<(&CostSnapshot, NaiveDate)> = None;
    for (snapshot, date) in ordered {
        if let Some((prior, prior_date)) =
            previous.filter(|(prior, _)| prior.session_id == snapshot.session_id)
        {
            if prior.snapshot_index == snapshot.snapshot_index {
                return Err(action_error(
                    Some(COST_AGGREGATE_ACTION_ID),
                    ActionErrorCode::DuplicateIdentifier,
                    format!(
                        "session `{}` repeats snapshot index {}",
                        snapshot.session_id, snapshot.snapshot_index
                    ),
                ));
            }
            if date < prior_date {
                return Err(invalid(format!(
                    "session `{}` moves backward from date {} to {}",
                    snapshot.session_id, prior.date, snapshot.date
                )));
            }
            if snapshot.cost_micro_usd < prior.cost_micro_usd {
                return Err(invalid(format!(
                    "session `{}` cumulative cost decreases at snapshot {}",
                    snapshot.session_id, snapshot.snapshot_index
                )));
            }
        }
        latest.insert(snapshot.session_id.as_str(), (snapshot, date));
        previous = Some((snapshot, date));
    }

    let mut groups = BTreeMap::<(&str, &str), (u64, u64)>::new();
    let mut total = 0_u64;
    let mut today_cost = 0_u64;
    for (snapshot, date) in latest.values() {
        total = validation::checked_add(
            COST_AGGREGATE_ACTION_ID,
            "total cost",
            total,
            snapshot.cost_micro_usd,
        )?;
        if *date == today_date {
            today_cost = validation::checked_add(
                COST_AGGREGATE_ACTION_ID,
                "today cost",
                today_cost,
                snapshot.cost_micro_usd,
            )?;
        }
        let group = groups
            .entry((snapshot.date.as_str(), snapshot.model.as_str()))
            .or_default();
        group.0 =
            validation::checked_add(COST_AGGREGATE_ACTION_ID, "group session count", group.0, 1)?;
        group.1 = validation::checked_add(
            COST_AGGREGATE_ACTION_ID,
            "group cost",
            group.1,
            snapshot.cost_micro_usd,
        )?;
    }
    let selected_sessions = u64::try_from(latest.len())
        .map_err(|_| invalid("selected session count does not fit in u64"))?;

    Ok(CostAggregateOutput {
        selected_sessions,
        total_cost_micro_usd: total,
        today_cost_micro_usd: today_cost,
        by_day_model: groups
            .into_iter()
            .map(
                |((date, model), (sessions, cost_micro_usd))| CostAggregateRow {
                    date: date.to_string(),
                    model: model.to_string(),
                    sessions,
                    cost_micro_usd,
                },
            )
            .collect(),
    })
}

#[cfg(test)]
#[path = "../../tests/internal/actions/cost.rs"]
mod tests;
