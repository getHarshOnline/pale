//! Exercises checked deserialization of digest-bound workspace modules.

use super::*;

fn module_preimage() -> WorkspaceModulePreimageV1 {
    WorkspaceModulePreimageV1 {
        module_id: WorkspaceModuleId::parse("module").unwrap(),
        release_id: WorkspaceReleaseId::parse("release-1").unwrap(),
        publisher: WorkspacePublisherRefV1 {
            publisher_id: WorkspacePublisherId::parse("publisher").unwrap(),
            root_key_id: WorkspacePublisherKeyId::parse("root-key").unwrap(),
            recovery_authority_id: WorkspaceRecoveryAuthorityId::parse("recovery").unwrap(),
            delegated_release_key_id: WorkspacePublisherKeyId::parse("release-key").unwrap(),
        },
        source_components: Vec::new(),
        imports: Vec::new(),
        exports: Vec::new(),
        extensions: Vec::new(),
        resources: Vec::new(),
    }
}

#[test]
fn workspace_module_valid_value_round_trips_through_checked_deserialization() {
    let module = WorkspaceModuleV1::seal(module_preimage()).unwrap();
    let decoded: WorkspaceModuleV1 =
        serde_json::from_value(serde_json::to_value(&module).unwrap()).unwrap();
    assert_eq!(decoded, module);
}

#[test]
fn checked_deserialization_rejects_redigested_workspace_module_invalidity() {
    let mut module = module_preimage();
    let duplicate = WorkspaceSourceComponentRefV1 {
        component_id: WorkspaceComponentId::parse("component").unwrap(),
        component_digest: WorkspaceComponentDigest::parse("a".repeat(64)).unwrap(),
    };
    module.source_components = vec![duplicate.clone(), duplicate];
    let module_digest =
        WorkspaceModuleDigest::hash_canonical(&("pale.workspace-module.v1", &module)).unwrap();
    let mut wire = serde_json::to_value(module).unwrap();
    let object = wire.as_object_mut().unwrap();
    let module_body = std::mem::take(object);
    object.insert("schema".into(), "pale/workspace-module.v1".into());
    object.insert("schema_version".into(), 1.into());
    object.insert("module".into(), module_body.into());
    object.insert(
        "module_digest".into(),
        serde_json::to_value(module_digest).unwrap(),
    );
    assert!(serde_json::from_value::<WorkspaceModuleV1>(wire).is_err());
}
