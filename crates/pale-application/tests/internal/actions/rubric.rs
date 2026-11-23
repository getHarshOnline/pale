//! Rubric proofs for normalized scoring, deterministic failure order, and closed identities.

use pale_core::records::{RubricCategory, RubricCheckResult};

use super::*;

#[test]
fn rubric_normalizes_categories_and_ranks_failures_stably() {
    let input = RubricEvaluationInput {
        categories: vec![
            RubricCategory {
                id: "safety".into(),
                weight_micros: 600_000,
            },
            RubricCategory {
                id: "clarity".into(),
                weight_micros: 400_000,
            },
        ],
        checks: vec![
            RubricCheckResult {
                id: "b".into(),
                category_id: "safety".into(),
                earned_points: 5,
                available_points: 10,
            },
            RubricCheckResult {
                id: "a".into(),
                category_id: "clarity".into(),
                earned_points: 10,
                available_points: 10,
            },
        ],
        top_failure_limit: 3,
    };
    let output = execute(&input).expect("valid rubric");
    assert_eq!(output.overall_score_micros, 700_000);
    assert_eq!(output.top_failures[0].check_id, "b");
    assert_eq!(output.categories[0].category_id, "clarity");
}

#[test]
fn incomplete_or_unknown_rubrics_fail_closed() {
    let unknown = RubricEvaluationInput {
        categories: vec![RubricCategory {
            id: "known".into(),
            weight_micros: 1,
        }],
        checks: vec![RubricCheckResult {
            id: "check".into(),
            category_id: "missing".into(),
            earned_points: 1,
            available_points: 1,
        }],
        top_failure_limit: 3,
    };
    assert_eq!(
        execute(&unknown).expect_err("unknown category").code,
        ActionErrorCode::MalformedInput
    );

    let incomplete = RubricEvaluationInput {
        categories: vec![
            RubricCategory {
                id: "used".into(),
                weight_micros: 1,
            },
            RubricCategory {
                id: "unused".into(),
                weight_micros: 1,
            },
        ],
        checks: vec![RubricCheckResult {
            id: "check".into(),
            category_id: "used".into(),
            earned_points: 1,
            available_points: 1,
        }],
        top_failure_limit: 3,
    };
    assert_eq!(
        execute(&incomplete)
            .expect_err("category without checks")
            .code,
        ActionErrorCode::MalformedInput
    );
}
