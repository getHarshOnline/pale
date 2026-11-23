//! Verifies that structural-query failures use the canonical API diagnostic owner.

use super::{authoring_query_failure, AuthoringMetadataFailure, AuthoringQueryError};
use crate::{DiagnosticDisposition, DiagnosticRetry, DiagnosticStage};

fn identity() -> pale_core::records::RecordSchemaIdentity {
    pale_contracts::record_schema_identity("diagnostic.v1")
        .expect("diagnostic identity is embedded")
}

#[test]
fn path_bounds_and_metadata_invariants_have_distinct_canonical_policy() {
    for (error, code, stage, disposition, retry) in [
        (
            AuthoringQueryError::ContinuationMismatch,
            "authoring_continuation_mismatch",
            DiagnosticStage::InputAdmission,
            DiagnosticDisposition::InvalidInput,
            DiagnosticRetry::AfterCorrection,
        ),
        (
            AuthoringQueryError::PathTooDeep {
                observed: 65,
                maximum: 64,
            },
            "authoring_path_too_deep",
            DiagnosticStage::InputAdmission,
            DiagnosticDisposition::InvalidInput,
            DiagnosticRetry::AfterCorrection,
        ),
        (
            AuthoringQueryError::Metadata(AuthoringMetadataFailure::Registry),
            "authoring_metadata_incoherent",
            DiagnosticStage::Internal,
            DiagnosticDisposition::InternalFailure,
            DiagnosticRetry::NotIndicated,
        ),
    ] {
        let diagnostic = authoring_query_failure(&identity(), error);
        assert_eq!(diagnostic.diagnostic().code(), code);
        assert_eq!(diagnostic.diagnostic().stage(), stage);
        assert_eq!(diagnostic.diagnostic().disposition(), disposition);
        assert_eq!(diagnostic.diagnostic().retry(), retry);
    }
}

#[test]
fn authoring_bounds_carry_bounded_recovery_without_exposing_continuations() {
    let continuation =
        authoring_query_failure(&identity(), AuthoringQueryError::ContinuationMismatch);
    let diagnostic = continuation.diagnostic();
    assert_eq!(diagnostic.facts(), Vec::new());
    assert_eq!(
        diagnostic.help(),
        [
            "Restart the authoring query without the prior continuation, then continue only from the new page."
        ]
    );

    let path = authoring_query_failure(
        &identity(),
        AuthoringQueryError::PathTooDeep {
            observed: 65,
            maximum: 64,
        },
    );
    let diagnostic = path.diagnostic();
    assert_eq!(
        diagnostic.help(),
        ["Reduce the authoring path to the stated maximum, then retry."]
    );
    assert_eq!(
        diagnostic
            .facts()
            .into_iter()
            .map(|fact| (fact.key().to_owned(), fact.value().to_owned()))
            .collect::<Vec<_>>(),
        [
            ("unit".to_owned(), "segments".to_owned()),
            ("observed".to_owned(), "65".to_owned()),
            ("maximum".to_owned(), "64".to_owned()),
        ]
    );
}

fn empty_workspace() -> (crate::PaleApi, crate::PortableWorkspaceCompilation) {
    empty_workspace_with_comments("")
}

fn empty_workspace_with_comments(
    comment: &str,
) -> (crate::PaleApi, crate::PortableWorkspaceCompilation) {
    use crate::{PaleApi, PortableWorkspaceDocument, PortableWorkspaceRequest};

    let documents = [
        (
            "pale-workspace.toml",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/workspace-language/empty-adopter/pale-workspace.toml"
            ))
            .as_slice(),
        ),
        (
            "resolution.lock.toml",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/workspace-language/empty-adopter/resolution.lock.toml"
            ))
            .as_slice(),
        ),
        (
            "universe.lock.toml",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/workspace-language/empty-adopter/universe.lock.toml"
            ))
            .as_slice(),
        ),
    ]
    .into_iter()
    .map(|(label, bytes)| {
        let mut bytes = bytes.to_vec();
        bytes.extend_from_slice(comment.as_bytes());
        PortableWorkspaceDocument::new(label.into(), bytes)
    })
    .collect();
    let api = PaleApi::new().unwrap();
    let compilation = api
        .compile_portable_workspace(PortableWorkspaceRequest::new(
            "pale-workspace.toml".into(),
            documents,
        ))
        .unwrap();
    (api, compilation)
}

