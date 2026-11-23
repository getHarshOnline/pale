//! Retained descriptor and identity checks for source-observation roots.

use std::fs::{self, File};
use std::path::{Path, PathBuf};

use pale_ports::SourceFilesystemError;

/// One opened directory capability plus its canonical diagnostic coordinate.
pub(super) struct OpenedSourceRoot {
    requested: PathBuf,
    canonical: PathBuf,
    directory: File,
}

impl OpenedSourceRoot {
    /// Open a non-symlink directory leaf and bind it to its canonical identity.
    pub(super) fn open(requested: &Path) -> Result<Self, SourceFilesystemError> {
        let metadata = fs::symlink_metadata(requested).map_err(|source| io(requested, source))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(SourceFilesystemError::InvalidEntry {
                path: requested.to_path_buf(),
            });
        }
        let canonical = fs::canonicalize(requested).map_err(|source| io(requested, source))?;
        let directory = open_directory(requested)?;
        let root = Self {
            requested: requested.to_path_buf(),
            canonical,
            directory,
        };
        root.verify()?;
        Ok(root)
    }

    /// Borrow the retained directory descriptor for descendant traversal.
    pub(super) fn directory(&self) -> &File {
        &self.directory
    }

    /// Reopen the canonical pathname and compare it with the retained handle.
    pub(super) fn verify(&self) -> Result<(), SourceFilesystemError> {
        use std::os::unix::fs::MetadataExt;

        let current = open_directory(&self.canonical)?;
        let retained = self
            .directory
            .metadata()
            .map_err(|source| io(&self.requested, source))?;
        let current = current
            .metadata()
            .map_err(|source| io(&self.requested, source))?;
        if retained.dev() != current.dev() || retained.ino() != current.ino() {
            return Err(SourceFilesystemError::InvalidEntry {
                path: self.requested.clone(),
            });
        }
        Ok(())
    }
}

fn open_directory(path: &Path) -> Result<File, SourceFilesystemError> {
    use rustix::fs::{open, Mode, OFlags};

    open(
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map(File::from)
    .map_err(|source| {
        io(
            path,
            std::io::Error::from_raw_os_error(source.raw_os_error()),
        )
    })
}

fn io(path: &Path, source: std::io::Error) -> SourceFilesystemError {
    SourceFilesystemError::Io {
        path: path.to_path_buf(),
        source,
    }
}
