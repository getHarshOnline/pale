//! Machine-observation authority laws for resolved workspace instances.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use pale_core::records::{
    InstanceDigest, ProfileDigest, ReadingOrderEntry, ResolvedProjectInstanceV1, SchemaDigest,
    SchemaId, SessionModel, SessionStateBinding, SessionTtlSeconds,
};
use pale_ports::{
    MachineFilesystemError, MachineFilesystemOperation, MachineFilesystemPort, MachinePathKind,
    MachinePathObservation,
};

use super::{verify_reading_order, verify_session_state_root};
use crate::workspace::WorkspaceError;

struct ObservedPaths(BTreeMap<PathBuf, MachinePathObservation>);

impl MachineFilesystemPort for ObservedPaths {
    fn inspect_requested(&self, path: &Path) -> Result<MachinePathKind, MachineFilesystemError> {
        self.0
            .get(path)
            .map(MachinePathObservation::requested_kind)
            .ok_or_else(|| missing_observation(path, MachineFilesystemOperation::InspectRequested))
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf, MachineFilesystemError> {
        self.0
            .get(path)
            .map(|observation| observation.canonical_path().to_path_buf())
            .ok_or_else(|| missing_observation(path, MachineFilesystemOperation::Canonicalize))
    }

    fn observe(&self, path: &Path) -> Result<MachinePathObservation, MachineFilesystemError> {
        self.0
            .get(path)
            .cloned()
            .ok_or_else(|| missing_observation(path, MachineFilesystemOperation::InspectRequested))
    }
}

fn missing_observation(
    path: &Path,
    operation: MachineFilesystemOperation,
) -> MachineFilesystemError {
    MachineFilesystemError::new(
        operation,
        path.to_path_buf(),
        io::Error::new(io::ErrorKind::NotFound, "absent test observation"),
    )
}

struct FailingFilesystem(io::ErrorKind);

impl MachineFilesystemPort for FailingFilesystem {
    fn inspect_requested(&self, path: &Path) -> Result<MachinePathKind, MachineFilesystemError> {
        Err(self.failure(MachineFilesystemOperation::InspectRequested, path))
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf, MachineFilesystemError> {
        Err(self.failure(MachineFilesystemOperation::Canonicalize, path))
    }

    fn observe(&self, path: &Path) -> Result<MachinePathObservation, MachineFilesystemError> {
        Err(self.failure(MachineFilesystemOperation::InspectRequested, path))
    }
}

impl FailingFilesystem {
    fn failure(
        &self,
        operation: MachineFilesystemOperation,
        path: &Path,
    ) -> MachineFilesystemError {
        MachineFilesystemError::new(
            operation,
            path.to_path_buf(),
            io::Error::new(self.0, "injected machine observation failure"),
        )
    }
}

fn observation(
    canonical: &str,
    requested_kind: MachinePathKind,
    canonical_kind: MachinePathKind,
    mode: u32,
) -> MachinePathObservation {
    MachinePathObservation::new(
        PathBuf::from(canonical),
        requested_kind,
        canonical_kind,
        Some(mode),
    )
}

fn instance(
    session_root: &str,
    authority_roots: BTreeMap<String, String>,
) -> ResolvedProjectInstanceV1 {
    ResolvedProjectInstanceV1 {
        schema: SchemaId::parse("pale/resolved-project-instance.v1").expect("schema identity"),
        schema_sha256: SchemaDigest::hash_bytes(b"resolved instance schema"),
        profile_digest: ProfileDigest::parse("b".repeat(64)).expect("profile digest"),
        machine_id: "machine".into(),
        repositories: BTreeMap::new(),
        git_metadata_roots: BTreeMap::new(),
        provider_repositories: Vec::new(),
        authority_roots,
        user_mode_defaults: BTreeMap::new(),
        surface_executable: None,
        native_harnesses: Vec::new(),
        session_state: Some(SessionStateBinding {
            root: session_root.into(),
            ttl_seconds: SessionTtlSeconds::new(60).expect("session TTL"),
        }),
        durable_evidence: None,
        materialization_lineage: None,
        instance_digest: InstanceDigest::parse("c".repeat(64)).expect("instance digest"),
    }
}

#[test]
fn state_root_policy_rejects_alias_type_and_permission_substitutions() {
    let root = "/state";
    let admitted = instance(root, BTreeMap::new());

    let valid = ObservedPaths(BTreeMap::from([(
        PathBuf::from(root),
        observation(
            root,
            MachinePathKind::Directory,
            MachinePathKind::Directory,
            0o700,
        ),
    )]));
    assert_eq!(
        verify_session_state_root(&valid, &admitted).expect("canonical private directory"),
        PathBuf::from(root)
    );

    enum ExpectedIssue {
        NotConfined,
        NotDirectory,
    }

    let cases = [
        (
            observation(
                root,
                MachinePathKind::Symlink,
                MachinePathKind::Directory,
                0o700,
            ),
            ExpectedIssue::NotConfined,
        ),
        (
            observation(
                root,
                MachinePathKind::RegularFile,
                MachinePathKind::RegularFile,
                0o600,
            ),
            ExpectedIssue::NotDirectory,
        ),
        (
            observation(
                "/other",
                MachinePathKind::Directory,
                MachinePathKind::Directory,
                0o700,
            ),
            ExpectedIssue::NotConfined,
        ),
    ];
    for (observed, expected) in cases {
        let filesystem = ObservedPaths(BTreeMap::from([(PathBuf::from(root), observed)]));
        let error = verify_session_state_root(&filesystem, &admitted)
            .expect_err("substituted state-root evidence must fail");
        assert!(match expected {
            ExpectedIssue::NotConfined => {
                matches!(error, WorkspaceError::SessionStateNotConfined(_))
            }
            ExpectedIssue::NotDirectory => {
                matches!(error, WorkspaceError::SessionStateNotDirectory(_))
            }
        });
    }

    #[cfg(unix)]
    {
        let filesystem = ObservedPaths(BTreeMap::from([(
            PathBuf::from(root),
            observation(
                root,
                MachinePathKind::Directory,
                MachinePathKind::Directory,
                0o722,
            ),
        )]));
        assert!(matches!(
            verify_session_state_root(&filesystem, &admitted),
            Err(WorkspaceError::SessionStateUnsafePermissions(_))
        ));
    }
}

#[test]
fn state_root_policy_preserves_non_absence_observation_failures() {
    let admitted = instance("/state", BTreeMap::new());
    let error = verify_session_state_root(
        &FailingFilesystem(io::ErrorKind::PermissionDenied),
        &admitted,
    )
    .expect_err("permission denial cannot masquerade as an absent state root");
    assert!(matches!(
        error,
        WorkspaceError::SessionPathObservation { source, .. }
            if source.kind() == io::ErrorKind::PermissionDenied
    ));
}

#[test]
fn reading_order_uses_canonical_containment_and_exact_file_authority() {
    let authority = "/authority";
    let target = "/authority/guide.md";
    let admitted = instance(
        "/state",
        BTreeMap::from([("context".into(), authority.into())]),
    );
    let session = SessionModel {
        activation_roots: vec![pale_core::records::LogicalRepositoryIdV1::parse("engine")
            .expect("repository identity")],
        additional_roots: Vec::new(),
        authority_access: Vec::new(),
        activation_path_ids: Vec::new(),
        lifecycle_events: Vec::new(),
        runtime: None,
        reading_order: vec![ReadingOrderEntry {
            root_id: "context".into(),
            rel_path: "guide.md".into(),
        }],
    };
    let valid = ObservedPaths(BTreeMap::from([
        (
            PathBuf::from(authority),
            observation(
                authority,
                MachinePathKind::Directory,
                MachinePathKind::Directory,
                0o700,
            ),
        ),
        (
            PathBuf::from(target),
            observation(
                target,
                MachinePathKind::RegularFile,
                MachinePathKind::RegularFile,
                0o600,
            ),
        ),
    ]));
    verify_reading_order(&valid, &admitted, &session).expect("confined regular document");

    let exact_file_instance = instance(
        "/state",
        BTreeMap::from([("context".into(), target.into())]),
    );
    let exact_file = ObservedPaths(BTreeMap::from([(
        PathBuf::from(target),
        observation(
            target,
            MachinePathKind::RegularFile,
            MachinePathKind::RegularFile,
            0o600,
        ),
    )]));
    verify_reading_order(&exact_file, &exact_file_instance, &session)
        .expect("exact-file authority admits only its selected document");

    let escaped = ObservedPaths(BTreeMap::from([
        (
            PathBuf::from(authority),
            observation(
                authority,
                MachinePathKind::Directory,
                MachinePathKind::Directory,
                0o700,
            ),
        ),
        (
            PathBuf::from(target),
            observation(
                "/outside/guide.md",
                MachinePathKind::Symlink,
                MachinePathKind::RegularFile,
                0o777,
            ),
        ),
    ]));
    assert!(matches!(
        verify_reading_order(&escaped, &admitted, &session),
        Err(WorkspaceError::ReadingOrderEscapesRoot { .. })
    ));

    let invalid_root = ObservedPaths(BTreeMap::from([(
        PathBuf::from(authority),
        observation(
            authority,
            MachinePathKind::Other,
            MachinePathKind::Other,
            0o700,
        ),
    )]));
    assert!(matches!(
        verify_reading_order(&invalid_root, &admitted, &session),
        Err(WorkspaceError::SessionPathMissing { .. })
    ));
}
