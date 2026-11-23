//! Exercises contextual admission where effect records depend on separately admitted owner evidence.

use super::*;
use serde::de::DeserializeOwned;

fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn artifact(byte: char) -> ArtifactDigest {
    ArtifactDigest::parse(sha(byte)).expect("artifact digest")
}

fn execution_fence_digest(byte: char) -> ExecutionOwnerFenceDigest {
    ExecutionOwnerFenceDigest::parse(sha(byte)).expect("execution-fence digest")
}

fn record_ref(value: &str) -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse(value).expect("resident record ref")
}

fn signature() -> CanonicalSignatureV1 {
    CanonicalSignatureV1::parse("A".repeat(86)).expect("signature")
}

fn signing_authority_at(
    runtime_authority_ref: &str,
    runtime_authority_sha256: char,
    deployment_binding_ref: &str,
    deployment_binding_sha256: char,
) -> SigningAuthorityBindingV1 {
    SigningAuthorityBindingV1::integrated_runtime(
        ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        ResidentKeyIdV1::parse("key-1").expect("key"),
        SigningKeyEpochV1::new(1).expect("valid fixture signing key epoch"),
        IntegratedRuntimeAuthorityV1 {
            deployment_runtime_authority_ref: record_ref(runtime_authority_ref),
            deployment_runtime_authority_sha256: artifact(runtime_authority_sha256),
            deployment_binding_ref: record_ref(deployment_binding_ref),
            deployment_binding_sha256: artifact(deployment_binding_sha256),
            checked_principal_trust_checkpoint_ref: record_ref("trust-checkpoint"),
            checked_principal_trust_checkpoint_sha256: artifact('c'),
        },
    )
    .expect("signing authority")
}

fn signing_authority() -> SigningAuthorityBindingV1 {
    signing_authority_at("runtime-authority", 'a', "deployment", 'b')
}

fn typed_state(state: &str) -> TypedDigestV1 {
    let value = serde_json::json!({"state": state});
    let canonical = crate::canon::to_canonical_string(&value).expect("canonical state");
    TypedDigestV1::admit(
        TypedDigestPreimageV1 {
            kind: BoundedResidentTextV1::parse("provider-request").expect("digest kind"),
            content_schema_id: SchemaId::parse("example/provider-request.v1")
                .expect("content schema"),
            content_schema_sha256: SchemaDigest::parse(sha('d')).expect("schema digest"),
            canonicalization: TypedDigestCanonicalizationV1::Rfc8785Json,
            digest_algorithm: TypedDigestAlgorithmV1::Sha256,
            digest: ArtifactDigest::hash_bytes(canonical.as_bytes()),
            content_binding: TypedDigestContentBindingV1::InlineJson { inline_json: value },
        },
        |_| None,
    )
    .expect("typed state")
}

fn action() -> ActionEnvelopeV1 {
    let payload = serde_json::json!({"status": "ready"});
    let canonical = crate::canon::to_canonical_string(&payload).expect("canonical action payload");
    ActionEnvelopeV1::admit(
        ActionEnvelopePreimageV1 {
            action_id: ActionIdV1::parse("action-1").expect("action id"),
            target_system: ActionTargetSystemV1::parse("crm").expect("target system"),
            target_identity: ActionTargetIdentityV1::parse("contact-1").expect("target identity"),
            conflict_resource_key_schema_id: SchemaId::parse("example/resource-key.v1")
                .expect("key schema"),
            conflict_resource_key_schema_sha256: SchemaDigest::parse(sha('e'))
                .expect("key schema digest"),
            conflict_resource_key: serde_json::json!({"contact":"contact-1"}),
            operation: ActionOperationV1::parse("update").expect("operation"),
            business_idempotency_key: BusinessIdempotencyKeyV1::parse("business-1")
                .expect("business key"),
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
    .expect("action")
}

fn execution_owner_fence_wire(
    preimage: &ExecutionOwnerFencePreimageV1,
) -> ExecutionOwnerFenceWireV1 {
    let canonical_execution_fence_sha256 =
        ExecutionOwnerFenceDigest::hash_canonical(preimage).expect("canonical fence digest");
    let mut wire = serde_json::to_value(preimage).expect("execution fence serializes");
    let object = wire.as_object_mut().expect("execution fence object");
    object.insert(
        "canonical_execution_fence_sha256".to_owned(),
        serde_json::to_value(canonical_execution_fence_sha256).expect("fence digest serializes"),
    );
    object.insert(
        "signature".to_owned(),
        serde_json::to_value(signature()).expect("signature serializes"),
    );
    serde_json::from_value(wire).expect("execution fence wire")
}

fn execution_owner_fence_with_expiry(
    expires_at: Option<UnixTimeV1>,
) -> (ResidentRecordRefV1, ExecutionOwnerFenceV1) {
    let preimage = ExecutionOwnerFencePreimageV1 {
        schema_id: SchemaId::parse(EXECUTION_OWNER_FENCE_V1_SCHEMA)
            .expect("execution-fence schema"),
        schema_sha256: SchemaDigest::parse(sha('7')).expect("execution-fence schema digest"),
        execution_fence_id: ExecutionOwnerFenceIdV1::parse("execution-fence-1")
            .expect("execution-fence id"),
        execution_attempt_id: ExecutionAttemptIdV1::parse("execution-1")
            .expect("execution attempt"),
        execution_path_id: ExecutionPathSelectionIdV1::parse("execution-path-1")
            .expect("execution path"),
        execution_owner_system_id: ExecutionOwnerSystemIdV1::parse("owner-system-1")
            .expect("execution owner system"),
        execution_owner_participant_id: ExecutionOwnerParticipantIdV1::parse("owner-participant-1")
            .expect("execution owner participant"),
        owner_authorization_schema_id: SchemaId::parse("owner/execution-authorization.v1")
            .expect("owner authorization schema"),
        owner_authorization_schema_sha256: SchemaDigest::parse(sha('8'))
            .expect("owner authorization schema digest"),
        owner_authorization_ref: record_ref("execution-authorization"),
        owner_authorization_sha256: artifact('9'),
        deployment_binding_ref: record_ref("deployment"),
        deployment_binding_sha256: DeploymentBindingDigest::parse(sha('b'))
            .expect("deployment binding digest"),
        principal_scope: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        controller_scope: ResidentControllerIdV1::parse("controller-1").expect("controller"),
        relationship_scope: None,
        owner_generation: ExecutionOwnerGenerationV1::new(1)
            .expect("valid fixture owner generation"),
        control_epoch: ExecutionControlEpochV1::new(1).expect("valid fixture control epoch"),
        previous_fence_ref: None,
        previous_fence_sha256: None,
        previous_execution_attempt_id: None,
        previous_owner_generation: None,
        previous_control_epoch: None,
        predecessor_disposition_schema_id: None,
        predecessor_disposition_schema_sha256: None,
        predecessor_disposition_ref: None,
        predecessor_disposition_sha256: None,
        state: ExecutionOwnerFenceStateV1::Active,
        issued_at: UnixTimeV1::new(8).expect("valid fixture time"),
        expires_at,
        deployment_runtime_authority_ref: record_ref("runtime-authority"),
        deployment_runtime_authority_sha256: artifact('a'),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("valid fixture signing key epoch"),
    };
    let wire = execution_owner_fence_wire(&preimage);
    let authority = signing_authority();
    let context = SignedResidentAdmissionContextV1 {
        expected_schema_sha256: &preimage.schema_sha256,
        signing_authority: &authority,
    };
    let fence = ExecutionOwnerFenceV1::admit_initial_wire(
        wire,
        &context,
        |candidate| {
            assert_eq!(
                candidate.owner_authorization_ref,
                preimage.owner_authorization_ref
            );
            Ok(())
        },
        |domain, candidate, digest, candidate_signature| {
            assert_eq!(domain, EXECUTION_OWNER_FENCE_V1_SIGNATURE_DOMAIN);
            assert_eq!(candidate, &preimage);
            assert_eq!(
                digest,
                &ExecutionOwnerFenceDigest::hash_canonical(&preimage)
                    .expect("canonical fence digest")
            );
            assert_eq!(candidate_signature, &signature());
            Ok(())
        },
    )
    .expect("admitted active execution-owner fence");
    (record_ref("execution-fence"), fence)
}

fn execution_owner_fence() -> (ResidentRecordRefV1, ExecutionOwnerFenceV1) {
    execution_owner_fence_with_expiry(Some(
        UnixTimeV1::new(100).expect("valid fixture fence expiry"),
    ))
}

#[test]
fn execution_owner_fence_rejects_runtime_authority_substitution() {
    let (_, fence) = execution_owner_fence();
    let mut encoded = serde_json::to_value(&fence).expect("fence serializes");
    encoded["deployment_runtime_authority_ref"] = serde_json::json!("other-authority");
    let mut preimage = encoded.clone();
    let object = preimage.as_object_mut().expect("fence object");
    object.remove("canonical_execution_fence_sha256");
    object.remove("signature");
    encoded["canonical_execution_fence_sha256"] = serde_json::to_value(
        ExecutionOwnerFenceDigest::hash_canonical(&preimage).expect("changed fence digest"),
    )
    .expect("fence digest serializes");
    let wire = serde_json::from_value(encoded).expect("changed fence remains structural");
    let authority = signing_authority();
    let context = SignedResidentAdmissionContextV1 {
        expected_schema_sha256: fence.schema_sha256(),
        signing_authority: &authority,
    };

    assert!(matches!(
        ExecutionOwnerFenceV1::admit_initial_wire(wire, &context, |_| Ok(()), |_, _, _, _| Ok(()),),
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "execution owner fence",
            invariant: "exact deployment runtime authority",
        })
    ));
}

