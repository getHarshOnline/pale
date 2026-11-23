//! Exercises signed reconciliation requests and outcomes with explicit authority custody.

use super::*;

fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn artifact(byte: char) -> ArtifactDigest {
    ArtifactDigest::parse(sha(byte)).expect("artifact digest")
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
    serde_json::from_value(serde_json::json!({
        "authority_mode": "integrated_runtime",
        "principal_id": "principal-1",
        "signer_key_id": "key-1",
        "signer_key_epoch": 1,
        "integrated_runtime": {
            "deployment_runtime_authority_ref": runtime_authority_ref,
            "deployment_runtime_authority_sha256": sha(runtime_authority_sha256),
            "deployment_binding_ref": deployment_binding_ref,
            "deployment_binding_sha256": sha(deployment_binding_sha256),
            "checked_principal_trust_checkpoint_ref": "trust",
            "checked_principal_trust_checkpoint_sha256": sha('c')
        }
    }))
    .expect("signing authority")
}

fn signing_authority() -> SigningAuthorityBindingV1 {
    signing_authority_at("authority", 'a', "deployment", 'b')
}

fn common_request_preimage_for_reconciliation(
    body: &ReconciliationRequestV1,
) -> CommonRequestEnvelopePreimageV1 {
    let payload = serde_json::to_value(body).expect("reconciliation body serializes");
    let canonical = crate::canon::to_canonical_string(&payload).expect("canonical request body");
    CommonRequestEnvelopePreimageV1 {
        envelope_schema_id: SchemaId::parse("pale/common-request-envelope.v1")
            .expect("envelope schema"),
        envelope_schema_sha256: SchemaDigest::parse(sha('0')).expect("envelope schema digest"),
        request_id: ResidentRequestIdV1::parse("reconciliation-request-1").expect("request id"),
        correlation_id: ResidentCorrelationIdV1::parse("correlation-1").expect("correlation id"),
        workspace_id: ResidentWorkspaceIdV1::parse("workspace-1").expect("workspace id"),
        capability_id: CapabilityIdV1::parse("reconcile").expect("capability"),
        operation: PackageResourceRefV1 {
            package_id: PackageIdV1::parse("test-package").expect("Package id"),
            resource_id: PackageResourceIdV1::parse("reconcile-operation").expect("operation id"),
        },
        caller: RequestCallerV1 {
            identity: RequestCallerIdV1::parse("caller-1").expect("caller"),
            role: RequestCallerRoleV1::parse("operator").expect("role"),
            surface: RequestCallerSurfaceV1::parse("native").expect("surface"),
            signer_key_id: ResidentKeyIdV1::parse("key-1").expect("key id"),
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
            route_id: ResidentRouteIdV1::parse("route-1").expect("route id"),
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
            session_grant_id: SessionGrantIdV1::parse("grant-1").expect("grant id"),
            session_grant_sha256: artifact('4'),
            deployment_binding_ref: record_ref("deployment"),
            deployment_binding_sha256: artifact('b'),
            session_floor_epoch: PrincipalSessionFloorEpochV1::new(1)
                .expect("valid fixture session floor"),
            grant_issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1")
                .expect("grant issuer"),
            grant_issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("grant key"),
            grant_issuer_key_epoch: PrincipalKeyEpochV1::new(1)
                .expect("valid fixture principal key epoch"),
            checked_revocation_epoch: PrincipalRevocationEpochV1::new(1)
                .expect("valid fixture revocation epoch"),
            checked_revocation_ref: None,
            checked_revocation_sha256: None,
            checked_revocation_checkpoint_ref: record_ref("revocation-checkpoint"),
            checked_revocation_checkpoint_sha256: artifact('5'),
            checked_at: UnixTimeV1::new(9).expect("valid fixture time"),
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
            purpose: BoundedResidentTextV1::parse("reconcile").expect("purpose"),
            authority_evidence: Vec::new(),
        },
        risk_class: BoundedResidentTextV1::parse("bounded").expect("risk"),
        resource_constraints: Vec::new(),
        expected_precondition: None,
        idempotency_key: RequestIdempotencyKeyV1::parse("reconciliation-idempotency-1")
            .expect("idempotency key"),
        issued_at: UnixTimeV1::new(10).expect("valid fixture time"),
        expires_at: UnixTimeV1::new(100).expect("valid fixture time"),
        generation: None,
        fence_token: None,
        body: CommonRequestBodyV1 {
            schema_id: SchemaId::parse("pale/reconciliation-request.v1").expect("body schema"),
            schema_sha256: SchemaDigest::parse(sha('6')).expect("body schema digest"),
            payload_sha256: ArtifactDigest::hash_bytes(canonical.as_bytes()),
            payload,
        },
    }
}

fn common_request_for_reconciliation(body: &ReconciliationRequestV1) -> CommonRequestEnvelopeV1 {
    CommonRequestEnvelopeV1::seal(
        common_request_preimage_for_reconciliation(body),
        signature(),
    )
    .expect("caller-signed common request")
}

fn request_binding_for_common_request(
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
        federation_request_admission_ref: None,
        federation_request_admission_sha256: None,
    })
    .expect("complete request binding")
}

fn typed_state(state: &str) -> TypedDigestV1 {
    let value = serde_json::json!({"state": state});
    let canonical = crate::canon::to_canonical_string(&value).expect("canonical state");
    TypedDigestV1::admit(
        TypedDigestPreimageV1 {
            kind: BoundedResidentTextV1::parse("state").expect("digest kind"),
            content_schema_id: SchemaId::parse("example/state.v1").expect("state schema"),
            content_schema_sha256: SchemaDigest::parse(sha('a')).expect("state schema digest"),
            canonicalization: TypedDigestCanonicalizationV1::Rfc8785Json,
            digest_algorithm: TypedDigestAlgorithmV1::Sha256,
            digest: ArtifactDigest::hash_bytes(canonical.as_bytes()),
            content_binding: TypedDigestContentBindingV1::InlineJson { inline_json: value },
        },
        |_| None,
    )
    .expect("typed state")
}

fn request_binding() -> RequestBindingV1 {
    RequestBindingV1::new(RequestBindingPreimageV1 {
        request_id: ResidentRequestIdV1::parse("request-1").expect("request id"),
        correlation_id: ResidentCorrelationIdV1::parse("correlation-1").expect("correlation id"),
        capability_id: CapabilityIdV1::parse("reconcile").expect("capability"),
        canonical_request_sha256: CommonRequestEnvelopeDigest::parse(sha('b'))
            .expect("request digest"),
        session_grant_id: SessionGrantIdV1::parse("grant-1").expect("session grant id"),
        session_grant_sha256: artifact('c'),
        deployment_binding_ref: record_ref("deployment"),
        deployment_binding_sha256: artifact('d'),
        route_binding_schema_id: SchemaId::parse("example/route.v1").expect("route schema"),
        route_binding_schema_sha256: SchemaDigest::parse(sha('e')).expect("route schema digest"),
        route_binding_ref: record_ref("route"),
        route_binding_sha256: artifact('f'),
        route_generation: RouteGenerationV1::new(1).expect("valid fixture route generation"),
        contract_set_ref: record_ref("contract-set"),
        contract_set_sha256: artifact('1'),
        execution_owner_fence_ref: None,
        execution_owner_fence_sha256: None,
        federation_request_admission_ref: None,
        federation_request_admission_sha256: None,
    })
    .expect("request binding")
}

fn receipt_evidence() -> ReconciliationOpeningEvidenceV1 {
    ReconciliationOpeningEvidenceV1::EffectReceipt {
        evidence_ref: record_ref("effect-receipt"),
        evidence_sha256: artifact('1'),
        evidence_schema_id: SchemaId::parse("pale/effect-receipt.v1").expect("receipt schema"),
        evidence_schema_sha256: SchemaDigest::parse(sha('2')).expect("receipt schema digest"),
    }
}

fn observation_opening() -> ReconciliationOpeningEvidenceV1 {
    ReconciliationOpeningEvidenceV1::Observation {
        evidence_ref: record_ref("new-observation"),
        evidence_sha256: artifact('3'),
        evidence_schema_id: SchemaId::parse("example/observation.v1").expect("observation schema"),
        evidence_schema_sha256: SchemaDigest::parse(sha('4')).expect("observation schema digest"),
        observation_kind: ObservationKindV1::parse("read_back").expect("observation kind"),
    }
}

fn reviewer_opening() -> ReconciliationOpeningEvidenceV1 {
    ReconciliationOpeningEvidenceV1::ReviewerEvidence {
        evidence_ref: record_ref("reviewer-evidence"),
        evidence_sha256: artifact('d'),
        evidence_schema_id: SchemaId::parse("example/reviewer-evidence.v1")
            .expect("reviewer evidence schema"),
        evidence_schema_sha256: SchemaDigest::parse(sha('e'))
            .expect("reviewer evidence schema digest"),
        reviewer_or_policy_ref: record_ref("reviewer-policy"),
        reviewer_or_policy_sha256: artifact('f'),
    }
}

fn case_preimage(origin: ReconciliationCaseOriginV1) -> ReconciliationCasePreimageV1 {
    let mut value = ReconciliationCasePreimageV1 {
        schema_id: SchemaId::parse("pale/reconciliation-case.v1").expect("case schema"),
        schema_sha256: SchemaDigest::parse(sha('5')).expect("case schema digest"),
        case_id: ReconciliationCaseIdV1::parse("case-1").expect("case id"),
        action_id: ActionIdV1::parse("action-1").expect("action id"),
        effect_admission_ref: record_ref("effect-admission"),
        effect_admission_sha256: EffectAdmissionDigest::parse(sha('6'))
            .expect("effect admission digest"),
        conflict_resource_key_schema_id: SchemaId::parse("example/resource-key.v1")
            .expect("resource-key schema"),
        conflict_resource_key_schema_sha256: SchemaDigest::parse(sha('7'))
            .expect("resource-key schema digest"),
        conflict_resource_key: serde_json::json!({"resource":"one"}),
        reserved_custody_id: ReconciliationCustodyIdV1::parse("custody-1").expect("custody id"),
        case_origin: origin,
        opening_evidence: vec![receipt_evidence()],
        expected_state_digest: typed_state("expected"),
        reconciliation_policy_ref: record_ref("reconciliation-policy"),
        reconciliation_policy_sha256: artifact('8'),
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
        opened_at: UnixTimeV1::new(10).expect("valid fixture time"),
        deployment_runtime_authority_ref: record_ref("authority"),
        deployment_runtime_authority_sha256: artifact('a'),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("valid fixture signing key epoch"),
    };
    if origin == ReconciliationCaseOriginV1::Reopened {
        value.opening_evidence = vec![observation_opening()];
        value.predecessor_case_ref = Some(record_ref("predecessor-case"));
        value.predecessor_case_sha256 =
            Some(ReconciliationCaseDigest::parse(sha('9')).expect("predecessor case digest"));
        value.predecessor_terminal_result_ref = Some(record_ref("terminal-result"));
        value.predecessor_terminal_result_sha256 =
            Some(ReconciliationResultDigest::parse(sha('a')).expect("terminal result digest"));
        value.reopen_authority_ref = Some(record_ref("reopen-authority"));
        value.reopen_authority_sha256 = Some(artifact('b'));
        value.reopen_policy_ref = Some(record_ref("reopen-policy"));
        value.reopen_policy_sha256 = Some(artifact('c'));
        value.reopen_reason_code =
            Some(ResidentReasonV1::parse("new_evidence").expect("reopen reason"));
    }
    value
}

