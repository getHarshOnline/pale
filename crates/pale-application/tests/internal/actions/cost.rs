//! Cost aggregation proofs for temporal ordering, snapshot semantics, and monotone history.

use super::*;

#[test]
fn latest_session_snapshot_is_aggregated_by_day_and_model() {
    let input = CostAggregateInput {
        today: "2026-08-05".into(),
        snapshots: vec![
            CostSnapshot {
                session_id: "s1".into(),
                snapshot_index: 2,
                date: "2026-08-05".into(),
                model: "alpha".into(),
                cost_micro_usd: 250,
            },
            CostSnapshot {
                session_id: "s2".into(),
                snapshot_index: 1,
                date: "2026-08-04".into(),
                model: "beta".into(),
                cost_micro_usd: 70,
            },
            CostSnapshot {
                session_id: "s1".into(),
                snapshot_index: 1,
                date: "2026-08-05".into(),
                model: "alpha".into(),
                cost_micro_usd: 100,
            },
        ],
    };
    let output = execute(&input).expect("valid aggregate");
    assert_eq!(output.selected_sessions, 2);
    assert_eq!(output.total_cost_micro_usd, 320);
    assert_eq!(output.today_cost_micro_usd, 250);
    assert_eq!(output.by_day_model[1].cost_micro_usd, 250);
}

#[test]
fn duplicate_ordering_and_invalid_dates_fail_closed() {
    let snapshot = CostSnapshot {
        session_id: "s1".into(),
        snapshot_index: 1,
        date: "2026-02-29".into(),
        model: "alpha".into(),
        cost_micro_usd: 1,
    };
    assert_eq!(
        execute(&CostAggregateInput {
            today: "2026-08-05".into(),
            snapshots: vec![snapshot.clone()],
        })
        .expect_err("invalid date")
        .code,
        ActionErrorCode::MalformedInput
    );
    let mut valid = snapshot;
    valid.date = "2026-02-28".into();
    assert_eq!(
        execute(&CostAggregateInput {
            today: "2026-08-05".into(),
            snapshots: vec![valid.clone(), valid],
        })
        .expect_err("duplicate snapshot index")
        .code,
        ActionErrorCode::DuplicateIdentifier
    );
}

#[test]
fn cumulative_session_history_cannot_regress() {
    let snapshot = |snapshot_index, date: &str, cost_micro_usd| CostSnapshot {
        session_id: "s1".into(),
        snapshot_index,
        date: date.into(),
        model: "alpha".into(),
        cost_micro_usd,
    };

    for snapshots in [
        vec![
            snapshot(1, "2026-08-05", 100),
            snapshot(2, "2026-08-04", 200),
        ],
        vec![
            snapshot(1, "2026-08-04", 200),
            snapshot(2, "2026-08-05", 100),
        ],
    ] {
        assert_eq!(
            execute(&CostAggregateInput {
                today: "2026-08-05".into(),
                snapshots,
            })
            .expect_err("cumulative history regression must fail")
            .code,
            ActionErrorCode::MalformedInput
        );
    }
}

#[test]
fn calendar_validity_is_stricter_than_parseable_numeric_fields() {
    for valid in ["2000-02-29", "2024-02-29", "9999-12-31"] {
        assert!(parse_date(valid).is_some(), "valid date {valid}");
    }
    for invalid in [
        "0000-01-01",
        "1900-02-29",
        "2026-02-29",
        "2026-2-01",
        "2026-02-1",
        "２０２６-02-01",
        "2026-00-01",
        "2026-01-00",
    ] {
        assert!(parse_date(invalid).is_none(), "invalid date {invalid}");
    }
}

#[test]
fn gregorian_month_ends_and_century_rules_are_exact() {
    for valid in [
        "0001-01-01",
        "1600-02-29",
        "2000-02-29",
        "2026-01-31",
        "2026-04-30",
        "2026-12-31",
        "9999-12-31",
    ] {
        assert!(parse_date(valid).is_some(), "valid date {valid}");
    }
    for invalid in [
        "1700-02-29",
        "1800-02-29",
        "1900-02-29",
        "2100-02-29",
        "2026-04-31",
        "2026-06-31",
        "2026-09-31",
        "2026-11-31",
        "9999-12-32",
        "２０２６-０８-０５",
    ] {
        assert!(parse_date(invalid).is_none(), "invalid date {invalid}");
    }
}

#[test]
fn snapshot_dates_are_not_future_and_remain_monotone_across_years() {
    let snapshot = |snapshot_index, date: &str| CostSnapshot {
        session_id: "s1".into(),
        snapshot_index,
        date: date.into(),
        model: "alpha".into(),
        cost_micro_usd: snapshot_index,
    };
    execute(&CostAggregateInput {
        today: "2026-01-01".into(),
        snapshots: vec![snapshot(1, "2025-12-31"), snapshot(2, "2026-01-01")],
    })
    .expect("year-boundary chronology is monotone");
    assert_eq!(
        execute(&CostAggregateInput {
            today: "2026-01-01".into(),
            snapshots: vec![snapshot(1, "2026-01-02")],
        })
        .expect_err("a future snapshot must fail")
        .code,
        ActionErrorCode::MalformedInput
    );
}
