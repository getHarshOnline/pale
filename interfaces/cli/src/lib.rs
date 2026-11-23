//! Typed command dispatcher and terminal projection for local and
//! attached-resident operations.
//!
//! This crate builds on `pale-sdk` alone within Pale; `pale-mcp` calls its
//! dispatcher directly.
//!
//! Inspection does not mutate target repositories. Apply and remove require a
//! self-verifying saved plan equal to a fresh plan of the same class. The reusable
//! [`command`] dispatcher also serves protocol clients. Repository hooks admit
//! routing fields and bounded native input, then deliver through an explicit
//! authenticated attachment. Ordinary file-configured hooks still block native
//! work; an embedder can supply independent admission for native responses.
//! Hooks do not compile or evaluate policy locally.

#[cfg(feature = "native-local")]
use std::path::Path;
#[cfg(feature = "native-local")]
use std::process::ExitCode;

#[cfg(feature = "native-local")]
use clap::Parser;

#[cfg(feature = "native-local")]
mod action;
#[cfg(feature = "native-local")]
mod cli;
pub mod command;
#[cfg(feature = "native-local")]
mod compiled_identity;
#[cfg(feature = "native-local")]
mod control_storage;
#[cfg(feature = "native-local")]
mod diagnostic;
#[cfg(feature = "native-local")]
mod doctor;
#[cfg(feature = "native-local")]
mod hook;
#[cfg(feature = "native-local")]
mod packages;
#[cfg(feature = "native-local")]
mod resident;
#[cfg(feature = "native-local")]
mod terminal;
#[cfg(feature = "native-local")]
mod workspace;
#[cfg(feature = "native-local")]
mod workspace_source;

#[cfg(test)]
#[path = "../tests/support/authoring_failure.rs"]
mod authoring_failure_fixture;
#[cfg(test)]
#[path = "../tests/support/fixtures.rs"]
mod test_fixtures;

#[cfg(feature = "native-local")]
use cli::{
    ActionDiscoveryCommand, AttachedCli, AttachedCmd, Cli, Cmd, ObservationAttachedCli,
    ObservationAttachedCmd, PlanKindArg, ResidentCommand, ResidentObservationCommand,
    ResultFormatArg,
};

/// Typed process boundary between API-owned diagnostics and leaf presentation.
///
/// [`Self::Api`] retains the exact admitted diagnostic produced by the shared
/// façade. [`Self::Host`] is deliberately limited to CLI-owned argument or
/// stream handling and terminal rendering; it owns no Pale application policy.
#[cfg(feature = "native-local")]
enum DispatchFailure {
    /// One admitted API failure requiring presentation only.
    Api(command::ApiError),
    /// One uncertain materialization failure plus its best-effort read-back.
    ApiWithReconciliation {
        /// Original admitted API failure. It remains the failure authority.
        error: command::ApiError,
        /// Bounded receipt and ownership observation made before custody ended.
        reconciliation: Box<pale_sdk::LocalMaterializationReconciliation>,
    },
    /// One admitted diagnostic carried by a closed API result.
    Admitted(Box<pale_sdk::DiagnosticV1>),
    /// One process-local failure still owned by the CLI composition shell.
    Host(anyhow::Error),
}

/// Borrowed command-line selection projected into one local API request.
#[cfg(feature = "native-local")]
struct WorkspaceSelection<'a> {
    manifest: &'a Path,
    project: &'a str,
    execution_workspace: &'a str,
    machine: &'a str,
}

#[cfg(feature = "native-local")]
impl From<command::ApiError> for DispatchFailure {
    fn from(error: command::ApiError) -> Self {
        Self::Api(error)
    }
}

#[cfg(feature = "native-local")]
impl From<anyhow::Error> for DispatchFailure {
    fn from(error: anyhow::Error) -> Self {
        Self::Host(error)
    }
}

