//! Order-independent reduction of normalized loop lifecycle evidence.
//!
//! Times and thresholds are caller-supplied milliseconds; this reducer does not
//! consult a clock.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    ActionError, ActionErrorCode, LoopEvent, LoopRecommendation, LoopStatusInput, LoopStatusOutput,
    LoopToolKind, OverdueWakeup, StaleToolUse, LOOP_STATUS_REDUCE_ACTION_ID,
};

use super::{error::action_error, validation};

/// Maximum lifecycle events reduced in one invocation.
const MAX_EVENTS: usize = 50_000;

fn invalid(message: impl Into<String>) -> ActionError {
    action_error(
        Some(LOOP_STATUS_REDUCE_ACTION_ID),
        ActionErrorCode::MalformedInput,
        message,
    )
}

fn event_id(event: &LoopEvent) -> &str {
    match event {
        LoopEvent::ToolUse { id, .. }
        | LoopEvent::ToolResult { id, .. }
        | LoopEvent::WakeupScheduled { id, .. }
        | LoopEvent::WakeupObserved { id, .. }
        | LoopEvent::ParseError { id, .. } => id,
    }
}

/// Classify loop progress without equating activity with convergence.
pub(super) fn execute(input: &LoopStatusInput) -> Result<LoopStatusOutput, ActionError> {
    validation::validate_safe_integer(LOOP_STATUS_REDUCE_ACTION_ID, "current time", input.now_ms)?;
    validation::validate_safe_integer(
        LOOP_STATUS_REDUCE_ACTION_ID,
        "Bash stale threshold",
        input.bash_stale_after_ms,
    )?;
    validation::validate_safe_integer(
        LOOP_STATUS_REDUCE_ACTION_ID,
        "wakeup grace threshold",
        input.wakeup_grace_ms,
    )?;
    if input.bash_stale_after_ms == 0 {
        return Err(invalid("bash_stale_after_ms must be non-zero"));
    }
    if input.events.len() > MAX_EVENTS {
        return Err(invalid(format!(
            "loop status exceeds the {MAX_EVENTS}-event bound"
        )));
    }

    let mut event_ids = BTreeSet::new();
    let mut tool_uses = BTreeMap::new();
    let mut tool_results = BTreeMap::new();
    let mut wakeups = BTreeMap::new();
    let mut observations = BTreeMap::new();
    let mut parse_errors = Vec::new();
    for event in &input.events {
        let id = event_id(event);
        validation::validate_token(LOOP_STATUS_REDUCE_ACTION_ID, "event id", id, 256)?;
        if !event_ids.insert(id) {
            return Err(action_error(
                Some(LOOP_STATUS_REDUCE_ACTION_ID),
                ActionErrorCode::DuplicateIdentifier,
                format!("duplicate loop event id `{id}`"),
            ));
        }
        match event {
            LoopEvent::ToolUse {
                id,
                tool_kind,
                started_at_ms,
            } => {
                validation::validate_safe_integer(
                    LOOP_STATUS_REDUCE_ACTION_ID,
                    "tool start time",
                    *started_at_ms,
                )?;
                if *started_at_ms > input.now_ms {
                    return Err(invalid(format!(
                        "tool use `{id}` starts after the supplied current time"
                    )));
                }
                tool_uses.insert(id.as_str(), (*tool_kind, *started_at_ms));
            }
            LoopEvent::ToolResult {
                id,
                tool_use_id,
                completed_at_ms,
                succeeded,
            } => {
                validation::validate_token(
                    LOOP_STATUS_REDUCE_ACTION_ID,
                    "tool use reference",
                    tool_use_id,
                    256,
                )?;
                validation::validate_safe_integer(
                    LOOP_STATUS_REDUCE_ACTION_ID,
                    "tool completion time",
                    *completed_at_ms,
                )?;
                if *completed_at_ms > input.now_ms {
                    return Err(invalid(format!(
                        "tool result `{id}` completes after the supplied current time"
                    )));
                }
                if tool_results
                    .insert(
                        tool_use_id.as_str(),
                        (id.as_str(), *completed_at_ms, *succeeded),
                    )
                    .is_some()
                {
                    return Err(action_error(
                        Some(LOOP_STATUS_REDUCE_ACTION_ID),
                        ActionErrorCode::DuplicateIdentifier,
                        format!("tool use `{tool_use_id}` has multiple results"),
                    ));
                }
            }
            LoopEvent::WakeupScheduled { id, due_at_ms } => {
                validation::validate_safe_integer(
                    LOOP_STATUS_REDUCE_ACTION_ID,
                    "wakeup due time",
                    *due_at_ms,
                )?;
                wakeups.insert(id.as_str(), *due_at_ms);
            }
            LoopEvent::WakeupObserved {
                id,
                wakeup_id,
                observed_at_ms,
            } => {
                validation::validate_token(
                    LOOP_STATUS_REDUCE_ACTION_ID,
                    "wakeup reference",
                    wakeup_id,
                    256,
                )?;
                validation::validate_safe_integer(
                    LOOP_STATUS_REDUCE_ACTION_ID,
                    "wakeup observation time",
                    *observed_at_ms,
                )?;
                if *observed_at_ms > input.now_ms {
                    return Err(invalid(format!(
                        "wakeup observation `{id}` occurs after the supplied current time"
                    )));
                }
                if observations
                    .insert(wakeup_id.as_str(), *observed_at_ms)
                    .is_some()
                {
                    return Err(action_error(
                        Some(LOOP_STATUS_REDUCE_ACTION_ID),
                        ActionErrorCode::DuplicateIdentifier,
                        format!("wakeup `{wakeup_id}` has multiple observations"),
                    ));
                }
            }
            LoopEvent::ParseError { id, observed_at_ms } => {
                validation::validate_safe_integer(
                    LOOP_STATUS_REDUCE_ACTION_ID,
                    "parse-error time",
                    *observed_at_ms,
                )?;
                if *observed_at_ms > input.now_ms {
                    return Err(invalid(format!(
                        "parse error `{id}` occurs after the supplied current time"
                    )));
                }
                parse_errors.push(id.clone());
            }
        }
    }

    for (tool_use_id, (_, completed_at_ms, _)) in &tool_results {
        let (_, started_at_ms) = tool_uses.get(tool_use_id).ok_or_else(|| {
            invalid(format!(
                "tool result references unknown tool use `{tool_use_id}`"
            ))
        })?;
        if completed_at_ms < started_at_ms {
            return Err(invalid(format!(
                "tool result for `{tool_use_id}` precedes its tool use"
            )));
        }
    }
    for (wakeup_id, observed_at_ms) in &observations {
        let due_at_ms = wakeups.get(wakeup_id).ok_or_else(|| {
            invalid(format!(
                "wakeup observation references unknown wakeup `{wakeup_id}`"
            ))
        })?;
        if observed_at_ms < due_at_ms {
            return Err(invalid(format!(
                "wakeup `{wakeup_id}` is observed before it is due"
            )));
        }
    }

    let mut stale_bash = Vec::new();
    for (tool_use_id, (kind, started_at_ms)) in &tool_uses {
        if *kind != LoopToolKind::Bash || tool_results.contains_key(tool_use_id) {
            continue;
        }
        let age_ms = input.now_ms - started_at_ms;
        if age_ms >= input.bash_stale_after_ms {
            stale_bash.push(StaleToolUse {
                tool_use_id: (*tool_use_id).to_string(),
                age_ms,
            });
        }
    }
    let mut overdue_wakeups = Vec::new();
    for (wakeup_id, due_at_ms) in &wakeups {
        if observations.contains_key(wakeup_id) {
            continue;
        }
        let deadline = due_at_ms
            .checked_add(input.wakeup_grace_ms)
            .ok_or_else(|| {
                action_error(
                    Some(LOOP_STATUS_REDUCE_ACTION_ID),
                    ActionErrorCode::NumericOverflow,
                    format!("wakeup `{wakeup_id}` deadline overflowed u64"),
                )
            })?;
        validation::validate_safe_integer(
            LOOP_STATUS_REDUCE_ACTION_ID,
            "wakeup deadline",
            deadline,
        )?;
        if input.now_ms > deadline {
            overdue_wakeups.push(OverdueWakeup {
                wakeup_id: (*wakeup_id).to_string(),
                overdue_ms: input.now_ms - deadline,
            });
        }
    }
    let mut failed_tool_result_ids = tool_results
        .values()
        .filter(|(_, _, succeeded)| !succeeded)
        .map(|(result_id, _, _)| (*result_id).to_string())
        .collect::<Vec<_>>();
    failed_tool_result_ids.sort();
    parse_errors.sort();

    let recommendation = if !parse_errors.is_empty() {
        LoopRecommendation::Stop
    } else if !stale_bash.is_empty()
        || !overdue_wakeups.is_empty()
        || !failed_tool_result_ids.is_empty()
    {
        LoopRecommendation::Intervene
    } else {
        LoopRecommendation::Continue
    };
    Ok(LoopStatusOutput {
        stale_bash,
        overdue_wakeups,
        failed_tool_result_ids,
        parse_error_ids: parse_errors,
        recommendation,
    })
}

#[cfg(test)]
#[path = "../../tests/internal/actions/loop_status.rs"]
mod tests;