#[test]
fn reconciliation_case_origin_and_opening_evidence_matrix() {
    let initial = case_preimage(ReconciliationCaseOriginV1::Initial);
    let reopened = case_preimage(ReconciliationCaseOriginV1::Reopened);
    let mut reopened_by_reviewer = reopened.clone();
    reopened_by_reviewer.opening_evidence = vec![reviewer_opening()];

    let mut initial_without_receipt = initial.clone();
    initial_without_receipt.opening_evidence = vec![observation_opening()];
    let mut initial_with_only_reviewer = initial.clone();
    initial_with_only_reviewer.opening_evidence = vec![reviewer_opening()];
    let mut initial_with_predecessor = initial.clone();
    initial_with_predecessor.predecessor_case_ref = Some(record_ref("predecessor-case"));
    let mut reopened_without_new_evidence = reopened.clone();
    reopened_without_new_evidence.opening_evidence = vec![receipt_evidence()];
    let mut reopened_without_complete_lineage = reopened.clone();
    reopened_without_complete_lineage.reopen_policy_sha256 = None;
    let mut empty_opening = initial.clone();
    empty_opening.opening_evidence.clear();

    let cases = [
        ("initial effect receipt", initial, true),
        ("reopened new observation", reopened, true),
        ("reopened reviewer evidence", reopened_by_reviewer, true),
        (
            "initial without effect receipt",
            initial_without_receipt,
            false,
        ),
        (
            "initial with only reviewer evidence",
            initial_with_only_reviewer,
            false,
        ),
        ("initial with predecessor", initial_with_predecessor, false),
        (
            "reopened without new evidence",
            reopened_without_new_evidence,
            false,
        ),
        (
            "reopened without complete lineage",
            reopened_without_complete_lineage,
            false,
        ),
        ("empty opening evidence", empty_opening, false),
    ];
    for (name, preimage, accepted) in cases {
        assert_eq!(
            ReconciliationCaseV1::seal(preimage, signature()).is_ok(),
            accepted,
            "{name}"
        );
    }
}

fn custody_preimage(state: ReconciliationCustodyStateV1) -> ReconciliationCustodyPreimageV1 {
    let mut value = ReconciliationCustodyPreimageV1 {
        schema_id: SchemaId::parse("pale/reconciliation-custody.v1").expect("custody schema"),
        schema_sha256: SchemaDigest::parse(sha('d')).expect("custody schema digest"),
        custody_id: ReconciliationCustodyIdV1::parse("custody-1").expect("custody id"),
        case_ref: record_ref("case"),
        case_sha256: ReconciliationCaseDigest::parse(sha('e')).expect("case digest"),
        conflict_resource_key_schema_id: SchemaId::parse("example/resource-key.v1")
            .expect("resource key schema"),
        conflict_resource_key_schema_sha256: SchemaDigest::parse(sha('f'))
            .expect("resource key schema digest"),
        conflict_resource_key: serde_json::json!({"resource":"one"}),
        custody_generation: ReconciliationCustodyGenerationV1::new(1)
            .expect("valid fixture custody generation"),
        custody_state: state,
        holder_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("holder principal"),
        holder_node_id: None,
        previous_custody_ref: None,
        previous_custody_sha256: None,
        terminal_result_ref: None,
        terminal_result_sha256: None,
        transferred_or_closed_at: None,
        deployment_runtime_authority_ref: record_ref("authority"),
        deployment_runtime_authority_sha256: artifact('a'),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer principal"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("valid fixture signing key epoch"),
    };
    if state != ReconciliationCustodyStateV1::Active {
        value.custody_generation =
            ReconciliationCustodyGenerationV1::new(2).expect("valid successor generation");
        value.previous_custody_ref = Some(record_ref("previous-custody"));
        value.previous_custody_sha256 =
            Some(ReconciliationCustodyDigest::parse(sha('1')).expect("previous custody digest"));
        value.transferred_or_closed_at = Some(UnixTimeV1::new(20).expect("valid fixture time"));
    }
    if state == ReconciliationCustodyStateV1::Closed {
        value.terminal_result_ref = Some(record_ref("terminal-result"));
        value.terminal_result_sha256 =
            Some(ReconciliationResultDigest::parse(sha('2')).expect("terminal result digest"));
    }
    value
}

#[test]
fn reconciliation_custody_generation_and_state_matrix() {
    let active = custody_preimage(ReconciliationCustodyStateV1::Active);
    let transferred = custody_preimage(ReconciliationCustodyStateV1::Transferred);
    let closed = custody_preimage(ReconciliationCustodyStateV1::Closed);

    let mut active_successor = active.clone();
    active_successor.custody_generation =
        ReconciliationCustodyGenerationV1::new(2).expect("valid successor generation");
    active_successor.previous_custody_ref = Some(record_ref("previous-custody"));
    active_successor.previous_custody_sha256 =
        Some(ReconciliationCustodyDigest::parse(sha('1')).expect("previous custody digest"));
    let mut transferred_without_time = transferred.clone();
    transferred_without_time.transferred_or_closed_at = None;
    let mut transferred_first_generation = transferred.clone();
    transferred_first_generation.custody_generation =
        ReconciliationCustodyGenerationV1::new(1).expect("valid initial generation");
    transferred_first_generation.previous_custody_ref = None;
    transferred_first_generation.previous_custody_sha256 = None;
    let mut transferred_with_terminal = transferred.clone();
    transferred_with_terminal.terminal_result_ref = Some(record_ref("terminal-result"));
    transferred_with_terminal.terminal_result_sha256 =
        Some(ReconciliationResultDigest::parse(sha('2')).expect("terminal result digest"));
    let mut closed_without_terminal = closed.clone();
    closed_without_terminal.terminal_result_ref = None;
    closed_without_terminal.terminal_result_sha256 = None;
    let mut closed_first_generation = closed.clone();
    closed_first_generation.custody_generation =
        ReconciliationCustodyGenerationV1::new(1).expect("valid initial generation");
    closed_first_generation.previous_custody_ref = None;
    closed_first_generation.previous_custody_sha256 = None;
    let mut incomplete_previous_pair = transferred.clone();
    incomplete_previous_pair.previous_custody_sha256 = None;
    let cases = [
        ("active generation one", active, true),
        ("transferred successor", transferred, true),
        ("closed successor", closed, true),
        ("active successor", active_successor, false),
        ("transferred without time", transferred_without_time, false),
        (
            "transferred first generation",
            transferred_first_generation,
            false,
        ),
        (
            "transferred with terminal",
            transferred_with_terminal,
            false,
        ),
        ("closed without terminal", closed_without_terminal, false),
        ("closed first generation", closed_first_generation, false),
        ("incomplete previous pair", incomplete_previous_pair, false),
    ];
    for (name, preimage, accepted) in cases {
        assert_eq!(
            ReconciliationCustodyV1::seal(preimage, signature()).is_ok(),
            accepted,
            "{name}"
        );
    }
}

fn result_preimage(
    state: ReconciliationCaseStateV1,
    resolution: Option<ReconciliationResolutionV1>,
) -> ReconciliationResultPreimageV1 {
    let expected = typed_state("expected");
    let mut value = ReconciliationResultPreimageV1 {
        schema_id: SchemaId::parse("pale/reconciliation-result.v1").expect("result schema"),
        schema_sha256: SchemaDigest::parse(sha('3')).expect("result schema digest"),
        request_binding: request_binding(),
        reconciliation_id: ReconciliationIdV1::parse("reconciliation-1")
            .expect("reconciliation id"),
        case_ref: record_ref("case"),
        case_sha256: ReconciliationCaseDigest::parse(sha('4')).expect("case digest"),
        custody_ref: record_ref("custody"),
        custody_sha256: ReconciliationCustodyDigest::parse(sha('5')).expect("custody digest"),
        custody_generation: ReconciliationCustodyGenerationV1::new(1)
            .expect("valid fixture custody generation"),
        case_revision: ReconciliationCaseRevisionV1::new(1).expect("valid fixture case revision"),
        predecessor_result_ref: None,
        predecessor_result_sha256: None,
        action_id: ActionIdV1::parse("action-1").expect("action id"),
        effect_admission_ref: record_ref("effect-admission"),
        effect_admission_sha256: EffectAdmissionDigest::parse(sha('6'))
            .expect("effect admission digest"),
        effect_receipts: vec![ReconciliationEffectReceiptRefV1 {
            receipt_ref: record_ref("effect-receipt"),
            receipt_sha256: EffectReceiptDigest::parse(sha('7')).expect("effect receipt digest"),
        }],
        principal_scope: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        controller_scope: ResidentControllerIdV1::parse("controller-1").expect("controller"),
        relationship_scope: None,
        reconciliation_policy_ref: record_ref("reconciliation-policy"),
        reconciliation_policy_sha256: artifact('8'),
        expected_state_digest: expected.clone(),
        observed_state_digest: None,
        observation_evidence: Vec::new(),
        absence_evidence_ref: None,
        absence_evidence_sha256: None,
        known_attempt_ids: vec![
            CapabilityAttemptIdV1::parse("attempt-1").expect("capability attempt")
        ],
        case_state: state,
        resolution,
        compensation: None,
        owner_evidence_ref: None,
        owner_evidence_sha256: None,
        reviewer_or_policy_ref: record_ref("reviewer-policy"),
        reviewer_or_policy_sha256: artifact('9'),
        requested_at: UnixTimeV1::new(10).expect("valid fixture time"),
        recorded_at: UnixTimeV1::new(20).expect("valid fixture time"),
        closed_at: Some(UnixTimeV1::new(30).expect("valid fixture time")),
        deployment_runtime_authority_ref: record_ref("authority"),
        deployment_runtime_authority_sha256: artifact('a'),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer principal"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("valid fixture signing key epoch"),
    };
    match state {
        ReconciliationCaseStateV1::Matched => {
            value.observed_state_digest = Some(expected);
        }
        ReconciliationCaseStateV1::NotApplied => {
            value.absence_evidence_ref = Some(record_ref("absence-evidence"));
            value.absence_evidence_sha256 = Some(artifact('a'));
        }
        ReconciliationCaseStateV1::Drift => {
            value.observed_state_digest = Some(typed_state("drifted"));
            if resolution.is_none() {
                value.closed_at = None;
            }
        }
        ReconciliationCaseStateV1::Unknown => {
            value.observation_evidence = vec![ReconciliationObservationEvidenceV1 {
                evidence_ref: record_ref("observation"),
                evidence_sha256: artifact('b'),
            }];
            if !matches!(resolution, Some(ReconciliationResolutionV1::StillUnknown)) {
                value.closed_at = None;
            }
        }
    }
    if matches!(resolution, Some(ReconciliationResolutionV1::Compensated)) {
        value.compensation = Some(ReconciliationCompensationBindingV1 {
            compensation_request_ref: record_ref("compensation-request"),
            compensation_request_sha256: CommonRequestEnvelopeDigest::parse(sha('c'))
                .expect("compensation request digest"),
            compensation_action_ref: record_ref("compensation-action"),
            compensation_action_sha256: ActionEnvelopeDigest::parse(sha('d'))
                .expect("compensation action digest"),
            compensation_result_ref: record_ref("compensation-result"),
            compensation_result_sha256: EffectReceiptDigest::parse(sha('e'))
                .expect("compensation result digest"),
            case_ref: value.case_ref.clone(),
            case_sha256: value.case_sha256.clone(),
            custody_ref: value.custody_ref.clone(),
            custody_sha256: value.custody_sha256.clone(),
            effect_admission_ref: value.effect_admission_ref.clone(),
            effect_admission_sha256: value.effect_admission_sha256.clone(),
            effect_receipts: value.effect_receipts.clone(),
        });
    }
    if state == ReconciliationCaseStateV1::Drift && resolution.is_some() {
        value.owner_evidence_ref = Some(record_ref("owner-evidence"));
        value.owner_evidence_sha256 = Some(artifact('d'));
    }
    value
}

