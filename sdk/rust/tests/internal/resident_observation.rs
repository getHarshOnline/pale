//! SDK-owned resident observation boundary tests.

#[path = "resident_native_start.rs"]
mod native_start;

use pale_core::records::{
    ArtifactDigest, CommonRequestEnvelopeDigest, OperationHandleDigest, OperationObservationDigest,
    PackageIdV1, PackageResourceIdV1, PackageResourceRefV1, SessionAdmissionIdV1,
    SessionAdmissionLeaseDigest,
};
use pale_ports::{
    LocalIpcCarrierError, LocalIpcFramePort, LocalIpcFrameSection, LocalIpcPeerCredentials,
    ResidentObserveFrameRequest, ResidentObserveFrameResponse,
    ResidentOperationFrameCancellationDisposition, ResidentOperationFrameFailure,
    ResidentOperationFrameLocator, ResidentOperationFrameRecords, ResidentOperationFrameRequest,
    ResidentOperationFrameResponse, ResidentOperationFrameUnsupportedReason,
};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::*;
use crate::resident_operation::{ResidentOperationClient, ResidentOperationClientError};
use crate::{
    AuthenticatedResidentCarrier, AuthenticatedResidentCarrierFailure,
    AuthenticatedResidentCarrierSource, AuthenticatedResidentCarrierSourceFailure,
    DiagnosticDisposition, DiagnosticEffectCertainty, DiagnosticRetry, DiagnosticStage,
    ResidentAttachment, ResidentAttachmentBuildError, ResidentConsultAdvanceRequest,
    ResidentConsultStartRequest, ResidentDelegateAdvanceOutcome, ResidentDelegateAdvanceRequest,
    ResidentDelegateStartRequest, ResidentInvokeAdvanceOutcome, ResidentInvokeAdvanceRequest,
    ResidentInvokeStartRequest, ResidentOperationCancellationDisposition,
    ResidentOperationCancellationRequest, ResidentOperationUnsupportedReason,
    ResidentResponseAdmission, ResidentResponseAdmissionFailure, RESIDENT_RECORD_BYTE_LIMIT,
};

impl ResidentAttachment {
    fn from_client(client: impl ResidentObservationClient + 'static) -> Self {
        Self {
            client: Some(Arc::new(client)),
            operation_client: None,
            hook_client: None,
        }
    }

    fn from_clients(
        observation: impl ResidentObservationClient + 'static,
        operation: impl ResidentOperationClient + 'static,
    ) -> Self {
        Self {
            client: Some(Arc::new(observation)),
            operation_client: Some(Arc::new(operation)),
            hook_client: None,
        }
    }

    fn from_framed_client(
        port: Box<dyn LocalIpcFramePort>,
        call_timeout: Duration,
        response_admission: Arc<dyn ResidentResponseAdmission>,
    ) -> Result<Self, ResidentAttachmentBuildError> {
        Self::provision_operations(
            carrier_source(FramePortCarrier { port }),
            call_timeout,
            response_admission,
        )
    }
}

struct SingleCarrierSource {
    carrier: Option<Box<dyn AuthenticatedResidentCarrier>>,
}

impl AuthenticatedResidentCarrierSource for SingleCarrierSource {
    fn open(
        &mut self,
        _deadline: Instant,
    ) -> Result<Box<dyn AuthenticatedResidentCarrier>, AuthenticatedResidentCarrierSourceFailure>
    {
        self.carrier
            .take()
            .ok_or(AuthenticatedResidentCarrierSourceFailure)
    }
}

fn carrier_source(
    carrier: impl AuthenticatedResidentCarrier + 'static,
) -> Box<dyn AuthenticatedResidentCarrierSource> {
    Box::new(SingleCarrierSource {
        carrier: Some(Box::new(carrier)),
    })
}

struct ScriptedCarrierSource {
    opens: Arc<AtomicUsize>,
    carriers: VecDeque<
        Result<Box<dyn AuthenticatedResidentCarrier>, AuthenticatedResidentCarrierSourceFailure>,
    >,
}

impl AuthenticatedResidentCarrierSource for ScriptedCarrierSource {
    fn open(
        &mut self,
        deadline: Instant,
    ) -> Result<Box<dyn AuthenticatedResidentCarrier>, AuthenticatedResidentCarrierSourceFailure>
    {
        assert!(Instant::now() < deadline, "source receives a live deadline");
        self.opens.fetch_add(1, Ordering::SeqCst);
        self.carriers
            .pop_front()
            .unwrap_or(Err(AuthenticatedResidentCarrierSourceFailure))
    }
}

struct FramePortCarrier {
    port: Box<dyn LocalIpcFramePort>,
}

struct ScriptedAuthenticatedCarrier {
    sent: Arc<Mutex<Vec<Vec<u8>>>>,
    responses: VecDeque<Result<Box<[u8]>, AuthenticatedResidentCarrierFailure>>,
}

impl AuthenticatedResidentCarrier for ScriptedAuthenticatedCarrier {
    fn exchange(
        &mut self,
        request: &[u8],
        deadline: Instant,
    ) -> Result<Box<[u8]>, AuthenticatedResidentCarrierFailure> {
        assert!(
            Instant::now() < deadline,
            "carrier receives a live deadline"
        );
        self.sent
            .lock()
            .expect("authenticated send log remains available")
            .push(request.to_vec());
        self.responses.pop_front().unwrap_or(Err(
            AuthenticatedResidentCarrierFailure::ConnectionUnavailable,
        ))
    }
}

impl AuthenticatedResidentCarrier for FramePortCarrier {
    fn exchange(
        &mut self,
        request: &[u8],
        deadline: Instant,
    ) -> Result<Box<[u8]>, AuthenticatedResidentCarrierFailure> {
        self.port
            .send_frame(request, deadline)
            .map_err(|_| AuthenticatedResidentCarrierFailure::OutcomeUnknown)?;
        self.port
            .receive_frame(deadline)
            .map_err(|_| AuthenticatedResidentCarrierFailure::OutcomeUnknown)
    }
}

const HANDLE_REF: &str = "operations/handle-1";
const OBSERVATION_REF: &str = "operations/observation-1";

fn sha(character: char) -> String {
    std::iter::repeat_n(character, 64).collect()
}

fn canonical(value: &serde_json::Value) -> Vec<u8> {
    pale_core::canon::to_canonical_string(value)
        .expect("fixture is canonical")
        .into_bytes()
}

fn schema_sha256(schema_id: &str) -> String {
    pale_contracts::schema_descriptor(schema_id)
        .expect("schema is embedded")
        .schema_sha256
        .to_owned()
}

fn handle_preimage(operation_handle_id: &str) -> serde_json::Value {
    serde_json::json!({
        "schema_id": "pale/operation-handle.v1",
        "schema_sha256": schema_sha256("operation-handle.v1"),
        "operation_handle_id": operation_handle_id,
        "operation_class": "invoke",
        "request_binding": {
            "request_id": "request-1",
            "correlation_id": "correlation-1",
            "capability_id": "operation.observe",
            "canonical_request_sha256": sha('2'),
            "session_grant_id": "grant-1",
            "session_grant_sha256": sha('3'),
            "deployment_binding_ref": "deployment-binding-1",
            "deployment_binding_sha256": sha('4'),
            "route_binding_schema_id": "pale/local-route-binding.v1",
            "route_binding_schema_sha256": sha('5'),
            "route_binding_ref": "route-binding-1",
            "route_binding_sha256": sha('6'),
            "route_generation": 1,
            "contract_set_ref": "contract-set-1",
            "contract_set_sha256": sha('7')
        },
        "operation_lease_ref": "operation-lease-1",
        "operation_lease_sha256": sha('8'),
        "compiled_generation_sha256": sha('9'),
        "activation_fence_sha256": sha('a'),
        "instance_sha256": sha('b'),
        "created_at": 10,
        "expires_at": 20,
        "retain_until": 30,
        "deployment_runtime_authority_ref": "runtime-authority-1",
        "deployment_runtime_authority_sha256": sha('c'),
        "issuer_principal_id": "principal-1",
        "issuer_key_id": "key-1",
        "issuer_key_epoch": 1
    })
}

fn sealed_handle(operation_handle_id: &str) -> (String, serde_json::Value) {
    reseal_handle(handle_preimage(operation_handle_id))
}

fn reseal_handle(mut record: serde_json::Value) -> (String, serde_json::Value) {
    let object = record.as_object_mut().expect("handle is an object");
    object.remove("canonical_operation_handle_sha256");
    object.remove("signature");
    let preimage = record;
    let digest = OperationHandleDigest::hash_canonical(&preimage)
        .expect("handle preimage hashes")
        .as_str()
        .to_owned();
    let mut record = preimage;
    record["canonical_operation_handle_sha256"] = serde_json::json!(digest);
    record["signature"] = serde_json::json!("A".repeat(86));
    (digest, record)
}

fn observation_preimage(
    operation_handle_id: &str,
    handle_ref: &str,
    handle_sha256: &str,
) -> serde_json::Value {
    serde_json::json!({
        "schema_id": "pale/operation-observation.v1",
        "schema_sha256": schema_sha256("operation-observation.v1"),
        "operation_handle_id": operation_handle_id,
        "operation_handle_ref": handle_ref,
        "operation_handle_sha256": handle_sha256,
        "revision": 1,
        "state": {"state": "active", "phase": {"phase": "resolving"}},
        "observed_at": 10,
        "deployment_runtime_authority_ref": "runtime-authority-1",
        "deployment_runtime_authority_sha256": sha('c'),
        "issuer_principal_id": "principal-1",
        "issuer_key_id": "key-1",
        "issuer_key_epoch": 1
    })
}

fn sealed_observation(
    operation_handle_id: &str,
    handle_ref: &str,
    handle_sha256: &str,
) -> serde_json::Value {
    reseal_observation(observation_preimage(
        operation_handle_id,
        handle_ref,
        handle_sha256,
    ))
}

fn reseal_observation(mut record: serde_json::Value) -> serde_json::Value {
    let object = record.as_object_mut().expect("observation is an object");
    object.remove("canonical_operation_observation_sha256");
    object.remove("signature");
    let preimage = record;
    let digest = OperationObservationDigest::hash_canonical(&preimage)
        .expect("observation preimage hashes")
        .as_str()
        .to_owned();
    let mut record = preimage;
    record["canonical_operation_observation_sha256"] = serde_json::json!(digest);
    record["signature"] = serde_json::json!("A".repeat(86));
    record
}

fn signed_common_request(handle: &serde_json::Value) -> Vec<u8> {
    let payload_sha256 = ArtifactDigest::hash_bytes(&canonical(handle))
        .as_str()
        .to_owned();
    let preimage = serde_json::json!({
        "envelope_schema_id": "pale/common-request-envelope.v1",
        "envelope_schema_sha256": schema_sha256("common-request-envelope.v1"),
        "request_id": "observe-request-1",
        "correlation_id": "correlation-1",
        "workspace_id": "workspace-1",
        "capability_id": "operation.observe",
        "operation": PackageResourceRefV1 {
            package_id: PackageIdV1::parse("example.engine")
                .expect("fixture Package id is valid"),
            resource_id: PackageResourceIdV1::parse("review")
                .expect("fixture operation id is valid"),
        },
        "caller": {
            "identity": "caller-1",
            "role": "operator",
            "surface": "rust-sdk",
            "signer_key_id": "key-1",
            "signer_key_epoch": 1
        },
        "signing_authority_binding": {
            "authority_mode": "integrated_runtime",
            "principal_id": "principal-1",
            "signer_key_id": "key-1",
            "signer_key_epoch": 1,
            "integrated_runtime": {
                "deployment_runtime_authority_ref": "runtime-authority-1",
                "deployment_runtime_authority_sha256": sha('c'),
                "deployment_binding_ref": "deployment-binding-1",
                "deployment_binding_sha256": sha('4'),
                "checked_principal_trust_checkpoint_ref": "trust-checkpoint-1",
                "checked_principal_trust_checkpoint_sha256": sha('d')
            }
        },
        "route": {
            "origin_principal_id": "principal-1",
            "origin_node_id": "node-1",
            "destination_principal_id": "principal-1",
            "route_id": "route-1",
            "route_generation": 1,
            "transport": "injected-resident-attachment",
            "route_binding_schema_id": "pale/local-route-binding.v1",
            "route_binding_schema_sha256": sha('5'),
            "route_binding_ref": "route-binding-1",
            "route_binding_sha256": sha('6'),
            "contract_set_ref": "contract-set-1",
            "contract_set_sha256": sha('7')
        },
        "session_admission": {
            "session_admission_id": SessionAdmissionIdV1::parse("session-1")
                .expect("fixture session admission id is valid"),
            "session_admission_lease_digest": SessionAdmissionLeaseDigest::parse(
                "3fbf04738441cf81c511f49d11a20c85d689e110ea572934bb70cdd394496a9d"
                    .to_owned(),
            )
            .expect("fixture session admission lease digest is valid"),
            "session_grant_id": "grant-1",
            "session_grant_sha256": sha('3'),
            "deployment_binding_ref": "deployment-binding-1",
            "deployment_binding_sha256": sha('4'),
            "session_floor_epoch": 1,
            "grant_issuer_principal_id": "principal-1",
            "grant_issuer_key_id": "key-1",
            "grant_issuer_key_epoch": 1,
            "checked_revocation_epoch": 1,
            "checked_revocation_checkpoint_ref": "revocation-checkpoint-1",
            "checked_revocation_checkpoint_sha256": sha('e'),
            "checked_at": 9
        },
        "scope": {
            "principal_scope": "principal-1",
            "controller_scope": "controller-1",
            "environment": "local",
            "data_classification": "internal",
            "purpose": "observe retained operation",
            "authority_evidence": []
        },
        "risk_class": "bounded-read",
        "resource_constraints": [],
        "idempotency_key": "observe-operation-1",
        "issued_at": 10,
        "expires_at": 20,
        "body": {
            "schema_id": "pale/operation-handle.v1",
            "schema_sha256": schema_sha256("operation-handle.v1"),
            "payload_sha256": payload_sha256,
            "payload": handle
        }
    });
    reseal_common_request(preimage)
}

fn reseal_common_request(mut envelope: serde_json::Value) -> Vec<u8> {
    let object = envelope.as_object_mut().expect("request is an object");
    object.remove("canonical_request_sha256");
    object.remove("caller_signature");
    let preimage = envelope;
    let digest = CommonRequestEnvelopeDigest::hash_canonical(&preimage)
        .expect("request preimage hashes")
        .as_str()
        .to_owned();
    let mut envelope = preimage;
    envelope["canonical_request_sha256"] = serde_json::json!(digest);
    envelope["caller_signature"] = serde_json::json!("A".repeat(86));
    canonical(&envelope)
}

fn parsed(bytes: &[u8]) -> serde_json::Value {
    pale_core::canon::parse_strict(bytes).expect("fixture parses")
}

