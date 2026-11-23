//! Resident admission, coalescing, and cancellation laws.

use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use pale_core::records::{
    CompilationInputDigest, RequestCallerIdV1, ResidentRequestIdV1, ResidentWorkspaceIdV1,
};

use super::{
    resident_cancellation_pair, FlightWake, ResidentAdmissionLimits, ResidentAdmissionScope,
    ResidentCompilationCoordinator, ResidentCompilationExecution, ResidentRequestControl,
    ResidentRequestInterruption, ResidentWorkCoordinationError,
};

fn nonzero(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).expect("positive test limit")
}

fn limits(maximum_parallel_computations: usize) -> ResidentAdmissionLimits {
    ResidentAdmissionLimits::new(
        nonzero(64),
        nonzero(64),
        nonzero(64),
        nonzero(maximum_parallel_computations),
    )
}

fn digest(byte: char) -> CompilationInputDigest {
    CompilationInputDigest::parse(std::iter::repeat_n(byte, 64).collect::<String>())
        .expect("compilation input digest")
}

fn control(id: &str, deadline: Instant) -> ResidentRequestControl {
    scoped_control(id, "caller-default", "workspace-default", deadline)
}

fn scoped_control(
    id: &str,
    caller: &str,
    workspace: &str,
    deadline: Instant,
) -> ResidentRequestControl {
    let (_trigger, cancellation) = resident_cancellation_pair();
    ResidentRequestControl::new(
        ResidentRequestIdV1::parse(id).expect("request id"),
        RequestCallerIdV1::parse(caller).expect("caller id"),
        ResidentWorkspaceIdV1::parse(workspace).expect("workspace id"),
        deadline,
        cancellation,
    )
}

#[test]
fn interruption_preserves_cancelled_deadline_product_state() {
    let now = Instant::now();
    let (trigger, cancellation) = resident_cancellation_pair();
    let request = ResidentRequestControl::new(
        ResidentRequestIdV1::parse("request-interruption").expect("request id"),
        RequestCallerIdV1::parse("caller-interruption").expect("caller id"),
        ResidentWorkspaceIdV1::parse("workspace-interruption").expect("workspace id"),
        now + Duration::from_secs(1),
        cancellation,
    );

    assert_eq!(request.interruption_at(now), None);
    assert!(trigger.cancel());
    assert_eq!(
        request.interruption_at(now),
        Some(ResidentRequestInterruption::Cancelled)
    );
    assert_eq!(
        request.interruption_at(request.deadline()),
        Some(ResidentRequestInterruption::CancelledAfterDeadline)
    );

    let uncancelled = control("request-deadline-only", now);
    assert_eq!(
        uncancelled.interruption_at(now),
        Some(ResidentRequestInterruption::DeadlineElapsed)
    );
    assert!(!trigger.cancel());
}

#[test]
fn cancellation_between_check_and_park_is_not_lost() {
    let wake = Arc::new(FlightWake::default());
    let (trigger, cancellation) = resident_cancellation_pair();
    cancellation
        .observe(&wake)
        .expect("register cancellation wake point");
    let gate = wake.gate.lock().expect("flight wake gate");
    assert!(!cancellation.is_cancelled());

    let canceller = thread::spawn(move || trigger.cancel());
    let cancellation_deadline = Instant::now() + Duration::from_secs(1);
    while !cancellation.is_cancelled() {
        assert!(
            Instant::now() < cancellation_deadline,
            "canceller must publish its predicate before the test parks"
        );
        thread::yield_now();
    }

    let (gate, timeout) = wake
        .changed
        .wait_timeout(gate, Duration::from_secs(1))
        .expect("wait across cancellation edge");
    assert!(
        !timeout.timed_out(),
        "cancellation after the predicate check must wake the registered waiter"
    );
    drop(gate);
    assert!(canceller.join().expect("canceller thread"));
}

