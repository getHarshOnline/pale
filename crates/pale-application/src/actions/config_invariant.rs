//! Bounded literal checks over caller-supplied configuration text.
//!
//! Matching is deliberately lexical: comments and quoted examples count. The
//! caller remains responsible for format-aware interpretation.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    ActionError, ActionErrorCode, ConfigInvariantEvidence, ConfigInvariantExpectation,
    ConfigInvariantResult, ConfigInvariantScanInput, ConfigInvariantScanOutput,
    CONFIG_INVARIANT_SCAN_ACTION_ID,
};

use super::{error::action_error, validation};

// Bound retained text, lexical comparisons, and returned evidence independently;
// the product check ceiling prevents many small records from evading byte limits.
/// Maximum configuration records scanned in one invocation.
const MAX_RECORDS: usize = 128;
/// Maximum independent invariants evaluated per invocation.
const MAX_INVARIANTS: usize = 256;
/// Maximum retained bytes for one configuration record.
const MAX_RECORD_BYTES: usize = 262_144;
/// Aggregate ceiling across every supplied configuration record.
const MAX_TOTAL_TEXT_BYTES: usize = 1_048_576;
/// Maximum bytes in one required or forbidden literal.
const MAX_LITERAL_BYTES: usize = 512;
/// Maximum record identities selected by one invariant scope.
const MAX_SCOPE: usize = 128;
/// Maximum evidence rows returned for one invariant.
const MAX_EVIDENCE_ROWS: usize = 64;
/// Maximum record-by-invariant comparisons admitted per invocation.
const MAX_CHECKS: usize = 8_192;

fn invalid(message: impl Into<String>) -> ActionError {
    action_error(
        Some(CONFIG_INVARIANT_SCAN_ACTION_ID),
        ActionErrorCode::MalformedInput,
        message,
    )
}

fn validate_config_text(text: &str) -> Result<(), ActionError> {
    if text.len() > MAX_RECORD_BYTES {
        return Err(invalid(format!(
            "configuration record exceeds the {MAX_RECORD_BYTES}-byte bound"
        )));
    }
    if text
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(invalid(
            "configuration text contains unsupported control bytes",
        ));
    }
    Ok(())
}

