//! Exact signed requests used by the real conformance resident.

use pale_core::records::*;

use crate::resident::{AdmittedAmbientSession, AmbientSessionAdmissionContext};

use super::{super::conformance_ingress_time, principal::signature};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in super::super) enum ConformanceRequestFreshness {
    Current,
    Expired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in super::super) enum ConformanceRequestPhase {
    Advance,
    Observe,
}

impl ConformanceRequestPhase {
    pub(in super::super) const fn purpose(self) -> &'static str {
        match self {
            Self::Advance => "operation-continuation",
            Self::Observe => "operation-observation",
        }
    }
}

fn schema(id: &str) -> SchemaDigest {
    pale_contracts::record_schema_identity(id)
        .expect("registered request schema")
        .schema_sha256
}

fn contract_schema(id: &str) -> SchemaDigest {
    SchemaDigest::parse(
        pale_contracts::schema_descriptor(id)
            .expect("registered contract schema")
            .schema_sha256,
    )
    .expect("contract schema digest")
}

pub(super) fn seal(preimage: &CommonRequestEnvelopePreimageV1) -> CommonRequestEnvelopeV1 {
    let digest =
        CommonRequestEnvelopeDigest::hash_canonical(preimage).expect("conformance request digest");
    let mut value = serde_json::to_value(preimage).expect("request preimage serializes");
    let object = value
        .as_object_mut()
        .expect("request preimage is an object");
    object.insert(
        "canonical_request_sha256".into(),
        serde_json::to_value(digest).expect("request digest serializes"),
    );
    object.insert(
        "caller_signature".into(),
        serde_json::to_value(signature()).expect("request signature serializes"),
    );
    let wire = serde_json::from_value(value).expect("request wire decodes");
    let context = context(preimage);
    CommonRequestEnvelopeV1::admit_contextual_wire(wire, &context, |_, _, _| Ok(()))
        .expect("conformance request admits")
}

fn context(value: &CommonRequestEnvelopePreimageV1) -> CommonRequestAdmissionContextV1<'_> {
    CommonRequestAdmissionContextV1 {
        expected_schema_sha256: &value.envelope_schema_sha256,
        expected_body_schema_id: &value.body.schema_id,
        expected_body_schema_sha256: &value.body.schema_sha256,
        caller: &value.caller,
        signing_authority: &value.signing_authority_binding,
        route: &value.route,
        session_admission: &value.session_admission,
        operation_target: value.operation_target.as_ref(),
        execution_owner_fence: None,
        federation_claim: None,
        workspace_id: &value.workspace_id,
        capability_id: &value.capability_id,
        operation: &value.operation,
        scope: &value.scope,
        risk_class: &value.risk_class,
        resource_constraints: &value.resource_constraints,
        expected_precondition: None,
        idempotency_key: &value.idempotency_key,
        issued_at: value.issued_at,
        expires_at: value.expires_at,
        generation: value.generation.as_ref(),
        fence_token: value.fence_token.as_ref(),
    }
}

