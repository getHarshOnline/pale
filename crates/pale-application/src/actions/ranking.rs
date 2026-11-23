//! Weighted ranking uses checked fixed-point arithmetic and identifier tie-breaking.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    ActionError, ActionErrorCode, RankedCandidate, WeightedRankingInput, WeightedRankingOutput,
    WEIGHTED_RANKING_ACTION_ID,
};

use super::{error::action_error, MAX_CANONICAL_INTEGER, MICRO_SCALE};

// Bound the candidate-by-criterion scoring surface and every retained identifier.
/// Maximum weighted criteria admitted in one ranking matrix.
const MAX_CRITERIA: usize = 1_024;
/// Maximum candidates admitted in one ranking matrix.
const MAX_CANDIDATES: usize = 10_000;
/// Maximum UTF-8 bytes in a candidate or criterion identity.
const MAX_ID_BYTES: usize = 256;

fn invalid(code: ActionErrorCode, message: impl Into<String>) -> ActionError {
    action_error(Some(WEIGHTED_RANKING_ACTION_ID), code, message)
}

fn validate_id(kind: &str, id: &str) -> Result<(), ActionError> {
    if id.is_empty()
        || id.len() > MAX_ID_BYTES
        || id.trim() != id
        || id.chars().any(char::is_control)
    {
        return Err(invalid(
            ActionErrorCode::InvalidRanking,
            format!(
                "{kind} ID must contain 1..={MAX_ID_BYTES} control-free bytes with no surrounding whitespace"
            ),
        ));
    }
    Ok(())
}

/// Rank bounded candidates deterministically with stable tie handling.
pub(super) fn execute(input: &WeightedRankingInput) -> Result<WeightedRankingOutput, ActionError> {
    if input.weights.is_empty() || input.weights.len() > MAX_CRITERIA {
        return Err(invalid(
            ActionErrorCode::InvalidRanking,
            format!("ranking requires between 1 and {MAX_CRITERIA} weights"),
        ));
    }
    if input.candidates.is_empty() || input.candidates.len() > MAX_CANDIDATES {
        return Err(invalid(
            ActionErrorCode::InvalidRanking,
            format!("ranking requires between 1 and {MAX_CANDIDATES} candidates"),
        ));
    }

    let mut weights = BTreeMap::new();
    let mut total_weight = 0_u64;
    for weight in &input.weights {
        validate_id("weight", &weight.id)?;
        if weight.weight_micros == 0 || weight.weight_micros > MICRO_SCALE {
            return Err(invalid(
                ActionErrorCode::InvalidRanking,
                format!("weight `{}` must be between 1 and {MICRO_SCALE}", weight.id),
            ));
        }
        if weights
            .insert(weight.id.as_str(), weight.weight_micros)
            .is_some()
        {
            return Err(invalid(
                ActionErrorCode::DuplicateIdentifier,
                format!("duplicate weight ID `{}`", weight.id),
            ));
        }
        total_weight = total_weight
            .checked_add(u64::from(weight.weight_micros))
            .ok_or_else(|| {
                invalid(
                    ActionErrorCode::NumericOverflow,
                    "total ranking weight overflowed u64",
                )
            })?;
    }

    let mut candidate_ids = BTreeSet::new();
    let mut ranking = Vec::with_capacity(input.candidates.len());
    for candidate in &input.candidates {
        validate_id("candidate", &candidate.id)?;
        if !candidate_ids.insert(candidate.id.as_str()) {
            return Err(invalid(
                ActionErrorCode::DuplicateIdentifier,
                format!("duplicate candidate ID `{}`", candidate.id),
            ));
        }

        let mut values = BTreeMap::new();
        for score in &candidate.scores {
            validate_id("score weight", &score.weight_id)?;
            if !weights.contains_key(score.weight_id.as_str()) {
                return Err(invalid(
                    ActionErrorCode::InvalidRanking,
                    format!(
                        "candidate `{}` supplies unknown weight `{}`",
                        candidate.id, score.weight_id
                    ),
                ));
            }
            if score.value_micros.unsigned_abs() > u64::from(MICRO_SCALE) {
                return Err(invalid(
                    ActionErrorCode::InvalidRanking,
                    format!(
                        "candidate `{}` score `{}` must be within ±{MICRO_SCALE}",
                        candidate.id, score.weight_id
                    ),
                ));
            }
            if values
                .insert(score.weight_id.as_str(), score.value_micros)
                .is_some()
            {
                return Err(invalid(
                    ActionErrorCode::DuplicateIdentifier,
                    format!(
                        "candidate `{}` repeats weight `{}`",
                        candidate.id, score.weight_id
                    ),
                ));
            }
        }

        if values.len() != weights.len() {
            let missing = weights
                .keys()
                .find(|weight_id| !values.contains_key(**weight_id))
                .copied()
                .unwrap_or("unknown");
            return Err(invalid(
                ActionErrorCode::InvalidRanking,
                format!(
                    "candidate `{}` has no score for weight `{missing}`",
                    candidate.id
                ),
            ));
        }

        let weighted_sum = weights
            .iter()
            .try_fold(0_i128, |sum, (weight_id, weight)| {
                let value = values.get(weight_id).copied().ok_or_else(|| {
                    invalid(
                        ActionErrorCode::InternalInvariant,
                        "validated ranking score disappeared",
                    )
                })?;
                let term = i128::from(value)
                    .checked_mul(i128::from(*weight))
                    .ok_or_else(|| {
                        invalid(
                            ActionErrorCode::NumericOverflow,
                            "weighted ranking term overflowed i128",
                        )
                    })?;
                sum.checked_add(term).ok_or_else(|| {
                    invalid(
                        ActionErrorCode::NumericOverflow,
                        "weighted ranking sum overflowed i128",
                    )
                })
            })?;

        let normalized = weighted_sum
            .checked_div(i128::from(total_weight))
            .ok_or_else(|| {
                invalid(
                    ActionErrorCode::NumericOverflow,
                    "weighted ranking normalization failed",
                )
            })?;

        if weighted_sum.unsigned_abs() > u128::from(MAX_CANONICAL_INTEGER) {
            return Err(invalid(
                ActionErrorCode::NumericOverflow,
                "weighted ranking sum exceeds the canonical JSON safe-integer bound",
            ));
        }

        ranking.push(RankedCandidate {
            id: candidate.id.clone(),
            weighted_sum: i64::try_from(weighted_sum).map_err(|_| {
                invalid(
                    ActionErrorCode::NumericOverflow,
                    "weighted ranking sum does not fit in i64",
                )
            })?,
            normalized_score_micros: i64::try_from(normalized).map_err(|_| {
                invalid(
                    ActionErrorCode::NumericOverflow,
                    "normalized ranking score does not fit in i64",
                )
            })?,
        });
    }

    ranking.sort_by(|left, right| {
        right
            .weighted_sum
            .cmp(&left.weighted_sum)
            .then_with(|| left.id.cmp(&right.id))
    });

    Ok(WeightedRankingOutput {
        scale_micros: MICRO_SCALE,
        total_weight_micros: total_weight,
        ranking,
    })
}

#[cfg(test)]
#[path = "../../tests/internal/actions/ranking.rs"]
mod tests;
