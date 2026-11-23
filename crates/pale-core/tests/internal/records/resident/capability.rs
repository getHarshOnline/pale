//! Law tests for signed capability results and bounded adapter failures.

use super::*;
use serde::{de::DeserializeOwned, Serialize};
use std::cell::Cell;

#[path = "capability/issuance.rs"]
mod issuance;

fn hex(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn schema_digest(byte: char) -> SchemaDigest {
    SchemaDigest::parse(hex(byte)).expect("valid schema digest")
}

fn artifact_digest(byte: char) -> ArtifactDigest {
    ArtifactDigest::parse(hex(byte)).expect("valid artifact digest")
}

fn resident_ref(value: &str) -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse(value).expect("valid resident record reference")
}

fn signature() -> CanonicalSignatureV1 {
    CanonicalSignatureV1::parse("A".repeat(86)).expect("valid signature")
}

fn wire<T: Serialize, W: DeserializeOwned>(record: &T) -> W {
    serde_json::from_value(serde_json::to_value(record).expect("record serializes"))
        .expect("record shape decodes as its wire")
}

fn signing_authority() -> SigningAuthorityBindingV1 {
    signing_authority_for("producer-1", "producer-key-1")
}

fn signing_authority_for(principal: &str, key: &str) -> SigningAuthorityBindingV1 {
    serde_json::from_value(serde_json::json!({
        "authority_mode": "integrated_runtime",
        "principal_id": principal,
        "signer_key_id": key,
        "signer_key_epoch": 1,
        "integrated_runtime": {
            "deployment_runtime_authority_ref": "runtime-authority-1",
            "deployment_runtime_authority_sha256": hex('a'),
            "deployment_binding_ref": "deployment-1",
            "deployment_binding_sha256": hex('b'),
            "checked_principal_trust_checkpoint_ref": "trust-checkpoint-1",
            "checked_principal_trust_checkpoint_sha256": hex('c')
        }
    }))
    .expect("valid signing authority")
}

fn contract_set() -> ContractSetV1 {
    ContractSetV1::seal(ContractSetPreimageV1 {
        schema_id: SchemaId::parse(CONTRACT_SET_V1_SCHEMA).expect("contract-set schema"),
        schema_sha256: schema_digest('1'),
        contract_set_id: ContractSetIdV1::parse("contract-set-1").expect("contract-set id"),
        entries: vec![ContractSetEntryV1::new(
            SchemaId::parse("example/result.v1").expect("result schema id"),
            resident_ref("result-schema-1"),
            schema_digest('2'),
        )],
        compatibility_profile_ref: resident_ref("compatibility-profile-1"),
        compatibility_profile_sha256: CompatibilityProfileDigest::parse(hex('3'))
            .expect("compatibility profile digest"),
    })
    .expect("valid contract set")
}

fn request_binding(contract_set: &ContractSetV1) -> RequestBindingV1 {
    RequestBindingV1::new(RequestBindingPreimageV1 {
        request_id: ResidentRequestIdV1::parse("request-1").expect("request id"),
        correlation_id: ResidentCorrelationIdV1::parse("correlation-1").expect("correlation id"),
        capability_id: CapabilityIdV1::parse("example.read").expect("capability id"),
        canonical_request_sha256: CommonRequestEnvelopeDigest::parse(hex('4'))
            .expect("request digest"),
        session_grant_id: SessionGrantIdV1::parse("grant-1").expect("grant id"),
        session_grant_sha256: artifact_digest('5'),
        deployment_binding_ref: resident_ref("deployment-1"),
        deployment_binding_sha256: artifact_digest('6'),
        route_binding_schema_id: SchemaId::parse("example/route.v1").expect("route schema id"),
        route_binding_schema_sha256: schema_digest('7'),
        route_binding_ref: resident_ref("route-1"),
        route_binding_sha256: artifact_digest('8'),
        route_generation: RouteGenerationV1::new(1).expect("route generation"),
        contract_set_ref: resident_ref("contract-set-1"),
        contract_set_sha256: ArtifactDigest::parse(
            contract_set
                .canonical_contract_set_sha256()
                .as_str()
                .to_owned(),
        )
        .expect("contract-set artifact digest"),
        execution_owner_fence_ref: None,
        execution_owner_fence_sha256: None,
        federation_request_admission_ref: None,
        federation_request_admission_sha256: None,
    })
    .expect("valid request binding")
}

fn attempt_id() -> CapabilityAttemptIdV1 {
    CapabilityAttemptIdV1::parse("attempt-1").expect("attempt id")
}

fn implementation_kind() -> BoundedResidentTextV1 {
    BoundedResidentTextV1::parse("native_test").expect("implementation kind")
}

fn implementation_version() -> BoundedResidentTextV1 {
    BoundedResidentTextV1::parse("1").expect("implementation version")
}

struct AdmissionFixture {
    contracts: ContractSetV1,
    request: RequestBindingV1,
    authority: SigningAuthorityBindingV1,
    result_schema: SchemaDigest,
    failure_schema: SchemaDigest,
    attempt: CapabilityAttemptIdV1,
    implementation_kind: BoundedResidentTextV1,
    implementation_version: BoundedResidentTextV1,
    contracts_ref: ResidentRecordRefV1,
}

impl AdmissionFixture {
    fn new() -> Self {
        let contracts = contract_set();
        let request = request_binding(&contracts);
        Self {
            contracts,
            request,
            authority: signing_authority(),
            result_schema: schema_digest('9'),
            failure_schema: schema_digest('a'),
            attempt: attempt_id(),
            implementation_kind: implementation_kind(),
            implementation_version: implementation_version(),
            contracts_ref: resident_ref("contract-set-1"),
        }
    }

