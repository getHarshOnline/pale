//! Remote history over the real operation SQLite/checkpoint/anchor journey.
//! The retained native dispatch supplies a storage fixture, NOT evidence that an
//! A2A send occurred. Signer, route, initial task and other evidence sources
//! remain explicit models; `GetTask` bytes pass the native protocol qualifier.

use super::*;
use std::cell::Cell;

#[path = "native_a2a_custody/cancellation.rs"]
mod cancellation_source;
#[path = "native_a2a_custody/discovery.rs"]
mod discovery;
#[path = "native_a2a_custody/fixture.rs"]
pub(super) mod fixture;
#[path = "native_a2a_custody/get_task.rs"]
mod get_task;
#[path = "native_a2a_custody/publication.rs"]
mod publication;
#[path = "native_a2a_custody/recovery.rs"]
mod recovery;
#[path = "native_a2a_custody/second_root.rs"]
mod second_root;
pub(super) use recovery::Restart;

pub(super) fn route(
    handle: &OperationHandleV1,
    authority: &SigningAuthorityBindingV1,
    provider: ProviderBindingRefV1,
    endpoint: &str,
    streaming: A2aStreamingAdmissionV1,
) -> A2aRouteAdmissionV1 {
    fixture::route(handle, authority, provider, endpoint, streaming)
}

#[derive(Clone, Copy)]
pub(super) enum Transport {
    Unary,
    Streaming,
}

fn stream_raw(reference: &str, bytes: &[u8], id: &str) -> OperationA2aEvidence {
    OperationA2aEvidence::new(
        fixture::evidence(reference, bytes),
        bytes.to_vec(),
        Some((id.into(), "message".into())),
        4096,
    )
    .unwrap()
}

fn raw(reference: &str, bytes: &[u8]) -> OperationA2aEvidence {
    OperationA2aEvidence::new(
        fixture::evidence(reference, bytes),
        bytes.to_vec(),
        None,
        4096,
    )
    .unwrap()
}

fn physical<'a>(
    append: PreparedA2aTaskAppend<'a>,
    operation: &IssuedLocalOperation,
    prior: (&'a ResidentRecordRefV1, &'a ControlStateCheckpointV1),
    port: &mut SqliteOperationCustody<'_>,
    time: u64,
    suffix: &str,
) -> Result<
    (
        PreparedOperationObservationCustody,
        ControlStateCheckpointPreimageV1,
    ),
    A2aTaskCustodyError,
> {
    let prepared = append.prepare_checkpoint(
        operation.checkpoint_selection(),
        prior.0,
        prior.1,
        AccumulatorPreparationLimits::new(16, 100_000, 100_000, 65_536, 1_000_000).unwrap(),
        AccumulatorMembershipLimits::new(16, 100_000, 1_000_000).unwrap(),
        port,
    )?;
    let mut fields = checkpoint_preimage(
        prior.1.domain_sequence().get() + 1,
        Some((
            prior.0.clone(),
            prior.1.canonical_control_checkpoint_sha256().clone(),
        )),
    );
    fields.checkpoint_domain_id = prior.1.checkpoint_domain_id().clone();
    fields.checkpoint_profile_ref = prior.1.checkpoint_profile_ref().clone();
    fields.checkpoint_profile_sha256 = prior.1.checkpoint_profile_sha256().clone();
    fields.committed_transaction_sequence = prepared.transaction_sequence();
    fields.state_root_sha256 = prepared.state_root().clone();
    fields.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
    fields.changed_leaf_keys = prepared.changed_keys().to_vec();
    fields.leaf_count = prepared.leaf_count();
    fields.issued_at = UnixTimeV1::new(time).unwrap();
    let selected = prepared
        .issue_checkpoint(
            &record_ref(&format!("a2a/checkpoint-{suffix}")),
            fields.clone(),
            &fields.schema_sha256,
            &checkpoint_authority(&fields),
            |_, _, _| Ok(signature()),
        )
        .unwrap();
    Ok((selected, fields))
}

