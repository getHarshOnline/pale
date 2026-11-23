//! Retained response phases cross real `SQLite` rollback, readback and anchor boundaries.
//! Signatures and the independent anchor remain fixtures; this is not a host trial.

use super::*;

#[path = "native_hook_response_expiry.rs"]
pub(in crate::operation::tests) mod expiry;
use pale_application::resident::hook_response_phase_test_support::{
    response_record, ResponsePhaseFixture, ResponsePhaseInputs,
};

#[derive(Clone, Copy, Debug)]
pub(in super::super) enum Case {
    Rollback,
    Anchor,
    UncertainDispatch,
    ColdPrepared,
    ColdExpiryRollback,
    ColdExpiryAnchor,
    ColdExpiryHost,
    ColdDispatched,
    ColdWrongRequest,
    ColdWrongHandle,
}

impl Case {
    pub(in super::super) const fn is_expiry(self) -> bool {
        matches!(
            self,
            Self::ColdExpiryRollback | Self::ColdExpiryAnchor | Self::ColdExpiryHost
        )
    }

    pub(in super::super) const fn is_cold(self) -> bool {
        matches!(
            self,
            Self::ColdPrepared
                | Self::ColdExpiryRollback
                | Self::ColdExpiryAnchor
                | Self::ColdExpiryHost
                | Self::ColdDispatched
                | Self::ColdWrongRequest
                | Self::ColdWrongHandle
        )
    }
}

