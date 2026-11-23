//! Header-authored pages pass through the compiler's shared Package reader.

use super::*;
use crate::packages::PackageReleaseIndex;
use pale_core::records::{
    DistributionLockFile, DistributionMeta, SchemaManifestDigest, SchemaSetDigest,
};

#[path = "pages/explicit_data.rs"]
mod explicit_data;

const BODY: &str = "\n# Review\n\nInspect the actual change.\n";

fn reference(id: &str) -> PackageResourceRefV1 {
    PackageResourceRefV1 {
        package_id: PackageIdV1::parse("example.pages").unwrap(),
        resource_id: PackageResourceIdV1::parse(id).unwrap(),
    }
}

fn fixture(guide_exposure: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let package = root.path().join("packages/pages");
    std::fs::create_dir_all(package.join("topic")).unwrap();
    let mut resources = vec![
        "{ id = 'topic', kind = 'doctrine', definition_source = 'topic.md', exposure = 'exported' }".to_owned(),
        "{ id = 'ceiling', kind = 'capability_policy', definition_source = 'ceiling.toml', exposure = 'internal' }".to_owned(),
    ];
    for kind in ["doctrine", "operation", "role", "policy_family", "stack"] {
        let exposure = if kind == "doctrine" {
            guide_exposure
        } else {
            "exported"
        };
        resources.push(format!("{{ id = 'topic/review', kind = '{kind}', definition_source = 'topic/review.md', exposure = '{exposure}' }}"));
    }
    std::fs::write(package.join("package.toml"), format!(
        "[package]\nid = 'example.pages'\nversion = '1.0.0'\nschema_version = 1\ntitle = 'Pages'\ndescription = 'Header reader fixture'\ncompatible_compilers = ['pale-surface@1']\nsources = [{{ path = 'topic.md', role = 'declaration' }}, {{ path = 'topic/review.md', role = 'declaration' }}, {{ path = 'ceiling.toml', role = 'declaration' }}]\nresources = [{}]\n", resources.join(", ")
    )).unwrap();
    std::fs::write(package.join("topic.md"), "+++\ndescription = 'Understand the change.'\n+++\n\n# Change\n\nRead the actual callers.\n").unwrap();
    std::fs::write(
        package.join("ceiling.toml"),
        "permitted_capabilities = ['repository.read']\npermitted_effects = ['read']\n",
    )
    .unwrap();
    std::fs::write(package.join("topic/review.md"), format!(
        "+++\ndescription = 'Review the change.'\n[guide]\nexposure = '{guide_exposure}'\n[task]\nroles = [{{ scope = 'local', resource_id = 'topic/review' }}]\ncapability_policy = {{ scope = 'local', resource_id = 'ceiling' }}\ncapability_requirements = [{{ id = 'source', description = 'Read the change.', capability = 'repository.read', effect_class = 'read', policy = {{ kind = 'required_block' }}, failure_posture = 'fail_closed' }}]\n[role]\n[policy]\n[stack]\n+++\n{BODY}"
    )).unwrap();
    root
}

fn admit(root: &std::path::Path) -> AdmittedPackageGraph {
    let snapshot =
        DistributionSnapshot::capture(&pale_adapter_fs::LocalSourceFilesystem, root).unwrap();
    let index = PackageReleaseIndex::capture(&snapshot).unwrap();
    let lock = DistributionLockFile {
        distribution: DistributionMeta {
            compiler: "pale-surface@1".into(),
            schema_manifest_sha256: SchemaManifestDigest::parse("0".repeat(64)).unwrap(),
            schema_set_sha256: SchemaSetDigest::parse("1".repeat(64)).unwrap(),
        },
        packages: index.releases().cloned().collect(),
    };
    let closure = index
        .select(&lock, &[reference("topic").package_id])
        .unwrap();
    assert!(matches!(
        closure
            .authoring_sources()
            .unwrap()
            .document("pages/topic/review.md"),
        Some(crate::workspace::AuthoringDocumentKind::PackagePage)
    ));
    AdmittedPackageGraph::admit(&snapshot, &closure, &mut Default::default()).unwrap()
}

