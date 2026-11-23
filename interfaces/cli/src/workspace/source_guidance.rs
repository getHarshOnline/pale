//! Thin source-guidance request mapping and SDK-owned result presentation.

use anyhow::Result;
use pale_sdk::{
    LocalSourceGuidanceChange, LocalSourceGuidancePlanReport, SourceGuidanceProjection,
};

use crate::{
    cli::{GuidanceCommand, GuidanceRecoveryCommand, ResultFormatArg},
    command::{CommandDispatcher, CommandOutcome, CommandRequest},
    terminal::{write_json_result, write_stdout, ResultCompletion},
    DispatchFailure,
};

mod diff;

/// The leaf selects an explicit operation and renders; the SDK owns all effects.
pub(crate) fn cmd_guidance(
    selection: pale_sdk::LocalOneShotCompilationRequest,
    action: &GuidanceCommand,
    format: ResultFormatArg,
) -> Result<(), DispatchFailure> {
    let dispatcher =
        CommandDispatcher::new().map_err(|error| DispatchFailure::Host(error.into()))?;
    let request = match action {
        GuidanceCommand::Context { repository } => CommandRequest::LocalRepositoryContext {
            selection,
            repository_id: repository.clone(),
        },
        GuidanceCommand::Inspect { repository, host } => CommandRequest::LocalSourceGuidance {
            selection,
            repository_id: repository.clone(),
            host: (*host).into(),
        },
        GuidanceCommand::Plan {
            repository,
            host,
            intent,
        } => {
            CommandRequest::LocalSourceGuidancePlan(pale_sdk::LocalSourceGuidancePlanRequest::new(
                selection,
                repository.clone(),
                (*host).into(),
                (*intent).into(),
            ))
        }
        GuidanceCommand::Apply {
            repository,
            host,
            intent,
            expect_plan,
        } => {
            let plan = pale_sdk::LocalSourceGuidancePlanRequest::new(
                selection,
                repository.clone(),
                (*host).into(),
                (*intent).into(),
            );
            CommandRequest::LocalSourceGuidanceWrite(match expect_plan {
                Some(digest) => {
                    pale_sdk::LocalSourceGuidanceWriteRequest::new(plan, digest.clone())
                }
                None => pale_sdk::LocalSourceGuidanceWriteRequest::remembered(plan),
            })
        }
        GuidanceCommand::Recovery { action } => match action {
            GuidanceRecoveryCommand::Plan {
                repository,
                host,
                intent,
            } => CommandRequest::LocalSourceGuidanceRecoveryPlan(
                pale_sdk::LocalSourceGuidanceRecoveryRequest::new(
                    selection,
                    repository.clone(),
                    (*host).into(),
                    (*intent).into(),
                ),
            ),
            GuidanceRecoveryCommand::Apply {
                repository,
                host,
                intent,
                expect_recovery,
            } => CommandRequest::LocalSourceGuidanceRecoveryExecute(
                pale_sdk::LocalSourceGuidanceRecoveryExecutionRequest::new(
                    pale_sdk::LocalSourceGuidanceRecoveryRequest::new(
                        selection,
                        repository.clone(),
                        (*host).into(),
                        (*intent).into(),
                    ),
                    expect_recovery.clone(),
                ),
            ),
        },
    };
    let outcome = dispatcher.dispatch(request)?;
    match (action, outcome) {
        (GuidanceCommand::Context { .. }, CommandOutcome::RepositoryContext(value)) => {
            if format == ResultFormatArg::Json {
                write_json_result(
                    ResultCompletion::Complete,
                    &value,
                    "private repository context",
                )?;
            } else {
                write_stdout(format_args!(
                    "{}",
                    value.strip_suffix('\n').unwrap_or(&value)
                ))?;
            }
        }
        (GuidanceCommand::Inspect { .. }, CommandOutcome::SourceGuidance(value)) => {
            cmd_source_guidance(&value, format)?
        }
        (GuidanceCommand::Plan { .. }, CommandOutcome::SourceGuidancePlan(value)) => {
            cmd_source_guidance_plan(value.report(), format)?;
            dispatcher.dispatch(CommandRequest::LocalSourceGuidanceRemember(value))?;
        }
        (GuidanceCommand::Apply { .. }, CommandOutcome::SourceGuidanceWrite(value)) => {
            if format == ResultFormatArg::Json {
                write_json_result(
                    ResultCompletion::Complete,
                    &value,
                    "source-guidance publication",
                )?;
            } else {
                write_stdout(format_args!("Published source guidance for {} ({:?}, {:?}); {} changed files.\nPlan: {}\nCoordination: {:?}",value.repository_id(),value.host(),value.intent(),value.changed_files(),value.plan_digest(),value.coordination_effect()))?;
            }
        }
        (
            GuidanceCommand::Recovery {
                action: GuidanceRecoveryCommand::Plan { .. },
            },
            CommandOutcome::SourceGuidanceRecoveryPlan(value),
        ) => {
            let report = value.as_ref().map(|p| p.report());
            if format == ResultFormatArg::Json {
                write_json_result(
                    ResultCompletion::Complete,
                    &report,
                    "source-guidance recovery plan",
                )?;
            } else if let Some(report) = report {
                let mut text = format!(
                    "Recovery for {} ({:?}, {:?})\nRecovery: {}\n",
                    report.repository_id(),
                    report.host(),
                    report.intent(),
                    report.digest()
                );
                for file in report.files() {
                    text.push_str(&format!(
                        "  {}  {} ({:?})\n",
                        change_label(file.file().change()),
                        file.file().rel_path(),
                        file.evidence()
                    ));
                }
                for directory in report.directories() {
                    text.push_str(&format!(
                        "  directory  {} ({:?}; remove only if identity matches and empty)\n",
                        directory.rel_path(),
                        directory.state()
                    ));
                }
                text.push_str("Use --result-format json for complete proposed bytes and evidence, including local project facts. Uncheckpointed after-images establish matching data, not execution origin. Planning writes nothing.\n");
                write_stdout(format_args!("{}", text.trim_end()))?;
            } else {
                write_stdout(format_args!(
                    "No pending source-guidance recovery journal was observed."
                ))?;
            }
        }
        (
            GuidanceCommand::Recovery {
                action: GuidanceRecoveryCommand::Apply { .. },
            },
            CommandOutcome::SourceGuidanceRecoveryResult(value),
        ) => {
            if format == ResultFormatArg::Json {
                write_json_result(
                    ResultCompletion::Complete,
                    &value,
                    "source-guidance recovery",
                )?;
            } else {
                write_stdout(format_args!("Recovered guidance for {} ({:?}, {:?}); {} changed files.\nRecovery: {}\nRemoved directories: {}; retained directories: {}",value.repository_id(),value.host(),value.intent(),value.changed_files(),value.recovery_digest(),value.removed_directories().len(),value.retained_directories().len()))?;
            }
        }
        _ => {
            return Err(crate::diagnostic::internal_failure(
                "source-guidance result did not match its explicit request",
                anyhow::anyhow!("closed command partition was violated"),
            )
            .into())
        }
    }
    Ok(())
}

