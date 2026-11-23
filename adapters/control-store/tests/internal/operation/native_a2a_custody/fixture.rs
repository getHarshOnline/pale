//! Contextual A2A record inputs for the SQLite-backed custody fixture.
//! Signature, Agent Card and exchange authorization checks are explicit fixture
//! models. These constructors neither send a request nor prove remote execution.

use pale_core::records::*;

pub(crate) fn reference(value: &str) -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse(value).unwrap()
}

pub(crate) fn schema(value: &str) -> SchemaDigest {
    pale_contracts::record_schema_identity(value)
        .unwrap()
        .schema_sha256
}

pub(crate) fn signature() -> CanonicalSignatureV1 {
    CanonicalSignatureV1::parse("A".repeat(86)).unwrap()
}

pub(crate) fn evidence(reference_value: &str, bytes: &[u8]) -> ImmutableRecordReferenceV1 {
    ImmutableRecordReferenceV1 {
        record_ref: reference(reference_value),
        record_sha256: ArtifactDigest::hash_bytes(bytes),
    }
}

pub(crate) fn route(
    handle: &OperationHandleV1,
    authority: &SigningAuthorityBindingV1,
    provider: ProviderBindingRefV1,
    endpoint: &str,
    streaming: A2aStreamingAdmissionV1,
) -> A2aRouteAdmissionV1 {
    let binding = handle.request_binding();
    let route = CommonRequestRouteV1 {
        origin_principal_id: handle.issuer_principal_id().clone(),
        origin_node_id: ResidentNodeIdV1::parse("node-1").unwrap(),
        destination_principal_id: ResidentPrincipalIdV1::parse("remote-principal").unwrap(),
        destination_node_id: Some(ResidentNodeIdV1::parse("remote-node").unwrap()),
        route_id: ResidentRouteIdV1::parse("task-custody-route").unwrap(),
        route_generation: *binding.route_generation(),
        transport: BoundedResidentTextV1::parse("a2a-json-rpc2-https").unwrap(),
        route_binding_schema_id: binding.route_binding_schema_id().clone(),
        route_binding_schema_sha256: binding.route_binding_schema_sha256().clone(),
        route_binding_ref: binding.route_binding_ref().clone(),
        route_binding_sha256: binding.route_binding_sha256().clone(),
        contract_set_ref: binding.contract_set_ref().clone(),
        contract_set_sha256: binding.contract_set_sha256().clone(),
    };
    let schema = schema(A2A_ROUTE_ADMISSION_V1_SCHEMA_ID);
    A2aRouteAdmissionV1::issue_contextual(
        A2aRouteAdmissionPreimageV1 {
            schema_id: SchemaId::parse(A2A_ROUTE_ADMISSION_V1_SCHEMA).unwrap(),
            schema_sha256: schema.clone(),
            route_admission_id: A2aRouteAdmissionIdV1::parse("task-custody-route").unwrap(),
            provider_binding: provider.clone(),
            route: route.clone(),
            endpoint: A2aHttpsEndpointV1::parse(endpoint).unwrap(),
            protocol_version: A2aProtocolVersionV1::V1_0,
            request_binding: A2aRequestBindingV1::JsonRpc2OverVerifiedHttps,
            agent_card: evidence("a2a/card", b"fixture Agent Card admission"),
            streaming,
            security: A2aSecurityAdmissionV1 {
                selected_scheme_ids: vec![],
                evidence: evidence("a2a/security", b"fixture selected HTTPS trust"),
            },
            observed_at: UnixTimeV1::new(10).unwrap(),
            fresh_until: UnixTimeV1::new(70).unwrap(),
            expires_at: UnixTimeV1::new(75).unwrap(),
            deployment_runtime_authority_ref: handle.deployment_runtime_authority_ref().clone(),
            deployment_runtime_authority_sha256: handle
                .deployment_runtime_authority_sha256()
                .clone(),
            issuer_principal_id: handle.issuer_principal_id().clone(),
            issuer_key_id: handle.issuer_key_id().clone(),
            issuer_key_epoch: *handle.issuer_key_epoch(),
        },
        &A2aRouteAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &schema,
                signing_authority: authority,
            },
            provider_binding: &provider,
            route: &route,
        },
        |_| Ok(()), // Explicit Agent Card/security authority model, not discovery.
        |_, _, _| Ok(signature()),
    )
    .unwrap()
}

pub(crate) struct Inputs {
    pub(crate) route_ref: ResidentRecordRefV1,
    pub(crate) binding_ref: ResidentRecordRefV1,
    pub(crate) binding: A2aTaskBindingPreimageV1,
    pub(crate) observation_ref: ResidentRecordRefV1,
    pub(crate) observation: A2aTaskObservationPreimageV1,
}

