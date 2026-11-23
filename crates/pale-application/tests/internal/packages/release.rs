//! Package release identity and closure tests.

use pale_core::paths::OwnedConfinedRelativePath;

use super::*;

#[cfg(unix)]
use crate::package_fixtures::{write_policy_manifest, write_policy_package};

fn digest(value: &[u8]) -> ArtifactDigest {
    ArtifactDigest::hash_bytes(value)
}

fn retained(
    id: &str,
    version: &str,
    dependencies: Vec<pale_core::records::PackageDependencyV1>,
    content: &[u8],
) -> RetainedPackageRelease {
    let id = PackageIdV1::parse(id).expect("valid Package id");
    let version =
        pale_core::records::PackageVersionV1::parse(version).expect("valid Package version");
    let declaration =
        OwnedConfinedRelativePath::parse("family.toml").expect("valid declaration path");
    RetainedPackageRelease {
        typed_data_sources: BTreeSet::new(),
        directory: PathBuf::new(),
        manifest: AgentSurfacePackageV1 {
            package: pale_core::records::PackageMeta {
                id: id.clone(),
                version: version.clone(),
                schema_version: 1,
                title: "Synthetic".to_string(),
                description: "Exact closure fixture".to_string(),
                compatible_compilers: vec!["pale-surface@1".to_string()],
                dependencies,
                sources: vec![pale_core::records::PackageSourceV1 {
                    path: declaration.clone(),
                    role: PackageSourceRoleV1::Declaration,
                }],
                resources: vec![pale_core::records::PackageResourceV1 {
                    id: pale_core::records::PackageResourceIdV1::parse("policy")
                        .expect("valid resource id"),
                    kind: pale_core::records::PackageResourceKindV1::PolicyFamily,
                    definition_source: declaration,
                    exposure: pale_core::records::PackageResourceExposureV1::Exported,
                }],
            },
            license: None,
        },
        release: PackageReleaseV1 {
            id,
            version,
            content_digest: PackageDigest::hash_bytes(content),
        },
    }
}

fn dependency(id: &str, version: &str) -> pale_core::records::PackageDependencyV1 {
    pale_core::records::PackageDependencyV1 {
        id: PackageIdV1::parse(id).expect("valid dependency id"),
        version: pale_core::records::PackageVersionV1::parse(version)
            .expect("valid dependency version"),
    }
}

#[test]
fn release_digest_is_source_order_invariant() {
    let manifest = digest(b"manifest");
    let first = package_content_digest(
        &manifest,
        [
            ("b.md", PackageSourceRoleV1::Body, digest(b"b")),
            ("a.toml", PackageSourceRoleV1::Declaration, digest(b"a")),
        ],
    );
    let second = package_content_digest(
        &manifest,
        [
            ("a.toml", PackageSourceRoleV1::Declaration, digest(b"a")),
            ("b.md", PackageSourceRoleV1::Body, digest(b"b")),
        ],
    );
    assert_eq!(first, second);
}

#[test]
fn release_digest_binds_manifest_member_bytes_inventory_and_roles() {
    let manifest = digest(b"manifest");
    let baseline = package_content_digest(
        &manifest,
        [("body.md", PackageSourceRoleV1::Body, digest(b"body"))],
    );
    let changed_manifest = package_content_digest(
        &digest(b"changed-manifest"),
        [("body.md", PackageSourceRoleV1::Body, digest(b"body"))],
    );
    let changed_body = package_content_digest(
        &manifest,
        [("body.md", PackageSourceRoleV1::Body, digest(b"changed"))],
    );
    let missing = package_content_digest(&manifest, []);
    let extra = package_content_digest(
        &manifest,
        [
            ("body.md", PackageSourceRoleV1::Body, digest(b"body")),
            (
                "extra.toml",
                PackageSourceRoleV1::Declaration,
                digest(b"extra"),
            ),
        ],
    );
    let changed_role = package_content_digest(
        &manifest,
        [(
            "body.md",
            PackageSourceRoleV1::Documentation,
            digest(b"body"),
        )],
    );
    for changed in [changed_manifest, changed_body, missing, extra, changed_role] {
        assert_ne!(baseline, changed);
    }
}

