//! Bounded, descriptor-confined measurement of logical build-file lengths.

use std::path::{Component, Path};

use crate::{ConfinedEntryKind, ConfinedRoot, ReadLimit, TraversalLimit};

/// Observed size of one tree; errors leave its total absent or explicitly partial.
#[derive(Debug, Default)]
pub struct BuildSizeObservation {
    /// Sum of regular-file lengths, counting each hard-link entry separately.
    /// This is not allocated, exclusive or reclaimable disk space.
    pub logical_bytes: Option<u64>,
    /// Successfully measured regular-file entries.
    pub file_count: u64,
    /// Symbolic links excluded from measurement.
    pub skipped_symlinks: u64,
    /// At most 32 failures; reaching that ceiling stops measurement.
    pub errors: Vec<String>,
}

/// Measure one absolute physical directory without opening file contents.
///
/// Roots reached through symlinks are refused. Descendants are read through
/// retained directory descriptors. Traversal permits at most one million
/// entries and 128 levels; exceeding either bound yields no whole-tree total.
/// A scan is not an atomic snapshot of concurrent builds.
#[must_use]
pub fn measure_build_tree(path: &Path) -> BuildSizeObservation {
    let mut report = BuildSizeObservation::default();
    let tree = (|| {
        if !path.is_absolute()
            || path
                .components()
                .any(|part| matches!(part, Component::ParentDir))
        {
            return Err(
                "Supply an absolute physical directory without parent components.".to_owned(),
            );
        }
        for ancestor in path.ancestors() {
            let metadata =
                std::fs::symlink_metadata(ancestor).map_err(|error| error.to_string())?;
            if metadata.is_symlink() {
                return Err(format!("Path uses a symlink: {}", ancestor.display()));
            }
        }
        let parent = path
            .parent()
            .ok_or("A filesystem root is not a build directory.")?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("Build directory name must be UTF-8.")?;
        let root = ConfinedRoot::open(parent).map_err(|error| error.to_string())?;
        let location = root.location(name).map_err(|error| error.to_string())?;
        let entries = location
            .walk_directory(TraversalLimit::new(1_000_000, 128))
            .map_err(|error| error.to_string())?
            .ok_or("Build directory is missing.")?;
        Ok::<_, String>((root, entries))
    })();
    let (root, entries) = match tree {
        Ok(tree) => tree,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };
    report.logical_bytes = Some(0);
    for entry in entries {
        let measured = match entry.kind {
            ConfinedEntryKind::Directory => continue,
            ConfinedEntryKind::Symlink => {
                report.skipped_symlinks += 1;
                continue;
            }
            ConfinedEntryKind::Other => Err(format!("Unsupported file type: {}", entry.relative)),
            ConfinedEntryKind::File => root
                .location(&entry.relative)
                .and_then(|location| location.inspect_file(ReadLimit::new(usize::MAX)))
                .map_err(|error| error.to_string())
                .and_then(|metadata| {
                    metadata.ok_or_else(|| format!("File disappeared: {}", entry.relative))
                })
                .and_then(|metadata| {
                    report
                        .logical_bytes
                        .unwrap_or(0)
                        .checked_add(metadata.len)
                        .ok_or_else(|| "Logical byte total exceeds u64.".to_owned())
                }),
        };
        match measured {
            Ok(total) => {
                report.logical_bytes = Some(total);
                report.file_count += 1;
            }
            Err(error) => {
                report.errors.push(error);
                if report.errors.len() == 32 {
                    break;
                }
            }
        }
    }
    report
}
