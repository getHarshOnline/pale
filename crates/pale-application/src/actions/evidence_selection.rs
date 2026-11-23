//! Evidence selection preserves caller-declared rule precedence.
//!
//! Rules run in input order; evidence identities within a match are deduplicated
//! and sorted. [`EvidenceSelectionMode::FirstMatch`] stops after the first
//! satisfied rule.

use std::collections::BTreeSet;

use pale_core::records::{
    ActionError, ActionErrorCode, EvidenceMatch, EvidenceRule, EvidenceSelectionInput,
    EvidenceSelectionMode, EvidenceSelectionOutput, EVIDENCE_SELECTION_ACTION_ID,
};

use super::{error::action_error, validation};

// Bound rule-set cardinality and each set operation over untrusted signal IDs.
/// Maximum observed evidence identities admitted per invocation.
const MAX_OBSERVED: usize = 10_000;
/// Maximum precedence-ordered selection rules.
const MAX_RULES: usize = 1_024;
/// Maximum required or excluded signals retained by one rule.
const MAX_SIGNALS_PER_RULE: usize = 1_024;
/// Maximum UTF-8 bytes in an evidence, rule, or signal identity.
const MAX_TOKEN_BYTES: usize = 256;

fn invalid(message: impl Into<String>) -> ActionError {
    action_error(
        Some(EVIDENCE_SELECTION_ACTION_ID),
        ActionErrorCode::MalformedInput,
        message,
    )
}

fn unique<'a>(values: &'a [String], kind: &str) -> Result<BTreeSet<&'a str>, ActionError> {
    let mut set = BTreeSet::new();
    for value in values {
        validation::validate_token(EVIDENCE_SELECTION_ACTION_ID, kind, value, MAX_TOKEN_BYTES)?;
        if !set.insert(value.as_str()) {
            return Err(invalid(format!("duplicate {kind} `{value}`")));
        }
    }
    Ok(set)
}

fn rule_match(
    rule: &EvidenceRule,
    observed: &BTreeSet<&str>,
) -> Result<Option<EvidenceMatch>, ActionError> {
    if rule.require_all.len() > MAX_SIGNALS_PER_RULE
        || rule.require_any.len() > MAX_SIGNALS_PER_RULE
        || rule.exclude.len() > MAX_SIGNALS_PER_RULE
    {
        return Err(invalid(format!(
            "rule `{}` exceeds the {MAX_SIGNALS_PER_RULE}-signal list bound",
            rule.id
        )));
    }
    validation::validate_token(
        EVIDENCE_SELECTION_ACTION_ID,
        "rule id",
        &rule.id,
        MAX_TOKEN_BYTES,
    )?;
    let required_all = unique(&rule.require_all, "require_all signal")?;
    let required_any = unique(&rule.require_any, "require_any signal")?;
    let excluded = unique(&rule.exclude, "exclude signal")?;
    if required_all
        .iter()
        .any(|signal| required_any.contains(signal))
    {
        return Err(invalid(format!(
            "rule `{}` repeats a positive signal across require_all and require_any",
            rule.id
        )));
    }
    if required_all.iter().any(|signal| excluded.contains(signal))
        || required_any.iter().any(|signal| excluded.contains(signal))
    {
        return Err(invalid(format!(
            "rule `{}` both requires and excludes the same signal",
            rule.id
        )));
    }
    if !required_all.iter().all(|signal| observed.contains(signal))
        || (!required_any.is_empty()
            && !required_any.iter().any(|signal| observed.contains(signal)))
        || excluded.iter().any(|signal| observed.contains(signal))
    {
        return Ok(None);
    }

    let evidence = required_all
        .iter()
        .chain(required_any.iter())
        .filter(|signal| observed.contains(*signal))
        .map(|signal| (*signal).to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Ok(Some(EvidenceMatch {
        id: rule.id.clone(),
        evidence,
    }))
}

/// Select bounded evidence while preserving every omission and truncation reason.
pub(super) fn execute(
    input: &EvidenceSelectionInput,
) -> Result<EvidenceSelectionOutput, ActionError> {
    if input.observed.len() > MAX_OBSERVED {
        return Err(invalid(format!(
            "observed evidence exceeds the {MAX_OBSERVED}-signal bound"
        )));
    }
    if input.rules.len() > MAX_RULES {
        return Err(invalid(format!(
            "rule count exceeds the {MAX_RULES}-rule bound"
        )));
    }
    let observed = unique(&input.observed, "observed signal")?;
    let mut rule_ids = BTreeSet::new();
    let mut used = BTreeSet::new();
    let mut selections = Vec::new();
    for rule in &input.rules {
        if !rule_ids.insert(rule.id.as_str()) {
            return Err(invalid(format!("duplicate rule id `{}`", rule.id)));
        }
        let Some(selection) = rule_match(rule, &observed)? else {
            continue;
        };
        used.extend(selection.evidence.iter().cloned());
        selections.push(selection);
        if input.mode == EvidenceSelectionMode::FirstMatch {
            break;
        }
    }

    Ok(EvidenceSelectionOutput {
        matches: selections,
        unmatched_observed: observed
            .into_iter()
            .filter(|signal| !used.contains(*signal))
            .map(str::to_string)
            .collect(),
    })
}

#[cfg(test)]
#[path = "../../tests/internal/actions/evidence_selection.rs"]
mod tests;
