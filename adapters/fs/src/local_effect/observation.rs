//! Exact file and ancestor observations for machine-local effect planning.

use pale_core::records::{
    ArtifactDigest, ExistingMachineLocalFileV1, FilesystemDeviceIdV1, FilesystemInodeIdV1,
    MachineLocalDirectoryIdentity, MachineLocalFileIdentityV1, UnixFileMode,
};

use crate::confine::{ConfinedRoot, ReadLimit};
use crate::errors::FsError;

pub use pale_ports::MachineLocalFileObservation;

/// Observe one bounded regular file and its complete ancestor partition.
///
/// # Errors
///
/// Rejects a replaced root, unsafe or non-UTF-8 paths, non-regular entries,
/// oversized content, and incomplete descriptor-relative observations.
pub fn observe_machine_local_file(
    root: &ConfinedRoot,
    rel_path: &str,
    limit: ReadLimit,
) -> Result<MachineLocalFileObservation, FsError> {
    let identity = root.machine_local_identity()?;
    let location = root.location(rel_path)?;
    let existing = location.read_file(limit)?;
    let (prior, bytes) = match existing {
        Some(existing) => (
            Some(ExistingMachineLocalFileV1 {
                sha256: ArtifactDigest::hash_bytes(&existing.bytes),
                mode: UnixFileMode::new(existing.mode).map_err(|source| {
                    FsError::InvalidMachineLocalEffect(format!(
                        "observed file carries an invalid mode: {source}"
                    ))
                })?,
                identity: MachineLocalFileIdentityV1 {
                    device: FilesystemDeviceIdV1::new(existing.identity.device),
                    inode: FilesystemInodeIdV1::new(existing.identity.inode),
                },
            }),
            Some(existing.bytes),
        ),
        None => (None, None),
    };
    let (retained_parent_directories, missing_parent_directories) =
        observe_parent_directories(root, rel_path)?;
    Ok(MachineLocalFileObservation {
        root: identity,
        prior,
        bytes,
        retained_parent_directories,
        missing_parent_directories,
    })
}

fn observe_parent_directories(
    root: &ConfinedRoot,
    rel_path: &str,
) -> Result<(Vec<MachineLocalDirectoryIdentity>, Vec<String>), FsError> {
    let relative = pale_core::paths::ConfinedRelativePath::parse(rel_path)
        .ok_or_else(|| FsError::Escape(rel_path.to_string()))?;
    let components = relative.components().collect::<Vec<_>>();
    let mut retained = Vec::new();
    let mut missing = Vec::new();
    let mut prefix = String::new();
    let mut ancestor_missing = false;
    for component in components.iter().take(components.len().saturating_sub(1)) {
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(component);
        if ancestor_missing {
            missing.push(prefix.clone());
            continue;
        }
        let location = root.location(&prefix)?;
        if let Some(identity) = location.directory_identity()? {
            retained.push(MachineLocalDirectoryIdentity {
                rel_path: prefix.clone(),
                device: FilesystemDeviceIdV1::new(identity.device),
                inode: FilesystemInodeIdV1::new(identity.inode),
            });
        } else {
            ancestor_missing = true;
            missing.push(prefix.clone());
        }
    }
    Ok((retained, missing))
}
