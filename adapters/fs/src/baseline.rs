//! Read the on-disk baseline for a set of surfaces.

use std::collections::BTreeSet;
use std::path::Path;

use pale_core::managed::split_managed_block;
use pale_core::records::{
    ArtifactDigest, BaselineEntry, BaselineStateV1, SurfaceOwnership, UnixFileMode,
};

use crate::confine::{ConfinedRoot, ReadLimit};
use crate::errors::{data_error, FsError};

/// Read the baseline for validated repository-relative paths.
///
/// Missing files are recorded as absent. Unsafe or unreadable entries fail closed;
/// each file is bounded by `file_limit` and hashed as a stream.
///
/// Results are not a coherent multi-file snapshot. A mutator must recheck pre-state
/// at its own effect boundary.
///
/// # Errors
///
/// Rejects duplicate request paths, unsafe roots or leaves, non-regular entries,
/// limit exhaustion, allocation failure, and incomplete reads rather than
/// normalizing them into absence.
pub fn read_baseline(
    root: &Path,
    rel_paths: &[String],
    file_limit: ReadLimit,
) -> Result<Vec<BaselineEntry>, FsError> {
    let root = ConfinedRoot::open(root)?;
    let mut out = Vec::new();
    out.try_reserve_exact(rel_paths.len())
        .map_err(|source| data_error("reserve baseline entries", root.canonical_path(), source))?;
    let mut seen = BTreeSet::new();
    for rel in rel_paths {
        if !seen.insert(rel.as_str()) {
            return Err(FsError::DuplicateRequestPath { path: rel.clone() });
        }
        let entry = match root.location(rel)?.digest_file(file_limit)? {
            Some(existing) => BaselineEntry {
                rel_path: rel.clone(),
                observation: BaselineStateV1::Present {
                    sha256: existing.sha256,
                    mode: UnixFileMode::new(existing.mode).map_err(|source| {
                        FsError::InvalidState(format!(
                            "baseline `{rel}` carries an invalid mode: {source}"
                        ))
                    })?,
                },
            },
            None => BaselineEntry {
                rel_path: rel.clone(),
                observation: BaselineStateV1::Absent {},
            },
        };
        out.push(entry);
    }
    root.verify_path_identity()?;
    out.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(out)
}

/// Compare the exact region owned by Pale, ignoring user text around a managed block.
///
/// # Errors
///
/// Unsafe paths and failures to read a target are returned as observation errors.
pub fn owned_surface_matches(
    root: &ConfinedRoot,
    path: &str,
    ownership: SurfaceOwnership,
    expected: &ArtifactDigest,
    mode: UnixFileMode,
) -> Result<bool, FsError> {
    let location = root.location(path)?;
    match ownership {
        SurfaceOwnership::WholeFile => Ok(location
            .digest_file(crate::MATERIALIZED_SURFACE_READ_LIMIT)?
            .is_some_and(|file| file.sha256 == *expected && file.mode == mode.get())),
        SurfaceOwnership::ManagedBlock => {
            let Some(file) = location.read_file(crate::MATERIALIZED_SURFACE_READ_LIMIT)? else {
                return Ok(false);
            };
            let block = std::str::from_utf8(&file.bytes)
                .ok()
                .and_then(|content| split_managed_block(content).ok());
            Ok(file.mode == mode.get()
                && block.is_some_and(|block| {
                    ArtifactDigest::hash_bytes(block.block().as_bytes()) == *expected
                }))
        }
    }
}
