//! Bounded one-hop scans against actual admitted compiler facts.

use super::*;

#[test]
fn direct_dependents_match_the_full_reference_relation_with_bounded_work() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspace-language/empty-adopter");
    let portable = load_portable_fixture(&fixture);
    // A comparison is only the test oracle; the query itself never builds one.
    let reference = portable.compare_with_predecessor(&portable).unwrap();
    let nodes = &reference.receipt().nodes;
    let mut saw_match = false;
    let mut saw_empty_incomplete = false;
    let mut saw_return_limit = false;
    for target in nodes {
        let expected = nodes
            .iter()
            .filter(|candidate| candidate.inputs.contains(&target.node_digest))
            .collect::<Vec<_>>();
        for (inspected, returned) in [
            (0, 0),
            (1, 256),
            (256, 1),
            (2, 1),
            (3, 2),
            (u16::MAX, u16::MAX),
        ] {
            let normalize = |value: u16| usize::from(if value == 0 { 64 } else { value.min(256) });
            let mut offset = 0;
            let mut collected = Vec::new();
            loop {
                let page = portable
                    .direct_dependents(&target.subject, offset, inspected, returned)
                    .unwrap()
                    .expect("the oracle subject is selected");
                assert_eq!(page.root(), target);
                assert!(page.inspected() > 0);
                assert!(page.inspected() <= normalize(inspected));
                assert!(page.dependents().len() <= normalize(returned));
                let examined = &nodes[offset..offset + page.inspected()];
                let matches = examined
                    .iter()
                    .filter(|candidate| candidate.inputs.contains(&target.node_digest))
                    .collect::<Vec<_>>();
                assert_eq!(page.dependents().collect::<Vec<_>>(), matches);
                // Reobserve this toolchain's comparator calls, not a frozen
                // universal exact count for std's binary-search implementation.
                let mut comparisons = 0;
                let mut comparison_bound = 0;
                for candidate in examined {
                    let _ = candidate.inputs.binary_search_by(|input| {
                        comparisons += 1;
                        input.cmp(&target.node_digest)
                    });
                    if !candidate.inputs.is_empty() {
                        comparison_bound +=
                            (usize::BITS - candidate.inputs.len().leading_zeros()) as usize + 1;
                    }
                }
                assert_eq!(page.input_digest_comparisons(), comparisons);
                assert!(comparisons <= comparison_bound);
                for consumer in page.dependents() {
                    saw_match = true;
                    assert!(consumer.inputs.contains(&page.root().node_digest));
                    let existing = portable
                        .direct_dependencies(&consumer.subject, 0, 1)
                        .unwrap()
                        .unwrap();
                    assert!(std::ptr::eq(consumer, existing.root()));
                    collected.push(consumer.clone());
                }
                let next = offset + page.inspected();
                assert_eq!(page.next_offset(), (next < nodes.len()).then_some(next));
                saw_empty_incomplete |=
                    page.dependents().len() == 0 && page.next_offset().is_some();
                if page.inspected() < normalize(inspected) && next < nodes.len() {
                    assert_eq!(page.dependents().len(), normalize(returned));
                    saw_return_limit = true;
                }
                let Some(next) = page.next_offset() else {
                    break;
                };
                assert!(next > offset);
                offset = next;
            }
            assert_eq!(collected.iter().collect::<Vec<_>>(), expected);
        }
    }
    assert!(
        saw_match,
        "actual compiler fixture must contain a direct edge"
    );
    assert!(
        saw_empty_incomplete,
        "nonmatches still consume candidate budget"
    );
    assert!(
        saw_return_limit,
        "returned limit can end a page before scan exhaustion"
    );
}

