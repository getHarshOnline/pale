//! Behavioral law tests for durable operation contracts.

use std::cell::Cell;

use super::*;

fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn schema_digest(byte: char) -> SchemaDigest {
    SchemaDigest::parse(sha(byte)).expect("schema digest")
}

fn artifact_digest(byte: char) -> ArtifactDigest {
    ArtifactDigest::parse(sha(byte)).expect("artifact digest")
}

fn record_ref(value: &str) -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse(value).expect("record reference")
}

fn signature() -> CanonicalSignatureV1 {
    CanonicalSignatureV1::parse("A".repeat(86)).expect("signature")
}

fn wire<T: Serialize, W: serde::de::DeserializeOwned>(record: &T) -> W {
    serde_json::from_value(serde_json::to_value(record).expect("record serializes"))
        .expect("wire deserializes")
}

fn signing_authority() -> SigningAuthorityBindingV1 {
    signing_authority_at("runtime-authority", '1')
}

fn signing_authority_at(
    runtime_authority_ref: &str,
    runtime_authority_digest: char,
) -> SigningAuthorityBindingV1 {
    SigningAuthorityBindingV1::integrated_runtime(
        ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        ResidentKeyIdV1::parse("key-1").expect("key"),
        SigningKeyEpochV1::new(1).expect("key epoch"),
        IntegratedRuntimeAuthorityV1 {
            deployment_runtime_authority_ref: record_ref(runtime_authority_ref),
            deployment_runtime_authority_sha256: artifact_digest(runtime_authority_digest),
            deployment_binding_ref: record_ref("deployment-binding"),
            deployment_binding_sha256: artifact_digest('2'),
            checked_principal_trust_checkpoint_ref: record_ref("trust-checkpoint"),
            checked_principal_trust_checkpoint_sha256: artifact_digest('3'),
        },
    )
    .expect("signing authority")
}

fn request_binding() -> RequestBindingV1 {
    RequestBindingV1::new(RequestBindingPreimageV1 {
        request_id: ResidentRequestIdV1::parse("request-1").expect("request"),
        correlation_id: ResidentCorrelationIdV1::parse("correlation-1").expect("correlation"),
        capability_id: CapabilityIdV1::parse("operation.observe").expect("capability"),
        canonical_request_sha256: CommonRequestEnvelopeDigest::parse(sha('4'))
            .expect("request digest"),
        session_grant_id: SessionGrantIdV1::parse("grant-1").expect("grant"),
        session_grant_sha256: artifact_digest('5'),
        deployment_binding_ref: record_ref("deployment-binding"),
        deployment_binding_sha256: artifact_digest('6'),
        route_binding_schema_id: SchemaId::parse("pale/local-route-binding.v1")
            .expect("route schema"),
        route_binding_schema_sha256: schema_digest('7'),
        route_binding_ref: record_ref("route-binding"),
        route_binding_sha256: artifact_digest('8'),
        route_generation: RouteGenerationV1::new(1).expect("route generation"),
        contract_set_ref: record_ref("contract-set"),
        contract_set_sha256: artifact_digest('9'),
        execution_owner_fence_ref: None,
        execution_owner_fence_sha256: None,
        federation_request_admission_ref: None,
        federation_request_admission_sha256: None,
    })
    .expect("request binding")
}

fn parent_session() -> SessionAdmissionLeaseV1 {
    SessionAdmissionLeaseV1::seal(
        SessionAdmissionLeasePreimageV1 {
            schema_id: SchemaId::parse(SESSION_ADMISSION_LEASE_V1_SCHEMA).expect("session schema"),
            schema_sha256: schema_digest('a'),
            session_admission_id: SessionAdmissionIdV1::parse("session-1").expect("session"),
            compiled_generation_digest: CompiledGenerationDigest::parse(sha('b'))
                .expect("generation"),
            active_generation_head_digest: ActiveGenerationHeadDigest::parse(sha('c'))
                .expect("active head"),
            activation_fence_digest: ActivationFenceDigest::parse(sha('d')).expect("fence"),
            deployment_binding_ref: record_ref("deployment-binding"),
            deployment_binding_sha256: artifact_digest('e'),
            deployment_runtime_authority_ref: record_ref("runtime-authority"),
            deployment_runtime_authority_sha256: artifact_digest('1'),
            principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
            controller_id: ResidentControllerIdV1::parse("controller-1").expect("controller"),
            relationship_scope: None,
            repository_id: LogicalRepositoryIdV1::parse("repository-1").expect("repository"),
            project_id: LogicalProjectIdV1::parse("project-1").expect("project"),
            execution_workspace_id: LogicalExecutionWorkspaceIdV1::parse("execution-workspace-1")
                .expect("execution workspace"),
            profile_digest: ProfileDigest::parse(sha('f')).expect("profile"),
            instance_digest: InstanceDigest::parse(sha('1')).expect("instance"),
            execution_path: ExecutionPathSelectionV1 {
                id: ExecutionPathSelectionIdV1::parse("direct-terminal").expect("path"),
                experience_path_id: "direct/terminal".into(),
            },
            harness_adapter_source_sha256: artifact_digest('2'),
            harness_adapter_build_sha256: artifact_digest('3'),
            harness_executable_sha256: artifact_digest('4'),
            harness_configuration_sha256: artifact_digest('5'),
            capability_ceiling: vec![CapabilityIdV1::parse("compute").expect("capability")],
            read_ceiling: vec![record_ref("read-a")],
            effect_ceiling: vec![EffectClass::Compute],
            budget_ceiling_units: LeaseBudgetUnitsV1::new(10).expect("budget"),
            timeout_ceiling_millis: LeaseTimeoutMillisV1::new(1_000).expect("timeout"),
            evidence_obligations: vec![record_ref("evidence-a")],
            evidence_destination: record_ref("evidence-destination"),
            issued_at: UnixTimeV1::new(10).expect("issued time"),
            expires_at: UnixTimeV1::new(100).expect("expiry"),
            revocation_epoch: PrincipalRevocationEpochV1::new(1).expect("revocation epoch"),
            checked_revocation_ref: None,
            checked_revocation_sha256: None,
            native_session_correlation: NativeSessionCorrelationV1::Unobserved {
                reason: ResidentReasonV1::parse("unavailable").expect("reason"),
            },
            issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
            issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
            issuer_key_epoch: SigningKeyEpochV1::new(1).expect("issuer epoch"),
        },
        signature(),
    )
    .expect("parent session")
}

