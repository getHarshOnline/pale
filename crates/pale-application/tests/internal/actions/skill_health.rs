//! Skill-health proofs for bounded observation windows and temporal consistency.

use pale_core::records::{SkillHealthSeries, SkillRunObservation};

use super::*;

#[test]
fn supplied_windows_produce_rates_trends_and_last_run() {
    let input = SkillHealthInput {
        evaluation_time_ms: 100,
        recent_window_start_ms: 70,
        long_window_start_ms: 10,
        trend_threshold_micros: 100_000,
        skills: vec![SkillHealthSeries {
            skill_id: "review".into(),
            pending_amendments: 2,
            observations: vec![
                SkillRunObservation {
                    id: "old-fail".into(),
                    occurred_at_ms: 20,
                    succeeded: false,
                },
                SkillRunObservation {
                    id: "recent-pass".into(),
                    occurred_at_ms: 90,
                    succeeded: true,
                },
            ],
        }],
    };
    let output = execute(&input).expect("valid health series");
    assert_eq!(output.recent.success_rate_micros, Some(1_000_000));
    assert_eq!(output.long.success_rate_micros, Some(500_000));
    assert_eq!(output.skills[0].trend, SkillHealthTrend::Improving);
    assert_eq!(output.last_run_at_ms, Some(90));
    assert_eq!(output.pending_amendments, 2);
}

#[test]
fn future_observations_and_inverted_windows_fail_closed() {
    let mut input = SkillHealthInput {
        evaluation_time_ms: 100,
        recent_window_start_ms: 70,
        long_window_start_ms: 10,
        trend_threshold_micros: 0,
        skills: vec![SkillHealthSeries {
            skill_id: "review".into(),
            pending_amendments: 0,
            observations: vec![SkillRunObservation {
                id: "future".into(),
                occurred_at_ms: 101,
                succeeded: true,
            }],
        }],
    };
    assert_eq!(
        execute(&input).expect_err("future observation").code,
        ActionErrorCode::MalformedInput
    );
    input.skills[0].observations.clear();
    input.long_window_start_ms = 80;
    assert_eq!(
        execute(&input).expect_err("inverted windows").code,
        ActionErrorCode::MalformedInput
    );
}
