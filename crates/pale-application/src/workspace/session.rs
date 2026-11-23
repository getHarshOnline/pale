//! Session working-root and authority-directory resolution.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use pale_core::records::{
    AuthorityAccessClass, EffectiveProjectProfileV1, LogicalRepositoryIdV1, ProfileRepository,
    ResolvedProjectInstanceV1, SessionModel,
};
use pale_ports::{MachineFilesystemError, MachineFilesystemPort, MachinePathKind};

use super::WorkspaceError;

fn session_path_observation_error(
    error: MachineFilesystemError,
    path: String,
    binding: String,
) -> WorkspaceError {
    if error.kind() == std::io::ErrorKind::NotFound {
        WorkspaceError::SessionPathMissing { path, binding }
    } else {
        WorkspaceError::SessionPathObservation {
            path,
            binding,
            source: Box::new(error),
        }
    }
}

fn canonical_session_path(
    observation: &pale_ports::MachinePathObservation,
    path: String,
    binding: String,
) -> Result<String, WorkspaceError> {
    observation
        .canonical_path()
        .to_str()
        .map(str::to_owned)
        .ok_or(WorkspaceError::SessionPathNotUtf8 { path, binding })
}

#[derive(Clone, Copy)]
enum ReadonlyPathPolicy {
    Directory,
    DirectoryOrFile,
}

fn resolve_readonly_path(
    filesystem: &dyn MachineFilesystemPort,
    path: &str,
    binding: String,
    policy: ReadonlyPathPolicy,
) -> Result<String, WorkspaceError> {
    let observation = filesystem
        .observe(Path::new(path))
        .map_err(|error| session_path_observation_error(error, path.to_owned(), binding.clone()))?;
    let kind_is_admissible = match policy {
        ReadonlyPathPolicy::Directory => observation.canonical_kind() == MachinePathKind::Directory,
        ReadonlyPathPolicy::DirectoryOrFile => matches!(
            observation.canonical_kind(),
            MachinePathKind::Directory | MachinePathKind::RegularFile
        ),
    };
    if !kind_is_admissible {
        return Err(WorkspaceError::SessionPathMissing {
            path: path.to_owned(),
            binding,
        });
    }
    canonical_session_path(&observation, path.to_owned(), binding)
}

/// Validate the portable session model before machine paths are resolved.
pub(super) fn validate_session_model(
    session: Option<&SessionModel>,
    authority_root_ids: &[String],
    repositories: &[ProfileRepository],
) -> Result<(), WorkspaceError> {
    let Some(session) = session else {
        return Ok(());
    };
    let repository_ids = repositories
        .iter()
        .map(|repository| &repository.id)
        .collect::<BTreeSet<_>>();
    if session.activation_roots.is_empty() {
        return Err(WorkspaceError::EmptyActivationRoots);
    }
    let mut activation_roots = BTreeSet::new();
    for root in &session.activation_roots {
        if !repository_ids.contains(root) {
            return Err(WorkspaceError::UnknownActivationRoot {
                repository: root.clone(),
            });
        }
        if !activation_roots.insert(root) {
            return Err(WorkspaceError::DuplicateActivationRoot {
                repository: root.clone(),
            });
        }
    }
    let mut additional_roots = BTreeSet::new();
    for root in &session.additional_roots {
        if !repository_ids.contains(root) {
            return Err(WorkspaceError::UnknownAdditionalRoot {
                repository: root.clone(),
            });
        }
        if activation_roots.contains(root) {
            return Err(WorkspaceError::ActivationAdditionalRootOverlap {
                repository: root.clone(),
            });
        }
        if !additional_roots.insert(root) {
            return Err(WorkspaceError::DuplicateAdditionalRoot {
                repository: root.clone(),
            });
        }
    }

    let authority_ids = authority_root_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for access in &session.authority_access {
        if !authority_ids.contains(access.root_id.as_str()) {
            return Err(WorkspaceError::SessionUnknownAuthority(
                access.root_id.clone(),
            ));
        }
    }
    for entry in &session.reading_order {
        if !authority_ids.contains(entry.root_id.as_str()) {
            return Err(WorkspaceError::SessionUnknownAuthority(
                entry.root_id.clone(),
            ));
        }
        if pale_core::paths::ConfinedRelativePath::parse(&entry.rel_path).is_none() {
            return Err(WorkspaceError::ReadingOrderNotConfined(
                entry.rel_path.clone(),
            ));
        }
    }

    Ok(())
}