fn operation_lease(parent: &SessionAdmissionLeaseV1) -> OperationLeaseV1 {
    OperationLeaseV1::seal(
        schema_digest('2'),
        parent,
        OperationLeaseRequestV1 {
            operation_lease_id: OperationLeaseIdV1::parse("lease-1").expect("lease"),
            operation: PackageResourceRefV1 {
                package_id: PackageIdV1::parse("operations").expect("package"),
                resource_id: PackageResourceIdV1::parse("delegate").expect("operation"),
            },
            capabilities: vec![CapabilityIdV1::parse("compute").expect("capability")],
            effects: vec![EffectClass::Compute],
            read_set: vec![record_ref("read-a")],
            evidence_obligations: vec![record_ref("evidence-a")],
            budget_units: LeaseBudgetUnitsV1::new(5).expect("budget"),
            timeout_millis: LeaseTimeoutMillisV1::new(500).expect("timeout"),
            expires_at: UnixTimeV1::new(90).expect("expiry"),
            invocation_correlation: InvocationCorrelationV1::Unobserved {
                reason: ResidentReasonV1::parse("unavailable").expect("reason"),
            },
        },
    )
    .expect("operation lease")
}

fn handle_preimage(lease: &OperationLeaseV1) -> OperationHandlePreimageV1 {
    OperationHandlePreimageV1 {
        schema_id: SchemaId::parse(OPERATION_HANDLE_V1_SCHEMA).expect("handle schema"),
        schema_sha256: schema_digest('4'),
        operation_handle_id: OperationHandleIdV1::parse("operation-1").expect("handle"),
        operation_class: OperationClassV1::Invoke,
        parent_operation: None,
        request_binding: request_binding(),
        operation_lease_ref: record_ref("lease/one"),
        operation_lease_sha256: lease.operation_lease_digest().clone(),
        compiled_generation_sha256: CompiledGenerationDigest::parse(sha('5')).expect("generation"),
        activation_fence_sha256: ActivationFenceDigest::parse(sha('6')).expect("fence"),
        instance_sha256: InstanceDigest::parse(sha('7')).expect("instance"),
        created_at: UnixTimeV1::new(20).expect("created"),
        expires_at: UnixTimeV1::new(80).expect("expiry"),
        retain_until: UnixTimeV1::new(120).expect("retention"),
        deployment_runtime_authority_ref: record_ref("runtime-authority"),
        deployment_runtime_authority_sha256: artifact_digest('1'),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("epoch"),
    }
}

fn evidence(value: &str, byte: char) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: record_ref(value),
        record_sha256: artifact_digest(byte),
    }
}

// Exercise contextual issuance and admission, not just the private phase
// relation. State-evidence callbacks are synthetic owner decisions here; this
// does not supply the missing live application reconciliation join.
fn assert_knowledge_chain(states: Vec<OperationObservationStateV1>) {
    let parent = parent_session();
    let lease = operation_lease(&parent);
    let handle = OperationHandleV1::seal(handle_preimage(&lease), signature()).expect("handle");
    let handle_ref = record_ref("operation/handle");
    let authority = signing_authority();
    let mut history: Vec<(ResidentRecordRefV1, OperationObservationV1, String)> = Vec::new();
    for (index, state) in states.into_iter().enumerate() {
        let predecessor = history
            .last()
            .map(|(reference, value, _)| (reference, value));
        let preimage = OperationObservationPreimageV1 {
            schema_id: SchemaId::parse(OPERATION_OBSERVATION_V1_SCHEMA).expect("schema"),
            schema_sha256: schema_digest('8'),
            operation_handle_id: handle.operation_handle_id().clone(),
            operation_handle_ref: handle_ref.clone(),
            operation_handle_sha256: handle.canonical_operation_handle_sha256().clone(),
            revision: OperationObservationRevisionV1::new(index as u64 + 1).expect("revision"),
            predecessor_ref: predecessor.map(|(reference, _)| reference.clone()),
            predecessor_sha256: predecessor
                .map(|(_, value)| value.canonical_operation_observation_sha256().clone()),
            state: state.clone(),
            observed_at: UnixTimeV1::new(81 + index as u64).expect("after execution expiry"),
            deployment_runtime_authority_ref: handle.deployment_runtime_authority_ref().clone(),
            deployment_runtime_authority_sha256: handle
                .deployment_runtime_authority_sha256()
                .clone(),
            issuer_principal_id: handle.issuer_principal_id().clone(),
            issuer_key_id: handle.issuer_key_id().clone(),
            issuer_key_epoch: *handle.issuer_key_epoch(),
        };
        let schema = preimage.schema_sha256.clone();
        let context = OperationObservationAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &schema,
                signing_authority: &authority,
            },
            handle_ref: &handle_ref,
            handle: &handle,
            predecessor,
        };
        let evidence_checked = Cell::new(false);
        let signed = Cell::new(false);
        let rejection = OperationObservationV1::issue_contextual(
            preimage.clone(),
            &context,
            |_| {
                evidence_checked.set(true);
                Err(contextual("test evidence owner", "not established"))
            },
            |_, _, _, _| Ok(()),
            |_, _, _| {
                signed.set(true);
                Ok(signature())
            },
        );
        assert!(rejection.is_err());
        assert!(
            evidence_checked.get(),
            "permitted knowledge successor must reach evidence admission"
        );
        assert!(!signed.get(), "unestablished evidence cannot be signed");
        let observation = OperationObservationV1::issue_contextual(
            preimage,
            &context,
            |candidate| {
                assert_eq!(candidate, &state);
                Ok(())
            },
            |_, _, _, _| Ok(()),
            |_, _, _| Ok(signature()),
        )
        .expect("later evidence is admissible without a new operation");
        let signature_checked = Cell::new(false);
        assert!(OperationObservationV1::admit_contextual_wire(
            wire(&observation),
            &context,
            |_| Err(contextual("test evidence owner", "not established")),
            |_, _, _, _| Ok(()),
            |_, _, _, _| {
                signature_checked.set(true);
                Ok(())
            },
        )
        .is_err());
        assert!(
            !signature_checked.get(),
            "wire admission must not bypass state evidence"
        );
        let admitted = OperationObservationV1::admit_contextual_wire(
            wire(&observation),
            &context,
            |candidate| {
                assert_eq!(candidate, &state);
                Ok(())
            },
            |_, _, _, _| Ok(()),
            |_, _, _, _| Ok(()),
        )
        .expect("same contextual wire law");
        assert_eq!(admitted, observation);
        if let Some((reference, previous)) = predecessor {
            observation
                .verify_admitted_successor_of(&handle_ref, &handle, reference, previous)
                .expect("same retained operation successor");
            assert!(observation.observed_at() > previous.observed_at());
            assert_ne!(
                observation.canonical_operation_observation_sha256(),
                previous.canonical_operation_observation_sha256()
            );
        }
        assert_eq!(
            observation.operation_handle_sha256(),
            handle.canonical_operation_handle_sha256()
        );
        let original_jcs =
            crate::canon::to_canonical_string(&observation).expect("canonical observation");
        history.push((
            record_ref(&format!("operation/observation-{}", index + 1)),
            observation,
            original_jcs,
        ));
        for (_, previous, original_jcs) in &history {
            assert_eq!(
                &crate::canon::to_canonical_string(previous)
                    .expect("retained canonical observation"),
                original_jcs
            );
            assert_eq!(previous.operation_handle_id(), handle.operation_handle_id());
        }
    }
}

