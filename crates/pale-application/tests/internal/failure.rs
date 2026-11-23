//! Trust-boundary laws for the shared application failure disposition.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::*;

#[test]
fn composition_schema_origin_and_compiler_phase_preserve_failure_consequences() {
    let invalid = pale_contracts::validate_value(
        &serde_json::json!({}),
        pale_core::records::EXPERIENCE_COMPOSITION_PLAN_V1_SCHEMA_ID,
    )
    .expect_err("an empty object violates the generated plan contract");
    assert!(matches!(invalid, SchemaError::Invalid(_)));
    for source in [
        SchemaError::UnknownId("declared.v1".into()),
        SchemaError::IdentityMismatch("declared.v1".into()),
        invalid,
    ] {
        assert_eq!(
            CompileError::CompositionInput(Box::new(CompositionError::DeclaredSchema {
                schema_id: "declared.v1".into(),
                source,
            }))
            .failure_disposition(),
            ApplicationFailureDisposition::InvalidInput
        );
    }
    for source in [
        SchemaError::UnknownId("embedded.v1".into()),
        SchemaError::IdentityMismatch("embedded.v1".into()),
        SchemaError::Integrity("broken registry".into()),
    ] {
        assert_eq!(
            CompileError::CompositionInput(Box::new(CompositionError::Schema(source)))
                .failure_disposition(),
            ApplicationFailureDisposition::Internal
        );
    }
    assert_eq!(
        CompositionError::DeclaredSchema {
            schema_id: "declared.v1".into(),
            source: SchemaError::Integrity("broken registry".into()),
        }
        .failure_disposition(),
        ApplicationFailureDisposition::Internal
    );
    let serialization = serde_json::to_value(BTreeMap::from([(vec![1, 2], 0)]))
        .expect_err("a sequence cannot be a JSON object key");
    assert_eq!(
        CompositionError::Canonicalization(pale_core::canon::CanonError::Canonicalize(
            serialization
        ))
        .failure_disposition(),
        ApplicationFailureDisposition::Internal
    );
    assert_eq!(
        CompileError::from(crate::compile::CompilationInvariantError::Composition(
            CompositionError::Schema(SchemaError::IdentityMismatch("generated.v1".into()))
        ))
        .failure_disposition(),
        ApplicationFailureDisposition::Internal
    );
}

#[test]
fn direct_and_compiler_owned_planning_failures_keep_distinct_trust_boundaries() {
    let supplied = PlanError::InvalidSuppliedPlan("caller bytes".into());
    assert_eq!(
        supplied.failure_disposition(),
        ApplicationFailureDisposition::InvalidInput
    );
    let compiled = CompileError::Plan(Box::new(supplied));
    assert_eq!(
        compiled.failure_disposition(),
        ApplicationFailureDisposition::Internal
    );

    assert_eq!(
        PlanError::OwnershipTransition(".agents/skills/review/SKILL.md".into())
            .failure_disposition(),
        ApplicationFailureDisposition::Blocked
    );
    assert_eq!(
        PlanError::PlanInvariant {
            coordinate: "fixture",
            detail: "impossible transition".into(),
        }
        .failure_disposition(),
        ApplicationFailureDisposition::Internal
    );
}

#[test]
fn direct_render_input_is_invalid_but_compiler_render_failure_is_internal() {
    let direct = RenderError::InvalidInput {
        field: "fixture",
        reason: "contradiction".into(),
    };
    assert_eq!(
        direct.failure_disposition(),
        ApplicationFailureDisposition::InvalidInput
    );
    assert_eq!(
        CompileError::Render(Box::new(direct)).failure_disposition(),
        ApplicationFailureDisposition::Internal
    );
}

#[test]
fn package_and_workspace_failures_preserve_all_four_consequences() {
    assert_eq!(
        PackageError::Read {
            path: PathBuf::from("package.toml"),
            source: std::io::Error::other("fixture"),
        }
        .failure_disposition(),
        ApplicationFailureDisposition::Operational
    );
    assert_eq!(
        PackageError::InvalidEntry {
            path: PathBuf::from("../escape"),
        }
        .failure_disposition(),
        ApplicationFailureDisposition::InvalidInput
    );
    assert_eq!(
        WorkspaceError::SurfaceExecutableMissing.failure_disposition(),
        ApplicationFailureDisposition::Blocked
    );
    assert_eq!(
        WorkspaceError::InputChanged("fixture".into()).failure_disposition(),
        ApplicationFailureDisposition::Operational
    );
    assert_eq!(
        WorkspaceError::InputNotUtf8(PathBuf::from("workspace.toml")).failure_disposition(),
        ApplicationFailureDisposition::InvalidInput
    );
}

#[test]
fn portable_workspace_classes_refine_one_shared_recovery_algebra() {
    use crate::workspace::PortableWorkspaceFailureClass as Portable;

    for class in [
        Portable::SourceRejected,
        Portable::ContractRejected,
        Portable::SemanticRejected,
        Portable::SemanticLimitExceeded,
        Portable::SourceIncomplete,
    ] {
        assert_eq!(
            class.failure_disposition(),
            ApplicationFailureDisposition::InvalidInput
        );
    }
    for class in [
        Portable::OwnerProofUnavailable,
        Portable::ReleaseNotAdmissible,
    ] {
        assert_eq!(
            class.failure_disposition(),
            ApplicationFailureDisposition::Blocked
        );
    }
    assert_eq!(
        Portable::InternalInvariant.failure_disposition(),
        ApplicationFailureDisposition::Internal
    );
}

#[test]
fn operation_admission_owns_input_blocking_and_invariant_consequences() {
    assert_eq!(
        OperationAdmissionError::UnknownOperation {
            operation: "missing".to_string(),
        }
        .failure_disposition(),
        ApplicationFailureDisposition::InvalidInput
    );
    assert_eq!(
        OperationAdmissionError::StaleLease.failure_disposition(),
        ApplicationFailureDisposition::Blocked
    );
    assert_eq!(
        OperationAdmissionError::BindingCatalogMismatch.failure_disposition(),
        ApplicationFailureDisposition::Internal
    );
}

#[test]
fn selected_embedded_schema_failures_keep_one_application_consequence() {
    assert_eq!(
        embedded_schema_failure_disposition(&SchemaError::IdentityMismatch(
            "fixture.v1".to_string(),
        )),
        ApplicationFailureDisposition::InvalidInput
    );
    assert_eq!(
        embedded_schema_failure_disposition(&SchemaError::UnknownId("fixture.v1".to_string(),)),
        ApplicationFailureDisposition::Internal
    );
}

#[test]
fn resident_hook_failures_separate_authored_input_from_carrier_state() {
    use crate::hooks::RepositoryHookRoutingError;
    use crate::resident::{ResidentHookIngressAdmissionError, ResidentHookTransportUnavailable};

    for error in [
        RepositoryHookRoutingError::InvalidGrammar,
        RepositoryHookRoutingError::UnknownEvent,
        RepositoryHookRoutingError::InvalidProviderBindingId,
    ] {
        assert_eq!(
            error.failure_disposition(),
            ApplicationFailureDisposition::InvalidInput
        );
    }
    for error in [
        ResidentHookIngressAdmissionError::Absent,
        ResidentHookIngressAdmissionError::Malformed("fixture"),
        ResidentHookIngressAdmissionError::EventMismatch {
            expected: "SessionStart",
        },
    ] {
        assert_eq!(
            error.failure_disposition(),
            ApplicationFailureDisposition::InvalidInput
        );
    }
    assert_eq!(
        ResidentHookTransportUnavailable.failure_disposition(),
        ApplicationFailureDisposition::Blocked
    );
}
