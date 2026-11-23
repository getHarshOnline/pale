//! Exhaustive process-group termination and direct-child reaping.

use std::time::{Duration, Instant};

use pale_ports::{
    ProcessTerminationIssue as TerminationIssue, ProcessTerminationMode as TerminationMode,
    ProcessTerminationPolicy as TerminationPolicy, ProcessTerminationStage as TerminationStage,
    ProcessTerminationUncertainty, SupervisedProcessExit,
};

/// Poll interval for bounded cleanup observation.
const POLL_INTERVAL: Duration = Duration::from_millis(5);

#[cfg(unix)]
#[derive(Clone, Copy)]
/// Process-group signal selected by the cleanup state machine.
enum GroupSignal {
    Terminate,
    Kill,
}

#[cfg(unix)]
#[derive(Clone, Copy)]
/// Whether the selected process group existed when signaling was attempted.
enum GroupSignalOutcome {
    Delivered,
    Absent,
}

/// Non-reaping direct-child state used while the group coordinate is signal-safe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DirectChildObservation {
    Running,
    Terminal(SupervisedProcessExit),
    /// The child is no longer waitable, so its numeric group coordinate may be reused.
    Reaped,
}

#[cfg(all(
    unix,
    not(any(
        target_os = "cygwin",
        target_os = "horizon",
        target_os = "openbsd",
        target_os = "redox",
        target_os = "wasi"
    ))
))]
pub(crate) fn observe_direct_child(
    child: &std::process::Child,
) -> std::io::Result<DirectChildObservation> {
    use rustix::io::Errno;
    use rustix::process::{waitid, Pid, WaitId, WaitIdOptions};

    let options = WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT;
    let status = match waitid(WaitId::Pid(Pid::from_child(child)), options) {
        Ok(None) => return Ok(DirectChildObservation::Running),
        Ok(Some(status)) => status,
        Err(Errno::CHILD) => return Ok(DirectChildObservation::Reaped),
        Err(source) => {
            return Err(std::io::Error::from_raw_os_error(source.raw_os_error()));
        }
    };
    if let Some(code) = status.exit_status() {
        return Ok(DirectChildObservation::Terminal(if code == 0 {
            SupervisedProcessExit::Success
        } else {
            SupervisedProcessExit::Failure { code: Some(code) }
        }));
    }
    if status.terminating_signal().is_some() {
        return Ok(DirectChildObservation::Terminal(
            SupervisedProcessExit::Failure { code: None },
        ));
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "waitid returned a non-terminal state for an exit-only observation",
    ))
}

#[cfg(all(
    unix,
    any(
        target_os = "cygwin",
        target_os = "horizon",
        target_os = "openbsd",
        target_os = "redox",
        target_os = "wasi"
    )
))]
pub(crate) fn observe_direct_child(
    _child: &std::process::Child,
) -> std::io::Result<DirectChildObservation> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "non-reaping direct-child observation is unsupported on this target",
    ))
}

#[cfg(not(unix))]
pub(crate) fn observe_direct_child(
    child: &mut std::process::Child,
) -> std::io::Result<DirectChildObservation> {
    Ok(match child.try_wait()? {
        Some(status) if status.success() => {
            DirectChildObservation::Terminal(SupervisedProcessExit::Success)
        }
        Some(status) => DirectChildObservation::Terminal(SupervisedProcessExit::Failure {
            code: status.code(),
        }),
        None => DirectChildObservation::Running,
    })
}

#[cfg(unix)]
/// Minimal process operations required by the termination state machine.
///
/// Production implements this for [`std::process::Child`]; tests substitute a
/// deterministic target to exercise every fallback and uncertainty branch.
trait TerminationTarget {
    /// Observe terminal state without reaping or surrendering group identity.
    fn observe_direct_child(&mut self) -> std::io::Result<DirectChildObservation>;
    fn try_reap_direct_child(&mut self) -> std::io::Result<bool>;
    fn signal_group(&mut self, signal: GroupSignal) -> std::io::Result<GroupSignalOutcome>;
    fn group_is_absent(&mut self) -> std::io::Result<bool>;
    fn kill_direct_child(&mut self) -> std::io::Result<()>;
}

#[cfg(unix)]
impl TerminationTarget for std::process::Child {
    fn observe_direct_child(&mut self) -> std::io::Result<DirectChildObservation> {
        observe_direct_child(&*self)
    }

    fn try_reap_direct_child(&mut self) -> std::io::Result<bool> {
        self.try_wait().map(|status| status.is_some())
    }