pub(in super::super) fn run(
    case: Case,
    attempt: NativeOperationAttemptInput<'_, '_>,
    checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
) -> Option<ColdSnapshot> {
    let operation = attempt.context.operation;
    let predecessor = attempt.context.observation.predecessor.unwrap();
    let coordinate = port.coordinate.clone();
    let resolve = |_: &OperationObservationV1| Ok(coordinate.clone());
    let evidence = |name: &str| ImmutableRecordReferenceV1 {
        record_ref: record_ref(name),
        record_sha256: ArtifactDigest::hash_bytes(name.as_bytes()),
    };
    let response = response_record(
        record_ref("native/hook-response"),
        &ResidentHookResponseRecord {
            schema_id: SchemaId::parse(PRIVATE_OPERATION_HOOK_RESPONSE_V1_SCHEMA).unwrap(),
            handle_sha256: if matches!(case, Case::ColdWrongHandle) {
                OperationHandleDigest::parse("a".repeat(64)).unwrap()
            } else {
                operation
                    .handle()
                    .1
                    .canonical_operation_handle_sha256()
                    .clone()
            },
            native_request_sha256: if matches!(case, Case::ColdWrongRequest) {
                ArtifactDigest::hash_bytes(b"different native request")
            } else {
                attempt.context.request.body().payload_sha256.clone()
            },
            permission: evidence("native/hook-permission"),
            disclosure: evidence("native/hook-disclosure"),
            stdout: "fixture approved output".to_owned(),
            stderr: String::new(),
            exit_code: 0,
        },
    );
    let response_evidence = ImmutableRecordReferenceV1 {
        record_ref: response.coordinate().record_ref().clone(),
        record_sha256: ArtifactDigest::hash_bytes(response.canonical_record_jcs().as_slice()),
    };
    let preparation = issue_observation(
        attempt.context.observation.signed.signing_authority,
        operation.handle().0,
        operation.handle().1,
        attempt
            .context
            .observation
            .signed
            .expected_schema_sha256
            .clone(),
        OperationObservationRevisionV1::new(predecessor.1.revision().get() + 1).unwrap(),
        Some(predecessor),
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::DispatchPrepared {
                attempt_id: attempt.attempt_id.clone(),
                evidence: response_evidence.clone(),
            },
        },
        checkpoint.1.issued_at().get() + 1,
    );
    let preparation_ref = record_ref("native/hook-prepared");
    let preparation_checkpoint_ref = record_ref("native/hook-checkpoint-prepared");
    let mut phase = ResponsePhaseFixture::new(
        predecessor,
        (preparation_ref.clone(), preparation.clone()),
        checkpoint,
        OperationCheckpointInput {
            reference: preparation_checkpoint_ref.clone(),
            id: ControlCheckpointIdV1::parse("hook-checkpoint-prepared").unwrap(),
        },
    );
    let mut fields = checkpoint_preimage(2, None);
    fields.checkpoint_domain_id = checkpoint.1.checkpoint_domain_id().clone();
    fields.checkpoint_profile_ref = checkpoint.1.checkpoint_profile_ref().clone();
    fields.checkpoint_profile_sha256 = checkpoint.1.checkpoint_profile_sha256().clone();
    let authority = checkpoint_authority(&fields);
    let signatures = Cell::new(0);
    let mut sign =
        |_: &[u8], _: &ControlStateCheckpointPreimageV1, _: &ControlStateCheckpointDigest| {
            signatures.set(signatures.get() + 1);
            Ok(signature())
        };
    let mut inputs = ResponsePhaseInputs {
        checkpoint_schema: &fields.schema_sha256,
        checkpoint_authority: &authority,
        resolve_store: &resolve,
        accumulator_limits: AccumulatorPreparationLimits::new(1, 10_000, 10_000, 4096, 100_000)
            .unwrap(),
        sign_checkpoint: &mut sign,
    };
    let before = retained_custody_shape(port.connection);
    match case {
        Case::Rollback => {
            port.connection.execute_batch("CREATE TEMP TRIGGER reject_hook_preparation BEFORE INSERT ON pale_deployment_control_record WHEN NEW.record_ref = 'native/hook-prepared' BEGIN SELECT RAISE(ABORT, 'fixture hook storage unavailable'); END;").unwrap();
            assert!(!phase.try_complete(operation, Some(&response), &mut inputs, port, anchor));
            assert_eq!(retained_custody_shape(port.connection), before);
            port.connection
                .execute_batch("DROP TRIGGER reject_hook_preparation")
                .unwrap();
        }
        Case::Anchor => {
            let mut failing_anchor = ports::Anchor {
                inner: anchor,
                case: super::Case::PreparationAnchorFailure,
                advances: 0,
            };
            assert!(!phase.try_complete(
                operation,
                Some(&response),
                &mut inputs,
                port,
                &mut failing_anchor
            ));
            assert_ne!(retained_custody_shape(port.connection), before);
        }
        Case::UncertainDispatch
        | Case::ColdPrepared
        | Case::ColdExpiryRollback
        | Case::ColdExpiryAnchor
        | Case::ColdExpiryHost
        | Case::ColdDispatched
        | Case::ColdWrongRequest
        | Case::ColdWrongHandle => {}
    }
    let issued = phase.checkpoint().cloned();
    assert!(phase.try_complete(operation, Some(&response), &mut inputs, port, anchor));
    assert_eq!(signatures.get(), 1);
    if let Some(issued) = issued {
        assert_eq!(phase.checkpoint(), Some(&issued));
    }
    let candidate = phase.completed().unwrap().candidate();
    assert_eq!(
        candidate.record(OperationCustodyRecordKind::HookResponse),
        Some(&response)
    );
    assert_eq!(
        port.observe_exact(candidate).unwrap(),
        OperationCustodyReadBackOutcome::ExactSuccessor
    );
    let after = retained_custody_shape(port.connection);
    assert!(phase.try_complete(operation, Some(&response), &mut inputs, port, anchor));
    assert_eq!(signatures.get(), 1);
    assert_eq!(retained_custody_shape(port.connection), after);
    let mut known = vec![
        (predecessor.0.clone(), predecessor.1.clone()),
        (preparation_ref.clone(), preparation.clone()),
    ];
    if matches!(
        case,
        Case::ColdPrepared
            | Case::ColdWrongRequest
            | Case::ColdWrongHandle
            | Case::ColdExpiryRollback
            | Case::ColdExpiryAnchor
            | Case::ColdExpiryHost
    ) {
        return Some(ColdSnapshot {
            response,
            known,
            admissible: !matches!(case, Case::ColdWrongRequest | Case::ColdWrongHandle),
            expiry: Some(case).filter(|case| {
                matches!(
                    case,
                    Case::ColdExpiryRollback | Case::ColdExpiryAnchor | Case::ColdExpiryHost
                )
            }),
        });
    }
    if !matches!(case, Case::UncertainDispatch | Case::ColdDispatched) {
        return None;
    }

    let dispatch = issue_observation(
        attempt.context.observation.signed.signing_authority,
        operation.handle().0,
        operation.handle().1,
        attempt
            .context
            .observation
            .signed
            .expected_schema_sha256
            .clone(),
        OperationObservationRevisionV1::new(preparation.revision().get() + 1).unwrap(),
        Some((&preparation_ref, &preparation)),
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Dispatched {
                attempt_id: attempt.attempt_id.clone(),
                evidence: response_evidence,
            },
        },
        preparation.observed_at().get() + 1,
    );
    known.push((record_ref("native/hook-dispatched"), dispatch.clone()));
    let mut dispatch = ResponsePhaseFixture::new(
        (&preparation_ref, &preparation),
        (record_ref("native/hook-dispatched"), dispatch),
        (&preparation_checkpoint_ref, phase.checkpoint().unwrap()),
        OperationCheckpointInput {
            reference: record_ref("native/hook-checkpoint-dispatched"),
            id: ControlCheckpointIdV1::parse("hook-checkpoint-dispatched").unwrap(),
        },
    );
    if matches!(case, Case::ColdDispatched) {
        assert!(dispatch.try_complete(operation, None, &mut inputs, port, anchor));
        assert_eq!(signatures.get(), 2);
        return Some(ColdSnapshot {
            response,
            known,
            admissible: true,
            expiry: None,
        });
    }
    {
        let mut uncertain = ports::Custody {
            inner: port,
            case: super::Case::DispatchUnknownReadBack,
            commits: 1,
            last: None,
        };
        assert!(!dispatch.try_complete(operation, None, &mut inputs, &mut uncertain, anchor));
        assert_eq!(uncertain.commits, 2);
    }
    let issued = dispatch.checkpoint().unwrap().clone();
    let after_unknown_commit = retained_custody_shape(port.connection);
    assert!(dispatch.try_complete(operation, None, &mut inputs, port, anchor));
    assert_eq!(dispatch.checkpoint(), Some(&issued));
    assert_eq!(signatures.get(), 2);
    assert_eq!(
        retained_custody_shape(port.connection),
        after_unknown_commit
    );
    assert!(matches!(
        dispatch.verify_fresh_dispatch(operation, &resolve, port, anchor),
        Err(OperationDispatchCustodyError::NotNewCommit)
    ));
    None
}

