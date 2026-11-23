//! Typed loaders for the private workspace input files.

use std::path::{Path, PathBuf};

use pale_core::records::{
    AdmittedLifecycleBehaviorV1, CapabilitiesFile, DistributionLockFile, ExecutionPathSelectionV1,
    LifecycleBehavior, LogicalExecutionWorkspaceV1, LogicalProjectV1, MachineBindingFile,
    WorkspaceId,
};
use pale_ports::SourceFilesystemPort;

use super::source::{parse_workspace_source, ParsedWorkspaceSource};
use super::source_admission::{
    admit_toml, read_utf8_source, SourceBackedWorkspaceDocuments, WorkspaceDocumentSource,
};
use super::{ResolvedEntry, SelectedRepositoryMembership, WorkspaceError};

/// Typed workspace records captured for one compilation.
///
/// Callers may retain paths for provenance, but compilation consumes these values
/// rather than reopening configuration files. Capture detects a differing second
/// read; it is not an atomic multi-file filesystem snapshot.
#[derive(Debug)]
pub struct WorkspaceInputSnapshot {
    /// Distribution root resolved from the same canonical workspace entry.
    pub(crate) distribution_root: PathBuf,
    /// Principal workspace owning the selected desired topology.
    pub(crate) workspace_id: WorkspaceId,
    /// Selected canonical project declaration.
    pub(crate) project: LogicalProjectV1,
    /// Selected canonical execution-workspace declaration.
    pub(crate) execution_workspace: LogicalExecutionWorkspaceV1,
    /// Neutral execution paths resolved exactly once from the selected workspace.
    pub(crate) execution_paths: Vec<ExecutionPathSelectionV1>,
    /// Project-owned repository semantics admitted by the selected workspace.
    pub(crate) repositories: Vec<SelectedRepositoryMembership>,
    /// Project-local capability provider bindings.
    pub(crate) capabilities: CapabilitiesFile,
    /// Private lifecycle mechanics with source guidance materialized once.
    pub(crate) lifecycle: Vec<AdmittedLifecycleBehaviorV1>,
    /// Selected machine-local executable, authority, and state bindings.
    pub(crate) machine: MachineBindingFile,
    /// Distribution compiler, package, and standalone-catalog pins.
    pub(crate) lock: DistributionLockFile,
    /// Stable portable workspace-language source captured from the same root.
    pub(crate) workspace_source: ParsedWorkspaceSource,
}

/// Build one coherent input snapshot from already parsed workspace records.
pub(super) fn load_input_snapshot_from_records(
    sources: &dyn SourceFilesystemPort,
    entry: ResolvedEntry,
) -> Result<WorkspaceInputSnapshot, WorkspaceError> {
    let workspace_documents = SourceBackedWorkspaceDocuments::new(sources, &entry.workspace_root);
    let workspace_source =
        parse_workspace_source(&workspace_documents, &entry.workspace_manifest_path)?;
    if workspace_source.manifest != entry.workspace_manifest {
        return Err(WorkspaceError::InputChanged(
            entry.workspace_manifest_path.display().to_string(),
        ));
    }
    let capabilities = load_capabilities(
        &workspace_documents,
        &entry.workspace_root,
        &entry.selection.project,
    )?;
    let lock = load_lock(&workspace_documents, &entry.lock_path)?;
    let workspace_id = entry.workspace_manifest.workspace.workspace_id.clone();
    let workspace_root = entry.workspace_root;
    let snapshot = WorkspaceInputSnapshot {
        distribution_root: entry.distribution_root,
        workspace_id,
        project: entry.selection.project,
        execution_workspace: entry.selection.execution_workspace,
        execution_paths: entry.selection.execution_paths,
        repositories: entry.selection.repositories,
        capabilities: capabilities.record,
        lifecycle: capabilities.lifecycle,
        machine: entry.machine,
        lock,
        workspace_source,
    };
    let workspace_documents = SourceBackedWorkspaceDocuments::new(sources, &workspace_root);
    let stable_workspace =
        parse_workspace_source(&workspace_documents, &entry.workspace_manifest_path)
            .is_ok_and(|value| value == snapshot.workspace_source);
    let stable = stable_workspace
        && load_capabilities(&workspace_documents, &workspace_root, &snapshot.project).is_ok_and(
            |value| value.record == snapshot.capabilities && value.lifecycle == snapshot.lifecycle,
        )
        && load_machine(&workspace_documents, &entry.machine_path)
            .is_ok_and(|value| value == snapshot.machine)
        && load_lock(&workspace_documents, &entry.lock_path)
            .is_ok_and(|value| value == snapshot.lock);
    if !stable {
        return Err(WorkspaceError::InputChanged(
            workspace_root.display().to_string(),
        ));
    }
    Ok(snapshot)
}

