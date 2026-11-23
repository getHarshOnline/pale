//! Law tests for the portable workspace diagnostic classification algebra.

use super::*;

impl RecipientQualifiedWorkspaceInspection {
    pub(crate) fn from_issued(
        compilation: PortableWorkspaceCompilation,
        issued: pale_application::workspace::AdmittedWorkspaceInspection,
    ) -> Result<Self, RecipientQualifiedWorkspaceOwnerMismatch> {
        if !issued.belongs_to(&compilation.universe) {
            return Err(RecipientQualifiedWorkspaceOwnerMismatch);
        }
        Ok(Self {
            compilation,
            admitted: issued,
        })
    }
}

#[test]
fn lock_rejections_preserve_typed_details_without_inventing_source_locations() {
    use pale_core::records::{InvalidWorkspaceLock, WorkspaceModuleId};

    let cases: [(InvalidWorkspaceLock, &[(&str, &str)]); 4] = [
        (
            InvalidWorkspaceLock::SchemaCoordinate { kind: "resolution" },
            &[
                ("lock_kind", "resolution"),
                ("violation", "schema_coordinate"),
            ],
        ),
        (
            InvalidWorkspaceLock::NonCanonical { kind: "universe" },
            &[("lock_kind", "universe"), ("violation", "entry_order")],
        ),
        (
            InvalidWorkspaceLock::DuplicateModule(WorkspaceModuleId::parse("module").unwrap()),
            &[("module", "module"), ("violation", "duplicate_module")],
        ),
        (
            InvalidWorkspaceLock::Limit {
                kind: "resolved release",
                observed: 1025,
                limit: 1024,
            },
            &[
                ("collection", "resolved release"),
                ("observed", "1025"),
                ("limit", "1024"),
                ("violation", "entry_limit"),
            ],
        ),
    ];
    for (error, expected_facts) in cases {
        let projection =
            WorkspaceFailureProjection::from_error(&WorkspaceError::InvalidWorkspaceLock(error));
        assert!(projection.summary.is_some());
        assert!(!projection.help.is_empty());
        assert_eq!(
            projection.facts,
            expected_facts
                .iter()
                .map(|(key, value)| fact(key, value))
                .collect::<Vec<_>>()
        );
        assert!(projection.location.is_none());
        assert!(projection.schema_coordinate.is_none());
    }

    let source = pale_core::canon::parse_strict(b"private-source-not-json").unwrap_err();
    let projection = WorkspaceFailureProjection::from_error(&WorkspaceError::InvalidWorkspaceLock(
        InvalidWorkspaceLock::Canonical(source),
    ));
    assert!(projection.summary.is_none());
    assert!(projection.help.is_empty());
    assert!(projection.facts.is_empty());
    assert!(projection.location.is_none());
}

#[test]
fn every_portable_failure_class_has_one_exhaustive_diagnostic_tuple() {
    let cases = [
        (
            PortableWorkspaceFailureClass::SourceRejected,
            PortableWorkspaceDiagnosticCode::SourceRejected,
            "workspace_source_rejected",
            DiagnosticStageV1::WorkspaceLoading,
            DiagnosticDispositionV1::InvalidInput,
            DiagnosticRetryV1::AfterCorrection,
        ),
        (
            PortableWorkspaceFailureClass::ContractRejected,
            PortableWorkspaceDiagnosticCode::ContractRejected,
            "workspace_contract_rejected",
            DiagnosticStageV1::ContractValidation,
            DiagnosticDispositionV1::InvalidInput,
            DiagnosticRetryV1::AfterCorrection,
        ),
        (
            PortableWorkspaceFailureClass::SemanticRejected,
            PortableWorkspaceDiagnosticCode::SemanticsRejected,
            "workspace_semantics_rejected",
            DiagnosticStageV1::SemanticAdmission,
            DiagnosticDispositionV1::InvalidInput,
            DiagnosticRetryV1::AfterCorrection,
        ),
        (
            PortableWorkspaceFailureClass::SemanticLimitExceeded,
            PortableWorkspaceDiagnosticCode::SemanticLimitExceeded,
            "workspace_semantic_limit_exceeded",
            DiagnosticStageV1::SemanticAdmission,
            DiagnosticDispositionV1::InvalidInput,
            DiagnosticRetryV1::AfterCorrection,
        ),
        (
            PortableWorkspaceFailureClass::SourceIncomplete,
            PortableWorkspaceDiagnosticCode::SourceIncomplete,
            "workspace_source_incomplete",
            DiagnosticStageV1::SemanticAdmission,
            DiagnosticDispositionV1::InvalidInput,
            DiagnosticRetryV1::AfterCorrection,
        ),
        (
            PortableWorkspaceFailureClass::OwnerProofUnavailable,
            PortableWorkspaceDiagnosticCode::OwnerProofUnavailable,
            "workspace_owner_proof_unavailable",
            DiagnosticStageV1::SemanticAdmission,
            DiagnosticDispositionV1::Blocked,
            DiagnosticRetryV1::AfterStateChange,
        ),
        (
            PortableWorkspaceFailureClass::ReleaseNotAdmissible,
            PortableWorkspaceDiagnosticCode::ReleaseNotAdmissible,
            "workspace_release_not_admissible",
            DiagnosticStageV1::SemanticAdmission,
            DiagnosticDispositionV1::Blocked,
            DiagnosticRetryV1::AfterStateChange,
        ),
        (
            PortableWorkspaceFailureClass::InternalInvariant,
            PortableWorkspaceDiagnosticCode::InternalFailure,
            "workspace_internal_failure",
            DiagnosticStageV1::Internal,
            DiagnosticDispositionV1::InternalFailure,
            DiagnosticRetryV1::NotIndicated,
        ),
    ];

    for (class, code, wire_code, stage, disposition, retry) in cases {
        let actual = classify_portable_workspace_failure(class);
        assert_eq!(actual.code, code);
        assert_eq!(actual.code.as_str(), wire_code);
        assert_eq!(actual.stage, stage);
        assert_eq!(actual.disposition, disposition);
        assert_eq!(actual.retry, retry);
    }
}
