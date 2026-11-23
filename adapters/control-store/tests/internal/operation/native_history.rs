//! One authenticated traversal retains dispatch and head from the same history.
//! Authority, signatures and intermediate-state evidence are explicit fixture models.

use super::*;
use crate::resident_fixture::local_operation_start_fixture::{profile, RecoveryParents};
use crate::resident_fixture::shared_resident::RecoveryResolver;

/// The request here selects the historical authority fixture, not the returned
/// request. The latter must be decoded and admitted from the saved-handle index.
pub(super) fn recover_saved(
    input: &Input<'_>,
    expected_request: &CommonRequestEnvelopeV1,
    parents: &RecoveryParents<'_>,
    sessions: &dyn SessionLeaseCustodyPort,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> RecoveredLocalOperation {
    let locator = ControlRecordCoordinate::new(
        input.operation.handle().0.clone(),
        ControlRecordDigest::parse(
            input
                .operation
                .handle()
                .1
                .canonical_operation_handle_sha256()
                .as_str(),
        )
        .unwrap(),
    );
    let writes = port.connection.total_changes();
    let recovered =
        with_recovery_admission(input, expected_request, parents, |context, admission| {
            recover_local_operation_from_handle(
                LocalOperationHandleRecoveryInput {
                    context,
                    expected_handle: &locator,
                    maximum_parent_depth: 0,
                },
                port,
                sessions,
                &mut ReadOnlyAnchor(anchor),
                admission.admit_request,
                LocalOperationRecoveryAdmission {
                    admit_historical_authority: admission.admit_handle_authority,
                    verify_handle_signature: admission.verify_handle_signature,
                    verify_checkpoint_signature: admission.verify_checkpoint_signature,
                    initial_observation: OperationObservationRecoveryAdmission {
                        admit_checkpoint: admission.admit_checkpoint,
                        resolve_signer: admission.resolve_observation_signer,
                        admit_state: admission.admit_observation_state,
                        admit_rotation: admission.admit_rotation,
                        verify_signature: admission.verify_observation_signature,
                    },
                },
            )
        })
        .unwrap();
    assert_eq!(recovered.operation().handle(), input.operation.handle());
    assert_eq!(recovered.request(), expected_request);
    assert_eq!(port.connection.total_changes(), writes);
    recovered
}

pub(super) fn with_recovery_admission<R, E>(
    input: &Input<'_>,
    expected_request: &CommonRequestEnvelopeV1,
    parents: &RecoveryParents<'_>,
    run: impl FnOnce(
        LocalOperationRecoveryContext<'_>,
        OperationControlRecoveryAdmission<'_>,
    ) -> Result<R, E>,
) -> Result<R, E> {
    with_admission_count(input, expected_request, parents, &[], 1, run)
}

pub(super) fn with_sweep_admission<R, E>(
    input: &Input<'_>,
    expected_request: &CommonRequestEnvelopeV1,
    parents: &RecoveryParents<'_>,
    run: impl FnOnce(
        LocalOperationRecoveryContext<'_>,
        OperationControlRecoveryAdmission<'_>,
    ) -> Result<R, E>,
) -> Result<R, E> {
    with_admission_count(input, expected_request, parents, &[], 2, run)
}

pub(super) fn with_descendant_admission<R, E>(
    input: &Input<'_>,
    expected_request: &CommonRequestEnvelopeV1,
    parents: &RecoveryParents<'_>,
    children: &[(&CommonRequestEnvelopeV1, &OperationHandleV1)],
    run: impl FnOnce(
        LocalOperationRecoveryContext<'_>,
        OperationControlRecoveryAdmission<'_>,
    ) -> Result<R, E>,
) -> Result<R, E> {
    with_admission_count(
        input,
        expected_request,
        parents,
        children,
        2 + children.len(),
        run,
    )
}

fn with_admission_count<R, E>(
    input: &Input<'_>,
    expected_request: &CommonRequestEnvelopeV1,
    parents: &RecoveryParents<'_>,
    children: &[(&CommonRequestEnvelopeV1, &OperationHandleV1)],
    maximum: usize,
    run: impl FnOnce(
        LocalOperationRecoveryContext<'_>,
        OperationControlRecoveryAdmission<'_>,
    ) -> Result<R, E>,
) -> Result<R, E> {
    let mut resolver = RecoveryResolver::default();
    resolver.insert_generation(parents.current.generation.compiled_generation);
    resolver.insert_active(parents.current.generation.active_head);
    resolver.insert_fence(parents.current.generation.activation_fence);
    resolver.insert_effective_profile(parents.current.generation.effective_profile);
    let supported = profile();
    let request_admissions = AtomicUsize::new(0);
    let request_signatures = AtomicUsize::new(0);
    let authority = checkpoint_authority(input.checkpoint);
    let recovery_context = LocalOperationRecoveryContext {
        store: input.coordinate,
        profile: &supported,
        handle_signing_authority: input.signing,
        checkpoint_authority: &authority,
        historical: &resolver,
        maximum_record_bytes: 1 << 20,
        maximum_operation_lease_bytes: 1 << 20,
        maximum_session_lease_bytes: 1 << 20,
        membership_limits: AccumulatorMembershipLimits::new(
            3 + 3 * children.len(),
            10_000,
            100_000,
        )
        .unwrap(),
        observation_limits: OperationObservationRecoveryLimits::new(8, 1 << 20, 1 << 20).unwrap(),
        checkpoint_limits: OperationCheckpointAncestryLimits::new(8, 1 << 20, 8).unwrap(),
    };
    let mut admit_request = |_: &ResidentRecordRefV1, wire: CommonRequestEnvelopeWireV1| {
        request_admissions.fetch_add(1, Ordering::SeqCst);
        let expected_request = children
            .iter()
            .find(|(request, _)| {
                Some(request.request_id().as_str())
                    == serde_json::to_value(&wire).unwrap()["request_id"].as_str()
            })
            .map_or(expected_request, |(request, _)| *request);
        let expected = expected_request.to_preimage();
        let context = CommonRequestAdmissionContextV1 {
            expected_schema_sha256: &expected.envelope_schema_sha256,
            expected_body_schema_id: &expected.body.schema_id,
            expected_body_schema_sha256: &expected.body.schema_sha256,
            caller: &expected.caller,
            signing_authority: &expected.signing_authority_binding,
            route: &expected.route,
            session_admission: &expected.session_admission,
            operation_target: expected.operation_target.as_ref(),
            execution_owner_fence: None,
            federation_claim: None,
            workspace_id: &expected.workspace_id,
            capability_id: &expected.capability_id,
            operation: &expected.operation,
            scope: &expected.scope,
            risk_class: &expected.risk_class,
            resource_constraints: &expected.resource_constraints,
            expected_precondition: None,
            idempotency_key: &expected.idempotency_key,
            issued_at: expected.issued_at,
            expires_at: expected.expires_at,
            generation: expected.generation.as_ref(),
            fence_token: expected.fence_token.as_ref(),
        };

        CommonRequestEnvelopeV1::admit_contextual_wire(
            wire,
            &context,
            |preimage, digest, actual| {
                request_signatures.fetch_add(1, Ordering::SeqCst);
                assert_eq!(preimage, &expected);
                assert_eq!(digest, expected_request.canonical_request_sha256());
                assert_eq!(actual, expected_request.caller_signature());
                Ok(())
            },
        )
    };
    let mut admission = LocalOperationRecoveryAdmission {
        admit_historical_authority:
            |preimage: &OperationHandlePreimageV1, parent: Option<&OperationHandleV1>| {
                let expected_handle = children
                    .iter()
                    .find(|(_, handle)| {
                        handle.operation_handle_id() == &preimage.operation_handle_id
                    })
                    .map_or(input.operation.handle().1, |(_, handle)| *handle);
                if let Some(expected_parent) = expected_handle.parent_operation() {
                    let actual = parent.expect("descendant needs its exact admitted parent");
                    assert_eq!(
                        actual.operation_handle_id(),
                        &expected_parent.operation_handle_id
                    );
                    assert_eq!(
                        actual.canonical_operation_handle_sha256(),
                        &expected_parent.operation_handle_sha256
                    );
                } else {
                    assert!(parent.is_none());
                }
                assert_eq!(
                    &OperationHandleDigest::hash_canonical(preimage).unwrap(),
                    expected_handle.canonical_operation_handle_sha256()
                );
                Ok(())
            },
        verify_handle_signature: |domain: &[u8],
                                  _: &OperationHandlePreimageV1,
                                  _: &OperationHandleDigest,
                                  actual: &CanonicalSignatureV1| {
            assert_eq!(domain, OPERATION_HANDLE_V1_SIGNATURE_DOMAIN);
            assert_eq!(actual, &signature());
            Ok(())
        },
        verify_checkpoint_signature:
            |domain: &[u8],
             _: &ControlStateCheckpointPreimageV1,
             _: &ControlStateCheckpointDigest,
             actual: &CanonicalSignatureV1| {
                assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                assert_eq!(actual, &signature());
                Ok(())
            },
        initial_observation: OperationObservationRecoveryAdmission {
            admit_checkpoint: |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                ControlStateCheckpointV1::admit_signed_wire(
                    wire,
                    &input.checkpoint.schema_sha256,
                    &checkpoint_authority(input.checkpoint),
                    |domain, _, _, actual| {
                        assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                        assert_eq!(actual, &signature());
                        Ok(())
                    },
                )
            },
            resolve_signer: |reference: &ResidentRecordRefV1, _: &OperationObservationWireV1| {
                assert!(input.known.iter().any(|(known, _)| *known == reference));
                Ok(input.signing.clone())
            },
            admit_state: |state: &OperationObservationStateV1| {
                assert!(input.known.iter().any(|(_, known)| known.state() == state));
                Ok(())
            },
            admit_rotation: |_: &ResidentRecordRefV1,
                             _: &ArtifactDigest,
                             _: &ResidentRecordRefV1,
                             _: &ArtifactDigest| {
                panic!("native history fixture has no authority rotation")
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
    };
    let result = run(
        recovery_context,
        OperationControlRecoveryAdmission {
            admit_request: &mut admit_request,
            admit_handle_authority: &mut admission.admit_historical_authority,
            verify_handle_signature: &mut admission.verify_handle_signature,
            admit_checkpoint: &mut admission.initial_observation.admit_checkpoint,
            resolve_observation_signer: &mut admission.initial_observation.resolve_signer,
            admit_observation_state: &mut admission.initial_observation.admit_state,
            admit_rotation: &mut admission.initial_observation.admit_rotation,
            verify_observation_signature: &mut admission.initial_observation.verify_signature,
            verify_checkpoint_signature: &mut admission.verify_checkpoint_signature,
        },
    );
    if result.is_ok() {
        assert_eq!(request_admissions.load(Ordering::SeqCst), maximum);
    } else {
        assert!(request_admissions.load(Ordering::SeqCst) <= maximum);
    }
    assert_eq!(
        request_signatures.load(Ordering::SeqCst),
        request_admissions.load(Ordering::SeqCst)
    );
    result
}

/// Exercises the real producer, with fixture signatures only after its checks.
pub(super) fn prepare_terminal<'a>(
    outcome: &'a NativeOperationOutcome,
    recovered: Option<&RecoveredOperationObservation>,
    signing: SignedResidentAdmissionContextV1<'_>,
    label: &str,
    fault: Option<usize>,
    allow_signatures: bool,
) -> Result<TerminalEvidence<'a>, OperationAttemptError> {
    let head_ref = recovered.map(|value| value.selected_head().0);
    let evidence_ref = if fault == Some(0) {
        head_ref.unwrap().clone()
    } else {
        record_ref(&format!("native/{label}-evidence"))
    };
    let observation_ref = if fault == Some(1) {
        head_ref.unwrap().clone()
    } else {
        record_ref(&format!("native/{label}-observation"))
    };
    let observed_at = UnixTimeV1::new(if fault == Some(2) { 89 } else { 95 }).unwrap();
    match outcome {
        NativeOperationOutcome::Succeeded(value) => value
            .prepare_completion(
                NativeOperationCompletionInput {
                    recovered_history: recovered,
                    result_ref: evidence_ref,
                    observation_ref,
                    completed_at: UnixTimeV1::new(68).unwrap(),
                    observed_at,
                    signing,
                },
                |_, _, _| {
                    assert!(
                        allow_signatures,
                        "invalid history must fail before result signing"
                    );
                    Ok(signature())
                },
                |_, _, _, _| {
                    assert!(allow_signatures);
                    Ok(())
                },
                |_, _, _| {
                    assert!(
                        allow_signatures,
                        "invalid history must fail before observation signing"
                    );
                    Ok(signature())
                },
            )
            .map(|value| TerminalEvidence::Succeeded(Box::new(value))),
        NativeOperationOutcome::Failed(value) => value
            .prepare_failure(
                NativeOperationFailureInput {
                    recovered_history: recovered,
                    diagnostic_ref: evidence_ref,
                    observation_ref,
                    completed_at: UnixTimeV1::new(68).unwrap(),
                    observed_at,
                    signing,
                    maximum_diagnostic_bytes: 65_536,
                },
                |_, _, _, _| {
                    assert!(allow_signatures);
                    Ok(())
                },
                |_, _, _| {
                    assert!(
                        allow_signatures,
                        "invalid history must fail before failure signing"
                    );
                    Ok(signature())
                },
            )
            .map(|value| TerminalEvidence::Failed(Box::new(value))),
    }
}

