//! Reuse the compiler's binding checks while retaining independent failures.

use std::path::Path;

use pale_ports::{MachineFilesystemPort, MachinePathKind, ProcessProbePort};

use super::{Check, CheckState};
use crate::{compile::Compilation, workspace};

/// Inspect machine paths, selected harnesses and the bound Pale executable.
///
/// Each independent binding is checked even after another fails. These probes
/// establish path and executable facts, not resident recovery or hook delivery.
pub fn inspect_machine(
    compilation: &Compilation,
    filesystem: &dyn MachineFilesystemPort,
    processes: &dyn ProcessProbePort,
) -> Vec<Check> {
    let instance = compilation.instance();
    let mut checks = Vec::new();
    for (code, id, path, accepts_file) in instance
        .repositories
        .iter()
        .map(|(id, path)| ("repository_root", id.as_str(), path, false))
        .chain(
            instance
                .authority_roots
                .iter()
                .map(|(id, path)| ("authority_root", id.as_str(), path, true)),
        )
    {
        let observed = filesystem.observe(Path::new(path));
        let (state, message) = match observed {
            Ok(value) if value.canonical_kind() == MachinePathKind::Directory => (
                CheckState::Passed,
                "Declared directory is accessible.".to_owned(),
            ),
            Ok(value) if accepts_file && value.canonical_kind() == MachinePathKind::RegularFile => {
                (
                    CheckState::Passed,
                    "Declared exact authority file is accessible.".to_owned(),
                )
            }
            Ok(_) => (
                CheckState::Failed,
                if accepts_file {
                    "Declared authority root is neither a directory nor an exact regular file."
                } else {
                    "Declared repository root is not a directory."
                }
                .to_owned(),
            ),
            Err(error) => (CheckState::Failed, error.to_string()),
        };
        checks.push(Check::new(code, id, state, message,
            "Correct this root in the selected machine binding or restore the declared directory or authority file."));
    }
    if let Some(session) = &compilation.profile().session {
        checks.push(result("reading_order", &instance.machine_id,
            workspace::verify_reading_order(filesystem, instance, session),
            "Restore each declared reading-order file within its authority root; an exact-file binding cannot authorize sibling files."));
    }
    if instance.surface_executable.is_some() {
        checks.push(result("pale_executable", &instance.machine_id,
            workspace::verify_instance_executable_binding(processes, instance).map(|_| ()),
            "Bind the intended Pale executable with its exact digest, version and interface protocol."));
    } else {
        checks.push(Check::new(
            "pale_executable",
            &instance.machine_id,
            CheckState::Unavailable,
            "No Pale executable is declared in this compilation-only binding.",
            "Select an executable binding before checking an installed deployment.",
        ));
    }
    if instance.native_harnesses.is_empty() {
        let selected_paths = !compilation.profile().execution_paths.is_empty();
        checks.push(Check::new("agent_executables", &instance.machine_id,
            if selected_paths { CheckState::Unavailable } else { CheckState::NotSelected },
            if selected_paths { "Execution paths are selected, but this compilation-only instance declares no native agent executable." }
            else { "No execution path or native agent harness is selected." },
            if selected_paths { "Bind the selected native harness identities before qualifying installed agents." } else { "" }));
    }
    for harness in &instance.native_harnesses {
        checks.push(result("agent_executable", &harness.provider_binding.provider_binding_id,
            workspace::verify_native_harness(processes, harness),
            "Correct the selected native harness binding; a version string alone does not prove host behavior."));
    }
    for (code, outcome) in [
        (
            "session_state",
            instance
                .session_state
                .as_ref()
                .map(|_| workspace::verify_session_state_root(filesystem, instance)),
        ),
        (
            "durable_evidence",
            instance
                .durable_evidence
                .as_ref()
                .map(|_| workspace::verify_durable_evidence_root(filesystem, instance)),
        ),
        (
            "materialization_lineage",
            instance
                .materialization_lineage
                .as_ref()
                .map(|_| workspace::verify_materialization_lineage_root(filesystem, instance)),
        ),
    ] {
        if let Some(outcome) = outcome {
            checks.push(result(code, &instance.machine_id, outcome.map(|_| ()),
                "Restore or correct the declared state root and its permissions through its owner; doctor creates no state."));
        } else {
            checks.push(Check::new(
                code,
                &instance.machine_id,
                CheckState::NotSelected,
                "This state root is not selected by the compiled instance.",
                "",
            ));
        }
    }
    checks
}

fn result(
    code: &str,
    subject: &str,
    outcome: Result<(), workspace::WorkspaceError>,
    help: &str,
) -> Check {
    match outcome {
        Ok(()) => Check::new(
            code,
            subject,
            CheckState::Passed,
            "Declared binding verified.",
            "",
        ),
        Err(error) => Check::new(code, subject, CheckState::Failed, error.to_string(), help),
    }
}
