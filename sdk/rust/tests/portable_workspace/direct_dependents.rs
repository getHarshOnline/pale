//! Direct-consumer scans match the existing compiler-input relation.

use super::*;
use pale_sdk::{PortableWorkspaceCompilation, PortableWorkspaceDependentPage};

fn assert_projection(
    workspace: &PortableWorkspaceCompilation,
    page: &PortableWorkspaceDependentPage,
) -> serde_json::Value {
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
            "admissionEvidenceDigest",
            "dependents",
            "inputDigestComparisons",
            "inspected",
            "isComplete",
            "principalUniverseDigest",
            "root",
            "semanticIrDigest",
            "subject",
            "workspaceId"
        ]
    );
    assert_eq!(value["workspaceId"], workspace.workspace_id());
    assert_eq!(
        value["principalUniverseDigest"],
        workspace.principal_universe_digest()
    );
    assert_eq!(value["semanticIrDigest"], workspace.semantic_ir_digest());
    assert_eq!(
        value["admissionEvidenceDigest"],
        workspace.admission_evidence_digest()
    );
    assert_eq!(value["inspected"], page.inspected());
    assert_eq!(
        value["inputDigestComparisons"],
        page.input_digest_comparisons()
    );
    assert_eq!(value["isComplete"], page.is_complete());
    assert_eq!(page.is_complete(), page.continuation().is_none());
    assert!(page.belongs_to(workspace));
    let inputs = workspace
        .direct_dependencies(page.subject(), None, 256)
        .unwrap();
    let input_value: serde_json::Value = serde_json::from_slice(inputs.canonical_bytes()).unwrap();
    assert_eq!(value["subject"], input_value["subject"]);
    assert_eq!(value["root"], input_value["root"]);
    assert_eq!(page.root(), inputs.root());
    let rows = value["dependents"].as_array().unwrap();
    assert_eq!(rows.len(), page.dependents().len());
    for (row, dependent) in rows.iter().zip(page.dependents()) {
        let reference = workspace
            .direct_dependencies(dependent.subject(), None, 256)
            .unwrap();
        let reference_value: serde_json::Value =
            serde_json::from_slice(reference.canonical_bytes()).unwrap();
        assert_eq!(Some(dependent), reference.root());
        assert_eq!(*row, reference_value["root"]);
    }
    assert_eq!(
        pale_core::canon::to_canonical_string(&value)
            .unwrap()
            .as_bytes(),
        page.canonical_bytes()
    );
    value
}