/// Compare one configuration value against its declared invariant.
pub(super) fn execute(
    input: &ConfigInvariantScanInput,
) -> Result<ConfigInvariantScanOutput, ActionError> {
    if input.records.is_empty() || input.records.len() > MAX_RECORDS {
        return Err(invalid(format!(
            "configuration scan requires 1..={MAX_RECORDS} records"
        )));
    }
    if input.invariants.is_empty() || input.invariants.len() > MAX_INVARIANTS {
        return Err(invalid(format!(
            "configuration scan requires 1..={MAX_INVARIANTS} invariants"
        )));
    }
    let mut records = BTreeMap::new();
    let mut total_bytes = 0_usize;
    for record in &input.records {
        validation::validate_token(
            CONFIG_INVARIANT_SCAN_ACTION_ID,
            "record id",
            &record.id,
            256,
        )?;
        validate_config_text(&record.text)?;
        total_bytes = total_bytes
            .checked_add(record.text.len())
            .ok_or_else(|| invalid("configuration text byte count overflowed the platform size"))?;
        if total_bytes > MAX_TOTAL_TEXT_BYTES {
            return Err(invalid(format!(
                "configuration text exceeds the {MAX_TOTAL_TEXT_BYTES}-byte total bound"
            )));
        }
        if records.insert(record.id.as_str(), record).is_some() {
            return Err(action_error(
                Some(CONFIG_INVARIANT_SCAN_ACTION_ID),
                ActionErrorCode::DuplicateIdentifier,
                format!("duplicate configuration record id `{}`", record.id),
            ));
        }
    }

    let mut invariant_ids = BTreeSet::new();
    let mut checked_records = 0_usize;
    let mut results = Vec::with_capacity(input.invariants.len());
    for invariant in &input.invariants {
        validation::validate_token(
            CONFIG_INVARIANT_SCAN_ACTION_ID,
            "invariant id",
            &invariant.id,
            256,
        )?;
        if !invariant_ids.insert(invariant.id.as_str()) {
            return Err(action_error(
                Some(CONFIG_INVARIANT_SCAN_ACTION_ID),
                ActionErrorCode::DuplicateIdentifier,
                format!("duplicate invariant id `{}`", invariant.id),
            ));
        }
        if invariant.literal.is_empty()
            || invariant.literal.len() > MAX_LITERAL_BYTES
            || invariant.literal.chars().any(char::is_control)
        {
            return Err(invalid(format!(
                "invariant `{}` literal must contain 1..={MAX_LITERAL_BYTES} non-control bytes",
                invariant.id
            )));
        }
        if invariant.record_ids.len() > MAX_SCOPE {
            return Err(invalid(format!(
                "invariant `{}` exceeds the {MAX_SCOPE}-record scope bound",
                invariant.id
            )));
        }

        let selected = if invariant.record_ids.is_empty() {
            records.keys().copied().collect::<Vec<_>>()
        } else {
            let mut scope = BTreeSet::new();
            for record_id in &invariant.record_ids {
                validation::validate_token(
                    CONFIG_INVARIANT_SCAN_ACTION_ID,
                    "scoped record id",
                    record_id,
                    256,
                )?;
                if !records.contains_key(record_id.as_str()) {
                    return Err(invalid(format!(
                        "invariant `{}` references unknown record `{record_id}`",
                        invariant.id
                    )));
                }
                if !scope.insert(record_id.as_str()) {
                    return Err(action_error(
                        Some(CONFIG_INVARIANT_SCAN_ACTION_ID),
                        ActionErrorCode::DuplicateIdentifier,
                        format!(
                            "invariant `{}` repeats scoped record `{record_id}`",
                            invariant.id
                        ),
                    ));
                }
            }
            scope.into_iter().collect()
        };
        checked_records = checked_records
            .checked_add(selected.len())
            .filter(|checks| *checks <= MAX_CHECKS)
            .ok_or_else(|| {
                invalid(format!(
                    "configuration scan exceeds the {MAX_CHECKS}-selected-record work bound"
                ))
            })?;

        let mut match_count = 0_u64;
        let mut evidence_rows = 0_u64;
        let mut evidence = Vec::new();
        for record_id in selected {
            let record = records.get(record_id).ok_or_else(|| {
                invalid("validated configuration scope disappeared during reduction")
            })?;
            for (line_index, line) in record.text.lines().enumerate() {
                let occurrences = line.matches(&invariant.literal).count();
                if occurrences == 0 {
                    continue;
                }
                let occurrences = u64::try_from(occurrences)
                    .map_err(|_| invalid("literal occurrence count does not fit in u64"))?;
                match_count = validation::checked_add(
                    CONFIG_INVARIANT_SCAN_ACTION_ID,
                    "literal match count",
                    match_count,
                    occurrences,
                )?;
                evidence_rows = validation::checked_add(
                    CONFIG_INVARIANT_SCAN_ACTION_ID,
                    "literal evidence row count",
                    evidence_rows,
                    1,
                )?;
                if evidence.len() < MAX_EVIDENCE_ROWS {
                    evidence.push(ConfigInvariantEvidence {
                        record_id: record_id.to_string(),
                        line: u64::try_from(line_index + 1)
                            .map_err(|_| invalid("line number does not fit in u64"))?,
                        occurrences,
                    });
                }
            }
        }
        let passed = match invariant.expectation {
            ConfigInvariantExpectation::RequiredLiteral => match_count > 0,
            ConfigInvariantExpectation::ForbiddenLiteral => match_count == 0,
        };
        results.push(ConfigInvariantResult {
            invariant_id: invariant.id.clone(),
            passed,
            match_count,
            evidence_truncated: evidence_rows
                > u64::try_from(evidence.len())
                    .map_err(|_| invalid("evidence count does not fit in u64"))?,
            evidence,
        });
    }
    results.sort_by(|left, right| left.invariant_id.cmp(&right.invariant_id));
    Ok(ConfigInvariantScanOutput {
        passed: results.iter().all(|result| result.passed),
        results,
    })
}

#[cfg(test)]
#[path = "../../tests/internal/actions/config_invariant.rs"]
mod tests;