#[test]
fn reconciliation_result_revision_predecessor_and_time_matrix() {
    let revision_one = result_preimage(
        ReconciliationCaseStateV1::Matched,
        Some(ReconciliationResolutionV1::EffectConfirmed),
    );
    let mut revision_two = revision_one.clone();
    revision_two.case_revision = ReconciliationCaseRevisionV1::new(2).expect("valid revision two");
    revision_two.predecessor_result_ref = Some(record_ref("predecessor-result"));
    revision_two.predecessor_result_sha256 =
        Some(ReconciliationResultDigest::parse(sha('d')).expect("predecessor result digest"));
    let mut revision_two_without_predecessor = revision_one.clone();
    revision_two_without_predecessor.case_revision =
        ReconciliationCaseRevisionV1::new(2).expect("valid revision two");
    let mut revision_zero = revision_one.clone();
    revision_zero.case_revision =
        ReconciliationCaseRevisionV1::new(0).expect("valid zero-aware revision");
    let mut revision_one_with_predecessor = revision_two.clone();
    revision_one_with_predecessor.case_revision =
        ReconciliationCaseRevisionV1::new(1).expect("valid revision one");
    let mut partial_predecessor_pair = revision_two.clone();
    partial_predecessor_pair.predecessor_result_sha256 = None;
    let mut request_after_record = revision_one.clone();
    request_after_record.requested_at = UnixTimeV1::new(21).expect("valid fixture time");
    let mut close_before_record = revision_one.clone();
    close_before_record.closed_at = Some(UnixTimeV1::new(19).expect("valid fixture time"));

    let cases = [
        ("revision one", revision_one, true),
        ("revision two", revision_two, true),
        (
            "revision two without predecessor",
            revision_two_without_predecessor,
            false,
        ),
        ("revision zero", revision_zero, false),
        (
            "revision one with predecessor",
            revision_one_with_predecessor,
            false,
        ),
        ("partial predecessor pair", partial_predecessor_pair, false),
        ("request after record", request_after_record, false),
        ("close before record", close_before_record, false),
    ];
    for (name, preimage, accepted) in cases {
        assert_eq!(
            ReconciliationResultV1::seal(preimage, signature()).is_ok(),
            accepted,
            "{name}"
        );
    }
}

#[test]
fn reconciliation_result_state_resolution_cross_product() {
    let states = [
        ReconciliationCaseStateV1::Matched,
        ReconciliationCaseStateV1::NotApplied,
        ReconciliationCaseStateV1::Drift,
        ReconciliationCaseStateV1::Unknown,
    ];
    let resolutions = [
        None,
        Some(ReconciliationResolutionV1::EffectConfirmed),
        Some(ReconciliationResolutionV1::EffectAbsent),
        Some(ReconciliationResolutionV1::Compensated),
        Some(ReconciliationResolutionV1::ProviderStateAccepted),
        Some(ReconciliationResolutionV1::ExpectedStateRestored),
        Some(ReconciliationResolutionV1::ManualReviewRequired),
        Some(ReconciliationResolutionV1::StillUnknown),
    ];

    for state in states {
        for resolution in resolutions {
            let accepted = matches!(
                (state, resolution),
                (
                    ReconciliationCaseStateV1::Matched,
                    Some(ReconciliationResolutionV1::EffectConfirmed)
                ) | (
                    ReconciliationCaseStateV1::NotApplied,
                    Some(ReconciliationResolutionV1::EffectAbsent)
                ) | (ReconciliationCaseStateV1::Drift, None)
                    | (
                        ReconciliationCaseStateV1::Drift,
                        Some(
                            ReconciliationResolutionV1::Compensated
                                | ReconciliationResolutionV1::ProviderStateAccepted
                                | ReconciliationResolutionV1::ExpectedStateRestored
                        )
                    )
                    | (ReconciliationCaseStateV1::Unknown, None)
                    | (
                        ReconciliationCaseStateV1::Unknown,
                        Some(
                            ReconciliationResolutionV1::ManualReviewRequired
                                | ReconciliationResolutionV1::StillUnknown
                        )
                    )
            );
            assert_eq!(
                ReconciliationResultV1::seal(result_preimage(state, resolution), signature(),)
                    .is_ok(),
                accepted,
                "state {state:?}, resolution {resolution:?}"
            );
        }
    }
}

#[test]
fn reconciliation_result_state_evidence_and_compensation_matrix() {
    let matched = result_preimage(
        ReconciliationCaseStateV1::Matched,
        Some(ReconciliationResolutionV1::EffectConfirmed),
    );
    let absence = result_preimage(
        ReconciliationCaseStateV1::NotApplied,
        Some(ReconciliationResolutionV1::EffectAbsent),
    );
    let drift_open = result_preimage(ReconciliationCaseStateV1::Drift, None);
    let drift_compensated = result_preimage(
        ReconciliationCaseStateV1::Drift,
        Some(ReconciliationResolutionV1::Compensated),
    );
    let drift_accepted = result_preimage(
        ReconciliationCaseStateV1::Drift,
        Some(ReconciliationResolutionV1::ProviderStateAccepted),
    );
    let drift_restored = result_preimage(
        ReconciliationCaseStateV1::Drift,
        Some(ReconciliationResolutionV1::ExpectedStateRestored),
    );
    let unknown_open = result_preimage(ReconciliationCaseStateV1::Unknown, None);
    let unknown_review = result_preimage(
        ReconciliationCaseStateV1::Unknown,
        Some(ReconciliationResolutionV1::ManualReviewRequired),
    );
    let unknown_terminal = result_preimage(
        ReconciliationCaseStateV1::Unknown,
        Some(ReconciliationResolutionV1::StillUnknown),
    );

    let mut matched_without_observation = matched.clone();
    matched_without_observation.observed_state_digest = None;
    let mut matched_wrong_observation = matched.clone();
    matched_wrong_observation.observed_state_digest = Some(typed_state("other"));
    let mut matched_without_close = matched.clone();
    matched_without_close.closed_at = None;
    let mut absence_without_evidence = absence.clone();
    absence_without_evidence.absence_evidence_ref = None;
    absence_without_evidence.absence_evidence_sha256 = None;
    let mut absence_with_observation = absence.clone();
    absence_with_observation.observed_state_digest = Some(typed_state("other"));
    let mut drift_without_observation = drift_open.clone();
    drift_without_observation.observed_state_digest = None;
    let mut open_drift_with_close = drift_open.clone();
    open_drift_with_close.closed_at = Some(UnixTimeV1::new(30).expect("valid fixture time"));
    let mut compensated_without_action = drift_compensated.clone();
    compensated_without_action.compensation = None;
    let mut noncompensated_with_action = drift_accepted.clone();
    noncompensated_with_action.compensation = drift_compensated.compensation.clone();
    let mut terminal_drift_without_owner_evidence = drift_restored.clone();
    terminal_drift_without_owner_evidence.owner_evidence_ref = None;
    terminal_drift_without_owner_evidence.owner_evidence_sha256 = None;
    let mut open_drift_with_owner_evidence = drift_open.clone();
    open_drift_with_owner_evidence.owner_evidence_ref = Some(record_ref("owner-evidence"));
    open_drift_with_owner_evidence.owner_evidence_sha256 = Some(artifact('f'));
    let mut unknown_without_evidence = unknown_open.clone();
    unknown_without_evidence.observation_evidence.clear();
    let mut unknown_with_observation_digest = unknown_open.clone();
    unknown_with_observation_digest.observed_state_digest = Some(typed_state("other"));
    let mut manual_review_closed = unknown_review.clone();
    manual_review_closed.closed_at = Some(UnixTimeV1::new(30).expect("valid fixture time"));
    let mut still_unknown_open = unknown_terminal.clone();
    still_unknown_open.closed_at = None;

    let cases = [
        ("matched", matched, true),
        ("effect absent", absence, true),
        ("open drift", drift_open, true),
        ("compensated drift", drift_compensated, true),
        ("accepted provider drift", drift_accepted, true),
        ("restored expected state", drift_restored, true),
        ("open unknown", unknown_open, true),
        ("manual review unknown", unknown_review, true),
        ("terminal unknown", unknown_terminal, true),
        (
            "matched without observation",
            matched_without_observation,
            false,
        ),
        (
            "matched wrong observation",
            matched_wrong_observation,
            false,
        ),
        ("matched without close", matched_without_close, false),
        ("absence without evidence", absence_without_evidence, false),
        ("absence with observation", absence_with_observation, true),
        (
            "drift without observation",
            drift_without_observation,
            false,
        ),
        ("open drift with close", open_drift_with_close, false),
        (
            "compensated without action",
            compensated_without_action,
            false,
        ),
        (
            "noncompensated with action",
            noncompensated_with_action,
            false,
        ),
        (
            "terminal drift without owner evidence",
            terminal_drift_without_owner_evidence,
            false,
        ),
        (
            "open drift with owner evidence",
            open_drift_with_owner_evidence,
            false,
        ),
        ("unknown without evidence", unknown_without_evidence, false),
        (
            "unknown with observation digest",
            unknown_with_observation_digest,
            false,
        ),
        ("manual review closed", manual_review_closed, false),
        ("still unknown open", still_unknown_open, false),
    ];
    for (name, preimage, accepted) in cases {
        assert_eq!(
            ReconciliationResultV1::seal(preimage, signature()).is_ok(),
            accepted,
            "{name}"
        );
    }
}

fn result_wire(preimage: ReconciliationResultPreimageV1) -> ReconciliationResultWireV1 {
    let value =
        ReconciliationResultV1::seal(preimage, signature()).expect("structurally valid result");
    serde_json::from_value(serde_json::to_value(value).expect("result serializes"))
        .expect("result wire decodes")
}

fn custody_wire(preimage: ReconciliationCustodyPreimageV1) -> ReconciliationCustodyWireV1 {
    let value =
        ReconciliationCustodyV1::seal(preimage, signature()).expect("structurally valid custody");
    serde_json::from_value(serde_json::to_value(value).expect("custody serializes"))
        .expect("custody wire decodes")
}

fn case_wire(preimage: ReconciliationCasePreimageV1) -> ReconciliationCaseWireV1 {
    let value = ReconciliationCaseV1::seal(preimage, signature()).expect("structurally valid case");
    serde_json::from_value(serde_json::to_value(value).expect("case serializes"))
        .expect("case wire decodes")
}

fn request_for_result(
    predecessor: Option<(&ResidentRecordRefV1, &ReconciliationResultV1)>,
    result: &ReconciliationResultPreimageV1,
) -> (
    ReconciliationRequestV1,
    CommonRequestEnvelopeV1,
    RequestBindingV1,
) {
    let request = ReconciliationRequestV1::seal(ReconciliationRequestPreimageV1 {
        reconciliation_id: result.reconciliation_id.clone(),
        case_ref: result.case_ref.clone(),
        case_sha256: result.case_sha256.clone(),
        custody_ref: result.custody_ref.clone(),
        custody_sha256: result.custody_sha256.clone(),
        expected_custody_generation: result.custody_generation,
        expected_case_revision: predecessor.map_or(
            ReconciliationCaseRevisionV1::new(0).expect("valid initial expected revision"),
            |(_, value)| *value.case_revision(),
        ),
        predecessor_result_ref: predecessor.map(|(reference, _)| reference.clone()),
        predecessor_result_sha256: predecessor
            .map(|(_, value)| value.canonical_reconciliation_sha256().clone()),
        action_id: result.action_id.clone(),
        effect_admission_ref: result.effect_admission_ref.clone(),
        effect_admission_sha256: result.effect_admission_sha256.clone(),
        effect_receipts: result.effect_receipts.clone(),
        expected_state_digest: result.expected_state_digest.clone(),
        reconciliation_policy_ref: result.reconciliation_policy_ref.clone(),
        reconciliation_policy_sha256: result.reconciliation_policy_sha256.clone(),
        observation_bounds: serde_json::json!({"max_attempts": 1}),
        requested_at: result.requested_at,
    })
    .expect("result request body");
    let outer_preimage = common_request_preimage_for_reconciliation(&request);
    let outer = CommonRequestEnvelopeV1::seal(outer_preimage.clone(), signature())
        .expect("caller-signed result request");
    let binding = request_binding_for_common_request(&outer, &outer_preimage);
    (request, outer, binding)
}

