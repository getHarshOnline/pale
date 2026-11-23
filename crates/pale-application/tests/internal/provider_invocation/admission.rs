//! Conformance laws for untrusted provider invocation wire admission.

use std::cell::Cell;

use pale_core::records::{
    AdapterFailureClassV1, AdapterFailureDigest, AdapterFailureEffectStateV1, AdapterFailureIdV1,
    AdapterFailurePreimageV1, AdapterFailureV1, AdapterFailureWireV1, ArtifactDigest,
    BoundedResidentTextV1, CanonicalSignatureV1, CapabilityAttemptIdV1, CapabilityFailureV1,
    CapabilityIdV1, CapabilityResultAdmissionContextV1, CapabilityResultDigest,
    CapabilityResultPreimageV1, CapabilityResultStatusV1, CapabilityResultWireV1,
    CapabilitySuccessAdmissionContextV1, CapabilitySuccessV1, CommonRequestEnvelopeDigest,
    CompiledGenerationDigest, ContractSetDigest, ContractSetEntryV1, ContractSetIdV1,
    ContractSetPreimageV1, ContractSetV1, ContractSetWireV1, EffectClass, InstanceDigest,
    InvalidResidentRecordV1, PackageIdV1, PackageResourceIdV1, PackageResourceRefV1,
    RequestBindingPreimageV1, RequestBindingV1, ResidentCorrelationIdV1, ResidentKeyIdV1,
    ResidentPrincipalIdV1, ResidentReasonV1, ResidentRecordRefV1, ResidentRequestIdV1,
    RouteGenerationV1, SchemaDigest, SchemaId, SessionGrantIdV1, SigningAuthorityBindingV1,
    SigningKeyEpochV1, UnixTimeV1, ADAPTER_FAILURE_V1_SCHEMA, ADAPTER_FAILURE_V1_SIGNATURE_DOMAIN,
    CAPABILITY_RESULT_V1_SCHEMA, CAPABILITY_RESULT_V1_SIGNATURE_DOMAIN, CONTRACT_SET_V1_SCHEMA,
};
use pale_ports::{
    ProviderInvocationCoordinate, ProviderInvocationEvidenceFrontier,
    ProviderInvocationEvidenceSubject, ProviderInvocationFailureWire,
    ProviderInvocationObservation, ProviderInvocationObservationCoordinate, ProviderInvocationPort,
    ProviderInvocationRequest, ProviderInvocationSuccessWire, ProviderInvocationUnavailability,
};
use serde::{de::DeserializeOwned, Serialize};

use super::super::*;

