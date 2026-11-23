//! Union discovery supplies identities; the existing point query owns their values.

use super::*;
use pale_sdk::{PortableWorkspaceComparison, PortableWorkspaceComparisonSubjectPage};
use std::collections::BTreeSet;

fn identity_bytes(subject: &PortableWorkspaceSubject) -> usize {
    [
        subject.source_id(),
        subject.component_id(),
        subject.module_id(),
        subject.owner_module_id(),
        subject.declaration_namespace_id(),
        subject.declaration_id(),
        subject.importer_module_id(),
        subject.upstream_module_id(),
        subject.upstream_release_id(),
        subject.export_id(),
        subject.source_namespace_id(),
        subject.local_namespace_id(),
        subject.extension_namespace_id(),
        subject.resource_id(),
        subject.workspace_id(),
    ]
    .into_iter()
    .flatten()
    .map(str::len)
    .sum()
}

fn check_projection(
    page: &PortableWorkspaceComparisonSubjectPage,
    owner: &PortableWorkspaceComparison,
) {
    let value: serde_json::Value = serde_json::from_slice(page.canonical_bytes()).unwrap();
    let mut keys = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "currentPrincipalUniverseDigest",
            "currentSemanticIrDigest",
            "inspected",
            "isComplete",
            "keyComparisons",
            "priorPrincipalUniverseDigest",
            "priorSemanticIrDigest",
            "receiptDigest",
            "subjectIdentityBytes",
            "subjects"
        ]
    );
    assert_eq!(value["receiptDigest"], owner.receipt_digest());
    assert_eq!(
        value["priorPrincipalUniverseDigest"],
        owner.prior_principal_universe_digest()
    );
    assert_eq!(
        value["currentPrincipalUniverseDigest"],
        owner.current_principal_universe_digest()
    );
    assert_eq!(
        value["priorSemanticIrDigest"],
        owner.prior_semantic_ir_digest()
    );
    assert_eq!(
        value["currentSemanticIrDigest"],
        owner.current_semantic_ir_digest()
    );
    assert_eq!(page.comparison(), &owner.summary());
    assert!(page.belongs_to(owner));
    assert_eq!(value["inspected"], page.inspected());
    assert_eq!(value["keyComparisons"], page.key_comparisons());
    assert_eq!(value["subjectIdentityBytes"], page.subject_identity_bytes());
    assert_eq!(
        page.subject_identity_bytes(),
        page.subjects().iter().map(identity_bytes).sum::<usize>()
    );
    assert!(page.subject_identity_bytes() <= pale_sdk::MAX_COMPARISON_SUBJECT_PAGE_IDENTITY_BYTES);
    assert_eq!(value["isComplete"], page.continuation().is_none());
    let rows = value["subjects"].as_array().unwrap();
    assert_eq!(rows.len(), page.subjects().len());
    for (subject, row) in page.subjects().iter().zip(rows) {
        let detail = owner.compare_subject(subject);
        assert!(!matches!(
            detail.values(),
            PortableWorkspaceSubjectValues::Absent
        ));
        let detail_json: serde_json::Value =
            serde_json::from_slice(detail.canonical_bytes()).unwrap();
        assert_eq!(*row, detail_json["subject"]);
    }
    assert_eq!(
        pale_core::canon::to_canonical_string(&value)
            .unwrap()
            .as_bytes(),
        page.canonical_bytes()
    );
}

#[test]
fn comparison_subjects_discover_prior_only_workspace_without_fabricating_impact() {
    use pale_core::records::{WorkspaceId, WorkspaceManifestFile, WorkspaceUniverseLockV1};
    let capability = pale_sdk::capability_inventory()
        .iter()
        .find(|entry| entry.id() == pale_sdk::ApiCapabilityId::PortableWorkspaceComparisonSubjects)
        .unwrap();
    assert_eq!(
        capability.eligible_leaves(),
        &[
            pale_sdk::ApiLeaf::Rust,
            pale_sdk::ApiLeaf::Node,
            pale_sdk::ApiLeaf::Python
        ]
    );
    let api = api();
    let prior = api.compile_portable_workspace(empty_request()).unwrap();
    let mut manifest: WorkspaceManifestFile =
        toml::from_str(std::str::from_utf8(ROOT).unwrap()).unwrap();
    manifest.workspace.workspace_id = WorkspaceId::parse("successor-workspace").unwrap();
    let original: WorkspaceUniverseLockV1 =
        toml::from_str(std::str::from_utf8(UNIVERSE_LOCK).unwrap()).unwrap();
    let universe = WorkspaceUniverseLockV1::seal(
        manifest.workspace.workspace_id.clone(),
        original.resolution_digest,
        original.adopted_releases,
    )
    .unwrap();
    let current = api
        .compile_portable_workspace(PortableWorkspaceRequest::new(
            "pale-workspace.toml".into(),
            vec![
                PortableWorkspaceDocument::new(
                    "pale-workspace.toml".into(),
                    toml::to_string(&manifest).unwrap().into_bytes(),
                ),
                PortableWorkspaceDocument::new(
                    "resolution.lock.toml".into(),
                    RESOLUTION_LOCK.to_vec(),
                ),
                PortableWorkspaceDocument::new(
                    "universe.lock.toml".into(),
                    toml::to_string(&universe).unwrap().into_bytes(),
                ),
            ],
        ))
        .expect("the typed successor uses no module or publisher bypass");
    let forward = api.compare_portable_workspaces(&prior, &current).unwrap();
    let reverse = api.compare_portable_workspaces(&current, &prior).unwrap();
    let expected = [&prior, &current]
        .into_iter()
        .flat_map(|owner| {
            // Separate self-comparisons supply a test oracle only.
            all_impact_subjects(&api.compare_portable_workspaces(owner, owner).unwrap())
                .into_iter()
                .map(|(subject, _)| subject)
        })
        .collect::<BTreeSet<_>>();
    let old = PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Workspace {
        workspace_id: prior.workspace_id().into(),
    })
    .unwrap();
    assert!(matches!(
        forward.compare_subject(&old).values(),
        PortableWorkspaceSubjectValues::PriorOnly { .. }
    ));
    assert!(matches!(
        reverse.compare_subject(&old).values(),
        PortableWorkspaceSubjectValues::CurrentOnly { .. }
    ));
    assert!(!all_impact_subjects(&forward)
        .iter()
        .any(|(subject, _)| subject == &old));
    for comparison in [&forward, &reverse] {
        for limit in [0, 1, 2, 257, u16::MAX] {
            let mut cursor = None;
            let mut observed = Vec::new();
            loop {
                let page = comparison.subjects(cursor.as_ref(), limit).unwrap();
                check_projection(&page, comparison);
                assert!(page.inspected() <= 2 * page.subjects().len() + 2);
                assert!(page.key_comparisons() <= page.subjects().len());
                assert!(!page.subjects().is_empty());
                observed.extend_from_slice(page.subjects());
                cursor = page.continuation().cloned();
                if cursor.is_none() {
                    break;
                }
            }
            assert_eq!(observed, expected.iter().cloned().collect::<Vec<_>>());
            assert!(observed.iter().any(|subject| matches!(
                comparison.compare_subject(subject).values(),
                PortableWorkspaceSubjectValues::Shared {
                    comparison: PortableWorkspaceValueComparison::Changed,
                    ..
                }
            )));
        }
    }
    drop(prior);
    drop(current);
    drop(api);
    assert!(forward.subjects(None, 1).unwrap().belongs_to(&forward));
}

