//! Shared source-backed compilation fixture for application tests.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pale_core::records::{
    DistributionLockFile, DistributionMeta, LogicalRepositoryIdV1, MachineBindingFile, MachineMeta,
    RepositoryCheckoutBindingV1,
};

use super::{inspect_distribution, resolve_entry, CompileError, WorkspaceInputSnapshot};

/// Minimal source-backed project shared by compiler and resident tests.
pub(crate) struct SyntheticWorkspace {
    pub(crate) root: tempfile::TempDir,
    pub(crate) manifest: PathBuf,
    pub(crate) machine_binding: PathBuf,
}

impl SyntheticWorkspace {
    pub(crate) fn new(with_unselected: bool) -> Self {
        let root = tempfile::tempdir().expect("temporary workspace");
        let distribution = root.path().join("distribution");
        super::role_package_fixture::write_role_package(
            &distribution,
            "engine",
            "example.engine",
            "# Engine\n\nExact retained role body.\n",
        );
        if with_unselected {
            super::package_fixtures::write_policy_package(
                &distribution,
                "unused",
                "example.unused",
                "# Unselected\n\nThis body is available but not used.\n",
            );
        }

        let workspace = root.path().join("workspace");
        let machines = workspace.join("machines");
        let repository = root.path().join("repository");
        let authority = root.path().join("authority");
        for directory in [&workspace, &machines, &repository, &authority] {
            std::fs::create_dir_all(directory).expect("create synthetic input directory");
        }

        std::fs::write(
            workspace.join("capabilities.toml"),
            r#"[[run_policies]]
id = "standard"
description = "Portable compilation without an action gate."
session_inject = false
child_propagation = "propagate"
action_gate = false
gated_tool_matchers = []
"#,
        )
        .expect("write private capabilities");
        let machine = MachineBindingFile {
            machine: MachineMeta {
                id: "machine".to_string(),
                schema_version: 1,
                distribution_root: Some(distribution.to_string_lossy().into_owned()),
            },
            repository_checkouts: vec![RepositoryCheckoutBindingV1 {
                repository_id: LogicalRepositoryIdV1::parse("engine").unwrap(),
                checkout_root: repository.to_string_lossy().into_owned(),
                git_metadata_root: None,
            }],
            provider_repositories: Vec::new(),
            authority_roots: BTreeMap::from([(
                "authority".to_string(),
                authority.to_string_lossy().into_owned(),
            )]),
            preferences: None,
            surface_executable: None,
            native_harnesses: Vec::new(),
            session_state: None,
            durable_evidence: None,
            materialization_lineage: None,
        };
        let machine_binding = machines.join("machine.toml");
        std::fs::write(
            &machine_binding,
            toml::to_string(&machine).expect("serialize machine binding"),
        )
        .expect("write machine binding");

        let manifest = workspace.join("pale-workspace.toml");
        let composition = super::composition_source_fixtures::workspace_composition_source(&[
            "direct-claude-code",
        ]);
        std::fs::write(
            workspace.join("composition.toml"),
            toml::to_string(&composition).expect("serialize synthetic composition source"),
        )
        .expect("write synthetic composition source");
        std::fs::write(
            &manifest,
            r#"[workspace]
schema_version = 1
workspace_id = "compile-tests"
distribution_lock = "distribution-lock.toml"
machine_binding_root = "machines"
desired_topology = { projects = [{ id = "demo", title = "Synthetic compile", run_policy = "standard", package_seeds = ["example.engine"], authority_root_ids = ["authority"], capabilities_file = "capabilities.toml" }], execution_workspaces = [{ id = "default", title = "Default execution workspace", execution_path_ids = ["direct-claude"] }], repositories = [{ id = "engine", title = "Engine" }], execution_paths = [{ id = "direct-claude", experience_path_id = "direct-claude-code" }], project_repositories = [{ project_id = "demo", repository_id = "engine", role = { package_id = "example.engine", resource_id = "engine" }, stacks = [], families = [], materialization_order = 0 }], project_execution_workspaces = [{ project_id = "demo", execution_workspace_id = "default" }], execution_workspace_repositories = [{ execution_workspace_id = "default", repository_id = "engine", access = "mutate" }] }
module_manifests = []
source_components = []
responsibility_derivations = []
organizational_translations = []
experience_compositions = ["composition.toml"]
resolution_lock = "workspace-resolution.lock.toml"
universe_lock = "workspace-universe.lock.toml"
"#,
        )
        .expect("write workspace entry");
        super::workspace_fixtures::write_empty_universe(&workspace, "compile-tests");

        let fixture = Self {
            root,
            manifest,
            machine_binding,
        };
        fixture.refresh_distribution_lock();
        fixture
    }

    pub(crate) fn capture_inputs(&self) -> Result<WorkspaceInputSnapshot, CompileError> {
        let entry = resolve_entry(
            &pale_adapter_fs::LocalSourceFilesystem,
            &self.manifest,
            "demo",
            "default",
            "machine",
        )?;
        Ok(entry.capture_inputs(&pale_adapter_fs::LocalSourceFilesystem)?)
    }

    /// Refresh the fixture lock after an intentional Package release change.
    pub(crate) fn refresh_distribution_lock(&self) {
        write_distribution_lock(
            &self.root.path().join("distribution"),
            &self.root.path().join("workspace"),
        );
    }
}

fn write_distribution_lock(distribution: &Path, workspace: &Path) {
    let info = inspect_distribution(&pale_adapter_fs::LocalSourceFilesystem, distribution)
        .expect("inspect synthetic Package releases");
    let lock = DistributionLockFile {
        distribution: DistributionMeta {
            compiler: info.compiler().to_string(),
            schema_manifest_sha256: info.schema_manifest_sha256().clone(),
            schema_set_sha256: info.schema_set_sha256().clone(),
        },
        packages: info.packages().to_vec(),
    };
    std::fs::write(
        workspace.join("distribution-lock.toml"),
        toml::to_string(&lock).expect("serialize exact distribution lock"),
    )
    .expect("write exact distribution lock");
}
