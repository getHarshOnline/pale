//! Configuration-invariant proofs over supplied text and explicitly bounded scopes.

use pale_core::records::{ConfigInvariant, ConfigTextRecord};

use super::*;

#[test]
fn required_and_forbidden_literals_are_evidenced_without_io() {
    let input = ConfigInvariantScanInput {
        records: vec![ConfigTextRecord {
            id: "settings".into(),
            text: "hooks = true\nallow = all\nhooks = true\n".into(),
        }],
        invariants: vec![
            ConfigInvariant {
                id: "hooks-enabled".into(),
                expectation: ConfigInvariantExpectation::RequiredLiteral,
                literal: "hooks = true".into(),
                record_ids: vec![],
            },
            ConfigInvariant {
                id: "no-open-allow".into(),
                expectation: ConfigInvariantExpectation::ForbiddenLiteral,
                literal: "allow = all".into(),
                record_ids: vec!["settings".into()],
            },
        ],
    };
    let output = execute(&input).expect("valid scan");
    assert!(!output.passed);
    assert_eq!(output.results[0].invariant_id, "hooks-enabled");
    assert_eq!(output.results[0].match_count, 2);
    assert_eq!(output.results[1].evidence[0].line, 2);
}

#[test]
fn comments_and_quoted_examples_are_intentionally_lexical_matches() {
    let input = ConfigInvariantScanInput {
        records: vec![ConfigTextRecord {
            id: "examples".into(),
            text: "# allow = all\nexample = \"allow = all\"\n".into(),
        }],
        invariants: vec![ConfigInvariant {
            id: "open-allow-lead".into(),
            expectation: ConfigInvariantExpectation::ForbiddenLiteral,
            literal: "allow = all".into(),
            record_ids: vec![],
        }],
    };

    let output = execute(&input).expect("valid lexical scan");
    assert!(!output.passed);
    assert_eq!(output.results[0].match_count, 2);
    assert_eq!(
        output.results[0]
            .evidence
            .iter()
            .map(|row| row.line)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
}

#[test]
fn unknown_scope_and_duplicate_ids_fail_closed() {
    let base = ConfigInvariantScanInput {
        records: vec![ConfigTextRecord {
            id: "config".into(),
            text: "safe = true".into(),
        }],
        invariants: vec![ConfigInvariant {
            id: "safe".into(),
            expectation: ConfigInvariantExpectation::RequiredLiteral,
            literal: "safe".into(),
            record_ids: vec!["missing".into()],
        }],
    };
    assert_eq!(
        execute(&base).expect_err("unknown scope").code,
        ActionErrorCode::MalformedInput
    );
    let mut duplicate = base;
    duplicate.invariants[0].record_ids = vec![];
    duplicate.records.push(duplicate.records[0].clone());
    assert_eq!(
        execute(&duplicate).expect_err("duplicate record").code,
        ActionErrorCode::DuplicateIdentifier
    );
}

#[test]
fn sparse_scopes_are_bounded_by_selected_records() {
    let records = (0..65)
        .map(|index| ConfigTextRecord {
            id: format!("record-{index}"),
            text: "needle".into(),
        })
        .collect();
    let invariants = (0..127)
        .map(|index| ConfigInvariant {
            id: format!("invariant-{index}"),
            expectation: ConfigInvariantExpectation::RequiredLiteral,
            literal: "needle".into(),
            record_ids: vec![format!("record-{}", index % 65)],
        })
        .collect();

    let output = execute(&ConfigInvariantScanInput {
        records,
        invariants,
    })
    .expect("sparse scopes remain below the actual work bound");

    assert!(output.passed);
    assert_eq!(output.results.len(), 127);
}
