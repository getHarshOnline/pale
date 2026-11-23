//! Verifies bounded, redacted projection of typed failures into public diagnostics.

use std::error::Error;
use std::fmt;

use pale_application::ApplicationFailureDisposition;
use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1,
    DiagnosticSubjectV1Wire, WorkspaceExportId, WorkspaceModuleId, WorkspaceVisibilityV1,
};

use super::{application_diagnostic_policy, ApiError, FailureSpecification};

// Expected public diagnostic limits, independent of the construction implementation.
const MAX_CAUSES: usize = 16;
const MAX_CAUSE_SCAN: usize = 64;
const MAX_TEXT_CHARS: usize = 4_096;

#[test]
fn composition_details_do_not_expose_free_form_reasons_or_make_internal_failures_retryable() {
    use pale_application::compile::{CompileError, CompositionError};

    let identity = pale_contracts::record_schema_identity("diagnostic.v1").unwrap();
    let marker = "private-composition-context";
    for (source, disposition, retry, subject) in [
        (
            CompositionError::Path {
                path: "selected-path".into(),
                reason: marker.into(),
            },
            crate::DiagnosticDisposition::InvalidInput,
            crate::DiagnosticRetry::AfterCorrection,
            Some(("experience_path", "selected-path")),
        ),
        (
            CompositionError::DeclaredSchema {
                schema_id: "selected-schema.v1".into(),
                source: pale_contracts::SchemaError::Integrity(marker.into()),
            },
            crate::DiagnosticDisposition::InternalFailure,
            crate::DiagnosticRetry::NotIndicated,
            None,
        ),
    ] {
        let error = crate::one_shot::compilation_failure(
            &identity,
            CompileError::CompositionInput(Box::new(source)),
        );
        assert_eq!(error.diagnostic().disposition(), disposition);
        assert_eq!(error.diagnostic().retry(), retry);
        assert_eq!(error.diagnostic().subject(), subject);
        assert!(error.source().is_none());
        for visible in [
            error.to_string(),
            format!("{error:?}"),
            error.diagnostic().canonical_json().to_owned(),
        ] {
            assert!(!visible.contains(marker));
        }
    }
}

#[test]
fn application_consequences_have_one_portable_retry_policy() {
    for (application, disposition, retry) in [
        (
            ApplicationFailureDisposition::Operational,
            DiagnosticDispositionV1::OperationalFailure,
            DiagnosticRetryV1::NotIndicated,
        ),
        (
            ApplicationFailureDisposition::InvalidInput,
            DiagnosticDispositionV1::InvalidInput,
            DiagnosticRetryV1::AfterCorrection,
        ),
        (
            ApplicationFailureDisposition::Blocked,
            DiagnosticDispositionV1::Blocked,
            DiagnosticRetryV1::AfterStateChange,
        ),
        (
            ApplicationFailureDisposition::Internal,
            DiagnosticDispositionV1::InternalFailure,
            DiagnosticRetryV1::NotIndicated,
        ),
    ] {
        assert_eq!(
            application_diagnostic_policy(application),
            (disposition, retry)
        );
    }
}

#[test]
fn compiler_correction_is_bound_to_the_sealed_diagnostic_identity() {
    let source = pale_application::workspace::WorkspaceError::ImportVisibilityWidening {
        source_label: "consumer.toml".to_owned(),
        importer_module: WorkspaceModuleId::parse("consumer".to_owned()).unwrap(),
        import_index: 2,
        upstream_module: WorkspaceModuleId::parse("owner".to_owned()).unwrap(),
        export_id: WorkspaceExportId::parse("surface".to_owned()).unwrap(),
        export_visibility: WorkspaceVisibilityV1::Protected,
        import_ceiling: WorkspaceVisibilityV1::Public,
    };
    let evidence = source
        .authoring_correction()
        .expect("the typed lattice failure proves one correction");
    let identity = pale_contracts::record_schema_identity("diagnostic.v1")
        .expect("diagnostic identity is coherent");
    let error = ApiError::new(
        &identity,
        source,
        FailureSpecification {
            code: "workspace_semantics_rejected",
            stage: DiagnosticStageV1::SemanticAdmission,
            disposition: DiagnosticDispositionV1::InvalidInput,
            summary: "portable workspace meaning was rejected",
            subject: None,
            schema_coordinate: None,
            location: None,
            retry: DiagnosticRetryV1::AfterCorrection,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: Vec::new(),
            help: Vec::new(),
            facts: Vec::new(),
        },
    )
    .with_authoring_correction(&evidence);

    let correction = error
        .authoring_corrections()
        .first()
        .expect("one bounded correction");
    assert_eq!(correction.source_label(), "consumer.toml");
    assert_eq!(correction.expected_source(), "\"public\"");
    assert_eq!(correction.replacement_source(), "\"protected\"");
    assert_eq!(
        correction.diagnostic_identity(),
        error.diagnostic().identity()
    );
}

