//! The retained comparison and either record independently preserve source custody.

use super::*;
use crate::OrganizationComparisonSide;
use crate::OrganizationCorrespondenceState;

#[test]
fn a_search_cursor_alone_retains_both_complete_sources_until_final_drop() {
    let prior = compile();
    let current = compile();
    let before = Arc::downgrade(&prior.universe);
    let after = Arc::downgrade(&current.universe);
    let comparison = PaleApi::new()
        .unwrap()
        .compare_portable_workspaces(&prior, &current)
        .unwrap();
    let handle = selected(
        &prior,
        |row| matches!(row, OrganizationRow::Scope { scope, .. } if scope.namespace == "gamma"),
    );
    let result = comparison
        .correspond_organization_record(
            OrganizationComparisonSide::Prior,
            &handle,
            None,
            1,
            MAX_ORGANIZATION_COPY_BYTES,
        )
        .unwrap();
    let OrganizationCorrespondenceState::Searching(cursor) = result.state() else {
        panic!("need actual incomplete scan");
    };
    let cursor = cursor.clone();
    drop(result);
    drop(handle);
    drop(comparison);
    drop(prior);
    drop(current);
    assert!(before.upgrade().is_some());
    assert!(after.upgrade().is_some());
    drop(cursor);
    assert!(before.upgrade().is_none());
    assert!(after.upgrade().is_none());
}

#[test]
fn matched_handles_keep_their_own_sources_after_result_and_pair_are_dropped() {
    let prior = compile();
    let current = compile();
    let before = Arc::downgrade(&prior.universe);
    let after = Arc::downgrade(&current.universe);
    let comparison = PaleApi::new()
        .unwrap()
        .compare_portable_workspaces(&prior, &current)
        .unwrap();
    let selected_handle = selected(&prior, |row| matches!(row, OrganizationRow::Role { .. }));
    let result = comparison
        .correspond_organization_record(
            OrganizationComparisonSide::Prior,
            &selected_handle,
            None,
            256,
            MAX_ORGANIZATION_COPY_BYTES,
        )
        .unwrap();
    let OrganizationCorrespondenceState::Matched { other, .. } = result.state() else {
        panic!("actual match required");
    };
    let other = other.handle().clone();
    let selected = result.selected().clone();
    drop(selected_handle);
    drop(result);
    drop(comparison);
    drop(prior);
    drop(current);
    assert!(selected
        .sources(None, 256, 256, MAX_ORGANIZATION_COPY_BYTES)
        .is_ok());
    assert!(other
        .sources(None, 256, 256, MAX_ORGANIZATION_COPY_BYTES)
        .is_ok());
    drop(selected);
    assert!(before.upgrade().is_none());
    assert!(after.upgrade().is_some());
    drop(other);
    assert!(after.upgrade().is_none());
}
