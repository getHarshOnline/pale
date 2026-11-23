//! Terminal rendering of the SDK-owned read-only diagnostic report.

use std::process::ExitCode;

use crate::cli::{Cli, ResultFormatArg};
use crate::command::{CommandDispatcher, CommandOutcome, CommandRequest};
use crate::{terminal, DispatchFailure};

pub(super) fn run(
    cli: &Cli,
    build_roots: &[String],
    build_limit: Option<u64>,
    format: ResultFormatArg,
) -> Result<ExitCode, DispatchFailure> {
    let arguments = &cli.arguments;
    let workspace = if arguments.workspace.is_some()
        && arguments.project.is_some()
        && arguments.execution_workspace.is_some()
        && arguments.machine.is_some()
    {
        Some(crate::local_one_shot_request(cli)?)
    } else {
        None
    };
    let dispatcher =
        CommandDispatcher::new().map_err(|error| DispatchFailure::Host(error.into()))?;
    let outcome =
        dispatcher.dispatch(CommandRequest::LocalDoctor(pale_sdk::LocalDoctorRequest {
            workspace,
            build_roots: build_roots.to_vec(),
            build_limit,
        }))?;
    let CommandOutcome::Doctor(inspection) = outcome else {
        return Err(crate::diagnostic::internal_failure(
            "doctor returned an unrelated command result",
            anyhow::anyhow!("closed command result did not match its request"),
        )
        .into());
    };
    let value = terminal::parse_owner_json(inspection.canonical_bytes(), "doctor report")?;
    match format {
        ResultFormatArg::Json => terminal::write_json_result(
            terminal::ResultCompletion::Complete,
            &value,
            "doctor report",
        )?,
        ResultFormatArg::Human => {
            terminal::write_stdout(format_args!(
                "Pale diagnosis — observations only; no repairs performed."
            ))?;
            if let Some(checks) = value["checks"].as_array() {
                for check in checks {
                    terminal::write_stdout(format_args!(
                        "{} {} ({}): {}",
                        check["state"].as_str().unwrap_or("unavailable"),
                        check["code"].as_str().unwrap_or(""),
                        check["subject"].as_str().unwrap_or(""),
                        check["message"].as_str().unwrap_or("")
                    ))?;
                    if let Some(help) = check["help"].as_str().filter(|help| {
                        !help.is_empty()
                            && check["state"] != "passed"
                            && check["state"] != "not_selected"
                    }) {
                        terminal::write_stdout(format_args!("  Next: {help}"))?;
                    }
                }
            }
            if let Some(builds) = value["builds"].as_array() {
                for build in builds {
                    terminal::write_stdout(format_args!(
                        "Build {}: {} logical bytes; {} files; {} symlinks skipped; complete={}",
                        build["root"].as_str().unwrap_or(""),
                        build["logical_bytes"],
                        build["file_count"],
                        build["skipped_symlinks"],
                        build["complete"]
                    ))?;
                    if let Some(errors) = build["errors"].as_array() {
                        for error in errors {
                            terminal::write_stdout(format_args!(
                                "  {}",
                                error.as_str().unwrap_or("")
                            ))?;
                        }
                    }
                }
            }
        }
    }
    Ok(if inspection.checks_passed() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}