fn set_body_payload(request: &mut serde_json::Value, handle: serde_json::Value) {
    let payload_sha256 = ArtifactDigest::hash_bytes(&canonical(&handle))
        .as_str()
        .to_owned();
    request["body"]["payload"] = handle;
    request["body"]["payload_sha256"] = serde_json::json!(payload_sha256);
}

struct Fixture {
    handle_sha256: String,
    handle_bytes: Vec<u8>,
    observation_bytes: Vec<u8>,
    request_bytes: Vec<u8>,
}

fn fixture() -> Fixture {
    let (handle_sha256, handle) = sealed_handle("operation-1");
    let observation = sealed_observation("operation-1", HANDLE_REF, &handle_sha256);
    let request_bytes = signed_common_request(&handle);
    Fixture {
        handle_sha256,
        handle_bytes: canonical(&handle),
        observation_bytes: canonical(&observation),
        request_bytes,
    }
}

fn request(fixture: &Fixture) -> ResidentOperationObservationRequest {
    ResidentOperationObservationRequest::new(
        OperationHandleLocator::new(HANDLE_REF.to_owned(), fixture.handle_sha256.clone()),
        fixture.request_bytes.clone(),
    )
}

#[derive(Clone)]
struct FakeClient {
    expected_request_bytes: Vec<u8>,
    response_handle_ref: String,
    handle_bytes: Vec<u8>,
    observation_bytes: Vec<u8>,
}

impl ResidentObservationClient for FakeClient {
    fn observe(
        &self,
        request: &ResidentOperationObservationRequest,
    ) -> Result<ResidentObservationResponse, ResidentAttachmentClientError> {
        assert_eq!(request.locator().operation_handle_record_ref(), HANDLE_REF);
        assert_eq!(request.signed_common_request(), self.expected_request_bytes);
        Ok(ResidentObservationResponse {
            operation_handle_record_ref: self.response_handle_ref.clone(),
            operation_handle_canonical_bytes: self.handle_bytes.clone(),
            operation_observation_record_ref: OBSERVATION_REF.to_owned(),
            operation_observation_canonical_bytes: self.observation_bytes.clone(),
        })
    }
}

fn attachment(fixture: &Fixture) -> ResidentAttachment {
    ResidentAttachment::from_client(FakeClient {
        expected_request_bytes: fixture.request_bytes.clone(),
        response_handle_ref: HANDLE_REF.to_owned(),
        handle_bytes: fixture.handle_bytes.clone(),
        observation_bytes: fixture.observation_bytes.clone(),
    })
}

fn operation_fixture(
    operation_handle_id: &str,
    operation_class: &str,
    parent: Option<(&str, &str, &Fixture)>,
) -> Fixture {
    operation_fixture_with_target(operation_handle_id, operation_class, parent, None)
}

fn operation_fixture_with_target(
    operation_handle_id: &str,
    operation_class: &str,
    parent: Option<(&str, &str, &Fixture)>,
    target: Option<&OperationHandleLocator>,
) -> Fixture {
    let (_, request_payload) = sealed_handle("operation-start-request-payload");
    let mut request_bytes = signed_common_request(&request_payload);
    if let Some(target) = target {
        request_bytes = operation_request_with_target(&request_bytes, target);
    }
    let request = parsed(&request_bytes);
    let mut handle = handle_preimage(operation_handle_id);
    handle["operation_class"] = serde_json::json!(operation_class);
    bind_handle_to_request(&mut handle, &request);
    if let Some((parent_id, parent_ref, parent)) = parent {
        handle["parent_operation"] = serde_json::json!({
            "operation_handle_id": parent_id,
            "operation_handle_ref": parent_ref,
            "operation_handle_sha256": parent.handle_sha256.clone(),
        });
    }
    let (handle_sha256, handle) = reseal_handle(handle);
    let observation = sealed_observation(operation_handle_id, HANDLE_REF, &handle_sha256);
    Fixture {
        handle_sha256,
        handle_bytes: canonical(&handle),
        observation_bytes: canonical(&observation),
        request_bytes,
    }
}

fn bind_handle_to_request(handle: &mut serde_json::Value, request: &serde_json::Value) {
    let binding = handle["request_binding"]
        .as_object_mut()
        .expect("handle request binding is an object");
    let request = request.as_object().expect("request is an object");
    let session = request["session_admission"]
        .as_object()
        .expect("session admission is an object");
    let route = request["route"]
        .as_object()
        .expect("request route is an object");
    for member in [
        "request_id",
        "correlation_id",
        "capability_id",
        "canonical_request_sha256",
    ] {
        binding.insert(member.to_owned(), request[member].clone());
    }
    for member in [
        "session_grant_id",
        "session_grant_sha256",
        "deployment_binding_ref",
        "deployment_binding_sha256",
    ] {
        binding.insert(member.to_owned(), session[member].clone());
    }
    for member in [
        "route_binding_schema_id",
        "route_binding_schema_sha256",
        "route_binding_ref",
        "route_binding_sha256",
        "route_generation",
        "contract_set_ref",
        "contract_set_sha256",
    ] {
        binding.insert(member.to_owned(), route[member].clone());
    }
}

fn operation_records(fixture: &Fixture) -> ResidentOperationFrameRecords {
    ResidentOperationFrameRecords::new(
        HANDLE_REF.to_owned(),
        String::from_utf8(fixture.handle_bytes.clone()).expect("handle is UTF-8 JSON"),
        OBSERVATION_REF.to_owned(),
        String::from_utf8(fixture.observation_bytes.clone()).expect("observation is UTF-8 JSON"),
    )
}

fn operation_locator(fixture: &Fixture) -> OperationHandleLocator {
    OperationHandleLocator::new(HANDLE_REF.to_owned(), fixture.handle_sha256.clone())
}

#[derive(Clone, Copy)]
enum OperationTargetFault {
    Missing,
    SubstitutedDigest,
    Duplicate,
}

fn operation_request_with_target(
    request_bytes: &[u8],
    locator: &OperationHandleLocator,
) -> Vec<u8> {
    let mut request = parsed(request_bytes);
    request["operation_target"] = serde_json::json!({
        "operation_handle_ref": locator.operation_handle_record_ref(),
        "operation_handle_sha256": locator.operation_handle_sha256(),
    });
    reseal_common_request(request)
}

fn operation_request_with_target_fault(
    request_bytes: &[u8],
    locator: &OperationHandleLocator,
    fault: OperationTargetFault,
) -> Vec<u8> {
    let mut request = parsed(request_bytes);
    match fault {
        OperationTargetFault::Missing => {
            request
                .as_object_mut()
                .expect("common request is an object")
                .remove("operation_target");
        }
        OperationTargetFault::SubstitutedDigest => {
            request["operation_target"] = serde_json::json!({
                "operation_handle_ref": locator.operation_handle_record_ref(),
                "operation_handle_sha256": if locator.operation_handle_sha256() == sha('0') {
                    sha('1')
                } else {
                    sha('0')
                },
            });
        }
        OperationTargetFault::Duplicate => {
            request["operation_target"] = serde_json::json!({
                "operation_handle_ref": locator.operation_handle_record_ref(),
                "operation_handle_sha256": locator.operation_handle_sha256(),
            });
            request["scope"]["authority_evidence"] = serde_json::json!([{
                "authority_ref": locator.operation_handle_record_ref(),
                "authority_sha256": locator.operation_handle_sha256(),
            }]);
        }
    }
    reseal_common_request(request)
}

fn cancellation_request(fixture: &Fixture) -> Vec<u8> {
    let mut request = parsed(&fixture.request_bytes);
    let handle = parsed(&fixture.handle_bytes);
    let handle_bytes = canonical(&handle);
    request["capability_id"] = serde_json::json!("operation.cancel");
    request["body"] = serde_json::json!({
        "schema_id": "pale/operation-handle.v1",
        "schema_sha256": schema_sha256("operation-handle.v1"),
        "payload": handle,
        "payload_sha256": ArtifactDigest::hash_bytes(&handle_bytes).as_str(),
    });
    request
        .as_object_mut()
        .expect("common request is an object")
        .remove("operation_target");
    reseal_common_request(request)
}

struct ExactOperationClient {
    expected: ResidentOperationFrameRequest,
    response: Mutex<Option<ResidentOperationFrameResponse>>,
}

impl ResidentOperationClient for ExactOperationClient {
    fn request(
        &self,
        request: &ResidentOperationFrameRequest,
    ) -> Result<ResidentOperationFrameResponse, ResidentOperationClientError> {
        assert_eq!(request, &self.expected);
        self.response
            .lock()
            .expect("operation response remains available")
            .take()
            .ok_or_else(|| ResidentOperationClientError::test("operation response was replayed"))
    }
}

struct NoSendOperationClient {
    calls: Arc<AtomicUsize>,
}

impl ResidentOperationClient for NoSendOperationClient {
    fn request(
        &self,
        _request: &ResidentOperationFrameRequest,
    ) -> Result<ResidentOperationFrameResponse, ResidentOperationClientError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(ResidentOperationClientError::test(
            "invalid request reached the operation carrier",
        ))
    }
}

fn operation_attachment(
    fixture: &Fixture,
    expected: ResidentOperationFrameRequest,
    response: ResidentOperationFrameResponse,
) -> ResidentAttachment {
    ResidentAttachment::from_clients(
        FakeClient {
            expected_request_bytes: fixture.request_bytes.clone(),
            response_handle_ref: HANDLE_REF.to_owned(),
            handle_bytes: fixture.handle_bytes.clone(),
            observation_bytes: fixture.observation_bytes.clone(),
        },
        ExactOperationClient {
            expected,
            response: Mutex::new(Some(response)),
        },
    )
}

#[derive(Clone)]
struct ApplicationFixture {
    handle_ref: String,
    handle_sha256: String,
    handle_bytes: Vec<u8>,
    observation_ref: String,
    observation_bytes: Vec<u8>,
    request_bytes: Vec<u8>,
}

fn application_fixture() -> ApplicationFixture {
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../crates/pale-application/tests/fixtures/resident/application-observe-sdk.v1.json"
    ))
    .expect("application-owned Observe fixture parses");
    let string = |name: &str| {
        value[name]
            .as_str()
            .unwrap_or_else(|| panic!("fixture member {name} is a string"))
            .to_owned()
    };
    assert_eq!(value["fixture_version"], 1);
    ApplicationFixture {
        handle_ref: string("operation_handle_record_ref"),
        handle_sha256: string("operation_handle_sha256"),
        handle_bytes: canonical(&value["operation_handle_record"]),
        observation_ref: string("operation_observation_record_ref"),
        observation_bytes: canonical(&value["operation_observation_record"]),
        request_bytes: canonical(&value["signed_common_request"]),
    }
}

fn application_invoke_fixture() -> (
    ResidentOperationFrameRequest,
    ResidentOperationFrameResponse,
) {
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../crates/pale-application/tests/fixtures/resident/application-invoke-sdk.v1.json"
    ))
    .expect("application-owned Invoke fixture parses");
    assert_eq!(value["fixture_version"], 1);
    let request = serde_json::from_value(value["request_frame"].clone())
        .expect("application-owned Invoke request frame decodes");
    let response = serde_json::from_value(value["response_frame"].clone())
        .expect("application-owned Invoke response frame decodes");
    (request, response)
}

fn application_invoke_advance_fixture() -> (
    ResidentOperationFrameRequest,
    ResidentOperationFrameResponse,
) {
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../crates/pale-application/tests/fixtures/resident/application-invoke-advance-sdk.v1.json"
    ))
    .expect("application-owned AdvanceInvoke fixture parses");
    assert_eq!(value["fixture_version"], 1);
    let request = serde_json::from_value(value["request_frame"].clone())
        .expect("application-owned AdvanceInvoke request frame decodes");
    let response = serde_json::from_value(value["response_frame"].clone())
        .expect("application-owned AdvanceInvoke response frame decodes");
    (request, response)
}

fn application_delegate_fixture() -> (
    ResidentOperationFrameRequest,
    ResidentOperationFrameResponse,
) {
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../crates/pale-application/tests/fixtures/resident/application-delegate-sdk.v1.json"
    ))
    .expect("application-owned Delegate fixture parses");
    assert_eq!(value["fixture_version"], 1);
    let request = serde_json::from_value(value["request_frame"].clone())
        .expect("application-owned Delegate request frame decodes");
    let response = serde_json::from_value(value["response_frame"].clone())
        .expect("application-owned Delegate response frame decodes");
    (request, response)
}

fn application_delegate_advance_fixture() -> (
    ResidentOperationFrameRequest,
    ResidentOperationFrameResponse,
) {
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../crates/pale-application/tests/fixtures/resident/application-delegate-advance-sdk.v1.json"
    ))
    .expect("application-owned AdvanceDelegate fixture parses");
    assert_eq!(value["fixture_version"], 1);
    let request = serde_json::from_value(value["request_frame"].clone())
        .expect("application-owned AdvanceDelegate request frame decodes");
    let response = serde_json::from_value(value["response_frame"].clone())
        .expect("application-owned AdvanceDelegate response frame decodes");
    (request, response)
}

fn application_cancel_fixture() -> (
    ResidentOperationFrameRequest,
    ResidentOperationFrameResponse,
) {
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../crates/pale-application/tests/fixtures/resident/application-cancel-sdk.v1.json"
    ))
    .expect("application-owned Cancel fixture parses");
    assert_eq!(value["fixture_version"], 1);
    let request = serde_json::from_value(value["request_frame"].clone())
        .expect("application-owned Cancel request frame decodes");
    let response = serde_json::from_value(value["response_frame"].clone())
        .expect("application-owned Cancel response frame decodes");
    (request, response)
}

fn application_request(fixture: &ApplicationFixture) -> ResidentOperationObservationRequest {
    ResidentOperationObservationRequest::new(
        OperationHandleLocator::new(fixture.handle_ref.clone(), fixture.handle_sha256.clone()),
        fixture.request_bytes.clone(),
    )
}

#[derive(Clone)]
struct ApplicationFixtureClient {
    fixture: ApplicationFixture,
}

struct ScriptedFramePort {
    sent: Arc<Mutex<Vec<Vec<u8>>>>,
    response: Option<Result<Box<[u8]>, LocalIpcCarrierError>>,
    receive_calls: Arc<AtomicUsize>,
}

struct SequencedFramePort {
    sent: Arc<Mutex<Vec<Vec<u8>>>>,
    responses: VecDeque<Result<Box<[u8]>, LocalIpcCarrierError>>,
    send_events: mpsc::Sender<()>,
}

