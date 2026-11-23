//! Current-contract workspace fixture shared by source-backed CLI journeys.

use std::path::{Path, PathBuf};

/// Lay out a source-complete workspace for project `demo`, execution workspace
/// `default`, and machine `synthetic`.
///
/// The fixture uses the current desired-topology and Package contracts. Its
/// machine paths are inert test values; source-backed inspection never treats
/// them as native readiness or effect evidence. The caller supplies one shared
/// synthetic distribution and its exact lock so a test binary never compiles a
/// second fixture owner.
pub(crate) fn lay_out(root: &Path, distribution_root: &Path, distribution_lock: &str) -> PathBuf {
    std::fs::create_dir_all(root.join("bindings/machines")).expect("create machine directory");
    std::fs::create_dir_all(root.join("distribution")).expect("create lock directory");

    std::fs::write(
        root.join("pale-workspace.toml"),
        r#"[workspace]
schema_version = 1
workspace_id = "syn-ws"
distribution_lock = "distribution/lock.toml"
machine_binding_root = "bindings/machines"
desired_topology = { projects = [{ id = "demo", title = "Demo", run_policy = "standard", package_seeds = ["pale.engineering", "pale.governance"], authority_root_ids = ["auth_a"], capabilities_file = "capabilities.toml" }], execution_workspaces = [{ id = "default", title = "Default execution workspace", execution_path_ids = ["direct-claude"] }], repositories = [{ id = "alpha", title = "Alpha" }, { id = "beta", title = "Beta" }], execution_paths = [{ id = "direct-claude", experience_path_id = "direct-claude-code" }], project_repositories = [{ project_id = "demo", repository_id = "alpha", role = { package_id = "pale.engineering", resource_id = "engine" }, stacks = [], families = [{ package_id = "pale.engineering", resource_id = "impact_analysis" }, { package_id = "pale.engineering", resource_id = "verification" }, { package_id = "pale.governance", resource_id = "authority-resolver" }], materialization_order = 0 }, { project_id = "demo", repository_id = "beta", role = { package_id = "pale.engineering", resource_id = "theme" }, stacks = [], families = [{ package_id = "pale.engineering", resource_id = "review" }, { package_id = "pale.governance", resource_id = "execution-contract" }], materialization_order = 1 }], project_execution_workspaces = [{ project_id = "demo", execution_workspace_id = "default" }], execution_workspace_repositories = [{ execution_workspace_id = "default", repository_id = "alpha", access = "mutate" }, { execution_workspace_id = "default", repository_id = "beta", access = "mutate" }] }
module_manifests = []
source_components = []
responsibility_derivations = []
organizational_translations = []
experience_compositions = ["composition.toml"]
resolution_lock = "workspace-resolution.lock.toml"
universe_lock = "workspace-universe.lock.toml"
"#,
    )
    .expect("write workspace manifest");
    std::fs::write(
        root.join("composition.toml"),
        include_str!("../fixtures/cross-leaf.v1/one-shot/workspace/composition.toml"),
    )
    .expect("write shared neutral composition fixture");

    std::fs::write(
        root.join("capabilities.toml"),
        r#"[boundary]
forbidden_token_authority = ["business_publication"]

[[run_policies]]
id = "standard"
description = "Canonical-entry fixture policy without lifecycle effects."
session_inject = false
child_propagation = "none"
action_gate = false
gated_tool_matchers = []
"#,
    )
    .expect("write capability fixture");

    std::fs::write(
        root.join("bindings/machines/synthetic.toml"),
        format!(
            r#"[machine]
id = "synthetic"
schema_version = 1
distribution_root = "{}"

[[repository_checkouts]]
repository_id = "alpha"
checkout_root = "/tmp/syn/alpha"

[[repository_checkouts]]
repository_id = "beta"
checkout_root = "/tmp/syn/beta"

[authority_roots]
auth_a = "/tmp/syn/auth"
"#,
            distribution_root.display()
        ),
    )
    .expect("write machine fixture");

    std::fs::write(root.join("distribution/lock.toml"), distribution_lock)
        .expect("write distribution lock");
    write_empty_universe(root);
    root.join("pale-workspace.toml")
}

fn write_empty_universe(root: &Path) {
    std::fs::write(
        root.join("workspace-resolution.lock.toml"),
        r#"schema = "pale/workspace-resolution-lock.v1"
schema_version = 1
releases = []
resolution_digest = "945db2ec1686d74bc476a920c073a6d9bbb69ea34ab78cda178d6177ce2b855c"
"#,
    )
    .expect("write empty resolution lock");
    std::fs::write(
        root.join("workspace-universe.lock.toml"),
        r#"schema = "pale/workspace-universe-lock.v1"
schema_version = 1
workspace_id = "syn-ws"
resolution_digest = "945db2ec1686d74bc476a920c073a6d9bbb69ea34ab78cda178d6177ce2b855c"
adopted_releases = []
universe_digest = "e411aab24851056caaaccae7815681ab1542a4ca2eb3b858a2e1bde16710293f"
"#,
    )
    .expect("write empty universe lock");
}
