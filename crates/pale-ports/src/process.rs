//! Outbound boundaries for retained executable identity and local processes.
//!
//! [`ProcessProbePort`] admits exact executable bytes before and after a
//! shell-free identity command and enforces post-spawn supervision and output
//! ceilings. [`SupervisedProcessPort`] retains a direct-child capability for a
//! long-lived process without turning its PID into an adoption handle. Both
//! boundaries return observations only; application policy interprets them.

use pale_core::records::ExecutableDigest;
use std::fmt;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

/// Resource ceilings that a [`ProcessProbePort`] must enforce for one probe.
///
/// A zero byte limit admits only an empty stream. A zero wall-time limit does
/// not skip execution; it makes the first incomplete observation time out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeLimits {
    /// Post-spawn supervision ceiling in milliseconds.
    pub wall_time_ms: u64,
    /// Maximum complete standard-output bytes.
    pub stdout_bytes: usize,
    /// Maximum complete standard-error bytes.
    pub stderr_bytes: usize,
    /// Maximum time spent containing and reaping a failed probe.
    pub cleanup_time_ms: u64,
}

impl ProbeLimits {
    /// Limits selected by the current workspace identity-verification use case.
    ///
    /// Callers with another admitted policy may construct [`ProbeLimits`]
    /// directly; this is a use-case default, not adapter-owned policy.
    #[must_use]
    pub const fn workspace_identity() -> Self {
        Self {
            wall_time_ms: 5_000,
            stdout_bytes: 64 * 1024,
            stderr_bytes: 16 * 1024,
            cleanup_time_ms: 5_000,
        }
    }
}

/// Exact executable and argument submitted for one bounded identity probe.
#[derive(Debug, Clone, Copy)]
pub struct IdentityProbeRequest<'a> {
    /// Absolute machine-bound executable path.
    pub executable: &'a str,
    /// Exact digest expected before and after the probe.
    pub expected_digest: &'a ExecutableDigest,
    /// One literal identity argument; no shell interpretation is permitted.
    pub argument: &'a str,
    /// Explicit resource ceiling selected by application policy.
    pub limits: ProbeLimits,
}

/// Exact executable submitted for identity-only verification.
#[derive(Debug, Clone, Copy)]
pub struct ExecutableIdentityRequest<'a> {
    /// Absolute machine-bound executable path.
    pub executable: &'a str,
    /// Exact digest expected from retained bytes.
    pub expected_digest: &'a ExecutableDigest,
}

/// Path identity observed after exact executable-byte verification.
///
/// Returning this value does not retain an operating-system handle or authorize
/// a later launch. A caller crossing another process boundary must verify the
/// executable again through [`ExecutableIdentityPort::verify_executable`] or a
/// stronger operation-specific contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableObservation {
    /// Canonical path that named the verified executable during observation.
    pub canonical_path: PathBuf,
}

/// Non-zero host process identifier observed for a directly spawned child.
///
/// This coordinate is diagnostic and process-local. It is never sufficient to
/// adopt an existing process because hosts may reuse it after the child exits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HostProcessId(std::num::NonZeroU32);

impl HostProcessId {
    /// Construct an identifier from the host's non-zero process coordinate.
    #[must_use]
    pub const fn new(value: std::num::NonZeroU32) -> Self {
        Self(value)
    }

    /// Return the host-native numeric coordinate.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }
}

/// Non-zero process-group identifier assigned to one supervised child tree.
///
/// A group coordinate permits bounded descendant cleanup while the adapter
/// retains the direct-child handle. It is not process identity or adoption
/// evidence by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HostProcessGroupId(std::num::NonZeroU32);

impl HostProcessGroupId {
    /// Construct an identifier from the host's non-zero group coordinate.
    #[must_use]
    pub const fn new(value: std::num::NonZeroU32) -> Self {
        Self(value)
    }

    /// Return the host-native numeric coordinate.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0.get()
    }
}

