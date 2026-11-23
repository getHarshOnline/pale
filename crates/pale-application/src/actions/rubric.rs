//! Checked rubric normalization with stable failure ranking.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    ActionError, ActionErrorCode, RubricCategoryScore, RubricEvaluationInput,
    RubricEvaluationOutput, RubricFailure, RUBRIC_EVALUATION_ACTION_ID,
};

use super::{error::action_error, math, validation, MICRO_SCALE};

// Bound category/check joins and the diagnostic failure slice independently.
/// Maximum categories in one normalized rubric.
const MAX_CATEGORIES: usize = 128;
/// Maximum check results evaluated across the rubric.
const MAX_CHECKS: usize = 4_096;
/// Maximum failing checks retained in the diagnostic result.
const MAX_FAILURES: usize = 64;

fn invalid(message: impl Into<String>) -> ActionError {
    action_error(
        Some(RUBRIC_EVALUATION_ACTION_ID),
        ActionErrorCode::MalformedInput,
        message,
    )
}

/// Score a bounded rubric while retaining unmet criteria in the result.
pub(super) fn execute(
    input: &RubricEvaluationInput,
) -> Result<RubricEvaluationOutput, ActionError> {
    if input.categories.is_empty() || input.categories.len() > MAX_CATEGORIES {
        return Err(invalid(format!(
            "rubric requires 1..={MAX_CATEGORIES} categories"
        )));
    }
    if input.checks.is_empty() || input.checks.len() > MAX_CHECKS {
        return Err(invalid(format!(
            "rubric requires 1..={MAX_CHECKS} check results"
        )));
    }
    let top_failure_limit = usize::try_from(input.top_failure_limit)
        .map_err(|_| invalid("top_failure_limit does not fit the platform size"))?;
    if top_failure_limit > MAX_FAILURES {
        return Err(invalid(format!(
            "top_failure_limit exceeds the {MAX_FAILURES}-row bound"
        )));
    }

    let mut categories = BTreeMap::new();
    let mut total_weight = 0_u64;
    for category in &input.categories {
        validation::validate_token(
            RUBRIC_EVALUATION_ACTION_ID,
            "category id",
            &category.id,
            256,
        )?;
        if category.weight_micros == 0 || category.weight_micros > MICRO_SCALE {
            return Err(invalid(format!(
                "category `{}` weight must be in 1..={MICRO_SCALE}",
                category.id
            )));
        }
        if categories
            .insert(category.id.as_str(), (category.weight_micros, 0_u64, 0_u64))
            .is_some()
        {
            return Err(action_error(
                Some(RUBRIC_EVALUATION_ACTION_ID),
                ActionErrorCode::DuplicateIdentifier,
                format!("duplicate rubric category `{}`", category.id),
            ));
        }
        total_weight = validation::checked_add(
            RUBRIC_EVALUATION_ACTION_ID,
            "total rubric weight",
            total_weight,
            u64::from(category.weight_micros),
        )?;
    }

    let mut check_ids = BTreeSet::new();
    let mut failures = Vec::new();
    for check in &input.checks {
        validation::validate_token(RUBRIC_EVALUATION_ACTION_ID, "check id", &check.id, 256)?;
        if !check_ids.insert(check.id.as_str()) {
            return Err(action_error(
                Some(RUBRIC_EVALUATION_ACTION_ID),
                ActionErrorCode::DuplicateIdentifier,
                format!("duplicate rubric check `{}`", check.id),
            ));
        }
        validation::validate_safe_integer(
            RUBRIC_EVALUATION_ACTION_ID,
            "earned points",
            check.earned_points,
        )?;
        validation::validate_safe_integer(
            RUBRIC_EVALUATION_ACTION_ID,
            "available points",
            check.available_points,
        )?;
        if check.available_points == 0 || check.earned_points > check.available_points {
            return Err(invalid(format!(
                "check `{}` requires 0 <= earned_points <= non-zero available_points",
                check.id
            )));
        }
        let category = categories
            .get_mut(check.category_id.as_str())
            .ok_or_else(|| {
                invalid(format!(
                    "check `{}` references unknown category `{}`",
                    check.id, check.category_id
                ))
            })?;
        category.1 = validation::checked_add(
            RUBRIC_EVALUATION_ACTION_ID,
            "category earned points",
            category.1,
            check.earned_points,
        )?;
        category.2 = validation::checked_add(
            RUBRIC_EVALUATION_ACTION_ID,
            "category available points",
            category.2,
            check.available_points,
        )?;

        let lost = check.available_points - check.earned_points;
        if lost > 0 {
            let loss_micros =
                math::ratio_micros(RUBRIC_EVALUATION_ACTION_ID, lost, check.available_points)?;
            let weighted_loss = u64::from(loss_micros)
                .checked_mul(u64::from(category.0))
                .ok_or_else(|| {
                    action_error(
                        Some(RUBRIC_EVALUATION_ACTION_ID),
                        ActionErrorCode::NumericOverflow,
                        "weighted rubric loss overflowed u64",
                    )
                })?;
            validation::validate_safe_integer(
                RUBRIC_EVALUATION_ACTION_ID,
                "weighted rubric loss",
                weighted_loss,
            )?;
            failures.push(RubricFailure {
                check_id: check.id.clone(),
                category_id: check.category_id.clone(),
                lost_points: lost,
                weighted_loss,
            });
        }
    }

    let mut weighted_score = 0_u128;
    let mut category_scores = Vec::with_capacity(categories.len());
    for (category_id, (weight, earned, available)) in categories {
        if available == 0 {
            return Err(invalid(format!(
                "category `{category_id}` has no check results"
            )));
        }
        let score = math::ratio_micros(RUBRIC_EVALUATION_ACTION_ID, earned, available)?;
        weighted_score = weighted_score
            .checked_add(u128::from(score) * u128::from(weight))
            .ok_or_else(|| {
                action_error(
                    Some(RUBRIC_EVALUATION_ACTION_ID),
                    ActionErrorCode::NumericOverflow,
                    "weighted rubric score overflowed",
                )
            })?;
        category_scores.push(RubricCategoryScore {
            category_id: category_id.to_string(),
            earned_points: earned,
            available_points: available,
            score_micros: score,
        });
    }
    let overall = weighted_score
        .checked_div(u128::from(total_weight))
        .ok_or_else(|| invalid("rubric total weight cannot be zero"))?;
    let overall_score_micros = u32::try_from(overall).map_err(|_| {
        action_error(
            Some(RUBRIC_EVALUATION_ACTION_ID),
            ActionErrorCode::NumericOverflow,
            "normalized rubric score does not fit in u32",
        )
    })?;

    failures.sort_by(|left, right| {
        right
            .weighted_loss
            .cmp(&left.weighted_loss)
            .then_with(|| left.check_id.cmp(&right.check_id))
    });
    failures.truncate(top_failure_limit);
    Ok(RubricEvaluationOutput {
        scale_micros: MICRO_SCALE,
        total_weight_micros: total_weight,
        overall_score_micros,
        categories: category_scores,
        top_failures: failures,
    })
}

#[cfg(test)]
#[path = "../../tests/internal/actions/rubric.rs"]
mod tests;
