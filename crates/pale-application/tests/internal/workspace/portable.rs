//! Law tests for already-captured workspace-source admission.

use std::borrow::Cow;
use std::path::Path;

use pale_core::paths::MAX_CONFINED_RELATIVE_PATH_BYTES;
use pale_ports::SourceReadLimit;

use crate::workspace::WorkspaceSourceCoordinate;

use super::*;

#[test]
fn document_count_boundary_is_exact() {
    assert!(
        CapturedWorkspaceSource::validate_document_count(MAX_CAPTURED_WORKSPACE_DOCUMENTS).is_ok()
    );

    let error =
        CapturedWorkspaceSource::validate_document_count(MAX_CAPTURED_WORKSPACE_DOCUMENTS + 1)
            .expect_err("the first out-of-bound member must fail before conversion");
    assert!(matches!(
        error.workspace_error(),
        WorkspaceError::CapturedDocumentCountExceeded {
            observed,
            limit,
        } if *observed == MAX_CAPTURED_WORKSPACE_DOCUMENTS + 1
            && *limit == MAX_CAPTURED_WORKSPACE_DOCUMENTS
    ));
}

#[test]
fn document_bytes_and_confined_path_boundaries_are_exact() {
    let exact_path = "a".repeat(MAX_CONFINED_RELATIVE_PATH_BYTES);
    assert!(
        CapturedWorkspaceDocument::new(exact_path, vec![0; WORKSPACE_FILE_LIMIT.max_bytes()],)
            .is_ok()
    );

    let error = CapturedWorkspaceDocument::new(
        "source.toml".to_string(),
        vec![0; WORKSPACE_FILE_LIMIT.max_bytes() + 1],
    )
    .expect_err("the first byte beyond the document ceiling is rejected");
    assert!(matches!(
        error.workspace_error(),
        WorkspaceError::CapturedDocumentTooLarge {
            observed,
            limit,
        } if *observed == WORKSPACE_FILE_LIMIT.max_bytes() + 1
            && *limit == WORKSPACE_FILE_LIMIT.max_bytes()
    ));
    let error = CapturedWorkspaceDocument::new(
        "a".repeat(MAX_CONFINED_RELATIVE_PATH_BYTES + 1),
        Vec::new(),
    )
    .expect_err("the path wire ceiling is exact");
    assert!(matches!(
        error.workspace_error(),
        WorkspaceError::EscapedReference(_)
    ));

    for rejected in [
        "../source.toml",
        "/source.toml",
        "nested\\source.toml",
        "bad\0.toml",
    ] {
        let error = CapturedWorkspaceDocument::new(rejected.to_string(), Vec::new())
            .expect_err("unsafe logical labels are rejected");
        assert!(matches!(
            error.workspace_error(),
            WorkspaceError::EscapedReference(_)
        ));
    }
}

#[test]
fn root_manifest_admission_is_owned_bounded_and_preconvertible() {
    let exact = "a".repeat(MAX_CONFINED_RELATIVE_PATH_BYTES);
    assert_eq!(
        CapturedWorkspaceSource::admit_root_manifest(exact.clone())
            .expect("the exact root path wire ceiling is admitted")
            .as_str(),
        exact
    );
    let oversized = "a".repeat(MAX_CONFINED_RELATIVE_PATH_BYTES + 1);
    let error = CapturedWorkspaceSource::admit_root_manifest(oversized)
        .expect_err("the root path wire ceiling is exact");
    assert!(matches!(
        error.workspace_error(),
        WorkspaceError::InvalidCapturedSourceRoot
    ));
    for rejected in [
        "../root.toml",
        "/root.toml",
        "nested\\root.toml",
        "bad\0.toml",
    ] {
        let error = CapturedWorkspaceSource::admit_root_manifest(rejected.to_string())
            .expect_err("unsafe root labels are rejected before document conversion");
        assert!(matches!(
            error.workspace_error(),
            WorkspaceError::InvalidCapturedSourceRoot
        ));
    }
}

#[test]
fn captured_source_reads_borrow_the_original_body() {
    let body = b"captured-body".to_vec();
    let source = CapturedWorkspaceSource::new(
        CapturedWorkspaceSource::admit_root_manifest("nested/root.toml".to_string())
            .expect("the nested root is confined"),
        vec![
            CapturedWorkspaceDocument::new("nested/root.toml".to_string(), body.clone())
                .expect("nested confined paths are valid"),
        ],
    )
    .expect("the selected root is present");

    let read = WorkspaceDocumentSource::read_regular(
        &source,
        Path::new("nested/root.toml"),
        WORKSPACE_FILE_LIMIT,
    )
    .expect("the captured member is available");
    assert!(matches!(read, Cow::Borrowed(bytes) if bytes == body));

    let error = WorkspaceDocumentSource::read_regular(
        &source,
        Path::new("nested/root.toml"),
        SourceReadLimit::new(body.len() - 1),
    )
    .expect_err("a narrower owning read ceiling rejects the complete member");
    assert!(matches!(
        error,
        WorkspaceError::CapturedDocumentTooLarge { observed, limit }
            if observed == body.len() && limit == body.len() - 1
    ));
}