fn assert_request_saturation(
    limits: ResidentAdmissionLimits,
    first: (&str, &str),
    second: (&str, &str),
    expected: &[ResidentAdmissionScope],
) {
    let coordinator = Arc::new(ResidentCompilationCoordinator::<u64, &'static str>::new(
        limits,
    ));
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let first_control = scoped_control(
        "request-admission-first",
        first.0,
        first.1,
        Instant::now() + Duration::from_secs(2),
    );
    let leader = {
        let coordinator = Arc::clone(&coordinator);
        thread::spawn(move || {
            coordinator.run(digest('1'), &first_control, || {
                started_tx.send(()).expect("signal admitted request");
                release_rx.recv().expect("release admitted request");
                Ok(1)
            })
        })
    };
    started_rx.recv().expect("first request admitted");
    let snapshot = coordinator
        .admission_snapshot()
        .expect("admission snapshot");
    assert_eq!(snapshot.active_requests(), 1);
    assert_eq!(snapshot.active_callers(), 1);
    assert_eq!(snapshot.active_workspaces(), 1);
    assert_eq!(snapshot.active_computations(), 1);

    let error = match coordinator.run(
        digest('2'),
        &scoped_control(
            "request-admission-second",
            second.0,
            second.1,
            Instant::now() + Duration::from_secs(2),
        ),
        || Ok(2),
    ) {
        Ok(_) => panic!("saturated request must be rejected without a queue"),
        Err(error) => error,
    };
    let ResidentWorkCoordinationError::Saturated { saturation, .. } = error else {
        panic!("request must retain typed saturation evidence");
    };
    assert_eq!(saturation.scopes().collect::<Vec<_>>(), expected);
    release_tx.send(()).expect("release first request");
    leader
        .join()
        .expect("leader thread")
        .expect("leader result");
    assert_eq!(
        coordinator.admission_snapshot().expect("released snapshot"),
        super::ResidentAdmissionSnapshot {
            active_requests: 0,
            active_callers: 0,
            active_workspaces: 0,
            active_computations: 0,
        }
    );
}

#[test]
fn request_saturation_preserves_each_scope_and_their_intersection() {
    assert_request_saturation(
        ResidentAdmissionLimits::new(nonzero(1), nonzero(4), nonzero(4), nonzero(2)),
        ("caller-a", "workspace-a"),
        ("caller-a", "workspace-b"),
        &[ResidentAdmissionScope::Caller],
    );
    assert_request_saturation(
        ResidentAdmissionLimits::new(nonzero(4), nonzero(1), nonzero(4), nonzero(2)),
        ("caller-a", "workspace-a"),
        ("caller-b", "workspace-a"),
        &[ResidentAdmissionScope::Workspace],
    );
    assert_request_saturation(
        ResidentAdmissionLimits::new(nonzero(4), nonzero(4), nonzero(1), nonzero(2)),
        ("caller-a", "workspace-a"),
        ("caller-b", "workspace-b"),
        &[ResidentAdmissionScope::Global],
    );
    assert_request_saturation(
        ResidentAdmissionLimits::new(nonzero(1), nonzero(1), nonzero(1), nonzero(2)),
        ("caller-a", "workspace-a"),
        ("caller-a", "workspace-a"),
        &[
            ResidentAdmissionScope::Caller,
            ResidentAdmissionScope::Workspace,
            ResidentAdmissionScope::Global,
        ],
    );
}

#[test]
fn saturated_caller_does_not_block_an_independent_load_class() {
    let coordinator = Arc::new(ResidentCompilationCoordinator::<u64, &'static str>::new(
        ResidentAdmissionLimits::new(nonzero(1), nonzero(2), nonzero(3), nonzero(2)),
    ));
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let leader = {
        let coordinator = Arc::clone(&coordinator);
        thread::spawn(move || {
            coordinator.run(
                digest('3'),
                &scoped_control(
                    "request-isolation-first",
                    "caller-a",
                    "workspace-a",
                    Instant::now() + Duration::from_secs(2),
                ),
                || {
                    started_tx.send(()).expect("signal first class");
                    release_rx.recv().expect("release first class");
                    Ok(31)
                },
            )
        })
    };
    started_rx.recv().expect("first class started");

    let rejected = match coordinator.run(
        digest('4'),
        &scoped_control(
            "request-isolation-rejected",
            "caller-a",
            "workspace-b",
            Instant::now() + Duration::from_secs(1),
        ),
        || Ok(41),
    ) {
        Ok(_) => panic!("saturated caller must close only its own request"),
        Err(error) => error,
    };
    assert!(matches!(
        rejected,
        ResidentWorkCoordinationError::Saturated { saturation, .. }
            if saturation.scopes().eq([ResidentAdmissionScope::Caller])
    ));

    let independent = coordinator
        .run(
            digest('5'),
            &scoped_control(
                "request-isolation-independent",
                "caller-b",
                "workspace-b",
                Instant::now() + Duration::from_secs(1),
            ),
            || Ok(51),
        )
        .expect("independent load class");
    assert_eq!(independent.result(), &Ok(51));
    let retained = coordinator
        .admission_snapshot()
        .expect("first class remains admitted");
    assert_eq!(retained.active_requests(), 1);
    assert_eq!(retained.active_callers(), 1);
    assert_eq!(retained.active_workspaces(), 1);
    assert_eq!(retained.active_computations(), 1);

    release_tx.send(()).expect("release first class");
    let first = leader.join().expect("first thread").expect("first result");
    assert_eq!(first.result(), &Ok(31));
}