pub(super) struct Intermediate {
    pub(super) reference: ResidentRecordRefV1,
    pub(super) observation: OperationObservationV1,
    pub(super) candidate: OperationCustodyCandidate,
    pub(super) checkpoint: ControlStateCheckpointV1,
    pub(super) fields: ControlStateCheckpointPreimageV1,
}

/// The interruption and its diagnostic/frontier admission are explicit fixture
/// authority, not a production delayed-completion or reconciliation issuer.
#[allow(clippy::too_many_arguments)]
pub(super) fn publish_intermediate(
    operation: &IssuedLocalOperation,
    dispatch: (&ResidentRecordRefV1, &OperationObservationV1),
    signing: &SigningAuthorityBindingV1,
    unknown: bool,
    prior: (&OperationCustodyCandidate, &ControlStateCheckpointV1),
    resolve: &dyn OperationObservationAuthorityStoreResolver,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> Intermediate {
    let (handle_ref, handle) = operation.handle();
    let reference = record_ref("native/fixture-intermediate");
    let diagnostic = ImmutableRecordReferenceV1 {
        record_ref: record_ref("native/fixture-intermediate-diagnostic"),
        record_sha256: ArtifactDigest::hash_bytes(b"fixture interruption diagnostic"),
    };
    let state = if unknown {
        OperationObservationStateV1::OutcomeUnknown {
            diagnostic,
            reconciliation: ReconciliationFrontierReferenceV1 {
                record_ref: record_ref("native/fixture-reconciliation-frontier"),
                record_sha256: ReconciliationFrontierDigest::parse("a".repeat(64)).unwrap(),
            },
            effect_receipt: None,
        }
    } else {
        OperationObservationStateV1::Expired { diagnostic }
    };
    let observation = issue_observation(
        signing,
        handle_ref,
        handle,
        dispatch.1.schema_sha256().clone(),
        OperationObservationRevisionV1::new(dispatch.1.revision().get() + 1).unwrap(),
        Some(dispatch),
        state,
        90,
    );
    let append = prepare_operation_observation_append(
        OperationObservationTransition::new(
            handle_ref,
            handle,
            dispatch.0,
            dispatch.1,
            &reference,
            &observation,
        ),
        resolve,
    )
    .unwrap();
    let (physical, fields) = prepare_append(
        append,
        operation,
        (
            prior.0.checkpoint().successor().coordinate().record_ref(),
            prior.1,
        ),
        port,
        91,
    );
    let committed = physical
        .commit(port, anchor)
        .unwrap()
        .advance_anchor(anchor)
        .unwrap();
    let candidate = committed.candidate().clone();
    let checkpoint = read_checkpoint(candidate.checkpoint().successor_record(), &fields);
    Intermediate {
        reference,
        observation,
        candidate,
        checkpoint,
        fields,
    }
}

pub(super) struct Input<'a> {
    pub(super) operation: &'a IssuedLocalOperation,
    pub(super) coordinate: &'a AuthorityStoreCoordinate,
    pub(super) selected_member: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    pub(super) known: &'a [(&'a ResidentRecordRefV1, &'a OperationObservationV1)],
    pub(super) checkpoint: &'a ControlStateCheckpointPreimageV1,
    pub(super) signing: &'a SigningAuthorityBindingV1,
}