impl LocalIpcFramePort for SequencedFramePort {
    fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
        Ok(LocalIpcPeerCredentials::new(501, 20, Some(7)))
    }

    fn send_frame(
        &mut self,
        payload: &[u8],
        _deadline: Instant,
    ) -> Result<(), LocalIpcCarrierError> {
        self.sent
            .lock()
            .expect("sequenced send log remains available")
            .push(payload.to_vec());
        self.send_events
            .send(())
            .expect("test observer retains the send receiver");
        Ok(())
    }

    fn receive_frame(&mut self, _deadline: Instant) -> Result<Box<[u8]>, LocalIpcCarrierError> {
        self.responses
            .pop_front()
            .unwrap_or(Err(LocalIpcCarrierError::ConnectionUnusable))
    }
}

impl LocalIpcFramePort for ScriptedFramePort {
    fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
        Ok(LocalIpcPeerCredentials::new(501, 20, Some(7)))
    }

    fn send_frame(
        &mut self,
        payload: &[u8],
        _deadline: Instant,
    ) -> Result<(), LocalIpcCarrierError> {
        self.sent
            .lock()
            .expect("scripted send log remains available")
            .push(payload.to_vec());
        Ok(())
    }

    fn receive_frame(&mut self, _deadline: Instant) -> Result<Box<[u8]>, LocalIpcCarrierError> {
        self.receive_calls.fetch_add(1, Ordering::SeqCst);
        self.response
            .take()
            .unwrap_or(Err(LocalIpcCarrierError::ConnectionUnusable))
    }
}

struct ExactFixtureResponseAdmission {
    calls: Arc<AtomicUsize>,
    handle_ref: String,
    observation_ref: String,
}

impl ResidentResponseAdmission for ExactFixtureResponseAdmission {
    fn admit(
        &self,
        operation_handle_record_ref: &str,
        operation_handle_canonical_bytes: &[u8],
        operation_observation_record_ref: &str,
        operation_observation_canonical_bytes: &[u8],
        deadline: Instant,
    ) -> Result<(), ResidentResponseAdmissionFailure> {
        assert!(
            Instant::now() < deadline,
            "admission receives a live deadline"
        );
        self.calls.fetch_add(1, Ordering::SeqCst);
        let expected = "A".repeat(86);
        if operation_handle_record_ref == self.handle_ref
            && operation_observation_record_ref == self.observation_ref
            && parsed(operation_handle_canonical_bytes)["signature"].as_str()
                == Some(expected.as_str())
            && parsed(operation_observation_canonical_bytes)["signature"].as_str()
                == Some(expected.as_str())
        {
            Ok(())
        } else {
            Err(ResidentResponseAdmissionFailure)
        }
    }
}

struct ExactApplicationRecordsAdmission {
    calls: Arc<AtomicUsize>,
    handle_ref: String,
    handle_bytes: Vec<u8>,
    observation_ref: String,
    observation_bytes: Vec<u8>,
}

impl ResidentResponseAdmission for ExactApplicationRecordsAdmission {
    fn admit(
        &self,
        operation_handle_record_ref: &str,
        operation_handle_canonical_bytes: &[u8],
        operation_observation_record_ref: &str,
        operation_observation_canonical_bytes: &[u8],
        deadline: Instant,
    ) -> Result<(), ResidentResponseAdmissionFailure> {
        assert!(
            Instant::now() < deadline,
            "admission receives a live deadline"
        );
        self.calls.fetch_add(1, Ordering::SeqCst);
        if operation_handle_record_ref == self.handle_ref
            && operation_handle_canonical_bytes == self.handle_bytes.as_slice()
            && operation_observation_record_ref == self.observation_ref
            && operation_observation_canonical_bytes == self.observation_bytes.as_slice()
        {
            Ok(())
        } else {
            Err(ResidentResponseAdmissionFailure)
        }
    }
}

struct BlockingFirstResponseAdmission {
    inner: ExactFixtureResponseAdmission,
    entered: Arc<(Mutex<bool>, Condvar)>,
    release: Arc<(Mutex<bool>, Condvar)>,
}

impl ResidentResponseAdmission for BlockingFirstResponseAdmission {
    fn admit(
        &self,
        operation_handle_record_ref: &str,
        operation_handle_canonical_bytes: &[u8],
        operation_observation_record_ref: &str,
        operation_observation_canonical_bytes: &[u8],
        deadline: Instant,
    ) -> Result<(), ResidentResponseAdmissionFailure> {
        let first = self.inner.calls.load(Ordering::SeqCst) == 0;
        self.inner.admit(
            operation_handle_record_ref,
            operation_handle_canonical_bytes,
            operation_observation_record_ref,
            operation_observation_canonical_bytes,
            deadline,
        )?;
        if first {
            let (entered_lock, entered_changed) = &*self.entered;
            *entered_lock.lock().expect("entry signal remains available") = true;
            entered_changed.notify_all();

            let (release_lock, release_changed) = &*self.release;
            let mut released = release_lock
                .lock()
                .expect("release signal remains available");
            while !*released {
                released = release_changed
                    .wait(released)
                    .expect("release wait remains available");
            }
        }
        Ok(())
    }
}

fn framed_response(fixture: &ApplicationFixture) -> Box<[u8]> {
    canonical(
        &serde_json::to_value(ResidentObserveFrameResponse::Observed {
            operation_handle_record_ref: fixture.handle_ref.clone(),
            operation_handle_canonical: String::from_utf8(fixture.handle_bytes.clone())
                .expect("fixture handle is UTF-8 JSON"),
            operation_observation_record_ref: fixture.observation_ref.clone(),
            operation_observation_canonical: String::from_utf8(fixture.observation_bytes.clone())
                .expect("fixture observation is UTF-8 JSON"),
        })
        .expect("frame response serializes"),
    )
    .into_boxed_slice()
}

impl ResidentObservationClient for ApplicationFixtureClient {
    fn observe(
        &self,
        request: &ResidentOperationObservationRequest,
    ) -> Result<ResidentObservationResponse, ResidentAttachmentClientError> {
        assert_eq!(
            request.locator().operation_handle_record_ref(),
            self.fixture.handle_ref
        );
        assert_eq!(
            request.locator().operation_handle_sha256(),
            self.fixture.handle_sha256
        );
        assert_eq!(request.signed_common_request(), self.fixture.request_bytes);
        Ok(ResidentObservationResponse {
            operation_handle_record_ref: self.fixture.handle_ref.clone(),
            operation_handle_canonical_bytes: self.fixture.handle_bytes.clone(),
            operation_observation_record_ref: self.fixture.observation_ref.clone(),
            operation_observation_canonical_bytes: self.fixture.observation_bytes.clone(),
        })
    }
}

#[derive(Clone)]
struct SignatureAdmittingApplicationClient {
    fixture: ApplicationFixture,
}

impl SignatureAdmittingApplicationClient {
    fn reject(
        message: &'static str,
        transmission: ResidentRequestTransmissionCertainty,
    ) -> ResidentAttachmentClientError {
        ResidentAttachmentClientError {
            source: std::io::Error::other(message).into(),
            transmission,
        }
    }

    fn admits_fixture_signature(bytes: &[u8]) -> bool {
        let expected = "A".repeat(86);
        parsed(bytes)["signature"].as_str() == Some(expected.as_str())
    }
}

impl ResidentObservationClient for SignatureAdmittingApplicationClient {
    fn observe(
        &self,
        request: &ResidentOperationObservationRequest,
    ) -> Result<ResidentObservationResponse, ResidentAttachmentClientError> {
        let expected = "A".repeat(86);
        if parsed(request.signed_common_request())["caller_signature"].as_str()
            != Some(expected.as_str())
        {
            return Err(Self::reject(
                "request signature rejected",
                ResidentRequestTransmissionCertainty::NotSent,
            ));
        }
        if !Self::admits_fixture_signature(&self.fixture.handle_bytes)
            || !Self::admits_fixture_signature(&self.fixture.observation_bytes)
        {
            return Err(Self::reject(
                "response signature rejected",
                ResidentRequestTransmissionCertainty::MayHaveBeenSent,
            ));
        }
        Ok(ResidentObservationResponse {
            operation_handle_record_ref: self.fixture.handle_ref.clone(),
            operation_handle_canonical_bytes: self.fixture.handle_bytes.clone(),
            operation_observation_record_ref: self.fixture.observation_ref.clone(),
            operation_observation_canonical_bytes: self.fixture.observation_bytes.clone(),
        })
    }
}

fn assert_request_rejected(fixture: &Fixture, locator_digest: String, request_bytes: Vec<u8>) {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let candidate = ResidentOperationObservationRequest::new(
        OperationHandleLocator::new(HANDLE_REF.to_owned(), locator_digest),
        request_bytes,
    );
    let error = api
        .observe_resident_operation(Some(&attachment(fixture)), candidate)
        .expect_err("invalid request must not reach the resident client");
    assert_eq!(
        error.diagnostic().code(),
        "resident_observation_request_invalid"
    );
}

#[test]
fn point_addressed_operations_reject_unsigned_or_ambiguous_targets_without_sending() {
    #[derive(Clone, Copy)]
    enum PointAddressedOperation {
        AdvanceInvoke,
        AdvanceConsult,
        StartDelegate,
        AdvanceDelegate,
    }

    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = operation_fixture("target-operation-1", "invoke", None);
    let locator = operation_locator(&fixture);
    let operation_calls = Arc::new(AtomicUsize::new(0));
    let attachment = ResidentAttachment::from_clients(
        FakeClient {
            expected_request_bytes: fixture.request_bytes.clone(),
            response_handle_ref: HANDLE_REF.to_owned(),
            handle_bytes: fixture.handle_bytes.clone(),
            observation_bytes: fixture.observation_bytes.clone(),
        },
        NoSendOperationClient {
            calls: Arc::clone(&operation_calls),
        },
    );
    let original_body = parsed(&fixture.request_bytes)["body"].clone();

    for operation in [
        PointAddressedOperation::AdvanceInvoke,
        PointAddressedOperation::AdvanceConsult,
        PointAddressedOperation::StartDelegate,
        PointAddressedOperation::AdvanceDelegate,
    ] {
        for fault in [
            OperationTargetFault::Missing,
            OperationTargetFault::SubstitutedDigest,
            OperationTargetFault::Duplicate,
        ] {
            let signed_request =
                operation_request_with_target_fault(&fixture.request_bytes, &locator, fault);
            assert_eq!(
                parsed(&signed_request)["body"],
                original_body,
                "the operation target must not replace an operation-specific body"
            );
            let result = match operation {
                PointAddressedOperation::AdvanceInvoke => api
                    .advance_resident_invoke(
                        Some(&attachment),
                        ResidentInvokeAdvanceRequest::new(locator.clone(), signed_request),
                    )
                    .map(|_| ()),
                PointAddressedOperation::AdvanceConsult => api
                    .advance_resident_consult(
                        Some(&attachment),
                        ResidentConsultAdvanceRequest::new(locator.clone(), signed_request),
                    )
                    .map(|_| ()),
                PointAddressedOperation::StartDelegate => api
                    .start_resident_delegate(
                        Some(&attachment),
                        ResidentDelegateStartRequest::new(locator.clone(), signed_request),
                    )
                    .map(|_| ()),
                PointAddressedOperation::AdvanceDelegate => api
                    .advance_resident_delegate(
                        Some(&attachment),
                        ResidentDelegateAdvanceRequest::new(locator.clone(), signed_request),
                    )
                    .map(|_| ()),
            };
            let error = result.expect_err("an unsigned or ambiguous target must be rejected");
            assert_eq!(
                error.diagnostic().code(),
                "resident_operation_request_invalid"
            );
        }
    }

    assert_eq!(
        operation_calls.load(Ordering::SeqCst),
        0,
        "invalid operation targets must fail before carrier I/O"
    );
}

#[test]
fn cancel_rejects_unbound_control_requests_without_sending() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = operation_fixture("cancel-target-operation-1", "invoke", None);
    let locator = operation_locator(&fixture);
    let operation_calls = Arc::new(AtomicUsize::new(0));
    let attachment = ResidentAttachment::from_clients(
        FakeClient {
            expected_request_bytes: fixture.request_bytes.clone(),
            response_handle_ref: HANDLE_REF.to_owned(),
            handle_bytes: fixture.handle_bytes.clone(),
            observation_bytes: fixture.observation_bytes.clone(),
        },
        NoSendOperationClient {
            calls: Arc::clone(&operation_calls),
        },
    );
    let exact = cancellation_request(&fixture);
    let mut wrong_capability = parsed(&exact);
    wrong_capability["capability_id"] = serde_json::json!("operation.observe");
    let wrong_capability = reseal_common_request(wrong_capability);
    let mut changed_payload = parsed(&exact);
    changed_payload["body"]["payload"]["created_at"] = serde_json::json!(11);
    let changed_payload = reseal_common_request(changed_payload);
    let mut continuation_shaped = parsed(&exact);
    continuation_shaped["operation_target"] = serde_json::json!({
        "operation_handle_ref": locator.operation_handle_record_ref(),
        "operation_handle_sha256": locator.operation_handle_sha256(),
    });
    let continuation_shaped = reseal_common_request(continuation_shaped);

    for (candidate_locator, signed_request) in [
        (locator.clone(), wrong_capability),
        (locator.clone(), changed_payload),
        (locator.clone(), continuation_shaped),
        (
            OperationHandleLocator::new(locator.operation_handle_record_ref().to_owned(), sha('0')),
            exact,
        ),
    ] {
        let error = api
            .cancel_resident_operation(
                Some(&attachment),
                ResidentOperationCancellationRequest::new(candidate_locator, signed_request),
            )
            .expect_err("an unbound Cancel request must be rejected before carrier I/O");
        assert_eq!(
            error.diagnostic().code(),
            "resident_operation_request_invalid"
        );
    }
    assert_eq!(
        operation_calls.load(Ordering::SeqCst),
        0,
        "invalid Cancel requests must fail before carrier I/O"
    );
}

#[test]
fn bounded_consult_start_and_policy_boundary_preserve_the_exact_handle() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = operation_fixture("consult-operation-1", "consult", None);
    let signed_request =
        String::from_utf8(fixture.request_bytes.clone()).expect("signed request is UTF-8 JSON");
    let start_frame = ResidentOperationFrameRequest::StartConsult {
        signed_common_request: signed_request.clone(),
    };
    let attachment = operation_attachment(
        &fixture,
        start_frame,
        ResidentOperationFrameResponse::Started {
            records: operation_records(&fixture),
        },
    );
    assert!(attachment.supports_operation_requests());
    let started = api
        .start_resident_consult(
            Some(&attachment),
            ResidentConsultStartRequest::new(fixture.request_bytes.clone()),
        )
        .expect("Consult start returns its durable records");
    assert_eq!(started.operation_handle_record_ref(), HANDLE_REF);

    let locator = operation_locator(&fixture);
    let advance_request = operation_request_with_target(&fixture.request_bytes, &locator);
    let frame_locator =
        ResidentOperationFrameLocator::new(HANDLE_REF.to_owned(), fixture.handle_sha256.clone());
    let advance_frame = ResidentOperationFrameRequest::AdvanceConsult {
        locator: frame_locator.clone(),
        signed_common_request: String::from_utf8(advance_request.clone())
            .expect("target-bound request is UTF-8 JSON"),
    };
    let attachment = operation_attachment(
        &fixture,
        advance_frame,
        ResidentOperationFrameResponse::Rejected {
            failure: ResidentOperationFrameFailure::ConfigurationUnavailable,
        },
    );
    let error = api
        .advance_resident_consult(
            Some(&attachment),
            ResidentConsultAdvanceRequest::new(locator, advance_request),
        )
        .expect_err("an unavailable compiled policy is not delegated to the caller");
    assert_eq!(
        error.diagnostic().code(),
        "resident_operation_configuration_unavailable"
    );
}

