//! Command-line syntax and user-facing help.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Arguments shared by the stateless and attachment-qualified parsers.
#[derive(Args)]
pub(super) struct CliArguments {
    /// Rendering used for post-parse Pale failures. Repository-hook failures
    /// always use structured JSON for their native host. Clap continues to own
    /// help, version, and command-line syntax responses.
    #[arg(long, global = true, value_enum, default_value_t = ErrorFormatArg::Human)]
    pub(super) error_format: ErrorFormatArg,
    /// Rendering for successful, partial, or non-terminal command results.
    /// JSON is available only when the SDK exposes an exact structured value.
    #[arg(long, global = true, value_enum, default_value_t = ResultFormatArg::Human)]
    pub(super) result_format: ResultFormatArg,
    /// Root `pale-workspace.toml`; all compiler inputs resolve from this entry.
    #[arg(long)]
    pub(super) workspace: Option<PathBuf>,
    /// Canonical project identity declared by the workspace topology.
    #[arg(long)]
    pub(super) project: Option<String>,
    /// Canonical execution-workspace identity linked to the selected project.
    #[arg(long)]
    pub(super) execution_workspace: Option<String>,
    /// Machine id beneath the manifest's declared binding root.
    #[arg(long)]
    pub(super) machine: Option<String>,
}

#[derive(Parser)]
#[command(
    name = "pale",
    version,
    about = "Pale workspace compiler and local command interface"
)]
/// One parsed stateless command invocation.
pub(super) struct Cli {
    /// Shared output and workspace selection.
    #[command(flatten)]
    pub(super) arguments: CliArguments,
    /// Selected stateless command and its command-local arguments.
    #[command(subcommand)]
    pub(super) cmd: Cmd,
}

#[derive(Parser)]
#[command(
    name = "pale",
    version,
    about = "Pale command interface with an authenticated resident attachment"
)]
/// One parsed invocation from an attachment-qualified embedder.
pub(super) struct AttachedCli {
    /// Shared output and workspace selection.
    #[command(flatten)]
    pub(super) arguments: CliArguments,
    /// Selected attached or stateless command.
    #[command(subcommand)]
    pub(super) cmd: AttachedCmd,
}

#[derive(Parser)]
#[command(
    name = "pale",
    version,
    about = "Pale command interface with a read-only resident attachment"
)]
/// One parsed invocation from a read-only attachment-qualified embedder.
pub(super) struct ObservationAttachedCli {
    /// Shared output and workspace selection.
    #[command(flatten)]
    pub(super) arguments: CliArguments,
    /// Selected read-only attached or stateless command.
    #[command(subcommand)]
    pub(super) cmd: ObservationAttachedCmd,
}

/// Commands visible through a read-only resident attachment.
#[derive(Subcommand)]
pub(super) enum ObservationAttachedCmd {
    /// Reuse the complete stateless command surface without duplicating it.
    #[command(flatten)]
    Stateless(Cmd),
    /// Read one retained operation from the caller-provisioned resident.
    Resident {
        /// Exact read-only resident operation.
        #[command(subcommand)]
        operation: ResidentObservationCommand,
    },
}

/// Commands visible only to an attachment-qualified parser.
#[derive(Subcommand)]
pub(super) enum AttachedCmd {
    /// Reuse the complete stateless command surface without duplicating it.
    #[command(flatten)]
    Stateless(Cmd),
    /// Use the caller-provisioned resident attachment.
    Resident {
        /// Exact resident operation to perform.
        #[command(subcommand)]
        operation: ResidentCommand,
    },
}

/// Closed renderings for post-parse command failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum ErrorFormatArg {
    /// Concise developer-facing text on standard error.
    Human,
    /// One human-readable JSON object on standard error.
    Json,
}

/// Closed renderings for command results written to standard output.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum ResultFormatArg {
    /// Keep the established command-specific terminal presentation.
    Human,
    /// Emit one CLI-owned envelope containing the exact structured owner value.
    Json,
}

/// Closed plan classes accepted at the command-line boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(super) enum PlanKindArg {
    /// Reconcile the selected surfaces into each repository.
    Apply,
    /// Remove only the files and settings that this machine's installation history shows Pale owns.
    Remove,
}

