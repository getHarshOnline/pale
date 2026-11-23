//! Check local authoring without a project, execution-workspace or machine selection.

use super::{bin, write_workspace, Command};

#[test]
fn check_source_reports_source_only_content_and_rejects_release_fields() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let manifest = write_workspace(&root);
    let mut document: toml::Value =
        toml::from_str(&std::fs::read_to_string(&manifest).unwrap()).unwrap();
    document["workspace"]["experience_compositions"] = toml::Value::Array(Vec::new());
    document["workspace"]["desired_topology"]["execution_paths"] = toml::Value::Array(Vec::new());
    document["workspace"]["desired_topology"]["execution_workspaces"][0]["execution_path_ids"] =
        toml::Value::Array(Vec::new());
    document["workspace"].as_table_mut().unwrap().insert(
        "module_sources".into(),
        toml::Value::Array(vec!["local.toml".into()]),
    );
    std::fs::write(&manifest, toml::to_string(&document).unwrap()).unwrap();
    let module = "schema = \"pale/workspace-module-source.v1\"\nschema_version = 1\nmodule_id = \"unpublished\"\nsource_components = []\nimports = []\nexports = []\nextensions = []\nresources = []\n";
    std::fs::write(root.join("local.toml"), module).unwrap();
    let output = Command::new(bin())
        .arg("--workspace")
        .arg(&manifest)
        .args(["--result-format", "json", "check-source"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["result"]["phase"], "source_only");
    assert_eq!(report["result"]["modules"][0]["module_id"], "unpublished");
    std::fs::write(
        root.join("local.toml"),
        format!("{module}release_id = \"invented\"\n"),
    )
    .unwrap();
    let output = Command::new(bin())
        .arg("--workspace")
        .arg(&manifest)
        .arg("check-source")
        .output()
        .unwrap();
    assert!(!output.status.success());
}
