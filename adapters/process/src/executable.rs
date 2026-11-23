//! Retained filesystem identity for every machine-bound executable.

use std::fs::{self, File};
use std::io::{Seek, SeekFrom};
use std::path::{Path, PathBuf};

use pale_core::records::ExecutableDigest;
use pale_ports::ExecutableIdentityError as ExecutableError;

#[cfg(unix)]
use rtepl_filesystem::unix::{open_readonly_nofollow, FileIdentity};

#[cfg(unix)]
fn file_identity(file: &File) -> Result<FileIdentity, ExecutableError> {
    let metadata = file
        .metadata()
        .map_err(|_| ExecutableError::NotFound("retained executable".into()))?;
    Ok(FileIdentity::from(&metadata))
}

#[cfg(unix)]
fn open_nofollow(path: &Path) -> Result<File, ExecutableError> {
    open_readonly_nofollow(path, false)
        .map_err(|_| ExecutableError::NotFound(path.display().to_string()))
}

#[cfg(unix)]
fn hash_file(file: &mut File, path: &Path) -> Result<ExecutableDigest, ExecutableError> {
    file.seek(SeekFrom::Start(0))
        .map_err(|_| ExecutableError::NotFound(path.display().to_string()))?;
    let digest = ExecutableDigest::hash_reader(&mut *file)
        .map_err(|_| ExecutableError::NotFound(path.display().to_string()))?;
    file.seek(SeekFrom::Start(0))
        .map_err(|_| ExecutableError::NotFound(path.display().to_string()))?;
    Ok(digest)
}

/// Retained descriptor and exact identity admitted for one bounded probe.
pub(super) struct VerifiedExecutable {
    path: PathBuf,
    digest: ExecutableDigest,
    #[cfg(unix)]
    identity: FileIdentity,
    file: File,
}

impl VerifiedExecutable {
    /// Retain an absolute no-follow executable and verify its exact digest and identity.
    pub(super) fn admit(
        executable: &str,
        expected_digest: &ExecutableDigest,
    ) -> Result<Self, ExecutableError> {
        let path = PathBuf::from(executable);
        if !path.is_absolute() {
            return Err(ExecutableError::NotAbsolute(executable.to_string()));
        }

        #[cfg(not(unix))]
        {
            let _ = expected_digest;
            return Err(ExecutableError::UnsupportedPlatform {
                platform: std::env::consts::OS,
            });
        }

        #[cfg(unix)]
        {
            let path_metadata = fs::symlink_metadata(&path)
                .map_err(|_| ExecutableError::NotFound(path.display().to_string()))?;
            if path_metadata.file_type().is_symlink() {
                return Err(ExecutableError::Symlink(path.display().to_string()));
            }
            let mut file = open_nofollow(&path)?;
            let metadata = file
                .metadata()
                .map_err(|_| ExecutableError::NotFound(path.display().to_string()))?;
            if !metadata.is_file() {
                return Err(ExecutableError::NotRegular(path.display().to_string()));
            }
            let identity = file_identity(&file)?;
            if identity.mode & 0o111 == 0 {
                return Err(ExecutableError::NotExecutable(path.display().to_string()));
            }
            let actual = hash_file(&mut file, &path)?;
            if &actual != expected_digest {
                return Err(ExecutableError::DigestMismatch {
                    actual,
                    expected: expected_digest.clone(),
                });
            }
            let mut admitted = Self {
                path,
                digest: expected_digest.clone(),
                identity,
                file,
            };
            admitted.revalidate()?;
            Ok(admitted)
        }
    }

    /// Borrow the admitted pathname used at the process-launch boundary.
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    /// Resolve the canonical pathname only while it still names the admitted file.
    pub(super) fn canonical_path(&mut self) -> Result<PathBuf, ExecutableError> {
        #[cfg(not(unix))]
        {
            return Err(ExecutableError::UnsupportedPlatform {
                platform: std::env::consts::OS,
            });
        }

        #[cfg(unix)]
        {
            self.revalidate()?;
            let changed = || ExecutableError::IdentityChanged(self.path.display().to_string());
            let canonical = std::fs::canonicalize(&self.path).map_err(|_| changed())?;
            let mut canonical_file = open_nofollow(&canonical).map_err(|_| changed())?;
            if file_identity(&canonical_file).map_err(|_| changed())? != self.identity
                || hash_file(&mut canonical_file, &canonical).map_err(|_| changed())? != self.digest
            {
                return Err(changed());
            }
            self.revalidate()?;
            Ok(canonical)
        }
    }

    /// Compare both the retained descriptor and current pathname to admission.
    pub(super) fn revalidate(&mut self) -> Result<(), ExecutableError> {
        #[cfg(not(unix))]
        {
            return Err(ExecutableError::UnsupportedPlatform {
                platform: std::env::consts::OS,
            });
        }

        #[cfg(unix)]
        {
            let changed = || ExecutableError::IdentityChanged(self.path.display().to_string());
            let retained_identity = file_identity(&self.file).map_err(|_| changed())?;
            let retained_digest = hash_file(&mut self.file, &self.path).map_err(|_| changed())?;
            let mut current = open_nofollow(&self.path).map_err(|_| changed())?;
            let current_identity = file_identity(&current).map_err(|_| changed())?;
            let current_digest = hash_file(&mut current, &self.path).map_err(|_| changed())?;
            if retained_identity != self.identity
                || current_identity != self.identity
                || retained_digest != self.digest
                || current_digest != self.digest
            {
                return Err(changed());
            }
            Ok(())
        }
    }
}

#[cfg(all(test, unix))]
#[path = "../tests/internal/executable/unix.rs"]
mod unix_tests;

#[cfg(all(test, not(unix)))]
#[path = "../tests/internal/executable/unsupported.rs"]
mod unsupported_platform_tests;
