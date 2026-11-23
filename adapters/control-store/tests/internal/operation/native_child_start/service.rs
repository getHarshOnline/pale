//! An issued child Invoke consumes the real native service; cold evidence is
//! recovered separately and never promoted into its parent's outcome.
//! Signing and historical state admission remain explicit fixture models.

use super::*;

pub(super) fn run(
    fixture: &Fixture<'_>,
    child: &IssuedLocalOperation,
    request: &CommonRequestEnvelopeV1,
    start: &OperationCustodyCandidate,
    fields: &ControlStateCheckpointPreimageV1,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
) -> (CompletedNativeOperation, ControlStateCheckpointPreimageV1) {
    let checkpoint = read_checkpoint(start.checkpoint().successor_record(), fields);
    run_after_checkpoint(
        fixture,
        child,
        request,
        start,
        (
            start.checkpoint().successor().coordinate().record_ref(),
            &checkpoint,
        ),
        fields,
        port,
        anchor,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_after_checkpoint(
    fixture: &Fixture<'_>,
    child: &IssuedLocalOperation,
    request: &CommonRequestEnvelopeV1,
    start: &OperationCustodyCandidate,
    current_checkpoint: (&ResidentRecordRefV1, &ControlStateCheckpointV1),
    current_checkpoint_fields: &ControlStateCheckpointPreimageV1,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
) -> (CompletedNativeOperation, ControlStateCheckpointPreimageV1) {
    fixture::with_child_session(fixture.parents, |session, current| {
        assert_eq!(child.historical_session_lease(), session.lease());
        assert_ne!(child.historical_operation_lease(), fixture.parents.lease);
        let (handle_ref, handle) = child.handle();
        assert_eq!(handle.operation_class(), &OperationClassV1::Invoke);
        let schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let signed = SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &schema,
            signing_authority: fixture.parents.signing,
        };
        let first_record = start
            .record(OperationCustodyRecordKind::OperationObservation)
            .unwrap();
        let first = OperationObservationV1::admit_contextual_wire(
            serde_json::from_slice(first_record.canonical_record_jcs().as_slice()).unwrap(),
            &OperationObservationAdmissionContextV1 {
                signed,
                handle_ref,
                handle,
                predecessor: None,
            },
            |state| {
                assert_eq!(
                    state,
                    &OperationObservationStateV1::Active {
                        phase: OperationActivePhaseV1::Resolving,
                    }
                );
                Ok(())
            },
            |_, _, _, _| panic!("no child authority rotation"),
            |_, _, _, actual| {
                assert_eq!(actual, &signature());
                Ok(())
            },
        )
        .unwrap();
        let first_observation = OperationObservationAdmissionContextV1 {
            signed,
            handle_ref,
            handle,
            predecessor: Some((first_record.coordinate().record_ref(), &first)),
        };
        let coordinate = port.coordinate.clone();
        let resolve = move |_: &OperationObservationV1| Ok(coordinate.clone());
        let context_binding = (!child
            .historical_operation_lease()
            .evidence_obligations()
            .is_empty())
        .then(|| {
            let evidence = ImmutableRecordReferenceV1 {
                record_ref: record_ref("native/child-service-context"),
                record_sha256: ArtifactDigest::hash_bytes(b"admitted child context projection"),
            };
            let prepared = prepare_operation_context_binding(
                OperationContextBindingInput {
                    operation: child,
                    request,
                    session,
                    current,
                    lease: child.historical_operation_lease(),
                    evidence: evidence.clone(),
                    observation_ref: record_ref("native/child-service-context-bound"),
                    observation: &first_observation,
                },
                &mut ProvingAuthority,
                |actual, obligations| {
                    assert_eq!(actual, &evidence);
                    assert_eq!(
                        obligations,
                        child.historical_operation_lease().evidence_obligations()
                    );
                    Ok(())
                },
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(signature()),
            )
            .unwrap();
            let (physical, context_fields) = super::super::prepare_append_with_ref(
                prepared.prepare_append(&resolve).unwrap(),
                child,
                current_checkpoint,
                port,
                67,
                &record_ref("native/child-service-context-checkpoint"),
            );
            let completed = physical
                .commit(port, anchor)
                .unwrap()
                .advance_anchor(anchor)
                .unwrap();
            let binding = prepared.finish(&completed, &resolve).unwrap();
            let checkpoint_ref = completed
                .candidate()
                .checkpoint()
                .successor()
                .coordinate()
                .record_ref()
                .clone();
            let checkpoint = read_checkpoint(
                completed.candidate().checkpoint().successor_record(),
                &context_fields,
            );
            (binding, checkpoint_ref, checkpoint)
        });
        let observation = OperationObservationAdmissionContextV1 {
            signed,
            handle_ref,
            handle,
            predecessor: Some(context_binding.as_ref().map_or(
                (first_record.coordinate().record_ref(), &first),
                |(binding, _, _)| binding.observation(),
            )),
        };
        let attempt_id = CapabilityAttemptIdV1::parse("native-child-service-attempt").unwrap();
        let kind = BoundedResidentTextV1::parse(CapabilityProvider::NativeRust.as_str()).unwrap();
        let version = BoundedResidentTextV1::parse(
            fixture
                .registry
                .contract(PATH_INVENTORY_ACTION_ID)
                .unwrap()
                .version
                .to_string(),
        )
        .unwrap();
        let result_schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
        let contracts = local_start_contract_set();
        let request_binding = request.local_request_binding().unwrap();
        let attempt = NativeOperationAttemptInput {
            context: OperationAttemptContext {
                operation: child,
                request,
                session,
                current,
                lease: child.historical_operation_lease(),
                context_binding: context_binding.as_ref().map(|(binding, _, _)| binding),
                generation: fixture.generation,
                selection_ref: record_ref("native/child-service-selection"),
                maximum_selection_bytes: 65_536,
                observation_ref: record_ref("native/child-service-prepared"),
                observation,
            },
            registry: fixture.registry,
            attempt_id: &attempt_id,
            result_admission: CapabilitySuccessAdmissionContextV1 {
                result: CapabilityResultAdmissionContextV1 {
                    expected_schema_sha256: &result_schema,
                    expected_request_binding: &request_binding,
                    expected_attempt_id: &attempt_id,
                    expected_implementation_kind: &kind,
                    expected_implementation_version: &version,
                    signing_authority: fixture.parents.signing,
                },
                contract_set_ref: request_binding.contract_set_ref(),
                contract_set: &contracts,
            },
        };
        let checkpoint = context_binding
            .as_ref()
            .map_or(current_checkpoint, |(_, reference, checkpoint)| {
                (reference, checkpoint)
            });
        let completed = super::super::service::run(
            super::super::service::Case::Success,
            attempt,
            checkpoint,
            port,
            anchor,
        )
        .unwrap();
        assert_eq!(completed.observation().1.operation_handle_ref(), handle_ref);
        assert_eq!(
            completed.observation().1.operation_handle_sha256(),
            handle.canonical_operation_handle_sha256()
        );
        assert_ne!(
            completed.observation().1.operation_handle_ref(),
            fixture.operation.handle().0
        );
        let mut final_fields = None;
        ControlStateCheckpointV1::admit_signed_wire(
            serde_json::from_slice(
                completed
                    .custody()
                    .candidate()
                    .checkpoint()
                    .successor_record()
                    .canonical_record_jcs()
                    .as_slice(),
            )
            .unwrap(),
            &current_checkpoint_fields.schema_sha256,
            &checkpoint_authority(current_checkpoint_fields),
            |domain, preimage, digest, actual| {
                verify_checkpoint_signature(domain, preimage, digest, actual)?;
                final_fields = Some(preimage.clone());
                Ok(())
            },
        )
        .unwrap();
        (completed, final_fields.unwrap())
    })
}

pub(super) fn recover(
    child: &IssuedLocalOperation,
    request: &CommonRequestEnvelopeV1,
    parents: &RecoveryParents<'_>,
    generation: &AdmittedCompiledGeneration,
    evidence: (&CompletedNativeOperation, &ControlStateCheckpointPreimageV1),
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut IndependentAnchor,
) -> RecoveredOperationObservation {
    let (completed, fields) = evidence;
    let coordinate = port.coordinate.clone();
    let writes = port.connection.total_changes();
    let terminal = completed.observation().1;
    let latest = read_operation_observation_for_handle(
        port,
        &coordinate,
        child.handle().0,
        child.handle().1,
        65_536,
    )
    .unwrap()
    .unwrap();
    // The durable index and bounded bodies select an untrusted history lookup;
    // only the protected traversal below admits the dispatch, staging member,
    // and terminal head.
    let claimed: OperationObservationWireV1 =
        serde_json::from_slice(latest.observation().canonical_record_jcs().as_slice()).unwrap();
    let staged_ref = claimed.predecessor_ref().as_ref().unwrap();
    let staged_digest = claimed.predecessor_sha256().as_ref().unwrap();
    assert_eq!(
        staged_ref,
        &record_ref("native/child-service-outcome-observation")
    );
    let staged = port
        .read_operation_record(
            &coordinate,
            &ControlRecordCoordinate::new(
                staged_ref.clone(),
                ControlRecordDigest::parse(staged_digest.as_str()).unwrap(),
            ),
            65_536,
        )
        .unwrap()
        .unwrap();
    let staged: OperationObservationWireV1 =
        serde_json::from_slice(staged.canonical_record_jcs().as_slice()).unwrap();
    let dispatch_ref = staged.predecessor_ref().as_ref().unwrap();
    let dispatch_digest = staged.predecessor_sha256().as_ref().unwrap();
    assert_eq!(dispatch_ref, &record_ref("native/child-service-dispatch"));
    let authority = checkpoint_authority(fields);
    let signers = AtomicUsize::new(0);
    let states = AtomicUsize::new(0);
    let has_context = !child
        .historical_operation_lease()
        .evidence_obligations()
        .is_empty();
    // Resolving, optional context, preparation, dispatch, durable outcome
    // staging, then terminal projection are all distinct signed observations.
    let history_length = if has_context { 6 } else { 5 };
    let child_suffix = child
        .handle()
        .1
        .operation_handle_id()
        .as_str()
        .strip_prefix("child-operation-")
        .expect("child fixture handle keeps its suffix");
    let initial_observation_ref = record_ref(&format!("child-observation/{child_suffix}"));
    let recovered = recover_operation_observation(
        OperationObservationRecoveryInput {
            store: &coordinate,
            operation: child,
            observation: OperationObservationSelection::HistoryMember {
                reference: dispatch_ref,
                digest: dispatch_digest,
            },
            checkpoint_authority: &authority,
            limits: OperationObservationRecoveryLimits::new(history_length, 1 << 20, 65_536)
                .unwrap(),
            checkpoint_limits: OperationCheckpointAncestryLimits::new(8, 1 << 20, 8).unwrap(),
            membership_limits: AccumulatorMembershipLimits::new(1, 10_000, 100_000).unwrap(),
        },
        port,
        anchor,
        OperationObservationRecoveryAdmission {
            admit_checkpoint: |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                ControlStateCheckpointV1::admit_signed_wire(
                    wire,
                    &fields.schema_sha256,
                    &authority,
                    verify_checkpoint_signature,
                )
            },
            resolve_signer: |reference: &ResidentRecordRefV1, wire: &OperationObservationWireV1| {
                signers.fetch_add(1, Ordering::SeqCst);
                match (has_context, wire.revision().get()) {
                    (_, 1) => assert_eq!(reference, &initial_observation_ref),
                    (true, 2) => {
                        assert_eq!(reference, &record_ref("native/child-service-context-bound"))
                    }
                    (true, 3) | (false, 2) => {
                        assert_eq!(reference, &record_ref("native/child-service-prepared"))
                    }
                    (true, 4) | (false, 3) => {
                        assert_eq!(reference, &record_ref("native/child-service-dispatch"))
                    }
                    (true, 5) | (false, 4) => {
                        assert_eq!(
                            reference,
                            &record_ref("native/child-service-outcome-observation")
                        )
                    }
                    (true, 6) | (false, 5) => {
                        assert_eq!(reference, &record_ref("native/child-service-terminal"))
                    }
                    _ => panic!("exact bounded child history"),
                }
                assert_eq!(
                    wire.deployment_runtime_authority_ref(),
                    child.handle().1.deployment_runtime_authority_ref()
                );
                assert_eq!(
                    wire.deployment_runtime_authority_sha256(),
                    child.handle().1.deployment_runtime_authority_sha256()
                );
                assert_eq!(wire.issuer_key_id(), child.handle().1.issuer_key_id());
                assert_eq!(wire.issuer_key_epoch(), child.handle().1.issuer_key_epoch());
                Ok(parents.signing.clone())
            },
            admit_state: |state: &OperationObservationStateV1| {
                states.fetch_add(1, Ordering::SeqCst);
                match state {
                    OperationObservationStateV1::Active {
                        phase: OperationActivePhaseV1::Resolving,
                    } => {}
                    OperationObservationStateV1::Active {
                        phase: OperationActivePhaseV1::ContextBound { evidence },
                    } => {
                        assert!(has_context);
                        assert_eq!(
                            evidence.record_ref,
                            record_ref("native/child-service-context")
                        );
                    }
                    OperationObservationStateV1::Active {
                        phase:
                            OperationActivePhaseV1::DispatchPrepared {
                                attempt_id,
                                evidence,
                            }
                            | OperationActivePhaseV1::Dispatched {
                                attempt_id,
                                evidence,
                            },
                    } => {
                        assert_eq!(attempt_id.as_str(), "native-child-service-attempt");
                        assert_eq!(
                            evidence.record_ref,
                            record_ref("native/child-service-selection")
                        );
                    }
                    _ => assert_eq!(state, terminal.state()),
                }
                Ok(())
            },
            admit_rotation: |_: &ResidentRecordRefV1,
                             _: &ArtifactDigest,
                             _: &ResidentRecordRefV1,
                             _: &ArtifactDigest| {
                panic!("no child authority rotation")
            },
            verify_signature: |domain: &[u8],
                               _: &OperationObservationPreimageV1,
                               _: &OperationObservationDigest,
                               actual: &CanonicalSignatureV1| {
                assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
                assert_eq!(actual, &signature());
                Ok(())
            },
        },
        verify_checkpoint_signature,
    )
    .unwrap();
    assert_eq!(
        (
            signers.load(Ordering::SeqCst),
            states.load(Ordering::SeqCst)
        ),
        (history_length, history_length)
    );
    assert_eq!(recovered.selected_head(), completed.observation());
    assert_eq!(recovered.selected().0, dispatch_ref);
    assert_eq!(recovered.store(), &coordinate);
    let dispatched = recovered.selected().1;
    let OperationObservationStateV1::Active {
        phase:
            OperationActivePhaseV1::Dispatched {
                evidence: selection,
                ..
            },
    } = dispatched.state()
    else {
        panic!("protected selected child dispatch");
    };
    let selection = port
        .read_operation_record(
            &coordinate,
            &ControlRecordCoordinate::new(
                selection.record_ref.clone(),
                ControlRecordDigest::parse(selection.record_sha256.as_str()).unwrap(),
            ),
            65_536,
        )
        .unwrap()
        .unwrap();
    let registry = NativeActionRegistry::new().unwrap();
    let contracts = local_start_contract_set();
    let history = || OperationAttemptHistoryInput {
        operation: child,
        request,
        session: child.historical_session_lease(),
        lease: child.historical_operation_lease(),
        generation,
        dispatched,
        selection: &selection,
        maximum_selection_bytes: 65_536,
    };
    match recovered.selected_head().1.state() {
        OperationObservationStateV1::Succeeded { result, .. } => {
            let result_coordinate = ControlRecordCoordinate::new(
                result.record_ref.clone(),
                ControlRecordDigest::parse(result.record_sha256.as_str()).unwrap(),
            );
            let result = port
                .read_operation_record(&coordinate, &result_coordinate, 65_536)
                .unwrap()
                .unwrap();
            let admitted = admit_native_operation_result(
                NativeOperationResultInput {
                    history: history(),
                    registry: &registry,
                    contract_set: &contracts,
                    terminal: NativeOperationTerminalEvidence::Recovered(&recovered),
                    result: &result,
                    maximum_result_bytes: 65_536,
                },
                |domain, _, _, actual| {
                    assert_eq!(domain, CAPABILITY_RESULT_V1_SIGNATURE_DOMAIN);
                    assert_eq!(actual, &signature());
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(&admitted, completed.result().unwrap());
        }
        OperationObservationStateV1::Failed {
            result: None,
            diagnostic,
        } => {
            let diagnostic_coordinate = ControlRecordCoordinate::new(
                diagnostic.record_ref.clone(),
                ControlRecordDigest::parse(diagnostic.record_sha256.as_str()).unwrap(),
            );
            let diagnostic = port
                .read_operation_record(&coordinate, &diagnostic_coordinate, 65_536)
                .unwrap()
                .unwrap();
            let admitted = admit_native_operation_failure(NativeOperationFailureEvidenceInput {
                history: history(),
                registry: &registry,
                contract_set: &contracts,
                terminal: NativeOperationTerminalEvidence::Recovered(&recovered),
                diagnostic: &diagnostic,
                maximum_diagnostic_bytes: 65_536,
            })
            .unwrap();
            assert_eq!(&admitted, completed.result().unwrap_err());
        }
        state => panic!("protected child head is not a native terminal state: {state:?}"),
    }
    // The parent may have advanced the shared checkpoint domain after this
    // child terminated. Protected recovery above proves the historical child
    // head; exact record reads below prove its retained typed bytes without
    // misclassifying that historical checkpoint as the store's current head.
    for record in completed.custody().candidate().records() {
        let retained = port
            .read_operation_record(&coordinate, record.record().coordinate(), 65_536)
            .unwrap()
            .expect("every protected child record remains retained");
        assert_eq!(&retained, record.record());
    }
    assert_eq!(
        port.connection.total_changes(),
        writes,
        "cold child result admission is read-only"
    );
    recovered
}