#[test]
fn shared_reader_admits_five_uses_one_body_and_implicit_parent_reading() {
    let fixture = fixture("exported");
    let graph = admit(fixture.path());
    assert_eq!(graph.resources.len(), 7);
    assert_eq!(graph.bodies.len(), 2);
    assert_eq!(graph.bodies[&reference("topic/review")].text(), BODY);
    for kind in [
        PackageResourceKindV1::Doctrine,
        PackageResourceKindV1::Operation,
        PackageResourceKindV1::Role,
        PackageResourceKindV1::PolicyFamily,
        PackageResourceKindV1::Stack,
    ] {
        let node = &graph.resources[&PackageResourceUse::new(reference("topic/review"), kind)];
        assert_eq!(
            node.page.as_ref().unwrap().required,
            vec![reference("topic")]
        );
    }
    let guide = graph
        .clone()
        .select_and_seal(
            &SemanticSelection {
                doctrine: vec![PackageResourceSelectorV1::Exact {
                    resource: reference("topic/review"),
                }],
                ..SemanticSelection::default()
            },
            &BTreeMap::new(),
        )
        .unwrap()
        .into_resident_parts();
    assert_eq!(guide.resources.len(), 2);
    assert!(guide.capability_policies.is_empty());
    let task = graph
        .select_and_seal(
            &SemanticSelection {
                operations: vec![PackageResourceSelectorV1::Exact {
                    resource: reference("topic/review"),
                }],
                ..SemanticSelection::default()
            },
            &BTreeMap::new(),
        )
        .unwrap()
        .into_resident_parts();
    assert_eq!(task.resources.len(), 4);
    assert_eq!(task.bodies.len(), 2);
    assert_eq!(task.capability_policies.len(), 1);
}

#[test]
fn header_visibility_and_catalog_use_the_same_admitted_page() {
    let fixture = fixture("internal");
    let graph = admit(fixture.path());
    assert!(graph
        .clone()
        .select_and_seal(
            &SemanticSelection {
                doctrine: vec![PackageResourceSelectorV1::Exact {
                    resource: reference("topic/review")
                }],
                ..SemanticSelection::default()
            },
            &BTreeMap::new()
        )
        .is_err());
    assert!(graph
        .select_and_seal(
            &SemanticSelection {
                operations: vec![PackageResourceSelectorV1::Exact {
                    resource: reference("topic/review")
                }],
                ..SemanticSelection::default()
            },
            &BTreeMap::new()
        )
        .is_ok());
    let catalog =
        crate::packages::read_catalog(&pale_adapter_fs::LocalSourceFilesystem, fixture.path())
            .unwrap();
    let shown = catalog.show("example.pages:topic/review").unwrap();
    assert!(shown.starts_with("# Review\n"));
    assert!(shown.contains("Read first: [Change](<../topic.md>)"));
    assert!(shown.contains("Inspect the actual change."));
    assert!(!shown.contains("capability_requirements"));
    assert!(!shown.contains("+++"));
}

fn compact_fixture() -> tempfile::TempDir {
    let fixture = fixture("internal");
    let package = fixture.path().join("packages/pages");
    let path = package.join("package.toml");
    let mut manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let meta = manifest["package"].as_table_mut().unwrap();
    meta.remove("sources");
    meta.remove("resources");
    std::fs::write(path, toml::to_string(&manifest).unwrap()).unwrap();
    std::fs::write(package.join("ceiling.toml"), "kind = 'capability_policy'\nexposure = 'internal'\n[policy]\npermitted_capabilities = ['repository.read']\npermitted_effects = ['read']\n").unwrap();
    std::fs::write(package.join("reader.toml"), "kind = 'tool_recipe'\n[recipe]\ndescription = 'Read repository source.'\ncapability = 'repository.read'\ntool_contract = 'repository-reader.v1'\ntool_operation = 'read'\neffect_class = 'read'\ninput_mapping.repository = '$.repository'\nevidence.required_fields = ['/source']\nevidence.proof_ceiling = 'source-read'\n").unwrap();
    std::fs::write(
        package.join("order.toml"),
        "kind = 'governing_order'\n[governing_order]\nschema_version = 1\n",
    )
    .unwrap();
    std::fs::write(
        package.join("README.md"),
        "# Package notes\n\nDocumentation is retained without becoming a selectable method.\n",
    )
    .unwrap();
    fixture
}

#[test]
fn compact_manifest_derives_pages_and_typed_data() {
    let fixture = compact_fixture();
    let graph = admit(fixture.path());
    assert_eq!(graph.resources.len(), 9);
    assert_eq!(graph.bodies.len(), 2);
    assert_eq!(graph.bodies[&reference("topic/review")].text(), BODY);
    for (id, kind) in [
        ("reader", PackageResourceKindV1::ToolRecipe),
        ("ceiling", PackageResourceKindV1::CapabilityPolicy),
        ("order", PackageResourceKindV1::GoverningOrder),
    ] {
        assert!(graph
            .resources
            .contains_key(&PackageResourceUse::new(reference(id), kind)));
    }
    let selected = graph
        .select_and_seal(
            &SemanticSelection {
                operations: vec![PackageResourceSelectorV1::Exact {
                    resource: reference("topic/review"),
                }],
                ..Default::default()
            },
            &BTreeMap::new(),
        )
        .unwrap()
        .into_resident_parts();
    assert_eq!(selected.resources.len(), 4);
    assert_eq!(selected.capability_policies.len(), 1);
}

