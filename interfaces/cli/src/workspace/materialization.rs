//! Terminal projection for API-owned local materialization settlement.

use pale_sdk::{
    DiagnosticEffectCertainty, LocalMaterializationAdmissionRequest,
    LocalMaterializationAdmittedRun, LocalMaterializationReconciliation,
    LocalMaterializationRunOutcome, LocalMaterializationTargetOutcome, LocalOneShotCompilation,
    OneShotPlanKind,
};

use crate::cli::{ErrorFormatArg, ResultFormatArg};
use crate::command::{
    CommandDispatcher, CommandOutcome, CommandRequest, LocalMaterializationCommand,
    LocalMaterializationOutcome, LocalMaterializationSettlementDisposition,
};
use crate::DispatchFailure;

/// Admit and settle one complete API-owned local materialization operation.
///
/// Saved-plan reads, current-target observation, executable-binding verification,
/// repository and machine-local effects, durable evidence, rollback, and
/// reconciliation remain behind the API boundary. This function owns only
/// terminal reporting of the binding-safe result.
pub(crate) fn cmd_materialize(
    dispatcher: &CommandDispatcher,
    compilation: Box<LocalOneShotCompilation>,
    kind: OneShotPlanKind,
    saved_plan_directory: String,
    evidence_directory: String,
    _error_format: ErrorFormatArg,
    result_format: ResultFormatArg,
) -> Result<(), DispatchFailure> {
    let outcome = dispatcher.dispatch(CommandRequest::LocalMaterialization(
        LocalMaterializationCommand::Admit {
            compilation,
            request: LocalMaterializationAdmissionRequest::new(
                kind,
                saved_plan_directory,
                evidence_directory,
            ),
        },
    ))?;
    let CommandOutcome::LocalMaterialization(LocalMaterializationOutcome::Admitted(run)) = outcome
    else {
        return Err(crate::diagnostic::internal_failure(
            "local materialization admission returned an unrelated command result",
            anyhow::anyhow!("closed command result did not match its request"),
        )
        .into());
    };
    let outcome = dispatcher.dispatch(CommandRequest::LocalMaterialization(
        LocalMaterializationCommand::Settle(run),
    ))?;
    let CommandOutcome::LocalMaterialization(LocalMaterializationOutcome::Settlement(disposition)) =
        outcome
    else {
        return Err(crate::diagnostic::internal_failure(
            "local materialization settlement returned an unrelated command result",
            anyhow::anyhow!("closed command result did not match its request"),
        )
        .into());
    };
    let result = match disposition {
        LocalMaterializationSettlementDisposition::Settled(result) => result,
        LocalMaterializationSettlementDisposition::Rejected { error, run } => {
            if error.diagnostic().effect() == DiagnosticEffectCertainty::OutcomeUnknown {
                if let Some(reconciliation) = inspect_reconciliation(dispatcher, run) {
                    return Err(DispatchFailure::ApiWithReconciliation {
                        error,
                        reconciliation: Box::new(reconciliation),
                    });
                }
            }
            return Err(error.into());
        }
    };

    if result_format == ResultFormatArg::Json {
        let value = crate::terminal::parse_owner_json(
            result.canonical_bytes(),
            "materialization run result",
        )?;
        let completion = result_completion(result.outcome());
        crate::terminal::report_json_result(completion, &value, "materialization run result");
    } else {
        for target in result.targets() {
            if target.outcome() != LocalMaterializationTargetOutcome::Succeeded {
                continue;
            }
            match result.kind() {
                OneShotPlanKind::Apply | OneShotPlanKind::Adopt => {
                    crate::terminal::report_stdout(format_args!(
                        "apply {}: {} written; {} deferred effects",
                        target.repository_id(),
                        target.changed_surface_count(),
                        target.deferred_effect_count()
                    ));
                    if target.local_git_exclude_changed() {
                        crate::terminal::report_stdout(format_args!(
                            "apply {}: local ignore updated (.git/info/exclude)",
                            target.repository_id()
                        ));
                    }
                }
                OneShotPlanKind::Remove => crate::terminal::report_stdout(format_args!(
                    "remove {}: {} removed/spliced",
                    target.repository_id(),
                    target.changed_surface_count()
                )),
            }
        }
    }

    if result.outcome() == LocalMaterializationRunOutcome::Succeeded {
        return Ok(());
    }
    let diagnostic = result
        .targets()
        .iter()
        .find_map(|target| target.failure())
        .ok_or_else(|| {
            crate::diagnostic::internal_failure(
                "a failed materialization result carried no target diagnostic",
                anyhow::anyhow!(
                    "API result {:?} had {} target rows and {} successful rows",
                    result.outcome(),
                    result.target_count(),
                    result.succeeded_target_count()
                ),
            )
        })?;
    Err(DispatchFailure::Admitted(Box::new(diagnostic.clone())))
}

/// Classify result completeness independently of settlement success.
fn result_completion(outcome: LocalMaterializationRunOutcome) -> crate::terminal::ResultCompletion {
    match outcome {
        LocalMaterializationRunOutcome::Succeeded
        | LocalMaterializationRunOutcome::FailedNoEffect => {
            crate::terminal::ResultCompletion::Complete
        }
        LocalMaterializationRunOutcome::Partial => crate::terminal::ResultCompletion::Partial,
    }
}

/// Best-effort read-back after an explicitly uncertain settlement.
///
/// The original outcome-unknown diagnostic remains authoritative. Inspection
/// failure never replaces that primary settlement error.
fn inspect_reconciliation(
    dispatcher: &CommandDispatcher,
    run: Box<LocalMaterializationAdmittedRun>,
) -> Option<LocalMaterializationReconciliation> {
    let Ok(outcome) = dispatcher.dispatch(CommandRequest::LocalMaterialization(
        LocalMaterializationCommand::Reconcile(run),
    )) else {
        return None;
    };
    let CommandOutcome::LocalMaterialization(LocalMaterializationOutcome::Reconciliation(
        observation,
    )) = outcome
    else {
        return None;
    };
    Some(observation)
}

#[cfg(test)]
#[path = "../../tests/internal/materialization_result.rs"]
mod tests;