#[test]
fn bounded_invoke_returns_its_durable_handle_before_one_honest_pending_turn() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = operation_fixture("invoke-operation-1", "invoke", None);
    let signed_request =
        String::from_utf8(fixture.request_bytes.clone()).expect("signed request is UTF-8 JSON");
    let attachment = operation_attachment(
        &fixture,
        ResidentOperationFrameRequest::StartInvoke {
            signed_common_request: signed_request.clone(),
        },
        ResidentOperationFrameResponse::Started {
            records: operation_records(&fixture),
        },
    );
    let started = api
        .start_resident_invoke(
            Some(&attachment),
            ResidentInvokeStartRequest::new(fixture.request_bytes.clone()),
        )
        .expect("Invoke start returns its durable handle before dispatch");
    assert_eq!(started.operation_handle_record_ref(), HANDLE_REF);

    let locator = operation_locator(&fixture);
    let advance_request = operation_request_with_target(&fixture.request_bytes, &locator);
    let frame_locator =
        ResidentOperationFrameLocator::new(HANDLE_REF.to_owned(), fixture.handle_sha256.clone());
    let attachment = operation_attachment(
        &fixture,
        ResidentOperationFrameRequest::AdvanceInvoke {
            locator: frame_locator.clone(),
            signed_common_request: String::from_utf8(advance_request.clone())
                .expect("target-bound request is UTF-8 JSON"),
        },
        ResidentOperationFrameResponse::Pending {
            locator: frame_locator,
        },
    );
    let outcome = api
        .advance_resident_invoke(
            Some(&attachment),
            ResidentInvokeAdvanceRequest::new(locator, advance_request),
        )
        .expect("Invoke continuation preserves its honest pending boundary");
    let ResidentInvokeAdvanceOutcome::Pending { locator } = outcome else {
        panic!("Invoke continuation must not fabricate a terminal result")
    };
    assert_eq!(locator.operation_handle_sha256(), fixture.handle_sha256);
}

#[test]
fn invoke_rejects_delegate_only_unsupported_reasons() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = operation_fixture("invoke-operation-wrong-boundary", "invoke", None);
    let locator = operation_locator(&fixture);
    let advance_request = operation_request_with_target(&fixture.request_bytes, &locator);
    let frame_locator =
        ResidentOperationFrameLocator::new(HANDLE_REF.to_owned(), fixture.handle_sha256.clone());
    let attachment = operation_attachment(
        &fixture,
        ResidentOperationFrameRequest::AdvanceInvoke {
            locator: frame_locator.clone(),
            signed_common_request: String::from_utf8(advance_request.clone())
                .expect("target-bound request is UTF-8 JSON"),
        },
        ResidentOperationFrameResponse::Unsupported {
            locator: frame_locator,
            reason: ResidentOperationFrameUnsupportedReason::DelegateRouteNotSelected,
        },
    );

    let error = api
        .advance_resident_invoke(
            Some(&attachment),
            ResidentInvokeAdvanceRequest::new(locator, advance_request),
        )
        .expect_err("Invoke cannot expose a Delegate-only owner boundary");
    assert_eq!(
        error.diagnostic().code(),
        "resident_operation_response_invalid"
    );
    assert_eq!(
        error.diagnostic().disposition(),
        DiagnosticDisposition::EffectOutcomeUnknown
    );
    assert_eq!(error.diagnostic().retry(), DiagnosticRetry::ReconcileFirst);
    assert_eq!(
        error.diagnostic().effect(),
        DiagnosticEffectCertainty::OutcomeUnknown
    );
}

#[test]
fn bounded_delegate_start_and_selected_continuation_keep_parent_and_child_distinct() {
    const PARENT_REF: &str = "operations/parent-handle";
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let parent = fixture();
    let parent_locator =
        OperationHandleLocator::new(PARENT_REF.to_owned(), parent.handle_sha256.clone());
    let child = operation_fixture_with_target(
        "delegate-operation-1",
        "delegate",
        Some(("operation-1", PARENT_REF, &parent)),
        Some(&parent_locator),
    );
    let start_request = child.request_bytes.clone();
    let start_frame = ResidentOperationFrameRequest::StartDelegate {
        parent: ResidentOperationFrameLocator::new(
            PARENT_REF.to_owned(),
            parent.handle_sha256.clone(),
        ),
        signed_common_request: String::from_utf8(start_request.clone())
            .expect("parent-bound request is UTF-8 JSON"),
    };
    let attachment = operation_attachment(
        &child,
        start_frame,
        ResidentOperationFrameResponse::Started {
            records: operation_records(&child),
        },
    );
    let started = api
        .start_resident_delegate(
            Some(&attachment),
            ResidentDelegateStartRequest::new(parent_locator, start_request),
        )
        .expect("Delegate start returns its durable child before dispatch");
    assert_eq!(started.operation_handle_record_ref(), HANDLE_REF);

    let locator = operation_locator(&child);
    let advance_request = operation_request_with_target(&child.request_bytes, &locator);
    let frame_locator =
        ResidentOperationFrameLocator::new(HANDLE_REF.to_owned(), child.handle_sha256.clone());
    let attachment = operation_attachment(
        &child,
        ResidentOperationFrameRequest::AdvanceDelegate {
            locator: frame_locator.clone(),
            signed_common_request: String::from_utf8(advance_request.clone())
                .expect("child-bound request is UTF-8 JSON"),
        },
        ResidentOperationFrameResponse::Pending {
            locator: frame_locator,
        },
    );
    let outcome = api
        .advance_resident_delegate(
            Some(&attachment),
            ResidentDelegateAdvanceRequest::new(locator, advance_request),
        )
        .expect("already-selected Delegate continuation remains pending");
    let ResidentDelegateAdvanceOutcome::Pending { locator } = outcome else {
        panic!("selected provider continuation must remain an honest pending turn")
    };
    assert_eq!(locator.operation_handle_sha256(), child.handle_sha256);
}

#[test]
fn resident_start_rejects_a_same_class_handle_for_another_signed_request() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = operation_fixture("consult-operation-1", "consult", None);
    let signed_request =
        String::from_utf8(fixture.request_bytes.clone()).expect("signed request is UTF-8 JSON");
    let expected_frame = ResidentOperationFrameRequest::StartConsult {
        signed_common_request: signed_request,
    };

    let mut substituted_handle = parsed(&fixture.handle_bytes);
    substituted_handle["request_binding"]["request_id"] =
        serde_json::json!("another-start-request");
    let (substituted_sha256, substituted_handle) = reseal_handle(substituted_handle);
    let substituted_observation =
        sealed_observation("consult-operation-1", HANDLE_REF, &substituted_sha256);
    let substituted_records = ResidentOperationFrameRecords::new(
        HANDLE_REF.to_owned(),
        String::from_utf8(canonical(&substituted_handle)).expect("handle is UTF-8 JSON"),
        OBSERVATION_REF.to_owned(),
        String::from_utf8(canonical(&substituted_observation)).expect("observation is UTF-8 JSON"),
    );
    let attachment = operation_attachment(
        &fixture,
        expected_frame,
        ResidentOperationFrameResponse::Started {
            records: substituted_records,
        },
    );

    let error = api
        .start_resident_consult(
            Some(&attachment),
            ResidentConsultStartRequest::new(fixture.request_bytes.clone()),
        )
        .expect_err("a same-class handle cannot substitute another request binding");
    assert_eq!(
        error.diagnostic().code(),
        "resident_operation_response_invalid"
    );
}

#[test]
fn resident_operation_rejections_preserve_distinct_recovery_semantics() {
    let cases = [
        (
            pale_ports::ResidentOperationFrameFailure::InvalidRequest,
            "resident_operation_request_rejected",
            DiagnosticStage::ResidentAdmission,
            DiagnosticDisposition::InvalidInput,
            DiagnosticRetry::AfterCorrection,
            DiagnosticEffectCertainty::NotStarted,
        ),
        (
            pale_ports::ResidentOperationFrameFailure::ReplayConflict,
            "resident_operation_replay_conflict",
            DiagnosticStage::ResidentAdmission,
            DiagnosticDisposition::InvalidInput,
            DiagnosticRetry::AfterCorrection,
            DiagnosticEffectCertainty::NotStarted,
        ),
        (
            pale_ports::ResidentOperationFrameFailure::AdmissionDenied,
            "resident_operation_admission_denied",
            DiagnosticStage::ResidentAdmission,
            DiagnosticDisposition::Blocked,
            DiagnosticRetry::AfterStateChange,
            DiagnosticEffectCertainty::NotStarted,
        ),
        (
            pale_ports::ResidentOperationFrameFailure::ResidentUnavailable,
            "resident_operation_outcome_unknown",
            DiagnosticStage::ResidentRecovery,
            DiagnosticDisposition::EffectOutcomeUnknown,
            DiagnosticRetry::ReconcileFirst,
            DiagnosticEffectCertainty::OutcomeUnknown,
        ),
    ];

    for (failure, code, stage, disposition, retry, effect) in cases {
        let api = PaleApi::new().expect("embedded API dependencies are coherent");
        let fixture = operation_fixture("invoke-operation-rejection", "invoke", None);
        let expected = ResidentOperationFrameRequest::StartInvoke {
            signed_common_request: String::from_utf8(fixture.request_bytes.clone())
                .expect("signed request is UTF-8 JSON"),
        };
        let attachment = operation_attachment(
            &fixture,
            expected,
            ResidentOperationFrameResponse::Rejected { failure },
        );

        let error = api
            .start_resident_invoke(
                Some(&attachment),
                ResidentInvokeStartRequest::new(fixture.request_bytes.clone()),
            )
            .expect_err("resident rejection remains a public diagnostic");
        let diagnostic = error.diagnostic();
        assert_eq!(diagnostic.code(), code);
        assert_eq!(diagnostic.stage(), stage);
        assert_eq!(diagnostic.disposition(), disposition);
        assert_eq!(diagnostic.retry(), retry);
        assert_eq!(diagnostic.effect(), effect);
        if code == "resident_operation_outcome_unknown" {
            assert!(diagnostic.help().iter().any(|help| {
                help.contains("resubmit the exact signed Start request bytes")
                    && help.contains("do not mint a new request identity")
            }));
        }
    }
}

#[test]
fn bounded_delegate_and_cancel_results_preserve_closed_owner_dispositions() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let delegate = operation_fixture("delegate-operation-1", "delegate", None);
    let locator = operation_locator(&delegate);
    let advance_request = operation_request_with_target(&delegate.request_bytes, &locator);
    let frame_locator =
        ResidentOperationFrameLocator::new(HANDLE_REF.to_owned(), delegate.handle_sha256.clone());
    let attachment = operation_attachment(
        &delegate,
        ResidentOperationFrameRequest::AdvanceDelegate {
            locator: frame_locator.clone(),
            signed_common_request: String::from_utf8(advance_request.clone())
                .expect("target-bound request is UTF-8 JSON"),
        },
        ResidentOperationFrameResponse::Unsupported {
            locator: frame_locator.clone(),
            reason: ResidentOperationFrameUnsupportedReason::DelegateRouteNotSelected,
        },
    );
    let outcome = api
        .advance_resident_delegate(
            Some(&attachment),
            ResidentDelegateAdvanceRequest::new(locator, advance_request),
        )
        .expect("route absence remains a typed unsupported result");
    assert!(matches!(
        outcome,
        ResidentDelegateAdvanceOutcome::Unsupported {
            reason: ResidentOperationUnsupportedReason::DelegateRouteNotSelected,
            ..
        }
    ));

    let locator = operation_locator(&delegate);
    let cancel_request = cancellation_request(&delegate);
    let attachment = operation_attachment(
        &delegate,
        ResidentOperationFrameRequest::Cancel {
            locator: frame_locator.clone(),
            signed_common_request: String::from_utf8(cancel_request.clone())
                .expect("signed Cancel request is UTF-8 JSON"),
        },
        ResidentOperationFrameResponse::CancellationDisposition {
            locator: frame_locator,
            disposition: ResidentOperationFrameCancellationDisposition::RequestRetained,
        },
    );
    let cancelled = api
        .cancel_resident_operation(
            Some(&attachment),
            ResidentOperationCancellationRequest::new(locator, cancel_request),
        )
        .expect("cancellation acknowledgement remains nonterminal");
    assert_eq!(
        cancelled.disposition(),
        ResidentOperationCancellationDisposition::RequestRetained
    );
}

#[test]
fn terminal_cancellation_disposition_preserves_its_exact_meaning_and_spelling() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = operation_fixture("terminal-operation-1", "invoke", None);
    let locator = operation_locator(&fixture);
    let cancel_request = cancellation_request(&fixture);
    let frame_locator =
        ResidentOperationFrameLocator::new(HANDLE_REF.to_owned(), fixture.handle_sha256.clone());
    let attachment = operation_attachment(
        &fixture,
        ResidentOperationFrameRequest::Cancel {
            locator: frame_locator.clone(),
            signed_common_request: String::from_utf8(cancel_request.clone())
                .expect("signed Cancel request is UTF-8 JSON"),
        },
        ResidentOperationFrameResponse::CancellationDisposition {
            locator: frame_locator,
            disposition: ResidentOperationFrameCancellationDisposition::TerminalAlreadyEstablished,
        },
    );

    let cancelled = api
        .cancel_resident_operation(
            Some(&attachment),
            ResidentOperationCancellationRequest::new(locator, cancel_request),
        )
        .expect("an existing terminal observation remains a complete result");

    assert_eq!(
        cancelled.disposition(),
        ResidentOperationCancellationDisposition::TerminalAlreadyEstablished
    );
    let encoded = serde_json::to_value(&cancelled).expect("the SDK result serializes");
    assert_eq!(
        encoded["disposition"],
        serde_json::json!("terminal_already_established")
    );
    assert!(
        serde_json::from_str::<ResidentOperationFrameCancellationDisposition>("\"established\"")
            .is_err(),
        "the ambiguous frame spelling is not accepted",
    );
}