    fn result_context(&self) -> CapabilityResultAdmissionContextV1<'_> {
        CapabilityResultAdmissionContextV1 {
            expected_schema_sha256: &self.result_schema,
            expected_request_binding: &self.request,
            expected_attempt_id: &self.attempt,
            expected_implementation_kind: &self.implementation_kind,
            expected_implementation_version: &self.implementation_version,
            signing_authority: &self.authority,
        }
    }

    fn success_context(&self) -> CapabilitySuccessAdmissionContextV1<'_> {
        CapabilitySuccessAdmissionContextV1 {
            result: self.result_context(),
            contract_set_ref: &self.contracts_ref,
            contract_set: &self.contracts,
        }
    }

    fn failure_context(&self) -> AdapterFailureAdmissionContextV1<'_> {
        AdapterFailureAdmissionContextV1 {
            expected_schema_sha256: &self.failure_schema,
            expected_request_binding: &self.request,
            expected_attempt_id: &self.attempt,
            expected_implementation_kind: &self.implementation_kind,
            expected_implementation_version: &self.implementation_version,
            signing_authority: &self.authority,
        }
    }
}

fn success_payload() -> CapabilitySuccessV1 {
    let inline = serde_json::json!({"answer": 42});
    let canonical = crate::canon::to_canonical_string(&inline).expect("canonical payload");
    CapabilitySuccessV1 {
        result_schema_id: SchemaId::parse("example/result.v1").expect("result schema id"),
        result_schema_sha256: schema_digest('2'),
        result_payload_sha256: ArtifactDigest::hash_bytes(canonical.as_bytes()),
        inline_payload: Some(inline),
        immutable_result_ref: None,
    }
}

fn immutable_success(bytes: &[u8]) -> CapabilitySuccessV1 {
    CapabilitySuccessV1 {
        result_schema_id: SchemaId::parse("example/result.v1").expect("result schema id"),
        result_schema_sha256: schema_digest('2'),
        result_payload_sha256: ArtifactDigest::hash_bytes(bytes),
        inline_payload: None,
        immutable_result_ref: Some(resident_ref("immutable-result-1")),
    }
}

fn request_with(
    request: &RequestBindingV1,
    member: &str,
    value: serde_json::Value,
) -> RequestBindingV1 {
    request_with_members(request, &[(member, value)])
}

fn request_with_members(
    request: &RequestBindingV1,
    members: &[(&str, serde_json::Value)],
) -> RequestBindingV1 {
    let mut projected = serde_json::to_value(request).expect("request binding serializes");
    for (member, value) in members {
        projected[*member] = value.clone();
    }
    serde_json::from_value(projected).expect("transposed request binding remains well formed")
}

