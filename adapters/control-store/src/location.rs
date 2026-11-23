//! Canonical, scope-bound control-store locations beneath one retained runtime root.
//!
//! # Locator algebra
//!
//! In the rendered locator law, `H` is SHA-256, `L` is length framing, the
//! vertical operator is byte concatenation, and each `D` is the corresponding
//! initial-v1 domain constant below. The workspace coordinate occurs only in
//! its `w-` parent. These digests are non-semantic locators; the store
//! separately persists every raw typed coordinate used to derive them.

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/control-store-locators.display.mathml"
))]
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::ErrorKind;
use std::os::unix::fs::FileExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use pale_core::records::{
    ArtifactDigest, ResidentPrincipalIdV1, ResidentRecordRefV1, ResidentWorkspaceIdV1,
};
use pale_core::sha256_hex;
use rusqlite::{Connection, OpenFlags};
use rustix::fs::{AtFlags, FileType, Mode, OFlags};
use rustix::io::Errno;
use url::Url;

use crate::{ControlSqliteError, ControlStoreKind};

/// Domain of a principal directory locator.
const PRINCIPAL_SEGMENT_DOMAIN_V1: &[u8] = b"PALE_CONTROL_STORE_PRINCIPAL_SEGMENT_V1\0";
/// Domain of a workspace directory locator.
const WORKSPACE_SEGMENT_DOMAIN_V1: &[u8] = b"PALE_CONTROL_STORE_WORKSPACE_SEGMENT_V1\0";
/// Domain of an immutable deployment-binding directory locator.
const DEPLOYMENT_BINDING_SEGMENT_DOMAIN_V1: &[u8] =
    b"PALE_CONTROL_STORE_DEPLOYMENT_BINDING_SEGMENT_V1\0";
/// Fixed coordination leaf beneath one canonical principal directory.
const PRINCIPAL_WRITER_LOCK_LEAF: &str = "resident-writer.lock";

/// Raw principal coordinate bound to one principal trust store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrincipalStoreScope {
    principal_id: ResidentPrincipalIdV1,
}

impl PrincipalStoreScope {
    /// Bind a trust-store scope to one exact principal identity.
    #[must_use]
    pub(crate) const fn new(principal_id: ResidentPrincipalIdV1) -> Self {
        Self { principal_id }
    }

    /// Raw principal identity persisted in the trust-store binding row.
    #[must_use]
    pub(crate) const fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.principal_id
    }
}

/// Raw immutable coordinates bound to one deployment control store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeploymentStoreScope {
    workspace_id: ResidentWorkspaceIdV1,
    deployment_binding_ref: ResidentRecordRefV1,
    deployment_binding_sha256: ArtifactDigest,
}

impl DeploymentStoreScope {
    /// Bind a deployment-store scope to one workspace and immutable binding.
    #[must_use]
    pub(crate) const fn new(
        workspace_id: ResidentWorkspaceIdV1,
        deployment_binding_ref: ResidentRecordRefV1,
        deployment_binding_sha256: ArtifactDigest,
    ) -> Self {
        Self {
            workspace_id,
            deployment_binding_ref,
            deployment_binding_sha256,
        }
    }

    /// Raw workspace identity persisted in the deployment binding row.
    #[must_use]
    pub(crate) const fn workspace_id(&self) -> &ResidentWorkspaceIdV1 {
        &self.workspace_id
    }

    /// Raw immutable deployment-binding reference persisted in the binding row.
    #[must_use]
    pub(crate) const fn deployment_binding_ref(&self) -> &ResidentRecordRefV1 {
        &self.deployment_binding_ref
    }

    /// Exact deployment-binding content hash persisted in the binding row.
    #[must_use]
    pub(crate) const fn deployment_binding_sha256(&self) -> &ArtifactDigest {
        &self.deployment_binding_sha256
    }
}

/// Closed scope grammar for the two physical control-store kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ControlStoreScope {
    /// One principal's local trust custody.
    Principal(PrincipalStoreScope),
    /// One immutable deployment binding's operational custody.
    Deployment(DeploymentStoreScope),
}

impl ControlStoreScope {
    /// Physical store kind implied by this typed scope.
    #[must_use]
    pub(crate) const fn kind(&self) -> ControlStoreKind {
        match self {
            Self::Principal(_) => ControlStoreKind::PrincipalTrust,
            Self::Deployment(_) => ControlStoreKind::DeploymentControl,
        }
    }
}

