//! Shared test-only identities and temporary-store constructors.

use crate::location::{
    ControlStorageRoot, ControlStoreLocation, ControlStoreScope, DeploymentStoreScope,
    PrincipalStoreScope,
};
use crate::store::{initialize_unadmitted_store, ControlStoreIdentity, InitializedStore};
use crate::ControlStoreKind;
use pale_core::records::{
    ArtifactDigest, ControlIncarnationIdV1, ControlStoreIdV1, ResidentPrincipalIdV1,
    ResidentRecordRefV1, ResidentWorkspaceIdV1,
};
use std::os::unix::fs::PermissionsExt;

mod anti_rollback;

pub(crate) use anti_rollback::RetainedAntiRollbackAnchors;

pub(crate) fn private_tempdir() -> tempfile::TempDir {
    let system_temporary = std::fs::canonicalize(std::env::temp_dir()).unwrap();
    let directory = tempfile::tempdir_in(system_temporary).unwrap();
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    directory
}

pub(crate) fn identity(kind: ControlStoreKind, suffix: &str) -> ControlStoreIdentity {
    ControlStoreIdentity::new(
        scope(kind, suffix),
        ControlStoreIdV1::parse(format!("store-{suffix}")).unwrap(),
        ControlIncarnationIdV1::parse(format!("incarnation-{suffix}")).unwrap(),
    )
}

pub(crate) fn scope(kind: ControlStoreKind, suffix: &str) -> ControlStoreScope {
    match kind {
        ControlStoreKind::PrincipalTrust => ControlStoreScope::Principal(PrincipalStoreScope::new(
            ResidentPrincipalIdV1::parse(format!("principal-{suffix}")).unwrap(),
        )),
        ControlStoreKind::DeploymentControl => {
            ControlStoreScope::Deployment(DeploymentStoreScope::new(
                ResidentWorkspaceIdV1::parse(format!("workspace-{suffix}")).unwrap(),
                ResidentRecordRefV1::parse(format!("deployment-binding-{suffix}")).unwrap(),
                ArtifactDigest::hash_bytes(format!("deployment-binding-{suffix}").as_bytes()),
            ))
        }
    }
}

pub(crate) fn location(
    directory: &tempfile::TempDir,
    kind: ControlStoreKind,
    suffix: &str,
) -> ControlStoreLocation {
    ControlStorageRoot::retain(directory.path())
        .unwrap()
        .prepare_store_location(scope(kind, suffix))
        .unwrap()
}

pub(crate) fn initialize(
    directory: &tempfile::TempDir,
    kind: ControlStoreKind,
    suffix: &str,
) -> (ControlStoreLocation, ControlStoreIdentity, InitializedStore) {
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let location = location(directory, kind, suffix);
    let identity = identity(kind, suffix);
    let initialized = initialize_unadmitted_store(&location, identity.clone()).unwrap();
    (location, identity, initialized)
}
