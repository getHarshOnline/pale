//! Atomic, identity-bound reservation of one canonical `SQLite` candidate.

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};
use rustix::fs::{AtFlags, FileType, Mode, OFlags};
use rustix::io::Errno;

use crate::location::{private_store_file, ControlStoreLocation, FileIdentity};
use crate::ControlSqliteError;

/// Retained parent capability and the main file created by this invocation.
///
/// The adapter never claims ownership of WAL or shared-memory sidecars. They
/// are rejected when present before reservation, but a later observation alone
/// cannot prove which same-principal process created them. Failure cleanup is
/// therefore deliberately limited to the identity-bound main file.
pub(crate) struct CandidateReservation {
    path: PathBuf,
    parent_path: PathBuf,
    leaf: OsString,
    parent_identity: FileIdentity,
    main_identity: FileIdentity,
    canonical_location: ControlStoreLocation,
    directory: File,
    reservation: File,
}

/// Result of attempting to remove the exact main-file identity reserved here.
enum CandidateRemoval {
    /// The reserved main file was removed or was already absent.
    RemovedOrAbsent,
    /// The pathname names bytes not owned by this reservation.
    ForeignPreserved,
}

impl CandidateReservation {
    /// Reserve the exact leaf derived from a retained runtime-root capability.
    pub(crate) fn reserve_store(
        location: &ControlStoreLocation,
    ) -> Result<Self, ControlSqliteError> {
        location.verify_parent_path_identity()?;
        let path = location.path();
        let parent_path = candidate_parent(&path);
        let leaf = location.leaf().to_owned();
        let directory = location.parent().try_clone().map_err(|source| {
            ControlSqliteError::io("clone canonical candidate parent", &parent_path, source)
        })?;
        Self::reserve_under(path, parent_path, leaf, directory, location.clone())
    }

    /// Perform one reservation beneath an already-retained private parent.
    fn reserve_under(
        path: PathBuf,
        parent_path: PathBuf,
        leaf: OsString,
        directory: File,
        canonical_location: ControlStoreLocation,
    ) -> Result<Self, ControlSqliteError> {
        let parent_metadata = directory.metadata().map_err(|source| {
            ControlSqliteError::io("inspect retained candidate parent", &parent_path, source)
        })?;
        let parent_identity = FileIdentity::from_metadata(&parent_metadata);

        ensure_absent(&directory, &path, &leaf)?;
        let descriptor = match rustix::fs::openat(
            &directory,
            &leaf,
            OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o600),
        ) {
            Ok(descriptor) => descriptor,
            Err(Errno::EXIST) => return Err(ControlSqliteError::InitializationTargetExists(path)),
            Err(source) => return Err(errno_error("reserve SQLite candidate", path, source)),
        };
        let reservation = File::from(descriptor);
        let main_metadata = reservation.metadata().map_err(|source| {
            ControlSqliteError::io("inspect reserved candidate", &path, source)
        })?;
        let candidate = Self {
            path,
            parent_path,
            leaf,
            parent_identity,
            main_identity: FileIdentity::from_metadata(&main_metadata),
            canonical_location,
            directory,
            reservation,
        };

