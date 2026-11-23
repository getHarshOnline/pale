//! Provider test transport returns core-issued records, never private admitted values.
//! The fixed signature is a test model, not a cryptographic backend qualification.

use super::*;
use crate::local_operation_start_fixture::{artifact_digest, operation_id, sha, signature};
use crate::provider_invocation::*;
use pale_ports::*;
use std::cell::Cell;

pub(in crate::resident) struct ResultPort {
    observation: ProviderInvocationObservation,
    calls: Cell<u32>,
}

impl ResultPort {
    pub(in crate::resident) fn success(result: ProviderInvocationSuccessWire) -> Self {
        Self {
            observation: ProviderInvocationObservation::Succeeded(Box::new(result)),
            calls: Cell::new(0),
        }
    }

    pub(in crate::resident) fn failure(result: ProviderInvocationFailureWire) -> Self {
        Self {
            observation: ProviderInvocationObservation::Failed(Box::new(result)),
            calls: Cell::new(0),
        }
    }

    pub(in crate::resident) fn unavailable(reason: ProviderInvocationUnavailability) -> Self {
        Self {
            observation: ProviderInvocationObservation::Unavailable(reason),
            calls: Cell::new(0),
        }
    }

    pub(in crate::resident) fn calls(&self) -> u32 {
        self.calls.get()
    }
}

impl ProviderInvocationPort for ResultPort {
    fn invoke(&self, _: ProviderInvocationRequest<'_>) -> ProviderInvocationObservation {
        self.calls.set(self.calls.get() + 1);
        self.observation.clone()
    }
}

pub(in crate::resident) struct Evidence {
    reject_result_signature: bool,
    reject_failure_signature: bool,
}

impl Evidence {
    pub(in crate::resident) const fn valid() -> Self {
        Self {
            reject_result_signature: false,
            reject_failure_signature: false,
        }
    }

    pub(in crate::resident) const fn rejecting_result_signature() -> Self {
        Self {
            reject_result_signature: true,
            reject_failure_signature: false,
        }
    }

    pub(super) const fn rejecting_failure_signature() -> Self {
        Self {
            reject_result_signature: false,
            reject_failure_signature: true,
        }
    }
}

impl ProviderInvocationEvidenceAdmission for Evidence {
    fn admit_evidence_frontier(
        &self,
        frontier: &ProviderInvocationEvidenceFrontier,
    ) -> Result<(), InvalidResidentRecordV1> {
        assert_eq!(frontier.evidence_ref(), &record_ref("provider-evidence"));
        Ok(())
    }

    fn resolve_immutable(&self, _: &ResidentRecordRefV1) -> Option<Vec<u8>> {
        None
    }

    fn admit_payload(
        &self,
        schema: &SchemaId,
        digest: &SchemaDigest,
        bytes: &[u8],
    ) -> Result<(), InvalidResidentRecordV1> {
        assert_eq!(schema.as_str(), "example/result.v1");
        assert_eq!(digest.as_str(), sha('2'));
        assert_eq!(bytes, br#"{"accepted":true}"#);
        Ok(())
    }

    fn admit_failure_class(
        &self,
        class: AdapterFailureClassV1,
        retry_at: Option<UnixTimeV1>,
        request: &RequestBindingV1,
        attempt: &CapabilityAttemptIdV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        assert_eq!(class, AdapterFailureClassV1::Unavailable);
        assert_eq!(retry_at, None);
        assert_eq!(request.capability_id().as_str(), "read");
        assert_eq!(attempt.as_str(), "attempt-1");
        Ok(())
    }

    fn admit_absence_evidence(
        &self,
        _: &ResidentRecordRefV1,
        _: &ArtifactDigest,
        _: &[u8],
        _: &RequestBindingV1,
        _: &CapabilityAttemptIdV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        panic!("Compute success does not assert external-effect absence")
    }

    fn verify_result_signature(
        &self,
        domain: &[u8],
        preimage: &CapabilityResultPreimageV1,
        digest: &CapabilityResultDigest,
        actual: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        assert_eq!(domain, CAPABILITY_RESULT_V1_SIGNATURE_DOMAIN);
        assert_eq!(
            digest,
            &CapabilityResultDigest::hash_canonical(preimage).unwrap()
        );
        assert_eq!(actual, &signature());
        if self.reject_result_signature {
            return Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "capability result",
                invariant: "test result signature verifier rejects the provider evidence",
            });
        }
        Ok(())
    }

    fn verify_failure_signature(
        &self,
        domain: &[u8],
        preimage: &AdapterFailurePreimageV1,
        digest: &AdapterFailureDigest,
        actual: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        assert_eq!(domain, ADAPTER_FAILURE_V1_SIGNATURE_DOMAIN);
        assert_eq!(
            digest,
            &AdapterFailureDigest::hash_canonical(preimage).unwrap()
        );
        assert_eq!(actual, &signature());
        if self.reject_failure_signature {
            return Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "adapter failure",
                invariant: "test failure signature verifier rejects the provider evidence",
            });
        }
        Ok(())
    }
}

