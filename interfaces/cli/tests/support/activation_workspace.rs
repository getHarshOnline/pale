//! Machine-bound workspace materialization shared by activation and mutation tests.

use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest as _, Sha256};

/// Per-machine values that cannot live in the static activation fixture.
pub(crate) struct MachineInputs<'a> {
    /// Target path bound to repository `alpha`.
    pub target: &'a Path,
    /// Read-only authority root bound as `auth_a`.
    pub auth: &'a Path,
    /// Exact executable selected for interface-identity verification.
    pub exec: &'a Path,
    /// SHA-256 of the selected executable bytes.
    pub digest: &'a str,
    /// Machine-local root for expiring session state.
    pub state_root: &'a Path,
    /// Machine-local root for durable materialization lineage.
    pub lineage_root: &'a Path,
}

/// Materialize the static activation workspace and bind it to one test machine.
///
/// # Panics
///
/// Panics when the fixture cannot be copied, rendered, permissioned, or bound to
/// the supplied test paths.
pub(crate) fn lay_out(workspace: &Path, machine: &MachineInputs<'_>) -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/activation");
    std::fs::create_dir_all(workspace).unwrap();
    let status = Command::new("cp")
        .arg("-R")
        .arg(format!("{}/.", source.display()))
        .arg(workspace)
        .status()
        .unwrap();
    assert!(
        status.success(),
        "copy activation fixture into {}",
        workspace.display()
    );

    let isolated_distribution = workspace.join("engine-distribution");
    super::synthetic_distribution::lay_out(&isolated_distribution);
    let lock = workspace.join("distribution/lock.toml");
    std::fs::create_dir_all(lock.parent().unwrap()).unwrap();
    std::fs::write(
        lock,
        super::synthetic_distribution::render_lock(&isolated_distribution),
    )
    .unwrap();
    let template_path = workspace.join("bindings/machines/m.toml.tmpl");
    let state_root = std::fs::canonicalize(machine.state_root).unwrap();
    let evidence_root = workspace.join("durable-evidence");
    std::fs::create_dir_all(&evidence_root).unwrap();
    let evidence_root = std::fs::canonicalize(evidence_root).unwrap();
    let lineage_root = std::fs::canonicalize(machine.lineage_root).unwrap();
    let harness_root = workspace.join("bindings/native-harnesses");
    std::fs::create_dir_all(&harness_root).unwrap();
    let claude_exec = harness_root.join("claude-code");
    let codex_exec = harness_root.join("codex");
    write_harness(&claude_exec, "Claude Code fixture 1.0.0");
    write_harness(&codex_exec, "codex-cli fixture 1.0.0");
    let claude_digest = hex::encode(Sha256::digest(std::fs::read(&claude_exec).unwrap()));
    let codex_digest = hex::encode(Sha256::digest(std::fs::read(&codex_exec).unwrap()));
    let rendered = std::fs::read_to_string(&template_path)
        .unwrap()
        .replace(
            "@DISTRIBUTION_ROOT@",
            &isolated_distribution.display().to_string(),
        )
        .replace("@TARGET@", &machine.target.display().to_string())
        .replace("@AUTH@", &machine.auth.display().to_string())
        .replace("@EXEC@", &machine.exec.display().to_string())
        .replace("@DIGEST@", machine.digest)
        .replace("@VERSION@", env!("CARGO_PKG_VERSION"))
        .replace("@CLAUDE_EXEC@", &claude_exec.display().to_string())
        .replace("@CLAUDE_DIGEST@", &claude_digest)
        .replace("@CODEX_EXEC@", &codex_exec.display().to_string())
        .replace("@CODEX_DIGEST@", &codex_digest)
        .replace("@STATE_ROOT@", &state_root.display().to_string())
        .replace("@EVIDENCE_ROOT@", &evidence_root.display().to_string())
        .replace("@LINEAGE_ROOT@", &lineage_root.display().to_string());
    std::fs::write(workspace.join("bindings/machines/m.toml"), rendered).unwrap();
    std::fs::remove_file(template_path).unwrap();
    workspace.join("pale-workspace.toml")
}

fn write_harness(path: &Path, version: &str) {
    std::fs::write(
        path,
        format!("#!/bin/sh\n[ \"$1\" = \"--version\" ] || exit 2\nprintf '%s\\n' '{version}'\n"),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}
