//! Real-process review decisions and refusal paths over isolated source roots.

use super::*;
use std::collections::BTreeMap;

fn review(
    baseline: &Path,
    candidate: &Path,
    action: &[&str],
    input: &[u8],
) -> std::process::Output {
    let mut child = Command::new(bin())
        .args([
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

pub(super) fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, path: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}

#[test]
fn real_cli_reviews_and_returns_each_decision_without_writing_configuration() {
    let root = tempfile::tempdir().unwrap();
    let baseline = write_workspace(&root.path().join("baseline"));
    let candidate = write_workspace(&root.path().join("candidate"));
    let before = files(root.path());
    let preview = review(&baseline, &candidate, &["preview"], &[]);
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    assert!(
        preview.stdout.starts_with(b"{\n"),
        "JSON is human-readable by default"
    );
    let summary: serde_json::Value = serde_json::from_slice(&preview.stdout).unwrap();
    for (action, disposition) in [
        ("accept", "accepted"),
        ("reject", "rejected"),
        ("cancel", "cancelled"),
        ("stale", "stale"),
    ] {
        let output = review(&baseline, &candidate, &[action], &preview.stdout);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let terminal: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(terminal["disposition"], disposition);
        assert_eq!(terminal["summary"], summary);
    }
    assert_eq!(
        files(root.path()),
        before,
        "review decisions never apply source changes"
    );
}

#[test]
fn real_cli_refuses_changed_or_malformed_review_summaries() {
    let root = tempfile::tempdir().unwrap();
    let baseline = write_workspace(&root.path().join("baseline"));
    let candidate = write_workspace(&root.path().join("candidate"));
    let preview = review(&baseline, &candidate, &["preview"], &[]);
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    for (input, code) in [
        (b"{}".to_vec(), "configuration_improvement_summary_mismatch"),
        (
            b"{\"private-marker\":1,\"private-marker\":2}".to_vec(),
            "configuration_improvement_summary_invalid_json",
        ),
        (
            vec![b' '; pale_sdk::ConfigurationImprovementPreviewSummary::MAX_PRESENTED_BYTES + 1],
            "configuration_improvement_summary_too_large",
        ),
    ] {
        let output = review(&baseline, &candidate, &["accept"], &input);
        assert_eq!(output.status.code(), Some(4));
        assert!(output.stdout.is_empty());
        let diagnostic = String::from_utf8(output.stderr).unwrap();
        assert!(diagnostic.contains(code), "{diagnostic}");
        assert!(!diagnostic.contains("private-marker"));
    }
    let machine = candidate
        .parent()
        .unwrap()
        .join("bindings/machines/synthetic.toml");
    let changed = std::fs::read_to_string(&machine)
        .unwrap()
        .replace("/tmp/syn/alpha", "/tmp/syn/changed-alpha");
    std::fs::write(machine, changed).unwrap();
    let before = files(root.path());
    let output = review(&baseline, &candidate, &["accept"], &preview.stdout);
    assert_eq!(output.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("configuration_improvement_summary_mismatch"));
    assert_eq!(files(root.path()), before);
}

#[test]
fn real_cli_impact_pages_match_the_review_owner_and_keep_the_summary_roundtrip() {
    let root = tempfile::tempdir().unwrap();
    let baseline = write_workspace(&root.path().join("baseline"));
    let candidate = write_workspace(&root.path().join("candidate"));
    let before = files(root.path());
    let api = pale_sdk::PaleApi::new().unwrap();
    let owner = api
        .preview_local_configuration_improvement(
            pale_sdk::LocalConfigurationImprovementRequest::new(
                pale_sdk::LocalWorkspaceLocator::new(baseline.to_str().unwrap().to_owned()),
                pale_sdk::LocalWorkspaceLocator::new(candidate.to_str().unwrap().to_owned()),
                "demo".to_owned(),
                "default".to_owned(),
                "synthetic".to_owned(),
            ),
        )
        .unwrap();
    let preview = review(&baseline, &candidate, &["preview"], &[]);
    assert!(preview.status.success());
    let summary: serde_json::Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(
        summary,
        serde_json::from_slice::<serde_json::Value>(owner.summary().canonical_bytes()).unwrap()
    );
    let first = owner.comparison().impacts(0, 1);
    assert_eq!(first.entries().len(), 1);
    let next = first
        .next_offset()
        .expect("fixture supports actual continuation");
    for (offset, limit) in [(0, 0), (0, 1), (next, 1), (0, u16::MAX), (u32::MAX, 1)] {
        let offset_text = offset.to_string();
        let limit_text = limit.to_string();
        let output = review(
            &baseline,
            &candidate,
            &["impacts", "--offset", &offset_text, "--limit", &limit_text],
            &preview.stdout,
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.starts_with(b"{\n"));
        let page: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            page,
            serde_json::from_slice::<serde_json::Value>(
                owner.comparison().impacts(offset, limit).canonical_bytes()
            )
            .unwrap()
        );
    }
    let implicit_default = review(&baseline, &candidate, &["impacts"], &preview.stdout);
    assert!(implicit_default.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&implicit_default.stdout).unwrap(),
        serde_json::from_slice::<serde_json::Value>(
            owner.comparison().impacts(0, 0).canonical_bytes()
        )
        .unwrap()
    );
    let terminal = review(&baseline, &candidate, &["accept"], &preview.stdout);
    assert!(terminal.status.success());
    let terminal: serde_json::Value = serde_json::from_slice(&terminal.stdout).unwrap();
    assert_eq!(terminal["summary"], summary);
    assert_eq!(terminal["disposition"], "accepted");
    for arguments in [
        ["impacts", "--limit", "65536"],
        ["impacts", "--offset", "4294967296"],
        ["impacts", "--offset", "-1"],
    ] {
        let invalid = review(&baseline, &candidate, &arguments, &[]);
        assert_eq!(invalid.status.code(), Some(2));
        assert!(invalid.stdout.is_empty());
    }
    assert_eq!(files(root.path()), before);
}

