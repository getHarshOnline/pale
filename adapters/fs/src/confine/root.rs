//! Root capabilities and platform-specific descendant traversal.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(unix)]
use crate::confine::canonical_root;
#[cfg(unix)]
use crate::errors::errno_error;
#[cfg(not(unix))]
use crate::errors::unsupported;
use crate::errors::{io_error, FsError};
use pale_core::records::{FilesystemDeviceIdV1, FilesystemInodeIdV1, MachineLocalRootIdentity};

#[derive(Debug)]
/// Shared retained descriptor and canonical diagnostic identity for one root.
pub(super) struct RootHandle {
    /// Canonical pathname verified against [`Self::directory`] at admission.
    pub(super) canonical: PathBuf,
    /// Directory capability from which descendant operations resolve.
    pub(super) directory: File,
}

/// A lexically validated relative name bound to a [`ConfinedRoot`].
/// Construction does not inspect the descendant path.
#[derive(Debug, Clone)]
pub struct ConfinedLocation {
    /// Retained root capability from which every descendant operation resolves.
    pub(super) root: ConfinedRoot,
    /// Canonical project-local relative path admitted at construction.
    pub(super) relative: String,
}

/// Retained root capability for confined descendant I/O.
///
/// The current implementation requires Unix descriptor-relative traversal. A
/// non-Unix build returns [`FsError::UnsupportedPlatform`] from [`Self::open`]
/// rather than providing weaker path-based behavior.
#[derive(Debug, Clone)]
pub struct ConfinedRoot {
    /// Shared retained root identity; cloning does not reopen or rebind the path.
    pub(super) handle: Arc<RootHandle>,
}

impl ConfinedRoot {
    /// Open an existing directory as a confinement root.
    ///
    /// Unix rejects a symlink leaf and verifies that the retained descriptor and
    /// canonical path identify the same directory.
    ///
    /// # Errors
    ///
    /// Fails on unsupported platforms, missing or non-directory roots, symlink
    /// roots, path races, and any open, canonicalization, or inspection failure.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use pale_adapter_fs::{ConfinedRoot, FsError, ReadLimit};
    ///
    /// # fn inspect(root: &Path) -> Result<(), FsError> {
    /// let root = ConfinedRoot::open(root)?;
    /// let settings = root.location("config/settings.json")?;
    /// let _current = settings.read_file(ReadLimit::new(64 * 1024))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn open(path: &Path) -> Result<Self, FsError> {
        #[cfg(not(unix))]
        {
            let _ = path;
            return Err(unsupported("open confined root"));
        }

        #[cfg(unix)]
        {
            use rustix::fs::{openat, Mode, OFlags, CWD};
            use std::os::unix::fs::MetadataExt;

            let fd = openat(
                CWD,
                path,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|source| errno_error("open confinement root", path, source))?;
            let directory = File::from(fd);
            let canonical = canonical_root(path)?;
            let canonical_fd = openat(
                CWD,
                canonical.as_path(),
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|source| errno_error("open canonical confinement root", &canonical, source))?;
            let opened = directory
                .metadata()
                .map_err(|source| io_error("inspect confinement root", path, source))?;
            let resolved_file = File::from(canonical_fd);
            let resolved = resolved_file.metadata().map_err(|source| {
                io_error("inspect canonical confinement root", &canonical, source)
            })?;
            if !opened.is_dir() || !resolved.is_dir() {
                return Err(FsError::Collision(
                    path.display().to_string(),
                    "confinement root is not a directory".into(),
                ));
            }
            if opened.dev() != resolved.dev() || opened.ino() != resolved.ino() {
                return Err(FsError::Collision(
                    path.display().to_string(),
                    "confinement root changed while it was opened".into(),
                ));
            }
            Ok(Self {
                handle: Arc::new(RootHandle {
                    canonical,
                    directory,
                }),
            })
        }
    }

    /// Canonical identity, diagnostic path, and explicit process-handoff root.
    ///
    /// It is not a substitute for descriptor-relative I/O. Call
    /// [`Self::verify_path_identity`] immediately before launching a process that
    /// consumes this pathname.
    pub fn canonical_path(&self) -> &Path {
        &self.handle.canonical
    }

    /// Confirm that the canonical pathname still names this retained directory.
    ///
    /// Descriptor-relative reads remain confined after a rename, but an external
    /// process launched with a pathname would otherwise observe a replacement root.
    /// Reopen or identity mismatch is a collision, never an implicit rebinding.
    ///
    /// # Errors
    ///
    /// Fails when the canonical pathname cannot be reopened and when its device and
    /// inode no longer match the retained root capability.
    pub fn verify_path_identity(&self) -> Result<(), FsError> {
        #[cfg(not(unix))]
        {
            return Err(unsupported("verify confined root identity"));
        }

        #[cfg(unix)]
        {
            use rustix::fs::{openat, Mode, OFlags, CWD};
            use std::os::unix::fs::MetadataExt;

            let current = File::from(
                openat(
                    CWD,
                    self.canonical_path(),
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(|source| {
                    errno_error(
                        "reopen confinement root pathname",
                        self.canonical_path(),
                        source,
                    )
                })?,
            );
            let retained = self.handle.directory.metadata().map_err(|source| {
                io_error(
                    "inspect retained confinement root",
                    self.canonical_path(),
                    source,
                )
            })?;
            let current = current.metadata().map_err(|source| {
                io_error(
                    "inspect reopened confinement root",
                    self.canonical_path(),
                    source,
                )
            })?;
            if retained.dev() != current.dev() || retained.ino() != current.ino() {
                return Err(FsError::Collision(
                    self.canonical_path().display().to_string(),
                    "confinement root pathname now identifies another directory".into(),
                ));
            }
            Ok(())
        }
    }

    /// Capture the retained root's persisted machine-local identity after
    /// confirming its canonical path still resolves to the same directory.
    pub(crate) fn machine_local_identity(&self) -> Result<MachineLocalRootIdentity, FsError> {
        self.verify_path_identity()?;
        #[cfg(not(unix))]
        {
            Err(unsupported("capture machine-local root identity"))
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            let metadata = self.handle.directory.metadata().map_err(|source| {
                io_error(
                    "inspect machine-local root identity",
                    self.canonical_path(),
                    source,
                )
            })?;
            let canonical_path = self
                .canonical_path()
                .to_str()
                .ok_or_else(|| {
                    crate::errors::invariant(
                        "encode machine-local root identity",
                        self.canonical_path(),
                        "canonical path is not UTF-8",
                    )
                })?
                .to_string();
            Ok(MachineLocalRootIdentity {
                canonical_path,
                device: FilesystemDeviceIdV1::new(metadata.dev()),
                inode: FilesystemInodeIdV1::new(metadata.ino()),
            })
        }
    }

    /// Validate and bind a repository-relative path without inspecting the filesystem.
    /// Returns [`FsError::Escape`] unless `rel` is an admitted project-local
    /// relative path; later effects still resolve every ancestor from the retained
    /// root handle.
    ///
    /// # Errors
    ///
    /// Returns [`FsError::Escape`] when `rel` is empty, absolute, non-canonical, or
    /// otherwise outside the admitted project-local grammar.
    pub fn location(&self, rel: &str) -> Result<ConfinedLocation, FsError> {
        pale_core::paths::ConfinedRelativePath::parse(rel)
            .ok_or_else(|| FsError::Escape(rel.to_string()))?;
        Ok(ConfinedLocation {
            root: self.clone(),
            relative: rel.to_string(),
        })
    }
}