fn retired_execution_owner_fence(
    predecessor_ref: &ResidentRecordRefV1,
    predecessor: &ExecutionOwnerFenceV1,
) -> (ResidentRecordRefV1, ExecutionOwnerFenceV1) {
    let preimage = ExecutionOwnerFencePreimageV1 {
        schema_id: predecessor.schema_id().clone(),
        schema_sha256: predecessor.schema_sha256().clone(),
        execution_fence_id: predecessor.execution_fence_id().clone(),
        execution_attempt_id: ExecutionAttemptIdV1::parse("execution-2")
            .expect("successor attempt"),
        execution_path_id: predecessor.execution_path_id().clone(),
        execution_owner_system_id: predecessor.execution_owner_system_id().clone(),
        execution_owner_participant_id: predecessor.execution_owner_participant_id().clone(),
        owner_authorization_schema_id: predecessor.owner_authorization_schema_id().clone(),
        owner_authorization_schema_sha256: SchemaDigest::parse(sha('a'))
            .expect("successor authorization schema digest"),
        owner_authorization_ref: record_ref("execution-authorization-2"),
        owner_authorization_sha256: artifact('c'),
        deployment_binding_ref: predecessor.deployment_binding_ref().clone(),
        deployment_binding_sha256: predecessor.deployment_binding_sha256().clone(),
        principal_scope: predecessor.principal_scope().clone(),
        controller_scope: predecessor.controller_scope().clone(),
        relationship_scope: predecessor.relationship_scope().clone(),
        owner_generation: *predecessor.owner_generation(),
        control_epoch: predecessor
            .control_epoch()
            .checked_successor()
            .expect("successor control epoch"),
        previous_fence_ref: Some(predecessor_ref.clone()),
        previous_fence_sha256: Some(predecessor.canonical_execution_fence_sha256().clone()),
        previous_execution_attempt_id: Some(predecessor.execution_attempt_id().clone()),
        previous_owner_generation: Some(*predecessor.owner_generation()),
        previous_control_epoch: Some(*predecessor.control_epoch()),
        predecessor_disposition_schema_id: Some(
            SchemaId::parse("owner/execution-disposition.v1").expect("disposition schema"),
        ),
        predecessor_disposition_schema_sha256: Some(
            SchemaDigest::parse(sha('d')).expect("disposition schema digest"),
        ),
        predecessor_disposition_ref: Some(record_ref("execution-disposition")),
        predecessor_disposition_sha256: Some(artifact('e')),
        state: ExecutionOwnerFenceStateV1::Retired,
        issued_at: UnixTimeV1::new(20).expect("valid fixture time"),
        expires_at: None,
        deployment_runtime_authority_ref: record_ref("runtime-authority"),
        deployment_runtime_authority_sha256: artifact('a'),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("valid fixture signing key epoch"),
    };
    let wire = execution_owner_fence_wire(&preimage);
    let authority = signing_authority();
    let context = SignedResidentAdmissionContextV1 {
        expected_schema_sha256: &preimage.schema_sha256,
        signing_authority: &authority,
    };
    let fence = ExecutionOwnerFenceV1::admit_successor_wire(
        wire,
        predecessor_ref,
        predecessor,
        &context,
        |candidate| {
            assert!(candidate.predecessor_disposition_ref.is_some());
            Ok(())
        },
        |_, _, _, _| Ok(()),
        |domain, candidate, digest, candidate_signature| {
            assert_eq!(domain, EXECUTION_OWNER_FENCE_V1_SIGNATURE_DOMAIN);
            assert_eq!(candidate, &preimage);
            assert_eq!(
                digest,
                &ExecutionOwnerFenceDigest::hash_canonical(&preimage)
                    .expect("canonical successor digest")
            );
            assert_eq!(candidate_signature, &signature());
            Ok(())
        },
    )
    .expect("admitted retired execution-owner fence");
    (record_ref("execution-fence-2"), fence)
}

#[test]
fn execution_owner_fence_rotation_requires_admitted_deployment_continuity() {
    let (predecessor_ref, predecessor) = execution_owner_fence();
    let (_, successor) = retired_execution_owner_fence(&predecessor_ref, &predecessor);
    let mut encoded = serde_json::to_value(&successor).expect("successor serializes");
    encoded["deployment_runtime_authority_ref"] = serde_json::json!("runtime-authority-rotated");
    encoded["deployment_runtime_authority_sha256"] = serde_json::json!(sha('0'));
    let mut preimage = encoded.clone();
    let object = preimage.as_object_mut().expect("successor object");
    object.remove("canonical_execution_fence_sha256");
    object.remove("signature");
    encoded["canonical_execution_fence_sha256"] = serde_json::to_value(
        ExecutionOwnerFenceDigest::hash_canonical(&preimage).expect("rotated fence digest"),
    )
    .expect("rotated fence digest serializes");
    let wire = || {
        serde_json::from_value::<ExecutionOwnerFenceWireV1>(encoded.clone())
            .expect("rotated fence wire")
    };

    let foreign_authority =
        signing_authority_at("runtime-authority-rotated", '0', "foreign-deployment", 'f');
    let foreign_context = SignedResidentAdmissionContextV1 {
        expected_schema_sha256: successor.schema_sha256(),
        signing_authority: &foreign_authority,
    };
    let signature_checked = std::cell::Cell::new(false);
    assert!(ExecutionOwnerFenceV1::admit_successor_wire(
        wire(),
        &predecessor_ref,
        &predecessor,
        &foreign_context,
        |_| Ok(()),
        |_, _, _, _| Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "execution owner fence",
            invariant: "same deployment runtime-authority lineage",
        }),
        |_, _, _, _| {
            signature_checked.set(true);
            Ok(())
        },
    )
    .is_err());
    assert!(!signature_checked.get());

    let same_deployment_authority =
        signing_authority_at("runtime-authority-rotated", '0', "deployment", 'b');
    let same_deployment_context = SignedResidentAdmissionContextV1 {
        expected_schema_sha256: successor.schema_sha256(),
        signing_authority: &same_deployment_authority,
    };
    let rotation_checked = std::cell::Cell::new(false);
    ExecutionOwnerFenceV1::admit_successor_wire(
        wire(),
        &predecessor_ref,
        &predecessor,
        &same_deployment_context,
        |_| Ok(()),
        |predecessor_ref, predecessor_sha256, successor_ref, successor_sha256| {
            assert_eq!(
                predecessor_ref,
                predecessor.deployment_runtime_authority_ref()
            );
            assert_eq!(
                predecessor_sha256,
                predecessor.deployment_runtime_authority_sha256()
            );
            assert_eq!(successor_ref.as_str(), "runtime-authority-rotated");
            assert_eq!(successor_sha256, &artifact('0'));
            rotation_checked.set(true);
            Ok(())
        },
        |_, _, _, _| Ok(()),
    )
    .expect("admitted same-deployment rotation");
    assert!(rotation_checked.get());
}

struct CommonRequestFixture {
    preimage: CommonRequestEnvelopePreimageV1,
    execution_fence_ref: ResidentRecordRefV1,
    execution_fence: ExecutionOwnerFenceV1,
}

fn action_request_fixture(action: &ActionEnvelopeV1) -> CommonRequestFixture {
    let payload = serde_json::to_value(action).expect("action serializes");
    let action_digest = action
        .canonical_action_body_sha256()
        .expect("action digest");
    let (execution_fence_ref, execution_fence) = execution_owner_fence();
    let preimage = CommonRequestEnvelopePreimageV1 {
        envelope_schema_id: SchemaId::parse(COMMON_REQUEST_ENVELOPE_V1_SCHEMA)
            .expect("request schema"),
        envelope_schema_sha256: SchemaDigest::parse(sha('0')).expect("request schema digest"),
        request_id: ResidentRequestIdV1::parse("request-1").expect("request id"),
        correlation_id: ResidentCorrelationIdV1::parse("correlation-1").expect("correlation id"),
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
            signer_key_epoch: SigningKeyEpochV1::new(1).expect("valid fixture signing key epoch"),
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
            route_generation: RouteGenerationV1::new(1).expect("valid fixture route generation"),
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
            session_floor_epoch: PrincipalSessionFloorEpochV1::new(1)
                .expect("valid fixture session floor"),
            grant_issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
            grant_issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
            grant_issuer_key_epoch: PrincipalKeyEpochV1::new(1)
                .expect("valid fixture principal key epoch"),
            checked_revocation_epoch: PrincipalRevocationEpochV1::new(1)
                .expect("valid fixture revocation epoch"),
            checked_revocation_ref: Some(record_ref("revocation")),
            checked_revocation_sha256: Some(artifact('5')),
            checked_revocation_checkpoint_ref: record_ref("revocation-checkpoint"),
            checked_revocation_checkpoint_sha256: artifact('6'),
            checked_at: UnixTimeV1::new(9).expect("valid fixture time"),
        },
        operation_target: None,
        execution_owner_fence: Some(CommonRequestExecutionOwnerFenceV1::from_admitted(
            &execution_fence_ref,
            &execution_fence,
        )),
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
        idempotency_key: RequestIdempotencyKeyV1::parse("idempotency-1").expect("idempotency"),
        issued_at: UnixTimeV1::new(10).expect("valid fixture time"),
        expires_at: UnixTimeV1::new(100).expect("valid fixture time"),
        generation: None,
        fence_token: None,
        body: CommonRequestBodyV1 {
            schema_id: SchemaId::parse(ACTION_ENVELOPE_V1_SCHEMA).expect("action schema"),
            schema_sha256: SchemaDigest::parse(sha('9')).expect("action schema digest"),
            payload_sha256: ArtifactDigest::parse(action_digest.as_str())
                .expect("action payload digest"),
            payload,
        },
    };
    CommonRequestFixture {
        preimage,
        execution_fence_ref,
        execution_fence,
    }
}

