//! End-to-end materialization reconciliation, deselection, and full removal.

use super::*;

const MANAGED_BEGIN: &str = "<!-- pale:begin managed -->";
const MANAGED_END: &str = "<!-- pale:end managed -->";

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn operations(plan: &serde_json::Value) -> &[serde_json::Value] {
    plan["surface_plan"]["operations"].as_array().unwrap()
}

fn action(operation: &serde_json::Value) -> Option<&str> {
    operation["transition"]["action"].as_str()
}

fn materialization_state(target: &Path) -> serde_json::Value {
    read_json(&target.join(".pale/materialization-state.json"))
}

/// Remove the selected operation together with its private mechanics.
///
/// Deselecting semantic meaning alone would leave an impossible private
/// binding graph, so reconciliation fixtures change both owners atomically.
fn deselect_all_operations(path: &Path) {
    let mut capabilities: toml::Value =
        toml::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    capabilities["semantic"]["operations"] = toml::Value::Array(Vec::new());
    capabilities["operation_bindings"] = toml::Value::Array(Vec::new());
    capabilities["capability_bindings"] = toml::Value::Table(Default::default());
    capabilities["repository_commands"] = toml::Value::Table(Default::default());
    std::fs::write(path, toml::to_string_pretty(&capabilities).unwrap()).unwrap();
}