#[test]
fn reconciliation_result_successor_rejects_every_immutable_substitution() {
    let predecessor_preimage = result_preimage(ReconciliationCaseStateV1::Drift, None);
    let predecessor = ReconciliationResultV1::seal(predecessor_preimage, signature())
        .expect("open predecessor result");
    let predecessor_ref = record_ref("result-revision-1");
    let mut successor = result_preimage(ReconciliationCaseStateV1::Drift, None);
    successor.case_revision =
        ReconciliationCaseRevisionV1::new(2).expect("valid successor revision");
    successor.predecessor_result_ref = Some(predecessor_ref.clone());
    successor.predecessor_result_sha256 =
        Some(predecessor.canonical_reconciliation_sha256().clone());
    successor.recorded_at = UnixTimeV1::new(21).expect("valid fixture time");
    let (request, outer_request, request_binding) =
        request_for_result(Some((&predecessor_ref, &predecessor)), &successor);
    successor.request_binding = request_binding.clone();

    ReconciliationResultV1::admit_successor_wire(
        result_wire(successor.clone()),
        &ReconciliationResultSuccessorAdmissionContextV1 {
            outer_request: &outer_request,
            request: &request,
            request_binding: &request_binding,
            predecessor_ref: &predecessor_ref,
            predecessor: &predecessor,
            compensation: None,
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: predecessor.schema_sha256(),
                signing_authority: &signing_authority(),
            },
        },
        |_| None,
        |_, _, _, _| Ok(()),
        |_, _, _| Ok(()),
    )
    .expect("exact successor admits");

    let mut cases = Vec::new();
    let mut changed = successor.clone();
    changed.case_revision = ReconciliationCaseRevisionV1::new(3).expect("valid skipped revision");
    cases.push(("revision jump", changed));
    let mut changed = successor.clone();
    changed.predecessor_result_ref = Some(record_ref("other-result"));
    cases.push(("predecessor ref", changed));
    let mut changed = successor.clone();
    changed.predecessor_result_sha256 =
        Some(ReconciliationResultDigest::parse(sha('0')).expect("other result digest"));
    cases.push(("predecessor digest", changed));
    let mut changed = successor.clone();
    changed.reconciliation_id =
        ReconciliationIdV1::parse("other-reconciliation").expect("reconciliation id");
    cases.push(("reconciliation id", changed));
    let mut changed = successor.clone();
    changed.case_ref = record_ref("other-case");
    cases.push(("case ref", changed));
    let mut changed = successor.clone();
    changed.case_sha256 = ReconciliationCaseDigest::parse(sha('0')).expect("other case digest");
    cases.push(("case digest", changed));
    let mut changed = successor.clone();
    changed.custody_ref = record_ref("other-custody");
    cases.push(("custody ref", changed));
    let mut changed = successor.clone();
    changed.custody_sha256 =
        ReconciliationCustodyDigest::parse(sha('0')).expect("other custody digest");
    cases.push(("custody digest", changed));
    let mut changed = successor.clone();
    changed.custody_generation =
        ReconciliationCustodyGenerationV1::new(2).expect("valid changed generation");
    cases.push(("custody generation", changed));
    let mut changed = successor.clone();
    changed.action_id = ActionIdV1::parse("other-action").expect("action id");
    cases.push(("action id", changed));
    let mut changed = successor.clone();
    changed.effect_admission_ref = record_ref("other-admission");
    cases.push(("effect admission ref", changed));
    let mut changed = successor.clone();
    changed.effect_admission_sha256 =
        EffectAdmissionDigest::parse(sha('0')).expect("other admission digest");
    cases.push(("effect admission digest", changed));
    let mut changed = successor.clone();
    changed.principal_scope = ResidentPrincipalIdV1::parse("other-principal").expect("principal");
    cases.push(("principal scope", changed));
    let mut changed = successor.clone();
    changed.controller_scope =
        ResidentControllerIdV1::parse("other-controller").expect("controller");
    cases.push(("controller scope", changed));
    let mut changed = successor.clone();
    changed.relationship_scope =
        Some(RelationshipScopeV1::parse("relationship").expect("relationship"));
    cases.push(("relationship scope", changed));
    let mut changed = successor.clone();
    changed.reconciliation_policy_ref = record_ref("other-policy");
    cases.push(("policy ref", changed));
    let mut changed = successor.clone();
    changed.reconciliation_policy_sha256 = artifact('0');
    cases.push(("policy digest", changed));
    let mut changed = successor.clone();
    changed.expected_state_digest = typed_state("other-expected");
    cases.push(("expected state", changed));
    let mut changed = successor.clone();
    changed.requested_at = UnixTimeV1::new(9).expect("valid fixture time");
    cases.push(("requested time regression", changed));
    let mut changed = successor.clone();
    changed.recorded_at = UnixTimeV1::new(19).expect("valid fixture time");
    cases.push(("recorded time regression", changed));
    let mut changed = successor.clone();
    changed.deployment_runtime_authority_ref = record_ref("other-authority");
    cases.push(("runtime authority ref", changed));
    let mut changed = successor.clone();
    changed.deployment_runtime_authority_sha256 = artifact('0');
    cases.push(("runtime authority digest", changed));

    for (name, candidate) in cases {
        assert!(
            ReconciliationResultV1::admit_successor_wire(
                result_wire(candidate),
                &ReconciliationResultSuccessorAdmissionContextV1 {
                    outer_request: &outer_request,
                    request: &request,
                    request_binding: &request_binding,
                    predecessor_ref: &predecessor_ref,
                    predecessor: &predecessor,
                    compensation: None,
                    signed: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: predecessor.schema_sha256(),
                        signing_authority: &signing_authority(),
                    },
                },
                |_| None,
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(()),
            )
            .is_err(),
            "{name} substitution must fail"
        );
    }

    let (wrong_cas_request, wrong_cas_outer, _) = request_for_result(None, &successor);
    assert!(ReconciliationResultV1::admit_successor_wire(
        result_wire(successor.clone()),
        &ReconciliationResultSuccessorAdmissionContextV1 {
            outer_request: &wrong_cas_outer,
            request: &wrong_cas_request,
            request_binding: &request_binding,
            predecessor_ref: &predecessor_ref,
            predecessor: &predecessor,
            compensation: None,
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: predecessor.schema_sha256(),
                signing_authority: &signing_authority(),
            },
        },
        |_| None,
        |_, _, _, _| Ok(()),
        |_, _, _| Ok(()),
    )
    .is_err());

    let terminal = ReconciliationResultV1::seal(
        result_preimage(
            ReconciliationCaseStateV1::Matched,
            Some(ReconciliationResolutionV1::EffectConfirmed),
        ),
        signature(),
    )
    .expect("terminal predecessor result");
    let terminal_ref = record_ref("terminal-result");
    let mut extension = successor;
    extension.predecessor_result_ref = Some(terminal_ref.clone());
    extension.predecessor_result_sha256 = Some(terminal.canonical_reconciliation_sha256().clone());
    extension.recorded_at = UnixTimeV1::new(31).expect("valid fixture time");
    let (terminal_request, terminal_outer_request, terminal_request_binding) =
        request_for_result(Some((&terminal_ref, &terminal)), &extension);
    extension.request_binding = terminal_request_binding.clone();
    assert!(ReconciliationResultV1::admit_successor_wire(
        result_wire(extension),
        &ReconciliationResultSuccessorAdmissionContextV1 {
            outer_request: &terminal_outer_request,
            request: &terminal_request,
            request_binding: &terminal_request_binding,
            predecessor_ref: &terminal_ref,
            predecessor: &terminal,
            compensation: None,
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: terminal.schema_sha256(),
                signing_authority: &signing_authority(),
            },
        },
        |_| None,
        |_, _, _, _| Ok(()),
        |_, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn reconciliation_result_rotation_requires_admitted_deployment_continuity() {
    let predecessor = ReconciliationResultV1::seal(
        result_preimage(ReconciliationCaseStateV1::Drift, None),
        signature(),
    )
    .expect("open predecessor result");
    let predecessor_ref = record_ref("result-revision-1");
    let mut successor = result_preimage(ReconciliationCaseStateV1::Drift, None);
    successor.case_revision = ReconciliationCaseRevisionV1::new(2).expect("successor revision");
    successor.predecessor_result_ref = Some(predecessor_ref.clone());
    successor.predecessor_result_sha256 =
        Some(predecessor.canonical_reconciliation_sha256().clone());
    successor.recorded_at = UnixTimeV1::new(21).expect("successor record time");
    successor.deployment_runtime_authority_ref = record_ref("authority-rotated");
    successor.deployment_runtime_authority_sha256 = artifact('0');
    let (request, outer_request, request_binding) =
        request_for_result(Some((&predecessor_ref, &predecessor)), &successor);
    successor.request_binding = request_binding.clone();

    let foreign_authority =
        signing_authority_at("authority-rotated", '0', "foreign-deployment", 'f');
    let signature_checked = std::cell::Cell::new(false);
    assert!(ReconciliationResultV1::admit_successor_wire(
        result_wire(successor.clone()),
        &ReconciliationResultSuccessorAdmissionContextV1 {
            outer_request: &outer_request,
            request: &request,
            request_binding: &request_binding,
            predecessor_ref: &predecessor_ref,
            predecessor: &predecessor,
            compensation: None,
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: predecessor.schema_sha256(),
                signing_authority: &foreign_authority,
            },
        },
        |_| None,
        |_, _, _, _| Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "reconciliation result",
            invariant: "same deployment runtime-authority lineage",
        }),
        |_, _, _| {
            signature_checked.set(true);
            Ok(())
        },
    )
    .is_err());
    assert!(!signature_checked.get());

    let same_deployment_authority =
        signing_authority_at("authority-rotated", '0', "deployment", 'b');
    let rotation_checked = std::cell::Cell::new(false);
    ReconciliationResultV1::admit_successor_wire(
        result_wire(successor),
        &ReconciliationResultSuccessorAdmissionContextV1 {
            outer_request: &outer_request,
            request: &request,
            request_binding: &request_binding,
            predecessor_ref: &predecessor_ref,
            predecessor: &predecessor,
            compensation: None,
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: predecessor.schema_sha256(),
                signing_authority: &same_deployment_authority,
            },
        },
        |_| None,
        |from_ref, from_sha256, to_ref, to_sha256| {
            assert_eq!(from_ref, predecessor.deployment_runtime_authority_ref());
            assert_eq!(
                from_sha256,
                predecessor.deployment_runtime_authority_sha256()
            );
            assert_eq!(to_ref.as_str(), "authority-rotated");
            assert_eq!(to_sha256, &artifact('0'));
            rotation_checked.set(true);
            Ok(())
        },
        |_, _, _| Ok(()),
    )
    .expect("admitted same-deployment result rotation");
    assert!(rotation_checked.get());
}

