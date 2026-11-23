//! Native start keeps the exact body while retaining existing response admission.
//! Record signatures and operation responses in these fixtures are models.

use super::*;
use crate::{
    PreparedResidentNativeTool, ResidentHookRequest, ResidentNativeRequestSigner,
    ResidentNativeRequestSigningFailure, ResidentNativeRequestSigningInput,
};

#[path = "resident_hook_response.rs"]
mod hook_response;

struct RequestSigner {
    expected: Vec<u8>,
    calls: usize,
    refuse: bool,
}

impl ResidentNativeRequestSigner for RequestSigner {
    fn sign(
        &mut self,
        input: ResidentNativeRequestSigningInput<'_>,
        deadline: Instant,
    ) -> Result<String, ResidentNativeRequestSigningFailure> {
        self.calls += 1;
        assert!(Instant::now() < deadline);
        assert_eq!(input.canonical_preimage(), self.expected);
        assert_eq!(input.purpose(), b"PALE_COMMON_REQUEST_ENVELOPE_V1\0");
        assert_eq!(
            input.digest(),
            ArtifactDigest::hash_bytes(&self.expected).as_str()
        );
        assert!(!format!("{input:?}").contains("native-session"));
        if self.refuse {
            Err(ResidentNativeRequestSigningFailure)
        } else {
            Ok("A".repeat(86))
        }
    }
}

fn unsigned_request(fixture: &Fixture) -> Vec<u8> {
    let mut value = parsed(&fixture.request_bytes);
    let object = value.as_object_mut().unwrap();
    object.remove("canonical_request_sha256");
    object.remove("caller_signature");
    canonical(&value)
}

