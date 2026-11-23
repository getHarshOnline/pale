//! Behavioral law tests for A2A admission and observation contracts.

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

fn evidence(value: &str, byte: char) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: record_ref(value),
        record_sha256: artifact_digest(byte),
    }
}

fn signature() -> CanonicalSignatureV1 {
    CanonicalSignatureV1::parse("A".repeat(86)).expect("signature")
}

fn wire<T: Serialize, W: serde::de::DeserializeOwned>(record: &T) -> W {
    serde_json::from_value(serde_json::to_value(record).expect("record serializes"))
        .expect("wire deserializes")
}

#[test]
fn a2a_https_endpoint_rejects_ambiguous_or_credential_bearing_urls() {
    for value in [
        "http://agent.example.test/a2a",
        "https://",
        "https:////agent.example.test/a2a",
        "https://user:secret@agent.example.test/a2a",
        "https://@agent.example.test/a2a",
        "https://agent.example.test/a2a#fragment",
        "https://agent.example.test\\a2a",
        "https://agent.example.test/%zz",
        "https://agent.example.test:99999/a2a",
        " https://agent.example.test/a2a",
    ] {
        assert!(
            matches!(
                A2aHttpsEndpointV1::parse(value),
                Err(InvalidResidentRecordV1::InvalidIdentity {
                    kind: "A2A HTTPS endpoint"
                })
            ),
            "unexpectedly admitted {value:?}"
        );
    }
}

#[test]
fn a2a_https_endpoint_stores_one_canonical_transport_identity() {
    let endpoint =
        A2aHttpsEndpointV1::parse("HTTPS://B\u{fc}CHER.example.test:443/a/../rpc?mode=delegate")
            .expect("canonical endpoint");

    assert_eq!(
        endpoint.as_str(),
        "https://xn--bcher-kva.example.test/rpc?mode=delegate"
    );
    assert_ne!(
        A2aHttpsEndpointV1::parse("https://agent.example.test/rpc")
            .expect("path without trailing slash"),
        A2aHttpsEndpointV1::parse("https://agent.example.test/rpc/")
            .expect("path with trailing slash")
    );
    assert_eq!(
        A2aHttpsEndpointV1::parse("https://agent.example.test/@owner?reply=@operator")
            .expect("at signs outside the authority are data")
            .as_str(),
        "https://agent.example.test/@owner?reply=@operator"
    );
}

#[test]
fn a2a_https_endpoint_applies_the_bound_after_canonicalization() {
    let expanded_path = format!("https://agent.example.test/{}", "\u{e9}".repeat(150));
    assert!(expanded_path.len() <= MAX_RESIDENT_ID_BYTES);
    assert!(matches!(
        A2aHttpsEndpointV1::parse(expanded_path),
        Err(InvalidResidentRecordV1::InvalidIdentity {
            kind: "A2A HTTPS endpoint"
        })
    ));
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
        capability_id: CapabilityIdV1::parse("operation.delegate").expect("capability"),
        canonical_request_sha256: CommonRequestEnvelopeDigest::parse(sha('4'))
            .expect("request digest"),
        session_grant_id: SessionGrantIdV1::parse("grant-1").expect("grant"),
        session_grant_sha256: artifact_digest('5'),
        deployment_binding_ref: record_ref("deployment-binding"),
        deployment_binding_sha256: artifact_digest('6'),
        route_binding_schema_id: SchemaId::parse("pale/a2a-route-binding.v1")
            .expect("route schema"),
        route_binding_schema_sha256: schema_digest('a'),
        route_binding_ref: record_ref("route/binding"),
        route_binding_sha256: artifact_digest('b'),
        route_generation: RouteGenerationV1::new(1).expect("route generation"),
        contract_set_ref: record_ref("contract/set"),
        contract_set_sha256: artifact_digest('c'),
        execution_owner_fence_ref: None,
        execution_owner_fence_sha256: None,
        federation_request_admission_ref: None,
        federation_request_admission_sha256: None,
    })
    .expect("request binding")
}

fn session() -> SessionAdmissionLeaseV1 {
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
                id: ExecutionPathSelectionIdV1::parse("remote-agent").expect("path"),
                experience_path_id: "remote/agent".into(),
            },
            harness_adapter_source_sha256: artifact_digest('2'),
            harness_adapter_build_sha256: artifact_digest('3'),
            harness_executable_sha256: artifact_digest('4'),
            harness_configuration_sha256: artifact_digest('5'),
            capability_ceiling: vec![CapabilityIdV1::parse("delegate").expect("capability")],
            read_ceiling: vec![record_ref("read-a")],
            effect_ceiling: vec![EffectClass::Compute],
            budget_ceiling_units: LeaseBudgetUnitsV1::new(10).expect("budget"),
            timeout_ceiling_millis: LeaseTimeoutMillisV1::new(1_000).expect("timeout"),
            evidence_obligations: vec![record_ref("evidence-a")],
            evidence_destination: record_ref("evidence-destination"),
            issued_at: UnixTimeV1::new(10).expect("issued"),
            expires_at: UnixTimeV1::new(100).expect("expiry"),
            revocation_epoch: PrincipalRevocationEpochV1::new(1).expect("revocation"),
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
    .expect("session")
}