#[test]
fn reconciliation_custody_successor_rejects_generation_and_immutable_substitutions() {
    let authority = signing_authority();
    let predecessor = ReconciliationCustodyV1::seal(
        custody_preimage(ReconciliationCustodyStateV1::Active),
        signature(),
    )
    .expect("active predecessor custody");
    let predecessor_ref = record_ref("custody-generation-1");
    let mut successor = custody_preimage(ReconciliationCustodyStateV1::Transferred);
    successor.holder_principal_id =
        ResidentPrincipalIdV1::parse("principal-2").expect("successor holder");
    successor.previous_custody_ref = Some(predecessor_ref.clone());
    successor.previous_custody_sha256 = Some(predecessor.canonical_custody_sha256().clone());

    ReconciliationCustodyV1::admit_successor_wire(
        custody_wire(successor.clone()),
        &ReconciliationCustodyTransitionContextV1::Transfer {
            predecessor: ReconciliationCustodyPredecessorV1 {
                predecessor_ref: &predecessor_ref,
                predecessor: &predecessor,
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: predecessor.schema_sha256(),
                    signing_authority: &authority,
                },
            },
        },
        |_, _, _, _| Ok(()),
        |_, _, _| Ok(()),
    )
    .expect("exact custody successor admits");

    let mut cases = Vec::new();
    let mut changed = successor.clone();
    changed.custody_generation =
        ReconciliationCustodyGenerationV1::new(3).expect("valid skipped generation");
    cases.push(("generation jump", changed));
    let mut changed = successor.clone();
    changed.previous_custody_ref = Some(record_ref("other-custody"));
    cases.push(("predecessor ref", changed));
    let mut changed = successor.clone();
    changed.previous_custody_sha256 =
        Some(ReconciliationCustodyDigest::parse(sha('0')).expect("other custody digest"));
    cases.push(("predecessor digest", changed));
    let mut changed = successor.clone();
    changed.custody_id = ReconciliationCustodyIdV1::parse("other-custody-id").expect("custody id");
    cases.push(("custody id", changed));
    let mut changed = successor.clone();
    changed.case_ref = record_ref("other-case");
    cases.push(("case ref", changed));
    let mut changed = successor.clone();
    changed.case_sha256 = ReconciliationCaseDigest::parse(sha('0')).expect("other case digest");
    cases.push(("case digest", changed));
    let mut changed = successor.clone();
    changed.conflict_resource_key_schema_id =
        SchemaId::parse("example/other-key.v1").expect("key schema");
    cases.push(("key schema", changed));
    let mut changed = successor.clone();
    changed.conflict_resource_key_schema_sha256 =
        SchemaDigest::parse(sha('0')).expect("key schema digest");
    cases.push(("key schema digest", changed));
    let mut changed = successor.clone();
    changed.conflict_resource_key = serde_json::json!({"resource":"other"});
    cases.push(("key value", changed));
    let mut changed = successor.clone();
    changed.holder_principal_id = predecessor.holder_principal_id().clone();
    changed.holder_node_id = predecessor.holder_node_id().clone();
    cases.push(("same holder transfer", changed));
    let mut changed = successor.clone();
    changed.deployment_runtime_authority_ref = record_ref("other-authority");
    cases.push(("runtime authority ref", changed));
    let mut changed = successor.clone();
    changed.deployment_runtime_authority_sha256 = artifact('0');
    cases.push(("runtime authority digest", changed));

    for (name, candidate) in cases {
        assert!(
            ReconciliationCustodyV1::admit_successor_wire(
                custody_wire(candidate),
                &ReconciliationCustodyTransitionContextV1::Transfer {
                    predecessor: ReconciliationCustodyPredecessorV1 {
                        predecessor_ref: &predecessor_ref,
                        predecessor: &predecessor,
                        signed: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: predecessor.schema_sha256(),
                            signing_authority: &authority,
                        },
                    },
                },
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(()),
            )
            .is_err(),
            "{name} substitution must fail"
        );
    }

    let terminal_ref = record_ref("custody-terminal-result");
    let mut terminal_preimage = result_preimage(
        ReconciliationCaseStateV1::Matched,
        Some(ReconciliationResolutionV1::EffectConfirmed),
    );
    terminal_preimage.case_ref = predecessor.case_ref().clone();
    terminal_preimage.case_sha256 = predecessor.case_sha256().clone();
    terminal_preimage.custody_ref = predecessor_ref.clone();
    terminal_preimage.custody_sha256 = predecessor.canonical_custody_sha256().clone();
    terminal_preimage.custody_generation = *predecessor.custody_generation();
    let terminal = ReconciliationResultV1::seal(terminal_preimage.clone(), signature())
        .expect("terminal result for current custody");
    let mut closed_preimage = custody_preimage(ReconciliationCustodyStateV1::Closed);
    closed_preimage.previous_custody_ref = Some(predecessor_ref.clone());
    closed_preimage.previous_custody_sha256 = Some(predecessor.canonical_custody_sha256().clone());
    closed_preimage.terminal_result_ref = Some(terminal_ref.clone());
    closed_preimage.terminal_result_sha256 =
        Some(terminal.canonical_reconciliation_sha256().clone());
    closed_preimage.transferred_or_closed_at =
        Some(UnixTimeV1::new(31).expect("valid fixture time"));
    ReconciliationCustodyV1::admit_successor_wire(
        custody_wire(closed_preimage.clone()),
        &ReconciliationCustodyTransitionContextV1::Close {
            predecessor: ReconciliationCustodyPredecessorV1 {
                predecessor_ref: &predecessor_ref,
                predecessor: &predecessor,
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: predecessor.schema_sha256(),
                    signing_authority: &authority,
                },
            },
            terminal_result_ref: &terminal_ref,
            terminal_result: &terminal,
        },
        |_, _, _, _| Ok(()),
        |_, _, _| Ok(()),
    )
    .expect("exact terminal result closes current custody");

    terminal_preimage.custody_ref = record_ref("other-custody");
    let fake_terminal = ReconciliationResultV1::seal(terminal_preimage, signature())
        .expect("structurally valid fake terminal");
    assert!(ReconciliationCustodyV1::admit_successor_wire(
        custody_wire(closed_preimage),
        &ReconciliationCustodyTransitionContextV1::Close {
            predecessor: ReconciliationCustodyPredecessorV1 {
                predecessor_ref: &predecessor_ref,
                predecessor: &predecessor,
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: predecessor.schema_sha256(),
                    signing_authority: &authority,
                },
            },
            terminal_result_ref: &terminal_ref,
            terminal_result: &fake_terminal,
        },
        |_, _, _, _| Ok(()),
        |_, _, _| Ok(()),
    )
    .is_err());

    let mut transferred_preimage = successor;
    transferred_preimage.custody_generation =
        ReconciliationCustodyGenerationV1::new(2).expect("valid transfer generation");
    let transferred = ReconciliationCustodyV1::seal(transferred_preimage, signature())
        .expect("transferred predecessor");
    let transferred_ref = record_ref("custody-generation-2");
    let mut regressed = custody_preimage(ReconciliationCustodyStateV1::Transferred);
    regressed.custody_generation =
        ReconciliationCustodyGenerationV1::new(3).expect("valid successor generation");
    regressed.previous_custody_ref = Some(transferred_ref.clone());
    regressed.previous_custody_sha256 = Some(transferred.canonical_custody_sha256().clone());
    regressed.transferred_or_closed_at = Some(UnixTimeV1::new(19).expect("valid fixture time"));
    assert!(ReconciliationCustodyV1::admit_successor_wire(
        custody_wire(regressed),
        &ReconciliationCustodyTransitionContextV1::Transfer {
            predecessor: ReconciliationCustodyPredecessorV1 {
                predecessor_ref: &transferred_ref,
                predecessor: &transferred,
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: transferred.schema_sha256(),
                    signing_authority: &authority,
                },
            },
        },
        |_, _, _, _| Ok(()),
        |_, _, _| Ok(()),
    )
    .is_err());

    let mut closed_preimage = custody_preimage(ReconciliationCustodyStateV1::Closed);
    closed_preimage.custody_generation =
        ReconciliationCustodyGenerationV1::new(3).expect("valid closed generation");
    closed_preimage.previous_custody_ref = Some(transferred_ref.clone());
    closed_preimage.previous_custody_sha256 = Some(transferred.canonical_custody_sha256().clone());
    closed_preimage.transferred_or_closed_at =
        Some(UnixTimeV1::new(21).expect("valid fixture time"));
    let closed = ReconciliationCustodyV1::seal(closed_preimage, signature())
        .expect("closed custody generation");
    let closed_ref = record_ref("custody-generation-3");
    let mut after_closed = custody_preimage(ReconciliationCustodyStateV1::Transferred);
    after_closed.custody_generation =
        ReconciliationCustodyGenerationV1::new(4).expect("valid post-close generation");
    after_closed.previous_custody_ref = Some(closed_ref.clone());
    after_closed.previous_custody_sha256 = Some(closed.canonical_custody_sha256().clone());
    after_closed.transferred_or_closed_at = Some(UnixTimeV1::new(22).expect("valid fixture time"));
    assert!(ReconciliationCustodyV1::admit_successor_wire(
        custody_wire(after_closed),
        &ReconciliationCustodyTransitionContextV1::Transfer {
            predecessor: ReconciliationCustodyPredecessorV1 {
                predecessor_ref: &closed_ref,
                predecessor: &closed,
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: closed.schema_sha256(),
                    signing_authority: &authority,
                },
            },
        },
        |_, _, _, _| Ok(()),
        |_, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn reconciliation_custody_rotation_requires_admitted_deployment_continuity() {
    let predecessor = ReconciliationCustodyV1::seal(
        custody_preimage(ReconciliationCustodyStateV1::Active),
        signature(),
    )
    .expect("active predecessor custody");
    let predecessor_ref = record_ref("custody-generation-1");
    let mut successor = custody_preimage(ReconciliationCustodyStateV1::Transferred);
    successor.holder_principal_id =
        ResidentPrincipalIdV1::parse("principal-2").expect("successor holder");
    successor.previous_custody_ref = Some(predecessor_ref.clone());
    successor.previous_custody_sha256 = Some(predecessor.canonical_custody_sha256().clone());
    successor.deployment_runtime_authority_ref = record_ref("authority-rotated");
    successor.deployment_runtime_authority_sha256 = artifact('0');

    let foreign_authority =
        signing_authority_at("authority-rotated", '0', "foreign-deployment", 'f');
    let signature_checked = std::cell::Cell::new(false);
    assert!(ReconciliationCustodyV1::admit_successor_wire(
        custody_wire(successor.clone()),
        &ReconciliationCustodyTransitionContextV1::Transfer {
            predecessor: ReconciliationCustodyPredecessorV1 {
                predecessor_ref: &predecessor_ref,
                predecessor: &predecessor,
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: predecessor.schema_sha256(),
                    signing_authority: &foreign_authority,
                },
            },
        },
        |_, _, _, _| Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "reconciliation custody",
            invariant: "same deployment runtime-authority lineage",
        }),
        |_, _, _| {
            signature_checked.set(true);
            Ok(())
        },
    )
    .is_err());
    assert!(!signature_checked.get());

    let same_deployment_authority =
        signing_authority_at("authority-rotated", '0', "deployment", 'b');
    let rotation_checked = std::cell::Cell::new(false);
    ReconciliationCustodyV1::admit_successor_wire(
        custody_wire(successor),
        &ReconciliationCustodyTransitionContextV1::Transfer {
            predecessor: ReconciliationCustodyPredecessorV1 {
                predecessor_ref: &predecessor_ref,
                predecessor: &predecessor,
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: predecessor.schema_sha256(),
                    signing_authority: &same_deployment_authority,
                },
            },
        },
        |from_ref, from_sha256, to_ref, to_sha256| {
            assert_eq!(from_ref, predecessor.deployment_runtime_authority_ref());
            assert_eq!(
                from_sha256,
                predecessor.deployment_runtime_authority_sha256()
            );
            assert_eq!(to_ref.as_str(), "authority-rotated");
            assert_eq!(to_sha256, &artifact('0'));
            rotation_checked.set(true);
            Ok(())
        },
        |_, _, _| Ok(()),
    )
    .expect("admitted same-deployment custody rotation");
    assert!(rotation_checked.get());
}

