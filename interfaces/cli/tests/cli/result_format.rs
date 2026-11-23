//! Machine-readable terminal results across exact CLI-owned output boundaries.

use super::*;

fn envelope(output: &std::process::Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is exactly one JSON value");
    assert_eq!(value.as_object().map(serde_json::Map::len), Some(2));
    assert!(value.get("completion").is_some());
    assert!(value.get("result").is_some());
    value
}

fn machine_review(
    baseline: &Path,
    candidate: &Path,
    action: &[&str],
    input: &[u8],
) -> std::process::Output {
    let mut child = Command::new(bin())
        .args([
            "--result-format",
            "json",
            "--error-format",
            "json",
            "--workspace",
            baseline.to_str().unwrap(),
            "--project",
            "demo",
            "--execution-workspace",
            "default",
            "--machine",
            "synthetic",
            "review-configuration",
            "--candidate-workspace",
            candidate.to_str().unwrap(),
        ])
        .args(action)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn complete_envelope_retains_the_exact_owner_value_and_default_bytes() {
    let help = Command::new(bin()).arg("--help").output().unwrap();
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).unwrap();
    assert!(help.contains("--result-format <RESULT_FORMAT>"));
    assert!(help.contains("successful, partial, or non-terminal"));

    let default = Command::new(bin())
        .arg("compiled-identity")
        .output()
        .unwrap();
    assert!(default.status.success());
    let exact: serde_json::Value = serde_json::from_slice(
        pale_sdk::PaleApi::new()
            .unwrap()
            .compiled_interface_identity()
            .unwrap()
            .canonical_bytes(),
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(default.stdout).unwrap(),
        format!("{}\n", serde_json::to_string_pretty(&exact).unwrap()),
        "the default human presentation must not change"
    );

    let output = Command::new(bin())
        .args(["--result-format", "json", "compiled-identity"])
        .output()
        .unwrap();
    let result = envelope(&output);
    assert_eq!(result["completion"], "complete");
    assert_eq!(result["result"], exact);
}