#[test]
fn dependency_closure_requires_the_exact_locked_version_and_content_digest() {
    let root = retained(
        "example.root",
        "1.0.0",
        vec![dependency("example.dependency", "2.0.0")],
        b"root",
    );
    let dependency = retained("example.dependency", "2.0.0", Vec::new(), b"dependency");
    let index = PackageReleaseIndex {
        packages: BTreeMap::from([
            (root.release.id.clone(), root.clone()),
            (dependency.release.id.clone(), dependency.clone()),
        ]),
    };
    let exact_lock = lock(vec![root.release.clone(), dependency.release.clone()]);
    let closure = index
        .select(
            &exact_lock,
            &[PackageIdV1::parse("example.root").expect("valid root id")],
        )
        .expect("exact direct dependency closes");
    assert_eq!(
        closure
            .releases()
            .iter()
            .map(|release| release.id.as_str())
            .collect::<Vec<_>>(),
        ["example.dependency", "example.root"]
    );

    let wrong_version = retained("example.dependency", "3.0.0", Vec::new(), b"dependency");
    assert!(matches!(
        index.select(
            &lock(vec![root.release.clone(), wrong_version.release]),
            &[PackageIdV1::parse("example.root").expect("valid root id")],
        ),
        Err(PackageError::MissingExactDependency { .. })
    ));

    let mut incompatible = root.clone();
    incompatible.manifest.package.compatible_compilers = vec!["other@1".to_string()];
    let incompatible_index = PackageReleaseIndex {
        packages: BTreeMap::from([
            (incompatible.release.id.clone(), incompatible.clone()),
            (dependency.release.id.clone(), dependency.clone()),
        ]),
    };
    assert!(matches!(
        incompatible_index.select(
            &lock(vec![
                incompatible.release.clone(),
                dependency.release.clone(),
            ]),
            &[PackageIdV1::parse("example.root").expect("valid root id")],
        ),
        Err(PackageError::IncompatibleCompiler { package, compiler })
            if package.as_str() == "example.root" && compiler == "pale-surface@1"
    ));

    let mut wrong_digest = dependency.release.clone();
    wrong_digest.content_digest = PackageDigest::hash_bytes(b"other bytes");
    assert!(matches!(
        index.select(
            &lock(vec![root.release, wrong_digest]),
            &[PackageIdV1::parse("example.root").expect("valid root id")],
        ),
        Err(PackageError::ReleaseMismatch { package })
            if package.as_str() == "example.dependency"
    ));
}

#[cfg(unix)]
#[test]
fn unselected_package_byte_change_does_not_change_selected_closure() {
    let root = tempfile::tempdir().expect("temporary distribution must be available");
    write_policy_package(root.path(), "a", "example.a", "alpha");
    write_policy_package(root.path(), "b", "example.b", "bravo");
    let first = DistributionSnapshot::capture(&pale_adapter_fs::LocalSourceFilesystem, root.path())
        .expect("synthetic distribution must be capturable");
    let first_index =
        PackageReleaseIndex::capture(&first).expect("synthetic manifests must be admissible");
    let lock = lock(first_index.releases().cloned().collect());
    let selected_id = PackageIdV1::parse("example.a").expect("static Package id is valid");
    let first_closure = first_index
        .select(&lock, std::slice::from_ref(&selected_id))
        .expect("selected release must match its lock")
        .releases()
        .to_vec();

    std::fs::write(
        root.path().join("packages/b/body.md"),
        "changed unselected body",
    )
    .expect("unselected fixture body must be mutable");
    let second =
        DistributionSnapshot::capture(&pale_adapter_fs::LocalSourceFilesystem, root.path())
            .expect("changed distribution must remain capturable");
    let second_index = PackageReleaseIndex::capture(&second)
        .expect("changed unselected manifest must remain admissible");
    let second_closure = second_index
        .select(&lock, &[selected_id])
        .expect("unselected release mismatch must not contaminate selection")
        .releases()
        .to_vec();
    assert_eq!(first_closure, second_closure);
}

#[cfg(unix)]
#[test]
fn retained_source_inventory_rejects_missing_and_extra_members() {
    let missing_root = tempfile::tempdir().expect("temporary distribution must be available");
    write_policy_manifest(missing_root.path(), "a", "example.a");
    std::fs::write(
        missing_root.path().join("packages/a/family.toml"),
        "id = 'policy'\ntitle = 'Policy'\npurpose = 'Proof'\nbody = 'body.md'\n",
    )
    .expect("declaration fixture must be writable");
    let missing =
        DistributionSnapshot::capture(&pale_adapter_fs::LocalSourceFilesystem, missing_root.path())
            .expect("missing-member tree remains structurally capturable");
    assert!(matches!(
        PackageReleaseIndex::capture(&missing),
        Err(PackageError::SourceInventoryMismatch { ref missing, .. }) if missing == &["body.md"]
    ));

    let extra_root = tempfile::tempdir().expect("temporary distribution must be available");
    write_policy_package(extra_root.path(), "a", "example.a", "alpha");
    std::fs::write(extra_root.path().join("packages/a/extra.md"), "extra")
        .expect("extra fixture member must be writable");
    let extra =
        DistributionSnapshot::capture(&pale_adapter_fs::LocalSourceFilesystem, extra_root.path())
            .expect("extra-member tree remains structurally capturable");
    assert!(matches!(
        PackageReleaseIndex::capture(&extra),
        Err(PackageError::SourceInventoryMismatch { ref undeclared, .. }) if undeclared == &["extra.md"]
    ));
}

#[cfg(unix)]
fn lock(packages: Vec<PackageReleaseV1>) -> DistributionLockFile {
    pale_core::records::DistributionLockFile {
        distribution: pale_core::records::DistributionMeta {
            compiler: "pale-surface@1".to_string(),
            schema_manifest_sha256: pale_core::records::SchemaManifestDigest::parse("0".repeat(64))
                .expect("static digest is valid"),
            schema_set_sha256: pale_core::records::SchemaSetDigest::parse("1".repeat(64))
                .expect("static digest is valid"),
        },
        packages,
    }
}
