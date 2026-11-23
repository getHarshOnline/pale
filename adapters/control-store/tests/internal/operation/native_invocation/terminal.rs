//! Commit terminal evidence and read it back through authenticated history.

use super::*;

pub(super) fn run(
    fixture: OutcomeFixture<'_>,
    outcome: UnstagedNativeOperationOutcome,
    mut sql: rusqlite::Connection,
    context_observation: Option<(&ResidentRecordRefV1, &OperationObservationV1)>,
    crash_selection: Option<ControlRecord>,
) {
    let OutcomeFixture {
        journey,
        generation,
        operation,
        request,
        parents,
        coordinate,
        location,
        file,
        first_record,
        first,
        prepared_done,
        preparation_history,
        dispatch_candidate,
        dispatch_checkpoint,
        dispatch_history_owned,
        observation,
        registry,
        contracts,
        clock,
        mut anchor,
    } = fixture;
    let resolve = |_: &OperationObservationV1| Ok(coordinate.clone());
    let dispatch_history = (&dispatch_history_owned.0, &dispatch_history_owned.1);
    let mut port = SqliteOperationCustody {
        connection: &mut sql,
        coordinate: coordinate.clone(),
    };
    let handle_ref = observation.handle_ref;
    let handle = observation.handle;
    let schema = observation.signed.expected_schema_sha256;
    let late = matches!(
        journey,
        Journey::SuccessAfterExpiry | Journey::FailureAfterUnknown
    );
    let stale_direct = late.then(|| {
        let terminal = history::prepare_terminal(
            outcome.outcome(),
            None,
            observation.signed,
            "stale-direct",
            None,
            true,
        )
        .unwrap();
        prepare_append(
            terminal.prepare_append(&resolve),
            operation,
            (
                dispatch_candidate
                    .checkpoint()
                    .successor()
                    .coordinate()
                    .record_ref(),
                &dispatch_checkpoint,
            ),
            &mut port,
            95,
        )
        .0
    });
    let intermediate = late.then(|| {
        history::publish_intermediate(
            operation,
            dispatch_history,
            observation.signed.signing_authority,
            matches!(journey, Journey::FailureAfterUnknown),
            (&dispatch_candidate, &dispatch_checkpoint),
            &resolve,
            &mut port,
            &mut anchor,
        )
    });
    if let Some(stale) = stale_direct {
        let before = port.connection.total_changes();
        let rejected = stale
            .commit(&mut port, &mut anchor)
            .expect_err("direct dispatch predecessor cannot overwrite the intermediate history");
        assert!(rejected.try_into_retryable().is_err());
        assert_eq!(port.connection.total_changes(), before);
    }
    let mut known = vec![(first_record.coordinate().record_ref(), first)];
    if let Some(binding) = context_observation {
        known.push(binding);
    }
    known.extend([
        (&preparation_history.0, &preparation_history.1),
        dispatch_history,
    ]);
    if let Some(intermediate) = &intermediate {
        known.push((&intermediate.reference, &intermediate.observation));
    }
    let recovered_history = intermediate.as_ref().map(|intermediate| {
        history::recover(
            history::Input {
                operation,
                coordinate: &coordinate,
                selected_member: dispatch_history,
                known: &known,
                checkpoint: &intermediate.fields,
                signing: parents.signing,
            },
            &mut port,
            &mut anchor,
        )
    });
    let competing = if let Some(proof) = &recovered_history {
        for fault in 0..3 {
            let rejected = history::prepare_terminal(
                outcome.outcome(),
                Some(proof),
                observation.signed,
                "invalid-history",
                Some(fault),
                false,
            );
            assert!(matches!(
                rejected,
                Err(OperationAttemptError::Association) | Err(OperationAttemptError::TimeOrder)
            ));
        }
        let intermediate = intermediate.as_ref().unwrap();
        let wrong_member = history::recover(
            history::Input {
                operation,
                coordinate: &coordinate,
                selected_member: (&preparation_history.0, &preparation_history.1),
                known: &known,
                checkpoint: &intermediate.fields,
                signing: parents.signing,
            },
            &mut port,
            &mut anchor,
        );
        assert!(matches!(
            history::prepare_terminal(
                outcome.outcome(),
                Some(&wrong_member),
                observation.signed,
                "wrong-selected-member",
                None,
                false
            ),
            Err(OperationAttemptError::Association)
        ));
        let terminal = history::prepare_terminal(
            outcome.outcome(),
            Some(proof),
            observation.signed,
            "competing-terminal",
            None,
            true,
        )
        .unwrap();
        Some(
            prepare_append(
                terminal.prepare_append(&resolve),
                operation,
                (
                    intermediate
                        .candidate
                        .checkpoint()
                        .successor()
                        .coordinate()
                        .record_ref(),
                    &intermediate.checkpoint,
                ),
                &mut port,
                95,
            )
            .0,
        )
    } else {
        None
    };
    let completion = {
        let signer = parents.signing.clone();
        let schema = observation.signed.expected_schema_sha256.clone();
        let terminal_signing = SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &schema,
            signing_authority: &signer,
        };
        match outcome.outcome() {
            NativeOperationOutcome::Succeeded(executed) => {
                assert!(matches!(
                    journey,
                    Journey::Success
                        | Journey::CrashRecovery
                        | Journey::SuccessAfterExpiry
                        | Journey::UnusedReadAuthority
                        | Journey::BoundedReadRefusal
                        | Journey::EvidenceOnly
                        | Journey::ReadAndEvidence
                ));
                let execution = executed.execution();
                if matches!(journey, Journey::CrashRecovery) {
                    // Recovery returned only after reproducing the private
                    // selection byte-for-byte. Its observable call must still
                    // carry the original body and every selected action identity.
                    let retained_selection: serde_json::Value = serde_json::from_slice(
                        crash_selection
                            .as_ref()
                            .unwrap()
                            .canonical_record_jcs()
                            .as_slice(),
                    )
                    .unwrap();
                    let selected = &retained_selection["implementation"];
                    assert_eq!(
                        serde_json::to_value(&execution.request.input).unwrap(),
                        request.body().payload
                    );
                    assert_eq!(
                        execution.request.action_id,
                        selected["action_id"].as_str().unwrap()
                    );
                    assert_eq!(
                        serde_json::to_value(execution.request.action_version).unwrap(),
                        selected["action_version"]
                    );
                    assert_eq!(
                        serde_json::to_value(&execution.request.contract_digest).unwrap(),
                        selected["contract_digest"]
                    );
                    assert_eq!(
                        serde_json::to_value(&execution.request.request_digest).unwrap(),
                        selected["request_digest"]
                    );
                    assert_eq!(
                        execution.result.request_digest,
                        execution.request.request_digest
                    );
                    assert_eq!(
                        execution.receipt.request_digest,
                        execution.request.request_digest
                    );
                }
                let ActionOutput::PathInventory(output) = &execution.result.output else {
                    panic!("actual selected handler")
                };
                assert_eq!((output.input_count, output.unique_count), (2, 1));
                assert_eq!(output.entries[0].normalized_path, "src/lib.rs");
                assert_eq!(output.entries[0].occurrences, 2);
                TerminalEvidence::Succeeded(Box::new(
                    executed
                        .prepare_completion(
                            NativeOperationCompletionInput {
                                recovered_history: recovered_history.as_ref(),
                                result_ref: record_ref("native/result"),
                                observation_ref: record_ref("native/succeeded"),
                                completed_at: UnixTimeV1::new(68).unwrap(),
                                observed_at: UnixTimeV1::new(95).unwrap(),
                                signing: terminal_signing,
                            },
                            |_, _, _| Ok(signature()),
                            |_, _, _, _| Ok(()),
                            |_, _, _| Ok(signature()),
                        )
                        .unwrap(),
                ))
            }
            NativeOperationOutcome::Failed(failed) => {
                assert!(matches!(
                    journey,
                    Journey::HandlerFailure | Journey::FailureAfterUnknown
                ));
                let Some(NativeActionInvocationError::Action(error)) = failed.error() else {
                    panic!("schema-valid input reaches the actual handler")
                };
                assert_eq!(error.code, ActionErrorCode::InvalidPath);
                TerminalEvidence::Failed(Box::new(
                    failed
                        .prepare_failure(
                            NativeOperationFailureInput {
                                recovered_history: recovered_history.as_ref(),
                                diagnostic_ref: record_ref("native/diagnostic"),
                                observation_ref: record_ref("native/failed"),
                                completed_at: UnixTimeV1::new(68).unwrap(),
                                observed_at: UnixTimeV1::new(95).unwrap(),
                                signing: terminal_signing,
                                maximum_diagnostic_bytes: 65_536,
                            },
                            |_, _, _, _| Ok(()),
                            |_, _, _| Ok(signature()),
                        )
                        .unwrap(),
                ))
            }
        }
    };
    // The prepared terminal owns its recovered predecessor; custody
    // does not borrow the authenticated recovery proof or its signer.
    drop(recovered_history);
    let append = completion.prepare_append(&resolve);
    let evidence_record = append.associated_record().unwrap().record().clone();
    let (physical, terminal_fields) = prepare_append(
        append,
        operation,
        intermediate.as_ref().map_or(
            (
                dispatch_candidate
                    .checkpoint()
                    .successor()
                    .coordinate()
                    .record_ref(),
                &dispatch_checkpoint,
            ),
            |value| {
                (
                    value
                        .candidate
                        .checkpoint()
                        .successor()
                        .coordinate()
                        .record_ref(),
                    &value.checkpoint,
                )
            },
        ),
        &mut port,
        95,
    );
    let terminal = physical
        .commit(&mut port, &mut anchor)
        .unwrap()
        .advance_anchor(&mut anchor)
        .unwrap();
    let terminal_candidate = terminal.candidate().clone();
    if let Some(competing) = competing {
        let before = port.connection.total_changes();
        let rejected = competing
            .commit(&mut port, &mut anchor)
            .expect_err("a later winner invalidates the recovered predecessor CAS");
        assert!(rejected.try_into_retryable().is_err());
        assert_eq!(port.connection.total_changes(), before);
        assert_eq!(
            port.observe_exact(&terminal_candidate).unwrap(),
            OperationCustodyReadBackOutcome::ExactSuccessor
        );
    }
    assert_eq!(terminal_candidate.records().len(), 2);
    drop(sql);
    let mut sql = connection(file);
    let mut port = SqliteOperationCustody {
        connection: &mut sql,
        coordinate: coordinate.clone(),
    };
    assert_eq!(
        port.observe_exact(&terminal_candidate).unwrap(),
        OperationCustodyReadBackOutcome::ExactSuccessor
    );
    let retained = port
        .read_operation_record(&coordinate, evidence_record.coordinate(), 65_536)
        .unwrap()
        .unwrap();
    assert_eq!(retained, evidence_record);
    match &completion {
        TerminalEvidence::Succeeded(completion) => {
            assert_eq!(
                retained.canonical_record_jcs().as_slice(),
                pale_core::canon::to_canonical_string(completion.result())
                    .unwrap()
                    .as_bytes()
            );
            let OperationObservationStateV1::Succeeded {
                result,
                effect_receipt,
            } = completion.observation().1.state()
            else {
                panic!("derived success evidence")
            };
            assert!(effect_receipt.is_none());
            assert_eq!(&result.record_ref, retained.coordinate().record_ref());
            assert_eq!(
                result.record_sha256.as_str(),
                retained.coordinate().record_sha256().as_str()
            );
        }
        TerminalEvidence::Failed(completion) => {
            let diagnostic = completion.diagnostic();
            assert_eq!(
                retained.canonical_record_jcs().as_slice(),
                diagnostic.canonical_bytes()
            );
            assert_eq!(diagnostic.wire().code, "invalid_path");
            assert!(
                !String::from_utf8_lossy(diagnostic.canonical_bytes()).contains("token=private")
            );
            let OperationObservationStateV1::Failed { result, diagnostic } =
                completion.observation().1.state()
            else {
                panic!("derived failure evidence")
            };
            assert!(result.is_none());
            assert_eq!(&diagnostic.record_ref, retained.coordinate().record_ref());
            assert_eq!(
                diagnostic.record_sha256.as_str(),
                retained.coordinate().record_sha256().as_str()
            );
            assert!(terminal_candidate.records().iter().all(|member| matches!(
                member,
                OperationCustodyRecord::OperationObservation(_)
                    | OperationCustodyRecord::Diagnostic(_)
            )));
            let has_result: bool = port.connection.query_row("SELECT EXISTS(SELECT 1 FROM pale_deployment_control_record WHERE schema_id=?1)", [CAPABILITY_RESULT_V1_SCHEMA], |row| row.get(0)).unwrap();
            assert!(
                !has_result,
                "a failed handler does not retain a successful result"
            );
        }
    }
    let replay_writes = port.connection.total_changes();
    assert_eq!(
        port.compare_and_commit(&terminal_candidate).unwrap(),
        OperationCustodyCommitOutcome::ExactReplay
    );
    assert_eq!(
        port.connection.total_changes(),
        replay_writes,
        "terminal replay performs no writes"
    );
    for member in terminal_candidate
        .records()
        .iter()
        .map(OperationCustodyRecord::record)
        .chain(std::iter::once(
            terminal_candidate.checkpoint().successor_record(),
        ))
    {
        let stamp: i64 = port.connection.query_row("SELECT inserted_transaction_sequence FROM pale_deployment_control_record WHERE record_ref=?1", [member.coordinate().record_ref().as_str()], |row| row.get(0)).unwrap();
        assert_eq!(
            u64::try_from(stamp).unwrap(),
            terminal_candidate
                .checkpoint()
                .successor()
                .committed_transaction_sequence()
                .get(),
            "terminal evidence, observation and checkpoint share one transaction"
        );
    }
    assert_eq!(clock.0.load(Ordering::SeqCst), 3);
    let writes = port.connection.total_changes();
    port.connection
        .pragma_update(None, "query_only", true)
        .unwrap();
    anchor.require_read_only();
    known.push(completion.observation());
    let sessions =
        DeploymentControlSqliteStore::open_unadmitted_candidate(location, coordinate.clone())
            .unwrap();
    let saved = history::recover_saved(
        &history::Input {
            operation,
            coordinate: &coordinate,
            selected_member: dispatch_history,
            known: &known,
            checkpoint: &terminal_fields,
            signing: parents.signing,
        },
        request,
        &parents,
        &sessions,
        &mut port,
        &mut anchor,
    );
    let recovered = history::recover(
        history::Input {
            operation: saved.operation(),
            coordinate: &coordinate,
            selected_member: dispatch_history,
            known: &known,
            checkpoint: &terminal_fields,
            signing: parents.signing,
        },
        &mut port,
        &mut anchor,
    );
    assert!(
        matches!(
            history::prepare_terminal(
                outcome.outcome(),
                Some(&recovered),
                observation.signed,
                "already-conclusive",
                None,
                false
            ),
            Err(OperationAttemptError::Association)
        ),
        "a conclusive recovered head cannot issue another terminal"
    );
    let (head_ref, head) = recovered.selected_head();
    assert_eq!((head_ref, head), completion.observation());
    let (dispatch_ref, dispatched) = recovered.selected();
    assert_eq!((dispatch_ref, dispatched), dispatch_history);
    assert_eq!(recovered.store(), &coordinate);
    let selected_record = prepared_done
        .candidate()
        .records()
        .iter()
        .find_map(|record| match record {
            OperationCustodyRecord::AttemptSelection(record) => Some(record),
            _ => None,
        })
        .unwrap();
    let selection = port
        .read_operation_record(&coordinate, selected_record.coordinate(), 65_536)
        .unwrap()
        .unwrap();
    assert_eq!(&selection, selected_record);
    let history = || OperationAttemptHistoryInput {
        operation: saved.operation(),
        request: saved.request(),
        session: saved.operation().historical_session_lease(),
        lease: saved.operation().historical_operation_lease(),
        generation,
        dispatched,
        selection: &selection,
        maximum_selection_bytes: 65_536,
    };
    assert_eq!(
        saved.operation().historical_operation_lease(),
        parents.lease
    );
    assert_eq!(
        saved.operation().historical_session_lease(),
        parents.session.lease()
    );
    match &completion {
        TerminalEvidence::Succeeded(completion) => {
            let OperationObservationStateV1::Succeeded {
                result: expected_result,
                ..
            } = head.state()
            else {
                panic!("authenticated successful head")
            };
            assert_eq!(
                &expected_result.record_ref,
                retained.coordinate().record_ref()
            );
            assert_eq!(
                expected_result.record_sha256.as_str(),
                retained.coordinate().record_sha256().as_str()
            );
            let admitted = admit_native_operation_result(
                NativeOperationResultInput {
                    history: history(),
                    registry,
                    contract_set: contracts,
                    terminal: NativeOperationTerminalEvidence::Recovered(&recovered),
                    result: &retained,
                    maximum_result_bytes: 65_536,
                },
                |domain, _, _, actual| {
                    assert_eq!(domain, CAPABILITY_RESULT_V1_SIGNATURE_DOMAIN);
                    assert_eq!(actual, &signature());
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(&admitted, completion.result());
        }
        TerminalEvidence::Failed(completion) => {
            let input = |maximum_diagnostic_bytes| NativeOperationFailureEvidenceInput {
                history: history(),
                registry,
                contract_set: contracts,
                terminal: NativeOperationTerminalEvidence::Recovered(&recovered),
                diagnostic: &retained,
                maximum_diagnostic_bytes,
            };
            let admitted = admit_native_operation_failure(input(
                retained.canonical_record_jcs().as_slice().len(),
            ))
            .unwrap();
            assert_eq!(&admitted, completion.diagnostic());

            for request_fault in 0..3 {
                let mut altered: serde_json::Value =
                    serde_json::from_slice(retained.canonical_record_jcs().as_slice()).unwrap();
                match request_fault {
                    0 => {
                        altered.as_object_mut().unwrap().remove("request");
                    }
                    1 => {
                        altered["request"]["kind"] =
                            serde_json::Value::String("provider_request".into());
                    }
                    2 => {
                        altered["request"]["id"] = serde_json::Value::String("0".repeat(64));
                    }
                    _ => unreachable!(),
                }
                let bytes = pale_core::canon::to_canonical_string(&altered)
                    .unwrap()
                    .into_bytes();
                let digest = ArtifactDigest::hash_bytes(&bytes);
                let reference =
                    record_ref(&format!("native/diagnostic-request-fault-{request_fault}"));
                let record = ControlRecord::new(
                    ControlRecordCoordinate::new(
                        reference.clone(),
                        ControlRecordDigest::parse(digest.as_str()).unwrap(),
                    ),
                    retained.schema_id().clone(),
                    retained.schema_sha256().clone(),
                    ControlRecordBytes::new(bytes).unwrap(),
                );
                let terminal = issue_observation(
                    parents.signing,
                    handle_ref,
                    handle,
                    schema.clone(),
                    OperationObservationRevisionV1::new(dispatched.revision().get() + 1).unwrap(),
                    Some((dispatch_ref, dispatched)),
                    OperationObservationStateV1::Failed {
                        result: None,
                        diagnostic: ImmutableRecordReferenceV1 {
                            record_ref: reference,
                            record_sha256: digest,
                        },
                    },
                    96,
                );
                assert!(matches!(
                    admit_native_operation_failure(NativeOperationFailureEvidenceInput {
                        history: history(),
                        registry,
                        contract_set: contracts,
                        terminal: NativeOperationTerminalEvidence::Direct {
                            dispatched_ref: dispatch_ref,
                            terminal: &terminal,
                        },
                        diagnostic: &record,
                        maximum_diagnostic_bytes: 65_536,
                    }),
                    Err(OperationAttemptError::Association)
                ));
            }

            for fault in 0..3 {
                let substituted = ControlRecord::new(
                    ControlRecordCoordinate::new(
                        if fault == 0 {
                            record_ref("native/diagnostic-alias")
                        } else {
                            retained.coordinate().record_ref().clone()
                        },
                        if fault == 1 {
                            ControlRecordDigest::parse("0".repeat(64)).unwrap()
                        } else {
                            retained.coordinate().record_sha256().clone()
                        },
                    ),
                    retained.schema_id().clone(),
                    if fault == 2 {
                        SchemaDigest::parse("0".repeat(64)).unwrap()
                    } else {
                        retained.schema_sha256().clone()
                    },
                    retained.canonical_record_jcs().clone(),
                );
                let mut altered = input(65_536);
                altered.diagnostic = &substituted;
                assert!(matches!(
                    admit_native_operation_failure(altered),
                    Err(OperationAttemptError::Association)
                ));
            }
            let wrong_dispatch_ref = record_ref("native/unrelated-dispatch");
            let mut altered = input(65_536);
            altered.terminal = NativeOperationTerminalEvidence::Direct {
                dispatched_ref: &wrong_dispatch_ref,
                terminal: head,
            };
            assert!(matches!(
                admit_native_operation_failure(altered),
                Err(OperationAttemptError::Association)
            ));
            let mut altered = input(65_536);
            altered.terminal = NativeOperationTerminalEvidence::Direct {
                dispatched_ref: dispatch_ref,
                terminal: &preparation_history.1,
            };
            assert!(matches!(
                admit_native_operation_failure(altered),
                Err(OperationAttemptError::Association)
            ));
            assert!(matches!(
                admit_native_operation_failure(input(
                    retained.canonical_record_jcs().as_slice().len() - 1
                )),
                Err(OperationAttemptError::BoundExceeded)
            ));
        }
    }
    assert_eq!(port.connection.total_changes(), writes);
    assert_eq!(
        clock.0.load(Ordering::SeqCst),
        3,
        "historical recovery does not sample the execution clock"
    );
}
