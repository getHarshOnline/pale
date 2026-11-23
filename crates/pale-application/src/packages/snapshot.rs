//! Immutable, bounded retained view of one Package distribution source tree.
//!
//! Capture rejects symlinks and special files, retains exact bytes in lexical
//! maps, and bounds every descendant, directory depth, individual file, and total
//! bytes. Unix traversal stays relative to one retained source-root descriptor;
//! compilation thereafter uses only the retained bytes.

use std::path::{Path, PathBuf};

use pale_ports::{SourceFilesystemPort, SourceTreeLimits, SourceTreeSnapshot};

use super::PackageError;

/// Maximum descendants admitted into one retained source tree.
pub(crate) const MAX_ENTRIES: usize = 8_192;
/// Maximum directory depth admitted beneath the retained source root.
pub(crate) const MAX_DEPTH: usize = 24;
/// Maximum bytes retained from one distribution source file.
pub(crate) const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
/// Maximum aggregate bytes retained from one distribution snapshot.
pub(crate) const MAX_TOTAL_BYTES: u64 = 64 * 1024 * 1024;

/// Production source-tree ceiling composed from the four owning bounds.
///
/// Every capture applies [`MAX_ENTRIES`], [`MAX_DEPTH`], [`MAX_FILE_BYTES`], and
/// [`MAX_TOTAL_BYTES`] as one indivisible admission policy.
const PRODUCTION_LIMITS: SourceTreeLimits =
    SourceTreeLimits::new(MAX_ENTRIES, MAX_DEPTH, MAX_FILE_BYTES, MAX_TOTAL_BYTES);

/// Exact source-tree bytes observed and retained for one compilation or inspection.
///
/// Relative paths index immutable in-memory bytes. The requested root is kept
/// only for diagnostics and is never consulted by snapshot reads. Capture does
/// not claim a filesystem-wide atomic point in time across independently mutable
/// files; all later consumers do share the exact retained byte set.
pub(crate) struct DistributionSnapshot {
    retained: SourceTreeSnapshot,
}

impl std::fmt::Debug for DistributionSnapshot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DistributionSnapshot")
            .field("directories", &self.retained.directory_count())
            .field("files", &self.retained.file_count())
            .field("retained_bytes", &self.retained.retained_bytes())
            .finish()
    }
}

impl DistributionSnapshot {
    /// Adopt a retained tree after the caller has enforced [`PRODUCTION_LIMITS`].
    ///
    /// This internal constructor performs no second limit pass; its only caller
    /// is the path-free captured ingress that applies the same four ceilings.
    pub(crate) fn from_retained(retained: SourceTreeSnapshot) -> Self {
        Self { retained }
    }

    /// Capture `<distribution_root>/packages` into bounded retained bytes.
    ///
    /// The root itself must be a real directory. Any symlink, special entry, or
    /// configured size/count/depth excess rejects the entire snapshot. Platforms
    /// without descriptor-relative no-follow traversal fail before reading source.
    ///
    /// # Errors
    ///
    /// Returns [`PackageError`] when the package root cannot be opened safely,
    /// contains an inadmissible entry, or exceeds a configured capture bound.
    pub(crate) fn capture(
        sources: &dyn SourceFilesystemPort,
        distribution_root: &Path,
    ) -> Result<Self, PackageError> {
        Self::capture_root(sources, &distribution_root.join("packages"))
    }

    fn capture_root(
        sources: &dyn SourceFilesystemPort,
        requested_root: &Path,
    ) -> Result<Self, PackageError> {
        Self::capture_root_with_limits(sources, requested_root, PRODUCTION_LIMITS)
    }

    fn capture_root_with_limits(
        sources: &dyn SourceFilesystemPort,
        requested_root: &Path,
        limits: SourceTreeLimits,
    ) -> Result<Self, PackageError> {
        Ok(Self {
            retained: sources.capture_tree(requested_root, limits)?,
        })
    }

    /// Borrow exact retained bytes for one snapshot-relative file.
    ///
    /// # Errors
    ///
    /// Returns [`PackageError::MissingSnapshotFile`] when no captured regular
    /// file has exactly this relative path.
    pub(crate) fn bytes(&self, relative: &Path) -> Result<&[u8], PackageError> {
        self.retained
            .file(relative)
            .ok_or_else(|| PackageError::MissingSnapshotFile {
                path: relative.to_path_buf(),
            })
    }

    /// Borrow one retained text file after UTF-8 admission.
    ///
    /// # Errors
    ///
    /// Returns [`PackageError::MissingSnapshotFile`] for an absent file or
    /// [`PackageError::NotUtf8`] when its retained bytes are not UTF-8.
    pub(super) fn text(&self, relative: &Path) -> Result<&str, PackageError> {
        std::str::from_utf8(self.bytes(relative)?).map_err(|_| PackageError::NotUtf8 {
            path: relative.to_path_buf(),
        })
    }

    /// Iterate retained files below `relative` in lexical path order.
    pub(crate) fn files_under<'a>(
        &'a self,
        relative: &'a Path,
    ) -> impl Iterator<Item = &'a Path> + 'a {
        self.retained
            .files()
            .filter(move |path| path.starts_with(relative))
    }

    /// Consume the retained tree into its exact lexical file map.
    ///
    /// This ownership transfer is used by the host-to-portable capture seam to
    /// avoid cloning every Package byte out of an already retained snapshot.
    pub(crate) fn into_files(self) -> std::collections::BTreeMap<PathBuf, Box<[u8]>> {
        self.retained.into_files()
    }

    /// Iterate package directories identified by retained `package.toml` files.
    pub(super) fn package_dirs(&self) -> impl Iterator<Item = PathBuf> + '_ {
        self.retained
            .files()
            .filter(|path| path.file_name().is_some_and(|name| name == "package.toml"))
            .filter_map(|path| path.parent().map(Path::to_path_buf))
    }
}

#[cfg(test)]
#[path = "../../tests/internal/packages/snapshot.rs"]
mod tests;