#[test]
fn unknown_outcome_can_acquire_conclusive_evidence_on_the_same_operation() {
    assert_knowledge_chain(vec![
        unknown_outcome(),
        unknown_outcome(),
        successful_operation(),
    ]);
}

#[test]
fn expired_execution_allows_bounded_reconciliation_without_reactivation() {
    assert_knowledge_chain(vec![
        expired_execution(),
        recovering_operation(),
        unknown_outcome(),
        successful_operation(),
    ]);
}

#[test]
fn cancellation_request_does_not_erase_a_later_established_result() {
    assert_knowledge_chain(vec![cancellation_requested(), successful_operation()]);
    assert!(!cancellation_requested().is_terminal());
    assert!(successful_operation().is_terminal());
    assert!(!operation_state_phase_allows_successor(
        &successful_operation(),
        &cancelled_operation()
    ));
}

#[test]
fn cancellation_request_can_close_as_unknown_without_reopening_execution() {
    assert_knowledge_chain(vec![cancellation_requested(), unknown_outcome()]);
    assert!(operation_state_phase_allows_successor(
        &cancellation_requested(),
        &unknown_outcome()
    ));
    assert!(!operation_state_phase_allows_successor(
        &unknown_outcome(),
        &cancellation_requested()
    ));
    assert_state_successor_rejected(unknown_outcome(), cancellation_requested());
}

#[test]
fn uncertain_or_expired_observations_cannot_reopen_execution_or_downgrade_uncertainty() {
    let execution_states: Vec<_> = representative_states()
        .into_iter()
        .filter(|state| {
            matches!(
                state,
                OperationObservationStateV1::Active { .. }
                    | OperationObservationStateV1::CancellationRequested { .. }
            )
        })
        .collect();
    for before in [
        recovering_operation(),
        unknown_outcome(),
        expired_execution(),
    ] {
        assert!(!before.is_terminal());
        for execution in &execution_states {
            assert!(!operation_state_phase_allows_successor(&before, execution));
            assert_state_successor_rejected(before.clone(), execution.clone());
        }
    }
    assert!(!operation_state_phase_allows_successor(
        &unknown_outcome(),
        &recovering_operation()
    ));
    assert!(!operation_state_phase_allows_successor(
        &recovering_operation(),
        &expired_execution()
    ));
    assert!(!operation_state_phase_allows_successor(
        &unknown_outcome(),
        &expired_execution()
    ));
    assert_state_successor_rejected(
        unknown_outcome(),
        OperationObservationStateV1::OutcomeUnknown {
            diagnostic: evidence("diagnostic/different-unknown", '1'),
            reconciliation: ReconciliationFrontierReferenceV1 {
                record_ref: record_ref("reconciliation/new-frontier"),
                record_sha256: ReconciliationFrontierDigest::parse(sha('2')).expect("frontier"),
            },
            effect_receipt: None,
        },
    );
}

fn cancellation_requested() -> OperationObservationStateV1 {
    OperationObservationStateV1::CancellationRequested {
        request: evidence("cancel/request", '8'),
    }
}

fn recovering_operation() -> OperationObservationStateV1 {
    OperationObservationStateV1::Recovering {
        reconciliation: ReconciliationFrontierReferenceV1 {
            record_ref: record_ref("reconciliation/frontier"),
            record_sha256: ReconciliationFrontierDigest::parse(sha('9')).expect("frontier digest"),
        },
    }
}

