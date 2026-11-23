//! Canonical project, execution-workspace, and repository graph laws.

use pale_core::{
    paths::OwnedConfinedRelativePath,
    records::{
        ExecutionPathSelectionIdV1, ExecutionPathSelectionV1, ExecutionWorkspaceRepositoryAccessV1,
        ExecutionWorkspaceRepositoryMembershipV1, LogicalExecutionWorkspaceIdV1,
        LogicalExecutionWorkspaceV1, LogicalProjectIdV1, LogicalProjectV1, LogicalRepositoryIdV1,
        LogicalRepositoryV1, PackageIdV1, PackageResourceIdV1, PackageResourceRefV1,
        ProjectExecutionWorkspaceMembershipV1, ProjectRepositoryMembershipV1, RunPolicyIdV1,
        WorkspaceDesiredTopologyV1,
    },
};

fn topology() -> WorkspaceDesiredTopologyV1 {
    let project_id = LogicalProjectIdV1::parse("pale").unwrap();
    let execution_workspace_id = LogicalExecutionWorkspaceIdV1::parse("engine-work").unwrap();
    let repository_id = LogicalRepositoryIdV1::parse("pale").unwrap();
    let path_id = ExecutionPathSelectionIdV1::parse("direct-terminal").unwrap();
    WorkspaceDesiredTopologyV1 {
        projects: vec![LogicalProjectV1 {
            id: project_id.clone(),
            title: "Pale".into(),
            owner: None,
            run_policy: RunPolicyIdV1::parse("engine-work").unwrap(),
            package_seeds: vec![PackageIdV1::parse("engineering").unwrap()],
            authority_root_ids: vec!["source".into()],
            capabilities_file: OwnedConfinedRelativePath::parse("projects/pale/capabilities.toml")
                .unwrap(),
        }],
        execution_workspaces: vec![LogicalExecutionWorkspaceV1 {
            id: execution_workspace_id.clone(),
            title: "Engine work".into(),
            execution_path_ids: vec![path_id.clone()],
            session: None,
        }],
        repositories: vec![LogicalRepositoryV1 {
            id: repository_id.clone(),
            title: "Pale".into(),
        }],
        execution_paths: vec![ExecutionPathSelectionV1 {
            id: path_id,
            experience_path_id: "engine.direct.terminal".into(),
        }],
        project_repositories: vec![ProjectRepositoryMembershipV1 {
            project_id: project_id.clone(),
            repository_id: repository_id.clone(),
            role: PackageResourceRefV1 {
                package_id: PackageIdV1::parse("engineering").unwrap(),
                resource_id: PackageResourceIdV1::parse("roles.engine").unwrap(),
            },
            stacks: Vec::new(),
            families: Vec::new(),
            responsibility_derivation_ids: Vec::new(),
            materialization_order: 0,
        }],
        project_execution_workspaces: vec![ProjectExecutionWorkspaceMembershipV1 {
            project_id,
            execution_workspace_id: execution_workspace_id.clone(),
        }],
        execution_workspace_repositories: vec![ExecutionWorkspaceRepositoryMembershipV1 {
            execution_workspace_id,
            repository_id,
            access: ExecutionWorkspaceRepositoryAccessV1::Mutate,
        }],
    }
}

#[test]
fn canonical_graph_admits_explicit_relationships() {
    topology().verify().unwrap();
}

#[test]
fn repository_duties_are_bounded_unique_and_canonical() {
    use pale_core::records::ResponsibilityDerivationId;
    let id = |value: &str| ResponsibilityDerivationId::parse(value.to_owned()).unwrap();
    for (duties, message) in [
        (vec![id("a"), id("a")], "repeats"),
        (vec![id("b"), id("a")], "canonical"),
        (
            (0..1025).map(|n| id(&format!("duty-{n:04}"))).collect(),
            "exceeds",
        ),
    ] {
        let mut candidate = topology();
        candidate.project_repositories[0].responsibility_derivation_ids = duties;
        assert!(candidate
            .verify()
            .unwrap_err()
            .to_string()
            .contains(message));
    }
}

#[test]
fn repository_access_is_a_closed_mutation_ceiling() {
    assert!(!ExecutionWorkspaceRepositoryAccessV1::Read.permits_repository_mutation());
    assert!(ExecutionWorkspaceRepositoryAccessV1::Mutate.permits_repository_mutation());
}

#[test]
fn provider_or_machine_presence_cannot_manufacture_a_missing_repository() {
    let mut candidate = topology();
    candidate.repositories.clear();
    let error = candidate.verify().unwrap_err().to_string();
    assert!(error.contains("unknown repository `pale`"));
}

#[test]
fn materialization_order_is_project_local_and_contiguous() {
    let mut candidate = topology();
    candidate.project_repositories[0].materialization_order = 1;
    let error = candidate.verify().unwrap_err().to_string();
    assert!(error.contains("not contiguous from zero"));
}

#[test]
fn one_repository_can_be_reused_without_redeclaration() {
    let mut candidate = topology();
    let second = LogicalProjectIdV1::parse("documentation").unwrap();
    let mut project = candidate.projects[0].clone();
    project.id = second.clone();
    project.title = "Documentation".into();
    candidate.projects.push(project);
    candidate
        .projects
        .sort_by(|left, right| left.id.cmp(&right.id));
    let mut membership = candidate.project_repositories[0].clone();
    membership.project_id = second;
    candidate.project_repositories.push(membership);
    candidate.project_repositories.sort_by(|left, right| {
        (&left.project_id, &left.repository_id).cmp(&(&right.project_id, &right.repository_id))
    });
    candidate.verify().unwrap();
    assert_eq!(candidate.repositories.len(), 1);
    assert_eq!(candidate.project_repositories.len(), 2);
}

#[test]
fn repeated_identity_is_rejected_even_when_descriptive_fields_differ() {
    let mut candidate = topology();
    let mut duplicate = candidate.projects[0].clone();
    duplicate.title = "A second description cannot redefine the project".into();
    candidate.projects.push(duplicate);
    let error = candidate.verify().unwrap_err().to_string();
    assert!(error.contains("repeats a project member"));
}