#[test]
fn operation_request_requires_an_operation_capable_attachment_and_exact_locator() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = operation_fixture("consult-operation-1", "consult", None);
    let unavailable = api
        .start_resident_consult(
            Some(&attachment(&fixture)),
            ResidentConsultStartRequest::new(fixture.request_bytes.clone()),
        )
        .expect_err("Observe-only attachment cannot start operations");
    assert_eq!(
        unavailable.diagnostic().code(),
        "resident_operation_unavailable"
    );

    let expected = operation_locator(&fixture);
    let advance_request = operation_request_with_target(&fixture.request_bytes, &expected);
    let substituted = ResidentOperationFrameLocator::new(
        "operations/substituted".to_owned(),
        fixture.handle_sha256.clone(),
    );
    let attachment = operation_attachment(
        &fixture,
        ResidentOperationFrameRequest::AdvanceConsult {
            locator: ResidentOperationFrameLocator::new(
                HANDLE_REF.to_owned(),
                fixture.handle_sha256.clone(),
            ),
            signed_common_request: String::from_utf8(advance_request.clone())
                .expect("target-bound request is UTF-8 JSON"),
        },
        ResidentOperationFrameResponse::Pending {
            locator: substituted,
        },
    );
    let invalid = api
        .advance_resident_consult(
            Some(&attachment),
            ResidentConsultAdvanceRequest::new(expected, advance_request),
        )
        .expect_err("resident cannot substitute the target locator");
    assert_eq!(
        invalid.diagnostic().code(),
        "resident_operation_response_invalid"
    );
}

#[test]
fn absent_attachment_has_one_canonical_unavailable_diagnostic() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = fixture();
    let error = api
        .observe_resident_operation(None, request(&fixture))
        .expect_err("an explicit attachment is required");
    assert_eq!(
        error.diagnostic().code(),
        "resident_observation_unavailable"
    );
    assert_eq!(
        error.diagnostic().stage(),
        crate::DiagnosticStage::ResidentAdmission
    );
    assert_eq!(
        error.diagnostic().disposition(),
        crate::DiagnosticDisposition::Blocked
    );
    assert_eq!(
        error.diagnostic().retry(),
        crate::DiagnosticRetry::AfterStateChange
    );
    assert_eq!(
        error.diagnostic().effect(),
        crate::DiagnosticEffectCertainty::NotApplicable
    );
}

#[test]
fn exact_request_preflights_without_an_attachment_or_live_capability() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = fixture();
    let preflight = api
        .preflight_resident_operation_observation(request(&fixture))
        .expect("exact request passes structural preflight");

    assert_eq!(
        preflight.request.locator().operation_handle_record_ref(),
        HANDLE_REF
    );
    assert_eq!(
        preflight.request.locator().operation_handle_sha256(),
        fixture.handle_sha256
    );
    assert_eq!(
        preflight.request.signed_common_request(),
        fixture.request_bytes
    );
    assert_eq!(
        format!("{preflight:?}"),
        format!(
            "ResidentOperationObservationPreflight {{ locator: OperationHandleLocator {{ operation_handle_record_ref: \"{HANDLE_REF}\", operation_handle_sha256: \"{}\" }}, signed_common_request_byte_len: {} }}",
            fixture.handle_sha256,
            fixture.request_bytes.len()
        )
    );
}

#[test]
fn malformed_request_fails_preflight_before_attachment_discovery() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = fixture();
    let malformed_request = ResidentOperationObservationRequest::new(
        OperationHandleLocator::new(HANDLE_REF.to_owned(), fixture.handle_sha256),
        br#"{\"envelope_schema_id\":\"pale/common-request-envelope.v1\"}"#.to_vec(),
    );
    let error = api
        .preflight_resident_operation_observation(malformed_request)
        .expect_err("an incomplete envelope cannot become a preflight state");

    assert_eq!(
        error.diagnostic().code(),
        "resident_observation_request_invalid"
    );
    assert_eq!(
        error.diagnostic().stage(),
        crate::DiagnosticStage::InputAdmission
    );
    assert_eq!(
        error.diagnostic().effect(),
        crate::DiagnosticEffectCertainty::NotApplicable
    );
}

#[test]
fn public_record_limit_is_the_exact_preflight_byte_boundary() {
    let fixture = fixture();
    let request_with = |byte_len| {
        ResidentOperationObservationRequest::new(
            OperationHandleLocator::new(HANDLE_REF.to_owned(), fixture.handle_sha256.clone()),
            vec![b'x'; byte_len],
        )
    };

    assert!(matches!(
        validate_request(&request_with(RESIDENT_RECORD_BYTE_LIMIT)),
        Err(ResidentObservationBoundaryError::SignedRequestCanonicalRecord)
    ));
    assert!(matches!(
        validate_request(&request_with(RESIDENT_RECORD_BYTE_LIMIT + 1)),
        Err(ResidentObservationBoundaryError::SignedRequestBounds)
    ));
}

#[test]
fn exact_canonical_records_cross_the_private_client_seam() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = fixture();
    let attachment = attachment(&fixture);
    let observed = api
        .observe_resident_operation(Some(&attachment), request(&fixture))
        .expect("exact resident projection is accepted");

    assert_eq!(observed.operation_handle_record_ref(), HANDLE_REF);
    assert_eq!(observed.operation_observation_record_ref(), OBSERVATION_REF);
    assert_eq!(
        observed.operation_handle_canonical_bytes(),
        fixture.handle_bytes
    );
    assert_eq!(
        observed.operation_handle_canonical_json().as_bytes(),
        observed.operation_handle_canonical_bytes()
    );
    assert_eq!(
        observed.operation_observation_canonical_bytes(),
        fixture.observation_bytes
    );
    assert_eq!(
        observed.operation_observation_canonical_json().as_bytes(),
        observed.operation_observation_canonical_bytes()
    );
    let projected = serde_json::to_value(&observed)
        .expect("admitted resident records have one readable JSON projection");
    assert_eq!(projected["operationHandleRecordRef"], HANDLE_REF);
    assert_eq!(
        projected["operationHandleCanonicalJson"],
        std::str::from_utf8(&fixture.handle_bytes).expect("canonical handle is UTF-8")
    );
    assert_eq!(projected["operationObservationRecordRef"], OBSERVATION_REF);
    assert_eq!(
        projected["operationObservationCanonicalJson"],
        std::str::from_utf8(&fixture.observation_bytes).expect("canonical observation is UTF-8")
    );
    assert!(projected.get("operationHandleCanonicalBytes").is_none());
    assert!(projected
        .get("operationObservationCanonicalBytes")
        .is_none());

    let debug = format!("{observed:?}");
    assert!(debug.contains(HANDLE_REF));
    assert!(debug.contains(OBSERVATION_REF));
    assert!(debug.contains(&format!(
        "operation_handle_canonical_byte_len: {}",
        fixture.handle_bytes.len()
    )));
    assert!(debug.contains(&format!(
        "operation_observation_canonical_byte_len: {}",
        fixture.observation_bytes.len()
    )));
    assert!(!debug.contains("payload"));
    assert!(!debug.contains(&"A".repeat(86)));
    assert!(!debug.contains(observed.operation_handle_canonical_json()));
    assert!(!debug.contains(observed.operation_observation_canonical_json()));
}

#[test]
fn application_owned_observation_fixture_crosses_the_sealed_sdk_projection() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = application_fixture();
    let attachment = ResidentAttachment::from_client(ApplicationFixtureClient {
        fixture: fixture.clone(),
    });
    let observed = api
        .observe_resident_operation(Some(&attachment), application_request(&fixture))
        .expect("real application Observe records project through the SDK");

    assert_eq!(observed.operation_handle_record_ref(), fixture.handle_ref);
    assert_eq!(
        observed.operation_handle_canonical_bytes(),
        fixture.handle_bytes
    );
    assert_eq!(
        observed.operation_observation_record_ref(),
        fixture.observation_ref
    );
    assert_eq!(
        observed.operation_observation_canonical_bytes(),
        fixture.observation_bytes
    );
}

#[test]
fn application_produced_invoke_start_crosses_the_sdk_without_claiming_a_live_attachment() {
    let (request_frame, response_frame) = application_invoke_fixture();
    let ResidentOperationFrameRequest::StartInvoke {
        signed_common_request,
    } = request_frame.clone()
    else {
        panic!("application Invoke fixture must contain one StartInvoke frame")
    };
    let ResidentOperationFrameResponse::Started { records } = response_frame.clone() else {
        panic!("application Invoke fixture must contain one Started frame")
    };
    let sent = Arc::new(Mutex::new(Vec::new()));
    let admission_calls = Arc::new(AtomicUsize::new(0));
    let attachment = ResidentAttachment::provision_operations(
        carrier_source(ScriptedAuthenticatedCarrier {
            sent: Arc::clone(&sent),
            responses: VecDeque::from([Ok(pale_core::canon::to_canonical_string(&response_frame)
                .expect("application response frame is canonical")
                .into_bytes()
                .into_boxed_slice())]),
        }),
        Duration::from_secs(1),
        Arc::new(ExactApplicationRecordsAdmission {
            calls: Arc::clone(&admission_calls),
            handle_ref: records.operation_handle_record_ref().to_owned(),
            handle_bytes: records.operation_handle_canonical().as_bytes().to_vec(),
            observation_ref: records.operation_observation_record_ref().to_owned(),
            observation_bytes: records
                .operation_observation_canonical()
                .as_bytes()
                .to_vec(),
        }),
    )
    .expect("positive timeout provisions the authenticated attachment");

    let started = PaleApi::new()
        .expect("embedded API dependencies are coherent")
        .start_resident_invoke(
            Some(&attachment),
            ResidentInvokeStartRequest::new(signed_common_request.into_bytes()),
        )
        .expect("the SDK projects the application-produced Started response");

    assert_eq!(
        started.operation_handle_record_ref(),
        records.operation_handle_record_ref()
    );
    assert_eq!(
        started.operation_handle_canonical_bytes(),
        records.operation_handle_canonical().as_bytes()
    );
    assert_eq!(
        started.operation_observation_record_ref(),
        records.operation_observation_record_ref()
    );
    assert_eq!(
        started.operation_observation_canonical_bytes(),
        records.operation_observation_canonical().as_bytes()
    );
    assert_eq!(admission_calls.load(Ordering::SeqCst), 1);
    let sent = sent
        .lock()
        .expect("authenticated send log remains available");
    assert_eq!(sent.len(), 1);
    let actual: ResidentOperationFrameRequest =
        serde_json::from_slice(&sent[0]).expect("SDK request frame decodes");
    assert_eq!(actual, request_frame);
}

#[test]
fn application_produced_invoke_advance_crosses_the_sdk_as_one_honest_pending_turn() {
    let (request_frame, response_frame) = application_invoke_advance_fixture();
    let ResidentOperationFrameRequest::AdvanceInvoke {
        locator,
        signed_common_request,
    } = request_frame.clone()
    else {
        panic!("application AdvanceInvoke fixture must contain one continuation frame")
    };
    assert_eq!(
        response_frame,
        ResidentOperationFrameResponse::Pending {
            locator: locator.clone(),
        }
    );
    let sent = Arc::new(Mutex::new(Vec::new()));
    let admission_calls = Arc::new(AtomicUsize::new(0));
    let attachment = ResidentAttachment::provision_operations(
        carrier_source(ScriptedAuthenticatedCarrier {
            sent: Arc::clone(&sent),
            responses: VecDeque::from([Ok(pale_core::canon::to_canonical_string(&response_frame)
                .expect("application response frame is canonical")
                .into_bytes()
                .into_boxed_slice())]),
        }),
        Duration::from_secs(1),
        Arc::new(ExactApplicationRecordsAdmission {
            calls: Arc::clone(&admission_calls),
            handle_ref: String::new(),
            handle_bytes: Vec::new(),
            observation_ref: String::new(),
            observation_bytes: Vec::new(),
        }),
    )
    .expect("positive timeout provisions the authenticated attachment");
    let requested_locator = OperationHandleLocator::new(
        locator.operation_handle_record_ref().to_owned(),
        locator.operation_handle_sha256().to_owned(),
    );

    let outcome = PaleApi::new()
        .expect("embedded API dependencies are coherent")
        .advance_resident_invoke(
            Some(&attachment),
            ResidentInvokeAdvanceRequest::new(
                requested_locator.clone(),
                signed_common_request.into_bytes(),
            ),
        )
        .expect("the SDK preserves the application-owned pending continuation");
    let ResidentInvokeAdvanceOutcome::Pending { locator: projected } = outcome else {
        panic!("the pending application continuation must not become an invented outcome")
    };
    assert_eq!(
        projected.operation_handle_record_ref(),
        requested_locator.operation_handle_record_ref()
    );
    assert_eq!(
        projected.operation_handle_sha256(),
        requested_locator.operation_handle_sha256()
    );
    assert_eq!(admission_calls.load(Ordering::SeqCst), 0);
    let sent = sent
        .lock()
        .expect("authenticated send log remains available");
    assert_eq!(sent.len(), 1);
    let actual: ResidentOperationFrameRequest =
        serde_json::from_slice(&sent[0]).expect("SDK request frame decodes");
    assert_eq!(
        actual, request_frame,
        "the SDK must not replay AdvanceInvoke"
    );
}

#[test]
fn application_produced_delegate_start_crosses_the_sdk_without_claiming_a_live_attachment() {
    let (request_frame, response_frame) = application_delegate_fixture();
    let ResidentOperationFrameRequest::StartDelegate {
        parent,
        signed_common_request,
    } = request_frame.clone()
    else {
        panic!("application Delegate fixture must contain one StartDelegate frame")
    };
    let ResidentOperationFrameResponse::Started { records } = response_frame.clone() else {
        panic!("application Delegate fixture must contain one Started frame")
    };
    let sent = Arc::new(Mutex::new(Vec::new()));
    let admission_calls = Arc::new(AtomicUsize::new(0));
    let attachment = ResidentAttachment::provision_operations(
        carrier_source(ScriptedAuthenticatedCarrier {
            sent: Arc::clone(&sent),
            responses: VecDeque::from([Ok(pale_core::canon::to_canonical_string(&response_frame)
                .expect("application response frame is canonical")
                .into_bytes()
                .into_boxed_slice())]),
        }),
        Duration::from_secs(1),
        Arc::new(ExactApplicationRecordsAdmission {
            calls: Arc::clone(&admission_calls),
            handle_ref: records.operation_handle_record_ref().to_owned(),
            handle_bytes: records.operation_handle_canonical().as_bytes().to_vec(),
            observation_ref: records.operation_observation_record_ref().to_owned(),
            observation_bytes: records
                .operation_observation_canonical()
                .as_bytes()
                .to_vec(),
        }),
    )
    .expect("positive timeout provisions the authenticated attachment");

    let started = PaleApi::new()
        .expect("embedded API dependencies are coherent")
        .start_resident_delegate(
            Some(&attachment),
            ResidentDelegateStartRequest::new(
                OperationHandleLocator::new(
                    parent.operation_handle_record_ref().to_owned(),
                    parent.operation_handle_sha256().to_owned(),
                ),
                signed_common_request.into_bytes(),
            ),
        )
        .expect("the SDK projects the application-produced Delegate Started response");

    assert_eq!(
        started.operation_handle_record_ref(),
        records.operation_handle_record_ref()
    );
    assert_eq!(
        started.operation_handle_canonical_bytes(),
        records.operation_handle_canonical().as_bytes()
    );
    assert_eq!(
        started.operation_observation_record_ref(),
        records.operation_observation_record_ref()
    );
    assert_eq!(
        started.operation_observation_canonical_bytes(),
        records.operation_observation_canonical().as_bytes()
    );
    assert_eq!(admission_calls.load(Ordering::SeqCst), 1);
    let sent = sent
        .lock()
        .expect("authenticated send log remains available");
    assert_eq!(sent.len(), 1);
    let actual: ResidentOperationFrameRequest =
        serde_json::from_slice(&sent[0]).expect("SDK request frame decodes");
    assert_eq!(actual, request_frame);
}