#[test]
fn independent_workspaces_progress_concurrently_without_result_crosstalk() {
    let coordinator = Arc::new(ResidentCompilationCoordinator::<u64, &'static str>::new(
        ResidentAdmissionLimits::new(nonzero(1), nonzero(1), nonzero(2), nonzero(2)),
    ));
    let (started_tx, started_rx) = mpsc::channel();
    let (release_a_tx, release_a_rx) = mpsc::channel();
    let (release_b_tx, release_b_rx) = mpsc::channel();
    let first = {
        let coordinator = Arc::clone(&coordinator);
        let started_tx = started_tx.clone();
        thread::spawn(move || {
            coordinator.run(
                digest('6'),
                &scoped_control(
                    "request-class-a",
                    "caller-a",
                    "workspace-a",
                    Instant::now() + Duration::from_secs(2),
                ),
                || {
                    started_tx.send(()).expect("signal class A");
                    release_a_rx.recv().expect("release class A");
                    Ok(61)
                },
            )
        })
    };
    let second = {
        let coordinator = Arc::clone(&coordinator);
        thread::spawn(move || {
            coordinator.run(
                digest('7'),
                &scoped_control(
                    "request-class-b",
                    "caller-b",
                    "workspace-b",
                    Instant::now() + Duration::from_secs(2),
                ),
                || {
                    started_tx.send(()).expect("signal class B");
                    release_b_rx.recv().expect("release class B");
                    Ok(71)
                },
            )
        })
    };
    started_rx.recv().expect("one class started");
    started_rx.recv().expect("both classes started");
    let concurrent = coordinator
        .admission_snapshot()
        .expect("concurrent load snapshot");
    assert_eq!(concurrent.active_requests(), 2);
    assert_eq!(concurrent.active_callers(), 2);
    assert_eq!(concurrent.active_workspaces(), 2);
    assert_eq!(concurrent.active_computations(), 2);
    release_a_tx.send(()).expect("release class A");
    release_b_tx.send(()).expect("release class B");
    let first = first
        .join()
        .expect("class A thread")
        .expect("class A result");
    let second = second
        .join()
        .expect("class B thread")
        .expect("class B result");
    assert_eq!(first.result(), &Ok(61));
    assert_eq!(second.result(), &Ok(71));
    assert!(!Arc::ptr_eq(
        &first.shared_result(),
        &second.shared_result()
    ));
}

#[test]
fn overlapping_exact_digest_has_one_producer_and_one_joiner() {
    let coordinator = Arc::new(ResidentCompilationCoordinator::<u64, &'static str>::new(
        limits(2),
    ));
    let computations = Arc::new(AtomicUsize::new(0));
    let started = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let leader = {
        let coordinator = Arc::clone(&coordinator);
        let computations = Arc::clone(&computations);
        let started = Arc::clone(&started);
        let release = Arc::clone(&release);
        thread::spawn(move || {
            coordinator.run(
                digest('a'),
                &control("request-leader", Instant::now() + Duration::from_secs(2)),
                || {
                    computations.fetch_add(1, Ordering::SeqCst);
                    started.wait();
                    release.wait();
                    Ok(41)
                },
            )
        })
    };
    started.wait();
    let (follower_cancel, follower_cancellation) = resident_cancellation_pair();
    let follower = {
        let coordinator = Arc::clone(&coordinator);
        let computations = Arc::clone(&computations);
        thread::spawn(move || {
            coordinator.run(
                digest('a'),
                &ResidentRequestControl::new(
                    ResidentRequestIdV1::parse("request-follower").expect("request id"),
                    RequestCallerIdV1::parse("caller-default").expect("caller id"),
                    ResidentWorkspaceIdV1::parse("workspace-default").expect("workspace id"),
                    Instant::now() + Duration::from_secs(2),
                    follower_cancellation,
                ),
                || {
                    computations.fetch_add(1, Ordering::SeqCst);
                    Ok(99)
                },
            )
        })
    };
    while follower_cancel
        .shared
        .observers
        .lock()
        .expect("cancellation observers")
        .is_empty()
    {
        thread::yield_now();
    }
    let overlapping = coordinator
        .admission_snapshot()
        .expect("overlapping admission snapshot");
    assert_eq!(overlapping.active_requests(), 2);
    assert_eq!(overlapping.active_callers(), 1);
    assert_eq!(overlapping.active_workspaces(), 1);
    assert_eq!(overlapping.active_computations(), 1);
    release.wait();

    let leader = leader
        .join()
        .expect("leader thread")
        .expect("leader result");
    let follower = follower
        .join()
        .expect("follower thread")
        .expect("follower result");
    assert_eq!(computations.load(Ordering::SeqCst), 1);
    assert_eq!(leader.execution(), ResidentCompilationExecution::Produced);
    assert_eq!(follower.execution(), ResidentCompilationExecution::Joined);
    assert_eq!(leader.result(), &Ok(41));
    assert_eq!(follower.result(), &Ok(41));
    assert!(Arc::ptr_eq(
        &leader.shared_result(),
        &follower.shared_result()
    ));
}

