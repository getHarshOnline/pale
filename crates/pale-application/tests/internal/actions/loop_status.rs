//! Loop-status reduction proofs for stale work, wakeups, and malformed observations.

use super::*;

#[test]
fn stale_work_overdue_wakeups_and_parse_errors_reduce_fail_closed() {
    let input = LoopStatusInput {
        now_ms: 10_000,
        bash_stale_after_ms: 1_000,
        wakeup_grace_ms: 100,
        events: vec![
            LoopEvent::ToolUse {
                id: "tool-1".into(),
                tool_kind: LoopToolKind::Bash,
                started_at_ms: 5_000,
            },
            LoopEvent::WakeupScheduled {
                id: "wake-1".into(),
                due_at_ms: 8_000,
            },
            LoopEvent::ParseError {
                id: "parse-1".into(),
                observed_at_ms: 9_000,
            },
        ],
    };
    let output = execute(&input).expect("valid loop evidence");
    assert_eq!(output.stale_bash[0].age_ms, 5_000);
    assert_eq!(output.overdue_wakeups[0].overdue_ms, 1_900);
    assert_eq!(output.recommendation, LoopRecommendation::Stop);
}

#[test]
fn completed_tools_are_not_stale_and_dangling_results_are_rejected() {
    let events = vec![
        LoopEvent::ToolResult {
            id: "result".into(),
            tool_use_id: "tool".into(),
            completed_at_ms: 2,
            succeeded: true,
        },
        LoopEvent::ToolUse {
            id: "tool".into(),
            tool_kind: LoopToolKind::Bash,
            started_at_ms: 1,
        },
    ];
    let output = execute(&LoopStatusInput {
        now_ms: 10,
        bash_stale_after_ms: 1,
        wakeup_grace_ms: 0,
        events,
    })
    .expect("input order is irrelevant");
    assert!(output.stale_bash.is_empty());

    let error = execute(&LoopStatusInput {
        now_ms: 10,
        bash_stale_after_ms: 1,
        wakeup_grace_ms: 0,
        events: vec![LoopEvent::ToolResult {
            id: "result".into(),
            tool_use_id: "missing".into(),
            completed_at_ms: 2,
            succeeded: true,
        }],
    })
    .expect_err("dangling result");
    assert_eq!(error.code, ActionErrorCode::MalformedInput);
}
