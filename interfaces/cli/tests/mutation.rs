//! Project mutation admission and command-boundary behavior.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[path = "support/activation_workspace.rs"]
pub mod activation_workspace;
#[path = "support/distribution.rs"]
mod synthetic_distribution;

#[path = "support/test_executable.rs"]
mod test_executable;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_pale")
}

fn add_engine_repository(workspace: &Path, repository_id: &str, target: &Path) {
    fn table(entries: impl IntoIterator<Item = (&'static str, toml::Value)>) -> toml::Value {
        toml::Value::Table(
            entries
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
        )
    }

    fn package_resource(package_id: &str, resource_id: &str) -> toml::Value {
        table([
            ("package_id", toml::Value::String(package_id.to_string())),
            ("resource_id", toml::Value::String(resource_id.to_string())),
        ])
    }

    let manifest_path = workspace.join("pale-workspace.toml");
    let mut manifest: toml::Value =
        toml::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    let topology = manifest["workspace"]["desired_topology"]
        .as_table_mut()
        .expect("fixture desired topology remains a TOML table");
    topology["repositories"]
        .as_array_mut()
        .unwrap()
        .push(table([
            ("id", toml::Value::String(repository_id.to_string())),
            ("title", toml::Value::String(repository_id.to_string())),
        ]));
    topology["project_repositories"]
        .as_array_mut()
        .unwrap()
        .push(table([
            ("project_id", toml::Value::String("act".to_string())),
            (
                "repository_id",
                toml::Value::String(repository_id.to_string()),
            ),
            ("role", package_resource("pale.engineering", "engine")),
            ("stacks", toml::Value::Array(Vec::new())),
            (
                "families",
                toml::Value::Array(vec![
                    package_resource("pale.engineering", "verification"),
                    package_resource("pale.governance", "authority-resolver"),
                ]),
            ),
            ("materialization_order", toml::Value::Integer(1)),
        ]));
    topology["execution_workspace_repositories"]
        .as_array_mut()
        .unwrap()
        .push(table([
            (
                "execution_workspace_id",
                toml::Value::String("default".to_string()),
            ),
            (
                "repository_id",
                toml::Value::String(repository_id.to_string()),
            ),
            ("access", toml::Value::String("mutate".to_string())),
        ]));
    fs::write(manifest_path, toml::to_string_pretty(&manifest).unwrap()).unwrap();

    let binding_path = workspace.join("bindings/machines/m.toml");
    let mut binding = fs::read_to_string(&binding_path).unwrap();
    let target = fs::canonicalize(target).unwrap();
    binding.push_str(&format!(
        "\n[[repository_checkouts]]\nrepository_id = {repository_id:?}\ncheckout_root = {:?}\n",
        target.display().to_string()
    ));
    fs::write(binding_path, binding).unwrap();
}

#[derive(Debug, Eq, PartialEq)]
struct SnapshotEntry {
    kind: &'static str,
    mode: u32,
    device: u64,
    inode: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    bytes: Vec<u8>,
}

fn snapshot_entry(kind: &'static str, metadata: &fs::Metadata, bytes: Vec<u8>) -> SnapshotEntry {
    SnapshotEntry {
        kind,
        mode: metadata.permissions().mode(),
        device: metadata.dev(),
        inode: metadata.ino(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        bytes,
    }
}

struct Fixture {
    temporary: tempfile::TempDir,
    executable: test_executable::TestExecutable,
    manifest: PathBuf,
    alpha: PathBuf,
    beta: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let executable = test_executable::snapshot(temporary.path()).unwrap();
        let ws = temporary.path().join("workspace");
        let alpha = temporary.path().join("alpha");
        let beta = temporary.path().join("beta");
        let auth = temporary.path().join("authority");
        let state = temporary.path().join("state");
        let lineage = temporary.path().join("lineage");
        for directory in [&ws, &alpha, &beta, &auth, &state, &lineage] {
            fs::create_dir_all(directory).unwrap();
        }
        fs::write(alpha.join("owner.txt"), b"alpha\n").unwrap();
        fs::write(beta.join("owner.txt"), b"beta\n").unwrap();
        fs::write(auth.join("README.md"), b"# authority\n").unwrap();
        let manifest = activation_workspace::lay_out(
            &ws,
            &activation_workspace::MachineInputs {
                target: &alpha,
                auth: &auth,
                exec: executable.path(),
                digest: executable.sha256(),
                state_root: &state,
                lineage_root: &lineage,
            },
        );
        add_engine_repository(&ws, "beta", &beta);
        Self {
            temporary,
            executable,
            manifest,
            alpha,
            beta,
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(self.executable.path());
        command.args([
            "--workspace",
            self.manifest.to_str().unwrap(),
            "--project",
            "act",
            "--execution-workspace",
            "default",
            "--machine",
            "m",
        ]);
        command
    }

    fn plan(&self, kind: &str, out: &Path) -> Output {
        self.command()
            .args(["plan", "--out", out.to_str().unwrap(), "--kind", kind])
            .output()
            .unwrap()
    }

    fn mutate(&self, kind: &str, plans: &Path, receipts: &Path) -> Output {
        self.command()
            .args([
                kind,
                "--plans",
                plans.to_str().unwrap(),
                "--receipts",
                receipts.to_str().unwrap(),
            ])
            .output()
            .unwrap()
    }
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, SnapshotEntry> {
    fn visit(root: &Path, current: &Path, entries: &mut BTreeMap<PathBuf, SnapshotEntry>) {
        let mut children = fs::read_dir(current)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        children.sort();
        for path in children {
            let metadata = fs::symlink_metadata(&path).unwrap();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            if metadata.file_type().is_dir() {
                entries.insert(relative, snapshot_entry("directory", &metadata, Vec::new()));
                visit(root, &path, entries);
            } else if metadata.file_type().is_symlink() {
                entries.insert(
                    relative,
                    snapshot_entry(
                        "symlink",
                        &metadata,
                        fs::read_link(&path)
                            .unwrap()
                            .as_os_str()
                            .as_encoded_bytes()
                            .to_vec(),
                    ),
                );
            } else {
                entries.insert(
                    relative,
                    snapshot_entry("file", &metadata, fs::read(path).unwrap()),
                );
            }
        }
    }

    let mut entries = BTreeMap::new();
    visit(root, root, &mut entries);
    entries
}

#[test]
fn every_saved_plan_is_admitted_before_apply_or_remove_can_touch_any_target() {
    let fixture = Fixture::new();
    let apply_plans = fixture.temporary.path().join("apply-plans");
    let rejected_apply_receipts = fixture.temporary.path().join("rejected-apply-receipts");
    let planned = fixture.plan("apply", &apply_plans);
    assert!(
        planned.status.success(),
        "apply planning failed: {}",
        String::from_utf8_lossy(&planned.stderr)
    );
    fs::remove_file(apply_plans.join("beta.apply.json")).unwrap();
    let alpha_before = snapshot(&fixture.alpha);
    let beta_before = snapshot(&fixture.beta);

    let rejected = fixture.mutate("apply", &apply_plans, &rejected_apply_receipts);

    assert!(!rejected.status.success());
    assert_eq!(snapshot(&fixture.alpha), alpha_before);
    assert_eq!(snapshot(&fixture.beta), beta_before);
    assert!(!rejected_apply_receipts.exists());

    let replanned = fixture.plan("apply", &apply_plans);
    assert!(replanned.status.success());
    let valid_apply_receipts = fixture.temporary.path().join("valid-apply-receipts");
    let applied = fixture.mutate("apply", &apply_plans, &valid_apply_receipts);
    assert!(
        applied.status.success(),
        "valid apply failed: {}",
        String::from_utf8_lossy(&applied.stderr)
    );

    let remove_plans = fixture.temporary.path().join("remove-plans");
    let planned = fixture.plan("remove", &remove_plans);
    assert!(planned.status.success());
    let beta_plan_path = remove_plans.join("beta.remove.json");
    let mut beta_plan: serde_json::Value =
        serde_json::from_slice(&fs::read(&beta_plan_path).unwrap()).unwrap();
    beta_plan["plan_digest"] = serde_json::Value::String("0".repeat(64));
    fs::write(&beta_plan_path, serde_json::to_vec(&beta_plan).unwrap()).unwrap();
    let alpha_before = snapshot(&fixture.alpha);
    let beta_before = snapshot(&fixture.beta);
    let rejected_remove_receipts = fixture.temporary.path().join("rejected-remove-receipts");

    let rejected = fixture.mutate("remove", &remove_plans, &rejected_remove_receipts);

    assert!(!rejected.status.success());
    assert_eq!(snapshot(&fixture.alpha), alpha_before);
    assert_eq!(snapshot(&fixture.beta), beta_before);
    assert!(!rejected_remove_receipts.exists());
}

#[test]
fn planning_preserves_a_confirmed_prefix_when_later_publication_fails() {
    let fixture = Fixture::new();
    let plans = fixture.temporary.path().join("plans");
    let occupied = plans.join("beta.apply.json");
    fs::create_dir_all(&occupied).unwrap();

    let output = fixture.plan("apply", &plans);

    assert!(!output.status.success());
    assert!(plans.join("alpha.apply.json").is_file());
    assert!(occupied.is_dir());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("plan alpha [apply]:"));
    assert!(!stdout.contains("plan beta [apply]:"));
}

#[test]
fn status_preserves_a_reported_prefix_when_later_observation_fails() {
    let fixture = Fixture::new();
    fs::remove_dir_all(&fixture.beta).unwrap();

    let output = fixture.command().arg("surface-status").output().unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("alpha:"));
    assert!(!stdout.contains("beta:"));
}

#[test]
fn surface_status_json_is_the_complete_sdk_result_and_writes_nothing() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.alpha.join(".git/info")).unwrap();
    fs::write(
        fixture.alpha.join(".git/info/exclude"),
        "# user-owned exclusion\n",
    )
    .unwrap();
    let before = snapshot(fixture.temporary.path());

    let human = fixture.command().arg("surface-status").output().unwrap();
    let json = fixture
        .command()
        .args(["--result-format", "json", "surface-status"])
        .output()
        .unwrap();

    assert!(human.status.success());
    assert!(json.status.success());
    assert!(human.stderr.is_empty());
    assert!(json.stderr.is_empty());
    assert_eq!(snapshot(fixture.temporary.path()), before);
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(value["completion"], "complete");
    assert_eq!(value["result"]["clean"], false);
    assert_eq!(value["result"]["repositories"][0]["repositoryId"], "alpha");
    assert_eq!(value["result"]["repositories"][1]["repositoryId"], "beta");
    let repositories = value["result"]["repositories"].as_array().unwrap();
    assert!(repositories[0]["drift"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| {
            row["kind"] == "git-exclude-protection"
                && row["path"] == "info/exclude"
                && row["action"] == "splice"
        }));
    let expected_human = repositories
        .iter()
        .map(|repository| {
            let repository_id = repository["repositoryId"].as_str().unwrap();
            let paths = repository["drift"]
                .as_array()
                .unwrap()
                .iter()
                .map(|drift| match drift["kind"].as_str().unwrap() {
                    "git-exclude-protection" => {
                        format!("git_exclude_protection:{}", drift["path"].as_str().unwrap())
                    }
                    _ => drift["path"].as_str().unwrap().to_owned(),
                })
                .collect::<Vec<_>>();
            format!("{repository_id}: drift on {}\n", paths.join(", "))
        })
        .collect::<String>()
        .into_bytes();
    assert_eq!(human.stdout, expected_human);
    for repository in repositories {
        for drift in repository["drift"].as_array().unwrap() {
            assert_eq!(
                drift
                    .as_object()
                    .unwrap()
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>(),
                vec!["action", "kind", "path"]
            );
        }
    }

    let drift_exit = fixture
        .command()
        .args(["--result-format", "json", "surface-status", "--exit-code"])
        .output()
        .unwrap();
    assert_eq!(drift_exit.status.code(), Some(3));
    assert_eq!(drift_exit.stdout, json.stdout);
    assert!(drift_exit.stderr.is_empty());
    assert_eq!(snapshot(fixture.temporary.path()), before);
}

