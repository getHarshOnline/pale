//! Descriptor-relative descendant traversal and exact ancestor recovery.

use std::fs::File;
use std::path::Path;

use super::{ConfinedLocation, ConfinedRoot, DirectoryLimit};
#[cfg(unix)]
use crate::errors::errno_error;
#[cfg(not(unix))]
use crate::errors::unsupported;
use crate::errors::{io_error, FsError};

/// Existing parent-directory capability paired with one validated leaf component.
pub(super) struct Parent {
    /// Retained descriptor for the existing parent directory.
    pub(super) directory: File,
    /// Validated single-component leaf beneath [`Self::directory`].
    pub(super) leaf: String,
}

/// Directory created by this adapter and retained for identity-checked recovery.
pub(crate) struct CreatedDirectory {
    location: ConfinedLocation,
    #[cfg(unix)]
    directory: Option<File>,
}

impl CreatedDirectory {
    fn pending(location: ConfinedLocation) -> Self {
        Self {
            location,
            #[cfg(unix)]
            directory: None,
        }
    }

    #[cfg(unix)]
    fn bind(&mut self, directory: &File) -> Result<(), FsError> {
        self.directory = Some(directory.try_clone().map_err(|source| {
            io_error(
                "retain created confined ancestor",
                self.location.display_path(),
                source,
            )
        })?);
        Ok(())
    }

    /// Return the validated relative directory name used in recovery diagnostics.
    pub(crate) fn relative(&self) -> &str {
        self.location.relative()
    }

    /// Identity of the descriptor retained when this transaction created the directory.
    pub(crate) fn identity(
        &self,
    ) -> Result<pale_core::records::MachineLocalDirectoryIdentity, FsError> {
        #[cfg(not(unix))]
        {
            return Err(unsupported("observe created confined ancestor identity"));
        }
        #[cfg(unix)]
        {
            let retained = self.directory.as_ref().ok_or_else(|| {
                FsError::Collision(
                    self.relative().to_owned(),
                    "created directory identity was not retained".into(),
                )
            })?;
            let stat = rustix::fs::fstat(retained).map_err(|source| {
                errno_error(
                    "inspect retained created ancestor",
                    self.location.display_path(),
                    source,
                )
            })?;
            Ok(pale_core::records::MachineLocalDirectoryIdentity {
                rel_path: self.relative().to_owned(),
                device: pale_core::records::FilesystemDeviceIdV1::new(
                    u64::try_from(stat.st_dev).map_err(|source| {
                        crate::errors::data_error(
                            "encode created ancestor device",
                            self.location.display_path(),
                            source,
                        )
                    })?,
                ),
                inode: pale_core::records::FilesystemInodeIdV1::new(stat.st_ino),
            })
        }
    }

    /// Remove only the directory identity retained when this transaction created it.
    pub(crate) fn remove_if_unchanged(&self) -> Result<(), FsError> {
        let expected = self.identity()?;
        self.location
            .remove_directory_if_identity(super::FileIdentity {
                device: expected.device.get(),
                inode: expected.inode.get(),
            })
            .map(drop)
    }
}

enum AncestorJournal<'a> {
    Locations(&'a mut Vec<ConfinedLocation>),
    Exact(&'a mut Vec<CreatedDirectory>),
}

impl AncestorJournal<'_> {
    fn reserve(&mut self, path: &Path) -> Result<(), FsError> {
        let result = match self {
            Self::Locations(entries) => entries.try_reserve(1),
            Self::Exact(entries) => entries.try_reserve(1),
        };
        result.map_err(|source| {
            crate::errors::data_error("reserve ancestor recovery journal", path, source)
        })
    }

    fn record_visible(&mut self, location: ConfinedLocation) {
        match self {
            Self::Locations(entries) => entries.push(location),
            Self::Exact(entries) => entries.push(CreatedDirectory::pending(location)),
        }
    }

    fn bind_latest(&mut self, directory: &File) -> Result<(), FsError> {
        match self {
            Self::Locations(_) => Ok(()),
            Self::Exact(entries) => entries
                .last_mut()
                .ok_or_else(|| {
                    crate::errors::invariant(
                        "bind created confined ancestor identity",
                        Path::new("<ancestor-journal>"),
                        "visible directory has no recovery journal entry",
                    )
                })?
                .bind(directory),
        }
    }
}

