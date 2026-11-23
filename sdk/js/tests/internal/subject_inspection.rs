//! Exact SDK bytes and owned projections from the Node-local portable fixture.

use crate::{PortableWorkspaceCompilation, PortableWorkspaceSubjectInspection};
use pale_sdk::{PaleApi, PortableWorkspaceDocument, PortableWorkspaceRequest};

#[test]
fn subject_inspection_copies_exact_sdk_pages_and_retains_terminal_outputs() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/cross-leaf.v1/portable");
    let mut paths = std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    paths.sort();
    let documents = paths
        .into_iter()
        .map(|path| {
            PortableWorkspaceDocument::new(
                path.file_name().unwrap().to_str().unwrap().to_owned(),
                std::fs::read(path).unwrap(),
            )
        })
        .collect();
    let api = PaleApi::new().unwrap();
    let workspace = PortableWorkspaceCompilation {
        inner: api
            .compile_portable_workspace(PortableWorkspaceRequest::new(
                "pale-workspace.toml".into(),
                documents,
            ))
            .unwrap(),
    };
    let comparison = api
        .compare_portable_workspaces(&workspace.inner, &workspace.inner)
        .unwrap();
    let subjects = comparison
        .impacts(0, 256)
        .entries()
        .iter()
        .map(|impact| impact.subject().clone())
        .collect::<Vec<_>>();
    assert_eq!(
        subjects.len(),
        9,
        "the existing empty-adopter fixture has nine facts"
    );
    drop(comparison);
    let mut retained = Vec::new();
    for subject in subjects {
        let mut after = None;
        loop {
            let expected = workspace
                .inner
                .inspect_subject(&subject, after.as_ref(), 1, 1, 1)
                .unwrap();
            let bytes = expected.canonical_bytes().to_vec();
            let root = expected.root().cloned();
            let dependencies = expected.dependencies().to_vec();
            let occurrences = expected.occurrences().to_vec();
            let inspected = expected.inspected();
            let inputs_complete = expected.inputs_complete();
            let sources_complete = expected.sources_complete();
            after = expected.continuation().cloned();
            let page = PortableWorkspaceSubjectInspection { inner: expected };
            assert_eq!(page.canonical_bytes().as_ref(), bytes.as_slice());
            assert_eq!(page.subject().inner, subject);
            assert_eq!(page.workspace_id(), workspace.inner.workspace_id());
            assert_eq!(
                page.principal_universe_digest(),
                workspace.inner.principal_universe_digest()
            );
            assert_eq!(
                page.semantic_ir_digest(),
                workspace.inner.semantic_ir_digest()
            );
            assert_eq!(
                page.admission_evidence_digest(),
                workspace.inner.admission_evidence_digest()
            );
            assert_eq!(
                page.root().as_ref().map(|value| value.node_digest()),
                root.as_ref().map(|value| value.node_digest().to_owned())
            );
            assert_eq!(
                page.dependencies()
                    .iter()
                    .map(|value| value.node_digest())
                    .collect::<Vec<_>>(),
                dependencies
                    .iter()
                    .map(|value| value.node_digest().to_owned())
                    .collect::<Vec<_>>()
            );
            assert_eq!(page.inspected(), inspected);
            assert_eq!(page.inputs_complete(), inputs_complete);
            assert_eq!(page.sources_complete(), sources_complete);
            assert_eq!(page.is_complete(), after.is_none());
            assert_eq!(page.continuation().is_none(), after.is_none());
            assert!(page.belongs_to(&workspace));
            assert!(page.record_anchor().unwrap().is_none());
            let projected = page.occurrences().unwrap();
            assert_eq!(projected.len(), occurrences.len());
            for (actual, expected) in projected.iter().zip(&occurrences) {
                assert_eq!(
                    actual.location.source_label,
                    expected.location().source_label()
                );
                assert_eq!(actual.location.path.len(), expected.location().path().len());
                for (actual, expected) in
                    actual.location.path.iter().zip(expected.location().path())
                {
                    match expected {
                        pale_sdk::AuthoringInstancePathSegment::Field(field) => {
                            assert_eq!(actual.field.as_ref(), Some(field));
                            assert_eq!(actual.index, None);
                        }
                        pale_sdk::AuthoringInstancePathSegment::Index(index) => {
                            assert_eq!(actual.index, Some(u32::try_from(*index).unwrap()));
                            assert_eq!(actual.field, None);
                        }
                    }
                }
                assert!(matches!(
                    (&actual.role, expected.role()),
                    (
                        crate::AuthoringOccurrenceRole::Definition,
                        pale_sdk::AuthoringOccurrenceRole::Definition
                    ) | (
                        crate::AuthoringOccurrenceRole::Reference,
                        pale_sdk::AuthoringOccurrenceRole::Reference
                    ) | (
                        crate::AuthoringOccurrenceRole::Projection,
                        pale_sdk::AuthoringOccurrenceRole::Projection
                    )
                ));
            }
            let mut copied = page.canonical_bytes();
            copied.as_mut().fill(0);
            assert_eq!(page.canonical_bytes().as_ref(), bytes.as_slice());
            if after.is_none() {
                retained.push((page, bytes));
                break;
            }
        }
    }
    drop(workspace);
    drop(api);
    for (page, bytes) in retained {
        assert!(page.is_complete());
        assert!(page.continuation().is_none());
        assert!(page.root().is_some());
        assert_eq!(page.canonical_bytes().as_ref(), bytes.as_slice());
    }
}
