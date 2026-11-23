//! Package record contract tests.

use super::*;

#[test]
fn resource_paths_are_confined_and_do_not_change_package_identity_grammar() {
    for path in ["motion", "motion/time-models", "web/css", "tools/v1.check"] {
        let id = PackageResourceIdV1::parse(path).unwrap();
        assert_eq!(id.as_str(), path);
        let wire = serde_json::to_string(&id).unwrap();
        assert_eq!(
            serde_json::from_str::<PackageResourceIdV1>(&wire).unwrap(),
            id
        );
    }
    assert!(PackageIdV1::parse("motion/time-models").is_err());
    for path in [
        "",
        "/motion",
        "motion/",
        "motion//time",
        "../motion",
        "motion/../time",
        "motion/./time",
        "motion\\time",
        "motion/%2e%2e",
        "motion/time#heading",
        "motion/time?mode=1",
        "motion/Time",
        "motion/ time",
        "motion/1time",
        "motion/éclair",
    ] {
        assert!(PackageResourceIdV1::parse(path).is_err(), "{path}");
    }
    let maximum = format!("a/{}", "b".repeat(MAX_PACKAGE_ID_BYTES - 2));
    assert!(PackageResourceIdV1::parse(&maximum).is_ok());
    assert!(PackageResourceIdV1::parse(format!("{maximum}b")).is_err());
}

#[test]
fn package_manifest_rejects_unconfined_inventory_members() {
    let value = serde_json::json!({
        "package": {
            "id": "example.review",
            "version": "1.0.0",
            "schema_version": 1,
            "title": "Review",
            "description": "Synthetic review intelligence.",
            "compatible_compilers": ["pale-engine-v1"],
            "dependencies": [],
            "sources": [{"path": "../outside.toml", "role": "declaration"}],
            "resources": []
        }
    });

    assert!(serde_json::from_value::<AgentSurfacePackageV1>(value).is_err());
}

#[test]
fn resource_wildcard_is_scoped_to_one_package() {
    let selector = PackageResourceSelectorV1::AllFromPackage {
        package_id: PackageIdV1::parse("example.review")
            .expect("synthetic Package id must be valid"),
    };
    let value = serde_json::to_value(selector).expect("selector must serialize");

    assert_eq!(value["selection"], "all_from_package");
    assert_eq!(value["package_id"], "example.review");
    assert!(value.get("resource_id").is_none());
}

#[test]
fn package_manifest_has_no_parallel_import_identity_system() {
    let schema = schemars::schema_for!(AgentSurfacePackageV1);
    let value = serde_json::to_value(schema).expect("schema must serialize");
    let package = &value["$defs"]["PackageMeta"]["properties"];

    assert!(package.get("resources").is_some());
    assert!(package.get("sources").is_some());
    assert!(package.get("imports").is_none());
    assert!(package.get("families").is_none());
    assert!(package.get("namespace").is_none());
}

#[test]
fn package_manifest_validation_rejects_another_contract_major() {
    let value = serde_json::json!({
        "package": {
            "id": "example.review",
            "version": "1.0.0",
            "schema_version": 2,
            "title": "Review",
            "description": "Synthetic review intelligence.",
            "compatible_compilers": ["pale-engine-v1"],
            "dependencies": [],
            "sources": [{"path": "operations.toml", "role": "declaration"}],
            "resources": [{
                "id": "review",
                "kind": "operation",
                "definition_source": "operations.toml",
                "exposure": "exported"
            }]
        }
    });
    let package: AgentSurfacePackageV1 =
        serde_json::from_value(value).expect("wire parsing does not imply admission");

    assert!(matches!(
        package.validate_contract(),
        Err(InvalidPackageManifestV1::UnsupportedSchemaVersion(2))
    ));
}

#[test]
fn page_uses_share_one_declaration_but_never_compete_for_an_identity() {
    let value = serde_json::json!({"package": {
        "id": "example.review", "version": "1.0.0", "schema_version": 1,
        "title": "Review", "description": "One method.", "compatible_compilers": ["pale-surface@1"],
        "sources": [{"path": "review.md", "role": "declaration"}, {"path": "other.md", "role": "declaration"}],
        "resources": [
            {"id": "review", "kind": "doctrine", "definition_source": "review.md", "exposure": "internal"},
            {"id": "review", "kind": "operation", "definition_source": "review.md", "exposure": "exported"}
        ]
    }});
    let manifest: AgentSurfacePackageV1 = serde_json::from_value(value).unwrap();
    manifest.validate_contract().unwrap();
    let mut duplicate = manifest.clone();
    duplicate
        .package
        .resources
        .push(duplicate.package.resources[0].clone());
    assert!(matches!(
        duplicate.validate_contract(),
        Err(InvalidPackageManifestV1::DuplicateResource { .. })
    ));
    let mut competing = manifest;
    competing.package.resources[1].definition_source =
        OwnedConfinedRelativePath::parse("other.md").unwrap();
    assert!(matches!(
        competing.validate_contract(),
        Err(InvalidPackageManifestV1::DuplicateResource { .. })
    ));
}