    fn signal_group(&mut self, signal: GroupSignal) -> std::io::Result<GroupSignalOutcome> {
        use rustix::io::Errno;
        use rustix::process::{kill_process_group, Pid, Signal};

        let signal = match signal {
            GroupSignal::Terminate => Signal::TERM,
            GroupSignal::Kill => Signal::KILL,
        };
        match kill_process_group(Pid::from_child(&*self), signal) {
            Ok(()) => Ok(GroupSignalOutcome::Delivered),
            Err(Errno::SRCH) => Ok(GroupSignalOutcome::Absent),
            Err(error) => Err(std::io::Error::from_raw_os_error(error.raw_os_error())),
        }
    }

    fn group_is_absent(&mut self) -> std::io::Result<bool> {
        use rustix::io::Errno;
        use rustix::process::{test_kill_process_group, Pid};

        match test_kill_process_group(Pid::from_child(&*self)) {
            Ok(()) => Ok(false),
            Err(Errno::SRCH) => Ok(true),
            Err(error) => Err(std::io::Error::from_raw_os_error(error.raw_os_error())),
        }
    }

    fn kill_direct_child(&mut self) -> std::io::Result<()> {
        self.kill()
    }
}

#[cfg(unix)]
fn issue(issues: &mut Vec<TerminationIssue>, stage: TerminationStage, result: std::io::Result<()>) {
    if let Err(source) = result {
        issues.push(TerminationIssue::new(stage, source));
    }
}

#[cfg(unix)]
fn signal_group(
    issues: &mut Vec<TerminationIssue>,
    stage: TerminationStage,
    result: std::io::Result<GroupSignalOutcome>,
) -> Option<GroupSignalOutcome> {
    match result {
        Ok(outcome) => Some(outcome),
        Err(source) => {
            issues.push(TerminationIssue::new(stage, source));
            None
        }
    }
}

#[cfg(unix)]
fn reap_until_deadline(
    target: &mut impl TerminationTarget,
    issues: &mut Vec<TerminationIssue>,
    deadline: Instant,
) -> bool {
    loop {
        match target.try_reap_direct_child() {
            Ok(true) => return true,
            Ok(false) if Instant::now() < deadline => {
                std::thread::sleep(
                    POLL_INTERVAL.min(deadline.saturating_duration_since(Instant::now())),
                );
            }
            Ok(false) => {
                issues.push(TerminationIssue::new(
                    TerminationStage::DirectChildReap,
                    std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "direct child did not exit within the bounded reap window",
                    ),
                ));
                return false;
            }
            Err(source) => {
                issues.push(TerminationIssue::new(
                    TerminationStage::DirectChildReap,
                    source,
                ));
                return false;
            }
        }
    }
}

#[cfg(unix)]
fn prove_group_absent(
    target: &mut impl TerminationTarget,
    issues: &mut Vec<TerminationIssue>,
    signal: Option<GroupSignalOutcome>,
    deadline: Instant,
) -> bool {
    if matches!(signal, Some(GroupSignalOutcome::Absent)) {
        return true;
    }
    loop {
        match target.group_is_absent() {
            Ok(true) => return true,
            Ok(false) if Instant::now() < deadline => {
                std::thread::sleep(
                    POLL_INTERVAL.min(deadline.saturating_duration_since(Instant::now())),
                );
            }
            Ok(false) => {
                issues.push(TerminationIssue::new(
                    TerminationStage::ForcedGroupExit,
                    std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "process group remained after forced termination",
                    ),
                ));
                return false;
            }
            Err(source) => {
                issues.push(TerminationIssue::new(
                    TerminationStage::ForcedGroupExit,
                    source,
                ));
                return false;
            }
        }
    }
}