impl ConfinedRoot {
    /// Count immediate children through the retained root capability.
    ///
    /// # Errors
    ///
    /// Returns [`FsError`] when directory enumeration is unsupported, an entry
    /// cannot be read, or the supplied [`DirectoryLimit`] is exceeded.
    pub fn entry_count(&self, limit: DirectoryLimit) -> Result<usize, FsError> {
        #[cfg(not(unix))]
        {
            let _ = limit;
            return Err(unsupported("count confined directory entries"));
        }

        #[cfg(unix)]
        {
            use rustix::fs::Dir;

            let mut entries = Dir::read_from(&self.handle.directory).map_err(|source| {
                errno_error(
                    "open confinement root directory stream",
                    self.canonical_path(),
                    source,
                )
            })?;
            let mut count = 0;
            for entry in &mut entries {
                let entry = entry.map_err(|source| {
                    errno_error(
                        "read confinement root directory entry",
                        self.canonical_path(),
                        source,
                    )
                })?;
                let name = entry.file_name().to_bytes();
                if name == b"." || name == b".." {
                    continue;
                }
                if count >= limit.max_entries() {
                    return Err(FsError::DirectoryEntryLimit {
                        path: self.canonical_path().display().to_string(),
                        max_entries: limit.max_entries(),
                    });
                }
                count += 1;
            }
            Ok(count)
        }
    }

    #[cfg(unix)]
    /// Resolve an existing parent without creating missing ancestors.
    pub(super) fn existing_parent(&self, rel: &str) -> Result<Option<Parent>, FsError> {
        self.open_parent_with_sync(rel, false, &mut Vec::new(), &mut |directory, path| {
            rustix::fs::fsync(directory)
                .map_err(|source| errno_error("sync created directory parent", path, source))
        })
    }

    #[cfg(unix)]
    /// Resolve a parent, creating and reporting missing ancestors for the caller.
    pub(super) fn ensure_parent(
        &self,
        rel: &str,
        created: &mut Vec<ConfinedLocation>,
    ) -> Result<Parent, FsError> {
        let mut journal = AncestorJournal::Locations(created);
        self.open_parent_with_journal(rel, true, &mut journal, &mut |directory, path| {
            rustix::fs::fsync(directory)
                .map_err(|source| errno_error("sync created directory parent", path, source))
        })?
        .ok_or_else(|| {
            crate::errors::invariant(
                "create confined parent",
                self.canonical_path().join(rel),
                "parent creation produced no directory",
            )
        })
    }

    #[cfg(unix)]
    /// Resolve a parent while retaining created-directory identities for recovery.
    pub(super) fn ensure_parent_tracked(
        &self,
        rel: &str,
        created: &mut Vec<CreatedDirectory>,
    ) -> Result<Parent, FsError> {
        self.ensure_parent_tracked_with_sync(rel, created, &mut |directory, path| {
            rustix::fs::fsync(directory)
                .map_err(|source| errno_error("sync created directory parent", path, source))
        })
    }

    #[cfg(unix)]
    fn ensure_parent_tracked_with_sync<F>(
        &self,
        rel: &str,
        created: &mut Vec<CreatedDirectory>,
        sync_parent: &mut F,
    ) -> Result<Parent, FsError>
    where
        F: FnMut(&File, &Path) -> Result<(), FsError>,
    {
        let mut journal = AncestorJournal::Exact(created);
        self.open_parent_with_journal(rel, true, &mut journal, sync_parent)?
            .ok_or_else(|| {
                crate::errors::invariant(
                    "create confined parent",
                    self.canonical_path().join(rel),
                    "parent creation produced no directory",
                )
            })
    }

    #[cfg(unix)]
    fn open_parent_with_sync<F>(
        &self,
        rel: &str,
        create: bool,
        created: &mut Vec<ConfinedLocation>,
        sync_parent: &mut F,
    ) -> Result<Option<Parent>, FsError>
    where
        F: FnMut(&File, &Path) -> Result<(), FsError>,
    {
        let mut journal = AncestorJournal::Locations(created);
        self.open_parent_with_journal(rel, create, &mut journal, sync_parent)
    }

