//! Process-level tests of the `pale` command interface.
//!
//! The real binary parses arguments, executes actions, reviews configuration,
//! inspects control storage, and materializes surfaces in isolated test roots.
//! Assertions cover public output, diagnostics, exit status, and resulting files.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// The activation workspace fixture is data under `tests/fixtures/activation`, materialized through
// the shared loader rather than hardcoded here. The bench harness uses the same loader (SSOT).
#[path = "support/activation_fixture.rs"]
pub mod activation_fixture;

#[path = "support/current_workspace.rs"]
mod current_workspace;

#[cfg(unix)]
#[path = "support/test_executable.rs"]
mod test_executable;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_pale")
}

/// Lay out a minimal valid synthetic workspace for the canonical entry: a root
/// manifest, one project `demo`, one execution workspace `default`, one machine
/// `synthetic` whose `distribution_root` points at an isolated two-Package test
/// distribution, and its exact derived lock. No consuming-project identity
/// appears anywhere. Returns the manifest path.
fn write_workspace(ws: &Path) -> PathBuf {
    let distribution =
        activation_fixture::synthetic_distribution::lay_out(&ws.join("engine-distribution"));
    let lock = activation_fixture::synthetic_distribution::render_lock(&distribution);
    current_workspace::lay_out(ws, &distribution, &lock)
}

fn run(
    manifest: &Path,
    project: &str,
    execution_workspace: &str,
    machine: &str,
) -> std::process::Output {
    Command::new(bin())
        .args([
            "--workspace",
            manifest.to_str().unwrap(),
            "--project",
            project,
            "--execution-workspace",
            execution_workspace,
            "--machine",
            machine,
            "validate",
        ])
        .output()
        .unwrap()
}

// ---- real activation apply + sealed hook roundtrip ----------

#[path = "cli/action.rs"]
mod action;
#[path = "cli/authoring.rs"]
mod authoring;
#[path = "cli/configuration_review.rs"]
mod configuration_review;
#[path = "cli/entry.rs"]
mod entry;
#[cfg(unix)]
#[path = "cli/hook.rs"]
mod hook;
#[path = "cli/packages.rs"]
mod packages;
#[path = "cli/result_format.rs"]
mod result_format;
#[path = "cli/source_guidance.rs"]
mod source_guidance;

#[cfg(unix)]
#[path = "cli/control_storage.rs"]
mod control_storage;

#[cfg(unix)]
#[path = "cli/materialization.rs"]
mod materialization;

#[path = "cli/doctor.rs"]
mod doctor;

#[path = "cli/workspace_source.rs"]
mod workspace_source;