fn assert_explicit_null_rejected<T: DeserializeOwned>(
    mut value: serde_json::Value,
    member: &'static str,
) {
    value[member] = serde_json::Value::Null;
    assert!(
        serde_json::from_value::<T>(value).is_err(),
        "explicit null admitted for {member}"
    );
}

#[test]
fn common_request_nested_optionals_omit_absence_and_reject_explicit_null() {
    let mut request = action_request_fixture(&action()).preimage;
    request.route.destination_node_id = None;
    request.session_admission.checked_revocation_ref = None;
    request.session_admission.checked_revocation_sha256 = None;
    request.scope.relationship_scope = None;

    let route = serde_json::to_value(&request.route).expect("route serializes");
    let session =
        serde_json::to_value(&request.session_admission).expect("session admission serializes");
    let scope = serde_json::to_value(&request.scope).expect("scope serializes");
    assert!(route.get("destination_node_id").is_none());
    assert!(session.get("checked_revocation_ref").is_none());
    assert!(session.get("checked_revocation_sha256").is_none());
    assert!(scope.get("relationship_scope").is_none());

    assert_explicit_null_rejected::<CommonRequestRouteV1>(route, "destination_node_id");
    assert_explicit_null_rejected::<CommonRequestSessionAdmissionV1>(
        session.clone(),
        "checked_revocation_ref",
    );
    assert_explicit_null_rejected::<CommonRequestSessionAdmissionV1>(
        session,
        "checked_revocation_sha256",
    );
    assert_explicit_null_rejected::<CommonRequestScopeV1>(scope, "relationship_scope");
}

#[test]
fn common_request_outer_optionals_match_the_wire_contract_when_absent_or_present() {
    const OPTIONALS: [&str; 6] = [
        "operation_target",
        "execution_owner_fence",
        "federation_claim",
        "expected_precondition",
        "generation",
        "fence_token",
    ];
    let mut absent = action_request_fixture(&action());
    absent.preimage.execution_owner_fence = None;
    let sealed = CommonRequestEnvelopeV1::seal(absent.preimage.clone(), signature())
        .expect("request with absent optionals seals");
    let preimage_value = serde_json::to_value(&absent.preimage).expect("preimage serializes");
    let envelope_value = serde_json::to_value(&sealed).expect("request serializes");
    let wire: CommonRequestEnvelopeWireV1 =
        serde_json::from_value(envelope_value.clone()).expect("request wire deserializes");
    assert_eq!(wire.untrusted_preimage(), absent.preimage);
    let wire_value = serde_json::to_value(&wire).expect("request wire serializes");
    for (shape, value) in [
        ("preimage", &preimage_value),
        ("envelope", &envelope_value),
        ("wire", &wire_value),
    ] {
        for member in OPTIONALS {
            assert!(
                value.get(member).is_none(),
                "absent optional {member} is omitted from the {shape} rather than serialized as null"
            );
        }
    }
    let schema = serde_json::to_value(schemars::schema_for!(CommonRequestEnvelopeWireV1))
        .expect("wire schema serializes");
    let required = schema["required"]
        .as_array()
        .expect("common-request schema has a required set");
    for member in OPTIONALS {
        assert!(
            !required.iter().any(|value| value.as_str() == Some(member)),
            "wire schema keeps {member} optional"
        );
    }
    CommonRequestEnvelopeV1::admit_contextual_wire(
        wire,
        &request_context(&absent.preimage, None),
        |_, _, _| Ok(()),
    )
    .expect("omitted optionals admit against an absent context");

    let mut present = action_request_fixture(&action());
    present.preimage.operation_target = Some(OperationTargetReferenceV1 {
        operation_handle_ref: record_ref("operation-handle"),
        operation_handle_sha256: OperationHandleDigest::parse(sha('d'))
            .expect("operation handle digest"),
    });
    present.preimage.federation_claim = Some(CommonRequestFederationClaimV1 {
        space_id: FederationSpaceIdV1::parse("space-1").expect("space"),
        space_governance_epoch: FederationGovernanceEpochV1::new(1).expect("governance epoch"),
        membership_id: FederationMembershipIdV1::parse("membership-1").expect("membership"),
        membership_state_epoch: FederationMembershipEpochV1::new(1).expect("membership epoch"),
        membership_sha256: artifact('a'),
        grant_id: FederationGrantIdV1::parse("federation-grant-1").expect("grant"),
        grant_sha256: artifact('b'),
        revocation_proofs: Vec::new(),
    });
    present.preimage.expected_precondition = Some(ExpectedPreconditionV1 {
        state_schema_id: SchemaId::parse("example/state.v1").expect("state schema"),
        state_schema_sha256: SchemaDigest::parse(sha('e')).expect("state schema digest"),
        expected_state_sha256: artifact('f'),
    });
    present.preimage.generation =
        Some(CompiledGenerationDigest::parse(sha('e')).expect("generation"));
    present.preimage.fence_token =
        Some(ActivationFenceDigest::parse(sha('f')).expect("activation fence"));
    let sealed = CommonRequestEnvelopeV1::seal(present.preimage.clone(), signature())
        .expect("request with present optionals seals");
    let preimage_value = serde_json::to_value(&present.preimage).expect("preimage serializes");
    let envelope_value = serde_json::to_value(&sealed).expect("request serializes");
    let wire: CommonRequestEnvelopeWireV1 =
        serde_json::from_value(envelope_value.clone()).expect("request wire deserializes");
    let wire_value = serde_json::to_value(&wire).expect("request wire serializes");
    for (shape, value) in [
        ("preimage", &preimage_value),
        ("envelope", &envelope_value),
        ("wire", &wire_value),
    ] {
        for member in OPTIONALS {
            assert!(
                value.get(member).is_some_and(|value| !value.is_null()),
                "present optional {member} remains explicit in the {shape}"
            );
        }
    }
    CommonRequestEnvelopeV1::admit_contextual_wire(
        wire,
        &request_context(
            &present.preimage,
            Some((&present.execution_fence_ref, &present.execution_fence)),
        ),
        |_, _, _| Ok(()),
    )
    .expect("present optionals admit against their exact context");
}

fn request_context<'a>(
    preimage: &'a CommonRequestEnvelopePreimageV1,
    execution_fence: Option<(&'a ResidentRecordRefV1, &'a ExecutionOwnerFenceV1)>,
) -> CommonRequestAdmissionContextV1<'a> {
    CommonRequestAdmissionContextV1 {
        expected_schema_sha256: &preimage.envelope_schema_sha256,
        expected_body_schema_id: &preimage.body.schema_id,
        expected_body_schema_sha256: &preimage.body.schema_sha256,
        caller: &preimage.caller,
        signing_authority: &preimage.signing_authority_binding,
        route: &preimage.route,
        session_admission: &preimage.session_admission,
        operation_target: preimage.operation_target.as_ref(),
        execution_owner_fence: execution_fence.map(|(execution_fence_ref, execution_fence)| {
            CommonRequestExecutionOwnerFenceAdmissionV1::new(execution_fence_ref, execution_fence)
        }),
        federation_claim: preimage.federation_claim.as_ref(),
        workspace_id: &preimage.workspace_id,
        capability_id: &preimage.capability_id,
        operation: &preimage.operation,
        scope: &preimage.scope,
        risk_class: &preimage.risk_class,
        resource_constraints: &preimage.resource_constraints,
        expected_precondition: preimage.expected_precondition.as_ref(),
        idempotency_key: &preimage.idempotency_key,
        issued_at: preimage.issued_at,
        expires_at: preimage.expires_at,
        generation: preimage.generation.as_ref(),
        fence_token: preimage.fence_token.as_ref(),
    }
}

fn common_request_wire(request: &CommonRequestEnvelopeV1) -> CommonRequestEnvelopeWireV1 {
    serde_json::from_value(serde_json::to_value(request).expect("request serializes"))
        .expect("request wire")
}

#[test]
fn local_request_binding_preserves_the_complete_admitted_request_and_optional_fence() {
    let mut fixture = action_request_fixture(&action());
    for include_fence in [true, false] {
        if !include_fence {
            fixture.preimage.execution_owner_fence = None;
        }
        let sealed = CommonRequestEnvelopeV1::seal(fixture.preimage.clone(), signature())
            .expect("request seals");
        let context = request_context(
            &fixture.preimage,
            include_fence.then_some((&fixture.execution_fence_ref, &fixture.execution_fence)),
        );
        let admitted = CommonRequestEnvelopeV1::admit_contextual_wire(
            common_request_wire(&sealed),
            &context,
            |_, _, _| Ok(()),
        )
        .expect("request context admits");
        let binding = admitted.local_request_binding().expect("local binding");
        // The existing independent binding fixture supplies every binding axis.
        // Remove its synthetic federation evidence: local requests admit none.
        let mut expected = serde_json::to_value(request_binding(&admitted, &fixture.preimage))
            .expect("expected binding");
        let expected = expected.as_object_mut().expect("binding object");
        expected.remove("federation_request_admission_ref");
        expected.remove("federation_request_admission_sha256");
        assert_eq!(
            serde_json::to_value(&binding).expect("binding serializes"),
            serde_json::Value::Object(expected.clone())
        );
        assert_eq!(
            binding.canonical_request_sha256(),
            admitted.canonical_request_sha256()
        );
    }
}

