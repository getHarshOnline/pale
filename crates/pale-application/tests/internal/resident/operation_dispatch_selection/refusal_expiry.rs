//! Expiry evidence from an actual unopened refusal, not an arbitrary terminal fixture.
//! The monotonic deadline uses the existing explicit fixture timer model, without sleep.

use super::*;
use crate::resident::{
    CompletedOperationObservationCustody, NativeOperationExpiryInput,
    NativeOperationRefusalCancellationInput, PreparedNativeOperationDispatch,
};
use pale_ports::{AntiRollbackAnchorPort, OperationCustodyPort};

#[derive(Clone, Copy, Debug)]
pub(super) enum Case {
    Handle,
    LeaseAndHandle,
    Deadline,
    CancellationCrossing,
    NoExpiry,
    Clock,
    Backward,
    CallerFloor,
    RejectedCrossing,
    Revocation,
    Replay,
    Recovered,
    ChangedHead,
}

struct MissingClock;
impl WallClockSourcePort for MissingClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        Err(WallClockSourceError::from_source(std::io::Error::other(
            "fixture clock absent",
        )))
    }
}

pub(super) fn check(
    case: Case,
    dispatch: PreparedNativeOperationDispatch<'_, '_>,
    completed: CompletedOperationObservationCustody,
    input: NativeOperationExecutionContext<'_>,
    signing: SignedResidentAdmissionContextV1<'_>,
    store: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) {
    let predecessor = dispatch.observation().0.clone();
    let predecessor_digest = dispatch
        .observation()
        .1
        .canonical_operation_observation_sha256()
        .clone();
    let sample = match case {
        Case::LeaseAndHandle => 90,
        Case::Handle
        | Case::Replay
        | Case::Recovered
        | Case::ChangedHead
        | Case::RejectedCrossing => 80,
        Case::Backward => 50,
        _ => 65,
    };
    let clock = Clock {
        times: [sample; 3],
        reads: AtomicUsize::new(0),
    };
    let mut current = input.current.at_time(input.current.now);
    if matches!(case, Case::CallerFloor) {
        current.now = UnixTimeV1::new(80).unwrap();
    }
    if matches!(case, Case::RejectedCrossing) {
        current.now = UnixTimeV1::new(90).unwrap();
    }
    let mut revocations = *current.session_revocations;
    if matches!(case, Case::Revocation) {
        revocations.epoch = PrincipalRevocationEpochV1::new(2).unwrap();
        current.session_revocations = &revocations;
    }
    let mut refused = dispatch
        .invoke(
            completed,
            NativeOperationExecutionContext {
                current: &current,
                resolve_store: input.resolve_store,
                clock: if matches!(
                    case,
                    Case::Clock | Case::Deadline | Case::CancellationCrossing
                ) {
                    &MissingClock
                } else {
                    &clock
                },
                maximum_record_bytes: if matches!(case, Case::NoExpiry) {
                    1
                } else {
                    65_536
                },
            },
            &mut ProvingAuthority,
            store,
            anchor,
        )
        .expect_err("the actual native call remains unopened");
    match case {
        Case::Clock | Case::Deadline | Case::CancellationCrossing => {
            assert!(matches!(refused.reason(), OperationAttemptError::Clock(_)))
        }
        Case::Backward | Case::CallerFloor | Case::RejectedCrossing => {
            assert!(matches!(refused.reason(), OperationAttemptError::TimeOrder))
        }
        Case::Revocation => assert!(matches!(
            refused.reason(),
            OperationAttemptError::Session(_)
        )),
        Case::NoExpiry => assert!(matches!(
            refused.reason(),
            OperationAttemptError::Custody(_)
        )),
        _ => assert!(matches!(
            refused.reason(),
            OperationAttemptError::Association
        )),
    }
    if matches!(case, Case::Deadline) {
        refused = refused.assert_elapsed_budget_retained(
            NativeOperationExecutionContext {
                current: input.current,
                resolve_store: input.resolve_store,
                clock: &MissingClock,
                maximum_record_bytes: 65_536,
            },
            &mut ProvingAuthority,
            store,
            anchor,
        );
    }
    if matches!(case, Case::CancellationCrossing) {
        let crossing = Clock {
            times: [80; 3],
            reads: AtomicUsize::new(0),
        };
        let cancellation = |clock| NativeOperationRefusalCancellationInput {
            current: input.current,
            signing,
            diagnostic_ref: record_ref("native-shell/unused-cancellation-diagnostic"),
            observation_ref: record_ref("native-shell/unused-cancelled"),
            maximum_diagnostic_bytes: 65_536,
            maximum_record_bytes: 65_536,
            clock,
            resolve_store: input.resolve_store,
        };
        let failed = refused
            .prepare_cancellation(
                cancellation(&crossing),
                &mut ProvingAuthority,
                store,
                anchor,
                |_, _, _, _| Ok(()),
                |_, _, _| panic!("expired authority cannot sign cancellation"),
            )
            .err()
            .unwrap();
        assert!(matches!(
            failed.reason(),
            OperationAttemptError::Association
        ));
        assert_eq!(crossing.reads.load(Ordering::SeqCst), 1);
        refused = failed.into_invocation_refusal();
        assert!(
            matches!(refused.reason(), OperationAttemptError::Clock(_)),
            "original non-temporal reason remains unchanged"
        );
        let failed = refused
            .prepare_cancellation(
                cancellation(&MissingClock),
                &mut ProvingAuthority,
                store,
                anchor,
                |_, _, _, _| Ok(()),
                |_, _, _| panic!("missing clock cannot sign cancellation"),
            )
            .err()
            .unwrap();
        assert!(matches!(failed.reason(), OperationAttemptError::Clock(_)));
        refused = failed.into_invocation_refusal();
    }
    let record_time = if matches!(case, Case::Deadline) {
        68
    } else {
        sample.max(if matches!(case, Case::RejectedCrossing) {
            90
        } else {
            80
        })
    };
    let selected = || NativeOperationExpiryInput {
        signing,
        diagnostic_ref: record_ref("native-shell/unopened-expiry-diagnostic"),
        observation_ref: record_ref("native-shell/unopened-expired"),
        observed_at: UnixTimeV1::new(record_time).unwrap(),
        maximum_diagnostic_bytes: 65_536,
        maximum_record_bytes: 65_536,
        resolve_store: input.resolve_store,
    };
    let reads = clock.reads.load(Ordering::SeqCst);
    if !matches!(
        case,
        Case::Handle | Case::LeaseAndHandle | Case::Deadline | Case::CancellationCrossing
    ) {
        let failed = refused
            .prepare_expiry(
                selected(),
                store,
                anchor,
                |_, _, _, _| panic!("unproven expiry cannot admit rotation"),
                |_, _, _| panic!("unproven expiry cannot sign"),
            )
            .err()
            .expect("failure retains the original unopened call");
        assert!(
            if matches!(case, Case::Replay | Case::Recovered | Case::ChangedHead) {
                matches!(failed.reason(), OperationAttemptError::Custody(_))
            } else {
                matches!(failed.reason(), OperationAttemptError::Association)
            }
        );
        assert_eq!(
            clock.reads.load(Ordering::SeqCst),
            reads,
            "recording cannot invent an expiry clock sample"
        );
        return;
    }
    // A later recording timestamp cannot erase the original floor/high-water.
    let mut early = selected();
    early.observed_at = UnixTimeV1::new(if matches!(case, Case::Deadline) {
        63
    } else {
        sample - 1
    })
    .unwrap();
    let failed = refused
        .prepare_expiry(
            early,
            store,
            anchor,
            |_, _, _, _| Ok(()),
            |_, _, _| panic!("time anomaly cannot sign"),
        )
        .err()
        .unwrap();
    assert!(matches!(failed.reason(), OperationAttemptError::TimeOrder));
    let mut late = selected();
    late.observed_at = UnixTimeV1::new(121).unwrap();
    let failed = failed
        .into_invocation_refusal()
        .prepare_expiry(
            late,
            store,
            anchor,
            |_, _, _, _| Ok(()),
            |_, _, _| panic!("past-retention evidence cannot sign"),
        )
        .err()
        .unwrap();
    assert!(matches!(
        failed.reason(),
        OperationAttemptError::RetentionElapsed
    ));
    let expired = failed
        .into_invocation_refusal()
        .prepare_expiry(
            selected(),
            store,
            anchor,
            |_, _, _, _| Ok(()),
            |_, _, _| Ok(signature()),
        )
        .unwrap_or_else(|failure| {
            panic!(
                "{case:?}: expiry preparation refused: {:?}",
                failure.reason()
            )
        });
    assert_eq!(
        expired.observation().1.predecessor_ref().as_ref(),
        Some(&predecessor)
    );
    assert_eq!(
        expired.observation().1.predecessor_sha256().as_ref(),
        Some(&predecessor_digest)
    );
    assert!(matches!(
        expired.observation().1.state(),
        OperationObservationStateV1::Expired { .. }
    ));
    assert_eq!(
        expired.diagnostic().wire().code,
        "native_operation_expired_before_release"
    );
    assert_eq!(
        expired.diagnostic().wire().effect,
        DiagnosticEffectCertaintyV1::NotApplicable
    );
    assert_eq!(
        expired.diagnostic().wire().disposition,
        DiagnosticDispositionV1::Blocked
    );
    assert_eq!(
        expired.diagnostic().wire().retry,
        DiagnosticRetryV1::NotIndicated
    );
    let cause = match case {
        Case::Handle | Case::CancellationCrossing => "operation handle expired",
        Case::LeaseAndHandle => "operation lease expired",
        Case::Deadline => "operation timeout elapsed",
        _ => unreachable!("only retained expiry evidence reaches signing"),
    };
    assert!(expired.diagnostic().wire().summary.starts_with(cause));
    assert!(matches!(
        expired
            .prepare_append(input.resolve_store)
            .unwrap()
            .associated_record(),
        Some(OperationCustodyRecord::Diagnostic(_))
    ));
    assert_eq!(clock.reads.load(Ordering::SeqCst), reads);
}