/// Shared retained descriptor and diagnostic path for one private runtime root.
#[derive(Debug)]
struct RuntimeRootHandle {
    path: PathBuf,
    directory: File,
    identity: FileIdentity,
}

/// Opaque capability from which every canonical control-store path is derived.
///
/// Retention opens every absolute path component with `NOFOLLOW`; descendants
/// are subsequently opened or created relative to the retained descriptor.
/// The retained root and every canonical descendant must be owned by the
/// process's effective UID and exclude group/other POSIX mode bits. This proves
/// only those UID, mode, no-follow, and retained-path facts: it does not inspect
/// ACLs, exclude another process with the same UID, select a Pale Workspace value,
/// or turn a path into deployment policy.
#[derive(Debug, Clone)]
pub(crate) struct ControlStorageRoot {
    handle: Arc<RuntimeRootHandle>,
}

impl ControlStorageRoot {
    /// Retain one already-selected absolute runtime-state root under the effective UID.
    ///
    /// # Errors
    ///
    /// Returns [`ControlSqliteError`] when the path is relative, contains a
    /// parent component, traverses a symlink or non-directory, cannot be opened,
    /// differs from the effective UID, or has group/other POSIX mode bits.
    pub(crate) fn retain(path: &Path) -> Result<Self, ControlSqliteError> {
        let (path, directory) = open_absolute_directory_without_links(path)?;
        if !directory_is_private(&directory, &path)? {
            return Err(ControlSqliteError::UnsafeRuntimeStateRoot(path));
        }
        let metadata = directory.metadata().map_err(|source| {
            ControlSqliteError::io("inspect retained runtime-state root", &path, source)
        })?;
        Ok(Self {
            handle: Arc::new(RuntimeRootHandle {
                path,
                identity: FileIdentity::from_metadata(&metadata),
                directory,
            }),
        })
    }

    /// Derive and retain the sole canonical store location for `scope`.
    ///
    /// Every fixed and digest-keyed directory is created as `0700` when absent,
    /// then reopened descriptor-relative with `NOFOLLOW`. Existing descendants
    /// must already satisfy the same effective-UID and POSIX-mode invariant.
    ///
    /// # Errors
    ///
    /// Returns [`ControlSqliteError`] when a descendant is unsafe, cannot be
    /// created or synchronized, or the retained root pathname was substituted.
    #[cfg(any(test, feature = "bench"))]
    pub(crate) fn prepare_store_location(
        &self,
        scope: ControlStoreScope,
    ) -> Result<ControlStoreLocation, ControlSqliteError> {
        self.resolve_store_location(scope, true)?
            .ok_or(ControlSqliteError::SourceInvariant(
                "prepared control-store location",
            ))
    }

    /// Resolve an existing canonical store parent without creating directories.
    ///
    /// `None` means that at least one canonical descendant does not exist. This
    /// keeps read-only inspection observational. Directory creation remains
    /// confined to the explicit create disposition of
    /// [`Self::resolve_store_location`].
    ///
    /// # Errors
    ///
    /// Returns [`ControlSqliteError`] when an existing descendant is unsafe or
    /// the retained root pathname was substituted.
    pub(crate) fn lookup_store_location(
        &self,
        scope: ControlStoreScope,
    ) -> Result<Option<ControlStoreLocation>, ControlSqliteError> {
        self.resolve_store_location(scope, false)
    }

