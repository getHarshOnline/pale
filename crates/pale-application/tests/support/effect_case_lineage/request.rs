//! Shared finite action/request construction for contextual resident tests.

use pale_core::records::*;

pub(crate) fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

pub(crate) fn artifact(byte: char) -> ArtifactDigest {
    ArtifactDigest::parse(sha(byte)).expect("artifact digest")
}

pub(crate) fn record_ref(value: &str) -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse(value).expect("record reference")
}

pub(crate) fn signature() -> CanonicalSignatureV1 {
    CanonicalSignatureV1::parse("A".repeat(86)).expect("signature")
}

pub(crate) fn signing_authority() -> SigningAuthorityBindingV1 {
    SigningAuthorityBindingV1::integrated_runtime(
        ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        ResidentKeyIdV1::parse("key-1").expect("key"),
        SigningKeyEpochV1::new(1).expect("key epoch"),
        IntegratedRuntimeAuthorityV1 {
            deployment_runtime_authority_ref: record_ref("runtime-authority"),
            deployment_runtime_authority_sha256: artifact('a'),
            deployment_binding_ref: record_ref("deployment"),
            deployment_binding_sha256: artifact('b'),
            checked_principal_trust_checkpoint_ref: record_ref("trust-checkpoint"),
            checked_principal_trust_checkpoint_sha256: artifact('c'),
        },
    )
    .expect("signing authority")
}

pub(crate) fn action(label: &str) -> ActionEnvelopeV1 {
    let payload = serde_json::json!({"status": label});
    let canonical = pale_core::canon::to_canonical_string(&payload).expect("canonical payload");
    ActionEnvelopeV1::admit(
        ActionEnvelopePreimageV1 {
            action_id: ActionIdV1::parse(format!("action-{label}")).expect("action id"),
            target_system: ActionTargetSystemV1::parse("crm").expect("target system"),
            target_identity: ActionTargetIdentityV1::parse(format!("contact-{label}"))
                .expect("target identity"),
            conflict_resource_key_schema_id: SchemaId::parse("example/resource-key.v1")
                .expect("resource schema"),
            conflict_resource_key_schema_sha256: SchemaDigest::parse(sha('e'))
                .expect("resource schema digest"),
            conflict_resource_key: serde_json::json!({"contact": label}),
            operation: ActionOperationV1::parse("update").expect("operation"),
            business_idempotency_key: BusinessIdempotencyKeyV1::parse(format!("business-{label}"))
                .expect("business idempotency"),
            expected_precondition_digest: None,
            payload_schema_id: SchemaId::parse("example/action-payload.v1")
                .expect("payload schema"),
            payload_schema_sha256: SchemaDigest::parse(sha('f')).expect("payload schema digest"),
            payload_hash: ArtifactDigest::hash_bytes(canonical.as_bytes()),
            inline_payload: Some(payload),
            immutable_payload_ref: None,
            effect_policy: record_ref("effect-policy"),
            reconciliation_policy: record_ref("reconciliation-policy"),
        },
        |_| None,
    )
    .expect("admitted action")
}

