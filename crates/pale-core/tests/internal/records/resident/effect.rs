//! Exercises resident effect-state records, request binding, and transition invariants.

use super::*;

fn sha(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn request_binding() -> RequestBindingV1 {
    RequestBindingV1::new(RequestBindingPreimageV1 {
        request_id: ResidentRequestIdV1::parse("request-1").expect("request id"),
        correlation_id: ResidentCorrelationIdV1::parse("correlation-1").expect("correlation id"),
        capability_id: CapabilityIdV1::parse("read").expect("capability id"),
        canonical_request_sha256: CommonRequestEnvelopeDigest::parse(sha('a'))
            .expect("request digest"),
        session_grant_id: SessionGrantIdV1::parse("grant-1").expect("session grant id"),
        session_grant_sha256: ArtifactDigest::parse(sha('b')).expect("grant digest"),
        deployment_binding_ref: ResidentRecordRefV1::parse("deployment").expect("deployment ref"),
        deployment_binding_sha256: ArtifactDigest::parse(sha('c')).expect("deployment digest"),
        route_binding_schema_id: SchemaId::parse("example/route.v1").expect("route schema"),
        route_binding_schema_sha256: SchemaDigest::parse(sha('d')).expect("route schema digest"),
        route_binding_ref: ResidentRecordRefV1::parse("route").expect("route ref"),
        route_binding_sha256: ArtifactDigest::parse(sha('e')).expect("route digest"),
        route_generation: RouteGenerationV1::new(1).expect("valid fixture route generation"),
        contract_set_ref: ResidentRecordRefV1::parse("contract-set").expect("contract set ref"),
        contract_set_sha256: ArtifactDigest::parse(sha('f')).expect("contract set digest"),
        execution_owner_fence_ref: None,
        execution_owner_fence_sha256: None,
        federation_request_admission_ref: None,
        federation_request_admission_sha256: None,
    })
    .expect("request binding")
}

fn signing_authority() -> SigningAuthorityBindingV1 {
    serde_json::from_value(serde_json::json!({
        "authority_mode": "integrated_runtime",
        "principal_id": "principal-1",
        "signer_key_id": "key-1",
        "signer_key_epoch": 1,
        "integrated_runtime": {
            "deployment_runtime_authority_ref": "authority",
            "deployment_runtime_authority_sha256": sha('a'),
            "deployment_binding_ref": "deployment",
            "deployment_binding_sha256": sha('b'),
            "checked_principal_trust_checkpoint_ref": "trust",
            "checked_principal_trust_checkpoint_sha256": sha('c')
        }
    }))
    .expect("signing authority")
}

fn state_digest() -> TypedDigestV1 {
    let value = serde_json::json!({"state":"ready"});
    let canonical = crate::canon::to_canonical_string(&value).expect("canonical state");
    TypedDigestV1::admit(
        TypedDigestPreimageV1 {
            kind: BoundedResidentTextV1::parse("postcondition").expect("digest kind"),
            content_schema_id: SchemaId::parse("example/state.v1").expect("state schema"),
            content_schema_sha256: SchemaDigest::parse(sha('a')).expect("state schema digest"),
            canonicalization: TypedDigestCanonicalizationV1::Rfc8785Json,
            digest_algorithm: TypedDigestAlgorithmV1::Sha256,
            digest: ArtifactDigest::hash_bytes(canonical.as_bytes()),
            content_binding: TypedDigestContentBindingV1::InlineJson { inline_json: value },
        },
        |_| None,
    )
    .expect("state digest")
}

fn receipt_preimage(state: ObservedEffectV1) -> EffectReceiptPreimageV1 {
    let needs_reconciliation =
        matches!(state, ObservedEffectV1::Unknown | ObservedEffectV1::Partial);
    EffectReceiptPreimageV1 {
        schema_id: SchemaId::parse("pale/effect-receipt.v1").expect("receipt schema"),
        schema_sha256: SchemaDigest::parse(sha('a')).expect("receipt schema digest"),
        request_binding: request_binding(),
        action_id: ActionIdV1::parse("action-1").expect("action id"),
        attempt_id: CapabilityAttemptIdV1::parse("attempt-1").expect("attempt id"),
        effect_dispatch_intent_ref: ResidentRecordRefV1::parse("dispatch").expect("dispatch ref"),
        effect_dispatch_intent_sha256: EffectDispatchIntentDigest::parse(sha('b'))
            .expect("dispatch digest"),
        principal_scope: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        controller_scope: ResidentControllerIdV1::parse("controller-1").expect("controller"),
        relationship_scope: None,
        transport_status: BoundedResidentTextV1::parse("accepted").expect("transport status"),
        observed_effect: state,
        external_reference: None,
        observed_postcondition_digest: matches!(
            state,
            ObservedEffectV1::Confirmed | ObservedEffectV1::Partial
        )
        .then(state_digest),
        absence_evidence_ref: (state == ObservedEffectV1::NotApplied)
            .then(|| ResidentRecordRefV1::parse("absence").expect("absence ref")),
        absence_evidence_sha256: (state == ObservedEffectV1::NotApplied)
            .then(|| ArtifactDigest::parse(sha('c')).expect("absence digest")),
        action_body_sha256: ActionEnvelopeDigest::parse(sha('d')).expect("action digest"),
        effect_admission_ref: ResidentRecordRefV1::parse("admission").expect("admission ref"),
        effect_admission_sha256: EffectAdmissionDigest::parse(sha('e')).expect("admission digest"),
        conflict_claim_ref: ResidentRecordRefV1::parse("claim").expect("claim ref"),
        conflict_claim_sha256: EffectConflictClaimDigest::parse(sha('f')).expect("claim digest"),
        provider_request_digest: None,
        response_digest: None,
        implementation_kind: BoundedResidentTextV1::parse("native").expect("implementation kind"),
        implementation_version: BoundedResidentTextV1::parse("1").expect("implementation version"),
        reconciliation_required: needs_reconciliation,
        reconciliation_case_id: needs_reconciliation
            .then(|| ReconciliationCaseIdV1::parse("case-1").expect("case id")),
        started_at: UnixTimeV1::new(1).expect("valid fixture time"),
        completed_at: UnixTimeV1::new(2).expect("valid fixture time"),
        deployment_runtime_authority_ref: ResidentRecordRefV1::parse("authority")
            .expect("runtime authority ref"),
        deployment_runtime_authority_sha256: ArtifactDigest::parse(sha('a'))
            .expect("runtime authority digest"),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("valid fixture signing key epoch"),
    }
}

#[test]
fn common_request_schema_uses_exact_outer_wire_names() {
    let schema = serde_json::to_value(schemars::schema_for!(CommonRequestEnvelopeV1))
        .expect("schema serializes");
    let properties = schema
        .pointer("/properties")
        .and_then(serde_json::Value::as_object)
        .expect("root object properties");
    for required in [
        "envelope_schema_id",
        "envelope_schema_sha256",
        "canonical_request_sha256",
        "caller_signature",
    ] {
        assert!(properties.contains_key(required), "missing {required}");
    }
    for retired in ["schema_id", "schema_sha256", "schema_version", "signature"] {
        assert!(!properties.contains_key(retired), "unexpected {retired}");
    }

    let execution_fence = schema
        .pointer("/$defs/CommonRequestExecutionOwnerFenceV1/properties")
        .and_then(serde_json::Value::as_object)
        .expect("neutral execution-owner projection");
    for required in [
        "execution_fence_ref",
        "execution_fence_sha256",
        "execution_attempt_id",
        "execution_path_id",
        "execution_owner_system_id",
        "execution_owner_participant_id",
        "owner_authorization_schema_id",
        "owner_authorization_schema_sha256",
        "owner_authorization_ref",
        "owner_authorization_sha256",
        "owner_generation",
        "control_epoch",
    ] {
        assert!(
            execution_fence.contains_key(required),
            "missing neutral execution-owner member {required}"
        );
    }
    for retired in [
        "paperclip_execution_authorization_ref",
        "paperclip_execution_authorization_sha256",
    ] {
        assert!(
            !execution_fence.contains_key(retired),
            "unexpected product-specific member {retired}"
        );
    }
}

#[test]
fn typed_digest_rejects_json_bound_as_raw_bytes() {
    let result = TypedDigestV1::admit(
        TypedDigestPreimageV1 {
            kind: BoundedResidentTextV1::parse("expected_state").expect("digest kind"),
            content_schema_id: SchemaId::parse("example/state.v1").expect("schema id"),
            content_schema_sha256: SchemaDigest::parse(sha('a')).expect("schema digest"),
            canonicalization: TypedDigestCanonicalizationV1::RawBytes,
            digest_algorithm: TypedDigestAlgorithmV1::Sha256,
            digest: ArtifactDigest::parse(sha('b')).expect("content digest"),
            content_binding: TypedDigestContentBindingV1::InlineJson {
                inline_json: serde_json::json!({"state": "ready"}),
            },
        },
        |_| None,
    );
    assert!(matches!(
        result,
        Err(InvalidResidentRecordV1::NonCanonical {
            relation: "typed digest content binding"
        })
    ));
}

#[test]
fn signing_authority_rejects_missing_selected_variant() {
    let value = serde_json::json!({
        "authority_mode": "integrated_runtime",
        "principal_id": "principal-1",
        "signer_key_id": "key-1",
        "signer_key_epoch": 1,
        "integrated_runtime": null
    });
    assert!(serde_json::from_value::<SigningAuthorityBindingV1>(value).is_err());
}

#[test]
fn signing_authority_deserialize_enforces_epoch_and_exact_variant() {
    let mut value = serde_json::json!({
        "authority_mode": "integrated_runtime",
        "principal_id": "principal-1",
        "signer_key_id": "key-1",
        "signer_key_epoch": 1,
        "integrated_runtime": {
            "deployment_runtime_authority_ref": "authority",
            "deployment_runtime_authority_sha256": sha('a'),
            "deployment_binding_ref": "deployment",
            "deployment_binding_sha256": sha('b'),
            "checked_principal_trust_checkpoint_ref": "trust",
            "checked_principal_trust_checkpoint_sha256": sha('c')
        }
    });
    let admitted: SigningAuthorityBindingV1 =
        serde_json::from_value(value.clone()).expect("exact authority admits");
    assert_eq!(admitted.signer_key_epoch().get(), 1);
    value["signer_key_epoch"] = serde_json::json!(0);
    assert!(serde_json::from_value::<SigningAuthorityBindingV1>(value).is_err());
}

#[test]
fn request_binding_deserialize_enforces_federation_pair() {
    let mut value = serde_json::json!({
        "request_id": "request-1",
        "correlation_id": "correlation-1",
        "capability_id": "read",
        "canonical_request_sha256": sha('a'),
        "session_grant_id": "grant-1",
        "session_grant_sha256": sha('b'),
        "deployment_binding_ref": "deployment",
        "deployment_binding_sha256": sha('c'),
        "route_binding_schema_id": "example/route.v1",
        "route_binding_schema_sha256": sha('d'),
        "route_binding_ref": "route",
        "route_binding_sha256": sha('e'),
        "route_generation": 1,
        "contract_set_ref": "contract-set",
        "contract_set_sha256": sha('f')
    });
    let admitted: RequestBindingV1 =
        serde_json::from_value(value.clone()).expect("local binding admits");
    assert_eq!(admitted.route_generation().get(), 1);
    value["federation_request_admission_ref"] = serde_json::json!("federation");
    assert!(serde_json::from_value::<RequestBindingV1>(value).is_err());
}

#[test]
fn shared_nested_bindings_serialize_absence_by_omission_and_reject_null() {
    let authority = signing_authority();
    let authority_json = serde_json::to_value(&authority).expect("serialize authority");
    assert!(authority_json.get("integrated_owner").is_none());
    assert!(authority_json.get("standalone_owner_native").is_none());

    let binding = request_binding();
    let binding_json = serde_json::to_value(&binding).expect("serialize request binding");
    assert!(binding_json.get("execution_owner_fence_ref").is_none());
    assert!(binding_json
        .get("federation_request_admission_ref")
        .is_none());

    let mut explicit_null = authority_json;
    explicit_null["integrated_owner"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<SigningAuthorityBindingV1>(explicit_null).is_err());

    let mut explicit_null = binding_json;
    explicit_null["execution_owner_fence_ref"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<RequestBindingV1>(explicit_null).is_err());
}

#[test]
fn typed_digest_checks_canonical_base64_and_decoded_bytes() {
    let valid = TypedDigestV1::admit(
        TypedDigestPreimageV1 {
            kind: BoundedResidentTextV1::parse("raw").expect("digest kind"),
            content_schema_id: SchemaId::parse("example/raw.v1").expect("schema id"),
            content_schema_sha256: SchemaDigest::parse(sha('a')).expect("schema digest"),
            canonicalization: TypedDigestCanonicalizationV1::RawBytes,
            digest_algorithm: TypedDigestAlgorithmV1::Sha256,
            digest: ArtifactDigest::hash_bytes(b"M"),
            content_binding: TypedDigestContentBindingV1::InlineBytesBase64 {
                inline_bytes_base64: "TQ==".into(),
            },
        },
        |_| None,
    );
    assert!(valid.is_ok());

    let noncanonical = TypedDigestV1::admit(
        TypedDigestPreimageV1 {
            kind: BoundedResidentTextV1::parse("raw").expect("digest kind"),
            content_schema_id: SchemaId::parse("example/raw.v1").expect("schema id"),
            content_schema_sha256: SchemaDigest::parse(sha('a')).expect("schema digest"),
            canonicalization: TypedDigestCanonicalizationV1::RawBytes,
            digest_algorithm: TypedDigestAlgorithmV1::Sha256,
            digest: ArtifactDigest::hash_bytes(b"M"),
            content_binding: TypedDigestContentBindingV1::InlineBytesBase64 {
                inline_bytes_base64: "TR==".into(),
            },
        },
        |_| None,
    );
    assert!(noncanonical.is_err());
}

#[test]
fn immutable_json_separates_source_byte_and_canonical_digests() {
    let bytes = br#"{ "state" : "ready" }"#.to_vec();
    let canonical = br#"{"state":"ready"}"#;
    let digest = TypedDigestV1::admit(
        TypedDigestPreimageV1 {
            kind: BoundedResidentTextV1::parse("expected_state").expect("digest kind"),
            content_schema_id: SchemaId::parse("example/state.v1").expect("schema id"),
            content_schema_sha256: SchemaDigest::parse(sha('a')).expect("schema digest"),
            canonicalization: TypedDigestCanonicalizationV1::Rfc8785Json,
            digest_algorithm: TypedDigestAlgorithmV1::Sha256,
            digest: ArtifactDigest::hash_bytes(canonical),
            content_binding: TypedDigestContentBindingV1::ImmutableContent {
                content_ref: ResidentRecordRefV1::parse("state-record").expect("record ref"),
                content_ref_sha256: ArtifactDigest::hash_bytes(&bytes),
            },
        },
        |_| Some(bytes.clone()),
    )
    .expect("typed digest");
    assert_eq!(digest.kind(), "expected_state");
}

#[test]
fn immutable_json_rejects_duplicate_keys_before_canonicalization() {
    let bytes = br#"{"state":"ready","state":"other"}"#.to_vec();
    let digest = TypedDigestV1::admit(
        TypedDigestPreimageV1 {
            kind: BoundedResidentTextV1::parse("expected_state").expect("digest kind"),
            content_schema_id: SchemaId::parse("example/state.v1").expect("schema id"),
            content_schema_sha256: SchemaDigest::parse(sha('a')).expect("schema digest"),
            canonicalization: TypedDigestCanonicalizationV1::Rfc8785Json,
            digest_algorithm: TypedDigestAlgorithmV1::Sha256,
            digest: ArtifactDigest::hash_bytes(br#"{"state":"other"}"#),
            content_binding: TypedDigestContentBindingV1::ImmutableContent {
                content_ref: ResidentRecordRefV1::parse("state-record").expect("record ref"),
                content_ref_sha256: ArtifactDigest::hash_bytes(&bytes),
            },
        },
        |_| Some(bytes.clone()),
    );
    assert!(digest.is_err());
}

#[test]
fn payload_shape_counts_depth_and_members() {
    let value = serde_json::json!({"a": [{"b": true}, {"c": false}]});
    assert_eq!(resident_json_shape(&value), (4, 5));
}

#[test]
fn effect_receipt_enforces_all_four_state_matrices_and_policy() {
    let signature = || CanonicalSignatureV1::parse("A".repeat(86)).expect("signature");
    let rejects = |preimage| EffectReceiptV1::seal(preimage, signature()).is_err();
    for state in [
        ObservedEffectV1::Confirmed,
        ObservedEffectV1::NotApplied,
        ObservedEffectV1::Unknown,
        ObservedEffectV1::Partial,
    ] {
        let receipt = EffectReceiptV1::seal(receipt_preimage(state), signature())
            .expect("valid state matrix");
        let policy = matches!(state, ObservedEffectV1::Unknown | ObservedEffectV1::Partial);
        assert!(receipt.verify_reconciliation_policy(policy).is_ok());
    }
    let mut invalid = receipt_preimage(ObservedEffectV1::Unknown);
    invalid.response_digest = Some(state_digest());
    assert!(rejects(invalid));

    let mut invalid = receipt_preimage(ObservedEffectV1::Confirmed);
    invalid.observed_postcondition_digest = None;
    assert!(rejects(invalid));
    let mut invalid = receipt_preimage(ObservedEffectV1::Confirmed);
    invalid.absence_evidence_ref = Some(ResidentRecordRefV1::parse("absence").expect("absence"));
    invalid.absence_evidence_sha256 =
        Some(ArtifactDigest::parse(sha('c')).expect("absence digest"));
    assert!(rejects(invalid));

    let mut invalid = receipt_preimage(ObservedEffectV1::NotApplied);
    invalid.absence_evidence_ref = None;
    invalid.absence_evidence_sha256 = None;
    assert!(rejects(invalid));
    let mut invalid = receipt_preimage(ObservedEffectV1::NotApplied);
    invalid.external_reference = Some(ResidentRecordRefV1::parse("external").expect("external"));
    assert!(rejects(invalid));
    let mut invalid = receipt_preimage(ObservedEffectV1::NotApplied);
    invalid.observed_postcondition_digest = Some(state_digest());
    assert!(rejects(invalid));

    let mut invalid = receipt_preimage(ObservedEffectV1::Unknown);
    invalid.external_reference = Some(ResidentRecordRefV1::parse("external").expect("external"));
    assert!(rejects(invalid));
    let mut invalid = receipt_preimage(ObservedEffectV1::Unknown);
    invalid.observed_postcondition_digest = Some(state_digest());
    assert!(rejects(invalid));
    let mut invalid = receipt_preimage(ObservedEffectV1::Unknown);
    invalid.absence_evidence_ref = Some(ResidentRecordRefV1::parse("absence").expect("absence"));
    invalid.absence_evidence_sha256 =
        Some(ArtifactDigest::parse(sha('c')).expect("absence digest"));
    assert!(rejects(invalid));
    let mut invalid = receipt_preimage(ObservedEffectV1::Unknown);
    invalid.reconciliation_required = false;
    invalid.reconciliation_case_id = None;
    assert!(rejects(invalid));

    let mut invalid = receipt_preimage(ObservedEffectV1::Partial);
    invalid.observed_postcondition_digest = None;
    assert!(rejects(invalid));
    let mut invalid = receipt_preimage(ObservedEffectV1::Partial);
    invalid.absence_evidence_ref = Some(ResidentRecordRefV1::parse("absence").expect("absence"));
    invalid.absence_evidence_sha256 =
        Some(ArtifactDigest::parse(sha('c')).expect("absence digest"));
    assert!(rejects(invalid));
    let mut invalid = receipt_preimage(ObservedEffectV1::Partial);
    invalid.reconciliation_required = false;
    invalid.reconciliation_case_id = None;
    assert!(rejects(invalid));

    let mut invalid = receipt_preimage(ObservedEffectV1::Unknown);
    invalid.reconciliation_case_id = None;
    assert!(rejects(invalid));

    let confirmed =
        EffectReceiptV1::seal(receipt_preimage(ObservedEffectV1::Confirmed), signature())
            .expect("confirmed receipt");
    assert!(confirmed.verify_reconciliation_policy(false).is_ok());
    assert!(confirmed.verify_reconciliation_policy(true).is_err());
    let not_applied =
        EffectReceiptV1::seal(receipt_preimage(ObservedEffectV1::NotApplied), signature())
            .expect("not-applied receipt");
    assert!(not_applied.verify_reconciliation_policy(false).is_ok());
    assert!(not_applied.verify_reconciliation_policy(true).is_err());

    for state in [ObservedEffectV1::Confirmed, ObservedEffectV1::NotApplied] {
        let mut required = receipt_preimage(state);
        required.reconciliation_required = true;
        required.reconciliation_case_id =
            Some(ReconciliationCaseIdV1::parse("case-1").expect("case id"));
        let receipt = EffectReceiptV1::seal(required, signature())
            .expect("policy-required reconciliation receipt");
        assert!(receipt.verify_reconciliation_policy(true).is_ok());
    }
}

fn active_claim_preimage() -> EffectConflictClaimPreimageV1 {
    EffectConflictClaimPreimageV1 {
        schema_id: SchemaId::parse("pale/effect-conflict-claim.v1").expect("claim schema"),
        schema_sha256: SchemaDigest::parse(sha('1')).expect("claim schema digest"),
        conflict_claim_id: EffectConflictClaimIdV1::parse("claim-1").expect("claim id"),
        claim_generation: EffectConflictClaimGenerationV1::new(1)
            .expect("valid fixture claim generation"),
        previous_claim_ref: None,
        previous_claim_sha256: None,
        conflict_resource_key_schema_id: SchemaId::parse("example/resource-key.v1")
            .expect("key schema"),
        conflict_resource_key_schema_sha256: SchemaDigest::parse(sha('2'))
            .expect("key schema digest"),
        conflict_resource_key: serde_json::json!({"contact":"contact-1"}),
        principal_scope: ResidentPrincipalIdV1::parse("principal-1").expect("principal"),
        controller_scope: ResidentControllerIdV1::parse("controller-1").expect("controller"),
        relationship_scope: None,
        request_id: ResidentRequestIdV1::parse("request-1").expect("request id"),
        canonical_request_sha256: CommonRequestEnvelopeDigest::parse(sha('3'))
            .expect("request digest"),
        action_id: ActionIdV1::parse("action-1").expect("action id"),
        action_body_sha256: ActionEnvelopeDigest::parse(sha('4')).expect("action digest"),
        business_idempotency_key: BusinessIdempotencyKeyV1::parse("business-1")
            .expect("business key"),
        claim_state: EffectConflictClaimStateV1::Active,
        holder_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("holder principal"),
        terminal_reason: None,
        effect_receipt_ref: None,
        effect_receipt_sha256: None,
        cancellation_authority_ref: None,
        cancellation_authority_sha256: None,
        reconciliation_custody_ref: None,
        reconciliation_custody_sha256: None,
        issued_at: UnixTimeV1::new(10).expect("valid fixture time"),
        transferred_or_released_at: None,
        deployment_runtime_authority_ref: ResidentRecordRefV1::parse("authority")
            .expect("runtime authority"),
        deployment_runtime_authority_sha256: ArtifactDigest::parse(sha('a'))
            .expect("runtime authority digest"),
        issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer principal"),
        issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("valid fixture signing key epoch"),
    }
}

#[test]
fn effect_conflict_claim_uses_typed_digest_and_contextual_successor() {
    let signature = || CanonicalSignatureV1::parse("A".repeat(86)).expect("signature");
    let predecessor =
        EffectConflictClaimV1::seal(active_claim_preimage(), signature()).expect("active claim");
    let predecessor_ref = ResidentRecordRefV1::parse("claim-generation-1").expect("claim ref");
    let mut successor = active_claim_preimage();
    successor.claim_generation =
        EffectConflictClaimGenerationV1::new(2).expect("valid successor generation");
    successor.previous_claim_ref = Some(predecessor_ref.clone());
    successor.previous_claim_sha256 =
        Some(predecessor.canonical_effect_conflict_claim_sha256().clone());
    successor.claim_state = EffectConflictClaimStateV1::Released;
    successor.terminal_reason = Some(EffectConflictTerminalReasonV1::CancelledBeforeDispatch);
    let cancellation_ref = ResidentRecordRefV1::parse("cancellation-1").expect("cancellation ref");
    let cancellation_sha256 = ArtifactDigest::parse(sha('5')).expect("cancellation digest");
    successor.cancellation_authority_ref = Some(cancellation_ref.clone());
    successor.cancellation_authority_sha256 = Some(cancellation_sha256.clone());
    successor.issued_at = UnixTimeV1::new(20).expect("valid fixture time");
    successor.transferred_or_released_at = Some(UnixTimeV1::new(20).expect("valid fixture time"));
    let sealed = EffectConflictClaimV1::seal(successor.clone(), signature())
        .expect("released successor shape");
    let wire: EffectConflictClaimWireV1 =
        serde_json::from_value(serde_json::to_value(&sealed).expect("claim serializes"))
            .expect("claim wire decodes");
    EffectConflictClaimV1::admit_successor_wire(
        wire,
        &EffectConflictClaimSuccessorContextV1::CancellationRelease {
            predecessor: EffectConflictClaimPredecessorV1 {
                predecessor_ref: &predecessor_ref,
                predecessor: &predecessor,
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: predecessor.schema_sha256(),
                    signing_authority: &signing_authority(),
                },
            },
            cancellation_authority_ref: &cancellation_ref,
            cancellation_authority_sha256: &cancellation_sha256,
        },
        |_, _, _| Ok(()),
    )
    .expect("exact successor admits");

    let wrong_cancellation_ref =
        ResidentRecordRefV1::parse("other-cancellation").expect("cancellation ref");
    let wrong_cancellation_sha256 = ArtifactDigest::parse(sha('6')).expect("cancellation digest");
    for (name, evidence_ref, evidence_sha256) in [
        (
            "cancellation reference",
            &wrong_cancellation_ref,
            &cancellation_sha256,
        ),
        (
            "cancellation digest",
            &cancellation_ref,
            &wrong_cancellation_sha256,
        ),
    ] {
        let evidence_wire: EffectConflictClaimWireV1 =
            serde_json::from_value(serde_json::to_value(&sealed).expect("claim serializes"))
                .expect("claim wire decodes");
        assert!(
            EffectConflictClaimV1::admit_successor_wire(
                evidence_wire,
                &EffectConflictClaimSuccessorContextV1::CancellationRelease {
                    predecessor: EffectConflictClaimPredecessorV1 {
                        predecessor_ref: &predecessor_ref,
                        predecessor: &predecessor,
                        signed: SignedResidentAdmissionContextV1 {
                            expected_schema_sha256: predecessor.schema_sha256(),
                            signing_authority: &signing_authority(),
                        },
                    },
                    cancellation_authority_ref: evidence_ref,
                    cancellation_authority_sha256: evidence_sha256,
                },
                |_, _, _| Ok(()),
            )
            .is_err(),
            "{name} substitution must fail"
        );
    }

    let released_ref = ResidentRecordRefV1::parse("claim-generation-2").expect("claim ref");
    let mut after_terminal = successor.clone();
    after_terminal.claim_generation =
        EffectConflictClaimGenerationV1::new(3).expect("valid terminal generation");
    after_terminal.previous_claim_ref = Some(released_ref.clone());
    after_terminal.previous_claim_sha256 =
        Some(sealed.canonical_effect_conflict_claim_sha256().clone());
    let after_terminal =
        EffectConflictClaimV1::seal(after_terminal, signature()).expect("terminal successor shape");
    let after_terminal_wire: EffectConflictClaimWireV1 =
        serde_json::from_value(serde_json::to_value(after_terminal).expect("claim serializes"))
            .expect("claim wire decodes");
    assert!(EffectConflictClaimV1::admit_successor_wire(
        after_terminal_wire,
        &EffectConflictClaimSuccessorContextV1::CancellationRelease {
            predecessor: EffectConflictClaimPredecessorV1 {
                predecessor_ref: &released_ref,
                predecessor: &sealed,
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: sealed.schema_sha256(),
                    signing_authority: &signing_authority(),
                },
            },
            cancellation_authority_ref: &cancellation_ref,
            cancellation_authority_sha256: &cancellation_sha256,
        },
        |_, _, _| Ok(()),
    )
    .is_err());

    successor.claim_generation =
        EffectConflictClaimGenerationV1::new(3).expect("valid skipped generation");
    let skipped = EffectConflictClaimV1::seal(successor, signature())
        .expect("skipped claim is structurally valid");
    let skipped_wire: EffectConflictClaimWireV1 =
        serde_json::from_value(serde_json::to_value(skipped).expect("claim serializes"))
            .expect("claim wire decodes");
    assert!(EffectConflictClaimV1::admit_successor_wire(
        skipped_wire,
        &EffectConflictClaimSuccessorContextV1::CancellationRelease {
            predecessor: EffectConflictClaimPredecessorV1 {
                predecessor_ref: &predecessor_ref,
                predecessor: &predecessor,
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: predecessor.schema_sha256(),
                    signing_authority: &signing_authority(),
                },
            },
            cancellation_authority_ref: &cancellation_ref,
            cancellation_authority_sha256: &cancellation_sha256,
        },
        |_, _, _| Ok(()),
    )
    .is_err());
}
