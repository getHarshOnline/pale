//! End-to-end review/apply checks using the maintained two-host starter.

use super::*;
use std::os::unix::fs::{symlink, PermissionsExt};

fn remembered(manifest: &Path, host: &str, intent: &str) -> std::process::Output {
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
        ])
        .output()
        .unwrap()
}

#[test]
fn readable_plan_remembered_apply_and_git_exclusions_work_for_both_hosts() {
    for (host, entry) in [("codex", "AGENTS.md"), ("claude-code", "AGENTS.md")] {
        let (root, manifest) = starter();
        let target = root.path().join("repository");
        assert!(Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&target)
            .status()
            .unwrap()
            .success());
        let original = "# Project exceptions\n/cache/\n!important.txt";
        std::fs::write(target.join(".gitignore"), original).unwrap();
        let missing = remembered(&manifest, host, "apply");
        assert_eq!(missing.status.code(), Some(5));
        assert!(String::from_utf8_lossy(&missing.stderr).contains("run guidance plan"));
        let output = selected(&manifest)
            .args(["guidance", "plan", "--repository", "source", "--host", host])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains(&format!("update {entry}")), "{text}");
        assert!(text.contains("add    .pale/"));
        assert!(text.contains(&format!("--- {entry} (current)\n+++ {entry} (proposed)")));
        assert!(text.contains(" Project facts and acceptance criteria remain here."));
        assert!(text.contains("+/.pale-source-guidance-recovery.json"));
        assert_eq!(
            std::fs::read_to_string(target.join(".gitignore")).unwrap(),
            original
        );
        let review_path = target.join(format!(".pale-source-guidance-review-{host}.json"));
        assert_eq!(
            std::fs::metadata(&review_path)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        let installed = result(&remembered(&manifest, host, "apply"));
        assert!(installed["changed_files"].as_u64().unwrap() > 0);
        let ignores = std::fs::read_to_string(target.join(".gitignore")).unwrap();
        assert!(ignores.starts_with(original));
        for path in [
            ".pale-source-guidance.lock",
            ".pale-source-guidance-recovery.json",
            ".pale-source-guidance-review-codex.json",
            ".pale-source-guidance-review-claude-code.json",
        ] {
            assert!(
                Command::new("git")
                    .args(["check-ignore", "--quiet", path])
                    .current_dir(&target)
                    .status()
                    .unwrap()
                    .success(),
                "{path}"
            );
        }
        assert!(!Command::new("git")
            .args(["check-ignore", "--quiet", "important.txt"])
            .current_dir(&target)
            .status()
            .unwrap()
            .success());
        // Reusing the pre-install proposal must not silently review its own writes.
        assert_eq!(remembered(&manifest, host, "apply").status.code(), Some(5));
        let unchanged = file_plan(&manifest, host, "apply");
        assert!(unchanged["publication_order"]
            .as_array()
            .unwrap()
            .is_empty());
        assert_eq!(
            result(&remembered(&manifest, host, "apply"))["changed_files"],
            0
        );
        // Rules outside the managed block remain user-owned, even on removal.
        std::fs::write(
            target.join(".gitignore"),
            format!("{ignores}\n/another-cache/\n"),
        )
        .unwrap();
        let before_remove = std::fs::read(target.join(".gitignore")).unwrap();
        file_plan(&manifest, host, "remove");
        result(&remembered(&manifest, host, "remove"));
        assert_eq!(
            std::fs::read(target.join(".gitignore")).unwrap(),
            before_remove
        );
        assert_eq!(
            std::fs::read_to_string(target.join(entry)).unwrap(),
            "Project facts and acceptance criteria remain here.\n"
        );
    }
}