#[test]
fn canonical_action_and_compilation_payloads_are_embedded_without_reconstruction() {
    let input = br#"{"kind":"path_inventory","payload":{"style":"posix_relative","paths":["src/./lib.rs"]}}"#;
    let expected_action = pale_sdk::PaleApi::new()
        .unwrap()
        .invoke_native_action(pale_sdk::NativeActionRequest::new(
            "pale.compute.path-inventory".to_owned(),
            input.to_vec(),
        ))
        .unwrap();
    let mut action = Command::new(bin())
        .args([
            "--result-format",
            "json",
            "action",
            "pale.compute.path-inventory",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    action.stdin.take().unwrap().write_all(input).unwrap();
    let action = envelope(&action.wait_with_output().unwrap());
    assert_eq!(action["completion"], "complete");
    assert_eq!(
        action["result"],
        serde_json::from_slice::<serde_json::Value>(expected_action.canonical_bytes()).unwrap()
    );

    let root = tempfile::tempdir().unwrap();
    let manifest = write_workspace(root.path());
    let summary = Command::new(bin())
        .args([
            "--result-format",
            "json",
            "--workspace",
            manifest.to_str().unwrap(),
            "--project",
            "demo",
            "--execution-workspace",
            "default",
            "--machine",
            "synthetic",
            "summary",
        ])
        .output()
        .unwrap();
    let summary = envelope(&summary);
    assert_eq!(summary["completion"], "complete");

    let default = Command::new(bin())
        .args([
            "--workspace",
            manifest.to_str().unwrap(),
            "--project",
            "demo",
            "--execution-workspace",
            "default",
            "--machine",
            "synthetic",
            "summary",
        ])
        .output()
        .unwrap();
    assert_eq!(
        summary["result"],
        serde_json::from_slice::<serde_json::Value>(&default.stdout).unwrap()
    );
}

#[test]
fn review_outcomes_map_explicit_stages_to_non_terminal_partial_and_complete() {
    let root = tempfile::tempdir().unwrap();
    let baseline = write_workspace(&root.path().join("baseline"));
    let candidate = write_workspace(&root.path().join("candidate"));

    let preview = envelope(&machine_review(&baseline, &candidate, &["preview"], &[]));
    assert_eq!(preview["completion"], "non_terminal");
    let summary = serde_json::to_vec(&preview["result"]).unwrap();

    let partial = envelope(&machine_review(
        &baseline,
        &candidate,
        &["impacts", "--limit", "1"],
        &summary,
    ));
    assert_eq!(partial["completion"], "partial");
    assert!(partial["result"]["nextOffset"].is_number());

    let complete = envelope(&machine_review(
        &baseline,
        &candidate,
        &["impacts", "--limit", "65535"],
        &summary,
    ));
    assert_eq!(complete["completion"], "complete");
    assert!(complete["result"]["nextOffset"].is_null());

    let finished = envelope(&machine_review(
        &baseline,
        &candidate,
        &["accept"],
        &summary,
    ));
    assert_eq!(finished["completion"], "complete");
    assert_eq!(finished["result"]["disposition"], "accepted");
    assert_eq!(finished["result"]["summary"], preview["result"]);
}

#[test]
fn machine_result_format_does_not_reclassify_diagnostics_or_contaminate_stdout() {
    let mut child = Command::new(bin())
        .args([
            "--result-format",
            "json",
            "--error-format",
            "json",
            "action",
            "pale.compute.does-not-exist",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"{}").unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(diagnostic["diagnostic"]["disposition"], "invalid_input");
}

#[cfg(unix)]
fn write_plan_workspace(root: &Path) -> (test_executable::TestExecutable, PathBuf) {
    let executable = test_executable::snapshot(root).unwrap();
    let workspace = root.join("workspace");
    let alpha = root.join("alpha");
    let beta = root.join("beta");
    let authority = root.join("authority");
    let state = root.join("state");
    let lineage = root.join("lineage");
    for directory in [&alpha, &beta, &authority, &state, &lineage] {
        std::fs::create_dir(directory).unwrap();
    }
    std::fs::write(authority.join("README.md"), "# fixture authority\n").unwrap();
    let manifest = activation_fixture::lay_out(
        &workspace,
        &activation_fixture::MachineInputs {
            target: &alpha,
            auth: &authority,
            exec: executable.path(),
            digest: executable.sha256(),
            state_root: &state,
            lineage_root: &lineage,
        },
    );

    let mut source: toml::Value =
        toml::from_str(&std::fs::read_to_string(&manifest).unwrap()).unwrap();
    let topology = &mut source["workspace"]["desired_topology"];
    let repositories = topology["repositories"].as_array_mut().unwrap();
    let mut second = repositories[0].clone();
    second["id"] = "beta".into();
    second["title"] = "Beta".into();
    repositories.push(second);
    for key in ["project_repositories", "execution_workspace_repositories"] {
        let repositories = topology[key].as_array_mut().unwrap();
        let mut second = repositories[0].clone();
        second["repository_id"] = "beta".into();
        if key == "project_repositories" {
            second["materialization_order"] = toml::Value::Integer(1);
        }
        repositories.push(second);
    }
    std::fs::write(&manifest, toml::to_string(&source).unwrap()).unwrap();

    let binding = workspace.join("bindings/machines/m.toml");
    let mut machine: toml::Value =
        toml::from_str(&std::fs::read_to_string(&binding).unwrap()).unwrap();
    let checkouts = machine["repository_checkouts"].as_array_mut().unwrap();
    let mut second = checkouts[0].clone();
    second["repository_id"] = "beta".into();
    second["checkout_root"] = beta.to_str().unwrap().into();
    checkouts.push(second);
    std::fs::write(binding, toml::to_string(&machine).unwrap()).unwrap();
    (executable, manifest)
}

#[cfg(unix)]
fn machine_plan(executable: &Path, manifest: &Path, output: &Path) -> std::process::Output {
    Command::new(executable)
        .args([
            "--result-format",
            "json",
            "--error-format",
            "json",
            "--workspace",
            manifest.to_str().unwrap(),
            "--project",
            "act",
            "--execution-workspace",
            "default",
            "--machine",
            "m",
            "plan",
            "--out",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap()
}

#[test]
#[cfg(unix)]
fn saved_plan_publication_emits_one_complete_document_in_repository_order() {
    let root = tempfile::tempdir().unwrap();
    let (executable, manifest) = write_plan_workspace(root.path());
    let plans = root.path().join("plans");
    let output = machine_plan(executable.path(), &manifest, &plans);
    let result = envelope(&output);

    assert_eq!(result["completion"], "complete");
    assert_eq!(result["result"]["plan_kind"], "apply");
    let publications = result["result"]["publications"].as_array().unwrap();
    assert_eq!(publications.len(), 2);
    assert_eq!(publications[0]["repository_id"], "alpha");
    assert_eq!(publications[1]["repository_id"], "beta");
    assert!(publications
        .iter()
        .all(|publication| publication["counts"]["total_operations"].is_number()));
    assert!(result["result"]["stopped_repository_id"].is_null());
    assert!(result["result"]["diagnostic"].is_null());
    assert!(plans.join("alpha.apply.json").is_file());
    assert!(plans.join("beta.apply.json").is_file());
}

#[test]
#[cfg(unix)]
fn saved_plan_publication_reports_failure_before_the_first_write() {
    let root = tempfile::tempdir().unwrap();
    let (executable, manifest) = write_plan_workspace(root.path());
    let blocked_output = root.path().join("not-a-directory");
    std::fs::write(&blocked_output, b"occupied\n").unwrap();
    let output = machine_plan(executable.path(), &manifest, &blocked_output);

    assert!(!output.status.success());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(result["completion"], "partial");
    assert_eq!(result["result"]["plan_kind"], "apply");
    assert_eq!(result["result"]["publications"], serde_json::json!([]));
    assert_eq!(result["result"]["stopped_repository_id"], "alpha");
    assert_eq!(result["result"]["diagnostic"], diagnostic["diagnostic"]);
    assert_eq!(diagnostic["diagnostic"]["effect"], "not_started");
    assert_eq!(std::fs::read(&blocked_output).unwrap(), b"occupied\n");
}

#[test]
#[cfg(unix)]
fn saved_plan_publication_reports_real_directory_creation_uncertainty() {
    let root = tempfile::tempdir().unwrap();
    let (executable, manifest) = write_plan_workspace(root.path());
    let new_parent = root.path().join("created-before-failure");
    // The missing parent sends preparation through recursive creation.
    // The final component exceeds the filesystem's per-name limit.
    let plans = new_parent.join("x".repeat(256));

    let output = machine_plan(executable.path(), &manifest, &plans);

    assert_eq!(
        output.status.code(),
        Some(7),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(result["completion"], "non_terminal");
    assert_eq!(result["result"]["plan_kind"], "apply");
    assert_eq!(result["result"]["publications"], serde_json::json!([]));
    assert_eq!(result["result"]["stopped_repository_id"], "alpha");
    assert_eq!(result["result"]["diagnostic"], diagnostic["diagnostic"]);
    assert_eq!(diagnostic["diagnostic"]["effect"], "outcome_unknown");
    assert_eq!(diagnostic["diagnostic"]["retry"], "reconcile_first");
    assert!(new_parent.is_dir());
    assert_eq!(std::fs::read_dir(&new_parent).unwrap().count(), 0);
}

#[test]
#[cfg(unix)]
fn saved_plan_publication_reports_the_completed_prefix_before_failure() {
    let root = tempfile::tempdir().unwrap();
    let (executable, manifest) = write_plan_workspace(root.path());
    let plans = root.path().join("plans");
    std::fs::create_dir_all(plans.join("beta.apply.json")).unwrap();
    let output = machine_plan(executable.path(), &manifest, &plans);

    assert!(!output.status.success());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["completion"], "partial");
    assert_eq!(result["result"]["plan_kind"], "apply");
    let publications = result["result"]["publications"].as_array().unwrap();
    assert_eq!(publications.len(), 1);
    assert_eq!(publications[0]["repository_id"], "alpha");
    assert_eq!(result["result"]["stopped_repository_id"], "beta");
    assert!(result["result"]["diagnostic"].is_object());
    assert!(plans.join("alpha.apply.json").is_file());
    assert!(plans.join("beta.apply.json").is_dir());
}

#[test]
fn unsupported_hook_json_is_rejected_before_delivery() {
    // A parsed hook invocation keeps the repository harness's fail-closed
    // status even when the rejected option is owned by the CLI.
    let unsupported = Command::new(bin())
        .args([
            "--result-format",
            "json",
            "--error-format",
            "json",
            "hook",
            "post-commit",
            "--workspace",
            "workspace",
            "--repository",
            "repository",
            "--provider-binding",
            "provider-binding",
            "--provider-binding-identity",
            "provider-binding-identity",
        ])
        .output()
        .unwrap();
    assert_eq!(
        unsupported.status.code(),
        Some(2),
        "parsed repository-hook failures retain the native hook denial status"
    );
    assert!(unsupported.stdout.is_empty());
    let diagnostic: serde_json::Value = serde_json::from_slice(&unsupported.stderr).unwrap();
    assert_eq!(diagnostic["diagnostic"]["code"], "host_input_rejected");
}
