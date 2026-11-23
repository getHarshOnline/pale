//! Source-fixture identity laws that run before portable compilation goldens.

use pale_core::records::{
    SchemaDigest, WorkspaceCompositionSourceV1, WorkspaceId, WorkspaceResolutionLockV1,
    WorkspaceUniverseLockV1,
};
use pale_sdk::{CompilationDocument, PackageDistributionRequest, PaleApi};
use std::path::{Path, PathBuf};

fn assert_bytes(path: &Path, bytes: &[u8]) {
    if std::env::var_os("PALE_UPDATE_CONFORMANCE").as_deref() == Some("1".as_ref()) {
        std::fs::write(path, bytes).expect("canonical source fixture writes");
    } else {
        assert_eq!(std::fs::read(path).unwrap(), bytes, "{}", path.display());
    }
}

#[test]
fn empty_source_locks_and_distribution_pins_match_their_owners() {
    let update = std::env::var_os("PALE_UPDATE_CONFORMANCE").as_deref() == Some("1".as_ref());
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(if update {
        "../../contracts/conformance"
    } else {
        "tests/fixtures"
    });
    let resolution = WorkspaceResolutionLockV1::seal(Vec::new()).unwrap();
    for (directory, resolution_name, universe_name, workspace) in [
        (
            "cross-leaf.v1/portable",
            "resolution.lock.toml",
            "universe.lock.toml",
            "empty-adopter",
        ),
        (
            "cross-leaf.v1/one-shot/workspace",
            "workspace-resolution.lock.toml",
            "workspace-universe.lock.toml",
            "act-ws",
        ),
        (
            if update {
                "workspace-language.v1/empty-adopter"
            } else {
                "workspace-language/empty-adopter"
            },
            "resolution.lock.toml",
            "universe.lock.toml",
            "empty-adopter",
        ),
    ] {
        let universe = WorkspaceUniverseLockV1::seal(
            WorkspaceId::parse(workspace.to_owned()).unwrap(),
            resolution.resolution_digest.clone(),
            Vec::new(),
        )
        .unwrap();
        assert_bytes(
            &root.join(directory).join(resolution_name),
            toml::to_string(&resolution).unwrap().as_bytes(),
        );
        assert_bytes(
            &root.join(directory).join(universe_name),
            toml::to_string(&universe).unwrap().as_bytes(),
        );
    }
    let path = root.join("cross-leaf.v1/one-shot/private/distribution-lock.toml");
    let mut document: toml::Value =
        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let api = PaleApi::new().unwrap();
    let inspection = api
        .inspect_package_distribution(PackageDistributionRequest::new(
            ["package.toml", "roles.toml", "role.md"]
                .into_iter()
                .map(|name| {
                    CompilationDocument::new(
                        format!("engine/{name}"),
                        std::fs::read(
                            root.join("cross-leaf.v1/one-shot/packages/engine")
                                .join(name),
                        )
                        .unwrap(),
                    )
                })
                .collect(),
        ))
        .unwrap();
    assert_eq!(inspection.packages().len(), 1);
    let table = document["distribution"].as_table_mut().unwrap();
    for (key, value) in [
        ("compiler", inspection.compiler()),
        (
            "schema_manifest_sha256",
            inspection.schema_manifest_sha256(),
        ),
        ("schema_set_sha256", inspection.schema_set_sha256()),
    ] {
        assert!(
            table.contains_key(key),
            "fixture retains its existing pin field"
        );
        table.insert(key.to_owned(), toml::Value::String(value.to_owned()));
    }
    let packages = document["package"].as_array_mut().unwrap();
    assert_eq!(packages.len(), 1);
    let package = inspection.packages().first().unwrap();
    assert_eq!(packages[0]["id"].as_str(), Some(package.id()));
    assert_eq!(packages[0]["version"].as_str(), Some(package.version()));
    packages[0]["content_digest"] = toml::Value::String(package.content_digest().to_owned());
    assert_bytes(&path, toml::to_string(&document).unwrap().as_bytes());

    let path = root.join("cross-leaf.v1/one-shot/workspace/composition.toml");
    let mut composition: WorkspaceCompositionSourceV1 =
        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    for binding in &mut composition.declarations.provider_bindings {
        for identity in &mut binding.responsibility_schema_identities {
            let descriptor = pale_contracts::schema_descriptor(identity.schema_id.as_str())
                .expect("fixture schema pin names an admitted registry member");
            identity.schema_sha256 =
                SchemaDigest::parse(descriptor.schema_sha256.to_owned()).unwrap();
        }
        binding
            .generation_coordinates
            .deployment_activation_identity =
            binding.recompute_deployment_activation_identity().unwrap();
        binding.binding_identity = binding.recompute_identity().unwrap();
    }
    for path in &mut composition.declarations.selected_experience_paths {
        path.identity = path.recompute_identity().unwrap();
    }
    assert_bytes(&path, toml::to_string(&composition).unwrap().as_bytes());
}