/// Parse, execute, and report one invocation across its effect boundary.
///
/// Help, version, and syntax responses retain Clap's rendering and status.
/// Post-parse failures use the selected diagnostic format and a stable bounded
/// exit taxonomy: operational failure is `1`, expected `surface-status --exit-code`
/// drift is `3`, invalid input is `4`, current-state admission denial is `5`,
/// a Pale application invariant is `6`, and an explicitly uncertain effect is `7`.
/// Repository-native `hook` failures have a dedicated process projection: one
/// structured diagnostic on standard error, no success output, and status `2`,
/// the harness protocol's fail-closed status. This does not change the general
/// command taxonomy.
/// Reporting after a durable effect remains best-effort.
#[must_use]
#[cfg(feature = "native-local")]
pub fn run() -> ExitCode {
    match parse_cli(Cli::try_parse()) {
        Ok(cli) => run_stateless(cli),
        Err(status) => status,
    }
}

/// Run the command-line interface with one caller-provisioned resident attachment.
///
/// The caller remains responsible for endpoint selection, peer authentication,
/// response admission, and attachment provisioning. This route performs no
/// discovery and opens no resident storage. Repository hooks use the supplied
/// hook or operation attachment; other commands retain their stateless behavior.
#[must_use]
#[cfg(feature = "native-local")]
pub fn run_with_resident_attachment(attachment: command::ResidentAttachment) -> ExitCode {
    if !attachment.supports_operation_requests() {
        if !attachment.supports_observation_requests() {
            return match parse_cli(Cli::try_parse()) {
                Ok(cli) => run_stateless_with_attachment(cli, Some(attachment)),
                Err(status) => status,
            };
        }
        return run_with_observation_attachment(attachment);
    }
    let cli = match parse_cli(AttachedCli::try_parse()) {
        Ok(cli) => cli,
        Err(status) => return status,
    };
    match cli.cmd {
        AttachedCmd::Stateless(cmd) => run_stateless_with_attachment(
            Cli {
                arguments: cli.arguments,
                cmd,
            },
            Some(attachment),
        ),
        AttachedCmd::Resident { operation } => {
            let error_format = cli.arguments.error_format;
            match dispatch_resident(attachment, &operation, cli.arguments.result_format) {
                Ok(()) => ExitCode::SUCCESS,
                Err(failure) => report_dispatch_failure(failure, error_format),
            }
        }
    }
}

#[cfg(feature = "native-local")]
fn run_with_observation_attachment(attachment: command::ResidentAttachment) -> ExitCode {
    let cli = match parse_cli(ObservationAttachedCli::try_parse()) {
        Ok(cli) => cli,
        Err(status) => return status,
    };
    match cli.cmd {
        ObservationAttachedCmd::Stateless(cmd) => run_stateless(Cli {
            arguments: cli.arguments,
            cmd,
        }),
        ObservationAttachedCmd::Resident {
            operation: ResidentObservationCommand::Observe(arguments),
        } => {
            let error_format = cli.arguments.error_format;
            let operation = ResidentCommand::Observe(arguments);
            match dispatch_resident(attachment, &operation, cli.arguments.result_format) {
                Ok(()) => ExitCode::SUCCESS,
                Err(failure) => report_dispatch_failure(failure, error_format),
            }
        }
    }
}

#[cfg(feature = "native-local")]
fn parse_cli<T>(result: Result<T, clap::Error>) -> Result<T, ExitCode> {
    match result {
        Ok(cli) => Ok(cli),
        Err(error) => {
            let status = ExitCode::from(u8::try_from(error.exit_code()).unwrap_or(1));
            drop(error.print());
            Err(status)
        }
    }
}

#[cfg(feature = "native-local")]
fn run_stateless(cli: Cli) -> ExitCode {
    run_stateless_with_attachment(cli, None)
}

#[cfg(feature = "native-local")]
fn run_stateless_with_attachment(
    cli: Cli,
    attachment: Option<command::ResidentAttachment>,
) -> ExitCode {
    let is_repository_hook = matches!(&cli.cmd, Cmd::Hook { .. });
    let error_format = if is_repository_hook {
        cli::ErrorFormatArg::Json
    } else {
        cli.arguments.error_format
    };
    match dispatch_with_attachment(cli, attachment) {
        Ok(status) => status,
        Err(failure) => {
            let status = report_dispatch_failure(failure, error_format);
            if is_repository_hook {
                ExitCode::from(2)
            } else {
                status
            }
        }
    }
}

