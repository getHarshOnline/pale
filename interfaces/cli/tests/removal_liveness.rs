//! Cleanup remains available when a previously admitted native harness disappears.

#![cfg(unix)]

use std::path::Path;
use std::process::Command;

#[path = "support/activation_fixture.rs"]
pub mod activation_fixture;

#[path = "support/test_executable.rs"]
mod test_executable;

fn run(executable: &Path, base: &[&str], args: &[&str]) -> std::process::Output {
    Command::new(executable)
        .args(base)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn owned_surfaces_can_be_removed_after_native_harness_loss() {
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

    let manifest = activation_fixture::lay_out(
        &workspace,
        &activation_fixture::MachineInputs {
            target: &target,
            auth: &authority,
            exec: executable.path(),
            digest: executable.sha256(),
            state_root: &state,
            lineage_root: &lineage,
        },
    );
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
    let apply_plans = temp.path().join("apply-plans");
    let apply_receipts = temp.path().join("apply-receipts");
    let planned = run(
        executable.path(),
        &base,
        &[
            "plan",
            "--out",
            apply_plans.to_str().unwrap(),
            "--kind",
            "apply",
        ],
    );
    assert!(
        planned.status.success(),
        "apply plan: {}",
        String::from_utf8_lossy(&planned.stderr)
    );
    let applied = run(
        executable.path(),
        &base,
        &[
            "apply",
            "--plans",
            apply_plans.to_str().unwrap(),
            "--receipts",
            apply_receipts.to_str().unwrap(),
        ],
    );
    assert!(
        applied.status.success(),
        "apply: {}",
        String::from_utf8_lossy(&applied.stderr)
    );

    std::fs::remove_dir_all(workspace.join("bindings/native-harnesses")).unwrap();
    let remove_plans = temp.path().join("remove-plans");
    let remove_receipts = temp.path().join("remove-receipts");
    let planned = run(
        executable.path(),
        &base,
        &[
            "plan",
            "--out",
            remove_plans.to_str().unwrap(),
            "--kind",
            "remove",
        ],
    );
    assert!(
        planned.status.success(),
        "cleanup planning must not probe the missing harnesses: {}",
        String::from_utf8_lossy(&planned.stderr)
    );
    let removed = run(
        executable.path(),
        &base,
        &[
            "remove",
            "--plans",
            remove_plans.to_str().unwrap(),
            "--receipts",
            remove_receipts.to_str().unwrap(),
        ],
    );
    assert!(
        removed.status.success(),
        "cleanup must remain live after harness loss: {}",
        String::from_utf8_lossy(&removed.stderr)
    );
    assert!(!target.join(".pale/materialization-state.json").exists());
}
