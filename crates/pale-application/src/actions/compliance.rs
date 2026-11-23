//! Deterministic grading of already-classified trace events.
//!
//! [`ComplianceTemporalRule::Before`] compares earliest witnesses,
//! [`ComplianceTemporalRule::After`] compares latest witnesses, and
//! [`ComplianceTemporalRule::Ordered`] requires one strictly increasing witness
//! sequence.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    ActionError, ActionErrorCode, ComplianceCheckResult, CompliancePromotion, ComplianceStep,
    ComplianceTemporalRule, ComplianceTraceGradeInput, ComplianceTraceGradeOutput,
    COMPLIANCE_TRACE_GRADE_ACTION_ID,
};

use super::{error::action_error, math, validation, MICRO_SCALE};

// Bound both retained trace evidence and the worst-case ordered-rule walk over
// caller-controlled event positions.
/// Maximum trace events retained for one pure grading call.
const MAX_EVENTS: usize = 10_000;
/// Maximum distinct step definitions addressable by temporal rules.
const MAX_STEPS: usize = 256;
/// Maximum temporal rules evaluated in one call.
const MAX_RULES: usize = 512;
/// Maximum witnesses traversed by one ordered rule.
const MAX_ORDERED_STEPS: usize = 256;

fn invalid(message: impl Into<String>) -> ActionError {
    action_error(
        Some(COMPLIANCE_TRACE_GRADE_ACTION_ID),
        ActionErrorCode::MalformedInput,
        message,
    )
}

fn rule_id(rule: &ComplianceTemporalRule) -> &str {
    match rule {
        ComplianceTemporalRule::Before { id, .. }
        | ComplianceTemporalRule::After { id, .. }
        | ComplianceTemporalRule::Ordered { id, .. } => id,
    }
}

fn step_positions<'a>(
    step_id: &str,
    steps: &BTreeMap<&'a str, &'a ComplianceStep>,
    events: &'a BTreeMap<&str, Vec<u64>>,
) -> Result<&'a [u64], ActionError> {
    let step = steps
        .get(step_id)
        .ok_or_else(|| invalid(format!("temporal rule references unknown step `{step_id}`")))?;
    Ok(events
        .get(step.event_class.as_str())
        .map(Vec::as_slice)
        .unwrap_or_default())
}

fn relation_passes(
    rule: &ComplianceTemporalRule,
    steps: &BTreeMap<&str, &ComplianceStep>,
    events: &BTreeMap<&str, Vec<u64>>,
) -> Result<bool, ActionError> {
    match rule {
        ComplianceTemporalRule::Before {
            first_step_id,
            second_step_id,
            ..
        } => {
            let first = step_positions(first_step_id, steps, events)?;
            let second = step_positions(second_step_id, steps, events)?;
            Ok(first
                .first()
                .zip(second.first())
                .is_some_and(|(left, right)| left < right))
        }
        ComplianceTemporalRule::After {
            first_step_id,
            second_step_id,
            ..
        } => {
            let first = step_positions(first_step_id, steps, events)?;
            let second = step_positions(second_step_id, steps, events)?;
            Ok(first
                .last()
                .zip(second.last())
                .is_some_and(|(left, right)| left > right))
        }
        ComplianceTemporalRule::Ordered { step_ids, .. } => {
            if step_ids.len() < 2 || step_ids.len() > MAX_ORDERED_STEPS {
                return Err(invalid(format!(
                    "ordered rule requires 2..={MAX_ORDERED_STEPS} step references"
                )));
            }
            let mut cursor = None;
            for step_id in step_ids {
                let positions = step_positions(step_id, steps, events)?;
                let next = match cursor {
                    Some(previous) => {
                        let next_index = positions.partition_point(|value| *value <= previous);
                        positions.get(next_index).copied()
                    }
                    None => positions.first().copied(),
                };
                let Some(next) = next else {
                    return Ok(false);
                };
                cursor = Some(next);
            }
            Ok(true)
        }
    }
}