/// Host-observed process start coordinate relative to the current boot.
///
/// The numeric value is meaningful only for equality on the same host boot;
/// its unit and rate are host-owned. Together with a process identifier it can
/// reject a reused identifier, but it does not replace the executable,
/// generation, configuration, or fence checks required by adoption policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HostProcessStartIdentity(std::num::NonZeroU64);

impl HostProcessStartIdentity {
    /// Retain one host-native boot-relative start coordinate.
    #[must_use]
    pub const fn new(boot_relative_ticks: std::num::NonZeroU64) -> Self {
        Self(boot_relative_ticks)
    }

    /// Return the opaque boot-relative coordinate.
    #[must_use]
    pub const fn boot_relative_ticks(self) -> u64 {
        self.0.get()
    }
}

/// Whether the adapter proved a host-native start identity for the child.
///
/// [`Self::Unavailable`] is a first-class uncertainty, not permission to fall
/// back to PID-only adoption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostProcessStartObservation {
    /// The host exposed a boot-relative process start coordinate.
    Observed(HostProcessStartIdentity),
    /// The target or observation path could not prove a start coordinate.
    Unavailable,
}

/// Exact process mechanics retained for one child spawned by Pale.
///
/// The direct-child operating-system handle remains inside the implementing
/// [`SupervisedProcess`] object. This projection deliberately contains more
/// than a PID, yet still is observation rather than restart-adoption authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectChildProcessIdentity {
    process_id: HostProcessId,
    process_group_id: Option<HostProcessGroupId>,
    start: HostProcessStartObservation,
}

impl DirectChildProcessIdentity {
    /// Construct the observation retained by a concrete process adapter.
    #[must_use]
    pub const fn new(
        process_id: HostProcessId,
        process_group_id: Option<HostProcessGroupId>,
        start: HostProcessStartObservation,
    ) -> Self {
        Self {
            process_id,
            process_group_id,
            start,
        }
    }

    /// Return the diagnostic host PID.
    #[must_use]
    pub const fn process_id(&self) -> HostProcessId {
        self.process_id
    }

    /// Return the isolated process group when the host supports one.
    #[must_use]
    pub const fn process_group_id(&self) -> Option<HostProcessGroupId> {
        self.process_group_id
    }

    /// Return the exact or explicitly unavailable host-start observation.
    #[must_use]
    pub const fn start(&self) -> HostProcessStartObservation {
        self.start
    }
}

/// One literal environment entry for a supervised child.
///
/// The adapter clears ambient environment state before applying this bounded,
/// caller-admitted list. Names and values are passed without shell parsing.
#[derive(Clone, Copy)]
pub struct ProcessEnvironmentEntry<'a> {
    /// Exact environment-variable name.
    pub name: &'a str,
    /// Exact environment-variable value.
    pub value: &'a str,
}

impl fmt::Debug for ProcessEnvironmentEntry<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProcessEnvironmentEntry")
            .field("name", &self.name)
            .field("value", &"<redacted>")
            .finish()
    }
}

/// Exact executable invocation for one long-lived directly supervised child.
///
/// Arguments and environment entries are literal. Standard streams are closed
/// by the local adapter; a future selected logging boundary must be a separate
/// explicit port rather than an ambient inherited pipe.
#[derive(Clone, Copy)]
pub struct SupervisedProcessRequest<'a> {
    /// Absolute machine-bound executable path.
    pub executable: &'a str,
    /// Exact digest expected before and immediately after spawn.
    pub expected_digest: &'a ExecutableDigest,
    /// Literal ordered argument vector, excluding argument zero.
    pub arguments: &'a [&'a str],
    /// Literal environment entries applied after clearing ambient state.
    pub environment: &'a [ProcessEnvironmentEntry<'a>],
    /// Caller-owned fallback used after post-spawn admission failure or drop.
    pub fallback_termination: ProcessTerminationPolicy,
}

