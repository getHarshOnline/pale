//! Owned, path-free ingress for one deferred one-shot compilation.
//!
//! Captured Package labels model the relative names that a bounded source-tree
//! observation would have produced. They are not host paths, and this module
//! performs no traversal, machine verification, resident admission, or effect.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use pale_core::paths::{ConfinedRelativePath, MAX_CONFINED_RELATIVE_PATH_BYTES};
use pale_ports::SourceTreeSnapshot;

use crate::captured_input::{CapturedCompilationInputError, CapturedInputLimit};
use crate::packages::{
    DistributionSnapshot, MAX_PACKAGE_SNAPSHOT_DEPTH, MAX_PACKAGE_SNAPSHOT_ENTRIES,
    MAX_PACKAGE_SNAPSHOT_FILE_BYTES, MAX_PACKAGE_SNAPSHOT_TOTAL_BYTES,
};
use crate::workspace::CapturedPrivateWorkspaceInputs;

/// One retained Package file addressed beneath the logical `packages` root.
pub struct CapturedPackageDocument {
    label: String,
    bytes: Vec<u8>,
}

impl std::fmt::Debug for CapturedPackageDocument {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapturedPackageDocument")
            .field("label_byte_len", &self.label.len())
            .field("byte_len", &self.bytes.len())
            .finish()
    }
}

impl CapturedPackageDocument {
    /// Maximum bytes accepted for one captured Package document.
    pub const MAX_BYTES: u64 = MAX_PACKAGE_SNAPSHOT_FILE_BYTES;

    /// Admit one confined Package label and one complete regular-file body.
    ///
    /// # Errors
    ///
    /// Returns [`CapturedCompilationInputError`] when `label` is outside the
    /// confined UTF-8 grammar or `bytes` exceeds the existing Package file
    /// ceiling. Tree-wide limits are enforced by [`CapturedPackageSource::new`].
    pub fn new(label: String, bytes: Vec<u8>) -> Result<Self, CapturedCompilationInputError> {
        if label.len() > MAX_CONFINED_RELATIVE_PATH_BYTES
            || ConfinedRelativePath::parse(&label).is_none()
        {
            return Err(CapturedCompilationInputError::InvalidLogicalLabel {
                kind: "Package document",
            });
        }
        if bytes.len() as u64 > MAX_PACKAGE_SNAPSHOT_FILE_BYTES {
            return Err(CapturedCompilationInputError::Limit {
                limit: CapturedInputLimit::PackageFileBytes,
                observed: bytes.len() as u64,
                maximum: MAX_PACKAGE_SNAPSHOT_FILE_BYTES,
            });
        }
        Ok(Self { label, bytes })
    }
}

/// Exact bounded Package tree retained entirely in owned memory.
///
/// Directory ancestors are inferred from document labels. The same entry,
/// directory-depth, per-file, and aggregate-byte ceilings as source-backed
/// Package capture are applied before the value can reach Package admission.
pub struct CapturedPackageSource {
    retained: SourceTreeSnapshot,
}

impl std::fmt::Debug for CapturedPackageSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapturedPackageSource")
            .field("directory_count", &self.retained.directory_count())
            .field("file_count", &self.retained.file_count())
            .field("retained_bytes", &self.retained.retained_bytes())
            .finish()
    }
}

impl CapturedPackageSource {
    /// Maximum file and inferred-directory entries in one Package tree.
    pub const MAX_ENTRIES: usize = MAX_PACKAGE_SNAPSHOT_ENTRIES;
    /// Maximum aggregate bytes retained by one Package tree.
    pub const MAX_TOTAL_BYTES: u64 = MAX_PACKAGE_SNAPSHOT_TOTAL_BYTES;