struct ReadOnlyAnchor<'a>(&'a mut dyn AntiRollbackAnchorPort);

impl AntiRollbackAnchorPort for ReadOnlyAnchor<'_> {
    fn read(
        &mut self,
        key: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        self.0.read(key)
    }

    fn observe(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        panic!("history recovery only reads the exact retained anchor")
    }

    fn compare_and_advance(
        &mut self,
        _: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        panic!("history recovery cannot advance an anchor")
    }
}

pub(super) fn recover(
    input: Input<'_>,
    port: &mut SqliteOperationCustody<'_>,
    anchor: &mut dyn AntiRollbackAnchorPort,
) -> RecoveredOperationObservation {
    let (_, handle) = input.operation.handle();
    let writes = port.connection.total_changes();
    let admitted_states = AtomicUsize::new(0);
    let resolved_signers = AtomicUsize::new(0);
    let admission = OperationObservationRecoveryAdmission {
        admit_checkpoint: |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
            ControlStateCheckpointV1::admit_signed_wire(
                wire,
                &input.checkpoint.schema_sha256,
                &checkpoint_authority(input.checkpoint),
                |domain, _, _, actual| {
                    assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                    assert_eq!(actual, &signature());
                    Ok(())
                },
            )
        },
        resolve_signer: |reference: &ResidentRecordRefV1, wire: &OperationObservationWireV1| {
            resolved_signers.fetch_add(1, Ordering::SeqCst);
            assert!(input
                .known
                .iter()
                .any(|(expected, _)| *expected == reference));
            assert_eq!(
                wire.deployment_runtime_authority_ref(),
                handle.deployment_runtime_authority_ref()
            );
            assert_eq!(
                wire.deployment_runtime_authority_sha256(),
                handle.deployment_runtime_authority_sha256()
            );
            assert_eq!(wire.issuer_key_id(), handle.issuer_key_id());
            assert_eq!(wire.issuer_key_epoch(), handle.issuer_key_epoch());
            Ok(input.signing.clone())
        },
        admit_state: |state: &OperationObservationStateV1| {
            assert!(
                input
                    .known
                    .iter()
                    .any(|(_, record)| record.state() == state),
                "only explicitly fixture-admitted states retained in this journey"
            );
            admitted_states.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
        admit_rotation: |_: &ResidentRecordRefV1,
                         _: &ArtifactDigest,
                         _: &ResidentRecordRefV1,
                         _: &ArtifactDigest| {
            panic!("this fixture does not rotate authority")
        },
        verify_signature: |domain: &[u8],
                           _: &OperationObservationPreimageV1,
                           _: &OperationObservationDigest,
                           actual: &CanonicalSignatureV1| {
            assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
            assert_eq!(actual, &signature());
            Ok(())
        },
    };
    let recovered = recover_operation_observation(
        OperationObservationRecoveryInput {
            store: input.coordinate,
            operation: input.operation,
            observation: OperationObservationSelection::HistoryMember {
                reference: input.selected_member.0,
                digest: input
                    .selected_member
                    .1
                    .canonical_operation_observation_sha256(),
            },
            checkpoint_authority: &checkpoint_authority(input.checkpoint),
            limits: OperationObservationRecoveryLimits::new(input.known.len(), 1 << 20, 65_536)
                .unwrap(),
            checkpoint_limits: OperationCheckpointAncestryLimits::new(8, 1 << 20, 8).unwrap(),
            membership_limits: AccumulatorMembershipLimits::new(1, 10_000, 100_000).unwrap(),
        },
        port,
        &mut ReadOnlyAnchor(anchor),
        admission,
        |domain, _, _, actual| {
            assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
            assert_eq!(actual, &signature());
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(
        admitted_states.load(Ordering::SeqCst),
        input.known.len(),
        "one traversal admits every revision, including the selected member and later states"
    );
    assert_eq!(
        resolved_signers.load(Ordering::SeqCst),
        input.known.len(),
        "the selected member and head must come from one authenticated traversal"
    );
    assert_eq!(recovered.selected(), input.selected_member);
    assert_eq!(recovered.selected_head(), *input.known.last().unwrap());
    assert_eq!(recovered.store(), input.coordinate);
    assert_eq!(
        port.connection.total_changes(),
        writes,
        "history recovery cannot write"
    );
    recovered
}