#[test]
fn native_signing_preserves_exact_request_through_start() {
    let api = PaleApi::new().unwrap();
    let prepared = prepared(&api);
    let fixture = native_fixture(parsed(prepared.request_body_json()));
    let unsigned = unsigned_request(&fixture);
    let mut signer = RequestSigner {
        expected: unsigned.clone(),
        calls: 0,
        refuse: false,
    };
    let request = api
        .sign_resident_native_tool_request(
            &prepared,
            &unsigned,
            &mut signer,
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
    assert_eq!(signer.calls, 1);
    let attachment = operation_attachment(
        &fixture,
        ResidentOperationFrameRequest::StartInvoke {
            signed_common_request: String::from_utf8(fixture.request_bytes.clone()).unwrap(),
        },
        ResidentOperationFrameResponse::Started {
            records: operation_records(&fixture),
        },
    );
    api.start_resident_native_tool(Some(&attachment), prepared, request)
        .unwrap();
}

#[test]
fn native_signing_rejects_substitution_and_noncanonical_claims_before_signer() {
    let api = PaleApi::new().unwrap();
    let prepared = prepared(&api);
    let fixture = native_fixture(parsed(prepared.request_body_json()));
    let unsigned = unsigned_request(&fixture);
    let mut candidates = Vec::new();
    let mut noncanonical = unsigned.clone();
    noncanonical.push(b'\n');
    candidates.push(noncanonical);
    for (field, value) in [
        ("unknown", serde_json::json!(true)),
        ("operation_target", serde_json::Value::Null),
        ("envelope_schema_sha256", serde_json::json!("f".repeat(64))),
        ("body", serde_json::json!({})),
    ] {
        let mut candidate = parsed(&unsigned);
        candidate[field] = value;
        candidates.push(canonical(&candidate));
    }
    let mut candidate = parsed(&unsigned);
    candidate["body"]["payload"]["native_event"] = serde_json::json!("{}");
    candidate["body"]["payload_sha256"] = serde_json::json!(ArtifactDigest::hash_bytes(
        &canonical(&candidate["body"]["payload"])
    )
    .as_str());
    candidates.push(canonical(&candidate));
    let mut signer = RequestSigner {
        expected: unsigned,
        calls: 0,
        refuse: false,
    };
    for candidate in candidates {
        let error = api
            .sign_resident_native_tool_request(
                &prepared,
                &candidate,
                &mut signer,
                Instant::now() + Duration::from_secs(2),
            )
            .unwrap_err();
        assert_eq!(error.diagnostic().code(), "resident_native_request_invalid");
    }
    assert_eq!(signer.calls, 0);
}

#[test]
fn native_signing_does_not_retry_refusal_or_invoke_an_expired_signer() {
    let api = PaleApi::new().unwrap();
    let prepared = prepared(&api);
    let fixture = native_fixture(parsed(prepared.request_body_json()));
    let unsigned = unsigned_request(&fixture);
    let mut signer = RequestSigner {
        expected: unsigned.clone(),
        calls: 0,
        refuse: true,
    };
    let error = api
        .sign_resident_native_tool_request(
            &prepared,
            &unsigned,
            &mut signer,
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap_err();
    assert_eq!(
        error.diagnostic().code(),
        "resident_native_request_signing_unavailable"
    );
    assert_eq!(signer.calls, 1);
    let error = api
        .sign_resident_native_tool_request(&prepared, &unsigned, &mut signer, Instant::now())
        .unwrap_err();
    assert_eq!(
        error.diagnostic().code(),
        "resident_native_request_signing_unavailable"
    );
    assert_eq!(signer.calls, 1);
}

const EVENT: &[u8] = br#"{ "hook_event_name":"PreToolUse", "session_id":"native-session", "tool_use_id":"native-tool-1", "tool_name":"Read", "tool_input":{"file_path":"source.rs"} }"#;

fn prepared(api: &PaleApi) -> PreparedResidentNativeTool {
    let admitted = api
        .admit_resident_hook(ResidentHookRequest::new(
            "pre-tool-use".into(),
            "workspace-1".into(),
            "repository-1".into(),
            "terminal".into(),
            "a".repeat(64),
            EVENT.to_vec(),
        ))
        .unwrap();
    api.prepare_resident_native_tool(admitted).unwrap()
}

fn native_fixture(body: serde_json::Value) -> Fixture {
    let mut fixture = operation_fixture("native-operation-1", "invoke", None);
    let mut request = parsed(&fixture.request_bytes);
    request["body"] = body;
    fixture.request_bytes = reseal_common_request(request);
    let mut handle = parsed(&fixture.handle_bytes);
    bind_handle_to_request(&mut handle, &parsed(&fixture.request_bytes));
    let (digest, handle) = reseal_handle(handle);
    fixture.handle_sha256 = digest;
    fixture.handle_bytes = canonical(&handle);
    fixture.observation_bytes = canonical(&sealed_observation(
        "native-operation-1",
        HANDLE_REF,
        &fixture.handle_sha256,
    ));
    fixture
}

#[test]
fn native_start_returns_a_bound_handle_but_no_native_permission() {
    let api = PaleApi::new().unwrap();
    let prepared = prepared(&api);
    let body = parsed(prepared.request_body_json());
    assert_eq!(
        body["payload"]["native_event"],
        std::str::from_utf8(EVENT).unwrap()
    );
    assert!(!format!("{prepared:?}").contains("native-session"));
    let fixture = native_fixture(body);
    let attachment = operation_attachment(
        &fixture,
        ResidentOperationFrameRequest::StartInvoke {
            signed_common_request: String::from_utf8(fixture.request_bytes.clone()).unwrap(),
        },
        ResidentOperationFrameResponse::Started {
            records: operation_records(&fixture),
        },
    );
    let started = api
        .start_resident_native_tool(
            Some(&attachment),
            prepared,
            ResidentInvokeStartRequest::new(fixture.request_bytes),
        )
        .unwrap();
    assert_eq!(
        started.observation().operation_handle_record_ref(),
        HANDLE_REF
    );
    let blocked = api
        .deliver_resident_hook(None, started.into_hook_admission())
        .unwrap_err();
    assert_eq!(
        blocked.diagnostic().disposition(),
        DiagnosticDisposition::Blocked
    );
}

#[test]
fn native_start_rejects_a_resigned_substitute_without_sending() {
    let api = PaleApi::new().unwrap();
    let prepared = prepared(&api);
    let mut body = parsed(prepared.request_body_json());
    let event = body["payload"]["native_event"]
        .as_str()
        .unwrap()
        .replace("source.rs", "other.rs");
    body["payload"]["native_event"] = event.into();
    body["payload_sha256"] = ArtifactDigest::hash_bytes(&canonical(&body["payload"]))
        .as_str()
        .into();
    let fixture = native_fixture(body);
    let calls = Arc::new(AtomicUsize::new(0));
    let attachment = ResidentAttachment::from_clients(
        FakeClient {
            expected_request_bytes: fixture.request_bytes.clone(),
            response_handle_ref: HANDLE_REF.into(),
            handle_bytes: fixture.handle_bytes,
            observation_bytes: fixture.observation_bytes,
        },
        NoSendOperationClient {
            calls: Arc::clone(&calls),
        },
    );
    let error = api
        .start_resident_native_tool(
            Some(&attachment),
            prepared,
            ResidentInvokeStartRequest::new(fixture.request_bytes),
        )
        .unwrap_err();
    assert_eq!(
        error.diagnostic().code(),
        "resident_operation_request_invalid"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
