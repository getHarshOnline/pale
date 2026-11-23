//! Workspace-session compilation and repository-ceiling laws.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use pale_core::records::{
    CapabilityIdV1, ExecutionWorkspaceRepositoryAccessV1, LogicalRepositoryIdV1, PackageIdV1,
    PackageResourceIdV1, PackageResourceRefV1, ProfileRepository,
};
use pale_ports::{
    MachineFilesystemError, MachineFilesystemOperation, MachineFilesystemPort, MachinePathKind,
    MachinePathObservation,
};

use super::{
    minimal_readonly_path_cover, readonly_repository_bindings, resolve_readonly_path,
    ReadonlyPathPolicy,
};
use crate::workspace::WorkspaceError;

enum TestObservation {
    Present(MachinePathObservation),
    Fails(io::ErrorKind),
}

struct TestFilesystem(TestObservation);

impl MachineFilesystemPort for TestFilesystem {
    fn inspect_requested(&self, path: &Path) -> Result<MachinePathKind, MachineFilesystemError> {
        match &self.0 {
            TestObservation::Present(observation) => Ok(observation.requested_kind()),
            TestObservation::Fails(kind) => Err(observation_failure(
                path,
                MachineFilesystemOperation::InspectRequested,
                *kind,
            )),
        }
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf, MachineFilesystemError> {
        match &self.0 {
            TestObservation::Present(observation) => Ok(observation.canonical_path().to_path_buf()),
            TestObservation::Fails(kind) => Err(observation_failure(
                path,
                MachineFilesystemOperation::Canonicalize,
                *kind,
            )),
        }
    }

    fn observe(&self, path: &Path) -> Result<MachinePathObservation, MachineFilesystemError> {
        match &self.0 {
            TestObservation::Present(observation) => Ok(observation.clone()),
            TestObservation::Fails(kind) => Err(observation_failure(
                path,
                MachineFilesystemOperation::InspectRequested,
                *kind,
            )),
        }
    }
}

fn observation_failure(
    path: &Path,
    operation: MachineFilesystemOperation,
    kind: io::ErrorKind,
) -> MachineFilesystemError {
    MachineFilesystemError::new(
        operation,
        path.to_path_buf(),
        io::Error::new(kind, "injected machine observation failure"),
    )
}

fn observation(canonical: PathBuf, kind: MachinePathKind) -> MachinePathObservation {
    MachinePathObservation::new(canonical, kind, kind, Some(0o700))
}

fn repository(id: &str, access: ExecutionWorkspaceRepositoryAccessV1) -> ProfileRepository {
    ProfileRepository {
        id: LogicalRepositoryIdV1::parse(id).expect("test repository identity"),
        access,
        role: PackageResourceRefV1 {
            package_id: PackageIdV1::parse("example.engine").expect("test Package identity"),
            resource_id: PackageResourceIdV1::parse("engine").expect("test resource identity"),
        },
        stacks: Vec::new(),
        families: Vec::new(),
        capability_commands: BTreeMap::<CapabilityIdV1, Vec<String>>::new(),
        mutating_not_readonly: Vec::new(),
        command_note: None,
    }
}

#[test]
fn sealed_read_membership_alone_projects_write_protection() {
    let read = repository("read-source", ExecutionWorkspaceRepositoryAccessV1::Read);
    let mutate = repository(
        "mutable-source",
        ExecutionWorkspaceRepositoryAccessV1::Mutate,
    );
    let paths = BTreeMap::from([
        (read.id.clone(), "/work/read-source".to_owned()),
        (mutate.id.clone(), "/work/mutable-source".to_owned()),
    ]);
    let repositories = [read, mutate];

    let bindings = readonly_repository_bindings(&repositories, &paths, "machine")
        .expect("both profile members have machine checkouts");

    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].0.id.as_str(), "read-source");
    assert_eq!(bindings[0].1, "/work/read-source");
}

#[test]
fn read_only_path_cover_is_sorted_unique_and_subsumes_descendants() {
    assert_eq!(
        minimal_readonly_path_cover(vec![
            "/work/repository/nested".to_owned(),
            "/work/other".to_owned(),
            "/work/repository".to_owned(),
            "/work/other".to_owned(),
        ]),
        vec!["/work/other".to_owned(), "/work/repository".to_owned()]
    );
}

#[test]
fn readonly_path_policy_preserves_kind_and_canonical_substitution() {
    let canonical = "/canonical/repository";
    let alias = TestFilesystem(TestObservation::Present(MachinePathObservation::new(
        PathBuf::from(canonical),
        MachinePathKind::Symlink,
        MachinePathKind::Directory,
        Some(0o777),
    )));
    assert_eq!(
        resolve_readonly_path(
            &alias,
            "/declared/repository",
            "repository".into(),
            ReadonlyPathPolicy::Directory,
        )
        .expect("canonical directory target remains the protected identity"),
        canonical
    );

    let file = TestFilesystem(TestObservation::Present(observation(
        PathBuf::from("/authority/guide.md"),
        MachinePathKind::RegularFile,
    )));
    assert!(matches!(
        resolve_readonly_path(
            &file,
            "/repository",
            "repository".into(),
            ReadonlyPathPolicy::Directory,
        ),
        Err(WorkspaceError::SessionPathMissing { .. })
    ));
    assert_eq!(
        resolve_readonly_path(
            &file,
            "/authority/guide.md",
            "authority".into(),
            ReadonlyPathPolicy::DirectoryOrFile,
        )
        .expect("an exact authority file is admissible"),
        "/authority/guide.md"
    );

    let other = TestFilesystem(TestObservation::Present(observation(
        PathBuf::from("/authority/socket"),
        MachinePathKind::Other,
    )));
    assert!(matches!(
        resolve_readonly_path(
            &other,
            "/authority/socket",
            "authority".into(),
            ReadonlyPathPolicy::DirectoryOrFile,
        ),
        Err(WorkspaceError::SessionPathMissing { .. })
    ));
}

#[test]
fn readonly_path_policy_distinguishes_absence_from_observation_failure() {
    let absent = TestFilesystem(TestObservation::Fails(io::ErrorKind::NotFound));
    assert!(matches!(
        resolve_readonly_path(
            &absent,
            "/repository",
            "repository".into(),
            ReadonlyPathPolicy::Directory,
        ),
        Err(WorkspaceError::SessionPathMissing { .. })
    ));

    let denied = TestFilesystem(TestObservation::Fails(io::ErrorKind::PermissionDenied));
    let error = resolve_readonly_path(
        &denied,
        "/repository",
        "repository".into(),
        ReadonlyPathPolicy::Directory,
    )
    .expect_err("permission denial is not absence");
    assert!(matches!(
        error,
        WorkspaceError::SessionPathObservation { source, .. }
            if source.kind() == io::ErrorKind::PermissionDenied
    ));
}

#[cfg(unix)]
#[test]
fn readonly_path_policy_rejects_non_utf8_canonical_identity() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let canonical = PathBuf::from(OsString::from_vec(b"/repository/\xff".to_vec()));
    let filesystem = TestFilesystem(TestObservation::Present(observation(
        canonical,
        MachinePathKind::Directory,
    )));
    assert!(matches!(
        resolve_readonly_path(
            &filesystem,
            "/repository",
            "repository".into(),
            ReadonlyPathPolicy::Directory,
        ),
        Err(WorkspaceError::SessionPathNotUtf8 { .. })
    ));
}