#[test]
fn surface_status_exit_code_distinguishes_drift_from_a_clean_target() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.alpha.join(".git/info")).unwrap();
    fs::write(
        fixture.alpha.join(".git/info/exclude"),
        "# user-owned exclusion\n",
    )
    .unwrap();
    let reported = fixture.command().arg("surface-status").output().unwrap();
    assert!(reported.status.success());
    let reported_stdout = String::from_utf8_lossy(&reported.stdout);
    assert!(reported_stdout.contains("drift on"));
    assert!(reported_stdout.contains("git_exclude_protection:info/exclude"));

    let drift = fixture
        .command()
        .args(["surface-status", "--exit-code"])
        .output()
        .unwrap();

    assert_eq!(drift.status.code(), Some(3));

    let plans = fixture.temporary.path().join("plans");
    let receipts = fixture.temporary.path().join("receipts");
    assert!(fixture.plan("apply", &plans).status.success());
    assert!(fixture.mutate("apply", &plans, &receipts).status.success());
    let clean = fixture
        .command()
        .args(["surface-status", "--exit-code"])
        .output()
        .unwrap();
    assert!(clean.status.success());
    assert_eq!(clean.stdout, b"alpha: no drift\nbeta: no drift\n");

    let before = snapshot(fixture.temporary.path());
    let clean_json = fixture
        .command()
        .args(["--result-format", "json", "surface-status"])
        .output()
        .unwrap();
    assert!(clean_json.status.success());
    assert!(clean_json.stderr.is_empty());
    assert_eq!(snapshot(fixture.temporary.path()), before);
    let value: serde_json::Value = serde_json::from_slice(&clean_json.stdout).unwrap();
    assert_eq!(
        value,
        serde_json::json!({
            "completion": "complete",
            "result": {
                "clean": true,
                "repositories": [
                    {"repositoryId": "alpha", "drift": []},
                    {"repositoryId": "beta", "drift": []}
                ]
            }
        })
    );
}

