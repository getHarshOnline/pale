//! Bounded directory observations beneath a retained [`super::ConfinedRoot`].
//!
//! Enumeration reports every child kind without following symbolic links. A
//! traversal is deterministic in ordering, but it is not an atomic snapshot:
//! disappearance or type drift is an error, never a silently omitted entry.

use super::limits::{DirectoryLimit, TraversalLimit};
use super::root::ConfinedLocation;
#[cfg(unix)]
use crate::errors::errno_error;
#[cfg(not(unix))]
use crate::errors::unsupported;
use crate::errors::{data_error, FsError};

/// Entry kinds reported by a confined directory scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfinedEntryKind {
    /// Child observed as a regular file without following it.
    File,
    /// Child observed as a directory without following it.
    Directory,
    /// Symbolic link, reported but never followed.
    Symlink,
    /// Socket, device, FIFO, or another unsupported entry kind.
    Other,
}

/// UTF-8 child name and type observed without following the child.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfinedDirectoryEntry {
    /// One UTF-8 path component, without a parent prefix.
    pub name: String,
    /// Point-in-time no-follow classification; effects must resolve again.
    pub kind: ConfinedEntryKind,
}

/// One descendant returned by a bounded confined traversal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfinedTreeEntry {
    /// Lexically valid path relative to the retained root.
    pub relative: String,
    /// Point-in-time no-follow classification; effects must resolve again.
    pub kind: ConfinedEntryKind,
}

#[derive(Clone, Copy)]
enum EntryLimitKind {
    Directory,
    Traversal { max_entries: usize },
}

impl ConfinedLocation {
    /// Enumerate at most `limit` UTF-8 children without following them.
    ///
    /// Every child kind consumes the budget. Results are name-sorted; absence
    /// returns `None`, and callers decide which entry kinds to admit. Unsafe or
    /// changing paths are never normalized into absence.
    ///
    /// # Errors
    ///
    /// Rejects non-directory or changing paths, non-UTF-8 names, exhausted budgets,
    /// allocation failure, and incomplete directory scans without a partial result.
    pub fn read_directory(
        &self,
        limit: DirectoryLimit,
    ) -> Result<Option<Vec<ConfinedDirectoryEntry>>, FsError> {
        self.read_directory_inner(
            limit.max_entries(),
            EntryLimitKind::Directory,
            self.relative(),
        )
    }

    /// Traverse descendants with aggregate count and depth bounds.
    ///
    /// Every descendant kind counts; symlinks and special entries are reported but
    /// not followed, and the root is excluded. Results are sorted. Traversal is not
    /// an atomic snapshot, so unsafe changes while walking are collisions.
    ///
    /// # Errors
    ///
    /// A name, count, depth, allocation, type change, disappearance, or I/O failure
    /// returns no partial tree.
    pub fn walk_directory(
        &self,
        limit: TraversalLimit,
    ) -> Result<Option<Vec<ConfinedTreeEntry>>, FsError> {
        let Some(root_entries) = self.read_directory_inner(
            limit.max_entries(),
            EntryLimitKind::Traversal {
                max_entries: limit.max_entries(),
            },
            self.relative(),
        )?
        else {
            return Ok(None);
        };
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(root_entries.len())
            .map_err(|error| self.directory_allocation_error("traversal", error))?;
        let mut pending = Vec::new();
        self.collect_tree_entries(&root_entries, 1, limit, &mut entries, &mut pending)?;

        while let Some((directory, depth)) = pending.pop() {
            let remaining = limit.max_entries().saturating_sub(entries.len());
            let children = directory
                .read_directory_inner(
                    remaining,
                    EntryLimitKind::Traversal {
                        max_entries: limit.max_entries(),
                    },
                    self.relative(),
                )?
                .ok_or_else(|| {
                    FsError::Collision(
                        directory.relative().to_string(),
                        "directory disappeared during confined traversal".into(),
                    )
                })?;
            directory.collect_tree_entries(
                &children,
                depth + 1,
                limit,
                &mut entries,
                &mut pending,
            )?;
        }
        entries.sort_unstable_by(|left, right| left.relative.cmp(&right.relative));
        Ok(Some(entries))
    }

