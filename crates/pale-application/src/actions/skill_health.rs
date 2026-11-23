//! Windowed health rates over caller-supplied skill observations.
//!
//! Window bounds and evaluation time are caller-supplied milliseconds. Empty
//! windows produce no rate rather than a synthetic zero-percent result.

use std::collections::BTreeSet;

use pale_core::records::{
    ActionError, ActionErrorCode, SkillHealthInput, SkillHealthOutput, SkillHealthRow,
    SkillHealthTrend, SkillRunObservation, SkillWindowRate, SKILL_HEALTH_AGGREGATE_ACTION_ID,
};

use super::{error::action_error, math, validation, MICRO_SCALE};

// Window aggregation visits every caller-supplied observation; bound the scan
// before building per-skill counters.
/// Maximum distinct skill identities aggregated in one call.
const MAX_SKILLS: usize = 1_024;
/// Maximum timestamped observations scanned in one call.
const MAX_OBSERVATIONS: usize = 100_000;

fn invalid(message: impl Into<String>) -> ActionError {
    action_error(
        Some(SKILL_HEALTH_AGGREGATE_ACTION_ID),
        ActionErrorCode::MalformedInput,
        message,
    )
}

fn window_rate(runs: u64, successes: u64) -> Result<SkillWindowRate, ActionError> {
    Ok(SkillWindowRate {
        runs,
        successes,
        success_rate_micros: (runs > 0)
            .then(|| math::ratio_micros(SKILL_HEALTH_AGGREGATE_ACTION_ID, successes, runs))
            .transpose()?,
    })
}

fn classify_trend(
    recent: &SkillWindowRate,
    long: &SkillWindowRate,
    threshold: u32,
) -> SkillHealthTrend {
    let (Some(recent), Some(long)) = (recent.success_rate_micros, long.success_rate_micros) else {
        return SkillHealthTrend::InsufficientData;
    };
    let difference = i64::from(recent) - i64::from(long);
    if difference > 0 && difference >= i64::from(threshold) {
        SkillHealthTrend::Improving
    } else if difference < 0 && -difference >= i64::from(threshold) {
        SkillHealthTrend::Declining
    } else {
        SkillHealthTrend::Stable
    }
}

fn observe(
    observation: &SkillRunObservation,
    input: &SkillHealthInput,
    recent: &mut (u64, u64),
    long: &mut (u64, u64),
) -> Result<(), ActionError> {
    validation::validate_token(
        SKILL_HEALTH_AGGREGATE_ACTION_ID,
        "observation id",
        &observation.id,
        256,
    )?;
    validation::validate_safe_integer(
        SKILL_HEALTH_AGGREGATE_ACTION_ID,
        "observation time",
        observation.occurred_at_ms,
    )?;
    if observation.occurred_at_ms > input.evaluation_time_ms {
        return Err(invalid(format!(
            "observation `{}` occurs after the supplied evaluation time",
            observation.id
        )));
    }
    if observation.occurred_at_ms >= input.long_window_start_ms {
        long.0 = validation::checked_add(
            SKILL_HEALTH_AGGREGATE_ACTION_ID,
            "long-window run count",
            long.0,
            1,
        )?;
        if observation.succeeded {
            long.1 = validation::checked_add(
                SKILL_HEALTH_AGGREGATE_ACTION_ID,
                "long-window success count",
                long.1,
                1,
            )?;
        }
    }
    if observation.occurred_at_ms >= input.recent_window_start_ms {
        recent.0 = validation::checked_add(
            SKILL_HEALTH_AGGREGATE_ACTION_ID,
            "recent-window run count",
            recent.0,
            1,
        )?;
        if observation.succeeded {
            recent.1 = validation::checked_add(
                SKILL_HEALTH_AGGREGATE_ACTION_ID,
                "recent-window success count",
                recent.1,
                1,
            )?;
        }
    }
    Ok(())
}