#[test]
fn local_request_binding_never_promotes_federation_claims_or_cross_principal_routes() {
    for with_claim in [false, true] {
        let mut fixture = action_request_fixture(&action());
        if with_claim {
            fixture.preimage.federation_claim = Some(CommonRequestFederationClaimV1 {
                space_id: FederationSpaceIdV1::parse("space-1").expect("space"),
                space_governance_epoch: FederationGovernanceEpochV1::new(1).expect("epoch"),
                membership_id: FederationMembershipIdV1::parse("membership-1").expect("membership"),
                membership_state_epoch: FederationMembershipEpochV1::new(1).expect("epoch"),
                membership_sha256: artifact('1'),
                grant_id: FederationGrantIdV1::parse("grant-1").expect("grant"),
                grant_sha256: artifact('2'),
                revocation_proofs: Vec::new(),
            });
        } else {
            fixture.preimage.route.destination_principal_id =
                ResidentPrincipalIdV1::parse("principal-2").expect("other principal");
        }
        let sealed = CommonRequestEnvelopeV1::seal(fixture.preimage.clone(), signature())
            .expect("structurally valid supplied request");
        let context = request_context(
            &fixture.preimage,
            Some((&fixture.execution_fence_ref, &fixture.execution_fence)),
        );
        let admitted = CommonRequestEnvelopeV1::admit_contextual_wire(
            common_request_wire(&sealed),
            &context,
            |_, _, _| Ok(()),
        )
        .expect("request admission preserves supplied coordinates, not federation authority");
        assert!(matches!(
            admitted.local_request_binding(),
            Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "local request binding",
                ..
            })
        ));
    }
}

fn request_binding(
    request: &CommonRequestEnvelopeV1,
    preimage: &CommonRequestEnvelopePreimageV1,
) -> RequestBindingV1 {
    RequestBindingV1::new(RequestBindingPreimageV1 {
        request_id: preimage.request_id.clone(),
        correlation_id: preimage.correlation_id.clone(),
        capability_id: preimage.capability_id.clone(),
        canonical_request_sha256: request.canonical_request_sha256().clone(),
        session_grant_id: preimage.session_admission.session_grant_id.clone(),
        session_grant_sha256: preimage.session_admission.session_grant_sha256.clone(),
        deployment_binding_ref: preimage.session_admission.deployment_binding_ref.clone(),
        deployment_binding_sha256: preimage.session_admission.deployment_binding_sha256.clone(),
        route_binding_schema_id: preimage.route.route_binding_schema_id.clone(),
        route_binding_schema_sha256: preimage.route.route_binding_schema_sha256.clone(),
        route_binding_ref: preimage.route.route_binding_ref.clone(),
        route_binding_sha256: preimage.route.route_binding_sha256.clone(),
        route_generation: preimage.route.route_generation,
        contract_set_ref: preimage.route.contract_set_ref.clone(),
        contract_set_sha256: preimage.route.contract_set_sha256.clone(),
        execution_owner_fence_ref: preimage
            .execution_owner_fence
            .as_ref()
            .map(|fence| fence.execution_fence_ref.clone()),
        execution_owner_fence_sha256: preimage
            .execution_owner_fence
            .as_ref()
            .map(|fence| fence.execution_fence_sha256.clone()),
        federation_request_admission_ref: Some(record_ref("federation-admission")),
        federation_request_admission_sha256: Some(artifact('a')),
    })
    .expect("request binding")
}

struct EffectLineage {
    request_preimage: CommonRequestEnvelopePreimageV1,
    request: CommonRequestEnvelopeV1,
    binding: RequestBindingV1,
    action: ActionEnvelopeV1,
    claim_ref: ResidentRecordRefV1,
    claim_preimage: EffectConflictClaimPreimageV1,
    claim: EffectConflictClaimV1,
    admission_ref: ResidentRecordRefV1,
    admission_preimage: EffectAdmissionPreimageV1,
    admission: EffectAdmissionV1,
    dispatch_ref: ResidentRecordRefV1,
    dispatch_preimage: EffectDispatchIntentPreimageV1,
    dispatch: EffectDispatchIntentV1,
    receipt_schema_sha256: SchemaDigest,
}

fn effect_lineage() -> EffectLineage {
    let action = action();
    let request_preimage = action_request_fixture(&action).preimage;
    let request = CommonRequestEnvelopeV1::seal(request_preimage.clone(), signature())
        .expect("common request");
    let binding = request_binding(&request, &request_preimage);
    let action_digest = action
        .canonical_action_body_sha256()
        .expect("action digest");
    let claim_ref = record_ref("claim");
    let claim_preimage = EffectConflictClaimPreimageV1 {
        schema_id: SchemaId::parse(EFFECT_CONFLICT_CLAIM_V1_SCHEMA).expect("claim schema"),
        schema_sha256: SchemaDigest::parse(sha('b')).expect("claim schema digest"),
        conflict_claim_id: EffectConflictClaimIdV1::parse("claim-1").expect("claim id"),
        claim_generation: EffectConflictClaimGenerationV1::new(1)
            .expect("valid fixture claim generation"),
        previous_claim_ref: None,
        previous_claim_sha256: None,
        conflict_resource_key_schema_id: action.conflict_resource_key_schema_id().clone(),
        conflict_resource_key_schema_sha256: action.conflict_resource_key_schema_sha256().clone(),
        conflict_resource_key: action.conflict_resource_key().clone(),
        principal_scope: request_preimage.scope.principal_scope.clone(),
        controller_scope: request_preimage.scope.controller_scope.clone(),
        relationship_scope: request_preimage.scope.relationship_scope.clone(),
        request_id: request_preimage.request_id.clone(),
        canonical_request_sha256: request.canonical_request_sha256().clone(),
        action_id: action.action_id().clone(),
        action_body_sha256: action_digest.clone(),
        business_idempotency_key: action.business_idempotency_key().clone(),
        claim_state: EffectConflictClaimStateV1::Active,
        holder_principal_id: request_preimage.scope.principal_scope.clone(),
        terminal_reason: None,
        effect_receipt_ref: None,
        effect_receipt_sha256: None,
        cancellation_authority_ref: None,
        cancellation_authority_sha256: None,
        reconciliation_custody_ref: None,
        reconciliation_custody_sha256: None,
        issued_at: UnixTimeV1::new(10).expect("valid fixture time"),
        transferred_or_released_at: None,
        deployment_runtime_authority_ref: record_ref("runtime-authority"),
        deployment_runtime_authority_sha256: artifact('a'),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("valid fixture signing key epoch"),
    };
    let claim =
        EffectConflictClaimV1::seal(claim_preimage.clone(), signature()).expect("active claim");
    let admission_ref = record_ref("admission");
    let admission_preimage = EffectAdmissionPreimageV1 {
        schema_id: SchemaId::parse(EFFECT_ADMISSION_V1_SCHEMA).expect("admission schema"),
        schema_sha256: SchemaDigest::parse(sha('c')).expect("admission schema digest"),
        effect_admission_id: EffectAdmissionIdV1::parse("admission-1").expect("admission id"),
        request_id: request_preimage.request_id.clone(),
        canonical_request_sha256: request.canonical_request_sha256().clone(),
        action_id: action.action_id().clone(),
        action_body_sha256: action_digest.clone(),
        conflict_resource_key_schema_id: action.conflict_resource_key_schema_id().clone(),
        conflict_resource_key_schema_sha256: action.conflict_resource_key_schema_sha256().clone(),
        conflict_resource_key: action.conflict_resource_key().clone(),
        conflict_claim_ref: claim_ref.clone(),
        conflict_claim_sha256: claim.canonical_effect_conflict_claim_sha256().clone(),
        authority_evidence: request_preimage.scope.authority_evidence.clone(),
        risk_class: request_preimage.risk_class.clone(),
        effect_policy: action.effect_policy().clone(),
        reconciliation_policy: action.reconciliation_policy().clone(),
        decision: EffectAdmissionDecisionV1::Admitted,
        issued_at: UnixTimeV1::new(11).expect("valid fixture time"),
        expires_at: UnixTimeV1::new(90).expect("valid fixture time"),
        deployment_runtime_authority_ref: record_ref("runtime-authority"),
        deployment_runtime_authority_sha256: artifact('a'),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("valid fixture signing key epoch"),
    };
    let admission =
        EffectAdmissionV1::seal(admission_preimage.clone(), signature()).expect("effect admission");
    let dispatch_ref = record_ref("dispatch");
    let execution_fence = request_preimage
        .execution_owner_fence
        .as_ref()
        .expect("execution fence");
    let dispatch_preimage = EffectDispatchIntentPreimageV1 {
        schema_id: SchemaId::parse(EFFECT_DISPATCH_INTENT_V1_SCHEMA).expect("dispatch schema"),
        schema_sha256: SchemaDigest::parse(sha('d')).expect("dispatch schema digest"),
        effect_dispatch_intent_id: EffectDispatchIntentIdV1::parse("dispatch-1")
            .expect("dispatch id"),
        attempt_id: CapabilityAttemptIdV1::parse("attempt-1").expect("attempt"),
        request_ref: record_ref("request"),
        canonical_request_sha256: request.canonical_request_sha256().clone(),
        action_id: action.action_id().clone(),
        action_body_sha256: action_digest,
        effect_admission_ref: admission_ref.clone(),
        effect_admission_sha256: admission.canonical_effect_admission_sha256().clone(),
        conflict_claim_ref: claim_ref.clone(),
        conflict_claim_sha256: claim.canonical_effect_conflict_claim_sha256().clone(),
        contract_set_ref: request_preimage.route.contract_set_ref.clone(),
        contract_set_sha256: request_preimage.route.contract_set_sha256.clone(),
        route_binding_schema_id: request_preimage.route.route_binding_schema_id.clone(),
        route_binding_schema_sha256: request_preimage.route.route_binding_schema_sha256.clone(),
        route_binding_ref: request_preimage.route.route_binding_ref.clone(),
        route_binding_sha256: request_preimage.route.route_binding_sha256.clone(),
        route_generation: request_preimage.route.route_generation,
        execution_owner_fence_ref: Some(execution_fence.execution_fence_ref.clone()),
        execution_owner_fence_sha256: Some(execution_fence.execution_fence_sha256.clone()),
        provider_request_digest: Some(typed_state("provider-request")),
        reserved_reconciliation_case_id: ReconciliationCaseIdV1::parse("case-1").expect("case id"),
        reserved_reconciliation_custody_id: ReconciliationCustodyIdV1::parse("custody-1")
            .expect("custody id"),
        implementation_kind: BoundedResidentTextV1::parse("native").expect("implementation"),
        implementation_version: BoundedResidentTextV1::parse("1").expect("version"),
        intended_at: UnixTimeV1::new(12).expect("valid fixture time"),
        deployment_runtime_authority_ref: record_ref("runtime-authority"),
        deployment_runtime_authority_sha256: artifact('a'),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("valid fixture signing key epoch"),
    };
    let dispatch = EffectDispatchIntentV1::seal(dispatch_preimage.clone(), signature())
        .expect("dispatch intent");
    EffectLineage {
        request_preimage,
        request,
        binding,
        action,
        claim_ref,
        claim_preimage,
        claim,
        admission_ref,
        admission_preimage,
        admission,
        dispatch_ref,
        dispatch_preimage,
        dispatch,
        receipt_schema_sha256: SchemaDigest::parse(sha('e')).expect("receipt schema digest"),
    }
}

