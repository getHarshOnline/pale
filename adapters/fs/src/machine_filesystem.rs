//! Local implementation of neutral machine-filesystem observations.

use std::fs;
use std::fs::Metadata;
use std::path::Path;

use pale_ports::{
    MachineFilesystemError, MachineFilesystemOperation, MachineFilesystemPort, MachinePathKind,
    MachinePathObservation,
};

/// Stateless local implementation of [`MachineFilesystemPort`].
///
/// Every call reports a fresh observation. It retains no descriptor, performs
/// no mutation, and leaves path, type, permission, and containment decisions to
/// the consuming application policy.
#[derive(Clone, Copy, Debug, Default)]
pub struct LocalMachineFilesystem;

impl MachineFilesystemPort for LocalMachineFilesystem {
    fn inspect_requested(&self, path: &Path) -> Result<MachinePathKind, MachineFilesystemError> {
        metadata(path, MachineFilesystemOperation::InspectRequested).map(|value| path_kind(&value))
    }

    fn canonicalize(&self, path: &Path) -> Result<std::path::PathBuf, MachineFilesystemError> {
        fs::canonicalize(path).map_err(|source| {
            MachineFilesystemError::new(
                MachineFilesystemOperation::Canonicalize,
                path.to_path_buf(),
                source,
            )
        })
    }

    fn observe(&self, path: &Path) -> Result<MachinePathObservation, MachineFilesystemError> {
        let requested = metadata(path, MachineFilesystemOperation::InspectRequested)?;
        let canonical_path = self.canonicalize(path)?;
        let canonical = metadata(
            &canonical_path,
            MachineFilesystemOperation::InspectCanonical,
        )?;

        Ok(MachinePathObservation::new(
            canonical_path,
            path_kind(&requested),
            path_kind(&canonical),
            unix_mode(&requested),
        ))
    }
}

fn metadata(
    path: &Path,
    operation: MachineFilesystemOperation,
) -> Result<Metadata, MachineFilesystemError> {
    fs::symlink_metadata(path)
        .map_err(|source| MachineFilesystemError::new(operation, path.to_path_buf(), source))
}

fn path_kind(metadata: &Metadata) -> MachinePathKind {
    let kind = metadata.file_type();
    if kind.is_dir() {
        MachinePathKind::Directory
    } else if kind.is_file() {
        MachinePathKind::RegularFile
    } else if kind.is_symlink() {
        MachinePathKind::Symlink
    } else {
        MachinePathKind::Other
    }
}

#[cfg(unix)]
fn unix_mode(metadata: &Metadata) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;

    Some(metadata.permissions().mode())
}

#[cfg(not(unix))]
fn unix_mode(_metadata: &Metadata) -> Option<u32> {
    None
}
