//! Canonical selection from one principal-owned desired workspace topology.
//!
//! Selection is an intersection over declared relationships. Machine checkout
//! and provider projections are deliberately absent: they may resolve selected
//! repository identities later, but they cannot create topology.

use std::collections::BTreeMap;

use pale_core::records::{
    ExecutionPathSelectionV1, ExecutionWorkspaceRepositoryAccessV1, LogicalExecutionWorkspaceV1,
    LogicalProjectV1, ProjectRepositoryMembershipV1, WorkspaceDesiredTopologyV1,
};

use super::WorkspaceError;

/// Project-owned semantics combined with one execution-workspace access ceiling.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(crate) struct SelectedRepositoryMembership {
    pub(crate) project: ProjectRepositoryMembershipV1,
    pub(crate) access: ExecutionWorkspaceRepositoryAccessV1,
}

/// Source-free desired declarations selected for one compiler entry.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(crate) struct SelectedWorkspaceTopology {
    pub(crate) project: LogicalProjectV1,
    pub(crate) execution_workspace: LogicalExecutionWorkspaceV1,
    pub(crate) execution_paths: Vec<ExecutionPathSelectionV1>,
    pub(crate) repositories: Vec<SelectedRepositoryMembership>,
}

/// Hash one selected desired-topology entry without private or Package inputs.
pub(crate) fn selected_topology_digest(
    selection: &SelectedWorkspaceTopology,
) -> Result<String, WorkspaceError> {
    selected_topology_digest_from_parts(
        &selection.project,
        &selection.execution_workspace,
        &selection.execution_paths,
        &selection.repositories,
    )
}

/// Hash one selected desired-topology entry from its canonical parts.
pub(crate) fn selected_topology_digest_from_parts(
    project: &LogicalProjectV1,
    execution_workspace: &LogicalExecutionWorkspaceV1,
    execution_paths: &[ExecutionPathSelectionV1],
    repositories: &[SelectedRepositoryMembership],
) -> Result<String, WorkspaceError> {
    pale_core::canon::digest(&(
        "pale.selected-workspace-topology.v1",
        project,
        execution_workspace,
        execution_paths,
        repositories,
    ))
    .map_err(WorkspaceError::Canon)
}

/// Select one project and execution workspace from the verified desired graph.
///
/// Repository selection is the explicit intersection of project membership and
/// execution-workspace access. Repository semantics and materialization order
/// remain project-owned; the workspace edge contributes availability only.
pub(crate) fn select_workspace_topology(
    topology: &WorkspaceDesiredTopologyV1,
    project_id: &str,
    execution_workspace_id: &str,
) -> Result<SelectedWorkspaceTopology, WorkspaceError> {
    topology
        .verify()
        .map_err(WorkspaceError::InvalidDesiredTopology)?;

    let project = topology
        .projects
        .iter()
        .find(|project| project.id.as_str() == project_id)
        .cloned()
        .ok_or_else(|| WorkspaceError::UnknownProject(project_id.to_owned()))?;
    let execution_workspace = topology
        .execution_workspaces
        .iter()
        .find(|workspace| workspace.id.as_str() == execution_workspace_id)
        .cloned()
        .ok_or_else(|| {
            WorkspaceError::UnknownExecutionWorkspace(execution_workspace_id.to_owned())
        })?;

    let declared_pair = topology.project_execution_workspaces.iter().any(|edge| {
        edge.project_id == project.id && edge.execution_workspace_id == execution_workspace.id
    });
    if !declared_pair {
        return Err(WorkspaceError::ProjectExecutionWorkspaceMembership {
            project: project.id.to_string(),
            execution_workspace: execution_workspace.id.to_string(),
        });
    }

    let workspace_repositories = topology
        .execution_workspace_repositories
        .iter()
        .filter(|edge| edge.execution_workspace_id == execution_workspace.id)
        .map(|edge| (&edge.repository_id, edge.access))
        .collect::<BTreeMap<_, _>>();
    let mut repositories = topology
        .project_repositories
        .iter()
        .filter_map(|edge| {
            (edge.project_id == project.id)
                .then(|| workspace_repositories.get(&edge.repository_id).copied())
                .flatten()
                .map(|access| SelectedRepositoryMembership {
                    project: edge.clone(),
                    access,
                })
        })
        .collect::<Vec<_>>();
    repositories.sort_by_key(|edge| edge.project.materialization_order);

    let paths = topology
        .execution_paths
        .iter()
        .map(|path| (&path.id, path))
        .collect::<BTreeMap<_, _>>();
    let execution_paths = execution_workspace
        .execution_path_ids
        .iter()
        .map(|id| {
            (*paths
                .get(id)
                .expect("desired-topology verification proved every execution-path reference"))
            .clone()
        })
        .collect();

    Ok(SelectedWorkspaceTopology {
        project,
        execution_workspace,
        execution_paths,
        repositories,
    })
}
