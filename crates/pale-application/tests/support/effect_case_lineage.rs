//! Shared construction helpers for resident effect-case lineage tests.

use pale_core::records::*;
use serde::{de::DeserializeOwned, Serialize};
#[path = "effect_case_lineage/request.rs"]
mod request_fixture;
pub(crate) use request_fixture::{action, request, request_preimage};
use request_fixture::{artifact, signature};
pub(super) use request_fixture::{record_ref, sha};

pub(super) fn signed_wire<P, D, W>(preimage: &P, digest_name: &str, digest: &D) -> W
where
    P: Serialize,
    D: Serialize,
    W: DeserializeOwned,
{
    let mut value = serde_json::to_value(preimage).expect("preimage serialization");
    let object = value.as_object_mut().expect("record object");
    object.insert(
        digest_name.to_owned(),
        serde_json::to_value(digest).expect("digest serialization"),
    );
    object.insert(
        "signature".to_owned(),
        serde_json::to_value(signature()).expect("signature serialization"),
    );
    serde_json::from_value(value).expect("record wire")
}

pub(super) struct EffectFixture {
    pub(super) request_preimage: CommonRequestEnvelopePreimageV1,
    pub(super) request_ref: ResidentRecordRefV1,
    pub(super) request: CommonRequestEnvelopeV1,
    pub(super) request_binding: RequestBindingV1,
    pub(super) action_ref: ResidentRecordRefV1,
    pub(super) action: ActionEnvelopeV1,
    pub(super) claim_preimage: EffectConflictClaimPreimageV1,
    pub(super) claim_ref: ResidentRecordRefV1,
    pub(super) claim: EffectConflictClaimV1,
    pub(super) admission_ref: ResidentRecordRefV1,
    pub(super) admission: EffectAdmissionV1,
    pub(super) intent_ref: ResidentRecordRefV1,
    pub(super) intent: EffectDispatchIntentV1,
}