fn hex(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn schema_digest(byte: char) -> SchemaDigest {
    SchemaDigest::parse(hex(byte)).expect("synthetic schema digest")
}

fn artifact_digest(byte: char) -> ArtifactDigest {
    ArtifactDigest::parse(hex(byte)).expect("synthetic artifact digest")
}

fn resident_ref(value: &str) -> ResidentRecordRefV1 {
    ResidentRecordRefV1::parse(value).expect("synthetic record reference")
}

fn signature() -> CanonicalSignatureV1 {
    CanonicalSignatureV1::parse("A".repeat(86)).expect("synthetic signature")
}

fn integrity_wire<P: Serialize, D: Serialize, W: DeserializeOwned>(
    preimage: &P,
    digest_field: &str,
    digest: &D,
) -> W {
    let mut value = serde_json::to_value(preimage).expect("preimage serializes");
    value
        .as_object_mut()
        .expect("record preimage is an object")
        .insert(
            digest_field.to_owned(),
            serde_json::to_value(digest).expect("digest serializes"),
        );
    serde_json::from_value(value).expect("integrity wire decodes")
}

fn signed_wire<P: Serialize, D: Serialize, W: DeserializeOwned>(
    preimage: &P,
    digest_field: &str,
    digest: &D,
) -> W {
    let mut value = serde_json::to_value(preimage).expect("preimage serializes");
    let object = value.as_object_mut().expect("record preimage is an object");
    object.insert(
        digest_field.to_owned(),
        serde_json::to_value(digest).expect("digest serializes"),
    );
    object.insert(
        "signature".to_owned(),
        serde_json::to_value(signature()).expect("signature serializes"),
    );
    serde_json::from_value(value).expect("signed wire decodes")
}

fn signing_authority() -> SigningAuthorityBindingV1 {
    serde_json::from_value(serde_json::json!({
        "authority_mode": "integrated_runtime",
        "principal_id": "synthetic-producer",
        "signer_key_id": "synthetic-producer-key",
        "signer_key_epoch": 1,
        "integrated_runtime": {
            "deployment_runtime_authority_ref": "synthetic-runtime-authority",
            "deployment_runtime_authority_sha256": hex('a'),
            "deployment_binding_ref": "synthetic-deployment",
            "deployment_binding_sha256": hex('b'),
            "checked_principal_trust_checkpoint_ref": "synthetic-trust-checkpoint",
            "checked_principal_trust_checkpoint_sha256": hex('c')
        }
    }))
    .expect("synthetic signing authority")
}

fn contract_set() -> ContractSetV1 {
    let preimage = ContractSetPreimageV1 {
        schema_id: SchemaId::parse(CONTRACT_SET_V1_SCHEMA).expect("contract schema"),
        schema_sha256: schema_digest('1'),
        contract_set_id: ContractSetIdV1::parse("synthetic-contract-set").expect("contract-set id"),
        entries: vec![ContractSetEntryV1::new(
            SchemaId::parse("synthetic/result.v1").expect("result schema id"),
            resident_ref("synthetic-result-schema"),
            schema_digest('2'),
        )],
        compatibility_profile_ref: resident_ref("synthetic-compatibility-profile"),
        compatibility_profile_sha256: pale_core::records::CompatibilityProfileDigest::parse(hex(
            '3',
        ))
        .expect("compatibility profile digest"),
    };
    let digest = ContractSetDigest::hash_canonical(&preimage).expect("contract-set digest");
    let wire: ContractSetWireV1 =
        integrity_wire(&preimage, "canonical_contract_set_sha256", &digest);
    ContractSetV1::admit_contextual_wire(
        wire,
        &preimage.schema_sha256,
        |_| Ok(()),
        |_, _, _| Ok(()),
    )
    .expect("synthetic contract set")
}

fn request_binding(contracts: &ContractSetV1, capability: &str) -> RequestBindingV1 {
    RequestBindingV1::new(RequestBindingPreimageV1 {
        request_id: ResidentRequestIdV1::parse("synthetic-request").expect("request id"),
        correlation_id: ResidentCorrelationIdV1::parse("synthetic-correlation")
            .expect("correlation id"),
        capability_id: CapabilityIdV1::parse(capability).expect("capability id"),
        canonical_request_sha256: CommonRequestEnvelopeDigest::parse(hex('4'))
            .expect("request digest"),
        session_grant_id: SessionGrantIdV1::parse("synthetic-grant").expect("grant id"),
        session_grant_sha256: artifact_digest('5'),
        deployment_binding_ref: resident_ref("synthetic-deployment"),
        deployment_binding_sha256: artifact_digest('6'),
        route_binding_schema_id: SchemaId::parse("synthetic/route.v1").expect("route schema id"),
        route_binding_schema_sha256: schema_digest('7'),
        route_binding_ref: resident_ref("synthetic-route"),
        route_binding_sha256: artifact_digest('8'),
        route_generation: RouteGenerationV1::new(1).expect("route generation"),
        contract_set_ref: resident_ref("synthetic-contract-set"),
        contract_set_sha256: ArtifactDigest::parse(
            contracts
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
    .expect("synthetic request binding")
}

fn success_payload() -> CapabilitySuccessV1 {
    let inline = serde_json::json!({"accepted": true});
    let bytes = pale_core::canon::to_canonical_string(&inline).expect("canonical payload");
    serde_json::from_value(serde_json::json!({
        "result_schema_id": "synthetic/result.v1",
        "result_schema_sha256": hex('2'),
        "result_payload_sha256": ArtifactDigest::hash_bytes(bytes.as_bytes()),
        "inline_payload": inline
    }))
    .expect("synthetic success payload")
}

fn failure_preimage(fixture: &Fixture) -> AdapterFailurePreimageV1 {
    AdapterFailurePreimageV1 {
        schema_id: SchemaId::parse(ADAPTER_FAILURE_V1_SCHEMA).expect("failure schema id"),
        schema_sha256: fixture.failure_schema.clone(),
        request_binding: fixture.request.clone(),
        failure_id: AdapterFailureIdV1::parse("synthetic-failure").expect("failure id"),
        attempt_id: fixture.attempt.clone(),
        failure_class: AdapterFailureClassV1::Unavailable,
        stable_code: ResidentReasonV1::parse("synthetic_unavailable").expect("stable code"),
        bounded_message: None,
        provider_status: None,
        retry_at: None,
        effect_state: AdapterFailureEffectStateV1::NotStarted,
        absence_evidence_ref: None,
        absence_evidence_sha256: None,
        implementation_kind: fixture.implementation_kind.clone(),
        implementation_version: fixture.implementation_version.clone(),
        recorded_at: UnixTimeV1::new(10).expect("recorded time"),
        signing_authority_binding: signing_authority(),
        issuer_principal_id: ResidentPrincipalIdV1::parse("synthetic-producer")
            .expect("producer principal"),
        issuer_key_id: ResidentKeyIdV1::parse("synthetic-producer-key").expect("producer key"),
        issuer_key_epoch: SigningKeyEpochV1::new(1).expect("producer key epoch"),
    }
}

struct Fixture {
    contracts: ContractSetV1,
    request: RequestBindingV1,
    provider: pale_core::records::ProviderBindingV1,
    generation: CompiledGenerationDigest,
    instance: InstanceDigest,
    operation: PackageResourceRefV1,
    attempt: CapabilityAttemptIdV1,
    implementation_kind: BoundedResidentTextV1,
    implementation_version: BoundedResidentTextV1,
    result_schema: SchemaDigest,
    failure_schema: SchemaDigest,
    authority: SigningAuthorityBindingV1,
    contracts_ref: ResidentRecordRefV1,
    frontier: ProviderInvocationEvidenceFrontier,
}

impl Fixture {
    fn new() -> Self {
        Self::new_for("synthetic-operation", 'd', 20)
    }

    fn new_for(operation_id: &str, generation_seed: char, valid_until: u64) -> Self {
        let contracts = contract_set();
        let request = request_binding(&contracts, "synthetic.read");
        let provider = crate::provider_binding_fixtures::provider_binding(
            crate::composition_source_fixtures::participant("synthetic-participant"),
        );
        let operation = PackageResourceRefV1 {
            package_id: PackageIdV1::parse("synthetic-package").expect("package id"),
            resource_id: PackageResourceIdV1::parse(operation_id).expect("resource id"),
        };
        let frontier = ProviderInvocationEvidenceFrontier::new(
            ProviderInvocationEvidenceSubject::from_parts(
                provider.binding_identity.clone(),
                operation.clone(),
                request.capability_id().clone(),
            ),
            resident_ref("synthetic-provider-evidence"),
            artifact_digest('0'),
            UnixTimeV1::new(8).expect("observed time"),
            UnixTimeV1::new(valid_until).expect("expiry time"),
        )
        .expect("evidence frontier");
        Self {
            request,
            contracts,
            provider,
            generation: CompiledGenerationDigest::parse(hex(generation_seed)).expect("generation"),
            instance: InstanceDigest::parse(hex('e')).expect("instance"),
            operation,
            attempt: CapabilityAttemptIdV1::parse("synthetic-attempt").expect("attempt"),
            implementation_kind: BoundedResidentTextV1::parse("synthetic_native")
                .expect("implementation kind"),
            implementation_version: BoundedResidentTextV1::parse("0.0.1")
                .expect("implementation version"),
            result_schema: schema_digest('9'),
            failure_schema: schema_digest('f'),
            authority: signing_authority(),
            contracts_ref: resident_ref("synthetic-contract-set"),
            frontier,
        }
    }

    fn provider_request(&self) -> ProviderInvocationRequest<'_> {
        ProviderInvocationRequest::new(
            ProviderInvocationCoordinate::new(
                &self.generation,
                &self.instance,
                &self.operation,
                &self.request,
                &self.attempt,
            ),
            &self.provider,
            &self.implementation_kind,
            &self.implementation_version,
        )
        .expect("sealed provider request")
    }

    fn result_preimage(
        &self,
        schema_sha256: SchemaDigest,
        status: CapabilityResultStatusV1,
        success: Option<CapabilitySuccessV1>,
        failure: Option<CapabilityFailureV1>,
    ) -> CapabilityResultPreimageV1 {
        CapabilityResultPreimageV1 {
            schema_id: SchemaId::parse(CAPABILITY_RESULT_V1_SCHEMA).expect("result schema id"),
            schema_sha256,
            request_binding: self.request.clone(),
            attempt_id: self.attempt.clone(),
            status,
            success,
            failure,
            implementation_kind: self.implementation_kind.clone(),
            implementation_version: self.implementation_version.clone(),
            started_at: UnixTimeV1::new(10).expect("start time"),
            completed_at: UnixTimeV1::new(11).expect("completion time"),
            signing_authority_binding: signing_authority(),
            issuer_principal_id: ResidentPrincipalIdV1::parse("synthetic-producer")
                .expect("producer principal"),
            issuer_key_id: ResidentKeyIdV1::parse("synthetic-producer-key").expect("producer key"),
            issuer_key_epoch: SigningKeyEpochV1::new(1).expect("producer key epoch"),
        }
    }

    fn context(&self) -> ProviderInvocationAdmissionContext<'_> {
        let result = CapabilityResultAdmissionContextV1 {
            expected_schema_sha256: &self.result_schema,
            expected_request_binding: &self.request,
            expected_attempt_id: &self.attempt,
            expected_implementation_kind: &self.implementation_kind,
            expected_implementation_version: &self.implementation_version,
            signing_authority: &self.authority,
        };
        ProviderInvocationAdmissionContext::new(
            CapabilitySuccessAdmissionContextV1 {
                result,
                contract_set_ref: &self.contracts_ref,
                contract_set: &self.contracts,
            },
            pale_core::records::AdapterFailureAdmissionContextV1 {
                expected_schema_sha256: &self.failure_schema,
                expected_request_binding: &self.request,
                expected_attempt_id: &self.attempt,
                expected_implementation_kind: &self.implementation_kind,
                expected_implementation_version: &self.implementation_version,
                signing_authority: &self.authority,
            },
            ProviderInvocationSafeFailureProof::BeforeDispatch,
        )
    }

    fn success_wire(&self) -> CapabilityResultWireV1 {
        let preimage = self.result_preimage(
            self.result_schema.clone(),
            CapabilityResultStatusV1::Succeeded,
            Some(success_payload()),
            None,
        );
        let digest = CapabilityResultDigest::hash_canonical(&preimage).expect("result digest");
        signed_wire(&preimage, "canonical_result_sha256", &digest)
    }

    fn failure_wires(
        &self,
    ) -> (
        ResidentRecordRefV1,
        AdapterFailureWireV1,
        CapabilityResultWireV1,
    ) {
        let failure_preimage = failure_preimage(self);
        let failure_digest =
            AdapterFailureDigest::hash_canonical(&failure_preimage).expect("failure digest");
        let failure_wire: AdapterFailureWireV1 = signed_wire(
            &failure_preimage,
            "canonical_failure_sha256",
            &failure_digest,
        );
        let failure_context = pale_core::records::AdapterFailureAdmissionContextV1 {
            expected_schema_sha256: &self.failure_schema,
            expected_request_binding: &self.request,
            expected_attempt_id: &self.attempt,
            expected_implementation_kind: &self.implementation_kind,
            expected_implementation_version: &self.implementation_version,
            signing_authority: &self.authority,
        };
        let failure = AdapterFailureV1::admit_before_dispatch_wire(
            failure_wire.clone(),
            &failure_context,
            |_, _, _, _| Ok(()),
            |_, _, _, _| Ok(()),
        )
        .expect("synthetic safe failure");
        let failure_ref = resident_ref("synthetic-failure-record");
        let failure_projection: CapabilityFailureV1 = serde_json::from_value(serde_json::json!({
            "failure_ref": failure_ref,
            "failure_sha256": failure.canonical_failure_sha256()
        }))
        .expect("synthetic failure projection");
        let result_preimage = self.result_preimage(
            self.result_schema.clone(),
            CapabilityResultStatusV1::Failed,
            None,
            Some(failure_projection),
        );
        let result_digest =
            CapabilityResultDigest::hash_canonical(&result_preimage).expect("result digest");
        let result_wire: CapabilityResultWireV1 =
            signed_wire(&result_preimage, "canonical_result_sha256", &result_digest);
        (failure_ref, failure_wire, result_wire)
    }

    fn expected_coordinate(&self) -> ProviderInvocationObservationCoordinate {
        ProviderInvocationObservationCoordinate::capture(
            self.provider_request(),
            self.frontier.clone(),
        )
        .expect("frontier subject matches request")
    }
}

struct SyntheticProvider {
    observation: ProviderInvocationObservation,
    calls: Cell<u32>,
}

impl SyntheticProvider {
    fn new(observation: ProviderInvocationObservation) -> Self {
        Self {
            observation,
            calls: Cell::new(0),
        }
    }
}

impl ProviderInvocationPort for SyntheticProvider {
    fn invoke(&self, _request: ProviderInvocationRequest<'_>) -> ProviderInvocationObservation {
        self.calls.set(self.calls.get() + 1);
        self.observation.clone()
    }
}

#[derive(Default)]
struct SyntheticAdmission {
    frontier_admitted: Cell<bool>,
    payload_admitted: Cell<bool>,
    result_signature_verified: Cell<bool>,
    failure_signature_verified: Cell<bool>,
}

impl ProviderInvocationEvidenceAdmission for SyntheticAdmission {
    fn admit_evidence_frontier(
        &self,
        frontier: &ProviderInvocationEvidenceFrontier,
    ) -> Result<(), InvalidResidentRecordV1> {
        assert_eq!(
            frontier.evidence_ref().as_str(),
            "synthetic-provider-evidence"
        );
        self.frontier_admitted.set(true);
        Ok(())
    }

    fn resolve_immutable(&self, _reference: &ResidentRecordRefV1) -> Option<Vec<u8>> {
        None
    }

    fn admit_payload(
        &self,
        schema_id: &SchemaId,
        schema_sha256: &SchemaDigest,
        bytes: &[u8],
    ) -> Result<(), InvalidResidentRecordV1> {
        assert_eq!(schema_id.as_str(), "synthetic/result.v1");
        assert_eq!(schema_sha256, &schema_digest('2'));
        assert_eq!(bytes, br#"{"accepted":true}"#);
        self.payload_admitted.set(true);
        Ok(())
    }

    fn admit_failure_class(
        &self,
        class: AdapterFailureClassV1,
        _retry_at: Option<UnixTimeV1>,
        _request: &RequestBindingV1,
        _attempt: &CapabilityAttemptIdV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        assert_eq!(class, AdapterFailureClassV1::Unavailable);
        Ok(())
    }

    fn admit_absence_evidence(
        &self,
        _reference: &ResidentRecordRefV1,
        _expected_sha256: &ArtifactDigest,
        _bytes: &[u8],
        _request: &RequestBindingV1,
        _attempt: &CapabilityAttemptIdV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        Ok(())
    }

    fn verify_result_signature(
        &self,
        domain: &[u8],
        _preimage: &CapabilityResultPreimageV1,
        _digest: &CapabilityResultDigest,
        _signature: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        assert_eq!(domain, CAPABILITY_RESULT_V1_SIGNATURE_DOMAIN);
        self.result_signature_verified.set(true);
        Ok(())
    }

    fn verify_failure_signature(
        &self,
        domain: &[u8],
        _preimage: &AdapterFailurePreimageV1,
        _digest: &AdapterFailureDigest,
        _signature: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        assert_eq!(domain, ADAPTER_FAILURE_V1_SIGNATURE_DOMAIN);
        self.failure_signature_verified.set(true);
        Ok(())
    }
}

#[test]
fn synthetic_provider_success_is_admitted_only_after_exact_coordinate_echo() {
    let fixture = Fixture::new();
    let port = SyntheticProvider::new(ProviderInvocationObservation::Succeeded(Box::new(
        ProviderInvocationSuccessWire::new(fixture.expected_coordinate(), fixture.success_wire()),
    )));
    let admission = SyntheticAdmission::default();

    let outcome =
        PreparedProviderInvocation::prepare(fixture.provider_request(), EffectClass::Read)
            .expect("read request prepares")
            .bind(fixture.context())
            .expect("admission context binds")
            .invoke(&port, &admission)
            .expect("exact signed result admits");

    let AdmittedProviderInvocationOutcome::Succeeded(success) = outcome else {
        panic!("synthetic provider returned a non-success outcome");
    };
    assert_eq!(
        success.coordinate().compiled_generation_digest(),
        &fixture.generation
    );
    assert_eq!(success.coordinate().instance_digest(), &fixture.instance);
    assert_eq!(success.coordinate().operation(), &fixture.operation);
    assert_eq!(success.coordinate().attempt_id(), &fixture.attempt);
    assert_eq!(
        success.coordinate().provider_binding_identity(),
        &fixture.provider.binding_identity
    );
    assert_eq!(success.coordinate().evidence_frontier(), &fixture.frontier);
    assert!(admission.payload_admitted.get());
    assert!(admission.frontier_admitted.get());
    assert!(admission.result_signature_verified.get());
    assert_eq!(port.calls.get(), 1);
}

#[test]
fn synthetic_signed_pre_dispatch_failure_and_failed_result_admit_together() {
    let fixture = Fixture::new();
    let (failure_ref, failure, result) = fixture.failure_wires();
    let port = SyntheticProvider::new(ProviderInvocationObservation::Failed(Box::new(
        ProviderInvocationFailureWire::new(
            fixture.expected_coordinate(),
            failure_ref,
            failure,
            result,
        ),
    )));
    let admission = SyntheticAdmission::default();

    let outcome =
        PreparedProviderInvocation::prepare(fixture.provider_request(), EffectClass::Compute)
            .expect("pure request prepares")
            .bind(fixture.context())
            .expect("admission context binds")
            .invoke(&port, &admission)
            .expect("signed safe failure admits");

    assert!(matches!(
        outcome,
        AdmittedProviderInvocationOutcome::Failed(_)
    ));
    assert!(admission.failure_signature_verified.get());
    assert!(admission.frontier_admitted.get());
    assert!(admission.result_signature_verified.get());
}

#[test]
fn every_unavailable_branch_remains_distinct_and_operation_local() {
    let reasons = [
        ProviderInvocationUnavailability::OwnerContractUnavailable,
        ProviderInvocationUnavailability::Down,
        ProviderInvocationUnavailability::Partitioned,
        ProviderInvocationUnavailability::Removed,
        ProviderInvocationUnavailability::EvidenceUnknown,
    ];
    let fixture = Fixture::new();
    let admission = SyntheticAdmission::default();

    for reason in reasons {
        let port = SyntheticProvider::new(ProviderInvocationObservation::Unavailable(reason));
        let outcome =
            PreparedProviderInvocation::prepare(fixture.provider_request(), EffectClass::Network)
                .expect("remote read prepares")
                .bind(fixture.context())
                .expect("admission context binds")
                .invoke(&port, &admission)
                .expect("unavailability is a closed outcome");
        let AdmittedProviderInvocationOutcome::Unavailable(attempt) = outcome else {
            panic!("synthetic provider returned a terminal wire");
        };
        assert_eq!(attempt.reason(), reason);
        assert_eq!(attempt.generation(), &fixture.generation);
        assert_eq!(attempt.instance(), &fixture.instance);
        assert_eq!(attempt.operation(), &fixture.operation);
        assert_eq!(attempt.capability_id(), fixture.request.capability_id());
        assert_eq!(
            attempt.provider_binding_identity(),
            &fixture.provider.binding_identity
        );
        assert!(!admission.frontier_admitted.get());
    }
}

pub(super) fn admitted_success_fixture(
    operation_id: &str,
    generation_seed: char,
    valid_until: u64,
) -> (
    AdmittedProviderInvocationSuccess,
    pale_core::records::ProviderBindingV1,
) {
    let fixture = Fixture::new_for(operation_id, generation_seed, valid_until);
    let port = SyntheticProvider::new(ProviderInvocationObservation::Succeeded(Box::new(
        ProviderInvocationSuccessWire::new(fixture.expected_coordinate(), fixture.success_wire()),
    )));
    let outcome =
        PreparedProviderInvocation::prepare(fixture.provider_request(), EffectClass::Read)
            .expect("read request prepares")
            .bind(fixture.context())
            .expect("admission context binds")
            .invoke(&port, &SyntheticAdmission::default())
            .expect("synthetic result admits");
    let AdmittedProviderInvocationOutcome::Succeeded(success) = outcome else {
        panic!("synthetic provider returned a non-success outcome");
    };
    (*success, fixture.provider)
}

#[test]
fn effectful_requests_never_reach_the_ordinary_provider_port() {
    let fixture = Fixture::new();
    let port = SyntheticProvider::new(ProviderInvocationObservation::Unavailable(
        ProviderInvocationUnavailability::EvidenceUnknown,
    ));

    for effect in [
        EffectClass::Execute,
        EffectClass::Write,
        EffectClass::Publish,
    ] {
        let error = PreparedProviderInvocation::prepare(fixture.provider_request(), effect)
            .err()
            .expect("effectful request is denied without dispatch intent");
        assert!(matches!(
            error,
            ProviderInvocationAdmissionError::DispatchIntentRequired { effect: rejected }
                if rejected == effect
        ));
    }
    assert_eq!(port.calls.get(), 0);
}

#[test]
fn evidence_frontier_for_another_subject_cannot_form_a_positive_observation() {
    let fixture = Fixture::new();
    let other_subject = ProviderInvocationEvidenceSubject::from_parts(
        fixture.provider.binding_identity.clone(),
        PackageResourceRefV1 {
            package_id: fixture.operation.package_id.clone(),
            resource_id: PackageResourceIdV1::parse("other-operation").expect("other operation"),
        },
        fixture.request.capability_id().clone(),
    );
    let frontier = ProviderInvocationEvidenceFrontier::new(
        other_subject,
        resident_ref("synthetic-provider-evidence"),
        artifact_digest('0'),
        UnixTimeV1::new(8).expect("observed time"),
        UnixTimeV1::new(20).expect("expiry time"),
    )
    .expect("ordered frontier");

    assert_eq!(
        ProviderInvocationObservationCoordinate::capture(fixture.provider_request(), frontier)
            .expect_err("another operation cannot supply positive evidence"),
        pale_ports::ProviderInvocationCoordinateError::EvidenceSubjectMismatch
    );
}

#[test]
fn coordinate_substitution_is_rejected_before_signed_wire_admission() {
    let fixture = Fixture::new();
    let baseline = fixture.expected_coordinate();
    let admission = SyntheticAdmission::default();

    let mut substitutions = Vec::new();
    let other_generation = CompiledGenerationDigest::parse(hex('1')).expect("other generation");
    let other_instance = InstanceDigest::parse(hex('2')).expect("other instance");
    let other_attempt = CapabilityAttemptIdV1::parse("other-attempt").expect("other attempt");
    let other_kind = BoundedResidentTextV1::parse("other_implementation").expect("other kind");
    let other_provider = crate::provider_binding_fixtures::provider_binding_with_coordinates(
        crate::composition_source_fixtures::participant("synthetic-participant"),
        "other-provider-binding",
        "other-executable",
    );

    substitutions.push(
        ProviderInvocationRequest::new(
            ProviderInvocationCoordinate::new(
                &other_generation,
                &fixture.instance,
                &fixture.operation,
                &fixture.request,
                &fixture.attempt,
            ),
            &fixture.provider,
            &fixture.implementation_kind,
            &fixture.implementation_version,
        )
        .expect("generation substitution request"),
    );
    substitutions.push(
        ProviderInvocationRequest::new(
            ProviderInvocationCoordinate::new(
                &fixture.generation,
                &other_instance,
                &fixture.operation,
                &fixture.request,
                &fixture.attempt,
            ),
            &fixture.provider,
            &fixture.implementation_kind,
            &fixture.implementation_version,
        )
        .expect("instance substitution request"),
    );
    substitutions.push(
        ProviderInvocationRequest::new(
            ProviderInvocationCoordinate::new(
                &fixture.generation,
                &fixture.instance,
                &fixture.operation,
                &fixture.request,
                &other_attempt,
            ),
            &fixture.provider,
            &fixture.implementation_kind,
            &fixture.implementation_version,
        )
        .expect("attempt substitution request"),
    );
    substitutions.push(
        ProviderInvocationRequest::new(
            ProviderInvocationCoordinate::new(
                &fixture.generation,
                &fixture.instance,
                &fixture.operation,
                &fixture.request,
                &fixture.attempt,
            ),
            &other_provider,
            &fixture.implementation_kind,
            &fixture.implementation_version,
        )
        .expect("binding substitution request"),
    );
    substitutions.push(
        ProviderInvocationRequest::new(
            ProviderInvocationCoordinate::new(
                &fixture.generation,
                &fixture.instance,
                &fixture.operation,
                &fixture.request,
                &fixture.attempt,
            ),
            &fixture.provider,
            &other_kind,
            &fixture.implementation_version,
        )
        .expect("implementation substitution request"),
    );

    for request in substitutions {
        let port = SyntheticProvider::new(ProviderInvocationObservation::Succeeded(Box::new(
            ProviderInvocationSuccessWire::new(baseline.clone(), fixture.success_wire()),
        )));
        let error = PreparedProviderInvocation::prepare(request, EffectClass::Read)
            .expect("read request prepares")
            .bind(fixture.context())
            .and_then(|invocation| invocation.invoke(&port, &admission))
            .expect_err("substitution is rejected");
        assert!(matches!(
            error,
            ProviderInvocationAdmissionError::AdmissionContextMismatch
                | ProviderInvocationAdmissionError::InvalidCoordinate(_)
        ));
    }
}

#[test]
fn schema_substitution_is_rejected_by_existing_signed_record_admission() {
    let fixture = Fixture::new();
    let substituted_preimage = fixture.result_preimage(
        schema_digest('8'),
        CapabilityResultStatusV1::Succeeded,
        Some(success_payload()),
        None,
    );
    let substituted_digest = CapabilityResultDigest::hash_canonical(&substituted_preimage)
        .expect("substituted result digest");
    let substituted: CapabilityResultWireV1 = signed_wire(
        &substituted_preimage,
        "canonical_result_sha256",
        &substituted_digest,
    );
    let port = SyntheticProvider::new(ProviderInvocationObservation::Succeeded(Box::new(
        ProviderInvocationSuccessWire::new(fixture.expected_coordinate(), substituted),
    )));

    let error = PreparedProviderInvocation::prepare(fixture.provider_request(), EffectClass::Read)
        .expect("read request prepares")
        .bind(fixture.context())
        .expect("admission context binds")
        .invoke(&port, &SyntheticAdmission::default())
        .expect_err("schema substitution is rejected");
    assert!(matches!(
        error,
        ProviderInvocationAdmissionError::InvalidEvidence(_)
    ));
}