pub(in super::super) fn start_request(
    session: &AdmittedAmbientSession,
    current: &AmbientSessionAdmissionContext<'_>,
    lease: &OperationLeaseV1,
) -> CommonRequestEnvelopeV1 {
    let payload = serde_json::json!({
        "kind": "path_inventory",
        "payload": {
            "style": "posix_relative",
            "paths": ["src/./lib.rs", "src/lib.rs"]
        }
    });
    let payload_jcs =
        pale_core::canon::to_canonical_string(&payload).expect("action input canonicalizes");
    let grant = current.grant.as_ref().expect("conformance grant").grant;
    let capability = current
        .capability_revocations
        .as_ref()
        .expect("conformance capability frontier");
    seal(&CommonRequestEnvelopePreimageV1 {
        envelope_schema_id: SchemaId::parse(COMMON_REQUEST_ENVELOPE_V1_SCHEMA)
            .expect("request schema"),
        envelope_schema_sha256: schema(COMMON_REQUEST_ENVELOPE_V1_SCHEMA_ID),
        request_id: ResidentRequestIdV1::parse("request-interface-conformance-start")
            .expect("request identity"),
        correlation_id: ResidentCorrelationIdV1::parse("correlation-interface-conformance")
            .expect("correlation identity"),
        workspace_id: ResidentWorkspaceIdV1::parse(
            current.generation.effective_profile.workspace_id.as_str(),
        )
        .expect("workspace identity"),
        capability_id: CapabilityIdV1::parse("read").expect("capability"),
        operation: lease.operation().clone(),
        caller: RequestCallerV1 {
            identity: current.subject.identity.clone(),
            role: current.subject.role.clone(),
            surface: current.subject.surface.clone(),
            signer_key_id: current.subject.subject_key_id.clone(),
            signer_key_epoch: current.subject.subject_key_epoch,
        },
        signing_authority_binding: current.generation.signed.signing_authority.clone(),
        route: CommonRequestRouteV1 {
            origin_principal_id: session.lease().principal_id().clone(),
            origin_node_id: current.route_use.origin_node_id.clone(),
            destination_principal_id: current.route_use.destination_principal_id.clone(),
            destination_node_id: current.route_use.destination_node_id.cloned(),
            route_id: current.route_use.route.route_id.clone(),
            route_generation: current.route_use.route.route_generation,
            transport: BoundedResidentTextV1::parse("in-process").expect("transport"),
            route_binding_schema_id: current.route_use.route.route_binding_schema_id.clone(),
            route_binding_schema_sha256: schema(LOCAL_ROUTE_BINDING_V1_SCHEMA_ID),
            route_binding_ref: current.route_use.route.route_binding_ref.clone(),
            route_binding_sha256: current.route_use.route.route_binding_sha256.clone(),
            contract_set_ref: current.route_use.contract_set.contract_set_ref.clone(),
            contract_set_sha256: ArtifactDigest::parse(
                current.route_use.contract_set.contract_set_sha256.as_str(),
            )
            .expect("contract-set digest"),
        },
        session_admission: CommonRequestSessionAdmissionV1 {
            session_admission_id: session.lease().session_admission_id().clone(),
            session_admission_lease_digest: session
                .lease()
                .session_admission_lease_digest()
                .clone(),
            session_grant_id: session.grant_id().clone(),
            session_grant_sha256: ArtifactDigest::parse(session.grant_sha256().as_str())
                .expect("grant digest"),
            deployment_binding_ref: session.lease().deployment_binding_ref().clone(),
            deployment_binding_sha256: session.lease().deployment_binding_sha256().clone(),
            session_floor_epoch: *grant.session_floor_epoch(),
            grant_issuer_principal_id: grant.issuer_principal_id().clone(),
            grant_issuer_key_id: grant.issuer_key_id().clone(),
            grant_issuer_key_epoch: PrincipalKeyEpochV1::new(grant.issuer_key_epoch().get())
                .expect("grant key epoch"),
            checked_revocation_epoch: current.session_revocations.epoch,
            checked_revocation_ref: current.session_revocations.revocation_ref.cloned(),
            checked_revocation_sha256: current.session_revocations.revocation_sha256.cloned(),
            checked_revocation_checkpoint_ref: capability.checkpoint_ref.clone(),
            checked_revocation_checkpoint_sha256: ArtifactDigest::parse(
                capability.checkpoint.canonical_checkpoint_sha256().as_str(),
            )
            .expect("capability checkpoint digest"),
            checked_at: conformance_ingress_time(),
        },
        operation_target: None,
        execution_owner_fence: None,
        federation_claim: None,
        scope: CommonRequestScopeV1 {
            principal_scope: session.lease().principal_id().clone(),
            controller_scope: session.lease().controller_id().clone(),
            relationship_scope: session.lease().relationship_scope().clone(),
            environment: BoundedResidentTextV1::parse("development").expect("environment"),
            data_classification: BoundedResidentTextV1::parse("internal").expect("classification"),
            purpose: BoundedResidentTextV1::parse("operations").expect("purpose"),
            authority_evidence: Vec::new(),
        },
        risk_class: BoundedResidentTextV1::parse("bounded").expect("risk class"),
        resource_constraints: Vec::new(),
        expected_precondition: None,
        idempotency_key: RequestIdempotencyKeyV1::parse("idempotency-interface-conformance-start")
            .expect("idempotency identity"),
        issued_at: current.now,
        expires_at: UnixTimeV1::new(current.now.get() + 30).expect("request expiry"),
        generation: Some(session.lease().compiled_generation_digest().clone()),
        fence_token: None,
        body: CommonRequestBodyV1 {
            schema_id: SchemaId::parse("action-input.v1").expect("action input schema"),
            schema_sha256: contract_schema("action-input.v1"),
            payload_sha256: ArtifactDigest::hash_bytes(payload_jcs.as_bytes()),
            payload,
        },
    })
}

