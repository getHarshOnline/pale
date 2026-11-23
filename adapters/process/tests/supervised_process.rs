//! Behavioral integration proof for directly owned long-lived child mechanics.

#![cfg(unix)]
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(not(any(
    target_os = "cygwin",
    target_os = "horizon",
    target_os = "openbsd",
    target_os = "redox",
    target_os = "wasi"
)))]
use rustix::process::{waitid, Pid, WaitId, WaitIdOptions};

use pale_adapter_process::LocalProcessSupervisor;
use pale_core::records::ExecutableDigest;
use pale_ports::{
    ExecutableIdentityError, HostProcessStartObservation, ProcessEnvironmentEntry,
    ProcessTerminationPolicy, SupervisedProcessExit, SupervisedProcessObservation,
    SupervisedProcessPort, SupervisedProcessRequest, SupervisedProcessStartError,
};

fn fallback_termination() -> ProcessTerminationPolicy {
    ProcessTerminationPolicy::immediate(Duration::from_secs(2))
}

fn immediate_termination() -> ProcessTerminationPolicy {
    ProcessTerminationPolicy::immediate(Duration::from_secs(2))
}

fn graceful_termination() -> ProcessTerminationPolicy {
    ProcessTerminationPolicy::graceful_then_forced(
        Duration::from_millis(250),
        Duration::from_secs(2),
    )
}

fn executable(directory: &tempfile::TempDir, body: &str) -> (PathBuf, ExecutableDigest) {
    let path = directory.path().join("child");
    let bytes = format!("#!/bin/sh\n{body}\n");
    fs::write(&path, &bytes).expect("write child fixture");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
        .expect("make child fixture executable");
    (path, ExecutableDigest::hash_bytes(bytes.as_bytes()))
}

// The descendant announces that it exists, then waits for the test to permit
// its observable action. Cleanup must not pass merely because spawn was slow.
// Removing the fixture directory also ends polling if a test unwinds and
// process-group cleanup is uncertain. The directory outlives every assertion.
const WAITING_DESCENDANT: &str = r#"
(printf armed > "$2"; while [ -f "$0" ] && [ ! -e "$3" ]; do sleep 0.005; done; if [ -f "$0" ]; then printf survived > "$1"; fi) &
"#;

// Allow the host to load the interpreter and reach the fixture's initial state.
// This is a test setup limit, separate from the supervisor's cleanup deadlines.
const FIXTURE_INITIAL_STATE_TIMEOUT: Duration = Duration::from_secs(10);

fn wait_for(path: &Path) {
    let deadline = Instant::now() + FIXTURE_INITIAL_STATE_TIMEOUT;
    while !path.exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    assert!(path.exists(), "child did not produce {}", path.display());
}

fn wait_for_exit(process: &mut dyn pale_ports::SupervisedProcess) -> SupervisedProcessExit {
    let deadline = Instant::now() + FIXTURE_INITIAL_STATE_TIMEOUT;
    loop {
        match process.observe().expect("observe child") {
            SupervisedProcessObservation::Exited(exit) => return exit,
            SupervisedProcessObservation::Running if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(5));
            }
            SupervisedProcessObservation::Running => panic!("child did not exit"),
        }
    }
}