#[test]
fn joiner_deadline_does_not_end_the_shared_computation() {
    let coordinator = Arc::new(ResidentCompilationCoordinator::<u64, &'static str>::new(
        limits(1),
    ));
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let leader = {
        let coordinator = Arc::clone(&coordinator);
        thread::spawn(move || {
            coordinator.run(
                digest('f'),
                &control(
                    "request-deadline-producer",
                    Instant::now() + Duration::from_secs(2),
                ),
                || {
                    started_tx.send(()).expect("signal producer");
                    release_rx.recv().expect("release producer");
                    Ok(11)
                },
            )
        })
    };
    started_rx.recv().expect("producer started");
    let error = match coordinator.run(
        digest('f'),
        &control(
            "request-deadline-joiner",
            Instant::now() + Duration::from_millis(10),
        ),
        || Ok(12),
    ) {
        Ok(_) => panic!("joiner deadline must close its wait"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        ResidentWorkCoordinationError::Interrupted {
            reason: ResidentRequestInterruption::DeadlineElapsed,
            ..
        }
    ));
    release_tx.send(()).expect("release producer");
    let produced = leader
        .join()
        .expect("leader thread")
        .expect("leader result");
    assert_eq!(produced.result(), &Ok(11));
}

#[test]
fn producer_panic_wakes_a_surviving_joiner_to_retry() {
    let coordinator = Arc::new(ResidentCompilationCoordinator::<u64, &'static str>::new(
        limits(1),
    ));
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let producer = {
        let coordinator = Arc::clone(&coordinator);
        thread::spawn(move || {
            coordinator.run(
                digest('9'),
                &control(
                    "request-panicking-producer",
                    Instant::now() + Duration::from_secs(2),
                ),
                || {
                    started_tx.send(()).expect("signal producer");
                    release_rx.recv().expect("release producer");
                    panic!("synthetic producer failure");
                },
            )
        })
    };
    started_rx.recv().expect("producer started");
    let (joiner_cancel, joiner_cancellation) = resident_cancellation_pair();
    let joiner = {
        let coordinator = Arc::clone(&coordinator);
        thread::spawn(move || {
            coordinator.run(
                digest('9'),
                &ResidentRequestControl::new(
                    ResidentRequestIdV1::parse("request-surviving-joiner").expect("request id"),
                    RequestCallerIdV1::parse("caller-default").expect("caller id"),
                    ResidentWorkspaceIdV1::parse("workspace-default").expect("workspace id"),
                    Instant::now() + Duration::from_secs(2),
                    joiner_cancellation,
                ),
                || Ok(73),
            )
        })
    };
    while joiner_cancel
        .shared
        .observers
        .lock()
        .expect("cancellation observers")
        .is_empty()
    {
        thread::yield_now();
    }
    release_tx.send(()).expect("release producer");

    assert!(producer.join().is_err());
    let retried = joiner.join().expect("joiner thread").expect("joiner retry");
    assert_eq!(retried.execution(), ResidentCompilationExecution::Produced);
    assert_eq!(retried.result(), &Ok(73));
}