#[cfg(feature = "native-local")]
fn dispatch_resident(
    attachment: command::ResidentAttachment,
    operation: &cli::ResidentCommand,
    result_format: ResultFormatArg,
) -> Result<(), DispatchFailure> {
    let dispatcher = command::CommandDispatcher::with_resident_attachment(attachment)
        .map_err(|error| DispatchFailure::Host(error.into()))?;
    resident::run(&dispatcher, operation, result_format)
}

/// Report one post-parse failure using the general command taxonomy.
#[cfg(feature = "native-local")]
fn report_dispatch_failure(
    failure: DispatchFailure,
    error_format: cli::ErrorFormatArg,
) -> ExitCode {
    let diagnostic = match failure {
        DispatchFailure::Api(error) => diagnostic::Diagnostic::from_api_error(&error),
        DispatchFailure::ApiWithReconciliation {
            error,
            reconciliation,
        } => diagnostic::Diagnostic::from_api_error_with_reconciliation(&error, &reconciliation),
        DispatchFailure::Admitted(document) => {
            diagnostic::Diagnostic::from_api_diagnostic(&document)
        }
        DispatchFailure::Host(error) => diagnostic::Diagnostic::from_host_error(&error),
    };
    diagnostic::report(&diagnostic, error_format);
    diagnostic.exit_code()
}

