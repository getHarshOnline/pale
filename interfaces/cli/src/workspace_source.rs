//! Present the SDK's source-only inspection without selecting a runtime machine.

use std::process::ExitCode;

use crate::cli::{Cli, ResultFormatArg};
use crate::command::{CommandDispatcher, CommandOutcome, CommandRequest};
use crate::{terminal, DispatchFailure};

pub(super) fn run(cli: &Cli) -> Result<ExitCode, DispatchFailure> {
    let manifest = cli
        .arguments
        .workspace
        .as_deref()
        .and_then(|path| path.to_str())
        .ok_or_else(|| {
            crate::diagnostic::invalid_input(
                "source inspection requires a workspace manifest",
                anyhow::anyhow!("supply --workspace with an absolute UTF-8 manifest path"),
            )
        })?;
    let dispatcher =
        CommandDispatcher::new().map_err(|error| DispatchFailure::Host(error.into()))?;
    let outcome = dispatcher.dispatch(CommandRequest::LocalWorkspaceSource(
        pale_sdk::LocalWorkspaceLocator::new(manifest.to_owned()),
    ))?;
    let CommandOutcome::WorkspaceSource(inspection) = outcome else {
        return Err(crate::diagnostic::internal_failure(
            "source inspection returned an unrelated command result",
            anyhow::anyhow!("closed command result did not match its request"),
        )
        .into());
    };
    let value = terminal::parse_owner_json(inspection.canonical_bytes(), "workspace source")?;
    match cli.arguments.result_format {
        ResultFormatArg::Json => terminal::write_json_result(
            terminal::ResultCompletion::Complete, &value, "workspace source",
        )?,
        ResultFormatArg::Human => terminal::write_stdout(format_args!(
            "Local source checked: {} modules; content {}. Publication and runtime admission remain separate checks.",
            value["modules"].as_array().map_or(0, Vec::len), inspection.content_digest(),
        ))?,
    }
    Ok(ExitCode::SUCCESS)
}
