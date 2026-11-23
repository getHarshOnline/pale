//! Bounded source-filesystem observations used by current compilation inputs.

use std::collections::{BTreeMap, BTreeSet, TryReserveError};
use std::fmt;
use std::path::{Component, Path, PathBuf};

/// Maximum complete bytes admitted from one standalone source file.
///
/// Zero admits only an empty file. Implementations must reject, rather than
/// truncate, a file that exceeds the ceiling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceReadLimit {
    max_bytes: usize,
}

impl SourceReadLimit {
    /// Create a complete-file byte ceiling.
    pub const fn new(max_bytes: usize) -> Self {
        Self { max_bytes }
    }

    /// Return the admitted complete-file byte ceiling.
    pub const fn max_bytes(self) -> usize {
        self.max_bytes
    }
}

/// Resource ceilings for one recursively retained source tree.
///
/// The entry count excludes the retained root, and immediate children have
/// depth one. Any zero dimension admits only a tree that consumes none of that
/// resource.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceTreeLimits {
    entries: usize,
    depth: usize,
    file_bytes: u64,
    total_bytes: u64,
}

impl SourceTreeLimits {
    /// Create exact entry, depth, per-file, and aggregate byte ceilings.
    pub const fn new(entries: usize, depth: usize, file_bytes: u64, total_bytes: u64) -> Self {
        Self {
            entries,
            depth,
            file_bytes,
            total_bytes,
        }
    }

    /// Maximum descendant entries.
    pub const fn entries(self) -> usize {
        self.entries
    }

    /// Maximum directory depth below the retained root.
    pub const fn depth(self) -> usize {
        self.depth
    }

    /// Maximum complete bytes retained from one file.
    pub const fn file_bytes(self) -> u64 {
        self.file_bytes
    }

    /// Maximum aggregate bytes retained from the tree.
    pub const fn total_bytes(self) -> u64 {
        self.total_bytes
    }
}

/// Exact bounded bytes observed from one source tree.
///
/// Paths are relative to [`Self::root`]. Concrete adapters construct the snapshot only
/// after no-follow traversal and root-identity revalidation; application policy
/// consumes the retained bytes without returning to the live tree.
pub struct SourceTreeSnapshot {
    root: PathBuf,
    directories: BTreeSet<PathBuf>,
    files: BTreeMap<PathBuf, Box<[u8]>>,
}

impl SourceTreeSnapshot {
    /// Construct a structurally valid retained observation from adapter-produced parts.
    ///
    /// This validates the representation only. The implementing adapter remains
    /// responsible for satisfying [`SourceFilesystemPort::capture_tree`]'s
    /// no-follow traversal and root-revalidation contract.
    ///
    /// # Errors
    ///
    /// Returns [`SourceFilesystemError::InvalidEntry`] when a retained path is
    /// not a normalized relative path, the retained root directory is absent,
    /// a file overlaps a directory, or an entry's directory ancestry is absent.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::{BTreeMap, BTreeSet};
    /// use std::path::PathBuf;
    /// use pale_ports::SourceTreeSnapshot;
    ///
    /// let snapshot = SourceTreeSnapshot::try_from_parts(
    ///     PathBuf::from("diagnostic-source"),
    ///     BTreeSet::from([PathBuf::new()]),
    ///     BTreeMap::from([(
    ///         PathBuf::from("README.md"),
    ///         Vec::from(&b"# retained\n"[..]).into_boxed_slice(),
    ///     )]),
    /// )?;
    /// assert_eq!(snapshot.file(PathBuf::from("README.md").as_path()), Some(&b"# retained\n"[..]));
    /// # Ok::<(), pale_ports::SourceFilesystemError>(())
    /// ```
    pub fn try_from_parts(
        root: PathBuf,
        directories: BTreeSet<PathBuf>,
        files: BTreeMap<PathBuf, Box<[u8]>>,
    ) -> Result<Self, SourceFilesystemError> {
        let retained_root = Path::new("");
        if !directories.contains(retained_root) {
            return Err(SourceFilesystemError::InvalidEntry { path: root });
        }
        for directory in &directories {
            validate_relative_entry(&root, directory, true)?;
            if !directory.as_os_str().is_empty()
                && directory
                    .parent()
                    .is_none_or(|parent| !directories.contains(parent))
            {
                return Err(SourceFilesystemError::InvalidEntry {
                    path: root.join(directory),
                });
            }
        }
        for file in files.keys() {
            validate_relative_entry(&root, file, false)?;
            if directories.contains(file)
                || file
                    .parent()
                    .is_none_or(|parent| !directories.contains(parent))
            {
                return Err(SourceFilesystemError::InvalidEntry {
                    path: root.join(file),
                });
            }
        }
        Ok(Self {
            root,
            directories,
            files,
        })
    }

    /// Requested source root retained only as a diagnostic coordinate.
    ///
    /// This path is not a live capability; consumers use the retained entries
    /// instead of reopening it as part of the observation.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Whether the retained observation contains exactly this directory.
    pub fn contains_dir(&self, relative: &Path) -> bool {
        self.directories.contains(relative)
    }

    /// Borrow the exact retained bytes for one relative regular file.
    pub fn file(&self, relative: &Path) -> Option<&[u8]> {
        self.files.get(relative).map(AsRef::as_ref)
    }