impl EffectFixture {
    pub(super) fn new(label: &str) -> Self {
        let action = action(label);
        let request_preimage = request_preimage(label, &action);
        let request = request(&request_preimage);
        let request_binding = RequestBindingV1::new(RequestBindingPreimageV1 {
            request_id: request_preimage.request_id.clone(),
            correlation_id: request_preimage.correlation_id.clone(),
            capability_id: request_preimage.capability_id.clone(),
            canonical_request_sha256: request.canonical_request_sha256().clone(),
            session_grant_id: request_preimage.session_admission.session_grant_id.clone(),
            session_grant_sha256: request_preimage
                .session_admission
                .session_grant_sha256
                .clone(),
            deployment_binding_ref: request_preimage
                .session_admission
                .deployment_binding_ref
                .clone(),
            deployment_binding_sha256: request_preimage
                .session_admission
                .deployment_binding_sha256
                .clone(),
            route_binding_schema_id: request_preimage.route.route_binding_schema_id.clone(),
            route_binding_schema_sha256: request_preimage.route.route_binding_schema_sha256.clone(),
            route_binding_ref: request_preimage.route.route_binding_ref.clone(),
            route_binding_sha256: request_preimage.route.route_binding_sha256.clone(),
            route_generation: request_preimage.route.route_generation,
            contract_set_ref: request_preimage.route.contract_set_ref.clone(),
            contract_set_sha256: request_preimage.route.contract_set_sha256.clone(),
            execution_owner_fence_ref: None,
            execution_owner_fence_sha256: None,
            federation_request_admission_ref: None,
            federation_request_admission_sha256: None,
        })
        .expect("request binding");
        let request_ref = record_ref(&format!("request-{label}"));
        let action_ref = record_ref(&format!("action-{label}"));
        let action_digest = action
            .canonical_action_body_sha256()
            .expect("action digest");
        let claim_ref = record_ref(&format!("claim-{label}"));
        let runtime_authority = request_preimage
            .signing_authority_binding
            .integrated_runtime_authority()
            .expect("integrated runtime authority");
        let claim_preimage = EffectConflictClaimPreimageV1 {
            schema_id: SchemaId::parse(EFFECT_CONFLICT_CLAIM_V1_SCHEMA).expect("claim schema"),
            schema_sha256: SchemaDigest::parse(sha('b')).expect("claim schema digest"),
            deployment_runtime_authority_ref: runtime_authority
                .deployment_runtime_authority_ref
                .clone(),
            deployment_runtime_authority_sha256: runtime_authority
                .deployment_runtime_authority_sha256
                .clone(),
            conflict_claim_id: EffectConflictClaimIdV1::parse(format!("claim-{label}"))
                .expect("claim id"),
            claim_generation: EffectConflictClaimGenerationV1::new(1).expect("claim generation"),
            previous_claim_ref: None,
            previous_claim_sha256: None,
            conflict_resource_key_schema_id: action.conflict_resource_key_schema_id().clone(),
            conflict_resource_key_schema_sha256: action
                .conflict_resource_key_schema_sha256()
                .clone(),
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
            issued_at: UnixTimeV1::new(10).expect("claim time"),
            transferred_or_released_at: None,
            issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
            issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
            issuer_key_epoch: SigningKeyEpochV1::new(1).expect("issuer epoch"),
        };
        let claim_digest =
            EffectConflictClaimDigest::hash_canonical(&claim_preimage).expect("claim digest");
        let claim_wire = signed_wire(
            &claim_preimage,
            "canonical_effect_conflict_claim_sha256",
            &claim_digest,
        );
        let signed = SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &claim_preimage.schema_sha256,
            signing_authority: &request_preimage.signing_authority_binding,
        };
        let claim = EffectConflictClaimV1::admit_initial_wire(
            claim_wire,
            &EffectConflictClaimAdmissionContextV1 {
                request: &request,
                action: &action,
                signed,
            },
            |_, _, _| Ok(()),
        )
        .expect("admitted claim");
        let admission_ref = record_ref(&format!("admission-{label}"));
        let admission_preimage = EffectAdmissionPreimageV1 {
            schema_id: SchemaId::parse(EFFECT_ADMISSION_V1_SCHEMA).expect("admission schema"),
            schema_sha256: SchemaDigest::parse(sha('c')).expect("admission schema digest"),
            deployment_runtime_authority_ref: runtime_authority
                .deployment_runtime_authority_ref
                .clone(),
            deployment_runtime_authority_sha256: runtime_authority
                .deployment_runtime_authority_sha256
                .clone(),
            effect_admission_id: EffectAdmissionIdV1::parse(format!("admission-{label}"))
                .expect("admission id"),
            request_id: request_preimage.request_id.clone(),
            canonical_request_sha256: request.canonical_request_sha256().clone(),
            action_id: action.action_id().clone(),
            action_body_sha256: action_digest.clone(),
            conflict_resource_key_schema_id: action.conflict_resource_key_schema_id().clone(),
            conflict_resource_key_schema_sha256: action
                .conflict_resource_key_schema_sha256()
                .clone(),
            conflict_resource_key: action.conflict_resource_key().clone(),
            conflict_claim_ref: claim_ref.clone(),
            conflict_claim_sha256: claim.canonical_effect_conflict_claim_sha256().clone(),
            authority_evidence: Vec::new(),
            risk_class: request_preimage.risk_class.clone(),
            effect_policy: action.effect_policy().clone(),
            reconciliation_policy: action.reconciliation_policy().clone(),
            decision: EffectAdmissionDecisionV1::Admitted,
            issued_at: UnixTimeV1::new(11).expect("admission time"),
            expires_at: UnixTimeV1::new(90).expect("admission expiry"),
            issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
            issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
            issuer_key_epoch: SigningKeyEpochV1::new(1).expect("issuer epoch"),
        };
        let admission_digest =
            EffectAdmissionDigest::hash_canonical(&admission_preimage).expect("admission digest");
        let admission_wire = signed_wire(
            &admission_preimage,
            "canonical_effect_admission_sha256",
            &admission_digest,
        );
        let admission = EffectAdmissionV1::admit_contextual_wire(
            admission_wire,
            &EffectAdmissionContextV1 {
                request: &request,
                action: &action,
                claim_ref: &claim_ref,
                claim: &claim,
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &admission_preimage.schema_sha256,
                    signing_authority: &request_preimage.signing_authority_binding,
                },
            },
            |_, _, _| Ok(()),
        )
        .expect("admitted effect admission");
        let intent_ref = record_ref(&format!("intent-{label}"));
        let intent_preimage = EffectDispatchIntentPreimageV1 {
            schema_id: SchemaId::parse(EFFECT_DISPATCH_INTENT_V1_SCHEMA).expect("intent schema"),
            schema_sha256: SchemaDigest::parse(sha('d')).expect("intent schema digest"),
            deployment_runtime_authority_ref: runtime_authority
                .deployment_runtime_authority_ref
                .clone(),
            deployment_runtime_authority_sha256: runtime_authority
                .deployment_runtime_authority_sha256
                .clone(),
            effect_dispatch_intent_id: EffectDispatchIntentIdV1::parse(format!("intent-{label}"))
                .expect("intent id"),
            attempt_id: CapabilityAttemptIdV1::parse(format!("attempt-{label}")).expect("attempt"),
            request_ref: request_ref.clone(),
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
            execution_owner_fence_ref: None,
            execution_owner_fence_sha256: None,
            provider_request_digest: None,
            reserved_reconciliation_case_id: ReconciliationCaseIdV1::parse(format!("case-{label}"))
                .expect("case id"),
            reserved_reconciliation_custody_id: ReconciliationCustodyIdV1::parse(format!(
                "custody-{label}"
            ))
            .expect("custody id"),
            implementation_kind: BoundedResidentTextV1::parse("native").expect("implementation"),
            implementation_version: BoundedResidentTextV1::parse("1").expect("version"),
            intended_at: UnixTimeV1::new(12).expect("intent time"),
            issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
            issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
            issuer_key_epoch: SigningKeyEpochV1::new(1).expect("issuer epoch"),
        };
        let intent_digest =
            EffectDispatchIntentDigest::hash_canonical(&intent_preimage).expect("intent digest");
        let intent_wire = signed_wire(
            &intent_preimage,
            "canonical_dispatch_intent_sha256",
            &intent_digest,
        );
        let intent = EffectDispatchIntentV1::admit_contextual_wire(
            intent_wire,
            &EffectDispatchIntentAdmissionContextV1 {
                request_ref: &request_ref,
                request: &request,
                action: &action,
                admission_ref: &admission_ref,
                admission: &admission,
                claim_ref: &claim_ref,
                claim: &claim,
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &intent_preimage.schema_sha256,
                    signing_authority: &request_preimage.signing_authority_binding,
                },
            },
            |_| None,
            |_, _, _| Ok(()),
        )
        .expect("admitted intent");
        Self {
            request_preimage,
            request_ref,
            request,
            request_binding,
            action_ref,
            action,
            claim_preimage,
            claim_ref,
            claim,
            admission_ref,
            admission,
            intent_ref,
            intent,
        }
    }
}