/// Classify skill evidence without upgrading declared availability or freshness.
pub(super) fn execute(input: &SkillHealthInput) -> Result<SkillHealthOutput, ActionError> {
    for (name, value) in [
        ("evaluation time", input.evaluation_time_ms),
        ("recent-window start", input.recent_window_start_ms),
        ("long-window start", input.long_window_start_ms),
    ] {
        validation::validate_safe_integer(SKILL_HEALTH_AGGREGATE_ACTION_ID, name, value)?;
    }
    if input.long_window_start_ms > input.recent_window_start_ms
        || input.recent_window_start_ms > input.evaluation_time_ms
    {
        return Err(invalid(
            "window boundaries must satisfy long_start <= recent_start <= evaluation_time",
        ));
    }
    if input.trend_threshold_micros > MICRO_SCALE {
        return Err(invalid(format!(
            "trend threshold must be within 0..={MICRO_SCALE}"
        )));
    }
    if input.skills.is_empty() || input.skills.len() > MAX_SKILLS {
        return Err(invalid(format!(
            "skill health requires 1..={MAX_SKILLS} skill series"
        )));
    }
    let total_observations = input
        .skills
        .iter()
        .try_fold(0_usize, |total, skill| {
            total.checked_add(skill.observations.len())
        })
        .ok_or_else(|| invalid("skill observation count overflowed the platform size"))?;
    if total_observations > MAX_OBSERVATIONS {
        return Err(invalid(format!(
            "skill health exceeds the {MAX_OBSERVATIONS}-observation bound"
        )));
    }

    let mut skill_ids = BTreeSet::new();
    let mut rows = Vec::with_capacity(input.skills.len());
    let mut portfolio_recent = (0_u64, 0_u64);
    let mut portfolio_long = (0_u64, 0_u64);
    let mut pending = 0_u64;
    let mut portfolio_last = None;
    for skill in &input.skills {
        validation::validate_token(
            SKILL_HEALTH_AGGREGATE_ACTION_ID,
            "skill id",
            &skill.skill_id,
            256,
        )?;
        if !skill_ids.insert(skill.skill_id.as_str()) {
            return Err(action_error(
                Some(SKILL_HEALTH_AGGREGATE_ACTION_ID),
                ActionErrorCode::DuplicateIdentifier,
                format!("duplicate skill series `{}`", skill.skill_id),
            ));
        }
        pending = validation::checked_add(
            SKILL_HEALTH_AGGREGATE_ACTION_ID,
            "pending amendment count",
            pending,
            u64::from(skill.pending_amendments),
        )?;
        let mut observation_ids = BTreeSet::new();
        let mut recent = (0_u64, 0_u64);
        let mut long = (0_u64, 0_u64);
        let mut last = None;
        for observation in &skill.observations {
            if !observation_ids.insert(observation.id.as_str()) {
                return Err(action_error(
                    Some(SKILL_HEALTH_AGGREGATE_ACTION_ID),
                    ActionErrorCode::DuplicateIdentifier,
                    format!(
                        "skill `{}` repeats observation `{}`",
                        skill.skill_id, observation.id
                    ),
                ));
            }
            observe(observation, input, &mut recent, &mut long)?;
            last = Some(last.map_or(observation.occurred_at_ms, |value: u64| {
                value.max(observation.occurred_at_ms)
            }));
        }
        portfolio_recent.0 = validation::checked_add(
            SKILL_HEALTH_AGGREGATE_ACTION_ID,
            "portfolio recent run count",
            portfolio_recent.0,
            recent.0,
        )?;
        portfolio_recent.1 = validation::checked_add(
            SKILL_HEALTH_AGGREGATE_ACTION_ID,
            "portfolio recent success count",
            portfolio_recent.1,
            recent.1,
        )?;
        portfolio_long.0 = validation::checked_add(
            SKILL_HEALTH_AGGREGATE_ACTION_ID,
            "portfolio long run count",
            portfolio_long.0,
            long.0,
        )?;
        portfolio_long.1 = validation::checked_add(
            SKILL_HEALTH_AGGREGATE_ACTION_ID,
            "portfolio long success count",
            portfolio_long.1,
            long.1,
        )?;
        portfolio_last = portfolio_last.max(last);
        let recent = window_rate(recent.0, recent.1)?;
        let long = window_rate(long.0, long.1)?;
        rows.push(SkillHealthRow {
            skill_id: skill.skill_id.clone(),
            trend: classify_trend(&recent, &long, input.trend_threshold_micros),
            recent,
            long,
            pending_amendments: skill.pending_amendments,
            last_run_at_ms: last,
        });
    }
    rows.sort_by(|left, right| left.skill_id.cmp(&right.skill_id));
    Ok(SkillHealthOutput {
        scale_micros: MICRO_SCALE,
        pending_amendments: pending,
        recent: window_rate(portfolio_recent.0, portfolio_recent.1)?,
        long: window_rate(portfolio_long.0, portfolio_long.1)?,
        last_run_at_ms: portfolio_last,
        skills: rows,
    })
}

#[cfg(test)]
#[path = "../../tests/internal/actions/skill_health.rs"]
mod tests;
