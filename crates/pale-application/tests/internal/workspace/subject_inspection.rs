//! Composed sections preserve their owning progress and consistency laws.

use super::*;
use crate::workspace::AuthoringInstancePathSegment;

fn empty_workspace() -> PortableWorkspaceUniverse {
    use crate::workspace::{
        compile_portable_workspace, CapturedWorkspaceDocument, CapturedWorkspaceSource,
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspace-language/empty-adopter");
    let documents = [
        "pale-workspace.toml",
        "resolution.lock.toml",
        "universe.lock.toml",
    ]
    .into_iter()
    .map(|label| {
        CapturedWorkspaceDocument::new(label.to_owned(), std::fs::read(root.join(label)).unwrap())
            .unwrap()
    })
    .collect();
    compile_portable_workspace(
        CapturedWorkspaceSource::new(
            CapturedWorkspaceSource::admit_root_manifest("pale-workspace.toml".into()).unwrap(),
            documents,
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn completed_sections_keep_the_root_without_restarting_either_query() {
    let workspace = empty_workspace();
    let subject = WorkspaceDerivationSubjectV1::Workspace {
        workspace_id: pale_core::records::WorkspaceId::parse(workspace.workspace_id()).unwrap(),
    };
    let expected = workspace
        .direct_dependencies(&subject, 0, 256)
        .unwrap()
        .unwrap();
    let mut after = None;
    let mut input_digests = Vec::new();
    let mut inputs_done = false;
    let mut sources_done = false;
    for _ in 0..1000 {
        let page = workspace
            .inspect_subject(&subject, after.as_ref(), 1, 1, 1)
            .unwrap();
        assert_eq!(page.root(), Some(expected.root()));
        if inputs_done {
            assert!(page.dependency_page().is_none());
            assert_eq!(page.dependency_copied_string_bytes(), 0);
        }
        if sources_done {
            assert!(page.source_page().is_none());
            assert_eq!(page.source_copied_string_bytes(), 0);
        }
        if let Some(inputs) = page.dependency_page() {
            input_digests.extend(inputs.dependencies().map(|node| node.node_digest.clone()));
        }
        inputs_done = page.inputs_complete();
        sources_done = page.sources_complete();
        after = page.continuation().cloned();
        if after.is_none() {
            break;
        }
    }
    assert!(inputs_done && sources_done);
    assert_eq!(
        input_digests,
        expected
            .dependencies()
            .map(|node| node.node_digest.clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn owning_errors_preserve_position_source_and_reject_membership_contradictions() {
    use std::error::Error;
    let workspace = empty_workspace();
    let subject = WorkspaceDerivationSubjectV1::Workspace {
        workspace_id: pale_core::records::WorkspaceId::parse(workspace.workspace_id()).unwrap(),
    };
    // Fault injection into a private application-local position, not a cursor
    // a public SDK caller can manufacture.
    let invalid = PortableWorkspaceSubjectInspectionCursor {
        input_offset: Some(usize::MAX),
        source_after: None,
    };
    let error = workspace
        .inspect_subject(&subject, Some(&invalid), 1, 1, 1)
        .err()
        .unwrap();
    assert!(matches!(
        &error,
        PortableWorkspaceSubjectInspectionError::InvalidInputPosition(_)
    ));
    assert!(error
        .source()
        .unwrap()
        .downcast_ref::<InvalidDependencyOffset>()
        .is_some());
    // Genuine source pages combined with a deliberately contradictory root flag
    // exercise this application guard; no real compiler inconsistency is claimed.
    let selected = workspace.subject_occurrences(&subject, None, 1, 1);
    assert!(matches!(
        consistent_source_page(false, selected),
        Err(PortableWorkspaceSubjectInspectionError::MembershipIncoherent)
    ));
    let absent = WorkspaceDerivationSubjectV1::Workspace {
        workspace_id: pale_core::records::WorkspaceId::parse("not-selected").unwrap(),
    };
    let unselected = workspace.subject_occurrences(&absent, None, 1, 1);
    assert!(matches!(
        consistent_source_page(true, unselected),
        Err(PortableWorkspaceSubjectInspectionError::MembershipIncoherent)
    ));
    let page = workspace
        .inspect_subject(&absent, None, 1, 256, 256)
        .unwrap();
    assert!(page.root().is_none());
    assert!(page.inputs_complete() && page.sources_complete());
}

#[test]
fn repeated_record_anchor_is_charged_after_the_occurrence_scan_finishes() {
    let anchor = WorkspaceAuthoringLocation::new(
        "consumer.toml".into(),
        vec![
            AuthoringInstancePathSegment::Field("module".into()),
            AuthoringInstancePathSegment::Field("imports".into()),
            AuthoringInstancePathSegment::Index(0),
        ],
    );
    assert_eq!(
        subject_source_copied_string_bytes(None, Some(&anchor)),
        "consumer.toml".len() + "module".len() + "imports".len()
    );
}
