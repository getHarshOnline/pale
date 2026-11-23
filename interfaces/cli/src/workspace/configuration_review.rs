//! Present local configuration reviews through the shared command dispatcher.

use std::io::Read;

use crate::cli::{ConfigurationReviewAction, ResultFormatArg};
use crate::command::{
    CommandDispatcher, CommandOutcome, CommandRequest, ConfigurationImprovementCommand,
    ConfigurationImprovementDisposition, ConfigurationImprovementOutcome,
    ConfigurationImprovementPreviewSummary, LocalConfigurationImprovementRequest,
};
use crate::DispatchFailure;
use anyhow::Context;

/// Keep stream I/O and presentation outside the SDK's capture and review owners.
pub(crate) fn cmd_review_configuration(
    request: LocalConfigurationImprovementRequest,
    action: ConfigurationReviewAction,
    format: ResultFormatArg,
) -> Result<(), DispatchFailure> {
    let dispatcher =
        CommandDispatcher::new().map_err(|error| DispatchFailure::Host(error.into()))?;
    let decision = match action {
        ConfigurationReviewAction::Preview | ConfigurationReviewAction::Impacts { .. } => None,
        ConfigurationReviewAction::Accept => Some(ConfigurationImprovementDisposition::Accepted),
        ConfigurationReviewAction::Reject => Some(ConfigurationImprovementDisposition::Rejected),
        ConfigurationReviewAction::Cancel => Some(ConfigurationImprovementDisposition::Cancelled),
        ConfigurationReviewAction::Stale => Some(ConfigurationImprovementDisposition::Stale),
    };
    let presented_summary = match action {
        ConfigurationReviewAction::Preview => Vec::new(),
        _ => read_summary()?,
    };
    let CommandOutcome::ConfigurationImprovementSources(sources) = dispatcher.dispatch(
        CommandRequest::LocalConfigurationImprovementCapture(request),
    )?
    else {
        return Err(unexpected_outcome());
    };
    let command = match (action, decision) {
        (ConfigurationReviewAction::Impacts { offset, limit }, _) => {
            ConfigurationImprovementCommand::ImpactsSources {
                sources,
                presented_summary,
                offset,
                limit,
            }
        }
        (_, Some(disposition)) => ConfigurationImprovementCommand::FinishSources {
            sources,
            presented_summary,
            disposition,
        },
        (_, None) => ConfigurationImprovementCommand::PreviewSources(sources),
    };
    let CommandOutcome::ConfigurationImprovement(outcome) =
        dispatcher.dispatch(CommandRequest::ConfigurationImprovement(command))?
    else {
        return Err(unexpected_outcome());
    };
    let (completion, bytes) = match outcome {
        ConfigurationImprovementOutcome::SourcePreviewed { preview, .. } => (
            crate::terminal::ResultCompletion::NonTerminal,
            preview.summary().canonical_bytes().to_vec(),
        ),
        ConfigurationImprovementOutcome::Impacts(page) => (
            if page.next_offset().is_some() {
                crate::terminal::ResultCompletion::Partial
            } else {
                crate::terminal::ResultCompletion::Complete
            },
            page.canonical_bytes().to_vec(),
        ),
        ConfigurationImprovementOutcome::Finished(terminal) => {
            let bytes =
                serde_json::to_vec(&terminal).context("render configuration-review decision")?;
            (crate::terminal::ResultCompletion::Complete, bytes)
        }
        _ => return Err(unexpected_outcome()),
    };
    match format {
        ResultFormatArg::Human => {
            let output = crate::terminal::render_pretty_json(&bytes, "configuration review")?;
            crate::terminal::write_stdout(format_args!("{output}")).map_err(Into::into)
        }
        ResultFormatArg::Json => {
            let result = crate::terminal::parse_owner_json(&bytes, "configuration review")?;
            crate::terminal::write_json_result(completion, &result, "configuration review")
                .map_err(Into::into)
        }
    }
}

/// Retain at most the SDK input limit plus one byte for its overflow diagnostic.
fn read_summary() -> Result<Vec<u8>, DispatchFailure> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .lock()
        .take(ConfigurationImprovementPreviewSummary::MAX_PRESENTED_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .context("read the presented review summary from stdin")?;
    Ok(bytes)
}

fn unexpected_outcome() -> DispatchFailure {
    crate::diagnostic::internal_failure(
        "configuration review returned an incompatible command outcome",
        anyhow::anyhow!("the review command did not return its declared result"),
    )
    .into()
}