fn dispatch_wire(preimage: EffectDispatchIntentPreimageV1) -> EffectDispatchIntentWireV1 {
    let record = EffectDispatchIntentV1::seal(preimage, signature()).expect("dispatch shape");
    serde_json::from_value(serde_json::to_value(record).expect("dispatch serializes"))
        .expect("dispatch wire")
}

fn claim_wire(preimage: EffectConflictClaimPreimageV1) -> EffectConflictClaimWireV1 {
    let record = EffectConflictClaimV1::seal(preimage, signature()).expect("claim shape");
    serde_json::from_value(serde_json::to_value(record).expect("claim serializes"))
        .expect("claim wire")
}

fn admission_wire(preimage: EffectAdmissionPreimageV1) -> EffectAdmissionWireV1 {
    let record = EffectAdmissionV1::seal(preimage, signature()).expect("admission shape");
    serde_json::from_value(serde_json::to_value(record).expect("admission serializes"))
        .expect("admission wire")
}

fn receipt_preimage(lineage: &EffectLineage) -> EffectReceiptPreimageV1 {
    EffectReceiptPreimageV1 {
        schema_id: SchemaId::parse(EFFECT_RECEIPT_V1_SCHEMA).expect("receipt schema"),
        schema_sha256: lineage.receipt_schema_sha256.clone(),
        request_binding: lineage.binding.clone(),
        action_id: lineage.action.action_id().clone(),
        attempt_id: lineage.dispatch.attempt_id().clone(),
        effect_dispatch_intent_ref: lineage.dispatch_ref.clone(),
        effect_dispatch_intent_sha256: lineage.dispatch.canonical_dispatch_intent_sha256().clone(),
        principal_scope: lineage.request_preimage.scope.principal_scope.clone(),
        controller_scope: lineage.request_preimage.scope.controller_scope.clone(),
        relationship_scope: lineage.request_preimage.scope.relationship_scope.clone(),
        transport_status: BoundedResidentTextV1::parse("accepted").expect("status"),
        observed_effect: ObservedEffectV1::Partial,
        external_reference: None,
        observed_postcondition_digest: Some(typed_state("partial")),
        absence_evidence_ref: None,
        absence_evidence_sha256: None,
        action_body_sha256: lineage
            .action
            .canonical_action_body_sha256()
            .expect("action digest"),
        effect_admission_ref: lineage.admission_ref.clone(),
        effect_admission_sha256: lineage
            .admission
            .canonical_effect_admission_sha256()
            .clone(),
        conflict_claim_ref: lineage.claim_ref.clone(),
        conflict_claim_sha256: lineage
            .claim
            .canonical_effect_conflict_claim_sha256()
            .clone(),
        provider_request_digest: lineage.dispatch.provider_request_digest().clone(),
        response_digest: None,
        implementation_kind: lineage.dispatch.implementation_kind().clone(),
        implementation_version: lineage.dispatch.implementation_version().clone(),
        reconciliation_required: true,
        reconciliation_case_id: Some(lineage.dispatch.reserved_reconciliation_case_id().clone()),
        started_at: UnixTimeV1::new(13).expect("valid fixture time"),
        completed_at: UnixTimeV1::new(14).expect("valid fixture time"),
        deployment_runtime_authority_ref: record_ref("runtime-authority"),
        deployment_runtime_authority_sha256: artifact('a'),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("valid fixture signing key epoch"),
    }
}

fn receipt_wire(preimage: EffectReceiptPreimageV1) -> EffectReceiptWireV1 {
    let record = EffectReceiptV1::seal(preimage, signature()).expect("receipt shape");
    serde_json::from_value(serde_json::to_value(record).expect("receipt serializes"))
        .expect("receipt wire")
}

fn dispatch_context<'a>(lineage: &'a EffectLineage) -> EffectDispatchIntentAdmissionContextV1<'a> {
    EffectDispatchIntentAdmissionContextV1 {
        request_ref: &lineage.dispatch_preimage.request_ref,
        request: &lineage.request,
        action: &lineage.action,
        admission_ref: &lineage.admission_ref,
        admission: &lineage.admission,
        claim_ref: &lineage.claim_ref,
        claim: &lineage.claim,
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &lineage.dispatch_preimage.schema_sha256,
            signing_authority: &lineage.request_preimage.signing_authority_binding,
        },
    }
}

fn receipt_context<'a>(lineage: &'a EffectLineage) -> EffectReceiptAdmissionContextV1<'a> {
    EffectReceiptAdmissionContextV1 {
        request: &lineage.request,
        request_binding: &lineage.binding,
        action: &lineage.action,
        dispatch_ref: &lineage.dispatch_ref,
        dispatch: &lineage.dispatch,
        admission_ref: &lineage.admission_ref,
        admission: &lineage.admission,
        claim_ref: &lineage.claim_ref,
        claim: &lineage.claim,
        policy_requires_reconciliation: false,
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &lineage.receipt_schema_sha256,
            signing_authority: &lineage.request_preimage.signing_authority_binding,
        },
    }
}

