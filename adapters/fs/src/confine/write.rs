//! Per-leaf staged publication with Unix descriptor-relative durability.
//!
//! Effects resolve from retained directory descriptors. A successful call syncs
//! the file effect and its parent directory, but this layer does not make a
//! sequence of calls crash-atomic.

#[cfg(unix)]
use std::fs::File;
#[cfg(unix)]
use std::io::Write;

use super::root::ConfinedLocation;
use super::traversal::Parent;
#[cfg(unix)]
use crate::errors::invariant;
#[cfg(not(unix))]
use crate::errors::unsupported;
use crate::errors::FsError;
#[cfg(unix)]
use crate::errors::{effect_uncertain, errno_error};

#[derive(Clone, Copy)]
/// Durability policy selected by the semantic class of the caller's record.
///
/// Every class receives file and parent synchronization. On Apple platforms,
/// only [`Self::Durable`] additionally requests a full-media flush.
enum PublicationClass {
    Durable,
    MaterializedSurface,
    SessionContinuity,
}

#[cfg(unix)]
#[derive(Clone, Copy)]
/// Metadata captured from the private staging file before publication.
struct StagingIdentity(rustix::fs::Stat);

#[cfg(unix)]
fn sync_file(
    file: &File,
    class: PublicationClass,
    location: &ConfinedLocation,
) -> Result<(), FsError> {
    #[cfg(target_vendor = "apple")]
    if matches!(class, PublicationClass::Durable) {
        return rustix::fs::fcntl_fullfsync(file).map_err(|source| {
            errno_error(
                "fully synchronize durable file",
                location.display_path(),
                source,
            )
        });
    }
    rustix::fs::fsync(file).map_err(|source| {
        errno_error(
            "synchronize published file",
            location.display_path(),
            source,
        )
    })
}

#[cfg(unix)]
fn random_suffix() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish()
}

#[cfg(unix)]
fn admitted_mode(location: &ConfinedLocation, mode: u32) -> Result<rustix::fs::Mode, FsError> {
    use rustix::fs::{Mode, RawMode};

    if mode & !0o7777 != 0 {
        return Err(FsError::InvalidMode {
            path: location.relative().to_string(),
            mode,
        });
    }
    let raw = RawMode::try_from(mode).map_err(|_| FsError::InvalidMode {
        path: location.relative().to_string(),
        mode,
    })?;
    Ok(Mode::from_raw_mode(raw))
}

#[cfg(unix)]
fn admit_replacement(location: &ConfinedLocation, parent: &Parent) -> Result<(), FsError> {
    use rustix::fs::{statat, AtFlags, FileType};
    use rustix::io::Errno;

    match statat(
        &parent.directory,
        parent.leaf.as_str(),
        AtFlags::SYMLINK_NOFOLLOW,
    ) {
        Ok(metadata) if FileType::from_raw_mode(metadata.st_mode).is_file() => Ok(()),
        Ok(_) => Err(FsError::Collision(
            location.relative().to_string(),
            "replacement target is not a regular file".into(),
        )),
        Err(Errno::NOENT) => Ok(()),
        Err(source) => Err(errno_error(
            "inspect confined replacement target",
            location.display_path(),
            source,
        )),
    }
}

impl ConfinedLocation {
    /// Validate a planned mode before a multi-surface operation begins publishing.
    pub(crate) fn validate_mode(&self, mode: u32) -> Result<(), FsError> {
        admitted_mode(self, mode).map(drop)
    }

    /// Atomically replace a missing or regular-file leaf with exact bytes and mode.
    ///
    /// This is the strongest durable publication class and may use an Apple
    /// full-media flush. It does not establish ownership: callers must authenticate
    /// any replaceable regular file. Non-regular leaves are collisions at the
    /// cooperating-writer check. The parent directory must already exist.
    /// [`FsError::EffectUncertain`] means replacement became visible but completion
    /// failed, so callers must reconcile rather than infer absence.
    ///
    /// # Errors
    ///
    /// Rejects invalid modes, missing or unsafe parents, and non-regular leaves.
    /// Staging, publication, synchronization, or cleanup failures retain whether the
    /// destination effect may already be visible.
    pub fn write_atomic(&self, bytes: &[u8], mode: u32) -> Result<(), FsError> {
        self.staged_write(bytes, mode, true, PublicationClass::Durable)
    }

    /// Atomically create this file without replacing an existing entry.
    /// The post-publication error boundary matches [`Self::write_atomic`]. Any
    /// existing destination is a collision, and the parent directory must already
    /// exist. A crash after the no-clobber link but before cleanup can leave a
    /// private staging hard link beside the visible file; post-link uncertainty is
    /// typed as [`FsError::EffectUncertain`].
    ///
    /// # Errors
    ///
    /// In addition to the staged-publication failures of [`Self::write_atomic`], an
    /// existing destination is always a collision and is never replaced.
    pub fn write_atomic_noclobber(&self, bytes: &[u8], mode: u32) -> Result<(), FsError> {
        self.staged_write(bytes, mode, false, PublicationClass::Durable)
    }

    /// Publish a generated surface without claiming durable-evidence semantics.
    ///
    /// File and parent-directory synchronization still make each leaf
    /// crash-consistent. On Apple platforms this deliberately avoids the stronger
    /// physical-media flush reserved for durable evidence. A visible effect whose
    /// completion cannot be confirmed returns [`FsError::EffectUncertain`].
    pub(crate) fn write_materialized_surface(
        &self,
        bytes: &[u8],
        mode: u32,
    ) -> Result<(), FsError> {
        self.staged_write(bytes, mode, true, PublicationClass::MaterializedSurface)
    }