    /// Form one deterministic retained Package tree from logical documents.
    ///
    /// The descendant ceiling counts every file and every inferred non-root
    /// directory, matching source-tree traversal. A file whose parent is at
    /// depth 24 is admissible; a parent at depth 25 is not.
    ///
    /// # Errors
    ///
    /// Returns [`CapturedCompilationInputError`] for duplicate labels or an
    /// entry, depth, or aggregate-byte excess.
    pub fn new(
        documents: Vec<CapturedPackageDocument>,
    ) -> Result<Self, CapturedCompilationInputError> {
        let mut files = BTreeMap::new();
        let mut duplicate_labels = BTreeSet::new();
        for document in documents {
            let relative = PathBuf::from(&document.label);
            if let std::collections::btree_map::Entry::Vacant(entry) = files.entry(relative) {
                entry.insert(document.bytes.into_boxed_slice());
            } else {
                duplicate_labels.insert(document.label);
            }
        }
        if let Some(label) = duplicate_labels.into_iter().next() {
            return Err(CapturedCompilationInputError::DuplicateLogicalLabel {
                kind: "Package document",
                label,
            });
        }

        let mut directories = BTreeSet::from([PathBuf::new()]);
        for relative in files.keys() {
            let parent_depth = relative.components().count().saturating_sub(1);
            if parent_depth > MAX_PACKAGE_SNAPSHOT_DEPTH {
                return Err(CapturedCompilationInputError::Limit {
                    limit: CapturedInputLimit::PackageDepth,
                    observed: parent_depth as u64,
                    maximum: MAX_PACKAGE_SNAPSHOT_DEPTH as u64,
                });
            }
            for ancestor in relative
                .ancestors()
                .skip(1)
                .filter(|ancestor| !ancestor.as_os_str().is_empty())
            {
                directories.insert(ancestor.to_path_buf());
            }
        }
        let entries = files
            .len()
            .saturating_add(directories.len().saturating_sub(1));
        if entries > MAX_PACKAGE_SNAPSHOT_ENTRIES {
            return Err(CapturedCompilationInputError::Limit {
                limit: CapturedInputLimit::PackageEntries,
                observed: entries as u64,
                maximum: MAX_PACKAGE_SNAPSHOT_ENTRIES as u64,
            });
        }
        let total_bytes = files.values().try_fold(0_u64, |total, bytes| {
            total
                .checked_add(bytes.len() as u64)
                .ok_or(CapturedCompilationInputError::Limit {
                    limit: CapturedInputLimit::PackageTotalBytes,
                    observed: u64::MAX,
                    maximum: MAX_PACKAGE_SNAPSHOT_TOTAL_BYTES,
                })
        })?;
        if total_bytes > MAX_PACKAGE_SNAPSHOT_TOTAL_BYTES {
            return Err(CapturedCompilationInputError::Limit {
                limit: CapturedInputLimit::PackageTotalBytes,
                observed: total_bytes,
                maximum: MAX_PACKAGE_SNAPSHOT_TOTAL_BYTES,
            });
        }
        let retained = SourceTreeSnapshot::try_from_parts(
            PathBuf::from("captured-packages"),
            directories,
            files,
        )
        .map_err(|_| CapturedCompilationInputError::InvalidLogicalLabel {
            kind: "Package document",
        })?;
        Ok(Self { retained })
    }

    pub(super) fn into_snapshot(self) -> DistributionSnapshot {
        DistributionSnapshot::from_retained(self.retained)
    }

    /// Inspect exact Package releases without a distribution lock or host path.
    ///
    /// This operation intentionally stops before Package selection and semantic
    /// graph admission. It exists for lock authoring and repair, where consuming
    /// the lock being replaced would be circular.
    ///
    /// # Errors
    ///
    /// Returns [`crate::packages::PackageError`] when a retained manifest,
    /// source inventory, release identity, or embedded schema is incoherent.
    pub fn inspect_distribution(
        self,
    ) -> Result<crate::packages::DistributionInfo, crate::packages::PackageError> {
        let snapshot = self.into_snapshot();
        crate::packages::inspect_snapshot(&snapshot)
    }

    /// Check Package declarations and reading links through the compiler's reader.
    ///
    /// # Errors
    ///
    /// Returns [`crate::packages::PackageError`] for invalid source, dependencies,
    /// bodies or links. This view selects no workspace resources.
    pub fn read_catalog(
        self,
    ) -> Result<crate::packages::PackageCatalog, crate::packages::PackageError> {
        crate::packages::read_catalog_snapshot(&self.into_snapshot())
    }
}

/// Captured private and Package inputs selected for one one-shot compilation.
///
/// The selector strings are checked against the admitted desired topology and
/// machine record during compilation. Construction alone proves no cross-record
/// coherence, machine readiness, Package admission, or common-root provenance.
pub struct CapturedOneShotCompilationInputs {
    pub(super) project_id: String,
    pub(super) execution_workspace_id: String,
    pub(super) machine_id: String,
    pub(super) private: CapturedPrivateWorkspaceInputs,
    pub(super) packages: CapturedPackageSource,
}

impl std::fmt::Debug for CapturedOneShotCompilationInputs {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapturedOneShotCompilationInputs")
            .field("project_id_byte_len", &self.project_id.len())
            .field(
                "execution_workspace_id_byte_len",
                &self.execution_workspace_id.len(),
            )
            .field("machine_id_byte_len", &self.machine_id.len())
            .field("private", &self.private)
            .field("packages", &self.packages)
            .finish()
    }
}

impl CapturedOneShotCompilationInputs {
    /// Bind selectors to already-bounded private and Package documents.
    ///
    /// Topology selection is deliberately deferred until the captured root
    /// workspace has passed the existing semantic admission path.
    #[must_use]
    pub fn new(
        project_id: String,
        execution_workspace_id: String,
        machine_id: String,
        private: CapturedPrivateWorkspaceInputs,
        packages: CapturedPackageSource,
    ) -> Self {
        Self {
            project_id,
            execution_workspace_id,
            machine_id,
            private,
            packages,
        }
    }
}
