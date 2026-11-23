//! Filesystem verification for machine-local state and ordered authority documents.

use std::path::{Component, Path, PathBuf};

use pale_core::records::{MaterializationLineageBinding, ResolvedProjectInstanceV1, SessionModel};
use pale_ports::{
    MachineFilesystemError, MachineFilesystemOperation, MachineFilesystemPort, MachinePathKind,
};

use crate::workspace::WorkspaceError;

#[derive(Debug)]
enum StateRootIssue {
    NotAbsolute,
    NotFound,
    NotDirectory,
    NotConfined,
    Observation(Box<MachineFilesystemError>),
    #[cfg(unix)]
    UnsafePermissions,
}

#[derive(Debug, Clone, Copy)]
enum StateRootKind {
    Session,
    DurableEvidence,
    MaterializationLineage,
}

fn state_root_error(kind: StateRootKind, issue: StateRootIssue, root: &str) -> WorkspaceError {
    let root = root.to_string();
    match (kind, issue) {
        (_, StateRootIssue::Observation(source)) => WorkspaceError::SessionPathObservation {
            path: root,
            binding: kind.binding().to_owned(),
            source,
        },
        (StateRootKind::Session, StateRootIssue::NotAbsolute) => {
            WorkspaceError::SessionStateNotAbsolute(root)
        }
        (StateRootKind::Session, StateRootIssue::NotFound) => {
            WorkspaceError::SessionStateNotFound(root)
        }
        (StateRootKind::Session, StateRootIssue::NotDirectory) => {
            WorkspaceError::SessionStateNotDirectory(root)
        }
        (StateRootKind::Session, StateRootIssue::NotConfined) => {
            WorkspaceError::SessionStateNotConfined(root)
        }
        #[cfg(unix)]
        (StateRootKind::Session, StateRootIssue::UnsafePermissions) => {
            WorkspaceError::SessionStateUnsafePermissions(root)
        }
        (StateRootKind::DurableEvidence, StateRootIssue::NotAbsolute) => {
            WorkspaceError::DurableEvidenceNotAbsolute(root)
        }
        (StateRootKind::DurableEvidence, StateRootIssue::NotFound) => {
            WorkspaceError::DurableEvidenceNotFound(root)
        }
        (StateRootKind::DurableEvidence, StateRootIssue::NotDirectory) => {
            WorkspaceError::DurableEvidenceNotDirectory(root)
        }
        (StateRootKind::DurableEvidence, StateRootIssue::NotConfined) => {
            WorkspaceError::DurableEvidenceNotConfined(root)
        }
        #[cfg(unix)]
        (StateRootKind::DurableEvidence, StateRootIssue::UnsafePermissions) => {
            WorkspaceError::DurableEvidenceUnsafePermissions(root)
        }
        (StateRootKind::MaterializationLineage, StateRootIssue::NotAbsolute) => {
            WorkspaceError::MaterializationLineageNotAbsolute(root)
        }
        (StateRootKind::MaterializationLineage, StateRootIssue::NotFound) => {
            WorkspaceError::MaterializationLineageNotFound(root)
        }
        (StateRootKind::MaterializationLineage, StateRootIssue::NotDirectory) => {
            WorkspaceError::MaterializationLineageNotDirectory(root)
        }
        (StateRootKind::MaterializationLineage, StateRootIssue::NotConfined) => {
            WorkspaceError::MaterializationLineageNotConfined(root)
        }
        #[cfg(unix)]
        (StateRootKind::MaterializationLineage, StateRootIssue::UnsafePermissions) => {
            WorkspaceError::MaterializationLineageUnsafePermissions(root)
        }
    }
}

impl StateRootKind {
    const fn binding(self) -> &'static str {
        match self {
            Self::Session => "session-state root",
            Self::DurableEvidence => "durable-evidence root",
            Self::MaterializationLineage => "materialization-lineage root",
        }
    }
}

fn verify_state_root(
    filesystem: &dyn MachineFilesystemPort,
    root: &str,
) -> Result<PathBuf, StateRootIssue> {
    let path = Path::new(root);
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(StateRootIssue::NotAbsolute);
    }
    let observation =
        filesystem
            .observe(path)
            .map_err(|error| match (error.operation(), error.kind()) {
                (MachineFilesystemOperation::InspectRequested, std::io::ErrorKind::NotFound) => {
                    StateRootIssue::NotFound
                }
                (
                    MachineFilesystemOperation::Canonicalize
                    | MachineFilesystemOperation::InspectCanonical,
                    std::io::ErrorKind::NotFound,
                ) => StateRootIssue::NotConfined,
                _ => StateRootIssue::Observation(Box::new(error)),
            })?;
    if observation.requested_kind() == MachinePathKind::Symlink {
        return Err(StateRootIssue::NotConfined);
    }
    if observation.canonical_kind() != MachinePathKind::Directory {
        return Err(StateRootIssue::NotDirectory);
    }
    let canonical = observation.canonical_path();
    if canonical != path {
        return Err(StateRootIssue::NotConfined);
    }
    #[cfg(unix)]
    {
        if observation
            .unix_mode()
            .is_some_and(|mode| mode & 0o022 != 0)
        {
            return Err(StateRootIssue::UnsafePermissions);
        }
    }
    Ok(canonical.to_path_buf())
}