/// Discovery commands over the compiled effect-free action catalog.
#[derive(Clone, Debug, Subcommand)]
pub(super) enum ActionDiscoveryCommand {
    /// List the available built-in computations in stable identity order.
    List,
    /// Show one action's contract and CLI input construction details.
    Describe {
        /// Exact action contract ID (for example `pale.compute.path-inventory`).
        id: String,
    },
}

/// Invocation or discovery of effect-free built-in computations.
#[derive(Args)]
#[command(subcommand_negates_reqs = true, args_conflicts_with_subcommands = true)]
pub(super) struct ActionArgs {
    /// Exact action contract ID. Required for invocation; omit it only when
    /// selecting `list` or `describe` (for example `pale.compute.path-inventory`).
    pub(super) id: Option<String>,
    /// Confinement root for `--input-file`; both flags must be supplied together.
    #[arg(long, requires = "input_file")]
    pub(super) input_root: Option<PathBuf>,
    /// Complete tagged `action-input.v1` JSON at a relative no-follow path
    /// beneath `--input-root`. When omitted, the CLI reads it from stdin.
    #[arg(long, requires = "input_root")]
    pub(super) input_file: Option<PathBuf>,
    /// Inspect the compiled catalog instead of running a computation.
    #[command(subcommand)]
    pub(super) discovery: Option<ActionDiscoveryCommand>,
}

#[derive(Subcommand)]
/// Supported identity, inspection, planning, and mutation commands.
pub(super) enum Cmd {
    /// Check local module sources and owner/duty references without publication or activation.
    CheckSource,
    /// Diagnose declared configuration and machine facts without repair or cleanup.
    Doctor {
        /// Absolute physical build directory; repeat for up to 32 separate trees.
        #[arg(long)]
        build_root: Vec<String>,
        /// Maximum logical regular-file bytes per tree, not reclaimable space.
        #[arg(long)]
        build_limit: Option<u64>,
    },
    /// Check Package sources or show a method and its reading links.
    Packages {
        /// Authoring action; no workspace or runtime is required.
        #[command(subcommand)]
        action: PackagesCommand,
    },
    /// Review, apply or recover Package instructions for a repository.
    Guidance {
        /// Instruction review, update or recovery action.
        #[command(subcommand)]
        action: GuidanceCommand,
    },
    /// Report compiled version and protocol as JSON, without verifying installation or health.
    CompiledIdentity,
    /// Inspect physical control-store state without creation, repair or resident admission.
    ControlStoreInspect(ControlStoreInspectionArgs),
    /// Compute canonical distribution and package digests without consuming the lock.
    DistributionInfo {
        /// Emit the exact inspected distribution lock as TOML instead of inventory JSON.
        /// This writes only stdout; review it before replacing a project lock.
        #[arg(long)]
        lockfile: bool,
    },
    /// Review a candidate configuration without applying it.
    ReviewConfiguration {
        /// Absolute candidate workspace manifest, using the baseline's selectors.
        #[arg(long)]
        candidate_workspace: PathBuf,
        /// Preview, or finish a fresh review using the earlier summary from stdin.
        #[command(subcommand)]
        action: ConfigurationReviewAction,
    },
    /// Inspect or run effect-free built-in computations, without a workspace.
    Action(ActionArgs),
    /// Load + validate the workspace and distribution (read-only).
    Validate,
    /// Compile and print generated surface counts and identity digests (read-only).
    Compile,
    /// Build and save exact plans without mutating target repositories.
    Plan {
        /// Absolute, parent-free directory that receives the saved plans.
        ///
        /// Pale creates the directory when it is absent. Publication validates
        /// and opens it before writing any plan file.
        #[arg(long)]
        out: PathBuf,
        /// Plan class: `apply` (default) or `remove`.
        #[arg(long, default_value = "apply")]
        kind: PlanKindArg,
        /// Explicitly plan first ownership of pre-existing generated targets.
        /// Whole files are replaced under their exact baseline; managed blocks
        /// require existing valid markers and preserve all surrounding bytes.
        /// Ordinary apply planning never adopts unowned surfaces.
        #[arg(long)]
        adopt_unowned: bool,
    },
    /// Report drift between the apply plan and each target (read-only).
    SurfaceStatus {
        /// Exit with status 3 when any target drifts.
        #[arg(long)]
        exit_code: bool,
    },
    /// Emit a deterministic, machine-independent semantic summary as readable JSON.
    Summary,
    /// Apply surfaces from saved plans that match freshly rebuilt apply plans.
    Apply {
        /// Directory containing the exact saved apply plans.
        #[arg(long)]
        plans: PathBuf,
        /// Directory to write durable receipts + the run result.
        #[arg(long)]
        receipts: PathBuf,
        /// Admit only plans generated with `plan --adopt-unowned` and record
        /// pre-existing generated targets as adoption rather than creation.
        #[arg(long)]
        adopt_unowned: bool,
    },
    /// Remove owned surfaces from saved plans that match freshly rebuilt remove plans.
    Remove {
        /// Directory containing the exact saved remove plans.
        #[arg(long)]
        plans: PathBuf,
        /// Directory to write durable receipts and the run result.
        #[arg(long)]
        receipts: PathBuf,
    },
    /// Submit a repository hook event through an explicitly selected resident connection.
    Hook {
        /// Protected connection configuration; no default or environment lookup.
        #[arg(long)]
        resident_hook_config: Option<PathBuf>,
        /// Hook event name from the selected host configuration.
        event: String,
        /// Workspace ID from the hook configuration.
        #[arg(long)]
        workspace: String,
        /// Repository ID where the host reported the event.
        #[arg(long)]
        repository: String,
        /// Provider binding ID from the hook configuration.
        #[arg(long)]
        provider_binding: String,
        /// Digest of the complete provider binding named by the hook configuration.
        #[arg(long)]
        provider_binding_identity: String,
    },
}

