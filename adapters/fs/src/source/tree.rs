//! Descriptor-relative Unix traversal for retained source trees.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use pale_ports::{SourceFilesystemError, SourceTreeLimits, SourceTreeSnapshot};

use super::root::OpenedSourceRoot;

/// Capture one complete bounded tree through the Unix descriptor-relative implementation.
pub(super) fn capture(
    requested_root: &Path,
    limits: SourceTreeLimits,
) -> Result<SourceTreeSnapshot, SourceFilesystemError> {
    capture_with(requested_root, limits, || {})
}

fn capture_with(
    requested_root: &Path,
    limits: SourceTreeLimits,
    after_collect: impl FnOnce(),
) -> Result<SourceTreeSnapshot, SourceFilesystemError> {
    capture_selected_with(requested_root, limits, &|_| false, &|_| true, after_collect)
}

pub(super) fn capture_selected(
    requested_root: &Path,
    limits: SourceTreeLimits,
    exclude: &dyn Fn(&Path) -> bool,
    include_file: &dyn Fn(&Path) -> bool,
) -> Result<SourceTreeSnapshot, SourceFilesystemError> {
    capture_selected_with(requested_root, limits, exclude, include_file, || {})
}

fn capture_selected_with(
    requested_root: &Path,
    limits: SourceTreeLimits,
    exclude: &dyn Fn(&Path) -> bool,
    include_file: &dyn Fn(&Path) -> bool,
    after_collect: impl FnOnce(),
) -> Result<SourceTreeSnapshot, SourceFilesystemError> {
    let root = OpenedSourceRoot::open(requested_root)?;
    let mut collector = Collector {
        root: requested_root,
        limits,
        directories: BTreeSet::new(),
        files: BTreeMap::new(),
        entries: 0,
        total_bytes: 0,
        exclude,
        include_file,
    };
    collector.collect(root.directory(), Path::new(""), 0)?;
    after_collect();
    root.verify()?;
    SourceTreeSnapshot::try_from_parts(
        requested_root.to_path_buf(),
        collector.directories,
        collector.files,
    )
}

struct Collector<'a> {
    root: &'a Path,
    limits: SourceTreeLimits,
    directories: BTreeSet<PathBuf>,
    files: BTreeMap<PathBuf, Box<[u8]>>,
    entries: usize,
    total_bytes: u64,
    exclude: &'a dyn Fn(&Path) -> bool,
    include_file: &'a dyn Fn(&Path) -> bool,
}

impl Collector<'_> {
    fn collect(
        &mut self,
        directory: &File,
        relative: &Path,
        depth: usize,
    ) -> Result<(), SourceFilesystemError> {
        use rustix::fs::{openat, statat, AtFlags, Dir, FileType, Mode, OFlags};

        if depth > self.limits.depth() {
            return Err(SourceFilesystemError::Limit {
                limit: "directory depth",
                value: depth as u64,
                maximum: self.limits.depth() as u64,
            });
        }
        self.directories.insert(relative.to_path_buf());
        let mut children = Vec::new();
        let mut stream = Dir::read_from(directory).map_err(|source| SourceFilesystemError::Io {
            path: self.root.join(relative),
            source: std::io::Error::from_raw_os_error(source.raw_os_error()),
        })?;
        for entry in &mut stream {
            let entry = entry.map_err(|source| SourceFilesystemError::Io {
                path: self.root.join(relative),
                source: std::io::Error::from_raw_os_error(source.raw_os_error()),
            })?;
            let name =
                entry
                    .file_name()
                    .to_str()
                    .map_err(|_| SourceFilesystemError::InvalidEntry {
                        path: self.root.join(relative).join("<non-utf8>"),
                    })?;
            if name == "." || name == ".." {
                continue;
            }
            if pale_core::paths::PlainFilename::parse(name).is_none() {
                return Err(SourceFilesystemError::InvalidEntry {
                    path: self.root.join(relative).join(name),
                });
            }
            self.entries = self.entries.saturating_add(1);
            if self.entries > self.limits.entries() {
                return Err(SourceFilesystemError::Limit {
                    limit: "entry count",
                    value: self.entries as u64,
                    maximum: self.limits.entries() as u64,
                });
            }
            let child = relative.join(name);
            if (self.exclude)(&child) {
                continue;
            }
            let metadata = statat(directory, entry.file_name(), AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|source| SourceFilesystemError::Io {
                    path: self.root.join(relative).join(name),
                    source: std::io::Error::from_raw_os_error(source.raw_os_error()),
                })?;
            let kind = FileType::from_raw_mode(metadata.st_mode);
            if !kind.is_file() && !kind.is_dir() {
                return Err(SourceFilesystemError::InvalidEntry {
                    path: self.root.join(relative).join(name),
                });
            }
            if kind.is_dir() || (self.include_file)(&child) {
                children.push((name.to_string(), kind.is_dir()));
            }
        }
        children.sort_unstable_by(|left, right| left.0.cmp(&right.0));

        for (name, is_dir) in children {
            let child = relative.join(&name);
            let path = self.root.join(&child);
            if is_dir {
                let child_dir = File::from(
                    openat(
                        directory,
                        name.as_str(),
                        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                        Mode::empty(),
                    )
                    .map_err(|source| SourceFilesystemError::Io {
                        path: path.clone(),
                        source: std::io::Error::from_raw_os_error(source.raw_os_error()),
                    })?,
                );
                self.collect(&child_dir, &child, depth + 1)?;
                continue;
            }
            let file = File::from(
                openat(
                    directory,
                    name.as_str(),
                    OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(|source| SourceFilesystemError::Io {
                    path: path.clone(),
                    source: std::io::Error::from_raw_os_error(source.raw_os_error()),
                })?,
            );
            let bytes = read_opened_bounded(file, &path, self.limits.file_bytes())?;
            self.total_bytes = self.total_bytes.checked_add(bytes.len() as u64).ok_or(
                SourceFilesystemError::Limit {
                    limit: "total bytes",
                    value: u64::MAX,
                    maximum: self.limits.total_bytes(),
                },
            )?;
            if self.total_bytes > self.limits.total_bytes() {
                return Err(SourceFilesystemError::Limit {
                    limit: "total bytes",
                    value: self.total_bytes,
                    maximum: self.limits.total_bytes(),
                });
            }
            self.files.insert(child, bytes.into_boxed_slice());
        }
        Ok(())
    }
}

fn read_opened_bounded(
    file: File,
    path: &Path,
    maximum: u64,
) -> Result<Vec<u8>, SourceFilesystemError> {
    let metadata = file
        .metadata()
        .map_err(|source| SourceFilesystemError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    if !metadata.is_file() {
        return Err(SourceFilesystemError::InvalidEntry {
            path: path.to_path_buf(),
        });
    }
    if metadata.len() > maximum {
        return Err(SourceFilesystemError::Limit {
            limit: "file bytes",
            value: metadata.len(),
            maximum,
        });
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(
            usize::try_from(metadata.len())
                .unwrap_or(usize::MAX)
                .min(64 * 1024),
        )
        .map_err(|source| SourceFilesystemError::Allocation {
            path: path.to_path_buf(),
            source,
        })?;
    file.take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| SourceFilesystemError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    if bytes.len() as u64 > maximum {
        return Err(SourceFilesystemError::Limit {
            limit: "file bytes",
            value: bytes.len() as u64,
            maximum,
        });
    }
    Ok(bytes)
}

#[cfg(test)]
#[path = "../../tests/internal/source/tree.rs"]
mod tests;