        if !main_metadata.is_file() || main_metadata.uid() != rustix::process::geteuid().as_raw() {
            let path = candidate.path.clone();
            return candidate.finish(Err(ControlSqliteError::CandidateOwnershipLost(path)));
        }
        if let Err(source) = rustix::fs::fchmod(&candidate.reservation, Mode::from_raw_mode(0o600))
        {
            let path = candidate.path.clone();
            return candidate.finish(Err(errno_error(
                "set private candidate permissions",
                path,
                source,
            )));
        }
        match candidate.reservation.metadata() {
            Ok(metadata)
                if metadata.uid() == rustix::process::geteuid().as_raw()
                    && metadata.permissions().mode() & 0o777 == 0o600 => {}
            Ok(_) => {
                let path = candidate.path.clone();
                return candidate.finish(Err(ControlSqliteError::CandidateOwnershipLost(path)));
            }
            Err(source) => {
                let path = candidate.path.clone();
                return candidate.finish(Err(ControlSqliteError::io(
                    "verify private candidate permissions",
                    path,
                    source,
                )));
            }
        }
        for (sidecar_leaf, sidecar_path) in
            sidecar_leaves_and_paths(candidate.path(), &candidate.leaf)
        {
            match rustix::fs::statat(
                &candidate.directory,
                &sidecar_leaf,
                AtFlags::SYMLINK_NOFOLLOW,
            ) {
                Ok(_) => {
                    return candidate.finish(Err(ControlSqliteError::InitializationTargetExists(
                        sidecar_path,
                    )));
                }
                Err(Errno::NOENT) => {}
                Err(source) => {
                    return candidate.finish(Err(errno_error(
                        "inspect initialization target",
                        sidecar_path,
                        source,
                    )));
                }
            }
        }
        Ok(candidate)
    }

    /// Borrow the canonical diagnostic pathname retained from the location capability.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Open the reserved file through `SQLite` without granting create authority.
    ///
    /// `SQLite` accepts a pathname rather than the retained directory descriptor.
    /// The immediate parent and leaf are therefore verified immediately before
    /// and after the open, but a same-UID actor can still perform an A/B/restore
    /// substitution entirely between those checks. The returned connection is
    /// structural bootstrap evidence, not a descriptor-identity or admission
    /// proof. The retained descriptors still own creation and cleanup.
    pub(crate) fn open_sqlite(
        &self,
        operation: &'static str,
    ) -> Result<Connection, ControlSqliteError> {
        self.verify_path_identity(operation)?;
        let connection = Connection::open_with_flags(
            self.path(),
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|source| ControlSqliteError::sqlite(operation, source))?;
        if let Err(error) = self.verify_path_identity(operation) {
            drop(connection);
            return Err(error);
        }
        Ok(connection)
    }

    /// Seal a successful candidate or clean its identity-bound main file on failure.
    ///
    /// Success requires the diagnostic parent and leaf to retain their captured
    /// identities and synchronizes the retained parent directory. Cleanup never
    /// removes a sidecar and removes the main file only after observing the
    /// reservation's identity. The documented same-UID observation-to-unlink
    /// race remains; this is not universal replacement exclusion.
    pub(crate) fn finish<T>(
        self,
        outcome: Result<T, ControlSqliteError>,
    ) -> Result<T, ControlSqliteError> {
        match outcome {
            Ok(value) => {
                let sealed = self.seal_identity_and_parent();
                match sealed {
                    Ok(()) => Ok(value),
                    Err(failure) => self.cleanup(failure),
                }
            }
            Err(failure) => self.cleanup(failure),
        }
    }

    /// Recheck the candidate identity and synchronize its retained parent.
    ///
    /// This leaves the reservation owned by the caller so a later acceptance
    /// check can still discard the main file without reconstructing custody.
    pub(crate) fn seal_identity_and_parent(&self) -> Result<(), ControlSqliteError> {
        self.verify_path_identity("seal SQLite candidate identity")?;
        rustix::fs::fsync(&self.directory).map_err(|source| {
            errno_error(
                "synchronize SQLite candidate parent",
                &self.parent_path,
                source,
            )
        })
    }

    /// Recheck the canonical parent capability and compare parent and main identities.
    fn verify_path_identity(&self, operation: &'static str) -> Result<(), ControlSqliteError> {
        let current_parent = {
            self.canonical_location
                .verify_parent_path_identity()
                .map_err(|_| {
                    ControlSqliteError::CandidateOwnershipLost(self.parent_path.clone())
                })?;
            self.canonical_location
                .parent()
                .try_clone()
                .map_err(|source| ControlSqliteError::io(operation, &self.parent_path, source))?
        };
        let current_parent_metadata = current_parent
            .metadata()
            .map_err(|source| ControlSqliteError::io(operation, &self.parent_path, source))?;
        if FileIdentity::from_metadata(&current_parent_metadata) != self.parent_identity {
            return Err(ControlSqliteError::CandidateOwnershipLost(
                self.parent_path.clone(),
            ));
        }
        let current_main =
            rustix::fs::statat(&current_parent, &self.leaf, AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|source| errno_error(operation, self.path(), source))?;
        let main_matches = self.main_identity.matches_stat(&current_main)?;
        if !private_store_file(&current_main) || !main_matches {
            return Err(ControlSqliteError::CandidateOwnershipLost(
                self.path.clone(),
            ));
        }
        Ok(())
    }

    /// Remove only the main identity created by [`Self::reserve_store`].
    fn cleanup<T>(self, failure: ControlSqliteError) -> Result<T, ControlSqliteError> {
        let operation = "clean failed store initialization";
        let path = self.path.clone();
        match self.remove_owned_main(operation) {
            Ok(CandidateRemoval::RemovedOrAbsent | CandidateRemoval::ForeignPreserved) => {
                Err(failure)
            }
            Err(ControlSqliteError::Io { source, .. }) => {
                Err(ControlSqliteError::CandidateCleanupFailed {
                    operation,
                    path,
                    failure: Box::new(failure),
                    cleanup: source,
                })
            }
            Err(other) => Err(other),
        }
    }

    /// Remove the reserved main file without touching an observed replacement.
    fn remove_owned_main(
        self,
        operation: &'static str,
    ) -> Result<CandidateRemoval, ControlSqliteError> {
        let path = self.path.clone();
        drop(self.reservation);

        let current =
            match rustix::fs::statat(&self.directory, &self.leaf, AtFlags::SYMLINK_NOFOLLOW) {
                Ok(current) => current,
                Err(Errno::NOENT) => return Ok(CandidateRemoval::RemovedOrAbsent),
                Err(source) => return Err(errno_error(operation, path, source)),
            };
        let main_matches = self.main_identity.matches_stat(&current)?;
        if !FileType::from_raw_mode(current.st_mode).is_file() || !main_matches {
            return Ok(CandidateRemoval::ForeignPreserved);
        }
        match rustix::fs::unlinkat(&self.directory, &self.leaf, AtFlags::empty()) {
            Ok(()) | Err(Errno::NOENT) => {}
            Err(source) => return Err(errno_error(operation, path, source)),
        }
        rustix::fs::fsync(&self.directory).map_err(|source| {
            errno_error(
                "synchronize cleaned SQLite candidate parent",
                &self.parent_path,
                source,
            )
        })?;
        Ok(CandidateRemoval::RemovedOrAbsent)
    }
}