#[test]
fn surface_status_observes_the_exact_owned_region_after_apply() {
    let fixture = Fixture::new();
    let plans = fixture.temporary.path().join("plans");
    let receipts = fixture.temporary.path().join("receipts");
    assert!(fixture.plan("apply", &plans).status.success());
    assert!(fixture.mutate("apply", &plans, &receipts).status.success());

    let status = |path: Option<&str>| {
        let before = snapshot(fixture.temporary.path());
        let output = fixture
            .command()
            .args(["--result-format", "json", "surface-status", "--exit-code"])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(if path.is_some() { 3 } else { 0 }),
            "stdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["result"]["clean"], path.is_none());
        let drift = value["result"]["repositories"][0]["drift"]
            .as_array()
            .unwrap();
        assert_eq!(drift.len(), usize::from(path.is_some()));
        if let Some(path) = path {
            assert_eq!(drift[0]["path"], path);
            assert_eq!(drift[0]["action"], "update");
        }
        assert_eq!(snapshot(fixture.temporary.path()), before);
    };

    let whole_path = ".pale/capabilities.md";
    let whole = fixture.alpha.join(whole_path);
    let original = fs::read(&whole).unwrap();
    let mode = fs::metadata(&whole).unwrap().permissions();
    let edited = [original.as_slice(), b"\nchanged owned file\n"].concat();
    fs::write(&whole, edited).unwrap();
    status(Some(whole_path));
    let human = fixture
        .command()
        .args(["surface-status", "--exit-code"])
        .output()
        .unwrap();
    assert_eq!(human.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&human.stdout).contains(whole_path));

    fs::remove_file(&whole).unwrap();
    status(Some(whole_path));
    fs::write(&whole, original).unwrap();
    fs::set_permissions(&whole, fs::Permissions::from_mode(mode.mode() ^ 0o100)).unwrap();
    status(Some(whole_path));
    fs::set_permissions(&whole, mode).unwrap();
    status(None);

    let managed_path = "AGENTS.md";
    let managed = fixture.alpha.join(managed_path);
    let original = fs::read_to_string(&managed).unwrap();
    let with_user_text = format!("user prefix\n{original}\nuser suffix\n");
    fs::write(&managed, &with_user_text).unwrap();
    status(None);

    let end = "<!-- pale:end managed -->";
    assert!(with_user_text.contains(end));
    let edited = with_user_text.replacen(end, &format!("changed owned block\n{end}"), 1);
    fs::write(&managed, edited).unwrap();
    status(Some(managed_path));
    fs::write(&managed, with_user_text.replacen(end, "", 1)).unwrap();
    status(Some(managed_path));
}