fn signed_wire<P, D, W>(preimage: &P, digest_field: &str, digest: &D) -> W
where
    P: serde::Serialize,
    D: serde::Serialize,
    W: serde::de::DeserializeOwned,
{
    let mut value = serde_json::to_value(preimage).unwrap();
    let object = value.as_object_mut().unwrap();
    object.insert(digest_field.into(), serde_json::to_value(digest).unwrap());
    object.insert(
        "signature".into(),
        serde_json::to_value(signature()).unwrap(),
    );
    serde_json::from_value(value).unwrap()
}

pub(in crate::resident) fn safe_failure(
    request: &CommonRequestEnvelopeV1,
    signing: &SigningAuthorityBindingV1,
    attempt: &CapabilityAttemptIdV1,
    coordinate: ProviderInvocationObservationCoordinate,
) -> ProviderInvocationFailureWire {
    safe_failure_at(
        request,
        signing,
        attempt,
        coordinate,
        UnixTimeV1::new(66).unwrap(),
        UnixTimeV1::new(70).unwrap(),
    )
}

pub(in crate::resident) fn safe_failure_at(
    request: &CommonRequestEnvelopeV1,
    signing: &SigningAuthorityBindingV1,
    attempt: &CapabilityAttemptIdV1,
    coordinate: ProviderInvocationObservationCoordinate,
    started_at: UnixTimeV1,
    completed_at: UnixTimeV1,
) -> ProviderInvocationFailureWire {
    let binding = request.local_request_binding().unwrap();
    let kind = BoundedResidentTextV1::parse("host_compute").unwrap();
    let version = BoundedResidentTextV1::parse("1").unwrap();
    let failure_schema = schema_digest(ADAPTER_FAILURE_V1_SCHEMA_ID);
    let result_schema = schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID);
    let failure_preimage = AdapterFailurePreimageV1 {
        schema_id: SchemaId::parse(ADAPTER_FAILURE_V1_SCHEMA).unwrap(),
        schema_sha256: failure_schema,
        request_binding: binding.clone(),
        failure_id: AdapterFailureIdV1::parse("provider-safe-failure").unwrap(),
        attempt_id: attempt.clone(),
        failure_class: AdapterFailureClassV1::Unavailable,
        stable_code: ResidentReasonV1::parse("provider_unavailable").unwrap(),
        bounded_message: None,
        provider_status: None,
        retry_at: None,
        effect_state: AdapterFailureEffectStateV1::NotStarted,
        absence_evidence_ref: None,
        absence_evidence_sha256: None,
        implementation_kind: kind.clone(),
        implementation_version: version.clone(),
        recorded_at: completed_at,
        signing_authority_binding: signing.clone(),
        issuer_principal_id: signing.principal_id().clone(),
        issuer_key_id: ResidentKeyIdV1::parse(signing.signer_key_id()).unwrap(),
        issuer_key_epoch: *signing.signer_key_epoch(),
    };
    let failure_digest = AdapterFailureDigest::hash_canonical(&failure_preimage).unwrap();
    let failure_wire = signed_wire(
        &failure_preimage,
        "canonical_failure_sha256",
        &failure_digest,
    );
    let failure_ref = record_ref("provider-service/adapter-failure");
    let result_preimage = CapabilityResultPreimageV1 {
        schema_id: SchemaId::parse(CAPABILITY_RESULT_V1_SCHEMA).unwrap(),
        schema_sha256: result_schema,
        request_binding: binding,
        attempt_id: attempt.clone(),
        status: CapabilityResultStatusV1::Failed,
        success: None,
        failure: Some(
            serde_json::from_value(serde_json::json!({
                "failure_ref": failure_ref,
                "failure_sha256": failure_digest,
            }))
            .unwrap(),
        ),
        implementation_kind: kind,
        implementation_version: version,
        started_at,
        completed_at,
        signing_authority_binding: signing.clone(),
        issuer_principal_id: signing.principal_id().clone(),
        issuer_key_id: ResidentKeyIdV1::parse(signing.signer_key_id()).unwrap(),
        issuer_key_epoch: *signing.signer_key_epoch(),
    };
    let result_digest = CapabilityResultDigest::hash_canonical(&result_preimage).unwrap();
    let result_wire = signed_wire(&result_preimage, "canonical_result_sha256", &result_digest);
    ProviderInvocationFailureWire::new(coordinate, failure_ref, failure_wire, result_wire)
}