#[derive(Subcommand)]
pub(super) enum PackagesCommand {
    /// Check all declared resources, prerequisites and local heading links.
    Check {
        /// Markdown caller directory to check (repeat for multiple trees).
        #[arg(long)]
        caller_root: Vec<String>,
        /// Caller subtree to skip; a single name is skipped at every depth.
        #[arg(long, requires = "caller_root")]
        exclude_caller: Vec<String>,
        /// Regenerate stale family indexes after all source checks pass.
        #[arg(long)]
        write: bool,
        /// Distribution directory containing packages/.
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Show one method, or the complete catalog when no resource is named.
    Show {
        /// Package-qualified resource, for example pale.design:animate.
        resource: Option<String>,
        /// Distribution directory containing packages/.
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
}

/// Agent whose instruction-file layout the command will use.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(super) enum GuidanceHostArg {
    /// Codex skills and a proposed AGENTS.md contribution.
    Codex,
    /// Claude Code skills and a proposed shared AGENTS.md contribution.
    ClaudeCode,
}

impl From<GuidanceHostArg> for pale_sdk::SourceGuidanceHost {
    fn from(value: GuidanceHostArg) -> Self {
        match value {
            GuidanceHostArg::Codex => Self::Codex,
            GuidanceHostArg::ClaudeCode => Self::ClaudeCode,
        }
    }
}

/// Work with Package instructions without activating the resident.
#[derive(Subcommand)]
pub(super) enum GuidanceCommand {
    /// Print private repository scope and source reading order; writes no files.
    Context {
        /// Repository ID in the selected workspace configuration.
        #[arg(long)]
        repository: String,
    },
    /// Preview Package instructions and the proposed repository entry text.
    Inspect {
        /// Repository ID in the selected workspace configuration.
        #[arg(long)]
        repository: String,
        /// Agent to prepare instructions for; Pale does not detect it automatically.
        #[arg(long, value_enum)]
        host: GuidanceHostArg,
    },
    /// Show file changes and text differences, then remember the proposal for apply.
    Plan {
        /// Repository ID in the selected workspace configuration.
        #[arg(long)]
        repository: String,
        /// Agent whose instruction files to update; Pale does not detect it automatically.
        #[arg(long, value_enum)]
        host: GuidanceHostArg,
        /// Update instructions or remove only files Pale manages for this agent.
        #[arg(long, value_enum, default_value = "apply")]
        intent: GuidanceIntentArg,
    },
    /// Recheck current inputs and apply only the instruction changes you reviewed.
    Apply {
        /// Repository ID in the selected workspace configuration.
        #[arg(long)]
        repository: String,
        /// Agent whose instruction files to update.
        #[arg(long, value_enum)]
        host: GuidanceHostArg,
        /// Update or remove the instructions Pale manages for this agent.
        #[arg(long, value_enum, default_value = "apply")]
        intent: GuidanceIntentArg,
        /// Explicit plan digest for automation; otherwise use the last displayed plan.
        #[arg(long)]
        expect_plan: Option<String>,
    },
    /// Review or carry out recovery after an interrupted instruction update.
    Recovery {
        /// Plan the recovery or apply a recovery you have reviewed.
        #[command(subcommand)]
        action: GuidanceRecoveryCommand,
    },
}

/// Recovery choice the caller must make before Pale changes files.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(super) enum GuidanceRecoveryIntentArg {
    /// Restore the files recorded before the write, preserving other changes.
    RestoreBefore,
    /// Keep the matching completed write, record its completion and clean up.
    FinalizePublished,
}
impl From<GuidanceRecoveryIntentArg> for pale_sdk::LocalSourceGuidanceRecoveryIntent {
    fn from(value: GuidanceRecoveryIntentArg) -> Self {
        match value {
            GuidanceRecoveryIntentArg::RestoreBefore => Self::RestoreBefore,
            GuidanceRecoveryIntentArg::FinalizePublished => Self::FinalizePublished,
        }
    }
}

