//! Save one plan per repository without applying target changes.

use pale_sdk::{
    DiagnosticEffectCertainty, LocalOneShotPlanCounts, LocalOneShotPlanPublication, OneShotPlanKind,
};

use crate::cli::ResultFormatArg;
use crate::command::{
    CommandDispatcher, CommandOutcome, CommandRequest, LocalMaterializationCommand,
    LocalMaterializationOutcome, LocalSavedPlanPublication, LocalSavedPlanPublicationDisposition,
    LocalSavedPlanPublicationRequest,
};
use crate::terminal::ResultCompletion;
use crate::DispatchFailure;

/// Save one exact plan per repository without applying target changes.
pub(crate) fn cmd_plan(
    dispatcher: &CommandDispatcher,
    compilation: Box<pale_sdk::LocalOneShotCompilation>,
    output_directory: String,
    kind: OneShotPlanKind,
    result_format: ResultFormatArg,
) -> Result<(), DispatchFailure> {
    let outcome = dispatcher.dispatch(CommandRequest::LocalMaterialization(
        LocalMaterializationCommand::PublishPlans(LocalSavedPlanPublicationRequest::new(
            compilation,
            kind,
            output_directory,
        )),
    ))?;
    let CommandOutcome::LocalMaterialization(LocalMaterializationOutcome::PlanPublication(
        disposition,
    )) = outcome
    else {
        return Err(crate::diagnostic::internal_failure(
            "saved-plan publication returned an unrelated command result",
            anyhow::anyhow!("closed command result did not match its request"),
        )
        .into());
    };

    let output_kind = match kind {
        OneShotPlanKind::Apply | OneShotPlanKind::Adopt => "apply",
        OneShotPlanKind::Remove => "remove",
    };
    match disposition {
        LocalSavedPlanPublicationDisposition::Complete {
            compilation: _,
            publications,
        } => {
            report_publications(&publications, output_kind, result_format);
            if result_format == ResultFormatArg::Json {
                let result = publication_result(kind, &publications, None, None);
                crate::terminal::report_json_result(
                    ResultCompletion::Complete,
                    &result,
                    "saved-plan publication result",
                );
            }
            Ok(())
        }
        LocalSavedPlanPublicationDisposition::Stopped {
            compilation: _,
            publications,
            stopped_repository_id,
            error,
        } => {
            report_publications(&publications, output_kind, result_format);
            if result_format == ResultFormatArg::Json {
                let diagnostic = crate::terminal::parse_owner_json(
                    error.diagnostic().canonical_bytes(),
                    "saved-plan publication diagnostic",
                )?;
                let result = publication_result(
                    kind,
                    &publications,
                    Some(&stopped_repository_id),
                    Some(&diagnostic),
                );
                crate::terminal::report_json_result(
                    stopped_completion(error.diagnostic().effect()),
                    &result,
                    "saved-plan publication result",
                );
            }
            Err(error.into())
        }
    }
}

/// Preserve the established line-oriented report for human output.
fn report_publications(
    publications: &[LocalSavedPlanPublication],
    output_kind: &str,
    result_format: ResultFormatArg,
) {
    if result_format == ResultFormatArg::Json {
        return;
    }
    for completed in publications {
        let publication = completed.publication();
        crate::terminal::report_stdout(format_args!(
            "plan {} [{}]: {} operations -> {}",
            publication.repository_id(),
            output_kind,
            completed.counts().total_operations(),
            publication.display_target()
        ));
    }
}

/// Classify a stopped batch without weakening the SDK's uncertainty.
const fn stopped_completion(effect: DiagnosticEffectCertainty) -> ResultCompletion {
    match effect {
        DiagnosticEffectCertainty::OutcomeUnknown => ResultCompletion::NonTerminal,
        DiagnosticEffectCertainty::NotApplicable
        | DiagnosticEffectCertainty::NotStarted
        | DiagnosticEffectCertainty::Confirmed => ResultCompletion::Partial,
    }
}

/// CLI transport projection for one ordered plan-publication batch.
#[derive(serde::Serialize)]
struct PlanPublicationResult<'a> {
    plan_kind: &'static str,
    publications: Vec<PlanPublicationResultEntry<'a>>,
    stopped_repository_id: Option<&'a str>,
    diagnostic: Option<&'a serde_json::Value>,
}

/// One completed publication inside the CLI batch result.
#[derive(serde::Serialize)]
struct PlanPublicationResultEntry<'a> {
    repository_id: &'a str,
    relative_target: &'a str,
    display_target: &'a str,
    counts: PlanPublicationCounts,
}

/// Operation counts projected from one completed SDK publication.
#[derive(serde::Serialize)]
struct PlanPublicationCounts {
    baseline: u32,
    operations: u32,
    total_operations: u32,
    create: u32,
    adopt: u32,
    update: u32,
    unchanged: u32,
    delete: u32,
    splice: u32,
    deferred_effects: u32,
    machine_local_effects: u32,
}

/// Build one typed CLI result from exact publication facts and diagnostics.
fn publication_result<'a>(
    kind: OneShotPlanKind,
    publications: &'a [LocalSavedPlanPublication],
    stopped_repository_id: Option<&'a str>,
    diagnostic: Option<&'a serde_json::Value>,
) -> PlanPublicationResult<'a> {
    let publications = publications
        .iter()
        .map(|completed| publication_value(completed.counts(), completed.publication()))
        .collect::<Vec<_>>();
    PlanPublicationResult {
        plan_kind: plan_kind(kind),
        publications,
        stopped_repository_id,
        diagnostic,
    }
}

/// Project one completed publication without reading the saved plan again.
fn publication_value<'a>(
    counts: LocalOneShotPlanCounts,
    publication: &'a LocalOneShotPlanPublication,
) -> PlanPublicationResultEntry<'a> {
    PlanPublicationResultEntry {
        repository_id: publication.repository_id(),
        relative_target: publication.relative_target(),
        display_target: publication.display_target(),
        counts: PlanPublicationCounts {
            baseline: counts.baseline(),
            operations: counts.operations(),
            total_operations: counts.total_operations(),
            create: counts.create(),
            adopt: counts.adopt(),
            update: counts.update(),
            unchanged: counts.unchanged(),
            delete: counts.delete(),
            splice: counts.splice(),
            deferred_effects: counts.deferred_effects(),
            machine_local_effects: counts.machine_local_effects(),
        },
    }
}

/// Stable CLI spelling for the SDK-owned plan class.
const fn plan_kind(kind: OneShotPlanKind) -> &'static str {
    match kind {
        OneShotPlanKind::Apply => "apply",
        OneShotPlanKind::Adopt => "adopt",
        OneShotPlanKind::Remove => "remove",
    }
}

#[cfg(test)]
#[path = "../../tests/internal/workspace/plan_publication.rs"]
mod tests;