fn typed_state(label: &str) -> TypedDigestV1 {
    let value = serde_json::json!({"state": label});
    let canonical = pale_core::canon::to_canonical_string(&value).expect("canonical state");
    TypedDigestV1::admit(
        TypedDigestPreimageV1 {
            kind: BoundedResidentTextV1::parse("state").expect("digest kind"),
            content_schema_id: SchemaId::parse("example/state.v1").expect("state schema"),
            content_schema_sha256: SchemaDigest::parse(sha('7')).expect("state schema digest"),
            canonicalization: TypedDigestCanonicalizationV1::Rfc8785Json,
            digest_algorithm: TypedDigestAlgorithmV1::Sha256,
            digest: ArtifactDigest::hash_bytes(canonical.as_bytes()),
            content_binding: TypedDigestContentBindingV1::InlineJson { inline_json: value },
        },
        |_| None,
    )
    .expect("typed state")
}

pub(super) struct OpenReconciliationFixture {
    pub(super) effect: EffectFixture,
    pub(super) receipt_ref: ResidentRecordRefV1,
    pub(super) receipt: EffectReceiptV1,
    pub(super) case_ref: ResidentRecordRefV1,
    pub(super) case: ReconciliationCaseV1,
    pub(super) custody_ref: ResidentRecordRefV1,
    pub(super) custody: ReconciliationCustodyV1,
    pub(super) transferred_claim_ref: ResidentRecordRefV1,
    pub(super) transferred_claim: EffectConflictClaimV1,
}