#[cfg(unix)]
fn terminate_target(
    target: &mut impl TerminationTarget,
    policy: TerminationPolicy,
) -> Result<SupervisedProcessExit, ProcessTerminationUncertainty> {
    let started = Instant::now();
    let cleanup_deadline = started
        .checked_add(policy.cleanup_time())
        .unwrap_or(started);
    let grace_deadline = started
        .checked_add(policy.cooperative_grace())
        .unwrap_or(started)
        .min(cleanup_deadline);
    let mut issues = Vec::new();
    let initial = match target.observe_direct_child() {
        Ok(observation) => observation,
        Err(source) => {
            issues.push(TerminationIssue::new(
                TerminationStage::InitialDirectChildObservation,
                source,
            ));
            return Err(ProcessTerminationUncertainty::both_unproven(issues));
        }
    };
    let completed_exit = match initial {
        DirectChildObservation::Terminal(exit) => exit,
        DirectChildObservation::Running => SupervisedProcessExit::Terminated,
        DirectChildObservation::Reaped => {
            // Reaping releases the numeric PID/PGID for reuse. Without a
            // stronger retained host identity, even a best-effort group signal
            // is unsafe.
            return Err(ProcessTerminationUncertainty::process_group_absence_unproven(issues));
        }
    };

    if policy.mode() == TerminationMode::GracefulThenForced {
        let graceful_sent = signal_group(
            &mut issues,
            TerminationStage::GracefulGroupSignal,
            target.signal_group(GroupSignal::Terminate),
        );
        if matches!(graceful_sent, Some(GroupSignalOutcome::Delivered)) {
            while Instant::now() < grace_deadline {
                match target.observe_direct_child() {
                    Ok(DirectChildObservation::Terminal(_)) => break,
                    Ok(DirectChildObservation::Running) => std::thread::sleep(
                        POLL_INTERVAL.min(grace_deadline.saturating_duration_since(Instant::now())),
                    ),
                    Ok(DirectChildObservation::Reaped) => {
                        return Err(
                            ProcessTerminationUncertainty::process_group_absence_unproven(issues),
                        );
                    }
                    Err(source) => {
                        issues.push(TerminationIssue::new(
                            TerminationStage::GraceDirectChildObservation,
                            source,
                        ));
                        return Err(ProcessTerminationUncertainty::both_unproven(issues));
                    }
                }
            }
        }
    }

    let forced_signal = signal_group(
        &mut issues,
        TerminationStage::ForcedGroupSignal,
        target.signal_group(GroupSignal::Kill),
    );
    if initial == DirectChildObservation::Running {
        issue(
            &mut issues,
            TerminationStage::DirectChildKill,
            target.kill_direct_child(),
        );
    }
    let direct_child_reaped = reap_until_deadline(target, &mut issues, cleanup_deadline);
    // This observation is deliberately non-signaling. No group signal may be
    // attempted after `direct_child_reaped` releases the numeric coordinate.
    let process_group_absent =
        prove_group_absent(target, &mut issues, forced_signal, cleanup_deadline);

    if process_group_absent && direct_child_reaped {
        return Ok(completed_exit);
    }
    match (process_group_absent, direct_child_reaped) {
        (false, true) => Err(ProcessTerminationUncertainty::process_group_absence_unproven(issues)),
        (true, false) => Err(ProcessTerminationUncertainty::direct_child_unreaped(issues)),
        (false, false) => Err(ProcessTerminationUncertainty::both_unproven(issues)),
        (true, true) => Ok(completed_exit),
    }
}

#[cfg(unix)]
/// Exhaust process-group and direct-child fallbacks, returning only after the
/// child is reaped or the remaining uncertainty has been recorded.
///
/// A successful return preserves an exit observed before cleanup began;
/// otherwise it reports [`SupervisedProcessExit::Terminated`].
///
/// # Errors
///
/// Returns [`ProcessTerminationUncertainty`] when bounded cleanup cannot prove
/// both direct-child reaping and process-group absence. Delivering a signal is
/// not treated as proof that the target exited.
pub(crate) fn terminate_and_reap(
    child: &mut std::process::Child,
    policy: TerminationPolicy,
) -> Result<SupervisedProcessExit, ProcessTerminationUncertainty> {
    terminate_target(child, policy)
}

#[cfg(not(unix))]
/// Exhaust available direct-child fallbacks, returning only after the child is
/// reaped or the remaining uncertainty has been recorded.
///
/// # Errors
///
/// Always returns [`ProcessTerminationUncertainty`] on platforms without a
/// process-group primitive because direct-child reaping cannot prove descendant
/// absence. The retained variant records whether direct-child reaping was also
/// unproved.
pub(crate) fn terminate_and_reap(
    child: &mut std::process::Child,
    policy: TerminationPolicy,
) -> Result<SupervisedProcessExit, ProcessTerminationUncertainty> {
    let mut issues = Vec::new();
    if let Err(source) = child.kill() {
        issues.push(TerminationIssue::new(
            TerminationStage::DirectChildKill,
            source,
        ));
    }
    let direct_child_reaped = {
        let deadline = Instant::now()
            .checked_add(policy.cleanup_time())
            .unwrap_or_else(Instant::now);
        let mut reaped = false;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => {
                    reaped = true;
                    break;
                }
                Ok(None) if Instant::now() < deadline => std::thread::sleep(POLL_INTERVAL),
                Ok(None) => {
                    issues.push(TerminationIssue::new(
                        TerminationStage::DirectChildReap,
                        std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            "direct child did not exit within the bounded reap window",
                        ),
                    ));
                    break;
                }
                Err(source) => {
                    issues.push(TerminationIssue::new(
                        TerminationStage::DirectChildReap,
                        source,
                    ));
                    break;
                }
            }
        }
        reaped
    };
    // Without a platform process-group primitive, reaping the direct child
    // cannot prove that descendants are gone.
    if direct_child_reaped {
        Err(ProcessTerminationUncertainty::process_group_absence_unproven(issues))
    } else {
        Err(ProcessTerminationUncertainty::both_unproven(issues))
    }
}

#[cfg(all(test, unix))]
#[path = "../tests/internal/termination.rs"]
mod tests;
