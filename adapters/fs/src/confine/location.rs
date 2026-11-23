//! Bounded file and directory access bound to a retained root capability.

use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use pale_core::records::ArtifactDigest;

use super::limits::ReadLimit;
#[cfg(unix)]
use super::mode_of;
use super::root::ConfinedLocation;
use super::traversal::Parent;
use super::CreatedDirectory;
#[cfg(unix)]
use crate::errors::errno_error;
#[cfg(not(unix))]
use crate::errors::unsupported;
use crate::errors::{data_error, FsError};

/// Bounded bytes and mode observed from one file handle.
///
/// This is not a lock or a filesystem snapshot; mutation requires a fresh identity
/// check. Debug output deliberately reports only byte length, never content.
pub struct ExistingFile {
    /// Complete bounded content from the opened handle.
    pub bytes: Vec<u8>,
    /// Unix permission bits.
    pub mode: u32,
    /// Open-handle identity retained for exact in-process recovery checks.
    pub(crate) identity: FileIdentity,
}

impl fmt::Debug for ExistingFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExistingFile")
            .field(
                "bytes",
                &format_args!("<redacted:{} bytes>", self.bytes.len()),
            )
            .field("mode", &format_args!("{:#o}", self.mode))
            .finish()
    }
}

/// Digest and metadata observed from one bounded file stream.
#[derive(Debug)]
pub(crate) struct ExistingFileDigest {
    /// Digest of the complete bounded stream.
    pub(crate) sha256: ArtifactDigest,
    /// Unix permission bits observed from the opened handle.
    pub(crate) mode: u32,
}

/// Metadata observed from one bounded regular-file handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExistingFileMetadata {
    /// Unix permission bits.
    pub mode: u32,
    /// Metadata length, which is not a content identity.
    pub len: u64,
}

/// Admitted no-follow handle plus metadata captured before streaming begins.
///
/// The metadata length is an early admission bound and allocation hint; streaming
/// enforces the explicit [`ReadLimit`] independently. This value is not a coherent
/// snapshot against external writers.
struct OpenedFile {
    file: File,
    mode: u32,
    reported_len: u64,
    identity: FileIdentity,
}

/// Stable identity of one opened regular-file handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FileIdentity {
    /// Filesystem device containing the retained entry.
    pub(crate) device: u64,
    /// Inode of the retained entry on [`Self::device`].
    pub(crate) inode: u64,
}

/// Reused read chunk; the caller-supplied [`ReadLimit`] remains authoritative.
const READ_BUFFER_BYTES: usize = 64 * 1024;

impl ConfinedLocation {
    /// Validated path relative to the owning root.
    pub fn relative(&self) -> &str {
        &self.relative
    }

    /// Absolute pathname for diagnostics.
    ///
    /// Reopening it does not preserve confinement or authenticate this descendant.
    /// Root identity verification protects only the root pathname; a caller handing
    /// this descendant to another process must separately re-admit that exact
    /// pathname or transfer a verified handle at the process boundary.
    pub fn display_path(&self) -> PathBuf {
        self.root.canonical_path().join(&self.relative)
    }

    /// Diagnostic pathname of the retained descendant's parent.
    pub(super) fn display_parent(&self) -> PathBuf {
        let path = self.display_path();
        path.parent().map_or_else(|| path.clone(), PathBuf::from)
    }

    /// Check existing ancestors now; later I/O resolves them again.
    /// Missing parents and the leaf are not required to exist.
    pub(crate) fn validate_ancestors(&self) -> Result<(), FsError> {
        self.parent().map(|_| ())
    }

    /// Create missing parents with mode `0o755`, using no-follow resolution.
    ///
    /// An error can follow a successful directory creation, including when parent
    /// synchronization fails. Such directories remain in place: this convenience
    /// effect carries no rollback authority. Identity-bound transactions use the
    /// private tracked variant instead.
    ///
    /// # Errors
    ///
    /// Returns [`FsError`] when an ancestor is unsafe, cannot be created, or
    /// cannot be synchronized durably.
    pub fn ensure_parent(&self) -> Result<(), FsError> {
        let mut observed_creations = Vec::new();
        self.root
            .ensure_parent(&self.relative, &mut observed_creations)?;
        Ok(())
    }