fn operation_lease(session: &SessionAdmissionLeaseV1) -> OperationLeaseV1 {
    OperationLeaseV1::seal(
        schema_digest('2'),
        session,
        OperationLeaseRequestV1 {
            operation_lease_id: OperationLeaseIdV1::parse("lease-1").expect("lease"),
            operation: PackageResourceRefV1 {
                package_id: PackageIdV1::parse("operations").expect("package"),
                resource_id: PackageResourceIdV1::parse("delegate").expect("operation"),
            },
            capabilities: vec![CapabilityIdV1::parse("delegate").expect("capability")],
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

fn operation_handle(lease: &OperationLeaseV1) -> OperationHandleV1 {
    OperationHandleV1::seal(
        OperationHandlePreimageV1 {
            schema_id: SchemaId::parse(OPERATION_HANDLE_V1_SCHEMA).expect("handle schema"),
            schema_sha256: schema_digest('3'),
            operation_handle_id: OperationHandleIdV1::parse("operation-1").expect("handle"),
            operation_class: OperationClassV1::Delegate,
            parent_operation: None,
            request_binding: request_binding(),
            operation_lease_ref: record_ref("lease/one"),
            operation_lease_sha256: lease.operation_lease_digest().clone(),
            compiled_generation_sha256: CompiledGenerationDigest::parse(sha('4'))
                .expect("generation"),
            activation_fence_sha256: ActivationFenceDigest::parse(sha('5')).expect("fence"),
            instance_sha256: InstanceDigest::parse(sha('6')).expect("instance"),
            created_at: UnixTimeV1::new(20).expect("created"),
            expires_at: UnixTimeV1::new(80).expect("expiry"),
            retain_until: UnixTimeV1::new(120).expect("retention"),
            deployment_runtime_authority_ref: record_ref("runtime-authority"),
            deployment_runtime_authority_sha256: artifact_digest('1'),
            issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
            issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("key"),
            issuer_key_epoch: SigningKeyEpochV1::new(1).expect("epoch"),
        },
        signature(),
    )
    .expect("operation handle")
}

fn operation_observation(
    handle: &OperationHandleV1,
    state: OperationObservationStateV1,
) -> OperationObservationV1 {
    operation_observation_at(handle, state, 30)
}

fn operation_observation_at(
    handle: &OperationHandleV1,
    state: OperationObservationStateV1,
    observed_at: u64,
) -> OperationObservationV1 {
    OperationObservationV1::seal(
        OperationObservationPreimageV1 {
            schema_id: SchemaId::parse(OPERATION_OBSERVATION_V1_SCHEMA)
                .expect("observation schema"),
            schema_sha256: schema_digest('7'),
            operation_handle_id: handle.operation_handle_id().clone(),
            operation_handle_ref: record_ref("operation/handle"),
            operation_handle_sha256: handle.canonical_operation_handle_sha256().clone(),
            revision: OperationObservationRevisionV1::new(1).expect("revision"),
            predecessor_ref: None,
            predecessor_sha256: None,
            state,
            observed_at: UnixTimeV1::new(observed_at).expect("observed"),
            deployment_runtime_authority_ref: record_ref("runtime-authority"),
            deployment_runtime_authority_sha256: artifact_digest('1'),
            issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
            issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("key"),
            issuer_key_epoch: SigningKeyEpochV1::new(1).expect("epoch"),
        },
        signature(),
    )
    .expect("operation observation")
}

fn dispatched(handle: &OperationHandleV1) -> OperationObservationV1 {
    dispatched_at(handle, 30)
}

fn dispatched_at(handle: &OperationHandleV1, observed_at: u64) -> OperationObservationV1 {
    operation_observation_at(
        handle,
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::Dispatched {
                attempt_id: CapabilityAttemptIdV1::parse("attempt-1").expect("attempt"),
                evidence: evidence("dispatch/one", '8'),
            },
        },
        observed_at,
    )
}

fn provider_binding() -> ProviderBindingRefV1 {
    ProviderBindingRefV1 {
        provider_binding_id: "provider-1".into(),
        provider_binding_identity: ProviderBindingIdentityV1 {
            provider_binding_sha256: artifact_digest('9'),
        },
    }
}

fn common_route() -> CommonRequestRouteV1 {
    CommonRequestRouteV1 {
        origin_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("origin"),
        origin_node_id: ResidentNodeIdV1::parse("node-1").expect("origin node"),
        destination_principal_id: ResidentPrincipalIdV1::parse("principal-2").expect("destination"),
        destination_node_id: Some(ResidentNodeIdV1::parse("node-2").expect("destination node")),
        route_id: ResidentRouteIdV1::parse("route-1").expect("route"),
        route_generation: RouteGenerationV1::new(1).expect("generation"),
        transport: BoundedResidentTextV1::parse("a2a-json-rpc2-https").expect("transport"),
        route_binding_schema_id: SchemaId::parse("pale/a2a-route-binding.v1")
            .expect("route schema"),
        route_binding_schema_sha256: schema_digest('a'),
        route_binding_ref: record_ref("route/binding"),
        route_binding_sha256: artifact_digest('b'),
        contract_set_ref: record_ref("contract/set"),
        contract_set_sha256: artifact_digest('c'),
    }
}

fn route_preimage(streaming: bool) -> A2aRouteAdmissionPreimageV1 {
    A2aRouteAdmissionPreimageV1 {
        schema_id: SchemaId::parse(A2A_ROUTE_ADMISSION_V1_SCHEMA).expect("route schema"),
        schema_sha256: schema_digest('d'),
        route_admission_id: A2aRouteAdmissionIdV1::parse("a2a-route-1").expect("route"),
        provider_binding: provider_binding(),
        route: common_route(),
        endpoint: A2aHttpsEndpointV1::parse("https://agent.example.test/a2a").expect("endpoint"),
        protocol_version: A2aProtocolVersionV1::V1_0,
        request_binding: A2aRequestBindingV1::JsonRpc2OverVerifiedHttps,
        agent_card: evidence("agent-card/one", 'e'),
        streaming: if streaming {
            A2aStreamingAdmissionV1::Advertised {
                evidence: evidence("agent-card/streaming", 'f'),
            }
        } else {
            A2aStreamingAdmissionV1::NotAdvertised
        },
        security: A2aSecurityAdmissionV1 {
            selected_scheme_ids: vec![
                A2aSecuritySchemeIdV1::parse("oauth2").expect("security scheme")
            ],
            evidence: evidence("route/security", '1'),
        },
        observed_at: UnixTimeV1::new(10).expect("observed"),
        fresh_until: UnixTimeV1::new(50).expect("fresh"),
        expires_at: UnixTimeV1::new(60).expect("expiry"),
        deployment_runtime_authority_ref: record_ref("runtime-authority"),
        deployment_runtime_authority_sha256: artifact_digest('1'),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("epoch"),
    }
}

fn admitted_route(streaming: bool) -> A2aRouteAdmissionV1 {
    A2aRouteAdmissionV1::seal(route_preimage(streaming), signature()).expect("route admission")
}

fn task_binding_preimage(
    handle: &OperationHandleV1,
    observation: &OperationObservationV1,
    route: &A2aRouteAdmissionV1,
) -> A2aTaskBindingPreimageV1 {
    A2aTaskBindingPreimageV1 {
        schema_id: SchemaId::parse(A2A_TASK_BINDING_V1_SCHEMA).expect("binding schema"),
        schema_sha256: schema_digest('2'),
        task_binding_id: A2aTaskBindingIdV1::parse("task-binding-1").expect("binding"),
        operation_handle_id: handle.operation_handle_id().clone(),
        operation_handle_ref: record_ref("operation/handle"),
        operation_handle_sha256: handle.canonical_operation_handle_sha256().clone(),
        operation_observation_ref: record_ref("operation/observation"),
        operation_observation_sha256: observation.canonical_operation_observation_sha256().clone(),
        route_admission_id: route.route_admission_id().clone(),
        route_admission_ref: record_ref("a2a/route"),
        route_admission_sha256: route.canonical_a2a_route_admission_sha256().clone(),
        attempt_id: CapabilityAttemptIdV1::parse("attempt-1").expect("attempt"),
        json_rpc_request_id: A2aJsonRpcRequestIdV1::parse("rpc-1").expect("request"),
        request_mode: A2aTaskRequestModeV1::SendMessage,
        remote_task_id: A2aRemoteTaskIdV1::parse("task-1").expect("task"),
        remote_context_id: A2aRemoteContextIdV1::parse("context-1").expect("context"),
        request_evidence: evidence("send/request", '3'),
        initial_task_evidence: evidence("send/response", '4'),
        bound_at: UnixTimeV1::new(40).expect("bound"),
        deployment_runtime_authority_ref: record_ref("runtime-authority"),
        deployment_runtime_authority_sha256: artifact_digest('1'),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("epoch"),
    }
}

fn initial_task_observation_preimage(binding: &A2aTaskBindingV1) -> A2aTaskObservationPreimageV1 {
    let source = match binding.request_mode() {
        A2aTaskRequestModeV1::SendMessage => {
            A2aTaskObservationSourceV1::InitialSendMessageResponse {
                evidence: binding.initial_task_evidence().clone(),
            }
        }
        A2aTaskRequestModeV1::SendStreamingMessage => {
            A2aTaskObservationSourceV1::InitialStreamingEvent {
                event_id: Some(A2aSseEventIdV1::parse("event-initial").expect("event")),
                evidence: binding.initial_task_evidence().clone(),
            }
        }
    };
    A2aTaskObservationPreimageV1 {
        schema_id: SchemaId::parse(A2A_TASK_OBSERVATION_V1_SCHEMA).expect("observation schema"),
        schema_sha256: schema_digest('9'),
        task_binding_id: binding.task_binding_id().clone(),
        task_binding_ref: record_ref("a2a/task-binding"),
        task_binding_sha256: binding.canonical_a2a_task_binding_sha256().clone(),
        revision: A2aTaskObservationRevisionV1::new(1).expect("revision"),
        predecessor_ref: None,
        predecessor_sha256: None,
        source,
        remote_state: A2aRemoteTaskStateV1::Submitted,
        reconciliation: None,
        observed_at: binding.bound_at().to_owned(),
        deployment_runtime_authority_ref: record_ref("runtime-authority"),
        deployment_runtime_authority_sha256: artifact_digest('1'),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("epoch"),
    }
}

fn successor_task_observation_preimage(
    binding: &A2aTaskBindingV1,
    predecessor_ref: &ResidentRecordRefV1,
    predecessor: &A2aTaskObservationV1,
    source: A2aTaskObservationSourceV1,
    remote_state: A2aRemoteTaskStateV1,
    observed_at: u64,
) -> A2aTaskObservationPreimageV1 {
    A2aTaskObservationPreimageV1 {
        schema_id: SchemaId::parse(A2A_TASK_OBSERVATION_V1_SCHEMA).expect("observation schema"),
        schema_sha256: predecessor.schema_sha256().clone(),
        task_binding_id: binding.task_binding_id().clone(),
        task_binding_ref: record_ref("a2a/task-binding"),
        task_binding_sha256: binding.canonical_a2a_task_binding_sha256().clone(),
        revision: predecessor
            .revision()
            .checked_successor()
            .expect("successor revision"),
        predecessor_ref: Some(predecessor_ref.clone()),
        predecessor_sha256: Some(predecessor.canonical_a2a_task_observation_sha256().clone()),
        source,
        remote_state,
        reconciliation: None,
        observed_at: UnixTimeV1::new(observed_at).expect("observation time"),
        deployment_runtime_authority_ref: record_ref("runtime-authority"),
        deployment_runtime_authority_sha256: artifact_digest('1'),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("epoch"),
    }
}

#[test]
fn route_admission_requires_canonical_advertised_security_selection() {
    let mut duplicate = route_preimage(true);
    duplicate
        .security
        .selected_scheme_ids
        .push(A2aSecuritySchemeIdV1::parse("oauth2").expect("duplicate security scheme"));
    assert!(A2aRouteAdmissionV1::seal(duplicate, signature()).is_err());

    let route = admitted_route(true);
    let authority = signing_authority();
    let provider = provider_binding();
    let common_route = common_route();
    let context = A2aRouteAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: route.schema_sha256(),
            signing_authority: &authority,
        },
        provider_binding: &provider,
        route: &common_route,
    };
    let inspected = Cell::new(false);
    A2aRouteAdmissionV1::admit_contextual_wire(
        wire(&route),
        &context,
        |preimage| {
            assert_eq!(preimage.security.selected_scheme_ids[0].as_str(), "oauth2");
            inspected.set(true);
            Ok(())
        },
        |domain, _, _, _| {
            assert_eq!(domain, A2A_ROUTE_ADMISSION_V1_SIGNATURE_DOMAIN);
            Ok(())
        },
    )
    .expect("exact advertised security selection admits");
    assert!(inspected.get());
}