#[test]
fn plan_saves_the_api_owned_local_plan_bytes_without_rebuilding_them() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.alpha.join(".git/info")).unwrap();
    fs::write(
        fixture.alpha.join(".git/info/exclude"),
        "# user-owned exclusion\n",
    )
    .unwrap();
    let api = pale_sdk::PaleApi::new().expect("embedded API dependencies are coherent");
    let compilation = api
        .compile_local_one_shot(pale_sdk::LocalOneShotCompilationRequest::new(
            pale_sdk::LocalWorkspaceLocator::new(
                fixture
                    .manifest
                    .to_str()
                    .expect("fixture path is UTF-8")
                    .to_owned(),
            ),
            "act".to_owned(),
            "default".to_owned(),
            "m".to_owned(),
        ))
        .expect("fixture compiles through the local API seam");
    let expected = compilation
        .summary()
        .repositories()
        .iter()
        .map(|repository| {
            let repository_id = repository.repository_id().to_owned();
            let plan = api
                .plan_local_one_shot(
                    &compilation,
                    pale_sdk::LocalOneShotPlanRequest::new(
                        repository_id.clone(),
                        pale_sdk::OneShotPlanKind::Apply,
                    ),
                )
                .expect("API builds the local apply candidate");
            (
                repository_id,
                (
                    format!(
                        "{}\n",
                        serde_json::to_string_pretty(
                            &serde_json::from_slice::<serde_json::Value>(
                                plan.document().canonical_bytes()
                            )
                            .unwrap()
                        )
                        .unwrap()
                    )
                    .into_bytes(),
                    plan.counts().total_operations(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let plans = fixture.temporary.path().join("plans");
    let output = fixture.plan("apply", &plans);
    assert!(
        output.status.success(),
        "CLI planning failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("CLI plan report is UTF-8");
    for (repository_id, (bytes, total_operations)) in expected {
        assert_eq!(
            fs::read(plans.join(format!("{repository_id}.apply.json"))).unwrap(),
            bytes
        );
        assert!(stdout.contains(&format!(
            "plan {repository_id} [apply]: {total_operations} operations ->"
        )));
    }
}

#[test]
fn plan_rejects_unknown_kind_and_compile_help_describes_actual_output() {
    let invalid = Command::new(bin())
        .args(["plan", "--out", "/tmp", "--kind", "migrate"])
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("possible values: apply, remove"));

    let incompatible_root = tempfile::tempdir().unwrap();
    let incompatible_output = incompatible_root.path().join("incompatible");
    let incompatible = Command::new(bin())
        .args([
            "plan",
            "--out",
            incompatible_output.to_str().unwrap(),
            "--kind",
            "remove",
            "--adopt-unowned",
        ])
        .output()
        .unwrap();
    assert_eq!(incompatible.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&incompatible.stderr)
        .contains("--adopt-unowned is valid only for apply plans"));
    assert!(!incompatible_output.exists());

    let help = Command::new(bin())
        .args(["compile", "--help"])
        .output()
        .unwrap();
    assert!(help.status.success());
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("generated surface counts"));
    assert!(help.contains("identity digests"));
}