#[test]
fn direct_dependents_match_the_full_input_relation_without_skipping_empty_pages() {
    use pale_sdk::{
        ApiCapabilityClass, ApiCapabilityContract, ApiCapabilityEffect, ApiCapabilityId, ApiLeaf,
    };
    let capability = pale_sdk::capability_inventory()
        .iter()
        .find(|entry| entry.id() == ApiCapabilityId::PortableWorkspaceDirectDependents)
        .unwrap();
    assert_eq!(
        capability.eligible_leaves(),
        &[ApiLeaf::Rust, ApiLeaf::Node]
    );
    assert_eq!(capability.class(), ApiCapabilityClass::Portable);
    assert_eq!(capability.effect(), ApiCapabilityEffect::EffectFree);
    assert_eq!(
        capability.input(),
        Some(ApiCapabilityContract::ApiProjection(
            "portable-workspace-direct-dependent-request"
        ))
    );
    assert_eq!(
        capability.output(),
        Some(ApiCapabilityContract::ApiProjection(
            "portable-workspace-dependent-page"
        ))
    );
    let api = api();
    let workspace = api.compile_portable_workspace(empty_request()).unwrap();
    let comparison = api
        .compare_portable_workspaces(&workspace, &workspace)
        .unwrap();
    let subjects = all_impact_subjects(&comparison);
    // The reference uses the already callable forward relation, not the new scan.
    let mut candidates = subjects
        .iter()
        .map(|(subject, _)| {
            let mut inputs = Vec::new();
            let mut cursor = None;
            let root = loop {
                let page = workspace
                    .direct_dependencies(subject, cursor.as_ref(), 256)
                    .unwrap();
                inputs.extend_from_slice(page.dependencies());
                cursor = page.continuation().cloned();
                if cursor.is_none() {
                    break page.root().unwrap().clone();
                }
            };
            (root, inputs)
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|(left, _), (right, _)| left.node_digest().cmp(right.node_digest()));
    let mut saw_edge = false;
    let mut saw_empty_incomplete = false;
    for (subject, _) in &subjects {
        let target = workspace.direct_dependencies(subject, None, 1).unwrap();
        let digest = target.root().unwrap().node_digest();
        let mut expected = candidates
            .iter()
            .filter(|(_, inputs)| inputs.iter().any(|input| input.node_digest() == digest))
            .map(|(root, _)| root.clone())
            .collect::<Vec<_>>();
        expected.sort_by(|left, right| left.node_digest().cmp(right.node_digest()));
        saw_edge |= !expected.is_empty();
        for (inspected, returned) in [(0, 0), (1, 256), (256, 1), (2, 1), (3, 2)] {
            let mut cursor = None;
            let mut actual = Vec::new();
            let mut total_inspected = 0;
            loop {
                let page = workspace
                    .direct_dependents(subject, cursor.as_ref(), inspected, returned)
                    .unwrap();
                assert_projection(&workspace, &page);
                assert!(
                    page.inspected() <= usize::from(if inspected == 0 { 64 } else { inspected })
                );
                assert!(
                    page.dependents().len()
                        <= usize::from(if returned == 0 { 64 } else { returned })
                );
                saw_empty_incomplete |= page.dependents().is_empty() && !page.is_complete();
                let mut comparisons = 0;
                for (_, inputs) in &candidates[total_inspected..total_inspected + page.inspected()]
                {
                    let _ = inputs.binary_search_by(|input| {
                        comparisons += 1;
                        input.node_digest().cmp(digest)
                    });
                }
                assert_eq!(page.input_digest_comparisons(), comparisons);
                total_inspected += page.inspected();
                assert!(total_inspected <= candidates.len());
                actual.extend_from_slice(page.dependents());
                cursor = page.continuation().cloned();
                if cursor.is_none() {
                    break;
                }
                assert!(page.inspected() > 0);
            }
            assert_eq!(total_inspected, candidates.len());
            assert_eq!(actual, expected);
        }
    }
    assert!(saw_edge && saw_empty_incomplete);
}

#[test]
fn direct_dependents_bound_candidates_and_bind_both_raw_limits_and_exact_owner() {
    let api = api();
    let workspace = std::sync::Arc::new(
        api.compile_portable_workspace(wide_dependency_request())
            .unwrap(),
    );
    let rebuilt = api
        .compile_portable_workspace(wide_dependency_request())
        .unwrap();
    let changed_source = api
        .compile_portable_workspace(inputs::wide_dependency_request_with_comments(
            "\n# retained input differs\n",
        ))
        .unwrap();
    assert_eq!(
        workspace.semantic_ir_digest(),
        changed_source.semantic_ir_digest()
    );
    let subject = PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Workspace {
        workspace_id: workspace.workspace_id().into(),
    })
    .unwrap();
    let other_subject =
        PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Workspace {
            workspace_id: "other-workspace".into(),
        })
        .unwrap();
    // The wide fixture establishes more than 256 candidates, not 300 matching consumers.
    for (inspected, returned) in [(0, 0), (257, 257), (1, 1)] {
        let first = workspace
            .direct_dependents(&subject, None, inspected, returned)
            .unwrap();
        assert_eq!(
            first.inspected(),
            match inspected {
                0 => 64,
                257 => 256,
                _ => 1,
            }
        );
        assert!(first.root().is_some());
        assert!(first.dependents().is_empty());
        let cursor = first.continuation().unwrap();
        assert_projection(&workspace, &first);
        let different = |raw| {
            if raw == 0 {
                64
            } else if raw == 257 {
                u16::MAX
            } else {
                2
            }
        };
        for (query_owner, query_subject, limits) in [
            (&*workspace, &subject, (different(inspected), returned)),
            (&*workspace, &subject, (inspected, different(returned))),
            (&rebuilt, &subject, (inspected, returned)),
            (&changed_source, &subject, (inspected, returned)),
            (&*workspace, &other_subject, (inspected, returned)),
        ] {
            let error = query_owner
                .direct_dependents(query_subject, Some(cursor), limits.0, limits.1)
                .unwrap_err();
            assert_eq!(
                error.diagnostic().code(),
                "workspace_dependent_continuation_mismatch"
            );
            assert_eq!(
                error.diagnostic().disposition(),
                DiagnosticDisposition::InvalidInput
            );
        }
        let alias = std::sync::Arc::clone(&workspace);
        let next = alias
            .direct_dependents(&subject, Some(cursor), inspected, returned)
            .unwrap();
        let repeated = workspace
            .direct_dependents(&subject, Some(cursor), inspected, returned)
            .unwrap();
        assert_eq!(next.canonical_bytes(), repeated.canonical_bytes());
        assert!(!first.belongs_to(&rebuilt));
        assert!(!first.belongs_to(&changed_source));
        assert_eq!(
            format!("{cursor:?}"),
            "PortableWorkspaceDependentCursor { .. }"
        );
        assert_eq!(
            format!("{first:?}"),
            "PortableWorkspaceDependentPage { .. }"
        );
    }
    let absent = workspace
        .direct_dependents(&other_subject, None, 1, 1)
        .unwrap();
    let value = assert_projection(&workspace, &absent);
    assert!(absent.root().is_none() && absent.is_complete());
    assert!(absent.dependents().is_empty());
    assert_eq!(
        (absent.inspected(), absent.input_digest_comparisons()),
        (0, 0)
    );
    assert!(value["root"].is_null());
}