#[test]
fn common_request_rejects_each_owner_resolved_coordinate_substitution() {
    let action = action();
    let fixture = action_request_fixture(&action);
    let expected = fixture.preimage;
    let request = CommonRequestEnvelopeV1::seal(expected.clone(), signature()).expect("request");
    CommonRequestEnvelopeV1::admit_contextual_wire(
        common_request_wire(&request),
        &request_context(
            &expected,
            Some((&fixture.execution_fence_ref, &fixture.execution_fence)),
        ),
        |_, _, _| Ok(()),
    )
    .expect("exact owner context admits");

    let mut cases = Vec::new();
    macro_rules! case {
        ($name:literal, $mutation:expr) => {{
            let mut changed = expected.clone();
            $mutation(&mut changed);
            cases.push(($name, changed));
        }};
    }
    case!(
        "schema digest",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            v.envelope_schema_sha256 = SchemaDigest::parse(sha('f')).expect("schema digest")
        }
    );
    case!(
        "body schema digest",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            v.body.schema_sha256 = SchemaDigest::parse(sha('f')).expect("body schema digest")
        }
    );
    case!(
        "body schema identity",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            v.body.schema_id = SchemaId::parse("example/other-body.v1").expect("body schema")
        }
    );
    case!("caller", |v: &mut CommonRequestEnvelopePreimageV1| {
        v.caller.role = RequestCallerRoleV1::parse("reviewer").expect("role")
    });
    case!(
        "operation target",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            v.operation_target = Some(OperationTargetReferenceV1 {
                operation_handle_ref: record_ref("other-operation-handle"),
                operation_handle_sha256: OperationHandleDigest::parse(sha('f'))
                    .expect("operation handle digest"),
            });
        }
    );
    case!(
        "signing authority",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            v.caller.signer_key_epoch =
                SigningKeyEpochV1::new(2).expect("valid changed signing key epoch");
            v.signing_authority_binding.signer_key_epoch =
                SigningKeyEpochV1::new(2).expect("valid changed signing key epoch");
        }
    );
    case!(
        "contract-set ref",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            v.route.contract_set_ref = record_ref("other-contract-set")
        }
    );
    case!(
        "contract-set digest",
        |v: &mut CommonRequestEnvelopePreimageV1| { v.route.contract_set_sha256 = artifact('f') }
    );
    case!("route schema", |v: &mut CommonRequestEnvelopePreimageV1| {
        v.route.route_binding_schema_id = SchemaId::parse("example/other-route.v1").expect("schema")
    });
    case!(
        "route schema digest",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            v.route.route_binding_schema_sha256 =
                SchemaDigest::parse(sha('f')).expect("schema digest")
        }
    );
    case!("route ref", |v: &mut CommonRequestEnvelopePreimageV1| {
        v.route.route_binding_ref = record_ref("other-route")
    });
    case!("route digest", |v: &mut CommonRequestEnvelopePreimageV1| {
        v.route.route_binding_sha256 = artifact('f')
    });
    case!(
        "route generation",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            v.route.route_generation =
                RouteGenerationV1::new(2).expect("valid changed route generation")
        }
    );
    case!(
        "route identity",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            v.route.route_id = ResidentRouteIdV1::parse("route-2").expect("route")
        }
    );
    case!(
        "route origin node",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            v.route.origin_node_id = ResidentNodeIdV1::parse("node-2").expect("origin node")
        }
    );
    case!(
        "route destination node",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            v.route.destination_node_id =
                Some(ResidentNodeIdV1::parse("node-2").expect("destination node"))
        }
    );
    case!(
        "route transport",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            v.route.transport = BoundedResidentTextV1::parse("verified-https").expect("transport")
        }
    );
    case!(
        "session admission",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            v.session_admission.session_admission_id =
                SessionAdmissionIdV1::parse("session-2").expect("session admission")
        }
    );
    case!(
        "session admission lease digest",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            v.session_admission.session_admission_lease_digest =
                SessionAdmissionLeaseDigest::parse(sha('f')).expect("session lease digest")
        }
    );
    case!(
        "session grant",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            v.session_admission.session_grant_id =
                SessionGrantIdV1::parse("grant-2").expect("grant")
        }
    );
    case!(
        "session digest",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            v.session_admission.session_grant_sha256 = artifact('f')
        }
    );
    case!(
        "deployment binding",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            let deployment_ref = record_ref("other-deployment");
            let deployment_sha256 = artifact('f');
            v.session_admission.deployment_binding_ref = deployment_ref.clone();
            v.session_admission.deployment_binding_sha256 = deployment_sha256.clone();
            let runtime = v
                .signing_authority_binding
                .integrated_runtime
                .as_mut()
                .expect("runtime authority");
            runtime.deployment_binding_ref = deployment_ref;
            runtime.deployment_binding_sha256 = deployment_sha256;
        }
    );
    case!(
        "revocation frontier",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            v.session_admission.checked_revocation_epoch =
                PrincipalRevocationEpochV1::new(2).expect("valid changed revocation epoch")
        }
    );
    macro_rules! fence_case {
        ($name:literal, $field:ident, $value:expr) => {
            case!($name, |v: &mut CommonRequestEnvelopePreimageV1| {
                v.execution_owner_fence.as_mut().expect("fence").$field = $value
            });
        };
    }
    fence_case!(
        "execution fence ref",
        execution_fence_ref,
        record_ref("other-execution-fence")
    );
    fence_case!(
        "execution fence digest",
        execution_fence_sha256,
        execution_fence_digest('f')
    );
    fence_case!(
        "execution fence attempt",
        execution_attempt_id,
        ExecutionAttemptIdV1::parse("execution-2").expect("attempt")
    );
    fence_case!(
        "execution fence path",
        execution_path_id,
        ExecutionPathSelectionIdV1::parse("execution-path-2").expect("path")
    );
    fence_case!(
        "execution fence owner system",
        execution_owner_system_id,
        ExecutionOwnerSystemIdV1::parse("owner-system-2").expect("owner system")
    );
    fence_case!(
        "execution fence owner participant",
        execution_owner_participant_id,
        ExecutionOwnerParticipantIdV1::parse("owner-participant-2").expect("owner participant")
    );
    fence_case!(
        "execution fence authorization schema",
        owner_authorization_schema_id,
        SchemaId::parse("owner/other-execution-authorization.v1").expect("authorization schema")
    );
    fence_case!(
        "execution fence authorization schema digest",
        owner_authorization_schema_sha256,
        SchemaDigest::parse(sha('f')).expect("authorization schema digest")
    );
    fence_case!(
        "execution fence authorization ref",
        owner_authorization_ref,
        record_ref("other-execution-authorization")
    );
    fence_case!(
        "execution fence authorization digest",
        owner_authorization_sha256,
        artifact('f')
    );
    fence_case!(
        "execution fence owner generation",
        owner_generation,
        ExecutionOwnerGenerationV1::new(2).expect("owner generation")
    );
    fence_case!(
        "execution fence control epoch",
        control_epoch,
        ExecutionControlEpochV1::new(2).expect("control epoch")
    );
    case!("federation", |v: &mut CommonRequestEnvelopePreimageV1| {
        v.federation_claim = Some(CommonRequestFederationClaimV1 {
            space_id: FederationSpaceIdV1::parse("space-1").expect("space"),
            space_governance_epoch: FederationGovernanceEpochV1::new(1)
                .expect("valid fixture governance epoch"),
            membership_id: FederationMembershipIdV1::parse("membership-1").expect("membership"),
            membership_state_epoch: FederationMembershipEpochV1::new(1)
                .expect("valid fixture membership epoch"),
            membership_sha256: artifact('a'),
            grant_id: FederationGrantIdV1::parse("federation-grant-1").expect("grant"),
            grant_sha256: artifact('b'),
            revocation_proofs: Vec::new(),
        })
    });
    case!("workspace", |v: &mut CommonRequestEnvelopePreimageV1| {
        v.workspace_id = ResidentWorkspaceIdV1::parse("workspace-2").expect("workspace")
    });
    case!("capability", |v: &mut CommonRequestEnvelopePreimageV1| {
        v.capability_id = CapabilityIdV1::parse("publish").expect("capability")
    });
    case!(
        "operation owner with the same resource identity",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            v.operation.package_id =
                PackageIdV1::parse("other-package").expect("other Package owner")
        }
    );
    case!("scope", |v: &mut CommonRequestEnvelopePreimageV1| {
        v.scope.purpose = BoundedResidentTextV1::parse("other-purpose").expect("purpose")
    });
    case!("risk", |v: &mut CommonRequestEnvelopePreimageV1| {
        v.risk_class = BoundedResidentTextV1::parse("elevated").expect("risk")
    });
    case!(
        "resource bounds",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            v.resource_constraints =
                vec![BoundedResidentTextV1::parse("resource-a").expect("constraint")]
        }
    );
    case!("precondition", |v: &mut CommonRequestEnvelopePreimageV1| {
        v.expected_precondition = Some(ExpectedPreconditionV1 {
            state_schema_id: SchemaId::parse("example/state.v1").expect("state schema"),
            state_schema_sha256: SchemaDigest::parse(sha('e')).expect("state schema digest"),
            expected_state_sha256: artifact('f'),
        })
    });
    case!("idempotency", |v: &mut CommonRequestEnvelopePreimageV1| {
        v.idempotency_key = RequestIdempotencyKeyV1::parse("idempotency-2").expect("idempotency")
    });
    case!("issued at", |v: &mut CommonRequestEnvelopePreimageV1| {
        v.issued_at = UnixTimeV1::new(11).expect("valid fixture time")
    });
    case!("expiry", |v: &mut CommonRequestEnvelopePreimageV1| {
        v.expires_at = UnixTimeV1::new(99).expect("valid fixture time")
    });
    case!("generation", |v: &mut CommonRequestEnvelopePreimageV1| {
        v.generation = Some(CompiledGenerationDigest::parse(sha('f')).expect("generation"))
    });
    case!(
        "activation fence",
        |v: &mut CommonRequestEnvelopePreimageV1| {
            v.fence_token = Some(ActivationFenceDigest::parse(sha('f')).expect("fence"))
        }
    );

    for (name, changed) in cases {
        assert!(
            CommonRequestEnvelopeV1::issue_contextual(
                changed.clone(),
                &request_context(
                    &expected,
                    Some((&fixture.execution_fence_ref, &fixture.execution_fence)),
                ),
                |_, _, _| panic!("{name} substitution must fail before signing"),
            )
            .is_err(),
            "{name} substitution must not be signed"
        );
        let changed = CommonRequestEnvelopeV1::seal(changed, signature())
            .expect("substitution remains structurally valid");
        assert!(
            CommonRequestEnvelopeV1::admit_contextual_wire(
                common_request_wire(&changed),
                &request_context(
                    &expected,
                    Some((&fixture.execution_fence_ref, &fixture.execution_fence)),
                ),
                |_, _, _| Ok(()),
            )
            .is_err(),
            "{name} substitution must fail"
        );
    }
}

#[path = "request_signing.rs"]
mod request_signing;

#[test]
fn common_request_wire_rejects_the_predecessor_schema_and_each_new_required_coordinate() {
    let action = action();
    let fixture = action_request_fixture(&action);
    let request = CommonRequestEnvelopeV1::seal(fixture.preimage.clone(), signature())
        .expect("current common request");
    let current = serde_json::to_value(&request).expect("common request serializes");

    let mut predecessor = current.clone();
    predecessor["envelope_schema_sha256"] = serde_json::Value::String(
        "a2f5dd1acffe988efa2f2df9fea42ad645200f4dc86f730c2b40727d8467c11f".to_owned(),
    );
    let predecessor: CommonRequestEnvelopeWireV1 =
        serde_json::from_value(predecessor).expect("predecessor remains structurally complete");
    assert!(CommonRequestEnvelopeV1::admit_contextual_wire(
        predecessor,
        &request_context(
            &fixture.preimage,
            Some((&fixture.execution_fence_ref, &fixture.execution_fence)),
        ),
        |_, _, _| Ok(()),
    )
    .is_err());

    for (container, member) in [
        (None, "operation"),
        (Some("session_admission"), "session_admission_id"),
        (Some("session_admission"), "session_admission_lease_digest"),
    ] {
        let mut missing = current.clone();
        let object = match container {
            Some(name) => missing[name]
                .as_object_mut()
                .unwrap_or_else(|| panic!("{name} is an object")),
            None => missing
                .as_object_mut()
                .expect("common request is an object"),
        };
        assert!(object.remove(member).is_some(), "fixture contains {member}");
        let error = serde_json::from_value::<CommonRequestEnvelopeWireV1>(missing)
            .expect_err("a new required coordinate cannot be omitted");
        assert!(
            error.to_string().contains(member),
            "missing-field error must identify {member}: {error}"
        );
    }
}

