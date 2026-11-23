//! Initial-v1 public Package inspection laws.

use pale_application::packages::{inspect_distribution, read_catalog, PackageError};

use crate::common::{write_manifest, write_policy_package};

#[test]
fn catalog_reads_current_sources_through_the_compilers_admission_path() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog = read_catalog(&pale_adapter_fs::LocalSourceFilesystem, &root).unwrap();
    let catalog = serde_json::to_value(catalog).unwrap();
    let packages = catalog["packages"].as_array().unwrap();
    assert_eq!(packages.len(), 14);
    let resources = catalog["resources"].as_array().unwrap();
    let task = resources
        .iter()
        .find(|resource| resource["package"] == "pale.automation" && resource["id"] == "check")
        .unwrap();
    assert_eq!(task["body"], "check.md");
    assert!(task["record"]["task"]["outputs"].as_array().unwrap().len() >= 3);
}

#[test]
fn catalog_rejects_broken_fragments_even_without_a_workspace_selection() {
    let root = tempfile::tempdir().unwrap();
    write_policy_package(
        root.path(),
        "example",
        "example.policy",
        "# Policy\n\n[read](#policy)\n",
    );
    read_catalog(&pale_adapter_fs::LocalSourceFilesystem, root.path()).unwrap();
    for destination in ["#missing", "#missing:part", "#missing%3Apart"] {
        std::fs::write(
            root.path().join("packages/example/body.md"),
            format!("# Policy\n\n[read]({destination})\n"),
        )
        .unwrap();
        assert!(
            matches!(
                read_catalog(&pale_adapter_fs::LocalSourceFilesystem, root.path()),
                Err(PackageError::Diagnostics(_))
            ),
            "{destination}"
        );
    }
}

#[test]
fn catalog_checks_colon_fragments_but_leaves_uri_schemes_external() {
    let root = tempfile::tempdir().unwrap();
    let package = write_manifest(
        root.path(),
        "example",
        "example.guide",
        "doctrine.toml",
        "body.md",
        "{ id = 'guide', kind = 'doctrine', definition_source = 'doctrine.toml', exposure = 'exported' }",
    );
    std::fs::write(
        package.join("doctrine.toml"),
        "[doctrine]\nschema_version = 1\n[[doctrine.entry]]\nid = 'guide'\ndescription = 'Read the guide.'\ndoctrine = 'body.md'\nplacement = 'standalone_judgment'\n",
    ).unwrap();
    std::fs::write(
        package.join("body.md"),
        concat!(
            "# Guide\n\n<a id=\"topic:part\"></a>\n\n",
            "[local](#topic:part) [encoded](#topic%3Apart)\n\n",
            "[named](body.md#topic:part) [relative](./body.md#topic:part)\n\n",
            "[web](https://example.com/page#missing:part)\n",
            "[mail](mailto:reader@example.com) [custom](web+demo:item)\n",
            "[network](//example.com/page#missing:part)\n",
        ),
    )
    .unwrap();
    read_catalog(&pale_adapter_fs::LocalSourceFilesystem, root.path()).unwrap();
    for destination in [
        "body.md#absent:part",
        "./body.md#absent:part",
        "./missing.md#topic:part",
        "folder/missing:part.md",
        "./missing:part.md",
    ] {
        std::fs::write(
            root.path().join("packages/example/body.md"),
            format!("# Policy\n\n[read]({destination})\n"),
        )
        .unwrap();
        assert!(
            matches!(
                read_catalog(&pale_adapter_fs::LocalSourceFilesystem, root.path()),
                Err(PackageError::Diagnostics(_))
            ),
            "{destination}"
        );
    }
}

#[test]
fn inspection_returns_canonical_release_coordinates_without_source_bytes() {
    let root = tempfile::tempdir().expect("temporary distribution");
    write_policy_package(root.path(), "zeta", "example.zeta", "private zeta body");
    write_policy_package(root.path(), "alpha", "example.alpha", "private alpha body");

    let info = inspect_distribution(&pale_adapter_fs::LocalSourceFilesystem, root.path())
        .expect("synthetic distribution is admissible");
    assert_eq!(
        info.compiler(),
        pale_application::compile::DISTRIBUTION_COMPILER_ID
    );
    assert_eq!(
        info.packages()
            .iter()
            .map(|release| release.id.as_str())
            .collect::<Vec<_>>(),
        ["example.alpha", "example.zeta"]
    );
    let registry = pale_contracts::schema_registry_identity().expect("embedded registry");
    assert_eq!(
        info.schema_manifest_sha256(),
        &registry.schema_manifest_sha256
    );
    assert_eq!(info.schema_set_sha256(), &registry.schema_set_sha256);
    let wire = serde_json::to_value(&info).expect("inspection output serializes");
    assert_eq!(
        wire["compiler"],
        serde_json::json!(pale_application::compile::DISTRIBUTION_COMPILER_ID)
    );
    assert_eq!(
        wire["packages"]
            .as_array()
            .expect("Package release array")
            .iter()
            .map(|release| release["id"].as_str().expect("Package id"))
            .collect::<Vec<_>>(),
        ["example.alpha", "example.zeta"]
    );
    let encoded = serde_json::to_string(&info).expect("inspection output serializes");
    assert!(!encoded.contains("private alpha body"));
    assert!(!encoded.contains("private zeta body"));
}

#[test]
fn inspection_binds_body_bytes_and_rejects_orphan_members() {
    let root = tempfile::tempdir().expect("temporary distribution");
    write_policy_package(root.path(), "alpha", "example.alpha", "first body");
    let first = serde_json::to_value(
        inspect_distribution(&pale_adapter_fs::LocalSourceFilesystem, root.path())
            .expect("first distribution is admissible"),
    )
    .expect("inspection output serializes");

    std::fs::write(root.path().join("packages/alpha/body.md"), "second body")
        .expect("replace synthetic body");
    let second = serde_json::to_value(
        inspect_distribution(&pale_adapter_fs::LocalSourceFilesystem, root.path())
            .expect("changed distribution remains admissible"),
    )
    .expect("inspection output serializes");
    assert_ne!(
        first["packages"][0]["content_digest"],
        second["packages"][0]["content_digest"]
    );

    std::fs::write(root.path().join("packages/alpha/orphan.md"), "orphan")
        .expect("write undeclared member");
    assert!(matches!(
        inspect_distribution(&pale_adapter_fs::LocalSourceFilesystem, root.path()),
        Err(PackageError::SourceInventoryMismatch { ref undeclared, .. })
            if undeclared == &vec!["orphan.md".to_string()]
    ));
}