#[test]
fn no_client_authentication_still_requires_route_security_evidence() {
    let mut preimage = route_preimage(false);
    preimage.security.selected_scheme_ids.clear();
    let authority = signing_authority();
    let provider = provider_binding();
    let common_route = common_route();
    let schema = preimage.schema_sha256.clone();
    let context = A2aRouteAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &schema,
            signing_authority: &authority,
        },
        provider_binding: &provider,
        route: &common_route,
    };
    let inspected = Cell::new(false);
    let admitted = A2aRouteAdmissionV1::issue_contextual(
        preimage.clone(),
        &context,
        |value| {
            assert!(value.security.selected_scheme_ids.is_empty());
            assert_eq!(value.endpoint.as_str(), "https://agent.example.test/a2a");
            assert_eq!(value.security.evidence, evidence("route/security", '1'));
            inspected.set(true);
            Ok(())
        },
        |_, _, _| Ok(signature()),
    )
    .expect("an explicitly qualified no-client-auth route admits");
    assert!(inspected.get());
    assert!(admitted.security().selected_scheme_ids.is_empty());

    let signed = Cell::new(false);
    assert!(A2aRouteAdmissionV1::issue_contextual(
        preimage,
        &context,
        |_| Err(contextual(
            "A2A route admission",
            "verified HTTPS peer evidence"
        )),
        |_, _, _| {
            signed.set(true);
            Ok(signature())
        },
    )
    .is_err());
    assert!(!signed.get());
    assert!(A2aRouteAdmissionV1::admit_contextual_wire(
        wire(&admitted),
        &context,
        |_| Err(contextual(
            "A2A route admission",
            "configured endpoint disclosure"
        )),
        |_, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn contextual_issuance_signs_only_the_admitted_a2a_preimages() {
    let authority = signing_authority();
    let provider = provider_binding();
    let common_route = common_route();
    let route_preimage = route_preimage(false);
    let mut wrong_route_authority = route_preimage.clone();
    wrong_route_authority.deployment_runtime_authority_sha256 = artifact_digest('0');
    let route_schema_sha256 = route_preimage.schema_sha256.clone();
    let route_context = A2aRouteAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &route_schema_sha256,
            signing_authority: &authority,
        },
        provider_binding: &provider,
        route: &common_route,
    };
    let route_signer_called = Cell::new(false);
    assert!(A2aRouteAdmissionV1::issue_contextual(
        wrong_route_authority,
        &route_context,
        |_| Ok(()),
        |_, _, _| {
            route_signer_called.set(true);
            Ok(signature())
        },
    )
    .is_err());
    assert!(!route_signer_called.get());
    let route = A2aRouteAdmissionV1::issue_contextual(
        route_preimage,
        &route_context,
        |_| Ok(()),
        |domain, preimage, digest| {
            assert_eq!(domain, A2A_ROUTE_ADMISSION_V1_SIGNATURE_DOMAIN);
            assert_eq!(digest, &A2aRouteAdmissionDigest::hash_canonical(preimage)?);
            Ok(signature())
        },
    )
    .expect("contextually issued route");

    let session = session();
    let lease = operation_lease(&session);
    let handle = operation_handle(&lease);
    let dispatch = dispatched(&handle);
    let binding_preimage = task_binding_preimage(&handle, &dispatch, &route);
    let mut wrong_binding_authority = binding_preimage.clone();
    wrong_binding_authority.deployment_runtime_authority_sha256 = artifact_digest('0');
    let binding_schema_sha256 = binding_preimage.schema_sha256.clone();
    let handle_ref = record_ref("operation/handle");
    let dispatch_ref = record_ref("operation/observation");
    let route_ref = record_ref("a2a/route");
    let binding_context = A2aTaskBindingAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &binding_schema_sha256,
            signing_authority: &authority,
        },
        operation_handle_ref: &handle_ref,
        operation_handle: &handle,
        operation_observation_ref: &dispatch_ref,
        operation_observation: &dispatch,
        route_admission_ref: &route_ref,
        route_admission: &route,
    };
    let binding_signer_called = Cell::new(false);
    assert!(A2aTaskBindingV1::issue_contextual(
        wrong_binding_authority,
        &binding_context,
        |_| Ok(()),
        |_, _, _| {
            binding_signer_called.set(true);
            Ok(signature())
        },
    )
    .is_err());
    assert!(!binding_signer_called.get());
    let binding = A2aTaskBindingV1::issue_contextual(
        binding_preimage,
        &binding_context,
        |_| Ok(()),
        |domain, preimage, digest| {
            assert_eq!(domain, A2A_TASK_BINDING_V1_SIGNATURE_DOMAIN);
            assert_eq!(digest, &A2aTaskBindingDigest::hash_canonical(preimage)?);
            Ok(signature())
        },
    )
    .expect("contextually issued task binding");

    let observation_preimage = initial_task_observation_preimage(&binding);
    let mut wrong_observation_authority = observation_preimage.clone();
    wrong_observation_authority.deployment_runtime_authority_sha256 = artifact_digest('0');
    let observation_schema_sha256 = observation_preimage.schema_sha256.clone();
    let binding_ref = record_ref("a2a/task-binding");
    let observation_context = A2aTaskObservationAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &observation_schema_sha256,
            signing_authority: &authority,
        },
        task_binding_ref: &binding_ref,
        task_binding: &binding,
        route_admission_ref: &route_ref,
        route_admission: &route,
        operation_handle_ref: &handle_ref,
        operation_handle: &handle,
        operation_observation: &dispatch,
        predecessor: None,
    };
    let observation_signer_called = Cell::new(false);
    assert!(A2aTaskObservationV1::issue_contextual(
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
    A2aTaskObservationV1::issue_contextual(
        observation_preimage,
        &observation_context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |domain, preimage, digest| {
            assert_eq!(domain, A2A_TASK_OBSERVATION_V1_SIGNATURE_DOMAIN);
            assert_eq!(digest, &A2aTaskObservationDigest::hash_canonical(preimage)?);
            Ok(signature())
        },
    )
    .expect("contextually issued task observation");
}

#[test]
fn task_binding_requires_a_real_task_and_required_context() {
    let session = session();
    let lease = operation_lease(&session);
    let handle = operation_handle(&lease);
    let observation = dispatched(&handle);
    let route = admitted_route(true);
    let binding = A2aTaskBindingV1::seal(
        task_binding_preimage(&handle, &observation, &route),
        signature(),
    )
    .expect("binding");
    let json = serde_json::to_value(&binding).expect("binding serializes");
    assert_eq!(json["remote_context_id"], "context-1");
    let mut missing_context = json;
    missing_context
        .as_object_mut()
        .expect("binding object")
        .remove("remote_context_id");
    assert!(serde_json::from_value::<A2aTaskBindingWireV1>(missing_context).is_err());

    let authority = signing_authority();
    let handle_ref = record_ref("operation/handle");
    let observation_ref = record_ref("operation/observation");
    let route_ref = record_ref("a2a/route");
    let context = A2aTaskBindingAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: binding.schema_sha256(),
            signing_authority: &authority,
        },
        operation_handle_ref: &handle_ref,
        operation_handle: &handle,
        operation_observation_ref: &observation_ref,
        operation_observation: &observation,
        route_admission_ref: &route_ref,
        route_admission: &route,
    };
    assert!(A2aTaskBindingV1::admit_contextual_wire(
        wire(&binding),
        &context,
        |_| Err(contextual("A2A task binding", "actual remote Task")),
        |_, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn fresh_task_binding_requires_the_exact_dispatched_attempt() {
    let session = session();
    let lease = operation_lease(&session);
    let handle = operation_handle(&lease);
    let dispatched = dispatched(&handle);
    let route = admitted_route(true);
    let binding = A2aTaskBindingV1::seal(
        task_binding_preimage(&handle, &dispatched, &route),
        signature(),
    )
    .expect("binding");
    let authority = signing_authority();
    let handle_ref = record_ref("operation/handle");
    let observation_ref = record_ref("operation/observation");
    let route_ref = record_ref("a2a/route");
    let context = A2aTaskBindingAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: binding.schema_sha256(),
            signing_authority: &authority,
        },
        operation_handle_ref: &handle_ref,
        operation_handle: &handle,
        operation_observation_ref: &observation_ref,
        operation_observation: &dispatched,
        route_admission_ref: &route_ref,
        route_admission: &route,
    };
    A2aTaskBindingV1::admit_contextual_wire(
        wire(&binding),
        &context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .expect("exact dispatched attempt admits");

    let mut wrong_attempt = task_binding_preimage(&handle, &dispatched, &route);
    wrong_attempt.attempt_id = CapabilityAttemptIdV1::parse("attempt-2").expect("attempt");
    let wrong_attempt = A2aTaskBindingV1::seal(wrong_attempt, signature()).expect("binding");
    assert!(A2aTaskBindingV1::admit_contextual_wire(
        wire(&wrong_attempt),
        &A2aTaskBindingAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: wrong_attempt.schema_sha256(),
                signing_authority: &authority,
            },
            ..context
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());

    for bound_at in [61, 81, 120] {
        let mut late = task_binding_preimage(&handle, &dispatched, &route);
        late.bound_at = UnixTimeV1::new(bound_at).expect("late binding time");
        let inspected = Cell::new(false);
        A2aTaskBindingV1::issue_contextual(
            late.clone(),
            &context,
            |value| {
                assert_eq!(value.request_evidence, evidence("send/request", '3'));
                assert_eq!(value.initial_task_evidence, evidence("send/response", '4'));
                inspected.set(true);
                Ok(())
            },
            |_, _, _| Ok(signature()),
        )
        .expect("exact valid dispatch can identify its task until retention ends");
        assert!(inspected.get());

        let signed = Cell::new(false);
        assert!(A2aTaskBindingV1::issue_contextual(
            late,
            &context,
            |_| Err(contextual(
                "A2A task binding",
                "exact dispatch response evidence"
            )),
            |_, _, _| {
                signed.set(true);
                Ok(signature())
            },
        )
        .is_err());
        assert!(!signed.get());
    }

    let mut outside_retention = task_binding_preimage(&handle, &dispatched, &route);
    outside_retention.bound_at = UnixTimeV1::new(121).expect("past retention");
    assert!(A2aTaskBindingV1::issue_contextual(
        outside_retention,
        &context,
        |_| panic!("retention failure must precede evidence admission"),
        |_, _, _| panic!("retention failure must precede signing"),
    )
    .is_err());

    let stale_dispatch = dispatched_at(&handle, 51);
    let mut longer_route = route_preimage(true);
    longer_route.fresh_until = UnixTimeV1::new(100).expect("fresh route");
    longer_route.expires_at = UnixTimeV1::new(110).expect("route expiry");
    let longer_route = A2aRouteAdmissionV1::seal(longer_route, signature()).expect("route");
    for dispatch_time in [19, 81] {
        let invalid_dispatch = dispatched_at(&handle, dispatch_time);
        let mut invalid = task_binding_preimage(&handle, &invalid_dispatch, &longer_route);
        invalid.bound_at = UnixTimeV1::new(90).expect("retained response");
        assert!(A2aTaskBindingV1::issue_contextual(
            invalid,
            &A2aTaskBindingAdmissionContextV1 {
                operation_observation: &invalid_dispatch,
                route_admission: &longer_route,
                ..context
            },
            |_| panic!("dispatch outside operation validity cannot identify new work"),
            |_, _, _| panic!("invalid dispatch cannot be signed"),
        )
        .is_err());
    }

    let mut stale_dispatch_preimage = task_binding_preimage(&handle, &stale_dispatch, &route);
    stale_dispatch_preimage.bound_at = UnixTimeV1::new(55).expect("response time");
    let stale_dispatch_binding =
        A2aTaskBindingV1::seal(stale_dispatch_preimage, signature()).expect("binding");
    assert!(A2aTaskBindingV1::admit_contextual_wire(
        wire(&stale_dispatch_binding),
        &A2aTaskBindingAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: stale_dispatch_binding.schema_sha256(),
                signing_authority: &authority,
            },
            operation_observation: &stale_dispatch,
            ..context
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());

    let mut late_route_preimage = route_preimage(true);
    late_route_preimage.observed_at = UnixTimeV1::new(31).expect("route observation time");
    let late_route =
        A2aRouteAdmissionV1::seal(late_route_preimage, signature()).expect("late route admission");
    let late_route_binding = A2aTaskBindingV1::seal(
        task_binding_preimage(&handle, &dispatched, &late_route),
        signature(),
    )
    .expect("binding observed before its route");
    assert!(A2aTaskBindingV1::admit_contextual_wire(
        wire(&late_route_binding),
        &A2aTaskBindingAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: late_route_binding.schema_sha256(),
                signing_authority: &authority,
            },
            route_admission: &late_route,
            ..context
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());

    let mut unrelated_route_preimage = route_preimage(true);
    unrelated_route_preimage.route.route_binding_sha256 = artifact_digest('d');
    let unrelated_route = A2aRouteAdmissionV1::seal(unrelated_route_preimage, signature())
        .expect("unrelated admitted route");
    let unrelated_binding = A2aTaskBindingV1::seal(
        task_binding_preimage(&handle, &dispatched, &unrelated_route),
        signature(),
    )
    .expect("structural unrelated-route binding");
    assert!(A2aTaskBindingV1::admit_contextual_wire(
        wire(&unrelated_binding),
        &A2aTaskBindingAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: unrelated_binding.schema_sha256(),
                signing_authority: &authority,
            },
            route_admission: &unrelated_route,
            ..context
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());

    for state in [
        OperationObservationStateV1::Active {
            phase: OperationActivePhaseV1::DispatchPrepared {
                attempt_id: CapabilityAttemptIdV1::parse("attempt-1").expect("attempt"),
                evidence: evidence("dispatch/prepared", '5'),
            },
        },
        OperationObservationStateV1::CancellationRequested {
            request: evidence("cancel/request", '6'),
        },
        OperationObservationStateV1::Recovering {
            reconciliation: ReconciliationFrontierReferenceV1 {
                record_ref: record_ref("reconciliation/frontier"),
                record_sha256: ReconciliationFrontierDigest::parse(sha('7')).expect("frontier"),
            },
        },
        OperationObservationStateV1::Expired {
            diagnostic: evidence("diagnostic/expired", '8'),
        },
    ] {
        let ineligible = operation_observation(&handle, state);
        let ineligible_context = A2aTaskBindingAdmissionContextV1 {
            operation_observation: &ineligible,
            ..context
        };
        assert!(A2aTaskBindingV1::admit_contextual_wire(
            wire(&binding),
            &ineligible_context,
            |_| Ok(()),
            |_, _, _, _| Ok(()),
        )
        .is_err());
    }
}

#[test]
fn streaming_task_identification_requires_advertisement_and_preserves_its_origin() {
    let session = session();
    let lease = operation_lease(&session);
    let handle = operation_handle(&lease);
    let dispatched = dispatched(&handle);
    let unavailable_route = admitted_route(false);
    let mut unavailable_preimage = task_binding_preimage(&handle, &dispatched, &unavailable_route);
    unavailable_preimage.request_mode = A2aTaskRequestModeV1::SendStreamingMessage;
    let unavailable_binding = A2aTaskBindingV1::seal(unavailable_preimage, signature())
        .expect("structural streaming binding");
    let authority = signing_authority();
    let handle_ref = record_ref("operation/handle");
    let observation_ref = record_ref("operation/observation");
    let route_ref = record_ref("a2a/route");
    let unavailable_context = A2aTaskBindingAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: unavailable_binding.schema_sha256(),
            signing_authority: &authority,
        },
        operation_handle_ref: &handle_ref,
        operation_handle: &handle,
        operation_observation_ref: &observation_ref,
        operation_observation: &dispatched,
        route_admission_ref: &route_ref,
        route_admission: &unavailable_route,
    };
    assert!(A2aTaskBindingV1::admit_contextual_wire(
        wire(&unavailable_binding),
        &unavailable_context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());

    let route = admitted_route(true);
    let mut binding_preimage = task_binding_preimage(&handle, &dispatched, &route);
    binding_preimage.request_mode = A2aTaskRequestModeV1::SendStreamingMessage;
    let binding =
        A2aTaskBindingV1::seal(binding_preimage, signature()).expect("streaming task binding");
    let binding_context = A2aTaskBindingAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: binding.schema_sha256(),
            signing_authority: &authority,
        },
        operation_handle_ref: &handle_ref,
        operation_handle: &handle,
        operation_observation_ref: &observation_ref,
        operation_observation: &dispatched,
        route_admission_ref: &route_ref,
        route_admission: &route,
    };
    A2aTaskBindingV1::admit_contextual_wire(
        wire(&binding),
        &binding_context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .expect("advertised streaming origin admits");

    let observation =
        A2aTaskObservationV1::seal(initial_task_observation_preimage(&binding), signature())
            .expect("initial streaming observation");
    let binding_ref = record_ref("a2a/task-binding");
    let observation_context = A2aTaskObservationAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: observation.schema_sha256(),
            signing_authority: &authority,
        },
        task_binding_ref: &binding_ref,
        task_binding: &binding,
        route_admission_ref: &route_ref,
        route_admission: &route,
        operation_handle_ref: &handle_ref,
        operation_handle: &handle,
        operation_observation: &dispatched,
        predecessor: None,
    };
    A2aTaskObservationV1::admit_contextual_wire(
        wire(&observation),
        &observation_context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .expect("matching initial stream evidence admits");

    let mut wrong_origin = initial_task_observation_preimage(&binding);
    wrong_origin.source = A2aTaskObservationSourceV1::InitialSendMessageResponse {
        evidence: binding.initial_task_evidence().clone(),
    };
    let wrong_origin = A2aTaskObservationV1::seal(wrong_origin, signature())
        .expect("structural wrong-origin observation");
    assert!(A2aTaskObservationV1::admit_contextual_wire(
        wire(&wrong_origin),
        &A2aTaskObservationAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: wrong_origin.schema_sha256(),
                signing_authority: &authority,
            },
            ..observation_context
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn admitted_task_remains_observable_after_route_expiry_until_handle_retention() {
    let session = session();
    let lease = operation_lease(&session);
    let handle = operation_handle(&lease);
    let operation_observation = dispatched(&handle);
    let route = admitted_route(false);
    let binding = A2aTaskBindingV1::seal(
        task_binding_preimage(&handle, &operation_observation, &route),
        signature(),
    )
    .expect("binding");
    let initial_observation =
        A2aTaskObservationV1::seal(initial_task_observation_preimage(&binding), signature())
            .expect("initial observation");
    let predecessor_ref = record_ref("a2a/observation/one");
    let task_observation = A2aTaskObservationV1::seal(
        successor_task_observation_preimage(
            &binding,
            &predecessor_ref,
            &initial_observation,
            A2aTaskObservationSourceV1::GetTask {
                request: evidence("get-task/request", 'a'),
                response: evidence("get-task/response", 'b'),
            },
            A2aRemoteTaskStateV1::Working,
            70,
        ),
        signature(),
    )
    .expect("post-expiry read-back observation");
    let authority = signing_authority();
    let binding_ref = record_ref("a2a/task-binding");
    let route_ref = record_ref("a2a/route");
    let handle_ref = record_ref("operation/handle");
    let context = A2aTaskObservationAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: task_observation.schema_sha256(),
            signing_authority: &authority,
        },
        task_binding_ref: &binding_ref,
        task_binding: &binding,
        route_admission_ref: &route_ref,
        route_admission: &route,
        operation_handle_ref: &handle_ref,
        operation_handle: &handle,
        operation_observation: &operation_observation,
        predecessor: Some((&predecessor_ref, &initial_observation)),
    };
    A2aTaskObservationV1::admit_contextual_wire(
        wire(&task_observation),
        &context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .expect("retained task read-back survives route expiry");
}

#[test]
fn task_observation_predecessor_cannot_cross_exact_binding_custody() {
    let session = session();
    let lease = operation_lease(&session);
    let handle = operation_handle(&lease);
    let operation_observation = dispatched(&handle);
    let route = admitted_route(false);
    let original_binding = A2aTaskBindingV1::seal(
        task_binding_preimage(&handle, &operation_observation, &route),
        signature(),
    )
    .expect("original binding");
    let original_observation = A2aTaskObservationV1::seal(
        initial_task_observation_preimage(&original_binding),
        signature(),
    )
    .expect("original observation");

    let mut substituted_preimage = task_binding_preimage(&handle, &operation_observation, &route);
    substituted_preimage.initial_task_evidence = evidence("send/substituted-response", '5');
    let substituted_binding =
        A2aTaskBindingV1::seal(substituted_preimage, signature()).expect("substituted binding");
    let predecessor_ref = record_ref("a2a/observation/original");
    let successor = A2aTaskObservationV1::seal(
        successor_task_observation_preimage(
            &substituted_binding,
            &predecessor_ref,
            &original_observation,
            A2aTaskObservationSourceV1::GetTask {
                request: evidence("get-task/request", 'a'),
                response: evidence("get-task/response", 'b'),
            },
            A2aRemoteTaskStateV1::Working,
            50,
        ),
        signature(),
    )
    .expect("successor shape");
    let authority = signing_authority();
    let binding_ref = record_ref("a2a/task-binding");
    let route_ref = record_ref("a2a/route");
    let handle_ref = record_ref("operation/handle");
    let context = A2aTaskObservationAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: successor.schema_sha256(),
            signing_authority: &authority,
        },
        task_binding_ref: &binding_ref,
        task_binding: &substituted_binding,
        route_admission_ref: &route_ref,
        route_admission: &route,
        operation_handle_ref: &handle_ref,
        operation_handle: &handle,
        operation_observation: &operation_observation,
        predecessor: Some((&predecessor_ref, &original_observation)),
    };

    assert!(A2aTaskObservationV1::admit_contextual_wire(
        wire(&successor),
        &context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn task_observation_predecessor_cannot_substitute_the_binding_reference() {
    let session = session();
    let lease = operation_lease(&session);
    let handle = operation_handle(&lease);
    let operation_observation = dispatched(&handle);
    let route = admitted_route(false);
    let binding = A2aTaskBindingV1::seal(
        task_binding_preimage(&handle, &operation_observation, &route),
        signature(),
    )
    .expect("binding");
    let mut substituted_predecessor_preimage = initial_task_observation_preimage(&binding);
    substituted_predecessor_preimage.task_binding_ref = record_ref("a2a/task-binding/substituted");
    let substituted_predecessor =
        A2aTaskObservationV1::seal(substituted_predecessor_preimage, signature())
            .expect("shape-valid substituted predecessor");

    let predecessor_ref = record_ref("a2a/observation/substituted");
    let successor = A2aTaskObservationV1::seal(
        successor_task_observation_preimage(
            &binding,
            &predecessor_ref,
            &substituted_predecessor,
            A2aTaskObservationSourceV1::GetTask {
                request: evidence("get-task/request", 'a'),
                response: evidence("get-task/response", 'b'),
            },
            A2aRemoteTaskStateV1::Working,
            50,
        ),
        signature(),
    )
    .expect("successor shape");
    let authority = signing_authority();
    let binding_ref = record_ref("a2a/task-binding");
    let route_ref = record_ref("a2a/route");
    let handle_ref = record_ref("operation/handle");
    let context = A2aTaskObservationAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: successor.schema_sha256(),
            signing_authority: &authority,
        },
        task_binding_ref: &binding_ref,
        task_binding: &binding,
        route_admission_ref: &route_ref,
        route_admission: &route,
        operation_handle_ref: &handle_ref,
        operation_handle: &handle,
        operation_observation: &operation_observation,
        predecessor: Some((&predecessor_ref, &substituted_predecessor)),
    };

    assert!(A2aTaskObservationV1::admit_contextual_wire(
        wire(&successor),
        &context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn task_observation_authority_rotation_requires_exact_lineage_admission() {
    let session = session();
    let lease = operation_lease(&session);
    let handle = operation_handle(&lease);
    let operation_observation = dispatched(&handle);
    let route = admitted_route(false);
    let binding = A2aTaskBindingV1::seal(
        task_binding_preimage(&handle, &operation_observation, &route),
        signature(),
    )
    .expect("binding");
    let initial =
        A2aTaskObservationV1::seal(initial_task_observation_preimage(&binding), signature())
            .expect("initial observation");
    let predecessor_ref = record_ref("a2a/observation/one");
    let mut successor_preimage = successor_task_observation_preimage(
        &binding,
        &predecessor_ref,
        &initial,
        A2aTaskObservationSourceV1::GetTask {
            request: evidence("get-task/request", 'a'),
            response: evidence("get-task/response", 'b'),
        },
        A2aRemoteTaskStateV1::Working,
        50,
    );
    successor_preimage.deployment_runtime_authority_ref = record_ref("runtime-authority/rotated");
    successor_preimage.deployment_runtime_authority_sha256 = artifact_digest('0');
    let successor = A2aTaskObservationV1::seal(successor_preimage, signature())
        .expect("rotated-authority successor");
    let authority = signing_authority_at("runtime-authority/rotated", '0');
    let binding_ref = record_ref("a2a/task-binding");
    let route_ref = record_ref("a2a/route");
    let handle_ref = record_ref("operation/handle");
    let context = A2aTaskObservationAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: successor.schema_sha256(),
            signing_authority: &authority,
        },
        task_binding_ref: &binding_ref,
        task_binding: &binding,
        route_admission_ref: &route_ref,
        route_admission: &route,
        operation_handle_ref: &handle_ref,
        operation_handle: &handle,
        operation_observation: &operation_observation,
        predecessor: Some((&predecessor_ref, &initial)),
    };
    let signature_checked = Cell::new(false);
    assert!(A2aTaskObservationV1::admit_contextual_wire(
        wire(&successor),
        &context,
        |_| Ok(()),
        |_, _, _, _| Err(contextual("A2A task observation", "same runtime lineage")),
        |_, _, _, _| {
            signature_checked.set(true);
            Ok(())
        },
    )
    .is_err());
    assert!(!signature_checked.get());

    let rotation_checked = Cell::new(false);
    A2aTaskObservationV1::admit_contextual_wire(
        wire(&successor),
        &context,
        |_| Ok(()),
        |predecessor_authority_ref,
         predecessor_authority_sha256,
         successor_authority_ref,
         successor_authority_sha256| {
            assert_eq!(
                predecessor_authority_ref,
                initial.deployment_runtime_authority_ref()
            );
            assert_eq!(
                predecessor_authority_sha256,
                initial.deployment_runtime_authority_sha256()
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
    .expect("admitted authority rotation preserves task-observation lineage");
    assert!(rotation_checked.get());
}

#[test]
fn streaming_observation_requires_advertisement_and_streaming_origin() {
    let session = session();
    let lease = operation_lease(&session);
    let handle = operation_handle(&lease);
    let operation_observation = dispatched(&handle);
    let route = admitted_route(false);
    let binding = A2aTaskBindingV1::seal(
        task_binding_preimage(&handle, &operation_observation, &route),
        signature(),
    )
    .expect("binding");
    let initial_observation =
        A2aTaskObservationV1::seal(initial_task_observation_preimage(&binding), signature())
            .expect("initial observation");
    let predecessor_ref = record_ref("a2a/observation/one");
    let task_observation = A2aTaskObservationV1::seal(
        successor_task_observation_preimage(
            &binding,
            &predecessor_ref,
            &initial_observation,
            A2aTaskObservationSourceV1::StreamingEvent {
                event_id: Some(A2aSseEventIdV1::parse("event-1").expect("event")),
                evidence: evidence("a2a/sse", 'b'),
            },
            A2aRemoteTaskStateV1::Working,
            50,
        ),
        signature(),
    )
    .expect("structural SSE observation");
    let authority = signing_authority();
    let binding_ref = record_ref("a2a/task-binding");
    let route_ref = record_ref("a2a/route");
    let handle_ref = record_ref("operation/handle");
    let context = A2aTaskObservationAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: task_observation.schema_sha256(),
            signing_authority: &authority,
        },
        task_binding_ref: &binding_ref,
        task_binding: &binding,
        route_admission_ref: &route_ref,
        route_admission: &route,
        operation_handle_ref: &handle_ref,
        operation_handle: &handle,
        operation_observation: &operation_observation,
        predecessor: Some((&predecessor_ref, &initial_observation)),
    };
    assert!(A2aTaskObservationV1::admit_contextual_wire(
        wire(&task_observation),
        &context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());

    let advertised_route = admitted_route(true);
    let unary_binding = A2aTaskBindingV1::seal(
        task_binding_preimage(&handle, &operation_observation, &advertised_route),
        signature(),
    )
    .expect("unary binding on advertised route");
    let unary_initial = A2aTaskObservationV1::seal(
        initial_task_observation_preimage(&unary_binding),
        signature(),
    )
    .expect("unary initial observation");
    let unary_successor = A2aTaskObservationV1::seal(
        successor_task_observation_preimage(
            &unary_binding,
            &predecessor_ref,
            &unary_initial,
            A2aTaskObservationSourceV1::StreamingEvent {
                event_id: Some(A2aSseEventIdV1::parse("event-2").expect("event")),
                evidence: evidence("a2a/sse/unary", 'c'),
            },
            A2aRemoteTaskStateV1::Working,
            50,
        ),
        signature(),
    )
    .expect("structural streaming successor");
    assert!(A2aTaskObservationV1::admit_contextual_wire(
        wire(&unary_successor),
        &A2aTaskObservationAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: unary_successor.schema_sha256(),
                signing_authority: &authority,
            },
            task_binding: &unary_binding,
            route_admission: &advertised_route,
            predecessor: Some((&predecessor_ref, &unary_initial)),
            ..context
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn cancel_task_read_back_requires_the_exact_cancellation_request() {
    let session = session();
    let lease = operation_lease(&session);
    let handle = operation_handle(&lease);
    let dispatched = dispatched(&handle);
    let route = admitted_route(false);
    let binding = A2aTaskBindingV1::seal(
        task_binding_preimage(&handle, &dispatched, &route),
        signature(),
    )
    .expect("binding");
    let initial_observation =
        A2aTaskObservationV1::seal(initial_task_observation_preimage(&binding), signature())
            .expect("initial observation");
    let predecessor_ref = record_ref("a2a/observation/one");
    let cancellation_request = evidence("cancel/request", 'c');
    let task_observation = A2aTaskObservationV1::seal(
        successor_task_observation_preimage(
            &binding,
            &predecessor_ref,
            &initial_observation,
            A2aTaskObservationSourceV1::CancelTaskReadBack {
                request: cancellation_request.clone(),
                response: evidence("cancel/response", 'd'),
            },
            A2aRemoteTaskStateV1::Canceled,
            50,
        ),
        signature(),
    )
    .expect("cancellation read-back observation");
    let authority = signing_authority();
    let binding_ref = record_ref("a2a/task-binding");
    let route_ref = record_ref("a2a/route");
    let handle_ref = record_ref("operation/handle");
    let active_context = A2aTaskObservationAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: task_observation.schema_sha256(),
            signing_authority: &authority,
        },
        task_binding_ref: &binding_ref,
        task_binding: &binding,
        route_admission_ref: &route_ref,
        route_admission: &route,
        operation_handle_ref: &handle_ref,
        operation_handle: &handle,
        operation_observation: &dispatched,
        predecessor: Some((&predecessor_ref, &initial_observation)),
    };
    assert!(A2aTaskObservationV1::admit_contextual_wire(
        wire(&task_observation),
        &active_context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());

    let cancellation = operation_observation(
        &handle,
        OperationObservationStateV1::CancellationRequested {
            request: cancellation_request.clone(),
        },
    );
    A2aTaskObservationV1::admit_contextual_wire(
        wire(&task_observation),
        &A2aTaskObservationAdmissionContextV1 {
            operation_observation: &cancellation,
            ..active_context
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .expect("exact cancellation request admits read-back evidence");

    let later_cancellation = operation_observation_at(
        &handle,
        OperationObservationStateV1::CancellationRequested {
            request: cancellation_request,
        },
        60,
    );
    assert!(A2aTaskObservationV1::admit_contextual_wire(
        wire(&task_observation),
        &A2aTaskObservationAdmissionContextV1 {
            operation_observation: &later_cancellation,
            ..active_context
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn initial_task_observation_is_exactly_bound_to_initial_response_and_parent_records() {
    let session = session();
    let lease = operation_lease(&session);
    let handle = operation_handle(&lease);
    let dispatched = dispatched(&handle);
    let route = admitted_route(false);
    let binding = A2aTaskBindingV1::seal(
        task_binding_preimage(&handle, &dispatched, &route),
        signature(),
    )
    .expect("binding");
    let observation =
        A2aTaskObservationV1::seal(initial_task_observation_preimage(&binding), signature())
            .expect("initial observation");
    let authority = signing_authority();
    let binding_ref = record_ref("a2a/task-binding");
    let route_ref = record_ref("a2a/route");
    let handle_ref = record_ref("operation/handle");
    let context = A2aTaskObservationAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: observation.schema_sha256(),
            signing_authority: &authority,
        },
        task_binding_ref: &binding_ref,
        task_binding: &binding,
        route_admission_ref: &route_ref,
        route_admission: &route,
        operation_handle_ref: &handle_ref,
        operation_handle: &handle,
        operation_observation: &dispatched,
        predecessor: None,
    };
    A2aTaskObservationV1::admit_contextual_wire(
        wire(&observation),
        &context,
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .expect("exact initial response admits");

    let mut wrong_evidence = initial_task_observation_preimage(&binding);
    wrong_evidence.source = A2aTaskObservationSourceV1::InitialSendMessageResponse {
        evidence: evidence("send/other-response", '5'),
    };
    let wrong_evidence =
        A2aTaskObservationV1::seal(wrong_evidence, signature()).expect("observation");
    assert!(A2aTaskObservationV1::admit_contextual_wire(
        wire(&wrong_evidence),
        &A2aTaskObservationAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: wrong_evidence.schema_sha256(),
                signing_authority: &authority,
            },
            ..context
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());

    let mut wrong_time = initial_task_observation_preimage(&binding);
    wrong_time.observed_at = UnixTimeV1::new(41).expect("observation time");
    let wrong_time = A2aTaskObservationV1::seal(wrong_time, signature()).expect("observation");
    assert!(A2aTaskObservationV1::admit_contextual_wire(
        wire(&wrong_time),
        &A2aTaskObservationAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: wrong_time.schema_sha256(),
                signing_authority: &authority,
            },
            ..context
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());

    let wrong_route_ref = record_ref("a2a/other-route");
    assert!(A2aTaskObservationV1::admit_contextual_wire(
        wire(&observation),
        &A2aTaskObservationAdmissionContextV1 {
            route_admission_ref: &wrong_route_ref,
            ..context
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());

    let wrong_handle_ref = record_ref("operation/other-handle");
    assert!(A2aTaskObservationV1::admit_contextual_wire(
        wire(&observation),
        &A2aTaskObservationAdmissionContextV1 {
            operation_handle_ref: &wrong_handle_ref,
            ..context
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());

    let mut different_route_preimage = route_preimage(false);
    different_route_preimage.endpoint =
        A2aHttpsEndpointV1::parse("https://other-agent.example.test/a2a").expect("other endpoint");
    let different_route = A2aRouteAdmissionV1::seal(different_route_preimage, signature())
        .expect("different route bytes");
    assert!(A2aTaskObservationV1::admit_contextual_wire(
        wire(&observation),
        &A2aTaskObservationAdmissionContextV1 {
            route_admission: &different_route,
            ..context
        },
        |_| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn remote_task_state_relation_is_monotone_and_terminally_absorbing() {
    use A2aRemoteTaskStateV1::{
        AuthRequired, Canceled, Completed, Failed, InputRequired, Rejected, Submitted, Unspecified,
        Working,
    };
    assert!(a2a_task_state_transition_is_monotone(Submitted, Working));
    assert!(a2a_task_state_transition_is_monotone(
        InputRequired,
        Working
    ));
    assert!(a2a_task_state_transition_is_monotone(
        AuthRequired,
        Completed
    ));
    assert!(!a2a_task_state_transition_is_monotone(Working, Submitted));
    assert!(!a2a_task_state_transition_is_monotone(Working, Unspecified));
    for terminal in [Completed, Canceled, Failed, Rejected] {
        assert!(terminal.is_terminal());
        assert!(a2a_task_state_transition_is_monotone(terminal, terminal));
        assert!(!a2a_task_state_transition_is_monotone(terminal, Working));
    }

    let states = [
        Unspecified,
        Submitted,
        Working,
        InputRequired,
        AuthRequired,
        Completed,
        Canceled,
        Failed,
        Rejected,
    ];
    for state in states {
        assert!(a2a_task_state_transition_is_monotone(state, state));
    }
    for left in states {
        for right in states {
            if a2a_task_state_transition_is_monotone(left, right)
                && a2a_task_state_transition_is_monotone(right, left)
            {
                assert_eq!(
                    A2aRemoteTaskPhaseV1::from(left),
                    A2aRemoteTaskPhaseV1::from(right)
                );
            }
            for last in states {
                if a2a_task_state_transition_is_monotone(left, right)
                    && a2a_task_state_transition_is_monotone(right, last)
                {
                    assert!(a2a_task_state_transition_is_monotone(left, last));
                }
            }
        }
    }
}
