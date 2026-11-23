//! Operation-scoped physical discovery over actual contextual binding custody.
//! Empty discovery never establishes a remote absence or permission to resend.

use super::*;

fn scope(candidate: &OperationCustodyCandidate) -> OperationA2aBindingScope {
    let operation_key = candidate
        .indexes()
        .iter()
        .find_map(|index| match index {
            OperationCustodyIndexMutation::A2aTaskBinding { operation_key, .. } => {
                Some(operation_key.clone())
            }
            _ => None,
        })
        .unwrap();
    let handle = candidate
        .a2a_retained_records()
        .iter()
        .find(|record| record.schema_id().as_str() == OPERATION_HANDLE_V1_SCHEMA)
        .unwrap();
    OperationA2aBindingScope::new(operation_key, handle.coordinate().clone())
}

fn page(
    port: &mut SqliteOperationCustody<'_>,
    scope: &OperationA2aBindingScope,
    cursor: Option<&OperationA2aBindingCursor>,
    count: usize,
    bytes: usize,
) -> Result<OperationA2aBindings, ControlStoreError> {
    let store = port.coordinate.clone();
    let before = port.connection.total_changes();
    let result = port.read_a2a_task_bindings(&store, scope, cursor, count, bytes);
    assert_eq!(
        port.connection.total_changes(),
        before,
        "enumeration performs no custody writes"
    );
    result
}

pub(super) fn assert_no_relations(port: &mut SqliteOperationCustody<'_>) {
    let count: i64 = port
        .connection
        .query_row(
            "SELECT COUNT(*) FROM pale_deployment_operation_a2a_binding",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        count, 0,
        "failed initial atom must roll back its discovery relation too"
    );
}

pub(super) fn assert_unidentified(
    port: &mut SqliteOperationCustody<'_>,
    operation: &IssuedLocalOperation,
) {
    let store = port.coordinate.clone();
    let before = port.connection.total_changes();
    let found = discover_a2a_task_bindings(&store, operation, None, 128, 1 << 20, port).unwrap();
    assert!(found.bindings.is_empty());
    assert!(found.continuation.is_none());
    assert_eq!(port.connection.total_changes(), before);
    // This empty physical result provides neither authenticated remote absence
    // nor permission to repeat an unknown send. The reader has no send port.
}

struct BindingContext<'a> {
    history: &'a history::Input<'a>,
    route: &'a A2aRouteAdmissionV1,
    route_record: &'a ControlRecord,
}

fn prepare(
    context: &BindingContext<'_>,
    number: u64,
    binding_id: Option<&A2aTaskBindingIdV1>,
    prior: (&OperationCustodyCandidate, &ControlStateCheckpointV1),
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
) -> Result<
    (
        PreparedOperationObservationCustody,
        ControlStateCheckpointPreimageV1,
    ),
    A2aTaskCustodyError,
> {
    let operation = context.history.operation;
    let proof = super::recover(context.history, port, anchor);
    let request_name = format!("a2a-discovery/request-{number}");
    let response_name = format!("a2a-discovery/response-{number}");
    let request = raw(
        &request_name,
        format!("{{\"id\":\"discovery-{number}\",\"method\":\"SendMessage\"}}").as_bytes(),
    );
    let response = raw(
        &response_name,
        format!("{{\"task\":\"discovery-{number}\"}}").as_bytes(),
    );
    // Distinct commits may share one accepted wall-clock tick. Discovery orders
    // by physical transaction and reference, never by observation time.
    let time = 100;
    let mut inputs = fixture::Inputs::new(
        operation.handle(),
        context.history.selected_member,
        (
            context.route_record.coordinate().record_ref(),
            context.route,
        ),
        fixture::evidence(&request_name, request.bytes()),
        fixture::evidence(&response_name, response.bytes()),
        time,
    );
    inputs.binding_ref = record_ref(&format!("a2a-discovery/binding-{number}"));
    inputs.binding.task_binding_id = binding_id.cloned().unwrap_or_else(|| {
        A2aTaskBindingIdV1::parse(format!("discovered-binding-{number}")).unwrap()
    });
    inputs.binding.remote_task_id =
        A2aRemoteTaskIdV1::parse(format!("discovered-task-{number}")).unwrap();
    inputs.binding.json_rpc_request_id =
        A2aJsonRpcRequestIdV1::parse(format!("discovery-{number}")).unwrap();
    inputs.observation_ref = record_ref(&format!("a2a-discovery/observation-{number}"));
    inputs.observation.task_binding_id = inputs.binding.task_binding_id.clone();
    inputs.observation.task_binding_ref = inputs.binding_ref.clone();
    inputs.observation.task_binding_sha256 =
        A2aTaskBindingDigest::hash_canonical(&inputs.binding).unwrap();
    let append = prepare_a2a_task_binding_custody(
        A2aTaskBindingCustodyInput {
            operation_custody: &proof,
            binding_ref: inputs.binding_ref.clone(),
            binding: inputs.binding.clone(),
            binding_context: inputs.binding_context(
                operation.handle(),
                context.history.selected_member,
                context.route,
                context.history.signing,
            ),
            observation_ref: inputs.observation_ref.clone(),
            observation: inputs.observation.clone(),
            observation_signing: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &inputs.observation.schema_sha256,
                signing_authority: context.history.signing,
            },
            route_predecessor: OperationCustodyIndexPredecessor::Exact(
                context.route_record.clone(),
            ),
            route_association: A2aRouteAssociation::Retained,
            new_evidence: vec![request.clone(), response.clone()],
            retained_evidence: vec![],
            maximum_evidence_bytes: 8192,
            maximum_record_bytes: 65_536,
        },
        |binding, observation, raw, retained| {
            assert_eq!(binding, &inputs.binding);
            assert_eq!(observation, &inputs.observation);
            assert_eq!(raw, &[request.clone(), response.clone()]);
            assert!(retained.is_empty());
            Ok(()) // Explicit exchange-admission fixture, never a network-send claim.
        },
        |_, _, _| Ok(signature()),
        |_, _, _| Ok(signature()),
        |_| Ok(port.coordinate.clone()),
    )
    .unwrap();
    physical(
        append,
        operation,
        (
            prior.0.checkpoint().successor().coordinate().record_ref(),
            prior.1,
        ),
        port,
        time,
        &format!("discovery-{number}"),
    )
}

