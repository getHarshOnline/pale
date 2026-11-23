//! Real-command physical classification and diagnostic parity.

use pale_cli::command::{CommandDispatcher, CommandOutcome, CommandRequest};
use pale_sdk::{LocalControlStorageInspectionRequest, PaleApi};
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

fn principal_request(root: &str) -> LocalControlStorageInspectionRequest {
    LocalControlStorageInspectionRequest::principal(
        root.to_owned(),
        "principal-inspection".to_owned(),
        "store-inspection".to_owned(),
        "incarnation-inspection".to_owned(),
    )
}

fn command(root: &str) -> Command {
    let mut command = Command::new(super::bin());
    command.args([
        "--error-format",
        "json",
        "control-store-inspect",
        "--runtime-state-root",
        root,
        "--logical-store-id",
        "store-inspection",
        "--control-incarnation-id",
        "incarnation-inspection",
    ]);
    command
}

fn private_root() -> tempfile::TempDir {
    let parent = std::fs::canonicalize(std::env::temp_dir()).unwrap();
    let directory = tempfile::tempdir_in(parent).unwrap();
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    directory
}

#[test]
fn physical_inspection_preserves_api_value_without_creating_store_paths() {
    let directory = private_root();
    let root = directory.path().to_str().unwrap();
    let expected = PaleApi::new()
        .unwrap()
        .inspect_local_control_store(principal_request(root))
        .unwrap();
    let outcome = CommandDispatcher::new()
        .unwrap()
        .dispatch(CommandRequest::LocalControlStorageInspection(
            principal_request(root),
        ))
        .unwrap();
    let CommandOutcome::LocalControlStorageInspection(actual) = outcome else {
        panic!("inspection must preserve its typed API outcome");
    };
    assert_eq!(actual, expected);
    let output = command(root)
        .args(["principal", "--principal-id", "principal-inspection"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("{}\n", serde_json::to_string_pretty(&expected).unwrap())
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn deployment_coordinates_select_a_distinct_read_only_store_scope() {
    let directory = private_root();
    let root = directory.path().to_str().unwrap();
    let digest = "0".repeat(64);
    let expected = PaleApi::new()
        .unwrap()
        .inspect_local_control_store(LocalControlStorageInspectionRequest::deployment(
            root.to_owned(),
            "workspace-inspection".to_owned(),
            "deployment-inspection".to_owned(),
            digest.clone(),
            "store-inspection".to_owned(),
            "incarnation-inspection".to_owned(),
        ))
        .unwrap();
    let output = command(root)
        .args([
            "deployment",
            "--workspace-id",
            "workspace-inspection",
            "--deployment-binding-ref",
            "deployment-inspection",
            "--deployment-binding-sha256",
            &digest,
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn invalid_inspection_coordinates_preserve_the_api_diagnostic() {
    let root = "relative/private-runtime-root";
    let expected = PaleApi::new()
        .unwrap()
        .inspect_local_control_store(principal_request(root))
        .unwrap_err();
    let output = command(root)
        .args(["principal", "--principal-id", "principal-inspection"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    let expected_json: serde_json::Value =
        serde_json::from_slice(expected.diagnostic().canonical_bytes()).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stderr).unwrap()["diagnostic"],
        expected_json
    );
    assert!(!String::from_utf8(output.stderr).unwrap().contains(root));
}

#[test]
fn scope_arguments_cannot_mix_principal_and_deployment_coordinates() {
    let output = command("/unused-root")
        .args([
            "principal",
            "--principal-id",
            "principal-inspection",
            "--workspace-id",
            "workspace-inspection",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}