#[test]
fn comparison_subject_pages_and_cursor_copies_release_both_input_owners() {
    use std::sync::Arc;
    for terminal in [true, false] {
        let (api, prior) = empty_workspace();
        let (other_api, current) = empty_workspace_with_comments("\n# captured source changed\n");
        let prior_owner = Arc::downgrade(&prior.application_universe_handle());
        let current_owner = Arc::downgrade(&current.application_universe_handle());
        let comparison = api.compare_portable_workspaces(&prior, &current).unwrap();
        let page = comparison
            .subjects(None, if terminal { 256 } else { 1 })
            .unwrap();
        assert_eq!(page.continuation().is_none(), terminal);
        let copy = page.clone();
        let cursor = page.continuation().cloned();
        let cursor_copy = cursor.clone();
        drop(prior);
        drop(current);
        drop(comparison);
        drop(api);
        drop(other_api);
        drop(page);
        assert!(prior_owner.upgrade().is_some());
        assert!(current_owner.upgrade().is_some());
        drop(copy);
        assert_eq!(prior_owner.upgrade().is_some(), !terminal);
        assert_eq!(current_owner.upgrade().is_some(), !terminal);
        drop(cursor);
        assert_eq!(prior_owner.upgrade().is_some(), !terminal);
        assert_eq!(current_owner.upgrade().is_some(), !terminal);
        drop(cursor_copy);
        assert!(prior_owner.upgrade().is_none());
        assert!(current_owner.upgrade().is_none());
    }
}

#[test]
fn contributing_root_results_and_cursor_copies_release_both_input_owners() {
    use crate::{PortableWorkspaceSubject, PortableWorkspaceSubjectSelector};
    use std::sync::Arc;
    for terminal in [true, false] {
        let (api, prior) = empty_workspace();
        let (other_api, current) = empty_workspace_with_comments("\n# captured source changed\n");
        let prior_owner = Arc::downgrade(&prior.application_universe_handle());
        let current_owner = Arc::downgrade(&current.application_universe_handle());
        let comparison = api.compare_portable_workspaces(&prior, &current).unwrap();
        let subject =
            PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::UniverseLock)
                .unwrap();
        let result = comparison
            .contributing_roots(&subject, None, if terminal { 256 } else { 1 })
            .unwrap();
        assert_eq!(result.continuation().is_none(), terminal);
        let copy = result.clone();
        let cursor = result.continuation().cloned();
        let cursor_copy = cursor.clone();
        drop(prior);
        drop(current);
        drop(comparison);
        drop(api);
        drop(other_api);
        drop(result);
        assert!(prior_owner.upgrade().is_some());
        assert!(current_owner.upgrade().is_some());
        drop(copy);
        assert_eq!(prior_owner.upgrade().is_some(), !terminal);
        assert_eq!(current_owner.upgrade().is_some(), !terminal);
        drop(cursor);
        assert_eq!(prior_owner.upgrade().is_some(), !terminal);
        assert_eq!(current_owner.upgrade().is_some(), !terminal);
        drop(cursor_copy);
        assert!(prior_owner.upgrade().is_none());
        assert!(current_owner.upgrade().is_none());
    }
}

#[test]
fn occurrence_cursor_keeps_the_compiler_alive_until_its_last_copy_is_dropped() {
    use crate::{AuthoringNavigationRequest, AuthoringOccurrencePageRequest};
    use std::sync::Arc;

    let (api, compilation) = empty_workspace();
    let owner = Arc::downgrade(&compilation.application_universe_handle());
    let identity = compilation
        .navigate_authoring(AuthoringNavigationRequest::new(
            "pale-workspace.toml".into(),
            Vec::new(),
        ))
        .identity()
        .unwrap()
        .clone();
    let page = compilation
        .authoring_occurrences(AuthoringOccurrencePageRequest::new(identity, 1, 1))
        .unwrap();
    let cursor = page.continuation().unwrap().clone();
    let cursor_copy = cursor.clone();
    drop(page);
    drop(compilation);
    drop(api);
    assert!(
        owner.upgrade().is_some(),
        "only cursor copies now retain the compiler"
    );
    drop(cursor);
    assert!(
        owner.upgrade().is_some(),
        "the last cursor copy still owns its continuation"
    );
    drop(cursor_copy);
    assert!(
        owner.upgrade().is_none(),
        "there is no hidden global owner or leaked cursor"
    );
}

#[test]
fn direct_dependent_results_and_cursor_copies_retain_the_exact_owner_until_last_drop() {
    use crate::{PortableWorkspaceSubject, PortableWorkspaceSubjectSelector};
    use std::sync::Arc;
    for mode in ["pending", "terminal", "unselected"] {
        let (api, compilation) = empty_workspace();
        let owner = Arc::downgrade(&compilation.application_universe_handle());
        let subject =
            PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Workspace {
                workspace_id: if mode == "unselected" {
                    "absent-workspace".into()
                } else {
                    compilation.workspace_id().into()
                },
            })
            .unwrap();
        let page = compilation
            .direct_dependents(&subject, None, if mode == "pending" { 1 } else { 256 }, 256)
            .unwrap();
        assert_eq!(page.is_complete(), mode != "pending");
        assert_eq!(page.root().is_none(), mode == "unselected");
        let copy = page.clone();
        let cursor = page.continuation().cloned();
        let cursor_copy = cursor.clone();
        drop(compilation);
        drop(api);
        drop(page);
        assert!(owner.upgrade().is_some());
        drop(copy);
        assert_eq!(owner.upgrade().is_some(), mode == "pending");
        drop(cursor);
        assert_eq!(owner.upgrade().is_some(), mode == "pending");
        drop(cursor_copy);
        assert!(owner.upgrade().is_none());
    }
}