/// Render the complete owner value as JSON or a concise human inventory.
fn cmd_source_guidance(guidance: &SourceGuidanceProjection, format: ResultFormatArg) -> Result<()> {
    if format == ResultFormatArg::Json {
        return write_json_result(ResultCompletion::Complete, guidance, "source guidance");
    }
    let mut text = format!(
        "Source guidance for {} ({:?})\nProfile: {}\nProjection: {}\n\nProposed addition to {}:\n{}\nProposed files:\n",
        guidance.repository_id(), guidance.host(), guidance.profile_digest(), guidance.digest(),
        guidance.entrypoint_path(), guidance.entrypoint_contribution(),
    );
    for file in guidance.files() {
        text.push_str(&format!(
            "  {}  {} ({} bytes)\n",
            file.sha256(),
            file.rel_path(),
            file.content().len()
        ));
    }
    text.push_str(
        "\nUse --result-format json to inspect complete file bodies. Inspection writes no files.\n",
    );
    write_stdout(format_args!("{}", text.trim_end()))
}

/// Render exact local proposals without copying reconciliation or effect policy.
fn cmd_source_guidance_plan(
    plan: &LocalSourceGuidancePlanReport,
    format: ResultFormatArg,
) -> Result<()> {
    if format == ResultFormatArg::Json {
        return write_json_result(ResultCompletion::Complete, plan, "source-guidance plan");
    }
    let mut text = format!(
        "Source-guidance plan for {} ({:?}, {:?})\nPlan: {}\n\n",
        plan.repository_id(),
        plan.host(),
        plan.intent(),
        plan.digest(),
    );
    for file in plan.files() {
        let label = match file.change() {
            LocalSourceGuidanceChange::Write { .. }
                if matches!(
                    file.baseline(),
                    pale_sdk::LocalSourceGuidanceBaseline::Absent
                ) =>
            {
                "add"
            }
            LocalSourceGuidanceChange::Write { .. } => "update",
            LocalSourceGuidanceChange::Delete => "remove",
            LocalSourceGuidanceChange::Keep => "keep",
        };
        let order = plan
            .publication_order()
            .iter()
            .position(|path| path == file.rel_path())
            .map_or_else(|| "    ".to_owned(), |index| format!("{:>3}.", index + 1));
        text.push_str(&format!("  {order} {label:6} {}\n", file.rel_path()));
    }
    for file in plan.files() {
        diff::append(&mut text, file);
    }
    text.push_str("\nInstruction files are unchanged. This proposal's identity will be remembered for this host after display. Run guidance apply with the same workspace, repository and host to apply it; changed sources or files require a new plan. Numbers show write order.\n");
    write_stdout(format_args!("{}", text.trim_end()))
}

fn change_label(change: &LocalSourceGuidanceChange) -> &'static str {
    match change {
        LocalSourceGuidanceChange::Keep => "keep",
        LocalSourceGuidanceChange::Write { .. } => "write",
        LocalSourceGuidanceChange::Delete => "delete",
    }
}