/// Resolve the immediate parent, treating a bare filename as relative to `.`.
fn candidate_parent(path: &Path) -> PathBuf {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf()
}

/// Reject an existing main, WAL, or shared-memory leaf without following links.
fn ensure_absent(directory: &File, path: &Path, leaf: &OsStr) -> Result<(), ControlSqliteError> {
    let candidates = std::iter::once((leaf.to_owned(), path.to_path_buf()))
        .chain(sidecar_leaves_and_paths(path, leaf));
    for (candidate_leaf, candidate_path) in candidates {
        match rustix::fs::statat(directory, &candidate_leaf, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(_) => {
                return Err(ControlSqliteError::InitializationTargetExists(
                    candidate_path,
                ));
            }
            Err(Errno::NOENT) => {}
            Err(source) => {
                return Err(errno_error(
                    "inspect initialization target",
                    candidate_path,
                    source,
                ));
            }
        }
    }
    Ok(())
}

/// Pair descriptor-relative sidecar leaves with their diagnostic paths.
fn sidecar_leaves_and_paths(path: &Path, leaf: &OsStr) -> [(OsString, PathBuf); 2] {
    ["-wal", "-shm"].map(|suffix| {
        let mut sidecar_leaf = leaf.to_owned();
        sidecar_leaf.push(suffix);
        let sidecar_path = sqlite_sidecar(path, suffix);
        (sidecar_leaf, sidecar_path)
    })
}

/// Append a `SQLite` sidecar suffix without requiring Unicode path conversion.
pub(crate) fn sqlite_sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut path = path.as_os_str().to_owned();
    path.push(suffix);
    PathBuf::from(path)
}

/// Attach stable operation and path context to a Unix error number.
fn errno_error(
    operation: &'static str,
    path: impl Into<PathBuf>,
    source: Errno,
) -> ControlSqliteError {
    ControlSqliteError::io(operation, path, io_error(source))
}

/// Preserve an exact Unix error number in the standard I/O source chain.
fn io_error(source: Errno) -> std::io::Error {
    std::io::Error::from_raw_os_error(source.raw_os_error())
}

#[cfg(test)]
#[path = "../tests/internal/candidate_reservation.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/support/candidate.rs"]
mod candidate_test_support;