fn readonly_repository_bindings<'a>(
    repositories: &'a [ProfileRepository],
    checkout_paths: &'a BTreeMap<LogicalRepositoryIdV1, String>,
    machine_id: &str,
) -> Result<Vec<(&'a ProfileRepository, &'a str)>, WorkspaceError> {
    repositories
        .iter()
        .filter(|repository| !repository.access.permits_repository_mutation())
        .map(|repository| {
            checkout_paths
                .get(&repository.id)
                .map(String::as_str)
                .map(|path| (repository, path))
                .ok_or_else(|| WorkspaceError::UnresolvedRepository {
                    repository: repository.id.clone(),
                    machine: machine_id.to_owned(),
                })
        })
        .collect()
}

/// Return declared checkout paths whose sealed execution-workspace edge is read-only.
///
/// This projection performs no filesystem admission. Portable compilation and
/// removal reconstruction may render it, but cannot treat it as machine proof.
pub(crate) fn declared_readonly_repository_paths(
    profile: &EffectiveProjectProfileV1,
    instance: &ResolvedProjectInstanceV1,
) -> Result<Vec<String>, WorkspaceError> {
    let mut paths = readonly_repository_bindings(
        &profile.repositories,
        &instance.repositories,
        &instance.machine_id,
    )?
    .into_iter()
    .map(|(_, path)| path.to_owned())
    .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    Ok(paths)
}

/// Resolve canonical directory roots for read-only repository memberships.
///
/// The access decision comes only from the sealed profile. The instance
/// contributes the selected checkout path but cannot widen
/// [`ProfileRepository::access`] or create a missing membership.
pub(crate) fn resolve_readonly_repository_paths(
    filesystem: &dyn MachineFilesystemPort,
    profile: &EffectiveProjectProfileV1,
    instance: &ResolvedProjectInstanceV1,
) -> Result<Vec<String>, WorkspaceError> {
    let mut paths = Vec::new();
    for (repository, declared) in readonly_repository_bindings(
        &profile.repositories,
        &instance.repositories,
        &instance.machine_id,
    )? {
        let binding = format!("read-only repository `{}`", repository.id);
        paths.push(resolve_readonly_path(
            filesystem,
            declared,
            binding,
            ReadonlyPathPolicy::Directory,
        )?);
    }
    Ok(minimal_readonly_path_cover(paths))
}

#[cfg(test)]
#[path = "../../tests/internal/workspace/session.rs"]
mod tests;

/// Reduce exact read-only paths to the same sorted minimal prefix cover.
pub(crate) fn minimal_readonly_path_cover(paths: Vec<String>) -> Vec<String> {
    let mut paths = paths.into_iter().map(PathBuf::from).collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    let mut minimal = Vec::<PathBuf>::new();
    for path in paths {
        if minimal.iter().any(|root| path.starts_with(root)) {
            continue;
        }
        minimal.push(path);
    }
    minimal
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

/// Return sorted declared paths for authority roots granted `read-only` access.
///
/// This projection performs no filesystem admission. It exists only for portable
/// rendering and removal reconstruction, both of which remain machine-unverified.
pub(crate) fn declared_readonly_authority_paths(
    instance: &ResolvedProjectInstanceV1,
    session: &SessionModel,
) -> Result<Vec<String>, WorkspaceError> {
    let mut paths = Vec::new();
    for access in session
        .authority_access
        .iter()
        .filter(|access| access.class == AuthorityAccessClass::ReadOnly)
    {
        let path = instance
            .authority_roots
            .get(&access.root_id)
            .ok_or_else(|| WorkspaceError::UnresolvedAuthorityRoot {
                authority_root: access.root_id.clone(),
                machine: instance.machine_id.clone(),
            })?;
        paths.push(path.clone());
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

/// Resolve exact read-only authority paths for a machine-admitted hook seal.
///
/// Directory roots protect their descendants. File roots remain exact files and
/// are never widened to their containing directory. Canonical paths sever the
/// runtime boundary from a mutable symlink spelling observed during compilation.
pub(crate) fn resolve_readonly_authority_paths(
    filesystem: &dyn MachineFilesystemPort,
    instance: &ResolvedProjectInstanceV1,
    session: &SessionModel,
) -> Result<Vec<String>, WorkspaceError> {
    let declared = declared_readonly_authority_paths(instance, session)?;
    let mut paths = Vec::with_capacity(declared.len());
    for path in declared {
        let binding = "read-only authority".to_string();
        paths.push(resolve_readonly_path(
            filesystem,
            &path,
            binding,
            ReadonlyPathPolicy::DirectoryOrFile,
        )?);
    }
    Ok(minimal_readonly_path_cover(paths))
}
