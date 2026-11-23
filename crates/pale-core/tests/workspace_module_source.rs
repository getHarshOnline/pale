//! Local authoring computes content identities without publication metadata.

use pale_core::records::*;
use std::collections::BTreeMap;

fn fixture() -> (WorkspaceModuleSourceV1, WorkspaceLanguagePrimitivesV1) {
    let module_id = WorkspaceModuleId::parse("local").unwrap();
    let component = WorkspaceLanguagePrimitivesV1 {
        schema: "pale/workspace-language-primitives.v1".into(),
        schema_version: 1,
        module_id: module_id.clone(),
        component_id: WorkspaceComponentId::parse("content").unwrap(),
        declarations: Vec::new(),
    };
    let source = WorkspaceModuleSourceV1 {
        schema: "pale/workspace-module-source.v1".into(),
        schema_version: 1,
        module_id,
        source_components: vec![component.component_id.clone()],
        imports: Vec::new(),
        exports: Vec::new(),
        extensions: Vec::new(),
        resources: Vec::new(),
    };
    (source, component)
}

#[test]
fn local_content_computes_component_identity_and_requires_explicit_publication_metadata() {
    let (source, component) = fixture();
    let components = BTreeMap::from([(component.component_id.clone(), component.clone())]);
    let lowered = source.lower(&components).unwrap();
    assert_eq!(
        lowered.source_components[0].component_digest,
        component.component_digest().unwrap()
    );
    let wire = serde_json::to_value(&source).unwrap();
    assert!(wire.get("publisher").is_none());
    assert!(wire.get("release_id").is_none());
    let decoded: WorkspaceModuleSourceV1 = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(
        lowered.source_digest().unwrap(),
        decoded.lower(&components).unwrap().source_digest().unwrap()
    );
    for field in ["publisher", "release_id", "module_digest"] {
        let mut invalid = wire.clone();
        invalid[field] = "invented".into();
        assert!(serde_json::from_value::<WorkspaceModuleSourceV1>(invalid).is_err());
    }
    let publisher = WorkspacePublisherRefV1 {
        publisher_id: WorkspacePublisherId::parse("synthetic-publisher").unwrap(),
        root_key_id: WorkspacePublisherKeyId::parse("synthetic-root").unwrap(),
        recovery_authority_id: WorkspaceRecoveryAuthorityId::parse("synthetic-recovery").unwrap(),
        delegated_release_key_id: WorkspacePublisherKeyId::parse("synthetic-release").unwrap(),
    };
    let published = lowered
        .clone()
        .prepare_release(
            WorkspaceReleaseId::parse("synthetic-release").unwrap(),
            publisher,
        )
        .unwrap();
    published.verify().unwrap();
    assert_eq!(
        published.module.source_components,
        lowered.source_components
    );
    let decoded: WorkspaceModuleV1 =
        serde_json::from_slice(&serde_json::to_vec(&published).unwrap()).unwrap();
    assert_eq!(published, decoded);
}

#[test]
fn missing_foreign_duplicate_and_changed_components_have_distinct_outcomes() {
    let (mut source, mut component) = fixture();
    assert!(matches!(
        source.lower(&BTreeMap::new()),
        Err(InvalidWorkspaceModuleSource::MissingComponent(_))
    ));
    let key = component.component_id.clone();
    let before = source
        .lower(&BTreeMap::from([(key.clone(), component.clone())]))
        .unwrap()
        .source_digest()
        .unwrap();
    component.declarations.push(WorkspaceDeclarationV1::Trait {
        id: WorkspaceDeclarationId::parse("changed").unwrap(),
        namespace: WorkspaceNamespaceId::parse("local").unwrap(),
        dependencies: Vec::new(),
    });
    let changed = BTreeMap::from([(key.clone(), component.clone())]);
    assert_ne!(
        before,
        source.lower(&changed).unwrap().source_digest().unwrap()
    );
    source.source_components.push(key.clone());
    assert!(source.lower(&changed).is_err());
    source.source_components.pop();
    component.module_id = WorkspaceModuleId::parse("foreign").unwrap();
    assert!(matches!(
        source.lower(&BTreeMap::from([(key, component)])),
        Err(InvalidWorkspaceModuleSource::ComponentOwner(_))
    ));
}