/// Synthetic finite source chain used to exercise projection bounds.
#[derive(Debug)]
struct ChainError {
    /// Safe index plus a deliberately credential-shaped value.
    message: String,
    /// Next older cause.
    source: Option<Box<Self>>,
}

impl fmt::Display for ChainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ChainError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_deref().map(|source| source as &dyn Error)
    }
}

#[test]
fn projection_redacts_and_bounds_the_complete_visible_source_chain() {
    let source = (0..80).rev().fold(None, |source, index| {
        Some(Box::new(ChainError {
            message: format!("cause-{index} token=private-{index}"),
            source,
        }))
    });
    let source = ChainError {
        message: "root token=private-root".to_string(),
        source,
    };
    let identity = pale_contracts::record_schema_identity("diagnostic.v1")
        .expect("diagnostic identity is coherent");
    let error = ApiError::new(
        &identity,
        source,
        FailureSpecification {
            code: "test_failure",
            stage: DiagnosticStageV1::Internal,
            disposition: DiagnosticDispositionV1::InternalFailure,
            summary: "summary token=private-summary",
            subject: None,
            schema_coordinate: None,
            location: None,
            retry: DiagnosticRetryV1::NotIndicated,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: Vec::new(),
            help: Vec::new(),
            facts: Vec::new(),
        },
    );
    let value = pale_core::canon::parse_strict(error.diagnostic().canonical_bytes())
        .expect("projected diagnostic remains strict JSON");
    let causes = value["causes"].as_array().expect("causes array");
    assert_eq!(causes.len(), MAX_CAUSES);
    assert_eq!(causes[0], "root token=[REDACTED]");
    assert_eq!(
        value["truncation"]["omitted_causes"],
        (MAX_CAUSE_SCAN - MAX_CAUSES + 1) as u64
    );
    assert!(!String::from_utf8_lossy(error.diagnostic().canonical_bytes()).contains("private"));
}

#[test]
fn shortened_fields_include_the_ellipsis_inside_the_contract_ceiling() {
    let identity = pale_contracts::record_schema_identity("diagnostic.v1")
        .expect("diagnostic identity is coherent");
    let error = ApiError::new(
        &identity,
        ChainError {
            message: "root".to_string(),
            source: None,
        },
        FailureSpecification {
            code: "test_failure",
            stage: DiagnosticStageV1::Internal,
            disposition: DiagnosticDispositionV1::InternalFailure,
            summary: "bounded subject",
            subject: Some(DiagnosticSubjectV1Wire {
                kind: "action".to_string(),
                id: "x".repeat(1_025),
            }),
            schema_coordinate: None,
            location: None,
            retry: DiagnosticRetryV1::NotIndicated,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: Vec::new(),
            help: Vec::new(),
            facts: Vec::new(),
        },
    );
    let subject = error.diagnostic().subject().expect("subject is retained").1;

    assert_eq!(subject.chars().count(), 1_024);
    assert!(subject.ends_with('…'));
    assert_eq!(error.diagnostic().truncation().shortened_fields(), 1);
}

