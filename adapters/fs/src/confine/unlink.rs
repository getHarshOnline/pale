//! Descriptor-relative file and directory removal with durable parent updates.

use super::{ConfinedLocation, FileIdentity};
#[cfg(not(unix))]
use crate::errors::unsupported;
use crate::errors::FsError;
#[cfg(unix)]
use crate::errors::{effect_uncertain, errno_error};

impl ConfinedLocation {
    /// Remove only the regular-file identity admitted through an earlier open.
    ///
    /// The comparison and unlink use the same retained parent descriptor, so
    /// ancestor replacement and pathname reopening cannot redirect removal.
    /// Like all POSIX name removal, the leaf binding is a cooperating-writer
    /// boundary; a replaced leaf is preserved as a collision.
    pub(crate) fn remove_file_if_identity(&self, expected: FileIdentity) -> Result<bool, FsError> {
        self.remove_if_identity(expected, false)
    }

    /// Remove only an empty directory with this exact observed identity.
    pub(crate) fn remove_directory_if_identity(
        &self,
        expected: FileIdentity,
    ) -> Result<bool, FsError> {
        self.remove_if_identity(expected, true)
    }

    fn remove_if_identity(&self, expected: FileIdentity, directory: bool) -> Result<bool, FsError> {
        #[cfg(not(unix))]
        {
            let _ = (expected, directory);
            return Err(unsupported("remove identity-bound confined entry"));
        }

        #[cfg(unix)]
        {
            use rustix::fs::{fsync, statat, unlinkat, AtFlags, FileType};
            use rustix::io::Errno;

            let Some(parent) = self.parent()? else {
                return Ok(false);
            };
            let parent_path = self.display_parent();
            let metadata = match statat(
                &parent.directory,
                parent.leaf.as_str(),
                AtFlags::SYMLINK_NOFOLLOW,
            ) {
                Ok(metadata) => metadata,
                Err(Errno::NOENT) => return Ok(false),
                Err(source) => {
                    return Err(errno_error(
                        "inspect identity-bound confined entry before removal",
                        self.display_path(),
                        source,
                    ));
                }
            };
            let kind = FileType::from_raw_mode(metadata.st_mode);
            if !(if directory {
                kind.is_dir()
            } else {
                kind.is_file()
            }) || u64::try_from(metadata.st_dev).ok() != Some(expected.device)
                || metadata.st_ino != expected.inode
            {
                return Err(FsError::Collision(
                    self.relative().to_string(),
                    "admitted entry was replaced; current entry preserved".into(),
                ));
            }
            match unlinkat(
                &parent.directory,
                parent.leaf.as_str(),
                if directory {
                    AtFlags::REMOVEDIR
                } else {
                    AtFlags::empty()
                },
            ) {
                Ok(()) => {
                    fsync(&parent.directory).map_err(|source| {
                        effect_uncertain(
                            "remove identity-bound confined entry",
                            self.display_path(),
                            errno_error(
                                "synchronize removed identity-bound entry parent",
                                &parent_path,
                                source,
                            ),
                        )
                    })?;
                    Ok(true)
                }
                Err(Errno::NOENT) => Ok(false),
                Err(source) => Err(errno_error(
                    "remove identity-bound confined entry",
                    self.display_path(),
                    source,
                )),
            }
        }
    }

    /// Remove a regular-file leaf without following it; absence returns `false`.
    ///
    /// The caller must establish ownership. Classification and unlink are a
    /// cooperating-writer boundary; a visible unlink with unproved durability is
    /// reported as [`FsError::EffectUncertain`].
    ///
    /// # Errors
    ///
    /// Returns [`FsError`] when confinement cannot be established, the leaf is
    /// not a regular file, removal fails, or parent synchronization is uncertain.
    pub fn remove_file(&self) -> Result<bool, FsError> {
        #[cfg(not(unix))]
        {
            return Err(unsupported("remove confined file"));
        }

        #[cfg(unix)]
        {
            use rustix::fs::{fsync, statat, unlinkat, AtFlags, FileType};
            use rustix::io::Errno;

            let Some(parent) = self.parent()? else {
                return Ok(false);
            };
            let parent_path = self.display_parent();
            let metadata = match statat(
                &parent.directory,
                parent.leaf.as_str(),
                AtFlags::SYMLINK_NOFOLLOW,
            ) {
                Ok(metadata) => metadata,
                Err(Errno::NOENT) => return Ok(false),
                Err(source) => {
                    return Err(errno_error(
                        "inspect confined file before removal",
                        self.display_path(),
                        source,
                    ));
                }
            };
            if !FileType::from_raw_mode(metadata.st_mode).is_file() {
                return Err(FsError::Collision(
                    self.relative().to_string(),
                    "expected a regular file for removal".into(),
                ));
            }
            match unlinkat(&parent.directory, parent.leaf.as_str(), AtFlags::empty()) {
                Ok(()) => {
                    fsync(&parent.directory).map_err(|source| {
                        effect_uncertain(
                            "remove confined file",
                            self.display_path(),
                            errno_error("synchronize removed file parent", &parent_path, source),
                        )
                    })?;
                    Ok(true)
                }
                Err(Errno::NOENT) => Ok(false),
                Err(source) => Err(errno_error(
                    "remove confined file",
                    self.display_path(),
                    source,
                )),
            }
        }
    }

    /// Remove an empty directory leaf after the caller proves transaction ownership.
    ///
    /// Absence returns `false`. Replacements, non-empty directories, and uncertain
    /// parent durability remain typed failures rather than inferred cleanup.
    ///
    /// # Errors
    ///
    /// Returns [`FsError`] when confinement cannot be established, the directory
    /// is not empty, removal fails, or parent synchronization is uncertain.
    pub fn remove_directory(&self) -> Result<bool, FsError> {
        #[cfg(not(unix))]
        {
            return Err(unsupported("remove confined directory"));
        }

        #[cfg(unix)]
        {
            use rustix::fs::{fsync, unlinkat, AtFlags};
            use rustix::io::Errno;

            let Some(parent) = self.parent()? else {
                return Ok(false);
            };
            let parent_path = self.display_parent();
            match unlinkat(&parent.directory, parent.leaf.as_str(), AtFlags::REMOVEDIR) {
                Ok(()) => {
                    fsync(&parent.directory).map_err(|source| {
                        effect_uncertain(
                            "remove confined directory",
                            self.display_path(),
                            errno_error(
                                "synchronize removed directory parent",
                                &parent_path,
                                source,
                            ),
                        )
                    })?;
                    Ok(true)
                }
                Err(Errno::NOENT) => Ok(false),
                Err(source) => Err(errno_error(
                    "remove confined directory",
                    self.display_path(),
                    source,
                )),
            }
        }
    }
}