#[test]
fn application_produced_delegate_advance_crosses_the_sdk_once() {
    let (request_frame, response_frame) = application_delegate_advance_fixture();
    let ResidentOperationFrameRequest::AdvanceDelegate {
        locator,
        signed_common_request,
    } = request_frame.clone()
    else {
        panic!("application AdvanceDelegate fixture must contain one continuation frame")
    };
    let ResidentOperationFrameResponse::Pending {
        locator: response_locator,
    } = response_frame.clone()
    else {
        panic!("application AdvanceDelegate fixture must contain one Pending response")
    };
    assert_eq!(response_locator, locator);

    let sent = Arc::new(Mutex::new(Vec::new()));
    let admission_calls = Arc::new(AtomicUsize::new(0));
    let attachment = ResidentAttachment::provision_operations(
        carrier_source(ScriptedAuthenticatedCarrier {
            sent: Arc::clone(&sent),
            responses: VecDeque::from([Ok(pale_core::canon::to_canonical_string(&response_frame)
                .expect("application response frame is canonical")
                .into_bytes()
                .into_boxed_slice())]),
        }),
        Duration::from_secs(1),
        Arc::new(ExactApplicationRecordsAdmission {
            calls: Arc::clone(&admission_calls),
            handle_ref: String::new(),
            handle_bytes: Vec::new(),
            observation_ref: String::new(),
            observation_bytes: Vec::new(),
        }),
    )
    .expect("positive timeout provisions the authenticated attachment");

    let outcome = PaleApi::new()
        .expect("embedded API dependencies are coherent")
        .advance_resident_delegate(
            Some(&attachment),
            ResidentDelegateAdvanceRequest::new(
                OperationHandleLocator::new(
                    locator.operation_handle_record_ref().to_owned(),
                    locator.operation_handle_sha256().to_owned(),
                ),
                signed_common_request.into_bytes(),
            ),
        )
        .expect("the SDK projects the application-produced Pending response");
    let ResidentDelegateAdvanceOutcome::Pending {
        locator: outcome_locator,
    } = outcome
    else {
        panic!("AdvanceDelegate returned an unrelated SDK outcome")
    };
    assert_eq!(
        outcome_locator.operation_handle_record_ref(),
        locator.operation_handle_record_ref()
    );
    assert_eq!(
        outcome_locator.operation_handle_sha256(),
        locator.operation_handle_sha256()
    );
    assert_eq!(admission_calls.load(Ordering::SeqCst), 0);
    let sent = sent
        .lock()
        .expect("authenticated send log remains available");
    assert_eq!(sent.len(), 1, "the SDK must not replay AdvanceDelegate");
    let actual: ResidentOperationFrameRequest =
        serde_json::from_slice(&sent[0]).expect("SDK request frame decodes");
    assert_eq!(actual, request_frame);
}

#[test]
fn application_produced_cancel_crosses_the_sdk_without_claiming_a_live_attachment() {
    let (request_frame, response_frame) = application_cancel_fixture();
    let ResidentOperationFrameRequest::Cancel {
        locator,
        signed_common_request,
    } = request_frame.clone()
    else {
        panic!("application Cancel fixture must contain one Cancel frame")
    };
    let ResidentOperationFrameResponse::CancellationDisposition {
        locator: response_locator,
        disposition,
    } = response_frame.clone()
    else {
        panic!("application Cancel fixture must contain one cancellation disposition")
    };
    assert_eq!(response_locator, locator);
    assert_eq!(
        disposition,
        ResidentOperationFrameCancellationDisposition::LocallyCancelled
    );
    let sent = Arc::new(Mutex::new(Vec::new()));
    let admission_calls = Arc::new(AtomicUsize::new(0));
    let attachment = ResidentAttachment::provision_operations(
        carrier_source(ScriptedAuthenticatedCarrier {
            sent: Arc::clone(&sent),
            responses: VecDeque::from([Ok(pale_core::canon::to_canonical_string(&response_frame)
                .expect("application response frame is canonical")
                .into_bytes()
                .into_boxed_slice())]),
        }),
        Duration::from_secs(1),
        Arc::new(ExactFixtureResponseAdmission {
            calls: Arc::clone(&admission_calls),
            handle_ref: String::new(),
            observation_ref: String::new(),
        }),
    )
    .expect("positive timeout provisions the authenticated attachment");

    let cancelled = PaleApi::new()
        .expect("embedded API dependencies are coherent")
        .cancel_resident_operation(
            Some(&attachment),
            ResidentOperationCancellationRequest::new(
                OperationHandleLocator::new(
                    locator.operation_handle_record_ref().to_owned(),
                    locator.operation_handle_sha256().to_owned(),
                ),
                signed_common_request.into_bytes(),
            ),
        )
        .expect("the SDK projects the application-produced cancellation disposition");

    assert_eq!(
        cancelled.locator().operation_handle_record_ref(),
        locator.operation_handle_record_ref()
    );
    assert_eq!(
        cancelled.locator().operation_handle_sha256(),
        locator.operation_handle_sha256()
    );
    assert_eq!(
        cancelled.disposition(),
        ResidentOperationCancellationDisposition::LocallyCancelled
    );
    assert_eq!(admission_calls.load(Ordering::SeqCst), 0);
    let sent = sent
        .lock()
        .expect("authenticated send log remains available");
    assert_eq!(sent.len(), 1);
    let actual: ResidentOperationFrameRequest =
        serde_json::from_slice(&sent[0]).expect("SDK request frame decodes");
    assert_eq!(actual, request_frame);
}

#[test]
fn framed_client_serializes_one_call_and_admits_returned_signatures() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = application_fixture();
    let sent = Arc::new(Mutex::new(Vec::new()));
    let receive_calls = Arc::new(AtomicUsize::new(0));
    let admission_calls = Arc::new(AtomicUsize::new(0));
    let attachment = ResidentAttachment::from_framed_client(
        Box::new(ScriptedFramePort {
            sent: Arc::clone(&sent),
            response: Some(Ok(framed_response(&fixture))),
            receive_calls: Arc::clone(&receive_calls),
        }),
        Duration::from_secs(1),
        Arc::new(ExactFixtureResponseAdmission {
            calls: Arc::clone(&admission_calls),
            handle_ref: fixture.handle_ref.clone(),
            observation_ref: fixture.observation_ref.clone(),
        }),
    )
    .expect("positive bounded framed client is admitted");

    let observed = api
        .observe_resident_operation(Some(&attachment), application_request(&fixture))
        .expect("framed application response passes SDK admission");

    assert_eq!(observed.operation_handle_record_ref(), fixture.handle_ref);
    assert_eq!(receive_calls.load(Ordering::SeqCst), 1);
    assert_eq!(admission_calls.load(Ordering::SeqCst), 1);
    let sent = sent.lock().expect("scripted send log remains available");
    assert_eq!(sent.len(), 1);
    let frame: ResidentObserveFrameRequest =
        serde_json::from_slice(&sent[0]).expect("sent request has the shared frame shape");
    assert_eq!(frame.operation_handle_record_ref(), fixture.handle_ref);
    assert_eq!(frame.operation_handle_sha256(), fixture.handle_sha256);
    assert_eq!(
        frame.signed_common_request().as_bytes(),
        fixture.request_bytes
    );
}

#[test]
fn public_provisioning_reuses_one_healthy_carrier_without_retry() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = application_fixture();
    let sent = Arc::new(Mutex::new(Vec::new()));
    let admission_calls = Arc::new(AtomicUsize::new(0));
    let pending = ResidentOperationFrameResponse::Pending {
        locator: ResidentOperationFrameLocator::new(
            fixture.handle_ref.clone(),
            fixture.handle_sha256.clone(),
        ),
    };
    let attachment = ResidentAttachment::provision_operations(
        carrier_source(ScriptedAuthenticatedCarrier {
            sent: Arc::clone(&sent),
            responses: VecDeque::from([
                Ok(framed_response(&fixture)),
                Ok(pale_core::canon::to_canonical_string(&pending)
                    .expect("pending frame is canonical")
                    .into_bytes()
                    .into_boxed_slice()),
                Err(AuthenticatedResidentCarrierFailure::OutcomeUnknown),
            ]),
        }),
        Duration::from_secs(1),
        Arc::new(ExactFixtureResponseAdmission {
            calls: Arc::clone(&admission_calls),
            handle_ref: fixture.handle_ref.clone(),
            observation_ref: fixture.observation_ref.clone(),
        }),
    )
    .expect("positive timeout provisions the authenticated attachment");

    api.observe_resident_operation(Some(&attachment), application_request(&fixture))
        .expect("Observe uses the attachment's healthy carrier");
    let operation_locator =
        OperationHandleLocator::new(fixture.handle_ref.clone(), fixture.handle_sha256.clone());
    let advance_request = operation_request_with_target(&fixture.request_bytes, &operation_locator);
    let outcome = api
        .advance_resident_invoke(
            Some(&attachment),
            ResidentInvokeAdvanceRequest::new(operation_locator.clone(), advance_request.clone()),
        )
        .expect("Invoke reuses the attachment's healthy carrier");
    assert!(matches!(
        outcome,
        ResidentInvokeAdvanceOutcome::Pending { .. }
    ));
    assert_eq!(admission_calls.load(Ordering::SeqCst), 1);

    for _ in 0..2 {
        let _ = api
            .advance_resident_invoke(
                Some(&attachment),
                ResidentInvokeAdvanceRequest::new(
                    operation_locator.clone(),
                    advance_request.clone(),
                ),
            )
            .expect_err("unknown carrier outcome closes the attachment");
    }
    assert_eq!(
        sent.lock()
            .expect("authenticated send log remains available")
            .len(),
        3,
        "the second failure does not resend after an unknown outcome"
    );
}

#[test]
fn public_provisioning_rejects_a_zero_timeout() {
    let error = ResidentAttachment::provision_observation(
        carrier_source(ScriptedAuthenticatedCarrier {
            sent: Arc::new(Mutex::new(Vec::new())),
            responses: VecDeque::new(),
        }),
        Duration::ZERO,
        Arc::new(ExactFixtureResponseAdmission {
            calls: Arc::new(AtomicUsize::new(0)),
            handle_ref: HANDLE_REF.to_owned(),
            observation_ref: OBSERVATION_REF.to_owned(),
        }),
    )
    .expect_err("an unbounded zero-time exchange cannot be provisioned");
    assert_eq!(error, ResidentAttachmentBuildError::ZeroTimeout);
}

#[test]
fn observation_only_provisioning_rejects_operations_without_carrier_io() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = operation_fixture("read-only-consult", "consult", None);
    let sent = Arc::new(Mutex::new(Vec::new()));
    let attachment = ResidentAttachment::provision_observation(
        carrier_source(ScriptedAuthenticatedCarrier {
            sent: Arc::clone(&sent),
            responses: VecDeque::from([Err(AuthenticatedResidentCarrierFailure::OutcomeUnknown)]),
        }),
        Duration::from_secs(1),
        Arc::new(ExactFixtureResponseAdmission {
            calls: Arc::new(AtomicUsize::new(0)),
            handle_ref: HANDLE_REF.to_owned(),
            observation_ref: OBSERVATION_REF.to_owned(),
        }),
    )
    .expect("positive timeout provisions a read-only attachment");

    let error = api
        .start_resident_consult(
            Some(&attachment),
            ResidentConsultStartRequest::new(fixture.request_bytes),
        )
        .expect_err("an Observe-only attachment cannot send Consult");

    assert_eq!(error.diagnostic().code(), "resident_operation_unavailable");
    assert!(
        sent.lock()
            .expect("authenticated send log remains available")
            .is_empty(),
        "the capability ceiling is enforced before carrier I/O"
    );
}

#[test]
fn public_provisioning_preserves_a_proven_not_sent_operation() {
    for failure in [
        AuthenticatedResidentCarrierFailure::NotSent,
        AuthenticatedResidentCarrierFailure::ConnectionUnavailable,
    ] {
        let api = PaleApi::new().expect("embedded API dependencies are coherent");
        let fixture = operation_fixture("invoke-not-sent", "invoke", None);
        let attachment = ResidentAttachment::provision_operations(
            carrier_source(ScriptedAuthenticatedCarrier {
                sent: Arc::new(Mutex::new(Vec::new())),
                responses: VecDeque::from([Err(failure)]),
            }),
            Duration::from_secs(1),
            Arc::new(ExactFixtureResponseAdmission {
                calls: Arc::new(AtomicUsize::new(0)),
                handle_ref: HANDLE_REF.to_owned(),
                observation_ref: OBSERVATION_REF.to_owned(),
            }),
        )
        .expect("positive timeout provisions an attachment");
        assert!(attachment.supports_operation_requests());

        let error = api
            .start_resident_invoke(
                Some(&attachment),
                ResidentInvokeStartRequest::new(fixture.request_bytes),
            )
            .expect_err("the carrier proves that no operation request was sent");
        assert_eq!(error.diagnostic().code(), "resident_operation_not_sent");
        assert_eq!(
            error.diagnostic().effect(),
            DiagnosticEffectCertainty::NotStarted
        );
        assert_eq!(
            error.diagnostic().retry(),
            DiagnosticRetry::AfterStateChange
        );
    }
}