#[test]
fn discarded_causes_do_not_inflate_retained_field_shortening() {
    let identity = pale_contracts::record_schema_identity("diagnostic.v1")
        .expect("diagnostic identity is coherent");
    let oversized = "x".repeat(MAX_TEXT_CHARS + 1);
    let error = ApiError::new(
        &identity,
        ChainError {
            message: oversized.clone(),
            source: None,
        },
        FailureSpecification {
            code: "test_failure",
            stage: DiagnosticStageV1::Internal,
            disposition: DiagnosticDispositionV1::InternalFailure,
            summary: &oversized,
            subject: None,
            schema_coordinate: None,
            location: None,
            retry: DiagnosticRetryV1::NotIndicated,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: Vec::new(),
            help: Vec::new(),
            facts: Vec::new(),
        },
    );

    assert!(error.diagnostic().causes().is_empty());
    assert_eq!(error.diagnostic().truncation().shortened_fields(), 1);

    let duplicate = ApiError::new(
        &identity,
        ChainError {
            message: "root".to_string(),
            source: Some(Box::new(ChainError {
                message: oversized.clone(),
                source: Some(Box::new(ChainError {
                    message: oversized,
                    source: None,
                })),
            })),
        },
        FailureSpecification {
            code: "test_failure",
            stage: DiagnosticStageV1::Internal,
            disposition: DiagnosticDispositionV1::InternalFailure,
            summary: "summary",
            subject: None,
            schema_coordinate: None,
            location: None,
            retry: DiagnosticRetryV1::NotIndicated,
            effect: DiagnosticEffectCertaintyV1::NotApplicable,
            request: None,
            notes: Vec::new(),
            help: Vec::new(),
            facts: Vec::new(),
        },
    );

    assert_eq!(duplicate.diagnostic().causes().len(), 2);
    assert_eq!(duplicate.diagnostic().truncation().shortened_fields(), 1);
    assert_eq!(duplicate.diagnostic().truncation().omitted_causes(), 1);
}

#[test]
fn lifecycle_conflict_explains_the_required_event_and_session_limits() {
    use pale_application::compile::CompileError;

    let identity = pale_contracts::record_schema_identity("diagnostic.v1").unwrap();
    let error = crate::one_shot::compilation_failure(
        &identity,
        CompileError::LifecycleConflict {
            execution_path: pale_core::records::ExecutionPathSelectionIdV1::parse("direct-claude")
                .unwrap(),
            event: pale_core::records::LifecycleEvent::PreToolUse,
            declared: vec![pale_core::records::LifecycleEvent::SessionStart],
        },
    );
    let diagnostic = error.diagnostic();
    assert_eq!(diagnostic.code(), "one_shot_lifecycle_conflict");
    assert_eq!(
        diagnostic.stage(),
        crate::DiagnosticStage::SemanticAdmission
    );
    assert_eq!(
        diagnostic.disposition(),
        crate::DiagnosticDisposition::InvalidInput
    );
    assert_eq!(diagnostic.retry(), crate::DiagnosticRetry::AfterCorrection);
    assert_eq!(
        diagnostic.subject(),
        Some(("execution_path", "direct-claude"))
    );
    for (key, value) in [
        ("field", "lifecycle_events"),
        ("required_event", "pre-tool-use"),
        ("declared_events", "session-start"),
    ] {
        assert!(diagnostic
            .facts()
            .iter()
            .any(|fact| fact.key() == key && fact.value() == value));
    }
    assert!(error.to_string().contains("lifecycle_events"));
    assert!(!diagnostic.help().is_empty());
    assert!(error.source().is_none());
}

#[test]
fn terminal_provider_failure_keeps_its_cause_private_and_its_correction_distinct() {
    use pale_application::compile::CompileError;
    use pale_application::terminal_adapters::TerminalAdapterResolutionError;

    let identity = pale_contracts::record_schema_identity("diagnostic.v1").unwrap();
    let marker = "private-provider-context";
    let error = crate::one_shot::compilation_failure(
        &identity,
        CompileError::TerminalProvider {
            execution_path: pale_core::records::ExecutionPathSelectionIdV1::parse("direct-claude")
                .unwrap(),
            source: TerminalAdapterResolutionError::UnknownAdapterPlugin(marker.into()),
        },
    );
    let diagnostic = error.diagnostic();
    assert_eq!(diagnostic.code(), "one_shot_terminal_provider_rejected");
    assert_eq!(
        diagnostic.stage(),
        crate::DiagnosticStage::SemanticAdmission
    );
    assert_eq!(
        diagnostic.disposition(),
        crate::DiagnosticDisposition::InvalidInput
    );
    assert_eq!(diagnostic.retry(), crate::DiagnosticRetry::AfterCorrection);
    assert_eq!(
        diagnostic.subject(),
        Some(("execution_path", "direct-claude"))
    );
    assert!(error.to_string().contains("terminal provider"));
    assert!(!error.to_string().contains("lifecycle_events"));
    assert!(!diagnostic.help().is_empty());
    assert!(error.source().is_none());
    for visible in [
        error.to_string(),
        format!("{error:?}"),
        diagnostic.canonical_json().to_owned(),
    ] {
        assert!(!visible.contains(marker));
        assert!(!visible.contains("lifecycle_events"));
    }
}
