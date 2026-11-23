//! CLI summary is the readable terminal projection of the shared typed result.

use std::process::Command;

#[path = "support/current_workspace.rs"]
mod current_workspace;
#[path = "support/distribution.rs"]
mod synthetic_distribution;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_pale")
}

#[test]
fn source_backed_summary_uses_the_path_free_api_projection() {
    let temporary = tempfile::tempdir().expect("create summary workspace");
    let distribution =
        synthetic_distribution::lay_out(&temporary.path().join("engine-distribution"));
    let lock = synthetic_distribution::render_lock(&distribution);
    let manifest = current_workspace::lay_out(temporary.path(), &distribution, &lock);
    let output = Command::new(bin())
        .args([
            "--workspace",
            manifest.to_str().expect("fixture path is UTF-8"),
            "--project",
            "demo",
            "--execution-workspace",
            "default",
            "--machine",
            "synthetic",
            "summary",
        ])
        .output()
        .expect("run summary command");
    assert!(
        output.status.success(),
        "summary failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let summary: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("summary is valid JSON");
    let expected_presentation = format!("{}\n", serde_json::to_string_pretty(&summary).unwrap());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        expected_presentation
    );
    assert_eq!(summary["project_id"], "demo");
    assert_eq!(summary["execution_workspace_id"], "default");
    assert_eq!(summary["repository_count"], 2);
    assert_eq!(
        summary["executable_identity_digest"],
        serde_json::Value::Null
    );
    assert!(summary.get("machine_id").is_none());

    let repositories = summary["repositories"]
        .as_array()
        .expect("summary repositories are ordered rows");
    assert_eq!(repositories.len(), 2);
    assert_eq!(repositories[0]["repository_id"], "alpha");
    assert_eq!(repositories[1]["repository_id"], "beta");
    for repository in repositories {
        assert_eq!(
            repository["activation_selected_execution_path_ids"],
            serde_json::json!([])
        );
        assert_eq!(
            repository["activation_selected_hook_events"],
            serde_json::json!([])
        );
    }
}
