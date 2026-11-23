//! Subject discovery merges retained maps instead of rebuilding an impact receipt.

use super::*;
use std::collections::BTreeSet;

#[test]
fn comparison_subject_pages_follow_full_keys_with_bounded_lookahead() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspace-language/empty-adopter");
    let prior = load_portable_fixture(&fixture);
    let current = load_portable_fixture(&fixture);
    // This complete receipt is a test oracle, not work done by the page.
    let reference = current.compare_with_predecessor(&prior).unwrap();
    let expected = reference
        .receipt()
        .nodes
        .iter()
        .map(|node| node.subject.clone())
        .collect::<BTreeSet<_>>();
    for requested in [0, 1, 2, 257, u16::MAX] {
        let limit = usize::from(if requested == 0 {
            64
        } else {
            requested.min(256)
        });
        let mut after = None;
        let mut observed = Vec::new();
        loop {
            let page = current.comparison_subjects(&prior, after.as_ref(), requested);
            let rows = page.subjects().cloned().collect::<Vec<_>>();
            assert!(rows.len() <= limit);
            assert!(
                page.subject_identity_bytes()
                    <= crate::workspace::MAX_COMPARISON_SUBJECT_PAGE_IDENTITY_BYTES
            );
            assert_eq!(page.key_comparisons(), rows.len());
            assert_eq!(
                page.inspected(),
                2 * rows.len() + if page.next_after().is_some() { 2 } else { 0 }
            );
            assert!(rows.windows(2).all(|pair| pair[0] < pair[1]));
            if let Some(last) = &after {
                assert!(rows.iter().all(|subject| subject > last));
            }
            observed.extend(rows);
            after = page.next_after().cloned();
            if after.is_none() {
                break;
            }
        }
        assert_eq!(observed, expected.iter().cloned().collect::<Vec<_>>());
    }
    let end = current.comparison_subjects(&prior, expected.last(), 1);
    assert_eq!(end.subjects().len(), 0);
    assert_eq!((end.inspected(), end.key_comparisons()), (0, 0));
    assert!(end.next_after().is_none());
}
