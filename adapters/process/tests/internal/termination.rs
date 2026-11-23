//! Exhaustive cleanup attempts when group signaling or direct-child killing fails.

use super::*;

#[derive(Default)]
struct FailingGroupTarget {
    calls: Vec<&'static str>,
    fail_direct_kill: bool,
    fail_reap: bool,
}

impl TerminationTarget for FailingGroupTarget {
    fn observe_direct_child(&mut self) -> std::io::Result<DirectChildObservation> {
        self.calls.push("observe");
        Ok(DirectChildObservation::Running)
    }

    fn try_reap_direct_child(&mut self) -> std::io::Result<bool> {
        self.calls.push("try-reap");
        if self.calls.contains(&"direct-kill") {
            if self.fail_reap {
                Err(std::io::Error::other("injected reap failure"))
            } else {
                Ok(true)
            }
        } else {
            Ok(false)
        }
    }

    fn signal_group(&mut self, signal: GroupSignal) -> std::io::Result<GroupSignalOutcome> {
        self.calls.push(match signal {
            GroupSignal::Terminate => "group-term",
            GroupSignal::Kill => "group-kill",
        });
        Err(std::io::Error::other("injected group signal failure"))
    }

    fn group_is_absent(&mut self) -> std::io::Result<bool> {
        self.calls.push("group-probe");
        Err(std::io::Error::other("injected group probe failure"))
    }

    fn kill_direct_child(&mut self) -> std::io::Result<()> {
        self.calls.push("direct-kill");
        if self.fail_direct_kill {
            Err(std::io::Error::other("injected direct kill failure"))
        } else {
            Ok(())
        }
    }
}

#[test]
fn group_signal_errors_still_force_and_reap_the_direct_child() {
    let mut target = FailingGroupTarget::default();
    let uncertainty = terminate_target(
        &mut target,
        TerminationPolicy::graceful_then_forced(Duration::ZERO, Duration::ZERO),
    )
    .unwrap_err();

    assert_eq!(
        target.calls,
        [
            "observe",
            "group-term",
            "group-kill",
            "direct-kill",
            "try-reap",
            "group-probe"
        ]
    );
    assert!(uncertainty.direct_child_reaped());
    assert!(!uncertainty.process_group_absent());
    assert!(uncertainty
        .issues()
        .iter()
        .any(|issue| issue.stage() == TerminationStage::GracefulGroupSignal));
    assert!(uncertainty
        .issues()
        .iter()
        .any(|issue| issue.stage() == TerminationStage::ForcedGroupSignal));
}

#[test]
fn direct_kill_failure_does_not_skip_the_reap_attempt() {
    let mut target = FailingGroupTarget {
        fail_direct_kill: true,
        fail_reap: true,
        ..FailingGroupTarget::default()
    };
    let uncertainty =
        terminate_target(&mut target, TerminationPolicy::immediate(Duration::ZERO)).unwrap_err();

    assert_eq!(
        target.calls,
        [
            "observe",
            "group-kill",
            "direct-kill",
            "try-reap",
            "group-probe"
        ]
    );
    assert!(!uncertainty.direct_child_reaped());
    assert!(uncertainty
        .issues()
        .iter()
        .any(|issue| issue.stage() == TerminationStage::DirectChildKill));
    assert!(uncertainty
        .issues()
        .iter()
        .any(|issue| issue.stage() == TerminationStage::DirectChildReap));
}

struct LingeringGroup;

impl TerminationTarget for LingeringGroup {
    fn observe_direct_child(&mut self) -> std::io::Result<DirectChildObservation> {
        Ok(DirectChildObservation::Terminal(
            SupervisedProcessExit::Success,
        ))
    }

    fn try_reap_direct_child(&mut self) -> std::io::Result<bool> {
        Ok(true)
    }

    fn signal_group(&mut self, _signal: GroupSignal) -> std::io::Result<GroupSignalOutcome> {
        Ok(GroupSignalOutcome::Delivered)
    }

