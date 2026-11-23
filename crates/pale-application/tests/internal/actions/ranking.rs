//! Fixed-point ranking proofs for completeness, bounds, and stable identity tie-breaking.

use pale_core::records::{RankingCandidate, RankingScore, RankingWeight};

use super::*;

fn ranking_input() -> WeightedRankingInput {
    WeightedRankingInput {
        weights: vec![
            RankingWeight {
                id: "quality".into(),
                weight_micros: 600_000,
            },
            RankingWeight {
                id: "speed".into(),
                weight_micros: 400_000,
            },
        ],
        candidates: vec![
            RankingCandidate {
                id: "beta".into(),
                scores: vec![
                    RankingScore {
                        weight_id: "quality".into(),
                        value_micros: 700_000,
                    },
                    RankingScore {
                        weight_id: "speed".into(),
                        value_micros: 800_000,
                    },
                ],
            },
            RankingCandidate {
                id: "alpha".into(),
                scores: vec![
                    RankingScore {
                        weight_id: "speed".into(),
                        value_micros: 800_000,
                    },
                    RankingScore {
                        weight_id: "quality".into(),
                        value_micros: 700_000,
                    },
                ],
            },
        ],
    }
}

#[test]
fn fixed_point_ranking_uses_identity_as_the_stable_tie_breaker() {
    let output = execute(&ranking_input()).expect("valid ranking");
    assert_eq!(output.total_weight_micros, 1_000_000);
    assert_eq!(
        output
            .ranking
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha", "beta"]
    );
    assert!(output
        .ranking
        .iter()
        .all(|item| item.normalized_score_micros == 740_000));
    assert!(output
        .ranking
        .iter()
        .all(|item| item.weighted_sum == 740_000_000_000));
}

#[test]
fn duplicate_incomplete_and_unbounded_identities_fail_closed() {
    let mut duplicate_weight = ranking_input();
    duplicate_weight.weights[1].id = "quality".into();
    assert_eq!(
        execute(&duplicate_weight)
            .expect_err("duplicate weight")
            .code,
        ActionErrorCode::DuplicateIdentifier
    );

    let mut duplicate_candidate = ranking_input();
    duplicate_candidate.candidates[1].id = "beta".into();
    assert_eq!(
        execute(&duplicate_candidate)
            .expect_err("duplicate candidate")
            .code,
        ActionErrorCode::DuplicateIdentifier
    );

    let mut incomplete = ranking_input();
    incomplete.candidates[0].scores.pop();
    assert_eq!(
        execute(&incomplete).expect_err("missing score").code,
        ActionErrorCode::InvalidRanking
    );

    let mut duplicate_score = ranking_input();
    duplicate_score.candidates[0].scores[1].weight_id = "quality".into();
    assert_eq!(
        execute(&duplicate_score).expect_err("duplicate score").code,
        ActionErrorCode::DuplicateIdentifier
    );

    let mut oversized = ranking_input();
    oversized.weights[0].id = "x".repeat(257);
    assert_eq!(
        execute(&oversized).expect_err("oversized identity").code,
        ActionErrorCode::InvalidRanking
    );
}
