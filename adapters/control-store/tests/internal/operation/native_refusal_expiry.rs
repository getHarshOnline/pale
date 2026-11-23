//! Actual unopened expiry across atomic native custody, rollback, replay and reopen.
//! Independent recording authority and anti-rollback anchoring remain fixture models.

use super::refusal_cancellation::{Context, Evidence};
use super::*;

struct ExpiredClock;
impl WallClockSourcePort for ExpiredClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        Ok(UnixTimeV1::new(80).unwrap())
    }
}

pub(super) fn run(
    dispatch: PreparedNativeOperationDispatch<'_, '_>,
    completed: CompletedOperationObservationCustody,
    context: Context<'_>,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
) -> Evidence {
    let original = completed.candidate().clone();
    let prior = original
        .checkpoint()
        .successor()
        .coordinate()
        .record_ref()
        .clone();
    let dispatch_ref = dispatch.observation().0.clone();
    let dispatch_digest = dispatch
        .observation()
        .1
        .canonical_operation_observation_sha256()
        .clone();
    let writes = port.connection.total_changes();
    let mut refused = dispatch
        .invoke(
            completed,
            NativeOperationExecutionContext {
                current: context.execution.current,
                resolve_store: context.execution.resolve_store,
                clock: &ExpiredClock,
                maximum_record_bytes: 65_536,
            },
            &mut ProvingAuthority,
            port,
            anchor,
        )
        .expect_err("actual handle expiry prevents opening the native handler");
    assert!(matches!(
        refused.reason(),
        OperationAttemptError::Association
    ));
    let selected = || NativeOperationExpiryInput {
        signing: context.signing,
        diagnostic_ref: record_ref("native/unopened-expiry-diagnostic"),
        observation_ref: record_ref("native/unopened-expired"),
        observed_at: UnixTimeV1::new(80).unwrap(),
        maximum_diagnostic_bytes: 65_536,
        maximum_record_bytes: 65_536,
        resolve_store: context.execution.resolve_store,
    };
    #[derive(Clone, Copy)]
    enum Fault {
        DiagnosticAlias,
        ObservationAlias,
        DiagnosticBound,
        ReadBound,
    }
    for fault in [
        Fault::DiagnosticAlias,
        Fault::ObservationAlias,
        Fault::DiagnosticBound,
        Fault::ReadBound,
    ] {
        let mut input = selected();
        match fault {
            Fault::DiagnosticAlias => input.diagnostic_ref = dispatch_ref.clone(),
            Fault::ObservationAlias => input.observation_ref = dispatch_ref.clone(),
            Fault::DiagnosticBound => input.maximum_diagnostic_bytes = 1,
            Fault::ReadBound => input.maximum_record_bytes = 1,
        }
        let failed = refused
            .prepare_expiry(
                input,
                port,
                anchor,
                |_, _, _, _| Ok(()),
                |_, _, _| panic!("invalid expiry input cannot sign"),
            )
            .err()
            .expect("preparation failure retains the original unopened call");
        assert!(match fault {
            Fault::DiagnosticAlias | Fault::ObservationAlias =>
                matches!(failed.reason(), OperationAttemptError::Association),
            Fault::DiagnosticBound =>
                matches!(failed.reason(), OperationAttemptError::BoundExceeded),
            Fault::ReadBound => matches!(failed.reason(), OperationAttemptError::Custody(_)),
        });
        refused = failed.into_invocation_refusal();
        assert_eq!(port.connection.total_changes(), writes);
    }
    let failed = refused
        .prepare_expiry(
            selected(),
            port,
            anchor,
            |_, _, _, _| Ok(()),
            |_, _, _| {
                Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "expiry fixture",
                    invariant: "signer unavailable",
                })
            },
        )
        .err()
        .expect("signing failure preserves the same refusal");
    assert_eq!(
        format!("{failed:?}"),
        "NativeOperationExpiryRefusal { retained: .. }"
    );
    let refused = failed.into_invocation_refusal();
    let refused = refused
        .recheck(
            NativeOperationExecutionContext {
                current: context.execution.current,
                resolve_store: context.execution.resolve_store,
                clock: context.execution.clock,
                maximum_record_bytes: 65_536,
            },
            &mut ProvingAuthority,
            port,
            anchor,
        )
        .expect_err("failed expiry recording cannot thaw the original closed invocation");
    let expired = refused
        .prepare_expiry(
            selected(),
            port,
            anchor,
            |_, _, _, _| Ok(()),
            |_, _, _| Ok(signature()),
        )
        .unwrap();
    assert_eq!(
        port.connection.total_changes(),
        writes,
        "expiry preparation is read-only"
    );
    let observation = expired.observation().1.clone();
    assert_eq!(observation.predecessor_ref().as_ref(), Some(&dispatch_ref));
    assert_eq!(
        observation.predecessor_sha256().as_ref(),
        Some(&dispatch_digest)
    );
    assert!(matches!(
        observation.state(),
        OperationObservationStateV1::Expired { .. }
    ));
    let append = || {
        expired
            .prepare_append(context.execution.resolve_store)
            .unwrap()
    };
    let OperationCustodyRecord::Diagnostic(diagnostic) =
        append().associated_record().unwrap().clone()
    else {
        panic!("expiry retains a diagnostic, never a capability result");
    };
    let observation_record = append().successor_record().clone();
    let (physical, fields) = prepare_append(
        append(),
        context.operation,
        (&prior, context.checkpoint),
        port,
        81,
    );
    let (replay, _) = prepare_append(
        append(),
        context.operation,
        (&prior, context.checkpoint),
        port,
        81,
    );
    let (stale, _) = prepare_append(
        append(),
        context.operation,
        (&prior, context.checkpoint),
        port,
        82,
    );
    // The actual call is gone. Only store-append values survive, even through rollback.
    drop(expired);
    port.connection.execute_batch("CREATE TEMP TRIGGER refuse_expiry BEFORE INSERT ON pale_deployment_operation_index WHEN NEW.index_role = 'observation_checkpoint' BEGIN SELECT RAISE(ABORT, 'injected expiry index failure'); END;").unwrap();
    let failed = physical
        .commit(port, anchor)
        .expect_err("injected observation index write must roll back the entire expiry append");
    let physical = failed
        .try_into_retryable()
        .expect("definitive no-mutation permits only the same store append retry");
    assert!(port
        .read_operation_record(&store_coordinate(), diagnostic.coordinate(), 65_536)
        .unwrap()
        .is_none());
    assert!(port
        .read_operation_record(&store_coordinate(), observation_record.coordinate(), 65_536)
        .unwrap()
        .is_none());
    assert_eq!(
        port.observe_exact(&original).unwrap(),
        OperationCustodyReadBackOutcome::ExactSuccessor
    );
    port.connection
        .execute_batch("DROP TRIGGER refuse_expiry;")
        .unwrap();
    let committed = physical
        .commit(port, anchor)
        .unwrap()
        .advance_anchor(anchor)
        .unwrap();
    assert_eq!(
        committed.database_evidence(),
        OperationCustodyDatabaseEvidence::New
    );
    let candidate = committed.candidate().clone();
    assert_eq!(candidate.records().len(), 2);
    assert!(candidate
        .record(OperationCustodyRecordKind::CapabilityResult)
        .is_none());
    let writes = port.connection.total_changes();
    let replayed = replay
        .commit(port, anchor)
        .unwrap()
        .advance_anchor(anchor)
        .unwrap();
    assert_eq!(
        replayed.database_evidence(),
        OperationCustodyDatabaseEvidence::ExactReplay
    );
    let refused = stale
        .commit(port, anchor)
        .expect_err("a different checkpoint cannot overwrite established expiry");
    assert!(refused.try_into_retryable().is_err());
    assert_eq!(port.connection.total_changes(), writes);
    assert_eq!(
        port.observe_exact(&candidate).unwrap(),
        OperationCustodyReadBackOutcome::ExactSuccessor
    );
    Evidence {
        candidate,
        checkpoint: fields,
        observation,
    }
}