/// Execute one successfully parsed command without owning process reporting.
#[cfg(feature = "native-local")]
fn dispatch_with_attachment(
    cli: Cli,
    attachment: Option<command::ResidentAttachment>,
) -> std::result::Result<ExitCode, DispatchFailure> {
    let error_format = cli.arguments.error_format;
    let result_format = cli.arguments.result_format;
    validate_result_format(&cli)?;
    if matches!(&cli.cmd, Cmd::CheckSource) {
        return workspace_source::run(&cli);
    }
    if let Cmd::Doctor {
        build_root,
        build_limit,
    } = &cli.cmd
    {
        return doctor::run(&cli, build_root, *build_limit, result_format);
    }
    if matches!(&cli.cmd, Cmd::CompiledIdentity) {
        compiled_identity::run(result_format)?;
        return Ok(ExitCode::SUCCESS);
    }

    if let Cmd::ControlStoreInspect(arguments) = &cli.cmd {
        control_storage::run(arguments, result_format)?;
        return Ok(ExitCode::SUCCESS);
    }

    if let Cmd::Action(arguments) = &cli.cmd {
        match &arguments.discovery {
            Some(ActionDiscoveryCommand::List) => {
                action::list_actions(result_format)?;
            }
            Some(ActionDiscoveryCommand::Describe { id }) => {
                action::describe_action(id, result_format)?;
            }
            None => {
                let id = arguments.id.as_deref().ok_or_else(|| {
                    diagnostic::invalid_input(
                        "action ID is required for invocation",
                        anyhow::anyhow!("select an action ID, or run `pale action list`"),
                    )
                })?;
                let source = match (
                    arguments.input_root.as_deref(),
                    arguments.input_file.as_deref(),
                ) {
                    (Some(root), Some(relative)) => {
                        action::ActionInput::Confined { root, relative }
                    }
                    (None, None) => action::ActionInput::Stdin,
                    _ => {
                        return Err(diagnostic::invalid_input(
                            "native-action input selection is incomplete",
                            anyhow::anyhow!(
                                "--input-root and --input-file must be supplied together"
                            ),
                        )
                        .into())
                    }
                };
                action::run_action(id, source, result_format)?;
            }
        }
        return Ok(ExitCode::SUCCESS);
    }

    // Admit hook input without falling back to one-shot policy evaluation.
    if let Cmd::Hook {
        resident_hook_config,
        event,
        workspace,
        repository,
        provider_binding,
        provider_binding_identity,
    } = &cli.cmd
    {
        return hook::run_hook(
            hook::HookInvocation {
                event,
                workspace,
                repository,
                provider_binding,
                provider_binding_identity,
            },
            attachment,
            resident_hook_config.as_deref(),
        );
    }

    if let Cmd::Packages { action } = &cli.cmd {
        packages::run(action, result_format)?;
        return Ok(ExitCode::SUCCESS);
    }
    if let Cmd::DistributionInfo { lockfile } = &cli.cmd {
        let selection = require_workspace_selection(&cli)?;
        let dispatcher = command::CommandDispatcher::new()
            .map_err(|error| DispatchFailure::Host(error.into()))?;
        let outcome = dispatcher.dispatch(command::CommandRequest::LocalPackageDistribution(
            command::LocalPackageDistributionRequest::new(
                local_workspace_locator(&selection)?,
                selection.project.to_owned(),
                selection.execution_workspace.to_owned(),
                selection.machine.to_owned(),
            ),
        ))?;
        let command::CommandOutcome::PackageDistribution(info) = outcome else {
            return Err(diagnostic::internal_failure(
                "Package inspection returned an unrelated command result",
                anyhow::anyhow!("closed command result did not match its request"),
            )
            .into());
        };
        workspace::cmd_distribution_info(&info, *lockfile, result_format)?;
        return Ok(ExitCode::SUCCESS);
    }
    if let Cmd::ReviewConfiguration {
        candidate_workspace,
        action,
    } = &cli.cmd
    {
        let selection = require_workspace_selection(&cli)?;
        let candidate = candidate_workspace.to_str().ok_or_else(|| {
            diagnostic::invalid_input(
                "candidate workspace path is not UTF-8",
                anyhow::anyhow!("--candidate-workspace must be valid UTF-8"),
            )
        })?;
        workspace::cmd_review_configuration(
            command::LocalConfigurationImprovementRequest::new(
                local_workspace_locator(&selection)?,
                command::LocalWorkspaceLocator::new(candidate.to_owned()),
                selection.project.to_owned(),
                selection.execution_workspace.to_owned(),
                selection.machine.to_owned(),
            ),
            *action,
            result_format,
        )?;
        return Ok(ExitCode::SUCCESS);
    }
    if let Cmd::Guidance { action } = &cli.cmd {
        workspace::cmd_guidance(local_one_shot_request(&cli)?, action, result_format)?;
        return Ok(ExitCode::SUCCESS);
    }
    if matches!(&cli.cmd, Cmd::Summary | Cmd::Validate | Cmd::Compile) {
        let summary = compile_local_one_shot_summary(&cli)?;
        match cli.cmd {
            Cmd::Summary => workspace::cmd_summary(&summary, result_format)?,
            Cmd::Validate => workspace::cmd_validate(&summary, result_format)?,
            Cmd::Compile => workspace::cmd_compile(&summary, result_format)?,
            _ => {
                return Err(diagnostic::internal_failure(
                    "a non-compilation command reached the one-shot projection branch",
                    anyhow::anyhow!("closed command partition was violated"),
                )
                .into())
            }
        }
        return Ok(ExitCode::SUCCESS);
    }

    if matches!(
        &cli.cmd,
        Cmd::Plan { .. } | Cmd::SurfaceStatus { .. } | Cmd::Apply { .. } | Cmd::Remove { .. }
    ) {
        let plan_kind = match &cli.cmd {
            Cmd::Plan {
                kind,
                adopt_unowned,
                ..
            } => Some(local_plan_kind(*kind, *adopt_unowned)?),
            Cmd::Apply { adopt_unowned, .. } => Some(if *adopt_unowned {
                pale_sdk::OneShotPlanKind::Adopt
            } else {
                pale_sdk::OneShotPlanKind::Apply
            }),
            Cmd::Remove { .. } => Some(pale_sdk::OneShotPlanKind::Remove),
            Cmd::SurfaceStatus { .. } => None,
            _ => unreachable!("the local-inspection command partition was checked"),
        };
        let dispatcher = command::CommandDispatcher::new()
            .map_err(|error| DispatchFailure::Host(error.into()))?;
        let compilation = compile_local_one_shot(&dispatcher, &cli)?;
        match cli.cmd {
            Cmd::Plan { out, .. } => {
                let kind = plan_kind.expect("a parsed plan command has one validated plan kind");
                let output_directory = local_directory_argument(&out, "--out")?;
                workspace::cmd_plan(
                    &dispatcher,
                    compilation,
                    output_directory,
                    kind,
                    result_format,
                )?;
            }
            Cmd::SurfaceStatus { exit_code } => {
                let outcome =
                    workspace::cmd_surface_status(&dispatcher, compilation, result_format)?;
                if exit_code && outcome == workspace::SurfaceStatusOutcome::Drift {
                    return Ok(ExitCode::from(3));
                }
            }
            Cmd::Apply {
                plans, receipts, ..
            }
            | Cmd::Remove { plans, receipts } => {
                let kind = plan_kind.expect("a parsed mutation command has one plan kind");
                workspace::cmd_materialize(
                    &dispatcher,
                    compilation,
                    kind,
                    local_directory_argument(&plans, "--plans")?,
                    local_directory_argument(&receipts, "--receipts")?,
                    error_format,
                    result_format,
                )?;
            }
            _ => {
                return Err(diagnostic::internal_failure(
                    "a non-local command reached the local API branch",
                    anyhow::anyhow!("closed command partition was violated"),
                )
                .into())
            }
        }
        return Ok(ExitCode::SUCCESS);
    }
    Err(diagnostic::internal_failure(
        "a parsed command reached no CLI dispatch partition",
        anyhow::anyhow!("closed command partition was violated"),
    )
    .into())
}

