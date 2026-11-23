//! Terminal projection for resident commands supplied by a trusted embedder.

use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;

use anyhow::Context;

use crate::cli::{
    ResidentCommand, ResidentConsultCommand, ResidentDelegateCommand, ResidentInvokeCommand,
    ResidentLocatedRequestArgs, ResultFormatArg,
};
use crate::command::{
    CommandDispatcher, CommandOutcome, CommandRequest, OperationHandleLocator,
    ResidentConsultAdvanceOutcome, ResidentConsultAdvanceRequest, ResidentConsultStartRequest,
    ResidentDelegateAdvanceOutcome, ResidentDelegateAdvanceRequest, ResidentDelegateStartRequest,
    ResidentInvokeAdvanceOutcome, ResidentInvokeAdvanceRequest, ResidentInvokeStartRequest,
    ResidentOperationCancellationDisposition, ResidentOperationCancellationRequest,
    ResidentOperationObservationRequest,
};
use crate::terminal::ResultCompletion;
use crate::DispatchFailure;

/// Whether output still participates in the command result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputObligation {
    /// A read-only observation is incomplete if its output cannot be written.
    Required,
    /// A durable resident transition cannot be reclassified by terminal I/O.
    EffectEstablished,
}

/// Execute one resident command through the supplied typed dispatcher.
pub(super) fn run(
    dispatcher: &CommandDispatcher,
    command: &ResidentCommand,
    format: ResultFormatArg,
) -> Result<(), DispatchFailure> {
    let stdout = std::io::stdout();
    run_with_writer(dispatcher, command, format, &mut stdout.lock())
}

/// Execute and project one resident command to an explicit writer.
fn run_with_writer(
    dispatcher: &CommandDispatcher,
    command: &ResidentCommand,
    format: ResultFormatArg,
    writer: &mut impl Write,
) -> Result<(), DispatchFailure> {
    let request = command_request(command)?;
    let outcome = dispatcher.dispatch(request)?;
    let (result, completion, subject, output_obligation) = project_outcome(outcome)?;

    let output = match format {
        ResultFormatArg::Human => serde_json::to_string_pretty(&result).map_err(|error| {
            crate::diagnostic::internal_failure(
                format!("the {subject} could not be rendered"),
                error.into(),
            )
        })?,
        ResultFormatArg::Json => crate::terminal::render_json_result(completion, &result, subject)?,
    };
    match output_obligation {
        OutputObligation::Required => crate::terminal::write_line(writer, format_args!("{output}"))
            .context("write command output")?,
        OutputObligation::EffectEstablished => {
            crate::terminal::report_line(writer, format_args!("{output}"));
        }
    }
    Ok(())
}

/// Convert the parsed leaf syntax into one existing dispatcher request.
fn command_request(command: &ResidentCommand) -> Result<CommandRequest, DispatchFailure> {
    match command {
        ResidentCommand::Observe(arguments) => Ok(CommandRequest::ResidentOperationObservation(
            ResidentOperationObservationRequest::new(
                locator(arguments),
                read_signed_request(&arguments.signed_request)?,
            ),
        )),
        ResidentCommand::Invoke { step } => match step {
            ResidentInvokeCommand::Start(arguments) => Ok(CommandRequest::ResidentInvokeStart(
                ResidentInvokeStartRequest::new(read_signed_request(&arguments.signed_request)?),
            )),
            ResidentInvokeCommand::Advance(arguments) => Ok(CommandRequest::ResidentInvokeAdvance(
                ResidentInvokeAdvanceRequest::new(
                    locator(arguments),
                    read_signed_request(&arguments.signed_request)?,
                ),
            )),
        },
        ResidentCommand::Consult { step } => match step {
            ResidentConsultCommand::Start(arguments) => Ok(CommandRequest::ResidentConsultStart(
                ResidentConsultStartRequest::new(read_signed_request(&arguments.signed_request)?),
            )),
            ResidentConsultCommand::Advance(arguments) => Ok(
                CommandRequest::ResidentConsultAdvance(ResidentConsultAdvanceRequest::new(
                    locator(arguments),
                    read_signed_request(&arguments.signed_request)?,
                )),
            ),
        },
        ResidentCommand::Delegate { step } => match step {
            ResidentDelegateCommand::Start(arguments) => Ok(CommandRequest::ResidentDelegateStart(
                ResidentDelegateStartRequest::new(
                    OperationHandleLocator::new(
                        arguments.parent_operation_handle_record_ref.clone(),
                        arguments.parent_operation_handle_sha256.clone(),
                    ),
                    read_signed_request(&arguments.signed_request)?,
                ),
            )),
            ResidentDelegateCommand::Advance(arguments) => Ok(
                CommandRequest::ResidentDelegateAdvance(ResidentDelegateAdvanceRequest::new(
                    locator(arguments),
                    read_signed_request(&arguments.signed_request)?,
                )),
            ),
        },
        ResidentCommand::Cancel(arguments) => Ok(CommandRequest::ResidentOperationCancellation(
            ResidentOperationCancellationRequest::new(
                locator(arguments),
                read_signed_request(&arguments.signed_request)?,
            ),
        )),
    }
}

fn locator(arguments: &ResidentLocatedRequestArgs) -> OperationHandleLocator {
    OperationHandleLocator::new(
        arguments.operation_handle_record_ref.clone(),
        arguments.operation_handle_sha256.clone(),
    )
}