#[test]
fn remembered_review_refuses_changed_sources_targets_modes_and_unsafe_cache() {
    let (root, manifest) = starter();
    let target = root.path().join("repository");
    file_plan(&manifest, "codex", "apply");
    std::fs::write(target.join("AGENTS.md"), "Changed after review.\n").unwrap();
    let before = files(&target);
    assert_eq!(
        remembered(&manifest, "codex", "apply").status.code(),
        Some(5)
    );
    assert_eq!(files(&target), before);
    file_plan(&manifest, "codex", "apply");
    std::fs::set_permissions(
        target.join("AGENTS.md"),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    assert_eq!(
        remembered(&manifest, "codex", "apply").status.code(),
        Some(5)
    );
    file_plan(&manifest, "codex", "apply");
    let source = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(
        &manifest,
        source.replace("resource_id = \"change\"", "resource_id = \"security\""),
    )
    .unwrap();
    assert_eq!(
        remembered(&manifest, "codex", "apply").status.code(),
        Some(5)
    );
    file_plan(&manifest, "codex", "apply");
    assert_eq!(
        remembered(&manifest, "codex", "remove").status.code(),
        Some(5)
    );
    assert_eq!(
        remembered(&manifest, "claude-code", "apply").status.code(),
        Some(5)
    );
    let cache = target.join(".pale-source-guidance-review-codex.json");
    let outside = root.path().join("outside.json");
    std::fs::rename(&cache, &outside).unwrap();
    symlink(&outside, &cache).unwrap();
    let prior = std::fs::read(&outside).unwrap();
    assert!(!remembered(&manifest, "codex", "apply").status.success());
    let output = selected(&manifest)
        .args([
            "guidance",
            "plan",
            "--repository",
            "source",
            "--host",
            "codex",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(std::fs::read(outside).unwrap(), prior);
    assert!(!target.join(".pale-source-guidance.lock").exists());
}

#[test]
fn edited_exclusion_block_is_not_overwritten() {
    let (root, manifest) = starter();
    let target = root.path().join("repository");
    file_plan(&manifest, "codex", "apply");
    result(&remembered(&manifest, "codex", "apply"));
    let path = target.join(".gitignore");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replace("# END Pale", "/my-rule/\n# END Pale")).unwrap();
    let before = files(&target);
    let output = selected(&manifest)
        .args([
            "guidance",
            "plan",
            "--repository",
            "source",
            "--host",
            "codex",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(5));
    assert_eq!(files(&target), before);
}

#[test]
fn linked_worktree_install_keeps_git_administration_and_other_rules_intact() {
    let (root, manifest) = starter();
    let main = root.path().join("main");
    std::fs::create_dir(&main).unwrap();
    for args in [
        vec!["init", "--quiet"],
        vec![
            "-c",
            "user.name=Guidance test",
            "-c",
            "user.email=guidance@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "Fixture",
        ],
    ] {
        assert!(Command::new("git")
            .args(args)
            .current_dir(&main)
            .status()
            .unwrap()
            .success());
    }
    let target = root.path().join("linked");
    assert!(Command::new("git")
        .args(["worktree", "add", "--quiet", "--detach"])
        .arg(&target)
        .current_dir(&main)
        .status()
        .unwrap()
        .success());
    assert!(target.join(".git").is_file());
    let target = target.canonicalize().unwrap();
    let old = root.path().join("repository").canonicalize().unwrap();
    let machine = manifest.parent().unwrap().join("machines/local.toml");
    let source = std::fs::read_to_string(&machine).unwrap();
    std::fs::write(
        machine,
        source.replace(old.to_str().unwrap(), target.to_str().unwrap()),
    )
    .unwrap();
    std::fs::write(target.join(".gitignore"), "/user-cache/\n").unwrap();
    let admin = files(&main.join(".git"));
    for host in ["codex", "claude-code"] {
        file_plan(&manifest, host, "apply");
        result(&remembered(&manifest, host, "apply"));
        assert!(Command::new("git")
            .args([
                "check-ignore",
                "--quiet",
                ".pale-source-guidance-recovery.json"
            ])
            .current_dir(&target)
            .status()
            .unwrap()
            .success());
        assert!(std::fs::read_to_string(target.join(".gitignore"))
            .unwrap()
            .starts_with("/user-cache/\n"));
        file_plan(&manifest, host, "remove");
        result(&remembered(&manifest, host, "remove"));
    }
    assert_eq!(files(&main.join(".git")), admin);
}