#[test]
fn portable_schema_failures_preserve_selected_contract_ownership() {
    use crate::ApplicationFailureDisposition::{Internal, InvalidInput};
    use pale_contracts::SchemaError;

    let value = serde_json::json!({});
    let invalid = pale_contracts::validate_value(&value, "pale-workspace.v1").unwrap_err();
    assert!(matches!(invalid, SchemaError::Invalid(_)));
    let unknown = pale_contracts::validate_value(&value, "not-a-selected-contract.v1").unwrap_err();
    assert!(matches!(unknown, SchemaError::UnknownId(_)));
    let definition = pale_contracts::validate_definition(
        &value,
        "capability-procedure.v1",
        "NotAnEmbeddedDefinition",
    )
    .unwrap_err();
    assert!(matches!(definition, SchemaError::UnknownDefinition { .. }));

    // These supplied errors test classification, not corruption of this binary.
    let encode =
        serde_json::to_value(std::collections::BTreeMap::from([(vec![1, 2], 0)])).unwrap_err();
    let parse = pale_core::canon::parse_strict(b"not-json").unwrap_err();
    for (schema_error, disposition, class) in [
        (
            invalid,
            InvalidInput,
            PortableWorkspaceFailureClass::ContractRejected,
        ),
        (
            unknown,
            Internal,
            PortableWorkspaceFailureClass::InternalInvariant,
        ),
        (
            definition,
            Internal,
            PortableWorkspaceFailureClass::InternalInvariant,
        ),
        (
            SchemaError::IdentityMismatch("selected.v1".into()),
            InvalidInput,
            PortableWorkspaceFailureClass::ContractRejected,
        ),
        (
            SchemaError::Integrity("injected registry failure".into()),
            Internal,
            PortableWorkspaceFailureClass::InternalInvariant,
        ),
        (
            SchemaError::Encode(encode),
            Internal,
            PortableWorkspaceFailureClass::InternalInvariant,
        ),
        (
            SchemaError::Parse(parse),
            Internal,
            PortableWorkspaceFailureClass::InternalInvariant,
        ),
    ] {
        let error = WorkspaceError::SchemaCheck {
            coordinate: WorkspaceSourceCoordinate::path_only(Path::new("root.toml")),
            source: Box::new(schema_error),
        };
        assert_eq!(error.failure_disposition(), disposition);
        assert_eq!(
            PortableWorkspaceFailure::from_workspace_error(error).class(),
            class
        );
    }
}

#[test]
fn portable_failure_algebra_is_closed_and_private_errors_fail_internal() {
    let contract_error =
        pale_contracts::validate_value(&serde_json::json!({}), "pale-workspace.v1")
            .expect_err("an empty object violates the workspace root contract");
    let cases = vec![
        (
            WorkspaceError::TomlToJson {
                coordinate: WorkspaceSourceCoordinate::path_only(Path::new("root.toml")),
                source: serde_json::to_value(std::collections::BTreeMap::from([(vec![1, 2], 0)]))
                    .expect_err("a sequence key cannot be represented as a JSON object key"),
            },
            PortableWorkspaceFailureClass::InternalInvariant,
        ),
        (
            WorkspaceError::GeneratedRecordSchema(pale_contracts::SchemaError::Integrity(
                "generated record invariant".into(),
            )),
            PortableWorkspaceFailureClass::InternalInvariant,
        ),
        (
            WorkspaceError::InvalidWorkspaceModule(
                pale_core::records::InvalidWorkspaceModule::DigestMismatch,
            ),
            PortableWorkspaceFailureClass::SemanticRejected,
        ),
        (
            WorkspaceError::EscapedReference("../source.toml".to_string()),
            PortableWorkspaceFailureClass::SourceRejected,
        ),
        (
            WorkspaceError::InvalidCapturedSourceRoot,
            PortableWorkspaceFailureClass::SourceRejected,
        ),
        (
            WorkspaceError::SchemaCheck {
                coordinate: WorkspaceSourceCoordinate::path_only(Path::new("root.toml")),
                source: Box::new(contract_error),
            },
            PortableWorkspaceFailureClass::ContractRejected,
        ),
        (
            WorkspaceError::NamespaceResolution("missing namespace".to_string()),
            PortableWorkspaceFailureClass::SemanticRejected,
        ),
        (
            WorkspaceError::WorkspaceGraphLimit {
                kind: "modules",
                observed: 2,
                limit: 1,
            },
            PortableWorkspaceFailureClass::SemanticLimitExceeded,
        ),
        (
            WorkspaceError::SourceDeliveryUnavailable {
                module: "consumer".to_string(),
                component: "source".to_string(),
            },
            PortableWorkspaceFailureClass::SourceIncomplete,
        ),
        (
            WorkspaceError::PublisherAdmissionUnavailable {
                publisher: "publisher".to_string(),
                release: "release".to_string(),
            },
            PortableWorkspaceFailureClass::OwnerProofUnavailable,
        ),
        (
            WorkspaceError::ProtectedDeliveryAdmissionUnavailable {
                importer: "consumer".to_string(),
                path: "operations.review".to_string(),
            },
            PortableWorkspaceFailureClass::OwnerProofUnavailable,
        ),
        (
            WorkspaceError::ReleaseStateNotAdmissible {
                module: "module".to_string(),
                release: "release".to_string(),
            },
            PortableWorkspaceFailureClass::ReleaseNotAdmissible,
        ),
        (
            WorkspaceError::PortableObservationOverflow {
                kind: "observations",
                observed: usize::MAX,
            },
            PortableWorkspaceFailureClass::InternalInvariant,
        ),
        (
            WorkspaceError::SurfaceExecutableMissing,
            PortableWorkspaceFailureClass::InternalInvariant,
        ),
    ];
    for (source, expected) in cases {
        if matches!(
            &source,
            WorkspaceError::TomlToJson { .. } | WorkspaceError::GeneratedRecordSchema(_)
        ) {
            assert_eq!(
                source.failure_disposition(),
                crate::ApplicationFailureDisposition::Internal
            );
        }
        assert_eq!(
            PortableWorkspaceFailure::from_workspace_error(source).class(),
            expected
        );
    }
}