pub(crate) fn request_preimage(
    label: &str,
    action: &ActionEnvelopeV1,
) -> CommonRequestEnvelopePreimageV1 {
    let payload = serde_json::to_value(action).expect("action serialization");
    let action_digest = action
        .canonical_action_body_sha256()
        .expect("action digest");
    CommonRequestEnvelopePreimageV1 {
        envelope_schema_id: SchemaId::parse(COMMON_REQUEST_ENVELOPE_V1_SCHEMA)
            .expect("request schema"),
        envelope_schema_sha256: SchemaDigest::parse(sha('0')).expect("request schema digest"),
        request_id: ResidentRequestIdV1::parse(format!("request-{label}")).expect("request id"),
        correlation_id: ResidentCorrelationIdV1::parse(format!("correlation-{label}"))
            .expect("correlation id"),
        workspace_id: ResidentWorkspaceIdV1::parse("workspace-1").expect("workspace"),
        capability_id: CapabilityIdV1::parse("write").expect("capability"),
        operation: PackageResourceRefV1 {
            package_id: PackageIdV1::parse("test-package").expect("Package id"),
            resource_id: PackageResourceIdV1::parse("write-operation").expect("operation id"),
        },
        caller: RequestCallerV1 {
            identity: RequestCallerIdV1::parse("caller-1").expect("caller"),
            role: RequestCallerRoleV1::parse("operator").expect("role"),
            surface: RequestCallerSurfaceV1::parse("native").expect("surface"),
            signer_key_id: ResidentKeyIdV1::parse("key-1").expect("key"),
            signer_key_epoch: SigningKeyEpochV1::new(1).expect("key epoch"),
        },
        signing_authority_binding: signing_authority(),
        route: CommonRequestRouteV1 {
            origin_principal_id: ResidentPrincipalIdV1::parse("principal-1")
                .expect("origin principal"),
            origin_node_id: ResidentNodeIdV1::parse("node-1").expect("origin node"),
            destination_principal_id: ResidentPrincipalIdV1::parse("principal-1")
                .expect("destination principal"),
            destination_node_id: None,
            route_id: ResidentRouteIdV1::parse("route-1").expect("route"),
            route_generation: RouteGenerationV1::new(1).expect("route generation"),
            transport: BoundedResidentTextV1::parse("local").expect("transport"),
            route_binding_schema_id: SchemaId::parse("example/route.v1").expect("route schema"),
            route_binding_schema_sha256: SchemaDigest::parse(sha('1'))
                .expect("route schema digest"),
            route_binding_ref: record_ref("route-binding"),
            route_binding_sha256: artifact('2'),
            contract_set_ref: record_ref("contract-set"),
            contract_set_sha256: artifact('3'),
        },
        session_admission: CommonRequestSessionAdmissionV1 {
            session_admission_id: SessionAdmissionIdV1::parse("session-1")
                .expect("session admission id"),
            session_admission_lease_digest: SessionAdmissionLeaseDigest::parse(sha('7'))
                .expect("session admission lease digest"),
            session_grant_id: SessionGrantIdV1::parse("grant-1").expect("grant"),
            session_grant_sha256: artifact('4'),
            deployment_binding_ref: record_ref("deployment"),
            deployment_binding_sha256: artifact('b'),
            session_floor_epoch: PrincipalSessionFloorEpochV1::new(1).expect("session floor"),
            grant_issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
            grant_issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
            grant_issuer_key_epoch: PrincipalKeyEpochV1::new(1).expect("principal key epoch"),
            checked_revocation_epoch: PrincipalRevocationEpochV1::new(1).expect("revocation epoch"),
            checked_revocation_ref: Some(record_ref("revocation")),
            checked_revocation_sha256: Some(artifact('5')),
            checked_revocation_checkpoint_ref: record_ref("revocation-checkpoint"),
            checked_revocation_checkpoint_sha256: artifact('6'),
            checked_at: UnixTimeV1::new(9).expect("checked time"),
        },
        operation_target: None,
        execution_owner_fence: None,
        federation_claim: None,
        scope: CommonRequestScopeV1 {
            principal_scope: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
            controller_scope: ResidentControllerIdV1::parse("controller-1").expect("controller"),
            relationship_scope: None,
            environment: BoundedResidentTextV1::parse("local").expect("environment"),
            data_classification: BoundedResidentTextV1::parse("internal").expect("classification"),
            purpose: BoundedResidentTextV1::parse("effect").expect("purpose"),
            authority_evidence: Vec::new(),
        },
        risk_class: BoundedResidentTextV1::parse("bounded").expect("risk"),
        resource_constraints: Vec::new(),
        expected_precondition: None,
        idempotency_key: RequestIdempotencyKeyV1::parse(format!("idempotency-{label}"))
            .expect("idempotency"),
        issued_at: UnixTimeV1::new(10).expect("issue time"),
        expires_at: UnixTimeV1::new(100).expect("expiry"),
        generation: None,
        fence_token: None,
        body: CommonRequestBodyV1 {
            schema_id: SchemaId::parse(ACTION_ENVELOPE_V1_SCHEMA).expect("action schema"),
            schema_sha256: SchemaDigest::parse(sha('9')).expect("action schema digest"),
            payload_sha256: ArtifactDigest::parse(action_digest.as_str()).expect("action digest"),
            payload,
        },
    }
}

pub(crate) fn request(preimage: &CommonRequestEnvelopePreimageV1) -> CommonRequestEnvelopeV1 {
    let digest = CommonRequestEnvelopeDigest::hash_canonical(preimage).expect("request digest");
    let mut value = serde_json::to_value(preimage).expect("request preimage");
    let object = value.as_object_mut().expect("request object");
    object.insert(
        "canonical_request_sha256".to_owned(),
        serde_json::to_value(digest).expect("request digest serialization"),
    );
    object.insert(
        "caller_signature".to_owned(),
        serde_json::to_value(signature()).expect("caller signature"),
    );
    let wire = serde_json::from_value(value).expect("request wire");
    let context = CommonRequestAdmissionContextV1 {
        expected_schema_sha256: &preimage.envelope_schema_sha256,
        expected_body_schema_id: &preimage.body.schema_id,
        expected_body_schema_sha256: &preimage.body.schema_sha256,
        caller: &preimage.caller,
        signing_authority: &preimage.signing_authority_binding,
        route: &preimage.route,
        session_admission: &preimage.session_admission,
        operation_target: preimage.operation_target.as_ref(),
        execution_owner_fence: None,
        federation_claim: None,
        workspace_id: &preimage.workspace_id,
        capability_id: &preimage.capability_id,
        operation: &preimage.operation,
        scope: &preimage.scope,
        risk_class: &preimage.risk_class,
        resource_constraints: &preimage.resource_constraints,
        expected_precondition: None,
        idempotency_key: &preimage.idempotency_key,
        issued_at: preimage.issued_at,
        expires_at: preimage.expires_at,
        generation: preimage.generation.as_ref(),
        fence_token: preimage.fence_token.as_ref(),
    };
    CommonRequestEnvelopeV1::admit_contextual_wire(wire, &context, |_, _, _| Ok(()))
        .expect("admitted request")
}