impl fmt::Debug for SupervisedProcessRequest<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SupervisedProcessRequest")
            .field("executable", &self.executable)
            .field("expected_digest", &self.expected_digest)
            .field("argument_count", &self.arguments.len())
            .field("environment_entry_count", &self.environment.len())
            .field("fallback_termination", &self.fallback_termination)
            .finish()
    }
}

/// Terminal state of a directly supervised child.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisedProcessExit {
    /// The child exited successfully without supervisor termination.
    Success,
    /// The child exited unsuccessfully; some hosts do not expose an exit code.
    Failure {
        /// Portable exit code when the host reported one.
        code: Option<i32>,
    },
    /// The child was reaped during supervisor-initiated termination.
    ///
    /// Process-group cleanup certainty is returned separately by
    /// [`SupervisedProcess::terminate`].
    Terminated,
}

/// Latest direct-child observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisedProcessObservation {
    /// The direct child has not reported a terminal status.
    Running,
    /// The direct child reported a terminal status, retained without surrendering
    /// the process identity required for safe descendant-group cleanup.
    Exited(SupervisedProcessExit),
}

/// Ordered process-group termination strategy selected by application policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessTerminationMode {
    /// Skip cooperative shutdown after an unrecoverable observation failure.
    Immediate,
    /// Offer a bounded cooperative grace period before forced cleanup.
    GracefulThenForced,
}

/// Caller-owned time and strategy ceilings for one cleanup attempt.
///
/// `cleanup_time` bounds the complete attempt. Under
/// [`ProcessTerminationMode::GracefulThenForced`], the cooperative window is
/// therefore `min(cooperative_grace, cleanup_time)`; forced cleanup uses only
/// the remaining time. The adapter owns polling mechanics, not either ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessTerminationPolicy {
    mode: ProcessTerminationMode,
    cooperative_grace: Duration,
    cleanup_time: Duration,
}

impl ProcessTerminationPolicy {
    /// Select immediate containment with one total cleanup-time ceiling.
    #[must_use]
    pub const fn immediate(cleanup_time: Duration) -> Self {
        Self {
            mode: ProcessTerminationMode::Immediate,
            cooperative_grace: Duration::ZERO,
            cleanup_time,
        }
    }

    /// Select cooperative group termination followed by forced containment.
    #[must_use]
    pub const fn graceful_then_forced(cooperative_grace: Duration, cleanup_time: Duration) -> Self {
        Self {
            mode: ProcessTerminationMode::GracefulThenForced,
            cooperative_grace,
            cleanup_time,
        }
    }

    /// Return the ordered termination strategy.
    #[must_use]
    pub const fn mode(self) -> ProcessTerminationMode {
        self.mode
    }

    /// Return the caller-selected cooperative window.
    #[must_use]
    pub const fn cooperative_grace(self) -> Duration {
        self.cooperative_grace
    }

    /// Return the caller-selected ceiling for the complete cleanup attempt.
    #[must_use]
    pub const fn cleanup_time(self) -> Duration {
        self.cleanup_time
    }
}