fn add(
    context: &BindingContext<'_>,
    number: u64,
    prior: (&OperationCustodyCandidate, &ControlStateCheckpointV1),
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
) -> (OperationCustodyCandidate, ControlStateCheckpointPreimageV1) {
    let (prepared, fields) = prepare(context, number, None, prior, port, anchor).unwrap();
    let completed = prepared
        .commit(port, anchor)
        .unwrap()
        .advance_anchor(anchor)
        .unwrap();
    let candidate = completed.candidate().clone();
    assert_exact(port, &candidate);
    (candidate, fields)
}

pub(super) fn extend(
    history: &history::Input<'_>,
    route: &A2aRouteAdmissionV1,
    first: (
        &OperationCustodyCandidate,
        &ControlStateCheckpointPreimageV1,
    ),
    other: &OperationCustodyCandidate,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
) -> (OperationCustodyCandidate, ControlStateCheckpointPreimageV1) {
    let selected = scope(first.0);
    let unrelated = scope(other);
    let initial = page(port, &selected, None, 128, 1 << 20).unwrap();
    assert_eq!(initial.bindings.len(), 1);
    assert_eq!(
        initial.bindings[0].binding(),
        first
            .0
            .record(OperationCustodyRecordKind::A2aTaskBinding)
            .unwrap()
    );
    let other_page = page(port, &unrelated, None, 128, 1 << 20).unwrap();
    assert_eq!(other_page.bindings.len(), 1);
    assert_eq!(
        other_page.bindings[0].binding(),
        other
            .record(OperationCustodyRecordKind::A2aTaskBinding)
            .unwrap()
    );
    assert_ne!(initial.bindings, other_page.bindings);
    let context = BindingContext {
        history,
        route,
        route_record: first
            .0
            .record(OperationCustodyRecordKind::A2aRouteAdmission)
            .unwrap(),
    };
    let checkpoint = read_checkpoint(first.0.checkpoint().successor_record(), first.1);
    let (second, fields) = add(&context, 2, (first.0, &checkpoint), port, anchor);
    let expected = page(port, &selected, None, 128, 1 << 20).unwrap().bindings;
    assert_eq!(
        expected.len(),
        2,
        "one attempt may truthfully identify multiple remote task bindings"
    );
    let prefix = page(port, &selected, None, 1, 1 << 20).unwrap();
    assert_eq!(prefix.bindings, expected[..1]);
    let cursor = prefix.continuation.unwrap();
    let checkpoint = read_checkpoint(second.checkpoint().successor_record(), &fields);
    let (third, fields) = add(&context, 3, (&second, &checkpoint), port, anchor);
    // Prior page bodies must not be reread against a later page's budget.
    let returned = expected[0].binding();
    port.connection.execute("UPDATE pale_deployment_control_record SET canonical_record_jcs=zeroblob(?1) WHERE record_ref=?2", rusqlite::params![i64::try_from(MAX_OPERATION_A2A_BINDING_BYTES + 1).unwrap(), returned.coordinate().record_ref().as_str()]).unwrap();
    let rest = page(port, &selected, Some(&cursor), 1, 1 << 20).unwrap();
    assert_eq!(rest.bindings, expected[1..]);
    assert!(
        rest.continuation.is_none(),
        "later committed binding is outside the retained prefix"
    );
    assert_eq!(
        page(port, &selected, None, 1, MAX_OPERATION_A2A_BINDING_BYTES)
            .unwrap_err()
            .kind(),
        ControlStoreErrorKind::BoundExceeded
    );
    port.connection
        .execute(
            "UPDATE pale_deployment_control_record SET canonical_record_jcs=?1 WHERE record_ref=?2",
            rusqlite::params![
                returned.canonical_record_jcs().as_slice(),
                returned.coordinate().record_ref().as_str()
            ],
        )
        .unwrap();
    let all = page(port, &selected, None, 128, 1 << 20).unwrap();
    assert_eq!(all.bindings.len(), 3);
    let attempts: Vec<_> = all
        .bindings
        .iter()
        .map(|source| {
            let wire: serde_json::Value =
                serde_json::from_slice(source.binding().canonical_record_jcs().as_slice()).unwrap();
            wire["attempt_id"].clone()
        })
        .collect();
    assert!(
        attempts.iter().all(|attempt| attempt == &attempts[0]),
        "distinct bindings preserve the same actual local attempt"
    );
    assert_eq!(&all.bindings[..2], &expected);
    assert_eq!(
        page(port, &unrelated, None, 128, 1 << 20).unwrap().bindings,
        other_page.bindings
    );
    let first_binding: serde_json::Value = serde_json::from_slice(
        initial.bindings[0]
            .binding()
            .canonical_record_jcs()
            .as_slice(),
    )
    .unwrap();
    let reused_binding_id = A2aTaskBindingIdV1::parse(
        first_binding["task_binding_id"]
            .as_str()
            .expect("stored task binding has its typed identity"),
    )
    .unwrap();
    let checkpoint = read_checkpoint(third.checkpoint().successor_record(), &fields);
    let writes = port.connection.total_changes();
    let transaction = port.prepare_transaction(&port.coordinate.clone()).unwrap();
    let collision = prepare(
        &context,
        4,
        Some(&reused_binding_id),
        (&third, &checkpoint),
        port,
        anchor,
    );
    assert!(
        matches!(
            collision,
            Err(A2aTaskCustodyError::Projection(
                OperationObservationCustodyProjectionError::AccumulatorPreparation(_)
            ))
        ),
        "accumulator preparation rejects unlike reuse of a task-binding identity"
    );
    assert_eq!(port.connection.total_changes(), writes);
    assert_eq!(
        port.prepare_transaction(&port.coordinate.clone()).unwrap(),
        transaction,
        "conflicting task identity cannot advance durable custody"
    );
    assert_eq!(
        page(port, &selected, None, 128, 1 << 20).unwrap().bindings,
        all.bindings,
        "conflicting JSON-RPC and remote Task identities cannot replace the retained binding"
    );
    for candidate in [first.0, &second] {
        let before = port.connection.total_changes();
        assert_eq!(
            port.compare_and_commit(candidate).unwrap_err().kind(),
            ControlStoreErrorKind::CandidateConflict
        );
        assert_eq!(port.connection.total_changes(), before);
    }
    assert_exact(port, &third);
    assert_eq!(
        page(port, &selected, None, 128, 1 << 20).unwrap().bindings,
        all.bindings
    );
    bounds(port, &selected, &unrelated, &cursor, &all.bindings);
    corruptions(port, first.0, &selected);
    (third, fields)
}