#[test]
fn later_request_opens_one_fresh_carrier_without_replaying_the_failed_request() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let first_fixture = operation_fixture("invoke-after-unknown", "invoke", None);
    let second_fixture = operation_fixture("consult-after-unknown", "consult", None);
    let sent = Arc::new(Mutex::new(Vec::new()));
    let opens = Arc::new(AtomicUsize::new(0));
    let admission_calls = Arc::new(AtomicUsize::new(0));
    let second_response =
        pale_core::canon::to_canonical_string(&ResidentOperationFrameResponse::Started {
            records: operation_records(&second_fixture),
        })
        .expect("second response is canonical")
        .into_bytes()
        .into_boxed_slice();
    let attachment = ResidentAttachment::provision_operations(
        Box::new(ScriptedCarrierSource {
            opens: Arc::clone(&opens),
            carriers: VecDeque::from([
                Ok(Box::new(ScriptedAuthenticatedCarrier {
                    sent: Arc::clone(&sent),
                    responses: VecDeque::from([Err(
                        AuthenticatedResidentCarrierFailure::OutcomeUnknown,
                    )]),
                }) as Box<dyn AuthenticatedResidentCarrier>),
                Ok(Box::new(ScriptedAuthenticatedCarrier {
                    sent: Arc::clone(&sent),
                    responses: VecDeque::from([Ok(second_response)]),
                }) as Box<dyn AuthenticatedResidentCarrier>),
            ]),
        }),
        Duration::from_secs(1),
        Arc::new(ExactFixtureResponseAdmission {
            calls: Arc::clone(&admission_calls),
            handle_ref: HANDLE_REF.to_owned(),
            observation_ref: OBSERVATION_REF.to_owned(),
        }),
    )
    .expect("positive timeout provisions an attachment");

    let first = api
        .start_resident_invoke(
            Some(&attachment),
            ResidentInvokeStartRequest::new(first_fixture.request_bytes),
        )
        .expect_err("the first operation has an unknown transmission outcome");
    assert_eq!(first.diagnostic().code(), "resident_operation_failed");
    assert_eq!(
        first.diagnostic().effect(),
        DiagnosticEffectCertainty::OutcomeUnknown
    );
    assert!(first.diagnostic().help().iter().any(|help| {
        help.contains("resubmit the exact signed Start request bytes")
            && help.contains("do not mint a new request identity")
    }));

    let second = api
        .start_resident_consult(
            Some(&attachment),
            ResidentConsultStartRequest::new(second_fixture.request_bytes),
        )
        .expect("the separate request opens one fresh carrier");
    assert_eq!(second.operation_handle_record_ref(), HANDLE_REF);
    assert_eq!(opens.load(Ordering::SeqCst), 2);
    assert_eq!(admission_calls.load(Ordering::SeqCst), 1);
    let sent = sent.lock().expect("send log remains available");
    assert_eq!(sent.len(), 2, "each request is sent exactly once");
    assert!(matches!(
        serde_json::from_slice::<ResidentOperationFrameRequest>(&sent[0])
            .expect("first sent frame is valid"),
        ResidentOperationFrameRequest::StartInvoke { .. }
    ));
    assert!(matches!(
        serde_json::from_slice::<ResidentOperationFrameRequest>(&sent[1])
            .expect("second sent frame is valid"),
        ResidentOperationFrameRequest::StartConsult { .. }
    ));
}

fn assert_failure_does_not_reuse_or_replay_single_carrier(
    response: Result<Box<[u8]>, AuthenticatedResidentCarrierFailure>,
    response_admission: Arc<dyn ResidentResponseAdmission>,
) -> (String, String) {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = application_fixture();
    let sent = Arc::new(Mutex::new(Vec::new()));
    let attachment = ResidentAttachment::provision_observation(
        carrier_source(ScriptedAuthenticatedCarrier {
            sent: Arc::clone(&sent),
            responses: VecDeque::from([response]),
        }),
        Duration::from_secs(1),
        response_admission,
    )
    .expect("positive timeout provisions an attachment");

    let first = api
        .observe_resident_operation(Some(&attachment), application_request(&fixture))
        .expect_err("the first invalid exchange fails closed");
    let second = api
        .observe_resident_operation(Some(&attachment), application_request(&fixture))
        .expect_err("the single-carrier source cannot open a replacement");
    assert_eq!(
        sent.lock().expect("send log remains available").len(),
        1,
        "the failed carrier is never reused and the request is never replayed"
    );
    (
        first.diagnostic().code().to_owned(),
        second.diagnostic().code().to_owned(),
    )
}

#[test]
fn attachment_never_resends_after_carrier_or_response_failure() {
    for failure in [
        AuthenticatedResidentCarrierFailure::NotSent,
        AuthenticatedResidentCarrierFailure::OutcomeUnknown,
        AuthenticatedResidentCarrierFailure::ConnectionUnavailable,
    ] {
        let fixture = application_fixture();
        let (first_code, second_code) = assert_failure_does_not_reuse_or_replay_single_carrier(
            Err(failure),
            Arc::new(ExactFixtureResponseAdmission {
                calls: Arc::new(AtomicUsize::new(0)),
                handle_ref: fixture.handle_ref,
                observation_ref: fixture.observation_ref,
            }),
        );
        assert_eq!(
            first_code,
            if matches!(
                failure,
                AuthenticatedResidentCarrierFailure::NotSent
                    | AuthenticatedResidentCarrierFailure::ConnectionUnavailable
            ) {
                "resident_observation_not_sent"
            } else {
                "resident_observation_failed"
            }
        );
        assert_eq!(second_code, "resident_observation_not_sent");
    }

    for response in [
        b"{".to_vec().into_boxed_slice(),
        vec![b'x'; RESIDENT_RECORD_BYTE_LIMIT.saturating_mul(4) + 1].into_boxed_slice(),
    ] {
        let fixture = application_fixture();
        let (first_code, second_code) = assert_failure_does_not_reuse_or_replay_single_carrier(
            Ok(response),
            Arc::new(ExactFixtureResponseAdmission {
                calls: Arc::new(AtomicUsize::new(0)),
                handle_ref: fixture.handle_ref,
                observation_ref: fixture.observation_ref,
            }),
        );
        assert_eq!(first_code, "resident_observation_failed");
        assert_eq!(second_code, "resident_observation_not_sent");
    }

    let fixture = application_fixture();
    let (first_code, second_code) = assert_failure_does_not_reuse_or_replay_single_carrier(
        Ok(framed_response(&fixture)),
        Arc::new(ExactFixtureResponseAdmission {
            calls: Arc::new(AtomicUsize::new(0)),
            handle_ref: "operations/untrusted-handle".to_owned(),
            observation_ref: fixture.observation_ref,
        }),
    );
    assert_eq!(first_code, "resident_observation_failed");
    assert_eq!(second_code, "resident_observation_not_sent");
}

#[test]
fn misbound_response_cannot_advance_response_admission() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = application_fixture();
    let admission_calls = Arc::new(AtomicUsize::new(0));
    let misbound = canonical(
        &serde_json::to_value(ResidentObserveFrameResponse::Observed {
            operation_handle_record_ref: "operations/another-handle".to_owned(),
            operation_handle_canonical: String::from_utf8(fixture.handle_bytes.clone())
                .expect("fixture handle is UTF-8 JSON"),
            operation_observation_record_ref: fixture.observation_ref.clone(),
            operation_observation_canonical: String::from_utf8(fixture.observation_bytes.clone())
                .expect("fixture observation is UTF-8 JSON"),
        })
        .expect("misbound response serializes"),
    )
    .into_boxed_slice();
    let sent = Arc::new(Mutex::new(Vec::new()));
    let attachment = ResidentAttachment::provision_observation(
        carrier_source(ScriptedAuthenticatedCarrier {
            sent: Arc::clone(&sent),
            responses: VecDeque::from([Ok(misbound)]),
        }),
        Duration::from_secs(1),
        Arc::new(ExactFixtureResponseAdmission {
            calls: Arc::clone(&admission_calls),
            handle_ref: fixture.handle_ref.clone(),
            observation_ref: fixture.observation_ref.clone(),
        }),
    )
    .expect("positive timeout provisions an attachment");

    api.observe_resident_operation(Some(&attachment), application_request(&fixture))
        .expect_err("another operation cannot advance response authority");
    api.observe_resident_operation(Some(&attachment), application_request(&fixture))
        .expect_err("a misbound response discards the failed carrier");
    assert_eq!(admission_calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        sent.lock().expect("send log remains available").len(),
        1,
        "the invalid response closes the attachment atomically"
    );
}

#[test]
fn wrong_operation_class_cannot_advance_response_admission() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let requested = operation_fixture("invoke-requested", "invoke", None);
    let returned = operation_fixture("consult-returned", "consult", None);
    let admission_calls = Arc::new(AtomicUsize::new(0));
    let sent = Arc::new(Mutex::new(Vec::new()));
    let response = ResidentOperationFrameResponse::Started {
        records: operation_records(&returned),
    };
    let attachment = ResidentAttachment::provision_operations(
        carrier_source(ScriptedAuthenticatedCarrier {
            sent: Arc::clone(&sent),
            responses: VecDeque::from([Ok(pale_core::canon::to_canonical_string(&response)
                .expect("operation response is canonical")
                .into_bytes()
                .into_boxed_slice())]),
        }),
        Duration::from_secs(1),
        Arc::new(ExactFixtureResponseAdmission {
            calls: Arc::clone(&admission_calls),
            handle_ref: HANDLE_REF.to_owned(),
            observation_ref: OBSERVATION_REF.to_owned(),
        }),
    )
    .expect("positive timeout provisions an attachment");

    api.start_resident_invoke(
        Some(&attachment),
        ResidentInvokeStartRequest::new(requested.request_bytes.clone()),
    )
    .expect_err("a Consult record cannot satisfy an Invoke response");
    api.start_resident_invoke(
        Some(&attachment),
        ResidentInvokeStartRequest::new(requested.request_bytes),
    )
    .expect_err("an unrelated response discards the failed carrier");
    assert_eq!(admission_calls.load(Ordering::SeqCst), 0);
    assert_eq!(sent.lock().expect("send log remains available").len(), 1);
}

#[test]
fn framed_client_holds_call_order_through_response_admission() {
    let fixture = application_fixture();
    let sent = Arc::new(Mutex::new(Vec::new()));
    let admission_calls = Arc::new(AtomicUsize::new(0));
    let entered = Arc::new((Mutex::new(false), Condvar::new()));
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let (send_events, sent_frames) = mpsc::channel();
    let attachment = Arc::new(
        ResidentAttachment::from_framed_client(
            Box::new(SequencedFramePort {
                sent: Arc::clone(&sent),
                responses: VecDeque::from([
                    Ok(framed_response(&fixture)),
                    Ok(framed_response(&fixture)),
                ]),
                send_events,
            }),
            Duration::from_secs(1),
            Arc::new(BlockingFirstResponseAdmission {
                inner: ExactFixtureResponseAdmission {
                    calls: Arc::clone(&admission_calls),
                    handle_ref: fixture.handle_ref.clone(),
                    observation_ref: fixture.observation_ref.clone(),
                },
                entered: Arc::clone(&entered),
                release: Arc::clone(&release),
            }),
        )
        .expect("positive bounded framed client is admitted"),
    );

    let first_attachment = Arc::clone(&attachment);
    let first_fixture = fixture.clone();
    let first = thread::spawn(move || {
        PaleApi::new()
            .expect("embedded API dependencies are coherent")
            .observe_resident_operation(
                Some(first_attachment.as_ref()),
                application_request(&first_fixture),
            )
    });
    sent_frames
        .recv_timeout(Duration::from_secs(1))
        .expect("the first call sends its frame");
    let (entered_lock, entered_changed) = &*entered;
    let mut has_entered = entered_lock.lock().expect("entry signal remains available");
    while !*has_entered {
        has_entered = entered_changed
            .wait(has_entered)
            .expect("entry wait remains available");
    }
    drop(has_entered);

    let second_attachment = Arc::clone(&attachment);
    let second_fixture = fixture.clone();
    let (second_started, second_started_rx) = mpsc::channel();
    let second = thread::spawn(move || {
        second_started
            .send(())
            .expect("test retains the start receiver");
        PaleApi::new()
            .expect("embedded API dependencies are coherent")
            .observe_resident_operation(
                Some(second_attachment.as_ref()),
                application_request(&second_fixture),
            )
    });
    second_started_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("the second call reaches the client");
    assert!(
        sent_frames.recv_timeout(Duration::from_millis(50)).is_err(),
        "a later call cannot send before the first response is admitted"
    );

    let (release_lock, release_changed) = &*release;
    *release_lock
        .lock()
        .expect("release signal remains available") = true;
    release_changed.notify_all();

    first
        .join()
        .expect("first Observe thread does not panic")
        .expect("first Observe response is admitted");
    second
        .join()
        .expect("second Observe thread does not panic")
        .expect("second Observe response is admitted");
    assert_eq!(admission_calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        sent.lock()
            .expect("sequenced send log remains available")
            .len(),
        2
    );
}