    /// Open the persistent coordination file for one principal writer.
    ///
    /// This creates only the canonical private directory chain and an empty
    /// owner-only regular file. It neither opens a database nor acquires the
    /// operating-system lock on the returned descriptor.
    pub(crate) fn open_principal_writer_lock_file(
        &self,
        principal_id: &ResidentPrincipalIdV1,
    ) -> Result<(File, PathBuf), ControlSqliteError> {
        self.verify_path_identity()?;
        let segments = vec![
            OsString::from("pale"),
            OsString::from("principals"),
            OsString::from(principal_segment(principal_id)),
        ];
        let parent =
            walk_private_descendants(&self.handle.directory, &self.handle.path, &segments, true)?
                .ok_or(ControlSqliteError::SourceInvariant(
                "prepared principal writer-lock directory",
            ))?;
        let path = segments
            .iter()
            .fold(self.handle.path.clone(), |path, segment| path.join(segment))
            .join(PRINCIPAL_WRITER_LOCK_LEAF);
        let (file, created) = match rustix::fs::openat(
            &parent,
            PRINCIPAL_WRITER_LOCK_LEAF,
            OFlags::RDWR
                | OFlags::CREATE
                | OFlags::EXCL
                | OFlags::NOFOLLOW
                | OFlags::CLOEXEC
                | OFlags::NONBLOCK,
            Mode::from_raw_mode(0o600),
        ) {
            Ok(descriptor) => (File::from(descriptor), true),
            Err(Errno::EXIST) => (
                File::from(
                    rustix::fs::openat(
                        &parent,
                        PRINCIPAL_WRITER_LOCK_LEAF,
                        OFlags::RDWR | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
                        Mode::empty(),
                    )
                    .map_err(|source| {
                        if matches!(source, Errno::LOOP | Errno::NOTDIR | Errno::ISDIR) {
                            ControlSqliteError::UnsafePrincipalWriterLock(path.clone())
                        } else {
                            errno_error("open principal writer lock", &path, source)
                        }
                    })?,
                ),
                false,
            ),
            Err(source) => {
                return Err(errno_error("create principal writer lock", &path, source));
            }
        };
        if created {
            rustix::fs::fchmod(&file, Mode::from_raw_mode(0o600)).map_err(|source| {
                errno_error(
                    "set private principal writer-lock permissions",
                    &path,
                    source,
                )
            })?;
            rustix::fs::fsync(&parent).map_err(|source| {
                errno_error("synchronize principal writer-lock parent", &path, source)
            })?;
        }
        let metadata = file.metadata().map_err(|source| {
            ControlSqliteError::io("inspect principal writer lock", &path, source)
        })?;
        if !metadata.is_file()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.permissions().mode() & 0o7777 != 0o600
            || metadata.nlink() != 1
            || metadata.len() != 0
        {
            return Err(ControlSqliteError::UnsafePrincipalWriterLock(path));
        }
        Ok((file, path))
    }

    /// Resolve one scope with an explicit create-or-observe disposition.
    fn resolve_store_location(
        &self,
        scope: ControlStoreScope,
        create: bool,
    ) -> Result<Option<ControlStoreLocation>, ControlSqliteError> {
        self.verify_path_identity()?;
        let (segments, leaf) = location_components(&scope)?;
        let Some(parent) =
            walk_private_descendants(&self.handle.directory, &self.handle.path, &segments, create)?
        else {
            return Ok(None);
        };
        Ok(Some(ControlStoreLocation {
            root: self.clone(),
            parent: Arc::new(parent),
            parent_path: segments
                .iter()
                .fold(self.handle.path.clone(), |path, segment| path.join(segment)),
            segments,
            leaf,
            scope,
        }))
    }

    /// Reopen the root component-by-component and compare its machine-local identity.
    fn verify_path_identity(&self) -> Result<(), ControlSqliteError> {
        let (_, current) = open_absolute_directory_without_links(&self.handle.path)
            .map_err(|_| ControlSqliteError::StorageLocationChanged(self.handle.path.clone()))?;
        if !directory_is_private(&current, &self.handle.path)? {
            return Err(ControlSqliteError::UnsafeRuntimeStateRoot(
                self.handle.path.clone(),
            ));
        }
        let metadata = current.metadata().map_err(|source| {
            ControlSqliteError::io(
                "inspect reopened runtime-state root",
                &self.handle.path,
                source,
            )
        })?;
        if FileIdentity::from_metadata(&metadata) == self.handle.identity {
            Ok(())
        } else {
            Err(ControlSqliteError::StorageLocationChanged(
                self.handle.path.clone(),
            ))
        }
    }
}

/// Canonical store leaf under a retained, scope-derived private directory.
#[derive(Debug, Clone)]
pub(crate) struct ControlStoreLocation {
    root: ControlStorageRoot,
    parent: Arc<File>,
    parent_path: PathBuf,
    segments: Vec<OsString>,
    leaf: &'static str,
    scope: ControlStoreScope,
}

impl ControlStoreLocation {
    /// Diagnostic pathname consumed only after retained-capability verification.
    #[must_use]
    pub(crate) fn path(&self) -> PathBuf {
        self.parent_path.join(self.leaf)
    }

    /// Typed semantic scope whose locator projection produced this path.
    #[must_use]
    pub(crate) const fn scope(&self) -> &ControlStoreScope {
        &self.scope
    }