#[cfg(not(any(
    target_os = "cygwin",
    target_os = "horizon",
    target_os = "openbsd",
    target_os = "redox",
    target_os = "wasi"
)))]
fn wait_for_unreaped_exit(process: &dyn pale_ports::SupervisedProcess) {
    let process_id = i32::try_from(process.identity().process_id().get())
        .ok()
        .and_then(Pid::from_raw)
        .expect("a host child PID must fit the platform PID type");
    let deadline = Instant::now() + FIXTURE_INITIAL_STATE_TIMEOUT;
    loop {
        let terminal = waitid(
            WaitId::Pid(process_id),
            WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
        )
        .expect("observe fixture child without reaping")
        .is_some();
        if terminal {
            return;
        }
        assert!(Instant::now() < deadline, "fixture child did not exit");
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn retains_the_direct_child_and_literal_invocation_until_bounded_termination() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let marker = directory.path().join("started");
    let body = "printf '%s|%s|%s' \"$2\" \"${ONLY-}\" \"${HOME-unset}\" > \"$1\"; trap 'exit 0' TERM; while :; do sleep 0.05; done";
    let (path, digest) = executable(&directory, body);
    let marker_text = marker.to_string_lossy();
    let argument = "literal;$(not-expanded) *";
    let arguments = [marker_text.as_ref(), argument];
    let environment = [ProcessEnvironmentEntry {
        name: "ONLY",
        value: "selected",
    }];
    let mut process = LocalProcessSupervisor
        .spawn(SupervisedProcessRequest {
            executable: path.to_str().expect("UTF-8 fixture path"),
            expected_digest: &digest,
            arguments: &arguments,
            environment: &environment,
            fallback_termination: fallback_termination(),
        })
        .expect("spawn supervised child");

    wait_for(&marker);
    assert_eq!(
        fs::read_to_string(&marker).expect("read marker"),
        format!("{argument}|selected|unset")
    );
    assert_eq!(
        process.identity().process_group_id().map(|id| id.get()),
        Some(process.identity().process_id().get())
    );
    if let HostProcessStartObservation::Observed(start) = process.identity().start() {
        assert!(start.boot_relative_ticks() > 0);
    }
    assert_eq!(
        process.observe().expect("observe running child"),
        SupervisedProcessObservation::Running
    );

    process
        .terminate(graceful_termination())
        .expect("terminate child tree");
    assert_eq!(
        process.observe().expect("observe terminated child"),
        SupervisedProcessObservation::Exited(SupervisedProcessExit::Terminated)
    );
    process
        .terminate(immediate_termination())
        .expect("terminal cleanup is idempotent");
}

#[test]
fn rejects_unverified_executable_bytes_before_any_child_effect() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let marker = directory.path().join("must-not-exist");
    let body = "printf started > \"$1\"";
    let (path, _digest) = executable(&directory, body);
    let wrong_digest = ExecutableDigest::hash_bytes(b"not the executable");
    let marker_text = marker.to_string_lossy();
    let arguments = [marker_text.as_ref()];

    let error = LocalProcessSupervisor
        .spawn(SupervisedProcessRequest {
            executable: path.to_str().expect("UTF-8 fixture path"),
            expected_digest: &wrong_digest,
            arguments: &arguments,
            environment: &[],
            fallback_termination: fallback_termination(),
        })
        .err()
        .expect("digest mismatch must reject spawn");

    assert!(matches!(
        error,
        SupervisedProcessStartError::Executable(ExecutableIdentityError::DigestMismatch { .. })
    ));
    assert!(!marker.exists(), "unverified executable was launched");
}

#[test]
fn preserves_a_natural_terminal_state_and_makes_later_cleanup_idempotent() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let (path, digest) = executable(&directory, "exit 7");
    let mut process = LocalProcessSupervisor
        .spawn(SupervisedProcessRequest {
            executable: path.to_str().expect("UTF-8 fixture path"),
            expected_digest: &digest,
            arguments: &[],
            environment: &[],
            fallback_termination: fallback_termination(),
        })
        .expect("spawn supervised child");

    assert_eq!(
        wait_for_exit(process.as_mut()),
        SupervisedProcessExit::Failure { code: Some(7) }
    );
    process
        .terminate(immediate_termination())
        .expect("natural terminal state remains terminal");
    assert_eq!(
        process.observe().expect("observe retained terminal state"),
        SupervisedProcessObservation::Exited(SupervisedProcessExit::Failure { code: Some(7) })
    );
}

#[test]
fn forced_cleanup_contains_descendants_in_the_isolated_process_group() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let marker = directory.path().join("descendant-survived");
    let armed = directory.path().join("descendant-armed");
    let release = directory.path().join("release-descendant");
    let (path, digest) = executable(&directory, &format!("{WAITING_DESCENDANT}wait"));
    let marker_text = marker.to_string_lossy();
    let armed_text = armed.to_string_lossy();
    let release_text = release.to_string_lossy();
    let arguments = [
        marker_text.as_ref(),
        armed_text.as_ref(),
        release_text.as_ref(),
    ];
    let mut process = LocalProcessSupervisor
        .spawn(SupervisedProcessRequest {
            executable: path.to_str().expect("UTF-8 fixture path"),
            expected_digest: &digest,
            arguments: &arguments,
            environment: &[],
            fallback_termination: fallback_termination(),
        })
        .expect("spawn child tree");

    wait_for(&armed);
    process
        .terminate(immediate_termination())
        .expect("contain child tree");
    fs::write(&release, b"release").expect("release descendant action after cleanup");
    thread::sleep(Duration::from_millis(300));
    assert!(
        !marker.exists(),
        "descendant survived process-group cleanup"
    );
}

#[test]
fn natural_direct_child_exit_does_not_skip_descendant_cleanup() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let marker = directory.path().join("detached-descendant-survived");
    let armed = directory.path().join("descendant-armed");
    let release = directory.path().join("release-descendant");
    let (path, digest) = executable(&directory, &format!("{WAITING_DESCENDANT}exit 0"));
    let marker_text = marker.to_string_lossy();
    let armed_text = armed.to_string_lossy();
    let release_text = release.to_string_lossy();
    let arguments = [
        marker_text.as_ref(),
        armed_text.as_ref(),
        release_text.as_ref(),
    ];
    let mut process = LocalProcessSupervisor
        .spawn(SupervisedProcessRequest {
            executable: path.to_str().expect("UTF-8 fixture path"),
            expected_digest: &digest,
            arguments: &arguments,
            environment: &[],
            fallback_termination: fallback_termination(),
        })
        .expect("spawn child tree");

    wait_for(&armed);
    assert_eq!(
        wait_for_exit(process.as_mut()),
        SupervisedProcessExit::Success
    );
    process
        .terminate(immediate_termination())
        .expect("clean descendants after direct child exit");
    fs::write(&release, b"release").expect("release descendant action after cleanup");
    thread::sleep(Duration::from_millis(300));
    assert!(
        !marker.exists(),
        "direct-child exit was mistaken for descendant cleanup"
    );
}