fn admit_inline_success(
    record: &CapabilityResultV1,
    context: &CapabilitySuccessAdmissionContextV1<'_>,
) -> Result<CapabilityResultV1, InvalidResidentRecordV1> {
    CapabilityResultV1::admit_success_wire(
        wire(record),
        context,
        |_| None,
        |_, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
}

fn admit_before_dispatch(
    record: &AdapterFailureV1,
    context: &AdapterFailureAdmissionContextV1<'_>,
) -> Result<AdapterFailureV1, InvalidResidentRecordV1> {
    AdapterFailureV1::admit_before_dispatch_wire(
        wire(record),
        context,
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
}

fn result_preimage(
    request_binding: RequestBindingV1,
    status: CapabilityResultStatusV1,
    success: Option<CapabilitySuccessV1>,
    failure: Option<CapabilityFailureV1>,
) -> CapabilityResultPreimageV1 {
    CapabilityResultPreimageV1 {
        schema_id: SchemaId::parse(CAPABILITY_RESULT_V1_SCHEMA).expect("result schema id"),
        schema_sha256: schema_digest('9'),
        request_binding,
        attempt_id: attempt_id(),
        status,
        success,
        failure,
        implementation_kind: implementation_kind(),
        implementation_version: implementation_version(),
        started_at: UnixTimeV1::new(10).expect("start time"),
        completed_at: UnixTimeV1::new(11).expect("completion time"),
        signing_authority_binding: signing_authority(),
        issuer_principal_id: ResidentPrincipalIdV1::parse("producer-1")
            .expect("producer principal"),
        issuer_key_id: ResidentKeyIdV1::parse("producer-key-1").expect("producer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("producer key epoch"),
    }
}

fn failure_preimage(
    request_binding: RequestBindingV1,
    effect_state: AdapterFailureEffectStateV1,
    absence_evidence_ref: Option<ResidentRecordRefV1>,
    absence_evidence_sha256: Option<ArtifactDigest>,
) -> AdapterFailurePreimageV1 {
    AdapterFailurePreimageV1 {
        schema_id: SchemaId::parse(ADAPTER_FAILURE_V1_SCHEMA).expect("failure schema id"),
        schema_sha256: schema_digest('a'),
        request_binding,
        failure_id: AdapterFailureIdV1::parse("failure-1").expect("failure id"),
        attempt_id: attempt_id(),
        failure_class: AdapterFailureClassV1::Unavailable,
        stable_code: ResidentReasonV1::parse("service_unavailable").expect("stable code"),
        bounded_message: Some(
            BoundedResidentTextV1::parse("selected service is unavailable")
                .expect("bounded message"),
        ),
        provider_status: None,
        retry_at: None,
        effect_state,
        absence_evidence_ref,
        absence_evidence_sha256,
        implementation_kind: implementation_kind(),
        implementation_version: implementation_version(),
        recorded_at: UnixTimeV1::new(10).expect("recorded time"),
        signing_authority_binding: signing_authority(),
        issuer_principal_id: ResidentPrincipalIdV1::parse("producer-1")
            .expect("producer principal"),
        issuer_key_id: ResidentKeyIdV1::parse("producer-key-1").expect("producer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("producer key epoch"),
    }
}

#[test]
fn successful_result_admits_one_contract_selected_inline_payload() {
    let fixture = AdmissionFixture::new();
    let record = CapabilityResultV1::seal(
        result_preimage(
            fixture.request.clone(),
            CapabilityResultStatusV1::Succeeded,
            Some(success_payload()),
            None,
        ),
        signature(),
    )
    .expect("valid successful result");
    let mut admitted_payload = false;
    let admitted = CapabilityResultV1::admit_success_wire(
        wire(&record),
        &fixture.success_context(),
        |_| None,
        |schema_id, schema_sha256, bytes| {
            assert_eq!(schema_id.as_str(), "example/result.v1");
            assert_eq!(schema_sha256, &schema_digest('2'));
            assert_eq!(bytes, br#"{"answer":42}"#);
            admitted_payload = true;
            Ok(())
        },
        |domain, _, digest, _| {
            assert_eq!(domain, CAPABILITY_RESULT_V1_SIGNATURE_DOMAIN);
            assert_eq!(digest, record.canonical_result_sha256());
            Ok(())
        },
    )
    .expect("successful result admits");
    assert!(admitted_payload);
    assert_eq!(admitted, record);
    assert!(CapabilityResultV1::admit_success_wire(
        wire(&record),
        &fixture.success_context(),
        |_| None,
        |_, _, _| Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "capability result payload",
            invariant: "selected result schema admission",
        }),
        |_, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn success_admission_rejects_every_transposed_transport_coordinate() {
    let fixture = AdmissionFixture::new();
    let record = CapabilityResultV1::seal(
        result_preimage(
            fixture.request.clone(),
            CapabilityResultStatusV1::Succeeded,
            Some(success_payload()),
            None,
        ),
        signature(),
    )
    .expect("valid successful result");

    let transposed_requests = [
        request_with(
            &fixture.request,
            "session_grant_id",
            serde_json::json!("grant-2"),
        ),
        request_with(
            &fixture.request,
            "route_binding_ref",
            serde_json::json!("route-2"),
        ),
        request_with(
            &fixture.request,
            "contract_set_ref",
            serde_json::json!("contract-set-2"),
        ),
        request_with_members(
            &fixture.request,
            &[
                (
                    "execution_owner_fence_ref",
                    serde_json::json!("execution-fence-2"),
                ),
                ("execution_owner_fence_sha256", serde_json::json!(hex('b'))),
            ],
        ),
        request_with_members(
            &fixture.request,
            &[
                (
                    "federation_request_admission_ref",
                    serde_json::json!("federation-admission-2"),
                ),
                (
                    "federation_request_admission_sha256",
                    serde_json::json!(hex('c')),
                ),
            ],
        ),
    ];
    for expected_request in &transposed_requests {
        let context = CapabilitySuccessAdmissionContextV1 {
            result: CapabilityResultAdmissionContextV1 {
                expected_schema_sha256: &fixture.result_schema,
                expected_request_binding: expected_request,
                expected_attempt_id: &fixture.attempt,
                expected_implementation_kind: &fixture.implementation_kind,
                expected_implementation_version: &fixture.implementation_version,
                signing_authority: &fixture.authority,
            },
            contract_set_ref: &fixture.contracts_ref,
            contract_set: &fixture.contracts,
        };
        assert!(admit_inline_success(&record, &context).is_err());
    }

    let wrong_schema = schema_digest('b');
    let wrong_attempt = CapabilityAttemptIdV1::parse("attempt-2").expect("attempt id");
    let wrong_kind = BoundedResidentTextV1::parse("other_kind").expect("implementation kind");
    let wrong_version = BoundedResidentTextV1::parse("2").expect("implementation version");
    let wrong_authority = signing_authority_for("producer-2", "producer-key-2");
    for result in [
        CapabilityResultAdmissionContextV1 {
            expected_schema_sha256: &wrong_schema,
            ..fixture.result_context()
        },
        CapabilityResultAdmissionContextV1 {
            expected_attempt_id: &wrong_attempt,
            ..fixture.result_context()
        },
        CapabilityResultAdmissionContextV1 {
            expected_implementation_kind: &wrong_kind,
            ..fixture.result_context()
        },
        CapabilityResultAdmissionContextV1 {
            expected_implementation_version: &wrong_version,
            ..fixture.result_context()
        },
        CapabilityResultAdmissionContextV1 {
            signing_authority: &wrong_authority,
            ..fixture.result_context()
        },
    ] {
        assert!(admit_inline_success(
            &record,
            &CapabilitySuccessAdmissionContextV1 {
                result,
                contract_set_ref: &fixture.contracts_ref,
                contract_set: &fixture.contracts,
            },
        )
        .is_err());
    }

    let mut issuer_swap = result_preimage(
        fixture.request.clone(),
        CapabilityResultStatusV1::Succeeded,
        Some(success_payload()),
        None,
    );
    issuer_swap.issuer_key_id = ResidentKeyIdV1::parse("producer-key-2").expect("producer key");
    let issuer_swap = CapabilityResultV1::seal(issuer_swap, signature())
        .expect("structurally valid transposed issuer record");
    assert!(
        admit_inline_success(&issuer_swap, &fixture.success_context()).is_err(),
        "issuer coordinates must remain bound to producer authority"
    );
}

#[test]
fn result_branch_and_representation_are_exact_sums() {
    let contracts = contract_set();
    let request = request_binding(&contracts);
    assert!(CapabilityResultV1::seal(
        result_preimage(
            request.clone(),
            CapabilityResultStatusV1::Succeeded,
            None,
            None,
        ),
        signature(),
    )
    .is_err());

    let mut both = success_payload();
    both.immutable_result_ref = Some(resident_ref("result-1"));
    assert!(CapabilityResultV1::seal(
        result_preimage(
            request,
            CapabilityResultStatusV1::Succeeded,
            Some(both),
            None,
        ),
        signature(),
    )
    .is_err());
}

#[test]
fn every_complementary_result_and_non_effect_branch_is_rejected() {
    let fixture = AdmissionFixture::new();
    let success = success_payload();
    let failure = CapabilityFailureV1 {
        failure_ref: resident_ref("failure-1"),
        failure_sha256: AdapterFailureDigest::parse(hex('d')).expect("failure digest"),
    };
    for (status, success, failure) in [
        (CapabilityResultStatusV1::Succeeded, None, None),
        (
            CapabilityResultStatusV1::Succeeded,
            Some(success.clone()),
            Some(failure.clone()),
        ),
        (
            CapabilityResultStatusV1::Succeeded,
            None,
            Some(failure.clone()),
        ),
        (CapabilityResultStatusV1::Failed, None, None),
        (
            CapabilityResultStatusV1::Failed,
            Some(success.clone()),
            Some(failure.clone()),
        ),
        (
            CapabilityResultStatusV1::Failed,
            Some(success.clone()),
            None,
        ),
    ] {
        assert!(CapabilityResultV1::seal(
            result_preimage(fixture.request.clone(), status, success, failure),
            signature(),
        )
        .is_err());
    }

    let reference = Some(resident_ref("absence-evidence-1"));
    let digest = Some(artifact_digest('e'));
    for (state, evidence_ref, evidence_digest) in [
        (
            AdapterFailureEffectStateV1::NotStarted,
            reference.clone(),
            None,
        ),
        (
            AdapterFailureEffectStateV1::NotStarted,
            None,
            digest.clone(),
        ),
        (
            AdapterFailureEffectStateV1::NotStarted,
            reference.clone(),
            digest.clone(),
        ),
        (
            AdapterFailureEffectStateV1::IndependentlyProvenNotApplied,
            None,
            None,
        ),
        (
            AdapterFailureEffectStateV1::IndependentlyProvenNotApplied,
            reference.clone(),
            None,
        ),
        (
            AdapterFailureEffectStateV1::IndependentlyProvenNotApplied,
            None,
            digest.clone(),
        ),
    ] {
        assert!(AdapterFailureV1::seal(
            failure_preimage(
                fixture.request.clone(),
                state,
                evidence_ref,
                evidence_digest,
            ),
            signature(),
        )
        .is_err());
    }
}

#[test]
fn immutable_success_requires_exact_bounded_bytes_and_selected_schema() {
    let fixture = AdmissionFixture::new();
    let bytes = b"owner-retained-result".to_vec();
    let record = CapabilityResultV1::seal(
        result_preimage(
            fixture.request.clone(),
            CapabilityResultStatusV1::Succeeded,
            Some(immutable_success(&bytes)),
            None,
        ),
        signature(),
    )
    .expect("valid immutable successful result");
    CapabilityResultV1::admit_success_wire(
        wire(&record),
        &fixture.success_context(),
        |reference| (reference.as_str() == "immutable-result-1").then(|| bytes.clone()),
        |schema_id, schema_sha256, admitted| {
            assert_eq!(schema_id.as_str(), "example/result.v1");
            assert_eq!(schema_sha256, &schema_digest('2'));
            assert_eq!(admitted, bytes);
            Ok(())
        },
        |_, _, _, _| Ok(()),
    )
    .expect("exact immutable result admits");

    for resolved in [None, Some(b"changed-owner-bytes".to_vec())] {
        assert!(CapabilityResultV1::admit_success_wire(
            wire(&record),
            &fixture.success_context(),
            |_| resolved.clone(),
            |_, _, _| Ok(()),
            |_, _, _, _| Ok(()),
        )
        .is_err());
    }

    let oversized = vec![b'x'; MAX_RESIDENT_CONTENT_BYTES + 1];
    let oversized_record = CapabilityResultV1::seal(
        result_preimage(
            fixture.request.clone(),
            CapabilityResultStatusV1::Succeeded,
            Some(immutable_success(&oversized)),
            None,
        ),
        signature(),
    )
    .expect("structurally valid oversized immutable reference");
    assert!(CapabilityResultV1::admit_success_wire(
        wire(&oversized_record),
        &fixture.success_context(),
        |_| Some(oversized.clone()),
        |_, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());

    let mut unselected = success_payload();
    unselected.result_schema_id =
        SchemaId::parse("example/unselected.v1").expect("unselected schema id");
    let unselected = CapabilityResultV1::seal(
        result_preimage(
            fixture.request.clone(),
            CapabilityResultStatusV1::Succeeded,
            Some(unselected),
            None,
        ),
        signature(),
    )
    .expect("structurally valid unselected schema");
    assert!(admit_inline_success(&unselected, &fixture.success_context()).is_err());

    let unsafe_integer = serde_json::json!(crate::canon::JSON_SAFE_INTEGER_MAX + 1);
    let mut unsafe_success = success_payload();
    unsafe_success.result_payload_sha256 =
        ArtifactDigest::hash_bytes(unsafe_integer.to_string().as_bytes());
    unsafe_success.inline_payload = Some(unsafe_integer);
    let unsafe_result = CapabilityResultV1::seal(
        result_preimage(
            fixture.request.clone(),
            CapabilityResultStatusV1::Succeeded,
            Some(unsafe_success),
            None,
        ),
        signature(),
    );
    assert!(unsafe_result.is_err());
}

#[test]
fn adapter_failure_enforces_the_two_non_effect_proof_branches() {
    let fixture = AdmissionFixture::new();
    let before_dispatch = AdapterFailureV1::seal(
        failure_preimage(
            fixture.request.clone(),
            AdapterFailureEffectStateV1::NotStarted,
            None,
            None,
        ),
        signature(),
    )
    .expect("valid pre-dispatch failure");
    AdapterFailureV1::admit_before_dispatch_wire(
        wire(&before_dispatch),
        &fixture.failure_context(),
        |class, retry_at, request, attempt| {
            assert_eq!(class, AdapterFailureClassV1::Unavailable);
            assert!(retry_at.is_none());
            assert_eq!(request, &fixture.request);
            assert_eq!(attempt, &fixture.attempt);
            Ok(())
        },
        |domain, _, digest, _| {
            assert_eq!(domain, ADAPTER_FAILURE_V1_SIGNATURE_DOMAIN);
            assert_eq!(digest, before_dispatch.canonical_failure_sha256());
            Ok(())
        },
    )
    .expect("pre-dispatch failure admits");

    let evidence = b"independent no-effect proof".to_vec();
    let evidence_digest = ArtifactDigest::hash_bytes(&evidence);
    let proven = AdapterFailureV1::seal(
        failure_preimage(
            fixture.request.clone(),
            AdapterFailureEffectStateV1::IndependentlyProvenNotApplied,
            Some(resident_ref("absence-evidence-1")),
            Some(evidence_digest.clone()),
        ),
        signature(),
    )
    .expect("valid proven-absence failure");
    AdapterFailureV1::admit_proven_not_applied_wire(
        wire(&proven),
        &fixture.failure_context(),
        |reference| (reference.as_str() == "absence-evidence-1").then(|| evidence.clone()),
        |reference, digest, bytes, admitted_request, admitted_attempt| {
            assert_eq!(reference.as_str(), "absence-evidence-1");
            assert_eq!(digest, &evidence_digest);
            assert_eq!(bytes, evidence);
            assert_eq!(admitted_request, &fixture.request);
            assert_eq!(admitted_attempt, &fixture.attempt);
            Ok(())
        },
        |_, _, request, attempt| {
            assert_eq!(request, &fixture.request);
            assert_eq!(attempt, &fixture.attempt);
            Ok(())
        },
        |domain, _, _, _| {
            assert_eq!(domain, ADAPTER_FAILURE_V1_SIGNATURE_DOMAIN);
            Ok(())
        },
    )
    .expect("proven-absence failure admits");

    let rejected = AdapterFailureV1::admit_proven_not_applied_wire(
        wire(&proven),
        &fixture.failure_context(),
        |_| Some(evidence.clone()),
        |_, _, _, _, _| {
            Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "adapter absence evidence",
                invariant: "proof binds the exact request and attempt",
            })
        },
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    );
    assert!(matches!(
        rejected,
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "adapter absence evidence",
            invariant: "proof binds the exact request and attempt"
        })
    ));

    for resolved in [None, Some(b"changed absence evidence".to_vec())] {
        assert!(AdapterFailureV1::admit_proven_not_applied_wire(
            wire(&proven),
            &fixture.failure_context(),
            |_| resolved.clone(),
            |_, _, _, _, _| Ok(()),
            |_, _, _, _| Ok(()),
            |_, _, _, _| Ok(()),
        )
        .is_err());
    }

    let oversized = vec![b'x'; MAX_RESIDENT_CONTENT_BYTES + 1];
    let oversized_failure = AdapterFailureV1::seal(
        failure_preimage(
            fixture.request.clone(),
            AdapterFailureEffectStateV1::IndependentlyProvenNotApplied,
            Some(resident_ref("absence-evidence-1")),
            Some(ArtifactDigest::hash_bytes(&oversized)),
        ),
        signature(),
    )
    .expect("structurally valid oversized evidence reference");
    assert!(AdapterFailureV1::admit_proven_not_applied_wire(
        wire(&oversized_failure),
        &fixture.failure_context(),
        |_| Some(oversized.clone()),
        |_, _, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
        |_, _, _, _| Ok(()),
    )
    .is_err());
}

#[test]
fn adapter_failure_rejects_every_transposed_transport_coordinate() {
    let fixture = AdmissionFixture::new();
    let failure = AdapterFailureV1::seal(
        failure_preimage(
            fixture.request.clone(),
            AdapterFailureEffectStateV1::NotStarted,
            None,
            None,
        ),
        signature(),
    )
    .expect("valid failure");
    let wrong_request = request_with(
        &fixture.request,
        "deployment_binding_ref",
        serde_json::json!("deployment-2"),
    );
    let wrong_schema = schema_digest('b');
    let wrong_attempt = CapabilityAttemptIdV1::parse("attempt-2").expect("attempt id");
    let wrong_kind = BoundedResidentTextV1::parse("other_kind").expect("implementation kind");
    let wrong_version = BoundedResidentTextV1::parse("2").expect("implementation version");
    let wrong_authority = signing_authority_for("producer-2", "producer-key-2");
    for context in [
        AdapterFailureAdmissionContextV1 {
            expected_request_binding: &wrong_request,
            ..fixture.failure_context()
        },
        AdapterFailureAdmissionContextV1 {
            expected_schema_sha256: &wrong_schema,
            ..fixture.failure_context()
        },
        AdapterFailureAdmissionContextV1 {
            expected_attempt_id: &wrong_attempt,
            ..fixture.failure_context()
        },
        AdapterFailureAdmissionContextV1 {
            expected_implementation_kind: &wrong_kind,
            ..fixture.failure_context()
        },
        AdapterFailureAdmissionContextV1 {
            expected_implementation_version: &wrong_version,
            ..fixture.failure_context()
        },
        AdapterFailureAdmissionContextV1 {
            signing_authority: &wrong_authority,
            ..fixture.failure_context()
        },
    ] {
        assert!(admit_before_dispatch(&failure, &context).is_err());
    }

    let mut issuer_swap = failure_preimage(
        fixture.request.clone(),
        AdapterFailureEffectStateV1::NotStarted,
        None,
        None,
    );
    issuer_swap.issuer_principal_id =
        ResidentPrincipalIdV1::parse("producer-2").expect("producer principal");
    let issuer_swap = AdapterFailureV1::seal(issuer_swap, signature())
        .expect("structurally valid transposed issuer record");
    assert!(admit_before_dispatch(&issuer_swap, &fixture.failure_context()).is_err());
}

#[test]
fn every_stable_failure_class_admits_only_through_explicit_capability_policy() {
    let fixture = AdmissionFixture::new();
    let classes = [
        AdapterFailureClassV1::Denied,
        AdapterFailureClassV1::Invalid,
        AdapterFailureClassV1::UnsupportedSchema,
        AdapterFailureClassV1::Conflict,
        AdapterFailureClassV1::Unavailable,
        AdapterFailureClassV1::RateLimited,
        AdapterFailureClassV1::Timeout,
        AdapterFailureClassV1::Unknown,
        AdapterFailureClassV1::Partial,
        AdapterFailureClassV1::Internal,
    ];
    for class in classes {
        let mut preimage = failure_preimage(
            fixture.request.clone(),
            AdapterFailureEffectStateV1::NotStarted,
            None,
            None,
        );
        preimage.failure_class = class;
        preimage.retry_at = (class == AdapterFailureClassV1::RateLimited)
            .then(|| UnixTimeV1::new(20).expect("retry time"));
        let failure = AdapterFailureV1::seal(preimage, signature()).expect("stable class seals");
        AdapterFailureV1::admit_before_dispatch_wire(
            wire(&failure),
            &fixture.failure_context(),
            |observed, retry_at, request, attempt| {
                assert_eq!(observed, class);
                assert_eq!(
                    retry_at.is_some(),
                    class == AdapterFailureClassV1::RateLimited
                );
                assert_eq!(request, &fixture.request);
                assert_eq!(attempt, &fixture.attempt);
                Ok(())
            },
            |_, _, _, _| Ok(()),
        )
        .expect("capability policy admits declared class");

        if matches!(
            class,
            AdapterFailureClassV1::Timeout | AdapterFailureClassV1::Partial
        ) {
            assert!(AdapterFailureV1::admit_before_dispatch_wire(
                wire(&failure),
                &fixture.failure_context(),
                |_, _, _, _| Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "adapter failure",
                    invariant: "capability-specific class policy",
                }),
                |_, _, _, _| Ok(()),
            )
            .is_err());
        }
    }
}

#[test]
fn rejected_signatures_suppress_all_resolution_and_policy_callbacks() {
    let fixture = AdmissionFixture::new();
    let reject = || InvalidResidentRecordV1::ContextualAdmission {
        record: "test signature",
        invariant: "rejected",
    };

    let bytes = b"owner-retained-result".to_vec();
    let success = CapabilityResultV1::seal(
        result_preimage(
            fixture.request.clone(),
            CapabilityResultStatusV1::Succeeded,
            Some(immutable_success(&bytes)),
            None,
        ),
        signature(),
    )
    .expect("successful result");
    let resolution_called = Cell::new(false);
    let payload_called = Cell::new(false);
    assert!(CapabilityResultV1::admit_success_wire(
        wire(&success),
        &fixture.success_context(),
        |_| {
            resolution_called.set(true);
            Some(bytes.clone())
        },
        |_, _, _| {
            payload_called.set(true);
            Ok(())
        },
        |_, _, _, _| Err(reject()),
    )
    .is_err());
    assert!(!resolution_called.get() && !payload_called.get());

    let failure = AdapterFailureV1::seal(
        failure_preimage(
            fixture.request.clone(),
            AdapterFailureEffectStateV1::NotStarted,
            None,
            None,
        ),
        signature(),
    )
    .expect("safe failure");
    let failed_result = CapabilityResultV1::seal(
        result_preimage(
            fixture.request.clone(),
            CapabilityResultStatusV1::Failed,
            None,
            Some(CapabilityFailureV1 {
                failure_ref: resident_ref("failure-1"),
                failure_sha256: failure.canonical_failure_sha256().clone(),
            }),
        ),
        signature(),
    )
    .expect("failed result");
    let failure_resolution_called = Cell::new(false);
    assert!(CapabilityResultV1::admit_failure_wire(
        wire(&failed_result),
        &fixture.result_context(),
        |_| {
            failure_resolution_called.set(true);
            Some(failure.clone())
        },
        |_, _, _, _| Err(reject()),
    )
    .is_err());
    assert!(!failure_resolution_called.get());

    let class_called = Cell::new(false);
    assert!(AdapterFailureV1::admit_before_dispatch_wire(
        wire(&failure),
        &fixture.failure_context(),
        |_, _, _, _| {
            class_called.set(true);
            Ok(())
        },
        |_, _, _, _| Err(reject()),
    )
    .is_err());
    assert!(!class_called.get());

    let evidence = b"independent no-effect proof".to_vec();
    let proven = AdapterFailureV1::seal(
        failure_preimage(
            fixture.request.clone(),
            AdapterFailureEffectStateV1::IndependentlyProvenNotApplied,
            Some(resident_ref("absence-evidence-1")),
            Some(ArtifactDigest::hash_bytes(&evidence)),
        ),
        signature(),
    )
    .expect("proven non-effect failure");
    let evidence_resolution_called = Cell::new(false);
    let evidence_policy_called = Cell::new(false);
    let proven_class_called = Cell::new(false);
    assert!(AdapterFailureV1::admit_proven_not_applied_wire(
        wire(&proven),
        &fixture.failure_context(),
        |_| {
            evidence_resolution_called.set(true);
            Some(evidence.clone())
        },
        |_, _, _, _, _| {
            evidence_policy_called.set(true);
            Ok(())
        },
        |_, _, _, _| {
            proven_class_called.set(true);
            Ok(())
        },
        |_, _, _, _| Err(reject()),
    )
    .is_err());
    assert!(
        !evidence_resolution_called.get()
            && !evidence_policy_called.get()
            && !proven_class_called.get()
    );
}

#[test]
fn failed_result_resolves_the_exact_signed_failure() {
    let fixture = AdmissionFixture::new();
    let failure = AdapterFailureV1::seal(
        failure_preimage(
            fixture.request.clone(),
            AdapterFailureEffectStateV1::NotStarted,
            None,
            None,
        ),
        signature(),
    )
    .expect("valid failure");
    let result = CapabilityResultV1::seal(
        result_preimage(
            fixture.request.clone(),
            CapabilityResultStatusV1::Failed,
            None,
            Some(CapabilityFailureV1 {
                failure_ref: resident_ref("failure-1"),
                failure_sha256: failure.canonical_failure_sha256().clone(),
            }),
        ),
        signature(),
    )
    .expect("valid failed result");
    CapabilityResultV1::admit_failure_wire(
        wire(&result),
        &fixture.result_context(),
        |reference| (reference.as_str() == "failure-1").then(|| failure.clone()),
        |domain, _, _, _| {
            assert_eq!(domain, CAPABILITY_RESULT_V1_SIGNATURE_DOMAIN);
            Ok(())
        },
    )
    .expect("failed result admits exact failure");
}

#[test]
fn failed_result_rejects_a_valid_failure_from_another_request_or_attempt() {
    let fixture = AdmissionFixture::new();
    let foreign_request = request_with(
        &fixture.request,
        "correlation_id",
        serde_json::json!("correlation-2"),
    );
    let request_failure = AdapterFailureV1::seal(
        failure_preimage(
            foreign_request,
            AdapterFailureEffectStateV1::NotStarted,
            None,
            None,
        ),
        signature(),
    )
    .expect("valid failure for another request");
    let mut attempt_preimage = failure_preimage(
        fixture.request.clone(),
        AdapterFailureEffectStateV1::NotStarted,
        None,
        None,
    );
    attempt_preimage.attempt_id = CapabilityAttemptIdV1::parse("attempt-2").expect("other attempt");
    let attempt_failure = AdapterFailureV1::seal(attempt_preimage, signature())
        .expect("valid failure for another attempt");

    for foreign in [request_failure, attempt_failure] {
        let result = CapabilityResultV1::seal(
            result_preimage(
                fixture.request.clone(),
                CapabilityResultStatusV1::Failed,
                None,
                Some(CapabilityFailureV1 {
                    failure_ref: resident_ref("failure-1"),
                    failure_sha256: foreign.canonical_failure_sha256().clone(),
                }),
            ),
            signature(),
        )
        .expect("structurally valid failed result");
        assert!(CapabilityResultV1::admit_failure_wire(
            wire(&result),
            &fixture.result_context(),
            |_| Some(foreign.clone()),
            |_, _, _, _| Ok(()),
        )
        .is_err());
    }
}

#[test]
fn wire_schema_projects_branch_laws_and_forbids_explicit_null() {
    let result_schema = serde_json::to_value(schemars::schema_for!(CapabilityResultWireV1))
        .expect("result schema serializes");
    assert_eq!(
        result_schema
            .pointer("/oneOf")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len),
        Some(2)
    );
    let success_schema = serde_json::to_value(schemars::schema_for!(CapabilitySuccessV1))
        .expect("success schema serializes");
    assert_eq!(
        success_schema
            .pointer("/oneOf")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len),
        Some(2)
    );
    let failure_schema = serde_json::to_value(schemars::schema_for!(AdapterFailureWireV1))
        .expect("failure schema serializes");
    assert_eq!(
        failure_schema
            .pointer("/oneOf")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len),
        Some(2)
    );
    assert_eq!(
        failure_schema.pointer("/allOf/0/if/properties/failure_class/const"),
        Some(&serde_json::json!("rate_limited"))
    );
    assert_eq!(
        failure_schema.pointer("/allOf/0/then/required"),
        Some(&serde_json::json!(["retry_at"]))
    );
    assert!(result_schema.pointer("/allOf").is_none());

    let contracts = contract_set();
    let record = CapabilityResultV1::seal(
        result_preimage(
            request_binding(&contracts),
            CapabilityResultStatusV1::Succeeded,
            Some(success_payload()),
            None,
        ),
        signature(),
    )
    .expect("valid result");
    let mut value = serde_json::to_value(record).expect("result serializes");
    value["failure"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<CapabilityResultWireV1>(value).is_err());

    let success = success_payload();
    let success_json = serde_json::to_value(success).expect("success serializes");
    for member in ["inline_payload", "immutable_result_ref"] {
        let mut null = success_json.clone();
        null[member] = serde_json::Value::Null;
        assert!(
            serde_json::from_value::<CapabilitySuccessV1>(null).is_err(),
            "explicit null admitted for {member}"
        );
    }

    let failure = AdapterFailureV1::seal(
        failure_preimage(
            request_binding(&contract_set()),
            AdapterFailureEffectStateV1::NotStarted,
            None,
            None,
        ),
        signature(),
    )
    .expect("valid failure");
    let failure_json = serde_json::to_value(failure).expect("failure serializes");
    for member in [
        "bounded_message",
        "provider_status",
        "retry_at",
        "absence_evidence_ref",
        "absence_evidence_sha256",
    ] {
        let mut null = failure_json.clone();
        null[member] = serde_json::Value::Null;
        assert!(
            serde_json::from_value::<AdapterFailureWireV1>(null).is_err(),
            "explicit null admitted for {member}"
        );
    }
    let mut unknown = failure_json;
    unknown["provider_debug_body"] = serde_json::json!("must not cross the boundary");
    assert!(serde_json::from_value::<AdapterFailureWireV1>(unknown).is_err());
}

