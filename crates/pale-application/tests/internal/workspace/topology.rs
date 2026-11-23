//! Canonical desired-topology selection laws.

use pale_core::paths::OwnedConfinedRelativePath;
use pale_core::records::{
    ExecutionPathSelectionIdV1, ExecutionPathSelectionV1, ExecutionWorkspaceRepositoryAccessV1,
    ExecutionWorkspaceRepositoryMembershipV1, LogicalExecutionWorkspaceIdV1,
    LogicalExecutionWorkspaceV1, LogicalProjectIdV1, LogicalProjectV1, LogicalRepositoryIdV1,
    LogicalRepositoryV1, PackageIdV1, PackageResourceIdV1, PackageResourceRefV1,
    ProjectExecutionWorkspaceMembershipV1, ProjectRepositoryMembershipV1, RunPolicyIdV1,
    WorkspaceDesiredTopologyV1,
};

use super::super::{select_workspace_topology, WorkspaceError};

fn topology(with_project_workspace_edge: bool) -> WorkspaceDesiredTopologyV1 {
    let project_id = LogicalProjectIdV1::parse("project").unwrap();
    let workspace_id = LogicalExecutionWorkspaceIdV1::parse("work").unwrap();
    let first = LogicalRepositoryIdV1::parse("first").unwrap();
    let second = LogicalRepositoryIdV1::parse("second").unwrap();
    let role = |resource: &str| PackageResourceRefV1 {
        package_id: PackageIdV1::parse("example.engine").unwrap(),
        resource_id: PackageResourceIdV1::parse(resource).unwrap(),
    };
    WorkspaceDesiredTopologyV1 {
        projects: vec![LogicalProjectV1 {
            id: project_id.clone(),
            title: "Project".into(),
            owner: None,
            run_policy: RunPolicyIdV1::parse("standard").unwrap(),
            package_seeds: vec![PackageIdV1::parse("example.engine").unwrap()],
            authority_root_ids: Vec::new(),
            capabilities_file: OwnedConfinedRelativePath::parse("capabilities.toml").unwrap(),
        }],
        execution_workspaces: vec![LogicalExecutionWorkspaceV1 {
            id: workspace_id.clone(),
            title: "Work".into(),
            execution_path_ids: vec![ExecutionPathSelectionIdV1::parse("codex").unwrap()],
            session: None,
        }],
        repositories: vec![
            LogicalRepositoryV1 {
                id: first.clone(),
                title: "First".into(),
            },
            LogicalRepositoryV1 {
                id: second.clone(),
                title: "Second".into(),
            },
        ],
        execution_paths: vec![ExecutionPathSelectionV1 {
            id: ExecutionPathSelectionIdV1::parse("codex").unwrap(),
            experience_path_id: "direct-codex".into(),
        }],
        project_repositories: vec![
            ProjectRepositoryMembershipV1 {
                project_id: project_id.clone(),
                repository_id: first,
                role: role("first"),
                stacks: Vec::new(),
                families: Vec::new(),
                responsibility_derivation_ids: Vec::new(),
                materialization_order: 0,
            },
            ProjectRepositoryMembershipV1 {
                project_id: project_id.clone(),
                repository_id: second.clone(),
                role: role("second"),
                stacks: Vec::new(),
                families: Vec::new(),
                responsibility_derivation_ids: Vec::new(),
                materialization_order: 1,
            },
        ],
        project_execution_workspaces: with_project_workspace_edge
            .then_some(ProjectExecutionWorkspaceMembershipV1 {
                project_id,
                execution_workspace_id: workspace_id.clone(),
            })
            .into_iter()
            .collect(),
        execution_workspace_repositories: vec![ExecutionWorkspaceRepositoryMembershipV1 {
            execution_workspace_id: workspace_id,
            repository_id: second,
            access: ExecutionWorkspaceRepositoryAccessV1::Read,
        }],
    }
}

#[test]
fn repository_selection_is_the_declared_project_workspace_intersection() {
    let selected = select_workspace_topology(&topology(true), "project", "work").unwrap();
    assert_eq!(selected.project.id.as_str(), "project");
    assert_eq!(selected.execution_workspace.id.as_str(), "work");
    assert_eq!(selected.execution_paths.len(), 1);
    assert_eq!(selected.repositories.len(), 1);
    assert_eq!(
        selected.repositories[0].project.repository_id.as_str(),
        "second"
    );
    assert_eq!(selected.repositories[0].project.materialization_order, 1);
    assert_eq!(
        selected.repositories[0].access,
        ExecutionWorkspaceRepositoryAccessV1::Read
    );
}

#[test]
fn project_workspace_membership_is_required_explicitly() {
    assert!(matches!(
        select_workspace_topology(&topology(false), "project", "work"),
        Err(WorkspaceError::ProjectExecutionWorkspaceMembership {
            project,
            execution_workspace,
        }) if project == "project" && execution_workspace == "work"
    ));
}

#[test]
fn selected_owner_and_repository_duties_change_identity_without_widening_access() {
    use pale_core::records::{
        ProjectOwnerReferenceV1, ResponsibilityDerivationId, WorkspaceModuleId, WorkspaceResourceId,
    };
    let base = select_workspace_topology(&topology(true), "project", "work").unwrap();
    let mut candidate = base.clone();
    candidate.project.owner = Some(ProjectOwnerReferenceV1::Principal {
        module_id: WorkspaceModuleId::parse("person").unwrap(),
        resource_id: WorkspaceResourceId::parse("self").unwrap(),
    });
    let digest = super::super::selected_topology_digest(&base).unwrap();
    let owner_digest = super::super::selected_topology_digest(&candidate).unwrap();
    assert_ne!(digest, owner_digest);
    candidate.repositories[0]
        .project
        .responsibility_derivation_ids =
        vec![ResponsibilityDerivationId::parse("verify-source").unwrap()];
    assert_ne!(
        owner_digest,
        super::super::selected_topology_digest(&candidate).unwrap()
    );
    assert_eq!(
        candidate.repositories[0].access,
        base.repositories[0].access
    );
    assert_eq!(candidate.execution_workspace, base.execution_workspace);
}
