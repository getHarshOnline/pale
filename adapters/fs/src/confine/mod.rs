//! Descriptor-relative confinement for target-local filesystem effects.
//!
//! Parsing establishes only that a path is repository-relative. On Unix, a
//! [`ConfinedRoot`] additionally retains an open directory descriptor and resolves
//! every descendant from it one component at a time with no-follow semantics.
//! Non-Unix targets are outside this adapter's current support envelope and fail
//! closed when opening a root.

mod directory;
mod limits;
mod location;
mod lock_file;
mod root;
mod traversal;
mod unlink;
mod write;

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::path::{Path, PathBuf};

pub use directory::{ConfinedDirectoryEntry, ConfinedEntryKind, ConfinedTreeEntry};
pub use limits::{DirectoryLimit, ReadLimit, TraversalLimit, MATERIALIZED_SURFACE_READ_LIMIT};
pub(crate) use location::FileIdentity;
pub use location::{ExistingFile, ExistingFileMetadata};
pub use root::{ConfinedLocation, ConfinedRoot};
pub(crate) use traversal::CreatedDirectory;

#[cfg(unix)]
use crate::errors::{io_error, FsError};

#[cfg(unix)]
fn mode_of(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o7777
}

#[cfg(unix)]
fn canonical_root(path: &Path) -> Result<PathBuf, FsError> {
    fs::canonicalize(path).map_err(|source| io_error("canonicalize root", path, source))
}