/// Verify the dedicated machine-local session-state root without creating it.
///
/// The root must be an absolute canonical directory with no symlink alias and,
/// on Unix, no group or world write bits.
///
/// # Errors
///
/// Returns an activation or confinement error when the binding is absent or the
/// declared root does not satisfy those exact path, type, and permission rules.
pub fn verify_session_state_root(
    filesystem: &dyn MachineFilesystemPort,
    instance: &ResolvedProjectInstanceV1,
) -> Result<PathBuf, WorkspaceError> {
    let binding = instance.session_state.as_ref().ok_or_else(|| {
        WorkspaceError::ActivationRequiresSessionState(instance.machine_id.clone())
    })?;
    verify_state_root(filesystem, &binding.root)
        .map_err(|issue| state_root_error(StateRootKind::Session, issue, &binding.root))
}

/// Verify recoverable evidence storage without creating or repairing it.
///
/// The root must be an absolute canonical directory with no symlink alias and,
/// on Unix, no group or world write bits.
///
/// # Errors
///
/// Returns an activation or confinement error when the binding is absent or the
/// recoverable evidence root fails its path, type, or permission contract.
pub(crate) fn verify_durable_evidence_root(
    filesystem: &dyn MachineFilesystemPort,
    instance: &ResolvedProjectInstanceV1,
) -> Result<PathBuf, WorkspaceError> {
    let binding = instance.durable_evidence.as_ref().ok_or_else(|| {
        WorkspaceError::ActivationRequiresDurableEvidence(instance.machine_id.clone())
    })?;
    verify_state_root(filesystem, &binding.root)
        .map_err(|issue| state_root_error(StateRootKind::DurableEvidence, issue, &binding.root))
}

/// Verify the durable machine-local materialization-lineage root without
/// creating or repairing it, under the same confinement and permission rules as
/// session state.
///
/// # Errors
///
/// Returns a lineage-root error when the binding is absent or its path, type,
/// canonical identity, or Unix permissions are unsafe.
pub fn verify_materialization_lineage_root(
    filesystem: &dyn MachineFilesystemPort,
    instance: &ResolvedProjectInstanceV1,
) -> Result<PathBuf, WorkspaceError> {
    let binding = instance.materialization_lineage.as_ref().ok_or_else(|| {
        WorkspaceError::MaterializationLineageMissing(instance.machine_id.clone())
    })?;
    verify_materialization_lineage_binding(filesystem, binding)
}

/// Verify one already-selected materialization-lineage binding.
///
/// This host-capture form preserves the same confinement law without requiring
/// a composition leaf to retain the semantic resolved-instance record.
///
/// # Errors
///
/// Returns the same lineage-root failures as
/// [`verify_materialization_lineage_root`].
pub fn verify_materialization_lineage_binding(
    filesystem: &dyn MachineFilesystemPort,
    binding: &MaterializationLineageBinding,
) -> Result<PathBuf, WorkspaceError> {
    verify_state_root(filesystem, &binding.root).map_err(|issue| {
        state_root_error(StateRootKind::MaterializationLineage, issue, &binding.root)
    })
}

/// Verify each reading-order pointer as a regular file under its authority root.
///
/// Existing paths are canonicalized before containment is checked, so an
/// authority-relative symlink cannot redirect a pointer outside the root.
///
/// # Errors
///
/// Returns a workspace error when an authority root is unresolved or a declared
/// reading-order file is missing, non-regular, or escapes its exact root.
pub(crate) fn verify_reading_order(
    filesystem: &dyn MachineFilesystemPort,
    instance: &ResolvedProjectInstanceV1,
    session: &SessionModel,
) -> Result<(), WorkspaceError> {
    for entry in &session.reading_order {
        let base = instance
            .authority_roots
            .get(&entry.root_id)
            .ok_or_else(|| WorkspaceError::UnresolvedAuthorityRoot {
                authority_root: entry.root_id.clone(),
                machine: instance.machine_id.clone(),
            })?;
        let root_observation = filesystem.observe(Path::new(base)).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                WorkspaceError::SessionPathMissing {
                    path: base.clone(),
                    binding: entry.root_id.clone(),
                }
            } else {
                WorkspaceError::SessionPathObservation {
                    path: base.clone(),
                    binding: entry.root_id.clone(),
                    source: Box::new(error),
                }
            }
        })?;
        let root = root_observation.canonical_path();
        let exact_file = (root_observation.canonical_kind() == MachinePathKind::RegularFile)
            .then(|| root.to_path_buf());
        let base_dir = match root_observation.canonical_kind() {
            MachinePathKind::Directory => root.to_path_buf(),
            MachinePathKind::RegularFile => {
                root.parent().map(Path::to_path_buf).ok_or_else(|| {
                    WorkspaceError::SessionPathMissing {
                        path: base.clone(),
                        binding: entry.root_id.clone(),
                    }
                })?
            }
            MachinePathKind::Symlink | MachinePathKind::Other => {
                return Err(WorkspaceError::SessionPathMissing {
                    path: base.clone(),
                    binding: entry.root_id.clone(),
                });
            }
        };
        let target_path = base_dir.join(&entry.rel_path);
        let target_observation = filesystem.observe(&target_path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                WorkspaceError::ReadingOrderMissing(entry.rel_path.clone())
            } else {
                WorkspaceError::SessionPathObservation {
                    path: entry.rel_path.clone(),
                    binding: format!("reading-order `{}`", entry.root_id),
                    source: Box::new(error),
                }
            }
        })?;
        let target = target_observation.canonical_path();
        if !target.starts_with(&base_dir)
            || exact_file
                .as_ref()
                .is_some_and(|authority_file| authority_file != target)
        {
            return Err(WorkspaceError::ReadingOrderEscapesRoot {
                path: entry.rel_path.clone(),
                root: entry.root_id.clone(),
            });
        }
        if target_observation.canonical_kind() != MachinePathKind::RegularFile {
            return Err(WorkspaceError::ReadingOrderMissing(entry.rel_path.clone()));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../tests/internal/workspace/instance/authority.rs"]
mod tests;