/// Review and apply instruction-update recovery through the shared SDK.
#[derive(Subcommand)]
pub(super) enum GuidanceRecoveryCommand {
    /// Inspect unfinished recovery without writing files or creating a lock.
    Plan {
        /// Repository ID in the selected workspace configuration.
        #[arg(long)]
        repository: String,
        /// Agent whose instruction update was interrupted.
        #[arg(long, value_enum)]
        host: GuidanceHostArg,
        /// Recovery choice to review.
        #[arg(long, value_enum)]
        intent: GuidanceRecoveryIntentArg,
    },
    /// Recheck current files and apply only the recovery you reviewed.
    Apply {
        /// Repository ID in the selected workspace configuration.
        #[arg(long)]
        repository: String,
        /// Agent whose instruction update was interrupted.
        #[arg(long, value_enum)]
        host: GuidanceHostArg,
        /// Reviewed recovery decision.
        #[arg(long, value_enum)]
        intent: GuidanceRecoveryIntentArg,
        /// Digest from the reviewed recovery plan; an instruction-update digest will not work.
        #[arg(long)]
        expect_recovery: String,
    },
}

/// Choice to update or remove Package instruction files.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(super) enum GuidanceIntentArg {
    /// Update selected instructions and remove replaced files Pale manages.
    Apply,
    /// Remove only the instructions Pale manages for this agent.
    Remove,
}

impl From<GuidanceIntentArg> for pale_sdk::LocalSourceGuidanceIntent {
    fn from(value: GuidanceIntentArg) -> Self {
        match value {
            GuidanceIntentArg::Apply => Self::Apply,
            GuidanceIntentArg::Remove => Self::Remove,
        }
    }
}

/// Resident commands available only through an injected attachment.
#[derive(Subcommand)]
pub(super) enum ResidentCommand {
    /// Read the current signed records for one retained operation.
    Observe(ResidentLocatedRequestArgs),
    /// Start or advance an Invoke.
    Invoke {
        /// Invoke step.
        #[command(subcommand)]
        step: ResidentInvokeCommand,
    },
    /// Start or advance a Consult.
    Consult {
        /// Consult step.
        #[command(subcommand)]
        step: ResidentConsultCommand,
    },
    /// Start or advance a Delegate.
    Delegate {
        /// Delegate step.
        #[command(subcommand)]
        step: ResidentDelegateCommand,
    },
    /// Send Cancel without claiming that the operation is terminal.
    Cancel(ResidentLocatedRequestArgs),
}

/// Resident commands available through a read-only attachment.
#[derive(Subcommand)]
pub(super) enum ResidentObservationCommand {
    /// Read the current signed records for one retained operation.
    Observe(ResidentLocatedRequestArgs),
}

/// Invoke steps at the command-line boundary.
#[derive(Subcommand)]
pub(super) enum ResidentInvokeCommand {
    /// Create one durable root Invoke before dispatch.
    Start(ResidentSignedRequestArgs),
    /// Advance one retained Invoke by at most one bounded turn.
    Advance(ResidentLocatedRequestArgs),
}