#[test]
fn cancellation_closes_only_the_joiner() {
    let coordinator = Arc::new(ResidentCompilationCoordinator::<u64, &'static str>::new(
        limits(1),
    ));
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let leader = {
        let coordinator = Arc::clone(&coordinator);
        thread::spawn(move || {
            coordinator.run(
                digest('b'),
                &control("request-producer", Instant::now() + Duration::from_secs(2)),
                || {
                    started_tx.send(()).expect("signal producer");
                    release_rx.recv().expect("release producer");
                    Ok(7)
                },
            )
        })
    };
    started_rx.recv().expect("producer started");
    let (cancel, cancellation) = resident_cancellation_pair();
    let joiner_control = ResidentRequestControl::new(
        ResidentRequestIdV1::parse("request-cancelled-joiner").expect("request id"),
        RequestCallerIdV1::parse("caller-default").expect("caller id"),
        ResidentWorkspaceIdV1::parse("workspace-default").expect("workspace id"),
        Instant::now() + Duration::from_secs(5),
        cancellation,
    );
    let (joiner_tx, joiner_rx) = mpsc::channel();
    let joiner = {
        let coordinator = Arc::clone(&coordinator);
        thread::spawn(move || {
            let result = coordinator.run(digest('b'), &joiner_control, || Ok(8));
            joiner_tx.send(result).expect("send joiner result");
        })
    };
    let observation_deadline = Instant::now() + Duration::from_secs(1);
    while cancel
        .shared
        .observers
        .lock()
        .expect("cancellation observers")
        .is_empty()
    {
        assert!(
            Instant::now() < observation_deadline,
            "joiner must register its cancellation wake point"
        );
        thread::yield_now();
    }
    assert!(cancel.cancel());
    let error = match joiner_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("cancellation must release the joiner before its producer")
    {
        Ok(_) => panic!("cancelled joiner must not receive shared work"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        ResidentWorkCoordinationError::Interrupted {
            reason: ResidentRequestInterruption::Cancelled,
            ..
        }
    ));
    joiner.join().expect("joiner thread");
    let retained = coordinator
        .admission_snapshot()
        .expect("producer-only admission snapshot");
    assert_eq!(retained.active_requests(), 1);
    assert_eq!(retained.active_callers(), 1);
    assert_eq!(retained.active_workspaces(), 1);
    assert_eq!(retained.active_computations(), 1);
    release_tx.send(()).expect("release producer");
    let produced = leader
        .join()
        .expect("leader thread")
        .expect("leader result");
    assert_eq!(produced.result(), &Ok(7));
}

#[test]
fn distinct_digest_is_rejected_at_the_explicit_cpu_ceiling() {
    let coordinator = Arc::new(ResidentCompilationCoordinator::<u64, &'static str>::new(
        limits(1),
    ));
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let leader = {
        let coordinator = Arc::clone(&coordinator);
        thread::spawn(move || {
            coordinator.run(
                digest('c'),
                &control("request-first", Instant::now() + Duration::from_secs(2)),
                || {
                    started_tx.send(()).expect("signal producer");
                    release_rx.recv().expect("release producer");
                    Ok(1)
                },
            )
        })
    };
    started_rx.recv().expect("producer started");
    let error = match coordinator.run(
        digest('d'),
        &control("request-second", Instant::now() + Duration::from_secs(2)),
        || Ok(2),
    ) {
        Ok(_) => panic!("second distinct flight must be saturated"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        ResidentWorkCoordinationError::Saturated { saturation, .. }
            if saturation.contains(ResidentAdmissionScope::Computation)
    ));
    release_tx.send(()).expect("release producer");
    leader
        .join()
        .expect("leader thread")
        .expect("leader result");
}

#[test]
fn completed_result_is_not_a_process_local_cache() {
    let coordinator = ResidentCompilationCoordinator::<u64, &'static str>::new(limits(1));
    let computations = AtomicUsize::new(0);
    for request in ["request-one", "request-two"] {
        let result = coordinator
            .run(
                digest('e'),
                &control(request, Instant::now() + Duration::from_secs(1)),
                || {
                    computations.fetch_add(1, Ordering::SeqCst);
                    Ok(3)
                },
            )
            .expect("sequential computation");
        assert_eq!(result.execution(), ResidentCompilationExecution::Produced);
    }
    assert_eq!(computations.load(Ordering::SeqCst), 2);
}