    /// Borrow the retained immediate parent descriptor.
    pub(crate) fn parent(&self) -> &File {
        &self.parent
    }

    /// Fixed database leaf beneath the retained parent.
    #[must_use]
    pub(crate) fn leaf(&self) -> &OsStr {
        OsStr::new(self.leaf)
    }

    /// Verify that the diagnostic pathname still reaches the retained parent.
    pub(crate) fn verify_parent_path_identity(&self) -> Result<(), ControlSqliteError> {
        self.root.verify_path_identity()?;
        let current = walk_private_descendants(
            &self.root.handle.directory,
            &self.root.handle.path,
            &self.segments,
            false,
        )?
        .ok_or_else(|| ControlSqliteError::StorageLocationChanged(self.parent_path.clone()))?;
        let current_metadata = current.metadata().map_err(|source| {
            ControlSqliteError::io(
                "inspect reopened control-store parent",
                &self.parent_path,
                source,
            )
        })?;
        let retained_metadata = self.parent.metadata().map_err(|source| {
            ControlSqliteError::io(
                "inspect retained control-store parent",
                &self.parent_path,
                source,
            )
        })?;
        if FileIdentity::from_metadata(&current_metadata)
            == FileIdentity::from_metadata(&retained_metadata)
        {
            Ok(())
        } else {
            Err(ControlSqliteError::StorageLocationChanged(
                self.parent_path.clone(),
            ))
        }
    }

    /// Inspect the canonical leaf without following it.
    pub(crate) fn entry(&self) -> Result<StoreFileObservation, ControlSqliteError> {
        self.verify_parent_path_identity()?;
        match rustix::fs::statat(self.parent(), self.leaf(), AtFlags::SYMLINK_NOFOLLOW) {
            Ok(stat) => {
                if !private_store_file(&stat) {
                    Ok(StoreFileObservation::Unsafe)
                } else {
                    Ok(StoreFileObservation::Regular {
                        identity: FileIdentity::from_stat(&stat)?,
                        empty: stat.st_size == 0,
                    })
                }
            }
            Err(Errno::NOENT) => Ok(StoreFileObservation::Missing),
            Err(source) => Err(errno_error(
                "inspect canonical control-store leaf",
                self.path(),
                source,
            )),
        }
    }

    /// Open one previously observed regular store with bounded path revalidation.
    ///
    /// `rusqlite` consumes a pathname and does not expose the resulting operating-
    /// system file descriptor for identity comparison. The adapter therefore
    /// verifies the retained parent and leaf immediately before and after opening,
    /// but cannot exclude a same-UID actor performing an A/B/restore substitution
    /// entirely between those checks. Success is structural evidence, not resident
    /// admission or a proof that the returned connection is descriptor-bound.
    ///
    /// # Errors
    ///
    /// Returns [`ControlSqliteError`] when `SQLite` cannot open the file or the
    /// retained root, parent, or leaf identity differs across the open.
    #[cfg(any(test, feature = "bench"))]
    pub(crate) fn open_existing_sqlite(
        &self,
        flags: OpenFlags,
        expected: FileIdentity,
        operation: &'static str,
    ) -> Result<Connection, ControlSqliteError> {
        self.verify_parent_path_identity()?;
        let connection =
            Connection::open_with_flags(self.path(), flags | OpenFlags::SQLITE_OPEN_NOFOLLOW)
                .map_err(|source| ControlSqliteError::sqlite(operation, source))?;
        match self.entry()? {
            StoreFileObservation::Regular { identity, .. } if identity == expected => {
                Ok(connection)
            }
            _ => {
                drop(connection);
                Err(ControlSqliteError::StorageLocationChanged(self.path()))
            }
        }
    }