pub(in super::super) struct ColdSnapshot {
    admissible: bool,
    expiry: Option<Case>,
    response: ControlRecord,
    known: Vec<(ResidentRecordRefV1, OperationObservationV1)>,
}

pub(in super::super) fn hook_request_body(
    generation: &AdmittedCompiledGeneration,
    profile: &EffectiveProjectProfileV1,
    request: &mut CommonRequestEnvelopePreimageV1,
) {
    let repository = &generation.record().repository_source_frontiers()[0].repository_id;
    let provider = generation
        .repository(repository)
        .unwrap()
        .render()
        .support
        .iter()
        .find(|row| row.execution_path_witness.selection == profile.execution_paths[0])
        .unwrap()
        .execution_path_witness
        .terminal_provider_binding
        .clone()
        .unwrap();
    let route = pale_application::hooks::RepositoryHookRoutingV1::new(
        LifecycleEvent::PreToolUse,
        profile.workspace_id.clone(),
        repository.clone(),
        provider,
    )
    .unwrap();
    let event = serde_json::json!({"hook_event_name":"PreToolUse","session_id":"native-session","tool_use_id":"native-hook-call","tool_name":"Read","tool_input":{"file_path":"source.rs"}});
    request.body = ResidentNativeToolStart::prepare(route, event.to_string().as_bytes())
        .unwrap()
        .body()
        .clone();
}