#[test]
fn reopened_case_rejects_every_predecessor_and_immutable_substitution() {
    let predecessor_case = ReconciliationCaseV1::seal(
        case_preimage(ReconciliationCaseOriginV1::Initial),
        signature(),
    )
    .expect("initial case");
    let predecessor_case_ref = record_ref("case");
    let mut terminal_preimage = result_preimage(
        ReconciliationCaseStateV1::Matched,
        Some(ReconciliationResolutionV1::EffectConfirmed),
    );
    terminal_preimage.case_sha256 = predecessor_case.canonical_case_sha256().clone();
    let terminal = ReconciliationResultV1::seal(terminal_preimage, signature())
        .expect("terminal predecessor result");
    let terminal_ref = record_ref("terminal-result");
    let mut reopened = case_preimage(ReconciliationCaseOriginV1::Reopened);
    reopened.case_id = ReconciliationCaseIdV1::parse("case-2").expect("new case id");
    reopened.reserved_custody_id =
        ReconciliationCustodyIdV1::parse("custody-2").expect("new custody id");
    reopened.predecessor_case_ref = Some(predecessor_case_ref.clone());
    reopened.predecessor_case_sha256 = Some(predecessor_case.canonical_case_sha256().clone());
    reopened.predecessor_terminal_result_ref = Some(terminal_ref.clone());
    reopened.predecessor_terminal_result_sha256 =
        Some(terminal.canonical_reconciliation_sha256().clone());
    reopened.opened_at = UnixTimeV1::new(31).expect("valid fixture time");

    ReconciliationCaseV1::admit_reopened_wire(
        case_wire(reopened.clone()),
        &ReconciliationCaseReopenAdmissionContextV1 {
            predecessor_case_ref: &predecessor_case_ref,
            predecessor_case: &predecessor_case,
            predecessor_terminal_result_ref: &terminal_ref,
            predecessor_terminal_result: &terminal,
            predecessor_evidence_closure: predecessor_case.opening_evidence(),
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: predecessor_case.schema_sha256(),
                signing_authority: &signing_authority(),
            },
        },
        |_| None,
        |_, _, _, _| Ok(()),
        |_, _, _| Ok(()),
    )
    .expect("exact reopened case admits");

    assert!(ReconciliationCaseV1::admit_reopened_wire(
        case_wire(reopened.clone()),
        &ReconciliationCaseReopenAdmissionContextV1 {
            predecessor_case_ref: &predecessor_case_ref,
            predecessor_case: &predecessor_case,
            predecessor_terminal_result_ref: &terminal_ref,
            predecessor_terminal_result: &terminal,
            predecessor_evidence_closure: &[],
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: predecessor_case.schema_sha256(),
                signing_authority: &signing_authority(),
            },
        },
        |_| None,
        |_, _, _, _| Ok(()),
        |_, _, _| Ok(()),
    )
    .is_err());

    let mut cases = Vec::new();
    let mut changed = reopened.clone();
    changed.predecessor_case_ref = Some(record_ref("other-case"));
    cases.push(("predecessor case ref", changed));
    let mut changed = reopened.clone();
    changed.predecessor_case_sha256 =
        Some(ReconciliationCaseDigest::parse(sha('0')).expect("case digest"));
    cases.push(("predecessor case digest", changed));
    let mut changed = reopened.clone();
    changed.predecessor_terminal_result_ref = Some(record_ref("other-result"));
    cases.push(("terminal result ref", changed));
    let mut changed = reopened.clone();
    changed.predecessor_terminal_result_sha256 =
        Some(ReconciliationResultDigest::parse(sha('0')).expect("result digest"));
    cases.push(("terminal result digest", changed));
    let mut changed = reopened.clone();
    changed.case_id = predecessor_case.case_id().clone();
    cases.push(("reused case id", changed));
    let mut changed = reopened.clone();
    changed.reserved_custody_id = predecessor_case.reserved_custody_id().clone();
    cases.push(("reused custody id", changed));
    let mut changed = reopened.clone();
    changed.action_id = ActionIdV1::parse("other-action").expect("action id");
    cases.push(("action id", changed));
    let mut changed = reopened.clone();
    changed.effect_admission_ref = record_ref("other-admission");
    cases.push(("effect admission ref", changed));
    let mut changed = reopened.clone();
    changed.effect_admission_sha256 =
        EffectAdmissionDigest::parse(sha('0')).expect("admission digest");
    cases.push(("effect admission digest", changed));
    let mut changed = reopened.clone();
    changed.conflict_resource_key_schema_id =
        SchemaId::parse("example/other-key.v1").expect("key schema");
    cases.push(("key schema", changed));
    let mut changed = reopened.clone();
    changed.conflict_resource_key_schema_sha256 =
        SchemaDigest::parse(sha('0')).expect("key schema digest");
    cases.push(("key schema digest", changed));
    let mut changed = reopened.clone();
    changed.conflict_resource_key = serde_json::json!({"resource":"other"});
    cases.push(("key value", changed));
    let mut changed = reopened.clone();
    changed.expected_state_digest = typed_state("other-expected");
    cases.push(("expected state", changed));
    let mut changed = reopened.clone();
    changed.reconciliation_policy_ref = record_ref("other-policy");
    cases.push(("policy ref", changed));
    let mut changed = reopened.clone();
    changed.reconciliation_policy_sha256 = artifact('0');
    cases.push(("policy digest", changed));
    let mut changed = reopened.clone();
    changed.opened_at = UnixTimeV1::new(29).expect("valid fixture time");
    cases.push(("opening time regression", changed));
    let mut changed = reopened.clone();
    changed.deployment_runtime_authority_ref = record_ref("other-authority");
    cases.push(("runtime authority ref", changed));
    let mut changed = reopened.clone();
    changed.deployment_runtime_authority_sha256 = artifact('0');
    cases.push(("runtime authority digest", changed));

    for (name, candidate) in cases {
        assert!(
            ReconciliationCaseV1::admit_reopened_wire(
                case_wire(candidate),
                &ReconciliationCaseReopenAdmissionContextV1 {
                    predecessor_case_ref: &predecessor_case_ref,
                    predecessor_case: &predecessor_case,
                    predecessor_terminal_result_ref: &terminal_ref,
                    predecessor_terminal_result: &terminal,
                    predecessor_evidence_closure: predecessor_case.opening_evidence(),
                    signed: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: predecessor_case.schema_sha256(),
                        signing_authority: &signing_authority(),
                    },
                },
                |_| None,
                |_, _, _, _| Ok(()),
                |_, _, _| Ok(()),
            )
            .is_err(),
            "{name} substitution must fail"
        );
    }

    let mut all_evidence_preimage = case_preimage(ReconciliationCaseOriginV1::Initial);
    all_evidence_preimage
        .opening_evidence
        .push(observation_opening());
    let all_evidence_predecessor = ReconciliationCaseV1::seal(all_evidence_preimage, signature())
        .expect("initial case with observation evidence");
    let all_evidence_ref = record_ref("all-evidence-case");
    let mut all_evidence_terminal_preimage = result_preimage(
        ReconciliationCaseStateV1::Matched,
        Some(ReconciliationResolutionV1::EffectConfirmed),
    );
    all_evidence_terminal_preimage.case_ref = all_evidence_ref.clone();
    all_evidence_terminal_preimage.case_sha256 =
        all_evidence_predecessor.canonical_case_sha256().clone();
    let all_evidence_terminal =
        ReconciliationResultV1::seal(all_evidence_terminal_preimage, signature())
            .expect("terminal result for evidence predecessor");
    let all_evidence_terminal_ref = record_ref("all-evidence-terminal");
    let mut no_new_evidence = case_preimage(ReconciliationCaseOriginV1::Reopened);
    no_new_evidence.case_id =
        ReconciliationCaseIdV1::parse("case-without-new-evidence").expect("case id");
    no_new_evidence.reserved_custody_id =
        ReconciliationCustodyIdV1::parse("custody-without-new-evidence").expect("custody id");
    no_new_evidence.opening_evidence = all_evidence_predecessor.opening_evidence().to_vec();
    no_new_evidence.predecessor_case_ref = Some(all_evidence_ref.clone());
    no_new_evidence.predecessor_case_sha256 =
        Some(all_evidence_predecessor.canonical_case_sha256().clone());
    no_new_evidence.predecessor_terminal_result_ref = Some(all_evidence_terminal_ref.clone());
    no_new_evidence.predecessor_terminal_result_sha256 = Some(
        all_evidence_terminal
            .canonical_reconciliation_sha256()
            .clone(),
    );
    no_new_evidence.opened_at = UnixTimeV1::new(31).expect("valid fixture time");
    assert!(ReconciliationCaseV1::admit_reopened_wire(
        case_wire(no_new_evidence),
        &ReconciliationCaseReopenAdmissionContextV1 {
            predecessor_case_ref: &all_evidence_ref,
            predecessor_case: &all_evidence_predecessor,
            predecessor_terminal_result_ref: &all_evidence_terminal_ref,
            predecessor_terminal_result: &all_evidence_terminal,
            predecessor_evidence_closure: all_evidence_predecessor.opening_evidence(),
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: all_evidence_predecessor.schema_sha256(),
                signing_authority: &signing_authority(),
            },
        },
        |_| None,
        |_, _, _, _| Ok(()),
        |_, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn reopened_case_rotation_requires_admitted_deployment_continuity() {
    let predecessor_case = ReconciliationCaseV1::seal(
        case_preimage(ReconciliationCaseOriginV1::Initial),
        signature(),
    )
    .expect("initial case");
    let predecessor_case_ref = record_ref("case");
    let mut terminal_preimage = result_preimage(
        ReconciliationCaseStateV1::Matched,
        Some(ReconciliationResolutionV1::EffectConfirmed),
    );
    terminal_preimage.case_ref = predecessor_case_ref.clone();
    terminal_preimage.case_sha256 = predecessor_case.canonical_case_sha256().clone();
    let terminal = ReconciliationResultV1::seal(terminal_preimage, signature())
        .expect("terminal predecessor result");
    let terminal_ref = record_ref("terminal-result");
    let mut reopened = case_preimage(ReconciliationCaseOriginV1::Reopened);
    reopened.case_id = ReconciliationCaseIdV1::parse("case-2").expect("new case id");
    reopened.reserved_custody_id =
        ReconciliationCustodyIdV1::parse("custody-2").expect("new custody id");
    reopened.predecessor_case_ref = Some(predecessor_case_ref.clone());
    reopened.predecessor_case_sha256 = Some(predecessor_case.canonical_case_sha256().clone());
    reopened.predecessor_terminal_result_ref = Some(terminal_ref.clone());
    reopened.predecessor_terminal_result_sha256 =
        Some(terminal.canonical_reconciliation_sha256().clone());
    reopened.opened_at = UnixTimeV1::new(31).expect("reopen time");
    reopened.deployment_runtime_authority_ref = record_ref("authority-rotated");
    reopened.deployment_runtime_authority_sha256 = artifact('0');

    let foreign_authority =
        signing_authority_at("authority-rotated", '0', "foreign-deployment", 'f');
    let signature_checked = std::cell::Cell::new(false);
    assert!(ReconciliationCaseV1::admit_reopened_wire(
        case_wire(reopened.clone()),
        &ReconciliationCaseReopenAdmissionContextV1 {
            predecessor_case_ref: &predecessor_case_ref,
            predecessor_case: &predecessor_case,
            predecessor_terminal_result_ref: &terminal_ref,
            predecessor_terminal_result: &terminal,
            predecessor_evidence_closure: predecessor_case.opening_evidence(),
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: predecessor_case.schema_sha256(),
                signing_authority: &foreign_authority,
            },
        },
        |_| None,
        |_, _, _, _| Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "reconciliation case",
            invariant: "same deployment runtime-authority lineage",
        }),
        |_, _, _| {
            signature_checked.set(true);
            Ok(())
        },
    )
    .is_err());
    assert!(!signature_checked.get());

    let same_deployment_authority =
        signing_authority_at("authority-rotated", '0', "deployment", 'b');
    let rotation_checked = std::cell::Cell::new(false);
    ReconciliationCaseV1::admit_reopened_wire(
        case_wire(reopened),
        &ReconciliationCaseReopenAdmissionContextV1 {
            predecessor_case_ref: &predecessor_case_ref,
            predecessor_case: &predecessor_case,
            predecessor_terminal_result_ref: &terminal_ref,
            predecessor_terminal_result: &terminal,
            predecessor_evidence_closure: predecessor_case.opening_evidence(),
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: predecessor_case.schema_sha256(),
                signing_authority: &same_deployment_authority,
            },
        },
        |_| None,
        |from_ref, from_sha256, to_ref, to_sha256| {
            assert_eq!(
                from_ref,
                predecessor_case.deployment_runtime_authority_ref()
            );
            assert_eq!(
                from_sha256,
                predecessor_case.deployment_runtime_authority_sha256()
            );
            assert_eq!(to_ref.as_str(), "authority-rotated");
            assert_eq!(to_sha256, &artifact('0'));
            rotation_checked.set(true);
            Ok(())
        },
        |_, _, _| Ok(()),
    )
    .expect("admitted same-deployment case rotation");
    assert!(rotation_checked.get());
}