    /// Create missing parents while reporting whether this call made a visible
    /// namespace change. A failure after a reported creation has an unknown
    /// auxiliary outcome because durability or final admission was not proved.
    pub(crate) fn ensure_parent_observed(&self) -> Result<bool, (FsError, bool)> {
        let mut observed_creations = Vec::new();
        match self
            .root
            .ensure_parent(&self.relative, &mut observed_creations)
        {
            Ok(_) => Ok(!observed_creations.is_empty()),
            Err(source) => Err((source, !observed_creations.is_empty())),
        }
    }

    /// Create missing parents while retaining exact directory identities for rollback.
    pub(crate) fn ensure_parent_tracked(
        &self,
        created: &mut Vec<CreatedDirectory>,
    ) -> Result<(), FsError> {
        self.root.ensure_parent_tracked(&self.relative, created)?;
        Ok(())
    }

    /// Read a bounded regular file without following the leaf.
    ///
    /// Absence returns `None`; unsafe entry kinds are collisions. Limit exhaustion,
    /// allocation failure, and I/O failure are reported without returning partial
    /// bytes.
    ///
    /// # Errors
    ///
    /// Rejects symlink or non-regular leaves, growth beyond `limit`, allocation
    /// failure, and any incomplete open, metadata, or read operation.
    pub fn read_file(&self, limit: ReadLimit) -> Result<Option<ExistingFile>, FsError> {
        let Some(opened) = self.open_regular_file(limit)? else {
            return Ok(None);
        };
        let max_bytes = limit.max_bytes();
        let initial_capacity = usize::try_from(opened.reported_len)
            .unwrap_or(usize::MAX)
            .min(max_bytes)
            .min(READ_BUFFER_BYTES);
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(initial_capacity)
            .map_err(|source| {
                data_error("reserve bounded file buffer", self.display_path(), source)
            })?;
        let mut reader = opened.file.take(read_ceiling(max_bytes));
        let mut buffer = Vec::new();
        buffer
            .try_reserve_exact(READ_BUFFER_BYTES)
            .map_err(|source| {
                data_error("reserve file read buffer", self.display_path(), source)
            })?;
        buffer.resize(READ_BUFFER_BYTES, 0);
        loop {
            let read = reader.read(&mut buffer).map_err(|source| {
                crate::errors::io_error("read confined file", self.display_path(), source)
            })?;
            if read == 0 {
                break;
            }
            if bytes
                .len()
                .checked_add(read)
                .is_none_or(|length| length > max_bytes)
            {
                return Err(self.file_limit_error(limit));
            }
            bytes.try_reserve(read).map_err(|source| {
                data_error("grow bounded file buffer", self.display_path(), source)
            })?;
            bytes.extend_from_slice(&buffer[..read]);
        }
        Ok(Some(ExistingFile {
            bytes,
            mode: opened.mode,
            identity: opened.identity,
        }))
    }

    /// Stream a bounded regular file into its content identity without retaining bytes.
    pub(crate) fn digest_file(
        &self,
        limit: ReadLimit,
    ) -> Result<Option<ExistingFileDigest>, FsError> {
        let Some(opened) = self.open_regular_file(limit)? else {
            return Ok(None);
        };
        let mode = opened.mode;
        let mut reader = opened.file.take(read_ceiling(limit.max_bytes()));
        let sha256 =
            ArtifactDigest::parse(pale_core::sha256_reader(&mut reader).map_err(|source| {
                crate::errors::io_error("digest confined file", self.display_path(), source)
            })?)
            .map_err(|source| {
                data_error(
                    "construct streamed artifact digest",
                    self.display_path(),
                    source,
                )
            })?;
        if reader.limit() == 0 {
            return Err(self.file_limit_error(limit));
        }
        Ok(Some(ExistingFileDigest { sha256, mode }))
    }