#[allow(clippy::too_many_arguments)]
pub(in super::super) fn recover(
    saved: ColdSnapshot,
    restart_generations: Option<[super::super::super::local_recovery::GenerationInputs; 2]>,
    file: &std::path::Path,
    location: &crate::location::ControlStoreLocation,
    coordinate: &AuthorityStoreCoordinate,
    operation: &IssuedLocalOperation,
    request: &CommonRequestEnvelopeV1,
    parents: &crate::resident_fixture::local_operation_start_fixture::RecoveryParents<'_>,
    fields: &ControlStateCheckpointPreimageV1,
    anchor: &mut IndependentAnchor,
) {
    use pale_application::resident::hook_response_phase_test_support::recover_response;
    let current = parents.current;
    assert!(operation
        .revalidate_session(current, &mut ProvingAuthority)
        .is_ok());
    let expired = AmbientSessionAdmissionContext {
        now: UnixTimeV1::new(parents.session.lease().expires_at().get() + 1).unwrap(),
        subject: current.subject,
        grant: current.grant,
        current_principal_trust: current.current_principal_trust,
        capability_revocations: current.capability_revocations,
        deployment_binding_ref: current.deployment_binding_ref,
        deployment_binding: current.deployment_binding,
        deployment_revocations: current.deployment_revocations,
        runtime_authority: current.runtime_authority,
        session_revocations: current.session_revocations,
        route_use: current.route_use,
        generation: SessionAdmissionLeaseContextV1 {
            active_head: current.generation.active_head,
            compiled_generation: current.generation.compiled_generation,
            effective_profile: current.generation.effective_profile,
            activation_fence: current.generation.activation_fence,
            signed: current.generation.signed,
        },
    };
    assert!(operation
        .revalidate_session(&expired, &mut ProvingAuthority)
        .is_err());
    let sessions =
        DeploymentControlSqliteStore::open_unadmitted_candidate(location, coordinate.clone())
            .unwrap();
    let mut sql = connection(file);
    sql.pragma_update(None, "query_only", true).unwrap();
    let mut port = SqliteOperationCustody {
        connection: &mut sql,
        coordinate: coordinate.clone(),
    };
    let known = saved
        .known
        .iter()
        .map(|(reference, record)| (reference, record))
        .collect::<Vec<_>>();
    let input = || history::Input {
        operation,
        coordinate,
        selected_member: *known.last().unwrap(),
        known: &known,
        checkpoint: fields,
        signing: parents.signing,
    };
    if let Some(case) = saved.expiry {
        expiry::run(
            case,
            &saved,
            restart_generations.unwrap(),
            file,
            &sessions,
            coordinate,
            request,
            parents,
            fields,
            &input(),
            &mut port,
            anchor,
        );
        return;
    }
    let recovered_operation =
        history::recover_saved(&input(), request, parents, &sessions, &mut port, anchor);
    let recovered_history = history::recover(input(), &mut port, anchor);
    let advances = anchor.advance_count();
    let writes = port.connection.total_changes();
    let recovered = recover_response(
        &recovered_operation,
        &recovered_history,
        &mut port,
        anchor,
        65_536,
    );
    if !saved.admissible {
        assert!(matches!(recovered, Err(OperationAttemptError::Association)));
        assert_eq!(port.connection.total_changes(), writes);
        assert_eq!(anchor.advance_count(), advances);
        return;
    }
    let exact = recovered.unwrap();
    assert_eq!(exact, saved.response);
    assert_eq!(port.connection.total_changes(), writes);
    assert_eq!(anchor.advance_count(), advances);
    assert!(recover_response(
        &recovered_operation,
        &recovered_history,
        &mut port,
        anchor,
        1
    )
    .is_err());

    port.connection
        .pragma_update(None, "query_only", false)
        .unwrap();
    let mut altered =
        pale_core::canon::parse_strict(saved.response.canonical_record_jcs().as_slice()).unwrap();
    altered["stdout"] = "substituted output".into();
    port.connection
        .execute(
            "UPDATE pale_deployment_control_record SET canonical_record_jcs=?1 WHERE record_ref=?2",
            rusqlite::params![
                pale_core::canon::to_canonical_string(&altered)
                    .unwrap()
                    .as_bytes(),
                saved.response.coordinate().record_ref().as_str()
            ],
        )
        .unwrap();
    assert!(recover_response(
        &recovered_operation,
        &recovered_history,
        &mut port,
        anchor,
        65_536
    )
    .is_err());
    port.connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    port.connection
        .execute(
            "DELETE FROM pale_deployment_control_record WHERE record_ref=?1",
            [saved.response.coordinate().record_ref().as_str()],
        )
        .unwrap();
    assert!(recover_response(
        &recovered_operation,
        &recovered_history,
        &mut port,
        anchor,
        65_536
    )
    .is_err());
    assert_eq!(anchor.advance_count(), advances);
}
