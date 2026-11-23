//! Contributing-root pages preserve verified impact without recomputing it.

use super::*;
use pale_sdk::{PortableWorkspaceComparison, PortableWorkspaceContributingRootsState as State};

fn assert_canonical_headers(
    page: &pale_sdk::PortableWorkspaceContributingRootsPage,
    comparison: &PortableWorkspaceComparison,
    state: &str,
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
            "currentPrincipalUniverseDigest",
            "currentSemanticIrDigest",
            "isComplete",
            "priorPrincipalUniverseDigest",
            "priorSemanticIrDigest",
            "receiptDigest",
            "roots",
            "state",
            "subject",
        ]
    );
    assert_eq!(value["receiptDigest"], comparison.receipt_digest());
    assert_eq!(
        value["priorPrincipalUniverseDigest"],
        comparison.prior_principal_universe_digest()
    );
    assert_eq!(
        value["currentPrincipalUniverseDigest"],
        comparison.current_principal_universe_digest()
    );
    assert_eq!(
        value["priorSemanticIrDigest"],
        comparison.prior_semantic_ir_digest()
    );
    assert_eq!(
        value["currentSemanticIrDigest"],
        comparison.current_semantic_ir_digest()
    );
    let subject_comparison = comparison.compare_subject(page.subject());
    let subject_value: serde_json::Value =
        serde_json::from_slice(subject_comparison.canonical_bytes()).unwrap();
    assert_eq!(value["subject"], subject_value["subject"]);
    assert_eq!(value["state"], state);
    assert_eq!(value["isComplete"], page.continuation().is_none());
    assert_eq!(value["roots"].as_array().unwrap().len(), page.roots().len());
    assert_eq!(
        pale_core::canon::to_canonical_string(&value)
            .unwrap()
            .as_bytes(),
        page.canonical_bytes()
    );
    value
}

fn all_roots(
    comparison: &PortableWorkspaceComparison,
) -> Vec<pale_sdk::PortableWorkspaceChangeRoot> {
    let mut roots = Vec::new();
    let mut offset = 0;
    loop {
        let page = comparison.change_roots(offset, 256);
        roots.extend_from_slice(page.entries());
        let Some(next) = page.next_offset() else {
            break;
        };
        offset = next;
    }
    roots
}