/// Read one exact request without accepting unbounded host input.
fn read_signed_request(path: &Path) -> Result<Vec<u8>, DispatchFailure> {
    let file = File::open(path).with_context(|| {
        format!(
            "open pre-signed resident request `{}`",
            path.to_string_lossy()
        )
    })?;
    let mut bytes = Vec::new();
    file.take((pale_sdk::RESIDENT_RECORD_BYTE_LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .with_context(|| {
            format!(
                "read pre-signed resident request `{}`",
                path.to_string_lossy()
            )
        })?;
    if bytes.len() > pale_sdk::RESIDENT_RECORD_BYTE_LIMIT {
        return Err(crate::diagnostic::invalid_input(
            "pre-signed resident request exceeds the CLI input limit",
            anyhow::anyhow!(
                "--signed-request accepts at most {} bytes",
                pale_sdk::RESIDENT_RECORD_BYTE_LIMIT
            ),
        )
        .into());
    }
    Ok(bytes)
}

/// Preserve the exact SDK result while adding only CLI completion metadata.
fn project_outcome(
    outcome: CommandOutcome,
) -> Result<
    (
        serde_json::Value,
        ResultCompletion,
        &'static str,
        OutputObligation,
    ),
    DispatchFailure,
> {
    let (result, completion, subject, output_obligation) = match outcome {
        CommandOutcome::ResidentOperationObservation(value) => (
            serde_json::to_value(value),
            ResultCompletion::Complete,
            "resident Observe result",
            OutputObligation::Required,
        ),
        CommandOutcome::ResidentInvokeStarted(value) => (
            serde_json::to_value(value),
            ResultCompletion::NonTerminal,
            "resident Invoke start",
            OutputObligation::EffectEstablished,
        ),
        CommandOutcome::ResidentConsultStarted(value) => (
            serde_json::to_value(value),
            ResultCompletion::NonTerminal,
            "resident Consult start",
            OutputObligation::EffectEstablished,
        ),
        CommandOutcome::ResidentDelegateStarted(value) => (
            serde_json::to_value(value),
            ResultCompletion::NonTerminal,
            "resident Delegate start",
            OutputObligation::EffectEstablished,
        ),
        CommandOutcome::ResidentInvokeAdvanced(value) => {
            let completion = match value {
                ResidentInvokeAdvanceOutcome::Observed { .. } => ResultCompletion::Complete,
                ResidentInvokeAdvanceOutcome::Pending { .. } => ResultCompletion::NonTerminal,
            };
            (
                serde_json::to_value(value),
                completion,
                "resident Invoke result",
                OutputObligation::EffectEstablished,
            )
        }
        CommandOutcome::ResidentConsultAdvanced(value) => {
            let completion = match value {
                ResidentConsultAdvanceOutcome::Observed { .. } => ResultCompletion::Complete,
                ResidentConsultAdvanceOutcome::Pending { .. } => ResultCompletion::NonTerminal,
            };
            (
                serde_json::to_value(value),
                completion,
                "resident Consult result",
                OutputObligation::EffectEstablished,
            )
        }
        CommandOutcome::ResidentDelegateAdvanced(value) => {
            let completion = match value {
                ResidentDelegateAdvanceOutcome::Observed { .. }
                | ResidentDelegateAdvanceOutcome::Unsupported { .. } => ResultCompletion::Complete,
                ResidentDelegateAdvanceOutcome::Pending { .. } => ResultCompletion::NonTerminal,
            };
            (
                serde_json::to_value(value),
                completion,
                "resident Delegate result",
                OutputObligation::EffectEstablished,
            )
        }
        CommandOutcome::ResidentOperationCancellation(value) => {
            let completion = match value.disposition() {
                ResidentOperationCancellationDisposition::TerminalAlreadyEstablished
                | ResidentOperationCancellationDisposition::LocallyCancelled
                | ResidentOperationCancellationDisposition::ConsultationCollected => {
                    ResultCompletion::Complete
                }
                ResidentOperationCancellationDisposition::AlreadyRequested
                | ResidentOperationCancellationDisposition::RequestRetained
                | ResidentOperationCancellationDisposition::DispatchedRequestRetained
                | ResidentOperationCancellationDisposition::A2aResponseReadStopped
                | ResidentOperationCancellationDisposition::ReconciliationOnly
                | ResidentOperationCancellationDisposition::DispatchOwnerRequired
                | ResidentOperationCancellationDisposition::ConsultationStopped
                | ResidentOperationCancellationDisposition::NotCollecting => {
                    ResultCompletion::NonTerminal
                }
            };
            (
                serde_json::to_value(value),
                completion,
                "resident Cancel result",
                OutputObligation::EffectEstablished,
            )
        }
        _ => {
            return Err(crate::diagnostic::internal_failure(
                "resident command returned an unrelated command result",
                anyhow::anyhow!("closed command result did not match its request"),
            )
            .into())
        }
    };
    result
        .map(|result| (result, completion, subject, output_obligation))
        .map_err(|error| {
            crate::diagnostic::internal_failure(
                format!("the {subject} could not be projected"),
                error.into(),
            )
            .into()
        })
}

#[cfg(test)]
#[path = "../tests/internal/resident.rs"]
mod tests;
