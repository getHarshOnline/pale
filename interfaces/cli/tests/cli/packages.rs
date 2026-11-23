//! Package authoring commands use the real reader without a workspace selection.

use super::*;

fn indexed_fixture(root: &Path) {
    for family in ["engineering", "governance"] {
        let folder = root.join("packages").join(family);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("package.toml"), format!(r#"[package]
id = "example.{family}"
version = "1.0.0"
schema_version = 1
title = "Fixture"
description = "Package index command fixture"
compatible_compilers = ["pale-surface@1"]
sources = [
  {{ path = "INDEX.md", role = "documentation" }},
  {{ path = "family.toml", role = "declaration" }},
  {{ path = "body.md", role = "body" }},
]
resources = [{{ id = "policy", kind = "policy_family", definition_source = "family.toml", exposure = "exported" }}]
"#)).unwrap();
        std::fs::write(
            folder.join("family.toml"),
            "id = 'policy'\ntitle = 'Policy'\npurpose = 'Fixture method'\nbody = 'body.md'\n",
        )
        .unwrap();
        std::fs::write(
            folder.join("body.md"),
            "# Policy\n\nKeep the authored method.\n",
        )
        .unwrap();
        std::fs::write(folder.join("INDEX.md"), "stale index\n").unwrap();
    }
}

fn check(root: &Path, write: bool) -> std::process::Output {
    let mut command = Command::new(bin());
    command.args(["packages", "check", "--root"]).arg(root);
    if write {
        command.arg("--write");
    }
    command.output().unwrap()
}

#[test]
fn wrapped_paragraphs_refuse_index_writes_without_rewriting_source() {
    let root = tempfile::tempdir().unwrap();
    indexed_fixture(root.path());
    let body = root.path().join("packages/engineering/body.md");
    let wrapped = "# Policy\n\nKeep this\nparagraph together.\n";
    std::fs::write(&body, wrapped).unwrap();
    for write in [false, true] {
        let output = check(root.path(), write);
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("packages/engineering/body.md:3"), "{error}");
        assert!(error.contains("one paragraph per source line"), "{error}");
        assert_eq!(std::fs::read_to_string(&body).unwrap(), wrapped);
        for family in ["engineering", "governance"] {
            assert_eq!(
                std::fs::read_to_string(root.path().join("packages").join(family).join("INDEX.md"))
                    .unwrap(),
                "stale index\n"
            );
        }
    }
    std::fs::write(&body, "# Policy\n\nKeep this paragraph together.\n").unwrap();
    let output = check(root.path(), true);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn broken_callers_block_index_writes_and_exclusions_are_explicit() {
    let root = tempfile::tempdir().unwrap();
    indexed_fixture(root.path());
    let callers = root.path().join("notes");
    std::fs::create_dir_all(callers.join("Records")).unwrap();
    std::fs::write(
        callers.join("Records/old.md"),
        "`example.engineering:old`\n",
    )
    .unwrap();
    let run = |exclude: bool| {
        let mut command = Command::new(bin());
        command
            .args(["packages", "check", "--root"])
            .arg(root.path())
            .args(["--write", "--caller-root"])
            .arg(&callers);
        if exclude {
            command.args(["--exclude-caller", "Records"]);
        }
        command.output().unwrap()
    };
    for bad in [
        "[missing](../packages/engineering/gone.md?token=SYNTHETIC_PRIVATE_MARKER)",
        "[missing](../packages/engineering/%ZZ.md)",
        "[missing](../packages/engineering/body.md#absent)",
        "`example.engineering:old`",
    ] {
        std::fs::write(callers.join("current.md"), bad).unwrap();
        let failed = run(true);
        assert!(!failed.status.success(), "{bad}");
        let error = String::from_utf8_lossy(&failed.stderr);
        assert!(error.contains("current.md:1"), "{error}");
        assert!(!error.contains("SYNTHETIC_PRIVATE_MARKER"), "{error}");
        for family in ["engineering", "governance"] {
            assert_eq!(
                std::fs::read_to_string(root.path().join("packages").join(family).join("INDEX.md"))
                    .unwrap(),
                "stale index\n"
            );
        }
    }
    std::fs::write(
        callers.join("current.md"),
        "[method](../packages/engineering/body.md#policy) `example.engineering:policy`\n",
    )
    .unwrap();
    assert!(!run(false).status.success());
    let passed = run(true);
    assert!(
        passed.status.success(),
        "{}",
        String::from_utf8_lossy(&passed.stderr)
    );
    assert!(String::from_utf8_lossy(&passed.stdout)
        .contains("2 incoming references in 1 caller documents"));
}

#[test]
fn incoming_index_fragments_are_checked_against_the_replacement() {
    let root = tempfile::tempdir().unwrap();
    indexed_fixture(root.path());
    let index = root.path().join("packages/engineering/INDEX.md");
    std::fs::write(&index, "# Removed heading\n").unwrap();
    let callers = root.path().join("notes");
    std::fs::create_dir(&callers).unwrap();
    let run = || {
        Command::new(bin())
            .args(["packages", "check", "--root"])
            .arg(root.path())
            .args(["--write", "--caller-root"])
            .arg(&callers)
            .output()
            .unwrap()
    };
    std::fs::write(
        callers.join("caller.md"),
        "[index](../packages/engineering/INDEX.md#removed-heading)\n",
    )
    .unwrap();
    assert!(!run().status.success());
    assert_eq!(
        std::fs::read_to_string(&index).unwrap(),
        "# Removed heading\n"
    );
    std::fs::write(
        callers.join("caller.md"),
        "[index](../packages/engineering/INDEX.md#fixture)\n",
    )
    .unwrap();
    let passed = run();
    assert!(
        passed.status.success(),
        "{}",
        String::from_utf8_lossy(&passed.stderr)
    );
}

#[test]
fn stale_indexes_require_explicit_write_and_preserve_authored_bodies() {
    let root = tempfile::tempdir().unwrap();
    indexed_fixture(root.path());
    let body = root.path().join("packages/engineering/body.md");
    let original = std::fs::read(&body).unwrap();
    let stale = check(root.path(), false);
    assert!(!stale.status.success());
    assert!(
        String::from_utf8_lossy(&stale.stderr).contains("indexes need regeneration"),
        "{}",
        String::from_utf8_lossy(&stale.stderr)
    );
    let written = check(root.path(), true);
    assert!(
        written.status.success(),
        "{}",
        String::from_utf8_lossy(&written.stderr)
    );
    assert_eq!(std::fs::read(body).unwrap(), original);
    assert!(check(root.path(), false).status.success());
    let index = root.path().join("packages/engineering/INDEX.md");
    let first = std::fs::read(&index).unwrap();
    assert!(check(root.path(), true).status.success());
    assert_eq!(std::fs::read(index).unwrap(), first);
}

#[test]
fn broken_source_link_blocks_all_index_writes() {
    let root = tempfile::tempdir().unwrap();
    indexed_fixture(root.path());
    for destination in [
        "#absent",
        "#absent:part",
        "body.md#absent:part",
        "./missing.md#absent:part",
    ] {
        std::fs::write(
            root.path().join("packages/engineering/body.md"),
            format!("# Scan\n\n[missing]({destination})\n"),
        )
        .unwrap();
        assert!(!check(root.path(), true).status.success(), "{destination}");
        for family in ["engineering", "governance"] {
            assert_eq!(
                std::fs::read_to_string(root.path().join("packages").join(family).join("INDEX.md"))
                    .unwrap(),
                "stale index\n",
                "{destination}"
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn symlinked_index_is_rejected_without_touching_its_target() {
    let root = tempfile::tempdir().unwrap();
    indexed_fixture(root.path());
    let outside = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(outside.path(), "outside\n").unwrap();
    let index = root.path().join("packages/engineering/INDEX.md");
    std::fs::remove_file(&index).unwrap();
    std::os::unix::fs::symlink(outside.path(), &index).unwrap();
    assert!(!check(root.path(), true).status.success());
    assert_eq!(
        std::fs::read_to_string(outside.path()).unwrap(),
        "outside\n"
    );
}

#[test]
fn package_check_and_show_work_without_workspace_flags() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let checked = Command::new(bin())
        .args(["packages", "check", "--root"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    let shown = Command::new(bin())
        .args(["packages", "show", "pale.design:inspect", "--root"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(
        shown.status.success(),
        "{}",
        String::from_utf8_lossy(&shown.stderr)
    );
    let text = String::from_utf8(shown.stdout).unwrap();
    assert!(text.contains("Read first:"));
    assert!(text.contains("access/web.md"));
    assert!(text.contains("Read as needed:"));
    assert!(text.contains("animate.md"));
    let missing = Command::new(bin())
        .args(["packages", "show", "pale.design:missing", "--root"])
        .arg(root)
        .output()
        .unwrap();
    assert!(!missing.status.success());
}

#[test]
fn package_catalog_json_carries_checked_source_relationships() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(bin())
        .args(["--result-format", "json", "packages", "show", "--root"])
        .arg(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["completion"], "complete");
    let resources = result["result"]["resources"].as_array().unwrap();
    let body = resources
        .iter()
        .find(|resource| resource["package"] == "pale.design" && resource["id"] == "inspect")
        .unwrap();
    assert!(body["read_first"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reference| reference["resource_id"] == "access/web"));
}
