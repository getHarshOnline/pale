//! Admission and creation policy for persistent advisory-lock entries.

use std::fs::File;

use super::ConfinedLocation;
#[cfg(not(unix))]
use crate::errors::unsupported;
use crate::errors::FsError;
#[cfg(unix)]
use crate::errors::{errno_error, invariant};
use crate::lock::{AdvisoryLockEffect, AdvisoryLockFailure};

impl ConfinedLocation {
    /// Open or exclusively create the empty owner-only regular file used as a lock.
    /// Existing entries are validated, never repaired or implicitly adopted.
    #[cfg(unix)]
    pub(crate) fn open_lock_file(&self) -> Result<(File, AdvisoryLockEffect), AdvisoryLockFailure> {
        use rustix::fs::{fchmod, openat, Mode, OFlags};
        use rustix::io::Errno;
        use std::os::unix::fs::MetadataExt;

        let parent_created = self.ensure_parent_observed().map_err(|(source, visible)| {
            AdvisoryLockFailure::new(
                source,
                if visible {
                    AdvisoryLockEffect::OutcomeUnknown
                } else {
                    AdvisoryLockEffect::Untouched
                },
            )
        })?;
        let mut effect = if parent_created {
            AdvisoryLockEffect::Confirmed
        } else {
            AdvisoryLockEffect::Untouched
        };
        for attempt in 0..2 {
            let Some(parent) = self
                .parent()
                .map_err(|source| AdvisoryLockFailure::new(source, effect))?
            else {
                if attempt == 0 {
                    continue;
                }
                return Err(AdvisoryLockFailure::new(
                    invariant(
                        "open confined lock",
                        self.display_path(),
                        "lock parent is missing",
                    ),
                    effect,
                ));
            };
            let (file, created) = match openat(
                &parent.directory,
                parent.leaf.as_str(),
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::from_raw_mode(0o600),
            ) {
                Ok(fd) => (File::from(fd), true),
                Err(Errno::EXIST) => {
                    let fd = openat(
                        &parent.directory,
                        parent.leaf.as_str(),
                        OFlags::WRONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                        Mode::empty(),
                    )
                    .map_err(|source| {
                        if source == Errno::LOOP {
                            FsError::Collision(
                                self.relative.clone(),
                                "lock is a symbolic link".into(),
                            )
                        } else {
                            errno_error("open existing confined lock", self.display_path(), source)
                        }
                    })
                    .map_err(|source| AdvisoryLockFailure::new(source, effect))?;
                    (File::from(fd), false)
                }
                Err(Errno::NOENT) if attempt == 0 => continue,
                Err(source) => {
                    return Err(AdvisoryLockFailure::new(
                        errno_error(
                            "open confined lock without following links",
                            self.display_path(),
                            source,
                        ),
                        effect,
                    ));
                }
            };
            if created {
                effect = AdvisoryLockEffect::Confirmed;
            }
            let metadata = file
                .metadata()
                .map_err(|source| {
                    crate::errors::io_error("inspect confined lock", self.display_path(), source)
                })
                .map_err(|source| {
                    AdvisoryLockFailure::new(
                        source,
                        if created {
                            AdvisoryLockEffect::OutcomeUnknown
                        } else {
                            effect
                        },
                    )
                })?;
            if !metadata.is_file() || metadata.len() != 0 {
                return Err(AdvisoryLockFailure::new(
                    FsError::Collision(
                        self.relative.clone(),
                        "lock must be an empty regular file".into(),
                    ),
                    effect,
                ));
            }
            let mode = metadata.mode() & 0o7777;
            if created {
                fchmod(&file, Mode::from_raw_mode(0o600))
                    .map_err(|source| {
                        errno_error("set new confined lock mode", self.display_path(), source)
                    })
                    .map_err(|source| {
                        AdvisoryLockFailure::new(source, AdvisoryLockEffect::OutcomeUnknown)
                    })?;
            } else if mode != 0o600 {
                return Err(AdvisoryLockFailure::new(
                    FsError::Collision(
                        self.relative.clone(),
                        format!("existing lock mode is {mode:#o}; expected 0o600"),
                    ),
                    effect,
                ));
            }
            return Ok((file, effect));
        }
        Err(AdvisoryLockFailure::new(
            invariant(
                "open confined lock",
                self.display_path(),
                "lock namespace did not stabilize",
            ),
            effect,
        ))
    }

    #[cfg(not(unix))]
    /// Fail closed where descriptor-relative lock admission is unavailable.
    pub(crate) fn open_lock_file(&self) -> Result<(File, AdvisoryLockEffect), AdvisoryLockFailure> {
        Err(AdvisoryLockFailure::new(
            unsupported("open confined lock"),
            AdvisoryLockEffect::Untouched,
        ))
    }
}