/// Evaluate every declared compliance item without treating an empty set as evidence.
pub(super) fn execute(
    input: &ComplianceTraceGradeInput,
) -> Result<ComplianceTraceGradeOutput, ActionError> {
    if input.events.len() > MAX_EVENTS {
        return Err(invalid(format!(
            "compliance trace exceeds the {MAX_EVENTS}-event bound"
        )));
    }
    if input.required_steps.is_empty() || input.required_steps.len() > MAX_STEPS {
        return Err(invalid(format!(
            "compliance grading requires 1..={MAX_STEPS} steps"
        )));
    }
    if input.temporal_rules.len() > MAX_RULES {
        return Err(invalid(format!(
            "compliance grading exceeds the {MAX_RULES}-rule bound"
        )));
    }
    if input.promotion_threshold_micros == 0 || input.promotion_threshold_micros > MICRO_SCALE {
        return Err(invalid(format!(
            "promotion threshold must be within 1..={MICRO_SCALE}"
        )));
    }

    let mut event_ids = BTreeSet::new();
    let mut sequences = BTreeSet::new();
    let mut events = BTreeMap::<&str, Vec<u64>>::new();
    for event in &input.events {
        validation::validate_token(COMPLIANCE_TRACE_GRADE_ACTION_ID, "event id", &event.id, 256)?;
        validation::validate_token(
            COMPLIANCE_TRACE_GRADE_ACTION_ID,
            "event class",
            &event.class,
            256,
        )?;
        validation::validate_safe_integer(
            COMPLIANCE_TRACE_GRADE_ACTION_ID,
            "event sequence",
            event.sequence,
        )?;
        if !event_ids.insert(event.id.as_str()) || !sequences.insert(event.sequence) {
            return Err(action_error(
                Some(COMPLIANCE_TRACE_GRADE_ACTION_ID),
                ActionErrorCode::DuplicateIdentifier,
                "classified events require unique ids and sequence values",
            ));
        }
        events
            .entry(event.class.as_str())
            .or_default()
            .push(event.sequence);
    }
    for positions in events.values_mut() {
        positions.sort_unstable();
    }

    let mut steps = BTreeMap::new();
    let mut step_results = Vec::with_capacity(input.required_steps.len());
    for step in &input.required_steps {
        validation::validate_token(COMPLIANCE_TRACE_GRADE_ACTION_ID, "step id", &step.id, 256)?;
        validation::validate_token(
            COMPLIANCE_TRACE_GRADE_ACTION_ID,
            "step event class",
            &step.event_class,
            256,
        )?;
        if step.minimum_occurrences == 0 {
            return Err(invalid(format!(
                "step `{}` minimum_occurrences must be non-zero",
                step.id
            )));
        }
        if steps.insert(step.id.as_str(), step).is_some() {
            return Err(action_error(
                Some(COMPLIANCE_TRACE_GRADE_ACTION_ID),
                ActionErrorCode::DuplicateIdentifier,
                format!("duplicate compliance step `{}`", step.id),
            ));
        }
        let occurrences = events.get(step.event_class.as_str()).map_or(0, Vec::len);
        let occurrences = u64::try_from(occurrences)
            .map_err(|_| invalid("event occurrence count does not fit in u64"))?;
        step_results.push(ComplianceCheckResult {
            id: step.id.clone(),
            passed: occurrences >= u64::from(step.minimum_occurrences),
        });
    }

    let mut rule_ids = BTreeSet::new();
    let mut temporal_results = Vec::with_capacity(input.temporal_rules.len());
    for rule in &input.temporal_rules {
        let id = rule_id(rule);
        validation::validate_token(
            COMPLIANCE_TRACE_GRADE_ACTION_ID,
            "temporal rule id",
            id,
            256,
        )?;
        if !rule_ids.insert(id) {
            return Err(action_error(
                Some(COMPLIANCE_TRACE_GRADE_ACTION_ID),
                ActionErrorCode::DuplicateIdentifier,
                format!("duplicate temporal rule `{id}`"),
            ));
        }
        temporal_results.push(ComplianceCheckResult {
            id: id.to_string(),
            passed: relation_passes(rule, &steps, &events)?,
        });
    }

    step_results.sort_by(|left, right| left.id.cmp(&right.id));
    temporal_results.sort_by(|left, right| left.id.cmp(&right.id));
    let passed_checks = step_results
        .iter()
        .chain(&temporal_results)
        .filter(|result| result.passed)
        .count();
    let total_checks = step_results.len() + temporal_results.len();
    let passed_checks = u64::try_from(passed_checks)
        .map_err(|_| invalid("passed compliance check count does not fit in u64"))?;
    let total_checks = u64::try_from(total_checks)
        .map_err(|_| invalid("total compliance check count does not fit in u64"))?;
    let compliance_rate_micros = math::ratio_micros(
        COMPLIANCE_TRACE_GRADE_ACTION_ID,
        passed_checks,
        total_checks,
    )?;
    let promotion = if compliance_rate_micros >= input.promotion_threshold_micros {
        CompliancePromotion::Eligible
    } else {
        CompliancePromotion::NotEligible
    };
    Ok(ComplianceTraceGradeOutput {
        scale_micros: MICRO_SCALE,
        passed_checks,
        total_checks,
        compliance_rate_micros,
        steps: step_results,
        temporal_rules: temporal_results,
        promotion,
    })
}

#[cfg(test)]
#[path = "../../tests/internal/actions/compliance.rs"]
mod tests;