impl OpenReconciliationFixture {
    pub(super) fn new(label: &str) -> Self {
        let effect = EffectFixture::new(label);
        let authority = &effect.request_preimage.signing_authority_binding;
        let runtime_authority = authority
            .integrated_runtime_authority()
            .expect("integrated runtime authority");
        let receipt_ref = record_ref(&format!("receipt-{label}"));
        let receipt_schema_sha256 = SchemaDigest::parse(sha('8')).expect("receipt schema digest");
        let receipt_preimage = EffectReceiptPreimageV1 {
            schema_id: SchemaId::parse(EFFECT_RECEIPT_V1_SCHEMA).expect("receipt schema"),
            schema_sha256: receipt_schema_sha256.clone(),
            request_binding: effect.request_binding.clone(),
            action_id: effect.action.action_id().clone(),
            attempt_id: effect.intent.attempt_id().clone(),
            effect_dispatch_intent_ref: effect.intent_ref.clone(),
            effect_dispatch_intent_sha256: effect.intent.canonical_dispatch_intent_sha256().clone(),
            principal_scope: effect.request_preimage.scope.principal_scope.clone(),
            controller_scope: effect.request_preimage.scope.controller_scope.clone(),
            relationship_scope: effect.request_preimage.scope.relationship_scope.clone(),
            transport_status: BoundedResidentTextV1::parse("accepted").expect("transport"),
            observed_effect: ObservedEffectV1::Unknown,
            external_reference: None,
            observed_postcondition_digest: None,
            absence_evidence_ref: None,
            absence_evidence_sha256: None,
            action_body_sha256: effect
                .action
                .canonical_action_body_sha256()
                .expect("action digest"),
            effect_admission_ref: effect.admission_ref.clone(),
            effect_admission_sha256: effect.admission.canonical_effect_admission_sha256().clone(),
            conflict_claim_ref: effect.claim_ref.clone(),
            conflict_claim_sha256: effect
                .claim
                .canonical_effect_conflict_claim_sha256()
                .clone(),
            provider_request_digest: None,
            response_digest: None,
            implementation_kind: effect.intent.implementation_kind().clone(),
            implementation_version: effect.intent.implementation_version().clone(),
            reconciliation_required: true,
            reconciliation_case_id: Some(effect.intent.reserved_reconciliation_case_id().clone()),
            started_at: UnixTimeV1::new(13).expect("started time"),
            completed_at: UnixTimeV1::new(14).expect("completed time"),
            deployment_runtime_authority_ref: runtime_authority
                .deployment_runtime_authority_ref
                .clone(),
            deployment_runtime_authority_sha256: runtime_authority
                .deployment_runtime_authority_sha256
                .clone(),
            issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
            issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
            issuer_key_epoch: SigningKeyEpochV1::new(1).expect("issuer epoch"),
        };
        let receipt_digest =
            EffectReceiptDigest::hash_canonical(&receipt_preimage).expect("receipt digest");
        let receipt_wire = signed_wire(
            &receipt_preimage,
            "canonical_receipt_sha256",
            &receipt_digest,
        );
        let receipt = EffectReceiptV1::admit_contextual_wire(
            receipt_wire,
            &EffectReceiptAdmissionContextV1 {
                request: &effect.request,
                request_binding: &effect.request_binding,
                action: &effect.action,
                dispatch_ref: &effect.intent_ref,
                dispatch: &effect.intent,
                admission_ref: &effect.admission_ref,
                admission: &effect.admission,
                claim_ref: &effect.claim_ref,
                claim: &effect.claim,
                policy_requires_reconciliation: false,
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &receipt_schema_sha256,
                    signing_authority: authority,
                },
            },
            |_| None,
            |_, _, _| Ok(()),
        )
        .expect("admitted ambiguous receipt");