    fn group_is_absent(&mut self) -> std::io::Result<bool> {
        Err(std::io::Error::other("group absence is unproven"))
    }

    fn kill_direct_child(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn delivered_kill_is_not_process_group_absence_evidence() {
    let uncertainty = terminate_target(
        &mut LingeringGroup,
        TerminationPolicy::immediate(Duration::ZERO),
    )
    .unwrap_err();

    assert!(uncertainty.direct_child_reaped());
    assert!(!uncertainty.process_group_absent());
    assert!(uncertainty
        .issues()
        .iter()
        .any(|issue| issue.stage() == TerminationStage::ForcedGroupExit));
}

struct AlreadyReapedTarget {
    group_signals: usize,
}

impl TerminationTarget for AlreadyReapedTarget {
    fn observe_direct_child(&mut self) -> std::io::Result<DirectChildObservation> {
        Ok(DirectChildObservation::Reaped)
    }

    fn try_reap_direct_child(&mut self) -> std::io::Result<bool> {
        Ok(true)
    }

    fn signal_group(&mut self, _signal: GroupSignal) -> std::io::Result<GroupSignalOutcome> {
        self.group_signals += 1;
        Ok(GroupSignalOutcome::Delivered)
    }

    fn group_is_absent(&mut self) -> std::io::Result<bool> {
        Ok(false)
    }

    fn kill_direct_child(&mut self) -> std::io::Result<()> {
        panic!("a reaped child must not receive a direct kill")
    }
}

#[test]
fn a_reaped_child_never_reuses_its_numeric_coordinate_for_group_signaling() {
    let mut target = AlreadyReapedTarget { group_signals: 0 };
    let uncertainty = terminate_target(
        &mut target,
        TerminationPolicy::graceful_then_forced(Duration::from_secs(1), Duration::from_secs(1)),
    )
    .unwrap_err();

    assert_eq!(target.group_signals, 0);
    assert!(uncertainty.direct_child_reaped());
    assert!(!uncertainty.process_group_absent());
}

struct SuccessfulTarget {
    initial: DirectChildObservation,
    direct_kills: usize,
}

impl TerminationTarget for SuccessfulTarget {
    fn observe_direct_child(&mut self) -> std::io::Result<DirectChildObservation> {
        Ok(self.initial)
    }

    fn try_reap_direct_child(&mut self) -> std::io::Result<bool> {
        Ok(true)
    }

    fn signal_group(&mut self, _signal: GroupSignal) -> std::io::Result<GroupSignalOutcome> {
        Ok(GroupSignalOutcome::Absent)
    }

    fn group_is_absent(&mut self) -> std::io::Result<bool> {
        panic!("an absent signal result already proves group absence")
    }

    fn kill_direct_child(&mut self) -> std::io::Result<()> {
        self.direct_kills += 1;
        Ok(())
    }
}

#[test]
fn successful_cleanup_preserves_preexisting_exit_and_classifies_a_running_child() {
    for (initial, expected, expected_kills) in [
        (
            DirectChildObservation::Terminal(SupervisedProcessExit::Success),
            SupervisedProcessExit::Success,
            0,
        ),
        (
            DirectChildObservation::Terminal(SupervisedProcessExit::Failure { code: Some(7) }),
            SupervisedProcessExit::Failure { code: Some(7) },
            0,
        ),
        (
            DirectChildObservation::Running,
            SupervisedProcessExit::Terminated,
            1,
        ),
    ] {
        let mut target = SuccessfulTarget {
            initial,
            direct_kills: 0,
        };
        let completed = terminate_target(
            &mut target,
            TerminationPolicy::immediate(Duration::from_secs(1)),
        )
        .expect("complete cleanup must retain the exact exit class");

        assert_eq!(completed, expected);
        assert_eq!(target.direct_kills, expected_kills);
    }
}
