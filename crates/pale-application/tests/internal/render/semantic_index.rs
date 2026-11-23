use pale_core::records::{PackageIdV1, PackageResourceIdV1};

use super::*;
use crate::compile::CompileError;
use crate::failure::ApplicationFailureDisposition;
use crate::render::semantic_helpers::resource_slug;

#[test]
fn path_ids_have_confined_distinct_host_names() {
    let resource = |id| PackageResourceRefV1 {
        package_id: PackageIdV1::parse("pale.example").unwrap(),
        resource_id: PackageResourceIdV1::parse(id).unwrap(),
    };
    let path = resource("testing/coverage");
    let flat = resource("testing-coverage");
    let long = resource(
        "testing/coverage/with-a-long-nested-resource-name-that-exceeds-the-host-prefix-limit",
    );
    let entries = [&path, &flat, &long].map(|id| (format!("pale-stack-{}", resource_slug(id)), id));
    validate_unique_host_names("skill", entries).unwrap();
    assert!(!resource_slug(&path).contains('/'));
    assert_ne!(resource_slug(&path), resource_slug(&flat));
}

#[test]
fn normalized_host_name_conflict_preserves_both_canonical_ids() {
    let resource = |id| PackageResourceRefV1 {
        package_id: PackageIdV1::parse("pale.example").unwrap(),
        resource_id: PackageResourceIdV1::parse(id).unwrap(),
    };
    let first = resource("review-route");
    let second = resource("review_route");
    let entries = [&first, &second].map(|id| (format!("pale-op-{}", resource_slug(id)), id));
    for entry in &entries {
        validate_unique_host_names("skill", [entry.clone()]).unwrap();
    }
    let error = validate_unique_host_names("skill", entries).unwrap_err();
    assert!(matches!(
        &error,
        RenderError::HostNameCollision { kind: "skill", name, first, second }
            if name == "pale-op-pale-example-review-route"
                && first == "pale.example:review-route"
                && second == "pale.example:review_route"
    ));
    assert_eq!(
        CompileError::Render(Box::new(error)).failure_disposition(),
        ApplicationFailureDisposition::InvalidInput
    );
}
