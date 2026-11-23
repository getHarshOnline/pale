//! Evidence-selection proofs for declared precedence and ambiguity rejection.

use super::*;

#[test]
fn declared_precedence_preserves_evidence_and_unknowns() {
    let output = execute(&EvidenceSelectionInput {
        mode: EvidenceSelectionMode::FirstMatch,
        observed: vec![
            "lock:pnpm".into(),
            "manifest:node".into(),
            "future:unknown".into(),
        ],
        rules: vec![
            EvidenceRule {
                id: "explicit-config".into(),
                require_all: vec!["config:pnpm".into()],
                require_any: vec![],
                exclude: vec![],
            },
            EvidenceRule {
                id: "pnpm-lock".into(),
                require_all: vec!["manifest:node".into(), "lock:pnpm".into()],
                require_any: vec![],
                exclude: vec!["config:npm".into()],
            },
            EvidenceRule {
                id: "fallback".into(),
                require_all: vec![],
                require_any: vec![],
                exclude: vec![],
            },
        ],
    })
    .expect("valid evidence rules");

    assert_eq!(output.matches.len(), 1);
    assert_eq!(output.matches[0].id, "pnpm-lock");
    assert_eq!(
        output.matches[0].evidence,
        vec!["lock:pnpm", "manifest:node"]
    );
    assert_eq!(output.unmatched_observed, vec!["future:unknown"]);
}

#[test]
fn ambiguous_rules_and_malformed_signals_fail_closed() {
    let ambiguous = EvidenceSelectionInput {
        mode: EvidenceSelectionMode::AllMatches,
        observed: vec!["lock:pnpm".into()],
        rules: vec![EvidenceRule {
            id: "ambiguous".into(),
            require_all: vec!["lock:pnpm".into()],
            require_any: vec!["lock:pnpm".into()],
            exclude: vec![],
        }],
    };
    assert_eq!(
        execute(&ambiguous)
            .expect_err("ambiguous positive rule")
            .code,
        ActionErrorCode::MalformedInput
    );

    let malformed = EvidenceSelectionInput {
        mode: EvidenceSelectionMode::FirstMatch,
        observed: vec![" lock:pnpm".into()],
        rules: vec![],
    };
    assert_eq!(
        execute(&malformed).expect_err("whitespace signal").code,
        ActionErrorCode::MalformedInput
    );
}
