//! Local no-follow source observations for current Pale loaders.

mod read;
#[cfg(unix)]
mod root;
#[cfg(unix)]
mod tree;

use std::path::Path;

use pale_ports::{
    SourceFilesystemError, SourceFilesystemPort, SourceReadLimit, SourceTreeLimits,
    SourceTreeSnapshot,
};

/// Stateless local implementation of [`SourceFilesystemPort`].
///
/// Each request supplies its own complete resource limits. Unsupported targets
/// fail closed rather than falling back to path-following traversal, and a
/// successful observation grants no later mutation authority.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalSourceFilesystem;

impl SourceFilesystemPort for LocalSourceFilesystem {
    fn read_regular(
        &self,
        path: &Path,
        limit: SourceReadLimit,
    ) -> Result<Vec<u8>, SourceFilesystemError> {
        read::read_regular(path, limit)
    }

    fn read_confined_regular(
        &self,
        root: &Path,
        relative: &Path,
        limit: SourceReadLimit,
    ) -> Result<Vec<u8>, SourceFilesystemError> {
        read::read_confined_regular(root, relative, limit)
    }

    fn capture_tree(
        &self,
        root: &Path,
        limits: SourceTreeLimits,
    ) -> Result<SourceTreeSnapshot, SourceFilesystemError> {
        #[cfg(unix)]
        {
            tree::capture(root, limits)
        }
        #[cfg(not(unix))]
        {
            let _ = (root, limits);
            Err(SourceFilesystemError::UnsupportedPlatform {
                platform: std::env::consts::OS,
            })
        }
    }
}

impl LocalSourceFilesystem {
    /// Capture selected files without opening excluded subtrees or other files.
    ///
    /// The caller owns selection policy. `exclude` prunes a relative entry before
    /// observing its type; `include_file` chooses regular files after that check.
    /// All traversed entries count toward the limit, including skipped files.
    /// Symlinks and special entries outside explicit exclusions are rejected.
    ///
    /// # Errors
    ///
    /// Returns a source error for unsafe entries, changed roots, failed reads or
    /// exceeded limits. Selection does not weaken descriptor-relative traversal.
    pub fn capture_selected_tree(
        &self,
        root: &Path,
        limits: SourceTreeLimits,
        exclude: &dyn Fn(&Path) -> bool,
        include_file: &dyn Fn(&Path) -> bool,
    ) -> Result<SourceTreeSnapshot, SourceFilesystemError> {
        #[cfg(unix)]
        {
            tree::capture_selected(root, limits, exclude, include_file)
        }
        #[cfg(not(unix))]
        {
            let _ = (root, limits, exclude, include_file);
            Err(SourceFilesystemError::UnsupportedPlatform {
                platform: std::env::consts::OS,
            })
        }
    }
}