#[test]
fn contributing_roots_enumerate_all_verified_causes_without_inferring_value_change() {
    use pale_sdk::{
        ApiCapabilityClass, ApiCapabilityContract, ApiCapabilityEffect, ApiCapabilityId, ApiLeaf,
    };
    let capability = pale_sdk::capability_inventory()
        .iter()
        .find(|entry| entry.id() == ApiCapabilityId::PortableWorkspaceContributingRoots)
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
            "portable-workspace-contributing-root-page-request"
        ))
    );
    assert_eq!(
        capability.output(),
        Some(ApiCapabilityContract::ApiProjection(
            "portable-workspace-contributing-root-page"
        ))
    );
    let api = api();
    let prior = api
        .compile_portable_workspace(wide_dependency_request())
        .unwrap();
    let current = api
        .compile_portable_workspace(inputs::wide_dependency_request_with_comments(
            "\n# source-only contribution\n",
        ))
        .unwrap();
    assert_eq!(prior.semantic_ir_digest(), current.semantic_ir_digest());
    let comparison = api.compare_portable_workspaces(&prior, &current).unwrap();
    let subject =
        PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::ResponsibilityClosure)
            .unwrap();
    let expected = all_roots(&comparison);
    let mut expected_json = std::collections::BTreeMap::new();
    let mut offset = 0;
    loop {
        let page = comparison.change_roots(offset, 256);
        let value: serde_json::Value = serde_json::from_slice(page.canonical_bytes()).unwrap();
        for root in value["entries"].as_array().unwrap() {
            expected_json.insert(
                root["rootDigest"].as_str().unwrap().to_owned(),
                root.clone(),
            );
        }
        let Some(next) = page.next_offset() else {
            break;
        };
        offset = next;
    }
    assert_eq!(expected.len(), 300);
    assert!(expected
        .iter()
        .all(|root| root.kind() == PortableWorkspaceChangeKind::Modified));
    assert!(matches!(
        comparison.compare_subject(&subject).values(),
        PortableWorkspaceSubjectValues::Shared {
            comparison: PortableWorkspaceValueComparison::Equal,
            ..
        }
    ));
    for limit in [0, 1, 64, 256, u16::MAX] {
        let mut roots = Vec::new();
        let mut after = None;
        loop {
            let page = comparison
                .contributing_roots(&subject, after.as_ref(), limit)
                .unwrap();
            assert_eq!(page.state(), State::Affected);
            assert_eq!(page.subject(), &subject);
            assert!(page.belongs_to(&comparison));
            assert_eq!(
                page.comparison().receipt_digest(),
                comparison.receipt_digest()
            );
            assert_eq!(
                page.comparison().prior_principal_universe_digest(),
                comparison.prior_principal_universe_digest()
            );
            assert_eq!(
                page.comparison().current_principal_universe_digest(),
                comparison.current_principal_universe_digest()
            );
            assert_eq!(
                page.comparison().prior_semantic_ir_digest(),
                comparison.prior_semantic_ir_digest()
            );
            assert_eq!(
                page.comparison().current_semantic_ir_digest(),
                comparison.current_semantic_ir_digest()
            );
            assert!(
                page.roots().len() <= usize::from(if limit == 0 { 64 } else { limit.min(256) })
            );
            assert!(!page.roots().is_empty());
            let value = assert_canonical_headers(&page, &comparison, "affected");
            for (encoded, root) in value["roots"].as_array().unwrap().iter().zip(page.roots()) {
                assert_eq!(encoded, &expected_json[root.root_digest()]);
            }
            roots.extend_from_slice(page.roots());
            after = page.continuation().cloned();
            if after.is_none() {
                break;
            }
        }
        assert_eq!(roots, expected);
    }
    assert_eq!(
        comparison.explain(&subject).unwrap().state(),
        PortableWorkspaceExplanationState::Affected
    );
}

#[test]
fn contributing_root_cursors_bind_exact_pair_full_subject_and_raw_limit() {
    let api = api();
    let prior = api
        .compile_portable_workspace(wide_dependency_request())
        .unwrap();
    let current = api
        .compile_portable_workspace(inputs::wide_dependency_request_with_comments(
            "\n# first capture\n",
        ))
        .unwrap();
    let different_source = api
        .compile_portable_workspace(inputs::wide_dependency_request_with_comments(
            "\n# different capture\n",
        ))
        .unwrap();
    assert_eq!(
        current.semantic_ir_digest(),
        different_source.semantic_ir_digest()
    );
    let comparison = api.compare_portable_workspaces(&prior, &current).unwrap();
    let same_pair = api.compare_portable_workspaces(&prior, &current).unwrap();
    let replaced_current = api
        .compare_portable_workspaces(&prior, &different_source)
        .unwrap();
    let recompiled_prior = api
        .compile_portable_workspace(wide_dependency_request())
        .unwrap();
    let replaced_prior = api
        .compare_portable_workspaces(&recompiled_prior, &current)
        .unwrap();
    let reversed = api.compare_portable_workspaces(&current, &prior).unwrap();
    let subject =
        PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::ResponsibilityClosure)
            .unwrap();
    let other = PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Workspace {
        workspace_id: current.workspace_id().into(),
    })
    .unwrap();
    for (limit, unlike) in [(1, 2), (0, 64), (257, u16::MAX)] {
        let first = comparison
            .contributing_roots(&subject, None, limit)
            .unwrap();
        let cursor = first.continuation().unwrap();
        for outcome in [
            comparison.contributing_roots(&subject, Some(cursor), unlike),
            comparison.contributing_roots(&other, Some(cursor), limit),
            replaced_current.contributing_roots(&subject, Some(cursor), limit),
            replaced_prior.contributing_roots(&subject, Some(cursor), limit),
            reversed.contributing_roots(&subject, Some(cursor), limit),
        ] {
            let error = outcome.unwrap_err();
            assert_eq!(
                error.diagnostic().code(),
                "workspace_contributing_roots_continuation_mismatch"
            );
            assert_eq!(
                error.diagnostic().disposition(),
                DiagnosticDisposition::InvalidInput
            );
        }
        assert!(!first.belongs_to(&replaced_current));
        assert!(!first.belongs_to(&replaced_prior));
        assert!(first.belongs_to(&same_pair));
        let next = comparison
            .contributing_roots(&subject, Some(cursor), limit)
            .unwrap();
        assert_eq!(
            next.canonical_bytes(),
            same_pair
                .contributing_roots(&subject, Some(cursor), limit)
                .unwrap()
                .canonical_bytes()
        );
        assert_eq!(
            next.canonical_bytes(),
            comparison
                .contributing_roots(&subject, Some(cursor), limit)
                .unwrap()
                .canonical_bytes()
        );
    }
    let workspace_page = comparison.contributing_roots(&other, None, 1).unwrap();
    let foreign_workspace =
        PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Workspace {
            workspace_id: "another-workspace".into(),
        })
        .unwrap();
    let error = comparison
        .contributing_roots(
            &foreign_workspace,
            Some(workspace_page.continuation().unwrap()),
            1,
        )
        .unwrap_err();
    assert_eq!(
        error.diagnostic().code(),
        "workspace_contributing_roots_continuation_mismatch"
    );
}

