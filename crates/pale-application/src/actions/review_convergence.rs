//! Stable finding deduplication and fail-closed review convergence.
//!
//! Findings group by file plus normalized evidence, or by file, title, and line
//! when evidence is empty. Required verifier failures always block acceptance.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    ActionError, ActionErrorCode, ConvergedFinding, FindingDisposition, ReviewConvergenceInput,
    ReviewConvergenceOutput, ReviewFinding, ReviewRecommendation, ReviewSeverity,
    REVIEW_BLOCKING_CONFIDENCE_MICROS, REVIEW_CONVERGENCE_REDUCE_ACTION_ID,
};

use super::{error::action_error, validation, MICRO_SCALE};

// Evidence is bounded per finding and in aggregate so many short findings cannot
// turn normalization and deduplication into unbounded retained text.
/// Maximum raw findings normalized in one convergence call.
const MAX_FINDINGS: usize = 10_000;
/// Maximum verifier dimensions joined into the final verdict.
const MAX_DIMENSIONS: usize = 128;
/// Maximum evidence bytes retained by one finding.
const MAX_EVIDENCE_BYTES: usize = 8_192;
/// Aggregate evidence ceiling across every supplied finding.
const MAX_TOTAL_EVIDENCE_BYTES: usize = 4_194_304;

fn invalid(message: impl Into<String>) -> ActionError {
    action_error(
        Some(REVIEW_CONVERGENCE_REDUCE_ACTION_ID),
        ActionErrorCode::MalformedInput,
        message,
    )
}

fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

fn severity_rank(severity: ReviewSeverity) -> u8 {
    match severity {
        ReviewSeverity::Critical => 4,
        ReviewSeverity::High => 3,
        ReviewSeverity::Medium => 2,
        ReviewSeverity::Low => 1,
    }
}

fn representative_order(left: &ReviewFinding, right: &ReviewFinding) -> Ordering {
    severity_rank(left.severity)
        .cmp(&severity_rank(right.severity))
        .then_with(|| {
            left.verifier_confidence_micros
                .cmp(&right.verifier_confidence_micros)
        })
        .then_with(|| right.id.cmp(&left.id))
}