fn bounds(
    port: &mut SqliteOperationCustody<'_>,
    selected: &OperationA2aBindingScope,
    unrelated: &OperationA2aBindingScope,
    cursor: &OperationA2aBindingCursor,
    expected: &[OperationA2aBindingSource],
) {
    let bytes: usize = expected
        .iter()
        .map(|source| source.binding().canonical_record_jcs().as_slice().len())
        .sum();
    assert_eq!(
        page(port, selected, None, expected.len(), bytes)
            .unwrap()
            .bindings,
        expected
    );
    for (count, limit) in [
        (0, bytes),
        (129, bytes),
        (3, 0),
        (3, bytes - 1),
        (3, MAX_OPERATION_A2A_BINDING_BYTES + 1),
    ] {
        assert_eq!(
            page(port, selected, None, count, limit).unwrap_err().kind(),
            ControlStoreErrorKind::BoundExceeded
        );
    }
    assert!(
        page(port, unrelated, Some(cursor), 1, bytes).is_err(),
        "cursor cannot cross operation scope"
    );
    let future = OperationA2aBindingCursor::new(
        port.coordinate.clone(),
        selected.clone(),
        ControlTransactionSequenceV1::new(cursor.through().get() + 100).unwrap(),
        cursor.after_transaction(),
        cursor.after_reference().clone(),
    )
    .unwrap();
    assert!(page(port, selected, Some(&future), 1, bytes).is_err());
    let absent_position = OperationA2aBindingCursor::new(
        port.coordinate.clone(),
        selected.clone(),
        cursor.through(),
        cursor.after_transaction(),
        record_ref("missing-binding-position"),
    )
    .unwrap();
    assert!(page(port, selected, Some(&absent_position), 1, bytes).is_err());
    let wrong = AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("other-store").unwrap(),
        port.coordinate.incarnation_id().clone(),
    );
    assert!(port
        .read_a2a_task_bindings(&wrong, selected, None, 1, bytes)
        .is_err());
    let wrong_incarnation = AuthorityStoreCoordinate::new(
        port.coordinate.store_id().clone(),
        ControlIncarnationIdV1::parse("other-incarnation").unwrap(),
    );
    assert!(port
        .read_a2a_task_bindings(&wrong_incarnation, selected, None, 1, bytes)
        .is_err());
    let wrong_handle =
        OperationA2aBindingScope::new(selected.operation_key().clone(), unrelated.handle().clone());
    assert!(page(port, &wrong_handle, None, 1, bytes).is_err());
    let missing = OperationA2aBindingScope::new(
        selected.operation_key().clone(),
        ControlRecordCoordinate::new(
            record_ref("missing-operation-handle"),
            selected.handle().record_sha256().clone(),
        ),
    );
    assert!(page(port, &missing, None, 1, bytes).is_err());
}