/// Consult steps at the command-line boundary.
#[derive(Subcommand)]
pub(super) enum ResidentConsultCommand {
    /// Create one durable Consult from an already selected roster.
    Start(ResidentSignedRequestArgs),
    /// Advance one retained Consult by at most one bounded turn.
    Advance(ResidentLocatedRequestArgs),
}

/// Delegate steps at the command-line boundary.
#[derive(Subcommand)]
pub(super) enum ResidentDelegateCommand {
    /// Create one durable child under its exact parent operation.
    Start(ResidentDelegateStartArgs),
    /// Advance one retained Delegate by at most one bounded turn.
    Advance(ResidentLocatedRequestArgs),
}

/// One pre-signed common request read from an explicit file.
#[derive(Args)]
pub(super) struct ResidentSignedRequestArgs {
    /// File containing one canonical, already signed common-request envelope.
    #[arg(long)]
    pub(super) signed_request: PathBuf,
}

/// One retained operation coordinate and its pre-signed request.
#[derive(Args)]
pub(super) struct ResidentLocatedRequestArgs {
    /// Immutable operation-handle record reference.
    #[arg(long)]
    pub(super) operation_handle_record_ref: String,
    /// SHA-256 of the unsigned canonical operation handle.
    #[arg(long)]
    pub(super) operation_handle_sha256: String,
    /// File containing one canonical, already signed common-request envelope.
    #[arg(long)]
    pub(super) signed_request: PathBuf,
}

/// Exact parent coordinate and pre-signed request for a new Delegate child.
#[derive(Args)]
pub(super) struct ResidentDelegateStartArgs {
    /// Immutable parent operation-handle record reference.
    #[arg(long)]
    pub(super) parent_operation_handle_record_ref: String,
    /// SHA-256 of the unsigned canonical parent operation handle.
    #[arg(long)]
    pub(super) parent_operation_handle_sha256: String,
    /// File containing one canonical, already signed common-request envelope.
    #[arg(long)]
    pub(super) signed_request: PathBuf,
}

/// Explicit effect-free configuration-review actions.
#[derive(Clone, Copy, Debug, Subcommand)]
pub(super) enum ConfigurationReviewAction {
    /// Compile both configurations and print the preview summary as JSON.
    Preview,
    /// Inspect one impact page after checking the original preview summary from stdin.
    Impacts {
        /// Zero-based offset in the SDK's canonical impact order.
        #[arg(long, default_value_t = 0)]
        offset: u32,
        /// Zero selects the SDK default; the SDK caps larger page sizes.
        #[arg(long, default_value_t = 0)]
        limit: u16,
    },
    /// Accept the reviewed candidate; this command does not write configuration.
    Accept,
    /// Reject the reviewed candidate.
    Reject,
    /// Record cancellation of this review, not of a running operation.
    Cancel,
    /// Record separately established source, editor or conversation drift.
    Stale,
}

/// Exact host and scope coordinates for an effect-free control-store inspection.
#[derive(Args)]
pub(super) struct ControlStoreInspectionArgs {
    /// Absolute private runtime-state root; missing descendants are not created.
    #[arg(long)]
    pub(super) runtime_state_root: PathBuf,
    /// Expected logical identity of the physical store.
    #[arg(long)]
    pub(super) logical_store_id: String,
    /// Expected control-store incarnation, independent of process lifetime.
    #[arg(long)]
    pub(super) control_incarnation_id: String,
    /// Principal trust or deployment control scope, with its own exact coordinates.
    #[command(subcommand)]
    pub(super) scope: ControlStoreInspectionScope,
}

/// Distinct scope-specific command-line products; neither implies the other.
#[derive(Subcommand)]
pub(super) enum ControlStoreInspectionScope {
    /// Inspect the exact principal-scoped trust store.
    Principal {
        /// Expected resident principal identity.
        #[arg(long)]
        principal_id: String,
    },
    /// Inspect the exact immutable-deployment-scoped control store.
    Deployment {
        /// Expected resident workspace identity.
        #[arg(long)]
        workspace_id: String,
        /// Exact deployment-binding reference.
        #[arg(long)]
        deployment_binding_ref: String,
        /// SHA-256 bound to that deployment-binding reference.
        #[arg(long)]
        deployment_binding_sha256: String,
    },
}

#[cfg(test)]
#[path = "../tests/internal/cli.rs"]
mod tests;
