//! Cancellation read-back uses the exact historical local request, never a current-state fiction.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    history: &history::Input<'_>,
    inputs: &fixture::Inputs,
    binding: &A2aTaskBindingV1,
    route: &A2aRouteAdmissionV1,
    remote: (&ResidentRecordRefV1, &A2aTaskObservationV1),
    prior: (&OperationCustodyCandidate, &ControlStateCheckpointV1),
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
    restart: Restart<'_>,
) -> OperationCustodyCandidate {
    let coordinate = port.coordinate.clone();
    let resolve = |_: &OperationObservationV1| Ok(coordinate.clone());
    let operation = history.operation;
    let request = raw(
        "a2a/cancel-request",
        b"{ \"method\":\"CancelTask\", \"id\":\"cancel-1\" }\n",
    );
    let request_ref = fixture::evidence("a2a/cancel-request", request.bytes());
    let canceled_ref = record_ref("a2a/local-cancellation-requested");
    let canceled = issue_observation(
        history.signing,
        operation.handle().0,
        operation.handle().1,
        history.selected_member.1.schema_sha256().clone(),
        history
            .selected_member
            .1
            .revision()
            .checked_successor()
            .unwrap(),
        Some(history.selected_member),
        OperationObservationStateV1::CancellationRequested {
            request: request_ref.clone(),
        },
        103,
    );
    let append = prepare_operation_observation_append(
        OperationObservationTransition::new(
            operation.handle().0,
            operation.handle().1,
            history.selected_member.0,
            history.selected_member.1,
            &canceled_ref,
            &canceled,
        ),
        &resolve,
    )
    .unwrap();
    let (prepared, fields) = prepare_append(
        append,
        operation,
        (
            prior.0.checkpoint().successor().coordinate().record_ref(),
            prior.1,
        ),
        port,
        103,
    );
    let done = prepared
        .commit(port, anchor)
        .unwrap()
        .advance_anchor(anchor)
        .unwrap();
    let checkpoint = read_checkpoint(done.candidate().checkpoint().successor_record(), &fields);
    // The same request recurs after the remote observation's time (105).
    // Recovery must retain cancellation 103, not the later matching request 106.
    let repeated_ref = record_ref("a2a/local-cancellation-repeated");
    let repeated = issue_observation(
        history.signing,
        operation.handle().0,
        operation.handle().1,
        canceled.schema_sha256().clone(),
        canceled.revision().checked_successor().unwrap(),
        Some((&canceled_ref, &canceled)),
        canceled.state().clone(),
        106,
    );
    let append = prepare_operation_observation_append(
        OperationObservationTransition::new(
            operation.handle().0,
            operation.handle().1,
            &canceled_ref,
            &canceled,
            &repeated_ref,
            &repeated,
        ),
        &resolve,
    )
    .unwrap();
    let (prepared, fields) = prepare_append(
        append,
        operation,
        (
            done.candidate()
                .checkpoint()
                .successor()
                .coordinate()
                .record_ref(),
            &checkpoint,
        ),
        port,
        106,
    );
    let done = prepared
        .commit(port, anchor)
        .unwrap()
        .advance_anchor(anchor)
        .unwrap();
    let checkpoint = read_checkpoint(done.candidate().checkpoint().successor_record(), &fields);
    let unknown_ref = record_ref("a2a/local-unknown");
    let unknown = issue_observation(
        history.signing,
        operation.handle().0,
        operation.handle().1,
        canceled.schema_sha256().clone(),
        repeated.revision().checked_successor().unwrap(),
        Some((&repeated_ref, &repeated)),
        OperationObservationStateV1::OutcomeUnknown {
            diagnostic: fixture::evidence(
                "a2a/unknown-diagnostic-model",
                b"fixture uncertainty evidence",
            ),
            reconciliation: ReconciliationFrontierReferenceV1 {
                record_ref: record_ref("a2a/reconciliation-model"),
                record_sha256: ReconciliationFrontierDigest::parse("a".repeat(64)).unwrap(),
            },
            effect_receipt: None,
        },
        107,
    );
    let append = prepare_operation_observation_append(
        OperationObservationTransition::new(
            operation.handle().0,
            operation.handle().1,
            &repeated_ref,
            &repeated,
            &unknown_ref,
            &unknown,
        ),
        &resolve,
    )
    .unwrap();
    let (prepared, fields) = prepare_append(
        append,
        operation,
        (
            done.candidate()
                .checkpoint()
                .successor()
                .coordinate()
                .record_ref(),
            &checkpoint,
        ),
        port,
        107,
    );
    let done = prepared
        .commit(port, anchor)
        .unwrap()
        .advance_anchor(anchor)
        .unwrap();
    let checkpoint = read_checkpoint(done.candidate().checkpoint().successor_record(), &fields);
    let mut known = history.known.to_vec();
    known.push((&canceled_ref, &canceled));
    known.push((&repeated_ref, &repeated));
    known.push((&unknown_ref, &unknown));
    let proof = history::recover(
        history::Input {
            operation,
            coordinate: &coordinate,
            selected_member: (&canceled_ref, &canceled),
            known: &known,
            checkpoint: &fields,
            signing: history.signing,
        },
        port,
        anchor,
    );
    assert_eq!(proof.selected(), (&canceled_ref, &canceled));
    assert_eq!(proof.selected_head(), (&unknown_ref, &unknown));
    let response = raw(
        "a2a/cancel-response",
        b"{ \"fixture-state\":\"Completed\" }\n",
    );
    let source = A2aTaskObservationSourceV1::CancelTaskReadBack {
        request: request_ref,
        response: fixture::evidence("a2a/cancel-response", response.bytes()),
    };
    let next = successor(inputs, remote, source, A2aRemoteTaskStateV1::Completed, 105);
    let context = A2aTaskObservationAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &inputs.observation.schema_sha256,
            signing_authority: history.signing,
        },
        task_binding_ref: &inputs.binding_ref,
        task_binding: binding,
        route_admission_ref: &inputs.route_ref,
        route_admission: route,
        operation_handle_ref: operation.handle().0,
        operation_handle: operation.handle().1,
        operation_observation: &canceled,
        predecessor: Some(remote),
    };
    let selected = |observation, request| A2aTaskObservationCustodyInput {
        operation_custody: &proof,
        operation_observation_ref: &canceled_ref,
        observation_ref: record_ref("a2a/cancellation-readback"),
        observation,
        context,
        new_evidence: vec![request, response.clone()],
        retained_evidence: vec![],
        maximum_evidence_bytes: 8192,
        maximum_record_bytes: 65_536,
    };
    let wrong = raw("a2a/different-cancel-request", b"another exact request");
    let mut changed = next.clone();
    changed.source = A2aTaskObservationSourceV1::CancelTaskReadBack {
        request: fixture::evidence("a2a/different-cancel-request", wrong.bytes()),
        response: fixture::evidence("a2a/cancel-response", response.bytes()),
    };
    let signs = Cell::new(0);
    let writes = port.connection.total_changes();
    assert!(prepare_a2a_task_observation_custody(
        selected(changed, wrong),
        |_, _, _| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _| {
            signs.set(signs.get() + 1);
            Ok(signature())
        },
        |_| Ok(coordinate.clone())
    )
    .is_err());
    assert_eq!(
        signs.get(),
        0,
        "unlike cancellation request fails before signing"
    );
    assert_eq!(port.connection.total_changes(), writes);
    let append = prepare_a2a_task_observation_custody(
        selected(next.clone(), request),
        |actual, new, retained| {
            assert_eq!(actual.source, next.source);
            assert_eq!(new.len(), 2);
            assert!(retained.is_empty());
            Ok(())
        },
        |_, _, _, _| Ok(()),
        |_, _, _| Ok(signature()),
        |_| Ok(coordinate.clone()),
    )
    .unwrap();
    let remote_head = (
        append.observation().0.clone(),
        append.observation().1.clone(),
    );
    let (prepared, fields) = physical(
        append,
        operation,
        (
            done.candidate()
                .checkpoint()
                .successor()
                .coordinate()
                .record_ref(),
            &checkpoint,
        ),
        port,
        108,
        "cancel-readback",
    )
    .unwrap();
    let done = prepared
        .commit(port, anchor)
        .unwrap()
        .advance_anchor(anchor)
        .unwrap();
    let candidate = done.candidate().clone();
    assert_exact(port, &candidate);
    let local = current_local(port, operation);
    assert_eq!(local.coordinate().record_ref(), &unknown_ref);
    assert!(matches!(
        unknown.state(),
        OperationObservationStateV1::OutcomeUnknown { .. }
    ));
    assert!(candidate
        .record(OperationCustodyRecordKind::OperationObservation)
        .is_none());
    recovery::run(
        restart,
        history::Input {
            operation,
            coordinate: &coordinate,
            selected_member: history.selected_member,
            known: &known,
            checkpoint: &fields,
            signing: history.signing,
        },
        inputs,
        binding,
        route,
        (&remote_head.0, &remote_head.1),
        &candidate,
        anchor,
    );
    candidate
}