fn current_local(
    port: &mut SqliteOperationCustody<'_>,
    operation: &IssuedLocalOperation,
) -> ControlRecord {
    let coordinate = port.coordinate.clone();
    read_operation_observation_for_handle(
        port,
        &coordinate,
        operation.handle().0,
        operation.handle().1,
        65_536,
    )
    .unwrap()
    .unwrap()
    .observation()
    .clone()
}

fn recover(
    input: &history::Input<'_>,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
) -> RecoveredOperationObservation {
    history::recover(
        history::Input {
            operation: input.operation,
            coordinate: input.coordinate,
            selected_member: input.selected_member,
            known: input.known,
            checkpoint: input.checkpoint,
            signing: input.signing,
        },
        port,
        anchor,
    )
}

fn assert_exact(port: &mut SqliteOperationCustody<'_>, candidate: &OperationCustodyCandidate) {
    let writes = port.connection.total_changes();
    assert_eq!(
        port.observe_exact(candidate).unwrap(),
        OperationCustodyReadBackOutcome::ExactSuccessor
    );
    assert_eq!(
        port.compare_and_commit(candidate).unwrap(),
        OperationCustodyCommitOutcome::ExactReplay
    );
    for record in candidate.records() {
        assert_eq!(
            port.read_operation_record(candidate.store(), record.record().coordinate(), 65_536)
                .unwrap()
                .as_ref(),
            Some(record.record())
        );
    }
    for raw in candidate
        .a2a_evidence()
        .iter()
        .chain(candidate.a2a_retained_evidence())
    {
        assert_eq!(
            port.read_a2a_evidence(candidate.store(), raw.coordinate(), raw.byte_len())
                .unwrap()
                .as_ref(),
            Some(raw)
        );
        assert!(port
            .read_a2a_evidence(candidate.store(), raw.coordinate(), raw.byte_len() - 1)
            .is_err());
        let (kind, stamp): (String, i64) = port.connection.query_row(
            "SELECT typeof(evidence_bytes), inserted_transaction_sequence FROM pale_deployment_operation_a2a_evidence WHERE record_ref=?1",
            [raw.coordinate().record_ref().as_str()], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        assert_eq!(kind, "blob");
        if candidate.a2a_evidence().contains(raw) {
            assert_eq!(
                u64::try_from(stamp).unwrap(),
                candidate
                    .accumulator()
                    .binding()
                    .transaction()
                    .successor()
                    .get()
            );
        }
    }
    assert_eq!(port.connection.total_changes(), writes);
    publication::assert_candidate(port, candidate);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    transport: Transport,
    generation: (&AdmittedCompiledGeneration, &EffectiveProjectProfileV1),
    history: history::Input<'_>,
    done: &CompletedOperationObservationCustody,
    signing: SignedResidentAdmissionContextV1<'_>,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
    restart: Restart<'_>,
) -> OperationCustodyCandidate {
    let mut second_root = second_root::start(generation.0, generation.1, port);
    let sources_before_remote = publication::all(port);
    assert!(sources_before_remote
        .iter()
        .all(|source| source.kind() == OperationPublicationSourceKind::OperationObservation));
    let operation = history.operation;
    discovery::assert_unidentified(port, operation);
    let dispatched = history.selected_member;
    let coordinate = port.coordinate.clone();
    let checkpoint = read_checkpoint(
        done.candidate().checkpoint().successor_record(),
        history.checkpoint,
    );
    let proof = recover(&history, port, anchor);
    let original_local = current_local(port, operation);
    let provider = ProviderBindingRefV1 {
        provider_binding_id: "fixture-a2a".into(),
        provider_binding_identity: ProviderBindingIdentityV1 {
            provider_binding_sha256: ArtifactDigest::hash_bytes(b"fixture provider owner"),
        },
    };
    let route = fixture::route(
        operation.handle().1,
        signing.signing_authority,
        provider,
        "https://agent.example.test/rpc",
        match transport {
            Transport::Unary => A2aStreamingAdmissionV1::NotAdvertised,
            Transport::Streaming => A2aStreamingAdmissionV1::Advertised {
                evidence: fixture::evidence(
                    "a2a/stream-advertisement",
                    b"fixture admitted streaming Agent Card",
                ),
            },
        },
    );
    let request = raw(
        "a2a/send-request",
        match transport {
            Transport::Unary => b"{ \"id\":\"rpc-1\", \"method\":\"SendMessage\" }\n",
            Transport::Streaming => b"{ \"id\":\"rpc-1\", \"method\":\"SendStreamingMessage\" }\n",
        },
    );
    let response = match transport {
        Transport::Unary => raw("a2a/send-response", b"{ \"task\": \"fixture-task-1\" }\n"),
        Transport::Streaming => stream_raw(
            "a2a/send-response",
            b"{ \"task\": \"fixture-task-1\" }\n",
            "event-1",
        ),
    };
    let mut inputs = fixture::Inputs::new(
        operation.handle(),
        dispatched,
        (&record_ref("a2a/route"), &route),
        fixture::evidence("a2a/send-request", request.bytes()),
        fixture::evidence("a2a/send-response", response.bytes()),
        100,
    );
    if matches!(transport, Transport::Streaming) {
        inputs.binding.request_mode = A2aTaskRequestModeV1::SendStreamingMessage;
        inputs.observation.task_binding_sha256 =
            A2aTaskBindingDigest::hash_canonical(&inputs.binding).unwrap();
        inputs.observation.source = A2aTaskObservationSourceV1::InitialStreamingEvent {
            event_id: Some(A2aSseEventIdV1::parse("event-1").unwrap()),
            evidence: inputs.binding.initial_task_evidence.clone(),
        };
    }
    let prepare = || {
        prepare_a2a_task_binding_custody(
            A2aTaskBindingCustodyInput {
                operation_custody: &proof,
                binding_ref: inputs.binding_ref.clone(),
                binding: inputs.binding.clone(),
                binding_context: inputs.binding_context(
                    operation.handle(),
                    dispatched,
                    &route,
                    signing.signing_authority,
                ),
                observation_ref: inputs.observation_ref.clone(),
                observation: inputs.observation.clone(),
                observation_signing: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &inputs.observation.schema_sha256,
                    signing_authority: signing.signing_authority,
                },
                route_predecessor: OperationCustodyIndexPredecessor::Absent,
                route_association: A2aRouteAssociation::Insert,
                new_evidence: vec![request.clone(), response.clone()],
                retained_evidence: vec![],
                maximum_evidence_bytes: 8192,
                maximum_record_bytes: 65_536,
            },
            |binding, observation, raw, retained| {
                // Model authorization of the exact source and Task; this is not a decoder or send receipt.
                assert_eq!(binding.request_evidence, inputs.binding.request_evidence);
                assert_eq!(observation.source, inputs.observation.source);
                assert_eq!(raw, &[request.clone(), response.clone()]);
                assert!(retained.is_empty());
                Ok(())
            },
            |_, _, _| Ok(signature()),
            |_, _, _| Ok(signature()),
            |_| Ok(coordinate.clone()),
        )
        .unwrap()
    };
    let append = prepare();
    let binding = append.binding().1.clone();
    let first = append.observation().1.clone();
    assert!(
        binding.bound_at() > route.expires_at(),
        "late exact identification does not renew route execution"
    );
    let prior = (
        done.candidate()
            .checkpoint()
            .successor()
            .coordinate()
            .record_ref(),
        &checkpoint,
    );
    let (prepared, fields) = physical(append, operation, prior, port, 100, "one").unwrap();
    let (replay, _) = physical(prepare(), operation, prior, port, 100, "one").unwrap();
    let (stale, _) = physical(prepare(), operation, prior, port, 100, "competing").unwrap();
    let (partial, _) = physical(prepare(), operation, prior, port, 100, "one").unwrap();
    let transaction = port.prepare_transaction(&coordinate).unwrap();
    port.connection.execute("INSERT INTO pale_deployment_operation_a2a_evidence (record_ref,record_sha256,evidence_bytes,sse_event_id,sse_event_type,inserted_transaction_sequence) VALUES (?1,?2,?3,NULL,NULL,?4)", rusqlite::params![response.coordinate().record_ref().as_str(), response.coordinate().record_sha256().as_str(), response.bytes(), i64::try_from(transaction.expected().get()).unwrap()]).unwrap();
    let writes = port.connection.total_changes();
    assert!(
        partial.commit(port, anchor).is_err(),
        "raw-only partial custody is not exact replay or fresh insertion"
    );
    assert_eq!(port.connection.total_changes(), writes);
    port.connection
        .execute(
            "DELETE FROM pale_deployment_operation_a2a_evidence WHERE record_ref=?1",
            [response.coordinate().record_ref().as_str()],
        )
        .unwrap();

    // Native deferred foreign-key commit failure: all atom writes execute,
    // SQLite rejects COMMIT, and the application must read back exact absence.
    port.connection.execute_batch("CREATE TEMP TABLE a2a_commit_parent (id INTEGER PRIMARY KEY); CREATE TEMP TABLE a2a_commit_child (parent_id INTEGER REFERENCES a2a_commit_parent(id) DEFERRABLE INITIALLY DEFERRED); CREATE TEMP TRIGGER a2a_reject_commit AFTER UPDATE ON pale_deployment_operation_checkpoint_head BEGIN INSERT INTO a2a_commit_child(parent_id) VALUES (1); END;").unwrap();
    let failure = prepared.commit(port, anchor).unwrap_err();
    let prepared = failure
        .try_into_retryable()
        .expect("retry only after exact predecessor read-back");
    assert_eq!(
        port.connection
            .query_row(
                "SELECT COUNT(*) FROM pale_deployment_operation_a2a_evidence",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    assert_eq!(current_local(port, operation), original_local);
    assert_eq!(
        publication::all(port),
        sources_before_remote,
        "uncertain commit resolved to absence cannot expose a remote source"
    );
    discovery::assert_no_relations(port);
    port.connection
        .execute_batch("DROP TRIGGER a2a_reject_commit")
        .unwrap();
    let first_done = prepared
        .commit(port, anchor)
        .unwrap()
        .advance_anchor(anchor)
        .unwrap();
    let first_candidate = first_done.candidate().clone();
    assert_exact(port, &first_candidate);
    assert_eq!(
        publication::all(port).len(),
        sources_before_remote.len() + 1
    );
    let publication_prefix = publication::Prefix::capture(port);
    assert_eq!(
        replay
            .commit(port, anchor)
            .unwrap()
            .advance_anchor(anchor)
            .unwrap()
            .database_evidence(),
        OperationCustodyDatabaseEvidence::ExactReplay
    );
    let writes = port.connection.total_changes();
    assert!(stale.commit(port, anchor).is_err());
    assert_eq!(port.connection.total_changes(), writes);
    assert_eq!(current_local(port, operation), original_local);
    assert_eq!(
        publication::all(port).len(),
        sources_before_remote.len() + 1,
        "replay and stale refusal do not allocate publication sources"
    );
    let other = second_root.associate(&route, &first_candidate, port);
    let (expanded, fields) = discovery::extend(
        &history,
        &route,
        (&first_candidate, &fields),
        &other,
        port,
        anchor,
    );
    let mut current_checkpoint = read_checkpoint(expanded.checkpoint().successor_record(), &fields);
    let mut current_ref = expanded
        .checkpoint()
        .successor()
        .coordinate()
        .record_ref()
        .clone();
    let mut previous_ref = inputs.observation_ref.clone();
    let mut previous = first;
    let get_request = get_task::request();
    let mut latest = expanded;
    for (revision, state) in [
        (2, A2aRemoteTaskStateV1::Working),
        (3, A2aRemoteTaskStateV1::Completed),
    ] {
        let proof = recover(&history, port, anchor);
        let response_name = format!("a2a/get-response-{revision}");
        let stream_successor = revision == 3 && matches!(transport, Transport::Streaming);
        let response = if stream_successor {
            let response_bytes = format!("{{ \"fixture-state\": \"{state:?}\" }}\n");
            stream_raw(&response_name, response_bytes.as_bytes(), "event-3")
        } else {
            get_task::response(&response_name, state)
        };
        let source = if stream_successor {
            A2aTaskObservationSourceV1::StreamingEvent {
                event_id: Some(A2aSseEventIdV1::parse("event-3").unwrap()),
                evidence: fixture::evidence(&response_name, response.bytes()),
            }
        } else {
            A2aTaskObservationSourceV1::GetTask {
                request: fixture::evidence("a2a/get-request", get_request.bytes()),
                response: fixture::evidence(&response_name, response.bytes()),
            }
        };
        let preimage = successor(
            &inputs,
            (&previous_ref, &previous),
            source,
            state,
            99 + revision,
        );
        let context = A2aTaskObservationAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &inputs.observation.schema_sha256,
                signing_authority: signing.signing_authority,
            },
            task_binding_ref: &inputs.binding_ref,
            task_binding: &binding,
            route_admission_ref: &inputs.route_ref,
            route_admission: &route,
            operation_handle_ref: operation.handle().0,
            operation_handle: operation.handle().1,
            operation_observation: dispatched.1,
            predecessor: Some((&previous_ref, &previous)),
        };
        let selected = |preimage, new_evidence, retained_evidence| A2aTaskObservationCustodyInput {
            operation_custody: &proof,
            operation_observation_ref: dispatched.0,
            observation_ref: record_ref(&format!("a2a/observation-{revision}")),
            observation: preimage,
            context,
            new_evidence,
            retained_evidence,
            maximum_evidence_bytes: 8192,
            maximum_record_bytes: 65_536,
        };
        if revision == 2 {
            let writes = port.connection.total_changes();
            let before = port.prepare_transaction(&coordinate).unwrap();
            get_task::boundaries(
                (&inputs.binding_ref, &binding),
                &preimage,
                &get_request,
                &response,
            );
            assert_eq!(port.connection.total_changes(), writes);
            assert_eq!(port.prepare_transaction(&coordinate).unwrap(), before);
            for rejected in get_task::rejected(&preimage, &get_request, &response) {
                let signs = Cell::new(0);
                let writes = port.connection.total_changes();
                let before = port.prepare_transaction(&coordinate).unwrap();
                assert!(
                    prepare_a2a_task_observation_custody(
                        selected(rejected.observation, rejected.evidence, vec![]),
                        |value, new, retained| get_task::admit_append(
                            (&inputs.binding_ref, &binding),
                            value,
                            new,
                            retained
                        ),
                        |_, _, _, _| Ok(()),
                        |_, _, _| {
                            signs.set(signs.get() + 1);
                            Ok(signature())
                        },
                        |_| Ok(coordinate.clone()),
                    )
                    .is_err(),
                    "{}",
                    rejected.label
                );
                assert_eq!(signs.get(), 0, "{}", rejected.label);
                assert_eq!(
                    port.connection.total_changes(),
                    writes,
                    "{}",
                    rejected.label
                );
                assert_eq!(port.prepare_transaction(&coordinate).unwrap(), before);
                assert_eq!(current_local(port, operation), original_local);
            }
        }
        if revision == 3 {
            let mut expired = preimage.clone();
            expired.observed_at = UnixTimeV1::new(121).unwrap();
            let signs = Cell::new(0);
            assert!(prepare_a2a_task_observation_custody(
                selected(
                    expired,
                    vec![response.clone()],
                    if stream_successor {
                        vec![]
                    } else {
                        vec![get_request.clone()]
                    }
                ),
                |value, new, retained| get_task::admit_append(
                    (&inputs.binding_ref, &binding),
                    value,
                    new,
                    retained
                ),
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
                "retention expiry fails at core admission before signing"
            );
        }
        let (new, retained) = if revision == 2 {
            (vec![get_request.clone(), response.clone()], vec![])
        } else if stream_successor {
            (vec![response.clone()], vec![])
        } else {
            (vec![response.clone()], vec![get_request.clone()])
        };
        if revision == 2 {
            // Exercise the opposite namespace order through the actual lease
            // writer, then freshly prepare so rejection is not a stale-T fence.
            let occupied =
                super::super::record("a2a/collision-response", 'e', OPERATION_LEASE_V1_SCHEMA);
            port.compare_and_retain_operation_lease(&coordinate, &occupied)
                .unwrap();
            let collision = get_task::response("a2a/collision-response", state);
            let mut changed = preimage.clone();
            changed.source = A2aTaskObservationSourceV1::GetTask {
                request: fixture::evidence("a2a/get-request", get_request.bytes()),
                response: fixture::evidence("a2a/collision-response", collision.bytes()),
            };
            let append = prepare_a2a_task_observation_custody(
                selected(changed, vec![get_request.clone(), collision], vec![]),
                |value, new, retained| {
                    get_task::admit_append((&inputs.binding_ref, &binding), value, new, retained)
                },
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(signature()),
                |_| Ok(coordinate.clone()),
            )
            .unwrap();
            let (candidate, _) = physical(
                append,
                operation,
                (&current_ref, &current_checkpoint),
                port,
                101,
                "collision",
            )
            .unwrap();
            let before = port.prepare_transaction(&coordinate).unwrap();
            let writes = port.connection.total_changes();
            assert!(candidate.commit(port, anchor).is_err());
            assert_eq!(port.prepare_transaction(&coordinate).unwrap(), before);
            assert_eq!(port.connection.total_changes(), writes);
            assert_eq!(
                port.read_operation_record(&coordinate, occupied.coordinate(), 65_536)
                    .unwrap()
                    .as_ref(),
                Some(&occupied)
            );
        }
        let append = prepare_a2a_task_observation_custody(
            selected(preimage, new, retained),
            |value, new, retained| {
                get_task::admit_append((&inputs.binding_ref, &binding), value, new, retained)
            },
            |_, _, _, _| Ok(()),
            |_, _, _| Ok(signature()),
            |_| Ok(coordinate.clone()),
        )
        .unwrap();
        let next_ref = append.observation().0.clone();
        let next = append.observation().1.clone();
        let (prepared, fields) = physical(
            append,
            operation,
            (&current_ref, &current_checkpoint),
            port,
            99 + revision,
            &revision.to_string(),
        )
        .unwrap();
        let done = prepared
            .commit(port, anchor)
            .unwrap()
            .advance_anchor(anchor)
            .unwrap();
        latest = done.candidate().clone();
        assert_exact(port, &latest);
        assert_eq!(
            current_local(port, operation),
            original_local,
            "remote terminal state never manufactures a local completion"
        );
        current_ref = latest
            .checkpoint()
            .successor()
            .coordinate()
            .record_ref()
            .clone();
        current_checkpoint = read_checkpoint(latest.checkpoint().successor_record(), &fields);
        previous_ref = next_ref;
        previous = next;
    }
    assert_eq!(previous.remote_state(), &A2aRemoteTaskStateV1::Completed);
    let writes = port.connection.total_changes();
    assert!(
        prepare()
            .prepare_checkpoint(
                operation.checkpoint_selection(),
                &current_ref,
                &current_checkpoint,
                AccumulatorPreparationLimits::new(16, 100_000, 100_000, 65_536, 1_000_000).unwrap(),
                AccumulatorMembershipLimits::new(16, 100_000, 1_000_000).unwrap(),
                port
            )
            .is_err(),
        "an old anchored recovery proof cannot be combined with a later checkpoint"
    );
    assert_eq!(port.connection.total_changes(), writes);
    let attempted_signed =
        super::super::record("a2a/get-response-3", 'e', OPERATION_LEASE_V1_SCHEMA);
    let before = port.prepare_transaction(&coordinate).unwrap();
    assert!(
        port.compare_and_retain_operation_lease(&coordinate, &attempted_signed)
            .is_err(),
        "ordinary signed retention cannot reuse a raw-evidence reference"
    );
    assert_eq!(port.prepare_transaction(&coordinate).unwrap(), before);
    assert!(port
        .read_operation_record(&coordinate, attempted_signed.coordinate(), 65_536)
        .unwrap()
        .is_none());
    // SQLite's total_changes includes rolled-back writes. Check durable state:
    // the transaction head and exact raw evidence must survive the rejection.
    assert_exact(port, &latest);
    latest = cancellation_source::run(
        &history,
        &inputs,
        &binding,
        &route,
        (&previous_ref, &previous),
        (&latest, &current_checkpoint),
        port,
        anchor,
        restart,
    );
    corruptions(port, &latest);
    publication_prefix.verify(port);
    publication::pages_and_bounds(port);
    publication::corrupt_checkpoint(port, &latest);
    latest
}

fn successor(
    inputs: &fixture::Inputs,
    previous: (&ResidentRecordRefV1, &A2aTaskObservationV1),
    source: A2aTaskObservationSourceV1,
    state: A2aRemoteTaskStateV1,
    time: u64,
) -> A2aTaskObservationPreimageV1 {
    let mut next = inputs.observation.clone();
    next.revision = previous.1.revision().checked_successor().unwrap();
    next.predecessor_ref = Some(previous.0.clone());
    next.predecessor_sha256 = Some(previous.1.canonical_a2a_task_observation_sha256().clone());
    next.source = source;
    next.remote_state = state;
    next.observed_at = UnixTimeV1::new(time).unwrap();
    next
}

fn corruptions(port: &mut SqliteOperationCustody<'_>, candidate: &OperationCustodyCandidate) {
    let raw = &candidate.a2a_evidence()[0];
    let writes = port.connection.total_changes();
    assert!(port.connection.execute("UPDATE pale_deployment_operation_a2a_evidence SET evidence_bytes=CAST(evidence_bytes AS TEXT) WHERE record_ref=?1", [raw.coordinate().record_ref().as_str()]).is_err(), "STRICT storage cannot silently coerce retained BLOB bytes to TEXT");
    assert_eq!(port.connection.total_changes(), writes);
    for update in [
        "evidence_bytes=zeroblob(length(evidence_bytes))",
        "sse_event_id='substituted',sse_event_type='message'",
        "inserted_transaction_sequence=1",
    ] {
        port.connection
            .execute(
                &format!(
                    "UPDATE pale_deployment_operation_a2a_evidence SET {update} WHERE record_ref=?1"
                ),
                [raw.coordinate().record_ref().as_str()],
            )
            .unwrap();
        assert_eq!(
            port.observe_exact(candidate).unwrap(),
            OperationCustodyReadBackOutcome::ConflictingOrPartial
        );
        assert!(port.compare_and_commit(candidate).is_err());
        // Public custody reads own their transaction. Commit test corruption
        // before calling them, then restore the exact fixture outside that read.
        let (id, kind) = raw
            .sse()
            .map_or((None, None), |(id, kind)| (Some(id), Some(kind)));
        port.connection.execute(
            "UPDATE pale_deployment_operation_a2a_evidence SET evidence_bytes=?1,sse_event_id=?2,sse_event_type=?3,inserted_transaction_sequence=?4 WHERE record_ref=?5",
            rusqlite::params![raw.bytes(), id, kind, i64::try_from(candidate.accumulator().binding().transaction().successor().get()).unwrap(), raw.coordinate().record_ref().as_str()],
        ).unwrap();
    }
    assert_exact(port, candidate);
}

pub(super) fn read_back(
    candidate: OperationCustodyCandidate,
    file: &std::path::Path,
    coordinate: &AuthorityStoreCoordinate,
) {
    let mut sql = connection(file);
    let mut port = SqliteOperationCustody {
        connection: &mut sql,
        coordinate: coordinate.clone(),
    };
    assert_exact(&mut port, &candidate);
    port.connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    publication::assert_candidate(&mut port, &candidate);
    publication::pages_and_bounds(&mut port);
}
