//! Zola roles and operations project only into Zola repositories, with one composed doctrine copy
//! in each native-harness role surface.

use std::fs;

use pale_application::compile::{compile_portable_inputs, Compilation};
use pale_application::packages::inspect_distribution;
use pale_application::workspace::resolve_entry;
use pale_core::records::{DistributionLockFile, DistributionMeta};

#[path = "support/composition_source.rs"]
mod composition_source_fixtures;
#[path = "support/provider_binding.rs"]
mod provider_binding_fixtures;
#[path = "support/distribution.rs"]
mod test_distribution;
#[path = "support/workspace.rs"]
mod workspace_fixtures;

/// Exact stack/role selection proving the Zola projection boundary.
const ZOLA_SELECTION: &str = r#"[semantic]
operations = [
  { selection = "exact", resource = { package_id = "example.zola", resource_id = "zola-review" } },
  { selection = "exact", resource = { package_id = "example.zola", resource_id = "zola-build" } },
]
roles = [
  { selection = "exact", resource = { package_id = "example.zola", resource_id = "zola-reviewer" } },
  { selection = "exact", resource = { package_id = "example.zola", resource_id = "zola-build-resolver" } },
]
stacks = [
  { selection = "exact", resource = { package_id = "example.zola", resource_id = "zola" } },
  { selection = "exact", resource = { package_id = "example.zola", resource_id = "rust" } },
]

[[operation_bindings]]
operation = { package_id = "example.zola", resource_id = "zola-review" }
[operation_bindings.binding.selection]
kind = "model_or_user"

[[operation_bindings]]
operation = { package_id = "example.zola", resource_id = "zola-build" }
[operation_bindings.binding.selection]
kind = "model_or_user"
"#;

/// Private run policy used by both host projections in this fixture.
const RUN_POLICY: &str = r#"
[[run_policies]]
id = "standard"
description = "Projection fixture policy without resident lifecycle mechanics."
session_inject = false
child_propagation = "none"
action_gate = false
gated_tool_matchers = []
"#;

/// Machine fixture whose harness bindings make both projections comparable.
const MACHINE: &str = r#"[machine]
id = "synthetic"
schema_version = 1
[[repository_checkouts]]
repository_id = "alpha"
checkout_root = "/tmp/synthetic/alpha"
[authority_roots]
auth_a = "/tmp/synthetic/auth"
"#;