#[test]
fn comparison_subjects_bind_raw_limits_and_retained_pair_without_eager_values() {
    let api = api();
    let prior = api
        .compile_portable_workspace(wide_dependency_request())
        .unwrap();
    let current = api
        .compile_portable_workspace(inputs::wide_dependency_request_with_comments(
            "\n# changed capture\n",
        ))
        .unwrap();
    let rebuilt = api
        .compile_portable_workspace(inputs::wide_dependency_request_with_comments(
            "\n# changed capture\n",
        ))
        .unwrap();
    assert_eq!(prior.semantic_ir_digest(), current.semantic_ir_digest());
    let comparison = api.compare_portable_workspaces(&prior, &current).unwrap();
    let same_pair = api.compare_portable_workspaces(&prior, &current).unwrap();
    let foreign = api.compare_portable_workspaces(&prior, &rebuilt).unwrap();
    let reversed = api.compare_portable_workspaces(&current, &prior).unwrap();
    for raw in [0, 257] {
        let first = comparison.subjects(None, raw).unwrap();
        assert!(first.subjects().len() <= if raw == 0 { 64 } else { 256 });
        assert_eq!(
            first.subject_identity_bytes(),
            first.subjects().iter().map(identity_bytes).sum::<usize>()
        );
        assert!(
            first.subject_identity_bytes() <= pale_sdk::MAX_COMPARISON_SUBJECT_PAGE_IDENTITY_BYTES
        );
        if raw == 257 {
            assert!(
                first.subjects().len() < 256,
                "the identity-byte ceiling must stop this wide page before its row ceiling"
            );
        }
        let cursor = first.continuation().unwrap();
        for (owner, limit) in [
            (&comparison, if raw == 0 { 64 } else { u16::MAX }),
            (&foreign, raw),
            (&reversed, raw),
        ] {
            let error = owner.subjects(Some(cursor), limit).unwrap_err();
            assert_eq!(
                error.diagnostic().code(),
                "workspace_comparison_subject_continuation_mismatch"
            );
            assert_eq!(
                error.diagnostic().disposition(),
                DiagnosticDisposition::InvalidInput
            );
        }
        assert!(!first.belongs_to(&foreign));
        let next = comparison.subjects(Some(cursor), raw).unwrap();
        let reused = same_pair.subjects(Some(cursor), raw).unwrap();
        assert_eq!(next.canonical_bytes(), reused.canonical_bytes());
        assert!(first.belongs_to(&same_pair));
        assert_eq!(
            format!("{first:?}"),
            "PortableWorkspaceComparisonSubjectPage { .. }"
        );
        assert_eq!(
            format!("{cursor:?}"),
            "PortableWorkspaceComparisonSubjectCursor { .. }"
        );
    }
    let workspace =
        PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Workspace {
            workspace_id: current.workspace_id().into(),
        })
        .unwrap();
    let discovered = {
        let mut cursor = None;
        loop {
            let page = comparison.subjects(cursor.as_ref(), 256).unwrap();
            if let Some(found) = page
                .subjects()
                .iter()
                .find(|subject| *subject == &workspace)
            {
                break found.clone();
            }
            cursor = Some(
                page.continuation()
                    .expect("the selected workspace remains in the union")
                    .clone(),
            );
        }
    };
    assert!(matches!(
        comparison.compare_subject(&discovered).values(),
        PortableWorkspaceSubjectValues::Shared {
            comparison: PortableWorkspaceValueComparison::Equal,
            ..
        }
    ));
    assert_eq!(
        comparison
            .contributing_roots(&discovered, None, 1)
            .unwrap()
            .state(),
        pale_sdk::PortableWorkspaceContributingRootsState::Affected
    );
}
