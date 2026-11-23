//! Compliance grading proofs for rule closure, event order, and deterministic witnesses.

use pale_core::records::{ClassifiedTraceEvent, ComplianceStep};

use super::*;

#[test]
fn classified_trace_is_graded_for_presence_and_order() {
    let input = ComplianceTraceGradeInput {
        events: vec![
            ClassifiedTraceEvent {
                id: "e2".into(),
                class: "verify".into(),
                sequence: 20,
            },
            ClassifiedTraceEvent {
                id: "e1".into(),
                class: "edit".into(),
                sequence: 10,
            },
        ],
        required_steps: vec![
            ComplianceStep {
                id: "edit".into(),
                event_class: "edit".into(),
                minimum_occurrences: 1,
            },
            ComplianceStep {
                id: "verify".into(),
                event_class: "verify".into(),
                minimum_occurrences: 1,
            },
        ],
        temporal_rules: vec![ComplianceTemporalRule::Before {
            id: "verify-after-edit".into(),
            first_step_id: "edit".into(),
            second_step_id: "verify".into(),
        }],
        promotion_threshold_micros: 800_000,
    };
    let output = execute(&input).expect("valid classified trace");
    assert_eq!(output.compliance_rate_micros, 1_000_000);
    assert_eq!(output.promotion, CompliancePromotion::Eligible);
}

#[test]
fn duplicate_sequence_and_unknown_rule_step_fail_closed() {
    let mut input = ComplianceTraceGradeInput {
        events: vec![
            ClassifiedTraceEvent {
                id: "a".into(),
                class: "edit".into(),
                sequence: 1,
            },
            ClassifiedTraceEvent {
                id: "b".into(),
                class: "verify".into(),
                sequence: 1,
            },
        ],
        required_steps: vec![ComplianceStep {
            id: "edit".into(),
            event_class: "edit".into(),
            minimum_occurrences: 1,
        }],
        temporal_rules: vec![],
        promotion_threshold_micros: 800_000,
    };
    assert_eq!(
        execute(&input).expect_err("duplicate sequence").code,
        ActionErrorCode::DuplicateIdentifier
    );
    input.events.pop();
    input.temporal_rules = vec![ComplianceTemporalRule::After {
        id: "unknown".into(),
        first_step_id: "missing".into(),
        second_step_id: "edit".into(),
    }];
    assert_eq!(
        execute(&input).expect_err("unknown step").code,
        ActionErrorCode::MalformedInput
    );
}

#[test]
fn repeated_events_use_earliest_before_and_latest_after_witnesses() {
    let steps = vec![
        ComplianceStep {
            id: "edit".into(),
            event_class: "edit".into(),
            minimum_occurrences: 1,
        },
        ComplianceStep {
            id: "verify".into(),
            event_class: "verify".into(),
            minimum_occurrences: 1,
        },
    ];
    let events = vec![
        ClassifiedTraceEvent {
            id: "edit-early".into(),
            class: "edit".into(),
            sequence: 5,
        },
        ClassifiedTraceEvent {
            id: "verify".into(),
            class: "verify".into(),
            sequence: 10,
        },
        ClassifiedTraceEvent {
            id: "edit-late".into(),
            class: "edit".into(),
            sequence: 20,
        },
    ];
    let output = execute(&ComplianceTraceGradeInput {
        events,
        required_steps: steps,
        temporal_rules: vec![
            ComplianceTemporalRule::Before {
                id: "edit-before-verify".into(),
                first_step_id: "edit".into(),
                second_step_id: "verify".into(),
            },
            ComplianceTemporalRule::After {
                id: "edit-after-verify".into(),
                first_step_id: "edit".into(),
                second_step_id: "verify".into(),
            },
        ],
        promotion_threshold_micros: 800_000,
    })
    .expect("valid repeated-event trace");
    assert!(output.temporal_rules.iter().all(|rule| rule.passed));

    let late_reference = execute(&ComplianceTraceGradeInput {
        events: vec![
            ClassifiedTraceEvent {
                id: "edit-early".into(),
                class: "edit".into(),
                sequence: 5,
            },
            ClassifiedTraceEvent {
                id: "verify-early".into(),
                class: "verify".into(),
                sequence: 10,
            },
            ClassifiedTraceEvent {
                id: "edit-late".into(),
                class: "edit".into(),
                sequence: 20,
            },
            ClassifiedTraceEvent {
                id: "verify-latest".into(),
                class: "verify".into(),
                sequence: 30,
            },
        ],
        required_steps: vec![
            ComplianceStep {
                id: "edit".into(),
                event_class: "edit".into(),
                minimum_occurrences: 1,
            },
            ComplianceStep {
                id: "verify".into(),
                event_class: "verify".into(),
                minimum_occurrences: 1,
            },
        ],
        temporal_rules: vec![ComplianceTemporalRule::After {
            id: "edit-after-latest-verify".into(),
            first_step_id: "edit".into(),
            second_step_id: "verify".into(),
        }],
        promotion_threshold_micros: 800_000,
    })
    .expect("valid trace with a later reference");
    assert!(!late_reference.temporal_rules[0].passed);
}
