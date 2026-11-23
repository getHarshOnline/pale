//! Cancel an actual unopened call, commit its evidence, and reopen native custody.
//! Signing and authority remain fixture models; physical read-back is not public Observe.

use super::*;

pub(super) struct Context<'a> {
    pub(super) execution: NativeOperationExecutionContext<'a>,
    pub(super) signing: SignedResidentAdmissionContextV1<'a>,
    pub(super) operation: &'a IssuedLocalOperation,
    pub(super) checkpoint: &'a ControlStateCheckpointV1,
}

pub(super) struct Evidence {
    pub(super) candidate: OperationCustodyCandidate,
    pub(super) checkpoint: ControlStateCheckpointPreimageV1,
    pub(super) observation: OperationObservationV1,
}

struct CancellationClock;
impl WallClockSourcePort for CancellationClock {
    fn observe_unix_time(&self) -> Result<UnixTimeV1, WallClockSourceError> {
        Ok(UnixTimeV1::new(68).unwrap())
    }
}

pub(super) fn run(
    dispatch: PreparedNativeOperationDispatch<'_, '_>,
    completed: CompletedOperationObservationCustody,
    context: Context<'_>,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
) -> Evidence {
    let prior = completed
        .candidate()
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
                clock: context.execution.clock,
                maximum_record_bytes: 1,
            },
            &mut ProvingAuthority,
            port,
            anchor,
        )
        .expect_err("actual bounded read blocks release");
    assert!(matches!(
        refused.reason(),
        OperationAttemptError::Custody(_)
    ));
    let selected = || NativeOperationRefusalCancellationInput {
        current: context.execution.current,
        signing: context.signing,
        diagnostic_ref: record_ref("native/refusal-cancel-diagnostic"),
        observation_ref: record_ref("native/refusal-cancelled"),
        maximum_diagnostic_bytes: 65_536,
        maximum_record_bytes: 65_536,
        clock: &CancellationClock,
        resolve_store: context.execution.resolve_store,
    };
    for fault in 0..4 {
        let mut input = selected();
        match fault {
            0 => input.diagnostic_ref = dispatch_ref.clone(),
            1 => input.observation_ref = dispatch_ref.clone(),
            2 => input.maximum_diagnostic_bytes = 1,
            _ => input.maximum_record_bytes = 1,
        }
        let failed = refused
            .prepare_cancellation(
                input,
                &mut ProvingAuthority,
                port,
                anchor,
                |_, _, _, _| Ok(()),
                |_, _, _| panic!("invalid input cannot sign"),
            )
            .err()
            .expect("failed preparation retains original call");
        assert!(match fault {
            0 | 1 => matches!(failed.reason(), OperationAttemptError::Association),
            2 => matches!(failed.reason(), OperationAttemptError::BoundExceeded),
            _ => matches!(failed.reason(), OperationAttemptError::Custody(_)),
        });
        assert_eq!(port.connection.total_changes(), writes);
        refused = failed.into_invocation_refusal();
    }
    let failed = refused
        .prepare_cancellation(
            selected(),
            &mut ProvingAuthority,
            port,
            anchor,
            |_, _, _, _| Ok(()),
            |_, _, _| {
                Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "cancellation test",
                    invariant: "signer temporarily unavailable",
                })
            },
        )
        .err()
        .expect("failed signing retains unopened call");
    assert_eq!(port.connection.total_changes(), writes);
    let cancelled = failed
        .into_invocation_refusal()
        .prepare_cancellation(
            selected(),
            &mut ProvingAuthority,
            port,
            anchor,
            |_, _, _, _| Ok(()),
            |_, _, _| Ok(signature()),
        )
        .unwrap();
    assert_eq!(
        port.connection.total_changes(),
        writes,
        "preparation is read-only"
    );
    let observation = cancelled.observation().1.clone();
    assert!(matches!(
        observation.state(),
        OperationObservationStateV1::Cancelled { .. }
    ));
    assert_eq!(observation.predecessor_ref().as_ref(), Some(&dispatch_ref));
    assert_eq!(
        observation.predecessor_sha256().as_ref(),
        Some(&dispatch_digest)
    );
    let (physical, fields) = prepare_append(
        cancelled
            .prepare_append(context.execution.resolve_store)
            .unwrap(),
        context.operation,
        (&prior, context.checkpoint),
        port,
        69,
    );
    // Successful preparation consumed the refusal and discarded its unopened
    // handler. Detaching this candidate cannot leave a call available to recheck.
    drop(cancelled);
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
    assert!(candidate
        .record(OperationCustodyRecordKind::CapabilityResult)
        .is_none());
    assert_eq!(candidate.records().len(), 2);
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

pub(super) fn read_back(
    evidence: Evidence,
    file: &std::path::Path,
    coordinate: &AuthorityStoreCoordinate,
) {
    let mut sql = connection(file);
    sql.pragma_update(None, "query_only", true).unwrap();
    let writes = sql.total_changes();
    let mut port = SqliteOperationCustody {
        connection: &mut sql,
        coordinate: coordinate.clone(),
    };
    assert_eq!(
        port.observe_exact(&evidence.candidate).unwrap(),
        OperationCustodyReadBackOutcome::ExactSuccessor
    );
    let (diagnostic, code) = match evidence.observation.state() {
        OperationObservationStateV1::Cancelled { diagnostic } => {
            (diagnostic, "native_operation_cancelled_before_release")
        }
        OperationObservationStateV1::Expired { diagnostic } => {
            (diagnostic, "native_operation_expired_before_release")
        }
        _ => panic!("the same committed unopened-call disposition must remain selected"),
    };
    for item in evidence.candidate.records() {
        assert_eq!(
            port.read_operation_record(coordinate, item.record().coordinate(), 65_536)
                .unwrap()
                .as_ref(),
            Some(item.record())
        );
    }
    let stored = evidence
        .candidate
        .record(OperationCustodyRecordKind::Diagnostic)
        .unwrap();
    assert_eq!(stored.coordinate().record_ref(), &diagnostic.record_ref);
    assert_eq!(
        ArtifactDigest::hash_bytes(stored.canonical_record_jcs().as_slice()),
        diagnostic.record_sha256
    );
    let admitted = pale_application::diagnostic::AdmittedDiagnostic::from_bytes(
        stored.canonical_record_jcs().as_slice(),
    )
    .unwrap();
    assert_eq!(admitted.wire().code, code);
    assert_eq!(
        admitted.wire().effect,
        DiagnosticEffectCertaintyV1::NotApplicable
    );
    let checkpoint = read_checkpoint(
        evidence.candidate.checkpoint().successor_record(),
        &evidence.checkpoint,
    );
    assert_eq!(
        port.read_checkpoint_head(coordinate, checkpoint.checkpoint_domain_id(), 65_536)
            .unwrap(),
        Some((
            evidence.candidate.checkpoint().successor().clone(),
            evidence.candidate.checkpoint().successor_record().clone()
        ))
    );
    assert_eq!(
        port.connection.total_changes(),
        writes,
        "reopened unopened-call evidence read-back cannot write"
    );
}