    /// Open one observed store through `SQLite`'s physically immutable reader.
    ///
    /// The `immutable=1` URI mode suppresses file locking and WAL-index
    /// creation, so classification cannot create or update the main database,
    /// WAL, or shared-memory sidecar. The retained parent and main-file identity
    /// are still checked immediately before and after opening. Callers must not
    /// use this connection as a concurrency snapshot: `SQLite` deliberately omits
    /// change detection in immutable mode, so this boundary owns physical
    /// inspection only and never resident readiness.
    ///
    /// # Errors
    ///
    /// Returns [`ControlSqliteError`] when an absolute file URI cannot be
    /// constructed, `SQLite` cannot open the immutable reader, or the retained
    /// pathname identity changes across the open.
    pub(crate) fn open_existing_sqlite_immutable(
        &self,
        expected: FileIdentity,
        operation: &'static str,
    ) -> Result<Connection, ControlSqliteError> {
        self.verify_parent_path_identity()?;
        let mut uri = Url::from_file_path(self.path()).map_err(|()| {
            ControlSqliteError::SourceInvariant("absolute immutable SQLite file URI")
        })?;
        uri.query_pairs_mut().append_pair("immutable", "1");
        let connection = Connection::open_with_flags(
            uri.as_str(),
            OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|source| ControlSqliteError::sqlite(operation, source))?;
        match self.entry()? {
            StoreFileObservation::Regular { identity, .. } if identity == expected => {
                Ok(connection)
            }
            _ => {
                drop(connection);
                Err(ControlSqliteError::StorageLocationChanged(self.path()))
            }
        }
    }

    /// Read the persistent `SQLite` journal-mode header without opening `SQLite`.
    ///
    /// Bytes 18 and 19 are the file-format read/write versions. WAL mode stores
    /// `2` in both positions; this descriptor-relative read is the immutable
    /// inspection substitute for `PRAGMA journal_mode`, which intentionally
    /// reports `delete` on an `immutable=1` connection.
    pub(crate) fn persistent_journal_mode(
        &self,
        expected: FileIdentity,
    ) -> Result<PersistentJournalMode, ControlSqliteError> {
        self.verify_parent_path_identity()?;
        let file = File::from(
            rustix::fs::openat(
                self.parent(),
                self.leaf(),
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|source| errno_error("open control-store header", self.path(), source))?,
        );
        let stat = rustix::fs::fstat(&file)
            .map_err(|source| errno_error("inspect control-store header", self.path(), source))?;
        if !private_store_file(&stat) || FileIdentity::from_stat(&stat)? != expected {
            return Err(ControlSqliteError::StorageLocationChanged(self.path()));
        }
        let mut header = [0_u8; 20];
        match file.read_exact_at(&mut header, 0) {
            Ok(()) if header[18] == 2 && header[19] == 2 => Ok(PersistentJournalMode::Wal),
            Ok(()) => Ok(PersistentJournalMode::Other),
            Err(source) if source.kind() == ErrorKind::UnexpectedEof => {
                Ok(PersistentJournalMode::Incomplete)
            }
            Err(source) => Err(ControlSqliteError::io(
                "read control-store header",
                self.path(),
                source,
            )),
        }
    }
}

/// Persistent `SQLite` journal-mode evidence from the database file header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PersistentJournalMode {
    /// Both file-format version bytes select WAL.
    Wal,
    /// A complete `SQLite` header selects another journal mode.
    Other,
    /// The file is too short to contain the complete header.
    Incomplete,
}

/// Descriptor-relative observation of a canonical store leaf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StoreFileObservation {
    /// No entry exists at the canonical leaf.
    Missing,
    /// The leaf lacks a regular-file, effective-UID, or exact `0600` mode proof.
    Unsafe,
    /// Regular file identity and whether it contains zero bytes.
    Regular {
        /// Device/inode identity captured before pathname-consuming `SQLite` opens.
        identity: FileIdentity,
        /// Whether the main file is empty.
        empty: bool,
    },
}

/// Unix device and inode pair used to reject pathname substitution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileIdentity {
    device: u64,
    inode: u64,
}

impl FileIdentity {
    /// Capture identity from an already-open standard-library descriptor.
    pub(crate) fn from_metadata(metadata: &std::fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
        }
    }

    /// Capture identity from one descriptor-relative observation.
    fn from_stat(stat: &rustix::fs::Stat) -> Result<Self, ControlSqliteError> {
        Ok(Self {
            device: u64::try_from(stat.st_dev)
                .map_err(|_| ControlSqliteError::SourceInvariant("filesystem device identity"))?,
            inode: stat.st_ino,
        })
    }

    /// Compare a later descriptor-relative observation with the captured identity.
    #[cfg(any(test, feature = "bench"))]
    pub(crate) fn matches_stat(self, stat: &rustix::fs::Stat) -> Result<bool, ControlSqliteError> {
        Ok(self == Self::from_stat(stat)?)
    }
}

