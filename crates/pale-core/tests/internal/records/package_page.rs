//! Page declarations keep their uses together without repeating source identity.

use super::*;

#[test]
fn ordinary_page_defaults_to_a_guide() {
    let page: PackagePageV1 =
        serde_json::from_value(serde_json::json!({"description": "Check a change."})).unwrap();
    assert!(page.is_guide());
    assert_eq!(page.exposure, PackageResourceExposureV1::Exported);
    assert!(page.task.is_none());
}

#[test]
fn one_page_can_declare_several_uses_without_several_bodies() {
    let page: PackagePageV1 = serde_json::from_value(serde_json::json!({
        "description": "Review a change and explain concrete findings.",
        "uses": [{"scope":"local", "resource_id":"review/shared-method"}],
        "guide": {},
        "task": {"outputs": ["Findings in the reply."]},
        "role": {"extends": [{"scope":"local", "resource_id":"review/independent"}]},
        "policy": {},
        "stack": {"role_uses": [{"scope":"local", "resource_id":"review/read-only"}]}
    }))
    .unwrap();
    assert!(page.is_guide());
    assert_eq!(page.uses.len(), 1);
    assert_eq!(
        page.task.as_ref().unwrap().outputs,
        ["Findings in the reply."]
    );
    assert_eq!(page.role.as_ref().unwrap().extends.len(), 1);
    assert!(page.policy.is_some());
    assert_eq!(page.stack.as_ref().unwrap().role_uses.len(), 1);
    let encoded = serde_json::to_value(&page).unwrap();
    assert_eq!(
        serde_json::from_value::<PackagePageV1>(encoded).unwrap(),
        page
    );
}

#[test]
fn explicit_task_does_not_silently_add_guide_use() {
    let page: PackagePageV1 = serde_json::from_value(serde_json::json!({
        "description": "Run a repository check.", "task": {}
    }))
    .unwrap();
    assert!(!page.is_guide());
}

#[test]
fn shared_body_keeps_internal_guidance_separate_from_exported_uses() {
    let page: PackagePageV1 = serde_json::from_value(serde_json::json!({
        "description": "Review a change.",
        "guide": {"exposure": "internal"},
        "task": {},
        "role": {}
    }))
    .unwrap();
    assert_eq!(page.exposure, PackageResourceExposureV1::Exported);
    assert_eq!(
        page.guide.unwrap().exposure,
        Some(PackageResourceExposureV1::Internal)
    );
    assert_eq!(page.task.unwrap().exposure, None);
    assert_eq!(page.role.unwrap().exposure, None);
}

#[test]
fn header_cannot_override_its_source_or_restore_old_placement_names() {
    for field in ["id", "body", "doctrine", "placement", "lens", "startup"] {
        let mut value = serde_json::json!({"description": "Check a change."});
        value[field] = serde_json::json!("override");
        assert!(
            serde_json::from_value::<PackagePageV1>(value).is_err(),
            "{field}"
        );
    }
    assert!(serde_json::from_value::<PackagePageV1>(serde_json::json!({
        "description": "Check a change.", "task": {"body":"other.md"}
    }))
    .is_err());
}
