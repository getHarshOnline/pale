//! Review-convergence proofs for finding aggregation and mandatory-dimension failure.

use pale_core::records::VerifierDimension;

use super::*;

fn finding(id: &str, severity: ReviewSeverity, confidence: u32) -> ReviewFinding {
    ReviewFinding {
        id: id.into(),
        file: "src/lib.rs".into(),
        line: Some(10),
        title: "Unchecked input".into(),
        evidence: "  User INPUT reaches sink  ".into(),
        severity,
        verifier_confidence_micros: confidence,
    }
}

#[test]
fn dedupe_aggregates_strictest_severity_and_max_confidence_independently() {
    let mut duplicate = finding("b", ReviewSeverity::Critical, 700_000);
    duplicate.evidence = "user input   reaches SINK".into();
    let input = ReviewConvergenceInput {
        findings: vec![finding("a", ReviewSeverity::Medium, 950_000), duplicate],
        verifier_dimensions: vec![VerifierDimension {
            id: "security".into(),
            required: true,
            passed: true,
        }],
    };
    let output = execute(&input).expect("valid review evidence");
    assert_eq!(output.findings.len(), 1);
    assert_eq!(output.findings[0].canonical_id, "b");
    assert_eq!(output.findings[0].source_ids, vec!["a", "b"]);
    assert_eq!(output.findings[0].severity, ReviewSeverity::Critical);
    assert_eq!(output.findings[0].verifier_confidence_micros, 950_000);
    assert_eq!(output.findings[0].disposition, FindingDisposition::Blocking);
    assert_eq!(output.recommendation, ReviewRecommendation::Block);

    let threshold = execute(&ReviewConvergenceInput {
        findings: vec![
            finding("below", ReviewSeverity::Low, 799_999),
            ReviewFinding {
                id: "at".into(),
                evidence: "different evidence".into(),
                verifier_confidence_micros: REVIEW_BLOCKING_CONFIDENCE_MICROS,
                ..finding("template", ReviewSeverity::Low, 0)
            },
        ],
        verifier_dimensions: vec![VerifierDimension {
            id: "quality".into(),
            required: true,
            passed: true,
        }],
    })
    .expect("valid threshold evidence");
    assert_eq!(threshold.findings[0].canonical_id, "at");
    assert_eq!(
        threshold.findings[0].disposition,
        FindingDisposition::Blocking
    );
    assert_eq!(threshold.findings[1].canonical_id, "below");
    assert_eq!(
        threshold.findings[1].disposition,
        FindingDisposition::Advisory
    );
}

#[test]
fn failed_required_dimension_blocks_even_without_findings() {
    let output = execute(&ReviewConvergenceInput {
        findings: vec![],
        verifier_dimensions: vec![VerifierDimension {
            id: "tests".into(),
            required: true,
            passed: false,
        }],
    })
    .expect("valid failed verifier");
    assert!(!output.required_dimensions_passed);
    assert_eq!(output.recommendation, ReviewRecommendation::Block);
}