fn successful_operation() -> OperationObservationStateV1 {
    OperationObservationStateV1::Succeeded {
        result: CapabilityResultReferenceV1 {
            record_ref: record_ref("result/success"),
            record_sha256: CapabilityResultDigest::parse(sha('a')).expect("result digest"),
        },
        effect_receipt: None,
    }
}

fn cancelled_operation() -> OperationObservationStateV1 {
    OperationObservationStateV1::Cancelled {
        diagnostic: evidence("diagnostic/cancelled", 'c'),
    }
}

fn unknown_outcome() -> OperationObservationStateV1 {
    OperationObservationStateV1::OutcomeUnknown {
        diagnostic: evidence("diagnostic/unknown", 'd'),
        reconciliation: ReconciliationFrontierReferenceV1 {
            record_ref: record_ref("reconciliation/unknown"),
            record_sha256: ReconciliationFrontierDigest::parse(sha('e')).expect("frontier digest"),
        },
        effect_receipt: None,
    }
}

fn expired_execution() -> OperationObservationStateV1 {
    OperationObservationStateV1::Expired {
        diagnostic: evidence("diagnostic/expired", 'f'),
    }
}

fn representative_states() -> Vec<OperationObservationStateV1> {
    vec![
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Resolving,
        },
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::ContextBound {
                evidence: evidence("context/bound", '7'),
            },
        },
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::ProviderResolved {
                provider_binding: ProviderBindingRefV1 {
                    provider_binding_id: "provider-1".into(),
                    provider_binding_identity: ProviderBindingIdentityV1 {
                        provider_binding_sha256: artifact_digest('6'),
                    },
                },
            },
        },
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::DispatchPrepared {
                attempt_id: CapabilityAttemptIdV1::parse("attempt-1").expect("attempt"),
                evidence: evidence("dispatch/prepared", '5'),
            },
        },
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Dispatched {
                attempt_id: CapabilityAttemptIdV1::parse("attempt-1").expect("attempt"),
                evidence: evidence("dispatch/committed", '4'),
            },
        },
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::CollectingConsultation {
                evidence: evidence("consult/collection", '3'),
            },
        },
        cancellation_requested(),
        recovering_operation(),
        successful_operation(),
        OperationObservationStateV1::Failed {
            result: None,
            diagnostic: evidence("diagnostic/failure", 'b'),
        },
        cancelled_operation(),
        unknown_outcome(),
        expired_execution(),
    ]
}

#[test]
fn operation_handle_requires_exact_admitted_parents_before_signature() {
    let parent = parent_session();
    let lease = operation_lease(&parent);
    let handle = OperationHandleV1::seal(handle_preimage(&lease), signature()).expect("handle");
    let authority = signing_authority();
    let context = OperationHandleAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: handle.schema_sha256(),
            signing_authority: &authority,
        },
        request_binding: handle.request_binding(),
        operation_lease_ref: handle.operation_lease_ref(),
        operation_lease: &lease,
        parent_operation: None,
        compiled_generation_sha256: handle.compiled_generation_sha256(),
        activation_fence_sha256: handle.activation_fence_sha256(),
        instance_sha256: handle.instance_sha256(),
    };
    let signature_checked = Cell::new(false);
    OperationHandleV1::admit_contextual_wire(
        wire(&handle),
        &context,
        |_, _| Ok(()),
        |domain, _, _, _| {
            assert_eq!(domain, OPERATION_HANDLE_V1_SIGNATURE_DOMAIN);
            signature_checked.set(true);
            Ok(())
        },
    )
    .expect("exact handle admits");
    assert!(signature_checked.get());

    let wrong_instance = InstanceDigest::parse(sha('3')).expect("instance");
    let wrong_context = OperationHandleAdmissionContextV1 {
        instance_sha256: &wrong_instance,
        ..context
    };
    assert!(OperationHandleV1::admit_contextual_wire(
        wire(&handle),
        &wrong_context,
        |_, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());

    let signature_attempted = Cell::new(false);
    assert!(OperationHandleV1::admit_contextual_wire(
        wire(&handle),
        &context,
        |_, _| Err(contextual("operation handle", "compiled authority meet")),
        |_, _, _, _| {
            signature_attempted.set(true);
            Ok(())
        },
    )
    .is_err());
    assert!(!signature_attempted.get());
}

