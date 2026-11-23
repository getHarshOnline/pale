//! One-use release refusal over the existing issued-start and custody fixture.

use super::*;
use crate::resident::{
    CompletedOperationObservationCustody, NativeOperationInvocationRefusal,
    PreparedNativeOperationDispatch, UnstagedNativeOperationOutcome,
};
use pale_ports::{AntiRollbackAnchorPort, OperationCustodyPort};

struct UnavailableClock;

impl WallClockSourcePort for UnavailableClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        Err(WallClockSourceError::from_source(std::io::Error::other(
            "controlled clock refusal",
        )))
    }
}

struct UnavailableStore;

impl OperationObservationAuthorityStoreResolver for UnavailableStore {
    fn resolve(
        &self,
        _: &OperationObservationV1,
    ) -> Result<AuthorityStoreCoordinate, OperationObservationAuthorityStoreRejection> {
        Err(OperationObservationAuthorityStoreRejection)
    }
}

struct UnusedClock;

impl WallClockSourcePort for UnusedClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        panic!("closed temporal refusal cannot begin another validation pass")
    }
}

pub(super) fn invoke<'operation, 'registry>(
    case: Case,
    dispatch: PreparedNativeOperationDispatch<'operation, 'registry>,
    completed: CompletedOperationObservationCustody,
    input: NativeOperationExecutionContext<'_>,
    store: &mut dyn OperationCustodyPort,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> Result<UnstagedNativeOperationOutcome, NativeOperationInvocationRefusal<'operation, 'registry>>
{
    let repeat = |clock, current| NativeOperationExecutionContext {
        current,
        resolve_store: input.resolve_store,
        clock,
        maximum_record_bytes: input.maximum_record_bytes,
    };
    let special = matches!(
        case,
        Case::ReleaseClockRecovery
            | Case::ReleaseProjectionRecovery
            | Case::ReleaseHighWater
            | Case::ReleaseTimeFloor
            | Case::ReleaseElapsedBudget
            | Case::ReleaseCurrentAuthority
    );
    if !special {
        let result = dispatch.invoke(
            completed,
            repeat(input.clock, input.current),
            &mut ProvingAuthority,
            store,
            anchor,
        );
        if matches!(
            case,
            Case::ExpiredBeforeRead
                | Case::BackwardBeforeRead
                | Case::ExpiredAfterAuthority
                | Case::BackwardAfterAuthority
        ) {
            let refused = result.expect_err("time refusal retains unopened call");
            let reason = format!("{:?}", refused.reason());
            let refused = refused
                .recheck(
                    repeat(&UnusedClock, input.current),
                    &mut ProvingAuthority,
                    store,
                    anchor,
                )
                .expect_err("observed temporal refusal cannot be erased");
            assert_eq!(format!("{:?}", refused.reason()), reason);
            return Err(refused);
        }
        if matches!(
            case,
            Case::Replay
                | Case::RecoveredCommit
                | Case::HandlerFailureReplay
                | Case::WrongCompletion
        ) {
            let refused = result.expect_err("non-new custody refuses release");
            let clock = Clock {
                times: [67, 67, 67],
                reads: AtomicUsize::new(0),
            };
            let refused = refused
                .recheck(
                    repeat(&clock, input.current),
                    &mut ProvingAuthority,
                    store,
                    anchor,
                )
                .expect_err("re-entry cannot promote replay or recovered uncertainty");
            use crate::resident::OperationDispatchCustodyError as Error;
            assert!(matches!(
                (case, refused.reason()),
                (
                    Case::WrongCompletion,
                    OperationAttemptError::Custody(Error::Association)
                ) | (
                    Case::Replay | Case::RecoveredCommit | Case::HandlerFailureReplay,
                    OperationAttemptError::Custody(Error::NotNewCommit)
                )
            ));
            assert_eq!(clock.reads.load(Ordering::SeqCst), 1);
            return Err(refused);
        }
        return result;
    }
    let observed = if matches!(case, Case::ReleaseHighWater) {
        67
    } else {
        65
    };
    let clock = Clock {
        times: [observed; 3],
        reads: AtomicUsize::new(0),
    };
    let later = input.current.at_time(UnixTimeV1::new(67).unwrap());
    let mut initial = repeat(&UnavailableClock, input.current);
    if matches!(
        case,
        Case::ReleaseProjectionRecovery | Case::ReleaseHighWater
    ) {
        initial.clock = &clock;
        initial.resolve_store = &UnavailableStore;
    }
    if matches!(case, Case::ReleaseTimeFloor) {
        initial.current = &later;
    }
    let refused = dispatch
        .invoke(completed, initial, &mut ProvingAuthority, store, anchor)
        .expect_err("controlled pre-handler refusal");
    assert_eq!(
        format!("{refused:?}"),
        "NativeOperationInvocationRefusal { retained: .. }"
    );
    assert_eq!(refused.to_string(), "native operation was not released");
    match case {
        Case::ReleaseElapsedBudget => Err(refused.assert_elapsed_budget_retained(
            repeat(&UnusedClock, input.current),
            &mut ProvingAuthority,
            store,
            anchor,
        )),
        Case::ReleaseHighWater | Case::ReleaseTimeFloor => {
            let refused = refused
                .recheck(
                    repeat(input.clock, input.current),
                    &mut ProvingAuthority,
                    store,
                    anchor,
                )
                .expect_err("older replacement time cannot erase the original floor/high-water");
            assert!(matches!(refused.reason(), OperationAttemptError::TimeOrder));
            let refused = refused
                .recheck(
                    repeat(&UnusedClock, &later),
                    &mut ProvingAuthority,
                    store,
                    anchor,
                )
                .expect_err("time-order refusal remains closed");
            assert!(matches!(refused.reason(), OperationAttemptError::TimeOrder));
            Err(refused)
        }
        Case::ReleaseCurrentAuthority => {
            let mut refused = refused;
            for missing in 0..4 {
                let mut absent = input.current.at_time(input.current.now);
                match missing {
                    0 => absent.grant = None,
                    1 => absent.runtime_authority = None,
                    2 => absent.current_principal_trust = None,
                    _ => absent.capability_revocations = None,
                }
                let clock = Clock {
                    times: [65; 3],
                    reads: AtomicUsize::new(0),
                };
                refused = refused
                    .recheck(
                        NativeOperationExecutionContext {
                            current: &absent,
                            resolve_store: input.resolve_store,
                            clock: &clock,
                            maximum_record_bytes: input.maximum_record_bytes,
                        },
                        &mut ProvingAuthority,
                        store,
                        anchor,
                    )
                    .expect_err("retaining a call cannot supply missing current authority");
                use crate::resident::AmbientSessionAdmissionError as Error;
                assert!(matches!(
                    (missing, refused.reason()),
                    (0, OperationAttemptError::Session(Error::GrantAbsent))
                        | (
                            1,
                            OperationAttemptError::Session(Error::RuntimeAuthorityAbsent)
                        )
                        | (
                            2,
                            OperationAttemptError::Session(Error::PrincipalTrustCheckpointAbsent)
                        )
                        | (
                            3,
                            OperationAttemptError::Session(
                                Error::CapabilityRevocationCheckpointAbsent
                            )
                        )
                ));
                assert_eq!(clock.reads.load(Ordering::SeqCst), 1);
            }
            // Restore only the exact originally resolved records. This must run
            // the original handler once through every release check.
            refused.recheck(
                repeat(input.clock, input.current),
                &mut ProvingAuthority,
                store,
                anchor,
            )
        }
        _ => refused.recheck(
            repeat(input.clock, input.current),
            &mut ProvingAuthority,
            store,
            anchor,
        ),
    }
}
