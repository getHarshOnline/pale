//! Report compiled version and protocol claims without verifying installation.

use crate::cli::ResultFormatArg;
use crate::command::{CommandDispatcher, CommandOutcome, CommandRequest};
use crate::DispatchFailure;

/// Print the SDK's identity claim as readable JSON; do not inspect resident state.
pub(super) fn run(format: ResultFormatArg) -> Result<(), DispatchFailure> {
    let dispatcher =
        CommandDispatcher::new().map_err(|error| DispatchFailure::Host(error.into()))?;
    let outcome = dispatcher.dispatch(CommandRequest::CompiledInterfaceIdentity)?;
    let CommandOutcome::CompiledInterfaceIdentity(identity) = outcome else {
        return Err(crate::diagnostic::internal_failure(
            "compiled identity returned an unrelated command result",
            anyhow::anyhow!("closed command result did not match its request"),
        )
        .into());
    };
    match format {
        ResultFormatArg::Human => {
            let output = crate::terminal::render_pretty_json(
                identity.canonical_bytes(),
                "compiled interface identity",
            )?;
            crate::terminal::write_stdout(format_args!("{output}")).map_err(DispatchFailure::from)
        }
        ResultFormatArg::Json => {
            let result = crate::terminal::parse_owner_json(
                identity.canonical_bytes(),
                "compiled interface identity",
            )?;
            crate::terminal::write_json_result(
                crate::terminal::ResultCompletion::Complete,
                &result,
                "compiled interface identity",
            )
            .map_err(DispatchFailure::from)
        }
    }
}
