//! Exact complete-owner release for endpoint/incident pages and cursors.

use super::*;

#[test]
fn endpoint_pages_and_incident_cursors_retain_then_release_the_complete_compilation() {
    let node = OrganizationNode::Role(OrganizationRoleRef {
        namespace: "alpha".into(),
        role_id: "accountable-a".into(),
    });
    retains_independently(|owner| {
        owner
            .inspect_organization_endpoint(&node, None, 1, 1, MAX_ORGANIZATION_COPY_BYTES)
            .unwrap()
    });
    retains_independently(|owner| {
        owner
            .inspect_organization_incident_relations(&node, None, 1, 1, MAX_ORGANIZATION_COPY_BYTES)
            .unwrap()
            .continuation()
            .unwrap()
            .clone()
    });
}

#[test]
fn endpoint_result_handle_outlives_page_and_releases_with_its_last_owner() {
    let owner = compile();
    let weak = Arc::downgrade(&owner.universe);
    let node = OrganizationNode::Role(OrganizationRoleRef {
        namespace: "alpha".into(),
        role_id: "accountable-a".into(),
    });
    let page = owner
        .inspect_organization_endpoint(&node, None, 256, 256, MAX_ORGANIZATION_COPY_BYTES)
        .unwrap();
    let handle = page.entries()[0].handle().clone();
    drop(page);
    drop(owner);
    assert!(weak.upgrade().is_some());
    assert!(handle
        .sources(None, 1, 1, MAX_ORGANIZATION_COPY_BYTES)
        .is_ok());
    drop(handle);
    assert!(weak.upgrade().is_none());
}
