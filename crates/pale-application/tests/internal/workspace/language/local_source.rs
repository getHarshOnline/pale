//! Exercise local owner/duty source without any release or publisher declarations.

use super::*;
use crate::workspace::{lower_local_workspace_source, WorkspaceSourceLowering};

fn write(root: &Path) -> WorkspaceManifestFile {
    let mut manifest: WorkspaceManifestFile = toml::from_str(include_str!(
        "../../../fixtures/workspace-language/empty-adopter/pale-workspace.toml"
    ))
    .unwrap();
    manifest.workspace.module_sources = vec![rel("local.toml")];
    manifest.workspace.source_components = vec![rel("content.toml")];
    manifest.workspace.responsibility_derivations = vec![rel("duty.toml")];
    let topology = &mut manifest.workspace.desired_topology;
    topology.projects.push(LogicalProjectV1 {
        id: LogicalProjectIdV1::parse("project").unwrap(),
        title: "Project".into(),
        owner: Some(ProjectOwnerReferenceV1::Principal {
            module_id: module_id("local"),
            resource_id: WorkspaceResourceId::parse("owner").unwrap(),
        }),
        run_policy: RunPolicyIdV1::parse("standard").unwrap(),
        package_seeds: Vec::new(),
        authority_root_ids: Vec::new(),
        capabilities_file: rel("unread-capabilities.toml"),
    });
    topology.repositories.push(LogicalRepositoryV1 {
        id: LogicalRepositoryIdV1::parse("source").unwrap(),
        title: "Source".into(),
    });
    topology
        .project_repositories
        .push(ProjectRepositoryMembershipV1 {
            project_id: topology.projects[0].id.clone(),
            repository_id: topology.repositories[0].id.clone(),
            role: PackageResourceRefV1 {
                package_id: PackageIdV1::parse("engineering").unwrap(),
                resource_id: PackageResourceIdV1::parse("change").unwrap(),
            },
            stacks: Vec::new(),
            families: Vec::new(),
            materialization_order: 0,
            responsibility_derivation_ids: vec![ResponsibilityDerivationId::parse(
                "responsibility.local",
            )
            .unwrap()],
        });
    let component = WorkspaceLanguagePrimitivesV1 {
        schema: "pale/workspace-language-primitives.v1".into(),
        schema_version: 1,
        component_id: WorkspaceComponentId::parse("content").unwrap(),
        module_id: module_id("local"),
        declarations: Vec::new(),
    };
    let module = WorkspaceModuleSourceV1 {
        schema: "pale/workspace-module-source.v1".into(),
        schema_version: 1,
        module_id: module_id("local"),
        source_components: vec![component.component_id.clone()],
        imports: Vec::new(),
        exports: Vec::new(),
        extensions: Vec::new(),
        resources: vec![WorkspaceResourceSelectionV1::Principal {
            resource_id: WorkspaceResourceId::parse("owner").unwrap(),
            authority_resolver: WorkspaceResourceId::parse("owner-authority").unwrap(),
        }],
    };
    let mut duty = responsibility_fixtures::responsibility_derivation("local", 0, false);
    duty.publisher_module = responsibility_fixtures::omitted_answer("unpublished");
    duty.adopter_module = responsibility_fixtures::omitted_answer("unadopted");
    let resolution = WorkspaceResolutionLockV1::seal(Vec::new()).unwrap();
    let universe = WorkspaceUniverseLockV1::seal(
        manifest.workspace.workspace_id.clone(),
        resolution.resolution_digest.clone(),
        Vec::new(),
    )
    .unwrap();
    for (path, text) in [
        ("pale-workspace.toml", toml::to_string(&manifest).unwrap()),
        ("local.toml", toml::to_string(&module).unwrap()),
        ("content.toml", toml::to_string(&component).unwrap()),
        ("duty.toml", toml::to_string(&duty).unwrap()),
        (
            "resolution.lock.toml",
            toml::to_string(&resolution).unwrap(),
        ),
        ("universe.lock.toml", toml::to_string(&universe).unwrap()),
    ] {
        fs::write(root.join(path), text).unwrap();
    }
    manifest
}

fn inspect(root: &Path) -> Result<WorkspaceSourceLowering, WorkspaceError> {
    lower_local_workspace_source(
        &pale_adapter_fs::LocalSourceFilesystem,
        &root.join("pale-workspace.toml"),
    )
}

#[test]
fn unpublished_owner_and_duty_lower_without_machine_or_package_files_but_cannot_be_admitted() {
    let root = tempfile::tempdir().unwrap();
    write(root.path());
    let lowered = inspect(root.path()).unwrap();
    let json: serde_json::Value =
        serde_json::from_slice(&lowered.canonical_bytes().unwrap()).unwrap();
    assert_eq!(json["phase"], "source_only");
    assert_eq!(json["modules"][0]["module_id"], "local");
    assert_eq!(
        json["responsibilities"]["derivations"][0]["publisher_module"]["disposition"],
        "omitted"
    );
    assert!(json["modules"][0].get("publisher").is_none());
    assert!(
        matches!(load_link_normalize(root.path()), Err(WorkspaceError::NamespaceResolution(message)) if message.contains("source-only"))
    );
}

#[test]
fn whitespace_changes_evidence_but_not_lowered_meaning() {
    let root = tempfile::tempdir().unwrap();
    write(root.path());
    let before = inspect(root.path()).unwrap();
    let file = root.path().join("local.toml");
    fs::write(
        &file,
        format!("# Formatting only\n{}", fs::read_to_string(&file).unwrap()),
    )
    .unwrap();
    let after = inspect(root.path()).unwrap();
    assert_eq!(before.content_digest(), after.content_digest());
    assert_ne!(
        before.canonical_bytes().unwrap(),
        after.canonical_bytes().unwrap()
    );
}

#[test]
fn unknown_owner_duty_and_foreign_lock_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    for case in ["owner", "duty", "lock"] {
        let mut manifest = write(root.path());
        match case {
            "owner" => {
                manifest.workspace.desired_topology.projects[0].owner =
                    Some(ProjectOwnerReferenceV1::Organization {
                        module_id: module_id("missing"),
                        resource_id: WorkspaceResourceId::parse("owner").unwrap(),
                    })
            }
            "duty" => {
                manifest.workspace.desired_topology.project_repositories[0]
                    .responsibility_derivation_ids =
                    vec![ResponsibilityDerivationId::parse("missing").unwrap()]
            }
            _ => manifest.workspace.workspace_id = WorkspaceId::parse("different").unwrap(),
        }
        fs::write(
            root.path().join("pale-workspace.toml"),
            toml::to_string(&manifest).unwrap(),
        )
        .unwrap();
        assert!(inspect(root.path()).is_err(), "{case}");
    }
}

#[cfg(unix)]
#[test]
fn local_module_symlink_cannot_escape_the_manifest_root() {
    let root = tempfile::tempdir().unwrap();
    write(root.path());
    let outside = tempfile::tempdir().unwrap();
    fs::rename(
        root.path().join("local.toml"),
        outside.path().join("local.toml"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("local.toml"),
        root.path().join("local.toml"),
    )
    .unwrap();
    assert!(inspect(root.path()).is_err());
}