/// Failure to start one directly supervised process.
#[derive(Debug, thiserror::Error)]
pub enum SupervisedProcessStartError {
    /// Executable bytes or filesystem identity were not admitted.
    #[error(transparent)]
    Executable(#[from] ExecutableIdentityError),
    /// The child could not be started.
    #[error("cannot start supervised process: {0}")]
    Spawn(#[source] io::Error),
    /// Executable identity changed across spawn and child containment was attempted.
    #[error("executable identity changed across supervised-process spawn: {identity}")]
    PostSpawnIdentity {
        /// Exact executable-identity failure.
        #[source]
        identity: ExecutableIdentityError,
        /// Cleanup uncertainty when containment could not prove both terminal facts.
        cleanup: Option<ProcessTerminationUncertainty>,
    },
}

impl SupervisedProcessStartError {
    /// Return cleanup uncertainty retained after post-spawn identity drift.
    #[must_use]
    pub fn cleanup_uncertainty(&self) -> Option<&ProcessTerminationUncertainty> {
        match self {
            Self::PostSpawnIdentity { cleanup, .. } => cleanup.as_ref(),
            Self::Executable(_) | Self::Spawn(_) => None,
        }
    }
}

/// Failure to observe one retained direct-child handle.
#[derive(Debug, thiserror::Error)]
pub enum SupervisedProcessObservationError {
    /// The host could not inspect direct-child status.
    #[error("cannot inspect supervised process status: {0}")]
    Wait(#[source] io::Error),
}

/// Successful bounded process observation returned by [`ProcessProbePort::probe`].
///
/// [`Debug`](std::fmt::Debug) reports only the byte count so diagnostics do not
/// disclose the captured identity payload.
#[derive(Clone, PartialEq, Eq)]
pub struct ProbeObservation {
    /// Complete standard-output bytes within the declared ceiling.
    pub stdout: Vec<u8>,
}

impl fmt::Debug for ProbeObservation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProbeObservation")
            .field("stdout_bytes", &self.stdout.len())
            .finish()
    }
}

/// Captured child stream associated with a process failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeStream {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

impl std::fmt::Display for ProbeStream {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
        })
    }
}

/// Bounded cleanup stage whose failure left process termination uncertain.
///
/// The variants describe host-process mechanics, not lifecycle or native-effect
/// states. In particular, delivering a group signal does not by itself prove
/// [`ProcessTerminationUncertainty::process_group_absent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessTerminationStage {
    /// The first non-reaping direct-child observation failed.
    InitialDirectChildObservation,
    /// Cooperative process-group termination could not be requested.
    GracefulGroupSignal,
    /// Direct-child observation failed during the cooperative grace window.
    GraceDirectChildObservation,
    /// Forced process-group termination could not be requested.
    ForcedGroupSignal,
    /// Process-group absence could not be established after forced termination.
    ForcedGroupExit,
    /// The direct-child fallback kill failed.
    DirectChildKill,
    /// The direct child could not be reaped within the bounded cleanup window.
    DirectChildReap,
}

impl fmt::Display for ProcessTerminationStage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InitialDirectChildObservation => "initial-direct-child-observation",
            Self::GracefulGroupSignal => "process-group-term",
            Self::GraceDirectChildObservation => "grace-direct-child-observation",
            Self::ForcedGroupSignal => "process-group-kill",
            Self::ForcedGroupExit => "process-group-exit",
            Self::DirectChildKill => "direct-child-kill",
            Self::DirectChildReap => "direct-child-reap",
        })
    }
}

/// One operating-system failure retained at its exact cleanup stage.
#[derive(Debug)]
pub struct ProcessTerminationIssue {
    stage: ProcessTerminationStage,
    source: io::Error,
}

impl ProcessTerminationIssue {
    /// Retain one typed operating-system failure without flattening it to prose.
    #[must_use]
    pub fn new(stage: ProcessTerminationStage, source: io::Error) -> Self {
        Self { stage, source }
    }

    /// Return the exact cleanup stage that failed.
    #[must_use]
    pub const fn stage(&self) -> ProcessTerminationStage {
        self.stage
    }

    /// Return the retained operating-system failure.
    #[must_use]
    pub const fn source_error(&self) -> &io::Error {
        &self.source
    }
}

/// Exhausted process cleanup without proof of both required terminal facts.
///
/// Let `R` mean that the direct child was reaped and `A` that the process group
/// was proved absent. The exact success condition is rendered below; this enum
/// is the closed complement containing the other three proof states.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/process-cleanup-proof.display.mathml"
))]
///
/// The concrete adapter records at most one issue for each
/// [`ProcessTerminationStage`], so the retained diagnostic is bounded by the
/// closed stage inventory. This value is observation evidence only: it does not
/// establish process identity, writer fencing, liveness authority, or native
/// effect outcome.
#[derive(Debug)]
pub enum ProcessTerminationUncertainty {
    /// The direct child was reaped, but process-group absence was not proved.
    ProcessGroupAbsenceUnproven {
        /// Ordered failures observed while exhausting bounded cleanup.
        issues: Vec<ProcessTerminationIssue>,
    },
    /// Descendant-group absence was proved, but the direct child was not reaped.
    DirectChildUnreaped {
        /// Ordered failures observed while exhausting bounded cleanup.
        issues: Vec<ProcessTerminationIssue>,
    },
    /// Neither descendant-group absence nor direct-child reaping was proved.
    BothUnproven {
        /// Ordered failures observed while exhausting bounded cleanup.
        issues: Vec<ProcessTerminationIssue>,
    },
}

