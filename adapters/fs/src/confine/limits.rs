//! Explicit resource budgets for confined reads and traversal.

/// Maximum bytes accepted from one regular-file read or digest operation.
///
/// The ceiling is enforced against both initial metadata and bytes consumed, so a
/// file that grows while being read is not silently admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadLimit {
    max_bytes: usize,
}

/// Per-file ceiling shared by baseline capture, reconciliation, and rollback.
pub const MATERIALIZED_SURFACE_READ_LIMIT: ReadLimit = ReadLimit::new(8 * 1024 * 1024);

impl ReadLimit {
    /// Build a byte budget. Zero admits only an empty file.
    pub const fn new(max_bytes: usize) -> Self {
        Self { max_bytes }
    }

    /// Return the exact byte ceiling for internal enforcement.
    pub(crate) const fn max_bytes(self) -> usize {
        self.max_bytes
    }
}

/// Maximum children accepted from one directory enumeration.
///
/// Every observed child counts, including symlinks and unsupported entry kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectoryLimit {
    max_entries: usize,
}

impl DirectoryLimit {
    /// Build an entry budget. Zero admits only an empty directory.
    pub const fn new(max_entries: usize) -> Self {
        Self { max_entries }
    }

    /// Return the exact child-entry ceiling for internal enforcement.
    pub(super) const fn max_entries(self) -> usize {
        self.max_entries
    }
}

/// Aggregate entry and nesting bounds for one recursive directory traversal.
///
/// The traversal root is excluded from the entry count; every descendant kind is
/// included, and immediate children have depth one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraversalLimit {
    max_entries: usize,
    max_depth: usize,
}

impl TraversalLimit {
    /// Build a traversal budget; zero depth admits only an empty root.
    pub const fn new(max_entries: usize, max_depth: usize) -> Self {
        Self {
            max_entries,
            max_depth,
        }
    }

    /// Return the aggregate descendant ceiling for internal enforcement.
    pub(super) const fn max_entries(self) -> usize {
        self.max_entries
    }

    /// Return the descendant-depth ceiling for internal enforcement.
    pub(super) const fn max_depth(self) -> usize {
        self.max_depth
    }
}
