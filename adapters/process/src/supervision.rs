//! Direct-child mechanics for long-lived supervised processes.
//!
//! The adapter retains the operating-system child handle, creates an isolated
//! process group on Unix, and uses the existing executable-identity and bounded
//! termination implementations. It does not decide desired state, restart,
//! readiness, health, generation, fencing, or adoption.

use std::num::NonZeroU32;
#[cfg(target_os = "linux")]
use std::num::NonZeroU64;
use std::process::{Command, Stdio};

use pale_ports::{
    DirectChildProcessIdentity, ExecutableIdentityPort, ExecutableIdentityRequest,
    ExecutableObservation, HostProcessGroupId, HostProcessId, HostProcessStartObservation,
    ProcessTerminationPolicy, ProcessTerminationUncertainty, SupervisedProcess,
    SupervisedProcessExit, SupervisedProcessObservation, SupervisedProcessObservationError,
    SupervisedProcessPort, SupervisedProcessRequest, SupervisedProcessStartError,
};

use crate::executable::VerifiedExecutable;
use crate::probe::LocalProcessProbe;
use crate::termination::{observe_direct_child, terminate_and_reap, DirectChildObservation};

/// Polling and termination capability for one directly spawned child.
///
/// The concrete child handle is intentionally private. In particular, callers
/// cannot reconstruct this capability from [`HostProcessId`] after a restart.
struct LocalSupervisedProcess {
    identity: DirectChildProcessIdentity,
    child: std::process::Child,
    terminal: Option<SupervisedProcessExit>,
    cleanup_complete: bool,
    cleanup_retry_safe: bool,
    fallback_termination: ProcessTerminationPolicy,
}

impl SupervisedProcess for LocalSupervisedProcess {
    fn identity(&self) -> &DirectChildProcessIdentity {
        &self.identity
    }

    fn observe(
        &mut self,
    ) -> Result<SupervisedProcessObservation, SupervisedProcessObservationError> {
        if let Some(terminal) = self.terminal {
            return Ok(SupervisedProcessObservation::Exited(terminal));
        }
        #[cfg(unix)]
        let observation = observe_direct_child(&self.child);
        #[cfg(not(unix))]
        let observation = observe_direct_child(&mut self.child);
        Ok(
            match observation.map_err(SupervisedProcessObservationError::Wait)? {
                DirectChildObservation::Terminal(terminal) => {
                    self.terminal = Some(terminal);
                    SupervisedProcessObservation::Exited(terminal)
                }
                DirectChildObservation::Running => SupervisedProcessObservation::Running,
                DirectChildObservation::Reaped => {
                    self.cleanup_retry_safe = false;
                    return Err(SupervisedProcessObservationError::Wait(
                        std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            "direct child was reaped outside its retained supervisor",
                        ),
                    ));
                }
            },
        )
    }

    fn terminate(
        &mut self,
        policy: ProcessTerminationPolicy,
    ) -> Result<(), ProcessTerminationUncertainty> {
        if self.cleanup_complete {
            return Ok(());
        }
        match terminate_and_reap(&mut self.child, policy) {
            Ok(terminal) => {
                self.cleanup_complete = true;
                self.terminal.get_or_insert(terminal);
                Ok(())
            }
            Err(uncertainty) => {
                // Once the direct child is reaped, its numeric group coordinate
                // may eventually be reused. Never retry group signaling from
                // Drop without a stronger host-start identity check.
                self.cleanup_retry_safe = !uncertainty.direct_child_reaped();
                Err(uncertainty)
            }
        }
    }
}

impl Drop for LocalSupervisedProcess {
    fn drop(&mut self) {
        if !self.cleanup_complete && self.cleanup_retry_safe {
            let _ = terminate_and_reap(&mut self.child, self.fallback_termination);
        }
    }
}

#[cfg(target_os = "linux")]
fn host_start_observation(process_id: HostProcessId) -> HostProcessStartObservation {
    linux_start_ticks(process_id)
        .and_then(NonZeroU64::new)
        .map(pale_ports::HostProcessStartIdentity::new)
        .map_or(
            HostProcessStartObservation::Unavailable,
            HostProcessStartObservation::Observed,
        )
}

#[cfg(target_os = "linux")]
fn linux_start_ticks(process_id: HostProcessId) -> Option<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{}/stat", process_id.get())).ok()?;
    let fields = stat.rsplit_once(") ")?.1.split_ascii_whitespace();
    // The suffix begins at field 3 (`state`); Linux `starttime` is field 22.
    fields.skip(19).next()?.parse().ok()
}

#[cfg(not(target_os = "linux"))]
fn host_start_observation(_process_id: HostProcessId) -> HostProcessStartObservation {
    // The current safe standard-library/rustix surface has no portable process
    // start token. PID-only inference would make restart adoption unsound.
    HostProcessStartObservation::Unavailable
}

fn direct_child_identity(child: &std::process::Child) -> DirectChildProcessIdentity {
    let process_id = HostProcessId::new(
        NonZeroU32::new(child.id()).expect("a successfully spawned child has a non-zero PID"),
    );
    #[cfg(unix)]
    let process_group_id = Some(HostProcessGroupId::new(
        NonZeroU32::new(child.id()).expect("a successfully spawned child has a non-zero group ID"),
    ));
    #[cfg(not(unix))]
    let process_group_id = None;

    DirectChildProcessIdentity::new(
        process_id,
        process_group_id,
        host_start_observation(process_id),
    )
}

/// Local implementation of the [`SupervisedProcessPort`] boundary.
///
/// The unit value owns no lifecycle policy. Every launch receives exact
/// executable bytes, arguments, and environment from its admitted caller.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalProcessSupervisor;

impl ExecutableIdentityPort for LocalProcessSupervisor {
    fn verify_executable(
        &self,
        request: ExecutableIdentityRequest<'_>,
    ) -> Result<ExecutableObservation, pale_ports::ExecutableIdentityError> {
        LocalProcessProbe.verify_executable(request)
    }
}

impl SupervisedProcessPort for LocalProcessSupervisor {
    fn spawn(
        &self,
        request: SupervisedProcessRequest<'_>,
    ) -> Result<Box<dyn SupervisedProcess>, SupervisedProcessStartError> {
        let mut executable =
            VerifiedExecutable::admit(request.executable, request.expected_digest)?;
        executable.revalidate()?;

        let mut command = Command::new(executable.path());
        command
            .args(request.arguments)
            .env_clear()
            .envs(
                request
                    .environment
                    .iter()
                    .map(|entry| (entry.name, entry.value)),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }

        let mut child = command
            .spawn()
            .map_err(SupervisedProcessStartError::Spawn)?;
        if let Err(identity) = executable.revalidate() {
            let cleanup = terminate_and_reap(&mut child, request.fallback_termination).err();
            return Err(SupervisedProcessStartError::PostSpawnIdentity { identity, cleanup });
        }
        let identity = direct_child_identity(&child);
        Ok(Box::new(LocalSupervisedProcess {
            identity,
            child,
            terminal: None,
            cleanup_complete: false,
            cleanup_retry_safe: true,
            fallback_termination: request.fallback_termination,
        }))
    }
}