impl ProcessTerminationUncertainty {
    /// Retain failures after only descendant-group absence remained unproved.
    ///
    /// Adapter implementations should emit no more than one issue per closed
    /// [`ProcessTerminationStage`]. An empty list is valid when a platform has
    /// no process-group primitive; absence of an operating-system error still
    /// does not prove descendant termination.
    #[must_use]
    pub fn process_group_absence_unproven(issues: Vec<ProcessTerminationIssue>) -> Self {
        Self::ProcessGroupAbsenceUnproven { issues }
    }

    /// Retain failures after only direct-child reaping remained unproved.
    #[must_use]
    pub fn direct_child_unreaped(issues: Vec<ProcessTerminationIssue>) -> Self {
        Self::DirectChildUnreaped { issues }
    }

    /// Retain failures after neither required terminal fact could be proved.
    #[must_use]
    pub fn both_unproven(issues: Vec<ProcessTerminationIssue>) -> Self {
        Self::BothUnproven { issues }
    }

    /// Whether process-group absence was established after cleanup.
    #[must_use]
    pub const fn process_group_absent(&self) -> bool {
        matches!(self, Self::DirectChildUnreaped { .. })
    }

    /// Whether the direct child was conclusively reaped.
    #[must_use]
    pub const fn direct_child_reaped(&self) -> bool {
        matches!(self, Self::ProcessGroupAbsenceUnproven { .. })
    }

    /// Ordered operating-system failures retained from exhausted fallbacks.
    #[must_use]
    pub fn issues(&self) -> &[ProcessTerminationIssue] {
        match self {
            Self::ProcessGroupAbsenceUnproven { issues }
            | Self::DirectChildUnreaped { issues }
            | Self::BothUnproven { issues } => issues,
        }
    }
}

impl fmt::Display for ProcessTerminationUncertainty {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "process_group_absent={}, direct_child_reaped={}",
            self.process_group_absent(),
            self.direct_child_reaped()
        )?;
        for issue in self.issues() {
            write!(formatter, "; {}: {}", issue.stage, issue.source)?;
        }
        Ok(())
    }
}

impl std::error::Error for ProcessTerminationUncertainty {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.issues()
            .first()
            .map(|issue| &issue.source as &(dyn std::error::Error + 'static))
    }
}

/// Failure to retain and verify an executable's exact machine identity.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExecutableIdentityError {
    /// The binding does not identify an absolute path.
    #[error("executable path is not absolute: {0}")]
    NotAbsolute(String),
    /// The path is absent or cannot be retained without following its leaf.
    #[error("executable does not exist or cannot be opened: {0}")]
    NotFound(String),
    /// The leaf is a symbolic link.
    #[error("executable is a symlink: {0}")]
    Symlink(String),
    /// The retained leaf is not a regular file.
    #[error("executable is not a regular file: {0}")]
    NotRegular(String),
    /// The retained file lacks executable permissions.
    #[error("executable bit is not set: {0}")]
    NotExecutable(String),
    /// Exact retained bytes differ from the machine binding.
    #[error("executable sha256 {actual} does not match bound digest {expected}")]
    DigestMismatch {
        /// Digest observed from retained bytes.
        actual: ExecutableDigest,
        /// Digest selected by the binding.
        expected: ExecutableDigest,
    },
    /// Retained descriptor or path identity changed across an observation boundary.
    #[error("executable identity changed during admission: {0}")]
    IdentityChanged(String),
    /// The target cannot implement retained executable identity admission.
    #[error("executable identity observation is unsupported on `{platform}`")]
    UnsupportedPlatform {
        /// Rust target operating-system token.
        platform: &'static str,
    },
}