#[cfg(not(any(
    target_os = "cygwin",
    target_os = "horizon",
    target_os = "openbsd",
    target_os = "redox",
    target_os = "wasi"
)))]
#[test]
fn cleanup_preserves_an_unobserved_natural_failure() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let (path, digest) = executable(&directory, "exit 7");
    let mut process = LocalProcessSupervisor
        .spawn(SupervisedProcessRequest {
            executable: path.to_str().expect("UTF-8 fixture path"),
            expected_digest: &digest,
            arguments: &[],
            environment: &[],
            fallback_termination: fallback_termination(),
        })
        .expect("spawn supervised child");

    wait_for_unreaped_exit(process.as_ref());
    process
        .terminate(immediate_termination())
        .expect("clean naturally failed child");
    assert_eq!(
        process.observe().expect("observe retained natural failure"),
        SupervisedProcessObservation::Exited(SupervisedProcessExit::Failure { code: Some(7) })
    );
}

#[cfg(not(any(
    target_os = "cygwin",
    target_os = "horizon",
    target_os = "openbsd",
    target_os = "redox",
    target_os = "wasi"
)))]
#[test]
fn cleanup_preserves_unobserved_natural_success_and_contains_descendants() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let marker = directory.path().join("detached-descendant-survived");
    let armed = directory.path().join("descendant-armed");
    let release = directory.path().join("release-descendant");
    let (path, digest) = executable(&directory, &format!("{WAITING_DESCENDANT}exit 0"));
    let marker_text = marker.to_string_lossy();
    let armed_text = armed.to_string_lossy();
    let release_text = release.to_string_lossy();
    let arguments = [
        marker_text.as_ref(),
        armed_text.as_ref(),
        release_text.as_ref(),
    ];
    let mut process = LocalProcessSupervisor
        .spawn(SupervisedProcessRequest {
            executable: path.to_str().expect("UTF-8 fixture path"),
            expected_digest: &digest,
            arguments: &arguments,
            environment: &[],
            fallback_termination: fallback_termination(),
        })
        .expect("spawn child tree");

    wait_for(&armed);
    wait_for_unreaped_exit(process.as_ref());
    process
        .terminate(immediate_termination())
        .expect("clean descendants after natural direct-child exit");
    fs::write(&release, b"release").expect("release descendant action after cleanup");
    assert_eq!(
        process.observe().expect("observe retained natural success"),
        SupervisedProcessObservation::Exited(SupervisedProcessExit::Success)
    );
    thread::sleep(Duration::from_millis(300));
    assert!(
        !marker.exists(),
        "natural direct-child success skipped descendant cleanup"
    );
}

#[test]
fn armed_descendant_performs_its_action_when_released_without_cleanup() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let marker = directory.path().join("descendant-survived");
    let armed = directory.path().join("descendant-armed");
    let release = directory.path().join("release-descendant");
    // Keep the direct child available for cleanup after the witness completes.
    let (path, digest) = executable(
        &directory,
        &format!("{WAITING_DESCENDANT}wait; while [ -f \"$0\" ]; do sleep 0.05; done"),
    );
    let marker_text = marker.to_string_lossy();
    let armed_text = armed.to_string_lossy();
    let release_text = release.to_string_lossy();
    let arguments = [
        marker_text.as_ref(),
        armed_text.as_ref(),
        release_text.as_ref(),
    ];
    let mut process = LocalProcessSupervisor
        .spawn(SupervisedProcessRequest {
            executable: path.to_str().expect("UTF-8 fixture path"),
            expected_digest: &digest,
            arguments: &arguments,
            environment: &[],
            fallback_termination: fallback_termination(),
        })
        .expect("spawn child tree");

    wait_for(&armed);
    assert_eq!(
        process.observe().expect("observe waiting parent"),
        SupervisedProcessObservation::Running
    );
    assert!(!marker.exists(), "descendant acted before release");
    fs::write(&release, b"release").expect("release descendant action without cleanup");
    let deadline = Instant::now() + Duration::from_secs(2);
    while fs::read(&marker).ok().as_deref() != Some(b"survived") && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        fs::read(&marker).expect("read descendant action"),
        b"survived"
    );
    process
        .terminate(immediate_termination())
        .expect("finish fixture cleanup");
}
