//! Cancellation consumes an unopened call; failed preparation preserves it.
//! Storage, signing and authority use the existing finite application fixture.

use super::*;
use crate::resident::{
    CompletedOperationObservationCustody, NativeOperationRefusalCancellationInput,
    PreparedNativeOperationDispatch,
};
use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError,
    AntiRollbackAnchorKey, AntiRollbackAnchorObservation, AntiRollbackAnchorPort,
    OperationCustodyPort, PreparedAntiRollbackAnchorTransition,
};

// The existing fixture still supplies every custody read and the exact anchor.
// This fault models loss of independently retained confirmation after dispatch;
// it does not model a native anchor backend or invent a replacement checkpoint.
struct MissingRetainedAnchor<'a> {
    inner: &'a mut dyn AntiRollbackAnchorPort,
    reads: usize,
}

impl AntiRollbackAnchorPort for MissingRetainedAnchor<'_> {
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        assert!(self.inner.read(key)?.is_some());
        self.reads += 1;
        Ok(None)
    }

    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        panic!("cancellation preparation cannot advance an anchor")
    }

    fn observe(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        panic!("cancellation preparation reads the exact retained anchor")
    }
}

struct MissingClock;
impl WallClockSourcePort for MissingClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        Err(WallClockSourceError::from_source(std::io::Error::other(
            "test clock unavailable",
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
    let dispatch_ref = dispatch.observation().0.clone();
    let dispatch_digest = dispatch
        .observation()
        .1
        .canonical_operation_observation_sha256()
        .clone();
    let mut refused = dispatch
        .invoke(
            completed,
            NativeOperationExecutionContext {
                current: input.current,
                resolve_store: input.resolve_store,
                clock: &MissingClock,
                maximum_record_bytes: input.maximum_record_bytes,
            },
            &mut ProvingAuthority,
            store,
            anchor,
        )
        .expect_err("unopened clock refusal");
    if matches!(case, Case::CancelRefusedDeadline) {
        refused = refused.assert_elapsed_budget_retained(
            NativeOperationExecutionContext {
                current: input.current,
                resolve_store: input.resolve_store,
                clock: &MissingClock,
                maximum_record_bytes: input.maximum_record_bytes,
            },
            &mut ProvingAuthority,
            store,
            anchor,
        );
    }
    let input_for = |clock| NativeOperationRefusalCancellationInput {
        current: input.current,
        signing,
        diagnostic_ref: record_ref("native-shell/refusal-cancel-diagnostic"),
        observation_ref: record_ref("native-shell/refusal-cancelled"),
        maximum_diagnostic_bytes: 65_536,
        maximum_record_bytes: 65_536,
        clock,
        resolve_store: input.resolve_store,
    };
    let sample = if matches!(case, Case::CancelRefusedExpired) {
        80
    } else {
        65
    };
    let clock = Clock {
        times: if matches!(case, Case::CancelRefusedExpiresAfterReads) {
            [65, 66, 80]
        } else {
            [sample; 3]
        },
        reads: AtomicUsize::new(0),
    };
    if matches!(case, Case::CancelRefusedMissingAnchor) {
        let mut missing = MissingRetainedAnchor {
            inner: anchor,
            reads: 0,
        };
        let failed = refused
            .prepare_cancellation(
                input_for(&clock),
                &mut ProvingAuthority,
                store,
                &mut missing,
                |_, _, _, _| panic!("missing anchor cannot admit rotation"),
                |_, _, _| panic!("missing anchor cannot sign cancellation"),
            )
            .err()
            .expect("historical completion cannot replace current anchor confirmation");
        assert!(matches!(
            failed.reason(),
            OperationAttemptError::Custody(
                crate::resident::OperationDispatchCustodyError::AnchorChanged
            )
        ));
        assert_eq!(missing.reads, 1);
        assert_eq!(clock.reads.load(Ordering::SeqCst), 1);
        assert!(matches!(
            failed.into_invocation_refusal().reason(),
            OperationAttemptError::Clock(_)
        ));
        return;
    }
    if matches!(
        case,
        Case::CancelRefusedReplay
            | Case::CancelRefusedRecovered
            | Case::CancelRefusedChangedHead
            | Case::CancelRefusedExpired
            | Case::CancelRefusedExpiresAfterReads
    ) {
        let failed = refused
            .prepare_cancellation(
                input_for(&clock),
                &mut ProvingAuthority,
                store,
                anchor,
                |_, _, _, _| panic!("rejected before rotation"),
                |_, _, _| panic!("rejected before signing"),
            )
            .err()
            .expect("retained refusal");
        if matches!(
            case,
            Case::CancelRefusedExpired | Case::CancelRefusedExpiresAfterReads
        ) {
            assert!(matches!(
                failed.reason(),
                OperationAttemptError::Association
            ));
            let refused = failed.into_invocation_refusal();
            let earlier = Clock {
                times: [65; 3],
                reads: AtomicUsize::new(0),
            };
            let failed = refused
                .prepare_cancellation(
                    input_for(&earlier),
                    &mut ProvingAuthority,
                    store,
                    anchor,
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                )
                .err()
                .expect("expired high-water cannot be erased");
            assert!(matches!(failed.reason(), OperationAttemptError::TimeOrder));
        } else {
            assert!(matches!(failed.reason(), OperationAttemptError::Custody(_)));
        }
        return;
    }
    // A missing authority input must return this same unopened call before I/O.
    let authority_clock = Clock {
        times: [65; 3],
        reads: AtomicUsize::new(0),
    };
    {
        let mut selected = input_for(&authority_clock);
        let mut missing = input.current.at_time(input.current.now);
        missing.grant = None;
        selected.current = &missing;
        let failed = refused
            .prepare_cancellation(
                selected,
                &mut ProvingAuthority,
                store,
                anchor,
                |_, _, _, _| Ok(()),
                |_, _, _| panic!("missing authority cannot sign"),
            )
            .err()
            .expect("failed preparation retains call");
        assert!(matches!(failed.reason(), OperationAttemptError::Session(_)));
        assert_eq!(
            format!("{failed:?}"),
            "NativeOperationCancellationRefusal { retained: .. }"
        );
        refused = failed.into_invocation_refusal();
    }
    if matches!(case, Case::CancelRefusedHighwater) {
        let high = Clock {
            times: [67; 3],
            reads: AtomicUsize::new(0),
        };
        refused = refused
            .prepare_cancellation(
                input_for(&high),
                &mut ProvingAuthority,
                store,
                anchor,
                |_, _, _, _| Ok(()),
                |_, _, _| {
                    Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "test",
                        invariant: "signer",
                    })
                },
            )
            .err()
            .unwrap()
            .into_invocation_refusal();
        let low = Clock {
            times: [66; 3],
            reads: AtomicUsize::new(0),
        };
        let failed = refused
            .prepare_cancellation(
                input_for(&low),
                &mut ProvingAuthority,
                store,
                anchor,
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(signature()),
            )
            .err()
            .expect("lower replacement cannot erase observed time");
        assert!(matches!(failed.reason(), OperationAttemptError::TimeOrder));
        let forward = Clock {
            times: [68; 3],
            reads: AtomicUsize::new(0),
        };
        assert!(
            failed
                .into_invocation_refusal()
                .recheck(
                    NativeOperationExecutionContext {
                        current: input.current,
                        resolve_store: input.resolve_store,
                        clock: &forward,
                        maximum_record_bytes: input.maximum_record_bytes,
                    },
                    &mut ProvingAuthority,
                    store,
                    anchor
                )
                .is_err(),
            "cancellation failure never thaws a closed invocation"
        );
        assert_eq!(forward.reads.load(Ordering::SeqCst), 0);
        return;
    }
    let final_clock = Clock {
        times: [68; 3],
        reads: AtomicUsize::new(0),
    };
    let cancelled = refused
        .prepare_cancellation(
            input_for(&final_clock),
            &mut ProvingAuthority,
            store,
            anchor,
            |_, _, _, _| Ok(()),
            |domain, _, _| {
                assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
                Ok(signature())
            },
        )
        .unwrap();
    // The original refusal has been consumed and its unopened handler dropped.
    // The cancellation can now detach an append, but cannot release that call.
    assert_eq!(
        cancelled.observation().1.predecessor_ref().as_ref(),
        Some(&dispatch_ref)
    );
    assert_eq!(
        cancelled.observation().1.predecessor_sha256().as_ref(),
        Some(&dispatch_digest)
    );
    assert_eq!(
        *cancelled.observation().1.observed_at(),
        UnixTimeV1::new(68).unwrap()
    );
    assert!(matches!(
        cancelled.observation().1.state(),
        OperationObservationStateV1::Cancelled { .. }
    ));
    assert_eq!(
        cancelled.diagnostic().wire().code,
        "native_operation_cancelled_before_release"
    );
    assert_eq!(
        cancelled.diagnostic().wire().effect,
        DiagnosticEffectCertaintyV1::NotApplicable
    );
    assert!(matches!(
        cancelled
            .prepare_append(input.resolve_store)
            .unwrap()
            .associated_record(),
        Some(OperationCustodyRecord::Diagnostic(_))
    ));
}