/// Failure of one bounded identity probe.
///
/// Operating-system failures remain typed [`io::Error`] sources. Callers may
/// inspect them through [`std::error::Error::source`] without parsing display
/// text; the display remains suitable for a bounded outer diagnostic layer.
#[derive(thiserror::Error)]
pub enum ProcessProbeError {
    /// Executable bytes or filesystem identity were not admitted.
    #[error(transparent)]
    Executable(#[from] ExecutableIdentityError),
    /// The child could not be started.
    #[error("cannot start probe: {0}")]
    Spawn(#[source] io::Error),
    /// A required captured pipe was unavailable after spawn.
    #[error("probe {stream} pipe is unavailable after spawn")]
    PipeUnavailable {
        /// Missing stream.
        stream: ProbeStream,
    },
    /// Child status could not be observed.
    #[error("cannot inspect probe status: {0}")]
    Wait(#[source] io::Error),
    /// The declared wall-time ceiling expired.
    #[error("probe exceeded its {millis} ms wall-time limit")]
    Timeout {
        /// Selected ceiling.
        millis: u128,
    },
    /// A captured stream exceeded its complete-byte ceiling.
    #[error("probe {stream} exceeded its {limit}-byte limit")]
    OutputTooLarge {
        /// Oversized stream.
        stream: ProbeStream,
        /// Selected byte ceiling.
        limit: usize,
    },
    /// Capturing a child stream failed.
    #[error("cannot capture probe {stream}: {source}")]
    Capture {
        /// Failed stream.
        stream: ProbeStream,
        /// Typed operating-system failure.
        #[source]
        source: io::Error,
    },
    /// Exhausted process-group and direct-child cleanup remained uncertain.
    #[error("cannot terminate or reap probe: {0}")]
    Terminate(#[source] ProcessTerminationUncertainty),
    /// The identity process returned a non-success exit.
    #[error("probe exited with code {code:?}; bounded stderr retained but redacted")]
    NonZero {
        /// Portable exit code when one was reported.
        code: Option<i32>,
        /// Bounded diagnostic suffix, empty or prefixed by `: `.
        stderr: String,
    },
}

impl fmt::Debug for ProcessProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Executable(error) => formatter.debug_tuple("Executable").field(error).finish(),
            Self::Spawn(source) => formatter.debug_tuple("Spawn").field(source).finish(),
            Self::PipeUnavailable { stream } => formatter
                .debug_struct("PipeUnavailable")
                .field("stream", stream)
                .finish(),
            Self::Wait(source) => formatter.debug_tuple("Wait").field(source).finish(),
            Self::Timeout { millis } => formatter
                .debug_struct("Timeout")
                .field("millis", millis)
                .finish(),
            Self::OutputTooLarge { stream, limit } => formatter
                .debug_struct("OutputTooLarge")
                .field("stream", stream)
                .field("limit", limit)
                .finish(),
            Self::Capture { stream, source } => formatter
                .debug_struct("Capture")
                .field("stream", stream)
                .field("source", source)
                .finish(),
            Self::Terminate(source) => formatter.debug_tuple("Terminate").field(source).finish(),
            Self::NonZero { code, stderr } => formatter
                .debug_struct("NonZero")
                .field("code", code)
                .field("stderr_bytes", &stderr.len())
                .finish(),
        }
    }
}

/// Outbound boundary for exact executable identity observations.
///
/// Implementations must verify retained bytes and pathname identity. This trait
/// does not launch the executable; launch-specific obligations belong to
/// [`ProcessProbePort`] or [`SupervisedProcessPort`].
pub trait ExecutableIdentityPort {
    /// Verify exact executable bytes and return their canonical path identity.
    ///
    /// # Errors
    ///
    /// Returns a typed failure when the path, file kind, permissions, digest,
    /// or retained identity cannot be proved.
    fn verify_executable(
        &self,
        request: ExecutableIdentityRequest<'_>,
    ) -> Result<ExecutableObservation, ExecutableIdentityError>;
}

/// Process probes extend exact executable identity with bounded launch mechanics.
///
/// Implementations must verify identity around launch, pass
/// [`IdentityProbeRequest::argument`] literally, clear ambient environment state,
/// enforce every [`ProbeLimits`] dimension, and on cleanup-required paths signal
/// the process group where supported and reap the direct child. An observed cleanup
/// failure returns [`ProcessProbeError::Terminate`]. A successful return proves the
/// direct child's successful exit and complete bounded output, not descendant
/// absence or semantic admission of that output.
pub trait ProcessProbePort: ExecutableIdentityPort {
    /// Perform one identity observation without shell interpretation.
    ///
    /// # Errors
    ///
    /// Returns a typed failure when executable identity, launch, capture,
    /// timeout, exit, or termination proof fails.
    fn probe(
        &self,
        request: IdentityProbeRequest<'_>,
    ) -> Result<ProbeObservation, ProcessProbeError>;
}

/// Owned direct-child capability returned by [`SupervisedProcessPort::spawn`].
///
/// Implementations retain the operating-system child handle and process-group
/// mechanics privately. Callers may observe or terminate only that retained
/// child; this boundary intentionally offers no PID-based adoption operation.
/// Dropping a live implementation may attempt best-effort containment, but an
/// explicit [`Self::terminate`] call is required to receive cleanup certainty.
pub trait SupervisedProcess: Send {
    /// Return the immutable host mechanics captured at spawn.
    fn identity(&self) -> &DirectChildProcessIdentity;

