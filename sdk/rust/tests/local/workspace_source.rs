//! Local authors can inspect source before selecting publisher keys or a machine.

use super::*;

#[test]
fn source_inspection_needs_no_machine_or_distribution_and_never_admits_local_modules() {
    let (temporary, manifest) = source_workspace();
    let root = temporary.path().canonicalize().unwrap();
    let mut document: toml::Value =
        toml::from_str(&fs::read_to_string(&manifest).unwrap()).unwrap();
    document["workspace"]["experience_compositions"] = toml::Value::Array(Vec::new());
    fs::write(&manifest, toml::to_string(&document).unwrap()).unwrap();
    let api = PaleApi::new().unwrap();
    api.compile_local_effective_profile(request(manifest.clone()))
        .unwrap();
    document["workspace"].as_table_mut().unwrap().insert(
        "module_sources".into(),
        toml::Value::Array(vec!["local.toml".into()]),
    );
    fs::write(&manifest, toml::to_string(&document).unwrap()).unwrap();
    fs::write(
        root.join("local.toml"),
        r#"
schema = "pale/workspace-module-source.v1"
schema_version = 1
module_id = "unpublished"
source_components = []
imports = []
exports = []
extensions = []
resources = []
"#,
    )
    .unwrap();
    let error = api
        .compile_local_effective_profile(request(manifest.clone()))
        .unwrap_err();
    assert_eq!(error.diagnostic().code(), "workspace_semantics_rejected");
    fs::remove_file(root.join("machines/machine.toml")).unwrap();
    fs::remove_file(root.join("distribution-lock.toml")).unwrap();
    let inspection = api
        .inspect_local_workspace_source(LocalWorkspaceLocator::new(manifest.clone()))
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(inspection.canonical_bytes()).unwrap();
    assert_eq!(report["phase"], "source_only");
    assert_eq!(report["modules"][0]["module_id"], "unpublished");
    assert_eq!(report["content_digest"], inspection.content_digest());
}

#[test]
fn source_inspection_rejects_parent_traversal_before_reading() {
    let api = PaleApi::new().unwrap();
    assert!(api
        .inspect_local_workspace_source(LocalWorkspaceLocator::new(
            "/tmp/../pale-workspace.toml".into()
        ))
        .is_err());
}
