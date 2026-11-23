//! CLI entry, workspace selection, validation, and distribution inspection behavior.

use super::*;

#[test]
fn canonical_entry_validate_compiles() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = write_workspace(tmp.path());
    let out = run(&manifest, "demo", "default", "synthetic");
    assert!(
        out.status.success(),
        "validate should compile the synthetic workspace; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn distribution_info_derives_reviewable_lock_inputs_without_trusting_the_old_lock() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = write_workspace(tmp.path());
    let lock_path = tmp.path().join("distribution/lock.toml");
    let mut stale_lock: toml::Value =
        toml::from_str(&std::fs::read_to_string(&lock_path).unwrap()).unwrap();
    stale_lock["package"][0]["content_digest"] = toml::Value::String("f".repeat(64));
    std::fs::write(
        tmp.path().join("distribution/lock.toml"),
        toml::to_string(&stale_lock).unwrap(),
    )
    .unwrap();
    let stale_lock_bytes = std::fs::read(&lock_path).unwrap();

    let validate = run(&manifest, "demo", "default", "synthetic");
    assert!(
        !validate.status.success(),
        "the stale lock must still fail closed"
    );

    let output = Command::new(bin())
        .args([
            "--workspace",
            manifest.to_str().unwrap(),
            "--project",
            "demo",
            "--execution-workspace",
            "default",
            "--machine",
            "synthetic",
            "distribution-info",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "digest derivation must not consume the stale digest it is replacing: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let info: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let expected = pale_sdk::PaleApi::new()
        .unwrap()
        .inspect_local_package_distribution(pale_sdk::LocalPackageDistributionRequest::new(
            pale_sdk::LocalWorkspaceLocator::new(manifest.to_str().unwrap().to_owned()),
            "demo".to_owned(),
            "default".to_owned(),
            "synthetic".to_owned(),
        ))
        .unwrap();
    assert_eq!(
        info,
        serde_json::from_slice::<serde_json::Value>(expected.canonical_bytes()).unwrap()
    );
    assert!(output.stderr.is_empty());
    assert_eq!(std::fs::read(&lock_path).unwrap(), stale_lock_bytes);
    assert_eq!(
        info["schema_manifest_sha256"].as_str().map(str::len),
        Some(64)
    );
    assert_eq!(info["schema_set_sha256"].as_str().map(str::len), Some(64));
    assert_eq!(info["packages"].as_array().map(Vec::len), Some(2));
    assert!(info["packages"]
        .as_array()
        .unwrap()
        .iter()
        .all(|package| package["content_digest"].as_str().map(str::len) == Some(64)));
}

#[test]
fn missing_workspace_flag_fails_closed() {
    let out = Command::new(bin()).arg("validate").output().unwrap();
    assert!(
        !out.status.success(),
        "validate with no --workspace must fail closed"
    );
}

#[test]
fn unknown_project_fails_closed() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = write_workspace(tmp.path());
    let out = run(&manifest, "does-not-exist", "default", "synthetic");
    assert!(
        !out.status.success(),
        "an unknown project identity must fail closed"
    );
}

#[test]
fn unknown_execution_workspace_fails_closed() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = write_workspace(tmp.path());
    let out = run(&manifest, "demo", "does-not-exist", "synthetic");
    assert!(
        !out.status.success(),
        "an unknown execution-workspace identity must fail closed"
    );
}

#[test]
fn unknown_machine_fails_closed() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = write_workspace(tmp.path());
    let out = run(&manifest, "demo", "default", "no-such-machine");
    assert!(
        !out.status.success(),
        "an unknown machine id must fail closed"
    );
}

#[test]
fn invalid_project_identity_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let manifest = write_workspace(tmp.path());
    let out = run(&manifest, "../demo", "default", "synthetic");
    assert!(
        !out.status.success(),
        "an invalid project identity must be rejected"
    );
}

#[test]
fn removed_four_path_flags_are_rejected() {
    // The old bypass-prone flags no longer exist; the arg parser must reject them,
    // proving the only entry is the canonical workspace/project/execution-workspace/machine tuple.
    let out = Command::new(bin())
        .args([
            "--distribution-root",
            "/x",
            "--project-set-dir",
            "/y",
            "--lock",
            "/z",
            "validate",
        ])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "the removed four-path flags must be rejected by the parser"
    );
}

#[test]
fn compiled_identity_reports_sdk_claim_without_a_workspace() {
    let directory = tempfile::tempdir().unwrap();
    let output = Command::new(bin())
        .current_dir(directory.path())
        .arg("compiled-identity")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let reported: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let identity = pale_sdk::PaleApi::new()
        .unwrap()
        .compiled_interface_identity()
        .unwrap();
    let expected: serde_json::Value = serde_json::from_slice(identity.canonical_bytes()).unwrap();
    assert_eq!(reported, expected);
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn version_flag_reports_the_canonical_binary_identity() {
    let out = Command::new(bin()).arg("--version").output().unwrap();
    assert!(out.status.success(), "--version must succeed");
    assert!(
        out.stderr.is_empty(),
        "--version must not emit diagnostics: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("pale {}\n", env!("CARGO_PKG_VERSION")),
        "--version must bind the canonical executable name to the package version"
    );
}

#[test]
fn surface_status_is_distinct_from_the_reserved_supervisor_status_name() {
    let surface_help = Command::new(bin())
        .args(["surface-status", "--help"])
        .output()
        .unwrap();
    assert!(surface_help.status.success());
    assert!(surface_help.stderr.is_empty());
    assert!(String::from_utf8_lossy(&surface_help.stdout).contains("Usage: pale surface-status"));

    let reserved = Command::new(bin()).arg("status").output().unwrap();
    assert_eq!(reserved.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&reserved.stderr).contains("unrecognized subcommand 'status'"));
}

#[test]
fn removed_tool_server_subcommand_has_no_compatibility_alias() {
    let removed = Command::new(bin()).arg("tool-server").output().unwrap();
    assert_eq!(removed.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&removed.stderr).contains("unrecognized subcommand 'tool-server'"),
        "the ordinary CLI must not retain an alias for the standalone pale-mcp process"
    );
}

#[test]
fn standalone_help_and_invocation_omit_attachment_only_resident_commands() {
    let help = Command::new(bin()).arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(help.stderr.is_empty());
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(
        !help
            .lines()
            .any(|line| line.trim_start().starts_with("resident ")),
        "the stateless executable must not advertise an unavailable resident route"
    );

    let unavailable_path = "/path-that-must-not-be-read/resident-request.json";
    let invocation = Command::new(bin())
        .args([
            "resident",
            "observe",
            "--operation-handle-record-ref",
            "operations/example",
            "--operation-handle-sha256",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "--signed-request",
            unavailable_path,
        ])
        .output()
        .unwrap();
    assert_eq!(invocation.status.code(), Some(2));
    assert!(invocation.stdout.is_empty());
    let error = String::from_utf8_lossy(&invocation.stderr);
    assert!(error.contains("unrecognized subcommand 'resident'"));
    assert!(
        !error.contains(unavailable_path),
        "parser rejection must occur before the request path can reach CLI input handling"
    );
}
