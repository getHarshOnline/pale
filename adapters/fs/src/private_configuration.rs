//! Shared bounded capture for explicit owner-private native configuration.

use crate::ReadLimit;
use rustix::fs::{open, openat, Mode, OFlags};
use std::{
    fs::{File, Metadata},
    io::Read,
    os::unix::fs::MetadataExt,
    path::{Component, Path, PathBuf},
};

/// Redacted refusal while capturing owner-private configuration.
#[derive(Debug, thiserror::Error)]
pub enum PrivateConfigurationError {
    /// Absolute normal path syntax or retained-directory identity failed.
    #[error("private configuration path is invalid or changed")]
    Path,
    /// Native no-follow traversal, read or metadata inspection failed.
    #[error("private configuration could not be captured")]
    Io,
    /// The effective-UID, POSIX mode, regular-file or link-count rule failed.
    #[error("private configuration is not owner-private")]
    Protection,
    /// The read exceeded its byte bound or file metadata changed during capture.
    #[error("private configuration exceeded its bound or changed during capture")]
    Content,
}

/// One retained owner-private parent directory, opened without following links.
///
/// These checks inspect POSIX ownership and mode, not ACLs, same-UID attackers
/// or authority over the configuration's meaning. No directory is created or
/// repaired and no metadata grants permission to perform a configured effect.
pub struct OwnerPrivateDirectory {
    directory: File,
}

impl OwnerPrivateDirectory {
    /// Open every parent component without following links and retain the
    /// immediate parent only when owned by the effective UID with no group or
    /// other mode bits. Returns the exact normal leaf separately.
    ///
    /// # Errors
    /// Refuses relative/non-normal paths, symlinks, native failures or an
    /// immediate parent that does not satisfy the owner-private rule.
    pub fn parent_of(path: &Path) -> Result<(Self, PathBuf), PrivateConfigurationError> {
        let mut normalized = PathBuf::new();
        let mut segments = Vec::new();
        for component in path.components() {
            match component {
                Component::RootDir => normalized.push(Path::new("/")),
                Component::Normal(segment) => {
                    normalized.push(segment);
                    segments.push(segment.to_os_string());
                }
                _ => return Err(PrivateConfigurationError::Path),
            }
        }
        if !path.is_absolute() || normalized.as_os_str() != path.as_os_str() {
            return Err(PrivateConfigurationError::Path);
        }
        let leaf = segments.pop().ok_or(PrivateConfigurationError::Path)?;
        let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let mut directory =
            File::from(open("/", flags, Mode::empty()).map_err(|_| PrivateConfigurationError::Io)?);
        for segment in segments {
            directory = File::from(
                openat(&directory, segment, flags, Mode::empty())
                    .map_err(|_| PrivateConfigurationError::Io)?,
            );
        }
        require_private_directory(&directory)?;
        Ok((Self { directory }, PathBuf::from(leaf)))
    }

    /// Reopen an explicit path's parent and compare it with the retained native
    /// directory after rechecking both directories' ownership and mode.
    ///
    /// # Errors
    /// Refuses unsafe traversal, protection changes or a replacement directory.
    pub fn verify_parent_of(&self, path: &Path) -> Result<(), PrivateConfigurationError> {
        let (current, _) = Self::parent_of(path)?;
        require_private_directory(&self.directory)?;
        let retained = self
            .directory
            .metadata()
            .map_err(|_| PrivateConfigurationError::Io)?;
        let current = current
            .directory
            .metadata()
            .map_err(|_| PrivateConfigurationError::Io)?;
        if retained.dev() != current.dev() || retained.ino() != current.ino() {
            return Err(PrivateConfigurationError::Path);
        }
        Ok(())
    }
}

/// Capture one regular, single-link `0600` file under an owner-private parent.
///
/// Path traversal never follows links. Both initial metadata and consumed bytes
/// obey `limit`; protection and length/modification/change metadata are checked
/// again after capture. This is a captured byte view, not a live path lease or
/// proof that another same-UID process cannot change the source afterwards.
/// No configuration parsing, lookup fallback, writes or permission repair occur.
///
/// # Errors
/// Returns a redacted path, I/O, protection, change or byte-bound refusal.
pub fn capture_private_configuration(
    path: &Path,
    limit: ReadLimit,
) -> Result<Vec<u8>, PrivateConfigurationError> {
    let maximum =
        u64::try_from(limit.max_bytes()).map_err(|_| PrivateConfigurationError::Content)?;
    let read_ceiling = maximum
        .checked_add(1)
        .ok_or(PrivateConfigurationError::Content)?;
    let (parent, leaf) = OwnerPrivateDirectory::parent_of(path)?;
    let mut file = File::from(
        openat(
            &parent.directory,
            leaf.as_os_str(),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| PrivateConfigurationError::Io)?,
    );
    let before = file.metadata().map_err(|_| PrivateConfigurationError::Io)?;
    require_private_file(&before)?;
    if before.len() > maximum {
        return Err(PrivateConfigurationError::Content);
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(read_ceiling)
        .read_to_end(&mut bytes)
        .map_err(|_| PrivateConfigurationError::Io)?;
    let after = file.metadata().map_err(|_| PrivateConfigurationError::Io)?;
    require_private_file(&after)?;
    if bytes.len() > limit.max_bytes()
        || before.len() != after.len()
        || before.mtime() != after.mtime()
        || before.mtime_nsec() != after.mtime_nsec()
        || before.ctime() != after.ctime()
        || before.ctime_nsec() != after.ctime_nsec()
    {
        return Err(PrivateConfigurationError::Content);
    }
    Ok(bytes)
}

fn require_private_directory(directory: &File) -> Result<(), PrivateConfigurationError> {
    let meta = directory
        .metadata()
        .map_err(|_| PrivateConfigurationError::Io)?;
    if !meta.is_dir()
        || meta.uid() != rustix::process::geteuid().as_raw()
        || meta.mode() & 0o077 != 0
    {
        return Err(PrivateConfigurationError::Protection);
    }
    Ok(())
}

fn require_private_file(meta: &Metadata) -> Result<(), PrivateConfigurationError> {
    if !meta.is_file()
        || meta.uid() != rustix::process::geteuid().as_raw()
        || meta.mode() & 0o777 != 0o600
        || meta.nlink() != 1
    {
        return Err(PrivateConfigurationError::Protection);
    }
    Ok(())
}
