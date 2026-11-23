//! Canonical control-store location and confinement laws.

use std::os::unix::fs::{MetadataExt, PermissionsExt};

use pale_core::records::{
    ArtifactDigest, ResidentPrincipalIdV1, ResidentRecordRefV1, ResidentWorkspaceIdV1,
};

use super::*;

fn principal(raw: &str) -> ControlStoreScope {
    ControlStoreScope::Principal(PrincipalStoreScope::new(
        ResidentPrincipalIdV1::parse(raw).unwrap(),
    ))
}

fn deployment(workspace: &str, binding_ref: &str, digest: char) -> ControlStoreScope {
    ControlStoreScope::Deployment(DeploymentStoreScope::new(
        ResidentWorkspaceIdV1::parse(workspace).unwrap(),
        ResidentRecordRefV1::parse(binding_ref).unwrap(),
        ArtifactDigest::parse(digest.to_string().repeat(64)).unwrap(),
    ))
}

#[test]
fn locator_hashes_are_exact_deterministic_and_domain_separated() {
    let principal =
        principal_segment(&ResidentPrincipalIdV1::parse("same").expect("principal identity"));
    let workspace =
        workspace_segment(&ResidentWorkspaceIdV1::parse("same").expect("workspace identity"));
    let binding_scope = deployment("same", "same", 'a');
    let binding = deployment_binding_segment(match &binding_scope {
        ControlStoreScope::Deployment(scope) => scope,
        ControlStoreScope::Principal(_) => unreachable!(),
    })
    .unwrap();

    assert_eq!(
        principal,
        "p-9b6c51b30caa5e684f8b4e1a215ddc8d6bdc90c17f1338dd23e1bfd5cb9725c8"
    );
    assert_eq!(
        workspace,
        "w-79f1bd3a7f64cdbe83b200e6058dd0bbfee8699d0203ecf3696470a6226ada60"
    );
    assert_eq!(
        binding,
        "b-f307c0574a9f35475afe4669b72f31760e2453ed4debcfba1c44b2d4905e4745"
    );
    assert_ne!(&principal[2..], &workspace[2..]);
    assert_ne!(&workspace[2..], &binding[2..]);

    let (first, _) = location_components(&deployment("workspace-one", "same", 'a')).unwrap();
    let (second, _) = location_components(&deployment("workspace-two", "same", 'a')).unwrap();
    assert_ne!(first[2], second[2]);
    assert_eq!(first[3], second[3]);
}

#[test]
fn raw_path_syntax_never_enters_a_canonical_segment() {
    let directory = crate::test_support::private_tempdir();
    let root = ControlStorageRoot::retain(directory.path()).unwrap();
    let scope = deployment(
        "../../workspace/with spaces",
        "../binding/../../../outside",
        'b',
    );
    let location = root.prepare_store_location(scope).unwrap();
    let path = location.path();
    let relative = path.strip_prefix(directory.path()).unwrap();
    let components = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    assert_eq!(components[0], "pale");
    assert_eq!(components[1], "deployments");
    assert!(components[2].starts_with("w-"));
    assert_eq!(components[2].len(), 66);
    assert!(components[3].starts_with("b-"));
    assert_eq!(components[3].len(), 66);
    assert_eq!(components[4], "control.sqlite3");
    assert!(!location.path().to_string_lossy().contains("workspace/with"));
    assert!(!location.path().to_string_lossy().contains("../binding"));
}

#[test]
fn lookup_observes_missing_descendants_without_creating_them() {
    let directory = crate::test_support::private_tempdir();
    let root = ControlStorageRoot::retain(directory.path()).unwrap();

    assert!(root
        .lookup_store_location(principal("principal-observe"))
        .unwrap()
        .is_none());
    assert!(!directory.path().join("pale").exists());

    let prepared = root
        .prepare_store_location(principal("principal-observe"))
        .unwrap();
    let observed = root
        .lookup_store_location(principal("principal-observe"))
        .unwrap()
        .unwrap();
    assert_eq!(prepared.path(), observed.path());
}

