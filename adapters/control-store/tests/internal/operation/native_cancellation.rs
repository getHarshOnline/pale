//! Either cancellation or dispatch may win the real preparation-head comparison.
//! Signatures and the independent anchor reuse the parent fixture's declared models.

use super::*;

pub(super) fn race(
    attempt: PreparedNativeOperationAttempt<'_, '_>,
    input: NativeOperationCancellationInput<'_>,
    operation: &IssuedLocalOperation,
    checkpoint: &ControlStateCheckpointV1,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
    cancellation_wins: bool,
) -> OperationCustodyCandidate {
    let prior = (
        input
            .completed
            .candidate()
            .checkpoint()
            .successor()
            .coordinate()
            .record_ref(),
        checkpoint,
    );
    let cancellation = attempt
        .prepare_cancellation(
            NativeOperationCancellationInput {
                completed: input.completed,
                current: input.current,
                signing: input.signing,
                diagnostic_ref: input.diagnostic_ref.clone(),
                observation_ref: input.observation_ref.clone(),
                maximum_diagnostic_bytes: input.maximum_diagnostic_bytes,
                resolve_store: input.resolve_store,
            },
            &mut ProvingAuthority,
            |_, _, _, _| Ok(()),
            |_, _, _| Ok(signature()),
        )
        .unwrap();
    assert!(matches!(
        cancellation.observation().1.state(),
        OperationObservationStateV1::Cancelled { .. }
    ));
    let diagnostic = cancellation
        .prepare_append(input.resolve_store)
        .unwrap()
        .associated_record()
        .unwrap()
        .record()
        .clone();
    let cancelled_observation = cancellation.observation().1.clone();
    let (cancel_custody, _) = prepare_append(
        cancellation.prepare_append(input.resolve_store).unwrap(),
        operation,
        prior,
        port,
        64,
    );
    // Both candidates use the same exact prepared head and next transaction.
    // The borrowed cancellation cannot invoke; release remains owned by dispatch.
    let dispatch = attempt
        .prepare_dispatch(
            NativeOperationDispatchContext {
                completed: input.completed,
                current: input.current,
                signing: input.signing,
                observation_ref: record_ref("native/cancel-race-dispatch"),
                resolve_store: input.resolve_store,
            },
            &mut ProvingAuthority,
            |_, _, _, _| Ok(()),
            |_, _, _| Ok(signature()),
        )
        .unwrap();
    let (dispatch_custody, _) = prepare_append(
        dispatch.prepare_append(input.resolve_store).unwrap(),
        operation,
        prior,
        port,
        64,
    );
    let (winning, losing) = if cancellation_wins {
        (cancel_custody, dispatch_custody)
    } else {
        (dispatch_custody, cancel_custody)
    };
    let completed = winning
        .commit(port, anchor)
        .unwrap()
        .advance_anchor(anchor)
        .unwrap();
    assert_eq!(
        completed.database_evidence(),
        OperationCustodyDatabaseEvidence::New
    );
    let writes = port.connection.total_changes();
    let rejected = losing
        .commit(port, anchor)
        .expect_err("the unlike successor cannot replace the winning head");
    assert!(
        rejected.try_into_retryable().is_err(),
        "a losing candidate grants no blind retry"
    );
    assert_eq!(
        port.connection.total_changes(),
        writes,
        "losing cancellation or dispatch performs no writes"
    );
    let candidate = completed.candidate();
    let observation = candidate
        .record(OperationCustodyRecordKind::OperationObservation)
        .unwrap();
    let expected = if cancellation_wins {
        &cancelled_observation
    } else {
        dispatch.observation().1
    };
    assert_eq!(
        observation.canonical_record_jcs().as_slice(),
        pale_core::canon::to_canonical_string(expected)
            .unwrap()
            .as_bytes()
    );
    assert_eq!(
        candidate.record(OperationCustodyRecordKind::Diagnostic),
        cancellation_wins.then_some(&diagnostic)
    );
    assert_eq!(
        port.read_operation_record(candidate.store(), diagnostic.coordinate(), 65_536)
            .unwrap(),
        cancellation_wins.then_some(diagnostic)
    );
    for record in candidate
        .records()
        .iter()
        .map(OperationCustodyRecord::record)
        .chain(std::iter::once(candidate.checkpoint().successor_record()))
    {
        let sequence: i64 = port.connection.query_row("SELECT inserted_transaction_sequence FROM pale_deployment_control_record WHERE record_ref=?1", [record.coordinate().record_ref().as_str()], |row| row.get(0)).unwrap();
        assert_eq!(
            u64::try_from(sequence).unwrap(),
            candidate
                .checkpoint()
                .successor()
                .committed_transaction_sequence()
                .get()
        );
    }
    // A cancelled winner has no fresh completed dispatch to pass to invoke.
    // A dispatch winner is deliberately not invoked by this race-only test.
    candidate.clone()
}