    /// Inspect bounded regular-file metadata without establishing content identity.
    /// Unsafe entries, a reported length above `limit`, and inspection failures are
    /// rejected. The result is an observation, not an ownership proof.
    ///
    /// # Errors
    ///
    /// Rejects symlink or non-regular leaves, a reported size above `limit`, and
    /// incomplete ancestor, open, or metadata operations.
    pub fn inspect_file(&self, limit: ReadLimit) -> Result<Option<ExistingFileMetadata>, FsError> {
        Ok(self
            .open_regular_file(limit)?
            .map(|opened| ExistingFileMetadata {
                mode: opened.mode,
                len: opened.reported_len,
            }))
    }

    #[cfg(unix)]
    fn open_regular_file(&self, limit: ReadLimit) -> Result<Option<OpenedFile>, FsError> {
        use rustix::fs::{openat, Mode, OFlags};
        use rustix::io::Errno;
        use std::os::unix::fs::MetadataExt;

        let Some(parent) = self.parent()? else {
            return Ok(None);
        };
        // Check the opened handle without waiting if the leaf became a FIFO.
        // Nonblocking mode leaves regular-file reads unchanged.
        let file = match openat(
            &parent.directory,
            parent.leaf.as_str(),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        ) {
            Ok(fd) => File::from(fd),
            Err(Errno::NOENT) => return Ok(None),
            Err(source) => {
                return Err(errno_error(
                    "open confined regular file",
                    self.display_path(),
                    source,
                ));
            }
        };
        let metadata = file.metadata().map_err(|source| {
            crate::errors::io_error("inspect confined regular file", self.display_path(), source)
        })?;
        if !metadata.is_file() {
            return Err(FsError::Collision(
                self.relative.clone(),
                "expected a regular file".into(),
            ));
        }
        if metadata.len() > u64::try_from(limit.max_bytes()).unwrap_or(u64::MAX) {
            return Err(self.file_limit_error(limit));
        }
        Ok(Some(OpenedFile {
            file,
            mode: mode_of(&metadata),
            reported_len: metadata.len(),
            identity: FileIdentity {
                device: metadata.dev(),
                inode: metadata.ino(),
            },
        }))
    }

    #[cfg(not(unix))]
    fn open_regular_file(&self, limit: ReadLimit) -> Result<Option<OpenedFile>, FsError> {
        let _ = (self, limit);
        Err(unsupported("open confined regular file"))
    }

    fn file_limit_error(&self, limit: ReadLimit) -> FsError {
        FsError::FileReadLimit {
            path: self.relative.clone(),
            max_bytes: limit.max_bytes(),
        }
    }

    #[cfg(unix)]
    /// Open this location as a no-follow directory capability when it exists.
    pub(super) fn open_directory(&self) -> Result<Option<File>, FsError> {
        use rustix::fs::{openat, Mode, OFlags};
        use rustix::io::Errno;

        let Some(parent) = self.parent()? else {
            return Ok(None);
        };
        match openat(
            &parent.directory,
            parent.leaf.as_str(),
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(fd) => Ok(Some(File::from(fd))),
            Err(Errno::NOENT) => Ok(None),
            Err(source) => Err(errno_error(
                "open confined directory",
                self.display_path(),
                source,
            )),
        }
    }

    /// Observe the exact identity of an existing no-follow directory.
    pub(crate) fn directory_identity(&self) -> Result<Option<FileIdentity>, FsError> {
        #[cfg(not(unix))]
        {
            return Err(unsupported("inspect confined directory identity"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            let Some(directory) = self.open_directory()? else {
                return Ok(None);
            };
            let metadata = directory.metadata().map_err(|source| {
                crate::errors::io_error(
                    "inspect confined directory identity",
                    self.display_path(),
                    source,
                )
            })?;
            Ok(Some(FileIdentity {
                device: metadata.dev(),
                inode: metadata.ino(),
            }))
        }
    }

    #[cfg(not(unix))]
    /// Fail closed where descriptor-relative directory opening is unavailable.
    pub(super) fn open_directory(&self) -> Result<Option<File>, FsError> {
        Err(unsupported("open confined directory"))
    }

    /// Resolve the existing parent capability and leaf name without following links.
    pub(super) fn parent(&self) -> Result<Option<Parent>, FsError> {
        self.root.existing_parent(&self.relative)
    }
}

fn read_ceiling(max_bytes: usize) -> u64 {
    u64::try_from(max_bytes)
        .unwrap_or(u64::MAX)
        .saturating_add(1)
}