#[test]
fn real_cli_impact_continuation_rejects_changed_private_instance_and_invalid_summaries() {
    let root = tempfile::tempdir().unwrap();
    let baseline = write_workspace(&root.path().join("baseline"));
    let candidate = write_workspace(&root.path().join("candidate"));
    let preview = review(&baseline, &candidate, &["preview"], &[]);
    assert!(preview.status.success());
    for (input, code) in [
        (Vec::new(), "configuration_improvement_summary_invalid_json"),
        (b"{}".to_vec(), "configuration_improvement_summary_mismatch"),
        (
            b"{\"private-marker\":1,\"private-marker\":2}".to_vec(),
            "configuration_improvement_summary_invalid_json",
        ),
        (
            vec![b' '; pale_sdk::ConfigurationImprovementPreviewSummary::MAX_PRESENTED_BYTES + 1],
            "configuration_improvement_summary_too_large",
        ),
    ] {
        let output = review(&baseline, &candidate, &["impacts"], &input);
        assert_eq!(output.status.code(), Some(4));
        assert!(output.stdout.is_empty());
        let diagnostic = String::from_utf8(output.stderr).unwrap();
        assert!(diagnostic.contains(code), "{diagnostic}");
        assert!(!diagnostic.contains("private-marker"));
    }
    let first = review(
        &baseline,
        &candidate,
        &["impacts", "--limit", "1"],
        &preview.stdout,
    );
    assert!(first.status.success());
    let page: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    let next = page["nextOffset"]
        .as_u64()
        .expect("actual continuation")
        .to_string();
    let machine = candidate
        .parent()
        .unwrap()
        .join("bindings/machines/synthetic.toml");
    let changed = std::fs::read_to_string(&machine)
        .unwrap()
        .replace("/tmp/syn/alpha", "/tmp/syn/changed-alpha");
    std::fs::write(machine, changed).unwrap();
    let before = files(root.path());
    let output = review(
        &baseline,
        &candidate,
        &["impacts", "--offset", &next, "--limit", "1"],
        &preview.stdout,
    );
    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("configuration_improvement_summary_mismatch"));
    assert_eq!(files(root.path()), before);
}