/// Load the capability record referenced by the selected project.
pub(super) fn load_capabilities(
    sources: &(impl WorkspaceDocumentSource + ?Sized),
    dir: &Path,
    project: &LogicalProjectV1,
) -> Result<LoadedCapabilities, WorkspaceError> {
    let path = dir.join(project.capabilities_file.as_str());
    let record: CapabilitiesFile = admit_toml(sources, &path, "capabilities.v1", "capabilities")?;
    let owner = path
        .parent()
        .ok_or_else(|| WorkspaceError::MissingParent(path.clone()))?;
    load_capabilities_from_record(sources, owner, record)
}

/// Materialize lifecycle guidance for one already schema-admitted capability record.
pub(super) fn load_capabilities_from_record(
    sources: &(impl WorkspaceDocumentSource + ?Sized),
    dir: &Path,
    record: CapabilitiesFile,
) -> Result<LoadedCapabilities, WorkspaceError> {
    let lifecycle = admit_lifecycle(sources, dir, &record)?;
    Ok(LoadedCapabilities { record, lifecycle })
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct LoadedCapabilities {
    pub(super) record: CapabilitiesFile,
    pub(super) lifecycle: Vec<AdmittedLifecycleBehaviorV1>,
}

fn admit_lifecycle(
    sources: &(impl WorkspaceDocumentSource + ?Sized),
    owner: &Path,
    capabilities: &CapabilitiesFile,
) -> Result<Vec<AdmittedLifecycleBehaviorV1>, WorkspaceError> {
    let Some(catalog) = &capabilities.lifecycle else {
        return Ok(Vec::new());
    };
    let mut admitted = catalog
        .behaviours
        .iter()
        .map(|behaviour| {
            let projected = catalog.project(behaviour);
            projected.validate_contract().map_err(|source| {
                WorkspaceError::InvalidLifecycleBehavior {
                    id: projected.id.as_str().to_owned(),
                    source,
                }
            })?;
            let guidance = projected
                .guidance
                .as_ref()
                .map(|relative| read_utf8_source(sources, &owner.join(relative.as_str())))
                .transpose()?;
            let LifecycleBehavior {
                id,
                description,
                guidance: _,
                event,
                predicate_kind,
                matchers,
                case_insensitive,
                existing_targets_only,
                allow_admitted_operation,
                action,
                activate_mode,
                modes,
                tool_reqs,
                failure_posture,
                state_scope,
                governed_by,
                timeout_ms,
                fallback,
            } = projected;
            Ok(AdmittedLifecycleBehaviorV1 {
                id,
                description,
                guidance,
                event,
                predicate_kind,
                matchers,
                case_insensitive,
                existing_targets_only,
                allow_admitted_operation,
                action,
                activate_mode,
                modes,
                tool_reqs,
                failure_posture,
                state_scope,
                governed_by,
                timeout_ms,
                fallback,
            })
        })
        .collect::<Result<Vec<_>, WorkspaceError>>()?;
    admitted.sort_by(|left, right| left.id.cmp(&right.id));
    for pair in admitted.windows(2) {
        if pair[0].id == pair[1].id {
            return Err(WorkspaceError::DuplicateLifecycleBehavior(
                pair[0].id.as_str().to_owned(),
            ));
        }
    }
    Ok(admitted)
}

/// Load a machine binding file.
pub(super) fn load_machine(
    sources: &(impl WorkspaceDocumentSource + ?Sized),
    path: &Path,
) -> Result<MachineBindingFile, WorkspaceError> {
    let machine: MachineBindingFile =
        admit_toml(sources, path, "machine-binding.v1", "machine binding")?;
    if machine.native_harnesses.windows(2).any(|pair| {
        pair[0]
            .provider_binding
            .canonical_cmp(&pair[1].provider_binding)
            != std::cmp::Ordering::Less
    }) {
        return Err(WorkspaceError::UnsortedOrDuplicateNativeHarnessBindings);
    }
    Ok(machine)
}

/// Load the distribution lock.
pub(super) fn load_lock(
    sources: &(impl WorkspaceDocumentSource + ?Sized),
    path: &Path,
) -> Result<DistributionLockFile, WorkspaceError> {
    admit_toml(sources, path, "distribution-lock.v1", "distribution lock")
}
