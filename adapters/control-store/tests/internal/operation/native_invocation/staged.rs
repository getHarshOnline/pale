//! Recover staged outcomes and reject substituted or incomplete evidence.

use super::*;

pub(super) fn run(
    fixture: OutcomeFixture<'_>,
    outcome: UnstagedNativeOperationOutcome,
    mut sql: rusqlite::Connection,
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

    let original_execution = match outcome.outcome() {
        NativeOperationOutcome::Succeeded(executed) => Some(executed.execution().clone()),
        NativeOperationOutcome::Failed(_) => None,
    };
    let stage = outcome
        .prepare_stage(
            NativeOperationOutcomeStageInput {
                outcome_ref: record_ref("native/staged-outcome"),
                observation_ref: record_ref("native/staged-observation"),
                completed_at: UnixTimeV1::new(68).unwrap(),
                observed_at: UnixTimeV1::new(68).unwrap(),
                signing: observation.signed,
                maximum_outcome_bytes: 65_536,
                maximum_diagnostic_bytes: 65_536,
            },
            |_, _, _, _| Ok(()),
            |_, _, _| Ok(signature()),
        )
        .unwrap();
    let stage_observation = (stage.observation().0.clone(), stage.observation().1.clone());
    let (physical, stage_fields) = prepare_append(
        stage.prepare_append(&resolve).unwrap(),
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
        68,
    );
    let completed = physical
        .commit(&mut port, &mut anchor)
        .unwrap()
        .advance_anchor(&mut anchor)
        .unwrap();
    let stage_candidate = completed.candidate().clone();
    let _staged = stage.finish(&completed).unwrap();
    assert!(stage_candidate
        .record(OperationCustodyRecordKind::NativeOutcome)
        .is_some());
    let replay_writes = port.connection.total_changes();
    assert_eq!(
        port.compare_and_commit(&stage_candidate).unwrap(),
        OperationCustodyCommitOutcome::ExactReplay
    );
    assert_eq!(port.connection.total_changes(), replay_writes);
    let selection = prepared_done
        .candidate()
        .record(OperationCustodyRecordKind::AttemptSelection)
        .unwrap()
        .clone();
    let cancellation = matches!(journey, Journey::StagedRecoveryAfterCancellation).then(|| {
        let cancellation_sessions =
            DeploymentControlSqliteStore::open_unadmitted_candidate(location, coordinate.clone())
                .unwrap();
        control_cancellation::publish_after_staged_outcome(
            operation,
            request,
            &parents,
            &stage_fields,
            &cancellation_sessions,
            observation.signed,
            &mut port,
            &mut anchor,
        )
    });
    drop(port);
    drop(sql);
    sql = connection(file);
    port = SqliteOperationCustody {
        connection: &mut sql,
        coordinate: coordinate.clone(),
    };
    let mut known = vec![
        (first_record.coordinate().record_ref(), first),
        (&preparation_history.0, &preparation_history.1),
        dispatch_history,
        (&stage_observation.0, &stage_observation.1),
    ];
    if let Some(cancellation) = &cancellation {
        known.push((&cancellation.reference, &cancellation.observation));
    }
    let selected_member = (&stage_observation.0, &stage_observation.1);
    let recovery_checkpoint = cancellation
        .as_ref()
        .map_or(&stage_fields, |cancellation| &cancellation.fields);
    let recovered = history::recover(
        history::Input {
            operation,
            coordinate: &coordinate,
            selected_member,
            known: &known,
            checkpoint: recovery_checkpoint,
            signing: parents.signing,
        },
        &mut port,
        &mut anchor,
    );
    if let Some(cancellation) = &cancellation {
        let skipped_stage = history::recover(
            history::Input {
                operation,
                coordinate: &coordinate,
                selected_member: dispatch_history,
                known: &known,
                checkpoint: &cancellation.fields,
                signing: parents.signing,
            },
            &mut port,
            &mut anchor,
        );
        let skipped = recover_native_operation_outcome(
            NativeOperationOutcomeRecoveryInput {
                history: OperationAttemptHistoryInput {
                    operation,
                    request,
                    session: operation.historical_session_lease(),
                    lease: operation.historical_operation_lease(),
                    generation,
                    dispatched: dispatch_history.1,
                    selection: &selection,
                    maximum_selection_bytes: 65_536,
                },
                maximum_outcome_bytes: 65_536,
            },
            &skipped_stage,
            registry,
            contracts,
            &mut port,
        );
        assert!(matches!(skipped, Err(OperationAttemptError::Association)));

        let substituted_dispatch = recover_native_operation_outcome(
            NativeOperationOutcomeRecoveryInput {
                history: OperationAttemptHistoryInput {
                    operation,
                    request,
                    session: operation.historical_session_lease(),
                    lease: operation.historical_operation_lease(),
                    generation,
                    dispatched: &stage_observation.1,
                    selection: &selection,
                    maximum_selection_bytes: 65_536,
                },
                maximum_outcome_bytes: 65_536,
            },
            &recovered,
            registry,
            contracts,
            &mut port,
        );
        assert!(matches!(
            substituted_dispatch,
            Err(OperationAttemptError::Association)
        ));

        let unrelated_request = recover_native_operation_outcome(
            NativeOperationOutcomeRecoveryInput {
                history: OperationAttemptHistoryInput {
                    operation,
                    request: &cancellation.request,
                    session: operation.historical_session_lease(),
                    lease: operation.historical_operation_lease(),
                    generation,
                    dispatched: dispatch_history.1,
                    selection: &selection,
                    maximum_selection_bytes: 65_536,
                },
                maximum_outcome_bytes: 65_536,
            },
            &recovered,
            registry,
            contracts,
            &mut port,
        );
        assert!(matches!(
            unrelated_request,
            Err(OperationAttemptError::Association)
        ));
    }
    if matches!(journey, Journey::StagedOutcomeSubstitution) {
        let stored = stage_candidate
            .record(OperationCustodyRecordKind::NativeOutcome)
            .unwrap();
        let mut substituted: serde_json::Value =
            serde_json::from_slice(stored.canonical_record_jcs().as_slice()).unwrap();
        substituted["dispatch_ref"] =
            serde_json::Value::String("native/substituted-dispatch".into());
        let bytes = pale_core::canon::to_canonical_string(&substituted)
            .unwrap()
            .into_bytes();
        let digest = pale_core::sha256_hex(&bytes);
        port.connection
            .execute(
                "INSERT INTO pale_deployment_control_record (record_ref, inserted_transaction_sequence, record_sha256, schema_id, schema_sha256, canonical_record_jcs) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    "native/substituted-outcome",
                    i64::try_from(
                        stage_candidate
                            .checkpoint()
                            .successor()
                            .committed_transaction_sequence()
                            .get(),
                    )
                    .unwrap(),
                    digest,
                    stored.schema_id().as_str(),
                    stored.schema_sha256().as_str(),
                    bytes,
                ],
            )
            .unwrap();
        port.connection
            .execute(
                "UPDATE pale_deployment_operation_index SET record_ref=?1, record_sha256=?2 WHERE index_role='native_outcome'",
                rusqlite::params!["native/substituted-outcome", digest],
            )
            .unwrap();
    }
    if matches!(
        journey,
        Journey::StagedFailureDiagnosticSubstitution
            | Journey::StagedFailureRequestSubstitution
            | Journey::StagedFailureRequestKindSubstitution
            | Journey::StagedFailureRequestMissing
    ) {
        let stored = stage_candidate
            .record(OperationCustodyRecordKind::NativeOutcome)
            .unwrap();
        let mut substituted: serde_json::Value =
            serde_json::from_slice(stored.canonical_record_jcs().as_slice()).unwrap();
        if matches!(journey, Journey::StagedFailureDiagnosticSubstitution) {
            substituted["outcome"]["diagnostic"]["subject"]["id"] =
                serde_json::Value::String("pale.compute.substituted".into());
        } else if matches!(journey, Journey::StagedFailureRequestSubstitution) {
            substituted["outcome"]["diagnostic"]["request"]["id"] =
                serde_json::Value::String("0".repeat(64));
        } else if matches!(journey, Journey::StagedFailureRequestKindSubstitution) {
            substituted["outcome"]["diagnostic"]["request"]["kind"] =
                serde_json::Value::String("provider_request".into());
        } else {
            substituted["outcome"]["diagnostic"]
                .as_object_mut()
                .unwrap()
                .remove("request");
        }
        let bytes = pale_core::canon::to_canonical_string(&substituted)
            .unwrap()
            .into_bytes();
        let digest = pale_core::sha256_hex(&bytes);
        port.connection
            .execute(
                "INSERT INTO pale_deployment_control_record (record_ref, inserted_transaction_sequence, record_sha256, schema_id, schema_sha256, canonical_record_jcs) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    "native/substituted-failure-outcome",
                    i64::try_from(
                        stage_candidate
                            .checkpoint()
                            .successor()
                            .committed_transaction_sequence()
                            .get(),
                    )
                    .unwrap(),
                    digest,
                    stored.schema_id().as_str(),
                    stored.schema_sha256().as_str(),
                    bytes,
                ],
            )
            .unwrap();
        port.connection
            .execute(
                "UPDATE pale_deployment_operation_index SET record_ref=?1, record_sha256=?2 WHERE index_role='native_outcome'",
                rusqlite::params!["native/substituted-failure-outcome", digest],
            )
            .unwrap();
    }
    if matches!(journey, Journey::StagedOutcomeMissingIndex) {
        port.connection
            .execute(
                "DELETE FROM pale_deployment_operation_index WHERE index_role='native_outcome'",
                [],
            )
            .unwrap();
    }
    if matches!(journey, Journey::StagedOutcomeMissingEvidence) {
        port.connection
            .execute_batch("PRAGMA foreign_keys=OFF")
            .unwrap();
        port.connection
            .execute(
                "UPDATE pale_deployment_operation_index SET record_ref=?1 WHERE index_role='native_outcome'",
                ["native/missing-outcome"],
            )
            .unwrap();
        port.connection
            .execute_batch("PRAGMA foreign_keys=ON")
            .unwrap();
    }
    if matches!(journey, Journey::StagedOutcomeMalformedEvidence) {
        let stored = stage_candidate
            .record(OperationCustodyRecordKind::NativeOutcome)
            .unwrap();
        let bytes = b"{".as_slice();
        let digest = pale_core::sha256_hex(bytes);
        port.connection
            .execute(
                "INSERT INTO pale_deployment_control_record (record_ref, inserted_transaction_sequence, record_sha256, schema_id, schema_sha256, canonical_record_jcs) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    "native/malformed-outcome",
                    i64::try_from(
                        stage_candidate
                            .checkpoint()
                            .successor()
                            .committed_transaction_sequence()
                            .get(),
                    )
                    .unwrap(),
                    digest,
                    stored.schema_id().as_str(),
                    stored.schema_sha256().as_str(),
                    bytes,
                ],
            )
            .unwrap();
        port.connection
            .execute(
                "UPDATE pale_deployment_operation_index SET record_ref=?1, record_sha256=?2 WHERE index_role='native_outcome'",
                rusqlite::params!["native/malformed-outcome", digest],
            )
            .unwrap();
    }
    if matches!(journey, Journey::StagedOutcomeWrongSchema) {
        let stored = stage_candidate
            .record(OperationCustodyRecordKind::NativeOutcome)
            .unwrap();
        port.connection
            .execute(
                "INSERT INTO pale_deployment_control_record (record_ref, inserted_transaction_sequence, record_sha256, schema_id, schema_sha256, canonical_record_jcs) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    "native/wrong-schema-outcome",
                    i64::try_from(
                        stage_candidate
                            .checkpoint()
                            .successor()
                            .committed_transaction_sequence()
                            .get(),
                    )
                    .unwrap(),
                    stored.coordinate().record_sha256().as_str(),
                    OPERATION_OBSERVATION_V1_SCHEMA,
                    stored.schema_sha256().as_str(),
                    stored.canonical_record_jcs().as_slice(),
                ],
            )
            .unwrap();
        port.connection
            .execute(
                "UPDATE pale_deployment_operation_index SET record_ref=?1 WHERE index_role='native_outcome'",
                ["native/wrong-schema-outcome"],
            )
            .unwrap();
    }
    let corrupted_custody = matches!(
        journey,
        Journey::StagedOutcomeMissingIndex
            | Journey::StagedOutcomeMissingEvidence
            | Journey::StagedOutcomeMalformedEvidence
            | Journey::StagedOutcomeWrongSchema
    )
    .then(|| {
        assert_eq!(
            port.connection
                .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
        let shape = retained_custody_shape(port.connection);
        assert_eq!((shape.results, shape.diagnostics), (0, 0));
        match journey {
            Journey::StagedOutcomeMissingIndex => {
                assert_eq!(
                    (shape.native_outcomes, shape.native_outcome_indexes),
                    (1, 0)
                );
            }
            Journey::StagedOutcomeMissingEvidence => {
                assert_eq!(
                    (shape.native_outcomes, shape.native_outcome_indexes),
                    (1, 1)
                );
            }
            Journey::StagedOutcomeMalformedEvidence => {
                assert_eq!(
                    (shape.native_outcomes, shape.native_outcome_indexes),
                    (2, 1)
                );
            }
            Journey::StagedOutcomeWrongSchema => {
                assert_eq!(
                    (shape.native_outcomes, shape.native_outcome_indexes),
                    (1, 1)
                );
            }
            _ => unreachable!(),
        }
        (port.connection.total_changes(), shape)
    });
    let rejected_evidence = matches!(
        journey,
        Journey::StagedOutcomeSubstitution
            | Journey::StagedFailureDiagnosticSubstitution
            | Journey::StagedFailureRequestSubstitution
            | Journey::StagedFailureRequestKindSubstitution
            | Journey::StagedFailureRequestMissing
    )
    .then(|| {
        (
            port.connection.total_changes(),
            retained_custody_shape(port.connection),
            anchor.advance_count(),
        )
    });
    let recovered_outcome = recover_native_operation_outcome(
        NativeOperationOutcomeRecoveryInput {
            history: OperationAttemptHistoryInput {
                operation,
                request,
                session: operation.historical_session_lease(),
                lease: operation.historical_operation_lease(),
                generation,
                dispatched: dispatch_history.1,
                selection: &selection,
                maximum_selection_bytes: 65_536,
            },
            maximum_outcome_bytes: 65_536,
        },
        &recovered,
        registry,
        contracts,
        &mut port,
    );
    if let Some((writes, shape)) = corrupted_custody {
        match journey {
            Journey::StagedOutcomeMissingIndex => {
                assert!(matches!(
                    recovered_outcome,
                    Err(OperationAttemptError::Association)
                ));
            }
            Journey::StagedOutcomeMissingEvidence => {
                assert!(matches!(
                    recovered_outcome,
                    Err(OperationAttemptError::Custody(_))
                ));
            }
            Journey::StagedOutcomeMalformedEvidence => {
                assert!(matches!(
                    recovered_outcome,
                    Err(OperationAttemptError::Encoding)
                ));
            }
            Journey::StagedOutcomeWrongSchema => {
                assert!(matches!(
                    recovered_outcome,
                    Err(OperationAttemptError::Custody(_))
                ));
            }
            _ => unreachable!(),
        }
        assert_eq!(
            port.connection.total_changes(),
            writes,
            "recovery cannot write after private evidence fails"
        );
        assert_eq!(
            retained_custody_shape(port.connection),
            shape,
            "recovery cannot fabricate terminal or unrelated custody"
        );
        assert_eq!(
            clock.0.load(Ordering::SeqCst),
            3,
            "recovery cannot resample or rerun the native handler"
        );
        return;
    }
    if matches!(
        journey,
        Journey::StagedOutcomeSubstitution
            | Journey::StagedFailureDiagnosticSubstitution
            | Journey::StagedFailureRequestSubstitution
            | Journey::StagedFailureRequestKindSubstitution
            | Journey::StagedFailureRequestMissing
    ) {
        assert!(matches!(
            recovered_outcome,
            Err(OperationAttemptError::Association)
        ));
        let (writes, shape, advances) = rejected_evidence.unwrap();
        assert_eq!(
            port.connection.total_changes(),
            writes,
            "rejected staged evidence cannot write"
        );
        assert_eq!(
            retained_custody_shape(port.connection),
            shape,
            "rejected staged evidence cannot promote a terminal outcome"
        );
        assert_eq!(
            anchor.advance_count(),
            advances,
            "rejected staged evidence cannot advance the independent anchor"
        );
        assert_eq!(
            clock.0.load(Ordering::SeqCst),
            3,
            "rejected staged evidence cannot rerun or resample the handler"
        );
        return;
    }
    let recovered_outcome = recovered_outcome
        .unwrap_or_else(|failure| panic!("{journey:?} staged recovery failed: {failure:?}"));
    if let Some(cancellation) = &cancellation {
        let NativeOperationOutcome::Succeeded(executed) = recovered_outcome.outcome() else {
            panic!("the cancellation recovery fixture stages success")
        };
        assert_eq!(executed.execution(), original_execution.as_ref().unwrap());
        let terminal = executed
            .prepare_completion(
                NativeOperationCompletionInput {
                    recovered_history: None,
                    result_ref: record_ref("native/staged-cancelled-result"),
                    observation_ref: record_ref("native/staged-cancelled-success"),
                    completed_at: UnixTimeV1::new(68).unwrap(),
                    observed_at: UnixTimeV1::new(71).unwrap(),
                    signing: observation.signed,
                },
                |_, _, _| Ok(signature()),
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(signature()),
            )
            .expect("cold recovery records without replaying the handler");
        assert_eq!(
            terminal.observation().1.predecessor_ref().as_ref(),
            Some(&cancellation.reference)
        );
        let terminal_observation = (
            terminal.observation().0.clone(),
            terminal.observation().1.clone(),
        );
        let (terminal_custody, terminal_fields) = prepare_append(
            terminal.prepare_append(&resolve).unwrap(),
            operation,
            (
                cancellation
                    .candidate
                    .checkpoint()
                    .successor()
                    .coordinate()
                    .record_ref(),
                &cancellation.checkpoint,
            ),
            &mut port,
            71,
        );
        terminal_custody
            .commit(&mut port, &mut anchor)
            .unwrap()
            .advance_anchor(&mut anchor)
            .unwrap();
        drop(port);
        drop(sql);
        sql = connection(file);
        port = SqliteOperationCustody {
            connection: &mut sql,
            coordinate: coordinate.clone(),
        };
        let terminal_known = [
            (first_record.coordinate().record_ref(), first),
            (&preparation_history.0, &preparation_history.1),
            dispatch_history,
            (&stage_observation.0, &stage_observation.1),
            (&cancellation.reference, &cancellation.observation),
            (&terminal_observation.0, &terminal_observation.1),
        ];
        let terminal_head = history::recover(
            history::Input {
                operation,
                coordinate: &coordinate,
                selected_member: (&stage_observation.0, &stage_observation.1),
                known: &terminal_known,
                checkpoint: &terminal_fields,
                signing: parents.signing,
            },
            &mut port,
            &mut anchor,
        );
        let rejected_terminal = recover_native_operation_outcome(
            NativeOperationOutcomeRecoveryInput {
                history: OperationAttemptHistoryInput {
                    operation,
                    request,
                    session: operation.historical_session_lease(),
                    lease: operation.historical_operation_lease(),
                    generation,
                    dispatched: dispatch_history.1,
                    selection: &selection,
                    maximum_selection_bytes: 65_536,
                },
                maximum_outcome_bytes: 65_536,
            },
            &terminal_head,
            registry,
            contracts,
            &mut port,
        );
        assert!(matches!(
            rejected_terminal,
            Err(OperationAttemptError::Association)
        ));
        assert_eq!(
            clock.0.load(Ordering::SeqCst),
            3,
            "cold outcome recovery cannot rerun or resample the handler"
        );
        return;
    }
    match recovered_outcome.outcome() {
        NativeOperationOutcome::Succeeded(executed) => {
            assert_eq!(executed.execution(), original_execution.as_ref().unwrap());
            let result = executed
                .issue_result(UnixTimeV1::new(69).unwrap(), |_, _, _| Ok(signature()))
                .unwrap();
            assert_eq!(
                serde_json::to_value(result).unwrap()["started_at"],
                serde_json::json!(67),
                "cold recovery preserves the original handler start time"
            );
        }
        NativeOperationOutcome::Failed(failed) => {
            assert!(failed.error().is_none());
            let terminal = failed
                .prepare_failure(
                    NativeOperationFailureInput {
                        recovered_history: None,
                        diagnostic_ref: record_ref("native/recovered-staged-diagnostic"),
                        observation_ref: record_ref("native/recovered-staged-failure"),
                        completed_at: UnixTimeV1::new(68).unwrap(),
                        observed_at: UnixTimeV1::new(69).unwrap(),
                        signing: observation.signed,
                        maximum_diagnostic_bytes: 65_536,
                    },
                    |_, _, _, _| Ok(()),
                    |_, _, _| Ok(signature()),
                )
                .unwrap();
            assert_eq!(terminal.diagnostic().wire().code, "invalid_path");
            assert!(
                !String::from_utf8_lossy(terminal.diagnostic().canonical_bytes())
                    .contains("token=private")
            );
        }
    }
    assert_eq!(clock.0.load(Ordering::SeqCst), 3);
}