#[test]
fn materialization_reconciles_noop_deselection_and_full_removal() {
    use std::fs;

    let tmp = tempfile::tempdir().unwrap();
    let executable = test_executable::snapshot(tmp.path()).unwrap();
    let ws = tmp.path().join("ws");
    let target = tmp.path().join("target");
    let auth = tmp.path().join("auth");
    let state_root = tmp.path().join("state");
    let lineage_root = tmp.path().join("lineage");
    for directory in [&ws, &target, &auth, &state_root, &lineage_root] {
        fs::create_dir_all(directory).unwrap();
    }
    fs::write(auth.join("README.md"), "# authority doc\n").unwrap();
    fs::write(target.join("CLAUDE.md"), "claude user text").unwrap();
    fs::write(target.join("AGENTS.md"), "codex user text\n").unwrap();
    fs::write(target.join("UNRELATED.txt"), "keep\n").unwrap();
    let manifest = activation_fixture::lay_out(
        &ws,
        &activation_fixture::MachineInputs {
            target: &target,
            auth: &auth,
            exec: executable.path(),
            digest: executable.sha256(),
            state_root: &state_root,
            lineage_root: &lineage_root,
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
    let plan = |kind: &str, out: &Path| {
        Command::new(executable.path())
            .args(base)
            .args(["plan", "--out", out.to_str().unwrap(), "--kind", kind])
            .output()
            .unwrap()
    };
    let mutate = |kind: &str, plans: &Path, receipts: &Path| {
        Command::new(executable.path())
            .args(base)
            .args([
                kind,
                "--plans",
                plans.to_str().unwrap(),
                "--receipts",
                receipts.to_str().unwrap(),
            ])
            .output()
            .unwrap()
    };
    let read_plan =
        |directory: &Path, kind: &str| read_json(&directory.join(format!("alpha.{kind}.json")));

    let first_plans = tmp.path().join("plans-first");
    let first_receipts = tmp.path().join("receipts-first");
    assert!(plan("apply", &first_plans).status.success());
    let plan_files = fs::read_dir(&first_plans)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(plan_files, ["alpha.apply.json".to_string()].into());
    let first_apply = mutate("apply", &first_plans, &first_receipts);
    assert!(
        first_apply.status.success(),
        "first apply: {}",
        String::from_utf8_lossy(&first_apply.stderr)
    );
    let installed = materialization_state(&target);
    let operation_path = installed["surfaces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|surface| surface["kind"] == "operation-skill")
        .expect("selected operation has a native semantic surface")["rel_path"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(target.join(&operation_path).is_file());

    let no_op_plans = tmp.path().join("plans-noop");
    let no_op_receipts = tmp.path().join("receipts-noop");
    assert!(plan("apply", &no_op_plans).status.success());
    let no_op_plan = read_plan(&no_op_plans, "apply");
    assert!(operations(&no_op_plan)
        .iter()
        .all(|operation| action(operation) == Some("unchanged")));
    let no_op_apply = Command::new(executable.path())
        .args(["--result-format", "json"])
        .args(base)
        .args([
            "apply",
            "--plans",
            no_op_plans.to_str().unwrap(),
            "--receipts",
            no_op_receipts.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        no_op_apply.status.success(),
        "no-op apply: {}",
        String::from_utf8_lossy(&no_op_apply.stderr)
    );
    let no_op_result: serde_json::Value = serde_json::from_slice(&no_op_apply.stdout).unwrap();
    assert_eq!(no_op_result["completion"], "complete");
    assert_eq!(
        no_op_result["result"],
        read_json(&no_op_receipts.join("run-result.apply.json")),
        "post-effect JSON must embed the exact application-owned run result"
    );

    let capabilities_path = ws.join("projects/act/capabilities.toml");
    let capabilities: toml::Value =
        toml::from_str(&fs::read_to_string(&capabilities_path).unwrap()).unwrap();
    assert!(capabilities["semantic"]["operations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|operation| operation["resource"]["resource_id"].as_str() == Some("repo-scan")));
    deselect_all_operations(&capabilities_path);
    let reduced_plans = tmp.path().join("plans-reduced");
    let reduced_receipts = tmp.path().join("receipts-reduced");
    assert!(plan("apply", &reduced_plans).status.success());
    let reduced_plan = read_plan(&reduced_plans, "apply");
    assert!(operations(&reduced_plan).iter().any(|operation| {
        operation["rel_path"] == operation_path && action(operation) == Some("delete")
    }));
    let reduced_apply = mutate("apply", &reduced_plans, &reduced_receipts);
    assert!(
        reduced_apply.status.success(),
        "reconciled apply: {}",
        String::from_utf8_lossy(&reduced_apply.stderr)
    );
    assert!(!target.join(&operation_path).exists());

    let reduced = materialization_state(&target);
    let remove_plans = tmp.path().join("plans-remove");
    let remove_receipts = tmp.path().join("receipts-remove");
    assert!(plan("remove", &remove_plans).status.success());
    let removed = mutate("remove", &remove_plans, &remove_receipts);
    assert!(
        removed.status.success(),
        "full remove: {}",
        String::from_utf8_lossy(&removed.stderr)
    );
    for surface in reduced["surfaces"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|surface| surface["ownership"] == "whole_file")
    {
        let relative = surface["rel_path"].as_str().unwrap();
        assert!(
            !target.join(relative).exists(),
            "whole-file surface {} was retired",
            relative
        );
    }
    assert_eq!(
        fs::read_to_string(target.join("CLAUDE.md")).unwrap(),
        "claude user text"
    );
    assert_eq!(
        fs::read_to_string(target.join("AGENTS.md")).unwrap(),
        "codex user text\n"
    );
    assert_eq!(
        fs::read_to_string(target.join("UNRELATED.txt")).unwrap(),
        "keep\n"
    );
    assert!(!target.join(".pale/materialization-state.json").exists());
}

#[test]
fn explicit_adoption_establishes_lineage_without_implicit_overwrite() {
    use std::fs;

    let tmp = tempfile::tempdir().unwrap();
    let executable = test_executable::snapshot(tmp.path()).unwrap();
    let ws = tmp.path().join("ws");
    let target = tmp.path().join("target");
    let auth = tmp.path().join("auth");
    let state_root = tmp.path().join("state");
    let lineage_root = tmp.path().join("lineage");
    for directory in [&ws, &target, &auth, &state_root, &lineage_root] {
        fs::create_dir_all(directory).unwrap();
    }
    fs::write(auth.join("README.md"), "# authority doc\n").unwrap();
    fs::create_dir_all(target.join(".pale")).unwrap();
    let unowned_path = target.join(".pale/capabilities.md");
    fs::write(&unowned_path, "pre-existing unowned projection\n").unwrap();
    let unowned_managed = format!(
        "user prefix\n{}\npre-existing managed projection\n{}\nuser suffix\n",
        MANAGED_BEGIN, MANAGED_END
    );
    for name in ["AGENTS.md", "CLAUDE.md"] {
        fs::write(target.join(name), &unowned_managed).unwrap();
    }
    let manifest = activation_fixture::lay_out(
        &ws,
        &activation_fixture::MachineInputs {
            target: &target,
            auth: &auth,
            exec: executable.path(),
            digest: executable.sha256(),
            state_root: &state_root,
            lineage_root: &lineage_root,
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
    let ordinary_plans = tmp.path().join("plans-ordinary");
    assert!(Command::new(executable.path())
        .args(base)
        .args(["plan", "--out", ordinary_plans.to_str().unwrap()])
        .status()
        .unwrap()
        .success());
    let ordinary_receipts = tmp.path().join("receipts-ordinary");
    let ordinary_apply = Command::new(executable.path())
        .args(base)
        .args([
            "apply",
            "--plans",
            ordinary_plans.to_str().unwrap(),
            "--receipts",
            ordinary_receipts.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!ordinary_apply.status.success());
    assert_eq!(
        fs::read_to_string(&unowned_path).unwrap(),
        "pre-existing unowned projection\n"
    );
    assert!(!target.join(".pale/materialization-state.json").exists());

    let adoption_plans = tmp.path().join("plans-adoption");
    assert!(Command::new(executable.path())
        .args(base)
        .args([
            "plan",
            "--out",
            adoption_plans.to_str().unwrap(),
            "--adopt-unowned",
        ])
        .status()
        .unwrap()
        .success());
    let plan = read_json(&adoption_plans.join("alpha.apply.json"));
    assert!(operations(&plan).iter().any(|operation| {
        operation["rel_path"] == ".pale/capabilities.md" && action(operation) == Some("adopt")
    }));
    assert!(operations(&plan).iter().any(|operation| {
        operation["rel_path"] == "AGENTS.md"
            && operation["transition"]["ownership"] == "managed_block"
            && action(operation) == Some("adopt")
    }));
    assert!(!operations(&plan)
        .iter()
        .any(|operation| operation["rel_path"] == "CLAUDE.md"));

    let wrong_mode_receipts = tmp.path().join("receipts-wrong-mode");
    let wrong_mode = Command::new(executable.path())
        .args(base)
        .args([
            "apply",
            "--plans",
            adoption_plans.to_str().unwrap(),
            "--receipts",
            wrong_mode_receipts.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!wrong_mode.status.success());
    assert_eq!(
        fs::read_to_string(&unowned_path).unwrap(),
        "pre-existing unowned projection\n"
    );

    let adoption_receipts = tmp.path().join("receipts-adoption");
    let adopted = Command::new(executable.path())
        .args(base)
        .args([
            "apply",
            "--plans",
            adoption_plans.to_str().unwrap(),
            "--receipts",
            adoption_receipts.to_str().unwrap(),
            "--adopt-unowned",
        ])
        .output()
        .unwrap();
    assert!(
        adopted.status.success(),
        "adoption apply: {}",
        String::from_utf8_lossy(&adopted.stderr)
    );
    assert_ne!(
        fs::read_to_string(&unowned_path).unwrap(),
        "pre-existing unowned projection\n"
    );
    let adopted = fs::read_to_string(target.join("AGENTS.md")).unwrap();
    assert!(adopted.starts_with("user prefix\n"));
    assert!(adopted.ends_with("\nuser suffix\n"));
    assert!(!adopted.contains("pre-existing managed projection"));
    assert_eq!(
        fs::read_to_string(target.join("CLAUDE.md")).unwrap(),
        unowned_managed,
        "independent Claude instructions are outside this adoption"
    );
    assert!(target.join(".pale/materialization-state.json").is_file());
    let receipt_path = fs::read_dir(&adoption_receipts)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.to_string_lossy().ends_with(".receipt.json"))
        .unwrap();
    let receipt = read_json(&receipt_path);
    assert!(receipt["surface_operations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|operation| {
            operation["rel_path"] == ".pale/capabilities.md"
                && operation["observation"]["action"] == "adopt"
        }));
}

/// The receipt is the surface's evidence and is IMMUTABLE (no-clobber, filename
/// bound to the plan digest). Receipt construction AND persistence live inside the
/// reversible boundary, so if the evidence cannot be sealed the just-written
/// surface is rolled back — a surface is never left on disk without its receipt.
///
/// To exercise that path deterministically, apply once, deselect one semantic
/// surface, then preplant the exact immutable-receipt filename of that new plan.
/// The reconciliation reaches the evidence step after deleting the retired
/// surface and updating ownership state; the collision must restore all three.
#[test]
fn apply_rolls_back_reconciliation_when_its_receipt_cannot_be_sealed() {
    use std::fs;
    let tmp = tempfile::tempdir().unwrap();
    let executable = test_executable::snapshot(tmp.path()).unwrap();
    let ws = tmp.path().join("ws");
    let target = tmp.path().join("target");
    let auth = tmp.path().join("auth");
    let state = tmp.path().join("state");
    let lineage = tmp.path().join("lineage");
    fs::create_dir_all(&ws).unwrap();
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(&auth).unwrap();
    fs::create_dir_all(&state).unwrap();
    fs::create_dir_all(&lineage).unwrap();
    fs::write(auth.join("README.md"), b"# authority doc\n").unwrap();
    let manifest = activation_fixture::lay_out(
        &ws,
        &activation_fixture::MachineInputs {
            target: &target,
            auth: &auth,
            exec: executable.path(),
            digest: executable.sha256(),
            state_root: &state,
            lineage_root: &lineage,
        },
    );
    let ms = manifest.to_str().unwrap();
    let plans = tmp.path().join("plans");
    let receipts = tmp.path().join("receipts");

    let plan = || {
        Command::new(executable.path())
            .args([
                "--workspace",
                ms,
                "--project",
                "act",
                "--execution-workspace",
                "default",
                "--machine",
                "m",
                "plan",
                "--out",
                plans.to_str().unwrap(),
                "--kind",
                "apply",
            ])
            .output()
            .unwrap()
    };
    let apply = || {
        Command::new(executable.path())
            .args([
                "--workspace",
                ms,
                "--project",
                "act",
                "--execution-workspace",
                "default",
                "--machine",
                "m",
                "apply",
                "--plans",
                plans.to_str().unwrap(),
                "--receipts",
                receipts.to_str().unwrap(),
            ])
            .output()
            .unwrap()
    };
    let receipt_count = || {
        fs::read_dir(&receipts)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.file_name()))
            .filter(|n| n.to_string_lossy().ends_with(".receipt.json"))
            .count()
    };

    // Run 1: the surface is materialized and its immutable receipt is sealed.
    assert!(plan().status.success());
    let first = apply();
    assert!(
        first.status.success(),
        "first apply: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(receipt_count(), 1, "exactly one receipt after run 1");
    let installed = materialization_state(&target);
    let owned_before = installed["surfaces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|surface| {
            let relative = surface["rel_path"].as_str().unwrap();
            (
                relative.to_owned(),
                fs::read(target.join(relative)).unwrap(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let state_before = fs::read(target.join(".pale/materialization-state.json")).unwrap();
    let lineage_path = fs::read_dir(lineage.join("materialization-lineage"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let lineage_before = fs::read(&lineage_path).unwrap();

    let capabilities_path = ws.join("projects/act/capabilities.toml");
    deselect_all_operations(&capabilities_path);
    let second_plan = plan();
    assert!(
        second_plan.status.success(),
        "second plan: {}",
        String::from_utf8_lossy(&second_plan.stderr)
    );
    let saved = read_json(&plans.join("alpha.apply.json"));
    assert!(operations(&saved)
        .iter()
        .any(|operation| action(operation) == Some("delete")));
    let plan_digest = saved["plan_digest"].as_str().unwrap();
    let collision_path = receipts.join(format!("alpha.apply.{plan_digest}.receipt.json"));
    fs::write(&collision_path, "occupied\n").unwrap();
    let receipt_count_before = receipt_count();

    let second = Command::new(executable.path())
        .args([
            "--result-format",
            "json",
            "--error-format",
            "json",
            "--workspace",
            ms,
            "--project",
            "act",
            "--execution-workspace",
            "default",
            "--machine",
            "m",
            "apply",
            "--plans",
            plans.to_str().unwrap(),
            "--receipts",
            receipts.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        !second.status.success(),
        "the second apply must fail: its receipt cannot be sealed (collision)"
    );
    assert_eq!(second.status.code(), Some(1));
    let terminal: serde_json::Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(terminal["completion"], "complete");
    assert_eq!(
        terminal["result"]["targets"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()["outcome"],
        "failed_no_effect",
        "the exact run result carries failure separately from envelope completeness"
    );
    let diagnostic: serde_json::Value = serde_json::from_slice(&second.stderr).unwrap();
    assert!(diagnostic["diagnostic"]["code"].is_string());
    assert_eq!(
        diagnostic["diagnostic"]["disposition"],
        "operational_failure"
    );

    for (rel_path, bytes) in owned_before {
        assert_eq!(
            fs::read(target.join(&rel_path)).unwrap(),
            bytes,
            "owned surface {rel_path} restored exactly"
        );
    }
    assert_eq!(
        fs::read(target.join(".pale/materialization-state.json")).unwrap(),
        state_before,
        "target ownership state restored exactly"
    );
    assert_eq!(
        fs::read(lineage_path).unwrap(),
        lineage_before,
        "machine-bound lineage restored exactly"
    );
    assert_eq!(materialization_state(&target), installed);
    assert_eq!(
        receipt_count(),
        receipt_count_before,
        "no receipt was added"
    );
    assert_eq!(fs::read_to_string(collision_path).unwrap(), "occupied\n");

    // The failure is recorded honestly rather than left as a stale success.
    let rr = fs::read_to_string(receipts.join("run-result.apply.json")).unwrap();
    assert!(
        rr.contains("failed_no_effect"),
        "run-result must honestly record failed_no_effect, got: {rr}"
    );
}