fn compile_repository(stack: &str) -> Compilation {
    let (_distribution_guard, distribution) = test_distribution::synthetic_contract_distribution();
    let workspace = tempfile::tempdir().unwrap();
    let machines = workspace.path().join("machines");
    fs::create_dir_all(&machines).unwrap();
    let semantic_selection = if stack == "zola" {
        ZOLA_SELECTION.to_string()
    } else {
        format!(
            "[semantic]\nstacks = [{{ selection = \"exact\", resource = {{ package_id = \"example.zola\", resource_id = \"{stack}\" }} }}]\n"
        )
    };
    fs::write(
        workspace.path().join("capabilities.toml"),
        format!("{semantic_selection}{RUN_POLICY}"),
    )
    .unwrap();
    workspace_fixtures::write_empty_universe(workspace.path(), "zola-projection-test");
    let mut machine = toml::from_str::<toml::Value>(MACHINE).unwrap();
    machine
        .get_mut("machine")
        .and_then(toml::Value::as_table_mut)
        .unwrap()
        .insert(
            "distribution_root".into(),
            toml::Value::String(distribution.to_string_lossy().into_owned()),
        );
    fs::write(
        machines.join("synthetic.toml"),
        toml::to_string(&machine).unwrap(),
    )
    .unwrap();
    let inspected = inspect_distribution(&pale_adapter_fs::LocalSourceFilesystem, &distribution)
        .expect("inspect the exact synthetic Zola Package");
    let lock = DistributionLockFile {
        distribution: DistributionMeta {
            compiler: inspected.compiler().to_string(),
            schema_manifest_sha256: inspected.schema_manifest_sha256().clone(),
            schema_set_sha256: inspected.schema_set_sha256().clone(),
        },
        packages: inspected.packages().to_vec(),
    };
    fs::write(
        workspace.path().join("distribution-lock.toml"),
        toml::to_string(&lock).expect("serialize the exact synthetic distribution lock"),
    )
    .unwrap();
    let composition = composition_source_fixtures::workspace_composition_source(&[
        "direct-claude-code",
        "direct-codex",
    ]);
    fs::write(
        workspace.path().join("composition.toml"),
        toml::to_string(&composition).expect("serialize the shared composition source"),
    )
    .unwrap();
    let manifest = workspace.path().join("pale-workspace.toml");
    fs::write(
        &manifest,
        format!(r#"[workspace]
schema_version = 1
workspace_id = "zola-projection-test"
distribution_lock = "distribution-lock.toml"
machine_binding_root = "machines"
desired_topology = {{ projects = [{{ id = "zola-projection", title = "Zola projection", run_policy = "standard", package_seeds = ["example.zola"], authority_root_ids = ["auth_a"], capabilities_file = "capabilities.toml" }}], execution_workspaces = [{{ id = "default", title = "Default execution workspace", execution_path_ids = ["claude", "codex"] }}], repositories = [{{ id = "alpha", title = "Alpha" }}], execution_paths = [{{ id = "claude", experience_path_id = "direct-claude-code" }}, {{ id = "codex", experience_path_id = "direct-codex" }}], project_repositories = [{{ project_id = "zola-projection", repository_id = "alpha", role = {{ package_id = "example.zola", resource_id = "site" }}, stacks = [{{ package_id = "example.zola", resource_id = "{stack}" }}], families = [], materialization_order = 0 }}], project_execution_workspaces = [{{ project_id = "zola-projection", execution_workspace_id = "default" }}], execution_workspace_repositories = [{{ execution_workspace_id = "default", repository_id = "alpha", access = "read" }}] }}
module_manifests = []
source_components = []
responsibility_derivations = []
organizational_translations = []
experience_compositions = ["composition.toml"]
resolution_lock = "workspace-resolution.lock.toml"
universe_lock = "workspace-universe.lock.toml"
"#),
    )
    .unwrap();
    let entry = resolve_entry(
        &pale_adapter_fs::LocalSourceFilesystem,
        &manifest,
        "zola-projection",
        "default",
        "synthetic",
    )
    .unwrap();
    compile_portable_inputs(
        entry
            .capture_inputs(&pale_adapter_fs::LocalSourceFilesystem)
            .unwrap(),
        &pale_adapter_fs::LocalSourceFilesystem,
    )
    .unwrap()
}

#[test]
fn zola_projection_is_host_equivalent_and_stack_confined() {
    let zola = compile_repository("zola");
    let surface = |path: &str| {
        zola.repositories()[0]
            .render()
            .surfaces
            .iter()
            .find(|surface| surface.rel_path == path)
            .unwrap_or_else(|| panic!("missing Zola surface `{path}`"))
            .content()
    };

    for role in ["zola-reviewer", "zola-build-resolver"] {
        let host_role = format!("example-zola-{role}");
        let claude = surface(&format!(".claude/agents/pale-role-{host_role}.md"));
        let claude_instructions = claude
            .split_once("---\n\n")
            .and_then(|(_, body)| body.split_once("\n\n"))
            .and_then(|(_, instructions)| {
                instructions
                    .split_once("\n\nWork from the supplied evidence")
                    .map(|(shared, _)| shared)
            })
            .unwrap_or_else(|| panic!("malformed Claude role surface for `{role}`"));
        let codex: toml::Value = toml::from_str(surface(&format!(
            ".codex/agents/pale-role-{host_role}.toml"
        )))
        .unwrap();
        let instructions = codex["developer_instructions"].as_str().unwrap();
        let codex_instructions = instructions
            .split_once("\n\nReturn findings with supporting evidence")
            .map(|(shared, _)| shared)
            .unwrap_or_else(|| panic!("malformed Codex role surface for `{role}`"));
        assert_eq!(
            claude_instructions, codex_instructions,
            "both native harnesses must receive the same composed role doctrine"
        );
    }
    let judgment = "pale-doctrine-example-zola-zola-judgment";
    let index = surface(".pale/semantic-index.md");
    assert!(index.contains(&format!(
        "- `example.zola:zola` - Synthetic Zola stack.\n  - Guidance skill `{judgment}`"
    )));
    for host in ["claude", "codex"] {
        surface(&format!(".{host}/skills/{judgment}/SKILL.md"));
        for operation in ["zola-review", "zola-build"] {
            let skill = surface(&format!(
                ".{host}/skills/pale-op-example-zola-{operation}/SKILL.md"
            ));
            assert!(skill.contains("Stacks section of `.pale/semantic-index.md`"));
        }
    }

    let non_zola = compile_repository("rust");
    for surface in &non_zola.repositories()[0].render().surfaces {
        assert!(
            !surface.rel_path.contains("zola-review")
                && !surface.rel_path.contains("zola-build")
                && !surface.rel_path.contains(judgment),
            "Zola-only surface leaked into a Rust repository: {}",
            surface.rel_path
        );
    }
}