    fn collect_tree_entries(
        &self,
        children: &[ConfinedDirectoryEntry],
        depth: usize,
        limit: TraversalLimit,
        entries: &mut Vec<ConfinedTreeEntry>,
        pending: &mut Vec<(ConfinedLocation, usize)>,
    ) -> Result<(), FsError> {
        let mut directories = Vec::new();
        for child in children {
            let relative = format!("{}/{}", self.relative(), child.name);
            if depth > limit.max_depth() {
                return Err(FsError::TraversalDepthLimit {
                    path: relative,
                    max_depth: limit.max_depth(),
                });
            }
            if entries.len() >= limit.max_entries() {
                return Err(FsError::TraversalEntryLimit {
                    path: self.relative().to_string(),
                    max_entries: limit.max_entries(),
                });
            }
            entries
                .try_reserve(1)
                .map_err(|error| self.directory_allocation_error("traversal", error))?;
            entries.push(ConfinedTreeEntry {
                relative: relative.clone(),
                kind: child.kind,
            });
            if child.kind == ConfinedEntryKind::Directory {
                directories
                    .try_reserve(1)
                    .map_err(|error| self.directory_allocation_error("traversal", error))?;
                directories.push((self.root.location(&relative)?, depth));
            }
        }
        pending
            .try_reserve(directories.len())
            .map_err(|error| self.directory_allocation_error("traversal", error))?;
        pending.extend(directories.into_iter().rev());
        Ok(())
    }

    fn read_directory_inner(
        &self,
        max_entries: usize,
        limit_kind: EntryLimitKind,
        traversal_root: &str,
    ) -> Result<Option<Vec<ConfinedDirectoryEntry>>, FsError> {
        let Some(directory) = self.open_directory()? else {
            return Ok(None);
        };
        #[cfg(unix)]
        {
            use rustix::fs::{statat, AtFlags, Dir, FileType};

            let mut children = Vec::new();
            let mut entries = Dir::read_from(&directory).map_err(|source| {
                errno_error(
                    "open confined directory stream",
                    self.display_path(),
                    source,
                )
            })?;
            for entry in &mut entries {
                let entry = entry.map_err(|source| {
                    errno_error("read confined directory entry", self.display_path(), source)
                })?;
                let name = entry.file_name().to_str().map_err(|_| {
                    FsError::Collision(
                        self.relative.clone(),
                        "directory contains a non-UTF-8 name".into(),
                    )
                })?;
                if name == "." || name == ".." {
                    continue;
                }
                let metadata = statat(&directory, entry.file_name(), AtFlags::SYMLINK_NOFOLLOW)
                    .map_err(|source| {
                        errno_error(
                            "inspect confined directory entry",
                            self.display_path().join(name),
                            source,
                        )
                    })?;
                let kind = match FileType::from_raw_mode(metadata.st_mode) {
                    kind if kind.is_file() => ConfinedEntryKind::File,
                    kind if kind.is_dir() => ConfinedEntryKind::Directory,
                    kind if kind.is_symlink() => ConfinedEntryKind::Symlink,
                    _ => ConfinedEntryKind::Other,
                };
                self.push_entry(
                    &mut children,
                    ConfinedDirectoryEntry {
                        name: name.to_string(),
                        kind,
                    },
                    max_entries,
                    limit_kind,
                    traversal_root,
                )?;
            }
            children.sort_unstable_by(|left, right| left.name.cmp(&right.name));
            Ok(Some(children))
        }
        #[cfg(not(unix))]
        {
            let _ = (max_entries, limit_kind, traversal_root, directory);
            Err(unsupported("read confined directory"))
        }
    }

    fn check_entry_limit(
        &self,
        observed: usize,
        max_entries: usize,
        limit_kind: EntryLimitKind,
        traversal_root: &str,
    ) -> Result<(), FsError> {
        if observed < max_entries {
            return Ok(());
        }
        Err(match limit_kind {
            EntryLimitKind::Directory => FsError::DirectoryEntryLimit {
                path: self.relative().to_string(),
                max_entries,
            },
            EntryLimitKind::Traversal { max_entries } => FsError::TraversalEntryLimit {
                path: traversal_root.to_string(),
                max_entries,
            },
        })
    }

    fn push_entry(
        &self,
        entries: &mut Vec<ConfinedDirectoryEntry>,
        entry: ConfinedDirectoryEntry,
        max_entries: usize,
        limit_kind: EntryLimitKind,
        traversal_root: &str,
    ) -> Result<(), FsError> {
        self.check_entry_limit(entries.len(), max_entries, limit_kind, traversal_root)?;
        entries
            .try_reserve(1)
            .map_err(|error| self.directory_allocation_error("scan", error))?;
        entries.push(entry);
        Ok(())
    }

    fn directory_allocation_error(
        &self,
        operation: &str,
        error: std::collections::TryReserveError,
    ) -> FsError {
        let operation = match operation {
            "traversal" => "reserve confined traversal buffer",
            _ => "reserve confined directory buffer",
        };
        data_error(operation, self.display_path(), error)
    }
}