/// Determine whether independent review evidence satisfies the declared threshold.
pub(super) fn execute(
    input: &ReviewConvergenceInput,
) -> Result<ReviewConvergenceOutput, ActionError> {
    if input.findings.len() > MAX_FINDINGS {
        return Err(invalid(format!(
            "review convergence exceeds the {MAX_FINDINGS}-finding bound"
        )));
    }
    if input.verifier_dimensions.is_empty() || input.verifier_dimensions.len() > MAX_DIMENSIONS {
        return Err(invalid(format!(
            "review convergence requires 1..={MAX_DIMENSIONS} verifier dimensions"
        )));
    }
    let mut dimension_ids = BTreeSet::new();
    let mut failed_required_dimensions = Vec::new();
    let mut has_required_dimension = false;
    for dimension in &input.verifier_dimensions {
        validation::validate_token(
            REVIEW_CONVERGENCE_REDUCE_ACTION_ID,
            "verifier dimension id",
            &dimension.id,
            256,
        )?;
        if !dimension_ids.insert(dimension.id.as_str()) {
            return Err(action_error(
                Some(REVIEW_CONVERGENCE_REDUCE_ACTION_ID),
                ActionErrorCode::DuplicateIdentifier,
                format!("duplicate verifier dimension `{}`", dimension.id),
            ));
        }
        has_required_dimension |= dimension.required;
        if dimension.required && !dimension.passed {
            failed_required_dimensions.push(dimension.id.clone());
        }
    }
    if !has_required_dimension {
        return Err(invalid(
            "review convergence requires at least one required verifier dimension",
        ));
    }
    failed_required_dimensions.sort();

    let mut finding_ids = BTreeSet::new();
    let mut total_evidence_bytes = 0_usize;
    let mut groups = BTreeMap::<String, Vec<&ReviewFinding>>::new();
    for finding in &input.findings {
        validation::validate_token(
            REVIEW_CONVERGENCE_REDUCE_ACTION_ID,
            "finding id",
            &finding.id,
            256,
        )?;
        validation::validate_token(
            REVIEW_CONVERGENCE_REDUCE_ACTION_ID,
            "finding file",
            &finding.file,
            4_096,
        )?;
        validation::validate_token(
            REVIEW_CONVERGENCE_REDUCE_ACTION_ID,
            "finding title",
            &finding.title,
            512,
        )?;
        if finding.evidence.len() > MAX_EVIDENCE_BYTES
            || finding
                .evidence
                .chars()
                .any(|character| character.is_control() && !character.is_whitespace())
        {
            return Err(invalid(format!(
                "finding `{}` has invalid or oversized evidence",
                finding.id
            )));
        }
        if finding.verifier_confidence_micros > MICRO_SCALE {
            return Err(invalid(format!(
                "finding `{}` confidence exceeds {MICRO_SCALE}",
                finding.id
            )));
        }
        if finding.line == Some(0) {
            return Err(invalid(format!(
                "finding `{}` line must be one-based",
                finding.id
            )));
        }
        if !finding_ids.insert(finding.id.as_str()) {
            return Err(action_error(
                Some(REVIEW_CONVERGENCE_REDUCE_ACTION_ID),
                ActionErrorCode::DuplicateIdentifier,
                format!("duplicate review finding `{}`", finding.id),
            ));
        }
        total_evidence_bytes = total_evidence_bytes
            .checked_add(finding.evidence.len())
            .ok_or_else(|| invalid("finding evidence byte count overflowed the platform size"))?;
        if total_evidence_bytes > MAX_TOTAL_EVIDENCE_BYTES {
            return Err(invalid(format!(
                "finding evidence exceeds the {MAX_TOTAL_EVIDENCE_BYTES}-byte total bound"
            )));
        }
        let normalized_evidence = normalize(&finding.evidence);
        let key = if normalized_evidence.is_empty() {
            let line = finding.line.ok_or_else(|| {
                invalid(format!(
                    "finding `{}` needs a line when evidence is empty",
                    finding.id
                ))
            })?;
            format!(
                "{}\0title:{}\0line:{line}",
                finding.file,
                normalize(&finding.title)
            )
        } else {
            format!("{}\0evidence:{normalized_evidence}", finding.file)
        };
        groups.entry(key).or_default().push(finding);
    }

    let mut findings = Vec::with_capacity(groups.len());
    for group in groups.values() {
        let representative = group
            .iter()
            .copied()
            .max_by(|left, right| representative_order(left, right))
            .ok_or_else(|| invalid("empty finding group escaped validation"))?;
        let severity = group
            .iter()
            .map(|finding| finding.severity)
            .max_by_key(|severity| severity_rank(*severity))
            .ok_or_else(|| invalid("empty finding group escaped validation"))?;
        let verifier_confidence_micros = group
            .iter()
            .map(|finding| finding.verifier_confidence_micros)
            .max()
            .ok_or_else(|| invalid("empty finding group escaped validation"))?;
        let mut source_ids = group
            .iter()
            .map(|finding| finding.id.clone())
            .collect::<Vec<_>>();
        source_ids.sort();
        let disposition = if verifier_confidence_micros >= REVIEW_BLOCKING_CONFIDENCE_MICROS {
            FindingDisposition::Blocking
        } else {
            FindingDisposition::Advisory
        };
        findings.push(ConvergedFinding {
            canonical_id: representative.id.clone(),
            source_ids,
            file: representative.file.clone(),
            line: representative.line,
            title: representative.title.clone(),
            evidence: representative.evidence.clone(),
            severity,
            verifier_confidence_micros,
            disposition,
        });
    }
    findings.sort_by(|left, right| {
        let left_blocking = left.disposition == FindingDisposition::Blocking;
        let right_blocking = right.disposition == FindingDisposition::Blocking;
        right_blocking
            .cmp(&left_blocking)
            .then_with(|| severity_rank(right.severity).cmp(&severity_rank(left.severity)))
            .then_with(|| left.file.cmp(&right.file))
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.canonical_id.cmp(&right.canonical_id))
    });

    let required_dimensions_passed = failed_required_dimensions.is_empty();
    let recommendation = if !required_dimensions_passed
        || findings
            .iter()
            .any(|finding| finding.disposition == FindingDisposition::Blocking)
    {
        ReviewRecommendation::Block
    } else {
        ReviewRecommendation::Accept
    };
    Ok(ReviewConvergenceOutput {
        required_dimensions_passed,
        failed_required_dimensions,
        findings,
        recommendation,
    })
}

#[cfg(test)]
#[path = "../../tests/internal/actions/review_convergence.rs"]
mod tests;