#[test]
fn reconciliation_request_is_an_unsigned_common_request_body() {
    let case = ReconciliationCaseV1::seal(
        case_preimage(ReconciliationCaseOriginV1::Initial),
        signature(),
    )
    .expect("initial case");
    let case_ref = record_ref("case");
    let mut custody_preimage = custody_preimage(ReconciliationCustodyStateV1::Active);
    custody_preimage.case_sha256 = case.canonical_case_sha256().clone();
    let custody =
        ReconciliationCustodyV1::seal(custody_preimage, signature()).expect("active custody");
    let custody_ref = record_ref("custody");
    let request_preimage = ReconciliationRequestPreimageV1 {
        reconciliation_id: ReconciliationIdV1::parse("reconciliation-1")
            .expect("reconciliation id"),
        case_ref: case_ref.clone(),
        case_sha256: case.canonical_case_sha256().clone(),
        custody_ref: custody_ref.clone(),
        custody_sha256: custody.canonical_custody_sha256().clone(),
        expected_custody_generation: ReconciliationCustodyGenerationV1::new(1)
            .expect("valid fixture expected custody"),
        expected_case_revision: ReconciliationCaseRevisionV1::new(0)
            .expect("valid fixture expected revision"),
        predecessor_result_ref: None,
        predecessor_result_sha256: None,
        action_id: case.action_id().clone(),
        effect_admission_ref: case.effect_admission_ref().clone(),
        effect_admission_sha256: case.effect_admission_sha256().clone(),
        effect_receipts: vec![ReconciliationEffectReceiptRefV1 {
            receipt_ref: record_ref("effect-receipt"),
            receipt_sha256: EffectReceiptDigest::parse(sha('1')).expect("receipt digest"),
        }],
        expected_state_digest: case.expected_state_digest().clone(),
        reconciliation_policy_ref: case.reconciliation_policy_ref().clone(),
        reconciliation_policy_sha256: case.reconciliation_policy_sha256().clone(),
        observation_bounds: serde_json::json!({"max_attempts": 1}),
        requested_at: UnixTimeV1::new(11).expect("valid fixture time"),
    };
    let request =
        ReconciliationRequestV1::seal(request_preimage.clone()).expect("unsigned request body");
    let encoded = serde_json::to_value(&request).expect("request serializes");
    assert!(encoded.get("signature").is_none());
    assert!(encoded
        .get("canonical_reconciliation_request_sha256")
        .is_none());
    assert!(encoded.get("schema_id").is_none());
    assert_eq!(
        request
            .canonical_reconciliation_request_sha256()
            .expect("body digest"),
        ReconciliationRequestDigest::hash_canonical(&request).expect("canonical body digest")
    );
    let outer_request = common_request_for_reconciliation(&request);
    let mut orphan_revocation = common_request_preimage_for_reconciliation(&request);
    orphan_revocation.session_admission.checked_revocation_ref =
        Some(record_ref("revocation-without-hash"));
    assert!(CommonRequestEnvelopeV1::seal(orphan_revocation, signature()).is_err());
    let wire: ReconciliationRequestWireV1 =
        serde_json::from_value(encoded).expect("structural request wire");
    ReconciliationRequestV1::admit_contextual_wire(
        wire,
        &ReconciliationRequestAdmissionContextV1 {
            outer_request: &outer_request,
            case_ref: &case_ref,
            case: &case,
            custody_ref: &custody_ref,
            custody: &custody,
            predecessor: None,
            intervening_custody: &[],
        },
        |_| None,
    )
    .expect("caller-signed body with exact case and custody admits");

    let invented_predecessor_path = [ReconciliationCustodyLineageMemberV1 {
        custody_ref: &custody_ref,
        custody: &custody,
    }];
    assert!(ReconciliationRequestV1::admit_contextual_wire(
        serde_json::from_value(serde_json::to_value(&request).expect("request serializes"))
            .expect("structural request wire"),
        &ReconciliationRequestAdmissionContextV1 {
            outer_request: &outer_request,
            case_ref: &case_ref,
            case: &case,
            custody_ref: &custody_ref,
            custody: &custody,
            predecessor: None,
            intervening_custody: &invented_predecessor_path,
        },
        |_| None,
    )
    .is_err());

    let mut wrong_generation = request_preimage;
    wrong_generation.expected_custody_generation =
        ReconciliationCustodyGenerationV1::new(2).expect("valid wrong generation");
    let wrong = ReconciliationRequestV1::seal(wrong_generation)
        .expect("wrong generation remains a structural body");
    let wrong_outer = common_request_for_reconciliation(&wrong);
    let wrong_wire: ReconciliationRequestWireV1 =
        serde_json::from_value(serde_json::to_value(wrong).expect("wrong request serializes"))
            .expect("wrong structural request wire");
    assert!(ReconciliationRequestV1::admit_contextual_wire(
        wrong_wire,
        &ReconciliationRequestAdmissionContextV1 {
            outer_request: &wrong_outer,
            case_ref: &case_ref,
            case: &case,
            custody_ref: &custody_ref,
            custody: &custody,
            predecessor: None,
            intervening_custody: &[],
        },
        |_| None,
    )
    .is_err());
}