pub(in crate::resident) fn success(
    request: &CommonRequestEnvelopeV1,
    generation: &AdmittedCompiledGeneration,
    signing: &SigningAuthorityBindingV1,
    attempt: &CapabilityAttemptIdV1,
    case: &str,
) -> AdmittedProviderInvocationSuccess {
    let mut binding = request.local_request_binding().unwrap();
    if case == "wrong-request" {
        let mut value = serde_json::to_value(&binding).unwrap();
        value["request_id"] = serde_json::json!("different-request");
        // The existing wire reader delegates to RequestBindingV1::new; the
        // candidate preimage deliberately has no independent wire reader.
        binding = serde_json::from_value::<RequestBindingV1>(value).unwrap();
    }
    let generation_digest = if case == "wrong-generation" {
        CompiledGenerationDigest::parse(sha('e')).unwrap()
    } else {
        generation.record().compiled_generation_digest().clone()
    };
    let instance = if case == "wrong-instance" {
        InstanceDigest::parse(sha('f')).unwrap()
    } else {
        generation.record().instance_digest().clone()
    };
    let mut operation = operation_id();
    if case == "wrong-operation" {
        operation.resource_id = PackageResourceIdV1::parse("different-operation").unwrap();
    }
    let attempt = if case == "wrong-attempt" {
        CapabilityAttemptIdV1::parse("different-attempt").unwrap()
    } else {
        attempt.clone()
    };
    let retained = generation
        .provider_binding("provider-binding-claude-code")
        .unwrap();
    let mut provider = retained.clone();
    if case == "wrong-provider" {
        let mut preimage = provider.to_preimage();
        preimage.id = "different-provider".into();
        provider = ProviderBindingV1::seal(preimage).unwrap();
    }
    let kind = BoundedResidentTextV1::parse(if case == "wrong-implementation-kind" {
        "other_compute"
    } else {
        "host_compute"
    })
    .unwrap();
    let version = BoundedResidentTextV1::parse(if case == "wrong-implementation-version" {
        "2"
    } else {
        "1"
    })
    .unwrap();
    let invocation = ProviderInvocationRequest::new(
        ProviderInvocationCoordinate::new(
            &generation_digest,
            &instance,
            &operation,
            &binding,
            &attempt,
        ),
        &provider,
        &kind,
        &version,
    )
    .unwrap();
    let frontier = ProviderInvocationEvidenceFrontier::new(
        ProviderInvocationEvidenceSubject::from_request(invocation),
        record_ref("provider-evidence"),
        artifact_digest('5'),
        UnixTimeV1::new(65).unwrap(),
        UnixTimeV1::new(75).unwrap(),
    )
    .unwrap();
    let coordinate =
        ProviderInvocationObservationCoordinate::capture(invocation, frontier).unwrap();
    let contracts = crate::session_admission_fixture::local_start_contract_set();
    let result_schema = if case == "wrong-outer-schema" {
        SchemaDigest::parse(sha('0')).unwrap()
    } else {
        schema_digest(CAPABILITY_RESULT_V1_SCHEMA_ID)
    };
    let failure_schema = schema_digest(ADAPTER_FAILURE_V1_SCHEMA_ID);
    let context = CapabilitySuccessAdmissionContextV1 {
        result: CapabilityResultAdmissionContextV1 {
            expected_schema_sha256: &result_schema,
            expected_request_binding: &binding,
            expected_attempt_id: &attempt,
            expected_implementation_kind: &kind,
            expected_implementation_version: &version,
            signing_authority: signing,
        },
        contract_set_ref: binding.contract_set_ref(),
        contract_set: &contracts,
    };
    let preimage = CapabilityResultPreimageV1 {
        schema_id: SchemaId::parse(CAPABILITY_RESULT_V1_SCHEMA).unwrap(),
        schema_sha256: result_schema.clone(),
        request_binding: binding.clone(),
        attempt_id: attempt.clone(),
        status: CapabilityResultStatusV1::Succeeded,
        success: Some(
            CapabilitySuccessV1::inline(
                SchemaId::parse("example/result.v1").unwrap(),
                SchemaDigest::parse(sha('2')).unwrap(),
                serde_json::json!({"accepted":true}),
            )
            .unwrap(),
        ),
        failure: None,
        implementation_kind: kind.clone(),
        implementation_version: version.clone(),
        started_at: UnixTimeV1::new(66).unwrap(),
        completed_at: UnixTimeV1::new(70).unwrap(),
        signing_authority_binding: signing.clone(),
        issuer_principal_id: signing.principal_id().clone(),
        issuer_key_id: ResidentKeyIdV1::parse(signing.signer_key_id()).unwrap(),
        issuer_key_epoch: *signing.signer_key_epoch(),
    };
    let result = CapabilityResultV1::issue_success_contextual(
        preimage,
        &context,
        |_| None,
        |schema, digest, bytes| Evidence::valid().admit_payload(schema, digest, bytes),
        |domain, _, _| {
            assert_eq!(domain, CAPABILITY_RESULT_V1_SIGNATURE_DOMAIN);
            Ok(signature())
        },
    )
    .unwrap();
    let wire = serde_json::from_value(serde_json::to_value(result).unwrap()).unwrap();
    let port = ResultPort::success(ProviderInvocationSuccessWire::new(coordinate, wire));
    let outcome = PreparedProviderInvocation::prepare(invocation, EffectClass::Compute)
        .unwrap()
        .bind(ProviderInvocationAdmissionContext::new(
            context,
            AdapterFailureAdmissionContextV1 {
                expected_schema_sha256: &failure_schema,
                expected_request_binding: &binding,
                expected_attempt_id: &attempt,
                expected_implementation_kind: &kind,
                expected_implementation_version: &version,
                signing_authority: signing,
            },
            ProviderInvocationSafeFailureProof::BeforeDispatch,
        ))
        .unwrap()
        .invoke(&port, &Evidence::valid())
        .unwrap();
    let AdmittedProviderInvocationOutcome::Succeeded(success) = outcome else {
        panic!("admitted success")
    };
    *success
}