        let case_ref = record_ref(&format!("case-{label}"));
        let case_schema_sha256 = SchemaDigest::parse(sha('a')).expect("case schema digest");
        let expected_state = typed_state(label);
        let reconciliation_policy_sha256 = artifact('b');
        let case_preimage = ReconciliationCasePreimageV1 {
            schema_id: SchemaId::parse(RECONCILIATION_CASE_V1_SCHEMA).expect("case schema"),
            schema_sha256: case_schema_sha256.clone(),
            case_id: effect.intent.reserved_reconciliation_case_id().clone(),
            action_id: effect.action.action_id().clone(),
            effect_admission_ref: effect.admission_ref.clone(),
            effect_admission_sha256: effect.admission.canonical_effect_admission_sha256().clone(),
            conflict_resource_key_schema_id: effect
                .action
                .conflict_resource_key_schema_id()
                .clone(),
            conflict_resource_key_schema_sha256: effect
                .action
                .conflict_resource_key_schema_sha256()
                .clone(),
            conflict_resource_key: effect.action.conflict_resource_key().clone(),
            reserved_custody_id: effect.intent.reserved_reconciliation_custody_id().clone(),
            case_origin: ReconciliationCaseOriginV1::Initial,
            opening_evidence: vec![ReconciliationOpeningEvidenceV1::EffectReceipt {
                evidence_ref: receipt_ref.clone(),
                evidence_sha256: ArtifactDigest::parse(receipt.canonical_receipt_sha256().as_str())
                    .expect("receipt artifact digest"),
                evidence_schema_id: receipt.schema_id().clone(),
                evidence_schema_sha256: receipt.schema_sha256().clone(),
            }],
            expected_state_digest: expected_state.clone(),
            reconciliation_policy_ref: effect.action.reconciliation_policy().clone(),
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
            case_revision: ReconciliationCaseRevisionV1::new(0).expect("opening revision"),
            opened_at: UnixTimeV1::new(15).expect("opened time"),
            deployment_runtime_authority_ref: runtime_authority
                .deployment_runtime_authority_ref
                .clone(),
            deployment_runtime_authority_sha256: runtime_authority
                .deployment_runtime_authority_sha256
                .clone(),
            issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
            issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
            issuer_key_epoch: SigningKeyEpochV1::new(1).expect("issuer epoch"),
        };
        let case_digest =
            ReconciliationCaseDigest::hash_canonical(&case_preimage).expect("case digest");
        let case_wire = signed_wire(&case_preimage, "canonical_case_sha256", &case_digest);
        let case = ReconciliationCaseV1::admit_initial_wire(
            case_wire,
            &ReconciliationCaseInitialAdmissionContextV1 {
                receipt_ref: &receipt_ref,
                receipt: &receipt,
                dispatch_ref: &effect.intent_ref,
                dispatch: &effect.intent,
                admission_ref: &effect.admission_ref,
                admission: &effect.admission,
                action: &effect.action,
                expected_state_digest: &expected_state,
                reconciliation_policy_sha256: &reconciliation_policy_sha256,
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &case_schema_sha256,
                    signing_authority: authority,
                },
            },
            |_| None,
            |_, _, _| Ok(()),
        )
        .expect("admitted reconciliation case");

        let custody_ref = record_ref(&format!("custody-{label}"));
        let custody_schema_sha256 = SchemaDigest::parse(sha('c')).expect("custody schema digest");
        let custody_preimage = ReconciliationCustodyPreimageV1 {
            schema_id: SchemaId::parse(RECONCILIATION_CUSTODY_V1_SCHEMA).expect("custody schema"),
            schema_sha256: custody_schema_sha256.clone(),
            custody_id: effect.intent.reserved_reconciliation_custody_id().clone(),
            case_ref: case_ref.clone(),
            case_sha256: case.canonical_case_sha256().clone(),
            conflict_resource_key_schema_id: effect
                .action
                .conflict_resource_key_schema_id()
                .clone(),
            conflict_resource_key_schema_sha256: effect
                .action
                .conflict_resource_key_schema_sha256()
                .clone(),
            conflict_resource_key: effect.action.conflict_resource_key().clone(),
            custody_generation: ReconciliationCustodyGenerationV1::new(1)
                .expect("custody generation"),
            custody_state: ReconciliationCustodyStateV1::Active,
            holder_principal_id: effect.request_preimage.scope.principal_scope.clone(),
            holder_node_id: None,
            previous_custody_ref: None,
            previous_custody_sha256: None,
            terminal_result_ref: None,
            terminal_result_sha256: None,
            transferred_or_closed_at: None,
            deployment_runtime_authority_ref: runtime_authority
                .deployment_runtime_authority_ref
                .clone(),
            deployment_runtime_authority_sha256: runtime_authority
                .deployment_runtime_authority_sha256
                .clone(),
            issuer_principal_id: ResidentPrincipalIdV1::parse("principal-1").expect("issuer"),
            issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
            issuer_key_epoch: SigningKeyEpochV1::new(1).expect("issuer epoch"),
        };
        let custody_digest =
            ReconciliationCustodyDigest::hash_canonical(&custody_preimage).expect("custody digest");
        let custody_wire = signed_wire(
            &custody_preimage,
            "canonical_custody_sha256",
            &custody_digest,
        );
        let custody = ReconciliationCustodyV1::admit_initial_wire(
            custody_wire,
            &case_ref,
            &case,
            &custody_schema_sha256,
            authority,
            |_, _, _| Ok(()),
        )
        .expect("admitted initial custody");

        let transferred_claim_ref = record_ref(&format!("claim-transferred-{label}"));
        let mut transferred_preimage = effect.claim_preimage.clone();
        transferred_preimage.claim_generation =
            EffectConflictClaimGenerationV1::new(2).expect("successor claim generation");
        transferred_preimage.previous_claim_ref = Some(effect.claim_ref.clone());
        transferred_preimage.previous_claim_sha256 = Some(
            effect
                .claim
                .canonical_effect_conflict_claim_sha256()
                .clone(),
        );
        transferred_preimage.claim_state = EffectConflictClaimStateV1::Transferred;
        transferred_preimage.terminal_reason =
            Some(EffectConflictTerminalReasonV1::ReconciliationCustody);
        transferred_preimage.reconciliation_custody_ref = Some(custody_ref.clone());
        transferred_preimage.reconciliation_custody_sha256 =
            Some(custody.canonical_custody_sha256().clone());
        transferred_preimage.transferred_or_released_at =
            Some(UnixTimeV1::new(16).expect("transfer time"));
        let transferred_digest = EffectConflictClaimDigest::hash_canonical(&transferred_preimage)
            .expect("transferred claim digest");
        let transferred_wire = signed_wire(
            &transferred_preimage,
            "canonical_effect_conflict_claim_sha256",
            &transferred_digest,
        );
        let transferred_claim = EffectConflictClaimV1::admit_successor_wire(
            transferred_wire,
            &EffectConflictClaimSuccessorContextV1::CustodyTransfer {
                predecessor: EffectConflictClaimPredecessorV1 {
                    predecessor_ref: &effect.claim_ref,
                    predecessor: &effect.claim,
                    signed: SignedResidentAdmissionContextV1 {
                        expected_schema_sha256: &effect.claim_preimage.schema_sha256,
                        signing_authority: authority,
                    },
                },
                case_ref: &case_ref,
                case: &case,
                custody_ref: &custody_ref,
                custody: &custody,
            },
            |_, _, _| Ok(()),
        )
        .expect("admitted transferred claim");

        Self {
            effect,
            receipt_ref,
            receipt,
            case_ref,
            case,
            custody_ref,
            custody,
            transferred_claim_ref,
            transferred_claim,
        }
    }
}