fn phase_request_preimage(
    start: &CommonRequestEnvelopeV1,
    phase: ConformanceRequestPhase,
    locator: Option<&pale_ports::ResidentOperationFrameLocator>,
    suffix: &str,
    freshness: ConformanceRequestFreshness,
) -> CommonRequestEnvelopePreimageV1 {
    let mut value = start.to_preimage();
    value.request_id =
        ResidentRequestIdV1::parse(format!("request-{suffix}")).expect("target request identity");
    value.idempotency_key = RequestIdempotencyKeyV1::parse(format!("idempotency-{suffix}"))
        .expect("target idempotency identity");
    value.scope.purpose = BoundedResidentTextV1::parse(phase.purpose()).expect("request purpose");
    value.operation_target = locator.map(|locator| OperationTargetReferenceV1 {
        operation_handle_ref: ResidentRecordRefV1::parse(
            locator.operation_handle_record_ref().to_owned(),
        )
        .expect("operation handle reference"),
        operation_handle_sha256: OperationHandleDigest::parse(
            locator.operation_handle_sha256().to_owned(),
        )
        .expect("operation handle digest"),
    });
    if freshness == ConformanceRequestFreshness::Expired {
        let start_issued_at = value.issued_at.get();
        value.issued_at =
            UnixTimeV1::new(start_issued_at.saturating_sub(1)).expect("expired request issue time");
        value.expires_at = UnixTimeV1::new(start_issued_at).expect("expired request deadline");
    }
    value
}

pub(in super::super) fn phase_request(
    start: &CommonRequestEnvelopeV1,
    phase: ConformanceRequestPhase,
    locator: Option<&pale_ports::ResidentOperationFrameLocator>,
    suffix: &str,
    freshness: ConformanceRequestFreshness,
) -> CommonRequestEnvelopeV1 {
    seal(&phase_request_preimage(
        start, phase, locator, suffix, freshness,
    ))
}

pub(in super::super) fn observe_request(
    start: &CommonRequestEnvelopeV1,
    operation_handle: serde_json::Value,
    activation_fence: &ActivationFenceDigest,
) -> CommonRequestEnvelopeV1 {
    let mut value = phase_request_preimage(
        start,
        ConformanceRequestPhase::Observe,
        None,
        "interface-conformance-observe",
        ConformanceRequestFreshness::Current,
    );
    let payload_jcs = pale_core::canon::to_canonical_string(&operation_handle)
        .expect("operation handle canonicalizes");
    value.capability_id = CapabilityIdV1::parse(crate::resident::OPERATION_OBSERVE_CAPABILITY_ID)
        .expect("Observe capability");
    value.fence_token = Some(activation_fence.clone());
    value.issued_at = conformance_ingress_time();
    value.expires_at =
        UnixTimeV1::new(value.issued_at.get() + 30).expect("bounded Observe request expiry");
    value.body = CommonRequestBodyV1 {
        schema_id: SchemaId::parse(OPERATION_HANDLE_V1_SCHEMA).expect("operation-handle schema"),
        schema_sha256: schema(OPERATION_HANDLE_V1_SCHEMA_ID),
        payload_sha256: ArtifactDigest::hash_bytes(payload_jcs.as_bytes()),
        payload: operation_handle,
    };
    seal(&value)
}

pub(in super::super) fn admit_historical_request(
    _: &ResidentRecordRefV1,
    wire: CommonRequestEnvelopeWireV1,
) -> Result<CommonRequestEnvelopeV1, InvalidResidentRecordV1> {
    let preimage = wire.untrusted_preimage();
    let context = context(&preimage);
    CommonRequestEnvelopeV1::admit_contextual_wire(wire, &context, |_, _, candidate| {
        (candidate == &super::principal::signature())
            .then_some(())
            .ok_or(InvalidResidentRecordV1::ContextualAdmission {
                record: "interface conformance request",
                invariant: "deterministic caller signature",
            })
    })
}