/// Reject machine-result requests that lack one exact structured owner value.
///
/// This check runs before workspace capture, native input reads, plan
/// publication, or materialization. Human presentation remains available for
/// every current command.
#[cfg(feature = "native-local")]
fn validate_result_format(cli: &Cli) -> std::result::Result<(), DispatchFailure> {
    if cli.arguments.result_format != ResultFormatArg::Json {
        return Ok(());
    }
    let unsupported = match &cli.cmd {
        Cmd::Hook { .. } => Some("repository-hook delivery has no successful result in this build"),
        _ => None,
    };
    if let Some(message) = unsupported {
        return Err(diagnostic::invalid_input(
            "JSON result format is unavailable for this command",
            anyhow::anyhow!(message),
        )
        .into());
    }
    Ok(())
}

/// Resolve and compile one stable local snapshot through the API-owned host seam.
#[cfg(feature = "native-local")]
fn compile_local_one_shot_summary(
    cli: &Cli,
) -> std::result::Result<pale_sdk::OneShotCompilationSummary, DispatchFailure> {
    let dispatcher =
        command::CommandDispatcher::new().map_err(|error| DispatchFailure::Host(error.into()))?;
    Ok(compile_local_one_shot(&dispatcher, cli)?.summary())
}

/// Resolve one local snapshot through the API-owned source-admission seam.
#[cfg(feature = "native-local")]
fn compile_local_one_shot(
    dispatcher: &command::CommandDispatcher,
    cli: &Cli,
) -> std::result::Result<Box<pale_sdk::LocalOneShotCompilation>, DispatchFailure> {
    let outcome = dispatcher.dispatch(command::CommandRequest::LocalOneShotCompilation(
        local_one_shot_request(cli)?,
    ))?;
    let command::CommandOutcome::LocalOneShotCompilation(compilation) = outcome else {
        return Err(diagnostic::internal_failure(
            "local compilation returned an unrelated command result",
            anyhow::anyhow!("closed command result did not match its request"),
        )
        .into());
    };
    Ok(compilation)
}