pub(super) struct ReconciliationResultFixture {
    pub(super) open: OpenReconciliationFixture,
    pub(super) result_ref: ResidentRecordRefV1,
    pub(super) result: ReconciliationResultV1,
}

impl ReconciliationResultFixture {
    pub(super) fn new(label: &str) -> Self {
        let open = OpenReconciliationFixture::new(label);
        let effect = &open.effect;
        let receipt = ReconciliationEffectReceiptRefV1 {
            receipt_ref: open.receipt_ref.clone(),
            receipt_sha256: open.receipt.canonical_receipt_sha256().clone(),
        };
        let requested_at = UnixTimeV1::new(17).expect("reconciliation request time");
        let request_preimage = ReconciliationRequestPreimageV1 {
            reconciliation_id: ReconciliationIdV1::parse(format!("reconciliation-{label}"))
                .expect("reconciliation identity"),
            case_ref: open.case_ref.clone(),
            case_sha256: open.case.canonical_case_sha256().clone(),
            custody_ref: open.custody_ref.clone(),
            custody_sha256: open.custody.canonical_custody_sha256().clone(),
            expected_custody_generation: *open.custody.custody_generation(),
            expected_case_revision: ReconciliationCaseRevisionV1::new(0)
                .expect("initial expected revision"),
            predecessor_result_ref: None,
            predecessor_result_sha256: None,
            action_id: effect.action.action_id().clone(),
            effect_admission_ref: effect.admission_ref.clone(),
            effect_admission_sha256: effect.admission.canonical_effect_admission_sha256().clone(),
            effect_receipts: vec![receipt.clone()],
            expected_state_digest: open.case.expected_state_digest().clone(),
            reconciliation_policy_ref: open.case.reconciliation_policy_ref().clone(),
            reconciliation_policy_sha256: open.case.reconciliation_policy_sha256().clone(),
            observation_bounds: serde_json::json!({"maximum_observations": 1}),
            requested_at,
        };
        let request_wire: ReconciliationRequestWireV1 = serde_json::from_value(
            serde_json::to_value(&request_preimage).expect("reconciliation request preimage"),
        )
        .expect("reconciliation request wire");
        let mut outer_preimage = effect.request_preimage.clone();
        outer_preimage.request_id =
            ResidentRequestIdV1::parse(format!("reconciliation-request-{label}"))
                .expect("request identity");
        outer_preimage.capability_id =
            CapabilityIdV1::parse("reconcile").expect("reconcile capability");
        outer_preimage.scope.purpose =
            BoundedResidentTextV1::parse("reconcile").expect("reconcile purpose");
        outer_preimage.issued_at = UnixTimeV1::new(16).expect("request issue time");
        outer_preimage.expires_at = UnixTimeV1::new(90).expect("request expiry");
        let payload = serde_json::to_value(&request_preimage).expect("reconciliation payload");
        let canonical = pale_core::canon::to_canonical_string(&payload)
            .expect("canonical reconciliation payload");
        outer_preimage.body = CommonRequestBodyV1 {
            schema_id: SchemaId::parse(RECONCILIATION_REQUEST_V1_SCHEMA)
                .expect("reconciliation body schema"),
            schema_sha256: SchemaDigest::parse(sha('6')).expect("body schema digest"),
            payload_sha256: ArtifactDigest::hash_bytes(canonical.as_bytes()),
            payload,
        };
        let request = request(&outer_preimage);
        let reconciliation_request = ReconciliationRequestV1::admit_contextual_wire(
            request_wire,
            &ReconciliationRequestAdmissionContextV1 {
                outer_request: &request,
                case_ref: &open.case_ref,
                case: &open.case,
                custody_ref: &open.custody_ref,
                custody: &open.custody,
                predecessor: None,
                intervening_custody: &[],
            },
            |_| None,
        )
        .expect("admitted reconciliation request");
        let result_schema_sha256 = SchemaDigest::parse(sha('5')).expect("result schema digest");
        let runtime_authority = effect
            .request_preimage
            .signing_authority_binding
            .integrated_runtime_authority()
            .expect("integrated runtime authority");
        let result_preimage = ReconciliationResultPreimageV1 {
            schema_id: SchemaId::parse(RECONCILIATION_RESULT_V1_SCHEMA).expect("result schema"),
            schema_sha256: result_schema_sha256.clone(),
            request_binding: request
                .local_request_binding()
                .expect("local reconciliation request binding"),
            reconciliation_id: reconciliation_request.reconciliation_id().clone(),
            case_ref: open.case_ref.clone(),
            case_sha256: open.case.canonical_case_sha256().clone(),
            custody_ref: open.custody_ref.clone(),
            custody_sha256: open.custody.canonical_custody_sha256().clone(),
            custody_generation: *open.custody.custody_generation(),
            case_revision: ReconciliationCaseRevisionV1::new(1).expect("initial result revision"),
            predecessor_result_ref: None,
            predecessor_result_sha256: None,
            action_id: effect.action.action_id().clone(),
            effect_admission_ref: effect.admission_ref.clone(),
            effect_admission_sha256: effect.admission.canonical_effect_admission_sha256().clone(),
            effect_receipts: vec![receipt],
            principal_scope: effect.request.scope().principal_scope.clone(),
            controller_scope: effect.request.scope().controller_scope.clone(),
            relationship_scope: effect.request.scope().relationship_scope.clone(),
            reconciliation_policy_ref: open.case.reconciliation_policy_ref().clone(),
            reconciliation_policy_sha256: open.case.reconciliation_policy_sha256().clone(),
            expected_state_digest: open.case.expected_state_digest().clone(),
            observed_state_digest: Some(open.case.expected_state_digest().clone()),
            observation_evidence: vec![ReconciliationObservationEvidenceV1 {
                evidence_ref: record_ref(&format!("native-observation-{label}")),
                evidence_sha256: artifact('4'),
            }],
            absence_evidence_ref: None,
            absence_evidence_sha256: None,
            known_attempt_ids: vec![effect.intent.attempt_id().clone()],
            case_state: ReconciliationCaseStateV1::Matched,
            resolution: Some(ReconciliationResolutionV1::EffectConfirmed),
            compensation: None,
            owner_evidence_ref: None,
            owner_evidence_sha256: None,
            reviewer_or_policy_ref: record_ref(&format!("reconciliation-policy-{label}")),
            reviewer_or_policy_sha256: artifact('3'),
            requested_at,
            recorded_at: UnixTimeV1::new(18).expect("result record time"),
            closed_at: Some(UnixTimeV1::new(19).expect("result close time")),
            deployment_runtime_authority_ref: runtime_authority
                .deployment_runtime_authority_ref
                .clone(),
            deployment_runtime_authority_sha256: runtime_authority
                .deployment_runtime_authority_sha256
                .clone(),
            issuer_principal_id: effect.request_preimage.scope.principal_scope.clone(),
            issuer_key_id: ResidentKeyIdV1::parse("key-1").expect("issuer key"),
            issuer_key_epoch: SigningKeyEpochV1::new(1).expect("issuer epoch"),
        };
        let digest =
            ReconciliationResultDigest::hash_canonical(&result_preimage).expect("result digest");
        let wire = signed_wire(&result_preimage, "canonical_reconciliation_sha256", &digest);
        let result = ReconciliationResultV1::admit_initial_wire(
            wire,
            &ReconciliationResultInitialAdmissionContextV1 {
                outer_request: &request,
                request: &reconciliation_request,
                request_binding: &result_preimage.request_binding,
                compensation: None,
                signed: SignedResidentAdmissionContextV1 {
                    expected_schema_sha256: &result_schema_sha256,
                    signing_authority: &effect.request_preimage.signing_authority_binding,
                },
            },
            |_| None,
            |_, _, _| Ok(()),
        )
        .expect("admitted reconciliation result");
        Self {
            open,
            result_ref: record_ref(&format!("reconciliation-result-{label}")),
            result,
        }
    }
}