#[test]
fn queued_call_expires_without_sending_and_the_shared_deadline_closes_the_first_call() {
    let fixture = application_fixture();
    let sent = Arc::new(Mutex::new(Vec::new()));
    let admission_calls = Arc::new(AtomicUsize::new(0));
    let entered = Arc::new((Mutex::new(false), Condvar::new()));
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let (send_events, sent_frames) = mpsc::channel();
    let attachment = Arc::new(
        ResidentAttachment::from_framed_client(
            Box::new(SequencedFramePort {
                sent: Arc::clone(&sent),
                responses: VecDeque::from([Ok(framed_response(&fixture))]),
                send_events,
            }),
            Duration::from_millis(100),
            Arc::new(BlockingFirstResponseAdmission {
                inner: ExactFixtureResponseAdmission {
                    calls: Arc::clone(&admission_calls),
                    handle_ref: fixture.handle_ref.clone(),
                    observation_ref: fixture.observation_ref.clone(),
                },
                entered: Arc::clone(&entered),
                release: Arc::clone(&release),
            }),
        )
        .expect("positive bounded framed client is admitted"),
    );

    let first_attachment = Arc::clone(&attachment);
    let first_fixture = fixture.clone();
    let first = thread::spawn(move || {
        PaleApi::new()
            .expect("embedded API dependencies are coherent")
            .observe_resident_operation(
                Some(first_attachment.as_ref()),
                application_request(&first_fixture),
            )
    });
    sent_frames
        .recv_timeout(Duration::from_secs(1))
        .expect("the first call sends its frame");
    let (entered_lock, entered_changed) = &*entered;
    let mut has_entered = entered_lock.lock().expect("entry signal remains available");
    while !*has_entered {
        has_entered = entered_changed
            .wait(has_entered)
            .expect("entry wait remains available");
    }
    drop(has_entered);

    let second_error = PaleApi::new()
        .expect("embedded API dependencies are coherent")
        .observe_resident_operation(Some(attachment.as_ref()), application_request(&fixture))
        .expect_err("the queued call expires before it can send");
    assert_eq!(
        second_error.diagnostic().code(),
        "resident_observation_not_sent"
    );
    assert_eq!(
        sent.lock().expect("send log remains available").len(),
        1,
        "the queued call never reaches the carrier"
    );

    let (release_lock, release_changed) = &*release;
    *release_lock
        .lock()
        .expect("release signal remains available") = true;
    release_changed.notify_all();
    let first_error = first
        .join()
        .expect("first Observe thread does not panic")
        .expect_err("response admission cannot outlive the shared deadline");
    assert_eq!(
        first_error.diagnostic().code(),
        "resident_observation_failed"
    );
    assert_eq!(admission_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn fatal_framing_failure_prevents_connection_reuse() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = application_fixture();
    let sent = Arc::new(Mutex::new(Vec::new()));
    let receive_calls = Arc::new(AtomicUsize::new(0));
    let attachment = ResidentAttachment::from_framed_client(
        Box::new(ScriptedFramePort {
            sent: Arc::clone(&sent),
            response: Some(Err(LocalIpcCarrierError::TruncatedFrame {
                section: LocalIpcFrameSection::Payload,
                expected_bytes: 10,
                received_bytes: 3,
            })),
            receive_calls: Arc::clone(&receive_calls),
        }),
        Duration::from_secs(1),
        Arc::new(ExactFixtureResponseAdmission {
            calls: Arc::new(AtomicUsize::new(0)),
            handle_ref: fixture.handle_ref.clone(),
            observation_ref: fixture.observation_ref.clone(),
        }),
    )
    .expect("positive bounded framed client is admitted");

    let first = api
        .observe_resident_operation(Some(&attachment), application_request(&fixture))
        .expect_err("the truncated response cannot yield an observation");
    assert_eq!(first.diagnostic().code(), "resident_observation_failed");

    let second = api
        .observe_resident_operation(Some(&attachment), application_request(&fixture))
        .expect_err("the exhausted single-carrier source cannot send another request");
    assert_eq!(second.diagnostic().code(), "resident_observation_not_sent");
    assert_eq!(
        receive_calls.load(Ordering::SeqCst),
        1,
        "the attachment receives from the failed carrier only once"
    );
    assert_eq!(
        sent.lock()
            .expect("scripted send log remains available")
            .len(),
        1,
        "the second call fails before another frame is sent"
    );
}

#[test]
fn authenticated_attachment_rejects_a_changed_request_signature() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = application_fixture();
    let mut request = parsed(&fixture.request_bytes);
    request["caller_signature"] = serde_json::json!(format!("{}Q", "A".repeat(85)));
    let attachment = ResidentAttachment::from_client(SignatureAdmittingApplicationClient {
        fixture: fixture.clone(),
    });
    let error = api
        .observe_resident_operation(
            Some(&attachment),
            ResidentOperationObservationRequest::new(
                OperationHandleLocator::new(
                    fixture.handle_ref.clone(),
                    fixture.handle_sha256.clone(),
                ),
                canonical(&request),
            ),
        )
        .expect_err("the authenticated attachment owns caller-signature admission");

    assert_eq!(error.diagnostic().code(), "resident_observation_not_sent");
    assert_eq!(error.diagnostic().stage(), DiagnosticStage::HostObservation);
    assert_eq!(
        error.diagnostic().disposition(),
        DiagnosticDisposition::OperationalFailure
    );
    assert_eq!(
        error.diagnostic().retry(),
        DiagnosticRetry::AfterStateChange
    );
    assert_eq!(
        error.diagnostic().effect(),
        DiagnosticEffectCertainty::NotApplicable
    );
}

#[test]
fn authenticated_attachment_rejects_a_changed_response_signature_after_resealing() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let mut fixture = application_fixture();
    let mut observation = parsed(&fixture.observation_bytes);
    let original_digest = observation["canonical_operation_observation_sha256"].clone();
    observation["observed_at"] = serde_json::json!(62);
    let mut observation = reseal_observation(observation);
    assert_ne!(
        observation["canonical_operation_observation_sha256"], original_digest,
        "changed content carries a recomputed digest"
    );
    observation["signature"] = serde_json::json!(format!("{}Q", "A".repeat(85)));
    fixture.observation_bytes = canonical(&observation);
    let attachment = ResidentAttachment::from_client(SignatureAdmittingApplicationClient {
        fixture: fixture.clone(),
    });
    let error = api
        .observe_resident_operation(Some(&attachment), application_request(&fixture))
        .expect_err("content resealing cannot replace response-signature admission");

    assert_eq!(error.diagnostic().code(), "resident_observation_failed");
}

#[test]
fn malformed_common_request_is_rejected_before_client_dispatch() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = fixture();
    let malformed_request = ResidentOperationObservationRequest::new(
        OperationHandleLocator::new(HANDLE_REF.to_owned(), fixture.handle_sha256.clone()),
        br#"{"envelope_schema_id":"pale/common-request-envelope.v1"}"#.to_vec(),
    );
    let error = api
        .observe_resident_operation(Some(&attachment(&fixture)), malformed_request)
        .expect_err("an incomplete envelope must not reach the client");
    assert_eq!(
        error.diagnostic().code(),
        "resident_observation_request_invalid"
    );
}

#[test]
fn self_inconsistent_common_request_digest_is_rejected_before_dispatch() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = fixture();
    let mut request_value: serde_json::Value =
        pale_core::canon::parse_strict(&fixture.request_bytes).expect("request parses");
    request_value["issued_at"] = serde_json::json!(11);
    let changed_request = ResidentOperationObservationRequest::new(
        OperationHandleLocator::new(HANDLE_REF.to_owned(), fixture.handle_sha256.clone()),
        canonical(&request_value),
    );
    let error = api
        .observe_resident_operation(Some(&attachment(&fixture)), changed_request)
        .expect_err("changed preimage must invalidate the common-request digest");
    assert_eq!(
        error.diagnostic().code(),
        "resident_observation_request_invalid"
    );
}

#[test]
fn request_with_another_schema_digest_is_rejected_after_resealing() {
    let fixture = fixture();
    let mut request_value = parsed(&fixture.request_bytes);
    request_value["envelope_schema_sha256"] = serde_json::json!(sha('0'));
    let request_bytes = reseal_common_request(request_value);
    assert_request_rejected(&fixture, fixture.handle_sha256.clone(), request_bytes);
}

#[test]
fn request_with_another_capability_is_rejected_after_resealing() {
    let fixture = fixture();
    let mut request_value = parsed(&fixture.request_bytes);
    request_value["capability_id"] = serde_json::json!("operation.cancel");
    let request_bytes = reseal_common_request(request_value);
    assert_request_rejected(&fixture, fixture.handle_sha256.clone(), request_bytes);
}

#[test]
fn request_with_another_body_schema_is_rejected_after_resealing() {
    let fixture = fixture();
    let mut request_value = parsed(&fixture.request_bytes);
    request_value["body"]["schema_id"] = serde_json::json!("pale/common-request-envelope.v1");
    request_value["body"]["schema_sha256"] =
        serde_json::json!(schema_sha256("common-request-envelope.v1"));
    let request_bytes = reseal_common_request(request_value);
    assert_request_rejected(&fixture, fixture.handle_sha256.clone(), request_bytes);
}

#[test]
fn request_with_another_body_schema_digest_is_rejected_after_resealing() {
    let fixture = fixture();
    let mut request_value = parsed(&fixture.request_bytes);
    request_value["body"]["schema_sha256"] = serde_json::json!(sha('0'));
    let request_bytes = reseal_common_request(request_value);
    assert_request_rejected(&fixture, fixture.handle_sha256.clone(), request_bytes);
}

#[test]
fn request_with_another_payload_digest_is_rejected_after_resealing() {
    let fixture = fixture();
    let mut request_value = parsed(&fixture.request_bytes);
    request_value["body"]["payload_sha256"] = serde_json::json!(sha('0'));
    let request_bytes = reseal_common_request(request_value);
    assert_request_rejected(&fixture, fixture.handle_sha256.clone(), request_bytes);
}

#[test]
fn request_body_rejects_a_self_inconsistent_handle() {
    let fixture = fixture();
    let mut handle = parsed(&fixture.handle_bytes);
    handle["created_at"] = serde_json::json!(11);
    let mut request_value = parsed(&fixture.request_bytes);
    set_body_payload(&mut request_value, handle);
    let request_bytes = reseal_common_request(request_value);
    assert_request_rejected(&fixture, fixture.handle_sha256.clone(), request_bytes);
}

#[test]
fn request_body_cannot_substitute_another_valid_handle() {
    let fixture = fixture();
    let (_, other_handle) = sealed_handle("operation-2");
    let mut request_value = parsed(&fixture.request_bytes);
    set_body_payload(&mut request_value, other_handle);
    let request_bytes = reseal_common_request(request_value);
    assert_request_rejected(&fixture, fixture.handle_sha256.clone(), request_bytes);
}

#[test]
fn embedded_handle_with_another_schema_digest_is_rejected_after_resealing() {
    let fixture = fixture();
    let mut handle = parsed(&fixture.handle_bytes);
    handle["schema_sha256"] = serde_json::json!(sha('0'));
    let (handle_sha256, handle) = reseal_handle(handle);
    let mut request_value = parsed(&fixture.request_bytes);
    set_body_payload(&mut request_value, handle);
    let request_bytes = reseal_common_request(request_value);
    assert_request_rejected(&fixture, handle_sha256, request_bytes);
}

#[test]
fn returned_handle_with_another_schema_digest_fails_record_admission() {
    let fixture = fixture();
    let mut handle = parsed(&fixture.handle_bytes);
    handle["schema_sha256"] = serde_json::json!(sha('0'));
    let (_, handle) = reseal_handle(handle);
    assert_eq!(
        parse_canonical_record(&canonical(&handle), "operation-handle.v1", "schema_sha256"),
        Err(CanonicalRecordError::Contract)
    );
}

#[test]
fn substituted_locator_is_rejected_before_public_projection() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = fixture();
    let substituted =
        sealed_observation("operation-1", "operations/handle-2", &fixture.handle_sha256);
    let attachment = ResidentAttachment::from_client(FakeClient {
        expected_request_bytes: fixture.request_bytes.clone(),
        response_handle_ref: "operations/handle-2".to_owned(),
        handle_bytes: fixture.handle_bytes.clone(),
        observation_bytes: canonical(&substituted),
    });
    let error = api
        .observe_resident_operation(Some(&attachment), request(&fixture))
        .expect_err("the response cannot substitute a locator");
    assert_eq!(
        error.diagnostic().code(),
        "resident_observation_response_invalid"
    );
}

#[test]
fn self_inconsistent_handle_digest_is_rejected() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = fixture();
    let mut handle: serde_json::Value =
        pale_core::canon::parse_strict(&fixture.handle_bytes).expect("handle parses");
    handle["created_at"] = serde_json::json!(11);
    let attachment = ResidentAttachment::from_client(FakeClient {
        expected_request_bytes: fixture.request_bytes.clone(),
        response_handle_ref: HANDLE_REF.to_owned(),
        handle_bytes: canonical(&handle),
        observation_bytes: fixture.observation_bytes.clone(),
    });
    let error = api
        .observe_resident_operation(Some(&attachment), request(&fixture))
        .expect_err("changed preimage must invalidate the handle digest");
    assert_eq!(
        error.diagnostic().code(),
        "resident_observation_response_invalid"
    );
}

#[test]
fn self_inconsistent_observation_digest_is_rejected() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = fixture();
    let mut observation: serde_json::Value =
        pale_core::canon::parse_strict(&fixture.observation_bytes).expect("observation parses");
    observation["observed_at"] = serde_json::json!(11);
    let attachment = ResidentAttachment::from_client(FakeClient {
        expected_request_bytes: fixture.request_bytes.clone(),
        response_handle_ref: HANDLE_REF.to_owned(),
        handle_bytes: fixture.handle_bytes.clone(),
        observation_bytes: canonical(&observation),
    });
    let error = api
        .observe_resident_operation(Some(&attachment), request(&fixture))
        .expect_err("changed preimage must invalidate the observation digest");
    assert_eq!(
        error.diagnostic().code(),
        "resident_observation_response_invalid"
    );
}

#[test]
fn observation_with_another_schema_digest_is_rejected_after_resealing() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = fixture();
    let mut observation = parsed(&fixture.observation_bytes);
    observation["schema_sha256"] = serde_json::json!(sha('0'));
    let observation = reseal_observation(observation);
    let attachment = ResidentAttachment::from_client(FakeClient {
        expected_request_bytes: fixture.request_bytes.clone(),
        response_handle_ref: HANDLE_REF.to_owned(),
        handle_bytes: fixture.handle_bytes.clone(),
        observation_bytes: canonical(&observation),
    });
    let error = api
        .observe_resident_operation(Some(&attachment), request(&fixture))
        .expect_err("the carried observation schema digest must be exact");
    assert_eq!(
        error.diagnostic().code(),
        "resident_observation_response_invalid"
    );
}

#[test]
fn unlike_operation_handle_ids_are_rejected() {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let fixture = fixture();
    let observation = sealed_observation("operation-2", HANDLE_REF, &fixture.handle_sha256);
    let attachment = ResidentAttachment::from_client(FakeClient {
        expected_request_bytes: fixture.request_bytes.clone(),
        response_handle_ref: HANDLE_REF.to_owned(),
        handle_bytes: fixture.handle_bytes.clone(),
        observation_bytes: canonical(&observation),
    });
    let error = api
        .observe_resident_operation(Some(&attachment), request(&fixture))
        .expect_err("returned records must name the same operation handle");
    assert_eq!(
        error.diagnostic().code(),
        "resident_observation_response_invalid"
    );
}

#[test]
fn hook_only_attachment_cannot_observe_or_start_an_operation() {
    struct NeverOpen;
    impl crate::AuthenticatedResidentCarrierSource for NeverOpen {
        fn open(
            &mut self,
            _: std::time::Instant,
        ) -> Result<
            Box<dyn crate::AuthenticatedResidentCarrier>,
            crate::AuthenticatedResidentCarrierSourceFailure,
        > {
            panic!("hook-only attachment must not send operation or observation frames")
        }
    }
    let attachment = crate::ResidentAttachment::provision_hooks(
        Box::new(NeverOpen),
        std::time::Duration::from_secs(1),
    )
    .unwrap();
    let api = PaleApi::new().unwrap();
    let fixture = fixture();
    let error = api
        .observe_resident_operation(Some(&attachment), request(&fixture))
        .unwrap_err();
    assert_eq!(
        error.diagnostic().code(),
        "resident_observation_unavailable"
    );
    let error = api
        .start_resident_invoke(
            Some(&attachment),
            ResidentInvokeStartRequest::new(
                operation_fixture("hook-only-rejected", "invoke", None).request_bytes,
            ),
        )
        .unwrap_err();
    assert_eq!(error.diagnostic().code(), "resident_operation_unavailable");
    assert!(!attachment.supports_operation_requests());
    assert!(!attachment.supports_observation_requests());
}
