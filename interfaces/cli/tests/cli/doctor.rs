//! Process-level diagnosis, including absent selection and independent scans.

use super::{bin, write_workspace, Command};

#[test]
fn doctor_reports_missing_checks_and_all_selected_builds_without_writes() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    std::fs::write(root.join("artifact"), b"12345").unwrap();
    let output = Command::new(bin())
        .args(["--result-format", "json", "doctor", "--build-root"])
        .arg(&root)
        .args(["--build-root"])
        .arg(root.join("missing"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["completion"], "complete");
    assert_eq!(report["result"]["checks_passed"], false);
    assert_eq!(report["result"]["builds"][0]["logical_bytes"], 5);
    assert_eq!(
        report["result"]["builds"][1]["logical_bytes"],
        serde_json::Value::Null
    );
    assert_eq!(std::fs::read(root.join("artifact")).unwrap(), b"12345");
    assert_eq!(std::fs::read_dir(root).unwrap().count(), 1);
}

#[test]
fn doctor_human_output_names_missing_declarations_and_next_actions() {
    let output = Command::new(bin()).arg("doctor").output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("unavailable toolchains"));
    assert!(text.contains("unavailable resident_reachability"));
    assert!(text.contains("Next:"));
}

#[test]
fn doctor_checks_workspace_through_the_existing_cli_selection() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let manifest = write_workspace(&root);
    let output = Command::new(bin())
        .arg("--workspace")
        .arg(&manifest)
        .args([
            "--project",
            "demo",
            "--execution-workspace",
            "default",
            "--machine",
            "synthetic",
            "--result-format",
            "json",
            "doctor",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let checks = report["result"]["checks"].as_array().unwrap();
    assert!(
        checks
            .iter()
            .any(|check| check["code"] == "workspace_compilation" && check["state"] == "passed"),
        "{report}"
    );
    assert!(checks
        .iter()
        .any(|check| check["code"] == "hook_delivery" && check["state"] == "unavailable"));
}