    /// Iterate retained relative file paths in lexical order.
    pub fn files(&self) -> impl Iterator<Item = &Path> {
        self.files.keys().map(PathBuf::as_path)
    }

    /// Consume the observation into its lexically ordered retained files.
    ///
    /// The returned paths remain relative labels and the byte boxes are moved,
    /// not copied. Root and directory metadata are discarded with the consumed
    /// snapshot; no live filesystem capability is returned.
    pub fn into_files(self) -> BTreeMap<PathBuf, Box<[u8]>> {
        self.files
    }

    /// Number of retained directories.
    pub fn directory_count(&self) -> usize {
        self.directories.len()
    }

    /// Number of retained files.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Aggregate retained file bytes.
    pub fn retained_bytes(&self) -> usize {
        self.files.values().map(|bytes| bytes.len()).sum()
    }
}

fn validate_relative_entry(
    root: &Path,
    relative: &Path,
    allow_empty: bool,
) -> Result<(), SourceFilesystemError> {
    let mut normalized = PathBuf::new();
    let valid_components = relative.components().all(|component| match component {
        Component::Normal(name) => {
            let Some(name) = name.to_str() else {
                return false;
            };
            if pale_core::paths::PlainFilename::parse(name).is_none() {
                return false;
            }
            normalized.push(name);
            true
        }
        _ => false,
    });
    let valid = (allow_empty || !relative.as_os_str().is_empty())
        && !relative.is_absolute()
        && valid_components
        && normalized.as_os_str() == relative.as_os_str();
    if valid || (allow_empty && relative.as_os_str().is_empty()) {
        Ok(())
    } else {
        Err(SourceFilesystemError::InvalidEntry {
            path: root.join(relative),
        })
    }
}

impl fmt::Debug for SourceTreeSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SourceTreeSnapshot")
            .field("root", &self.root)
            .field("directories", &self.directory_count())
            .field("files", &self.file_count())
            .field("retained_bytes", &self.retained_bytes())
            .finish()
    }
}

#[cfg(test)]
#[path = "../tests/internal/source.rs"]
mod tests;

/// Failure to observe a bounded no-follow source file or tree.
#[derive(Debug, thiserror::Error)]
pub enum SourceFilesystemError {
    /// Opening, inspecting, or streaming the requested source failed.
    #[error("cannot read source `{path}`: {source}")]
    Io {
        /// Source path being observed.
        path: PathBuf,
        /// Underlying filesystem failure.
        #[source]
        source: std::io::Error,
    },
    /// An entry was symlinked, special, escaped, or not the required type.
    #[error("source entry is not a confined regular file or directory: `{path}`", path = .path.display())]
    InvalidEntry {
        /// Rejected source path.
        path: PathBuf,
    },
    /// Memory for bounded source bytes could not be reserved.
    #[error("cannot allocate bounded source `{path}`: {source}")]
    Allocation {
        /// Source path whose bytes were being retained.
        path: PathBuf,
        /// Failed bounded vector reservation.
        #[source]
        source: TryReserveError,
    },
    /// A configured source-tree dimension exceeded its ceiling.
    #[error("source tree exceeds {limit}: {value} > {maximum}")]
    Limit {
        /// Bounded dimension that was exceeded.
        limit: &'static str,
        /// Exact observed value, or the smallest value proven to exceed the limit.
        value: u64,
        /// Maximum admitted value.
        maximum: u64,
    },
    /// This target cannot implement the required no-follow observation.
    #[error("bounded source observation is unsupported on `{platform}`")]
    UnsupportedPlatform {
        /// Rust target operating-system token.
        platform: &'static str,
    },
}

/// Outbound source observation required by current loaders and compilation.
///
/// Implementations return complete retained observations or a typed error; they
/// never expose a live handle or authorize later mutation. Root-confined reads
/// and recursive capture resolve descendants from retained directory
/// descriptors and revalidate the selected root identity.
pub trait SourceFilesystemPort {
    /// Read one complete no-follow regular-file leaf within `limit`.
    ///
    /// # Errors
    ///
    /// Returns [`SourceFilesystemError`] when the leaf is not a regular file,
    /// cannot be observed without following it, or exceeds the declared bound.
    fn read_regular(
        &self,
        path: &Path,
        limit: SourceReadLimit,
    ) -> Result<Vec<u8>, SourceFilesystemError>;

    /// Read one complete root-relative file through retained descriptors.
    ///
    /// # Errors
    ///
    /// Returns [`SourceFilesystemError`] when `relative` is not lexically
    /// confined, the root or a descendant changes or follows a symlink, the leaf
    /// is not regular, or the stream exceeds `limit`.
    fn read_confined_regular(
        &self,
        root: &Path,
        relative: &Path,
        limit: SourceReadLimit,
    ) -> Result<Vec<u8>, SourceFilesystemError>;

    /// Retain one recursively bounded no-follow source tree.
    ///
    /// # Errors
    ///
    /// Returns [`SourceFilesystemError`] when traversal encounters an unsafe
    /// entry, resource ceiling, unsupported platform, or changed root identity.
    fn capture_tree(
        &self,
        root: &Path,
        limits: SourceTreeLimits,
    ) -> Result<SourceTreeSnapshot, SourceFilesystemError>;
}