#[test]
fn every_created_directory_is_private_and_store_files_are_owner_only() {
    let directory = crate::test_support::private_tempdir();
    let root = ControlStorageRoot::retain(directory.path()).unwrap();
    let scope = deployment("workspace-mode", "binding-mode", 'c');
    let location = root.prepare_store_location(scope.clone()).unwrap();
    let identity = crate::store::ControlStoreIdentity::new(
        scope,
        pale_core::records::ControlStoreIdV1::parse("logical-mode").unwrap(),
        pale_core::records::ControlIncarnationIdV1::parse("incarnation-mode").unwrap(),
    );
    crate::store::initialize_unadmitted_store(&location, identity.clone()).unwrap();

    let mut current = directory.path().to_path_buf();
    for segment in location
        .path()
        .strip_prefix(directory.path())
        .unwrap()
        .parent()
        .unwrap()
        .iter()
    {
        current.push(segment);
        assert_eq!(
            std::fs::metadata(&current).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
    assert_eq!(
        std::fs::metadata(location.path())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(
        std::fs::metadata(location.path()).unwrap().uid(),
        rustix::process::geteuid().as_raw()
    );

    std::fs::set_permissions(location.path(), std::fs::Permissions::from_mode(0o640)).unwrap();
    assert_eq!(
        crate::store::classify_store(&location, &identity).unwrap(),
        crate::store::StoreOpenClassification::Rejected(
            crate::store::StoreOpenRejection::UnsafeEntryKind,
        )
    );
}

#[test]
fn root_and_descendant_symlinks_are_rejected_without_following_them() {
    let directory = crate::test_support::private_tempdir();
    let real = directory.path().join("real-root");
    std::fs::create_dir(&real).unwrap();
    std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o700)).unwrap();
    let linked = directory.path().join("linked-root");
    std::os::unix::fs::symlink(&real, &linked).unwrap();
    assert!(matches!(
        ControlStorageRoot::retain(&linked),
        Err(ControlSqliteError::UnsafeRuntimeStateRoot(_))
    ));

    let root = ControlStorageRoot::retain(directory.path()).unwrap();
    let outside = directory.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::set_permissions(&outside, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::os::unix::fs::symlink(&outside, directory.path().join("pale")).unwrap();
    assert!(matches!(
        root.prepare_store_location(principal("principal-link")),
        Err(ControlSqliteError::UnsafeControlStorageDirectory(_))
    ));
}

#[test]
fn group_visible_existing_intermediate_is_rejected() {
    let directory = crate::test_support::private_tempdir();
    let pale = directory.path().join("pale");
    std::fs::create_dir(&pale).unwrap();
    std::fs::set_permissions(&pale, std::fs::Permissions::from_mode(0o750)).unwrap();
    let root = ControlStorageRoot::retain(directory.path()).unwrap();

    assert!(matches!(
        root.prepare_store_location(principal("principal-mode")),
        Err(ControlSqliteError::UnsafeControlStorageDirectory(path)) if path == pale
    ));
}

#[test]
fn selected_root_mode_is_checked_at_retention_and_before_each_resolution() {
    let directory = crate::test_support::private_tempdir();
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o750)).unwrap();
    assert!(matches!(
        ControlStorageRoot::retain(directory.path()),
        Err(ControlSqliteError::UnsafeRuntimeStateRoot(path)) if path == directory.path()
    ));

    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let root = ControlStorageRoot::retain(directory.path()).unwrap();
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o750)).unwrap();
    assert!(matches!(
        root.prepare_store_location(principal("principal-root-mode")),
        Err(ControlSqliteError::UnsafeRuntimeStateRoot(path)) if path == directory.path()
    ));
    assert!(!directory.path().join("pale").exists());

    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
}