#[test]
fn compact_release_identity_includes_documentation_and_unselected_data() {
    let fixture = compact_fixture();
    let release = || {
        let snapshot =
            DistributionSnapshot::capture(&pale_adapter_fs::LocalSourceFilesystem, fixture.path())
                .unwrap();
        PackageReleaseIndex::capture(&snapshot)
            .unwrap()
            .releases()
            .next()
            .unwrap()
            .clone()
    };
    let initial = release();
    std::fs::write(
        fixture.path().join("packages/pages/README.md"),
        "# Changed Package notes\n",
    )
    .unwrap();
    let changed_notes = release();
    assert_eq!(initial.id, changed_notes.id);
    assert_ne!(initial.content_digest, changed_notes.content_digest);
    std::fs::write(fixture.path().join("packages/pages/order.toml"), "kind = 'governing_order'\n# A source comment is part of this exact release.\n[governing_order]\nschema_version = 1\n").unwrap();
    assert_ne!(changed_notes.content_digest, release().content_digest);
}

#[test]
fn compact_sources_reject_unknown_data_and_conflicting_path_owners() {
    for (path, text) in [
        ("ceiling.toml", "kind = 'policy'\n[policy]\npermitted_effects = ['read']\n"),
        ("ceiling.toml", "kind = 'capability_policy'\n[policy]\npermitted_effects = ['read']\nextra_permission = true\n"),
        ("topic.md", "+++\ndescription = 'Missing closing delimiter'\n"),
        ("ceiling.md", "+++\ndescription = 'Conflicting owner of the data file identity'\n+++\n\n# Conflict\n"),
    ] {
        let fixture = compact_fixture();
        std::fs::write(fixture.path().join("packages/pages").join(path), text).unwrap();
        let snapshot = DistributionSnapshot::capture(&pale_adapter_fs::LocalSourceFilesystem, fixture.path()).unwrap();
        assert!(PackageReleaseIndex::capture(&snapshot).is_err(), "{path}");
    }
}

#[test]
fn compact_governing_order_is_scoped_to_the_role_use_of_a_shared_page() {
    use PackageResourceKindV1 as Kind;
    let fixture = compact_fixture();
    let package = fixture.path().join("packages/pages");
    let review = package.join("topic/review.md");
    let source = std::fs::read_to_string(&review)
        .unwrap()
        .replace("exposure = 'internal'", "exposure = 'exported'");
    std::fs::write(review, source).unwrap();
    std::fs::write(package.join("topic.md"), "+++\ndescription = 'Understand the change.'\n[guide]\n[policy]\n+++\n\n# Change\n\nRead the actual callers.\n").unwrap();
    std::fs::write(package.join("order.toml"), "kind = 'governing_order'\n[governing_order]\nschema_version = 1\n[[governing_order.order]]\npurpose = 'reasoning'\nscope = { scope = 'local', resource_id = 'topic/review', kind = 'role' }\nbefore = { scope = 'local', resource_id = 'topic', kind = 'policy_family' }\nafter = { scope = 'local', resource_id = 'topic/review', kind = 'policy_family' }\n").unwrap();
    let graph = admit(fixture.path());
    let selector = PackageResourceSelectorV1::Exact {
        resource: reference("topic/review"),
    };
    let guide = graph
        .clone()
        .select_and_seal(
            &SemanticSelection {
                doctrine: vec![selector.clone()],
                ..Default::default()
            },
            &BTreeMap::new(),
        )
        .unwrap();
    assert!(guide.profile_governing_order().order().is_empty());
    assert_eq!(guide.resource_count(), 2);
    let role = graph
        .select_and_seal(
            &SemanticSelection {
                roles: vec![selector],
                ..Default::default()
            },
            &BTreeMap::new(),
        )
        .unwrap();
    let order = role.profile_governing_order().order();
    assert_eq!(order.len(), 1);
    assert_eq!(
        order[0].scope,
        PackageResourceUse::new(reference("topic/review"), Kind::Role)
    );
    assert_eq!(
        order[0].after,
        PackageResourceUse::new(reference("topic/review"), Kind::PolicyFamily)
    );
    assert!(role.resources().contains_key(&order[0].before));
    assert!(role.resources().contains_key(&order[0].after));
    assert!(!role.resources().contains_key(&PackageResourceUse::new(
        reference("topic/review"),
        Kind::Operation
    )));
}
