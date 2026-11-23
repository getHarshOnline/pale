//! Read-only workspace inspection commands.

use anyhow::Result;

use pale_sdk::{LocalOneShotCompilation, OneShotCompilationSummary};

use crate::cli::ResultFormatArg;
use crate::command::{
    CommandDispatcher, CommandOutcome, CommandRequest, LocalMaterializationCommand,
    LocalMaterializationOutcome,
};
use crate::DispatchFailure;

/// Deliver API-owned Package coordinates as inspectable JSON.
pub(crate) fn cmd_distribution_info(
    info: &pale_sdk::PackageDistributionInspection,
    lockfile: bool,
    format: ResultFormatArg,
) -> Result<()> {
    if lockfile {
        return match format {
            ResultFormatArg::Human => {
                crate::terminal::write_stdout(format_args!("{}", info.lockfile_toml().trim_end()))
            }
            ResultFormatArg::Json => crate::terminal::write_json_result(
                crate::terminal::ResultCompletion::Complete,
                info.lockfile_toml(),
                "distribution lock",
            ),
        };
    }
    match format {
        ResultFormatArg::Human => {
            let output = crate::terminal::render_pretty_json(
                info.canonical_bytes(),
                "Package distribution inspection",
            )?;
            crate::terminal::write_stdout(format_args!("{output}"))
        }
        ResultFormatArg::Json => {
            let result = crate::terminal::parse_owner_json(
                info.canonical_bytes(),
                "Package distribution inspection",
            )?;
            crate::terminal::write_json_result(
                crate::terminal::ResultCompletion::Complete,
                &result,
                "Package distribution inspection",
            )
        }
    }
}

/// Report the identities established by a successful compilation.
pub(crate) fn cmd_validate(
    summary: &OneShotCompilationSummary,
    format: ResultFormatArg,
) -> Result<()> {
    if format == ResultFormatArg::Json {
        let result = crate::terminal::parse_owner_json(
            summary.canonical_bytes(),
            "validated compilation summary",
        )?;
        return crate::terminal::write_json_result(
            crate::terminal::ResultCompletion::Complete,
            &result,
            "validated compilation summary",
        );
    }
    let execution_paths = summary
        .repositories()
        .iter()
        .flat_map(|repository| repository.execution_paths())
        .map(|path| path.selection_id())
        .collect::<Vec<_>>();
    crate::terminal::write_stdout(format_args!(
        "validated: {} repositories; execution paths {:?}; profile {}",
        summary.repository_count(),
        execution_paths,
        summary.profile_digest()
    ))
}

/// Report generated surface and manifest counts without materializing them.
pub(crate) fn cmd_compile(
    summary: &OneShotCompilationSummary,
    format: ResultFormatArg,
) -> Result<()> {
    if format == ResultFormatArg::Json {
        let result = crate::terminal::parse_owner_json(
            summary.canonical_bytes(),
            "compiled workspace summary",
        )?;
        return crate::terminal::write_json_result(
            crate::terminal::ResultCompletion::Complete,
            &result,
            "compiled workspace summary",
        );
    }
    for repository in summary.repositories() {
        crate::terminal::write_stdout(format_args!(
            "{}: {} surfaces; manifest {}",
            repository.repository_id(),
            repository.surface_count(),
            repository.manifest_digest()
        ))?;
    }
    Ok(())
}

/// Typed result of comparing every target with the freshly reconstructed apply state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SurfaceStatusOutcome {
    /// Every target, ownership record, and machine-local policy file matches.
    Clean,
    /// At least one target, ownership record, or machine-local policy file differs.
    Drift,
}

/// Compare each target with fresh apply-state observations without terminating the process.
pub(crate) fn cmd_surface_status(
    dispatcher: &CommandDispatcher,
    compilation: Box<LocalOneShotCompilation>,
    format: ResultFormatArg,
) -> std::result::Result<SurfaceStatusOutcome, DispatchFailure> {
    if format == ResultFormatArg::Human {
        return report_surface_status_lines(dispatcher, compilation);
    }
    let outcome = dispatcher.dispatch(CommandRequest::LocalMaterialization(
        LocalMaterializationCommand::InspectWorkspace { compilation },
    ))?;
    let CommandOutcome::LocalMaterialization(LocalMaterializationOutcome::WorkspaceStatus {
        compilation: _,
        status,
    }) = outcome
    else {
        return Err(crate::diagnostic::internal_failure(
            "local workspace inspection returned an unrelated command result",
            anyhow::anyhow!("closed command result did not match its request"),
        )
        .into());
    };
    let result =
        crate::terminal::parse_owner_json(status.canonical_bytes(), "local workspace status")?;
    crate::terminal::write_json_result(
        crate::terminal::ResultCompletion::Complete,
        &result,
        "local workspace status",
    )?;
    Ok(if status.is_clean() {
        SurfaceStatusOutcome::Clean
    } else {
        SurfaceStatusOutcome::Drift
    })
}

fn report_surface_status_lines(
    dispatcher: &CommandDispatcher,
    compilation: Box<LocalOneShotCompilation>,
) -> std::result::Result<SurfaceStatusOutcome, DispatchFailure> {
    let repository_ids = compilation
        .summary()
        .repositories()
        .iter()
        .map(|repository| repository.repository_id().to_owned())
        .collect::<Vec<_>>();
    let mut compilation = compilation;
    let mut any_drift = false;
    for repository_id in repository_ids {
        let outcome = dispatcher.dispatch(CommandRequest::LocalMaterialization(
            LocalMaterializationCommand::InspectRepository {
                compilation,
                repository_id,
            },
        ))?;
        let CommandOutcome::LocalMaterialization(LocalMaterializationOutcome::RepositoryStatus {
            compilation: next,
            status,
        }) = outcome
        else {
            return Err(crate::diagnostic::internal_failure(
                "local repository inspection returned an unrelated command result",
                anyhow::anyhow!("closed command result did not match its request"),
            )
            .into());
        };
        compilation = next;
        if status.is_clean() {
            crate::terminal::write_stdout(format_args!("{}: no drift", status.repository_id()))?;
        } else {
            any_drift = true;
            let paths = status
                .drift()
                .iter()
                .map(|drift| drift.status_label())
                .collect::<Vec<_>>();
            crate::terminal::write_stdout(format_args!(
                "{}: drift on {}",
                status.repository_id(),
                paths.join(", ")
            ))?;
        }
    }
    Ok(if any_drift {
        SurfaceStatusOutcome::Drift
    } else {
        SurfaceStatusOutcome::Clean
    })
}
