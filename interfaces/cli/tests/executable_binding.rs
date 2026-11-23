//! Effectful commands cannot borrow another installation's executable binding.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::process::Command;

#[path = "support/activation_fixture.rs"]
pub mod activation_fixture;

#[path = "support/test_executable.rs"]
mod test_executable;

#[test]
fn alternate_launcher_cannot_apply_with_a_genuine_installations_binding() {
    let temp = tempfile::tempdir().unwrap();
    let executable = test_executable::snapshot(temp.path()).unwrap();
    let workspace = temp.path().join("workspace");
    let target = temp.path().join("target");
    let authority = temp.path().join("authority");
    let state = temp.path().join("state");
    let lineage = temp.path().join("lineage");
    for directory in [&target, &authority, &state, &lineage] {
        std::fs::create_dir_all(directory).unwrap();
    }
    std::fs::write(authority.join("README.md"), "# authority\n").unwrap();
    let installed = executable.path();
    let manifest = activation_fixture::lay_out(
        &workspace,
        &activation_fixture::MachineInputs {
            target: &target,
            auth: &authority,
            exec: installed,
            digest: executable.sha256(),
            state_root: &state,
            lineage_root: &lineage,
        },
    );
    let plans = temp.path().join("plans");
    let base = [
        "--workspace",
        manifest.to_str().unwrap(),
        "--project",
        "act",
        "--execution-workspace",
        "default",
        "--machine",
        "m",
    ];
    let planned = Command::new(installed)
        .args(base)
        .args(["plan", "--out", plans.to_str().unwrap(), "--kind", "apply"])
        .output()
        .unwrap();
    assert!(planned.status.success());
    let alternate = temp.path().join("alternate-pale");
    std::fs::copy(installed, &alternate).unwrap();
    std::fs::set_permissions(&alternate, std::fs::Permissions::from_mode(0o755)).unwrap();
    let receipts = temp.path().join("receipts");
    let rejected = Command::new(alternate)
        .args(base)
        .args([
            "apply",
            "--plans",
            plans.to_str().unwrap(),
            "--receipts",
            receipts.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !rejected.status.success(),
        "identical bytes at another path are not the admitted running installation"
    );
    assert!(!target.join(".pale/materialization-state.json").exists());
}