#[test]
fn reconciliation_result_chain_survives_an_admitted_custody_transfer() {
    let authority = signing_authority();
    let case = ReconciliationCaseV1::seal(
        case_preimage(ReconciliationCaseOriginV1::Initial),
        signature(),
    )
    .expect("initial case");
    let case_ref = record_ref("case");

    let mut initial_custody_preimage = custody_preimage(ReconciliationCustodyStateV1::Active);
    initial_custody_preimage.case_sha256 = case.canonical_case_sha256().clone();
    let initial_custody = ReconciliationCustodyV1::seal(initial_custody_preimage, signature())
        .expect("initial custody");
    let initial_custody_ref = record_ref("custody-generation-1");

    let mut predecessor_preimage = result_preimage(ReconciliationCaseStateV1::Drift, None);
    predecessor_preimage.case_ref = case_ref.clone();
    predecessor_preimage.case_sha256 = case.canonical_case_sha256().clone();
    predecessor_preimage.custody_ref = initial_custody_ref.clone();
    predecessor_preimage.custody_sha256 = initial_custody.canonical_custody_sha256().clone();
    predecessor_preimage.custody_generation = *initial_custody.custody_generation();
    let predecessor = ReconciliationResultV1::seal(predecessor_preimage.clone(), signature())
        .expect("open predecessor result");
    let predecessor_ref = record_ref("result-revision-1");

    let mut transferred_preimage = custody_preimage(ReconciliationCustodyStateV1::Transferred);
    transferred_preimage.case_ref = case_ref.clone();
    transferred_preimage.case_sha256 = case.canonical_case_sha256().clone();
    transferred_preimage.previous_custody_ref = Some(initial_custody_ref.clone());
    transferred_preimage.previous_custody_sha256 =
        Some(initial_custody.canonical_custody_sha256().clone());
    transferred_preimage.holder_principal_id =
        ResidentPrincipalIdV1::parse("principal-2").expect("new holder");
    let transferred = ReconciliationCustodyV1::admit_successor_wire(
        custody_wire(transferred_preimage),
        &ReconciliationCustodyTransitionContextV1::Transfer {
            predecessor: ReconciliationCustodyPredecessorV1 {
                predecessor_ref: &initial_custody_ref,
                predecessor: &initial_custody,
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: initial_custody.schema_sha256(),
                    signing_authority: &authority,
                },
            },
        },
        |_, _, _, _| Ok(()),
        |_, _, _| Ok(()),
    )
    .expect("admitted custody transfer");
    let transferred_ref = record_ref("custody-generation-2");

    let mut successor_preimage = predecessor_preimage.clone();
    successor_preimage.custody_ref = transferred_ref.clone();
    successor_preimage.custody_sha256 = transferred.canonical_custody_sha256().clone();
    successor_preimage.custody_generation = *transferred.custody_generation();
    successor_preimage.case_revision =
        ReconciliationCaseRevisionV1::new(2).expect("successor revision");
    successor_preimage.predecessor_result_ref = Some(predecessor_ref.clone());
    successor_preimage.predecessor_result_sha256 =
        Some(predecessor.canonical_reconciliation_sha256().clone());
    successor_preimage.requested_at = UnixTimeV1::new(21).expect("request time");
    successor_preimage.recorded_at = UnixTimeV1::new(22).expect("record time");
    let (request, outer_request, request_binding) =
        request_for_result(Some((&predecessor_ref, &predecessor)), &successor_preimage);
    let request_wire =
        serde_json::from_value(serde_json::to_value(&request).expect("request serializes"))
            .expect("request wire decodes");
    let request = ReconciliationRequestV1::admit_contextual_wire(
        request_wire,
        &ReconciliationRequestAdmissionContextV1 {
            outer_request: &outer_request,
            case_ref: &case_ref,
            case: &case,
            custody_ref: &transferred_ref,
            custody: &transferred,
            predecessor: Some((&predecessor_ref, &predecessor)),
            intervening_custody: &[],
        },
        |_| None,
    )
    .expect("request follows transferred custody");
    successor_preimage.request_binding = request_binding.clone();

    ReconciliationResultV1::admit_successor_wire(
        result_wire(successor_preimage),
        &ReconciliationResultSuccessorAdmissionContextV1 {
            outer_request: &outer_request,
            request: &request,
            request_binding: &request_binding,
            predecessor_ref: &predecessor_ref,
            predecessor: &predecessor,
            compensation: None,
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: predecessor.schema_sha256(),
                signing_authority: &authority,
            },
        },
        |_| None,
        |_, _, _, _| Ok(()),
        |_, _, _| Ok(()),
    )
    .expect("result advances under the current custody generation");

    let mut second_transfer_preimage = custody_preimage(ReconciliationCustodyStateV1::Transferred);
    second_transfer_preimage.case_ref = case_ref.clone();
    second_transfer_preimage.case_sha256 = case.canonical_case_sha256().clone();
    second_transfer_preimage.custody_generation =
        ReconciliationCustodyGenerationV1::new(3).expect("second transfer generation");
    second_transfer_preimage.previous_custody_ref = Some(transferred_ref.clone());
    second_transfer_preimage.previous_custody_sha256 =
        Some(transferred.canonical_custody_sha256().clone());
    second_transfer_preimage.holder_principal_id =
        ResidentPrincipalIdV1::parse("principal-3").expect("second holder");
    second_transfer_preimage.transferred_or_closed_at =
        Some(UnixTimeV1::new(23).expect("second transfer time"));
    let second_transfer = ReconciliationCustodyV1::admit_successor_wire(
        custody_wire(second_transfer_preimage),
        &ReconciliationCustodyTransitionContextV1::Transfer {
            predecessor: ReconciliationCustodyPredecessorV1 {
                predecessor_ref: &transferred_ref,
                predecessor: &transferred,
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: transferred.schema_sha256(),
                    signing_authority: &authority,
                },
            },
        },
        |_, _, _, _| Ok(()),
        |_, _, _| Ok(()),
    )
    .expect("second admitted custody transfer");
    let second_transfer_ref = record_ref("custody-generation-3");

    let mut second_successor = predecessor_preimage;
    second_successor.custody_ref = second_transfer_ref.clone();
    second_successor.custody_sha256 = second_transfer.canonical_custody_sha256().clone();
    second_successor.custody_generation = *second_transfer.custody_generation();
    second_successor.case_revision =
        ReconciliationCaseRevisionV1::new(2).expect("successor revision");
    second_successor.predecessor_result_ref = Some(predecessor_ref.clone());
    second_successor.predecessor_result_sha256 =
        Some(predecessor.canonical_reconciliation_sha256().clone());
    second_successor.requested_at = UnixTimeV1::new(24).expect("request time");
    second_successor.recorded_at = UnixTimeV1::new(25).expect("record time");
    let (request, outer_request, _) =
        request_for_result(Some((&predecessor_ref, &predecessor)), &second_successor);
    let wire = serde_json::from_value(serde_json::to_value(&request).expect("request serializes"))
        .expect("request wire decodes");
    assert!(ReconciliationRequestV1::admit_contextual_wire(
        wire,
        &ReconciliationRequestAdmissionContextV1 {
            outer_request: &outer_request,
            case_ref: &case_ref,
            case: &case,
            custody_ref: &second_transfer_ref,
            custody: &second_transfer,
            predecessor: Some((&predecessor_ref, &predecessor)),
            intervening_custody: &[],
        },
        |_| None,
    )
    .is_err());

    let lineage = [ReconciliationCustodyLineageMemberV1 {
        custody_ref: &transferred_ref,
        custody: &transferred,
    }];
    ReconciliationRequestV1::admit_contextual_wire(
        serde_json::from_value(serde_json::to_value(&request).expect("request serializes"))
            .expect("request wire decodes"),
        &ReconciliationRequestAdmissionContextV1 {
            outer_request: &outer_request,
            case_ref: &case_ref,
            case: &case,
            custody_ref: &second_transfer_ref,
            custody: &second_transfer,
            predecessor: Some((&predecessor_ref, &predecessor)),
            intervening_custody: &lineage,
        },
        |_| None,
    )
    .expect("gap-free custody lineage admits the later generation");

    let wrong_lineage = [ReconciliationCustodyLineageMemberV1 {
        custody_ref: &record_ref("wrong-generation-2-reference"),
        custody: &transferred,
    }];
    assert!(ReconciliationRequestV1::admit_contextual_wire(
        serde_json::from_value(serde_json::to_value(&request).expect("request serializes"),)
            .expect("request wire decodes"),
        &ReconciliationRequestAdmissionContextV1 {
            outer_request: &outer_request,
            case_ref: &case_ref,
            case: &case,
            custody_ref: &second_transfer_ref,
            custody: &second_transfer,
            predecessor: Some((&predecessor_ref, &predecessor)),
            intervening_custody: &wrong_lineage,
        },
        |_| None,
    )
    .is_err());
}

#[test]
fn compensation_rejects_an_incomplete_expected_request_binding() {
    let payload = serde_json::json!({"restore": "expected"});
    let canonical = crate::canon::to_canonical_string(&payload).expect("canonical action payload");
    let compensation_action = ActionEnvelopeV1::admit(
        ActionEnvelopePreimageV1 {
            action_id: ActionIdV1::parse("compensation-action-1").expect("action id"),
            target_system: ActionTargetSystemV1::parse("provider").expect("target system"),
            target_identity: ActionTargetIdentityV1::parse("resource-1").expect("target identity"),
            conflict_resource_key_schema_id: SchemaId::parse("example/resource-key.v1")
                .expect("key schema"),
            conflict_resource_key_schema_sha256: SchemaDigest::parse(sha('1'))
                .expect("key schema digest"),
            conflict_resource_key: serde_json::json!({"resource":"one"}),
            operation: ActionOperationV1::parse("restore").expect("operation"),
            business_idempotency_key: BusinessIdempotencyKeyV1::parse("compensation-business-1")
                .expect("business key"),
            expected_precondition_digest: None,
            payload_schema_id: SchemaId::parse("example/compensation.v1").expect("payload schema"),
            payload_schema_sha256: SchemaDigest::parse(sha('2')).expect("payload schema digest"),
            payload_hash: ArtifactDigest::hash_bytes(canonical.as_bytes()),
            inline_payload: Some(payload),
            immutable_payload_ref: None,
            effect_policy: record_ref("compensation-effect-policy"),
            reconciliation_policy: record_ref("reconciliation-policy"),
        },
        |_| None,
    )
    .expect("compensation action");

    let mut result = result_preimage(
        ReconciliationCaseStateV1::Drift,
        Some(ReconciliationResolutionV1::Compensated),
    );
    let (carrier_body, _, _) = request_for_result(None, &result);
    let mut compensation_request_preimage =
        common_request_preimage_for_reconciliation(&carrier_body);
    compensation_request_preimage.request_id =
        ResidentRequestIdV1::parse("compensation-request-1").expect("request id");
    compensation_request_preimage.capability_id =
        CapabilityIdV1::parse("compensate").expect("capability");
    compensation_request_preimage.idempotency_key =
        RequestIdempotencyKeyV1::parse("compensation-idempotency-1").expect("idempotency");
    let action_payload =
        serde_json::to_value(&compensation_action).expect("compensation action serializes");
    compensation_request_preimage.body = CommonRequestBodyV1 {
        schema_id: SchemaId::parse(ACTION_ENVELOPE_V1_SCHEMA).expect("action schema"),
        schema_sha256: SchemaDigest::parse(sha('3')).expect("action schema digest"),
        payload_sha256: ArtifactDigest::parse(
            compensation_action
                .canonical_action_body_sha256()
                .expect("action digest")
                .as_str(),
        )
        .expect("action body digest"),
        payload: action_payload,
    };
    let compensation_request =
        CommonRequestEnvelopeV1::seal(compensation_request_preimage.clone(), signature())
            .expect("compensation request");
    let compensation_request_binding =
        request_binding_for_common_request(&compensation_request, &compensation_request_preimage);
    let compensation_result = EffectReceiptV1::seal(
        EffectReceiptPreimageV1 {
            schema_id: SchemaId::parse(EFFECT_RECEIPT_V1_SCHEMA).expect("receipt schema"),
            schema_sha256: SchemaDigest::parse(sha('4')).expect("receipt schema digest"),
            request_binding: compensation_request_binding.clone(),
            action_id: compensation_action.action_id().clone(),
            attempt_id: CapabilityAttemptIdV1::parse("compensation-attempt-1").expect("attempt"),
            effect_dispatch_intent_ref: record_ref("compensation-dispatch"),
            effect_dispatch_intent_sha256: EffectDispatchIntentDigest::parse(sha('5'))
                .expect("dispatch digest"),
            principal_scope: compensation_request_preimage.scope.principal_scope.clone(),
            controller_scope: compensation_request_preimage.scope.controller_scope.clone(),
            relationship_scope: compensation_request_preimage
                .scope
                .relationship_scope
                .clone(),
            transport_status: BoundedResidentTextV1::parse("accepted").expect("status"),
            observed_effect: ObservedEffectV1::Confirmed,
            external_reference: None,
            observed_postcondition_digest: Some(typed_state("expected")),
            absence_evidence_ref: None,
            absence_evidence_sha256: None,
            action_body_sha256: compensation_action
                .canonical_action_body_sha256()
                .expect("action digest"),
            effect_admission_ref: record_ref("compensation-admission"),
            effect_admission_sha256: EffectAdmissionDigest::parse(sha('6'))
                .expect("admission digest"),
            conflict_claim_ref: record_ref("compensation-claim"),
            conflict_claim_sha256: EffectConflictClaimDigest::parse(sha('7'))
                .expect("claim digest"),
            provider_request_digest: None,
            response_digest: None,
            implementation_kind: BoundedResidentTextV1::parse("native").expect("kind"),
            implementation_version: BoundedResidentTextV1::parse("1").expect("version"),
            reconciliation_required: false,
            reconciliation_case_id: None,
            started_at: UnixTimeV1::new(11).expect("valid fixture time"),
            completed_at: UnixTimeV1::new(12).expect("valid fixture time"),
            deployment_runtime_authority_ref: record_ref("authority"),
            deployment_runtime_authority_sha256: artifact('a'),
            issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
            issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
            issuer_key_epoch: SigningKeyEpochV1::new(1).expect("valid fixture signing key epoch"),
        },
        signature(),
    )
    .expect("compensation result receipt");
    let compensation_request_ref = record_ref("compensation-request");
    let compensation_action_ref = record_ref("compensation-action");
    let compensation_result_ref = record_ref("compensation-result");
    let binding = result.compensation.as_mut().expect("compensation binding");
    binding.compensation_request_ref = compensation_request_ref.clone();
    binding.compensation_request_sha256 = compensation_request.canonical_request_sha256().clone();
    binding.compensation_action_ref = compensation_action_ref.clone();
    binding.compensation_action_sha256 = compensation_action
        .canonical_action_body_sha256()
        .expect("action digest");
    binding.compensation_result_ref = compensation_result_ref.clone();
    binding.compensation_result_sha256 = compensation_result.canonical_receipt_sha256().clone();
    let resolved = ResolvedReconciliationCompensationV1 {
        request_ref: &compensation_request_ref,
        request: &compensation_request,
        request_binding: &compensation_request_binding,
        action_ref: &compensation_action_ref,
        action: &compensation_action,
        result_ref: &compensation_result_ref,
        result: &compensation_result,
    };
    validate_compensation_context(&result, Some(&resolved))
        .expect("complete compensation binding admits");

    let mut incomplete_binding =
        serde_json::to_value(&compensation_request_binding).expect("binding serializes");
    incomplete_binding["correlation_id"] = serde_json::json!("other-correlation");
    let incomplete_binding: RequestBindingV1 =
        serde_json::from_value(incomplete_binding).expect("binding shape");
    let incomplete = ResolvedReconciliationCompensationV1 {
        request_binding: &incomplete_binding,
        ..resolved
    };
    assert!(validate_compensation_context(&result, Some(&incomplete)).is_err());
}
