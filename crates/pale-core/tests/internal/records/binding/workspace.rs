//! Verifies workspace binding paths remain confined during structural decoding.

use super::WorkspaceMeta;

#[test]
fn workspace_paths_are_confined_during_deserialization() {
    let value = serde_json::json!({
        "schema_version": 1,
        "workspace_id": "example-workspace",
        "distribution_lock": "distribution.lock.toml",
        "desired_topology": {
            "projects": [],
            "execution_workspaces": [],
            "repositories": [],
            "execution_paths": [],
            "project_repositories": [],
            "project_execution_workspaces": [],
            "execution_workspace_repositories": []
        },
        "machine_binding_root": "machines",
        "module_manifests": ["../outside.toml"],
        "source_components": ["components/source.toml"],
        "responsibility_derivations": [],
        "organizational_translations": [],
        "experience_compositions": [],
        "resolution_lock": "resolution.lock.toml",
        "universe_lock": "universe.lock.toml"
    });

    let error = serde_json::from_value::<WorkspaceMeta>(value)
        .expect_err("a parent component must not cross the workspace root");
    assert!(
        error
            .to_string()
            .contains("path must be a non-empty confined relative path"),
        "the control record must otherwise be valid: {error}"
    );
}