/// Select exact safe directory segments and fixed database leaf for one scope.
fn location_components(
    scope: &ControlStoreScope,
) -> Result<(Vec<OsString>, &'static str), ControlSqliteError> {
    Ok(match scope {
        ControlStoreScope::Principal(scope) => (
            vec![
                OsString::from("pale"),
                OsString::from("principals"),
                OsString::from(principal_segment(scope.principal_id())),
            ],
            "trust.sqlite3",
        ),
        ControlStoreScope::Deployment(scope) => (
            vec![
                OsString::from("pale"),
                OsString::from("deployments"),
                OsString::from(workspace_segment(scope.workspace_id())),
                OsString::from(deployment_binding_segment(scope)?),
            ],
            "control.sqlite3",
        ),
    })
}

/// Hash one raw principal identity into its non-semantic path projection.
fn principal_segment(principal_id: &ResidentPrincipalIdV1) -> String {
    let mut preimage =
        Vec::with_capacity(PRINCIPAL_SEGMENT_DOMAIN_V1.len() + principal_id.as_str().len());
    preimage.extend_from_slice(PRINCIPAL_SEGMENT_DOMAIN_V1);
    preimage.extend_from_slice(principal_id.as_str().as_bytes());
    format!("p-{}", sha256_hex(&preimage))
}

/// Hash one raw workspace identity into its non-semantic path projection.
fn workspace_segment(workspace_id: &ResidentWorkspaceIdV1) -> String {
    let mut preimage =
        Vec::with_capacity(WORKSPACE_SEGMENT_DOMAIN_V1.len() + workspace_id.as_str().len());
    preimage.extend_from_slice(WORKSPACE_SEGMENT_DOMAIN_V1);
    preimage.extend_from_slice(workspace_id.as_str().as_bytes());
    format!("w-{}", sha256_hex(&preimage))
}

/// Hash the exact length-framed binding ref/hash pair into its locator.
fn deployment_binding_segment(scope: &DeploymentStoreScope) -> Result<String, ControlSqliteError> {
    let reference = scope.deployment_binding_ref().as_str().as_bytes();
    let digest = scope.deployment_binding_sha256().as_str().as_bytes();
    let mut preimage = Vec::with_capacity(
        DEPLOYMENT_BINDING_SEGMENT_DOMAIN_V1.len()
            + std::mem::size_of::<u64>() * 2
            + reference.len()
            + digest.len(),
    );
    preimage.extend_from_slice(DEPLOYMENT_BINDING_SEGMENT_DOMAIN_V1);
    append_length_framed(&mut preimage, reference)?;
    append_length_framed(&mut preimage, digest)?;
    Ok(format!("b-{}", sha256_hex(&preimage)))
}

/// Append one explicit big-endian length and its exact bytes.
fn append_length_framed(preimage: &mut Vec<u8>, value: &[u8]) -> Result<(), ControlSqliteError> {
    let length = u64::try_from(value.len())
        .map_err(|_| ControlSqliteError::SourceInvariant("locator preimage length"))?;
    preimage.extend_from_slice(&length.to_be_bytes());
    preimage.extend_from_slice(value);
    Ok(())
}

/// Open every absolute root component without following links.
fn open_absolute_directory_without_links(
    path: &Path,
) -> Result<(PathBuf, File), ControlSqliteError> {
    if !path.is_absolute() {
        return Err(ControlSqliteError::UnsafeRuntimeStateRoot(
            path.to_path_buf(),
        ));
    }
    let mut directory = File::from(
        rustix::fs::openat(
            rustix::fs::CWD,
            Path::new("/"),
            directory_open_flags(),
            Mode::empty(),
        )
        .map_err(|source| errno_error("open filesystem root", Path::new("/"), source))?,
    );
    let mut normalized = PathBuf::from("/");
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(component) => {
                normalized.push(component);
                directory = File::from(
                    rustix::fs::openat(
                        &directory,
                        component,
                        directory_open_flags(),
                        Mode::empty(),
                    )
                    .map_err(|source| {
                        if matches!(source, Errno::LOOP | Errno::NOTDIR) {
                            ControlSqliteError::UnsafeRuntimeStateRoot(normalized.clone())
                        } else {
                            errno_error("open runtime-state-root component", &normalized, source)
                        }
                    })?,
                );
            }
            Component::CurDir | Component::ParentDir | Component::Prefix(_) => {
                return Err(ControlSqliteError::UnsafeRuntimeStateRoot(
                    path.to_path_buf(),
                ));
            }
        }
    }
    Ok((normalized, directory))
}