    /// Poll the retained direct-child handle without blocking for termination.
    ///
    /// # Errors
    ///
    /// Returns a typed host observation failure. The process may still be
    /// running when this method fails; callers must retain the handle and apply
    /// their selected recovery or termination policy.
    fn observe(
        &mut self,
    ) -> Result<SupervisedProcessObservation, SupervisedProcessObservationError>;

    /// Exhaust the selected process-group cleanup strategy and reap the child.
    ///
    /// `policy` owns the cooperative and total time ceilings. Once cleanup
    /// begins it is non-cancellable: returning early would surrender the only
    /// retained direct-child identity while its effect remained uncertain.
    /// After successful cleanup, repeated calls are idempotent. Observing the
    /// direct child's exit alone does not establish process-group cleanup.
    ///
    /// # Errors
    ///
    /// Returns [`ProcessTerminationUncertainty`] when cleanup cannot prove both
    /// direct-child reaping and process-group absence.
    fn terminate(
        &mut self,
        policy: ProcessTerminationPolicy,
    ) -> Result<(), ProcessTerminationUncertainty>;
}

/// Outbound boundary for exact, directly owned long-lived child mechanics.
///
/// Implementations re-use [`ExecutableIdentityPort`] admission, clear inherited
/// environment variables, pass the declared arguments and environment literally,
/// isolate a process group where supported, and retain the direct-child handle.
/// Successful spawn proves none of application readiness, generation/fence
/// admission, semantic health, or restart adoption.
pub trait SupervisedProcessPort: ExecutableIdentityPort {
    /// Start one exact executable and return its retained direct-child capability.
    ///
    /// # Errors
    ///
    /// Returns a typed failure when executable admission, spawn, post-spawn
    /// identity verification, or required containment fails.
    fn spawn(
        &self,
        request: SupervisedProcessRequest<'_>,
    ) -> Result<Box<dyn SupervisedProcess>, SupervisedProcessStartError>;
}

#[cfg(test)]
#[path = "../tests/internal/process.rs"]
mod tests;