#[test]
fn direct_dependents_separate_missing_roots_from_terminal_empty_scans() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspace-language/empty-adopter");
    let portable = load_portable_fixture(&fixture);
    let reference = portable.compare_with_predecessor(&portable).unwrap();
    let count = reference.receipt().nodes.len();
    assert!(count > 1);
    let subject = WorkspaceDerivationSubjectV1::Workspace {
        workspace_id: WorkspaceId::parse(portable.workspace_id()).unwrap(),
    };
    let target = reference
        .receipt()
        .nodes
        .iter()
        .find(|node| node.subject == subject)
        .unwrap();
    assert!(reference
        .receipt()
        .nodes
        .iter()
        .all(|node| !node.inputs.contains(&target.node_digest)));
    let first = portable
        .direct_dependents(&subject, 0, 1, 1)
        .unwrap()
        .unwrap();
    assert_eq!(first.dependents().len(), 0);
    assert_eq!(first.inspected(), 1);
    assert_eq!(first.next_offset(), Some(1));
    let end = portable
        .direct_dependents(&subject, count, 0, 0)
        .unwrap()
        .unwrap();
    assert_eq!(end.root(), target);
    assert_eq!(end.dependents().len(), 0);
    assert_eq!(end.inspected(), 0);
    assert_eq!(end.input_digest_comparisons(), 0);
    assert_eq!(end.next_offset(), None);
    for offset in [count + 1, usize::MAX] {
        assert!(portable.direct_dependents(&subject, offset, 1, 1).is_err());
    }
    let missing = WorkspaceDerivationSubjectV1::Workspace {
        workspace_id: WorkspaceId::parse("not-selected").unwrap(),
    };
    for offset in [0, usize::MAX] {
        assert!(portable
            .direct_dependents(&missing, offset, 1, 1)
            .unwrap()
            .is_none());
    }
}

#[test]
fn direct_dependents_bound_aggregate_input_search_work_without_skipping_candidates() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspace-language/empty-adopter");
    let portable = load_portable_fixture(&fixture);
    let reference = portable.compare_with_predecessor(&portable).unwrap();
    let nodes = &reference.receipt().nodes;
    let (root, consumer) = nodes
        .iter()
        .find_map(|root| {
            nodes
                .iter()
                .find(|candidate| candidate.inputs.contains(&root.node_digest))
                .map(|consumer| (root, consumer))
        })
        .expect("fixture has at least one direct proof edge");

    // Exercise the page owner with an admitted-shape maximum that would exceed
    // the aggregate comparison ceiling if the candidate limit were the only
    // work bound. Repeated nodes are deliberate private fault injection; the
    // paging primitive does not own receipt admission.
    let mut wide_inputs = (0..1_024_u16)
        .map(|index| WorkspaceDerivationNodeDigest::hash_canonical(&index).unwrap())
        .collect::<Vec<_>>();
    wide_inputs.push(root.node_digest.clone());
    wide_inputs.sort();
    wide_inputs.dedup();
    assert!(wide_inputs.len() <= MAX_WORKSPACE_DERIVATION_INPUTS);
    let mut wide_consumer = consumer.clone();
    wide_consumer.inputs = wide_inputs;
    let candidates = vec![wide_consumer; usize::from(crate::workspace::MAX_DEPENDENT_PAGE_LIMIT)];

    let mut offset = 0;
    let mut pages = 0;
    while offset < candidates.len() {
        let page = crate::workspace::PortableWorkspaceDependentPage::new(
            root,
            &candidates,
            offset,
            crate::workspace::MAX_DEPENDENT_PAGE_LIMIT,
            crate::workspace::MAX_DEPENDENT_PAGE_LIMIT,
        )
        .unwrap();
        pages += 1;
        assert!(
            page.inspected() > 0,
            "every continuation must make progress"
        );
        assert_eq!(page.dependents().len(), page.inspected());
        assert!(
            page.input_digest_comparisons()
                <= crate::workspace::MAX_DEPENDENT_PAGE_INPUT_COMPARISONS
        );
        let next = offset + page.inspected();
        assert_eq!(
            page.next_offset(),
            (next < candidates.len()).then_some(next)
        );
        offset = next;
    }
    assert!(pages > 1, "the aggregate work ceiling must split this scan");
}