fn corruptions(
    port: &mut SqliteOperationCustody<'_>,
    candidate: &OperationCustodyCandidate,
    selected: &OperationA2aBindingScope,
) {
    let binding = candidate
        .record(OperationCustodyRecordKind::A2aTaskBinding)
        .unwrap();
    let reference = binding.coordinate().record_ref().as_str();
    let (operation_key, binding_key, digest, stamp): (String, String, String, i64) = port.connection.query_row("SELECT operation_key,binding_key,record_sha256,inserted_transaction_sequence FROM pale_deployment_operation_a2a_binding WHERE record_ref=?1", [reference], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).unwrap();
    let transaction = port.prepare_transaction(candidate.store()).unwrap();
    port.connection
        .execute(
            "DELETE FROM pale_deployment_operation_a2a_binding WHERE record_ref=?1",
            [reference],
        )
        .unwrap();
    let before = port.connection.total_changes();
    assert_eq!(
        port.observe_exact(candidate).unwrap(),
        OperationCustodyReadBackOutcome::ConflictingOrPartial
    );
    assert_eq!(
        port.compare_and_commit(candidate).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired,
        "missing relation cannot be silently repaired as replay or treated as an unrelated winner"
    );
    assert_eq!(port.connection.total_changes(), before);
    port.connection.execute("INSERT INTO pale_deployment_operation_a2a_binding(operation_key,binding_key,record_ref,record_sha256,inserted_transaction_sequence) VALUES (?1,?2,?3,?4,?5)", rusqlite::params![operation_key,binding_key,reference,digest,stamp]).unwrap();
    port.connection.execute("UPDATE pale_deployment_operation_a2a_binding SET inserted_transaction_sequence=1 WHERE record_ref=?1", [reference]).unwrap();
    assert!(page(port, selected, None, 128, 1 << 20).is_err());
    let before = port.connection.total_changes();
    assert_eq!(
        port.observe_exact(candidate).unwrap(),
        OperationCustodyReadBackOutcome::ConflictingOrPartial
    );
    assert_eq!(
        port.compare_and_commit(candidate).unwrap_err().kind(),
        ControlStoreErrorKind::RecoveryRequired
    );
    assert_eq!(port.connection.total_changes(), before);
    port.connection.execute("UPDATE pale_deployment_operation_a2a_binding SET inserted_transaction_sequence=?1 WHERE record_ref=?2", rusqlite::params![stamp,reference]).unwrap();
    assert_eq!(
        port.prepare_transaction(candidate.store()).unwrap(),
        transaction
    );
    assert_eq!(
        port.compare_and_commit(candidate).unwrap_err().kind(),
        ControlStoreErrorKind::CandidateConflict,
        "restoring exact historical relation returns the ordinary stale-head classification"
    );
}
