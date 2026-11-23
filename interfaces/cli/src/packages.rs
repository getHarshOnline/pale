//! Terminal projection of the SDK's checked Package catalog.

use crate::cli::{PackagesCommand, ResultFormatArg};
use crate::command::{CommandDispatcher, CommandOutcome, CommandRequest};
use crate::{diagnostic, terminal, DispatchFailure};

pub(super) fn run(
    action: &PackagesCommand,
    format: ResultFormatArg,
) -> Result<(), DispatchFailure> {
    let (root, resource) = match action {
        PackagesCommand::Check { root, .. } => (root, None),
        PackagesCommand::Show { root, resource } => (root, resource.as_deref()),
    };
    let root = root.to_str().ok_or_else(|| {
        diagnostic::invalid_input(
            "Package root must be UTF-8",
            anyhow::anyhow!("invalid source path"),
        )
    })?;
    let dispatcher =
        CommandDispatcher::new().map_err(|error| DispatchFailure::Host(error.into()))?;
    let request = match action {
        PackagesCommand::Check {
            write,
            caller_root,
            exclude_caller,
            ..
        } => CommandRequest::LocalPackageCheck(pale_sdk::LocalPackageCheckRequest {
            root: root.to_owned(),
            caller_roots: caller_root.clone(),
            exclude_callers: exclude_caller.clone(),
            write: *write,
        }),
        PackagesCommand::Show { .. } => CommandRequest::LocalPackageCatalog(root.to_owned()),
    };
    let CommandOutcome::PackageCatalog(catalog) = dispatcher.dispatch(request)? else {
        return Err(diagnostic::internal_failure(
            "Package reader returned an unrelated result",
            anyhow::anyhow!("unexpected command result"),
        )
        .into());
    };
    if matches!(action, PackagesCommand::Check { .. }) && !catalog.stale_indexes().is_empty() {
        return Err(diagnostic::invalid_input(
            "Package indexes need regeneration; run packages check --write",
            anyhow::anyhow!("{}", catalog.stale_indexes().join(", ")),
        )
        .into());
    }
    if let Some(resource) = resource {
        let text = catalog.show(resource).ok_or_else(|| {
            diagnostic::invalid_input(
                "Resource is absent or has no Markdown body",
                anyhow::anyhow!("resource {resource}"),
            )
        })?;
        return match format {
            ResultFormatArg::Human => {
                terminal::write_stdout(format_args!("{text}")).map_err(Into::into)
            }
            ResultFormatArg::Json => terminal::write_json_result(
                terminal::ResultCompletion::Complete,
                &text,
                "Package body",
            )
            .map_err(Into::into),
        };
    }
    let value = terminal::parse_owner_json(catalog.canonical_bytes(), "Package catalog")?;
    if matches!(format, ResultFormatArg::Json) || matches!(action, PackagesCommand::Show { .. }) {
        terminal::write_json_result(
            terminal::ResultCompletion::Complete,
            &value,
            "Package catalog",
        )?;
    } else {
        terminal::write_stdout(format_args!(
            "Package declarations and reading links passed."
        ))?;
        if let Some(callers) = value.get("callers") {
            terminal::write_stdout(format_args!(
                "Checked {} incoming references in {} caller documents.",
                callers["references"], callers["documents"]
            ))?;
        }
    }
    Ok(())
}