/// Translate one complete CLI selection into the API-owned local request.
#[cfg(feature = "native-local")]
fn local_one_shot_request(
    cli: &Cli,
) -> std::result::Result<pale_sdk::LocalOneShotCompilationRequest, DispatchFailure> {
    let selection = require_workspace_selection(cli)?;
    Ok(pale_sdk::LocalOneShotCompilationRequest::new(
        local_workspace_locator(&selection)?,
        selection.project.to_owned(),
        selection.execution_workspace.to_owned(),
        selection.machine.to_owned(),
    ))
}

/// Translate the CLI's apply/adopt/remove spelling into the API-owned plan law.
#[cfg(feature = "native-local")]
fn local_plan_kind(
    kind: PlanKindArg,
    adopt_unowned: bool,
) -> std::result::Result<pale_sdk::OneShotPlanKind, DispatchFailure> {
    match (kind, adopt_unowned) {
        (PlanKindArg::Apply, false) => Ok(pale_sdk::OneShotPlanKind::Apply),
        (PlanKindArg::Apply, true) => Ok(pale_sdk::OneShotPlanKind::Adopt),
        (PlanKindArg::Remove, false) => Ok(pale_sdk::OneShotPlanKind::Remove),
        (PlanKindArg::Remove, true) => Err(diagnostic::invalid_input(
            "the selected plan options are incompatible",
            anyhow::anyhow!("--adopt-unowned is valid only for apply plans"),
        )
        .into()),
    }
}

/// Convert one command-line directory spelling at the sole leaf-owned boundary.
#[cfg(feature = "native-local")]
fn local_directory_argument(
    path: &Path,
    flag: &'static str,
) -> std::result::Result<String, DispatchFailure> {
    path.to_str().map(str::to_owned).ok_or_else(|| {
        diagnostic::invalid_input(
            "local directory selection is not UTF-8",
            anyhow::anyhow!("{flag} must be valid UTF-8"),
        )
        .into()
    })
}

/// Convert the CLI's UTF-8 path spelling into the API-owned local locator.
///
/// The API owns absolute-path, traversal, source-capture, and workspace
/// admission policy. The CLI owns only command-line conversion, including the
/// fact that a non-UTF-8 argv path cannot cross this binding-safe boundary.
#[cfg(feature = "native-local")]
fn local_workspace_locator(
    selection: &WorkspaceSelection<'_>,
) -> std::result::Result<pale_sdk::LocalWorkspaceLocator, DispatchFailure> {
    let manifest = selection.manifest.to_str().ok_or_else(|| {
        diagnostic::invalid_input(
            "workspace selection is not UTF-8",
            anyhow::anyhow!("--workspace must be valid UTF-8"),
        )
    })?;
    Ok(pale_sdk::LocalWorkspaceLocator::new(manifest.to_owned()))
}

/// Require the canonical workspace/project/execution-workspace/machine tuple.
#[cfg(feature = "native-local")]
fn require_workspace_selection(cli: &Cli) -> anyhow::Result<WorkspaceSelection<'_>> {
    let manifest = cli.arguments.workspace.as_deref().ok_or_else(|| {
        diagnostic::invalid_input(
            "workspace selection is incomplete",
            anyhow::anyhow!("--workspace <pale-workspace.toml> is required"),
        )
    })?;
    let project = cli.arguments.project.as_deref().ok_or_else(|| {
        diagnostic::invalid_input(
            "workspace selection is incomplete",
            anyhow::anyhow!("--project <id> is required"),
        )
    })?;
    let execution_workspace = cli
        .arguments
        .execution_workspace
        .as_deref()
        .ok_or_else(|| {
            diagnostic::invalid_input(
                "workspace selection is incomplete",
                anyhow::anyhow!("--execution-workspace <id> is required"),
            )
        })?;
    let machine = cli.arguments.machine.as_deref().ok_or_else(|| {
        diagnostic::invalid_input(
            "workspace selection is incomplete",
            anyhow::anyhow!("--machine <id> is required"),
        )
    })?;
    Ok(WorkspaceSelection {
        manifest,
        project,
        execution_workspace,
        machine,
    })
}