#[test]
fn common_request_derives_fence_projection_and_checks_existing_deployment_and_scope() {
    let action = action();
    let fixture = action_request_fixture(&action);
    let expected_projection = CommonRequestExecutionOwnerFenceV1::from_admitted(
        &fixture.execution_fence_ref,
        &fixture.execution_fence,
    );
    assert_eq!(
        fixture.preimage.execution_owner_fence.as_ref(),
        Some(&expected_projection)
    );

    let request = CommonRequestEnvelopeV1::seal(fixture.preimage.clone(), signature())
        .expect("common request");
    CommonRequestEnvelopeV1::admit_contextual_wire(
        common_request_wire(&request),
        &request_context(
            &fixture.preimage,
            Some((&fixture.execution_fence_ref, &fixture.execution_fence)),
        ),
        |_, _, _| Ok(()),
    )
    .expect("derived execution-fence projection admits");

    let mut exact_lifetime = fixture.preimage.clone();
    exact_lifetime.issued_at = *fixture.execution_fence.issued_at();
    exact_lifetime.expires_at = *fixture
        .execution_fence
        .expires_at()
        .as_ref()
        .expect("expiring fixture fence");
    let exact_lifetime_request = CommonRequestEnvelopeV1::seal(exact_lifetime.clone(), signature())
        .expect("boundary-aligned request");
    CommonRequestEnvelopeV1::admit_contextual_wire(
        common_request_wire(&exact_lifetime_request),
        &request_context(
            &exact_lifetime,
            Some((&fixture.execution_fence_ref, &fixture.execution_fence)),
        ),
        |_, _, _| Ok(()),
    )
    .expect("fence lifetime boundaries are inclusive");

    let mut without_fence = fixture.preimage.clone();
    without_fence.execution_owner_fence = None;
    let request_without_fence =
        CommonRequestEnvelopeV1::seal(without_fence.clone(), signature()).expect("direct request");
    CommonRequestEnvelopeV1::admit_contextual_wire(
        common_request_wire(&request_without_fence),
        &request_context(&without_fence, None),
        |_, _, _| Ok(()),
    )
    .expect("a path with no selected execution owner needs no synthetic fence");

    let (non_expiring_ref, non_expiring_fence) = execution_owner_fence_with_expiry(None);
    let mut non_expiring_request = fixture.preimage.clone();
    non_expiring_request.execution_owner_fence = Some(
        CommonRequestExecutionOwnerFenceV1::from_admitted(&non_expiring_ref, &non_expiring_fence),
    );
    let request = CommonRequestEnvelopeV1::seal(non_expiring_request.clone(), signature())
        .expect("request under a non-expiring fence");
    CommonRequestEnvelopeV1::admit_contextual_wire(
        common_request_wire(&request),
        &request_context(
            &non_expiring_request,
            Some((&non_expiring_ref, &non_expiring_fence)),
        ),
        |_, _, _| Ok(()),
    )
    .expect("an active non-expiring fence admits a later bounded request");

    let mut mismatches = Vec::new();
    let mut deployment = fixture.preimage.clone();
    deployment.session_admission.deployment_binding_ref = record_ref("other-deployment");
    deployment.session_admission.deployment_binding_sha256 = artifact('f');
    let runtime = deployment
        .signing_authority_binding
        .integrated_runtime
        .as_mut()
        .expect("runtime authority");
    runtime.deployment_binding_ref = deployment.session_admission.deployment_binding_ref.clone();
    runtime.deployment_binding_sha256 = deployment
        .session_admission
        .deployment_binding_sha256
        .clone();
    mismatches.push(("deployment", deployment));

    let mut principal = fixture.preimage.clone();
    principal.scope.principal_scope =
        ResidentPrincipalIdV1::parse("principal-2").expect("principal");
    mismatches.push(("principal scope", principal));

    let mut controller = fixture.preimage.clone();
    controller.scope.controller_scope =
        ResidentControllerIdV1::parse("controller-2").expect("controller");
    mismatches.push(("controller scope", controller));

    let mut relationship = fixture.preimage.clone();
    relationship.scope.relationship_scope =
        Some(RelationshipScopeV1::parse("relationship-1").expect("relationship"));
    mismatches.push(("relationship scope", relationship));

    let mut before_fence = fixture.preimage.clone();
    before_fence.issued_at = UnixTimeV1::new(7).expect("valid fixture time");
    mismatches.push(("issue time before fence", before_fence));

    let mut beyond_fence = fixture.preimage.clone();
    beyond_fence.expires_at = UnixTimeV1::new(101).expect("valid fixture time");
    mismatches.push(("expiry beyond fence", beyond_fence));

    for (axis, mismatch) in mismatches {
        let request = CommonRequestEnvelopeV1::seal(mismatch.clone(), signature())
            .expect("mismatch remains structurally valid");
        assert!(
            CommonRequestEnvelopeV1::admit_contextual_wire(
                common_request_wire(&request),
                &request_context(
                    &mismatch,
                    Some((&fixture.execution_fence_ref, &fixture.execution_fence)),
                ),
                |_, _, _| Ok(()),
            )
            .is_err(),
            "{axis} must match the admitted execution fence"
        );
    }

    let (retired_ref, retired) =
        retired_execution_owner_fence(&fixture.execution_fence_ref, &fixture.execution_fence);
    let mut retired_request = fixture.preimage;
    retired_request.execution_owner_fence = Some(
        CommonRequestExecutionOwnerFenceV1::from_admitted(&retired_ref, &retired),
    );
    retired_request.issued_at = *retired.issued_at();
    let request = CommonRequestEnvelopeV1::seal(retired_request.clone(), signature())
        .expect("retired-fence request remains structural");
    assert!(
        CommonRequestEnvelopeV1::admit_contextual_wire(
            common_request_wire(&request),
            &request_context(&retired_request, Some((&retired_ref, &retired))),
            |_, _, _| Ok(()),
        )
        .is_err(),
        "a retired execution fence must not admit new work"
    );
}

#[test]
fn contextual_effect_lineage_and_dispatch_route_fence_axes_are_exact() {
    let lineage = effect_lineage();
    let claim_wire: EffectConflictClaimWireV1 =
        serde_json::from_value(serde_json::to_value(&lineage.claim).expect("claim serializes"))
            .expect("claim wire");
    EffectConflictClaimV1::admit_initial_wire(
        claim_wire,
        &EffectConflictClaimAdmissionContextV1 {
            request: &lineage.request,
            action: &lineage.action,
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: lineage.claim.schema_sha256(),
                signing_authority: &lineage.request_preimage.signing_authority_binding,
            },
        },
        |_, _, _| Ok(()),
    )
    .expect("initial claim admits contextually");
    let admission_wire: EffectAdmissionWireV1 = serde_json::from_value(
        serde_json::to_value(&lineage.admission).expect("admission serializes"),
    )
    .expect("admission wire");
    EffectAdmissionV1::admit_contextual_wire(
        admission_wire,
        &EffectAdmissionContextV1 {
            request: &lineage.request,
            action: &lineage.action,
            claim_ref: &lineage.claim_ref,
            claim: &lineage.claim,
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: lineage.admission.schema_sha256(),
                signing_authority: &lineage.request_preimage.signing_authority_binding,
            },
        },
        |_, _, _| Ok(()),
    )
    .expect("effect admission admits contextually");
    EffectDispatchIntentV1::admit_contextual_wire(
        dispatch_wire(lineage.dispatch_preimage.clone()),
        &dispatch_context(&lineage),
        |_| None,
        |_, _, _| Ok(()),
    )
    .expect("exact dispatch admits");

    let mut cases = Vec::new();
    macro_rules! case {
        ($name:literal, $field:ident, $value:expr) => {{
            let mut changed = lineage.dispatch_preimage.clone();
            changed.$field = $value;
            cases.push(($name, changed));
        }};
    }
    case!(
        "contract set ref",
        contract_set_ref,
        record_ref("other-contract-set")
    );
    case!("contract set digest", contract_set_sha256, artifact('f'));
    case!(
        "route schema",
        route_binding_schema_id,
        SchemaId::parse("example/other-route.v1").expect("route schema")
    );
    case!(
        "route schema digest",
        route_binding_schema_sha256,
        SchemaDigest::parse(sha('f')).expect("route schema digest")
    );
    case!("route ref", route_binding_ref, record_ref("other-route"));
    case!("route digest", route_binding_sha256, artifact('f'));
    case!(
        "route generation",
        route_generation,
        RouteGenerationV1::new(2).expect("valid changed route generation")
    );
    case!(
        "execution fence ref",
        execution_owner_fence_ref,
        Some(record_ref("other-fence"))
    );
    case!(
        "execution fence digest",
        execution_owner_fence_sha256,
        Some(execution_fence_digest('f'))
    );
    for (name, changed) in cases {
        assert!(
            EffectDispatchIntentV1::admit_contextual_wire(
                dispatch_wire(changed),
                &dispatch_context(&lineage),
                |_| None,
                |_, _, _| Ok(()),
            )
            .is_err(),
            "{name} substitution must fail"
        );
    }
}

#[test]
fn effect_lineage_rejects_runtime_authority_substitution_at_every_signed_stage() {
    let lineage = effect_lineage();
    let claim_context = |authority| EffectConflictClaimAdmissionContextV1 {
        request: &lineage.request,
        action: &lineage.action,
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &lineage.claim_preimage.schema_sha256,
            signing_authority: authority,
        },
    };
    let admission_context = EffectAdmissionContextV1 {
        request: &lineage.request,
        action: &lineage.action,
        claim_ref: &lineage.claim_ref,
        claim: &lineage.claim,
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &lineage.admission_preimage.schema_sha256,
            signing_authority: &lineage.request_preimage.signing_authority_binding,
        },
    };

    for change_ref in [true, false] {
        let mut changed_claim = lineage.claim_preimage.clone();
        let mut changed_admission = lineage.admission_preimage.clone();
        let mut changed_dispatch = lineage.dispatch_preimage.clone();
        if change_ref {
            let unrelated = record_ref("other-runtime-authority");
            changed_claim.deployment_runtime_authority_ref = unrelated.clone();
            changed_admission.deployment_runtime_authority_ref = unrelated.clone();
            changed_dispatch.deployment_runtime_authority_ref = unrelated;
        } else {
            let unrelated = artifact('0');
            changed_claim.deployment_runtime_authority_sha256 = unrelated.clone();
            changed_admission.deployment_runtime_authority_sha256 = unrelated.clone();
            changed_dispatch.deployment_runtime_authority_sha256 = unrelated;
        }

        assert!(EffectConflictClaimV1::admit_initial_wire(
            claim_wire(changed_claim),
            &claim_context(&lineage.request_preimage.signing_authority_binding),
            |_, _, _| Ok(()),
        )
        .is_err());
        assert!(EffectAdmissionV1::admit_contextual_wire(
            admission_wire(changed_admission),
            &admission_context,
            |_, _, _| Ok(()),
        )
        .is_err());
        assert!(EffectDispatchIntentV1::admit_contextual_wire(
            dispatch_wire(changed_dispatch),
            &dispatch_context(&lineage),
            |_| None,
            |_, _, _| Ok(()),
        )
        .is_err());
    }

    let same_key_unrelated_authority = SigningAuthorityBindingV1::integrated_runtime(
        ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        ResidentKeyIdV1::parse("key-1").expect("same key"),
        SigningKeyEpochV1::new(1).expect("same key epoch"),
        IntegratedRuntimeAuthorityV1 {
            deployment_runtime_authority_ref: record_ref("other-runtime-authority"),
            deployment_runtime_authority_sha256: artifact('0'),
            deployment_binding_ref: record_ref("deployment"),
            deployment_binding_sha256: artifact('b'),
            checked_principal_trust_checkpoint_ref: record_ref("trust-checkpoint"),
            checked_principal_trust_checkpoint_sha256: artifact('c'),
        },
    )
    .expect("same-key unrelated authority");
    assert!(EffectConflictClaimV1::admit_initial_wire(
        claim_wire(lineage.claim_preimage.clone()),
        &claim_context(&same_key_unrelated_authority),
        |_, _, _| Ok(()),
    )
    .is_err());
}