#[test]
fn source_inspection_and_subject_cursor_release_their_last_compilation_owner() {
    use crate::{
        AuthoringNavigationRequest, PortableWorkspaceSourceSubject,
        PortableWorkspaceSubjectOccurrences,
    };
    use std::sync::Arc;

    let (api, compilation) = empty_workspace();
    let owner = Arc::downgrade(&compilation.application_universe_handle());
    let inspection = compilation
        .source_subject(AuthoringNavigationRequest::new(
            "pale-workspace.toml".into(),
            Vec::new(),
        ))
        .unwrap();
    let copy = inspection.clone();
    drop(compilation);
    drop(api);
    drop(inspection);
    assert!(owner.upgrade().is_some());
    assert!(matches!(
        copy.result(),
        PortableWorkspaceSourceSubject::Selected { .. }
    ));
    drop(copy);
    assert!(owner.upgrade().is_none());

    let (api, compilation) = empty_workspace();
    let owner = Arc::downgrade(&compilation.application_universe_handle());
    let inspection = compilation
        .source_subject(AuthoringNavigationRequest::new(
            "pale-workspace.toml".into(),
            Vec::new(),
        ))
        .unwrap();
    let PortableWorkspaceSourceSubject::Selected { subject, .. } = inspection.result() else {
        panic!()
    };
    let PortableWorkspaceSubjectOccurrences::Selected(page) = compilation
        .subject_occurrences(subject, None, 1, 1)
        .unwrap()
    else {
        panic!()
    };
    let cursor = page.continuation().unwrap().clone();
    drop(page);
    drop(inspection);
    drop(compilation);
    drop(api);
    assert!(owner.upgrade().is_some());
    drop(cursor);
    assert!(owner.upgrade().is_none());
}

#[test]
fn composed_inspection_retains_terminal_results_and_independent_cursor_copies() {
    use crate::{PortableWorkspaceSubject, PortableWorkspaceSubjectSelector};
    use std::sync::Arc;
    for complete in [true, false] {
        let (api, compilation) = empty_workspace();
        let owner = Arc::downgrade(&compilation.application_universe_handle());
        let subject =
            PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Workspace {
                workspace_id: compilation.workspace_id().to_owned(),
            })
            .unwrap();
        let limit = if complete { 256 } else { 1 };
        let result = compilation
            .inspect_subject(&subject, None, limit, limit, limit)
            .unwrap();
        assert_eq!(result.is_complete(), complete);
        let copy = result.clone();
        let cursor = result.continuation().cloned();
        let cursor_copy = cursor.clone();
        drop(compilation);
        drop(api);
        drop(result);
        assert!(
            owner.upgrade().is_some(),
            "the result copy retains terminal or pending custody"
        );
        drop(copy);
        assert_eq!(owner.upgrade().is_some(), !complete);
        drop(cursor);
        assert_eq!(owner.upgrade().is_some(), !complete);
        drop(cursor_copy);
        assert!(
            owner.upgrade().is_none(),
            "last result or cursor releases its owner"
        );
    }
    let (api, compilation) = empty_workspace();
    let owner = Arc::downgrade(&compilation.application_universe_handle());
    let subject = PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Workspace {
        workspace_id: compilation.workspace_id().to_owned(),
    })
    .unwrap();
    let result = compilation
        .inspect_subject(&subject, None, 1, 1, 1)
        .unwrap();
    drop(compilation);
    drop(api);
    assert!(
        owner.upgrade().is_some(),
        "pending result itself retains its owner"
    );
    drop(result);
    assert!(owner.upgrade().is_none());
}

#[test]
fn inspection_record_location_mapper_preserves_structured_error_coordinates() {
    use pale_application::workspace::{
        compile_portable_workspace, CapturedWorkspaceDocument, CapturedWorkspaceSource,
    };
    // An actual rejected-input location, not successful public module admission.
    // The shared mapper must preserve the application's field/index distinction.
    let mut root: toml::Value = toml::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/workspace-language/empty-adopter/pale-workspace.toml"
    )))
    .unwrap();
    root["workspace"]["module_manifests"] = toml::Value::Array(vec![
        toml::Value::String("not-loaded.toml".into()),
        toml::Value::Integer(7),
    ]);
    let error = compile_portable_workspace(
        CapturedWorkspaceSource::new(
            CapturedWorkspaceSource::admit_root_manifest("原本.toml".into()).unwrap(),
            vec![CapturedWorkspaceDocument::new(
                "原本.toml".into(),
                toml::to_string(&root).unwrap().into_bytes(),
            )
            .unwrap()],
        )
        .unwrap(),
    )
    .unwrap_err();
    let location = error.workspace_error().authoring_location().unwrap();
    let copied = super::AuthoringSourceLocation::from_application(&location);
    drop(error);
    drop(location);
    assert_eq!(copied.source_label(), "原本.toml");
    assert_eq!(
        copied.path(),
        &[
            super::AuthoringInstancePathSegment::Field("workspace".into()),
            super::AuthoringInstancePathSegment::Field("module_manifests".into()),
            super::AuthoringInstancePathSegment::Index(1),
        ]
    );
}

#[path = "authoring/pages.rs"]
mod pages;