#[test]
fn rate_limit_requires_retry_and_result_time_is_monotone() {
    let contracts = contract_set();
    let request = request_binding(&contracts);
    let mut rate_limited = failure_preimage(
        request.clone(),
        AdapterFailureEffectStateV1::NotStarted,
        None,
        None,
    );
    rate_limited.failure_class = AdapterFailureClassV1::RateLimited;
    assert!(AdapterFailureV1::seal(rate_limited, signature()).is_err());

    let mut inverted = result_preimage(
        request,
        CapabilityResultStatusV1::Succeeded,
        Some(success_payload()),
        None,
    );
    inverted.started_at = UnixTimeV1::new(12).expect("start time");
    inverted.completed_at = UnixTimeV1::new(11).expect("completion time");
    assert!(CapabilityResultV1::seal(inverted, signature()).is_err());
}

#[test]
fn canonical_preimage_vectors_are_stable_and_self_authentication_free() {
    let fixture = AdmissionFixture::new();
    let mut result = result_preimage(
        fixture.request.clone(),
        CapabilityResultStatusV1::Succeeded,
        Some(success_payload()),
        None,
    );
    let mut failure = failure_preimage(
        fixture.request,
        AdapterFailureEffectStateV1::NotStarted,
        None,
        None,
    );
    let result_fixture = crate::canon::parse_strict(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/resident/capability-result-preimage.v1.json"
    )))
    .expect("result fixture is strict JSON");
    let result_vector =
        crate::canon::to_canonical_string(&result_fixture).expect("result fixture canonicalizes");
    let failure_fixture = crate::canon::parse_strict(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/resident/adapter-failure-preimage.v1.json"
    )))
    .expect("failure fixture is strict JSON");
    let failure_vector =
        crate::canon::to_canonical_string(&failure_fixture).expect("failure fixture canonicalizes");
    assert_eq!(
        crate::canon::to_canonical_string(&result).expect("result canonicalizes"),
        result_vector
    );
    assert_eq!(
        crate::canon::to_canonical_string(&failure).expect("failure canonicalizes"),
        failure_vector
    );
    let reordered_result = result_vector.replacen(
        "{\"attempt_id\":\"attempt-1\",\"completed_at\":11",
        "{\"completed_at\":11,\"attempt_id\":\"attempt-1\"",
        1,
    );
    let reordered_failure = failure_vector.replacen(
        "{\"attempt_id\":\"attempt-1\",\"bounded_message\"",
        "{\"bounded_message\"",
        1,
    );
    let reordered_failure = reordered_failure.replacen(
        "\":\"selected service is unavailable\"",
        "\":\"selected service is unavailable\",\"attempt_id\":\"attempt-1\"",
        1,
    );
    assert_ne!(reordered_result, result_vector);
    assert_ne!(reordered_failure, failure_vector);
    assert_eq!(
        crate::canon::to_canonical_string(
            &crate::canon::parse_strict(reordered_result.as_bytes())
                .expect("reordered result parses"),
        )
        .expect("reordered result canonicalizes"),
        result_vector
    );
    assert_eq!(
        crate::canon::to_canonical_string(
            &crate::canon::parse_strict(reordered_failure.as_bytes())
                .expect("reordered failure parses"),
        )
        .expect("reordered failure canonicalizes"),
        failure_vector
    );
    assert!(!result_vector.contains("signature") && !result_vector.contains("canonical_result"));
    assert!(!failure_vector.contains("signature") && !failure_vector.contains("canonical_failure"));
    let result_digest = CapabilityResultV1::seal(result.clone(), signature())
        .expect("result seals")
        .canonical_result_sha256()
        .clone();
    let failure_digest = AdapterFailureV1::seal(failure.clone(), signature())
        .expect("failure seals")
        .canonical_failure_sha256()
        .clone();
    assert_eq!(
        result_digest.as_str(),
        "dccb9bcbb7cd0a5447652458a09d2f0cbb018962668b801d7a1a6188b5ea23fe"
    );
    assert_eq!(
        failure_digest.as_str(),
        "24b4abdd5315ee2053a5792ae86ea3b8f8c4a5c95005f87deca4504a0bd289c0"
    );
    let result_signature_message = [
        CAPABILITY_RESULT_V1_SIGNATURE_DOMAIN,
        result_digest.as_str().as_bytes(),
    ]
    .concat();
    let failure_signature_message = [
        ADAPTER_FAILURE_V1_SIGNATURE_DOMAIN,
        failure_digest.as_str().as_bytes(),
    ]
    .concat();
    assert_eq!(
        result_signature_message,
        b"PALE_CAPABILITY_RESULT_V1\x00dccb9bcbb7cd0a5447652458a09d2f0cbb018962668b801d7a1a6188b5ea23fe"
    );
    assert_eq!(
        failure_signature_message,
        b"PALE_ADAPTER_FAILURE_V1\x0024b4abdd5315ee2053a5792ae86ea3b8f8c4a5c95005f87deca4504a0bd289c0"
    );

    result.completed_at = UnixTimeV1::new(12).expect("completion time");
    failure.stable_code = ResidentReasonV1::parse("service_busy").expect("stable code");
    assert_ne!(
        CapabilityResultV1::seal(result, signature())
            .expect("mutated result seals")
            .canonical_result_sha256(),
        &result_digest
    );
    assert_ne!(
        AdapterFailureV1::seal(failure, signature())
            .expect("failure seals")
            .canonical_failure_sha256(),
        &failure_digest
    );
}
