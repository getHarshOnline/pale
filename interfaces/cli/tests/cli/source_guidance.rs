//! Native CLI inspection and meaningful candidate review over the maintained starter.
//!
//! These cover native file changes and source guidance, not native agent judgment.

use super::configuration_review::files;
use super::*;

#[path = "source_guidance_review.rs"]
mod review;

#[test]
fn private_context_uses_declared_order_without_writing_or_publishing_it() {
    let (root, manifest) = starter();
    let private_root = root.path().join("private-owner");
    std::fs::create_dir(&private_root).unwrap();
    let mut source: toml::Value =
        toml::from_str(&std::fs::read_to_string(&manifest).unwrap()).unwrap();
    let topology = &mut source["workspace"]["desired_topology"];
    topology["projects"][0]["authority_root_ids"] =
        toml::Value::Array(vec!["owner-context".into()]);
    let session = toml::from_str::<toml::Value>(
        "activation_roots = ['source']\nauthority_access = [{root_id = 'owner-context', class = 'read-only'}]\nreading_order = [{root_id = 'owner-context', rel_path = 'Owner.md'}, {root_id = 'owner-context', rel_path = 'Duty.md'}]\n",
    )
    .unwrap();
    topology["execution_workspaces"][0]
        .as_table_mut()
        .unwrap()
        .insert("session".into(), session);
    std::fs::write(&manifest, toml::to_string(&source).unwrap()).unwrap();
    let machine_path = manifest.parent().unwrap().join("machines/local.toml");
    let mut machine: toml::Value =
        toml::from_str(&std::fs::read_to_string(&machine_path).unwrap()).unwrap();
    machine["authority_roots"].as_table_mut().unwrap().insert(
        "owner-context".into(),
        private_root.to_str().unwrap().into(),
    );
    std::fs::write(machine_path, toml::to_string(&machine).unwrap()).unwrap();
    let before = files(root.path());
    let output = selected(&manifest)
        .args(["guidance", "context", "--repository", "source"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let context = String::from_utf8(output.stdout).unwrap();
    assert!(context.contains(private_root.to_str().unwrap()));
    assert!(context.find("Owner.md").unwrap() < context.find("Duty.md").unwrap());
    assert!(context.contains("not checked these source files"));
    let json = selected(&manifest)
        .args([
            "--result-format",
            "json",
            "guidance",
            "context",
            "--repository",
            "source",
        ])
        .output()
        .unwrap();
    assert_eq!(result(&json), context);
    for host in ["codex", "claude-code"] {
        let inspected = selected(&manifest)
            .args([
                "--result-format",
                "json",
                "guidance",
                "inspect",
                "--repository",
                "source",
                "--host",
                host,
            ])
            .output()
            .unwrap();
        let guidance = result(&inspected).to_string();
        assert!(!guidance.contains(private_root.to_str().unwrap()));
        assert!(!guidance.contains("Owner.md"));
    }
    assert_eq!(files(root.path()), before);
}

fn selected(manifest: &Path) -> Command {
    let mut command = Command::new(bin());
    command.args([
        "--workspace",
        manifest.to_str().unwrap(),
        "--project",
        "repository",
        "--execution-workspace",
        "local-guidance",
        "--machine",
        "local",
    ]);
    command
}

fn copy_sources(from: &Path, to: &Path) {
    for (relative, bytes) in files(from) {
        let path = to.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
}

fn starter() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let pale = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let workspace = root.path().join("workspace");
    copy_sources(&pale.join("docs/examples/guidance-workspace"), &workspace);
    let repository = root.path().join("repository");
    std::fs::create_dir_all(&repository).unwrap();
    let repository = repository.canonicalize().unwrap();
    for entry in ["AGENTS.md", "CLAUDE.md"] {
        std::fs::write(
            repository.join(entry),
            "Project facts and acceptance criteria remain here.\n",
        )
        .unwrap();
    }
    let path = workspace.join("machines/local.toml");
    let template = std::fs::read_to_string(&path).unwrap();
    let machine = template
        .replace("/absolute/path/to/pale", pale.to_str().unwrap())
        .replace(
            "/absolute/path/to/your/repository",
            repository.to_str().unwrap(),
        );
    assert_ne!(template, machine);
    std::fs::write(path, machine).unwrap();
    let manifest = workspace.join("pale-workspace.toml");
    let inspected = selected(&manifest)
        .args(["distribution-info", "--lockfile"])
        .output()
        .unwrap();
    assert!(
        inspected.status.success(),
        "{}",
        String::from_utf8_lossy(&inspected.stderr)
    );
    assert!(inspected.stderr.is_empty());
    let lock: toml::Value =
        toml::from_str(std::str::from_utf8(&inspected.stdout).unwrap()).unwrap();
    assert!(lock["package"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["id"].as_str() == Some("pale.engineering")));
    std::fs::write(workspace.join("distribution.lock.toml"), inspected.stdout).unwrap();
    (root, manifest)
}

#[test]
fn native_guidance_planning_matches_sdk_and_preserves_instructions() {
    let (root, manifest) = starter();
    let before = files(root.path());
    let api = pale_sdk::PaleApi::new().unwrap();
    for (host, layout, entry) in [
        ("codex", pale_sdk::SourceGuidanceHost::Codex, "AGENTS.md"),
        (
            "claude-code",
            pale_sdk::SourceGuidanceHost::ClaudeCode,
            "AGENTS.md",
        ),
    ] {
        let output = selected(&manifest)
            .args([
                "--result-format",
                "json",
                "guidance",
                "plan",
                "--repository",
                "source",
                "--host",
                host,
                "--intent",
                "apply",
            ])
            .output()
            .unwrap();
        let observed = result(&output);
        let expected = api
            .plan_local_source_guidance(pale_sdk::LocalSourceGuidancePlanRequest::new(
                pale_sdk::LocalOneShotCompilationRequest::new(
                    pale_sdk::LocalWorkspaceLocator::new(manifest.to_str().unwrap().to_owned()),
                    "repository".into(),
                    "local-guidance".into(),
                    "local".into(),
                ),
                "source".into(),
                layout,
                pale_sdk::LocalSourceGuidanceIntent::Apply,
            ))
            .unwrap();
        assert_eq!(observed, serde_json::to_value(expected.report()).unwrap());
        assert_eq!(observed["intent"], "apply");
        let entry = observed["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["rel_path"] == entry)
            .unwrap();
        assert_eq!(entry["baseline"]["kind"], "present");
        assert!(entry["change"]["content"]
            .as_str()
            .unwrap()
            .starts_with("Project facts and acceptance criteria remain here.\n"));
        let human = selected(&manifest)
            .args([
                "guidance",
                "plan",
                "--repository",
                "source",
                "--host",
                host,
                "--intent",
                "remove",
            ])
            .output()
            .unwrap();
        assert!(
            human.status.success(),
            "{}",
            String::from_utf8_lossy(&human.stderr)
        );
        assert!(String::from_utf8(human.stdout)
            .unwrap()
            .contains("Instruction files are unchanged"));
    }
    let mut after = files(root.path());
    for host in ["codex", "claude-code"] {
        assert!(after
            .remove(&PathBuf::from(format!(
                "repository/.pale-source-guidance-review-{host}.json"
            )))
            .is_some());
    }
    assert_eq!(after, before);
}

fn inspect(manifest: &Path, host: &str) -> std::process::Output {
    selected(manifest)
        .args([
            "--result-format",
            "json",
            "--error-format",
            "json",
            "guidance",
            "inspect",
            "--repository",
            "source",
            "--host",
            host,
        ])
        .output()
        .unwrap()
}

fn result(output: &std::process::Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["completion"], "complete");
    envelope["result"].clone()
}

fn file_plan(manifest: &Path, host: &str, intent: &str) -> serde_json::Value {
    result(
        &selected(manifest)
            .args([
                "--result-format",
                "json",
                "guidance",
                "plan",
                "--repository",
                "source",
                "--host",
                host,
                "--intent",
                intent,
            ])
            .output()
            .unwrap(),
    )
}
fn apply(manifest: &Path, host: &str, intent: &str, digest: &str) -> std::process::Output {
    selected(manifest)
        .args([
            "--result-format",
            "json",
            "--error-format",
            "json",
            "guidance",
            "apply",
            "--repository",
            "source",
            "--host",
            host,
            "--intent",
            intent,
            "--expect-plan",
            digest,
        ])
        .output()
        .unwrap()
}

#[test]
fn cli_reviewed_install_role_update_and_removal_preserve_local_context_for_both_hosts() {
    for (host, entry) in [("codex", "AGENTS.md"), ("claude-code", "AGENTS.md")] {
        let (root, manifest) = starter();
        let target = root.path().join("repository");
        let original = std::fs::read_to_string(target.join(entry)).unwrap();
        let initial = file_plan(&manifest, host, "apply");
        std::fs::write(
            target.join(entry),
            format!("{original}A new local acceptance criterion.\n"),
        )
        .unwrap();
        let stale = apply(
            &manifest,
            host,
            "apply",
            initial["digest"].as_str().unwrap(),
        );
        assert_eq!(stale.status.code(), Some(5));
        assert!(stale.stdout.is_empty());
        assert!(!target.join(".pale-source-guidance.lock").exists());
        let initial = file_plan(&manifest, host, "apply");
        let old_role = initial["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["rel_path"].as_str().unwrap().ends_with("change.md"))
            .unwrap()["rel_path"]
            .as_str()
            .unwrap()
            .to_owned();
        let installed = result(&apply(
            &manifest,
            host,
            "apply",
            initial["digest"].as_str().unwrap(),
        ));
        assert!(installed["changed_files"].as_u64().unwrap() > 0);
        assert!(target.join(&old_role).is_file());
        let clean = file_plan(&manifest, host, "apply");
        assert!(clean["publication_order"].as_array().unwrap().is_empty());
        let source = std::fs::read_to_string(&manifest).unwrap();
        assert!(source.contains("resource_id = \"change\""));
        std::fs::write(
            &manifest,
            source.replace("resource_id = \"change\"", "resource_id = \"security\""),
        )
        .unwrap();
        let before = files(&target);
        let stale = apply(&manifest, host, "apply", clean["digest"].as_str().unwrap());
        assert_eq!(stale.status.code(), Some(5));
        assert_eq!(
            files(&target),
            before,
            "fresh source changes must require a new review"
        );
        let changed = file_plan(&manifest, host, "apply");
        let new_role = changed["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["rel_path"].as_str().unwrap().ends_with("security.md"))
            .unwrap()["rel_path"]
            .as_str()
            .unwrap()
            .to_owned();
        result(&apply(
            &manifest,
            host,
            "apply",
            changed["digest"].as_str().unwrap(),
        ));
        assert!(!target.join(old_role).exists());
        assert!(std::fs::read_to_string(target.join(new_role))
            .unwrap()
            .contains("Review a specific part of the system"));
        let text = std::fs::read_to_string(target.join(entry)).unwrap();
        std::fs::write(
            target.join(entry),
            format!("{text}Later project-specific adaptation.\n"),
        )
        .unwrap();
        let remove = file_plan(&manifest, host, "remove");
        result(&apply(
            &manifest,
            host,
            "remove",
            remove["digest"].as_str().unwrap(),
        ));
        assert_eq!(
            std::fs::read_to_string(target.join(entry)).unwrap(),
            format!(
                "{original}A new local acceptance criterion.\nLater project-specific adaptation.\n"
            )
        );
        assert!(!target.join(".pale-source-guidance-recovery.json").exists());
        assert!(target.join(".pale-source-guidance.lock").is_file());
    }
}

#[test]
fn cli_recovery_absence_and_invalid_review_remain_distinct_from_execution() {
    let (root, manifest) = starter();
    let before = files(root.path());
    let output = selected(&manifest)
        .args([
            "--result-format",
            "json",
            "guidance",
            "recovery",
            "plan",
            "--repository",
            "source",
            "--host",
            "codex",
            "--intent",
            "restore-before",
        ])
        .output()
        .unwrap();
    assert!(result(&output).is_null());
    let output = selected(&manifest)
        .args([
            "--result-format",
            "json",
            "--error-format",
            "json",
            "guidance",
            "recovery",
            "apply",
            "--repository",
            "source",
            "--host",
            "codex",
            "--intent",
            "restore-before",
            "--expect-recovery",
            &"0".repeat(64),
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(5));
    assert!(output.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        value["diagnostic"]["code"],
        "source_guidance_recovery_absent"
    );
    assert_eq!(files(root.path()), before);
}

#[test]
fn maintained_starter_inspects_both_hosts_and_reviews_a_real_role_change_without_writes() {
    let (root, manifest) = starter();
    let before = files(root.path());
    let human = selected(&manifest)
        .args([
            "guidance",
            "inspect",
            "--repository",
            "source",
            "--host",
            "codex",
        ])
        .output()
        .unwrap();
    assert!(human.status.success());
    let human = String::from_utf8(human.stdout).unwrap();
    assert!(human.contains("Proposed addition to AGENTS.md"));
    assert!(human.contains(".pale/guidance/codex/index.md"));
    let api = pale_sdk::PaleApi::new().unwrap();
    let profile = api
        .compile_local_effective_profile(pale_sdk::LocalOneShotCompilationRequest::new(
            pale_sdk::LocalWorkspaceLocator::new(manifest.to_str().unwrap().to_owned()),
            "repository".into(),
            "local-guidance".into(),
            "local".into(),
        ))
        .unwrap();
    for (host, layout, entry) in [
        ("codex", pale_sdk::SourceGuidanceHost::Codex, "AGENTS.md"),
        (
            "claude-code",
            pale_sdk::SourceGuidanceHost::ClaudeCode,
            "AGENTS.md",
        ),
    ] {
        let observed = result(&inspect(&manifest, host));
        let expected =
            serde_json::to_value(profile.source_guidance("source", layout).unwrap()).unwrap();
        assert_eq!(
            observed, expected,
            "CLI must return the exact SDK-owned projection"
        );
        assert_eq!(observed["entrypoint_path"], entry);
        assert!(observed["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file["content"]
                .as_str()
                .unwrap()
                .contains("Read the repository's `AGENTS.md` and `CONTRIBUTING.md`")));
        assert!(!observed["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file["rel_path"].as_str() == Some(entry)));
    }
    assert_eq!(files(root.path()), before);
    let candidate = root.path().join("candidate");
    copy_sources(manifest.parent().unwrap(), &candidate);
    let candidate_manifest = candidate.join("pale-workspace.toml");
    let source = std::fs::read_to_string(&candidate_manifest).unwrap();
    assert!(source.contains("resource_id = \"change\""));
    std::fs::write(
        &candidate_manifest,
        source.replace("resource_id = \"change\"", "resource_id = \"security\""),
    )
    .unwrap();
    let before_review = files(root.path());
    let preview = selected(&manifest)
        .args([
            "review-configuration",
            "--candidate-workspace",
            candidate_manifest.to_str().unwrap(),
            "preview",
        ])
        .output()
        .unwrap();
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let summary: serde_json::Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert!(summary["profile_digest"].is_string());
    assert!(summary["candidate_profile_digest"].is_string());
    assert_eq!(
        summary["profile_digest"].as_str(),
        Some(profile.profile_digest())
    );
    assert_ne!(
        summary["profile_digest"],
        summary["candidate_profile_digest"]
    );
    let mut impacts = selected(&manifest)
        .args([
            "review-configuration",
            "--candidate-workspace",
            candidate_manifest.to_str().unwrap(),
            "impacts",
            "--limit",
            "100",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    impacts
        .stdin
        .take()
        .unwrap()
        .write_all(&preview.stdout)
        .unwrap();
    let impacts = impacts.wait_with_output().unwrap();
    assert!(
        impacts.status.success(),
        "{}",
        String::from_utf8_lossy(&impacts.stderr)
    );
    let page: serde_json::Value = serde_json::from_slice(&impacts.stdout).unwrap();
    assert!(
        page["nextOffset"].is_null(),
        "qualify the complete impact partition for this bounded starter"
    );
    assert!(page["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["state"].as_str() == Some("affected")));
    for host in ["codex", "claude-code"] {
        let projection = result(&inspect(&candidate_manifest, host));
        assert_eq!(
            projection["profile_digest"],
            summary["candidate_profile_digest"]
        );
        let role = projection["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|file| file["rel_path"].as_str().unwrap().ends_with("security.md"))
            .unwrap();
        let body = role["content"].as_str().unwrap();
        assert!(body.contains(
            "Review a specific part of the system by following attacker- or environment-controlled input"
        ));
        assert!(body.contains("Review the exact change against the requested outcome"));
        assert_eq!(
            body.matches(
                "Review a specific part of the system by following attacker- or environment-controlled input"
            )
            .count(),
            1
        );
    }
    assert_eq!(
        files(root.path()),
        before_review,
        "inspection and review must preserve every input and target file"
    );
}

#[test]
fn native_guidance_refuses_runtime_selections_and_unselected_repositories() {
    let (root, manifest) = starter();
    let output = selected(&manifest)
        .args([
            "--error-format",
            "json",
            "guidance",
            "inspect",
            "--repository",
            "unselected",
            "--host",
            "codex",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    let path = manifest.parent().unwrap().join("capabilities.toml");
    let source = std::fs::read_to_string(&path).unwrap();
    std::fs::write(path, format!("[semantic]\noperations = [{{ selection = 'exact', resource = {{ package_id = 'pale.engineering', resource_id = 'testing/repository' }} }}]\n\n[[operation_bindings]]\noperation = {{ package_id = 'pale.engineering', resource_id = 'testing/repository' }}\n[operation_bindings.binding.selection]\nkind = 'user_only'\naliases = []\n\n{source}")).unwrap();
    let before = files(root.path());
    for host in ["codex", "claude-code"] {
        let output = inspect(&manifest, host);
        assert_eq!(
            output.status.code(),
            Some(4),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
        let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(
            diagnostic["diagnostic"]["code"],
            "source_guidance_projection_rejected"
        );
        assert!(diagnostic["diagnostic"]["summary"]
            .as_str()
            .unwrap()
            .contains("require runtime projection"));
    }
    assert_eq!(files(root.path()), before);
}