    /// Atomically replace expiring session-continuity state.
    ///
    /// This path uses ordinary file and directory synchronization. It does not
    /// pay Apple's stronger durable-evidence flush on every state transition.
    pub(crate) fn write_session_state(&self, bytes: &[u8], mode: u32) -> Result<(), FsError> {
        self.staged_write(bytes, mode, true, PublicationClass::SessionContinuity)
    }

    #[cfg(unix)]
    fn staged_write(
        &self,
        bytes: &[u8],
        mode: u32,
        clobber: bool,
        class: PublicationClass,
    ) -> Result<(), FsError> {
        use rustix::fs::{fchmod, fsync, linkat, openat, renameat, AtFlags, OFlags};
        use rustix::io::Errno;

        let admitted_mode = admitted_mode(self, mode)?;
        let parent = self.parent()?.ok_or_else(|| {
            invariant(
                "publish confined file",
                self.display_path(),
                "destination parent is missing",
            )
        })?;
        if clobber {
            admit_replacement(self, &parent)?;
        }
        let temporary = format!(".pale-stage-{:016x}.tmp", random_suffix());
        let parent_path = self.display_parent();
        let temporary_path = parent_path.join(&temporary);
        let fd = openat(
            &parent.directory,
            temporary.as_str(),
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            admitted_mode,
        )
        .map_err(|source| errno_error("create staging file", &temporary_path, source))?;
        let mut file = File::from(fd);
        let staging_identity = StagingIdentity(
            rustix::fs::fstat(&file)
                .map_err(|source| errno_error("inspect staging file", &temporary_path, source))?,
        );
        let mut effect_visible = false;
        let publish = (|| {
            file.write_all(bytes).map_err(|source| {
                crate::errors::io_error("write staging file", &temporary_path, source)
            })?;
            fchmod(&file, admitted_mode)
                .map_err(|source| errno_error("set staging file mode", &temporary_path, source))?;
            sync_file(&file, class, self)?;
            drop(file);

            if clobber {
                renameat(
                    &parent.directory,
                    temporary.as_str(),
                    &parent.directory,
                    parent.leaf.as_str(),
                )
                .map_err(|source| {
                    errno_error("publish staged replacement", self.display_path(), source)
                })?;
                effect_visible = true;
            } else {
                linkat(
                    &parent.directory,
                    temporary.as_str(),
                    &parent.directory,
                    parent.leaf.as_str(),
                    AtFlags::empty(),
                )
                .map_err(|source| {
                    if source == Errno::EXIST {
                        FsError::Collision(
                            self.relative().to_string(),
                            "target already exists (no-clobber)".into(),
                        )
                    } else {
                        errno_error(
                            "publish staged no-clobber file",
                            self.display_path(),
                            source,
                        )
                    }
                })?;
                effect_visible = true;
                cleanup_temporary(
                    &parent.directory,
                    &temporary,
                    &temporary_path,
                    staging_identity,
                    false,
                )?;
            }
            fsync(&parent.directory).map_err(|source| {
                errno_error("synchronize published file parent", &parent_path, source)
            })
        })();

        match publish {
            Ok(()) => Ok(()),
            Err(source) => {
                let source = if effect_visible {
                    effect_uncertain("publish confined file", self.display_path(), source)
                } else {
                    source
                };
                match cleanup_temporary(
                    &parent.directory,
                    &temporary,
                    &temporary_path,
                    staging_identity,
                    true,
                ) {
                    Ok(()) => Err(source),
                    Err(cleanup) => Err(FsError::CleanupFailed {
                        operation: "publish confined file",
                        path: self.display_path(),
                        source: Box::new(source),
                        cleanup: Box::new(cleanup),
                    }),
                }
            }
        }
    }

    #[cfg(not(unix))]
    fn staged_write(
        &self,
        bytes: &[u8],
        mode: u32,
        clobber: bool,
        class: PublicationClass,
    ) -> Result<(), FsError> {
        let _ = (self, bytes, mode, clobber, class);
        Err(unsupported("publish confined file"))
    }
}

#[cfg(unix)]
fn cleanup_temporary(
    parent: &File,
    temporary: &str,
    temporary_path: &std::path::Path,
    expected: StagingIdentity,
    sync_parent: bool,
) -> Result<(), FsError> {
    use rustix::fs::{fsync, statat, unlinkat, AtFlags, FileType};
    use rustix::io::Errno;

    let metadata = match statat(parent, temporary, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(metadata) => metadata,
        Err(Errno::NOENT) => {
            if sync_parent {
                fsync(parent).map_err(|source| {
                    errno_error(
                        "synchronize staging cleanup parent",
                        temporary_path.parent().unwrap_or(temporary_path),
                        source,
                    )
                })?;
            }
            return Ok(());
        }
        Err(source) => {
            return Err(errno_error(
                "inspect staging file for cleanup",
                temporary_path,
                source,
            ));
        }
    };
    if !FileType::from_raw_mode(metadata.st_mode).is_file()
        || metadata.st_dev != expected.0.st_dev
        || metadata.st_ino != expected.0.st_ino
    {
        return Err(FsError::Collision(
            temporary_path.display().to_string(),
            "staging name no longer identifies the file created by this operation".into(),
        ));
    }
    match unlinkat(parent, temporary, AtFlags::empty()) {
        Ok(()) | Err(Errno::NOENT) => {
            if sync_parent {
                fsync(parent).map_err(|source| {
                    errno_error(
                        "synchronize staging cleanup parent",
                        temporary_path.parent().unwrap_or(temporary_path),
                        source,
                    )
                })?;
            }
            Ok(())
        }
        Err(source) => Err(errno_error("remove staging file", temporary_path, source)),
    }
}

#[cfg(all(test, unix))]
#[path = "../../tests/internal/confine/write.rs"]
mod tests;