#[test]
fn contextual_issuance_signs_only_the_admitted_handle_and_observation_preimages() {
    let parent = parent_session();
    let lease = operation_lease(&parent);
    let handle_preimage = handle_preimage(&lease);
    let mut wrong_handle_authority = handle_preimage.clone();
    wrong_handle_authority.deployment_runtime_authority_sha256 = artifact_digest('0');
    let authority = signing_authority();
    let expected_schema_sha256 = handle_preimage.schema_sha256.clone();
    let request_binding = handle_preimage.request_binding.clone();
    let operation_lease_ref = handle_preimage.operation_lease_ref.clone();
    let compiled_generation_sha256 = handle_preimage.compiled_generation_sha256.clone();
    let activation_fence_sha256 = handle_preimage.activation_fence_sha256.clone();
    let instance_sha256 = handle_preimage.instance_sha256.clone();
    let context = OperationHandleAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &expected_schema_sha256,
            signing_authority: &authority,
        },
        request_binding: &request_binding,
        operation_lease_ref: &operation_lease_ref,
        operation_lease: &lease,
        parent_operation: None,
        compiled_generation_sha256: &compiled_generation_sha256,
        activation_fence_sha256: &activation_fence_sha256,
        instance_sha256: &instance_sha256,
    };
    let signer_called = Cell::new(false);
    assert!(OperationHandleV1::issue_contextual(
        wrong_handle_authority,
        &context,
        |_, _| Ok(()),
        |_, _, _| {
            signer_called.set(true);
            Ok(signature())
        },
    )
    .is_err());
    assert!(!signer_called.get());
    let handle = OperationHandleV1::issue_contextual(
        handle_preimage,
        &context,
        |_, parent| {
            assert!(parent.is_none());
            Ok(())
        },
        |domain, preimage, digest| {
            assert_eq!(domain, OPERATION_HANDLE_V1_SIGNATURE_DOMAIN);
            assert_eq!(digest, &OperationHandleDigest::hash_canonical(preimage)?);
            Ok(signature())
        },
    )
    .expect("contextually issued handle");

    let observation_preimage = OperationObservationPreimageV1 {
        schema_id: SchemaId::parse(OPERATION_OBSERVATION_V1_SCHEMA).expect("observation schema"),
        schema_sha256: schema_digest('8'),
        operation_handle_id: handle.operation_handle_id().clone(),
        operation_handle_ref: record_ref("operation/handle"),
        operation_handle_sha256: handle.canonical_operation_handle_sha256().clone(),
        revision: OperationObservationRevisionV1::new(1).expect("revision"),
        predecessor_ref: None,
        predecessor_sha256: None,
        state: OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Resolving,
        },
        observed_at: handle.created_at().to_owned(),
        deployment_runtime_authority_ref: record_ref("runtime-authority"),
        deployment_runtime_authority_sha256: artifact_digest('1'),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("epoch"),
    };
    let mut wrong_observation_authority = observation_preimage.clone();
    wrong_observation_authority.deployment_runtime_authority_sha256 = artifact_digest('0');
    let observation_schema_sha256 = observation_preimage.schema_sha256.clone();
    let handle_ref = observation_preimage.operation_handle_ref.clone();
    let observation_context = OperationObservationAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &observation_schema_sha256,
            signing_authority: &authority,
        },
        handle_ref: &handle_ref,
        handle: &handle,
        predecessor: None,
    };
    let observation_signer_called = Cell::new(false);
    assert!(OperationObservationV1::issue_contextual(
        wrong_observation_authority,
        &observation_context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _| {
            observation_signer_called.set(true);
            Ok(signature())
        },
    )
    .is_err());
    assert!(!observation_signer_called.get());
    OperationObservationV1::issue_contextual(
        observation_preimage,
        &observation_context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |domain, preimage, digest| {
            assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
            assert_eq!(
                digest,
                &OperationObservationDigest::hash_canonical(preimage)?
            );
            Ok(signature())
        },
    )
    .expect("contextually issued initial observation");
}