fn changed_binding(
    binding: &RequestBindingV1,
    field: &str,
    value: serde_json::Value,
) -> RequestBindingV1 {
    let mut encoded = serde_json::to_value(binding).expect("binding serializes");
    encoded[field] = value;
    serde_json::from_value(encoded).expect("changed binding remains structural")
}

#[test]
fn receipt_rejects_complete_binding_scope_implementation_provider_and_case_swaps() {
    let lineage = effect_lineage();
    EffectReceiptV1::admit_contextual_wire(
        receipt_wire(receipt_preimage(&lineage)),
        &receipt_context(&lineage),
        |_| None,
        |_, _, _| Ok(()),
    )
    .expect("exact receipt admits");

    let mut cases = Vec::new();
    for (field, value) in [
        ("correlation_id", serde_json::json!("correlation-2")),
        ("capability_id", serde_json::json!("publish")),
        ("session_grant_id", serde_json::json!("grant-2")),
        ("session_grant_sha256", serde_json::json!(sha('f'))),
        (
            "deployment_binding_ref",
            serde_json::json!("other-deployment"),
        ),
        ("deployment_binding_sha256", serde_json::json!(sha('f'))),
        (
            "route_binding_schema_id",
            serde_json::json!("example/other-route.v1"),
        ),
        ("route_binding_schema_sha256", serde_json::json!(sha('f'))),
        ("route_binding_ref", serde_json::json!("other-route")),
        ("route_binding_sha256", serde_json::json!(sha('f'))),
        ("route_generation", serde_json::json!(2)),
        ("contract_set_ref", serde_json::json!("other-contract-set")),
        ("contract_set_sha256", serde_json::json!(sha('f'))),
        (
            "execution_owner_fence_ref",
            serde_json::json!("other-fence"),
        ),
        ("execution_owner_fence_sha256", serde_json::json!(sha('f'))),
        (
            "federation_request_admission_ref",
            serde_json::json!("other-federation-admission"),
        ),
        (
            "federation_request_admission_sha256",
            serde_json::json!(sha('f')),
        ),
    ] {
        let mut changed = receipt_preimage(&lineage);
        changed.request_binding = changed_binding(&lineage.binding, field, value);
        cases.push((field, changed));
    }
    let mut changed = receipt_preimage(&lineage);
    changed.principal_scope = ResidentPrincipalIdV1::parse("principal-2").expect("principal");
    cases.push(("principal scope", changed));
    let mut changed = receipt_preimage(&lineage);
    changed.controller_scope = ResidentControllerIdV1::parse("controller-2").expect("controller");
    cases.push(("controller scope", changed));
    let mut changed = receipt_preimage(&lineage);
    changed.implementation_kind = BoundedResidentTextV1::parse("remote").expect("kind");
    cases.push(("implementation kind", changed));
    let mut changed = receipt_preimage(&lineage);
    changed.implementation_version = BoundedResidentTextV1::parse("2").expect("version");
    cases.push(("implementation version", changed));
    let mut changed = receipt_preimage(&lineage);
    changed.provider_request_digest = Some(typed_state("other-provider-request"));
    cases.push(("provider request", changed));
    let mut changed = receipt_preimage(&lineage);
    changed.reconciliation_case_id = Some(ReconciliationCaseIdV1::parse("case-2").expect("case"));
    cases.push(("reserved case", changed));
    let mut changed = receipt_preimage(&lineage);
    changed.deployment_runtime_authority_ref = record_ref("other-runtime-authority");
    cases.push(("runtime authority ref", changed));
    let mut changed = receipt_preimage(&lineage);
    changed.deployment_runtime_authority_sha256 = artifact('0');
    cases.push(("runtime authority digest", changed));

    for (name, changed) in cases {
        assert!(
            EffectReceiptV1::admit_contextual_wire(
                receipt_wire(changed),
                &receipt_context(&lineage),
                |_| None,
                |_, _, _| Ok(()),
            )
            .is_err(),
            "{name} substitution must fail"
        );
    }
}

#[test]
fn initial_case_rejects_owner_resolved_expected_state_and_policy_swaps() {
    let lineage = effect_lineage();
    let receipt =
        EffectReceiptV1::seal(receipt_preimage(&lineage), signature()).expect("ambiguity receipt");
    let receipt_ref = lineage.dispatch_ref.clone();
    let expected_state = typed_state("expected");
    let reconciliation_policy_sha256 = artifact('b');
    let preimage = ReconciliationCasePreimageV1 {
        schema_id: SchemaId::parse(RECONCILIATION_CASE_V1_SCHEMA).expect("case schema"),
        schema_sha256: SchemaDigest::parse(sha('f')).expect("case schema digest"),
        case_id: lineage.dispatch.reserved_reconciliation_case_id().clone(),
        action_id: lineage.action.action_id().clone(),
        effect_admission_ref: lineage.admission_ref.clone(),
        effect_admission_sha256: lineage
            .admission
            .canonical_effect_admission_sha256()
            .clone(),
        conflict_resource_key_schema_id: lineage.action.conflict_resource_key_schema_id().clone(),
        conflict_resource_key_schema_sha256: lineage
            .action
            .conflict_resource_key_schema_sha256()
            .clone(),
        conflict_resource_key: lineage.action.conflict_resource_key().clone(),
        reserved_custody_id: lineage
            .dispatch
            .reserved_reconciliation_custody_id()
            .clone(),
        case_origin: ReconciliationCaseOriginV1::Initial,
        opening_evidence: vec![ReconciliationOpeningEvidenceV1::EffectReceipt {
            evidence_ref: receipt_ref.clone(),
            evidence_sha256: ArtifactDigest::parse(receipt.canonical_receipt_sha256().as_str())
                .expect("receipt digest"),
            evidence_schema_id: receipt.schema_id().clone(),
            evidence_schema_sha256: receipt.schema_sha256().clone(),
        }],
        expected_state_digest: expected_state.clone(),
        reconciliation_policy_ref: lineage.action.reconciliation_policy().clone(),
        reconciliation_policy_sha256: reconciliation_policy_sha256.clone(),
        predecessor_case_ref: None,
        predecessor_case_sha256: None,
        predecessor_terminal_result_ref: None,
        predecessor_terminal_result_sha256: None,
        reopen_authority_ref: None,
        reopen_authority_sha256: None,
        reopen_policy_ref: None,
        reopen_policy_sha256: None,
        reopen_reason_code: None,
        case_revision: ReconciliationCaseRevisionV1::new(0).expect("valid fixture case revision"),
        opened_at: UnixTimeV1::new(15).expect("valid fixture time"),
        deployment_runtime_authority_ref: record_ref("runtime-authority"),
        deployment_runtime_authority_sha256: artifact('a'),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("valid fixture signing key epoch"),
    };
    let case = ReconciliationCaseV1::seal(preimage, signature()).expect("case shape");
    let wire = || {
        serde_json::from_value::<ReconciliationCaseWireV1>(
            serde_json::to_value(&case).expect("case serializes"),
        )
        .expect("case wire")
    };
    let context = |expected_state_digest, reconciliation_policy_sha256| {
        ReconciliationCaseInitialAdmissionContextV1 {
            receipt_ref: &receipt_ref,
            receipt: &receipt,
            dispatch_ref: &lineage.dispatch_ref,
            dispatch: &lineage.dispatch,
            admission_ref: &lineage.admission_ref,
            admission: &lineage.admission,
            action: &lineage.action,
            expected_state_digest,
            reconciliation_policy_sha256,
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: case.schema_sha256(),
                signing_authority: &lineage.request_preimage.signing_authority_binding,
            },
        }
    };
    ReconciliationCaseV1::admit_initial_wire(
        wire(),
        &context(&expected_state, &reconciliation_policy_sha256),
        |_| None,
        |_, _, _| Ok(()),
    )
    .expect("exact initial case admits");
    let wrong_state = typed_state("other-expected");
    assert!(ReconciliationCaseV1::admit_initial_wire(
        wire(),
        &context(&wrong_state, &reconciliation_policy_sha256),
        |_| None,
        |_, _, _| Ok(()),
    )
    .is_err());
    let wrong_policy = artifact('c');
    assert!(ReconciliationCaseV1::admit_initial_wire(
        wire(),
        &context(&expected_state, &wrong_policy),
        |_| None,
        |_, _, _| Ok(()),
    )
    .is_err());
}