#[test]
fn contributing_roots_distinguish_absence_unaffected_and_prior_only_causes() {
    let api = api();
    let empty = api.compile_portable_workspace(empty_request()).unwrap();
    let wide = api
        .compile_portable_workspace(wide_dependency_request())
        .unwrap();
    let subject =
        PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::ResponsibilityClosure)
            .unwrap();
    let unchanged = api.compare_portable_workspaces(&empty, &empty).unwrap();
    let page = unchanged.contributing_roots(&subject, None, 0).unwrap();
    assert_eq!(page.state(), State::Unaffected);
    assert!(page.roots().is_empty());
    assert!(page.continuation().is_none());
    assert_canonical_headers(&page, &unchanged, "unaffected");
    let addition = api.compare_portable_workspaces(&empty, &wide).unwrap();
    let added_roots = all_roots(&addition);
    let source = added_roots
        .iter()
        .find(|root| root.kind() == PortableWorkspaceChangeKind::Added)
        .unwrap()
        .subject()
        .clone();
    let removal = api.compare_portable_workspaces(&wide, &empty).unwrap();
    assert!(matches!(
        removal.compare_subject(&source).values(),
        PortableWorkspaceSubjectValues::PriorOnly { .. }
    ));
    let absent = removal.contributing_roots(&source, None, 1).unwrap();
    assert_eq!(absent.state(), State::NotPresent);
    assert!(absent.roots().is_empty());
    assert!(absent.continuation().is_none());
    assert_canonical_headers(&absent, &removal, "not_present");
    let mut contributed = Vec::new();
    let mut cursor = None;
    loop {
        let page = removal
            .contributing_roots(&subject, cursor.as_ref(), 64)
            .unwrap();
        assert_eq!(page.state(), State::Affected);
        assert_canonical_headers(&page, &removal, "affected");
        contributed.extend_from_slice(page.roots());
        cursor = page.continuation().cloned();
        if cursor.is_none() {
            break;
        }
    }
    assert!(contributed
        .iter()
        .any(|root| root.kind() == PortableWorkspaceChangeKind::Removed));
    assert!(contributed
        .iter()
        .any(|root| root.kind() == PortableWorkspaceChangeKind::Modified));
    assert_eq!(contributed.len(), 301);
    assert_eq!(contributed, all_roots(&removal));
    let retained = absent.clone();
    drop(absent);
    drop(removal);
    drop(addition);
    drop(unchanged);
    drop(wide);
    drop(empty);
    drop(api);
    assert_eq!(retained.state(), State::NotPresent);
    assert_eq!(retained.subject(), &source);
    assert!(!retained.canonical_bytes().is_empty());
}
