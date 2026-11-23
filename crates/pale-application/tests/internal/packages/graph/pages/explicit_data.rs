//! Explicit inventories and compact inventories read the same typed data.

use super::*;
use crate::workspace::{AuthoringDocumentKind, AuthoringTomlGrammar};

fn explicit_fixture() -> tempfile::TempDir {
    let fixture = compact_fixture();
    let snapshot =
        DistributionSnapshot::capture(&pale_adapter_fs::LocalSourceFilesystem, fixture.path())
            .unwrap();
    let path = fixture.path().join("packages/pages/package.toml");
    let mut manifest = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    crate::packages::inventory::derive(&snapshot, std::path::Path::new("pages"), &mut manifest)
        .unwrap();
    std::fs::write(path, toml::to_string(&manifest).unwrap()).unwrap();
    fixture
}

fn update_manifest(fixture: &tempfile::TempDir, update: impl FnOnce(&mut toml::Value)) {
    let path = fixture.path().join("packages/pages/package.toml");
    let mut manifest = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    update(&mut manifest);
    std::fs::write(path, toml::to_string(&manifest).unwrap()).unwrap();
}

#[test]
fn explicit_inventory_accepts_all_typed_data_and_retains_held_documentation() {
    let fixture = explicit_fixture();
    let package = fixture.path().join("packages/pages");
    std::fs::write(
        package.join("held.toml"),
        "kind = 'old-format'\nnot valid TOML",
    )
    .unwrap();
    std::fs::write(
        package.join("legacy.toml"),
        "permitted_capabilities = ['repository.read']\npermitted_effects = ['read']\n",
    )
    .unwrap();
    update_manifest(&fixture, |manifest| {
        let sources = manifest["package"]["sources"].as_array_mut().unwrap();
        for (path, role) in [
            ("held.toml", "documentation"),
            ("legacy.toml", "declaration"),
        ] {
            sources.push(toml::from_str(&format!("path = '{path}'\nrole = '{role}'")).unwrap());
        }
        manifest["package"]["resources"].as_array_mut().unwrap().push(
            toml::from_str("id = 'legacy-ceiling'\nkind = 'capability_policy'\ndefinition_source = 'legacy.toml'\nexposure = 'internal'").unwrap(),
        );
    });
    let graph = admit(fixture.path());
    assert_eq!(graph.resources.len(), 10);
    for (id, kind) in [
        ("reader", PackageResourceKindV1::ToolRecipe),
        ("ceiling", PackageResourceKindV1::CapabilityPolicy),
        ("order", PackageResourceKindV1::GoverningOrder),
        ("legacy-ceiling", PackageResourceKindV1::CapabilityPolicy),
    ] {
        assert!(graph
            .resources
            .contains_key(&PackageResourceUse::new(reference(id), kind)));
    }
    let snapshot =
        DistributionSnapshot::capture(&pale_adapter_fs::LocalSourceFilesystem, fixture.path())
            .unwrap();
    let index = PackageReleaseIndex::capture(&snapshot).unwrap();
    let initial = index.releases().next().unwrap().clone();
    let lock = DistributionLockFile {
        distribution: DistributionMeta {
            compiler: "pale-surface@1".into(),
            schema_manifest_sha256: SchemaManifestDigest::parse("0".repeat(64)).unwrap(),
            schema_set_sha256: SchemaSetDigest::parse("1".repeat(64)).unwrap(),
        },
        packages: vec![initial.clone()],
    };
    let closure = index
        .select(&lock, &[reference("topic").package_id])
        .unwrap();
    let sources = closure.authoring_sources().unwrap();
    for path in ["reader.toml", "ceiling.toml", "order.toml"] {
        assert_eq!(
            sources.document(&format!("pages/{path}")),
            Some(AuthoringDocumentKind::Toml(AuthoringTomlGrammar::record(
                "package-data.v1"
            )))
        );
    }
    assert_eq!(
        sources.document("pages/held.toml"),
        Some(AuthoringDocumentKind::Text)
    );
    assert_eq!(
        sources.document("pages/legacy.toml"),
        Some(AuthoringDocumentKind::Toml(
            AuthoringTomlGrammar::definition("capability-procedure.v1", "CapabilityPolicy")
        ))
    );
    std::fs::write(package.join("held.toml"), "Still held; changed bytes.").unwrap();
    let snapshot =
        DistributionSnapshot::capture(&pale_adapter_fs::LocalSourceFilesystem, fixture.path())
            .unwrap();
    let changed = PackageReleaseIndex::capture(&snapshot).unwrap();
    assert_ne!(
        initial.content_digest,
        changed.releases().next().unwrap().content_digest
    );
}

#[test]
fn explicit_typed_data_must_match_path_kind_exposure_and_cardinality() {
    for mismatch in ["id", "kind", "exposure", "extra"] {
        let fixture = explicit_fixture();
        update_manifest(&fixture, |manifest| {
            let resources = manifest["package"]["resources"].as_array_mut().unwrap();
            let declaration = resources
                .iter_mut()
                .find(|resource| resource["id"].as_str() == Some("ceiling"))
                .unwrap();
            if mismatch == "extra" {
                let mut extra = declaration.clone();
                extra["id"] = "second-ceiling".into();
                resources.push(extra);
            } else {
                declaration[mismatch] = match mismatch {
                    "id" => "renamed-ceiling",
                    "kind" => "tool_recipe",
                    "exposure" => "exported",
                    _ => unreachable!(),
                }
                .into();
            }
        });
        let error =
            crate::packages::read_catalog(&pale_adapter_fs::LocalSourceFilesystem, fixture.path())
                .unwrap_err();
        assert!(
            error.to_string().contains("declaration"),
            "{mismatch}: {error}"
        );
    }
}

#[test]
fn explicit_typed_data_rejects_unknown_discriminators_and_fields() {
    for text in [
        "kind = 'policy'\n[policy]\npermitted_effects = ['read']\n",
        "kind = 'capability_policy'\n[policy]\npermitted_effects = ['read']\nextra_permission = true\n",
    ] {
        let fixture = explicit_fixture();
        std::fs::write(fixture.path().join("packages/pages/ceiling.toml"), text).unwrap();
        assert!(crate::packages::read_catalog(&pale_adapter_fs::LocalSourceFilesystem, fixture.path()).is_err());
    }
}

#[test]
fn source_classification_leaves_syntax_rejection_to_selected_semantic_admission() {
    let fixture = explicit_fixture();
    std::fs::write(
        fixture.path().join("packages/pages/ceiling.toml"),
        "kind = 'capability_policy'\n[policy\n",
    )
    .unwrap();
    let snapshot =
        DistributionSnapshot::capture(&pale_adapter_fs::LocalSourceFilesystem, fixture.path())
            .unwrap();
    assert!(PackageReleaseIndex::capture(&snapshot).is_ok());
    assert!(
        crate::packages::read_catalog(&pale_adapter_fs::LocalSourceFilesystem, fixture.path())
            .is_err()
    );
}