    #[cfg(unix)]
    fn open_parent_with_journal<F>(
        &self,
        rel: &str,
        create: bool,
        journal: &mut AncestorJournal<'_>,
        sync_parent: &mut F,
    ) -> Result<Option<Parent>, FsError>
    where
        F: FnMut(&File, &Path) -> Result<(), FsError>,
    {
        use rustix::fs::{mkdirat, openat, Mode, OFlags};
        use rustix::io::Errno;

        let relative = pale_core::paths::ConfinedRelativePath::parse(rel)
            .ok_or_else(|| FsError::Escape(rel.to_string()))?;
        let mut components = relative.components().peekable();
        let mut directory = self.handle.directory.try_clone().map_err(|source| {
            io_error(
                "clone confinement root handle",
                self.canonical_path(),
                source,
            )
        })?;
        let mut walked = String::new();
        while let Some(component) = components.next() {
            if components.peek().is_none() {
                return Ok(Some(Parent {
                    directory,
                    leaf: component.to_string(),
                }));
            }
            let parent_path = self.canonical_path().join(&walked);
            if !walked.is_empty() {
                walked.push('/');
            }
            walked.push_str(component);
            let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
            let next = match openat(&directory, component, flags, Mode::empty()) {
                Ok(fd) => File::from(fd),
                Err(Errno::NOENT) if create => {
                    let created_path = self.canonical_path().join(&walked);
                    journal.reserve(&created_path)?;
                    match mkdirat(&directory, component, Mode::from_raw_mode(0o755)) {
                        Ok(()) => {
                            journal.record_visible(ConfinedLocation {
                                root: self.clone(),
                                relative: walked.clone(),
                            });
                            let created_directory = File::from(
                                openat(&directory, component, flags, Mode::empty()).map_err(
                                    |source| {
                                        errno_error(
                                            "open created confined ancestor",
                                            &created_path,
                                            source,
                                        )
                                    },
                                )?,
                            );
                            journal.bind_latest(&created_directory)?;
                            sync_parent(&directory, &parent_path)?;
                            created_directory
                        }
                        Err(Errno::EXIST) => File::from(
                            openat(&directory, component, flags, Mode::empty()).map_err(
                                |source| {
                                    errno_error(
                                        "open concurrently created confined ancestor",
                                        &created_path,
                                        source,
                                    )
                                },
                            )?,
                        ),
                        Err(source) => {
                            return Err(errno_error(
                                "create confined ancestor",
                                &created_path,
                                source,
                            ));
                        }
                    }
                }
                Err(Errno::NOENT) => return Ok(None),
                Err(source) => {
                    return Err(errno_error(
                        "open confined ancestor without following links",
                        self.canonical_path().join(&walked),
                        source,
                    ));
                }
            };
            directory = next;
        }
        Err(FsError::Escape(rel.to_string()))
    }

    #[cfg(not(unix))]
    /// Reject parent resolution where descriptor-relative confinement is unsupported.
    pub(super) fn existing_parent(&self, rel: &str) -> Result<Option<Parent>, FsError> {
        let _ = (self, rel);
        Err(unsupported("resolve confined descendant"))
    }

    #[cfg(not(unix))]
    /// Reject parent creation where descriptor-relative confinement is unsupported.
    pub(super) fn ensure_parent(
        &self,
        rel: &str,
        created: &mut Vec<ConfinedLocation>,
    ) -> Result<Parent, FsError> {
        let _ = (self, rel, created);
        Err(unsupported("create confined descendant parent"))
    }

    #[cfg(not(unix))]
    /// Reject tracked parent creation where confinement is unsupported.
    pub(super) fn ensure_parent_tracked(
        &self,
        rel: &str,
        created: &mut Vec<CreatedDirectory>,
    ) -> Result<Parent, FsError> {
        let _ = (self, rel, created);
        Err(unsupported("create confined descendant parent"))
    }
}

#[cfg(all(test, unix))]
#[path = "../../tests/internal/confine/traversal.rs"]
mod tests;
