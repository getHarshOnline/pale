//! Compiler failure advice must match the candidate's actual recovery condition.

use pale_application::compile::{CompileError, CompositionError};

#[test]
fn candidate_composition_failure_advice_distinguishes_bad_input_from_engine_failure() {
    let identity = pale_contracts::record_schema_identity("diagnostic.v1").unwrap();
    for (source, disposition, retry, help_expected, subject) in [
        (
            CompositionError::Path {
                path: "selected-path".into(),
                reason: "private-reason".into(),
            },
            crate::DiagnosticDisposition::InvalidInput,
            crate::DiagnosticRetry::AfterCorrection,
            true,
            Some(("experience_path", "selected-path")),
        ),
        (
            CompositionError::Schema(pale_contracts::SchemaError::Integrity(
                "private-reason".into(),
            )),
            crate::DiagnosticDisposition::InternalFailure,
            crate::DiagnosticRetry::NotIndicated,
            false,
            None,
        ),
    ] {
        let error = super::improvement_profile_failure(
            &identity,
            CompileError::CompositionInput(Box::new(source)),
        );
        let diagnostic = error.diagnostic();
        assert_eq!(diagnostic.disposition(), disposition);
        assert_eq!(diagnostic.retry(), retry);
        assert_eq!(!diagnostic.help().is_empty(), help_expected);
        assert_eq!(diagnostic.subject(), subject);
        assert!(!diagnostic.canonical_json().contains("private-reason"));
    }
}