impl Inputs {
    pub(crate) fn new(
        handle: (&ResidentRecordRefV1, &OperationHandleV1),
        dispatch: (&ResidentRecordRefV1, &OperationObservationV1),
        route: (&ResidentRecordRefV1, &A2aRouteAdmissionV1),
        request: ImmutableRecordReferenceV1,
        response: ImmutableRecordReferenceV1,
        bound_at: u64,
    ) -> Self {
        let route_ref = route.0.clone();
        let route = route.1;
        let binding_ref = reference("a2a/binding");
        let binding = A2aTaskBindingPreimageV1 {
            schema_id: SchemaId::parse(A2A_TASK_BINDING_V1_SCHEMA).unwrap(),
            schema_sha256: schema(A2A_TASK_BINDING_V1_SCHEMA_ID),
            task_binding_id: A2aTaskBindingIdV1::parse("task-custody-binding").unwrap(),
            operation_handle_id: handle.1.operation_handle_id().clone(),
            operation_handle_ref: handle.0.clone(),
            operation_handle_sha256: handle.1.canonical_operation_handle_sha256().clone(),
            operation_observation_ref: dispatch.0.clone(),
            operation_observation_sha256: dispatch
                .1
                .canonical_operation_observation_sha256()
                .clone(),
            route_admission_id: route.route_admission_id().clone(),
            route_admission_ref: route_ref.clone(),
            route_admission_sha256: route.canonical_a2a_route_admission_sha256().clone(),
            attempt_id: dispatch.1.state().dispatched_attempt_id().unwrap().clone(),
            json_rpc_request_id: A2aJsonRpcRequestIdV1::parse("rpc-1").unwrap(),
            request_mode: A2aTaskRequestModeV1::SendMessage,
            remote_task_id: A2aRemoteTaskIdV1::parse("task-1").unwrap(),
            remote_context_id: A2aRemoteContextIdV1::parse("context-1").unwrap(),
            request_evidence: request,
            initial_task_evidence: response.clone(),
            bound_at: UnixTimeV1::new(bound_at).unwrap(),
            deployment_runtime_authority_ref: handle.1.deployment_runtime_authority_ref().clone(),
            deployment_runtime_authority_sha256: handle
                .1
                .deployment_runtime_authority_sha256()
                .clone(),
            issuer_principal_id: handle.1.issuer_principal_id().clone(),
            issuer_key_id: handle.1.issuer_key_id().clone(),
            issuer_key_epoch: *handle.1.issuer_key_epoch(),
        };
        let observation = A2aTaskObservationPreimageV1 {
            schema_id: SchemaId::parse(A2A_TASK_OBSERVATION_V1_SCHEMA).unwrap(),
            schema_sha256: schema(A2A_TASK_OBSERVATION_V1_SCHEMA_ID),
            task_binding_id: binding.task_binding_id.clone(),
            task_binding_ref: binding_ref.clone(),
            task_binding_sha256: A2aTaskBindingDigest::hash_canonical(&binding).unwrap(),
            revision: A2aTaskObservationRevisionV1::new(1).unwrap(),
            predecessor_ref: None,
            predecessor_sha256: None,
            source: A2aTaskObservationSourceV1::InitialSendMessageResponse { evidence: response },
            remote_state: A2aRemoteTaskStateV1::Working,
            reconciliation: None,
            observed_at: binding.bound_at,
            deployment_runtime_authority_ref: binding.deployment_runtime_authority_ref.clone(),
            deployment_runtime_authority_sha256: binding
                .deployment_runtime_authority_sha256
                .clone(),
            issuer_principal_id: binding.issuer_principal_id.clone(),
            issuer_key_id: binding.issuer_key_id.clone(),
            issuer_key_epoch: binding.issuer_key_epoch,
        };
        Self {
            route_ref,
            binding_ref,
            binding,
            observation_ref: reference("a2a/observation-1"),
            observation,
        }
    }

    pub(crate) fn binding_context<'a>(
        &'a self,
        handle: (&'a ResidentRecordRefV1, &'a OperationHandleV1),
        dispatch: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
        route: &'a A2aRouteAdmissionV1,
        authority: &'a SigningAuthorityBindingV1,
    ) -> A2aTaskBindingAdmissionContextV1<'a> {
        A2aTaskBindingAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &self.binding.schema_sha256,
                signing_authority: authority,
            },
            operation_handle_ref: handle.0,
            operation_handle: handle.1,
            operation_observation_ref: dispatch.0,
            operation_observation: dispatch.1,
            route_admission_ref: &self.route_ref,
            route_admission: route,
        }
    }
}