/// Walk fixed safe descendants from one retained directory capability.
fn walk_private_descendants(
    root: &File,
    root_path: &Path,
    segments: &[OsString],
    create: bool,
) -> Result<Option<File>, ControlSqliteError> {
    let mut directory = root.try_clone().map_err(|source| {
        ControlSqliteError::io("clone retained runtime-state root", root_path, source)
    })?;
    let mut path = root_path.to_path_buf();
    for segment in segments {
        path.push(segment);
        let Some(next) = open_private_descendant(&directory, segment, &path, create)? else {
            return Ok(None);
        };
        directory = next;
    }
    Ok(Some(directory))
}

/// Open or create one owner-private descendant without following its leaf.
fn open_private_descendant(
    parent: &File,
    segment: &OsStr,
    path: &Path,
    create: bool,
) -> Result<Option<File>, ControlSqliteError> {
    let opened = match rustix::fs::openat(parent, segment, directory_open_flags(), Mode::empty()) {
        Ok(descriptor) => File::from(descriptor),
        Err(Errno::NOENT) if create => {
            match rustix::fs::mkdirat(parent, segment, Mode::from_raw_mode(0o700)) {
                Ok(()) => {
                    let descriptor =
                        rustix::fs::openat(parent, segment, directory_open_flags(), Mode::empty())
                            .map_err(|source| {
                                errno_error("open created control-storage directory", path, source)
                            })?;
                    let directory = File::from(descriptor);
                    rustix::fs::fchmod(&directory, Mode::from_raw_mode(0o700)).map_err(
                        |source| {
                            errno_error("set private control-storage permissions", path, source)
                        },
                    )?;
                    rustix::fs::fsync(parent).map_err(|source| {
                        errno_error("synchronize control-storage parent", path, source)
                    })?;
                    directory
                }
                Err(Errno::EXIST) => File::from(
                    rustix::fs::openat(parent, segment, directory_open_flags(), Mode::empty())
                        .map_err(|source| {
                            unsafe_descendant_or_io(
                                "open concurrently created control-storage directory",
                                path,
                                source,
                            )
                        })?,
                ),
                Err(source) => {
                    return Err(errno_error(
                        "create private control-storage directory",
                        path,
                        source,
                    ));
                }
            }
        }
        Err(Errno::NOENT) => return Ok(None),
        Err(source) => {
            return Err(unsafe_descendant_or_io(
                "open control-storage directory without following links",
                path,
                source,
            ));
        }
    };
    if !directory_is_private(&opened, path)? {
        return Err(ControlSqliteError::UnsafeControlStorageDirectory(
            path.to_path_buf(),
        ));
    }
    Ok(Some(opened))
}

/// Test one retained directory against the shared effective-UID and mode law.
fn directory_is_private(directory: &File, path: &Path) -> Result<bool, ControlSqliteError> {
    let metadata = directory.metadata().map_err(|source| {
        ControlSqliteError::io("inspect private control-storage directory", path, source)
    })?;
    Ok(metadata.is_dir()
        && metadata.uid() == rustix::process::geteuid().as_raw()
        && metadata.permissions().mode() & 0o077 == 0)
}

/// Test one descriptor-relative store observation against exact local file policy.
pub(crate) fn private_store_file(stat: &rustix::fs::Stat) -> bool {
    FileType::from_raw_mode(stat.st_mode).is_file()
        && stat.st_uid == rustix::process::geteuid().as_raw()
        && stat.st_mode & 0o777 == 0o600
}

/// Stable directory-open flags used by every root and descendant hop.
fn directory_open_flags() -> OFlags {
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
}

/// Classify link/non-directory failures without discarding other native causes.
fn unsafe_descendant_or_io(
    operation: &'static str,
    path: &Path,
    source: Errno,
) -> ControlSqliteError {
    if matches!(source, Errno::LOOP | Errno::NOTDIR) {
        ControlSqliteError::UnsafeControlStorageDirectory(path.to_path_buf())
    } else {
        errno_error(operation, path, source)
    }
}

/// Attach stable operation and path context to one Unix error number.
fn errno_error(
    operation: &'static str,
    path: impl Into<PathBuf>,
    source: Errno,
) -> ControlSqliteError {
    ControlSqliteError::io(
        operation,
        path,
        std::io::Error::from_raw_os_error(source.raw_os_error()),
    )
}

#[cfg(test)]
#[path = "../tests/internal/location.rs"]
mod tests;