#[test]
fn operation_handle_lifetime_is_ordered_and_lease_bounded() {
    let parent = parent_session();
    let lease = operation_lease(&parent);
    let mut unordered = handle_preimage(&lease);
    unordered.expires_at = unordered.created_at;
    assert!(OperationHandleV1::seal(unordered, signature()).is_err());

    let mut outlives_lease = handle_preimage(&lease);
    outlives_lease.expires_at = UnixTimeV1::new(95).expect("expiry");
    let handle = OperationHandleV1::seal(outlives_lease, signature()).expect("structural handle");
    let authority = signing_authority();
    let context = OperationHandleAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: handle.schema_sha256(),
            signing_authority: &authority,
        },
        request_binding: handle.request_binding(),
        operation_lease_ref: handle.operation_lease_ref(),
        operation_lease: &lease,
        parent_operation: None,
        compiled_generation_sha256: handle.compiled_generation_sha256(),
        activation_fence_sha256: handle.activation_fence_sha256(),
        instance_sha256: handle.instance_sha256(),
    };
    assert!(OperationHandleV1::admit_contextual_wire(
        wire(&handle),
        &context,
        |_, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn child_handle_binds_exact_parent_and_narrows_parent_operation_lease() {
    let session = parent_session();
    let parent_lease = operation_lease(&session);
    let parent_handle = OperationHandleV1::seal(handle_preimage(&parent_lease), signature())
        .expect("parent handle");
    let child_lease = OperationLeaseV1::seal(
        schema_digest('2'),
        &session,
        OperationLeaseRequestV1 {
            operation_lease_id: OperationLeaseIdV1::parse("lease-2").expect("lease"),
            operation: PackageResourceRefV1 {
                package_id: PackageIdV1::parse("operations").expect("package"),
                resource_id: PackageResourceIdV1::parse("consult").expect("operation"),
            },
            capabilities: vec![CapabilityIdV1::parse("compute").expect("capability")],
            effects: vec![],
            read_set: vec![record_ref("read-a")],
            evidence_obligations: vec![record_ref("evidence-a")],
            budget_units: LeaseBudgetUnitsV1::new(4).expect("budget"),
            timeout_millis: LeaseTimeoutMillisV1::new(400).expect("timeout"),
            expires_at: UnixTimeV1::new(85).expect("expiry"),
            invocation_correlation: InvocationCorrelationV1::Unobserved {
                reason: ResidentReasonV1::parse("unavailable").expect("reason"),
            },
        },
    )
    .expect("session-bounded child lease");
    let parent_handle_ref = record_ref("operation/parent");
    let mut child_preimage = handle_preimage(&child_lease);
    child_preimage.operation_handle_id =
        OperationHandleIdV1::parse("operation-2").expect("child handle");
    child_preimage.operation_class = OperationClassV1::Consult;
    child_preimage.parent_operation = Some(ParentOperationReferenceV1 {
        operation_handle_id: parent_handle.operation_handle_id().clone(),
        operation_handle_ref: parent_handle_ref.clone(),
        operation_handle_sha256: parent_handle.canonical_operation_handle_sha256().clone(),
    });
    let child_handle =
        OperationHandleV1::seal(child_preimage, signature()).expect("structural child handle");
    let authority = signing_authority();
    let context = OperationHandleAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: child_handle.schema_sha256(),
            signing_authority: &authority,
        },
        request_binding: child_handle.request_binding(),
        operation_lease_ref: child_handle.operation_lease_ref(),
        operation_lease: &child_lease,
        parent_operation: Some(ParentOperationAdmissionContextV1 {
            operation_handle_ref: &parent_handle_ref,
            operation_handle: &parent_handle,
            operation_lease: &parent_lease,
        }),
        compiled_generation_sha256: child_handle.compiled_generation_sha256(),
        activation_fence_sha256: child_handle.activation_fence_sha256(),
        instance_sha256: child_handle.instance_sha256(),
    };
    OperationHandleV1::admit_contextual_wire(
        wire(&child_handle),
        &context,
        |_, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .expect("exact narrowed child admits");

    let widened_lease = OperationLeaseV1::seal(
        schema_digest('2'),
        &session,
        OperationLeaseRequestV1 {
            operation_lease_id: OperationLeaseIdV1::parse("lease-3").expect("lease"),
            operation: child_lease.operation().clone(),
            capabilities: child_lease.capabilities().to_vec(),
            effects: child_lease.effects().to_vec(),
            read_set: child_lease.read_set().to_vec(),
            evidence_obligations: child_lease.evidence_obligations().to_vec(),
            budget_units: LeaseBudgetUnitsV1::new(6).expect("budget"),
            timeout_millis: *child_lease.timeout_millis(),
            expires_at: child_lease.expires_at(),
            invocation_correlation: child_lease.invocation_correlation().clone(),
        },
    )
    .expect("lease is still session-bounded");
    let mut widened_preimage = handle_preimage(&widened_lease);
    widened_preimage.operation_class = OperationClassV1::Delegate;
    widened_preimage.parent_operation = child_handle.parent_operation().clone();
    let widened_handle =
        OperationHandleV1::seal(widened_preimage, signature()).expect("structural handle");
    let widened_context = OperationHandleAdmissionContextV1 {
        operation_lease_ref: widened_handle.operation_lease_ref(),
        operation_lease: &widened_lease,
        request_binding: widened_handle.request_binding(),
        compiled_generation_sha256: widened_handle.compiled_generation_sha256(),
        activation_fence_sha256: widened_handle.activation_fence_sha256(),
        instance_sha256: widened_handle.instance_sha256(),
        ..context
    };
    assert!(OperationHandleV1::admit_contextual_wire(
        wire(&widened_handle),
        &widened_context,
        |_, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn only_dispatched_active_phase_exposes_a_binding_attempt() {
    let attempt = CapabilityAttemptIdV1::parse("attempt-1").expect("attempt");
    let dispatched = OperationObservationStateV1::Active {
        phase: OperationActivePhaseV1::Dispatched {
            attempt_id: attempt.clone(),
            evidence: evidence("dispatch/committed", '4'),
        },
    };
    let prepared = OperationObservationStateV1::Active {
        phase: OperationActivePhaseV1::DispatchPrepared {
            attempt_id: attempt.clone(),
            evidence: evidence("dispatch/prepared", '5'),
        },
    };
    assert_eq!(dispatched.dispatched_attempt_id(), Some(&attempt));
    assert_eq!(prepared.dispatched_attempt_id(), None);
}

#[test]
fn operation_state_classes_form_the_declared_successor_envelope() {
    let states = representative_states();
    for state in &states {
        assert!(operation_state_phase_allows_successor(state, state));
    }
    for left in &states {
        for right in &states {
            if operation_state_phase_allows_successor(left, right)
                && operation_state_phase_allows_successor(right, left)
            {
                assert_eq!(left, right);
            }
            for last in &states {
                if operation_state_phase_allows_successor(left, right)
                    && operation_state_phase_allows_successor(right, last)
                {
                    assert!(operation_state_phase_allows_successor(left, last));
                }
            }
        }
    }

    let alternate_attempt = OperationObservationStateV1::Active {
        phase: OperationActivePhaseV1::DispatchPrepared {
            attempt_id: CapabilityAttemptIdV1::parse("attempt-2").expect("attempt"),
            evidence: evidence("dispatch/other-preparation", '2'),
        },
    };
    let prepared = &states[3];
    assert!(!operation_state_phase_allows_successor(
        prepared,
        &alternate_attempt
    ));
    assert!(!operation_state_phase_allows_successor(
        &alternate_attempt,
        prepared
    ));
}

#[test]
fn only_open_consultation_can_change_evidence_without_changing_active_phase() {
    let first_open_frontier = OperationObservationStateV1::Active {
        phase: OperationActivePhaseV1::CollectingConsultation {
            evidence: evidence("consult/collection/one", '1'),
        },
    };
    let second_open_frontier = OperationObservationStateV1::Active {
        phase: OperationActivePhaseV1::CollectingConsultation {
            evidence: evidence("consult/collection/two", '2'),
        },
    };
    assert!(operation_state_phase_allows_successor(
        &first_open_frontier,
        &second_open_frontier
    ));
    assert!(operation_state_phase_allows_successor(
        &second_open_frontier,
        &first_open_frontier
    ));

    let first_stopped_frontier = OperationObservationStateV1::Active {
        phase: OperationActivePhaseV1::ConsultationCollected {
            evidence: evidence("consult/collected/one", '3'),
        },
    };
    let different_stopped_frontier = OperationObservationStateV1::Active {
        phase: OperationActivePhaseV1::ConsultationCollected {
            evidence: evidence("consult/collected/two", '4'),
        },
    };
    assert!(!operation_state_phase_allows_successor(
        &first_stopped_frontier,
        &different_stopped_frontier
    ));
}

fn assert_state_successor_rejected(
    predecessor_state: OperationObservationStateV1,
    successor_state: OperationObservationStateV1,
) {
    let parent = parent_session();
    let lease = operation_lease(&parent);
    let handle = OperationHandleV1::seal(handle_preimage(&lease), signature()).expect("handle");
    let handle_ref = record_ref("operation/handle");
    let first = OperationObservationV1::seal(
        OperationObservationPreimageV1 {
            schema_id: SchemaId::parse(OPERATION_OBSERVATION_V1_SCHEMA)
                .expect("observation schema"),
            schema_sha256: schema_digest('8'),
            operation_handle_id: handle.operation_handle_id().clone(),
            operation_handle_ref: handle_ref.clone(),
            operation_handle_sha256: handle.canonical_operation_handle_sha256().clone(),
            revision: OperationObservationRevisionV1::new(1).expect("revision"),
            predecessor_ref: None,
            predecessor_sha256: None,
            state: predecessor_state,
            observed_at: UnixTimeV1::new(30).expect("observation time"),
            deployment_runtime_authority_ref: record_ref("runtime-authority"),
            deployment_runtime_authority_sha256: artifact_digest('1'),
            issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
            issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("key"),
            issuer_key_epoch: SigningKeyEpochV1::new(1).expect("epoch"),
        },
        signature(),
    )
    .expect("predecessor observation");
    let predecessor_ref = record_ref("operation/observation/one");
    let successor = OperationObservationV1::seal(
        OperationObservationPreimageV1 {
            schema_id: first.schema_id().clone(),
            schema_sha256: first.schema_sha256().clone(),
            operation_handle_id: first.operation_handle_id().clone(),
            operation_handle_ref: first.operation_handle_ref().clone(),
            operation_handle_sha256: first.operation_handle_sha256().clone(),
            revision: OperationObservationRevisionV1::new(2).expect("revision"),
            predecessor_ref: Some(predecessor_ref.clone()),
            predecessor_sha256: Some(first.canonical_operation_observation_sha256().clone()),
            state: successor_state,
            observed_at: UnixTimeV1::new(31).expect("observation time"),
            deployment_runtime_authority_ref: first.deployment_runtime_authority_ref().clone(),
            deployment_runtime_authority_sha256: first
                .deployment_runtime_authority_sha256()
                .clone(),
            issuer_principal_id: first.issuer_principal_id().clone(),
            issuer_key_id: first.issuer_key_id().clone(),
            issuer_key_epoch: *first.issuer_key_epoch(),
        },
        signature(),
    )
    .expect("structural successor");
    let authority = signing_authority();
    let context = OperationObservationAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: successor.schema_sha256(),
            signing_authority: &authority,
        },
        handle_ref: &handle_ref,
        handle: &handle,
        predecessor: Some((&predecessor_ref, &first)),
    };
    assert!(OperationObservationV1::admit_contextual_wire(
        wire(&successor),
        &context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn cancellation_request_payload_cannot_change_at_the_same_observation_phase() {
    assert_state_successor_rejected(
        OperationObservationStateV1::CancellationRequested {
            request: evidence("cancellation/request/one", '1'),
        },
        OperationObservationStateV1::CancellationRequested {
            request: evidence("cancellation/request/two", '2'),
        },
    );
}

#[test]
fn recovery_frontier_payload_cannot_change_at_the_same_observation_phase() {
    assert_state_successor_rejected(
        OperationObservationStateV1::Recovering {
            reconciliation: ReconciliationFrontierReferenceV1 {
                record_ref: record_ref("reconciliation/frontier/one"),
                record_sha256: ReconciliationFrontierDigest::parse(sha('3'))
                    .expect("frontier digest"),
            },
        },
        OperationObservationStateV1::Recovering {
            reconciliation: ReconciliationFrontierReferenceV1 {
                record_ref: record_ref("reconciliation/frontier/two"),
                record_sha256: ReconciliationFrontierDigest::parse(sha('4'))
                    .expect("frontier digest"),
            },
        },
    );
}

#[test]
fn terminal_operation_observation_has_no_successor() {
    let parent = parent_session();
    let lease = operation_lease(&parent);
    let handle = OperationHandleV1::seal(handle_preimage(&lease), signature()).expect("handle");
    let first = OperationObservationV1::seal(
        OperationObservationPreimageV1 {
            schema_id: SchemaId::parse(OPERATION_OBSERVATION_V1_SCHEMA)
                .expect("observation schema"),
            schema_sha256: schema_digest('8'),
            operation_handle_id: handle.operation_handle_id().clone(),
            operation_handle_ref: record_ref("operation/handle"),
            operation_handle_sha256: handle.canonical_operation_handle_sha256().clone(),
            revision: OperationObservationRevisionV1::new(1).expect("revision"),
            predecessor_ref: None,
            predecessor_sha256: None,
            state: OperationObservationStateV1::Cancelled {
                diagnostic: evidence("diagnostic/cancelled", '9'),
            },
            observed_at: UnixTimeV1::new(80).expect("observation time"),
            deployment_runtime_authority_ref: record_ref("runtime-authority"),
            deployment_runtime_authority_sha256: artifact_digest('1'),
            issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
            issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("key"),
            issuer_key_epoch: SigningKeyEpochV1::new(1).expect("epoch"),
        },
        signature(),
    )
    .expect("terminal observation");
    let successor = OperationObservationV1::seal(
        OperationObservationPreimageV1 {
            schema_id: SchemaId::parse(OPERATION_OBSERVATION_V1_SCHEMA)
                .expect("observation schema"),
            schema_sha256: first.schema_sha256().clone(),
            operation_handle_id: first.operation_handle_id().clone(),
            operation_handle_ref: first.operation_handle_ref().clone(),
            operation_handle_sha256: first.operation_handle_sha256().clone(),
            revision: OperationObservationRevisionV1::new(2).expect("revision"),
            predecessor_ref: Some(record_ref("observation/one")),
            predecessor_sha256: Some(first.canonical_operation_observation_sha256().clone()),
            state: OperationObservationStateV1::Cancelled {
                diagnostic: evidence("diagnostic/cancelled", '9'),
            },
            observed_at: UnixTimeV1::new(81).expect("observation time"),
            deployment_runtime_authority_ref: first.deployment_runtime_authority_ref().clone(),
            deployment_runtime_authority_sha256: first
                .deployment_runtime_authority_sha256()
                .clone(),
            issuer_principal_id: first.issuer_principal_id().clone(),
            issuer_key_id: first.issuer_key_id().clone(),
            issuer_key_epoch: *first.issuer_key_epoch(),
        },
        signature(),
    )
    .expect("structural successor");
    let authority = signing_authority();
    let predecessor_ref = record_ref("observation/one");
    let handle_ref = record_ref("operation/handle");
    let context = OperationObservationAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: successor.schema_sha256(),
            signing_authority: &authority,
        },
        handle_ref: &handle_ref,
        handle: &handle,
        predecessor: Some((&predecessor_ref, &first)),
    };
    assert!(OperationObservationV1::admit_contextual_wire(
        wire(&successor),
        &context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn operation_observation_authority_rotation_requires_exact_lineage_admission() {
    let parent = parent_session();
    let lease = operation_lease(&parent);
    let handle = OperationHandleV1::seal(handle_preimage(&lease), signature()).expect("handle");
    let handle_ref = record_ref("operation/handle");
    let first = OperationObservationV1::seal(
        OperationObservationPreimageV1 {
            schema_id: SchemaId::parse(OPERATION_OBSERVATION_V1_SCHEMA)
                .expect("observation schema"),
            schema_sha256: schema_digest('8'),
            operation_handle_id: handle.operation_handle_id().clone(),
            operation_handle_ref: handle_ref.clone(),
            operation_handle_sha256: handle.canonical_operation_handle_sha256().clone(),
            revision: OperationObservationRevisionV1::new(1).expect("revision"),
            predecessor_ref: None,
            predecessor_sha256: None,
            state: OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Resolving,
            },
            observed_at: UnixTimeV1::new(20).expect("observation time"),
            deployment_runtime_authority_ref: record_ref("runtime-authority"),
            deployment_runtime_authority_sha256: artifact_digest('1'),
            issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
            issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("key"),
            issuer_key_epoch: SigningKeyEpochV1::new(1).expect("epoch"),
        },
        signature(),
    )
    .expect("first observation");
    let predecessor_ref = record_ref("operation/observation/one");
    let successor = OperationObservationV1::seal(
        OperationObservationPreimageV1 {
            schema_id: first.schema_id().clone(),
            schema_sha256: first.schema_sha256().clone(),
            operation_handle_id: first.operation_handle_id().clone(),
            operation_handle_ref: first.operation_handle_ref().clone(),
            operation_handle_sha256: first.operation_handle_sha256().clone(),
            revision: OperationObservationRevisionV1::new(2).expect("revision"),
            predecessor_ref: Some(predecessor_ref.clone()),
            predecessor_sha256: Some(first.canonical_operation_observation_sha256().clone()),
            state: OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::ContextBound {
                    evidence: evidence("context/bound", '2'),
                },
            },
            observed_at: UnixTimeV1::new(21).expect("observation time"),
            deployment_runtime_authority_ref: record_ref("runtime-authority/rotated"),
            deployment_runtime_authority_sha256: artifact_digest('0'),
            issuer_principal_id: first.issuer_principal_id().clone(),
            issuer_key_id: first.issuer_key_id().clone(),
            issuer_key_epoch: *first.issuer_key_epoch(),
        },
        signature(),
    )
    .expect("rotated-authority successor");
    let authority = signing_authority_at("runtime-authority/rotated", '0');
    let context = OperationObservationAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: successor.schema_sha256(),
            signing_authority: &authority,
        },
        handle_ref: &handle_ref,
        handle: &handle,
        predecessor: Some((&predecessor_ref, &first)),
    };
    let signature_checked = Cell::new(false);
    assert!(OperationObservationV1::admit_contextual_wire(
        wire(&successor),
        &context,
        |_| Ok(()),
        |_, _, _, _| Err(contextual("operation observation", "same runtime lineage")),
        |_, _, _, _| {
            signature_checked.set(true);
            Ok(())
        },
    )
    .is_err());
    assert!(!signature_checked.get());

    let rotation_checked = Cell::new(false);
    OperationObservationV1::admit_contextual_wire(
        wire(&successor),
        &context,
        |_| Ok(()),
        |predecessor_authority_ref,
         predecessor_authority_sha256,
         successor_authority_ref,
         successor_authority_sha256| {
            assert_eq!(
                predecessor_authority_ref,
                first.deployment_runtime_authority_ref()
            );
            assert_eq!(
                predecessor_authority_sha256,
                first.deployment_runtime_authority_sha256()
            );
            assert_eq!(
                successor_authority_ref,
                successor.deployment_runtime_authority_ref()
            );
            assert_eq!(
                successor_authority_sha256,
                successor.deployment_runtime_authority_sha256()
            );
            rotation_checked.set(true);
            Ok(())
        },
        |_, _, _, _| Ok(()),
    )
    .expect("admitted authority rotation preserves observation lineage");
    assert!(rotation_checked.get());
}

#[test]
fn observation_schema_requires_complete_predecessor_coordinate() {
    let schema = serde_json::to_value(schemars::schema_for!(OperationObservationWireV1))
        .expect("schema serializes");
    let dependencies = schema
        .get("dependentRequired")
        .and_then(serde_json::Value::as_object)
        .expect("predecessor dependencies");
    assert_eq!(
        dependencies["predecessor_ref"],
        serde_json::json!(["predecessor_sha256"])
    );
    assert_eq!(
        dependencies["predecessor_sha256"],
        serde_json::json!(["predecessor_ref"])
    );
}
